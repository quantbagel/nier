# EKS Module Variables

variable "project_name" {
  description = "Name of the project for resource naming"
  type        = string
}

variable "environment" {
  description = "Environment name (dev, staging, prod)"
  type        = string
}

variable "vpc_id" {
  description = "VPC ID for the EKS cluster"
  type        = string
}

variable "private_subnet_ids" {
  description = "Private subnet IDs for EKS nodes"
  type        = list(string)
}

variable "kubernetes_version" {
  description = "Kubernetes version for the EKS cluster"
  type        = string
  default     = "1.29"
}

variable "enable_public_access" {
  description = "Enable public access to the EKS API endpoint"
  type        = bool
  default     = false
}

variable "public_access_cidrs" {
  description = "CIDR blocks for public access to EKS API"
  type        = list(string)
  default     = ["0.0.0.0/0"]
}

variable "enabled_cluster_log_types" {
  description = "List of EKS cluster log types to enable"
  type        = list(string)
  default     = ["api", "audit", "authenticator", "controllerManager", "scheduler"]
}

variable "log_retention_days" {
  description = "CloudWatch log retention in days"
  type        = number
  default     = 30
}

variable "kms_key_arn" {
  description = "ARN of existing KMS key for secrets encryption (creates new if null)"
  type        = string
  default     = null
}

# Ingest Worker Node Group Configuration
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

# General Node Group Configuration
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

variable "node_disk_size" {
  description = "Disk size for worker nodes in GB"
  type        = number
  default     = 100
}

variable "use_spot_instances" {
  description = "Use spot instances for cost savings"
  type        = bool
  default     = false
}

variable "s3_bucket_arns" {
  description = "S3 bucket ARNs for node access"
  type        = list(string)
  default     = ["arn:aws:s3:::*"]
}

variable "tags" {
  description = "Common tags for all resources"
  type        = map(string)
  default     = {}
}
