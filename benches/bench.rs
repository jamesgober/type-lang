//! Benchmarks for the core operations: minting variables, unifying nested types,
//! and resolving a substituted type.
//!
//! Unification is the operation a type checker runs at every constrained node, so
//! its cost on progressively larger type terms is what the design exists to keep in
//! check. The terms are balanced binary trees of a `Pair` constructor; a tree of
//! depth `d` has `2^d` leaves, so the structural work grows with the term size.

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use std::hint::black_box;
use type_lang::{TyCon, Type, Unifier};

const PAIR: TyCon = TyCon::new(0);
const INT: TyCon = TyCon::new(1);

/// A balanced `Pair` tree with a fresh variable at every leaf.
fn var_tree(u: &mut Unifier, depth: u32) -> Type {
    if depth == 0 {
        Type::var(u.fresh())
    } else {
        Type::app(PAIR, [var_tree(u, depth - 1), var_tree(u, depth - 1)])
    }
}

/// A balanced `Pair` tree of the same shape with `int` at every leaf.
fn con_tree(depth: u32) -> Type {
    if depth == 0 {
        Type::con(INT)
    } else {
        Type::app(PAIR, [con_tree(depth - 1), con_tree(depth - 1)])
    }
}

fn bench_fresh(c: &mut Criterion) {
    let mut group = c.benchmark_group("fresh");
    for &count in &[16usize, 256, 4096] {
        group.bench_with_input(BenchmarkId::from_parameter(count), &count, |b, &count| {
            b.iter(|| {
                let mut u = Unifier::with_capacity(count);
                for _ in 0..count {
                    black_box(u.fresh());
                }
            });
        });
    }
    group.finish();
}

fn bench_unify(c: &mut Criterion) {
    let mut group = c.benchmark_group("unify");
    for &depth in &[4u32, 6, 8] {
        let leaves = 1usize << depth;
        group.bench_with_input(BenchmarkId::from_parameter(leaves), &depth, |b, &depth| {
            b.iter_batched(
                || {
                    let mut u = Unifier::with_capacity(leaves);
                    let vars = var_tree(&mut u, depth);
                    let cons = con_tree(depth);
                    (u, vars, cons)
                },
                |(mut u, vars, cons)| {
                    u.unify(black_box(&vars), black_box(&cons))
                        .expect("unifies");
                    black_box(&u);
                },
                criterion::BatchSize::SmallInput,
            );
        });
    }
    group.finish();
}

fn bench_resolve(c: &mut Criterion) {
    let mut group = c.benchmark_group("resolve");
    for &depth in &[4u32, 6, 8] {
        let leaves = 1usize << depth;
        // Bind every leaf variable to int, then measure resolving the whole tree.
        let mut u = Unifier::with_capacity(leaves);
        let vars = var_tree(&mut u, depth);
        let cons = con_tree(depth);
        u.unify(&vars, &cons).expect("unifies");

        group.bench_with_input(BenchmarkId::from_parameter(leaves), &depth, |b, _| {
            b.iter(|| black_box(u.resolve(black_box(&vars))));
        });
    }
    group.finish();
}

criterion_group!(benches, bench_fresh, bench_unify, bench_resolve);
criterion_main!(benches);
