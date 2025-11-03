//! Performance benchmarks for relationship storage and traversal

#![cfg(feature = "approvals")]

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use ipe_core::relationship::{Relationship, RelationshipQuery, RelationshipStore};

fn bench_relationship_add(c: &mut Criterion) {
    let mut group = c.benchmark_group("relationship_add");

    let store = RelationshipStore::new_temp().unwrap();

    group.bench_function("single_add", |b| {
        let mut counter = 0;
        b.iter(|| {
            counter += 1;
            store
                .add_relationship(Relationship::role(
                    format!("user-{}", counter),
                    "editor",
                    "document-123",
                    "admin",
                ))
                .unwrap();
        });
    });

    group.finish();
}

fn bench_relationship_lookup(c: &mut Criterion) {
    let mut group = c.benchmark_group("relationship_lookup");

    for size in [100, 1_000, 10_000].iter() {
        let store = RelationshipStore::new_temp().unwrap();

        // Pre-populate with relationships
        for i in 0..*size {
            store
                .add_relationship(Relationship::role(
                    format!("user-{}", i),
                    "editor",
                    "document-123",
                    "admin",
                ))
                .unwrap();
        }

        group.throughput(Throughput::Elements(1));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| {
                let user_id = format!("user-{}", size / 2);
                black_box(store.has_relationship(&user_id, "editor", "document-123").unwrap())
            });
        });
    }

    group.finish();
}

fn bench_transitive_relationship(c: &mut Criterion) {
    let mut group = c.benchmark_group("transitive_relationship");

    for depth in [2, 5, 10].iter() {
        let store = RelationshipStore::new_temp().unwrap().with_max_depth(*depth + 5);

        // Build trust chain
        for i in 0..*depth {
            store
                .add_relationship(Relationship::trust(
                    format!("node-{}", i),
                    format!("node-{}", i + 1),
                    "system",
                ))
                .unwrap();
        }

        group.throughput(Throughput::Elements(1));
        group.bench_with_input(BenchmarkId::from_parameter(depth), depth, |b, _| {
            b.iter(|| {
                black_box(
                    store
                        .has_transitive_relationship(
                            "node-0",
                            "trusted_by",
                            &format!("node-{}", depth),
                        )
                        .unwrap(),
                )
            });
        });
    }

    group.finish();
}

fn bench_trust_chain_traversal(c: &mut Criterion) {
    let mut group = c.benchmark_group("trust_chain_traversal");

    for chain_length in [2, 5, 10].iter() {
        let store = RelationshipStore::new_temp().unwrap();

        // Build linear trust chain
        for i in 0..*chain_length {
            store
                .add_relationship(Relationship::trust(
                    format!("cert-{}", i),
                    format!("ca-{}", i),
                    "pki",
                ))
                .unwrap();

            if i > 0 {
                store
                    .add_relationship(Relationship::trust(
                        format!("ca-{}", i - 1),
                        format!("cert-{}", i),
                        "pki",
                    ))
                    .unwrap();
            }
        }

        group.bench_with_input(BenchmarkId::from_parameter(chain_length), chain_length, |b, _| {
            b.iter(|| {
                black_box(
                    store
                        .find_relationship_path(
                            "cert-0",
                            "trusted_by",
                            &format!("ca-{}", chain_length - 1),
                        )
                        .unwrap(),
                )
            });
        });
    }

    group.finish();
}

fn bench_membership_hierarchy(c: &mut Criterion) {
    let mut group = c.benchmark_group("membership_hierarchy");

    let store = RelationshipStore::new_temp().unwrap();

    // Build org hierarchy
    // alice -> engineers -> tech -> employees -> everyone
    store
        .add_relationship(Relationship::membership("alice", "engineers", "hr"))
        .unwrap();
    store
        .add_relationship(Relationship::membership("engineers", "tech", "hr"))
        .unwrap();
    store
        .add_relationship(Relationship::membership("tech", "employees", "hr"))
        .unwrap();
    store
        .add_relationship(Relationship::membership("employees", "everyone", "hr"))
        .unwrap();

    group.bench_function("direct_membership", |b| {
        b.iter(|| black_box(store.has_relationship("alice", "member_of", "engineers").unwrap()))
    });

    group.bench_function("transitive_2_hops", |b| {
        b.iter(|| {
            black_box(store.has_transitive_relationship("alice", "member_of", "tech").unwrap())
        })
    });

    group.bench_function("transitive_4_hops", |b| {
        b.iter(|| {
            black_box(store.has_transitive_relationship("alice", "member_of", "everyone").unwrap())
        })
    });

    group.finish();
}

fn bench_batch_relationship_checks(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_relationship_checks");

    let store = RelationshipStore::new_temp().unwrap();

    // Pre-populate
    for i in 0..1000 {
        store
            .add_relationship(Relationship::role(
                format!("user-{}", i),
                "editor",
                format!("doc-{}", i % 100),
                "admin",
            ))
            .unwrap();
    }

    for batch_size in [10, 100, 1000].iter() {
        let queries: Vec<RelationshipQuery> = (0..*batch_size)
            .map(|i| {
                RelationshipQuery::new(
                    format!("user-{}", i * 10),
                    "editor",
                    format!("doc-{}", (i * 10) % 100),
                )
            })
            .collect();

        group.throughput(Throughput::Elements(*batch_size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(batch_size), &queries, |b, queries| {
            b.iter(|| black_box(store.check_relationships(queries.clone()).unwrap()));
        });
    }

    group.finish();
}

fn bench_list_subject_relationships(c: &mut Criterion) {
    let mut group = c.benchmark_group("list_subject_relationships");

    for rels_per_subject in [1, 10, 100].iter() {
        let store = RelationshipStore::new_temp().unwrap();

        // Target user has many relationships
        for i in 0..*rels_per_subject {
            store
                .add_relationship(Relationship::role(
                    "target-user",
                    "editor",
                    format!("doc-{}", i),
                    "admin",
                ))
                .unwrap();
        }

        // Add noise from other users
        for i in 0..1000 {
            store
                .add_relationship(Relationship::role(
                    format!("other-user-{}", i),
                    "viewer",
                    "doc-X",
                    "admin",
                ))
                .unwrap();
        }

        group.bench_with_input(
            BenchmarkId::from_parameter(rels_per_subject),
            rels_per_subject,
            |b, _| {
                b.iter(|| black_box(store.list_subject_relationships("target-user").unwrap()));
            },
        );
    }

    group.finish();
}

fn bench_complex_graph_traversal(c: &mut Criterion) {
    let mut group = c.benchmark_group("complex_graph_traversal");

    let store = RelationshipStore::new_temp().unwrap();

    // Build a complex graph with multiple paths
    //          root
    //         /    \
    //       int1    int2
    //       / \     / \
    //      c1  c2  c3  c4
    store.add_relationship(Relationship::trust("int1", "root", "pki")).unwrap();
    store.add_relationship(Relationship::trust("int2", "root", "pki")).unwrap();
    store.add_relationship(Relationship::trust("c1", "int1", "pki")).unwrap();
    store.add_relationship(Relationship::trust("c2", "int1", "pki")).unwrap();
    store.add_relationship(Relationship::trust("c3", "int2", "pki")).unwrap();
    store.add_relationship(Relationship::trust("c4", "int2", "pki")).unwrap();

    group.bench_function("find_path_2_hops", |b| {
        b.iter(|| black_box(store.find_relationship_path("c1", "trusted_by", "root").unwrap()))
    });

    group.bench_function("transitive_check_2_hops", |b| {
        b.iter(|| black_box(store.has_transitive_relationship("c1", "trusted_by", "root").unwrap()))
    });

    group.finish();
}

fn bench_role_lookups_vs_trust_chains(c: &mut Criterion) {
    let mut group = c.benchmark_group("role_vs_trust");

    let store = RelationshipStore::new_temp().unwrap();

    // Add 1000 role relationships (non-transitive)
    for i in 0..1000 {
        store
            .add_relationship(Relationship::role(
                format!("user-{}", i),
                "editor",
                format!("doc-{}", i),
                "admin",
            ))
            .unwrap();
    }

    // Add trust chain (transitive)
    for i in 0..5 {
        store
            .add_relationship(Relationship::trust(
                format!("cert-{}", i),
                format!("ca-{}", i),
                "pki",
            ))
            .unwrap();
    }

    group.bench_function("direct_role_lookup", |b| {
        b.iter(|| black_box(store.has_relationship("user-500", "editor", "doc-500").unwrap()))
    });

    group.bench_function("trust_chain_5_hops", |b| {
        b.iter(|| {
            black_box(store.has_transitive_relationship("cert-0", "trusted_by", "ca-4").unwrap())
        })
    });

    group.finish();
}

fn bench_with_expiration(c: &mut Criterion) {
    let mut group = c.benchmark_group("relationship_expiration");

    let store = RelationshipStore::new_temp().unwrap();

    // Mix of expired and valid relationships
    for i in 0..5000 {
        let mut rel = Relationship::role(format!("user-{}", i), "editor", "document-123", "admin");

        if i % 2 == 0 {
            // Half expired
            rel.expires_at = Some(chrono::Utc::now().timestamp() - 100);
        } else {
            // Half valid
            rel.expires_at = Some(chrono::Utc::now().timestamp() + 3600);
        }

        store.add_relationship(rel).unwrap();
    }

    group.bench_function("check_valid", |b| {
        b.iter(|| black_box(store.has_relationship("user-1", "editor", "document-123").unwrap()))
    });

    group.bench_function("check_expired", |b| {
        b.iter(|| black_box(store.has_relationship("user-0", "editor", "document-123").unwrap()))
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_relationship_add,
    bench_relationship_lookup,
    bench_transitive_relationship,
    bench_trust_chain_traversal,
    bench_membership_hierarchy,
    bench_batch_relationship_checks,
    bench_list_subject_relationships,
    bench_complex_graph_traversal,
    bench_role_lookups_vs_trust_chains,
    bench_with_expiration,
);

criterion_main!(benches);
