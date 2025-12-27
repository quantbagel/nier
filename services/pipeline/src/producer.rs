//! Kafka producer wrapper for the Nier pipeline.
//!
//! This module provides a high-level, type-safe interface for producing messages
//! to Kafka topics with support for protobuf serialization and reliable delivery.

use crate::config::KafkaConfig;
use prost::Message;
use rdkafka::producer::{FutureProducer, FutureRecord, Producer};
use rdkafka::util::Timeout;
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use tracing::{debug, error, info, instrument, warn};
use uuid::Uuid;

/// Errors that can occur during message production
#[derive(Error, Debug)]
pub enum ProducerError {
    #[error("Failed to create producer: {0}")]
    CreationError(String),

    #[error("Failed to serialize message: {0}")]
    SerializationError(String),

    #[error("Failed to send message to topic {topic}: {message}")]
    SendError { topic: String, message: String },

    #[error("Producer timeout after {0:?}")]
    Timeout(Duration),

    #[error("Producer is not connected")]
    NotConnected,
}

/// Result of a successful message delivery
#[derive(Debug, Clone)]
pub struct DeliveryResult {
    /// Topic the message was delivered to
    pub topic: String,
    /// Partition the message was delivered to
    pub partition: i32,
    /// Offset of the message in the partition
    pub offset: i64,
    /// Message key (if provided)
    pub key: Option<String>,
}

/// Message to be sent to Kafka
#[derive(Debug, Clone)]
pub struct OutgoingMessage {
    /// Topic to send the message to
    pub topic: String,
    /// Optional message key for partitioning
    pub key: Option<String>,
    /// Serialized message payload
    pub payload: Vec<u8>,
    /// Optional headers
    pub headers: Vec<(String, String)>,
}

impl OutgoingMessage {
    /// Create a new outgoing message with a protobuf payload
    pub fn new_proto<M: Message>(topic: impl Into<String>, message: &M) -> Result<Self, ProducerError> {
        let payload = message.encode_to_vec();
        Ok(Self {
            topic: topic.into(),
            key: None,
            payload,
            headers: Vec::new(),
        })
    }

    /// Create a new outgoing message with a JSON payload
    pub fn new_json<T: serde::Serialize>(
        topic: impl Into<String>,
        message: &T,
    ) -> Result<Self, ProducerError> {
        let payload = serde_json::to_vec(message)
            .map_err(|e| ProducerError::SerializationError(e.to_string()))?;
        Ok(Self {
            topic: topic.into(),
            key: None,
            payload,
            headers: Vec::new(),
        })
    }

    /// Set the message key
    pub fn with_key(mut self, key: impl Into<String>) -> Self {
        self.key = Some(key.into());
        self
    }

    /// Add a header to the message
    pub fn with_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((key.into(), value.into()));
        self
    }

    /// Add a correlation ID header
    pub fn with_correlation_id(self, id: impl Into<String>) -> Self {
        self.with_header("correlation-id", id)
    }

    /// Add a message type header
    pub fn with_message_type(self, msg_type: impl Into<String>) -> Self {
        self.with_header("message-type", msg_type)
    }
}

/// High-level Kafka producer wrapper
pub struct NierProducer {
    producer: FutureProducer,
    config: Arc<KafkaConfig>,
    default_timeout: Duration,
}

impl NierProducer {
    /// Create a new producer with the given configuration
    pub fn new(config: KafkaConfig) -> Result<Self, ProducerError> {
        info!(
            "Creating Kafka producer for {}",
            config.bootstrap_servers
        );

        let producer_config = config.build_producer_config();
        let producer: FutureProducer = producer_config
            .create()
            .map_err(|e| ProducerError::CreationError(e.to_string()))?;

        let default_timeout = config.request_timeout();

        Ok(Self {
            producer,
            config: Arc::new(config),
            default_timeout,
        })
    }

    /// Get the configuration
    pub fn config(&self) -> &KafkaConfig {
        &self.config
    }

    /// Send a message and wait for delivery confirmation
    #[instrument(skip(self, message), fields(topic = %message.topic, key = ?message.key))]
    pub async fn send(&self, message: OutgoingMessage) -> Result<DeliveryResult, ProducerError> {
        self.send_with_timeout(message, self.default_timeout).await
    }

    /// Send a message with a custom timeout
    #[instrument(skip(self, message), fields(topic = %message.topic, key = ?message.key))]
    pub async fn send_with_timeout(
        &self,
        message: OutgoingMessage,
        timeout: Duration,
    ) -> Result<DeliveryResult, ProducerError> {
        let topic = message.topic.clone();
        let key = message.key.clone();

        let mut record = FutureRecord::to(&topic).payload(&message.payload);

        if let Some(ref k) = key {
            record = record.key(k);
        }

        debug!(
            "Sending message to topic {} (size: {} bytes)",
            topic,
            message.payload.len()
        );

        let delivery_result = self
            .producer
            .send(record, Timeout::After(timeout))
            .await
            .map_err(|(e, _)| ProducerError::SendError {
                topic: topic.clone(),
                message: e.to_string(),
            })?;

        let result = DeliveryResult {
            topic,
            partition: delivery_result.0,
            offset: delivery_result.1,
            key,
        };

        debug!(
            "Message delivered to partition {} at offset {}",
            result.partition, result.offset
        );

        Ok(result)
    }

    /// Send multiple messages in parallel
    #[instrument(skip(self, messages), fields(count = messages.len()))]
    pub async fn send_batch(
        &self,
        messages: Vec<OutgoingMessage>,
    ) -> Vec<Result<DeliveryResult, ProducerError>> {
        let futures: Vec<_> = messages.into_iter().map(|msg| self.send(msg)).collect();

        futures::future::join_all(futures).await
    }

    /// Send a detection event to the detections topic
    pub async fn send_detection_event<M: Message>(
        &self,
        event: &M,
        event_id: impl Into<String>,
    ) -> Result<DeliveryResult, ProducerError> {
        let event_id = event_id.into();
        let message = OutgoingMessage::new_proto(&self.config.topics.detections, event)?
            .with_key(&event_id)
            .with_message_type("detection_event")
            .with_correlation_id(&event_id);

        self.send(message).await
    }

    /// Send frame metadata to the frames topic
    pub async fn send_frame_metadata<M: Message>(
        &self,
        metadata: &M,
        frame_id: impl Into<String>,
    ) -> Result<DeliveryResult, ProducerError> {
        let frame_id = frame_id.into();
        let message = OutgoingMessage::new_proto(&self.config.topics.frames, metadata)?
            .with_key(&frame_id)
            .with_message_type("frame_metadata");

        self.send(message).await
    }

    /// Send an alert to the alerts topic
    pub async fn send_alert<M: Message>(
        &self,
        alert: &M,
        alert_id: impl Into<String>,
    ) -> Result<DeliveryResult, ProducerError> {
        let alert_id = alert_id.into();
        let message = OutgoingMessage::new_proto(&self.config.topics.alerts, alert)?
            .with_key(&alert_id)
            .with_message_type("alert");

        self.send(message).await
    }

    /// Send a message to the dead letter queue
    pub async fn send_to_dlq(
        &self,
        original_topic: &str,
        original_message: &[u8],
        error: &str,
    ) -> Result<DeliveryResult, ProducerError> {
        let dlq_message = serde_json::json!({
            "original_topic": original_topic,
            "original_message_base64": base64_encode(original_message),
            "error": error,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });

        let message = OutgoingMessage::new_json(&self.config.topics.dead_letter_queue, &dlq_message)?
            .with_key(Uuid::new_v4().to_string())
            .with_message_type("dead_letter")
            .with_header("original-topic", original_topic)
            .with_header("error-reason", error);

        self.send(message).await
    }

    /// Flush all pending messages
    pub fn flush(&self, timeout: Duration) -> Result<(), ProducerError> {
        self.producer.flush(Timeout::After(timeout)).map_err(|_| {
            ProducerError::Timeout(timeout)
        })
    }

    /// Get the number of messages in the producer queue
    pub fn queue_len(&self) -> usize {
        self.producer.in_flight_count()
    }
}

impl Drop for NierProducer {
    fn drop(&mut self) {
        info!("Shutting down Kafka producer");
        if let Err(e) = self.flush(Duration::from_secs(5)) {
            warn!("Failed to flush producer on shutdown: {}", e);
        }
    }
}

/// Simple base64 encoding helper
fn base64_encode(data: &[u8]) -> String {
    use std::io::Write;
    let mut buf = Vec::new();
    {
        let mut encoder = Base64Encoder::new(&mut buf);
        encoder.write_all(data).ok();
    }
    String::from_utf8(buf).unwrap_or_default()
}

/// Simple base64 encoder
struct Base64Encoder<W: std::io::Write> {
    writer: W,
}

impl<W: std::io::Write> Base64Encoder<W> {
    fn new(writer: W) -> Self {
        Self { writer }
    }
}

impl<W: std::io::Write> std::io::Write for Base64Encoder<W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

        for chunk in buf.chunks(3) {
            let b0 = chunk[0] as usize;
            let b1 = chunk.get(1).copied().unwrap_or(0) as usize;
            let b2 = chunk.get(2).copied().unwrap_or(0) as usize;

            let combined = (b0 << 16) | (b1 << 8) | b2;

            self.writer.write_all(&[ALPHABET[(combined >> 18) & 0x3F]])?;
            self.writer.write_all(&[ALPHABET[(combined >> 12) & 0x3F]])?;

            if chunk.len() > 1 {
                self.writer.write_all(&[ALPHABET[(combined >> 6) & 0x3F]])?;
            } else {
                self.writer.write_all(b"=")?;
            }

            if chunk.len() > 2 {
                self.writer.write_all(&[ALPHABET[combined & 0x3F]])?;
            } else {
                self.writer.write_all(b"=")?;
            }
        }

        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.writer.flush()
    }
}

/// Builder for creating producers with custom settings
pub struct ProducerBuilder {
    config: KafkaConfig,
}

impl ProducerBuilder {
    /// Create a new producer builder
    pub fn new(bootstrap_servers: impl Into<String>) -> Self {
        Self {
            config: KafkaConfig::new(bootstrap_servers),
        }
    }

    /// Set the client ID
    pub fn client_id(mut self, client_id: impl Into<String>) -> Self {
        self.config.client_id = client_id.into();
        self
    }

    /// Enable idempotent producer
    pub fn idempotent(mut self, enable: bool) -> Self {
        self.config.reliability.enable_idempotence = enable;
        self
    }

    /// Set compression type
    pub fn compression(mut self, compression: impl Into<String>) -> Self {
        self.config.producer.compression_type = compression.into();
        self
    }

    /// Set batch size
    pub fn batch_size(mut self, size: usize) -> Self {
        self.config.producer.batch_size = size;
        self
    }

    /// Set linger time
    pub fn linger_ms(mut self, ms: u64) -> Self {
        self.config.producer.linger_ms = ms;
        self
    }

    /// Build the producer
    pub fn build(self) -> Result<NierProducer, ProducerError> {
        NierProducer::new(self.config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_outgoing_message_builder() {
        let message = OutgoingMessage {
            topic: "test".to_string(),
            key: None,
            payload: vec![1, 2, 3],
            headers: vec![],
        }
        .with_key("my-key")
        .with_header("header1", "value1")
        .with_correlation_id("corr-123");

        assert_eq!(message.key, Some("my-key".to_string()));
        assert_eq!(message.headers.len(), 2);
    }

    #[test]
    fn test_base64_encode() {
        let data = b"Hello, World!";
        let encoded = base64_encode(data);
        assert!(!encoded.is_empty());
    }
}
