//! Nier Pipeline - Example Kafka consumer and producer
//!
//! This binary demonstrates how to use the Nier pipeline library to:
//! - Produce messages to Kafka topics
//! - Consume and process messages from Kafka topics
//! - Handle errors and dead letter queues

use anyhow::Result;
use nier_pipeline::prelude::*;
use std::sync::Arc;
use tracing::{error, info, Level};
use tracing_subscriber::FmtSubscriber;

/// Example detection event handler
struct DetectionHandler {
    producer: Arc<NierProducer>,
}

impl DetectionHandler {
    fn new(producer: Arc<NierProducer>) -> Self {
        Self { producer }
    }
}

#[async_trait]
impl MessageHandler for DetectionHandler {
    async fn handle(&self, message: IncomingMessage) -> Result<(), ConsumerError> {
        info!(
            "Processing detection event from partition={}, offset={}",
            message.metadata.partition, message.metadata.offset
        );

        // Parse the message type from headers
        match message.message_type() {
            Some("detection_event") => {
                info!("Received detection event, size={} bytes", message.payload.len());

                // In a real implementation, you would:
                // 1. Deserialize the protobuf message
                // 2. Process the detection (check for violations, aggregate stats, etc.)
                // 3. Potentially generate alerts

                // Example: Generate an alert if this was a PPE violation
                // let event = message.decode_proto::<DetectionEvent>()?;
                // if !event.ppe_violations.is_empty() {
                //     self.producer.send_alert(&alert, alert_id).await?;
                // }

                Ok(())
            }
            Some("frame_metadata") => {
                info!("Received frame metadata, size={} bytes", message.payload.len());
                Ok(())
            }
            Some(other) => {
                info!("Received unknown message type: {}", other);
                Ok(())
            }
            None => {
                info!("Received message without type header");
                Ok(())
            }
        }
    }

    async fn on_error(&self, message: IncomingMessage, error: ConsumerError) {
        error!(
            "Failed to process message from topic={}, partition={}, offset={}: {}",
            message.metadata.topic,
            message.metadata.partition,
            message.metadata.offset,
            error
        );
    }
}

/// Run in producer mode - send example messages
async fn run_producer(config: KafkaConfig) -> Result<()> {
    info!("Starting producer example");

    let producer = NierProducer::new(config)?;

    // Send a few example messages
    for i in 0..5 {
        let message = OutgoingMessage {
            topic: producer.config().topics.detections.clone(),
            key: Some(format!("event-{}", i)),
            payload: format!("Example detection event {}", i).into_bytes(),
            headers: vec![
                ("message-type".to_string(), "detection_event".to_string()),
                ("correlation-id".to_string(), format!("corr-{}", i)),
            ],
        };

        match producer.send(message).await {
            Ok(result) => {
                info!(
                    "Sent message {} to partition {} at offset {}",
                    i, result.partition, result.offset
                );
            }
            Err(e) => {
                error!("Failed to send message {}: {}", i, e);
            }
        }
    }

    // Flush remaining messages
    producer.flush(std::time::Duration::from_secs(5))?;
    info!("Producer finished");

    Ok(())
}

/// Run in consumer mode - receive and process messages
async fn run_consumer(config: KafkaConfig) -> Result<()> {
    info!("Starting consumer example");

    // Create a producer for the DLQ and for sending alerts
    let producer = Arc::new(NierProducer::new(config.clone())?);

    // Create consumer with DLQ support
    let consumer = ConsumerBuilder::new(&config.bootstrap_servers)
        .group_id(&config.consumer.group_id)
        .client_id("nier-pipeline-example")
        .auto_offset_reset("earliest")
        .enable_auto_commit(false)
        .with_dlq_producer(producer.clone())
        .build()?;

    // Subscribe to detection events
    consumer.subscribe_detections()?;

    // Create the handler
    let handler = Arc::new(DetectionHandler::new(producer));

    // Set up graceful shutdown
    let consumer_ref = &consumer;
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.ok();
        info!("Received Ctrl+C, shutting down...");
        consumer_ref.shutdown();
    });

    // Run the consumer
    consumer.run(handler).await?;

    info!("Consumer finished");
    Ok(())
}

/// Run in both mode - demonstrate full pipeline
async fn run_both(config: KafkaConfig) -> Result<()> {
    info!("Starting full pipeline example");

    // Start consumer in background
    let consumer_config = config.clone();
    let consumer_handle = tokio::spawn(async move {
        if let Err(e) = run_consumer(consumer_config).await {
            error!("Consumer error: {}", e);
        }
    });

    // Give consumer time to start
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    // Run producer
    run_producer(config).await?;

    // Wait a bit for messages to be processed
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;

    // The consumer will keep running until Ctrl+C
    info!("Press Ctrl+C to stop the consumer...");
    consumer_handle.await?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(false)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("Nier Pipeline Example");
    info!("=====================");

    // Load configuration from environment
    let config = KafkaConfig::from_env()?;
    info!("Kafka brokers: {}", config.bootstrap_servers);
    info!("Consumer group: {}", config.consumer.group_id);

    // Parse command line arguments
    let args: Vec<String> = std::env::args().collect();
    let mode = args.get(1).map(|s| s.as_str()).unwrap_or("both");

    match mode {
        "producer" => run_producer(config).await?,
        "consumer" => run_consumer(config).await?,
        "both" => run_both(config).await?,
        _ => {
            println!("Usage: pipeline [producer|consumer|both]");
            println!();
            println!("Modes:");
            println!("  producer - Send example messages to Kafka");
            println!("  consumer - Receive and process messages from Kafka");
            println!("  both     - Run both producer and consumer (default)");
            println!();
            println!("Environment variables:");
            println!("  KAFKA_BOOTSTRAP_SERVERS - Kafka broker addresses (default: localhost:9092)");
            println!("  KAFKA_GROUP_ID          - Consumer group ID (default: nier-pipeline)");
            println!("  KAFKA_CLIENT_ID         - Client ID (default: nier-pipeline)");
            println!("  KAFKA_SECURITY_PROTOCOL - Security protocol (plaintext, ssl, sasl_ssl)");
            println!("  KAFKA_SASL_USERNAME     - SASL username");
            println!("  KAFKA_SASL_PASSWORD     - SASL password");
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_from_env() {
        // Should not panic with default values
        let config = KafkaConfig::from_env();
        assert!(config.is_ok());
    }
}
