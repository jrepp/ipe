//! Performance benchmarks for approval storage and retrieval

#![cfg(feature = "approvals")]

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use ipe_core::approval::{Approval, ApprovalCheck, ApprovalStore};
use std::sync::Arc;

fn bench_approval_grant(c: &mut Criterion) {
    let mut group = c.benchmark_group("approval_grant");

    let store = ApprovalStore::new_temp().unwrap();

    group.bench_function("single_grant", |b| {
        let mut counter = 0;
        b.iter(|| {
            counter += 1;
            store
                .grant_approval(Approval::new(
                    format!("bot-{}", counter),
                    "https://api.example.com/data",
                    "GET",
                    "admin",
                ))
                .unwrap();
        });
    });

    group.finish();
}

fn bench_approval_lookup(c: &mut Criterion) {
    let mut group = c.benchmark_group("approval_lookup");

    for size in [100, 1_000, 10_000].iter() {
        let store = ApprovalStore::new_temp().unwrap();

        // Pre-populate with approvals
        for i in 0..*size {
            store
                .grant_approval(Approval::new(
                    format!("bot-{}", i),
                    "https://api.example.com/data",
                    "GET",
                    "admin",
                ))
                .unwrap();
        }

        group.throughput(Throughput::Elements(1));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| {
                let bot_id = format!("bot-{}", size / 2); // Middle element
                black_box(
                    store
                        .has_approval(&bot_id, "https://api.example.com/data", "GET")
                        .unwrap(),
                )
            });
        });
    }

    group.finish();
}

fn bench_approval_lookup_negative(c: &mut Criterion) {
    let mut group = c.benchmark_group("approval_lookup_negative");

    for size in [100, 1_000, 10_000].iter() {
        let store = ApprovalStore::new_temp().unwrap();

        // Pre-populate with approvals
        for i in 0..*size {
            store
                .grant_approval(Approval::new(
                    format!("bot-{}", i),
                    "https://api.example.com/data",
                    "GET",
                    "admin",
                ))
                .unwrap();
        }

        group.throughput(Throughput::Elements(1));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| {
                // Lookup non-existent approval (tests bloom filter)
                black_box(
                    store
                        .has_approval("bot-99999", "https://api.example.com/data", "GET")
                        .unwrap(),
                )
            });
        });
    }

    group.finish();
}

fn bench_set_membership(c: &mut Criterion) {
    let mut group = c.benchmark_group("set_membership");

    for size in [100, 1_000, 10_000, 100_000].iter() {
        let store = ApprovalStore::new_temp().unwrap();

        // Pre-populate with approvals
        for i in 0..*size {
            store
                .grant_approval(Approval::new(
                    format!("bot-{}", i),
                    "https://api.example.com/data",
                    "GET",
                    "admin",
                ))
                .unwrap();
        }

        group.throughput(Throughput::Elements(1));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| {
                let bot_id = format!("bot-{}", size / 2);
                black_box(
                    store
                        .is_in_approved_set(&bot_id, "https://api.example.com/data")
                        .unwrap(),
                )
            });
        });
    }

    group.finish();
}

fn bench_batch_checks(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_checks");

    let store = ApprovalStore::new_temp().unwrap();

    // Pre-populate with 10k approvals
    for i in 0..10_000 {
        store
            .grant_approval(Approval::new(
                format!("bot-{}", i),
                format!("resource-{}", i % 100),
                if i % 3 == 0 { "POST" } else { "GET" },
                "admin",
            ))
            .unwrap();
    }

    for batch_size in [10, 100, 1000].iter() {
        let checks: Vec<ApprovalCheck> = (0..*batch_size)
            .map(|i| {
                ApprovalCheck::new(
                    format!("bot-{}", i * 10),
                    format!("resource-{}", (i * 10) % 100),
                    if (i * 10) % 3 == 0 { "POST" } else { "GET" },
                )
            })
            .collect();

        group.throughput(Throughput::Elements(*batch_size as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(batch_size),
            &checks,
            |b, checks| {
                b.iter(|| black_box(store.check_approvals(checks.clone()).unwrap()));
            },
        );
    }

    group.finish();
}

fn bench_list_approvals(c: &mut Criterion) {
    let mut group = c.benchmark_group("list_approvals");

    for approvals_per_identity in [1, 10, 100].iter() {
        let store = ApprovalStore::new_temp().unwrap();

        // Grant multiple approvals to bot-target
        for i in 0..*approvals_per_identity {
            store
                .grant_approval(Approval::new(
                    "bot-target",
                    format!("resource-{}", i),
                    "GET",
                    "admin",
                ))
                .unwrap();
        }

        // Add noise from other bots
        for i in 0..1000 {
            store
                .grant_approval(Approval::new(
                    format!("bot-other-{}", i),
                    "resource-X",
                    "GET",
                    "admin",
                ))
                .unwrap();
        }

        group.bench_with_input(
            BenchmarkId::from_parameter(approvals_per_identity),
            approvals_per_identity,
            |b, _| {
                b.iter(|| black_box(store.list_approvals("bot-target").unwrap()));
            },
        );
    }

    group.finish();
}

fn bench_approval_with_expiration(c: &mut Criterion) {
    let mut group = c.benchmark_group("approval_expiration");

    let store = ApprovalStore::new_temp().unwrap();

    // Mix of expired and valid approvals
    for i in 0..5000 {
        let mut approval = Approval::new(
            format!("bot-{}", i),
            "https://api.example.com/data",
            "GET",
            "admin",
        );

        if i % 2 == 0 {
            // Half expired
            approval.expires_at = Some(chrono::Utc::now().timestamp() - 100);
        } else {
            // Half valid
            approval.expires_at = Some(chrono::Utc::now().timestamp() + 3600);
        }

        store.grant_approval(approval).unwrap();
    }

    group.bench_function("check_with_expiration", |b| {
        b.iter(|| {
            // Check valid approval
            black_box(
                store
                    .has_approval("bot-1", "https://api.example.com/data", "GET")
                    .unwrap(),
            )
        });
    });

    group.bench_function("check_expired", |b| {
        b.iter(|| {
            // Check expired approval
            black_box(
                store
                    .has_approval("bot-0", "https://api.example.com/data", "GET")
                    .unwrap(),
            )
        });
    });

    group.finish();
}

fn bench_concurrent_access(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrent_access");

    let store = Arc::new(ApprovalStore::new_temp().unwrap());

    // Pre-populate
    for i in 0..1000 {
        store
            .grant_approval(Approval::new(
                format!("bot-{}", i),
                "https://api.example.com/data",
                "GET",
                "admin",
            ))
            .unwrap();
    }

    group.bench_function("parallel_reads", |b| {
        b.iter(|| {
            let handles: Vec<_> = (0..4)
                .map(|thread_id| {
                    let store = store.clone();
                    std::thread::spawn(move || {
                        for i in 0..100 {
                            let bot_id = format!("bot-{}", (thread_id * 100 + i) % 1000);
                            store
                                .has_approval(&bot_id, "https://api.example.com/data", "GET")
                                .unwrap();
                        }
                    })
                })
                .collect();

            for handle in handles {
                handle.join().unwrap();
            }
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_approval_grant,
    bench_approval_lookup,
    bench_approval_lookup_negative,
    bench_set_membership,
    bench_batch_checks,
    bench_list_approvals,
    bench_approval_with_expiration,
    bench_concurrent_access,
);

criterion_main!(benches);
