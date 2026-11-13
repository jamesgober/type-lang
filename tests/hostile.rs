//! Inputs built to make unification slow or deep: bindings that share structure,
//! long chains of bindings, and errors raised at the end of such chains.
//!
//! Each test runs on a thread with a small stack, so recursion over a deep type
//! fails here instead of hiding behind a large main-thread stack.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use type_lang::{TyCon, TyVar, Type, TypeError, Unifier};

const PAIR: TyCon = TyCon::new(0);
const LIST: TyCon = TyCon::new(1);
const INT: TyCon = TyCon::new(2);
const BOOL: TyCon = TyCon::new(3);

fn on_small_stack(f: impl FnOnce() + Send + 'static) {
    std::thread::Builder::new()
        .stack_size(256 * 1024)
        .spawn(f)
        .unwrap()
        .join()
        .unwrap();
}

/// Drops a type of any depth without recursing.
fn drop_flat(ty: Type) {
    let mut stack = vec![ty];
    while let Some(term) = stack.pop() {
        if let Type::App(_, args) = term {
            stack.extend(args);
        }
    }
}

/// The deepest nesting of a type, measured without recursion.
fn depth_of(ty: &Type) -> usize {
    let mut deepest = 0;
    let mut stack = vec![(ty, 1usize)];
    while let Some((term, depth)) = stack.pop() {
        deepest = deepest.max(depth);
        for arg in term.args() {
            stack.push((arg, depth + 1));
        }
    }
    deepest
}

/// The number of nodes in a type, counted without recursion.
fn size_of(ty: &Type) -> usize {
    let mut nodes = 0;
    let mut stack = vec![ty];
    while let Some(term) = stack.pop() {
        nodes += 1;
        stack.extend(term.args());
    }
    nodes
}

/// Binds `vars[i] := Pair(vars[i + 1], vars[i + 1])`, so `vars[0]` stands for a
/// tree with `2^n` leaves held in `n` bindings.
fn doubling(u: &mut Unifier, n: usize) -> Vec<TyVar> {
    let vars: Vec<_> = (0..=n).map(|_| u.fresh()).collect();
    for i in 0..n {
        let next = Type::var(vars[i + 1]);
        u.unify(&Type::var(vars[i]), &Type::app(PAIR, [next.clone(), next]))
            .unwrap();
    }
    vars
}

/// Binds `vars[i] := List<vars[i + 1]>` for a chain of `n` bindings.
fn list_chain(u: &mut Unifier, n: usize) -> Vec<TyVar> {
    let vars: Vec<_> = (0..=n).map(|_| u.fresh()).collect();
    for i in 0..n {
        u.unify(
            &Type::var(vars[i]),
            &Type::app(LIST, [Type::var(vars[i + 1])]),
        )
        .unwrap();
    }
    vars
}

#[test]
fn test_occurs_check_on_shared_bindings_is_linear() {
    on_small_stack(|| {
        let mut u = Unifier::new();
        let vars = doubling(&mut u, 64);
        let z = u.fresh();
        // Binding z to the shared tree runs the occurs check over it.
        u.unify(&Type::var(z), &Type::var(vars[0])).unwrap();
        u.unify(&Type::var(vars[64]), &Type::con(INT)).unwrap();
    });
}

#[test]
fn test_occurs_error_on_shared_bindings_has_a_small_payload() {
    on_small_stack(|| {
        let mut u = Unifier::new();
        let vars = doubling(&mut u, 64);
        // The last variable would have to contain the whole tree, itself included.
        let err = u
            .unify(&Type::var(vars[64]), &Type::var(vars[0]))
            .unwrap_err();
        match err {
            TypeError::Occurs { var, ty } => {
                assert_eq!(var, vars[64]);
                assert!(size_of(&ty) <= 4096, "payload has {} nodes", size_of(&ty));
                // The payload is still the right type, just not fully expanded.
                assert_eq!(ty.head(), Some(PAIR));
            }
            other => panic!("expected an occurs error, got {other:?}"),
        }
    });
}

#[test]
fn test_unifying_two_shared_trees_is_linear() {
    on_small_stack(|| {
        let mut u = Unifier::new();
        let left = doubling(&mut u, 64);
        let right = doubling(&mut u, 64);
        u.unify(&Type::var(left[0]), &Type::var(right[0])).unwrap();
        u.unify(&Type::var(left[64]), &Type::con(INT)).unwrap();
        // Both leaves are now int, so unifying them with bool must fail.
        let err = u
            .unify(&Type::var(right[64]), &Type::con(BOOL))
            .unwrap_err();
        assert!(matches!(err, TypeError::Mismatch { .. }));
    });
}

#[test]
fn test_mismatch_at_the_end_of_a_deep_chain_does_not_overflow() {
    on_small_stack(|| {
        let mut u = Unifier::new();
        let vars = list_chain(&mut u, 200_000);
        let err = u.unify(&Type::var(vars[0]), &Type::con(INT)).unwrap_err();
        match &err {
            TypeError::Mismatch { expected, found } => {
                assert_eq!(expected.head(), Some(LIST));
                assert!(depth_of(expected) <= 128, "depth {}", depth_of(expected));
                assert_eq!(*found, Type::con(INT));
            }
            other => panic!("expected a mismatch, got {other:?}"),
        }
        // The payload is small enough to print, clone, compare and drop.
        let text = err.to_string();
        assert!(text.starts_with("type mismatch"));
        assert_eq!(err.clone(), err);
        drop(err);
    });
}

#[test]
fn test_occurs_error_at_the_end_of_a_deep_chain_does_not_overflow() {
    on_small_stack(|| {
        let mut u = Unifier::new();
        let vars = list_chain(&mut u, 200_000);
        let err = u
            .unify(&Type::var(vars[200_000]), &Type::var(vars[0]))
            .unwrap_err();
        match &err {
            TypeError::Occurs { ty, .. } => {
                assert!(depth_of(ty) <= 128, "depth {}", depth_of(ty));
            }
            other => panic!("expected an occurs error, got {other:?}"),
        }
        let _ = err.to_string();
    });
}

#[test]
fn test_resolve_of_a_deep_chain_does_not_overflow() {
    on_small_stack(|| {
        let mut u = Unifier::new();
        let vars = list_chain(&mut u, 200_000);
        u.unify(&Type::var(vars[200_000]), &Type::con(INT)).unwrap();
        let resolved = u.resolve(&Type::var(vars[0]));
        assert_eq!(depth_of(&resolved), 200_001);
        drop_flat(resolved);
    });
}

#[test]
fn test_display_of_a_deep_type_does_not_overflow() {
    on_small_stack(|| {
        let mut ty = Type::con(INT);
        for _ in 0..100_000 {
            ty = Type::app(LIST, [ty]);
        }
        let text = ty.to_string();
        assert_eq!(text.len(), "#1(".len() * 100_000 + "#2".len() + 100_000);
        assert!(text.starts_with("#1(#1("));
        drop_flat(ty);
    });
}

#[test]
fn test_long_variable_chain_resolves() {
    on_small_stack(|| {
        let mut u = Unifier::new();
        let vars: Vec<_> = (0..100_000).map(|_| u.fresh()).collect();
        for pair in vars.windows(2) {
            u.unify(&Type::var(pair[0]), &Type::var(pair[1])).unwrap();
        }
        u.unify(&Type::var(vars[0]), &Type::con(INT)).unwrap();
        for v in vars.iter().step_by(997) {
            assert_eq!(u.resolve(&Type::var(*v)), Type::con(INT));
        }
        // Unifying through the chain again is cheap and still correct.
        for v in vars.iter().step_by(13) {
            u.unify(&Type::var(*v), &Type::con(INT)).unwrap();
        }
        assert!(u.unify(&Type::var(vars[50_000]), &Type::con(BOOL)).is_err());
    });
}
