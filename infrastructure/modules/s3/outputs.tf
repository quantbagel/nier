# S3 Module Outputs

output "raw_frames_bucket_id" {
  description = "ID of the raw frames bucket"
  value       = aws_s3_bucket.raw_frames.id
}

output "raw_frames_bucket_arn" {
  description = "ARN of the raw frames bucket"
  value       = aws_s3_bucket.raw_frames.arn
}

output "raw_frames_bucket_domain_name" {
  description = "Domain name of the raw frames bucket"
  value       = aws_s3_bucket.raw_frames.bucket_domain_name
}

output "analytics_bucket_id" {
  description = "ID of the analytics bucket"
  value       = aws_s3_bucket.analytics.id
}

output "analytics_bucket_arn" {
  description = "ARN of the analytics bucket"
  value       = aws_s3_bucket.analytics.arn
}

output "analytics_bucket_domain_name" {
  description = "Domain name of the analytics bucket"
  value       = aws_s3_bucket.analytics.bucket_domain_name
}

output "models_bucket_id" {
  description = "ID of the models bucket"
  value       = aws_s3_bucket.models.id
}

output "models_bucket_arn" {
  description = "ARN of the models bucket"
  value       = aws_s3_bucket.models.arn
}

output "models_bucket_domain_name" {
  description = "Domain name of the models bucket"
  value       = aws_s3_bucket.models.bucket_domain_name
}

output "logs_bucket_id" {
  description = "ID of the logs bucket"
  value       = aws_s3_bucket.logs.id
}

output "logs_bucket_arn" {
  description = "ARN of the logs bucket"
  value       = aws_s3_bucket.logs.arn
}

output "all_bucket_arns" {
  description = "List of all bucket ARNs"
  value = [
    aws_s3_bucket.raw_frames.arn,
    "${aws_s3_bucket.raw_frames.arn}/*",
    aws_s3_bucket.analytics.arn,
    "${aws_s3_bucket.analytics.arn}/*",
    aws_s3_bucket.models.arn,
    "${aws_s3_bucket.models.arn}/*",
    aws_s3_bucket.logs.arn,
    "${aws_s3_bucket.logs.arn}/*"
  ]
}

output "kms_key_arn" {
  description = "ARN of the KMS key used for encryption"
  value       = var.kms_key_arn != null ? var.kms_key_arn : aws_kms_key.s3[0].arn
}
