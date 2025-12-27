# Nier Factory Floor Analytics Platform - Outputs
# Root module outputs

# VPC Outputs
output "vpc_id" {
  description = "ID of the VPC"
  value       = module.vpc.vpc_id
}

output "vpc_cidr" {
  description = "CIDR block of the VPC"
  value       = module.vpc.vpc_cidr
}

output "public_subnet_ids" {
  description = "List of public subnet IDs"
  value       = module.vpc.public_subnet_ids
}

output "private_subnet_ids" {
  description = "List of private subnet IDs"
  value       = module.vpc.private_subnet_ids
}

output "database_subnet_ids" {
  description = "List of database subnet IDs"
  value       = module.vpc.database_subnet_ids
}

output "availability_zones" {
  description = "List of availability zones used"
  value       = module.vpc.availability_zones
}

# EKS Outputs
output "eks_cluster_name" {
  description = "EKS cluster name"
  value       = module.eks.cluster_name
}

output "eks_cluster_endpoint" {
  description = "EKS cluster API endpoint"
  value       = module.eks.cluster_endpoint
}

output "eks_cluster_certificate_authority_data" {
  description = "Base64 encoded certificate data for cluster auth"
  value       = module.eks.cluster_certificate_authority_data
  sensitive   = true
}

output "eks_cluster_oidc_issuer_url" {
  description = "OIDC issuer URL for the EKS cluster"
  value       = module.eks.cluster_oidc_issuer_url
}

output "eks_oidc_provider_arn" {
  description = "ARN of the OIDC provider"
  value       = module.eks.oidc_provider_arn
}

output "eks_node_security_group_id" {
  description = "Security group ID for EKS worker nodes"
  value       = module.eks.node_security_group_id
}

output "eks_cluster_autoscaler_role_arn" {
  description = "IAM role ARN for cluster autoscaler"
  value       = module.eks.cluster_autoscaler_role_arn
}

# MSK Outputs
output "msk_cluster_arn" {
  description = "ARN of the MSK cluster"
  value       = module.msk.cluster_arn
}

output "msk_bootstrap_brokers_tls" {
  description = "TLS bootstrap brokers for MSK"
  value       = module.msk.bootstrap_brokers_tls
}

output "msk_bootstrap_brokers_sasl_iam" {
  description = "SASL IAM bootstrap brokers for MSK"
  value       = module.msk.bootstrap_brokers_sasl_iam
}

output "msk_zookeeper_connect_string" {
  description = "Zookeeper connection string"
  value       = module.msk.zookeeper_connect_string
}

output "msk_security_group_id" {
  description = "Security group ID for MSK"
  value       = module.msk.security_group_id
}

# RDS Outputs
output "rds_endpoint" {
  description = "RDS instance endpoint"
  value       = module.rds.db_instance_endpoint
}

output "rds_address" {
  description = "RDS instance address"
  value       = module.rds.db_instance_address
}

output "rds_port" {
  description = "RDS instance port"
  value       = module.rds.db_instance_port
}

output "rds_database_name" {
  description = "Database name"
  value       = module.rds.db_instance_name
}

output "rds_credentials_secret_arn" {
  description = "ARN of the Secrets Manager secret containing DB credentials"
  value       = module.rds.db_credentials_secret_arn
}

output "rds_security_group_id" {
  description = "Security group ID for RDS"
  value       = module.rds.db_security_group_id
}

output "rds_replica_endpoint" {
  description = "Read replica endpoint (if created)"
  value       = module.rds.replica_instance_endpoint
}

# S3 Outputs
output "s3_raw_frames_bucket_id" {
  description = "ID of the raw frames bucket"
  value       = module.s3.raw_frames_bucket_id
}

output "s3_raw_frames_bucket_arn" {
  description = "ARN of the raw frames bucket"
  value       = module.s3.raw_frames_bucket_arn
}

output "s3_analytics_bucket_id" {
  description = "ID of the analytics bucket"
  value       = module.s3.analytics_bucket_id
}

output "s3_analytics_bucket_arn" {
  description = "ARN of the analytics bucket"
  value       = module.s3.analytics_bucket_arn
}

output "s3_models_bucket_id" {
  description = "ID of the models bucket"
  value       = module.s3.models_bucket_id
}

output "s3_models_bucket_arn" {
  description = "ARN of the models bucket"
  value       = module.s3.models_bucket_arn
}

output "s3_logs_bucket_id" {
  description = "ID of the logs bucket"
  value       = module.s3.logs_bucket_id
}

# EC2-GPU Outputs
output "gpu_autoscaling_group_name" {
  description = "Name of the GPU Auto Scaling Group"
  value       = module.ec2_gpu.autoscaling_group_name
}

output "gpu_load_balancer_dns_name" {
  description = "DNS name of the GPU Application Load Balancer"
  value       = module.ec2_gpu.load_balancer_dns_name
}

output "gpu_security_group_id" {
  description = "Security group ID for GPU instances"
  value       = module.ec2_gpu.security_group_id
}

output "gpu_iam_role_arn" {
  description = "IAM role ARN for GPU instances"
  value       = module.ec2_gpu.iam_role_arn
}

output "gpu_cloudwatch_log_group_name" {
  description = "CloudWatch log group name for GPU instances"
  value       = module.ec2_gpu.cloudwatch_log_group_name
}

# Kubeconfig helper
output "configure_kubectl" {
  description = "Command to configure kubectl"
  value       = "aws eks update-kubeconfig --region ${var.aws_region} --name ${module.eks.cluster_name}"
}
