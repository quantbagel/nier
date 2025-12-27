-- Create frames table for storing frame metadata
-- Partitioning: date/device/event-type is handled at S3 level
-- This table provides queryable metadata index

CREATE TABLE IF NOT EXISTS frames (
    -- Primary identifier
    id UUID PRIMARY KEY,

    -- Original event ID from Kafka
    event_id UUID NOT NULL,

    -- Device identification
    device_id VARCHAR(255) NOT NULL,

    -- Frame timing
    timestamp TIMESTAMPTZ NOT NULL,
    frame_number BIGINT NOT NULL,

    -- S3 storage location
    s3_key VARCHAR(1024) NOT NULL UNIQUE,

    -- Frame properties
    width INTEGER NOT NULL,
    height INTEGER NOT NULL,
    format VARCHAR(32) NOT NULL,

    -- Storage trigger info
    trigger_type VARCHAR(64) NOT NULL,
    storage_reason TEXT NOT NULL,

    -- Detection summary for quick filtering
    detection_count INTEGER NOT NULL DEFAULT 0,
    detection_types TEXT, -- Comma-separated list
    max_confidence REAL,

    -- Size tracking
    size_bytes BIGINT NOT NULL,

    -- Additional metadata as JSON
    metadata JSONB DEFAULT '{}',

    -- Audit
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes for common query patterns

-- Query frames by device and time range (primary use case)
CREATE INDEX idx_frames_device_timestamp ON frames (device_id, timestamp DESC);

-- Query frames by timestamp (for time-range queries across devices)
CREATE INDEX idx_frames_timestamp ON frames (timestamp DESC);

-- Query frames by trigger type (filter detection vs sample frames)
CREATE INDEX idx_frames_trigger_type ON frames (trigger_type);

-- Query frames with detections
CREATE INDEX idx_frames_detection_count ON frames (detection_count) WHERE detection_count > 0;

-- Query by confidence threshold
CREATE INDEX idx_frames_max_confidence ON frames (max_confidence) WHERE max_confidence IS NOT NULL;

-- Full-text search on detection types
CREATE INDEX idx_frames_detection_types ON frames USING gin (to_tsvector('english', detection_types));

-- Query by S3 key (for reverse lookups)
CREATE INDEX idx_frames_s3_key ON frames (s3_key);

-- Query by event ID (for correlation with other services)
CREATE INDEX idx_frames_event_id ON frames (event_id);

-- Comment on table
COMMENT ON TABLE frames IS 'Frame metadata index for stored frames in S3';
COMMENT ON COLUMN frames.id IS 'Unique identifier for the stored frame record';
COMMENT ON COLUMN frames.event_id IS 'Original event ID from the storage trigger Kafka message';
COMMENT ON COLUMN frames.device_id IS 'Identifier of the camera glasses device';
COMMENT ON COLUMN frames.timestamp IS 'Original capture timestamp of the frame';
COMMENT ON COLUMN frames.s3_key IS 'S3 object key where the frame is stored';
COMMENT ON COLUMN frames.trigger_type IS 'What triggered storage: detection, sample, debug, manual, alert';
COMMENT ON COLUMN frames.detection_types IS 'Comma-separated list of detection types present in frame';
COMMENT ON COLUMN frames.max_confidence IS 'Highest confidence score among all detections in frame';
