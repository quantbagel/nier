"""PPE detection model wrapper for helmet, vest, and goggles detection."""

from __future__ import annotations

import asyncio
import time
from pathlib import Path
from typing import Any

import numpy as np
import structlog
import torch

from src.models.base import (
    BaseDetector,
    BoundingBox,
    Detection,
    DetectionResult,
    PPEClass,
)

logger = structlog.get_logger(__name__)


# PPE class mapping - maps model output indices to PPE classes
PPE_CLASS_MAP: dict[int, str] = {
    0: PPEClass.PERSON.value,
    1: PPEClass.HELMET.value,
    2: PPEClass.NO_HELMET.value,
    3: PPEClass.VEST.value,
    4: PPEClass.NO_VEST.value,
    5: PPEClass.GOGGLES.value,
    6: PPEClass.NO_GOGGLES.value,
}


class PPEDetector(BaseDetector):
    """PPE detection model using YOLOv8 or ONNX Runtime.

    Detects personal protective equipment (helmet, vest, goggles)
    and their absence on factory floor workers.
    """

    def __init__(
        self,
        model_path: str,
        device: str = "cuda:0",
        confidence_threshold: float = 0.5,
        iou_threshold: float = 0.45,
        half_precision: bool = True,
        model_type: str = "yolo",
        input_size: tuple[int, int] = (640, 640),
    ) -> None:
        """Initialize PPE detector.

        Args:
            model_path: Path to the model weights.
            device: Device to run inference on.
            confidence_threshold: Minimum confidence for detections.
            iou_threshold: IoU threshold for NMS.
            half_precision: Whether to use FP16 inference.
            model_type: Type of model ('yolo' or 'onnx').
            input_size: Model input size (width, height).
        """
        super().__init__(
            model_path=model_path,
            device=device,
            confidence_threshold=confidence_threshold,
            iou_threshold=iou_threshold,
            half_precision=half_precision,
        )
        self.model_type = model_type
        self.input_size = input_size
        self._onnx_session: Any = None
        self._lock = asyncio.Lock()

    async def load(self) -> None:
        """Load the PPE detection model into memory."""
        async with self._lock:
            if self._is_loaded:
                logger.warning("Model already loaded, skipping")
                return

            logger.info(
                "Loading PPE detection model",
                model_path=self.model_path,
                device=self.device,
                model_type=self.model_type,
            )

            start_time = time.perf_counter()

            try:
                if self.model_type == "yolo":
                    await self._load_yolo_model()
                elif self.model_type == "onnx":
                    await self._load_onnx_model()
                else:
                    raise ValueError(f"Unsupported model type: {self.model_type}")

                self._is_loaded = True
                load_time = (time.perf_counter() - start_time) * 1000

                logger.info(
                    "PPE detection model loaded successfully",
                    load_time_ms=load_time,
                    gpu_memory=self._check_gpu_memory(),
                )

            except Exception as e:
                logger.error("Failed to load PPE detection model", error=str(e))
                raise

    async def _load_yolo_model(self) -> None:
        """Load YOLO model using ultralytics."""
        from ultralytics import YOLO

        # Run model loading in thread pool to avoid blocking
        loop = asyncio.get_event_loop()
        self._model = await loop.run_in_executor(
            None,
            lambda: YOLO(self.model_path),
        )

        # Move to device and set precision
        if "cuda" in self.device:
            self._model.to(self.device)
            if self.half_precision:
                self._model.model.half()

    async def _load_onnx_model(self) -> None:
        """Load ONNX model using ONNX Runtime."""
        import onnxruntime as ort

        # Configure ONNX Runtime session
        providers = []
        if "cuda" in self.device:
            device_id = int(self.device.split(":")[-1]) if ":" in self.device else 0
            providers.append(
                (
                    "CUDAExecutionProvider",
                    {
                        "device_id": device_id,
                        "arena_extend_strategy": "kNextPowerOfTwo",
                        "gpu_mem_limit": 4 * 1024 * 1024 * 1024,  # 4GB limit
                        "cudnn_conv_algo_search": "EXHAUSTIVE",
                    },
                )
            )
        providers.append("CPUExecutionProvider")

        sess_options = ort.SessionOptions()
        sess_options.graph_optimization_level = ort.GraphOptimizationLevel.ORT_ENABLE_ALL
        sess_options.intra_op_num_threads = 4
        sess_options.inter_op_num_threads = 4

        # Run session creation in thread pool
        loop = asyncio.get_event_loop()
        self._onnx_session = await loop.run_in_executor(
            None,
            lambda: ort.InferenceSession(
                self.model_path,
                sess_options=sess_options,
                providers=providers,
            ),
        )
        self._model = self._onnx_session

    async def unload(self) -> None:
        """Unload the model and free GPU memory."""
        async with self._lock:
            if not self._is_loaded:
                logger.warning("Model not loaded, nothing to unload")
                return

            logger.info("Unloading PPE detection model")

            self._model = None
            self._onnx_session = None
            self._is_loaded = False
            self._clear_gpu_cache()

            logger.info(
                "PPE detection model unloaded",
                gpu_memory=self._check_gpu_memory(),
            )

    async def predict(
        self,
        images: list[np.ndarray],
        frame_ids: list[str],
        timestamps: list[int],
    ) -> list[DetectionResult]:
        """Run PPE detection on a batch of images.

        Args:
            images: List of images as numpy arrays (BGR format).
            frame_ids: List of unique frame identifiers.
            timestamps: List of timestamps in milliseconds.

        Returns:
            List of detection results, one per image.
        """
        if not self._is_loaded:
            raise RuntimeError("Model not loaded. Call load() first.")

        if len(images) != len(frame_ids) or len(images) != len(timestamps):
            raise ValueError("Length mismatch between images, frame_ids, and timestamps")

        if len(images) == 0:
            return []

        start_time = time.perf_counter()

        try:
            if self.model_type == "yolo":
                results = await self._predict_yolo(images)
            else:
                results = await self._predict_onnx(images)

            inference_time = (time.perf_counter() - start_time) * 1000

            # Build detection results
            detection_results = []
            per_image_time = inference_time / len(images)

            for idx, (image, frame_id, timestamp) in enumerate(
                zip(images, frame_ids, timestamps)
            ):
                height, width = image.shape[:2]
                detections = self._parse_detections(results[idx], width, height)

                detection_results.append(
                    DetectionResult(
                        frame_id=frame_id,
                        timestamp_ms=timestamp,
                        detections=detections,
                        inference_time_ms=per_image_time,
                        image_width=width,
                        image_height=height,
                        metadata={
                            "model_type": self.model_type,
                            "batch_size": len(images),
                            "device": self.device,
                        },
                    )
                )

            logger.debug(
                "Batch inference completed",
                batch_size=len(images),
                total_inference_time_ms=inference_time,
                total_detections=sum(r.detection_count for r in detection_results),
            )

            return detection_results

        except Exception as e:
            logger.error(
                "Inference failed",
                error=str(e),
                batch_size=len(images),
            )
            raise

    async def _predict_yolo(
        self,
        images: list[np.ndarray],
    ) -> list[Any]:
        """Run YOLO inference on batch of images."""
        loop = asyncio.get_event_loop()

        # Run inference in thread pool to avoid blocking
        results = await loop.run_in_executor(
            None,
            lambda: self._model.predict(
                images,
                conf=self.confidence_threshold,
                iou=self.iou_threshold,
                verbose=False,
                device=self.device,
            ),
        )

        return results

    async def _predict_onnx(
        self,
        images: list[np.ndarray],
    ) -> list[np.ndarray]:
        """Run ONNX inference on batch of images."""
        import cv2

        # Preprocess images
        preprocessed = []
        for image in images:
            # Resize to model input size
            resized = cv2.resize(image, self.input_size)
            # Convert BGR to RGB
            rgb = cv2.cvtColor(resized, cv2.COLOR_BGR2RGB)
            # Normalize to [0, 1]
            normalized = rgb.astype(np.float32) / 255.0
            # Transpose to CHW format
            transposed = np.transpose(normalized, (2, 0, 1))
            preprocessed.append(transposed)

        # Stack into batch
        batch = np.stack(preprocessed, axis=0)

        if self.half_precision:
            batch = batch.astype(np.float16)

        # Run inference in thread pool
        loop = asyncio.get_event_loop()
        input_name = self._onnx_session.get_inputs()[0].name

        outputs = await loop.run_in_executor(
            None,
            lambda: self._onnx_session.run(None, {input_name: batch}),
        )

        return outputs[0]

    def _parse_detections(
        self,
        raw_result: Any,
        image_width: int,
        image_height: int,
    ) -> list[Detection]:
        """Parse raw model output into Detection objects.

        Args:
            raw_result: Raw model output.
            image_width: Original image width.
            image_height: Original image height.

        Returns:
            List of Detection objects.
        """
        detections = []

        if self.model_type == "yolo":
            # Parse YOLO ultralytics result
            if hasattr(raw_result, "boxes") and raw_result.boxes is not None:
                boxes = raw_result.boxes

                for i in range(len(boxes)):
                    # Get box coordinates (xyxy format)
                    xyxy = boxes.xyxy[i].cpu().numpy()
                    conf = float(boxes.conf[i].cpu().numpy())
                    cls_id = int(boxes.cls[i].cpu().numpy())

                    # Skip low confidence detections
                    if conf < self.confidence_threshold:
                        continue

                    # Normalize coordinates
                    bbox = BoundingBox(
                        x_min=float(xyxy[0]) / image_width,
                        y_min=float(xyxy[1]) / image_height,
                        x_max=float(xyxy[2]) / image_width,
                        y_max=float(xyxy[3]) / image_height,
                    )

                    # Get class name
                    class_name = PPE_CLASS_MAP.get(cls_id, f"class_{cls_id}")

                    detections.append(
                        Detection(
                            class_name=class_name,
                            class_id=cls_id,
                            confidence=conf,
                            bbox=bbox,
                        )
                    )

        elif self.model_type == "onnx":
            # Parse ONNX output (assuming YOLO format: [batch, num_detections, 7])
            # Format: [x_center, y_center, width, height, obj_conf, class_conf, class_id]
            if isinstance(raw_result, np.ndarray):
                for det in raw_result:
                    if len(det) >= 6:
                        x_center, y_center, w, h = det[:4]
                        obj_conf = det[4]
                        class_conf = det[5]
                        cls_id = int(det[6]) if len(det) > 6 else 0

                        conf = float(obj_conf * class_conf)

                        if conf < self.confidence_threshold:
                            continue

                        # Convert center format to corner format and normalize
                        x_min = (x_center - w / 2) / self.input_size[0]
                        y_min = (y_center - h / 2) / self.input_size[1]
                        x_max = (x_center + w / 2) / self.input_size[0]
                        y_max = (y_center + h / 2) / self.input_size[1]

                        # Clamp to [0, 1]
                        x_min = max(0.0, min(1.0, x_min))
                        y_min = max(0.0, min(1.0, y_min))
                        x_max = max(0.0, min(1.0, x_max))
                        y_max = max(0.0, min(1.0, y_max))

                        bbox = BoundingBox(
                            x_min=x_min,
                            y_min=y_min,
                            x_max=x_max,
                            y_max=y_max,
                        )

                        class_name = PPE_CLASS_MAP.get(cls_id, f"class_{cls_id}")

                        detections.append(
                            Detection(
                                class_name=class_name,
                                class_id=cls_id,
                                confidence=conf,
                                bbox=bbox,
                            )
                        )

        return detections

    async def warmup(self, batch_size: int = 1) -> None:
        """Warmup the model with dummy inference.

        Args:
            batch_size: Number of dummy images for warmup.
        """
        if not self._is_loaded:
            raise RuntimeError("Model not loaded. Call load() first.")

        logger.info("Warming up PPE detection model", batch_size=batch_size)

        # Create dummy images
        dummy_images = [
            np.random.randint(0, 255, (640, 640, 3), dtype=np.uint8)
            for _ in range(batch_size)
        ]
        dummy_frame_ids = [f"warmup_{i}" for i in range(batch_size)]
        dummy_timestamps = [0] * batch_size

        # Run warmup inference
        start_time = time.perf_counter()

        for _ in range(3):  # Multiple warmup iterations
            await self.predict(dummy_images, dummy_frame_ids, dummy_timestamps)

        warmup_time = (time.perf_counter() - start_time) * 1000

        logger.info(
            "PPE detection model warmed up",
            warmup_time_ms=warmup_time,
            gpu_memory=self._check_gpu_memory(),
        )

    def get_class_names(self) -> list[str]:
        """Get list of PPE class names.

        Returns:
            List of class names.
        """
        return list(PPE_CLASS_MAP.values())

    def get_compliance_classes(self) -> list[str]:
        """Get PPE compliance classes (equipment present).

        Returns:
            List of compliance class names.
        """
        return [
            PPEClass.HELMET.value,
            PPEClass.VEST.value,
            PPEClass.GOGGLES.value,
        ]

    def get_violation_classes(self) -> list[str]:
        """Get PPE violation classes (equipment missing).

        Returns:
            List of violation class names.
        """
        return [
            PPEClass.NO_HELMET.value,
            PPEClass.NO_VEST.value,
            PPEClass.NO_GOGGLES.value,
        ]
