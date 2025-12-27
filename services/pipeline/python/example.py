#!/usr/bin/env python3
"""
Example usage of the Nier Pipeline Python wrapper.

This script demonstrates how to use the Python wrapper for the inference service.
"""

import logging
import sys
import uuid
from datetime import datetime

from nier_pipeline import (
    Alert,
    AlertSeverity,
    AlertStatus,
    AlertType,
    BoundingBox,
    ConfidenceScore,
    ConsumerBuilder,
    DetectionEvent,
    FrameMetadata,
    IncomingMessage,
    KafkaConfig,
    MessageHandler,
    NierConsumer,
    NierProducer,
    PPEViolation,
    PPEViolationType,
    ProducerBuilder,
)

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s - %(name)s - %(levelname)s - %(message)s",
)
logger = logging.getLogger(__name__)


class DetectionHandler(MessageHandler):
    """Example handler for detection events."""

    def __init__(self, producer: NierProducer):
        self.producer = producer

    def handle(self, message: IncomingMessage) -> None:
        logger.info(
            "Processing message from topic=%s, partition=%d, offset=%d",
            message.metadata.topic,
            message.metadata.partition,
            message.metadata.offset,
        )

        message_type = message.message_type

        if message_type == "detection_event":
            event = message.decode_detection_event()
            logger.info(
                "Detection event %s: %d PPE violations, %d activity detections",
                event.event_id,
                len(event.ppe_violations),
                len(event.activity_detections),
            )

            # Generate alert if PPE violation detected
            if event.ppe_violations:
                self._create_alert(event)

        elif message_type == "frame_metadata":
            metadata = message.decode_frame_metadata()
            logger.info(
                "Frame metadata %s: device=%s, size=%d bytes",
                metadata.frame_id,
                metadata.device_id,
                metadata.frame_size_bytes,
            )

        elif message_type == "alert":
            alert = message.decode_alert()
            logger.info(
                "Alert %s: %s - %s",
                alert.alert_id,
                alert.title,
                alert.severity.name,
            )

        else:
            logger.warning("Unknown message type: %s", message_type)

    def _create_alert(self, event: DetectionEvent) -> None:
        """Create an alert from a detection event."""
        now = datetime.utcnow()
        alert = Alert(
            alert_id=str(uuid.uuid4()),
            alert_type=AlertType.PPE_VIOLATION,
            severity=AlertSeverity.WARNING,
            status=AlertStatus.NEW,
            title="PPE Violation Detected",
            description=f"Detected {len(event.ppe_violations)} PPE violation(s)",
            created_at=now,
            updated_at=now,
            device_id=event.device_id,
            rule_id="ppe-violation-rule",
            source_detection_ids=[event.event_id],
        )

        try:
            result = self.producer.send_alert(alert)
            logger.info(
                "Alert sent to partition %d at offset %d",
                result.partition,
                result.offset,
            )
        except Exception as e:
            logger.error("Failed to send alert: %s", e)

    def on_error(self, message: IncomingMessage, error: Exception) -> None:
        logger.error(
            "Failed to process message from offset %d: %s",
            message.metadata.offset,
            error,
        )


def run_producer_example(config: KafkaConfig) -> None:
    """Run producer example - send sample messages."""
    logger.info("Starting producer example")

    producer = NierProducer(config)

    # Create a sample detection event
    now = datetime.utcnow()
    event = DetectionEvent(
        event_id=str(uuid.uuid4()),
        frame_id=str(uuid.uuid4()),
        device_id="glasses-001",
        timestamp=now,
        model_id="yolo-ppe-v2",
        model_version="2.1.0",
        processing_latency_ms=45,
        ppe_violations=[
            PPEViolation(
                violation_type=PPEViolationType.NO_HELMET,
                bounding_box=BoundingBox(x_min=0.1, y_min=0.2, x_max=0.3, y_max=0.5),
                confidence=ConfidenceScore(overall=0.95),
            )
        ],
    )

    # Send the detection event
    result = producer.send_detection_event(event)
    logger.info(
        "Sent detection event to partition %d at offset %d",
        result.partition,
        result.offset,
    )

    # Create a sample frame metadata
    frame = FrameMetadata(
        frame_id=str(uuid.uuid4()),
        device_id="glasses-001",
        capture_timestamp=now,
        upload_timestamp=now,
        width=1920,
        height=1080,
        frame_number=12345,
        frame_data_uri="s3://nier-frames/2024/01/15/frame-12345.jpg",
        frame_size_bytes=256000,
        session_id="session-abc",
    )

    result = producer.send_frame_metadata(frame)
    logger.info(
        "Sent frame metadata to partition %d at offset %d",
        result.partition,
        result.offset,
    )

    producer.flush()
    logger.info("Producer finished")


def run_consumer_example(config: KafkaConfig) -> None:
    """Run consumer example - receive and process messages."""
    logger.info("Starting consumer example")

    producer = NierProducer(config)
    consumer = (
        ConsumerBuilder(config.bootstrap_servers)
        .group_id(config.consumer.group_id)
        .client_id("nier-example-consumer")
        .auto_offset_reset("earliest")
        .enable_auto_commit(False)
        .with_dlq_producer(producer)
        .build()
    )

    # Subscribe to detection events
    consumer.subscribe_detections()

    # Create handler
    handler = DetectionHandler(producer)

    # Run consumer (blocks until shutdown)
    consumer.run(handler)

    consumer.close()
    logger.info("Consumer finished")


def main() -> None:
    """Main entry point."""
    logger.info("Nier Pipeline Python Example")
    logger.info("============================")

    config = KafkaConfig.from_env()
    logger.info("Kafka brokers: %s", config.bootstrap_servers)
    logger.info("Consumer group: %s", config.consumer.group_id)

    if len(sys.argv) < 2:
        print("Usage: python example.py [producer|consumer]")
        print()
        print("Modes:")
        print("  producer - Send sample messages to Kafka")
        print("  consumer - Receive and process messages from Kafka")
        print()
        print("Environment variables:")
        print("  KAFKA_BOOTSTRAP_SERVERS - Kafka broker addresses")
        print("  KAFKA_GROUP_ID          - Consumer group ID")
        sys.exit(1)

    mode = sys.argv[1]

    if mode == "producer":
        run_producer_example(config)
    elif mode == "consumer":
        run_consumer_example(config)
    else:
        print(f"Unknown mode: {mode}")
        sys.exit(1)


if __name__ == "__main__":
    main()
