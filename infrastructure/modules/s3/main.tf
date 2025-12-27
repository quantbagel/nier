# S3 Module for Nier Factory Floor Analytics Platform
# Provides S3 buckets with lifecycle policies for video storage and analytics

locals {
  bucket_prefix = "${var.project_name}-${var.environment}"
}

data "aws_caller_identity" "current" {}

# KMS Key for S3 encryption
resource "aws_kms_key" "s3" {
  count                   = var.kms_key_arn == null ? 1 : 0
  description             = "KMS key for S3 encryption - ${local.bucket_prefix}"
  deletion_window_in_days = 7
  enable_key_rotation     = true

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Sid    = "Enable IAM User Permissions"
        Effect = "Allow"
        Principal = {
          AWS = "arn:aws:iam::${data.aws_caller_identity.current.account_id}:root"
        }
        Action   = "kms:*"
        Resource = "*"
      },
      {
        Sid    = "Allow S3 Service"
        Effect = "Allow"
        Principal = {
          Service = "s3.amazonaws.com"
        }
        Action = [
          "kms:Encrypt",
          "kms:Decrypt",
          "kms:GenerateDataKey*"
        ]
        Resource = "*"
      }
    ]
  })

  tags = merge(var.tags, {
    Name = "${local.bucket_prefix}-s3-kms-key"
  })
}

resource "aws_kms_alias" "s3" {
  count         = var.kms_key_arn == null ? 1 : 0
  name          = "alias/${local.bucket_prefix}-s3"
  target_key_id = aws_kms_key.s3[0].key_id
}

# Raw Video Frames Bucket - stores incoming camera frames
resource "aws_s3_bucket" "raw_frames" {
  bucket = "${local.bucket_prefix}-raw-frames-${data.aws_caller_identity.current.account_id}"

  tags = merge(var.tags, {
    Name = "${local.bucket_prefix}-raw-frames"
    Type = "raw-frames"
  })
}

resource "aws_s3_bucket_versioning" "raw_frames" {
  bucket = aws_s3_bucket.raw_frames.id

  versioning_configuration {
    status = var.enable_versioning ? "Enabled" : "Disabled"
  }
}

resource "aws_s3_bucket_server_side_encryption_configuration" "raw_frames" {
  bucket = aws_s3_bucket.raw_frames.id

  rule {
    apply_server_side_encryption_by_default {
      sse_algorithm     = "aws:kms"
      kms_master_key_id = var.kms_key_arn != null ? var.kms_key_arn : aws_kms_key.s3[0].arn
    }
    bucket_key_enabled = true
  }
}

resource "aws_s3_bucket_public_access_block" "raw_frames" {
  bucket = aws_s3_bucket.raw_frames.id

  block_public_acls       = true
  block_public_policy     = true
  ignore_public_acls      = true
  restrict_public_buckets = true
}

resource "aws_s3_bucket_lifecycle_configuration" "raw_frames" {
  bucket = aws_s3_bucket.raw_frames.id

  rule {
    id     = "transition-to-ia"
    status = "Enabled"

    transition {
      days          = var.raw_frames_ia_transition_days
      storage_class = "STANDARD_IA"
    }

    transition {
      days          = var.raw_frames_glacier_transition_days
      storage_class = "GLACIER"
    }

    expiration {
      days = var.raw_frames_expiration_days
    }

    noncurrent_version_expiration {
      noncurrent_days = 30
    }
  }
}

# Processed Analytics Bucket - stores processed analytics results
resource "aws_s3_bucket" "analytics" {
  bucket = "${local.bucket_prefix}-analytics-${data.aws_caller_identity.current.account_id}"

  tags = merge(var.tags, {
    Name = "${local.bucket_prefix}-analytics"
    Type = "analytics"
  })
}

resource "aws_s3_bucket_versioning" "analytics" {
  bucket = aws_s3_bucket.analytics.id

  versioning_configuration {
    status = "Enabled"
  }
}

resource "aws_s3_bucket_server_side_encryption_configuration" "analytics" {
  bucket = aws_s3_bucket.analytics.id

  rule {
    apply_server_side_encryption_by_default {
      sse_algorithm     = "aws:kms"
      kms_master_key_id = var.kms_key_arn != null ? var.kms_key_arn : aws_kms_key.s3[0].arn
    }
    bucket_key_enabled = true
  }
}

resource "aws_s3_bucket_public_access_block" "analytics" {
  bucket = aws_s3_bucket.analytics.id

  block_public_acls       = true
  block_public_policy     = true
  ignore_public_acls      = true
  restrict_public_buckets = true
}

resource "aws_s3_bucket_lifecycle_configuration" "analytics" {
  bucket = aws_s3_bucket.analytics.id

  rule {
    id     = "transition-analytics"
    status = "Enabled"

    transition {
      days          = var.analytics_ia_transition_days
      storage_class = "STANDARD_IA"
    }

    noncurrent_version_expiration {
      noncurrent_days = 90
    }
  }
}

# ML Models Bucket - stores trained models
resource "aws_s3_bucket" "models" {
  bucket = "${local.bucket_prefix}-models-${data.aws_caller_identity.current.account_id}"

  tags = merge(var.tags, {
    Name = "${local.bucket_prefix}-models"
    Type = "ml-models"
  })
}

resource "aws_s3_bucket_versioning" "models" {
  bucket = aws_s3_bucket.models.id

  versioning_configuration {
    status = "Enabled"
  }
}

resource "aws_s3_bucket_server_side_encryption_configuration" "models" {
  bucket = aws_s3_bucket.models.id

  rule {
    apply_server_side_encryption_by_default {
      sse_algorithm     = "aws:kms"
      kms_master_key_id = var.kms_key_arn != null ? var.kms_key_arn : aws_kms_key.s3[0].arn
    }
    bucket_key_enabled = true
  }
}

resource "aws_s3_bucket_public_access_block" "models" {
  bucket = aws_s3_bucket.models.id

  block_public_acls       = true
  block_public_policy     = true
  ignore_public_acls      = true
  restrict_public_buckets = true
}

# Logs Bucket - stores application and access logs
resource "aws_s3_bucket" "logs" {
  bucket = "${local.bucket_prefix}-logs-${data.aws_caller_identity.current.account_id}"

  tags = merge(var.tags, {
    Name = "${local.bucket_prefix}-logs"
    Type = "logs"
  })
}

resource "aws_s3_bucket_versioning" "logs" {
  bucket = aws_s3_bucket.logs.id

  versioning_configuration {
    status = "Disabled"
  }
}

resource "aws_s3_bucket_server_side_encryption_configuration" "logs" {
  bucket = aws_s3_bucket.logs.id

  rule {
    apply_server_side_encryption_by_default {
      sse_algorithm = "AES256"
    }
  }
}

resource "aws_s3_bucket_public_access_block" "logs" {
  bucket = aws_s3_bucket.logs.id

  block_public_acls       = true
  block_public_policy     = true
  ignore_public_acls      = true
  restrict_public_buckets = true
}

resource "aws_s3_bucket_lifecycle_configuration" "logs" {
  bucket = aws_s3_bucket.logs.id

  rule {
    id     = "expire-logs"
    status = "Enabled"

    transition {
      days          = 30
      storage_class = "STANDARD_IA"
    }

    transition {
      days          = 90
      storage_class = "GLACIER"
    }

    expiration {
      days = var.logs_expiration_days
    }
  }
}

# Enable access logging for raw frames bucket
resource "aws_s3_bucket_logging" "raw_frames" {
  count  = var.enable_access_logging ? 1 : 0
  bucket = aws_s3_bucket.raw_frames.id

  target_bucket = aws_s3_bucket.logs.id
  target_prefix = "s3-access-logs/raw-frames/"
}

resource "aws_s3_bucket_logging" "analytics" {
  count  = var.enable_access_logging ? 1 : 0
  bucket = aws_s3_bucket.analytics.id

  target_bucket = aws_s3_bucket.logs.id
  target_prefix = "s3-access-logs/analytics/"
}

resource "aws_s3_bucket_logging" "models" {
  count  = var.enable_access_logging ? 1 : 0
  bucket = aws_s3_bucket.models.id

  target_bucket = aws_s3_bucket.logs.id
  target_prefix = "s3-access-logs/models/"
}

# Bucket policy for logs bucket to receive access logs
resource "aws_s3_bucket_policy" "logs" {
  bucket = aws_s3_bucket.logs.id

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Sid    = "AllowS3LoggingService"
        Effect = "Allow"
        Principal = {
          Service = "logging.s3.amazonaws.com"
        }
        Action = "s3:PutObject"
        Resource = "${aws_s3_bucket.logs.arn}/*"
        Condition = {
          ArnLike = {
            "aws:SourceArn" = [
              aws_s3_bucket.raw_frames.arn,
              aws_s3_bucket.analytics.arn,
              aws_s3_bucket.models.arn
            ]
          }
          StringEquals = {
            "aws:SourceAccount" = data.aws_caller_identity.current.account_id
          }
        }
      }
    ]
  })
}

# CORS configuration for analytics bucket (for dashboard access)
resource "aws_s3_bucket_cors_configuration" "analytics" {
  count  = var.enable_cors ? 1 : 0
  bucket = aws_s3_bucket.analytics.id

  cors_rule {
    allowed_headers = ["*"]
    allowed_methods = ["GET", "HEAD"]
    allowed_origins = var.cors_allowed_origins
    expose_headers  = ["ETag"]
    max_age_seconds = 3000
  }
}

# Intelligent Tiering configuration for raw frames (cost optimization)
resource "aws_s3_bucket_intelligent_tiering_configuration" "raw_frames" {
  count  = var.enable_intelligent_tiering ? 1 : 0
  bucket = aws_s3_bucket.raw_frames.id
  name   = "archive-old-frames"

  tiering {
    access_tier = "ARCHIVE_ACCESS"
    days        = 90
  }

  tiering {
    access_tier = "DEEP_ARCHIVE_ACCESS"
    days        = 180
  }
}
