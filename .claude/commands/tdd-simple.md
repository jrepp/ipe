Implement using Test-Driven Development:

1. Write tests FIRST (Red phase)
2. Implement to pass tests (Green phase)
3. Refactor while keeping tests green
4. Run: cargo test --lib --package ipe-core
5. Check coverage: cargo llvm-cov --lib --package ipe-core
6. Target: >90% coverage, 0 warnings
7. Update TDD_PROGRESS.md

Use TodoWrite to track progress. Mark items completed as you go.

Now implement the requested feature following this methodology.
