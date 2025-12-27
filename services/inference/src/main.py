"""Main entry point for the Nier Inference Service.

Runs both FastAPI HTTP server and gRPC server concurrently.
"""

from __future__ import annotations

import asyncio
import signal
import sys
import time
from contextlib import asynccontextmanager
from typing import Any, AsyncIterator

import numpy as np
import structlog
import uvicorn
from fastapi import FastAPI, File, Form, HTTPException, UploadFile, status
from fastapi.middleware.cors import CORSMiddleware
from fastapi.responses import JSONResponse
from prometheus_client import Counter, Gauge, Histogram, generate_latest
from pydantic import BaseModel

from src.config import Settings, get_settings
from src.inference_server import InferenceServicer, create_grpc_server
from src.kafka_producer import DetectionKafkaProducer
from src.models.ppe_detector import PPEDetector

# Configure structured logging
structlog.configure(
    processors=[
        structlog.stdlib.filter_by_level,
        structlog.stdlib.add_logger_name,
        structlog.stdlib.add_log_level,
        structlog.stdlib.PositionalArgumentsFormatter(),
        structlog.processors.TimeStamper(fmt="iso"),
        structlog.processors.StackInfoRenderer(),
        structlog.processors.format_exc_info,
        structlog.processors.UnicodeDecoder(),
        structlog.processors.JSONRenderer(),
    ],
    wrapper_class=structlog.stdlib.BoundLogger,
    context_class=dict,
    logger_factory=structlog.stdlib.LoggerFactory(),
    cache_logger_on_first_use=True,
)

logger = structlog.get_logger(__name__)

# Prometheus metrics
INFERENCE_REQUESTS = Counter(
    "nier_inference_requests_total",
    "Total number of inference requests",
    ["endpoint", "status"],
)
INFERENCE_LATENCY = Histogram(
    "nier_inference_latency_seconds",
    "Inference request latency in seconds",
    ["endpoint"],
    buckets=[0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0],
)
DETECTIONS_COUNT = Counter(
    "nier_detections_total",
    "Total number of detections",
    ["class_name"],
)
GPU_MEMORY_USED = Gauge(
    "nier_gpu_memory_used_mb",
    "GPU memory used in MB",
)
QUEUE_DEPTH = Gauge(
    "nier_inference_queue_depth",
    "Current inference queue depth",
)
MODEL_LOADED = Gauge(
    "nier_model_loaded",
    "Whether the model is loaded (1) or not (0)",
)


# Global state
class AppState:
    """Application state container."""

    def __init__(self) -> None:
        """Initialize application state."""
        self.settings: Settings | None = None
        self.detector: PPEDetector | None = None
        self.kafka_producer: DetectionKafkaProducer | None = None
        self.grpc_servicer: InferenceServicer | None = None
        self.grpc_server: Any = None
        self.start_time: float = time.time()
        self.ready: bool = False


app_state = AppState()


# Pydantic models for API
class DetectionResponse(BaseModel):
    """Single detection in response."""

    class_name: str
    class_id: int
    confidence: float
    bbox: dict[str, float]


class InferenceResponse(BaseModel):
    """Inference API response."""

    frame_id: str
    timestamp_ms: int
    detections: list[DetectionResponse]
    inference_time_ms: float
    image_width: int
    image_height: int
    detection_count: int
    compliance: dict[str, Any]


class HealthResponse(BaseModel):
    """Health check response."""

    status: str
    model_loaded: bool
    kafka_connected: bool
    uptime_seconds: float
    version: str


class ModelInfoResponse(BaseModel):
    """Model info response."""

    model_name: str
    model_type: str
    class_names: list[str]
    input_size: dict[str, int]
    is_loaded: bool
    device: str


@asynccontextmanager
async def lifespan(app: FastAPI) -> AsyncIterator[None]:
    """Application lifespan manager.

    Handles startup and shutdown of all services.
    """
    logger.info("Starting Nier Inference Service")

    try:
        # Load settings
        app_state.settings = get_settings()
        settings = app_state.settings

        logger.info(
            "Configuration loaded",
            environment=settings.environment,
            device=settings.model.device,
            batch_size=settings.model.batch_size,
        )

        # Initialize detector
        app_state.detector = PPEDetector(
            model_path=str(settings.model.ppe_model_path),
            device=settings.model.device,
            confidence_threshold=settings.model.confidence_threshold,
            iou_threshold=settings.model.iou_threshold,
            half_precision=settings.model.half_precision,
            model_type=settings.model.ppe_model_type,
        )

        # Load model
        await app_state.detector.load()
        MODEL_LOADED.set(1)

        # Warmup model
        await app_state.detector.warmup(batch_size=settings.model.batch_size)

        # Initialize Kafka producer
        app_state.kafka_producer = DetectionKafkaProducer(settings.kafka)
        try:
            await app_state.kafka_producer.connect()
        except ConnectionError as e:
            logger.warning("Kafka connection failed, continuing without Kafka", error=str(e))

        # Initialize gRPC servicer
        app_state.grpc_servicer = InferenceServicer(
            detector=app_state.detector,
            kafka_producer=app_state.kafka_producer,
            settings=settings,
        )
        await app_state.grpc_servicer.start()

        # Create and start gRPC server
        app_state.grpc_server = await create_grpc_server(
            app_state.grpc_servicer,
            settings,
        )
        await app_state.grpc_server.start()

        logger.info(
            "gRPC server started",
            port=settings.server.grpc_port,
        )

        app_state.ready = True
        logger.info("Nier Inference Service ready")

        yield

    except Exception as e:
        logger.error("Startup failed", error=str(e))
        raise

    finally:
        # Shutdown
        logger.info("Shutting down Nier Inference Service")

        app_state.ready = False

        # Stop gRPC server
        if app_state.grpc_server:
            await app_state.grpc_server.stop(grace=5)

        # Stop servicer
        if app_state.grpc_servicer:
            await app_state.grpc_servicer.stop()

        # Disconnect Kafka
        if app_state.kafka_producer:
            await app_state.kafka_producer.disconnect()

        # Unload model
        if app_state.detector:
            await app_state.detector.unload()
            MODEL_LOADED.set(0)

        logger.info("Nier Inference Service stopped")


# Create FastAPI app
app = FastAPI(
    title="Nier Inference Service",
    description="GPU-accelerated PPE detection for factory floor analytics",
    version="0.1.0",
    lifespan=lifespan,
)

# Add CORS middleware
app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)


@app.get("/health", response_model=HealthResponse)
async def health_check() -> HealthResponse:
    """Check service health."""
    uptime = time.time() - app_state.start_time
    model_loaded = app_state.detector.is_loaded if app_state.detector else False
    kafka_connected = (
        app_state.kafka_producer.is_connected
        if app_state.kafka_producer
        else False
    )

    if model_loaded and app_state.ready:
        status_str = "healthy"
    elif model_loaded:
        status_str = "degraded"
    else:
        status_str = "unhealthy"

    return HealthResponse(
        status=status_str,
        model_loaded=model_loaded,
        kafka_connected=kafka_connected,
        uptime_seconds=uptime,
        version="0.1.0",
    )


@app.get("/ready")
async def readiness_check() -> JSONResponse:
    """Kubernetes readiness probe."""
    if app_state.ready:
        return JSONResponse({"ready": True})
    return JSONResponse({"ready": False}, status_code=status.HTTP_503_SERVICE_UNAVAILABLE)


@app.get("/live")
async def liveness_check() -> JSONResponse:
    """Kubernetes liveness probe."""
    return JSONResponse({"alive": True})


@app.get("/metrics")
async def metrics() -> bytes:
    """Prometheus metrics endpoint."""
    # Update metrics
    if app_state.detector:
        gpu_info = app_state.detector._check_gpu_memory()
        if gpu_info.get("available"):
            GPU_MEMORY_USED.set(gpu_info.get("allocated_mb", 0))

    if app_state.grpc_servicer:
        QUEUE_DEPTH.set(app_state.grpc_servicer._batcher.queue_depth)

    return generate_latest()


@app.get("/model/info", response_model=ModelInfoResponse)
async def get_model_info() -> ModelInfoResponse:
    """Get model information."""
    if not app_state.detector:
        raise HTTPException(
            status_code=status.HTTP_503_SERVICE_UNAVAILABLE,
            detail="Model not initialized",
        )

    return ModelInfoResponse(
        model_name="ppe-detector",
        model_type=app_state.detector.model_type,
        class_names=app_state.detector.get_class_names(),
        input_size={
            "width": app_state.detector.input_size[0],
            "height": app_state.detector.input_size[1],
        },
        is_loaded=app_state.detector.is_loaded,
        device=app_state.detector.device,
    )


@app.post("/infer", response_model=InferenceResponse)
async def infer(
    file: UploadFile = File(...),
    frame_id: str = Form(...),
    timestamp_ms: int = Form(default=0),
    worker_id: str | None = Form(default=None),
    camera_id: str | None = Form(default=None),
) -> InferenceResponse:
    """Run PPE detection on a single image.

    Args:
        file: Image file (JPEG or PNG).
        frame_id: Unique frame identifier.
        timestamp_ms: Frame timestamp in milliseconds.
        worker_id: Optional worker identifier.
        camera_id: Optional camera identifier.

    Returns:
        Detection results.
    """
    if not app_state.ready or not app_state.grpc_servicer:
        raise HTTPException(
            status_code=status.HTTP_503_SERVICE_UNAVAILABLE,
            detail="Service not ready",
        )

    start_time = time.perf_counter()

    try:
        # Read image
        import cv2

        contents = await file.read()
        nparr = np.frombuffer(contents, np.uint8)
        image = cv2.imdecode(nparr, cv2.IMREAD_COLOR)

        if image is None:
            INFERENCE_REQUESTS.labels(endpoint="infer", status="error").inc()
            raise HTTPException(
                status_code=status.HTTP_400_BAD_REQUEST,
                detail="Failed to decode image",
            )

        # Get timestamp
        if timestamp_ms == 0:
            timestamp_ms = int(time.time() * 1000)

        # Run inference through batcher
        result = await app_state.grpc_servicer._batcher.submit(
            image=image,
            frame_id=frame_id,
            timestamp_ms=timestamp_ms,
            worker_id=worker_id,
            camera_id=camera_id,
        )

        # Update metrics
        INFERENCE_REQUESTS.labels(endpoint="infer", status="success").inc()
        INFERENCE_LATENCY.labels(endpoint="infer").observe(
            time.perf_counter() - start_time
        )

        for det in result.detections:
            DETECTIONS_COUNT.labels(class_name=det.class_name).inc()

        # Build compliance summary
        violations = []
        compliant_items = []

        for det in result.detections:
            if det.class_name.startswith("no_"):
                violations.append(det.class_name)
            elif det.class_name in ("helmet", "vest", "goggles"):
                compliant_items.append(det.class_name)

        return InferenceResponse(
            frame_id=result.frame_id,
            timestamp_ms=result.timestamp_ms,
            detections=[
                DetectionResponse(
                    class_name=d.class_name,
                    class_id=d.class_id,
                    confidence=d.confidence,
                    bbox=d.bbox.to_dict(),
                )
                for d in result.detections
            ],
            inference_time_ms=result.inference_time_ms,
            image_width=result.image_width,
            image_height=result.image_height,
            detection_count=result.detection_count,
            compliance={
                "has_violations": len(violations) > 0,
                "violation_count": len(violations),
                "violations": violations,
                "compliant_items": compliant_items,
            },
        )

    except HTTPException:
        raise
    except Exception as e:
        INFERENCE_REQUESTS.labels(endpoint="infer", status="error").inc()
        logger.error("Inference failed", error=str(e))
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Inference failed: {e}",
        ) from e


@app.post("/infer/batch")
async def infer_batch(
    files: list[UploadFile] = File(...),
    frame_ids: str = Form(...),  # Comma-separated
    timestamps: str = Form(default=""),  # Comma-separated
) -> dict[str, Any]:
    """Run PPE detection on multiple images.

    Args:
        files: List of image files.
        frame_ids: Comma-separated frame identifiers.
        timestamps: Comma-separated timestamps (optional).

    Returns:
        Batch detection results.
    """
    if not app_state.ready or not app_state.grpc_servicer:
        raise HTTPException(
            status_code=status.HTTP_503_SERVICE_UNAVAILABLE,
            detail="Service not ready",
        )

    start_time = time.perf_counter()

    try:
        import cv2

        # Parse frame IDs and timestamps
        frame_id_list = [fid.strip() for fid in frame_ids.split(",")]
        timestamp_list = (
            [int(ts.strip()) for ts in timestamps.split(",") if ts.strip()]
            if timestamps
            else [int(time.time() * 1000)] * len(files)
        )

        if len(files) != len(frame_id_list):
            raise HTTPException(
                status_code=status.HTTP_400_BAD_REQUEST,
                detail="Number of files must match number of frame IDs",
            )

        # Process each image
        tasks = []
        for file, frame_id, ts in zip(files, frame_id_list, timestamp_list):
            contents = await file.read()
            nparr = np.frombuffer(contents, np.uint8)
            image = cv2.imdecode(nparr, cv2.IMREAD_COLOR)

            if image is not None:
                task = app_state.grpc_servicer._batcher.submit(
                    image=image,
                    frame_id=frame_id,
                    timestamp_ms=ts,
                )
                tasks.append(task)

        # Wait for all results
        results = await asyncio.gather(*tasks, return_exceptions=True)

        # Build response
        responses = []
        successful = 0
        failed = 0

        for result in results:
            if isinstance(result, Exception):
                failed += 1
            else:
                responses.append(result.to_dict())
                successful += 1
                INFERENCE_REQUESTS.labels(endpoint="infer_batch", status="success").inc()

        total_time = (time.perf_counter() - start_time) * 1000

        return {
            "results": responses,
            "total_time_ms": total_time,
            "avg_time_per_frame_ms": total_time / len(files) if files else 0,
            "successful_count": successful,
            "failed_count": failed,
        }

    except HTTPException:
        raise
    except Exception as e:
        INFERENCE_REQUESTS.labels(endpoint="infer_batch", status="error").inc()
        logger.error("Batch inference failed", error=str(e))
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Batch inference failed: {e}",
        ) from e


def main() -> None:
    """Run the inference service."""
    settings = get_settings()

    # Configure logging level
    import logging

    logging.basicConfig(level=getattr(logging, settings.logging.level))

    # Handle shutdown signals
    loop = asyncio.new_event_loop()
    asyncio.set_event_loop(loop)

    shutdown_event = asyncio.Event()

    def handle_signal(sig: signal.Signals) -> None:
        logger.info("Received shutdown signal", signal=sig.name)
        shutdown_event.set()

    for sig in (signal.SIGTERM, signal.SIGINT):
        loop.add_signal_handler(sig, handle_signal, sig)

    # Run uvicorn
    config = uvicorn.Config(
        app="src.main:app",
        host=settings.server.http_host,
        port=settings.server.http_port,
        loop="asyncio",
        log_level=settings.logging.level.lower(),
        access_log=True,
    )

    server = uvicorn.Server(config)

    logger.info(
        "Starting HTTP server",
        host=settings.server.http_host,
        port=settings.server.http_port,
    )

    try:
        loop.run_until_complete(server.serve())
    except KeyboardInterrupt:
        logger.info("Keyboard interrupt received")
    finally:
        loop.close()


if __name__ == "__main__":
    main()
