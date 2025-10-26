---
description: Implement features using strict Test-Driven Development methodology with >90% coverage
---

Implement the requested feature using strict Test-Driven Development methodology:

## TDD Process

1. **Plan with TodoWrite**: Create task list breaking down feature into testable units

2. **For each unit, follow Red-Green-Refactor**:
   - **RED**: Write comprehensive tests FIRST (they should fail)
   - **GREEN**: Implement minimum code to pass all tests
   - **REFACTOR**: Clean up code while keeping tests green

3. **Run tests frequently**:
   ```bash
   cargo test --lib --package ipe-core
   ```

4. **Check coverage after each major unit**:
   ```bash
   cargo llvm-cov --lib --package ipe-core
   ```

5. **Update progress**: Add results to TDD_PROGRESS.md

6. **Mark todos completed**: Use TodoWrite as you complete each unit

## Coverage Targets

- **Overall**: >90% line coverage
- **Functions**: >90% function coverage
- **Quality**: 0 compiler warnings
- **Tests**: 100% passing, no flaky tests

## Test Priorities

Write tests covering:
- ✓ Happy path cases (typical usage)
- ✓ Edge cases and boundaries (empty, max, min values)
- ✓ Error conditions (invalid input, failures)
- ✓ Complex/nested scenarios (real-world complexity)
- ✓ RFC/specification examples (if applicable)

## Final Report Format

After completion, report:
```
Tests: X passed, Y failed
Coverage: Z% overall (target: >90%)
Module Coverage:
  - module.rs: X%
  - other.rs: Y%
New Features:
  - Feature 1
  - Feature 2
```

## Code Quality Checks

Before marking complete:
- [ ] All tests passing
- [ ] Coverage >90%
- [ ] No compiler warnings: `cargo build --lib`
- [ ] No clippy warnings: `cargo clippy --lib`
- [ ] Code formatted: `cargo fmt`
- [ ] TDD_PROGRESS.md updated

Now implement the requested feature following this methodology.
