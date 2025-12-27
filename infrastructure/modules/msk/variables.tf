# MSK Module Variables

variable "project_name" {
  description = "Name of the project for resource naming"
  type        = string
}

variable "environment" {
  description = "Environment name (dev, staging, prod)"
  type        = string
}

variable "vpc_id" {
  description = "VPC ID for the MSK cluster"
  type        = string
}

variable "vpc_cidr" {
  description = "VPC CIDR block for security group rules"
  type        = string
}

variable "private_subnet_ids" {
  description = "Private subnet IDs for MSK brokers"
  type        = list(string)
}

variable "kafka_version" {
  description = "Apache Kafka version"
  type        = string
  default     = "3.5.1"
}

variable "broker_count" {
  description = "Number of Kafka broker nodes"
  type        = number
  default     = 3
}

variable "broker_instance_type" {
  description = "Instance type for Kafka brokers"
  type        = string
  default     = "kafka.m5.large"
}

variable "broker_volume_size" {
  description = "EBS volume size for brokers in GB"
  type        = number
  default     = 500
}

variable "enable_provisioned_throughput" {
  description = "Enable provisioned throughput for EBS volumes"
  type        = bool
  default     = false
}

variable "provisioned_throughput" {
  description = "Provisioned throughput in MiB/s (if enabled)"
  type        = number
  default     = 250
}

variable "kms_key_arn" {
  description = "ARN of existing KMS key (creates new if null)"
  type        = string
  default     = null
}

variable "encryption_in_transit" {
  description = "Encryption setting for data in transit (TLS, TLS_PLAINTEXT, PLAINTEXT)"
  type        = string
  default     = "TLS"
}

variable "enable_iam_auth" {
  description = "Enable IAM authentication"
  type        = bool
  default     = true
}

variable "enable_scram_auth" {
  description = "Enable SCRAM authentication"
  type        = bool
  default     = false
}

variable "enable_unauthenticated" {
  description = "Enable unauthenticated access"
  type        = bool
  default     = false
}

variable "scram_username" {
  description = "SCRAM username (if SCRAM auth enabled)"
  type        = string
  default     = "kafka-admin"
  sensitive   = true
}

variable "scram_password" {
  description = "SCRAM password (if SCRAM auth enabled)"
  type        = string
  default     = ""
  sensitive   = true
}

variable "auto_create_topics" {
  description = "Allow automatic topic creation"
  type        = bool
  default     = false
}

variable "default_replication_factor" {
  description = "Default replication factor for topics"
  type        = number
  default     = 3
}

variable "min_insync_replicas" {
  description = "Minimum in-sync replicas"
  type        = number
  default     = 2
}

variable "default_partitions" {
  description = "Default number of partitions for topics"
  type        = number
  default     = 6
}

variable "log_retention_hours" {
  description = "Kafka log retention in hours"
  type        = number
  default     = 168 # 7 days
}

variable "log_retention_days" {
  description = "CloudWatch log retention in days"
  type        = number
  default     = 30
}

variable "enable_cloudwatch_logs" {
  description = "Enable CloudWatch logging for MSK"
  type        = bool
  default     = true
}

variable "enable_s3_logs" {
  description = "Enable S3 logging for MSK"
  type        = bool
  default     = false
}

variable "enable_prometheus_jmx" {
  description = "Enable Prometheus JMX exporter"
  type        = bool
  default     = true
}

variable "enable_prometheus_node" {
  description = "Enable Prometheus Node exporter"
  type        = bool
  default     = true
}

variable "enable_public_access" {
  description = "Enable public access to MSK"
  type        = bool
  default     = false
}

variable "tags" {
  description = "Common tags for all resources"
  type        = map(string)
  default     = {}
}
