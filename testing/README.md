# IPE Testing Directory

This directory contains example policy trees for testing GitOps-based policy synchronization (RFC-004).

## Directory Structure

The filesystem structure directly maps to the policy tree hierarchy:

```
testing/
└── policies/                          → Root of policy tree
    ├── _features/                     → _features (internal namespace)
    │   ├── sync/                      → _features.sync (namespace)
    │   │   ├── require_validation.ipe → _features.sync.require_validation (policy)
    │   │   └── allow_partial_sync.ipe → _features.sync.allow_partial_sync (policy)
    │   └── registry/                  → _features.registry (namespace)
    │       ├── enable_stats_collection.ipe → _features.registry.enable_stats_collection (policy)
    │       └── require_heartbeat.ipe  → _features.registry.require_heartbeat (policy)
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
├── _features (directory) [INTERNAL]
│   ├── sync (directory)
│   │   ├── require_validation (policy)
│   │   └── allow_partial_sync (policy)
│   └── registry (directory)
│       ├── enable_stats_collection (policy)
│       └── require_heartbeat (policy)
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

### Internal `_features` Policies

The `_features` namespace is **reserved for internal control plane policies**. These policies are loaded into the control plane's embedded IPE engine and control the behavior of the control plane itself.

**Important Notes:**
- `_features` policies are NOT exposed to data plane instances
- IPE instances do NOT get feature flags
- Only the control plane evaluates these policies to make operational decisions

**_features.sync.require_validation**
- Controls whether sync operations must validate all policies before applying
- Production: validation always required
- Staging: can optionally skip for rapid iteration

**_features.sync.allow_partial_sync**
- Controls whether partial syncs (some policies fail) are allowed
- Production: all-or-nothing required
- Dev/staging: can allow partial sync if explicitly requested

**_features.registry.enable_stats_collection**
- Controls detailed statistics collection from registered instances
- Production: always enabled for observability
- Other environments: configurable

**_features.registry.require_heartbeat**
- Controls heartbeat monitoring requirements
- Production: strict heartbeat requirements
- Staging/dev: more lenient

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

# Get the commit hash for sync
COMMIT_HASH=$(git rev-parse HEAD)

# Configure IPE control plane to sync from this repo
# In ipe.toml:
[control_plane.git_sync]
repository_url = "file:///path/to/ipe/testing"
branch = "main"
policies_root_path = "policies"

# Sync to specific commit (required)
curl --unix-socket /var/run/ipe/control.sock \
  -X POST \
  -d "{
    \"method\": \"sync\",
    \"params\": {
      \"commit\": \"$COMMIT_HASH\"
    }
  }"
```

### Querying Instance Registry

```bash
# List all registered IPE instances with stats
curl --unix-socket /var/run/ipe/control.sock \
  -X POST \
  -d '{"method": "list"}'

# Get detailed stats for a specific instance
curl --unix-socket /var/run/ipe/control.sock \
  -X POST \
  -d '{
    "method": "stats",
    "params": {"id": "ipe-abc-123"}
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

## Control Plane Feature Flags

The control plane uses its internal `_features` policy tree to control its own behavior. These are NOT feature flags for IPE instances - they are policies that the control plane evaluates internally to make operational decisions.

The control plane embeds the IPE engine directly (no socket communication needed). Before operations like syncing, the control plane evaluates `_features` policies:

```
Example: Before syncing, control plane internally evaluates:
  Policy: _features.sync.require_validation
  Context: {
    operation: "sync",
    environment: "production",
    skip_validation: false
  }
  Result: allow/deny with reasoning
```

**Remember**: Data plane instances advertise feature flags in their hello messages (RFC-002), but those are separate from the control plane's `_features` policies.

## Data Plane Usage

The data plane allows services to write dynamic data that predicates evaluate against (see RFC-005).

### Writing Approval Data

```bash
# Deployment service writes approval data
curl --unix-socket /var/run/ipe/data.sock \
  -X POST \
  -d '{
    "method": "put",
    "params": {
      "key": "approvals.deploy-123",
      "value": {
        "deployment_id": "deploy-123",
        "approvers": [
          {"user": "alice", "role": "senior_engineer", "timestamp": 1698765000},
          {"user": "bob", "role": "senior_engineer", "timestamp": 1698765100}
        ]
      },
      "ttl": 3600
    }
  }'

# Predicate prod.deployment.approval reads this data during evaluation
```

### Writing Relationship Data

```bash
# Identity service writes user relationships
curl --unix-socket /var/run/ipe/data.sock \
  -X POST \
  -d '{
    "method": "put",
    "params": {
      "key": "relationships.user-alice",
      "value": {
        "user_id": "alice",
        "groups": ["engineering", "senior-engineers"],
        "manager": "charlie",
        "org_unit": "platform"
      }
    }
  }'
```

### Reading Data in Predicates

Predicates access data using `data.get()`:

```ipe
policy RequireDeploymentApproval {
    // ... context checks ...

    // Fetch approval data
    let approval_data = data.get("approvals." + context.deployment_id) || {}
    let approvers = approval_data.approvers || []
    let senior_approvers = approvers.filter(a => a.role == "senior_engineer")

    if senior_approvers.length >= 2 {
        allow("Sufficient approvals")
    } else {
        deny("Requires 2 senior engineer approvals")
    }
}
```

**Key Points:**
- Services write data directly to data plane (not control plane)
- Different authorization model than policies
- High write throughput with eventual consistency (AP)
- Predicates read from local cache (fast)
- Data distributed via message plane (pub/sub or gossip)

## References

- [RFC-001: Predicate Service Architecture](../rfcs/001-predicate-service-architecture.md)
- [RFC-002: SSE/JSON Protocol](../rfcs/002-sse-json-protocol.md)
- [RFC-003: Policy Tree Storage](../rfcs/003-policy-tree-storage.md)
- [RFC-004: Control Plane Architecture](../rfcs/004-control-plane.md)
- [RFC-005: Data Plane Architecture](../rfcs/005-data-plane.md)
