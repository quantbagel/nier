"""Kafka producer for publishing detection results."""

from __future__ import annotations

import asyncio
import json
import time
from typing import Any

import structlog
from kafka import KafkaProducer
from kafka.errors import KafkaError, NoBrokersAvailable

from src.config import KafkaConfig
from src.models.base import DetectionResult

logger = structlog.get_logger(__name__)


class DetectionKafkaProducer:
    """Async-compatible Kafka producer for detection results.

    Publishes PPE detection results to Kafka topics for downstream
    processing by the analytics pipeline.
    """

    def __init__(self, config: KafkaConfig) -> None:
        """Initialize the Kafka producer.

        Args:
            config: Kafka configuration settings.
        """
        self.config = config
        self._producer: KafkaProducer | None = None
        self._is_connected = False
        self._lock = asyncio.Lock()
        self._pending_messages: int = 0
        self._max_pending: int = 10000

    @property
    def is_connected(self) -> bool:
        """Check if producer is connected to Kafka."""
        return self._is_connected

    async def connect(self) -> None:
        """Connect to Kafka brokers.

        Raises:
            ConnectionError: If unable to connect to Kafka.
        """
        async with self._lock:
            if self._is_connected:
                logger.warning("Kafka producer already connected")
                return

            logger.info(
                "Connecting to Kafka",
                bootstrap_servers=self.config.bootstrap_servers,
                topic=self.config.topic,
            )

            try:
                # Run producer creation in thread pool (blocking operation)
                loop = asyncio.get_event_loop()
                self._producer = await loop.run_in_executor(
                    None,
                    self._create_producer,
                )
                self._is_connected = True

                logger.info("Kafka producer connected successfully")

            except NoBrokersAvailable as e:
                logger.error(
                    "No Kafka brokers available",
                    bootstrap_servers=self.config.bootstrap_servers,
                    error=str(e),
                )
                raise ConnectionError(f"No Kafka brokers available: {e}") from e

            except KafkaError as e:
                logger.error("Failed to connect to Kafka", error=str(e))
                raise ConnectionError(f"Kafka connection failed: {e}") from e

    def _create_producer(self) -> KafkaProducer:
        """Create and configure Kafka producer (blocking).

        Returns:
            Configured KafkaProducer instance.
        """
        return KafkaProducer(
            bootstrap_servers=self.config.bootstrap_servers.split(","),
            client_id=self.config.client_id,
            acks=self.config.acks,
            compression_type=self.config.compression_type,
            batch_size=self.config.batch_size,
            linger_ms=self.config.linger_ms,
            max_request_size=self.config.max_request_size,
            value_serializer=lambda v: json.dumps(v).encode("utf-8"),
            key_serializer=lambda k: k.encode("utf-8") if k else None,
            retries=3,
            retry_backoff_ms=100,
        )

    async def disconnect(self) -> None:
        """Disconnect from Kafka and flush pending messages."""
        async with self._lock:
            if not self._is_connected or self._producer is None:
                logger.warning("Kafka producer not connected")
                return

            logger.info("Disconnecting Kafka producer", pending_messages=self._pending_messages)

            try:
                # Flush pending messages
                loop = asyncio.get_event_loop()
                await loop.run_in_executor(
                    None,
                    lambda: self._producer.flush(timeout=10),
                )

                # Close producer
                await loop.run_in_executor(
                    None,
                    self._producer.close,
                )

            except Exception as e:
                logger.error("Error during Kafka disconnect", error=str(e))

            finally:
                self._producer = None
                self._is_connected = False
                self._pending_messages = 0

            logger.info("Kafka producer disconnected")

    async def publish_detection(
        self,
        result: DetectionResult,
        worker_id: str | None = None,
        camera_id: str | None = None,
    ) -> bool:
        """Publish a single detection result to Kafka.

        Args:
            result: Detection result to publish.
            worker_id: Optional worker identifier for message key.
            camera_id: Optional camera identifier.

        Returns:
            True if message was sent successfully.
        """
        if not self._is_connected or self._producer is None:
            raise RuntimeError("Kafka producer not connected. Call connect() first.")

        if self._pending_messages >= self._max_pending:
            logger.warning(
                "Too many pending messages, dropping",
                pending=self._pending_messages,
                max=self._max_pending,
            )
            return False

        try:
            # Build message payload
            message = self._build_message(result, worker_id, camera_id)

            # Use frame_id as key for partition assignment
            key = result.frame_id

            # Send asynchronously
            self._pending_messages += 1

            loop = asyncio.get_event_loop()
            future = await loop.run_in_executor(
                None,
                lambda: self._producer.send(
                    self.config.topic,
                    key=key,
                    value=message,
                ),
            )

            # Add callback for tracking
            future.add_callback(self._on_send_success)
            future.add_errback(self._on_send_error)

            return True

        except KafkaError as e:
            logger.error(
                "Failed to publish detection",
                error=str(e),
                frame_id=result.frame_id,
            )
            return False

    async def publish_batch(
        self,
        results: list[DetectionResult],
        worker_id: str | None = None,
        camera_id: str | None = None,
    ) -> int:
        """Publish a batch of detection results to Kafka.

        Args:
            results: List of detection results to publish.
            worker_id: Optional worker identifier.
            camera_id: Optional camera identifier.

        Returns:
            Number of messages successfully queued.
        """
        if not self._is_connected or self._producer is None:
            raise RuntimeError("Kafka producer not connected. Call connect() first.")

        successful = 0

        for result in results:
            if await self.publish_detection(result, worker_id, camera_id):
                successful += 1

        logger.debug(
            "Batch published to Kafka",
            total=len(results),
            successful=successful,
        )

        return successful

    def _build_message(
        self,
        result: DetectionResult,
        worker_id: str | None,
        camera_id: str | None,
    ) -> dict[str, Any]:
        """Build Kafka message from detection result.

        Args:
            result: Detection result.
            worker_id: Worker identifier.
            camera_id: Camera identifier.

        Returns:
            Message dictionary.
        """
        message = result.to_dict()

        # Add metadata
        message["publish_timestamp_ms"] = int(time.time() * 1000)
        message["service"] = "nier-inference"

        if worker_id:
            message["worker_id"] = worker_id

        if camera_id:
            message["camera_id"] = camera_id

        # Add compliance summary
        violations = []
        compliant_items = []

        for detection in result.detections:
            if detection.class_name.startswith("no_"):
                violations.append(detection.class_name)
            elif detection.class_name in ("helmet", "vest", "goggles"):
                compliant_items.append(detection.class_name)

        message["compliance_summary"] = {
            "violations": violations,
            "compliant_items": compliant_items,
            "has_violations": len(violations) > 0,
            "violation_count": len(violations),
        }

        return message

    def _on_send_success(self, record_metadata: Any) -> None:
        """Callback for successful message send."""
        self._pending_messages = max(0, self._pending_messages - 1)
        logger.debug(
            "Message sent successfully",
            topic=record_metadata.topic,
            partition=record_metadata.partition,
            offset=record_metadata.offset,
        )

    def _on_send_error(self, exc: Exception) -> None:
        """Callback for failed message send."""
        self._pending_messages = max(0, self._pending_messages - 1)
        logger.error("Message send failed", error=str(exc))

    async def flush(self, timeout_seconds: float = 10.0) -> None:
        """Flush all pending messages.

        Args:
            timeout_seconds: Maximum time to wait for flush.
        """
        if not self._is_connected or self._producer is None:
            return

        logger.info("Flushing Kafka producer", pending=self._pending_messages)

        loop = asyncio.get_event_loop()
        await loop.run_in_executor(
            None,
            lambda: self._producer.flush(timeout=timeout_seconds),
        )

        logger.info("Kafka producer flushed")

    async def health_check(self) -> dict[str, Any]:
        """Check Kafka producer health.

        Returns:
            Health status dictionary.
        """
        if not self._is_connected or self._producer is None:
            return {
                "healthy": False,
                "connected": False,
                "error": "Producer not connected",
            }

        try:
            # Check if producer can get cluster metadata
            loop = asyncio.get_event_loop()
            partitions = await loop.run_in_executor(
                None,
                lambda: self._producer.partitions_for(self.config.topic),
            )

            return {
                "healthy": True,
                "connected": True,
                "topic": self.config.topic,
                "partitions": list(partitions) if partitions else [],
                "pending_messages": self._pending_messages,
            }

        except Exception as e:
            return {
                "healthy": False,
                "connected": self._is_connected,
                "error": str(e),
            }


class AlertKafkaProducer:
    """Kafka producer for high-priority safety alerts.

    Publishes critical safety violations to a dedicated alert topic
    for immediate notification.
    """

    def __init__(
        self,
        config: KafkaConfig,
        alert_topic: str = "ppe-alerts",
    ) -> None:
        """Initialize the alert producer.

        Args:
            config: Kafka configuration.
            alert_topic: Topic for safety alerts.
        """
        self.config = config
        self.alert_topic = alert_topic
        self._producer: KafkaProducer | None = None
        self._is_connected = False

    async def connect(self) -> None:
        """Connect to Kafka brokers."""
        if self._is_connected:
            return

        loop = asyncio.get_event_loop()
        self._producer = await loop.run_in_executor(
            None,
            lambda: KafkaProducer(
                bootstrap_servers=self.config.bootstrap_servers.split(","),
                client_id=f"{self.config.client_id}-alerts",
                acks="all",  # Ensure delivery for alerts
                compression_type="gzip",
                value_serializer=lambda v: json.dumps(v).encode("utf-8"),
                key_serializer=lambda k: k.encode("utf-8") if k else None,
                retries=5,
                retry_backoff_ms=200,
            ),
        )
        self._is_connected = True

    async def disconnect(self) -> None:
        """Disconnect from Kafka."""
        if self._producer:
            loop = asyncio.get_event_loop()
            await loop.run_in_executor(None, self._producer.close)
            self._producer = None
            self._is_connected = False

    async def publish_alert(
        self,
        result: DetectionResult,
        alert_type: str,
        severity: str = "high",
        worker_id: str | None = None,
    ) -> bool:
        """Publish a safety alert.

        Args:
            result: Detection result triggering the alert.
            alert_type: Type of alert (e.g., "missing_helmet").
            severity: Alert severity (low, medium, high, critical).
            worker_id: Worker identifier.

        Returns:
            True if alert was published.
        """
        if not self._is_connected or self._producer is None:
            return False

        alert = {
            "alert_type": alert_type,
            "severity": severity,
            "timestamp_ms": int(time.time() * 1000),
            "frame_id": result.frame_id,
            "frame_timestamp_ms": result.timestamp_ms,
            "detections": [d.to_dict() for d in result.detections],
            "worker_id": worker_id,
        }

        try:
            loop = asyncio.get_event_loop()
            await loop.run_in_executor(
                None,
                lambda: self._producer.send(
                    self.alert_topic,
                    key=worker_id or result.frame_id,
                    value=alert,
                ).get(timeout=5),  # Wait for confirmation
            )
            return True

        except Exception as e:
            logger.error("Failed to publish alert", error=str(e))
            return False
