# IPE Testing Directory

This directory contains example policy trees for testing GitOps-based policy synchronization (RFC-004).

## Directory Structure

The filesystem structure directly maps to the policy tree hierarchy:

```
testing/
└── policies/                          → Root of policy tree
    ├── prod/                          → prod (namespace)
    │   ├── deployment/                → prod.deployment (namespace)
    │   │   ├── approval.ipe           → prod.deployment.approval (policy)
    │   │   └── validation.ipe         → prod.deployment.validation (policy)
    │   └── api/                       → prod.api (namespace)
    │       └── rate_limit.ipe         → prod.api.rate_limit (policy)
    └── staging/                       → staging (namespace)
        └── deployment/                → staging.deployment (namespace)
            └── approval.ipe           → staging.deployment.approval (policy)
```

## Policy Tree Representation

The filesystem structure above translates to this policy tree:

```
root
├── prod (directory)
│   ├── deployment (directory)
│   │   ├── approval (policy)
│   │   └── validation (policy)
│   └── api (directory)
│       └── rate_limit (policy)
└── staging (directory)
    └── deployment (directory)
        └── approval (policy)
```

## Example Policies

### Production Environment

**prod.deployment.approval**
- Requires 2 senior engineer approvals for production deployments
- Enforces strict approval workflow

**prod.deployment.validation**
- Validates deployment readiness
- Checks: tests passed, security scans, documentation, rollback plan

**prod.api.rate_limit**
- Enforces API rate limiting (1000 requests/hour)
- Tracks per-user request counts

### Staging Environment

**staging.deployment.approval**
- More relaxed: requires only 1 engineer approval
- Faster iteration for staging deployments

## Usage

### Testing GitOps Sync

```bash
# Initialize a git repository (simulate GitOps source)
cd testing
git init
git add policies/
git commit -m "Initial policy tree"

# Configure IPE sidecar to sync from this repo
# In ipe.toml:
[control_plane.git_sync]
repository_url = "file:///path/to/ipe/testing"
branch = "main"
policies_directory = "policies"
```

### Manual Policy Loading

```bash
# Use the control plane API to load policies
curl --unix-socket /var/run/ipe/control.sock \
  -X POST \
  -d '{
    "method": "sync-from-git",
    "params": {
      "repository": "file:///path/to/ipe/testing",
      "branch": "main"
    }
  }'
```

### Querying the Policy Tree

```bash
# List all policies
curl --unix-socket /var/run/ipe/eval.sock \
  -X POST \
  -d '{"method": "list-policies", "params": {"prefix": ""}}'

# Get prod.deployment subtree
curl --unix-socket /var/run/ipe/eval.sock \
  -X POST \
  -d '{"method": "get-subtree", "params": {"path": "prod.deployment"}}'
```

### Evaluating Policies

```bash
# Test production deployment approval
curl --unix-socket /var/run/ipe/eval.sock \
  -X POST \
  -d '{
    "method": "evaluate",
    "params": {
      "policies": ["prod.deployment.approval"],
      "context": {
        "resource": {
          "type": "Deployment",
          "environment": "production"
        },
        "approvals": [
          {"user": "alice", "role": "senior_engineer"},
          {"user": "bob", "role": "senior_engineer"}
        ]
      }
    }
  }'
```

## Adding New Policies

To add a new policy:

1. Create directory structure following the pattern: `policies/namespace/subnamespace/`
2. Create `.ipe` file: `policy_name.ipe`
3. The policy path will be: `namespace.subnamespace.policy_name`

Example:
```bash
# Create new policy at prod.api.auth
mkdir -p policies/prod/api
cat > policies/prod/api/auth.ipe <<EOF
policy RequireAuthentication {
    require resource.type == "APIRequest"
    require request.auth_token != null
    allow("Authenticated request")
}
EOF
```

## Feature Flags Example

Policies can check feature flags advertised by the server:

```ipe
policy ConditionalFeature {
    // Check if experimental caching is enabled
    if feature_flags.experimental_caching {
        // Use caching-aware logic
        allow("Using cached evaluation")
    } else {
        // Standard evaluation
        allow("Standard evaluation")
    }
}
```

## References

- [RFC-004: Control Plane Architecture](../rfcs/004-control-plane.md)
- [RFC-003: Policy Tree Storage](../rfcs/003-policy-tree-storage.md)
- [RFC-002: SSE/JSON Protocol](../rfcs/002-sse-json-protocol.md)
