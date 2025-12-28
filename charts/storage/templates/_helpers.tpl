{{/*
Expand the name of the chart.
*/}}
{{- define "nier-storage.name" -}}
{{- default .Chart.Name .Values.nameOverride | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Create a default fully qualified app name.
We truncate at 63 chars because some Kubernetes name fields are limited to this (by the DNS naming spec).
If release name contains chart name it will be used as a full name.
*/}}
{{- define "nier-storage.fullname" -}}
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
{{- define "nier-storage.chart" -}}
{{- printf "%s-%s" .Chart.Name .Chart.Version | replace "+" "_" | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Common labels
*/}}
{{- define "nier-storage.labels" -}}
helm.sh/chart: {{ include "nier-storage.chart" . }}
{{ include "nier-storage.selectorLabels" . }}
{{- if .Chart.AppVersion }}
app.kubernetes.io/version: {{ .Chart.AppVersion | quote }}
{{- end }}
app.kubernetes.io/managed-by: {{ .Release.Service }}
app.kubernetes.io/component: storage
app.kubernetes.io/part-of: nier
{{- end }}

{{/*
Selector labels
*/}}
{{- define "nier-storage.selectorLabels" -}}
app.kubernetes.io/name: {{ include "nier-storage.name" . }}
app.kubernetes.io/instance: {{ .Release.Name }}
{{- end }}

{{/*
Create the name of the service account to use
*/}}
{{- define "nier-storage.serviceAccountName" -}}
{{- if .Values.serviceAccount.create }}
{{- default (include "nier-storage.fullname" .) .Values.serviceAccount.name }}
{{- else }}
{{- default "default" .Values.serviceAccount.name }}
{{- end }}
{{- end }}

{{/*
Create the database URL reference
*/}}
{{- define "nier-storage.databaseUrl" -}}
{{- if .Values.database.existingSecret }}
valueFrom:
  secretKeyRef:
    name: {{ .Values.database.existingSecret }}
    key: {{ .Values.database.existingSecretKey }}
{{- else if .Values.database.url }}
value: {{ .Values.database.url | quote }}
{{- else }}
value: ""
{{- end }}
{{- end }}

{{/*
S3 endpoint configuration
*/}}
{{- define "nier-storage.s3Endpoint" -}}
{{- if .Values.s3.endpoint }}
- name: S3_ENDPOINT
  value: {{ .Values.s3.endpoint | quote }}
{{- end }}
{{- end }}
