# type-lang - Roadmap

> Path from scaffold to a stable 1.0. Hard parts are front-loaded; each phase has hard exit criteria.
> Master plan: ../../_strategy/LANG_COLLECTION.md
>
> **Anti-deferral rule:** no listed hard task moves to a later phase unless this file records the move and the reason.

## v0.1.0 - Scaffold (DONE)
Compiles, CI green, structure correct, no domain logic.
- [x] Manifest, README, CHANGELOG, REPS, dual license, CI, deny, clippy, rustfmt.

## v0.2.0 - Core (THE HARD PART, NOT DEFERRED) (DONE)
Type representation, unification, and inference scaffolding; per-language rules layer on top.
- [x] `Type` (Var / App), `TyVar`, `TyCon`, `Unifier` (fresh/unify/resolve), `TypeError`.
- [x] First-order unification with occurs check; iterative (no stack overflow on deep input).
- [x] Every public item has rustdoc + a runnable example.
- [x] Core invariants property-tested (soundness, symmetry, occurs check, idempotence, reflexivity).
- [x] Full DIRECTIVES + API authored at this stage; serde, integration tests, and benchmarks added.

**Deferred (recorded per the anti-deferral rule): wiring `ast-lang`, `symbol-lang`,
and `diag-lang`.** The roadmap slated these to be wired "when first used" at v0.2.0.
The v0.2.0 unification core — type terms, variables, `unify`, `resolve`, and the
error — is self-contained and uses none of them, so wiring any of them now would add
an unused dependency, which the dependency policy prohibits. Integration with a
syntax tree, a symbol table, and a diagnostic renderer is the consumer's to make for
now: `TypeError` is shaped to map onto a `diag-lang` diagnostic in the caller's own
type names. They will be wired only if and when a consumer-facing pass inside this
crate first needs them.

## v1.0.0 - API freeze
Public surface stable and frozen until 2.0.
- [ ] docs/API.md marked stable; SemVer promise recorded.
- [ ] Full test + benchmark suite green on all three platforms.
