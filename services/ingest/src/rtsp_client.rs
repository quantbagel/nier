//! RTSP client with automatic reconnection and stream management.
//!
//! This module handles connecting to RTSP streams from worker camera glasses,
//! managing the GStreamer pipeline, and providing frames to the processing pipeline.

use crate::config::RtspConfig;
use backoff::{backoff::Backoff, ExponentialBackoff};
use gstreamer as gst;
use gstreamer::prelude::*;
use gstreamer_app as gst_app;
use parking_lot::RwLock;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use thiserror::Error;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

/// Errors that can occur during RTSP operations.
#[derive(Debug, Error)]
pub enum RtspError {
    #[error("GStreamer initialization failed: {0}")]
    GstreamerInit(String),

    #[error("Pipeline creation failed: {0}")]
    PipelineCreation(String),

    #[error("Pipeline element not found: {0}")]
    ElementNotFound(String),

    #[error("Stream connection failed: {0}")]
    ConnectionFailed(String),

    #[error("Stream disconnected unexpectedly")]
    Disconnected,

    #[error("Maximum reconnection attempts exceeded")]
    MaxReconnectAttemptsExceeded,

    #[error("Pipeline state change failed: {0}")]
    StateChangeFailed(String),

    #[error("Frame extraction failed: {0}")]
    FrameExtractionFailed(String),
}

/// A raw frame extracted from the RTSP stream.
#[derive(Debug, Clone)]
pub struct RawFrame {
    /// Frame data as bytes
    pub data: Vec<u8>,

    /// Frame width in pixels
    pub width: u32,

    /// Frame height in pixels
    pub height: u32,

    /// Presentation timestamp
    pub pts: Option<u64>,

    /// Frame sequence number
    pub sequence: u64,

    /// Timestamp when frame was captured
    pub captured_at: Instant,

    /// Pixel format
    pub format: String,
}

/// Statistics for the RTSP stream.
#[derive(Debug, Default, Clone)]
pub struct StreamStats {
    pub frames_received: u64,
    pub frames_dropped: u64,
    pub bytes_received: u64,
    pub reconnect_count: u32,
    pub last_frame_at: Option<Instant>,
    pub stream_start: Option<Instant>,
    pub current_fps: f64,
}

/// State of the RTSP connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting,
    Failed,
}

/// RTSP client for managing camera streams.
pub struct RtspClient {
    config: RtspConfig,
    pipeline: Option<gst::Pipeline>,
    state: Arc<RwLock<ConnectionState>>,
    running: Arc<AtomicBool>,
    frame_sequence: Arc<AtomicU64>,
    stats: Arc<RwLock<StreamStats>>,
    frame_sender: Option<mpsc::Sender<RawFrame>>,
}

impl RtspClient {
    /// Create a new RTSP client with the given configuration.
    pub fn new(config: RtspConfig) -> Result<Self, RtspError> {
        // Initialize GStreamer
        gst::init().map_err(|e| RtspError::GstreamerInit(e.to_string()))?;

        Ok(Self {
            config,
            pipeline: None,
            state: Arc::new(RwLock::new(ConnectionState::Disconnected)),
            running: Arc::new(AtomicBool::new(false)),
            frame_sequence: Arc::new(AtomicU64::new(0)),
            stats: Arc::new(RwLock::new(StreamStats::default())),
            frame_sender: None,
        })
    }

    /// Get the current connection state.
    pub fn state(&self) -> ConnectionState {
        *self.state.read()
    }

    /// Get current stream statistics.
    pub fn stats(&self) -> StreamStats {
        self.stats.read().clone()
    }

    /// Check if the client is running.
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// Start the RTSP stream and return a receiver for frames.
    pub async fn start(&mut self) -> Result<mpsc::Receiver<RawFrame>, RtspError> {
        let (tx, rx) = mpsc::channel(self.config.buffer_ms as usize);
        self.frame_sender = Some(tx);
        self.running.store(true, Ordering::SeqCst);

        // Connect with retry logic
        self.connect_with_retry().await?;

        // Start the frame extraction loop
        self.start_frame_loop();

        Ok(rx)
    }

    /// Stop the RTSP stream.
    pub async fn stop(&mut self) {
        info!(device_id = %self.config.device_id, "Stopping RTSP client");
        self.running.store(false, Ordering::SeqCst);

        if let Some(pipeline) = self.pipeline.take() {
            let _ = pipeline.set_state(gst::State::Null);
        }

        *self.state.write() = ConnectionState::Disconnected;
        self.frame_sender = None;
    }

    /// Connect to the RTSP stream with exponential backoff retry.
    async fn connect_with_retry(&mut self) -> Result<(), RtspError> {
        let mut backoff = ExponentialBackoff {
            initial_interval: self.config.reconnect_base_delay(),
            max_interval: self.config.reconnect_max_delay(),
            max_elapsed_time: None, // Retry forever unless max_reconnect_attempts is set
            ..Default::default()
        };

        let mut attempts = 0u32;
        let max_attempts = self.config.max_reconnect_attempts;

        loop {
            if !self.running.load(Ordering::SeqCst) {
                return Err(RtspError::Disconnected);
            }

            *self.state.write() = if attempts == 0 {
                ConnectionState::Connecting
            } else {
                ConnectionState::Reconnecting
            };

            match self.create_and_start_pipeline() {
                Ok(()) => {
                    *self.state.write() = ConnectionState::Connected;
                    info!(
                        device_id = %self.config.device_id,
                        url = %self.config.url,
                        attempts = attempts,
                        "Connected to RTSP stream"
                    );
                    return Ok(());
                }
                Err(e) => {
                    attempts += 1;
                    self.stats.write().reconnect_count = attempts;

                    if max_attempts > 0 && attempts >= max_attempts {
                        *self.state.write() = ConnectionState::Failed;
                        error!(
                            device_id = %self.config.device_id,
                            attempts = attempts,
                            error = %e,
                            "Max reconnection attempts exceeded"
                        );
                        return Err(RtspError::MaxReconnectAttemptsExceeded);
                    }

                    if let Some(delay) = backoff.next_backoff() {
                        warn!(
                            device_id = %self.config.device_id,
                            attempt = attempts,
                            delay_ms = delay.as_millis(),
                            error = %e,
                            "Connection failed, retrying"
                        );
                        tokio::time::sleep(delay).await;
                    } else {
                        backoff.reset();
                    }
                }
            }
        }
    }

    /// Create and start the GStreamer pipeline.
    fn create_and_start_pipeline(&mut self) -> Result<(), RtspError> {
        let pipeline_str = self.build_pipeline_string();
        debug!(pipeline = %pipeline_str, "Creating GStreamer pipeline");

        let pipeline = gst::parse::launch(&pipeline_str)
            .map_err(|e| RtspError::PipelineCreation(e.to_string()))?
            .downcast::<gst::Pipeline>()
            .map_err(|_| RtspError::PipelineCreation("Failed to cast to Pipeline".to_string()))?;

        // Get the appsink element
        let appsink = pipeline
            .by_name("sink")
            .ok_or_else(|| RtspError::ElementNotFound("appsink".to_string()))?
            .downcast::<gst_app::AppSink>()
            .map_err(|_| RtspError::ElementNotFound("Could not cast to AppSink".to_string()))?;

        // Configure appsink callbacks
        self.configure_appsink(&appsink)?;

        // Start the pipeline
        pipeline
            .set_state(gst::State::Playing)
            .map_err(|e| RtspError::StateChangeFailed(e.to_string()))?;

        // Wait for state change to complete
        let (result, _state, _pending) = pipeline.state(gst::ClockTime::from_seconds(
            self.config.connection_timeout_secs,
        ));

        if result.is_err() {
            let _ = pipeline.set_state(gst::State::Null);
            return Err(RtspError::ConnectionFailed(
                "Timeout waiting for pipeline to start".to_string(),
            ));
        }

        self.pipeline = Some(pipeline);
        self.stats.write().stream_start = Some(Instant::now());

        Ok(())
    }

    /// Build the GStreamer pipeline string.
    fn build_pipeline_string(&self) -> String {
        let transport = match self.config.transport.as_str() {
            "udp" => "0",
            "udp-mcast" => "1",
            _ => "2", // tcp
        };

        format!(
            "rtspsrc location={url} protocols={transport} latency={latency} \
             ! rtph264depay ! h264parse ! avdec_h264 \
             ! videoconvert ! videoscale \
             ! video/x-raw,format=RGB,width={width},height={height} \
             ! appsink name=sink emit-signals=true sync=false max-buffers=2 drop=true",
            url = self.config.url,
            transport = transport,
            latency = self.config.buffer_ms,
            width = 640,  // Default, will be overridden by processor
            height = 480, // Default, will be overridden by processor
        )
    }

    /// Configure the appsink with callbacks for frame handling.
    fn configure_appsink(&self, appsink: &gst_app::AppSink) -> Result<(), RtspError> {
        let sender = self
            .frame_sender
            .clone()
            .ok_or_else(|| RtspError::FrameExtractionFailed("No frame sender".to_string()))?;
        let sequence = self.frame_sequence.clone();
        let stats = self.stats.clone();
        let running = self.running.clone();
        let device_id = self.config.device_id.clone();

        appsink.set_callbacks(
            gst_app::AppSinkCallbacks::builder()
                .new_sample(move |sink| {
                    if !running.load(Ordering::SeqCst) {
                        return Err(gst::FlowError::Eos);
                    }

                    let sample = sink.pull_sample().map_err(|_| gst::FlowError::Error)?;
                    let buffer = sample.buffer().ok_or(gst::FlowError::Error)?;
                    let caps = sample.caps().ok_or(gst::FlowError::Error)?;

                    // Extract frame dimensions from caps
                    let structure = caps.structure(0).ok_or(gst::FlowError::Error)?;
                    let width: i32 = structure.get("width").unwrap_or(640);
                    let height: i32 = structure.get("height").unwrap_or(480);
                    let format: String = structure
                        .get::<&str>("format")
                        .unwrap_or("RGB")
                        .to_string();

                    // Map buffer to read data
                    let map = buffer.map_readable().map_err(|_| gst::FlowError::Error)?;
                    let data = map.as_slice().to_vec();

                    let seq = sequence.fetch_add(1, Ordering::SeqCst);

                    let frame = RawFrame {
                        data,
                        width: width as u32,
                        height: height as u32,
                        pts: buffer.pts().map(|t| t.nseconds()),
                        sequence: seq,
                        captured_at: Instant::now(),
                        format,
                    };

                    // Update stats
                    {
                        let mut s = stats.write();
                        s.frames_received += 1;
                        s.bytes_received += frame.data.len() as u64;
                        s.last_frame_at = Some(Instant::now());

                        // Calculate FPS
                        if let Some(start) = s.stream_start {
                            let elapsed = start.elapsed().as_secs_f64();
                            if elapsed > 0.0 {
                                s.current_fps = s.frames_received as f64 / elapsed;
                            }
                        }
                    }

                    // Send frame to channel
                    match sender.try_send(frame) {
                        Ok(()) => Ok(gst::FlowSuccess::Ok),
                        Err(mpsc::error::TrySendError::Full(_)) => {
                            stats.write().frames_dropped += 1;
                            debug!(device_id = %device_id, "Frame dropped due to backpressure");
                            Ok(gst::FlowSuccess::Ok)
                        }
                        Err(mpsc::error::TrySendError::Closed(_)) => Err(gst::FlowError::Eos),
                    }
                })
                .build(),
        );

        Ok(())
    }

    /// Start the frame extraction loop with reconnection handling.
    fn start_frame_loop(&self) {
        let pipeline = match &self.pipeline {
            Some(p) => p.clone(),
            None => return,
        };

        let state = self.state.clone();
        let running = self.running.clone();
        let device_id = self.config.device_id.clone();

        // Spawn a task to monitor the pipeline bus for errors
        tokio::spawn(async move {
            let bus = match pipeline.bus() {
                Some(b) => b,
                None => return,
            };

            loop {
                if !running.load(Ordering::SeqCst) {
                    break;
                }

                // Poll for messages with timeout
                if let Some(msg) = bus.timed_pop(gst::ClockTime::from_mseconds(100)) {
                    match msg.view() {
                        gst::MessageView::Error(err) => {
                            error!(
                                device_id = %device_id,
                                error = %err.error(),
                                debug = ?err.debug(),
                                "GStreamer pipeline error"
                            );
                            *state.write() = ConnectionState::Disconnected;
                            break;
                        }
                        gst::MessageView::Eos(_) => {
                            info!(device_id = %device_id, "End of stream");
                            *state.write() = ConnectionState::Disconnected;
                            break;
                        }
                        gst::MessageView::StateChanged(s) => {
                            if let Some(element) = msg.src() {
                                if element.type_() == gst::Pipeline::static_type() {
                                    debug!(
                                        device_id = %device_id,
                                        old = ?s.old(),
                                        new = ?s.current(),
                                        "Pipeline state changed"
                                    );
                                }
                            }
                        }
                        gst::MessageView::Warning(w) => {
                            warn!(
                                device_id = %device_id,
                                warning = %w.error(),
                                "GStreamer warning"
                            );
                        }
                        _ => {}
                    }
                }

                tokio::task::yield_now().await;
            }
        });
    }

    /// Reconnect to the stream after a disconnection.
    pub async fn reconnect(&mut self) -> Result<(), RtspError> {
        // Stop existing pipeline
        if let Some(pipeline) = self.pipeline.take() {
            let _ = pipeline.set_state(gst::State::Null);
        }

        // Reset sequence counter for new connection
        self.frame_sequence.store(0, Ordering::SeqCst);

        // Reconnect with retry logic
        self.connect_with_retry().await?;

        // Restart frame loop
        self.start_frame_loop();

        Ok(())
    }
}

impl Drop for RtspClient {
    fn drop(&mut self) {
        self.running.store(false, Ordering::SeqCst);
        if let Some(pipeline) = self.pipeline.take() {
            let _ = pipeline.set_state(gst::State::Null);
        }
    }
}

/// Handle for monitoring and controlling an active RTSP stream.
pub struct StreamHandle {
    client: Arc<RwLock<RtspClient>>,
    frame_receiver: mpsc::Receiver<RawFrame>,
}

impl StreamHandle {
    /// Create a new stream handle.
    pub fn new(client: Arc<RwLock<RtspClient>>, receiver: mpsc::Receiver<RawFrame>) -> Self {
        Self {
            client,
            frame_receiver: receiver,
        }
    }

    /// Receive the next frame.
    pub async fn recv(&mut self) -> Option<RawFrame> {
        self.frame_receiver.recv().await
    }

    /// Try to receive a frame without blocking.
    pub fn try_recv(&mut self) -> Result<RawFrame, mpsc::error::TryRecvError> {
        self.frame_receiver.try_recv()
    }

    /// Get current stream statistics.
    pub fn stats(&self) -> StreamStats {
        self.client.read().stats()
    }

    /// Get current connection state.
    pub fn state(&self) -> ConnectionState {
        self.client.read().state()
    }

    /// Check if stream is connected.
    pub fn is_connected(&self) -> bool {
        self.state() == ConnectionState::Connected
    }

    /// Request reconnection.
    pub async fn reconnect(&self) -> Result<(), RtspError> {
        self.client.write().reconnect().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_config() -> RtspConfig {
        RtspConfig {
            url: "rtsp://test:554/stream".to_string(),
            device_id: "test-device".to_string(),
            worker_id: None,
            zone_id: None,
            connection_timeout_secs: 5,
            max_reconnect_attempts: 3,
            reconnect_base_delay_ms: 100,
            reconnect_max_delay_ms: 1000,
            transport: "tcp".to_string(),
            buffer_ms: 100,
        }
    }

    #[test]
    fn test_connection_state_default() {
        let config = create_test_config();
        let client = RtspClient::new(config).unwrap();
        assert_eq!(client.state(), ConnectionState::Disconnected);
    }

    #[test]
    fn test_stats_default() {
        let config = create_test_config();
        let client = RtspClient::new(config).unwrap();
        let stats = client.stats();
        assert_eq!(stats.frames_received, 0);
        assert_eq!(stats.reconnect_count, 0);
    }

    #[test]
    fn test_pipeline_string_tcp() {
        let config = create_test_config();
        let client = RtspClient::new(config).unwrap();
        let pipeline = client.build_pipeline_string();
        assert!(pipeline.contains("protocols=2")); // TCP
        assert!(pipeline.contains("rtsp://test:554/stream"));
    }

    #[test]
    fn test_pipeline_string_udp() {
        let mut config = create_test_config();
        config.transport = "udp".to_string();
        let client = RtspClient::new(config).unwrap();
        let pipeline = client.build_pipeline_string();
        assert!(pipeline.contains("protocols=0")); // UDP
    }
}
