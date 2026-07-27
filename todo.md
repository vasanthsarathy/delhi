# delhi — working checklist

Detailed plan: **`docs/superpowers/plans/2026-07-27-delhi-v0.1-semantic-core.md`**
Design spec: **`docs/superpowers/specs/2026-07-25-delhi-core-design.md`**

---

## Done before implementation

- [x] Design spec, reviewed against [T], [KR24], [KR21], [KR21-S], [MBD], and the Java source
- [x] Correctness pass — 4 errors fixed (Coin Lie arrow direction, a false accusation against `[J]`, `→ᵢ` non-emptiness, `pre` typing)
- [x] Background sections on frames, modal systems, and bisimulation
- [x] **§6 blocker resolved** — `~R` proved sound, incompleteness measured at 5–10%, cause diagnosed (`research/bisimulation/`)

---

## Plan 1 — semantic core (`delhi-syntax`, `delhi-core`, `delhi-mb`) — **COMPLETE**

- [x] **Task 1** — workspace layout and the symbol interner
- [x] **Task 2** — hash-consed formula store
- [x] **Task 3** — derived-attitude sugar constructors
- [x] **Task 4** — `Bits` bitset primitive
- [x] **Task 5** — `Model`, `State`, frame validation
- [x] **Task 6** — derived relations, with the corrected `maxima` precondition
- [x] **Task 7** — memoised entailment for `K` / `B` / `□` / `C` / `B^ψ`
- [x] **Task 8** — `~R` and `~D` bisimulation, with the §6.1.3 witness as a test
- [x] **Task 9** — canonical state keys
- [x] **Task 10** — action theories and well-formedness diagnostics
- [x] **Task 11** — action model construction (ontic, sensing, announcement)
- [x] **Task 12** — product update, thesis rule and [MBD] variant
- [x] **Task 13** — Coin Lie figure reproduction, [T] Figs 5.4–5.10
- [x] **Task 14** — known-defect tests, ignored and failing by design
- [x] **Task 15** — property suite, gap regression, `delhi-core` trait
- [x] Final whole-branch review + one fix wave + scoped re-review

**Delivered:** 26 commits, 62 passing tests, 2 ignored by design, clippy clean under
`--all-targets`, zero runtime dependencies.

## Plan 1 follow-on

- [ ] **Task 16** — `𝒦^eff` / `𝒦^obs` from [KR21] §4.1: what observers learn about effect
      *conditions*. Deferred by design — the construction (Task 11) had to exist first.

## Plan 2 — surface language and CLI (`delhi-lang`, `delhi-cli`)

- [ ] Plan not yet written. Lexer, recursive-descent parser, type/object/grounding front-end,
      `initially`/`state` lowering, sugar desugaring, and the six `delhi` subcommands
      including `dot`. Depends only on Plan 1's public API.

---

## Open questions carried forward

- [ ] Is `~D` a congruence for product update? (§6.3) If yes, `contract_dynamic` becomes
      `contract_full` and the ~10% merge improvement applies to search.
- [ ] **§4.7(a) needs settling against the primary source** before anyone attempts the θ/τ
      announcement fix — see the correction note in the spec. The documented defect does not
      manifest as described.
- [ ] Is the §4.8 hypothetical-actions gap deliberate scoping in mB, or an oversight?
      Pinned by an ignored failing test either way. **Needs Vasanth's judgement.**

## Deferred minors (triaged at final review, not blocking)

- [ ] `Model::new` uses `Bits::new(n_atoms.max(1))`; zero-atom models mask `Bits::set`'s assert.
      Needs a small design call on `Bits::new(0)` semantics.
- [ ] Coin Lie's `ActionDef`s never run through `validate()` — the deeper question is whether
      `build()` should validate internally.
- [ ] No generic test through the `EpistemicState` trait; worth adding when a second backend lands.
- [ ] `undecided`'s test derives its expectation from `believes_whether` rather than primitives.
      Safe, since `believes_whether` is verified against primitives on the line above.

---

## Review

**What was built.** Three crates. `delhi-syntax` holds the query language — hash-consed formulas
over the six primitive operators, plus sugar for nine derived attitudes that desugars before it
reaches the semantics. `delhi-mb` holds the mB+ backend: bitset-backed plausibility models with
frame validation, memoised entailment, two distinct bisimulation notions with contraction and
canonical hashing, action-theory compilation into event models, and product update.
`delhi-core` declares the backend-agnostic trait a future planner will be generic over.

**What validates it.** The Coin Lie scenario from [T] Figs 5.4–5.10 reproduces end to end,
including the second-order false belief at the final state — it passed on the first run, which
is the strongest evidence the semantics were transcribed correctly. The bisimulation gap
regression reproduces the measurement from `research/bisimulation/` exactly (115 models, 6
incomplete, 0 unsound). Ten property tests cover frame preservation across update, the KB bridge
axioms, seriality, `~R ⊆ ~D`, and two post-update semantic invariants.

**Three defects in my own planning documents, found by execution.**

1. *Task 9's canonical encoding was not canonical.* It ordered blocks by a refinement that numbers
   by first occurrence in world order, so renaming worlds changed the key. Its own renaming test
   would have failed. Caught by the pre-flight scan; replaced with sorted-signature ranking, which
   also deleted a dead factorial permutation search.

2. *The spec claimed the two transition rules agree on every worked example.* They do not — Coin
   Lie is itself the differential case. Found by an implementer who reported BLOCKED rather than
   adjusting a failing test to pass. Under the draft rule the lie fails to land at all, which is
   direct evidence for the thesis rule being authoritative. Spec §4.5.1 amended.

3. *The spec's §4.7(a) describes a defect that does not manifest as described.* The announcement
   limitation was characterised as a full observer learning too much; in fact a partial observer
   ends up undecided, so the acceptance test fails on a different assertion than predicted. Spec
   §4.7(a) amended with a warning not to attempt the fix before settling it.

**Two assertions were structurally incapable of firing.** A reflexivity check in product update
could never fail because the model constructor pre-seeded the diagonal; a second one could never
fail because the same pure memoised predicate that selected the event had already populated the
index. Both looked like live checks and were not.

**The pattern worth keeping.** In ten of the fifteen tasks, a review found that a test specified
in the plan could not have failed against the bug it nominally covered. The production code was
almost always right; the tests guarding it usually were not. The discipline that caught these was
requiring a red/green experiment — sabotage the code the test claims to protect, observe the
failure, restore — which turns "this test passes" into "this test would fail if the thing it
protects broke."
