//! Round-trip tests for the optional `serde` support on the type terms and the
//! unifier state. Only compiled when the `serde` feature is enabled.

#![cfg(feature = "serde")]
#![allow(clippy::unwrap_used)]

use type_lang::{TyCon, Type, Unifier};

const FUNCTION: TyCon = TyCon::new(0);
const INT: TyCon = TyCon::new(1);
const LIST: TyCon = TyCon::new(2);

#[test]
fn type_term_round_trips_through_json() {
    let mut u = Unifier::new();
    let v = u.fresh();

    // List<(int) -> ?v>
    let term = Type::app(LIST, [Type::app(FUNCTION, [Type::con(INT), Type::var(v)])]);

    let json = serde_json::to_string(&term).unwrap();
    let restored: Type = serde_json::from_str(&json).unwrap();
    assert_eq!(term, restored);
}

#[test]
fn nullary_constructor_round_trips() {
    let term = Type::con(INT);
    let json = serde_json::to_string(&term).unwrap();
    let restored: Type = serde_json::from_str(&json).unwrap();
    assert_eq!(restored, Type::con(INT));
    assert!(restored.args().is_empty());
}

#[test]
fn unifier_state_round_trips_and_keeps_its_bindings() {
    let mut u = Unifier::new();
    let a = u.fresh();
    let b = u.fresh();
    u.unify(&Type::var(a), &Type::app(LIST, [Type::var(b)]))
        .unwrap();
    u.unify(&Type::var(b), &Type::con(INT)).unwrap();

    let json = serde_json::to_string(&u).unwrap();
    let restored: Unifier = serde_json::from_str(&json).unwrap();

    // The substitution survives the round trip: ?a still resolves to List<int>.
    assert_eq!(restored.var_count(), u.var_count());
    assert_eq!(
        restored.resolve(&Type::var(a)),
        Type::app(LIST, [Type::con(INT)]),
    );
}
