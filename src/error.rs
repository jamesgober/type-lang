//! The error returned when two types cannot be unified.

use core::fmt;

use crate::ty::{TyVar, Type};

/// The reason a [`unify`](crate::Unifier::unify) call failed.
///
/// Unification fails in exactly two ways, each a distinct, defined outcome rather
/// than a panic:
///
/// - the two types are built from different constructors, or the same constructor
///   applied to a different number of arguments ([`Mismatch`](Self::Mismatch)), or
/// - binding a variable would make it refer to itself, producing an infinite type
///   ([`Occurs`](Self::Occurs)).
///
/// Both variants carry the offending types, fully resolved under the substitution
/// at the point of failure, so the failure is actionable when it is rendered far
/// from the call that produced it — typically by mapping it onto a `diag-lang`
/// diagnostic with the consumer's own type names.
///
/// The enum is `#[non_exhaustive]`: a downstream `match` must include a wildcard
/// arm, so a later addition is a minor change, not a breaking one.
///
/// # Examples
///
/// ```
/// use type_lang::{TyCon, Type, TypeError, Unifier};
///
/// const INT: TyCon = TyCon::new(0);
/// const BOOL: TyCon = TyCon::new(1);
///
/// let mut unifier = Unifier::new();
/// let err = unifier
///     .unify(&Type::con(INT), &Type::con(BOOL))
///     .unwrap_err();
/// assert!(matches!(err, TypeError::Mismatch { .. }));
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum TypeError {
    /// The two types do not share a constructor.
    ///
    /// Returned when the heads differ (`Int` versus `Bool`) or when the same head
    /// is applied to a different arity (`Pair<A>` versus `Pair<A, B>`). Both sides
    /// are resolved under the current substitution before being stored, so they
    /// show the most concrete form known at the point of failure.
    ///
    /// `unify(a, b)` records `a` as `expected` and `b` as `found`. Unification
    /// itself is symmetric — the labels only reflect the argument order, to match
    /// the common "expected this, found that" phrasing of a type error.
    Mismatch {
        /// The first type given to `unify`, resolved.
        expected: Type,
        /// The second type given to `unify`, resolved.
        found: Type,
    },

    /// Binding a variable would make it occur within its own definition.
    ///
    /// Returned when a variable would be bound to a type that already contains it —
    /// for example unifying `?0` with `List<?0>`. Such a binding describes an
    /// infinitely deep type, which the occurs check rejects so that resolution
    /// always terminates.
    Occurs {
        /// The variable that would refer to itself.
        var: TyVar,
        /// The type it would have been bound to, resolved.
        ty: Type,
    },
}

impl fmt::Display for TypeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Mismatch { expected, found } => {
                write!(f, "type mismatch: expected `{expected}`, found `{found}`")
            }
            Self::Occurs { var, ty } => {
                write!(
                    f,
                    "recursive type: variable `?{}` occurs in `{ty}`",
                    var.to_u32(),
                )
            }
        }
    }
}

impl core::error::Error for TypeError {}

#[cfg(test)]
mod tests {
    extern crate alloc;
    use alloc::string::ToString;
    use alloc::vec;

    use super::*;
    use crate::ty::TyCon;

    const INT: TyCon = TyCon::new(0);
    const LIST: TyCon = TyCon::new(7);

    #[test]
    fn test_mismatch_display_names_both_types() {
        let err = TypeError::Mismatch {
            expected: Type::con(INT),
            found: Type::app(LIST, vec![Type::con(INT)]),
        };
        let text = err.to_string();
        assert!(text.contains("#0"), "{text}");
        assert!(text.contains("#7(#0)"), "{text}");
    }

    #[test]
    fn test_occurs_display_names_variable() {
        let err = TypeError::Occurs {
            var: TyVar::from_index(3),
            ty: Type::app(LIST, vec![Type::var(TyVar::from_index(3))]),
        };
        let text = err.to_string();
        assert!(text.contains("?3"), "{text}");
    }

    #[test]
    fn test_error_is_clonable_and_equatable() {
        let a = TypeError::Mismatch {
            expected: Type::con(INT),
            found: Type::con(LIST),
        };
        let b = a.clone();
        assert_eq!(a, b);
    }
}
