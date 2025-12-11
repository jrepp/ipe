{{/*
Expand the name of the chart.
*/}}
{{- define "ipe.name" -}}
{{- default .Chart.Name .Values.nameOverride | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Create a default fully qualified app name.
*/}}
{{- define "ipe.fullname" -}}
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
{{- define "ipe.chart" -}}
{{- printf "%s-%s" .Chart.Name .Chart.Version | replace "+" "_" | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Common labels
*/}}
{{- define "ipe.labels" -}}
helm.sh/chart: {{ include "ipe.chart" . }}
{{ include "ipe.selectorLabels" . }}
{{- if .Chart.AppVersion }}
app.kubernetes.io/version: {{ .Chart.AppVersion | quote }}
{{- end }}
app.kubernetes.io/managed-by: {{ .Release.Service }}
{{- end }}

{{/*
Selector labels
*/}}
{{- define "ipe.selectorLabels" -}}
app.kubernetes.io/name: {{ include "ipe.name" . }}
app.kubernetes.io/instance: {{ .Release.Name }}
{{- end }}

{{/*
Create the name of the service account to use
*/}}
{{- define "ipe.serviceAccountName" -}}
{{- if .Values.serviceAccount.create }}
{{- default (include "ipe.fullname" .) .Values.serviceAccount.name }}
{{- else }}
{{- default "default" .Values.serviceAccount.name }}
{{- end }}
{{- end }}

{{/*
Generate the config.toml content
*/}}
{{- define "ipe.config" -}}
[service]
name = {{ .Values.config.service.name | quote }}
{{- if .Values.config.service.instanceId }}
instance_id = {{ .Values.config.service.instanceId | quote }}
{{- else }}
instance_id = "$(POD_NAME)"
{{- end }}

max_heap_mb = {{ .Values.config.memory.maxHeap }}
policy_cache_mb = {{ .Values.config.memory.policyCache }}
data_cache_mb = {{ .Values.config.memory.dataCache }}

[data_plane]
worker_threads = {{ .Values.config.dataPlane.workerThreads }}
max_concurrent_evals = {{ .Values.config.dataPlane.maxConcurrentEvals }}

{{- if .Values.config.dataPlane.unix.enabled }}
[[data_plane.listeners]]
type = "unix"
path = {{ .Values.config.dataPlane.unix.path | quote }}
mode = {{ .Values.config.dataPlane.unix.mode }}
{{- end }}

{{- if .Values.config.dataPlane.tcp.enabled }}
[[data_plane.listeners]]
type = "tcp"
bind = "{{ .Values.config.dataPlane.tcp.bind }}:{{ .Values.config.dataPlane.tcp.port }}"
max_connections = {{ .Values.config.dataPlane.tcp.maxConnections }}
{{- end }}

[control_plane]
{{- if .Values.config.controlPlane.unix.enabled }}
type = "unix"
path = {{ .Values.config.controlPlane.unix.path | quote }}
mode = {{ .Values.config.controlPlane.unix.mode }}
{{- end }}
require_auth = {{ .Values.config.controlPlane.requireAuth }}
atomic_swap = {{ .Values.config.controlPlane.atomicSwap }}
validation_required = {{ .Values.config.controlPlane.validationRequired }}

[storage]
policy_backend = {{ .Values.config.storage.policyBackend | quote }}
policy_path = {{ .Values.config.storage.policyPath | quote }}
data_backend = {{ .Values.config.storage.dataBackend | quote }}
data_path = {{ .Values.config.storage.dataPath | quote }}
persist_on_update = {{ .Values.config.storage.persistOnUpdate }}
snapshot_interval = {{ .Values.config.storage.snapshotInterval }}

[observability]
metrics_enabled = {{ .Values.config.observability.metricsEnabled }}
metrics_path = {{ .Values.config.observability.metricsPath | quote }}
trace_sampling_rate = {{ .Values.config.observability.traceSamplingRate }}
{{- end }}
