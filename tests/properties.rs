//! Property tests for the unification invariants in `dev/DIRECTIVES.md`: soundness,
//! symmetry, the occurs check, and idempotent resolution.
//!
//! Types are generated from an abstract skeleton and then realized against a pool
//! of real inference variables minted by the unifier under test — `TyVar` is
//! opaque, so the variables can only come from `fresh`.

#![allow(clippy::unwrap_used)]

use proptest::prelude::*;
use type_lang::{TyCon, Type, TypeError, Unifier};

/// An abstract type term. `Var(i)` refers to the `i`-th variable in a pool chosen
/// at realization time, so the same skeleton can be instantiated in two different
/// unifiers and produce structurally identical types.
#[derive(Clone, Debug)]
enum Skel {
    Var(usize),
    Con(u32),
    App(u32, Vec<Skel>),
}

/// A bounded strategy over skeletons: a handful of constructors and variable slots,
/// nested a few levels deep.
fn skel() -> impl Strategy<Value = Skel> {
    let leaf = prop_oneof![
        (0usize..6).prop_map(Skel::Var),
        (0u32..4).prop_map(Skel::Con),
    ];
    leaf.prop_recursive(4, 24, 3, |inner| {
        (0u32..4, prop::collection::vec(inner, 0..3)).prop_map(|(tag, args)| Skel::App(tag, args))
    })
}

/// Instantiates a skeleton into a real [`Type`], mapping each variable slot onto a
/// variable from `vars`. With no variables available, a slot degrades to a nullary
/// constructor so the term is still well-formed.
fn realize(skel: &Skel, vars: &[type_lang::TyVar]) -> Type {
    match skel {
        Skel::Var(i) => {
            if vars.is_empty() {
                Type::con(TyCon::new(0))
            } else {
                Type::var(vars[i % vars.len()])
            }
        }
        Skel::Con(tag) => Type::con(TyCon::new(*tag)),
        Skel::App(tag, args) => Type::app(
            TyCon::new(*tag),
            args.iter().map(|s| realize(s, vars)).collect::<Vec<_>>(),
        ),
    }
}

/// Builds a unifier with `n` fresh variables and returns it alongside the pool.
fn unifier_with_vars(n: usize) -> (Unifier, Vec<type_lang::TyVar>) {
    let mut u = Unifier::new();
    let vars = (0..n).map(|_| u.fresh()).collect();
    (u, vars)
}

proptest! {
    /// Soundness: if two types unify, they resolve to the same type under the
    /// resulting substitution. This is the contract a type checker depends on.
    #[test]
    fn unification_makes_types_equal(a in skel(), b in skel(), n in 1usize..6) {
        let (mut u, vars) = unifier_with_vars(n);
        let a = realize(&a, &vars);
        let b = realize(&b, &vars);
        if u.unify(&a, &b).is_ok() {
            prop_assert_eq!(u.resolve(&a), u.resolve(&b));
        }
    }

    /// Symmetry: argument order does not change whether two types unify.
    #[test]
    fn unification_is_symmetric(a in skel(), b in skel(), n in 1usize..6) {
        let (mut forward, vf) = unifier_with_vars(n);
        let (mut backward, vb) = unifier_with_vars(n);

        let af = realize(&a, &vf);
        let bf = realize(&b, &vf);
        let ab = realize(&a, &vb);
        let bb = realize(&b, &vb);

        prop_assert_eq!(forward.unify(&af, &bf).is_ok(), backward.unify(&bb, &ab).is_ok());
    }

    /// Idempotence: resolving an already-resolved type changes nothing, because a
    /// resolved type contains only unbound variables.
    #[test]
    fn resolution_is_idempotent(a in skel(), b in skel(), n in 1usize..6) {
        let (mut u, vars) = unifier_with_vars(n);
        let a = realize(&a, &vars);
        let b = realize(&b, &vars);
        let _ = u.unify(&a, &b);

        let once = u.resolve(&a);
        let twice = u.resolve(&once);
        prop_assert_eq!(once, twice);
    }

    /// Reflexivity: a type always unifies with itself, binding nothing it did not
    /// already require.
    #[test]
    fn a_type_unifies_with_itself(a in skel(), n in 1usize..6) {
        let (mut u, vars) = unifier_with_vars(n);
        let a = realize(&a, &vars);
        prop_assert!(u.unify(&a, &a).is_ok());
    }

    /// Occurs check: binding a variable to a constructor term that mentions it is
    /// always rejected, never allowed to build an infinite type.
    #[test]
    fn a_variable_never_unifies_inside_its_own_term(tag in 0u32..4, n in 1usize..4) {
        let (mut u, vars) = unifier_with_vars(n);
        let v = vars[0];
        // A constructor term that contains v at least once.
        let term = Type::app(TyCon::new(tag), [Type::var(v)]);
        let err = u.unify(&Type::var(v), &term).unwrap_err();
        let is_occurs = matches!(err, TypeError::Occurs { .. });
        prop_assert!(is_occurs);
    }
}
