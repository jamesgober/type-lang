# type-lang &mdash; API Reference

> **Status: pre-1.0 scaffold.** This crate is the `v0.1.0` bootstrap: it has no
> public surface yet. The type representation, the unification core, the
> substitution machinery, and the inference scaffolding are authored — with a full
> reference on this page — at `v0.2.0`. See [`../dev/ROADMAP.md`](../dev/ROADMAP.md)
> for the phase plan and [`../CHANGELOG.md`](../CHANGELOG.md) for what has shipped.

## Overview

type-lang is the type-system substrate of a compiler front-end. It represents
types, unifies them under a substitution, mints fresh inference variables, and
reports why a unification fails. It owns the representation and the
unification/substitution machinery only; a language's own type rules — its
primitives, subtyping, and coercions — layer on top in terms of that core. It does
no parsing and renders no diagnostics.

## Public surface

None in `v0.1.0`. The crate compiles, passes the full CI gate, and reserves the
`std` (default) and `serde` feature flags; there is no exported type, trait, or
function to document yet. This page is filled in alongside the implementation at
`v0.2.0`, where every public item ships with a description, its parameters and
return contract, error semantics, and runnable examples.

## Feature flags

| Feature | Default | Description |
|---------|---------|-------------|
| `std`   | yes     | Builds against the standard library. With `default-features = false` the crate is `no_std`. |
| `serde` | no      | Reserved for `Serialize`/`Deserialize` on the type terms, landing with the core at `v0.2.0`. |

---

<sub>Copyright &copy; 2026 <strong>James Gober</strong>.</sub>
