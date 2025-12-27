use crate::config::KafkaConfig;
use crate::frame_selector::{FrameSelector, StorageDecision};
use crate::metadata_store::MetadataStore;
use crate::s3_uploader::S3Uploader;
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use futures::StreamExt;
use rdkafka::config::ClientConfig;
use rdkafka::consumer::{CommitMode, Consumer, StreamConsumer};
use rdkafka::message::{BorrowedMessage, Message};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Semaphore;
use tracing::{debug, error, info, instrument, warn};
use uuid::Uuid;

/// Storage trigger event received from Kafka
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageTriggerEvent {
    /// Unique event ID
    pub event_id: Uuid,
    /// Device ID (camera glasses identifier)
    pub device_id: String,
    /// Frame timestamp
    pub timestamp: DateTime<Utc>,
    /// Frame sequence number within the stream
    pub frame_number: u64,
    /// Raw frame data (JPEG/PNG encoded)
    #[serde(with = "base64_serde")]
    pub frame_data: Vec<u8>,
    /// Frame width in pixels
    pub width: u32,
    /// Frame height in pixels
    pub height: u32,
    /// Frame format (jpeg, png, etc.)
    pub format: String,
    /// Associated detections (if any)
    #[serde(default)]
    pub detections: Vec<Detection>,
    /// Event type that triggered storage consideration
    pub trigger_type: TriggerType,
    /// Additional metadata
    #[serde(default)]
    pub metadata: serde_json::Value,
}

/// Detection information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Detection {
    /// Detection type/class
    pub detection_type: String,
    /// Confidence score (0.0 - 1.0)
    pub confidence: f32,
    /// Bounding box [x, y, width, height] normalized 0-1
    pub bbox: [f32; 4],
    /// Additional detection metadata
    #[serde(default)]
    pub attributes: serde_json::Value,
}

/// Type of event that triggered storage consideration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TriggerType {
    /// Frame contains detections
    Detection,
    /// Periodic sample frame
    Sample,
    /// Debug/troubleshooting frame
    Debug,
    /// Manual trigger from operator
    Manual,
    /// Alert condition triggered
    Alert,
}

/// Base64 serialization helper
mod base64_serde {
    use base64::{engine::general_purpose::STANDARD, Engine};
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&STANDARD.encode(bytes))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        STANDARD.decode(s).map_err(serde::de::Error::custom)
    }
}

/// Kafka consumer for storage trigger events
pub struct StorageKafkaConsumer {
    consumer: StreamConsumer,
    frame_selector: Arc<FrameSelector>,
    s3_uploader: Arc<S3Uploader>,
    metadata_store: Arc<MetadataStore>,
    upload_semaphore: Arc<Semaphore>,
}

impl StorageKafkaConsumer {
    /// Create a new Kafka consumer for storage events
    pub async fn new(
        config: &KafkaConfig,
        frame_selector: Arc<FrameSelector>,
        s3_uploader: Arc<S3Uploader>,
        metadata_store: Arc<MetadataStore>,
        upload_concurrency: usize,
    ) -> Result<Self> {
        let mut client_config = ClientConfig::new();

        client_config
            .set("bootstrap.servers", &config.bootstrap_servers)
            .set("group.id", &config.consumer_group)
            .set("auto.offset.reset", &config.auto_offset_reset)
            .set("enable.auto.commit", "false")
            .set("session.timeout.ms", config.session_timeout_ms.to_string())
            .set("max.poll.interval.ms", config.max_poll_interval_ms.to_string());

        // Configure SSL if enabled
        if config.ssl_enabled {
            client_config.set("security.protocol", "SASL_SSL");
            if let Some(ref ca_location) = config.ssl_ca_location {
                client_config.set("ssl.ca.location", ca_location);
            }
        }

        // Configure SASL if credentials provided
        if let (Some(ref username), Some(ref password)) =
            (&config.sasl_username, &config.sasl_password)
        {
            client_config
                .set("sasl.mechanisms", "PLAIN")
                .set("sasl.username", username)
                .set("sasl.password", password);
        }

        let consumer: StreamConsumer = client_config
            .create()
            .context("Failed to create Kafka consumer")?;

        consumer
            .subscribe(&[&config.storage_trigger_topic])
            .context("Failed to subscribe to storage trigger topic")?;

        info!(
            topic = %config.storage_trigger_topic,
            group = %config.consumer_group,
            "Subscribed to Kafka topic"
        );

        Ok(Self {
            consumer,
            frame_selector,
            s3_uploader,
            metadata_store,
            upload_semaphore: Arc::new(Semaphore::new(upload_concurrency)),
        })
    }

    /// Start consuming and processing messages
    #[instrument(skip(self))]
    pub async fn run(&self) -> Result<()> {
        info!("Starting storage Kafka consumer");

        let mut message_stream = self.consumer.stream();

        while let Some(message_result) = message_stream.next().await {
            match message_result {
                Ok(message) => {
                    if let Err(e) = self.process_message(&message).await {
                        error!(
                            error = %e,
                            partition = message.partition(),
                            offset = message.offset(),
                            "Failed to process message"
                        );
                        // Continue processing other messages
                        metrics::counter!("storage.messages.failed").increment(1);
                    } else {
                        // Commit offset on success
                        if let Err(e) = self.consumer.commit_message(&message, CommitMode::Async) {
                            warn!(error = %e, "Failed to commit offset");
                        }
                        metrics::counter!("storage.messages.processed").increment(1);
                    }
                }
                Err(e) => {
                    error!(error = %e, "Kafka consumer error");
                    metrics::counter!("storage.kafka.errors").increment(1);
                }
            }
        }

        Ok(())
    }

    /// Process a single Kafka message
    #[instrument(skip(self, message), fields(partition = message.partition(), offset = message.offset()))]
    async fn process_message(&self, message: &BorrowedMessage<'_>) -> Result<()> {
        let payload = message
            .payload()
            .context("Message has no payload")?;

        let event: StorageTriggerEvent = serde_json::from_slice(payload)
            .context("Failed to deserialize storage trigger event")?;

        debug!(
            event_id = %event.event_id,
            device_id = %event.device_id,
            trigger_type = ?event.trigger_type,
            "Received storage trigger event"
        );

        // Check if frame should be stored
        let decision = self.frame_selector.should_store(&event);

        match decision {
            StorageDecision::Store { reason } => {
                info!(
                    event_id = %event.event_id,
                    device_id = %event.device_id,
                    reason = %reason,
                    "Storing frame"
                );
                self.store_frame(event, reason).await?;
            }
            StorageDecision::Skip { reason } => {
                debug!(
                    event_id = %event.event_id,
                    device_id = %event.device_id,
                    reason = %reason,
                    "Skipping frame storage"
                );
                metrics::counter!("storage.frames.skipped").increment(1);
            }
        }

        Ok(())
    }

    /// Store a frame to S3 and index in metadata store
    #[instrument(skip(self, event), fields(event_id = %event.event_id, device_id = %event.device_id))]
    async fn store_frame(&self, event: StorageTriggerEvent, storage_reason: String) -> Result<()> {
        // Acquire semaphore permit to limit concurrency
        let _permit = self
            .upload_semaphore
            .acquire()
            .await
            .context("Failed to acquire upload semaphore")?;

        let timer = metrics::histogram!("storage.upload.duration_seconds").start_timer();

        // Upload to S3
        let s3_key = self.s3_uploader.upload_frame(&event).await?;

        timer.stop();

        // Store metadata in Postgres
        self.metadata_store
            .index_frame(&event, &s3_key, &storage_reason)
            .await?;

        metrics::counter!("storage.frames.stored").increment(1);
        metrics::counter!("storage.bytes.uploaded").increment(event.frame_data.len() as u64);

        info!(
            event_id = %event.event_id,
            s3_key = %s3_key,
            size_bytes = event.frame_data.len(),
            "Frame stored successfully"
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_storage_trigger_event() {
        let json = r#"{
            "event_id": "550e8400-e29b-41d4-a716-446655440000",
            "device_id": "glasses-001",
            "timestamp": "2024-01-15T10:30:00Z",
            "frame_number": 12345,
            "frame_data": "SGVsbG8gV29ybGQ=",
            "width": 1920,
            "height": 1080,
            "format": "jpeg",
            "detections": [{
                "detection_type": "safety_vest",
                "confidence": 0.95,
                "bbox": [0.1, 0.2, 0.3, 0.4],
                "attributes": {}
            }],
            "trigger_type": "detection",
            "metadata": {}
        }"#;

        let event: StorageTriggerEvent = serde_json::from_str(json).unwrap();
        assert_eq!(event.device_id, "glasses-001");
        assert_eq!(event.detections.len(), 1);
        assert_eq!(event.trigger_type, TriggerType::Detection);
    }

    #[test]
    fn test_trigger_type_serialization() {
        assert_eq!(
            serde_json::to_string(&TriggerType::Detection).unwrap(),
            "\"detection\""
        );
        assert_eq!(
            serde_json::to_string(&TriggerType::Sample).unwrap(),
            "\"sample\""
        );
    }
}
