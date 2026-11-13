<h1 align="center">
    <img width="90px" height="auto" src="https://raw.githubusercontent.com/jamesgober/jamesgober/main/media/icons/hexagon-3.svg" alt="Triple Hexagon">
    <br><b>CHANGELOG</b>
</h1>
<p>
  All notable changes to <code>type-lang</code> will be documented in this file. The format is based on <a href="https://keepachangelog.com/en/1.1.0/">Keep a Changelog</a>,
  and this project adheres to <a href="https://semver.org/spec/v2.0.0.html/">Semantic Versioning</a>.
</p>

---

## [Unreleased]

---

## [1.0.1] - 2026-11-12

### Fixed

- The occurs check took time exponential in the depth of shared bindings. With
  `?i := Pair(?i+1, ?i+1)` for 30 variables, binding one more variable to `?0`
  took seconds and grew twice as slow with each step. Each binding is now
  searched once per check.
- `unify` of two types whose bindings share structure the same way repeated the
  work once per path. It now unifies each pair of variables once per call.
- A `Mismatch` or `Occurs` error at the end of a long chain of bindings
  overflowed the stack, because the error payload was resolved recursively.
  `resolve` is now iterative, and error payloads stop substituting variables
  once they are 64 levels deep or hold about 1,024 nodes. Past that, a bound
  variable is left in place; `resolve` expands it fully.
- `Display` for `Type` is now iterative, so printing a deeply nested type does
  not overflow the stack.
- With the `serde` feature, deserializing a `Unifier` whose bindings form a
  cycle (for example `?0` bound to `?0`) used to succeed, and later calls then
  looped forever. It is now refused with an error.

### Changed

- `unify` shortens chains of variables bound to variables as it walks them, so
  a long chain is walked once instead of on every call.

---

## [1.0.0] - 2026-06-29

The stable release. The public surface from `0.2.0` is ratified as the `1.0`
contract, with a recorded SemVer promise, a pre-release soundness fix, and hardening
of the unifier's variable handling.

### Changed

- The public API is **stable** and follows Semantic Versioning: no breaking changes
  before `2.0`. The promise is recorded in [`docs/API.md`](docs/API.md#stability).
  `TypeError` stays `#[non_exhaustive]` so a new failure variant remains a minor
  change, and the MSRV (Rust 1.85) only rises in a minor release.

### Fixed

- `Unifier::unify` could report success without recording a binding when given a
  variable the unifier had not minted, leaving the two types unequal in violation of
  its own postcondition. Such a variable now joins the unifier's id space and is
  bound, so unification is sound for every input. Found in pre-release review.
- `Unifier::fresh` now asserts the 32-bit variable-id bound in debug builds, and the
  `TyVar` documentation describes the id-space contract the API actually upholds.

---

## [0.2.0] - 2026-06-29

The core the scaffold was built for: a type representation, inference variables, and
first-order unification with an occurs check. This is the soundness-critical layer a
type checker is written against; per-language type rules layer on top by choosing
what each constructor means.

### Added

- `Type` — the type term: an inference `Var`, or an `App` of a constructor to its
  arguments. Built with `Type::var`, `Type::con`, and `Type::app`, inspected with
  `as_var`, `head`, `args`, and `is_var`, and rendered structurally via `Display`.
- `TyVar` — an opaque, `Copy` inference-variable handle, minted by the unifier and
  stable for its life.
- `TyCon` — an opaque constructor tag the consumer assigns meaning to; `new` and
  `to_u32`.
- `Unifier` — holds the inference variables and the substitution over them, with
  `new`, `with_capacity`, `fresh`, `unify`, `resolve`, `var_count`, and `is_empty`.
  Unification is iterative (an explicit work stack), binds variables under an occurs
  check, and yields a most-general unifier.
- `TypeError` — `#[non_exhaustive]` failure type with `Mismatch` and `Occurs`
  variants, each carrying the offending types resolved under the substitution;
  implements `Display` and `core::error::Error`.
- `serde::Serialize` / `serde::Deserialize` for `Type`, `TyVar`, `TyCon`, and
  `Unifier` under the `serde` feature, so type terms and a whole inference state can
  be persisted.
- Property tests (`tests/properties.rs`) covering the unification invariants:
  soundness, symmetry, idempotent resolution, reflexivity, and the occurs check.
- Integration tests (`tests/unify.rs`) exercising function application, generic
  instantiation, mismatch reporting, and speculative unification via `clone`.
- `criterion` benchmarks for variable minting, unification, and resolution.

### Changed

- `clippy.toml` MSRV aligned to `1.85` to match `Cargo.toml`, removing the clippy
  configuration-mismatch warning.

### Fixed

- `Cargo.toml` `keywords` and `categories` were unquoted arrays — invalid TOML that
  stopped the manifest from parsing. They are now quoted string lists.
- `deny.toml` header named the wrong crate (a copy-paste leftover).

---

## [0.1.0] - 2026-06-18

Initial scaffold and repository bootstrap. No domain logic yet &mdash; this release establishes the structure, tooling, and quality gates the implementation will be built on.

### Added

- `Cargo.toml` with crate metadata, Rust 2024 edition, MSRV 1.85.
- Dual `Apache-2.0 OR MIT` license files.
- `README.md`, `CHANGELOG.md`, and a documentation skeleton.
- `REPS.md` compliance baseline.
- `.github/workflows/ci.yml` CI matrix; `deny.toml`, `clippy.toml`, `rustfmt.toml`.
- `dev/DIRECTIVES.md` and `dev/ROADMAP.md` (committed engineering standards + plan).

[Unreleased]: https://github.com/jamesgober/type-lang/compare/v1.0.1...HEAD
[1.0.1]: https://github.com/jamesgober/type-lang/compare/v1.0.0...v1.0.1
[1.0.0]: https://github.com/jamesgober/type-lang/compare/v0.2.0...v1.0.0
[0.2.0]: https://github.com/jamesgober/type-lang/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/jamesgober/type-lang/releases/tag/v0.1.0
