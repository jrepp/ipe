# RFC-005: Control Plane Architecture

**Status:** 📝 Draft
**Created:** 2025-10-27
**Author:** IPE Contributors
**Depends On:** RFC-001, RFC-002, RFC-003

## Summary

The control plane manages policy synchronization, updates, and metadata tracking. It provides GitOps-based workflows, embedding Git metadata into policies and mapping filesystem directory structures directly to policy tree hierarchies.

## Motivation

Requirements:
- **GitOps Native:** Policies versioned in Git, synchronized to sidecar
- **Filesystem Mapping:** Directory structure → Policy tree structure
- **Metadata Rich:** Git commit info, author, timestamp embedded in policies
- **Secure:** Authenticated access, audit logging
- **Atomic:** All-or-nothing policy updates
- **Observable:** Track sync status, drift detection

## Key Features

✅ GitOps-based policy synchronization
✅ Filesystem tree → Policy tree mapping
✅ Git metadata embedded in policy objects
✅ Atomic policy updates with rollback
✅ Feature flags configuration
✅ Audit logging for all operations

## Architecture

### Control Plane Components

```
┌─────────────────────────────────────────────┐
│            Control Plane                     │
├─────────────────────────────────────────────┤
│                                             │
│  ┌──────────────┐      ┌─────────────────┐ │
│  │ Git Sync     │      │  Policy Store   │ │
│  │ Manager      │─────▶│  Manager        │ │
│  │              │      │                 │ │
│  │ - Clone      │      │ - Validate      │ │
│  │ - Pull       │      │ - Compile       │ │
│  │ - Watch      │      │ - Store         │ │
│  └──────────────┘      └─────────────────┘ │
│         │                       │           │
│         │                       │           │
│  ┌──────▼───────────────────────▼────────┐  │
│  │    Control API (RFC-002)              │  │
│  │                                        │  │
│  │  - update-policy                      │  │
│  │  - sync-from-git                      │  │
│  │  - list-policies                      │  │
│  │  - get-metadata                       │  │
│  │  - set-feature-flags                  │  │
│  └────────────────────────────────────────┘  │
│                    │                         │
└────────────────────┼─────────────────────────┘
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
    synced_at: u64,              // When loaded into sidecar
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
auth_method = "ssh_key"  # or "token", "none"
ssh_key_path = "/var/lib/ipe/ssh/id_rsa"

# Sync behavior
sync_interval = 60  # seconds (0 = manual only)
auto_sync_on_start = true
policies_directory = "policies"  # Subdirectory in repo

# Git settings
shallow_clone = true  # --depth=1 for faster clones
fetch_tags = false

# Validation
require_all_valid = true  # Reject sync if any policy fails compilation
dry_run_before_apply = true
```

## Control Plane API

### Sync Operations

```json
// Manual sync from Git
event: sync-from-git
data: {
  "method": "sync-from-git",
  "params": {
    "repository": "https://github.com/example/policies.git",
    "branch": "main",
    "force": false  // Skip validation checks
  }
}

// Response
data: {
  "result": {
    "sync_id": "sync-abc123",
    "status": "success",
    "policies_updated": 5,
    "policies_added": 2,
    "policies_removed": 1,
    "commit": "abc123def456",
    "commit_message": "Add new deployment policies",
    "synced_at": 1698765432
  }
}

// Get sync status
event: get-sync-status
data: {"method": "get-sync-status"}

// Response
data: {
  "result": {
    "last_sync": {
      "sync_id": "sync-abc123",
      "commit": "abc123",
      "synced_at": 1698765432,
      "status": "success"
    },
    "repository": "https://github.com/example/policies.git",
    "branch": "main",
    "auto_sync_enabled": true,
    "next_sync_in": 45  // seconds
  }
}
```

### Policy Metadata Queries

```json
// Get policy metadata including Git info
event: get-policy-metadata
data: {
  "method": "get-policy-metadata",
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

### Feature Flags Management

```json
// Set feature flags
event: set-feature-flags
data: {
  "method": "set-feature-flags",
  "params": {
    "flags": {
      "streaming_enabled": true,
      "experimental_caching": true
    }
  }
}

// Response
data: {
  "result": {
    "updated_flags": ["streaming_enabled", "experimental_caching"],
    "current_flags": {
      "streaming_enabled": true,
      "policy_versioning": true,
      "tree_enumeration": true,
      "experimental_caching": true
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

## Rollback and Recovery

### Rollback to Previous Commit

```json
// Rollback to specific Git commit
event: rollback-to-commit
data: {
  "method": "rollback-to-commit",
  "params": {
    "commit": "abc123def456",
    "reason": "Broken policy in latest commit"
  }
}
```

### Policy Version History

Leverages RFC-003's version history to track policy changes:
- Each sync creates new policy versions
- Git commits linked to policy hashes
- Can restore to any previous version

## Implementation Phases

| Week | Deliverables |
|------|--------------|
| 1 | Filesystem scanning, path mapping, metadata extraction |
| 2 | Git clone/pull, commit metadata embedding |
| 3 | Sync API, atomic updates, notification |
| 4 | Feature flags API, audit logging, rollback |
| 5 | Auto-sync daemon, drift detection, reconciliation |

## Success Metrics

- <5s sync time for 100 policies
- <10ms metadata queries
- 100% audit coverage for control operations
- Zero policy loss during sync failures
- <1min to rollback to previous version

## Alternatives Considered

| Approach | Rejected Because |
|----------|------------------|
| Manual file upload | No version history, error-prone |
| Database-backed | Loses Git integration benefits |
| Polling Git API | Less efficient than git pull |
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
- [RFC-001: Sidecar Service Architecture](001-sidecar-service-architecture.md)
- [RFC-002: SSE/JSON Protocol](002-sse-json-protocol.md)
- [RFC-003: Policy Tree Storage](003-policy-tree-storage.md)
