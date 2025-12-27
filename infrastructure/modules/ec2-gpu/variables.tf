# EC2-GPU Module Variables

variable "project_name" {
  description = "Name of the project for resource naming"
  type        = string
}

variable "environment" {
  description = "Environment name (dev, staging, prod)"
  type        = string
}

variable "vpc_id" {
  description = "VPC ID for the GPU instances"
  type        = string
}

variable "vpc_cidr" {
  description = "VPC CIDR block for security group rules"
  type        = string
}

variable "private_subnet_ids" {
  description = "Private subnet IDs for GPU instances"
  type        = list(string)
}

variable "allowed_security_group_ids" {
  description = "Security group IDs allowed to access GPU instances"
  type        = list(string)
  default     = []
}

# Instance configuration
variable "instance_type" {
  description = "EC2 instance type for GPU instances"
  type        = string
  default     = "g4dn.xlarge"
}

variable "ami_id" {
  description = "AMI ID for GPU instances (uses Deep Learning AMI if null)"
  type        = string
  default     = null
}

variable "root_volume_size" {
  description = "Root volume size in GB"
  type        = number
  default     = 100
}

variable "data_volume_size" {
  description = "Data volume size in GB"
  type        = number
  default     = 200
}

variable "ebs_kms_key_arn" {
  description = "KMS key ARN for EBS encryption"
  type        = string
  default     = null
}

# Auto Scaling configuration
variable "desired_capacity" {
  description = "Desired number of GPU instances"
  type        = number
  default     = 2
}

variable "min_size" {
  description = "Minimum number of GPU instances"
  type        = number
  default     = 1
}

variable "max_size" {
  description = "Maximum number of GPU instances"
  type        = number
  default     = 10
}

variable "enable_autoscaling" {
  description = "Enable auto scaling based on GPU utilization"
  type        = bool
  default     = true
}

variable "target_gpu_utilization" {
  description = "Target GPU utilization for auto scaling"
  type        = number
  default     = 70
}

# Scheduled scaling
variable "enable_scheduled_scaling" {
  description = "Enable scheduled scaling"
  type        = bool
  default     = false
}

variable "scheduled_max_size" {
  description = "Maximum size during peak hours"
  type        = number
  default     = 5
}

variable "scale_up_cron" {
  description = "Cron expression for scaling up (UTC)"
  type        = string
  default     = "0 8 * * MON-FRI"
}

variable "scale_down_cron" {
  description = "Cron expression for scaling down (UTC)"
  type        = string
  default     = "0 18 * * MON-FRI"
}

# Network configuration
variable "inference_port" {
  description = "Port for inference API"
  type        = number
  default     = 8080
}

variable "health_check_port" {
  description = "Port for health checks"
  type        = number
  default     = 8081
}

# IAM configuration
variable "s3_bucket_arns" {
  description = "S3 bucket ARNs for GPU instance access"
  type        = list(string)
  default     = ["arn:aws:s3:::*"]
}

variable "secrets_arns" {
  description = "Secrets Manager ARNs for GPU instance access"
  type        = list(string)
  default     = ["*"]
}

variable "kms_key_arns" {
  description = "KMS key ARNs for GPU instance access"
  type        = list(string)
  default     = ["*"]
}

variable "models_bucket" {
  description = "S3 bucket name for ML models"
  type        = string
}

# Logging
variable "log_retention_days" {
  description = "CloudWatch log retention in days"
  type        = number
  default     = 30
}

# Alarms
variable "enable_alarms" {
  description = "Enable CloudWatch alarms"
  type        = bool
  default     = true
}

variable "latency_threshold_ms" {
  description = "Inference latency threshold in milliseconds"
  type        = number
  default     = 500
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

# Load Balancer
variable "create_load_balancer" {
  description = "Create Application Load Balancer for GPU instances"
  type        = bool
  default     = true
}

variable "tags" {
  description = "Common tags for all resources"
  type        = map(string)
  default     = {}
}
