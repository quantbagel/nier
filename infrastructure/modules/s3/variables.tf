# S3 Module Variables

variable "project_name" {
  description = "Name of the project for resource naming"
  type        = string
}

variable "environment" {
  description = "Environment name (dev, staging, prod)"
  type        = string
}

variable "kms_key_arn" {
  description = "ARN of existing KMS key (creates new if null)"
  type        = string
  default     = null
}

variable "enable_versioning" {
  description = "Enable versioning for raw frames bucket"
  type        = bool
  default     = false
}

# Lifecycle configuration for raw frames
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

# Lifecycle configuration for analytics
variable "analytics_ia_transition_days" {
  description = "Days before transitioning analytics to Standard-IA"
  type        = number
  default     = 90
}

# Lifecycle configuration for logs
variable "logs_expiration_days" {
  description = "Days before expiring logs"
  type        = number
  default     = 365
}

variable "enable_access_logging" {
  description = "Enable S3 access logging"
  type        = bool
  default     = true
}

variable "enable_cors" {
  description = "Enable CORS for analytics bucket"
  type        = bool
  default     = false
}

variable "cors_allowed_origins" {
  description = "Allowed origins for CORS"
  type        = list(string)
  default     = ["*"]
}

variable "enable_intelligent_tiering" {
  description = "Enable Intelligent Tiering for raw frames"
  type        = bool
  default     = false
}

variable "tags" {
  description = "Common tags for all resources"
  type        = map(string)
  default     = {}
}
