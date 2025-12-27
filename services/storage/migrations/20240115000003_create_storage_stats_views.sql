-- Create views for storage statistics and reporting

-- Daily storage statistics per device
CREATE OR REPLACE VIEW daily_storage_stats AS
SELECT
    date_trunc('day', timestamp) AS date,
    device_id,
    COUNT(*) AS frame_count,
    SUM(size_bytes) AS total_bytes,
    SUM(detection_count) AS total_detections,
    AVG(detection_count)::REAL AS avg_detections_per_frame,
    COUNT(*) FILTER (WHERE trigger_type = 'detection') AS detection_frames,
    COUNT(*) FILTER (WHERE trigger_type = 'sample') AS sample_frames,
    COUNT(*) FILTER (WHERE trigger_type = 'debug') AS debug_frames,
    COUNT(*) FILTER (WHERE trigger_type = 'manual') AS manual_frames,
    COUNT(*) FILTER (WHERE trigger_type = 'alert') AS alert_frames,
    MAX(max_confidence) AS peak_confidence
FROM frames
GROUP BY date_trunc('day', timestamp), device_id
ORDER BY date DESC, device_id;

COMMENT ON VIEW daily_storage_stats IS 'Daily aggregated storage statistics per device';

-- Hourly storage statistics (for recent monitoring)
CREATE OR REPLACE VIEW hourly_storage_stats AS
SELECT
    date_trunc('hour', timestamp) AS hour,
    device_id,
    COUNT(*) AS frame_count,
    SUM(size_bytes) AS total_bytes,
    SUM(detection_count) AS total_detections,
    trigger_type,
    COUNT(*) AS trigger_count
FROM frames
WHERE timestamp > NOW() - INTERVAL '24 hours'
GROUP BY date_trunc('hour', timestamp), device_id, trigger_type
ORDER BY hour DESC, device_id;

COMMENT ON VIEW hourly_storage_stats IS 'Hourly storage statistics for the last 24 hours';

-- Detection type distribution
CREATE OR REPLACE VIEW detection_type_stats AS
SELECT
    detection_type,
    COUNT(*) AS detection_count,
    AVG(confidence)::REAL AS avg_confidence,
    MIN(confidence) AS min_confidence,
    MAX(confidence) AS max_confidence,
    COUNT(DISTINCT frame_id) AS unique_frames
FROM detections
GROUP BY detection_type
ORDER BY detection_count DESC;

COMMENT ON VIEW detection_type_stats IS 'Statistics on detection types across all frames';

-- Device summary
CREATE OR REPLACE VIEW device_summary AS
SELECT
    device_id,
    COUNT(*) AS total_frames,
    SUM(size_bytes) AS total_bytes,
    MIN(timestamp) AS first_frame_at,
    MAX(timestamp) AS last_frame_at,
    SUM(detection_count) AS total_detections,
    COUNT(DISTINCT date_trunc('day', timestamp)) AS active_days
FROM frames
GROUP BY device_id
ORDER BY last_frame_at DESC;

COMMENT ON VIEW device_summary IS 'Summary statistics per device';

-- Recent high-confidence detections
CREATE OR REPLACE VIEW recent_high_confidence_detections AS
SELECT
    d.id AS detection_id,
    d.detection_type,
    d.confidence,
    d.bbox,
    f.id AS frame_id,
    f.device_id,
    f.timestamp,
    f.s3_key
FROM detections d
JOIN frames f ON d.frame_id = f.id
WHERE d.confidence >= 0.9
  AND f.timestamp > NOW() - INTERVAL '1 hour'
ORDER BY d.confidence DESC, f.timestamp DESC
LIMIT 100;

COMMENT ON VIEW recent_high_confidence_detections IS 'Recent high-confidence detections for alerting';

-- Storage retention candidates (frames older than 30 days)
CREATE OR REPLACE VIEW retention_candidates AS
SELECT
    id,
    device_id,
    timestamp,
    s3_key,
    size_bytes,
    trigger_type,
    detection_count
FROM frames
WHERE timestamp < NOW() - INTERVAL '30 days'
ORDER BY timestamp ASC;

COMMENT ON VIEW retention_candidates IS 'Frames eligible for retention policy cleanup';
