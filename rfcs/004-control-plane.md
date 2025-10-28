# RFC-004: Control Plane Architecture

**Status:** 📝 Draft
**Created:** 2025-10-27
**Author:** IPE Contributors
**Depends On:** RFC-001, RFC-002, RFC-003

## Summary

The control plane manages policy synchronization, updates, and metadata tracking. It provides GitOps-based workflows, embedding Git metadata into policies and mapping filesystem directory structures directly to policy tree hierarchies.

## Motivation

Requirements:
- **GitOps Native:** Policies versioned in Git, synchronized to instances
- **Filesystem Mapping:** Directory structure → Policy tree structure
- **Metadata Rich:** Git commit info, author, timestamp embedded in policies
- **Secure:** Authenticated access, audit logging
- **Atomic:** All-or-nothing policy updates
- **Observable:** Instance registry with stats and uptime
- **Roll Forward Only:** No rollback, only forward to specific commits
- **Self-Hosting:** Control plane embeds IPE engine for internal decisions

## Key Features

✅ GitOps-based policy synchronization
✅ Filesystem tree → Policy tree mapping with configurable root path
✅ Git metadata embedded in policy objects
✅ Commit-hash based sync (idempotent, no status tracking)
✅ Roll forward only (sync to any commit)
✅ Embedded IPE engine for evaluating internal policies
✅ `_features` internal policy tree for control plane feature flags
✅ Instance registry with node info, uptime, and execution stats
✅ Terse, DRY API verbs (sync, list, stats, metadata)
✅ Audit logging for all operations

## Architecture

### Control Plane Components

```
┌──────────────────────────────────────────────────────────┐
│                   Control Plane                          │
├──────────────────────────────────────────────────────────┤
│                                                          │
│  ┌──────────────┐      ┌─────────────────┐             │
│  │ Git Sync     │      │  Policy Store   │             │
│  │ Manager      │─────▶│  Manager        │             │
│  │              │      │                 │             │
│  │ - Clone      │      │ - Validate      │             │
│  │ - Pull       │      │ - Compile       │             │
│  │ - Checkout   │      │ - Store         │             │
│  └──────────────┘      └─────────────────┘             │
│         │                       │                       │
│         │                       │                       │
│  ┌──────▼───────────────────────▼────────────────────┐  │
│  │         Control API (RFC-002)                     │  │
│  │                                                    │  │
│  │  - sync (commit-hash based)                       │  │
│  │  - list (registry with stats)                     │  │
│  │  - stats (detailed metrics)                       │  │
│  │  - policies (tree enumeration)                    │  │
│  │  - metadata (policy info)                         │  │
│  └────────────────────────────────────────────────────┘  │
│         │                       ▲                        │
│         │    ┌──────────────────┘                        │
│         │    │                                           │
│  ┌──────▼────▼───────────────┐    ┌──────────────────┐  │
│  │  Embedded IPE Engine      │    │    Registry      │  │
│  │                           │    │                  │  │
│  │  Policy Tree:             │    │ - Node info      │  │
│  │  - _features (internal)   │    │ - Uptime         │  │
│  │                           │    │ - Exec stats     │  │
│  │  Evaluates policies for   │    │ - Health         │  │
│  │  control plane decisions  │    │                  │  │
│  └───────────────────────────┘    └──────────────────┘  │
│                                                          │
└──────────────────────────────────────────────────────────┘
                     │
                     ▼
            Control Socket
         /var/run/ipe/control.sock
```

## GitOps Integration

### Repository Structure

Policies are organized in Git using filesystem directories that map directly to policy paths:

```
policies/
├── prod/
│   ├── deployment/
│   │   ├── approval.ipe          → prod.deployment.approval
│   │   └── validation.ipe        → prod.deployment.validation
│   └── api/
│       └── rate_limit.ipe        → prod.api.rate_limit
└── staging/
    └── deployment/
        └── approval.ipe          → staging.deployment.approval
```

**Mapping Rules:**
- Directory names become policy path segments (interior nodes)
- `.ipe` files become policy leaf nodes
- Path separator is `.` (e.g., `prod.deployment.approval`)
- Subdirectories create nested policy namespaces
- Non-`.ipe` files are ignored (README.md, etc.)

### Git Metadata

Each policy stores rich Git metadata:

```rust
struct PolicyMetadata {
    // Git information
    git_commit: String,          // SHA-1 commit hash
    git_author: String,          // Author name <email>
    git_committer: String,       // Committer name <email>
    git_commit_time: u64,        // Unix timestamp
    git_commit_message: String,  // First line of commit message
    git_branch: String,          // Branch name (e.g., "main")
    git_repository: String,      // Repository URL
    git_file_path: String,       // Path in repo (e.g., "prod/deployment/approval.ipe")

    // Policy information
    policy_path: String,         // IPE path (e.g., "prod.deployment.approval")
    policy_hash: Hash,           // SHA-256 of compiled bytecode
    compiled_at: u64,            // Compilation timestamp

    // Sync tracking
    sync_id: String,             // Unique sync operation ID
    synced_at: u64,              // When loaded into instance
}
```

### Sync Process

```
1. Git Sync Manager
   ├─ Clone repo (first time) or Pull (updates)
   ├─ Detect changed files since last sync
   └─ Read .ipe files with directory structure

2. Policy Processing
   ├─ Parse each .ipe file
   ├─ Compile to bytecode
   ├─ Extract Git metadata (git log, git rev-parse)
   └─ Compute policy hash (SHA-256 of bytecode)

3. Tree Construction
   ├─ Build policy tree from filesystem structure
   ├─ Directory "prod/deployment/" → tree path "prod.deployment"
   ├─ File "approval.ipe" → policy node "prod.deployment.approval"
   └─ Attach metadata to each policy node

4. Atomic Update
   ├─ Validate all policies compile successfully
   ├─ Batch insert into policy store (RFC-003)
   ├─ Atomic root swap (arc-swap)
   └─ Notify data planes of update
```

### Sync Configuration

```toml
[control_plane.git_sync]
# Repository configuration
repository_url = "https://github.com/example/policies.git"
branch = "main"
policies_root_path = "policies"  # Path in repo to policy tree root (default: "")
auth_method = "ssh_key"  # or "token", "none"
ssh_key_path = "/var/lib/ipe/ssh/id_rsa"

# Sync behavior
# Note: No auto-sync or intervals - sync is explicitly called with commit hash
# This makes sync idempotent and allows roll forward to any commit

# Git settings
shallow_clone = false  # Need full history to checkout specific commits
fetch_tags = false

# Validation
require_all_valid = true  # Reject sync if any policy fails compilation

[control_plane.embedded_ipe]
# Control plane embeds IPE engine for evaluating internal policies
policy_tree = "_features"  # Internal policy tree for control plane feature flags
# No socket needed - direct in-process evaluation
```

## Control Plane API

### Sync Operations

Sync operations are **commit-hash based** and **idempotent**. The control plane compiles all source policies to bytecode before applying. If any policy fails compilation, the entire sync is rejected.

```json
// Sync to specific commit (required)
event: sync
data: {
  "method": "sync",
  "params": {
    "commit": "abc123def456",  // REQUIRED: specific commit hash
    "repo": "https://github.com/example/policies.git",  // Optional: override config
    "path": "policies"  // Optional: override config (root path in repo)
  }
}

// Response: up-to-date (already at this commit)
data: {
  "result": {
    "status": "up-to-date",
    "commit": "abc123def456",
    "commit_message": "Add new deployment policies",
    "commit_time": 1698765400,
    "current_commit": "abc123def456",
    "policies_count": 8
  }
}

// Response: superseded (updated to new commit)
data: {
  "result": {
    "status": "superseded",
    "previous_commit": "xyz789old",
    "new_commit": "abc123def456",
    "commit_message": "Add new deployment policies",
    "commit_time": 1698765400,
    "policies_updated": 5,
    "policies_added": 2,
    "policies_removed": 1,
    "policies_compiled": 8,
    "synced_at": 1698765432
  }
}

// Response: compilation failure (sync rejected)
data: {
  "error": {
    "code": -32001,
    "message": "Policy compilation failed",
    "data": {
      "commit": "abc123def456",
      "failed_policies": [
        {
          "path": "prod.deployment.approval",
          "file": "policies/prod/deployment/approval.ipe",
          "error": "Syntax error at line 15: expected '}'"
        }
      ]
    }
  }
}
```

**Key Properties:**
- Sync is idempotent: calling with same commit hash multiple times is safe
- No sync status tracking: response tells you if up-to-date or superseded
- Roll forward only: sync to any commit (newer or older)
- All policies compiled before applying: atomic all-or-nothing update

### Policy Metadata

```json
// Get policy metadata including Git info
event: metadata
data: {
  "method": "metadata",
  "params": {"path": "prod.deployment.approval"}
}

// Response
data: {
  "result": {
    "path": "prod.deployment.approval",
    "hash": "sha256:abc123...",
    "git": {
      "commit": "abc123def456",
      "author": "Alice <alice@example.com>",
      "commit_time": 1698765432,
      "commit_message": "Require 2 approvers for prod deploys",
      "branch": "main",
      "repository": "https://github.com/example/policies.git",
      "file_path": "prod/deployment/approval.ipe"
    },
    "compiled_at": 1698765433,
    "synced_at": 1698765434
  }
}
```

### Registry

The control plane maintains a registry of all IPE instances with their status and statistics.

```json
// List all registered instances
event: list
data: {"method": "list"}

// Response
data: {
  "result": {
    "instances": [
      {
        "id": "ipe-abc-123",
        "node_name": "k8s-node-1",
        "hostname": "pod-web-app-xyz",
        "socket_path": "/var/run/ipe/eval.sock",
        "uptime_seconds": 86400,
        "started_at": 1698679032,
        "current_commit": "abc123def456",
        "policies_loaded": 8,
        "stats": {
          "total_evaluations": 150234,
          "evaluations_per_second": 42.3,
          "avg_eval_time_us": 234,
          "p50_eval_time_us": 180,
          "p99_eval_time_us": 890,
          "error_count": 12,
          "error_rate": 0.00008
        },
        "health": "healthy",
        "last_heartbeat": 1698765430
      },
      {
        "id": "ipe-def-456",
        "node_name": "k8s-node-2",
        "hostname": "pod-api-gateway-123",
        "socket_path": "/var/run/ipe/eval.sock",
        "uptime_seconds": 43200,
        "started_at": 1698722232,
        "current_commit": "abc123def456",
        "policies_loaded": 8,
        "stats": {
          "total_evaluations": 89234,
          "evaluations_per_second": 28.7,
          "avg_eval_time_us": 198,
          "p50_eval_time_us": 165,
          "p99_eval_time_us": 745,
          "error_count": 5,
          "error_rate": 0.00006
        },
        "health": "healthy",
        "last_heartbeat": 1698765429
      }
    ],
    "total": 2,
    "healthy": 2
  }
}

// Get detailed stats for specific instance
event: stats
data: {
  "method": "stats",
  "params": {"id": "ipe-abc-123"}
}

// Response includes detailed stats and recent evaluations
data: {
  "result": {
    "id": "ipe-abc-123",
    "node_name": "k8s-node-1",
    "stats": {
      "total_evaluations": 150234,
      "evaluations_by_policy": {
        "prod.deployment.approval": 45234,
        "prod.deployment.validation": 38923,
        "prod.api.rate_limit": 66077
      },
      "decision_breakdown": {
        "allow": 142345,
        "deny": 7889
      },
      "performance": {
        "avg_eval_time_us": 234,
        "p50_eval_time_us": 180,
        "p95_eval_time_us": 567,
        "p99_eval_time_us": 890,
        "max_eval_time_us": 2340
      }
    }
  }
}
```

## Filesystem to Policy Tree Mapping

### Example Mapping

```
Filesystem:                     Policy Tree:
policies/                       root
├── prod/                       ├── prod (directory)
│   ├── deployment/             │   ├── deployment (directory)
│   │   ├── approval.ipe        │   │   ├── approval (policy)
│   │   └── validation.ipe      │   │   └── validation (policy)
│   └── api/                    │   └── api (directory)
│       └── rate_limit.ipe      │       └── rate_limit (policy)
└── staging/                    └── staging (directory)
    └── deployment/                 └── deployment (directory)
        └── approval.ipe                └── approval (policy)
```

### Path Translation

```rust
// Filesystem path → Policy path
fn fs_to_policy_path(fs_path: &Path, base: &Path) -> String {
    // "policies/prod/deployment/approval.ipe" → "prod.deployment.approval"
    let relative = fs_path.strip_prefix(base).unwrap();
    let path_str = relative.to_str().unwrap();

    // Remove .ipe extension and replace / with .
    path_str
        .strip_suffix(".ipe").unwrap()
        .replace('/', ".")
}

// Policy path → Filesystem path
fn policy_to_fs_path(policy_path: &str, base: &Path) -> PathBuf {
    // "prod.deployment.approval" → "policies/prod/deployment/approval.ipe"
    let fs_path = policy_path.replace('.', "/");
    base.join(format!("{}.ipe", fs_path))
}
```

## Security

### Authentication

```toml
[control_plane.auth]
# Require authentication for all control operations
enabled = true

# Authentication methods
methods = ["token", "mTLS"]

# Token-based auth
token_file = "/var/lib/ipe/tokens/admin.token"

# mTLS configuration
client_ca = "/etc/ipe/ca.crt"
require_client_cert = true
```

### Audit Logging

All control plane operations are logged:

```json
{
  "timestamp": 1698765432,
  "operation": "sync-from-git",
  "user": "admin",
  "source_ip": "127.0.0.1",
  "params": {"repository": "...", "branch": "main"},
  "result": "success",
  "policies_affected": ["prod.deployment.approval"],
  "duration_ms": 234
}
```

## Internal Policy Tree: `_features`

The control plane embeds the IPE engine directly to evaluate an internal policy tree called `_features`. This tree is used exclusively by the control plane to make decisions about control plane operations.

**Key Characteristics:**
- **Embedded**: IPE engine runs in-process within control plane
- **Internal only**: Not exposed to data plane instances
- **Control plane feature flags**: Policies control control plane behavior
- **Self-hosting**: Control plane uses IPE to evaluate its own operational decisions
- **Reserved namespace**: `_features` is a reserved prefix

**Important**: IPE instances do NOT get feature flags. The `_features` policies control only the control plane's behavior.

### Example `_features` Policies

```
_features/
├── sync/
│   ├── allow_auto_compile.ipe      → _features.sync.allow_auto_compile
│   └── require_validation.ipe      → _features.sync.require_validation
└── registry/
    └── enable_stats_collection.ipe → _features.registry.enable_stats_collection
```

Example policy:
```ipe
// _features.sync.require_validation
policy RequireValidation {
    // Always require validation before applying policies
    require context.operation == "sync"

    if context.skip_validation == true {
        deny("Validation cannot be skipped in production")
    }

    allow("Validation required")
}
```

The control plane evaluates these policies before performing operations, allowing runtime configuration of control plane behavior.

## Roll Forward Semantics

There is **no rollback** - only **roll forward**. To revert to a previous state:

1. Identify the desired commit hash
2. Call `sync` with that commit hash
3. Control plane will checkout that commit and apply it

This is still "rolling forward" to a specific commit, even if it's older than the current one.

```json
// Roll forward to previous commit (not a rollback)
event: sync
data: {
  "method": "sync",
  "params": {
    "commit": "xyz789old"  // Older commit - still a forward operation
  }
}
```

**Benefits:**
- All changes are explicit operations
- Full audit trail (every sync is logged)
- No special rollback machinery
- Git history is the source of truth

## Implementation Phases

| Week | Deliverables |
|------|--------------|
| 1 | Filesystem scanning, path mapping, metadata extraction, compilation |
| 2 | Git operations: clone/pull/checkout, commit-hash based sync |
| 3 | Sync API (commit-based), atomic updates, instance notification |
| 4 | Embedded IPE engine, `_features` policy tree |
| 5 | Instance registry, stats collection, heartbeat monitoring |
| 6 | Audit logging, comprehensive error handling |

## Success Metrics

- <5s sync time for 100 policies (including compilation)
- <10ms metadata queries
- <100ms registry queries
- 100% audit coverage for control operations
- Zero policy loss during sync failures
- Idempotent sync: same commit hash always produces same result
- <30s heartbeat interval for health monitoring

## Alternatives Considered

| Approach | Rejected Because |
|----------|------------------|
| Manual file upload | No version history, error-prone |
| Database-backed | Loses Git integration benefits |
| Auto-sync with polling | Less explicit, harder to reason about state |
| Sync status tracking | Adds complexity, commit hash is sufficient |
| Rollback operations | Roll forward is simpler and more auditable |
| Custom VCS | Reinventing the wheel, Git is proven |

## Future Enhancements

### Multi-Repository Support

```toml
[[control_plane.git_sync.repositories]]
name = "prod-policies"
url = "https://github.com/example/prod-policies.git"
branch = "main"
base_path = "prod"  # Mount at "prod.*"

[[control_plane.git_sync.repositories]]
name = "staging-policies"
url = "https://github.com/example/staging-policies.git"
branch = "main"
base_path = "staging"  # Mount at "staging.*"
```

### Drift Detection

Monitor for divergence between Git source and runtime policies:
- Compare Git commit hash vs loaded policy metadata
- Alert on unexpected changes
- Auto-remediation via re-sync

### Webhook Integration

```toml
[control_plane.webhooks]
# Trigger sync on Git push events
enabled = true
listen_address = "0.0.0.0:8080"
secret = "webhook-secret"
```

## References

- [GitOps Principles](https://opengitops.dev/)
- [Git Internals](https://git-scm.com/book/en/v2/Git-Internals-Plumbing-and-Porcelain)
- [Kubernetes GitOps (Flux)](https://fluxcd.io/)
- [RFC-001: Predicate Service Architecture](001-predicate-service-architecture.md)
- [RFC-002: SSE/JSON Protocol](002-sse-json-protocol.md)
- [RFC-003: Policy Tree Storage](003-policy-tree-storage.md)
- [RFC-005: Data Plane Architecture](005-data-plane.md)
