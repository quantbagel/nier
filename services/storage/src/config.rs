use serde::Deserialize;
use std::time::Duration;

/// Main configuration for the storage service
#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    /// Service configuration
    pub service: ServiceConfig,
    /// Kafka configuration
    pub kafka: KafkaConfig,
    /// S3 configuration
    pub s3: S3Config,
    /// Database configuration
    pub database: DatabaseConfig,
    /// Frame selection configuration
    pub frame_selection: FrameSelectionConfig,
    /// API configuration
    pub api: ApiConfig,
}

/// Service-level configuration
#[derive(Debug, Clone, Deserialize)]
pub struct ServiceConfig {
    /// Service name for logging/metrics
    #[serde(default = "default_service_name")]
    pub name: String,
    /// Log level (trace, debug, info, warn, error)
    #[serde(default = "default_log_level")]
    pub log_level: String,
    /// Metrics port
    #[serde(default = "default_metrics_port")]
    pub metrics_port: u16,
}

/// Kafka consumer configuration
#[derive(Debug, Clone, Deserialize)]
pub struct KafkaConfig {
    /// Kafka bootstrap servers
    pub bootstrap_servers: String,
    /// Consumer group ID
    #[serde(default = "default_consumer_group")]
    pub consumer_group: String,
    /// Topic for storage trigger events
    #[serde(default = "default_storage_topic")]
    pub storage_trigger_topic: String,
    /// Topic for detection events (used for frame selection)
    #[serde(default = "default_detections_topic")]
    pub detections_topic: String,
    /// Enable SSL
    #[serde(default)]
    pub ssl_enabled: bool,
    /// SSL CA certificate path
    pub ssl_ca_location: Option<String>,
    /// SASL username
    pub sasl_username: Option<String>,
    /// SASL password
    pub sasl_password: Option<String>,
    /// Auto offset reset policy
    #[serde(default = "default_auto_offset_reset")]
    pub auto_offset_reset: String,
    /// Session timeout in milliseconds
    #[serde(default = "default_session_timeout_ms")]
    pub session_timeout_ms: u32,
    /// Max poll interval in milliseconds
    #[serde(default = "default_max_poll_interval_ms")]
    pub max_poll_interval_ms: u32,
}

/// S3 storage configuration
#[derive(Debug, Clone, Deserialize)]
pub struct S3Config {
    /// S3 bucket name for frame storage
    pub bucket: String,
    /// AWS region
    #[serde(default = "default_region")]
    pub region: String,
    /// Custom endpoint URL (for MinIO, LocalStack, etc.)
    pub endpoint_url: Option<String>,
    /// Force path-style access (required for MinIO)
    #[serde(default)]
    pub force_path_style: bool,
    /// Presigned URL expiration in seconds
    #[serde(default = "default_presigned_url_expiry_secs")]
    pub presigned_url_expiry_secs: u64,
    /// Upload concurrency limit
    #[serde(default = "default_upload_concurrency")]
    pub upload_concurrency: usize,
    /// Multipart upload threshold in bytes (5MB default)
    #[serde(default = "default_multipart_threshold")]
    pub multipart_threshold_bytes: usize,
    /// Part size for multipart uploads in bytes (5MB default)
    #[serde(default = "default_part_size")]
    pub part_size_bytes: usize,
}

/// Database configuration
#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    /// PostgreSQL connection URL
    pub url: String,
    /// Maximum number of connections in the pool
    #[serde(default = "default_max_connections")]
    pub max_connections: u32,
    /// Minimum number of connections in the pool
    #[serde(default = "default_min_connections")]
    pub min_connections: u32,
    /// Connection timeout in seconds
    #[serde(default = "default_connect_timeout_secs")]
    pub connect_timeout_secs: u64,
    /// Idle connection timeout in seconds
    #[serde(default = "default_idle_timeout_secs")]
    pub idle_timeout_secs: u64,
    /// Run migrations on startup
    #[serde(default = "default_run_migrations")]
    pub run_migrations: bool,
}

/// Frame selection configuration
#[derive(Debug, Clone, Deserialize)]
pub struct FrameSelectionConfig {
    /// Store frames with detections
    #[serde(default = "default_true")]
    pub store_detections: bool,
    /// Store periodic sample frames (even without detections)
    #[serde(default = "default_true")]
    pub store_samples: bool,
    /// Sample rate: store 1 frame every N frames when no detections
    #[serde(default = "default_sample_rate")]
    pub sample_rate: u32,
    /// Store frames marked for debug
    #[serde(default = "default_true")]
    pub store_debug: bool,
    /// Minimum confidence threshold for storing detection frames
    #[serde(default = "default_min_confidence")]
    pub min_confidence: f32,
    /// Detection types to store (empty = all)
    #[serde(default)]
    pub detection_types: Vec<String>,
    /// Maximum frame age in seconds (reject frames older than this)
    #[serde(default = "default_max_frame_age_secs")]
    pub max_frame_age_secs: u64,
}

/// API configuration for presigned URL endpoint
#[derive(Debug, Clone, Deserialize)]
pub struct ApiConfig {
    /// API listen address
    #[serde(default = "default_api_host")]
    pub host: String,
    /// API listen port
    #[serde(default = "default_api_port")]
    pub port: u16,
    /// Enable CORS
    #[serde(default = "default_true")]
    pub cors_enabled: bool,
    /// Allowed CORS origins
    #[serde(default)]
    pub cors_origins: Vec<String>,
}

// Default value functions
fn default_service_name() -> String {
    "storage-service".to_string()
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_metrics_port() -> u16 {
    9090
}

fn default_consumer_group() -> String {
    "storage-service".to_string()
}

fn default_storage_topic() -> String {
    "nier.storage.triggers".to_string()
}

fn default_detections_topic() -> String {
    "nier.detections".to_string()
}

fn default_auto_offset_reset() -> String {
    "earliest".to_string()
}

fn default_session_timeout_ms() -> u32 {
    30000
}

fn default_max_poll_interval_ms() -> u32 {
    300000
}

fn default_region() -> String {
    "us-east-1".to_string()
}

fn default_presigned_url_expiry_secs() -> u64 {
    3600
}

fn default_upload_concurrency() -> usize {
    10
}

fn default_multipart_threshold() -> usize {
    5 * 1024 * 1024 // 5MB
}

fn default_part_size() -> usize {
    5 * 1024 * 1024 // 5MB
}

fn default_max_connections() -> u32 {
    10
}

fn default_min_connections() -> u32 {
    2
}

fn default_connect_timeout_secs() -> u64 {
    30
}

fn default_idle_timeout_secs() -> u64 {
    600
}

fn default_run_migrations() -> bool {
    true
}

fn default_true() -> bool {
    true
}

fn default_sample_rate() -> u32 {
    30 // Store 1 frame per second at 30fps
}

fn default_min_confidence() -> f32 {
    0.5
}

fn default_max_frame_age_secs() -> u64 {
    300 // 5 minutes
}

fn default_api_host() -> String {
    "0.0.0.0".to_string()
}

fn default_api_port() -> u16 {
    8080
}

impl Config {
    /// Load configuration from environment and config files
    pub fn load() -> anyhow::Result<Self> {
        let config = config::Config::builder()
            // Start with default values
            .set_default("service.name", "storage-service")?
            .set_default("service.log_level", "info")?
            .set_default("service.metrics_port", 9090)?
            // Add config file if present
            .add_source(
                config::File::with_name("config/storage")
                    .required(false)
            )
            .add_source(
                config::File::with_name("/etc/nier/storage")
                    .required(false)
            )
            // Override with environment variables
            // STORAGE__KAFKA__BOOTSTRAP_SERVERS -> kafka.bootstrap_servers
            .add_source(
                config::Environment::with_prefix("STORAGE")
                    .separator("__")
                    .try_parsing(true)
            )
            .build()?;

        config.try_deserialize().map_err(Into::into)
    }

    /// Get database connection timeout as Duration
    pub fn db_connect_timeout(&self) -> Duration {
        Duration::from_secs(self.database.connect_timeout_secs)
    }

    /// Get database idle timeout as Duration
    pub fn db_idle_timeout(&self) -> Duration {
        Duration::from_secs(self.database.idle_timeout_secs)
    }

    /// Get presigned URL expiry as Duration
    pub fn presigned_url_expiry(&self) -> Duration {
        Duration::from_secs(self.s3.presigned_url_expiry_secs)
    }

    /// Get maximum frame age as Duration
    pub fn max_frame_age(&self) -> Duration {
        Duration::from_secs(self.frame_selection.max_frame_age_secs)
    }
}

impl Default for ServiceConfig {
    fn default() -> Self {
        Self {
            name: default_service_name(),
            log_level: default_log_level(),
            metrics_port: default_metrics_port(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_values() {
        assert_eq!(default_sample_rate(), 30);
        assert_eq!(default_min_confidence(), 0.5);
        assert_eq!(default_presigned_url_expiry_secs(), 3600);
    }
}
