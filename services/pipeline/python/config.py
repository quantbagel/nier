"""
Kafka configuration for the Nier pipeline.

This module provides configuration classes for connecting to Kafka brokers
with support for SSL/SASL authentication.
"""

import os
from dataclasses import dataclass, field
from enum import Enum
from typing import Dict, Optional


class SecurityProtocol(Enum):
    """Security protocol for Kafka connections."""
    PLAINTEXT = "PLAINTEXT"
    SSL = "SSL"
    SASL_PLAINTEXT = "SASL_PLAINTEXT"
    SASL_SSL = "SASL_SSL"


class SaslMechanism(Enum):
    """SASL mechanism for authentication."""
    PLAIN = "PLAIN"
    SCRAM_SHA_256 = "SCRAM-SHA-256"
    SCRAM_SHA_512 = "SCRAM-SHA-512"
    OAUTHBEARER = "OAUTHBEARER"


@dataclass
class SslConfig:
    """SSL/TLS configuration."""
    ca_location: Optional[str] = None
    certificate_location: Optional[str] = None
    key_location: Optional[str] = None
    key_password: Optional[str] = None
    enable_verification: bool = True


@dataclass
class SaslConfig:
    """SASL authentication configuration."""
    mechanism: SaslMechanism = SaslMechanism.PLAIN
    username: Optional[str] = None
    password: Optional[str] = None
    oauth_token: Optional[str] = None


@dataclass
class ReliabilityConfig:
    """Retry and reliability configuration."""
    retries: int = 3
    retry_backoff_ms: int = 100
    request_timeout_ms: int = 30000
    enable_idempotence: bool = True
    acks: str = "all"


@dataclass
class ProducerSettings:
    """Producer-specific configuration."""
    batch_size: int = 16384
    linger_ms: int = 5
    compression_type: str = "lz4"
    max_in_flight_requests: int = 5


@dataclass
class ConsumerSettings:
    """Consumer-specific configuration."""
    group_id: str = "nier-pipeline"
    auto_offset_reset: str = "earliest"
    enable_auto_commit: bool = False
    auto_commit_interval_ms: int = 5000
    session_timeout_ms: int = 30000
    heartbeat_interval_ms: int = 3000
    max_poll_interval_ms: int = 300000
    max_poll_records: int = 500


@dataclass
class TopicConfig:
    """Topic configuration for the Nier pipeline."""
    frames: str = "nier.frames"
    detections: str = "nier.detections"
    alerts: str = "nier.alerts"
    dead_letter_queue: str = "nier.dlq"


@dataclass
class KafkaConfig:
    """Main Kafka configuration for the Nier pipeline."""
    bootstrap_servers: str = "localhost:9092"
    client_id: str = "nier-pipeline-python"
    security_protocol: SecurityProtocol = SecurityProtocol.PLAINTEXT
    ssl: SslConfig = field(default_factory=SslConfig)
    sasl: SaslConfig = field(default_factory=SaslConfig)
    reliability: ReliabilityConfig = field(default_factory=ReliabilityConfig)
    producer: ProducerSettings = field(default_factory=ProducerSettings)
    consumer: ConsumerSettings = field(default_factory=ConsumerSettings)
    topics: TopicConfig = field(default_factory=TopicConfig)
    extra_properties: Dict[str, str] = field(default_factory=dict)

    @classmethod
    def from_env(cls) -> "KafkaConfig":
        """Load configuration from environment variables."""
        config = cls()

        # Bootstrap servers
        if servers := os.environ.get("KAFKA_BOOTSTRAP_SERVERS"):
            config.bootstrap_servers = servers

        # Client ID
        if client_id := os.environ.get("KAFKA_CLIENT_ID"):
            config.client_id = client_id

        # Group ID
        if group_id := os.environ.get("KAFKA_GROUP_ID"):
            config.consumer.group_id = group_id

        # Security protocol
        if protocol := os.environ.get("KAFKA_SECURITY_PROTOCOL"):
            protocol_upper = protocol.upper()
            if protocol_upper in SecurityProtocol.__members__:
                config.security_protocol = SecurityProtocol[protocol_upper]

        # SASL credentials
        if username := os.environ.get("KAFKA_SASL_USERNAME"):
            config.sasl.username = username
        if password := os.environ.get("KAFKA_SASL_PASSWORD"):
            config.sasl.password = password

        # SSL paths
        if ca := os.environ.get("KAFKA_SSL_CA_LOCATION"):
            config.ssl.ca_location = ca

        return config

    def to_producer_config(self) -> Dict[str, str]:
        """Build a configuration dictionary for confluent-kafka Producer."""
        config = self._build_base_config()

        # Reliability settings
        config["retries"] = str(self.reliability.retries)
        config["retry.backoff.ms"] = str(self.reliability.retry_backoff_ms)
        config["request.timeout.ms"] = str(self.reliability.request_timeout_ms)
        config["acks"] = self.reliability.acks

        if self.reliability.enable_idempotence:
            config["enable.idempotence"] = "true"

        # Producer settings
        config["batch.size"] = str(self.producer.batch_size)
        config["linger.ms"] = str(self.producer.linger_ms)
        config["compression.type"] = self.producer.compression_type
        config["max.in.flight.requests.per.connection"] = str(
            self.producer.max_in_flight_requests
        )

        return config

    def to_consumer_config(self) -> Dict[str, str]:
        """Build a configuration dictionary for confluent-kafka Consumer."""
        config = self._build_base_config()

        # Consumer settings
        config["group.id"] = self.consumer.group_id
        config["auto.offset.reset"] = self.consumer.auto_offset_reset
        config["enable.auto.commit"] = str(self.consumer.enable_auto_commit).lower()
        config["auto.commit.interval.ms"] = str(self.consumer.auto_commit_interval_ms)
        config["session.timeout.ms"] = str(self.consumer.session_timeout_ms)
        config["heartbeat.interval.ms"] = str(self.consumer.heartbeat_interval_ms)
        config["max.poll.interval.ms"] = str(self.consumer.max_poll_interval_ms)

        return config

    def _build_base_config(self) -> Dict[str, str]:
        """Build base configuration common to producer and consumer."""
        config = {
            "bootstrap.servers": self.bootstrap_servers,
            "client.id": self.client_id,
            "security.protocol": self.security_protocol.value,
        }

        # SSL configuration
        if self.ssl.ca_location:
            config["ssl.ca.location"] = self.ssl.ca_location
        if self.ssl.certificate_location:
            config["ssl.certificate.location"] = self.ssl.certificate_location
        if self.ssl.key_location:
            config["ssl.key.location"] = self.ssl.key_location
        if self.ssl.key_password:
            config["ssl.key.password"] = self.ssl.key_password
        if not self.ssl.enable_verification:
            config["enable.ssl.certificate.verification"] = "false"

        # SASL configuration
        config["sasl.mechanism"] = self.sasl.mechanism.value
        if self.sasl.username:
            config["sasl.username"] = self.sasl.username
        if self.sasl.password:
            config["sasl.password"] = self.sasl.password

        # Extra properties
        config.update(self.extra_properties)

        return config

    def validate(self) -> None:
        """Validate the configuration."""
        if not self.bootstrap_servers:
            raise ValueError("bootstrap_servers is required")

        if not self.consumer.group_id:
            raise ValueError("consumer.group_id is required")

        # Validate SASL config if using SASL
        if self.security_protocol in (
            SecurityProtocol.SASL_PLAINTEXT,
            SecurityProtocol.SASL_SSL,
        ):
            if not self.sasl.username:
                raise ValueError("sasl.username is required for SASL authentication")
