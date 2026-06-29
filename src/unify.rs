//! The unifier: inference variables, their substitution, and unification.

use alloc::vec::Vec;

use crate::error::TypeError;
use crate::ty::{TyVar, Type};

/// Holds the inference variables of a type problem and the substitution that
/// unification builds over them.
///
/// A `Unifier` is the working state of type inference. It mints fresh
/// [`TyVar`]s with [`fresh`](Self::fresh), makes two types equal with
/// [`unify`](Self::unify) — recording the variable bindings that equality
/// requires — and reads a type back under those bindings with
/// [`resolve`](Self::resolve). A variable belongs to the unifier that minted it.
///
/// Unification is the standard first-order algorithm: two constructors match only
/// if their heads and arity agree, in which case their arguments are unified
/// pairwise; a variable unifies with any type by being bound to it, guarded by an
/// occurs check so a variable can never be bound to a type that contains it. The
/// substitution it produces is a most-general unifier — it constrains a variable
/// only as far as making the two types equal demands.
///
/// # Examples
///
/// ```
/// use type_lang::{TyCon, Type, Unifier};
///
/// const FUNCTION: TyCon = TyCon::new(0);
/// const INT: TyCon = TyCon::new(1);
///
/// let mut unifier = Unifier::new();
/// let arg = unifier.fresh();
/// let ret = unifier.fresh();
///
/// // Unify the inferred signature  (?arg) -> ?ret  with  (int) -> int.
/// let inferred = Type::app(FUNCTION, [Type::var(arg), Type::var(ret)]);
/// let known = Type::app(FUNCTION, [Type::con(INT), Type::con(INT)]);
/// unifier.unify(&inferred, &known).expect("same constructor and arity");
///
/// // Both variables are now known to be int.
/// assert_eq!(unifier.resolve(&Type::var(arg)), Type::con(INT));
/// assert_eq!(unifier.resolve(&Type::var(ret)), Type::con(INT));
/// ```
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, Default)]
pub struct Unifier {
    /// Indexed by [`TyVar`]: `None` is unbound, `Some(ty)` is the variable's
    /// current binding (which may itself be another variable).
    bindings: Vec<Option<Type>>,
}

impl Unifier {
    /// Creates an empty unifier with no variables.
    ///
    /// # Examples
    ///
    /// ```
    /// use type_lang::Unifier;
    ///
    /// let unifier = Unifier::new();
    /// assert!(unifier.is_empty());
    /// ```
    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        Self {
            bindings: Vec::new(),
        }
    }

    /// Creates an empty unifier with room for `vars` variables preallocated.
    ///
    /// A hint only: it sizes the internal table so that minting up to `vars`
    /// variables does not reallocate. Use it when the variable count is known up
    /// front — for instance, one per binding in the scope being checked.
    ///
    /// # Examples
    ///
    /// ```
    /// use type_lang::Unifier;
    ///
    /// let mut unifier = Unifier::with_capacity(8);
    /// for _ in 0..8 {
    ///     let _ = unifier.fresh();
    /// }
    /// assert_eq!(unifier.var_count(), 8);
    /// ```
    #[inline]
    #[must_use]
    pub fn with_capacity(vars: usize) -> Self {
        Self {
            bindings: Vec::with_capacity(vars),
        }
    }

    /// Mints a fresh, unbound inference variable.
    ///
    /// Variables are numbered in creation order from `0` and stay valid for the
    /// life of the unifier. A fresh variable stands for "some type not yet known";
    /// it gains a binding only when [`unify`](Self::unify) requires one.
    ///
    /// # Examples
    ///
    /// ```
    /// use type_lang::Unifier;
    ///
    /// let mut unifier = Unifier::new();
    /// let a = unifier.fresh();
    /// let b = unifier.fresh();
    /// assert_ne!(a, b);
    /// assert_eq!(unifier.var_count(), 2);
    /// ```
    #[inline]
    #[must_use]
    pub fn fresh(&mut self) -> TyVar {
        let index = self.bindings.len() as u32;
        self.bindings.push(None);
        TyVar::from_index(index)
    }

    /// Returns the number of variables this unifier has minted.
    #[inline]
    #[must_use]
    pub fn var_count(&self) -> usize {
        self.bindings.len()
    }

    /// Returns `true` if no variables have been minted.
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.bindings.is_empty()
    }

    /// Unifies two types, binding variables as needed to make them equal.
    ///
    /// On success the unifier's substitution is extended so that `a` and `b`
    /// [`resolve`](Self::resolve) to the same type. On failure the call returns a
    /// [`TypeError`] describing why, and **any bindings made before the conflict
    /// was reached are kept** — unification is not transactional. A caller that
    /// needs to try a unification speculatively should [`clone`](Clone::clone) the
    /// unifier first and keep the clone only if it succeeds.
    ///
    /// # Errors
    ///
    /// - [`TypeError::Mismatch`] if the two types are built from different
    ///   constructors or the same constructor at a different arity.
    /// - [`TypeError::Occurs`] if making them equal would require binding a
    ///   variable to a type that contains it (an infinite type).
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
    /// let v = unifier.fresh();
    ///
    /// // A variable unifies with a concrete type by being bound to it.
    /// unifier.unify(&Type::var(v), &Type::con(INT)).unwrap();
    /// assert_eq!(unifier.resolve(&Type::var(v)), Type::con(INT));
    ///
    /// // Two different constructors do not unify.
    /// let err = unifier.unify(&Type::con(INT), &Type::con(BOOL)).unwrap_err();
    /// assert!(matches!(err, TypeError::Mismatch { .. }));
    /// ```
    pub fn unify(&mut self, a: &Type, b: &Type) -> Result<(), TypeError> {
        let mut work = Vec::new();
        work.push((a.clone(), b.clone()));

        while let Some((lhs, rhs)) = work.pop() {
            let lhs = self.shallow(lhs);
            let rhs = self.shallow(rhs);
            match (lhs, rhs) {
                (Type::Var(x), Type::Var(y)) if x == y => {}
                (Type::Var(x), other) | (other, Type::Var(x)) => self.bind(x, other)?,
                (Type::App(head1, args1), Type::App(head2, args2)) => {
                    if head1 != head2 || args1.len() != args2.len() {
                        return Err(TypeError::Mismatch {
                            expected: self.resolve(&Type::App(head1, args1)),
                            found: self.resolve(&Type::App(head2, args2)),
                        });
                    }
                    work.extend(args1.into_iter().zip(args2));
                }
            }
        }
        Ok(())
    }

    /// Resolves a type fully under the current substitution.
    ///
    /// Every bound variable in `ty` is replaced by what it was bound to, all the
    /// way down, leaving a type whose only variables are still unbound. A type with
    /// no bound variables resolves to an equal copy of itself.
    ///
    /// The walk is proportional to the size of the resolved result, which — for a
    /// substitution that maps a variable to a type mentioning it twice — can be
    /// larger than the input. Resolve once you are done constraining a type, rather
    /// than after every step.
    ///
    /// # Examples
    ///
    /// ```
    /// use type_lang::{TyCon, Type, Unifier};
    ///
    /// const LIST: TyCon = TyCon::new(0);
    /// const INT: TyCon = TyCon::new(1);
    ///
    /// let mut unifier = Unifier::new();
    /// let element = unifier.fresh();
    /// let collection = unifier.fresh();
    ///
    /// // ?collection = List<?element>, then ?element = int.
    /// unifier
    ///     .unify(&Type::var(collection), &Type::app(LIST, [Type::var(element)]))
    ///     .unwrap();
    /// unifier.unify(&Type::var(element), &Type::con(INT)).unwrap();
    ///
    /// // Resolving the outer variable substitutes all the way down.
    /// assert_eq!(
    ///     unifier.resolve(&Type::var(collection)),
    ///     Type::app(LIST, [Type::con(INT)]),
    /// );
    ///
    /// // An unbound variable resolves to itself.
    /// let free = unifier.fresh();
    /// assert_eq!(unifier.resolve(&Type::var(free)), Type::var(free));
    /// ```
    #[must_use]
    pub fn resolve(&self, ty: &Type) -> Type {
        match ty {
            Type::Var(v) => match self.binding(*v) {
                Some(bound) => self.resolve(bound),
                None => Type::Var(*v),
            },
            Type::App(head, args) => {
                Type::App(*head, args.iter().map(|arg| self.resolve(arg)).collect())
            }
        }
    }

    /// Returns the direct binding of `var`, or `None` if it is unbound (or not from
    /// this unifier). The binding is shallow — it may itself be another variable.
    #[inline]
    fn binding(&self, var: TyVar) -> Option<&Type> {
        self.bindings.get(var.to_u32() as usize)?.as_ref()
    }

    /// Follows variable bindings at the top level until the term is either an
    /// unbound variable or a constructor. Does not descend into arguments.
    fn shallow(&self, mut ty: Type) -> Type {
        while let Type::Var(v) = ty {
            match self.binding(v) {
                Some(bound) => ty = bound.clone(),
                None => return Type::Var(v),
            }
        }
        ty
    }

    /// Binds an unbound variable to a type, after the occurs check.
    ///
    /// `var` is known to be unbound and `ty` is shallow-resolved (not a bound
    /// variable), because both come straight out of [`shallow`](Self::shallow).
    fn bind(&mut self, var: TyVar, ty: Type) -> Result<(), TypeError> {
        if self.occurs(var, &ty) {
            return Err(TypeError::Occurs {
                var,
                ty: self.resolve(&ty),
            });
        }
        // The slot exists for every variable this unifier minted; a variable from
        // another unifier is out of contract and is left untracked rather than
        // panicking.
        if let Some(slot) = self.bindings.get_mut(var.to_u32() as usize) {
            *slot = Some(ty);
        }
        Ok(())
    }

    /// Reports whether `var` occurs anywhere within `ty`, looking through bindings.
    ///
    /// Iterative, with an explicit work stack, so an arbitrarily deep type cannot
    /// overflow the call stack.
    fn occurs(&self, var: TyVar, ty: &Type) -> bool {
        let mut stack = Vec::new();
        stack.push(ty);
        while let Some(term) = stack.pop() {
            match term {
                Type::Var(other) => {
                    if *other == var {
                        return true;
                    }
                    if let Some(bound) = self.binding(*other) {
                        stack.push(bound);
                    }
                }
                Type::App(_, args) => stack.extend(args.iter()),
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    extern crate alloc;
    use alloc::vec;

    use super::*;
    use crate::ty::TyCon;

    const INT: TyCon = TyCon::new(0);
    const BOOL: TyCon = TyCon::new(1);
    const LIST: TyCon = TyCon::new(2);
    const PAIR: TyCon = TyCon::new(3);

    #[test]
    fn test_fresh_variables_are_distinct_and_counted() {
        let mut u = Unifier::new();
        let a = u.fresh();
        let b = u.fresh();
        assert_ne!(a, b);
        assert_eq!(u.var_count(), 2);
        assert!(!u.is_empty());
    }

    #[test]
    fn test_unify_variable_with_constructor_binds_it() {
        let mut u = Unifier::new();
        let v = u.fresh();
        u.unify(&Type::var(v), &Type::con(INT)).unwrap();
        assert_eq!(u.resolve(&Type::var(v)), Type::con(INT));
    }

    #[test]
    fn test_unify_propagates_through_a_variable_chain() {
        let mut u = Unifier::new();
        let x = u.fresh();
        let y = u.fresh();
        u.unify(&Type::var(x), &Type::var(y)).unwrap();
        u.unify(&Type::var(y), &Type::con(BOOL)).unwrap();
        // Binding y is visible through x.
        assert_eq!(u.resolve(&Type::var(x)), Type::con(BOOL));
    }

    #[test]
    fn test_unify_matching_constructors_unifies_arguments() {
        let mut u = Unifier::new();
        let a = u.fresh();
        let b = u.fresh();
        let lhs = Type::app(PAIR, vec![Type::var(a), Type::con(INT)]);
        let rhs = Type::app(PAIR, vec![Type::con(BOOL), Type::var(b)]);
        u.unify(&lhs, &rhs).unwrap();
        assert_eq!(u.resolve(&Type::var(a)), Type::con(BOOL));
        assert_eq!(u.resolve(&Type::var(b)), Type::con(INT));
    }

    #[test]
    fn test_unify_different_heads_is_mismatch() {
        let mut u = Unifier::new();
        let err = u.unify(&Type::con(INT), &Type::con(BOOL)).unwrap_err();
        assert!(matches!(err, TypeError::Mismatch { .. }));
    }

    #[test]
    fn test_unify_same_head_different_arity_is_mismatch() {
        let mut u = Unifier::new();
        let one = Type::app(PAIR, vec![Type::con(INT)]);
        let two = Type::app(PAIR, vec![Type::con(INT), Type::con(INT)]);
        let err = u.unify(&one, &two).unwrap_err();
        assert!(matches!(err, TypeError::Mismatch { .. }));
    }

    #[test]
    fn test_occurs_check_rejects_infinite_type() {
        let mut u = Unifier::new();
        let v = u.fresh();
        let recursive = Type::app(LIST, vec![Type::var(v)]);
        let err = u.unify(&Type::var(v), &recursive).unwrap_err();
        match err {
            TypeError::Occurs { var, .. } => assert_eq!(var, v),
            other => panic!("expected occurs error, got {other:?}"),
        }
    }

    #[test]
    fn test_occurs_check_sees_through_a_binding() {
        let mut u = Unifier::new();
        let x = u.fresh();
        let y = u.fresh();
        // y = List<x>, then x = y would make x = List<x>.
        u.unify(&Type::var(y), &Type::app(LIST, vec![Type::var(x)]))
            .unwrap();
        let err = u.unify(&Type::var(x), &Type::var(y)).unwrap_err();
        assert!(matches!(err, TypeError::Occurs { .. }));
    }

    #[test]
    fn test_unify_identical_terms_always_succeeds() {
        let mut u = Unifier::new();
        let v = u.fresh();
        let term = Type::app(PAIR, vec![Type::var(v), Type::con(INT)]);
        u.unify(&term, &term).unwrap();
    }

    #[test]
    fn test_resolve_is_idempotent() {
        let mut u = Unifier::new();
        let x = u.fresh();
        u.unify(&Type::var(x), &Type::app(LIST, vec![Type::con(INT)]))
            .unwrap();
        let once = u.resolve(&Type::var(x));
        let twice = u.resolve(&once);
        assert_eq!(once, twice);
    }

    #[test]
    fn test_default_matches_new() {
        let a = Unifier::default();
        let b = Unifier::new();
        assert_eq!(a.var_count(), b.var_count());
    }
}
