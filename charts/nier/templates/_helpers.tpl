{{/*
Expand the name of the chart.
*/}}
{{- define "nier.name" -}}
{{- default .Chart.Name .Values.nameOverride | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Create a default fully qualified app name.
We truncate at 63 chars because some Kubernetes name fields are limited to this (by the DNS naming spec).
If release name contains chart name it will be used as a full name.
*/}}
{{- define "nier.fullname" -}}
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
{{- define "nier.chart" -}}
{{- printf "%s-%s" .Chart.Name .Chart.Version | replace "+" "_" | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Common labels
*/}}
{{- define "nier.labels" -}}
helm.sh/chart: {{ include "nier.chart" . }}
{{ include "nier.selectorLabels" . }}
{{- if .Chart.AppVersion }}
app.kubernetes.io/version: {{ .Chart.AppVersion | quote }}
{{- end }}
app.kubernetes.io/managed-by: {{ .Release.Service }}
app.kubernetes.io/part-of: nier-platform
{{- end }}

{{/*
Selector labels
*/}}
{{- define "nier.selectorLabels" -}}
app.kubernetes.io/name: {{ include "nier.name" . }}
app.kubernetes.io/instance: {{ .Release.Name }}
{{- end }}

{{/*
Create the name of the service account to use
*/}}
{{- define "nier.serviceAccountName" -}}
{{- if .Values.serviceAccount.create }}
{{- default (include "nier.fullname" .) .Values.serviceAccount.name }}
{{- else }}
{{- default "default" .Values.serviceAccount.name }}
{{- end }}
{{- end }}

{{/*
Return the proper image registry
*/}}
{{- define "nier.imageRegistry" -}}
{{- if .Values.global.imageRegistry }}
{{- .Values.global.imageRegistry }}/
{{- else }}
{{- "" }}
{{- end }}
{{- end }}

{{/*
Return the Kafka brokers
*/}}
{{- define "nier.kafkaBrokers" -}}
{{- .Values.global.kafka.brokers }}
{{- end }}

{{/*
Return the PostgreSQL host
*/}}
{{- define "nier.postgresqlHost" -}}
{{- .Values.global.postgresql.host }}
{{- end }}

{{/*
Return the PostgreSQL port
*/}}
{{- define "nier.postgresqlPort" -}}
{{- .Values.global.postgresql.port | default 5432 }}
{{- end }}

{{/*
Return the PostgreSQL database name
*/}}
{{- define "nier.postgresqlDatabase" -}}
{{- .Values.global.postgresql.database | default "nier" }}
{{- end }}

{{/*
Return the S3 bucket name
*/}}
{{- define "nier.s3Bucket" -}}
{{- .Values.global.storage.bucket }}
{{- end }}

{{/*
Return the S3 region
*/}}
{{- define "nier.s3Region" -}}
{{- .Values.global.storage.region | default "us-west-2" }}
{{- end }}

{{/*
Return the Redis host
*/}}
{{- define "nier.redisHost" -}}
{{- .Values.global.redis.host }}
{{- end }}

{{/*
Return the Redis port
*/}}
{{- define "nier.redisPort" -}}
{{- .Values.global.redis.port | default 6379 }}
{{- end }}

{{/*
Return the environment name
*/}}
{{- define "nier.environment" -}}
{{- .Values.global.environment | default "production" }}
{{- end }}

{{/*
Return the OTLP endpoint for tracing
*/}}
{{- define "nier.otlpEndpoint" -}}
{{- if .Values.global.observability.tracing.enabled }}
{{- .Values.global.observability.tracing.otlpEndpoint }}
{{- else }}
{{- "" }}
{{- end }}
{{- end }}

{{/*
Common environment variables for all services
*/}}
{{- define "nier.commonEnv" -}}
- name: NIER_ENVIRONMENT
  value: {{ include "nier.environment" . | quote }}
- name: KAFKA_BROKERS
  value: {{ include "nier.kafkaBrokers" . | quote }}
{{- if .Values.global.kafka.ssl.enabled }}
- name: KAFKA_SSL_ENABLED
  value: "true"
{{- end }}
{{- if .Values.global.kafka.sasl.enabled }}
- name: KAFKA_SASL_ENABLED
  value: "true"
- name: KAFKA_SASL_MECHANISM
  value: {{ .Values.global.kafka.sasl.mechanism | quote }}
{{- end }}
- name: POSTGRESQL_HOST
  value: {{ include "nier.postgresqlHost" . | quote }}
- name: POSTGRESQL_PORT
  value: {{ include "nier.postgresqlPort" . | quote }}
- name: POSTGRESQL_DATABASE
  value: {{ include "nier.postgresqlDatabase" . | quote }}
- name: S3_BUCKET
  value: {{ include "nier.s3Bucket" . | quote }}
- name: S3_REGION
  value: {{ include "nier.s3Region" . | quote }}
- name: REDIS_HOST
  value: {{ include "nier.redisHost" . | quote }}
- name: REDIS_PORT
  value: {{ include "nier.redisPort" . | quote }}
{{- if .Values.global.redis.ssl }}
- name: REDIS_SSL_ENABLED
  value: "true"
{{- end }}
{{- if .Values.global.observability.metrics.enabled }}
- name: METRICS_ENABLED
  value: "true"
- name: METRICS_PORT
  value: {{ .Values.global.observability.metrics.port | quote }}
{{- end }}
{{- if .Values.global.observability.tracing.enabled }}
- name: TRACING_ENABLED
  value: "true"
- name: OTLP_ENDPOINT
  value: {{ include "nier.otlpEndpoint" . | quote }}
{{- end }}
{{- end }}

{{/*
Database credentials environment variables
*/}}
{{- define "nier.databaseCredentialsEnv" -}}
{{- if .Values.global.postgresql.existingSecret }}
- name: POSTGRESQL_USERNAME
  valueFrom:
    secretKeyRef:
      name: {{ .Values.global.postgresql.existingSecret }}
      key: {{ .Values.global.postgresql.existingSecretUsernameKey | default "username" }}
- name: POSTGRESQL_PASSWORD
  valueFrom:
    secretKeyRef:
      name: {{ .Values.global.postgresql.existingSecret }}
      key: {{ .Values.global.postgresql.existingSecretPasswordKey | default "password" }}
{{- end }}
{{- end }}

{{/*
Return the namespace
*/}}
{{- define "nier.namespace" -}}
{{- .Values.namespace.name | default "nier" }}
{{- end }}

{{/*
Pod security context
*/}}
{{- define "nier.podSecurityContext" -}}
runAsNonRoot: true
runAsUser: 1000
runAsGroup: 1000
fsGroup: 1000
seccompProfile:
  type: RuntimeDefault
{{- end }}

{{/*
Container security context
*/}}
{{- define "nier.containerSecurityContext" -}}
allowPrivilegeEscalation: false
readOnlyRootFilesystem: true
capabilities:
  drop:
    - ALL
{{- end }}

{{/*
Affinity for high availability - spread across availability zones
*/}}
{{- define "nier.haAffinity" -}}
podAntiAffinity:
  preferredDuringSchedulingIgnoredDuringExecution:
    - weight: 100
      podAffinityTerm:
        labelSelector:
          matchLabels:
            {{- include "nier.selectorLabels" . | nindent 12 }}
        topologyKey: topology.kubernetes.io/zone
    - weight: 50
      podAffinityTerm:
        labelSelector:
          matchLabels:
            {{- include "nier.selectorLabels" . | nindent 12 }}
        topologyKey: kubernetes.io/hostname
{{- end }}

{{/*
Checksum for config changes - triggers pod restart on config change
*/}}
{{- define "nier.configChecksum" -}}
checksum/config: {{ include (print $.Template.BasePath "/kafka-topics.yaml") . | sha256sum }}
{{- end }}
