# RDS Module Variables

variable "project_name" {
  description = "Name of the project for resource naming"
  type        = string
}

variable "environment" {
  description = "Environment name (dev, staging, prod)"
  type        = string
}

variable "vpc_id" {
  description = "VPC ID for the RDS instance"
  type        = string
}

variable "vpc_cidr" {
  description = "VPC CIDR block for security group rules"
  type        = string
}

variable "db_subnet_group_name" {
  description = "DB subnet group name"
  type        = string
}

variable "allowed_security_group_ids" {
  description = "Security group IDs allowed to access RDS"
  type        = list(string)
  default     = []
}

# Engine configuration
variable "engine_version" {
  description = "PostgreSQL engine version"
  type        = string
  default     = "15.4"
}

variable "engine_version_major" {
  description = "PostgreSQL major version for parameter group"
  type        = string
  default     = "15"
}

variable "instance_class" {
  description = "RDS instance class"
  type        = string
  default     = "db.t3.medium"
}

variable "allocated_storage" {
  description = "Allocated storage in GB"
  type        = number
  default     = 100
}

variable "max_allocated_storage" {
  description = "Maximum allocated storage for autoscaling in GB"
  type        = number
  default     = 500
}

variable "storage_type" {
  description = "Storage type (gp2, gp3, io1)"
  type        = string
  default     = "gp3"
}

variable "iops" {
  description = "IOPS for io1 storage type"
  type        = number
  default     = 3000
}

variable "storage_throughput" {
  description = "Storage throughput for gp3 in MiBps"
  type        = number
  default     = 125
}

# Database configuration
variable "database_name" {
  description = "Name of the database to create"
  type        = string
  default     = "nier"
}

variable "master_username" {
  description = "Master username"
  type        = string
  default     = "nieradmin"
}

variable "port" {
  description = "Database port"
  type        = number
  default     = 5432
}

# Performance parameters
variable "shared_buffers" {
  description = "Shared buffers (percentage of available memory)"
  type        = string
  default     = "{DBInstanceClassMemory/32768}"
}

variable "max_connections" {
  description = "Maximum connections"
  type        = number
  default     = 200
}

variable "work_mem" {
  description = "Work memory in KB"
  type        = string
  default     = "16384"
}

variable "maintenance_work_mem" {
  description = "Maintenance work memory in KB"
  type        = string
  default     = "524288"
}

variable "effective_cache_size" {
  description = "Effective cache size (percentage of available memory)"
  type        = string
  default     = "{DBInstanceClassMemory*3/32768}"
}

variable "log_min_duration_statement" {
  description = "Log statements taking longer than this (ms)"
  type        = string
  default     = "1000"
}

variable "force_ssl" {
  description = "Force SSL connections"
  type        = bool
  default     = true
}

# Backup configuration
variable "backup_retention_period" {
  description = "Backup retention period in days"
  type        = number
  default     = 7
}

variable "backup_window" {
  description = "Preferred backup window"
  type        = string
  default     = "03:00-04:00"
}

variable "maintenance_window" {
  description = "Preferred maintenance window"
  type        = string
  default     = "sun:04:00-sun:05:00"
}

# Encryption
variable "kms_key_arn" {
  description = "ARN of existing KMS key (creates new if null)"
  type        = string
  default     = null
}

# Monitoring
variable "monitoring_interval" {
  description = "Enhanced monitoring interval (0 to disable)"
  type        = number
  default     = 60
}

variable "performance_insights_enabled" {
  description = "Enable Performance Insights"
  type        = bool
  default     = true
}

variable "performance_insights_retention" {
  description = "Performance Insights retention in days"
  type        = number
  default     = 7
}

# Availability
variable "multi_az" {
  description = "Enable Multi-AZ deployment"
  type        = bool
  default     = false
}

# Deletion protection
variable "deletion_protection" {
  description = "Enable deletion protection"
  type        = bool
  default     = true
}

variable "skip_final_snapshot" {
  description = "Skip final snapshot on deletion"
  type        = bool
  default     = false
}

variable "auto_minor_version_upgrade" {
  description = "Enable auto minor version upgrade"
  type        = bool
  default     = true
}

variable "enable_iam_auth" {
  description = "Enable IAM database authentication"
  type        = bool
  default     = true
}

# Read Replica
variable "create_read_replica" {
  description = "Create a read replica"
  type        = bool
  default     = false
}

variable "replica_instance_class" {
  description = "Instance class for read replica"
  type        = string
  default     = "db.t3.medium"
}

# CloudWatch Alarms
variable "enable_cloudwatch_alarms" {
  description = "Enable CloudWatch alarms"
  type        = bool
  default     = true
}

variable "alarm_actions" {
  description = "SNS topic ARNs for alarm actions"
  type        = list(string)
  default     = []
}

variable "ok_actions" {
  description = "SNS topic ARNs for OK actions"
  type        = list(string)
  default     = []
}

variable "tags" {
  description = "Common tags for all resources"
  type        = map(string)
  default     = {}
}
