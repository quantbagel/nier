//! Kafka consumer wrapper for the Nier pipeline.
//!
//! This module provides a high-level, type-safe interface for consuming messages
//! from Kafka topics with support for protobuf deserialization and reliable processing.

use crate::config::KafkaConfig;
use crate::producer::{NierProducer, ProducerError};
use prost::Message;
use rdkafka::consumer::{Consumer, StreamConsumer};
use rdkafka::message::{Headers, Message as KafkaMessage};
use rdkafka::TopicPartitionList;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use tokio::sync::broadcast;
use tracing::{debug, error, info, instrument, warn};

/// Errors that can occur during message consumption
#[derive(Error, Debug)]
pub enum ConsumerError {
    #[error("Failed to create consumer: {0}")]
    CreationError(String),

    #[error("Failed to subscribe to topics: {0}")]
    SubscriptionError(String),

    #[error("Failed to deserialize message: {0}")]
    DeserializationError(String),

    #[error("Failed to commit offset: {0}")]
    CommitError(String),

    #[error("Consumer poll error: {0}")]
    PollError(String),

    #[error("Message processing error: {0}")]
    ProcessingError(String),

    #[error("Consumer shutdown")]
    Shutdown,
}

/// Metadata about a received message
#[derive(Debug, Clone)]
pub struct MessageMetadata {
    /// Topic the message was received from
    pub topic: String,
    /// Partition the message was received from
    pub partition: i32,
    /// Offset of the message in the partition
    pub offset: i64,
    /// Message key (if present)
    pub key: Option<Vec<u8>>,
    /// Timestamp of the message
    pub timestamp: Option<i64>,
    /// Message headers
    pub headers: HashMap<String, String>,
}

/// A received message with payload and metadata
#[derive(Debug, Clone)]
pub struct IncomingMessage {
    /// Raw message payload
    pub payload: Vec<u8>,
    /// Message metadata
    pub metadata: MessageMetadata,
}

impl IncomingMessage {
    /// Deserialize the payload as a protobuf message
    pub fn decode_proto<M: Message + Default>(&self) -> Result<M, ConsumerError> {
        M::decode(self.payload.as_slice())
            .map_err(|e| ConsumerError::DeserializationError(e.to_string()))
    }

    /// Deserialize the payload as JSON
    pub fn decode_json<T: serde::de::DeserializeOwned>(&self) -> Result<T, ConsumerError> {
        serde_json::from_slice(&self.payload)
            .map_err(|e| ConsumerError::DeserializationError(e.to_string()))
    }

    /// Get the message key as a string
    pub fn key_str(&self) -> Option<String> {
        self.metadata
            .key
            .as_ref()
            .and_then(|k| String::from_utf8(k.clone()).ok())
    }

    /// Get a header value
    pub fn header(&self, key: &str) -> Option<&str> {
        self.metadata.headers.get(key).map(|s| s.as_str())
    }

    /// Get the correlation ID header
    pub fn correlation_id(&self) -> Option<&str> {
        self.header("correlation-id")
    }

    /// Get the message type header
    pub fn message_type(&self) -> Option<&str> {
        self.header("message-type")
    }
}

/// Handler trait for processing messages
#[async_trait::async_trait]
pub trait MessageHandler: Send + Sync {
    /// Process a single message
    async fn handle(&self, message: IncomingMessage) -> Result<(), ConsumerError>;

    /// Called when message processing fails
    async fn on_error(&self, message: IncomingMessage, error: ConsumerError) {
        warn!(
            "Message processing failed for topic={}, partition={}, offset={}: {}",
            message.metadata.topic,
            message.metadata.partition,
            message.metadata.offset,
            error
        );
    }
}

/// Function-based message handler
pub struct FnHandler<F>
where
    F: Fn(IncomingMessage) -> Result<(), ConsumerError> + Send + Sync,
{
    handler: F,
}

impl<F> FnHandler<F>
where
    F: Fn(IncomingMessage) -> Result<(), ConsumerError> + Send + Sync,
{
    pub fn new(handler: F) -> Self {
        Self { handler }
    }
}

#[async_trait::async_trait]
impl<F> MessageHandler for FnHandler<F>
where
    F: Fn(IncomingMessage) -> Result<(), ConsumerError> + Send + Sync,
{
    async fn handle(&self, message: IncomingMessage) -> Result<(), ConsumerError> {
        (self.handler)(message)
    }
}

/// High-level Kafka consumer wrapper
pub struct NierConsumer {
    consumer: StreamConsumer,
    config: Arc<KafkaConfig>,
    shutdown_tx: broadcast::Sender<()>,
    dlq_producer: Option<Arc<NierProducer>>,
}

impl NierConsumer {
    /// Create a new consumer with the given configuration
    pub fn new(config: KafkaConfig) -> Result<Self, ConsumerError> {
        info!(
            "Creating Kafka consumer for {} with group {}",
            config.bootstrap_servers, config.consumer.group_id
        );

        let consumer_config = config.build_consumer_config();
        let consumer: StreamConsumer = consumer_config
            .create()
            .map_err(|e| ConsumerError::CreationError(e.to_string()))?;

        let (shutdown_tx, _) = broadcast::channel(1);

        Ok(Self {
            consumer,
            config: Arc::new(config),
            shutdown_tx,
            dlq_producer: None,
        })
    }

    /// Set the dead letter queue producer
    pub fn with_dlq_producer(mut self, producer: Arc<NierProducer>) -> Self {
        self.dlq_producer = Some(producer);
        self
    }

    /// Get the configuration
    pub fn config(&self) -> &KafkaConfig {
        &self.config
    }

    /// Subscribe to the specified topics
    pub fn subscribe(&self, topics: &[&str]) -> Result<(), ConsumerError> {
        info!("Subscribing to topics: {:?}", topics);
        self.consumer
            .subscribe(topics)
            .map_err(|e| ConsumerError::SubscriptionError(e.to_string()))
    }

    /// Subscribe to all Nier pipeline topics
    pub fn subscribe_all(&self) -> Result<(), ConsumerError> {
        let topics = [
            self.config.topics.frames.as_str(),
            self.config.topics.detections.as_str(),
            self.config.topics.alerts.as_str(),
        ];
        self.subscribe(&topics)
    }

    /// Subscribe to frame metadata topic
    pub fn subscribe_frames(&self) -> Result<(), ConsumerError> {
        self.subscribe(&[self.config.topics.frames.as_str()])
    }

    /// Subscribe to detection events topic
    pub fn subscribe_detections(&self) -> Result<(), ConsumerError> {
        self.subscribe(&[self.config.topics.detections.as_str()])
    }

    /// Subscribe to alerts topic
    pub fn subscribe_alerts(&self) -> Result<(), ConsumerError> {
        self.subscribe(&[self.config.topics.alerts.as_str()])
    }

    /// Commit the current offsets synchronously
    pub fn commit(&self) -> Result<(), ConsumerError> {
        self.consumer
            .commit_consumer_state(rdkafka::consumer::CommitMode::Sync)
            .map_err(|e| ConsumerError::CommitError(e.to_string()))
    }

    /// Commit offsets asynchronously
    pub fn commit_async(&self) {
        if let Err(e) = self
            .consumer
            .commit_consumer_state(rdkafka::consumer::CommitMode::Async)
        {
            warn!("Failed to commit offsets asynchronously: {}", e);
        }
    }

    /// Get a shutdown receiver
    pub fn shutdown_receiver(&self) -> broadcast::Receiver<()> {
        self.shutdown_tx.subscribe()
    }

    /// Signal shutdown to stop consuming
    pub fn shutdown(&self) {
        info!("Signaling consumer shutdown");
        let _ = self.shutdown_tx.send(());
    }

    /// Start consuming messages and process them with the given handler
    #[instrument(skip(self, handler))]
    pub async fn run<H: MessageHandler>(&self, handler: Arc<H>) -> Result<(), ConsumerError> {
        use rdkafka::message::Message as _;
        use tokio_stream::StreamExt;

        let mut shutdown_rx = self.shutdown_receiver();
        let stream = self.consumer.stream();
        tokio::pin!(stream);

        info!("Starting message consumption loop");

        loop {
            tokio::select! {
                _ = shutdown_rx.recv() => {
                    info!("Received shutdown signal");
                    break;
                }
                message_result = stream.next() => {
                    match message_result {
                        Some(Ok(borrowed_message)) => {
                            let incoming = self.convert_message(&borrowed_message);

                            debug!(
                                "Received message from topic={}, partition={}, offset={}",
                                incoming.metadata.topic,
                                incoming.metadata.partition,
                                incoming.metadata.offset
                            );

                            match handler.handle(incoming.clone()).await {
                                Ok(()) => {
                                    if !self.config.consumer.enable_auto_commit {
                                        self.commit_async();
                                    }
                                }
                                Err(e) => {
                                    error!("Message processing failed: {}", e);
                                    handler.on_error(incoming.clone(), e).await;

                                    // Send to DLQ if configured
                                    if let Some(ref dlq) = self.dlq_producer {
                                        if let Err(dlq_err) = dlq
                                            .send_to_dlq(
                                                &incoming.metadata.topic,
                                                &incoming.payload,
                                                "Processing failed",
                                            )
                                            .await
                                        {
                                            error!("Failed to send to DLQ: {}", dlq_err);
                                        }
                                    }
                                }
                            }
                        }
                        Some(Err(e)) => {
                            error!("Kafka error: {}", e);
                        }
                        None => {
                            debug!("Stream ended");
                            break;
                        }
                    }
                }
            }
        }

        // Final commit before shutdown
        if !self.config.consumer.enable_auto_commit {
            if let Err(e) = self.commit() {
                warn!("Failed to commit on shutdown: {}", e);
            }
        }

        Ok(())
    }

    /// Consume messages with a simple callback function
    pub async fn run_with_callback<F, Fut>(&self, callback: F) -> Result<(), ConsumerError>
    where
        F: Fn(IncomingMessage) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<(), ConsumerError>> + Send,
    {
        use rdkafka::message::Message as _;
        use tokio_stream::StreamExt;

        let mut shutdown_rx = self.shutdown_receiver();
        let stream = self.consumer.stream();
        tokio::pin!(stream);

        loop {
            tokio::select! {
                _ = shutdown_rx.recv() => {
                    break;
                }
                message_result = stream.next() => {
                    match message_result {
                        Some(Ok(borrowed_message)) => {
                            let incoming = self.convert_message(&borrowed_message);
                            if let Err(e) = callback(incoming).await {
                                error!("Callback error: {}", e);
                            } else if !self.config.consumer.enable_auto_commit {
                                self.commit_async();
                            }
                        }
                        Some(Err(e)) => {
                            error!("Kafka error: {}", e);
                        }
                        None => break,
                    }
                }
            }
        }

        Ok(())
    }

    /// Convert a borrowed Kafka message to our IncomingMessage type
    fn convert_message<M: KafkaMessage>(&self, msg: &M) -> IncomingMessage {
        let payload = msg.payload().unwrap_or(&[]).to_vec();
        let key = msg.key().map(|k| k.to_vec());

        let mut headers = HashMap::new();
        if let Some(h) = msg.headers() {
            for header in h.iter() {
                if let Some(value) = header.value {
                    if let Ok(v) = String::from_utf8(value.to_vec()) {
                        headers.insert(header.key.to_string(), v);
                    }
                }
            }
        }

        IncomingMessage {
            payload,
            metadata: MessageMetadata {
                topic: msg.topic().to_string(),
                partition: msg.partition(),
                offset: msg.offset(),
                key,
                timestamp: msg.timestamp().to_millis(),
                headers,
            },
        }
    }

    /// Get the current partition assignment
    pub fn assignment(&self) -> Result<TopicPartitionList, ConsumerError> {
        self.consumer
            .assignment()
            .map_err(|e| ConsumerError::PollError(e.to_string()))
    }

    /// Get the current position for all assigned partitions
    pub fn position(&self) -> Result<TopicPartitionList, ConsumerError> {
        self.consumer
            .position()
            .map_err(|e| ConsumerError::PollError(e.to_string()))
    }

    /// Pause consumption for specific partitions
    pub fn pause(&self, partitions: &TopicPartitionList) -> Result<(), ConsumerError> {
        self.consumer
            .pause(partitions)
            .map_err(|e| ConsumerError::PollError(e.to_string()))
    }

    /// Resume consumption for specific partitions
    pub fn resume(&self, partitions: &TopicPartitionList) -> Result<(), ConsumerError> {
        self.consumer
            .resume(partitions)
            .map_err(|e| ConsumerError::PollError(e.to_string()))
    }
}

/// Builder for creating consumers with custom settings
pub struct ConsumerBuilder {
    config: KafkaConfig,
    dlq_producer: Option<Arc<NierProducer>>,
}

impl ConsumerBuilder {
    /// Create a new consumer builder
    pub fn new(bootstrap_servers: impl Into<String>) -> Self {
        Self {
            config: KafkaConfig::new(bootstrap_servers),
            dlq_producer: None,
        }
    }

    /// Set the consumer group ID
    pub fn group_id(mut self, group_id: impl Into<String>) -> Self {
        self.config.consumer.group_id = group_id.into();
        self
    }

    /// Set the client ID
    pub fn client_id(mut self, client_id: impl Into<String>) -> Self {
        self.config.client_id = client_id.into();
        self
    }

    /// Set auto offset reset behavior
    pub fn auto_offset_reset(mut self, reset: impl Into<String>) -> Self {
        self.config.consumer.auto_offset_reset = reset.into();
        self
    }

    /// Enable or disable auto commit
    pub fn enable_auto_commit(mut self, enable: bool) -> Self {
        self.config.consumer.enable_auto_commit = enable;
        self
    }

    /// Set the dead letter queue producer
    pub fn with_dlq_producer(mut self, producer: Arc<NierProducer>) -> Self {
        self.dlq_producer = Some(producer);
        self
    }

    /// Build the consumer
    pub fn build(self) -> Result<NierConsumer, ConsumerError> {
        let mut consumer = NierConsumer::new(self.config)?;
        if let Some(dlq) = self.dlq_producer {
            consumer = consumer.with_dlq_producer(dlq);
        }
        Ok(consumer)
    }
}

/// Async trait for message handlers (re-export for convenience)
pub use async_trait::async_trait;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_incoming_message_headers() {
        let mut headers = HashMap::new();
        headers.insert("correlation-id".to_string(), "test-123".to_string());
        headers.insert("message-type".to_string(), "detection_event".to_string());

        let message = IncomingMessage {
            payload: vec![1, 2, 3],
            metadata: MessageMetadata {
                topic: "test".to_string(),
                partition: 0,
                offset: 100,
                key: Some(b"key".to_vec()),
                timestamp: Some(1234567890),
                headers,
            },
        };

        assert_eq!(message.correlation_id(), Some("test-123"));
        assert_eq!(message.message_type(), Some("detection_event"));
        assert_eq!(message.key_str(), Some("key".to_string()));
    }
}
