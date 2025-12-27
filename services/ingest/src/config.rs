//! Configuration management for the RTSP ingest service.
//!
//! This module handles loading and validating configuration from environment
//! variables and configuration files.

use config::{Config, ConfigError, Environment, File};
use serde::Deserialize;
use std::time::Duration;

/// Main configuration for the ingest service.
#[derive(Debug, Clone, Deserialize)]
pub struct IngestConfig {
    /// RTSP stream configuration
    pub rtsp: RtspConfig,

    /// Frame processing configuration
    pub processing: ProcessingConfig,

    /// gRPC client configuration
    pub grpc: GrpcConfig,

    /// Logging configuration
    #[serde(default)]
    pub logging: LoggingConfig,

    /// Health check configuration
    #[serde(default)]
    pub health: HealthConfig,
}

/// RTSP stream connection configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct RtspConfig {
    /// RTSP stream URL (e.g., "rtsp://camera:554/stream")
    pub url: String,

    /// Device identifier for this camera
    pub device_id: String,

    /// Worker identifier associated with this device
    #[serde(default)]
    pub worker_id: Option<String>,

    /// Factory zone identifier
    #[serde(default)]
    pub zone_id: Option<String>,

    /// Connection timeout in seconds
    #[serde(default = "default_connection_timeout")]
    pub connection_timeout_secs: u64,

    /// Maximum number of reconnection attempts (0 = infinite)
    #[serde(default = "default_max_reconnect_attempts")]
    pub max_reconnect_attempts: u32,

    /// Base delay between reconnection attempts in milliseconds
    #[serde(default = "default_reconnect_base_delay_ms")]
    pub reconnect_base_delay_ms: u64,

    /// Maximum delay between reconnection attempts in milliseconds
    #[serde(default = "default_reconnect_max_delay_ms")]
    pub reconnect_max_delay_ms: u64,

    /// RTSP transport protocol (tcp, udp, or udp-mcast)
    #[serde(default = "default_transport")]
    pub transport: String,

    /// Buffer size for RTSP stream in milliseconds
    #[serde(default = "default_buffer_ms")]
    pub buffer_ms: u32,
}

/// Frame processing configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct ProcessingConfig {
    /// Target width for preprocessed frames
    #[serde(default = "default_target_width")]
    pub target_width: u32,

    /// Target height for preprocessed frames
    #[serde(default = "default_target_height")]
    pub target_height: u32,

    /// Target frames per second (frame decimation)
    #[serde(default = "default_target_fps")]
    pub target_fps: f32,

    /// Output pixel format
    #[serde(default = "default_pixel_format")]
    pub pixel_format: String,

    /// Maximum queue size for frames pending processing
    #[serde(default = "default_queue_size")]
    pub queue_size: usize,

    /// Number of worker threads for frame processing
    #[serde(default = "default_num_workers")]
    pub num_workers: usize,

    /// Whether to drop frames when queue is full
    #[serde(default = "default_drop_on_backpressure")]
    pub drop_on_backpressure: bool,
}

/// gRPC client configuration for inference service.
#[derive(Debug, Clone, Deserialize)]
pub struct GrpcConfig {
    /// Inference service endpoint
    pub inference_endpoint: String,

    /// Request timeout in seconds
    #[serde(default = "default_request_timeout")]
    pub request_timeout_secs: u64,

    /// Connection timeout in seconds
    #[serde(default = "default_grpc_connection_timeout")]
    pub connection_timeout_secs: u64,

    /// Maximum concurrent requests
    #[serde(default = "default_max_concurrent_requests")]
    pub max_concurrent_requests: usize,

    /// Whether to use TLS
    #[serde(default)]
    pub use_tls: bool,

    /// Path to CA certificate (if using TLS)
    pub ca_cert_path: Option<String>,

    /// Enable gRPC compression
    #[serde(default)]
    pub enable_compression: bool,

    /// Batch size for frame submission
    #[serde(default = "default_batch_size")]
    pub batch_size: usize,

    /// Maximum time to wait for batch to fill in milliseconds
    #[serde(default = "default_batch_timeout_ms")]
    pub batch_timeout_ms: u64,
}

/// Logging configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct LoggingConfig {
    /// Log level (trace, debug, info, warn, error)
    #[serde(default = "default_log_level")]
    pub level: String,

    /// Output format (json, pretty)
    #[serde(default = "default_log_format")]
    pub format: String,

    /// Whether to include source code location
    #[serde(default)]
    pub include_location: bool,
}

/// Health check configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct HealthConfig {
    /// Interval between health checks in seconds
    #[serde(default = "default_health_interval")]
    pub interval_secs: u64,

    /// Port for health check HTTP server
    #[serde(default = "default_health_port")]
    pub port: u16,

    /// Enable Prometheus metrics export
    #[serde(default)]
    pub enable_metrics: bool,
}

// Default value functions
fn default_connection_timeout() -> u64 {
    10
}
fn default_max_reconnect_attempts() -> u32 {
    0
}
fn default_reconnect_base_delay_ms() -> u64 {
    1000
}
fn default_reconnect_max_delay_ms() -> u64 {
    30000
}
fn default_transport() -> String {
    "tcp".to_string()
}
fn default_buffer_ms() -> u32 {
    200
}
fn default_target_width() -> u32 {
    640
}
fn default_target_height() -> u32 {
    480
}
fn default_target_fps() -> f32 {
    10.0
}
fn default_pixel_format() -> String {
    "RGB".to_string()
}
fn default_queue_size() -> usize {
    100
}
fn default_num_workers() -> usize {
    2
}
fn default_drop_on_backpressure() -> bool {
    true
}
fn default_request_timeout() -> u64 {
    30
}
fn default_grpc_connection_timeout() -> u64 {
    10
}
fn default_max_concurrent_requests() -> usize {
    10
}
fn default_batch_size() -> usize {
    1
}
fn default_batch_timeout_ms() -> u64 {
    100
}
fn default_log_level() -> String {
    "info".to_string()
}
fn default_log_format() -> String {
    "json".to_string()
}
fn default_health_interval() -> u64 {
    30
}
fn default_health_port() -> u16 {
    8080
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
            format: default_log_format(),
            include_location: false,
        }
    }
}

impl Default for HealthConfig {
    fn default() -> Self {
        Self {
            interval_secs: default_health_interval(),
            port: default_health_port(),
            enable_metrics: false,
        }
    }
}

impl IngestConfig {
    /// Load configuration from file and environment variables.
    ///
    /// Configuration is loaded in the following order (later sources override earlier):
    /// 1. Default config file (config/default.toml)
    /// 2. Environment-specific config (config/{env}.toml)
    /// 3. Environment variables (prefixed with INGEST_)
    pub fn load() -> Result<Self, ConfigError> {
        let run_mode = std::env::var("RUN_MODE").unwrap_or_else(|_| "development".into());

        let config = Config::builder()
            // Start with default config
            .add_source(File::with_name("config/default").required(false))
            // Add environment-specific config
            .add_source(File::with_name(&format!("config/{}", run_mode)).required(false))
            // Override with environment variables (e.g., INGEST_RTSP__URL)
            .add_source(
                Environment::with_prefix("INGEST")
                    .separator("__")
                    .try_parsing(true),
            )
            .build()?;

        config.try_deserialize()
    }

    /// Create configuration from environment variables only.
    pub fn from_env() -> Result<Self, ConfigError> {
        let config = Config::builder()
            .add_source(
                Environment::with_prefix("INGEST")
                    .separator("__")
                    .try_parsing(true),
            )
            .build()?;

        config.try_deserialize()
    }

    /// Validate the configuration.
    pub fn validate(&self) -> Result<(), ConfigValidationError> {
        // Validate RTSP URL
        if self.rtsp.url.is_empty() {
            return Err(ConfigValidationError::MissingField("rtsp.url".to_string()));
        }
        if !self.rtsp.url.starts_with("rtsp://") && !self.rtsp.url.starts_with("rtsps://") {
            return Err(ConfigValidationError::InvalidValue {
                field: "rtsp.url".to_string(),
                message: "URL must start with rtsp:// or rtsps://".to_string(),
            });
        }

        // Validate device ID
        if self.rtsp.device_id.is_empty() {
            return Err(ConfigValidationError::MissingField(
                "rtsp.device_id".to_string(),
            ));
        }

        // Validate processing config
        if self.processing.target_width == 0 || self.processing.target_height == 0 {
            return Err(ConfigValidationError::InvalidValue {
                field: "processing.target_width/height".to_string(),
                message: "Dimensions must be greater than 0".to_string(),
            });
        }

        if self.processing.target_fps <= 0.0 {
            return Err(ConfigValidationError::InvalidValue {
                field: "processing.target_fps".to_string(),
                message: "FPS must be greater than 0".to_string(),
            });
        }

        // Validate gRPC config
        if self.grpc.inference_endpoint.is_empty() {
            return Err(ConfigValidationError::MissingField(
                "grpc.inference_endpoint".to_string(),
            ));
        }

        Ok(())
    }
}

impl RtspConfig {
    /// Get connection timeout as Duration.
    pub fn connection_timeout(&self) -> Duration {
        Duration::from_secs(self.connection_timeout_secs)
    }

    /// Get base reconnection delay as Duration.
    pub fn reconnect_base_delay(&self) -> Duration {
        Duration::from_millis(self.reconnect_base_delay_ms)
    }

    /// Get maximum reconnection delay as Duration.
    pub fn reconnect_max_delay(&self) -> Duration {
        Duration::from_millis(self.reconnect_max_delay_ms)
    }
}

impl GrpcConfig {
    /// Get request timeout as Duration.
    pub fn request_timeout(&self) -> Duration {
        Duration::from_secs(self.request_timeout_secs)
    }

    /// Get connection timeout as Duration.
    pub fn connection_timeout(&self) -> Duration {
        Duration::from_secs(self.connection_timeout_secs)
    }

    /// Get batch timeout as Duration.
    pub fn batch_timeout(&self) -> Duration {
        Duration::from_millis(self.batch_timeout_ms)
    }
}

/// Configuration validation errors.
#[derive(Debug, thiserror::Error)]
pub enum ConfigValidationError {
    #[error("Missing required field: {0}")]
    MissingField(String),

    #[error("Invalid value for {field}: {message}")]
    InvalidValue { field: String, message: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_config() -> IngestConfig {
        IngestConfig {
            rtsp: RtspConfig {
                url: "rtsp://camera:554/stream".to_string(),
                device_id: "camera-001".to_string(),
                worker_id: Some("worker-123".to_string()),
                zone_id: Some("zone-a".to_string()),
                connection_timeout_secs: 10,
                max_reconnect_attempts: 5,
                reconnect_base_delay_ms: 1000,
                reconnect_max_delay_ms: 30000,
                transport: "tcp".to_string(),
                buffer_ms: 200,
            },
            processing: ProcessingConfig {
                target_width: 640,
                target_height: 480,
                target_fps: 10.0,
                pixel_format: "RGB".to_string(),
                queue_size: 100,
                num_workers: 2,
                drop_on_backpressure: true,
            },
            grpc: GrpcConfig {
                inference_endpoint: "http://inference:50051".to_string(),
                request_timeout_secs: 30,
                connection_timeout_secs: 10,
                max_concurrent_requests: 10,
                use_tls: false,
                ca_cert_path: None,
                enable_compression: false,
                batch_size: 1,
                batch_timeout_ms: 100,
            },
            logging: LoggingConfig::default(),
            health: HealthConfig::default(),
        }
    }

    #[test]
    fn test_valid_config() {
        let config = create_test_config();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_missing_rtsp_url() {
        let mut config = create_test_config();
        config.rtsp.url = String::new();
        assert!(matches!(
            config.validate(),
            Err(ConfigValidationError::MissingField(_))
        ));
    }

    #[test]
    fn test_invalid_rtsp_url() {
        let mut config = create_test_config();
        config.rtsp.url = "http://camera:554/stream".to_string();
        assert!(matches!(
            config.validate(),
            Err(ConfigValidationError::InvalidValue { .. })
        ));
    }

    #[test]
    fn test_missing_device_id() {
        let mut config = create_test_config();
        config.rtsp.device_id = String::new();
        assert!(matches!(
            config.validate(),
            Err(ConfigValidationError::MissingField(_))
        ));
    }

    #[test]
    fn test_invalid_dimensions() {
        let mut config = create_test_config();
        config.processing.target_width = 0;
        assert!(matches!(
            config.validate(),
            Err(ConfigValidationError::InvalidValue { .. })
        ));
    }
}
