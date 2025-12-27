mod config;
mod frame_selector;
mod kafka_consumer;
mod metadata_store;
mod presigned_urls;
mod s3_uploader;

use anyhow::{Context, Result};
use config::Config;
use frame_selector::FrameSelector;
use kafka_consumer::StorageKafkaConsumer;
use metadata_store::MetadataStore;
use presigned_urls::{start_api_server, AppState};
use s3_uploader::S3Uploader;
use std::sync::Arc;
use tokio::signal;
use tracing::{error, info, warn};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[tokio::main]
async fn main() -> Result<()> {
    // Load configuration
    let config = Config::load().context("Failed to load configuration")?;

    // Initialize logging
    init_tracing(&config.service.log_level);

    info!(
        service = %config.service.name,
        "Starting Nier Storage Service"
    );

    // Initialize metrics
    init_metrics(config.service.metrics_port)?;

    // Initialize components
    let metadata_store = Arc::new(
        MetadataStore::new(&config.database)
            .await
            .context("Failed to initialize metadata store")?,
    );

    // Run migrations if enabled
    if config.database.run_migrations {
        metadata_store
            .run_migrations()
            .await
            .context("Failed to run database migrations")?;
    }

    let s3_uploader = Arc::new(
        S3Uploader::new(&config.s3)
            .await
            .context("Failed to initialize S3 uploader")?,
    );

    let frame_selector = Arc::new(FrameSelector::new(config.frame_selection.clone()));

    // Create Kafka consumer
    let kafka_consumer = StorageKafkaConsumer::new(
        &config.kafka,
        frame_selector.clone(),
        s3_uploader.clone(),
        metadata_store.clone(),
        config.s3.upload_concurrency,
    )
    .await
    .context("Failed to initialize Kafka consumer")?;

    // Create API state
    let api_state = AppState {
        s3_uploader: s3_uploader.clone(),
        metadata_store: metadata_store.clone(),
        presigned_url_expiry: config.presigned_url_expiry(),
    };

    // Spawn Kafka consumer task
    let consumer_handle = tokio::spawn(async move {
        if let Err(e) = kafka_consumer.run().await {
            error!(error = %e, "Kafka consumer error");
        }
    });

    // Spawn API server task
    let api_config = config.api.clone();
    let api_handle = tokio::spawn(async move {
        if let Err(e) = start_api_server(api_state, &api_config).await {
            error!(error = %e, "API server error");
        }
    });

    info!("Storage service started successfully");

    // Wait for shutdown signal
    shutdown_signal().await;

    info!("Shutting down storage service");

    // Abort tasks
    consumer_handle.abort();
    api_handle.abort();

    info!("Storage service stopped");

    Ok(())
}

/// Initialize tracing/logging
fn init_tracing(log_level: &str) {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(log_level));

    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt::layer().json())
        .init();
}

/// Initialize Prometheus metrics exporter
fn init_metrics(port: u16) -> Result<()> {
    let builder = metrics_exporter_prometheus::PrometheusBuilder::new();

    builder
        .with_http_listener(([0, 0, 0, 0], port))
        .install()
        .context("Failed to install Prometheus metrics exporter")?;

    info!(port = port, "Prometheus metrics exporter started");

    Ok(())
}

/// Wait for shutdown signal (SIGINT or SIGTERM)
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            info!("Received Ctrl+C signal");
        }
        _ = terminate => {
            info!("Received SIGTERM signal");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_compiles() {
        // Basic compilation test
        assert!(true);
    }
}
