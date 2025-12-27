# MSK Module for Nier Factory Floor Analytics Platform
# Provides Managed Apache Kafka for streaming camera data

locals {
  cluster_name = "${var.project_name}-${var.environment}"
}

# Security Group for MSK
resource "aws_security_group" "msk" {
  name        = "${local.cluster_name}-msk-sg"
  description = "Security group for MSK cluster"
  vpc_id      = var.vpc_id

  # Kafka plaintext
  ingress {
    description     = "Kafka plaintext from VPC"
    from_port       = 9092
    to_port         = 9092
    protocol        = "tcp"
    cidr_blocks     = [var.vpc_cidr]
  }

  # Kafka TLS
  ingress {
    description     = "Kafka TLS from VPC"
    from_port       = 9094
    to_port         = 9094
    protocol        = "tcp"
    cidr_blocks     = [var.vpc_cidr]
  }

  # Kafka SASL/SCRAM
  ingress {
    description     = "Kafka SASL/SCRAM from VPC"
    from_port       = 9096
    to_port         = 9096
    protocol        = "tcp"
    cidr_blocks     = [var.vpc_cidr]
  }

  # Kafka IAM
  ingress {
    description     = "Kafka IAM from VPC"
    from_port       = 9098
    to_port         = 9098
    protocol        = "tcp"
    cidr_blocks     = [var.vpc_cidr]
  }

  # Zookeeper
  ingress {
    description     = "Zookeeper from VPC"
    from_port       = 2181
    to_port         = 2181
    protocol        = "tcp"
    cidr_blocks     = [var.vpc_cidr]
  }

  # JMX Exporter
  ingress {
    description     = "JMX Exporter"
    from_port       = 11001
    to_port         = 11001
    protocol        = "tcp"
    cidr_blocks     = [var.vpc_cidr]
  }

  # Node Exporter
  ingress {
    description     = "Node Exporter"
    from_port       = 11002
    to_port         = 11002
    protocol        = "tcp"
    cidr_blocks     = [var.vpc_cidr]
  }

  egress {
    from_port   = 0
    to_port     = 0
    protocol    = "-1"
    cidr_blocks = ["0.0.0.0/0"]
  }

  tags = merge(var.tags, {
    Name = "${local.cluster_name}-msk-sg"
  })
}

# KMS Key for MSK encryption
resource "aws_kms_key" "msk" {
  count                   = var.kms_key_arn == null ? 1 : 0
  description             = "KMS key for MSK encryption - ${local.cluster_name}"
  deletion_window_in_days = 7
  enable_key_rotation     = true

  tags = merge(var.tags, {
    Name = "${local.cluster_name}-msk-kms-key"
  })
}

resource "aws_kms_alias" "msk" {
  count         = var.kms_key_arn == null ? 1 : 0
  name          = "alias/${local.cluster_name}-msk"
  target_key_id = aws_kms_key.msk[0].key_id
}

# CloudWatch Log Group for MSK
resource "aws_cloudwatch_log_group" "msk" {
  name              = "/aws/msk/${local.cluster_name}"
  retention_in_days = var.log_retention_days

  tags = var.tags
}

# S3 bucket for MSK logs (optional)
resource "aws_s3_bucket" "msk_logs" {
  count  = var.enable_s3_logs ? 1 : 0
  bucket = "${local.cluster_name}-msk-logs-${data.aws_caller_identity.current.account_id}"

  tags = merge(var.tags, {
    Name = "${local.cluster_name}-msk-logs"
  })
}

resource "aws_s3_bucket_versioning" "msk_logs" {
  count  = var.enable_s3_logs ? 1 : 0
  bucket = aws_s3_bucket.msk_logs[0].id

  versioning_configuration {
    status = "Enabled"
  }
}

resource "aws_s3_bucket_server_side_encryption_configuration" "msk_logs" {
  count  = var.enable_s3_logs ? 1 : 0
  bucket = aws_s3_bucket.msk_logs[0].id

  rule {
    apply_server_side_encryption_by_default {
      sse_algorithm = "aws:kms"
    }
  }
}

resource "aws_s3_bucket_public_access_block" "msk_logs" {
  count  = var.enable_s3_logs ? 1 : 0
  bucket = aws_s3_bucket.msk_logs[0].id

  block_public_acls       = true
  block_public_policy     = true
  ignore_public_acls      = true
  restrict_public_buckets = true
}

data "aws_caller_identity" "current" {}

# MSK Configuration
resource "aws_msk_configuration" "main" {
  kafka_versions = [var.kafka_version]
  name           = "${local.cluster_name}-config"

  server_properties = <<PROPERTIES
auto.create.topics.enable=${var.auto_create_topics}
default.replication.factor=${var.default_replication_factor}
min.insync.replicas=${var.min_insync_replicas}
num.io.threads=8
num.network.threads=5
num.partitions=${var.default_partitions}
num.replica.fetchers=2
replica.lag.time.max.ms=30000
socket.receive.buffer.bytes=102400
socket.request.max.bytes=104857600
socket.send.buffer.bytes=102400
unclean.leader.election.enable=false
zookeeper.session.timeout.ms=18000
log.retention.hours=${var.log_retention_hours}
log.segment.bytes=1073741824
message.max.bytes=10485760
PROPERTIES
}

# MSK Cluster
resource "aws_msk_cluster" "main" {
  cluster_name           = local.cluster_name
  kafka_version          = var.kafka_version
  number_of_broker_nodes = var.broker_count

  broker_node_group_info {
    instance_type   = var.broker_instance_type
    client_subnets  = var.private_subnet_ids
    security_groups = [aws_security_group.msk.id]

    storage_info {
      ebs_storage_info {
        volume_size = var.broker_volume_size
        provisioned_throughput {
          enabled           = var.enable_provisioned_throughput
          volume_throughput = var.enable_provisioned_throughput ? var.provisioned_throughput : null
        }
      }
    }

    connectivity_info {
      public_access {
        type = var.enable_public_access ? "SERVICE_PROVIDED_EIPS" : "DISABLED"
      }
    }
  }

  configuration_info {
    arn      = aws_msk_configuration.main.arn
    revision = aws_msk_configuration.main.latest_revision
  }

  encryption_info {
    encryption_at_rest_kms_key_arn = var.kms_key_arn != null ? var.kms_key_arn : aws_kms_key.msk[0].arn

    encryption_in_transit {
      client_broker = var.encryption_in_transit
      in_cluster    = true
    }
  }

  client_authentication {
    sasl {
      iam   = var.enable_iam_auth
      scram = var.enable_scram_auth
    }

    unauthenticated = var.enable_unauthenticated
  }

  open_monitoring {
    prometheus {
      jmx_exporter {
        enabled_in_broker = var.enable_prometheus_jmx
      }
      node_exporter {
        enabled_in_broker = var.enable_prometheus_node
      }
    }
  }

  logging_info {
    broker_logs {
      cloudwatch_logs {
        enabled   = var.enable_cloudwatch_logs
        log_group = aws_cloudwatch_log_group.msk.name
      }

      s3 {
        enabled = var.enable_s3_logs
        bucket  = var.enable_s3_logs ? aws_s3_bucket.msk_logs[0].id : null
        prefix  = var.enable_s3_logs ? "logs/${local.cluster_name}/" : null
      }
    }
  }

  tags = merge(var.tags, {
    Name = local.cluster_name
  })
}

# Secrets Manager secret for SCRAM authentication (if enabled)
resource "aws_secretsmanager_secret" "msk_scram" {
  count       = var.enable_scram_auth ? 1 : 0
  name        = "AmazonMSK_${local.cluster_name}_credentials"
  description = "SCRAM credentials for MSK cluster ${local.cluster_name}"
  kms_key_id  = var.kms_key_arn != null ? var.kms_key_arn : aws_kms_key.msk[0].arn

  tags = var.tags
}

resource "aws_secretsmanager_secret_version" "msk_scram" {
  count     = var.enable_scram_auth ? 1 : 0
  secret_id = aws_secretsmanager_secret.msk_scram[0].id

  secret_string = jsonencode({
    username = var.scram_username
    password = var.scram_password
  })
}

resource "aws_msk_scram_secret_association" "main" {
  count           = var.enable_scram_auth ? 1 : 0
  cluster_arn     = aws_msk_cluster.main.arn
  secret_arn_list = [aws_secretsmanager_secret.msk_scram[0].arn]

  depends_on = [aws_secretsmanager_secret_version.msk_scram]
}
