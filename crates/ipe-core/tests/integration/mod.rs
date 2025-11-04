//! Integration tests for IPE with approval-based authorization

#[cfg(feature = "approvals")]
mod approval_tests;

#[cfg(feature = "approvals")]
mod e2e_tests;

#[cfg(feature = "approvals")]
mod security_tests;

#[cfg(feature = "approvals")]
mod relationship_tests;

#[cfg(feature = "approvals")]
mod scope_ttl_tests;

mod rar_tests;
