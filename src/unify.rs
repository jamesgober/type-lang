//! The unifier: inference variables, their substitution, and unification.

use alloc::collections::BTreeSet;
use alloc::vec::Vec;

use crate::error::TypeError;
use crate::ty::{TyCon, TyVar, Type};

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
#[cfg_attr(feature = "serde", serde(try_from = "serde_repr::UnifierRepr"))]
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
    /// Variable ids are 32-bit, so a unifier addresses up to `u32::MAX` variables —
    /// far beyond any realistic inference problem, since the binding table alone
    /// would need tens of gigabytes long before the id space ran out. The bound is
    /// asserted in debug builds.
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
        debug_assert!(
            self.bindings.len() < u32::MAX as usize,
            "Unifier variable count exceeded the u32 id space",
        );
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
        // Pairs of variables already queued in this call. Bindings can share
        // structure (two variables bound to types that mention the same third
        // variable), and without this a shared pair would be unified once per
        // path to it, which grows exponentially with the depth of the sharing.
        // Skipping a repeat is sound: the first occurrence queued all the work
        // that makes the pair equal, and every queued item runs before `unify`
        // returns.
        let mut seen_pairs: BTreeSet<(TyVar, TyVar)> = BTreeSet::new();

        while let Some((lhs, rhs)) = work.pop() {
            if let (Type::Var(x), Type::Var(y)) = (&lhs, &rhs) {
                let pair = if x <= y { (*x, *y) } else { (*y, *x) };
                if !seen_pairs.insert(pair) {
                    continue;
                }
            }
            let lhs = self.shallow_compress(lhs);
            let rhs = self.shallow_compress(rhs);
            match (lhs, rhs) {
                (Type::Var(x), Type::Var(y)) if x == y => {}
                (Type::Var(x), other) | (other, Type::Var(x)) => self.bind(x, other)?,
                (Type::App(head1, args1), Type::App(head2, args2)) => {
                    if head1 != head2 || args1.len() != args2.len() {
                        return Err(TypeError::Mismatch {
                            expected: self.resolve_for_error(&Type::App(head1, args1)),
                            found: self.resolve_for_error(&Type::App(head2, args2)),
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
        self.resolve_within(ty, usize::MAX, usize::MAX)
    }

    /// Resolves a type for an error payload. Variables are substituted only
    /// while the result stays within [`ERROR_DEPTH`] levels and [`ERROR_NODES`]
    /// nodes; past that, a bound variable is left as it is. The payload stays
    /// correct (resolving it again gives the full type) and small enough to
    /// print, compare and drop, even when the full type is huge or very deep.
    fn resolve_for_error(&self, ty: &Type) -> Type {
        self.resolve_within(ty, ERROR_DEPTH, ERROR_NODES)
    }

    /// Resolves `ty`, substituting a bound variable only while the output is
    /// shallower than `max_depth` and has fewer than `max_nodes` nodes.
    /// Constructors already present in `ty` are always copied.
    ///
    /// Iterative: explicit stacks replace recursion, so a deep type cannot
    /// overflow the call stack here.
    fn resolve_within(&self, ty: &Type, max_depth: usize, max_nodes: usize) -> Type {
        enum Step<'a> {
            /// Resolve this term at this output depth.
            Visit(&'a Type, usize),
            /// Pop this many resolved arguments and apply the constructor.
            Build(TyCon, usize),
        }

        let mut steps = Vec::new();
        let mut out: Vec<Type> = Vec::new();
        let mut nodes = 0usize;
        steps.push(Step::Visit(ty, 0));

        while let Some(step) = steps.pop() {
            match step {
                Step::Visit(Type::Var(v), depth) => {
                    let expand = depth < max_depth && nodes < max_nodes;
                    match self.binding(*v) {
                        Some(bound) if expand => steps.push(Step::Visit(bound, depth)),
                        _ => {
                            nodes = nodes.saturating_add(1);
                            out.push(Type::Var(*v));
                        }
                    }
                }
                Step::Visit(Type::App(head, args), depth) => {
                    nodes = nodes.saturating_add(1);
                    steps.push(Step::Build(*head, args.len()));
                    // Pushed in reverse so the first argument is resolved first
                    // and the results land on `out` in order.
                    for arg in args.iter().rev() {
                        steps.push(Step::Visit(arg, depth.saturating_add(1)));
                    }
                }
                Step::Build(head, arity) => {
                    let args = out.split_off(out.len().saturating_sub(arity));
                    out.push(Type::App(head, args));
                }
            }
        }
        out.pop().unwrap_or_else(|| ty.clone())
    }

    /// Returns the direct binding of `var`, or `None` if it is unbound (or not from
    /// this unifier). The binding is shallow — it may itself be another variable.
    #[inline]
    fn binding(&self, var: TyVar) -> Option<&Type> {
        self.bindings.get(var.to_u32() as usize)?.as_ref()
    }

    /// Follows variable bindings at the top level until the term is either an
    /// unbound variable or a constructor. Does not descend into arguments.
    ///
    /// Along the way it shortens chains of variables bound to variables: every
    /// variable on the chain is rebound to the last variable of the chain, so a
    /// later walk from any of them takes at most two steps.
    fn shallow_compress(&mut self, ty: Type) -> Type {
        let Type::Var(start) = ty else {
            return ty;
        };

        // First pass: find the end of the chain of variable-to-variable links.
        let mut end = start;
        let mut links = 0usize;
        while let Some(Type::Var(next)) = self.binding(end) {
            end = *next;
            links += 1;
        }

        // Second pass: point every variable before the last link straight at
        // `end`. A chain of one link is already as short as it gets.
        if links > 1 {
            let mut v = start;
            while v != end {
                let next = match self.binding(v) {
                    Some(Type::Var(next)) => *next,
                    _ => break,
                };
                if next != end {
                    if let Some(slot) = self.bindings.get_mut(v.to_u32() as usize) {
                        *slot = Some(Type::Var(end));
                    }
                }
                v = next;
            }
        }

        match self.binding(end) {
            Some(bound) => bound.clone(),
            None => Type::Var(end),
        }
    }

    /// Binds an unbound variable to a type, after the occurs check.
    ///
    /// `var` is known to be unbound and `ty` is shallow-resolved (not a bound
    /// variable), because both come straight out of [`shallow`](Self::shallow).
    fn bind(&mut self, var: TyVar, ty: Type) -> Result<(), TypeError> {
        if self.occurs(var, &ty) {
            return Err(TypeError::Occurs {
                var,
                ty: self.resolve_for_error(&ty),
            });
        }
        // A variable this unifier has not minted (one passed in from elsewhere)
        // joins its id space here as a freshly bound variable. Growing the table
        // keeps `unify`'s postcondition sound — the binding is always recorded —
        // rather than dropping it and reporting a success that did not happen. In
        // normal single-unifier use the index is always in range and no growth
        // occurs.
        let index = var.to_u32() as usize;
        if index >= self.bindings.len() {
            self.bindings.resize(index + 1, None);
        }
        self.bindings[index] = Some(ty);
        Ok(())
    }

    /// Reports whether `var` occurs anywhere within `ty`, looking through bindings.
    ///
    /// Iterative, with an explicit work stack, so an arbitrarily deep type cannot
    /// overflow the call stack. Each bound variable's binding is searched at most
    /// once, so bindings that share structure cost time linear in their size, not
    /// in the number of paths through them.
    fn occurs(&self, var: TyVar, ty: &Type) -> bool {
        let mut stack = Vec::new();
        let mut searched: BTreeSet<TyVar> = BTreeSet::new();
        stack.push(ty);
        while let Some(term) = stack.pop() {
            match term {
                Type::Var(other) => {
                    if *other == var {
                        return true;
                    }
                    if let Some(bound) = self.binding(*other) {
                        if searched.insert(*other) {
                            stack.push(bound);
                        }
                    }
                }
                Type::App(_, args) => stack.extend(args.iter()),
            }
        }
        false
    }
}

/// How many levels deep an error payload's type may grow through substituted
/// variables.
const ERROR_DEPTH: usize = 64;

/// How many nodes an error payload's type may reach through substituted
/// variables.
const ERROR_NODES: usize = 1024;

#[cfg(feature = "serde")]
mod serde_repr {
    //! Deserialization goes through a plain mirror of [`Unifier`] and is
    //! checked before it becomes one, so restored state cannot hold a binding
    //! cycle that would make later calls loop forever.

    use alloc::vec::Vec;
    use core::fmt;

    use super::Unifier;
    use crate::ty::Type;

    /// The serialized shape of a [`Unifier`].
    #[derive(serde::Deserialize)]
    pub(super) struct UnifierRepr {
        bindings: Vec<Option<Type>>,
    }

    /// Restored bindings that refer back to themselves.
    #[derive(Debug)]
    pub(super) struct CyclicBindings;

    impl fmt::Display for CyclicBindings {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("unifier bindings contain a cycle")
        }
    }

    impl TryFrom<UnifierRepr> for Unifier {
        type Error = CyclicBindings;

        fn try_from(repr: UnifierRepr) -> Result<Self, Self::Error> {
            if has_cycle(&repr.bindings) {
                return Err(CyclicBindings);
            }
            Ok(Unifier {
                bindings: repr.bindings,
            })
        }
    }

    /// Pushes the bound variables that `ty` mentions directly onto `out`.
    fn bound_vars_in(ty: &Type, bindings: &[Option<Type>], out: &mut Vec<usize>) {
        let mut stack = Vec::new();
        stack.push(ty);
        while let Some(term) = stack.pop() {
            match term {
                Type::Var(v) => {
                    let index = v.to_u32() as usize;
                    if matches!(bindings.get(index), Some(Some(_))) {
                        out.push(index);
                    }
                }
                Type::App(_, args) => stack.extend(args.iter()),
            }
        }
    }

    /// Depth-first search for a cycle among bound variables. Iterative, so a
    /// long chain cannot overflow the call stack.
    fn has_cycle(bindings: &[Option<Type>]) -> bool {
        const NEW: u8 = 0;
        const OPEN: u8 = 1;

        let mut state = alloc::vec![NEW; bindings.len()];
        let mut stack: Vec<(usize, Vec<usize>)> = Vec::new();

        for (root, binding) in bindings.iter().enumerate() {
            let Some(ty) = binding else { continue };
            if state[root] != NEW {
                continue;
            }
            let mut children = Vec::new();
            bound_vars_in(ty, bindings, &mut children);
            state[root] = OPEN;
            stack.push((root, children));

            while let Some((node, children)) = stack.last_mut() {
                if let Some(child) = children.pop() {
                    match state[child] {
                        OPEN => return true,
                        NEW => {
                            let mut grand = Vec::new();
                            if let Some(Some(ty)) = bindings.get(child) {
                                bound_vars_in(ty, bindings, &mut grand);
                            }
                            state[child] = OPEN;
                            stack.push((child, grand));
                        }
                        _ => {}
                    }
                } else {
                    // Finished: every path from here is known to be acyclic.
                    state[*node] = 2;
                    stack.pop();
                }
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

    #[test]
    fn test_two_unbound_variables_share_a_representative() {
        let mut u = Unifier::new();
        let x = u.fresh();
        let y = u.fresh();
        u.unify(&Type::var(x), &Type::var(y)).unwrap();
        // Neither is bound to a constructor, but both now resolve to one variable.
        assert_eq!(u.resolve(&Type::var(x)), u.resolve(&Type::var(y)));
    }

    #[test]
    fn test_variable_from_another_unifier_binds_soundly() {
        // Regression: a variable this unifier did not mint must still be recorded
        // when bound — `unify` may never report success while leaving the two sides
        // unequal. The variable joins this unifier's id space.
        let mut origin = Unifier::new();
        let v = origin.fresh(); // belongs to `origin`

        let mut other = Unifier::new(); // has no variables of its own yet
        other.unify(&Type::var(v), &Type::con(INT)).unwrap();

        // The postcondition holds: both sides resolve to the same type.
        assert_eq!(other.resolve(&Type::var(v)), Type::con(INT));
        assert_eq!(other.var_count(), 1);
    }

    #[test]
    fn test_resolve_handles_a_deep_chain() {
        // v0 = List<v1>, v1 = List<v2>, ..., v_{n-1} = int. Resolving v0 nests the
        // whole chain; check it terminates correctly without misbehaving.
        let mut u = Unifier::new();
        let depth = 1_000usize;
        let vars: Vec<_> = (0..depth).map(|_| u.fresh()).collect();
        for window in vars.windows(2) {
            u.unify(
                &Type::var(window[0]),
                &Type::app(LIST, vec![Type::var(window[1])]),
            )
            .unwrap();
        }
        u.unify(&Type::var(vars[depth - 1]), &Type::con(INT))
            .unwrap();

        let resolved = u.resolve(&Type::var(vars[0]));

        // Walk the resolved structure iteratively: depth-1 List wrappers, then int.
        let mut cur = &resolved;
        let mut wrappers = 0usize;
        while cur.head() == Some(LIST) {
            wrappers += 1;
            cur = &cur.args()[0];
        }
        assert_eq!(wrappers, depth - 1);
        assert_eq!(*cur, Type::con(INT));
    }
}
