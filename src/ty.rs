//! The type term: variables, constructors, and their application.

use alloc::vec::Vec;
use core::fmt;

/// An inference variable — a placeholder for a type not yet known.
///
/// A `TyVar` is a 32-bit handle minted by [`Unifier::fresh`](crate::Unifier::fresh)
/// and stable for the life of that unifier. It is deliberately opaque: there is no
/// public constructor, so a variable can only be obtained from a unifier. Mint and
/// use variables in the same unifier — a handle is an index into that unifier's id
/// space, and a unifier treats an id it has not minted as an unbound variable, so a
/// handle is only meaningful against the unifier that produced it.
///
/// A variable starts life unbound — it stands for "some type" — and unification
/// may bind it to a concrete type or to another variable. Use
/// [`Unifier::resolve`](crate::Unifier::resolve) to read back what a variable has
/// become; a variable that is still unbound resolves to itself.
///
/// # Examples
///
/// ```
/// use type_lang::Unifier;
///
/// let mut unifier = Unifier::new();
/// let a = unifier.fresh();
/// let b = unifier.fresh();
///
/// // Variables are minted in order and stay distinct.
/// assert_eq!(a.to_u32(), 0);
/// assert_eq!(b.to_u32(), 1);
/// assert_ne!(a, b);
/// ```
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TyVar(u32);

impl TyVar {
    /// Wraps a raw index. Internal: only a [`Unifier`](crate::Unifier) mints
    /// variables, so that every variable in circulation belongs to a unifier that
    /// can resolve it.
    #[inline]
    pub(crate) const fn from_index(index: u32) -> Self {
        Self(index)
    }

    /// Returns the raw index this variable wraps.
    ///
    /// The value is the variable's creation order, starting at `0`. It is useful as
    /// a dense key into a side table of per-variable data; prefer passing the
    /// opaque `TyVar` itself wherever a handle will do.
    ///
    /// # Examples
    ///
    /// ```
    /// use type_lang::Unifier;
    ///
    /// let mut unifier = Unifier::new();
    /// let v = unifier.fresh();
    /// assert_eq!(v.to_u32(), 0);
    /// ```
    #[inline]
    #[must_use]
    pub const fn to_u32(self) -> u32 {
        self.0
    }
}

/// A type constructor — the name a concrete type is built from.
///
/// A `TyCon` is an opaque 32-bit tag that the *consumer* assigns meaning to. This
/// crate stores and compares constructors but never interprets them, so a language
/// front-end is free to map its own primitives and type formers onto whatever tags
/// it likes:
///
/// ```
/// use type_lang::TyCon;
///
/// const INT: TyCon = TyCon::new(0);
/// const BOOL: TyCon = TyCon::new(1);
/// const FUNCTION: TyCon = TyCon::new(2);
/// const LIST: TyCon = TyCon::new(3);
/// ```
///
/// Two constructors are the same type former when their tags are equal, so keeping
/// the tag assignment stable across a compilation is the caller's responsibility —
/// a `const` table, as above, is the usual way.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TyCon(u32);

impl TyCon {
    /// Creates a constructor from a tag.
    ///
    /// The tag identifies the type former; the caller chooses the numbering and is
    /// responsible for keeping it consistent. Equal tags are treated as the same
    /// constructor during unification.
    ///
    /// # Examples
    ///
    /// ```
    /// use type_lang::TyCon;
    ///
    /// let int = TyCon::new(0);
    /// assert_eq!(int.to_u32(), 0);
    /// ```
    #[inline]
    #[must_use]
    pub const fn new(tag: u32) -> Self {
        Self(tag)
    }

    /// Returns the raw tag this constructor wraps.
    #[inline]
    #[must_use]
    pub const fn to_u32(self) -> u32 {
        self.0
    }
}

/// A type term: either an inference variable or a constructor applied to arguments.
///
/// This is the structural core every concrete type is expressed in. A type is one
/// of two shapes:
///
/// - [`Var`](Self::Var) — an inference [`TyVar`], possibly still unbound.
/// - [`App`](Self::App) — a [`TyCon`] applied to zero or more argument types. A
///   primitive like `int` is the zero-argument case; a `List<int>` is `List`
///   applied to `int`; a function `(A) -> B` is some `->` constructor applied to
///   `A` and `B`.
///
/// The same two shapes describe primitives, generics, functions, tuples, and any
/// other first-order type former — the meaning of each constructor is the
/// consumer's to define. Build terms with the [`var`](Self::var),
/// [`con`](Self::con), and [`app`](Self::app) constructors.
///
/// # Examples
///
/// ```
/// use type_lang::{TyCon, Type, Unifier};
///
/// const INT: TyCon = TyCon::new(0);
/// const LIST: TyCon = TyCon::new(1);
///
/// let mut unifier = Unifier::new();
/// let element = unifier.fresh();
///
/// // List<?element>
/// let list_of = Type::app(LIST, [Type::var(element)]);
/// // List<int>
/// let list_int = Type::app(LIST, [Type::con(INT)]);
///
/// unifier.unify(&list_of, &list_int).expect("same constructor");
/// assert_eq!(unifier.resolve(&Type::var(element)), Type::con(INT));
/// ```
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Type {
    /// An inference variable.
    Var(TyVar),
    /// A constructor applied to its arguments. The argument list is empty for a
    /// nullary constructor such as a primitive.
    App(TyCon, Vec<Type>),
}

impl Type {
    /// Builds a variable type from a [`TyVar`].
    ///
    /// # Examples
    ///
    /// ```
    /// use type_lang::{Type, Unifier};
    ///
    /// let mut unifier = Unifier::new();
    /// let v = unifier.fresh();
    /// let ty = Type::var(v);
    /// assert_eq!(ty.as_var(), Some(v));
    /// ```
    #[inline]
    #[must_use]
    pub const fn var(var: TyVar) -> Self {
        Self::Var(var)
    }

    /// Builds a nullary constructor type, such as a primitive.
    ///
    /// This is the zero-argument case of [`app`](Self::app); `Type::con(c)` and
    /// `Type::app(c, [])` are the same term.
    ///
    /// # Examples
    ///
    /// ```
    /// use type_lang::{TyCon, Type};
    ///
    /// const UNIT: TyCon = TyCon::new(0);
    /// let ty = Type::con(UNIT);
    /// assert_eq!(ty.head(), Some(UNIT));
    /// assert!(ty.args().is_empty());
    /// ```
    #[inline]
    #[must_use]
    pub const fn con(head: TyCon) -> Self {
        Self::App(head, Vec::new())
    }

    /// Builds a constructor applied to a list of argument types.
    ///
    /// Accepts anything that turns into a `Vec<Type>` — an array, a `Vec`, or any
    /// iterator's `collect()` — so a fixed-arity type reads naturally:
    ///
    /// ```
    /// use type_lang::{TyCon, Type, Unifier};
    ///
    /// const FUNCTION: TyCon = TyCon::new(0);
    /// const INT: TyCon = TyCon::new(1);
    ///
    /// let mut unifier = Unifier::new();
    /// let result = unifier.fresh();
    ///
    /// // (int) -> ?result
    /// let signature = Type::app(FUNCTION, [Type::con(INT), Type::var(result)]);
    /// assert_eq!(signature.head(), Some(FUNCTION));
    /// assert_eq!(signature.args().len(), 2);
    /// ```
    #[inline]
    #[must_use]
    pub fn app(head: TyCon, args: impl Into<Vec<Type>>) -> Self {
        Self::App(head, args.into())
    }

    /// Returns the variable if this is a [`Var`](Self::Var), otherwise `None`.
    ///
    /// # Examples
    ///
    /// ```
    /// use type_lang::{TyCon, Type, Unifier};
    ///
    /// let mut unifier = Unifier::new();
    /// let v = unifier.fresh();
    /// assert_eq!(Type::var(v).as_var(), Some(v));
    /// assert_eq!(Type::con(TyCon::new(0)).as_var(), None);
    /// ```
    #[inline]
    #[must_use]
    pub const fn as_var(&self) -> Option<TyVar> {
        match self {
            Self::Var(v) => Some(*v),
            Self::App(..) => None,
        }
    }

    /// Returns the head constructor if this is an [`App`](Self::App), otherwise
    /// `None` (a variable has no constructor).
    ///
    /// # Examples
    ///
    /// ```
    /// use type_lang::{TyCon, Type, Unifier};
    ///
    /// const INT: TyCon = TyCon::new(0);
    /// let mut unifier = Unifier::new();
    ///
    /// assert_eq!(Type::con(INT).head(), Some(INT));
    /// assert_eq!(Type::var(unifier.fresh()).head(), None);
    /// ```
    #[inline]
    #[must_use]
    pub const fn head(&self) -> Option<TyCon> {
        match self {
            Self::App(head, _) => Some(*head),
            Self::Var(_) => None,
        }
    }

    /// Returns the argument types of an [`App`](Self::App), or an empty slice for a
    /// variable or a nullary constructor.
    ///
    /// # Examples
    ///
    /// ```
    /// use type_lang::{TyCon, Type};
    ///
    /// const PAIR: TyCon = TyCon::new(0);
    /// const INT: TyCon = TyCon::new(1);
    ///
    /// let pair = Type::app(PAIR, [Type::con(INT), Type::con(INT)]);
    /// assert_eq!(pair.args().len(), 2);
    /// assert!(Type::con(INT).args().is_empty());
    /// ```
    #[inline]
    #[must_use]
    pub fn args(&self) -> &[Type] {
        match self {
            Self::App(_, args) => args,
            Self::Var(_) => &[],
        }
    }

    /// Returns `true` if this term is an inference variable.
    #[inline]
    #[must_use]
    pub const fn is_var(&self) -> bool {
        matches!(self, Self::Var(_))
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Var(v) => write!(f, "?{}", v.to_u32()),
            Self::App(head, args) => {
                write!(f, "#{}", head.to_u32())?;
                if let [first, rest @ ..] = args.as_slice() {
                    write!(f, "({first}")?;
                    for arg in rest {
                        write!(f, ", {arg}")?;
                    }
                    write!(f, ")")?;
                }
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    extern crate alloc;
    use alloc::string::ToString;
    use alloc::vec;

    use super::*;

    const INT: TyCon = TyCon::new(0);
    const PAIR: TyCon = TyCon::new(5);

    #[test]
    fn test_con_is_app_with_no_arguments() {
        assert_eq!(Type::con(INT), Type::app(INT, vec![]));
    }

    #[test]
    fn test_accessors_split_var_from_app() {
        let v = TyVar::from_index(2);
        assert_eq!(Type::var(v).as_var(), Some(v));
        assert_eq!(Type::var(v).head(), None);
        assert!(Type::var(v).is_var());

        let pair = Type::app(PAIR, vec![Type::con(INT), Type::var(v)]);
        assert_eq!(pair.as_var(), None);
        assert_eq!(pair.head(), Some(PAIR));
        assert_eq!(pair.args().len(), 2);
        assert!(!pair.is_var());
    }

    #[test]
    fn test_display_renders_structure() {
        assert_eq!(Type::var(TyVar::from_index(4)).to_string(), "?4");
        assert_eq!(Type::con(INT).to_string(), "#0");
        let nested = Type::app(PAIR, vec![Type::con(INT), Type::var(TyVar::from_index(1))]);
        assert_eq!(nested.to_string(), "#5(#0, ?1)");
    }

    #[test]
    fn test_handles_work_as_map_keys() {
        use alloc::collections::BTreeMap;

        // TyVar and TyCon derive Ord + Hash, so they key ordered and hashed maps.
        let mut cons: BTreeMap<TyCon, u32> = BTreeMap::new();
        let _ = cons.insert(INT, 0);
        let _ = cons.insert(PAIR, 1);
        assert_eq!(cons.get(&PAIR), Some(&1));

        let mut vars: BTreeMap<TyVar, &str> = BTreeMap::new();
        let _ = vars.insert(TyVar::from_index(0), "a");
        let _ = vars.insert(TyVar::from_index(1), "b");
        assert_eq!(vars.get(&TyVar::from_index(1)), Some(&"b"));
    }
}
