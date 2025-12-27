-- Create detections table for storing individual detection records
-- Each detection is linked to a frame and contains bounding box and attributes

CREATE TABLE IF NOT EXISTS detections (
    -- Primary identifier
    id UUID PRIMARY KEY,

    -- Link to parent frame
    frame_id UUID NOT NULL REFERENCES frames(id) ON DELETE CASCADE,

    -- Detection classification
    detection_type VARCHAR(255) NOT NULL,
    confidence REAL NOT NULL,

    -- Bounding box as JSON array [x, y, width, height] normalized 0-1
    bbox JSONB NOT NULL,

    -- Additional detection attributes (pose, tracking ID, etc.)
    attributes JSONB DEFAULT '{}',

    -- Audit
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes for detection queries

-- Query detections by frame
CREATE INDEX idx_detections_frame_id ON detections (frame_id);

-- Query detections by type
CREATE INDEX idx_detections_type ON detections (detection_type);

-- Query high-confidence detections
CREATE INDEX idx_detections_confidence ON detections (confidence DESC);

-- Query detections by type and confidence
CREATE INDEX idx_detections_type_confidence ON detections (detection_type, confidence DESC);

-- GIN index for attribute queries
CREATE INDEX idx_detections_attributes ON detections USING gin (attributes);

-- Comment on table
COMMENT ON TABLE detections IS 'Individual detection records associated with stored frames';
COMMENT ON COLUMN detections.id IS 'Unique identifier for the detection record';
COMMENT ON COLUMN detections.frame_id IS 'Reference to the parent frame';
COMMENT ON COLUMN detections.detection_type IS 'Classification type (e.g., safety_vest, hard_hat, person)';
COMMENT ON COLUMN detections.confidence IS 'Detection confidence score between 0 and 1';
COMMENT ON COLUMN detections.bbox IS 'Bounding box as [x, y, width, height] normalized to 0-1 range';
COMMENT ON COLUMN detections.attributes IS 'Additional attributes like pose keypoints, tracking ID, etc.';
