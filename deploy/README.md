# IPE Deployment

This directory contains deployment configurations for the IPE Policy Engine.

## Prerequisites

- Docker Desktop with Kubernetes enabled, or another local Kubernetes cluster
- Helm 3.x installed
- kubectl configured to connect to your cluster

## Quick Start

### 1. Build the Docker image

```bash
# From the repository root
docker build -t ipe:0.1.0 .
```

### 2. Create the namespace

```bash
kubectl create namespace ipe
```

### 3. Install with Helm

```bash
# Install with default values
helm install ipe ./deploy/helm/ipe -n ipe

# Or install with custom values
helm install ipe ./deploy/helm/ipe -n ipe -f my-values.yaml
```

### 4. Verify the deployment

```bash
kubectl get pods -n ipe
kubectl get svc -n ipe
```

### 5. Port-forward to access locally

```bash
kubectl port-forward -n ipe svc/ipe 9001:9001
```

## Configuration

### Common customizations

Create a `my-values.yaml` file:

```yaml
# Increase replicas for HA
replicaCount: 3

# Use RocksDB for persistent storage
config:
  storage:
    policyBackend: "rocksdb"
    dataBackend: "rocksdb"

# Enable persistence
persistence:
  enabled: true
  size: 5Gi

# Adjust resources
resources:
  limits:
    cpu: 500m
    memory: 128Mi
  requests:
    cpu: 100m
    memory: 64Mi
```

### Loading policies via ConfigMap

```yaml
policies:
  enabled: true
  data:
    deployment-approval.ipe: |
      predicate RequireApproval:
        "Production deployments need 2+ approvals"
        triggers when
          resource.type == "Deployment"
          and environment == "production"
        requires
          approvals.count >= 2
```

### Enabling Prometheus metrics

```yaml
metrics:
  enabled: true
  serviceMonitor:
    enabled: true
    labels:
      release: prometheus
```

## Uninstall

```bash
helm uninstall ipe -n ipe
kubectl delete namespace ipe
```

## Directory Structure

```
deploy/
├── config/
│   └── default.toml      # Default IPE configuration
├── helm/
│   └── ipe/
│       ├── Chart.yaml
│       ├── values.yaml
│       └── templates/
│           ├── _helpers.tpl
│           ├── configmap.yaml
│           ├── configmap-policies.yaml
│           ├── deployment.yaml
│           ├── NOTES.txt
│           ├── pvc.yaml
│           ├── service.yaml
│           ├── serviceaccount.yaml
│           └── servicemonitor.yaml
└── README.md
```
