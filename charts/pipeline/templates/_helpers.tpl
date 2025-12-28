{{/*
Expand the name of the chart.
*/}}
{{- define "nier-pipeline.name" -}}
{{- default .Chart.Name .Values.nameOverride | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Create a default fully qualified app name.
We truncate at 63 chars because some Kubernetes name fields are limited to this (by the DNS naming spec).
If release name contains chart name it will be used as a full name.
*/}}
{{- define "nier-pipeline.fullname" -}}
{{- if .Values.fullnameOverride }}
{{- .Values.fullnameOverride | trunc 63 | trimSuffix "-" }}
{{- else }}
{{- $name := default .Chart.Name .Values.nameOverride }}
{{- if contains $name .Release.Name }}
{{- .Release.Name | trunc 63 | trimSuffix "-" }}
{{- else }}
{{- printf "%s-%s" .Release.Name $name | trunc 63 | trimSuffix "-" }}
{{- end }}
{{- end }}
{{- end }}

{{/*
Create chart name and version as used by the chart label.
*/}}
{{- define "nier-pipeline.chart" -}}
{{- printf "%s-%s" .Chart.Name .Chart.Version | replace "+" "_" | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Common labels
*/}}
{{- define "nier-pipeline.labels" -}}
helm.sh/chart: {{ include "nier-pipeline.chart" . }}
{{ include "nier-pipeline.selectorLabels" . }}
{{- if .Chart.AppVersion }}
app.kubernetes.io/version: {{ .Chart.AppVersion | quote }}
{{- end }}
app.kubernetes.io/managed-by: {{ .Release.Service }}
app.kubernetes.io/part-of: nier
{{- end }}

{{/*
Selector labels
*/}}
{{- define "nier-pipeline.selectorLabels" -}}
app.kubernetes.io/name: {{ include "nier-pipeline.name" . }}
app.kubernetes.io/instance: {{ .Release.Name }}
{{- end }}

{{/*
Consumer group coordination labels
These labels are used for Kafka consumer group coordination and pod identity
*/}}
{{- define "nier-pipeline.consumerGroupLabels" -}}
nier.io/consumer-group: {{ .Values.kafka.consumerGroup | quote }}
nier.io/input-topic: {{ .Values.kafka.inputTopic | quote }}
nier.io/component: "kafka-consumer"
nier.io/role: "pipeline"
{{- end }}

{{/*
Create the name of the service account to use
*/}}
{{- define "nier-pipeline.serviceAccountName" -}}
{{- if .Values.serviceAccount.create }}
{{- default (include "nier-pipeline.fullname" .) .Values.serviceAccount.name }}
{{- else }}
{{- default "default" .Values.serviceAccount.name }}
{{- end }}
{{- end }}

{{/*
Create the name of the configmap
*/}}
{{- define "nier-pipeline.configmapName" -}}
{{- printf "%s-config" (include "nier-pipeline.fullname" .) }}
{{- end }}

{{/*
Pod annotations including checksum for configmap
*/}}
{{- define "nier-pipeline.podAnnotations" -}}
{{- if .Values.podAnnotations }}
{{- toYaml .Values.podAnnotations }}
{{- end }}
checksum/config: {{ include (print $.Template.BasePath "/configmap.yaml") . | sha256sum }}
{{- end }}
