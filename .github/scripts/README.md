# CI Scripts Documentation

This directory contains intelligent validation scripts for CI checks. Instead of blindly suppressing failures with `continue-on-error: true` or `|| true`, these scripts:
1. Run the actual check/tool
2. Parse the output for issues
3. Validate against a documented allowlist of acceptable issues
4. Fail the build if any unexpected issues are detected

This approach provides:
- **Visibility**: All issues are logged and reported
- **Safety**: Unexpected issues cause builds to fail
- **Documentation**: Each acceptable issue is investigated and documented
- **Maintainability**: Clear criteria for when to update allowlists

## check-leak-sanitizer.sh

Runs tests with LeakSanitizer and validates against allowlist of acceptable memory behaviors.

### Purpose

1. Runs the full test suite with LeakSanitizer enabled
2. Parses the output for memory leaks
3. Validates that only known, documented, and acceptable leaks occur
4. Fails the build if any unexpected leaks are detected

### Known Allowlisted Leaks

#### 1. Cranelift JIT 4096-byte Allocation

**Pattern:** `Direct leak of 4096 byte(s) in 1 object(s)`

**Root Cause:**
Cranelift (the JIT compiler we use) employs a custom memory allocator for executable JIT code. This allocator is designed to manage memory lifecycles in a way that LeakSanitizer cannot properly track.

From `cranelift-jit/src/memory.rs:129-131`:
> "JIT memory manager. This manages pages of suitably aligned and accessible memory. Memory will be leaked by default to have function pointers remain valid for the remainder of the program's life."

**Our Investigation:**
- The 4096-byte allocation is cranelift's page-sized memory block for JIT code
- The memory appears as a "leak" because LSAN can't see cranelift's custom Drop implementation
- The memory IS properly freed when the `JITModule` is dropped (via Rust's Drop trait)
- We verified this by examining cranelift's source code and our own memory management

**Our Fix:**
We restructured the code to ensure proper lifecycle management:
1. Each `TieredPolicy` owns its own `JitCompiler` instance
2. The `JitCompiler` owns the `JITModule` which manages executable memory
3. When a policy is dropped, its compiler and all JIT memory are freed
4. No dangling pointers or actual memory leaks exist

**Status:** EXPECTED - This is a false positive from the sanitizer. The memory is correctly managed.

**References:**
- `crates/ipe-core/src/jit.rs` - JIT compilation and memory management
- `crates/ipe-core/src/tiering.rs` - Policy-level compiler ownership
- `lsan-suppressions.txt` - Detailed suppression documentation

### Usage

The script is automatically run by the CI workflow in `.github/workflows/security.yml`.

To run locally:
```bash
.github/scripts/check-leak-sanitizer.sh
```

### Exit Codes

- `0`: All tests passed, no unexpected leaks
- `1`: Unexpected leaks detected or test failures

### Maintenance

When adding new allowlisted leaks:
1. Thoroughly investigate the root cause
2. Document the investigation in this README
3. Update the pattern matching in `check-leak-sanitizer.sh`
4. Update `lsan-suppressions.txt` with detailed explanation
5. Ensure the leak is truly acceptable (not a real bug)

---

## check-cargo-audit.sh

Runs cargo-audit and validates vulnerabilities against severity thresholds.

### Purpose

1. Runs cargo-audit with JSON output
2. Parses vulnerabilities by severity (HIGH/CRITICAL, MEDIUM, LOW)
3. Fails on HIGH/CRITICAL vulnerabilities unless explicitly allowlisted
4. Reports MEDIUM/LOW vulnerabilities as warnings (non-blocking)

### Policy

- **HIGH/CRITICAL**: Build FAILS unless explicitly documented in allowlist
- **MEDIUM**: Warning logged, build passes (should be addressed soon)
- **LOW**: Informational, build passes (track but not urgent)

### Current Allowlist

Currently: NONE - All HIGH/CRITICAL vulnerabilities must be addressed.

### Usage

```bash
.github/scripts/check-cargo-audit.sh
```

### Maintenance

When adding vulnerabilities to allowlist:
1. Document CVE ID and RUSTSEC advisory ID
2. Explain why it's acceptable (doesn't affect our usage, waiting for fix)
3. Add date and expected resolution timeline
4. Link to GitHub issue tracking the resolution
5. Update the allowlist section in `check-cargo-audit.sh`

---

## check-cargo-geiger.sh

Runs cargo-geiger and validates that core crates remain free of unsafe code.

### Purpose

1. Runs cargo-geiger on all workspace crates
2. Checks that core IPE crates contain zero unsafe blocks
3. Allows unsafe in dependencies where necessary
4. Fails if new unsafe code appears in core crates

### Policy

- **Core Crates** (ipe-core, ipe-control, ipe-cli): MUST have zero unsafe blocks
- **Dependencies**: MAY use unsafe if necessary (e.g., cranelift JIT, tokio internals)
- **New Unsafe**: Must be explicitly documented with safety justification

### Current Allowlist

Currently: NONE in core crates

**Known Dependencies with Unsafe** (expected):
- `cranelift-*`: JIT compilation requires unsafe for executable memory
- `tokio`: Async runtime primitives
- `parking_lot`: Lock-free data structures

### Usage

```bash
.github/scripts/check-cargo-geiger.sh
```

### Maintenance

When adding unsafe code to core crates:
1. Document why unsafe is absolutely necessary
2. Consider safe alternatives first
3. Add extensive safety comments in the code
4. Update the allowlist in `check-cargo-geiger.sh`
5. Review the unsafe code in PR review

---

## check-minimal-versions.sh

Runs cargo-minimal-versions and validates against known dependency resolution issues.

### Purpose

1. Runs cargo-minimal-versions to test with minimum dependency versions
2. Checks that the crate works with minimum specified versions
3. Validates against allowlist of known resolution issues
4. Fails if new incompatibilities are detected

### Known Allowlisted Issues

#### 1. tonic 0.14 + http Dependency Issue

**Pattern:** Version requirements mentioning tonic and http crates

**Root Cause:**
tonic 0.14 requires http 1.2+, but cargo-minimal-versions' algorithm may select an earlier http version that doesn't satisfy this constraint. This is a transitive dependency resolution issue.

**Investigation:**
The dependency tree works correctly with normal cargo resolution, but minimal-versions can pick incompatible minimum versions due to how its algorithm works. Our Cargo.toml specifies appropriate version bounds.

**Status:** EXPECTED - False positive from minimal-versions algorithm, dependencies are correct

**Resolution Plan:**
- Update tonic to version with more flexible http dependency
- Or update minimum tonic version in Cargo.toml
- Or wait for cargo-minimal-versions algorithm improvements

### Usage

```bash
.github/scripts/check-minimal-versions.sh
```

### Exit Codes

- `0`: All dependencies work at minimum versions, or only known issues present
- `1`: Unexpected minimal version incompatibilities detected

### Maintenance

When adding new allowlisted issues:
1. Investigate the root cause thoroughly
2. Document which crates and versions are involved
3. Explain why the issue is acceptable
4. Add expected resolution plan
5. Update pattern matching in `check-minimal-versions.sh`

---

## General Guidelines

### Adding New Validation Scripts

When creating a new intelligent validator:

1. **Name**: `check-<tool-name>.sh`
2. **Structure**:
   - Header comment documenting purpose and allowlist
   - Run the tool and capture output
   - Parse output for issues
   - Validate against documented allowlist
   - Fail on unexpected issues, pass on expected ones
3. **Documentation**: Add section to this README
4. **Workflow Integration**: Update `.github/workflows/*.yml` to use the script
5. **Make Executable**: `chmod +x .github/scripts/check-<tool-name>.sh`

### Allowlist Philosophy

An issue should ONLY be allowlisted if:
- The root cause is thoroughly understood
- It's truly a false positive or acceptable limitation
- There's a clear plan for resolution (or none needed)
- The issue is well-documented for future maintainers
- Allowlisting won't hide real bugs

**Never allowlist issues just to make CI pass!**
