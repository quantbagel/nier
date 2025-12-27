"""Base model interface for inference service."""

from __future__ import annotations

from abc import ABC, abstractmethod
from dataclasses import dataclass, field
from enum import Enum
from typing import Any

import numpy as np
import torch


class PPEClass(Enum):
    """PPE detection classes for factory floor safety."""

    HELMET = "helmet"
    VEST = "vest"
    GOGGLES = "goggles"
    NO_HELMET = "no_helmet"
    NO_VEST = "no_vest"
    NO_GOGGLES = "no_goggles"
    PERSON = "person"


@dataclass
class BoundingBox:
    """Bounding box coordinates in normalized format [0, 1]."""

    x_min: float
    y_min: float
    x_max: float
    y_max: float

    def __post_init__(self) -> None:
        """Validate bounding box coordinates."""
        if not (0 <= self.x_min <= self.x_max <= 1):
            raise ValueError(f"Invalid x coordinates: {self.x_min}, {self.x_max}")
        if not (0 <= self.y_min <= self.y_max <= 1):
            raise ValueError(f"Invalid y coordinates: {self.y_min}, {self.y_max}")

    @property
    def width(self) -> float:
        """Get bounding box width."""
        return self.x_max - self.x_min

    @property
    def height(self) -> float:
        """Get bounding box height."""
        return self.y_max - self.y_min

    @property
    def area(self) -> float:
        """Get bounding box area."""
        return self.width * self.height

    @property
    def center(self) -> tuple[float, float]:
        """Get bounding box center coordinates."""
        return (
            (self.x_min + self.x_max) / 2,
            (self.y_min + self.y_max) / 2,
        )

    def to_absolute(self, width: int, height: int) -> tuple[int, int, int, int]:
        """Convert to absolute pixel coordinates.

        Args:
            width: Image width in pixels.
            height: Image height in pixels.

        Returns:
            Tuple of (x_min, y_min, x_max, y_max) in pixels.
        """
        return (
            int(self.x_min * width),
            int(self.y_min * height),
            int(self.x_max * width),
            int(self.y_max * height),
        )

    def to_dict(self) -> dict[str, float]:
        """Convert to dictionary representation."""
        return {
            "x_min": self.x_min,
            "y_min": self.y_min,
            "x_max": self.x_max,
            "y_max": self.y_max,
        }


@dataclass
class Detection:
    """Single detection result from a model."""

    class_name: str
    class_id: int
    confidence: float
    bbox: BoundingBox
    metadata: dict[str, Any] = field(default_factory=dict)

    def to_dict(self) -> dict[str, Any]:
        """Convert detection to dictionary representation."""
        return {
            "class_name": self.class_name,
            "class_id": self.class_id,
            "confidence": self.confidence,
            "bbox": self.bbox.to_dict(),
            "metadata": self.metadata,
        }


@dataclass
class DetectionResult:
    """Result of inference on a single image."""

    frame_id: str
    timestamp_ms: int
    detections: list[Detection]
    inference_time_ms: float
    image_width: int
    image_height: int
    metadata: dict[str, Any] = field(default_factory=dict)

    @property
    def detection_count(self) -> int:
        """Get total number of detections."""
        return len(self.detections)

    def filter_by_class(self, class_names: list[str]) -> list[Detection]:
        """Filter detections by class names.

        Args:
            class_names: List of class names to include.

        Returns:
            Filtered list of detections.
        """
        return [d for d in self.detections if d.class_name in class_names]

    def filter_by_confidence(self, min_confidence: float) -> list[Detection]:
        """Filter detections by minimum confidence.

        Args:
            min_confidence: Minimum confidence threshold.

        Returns:
            Filtered list of detections.
        """
        return [d for d in self.detections if d.confidence >= min_confidence]

    def to_dict(self) -> dict[str, Any]:
        """Convert result to dictionary representation."""
        return {
            "frame_id": self.frame_id,
            "timestamp_ms": self.timestamp_ms,
            "detections": [d.to_dict() for d in self.detections],
            "inference_time_ms": self.inference_time_ms,
            "image_width": self.image_width,
            "image_height": self.image_height,
            "detection_count": self.detection_count,
            "metadata": self.metadata,
        }


class BaseDetector(ABC):
    """Abstract base class for detection models."""

    def __init__(
        self,
        model_path: str,
        device: str = "cuda:0",
        confidence_threshold: float = 0.5,
        iou_threshold: float = 0.45,
        half_precision: bool = True,
    ) -> None:
        """Initialize the detector.

        Args:
            model_path: Path to the model weights.
            device: Device to run inference on.
            confidence_threshold: Minimum confidence for detections.
            iou_threshold: IoU threshold for NMS.
            half_precision: Whether to use FP16 inference.
        """
        self.model_path = model_path
        self.device = device
        self.confidence_threshold = confidence_threshold
        self.iou_threshold = iou_threshold
        self.half_precision = half_precision
        self._model: Any = None
        self._is_loaded = False

    @property
    def is_loaded(self) -> bool:
        """Check if model is loaded."""
        return self._is_loaded

    @abstractmethod
    async def load(self) -> None:
        """Load the model into memory.

        This method should be called before inference.
        """
        pass

    @abstractmethod
    async def unload(self) -> None:
        """Unload the model from memory.

        This method frees GPU memory.
        """
        pass

    @abstractmethod
    async def predict(
        self,
        images: list[np.ndarray],
        frame_ids: list[str],
        timestamps: list[int],
    ) -> list[DetectionResult]:
        """Run inference on a batch of images.

        Args:
            images: List of images as numpy arrays (BGR format).
            frame_ids: List of unique frame identifiers.
            timestamps: List of timestamps in milliseconds.

        Returns:
            List of detection results, one per image.
        """
        pass

    @abstractmethod
    async def warmup(self, batch_size: int = 1) -> None:
        """Warmup the model with dummy inference.

        Args:
            batch_size: Batch size for warmup inference.
        """
        pass

    def _check_gpu_memory(self) -> dict[str, float]:
        """Check GPU memory usage.

        Returns:
            Dictionary with memory usage statistics.
        """
        if not torch.cuda.is_available():
            return {"available": False}

        device_idx = (
            int(self.device.split(":")[-1])
            if ":" in self.device
            else 0
        )

        total = torch.cuda.get_device_properties(device_idx).total_memory
        allocated = torch.cuda.memory_allocated(device_idx)
        cached = torch.cuda.memory_reserved(device_idx)
        free = total - allocated

        return {
            "available": True,
            "total_mb": total / 1024 / 1024,
            "allocated_mb": allocated / 1024 / 1024,
            "cached_mb": cached / 1024 / 1024,
            "free_mb": free / 1024 / 1024,
            "utilization_percent": (allocated / total) * 100,
        }

    def _clear_gpu_cache(self) -> None:
        """Clear GPU memory cache."""
        if torch.cuda.is_available():
            torch.cuda.empty_cache()


@dataclass
class BatchRequest:
    """Request for batched inference."""

    images: list[np.ndarray]
    frame_ids: list[str]
    timestamps: list[int]
    request_ids: list[str]

    def __len__(self) -> int:
        """Get batch size."""
        return len(self.images)


@dataclass
class BatchResponse:
    """Response from batched inference."""

    results: list[DetectionResult]
    request_ids: list[str]
    total_inference_time_ms: float
