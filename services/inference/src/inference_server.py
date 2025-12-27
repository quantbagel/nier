"""gRPC server implementation for inference requests."""

from __future__ import annotations

import asyncio
import time
from collections import deque
from concurrent import futures
from dataclasses import dataclass
from typing import Any, AsyncIterator

import cv2
import grpc
import numpy as np
import structlog
from grpc import aio as grpc_aio

from src.config import Settings
from src.kafka_producer import DetectionKafkaProducer
from src.models.base import DetectionResult
from src.models.ppe_detector import PPEDetector

logger = structlog.get_logger(__name__)


# These would be generated from the proto file
# For now, we define placeholder types for the implementation
# In production, run: python -m grpc_tools.protoc -I./proto --python_out=./src/generated --grpc_python_out=./src/generated ./proto/inference.proto


@dataclass
class BatchItem:
    """Item in the inference batch queue."""

    image: np.ndarray
    frame_id: str
    timestamp_ms: int
    worker_id: str | None
    camera_id: str | None
    future: asyncio.Future[DetectionResult]


class InferenceBatcher:
    """Batches inference requests for efficient GPU processing.

    Collects individual requests into batches and dispatches them
    when either the batch is full or the timeout expires.
    """

    def __init__(
        self,
        detector: PPEDetector,
        kafka_producer: DetectionKafkaProducer | None,
        max_batch_size: int = 8,
        batch_timeout_ms: int = 50,
    ) -> None:
        """Initialize the batcher.

        Args:
            detector: PPE detection model.
            kafka_producer: Kafka producer for results.
            max_batch_size: Maximum items per batch.
            batch_timeout_ms: Max time to wait for batch to fill.
        """
        self.detector = detector
        self.kafka_producer = kafka_producer
        self.max_batch_size = max_batch_size
        self.batch_timeout_ms = batch_timeout_ms

        self._queue: deque[BatchItem] = deque()
        self._lock = asyncio.Lock()
        self._batch_event = asyncio.Event()
        self._running = False
        self._batch_task: asyncio.Task | None = None

        # Metrics
        self._total_requests = 0
        self._total_batches = 0

    async def start(self) -> None:
        """Start the batch processing loop."""
        if self._running:
            return

        self._running = True
        self._batch_task = asyncio.create_task(self._batch_loop())
        logger.info(
            "Inference batcher started",
            max_batch_size=self.max_batch_size,
            batch_timeout_ms=self.batch_timeout_ms,
        )

    async def stop(self) -> None:
        """Stop the batch processing loop."""
        self._running = False

        if self._batch_task:
            self._batch_event.set()  # Wake up the loop
            await self._batch_task
            self._batch_task = None

        # Cancel any pending requests
        async with self._lock:
            for item in self._queue:
                if not item.future.done():
                    item.future.cancel()
            self._queue.clear()

        logger.info("Inference batcher stopped")

    async def submit(
        self,
        image: np.ndarray,
        frame_id: str,
        timestamp_ms: int,
        worker_id: str | None = None,
        camera_id: str | None = None,
    ) -> DetectionResult:
        """Submit an image for inference.

        Args:
            image: Image array in BGR format.
            frame_id: Unique frame identifier.
            timestamp_ms: Frame timestamp.
            worker_id: Optional worker identifier.
            camera_id: Optional camera identifier.

        Returns:
            Detection result for the image.
        """
        loop = asyncio.get_event_loop()
        future: asyncio.Future[DetectionResult] = loop.create_future()

        item = BatchItem(
            image=image,
            frame_id=frame_id,
            timestamp_ms=timestamp_ms,
            worker_id=worker_id,
            camera_id=camera_id,
            future=future,
        )

        async with self._lock:
            self._queue.append(item)
            self._total_requests += 1

            # Wake up batch loop if we have a full batch
            if len(self._queue) >= self.max_batch_size:
                self._batch_event.set()

        return await future

    async def _batch_loop(self) -> None:
        """Main batch processing loop."""
        while self._running:
            try:
                # Wait for batch event or timeout
                try:
                    await asyncio.wait_for(
                        self._batch_event.wait(),
                        timeout=self.batch_timeout_ms / 1000,
                    )
                except asyncio.TimeoutError:
                    pass

                self._batch_event.clear()

                # Collect batch
                batch_items: list[BatchItem] = []

                async with self._lock:
                    while self._queue and len(batch_items) < self.max_batch_size:
                        batch_items.append(self._queue.popleft())

                if not batch_items:
                    continue

                # Process batch
                await self._process_batch(batch_items)

            except asyncio.CancelledError:
                break
            except Exception as e:
                logger.error("Error in batch loop", error=str(e))

    async def _process_batch(self, items: list[BatchItem]) -> None:
        """Process a batch of items.

        Args:
            items: List of batch items to process.
        """
        start_time = time.perf_counter()

        try:
            # Extract data from items
            images = [item.image for item in items]
            frame_ids = [item.frame_id for item in items]
            timestamps = [item.timestamp_ms for item in items]

            # Run inference
            results = await self.detector.predict(images, frame_ids, timestamps)

            # Publish to Kafka and resolve futures
            for item, result in zip(items, results):
                # Publish to Kafka if connected
                if self.kafka_producer and self.kafka_producer.is_connected:
                    await self.kafka_producer.publish_detection(
                        result,
                        worker_id=item.worker_id,
                        camera_id=item.camera_id,
                    )

                # Resolve the future
                if not item.future.done():
                    item.future.set_result(result)

            self._total_batches += 1
            batch_time = (time.perf_counter() - start_time) * 1000

            logger.debug(
                "Batch processed",
                batch_size=len(items),
                batch_time_ms=batch_time,
                avg_time_per_item_ms=batch_time / len(items),
            )

        except Exception as e:
            logger.error("Batch processing failed", error=str(e), batch_size=len(items))

            # Fail all futures in the batch
            for item in items:
                if not item.future.done():
                    item.future.set_exception(e)

    @property
    def queue_depth(self) -> int:
        """Get current queue depth."""
        return len(self._queue)

    @property
    def stats(self) -> dict[str, Any]:
        """Get batcher statistics."""
        return {
            "total_requests": self._total_requests,
            "total_batches": self._total_batches,
            "queue_depth": self.queue_depth,
            "avg_batch_size": (
                self._total_requests / self._total_batches
                if self._total_batches > 0
                else 0
            ),
        }


class InferenceServicer:
    """gRPC servicer implementing the InferenceService.

    Handles incoming gRPC requests for PPE detection inference.
    """

    def __init__(
        self,
        detector: PPEDetector,
        kafka_producer: DetectionKafkaProducer | None,
        settings: Settings,
    ) -> None:
        """Initialize the servicer.

        Args:
            detector: PPE detection model.
            kafka_producer: Kafka producer for results.
            settings: Application settings.
        """
        self.detector = detector
        self.kafka_producer = kafka_producer
        self.settings = settings

        self._batcher = InferenceBatcher(
            detector=detector,
            kafka_producer=kafka_producer,
            max_batch_size=settings.model.batch_size,
            batch_timeout_ms=settings.model.batch_timeout_ms,
        )

        self._start_time = time.time()
        self._requests_processed = 0

    async def start(self) -> None:
        """Start the servicer."""
        await self._batcher.start()

    async def stop(self) -> None:
        """Stop the servicer."""
        await self._batcher.stop()

    async def Infer(
        self,
        request: Any,
        context: grpc_aio.ServicerContext,
    ) -> Any:
        """Handle single inference request.

        Args:
            request: InferRequest protobuf message.
            context: gRPC servicer context.

        Returns:
            InferResponse protobuf message.
        """
        try:
            # Decode image
            image = self._decode_image(request.image_data, request.format)

            if image is None:
                await context.abort(
                    grpc.StatusCode.INVALID_ARGUMENT,
                    "Failed to decode image",
                )
                return None

            # Submit for inference
            result = await self._batcher.submit(
                image=image,
                frame_id=request.frame_id,
                timestamp_ms=request.timestamp_ms,
                worker_id=request.worker_id or None,
                camera_id=request.camera_id or None,
            )

            self._requests_processed += 1

            return self._build_response(result)

        except Exception as e:
            logger.error("Inference request failed", error=str(e))
            await context.abort(
                grpc.StatusCode.INTERNAL,
                f"Inference failed: {e}",
            )
            return None

    async def InferBatch(
        self,
        request: Any,
        context: grpc_aio.ServicerContext,
    ) -> Any:
        """Handle batch inference request.

        Args:
            request: InferBatchRequest protobuf message.
            context: gRPC servicer context.

        Returns:
            InferBatchResponse protobuf message.
        """
        start_time = time.perf_counter()

        try:
            # Process each request
            tasks = []
            for req in request.requests:
                image = self._decode_image(req.image_data, req.format)
                if image is not None:
                    task = self._batcher.submit(
                        image=image,
                        frame_id=req.frame_id,
                        timestamp_ms=req.timestamp_ms,
                        worker_id=req.worker_id or None,
                        camera_id=req.camera_id or None,
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
                    responses.append(self._build_response(result))
                    successful += 1

            total_time = (time.perf_counter() - start_time) * 1000
            self._requests_processed += len(request.requests)

            return {
                "responses": responses,
                "total_time_ms": total_time,
                "avg_time_per_frame_ms": total_time / len(request.requests) if request.requests else 0,
                "successful_count": successful,
                "failed_count": failed,
            }

        except Exception as e:
            logger.error("Batch inference failed", error=str(e))
            await context.abort(
                grpc.StatusCode.INTERNAL,
                f"Batch inference failed: {e}",
            )
            return None

    async def InferStream(
        self,
        request_iterator: AsyncIterator[Any],
        context: grpc_aio.ServicerContext,
    ) -> AsyncIterator[Any]:
        """Handle streaming inference requests.

        Args:
            request_iterator: Stream of InferRequest messages.
            context: gRPC servicer context.

        Yields:
            InferResponse messages.
        """
        async for request in request_iterator:
            try:
                image = self._decode_image(request.image_data, request.format)

                if image is None:
                    continue

                result = await self._batcher.submit(
                    image=image,
                    frame_id=request.frame_id,
                    timestamp_ms=request.timestamp_ms,
                    worker_id=request.worker_id or None,
                    camera_id=request.camera_id or None,
                )

                self._requests_processed += 1
                yield self._build_response(result)

            except Exception as e:
                logger.error("Stream inference failed", error=str(e))

    async def GetModelInfo(
        self,
        request: Any,
        context: grpc_aio.ServicerContext,
    ) -> Any:
        """Get model information.

        Args:
            request: GetModelInfoRequest (empty).
            context: gRPC servicer context.

        Returns:
            GetModelInfoResponse message.
        """
        gpu_memory = self.detector._check_gpu_memory()

        return {
            "model_name": "ppe-detector",
            "model_version": "1.0.0",
            "model_type": self.detector.model_type,
            "class_names": self.detector.get_class_names(),
            "input_size": {
                "width": self.detector.input_size[0],
                "height": self.detector.input_size[1],
            },
            "is_loaded": self.detector.is_loaded,
            "device": self.detector.device,
            "gpu_memory": {
                "available": gpu_memory.get("available", False),
                "total_mb": gpu_memory.get("total_mb", 0),
                "allocated_mb": gpu_memory.get("allocated_mb", 0),
                "free_mb": gpu_memory.get("free_mb", 0),
                "utilization_percent": gpu_memory.get("utilization_percent", 0),
            },
        }

    async def HealthCheck(
        self,
        request: Any,
        context: grpc_aio.ServicerContext,
    ) -> Any:
        """Check service health.

        Args:
            request: HealthCheckRequest (empty).
            context: gRPC servicer context.

        Returns:
            HealthCheckResponse message.
        """
        uptime = time.time() - self._start_time
        model_loaded = self.detector.is_loaded
        kafka_connected = (
            self.kafka_producer.is_connected
            if self.kafka_producer
            else False
        )

        # Determine overall status
        if model_loaded and (kafka_connected or self.kafka_producer is None):
            status = "HEALTH_STATUS_HEALTHY"
        elif model_loaded:
            status = "HEALTH_STATUS_DEGRADED"
        else:
            status = "HEALTH_STATUS_UNHEALTHY"

        return {
            "status": status,
            "model_loaded": model_loaded,
            "kafka_connected": kafka_connected,
            "uptime_seconds": uptime,
            "requests_processed": self._requests_processed,
            "queue_depth": self._batcher.queue_depth,
            "components": {
                "model": {
                    "healthy": model_loaded,
                    "message": "Model loaded" if model_loaded else "Model not loaded",
                    "last_check_ms": int(time.time() * 1000),
                },
                "kafka": {
                    "healthy": kafka_connected,
                    "message": "Connected" if kafka_connected else "Not connected",
                    "last_check_ms": int(time.time() * 1000),
                },
                "batcher": {
                    "healthy": True,
                    "message": f"Queue depth: {self._batcher.queue_depth}",
                    "last_check_ms": int(time.time() * 1000),
                },
            },
        }

    def _decode_image(self, image_data: bytes, format_type: int) -> np.ndarray | None:
        """Decode image bytes to numpy array.

        Args:
            image_data: Raw image bytes.
            format_type: Image format enum value.

        Returns:
            Decoded image as BGR numpy array, or None if failed.
        """
        try:
            if format_type in (0, 1, 2):  # UNSPECIFIED, JPEG, PNG
                # Decode using OpenCV
                nparr = np.frombuffer(image_data, np.uint8)
                image = cv2.imdecode(nparr, cv2.IMREAD_COLOR)
                return image

            elif format_type == 3:  # RAW_BGR
                # Assume known dimensions or parse from header
                # This is a simplified implementation
                image = np.frombuffer(image_data, dtype=np.uint8)
                # Would need dimensions from request
                return None

            elif format_type == 4:  # RAW_RGB
                image = np.frombuffer(image_data, dtype=np.uint8)
                # Convert RGB to BGR
                # Would need dimensions from request
                return None

            return None

        except Exception as e:
            logger.error("Failed to decode image", error=str(e))
            return None

    def _build_response(self, result: DetectionResult) -> dict[str, Any]:
        """Build response dict from detection result.

        Args:
            result: Detection result from model.

        Returns:
            Response dictionary matching protobuf structure.
        """
        # Build detections
        detections = []
        for det in result.detections:
            detections.append({
                "class_name": det.class_name,
                "class_id": det.class_id,
                "confidence": det.confidence,
                "bbox": {
                    "x_min": det.bbox.x_min,
                    "y_min": det.bbox.y_min,
                    "x_max": det.bbox.x_max,
                    "y_max": det.bbox.y_max,
                },
                "metadata": det.metadata,
            })

        # Build compliance summary
        violations = []
        compliant_items = []
        person_count = 0

        for det in result.detections:
            if det.class_name == "person":
                person_count += 1
            elif det.class_name.startswith("no_"):
                violations.append(det.class_name)
            elif det.class_name in ("helmet", "vest", "goggles"):
                compliant_items.append(det.class_name)

        return {
            "frame_id": result.frame_id,
            "timestamp_ms": result.timestamp_ms,
            "detections": detections,
            "inference_time_ms": result.inference_time_ms,
            "image_dimensions": {
                "width": result.image_width,
                "height": result.image_height,
            },
            "metadata": {
                "model_type": result.metadata.get("model_type", ""),
                "device": result.metadata.get("device", ""),
                "batch_size": result.metadata.get("batch_size", 1),
                "half_precision": True,
                "service_version": "0.1.0",
            },
            "compliance": {
                "has_violations": len(violations) > 0,
                "violation_count": len(violations),
                "violations": violations,
                "compliant_items": compliant_items,
                "person_count": person_count,
            },
        }


async def create_grpc_server(
    servicer: InferenceServicer,
    settings: Settings,
) -> grpc_aio.Server:
    """Create and configure gRPC server.

    Args:
        servicer: Inference servicer instance.
        settings: Application settings.

    Returns:
        Configured gRPC server (not started).
    """
    server = grpc_aio.server(
        futures.ThreadPoolExecutor(max_workers=settings.server.grpc_max_workers),
        options=[
            ("grpc.max_send_message_length", settings.server.grpc_max_message_length),
            ("grpc.max_receive_message_length", settings.server.grpc_max_message_length),
            ("grpc.keepalive_time_ms", 30000),
            ("grpc.keepalive_timeout_ms", 10000),
            ("grpc.keepalive_permit_without_calls", True),
            ("grpc.http2.max_pings_without_data", 0),
            ("grpc.http2.min_ping_interval_without_data_ms", 10000),
        ],
    )

    # In production, register the generated servicer:
    # inference_pb2_grpc.add_InferenceServiceServicer_to_server(servicer, server)

    listen_addr = f"{settings.server.grpc_host}:{settings.server.grpc_port}"
    server.add_insecure_port(listen_addr)

    logger.info("gRPC server configured", address=listen_addr)

    return server
