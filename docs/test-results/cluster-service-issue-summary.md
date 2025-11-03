# ClusterService "Not Implemented" Error - Root Cause Analysis

**Date:** 2025-10-28
**Issue:** Web console at localhost:8090 shows "Service Not Implemented" error
**Error:** `rpc error: code = Unimplemented desc = unknown service hashicorp.cloud.agf.v20251017.ClusterService`

## Summary

The ClusterService IS implemented and registered in the admin plane, but it cannot function because the database schema initialization failed. The database is missing critical tables including the `clusters` table.

## Root Cause

The database migration file `20251019100000_create_audit_logs.sql` is **missing goose migration markers**, causing the goose migration parser to fail:

```
ERROR: failed to run migrations: ERROR 20251019100000_create_audit_logs.sql:
failed to parse SQL migration file: failed to parse migration:
unexpected state 0 on line "CREATE TABLE IF NOT EXISTS audit_logs ("
```

When migrations fail:
1. The database schema is incomplete (missing tables)
2. GORM's `AutoMigrate` (called by `InitSchema` in `store.go:136`) either doesn't run or fails
3. The `clusters` table is never created
4. ClusterService queries fail when trying to access the non-existent table
5. The service returns "Unimplemented" error instead of a more specific database error

## Evidence

### 1. Service IS Registered
```json
{"@level":"info","@message":"All gRPC services registered","services":["FabricAdminService","ClusterService","ComponentService"]}
```

### 2. Database Schema Failed
```
ERROR: relation "clusters" does not exist (SQLSTATE 42P01)
ALTER TABLE "fabrics" ADD CONSTRAINT "fk_clusters_fabric" FOREIGN KEY ("fabric_id") REFERENCES "clusters"("fabric_id")
Failed to initialize database schema
```

### 3. Migration Parse Error
```
failed to run migrations: ERROR 20251019100000_create_audit_logs.sql:
failed to parse SQL migration file: failed to parse migration:
unexpected state 0 on line "CREATE TABLE IF NOT EXISTS audit_logs ("
```

### 4. Playwright Test Confirmation
Test successfully reproduced the issue:
- Navigating to "Fabrics" page triggers ClusterService call
- Returns HTTP 501 Not Implemented
- Error page displays: "rpc error: code = Unimplemented desc = unknown service hashicorp.cloud.agf.v20251017.ClusterService"

## Solution

### Fix the Migration File

The file `/Users/jrepp/hc/cloud-agf-devportal/models/migrations/20251019100000_create_audit_logs.sql` needs goose markers:

**Current (broken):**
```sql
-- Migration: Create audit_logs table
-- Version: 003
-- Description: Implements comprehensive audit logging...

CREATE TABLE IF NOT EXISTS audit_logs (
  ...
);
```

**Should be:**
```sql
-- +goose Up
-- +goose StatementBegin
CREATE TABLE IF NOT EXISTS audit_logs (
  ...
);

-- Create indexes...
CREATE INDEX idx_audit_logs_principal ON audit_logs(principal);
...

-- Add comments...
COMMENT ON TABLE audit_logs IS '...';
...
-- +goose StatementEnd

-- +goose Down
-- +goose StatementBegin
DROP TABLE IF EXISTS audit_logs;
-- +goose StatementEnd
```

### Steps to Fix

1. **Add goose markers to the migration file**
   ```bash
   cd /Users/jrepp/hc/cloud-agf-devportal
   # Edit models/migrations/20251019100000_create_audit_logs.sql
   # Add -- +goose Up/Down markers
   ```

2. **Rebuild the admin plane service**
   ```bash
   docker-compose up -d --build agf-admin-plane-1 agf-admin-plane-2
   ```

3. **Verify migrations run successfully**
   ```bash
   docker logs agf-admin-plane-1 | grep -i "migration\|schema"
   ```

4. **Check the database has all tables**
   ```bash
   docker exec agf-admin-plane-1 psql -U postgres -d agf_admin -c "\dt"
   ```

5. **Verify ClusterService is functional**
   ```bash
   curl http://localhost:8090
   # Navigate to Fabrics page - should work without error
   ```

## Files Involved

- `/Users/jrepp/hc/cloud-agf-devportal/models/migrations/20251019100000_create_audit_logs.sql` - Broken migration
- `/Users/jrepp/hc/cloud-agf-devportal/pkg/state/store.go:134-143` - InitSchema with AutoMigrate
- `/Users/jrepp/hc/cloud-agf-devportal/pkg/state/store.go:40` - Cluster struct definition
- Web console: `agf-admin-web-console` container (port 8090)
- Admin plane: `agf-admin-plane-1` container (port 28082)

## Playwright Tests Created

Three test files were created in `/Users/jrepp/dev/ipe/docs/tests/`:

1. **admin-console.spec.js** - Basic connectivity tests
2. **admin-console-debug.spec.js** - Screenshot and HTML capture
3. **admin-console-clusters.spec.js** - Reproduces the ClusterService error (✓ Confirmed)

Run with:
```bash
cd /Users/jrepp/dev/ipe/docs
npx playwright test admin-console-clusters.spec.js
```

## Additional Notes

- The migration file format inconsistency suggests it was created manually or from a different template
- Other migrations (20251017*) all have proper goose markers and work correctly
- This is a development environment issue - production deployments should catch this in CI/CD
- Consider adding migration syntax validation to CI pipeline
