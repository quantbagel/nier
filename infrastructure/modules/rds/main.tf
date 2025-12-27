# RDS Module for Nier Factory Floor Analytics Platform
# Provides PostgreSQL database for metadata storage

locals {
  db_identifier = "${var.project_name}-${var.environment}"
}

# Security Group for RDS
resource "aws_security_group" "rds" {
  name        = "${local.db_identifier}-rds-sg"
  description = "Security group for RDS PostgreSQL"
  vpc_id      = var.vpc_id

  ingress {
    description     = "PostgreSQL from VPC"
    from_port       = var.port
    to_port         = var.port
    protocol        = "tcp"
    cidr_blocks     = [var.vpc_cidr]
  }

  # Allow from EKS nodes security group
  dynamic "ingress" {
    for_each = var.allowed_security_group_ids
    content {
      description     = "PostgreSQL from allowed security groups"
      from_port       = var.port
      to_port         = var.port
      protocol        = "tcp"
      security_groups = [ingress.value]
    }
  }

  egress {
    from_port   = 0
    to_port     = 0
    protocol    = "-1"
    cidr_blocks = ["0.0.0.0/0"]
  }

  tags = merge(var.tags, {
    Name = "${local.db_identifier}-rds-sg"
  })
}

# KMS Key for RDS encryption
resource "aws_kms_key" "rds" {
  count                   = var.kms_key_arn == null ? 1 : 0
  description             = "KMS key for RDS encryption - ${local.db_identifier}"
  deletion_window_in_days = 7
  enable_key_rotation     = true

  tags = merge(var.tags, {
    Name = "${local.db_identifier}-rds-kms-key"
  })
}

resource "aws_kms_alias" "rds" {
  count         = var.kms_key_arn == null ? 1 : 0
  name          = "alias/${local.db_identifier}-rds"
  target_key_id = aws_kms_key.rds[0].key_id
}

# DB Parameter Group
resource "aws_db_parameter_group" "main" {
  name        = "${local.db_identifier}-pg-params"
  family      = "postgres${var.engine_version_major}"
  description = "PostgreSQL parameter group for ${local.db_identifier}"

  # Performance parameters
  parameter {
    name  = "shared_buffers"
    value = var.shared_buffers
  }

  parameter {
    name  = "max_connections"
    value = var.max_connections
  }

  parameter {
    name  = "work_mem"
    value = var.work_mem
  }

  parameter {
    name  = "maintenance_work_mem"
    value = var.maintenance_work_mem
  }

  parameter {
    name  = "effective_cache_size"
    value = var.effective_cache_size
  }

  # Logging parameters
  parameter {
    name  = "log_min_duration_statement"
    value = var.log_min_duration_statement
  }

  parameter {
    name  = "log_statement"
    value = "ddl"
  }

  # Security parameters
  parameter {
    name  = "rds.force_ssl"
    value = var.force_ssl ? "1" : "0"
  }

  tags = var.tags
}

# Secrets Manager for database credentials
resource "aws_secretsmanager_secret" "db_credentials" {
  name        = "${local.db_identifier}-db-credentials"
  description = "Database credentials for ${local.db_identifier}"
  kms_key_id  = var.kms_key_arn != null ? var.kms_key_arn : aws_kms_key.rds[0].arn

  tags = var.tags
}

resource "random_password" "master" {
  length           = 32
  special          = true
  override_special = "!#$%&*()-_=+[]{}<>:?"
}

resource "aws_secretsmanager_secret_version" "db_credentials" {
  secret_id = aws_secretsmanager_secret.db_credentials.id

  secret_string = jsonencode({
    username = var.master_username
    password = random_password.master.result
    host     = aws_db_instance.main.address
    port     = var.port
    database = var.database_name
    engine   = "postgres"
  })
}

# IAM Role for Enhanced Monitoring
resource "aws_iam_role" "rds_monitoring" {
  count = var.monitoring_interval > 0 ? 1 : 0
  name  = "${local.db_identifier}-rds-monitoring-role"

  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Action = "sts:AssumeRole"
        Effect = "Allow"
        Principal = {
          Service = "monitoring.rds.amazonaws.com"
        }
      }
    ]
  })

  tags = var.tags
}

resource "aws_iam_role_policy_attachment" "rds_monitoring" {
  count      = var.monitoring_interval > 0 ? 1 : 0
  role       = aws_iam_role.rds_monitoring[0].name
  policy_arn = "arn:aws:iam::aws:policy/service-role/AmazonRDSEnhancedMonitoringRole"
}

# RDS Instance
resource "aws_db_instance" "main" {
  identifier = local.db_identifier

  # Engine configuration
  engine               = "postgres"
  engine_version       = var.engine_version
  instance_class       = var.instance_class
  allocated_storage    = var.allocated_storage
  max_allocated_storage = var.max_allocated_storage
  storage_type         = var.storage_type
  iops                 = var.storage_type == "io1" ? var.iops : null
  storage_throughput   = var.storage_type == "gp3" ? var.storage_throughput : null

  # Database configuration
  db_name  = var.database_name
  username = var.master_username
  password = random_password.master.result
  port     = var.port

  # Network configuration
  db_subnet_group_name   = var.db_subnet_group_name
  vpc_security_group_ids = [aws_security_group.rds.id]
  publicly_accessible    = false

  # Parameter and option groups
  parameter_group_name = aws_db_parameter_group.main.name

  # Backup configuration
  backup_retention_period = var.backup_retention_period
  backup_window          = var.backup_window
  maintenance_window     = var.maintenance_window
  copy_tags_to_snapshot  = true

  # Encryption
  storage_encrypted = true
  kms_key_id       = var.kms_key_arn != null ? var.kms_key_arn : aws_kms_key.rds[0].arn

  # Monitoring
  monitoring_interval             = var.monitoring_interval
  monitoring_role_arn            = var.monitoring_interval > 0 ? aws_iam_role.rds_monitoring[0].arn : null
  performance_insights_enabled   = var.performance_insights_enabled
  performance_insights_retention_period = var.performance_insights_enabled ? var.performance_insights_retention : null
  performance_insights_kms_key_id = var.performance_insights_enabled ? (var.kms_key_arn != null ? var.kms_key_arn : aws_kms_key.rds[0].arn) : null

  # Availability
  multi_az = var.multi_az

  # Deletion protection
  deletion_protection      = var.deletion_protection
  skip_final_snapshot     = var.skip_final_snapshot
  final_snapshot_identifier = var.skip_final_snapshot ? null : "${local.db_identifier}-final-snapshot"

  # Minor version auto upgrade
  auto_minor_version_upgrade = var.auto_minor_version_upgrade

  # Enable IAM authentication
  iam_database_authentication_enabled = var.enable_iam_auth

  tags = merge(var.tags, {
    Name = local.db_identifier
  })

  depends_on = [aws_iam_role_policy_attachment.rds_monitoring]
}

# Read Replica (optional)
resource "aws_db_instance" "replica" {
  count      = var.create_read_replica ? 1 : 0
  identifier = "${local.db_identifier}-replica"

  replicate_source_db = aws_db_instance.main.identifier
  instance_class      = var.replica_instance_class
  storage_type        = var.storage_type

  publicly_accessible    = false
  vpc_security_group_ids = [aws_security_group.rds.id]

  # Backup configuration (disabled for replica)
  backup_retention_period = 0

  # Encryption
  storage_encrypted = true
  kms_key_id       = var.kms_key_arn != null ? var.kms_key_arn : aws_kms_key.rds[0].arn

  # Monitoring
  monitoring_interval           = var.monitoring_interval
  monitoring_role_arn          = var.monitoring_interval > 0 ? aws_iam_role.rds_monitoring[0].arn : null
  performance_insights_enabled = var.performance_insights_enabled
  performance_insights_retention_period = var.performance_insights_enabled ? var.performance_insights_retention : null

  # Deletion protection
  deletion_protection  = false
  skip_final_snapshot = true

  auto_minor_version_upgrade = var.auto_minor_version_upgrade

  tags = merge(var.tags, {
    Name = "${local.db_identifier}-replica"
  })
}

# CloudWatch Alarms for RDS
resource "aws_cloudwatch_metric_alarm" "cpu_high" {
  count               = var.enable_cloudwatch_alarms ? 1 : 0
  alarm_name          = "${local.db_identifier}-cpu-high"
  comparison_operator = "GreaterThanThreshold"
  evaluation_periods  = 2
  metric_name         = "CPUUtilization"
  namespace           = "AWS/RDS"
  period              = 300
  statistic           = "Average"
  threshold           = 80
  alarm_description   = "High CPU utilization on ${local.db_identifier}"

  dimensions = {
    DBInstanceIdentifier = aws_db_instance.main.identifier
  }

  alarm_actions = var.alarm_actions
  ok_actions    = var.ok_actions

  tags = var.tags
}

resource "aws_cloudwatch_metric_alarm" "storage_low" {
  count               = var.enable_cloudwatch_alarms ? 1 : 0
  alarm_name          = "${local.db_identifier}-storage-low"
  comparison_operator = "LessThanThreshold"
  evaluation_periods  = 2
  metric_name         = "FreeStorageSpace"
  namespace           = "AWS/RDS"
  period              = 300
  statistic           = "Average"
  threshold           = 10737418240 # 10GB
  alarm_description   = "Low storage space on ${local.db_identifier}"

  dimensions = {
    DBInstanceIdentifier = aws_db_instance.main.identifier
  }

  alarm_actions = var.alarm_actions
  ok_actions    = var.ok_actions

  tags = var.tags
}

resource "aws_cloudwatch_metric_alarm" "connections_high" {
  count               = var.enable_cloudwatch_alarms ? 1 : 0
  alarm_name          = "${local.db_identifier}-connections-high"
  comparison_operator = "GreaterThanThreshold"
  evaluation_periods  = 2
  metric_name         = "DatabaseConnections"
  namespace           = "AWS/RDS"
  period              = 300
  statistic           = "Average"
  threshold           = var.max_connections * 0.8
  alarm_description   = "High connection count on ${local.db_identifier}"

  dimensions = {
    DBInstanceIdentifier = aws_db_instance.main.identifier
  }

  alarm_actions = var.alarm_actions
  ok_actions    = var.ok_actions

  tags = var.tags
}
