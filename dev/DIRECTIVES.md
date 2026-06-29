# type-lang &mdash; Engineering Directives

> Engineering standards and the definition of done for this project. Read alongside `REPS.md` (root, authoritative) and `dev/ROADMAP.md` (current phase). If anything here conflicts with `REPS.md`, `REPS.md` wins.

---

## 0. Philosophy

This library is built and maintained to a production standard and treated as a flagship piece of work. Plan the full path, then build one verified step at a time. "Good enough" is treated as a defect. type-lang sits at the semantic layer of a compiler front-end: it decides whether two types are the same, what an unconstrained type must become, and when a program is ill-typed. A soundness bug here is not a wrong answer in one place — it lets an entire class of invalid programs through, so the unification core is proven before any convenience is built on top of it.

---

## 1. What this is

type-lang is the type-system substrate a front-end reasons with. It represents types, unifies them under a substitution, mints fresh inference variables, and reports the precise reason a unification fails. It is the structural core that per-language type rules layer on top of: this crate owns the representation and the unification/substitution machinery; the rules of any one language — its primitives, its subtyping, its coercions — are expressed by the consumer in terms of that core. It does no parsing and renders no diagnostics; it assigns types to an `ast-lang` tree, resolves names through `symbol-lang`, and hands a structured type error to `diag-lang` to render.

---

## 2. Engineering law (non-negotiable)

- **Performance** — peak is the baseline; unification is near-linear with path-compressed variable resolution, not a repeated deep walk; the steady-state unify/substitute path does not allocate per call; no "faster" claim without `criterion` numbers.
- **Correctness** — the invariants in section 4 are covered by property tests. Unification is sound (it never equates two types that have no common instance), it yields a most-general unifier, and the occurs check rejects infinite types.
- **Security** — every input is treated as hostile: an arbitrarily deep or cyclic type term is a defined error or a bounded walk, never a stack overflow or UB; variable-id arithmetic cannot overflow into a live variable.
- **Architecture** — SOLID, KISS, YAGNI; one responsibility; the type representation, the substitution, and the unification algorithm each sit behind their own seam, and per-language policy is injected, never baked in.
- **Cross-platform** — Linux/macOS/Windows first-class, verified by CI; no platform-specific behavior in the core.
- **Error handling** — every fallible path (a failed unification, an occurs-check violation, an unbound variable) returns `Result`/`Option` per the documented contract; a type error carries enough structure for a caller to render it.
- **Production-ready** — `#![forbid(unsafe_code)]` and `#![deny(missing_docs)]` from the first commit; no stray `println!`/`dbg!`; every public item has rustdoc with a runnable example.

---

## 3. Definition of done

1. Compiles clean on Linux/macOS/Windows, stable and MSRV 1.85.
2. `fmt`, `clippy -D warnings`, `test --all-features`, `cargo doc -D warnings` clean.
3. `cargo audit` + `cargo deny check` pass.
4. No `unwrap`/`expect`/`todo!`/`dbg!` in shipping code.
5. A Tier-1 API exists and headlines the docs.
6. Property tests cover every section-4 invariant.
7. Hot-path changes carry benchmarks; no regression over 5%.
8. Docs and `CHANGELOG.md` updated; the matching `docs/release/vX.Y.Z.md` written before the tag.

---

## 4. Project-specific invariants

- Unification is sound: if `unify(a, b)` succeeds, applying the resulting substitution to `a` and to `b` yields the same type; if it fails, the two types have no common instance under any substitution.
- Unification is symmetric: `unify(a, b)` and `unify(b, a)` succeed or fail together and produce equivalent substitutions.
- The result of a successful unification is a most-general unifier — no more specific than the constraint requires.
- The occurs check holds: a type variable never unifies with a term that contains it, so no infinite type is ever constructed.
- A fresh inference variable is distinct from every variable minted before it, and ids are stable for the life of the inference context.
- Resolving, unifying, or substituting over any well-formed type term terminates and never panics; an unbound or ill-formed input is a defined error value.
