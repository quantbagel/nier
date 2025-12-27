//! Frame processing and preprocessing for inference.
//!
//! This module handles decoding, resizing, format conversion, and
//! frame rate control for camera frames before sending to inference.

use crate::config::ProcessingConfig;
use crate::rtsp_client::RawFrame;
use bytes::Bytes;
use gstreamer as gst;
use gstreamer::prelude::*;
use gstreamer_video as gst_video;
use parking_lot::RwLock;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use thiserror::Error;
use tokio::sync::mpsc;
use tracing::{debug, error, info, trace, warn};

/// Errors that can occur during frame processing.
#[derive(Debug, Error)]
pub enum ProcessingError {
    #[error("Frame processing failed: {0}")]
    ProcessingFailed(String),

    #[error("Invalid frame format: {0}")]
    InvalidFormat(String),

    #[error("Resize failed: {0}")]
    ResizeFailed(String),

    #[error("Color conversion failed: {0}")]
    ColorConversionFailed(String),

    #[error("Queue full, frame dropped")]
    QueueFull,

    #[error("Processor shutdown")]
    Shutdown,
}

/// A processed frame ready for inference.
#[derive(Debug, Clone)]
pub struct ProcessedFrame {
    /// Unique frame identifier
    pub frame_id: String,

    /// Device/camera identifier
    pub device_id: String,

    /// Processed frame data
    pub data: Bytes,

    /// Frame width after processing
    pub width: u32,

    /// Frame height after processing
    pub height: u32,

    /// Pixel format (e.g., "RGB24")
    pub pixel_format: String,

    /// Original frame dimensions
    pub original_width: u32,
    pub original_height: u32,

    /// Frame sequence number
    pub sequence: u64,

    /// Timestamp when original frame was captured
    pub captured_at: Instant,

    /// Timestamp when processing completed
    pub processed_at: Instant,

    /// Processing latency in microseconds
    pub processing_latency_us: u64,
}

/// Statistics for the frame processor.
#[derive(Debug, Default, Clone)]
pub struct ProcessorStats {
    pub frames_processed: u64,
    pub frames_dropped_rate_limit: u64,
    pub frames_dropped_backpressure: u64,
    pub total_processing_time_us: u64,
    pub avg_processing_time_us: f64,
    pub last_frame_at: Option<Instant>,
}

/// Frame processor configuration for runtime adjustments.
#[derive(Debug, Clone)]
pub struct ProcessorSettings {
    pub target_width: u32,
    pub target_height: u32,
    pub target_fps: f32,
    pub drop_on_backpressure: bool,
}

impl From<&ProcessingConfig> for ProcessorSettings {
    fn from(config: &ProcessingConfig) -> Self {
        Self {
            target_width: config.target_width,
            target_height: config.target_height,
            target_fps: config.target_fps,
            drop_on_backpressure: config.drop_on_backpressure,
        }
    }
}

/// Frame processor for preparing camera frames for inference.
pub struct FrameProcessor {
    config: ProcessingConfig,
    device_id: String,
    settings: Arc<RwLock<ProcessorSettings>>,
    stats: Arc<RwLock<ProcessorStats>>,
    running: Arc<AtomicBool>,
    frame_counter: Arc<AtomicU64>,
    last_frame_time: Arc<RwLock<Option<Instant>>>,
}

impl FrameProcessor {
    /// Create a new frame processor.
    pub fn new(config: ProcessingConfig, device_id: String) -> Self {
        let settings = ProcessorSettings::from(&config);

        Self {
            config,
            device_id,
            settings: Arc::new(RwLock::new(settings)),
            stats: Arc::new(RwLock::new(ProcessorStats::default())),
            running: Arc::new(AtomicBool::new(false)),
            frame_counter: Arc::new(AtomicU64::new(0)),
            last_frame_time: Arc::new(RwLock::new(None)),
        }
    }

    /// Get current processor statistics.
    pub fn stats(&self) -> ProcessorStats {
        self.stats.read().clone()
    }

    /// Update processor settings at runtime.
    pub fn update_settings(&self, settings: ProcessorSettings) {
        *self.settings.write() = settings;
        info!(
            device_id = %self.device_id,
            width = settings.target_width,
            height = settings.target_height,
            fps = settings.target_fps,
            "Processor settings updated"
        );
    }

    /// Check if processor is running.
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// Start the frame processing pipeline.
    ///
    /// Returns a receiver for processed frames.
    pub fn start(
        &self,
        input: mpsc::Receiver<RawFrame>,
    ) -> mpsc::Receiver<ProcessedFrame> {
        let (tx, rx) = mpsc::channel(self.config.queue_size);

        self.running.store(true, Ordering::SeqCst);

        // Spawn worker tasks
        for worker_id in 0..self.config.num_workers {
            self.spawn_worker(worker_id, input.clone(), tx.clone());
        }

        // Note: The input receiver is cloned for each worker. In practice,
        // you'd want to use a proper work-stealing queue. For now, we'll
        // use a single-consumer pattern in the main start method.

        rx
    }

    /// Start with a single worker (simpler pattern).
    pub async fn run(
        &self,
        mut input: mpsc::Receiver<RawFrame>,
        output: mpsc::Sender<ProcessedFrame>,
    ) {
        self.running.store(true, Ordering::SeqCst);

        info!(
            device_id = %self.device_id,
            target_width = self.config.target_width,
            target_height = self.config.target_height,
            target_fps = self.config.target_fps,
            "Frame processor started"
        );

        while self.running.load(Ordering::SeqCst) {
            match input.recv().await {
                Some(frame) => {
                    if let Err(e) = self.process_and_send(frame, &output).await {
                        match e {
                            ProcessingError::QueueFull => {
                                self.stats.write().frames_dropped_backpressure += 1;
                            }
                            ProcessingError::Shutdown => break,
                            _ => {
                                warn!(
                                    device_id = %self.device_id,
                                    error = %e,
                                    "Frame processing error"
                                );
                            }
                        }
                    }
                }
                None => {
                    info!(device_id = %self.device_id, "Input channel closed");
                    break;
                }
            }
        }

        self.running.store(false, Ordering::SeqCst);
        info!(device_id = %self.device_id, "Frame processor stopped");
    }

    /// Process a frame and send it to the output channel.
    async fn process_and_send(
        &self,
        frame: RawFrame,
        output: &mpsc::Sender<ProcessedFrame>,
    ) -> Result<(), ProcessingError> {
        let settings = self.settings.read().clone();

        // Frame rate limiting
        if !self.should_process_frame(&settings) {
            self.stats.write().frames_dropped_rate_limit += 1;
            trace!(
                device_id = %self.device_id,
                sequence = frame.sequence,
                "Frame dropped due to rate limiting"
            );
            return Ok(());
        }

        // Process the frame
        let processed = self.process_frame(frame, &settings)?;

        // Send to output
        if settings.drop_on_backpressure {
            match output.try_send(processed) {
                Ok(()) => {}
                Err(mpsc::error::TrySendError::Full(_)) => {
                    return Err(ProcessingError::QueueFull);
                }
                Err(mpsc::error::TrySendError::Closed(_)) => {
                    return Err(ProcessingError::Shutdown);
                }
            }
        } else {
            output
                .send(processed)
                .await
                .map_err(|_| ProcessingError::Shutdown)?;
        }

        Ok(())
    }

    /// Check if we should process this frame based on target FPS.
    fn should_process_frame(&self, settings: &ProcessorSettings) -> bool {
        let min_interval = Duration::from_secs_f32(1.0 / settings.target_fps);

        let mut last_time = self.last_frame_time.write();

        match *last_time {
            Some(t) if t.elapsed() < min_interval => false,
            _ => {
                *last_time = Some(Instant::now());
                true
            }
        }
    }

    /// Process a single frame.
    fn process_frame(
        &self,
        frame: RawFrame,
        settings: &ProcessorSettings,
    ) -> Result<ProcessedFrame, ProcessingError> {
        let start = Instant::now();

        // Resize and convert if needed
        let processed_data = self.resize_and_convert(
            &frame.data,
            frame.width,
            frame.height,
            settings.target_width,
            settings.target_height,
            &frame.format,
        )?;

        let processing_time = start.elapsed();
        let processing_latency_us = processing_time.as_micros() as u64;

        // Generate frame ID
        let frame_id = format!(
            "{}-{}-{}",
            self.device_id,
            frame.sequence,
            frame.captured_at.elapsed().as_nanos()
        );

        // Update stats
        {
            let mut stats = self.stats.write();
            stats.frames_processed += 1;
            stats.total_processing_time_us += processing_latency_us;
            stats.avg_processing_time_us =
                stats.total_processing_time_us as f64 / stats.frames_processed as f64;
            stats.last_frame_at = Some(Instant::now());
        }

        debug!(
            device_id = %self.device_id,
            frame_id = %frame_id,
            original_size = format!("{}x{}", frame.width, frame.height),
            processed_size = format!("{}x{}", settings.target_width, settings.target_height),
            processing_time_us = processing_latency_us,
            "Frame processed"
        );

        Ok(ProcessedFrame {
            frame_id,
            device_id: self.device_id.clone(),
            data: Bytes::from(processed_data),
            width: settings.target_width,
            height: settings.target_height,
            pixel_format: "RGB24".to_string(),
            original_width: frame.width,
            original_height: frame.height,
            sequence: frame.sequence,
            captured_at: frame.captured_at,
            processed_at: Instant::now(),
            processing_latency_us,
        })
    }

    /// Resize and convert frame to target format.
    ///
    /// For production use, this would use GPU acceleration (CUDA, OpenCL)
    /// or optimized CPU libraries. This is a placeholder implementation.
    fn resize_and_convert(
        &self,
        data: &[u8],
        src_width: u32,
        src_height: u32,
        dst_width: u32,
        dst_height: u32,
        src_format: &str,
    ) -> Result<Vec<u8>, ProcessingError> {
        // If no resize needed and format is already RGB, return as-is
        if src_width == dst_width && src_height == dst_height && src_format == "RGB" {
            return Ok(data.to_vec());
        }

        // Simple bilinear resize implementation
        // In production, use GPU-accelerated resize or libraries like image-rs
        let dst_size = (dst_width * dst_height * 3) as usize;
        let mut output = vec![0u8; dst_size];

        let x_ratio = src_width as f32 / dst_width as f32;
        let y_ratio = src_height as f32 / dst_height as f32;

        for y in 0..dst_height {
            for x in 0..dst_width {
                let src_x = (x as f32 * x_ratio) as u32;
                let src_y = (y as f32 * y_ratio) as u32;

                let src_idx = ((src_y * src_width + src_x) * 3) as usize;
                let dst_idx = ((y * dst_width + x) * 3) as usize;

                if src_idx + 2 < data.len() && dst_idx + 2 < output.len() {
                    output[dst_idx] = data[src_idx];
                    output[dst_idx + 1] = data[src_idx + 1];
                    output[dst_idx + 2] = data[src_idx + 2];
                }
            }
        }

        Ok(output)
    }

    /// Spawn a worker task for processing frames.
    fn spawn_worker(
        &self,
        worker_id: usize,
        _input: mpsc::Receiver<RawFrame>,
        _output: mpsc::Sender<ProcessedFrame>,
    ) {
        let device_id = self.device_id.clone();
        let settings = self.settings.clone();
        let stats = self.stats.clone();
        let running = self.running.clone();
        let last_frame_time = self.last_frame_time.clone();

        tokio::spawn(async move {
            debug!(
                device_id = %device_id,
                worker_id = worker_id,
                "Frame processor worker started"
            );

            // Worker implementation would go here
            // For now, this is a placeholder showing the pattern

            while running.load(Ordering::SeqCst) {
                tokio::time::sleep(Duration::from_millis(100)).await;
            }

            debug!(
                device_id = %device_id,
                worker_id = worker_id,
                "Frame processor worker stopped"
            );
        });
    }

    /// Stop the processor.
    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }
}

/// Frame buffer for temporal operations.
pub struct FrameBuffer {
    frames: Vec<ProcessedFrame>,
    capacity: usize,
}

impl FrameBuffer {
    /// Create a new frame buffer.
    pub fn new(capacity: usize) -> Self {
        Self {
            frames: Vec::with_capacity(capacity),
            capacity,
        }
    }

    /// Add a frame to the buffer.
    pub fn push(&mut self, frame: ProcessedFrame) {
        if self.frames.len() >= self.capacity {
            self.frames.remove(0);
        }
        self.frames.push(frame);
    }

    /// Get the latest frame.
    pub fn latest(&self) -> Option<&ProcessedFrame> {
        self.frames.last()
    }

    /// Get frames within a time window.
    pub fn frames_in_window(&self, duration: Duration) -> Vec<&ProcessedFrame> {
        let cutoff = Instant::now() - duration;
        self.frames
            .iter()
            .filter(|f| f.processed_at >= cutoff)
            .collect()
    }

    /// Clear the buffer.
    pub fn clear(&mut self) {
        self.frames.clear();
    }

    /// Get buffer size.
    pub fn len(&self) -> usize {
        self.frames.len()
    }

    /// Check if buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_config() -> ProcessingConfig {
        ProcessingConfig {
            target_width: 320,
            target_height: 240,
            target_fps: 10.0,
            pixel_format: "RGB".to_string(),
            queue_size: 10,
            num_workers: 1,
            drop_on_backpressure: true,
        }
    }

    fn create_test_frame(width: u32, height: u32) -> RawFrame {
        let size = (width * height * 3) as usize;
        RawFrame {
            data: vec![128u8; size],
            width,
            height,
            pts: Some(0),
            sequence: 0,
            captured_at: Instant::now(),
            format: "RGB".to_string(),
        }
    }

    #[test]
    fn test_processor_creation() {
        let config = create_test_config();
        let processor = FrameProcessor::new(config, "test-device".to_string());
        assert!(!processor.is_running());
    }

    #[test]
    fn test_frame_resize() {
        let config = create_test_config();
        let processor = FrameProcessor::new(config, "test-device".to_string());
        let settings = processor.settings.read().clone();

        let frame = create_test_frame(640, 480);
        let result = processor.process_frame(frame, &settings);

        assert!(result.is_ok());
        let processed = result.unwrap();
        assert_eq!(processed.width, 320);
        assert_eq!(processed.height, 240);
    }

    #[test]
    fn test_frame_buffer() {
        let mut buffer = FrameBuffer::new(3);
        assert!(buffer.is_empty());

        let config = create_test_config();
        let processor = FrameProcessor::new(config, "test".to_string());
        let settings = processor.settings.read().clone();

        for i in 0..5 {
            let mut frame = create_test_frame(320, 240);
            frame.sequence = i;
            let processed = processor.process_frame(frame, &settings).unwrap();
            buffer.push(processed);
        }

        assert_eq!(buffer.len(), 3); // Capacity is 3
        assert_eq!(buffer.latest().unwrap().sequence, 4);
    }

    #[test]
    fn test_stats_update() {
        let config = create_test_config();
        let processor = FrameProcessor::new(config, "test-device".to_string());
        let settings = processor.settings.read().clone();

        let frame = create_test_frame(640, 480);
        let _ = processor.process_frame(frame, &settings);

        let stats = processor.stats();
        assert_eq!(stats.frames_processed, 1);
        assert!(stats.avg_processing_time_us > 0.0);
    }

    #[test]
    fn test_settings_update() {
        let config = create_test_config();
        let processor = FrameProcessor::new(config, "test-device".to_string());

        let new_settings = ProcessorSettings {
            target_width: 160,
            target_height: 120,
            target_fps: 5.0,
            drop_on_backpressure: false,
        };

        processor.update_settings(new_settings);

        let current = processor.settings.read().clone();
        assert_eq!(current.target_width, 160);
        assert_eq!(current.target_height, 120);
        assert_eq!(current.target_fps, 5.0);
    }
}
