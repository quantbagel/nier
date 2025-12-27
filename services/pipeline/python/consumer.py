"""
Kafka consumer wrapper for the Nier pipeline.

This module provides a high-level, type-safe interface for consuming messages
from Kafka topics with support for JSON deserialization and reliable processing.
"""

import logging
import signal
import threading
from abc import ABC, abstractmethod
from dataclasses import dataclass, field
from datetime import datetime
from typing import Any, Callable, Dict, List, Optional, Type, TypeVar, Union

from confluent_kafka import Consumer, KafkaError, KafkaException, TopicPartition

from .config import KafkaConfig
from .producer import NierProducer
from .schemas import Alert, DetectionEvent, FrameMetadata

logger = logging.getLogger(__name__)

T = TypeVar("T")


@dataclass
class MessageMetadata:
    """Metadata about a received message."""
    topic: str
    partition: int
    offset: int
    key: Optional[bytes] = None
    timestamp: Optional[int] = None
    headers: Dict[str, str] = field(default_factory=dict)


@dataclass
class IncomingMessage:
    """A received message with payload and metadata."""
    payload: bytes
    metadata: MessageMetadata

    def decode_json(self, cls: Type[T]) -> T:
        """
        Deserialize the payload as a schema object.

        Args:
            cls: Schema class to deserialize to (DetectionEvent, FrameMetadata, Alert)

        Returns:
            Deserialized object
        """
        return cls.from_bytes(self.payload)

    def decode_detection_event(self) -> DetectionEvent:
        """Deserialize as a DetectionEvent."""
        return DetectionEvent.from_bytes(self.payload)

    def decode_frame_metadata(self) -> FrameMetadata:
        """Deserialize as FrameMetadata."""
        return FrameMetadata.from_bytes(self.payload)

    def decode_alert(self) -> Alert:
        """Deserialize as an Alert."""
        return Alert.from_bytes(self.payload)

    @property
    def key_str(self) -> Optional[str]:
        """Get the message key as a string."""
        if self.metadata.key:
            return self.metadata.key.decode("utf-8")
        return None

    def header(self, key: str) -> Optional[str]:
        """Get a header value."""
        return self.metadata.headers.get(key)

    @property
    def correlation_id(self) -> Optional[str]:
        """Get the correlation ID header."""
        return self.header("correlation-id")

    @property
    def message_type(self) -> Optional[str]:
        """Get the message type header."""
        return self.header("message-type")


class MessageHandler(ABC):
    """Abstract base class for message handlers."""

    @abstractmethod
    def handle(self, message: IncomingMessage) -> None:
        """
        Process a single message.

        Args:
            message: Incoming message to process

        Raises:
            Exception: If processing fails
        """
        pass

    def on_error(self, message: IncomingMessage, error: Exception) -> None:
        """
        Called when message processing fails.

        Args:
            message: Message that failed to process
            error: Exception that occurred
        """
        logger.warning(
            "Message processing failed for topic=%s, partition=%d, offset=%d: %s",
            message.metadata.topic,
            message.metadata.partition,
            message.metadata.offset,
            error,
        )


class FunctionHandler(MessageHandler):
    """Wrapper to use a function as a message handler."""

    def __init__(self, func: Callable[[IncomingMessage], None]):
        self._func = func

    def handle(self, message: IncomingMessage) -> None:
        self._func(message)


class ConsumerError(Exception):
    """Exception raised for consumer errors."""
    pass


class NierConsumer:
    """High-level Kafka consumer wrapper for the Nier pipeline."""

    def __init__(
        self,
        config: KafkaConfig,
        dlq_producer: Optional[NierProducer] = None,
    ):
        """
        Create a new consumer with the given configuration.

        Args:
            config: Kafka configuration
            dlq_producer: Optional producer for dead letter queue
        """
        self.config = config
        self._consumer = Consumer(config.to_consumer_config())
        self._dlq_producer = dlq_producer
        self._running = False
        self._shutdown_event = threading.Event()

        logger.info(
            "Created Kafka consumer for %s with group %s",
            config.bootstrap_servers,
            config.consumer.group_id,
        )

    def subscribe(self, topics: List[str]) -> None:
        """
        Subscribe to the specified topics.

        Args:
            topics: List of topic names to subscribe to
        """
        logger.info("Subscribing to topics: %s", topics)
        self._consumer.subscribe(topics)

    def subscribe_all(self) -> None:
        """Subscribe to all Nier pipeline topics."""
        self.subscribe([
            self.config.topics.frames,
            self.config.topics.detections,
            self.config.topics.alerts,
        ])

    def subscribe_frames(self) -> None:
        """Subscribe to frame metadata topic."""
        self.subscribe([self.config.topics.frames])

    def subscribe_detections(self) -> None:
        """Subscribe to detection events topic."""
        self.subscribe([self.config.topics.detections])

    def subscribe_alerts(self) -> None:
        """Subscribe to alerts topic."""
        self.subscribe([self.config.topics.alerts])

    def commit(self) -> None:
        """Commit the current offsets synchronously."""
        self._consumer.commit()

    def commit_async(self) -> None:
        """Commit offsets asynchronously."""
        self._consumer.commit(asynchronous=True)

    def shutdown(self) -> None:
        """Signal shutdown to stop consuming."""
        logger.info("Signaling consumer shutdown")
        self._running = False
        self._shutdown_event.set()

    def run(
        self,
        handler: Union[MessageHandler, Callable[[IncomingMessage], None]],
        poll_timeout: float = 1.0,
    ) -> None:
        """
        Start consuming messages and process them with the given handler.

        Args:
            handler: Message handler or callback function
            poll_timeout: Timeout for each poll in seconds
        """
        if callable(handler) and not isinstance(handler, MessageHandler):
            handler = FunctionHandler(handler)

        self._running = True
        self._shutdown_event.clear()

        # Set up signal handlers for graceful shutdown
        original_sigint = signal.getsignal(signal.SIGINT)
        original_sigterm = signal.getsignal(signal.SIGTERM)

        def signal_handler(signum, frame):
            logger.info("Received signal %d, shutting down...", signum)
            self.shutdown()

        signal.signal(signal.SIGINT, signal_handler)
        signal.signal(signal.SIGTERM, signal_handler)

        logger.info("Starting message consumption loop")

        try:
            while self._running:
                msg = self._consumer.poll(poll_timeout)

                if msg is None:
                    continue

                if msg.error():
                    if msg.error().code() == KafkaError._PARTITION_EOF:
                        logger.debug(
                            "Reached end of partition %s [%d]",
                            msg.topic(),
                            msg.partition(),
                        )
                    else:
                        logger.error("Consumer error: %s", msg.error())
                    continue

                # Convert to IncomingMessage
                incoming = self._convert_message(msg)

                logger.debug(
                    "Received message from topic=%s, partition=%d, offset=%d",
                    incoming.metadata.topic,
                    incoming.metadata.partition,
                    incoming.metadata.offset,
                )

                try:
                    handler.handle(incoming)

                    if not self.config.consumer.enable_auto_commit:
                        self.commit_async()

                except Exception as e:
                    logger.error("Message processing failed: %s", e)
                    handler.on_error(incoming, e)

                    # Send to DLQ if configured
                    if self._dlq_producer:
                        try:
                            self._dlq_producer.send_to_dlq(
                                incoming.metadata.topic,
                                incoming.payload,
                                str(e),
                            )
                        except Exception as dlq_error:
                            logger.error("Failed to send to DLQ: %s", dlq_error)

        finally:
            # Restore original signal handlers
            signal.signal(signal.SIGINT, original_sigint)
            signal.signal(signal.SIGTERM, original_sigterm)

            # Final commit before shutdown
            if not self.config.consumer.enable_auto_commit:
                try:
                    self.commit()
                except Exception as e:
                    logger.warning("Failed to commit on shutdown: %s", e)

            logger.info("Consumer stopped")

    def run_with_callback(
        self,
        callback: Callable[[IncomingMessage], None],
        poll_timeout: float = 1.0,
    ) -> None:
        """
        Consume messages with a simple callback function.

        Args:
            callback: Function to call for each message
            poll_timeout: Timeout for each poll in seconds
        """
        self.run(FunctionHandler(callback), poll_timeout)

    def _convert_message(self, msg: Any) -> IncomingMessage:
        """Convert a confluent-kafka message to IncomingMessage."""
        headers = {}
        if msg.headers():
            for key, value in msg.headers():
                if value is not None:
                    headers[key] = value.decode("utf-8")

        return IncomingMessage(
            payload=msg.value() or b"",
            metadata=MessageMetadata(
                topic=msg.topic(),
                partition=msg.partition(),
                offset=msg.offset(),
                key=msg.key(),
                timestamp=msg.timestamp()[1] if msg.timestamp()[0] != 0 else None,
                headers=headers,
            ),
        )

    def assignment(self) -> List[TopicPartition]:
        """Get the current partition assignment."""
        return self._consumer.assignment()

    def position(self, partitions: List[TopicPartition]) -> List[TopicPartition]:
        """Get the current position for partitions."""
        return self._consumer.position(partitions)

    def pause(self, partitions: List[TopicPartition]) -> None:
        """Pause consumption for specific partitions."""
        self._consumer.pause(partitions)

    def resume(self, partitions: List[TopicPartition]) -> None:
        """Resume consumption for specific partitions."""
        self._consumer.resume(partitions)

    def close(self) -> None:
        """Close the consumer."""
        self._consumer.close()
        logger.info("Consumer closed")

    def __enter__(self) -> "NierConsumer":
        return self

    def __exit__(self, exc_type, exc_val, exc_tb) -> None:
        self.close()


class ConsumerBuilder:
    """Builder for creating consumers with custom settings."""

    def __init__(self, bootstrap_servers: str):
        """
        Create a new consumer builder.

        Args:
            bootstrap_servers: Comma-separated list of broker addresses
        """
        self.config = KafkaConfig(bootstrap_servers=bootstrap_servers)
        self._dlq_producer: Optional[NierProducer] = None

    def group_id(self, group_id: str) -> "ConsumerBuilder":
        """Set the consumer group ID."""
        self.config.consumer.group_id = group_id
        return self

    def client_id(self, client_id: str) -> "ConsumerBuilder":
        """Set the client ID."""
        self.config.client_id = client_id
        return self

    def auto_offset_reset(self, reset: str) -> "ConsumerBuilder":
        """Set auto offset reset behavior."""
        self.config.consumer.auto_offset_reset = reset
        return self

    def enable_auto_commit(self, enable: bool) -> "ConsumerBuilder":
        """Enable or disable auto commit."""
        self.config.consumer.enable_auto_commit = enable
        return self

    def with_dlq_producer(self, producer: NierProducer) -> "ConsumerBuilder":
        """Set the dead letter queue producer."""
        self._dlq_producer = producer
        return self

    def build(self) -> NierConsumer:
        """Build the consumer."""
        return NierConsumer(self.config, self._dlq_producer)
