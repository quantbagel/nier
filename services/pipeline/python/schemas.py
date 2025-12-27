"""
Schema definitions for Nier pipeline messages.

These classes mirror the protobuf definitions and provide Python-native
data structures for working with pipeline messages.
"""

from dataclasses import dataclass, field
from datetime import datetime
from enum import IntEnum
from typing import Dict, List, Optional
import json


class PPEViolationType(IntEnum):
    """PPE violation types detected by the system."""
    UNSPECIFIED = 0
    NO_HELMET = 1
    NO_SAFETY_VEST = 2
    NO_SAFETY_GLASSES = 3
    NO_GLOVES = 4
    NO_SAFETY_BOOTS = 5
    NO_EAR_PROTECTION = 6
    NO_FACE_MASK = 7


class ActivityType(IntEnum):
    """Activity types that can be detected."""
    UNSPECIFIED = 0
    WALKING = 1
    STANDING = 2
    OPERATING_MACHINERY = 3
    LIFTING = 4
    CLIMBING = 5
    RUNNING = 6
    FALLING = 7
    REACHING = 8
    CARRYING = 9


class AlertSeverity(IntEnum):
    """Severity level of the alert."""
    UNSPECIFIED = 0
    INFO = 1
    WARNING = 2
    CRITICAL = 3
    EMERGENCY = 4


class AlertType(IntEnum):
    """Type of safety alert."""
    UNSPECIFIED = 0
    PPE_VIOLATION = 1
    PPE_MISSING = 2
    UNSAFE_ACTIVITY = 3
    RESTRICTED_ZONE_ENTRY = 4
    FALL_DETECTED = 5
    UNUSUAL_INACTIVITY = 6
    HAZARD_DETECTED = 7
    EQUIPMENT_MALFUNCTION = 8
    DEVICE_LOW_BATTERY = 9
    DEVICE_OFFLINE = 10
    DEVICE_ERROR = 11
    PATTERN_DETECTED = 12
    THRESHOLD_EXCEEDED = 13


class AlertStatus(IntEnum):
    """Current status of the alert."""
    UNSPECIFIED = 0
    NEW = 1
    ACKNOWLEDGED = 2
    IN_PROGRESS = 3
    RESOLVED = 4
    DISMISSED = 5
    ESCALATED = 6


@dataclass
class BoundingBox:
    """Bounding box for detected objects (normalized 0.0-1.0)."""
    x_min: float
    y_min: float
    x_max: float
    y_max: float

    def to_dict(self) -> Dict:
        return {
            "x_min": self.x_min,
            "y_min": self.y_min,
            "x_max": self.x_max,
            "y_max": self.y_max,
        }

    @classmethod
    def from_dict(cls, data: Dict) -> "BoundingBox":
        return cls(
            x_min=data["x_min"],
            y_min=data["y_min"],
            x_max=data["x_max"],
            y_max=data["y_max"],
        )


@dataclass
class ConfidenceScore:
    """Confidence score with optional breakdown."""
    overall: float
    breakdown: Dict[str, float] = field(default_factory=dict)

    def to_dict(self) -> Dict:
        return {"overall": self.overall, "breakdown": self.breakdown}

    @classmethod
    def from_dict(cls, data: Dict) -> "ConfidenceScore":
        return cls(
            overall=data["overall"],
            breakdown=data.get("breakdown", {}),
        )


@dataclass
class PPEViolation:
    """PPE violation detection event."""
    violation_type: PPEViolationType
    bounding_box: BoundingBox
    confidence: ConfidenceScore
    worker_id: Optional[str] = None

    def to_dict(self) -> Dict:
        return {
            "violation_type": self.violation_type.value,
            "bounding_box": self.bounding_box.to_dict(),
            "confidence": self.confidence.to_dict(),
            "worker_id": self.worker_id,
        }

    @classmethod
    def from_dict(cls, data: Dict) -> "PPEViolation":
        return cls(
            violation_type=PPEViolationType(data["violation_type"]),
            bounding_box=BoundingBox.from_dict(data["bounding_box"]),
            confidence=ConfidenceScore.from_dict(data["confidence"]),
            worker_id=data.get("worker_id"),
        )


@dataclass
class ActivityDetection:
    """Activity detection event."""
    activity_type: ActivityType
    bounding_box: BoundingBox
    confidence: ConfidenceScore
    duration_ms: Optional[int] = None
    worker_id: Optional[str] = None

    def to_dict(self) -> Dict:
        return {
            "activity_type": self.activity_type.value,
            "bounding_box": self.bounding_box.to_dict(),
            "confidence": self.confidence.to_dict(),
            "duration_ms": self.duration_ms,
            "worker_id": self.worker_id,
        }

    @classmethod
    def from_dict(cls, data: Dict) -> "ActivityDetection":
        return cls(
            activity_type=ActivityType(data["activity_type"]),
            bounding_box=BoundingBox.from_dict(data["bounding_box"]),
            confidence=ConfidenceScore.from_dict(data["confidence"]),
            duration_ms=data.get("duration_ms"),
            worker_id=data.get("worker_id"),
        )


@dataclass
class Zone:
    """Zone information where detection occurred."""
    zone_id: str
    zone_name: str
    zone_type: str
    required_ppe: List[PPEViolationType] = field(default_factory=list)

    def to_dict(self) -> Dict:
        return {
            "zone_id": self.zone_id,
            "zone_name": self.zone_name,
            "zone_type": self.zone_type,
            "required_ppe": [p.value for p in self.required_ppe],
        }

    @classmethod
    def from_dict(cls, data: Dict) -> "Zone":
        return cls(
            zone_id=data["zone_id"],
            zone_name=data["zone_name"],
            zone_type=data["zone_type"],
            required_ppe=[PPEViolationType(p) for p in data.get("required_ppe", [])],
        )


@dataclass
class DetectionEvent:
    """Main detection event message."""
    event_id: str
    frame_id: str
    device_id: str
    timestamp: datetime
    model_id: str
    model_version: str
    processing_latency_ms: int
    ppe_violations: List[PPEViolation] = field(default_factory=list)
    activity_detections: List[ActivityDetection] = field(default_factory=list)
    zone: Optional[Zone] = None
    metadata: Dict[str, str] = field(default_factory=dict)

    def to_dict(self) -> Dict:
        return {
            "event_id": self.event_id,
            "frame_id": self.frame_id,
            "device_id": self.device_id,
            "timestamp": self.timestamp.isoformat(),
            "model_id": self.model_id,
            "model_version": self.model_version,
            "processing_latency_ms": self.processing_latency_ms,
            "ppe_violations": [v.to_dict() for v in self.ppe_violations],
            "activity_detections": [a.to_dict() for a in self.activity_detections],
            "zone": self.zone.to_dict() if self.zone else None,
            "metadata": self.metadata,
        }

    def to_json(self) -> str:
        return json.dumps(self.to_dict())

    def to_bytes(self) -> bytes:
        return self.to_json().encode("utf-8")

    @classmethod
    def from_dict(cls, data: Dict) -> "DetectionEvent":
        return cls(
            event_id=data["event_id"],
            frame_id=data["frame_id"],
            device_id=data["device_id"],
            timestamp=datetime.fromisoformat(data["timestamp"]),
            model_id=data["model_id"],
            model_version=data["model_version"],
            processing_latency_ms=data["processing_latency_ms"],
            ppe_violations=[
                PPEViolation.from_dict(v) for v in data.get("ppe_violations", [])
            ],
            activity_detections=[
                ActivityDetection.from_dict(a)
                for a in data.get("activity_detections", [])
            ],
            zone=Zone.from_dict(data["zone"]) if data.get("zone") else None,
            metadata=data.get("metadata", {}),
        )

    @classmethod
    def from_json(cls, json_str: str) -> "DetectionEvent":
        return cls.from_dict(json.loads(json_str))

    @classmethod
    def from_bytes(cls, data: bytes) -> "DetectionEvent":
        return cls.from_json(data.decode("utf-8"))


@dataclass
class GeoLocation:
    """GPS coordinates."""
    latitude: float
    longitude: float
    accuracy: float
    altitude: Optional[float] = None

    def to_dict(self) -> Dict:
        return {
            "latitude": self.latitude,
            "longitude": self.longitude,
            "accuracy": self.accuracy,
            "altitude": self.altitude,
        }

    @classmethod
    def from_dict(cls, data: Dict) -> "GeoLocation":
        return cls(
            latitude=data["latitude"],
            longitude=data["longitude"],
            accuracy=data["accuracy"],
            altitude=data.get("altitude"),
        )


@dataclass
class IMUData:
    """IMU sensor data."""
    accel_x: float
    accel_y: float
    accel_z: float
    gyro_x: float
    gyro_y: float
    gyro_z: float
    mag_x: Optional[float] = None
    mag_y: Optional[float] = None
    mag_z: Optional[float] = None

    def to_dict(self) -> Dict:
        return {
            "accel_x": self.accel_x,
            "accel_y": self.accel_y,
            "accel_z": self.accel_z,
            "gyro_x": self.gyro_x,
            "gyro_y": self.gyro_y,
            "gyro_z": self.gyro_z,
            "mag_x": self.mag_x,
            "mag_y": self.mag_y,
            "mag_z": self.mag_z,
        }

    @classmethod
    def from_dict(cls, data: Dict) -> "IMUData":
        return cls(
            accel_x=data["accel_x"],
            accel_y=data["accel_y"],
            accel_z=data["accel_z"],
            gyro_x=data["gyro_x"],
            gyro_y=data["gyro_y"],
            gyro_z=data["gyro_z"],
            mag_x=data.get("mag_x"),
            mag_y=data.get("mag_y"),
            mag_z=data.get("mag_z"),
        )


@dataclass
class DeviceHealth:
    """Device health status."""
    battery_level: int
    temperature_celsius: float
    storage_remaining_bytes: int
    wifi_signal_dbm: int
    is_charging: bool

    def to_dict(self) -> Dict:
        return {
            "battery_level": self.battery_level,
            "temperature_celsius": self.temperature_celsius,
            "storage_remaining_bytes": self.storage_remaining_bytes,
            "wifi_signal_dbm": self.wifi_signal_dbm,
            "is_charging": self.is_charging,
        }

    @classmethod
    def from_dict(cls, data: Dict) -> "DeviceHealth":
        return cls(
            battery_level=data["battery_level"],
            temperature_celsius=data["temperature_celsius"],
            storage_remaining_bytes=data["storage_remaining_bytes"],
            wifi_signal_dbm=data["wifi_signal_dbm"],
            is_charging=data["is_charging"],
        )


@dataclass
class FrameMetadata:
    """Frame metadata message."""
    frame_id: str
    device_id: str
    capture_timestamp: datetime
    upload_timestamp: datetime
    width: int
    height: int
    frame_number: int
    frame_data_uri: str
    frame_size_bytes: int
    session_id: str
    device_type: str = "SMART_GLASSES_V1"
    device_firmware_version: str = "1.0.0"
    resolution: str = "1080P"
    format: str = "JPEG"
    location: Optional[GeoLocation] = None
    imu_data: Optional[IMUData] = None
    device_health: Optional[DeviceHealth] = None
    brightness: Optional[float] = None
    blur_score: Optional[float] = None
    is_occluded: Optional[bool] = None
    metadata: Dict[str, str] = field(default_factory=dict)

    def to_dict(self) -> Dict:
        return {
            "frame_id": self.frame_id,
            "device_id": self.device_id,
            "device_type": self.device_type,
            "device_firmware_version": self.device_firmware_version,
            "capture_timestamp": self.capture_timestamp.isoformat(),
            "upload_timestamp": self.upload_timestamp.isoformat(),
            "resolution": self.resolution,
            "format": self.format,
            "width": self.width,
            "height": self.height,
            "frame_number": self.frame_number,
            "frame_data_uri": self.frame_data_uri,
            "frame_size_bytes": self.frame_size_bytes,
            "session_id": self.session_id,
            "location": self.location.to_dict() if self.location else None,
            "imu_data": self.imu_data.to_dict() if self.imu_data else None,
            "device_health": self.device_health.to_dict() if self.device_health else None,
            "brightness": self.brightness,
            "blur_score": self.blur_score,
            "is_occluded": self.is_occluded,
            "metadata": self.metadata,
        }

    def to_json(self) -> str:
        return json.dumps(self.to_dict())

    def to_bytes(self) -> bytes:
        return self.to_json().encode("utf-8")

    @classmethod
    def from_dict(cls, data: Dict) -> "FrameMetadata":
        return cls(
            frame_id=data["frame_id"],
            device_id=data["device_id"],
            device_type=data.get("device_type", "SMART_GLASSES_V1"),
            device_firmware_version=data.get("device_firmware_version", "1.0.0"),
            capture_timestamp=datetime.fromisoformat(data["capture_timestamp"]),
            upload_timestamp=datetime.fromisoformat(data["upload_timestamp"]),
            resolution=data.get("resolution", "1080P"),
            format=data.get("format", "JPEG"),
            width=data["width"],
            height=data["height"],
            frame_number=data["frame_number"],
            frame_data_uri=data["frame_data_uri"],
            frame_size_bytes=data["frame_size_bytes"],
            session_id=data["session_id"],
            location=GeoLocation.from_dict(data["location"]) if data.get("location") else None,
            imu_data=IMUData.from_dict(data["imu_data"]) if data.get("imu_data") else None,
            device_health=DeviceHealth.from_dict(data["device_health"]) if data.get("device_health") else None,
            brightness=data.get("brightness"),
            blur_score=data.get("blur_score"),
            is_occluded=data.get("is_occluded"),
            metadata=data.get("metadata", {}),
        )

    @classmethod
    def from_json(cls, json_str: str) -> "FrameMetadata":
        return cls.from_dict(json.loads(json_str))

    @classmethod
    def from_bytes(cls, data: bytes) -> "FrameMetadata":
        return cls.from_json(data.decode("utf-8"))


@dataclass
class Alert:
    """Main alert message."""
    alert_id: str
    alert_type: AlertType
    severity: AlertSeverity
    status: AlertStatus
    title: str
    description: str
    created_at: datetime
    updated_at: datetime
    device_id: str
    rule_id: str
    priority_score: int = 0
    worker_id: Optional[str] = None
    expires_at: Optional[datetime] = None
    source_detection_ids: List[str] = field(default_factory=list)
    tags: List[str] = field(default_factory=list)
    metadata: Dict[str, str] = field(default_factory=dict)

    def to_dict(self) -> Dict:
        return {
            "alert_id": self.alert_id,
            "alert_type": self.alert_type.value,
            "severity": self.severity.value,
            "status": self.status.value,
            "title": self.title,
            "description": self.description,
            "created_at": self.created_at.isoformat(),
            "updated_at": self.updated_at.isoformat(),
            "expires_at": self.expires_at.isoformat() if self.expires_at else None,
            "device_id": self.device_id,
            "worker_id": self.worker_id,
            "rule_id": self.rule_id,
            "priority_score": self.priority_score,
            "source_detection_ids": self.source_detection_ids,
            "tags": self.tags,
            "metadata": self.metadata,
        }

    def to_json(self) -> str:
        return json.dumps(self.to_dict())

    def to_bytes(self) -> bytes:
        return self.to_json().encode("utf-8")

    @classmethod
    def from_dict(cls, data: Dict) -> "Alert":
        return cls(
            alert_id=data["alert_id"],
            alert_type=AlertType(data["alert_type"]),
            severity=AlertSeverity(data["severity"]),
            status=AlertStatus(data["status"]),
            title=data["title"],
            description=data["description"],
            created_at=datetime.fromisoformat(data["created_at"]),
            updated_at=datetime.fromisoformat(data["updated_at"]),
            expires_at=datetime.fromisoformat(data["expires_at"]) if data.get("expires_at") else None,
            device_id=data["device_id"],
            worker_id=data.get("worker_id"),
            rule_id=data["rule_id"],
            priority_score=data.get("priority_score", 0),
            source_detection_ids=data.get("source_detection_ids", []),
            tags=data.get("tags", []),
            metadata=data.get("metadata", {}),
        )

    @classmethod
    def from_json(cls, json_str: str) -> "Alert":
        return cls.from_dict(json.loads(json_str))

    @classmethod
    def from_bytes(cls, data: bytes) -> "Alert":
        return cls.from_json(data.decode("utf-8"))
