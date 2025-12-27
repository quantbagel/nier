# EC2-GPU Module Outputs

output "autoscaling_group_id" {
  description = "ID of the Auto Scaling Group"
  value       = aws_autoscaling_group.gpu.id
}

output "autoscaling_group_name" {
  description = "Name of the Auto Scaling Group"
  value       = aws_autoscaling_group.gpu.name
}

output "autoscaling_group_arn" {
  description = "ARN of the Auto Scaling Group"
  value       = aws_autoscaling_group.gpu.arn
}

output "launch_template_id" {
  description = "ID of the Launch Template"
  value       = aws_launch_template.gpu.id
}

output "launch_template_latest_version" {
  description = "Latest version of the Launch Template"
  value       = aws_launch_template.gpu.latest_version
}

output "security_group_id" {
  description = "Security group ID for GPU instances"
  value       = aws_security_group.gpu.id
}

output "iam_role_arn" {
  description = "IAM role ARN for GPU instances"
  value       = aws_iam_role.gpu.arn
}

output "iam_role_name" {
  description = "IAM role name for GPU instances"
  value       = aws_iam_role.gpu.name
}

output "instance_profile_arn" {
  description = "Instance profile ARN for GPU instances"
  value       = aws_iam_instance_profile.gpu.arn
}

output "cloudwatch_log_group_name" {
  description = "CloudWatch log group name"
  value       = aws_cloudwatch_log_group.gpu.name
}

output "cloudwatch_log_group_arn" {
  description = "CloudWatch log group ARN"
  value       = aws_cloudwatch_log_group.gpu.arn
}

output "load_balancer_arn" {
  description = "ARN of the Application Load Balancer"
  value       = var.create_load_balancer ? aws_lb.gpu[0].arn : null
}

output "load_balancer_dns_name" {
  description = "DNS name of the Application Load Balancer"
  value       = var.create_load_balancer ? aws_lb.gpu[0].dns_name : null
}

output "target_group_arn" {
  description = "ARN of the target group"
  value       = var.create_load_balancer ? aws_lb_target_group.gpu[0].arn : null
}

output "alb_security_group_id" {
  description = "Security group ID for the ALB"
  value       = var.create_load_balancer ? aws_security_group.alb[0].id : null
}
