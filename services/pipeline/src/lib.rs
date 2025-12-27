//! Nier Pipeline - Kafka message pipeline for factory floor analytics
//!
//! This library provides a high-level interface for producing and consuming
//! messages in the Nier factory floor analytics platform. It handles:
//!
//! - Frame metadata from worker-worn camera glasses
//! - Detection events (PPE violations, activity detections)
//! - Safety alerts and notifications
//!
//! # Example
//!
//! ```rust,no_run
//! use nier_pipeline::{KafkaConfig, NierProducer, NierConsumer};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     // Create a producer
//!     let config = KafkaConfig::from_env()?;
//!     let producer = NierProducer::new(config.clone())?;
//!
//!     // Create a consumer
//!     let consumer = NierConsumer::new(config)?;
//!     consumer.subscribe_detections()?;
//!
//!     Ok(())
//! }
//! ```

pub mod config;
pub mod consumer;
pub mod producer;

// Re-export main types
pub use config::{
    ConfigError, ConsumerConfig, KafkaConfig, ProducerConfig, ReliabilityConfig,
    SaslConfig, SaslMechanism, SecurityProtocol, SslConfig, TopicConfig,
};
pub use consumer::{
    async_trait, ConsumerBuilder, ConsumerError, IncomingMessage, MessageHandler,
    MessageMetadata, NierConsumer,
};
pub use producer::{
    DeliveryResult, NierProducer, OutgoingMessage, ProducerBuilder, ProducerError,
};

/// Prelude module for convenient imports
pub mod prelude {
    pub use crate::config::KafkaConfig;
    pub use crate::consumer::{
        async_trait, ConsumerBuilder, ConsumerError, IncomingMessage, MessageHandler,
        NierConsumer,
    };
    pub use crate::producer::{NierProducer, OutgoingMessage, ProducerBuilder, ProducerError};
}

/// Protocol buffer generated types (when compiled with build.rs)
#[cfg(feature = "proto")]
pub mod proto {
    include!(concat!(env!("OUT_DIR"), "/nier.pipeline.rs"));
}
