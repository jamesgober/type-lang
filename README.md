<h1 align="center">
    <img width="99" alt="Rust logo" src="https://raw.githubusercontent.com/jamesgober/rust-collection/72baabd71f00e14aa9184efcb16fa3deddda3a0a/assets/rust-logo.svg">
    <br>
    <b>type-lang</b>
    <br>
    <sub><sup>TYPE SYSTEM</sup></sub>
</h1>

<div align="center">
    <a href="https://crates.io/crates/type-lang"><img alt="Crates.io" src="https://img.shields.io/crates/v/type-lang"></a>
    <a href="https://crates.io/crates/type-lang"><img alt="Downloads" src="https://img.shields.io/crates/d/type-lang?color=%230099ff"></a>
    <a href="https://docs.rs/type-lang"><img alt="docs.rs" src="https://img.shields.io/docsrs/type-lang"></a>
    <a href="https://github.com/jamesgober/type-lang/actions"><img alt="CI" src="https://github.com/jamesgober/type-lang/actions/workflows/ci.yml/badge.svg"></a>
    <a href="https://github.com/rust-lang/rfcs/blob/master/text/2495-min-rust-version.md"><img alt="MSRV" src="https://img.shields.io/badge/MSRV-1.85%2B-blue"></a>
</div>

<br>

<div align="left">
    <p>
        <b>type-lang</b> is the type-system substrate of a compiler front-end: a representation for types, a unifier that makes two types equal and records what that requires, and a structured error for when they cannot be made equal. It owns the soundness-critical core &mdash; type terms, inference variables, and first-order unification &mdash; and a language's own type rules layer on top. It does no parsing and renders no diagnostics. Part of the <code>-lang</code> language-construction family.
    </p>
    <br>
    <hr>
    <p>
        <strong>MSRV is 1.85+</strong> (Rust 2024 edition).
    </p>
    <blockquote>
        <strong>Status: stable.</strong> The public API is frozen as of <code>1.0.0</code> and follows Semantic Versioning &mdash; no breaking changes before <code>2.0</code>. See <a href="./CHANGELOG.md"><code>CHANGELOG.md</code></a> and <a href="./docs/API.md"><code>docs/API.md</code></a>.
    </blockquote>
</div>

<hr>
<br>

## Highlights

- **First-order unification** with an occurs check, so a variable can never be bound to a type that contains itself. The substitution produced is a most-general unifier.
- **Language-agnostic** type representation: constructors are opaque tags the consumer assigns meaning to, so the same core expresses primitives, generics, functions, and tuples.
- **Iterative core** &mdash; unification and the occurs check use explicit work stacks, so a deeply nested type cannot overflow the call stack.
- **`no_std`-friendly** (needs only `alloc`), with optional `serde` for serialising type terms and inference state.
- **No panics** in shipping code; every failure is a defined `TypeError`. `#![forbid(unsafe_code)]`.

<br>

## Installation

```toml
[dependencies]
type-lang = "1.0"
```

<br>

## Usage

A consumer assigns meaning to constructor tags, then unifies the types it builds.
Unification discovers what the inference variables must be:

```rust
use type_lang::{TyCon, Type, Unifier};

// The consumer's constructor table — the tags are arbitrary but stable.
const FUNCTION: TyCon = TyCon::new(0);
const INT: TyCon = TyCon::new(1);
const BOOL: TyCon = TyCon::new(2);

let mut unifier = Unifier::new();
let arg = unifier.fresh();
let ret = unifier.fresh();

// A call site needs a function (?arg) -> ?ret; the callee is (int) -> bool.
let needed = Type::app(FUNCTION, [Type::var(arg), Type::var(ret)]);
let callee = Type::app(FUNCTION, [Type::con(INT), Type::con(BOOL)]);
unifier.unify(&needed, &callee).expect("same constructor and arity");

// Unification solved both variables.
assert_eq!(unifier.resolve(&Type::var(arg)), Type::con(INT));
assert_eq!(unifier.resolve(&Type::var(ret)), Type::con(BOOL));
```

Failures are values, not panics — a mismatch carries the clashing types, and a
self-referential type is rejected by the occurs check:

```rust
use type_lang::{TyCon, Type, TypeError, Unifier};

const INT: TyCon = TyCon::new(0);
const LIST: TyCon = TyCon::new(1);

let mut unifier = Unifier::new();

// Different constructors do not unify.
let err = unifier.unify(&Type::con(INT), &Type::con(LIST)).unwrap_err();
assert!(matches!(err, TypeError::Mismatch { .. }));

// ?v = List<?v> would be an infinite type.
let v = unifier.fresh();
let recursive = Type::app(LIST, [Type::var(v)]);
let err = unifier.unify(&Type::var(v), &recursive).unwrap_err();
assert!(matches!(err, TypeError::Occurs { .. }));
```

See [`docs/API.md`](./docs/API.md) for the full reference.

<br>

## Feature flags

| Feature | Default | Description |
|---------|---------|-------------|
| `std`   | yes     | Builds against the standard library. With `default-features = false` the crate is `no_std` (it always needs `alloc`); the full API works unchanged. |
| `serde` | no      | Derives `Serialize` / `Deserialize` for `Type`, `TyVar`, `TyCon`, and `Unifier`. |

<hr>
<br>

## Contributing

See <a href="./dev/DIRECTIVES.md"><code>dev/DIRECTIVES.md</code></a> for engineering standards and the definition of done. Before a PR: `cargo fmt --all`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test --all-features` must be clean.

<br>

<div id="license">
    <h2>License</h2>
    <p>Licensed under either of</p>
    <ul>
        <li><b>Apache License, Version 2.0</b> &mdash; <a href="./LICENSE-APACHE">LICENSE-APACHE</a></li>
        <li><b>MIT License</b> &mdash; <a href="./LICENSE-MIT">LICENSE-MIT</a></li>
    </ul>
    <p>at your option.</p>
</div>

<div align="center">
  <h2></h2>
  <sup>COPYRIGHT <small>&copy;</small> 2026 <strong>James Gober <me@jamesgober.com>.</strong></sup>
</div>
