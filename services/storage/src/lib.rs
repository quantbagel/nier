//! Nier Storage Service
//!
//! Selective frame storage service for the Nier factory floor analytics platform.
//! This service consumes storage trigger events from Kafka, intelligently selects
//! which frames to store based on configurable criteria, uploads them to S3 with
//! proper partitioning, and indexes metadata in PostgreSQL.
//!
//! ## Features
//!
//! - **Intelligent Frame Selection**: Store frames based on detections, periodic
//!   sampling, debug flags, or manual triggers
//! - **Efficient S3 Storage**: Proper partitioning by date/device/event-type,
//!   multipart uploads for large frames
//! - **Reliable Metadata Indexing**: PostgreSQL-backed metadata store with full
//!   query capabilities
//! - **Presigned URL Generation**: API for generating time-limited access URLs
//!   for dashboard playback
//!
//! ## Architecture
//!
//! ```text
//! Kafka Topics                S3 Bucket                 PostgreSQL
//! ┌──────────────┐           ┌──────────────┐          ┌──────────────┐
//! │ Storage      │           │ frames/      │          │ frames       │
//! │ Triggers     │──────────▶│   {date}/    │          │ detections   │
//! └──────────────┘           │   {device}/  │          └──────────────┘
//!        │                   │   {type}/    │                 ▲
//!        │                   └──────────────┘                 │
//!        ▼                          │                         │
//! ┌──────────────┐                  │                         │
//! │ Frame        │                  │                         │
//! │ Selector     │                  │                         │
//! └──────────────┘                  │                         │
//!        │                          │                         │
//!        ▼                          ▼                         │
//! ┌──────────────┐           ┌──────────────┐                │
//! │ S3           │           │ Metadata     │────────────────┘
//! │ Uploader     │           │ Store        │
//! └──────────────┘           └──────────────┘
//!                                   │
//!                                   ▼
//!                            ┌──────────────┐
//!                            │ Presigned    │
//!                            │ URL API      │
//!                            └──────────────┘
//! ```

pub mod config;
pub mod frame_selector;
pub mod kafka_consumer;
pub mod metadata_store;
pub mod presigned_urls;
pub mod s3_uploader;

pub use config::Config;
pub use frame_selector::{FrameSelector, FrameSelectorBuilder, StorageDecision};
pub use kafka_consumer::{Detection, StorageKafkaConsumer, StorageTriggerEvent, TriggerType};
pub use metadata_store::{FrameMetadata, FrameQuery, MetadataStore, StorageStats};
pub use presigned_urls::{AppState, PresignedUrlResponse};
pub use s3_uploader::{BatchUploader, S3Uploader};
