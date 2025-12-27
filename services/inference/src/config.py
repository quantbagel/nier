"""Configuration management for the inference service."""

from __future__ import annotations

from functools import lru_cache
from pathlib import Path
from typing import Literal

from pydantic import Field, field_validator
from pydantic_settings import BaseSettings, SettingsConfigDict


class ModelConfig(BaseSettings):
    """Configuration for ML models."""

    model_config = SettingsConfigDict(env_prefix="MODEL_")

    # Model paths and settings
    ppe_model_path: Path = Field(
        default=Path("/models/ppe_detector.pt"),
        description="Path to the PPE detection model",
    )
    ppe_model_type: Literal["yolo", "onnx"] = Field(
        default="yolo",
        description="Type of PPE detection model (yolo or onnx)",
    )

    # Inference settings
    confidence_threshold: float = Field(
        default=0.5,
        ge=0.0,
        le=1.0,
        description="Minimum confidence threshold for detections",
    )
    iou_threshold: float = Field(
        default=0.45,
        ge=0.0,
        le=1.0,
        description="IoU threshold for NMS",
    )
    max_detections: int = Field(
        default=100,
        ge=1,
        description="Maximum number of detections per image",
    )

    # Batching settings
    batch_size: int = Field(
        default=8,
        ge=1,
        le=64,
        description="Maximum batch size for inference",
    )
    batch_timeout_ms: int = Field(
        default=50,
        ge=1,
        le=1000,
        description="Maximum time to wait for batch to fill (ms)",
    )

    # GPU settings
    device: str = Field(
        default="cuda:0",
        description="Device to run inference on (cuda:X or cpu)",
    )
    half_precision: bool = Field(
        default=True,
        description="Use FP16 inference for faster processing",
    )
    gpu_memory_fraction: float = Field(
        default=0.8,
        ge=0.1,
        le=1.0,
        description="Fraction of GPU memory to use",
    )


class KafkaConfig(BaseSettings):
    """Configuration for Kafka producer."""

    model_config = SettingsConfigDict(env_prefix="KAFKA_")

    bootstrap_servers: str = Field(
        default="localhost:9092",
        description="Kafka bootstrap servers (comma-separated)",
    )
    topic: str = Field(
        default="ppe-detections",
        description="Kafka topic for detection results",
    )
    client_id: str = Field(
        default="nier-inference",
        description="Kafka client ID",
    )

    # Producer settings
    acks: Literal["all", "0", "1"] = Field(
        default="all",
        description="Kafka acknowledgment level",
    )
    compression_type: Literal["none", "gzip", "snappy", "lz4", "zstd"] = Field(
        default="lz4",
        description="Compression type for Kafka messages",
    )
    batch_size: int = Field(
        default=16384,
        ge=0,
        description="Kafka producer batch size in bytes",
    )
    linger_ms: int = Field(
        default=10,
        ge=0,
        description="Time to wait before sending a batch (ms)",
    )
    max_request_size: int = Field(
        default=1048576,
        ge=1,
        description="Maximum size of a Kafka request in bytes",
    )

    @field_validator("bootstrap_servers")
    @classmethod
    def validate_bootstrap_servers(cls, v: str) -> str:
        """Validate bootstrap servers format."""
        servers = v.split(",")
        for server in servers:
            if ":" not in server:
                raise ValueError(f"Invalid server format: {server}. Expected host:port")
        return v


class ServerConfig(BaseSettings):
    """Configuration for HTTP and gRPC servers."""

    model_config = SettingsConfigDict(env_prefix="SERVER_")

    # FastAPI settings
    http_host: str = Field(default="0.0.0.0", description="HTTP server host")
    http_port: int = Field(default=8080, ge=1, le=65535, description="HTTP server port")

    # gRPC settings
    grpc_host: str = Field(default="0.0.0.0", description="gRPC server host")
    grpc_port: int = Field(default=50051, ge=1, le=65535, description="gRPC server port")
    grpc_max_workers: int = Field(default=10, ge=1, description="Max gRPC worker threads")
    grpc_max_message_length: int = Field(
        default=100 * 1024 * 1024,  # 100MB
        ge=1,
        description="Max gRPC message size in bytes",
    )

    # Request settings
    max_concurrent_requests: int = Field(
        default=100,
        ge=1,
        description="Maximum concurrent inference requests",
    )
    request_timeout_seconds: float = Field(
        default=30.0,
        ge=1.0,
        description="Request timeout in seconds",
    )


class LoggingConfig(BaseSettings):
    """Configuration for logging."""

    model_config = SettingsConfigDict(env_prefix="LOG_")

    level: Literal["DEBUG", "INFO", "WARNING", "ERROR", "CRITICAL"] = Field(
        default="INFO",
        description="Logging level",
    )
    format: Literal["json", "console"] = Field(
        default="json",
        description="Log format (json for production, console for development)",
    )
    include_timestamp: bool = Field(
        default=True,
        description="Include timestamp in log messages",
    )


class Settings(BaseSettings):
    """Main application settings aggregating all configuration."""

    model_config = SettingsConfigDict(
        env_prefix="NIER_",
        env_nested_delimiter="__",
        case_sensitive=False,
    )

    # Service metadata
    service_name: str = Field(default="nier-inference", description="Service name")
    environment: Literal["development", "staging", "production"] = Field(
        default="development",
        description="Deployment environment",
    )

    # Nested configurations
    model: ModelConfig = Field(default_factory=ModelConfig)
    kafka: KafkaConfig = Field(default_factory=KafkaConfig)
    server: ServerConfig = Field(default_factory=ServerConfig)
    logging: LoggingConfig = Field(default_factory=LoggingConfig)

    # Health check settings
    health_check_interval_seconds: float = Field(
        default=30.0,
        ge=1.0,
        description="Interval for health checks",
    )

    # Metrics
    enable_metrics: bool = Field(
        default=True,
        description="Enable Prometheus metrics",
    )
    metrics_port: int = Field(
        default=9090,
        ge=1,
        le=65535,
        description="Prometheus metrics port",
    )


@lru_cache
def get_settings() -> Settings:
    """Get cached application settings.

    Returns:
        Settings: Application configuration instance.
    """
    return Settings()
