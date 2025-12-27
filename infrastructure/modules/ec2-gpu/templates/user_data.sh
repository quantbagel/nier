#!/bin/bash
set -e

# Configure logging
exec > >(tee /var/log/user-data.log|logger -t user-data -s 2>/dev/console) 2>&1

echo "Starting GPU instance initialization..."

# Update system
apt-get update -y
apt-get upgrade -y

# Install CloudWatch agent
wget https://s3.amazonaws.com/amazoncloudwatch-agent/ubuntu/amd64/latest/amazon-cloudwatch-agent.deb
dpkg -i -E ./amazon-cloudwatch-agent.deb
rm amazon-cloudwatch-agent.deb

# Configure CloudWatch agent
cat > /opt/aws/amazon-cloudwatch-agent/etc/amazon-cloudwatch-agent.json << 'EOF'
{
  "agent": {
    "metrics_collection_interval": 60,
    "run_as_user": "root"
  },
  "logs": {
    "logs_collected": {
      "files": {
        "collect_list": [
          {
            "file_path": "/var/log/nier/inference.log",
            "log_group_name": "${log_group}",
            "log_stream_name": "{instance_id}/inference",
            "retention_in_days": 30
          },
          {
            "file_path": "/var/log/nvidia-smi.log",
            "log_group_name": "${log_group}",
            "log_stream_name": "{instance_id}/nvidia-smi",
            "retention_in_days": 7
          }
        ]
      }
    }
  },
  "metrics": {
    "namespace": "Nier/GPU",
    "metrics_collected": {
      "cpu": {
        "measurement": ["cpu_usage_active", "cpu_usage_idle"],
        "metrics_collection_interval": 60
      },
      "disk": {
        "measurement": ["used_percent"],
        "metrics_collection_interval": 60,
        "resources": ["/", "/data"]
      },
      "mem": {
        "measurement": ["mem_used_percent"],
        "metrics_collection_interval": 60
      }
    },
    "append_dimensions": {
      "AutoScalingGroupName": "$${aws:AutoScalingGroupName}",
      "InstanceId": "$${aws:InstanceId}"
    }
  }
}
EOF

# Start CloudWatch agent
/opt/aws/amazon-cloudwatch-agent/bin/amazon-cloudwatch-agent-ctl -a fetch-config -m ec2 -s -c file:/opt/aws/amazon-cloudwatch-agent/etc/amazon-cloudwatch-agent.json

# Mount data volume
mkdir -p /data
# Wait for the volume to be attached
while [ ! -e /dev/nvme1n1 ] && [ ! -e /dev/xvdf ]; do
  echo "Waiting for data volume..."
  sleep 5
done

# Determine the device name
if [ -e /dev/nvme1n1 ]; then
  DATA_DEVICE=/dev/nvme1n1
else
  DATA_DEVICE=/dev/xvdf
fi

# Check if the volume is already formatted
if ! blkid $DATA_DEVICE; then
  mkfs.ext4 $DATA_DEVICE
fi

mount $DATA_DEVICE /data
echo "$DATA_DEVICE /data ext4 defaults,nofail 0 2" >> /etc/fstab

# Create directories
mkdir -p /data/models
mkdir -p /data/cache
mkdir -p /var/log/nier

# Set up NVIDIA GPU monitoring script
cat > /usr/local/bin/gpu-metrics.sh << 'SCRIPT'
#!/bin/bash
while true; do
  nvidia-smi --query-gpu=utilization.gpu,utilization.memory,memory.total,memory.used,temperature.gpu --format=csv,noheader,nounits >> /var/log/nvidia-smi.log

  # Parse and send custom metrics to CloudWatch
  GPU_UTIL=$(nvidia-smi --query-gpu=utilization.gpu --format=csv,noheader,nounits | head -1)
  MEM_UTIL=$(nvidia-smi --query-gpu=utilization.memory --format=csv,noheader,nounits | head -1)

  aws cloudwatch put-metric-data --region ${region} \
    --namespace "Nier/GPU" \
    --metric-name "GPUUtilization" \
    --value $GPU_UTIL \
    --unit Percent \
    --dimensions "AutoScalingGroupName=$(curl -s http://169.254.169.254/latest/meta-data/tags/instance/aws:autoscaling:groupName)"

  aws cloudwatch put-metric-data --region ${region} \
    --namespace "Nier/GPU" \
    --metric-name "GPUMemoryUtilization" \
    --value $MEM_UTIL \
    --unit Percent \
    --dimensions "AutoScalingGroupName=$(curl -s http://169.254.169.254/latest/meta-data/tags/instance/aws:autoscaling:groupName)"

  sleep 60
done
SCRIPT

chmod +x /usr/local/bin/gpu-metrics.sh

# Create systemd service for GPU metrics
cat > /etc/systemd/system/gpu-metrics.service << 'EOF'
[Unit]
Description=GPU Metrics Reporter
After=network.target

[Service]
Type=simple
ExecStart=/usr/local/bin/gpu-metrics.sh
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
EOF

systemctl daemon-reload
systemctl enable gpu-metrics
systemctl start gpu-metrics

# Download models from S3
aws s3 sync s3://${models_bucket}/models/ /data/models/ --region ${region}

# Create inference service configuration
cat > /etc/nier/inference.conf << EOF
ENVIRONMENT=${environment}
INFERENCE_PORT=${inference_port}
HEALTH_CHECK_PORT=${health_check_port}
MODELS_PATH=/data/models
CACHE_PATH=/data/cache
LOG_LEVEL=INFO
EOF

# Create inference service (placeholder - actual service would be deployed separately)
cat > /etc/systemd/system/nier-inference.service << 'EOF'
[Unit]
Description=Nier Inference Service
After=network.target

[Service]
Type=simple
EnvironmentFile=/etc/nier/inference.conf
ExecStart=/usr/local/bin/nier-inference
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
EOF

# Enable inference service (will start when binary is deployed)
systemctl daemon-reload
systemctl enable nier-inference

echo "GPU instance initialization complete!"
