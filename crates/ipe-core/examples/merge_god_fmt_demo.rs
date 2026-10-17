//! Deliberately unformatted demo used to exercise merge-god's mechanical
//! remediation path: CI runs `cargo fmt --check`, which fails on this file,
//! and the merge policy (mechanical-fixes) should apply `cargo fmt` and land
//! the result.

fn main() {
    let greeting = "merge-god rustfmt remediation demo";
    println!("{greeting}");
    println!("if this file lands tidy, mechanical remediation works");
}
