//! Integration tests: unification driven entirely through the public API, in the
//! shapes a real front-end produces — function application, generic instantiation,
//! and the failure modes a type checker reports.

#![allow(clippy::unwrap_used)]

use type_lang::{TyCon, Type, TypeError, Unifier};

// A consumer's constructor table. The tags are arbitrary but stable.
const INT: TyCon = TyCon::new(0);
const BOOL: TyCon = TyCon::new(1);
const STR: TyCon = TyCon::new(2);
const FUNCTION: TyCon = TyCon::new(10);
const LIST: TyCon = TyCon::new(11);
const PAIR: TyCon = TyCon::new(12);

/// `(a) -> b` as a function type.
fn func(arg: Type, ret: Type) -> Type {
    Type::app(FUNCTION, [arg, ret])
}

#[test]
fn checks_a_function_application() {
    // identity-style call: applying  (?a) -> ?a  to an int argument should make
    // both the parameter and the result int.
    let mut u = Unifier::new();
    let a = u.fresh();
    let identity = func(Type::var(a), Type::var(a));

    let result = u.fresh();
    let call = func(Type::con(INT), Type::var(result));

    u.unify(&identity, &call).unwrap();

    assert_eq!(u.resolve(&Type::var(a)), Type::con(INT));
    assert_eq!(u.resolve(&Type::var(result)), Type::con(INT));
}

#[test]
fn instantiates_a_generic_through_nested_unification() {
    // map : (?elem) -> ?out  applied over List<?elem> giving List<?out>; pin
    // ?elem to int and ?out to bool, then read the container types back.
    let mut u = Unifier::new();
    let elem = u.fresh();
    let out = u.fresh();

    let input = Type::app(LIST, [Type::var(elem)]);
    let output = Type::app(LIST, [Type::var(out)]);

    u.unify(&input, &Type::app(LIST, [Type::con(INT)])).unwrap();
    u.unify(&output, &Type::app(LIST, [Type::con(BOOL)]))
        .unwrap();

    assert_eq!(u.resolve(&Type::var(elem)), Type::con(INT));
    assert_eq!(u.resolve(&output), Type::app(LIST, [Type::con(BOOL)]));
}

#[test]
fn propagates_a_binding_across_independent_uses() {
    // Two separately built types share a variable; binding it once is visible in
    // both, the way a let-bound value's type flows to every use site.
    let mut u = Unifier::new();
    let shared = u.fresh();

    let first = Type::app(PAIR, [Type::var(shared), Type::con(STR)]);
    let second = Type::app(PAIR, [Type::var(shared), Type::con(BOOL)]);

    u.unify(&first, &Type::app(PAIR, [Type::con(INT), Type::con(STR)]))
        .unwrap();

    // `shared` is now int, observable through the second type as well.
    assert_eq!(
        u.resolve(&second),
        Type::app(PAIR, [Type::con(INT), Type::con(BOOL)]),
    );
}

#[test]
fn reports_argument_mismatch_with_resolved_types() {
    // A function expecting int is called with bool; the mismatch should carry the
    // concrete clashing types, not the raw variables.
    let mut u = Unifier::new();
    let param = u.fresh();
    let signature = func(Type::var(param), Type::con(STR));

    u.unify(&Type::var(param), &Type::con(INT)).unwrap();

    let call = func(Type::con(BOOL), Type::con(STR));
    let err = u.unify(&signature, &call).unwrap_err();

    match err {
        TypeError::Mismatch { expected, found } => {
            assert_eq!(expected, Type::con(INT));
            assert_eq!(found, Type::con(BOOL));
        }
        other => panic!("expected a mismatch, got {other:?}"),
    }
}

#[test]
fn rejects_a_self_referential_type() {
    // Unifying ?x with List<?x> would describe an infinite list-of-itself.
    let mut u = Unifier::new();
    let x = u.fresh();
    let recursive = Type::app(LIST, [Type::var(x)]);

    let err = u.unify(&Type::var(x), &recursive).unwrap_err();
    assert!(matches!(err, TypeError::Occurs { var, .. } if var == x));
}

#[test]
fn a_failed_unification_leaves_partial_bindings_in_place() {
    // unify is not transactional: when a compound unification binds a variable and
    // then hits a conflict, the binding made before the conflict is kept. Which
    // argument is visited first is unspecified, so we only assert that `a` ends up
    // bound — not to which type.
    let mut u = Unifier::new();
    let a = u.fresh();

    let lhs = Type::app(PAIR, [Type::var(a), Type::var(a)]);
    let rhs = Type::app(PAIR, [Type::con(INT), Type::con(BOOL)]);

    let err = u.unify(&lhs, &rhs).unwrap_err();
    assert!(matches!(err, TypeError::Mismatch { .. }));
    // `a` was bound partway through; the failure did not roll it back.
    assert!(!u.resolve(&Type::var(a)).is_var());
}

#[test]
fn cloning_a_unifier_isolates_a_speculative_unification() {
    // The documented way to try a unification without committing: clone, attempt,
    // discard on failure.
    let mut u = Unifier::new();
    let v = u.fresh();
    u.unify(&Type::var(v), &Type::con(INT)).unwrap();

    let mut trial = u.clone();
    assert!(trial.unify(&Type::var(v), &Type::con(BOOL)).is_err());

    // The original is untouched by the failed trial.
    assert_eq!(u.resolve(&Type::var(v)), Type::con(INT));
}
