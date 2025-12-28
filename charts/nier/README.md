# Nier Platform Helm Chart

Factory floor analytics platform with egocentric camera glasses.

## Architecture

```
                                    ┌─────────────────────────────────────────┐
                                    │              AWS Cloud                   │
┌──────────────┐                    │                                          │
│   Camera     │                    │  ┌─────────┐    ┌──────────┐            │
│   Glasses    │───RTSP/WebSocket──▶│  │ Ingest  │───▶│Inference │            │
│   (Workers)  │                    │  │ (Rust)  │    │ (Python) │            │
└──────────────┘                    │  └────┬────┘    └────┬─────┘            │
                                    │       │              │                   │
                                    │       ▼              ▼                   │
                                    │  ┌────────────────────────┐             │
                                    │  │      Kafka (MSK)       │             │
                                    │  └───────────┬────────────┘             │
                                    │              │                          │
                                    │       ┌──────┴──────┐                   │
                                    │       ▼             ▼                   │
                                    │  ┌─────────┐  ┌──────────┐             │
                                    │  │Pipeline │  │ Storage  │             │
                                    │  │ (Rust)  │  │  (Rust)  │             │
                                    │  └────┬────┘  └────┬─────┘             │
                                    │       │            │                    │
                                    │       ▼            ▼                    │
                                    │  ┌─────────┐  ┌──────────┐             │
                                    │  │   RDS   │  │    S3    │             │
                                    │  │(Postgres)│ │ (Frames) │             │
                                    │  └─────────┘  └──────────┘             │
                                    │       │                                 │
                                    │       ▼                                 │
                                    │  ┌──────────────────────┐    ┌────────┐│
                                    │  │     Dashboard        │◀───│  ALB   ││
                                    │  │     (Next.js)        │    └────────┘│
                                    │  └──────────────────────┘              │
                                    └─────────────────────────────────────────┘
```

## Prerequisites

- Kubernetes 1.25+
- Helm 3.10+
- AWS EKS cluster
- AWS MSK (Kafka)
- AWS RDS (PostgreSQL)
- AWS S3 bucket
- GPU nodes for inference (g4dn.xlarge or similar)

## Quick Start

```bash
# Add dependencies
helm dependency update ./charts/nier

# Install with default values
helm install nier ./charts/nier -n nier --create-namespace

# Install with custom values
helm install nier ./charts/nier -n nier --create-namespace -f my-values.yaml
```

## Configuration

### Global Settings

| Parameter | Description | Default |
|-----------|-------------|---------|
| `global.imageRegistry` | Container registry | `""` |
| `global.kafka.brokers` | MSK bootstrap servers | `kafka:9092` |
| `global.postgresql.host` | RDS endpoint | `""` |
| `global.storage.bucket` | S3 bucket name | `nier-analytics-data` |
| `global.environment` | Environment name | `production` |

### Service Configuration

| Service | Enabled | Replicas | Resources |
|---------|---------|----------|-----------|
| `ingest` | true | 3 | 500m-2000m CPU, 512Mi-2Gi |
| `inference` | true | 2 | 2-4 CPU, 4-8Gi, 1 GPU |
| `pipeline` | true | 3 | 500m-2000m CPU, 1-4Gi |
| `storage` | true | 2 | 250m-1000m CPU, 512Mi-2Gi |
| `dashboard` | true | 2 | 100m-500m CPU, 128-512Mi |

### Autoscaling

All services support HPA:

```yaml
ingest:
  autoscaling:
    enabled: true
    minReplicas: 3
    maxReplicas: 20
    targetCPUUtilizationPercentage: 70
```

## Deployment

### Development

```bash
helm install nier ./charts/nier \
  -n nier-dev \
  --create-namespace \
  -f charts/nier/environments/dev.yaml
```

### Production

```bash
helm install nier ./charts/nier \
  -n nier \
  --create-namespace \
  -f charts/nier/environments/prod.yaml \
  --set global.postgresql.existingSecret=nier-db-creds \
  --set serviceAccount.annotations."eks\.amazonaws\.com/role-arn"=arn:aws:iam::123456789:role/nier-irsa
```

## Kafka Topics

The chart creates a ConfigMap with topic definitions. Use the included script:

```bash
kubectl get cm nier-kafka-topics -n nier -o jsonpath='{.data.create-topics\.sh}' | bash
```

Topics:
- `nier.ingest.frames` - Raw video frames (12 partitions)
- `nier.inference.results` - ML results (12 partitions)
- `nier.pipeline.events` - Analytics events (6 partitions)
- `nier.alerts` - Safety alerts (3 partitions)

## Monitoring

Prometheus metrics exposed on port 9090. Dashboard includes:
- Real-time camera feeds
- PPE compliance rates
- Zone efficiency metrics
- Safety alerts

## Upgrading

```bash
helm upgrade nier ./charts/nier -n nier -f my-values.yaml
```

## Uninstalling

```bash
helm uninstall nier -n nier
```
