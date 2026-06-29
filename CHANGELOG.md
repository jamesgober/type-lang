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

### Added

### Changed

### Fixed

### Security

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

[Unreleased]: https://github.com/jamesgober/type-lang/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/jamesgober/type-lang/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/jamesgober/type-lang/releases/tag/v0.1.0
