# type-lang &mdash; API Reference

> Complete reference for every public item in `type-lang`, with examples.
> **Status: stable (1.0).** The surface below is the `1.0` contract; it follows
> [Semantic Versioning](#stability) and will not change in a breaking way before
> `2.0`. See [`../dev/ROADMAP.md`](../dev/ROADMAP.md).

## Table of Contents

- [Overview](#overview)
- [Stability](#stability)
- [Installation](#installation)
- [Quick start](#quick-start)
- [The model](#the-model)
- [`Type`](#type)
  - [`Type::var`](#typevar)
  - [`Type::con`](#typecon)
  - [`Type::app`](#typeapp)
  - [`Type::as_var`](#typeas_var)
  - [`Type::head`](#typehead)
  - [`Type::args`](#typeargs)
  - [`Type::is_var`](#typeis_var)
  - [`Display`](#type-display)
- [`TyVar`](#tyvar)
- [`TyCon`](#tycon)
- [`Unifier`](#unifier)
  - [`Unifier::new`](#unifiernew)
  - [`Unifier::with_capacity`](#unifierwith_capacity)
  - [`Unifier::fresh`](#unifierfresh)
  - [`Unifier::unify`](#unifierunify)
  - [`Unifier::resolve`](#unifierresolve)
  - [`Unifier::var_count` / `is_empty`](#unifiervar_count--is_empty)
- [`TypeError`](#typeerror)
- [Serialization](#serialization)
- [Feature flags](#feature-flags)

---

## Overview

type-lang is the type-system substrate of a compiler front-end. It provides a
representation for types ([`Type`](#type)), a [`Unifier`](#unifier) that makes two
types equal and records the variable bindings that requires, and a structured
[`TypeError`](#typeerror) for when two types cannot be made equal.

It owns the soundness-critical core — type terms, inference variables, and
first-order unification — and nothing else. A language's own type rules (its
primitives, its subtyping, its coercions) layer on top by choosing what each
constructor means; this crate stores and compares constructors but never
interprets them. It does no parsing and renders no diagnostics.

---

## Stability

As of `1.0.0` the public API documented here is **stable**. The crate follows
[Semantic Versioning](https://semver.org):

- No item in this reference will be **removed or changed in a breaking way** within
  the `1.x` series. Breaking changes wait for `2.0`.
- New functionality arrives in **minor** releases (`1.1`, `1.2`, …) and is additive.
  [`TypeError`](#typeerror) is `#[non_exhaustive]`, so a new failure variant is a
  minor change, not a breaking one — match it with a wildcard arm.
- Bug fixes, documentation, and internal optimisation are **patch** releases.
  Internal representation (how the substitution is stored, whether resolution is
  path-compressed) is not part of the contract and may change in a patch.
- The **MSRV** is Rust `1.85`. Raising it is treated as a minor change and called
  out in the changelog; it is never a patch.
- The **`serde` wire format** of `Type`, `TyVar`, `TyCon`, and `Unifier` is part of
  the contract: a value serialised by one `1.x` reads back in any later `1.x`.

Anything not in this reference — private fields, exact `Debug` output, the
structural `Display` notation — is not part of the contract and may change.

---

## Installation

```toml
[dependencies]
type-lang = "1.0"
```

Or from the terminal:

```bash
cargo add type-lang
```

The crate is `no_std`-friendly: it needs `alloc` but not the full standard library.
With `default-features = false` it stays `no_std`, and the entire API works
unchanged. See [Feature flags](#feature-flags).

---

## Quick start

```rust
use type_lang::{TyCon, Type, Unifier};

// The consumer assigns meaning to constructor tags.
const FUNCTION: TyCon = TyCon::new(0);
const INT: TyCon = TyCon::new(1);
const BOOL: TyCon = TyCon::new(2);

let mut unifier = Unifier::new();
let arg = unifier.fresh();
let ret = unifier.fresh();

// A call site needs a function (?arg) -> ?ret; the callee is (int) -> bool.
let needed = Type::app(FUNCTION, [Type::var(arg), Type::var(ret)]);
let callee = Type::app(FUNCTION, [Type::con(INT), Type::con(BOOL)]);
unifier.unify(&needed, &callee)?;

// Unification has discovered both argument and result types.
assert_eq!(unifier.resolve(&Type::var(arg)), Type::con(INT));
assert_eq!(unifier.resolve(&Type::var(ret)), Type::con(BOOL));
# Ok::<(), type_lang::TypeError>(())
```

---

## The model

A [`Type`](#type) is one of two shapes:

- an inference variable [`TyVar`](#tyvar), or
- a constructor [`TyCon`](#tycon) applied to zero or more argument types.

Those two shapes describe every first-order type. A primitive such as `int` is a
constructor with no arguments; `List<int>` is the `List` constructor applied to
`int`; a function `(int) -> bool` is some function constructor applied to `int` and
`bool`. Constructors are opaque numeric tags the consumer assigns and keeps stable;
two constructors are the same type former exactly when their tags are equal.

A [`Unifier`](#unifier) holds the inference variables and the substitution over
them. It mints variables, unifies two types — binding variables so they become
equal, or failing — and resolves a type back to its most concrete known form.
Unification is the textbook first-order algorithm: two constructors match only on
equal head and arity, a variable binds to any type guarded by an occurs check, and
the substitution produced is a most-general unifier.

---

## `Type`

```rust
pub enum Type {
    Var(TyVar),
    App(TyCon, Vec<Type>),
}
```

A type term: either an inference variable or a constructor applied to its arguments.
The two variants are public, so a consumer can `match` on a type directly, and the
constructor/accessor methods below cover the common cases without a `match`.

`Type` derives `Clone`, `Debug`, `PartialEq`, `Eq`, and `Hash`. Two types are equal
when they are structurally identical; equality does **not** consult any unifier, so
compare resolved types if you want equality "up to the current substitution"
(resolve both first — see [`resolve`](#unifierresolve)).

```rust
use type_lang::{TyCon, Type};

const INT: TyCon = TyCon::new(0);
const LIST: TyCon = TyCon::new(1);

// List<int>, built two equivalent ways.
let a = Type::app(LIST, [Type::con(INT)]);
let b = Type::app(LIST, [Type::app(INT, [])]);
assert_eq!(a, b);
```

### `Type::var`

```rust
pub const fn var(var: TyVar) -> Type
```

Wraps an inference variable as a type. The variable must come from a
[`Unifier`](#unifier) (see [`fresh`](#unifierfresh)).

**Parameters**

- `var` — a [`TyVar`](#tyvar) handle.

```rust
use type_lang::{Type, Unifier};

let mut unifier = Unifier::new();
let v = unifier.fresh();
let ty = Type::var(v);
assert!(ty.is_var());
```

### `Type::con`

```rust
pub const fn con(head: TyCon) -> Type
```

Builds a nullary constructor type — a primitive. It is the zero-argument case of
[`app`](#typeapp): `Type::con(c)` and `Type::app(c, [])` are the same term. The
function is `const`, so a primitive can initialise a `const` or `static`, and it
allocates nothing.

**Parameters**

- `head` — the constructor [`TyCon`](#tycon).

```rust
use type_lang::{TyCon, Type};

const UNIT: TyCon = TyCon::new(0);
const INT: TyCon = TyCon::new(1);

// A `const` table of primitives costs no allocation.
const UNIT_TY: Type = Type::con(UNIT);

assert_eq!(Type::con(INT).head(), Some(INT));
assert!(UNIT_TY.args().is_empty());
```

### `Type::app`

```rust
pub fn app(head: TyCon, args: impl Into<Vec<Type>>) -> Type
```

Builds a constructor applied to a list of argument types.

**Parameters**

- `head` — the constructor [`TyCon`](#tycon).
- `args` — the argument types. Anything that converts into a `Vec<Type>`: a fixed
  array `[a, b]`, a `Vec`, or an iterator's `collect()`. An empty list is the same
  as [`con`](#typecon).

```rust
use type_lang::{TyCon, Type, Unifier};

const FUNCTION: TyCon = TyCon::new(0);
const INT: TyCon = TyCon::new(1);

let mut unifier = Unifier::new();
let ret = unifier.fresh();

// (int) -> ?ret, from a fixed array.
let signature = Type::app(FUNCTION, [Type::con(INT), Type::var(ret)]);
assert_eq!(signature.head(), Some(FUNCTION));
assert_eq!(signature.args().len(), 2);

// The same type built from a Vec computed at runtime.
let parts = vec![Type::con(INT), Type::var(ret)];
assert_eq!(Type::app(FUNCTION, parts), signature);
```

### `Type::as_var`

```rust
pub const fn as_var(&self) -> Option<TyVar>
```

Returns the variable if this term is a [`Var`](#type), otherwise `None`. Handy after
[`resolve`](#unifierresolve) to test whether a type is still an unbound variable
(a free type variable in the result).

```rust
use type_lang::{TyCon, Type, Unifier};

let mut unifier = Unifier::new();
let v = unifier.fresh();

assert_eq!(Type::var(v).as_var(), Some(v));
assert_eq!(Type::con(TyCon::new(0)).as_var(), None);

// After resolving, a still-unbound variable is a free type variable.
assert!(unifier.resolve(&Type::var(v)).as_var().is_some());
```

### `Type::head`

```rust
pub const fn head(&self) -> Option<TyCon>
```

Returns the head constructor of an [`App`](#type), or `None` for a variable (a
variable has no constructor).

```rust
use type_lang::{TyCon, Type, Unifier};

const INT: TyCon = TyCon::new(0);
let mut unifier = Unifier::new();

assert_eq!(Type::con(INT).head(), Some(INT));
assert_eq!(Type::var(unifier.fresh()).head(), None);
```

### `Type::args`

```rust
pub fn args(&self) -> &[Type]
```

Returns the argument types of an [`App`](#type), borrowed. A variable and a nullary
constructor both yield an empty slice, so this never panics and needs no `match`.

```rust
use type_lang::{TyCon, Type};

const PAIR: TyCon = TyCon::new(0);
const INT: TyCon = TyCon::new(1);

let pair = Type::app(PAIR, [Type::con(INT), Type::con(INT)]);
assert_eq!(pair.args().len(), 2);
assert!(Type::con(INT).args().is_empty());
```

### `Type::is_var`

```rust
pub const fn is_var(&self) -> bool
```

Returns `true` if this term is an inference variable.

```rust
use type_lang::{TyCon, Type, Unifier};

let mut unifier = Unifier::new();
assert!(Type::var(unifier.fresh()).is_var());
assert!(!Type::con(TyCon::new(0)).is_var());
```

### `Display` { #type-display }

`Type` implements `Display` with a compact structural rendering, useful for
debugging and as a last-resort fallback when the consumer has no names of its own:

- a variable as `?n` (its index),
- a nullary constructor as `#tag`, and
- an applied constructor as `#tag(arg, …)`.

```rust
use type_lang::{TyCon, Type};

const PAIR: TyCon = TyCon::new(5);
const INT: TyCon = TyCon::new(0);

let ty = Type::app(PAIR, [Type::con(INT), Type::app(PAIR, [])]);
assert_eq!(ty.to_string(), "#5(#0, #5)");
```

For user-facing diagnostics, map `Type` onto your own type names rather than showing
the tags directly.

---

## `TyVar`

```rust
pub struct TyVar(/* private */);

pub const fn to_u32(self) -> u32
```

A small, copyable handle to one inference variable. A `TyVar` is a 32-bit index
minted by [`Unifier::fresh`](#unifierfresh) and stable for the life of that unifier.
It is deliberately opaque — there is no public constructor — so a variable can only
come from the unifier that tracks it, and that unifier always knows how to resolve
it. Use a `TyVar` from the unifier that minted it; another unifier does not track it.

`to_u32` exposes the raw creation-order index (starting at `0`), useful as a dense
key into a side table of per-variable data.

`TyVar` derives `Clone`, `Copy`, `Debug`, `PartialEq`, `Eq`, `PartialOrd`, `Ord`,
and `Hash`, so it works as a `HashMap` / `BTreeMap` key.

```rust
use type_lang::Unifier;

let mut unifier = Unifier::new();
let a = unifier.fresh();
let b = unifier.fresh();

assert_eq!(a.to_u32(), 0);
assert_eq!(b.to_u32(), 1);
assert_ne!(a, b);
```

---

## `TyCon`

```rust
pub struct TyCon(/* private */);

pub const fn new(tag: u32) -> TyCon
pub const fn to_u32(self) -> u32
```

A type constructor — the name a concrete type is built from. A `TyCon` is an opaque
32-bit tag that the **consumer** assigns meaning to; this crate stores and compares
constructors but never interprets them. A language front-end maps its own primitives
and type formers onto whatever tags it likes, usually in a `const` table.

**`new`** wraps a tag; **`to_u32`** returns it. Two constructors are the same type
former exactly when their tags are equal, so keep the tag assignment stable across a
compilation.

`TyCon` derives `Clone`, `Copy`, `Debug`, `PartialEq`, `Eq`, `PartialOrd`, `Ord`,
and `Hash`.

```rust
use type_lang::TyCon;

// A consumer's constructor table.
const INT: TyCon = TyCon::new(0);
const BOOL: TyCon = TyCon::new(1);
const FUNCTION: TyCon = TyCon::new(2);

assert_eq!(INT.to_u32(), 0);
assert_ne!(INT, BOOL);
```

---

## `Unifier`

```rust
pub struct Unifier { /* private */ }
```

Holds the inference variables of a type problem and the substitution that
unification builds over them. This is the type you construct, mint variables from,
and unify against. It derives `Clone`, `Debug`, and `Default` (`Default` is
[`new`](#unifiernew)).

Cloning a `Unifier` is the supported way to try a unification speculatively: clone,
[`unify`](#unifierunify) on the clone, and keep it only if it succeeds — unification
is not transactional and a failed call leaves partial bindings behind.

```rust
use type_lang::{TyCon, Type, Unifier};

const INT: TyCon = TyCon::new(0);

let mut unifier = Unifier::new();
let v = unifier.fresh();
unifier.unify(&Type::var(v), &Type::con(INT)).expect("binds");
assert_eq!(unifier.resolve(&Type::var(v)), Type::con(INT));
```

### `Unifier::new`

```rust
pub const fn new() -> Unifier
```

Creates an empty unifier with no variables. `const`, so it can initialise a `static`
or `const`.

```rust
use type_lang::Unifier;

let unifier = Unifier::new();
assert!(unifier.is_empty());
```

### `Unifier::with_capacity`

```rust
pub fn with_capacity(vars: usize) -> Unifier
```

Creates an empty unifier with room for `vars` variables preallocated.

**Parameters**

- `vars` — the number of variables to reserve space for. A hint only: it sizes the
  internal table so that minting up to `vars` variables does not reallocate.

Use it when the variable count is known up front — for instance, one variable per
binding in the scope being checked.

```rust
use type_lang::Unifier;

let mut unifier = Unifier::with_capacity(8);
for _ in 0..8 {
    let _ = unifier.fresh();
}
assert_eq!(unifier.var_count(), 8);
```

### `Unifier::fresh`

```rust
pub fn fresh(&mut self) -> TyVar
```

Mints a fresh, unbound inference variable. Variables are numbered in creation order
from `0` and stay valid for the life of the unifier. A fresh variable stands for
"some type not yet known"; it gains a binding only when [`unify`](#unifierunify)
requires one.

```rust
use type_lang::Unifier;

let mut unifier = Unifier::new();
let a = unifier.fresh();
let b = unifier.fresh();
assert_ne!(a, b);
assert_eq!(unifier.var_count(), 2);
```

### `Unifier::unify`

```rust
pub fn unify(&mut self, a: &Type, b: &Type) -> Result<(), TypeError>
```

Unifies two types, binding variables as needed to make them equal. On success the
substitution is extended so that `a` and `b` [`resolve`](#unifierresolve) to the same
type.

**Parameters**

- `a`, `b` — the two types to make equal. `unify(a, b)` records `a` as `expected` and
  `b` as `found` in a [`Mismatch`](#typeerror) error; the unification itself is
  symmetric, so the argument order affects only that labelling.

**Errors**

- [`TypeError::Mismatch`](#typeerror) if the two types are built from different
  constructors, or the same constructor at a different arity.
- [`TypeError::Occurs`](#typeerror) if making them equal would require binding a
  variable to a type that contains it (an infinite type).

**Not transactional.** Bindings made before a conflict is reached are **kept**. To
attempt a unification without committing, clone the unifier first (see
[`Unifier`](#unifier)).

```rust
use type_lang::{TyCon, Type, TypeError, Unifier};

const INT: TyCon = TyCon::new(0);
const BOOL: TyCon = TyCon::new(1);
const PAIR: TyCon = TyCon::new(2);

let mut unifier = Unifier::new();
let a = unifier.fresh();
let b = unifier.fresh();

// Unifying Pair<?a, int> with Pair<bool, ?b> solves both variables.
let lhs = Type::app(PAIR, [Type::var(a), Type::con(INT)]);
let rhs = Type::app(PAIR, [Type::con(BOOL), Type::var(b)]);
unifier.unify(&lhs, &rhs).unwrap();
assert_eq!(unifier.resolve(&Type::var(a)), Type::con(BOOL));
assert_eq!(unifier.resolve(&Type::var(b)), Type::con(INT));

// Mismatched constructors do not unify.
let err = unifier.unify(&Type::con(INT), &Type::con(BOOL)).unwrap_err();
assert!(matches!(err, TypeError::Mismatch { .. }));
```

Handling the failure modes explicitly:

```rust
use type_lang::{TyCon, Type, TypeError, Unifier};

fn try_unify(unifier: &mut Unifier, a: &Type, b: &Type) {
    match unifier.unify(a, b) {
        Ok(()) => { /* the two types are now equal */ }
        Err(TypeError::Mismatch { expected, found }) => {
            eprintln!("expected `{expected}`, found `{found}`");
        }
        Err(TypeError::Occurs { var, ty }) => {
            eprintln!("variable ?{} would be infinite in `{ty}`", var.to_u32());
        }
        // `TypeError` is `#[non_exhaustive]`, so a wildcard is required.
        Err(e) => eprintln!("{e}"),
    }
}
```

### `Unifier::resolve`

```rust
pub fn resolve(&self, ty: &Type) -> Type
```

Resolves a type fully under the current substitution. Every bound variable in `ty`
is replaced by what it was bound to, all the way down, leaving a type whose only
variables are still unbound. A type with no bound variables resolves to an equal copy
of itself; an unbound variable resolves to itself.

**Parameters**

- `ty` — the type to resolve. Borrowed; a fresh resolved `Type` is returned.

> **Cost.** The walk is proportional to the size of the resolved *result*. A
> substitution that maps a variable to a type mentioning it more than once can make
> the result larger than the input, so resolve once a type is fully constrained
> rather than after every step.

```rust
use type_lang::{TyCon, Type, Unifier};

const LIST: TyCon = TyCon::new(0);
const INT: TyCon = TyCon::new(1);

let mut unifier = Unifier::new();
let element = unifier.fresh();
let collection = unifier.fresh();

// ?collection = List<?element>, then ?element = int.
unifier
    .unify(&Type::var(collection), &Type::app(LIST, [Type::var(element)]))
    .unwrap();
unifier.unify(&Type::var(element), &Type::con(INT)).unwrap();

// Resolving the outer variable substitutes all the way down.
assert_eq!(
    unifier.resolve(&Type::var(collection)),
    Type::app(LIST, [Type::con(INT)]),
);

// An unbound variable resolves to itself.
let free = unifier.fresh();
assert_eq!(unifier.resolve(&Type::var(free)), Type::var(free));
```

### `Unifier::var_count` / `is_empty`

```rust
pub fn var_count(&self) -> usize
pub fn is_empty(&self) -> bool
```

The number of variables the unifier has minted, and whether it has minted none.

```rust
use type_lang::Unifier;

let mut unifier = Unifier::new();
assert!(unifier.is_empty());
let _ = unifier.fresh();
assert_eq!(unifier.var_count(), 1);
assert!(!unifier.is_empty());
```

---

## `TypeError`

```rust
#[non_exhaustive]
pub enum TypeError {
    Mismatch { expected: Type, found: Type },
    Occurs { var: TyVar, ty: Type },
}
```

The reason a [`unify`](#unifierunify) call failed. The enum is `#[non_exhaustive]`,
so a downstream `match` must include a wildcard arm. It derives `Clone`, `Debug`,
`PartialEq`, and `Eq`, and implements `core::error::Error` and `Display`. Both
variants carry their types **resolved** under the substitution at the point of
failure, so they show the most concrete form known.

**`Mismatch { expected, found }`** — the two types do not share a constructor: the
heads differ, or the same head is applied to a different arity.

- `expected` — the first type given to `unify`, resolved.
- `found` — the second type given to `unify`, resolved.

**`Occurs { var, ty }`** — binding a variable would make it occur within its own
definition (an infinite type), so the occurs check rejected it.

- `var` — the variable that would refer to itself.
- `ty` — the type it would have been bound to, resolved.

`Display` renders a one-line message using the structural [`Type`](#type-display)
notation; map the variants onto your own type names for user-facing diagnostics.

```rust
use type_lang::{TyCon, Type, TypeError, Unifier};

const INT: TyCon = TyCon::new(0);
const LIST: TyCon = TyCon::new(1);

let mut unifier = Unifier::new();

// A mismatch carries both clashing types.
let err = unifier.unify(&Type::con(INT), &Type::con(LIST)).unwrap_err();
assert!(matches!(err, TypeError::Mismatch { .. }));

// An occurs failure names the offending variable.
let v = unifier.fresh();
let recursive = Type::app(LIST, [Type::var(v)]);
let err = unifier.unify(&Type::var(v), &recursive).unwrap_err();
assert!(matches!(err, TypeError::Occurs { var, .. } if var == v));
assert!(err.to_string().contains("recursive type"));
```

---

## Serialization

With the `serde` feature, [`Type`](#type), [`TyVar`](#tyvar), [`TyCon`](#tycon), and
[`Unifier`](#unifier) implement `serde::Serialize` and `serde::Deserialize`. A type
term round-trips through any serde format, and a whole `Unifier` — its variables and
their bindings — can be serialised to persist or transfer an inference state.

```rust,ignore
use type_lang::{TyCon, Type, Unifier};

const LIST: TyCon = TyCon::new(0);
const INT: TyCon = TyCon::new(1);

let mut unifier = Unifier::new();
let a = unifier.fresh();
let b = unifier.fresh();
unifier.unify(&Type::var(a), &Type::app(LIST, [Type::var(b)])).unwrap();
unifier.unify(&Type::var(b), &Type::con(INT)).unwrap();

// The substitution survives the round trip.
let json = serde_json::to_string(&unifier)?;
let restored: Unifier = serde_json::from_str(&json)?;
assert_eq!(
    restored.resolve(&Type::var(a)),
    Type::app(LIST, [Type::con(INT)]),
);
# Ok::<(), Box<dyn std::error::Error>>(())
```

---

## Feature flags

| Feature | Default | Description |
|---------|---------|-------------|
| `std`   | yes     | Builds against the standard library. With `default-features = false` the crate is `no_std` (it always needs `alloc`); the entire API works unchanged. |
| `serde` | no      | Derives `Serialize` / `Deserialize` for `Type`, `TyVar`, `TyCon`, and `Unifier`. See [Serialization](#serialization). |

Disabling `std` keeps the crate `no_std`:

```toml
[dependencies]
type-lang = { version = "1.0", default-features = false }
```

---

<sub>Copyright &copy; 2026 <strong>James Gober</strong>.</sub>
