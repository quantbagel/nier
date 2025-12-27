# Nier Factory Floor Analytics Platform - Development Environment
# terraform.tfvars for dev environment

# General Configuration
project_name = "nier"
environment  = "dev"
aws_region   = "us-west-2"
cost_center  = "engineering-dev"

# VPC Configuration - Cost optimized for dev
vpc_cidr               = "10.0.0.0/16"
az_count               = 2  # Reduced for cost savings
enable_nat_gateway     = true
single_nat_gateway     = true  # Single NAT for cost savings
enable_flow_logs       = true
flow_log_retention_days = 7    # Shorter retention for dev
enable_vpc_endpoints   = true

# EKS Configuration - Smaller cluster for dev
kubernetes_version          = "1.29"
eks_enable_public_access    = true  # Enable for easier developer access
eks_public_access_cidrs     = ["0.0.0.0/0"]  # Restrict in real deployment
eks_cluster_log_types       = ["api", "audit"]
eks_log_retention_days      = 7

# Ingest workers - minimal for dev
ingest_worker_instance_types = ["m6i.large"]
ingest_worker_desired_size   = 1
ingest_worker_min_size       = 1
ingest_worker_max_size       = 3

# General nodes - minimal for dev
general_instance_types = ["t3.medium"]
general_desired_size   = 1
general_min_size       = 1
general_max_size       = 2

eks_node_disk_size     = 50
eks_use_spot_instances = true  # Use spot for cost savings

# MSK Configuration - Smaller cluster for dev
kafka_version                     = "3.5.1"
msk_broker_count                  = 2  # Minimum required
msk_broker_instance_type          = "kafka.t3.small"
msk_broker_volume_size            = 100
msk_enable_provisioned_throughput = false
msk_encryption_in_transit         = "TLS"
msk_enable_iam_auth               = true
msk_enable_scram_auth             = false
msk_enable_unauthenticated        = false
msk_auto_create_topics            = true  # Allow for easier development
msk_default_replication_factor    = 2
msk_min_insync_replicas           = 1
msk_default_partitions            = 3
msk_log_retention_hours           = 24  # Short retention for dev
msk_cloudwatch_log_retention_days = 7
msk_enable_cloudwatch_logs        = true
msk_enable_s3_logs                = false
msk_enable_prometheus_jmx         = false
msk_enable_prometheus_node        = false

# RDS Configuration - Smaller instance for dev
rds_engine_version               = "15.4"
rds_engine_version_major         = "15"
rds_instance_class               = "db.t3.small"
rds_allocated_storage            = 20
rds_max_allocated_storage        = 100
rds_storage_type                 = "gp3"
rds_database_name                = "nier"
rds_master_username              = "nieradmin"
rds_port                         = 5432
rds_max_connections              = 100
rds_force_ssl                    = true
rds_backup_retention_period      = 3  # Shorter for dev
rds_backup_window                = "03:00-04:00"
rds_maintenance_window           = "sun:04:00-sun:05:00"
rds_monitoring_interval          = 0   # Disable enhanced monitoring for cost
rds_performance_insights_enabled = false
rds_multi_az                     = false  # Single AZ for dev
rds_deletion_protection          = false  # Allow deletion in dev
rds_skip_final_snapshot          = true   # Skip snapshot in dev
rds_auto_minor_version_upgrade   = true
rds_enable_iam_auth              = true
rds_create_read_replica          = false

# S3 Configuration
s3_enable_versioning               = false  # Disable for dev
raw_frames_ia_transition_days      = 7
raw_frames_glacier_transition_days = 30
raw_frames_expiration_days         = 90
analytics_ia_transition_days       = 30
logs_expiration_days               = 30
enable_s3_access_logging           = false  # Disable for cost
enable_s3_cors                     = true
s3_cors_allowed_origins            = ["*"]
enable_intelligent_tiering         = false

# EC2-GPU Configuration - Minimal for dev
gpu_instance_type          = "g4dn.xlarge"
gpu_ami_id                 = null
gpu_root_volume_size       = 50
gpu_data_volume_size       = 100
gpu_desired_capacity       = 1
gpu_min_size               = 0  # Allow scaling to zero
gpu_max_size               = 2
gpu_enable_autoscaling     = true
gpu_target_utilization     = 70
gpu_enable_scheduled_scaling = false
gpu_inference_port         = 8080
gpu_health_check_port      = 8081
gpu_log_retention_days     = 7
gpu_latency_threshold_ms   = 1000  # More lenient for dev
gpu_create_load_balancer   = true

# Monitoring Configuration
enable_cloudwatch_alarms = false  # Disable alarms in dev
alarm_sns_topic_arns     = []
