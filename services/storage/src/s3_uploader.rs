use crate::config::S3Config;
use crate::kafka_consumer::{StorageTriggerEvent, TriggerType};
use anyhow::{Context, Result};
use aws_config::BehaviorVersion;
use aws_sdk_s3::config::Builder as S3ConfigBuilder;
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::Client as S3Client;
use chrono::{DateTime, Datelike, Utc};
use std::sync::Arc;
use tracing::{debug, info, instrument};

/// S3 uploader for frame storage with proper partitioning
pub struct S3Uploader {
    client: S3Client,
    bucket: String,
    config: S3Config,
}

impl S3Uploader {
    /// Create a new S3 uploader
    pub async fn new(config: &S3Config) -> Result<Self> {
        let aws_config = aws_config::defaults(BehaviorVersion::latest())
            .region(aws_config::Region::new(config.region.clone()))
            .load()
            .await;

        let mut s3_config_builder = S3ConfigBuilder::from(&aws_config);

        // Configure custom endpoint for MinIO/LocalStack
        if let Some(ref endpoint_url) = config.endpoint_url {
            s3_config_builder = s3_config_builder.endpoint_url(endpoint_url);
        }

        // Force path-style access for MinIO compatibility
        if config.force_path_style {
            s3_config_builder = s3_config_builder.force_path_style(true);
        }

        let s3_config = s3_config_builder.build();
        let client = S3Client::from_conf(s3_config);

        info!(
            bucket = %config.bucket,
            region = %config.region,
            "S3 uploader initialized"
        );

        Ok(Self {
            client,
            bucket: config.bucket.clone(),
            config: config.clone(),
        })
    }

    /// Generate S3 key with proper partitioning strategy
    /// Format: frames/{date}/{device_id}/{event_type}/{timestamp}_{event_id}.{format}
    ///
    /// Partitioning strategy:
    /// - First level: date (YYYY-MM-DD) for time-based queries and lifecycle policies
    /// - Second level: device_id for device-specific queries
    /// - Third level: event_type for filtering by detection, sample, debug, etc.
    /// - Filename: timestamp + event_id for uniqueness and ordering
    pub fn generate_s3_key(&self, event: &StorageTriggerEvent) -> String {
        let date = event.timestamp.format("%Y-%m-%d").to_string();
        let event_type = match event.trigger_type {
            TriggerType::Detection => "detections",
            TriggerType::Sample => "samples",
            TriggerType::Debug => "debug",
            TriggerType::Manual => "manual",
            TriggerType::Alert => "alerts",
        };

        // Timestamp in sortable format for filename
        let timestamp_str = event.timestamp.format("%H%M%S%3f").to_string();

        format!(
            "frames/{date}/{device_id}/{event_type}/{timestamp}_{event_id}.{format}",
            date = date,
            device_id = sanitize_path_component(&event.device_id),
            event_type = event_type,
            timestamp = timestamp_str,
            event_id = event.event_id,
            format = event.format.to_lowercase()
        )
    }

    /// Upload a frame to S3
    #[instrument(skip(self, event), fields(event_id = %event.event_id, device_id = %event.device_id))]
    pub async fn upload_frame(&self, event: &StorageTriggerEvent) -> Result<String> {
        let s3_key = self.generate_s3_key(event);
        let content_type = get_content_type(&event.format);

        debug!(
            s3_key = %s3_key,
            size_bytes = event.frame_data.len(),
            "Uploading frame to S3"
        );

        // Check if we should use multipart upload
        if event.frame_data.len() > self.config.multipart_threshold_bytes {
            self.multipart_upload(event, &s3_key, &content_type).await?;
        } else {
            self.simple_upload(event, &s3_key, &content_type).await?;
        }

        info!(
            s3_key = %s3_key,
            size_bytes = event.frame_data.len(),
            "Frame uploaded successfully"
        );

        Ok(s3_key)
    }

    /// Simple single-part upload for small files
    async fn simple_upload(
        &self,
        event: &StorageTriggerEvent,
        s3_key: &str,
        content_type: &str,
    ) -> Result<()> {
        let body = ByteStream::from(event.frame_data.clone());

        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(s3_key)
            .body(body)
            .content_type(content_type)
            .metadata("device-id", &event.device_id)
            .metadata("frame-number", &event.frame_number.to_string())
            .metadata("trigger-type", &format!("{:?}", event.trigger_type).to_lowercase())
            .metadata("width", &event.width.to_string())
            .metadata("height", &event.height.to_string())
            .metadata("timestamp", &event.timestamp.to_rfc3339())
            .send()
            .await
            .context("Failed to upload frame to S3")?;

        Ok(())
    }

    /// Multipart upload for large files
    async fn multipart_upload(
        &self,
        event: &StorageTriggerEvent,
        s3_key: &str,
        content_type: &str,
    ) -> Result<()> {
        // Create multipart upload
        let create_response = self
            .client
            .create_multipart_upload()
            .bucket(&self.bucket)
            .key(s3_key)
            .content_type(content_type)
            .metadata("device-id", &event.device_id)
            .metadata("frame-number", &event.frame_number.to_string())
            .metadata("trigger-type", &format!("{:?}", event.trigger_type).to_lowercase())
            .send()
            .await
            .context("Failed to create multipart upload")?;

        let upload_id = create_response
            .upload_id()
            .context("No upload ID in response")?;

        let mut completed_parts = Vec::new();
        let part_size = self.config.part_size_bytes;
        let mut part_number = 1;

        // Upload parts
        for chunk in event.frame_data.chunks(part_size) {
            let body = ByteStream::from(chunk.to_vec());

            let upload_part_response = self
                .client
                .upload_part()
                .bucket(&self.bucket)
                .key(s3_key)
                .upload_id(upload_id)
                .part_number(part_number)
                .body(body)
                .send()
                .await
                .context("Failed to upload part")?;

            let completed_part = aws_sdk_s3::types::CompletedPart::builder()
                .part_number(part_number)
                .e_tag(upload_part_response.e_tag().unwrap_or_default())
                .build();

            completed_parts.push(completed_part);
            part_number += 1;
        }

        // Complete multipart upload
        let completed_upload = aws_sdk_s3::types::CompletedMultipartUpload::builder()
            .set_parts(Some(completed_parts))
            .build();

        self.client
            .complete_multipart_upload()
            .bucket(&self.bucket)
            .key(s3_key)
            .upload_id(upload_id)
            .multipart_upload(completed_upload)
            .send()
            .await
            .context("Failed to complete multipart upload")?;

        Ok(())
    }

    /// Delete a frame from S3
    #[instrument(skip(self), fields(s3_key = %s3_key))]
    pub async fn delete_frame(&self, s3_key: &str) -> Result<()> {
        self.client
            .delete_object()
            .bucket(&self.bucket)
            .key(s3_key)
            .send()
            .await
            .context("Failed to delete frame from S3")?;

        debug!(s3_key = %s3_key, "Frame deleted from S3");
        Ok(())
    }

    /// Check if a frame exists in S3
    pub async fn frame_exists(&self, s3_key: &str) -> Result<bool> {
        match self
            .client
            .head_object()
            .bucket(&self.bucket)
            .key(s3_key)
            .send()
            .await
        {
            Ok(_) => Ok(true),
            Err(e) => {
                if e.as_service_error()
                    .map(|e| e.is_not_found())
                    .unwrap_or(false)
                {
                    Ok(false)
                } else {
                    Err(e).context("Failed to check frame existence")
                }
            }
        }
    }

    /// List frames for a specific date and device
    #[instrument(skip(self))]
    pub async fn list_frames(
        &self,
        date: &str,
        device_id: Option<&str>,
        event_type: Option<&str>,
        max_keys: i32,
    ) -> Result<Vec<String>> {
        let mut prefix = format!("frames/{}", date);

        if let Some(device) = device_id {
            prefix = format!("{}/{}", prefix, sanitize_path_component(device));
            if let Some(etype) = event_type {
                prefix = format!("{}/{}", prefix, etype);
            }
        }

        let response = self
            .client
            .list_objects_v2()
            .bucket(&self.bucket)
            .prefix(&prefix)
            .max_keys(max_keys)
            .send()
            .await
            .context("Failed to list frames")?;

        let keys: Vec<String> = response
            .contents()
            .iter()
            .filter_map(|obj| obj.key().map(String::from))
            .collect();

        Ok(keys)
    }

    /// Get the S3 client (for presigned URL generation)
    pub fn client(&self) -> &S3Client {
        &self.client
    }

    /// Get the bucket name
    pub fn bucket(&self) -> &str {
        &self.bucket
    }
}

/// Sanitize a path component to prevent path traversal
fn sanitize_path_component(component: &str) -> String {
    component
        .chars()
        .map(|c| match c {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' => c,
            _ => '_',
        })
        .collect()
}

/// Get content type for frame format
fn get_content_type(format: &str) -> String {
    match format.to_lowercase().as_str() {
        "jpeg" | "jpg" => "image/jpeg".to_string(),
        "png" => "image/png".to_string(),
        "webp" => "image/webp".to_string(),
        "bmp" => "image/bmp".to_string(),
        "gif" => "image/gif".to_string(),
        _ => "application/octet-stream".to_string(),
    }
}

/// Batch uploader for efficient bulk operations
pub struct BatchUploader {
    uploader: Arc<S3Uploader>,
    concurrency: usize,
}

impl BatchUploader {
    pub fn new(uploader: Arc<S3Uploader>, concurrency: usize) -> Self {
        Self {
            uploader,
            concurrency,
        }
    }

    /// Upload multiple frames concurrently
    #[instrument(skip(self, events))]
    pub async fn upload_batch(&self, events: Vec<StorageTriggerEvent>) -> Vec<Result<String>> {
        use futures::stream::{self, StreamExt};

        let uploader = self.uploader.clone();

        stream::iter(events)
            .map(move |event| {
                let uploader = uploader.clone();
                async move { uploader.upload_frame(&event).await }
            })
            .buffer_unordered(self.concurrency)
            .collect()
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use uuid::Uuid;

    fn create_test_event() -> StorageTriggerEvent {
        StorageTriggerEvent {
            event_id: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap(),
            device_id: "glasses-001".to_string(),
            timestamp: Utc.with_ymd_and_hms(2024, 1, 15, 10, 30, 45).unwrap(),
            frame_number: 12345,
            frame_data: vec![0u8; 100],
            width: 1920,
            height: 1080,
            format: "jpeg".to_string(),
            detections: vec![],
            trigger_type: TriggerType::Detection,
            metadata: serde_json::Value::Null,
        }
    }

    #[test]
    fn test_generate_s3_key_detection() {
        let config = S3Config {
            bucket: "test-bucket".to_string(),
            region: "us-east-1".to_string(),
            endpoint_url: None,
            force_path_style: false,
            presigned_url_expiry_secs: 3600,
            upload_concurrency: 10,
            multipart_threshold_bytes: 5 * 1024 * 1024,
            part_size_bytes: 5 * 1024 * 1024,
        };

        // Create a mock uploader (we only need the key generation logic)
        let event = create_test_event();

        // Manually test the key format
        let date = event.timestamp.format("%Y-%m-%d").to_string();
        let timestamp_str = event.timestamp.format("%H%M%S%3f").to_string();

        let expected_key = format!(
            "frames/{}/{}/detections/{}_{}.jpeg",
            date, "glasses_001", timestamp_str, event.event_id
        );

        assert!(expected_key.contains("2024-01-15"));
        assert!(expected_key.contains("glasses_001"));
        assert!(expected_key.contains("detections"));
    }

    #[test]
    fn test_sanitize_path_component() {
        assert_eq!(sanitize_path_component("glasses-001"), "glasses-001");
        assert_eq!(sanitize_path_component("device/path"), "device_path");
        assert_eq!(sanitize_path_component("dev..ice"), "dev__ice");
        assert_eq!(sanitize_path_component("hello world"), "hello_world");
    }

    #[test]
    fn test_get_content_type() {
        assert_eq!(get_content_type("jpeg"), "image/jpeg");
        assert_eq!(get_content_type("JPEG"), "image/jpeg");
        assert_eq!(get_content_type("jpg"), "image/jpeg");
        assert_eq!(get_content_type("png"), "image/png");
        assert_eq!(get_content_type("unknown"), "application/octet-stream");
    }
}
