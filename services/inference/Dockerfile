# Nier Inference Service - GPU-enabled Docker image
# Multi-stage build for optimal image size and security

# =============================================================================
# Stage 1: Builder
# =============================================================================
FROM nvidia/cuda:12.2.2-cudnn8-devel-ubuntu22.04 AS builder

# Prevent interactive prompts during build
ENV DEBIAN_FRONTEND=noninteractive

# Install Python and build dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    python3.11 \
    python3.11-venv \
    python3.11-dev \
    python3-pip \
    build-essential \
    git \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Set Python 3.11 as default
RUN update-alternatives --install /usr/bin/python3 python3 /usr/bin/python3.11 1 \
    && update-alternatives --install /usr/bin/python python /usr/bin/python3.11 1

# Create virtual environment
RUN python3 -m venv /opt/venv
ENV PATH="/opt/venv/bin:$PATH"

# Upgrade pip
RUN pip install --no-cache-dir --upgrade pip setuptools wheel

# Copy requirements and install dependencies
WORKDIR /build

# Copy project files
COPY pyproject.toml .
COPY src/ src/
COPY proto/ proto/

# Install dependencies
RUN pip install --no-cache-dir .

# Compile protobuf files
RUN mkdir -p src/generated && \
    python -m grpc_tools.protoc \
    -I./proto \
    --python_out=./src/generated \
    --grpc_python_out=./src/generated \
    ./proto/inference.proto || true

# =============================================================================
# Stage 2: Runtime
# =============================================================================
FROM nvidia/cuda:12.2.2-cudnn8-runtime-ubuntu22.04 AS runtime

# Prevent interactive prompts
ENV DEBIAN_FRONTEND=noninteractive

# Install Python runtime and OpenCV dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    python3.11 \
    python3.11-venv \
    libgl1 \
    libglib2.0-0 \
    libsm6 \
    libxext6 \
    libxrender1 \
    libgomp1 \
    curl \
    && rm -rf /var/lib/apt/lists/* \
    && apt-get clean

# Set Python
RUN update-alternatives --install /usr/bin/python3 python3 /usr/bin/python3.11 1 \
    && update-alternatives --install /usr/bin/python python /usr/bin/python3.11 1

# Create non-root user for security
RUN groupadd --gid 1000 nier \
    && useradd --uid 1000 --gid nier --shell /bin/bash --create-home nier

# Copy virtual environment from builder
COPY --from=builder /opt/venv /opt/venv
ENV PATH="/opt/venv/bin:$PATH"

# Create application directory
WORKDIR /app

# Copy application code
COPY --chown=nier:nier src/ src/
COPY --chown=nier:nier proto/ proto/

# Copy generated protobuf files if they exist
COPY --from=builder --chown=nier:nier /build/src/generated/ src/generated/ 2>/dev/null || true

# Create directories for models and data
RUN mkdir -p /models /data /logs \
    && chown -R nier:nier /models /data /logs /app

# Environment variables
ENV PYTHONUNBUFFERED=1 \
    PYTHONDONTWRITEBYTECODE=1 \
    PYTHONPATH=/app \
    # CUDA settings
    NVIDIA_VISIBLE_DEVICES=all \
    NVIDIA_DRIVER_CAPABILITIES=compute,utility \
    # Application settings
    NIER_ENVIRONMENT=production \
    NIER_SERVICE_NAME=nier-inference \
    MODEL_DEVICE=cuda:0 \
    MODEL_HALF_PRECISION=true \
    MODEL_PPE_MODEL_PATH=/models/ppe_detector.pt \
    SERVER_HTTP_HOST=0.0.0.0 \
    SERVER_HTTP_PORT=8080 \
    SERVER_GRPC_HOST=0.0.0.0 \
    SERVER_GRPC_PORT=50051 \
    LOG_LEVEL=INFO \
    LOG_FORMAT=json

# Expose ports
EXPOSE 8080 50051 9090

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=60s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

# Switch to non-root user
USER nier

# Default command
CMD ["python", "-m", "src.main"]

# =============================================================================
# Labels
# =============================================================================
LABEL org.opencontainers.image.title="Nier Inference Service" \
    org.opencontainers.image.description="GPU-accelerated PPE detection for factory floor analytics" \
    org.opencontainers.image.vendor="Nier" \
    org.opencontainers.image.version="0.1.0" \
    org.opencontainers.image.licenses="MIT"
