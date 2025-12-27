use crate::config::{ApiConfig, S3Config};
use crate::metadata_store::{FrameMetadata, FrameQuery, MetadataStore};
use crate::s3_uploader::S3Uploader;
use anyhow::{Context, Result};
use aws_sdk_s3::presigning::PresigningConfig;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing::{error, info, instrument};
use uuid::Uuid;

/// Application state shared across handlers
#[derive(Clone)]
pub struct AppState {
    pub s3_uploader: Arc<S3Uploader>,
    pub metadata_store: Arc<MetadataStore>,
    pub presigned_url_expiry: Duration,
}

/// Presigned URL response
#[derive(Debug, Serialize)]
pub struct PresignedUrlResponse {
    /// The presigned URL for frame access
    pub url: String,
    /// URL expiration time
    pub expires_at: DateTime<Utc>,
    /// Frame metadata
    pub frame: FrameMetadataResponse,
}

/// Frame metadata in API responses
#[derive(Debug, Serialize)]
pub struct FrameMetadataResponse {
    pub id: Uuid,
    pub device_id: String,
    pub timestamp: DateTime<Utc>,
    pub frame_number: i64,
    pub width: i32,
    pub height: i32,
    pub format: String,
    pub trigger_type: String,
    pub detection_count: i32,
    pub detection_types: Option<String>,
    pub max_confidence: Option<f32>,
}

impl From<FrameMetadata> for FrameMetadataResponse {
    fn from(f: FrameMetadata) -> Self {
        Self {
            id: f.id,
            device_id: f.device_id,
            timestamp: f.timestamp,
            frame_number: f.frame_number,
            width: f.width,
            height: f.height,
            format: f.format,
            trigger_type: f.trigger_type,
            detection_count: f.detection_count,
            detection_types: f.detection_types,
            max_confidence: f.max_confidence,
        }
    }
}

/// Query parameters for frame list
#[derive(Debug, Deserialize)]
pub struct FrameListQuery {
    /// Filter by device ID
    pub device_id: Option<String>,
    /// Start time (ISO 8601)
    pub start_time: Option<DateTime<Utc>>,
    /// End time (ISO 8601)
    pub end_time: Option<DateTime<Utc>>,
    /// Filter by trigger type
    pub trigger_type: Option<String>,
    /// Filter by detection type
    pub detection_type: Option<String>,
    /// Minimum confidence
    pub min_confidence: Option<f32>,
    /// Maximum results
    #[serde(default = "default_limit")]
    pub limit: i64,
    /// Offset for pagination
    #[serde(default)]
    pub offset: i64,
    /// Include presigned URLs
    #[serde(default)]
    pub include_urls: bool,
}

fn default_limit() -> i64 {
    50
}

/// Frame list response
#[derive(Debug, Serialize)]
pub struct FrameListResponse {
    pub frames: Vec<FrameWithUrl>,
    pub total_count: i64,
    pub has_more: bool,
}

/// Frame with optional presigned URL
#[derive(Debug, Serialize)]
pub struct FrameWithUrl {
    #[serde(flatten)]
    pub frame: FrameMetadataResponse,
    /// Presigned URL (if requested)
    pub url: Option<String>,
    /// URL expiration (if URL included)
    pub url_expires_at: Option<DateTime<Utc>>,
}

/// Batch presigned URL request
#[derive(Debug, Deserialize)]
pub struct BatchPresignedUrlRequest {
    /// Frame IDs to generate URLs for
    pub frame_ids: Vec<Uuid>,
}

/// Batch presigned URL response
#[derive(Debug, Serialize)]
pub struct BatchPresignedUrlResponse {
    pub urls: Vec<PresignedUrlResult>,
}

/// Individual URL result in batch
#[derive(Debug, Serialize)]
pub struct PresignedUrlResult {
    pub frame_id: Uuid,
    pub url: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub error: Option<String>,
}

/// Error response
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub code: String,
}

/// Create the API router
pub fn create_router(state: AppState, config: &ApiConfig) -> Router {
    let cors = if config.cors_enabled {
        if config.cors_origins.is_empty() {
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any)
        } else {
            let origins: Vec<_> = config
                .cors_origins
                .iter()
                .filter_map(|o| o.parse().ok())
                .collect();
            CorsLayer::new()
                .allow_origin(origins)
                .allow_methods(Any)
                .allow_headers(Any)
        }
    } else {
        CorsLayer::new()
    };

    Router::new()
        .route("/health", get(health_check))
        .route("/ready", get(readiness_check))
        .route("/api/v1/frames", get(list_frames))
        .route("/api/v1/frames/:frame_id", get(get_frame))
        .route("/api/v1/frames/:frame_id/url", get(get_presigned_url))
        .route("/api/v1/frames/batch-urls", post(batch_presigned_urls))
        .route("/api/v1/playback/:device_id", get(get_playback_urls))
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .with_state(state)
}

/// Health check endpoint
async fn health_check() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "healthy",
        "service": "storage-service"
    }))
}

/// Readiness check endpoint
async fn readiness_check(State(state): State<AppState>) -> impl IntoResponse {
    // Check database connectivity
    match sqlx::query("SELECT 1")
        .fetch_one(state.metadata_store.pool())
        .await
    {
        Ok(_) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "status": "ready",
                "database": "connected"
            })),
        ),
        Err(e) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({
                "status": "not_ready",
                "database": "disconnected",
                "error": e.to_string()
            })),
        ),
    }
}

/// List frames with filtering
#[instrument(skip(state))]
async fn list_frames(
    State(state): State<AppState>,
    Query(params): Query<FrameListQuery>,
) -> Result<Json<FrameListResponse>, (StatusCode, Json<ErrorResponse>)> {
    let query = FrameQuery {
        device_id: params.device_id,
        start_time: params.start_time,
        end_time: params.end_time,
        trigger_type: params.trigger_type,
        detection_type: params.detection_type,
        min_confidence: params.min_confidence,
        limit: Some(params.limit + 1), // Fetch one extra to check has_more
        offset: Some(params.offset),
        ascending: false,
    };

    let mut frames = state
        .metadata_store
        .query_frames(&query)
        .await
        .map_err(|e| {
            error!(error = %e, "Failed to query frames");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to query frames".to_string(),
                    code: "QUERY_ERROR".to_string(),
                }),
            )
        })?;

    let has_more = frames.len() > params.limit as usize;
    if has_more {
        frames.pop();
    }

    let mut frame_responses = Vec::with_capacity(frames.len());

    for frame in frames {
        let (url, url_expires_at) = if params.include_urls {
            match generate_presigned_url(&state, &frame.s3_key).await {
                Ok((url, expires)) => (Some(url), Some(expires)),
                Err(e) => {
                    error!(error = %e, s3_key = %frame.s3_key, "Failed to generate presigned URL");
                    (None, None)
                }
            }
        } else {
            (None, None)
        };

        frame_responses.push(FrameWithUrl {
            frame: frame.into(),
            url,
            url_expires_at,
        });
    }

    // Get total count
    let total_count = state
        .metadata_store
        .get_frame_count(
            query.device_id.as_deref(),
            query.start_time,
            query.end_time,
        )
        .await
        .unwrap_or(0);

    Ok(Json(FrameListResponse {
        frames: frame_responses,
        total_count,
        has_more,
    }))
}

/// Get single frame metadata
#[instrument(skip(state))]
async fn get_frame(
    State(state): State<AppState>,
    Path(frame_id): Path<Uuid>,
) -> Result<Json<FrameMetadataResponse>, (StatusCode, Json<ErrorResponse>)> {
    let frame = state
        .metadata_store
        .get_frame(frame_id)
        .await
        .map_err(|e| {
            error!(error = %e, "Failed to get frame");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to get frame".to_string(),
                    code: "QUERY_ERROR".to_string(),
                }),
            )
        })?;

    match frame {
        Some(f) => Ok(Json(f.into())),
        None => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "Frame not found".to_string(),
                code: "NOT_FOUND".to_string(),
            }),
        )),
    }
}

/// Get presigned URL for a frame
#[instrument(skip(state))]
async fn get_presigned_url(
    State(state): State<AppState>,
    Path(frame_id): Path<Uuid>,
) -> Result<Json<PresignedUrlResponse>, (StatusCode, Json<ErrorResponse>)> {
    let frame = state
        .metadata_store
        .get_frame(frame_id)
        .await
        .map_err(|e| {
            error!(error = %e, "Failed to get frame");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to get frame".to_string(),
                    code: "QUERY_ERROR".to_string(),
                }),
            )
        })?;

    let frame = frame.ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "Frame not found".to_string(),
                code: "NOT_FOUND".to_string(),
            }),
        )
    })?;

    let (url, expires_at) = generate_presigned_url(&state, &frame.s3_key)
        .await
        .map_err(|e| {
            error!(error = %e, "Failed to generate presigned URL");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to generate presigned URL".to_string(),
                    code: "PRESIGN_ERROR".to_string(),
                }),
            )
        })?;

    Ok(Json(PresignedUrlResponse {
        url,
        expires_at,
        frame: frame.into(),
    }))
}

/// Batch generate presigned URLs
#[instrument(skip(state))]
async fn batch_presigned_urls(
    State(state): State<AppState>,
    Json(request): Json<BatchPresignedUrlRequest>,
) -> Result<Json<BatchPresignedUrlResponse>, (StatusCode, Json<ErrorResponse>)> {
    if request.frame_ids.len() > 100 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Maximum 100 frames per batch".to_string(),
                code: "BATCH_TOO_LARGE".to_string(),
            }),
        ));
    }

    let mut results = Vec::with_capacity(request.frame_ids.len());

    for frame_id in request.frame_ids {
        let result = match state.metadata_store.get_frame(frame_id).await {
            Ok(Some(frame)) => match generate_presigned_url(&state, &frame.s3_key).await {
                Ok((url, expires_at)) => PresignedUrlResult {
                    frame_id,
                    url: Some(url),
                    expires_at: Some(expires_at),
                    error: None,
                },
                Err(e) => PresignedUrlResult {
                    frame_id,
                    url: None,
                    expires_at: None,
                    error: Some(e.to_string()),
                },
            },
            Ok(None) => PresignedUrlResult {
                frame_id,
                url: None,
                expires_at: None,
                error: Some("Frame not found".to_string()),
            },
            Err(e) => PresignedUrlResult {
                frame_id,
                url: None,
                expires_at: None,
                error: Some(e.to_string()),
            },
        };

        results.push(result);
    }

    Ok(Json(BatchPresignedUrlResponse { urls: results }))
}

/// Get playback URLs for a device within a time range
#[instrument(skip(state))]
async fn get_playback_urls(
    State(state): State<AppState>,
    Path(device_id): Path<String>,
    Query(params): Query<PlaybackQuery>,
) -> Result<Json<PlaybackResponse>, (StatusCode, Json<ErrorResponse>)> {
    let query = FrameQuery {
        device_id: Some(device_id.clone()),
        start_time: params.start_time,
        end_time: params.end_time,
        limit: Some(params.limit.unwrap_or(100).min(500)),
        ascending: true, // Chronological order for playback
        ..Default::default()
    };

    let frames = state
        .metadata_store
        .query_frames(&query)
        .await
        .map_err(|e| {
            error!(error = %e, "Failed to query frames for playback");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to query frames".to_string(),
                    code: "QUERY_ERROR".to_string(),
                }),
            )
        })?;

    let mut playback_frames = Vec::with_capacity(frames.len());

    for frame in frames {
        let (url, expires_at) = generate_presigned_url(&state, &frame.s3_key)
            .await
            .map_err(|e| {
                error!(error = %e, "Failed to generate presigned URL");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        error: "Failed to generate presigned URL".to_string(),
                        code: "PRESIGN_ERROR".to_string(),
                    }),
                )
            })?;

        playback_frames.push(PlaybackFrame {
            frame_id: frame.id,
            timestamp: frame.timestamp,
            frame_number: frame.frame_number,
            url,
            expires_at,
            detection_count: frame.detection_count,
        });
    }

    Ok(Json(PlaybackResponse {
        device_id,
        frames: playback_frames,
    }))
}

/// Query parameters for playback
#[derive(Debug, Deserialize)]
pub struct PlaybackQuery {
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub limit: Option<i64>,
}

/// Playback response
#[derive(Debug, Serialize)]
pub struct PlaybackResponse {
    pub device_id: String,
    pub frames: Vec<PlaybackFrame>,
}

/// Frame for playback
#[derive(Debug, Serialize)]
pub struct PlaybackFrame {
    pub frame_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub frame_number: i64,
    pub url: String,
    pub expires_at: DateTime<Utc>,
    pub detection_count: i32,
}

/// Generate a presigned URL for an S3 key
async fn generate_presigned_url(
    state: &AppState,
    s3_key: &str,
) -> Result<(String, DateTime<Utc>)> {
    let presigning_config = PresigningConfig::expires_in(state.presigned_url_expiry)
        .context("Failed to create presigning config")?;

    let presigned = state
        .s3_uploader
        .client()
        .get_object()
        .bucket(state.s3_uploader.bucket())
        .key(s3_key)
        .presigned(presigning_config)
        .await
        .context("Failed to generate presigned URL")?;

    let expires_at = Utc::now() + chrono::Duration::from_std(state.presigned_url_expiry).unwrap();

    Ok((presigned.uri().to_string(), expires_at))
}

/// Start the presigned URL API server
pub async fn start_api_server(
    state: AppState,
    config: &ApiConfig,
) -> Result<()> {
    let router = create_router(state, config);
    let addr = format!("{}:{}", config.host, config.port);

    info!(address = %addr, "Starting presigned URL API server");

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .context("Failed to bind to address")?;

    axum::serve(listener, router)
        .await
        .context("API server error")?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frame_metadata_response_from() {
        let frame = FrameMetadata {
            id: Uuid::new_v4(),
            event_id: Uuid::new_v4(),
            device_id: "test-device".to_string(),
            timestamp: Utc::now(),
            frame_number: 100,
            s3_key: "test/key.jpg".to_string(),
            width: 1920,
            height: 1080,
            format: "jpeg".to_string(),
            trigger_type: "detection".to_string(),
            storage_reason: "test".to_string(),
            detection_count: 2,
            detection_types: Some("safety_vest,hard_hat".to_string()),
            max_confidence: Some(0.95),
            size_bytes: 50000,
            metadata: serde_json::Value::Null,
            created_at: Utc::now(),
        };

        let response: FrameMetadataResponse = frame.into();
        assert_eq!(response.detection_count, 2);
        assert_eq!(response.format, "jpeg");
    }
}
