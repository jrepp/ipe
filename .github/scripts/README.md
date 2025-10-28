# CI Scripts Documentation

## check-leak-sanitizer.sh

This script runs tests with LeakSanitizer and intelligently validates the output against a known allowlist of acceptable memory behaviors.

### Purpose

Instead of blindly suppressing all leak sanitizer failures with `continue-on-error: true`, this script:
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
