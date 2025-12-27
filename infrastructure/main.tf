# Nier Factory Floor Analytics Platform - Infrastructure
# Root Terraform module

terraform {
  required_version = ">= 1.5.0"

  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 5.0"
    }
    random = {
      source  = "hashicorp/random"
      version = "~> 3.5"
    }
    tls = {
      source  = "hashicorp/tls"
      version = "~> 4.0"
    }
  }

  # Backend configuration - uncomment and configure for your environment
  # backend "s3" {
  #   bucket         = "nier-terraform-state"
  #   key            = "infrastructure/terraform.tfstate"
  #   region         = "us-west-2"
  #   encrypt        = true
  #   dynamodb_table = "nier-terraform-locks"
  # }
}

provider "aws" {
  region = var.aws_region

  default_tags {
    tags = {
      Project     = var.project_name
      Environment = var.environment
      ManagedBy   = "Terraform"
      Application = "nier-factory-analytics"
    }
  }
}

locals {
  common_tags = {
    Project     = var.project_name
    Environment = var.environment
    ManagedBy   = "Terraform"
    CostCenter  = var.cost_center
  }
}

# VPC Module
module "vpc" {
  source = "./modules/vpc"

  project_name           = var.project_name
  environment            = var.environment
  aws_region             = var.aws_region
  vpc_cidr               = var.vpc_cidr
  az_count               = var.az_count
  enable_nat_gateway     = var.enable_nat_gateway
  single_nat_gateway     = var.single_nat_gateway
  enable_flow_logs       = var.enable_flow_logs
  flow_log_retention_days = var.flow_log_retention_days
  enable_vpc_endpoints   = var.enable_vpc_endpoints
  tags                   = local.common_tags
}

# S3 Module (created early as other modules may reference bucket ARNs)
module "s3" {
  source = "./modules/s3"

  project_name                    = var.project_name
  environment                     = var.environment
  enable_versioning               = var.s3_enable_versioning
  raw_frames_ia_transition_days   = var.raw_frames_ia_transition_days
  raw_frames_glacier_transition_days = var.raw_frames_glacier_transition_days
  raw_frames_expiration_days      = var.raw_frames_expiration_days
  analytics_ia_transition_days    = var.analytics_ia_transition_days
  logs_expiration_days            = var.logs_expiration_days
  enable_access_logging           = var.enable_s3_access_logging
  enable_cors                     = var.enable_s3_cors
  cors_allowed_origins            = var.s3_cors_allowed_origins
  enable_intelligent_tiering      = var.enable_intelligent_tiering
  tags                            = local.common_tags
}

# EKS Module
module "eks" {
  source = "./modules/eks"

  project_name                = var.project_name
  environment                 = var.environment
  vpc_id                      = module.vpc.vpc_id
  private_subnet_ids          = module.vpc.private_subnet_ids
  kubernetes_version          = var.kubernetes_version
  enable_public_access        = var.eks_enable_public_access
  public_access_cidrs         = var.eks_public_access_cidrs
  enabled_cluster_log_types   = var.eks_cluster_log_types
  log_retention_days          = var.eks_log_retention_days
  ingest_worker_instance_types = var.ingest_worker_instance_types
  ingest_worker_desired_size  = var.ingest_worker_desired_size
  ingest_worker_min_size      = var.ingest_worker_min_size
  ingest_worker_max_size      = var.ingest_worker_max_size
  general_instance_types      = var.general_instance_types
  general_desired_size        = var.general_desired_size
  general_min_size            = var.general_min_size
  general_max_size            = var.general_max_size
  node_disk_size              = var.eks_node_disk_size
  use_spot_instances          = var.eks_use_spot_instances
  s3_bucket_arns              = module.s3.all_bucket_arns
  tags                        = local.common_tags
}

# MSK Module
module "msk" {
  source = "./modules/msk"

  project_name              = var.project_name
  environment               = var.environment
  vpc_id                    = module.vpc.vpc_id
  vpc_cidr                  = module.vpc.vpc_cidr
  private_subnet_ids        = module.vpc.private_subnet_ids
  kafka_version             = var.kafka_version
  broker_count              = var.msk_broker_count
  broker_instance_type      = var.msk_broker_instance_type
  broker_volume_size        = var.msk_broker_volume_size
  enable_provisioned_throughput = var.msk_enable_provisioned_throughput
  provisioned_throughput    = var.msk_provisioned_throughput
  encryption_in_transit     = var.msk_encryption_in_transit
  enable_iam_auth           = var.msk_enable_iam_auth
  enable_scram_auth         = var.msk_enable_scram_auth
  enable_unauthenticated    = var.msk_enable_unauthenticated
  auto_create_topics        = var.msk_auto_create_topics
  default_replication_factor = var.msk_default_replication_factor
  min_insync_replicas       = var.msk_min_insync_replicas
  default_partitions        = var.msk_default_partitions
  log_retention_hours       = var.msk_log_retention_hours
  log_retention_days        = var.msk_cloudwatch_log_retention_days
  enable_cloudwatch_logs    = var.msk_enable_cloudwatch_logs
  enable_s3_logs            = var.msk_enable_s3_logs
  enable_prometheus_jmx     = var.msk_enable_prometheus_jmx
  enable_prometheus_node    = var.msk_enable_prometheus_node
  enable_public_access      = false
  tags                      = local.common_tags
}

# RDS Module
module "rds" {
  source = "./modules/rds"

  project_name               = var.project_name
  environment                = var.environment
  vpc_id                     = module.vpc.vpc_id
  vpc_cidr                   = module.vpc.vpc_cidr
  db_subnet_group_name       = module.vpc.database_subnet_group_name
  allowed_security_group_ids = [module.eks.node_security_group_id]
  engine_version             = var.rds_engine_version
  engine_version_major       = var.rds_engine_version_major
  instance_class             = var.rds_instance_class
  allocated_storage          = var.rds_allocated_storage
  max_allocated_storage      = var.rds_max_allocated_storage
  storage_type               = var.rds_storage_type
  database_name              = var.rds_database_name
  master_username            = var.rds_master_username
  port                       = var.rds_port
  max_connections            = var.rds_max_connections
  force_ssl                  = var.rds_force_ssl
  backup_retention_period    = var.rds_backup_retention_period
  backup_window              = var.rds_backup_window
  maintenance_window         = var.rds_maintenance_window
  monitoring_interval        = var.rds_monitoring_interval
  performance_insights_enabled = var.rds_performance_insights_enabled
  performance_insights_retention = var.rds_performance_insights_retention
  multi_az                   = var.rds_multi_az
  deletion_protection        = var.rds_deletion_protection
  skip_final_snapshot        = var.rds_skip_final_snapshot
  auto_minor_version_upgrade = var.rds_auto_minor_version_upgrade
  enable_iam_auth            = var.rds_enable_iam_auth
  create_read_replica        = var.rds_create_read_replica
  replica_instance_class     = var.rds_replica_instance_class
  enable_cloudwatch_alarms   = var.enable_cloudwatch_alarms
  alarm_actions              = var.alarm_sns_topic_arns
  ok_actions                 = var.alarm_sns_topic_arns
  tags                       = local.common_tags
}

# EC2-GPU Module
module "ec2_gpu" {
  source = "./modules/ec2-gpu"

  project_name               = var.project_name
  environment                = var.environment
  vpc_id                     = module.vpc.vpc_id
  vpc_cidr                   = module.vpc.vpc_cidr
  private_subnet_ids         = module.vpc.private_subnet_ids
  allowed_security_group_ids = [module.eks.node_security_group_id]
  instance_type              = var.gpu_instance_type
  ami_id                     = var.gpu_ami_id
  root_volume_size           = var.gpu_root_volume_size
  data_volume_size           = var.gpu_data_volume_size
  ebs_kms_key_arn            = module.s3.kms_key_arn
  desired_capacity           = var.gpu_desired_capacity
  min_size                   = var.gpu_min_size
  max_size                   = var.gpu_max_size
  enable_autoscaling         = var.gpu_enable_autoscaling
  target_gpu_utilization     = var.gpu_target_utilization
  enable_scheduled_scaling   = var.gpu_enable_scheduled_scaling
  scheduled_max_size         = var.gpu_scheduled_max_size
  scale_up_cron              = var.gpu_scale_up_cron
  scale_down_cron            = var.gpu_scale_down_cron
  inference_port             = var.gpu_inference_port
  health_check_port          = var.gpu_health_check_port
  s3_bucket_arns             = module.s3.all_bucket_arns
  secrets_arns               = [module.rds.db_credentials_secret_arn]
  kms_key_arns               = [module.s3.kms_key_arn, module.msk.kms_key_arn]
  models_bucket              = module.s3.models_bucket_id
  log_retention_days         = var.gpu_log_retention_days
  enable_alarms              = var.enable_cloudwatch_alarms
  latency_threshold_ms       = var.gpu_latency_threshold_ms
  alarm_actions              = var.alarm_sns_topic_arns
  ok_actions                 = var.alarm_sns_topic_arns
  create_load_balancer       = var.gpu_create_load_balancer
  tags                       = local.common_tags
}
