//! # type_lang
//!
//! The type-system substrate of a compiler front-end: a representation for types,
//! a [`Unifier`] that makes two types equal and records what that requires, and a
//! structured [`TypeError`] for when they cannot be made equal.
//!
//! It owns the soundness-critical core — type terms, inference variables, and
//! first-order unification — and nothing else. A language's own type rules (its
//! primitives, its subtyping, its coercions) layer on top by choosing what each
//! constructor means; this crate stores and compares those constructors but never
//! interprets them. It does no parsing and renders no diagnostics: it assigns types
//! to a syntax tree the caller walks, and a [`TypeError`] maps cleanly onto a
//! rendered diagnostic in the caller's own type names.
//!
//! ## The model
//!
//! A [`Type`] is one of two shapes: an inference [`TyVar`], or a [`TyCon`] applied
//! to zero or more argument types. Those two shapes describe every first-order
//! type — `int` is a constructor with no arguments, `List<int>` is `List` applied
//! to `int`, and `(int) -> bool` is a function constructor applied to `int` and
//! `bool`. Constructors are opaque tags the consumer assigns; equal tags are the
//! same type former.
//!
//! A [`Unifier`] holds the inference variables and the substitution over them. It
//! [`fresh`](Unifier::fresh)ens variables, [`unify`](Unifier::unify)es two types —
//! binding variables so the two become equal, or failing with a [`TypeError`] — and
//! [`resolve`](Unifier::resolve)s a type back to its most concrete known form. The
//! algorithm is textbook first-order unification: constructors match only on equal
//! head and arity, a variable binds to any type guarded by an occurs check, and the
//! result is a most-general unifier.
//!
//! ## Quickstart
//!
//! ```
//! use type_lang::{TyCon, Type, Unifier};
//!
//! // The consumer assigns meaning to constructor tags.
//! const FUNCTION: TyCon = TyCon::new(0);
//! const INT: TyCon = TyCon::new(1);
//! const BOOL: TyCon = TyCon::new(2);
//!
//! let mut unifier = Unifier::new();
//! let arg = unifier.fresh();
//! let ret = unifier.fresh();
//!
//! // We have inferred a call site needs a function  (?arg) -> ?ret,
//! // and the callee is known to be  (int) -> bool.
//! let needed = Type::app(FUNCTION, [Type::var(arg), Type::var(ret)]);
//! let callee = Type::app(FUNCTION, [Type::con(INT), Type::con(BOOL)]);
//! unifier.unify(&needed, &callee)?;
//!
//! // Unification has discovered the argument and result types.
//! assert_eq!(unifier.resolve(&Type::var(arg)), Type::con(INT));
//! assert_eq!(unifier.resolve(&Type::var(ret)), Type::con(BOOL));
//! # Ok::<(), type_lang::TypeError>(())
//! ```
//!
//! ## Stability
//!
//! The public surface is being designed across the 0.x series and frozen at `1.0`,
//! after which it follows Semantic Versioning. [`TypeError`] is `#[non_exhaustive]`
//! so new failure modes remain additive. See
//! [`docs/API.md`](https://github.com/jamesgober/type-lang/blob/main/docs/API.md)
//! and [`dev/ROADMAP.md`](https://github.com/jamesgober/type-lang/blob/main/dev/ROADMAP.md).

#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![deny(missing_docs)]
#![forbid(unsafe_code)]

extern crate alloc;

mod error;
mod ty;
mod unify;

pub use error::TypeError;
pub use ty::{TyCon, TyVar, Type};
pub use unify::Unifier;
