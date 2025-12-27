# Nier Factory Floor Analytics Platform - Production Environment
# terraform.tfvars for prod environment

# General Configuration
project_name = "nier"
environment  = "prod"
aws_region   = "us-west-2"
cost_center  = "engineering-prod"

# VPC Configuration - Full HA for production
vpc_cidr               = "10.0.0.0/16"
az_count               = 3  # Multi-AZ for high availability
enable_nat_gateway     = true
single_nat_gateway     = false  # NAT per AZ for HA
enable_flow_logs       = true
flow_log_retention_days = 90    # Longer retention for compliance
enable_vpc_endpoints   = true

# EKS Configuration - Production-sized cluster
kubernetes_version          = "1.29"
eks_enable_public_access    = false  # Private access only
eks_public_access_cidrs     = []
eks_cluster_log_types       = ["api", "audit", "authenticator", "controllerManager", "scheduler"]
eks_log_retention_days      = 90

# Ingest workers - production capacity
ingest_worker_instance_types = ["m6i.xlarge", "m6i.2xlarge"]
ingest_worker_desired_size   = 5
ingest_worker_min_size       = 3
ingest_worker_max_size       = 20

# General nodes - production capacity
general_instance_types = ["m6i.large", "m6i.xlarge"]
general_desired_size   = 3
general_min_size       = 2
general_max_size       = 10

eks_node_disk_size     = 200
eks_use_spot_instances = false  # On-demand for stability

# MSK Configuration - Production-sized cluster
kafka_version                     = "3.5.1"
msk_broker_count                  = 6  # 2 per AZ
msk_broker_instance_type          = "kafka.m5.xlarge"
msk_broker_volume_size            = 1000
msk_enable_provisioned_throughput = true
msk_provisioned_throughput        = 500
msk_encryption_in_transit         = "TLS"
msk_enable_iam_auth               = true
msk_enable_scram_auth             = false
msk_enable_unauthenticated        = false
msk_auto_create_topics            = false  # Explicit topic management
msk_default_replication_factor    = 3
msk_min_insync_replicas           = 2
msk_default_partitions            = 12
msk_log_retention_hours           = 168  # 7 days
msk_cloudwatch_log_retention_days = 90
msk_enable_cloudwatch_logs        = true
msk_enable_s3_logs                = true
msk_enable_prometheus_jmx         = true
msk_enable_prometheus_node        = true

# RDS Configuration - Production instance
rds_engine_version               = "15.4"
rds_engine_version_major         = "15"
rds_instance_class               = "db.r6g.xlarge"
rds_allocated_storage            = 500
rds_max_allocated_storage        = 2000
rds_storage_type                 = "gp3"
rds_database_name                = "nier"
rds_master_username              = "nieradmin"
rds_port                         = 5432
rds_max_connections              = 500
rds_force_ssl                    = true
rds_backup_retention_period      = 30  # Monthly backups
rds_backup_window                = "03:00-04:00"
rds_maintenance_window           = "sun:04:00-sun:05:00"
rds_monitoring_interval          = 60   # Enhanced monitoring enabled
rds_performance_insights_enabled = true
rds_performance_insights_retention = 31 # 1 month
rds_multi_az                     = true  # Multi-AZ for HA
rds_deletion_protection          = true  # Prevent accidental deletion
rds_skip_final_snapshot          = false # Always create final snapshot
rds_auto_minor_version_upgrade   = true
rds_enable_iam_auth              = true
rds_create_read_replica          = true
rds_replica_instance_class       = "db.r6g.large"

# S3 Configuration - Full retention for production
s3_enable_versioning               = true
raw_frames_ia_transition_days      = 30
raw_frames_glacier_transition_days = 90
raw_frames_expiration_days         = 730  # 2 years
analytics_ia_transition_days       = 90
logs_expiration_days               = 365  # 1 year
enable_s3_access_logging           = true
enable_s3_cors                     = true
s3_cors_allowed_origins            = ["https://dashboard.nier.example.com"]  # Update with real domain
enable_intelligent_tiering         = true

# EC2-GPU Configuration - Production capacity
gpu_instance_type          = "g4dn.2xlarge"
gpu_ami_id                 = null
gpu_root_volume_size       = 100
gpu_data_volume_size       = 500
gpu_desired_capacity       = 4
gpu_min_size               = 2
gpu_max_size               = 20
gpu_enable_autoscaling     = true
gpu_target_utilization     = 70
gpu_enable_scheduled_scaling = true
gpu_scheduled_max_size     = 10
gpu_scale_up_cron          = "0 6 * * MON-FRI"   # Scale up at 6 AM UTC
gpu_scale_down_cron        = "0 22 * * MON-FRI"  # Scale down at 10 PM UTC
gpu_inference_port         = 8080
gpu_health_check_port      = 8081
gpu_log_retention_days     = 90
gpu_latency_threshold_ms   = 500
gpu_create_load_balancer   = true

# Monitoring Configuration
enable_cloudwatch_alarms = true
# alarm_sns_topic_arns   = ["arn:aws:sns:us-west-2:123456789012:nier-prod-alerts"]  # Update with real ARN
alarm_sns_topic_arns     = []
