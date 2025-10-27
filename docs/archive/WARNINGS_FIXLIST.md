# IPE Core - Warnings and Errors Fix List

**Generated**: 2025-10-26
**Total Warnings**: ~346
**Strategy**: Fix in priority order, batch similar fixes together

## Summary by Category

| Category | Count | Priority | Estimated Effort |
|----------|-------|----------|------------------|
| Unnecessary structure name repetition | 129 | High | Auto-fix (5 min) |
| Missing `#[must_use]` attributes | 80 | Medium | Auto-fix (5 min) |
| Missing `const fn` | 41 | Medium | Manual review (30 min) |
| `format!` string improvements | 25 | Low | Auto-fix (5 min) |
| Missing `# Errors` docs | 14 | Medium | Manual (1-2 hours) |
| Unused imports | 3 | High | Manual (5 min) |
| Casting warnings | 8 | Medium | Manual review (30 min) |
| Other | ~50 | Various | Manual review (1-2 hours) |

**Total estimated time**: 4-6 hours for complete cleanup

---

## Phase 1: Quick Auto-Fixes (30 minutes)

### Step 1.1: Fix Unused Imports (5 minutes)

```bash
# Files to fix manually:
# 1. crates/ipe-core/src/lib.rs:16 - unexpected cfg condition
# 2. crates/ipe-core/src/index.rs:2 - unused Interpreter
# 3. crates/ipe-core/src/interpreter.rs:1 - unused CompOp
```

**Action**:
```rust
// index.rs:2
- use crate::interpreter::{FieldMapping, Interpreter};
+ use crate::interpreter::FieldMapping;

// Already correctly imported in interpreter.rs
```

**Cargo.toml fix**:
```toml
# Add to [features]
testing = []
```

---

### Step 1.2: Run Clippy Auto-Fix (10 minutes)

```bash
cargo clippy --fix --lib -p ipe-core --allow-dirty --allow-staged
```

This will automatically fix:
- ✅ Unnecessary structure name repetition (129 warnings)
- ✅ `format!` string improvements (25 warnings)
- ✅ Some `#[must_use]` attributes
- ✅ Redundant closures
- ✅ Redundant clones

**Expected fixes**: ~200 warnings

---

### Step 1.3: Add `const fn` Where Possible (15 minutes)

Run this to see which functions can be const:

```bash
cargo clippy --lib -p ipe-core -- -W clippy::missing-const-for-fn 2>&1 | grep "pub fn" | grep "const fn"
```

**Files affected** (41 warnings):
- `ast/nodes.rs`: Constructor and helper functions
- `bytecode.rs`: Constant creation functions
- `engine.rs`: Builder methods

**Action**: Review each and add `const` where appropriate.

---

## Phase 2: Documentation Improvements (2 hours)

### Step 2.1: Add Missing `# Errors` Sections (1 hour)

**Files affected** (14 warnings):
- `compiler.rs`: `compile()`, `compile_expression()`, etc.
- `interpreter.rs`: `evaluate()`, `load_field()`, etc.
- `parser/parse.rs`: `parse_policy()`, `parse_expression()`, etc.
- `store.rs`: `compile_policy()`, `process_update()`, etc.

**Template**:
```rust
/// <existing doc>
///
/// # Errors
///
/// Returns an error if:
/// - <specific error condition 1>
/// - <specific error condition 2>
pub fn function_name(...) -> Result<...> { ... }
```

**Example**:
```rust
/// Compile a policy from source
///
/// # Errors
///
/// Returns an error if:
/// - Policy source fails to parse
/// - AST compilation fails
/// - Field mapping is invalid
fn compile_policy(...) -> Result<PolicyEntry> { ... }
```

---

### Step 2.2: Add Missing `# Panics` Sections (30 minutes)

**Files affected** (4 warnings):
- Functions that use `.unwrap()`, `.expect()`, array indexing

**Template**:
```rust
/// <existing doc>
///
/// # Panics
///
/// Panics if:
/// - <condition that causes panic>
pub fn function_name(...) { ... }
```

---

### Step 2.3: Add Backticks to Documentation Items (30 minutes)

**Files affected** (4 warnings):
- Code items in doc comments that aren't wrapped in backticks

**Example**:
```rust
- /// Returns the Result with the value
+ /// Returns the `Result` with the value
```

---

## Phase 3: Code Quality Improvements (2 hours)

### Step 3.1: Fix Casting Warnings (30 minutes)

**8 casting warnings to fix**:

1. **i16 to i32** (2 warnings):
```rust
- let x = offset as i32;
+ let x = i32::from(offset);
```

2. **usize to u16** (may truncate - 2 warnings):
```rust
// Add bounds checking:
let idx = usize::min(value, u16::MAX as usize) as u16;
// Or document why truncation is safe
```

3. **usize to i32** (may wrap/truncate - 4 warnings):
```rust
// Add bounds checking or use try_from:
let val = i32::try_from(value).expect("value too large");
```

4. **u32 to i64** (1 warning):
```rust
- let x = value as i64;
+ let x = i64::from(value);
```

---

### Step 3.2: Implement `Eq` for Types with `PartialEq` (15 minutes)

**4 types to fix**:
```rust
- #[derive(Debug, Clone, PartialEq)]
+ #[derive(Debug, Clone, PartialEq, Eq)]
pub struct SomeType { ... }
```

**Files**: Check compiler.rs, bytecode.rs, ast/types.rs

---

### Step 3.3: Replace `or_insert_with` Default (5 minutes)

```rust
- map.entry(key).or_insert_with(Default::default)
+ map.entry(key).or_default()
```

---

### Step 3.4: Fix Method Name Confusion (10 minutes)

**Issue**: `Expression::not()` conflicts with `std::ops::Not::not`

**Options**:
1. Rename to `logical_not()`
2. Implement `std::ops::Not` trait

**Recommendation**: Rename for clarity.

```rust
- pub fn not(operand: Expression) -> Self {
+ pub fn logical_not(operand: Expression) -> Self {
```

---

### Step 3.5: Fix Identical Match Arms (10 minutes)

**2 warnings**: Combine identical match arms

```rust
// Before
match x {
    A => do_something(),
    B => do_something(),
}

// After
match x {
    A | B => do_something(),
}
```

---

### Step 3.6: Fix Unused `self` Arguments (10 minutes)

**3 warnings**: Functions that don't use `self` should be associated functions

```rust
// Before
fn helper(&self, arg: T) -> R { ... } // doesn't use self

// After
fn helper(arg: T) -> R { ... }
```

---

### Step 3.7: Replace `if let/else` with `Option::map_or_else` (10 minutes)

**2 warnings**: More idiomatic Rust

```rust
// Before
if let Some(x) = opt {
    use_x(x)
} else {
    default()
}

// After
opt.map_or_else(default, use_x)
```

---

### Step 3.8: Replace Manual Impls with `#[derive]` (10 minutes)

**2 warnings**: Use derive macros instead of manual impls

```rust
// Can be derived for simple cases
- impl Default for MyStruct { ... }
+ #[derive(Default)]
```

---

### Step 3.9: Fix Redundant Clones (10 minutes)

**2 warnings**: Remove unnecessary `.clone()` calls

```rust
// Before
let x = value.clone();
use_value(x); // only used once

// After
use_value(value);
```

---

### Step 3.10: Fix Common Code in If Blocks (10 minutes)

**1 warning**: Extract common code from if blocks

```rust
// Before
if cond {
    do_a();
    common();
} else {
    do_b();
    common();
}

// After
if cond {
    do_a();
} else {
    do_b();
}
common();
```

---

## Phase 4: Validation & Testing (30 minutes)

### Step 4.1: Run Full Test Suite
```bash
cargo test --lib --package ipe-core
```

**Expected**: All 248 tests passing

---

### Step 4.2: Run Clippy Again
```bash
cargo clippy --lib --package ipe-core
```

**Expected**: 0 warnings

---

### Step 4.3: Check Coverage
```bash
cargo llvm-cov --lib --package ipe-core
```

**Expected**: Coverage ≥93.67% (no regressions)

---

### Step 4.4: Format Code
```bash
cargo fmt --all
```

---

### Step 4.5: Build in Release Mode
```bash
cargo build --release --package ipe-core
```

**Expected**: No warnings with full optimization

---

## Execution Plan

### Recommended Order:

1. **Week 1, Day 1** (30 min): Phase 1 - Auto-fixes
   - Run clippy --fix
   - Fix unused imports
   - Commit: "Fix auto-fixable clippy warnings"

2. **Week 1, Day 2** (2 hours): Phase 2 - Documentation
   - Add `# Errors` sections
   - Add `# Panics` sections
   - Add backticks
   - Commit: "Complete documentation for public API"

3. **Week 1, Day 3** (2 hours): Phase 3 - Code Quality
   - Fix casting warnings
   - Implement remaining improvements
   - Commit: "Improve code quality per clippy pedantic"

4. **Week 1, Day 4** (30 min): Phase 4 - Validation
   - Run all checks
   - Commit: "Validate zero-warning build"

---

## Tracking Progress

Create issues/tasks for each phase:

- [ ] Phase 1: Auto-fixes (30 min)
- [ ] Phase 2: Documentation (2 hours)
- [ ] Phase 3: Code Quality (2 hours)
- [ ] Phase 4: Validation (30 min)

---

## Notes

- **Batch similar fixes together** for cleaner git history
- **Run tests after each phase** to catch regressions early
- **Some warnings are subjective** - discuss with team before fixing
- **Pedantic linting is optional** - core team decides enforcement level

---

## Quick Reference Commands

```bash
# See all warnings
cargo clippy --lib -p ipe-core -- -W clippy::pedantic -W clippy::nursery 2>&1 | less

# Auto-fix what's possible
cargo clippy --fix --lib -p ipe-core --allow-dirty

# Count remaining warnings
cargo clippy --lib -p ipe-core -- -W clippy::pedantic 2>&1 | grep -c "warning:"

# Run tests
cargo test --lib -p ipe-core

# Check coverage
cargo llvm-cov --lib -p ipe-core

# Format
cargo fmt --all

# Build release
cargo build --release -p ipe-core
```

---

**Last Updated**: 2025-10-26
**Status**: Ready for execution
