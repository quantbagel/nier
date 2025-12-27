"""
Nier Pipeline - Python wrapper for Kafka message pipeline

This package provides a Python interface for producing and consuming
messages in the Nier factory floor analytics platform.

Example:
    from nier_pipeline import NierProducer, NierConsumer, KafkaConfig

    config = KafkaConfig.from_env()
    producer = NierProducer(config)

    # Send a detection event
    producer.send_detection_event(event_data, event_id="evt-123")
"""

from .config import KafkaConfig, SecurityProtocol, SaslMechanism
from .producer import NierProducer, DeliveryResult
from .consumer import NierConsumer, IncomingMessage, MessageHandler
from .schemas import (
    DetectionEvent,
    FrameMetadata,
    Alert,
    PPEViolationType,
    ActivityType,
    AlertSeverity,
    AlertType,
)

__version__ = "0.1.0"
__all__ = [
    # Config
    "KafkaConfig",
    "SecurityProtocol",
    "SaslMechanism",
    # Producer
    "NierProducer",
    "DeliveryResult",
    # Consumer
    "NierConsumer",
    "IncomingMessage",
    "MessageHandler",
    # Schemas
    "DetectionEvent",
    "FrameMetadata",
    "Alert",
    "PPEViolationType",
    "ActivityType",
    "AlertSeverity",
    "AlertType",
]
