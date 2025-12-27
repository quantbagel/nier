# Nier RTSP Ingest Service

RTSP video ingest service for the Nier factory floor analytics platform. This service captures video streams from worker-worn camera glasses, processes the frames, and sends them to the inference service for real-time analysis.

## Architecture

```
Camera Glasses (RTSP)
        |
        v
+------------------+
|   RtspClient     |  <- Handles RTSP connection with auto-reconnect
+------------------+
        |
        v
+------------------+
| FrameProcessor   |  <- Decodes, resizes, and rate-limits frames
+------------------+
        |
        v
+------------------+
|   GrpcClient     |  <- Batches and sends frames to inference
+------------------+
        |
        v
  Inference Service
```

## Features

- RTSP stream ingestion with H.264 decoding via GStreamer
- Automatic reconnection with exponential backoff
- Frame preprocessing (resize, format conversion)
- Configurable frame rate limiting
- Batched gRPC submission to inference service
- Comprehensive metrics and health monitoring
- Graceful shutdown handling

## Requirements

- Rust 1.70 or later
- GStreamer 1.20 or later with the following plugins:
  - gstreamer-plugins-base
  - gstreamer-plugins-good
  - gstreamer-plugins-bad (for H.264)
  - gstreamer-libav

### Installing GStreamer

**macOS:**
```bash
brew install gstreamer gst-plugins-base gst-plugins-good gst-plugins-bad gst-libav
```

**Ubuntu/Debian:**
```bash
apt-get install libgstreamer1.0-dev libgstreamer-plugins-base1.0-dev \
    gstreamer1.0-plugins-base gstreamer1.0-plugins-good \
    gstreamer1.0-plugins-bad gstreamer1.0-libav
```

## Building

```bash
cd services/ingest
cargo build --release
```

## Configuration

The service can be configured via:

1. Configuration files (`config/default.toml`, `config/{RUN_MODE}.toml`)
2. Environment variables (prefixed with `INGEST_`)

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `INGEST_RTSP__URL` | RTSP stream URL | Required |
| `INGEST_RTSP__DEVICE_ID` | Camera device identifier | Required |
| `INGEST_RTSP__WORKER_ID` | Associated worker ID | Optional |
| `INGEST_RTSP__ZONE_ID` | Factory zone identifier | Optional |
| `INGEST_RTSP__TRANSPORT` | Transport protocol (tcp/udp) | `tcp` |
| `INGEST_RTSP__MAX_RECONNECT_ATTEMPTS` | Max reconnect attempts (0=infinite) | `0` |
| `INGEST_PROCESSING__TARGET_WIDTH` | Output frame width | `640` |
| `INGEST_PROCESSING__TARGET_HEIGHT` | Output frame height | `480` |
| `INGEST_PROCESSING__TARGET_FPS` | Target frames per second | `10.0` |
| `INGEST_PROCESSING__DROP_ON_BACKPRESSURE` | Drop frames when queue full | `true` |
| `INGEST_GRPC__INFERENCE_ENDPOINT` | Inference service URL | Required |
| `INGEST_GRPC__BATCH_SIZE` | Frames per batch | `1` |
| `INGEST_LOGGING__LEVEL` | Log level (trace/debug/info/warn/error) | `info` |
| `INGEST_LOGGING__FORMAT` | Log format (json/pretty) | `json` |

### Example Configuration File

```toml
# config/default.toml

[rtsp]
url = "rtsp://camera.local:554/stream"
device_id = "camera-001"
worker_id = "worker-123"
zone_id = "assembly-line-a"
transport = "tcp"
connection_timeout_secs = 10
max_reconnect_attempts = 0
reconnect_base_delay_ms = 1000
reconnect_max_delay_ms = 30000

[processing]
target_width = 640
target_height = 480
target_fps = 10.0
pixel_format = "RGB"
queue_size = 100
num_workers = 2
drop_on_backpressure = true

[grpc]
inference_endpoint = "http://inference:50051"
request_timeout_secs = 30
connection_timeout_secs = 10
max_concurrent_requests = 10
batch_size = 4
batch_timeout_ms = 100

[logging]
level = "info"
format = "json"

[health]
interval_secs = 30
port = 8080
enable_metrics = true
```

## Running

```bash
# With environment variables
INGEST_RTSP__URL=rtsp://camera:554/stream \
INGEST_RTSP__DEVICE_ID=camera-001 \
INGEST_GRPC__INFERENCE_ENDPOINT=http://inference:50051 \
./target/release/nier-ingest

# With config file
RUN_MODE=production ./target/release/nier-ingest
```

## Docker

```dockerfile
FROM rust:1.75 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y \
    libgstreamer1.0-0 \
    gstreamer1.0-plugins-base \
    gstreamer1.0-plugins-good \
    gstreamer1.0-plugins-bad \
    gstreamer1.0-libav \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/nier-ingest /usr/local/bin/
ENTRYPOINT ["nier-ingest"]
```

## Metrics

When `health.enable_metrics` is enabled, Prometheus metrics are exposed at `http://localhost:{health.port}/metrics`:

- `ingest_frames_received_total` - Total frames received from RTSP
- `ingest_frames_dropped_total` - Frames dropped due to backpressure/rate limiting
- `ingest_frames_sent_total` - Frames sent to inference service
- `ingest_processing_latency_seconds` - Frame processing latency histogram
- `ingest_grpc_latency_seconds` - gRPC request latency histogram
- `ingest_rtsp_reconnects_total` - RTSP reconnection count

## Development

```bash
# Run tests
cargo test

# Run with debug logging
RUST_LOG=debug cargo run

# Check formatting
cargo fmt --check

# Run linter
cargo clippy
```

## Protocol Buffers

The gRPC protocol is defined in `proto/ingest.proto`. To regenerate the Rust types:

```bash
cargo build  # build.rs generates the types
```

## Troubleshooting

### GStreamer Errors

If you see GStreamer initialization errors, ensure all required plugins are installed:

```bash
gst-inspect-1.0 rtspsrc
gst-inspect-1.0 avdec_h264
```

### Connection Issues

- Verify the RTSP URL is accessible: `ffprobe rtsp://camera:554/stream`
- Check firewall rules for RTSP (554/tcp) and RTP ports
- Try UDP transport if TCP fails: `INGEST_RTSP__TRANSPORT=udp`

### Performance Tuning

- Increase `queue_size` for high-latency networks
- Decrease `target_fps` if CPU is overloaded
- Enable batching for better throughput: `batch_size > 1`
- Use `drop_on_backpressure = true` to prevent memory buildup

## License

MIT License - See LICENSE file for details.
