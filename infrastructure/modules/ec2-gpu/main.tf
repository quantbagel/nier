# EC2-GPU Module for Nier Factory Floor Analytics Platform
# Provides GPU instances for ML inference on video frames

locals {
  name_prefix = "${var.project_name}-${var.environment}"
}

data "aws_caller_identity" "current" {}
data "aws_region" "current" {}

# Get latest Deep Learning AMI
data "aws_ami" "deep_learning" {
  most_recent = true
  owners      = ["amazon"]

  filter {
    name   = "name"
    values = ["Deep Learning AMI (Ubuntu 20.04) Version *"]
  }

  filter {
    name   = "virtualization-type"
    values = ["hvm"]
  }

  filter {
    name   = "architecture"
    values = ["x86_64"]
  }
}

# Security Group for GPU instances
resource "aws_security_group" "gpu" {
  name        = "${local.name_prefix}-gpu-sg"
  description = "Security group for GPU inference instances"
  vpc_id      = var.vpc_id

  # Allow SSH from bastion/VPN
  ingress {
    description = "SSH from VPC"
    from_port   = 22
    to_port     = 22
    protocol    = "tcp"
    cidr_blocks = [var.vpc_cidr]
  }

  # Inference API endpoint
  ingress {
    description = "Inference API from VPC"
    from_port   = var.inference_port
    to_port     = var.inference_port
    protocol    = "tcp"
    cidr_blocks = [var.vpc_cidr]
  }

  # Health check endpoint
  ingress {
    description = "Health check from VPC"
    from_port   = var.health_check_port
    to_port     = var.health_check_port
    protocol    = "tcp"
    cidr_blocks = [var.vpc_cidr]
  }

  # Prometheus metrics
  ingress {
    description = "Prometheus metrics from VPC"
    from_port   = 9090
    to_port     = 9090
    protocol    = "tcp"
    cidr_blocks = [var.vpc_cidr]
  }

  # Allow from EKS nodes
  dynamic "ingress" {
    for_each = var.allowed_security_group_ids
    content {
      description     = "Traffic from EKS nodes"
      from_port       = var.inference_port
      to_port         = var.inference_port
      protocol        = "tcp"
      security_groups = [ingress.value]
    }
  }

  egress {
    from_port   = 0
    to_port     = 0
    protocol    = "-1"
    cidr_blocks = ["0.0.0.0/0"]
  }

  tags = merge(var.tags, {
    Name = "${local.name_prefix}-gpu-sg"
  })
}

# IAM Role for GPU instances
resource "aws_iam_role" "gpu" {
  name = "${local.name_prefix}-gpu-role"

  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Action = "sts:AssumeRole"
        Effect = "Allow"
        Principal = {
          Service = "ec2.amazonaws.com"
        }
      }
    ]
  })

  tags = var.tags
}

# Instance profile
resource "aws_iam_instance_profile" "gpu" {
  name = "${local.name_prefix}-gpu-profile"
  role = aws_iam_role.gpu.name
}

# SSM managed instance policy for secure access
resource "aws_iam_role_policy_attachment" "ssm" {
  role       = aws_iam_role.gpu.name
  policy_arn = "arn:aws:iam::aws:policy/AmazonSSMManagedInstanceCore"
}

# CloudWatch agent policy
resource "aws_iam_role_policy_attachment" "cloudwatch" {
  role       = aws_iam_role.gpu.name
  policy_arn = "arn:aws:iam::aws:policy/CloudWatchAgentServerPolicy"
}

# Custom policy for S3, MSK, and Secrets Manager access
resource "aws_iam_role_policy" "gpu_custom" {
  name = "${local.name_prefix}-gpu-custom-policy"
  role = aws_iam_role.gpu.id

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Effect = "Allow"
        Action = [
          "s3:GetObject",
          "s3:PutObject",
          "s3:ListBucket"
        ]
        Resource = var.s3_bucket_arns
      },
      {
        Effect = "Allow"
        Action = [
          "kafka:DescribeCluster",
          "kafka:GetBootstrapBrokers",
          "kafka-cluster:Connect",
          "kafka-cluster:DescribeTopic",
          "kafka-cluster:ReadData"
        ]
        Resource = "*"
      },
      {
        Effect = "Allow"
        Action = [
          "secretsmanager:GetSecretValue"
        ]
        Resource = var.secrets_arns
      },
      {
        Effect = "Allow"
        Action = [
          "ecr:GetAuthorizationToken",
          "ecr:BatchCheckLayerAvailability",
          "ecr:GetDownloadUrlForLayer",
          "ecr:BatchGetImage"
        ]
        Resource = "*"
      },
      {
        Effect = "Allow"
        Action = [
          "kms:Decrypt",
          "kms:GenerateDataKey"
        ]
        Resource = var.kms_key_arns
      }
    ]
  })
}

# Launch Template for GPU instances
resource "aws_launch_template" "gpu" {
  name          = "${local.name_prefix}-gpu-lt"
  image_id      = var.ami_id != null ? var.ami_id : data.aws_ami.deep_learning.id
  instance_type = var.instance_type

  iam_instance_profile {
    arn = aws_iam_instance_profile.gpu.arn
  }

  network_interfaces {
    associate_public_ip_address = false
    security_groups             = [aws_security_group.gpu.id]
  }

  block_device_mappings {
    device_name = "/dev/sda1"

    ebs {
      volume_size           = var.root_volume_size
      volume_type           = "gp3"
      encrypted             = true
      kms_key_id            = var.ebs_kms_key_arn
      delete_on_termination = true
      throughput            = 250
      iops                  = 3000
    }
  }

  # Additional data volume for models and cache
  block_device_mappings {
    device_name = "/dev/sdf"

    ebs {
      volume_size           = var.data_volume_size
      volume_type           = "gp3"
      encrypted             = true
      kms_key_id            = var.ebs_kms_key_arn
      delete_on_termination = true
      throughput            = 500
      iops                  = 6000
    }
  }

  metadata_options {
    http_endpoint               = "enabled"
    http_tokens                 = "required"  # IMDSv2 required
    http_put_response_hop_limit = 1
  }

  monitoring {
    enabled = true
  }

  user_data = base64encode(templatefile("${path.module}/templates/user_data.sh", {
    region              = data.aws_region.current.name
    environment         = var.environment
    inference_port      = var.inference_port
    health_check_port   = var.health_check_port
    models_bucket       = var.models_bucket
    log_group           = aws_cloudwatch_log_group.gpu.name
  }))

  tag_specifications {
    resource_type = "instance"
    tags = merge(var.tags, {
      Name = "${local.name_prefix}-gpu-inference"
    })
  }

  tag_specifications {
    resource_type = "volume"
    tags = merge(var.tags, {
      Name = "${local.name_prefix}-gpu-volume"
    })
  }

  tags = var.tags
}

# Auto Scaling Group
resource "aws_autoscaling_group" "gpu" {
  name                = "${local.name_prefix}-gpu-asg"
  desired_capacity    = var.desired_capacity
  min_size            = var.min_size
  max_size            = var.max_size
  vpc_zone_identifier = var.private_subnet_ids
  health_check_type   = "EC2"
  health_check_grace_period = 300

  launch_template {
    id      = aws_launch_template.gpu.id
    version = "$Latest"
  }

  instance_refresh {
    strategy = "Rolling"
    preferences {
      min_healthy_percentage = 50
    }
  }

  dynamic "tag" {
    for_each = merge(var.tags, {
      Name = "${local.name_prefix}-gpu-inference"
    })
    content {
      key                 = tag.key
      value               = tag.value
      propagate_at_launch = true
    }
  }

  lifecycle {
    create_before_destroy = true
  }
}

# Target Tracking Scaling Policy - GPU Utilization
resource "aws_autoscaling_policy" "gpu_utilization" {
  count                  = var.enable_autoscaling ? 1 : 0
  name                   = "${local.name_prefix}-gpu-utilization-scaling"
  autoscaling_group_name = aws_autoscaling_group.gpu.name
  policy_type            = "TargetTrackingScaling"

  target_tracking_configuration {
    customized_metric_specification {
      metric_dimension {
        name  = "AutoScalingGroupName"
        value = aws_autoscaling_group.gpu.name
      }
      metric_name = "GPUUtilization"
      namespace   = "Nier/GPU"
      statistic   = "Average"
    }
    target_value = var.target_gpu_utilization
  }
}

# Scheduled scaling for predictable workloads
resource "aws_autoscaling_schedule" "scale_up" {
  count                  = var.enable_scheduled_scaling ? 1 : 0
  scheduled_action_name  = "${local.name_prefix}-scale-up"
  autoscaling_group_name = aws_autoscaling_group.gpu.name
  min_size               = var.scheduled_max_size
  max_size               = var.scheduled_max_size
  desired_capacity       = var.scheduled_max_size
  recurrence             = var.scale_up_cron
}

resource "aws_autoscaling_schedule" "scale_down" {
  count                  = var.enable_scheduled_scaling ? 1 : 0
  scheduled_action_name  = "${local.name_prefix}-scale-down"
  autoscaling_group_name = aws_autoscaling_group.gpu.name
  min_size               = var.min_size
  max_size               = var.max_size
  desired_capacity       = var.min_size
  recurrence             = var.scale_down_cron
}

# CloudWatch Log Group
resource "aws_cloudwatch_log_group" "gpu" {
  name              = "/nier/${var.environment}/gpu-inference"
  retention_in_days = var.log_retention_days

  tags = var.tags
}

# CloudWatch Alarms
resource "aws_cloudwatch_metric_alarm" "gpu_high" {
  count               = var.enable_alarms ? 1 : 0
  alarm_name          = "${local.name_prefix}-gpu-high-utilization"
  comparison_operator = "GreaterThanThreshold"
  evaluation_periods  = 3
  metric_name         = "GPUUtilization"
  namespace           = "Nier/GPU"
  period              = 300
  statistic           = "Average"
  threshold           = 90
  alarm_description   = "GPU utilization is high"

  dimensions = {
    AutoScalingGroupName = aws_autoscaling_group.gpu.name
  }

  alarm_actions = var.alarm_actions
  ok_actions    = var.ok_actions

  tags = var.tags
}

resource "aws_cloudwatch_metric_alarm" "inference_latency" {
  count               = var.enable_alarms ? 1 : 0
  alarm_name          = "${local.name_prefix}-inference-latency-high"
  comparison_operator = "GreaterThanThreshold"
  evaluation_periods  = 3
  metric_name         = "InferenceLatency"
  namespace           = "Nier/GPU"
  period              = 300
  statistic           = "p99"
  threshold           = var.latency_threshold_ms
  alarm_description   = "Inference latency is high"

  dimensions = {
    AutoScalingGroupName = aws_autoscaling_group.gpu.name
  }

  alarm_actions = var.alarm_actions
  ok_actions    = var.ok_actions

  tags = var.tags
}

# Application Load Balancer for inference endpoints
resource "aws_lb" "gpu" {
  count              = var.create_load_balancer ? 1 : 0
  name               = "${local.name_prefix}-gpu-alb"
  internal           = true
  load_balancer_type = "application"
  security_groups    = [aws_security_group.alb[0].id]
  subnets            = var.private_subnet_ids

  enable_deletion_protection = var.environment == "prod"

  tags = merge(var.tags, {
    Name = "${local.name_prefix}-gpu-alb"
  })
}

resource "aws_security_group" "alb" {
  count       = var.create_load_balancer ? 1 : 0
  name        = "${local.name_prefix}-gpu-alb-sg"
  description = "Security group for GPU ALB"
  vpc_id      = var.vpc_id

  ingress {
    description = "HTTPS from VPC"
    from_port   = 443
    to_port     = 443
    protocol    = "tcp"
    cidr_blocks = [var.vpc_cidr]
  }

  ingress {
    description = "HTTP from VPC"
    from_port   = 80
    to_port     = 80
    protocol    = "tcp"
    cidr_blocks = [var.vpc_cidr]
  }

  egress {
    from_port   = 0
    to_port     = 0
    protocol    = "-1"
    cidr_blocks = ["0.0.0.0/0"]
  }

  tags = merge(var.tags, {
    Name = "${local.name_prefix}-gpu-alb-sg"
  })
}

resource "aws_lb_target_group" "gpu" {
  count    = var.create_load_balancer ? 1 : 0
  name     = "${local.name_prefix}-gpu-tg"
  port     = var.inference_port
  protocol = "HTTP"
  vpc_id   = var.vpc_id

  health_check {
    enabled             = true
    healthy_threshold   = 2
    interval            = 30
    matcher             = "200"
    path                = "/health"
    port                = tostring(var.health_check_port)
    protocol            = "HTTP"
    timeout             = 5
    unhealthy_threshold = 3
  }

  tags = var.tags
}

resource "aws_lb_listener" "gpu" {
  count             = var.create_load_balancer ? 1 : 0
  load_balancer_arn = aws_lb.gpu[0].arn
  port              = 80
  protocol          = "HTTP"

  default_action {
    type             = "forward"
    target_group_arn = aws_lb_target_group.gpu[0].arn
  }
}

resource "aws_autoscaling_attachment" "gpu" {
  count                  = var.create_load_balancer ? 1 : 0
  autoscaling_group_name = aws_autoscaling_group.gpu.name
  lb_target_group_arn    = aws_lb_target_group.gpu[0].arn
}
