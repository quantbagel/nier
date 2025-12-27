use crate::config::DatabaseConfig;
use crate::kafka_consumer::{Detection, StorageTriggerEvent, TriggerType};
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::postgres::{PgPool, PgPoolOptions};
use sqlx::FromRow;
use std::time::Duration;
use tracing::{debug, info, instrument};
use uuid::Uuid;

/// Stored frame metadata
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct FrameMetadata {
    /// Unique frame ID
    pub id: Uuid,
    /// Original event ID
    pub event_id: Uuid,
    /// Device ID (camera glasses)
    pub device_id: String,
    /// Frame timestamp
    pub timestamp: DateTime<Utc>,
    /// Frame sequence number
    pub frame_number: i64,
    /// S3 object key
    pub s3_key: String,
    /// Frame width
    pub width: i32,
    /// Frame height
    pub height: i32,
    /// Frame format
    pub format: String,
    /// Trigger type that caused storage
    pub trigger_type: String,
    /// Reason for storage
    pub storage_reason: String,
    /// Number of detections in frame
    pub detection_count: i32,
    /// Detection types present (comma-separated)
    pub detection_types: Option<String>,
    /// Highest detection confidence
    pub max_confidence: Option<f32>,
    /// Frame size in bytes
    pub size_bytes: i64,
    /// Additional metadata as JSON
    pub metadata: serde_json::Value,
    /// When the record was created
    pub created_at: DateTime<Utc>,
}

/// Query parameters for frame search
#[derive(Debug, Clone, Default)]
pub struct FrameQuery {
    /// Filter by device ID
    pub device_id: Option<String>,
    /// Start time (inclusive)
    pub start_time: Option<DateTime<Utc>>,
    /// End time (exclusive)
    pub end_time: Option<DateTime<Utc>>,
    /// Filter by trigger type
    pub trigger_type: Option<String>,
    /// Filter by detection type
    pub detection_type: Option<String>,
    /// Minimum confidence threshold
    pub min_confidence: Option<f32>,
    /// Maximum number of results
    pub limit: Option<i64>,
    /// Offset for pagination
    pub offset: Option<i64>,
    /// Order by timestamp (true = ascending, false = descending)
    pub ascending: bool,
}

/// Detection metadata stored in database
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DetectionRecord {
    /// Unique detection ID
    pub id: Uuid,
    /// Associated frame ID
    pub frame_id: Uuid,
    /// Detection type/class
    pub detection_type: String,
    /// Confidence score
    pub confidence: f32,
    /// Bounding box as JSON [x, y, w, h]
    pub bbox: serde_json::Value,
    /// Additional attributes
    pub attributes: serde_json::Value,
    /// When the record was created
    pub created_at: DateTime<Utc>,
}

/// Metadata store for frame indexing in PostgreSQL
pub struct MetadataStore {
    pool: PgPool,
}

impl MetadataStore {
    /// Create a new metadata store with connection pool
    pub async fn new(config: &DatabaseConfig) -> Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(config.max_connections)
            .min_connections(config.min_connections)
            .acquire_timeout(Duration::from_secs(config.connect_timeout_secs))
            .idle_timeout(Some(Duration::from_secs(config.idle_timeout_secs)))
            .connect(&config.url)
            .await
            .context("Failed to connect to PostgreSQL")?;

        info!("Connected to PostgreSQL database");

        Ok(Self { pool })
    }

    /// Run database migrations
    pub async fn run_migrations(&self) -> Result<()> {
        info!("Running database migrations");

        sqlx::migrate!("./migrations")
            .run(&self.pool)
            .await
            .context("Failed to run migrations")?;

        info!("Database migrations completed");
        Ok(())
    }

    /// Index a frame in the metadata store
    #[instrument(skip(self, event), fields(event_id = %event.event_id, device_id = %event.device_id))]
    pub async fn index_frame(
        &self,
        event: &StorageTriggerEvent,
        s3_key: &str,
        storage_reason: &str,
    ) -> Result<Uuid> {
        let frame_id = Uuid::new_v4();
        let trigger_type = format!("{:?}", event.trigger_type).to_lowercase();

        // Extract detection summary
        let detection_count = event.detections.len() as i32;
        let detection_types: Option<String> = if event.detections.is_empty() {
            None
        } else {
            let types: Vec<String> = event
                .detections
                .iter()
                .map(|d| d.detection_type.clone())
                .collect();
            Some(types.join(","))
        };
        let max_confidence: Option<f32> = event
            .detections
            .iter()
            .map(|d| d.confidence)
            .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        // Start transaction
        let mut tx = self.pool.begin().await.context("Failed to begin transaction")?;

        // Insert frame metadata
        sqlx::query(
            r#"
            INSERT INTO frames (
                id, event_id, device_id, timestamp, frame_number,
                s3_key, width, height, format, trigger_type,
                storage_reason, detection_count, detection_types,
                max_confidence, size_bytes, metadata, created_at
            ) VALUES (
                $1, $2, $3, $4, $5,
                $6, $7, $8, $9, $10,
                $11, $12, $13, $14, $15,
                $16, NOW()
            )
            "#,
        )
        .bind(frame_id)
        .bind(event.event_id)
        .bind(&event.device_id)
        .bind(event.timestamp)
        .bind(event.frame_number as i64)
        .bind(s3_key)
        .bind(event.width as i32)
        .bind(event.height as i32)
        .bind(&event.format)
        .bind(&trigger_type)
        .bind(storage_reason)
        .bind(detection_count)
        .bind(&detection_types)
        .bind(max_confidence)
        .bind(event.frame_data.len() as i64)
        .bind(&event.metadata)
        .execute(&mut *tx)
        .await
        .context("Failed to insert frame metadata")?;

        // Insert detection records
        for detection in &event.detections {
            let detection_id = Uuid::new_v4();
            let bbox_json = serde_json::to_value(&detection.bbox)?;

            sqlx::query(
                r#"
                INSERT INTO detections (
                    id, frame_id, detection_type, confidence,
                    bbox, attributes, created_at
                ) VALUES (
                    $1, $2, $3, $4, $5, $6, NOW()
                )
                "#,
            )
            .bind(detection_id)
            .bind(frame_id)
            .bind(&detection.detection_type)
            .bind(detection.confidence)
            .bind(&bbox_json)
            .bind(&detection.attributes)
            .execute(&mut *tx)
            .await
            .context("Failed to insert detection record")?;
        }

        tx.commit().await.context("Failed to commit transaction")?;

        debug!(
            frame_id = %frame_id,
            s3_key = %s3_key,
            detection_count = detection_count,
            "Frame indexed successfully"
        );

        metrics::counter!("storage.frames.indexed").increment(1);

        Ok(frame_id)
    }

    /// Get frame metadata by ID
    pub async fn get_frame(&self, frame_id: Uuid) -> Result<Option<FrameMetadata>> {
        let frame = sqlx::query_as::<_, FrameMetadata>(
            r#"
            SELECT id, event_id, device_id, timestamp, frame_number,
                   s3_key, width, height, format, trigger_type,
                   storage_reason, detection_count, detection_types,
                   max_confidence, size_bytes, metadata, created_at
            FROM frames
            WHERE id = $1
            "#,
        )
        .bind(frame_id)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to query frame")?;

        Ok(frame)
    }

    /// Get frame metadata by S3 key
    pub async fn get_frame_by_s3_key(&self, s3_key: &str) -> Result<Option<FrameMetadata>> {
        let frame = sqlx::query_as::<_, FrameMetadata>(
            r#"
            SELECT id, event_id, device_id, timestamp, frame_number,
                   s3_key, width, height, format, trigger_type,
                   storage_reason, detection_count, detection_types,
                   max_confidence, size_bytes, metadata, created_at
            FROM frames
            WHERE s3_key = $1
            "#,
        )
        .bind(s3_key)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to query frame by S3 key")?;

        Ok(frame)
    }

    /// Query frames with filters
    #[instrument(skip(self))]
    pub async fn query_frames(&self, query: &FrameQuery) -> Result<Vec<FrameMetadata>> {
        let mut sql = String::from(
            r#"
            SELECT id, event_id, device_id, timestamp, frame_number,
                   s3_key, width, height, format, trigger_type,
                   storage_reason, detection_count, detection_types,
                   max_confidence, size_bytes, metadata, created_at
            FROM frames
            WHERE 1=1
            "#,
        );

        let mut bindings: Vec<String> = Vec::new();
        let mut param_count = 0;

        if query.device_id.is_some() {
            param_count += 1;
            sql.push_str(&format!(" AND device_id = ${}", param_count));
        }

        if query.start_time.is_some() {
            param_count += 1;
            sql.push_str(&format!(" AND timestamp >= ${}", param_count));
        }

        if query.end_time.is_some() {
            param_count += 1;
            sql.push_str(&format!(" AND timestamp < ${}", param_count));
        }

        if query.trigger_type.is_some() {
            param_count += 1;
            sql.push_str(&format!(" AND trigger_type = ${}", param_count));
        }

        if query.detection_type.is_some() {
            param_count += 1;
            sql.push_str(&format!(" AND detection_types LIKE ${}", param_count));
        }

        if query.min_confidence.is_some() {
            param_count += 1;
            sql.push_str(&format!(" AND max_confidence >= ${}", param_count));
        }

        // Order by timestamp
        if query.ascending {
            sql.push_str(" ORDER BY timestamp ASC");
        } else {
            sql.push_str(" ORDER BY timestamp DESC");
        }

        // Limit and offset
        if let Some(limit) = query.limit {
            param_count += 1;
            sql.push_str(&format!(" LIMIT ${}", param_count));
        }

        if let Some(offset) = query.offset {
            param_count += 1;
            sql.push_str(&format!(" OFFSET ${}", param_count));
        }

        // Build and execute query
        let mut query_builder = sqlx::query_as::<_, FrameMetadata>(&sql);

        if let Some(ref device_id) = query.device_id {
            query_builder = query_builder.bind(device_id);
        }
        if let Some(start_time) = query.start_time {
            query_builder = query_builder.bind(start_time);
        }
        if let Some(end_time) = query.end_time {
            query_builder = query_builder.bind(end_time);
        }
        if let Some(ref trigger_type) = query.trigger_type {
            query_builder = query_builder.bind(trigger_type);
        }
        if let Some(ref detection_type) = query.detection_type {
            query_builder = query_builder.bind(format!("%{}%", detection_type));
        }
        if let Some(min_confidence) = query.min_confidence {
            query_builder = query_builder.bind(min_confidence);
        }
        if let Some(limit) = query.limit {
            query_builder = query_builder.bind(limit);
        }
        if let Some(offset) = query.offset {
            query_builder = query_builder.bind(offset);
        }

        let frames = query_builder
            .fetch_all(&self.pool)
            .await
            .context("Failed to query frames")?;

        Ok(frames)
    }

    /// Get detections for a frame
    pub async fn get_frame_detections(&self, frame_id: Uuid) -> Result<Vec<DetectionRecord>> {
        let detections = sqlx::query_as::<_, DetectionRecord>(
            r#"
            SELECT id, frame_id, detection_type, confidence,
                   bbox, attributes, created_at
            FROM detections
            WHERE frame_id = $1
            ORDER BY confidence DESC
            "#,
        )
        .bind(frame_id)
        .fetch_all(&self.pool)
        .await
        .context("Failed to query detections")?;

        Ok(detections)
    }

    /// Get frame count by device and time range
    pub async fn get_frame_count(
        &self,
        device_id: Option<&str>,
        start_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
    ) -> Result<i64> {
        let count: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM frames
            WHERE ($1::text IS NULL OR device_id = $1)
              AND ($2::timestamptz IS NULL OR timestamp >= $2)
              AND ($3::timestamptz IS NULL OR timestamp < $3)
            "#,
        )
        .bind(device_id)
        .bind(start_time)
        .bind(end_time)
        .fetch_one(&self.pool)
        .await
        .context("Failed to count frames")?;

        Ok(count.0)
    }

    /// Get storage statistics
    pub async fn get_storage_stats(&self) -> Result<StorageStats> {
        let stats: StorageStats = sqlx::query_as(
            r#"
            SELECT
                COUNT(*) as total_frames,
                COALESCE(SUM(size_bytes), 0) as total_bytes,
                COALESCE(SUM(detection_count), 0) as total_detections,
                COUNT(DISTINCT device_id) as device_count
            FROM frames
            "#,
        )
        .fetch_one(&self.pool)
        .await
        .context("Failed to get storage stats")?;

        Ok(stats)
    }

    /// Delete old frames (for retention policy)
    #[instrument(skip(self))]
    pub async fn delete_frames_before(&self, before: DateTime<Utc>) -> Result<i64> {
        let result = sqlx::query(
            r#"
            WITH deleted_frames AS (
                DELETE FROM frames
                WHERE timestamp < $1
                RETURNING id
            )
            SELECT COUNT(*) FROM deleted_frames
            "#,
        )
        .bind(before)
        .fetch_one(&self.pool)
        .await
        .context("Failed to delete old frames")?;

        let count: i64 = result.get(0);

        info!(deleted_count = count, before = %before, "Deleted old frames");

        Ok(count)
    }

    /// Get the connection pool (for health checks)
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }
}

/// Storage statistics
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct StorageStats {
    pub total_frames: i64,
    pub total_bytes: i64,
    pub total_detections: i64,
    pub device_count: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frame_query_builder() {
        let query = FrameQuery {
            device_id: Some("glasses-001".to_string()),
            start_time: Some(Utc::now()),
            limit: Some(100),
            ..Default::default()
        };

        assert_eq!(query.device_id, Some("glasses-001".to_string()));
        assert_eq!(query.limit, Some(100));
    }
}
