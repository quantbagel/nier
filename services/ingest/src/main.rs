//! RTSP Ingest Service for Nier Factory Floor Analytics
//!
//! This service captures video streams from worker-worn camera glasses via RTSP,
//! processes the frames, and sends them to the inference service for analysis.
//!
//! # Architecture
//!
//! ```text
//! RTSP Stream -> RtspClient -> FrameProcessor -> GrpcClient -> Inference Service
//! ```
//!
//! # Configuration
//!
//! Configuration is loaded from:
//! 1. Configuration files (config/default.toml, config/{env}.toml)
//! 2. Environment variables (prefixed with INGEST_)
//!
//! See `config.rs` for detailed configuration options.

mod config;
mod frame_processor;
mod grpc_client;
mod rtsp_client;

use config::IngestConfig;
use frame_processor::{FrameProcessor, ProcessedFrame};
use grpc_client::{BatchingClient, InferenceClient, InferenceGrpcClient};
use rtsp_client::RtspClient;

use parking_lot::RwLock;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::signal;
use tokio::sync::mpsc;
use tracing::{error, info, warn, Level};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

/// Application state and shared resources.
struct AppState {
    config: IngestConfig,
    running: Arc<AtomicBool>,
    rtsp_client: Option<Arc<RwLock<RtspClient>>>,
    grpc_client: Option<Arc<InferenceGrpcClient>>,
}

impl AppState {
    fn new(config: IngestConfig) -> Self {
        Self {
            config,
            running: Arc::new(AtomicBool::new(false)),
            rtsp_client: None,
            grpc_client: None,
        }
    }

    fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    fn shutdown(&self) {
        self.running.store(false, Ordering::SeqCst);
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load configuration
    let config = load_config()?;

    // Initialize logging
    init_logging(&config.logging)?;

    info!(
        service = "nier-ingest",
        version = env!("CARGO_PKG_VERSION"),
        device_id = %config.rtsp.device_id,
        "Starting RTSP ingest service"
    );

    // Validate configuration
    config.validate()?;

    // Create application state
    let state = Arc::new(RwLock::new(AppState::new(config.clone())));
    state.write().running.store(true, Ordering::SeqCst);

    // Run the main pipeline
    let result = run_pipeline(state.clone()).await;

    // Handle result
    match result {
        Ok(()) => {
            info!("Ingest service completed successfully");
        }
        Err(e) => {
            error!(error = %e, "Ingest service failed");
            return Err(e);
        }
    }

    Ok(())
}

/// Load and validate configuration.
fn load_config() -> anyhow::Result<IngestConfig> {
    // Try loading from files first, fall back to environment
    let config = IngestConfig::load().or_else(|e| {
        warn!(error = %e, "Failed to load config from files, trying environment");
        IngestConfig::from_env()
    })?;

    Ok(config)
}

/// Initialize the tracing/logging subsystem.
fn init_logging(config: &config::LoggingConfig) -> anyhow::Result<()> {
    let level = match config.level.to_lowercase().as_str() {
        "trace" => Level::TRACE,
        "debug" => Level::DEBUG,
        "info" => Level::INFO,
        "warn" => Level::WARN,
        "error" => Level::ERROR,
        _ => Level::INFO,
    };

    let filter = EnvFilter::from_default_env()
        .add_directive(format!("nier_ingest={}", level).parse()?)
        .add_directive("gstreamer=warn".parse()?)
        .add_directive("tonic=info".parse()?);

    let subscriber = tracing_subscriber::registry().with(filter);

    if config.format == "json" {
        subscriber.with(fmt::layer().json()).init();
    } else {
        subscriber.with(fmt::layer().pretty()).init();
    }

    Ok(())
}

/// Run the main ingest pipeline.
async fn run_pipeline(state: Arc<RwLock<AppState>>) -> anyhow::Result<()> {
    let config = state.read().config.clone();

    // Create RTSP client
    let mut rtsp_client = RtspClient::new(config.rtsp.clone())?;

    // Create gRPC client
    let grpc_client = Arc::new(InferenceGrpcClient::new(config.grpc.clone()));

    // Connect to inference service
    info!("Connecting to inference service...");
    grpc_client.connect_with_retry().await?;

    // Start RTSP stream
    info!(
        url = %config.rtsp.url,
        device_id = %config.rtsp.device_id,
        "Starting RTSP stream..."
    );
    let raw_frame_rx = rtsp_client.start().await?;

    // Store clients in state
    {
        let mut state_guard = state.write();
        state_guard.rtsp_client = Some(Arc::new(RwLock::new(rtsp_client)));
        state_guard.grpc_client = Some(grpc_client.clone());
    }

    // Create channels for the pipeline
    let (processed_tx, processed_rx) = mpsc::channel::<ProcessedFrame>(config.processing.queue_size);

    // Create frame processor
    let processor = FrameProcessor::new(
        config.processing.clone(),
        config.rtsp.device_id.clone(),
    );

    // Create batching client
    let batching_client = BatchingClient::new(grpc_client.clone(), config.grpc.clone());

    // Spawn the frame processor task
    let processor_handle = tokio::spawn({
        let state = state.clone();
        async move {
            processor.run(raw_frame_rx, processed_tx).await;
            info!("Frame processor task completed");
        }
    });

    // Spawn the batching client task
    let client_handle = tokio::spawn({
        let state = state.clone();
        async move {
            batching_client.run(processed_rx).await;
            info!("Batching client task completed");
        }
    });

    // Spawn the health monitoring task
    let health_handle = tokio::spawn({
        let state = state.clone();
        let grpc_client = grpc_client.clone();
        let device_id = config.rtsp.device_id.clone();
        let interval = std::time::Duration::from_secs(config.health.interval_secs);

        async move {
            run_health_monitor(state, grpc_client, device_id, interval).await;
        }
    });

    // Wait for shutdown signal
    let shutdown_signal = async {
        signal::ctrl_c().await.expect("Failed to listen for Ctrl+C");
        info!("Received shutdown signal");
    };

    tokio::select! {
        _ = shutdown_signal => {
            info!("Initiating graceful shutdown...");
        }
        _ = processor_handle => {
            warn!("Processor task exited unexpectedly");
        }
        _ = client_handle => {
            warn!("Client task exited unexpectedly");
        }
    }

    // Trigger shutdown
    state.write().shutdown();

    // Stop health monitor
    health_handle.abort();

    // Stop RTSP client
    if let Some(rtsp) = &state.read().rtsp_client {
        rtsp.write().stop().await;
    }

    // Disconnect gRPC client
    if let Some(client) = &state.read().grpc_client {
        client.disconnect().await;
    }

    // Log final statistics
    log_final_stats(&state.read());

    info!("Shutdown complete");
    Ok(())
}

/// Run the health monitoring loop.
async fn run_health_monitor(
    state: Arc<RwLock<AppState>>,
    grpc_client: Arc<InferenceGrpcClient>,
    device_id: String,
    interval: std::time::Duration,
) {
    let mut ticker = tokio::time::interval(interval);

    while state.read().is_running() {
        ticker.tick().await;

        // Check gRPC health
        match grpc_client.health_check(&device_id).await {
            Ok(healthy) => {
                if !healthy {
                    warn!("Inference service reported unhealthy");
                }
            }
            Err(e) => {
                error!(error = %e, "Health check failed");
            }
        }

        // Log stats
        if let Some(rtsp) = &state.read().rtsp_client {
            let rtsp_stats = rtsp.read().stats();
            info!(
                frames_received = rtsp_stats.frames_received,
                frames_dropped = rtsp_stats.frames_dropped,
                fps = format!("{:.2}", rtsp_stats.current_fps),
                reconnects = rtsp_stats.reconnect_count,
                "RTSP stream stats"
            );
        }

        let grpc_stats = grpc_client.stats();
        info!(
            frames_sent = grpc_stats.frames_sent,
            frames_accepted = grpc_stats.frames_accepted,
            frames_rejected = grpc_stats.frames_rejected,
            avg_latency_ms = format!("{:.2}", grpc_stats.avg_latency_ms),
            "gRPC client stats"
        );
    }
}

/// Log final statistics on shutdown.
fn log_final_stats(state: &AppState) {
    info!("=== Final Statistics ===");

    if let Some(rtsp) = &state.rtsp_client {
        let stats = rtsp.read().stats();
        info!(
            frames_received = stats.frames_received,
            frames_dropped = stats.frames_dropped,
            bytes_received = stats.bytes_received,
            reconnect_count = stats.reconnect_count,
            "RTSP final stats"
        );
    }

    if let Some(client) = &state.grpc_client {
        let stats = client.stats();
        info!(
            frames_sent = stats.frames_sent,
            frames_accepted = stats.frames_accepted,
            frames_rejected = stats.frames_rejected,
            batches_sent = stats.batches_sent,
            avg_latency_ms = format!("{:.2}", stats.avg_latency_ms),
            "gRPC final stats"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_state_creation() {
        let config = IngestConfig {
            rtsp: config::RtspConfig {
                url: "rtsp://test:554/stream".to_string(),
                device_id: "test-device".to_string(),
                worker_id: None,
                zone_id: None,
                connection_timeout_secs: 10,
                max_reconnect_attempts: 3,
                reconnect_base_delay_ms: 1000,
                reconnect_max_delay_ms: 30000,
                transport: "tcp".to_string(),
                buffer_ms: 200,
            },
            processing: config::ProcessingConfig {
                target_width: 640,
                target_height: 480,
                target_fps: 10.0,
                pixel_format: "RGB".to_string(),
                queue_size: 100,
                num_workers: 2,
                drop_on_backpressure: true,
            },
            grpc: config::GrpcConfig {
                inference_endpoint: "http://localhost:50051".to_string(),
                request_timeout_secs: 30,
                connection_timeout_secs: 10,
                max_concurrent_requests: 10,
                use_tls: false,
                ca_cert_path: None,
                enable_compression: false,
                batch_size: 1,
                batch_timeout_ms: 100,
            },
            logging: config::LoggingConfig::default(),
            health: config::HealthConfig::default(),
        };

        let state = AppState::new(config);
        assert!(!state.is_running());
    }

    #[test]
    fn test_app_state_shutdown() {
        let config = IngestConfig {
            rtsp: config::RtspConfig {
                url: "rtsp://test:554/stream".to_string(),
                device_id: "test-device".to_string(),
                worker_id: None,
                zone_id: None,
                connection_timeout_secs: 10,
                max_reconnect_attempts: 3,
                reconnect_base_delay_ms: 1000,
                reconnect_max_delay_ms: 30000,
                transport: "tcp".to_string(),
                buffer_ms: 200,
            },
            processing: config::ProcessingConfig {
                target_width: 640,
                target_height: 480,
                target_fps: 10.0,
                pixel_format: "RGB".to_string(),
                queue_size: 100,
                num_workers: 2,
                drop_on_backpressure: true,
            },
            grpc: config::GrpcConfig {
                inference_endpoint: "http://localhost:50051".to_string(),
                request_timeout_secs: 30,
                connection_timeout_secs: 10,
                max_concurrent_requests: 10,
                use_tls: false,
                ca_cert_path: None,
                enable_compression: false,
                batch_size: 1,
                batch_timeout_ms: 100,
            },
            logging: config::LoggingConfig::default(),
            health: config::HealthConfig::default(),
        };

        let state = AppState::new(config);
        state.running.store(true, Ordering::SeqCst);
        assert!(state.is_running());

        state.shutdown();
        assert!(!state.is_running());
    }
}
