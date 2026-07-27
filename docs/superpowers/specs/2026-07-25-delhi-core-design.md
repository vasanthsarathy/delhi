# delhi — Design Spec v0.1: Semantic Core, Action Layer, and Surface Language

**Date:** 2026-07-25
**Status:** Approved design, pending implementation plan
**Scope:** v0.1 only. The planner is v0.2 and is specified here only where v0.1 must accommodate it.

---

## 1. Purpose

delhi is an epistemic model checker and reasoning system for multi-agent domains involving
knowledge, belief, action observability, announcements (including untruthful ones), belief
revision, and higher-order false beliefs.

It is a redesign — not a transliteration — of mecaPlanner (Buckingham, Tufts), which implements
the mB action language and the cooperation-agnostic search algorithm from Buckingham's
dissertation. delhi targets the same semantics with corrections, a stronger query language, a new
surface syntax, and a test suite derived from the papers' own correctness claims.

### 1.1 Reference documents

- **[T]** Buckingham, D. *Dissertation* (`refs/Buckingham - In partial fulfillment...pdf`).
  Ch. 3 mA-local, Ch. 4 mA-revise, **Ch. 5 mB** (the semantics delhi implements),
  Ch. 6 cooperation-agnostic search, §9.2–9.3 proofs.
- **[KR24]** Buckingham, Scheutz, Son, Fabiano. *Action Language mA\* with Higher-Order Action
  Observability*, KR 2024 (`refs/kr2024-0020-buckingham-et-al.pdf`). = [T] Ch. 3, mA-local.
- **[KR21]** Buckingham, **Sarathy**, Scheutz, Son. *A Multi-Agent Epistemic and Doxastic Action
  Language with Belief Revision and Local Dynamic Observability*, KR 2021
  (`refs/buckingham_kr2021.pdf`), with **[KR21-S]** its supplementary appendix
  (`refs/buckingham_kr2021_supplement.pdf`). = [T] Ch. 4, mA-revise.
- **[MBD]** *mB*, working draft dated 2022-02-21 (`refs/mb_draft_1.pdf`). An **earlier and
  non-equivalent** version of [T] Ch. 5; see §3.5 and §3.3. Incomplete (placeholder
  "Intuitively, …" sections, no propositions). [T] Ch. 5 supersedes it.
- **[J]** The mecaPlanner Java source (`refs/mecaPlanner-main/`), used as a reference
  implementation and as a catalogue of defects to avoid.

**Lineage.** KR2020 (Buckingham, Kasenberg, Scheutz) → [KR21] mA-revise → [T] Ch. 5 mB;
separately [KR24] mA-local. Where documents conflict, **[T] is authoritative** as the latest and
most complete, with conflicts recorded explicitly below rather than silently resolved.

### 1.2 Non-goals for v0.1

- No planner, no search, no environment-agent behavior models.
- No DEPL importer. (Reconsidered in v0.2, when the EFP benchmark corpus has a consumer.)
- No probabilistic or graded-degree belief.
- No awareness logic (agents unaware that a proposition exists).
- No general formula-satisfiability-driven model synthesis.
- No hypothetical actions ([KR21] eq. 23). Known limitation, pinned by a failing test — see §3.8.

---

## 2. Decisions

| # | Decision | Rationale |
|---|---|---|
| D1 | **mB as the semantic base**, with backend-agnostic traits so mA-local can be added later | mB is the only language in the family covering all required features, notably untruthful announcements. [T] §6.4 already specifies the representation-agnostic interface. |
| D2 | **mB+**: add conditional belief `B^ψ` and safe belief `□`; fix the [T] §5.3 announcement defect; add conditional effects | Cheap expressivity gain over the same models; §5.3 is an acknowledged defect in exactly the higher-order machinery that matters. |
| D3 | **Rust** | Enums + exhaustive matching fit formula ASTs and event models; fast hash sets; the search parallelises in v0.2. The semantics are already pinned down by proofs, so iteration speed matters less than usual. |
| D4 | **New surface language** with a type/object/grounding front-end | DEPL's grammar has drifted from its own corpus; grounding and constant folding earn their keep; the new modalities have no DEPL syntax. |
| D5 | **Initial states**: declarative form + explicit escape hatch | The declarative form is what people write; the explicit form reproduces published figures exactly and is what the pretty-printer emits. |
| D6 | **Incompleteness: tiers 1 + 2** — measure the gap, and ship complete equivalence for the K/B/□/C fragment | Bounded-depth merging (tier 3) has no consumer until the planner and carries unsoundness risk. |
| D7 | **Hypothetical actions: test and document, do not fix in v0.1** | mB+ appears to inherit the defect [KR21] §7 exists to fix. Pin it with a failing test before changing Def. 2, the one part of mB with published proofs. See §3.8. |
| D8 | **At most one `pre` clause per action**; conjunction written explicitly | [T], [MBD], [KR21], and the `[J]` corpus disagree three ways on how multiple executability statements combine. Removing multiplicity removes the ambiguity instead of picking a side. See §3.3. |

---

## 3. Semantics (mB+)

### 3.1 Plausibility models — [T] §5.1.1

Given finite sets of propositions `P` and agents `G`, a **plausibility model** is
`M = ⟨W, R, V⟩`:

- `W` a finite set of worlds,
- `R : W × W → (G → bool)`, written `u Rᵢ v` for `R(u,v)(i) = true`, read
  *"agent i considers v at least as plausible as u"*,
- `V : P → 2^W` a valuation.

Each `Rᵢ` must be a **locally well-preordered relation**: a preorder (reflexive, transitive)
whose restriction to any comparability class is connected. A **state** is a pointed model
`⟨M, u⟩`.

Derived:

- `u ~ᵢ v` iff `u Rᵢ v` or `v Rᵢ u` — an equivalence relation. `~ᵢᵘ := {v | u ~ᵢ v}`.
- `→ᵢ C := {u ∈ C | u' Rᵢ u for all u' ∈ C}` — the most-plausible elements of `C`.
  `→ᵢᵘ := →ᵢ ~ᵢᵘ`. Non-empty whenever `C` is.

### 3.2 The query language `L_GB` — extends [T] Def. 1

```
φ ::= p | ¬φ | φ ∧ φ | Kᵢφ | Bᵢφ | C_g φ | Bᵢ^ψ φ | □ᵢ φ
```

| form | holds at `⟨M,u⟩` iff |
|---|---|
| `p` | `u ∈ V(p)` |
| `Kᵢφ` | `φ` at all `v ∈ ~ᵢᵘ` |
| `Bᵢφ` | `φ` at all `v ∈ →ᵢᵘ` |
| `C_g φ` | `φ` at all `v` with `u (∪_{i∈g} ~ᵢ)* v` — **common knowledge**, since it is built on `~ᵢ`, not on `Belᵢ`. ([KR24] uses `C_g` for common *belief* over `Rᵢ`; do not conflate. [MBD] has only ungrouped `C` over all of `G`; [T] generalises to groups.) |
| **`Bᵢ^ψ φ`** | `φ` at all `v ∈ →ᵢ(~ᵢᵘ ∩ ⟦ψ⟧)`; vacuously true if that set is empty |
| **`□ᵢ φ`** | `φ` at all `v` with `u Rᵢ v` |

`∨`, `→`, `⊤`, `⊥` and the duals (`K'`, `B'`, `S'`) are sugar. The last two rows are new in mB+;
`Bᵢφ ≡ Bᵢ^⊤ φ` must hold and is a property test.

### 3.3 Action theories — extends [T] §5.2

An action theory `T` is a set of statements. **Formula typing is load-bearing** and was omitted
from [MBD] (which restricts everything to `L^P`); [T] §5.2 and [KR21] §3 agree on the following:

| # | form | typing |
|---|---|---|
| 1 | `a requires φ` | `φ ∈ L^P` |
| 2 | `a causes l₀, …, lₙ` **`if φ`** *(the `if` is new in mB+)* | `lⱼ` propositional literals, `φ ∈ L^P` |
| 3 | `a determines φ` | `φ ∈ L^P` |
| 4 | `a announces ψ` | **`ψ ∈ L^P_GB` — modal, and need not be true** |
| 5 | `i observes a if φ` | `φ ∈ L^P` |
| 6 | `i aware_of a if φ` | `φ ∈ L^P` |

Form 4 taking a modal `ψ` has real consequences: announcement event preconditions
(`a_pre ∧ ψ`, §3.6) are modal, so `pre` evaluation requires full modal entailment against the
*pre-update* model, and no consistency check on announcements may assume propositional formulas.
[KR21] §3 is explicit: "ψ may be a belief formula, admitting announcements about beliefs and
knowledge, and that ψ need not be true."

**Preconditions — D8.** The sources disagree three ways:

| source | rule |
|---|---|
| [T] eq. 5.1 | `a_pre := ⋁_{"if φ then a is executable" ∈ T} φ` — **disjunction** |
| [MBD] line 150 | **at most one** form-1 sentence; `a_pre := φ` if present, else `⊤` |
| [KR21] form 3 | `α requires φ`, phrasing implies necessity |
| `[J] example.depl` | repeated `precondition{…}` clauses, semantically **conjoined** — `move` requires `at(?a,?f)` *and* `connected(…)` |

Under [T]'s disjunction, *adding* a precondition makes an action *more* applicable, which
contradicts both the word "precondition" and every file in the `[J]` corpus. delhi adopts
**[MBD]'s at-most-one rule**: a single `pre` clause per action, with conjunction written
explicitly (`pre φ & ψ`). Absent a `pre` clause, `a_pre := ⊤`. A second `pre` clause is a
lowering-time error, not a silent combination.

**Well-formedness** (from [T] §5.2, promoted from runtime assertion to a lowering-time
diagnostic with a source span):

- exactly one statement of form 2, 3, or 4 per action;
- at most one statement of form 1 per action (D8);
- for every state and every pair `observes a if φ` / `aware_of a if ψ`, `⊭ φ ∧ ψ`
  — checked syntactically where decidable, else emitted as a runtime-checked obligation.
  ([KR21] eq. 2 states the same requirement; eq. 6 derives `F(α,u) ∩ P(α,u) = ∅` from it.)
- no `causes` list containing both `p` and `¬p` ([KR21] eq. 1 states the same requirement).

### 3.4 Action plausibility models — [T] §5.1.2

`⟨E, Q, pre, add, del, Γ⟩` with `Q : E × E → (G → L^P)` the edge conditions,
`pre : E → L^P`, `add, del : E → 2^P`, `Γ ⊆ E` designated.

Edge labels:

- `FPN(i) := ⊤`
- `PN(i) := ¬⋁_{"observes a if φ" ∈ T} φ`
- `N(i) := ¬((⋁_{observes a if φ} φ) ∨ (⋁_{aware_of a if φ} φ))`

Implicit throughout: every event has a reflexive `FPN` edge for every agent; every unlisted
edge is `⊥`; every world has a reflexive `Rᵢ` edge.

### 3.5 State transition — [T] Def. 2

With `e ⟶^{iuv} f := ⟨M,u⟩ ⊨ Q(e,f)(i)` **and** `⟨M,v⟩ ⊨ Q(e,f)(i)`:

- `W' = {⟨u,e⟩ | u ∈ W, e ∈ E, ⟨M,u⟩ ⊨ pre(e)}`
- `R'(⟨u,e⟩,⟨v,f⟩)(i) =`
  `(e ⟶^{iuv} f and not f ⟶^{iuv} e and u ~ᵢ v)` **or**
  `(e ⟶^{iuv} f and f ⟶^{iuv} e and u Rᵢ v)`
- `V'(⟨u,e⟩) = (V(u) ∪ add(e)) \ del(e)`
- `d' = ⟨d,e⟩` for the unique `e ∈ Γ` with `⟨M,d⟩ ⊨ pre(e)`

This is Baltag–Smets action-priority update: event plausibility overrides state plausibility
(first disjunct), so incoming information takes precedence over prior belief unless it
contradicts prior knowledge.

#### 3.5.1 [MBD] gives a different, non-equivalent rule

[MBD] line 129:

```
R′(⟨u,e⟩,⟨v,f⟩)(i) = ( uRᵢv ∧ (e ⟶ᶦᵘᵛ f ∨ f ⟶ᶦᵘᵛ e) )
                   ∨ ( vRᵢu ∧ e ⟶ᶦᵘᵛ f ∧ ¬ f ⟶ᶦᵘᵛ e )
```

Case analysis over the four event-comparability configurations:

| configuration | [T] Def. 2 | [MBD] | agree? |
|---|---|---|---|
| `e→f ∧ f→e` (tie) | `uRᵢv` | `uRᵢv` | yes |
| `e→f ∧ ¬f→e` (`f` strictly preferred) | `u ~ᵢ v` | `uRᵢv ∨ vRᵢu` = `u ~ᵢ v` | yes |
| neither direction | `false` | `false` | yes |
| **`f→e ∧ ¬e→f`** (**`e` strictly preferred**) | **`false`** | **`uRᵢv`** | **no** |

In the divergent case [T] keeps `⟨u,e⟩` strictly more plausible than `⟨v,f⟩`, while [MBD] can make
them equiplausible, destroying a belief [T] retains. **[T] is authoritative** — it is later, and
it is the reading consistent with action priority (a strict event preference must not be washed
out by the state order).

*Test obligation:* implement both rules behind a feature flag, assert they agree on every worked
example in §8, and assert the divergent configuration is reachable by at least one constructed
case. This guards against having transcribed the wrong rule.

### 3.6 The three constructions

**Ontic** ([T] Def. 4), with conditional effects. Base case (`P⁺`, `P⁻` the positive and
negative literals of the `causes` list):

`E = {e^c, e^⊤}`, `Q = {⟨⟨e^c, e^⊤⟩, N⟩}`, `pre(e^c) = a_pre`, `pre(e^⊤) = ⊤`,
`add(e^c) = P⁺`, `del(e^c) = P⁻`, `Γ = {e^c}`.

*Conflict note:* [MBD] line 188 defines `P⁻ := {p | lᵢ = ¬p for some i **and p ∉ P⁺**}` —
tolerating contradictory literals by letting `add` silently win. [T] omits the exclusion and
instead *forbids* contradictory literals as a well-formedness rule. delhi follows [T]: a
contradiction is a diagnostic, not a silent precedence rule. ([MBD] also carries a vestigial
`post` map at line 193 that its own tuple definition does not include; not ported.)

With conditional effects, `e^c` splits into one event per realizable outcome, with mutually
exclusive preconditions `a_pre ∧ (condition combination yielding that outcome)`, each with an
`N` edge to `e^⊤`. Mutual exclusivity preserves the "exactly one designated event" applicability
requirement. **This is the highest-risk part of the spec**; see §3.7.

**Announcement** ([T] Def. 3), *pending the §5.3 fix*:

`E = {e^φ, e^¬φ, e^⊤}`,
`Q = {⟨⟨e^φ,e^¬φ⟩, PN⟩, ⟨⟨e^¬φ,e^φ⟩, FPN⟩, ⟨⟨e^φ,e^⊤⟩, N⟩, ⟨⟨e^¬φ,e^⊤⟩, N⟩}`,
`pre(e^φ) = a_pre ∧ φ`, `pre(e^¬φ) = a_pre ∧ ¬φ`, `pre(e^⊤) = ⊤`,
`add = del = ∅`, `Γ = {e^φ, e^¬φ}`.

**Sensing** ([T] Fig. 5.2): as announcement but `⟨⟨e^¬φ,e^φ⟩, PN⟩` instead of `FPN`, so full
observers can epistemically distinguish the two events and thereby come to *know* whether `φ`.

### 3.7 The two known defects and their acceptance criteria

**(a) The §5.3 announcement defect.** [T] §5.3 states: given `a announces φ`, full observer `j`,
partial observer `i`, "we have that `j` comes to know that `i` believes *that* `φ` (and not just
*whether* `φ`)."

*Hypothesis:* the cause is that mB's construction has only θ-style events, whereas mA-local
([KR24] eq. 10) uses two families — θ-events and τ-events — introduced precisely so that
"partial observers have access between θ-events and τ-events, expressing their uncertainty about
what has been observed or announced." The fix is to import the θ/τ split into mB's announcement
construction.

*Acceptance test (independent of the hypothesis):* after `a announces φ` with `j` a full observer
and `i` a partial observer, the resulting state must entail `K[j](B[i]φ | B[i]!φ)` and must **not**
entail `K[j]B[i]φ`.

**(b) Conditional effects.** [T] §5.3 sanctions compiling `a causes p if φ` into two actions with
`φ` / `¬φ` pushed into preconditions. **This is sound only when the actor knows whether `φ` holds**;
when the actor is uncertain, splitting changes which actions are executable, so it is not
semantics-preserving in general. delhi implements the direct event-splitting construction and
retains compilation as a documented alternative for domains where the actor knows `φ`.

The subtle question is what observers learn about effect *conditions*. **This is fully specified
in [KR21] §4.1** — it is a port, not an invention:

- `φ_uαf := ⋁ {φ | "if φ then α causes f to become l" ∈ Γ and V(f,u) ≠ l}` — the disjunction of
  conditions of effects that would *alter* `f` at `u`. Effects that would not change `f` contribute
  nothing.
- `𝒦^eff_u := ⋀_{f ∈ ℱ} (φ_uαf if u ⊨ φ_uαf, else ¬φ_uαf)` (eq. 8) — an observer who sees `f`
  change learns that at least one responsible condition held, *without* learning which; an observer
  who sees `f` not change learns none held.
- `𝒦^obs_iu` (eq. 9) — what an agent learns about her own and others' observability, split by
  observer class.
- `𝒦^α_iu` (eq. 10) — combined per class: full observers get `det ∧ eff ∧ obs`, partial observers
  `eff ∧ obs`, oblivious agents `obs` only.
- **Theorem 1** ([KR21]) — all acquired knowledge is true. This is the property test.

The commented-out `intermediateTransition` in `[J] actions/Action.java` is a partial transcription
of exactly these equations (its comments cite "equation 4.6" … "equation 4.12"), abandoned mid-way
— note its `m.get(m).add(c)` typo, which is why it never worked.

*Acceptance tests:* (i) [KR21] Theorem 1 as a property — every formula in `𝒦^α_iu` holds at `u`;
(ii) a full observer of `a causes p if φ` comes to know `p` iff `φ` held, and learns `φ`'s truth
value only where the effect made it discernible; (iii) when two conditions could each have caused
the same change, the observer learns the *disjunction* and not either disjunct.

### 3.8 Hypothetical actions — a gap in mB (D7)

[KR21] §7 identifies its "most significant difference" from KR2020 as the treatment of oblivious
agents: an agent oblivious to an action must consider **every** action she could not have ruled
out, including `No-op`. Formally `Hᵢ` at [KR21] eq. 23:

```
Hᵢ = { M ×̄ α | α ∈ 𝒜⁰, ∃u ∈ S. i ∈ O(α,u), ∀"α requires φ" ∈ Γ. u ⊨ φ }
```

where `𝒜⁰ = 𝒜 ∪ {No-op}`. Eqs. 26 and 29 then interconnect all sub-models in `Hᵢ`, since an
oblivious agent does not know which sub-model she is in.

**The demonstrating case — Bicycle-3** ([KR21] §7): T can perform `tim_look` *or* `tim_play`
(which `causes p`); M is oblivious to both. M must not come to know `¬p`. [KR21] Fig. 7 is correct;
Fig. 8 shows KR2020 getting it wrong, with M knowing `¬p`.

**mB appears to inherit that defect.** Its transition is `⟨M,d⟩ × a`, indexed by the single action
that occurred, and non-observers reach only the `e^⊤` "nothing happened" event. Under mB+,
`tim_look` has no ontic effects and no source world satisfies `p`, so no resulting world satisfies
`p`, and M knows `¬p`. [T] §5.3's Discussion lists two mB limitations (incomplete bisimulation, the
nested-announcement issue) and does **not** list this one, so it reads as an unacknowledged gap.

**Two qualifications.** First, the above is a derivation from the definitions, not a claim either
document makes; it is therefore recorded as a *test*, not an assumption. Second, at the planner
level (v0.2) `PERSPECTIVE` ([T] §6.2) partially routes around it: several g-states collapse into one
p-node, so M's uncertainty about T's action lives in a p-node referencing multiple g-states. That
suffices for search but not for a model checker, which is precisely what v0.1 is.

**v0.1 treatment (D7).** Add Bicycle-3 to the figure suite as a `#[should_panic]` test asserting
`!K[m]!p`, so the gap is pinned and demonstrated rather than assumed, and document it as a known
limitation. Do **not** modify Def. 2 in v0.1: Def. 2 is the one part of mB with published
frame-preservation proofs ([T] §9.2.1), extending it to `Hᵢ` would require re-establishing them,
and it would widen the transition interface from one action to an action *set*.

Options deferred to v0.2, in §11.

---

## 4. Architecture

```
delhi/
├── crates/
│   ├── delhi-syntax/   formula AST, L_GB, symbol interning, hash-consing
│   ├── delhi-core/     backend-agnostic traits
│   ├── delhi-mb/       mB+ backend: models, action models, update, equivalence
│   ├── delhi-lang/     lexer → parser → AST → typecheck/ground → IR
│   └── delhi-cli/      the `delhi` binary
└── tests/              figure reproductions, property suites
```

`delhi-syntax` has no knowledge of models: the query language is shared across backends even
though the model representation is not.

`delhi-core` defines the interface [T] §6.1 specifies for the planner — `S`, `Ŝ`, `~g`, `~p`,
`⊨g`, `⊨p`, the perspective shift `sⁱ`, the transition `×`, `app`, `β` — so v0.2's search is
generic over it without touching the semantics.

### 4.1 Representation decisions

**Hash-consed formulas.** Formulas live in an arena; identical subterms share a `FormulaId`.
Structural equality is an integer compare. Entailment memoizes on `(FormulaId, WorldId)` per
model, so a repeated subformula like `B[r]h` in a compound goal is evaluated once per world
rather than once per occurrence. `[J]` re-walks the tree every time.

**Valuations as bitsets.** After grounding the atom set is fixed, so `V(u)` is a fixed-width
bitset and `V'(⟨u,e⟩) = (V(u) ∪ add(e)) \ del(e)` is two bit operations. Valuation equality —
which drives the initial partition of bisimulation refinement — becomes a word compare rather
than a `HashSet<Fluent>` comparison.

**Relations as adjacency bitsets.** `rel[agent][u]` is the bitset of `v` with `u Rᵢ v`.
Comparability, the `→ᵢᵘ` minimal-element scan, and `C_g` reachability become bitset kernels.

**Canonical state keys.** This targets the bottleneck [T] §6.4 names explicitly: "the high cost
of checking semantic equivalence to construct p-nodes is the main limiting factor of this
algorithm." `[J] PlausibilityState` has `hashCode() { return 1; }` and an `equals()` that returns
`false` unconditionally, so every hash lookup degenerates to a linear scan of graph-refinement
calls.

1. Bisimulation contraction by partition refinement.
2. Canonical labelling of the contracted model: iterative colour refinement over the multiset of
   `(agent, neighbour-colour)` pairs plus valuation, with explicit tie-breaking, producing a
   byte-string key.
3. State equality is a key comparison.

**Claim boundary:** this gives hash-speed equality *up to bisimilarity*. It does not repair the
incompleteness of §5 below. That conservatism is sound and is what [T] §6.1 already assumes
("it is not assumed that the bisimulation operators are complete").

---

## 5. Incompleteness

### 5.1 Where it comes from

[T] p. 68 notes that bisimulation for multi-agent plausibility models is sound but not complete:
two states may be modally equivalent without being bisimilar. The cause is localised.

| operator | box over | fixed relation? |
|---|---|---|
| `□ᵢ` | `Rᵢ` | yes |
| `Kᵢ` | `~ᵢ` | yes |
| `Bᵢ` | `Belᵢ = {(u,v) : v ∈ →ᵢᵘ}` | yes (derived) |
| `C_g` | `(∪_{i∈g} ~ᵢ)*` | yes (derived) |
| `Bᵢ^ψ` | min of `~ᵢᵘ ∩ ⟦ψ⟧` | **no — varies with ψ** |

`Bᵢ` minimises over a *fixed* class, so it factors through a derived relation. `Bᵢ^ψ` minimises
over a set that changes with the formula, so no fixed-relation bisimulation can be complete for
it.

### 5.2 Tier 1 — measure the gap (v0.1)

Build a brute-force modal-equivalence oracle: enumerate formulas to bounded depth over the ground
atom set and compare truth sets. Property-test the discordance rate on random small model pairs —
how often is `bisimilar = false` while `equivalent = true`?

"Incomplete in the multi-agent case" is currently a footnote with no magnitude attached. Every
downstream decision depends on whether it bites on 40% of pairs or 0.1%, and the number is not
published. The oracle is also a first-class correctness test in its own right.

### 5.3 Tier 2 — complete equivalence for K/B/□/C (v0.1)

**Claim to be proved or refuted as the first task:** bisimulation over the *derived* relations
`{~ᵢ, Belᵢ, Rᵢ, C-closure}` is complete for the `K/B/□/C` fragment, by Hennessy–Milner on finite
models. This is reasoned, not cited; it must be established before anything depends on it.

**Critical constraint.** Derived-relation bisimilarity is **not a congruence for product update**.
Two states can agree on `~ᵢ`, `Belᵢ`, `C` yet differ on `Rᵢ`, and Def. 2 reads `Rᵢ` directly, so
they diverge after an action. Therefore:

- `equivalent_static(s, s')` — derived relations, complete for K/B/□/C, for the user-facing
  question and for dedup where nothing further will be applied.
- `bisimilar(s, s')` and `contract(s)` — `Rᵢ` only, sound, incomplete, preserved by update. Used
  inside the dynamics.

These are separate functions with names that say so. Conflating them is the failure mode this
section exists to prevent, and it would be miserable to debug.

`Bᵢ^ψ` queries are documented as the boundary where completeness is unavailable.

### 5.4 Tier 3 — bounded-depth merging (deferred to v0.2)

For a fixed problem, states need only be distinguished up to the modal depth its formulas can
observe: roughly `goal depth + horizon × max condition depth`. `d`-bisimulation is exactly
computable and merges strictly *more* than full bisimulation, making incompleteness irrelevant
relative to the problem.

Deferred because the depth accounting is subtle and a bookkeeping error flips the failure mode
from "conservative, merges too little" to **"unsound, merges too much"** — worse than the problem
it solves. It also has no consumer until the planner exists. When built: behind a flag, with
differential testing against exact bisimulation.

### 5.5 Explicitly not attempted

A canonical possibilities-style representation ([KR24] refs: Le, Fabiano, Son, Pontelli) would
dissolve the problem rather than manage it, but possibilities are defined for KD45/S5 relations
and extending them to preorders with conditional-belief semantics is an open research problem.

---

## 6. Surface language

### 6.1 Structure

A delhi file has sections: `types`, `objects`, `agents`, `props`, `constants?`,
(`initially` | `state`), `goal?`, `actions`. Whitespace-insensitive; `//` and `/* */` comments.

Types begin uppercase, objects and predicates lowercase, variables are `?name`. `Object` is
built-in and every type is a subtype of it.

Type expansion, object grounding, and constant folding are **parse-time only**; no type
information reaches the semantics. Constant folding matters for scale: declaring
`!adjacent(Location, Location)` then overriding specific pairs means impossible actions are never
generated, rather than being generated and repeatedly failing their preconditions.

### 6.2 Actions

Action bodies are written as mB statements so they read like theory `T` in the papers:

```
action peek(?a - Actor) {
  actor      ?a
  pre        at(?a, study)
  determines heads

  ?a    observes
  bob   aware
  alice aware if !distracted(alice)
}
```

`causes` takes an optional `if`:

```
action move(?a - Actor, ?f - Location, ?t - Location) {
  actor  ?a
  pre    at(?a, ?f) & (adjacent(?f,?t) | adjacent(?t,?f))
  causes at(?a, ?t), !at(?a, ?f)

  ?o observes if at(?o,?f) | at(?o,?t)   // ?o - Actor, scoped to the clause
}
```

### 6.3 Initial states

Declarative form, compiled to a plausibility model by a direct total construction with no search:

```
initially {
  heads, at(alice, study), at(bob, study)

  ?[carol] heads                          // uncertain whether
  B[alice] !distracted(bob)               // believes
  K[alice, bob] at(carol, hall)           // knows
  C[*] (K[alice] heads | K[alice] !heads) // common knowledge
}
```

Explicit form, for exact plausibility structure and for reproducing published figures:

```
state {
  *u <- { heads }
   v <- { }

  carol: u ~ v      // equiplausible => uncertainty
  alice: u < v      // u strictly preferred => belief
}
```

Both lower to the same model. **The explicit form is also what the pretty-printer emits**, so a
declarative state can always be inspected as the structure it built. This is deliberate: it makes
the declarative form debuggable rather than magic.

Full formula-satisfiability model synthesis is out of scope — it is satisfiability for multi-agent
doxastic logic, PSPACE-complete even for plain KD45, more delicate over locally-well-preordered
frames, and finding a canonical minimal model is harder still.

### 6.4 Compiler structure

Distinct stages — `lex → parse → AST → typecheck/ground → IR` — not `[J]`'s 996-line one-pass
`DeplToProblem` visitor. Diagnostics carry source spans. A second front-end can be added against
the IR without touching the semantics.

---

## 7. Using delhi: worked examples

This section is normative for the surface syntax and illustrative for the semantics. Every formula
here is a query delhi must accept and answer.

### 7.1 The five epistemic attitudes, in plain terms

| you write | it means | can it be wrong? |
|---|---|---|
| `h` | the coin *is* heads-up | — it's the fact itself |
| `K[a] h` | **a knows** the coin is heads | **No.** Knowledge is correct by construction. |
| `□[a] h` | **a safely believes** it: she believes it, and no *true* information could ever dislodge that | No — but it's weaker than knowledge |
| `B[a] h` | **a believes** it | **Yes.** This is the point of the system. |
| `B^ψ[a] h` | **if a learned ψ**, she would believe it | Yes — it's a hypothetical |
| `C[g] h` | it is **common knowledge** in group `g`: everyone knows it, everyone knows everyone knows, forever | No |

The three unconditional attitudes form a chain, `K[a]φ → □[a]φ → B[a]φ`, which follows from the
model structure: `K` quantifies over the whole comparability class `~ᵢᵘ`, `□` over the worlds `Rᵢ`
reaches from `u`, and `B` over the most plausible worlds in `~ᵢᵘ` — each a subset of the last.
Reading it backwards is the useful direction: a belief may be false; a *safe* belief may still be
false but nothing true will fix it; knowledge cannot be false at all.

**Why belief is not just "weak knowledge."** `B[c] !h & h` says C believes the coin is tails while
it is in fact heads. No amount of `K` can express that, because `K[c] !h` would entail `!h`. Every
false-belief task in the literature lives in this gap.

**Why safe belief earns its place.** Consider a robot deciding whether to correct a human:

```
B[human] safe & !safe          // the human wrongly believes it's safe
!□[human] safe                 // ...and that belief IS dislodgeable by true information
```

The second line is what tells the robot intervening will work. If `□[human] safe` held instead,
the human's error is entrenched and no truthful announcement will shift it — the robot needs a
different plan. Plain `B` cannot distinguish these two cases.

**Why conditional belief earns its place.** `B^ψ[a] φ` asks what `a` *would* believe on learning
`ψ`, which is a look at the plausibility ordering *underneath* the top layer. Its practical use is
**previewing belief revision before acting**:

```
B^{!heads}[carol] distracted_alice
```

"If Carol were told the coin is tails, she would conclude Alice was distracted." A planner can ask
this *before* choosing to announce, rather than announcing and inspecting the wreckage. Note
`B[a]φ ≡ B^⊤[a]φ`, which §8 records as a property test.

### 7.2 Nesting: the attitudes that actually matter

Nesting is where this system does work nothing simpler can:

```
B[alice] B[carol] !heads              // Alice thinks Carol thinks it's tails
B[alice] B[carol] !heads & K[carol] heads   // ...and Alice is WRONG: Carol knows it's heads
K[bob] K[alice] heads                 // Bob knows that Alice knows
!B[human] B[robot] heads              // the human does NOT think the robot has figured it out
```

The last line is lifted from mecaPlanner's own `example.depl`, whose full goal is:

```
goal { (B[robot] heads & !B[human] B[robot] heads)
     | (B[robot] !heads & !B[human] B[robot] !heads) }
```

In English: *"I want to find out how the coin lies without the human realising I've found out."*
That single formula is the entire justification for higher-order modalities — you cannot state that
goal at all in a system with only first-order belief.

### 7.3 Query idioms

Common things people want to say, and how they're written:

| intent | formula |
|---|---|
| a is uncertain whether φ | `!K[a]φ & !K[a]!φ` — or `?[a]φ` in an `initially` block |
| a knows whether φ (but you don't care which) | `K[a]φ \| K[a]!φ` |
| a has a **false belief** that φ | `B[a]φ & !φ` |
| a believes φ but isn't certain | `B[a]φ & !K[a]φ` |
| a considers φ possible | `K'[a]φ` — sugar for `!K[a]!φ` |
| a doesn't rule out φ | `B'[a]φ` — sugar for `!B[a]!φ` |
| a is **wrong about b's beliefs** (2nd-order false belief) | `B[a]B[b]φ & !B[b]φ` |
| a's belief in φ is entrenched | `□[a]φ` |
| telling a that ψ would make her believe φ | `B^ψ[a]φ` |
| everyone in the room knows, and knows they all know | `C[alice,bob,carol] φ` — `C[*]` for all agents |

### 7.4 A full trace: the Coin Lie scenario

Three agents (A, B, C), two propositions: `h` (coin is heads-up) and `d` (A is distracted). The
coin *is* heads. The story: A lies that it isn't; B distracts A; C peeks and learns the truth; A
never sees the peek, so A's picture of C goes stale. From [T] §5.2.5, Figs. 5.4–5.10.

```
action announce_not_heads {         // A lies
  actor     alice
  announces !h                      // NOT required to be true
  alice observes, bob observes, carol observes
}

action distract_a {                 // B distracts A
  actor  bob
  causes d
  alice observes, bob observes, carol observes
}

action peek_c {                     // C looks at the coin
  actor      carol
  determines h
  carol observes                    // sees it: comes to KNOW
  bob   aware                       // hears it: knows C learned something, not what
  alice aware if !d                 // only notices if she isn't distracted
}
```

Applying them in order, `s0 → s1 → s2 → s3`, the queries that hold at each stage:

| stage | query | plain reading |
|---|---|---|
| `s0` | `K[a] h & K[b] h` | A and B know the coin is heads |
| `s0` | `!K[c] h & !K[c] !h` | C does not know which way it lies |
| `s0` | `C[*] (K[a] h \| K[a] !h)` | everyone knows that A knows which way — common knowledge |
| `s1` | `B[c] !h & !K[c] !h` | the lie worked: C now *believes* tails, wrongly, but doesn't *know* it |
| `s1` | `K[a] h & K[b] h` | the lie changed nothing for A and B — it contradicts what they know, so it is rejected |
| `s1` | `K[a] B[c] !h` | A knows her lie landed |
| `s2` | `d` | A is now distracted |
| `s3` | `K[a] h & K[b] h & K[c] h` | everyone now knows the coin is heads |
| `s3` | **`B[a] B[c] !h`** | **but A still believes C believes tails** |
| `s3` | `B[a] B[c] !h & K[c] h` | **second-order false belief**: A is wrong about C |

The mechanism is worth stating plainly, because it generalises. **A's picture of C froze at the
moment A stopped observing.** `alice aware if !d` made A's observer status depend on `d`; once `d`
became true, A was oblivious to `peek_c`, so A's most-plausible worlds still contain the pre-peek
version of C. That is the recipe for second-order false belief:

1. Give `b` a belief (announcement or sensing).
2. Change `b`'s belief with an action `a` is **oblivious** to.
3. `a`'s belief about `b` stays stale, and is now wrong.

Note also line `s1`: **A's lie does not shift B's or A's own knowledge.** Announcements are soft
information — they are rejected outright when they contradict what an agent *knows*. That is the
whole reason `announces` confers belief rather than knowledge (§7.6).

### 7.5 Observability: `observes`, `aware`, and neither

Every action assigns each agent to exactly one of three classes, per world:

| clause | class | intuition |
|---|---|---|
| `i observes a if φ` | **full observer** (where φ holds) | sees *what* happened |
| `i aware a if φ` | **partial observer** | knows *that* something happened, not what |
| neither clause fires | **oblivious** | believes nothing happened at all |

`observes` and `aware` must never both hold for the same agent in the same world — §3.3 makes that
a lowering-time diagnostic. An agent with no clause at all is oblivious everywhere.

The `if` is the important part. **Static observability cannot produce false beliefs about
observation; dynamic observability can.** `alice aware if !d` means Alice's observer status varies
by world — so other agents who don't know whether `d` holds don't know whether Alice observed, and
can therefore be wrong about what Alice believes. This is exactly the "higher-order action
observability" of [KR24]; a fixed observer list would make Alice's status common knowledge and the
whole class of tasks would collapse.

What each class actually learns:

| action type | full observer | partial observer | oblivious |
|---|---|---|---|
| `causes` (ontic) | the effects, and that they happened | the same — mB does not distinguish full from partial for ontic actions | believes nothing happened |
| `determines φ` (sensing) | **knows** whether φ | knows *that* full observers learned whether φ — not which | believes nothing happened |
| `announces ψ` | comes to **believe** ψ, unless she knows `¬ψ` | knows either ψ or `¬ψ` was announced, not which | believes nothing happened |

"Believes nothing happened" is a *belief*, not knowledge: the action-worlds remain epistemically
accessible, so an oblivious agent knows the action *could* have occurred while believing it did
not. See [T] Prop. 5.2.8 — and §3.8 for the limitation that she does not consider *other* actions
that might have occurred instead.

### 7.6 The three action types, and when to use which

**`causes` — ontic. Changes the world.**

```
action move(?a - Actor, ?f - Location, ?t - Location) {
  actor  ?a
  pre    at(?a,?f) & (adjacent(?f,?t) | adjacent(?t,?f))
  causes at(?a,?t), !at(?a,?f)

  ?o observes if at(?o,?f) | at(?o,?t)    // anyone in either room sees it
}
```

The `?o` is scoped to its clause and expands over `Actor` — one `observes` statement per actor,
each conditioned on that actor's location. Note the single `pre` with an explicit `&` (D8, §3.3).

With conditional effects:

```
action flip_switch {
  actor  robot
  causes light_on if !broken       // works only if the bulb is sound
  causes sparks   if broken

  ?o observes if in_room(?o)
}
```

Here an observer who sees the light come on thereby learns `!broken` — they observed an effect that
only fires under that condition. When two different conditions could each have caused the *same*
change, the observer learns only the **disjunction**, not which one fired. That is the "discernible
conditions" machinery of §3.7(b), specified in [KR21] §4.1.

**`determines` — sensing. Hard information: confers knowledge, cannot be wrong.**

```
action peek_c {
  actor      carol
  determines h
  carol observes
}
```

Use for looking, measuring, reading a sensor. A full observer ends up with `K[carol] h` or
`K[carol] !h` depending on the actual value. Because it yields knowledge, `determines` takes a
**propositional** formula only (§3.3).

**`announces` — communication. Soft information: confers belief, may be false.**

```
action announce_not_heads {
  actor     alice
  announces !h                     // a lie; truth is not required
  alice observes, bob observes, carol observes
}
```

Full observers come to *believe* the announcement — unless it contradicts what they already
**know**, in which case they reject it outright. This asymmetry is not a design quirk; it is
forced. If announcements conferred knowledge, a lie would make an agent know something false, which
is impossible by definition. So an announcement reorders plausibility instead, which is exactly
what a plausibility model is for.

Unlike `determines`, `announces` takes a **modal** formula (§3.3), so agents can talk about mental
states:

```
action bob_tells_carol_about_alice {
  actor     bob
  announces K[alice] h             // "Alice knows how the coin lies"
  bob observes, carol observes
}
```

**Choosing between them**, in one line each: `causes` changes the world; `determines` is what an
agent finds out for herself and cannot be wrong about; `announces` is what one agent tells another
and may be a lie.

---

## 8. Testing

The strongest available lever: **the papers' correctness claims are universally quantified
statements**, i.e. property tests written in prose. `[J]` has no test suite at all
(`tools/Test.java` is an interactive REPL).

**L1 — Unit tests** per module.

**L2 — Figure reproduction**, snapshotted with `insta`. Every worked example in both documents:

| source | example |
|---|---|
| [T] Figs 3.1–3.3 | Second-Order Sally-Anne |
| [T] Figs 3.4–3.6 | Second-Order Coin |
| [T] Figs 3.7–3.9 | Loud Phonecall |
| [T] Figs 3.10–3.14 | Secret Distract |
| [T] Figs 4.1–4.8 | Bicycle 1/2/3 |
| [T] Figs 5.4–5.10 | Coin Lie |
| [KR24] Figs 1, 3–4 | move-marble (Sally-Anne) |
| [KR24] Figs 5–7 | Eavesdropping |
| [KR21] Figs 1–3 | Bicycle (mA-revise reference values) |
| [KR21] Figs 4–6 | Bicycle-2 — local observability; Fig. 6 is mA\*'s *wrong* answer |
| **[KR21] Figs 7–8** | **Bicycle-3 — `#[should_panic]`, the §3.8 gap** |
| [MBD] Figs 4–10 | Coin Lie under the [MBD] transition rule (§3.5.1 differential) |

Each asserts both the entailments the text claims and a pretty-printed model snapshot, so a
semantics change surfaces as a readable diff.

Three notes on this table. [T] Ch. 3 examples are mA-local and [KR21]'s are mA-revise; under mB+
they must be **re-derived**, and any divergence from the published figure is itself a finding to
record, not a test failure to suppress. [KR21] Fig. 6 and Fig. 8 are deliberately *incorrect*
outputs of prior formalisms — they are negative tests, asserting mB+ does **not** reproduce them.
And [MBD] Figs 4–10 depict the same Coin Lie scenario as [T] Figs 5.4–5.10, so running both is the
§3.5.1 differential test.

**L3 — Propositions as `proptest` properties.**

- [T] §9.2.1 (frame restriction preservation): product update of a locally-well-preordered model
  with a well-formed action model is again reflexive, transitive, and locally connected.
- [T] Prop. 5.2.1 — full observers come to believe announcements not contradicting prior knowledge
  or preconditions.
- [T] Prop. 5.2.2 — full and partial observers know that full observers come to believe them.
- [T] Props. 5.2.3–5.2.4 — sensing confers knowledge; observers know that it does.
- [T] Prop. 5.2.5 — `a causes l` ⇒ result entails `l`.
- [T] Props. 5.2.6–5.2.7 — observers learn ontic effects, and learn that observers learn them.
- [T] Prop. 5.2.8 — non-observers' beliefs unchanged on the K-free fragment.
- [KR21] Theorem 1 — all acquired knowledge is true: `∀α, i, u. ⟨M,u⟩ ⊨ 𝒦^α_iu` (§3.7(b)).

**[KR21-S] as a ready-made frame suite.** The supplementary appendix proves, for mA-revise's
*separate* `Kᵢ`/`Bᵢ`, that update preserves S5 (Thm. 2), KD45 (Thms. 5, Lemma 6), KB1 (Thm. 3), and
KB2 (Thm. 4), enumerating each frame property individually — reflexivity, symmetry, transitivity,
seriality, Euclideanness. Two uses:

1. **Now:** mB derives `Kᵢ` and `Bᵢ` from a single preorder, so KB1 (`Belᵢ ⊆ ~ᵢ`) and KB2
   (`(u,v) ∈ ~ᵢ ∧ (v,w) ∈ Belᵢ ⇒ (u,w) ∈ Belᵢ`) should hold **by construction**. That makes them
   theorems to *verify*, not axioms to assert — cheap property tests with the exact statements
   given by [KR21-S] lines 7–9.
2. **If mA-revise becomes a backend (§11):** Thms. 2–5 transcribe directly into its property suite.

**L4 — Algebraic and metamorphic properties.**

- `s ⊨ φ ⟺ ¬(s ⊨ ¬φ)`
- `Bᵢφ ≡ Bᵢ^⊤ φ`
- KB1 (`Kᵢφ ⇒ Bᵢφ`), KB2, seriality of belief
- contraction preserves entailment: `s ⊨ φ ⟺ contract(s) ⊨ φ`
- bisimilar states agree on random formulas
- canonical key soundness: `key(s) == key(s')` ⇒ `bisimilar(s, s')`
- the tier-1 oracle as a differential test

**Generators** are the real work: producing *valid* locally-well-preordered frames with useful
coverage (construct from random total preorders over random partitions, rather than generating
relations and rejecting), bounded-depth random formulas, and well-formed random action theories.

---

## 9. Interfaces

Library API as the primary surface:

```rust
let problem = delhi::load("coin_lie.delhi")?;
let s0 = problem.initial_state()?;
let s3 = s0.apply(problem.action("announce_not_heads")?)?
           .apply(problem.action("distract_a")?)?
           .apply(problem.action("peek_c")?)?;
assert!(s3.entails(f!("K[b] h & B[a] B[c] !h"))?);
```

CLI:

| command | purpose |
|---|---|
| `delhi check FILE` | parse, typecheck, ground, validate well-formedness and frame conditions |
| `delhi eval FILE -f FORMULA` | evaluate a formula in the initial state |
| `delhi step FILE -a ACTION...` | apply a sequence of actions, print the resulting state |
| `delhi show FILE` | pretty-print a state in explicit form |
| `delhi repl FILE` | interactive exploration (replaces `[J] tools/Test.java`) |
| **`delhi dot FILE`** | Graphviz output of states and event models |

`dot` is not a nicety. The figures in these papers *are* the debugging medium for this work, and
`[J]`'s answer was a bespoke 658-line `KR2021.java` that hand-generated the figures for a single
paper. Graphviz output as a first-class command puts every confusing state one command away from
being a picture.

---

## 10. Defects in mecaPlanner this design addresses

| # | Defect | Addressed by |
|---|---|---|
| 1 | No tests whatsoever | §8 |
| 2 | `Depl.g4` action syntax matches none of the ~90 corpus files | §6, new language |
| 3 | `PlausibilityState.hashCode()` returns `1`; `equals()` returns `false` unconditionally | §4.1 canonical keys |
| 4 | Event models built without the edge conditions Defs. 3–4 require | §3.6 |
| 5 | Dead commented-out mA-revise code in `Action.java` | not ported; its idea reused in §3.7(b) |
| 6 | Well-formedness as runtime `assert`, often disabled | §3.3 lowering-time diagnostics |
| 7 | Environment models as compiled Java classes | deferred to v0.2 with a registry design |
| 8 | 996-line one-pass parser visitor | §6.4 staged compiler |
| 9 | `C[g]` documented in the README but absent from both `Depl.g4` and `formulae/`; `S'` (safe belief) reserved in the grammar with no implementing class. `[J] todo` lists "common knowledge" as future work | §3.2 |
| 10 | No visualisation | §9 `delhi dot` |
| 11 | `[J]` treats repeated `precondition{…}` clauses as a **conjunction** while [T] eq. 5.1 defines `a_pre` as a **disjunction** — the implementation and the paper contradict each other | §3.3 D8: one `pre` clause, explicit `&` |
| 12 | `intermediateTransition` in `Action.java` is an abandoned transcription of [KR21] eqs. 4.6–4.12, containing the typo `m.get(m).add(c)` (should be `m.get(f)`), which is why it never worked | §3.7(b), ported properly from [KR21] §4.1 |

---

## 11. Open questions for v0.2

**Hypothetical actions (§3.8)** — the live one, informed by whatever the Bicycle-3 test shows:

1. *Extend mB+*: port `Hᵢ` ([KR21] eq. 23) into product update, unioning in the action models of
   every action an oblivious agent could not rule out, plus `No-op`. Widens the transition
   interface from one action to an action set, and requires re-establishing [T] §9.2.1's
   frame-preservation proofs for the extended Def. 2.
2. *Add mA-revise as a second backend*: it already solves this, and [KR21-S] supplies the proofs as
   a test suite. Cost: a second complete semantics using ad-hoc transitions rather than action
   models — which [T] §5 argues is the weaker foundation.
3. *Leave it to the planner*: rely on `PERSPECTIVE` collapsing g-states into p-nodes, accepting
   that the model checker cannot answer Bicycle-3.

**Backend priority.** An earlier draft of this section reasoned that mB+ subsumes mA-local, making a second
backend marginal. That remains true for mA-local but is **false for mA-revise**, which supports
hypothetical actions that mB+ does not. If exactly one second backend is built, mA-revise is now the
stronger candidate — and it is the one with a published supplementary proof appendix.

**Also open:**

- Environment-agent behavior model registry (replacing named Java classes).
- Cooperation-agnostic search ([T] Ch. 6) generic over `delhi-core` traits.
- Whether to build the DEPL importer for the EFP benchmark corpus.
- Tier-3 bounded-depth merging (§5.4).
- Whether mB+'s announcement `ψ ∈ L^P_GB` (§3.3) interacts badly with `Hᵢ`, since a modal
  announcement precondition evaluated across hypothetical sub-models may not be well defined.
