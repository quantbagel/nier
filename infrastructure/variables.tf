# Nier Factory Floor Analytics Platform - Variables
# Root module input variables

# General Configuration
variable "project_name" {
  description = "Name of the project for resource naming"
  type        = string
  default     = "nier"
}

variable "environment" {
  description = "Environment name (dev, staging, prod)"
  type        = string
}

variable "aws_region" {
  description = "AWS region for all resources"
  type        = string
  default     = "us-west-2"
}

variable "cost_center" {
  description = "Cost center for billing purposes"
  type        = string
  default     = "engineering"
}

# VPC Configuration
variable "vpc_cidr" {
  description = "CIDR block for VPC"
  type        = string
  default     = "10.0.0.0/16"
}

variable "az_count" {
  description = "Number of availability zones to use"
  type        = number
  default     = 3
}

variable "enable_nat_gateway" {
  description = "Enable NAT Gateway for private subnets"
  type        = bool
  default     = true
}

variable "single_nat_gateway" {
  description = "Use single NAT Gateway (cost savings for non-prod)"
  type        = bool
  default     = false
}

variable "enable_flow_logs" {
  description = "Enable VPC Flow Logs for security monitoring"
  type        = bool
  default     = true
}

variable "flow_log_retention_days" {
  description = "Retention period for VPC Flow Logs"
  type        = number
  default     = 30
}

variable "enable_vpc_endpoints" {
  description = "Enable VPC endpoints for AWS services"
  type        = bool
  default     = true
}

# EKS Configuration
variable "kubernetes_version" {
  description = "Kubernetes version for EKS cluster"
  type        = string
  default     = "1.29"
}

variable "eks_enable_public_access" {
  description = "Enable public access to EKS API endpoint"
  type        = bool
  default     = false
}

variable "eks_public_access_cidrs" {
  description = "CIDR blocks for public access to EKS API"
  type        = list(string)
  default     = ["0.0.0.0/0"]
}

variable "eks_cluster_log_types" {
  description = "EKS cluster log types to enable"
  type        = list(string)
  default     = ["api", "audit", "authenticator", "controllerManager", "scheduler"]
}

variable "eks_log_retention_days" {
  description = "CloudWatch log retention for EKS"
  type        = number
  default     = 30
}

variable "ingest_worker_instance_types" {
  description = "Instance types for ingest worker nodes"
  type        = list(string)
  default     = ["m6i.xlarge", "m6i.2xlarge"]
}

variable "ingest_worker_desired_size" {
  description = "Desired number of ingest worker nodes"
  type        = number
  default     = 3
}

variable "ingest_worker_min_size" {
  description = "Minimum number of ingest worker nodes"
  type        = number
  default     = 2
}

variable "ingest_worker_max_size" {
  description = "Maximum number of ingest worker nodes"
  type        = number
  default     = 10
}

variable "general_instance_types" {
  description = "Instance types for general purpose nodes"
  type        = list(string)
  default     = ["m6i.large", "m6i.xlarge"]
}

variable "general_desired_size" {
  description = "Desired number of general purpose nodes"
  type        = number
  default     = 2
}

variable "general_min_size" {
  description = "Minimum number of general purpose nodes"
  type        = number
  default     = 1
}

variable "general_max_size" {
  description = "Maximum number of general purpose nodes"
  type        = number
  default     = 5
}

variable "eks_node_disk_size" {
  description = "Disk size for EKS worker nodes in GB"
  type        = number
  default     = 100
}

variable "eks_use_spot_instances" {
  description = "Use spot instances for EKS nodes"
  type        = bool
  default     = false
}

# MSK Configuration
variable "kafka_version" {
  description = "Apache Kafka version"
  type        = string
  default     = "3.5.1"
}

variable "msk_broker_count" {
  description = "Number of Kafka broker nodes"
  type        = number
  default     = 3
}

variable "msk_broker_instance_type" {
  description = "Instance type for Kafka brokers"
  type        = string
  default     = "kafka.m5.large"
}

variable "msk_broker_volume_size" {
  description = "EBS volume size for brokers in GB"
  type        = number
  default     = 500
}

variable "msk_enable_provisioned_throughput" {
  description = "Enable provisioned throughput for EBS"
  type        = bool
  default     = false
}

variable "msk_provisioned_throughput" {
  description = "Provisioned throughput in MiB/s"
  type        = number
  default     = 250
}

variable "msk_encryption_in_transit" {
  description = "Encryption setting for data in transit"
  type        = string
  default     = "TLS"
}

variable "msk_enable_iam_auth" {
  description = "Enable IAM authentication for MSK"
  type        = bool
  default     = true
}

variable "msk_enable_scram_auth" {
  description = "Enable SCRAM authentication for MSK"
  type        = bool
  default     = false
}

variable "msk_enable_unauthenticated" {
  description = "Enable unauthenticated access to MSK"
  type        = bool
  default     = false
}

variable "msk_auto_create_topics" {
  description = "Allow automatic topic creation"
  type        = bool
  default     = false
}

variable "msk_default_replication_factor" {
  description = "Default replication factor"
  type        = number
  default     = 3
}

variable "msk_min_insync_replicas" {
  description = "Minimum in-sync replicas"
  type        = number
  default     = 2
}

variable "msk_default_partitions" {
  description = "Default number of partitions"
  type        = number
  default     = 6
}

variable "msk_log_retention_hours" {
  description = "Kafka log retention in hours"
  type        = number
  default     = 168
}

variable "msk_cloudwatch_log_retention_days" {
  description = "CloudWatch log retention for MSK"
  type        = number
  default     = 30
}

variable "msk_enable_cloudwatch_logs" {
  description = "Enable CloudWatch logging for MSK"
  type        = bool
  default     = true
}

variable "msk_enable_s3_logs" {
  description = "Enable S3 logging for MSK"
  type        = bool
  default     = false
}

variable "msk_enable_prometheus_jmx" {
  description = "Enable Prometheus JMX exporter"
  type        = bool
  default     = true
}

variable "msk_enable_prometheus_node" {
  description = "Enable Prometheus Node exporter"
  type        = bool
  default     = true
}

# RDS Configuration
variable "rds_engine_version" {
  description = "PostgreSQL engine version"
  type        = string
  default     = "15.4"
}

variable "rds_engine_version_major" {
  description = "PostgreSQL major version"
  type        = string
  default     = "15"
}

variable "rds_instance_class" {
  description = "RDS instance class"
  type        = string
  default     = "db.t3.medium"
}

variable "rds_allocated_storage" {
  description = "Allocated storage in GB"
  type        = number
  default     = 100
}

variable "rds_max_allocated_storage" {
  description = "Maximum allocated storage for autoscaling"
  type        = number
  default     = 500
}

variable "rds_storage_type" {
  description = "Storage type (gp2, gp3, io1)"
  type        = string
  default     = "gp3"
}

variable "rds_database_name" {
  description = "Name of the database to create"
  type        = string
  default     = "nier"
}

variable "rds_master_username" {
  description = "Master username"
  type        = string
  default     = "nieradmin"
}

variable "rds_port" {
  description = "Database port"
  type        = number
  default     = 5432
}

variable "rds_max_connections" {
  description = "Maximum connections"
  type        = number
  default     = 200
}

variable "rds_force_ssl" {
  description = "Force SSL connections"
  type        = bool
  default     = true
}

variable "rds_backup_retention_period" {
  description = "Backup retention period in days"
  type        = number
  default     = 7
}

variable "rds_backup_window" {
  description = "Preferred backup window"
  type        = string
  default     = "03:00-04:00"
}

variable "rds_maintenance_window" {
  description = "Preferred maintenance window"
  type        = string
  default     = "sun:04:00-sun:05:00"
}

variable "rds_monitoring_interval" {
  description = "Enhanced monitoring interval (0 to disable)"
  type        = number
  default     = 60
}

variable "rds_performance_insights_enabled" {
  description = "Enable Performance Insights"
  type        = bool
  default     = true
}

variable "rds_performance_insights_retention" {
  description = "Performance Insights retention in days"
  type        = number
  default     = 7
}

variable "rds_multi_az" {
  description = "Enable Multi-AZ deployment"
  type        = bool
  default     = false
}

variable "rds_deletion_protection" {
  description = "Enable deletion protection"
  type        = bool
  default     = true
}

variable "rds_skip_final_snapshot" {
  description = "Skip final snapshot on deletion"
  type        = bool
  default     = false
}

variable "rds_auto_minor_version_upgrade" {
  description = "Enable auto minor version upgrade"
  type        = bool
  default     = true
}

variable "rds_enable_iam_auth" {
  description = "Enable IAM database authentication"
  type        = bool
  default     = true
}

variable "rds_create_read_replica" {
  description = "Create a read replica"
  type        = bool
  default     = false
}

variable "rds_replica_instance_class" {
  description = "Instance class for read replica"
  type        = string
  default     = "db.t3.medium"
}

# S3 Configuration
variable "s3_enable_versioning" {
  description = "Enable versioning for raw frames bucket"
  type        = bool
  default     = false
}

variable "raw_frames_ia_transition_days" {
  description = "Days before transitioning raw frames to Standard-IA"
  type        = number
  default     = 30
}

variable "raw_frames_glacier_transition_days" {
  description = "Days before transitioning raw frames to Glacier"
  type        = number
  default     = 90
}

variable "raw_frames_expiration_days" {
  description = "Days before expiring raw frames"
  type        = number
  default     = 365
}

variable "analytics_ia_transition_days" {
  description = "Days before transitioning analytics to Standard-IA"
  type        = number
  default     = 90
}

variable "logs_expiration_days" {
  description = "Days before expiring logs"
  type        = number
  default     = 365
}

variable "enable_s3_access_logging" {
  description = "Enable S3 access logging"
  type        = bool
  default     = true
}

variable "enable_s3_cors" {
  description = "Enable CORS for analytics bucket"
  type        = bool
  default     = false
}

variable "s3_cors_allowed_origins" {
  description = "Allowed origins for CORS"
  type        = list(string)
  default     = ["*"]
}

variable "enable_intelligent_tiering" {
  description = "Enable Intelligent Tiering for raw frames"
  type        = bool
  default     = false
}

# EC2-GPU Configuration
variable "gpu_instance_type" {
  description = "EC2 instance type for GPU instances"
  type        = string
  default     = "g4dn.xlarge"
}

variable "gpu_ami_id" {
  description = "AMI ID for GPU instances (uses Deep Learning AMI if null)"
  type        = string
  default     = null
}

variable "gpu_root_volume_size" {
  description = "Root volume size in GB"
  type        = number
  default     = 100
}

variable "gpu_data_volume_size" {
  description = "Data volume size in GB"
  type        = number
  default     = 200
}

variable "gpu_desired_capacity" {
  description = "Desired number of GPU instances"
  type        = number
  default     = 2
}

variable "gpu_min_size" {
  description = "Minimum number of GPU instances"
  type        = number
  default     = 1
}

variable "gpu_max_size" {
  description = "Maximum number of GPU instances"
  type        = number
  default     = 10
}

variable "gpu_enable_autoscaling" {
  description = "Enable auto scaling for GPU instances"
  type        = bool
  default     = true
}

variable "gpu_target_utilization" {
  description = "Target GPU utilization for auto scaling"
  type        = number
  default     = 70
}

variable "gpu_enable_scheduled_scaling" {
  description = "Enable scheduled scaling for GPU instances"
  type        = bool
  default     = false
}

variable "gpu_scheduled_max_size" {
  description = "Maximum size during peak hours"
  type        = number
  default     = 5
}

variable "gpu_scale_up_cron" {
  description = "Cron expression for scaling up (UTC)"
  type        = string
  default     = "0 8 * * MON-FRI"
}

variable "gpu_scale_down_cron" {
  description = "Cron expression for scaling down (UTC)"
  type        = string
  default     = "0 18 * * MON-FRI"
}

variable "gpu_inference_port" {
  description = "Port for inference API"
  type        = number
  default     = 8080
}

variable "gpu_health_check_port" {
  description = "Port for health checks"
  type        = number
  default     = 8081
}

variable "gpu_log_retention_days" {
  description = "CloudWatch log retention for GPU instances"
  type        = number
  default     = 30
}

variable "gpu_latency_threshold_ms" {
  description = "Inference latency threshold in milliseconds"
  type        = number
  default     = 500
}

variable "gpu_create_load_balancer" {
  description = "Create Application Load Balancer for GPU instances"
  type        = bool
  default     = true
}

# Monitoring Configuration
variable "enable_cloudwatch_alarms" {
  description = "Enable CloudWatch alarms"
  type        = bool
  default     = true
}

variable "alarm_sns_topic_arns" {
  description = "SNS topic ARNs for alarm notifications"
  type        = list(string)
  default     = []
}
