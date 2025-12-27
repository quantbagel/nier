use crate::config::FrameSelectionConfig;
use crate::kafka_consumer::{StorageTriggerEvent, TriggerType};
use chrono::Utc;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::RwLock;
use std::time::Duration;
use tracing::{debug, trace};

/// Decision on whether to store a frame
#[derive(Debug, Clone)]
pub enum StorageDecision {
    /// Store the frame with given reason
    Store { reason: String },
    /// Skip storing the frame with given reason
    Skip { reason: String },
}

/// Frame selector that decides which frames to store
///
/// Implements intelligent frame selection based on:
/// - Detection presence and confidence
/// - Periodic sampling for non-detection frames
/// - Debug/manual triggers
/// - Frame age limits
pub struct FrameSelector {
    config: FrameSelectionConfig,
    /// Frame counters per device for sampling
    device_counters: RwLock<HashMap<String, AtomicU64>>,
    /// Maximum age for frames
    max_frame_age: Duration,
}

impl FrameSelector {
    /// Create a new frame selector with the given configuration
    pub fn new(config: FrameSelectionConfig) -> Self {
        let max_frame_age = Duration::from_secs(config.max_frame_age_secs);

        Self {
            config,
            device_counters: RwLock::new(HashMap::new()),
            max_frame_age,
        }
    }

    /// Determine if a frame should be stored
    pub fn should_store(&self, event: &StorageTriggerEvent) -> StorageDecision {
        // Check frame age first
        if let Some(decision) = self.check_frame_age(event) {
            return decision;
        }

        // Decision based on trigger type
        match event.trigger_type {
            TriggerType::Detection => self.evaluate_detection_frame(event),
            TriggerType::Sample => self.evaluate_sample_frame(event),
            TriggerType::Debug => self.evaluate_debug_frame(event),
            TriggerType::Manual => self.evaluate_manual_frame(event),
            TriggerType::Alert => self.evaluate_alert_frame(event),
        }
    }

    /// Check if frame is too old
    fn check_frame_age(&self, event: &StorageTriggerEvent) -> Option<StorageDecision> {
        let now = Utc::now();
        let frame_age = now.signed_duration_since(event.timestamp);

        if frame_age.num_seconds() > self.max_frame_age.as_secs() as i64 {
            return Some(StorageDecision::Skip {
                reason: format!(
                    "Frame too old: {}s > max {}s",
                    frame_age.num_seconds(),
                    self.max_frame_age.as_secs()
                ),
            });
        }

        None
    }

    /// Evaluate whether to store a detection frame
    fn evaluate_detection_frame(&self, event: &StorageTriggerEvent) -> StorageDecision {
        if !self.config.store_detections {
            return StorageDecision::Skip {
                reason: "Detection frame storage disabled".to_string(),
            };
        }

        // Check if there are any detections
        if event.detections.is_empty() {
            return StorageDecision::Skip {
                reason: "No detections in detection-triggered frame".to_string(),
            };
        }

        // Filter detections by confidence
        let high_confidence_detections: Vec<_> = event
            .detections
            .iter()
            .filter(|d| d.confidence >= self.config.min_confidence)
            .collect();

        if high_confidence_detections.is_empty() {
            return StorageDecision::Skip {
                reason: format!(
                    "No detections above confidence threshold {}",
                    self.config.min_confidence
                ),
            };
        }

        // Filter by detection types if configured
        if !self.config.detection_types.is_empty() {
            let matching_detections: Vec<_> = high_confidence_detections
                .iter()
                .filter(|d| {
                    self.config
                        .detection_types
                        .iter()
                        .any(|t| t.eq_ignore_ascii_case(&d.detection_type))
                })
                .collect();

            if matching_detections.is_empty() {
                return StorageDecision::Skip {
                    reason: format!(
                        "No detections matching configured types: {:?}",
                        self.config.detection_types
                    ),
                };
            }

            let detection_summary: Vec<String> = matching_detections
                .iter()
                .map(|d| format!("{}({:.2})", d.detection_type, d.confidence))
                .collect();

            return StorageDecision::Store {
                reason: format!("Matching detections: {}", detection_summary.join(", ")),
            };
        }

        // Store all high-confidence detections
        let detection_summary: Vec<String> = high_confidence_detections
            .iter()
            .map(|d| format!("{}({:.2})", d.detection_type, d.confidence))
            .collect();

        StorageDecision::Store {
            reason: format!("Detections: {}", detection_summary.join(", ")),
        }
    }

    /// Evaluate whether to store a sample frame
    fn evaluate_sample_frame(&self, event: &StorageTriggerEvent) -> StorageDecision {
        if !self.config.store_samples {
            return StorageDecision::Skip {
                reason: "Sample frame storage disabled".to_string(),
            };
        }

        // Increment counter for this device and check if we should sample
        let should_store = self.check_sample_rate(&event.device_id);

        if should_store {
            StorageDecision::Store {
                reason: format!(
                    "Periodic sample (1 per {} frames)",
                    self.config.sample_rate
                ),
            }
        } else {
            StorageDecision::Skip {
                reason: format!(
                    "Not sampled (rate: 1 per {} frames)",
                    self.config.sample_rate
                ),
            }
        }
    }

    /// Check sample rate for a device
    fn check_sample_rate(&self, device_id: &str) -> bool {
        // Get or create counter for device
        {
            let counters = self.device_counters.read().unwrap();
            if let Some(counter) = counters.get(device_id) {
                let count = counter.fetch_add(1, Ordering::Relaxed);
                return count % self.config.sample_rate as u64 == 0;
            }
        }

        // Counter doesn't exist, create it
        {
            let mut counters = self.device_counters.write().unwrap();
            counters
                .entry(device_id.to_string())
                .or_insert_with(|| AtomicU64::new(1));
        }

        // First frame for this device - always store
        true
    }

    /// Evaluate whether to store a debug frame
    fn evaluate_debug_frame(&self, event: &StorageTriggerEvent) -> StorageDecision {
        if !self.config.store_debug {
            return StorageDecision::Skip {
                reason: "Debug frame storage disabled".to_string(),
            };
        }

        StorageDecision::Store {
            reason: "Debug frame".to_string(),
        }
    }

    /// Evaluate whether to store a manual trigger frame
    fn evaluate_manual_frame(&self, event: &StorageTriggerEvent) -> StorageDecision {
        // Manual triggers are always stored
        StorageDecision::Store {
            reason: "Manual trigger".to_string(),
        }
    }

    /// Evaluate whether to store an alert frame
    fn evaluate_alert_frame(&self, event: &StorageTriggerEvent) -> StorageDecision {
        // Alert frames are always stored - they're important
        let alert_info = event
            .metadata
            .get("alert_type")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        StorageDecision::Store {
            reason: format!("Alert: {}", alert_info),
        }
    }

    /// Reset counters for a specific device (useful for testing)
    pub fn reset_device_counter(&self, device_id: &str) {
        let mut counters = self.device_counters.write().unwrap();
        counters.remove(device_id);
    }

    /// Get current counter value for a device (useful for testing)
    pub fn get_device_counter(&self, device_id: &str) -> Option<u64> {
        let counters = self.device_counters.read().unwrap();
        counters.get(device_id).map(|c| c.load(Ordering::Relaxed))
    }
}

/// Builder for creating FrameSelector with custom settings
pub struct FrameSelectorBuilder {
    config: FrameSelectionConfig,
}

impl FrameSelectorBuilder {
    pub fn new() -> Self {
        Self {
            config: FrameSelectionConfig {
                store_detections: true,
                store_samples: true,
                sample_rate: 30,
                store_debug: true,
                min_confidence: 0.5,
                detection_types: vec![],
                max_frame_age_secs: 300,
            },
        }
    }

    pub fn store_detections(mut self, enabled: bool) -> Self {
        self.config.store_detections = enabled;
        self
    }

    pub fn store_samples(mut self, enabled: bool) -> Self {
        self.config.store_samples = enabled;
        self
    }

    pub fn sample_rate(mut self, rate: u32) -> Self {
        self.config.sample_rate = rate;
        self
    }

    pub fn min_confidence(mut self, confidence: f32) -> Self {
        self.config.min_confidence = confidence;
        self
    }

    pub fn detection_types(mut self, types: Vec<String>) -> Self {
        self.config.detection_types = types;
        self
    }

    pub fn max_frame_age_secs(mut self, secs: u64) -> Self {
        self.config.max_frame_age_secs = secs;
        self
    }

    pub fn build(self) -> FrameSelector {
        FrameSelector::new(self.config)
    }
}

impl Default for FrameSelectorBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kafka_consumer::Detection;
    use chrono::TimeZone;
    use uuid::Uuid;

    fn create_test_event(trigger_type: TriggerType) -> StorageTriggerEvent {
        StorageTriggerEvent {
            event_id: Uuid::new_v4(),
            device_id: "test-device".to_string(),
            timestamp: Utc::now(),
            frame_number: 1,
            frame_data: vec![],
            width: 1920,
            height: 1080,
            format: "jpeg".to_string(),
            detections: vec![],
            trigger_type,
            metadata: serde_json::Value::Null,
        }
    }

    fn create_detection(detection_type: &str, confidence: f32) -> Detection {
        Detection {
            detection_type: detection_type.to_string(),
            confidence,
            bbox: [0.0, 0.0, 0.5, 0.5],
            attributes: serde_json::Value::Null,
        }
    }

    #[test]
    fn test_detection_frame_with_high_confidence() {
        let selector = FrameSelectorBuilder::new()
            .min_confidence(0.5)
            .build();

        let mut event = create_test_event(TriggerType::Detection);
        event.detections = vec![create_detection("safety_vest", 0.9)];

        match selector.should_store(&event) {
            StorageDecision::Store { reason } => {
                assert!(reason.contains("safety_vest"));
            }
            StorageDecision::Skip { reason } => {
                panic!("Expected Store, got Skip: {}", reason);
            }
        }
    }

    #[test]
    fn test_detection_frame_with_low_confidence() {
        let selector = FrameSelectorBuilder::new()
            .min_confidence(0.8)
            .build();

        let mut event = create_test_event(TriggerType::Detection);
        event.detections = vec![create_detection("safety_vest", 0.5)];

        match selector.should_store(&event) {
            StorageDecision::Skip { reason } => {
                assert!(reason.contains("confidence threshold"));
            }
            StorageDecision::Store { reason } => {
                panic!("Expected Skip, got Store: {}", reason);
            }
        }
    }

    #[test]
    fn test_detection_type_filter() {
        let selector = FrameSelectorBuilder::new()
            .detection_types(vec!["safety_vest".to_string()])
            .build();

        // Matching detection type
        let mut event = create_test_event(TriggerType::Detection);
        event.detections = vec![create_detection("safety_vest", 0.9)];

        assert!(matches!(
            selector.should_store(&event),
            StorageDecision::Store { .. }
        ));

        // Non-matching detection type
        event.detections = vec![create_detection("hard_hat", 0.9)];

        assert!(matches!(
            selector.should_store(&event),
            StorageDecision::Skip { .. }
        ));
    }

    #[test]
    fn test_sample_rate() {
        let selector = FrameSelectorBuilder::new()
            .sample_rate(3)
            .build();

        let event = create_test_event(TriggerType::Sample);

        // First frame - should store
        assert!(matches!(
            selector.should_store(&event),
            StorageDecision::Store { .. }
        ));

        // Second frame - should skip
        assert!(matches!(
            selector.should_store(&event),
            StorageDecision::Skip { .. }
        ));

        // Third frame - should skip
        assert!(matches!(
            selector.should_store(&event),
            StorageDecision::Skip { .. }
        ));

        // Fourth frame (1st of next cycle) - should store
        assert!(matches!(
            selector.should_store(&event),
            StorageDecision::Store { .. }
        ));
    }

    #[test]
    fn test_old_frame_rejection() {
        let selector = FrameSelectorBuilder::new()
            .max_frame_age_secs(60)
            .build();

        let mut event = create_test_event(TriggerType::Manual);
        event.timestamp = Utc::now() - chrono::Duration::seconds(120);

        match selector.should_store(&event) {
            StorageDecision::Skip { reason } => {
                assert!(reason.contains("too old"));
            }
            StorageDecision::Store { reason } => {
                panic!("Expected Skip, got Store: {}", reason);
            }
        }
    }

    #[test]
    fn test_manual_trigger_always_stores() {
        let selector = FrameSelectorBuilder::new().build();
        let event = create_test_event(TriggerType::Manual);

        assert!(matches!(
            selector.should_store(&event),
            StorageDecision::Store { .. }
        ));
    }

    #[test]
    fn test_alert_trigger_always_stores() {
        let selector = FrameSelectorBuilder::new().build();
        let event = create_test_event(TriggerType::Alert);

        assert!(matches!(
            selector.should_store(&event),
            StorageDecision::Store { .. }
        ));
    }
}
