//! Kafka configuration module for the Nier pipeline.
//!
//! This module provides configuration structures and utilities for connecting
//! to Kafka brokers with support for SSL/SASL authentication.

use rdkafka::config::ClientConfig;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use thiserror::Error;

/// Errors that can occur during configuration
#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Missing required configuration: {0}")]
    MissingRequired(String),

    #[error("Invalid configuration value for {key}: {message}")]
    InvalidValue { key: String, message: String },

    #[error("Failed to load configuration: {0}")]
    LoadError(String),
}

/// Security protocol for Kafka connections
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SecurityProtocol {
    #[default]
    Plaintext,
    Ssl,
    SaslPlaintext,
    SaslSsl,
}

impl SecurityProtocol {
    pub fn as_str(&self) -> &'static str {
        match self {
            SecurityProtocol::Plaintext => "plaintext",
            SecurityProtocol::Ssl => "ssl",
            SecurityProtocol::SaslPlaintext => "sasl_plaintext",
            SecurityProtocol::SaslSsl => "sasl_ssl",
        }
    }
}

/// SASL mechanism for authentication
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SaslMechanism {
    #[default]
    Plain,
    ScramSha256,
    ScramSha512,
    OAuthBearer,
}

impl SaslMechanism {
    pub fn as_str(&self) -> &'static str {
        match self {
            SaslMechanism::Plain => "PLAIN",
            SaslMechanism::ScramSha256 => "SCRAM-SHA-256",
            SaslMechanism::ScramSha512 => "SCRAM-SHA-512",
            SaslMechanism::OAuthBearer => "OAUTHBEARER",
        }
    }
}

/// SSL/TLS configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SslConfig {
    /// Path to CA certificate file
    pub ca_location: Option<String>,
    /// Path to client certificate file
    pub certificate_location: Option<String>,
    /// Path to client private key file
    pub key_location: Option<String>,
    /// Private key password
    pub key_password: Option<String>,
    /// Enable certificate verification
    #[serde(default = "default_true")]
    pub enable_verification: bool,
}

fn default_true() -> bool {
    true
}

/// SASL authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SaslConfig {
    pub mechanism: SaslMechanism,
    pub username: Option<String>,
    pub password: Option<String>,
    /// OAuth bearer token (for OAuthBearer mechanism)
    pub oauth_token: Option<String>,
}

/// Retry and reliability configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReliabilityConfig {
    /// Number of retries for failed operations
    #[serde(default = "default_retries")]
    pub retries: u32,
    /// Retry backoff in milliseconds
    #[serde(default = "default_retry_backoff_ms")]
    pub retry_backoff_ms: u64,
    /// Request timeout in milliseconds
    #[serde(default = "default_request_timeout_ms")]
    pub request_timeout_ms: u64,
    /// Enable idempotent producer
    #[serde(default = "default_true")]
    pub enable_idempotence: bool,
    /// Required acknowledgments: 0, 1, or -1 (all)
    #[serde(default = "default_acks")]
    pub acks: String,
}

fn default_retries() -> u32 {
    3
}

fn default_retry_backoff_ms() -> u64 {
    100
}

fn default_request_timeout_ms() -> u64 {
    30000
}

fn default_acks() -> String {
    "all".to_string()
}

impl Default for ReliabilityConfig {
    fn default() -> Self {
        Self {
            retries: default_retries(),
            retry_backoff_ms: default_retry_backoff_ms(),
            request_timeout_ms: default_request_timeout_ms(),
            enable_idempotence: true,
            acks: default_acks(),
        }
    }
}

/// Producer-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProducerConfig {
    /// Batch size in bytes
    #[serde(default = "default_batch_size")]
    pub batch_size: usize,
    /// Linger time in milliseconds
    #[serde(default = "default_linger_ms")]
    pub linger_ms: u64,
    /// Compression type: none, gzip, snappy, lz4, zstd
    #[serde(default = "default_compression")]
    pub compression_type: String,
    /// Maximum in-flight requests per connection
    #[serde(default = "default_max_in_flight")]
    pub max_in_flight_requests: u32,
}

fn default_batch_size() -> usize {
    16384
}

fn default_linger_ms() -> u64 {
    5
}

fn default_compression() -> String {
    "lz4".to_string()
}

fn default_max_in_flight() -> u32 {
    5
}

impl Default for ProducerConfig {
    fn default() -> Self {
        Self {
            batch_size: default_batch_size(),
            linger_ms: default_linger_ms(),
            compression_type: default_compression(),
            max_in_flight_requests: default_max_in_flight(),
        }
    }
}

/// Consumer-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsumerConfig {
    /// Consumer group ID
    pub group_id: String,
    /// Auto offset reset: earliest, latest, none
    #[serde(default = "default_auto_offset_reset")]
    pub auto_offset_reset: String,
    /// Enable auto commit
    #[serde(default)]
    pub enable_auto_commit: bool,
    /// Auto commit interval in milliseconds
    #[serde(default = "default_auto_commit_interval")]
    pub auto_commit_interval_ms: u64,
    /// Session timeout in milliseconds
    #[serde(default = "default_session_timeout")]
    pub session_timeout_ms: u64,
    /// Heartbeat interval in milliseconds
    #[serde(default = "default_heartbeat_interval")]
    pub heartbeat_interval_ms: u64,
    /// Maximum poll interval in milliseconds
    #[serde(default = "default_max_poll_interval")]
    pub max_poll_interval_ms: u64,
    /// Maximum records to fetch per poll
    #[serde(default = "default_max_poll_records")]
    pub max_poll_records: u32,
}

fn default_auto_offset_reset() -> String {
    "earliest".to_string()
}

fn default_auto_commit_interval() -> u64 {
    5000
}

fn default_session_timeout() -> u64 {
    30000
}

fn default_heartbeat_interval() -> u64 {
    3000
}

fn default_max_poll_interval() -> u64 {
    300000
}

fn default_max_poll_records() -> u32 {
    500
}

impl Default for ConsumerConfig {
    fn default() -> Self {
        Self {
            group_id: "nier-pipeline".to_string(),
            auto_offset_reset: default_auto_offset_reset(),
            enable_auto_commit: false,
            auto_commit_interval_ms: default_auto_commit_interval(),
            session_timeout_ms: default_session_timeout(),
            heartbeat_interval_ms: default_heartbeat_interval(),
            max_poll_interval_ms: default_max_poll_interval(),
            max_poll_records: default_max_poll_records(),
        }
    }
}

/// Topic configuration for the Nier pipeline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopicConfig {
    /// Topic for raw frame metadata
    #[serde(default = "default_frames_topic")]
    pub frames: String,
    /// Topic for detection events
    #[serde(default = "default_detections_topic")]
    pub detections: String,
    /// Topic for safety alerts
    #[serde(default = "default_alerts_topic")]
    pub alerts: String,
    /// Dead letter queue topic
    #[serde(default = "default_dlq_topic")]
    pub dead_letter_queue: String,
}

fn default_frames_topic() -> String {
    "nier.frames".to_string()
}

fn default_detections_topic() -> String {
    "nier.detections".to_string()
}

fn default_alerts_topic() -> String {
    "nier.alerts".to_string()
}

fn default_dlq_topic() -> String {
    "nier.dlq".to_string()
}

impl Default for TopicConfig {
    fn default() -> Self {
        Self {
            frames: default_frames_topic(),
            detections: default_detections_topic(),
            alerts: default_alerts_topic(),
            dead_letter_queue: default_dlq_topic(),
        }
    }
}

/// Main Kafka configuration for the Nier pipeline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KafkaConfig {
    /// Comma-separated list of broker addresses
    pub bootstrap_servers: String,
    /// Client ID for this connection
    #[serde(default = "default_client_id")]
    pub client_id: String,
    /// Security protocol
    #[serde(default)]
    pub security_protocol: SecurityProtocol,
    /// SSL configuration
    #[serde(default)]
    pub ssl: SslConfig,
    /// SASL configuration
    #[serde(default)]
    pub sasl: SaslConfig,
    /// Reliability settings
    #[serde(default)]
    pub reliability: ReliabilityConfig,
    /// Producer settings
    #[serde(default)]
    pub producer: ProducerConfig,
    /// Consumer settings
    #[serde(default)]
    pub consumer: ConsumerConfig,
    /// Topic configuration
    #[serde(default)]
    pub topics: TopicConfig,
    /// Additional Kafka properties
    #[serde(default)]
    pub extra_properties: HashMap<String, String>,
}

fn default_client_id() -> String {
    "nier-pipeline".to_string()
}

impl Default for KafkaConfig {
    fn default() -> Self {
        Self {
            bootstrap_servers: "localhost:9092".to_string(),
            client_id: default_client_id(),
            security_protocol: SecurityProtocol::default(),
            ssl: SslConfig::default(),
            sasl: SaslConfig::default(),
            reliability: ReliabilityConfig::default(),
            producer: ProducerConfig::default(),
            consumer: ConsumerConfig::default(),
            topics: TopicConfig::default(),
            extra_properties: HashMap::new(),
        }
    }
}

impl KafkaConfig {
    /// Create a new KafkaConfig with the specified bootstrap servers
    pub fn new(bootstrap_servers: impl Into<String>) -> Self {
        Self {
            bootstrap_servers: bootstrap_servers.into(),
            ..Default::default()
        }
    }

    /// Load configuration from environment variables
    pub fn from_env() -> Result<Self, ConfigError> {
        let bootstrap_servers = std::env::var("KAFKA_BOOTSTRAP_SERVERS")
            .unwrap_or_else(|_| "localhost:9092".to_string());

        let mut config = Self::new(bootstrap_servers);

        // Load optional environment variables
        if let Ok(client_id) = std::env::var("KAFKA_CLIENT_ID") {
            config.client_id = client_id;
        }

        if let Ok(group_id) = std::env::var("KAFKA_GROUP_ID") {
            config.consumer.group_id = group_id;
        }

        if let Ok(protocol) = std::env::var("KAFKA_SECURITY_PROTOCOL") {
            config.security_protocol = match protocol.to_lowercase().as_str() {
                "ssl" => SecurityProtocol::Ssl,
                "sasl_plaintext" => SecurityProtocol::SaslPlaintext,
                "sasl_ssl" => SecurityProtocol::SaslSsl,
                _ => SecurityProtocol::Plaintext,
            };
        }

        // Load SASL credentials
        if let Ok(username) = std::env::var("KAFKA_SASL_USERNAME") {
            config.sasl.username = Some(username);
        }
        if let Ok(password) = std::env::var("KAFKA_SASL_PASSWORD") {
            config.sasl.password = Some(password);
        }

        // Load SSL paths
        if let Ok(ca) = std::env::var("KAFKA_SSL_CA_LOCATION") {
            config.ssl.ca_location = Some(ca);
        }

        Ok(config)
    }

    /// Build a base rdkafka ClientConfig from this configuration
    fn build_base_config(&self) -> ClientConfig {
        let mut config = ClientConfig::new();

        config.set("bootstrap.servers", &self.bootstrap_servers);
        config.set("client.id", &self.client_id);
        config.set("security.protocol", self.security_protocol.as_str());

        // SSL configuration
        if let Some(ref ca) = self.ssl.ca_location {
            config.set("ssl.ca.location", ca);
        }
        if let Some(ref cert) = self.ssl.certificate_location {
            config.set("ssl.certificate.location", cert);
        }
        if let Some(ref key) = self.ssl.key_location {
            config.set("ssl.key.location", key);
        }
        if let Some(ref password) = self.ssl.key_password {
            config.set("ssl.key.password", password);
        }
        if !self.ssl.enable_verification {
            config.set("enable.ssl.certificate.verification", "false");
        }

        // SASL configuration
        config.set("sasl.mechanism", self.sasl.mechanism.as_str());
        if let Some(ref username) = self.sasl.username {
            config.set("sasl.username", username);
        }
        if let Some(ref password) = self.sasl.password {
            config.set("sasl.password", password);
        }

        // Extra properties
        for (key, value) in &self.extra_properties {
            config.set(key, value);
        }

        config
    }

    /// Build a producer ClientConfig
    pub fn build_producer_config(&self) -> ClientConfig {
        let mut config = self.build_base_config();

        // Reliability settings
        config.set("retries", self.reliability.retries.to_string());
        config.set("retry.backoff.ms", self.reliability.retry_backoff_ms.to_string());
        config.set("request.timeout.ms", self.reliability.request_timeout_ms.to_string());
        config.set("acks", &self.reliability.acks);

        if self.reliability.enable_idempotence {
            config.set("enable.idempotence", "true");
        }

        // Producer settings
        config.set("batch.size", self.producer.batch_size.to_string());
        config.set("linger.ms", self.producer.linger_ms.to_string());
        config.set("compression.type", &self.producer.compression_type);
        config.set(
            "max.in.flight.requests.per.connection",
            self.producer.max_in_flight_requests.to_string(),
        );

        config
    }

    /// Build a consumer ClientConfig
    pub fn build_consumer_config(&self) -> ClientConfig {
        let mut config = self.build_base_config();

        // Consumer settings
        config.set("group.id", &self.consumer.group_id);
        config.set("auto.offset.reset", &self.consumer.auto_offset_reset);
        config.set(
            "enable.auto.commit",
            self.consumer.enable_auto_commit.to_string(),
        );
        config.set(
            "auto.commit.interval.ms",
            self.consumer.auto_commit_interval_ms.to_string(),
        );
        config.set(
            "session.timeout.ms",
            self.consumer.session_timeout_ms.to_string(),
        );
        config.set(
            "heartbeat.interval.ms",
            self.consumer.heartbeat_interval_ms.to_string(),
        );
        config.set(
            "max.poll.interval.ms",
            self.consumer.max_poll_interval_ms.to_string(),
        );

        config
    }

    /// Get request timeout as Duration
    pub fn request_timeout(&self) -> Duration {
        Duration::from_millis(self.reliability.request_timeout_ms)
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.bootstrap_servers.is_empty() {
            return Err(ConfigError::MissingRequired(
                "bootstrap_servers".to_string(),
            ));
        }

        if self.consumer.group_id.is_empty() {
            return Err(ConfigError::MissingRequired(
                "consumer.group_id".to_string(),
            ));
        }

        // Validate SASL config if using SASL
        match self.security_protocol {
            SecurityProtocol::SaslPlaintext | SecurityProtocol::SaslSsl => {
                if self.sasl.username.is_none() {
                    return Err(ConfigError::MissingRequired(
                        "sasl.username (required for SASL)".to_string(),
                    ));
                }
            }
            _ => {}
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = KafkaConfig::default();
        assert_eq!(config.bootstrap_servers, "localhost:9092");
        assert_eq!(config.client_id, "nier-pipeline");
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_producer_config_build() {
        let config = KafkaConfig::new("localhost:9092");
        let producer_config = config.build_producer_config();

        // Verify key settings are present
        assert!(producer_config.get("bootstrap.servers").is_some());
        assert!(producer_config.get("acks").is_some());
    }

    #[test]
    fn test_consumer_config_build() {
        let config = KafkaConfig::new("localhost:9092");
        let consumer_config = config.build_consumer_config();

        // Verify key settings are present
        assert!(consumer_config.get("bootstrap.servers").is_some());
        assert!(consumer_config.get("group.id").is_some());
    }
}
