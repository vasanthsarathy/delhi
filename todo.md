# delhi — working checklist

Detailed plan with per-step code: **`docs/superpowers/plans/2026-07-27-delhi-v0.1-semantic-core.md`**
Design spec: **`docs/superpowers/specs/2026-07-25-delhi-core-design.md`**

---

## Done before implementation

- [x] Design spec, reviewed against [T], [KR24], [KR21], [KR21-S], [MBD], and the Java source
- [x] Correctness pass — 4 errors fixed (Coin Lie arrow direction, a false accusation against `[J]`, `→ᵢ` non-emptiness, `pre` typing)
- [x] Background sections on frames, modal systems, and bisimulation
- [x] **§6 blocker resolved** — `~R` proved sound, incompleteness measured at 5–10%, cause diagnosed (`research/bisimulation/`)

---

## Plan 1 — semantic core (`delhi-syntax`, `delhi-core`, `delhi-mb`)

- [ ] **Task 1** — workspace layout and the symbol interner
- [ ] **Task 2** — hash-consed formula store
- [ ] **Task 3** — derived-attitude sugar constructors
- [ ] **Task 4** — `Bits` bitset primitive
- [ ] **Task 5** — `Model`, `State`, frame validation
- [ ] **Task 6** — derived relations, with the corrected `maxima` precondition
- [ ] **Task 7** — memoised entailment for `K` / `B` / `□` / `C` / `B^ψ`
- [ ] **Task 8** — `~R` and `~D` bisimulation, with the §6.1.3 witness as a test
- [ ] **Task 9** — canonical state keys
- [ ] **Task 10** — action theories and well-formedness diagnostics
- [ ] **Task 11** — action model construction (ontic, sensing, announcement)
- [ ] **Task 12** — product update, thesis rule and [MBD] variant
- [ ] **Task 13** — Coin Lie figure reproduction, [T] Figs 5.4–5.10
- [ ] **Task 14** — known-defect tests, ignored and failing by design
- [ ] **Task 15** — property suite, gap regression, `delhi-core` trait

Deliverable: a Rust library that builds plausibility models, answers `L_GB` queries,
compiles action theories, applies product update, and contracts and hashes states —
validated against the published figures and the papers' own propositions.

## Plan 1 follow-on (not yet planned in detail)

- [ ] **Task 16** — `𝒦^eff` / `𝒦^obs` from [KR21] §4.1: what observers learn about
      effect *conditions*. Deliberately deferred; the construction (Task 11) has to
      exist before the observer refinement can be tested against it.

## Plan 2 — surface language and CLI (`delhi-lang`, `delhi-cli`)

- [ ] Plan not yet written. Lexer, recursive-descent parser, type/object/grounding
      front-end, `initially`/`state` lowering, sugar desugaring, and the six `delhi`
      subcommands including `dot`. Depends only on Plan 1's public API.

---

## Open questions carried into implementation

- [ ] Is `~D` a congruence for product update? (§6.3) If yes, `contract_dynamic`
      becomes `contract_full` and the ~10% merge improvement applies to search.
      Needs product update implemented first — so it lands after Task 12.
- [ ] Is the §4.8 hypothetical-actions gap a deliberate scoping decision in mB, or an
      oversight? Task 14 pins the behaviour either way. **Needs Vasanth's judgement.**
- [ ] Is [T] Def. 2 definitely intended over the 2022 draft's rule? Task 12 implements
      both and Task 13 checks they agree. **Needs Vasanth's judgement.**

---

## Review

*(To be filled in after implementation, per `CLAUDE.md` step 7: a summary of the
changes made and anything else worth recording.)*
