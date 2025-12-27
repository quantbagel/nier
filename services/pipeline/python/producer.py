"""
Kafka producer wrapper for the Nier pipeline.

This module provides a high-level, type-safe interface for producing messages
to Kafka topics with support for JSON serialization and reliable delivery.
"""

import logging
import uuid
from dataclasses import dataclass
from datetime import datetime
from typing import Any, Dict, List, Optional, Union

from confluent_kafka import Producer
from confluent_kafka.admin import AdminClient

from .config import KafkaConfig
from .schemas import Alert, DetectionEvent, FrameMetadata

logger = logging.getLogger(__name__)


@dataclass
class DeliveryResult:
    """Result of a successful message delivery."""
    topic: str
    partition: int
    offset: int
    key: Optional[str] = None
    timestamp: Optional[datetime] = None


class ProducerError(Exception):
    """Exception raised for producer errors."""
    pass


class NierProducer:
    """High-level Kafka producer wrapper for the Nier pipeline."""

    def __init__(self, config: KafkaConfig):
        """
        Create a new producer with the given configuration.

        Args:
            config: Kafka configuration
        """
        self.config = config
        self._producer = Producer(config.to_producer_config())
        self._pending_callbacks: Dict[str, Optional[DeliveryResult]] = {}
        self._pending_errors: Dict[str, Optional[Exception]] = {}

        logger.info(
            "Created Kafka producer for %s", config.bootstrap_servers
        )

    def _delivery_callback(
        self, err: Optional[Exception], msg: Any, callback_id: str
    ) -> None:
        """Internal delivery callback."""
        if err is not None:
            self._pending_errors[callback_id] = err
            self._pending_callbacks[callback_id] = None
            logger.error("Message delivery failed: %s", err)
        else:
            self._pending_callbacks[callback_id] = DeliveryResult(
                topic=msg.topic(),
                partition=msg.partition(),
                offset=msg.offset(),
                key=msg.key().decode("utf-8") if msg.key() else None,
            )
            self._pending_errors[callback_id] = None
            logger.debug(
                "Message delivered to %s [%d] @ %d",
                msg.topic(),
                msg.partition(),
                msg.offset(),
            )

    def send(
        self,
        topic: str,
        value: bytes,
        key: Optional[str] = None,
        headers: Optional[Dict[str, str]] = None,
    ) -> DeliveryResult:
        """
        Send a message and wait for delivery confirmation.

        Args:
            topic: Topic to send the message to
            value: Message payload as bytes
            key: Optional message key for partitioning
            headers: Optional message headers

        Returns:
            DeliveryResult with topic, partition, and offset

        Raises:
            ProducerError: If message delivery fails
        """
        callback_id = str(uuid.uuid4())

        # Convert headers to list of tuples
        header_list = None
        if headers:
            header_list = [(k, v.encode("utf-8")) for k, v in headers.items()]

        # Produce the message
        self._producer.produce(
            topic=topic,
            value=value,
            key=key.encode("utf-8") if key else None,
            headers=header_list,
            callback=lambda err, msg: self._delivery_callback(err, msg, callback_id),
        )

        # Wait for delivery
        self._producer.flush()

        # Check result
        if callback_id in self._pending_errors and self._pending_errors[callback_id]:
            error = self._pending_errors.pop(callback_id)
            self._pending_callbacks.pop(callback_id, None)
            raise ProducerError(f"Failed to send message: {error}")

        result = self._pending_callbacks.pop(callback_id, None)
        self._pending_errors.pop(callback_id, None)

        if result is None:
            raise ProducerError("No delivery result received")

        return result

    def send_async(
        self,
        topic: str,
        value: bytes,
        key: Optional[str] = None,
        headers: Optional[Dict[str, str]] = None,
    ) -> None:
        """
        Send a message without waiting for delivery confirmation.

        Args:
            topic: Topic to send the message to
            value: Message payload as bytes
            key: Optional message key for partitioning
            headers: Optional message headers
        """
        header_list = None
        if headers:
            header_list = [(k, v.encode("utf-8")) for k, v in headers.items()]

        self._producer.produce(
            topic=topic,
            value=value,
            key=key.encode("utf-8") if key else None,
            headers=header_list,
        )

    def send_detection_event(
        self,
        event: DetectionEvent,
        event_id: Optional[str] = None,
    ) -> DeliveryResult:
        """
        Send a detection event to the detections topic.

        Args:
            event: Detection event to send
            event_id: Optional event ID (uses event.event_id if not provided)

        Returns:
            DeliveryResult with delivery information
        """
        key = event_id or event.event_id
        headers = {
            "message-type": "detection_event",
            "correlation-id": key,
        }

        return self.send(
            topic=self.config.topics.detections,
            value=event.to_bytes(),
            key=key,
            headers=headers,
        )

    def send_frame_metadata(
        self,
        metadata: FrameMetadata,
        frame_id: Optional[str] = None,
    ) -> DeliveryResult:
        """
        Send frame metadata to the frames topic.

        Args:
            metadata: Frame metadata to send
            frame_id: Optional frame ID (uses metadata.frame_id if not provided)

        Returns:
            DeliveryResult with delivery information
        """
        key = frame_id or metadata.frame_id
        headers = {
            "message-type": "frame_metadata",
        }

        return self.send(
            topic=self.config.topics.frames,
            value=metadata.to_bytes(),
            key=key,
            headers=headers,
        )

    def send_alert(
        self,
        alert: Alert,
        alert_id: Optional[str] = None,
    ) -> DeliveryResult:
        """
        Send an alert to the alerts topic.

        Args:
            alert: Alert to send
            alert_id: Optional alert ID (uses alert.alert_id if not provided)

        Returns:
            DeliveryResult with delivery information
        """
        key = alert_id or alert.alert_id
        headers = {
            "message-type": "alert",
        }

        return self.send(
            topic=self.config.topics.alerts,
            value=alert.to_bytes(),
            key=key,
            headers=headers,
        )

    def send_to_dlq(
        self,
        original_topic: str,
        original_message: bytes,
        error: str,
    ) -> DeliveryResult:
        """
        Send a message to the dead letter queue.

        Args:
            original_topic: Original topic the message was from
            original_message: Original message payload
            error: Error description

        Returns:
            DeliveryResult with delivery information
        """
        import base64
        import json

        dlq_message = {
            "original_topic": original_topic,
            "original_message_base64": base64.b64encode(original_message).decode(
                "utf-8"
            ),
            "error": error,
            "timestamp": datetime.utcnow().isoformat(),
        }

        headers = {
            "message-type": "dead_letter",
            "original-topic": original_topic,
            "error-reason": error[:256],  # Truncate long errors
        }

        return self.send(
            topic=self.config.topics.dead_letter_queue,
            value=json.dumps(dlq_message).encode("utf-8"),
            key=str(uuid.uuid4()),
            headers=headers,
        )

    def flush(self, timeout: float = 30.0) -> int:
        """
        Flush all pending messages.

        Args:
            timeout: Maximum time to wait in seconds

        Returns:
            Number of messages still in queue
        """
        return self._producer.flush(timeout)

    def poll(self, timeout: float = 0) -> int:
        """
        Poll for events and trigger callbacks.

        Args:
            timeout: Maximum time to wait in seconds

        Returns:
            Number of events processed
        """
        return self._producer.poll(timeout)

    def __enter__(self) -> "NierProducer":
        return self

    def __exit__(self, exc_type, exc_val, exc_tb) -> None:
        self.flush()

    def close(self) -> None:
        """Close the producer and flush pending messages."""
        self.flush()
        logger.info("Producer closed")


class ProducerBuilder:
    """Builder for creating producers with custom settings."""

    def __init__(self, bootstrap_servers: str):
        """
        Create a new producer builder.

        Args:
            bootstrap_servers: Comma-separated list of broker addresses
        """
        self.config = KafkaConfig(bootstrap_servers=bootstrap_servers)

    def client_id(self, client_id: str) -> "ProducerBuilder":
        """Set the client ID."""
        self.config.client_id = client_id
        return self

    def idempotent(self, enable: bool = True) -> "ProducerBuilder":
        """Enable idempotent producer."""
        self.config.reliability.enable_idempotence = enable
        return self

    def compression(self, compression_type: str) -> "ProducerBuilder":
        """Set compression type."""
        self.config.producer.compression_type = compression_type
        return self

    def batch_size(self, size: int) -> "ProducerBuilder":
        """Set batch size."""
        self.config.producer.batch_size = size
        return self

    def linger_ms(self, ms: int) -> "ProducerBuilder":
        """Set linger time."""
        self.config.producer.linger_ms = ms
        return self

    def build(self) -> NierProducer:
        """Build the producer."""
        return NierProducer(self.config)
