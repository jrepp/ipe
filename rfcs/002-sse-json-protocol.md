# RFC-002: SSE/JSON Protocol

**Status:** 📝 Draft
**Created:** 2025-10-27
**Author:** IPE Contributors
**Depends On:** RFC-001

## Summary

MCP-inspired protocol using Server-Sent Events (SSE) + JSON-RPC 2.0 for policy evaluation. Supports both request/response and streaming patterns with sub-millisecond overhead.

## Motivation

Requirements:
- **Simple:** No code generation, any language
- **Fast:** <100μs protocol overhead
- **Streaming:** Server-push for updates
- **Debuggable:** Human-readable JSON
- **Extensible:** Capability negotiation

## Key Features

✅ JSON-RPC 2.0 for consistency
✅ SSE for streaming (HTTP-compatible)
✅ Optional binary encoding (MessagePack)
✅ Multiple eval requests (N >= 1) in sequence
✅ Server-advertised capabilities in hello
✅ Policy tree enumeration
✅ Feature flags support

## Protocol Stack

```
Application  → JSON-RPC 2.0 messages
Message      → SSE format (event/id/data)
Transport    → Unix socket / TCP
```

### Message Format (SSE)

```
event: <type>
id: <message-id>
data: <json-rpc-payload>

```

### Connection Patterns

1. **Request/Response:** Single eval → single result
2. **Streaming:** Subscribe → continuous results
3. **Control:** Commands with notifications

## Data Plane Protocol

### Connection Handshake

```
Client → GET /eval HTTP/1.1
         Upgrade: ipe/1.0
         Accept: text/event-stream

Server → HTTP/1.1 200 OK
         Content-Type: text/event-stream

         event: hello
         data: {
           "jsonrpc": "2.0",
           "method": "hello",
           "params": {
             "protocol_version": "1.0",
             "capabilities": ["evaluate", "subscribe", "list-policies", "feature-flags"],
             "encodings": ["json", "msgpack"],
             "server_version": "0.1.0",
             "feature_flags": {
               "streaming_enabled": true,
               "policy_versioning": true,
               "tree_enumeration": true
             }
           }
         }
```

The server MUST send a `hello` message immediately upon connection, advertising its capabilities and enabled feature flags.

### Request/Response Example

**Request:**
```json
event: evaluate
id: req-001
data: {
  "jsonrpc": "2.0",
  "method": "evaluate",
  "params": {
    "policies": ["prod.deployment.approval"],
    "context": {
      "resource": {"type": "Deployment", "environment": "production"},
      "action": "deploy",
      "request": {"user": "alice@example.com"}
    }
  }
}
```

**Response:**
```json
event: result
id: req-001
data: {
  "jsonrpc": "2.0",
  "result": {
    "decision": "deny",
    "reason": "Requires 2 senior engineer approvals",
    "evaluation_time_us": 150
  }
}
```

### Streaming Example

```json
// Subscribe
event: subscribe
data: {"method": "subscribe", "params": {"policies": ["*"]}}

// Server sends multiple results + notifications
event: result
data: {"result": {"decision": "allow", ...}}

event: policy-updated
data: {"method": "notification", "params": {"policy": "...", "version": "..."}}
```

### Multiple Evaluations

Clients can send N evaluation requests (where N >= 1) without waiting for responses. The server will process them and send back results with matching request IDs:

```json
// Client sends multiple requests with unique IDs
event: evaluate
id: req-001
data: {"method": "evaluate", "params": {"policies": ["a"], "context": {...}}}

event: evaluate
id: req-002
data: {"method": "evaluate", "params": {"policies": ["b"], "context": {...}}}

// Server responds with matching IDs (order not guaranteed)
event: result
id: req-001
data: {"result": {"decision": "allow", ...}}

event: result
id: req-002
data: {"result": {"decision": "deny", ...}}
```

Clients MUST include unique IDs to correlate requests and responses.

## Control Plane Protocol

**Handshake** (requires authentication):
```
GET /control HTTP/1.1
Authorization: Bearer <token>
```

### Policy Update Example

```json
// Update policy
event: update-policy
data: {
  "method": "update-policy",
  "params": {
    "path": "prod.deployment.approval",
    "version": "abc123",
    "source": "policy RequireApproval: ..."
  }
}

// Response
data: {"result": {"status": "applied", "version": "abc123"}}
```

### Data Update Example

```json
// Update dynamic data
event: update-data
data: {
  "method": "update-data",
  "params": {
    "key": "approvals.deploy-123",
    "value": {"approvers": [...]}
  }
}
```

### Policy Tree Enumeration

Clients can enumerate the policy tree to discover available policies and navigate the hierarchy:

```json
// List policies with prefix filter
event: list-policies
data: {
  "method": "list-policies",
  "params": {
    "prefix": "prod.",
    "depth": 2,          // Optional: limit tree depth
    "include_metadata": true
  }
}

// Response with tree structure
data: {
  "result": {
    "root": "prod",
    "children": [
      {
        "name": "deployment",
        "type": "directory",
        "children": [
          {
            "name": "approval",
            "type": "policy",
            "path": "prod.deployment.approval",
            "version": "abc123",
            "hash": "sha256:...",
            "updated_at": 1698765432
          }
        ]
      },
      {
        "name": "api",
        "type": "directory",
        "children": [...]
      }
    ]
  }
}

// Get subtree at specific path
event: get-subtree
data: {"method": "get-subtree", "params": {"path": "prod.deployment"}}

// Response includes only that subtree
data: {
  "result": {
    "path": "prod.deployment",
    "type": "directory",
    "children": [
      {"name": "approval", "type": "policy", ...},
      {"name": "validation", "type": "policy", ...}
    ]
  }
}
```

## Error Handling

### Error Format (JSON-RPC 2.0)

```json
event: error
data: {
  "jsonrpc": "2.0",
  "error": {
    "code": -32600,
    "message": "Invalid request",
    "data": {"field": "params.context"}
  }
}
```

### Error Codes

| Code | Meaning |
|------|---------|
| -32700 | Parse error (invalid JSON) |
| -32600 | Invalid request format |
| -32601 | Method not found |
| -32602 | Invalid parameters |
| -32000 | Policy not found |
| -32001 | Policy compilation failed |
| -32002 | Evaluation error |
| -32003 | Unauthorized |

## Performance Optimizations

| Optimization | Benefit | When to Use |
|--------------|---------|-------------|
| Connection pooling | Reuse sockets | Always |
| Binary encoding (MessagePack) | 30-50% smaller | High throughput |
| Compression (gzip) | 70% smaller | Large payloads |
| Zero-copy (SCM_RIGHTS) | No buffer copy | Huge datasets |

```
// Optional binary encoding
data-encoding: msgpack
data: <binary-data>
```

## Client Examples

### Rust
```rust
let client = Client::connect("/var/run/ipe/eval.sock").await?;
let decision = client.evaluate(&["prod.deployment.approval"], ctx).await?;
```

### Python
```python
async with Client("/var/run/ipe/eval.sock") as client:
    decision = await client.evaluate(["prod.deployment.approval"], ctx)
```

### cURL (testing)
```bash
curl --unix-socket /var/run/ipe/eval.sock \
  -H "Accept: text/event-stream" \
  -d '{"method":"evaluate","params":{...}}'
```

## Feature Flags

Feature flags allow servers to enable/disable functionality and clients to adapt behavior:

```json
// Server advertises feature flags in hello message (see Connection Handshake)
"feature_flags": {
  "streaming_enabled": true,
  "policy_versioning": true,
  "tree_enumeration": true,
  "audit_logging": false,
  "experimental_caching": false
}

// Client can query current feature flags
event: get-feature-flags
data: {"method": "get-feature-flags"}

// Response
data: {
  "result": {
    "flags": {
      "streaming_enabled": true,
      "policy_versioning": true,
      ...
    }
  }
}
```

Clients SHOULD check feature flags before using optional features. Servers MUST reject requests for disabled features with error code -32601 (Method not found).

## Security

- Unix socket: File system ACLs
- TCP: Bearer token authentication
- Rate limiting per connection
- Payload size limits (<1MB default)
- Request timeouts (30s default)

## Implementation Phases

| Week | Deliverables |
|------|--------------|
| 1 | SSE framing, JSON, request/response |
| 2 | Streaming, notifications |
| 3 | Control plane commands |
| 4 | Connection pooling, binary encoding |

## Success Metrics

- <100μs protocol overhead
- \>20k msg/sec per connection
- <1KB connection overhead
- <50 bytes message overhead

## References

- [SSE Spec](https://html.spec.whatwg.org/multipage/server-sent-events.html)
- [JSON-RPC 2.0](https://www.jsonrpc.org/specification)
- [MCP](https://modelcontextprotocol.io/)
- [MessagePack](https://msgpack.org/)
