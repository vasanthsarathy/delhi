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
  Ch. 6 cooperation-agnostic search, §9.2–9.3 proofs (all section numbers in this bullet are [T]'s).
- **[KR24]** Buckingham, Scheutz, Son, Fabiano. *Action Language mA\* with Higher-Order Action
  Observability*, KR 2024 (`refs/kr2024-0020-buckingham-et-al.pdf`). = [T] Ch. 3, mA-local.
- **[KR21]** Buckingham, **Sarathy**, Scheutz, Son. *A Multi-Agent Epistemic and Doxastic Action
  Language with Belief Revision and Local Dynamic Observability*, KR 2021
  (`refs/buckingham_kr2021.pdf`), with **[KR21-S]** its supplementary appendix
  (`refs/buckingham_kr2021_supplement.pdf`). = [T] Ch. 4, mA-revise.
- **[MBD]** *mB*, working draft dated 2022-02-21 (`refs/mb_draft_1.pdf`). An **earlier and
  non-equivalent** version of [T] Ch. 5; see §4.5 and §4.3. Incomplete (placeholder
  "Intuitively, …" sections, no propositions). [T] Ch. 5 supersedes it.
- **[J]** The mecaPlanner Java source (`refs/mecaPlanner-main/`), used as a reference
  implementation and as a catalogue of defects to avoid.

**Lineage.** KR2020 (Buckingham, Kasenberg, Scheutz) → [KR21] mA-revise → [T] Ch. 5 mB;
separately [KR24] mA-local. Where documents conflict, **[T] is authoritative** as the latest and
most complete, with conflicts recorded explicitly below rather than silently resolved.

**Citation convention.** A bare `§N` always refers to a section of *this* document. Every reference
into a source document carries its tag — `[T] §5.3`, `[KR21] §4.1`. Untagged references to source
sections are errors; they have already caused one round of them.

### 1.2 Non-goals for v0.1

- No planner, no search, no environment-agent behavior models.
- No DEPL importer. (Reconsidered in v0.2, when the EFP benchmark corpus has a consumer.)
- No probabilistic or graded-degree belief.
- No awareness logic (agents unaware that a proposition exists).
- No general formula-satisfiability-driven model synthesis.
- No hypothetical actions ([KR21] eq. 23). Known limitation, pinned by a failing test — see §4.8.

---

## 2. Decisions

| # | Decision | Rationale |
|---|---|---|
| D1 | **mB as the semantic base**, with backend-agnostic traits so mA-local can be added later | mB is the only language in the family covering all required features, notably untruthful announcements. [T] §6.4 already specifies the representation-agnostic interface. |
| D2 | **mB+**: add conditional belief `B^ψ` and safe belief `□`; fix the [T] §5.3 announcement defect; add conditional effects | Cheap expressivity gain over the same models; [T] §5.3 is an acknowledged defect in exactly the higher-order machinery that matters. |
| D3 | **Rust** | Enums + exhaustive matching fit formula ASTs and event models; fast hash sets; the search parallelises in v0.2. The semantics are already pinned down by proofs, so iteration speed matters less than usual. |
| D4 | **New surface language** with a type/object/grounding front-end | DEPL's grammar has drifted from its own corpus; grounding and constant folding earn their keep; the new modalities have no DEPL syntax. |
| D5 | **Initial states**: declarative form + explicit escape hatch | The declarative form is what people write; the explicit form reproduces published figures exactly and is what the pretty-printer emits. |
| D6 | **Incompleteness: tiers 1 + 2**. Both resolved ahead of implementation (§6, `research/bisimulation/`): `~R` is sound but incomplete at ~10% of 4-world models; the cause is refining against `Rᵢ⁻¹`; `~D` is exactly modal equivalence and merges more. | Tier 3 (bounded-depth) still deferred — no consumer until the planner, and it carries unsoundness risk. |
| D7 | **Hypothetical actions: test and document, do not fix in v0.1** | mB+ appears to inherit the defect [KR21] §7 exists to fix. Pin it with a failing test before changing Def. 2, the one part of mB with published proofs. See §4.8. |
| D8 | **At most one `pre` clause per action**; conjunction written explicitly | [T], [MBD], [KR21], and the `[J]` corpus disagree three ways on how multiple executability statements combine. Removing multiplicity removes the ambiguity instead of picking a side. See §4.3. |

---

## 3. Background: Kripke frames, modal systems, and bisimulation

*Orientation only — nothing here is a delhi design decision. Skip if `S5`, `KD45`, "Euclidean", and
"bisimulation" are already familiar. §4 assumes this vocabulary throughout, and §6 assumes §3.7.*

### 3.1 What a Kripke model is

Three ingredients: **worlds** (complete ways things might be), **arrows** (one set per agent,
saying which worlds look possible from where), and a **valuation** (which propositions hold in
which world). One world is **designated** — that's what is actually the case.

```
       ┌─────────┐    a     ┌─────────┐
       │    u    │ ───────► │    v    │
       │    p    │          │   ¬p    │
       └─────────┘          └─────────┘
        (actual)
```

From `u`, agent `a` considers `v` possible. Since `p` holds at `u` but not `v`, `a` **does not
know** `p` — she cannot rule out a world where it fails.

That is the whole idea: **a modality is a quantifier over arrows.** `□φ` holds at `u` when `φ`
holds at every world `u` points to. Change which arrows exist and you change what the modality
means. All the named systems below are just constraints on the arrows.

### 3.2 The five frame properties

Each property forces a corresponding axiom. The axiom is the *behaviour*; the property is the
*shape of the picture* that produces it.

**Reflexive** — `u R u`. Every world points to itself.

```
        ⟲         ⟲
      ┌─────┐   ┌─────┐
      │  u  │──►│  v  │
      └─────┘   └─────┘
```

*Plain reading:* you never rule out reality — the actual world is always among those you consider
possible. Validates **T**: `□φ → φ`. This is what makes knowledge **factive**: if you know it, it's
true. Drop reflexivity and `□` can hold at a world where `φ` is false.

**Transitive** — `u R v` and `v R w` implies `u R w`.

```
      ┌─────┐   ┌─────┐   ┌─────┐
      │  u  │──►│  v  │──►│  w  │
      └──┬──┘   └─────┘   └──▲──┘
         │                   │
         └───────────────────┘     ← this arrow is forced
```

*Plain reading:* no surprises two steps out. Validates **4**: `□φ → □□φ`, **positive
introspection** — if you know something, you know that you know it.

**Symmetric** — `u R v` implies `v R u`.

```
      ┌─────┐◄────►┌─────┐
      │  u  │      │  v  │
      └─────┘      └─────┘
```

*Plain reading:* possibility is mutual. Validates **B**: `φ → □◇φ` — whatever is actually true,
you don't rule out that you're right about it.

**Serial** — every world has at least one outgoing arrow.

```
   SERIAL                     NOT SERIAL
   ┌─────┐   ┌─────┐          ┌─────┐
   │  u  │──►│  v  │          │  u  │     ← dead end
   └─────┘   └─────┘          └─────┘
```

*Plain reading:* you always consider *something* possible. Validates **D**: `□φ → ◇φ`, equivalently
`¬□⊥` — **your beliefs are never outright contradictory**. From a dead-end world, `□φ` holds
vacuously for *every* `φ`, so the agent would "believe" both `φ` and `¬φ`.

**Euclidean** — `u R v` and `u R w` implies `v R w`.

```
             ┌─────┐
          ┌─►│  v  │
          │  └─────┘
       ┌─────┐  ▲│
       │  u  │  ││    ← v and w must see each other
       └─────┘  │▼
          │  ┌─────┐
          └─►│  w  │
             └─────┘
```

*Plain reading:* every world you consider possible agrees with every other about what you consider
possible — you have perfect access to your own state of mind. Validates **5**: `◇φ → □◇φ`,
**negative introspection** — if you *don't* believe something, you know that you don't.

### 3.3 The named systems

A system is just a bundle of these properties. Two matter here.

| system | frame properties | axioms | models |
|---|---|---|---|
| K | none | K | the bare minimum |
| T | reflexive | K, T | |
| S4 | reflexive, transitive | K, T, 4 | |
| **S5** | reflexive, transitive, symmetric (equivalently: reflexive + Euclidean) | K, T, 4, 5, B | **knowledge** |
| **KD45** | **serial**, transitive, Euclidean | K, **D**, 4, 5 | **belief** |

**S5 and KD45 differ in exactly one property, and that one difference is the entire distinction
between knowing and believing.** S5 is reflexive and so validates `T`: `Kφ → φ`. KD45 replaces
reflexivity with the weaker seriality and so validates only `D`: belief must be *consistent*, not
*true*.

```
   S5  —  KNOWLEDGE                     KD45  —  BELIEF

      ⟲          ⟲                     ┌────────────┐
   ┌──────┐   ┌──────┐                 │   u    p   │  ← ACTUAL world,
   │ u  p │◄─►│ v  p │                 └──────┬─────┘    with NO self-loop
   └──────┘   └──────┘                        │
                                              ▼      ⟲
   u is inside its own                 ┌────────────┐
   accessible set, so                  │   v   ¬p   │
   K p holds AND p is true             └────────────┘

                                       B ¬p holds — but p is actually true.
                                       A FALSE BELIEF.
```

Reflexivity is precisely what forbids the right-hand picture. Seriality permits it, which is why
belief can be wrong and knowledge cannot.

### 3.4 Two relations: the bimodal approach ([KR21] / mA-revise)

The traditional way to get both ([KR21] §2, after Hintikka) is to carry **two relations per
agent** — an S5 `Kᵢ` and a KD45 `Bᵢ` — plus bridge axioms tying them together:

- **KB1**: `Bᵢ ⊆ Kᵢ` as relations, i.e. `Kᵢφ → Bᵢφ` — you believe what you know.
- **KB2**: `(u,v) ∈ Kᵢ, (v,w) ∈ Bᵢ ⇒ (u,w) ∈ Bᵢ`, i.e. `Bᵢφ → KᵢBᵢφ` — you know what you believe.

```
   legend:  ═══►  knowledge Kᵢ (S5)
            ───►  belief Bᵢ (KD45)

            ┌──────┐ ═══════► ┌──────┐
            │  u   │          │  v   │ ⟲ ═ ─
            │  p   │ ───────► │  ¬p  │
            └──────┘          └──────┘
              ⟲ ═                ◄══════ (K is symmetric)

   Kᵢ = {(u,u),(u,v),(v,u),(v,v)}    S5   ⇒ K nothing: p fails at v
   Bᵢ = {(u,v),(v,v)}               KD45 ⇒ B ¬p, though p holds at u
   KB1: Bᵢ ⊆ Kᵢ ✓        KB2: (u,v)∈Kᵢ, (v,v)∈Bᵢ ⇒ (u,v)∈Bᵢ ✓
```

Unlike the plausibility figures later, this one shows **every** edge, including the reflexive and
serial ones the source papers omit — because with two relations the omitted edges are exactly where
KB1 and KB2 get violated by accident.

The cost is that **nothing keeps the two relations coherent as actions occur**. After an
announcement, `Bᵢ` can lose seriality, so [KR21] needs a three-stage repair ([KR21] eqs. 18–20,
`B^α1 → B^α2 → B^α`) that falls back on the knowledge relation to rebuild a serial belief relation.
That repair is what [T] §5 calls "ad-hoc state transitions."

### 3.5 mB's move: one preorder that generates both

mB carries **one relation per agent**, `Rᵢ`, meaning *"v is at least as plausible as u."* It is:

- reflexive ✓ and transitive ✓ — so a **preorder**,
- **not symmetric** — deliberately, since the asymmetry is what encodes preference,
- **locally connected** — see below.

`Rᵢ` is not an accessibility relation at all. It is a **ranking**. Knowledge and belief are then
*read off* it rather than stipulated:

```
                plausibility increases ──────────────►

   ┌────────────┐                          ┌────────────┐
   │     u      │ ───────────────────────► │     v      │
   │     p      │                          │    ¬p      │
   │  (ACTUAL)  │                          │            │
   └────────────┘                          └────────────┘
        (reflexive self-loops on both, omitted as always)

   ~ᵢᵘ   = {u, v}   comparable at all     →  !K[i]p  and  !K[i]!p
   Rᵢ(u) = {u, v}   at least as plausible →  ![][i]p
   →ᵢᵘ   = {v}      the maximum           →  B[i]!p    ← FALSE BELIEF
```

One picture, three readings. And the derived relations land exactly where they should:

- `~ᵢ := Rᵢ ∪ Rᵢ⁻¹` is reflexive, symmetric, transitive — an equivalence relation, hence **S5**.
  → knowledge.
- `Belᵢ := {(u,v) | v ∈ →ᵢᵘ}` is serial, transitive, Euclidean — hence **KD45**. → belief.
  It is *not* reflexive (a world need not be among its own most-plausible), which is exactly what
  licenses false belief.

**KB1 and KB2 now hold by construction.** `→ᵢᵘ ⊆ ~ᵢᵘ` gives KB1 immediately. For KB2: if `u ~ᵢ v`
then `~ᵢᵛ = ~ᵢᵘ`, so `→ᵢᵛ = →ᵢᵘ`. You cannot build an incoherent state, because there is only one
relation to get wrong. §9 records both as property tests rather than axioms.

**Local connectedness**, the unusual condition: whenever two worlds are joined by *any* undirected
chain of `Rᵢ` edges, they must be directly comparable.

```
   FORBIDDEN:

      ┌───┐                    ┌───┐
      │ u │──────►┌───┐◄───────│ v │
      └───┘       │ w │        └───┘
                  └───┘

      u and v are linked through w, so they must be comparable —
      but neither u──►v nor v──►u exists. The frame is illegal.

   LEGAL FIXES:   add u──►v,  or  add v──►u,  or  add both (a tie)
```

This is what makes `~ᵢ` transitive (hence an equivalence relation, hence `K` is S5) and what
guarantees `→ᵢᵘ` is never empty (hence `Belᵢ` is serial, hence `B` is KD45). Both of mB's derived
systems depend on it, so **product update must preserve it** — that is what [T] §9.2.1 proves and
what §9's property suite checks.

**What mB gives up.** `Rᵢ` itself is neither S5 nor KD45; it is S4 plus local connectedness, so the
standard results for S5/KD45 do not transfer to it directly. *(An earlier draft called this "the
root of the incompleteness discussed in §6." It is not — §6.1.3 traces that to the contraction
algorithm refining against `Rᵢ⁻¹`, a relation no operator is a box over. The two are unrelated.)*

### 3.6 Which properties hold where

| relation | refl | trans | symm | serial | Eucl | system | gives you |
|---|:---:|:---:|:---:|:---:|:---:|---|---|
| mB `Rᵢ` — **primitive** | ✓ | ✓ | ✗ | ✓ | ✗ | S4 + local connectedness | the plausibility ranking; `□` safe belief, `B^ψ` conditional belief |
| mB `~ᵢ` — derived | ✓ | ✓ | ✓ | ✓ | ✓ | **S5** | `K` knowledge |
| mB `Belᵢ` — derived | ✗ | ✓ | ✗ | ✓ | ✓ | **KD45** | `B` belief |
| [KR21] `Kᵢ` — primitive | ✓ | ✓ | ✓ | ✓ | ✓ | S5 | knowledge |
| [KR21] `Bᵢ` — primitive | ✗ | ✓ | ✗ | ✓ | ✓ | KD45 | belief |
| [KR24] `Rᵢ` — primitive | ✗ | — | ✗ | ✓ | — | serial; KD45 *assumed* | belief only — no knowledge modality |

The last row is worth noting: [KR24] requires only seriality outright and states that whether its
new semantics **preserves** KD45 across action occurrences "will be a topic of our future
investigation." mB's analogous obligation — preservation of local well-preorderedness — *is*
proved, in [T] §9.2.1. That is a point in mB's favour and a reason §9 treats those proofs as the
specification for the frame property tests.

### 3.7 Bisimulation: when are two models the same?

#### 3.7.1 The problem

Two Kripke models can be drawn completely differently — different world counts, different names —
and yet **no formula in the language can tell them apart**. You need a test for that, and you need
it for two reasons:

- **Size.** Product update (§4.5) builds `W' ⊆ W × E`, so every action multiplies the world count.
  Ten actions with a 3-event model takes 2 worlds to 118 098. Without collapsing redundant worlds
  after each step, nothing runs.
- **Termination.** A planner must recognise "I have been in this state before." If it cannot, it
  re-explores forever. [T] §6.4 names this the algorithm's main cost.

The obvious test — *"do they satisfy the same formulas?"* — is useless directly: there are infinitely
many formulas. Bisimulation is the **structural** test that stands in for it.

#### 3.7.2 The definition

A **bisimulation** is a relation `Z` between the worlds of two models such that whenever `u Z u'`:

1. **atoms** — `u` and `u'` satisfy exactly the same propositions;
2. **forth** — for every `i`, if `u Rᵢ v`, there is some `v'` with `u' Rᵢ v'` and `v Z v'`;
3. **back** — for every `i`, if `u' Rᵢ v'`, there is some `v` with `u Rᵢ v` and `v Z v'`.

Two *states* are **bisimilar** when some bisimulation relates their designated worlds.

```
        u  ────── Z ──────  u'
        │                   ┊
     Rᵢ │                   ┊ Rᵢ        FORTH: given the solid edge,
        ▼                   ▼           some dotted edge must exist,
        v  ────── Z ──────  v'          landing Z-related.

     BACK is the same picture mirrored: start from u' ──► v'.
```

The useful mental model is a **two-player game**. *Spoiler* is trying to prove the models differ;
*Duplicator* is trying to prove they don't. Spoiler picks either model and walks along an edge;
Duplicator must walk a matching edge in the other. If Duplicator can keep matching forever, the
models are bisimilar. If Spoiler can force a position where the two current worlds disagree on some
atom, they are not.

#### 3.7.3 Two worked examples

**Bisimilar, despite different shapes:**

```
   Model A                    Model B

   ┌───────┐                  ┌───────┐
   │  u  p │                  │  x  p │
   └───┬───┘                  └───┬───┘
       │                    ┌─────┴─────┐
       ▼                    ▼           ▼
   ┌───────┐            ┌───────┐   ┌───────┐
   │  v ¬p │            │  y ¬p │   │  z ¬p │
   └───────┘            └───────┘   └───────┘

   Z = { (u,x), (v,y), (v,z) }
```

B has an extra world, but `y` and `z` are indistinguishable from `v`, so duplicating a successor
changes nothing any formula can see. Check it: at `u`, `□φ` asks about `v`; at `x` it asks about `y`
*and* `z` — but both are `Z`-matched to `v` and agree on atoms, so the answer is the same. Spoiler
has no winning move. **Bisimulation does not care how many copies of a world you have.**

**Not bisimilar:**

```
   Model A                    Model C

   ┌───────┐                  ┌───────┐
   │  u  p │                  │  x  p │
   └───┬───┘                  └───┬───┘
       │                    ┌─────┴─────┐
       ▼                    ▼           ▼
   ┌───────┐            ┌───────┐   ┌───────┐
   │  v ¬p │            │  y ¬p │   │  z  p │   ← p, not ¬p
   └───────┘            └───────┘   └───────┘
```

Spoiler moves `x ──► z`. Duplicator's only reply is `u ──► v`, and `p` holds at `z` but fails at
`v`. Spoiler wins. And sure enough a formula witnesses it: `◇p` (i.e. `!□!p`) is **true** at `x`,
**false** at `u`.

*(These are generic frames drawn to isolate the idea — the sink worlds have no outgoing edges, which
a belief relation would forbid by seriality. Adding self-loops changes nothing in either argument.)*

#### 3.7.4 Soundness and completeness — the two directions

This is the distinction §6 turns on, so it is worth naming carefully.

| direction | statement | difficulty |
|---|---|---|
| **soundness** | bisimilar ⇒ modally equivalent | the easy direction; holds essentially always |
| **completeness** | modally equivalent ⇒ bisimilar | the hard direction — **this is what fails in mB** |

Soundness is what makes bisimulation *safe*: collapse two bisimilar worlds and no formula notices.
Completeness is what makes it *effective*: without it, you have states that are genuinely
interchangeable but that your algorithm refuses to merge, so you keep redundant copies and do
redundant work. **Losing completeness costs performance, not correctness** — which is why §6's
conservatism is tolerable while it is being sorted out.

**Hennessy–Milner.** For *image-finite* models (every world has finitely many successors) and the
*basic* modal language — plain boxes over fixed relations — both directions hold, and bisimilarity
*is* modal equivalence. This theorem is exactly what §6.3 tries to invoke: if `K`, `B`, `□`, `C` are
all boxes over fixed (possibly derived) relations, the fragment should behave like a basic modal
language and completeness should follow.

#### 3.7.5 Why plausibility models break it

Two features of mB sit outside the Hennessy–Milner setting:

**`Bᵢ` is defined by *maximality*, which is a global property.** `→ᵢᵘ` is "the worlds that
*everything* in the comparability class points to." Bisimulation's conditions are **local** — one
edge at a time — so it is not obvious that matching every edge preserves a property quantified over
an entire class. **It does**, as it turns out: §6.1.2 proves it, using local connectedness (each
class is a *total* preorder, so "top level" is well defined) plus finiteness. This one looked like a
problem and is not.

**`Bᵢ^ψ` maximises over a set that depends on the formula.** There is no fixed relation for a
bisimulation to be *about*, so no fixed-relation bisimulation can be complete for it (§6.1.1). This
one is a genuine obstruction, not a gap in a proof.

Neither of these is what actually makes mB incomplete. That turns out to be an artefact of the
contraction *algorithm* — refining against `Rᵢ⁻¹`, which no operator is a box over (§6.1.3).

#### 3.7.6 Contraction: the algorithm delhi actually runs

**Bisimulation contraction** (or quotienting) takes one model and returns the smallest model
bisimilar to it. Take the coarsest bisimulation *of the model with itself* — that is an equivalence
relation on worlds — and collapse each class to a single world.

The standard algorithm is **partition refinement** (Kanellakis–Smolka; Paige–Tarjan for the fast
version):

```
  1. Start: group worlds by valuation.        [ u v w ] [ x y ]

  2. Repeat: if some block has worlds that
     can reach block C and worlds that
     cannot, split it.                        [ u v ] [ w ] [ x y ]

  3. Stop when no block splits.               ← 3 worlds instead of 5
```

`[J] PlausibilityState.refineSystem` and `splitBlocks` implement exactly this, refining against both
`lessToMorePlausible` and `moreToLessPlausible` — i.e. against `Rᵢ` **and** `Rᵢ⁻¹`, which is what
makes it sound for `K` as well as `□`.

Note what contraction gives you: **the smallest model bisimilar to yours.** If bisimulation is
incomplete for your language, that is still larger than the smallest *equivalent* model. The
difference is the gap §6 exists to measure.

#### 3.7.7 Why contraction is not enough, and canonical keys

Contraction minimises **one** model. The question a planner asks is different: *"is this state one
I already have?"* Answering it by contracting both and comparing is a graph-isomorphism test, run
once per pair — and `[J]` does it as a linear scan because `hashCode()` returns a constant (§5.1).

**Canonical labelling** goes one step further: assign each contracted model a canonical byte string,
so that bisimilar models get identical strings. Then equality is a byte comparison and states can go
in a hash map. That is the difference between a bisimulation check per candidate and one hash
lookup.

| operation | what it is | used for | §  |
|---|---|---|---|
| `bisimilar(s, s')` | back-and-forth over `Rᵢ`, `Rᵢ⁻¹` | inside the dynamics — sound, preserved by update | §6.3 |
| `contract(s)` | quotient by the coarsest bisimulation | after every update, to stop worlds multiplying | §5.1 |
| `key(s)` | canonical byte string of `contract(s)` | hashing and dedup | §5.1 |
| `equivalent_static(s, s')` | ⚠ conjectured complete for `K/B/□/C` | the user-facing "are these the same?" | §6.3 |

The first three are sound-but-possibly-incomplete and that is fine. The fourth is the one making a
completeness claim, and §6.1.2 explains why that claim is currently in doubt.

---

## 4. Semantics (mB+)

### 4.1 Plausibility models — [T] §5.1.1

*Frame vocabulary — reflexive, transitive, serial, Euclidean, preorder, S5, KD45 — is explained
with diagrams in §3. §3.5 in particular shows how the single relation below yields both knowledge
and belief.*

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
  `→ᵢᵘ := →ᵢ ~ᵢᵘ`.

**Non-emptiness — a precondition [T] omits.** [T] §5.1.1 asserts "if `C ≠ ∅` then `|→ᵢ C| > 0`"
without qualification. **That is false in general.** If `C` contains two worlds from *different*
comparability classes, neither is `Rᵢ`-related to the other, so neither can be in `→ᵢ C`, and the
set is empty. The correct statement:

> `→ᵢ C` is non-empty whenever `C` is a non-empty **finite** subset of a **single comparability
> class** — because local connectedness makes `Rᵢ` restricted to that class a total preorder, and a
> finite total preorder has maxima.

Both uses in delhi satisfy this: `→ᵢ ~ᵢᵘ` is a whole class, and `→ᵢ(~ᵢᵘ ∩ ⟦ψ⟧)` for `Bᵢ^ψ` is a
subset of one class (§4.2 handles the `~ᵢᵘ ∩ ⟦ψ⟧ = ∅` case explicitly). **`→ᵢ` must therefore be
implemented with a documented precondition, not an unconditional non-empty assertion.** `[J]
PlausibilityState.getMinimum` carries a bare `assert(!min.isEmpty())` that would fire on
multi-class input; delhi's equivalent takes a class-identifier argument or debug-asserts
single-class membership.

### 4.2 The query language `L_GB` — extends [T] Def. 1

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

### 4.3 Action theories — extends [T] §5.2

An action theory `T` is a set of statements. **The numbering below is delhi's own** — [T] §5.2 and
[KR21] §3 order the same six forms differently ([T]: executable, observes, aware, causes,
determines, announces; [KR21]: observes, aware, requires, causes, determines, announces). Wherever
this spec says "form *n*" it means delhi's numbering; source citations are given separately.

**Formula typing is load-bearing** and was omitted from [MBD] (which restricts everything to
`L^P`); [T] §5.2 and [KR21] §3 agree on it:

| # | form | typing |
|---|---|---|
| 1 | `a requires φ` | `φ ∈ L^P` |
| 2 | `a causes l₀, …, lₙ` **`if φ`** *(the `if` is new in mB+)* | `lⱼ` propositional literals, `φ ∈ L^P` |

| 3 | `a determines φ` | `φ ∈ L^P` |
| 4 | `a announces ψ` | **`ψ ∈ L^P_GB` — modal, and need not be true** |
| 5 | `i observes a if φ` | `φ ∈ L^P` |
| 6 | `i aware_of a if φ` | `φ ∈ L^P` |

**Form 2 is a synthesis, not a transcription.** [T] form 4 is a *list* of literals with **no**
condition; [KR21] form 4 is a *single* fluent **with** a condition (`if φ then α causes f to become
l`). delhi takes the list *and* the condition. [KR21]'s `φ_uαf` machinery (§4.7(b)) is stated
per-fluent and carries over unchanged: for fluent `f`, collect every statement whose literal list
mentions `f` and disjoin their conditions.

**A capability both papers lack but `[J]` has.** `[J] Depl.g4` admits `causes f <- φ` and
`[J] Assignment` stores `⟨Fluent, Formula⟩` — *formula-valued postconditions*, assigning `f` the
truth value of an arbitrary `φ` (Andersen–Bolander–Jensen style). This strictly subsumes conditional
effects: `causes p if φ` is `p <- (p | φ)`. delhi v0.1 implements the conditional-effect form
because that is what [KR21] §4.1's observer machinery is specified for; whether to generalise to
assignments — and what observers should then learn — is recorded in §12.

Form 4 taking a modal `ψ` has real consequences: announcement event preconditions
(`a_pre ∧ ψ`, §4.6) are modal, so `pre` evaluation requires full modal entailment against the
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

### 4.4 Action plausibility models — [T] §5.1.2

`⟨E, Q, pre, add, del, Γ⟩` with `Q : E × E → (G → L^P)` the edge conditions,
**`pre : E → L^P_GB`**, `add, del : E → 2^P`, `Γ ⊆ E` designated.

**Typing correction — [T] is internally inconsistent here.** [T] §5.1.2 types `pre : E → L^P`
(propositional), but [T] §5.2 permits `a announces ψ` with `ψ ∈ L^P_GB` (modal), and [T] Def. 3
then sets `pre(e^ψ) = a_pre ∧ ψ` — a modal precondition, which the `L^P` typing forbids. The two
cannot both stand. delhi resolves it by widening `pre` to `L^P_GB`, since the alternative
(restricting announcements to propositional formulas) would discard expressivity that [T] §5.2 and
[KR21] §3 both state explicitly.

`Q` stays propositional: edge labels are built only from `FPN`/`PN`/`N`, which are boolean
combinations of observability conditions, and those are `L^P` by §4.3. So modal evaluation is
needed for `pre` but not for edge conditions — worth keeping distinct, because `e ⟶^{iuv} f`
(§4.5) is evaluated at two worlds per edge per agent and is the hottest loop in the update.

Edge labels:

- `FPN(i) := ⊤`
- `PN(i) := ¬⋁_{"observes a if φ" ∈ T} φ`
- `N(i) := ¬((⋁_{observes a if φ} φ) ∨ (⋁_{aware_of a if φ} φ))`

Implicit throughout: every event has a reflexive `FPN` edge for every agent; every unlisted
edge is `⊥`; every world has a reflexive `Rᵢ` edge.

### 4.5 State transition — [T] Def. 2

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

#### 4.5.1 [MBD] gives a different, non-equivalent rule

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
example in §9, and assert the divergent configuration is reachable by at least one constructed
case. This guards against having transcribed the wrong rule.

### 4.6 The three constructions

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
requirement. See §4.7(b) — no longer the highest-risk item, since [KR21] §4.1 specifies it completely.

**Announcement** ([T] Def. 3), *pending the [T] §5.3 fix*:

`E = {e^φ, e^¬φ, e^⊤}`,
`Q = {⟨⟨e^φ,e^¬φ⟩, PN⟩, ⟨⟨e^¬φ,e^φ⟩, FPN⟩, ⟨⟨e^φ,e^⊤⟩, N⟩, ⟨⟨e^¬φ,e^⊤⟩, N⟩}`,
`pre(e^φ) = a_pre ∧ φ`, `pre(e^¬φ) = a_pre ∧ ¬φ`, `pre(e^⊤) = ⊤`,
`add = del = ∅`, `Γ = {e^φ, e^¬φ}`.

**Sensing** ([T] Fig. 5.2): as announcement but `⟨⟨e^¬φ,e^φ⟩, PN⟩` instead of `FPN`, so full
observers can epistemically distinguish the two events and thereby come to *know* whether `φ`.

### 4.7 The two known defects and their acceptance criteria

**(a) The [T] §5.3 announcement defect.** [T] §5.3 states: given `a announces φ`, full observer `j`,
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

### 4.8 Hypothetical actions — a gap in mB (D7)

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

Options deferred to v0.2, in §12.

---

## 5. Architecture

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

### 5.1 Representation decisions

**Hash-consed formulas.** Formulas live in an arena; identical subterms share a `FormulaId`.
Structural equality is an integer compare. Entailment memoizes on `(FormulaId, WorldId)` per
model, so a repeated subformula like `B[r]h` in a compound goal is evaluated once per world
rather than once per occurrence. `[J]` re-walks the tree every time.

**Valuations as bitsets.** After grounding the atom set is fixed, so `V(u)` is a fixed-width
bitset and `V'(⟨u,e⟩) = (V(u) ∪ add(e)) \ del(e)` is two bit operations. Valuation equality —
which drives the initial partition of bisimulation refinement — becomes a word compare rather
than a `HashSet<Fluent>` comparison.

**Relations as adjacency bitsets.** `rel[agent][u]` is the bitset of `v` with `u Rᵢ v`.
Comparability, the `→ᵢᵘ` most-plausible-element scan, and `C_g` reachability become bitset kernels.

**Canonical state keys.** This targets the bottleneck [T] §6.4 names explicitly: "the high cost
of checking semantic equivalence to construct p-nodes is the main limiting factor of this
algorithm." `[J] PlausibilityState` has `hashCode() { return 1; }` and an `equals()` that returns
`false` for any two distinct objects, so every hash lookup degenerates to a linear scan of graph-refinement
calls.

1. Bisimulation contraction by partition refinement.
2. Canonical labelling of the contracted model: iterative colour refinement over the multiset of
   `(agent, neighbour-colour)` pairs plus valuation, with explicit tie-breaking, producing a
   byte-string key.
3. State equality is a key comparison.

**Claim boundary:** this gives hash-speed equality *up to bisimilarity*. It does not repair the
incompleteness of §6 below. That conservatism is sound and is what [T] §6.1 already assumes
("it is not assumed that the bisimulation operators are complete").

---

## 6. Incompleteness

### 6.1 Where it comes from — **resolved**

> **Status: settled.** Investigated 2026-07-27; full write-up and reproducible programs in
> `research/bisimulation/` (`FINDINGS.md`, `soundness_probe.rs`, `gap_measurement.rs`).
> An earlier draft of this section carried three mutually inconsistent conjectures; all three were
> wrong, and are replaced below. §3.7 is the prerequisite for reading it.

Notation: **`~R`** is bisimilarity over `{Rᵢ, Rᵢ⁻¹}` — what [T] describes and `[J]` implements.
**`~D`** is bisimilarity over `{Rᵢ, ~ᵢ, Belᵢ, C-closure}` — one relation per operator. **`≡`** is
modal equivalence for `K/B/□/C`.

#### 6.1.1 Which operators factor through a fixed relation

| operator | box over | fixed relation? |
|---|---|---|
| `□ᵢ` | `Rᵢ` | yes |
| `Kᵢ` | `~ᵢ = Rᵢ ∪ Rᵢ⁻¹` | yes (derived) |
| `Bᵢ` | `Belᵢ = {(u,v) : v ∈ →ᵢᵘ}` | yes (derived) |
| `C_g` | `(∪_{i∈g} ~ᵢ)*` | yes (derived) |
| `Bᵢ^ψ` | maxima of `~ᵢᵘ ∩ ⟦ψ⟧` | **no — varies with ψ** |

The last row stands: no fixed-relation bisimulation can be complete for conditional belief. The
rest of the table is what makes §6.3 work.

#### 6.1.2 `~R` is sound — `[J]` does not produce wrong answers

The dangerous possibility was that bisimulation fails to preserve `Bᵢ`, which would make
contraction *unsound* and mecaPlanner's outputs wrong rather than merely slow. **It does not.**

*Proof.* Local connectedness makes each comparability class a total preorder, so `→ᵢᵘ` is its top
level. Let `Z` be an `Rᵢ`-bisimulation, `u Z u'`, `w ∈ →ᵢᵘ`. Forth gives `w'` with `u' Rᵢ w'`,
`w Z w'`. If `w'` is not maximal, take `y'` strictly above it; back gives `y` with `w Rᵢ y`,
`y Z y'`; maximality of `w` puts `y` at the top level, so `y Rᵢ w`; forth on `y Z y'` gives `w''`
with `y' Rᵢ w''` and `w Z w''`, whence `level(w'') ≥ level(y') > level(w')`. Iterate — levels are
finite, so this terminates at a maximal `w*` with `w Z w*`. Symmetric in the other direction. ∎

Only `Rᵢ` forth/back is used; the converse is needed for `Kᵢ`, not `Bᵢ`. Checked exhaustively for
n ≤ 4 (451 730 models) and on 24 000 000 random models at n = 5…8 with 2–3 agents: **zero
violations**.

#### 6.1.3 The real cause: refining against `Rᵢ⁻¹`, which no operator uses

`[J] splitBlocks` refines against `lessToMorePlausible` **and** `moreToLessPlausible` — i.e. `Rᵢ`
and `Rᵢ⁻¹` as separate relations. But **no operator in `L_GB` is a box over `Rᵢ⁻¹`.** `Kᵢ` is a box
over the *union* `~ᵢ`, never over the converse alone. Refining against the converse separately
discriminates on structure the language cannot express, so `~R` over-refines.

Smallest witness (n=3, **one** agent), from `gap_measurement.rs`:

```
   worlds 0,1,2      valuations:  0 ↦ a,   1 ↦ b,   2 ↦ b

   Rᵢ:  0 ⇄ 1                    levels:  {2} < {0,1}
        2 → 0,  2 → 1            →ᵢ = {0,1} at every world
```

Worlds 1 and 2 agree on valuation, on `~ᵢ` class (`{0,1,2}` both), and on `→ᵢ` (`{0,1}` both).
`Rᵢ(1) = {0,1}` while `Rᵢ(2) = {0,1,2}` — but the extra world *is* 2, equivalent to 1, so no
formula sees it. `~D` merges them; `~R` splits them, because `Rᵢ⁻¹(1) = {0,1,2}` and
`Rᵢ⁻¹(2) = {2}`.

**It is not about conditional belief.** An earlier draft blamed `Bᵢ^ψ` — which cannot be right,
since mB as published has no `Bᵢ^ψ`. **Nor is it about `Rᵢ` failing to be S5 or KD45**, as §3.5
once suggested. Both remarks have been removed.

**It is also not a multi-agent phenomenon**, contrary to [T] p. 68. The single-agent rate (§6.2) is
as high as the multi-agent one. [T] cites Andersen, Bolander, van Ditmarsch et al. (2013), whose
notion *is* complete for a single agent — so `[J]` is **not implementing the technique [T] says it
uses**; it runs plain Kripke partition refinement. Corroborating: `[J] reduce()` opens with
`//normalize();`, commented out, where `normalize()` rebuilds relations from per-class minima.

### 6.2 Tier 1 — the gap, measured

Done ahead of implementation, since it decided §6.3. Fraction of models in which `~R` separates at
least one pair that `≡` identifies:

| n | agents | models | incomplete | rate |
|---|---|---|---|---|
| 2 | 1 | 8 | 0 | 0 % |
| 2 | 2 | 32 | 0 | 0 % |
| 3 | 1 | 115 | 6 | **5.22 %** |
| 3 | 2 | 2 645 | 144 | **5.44 %** |
| 4 | 1 | 2 595 | 264 | **10.17 %** |
| 4 | 2 | 448 935 | 42 120 | **9.38 %** |

The rate roughly doubles from n=3 to n=4, and product update grows models — so this is the regime
that matters. `[T]` attaches no number to "not complete"; this appears to be the first measurement.

`gap_measurement.rs` becomes a v0.1 regression test: it must keep reporting **0 unsound** for `~R`,
and delhi's own `~D` implementation must reproduce these merge counts.

### 6.3 Tier 2 — `~D` is exactly modal equivalence (v0.1)

**Established.** Every operator in the fragment is a box over a relation in `~D`'s set — `□ᵢ` over
`Rᵢ`, `Kᵢ` over `~ᵢ`, `Bᵢ` over `Belᵢ`, `C_g` over the closure — and models are finite, hence
image-finite. Hennessy–Milner (§3.7.4) then gives `~D = ≡` directly. The claim turned out duller
than expected: it is that theorem applied to the right set of relations, not a new result.

**Correction: `~D` merges MORE than `~R`, not fewer.** An earlier draft argued `~D` must be *finer*
and therefore could only ever be a decision procedure. That was backwards. The error was assuming
`~D` respects `Rᵢ⁻¹` because it respects `~ᵢ` — but back-and-forth on a *union* does not imply
back-and-forth on each part. Measured across 451 730 exhaustive models, `~R ⊆ ~D` with zero
exceptions, and `~D` admits a merge in ~10 % more models at n=4 (§6.2).

So tier 2 is a **correctness fix and a performance win at once**: it is the complete notion, *and*
it contracts harder than what `[J]` does.

**The one thing still open: is `~D` a congruence for product update?** Required before `~D` may
replace `~R` *inside* the dynamics. [T] Def. 2 reads `u ~ᵢ v`, `u Rᵢ v`, and `Q(e,f)(i)` at both
`u` and `v`. The first two are `~D` relations; the third is propositional and therefore preserved,
since `~D ⊆ ≡`. It plausibly holds, but proving it needs product update implemented, so it is a
v0.1 work item rather than a precondition for starting.

Until it is settled, ship both, named so they cannot be confused:

- `bisimilar_dynamic(s, s')` / `contract_dynamic(s)` — `~R`. Sound (§6.1.2), a congruence by the
  standard DEL argument, incomplete. Used inside the dynamics.
- `equivalent(s, s')` / `contract_full(s)` — `~D`. Complete for `K/B/□/C`. Used for the user-facing
  question and for dedup where nothing further is applied.

If the congruence result goes through, `contract_dynamic` becomes `contract_full` and the ~10 %
improvement applies to search as well. `Bᵢ^ψ` remains outside any fixed-relation bisimulation
(§6.1.1); that boundary is unaffected and is documented as such.

### 6.4 Tier 3 — bounded-depth merging (deferred to v0.2)

For a fixed problem, states need only be distinguished up to the modal depth its formulas can
observe: roughly `goal depth + horizon × max condition depth`. `d`-bisimulation is exactly
computable and merges strictly *more* than full bisimulation, making incompleteness irrelevant
relative to the problem.

Deferred because the depth accounting is subtle and a bookkeeping error flips the failure mode
from "conservative, merges too little" to **"unsound, merges too much"** — worse than the problem
it solves. It also has no consumer until the planner exists. When built: behind a flag, with
differential testing against exact bisimulation.

### 6.5 Explicitly not attempted

A canonical possibilities-style representation ([KR24] refs: Le, Fabiano, Son, Pontelli) would
dissolve the problem rather than manage it, but possibilities are defined for KD45/S5 relations
and extending them to preorders with conditional-belief semantics is an open research problem.

---

## 7. Surface language

### 7.1 Structure

A delhi file has sections: `types`, `objects`, `agents`, `props`, `constants?`,
(`initially` | `state`), `goal?`, `actions`. Whitespace-insensitive; `//` and `/* */` comments.

Types begin uppercase, objects and predicates lowercase, variables are `?name`. `Object` is
built-in and every type is a subtype of it.

Type expansion, object grounding, and constant folding are **parse-time only**; no type
information reaches the semantics. Constant folding matters for scale: declaring
`!adjacent(Location, Location)` then overriding specific pairs means impossible actions are never
generated, rather than being generated and repeatedly failing their preconditions.

### 7.2 Actions

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

### 7.3 Initial states

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

  carol: u ~ v      // equiplausible: carol can rank them but has no preference
  alice: u < v      // v strictly MORE plausible: alice believes whatever holds at v
}
```

**Notation, fixed once.** `u < v` reads *"`v` is strictly more plausible than `u`"* and lowers to
`u Rᵢ v` without `v Rᵢ u`. `u ~ v` lowers to both directions. `u <= v` lowers to `u Rᵢ v` alone
without asserting anything about the converse. The mnemonic is that **plausibility increases to the
right**, matching `u Rᵢ v` = *"`v` is at least as plausible as `u`"* (§4.1) and the arrow convention
in every figure. An earlier draft annotated `u < v` as "u strictly preferred", which is backwards;
§8.1 records why getting this backwards is easy and expensive.

Both lower to the same model. **The explicit form is also what the pretty-printer emits**, so a
declarative state can always be inspected as the structure it built. This is deliberate: it makes
the declarative form debuggable rather than magic.

Full formula-satisfiability model synthesis is out of scope — it is satisfiability for multi-agent
doxastic logic, PSPACE-complete even for plain KD45, more delicate over locally-well-preordered
frames, and finding a canonical minimal model is harder still.

### 7.4 Formula sugar

The core query language (§4.2) has six operators and gains none. Everything in the attitude
catalogue (§8.4) is a boolean combination of those six, and the parser desugars each one before
lowering, so `delhi-mb` implements six entailment cases and no more.

| surface | desugars to | name |
|---|---|---|
| `K'[a]φ` | `!K[a]!φ` | considers possible |
| `B'[a]φ` | `!B[a]!φ` | does not rule out |
| `S'[a]φ` | `!□[a]!φ` | safe-belief dual |
| `Kw[a]φ` | `K[a]φ \| K[a]!φ` | knows whether |
| `Bw[a]φ` | `B[a]φ \| B[a]!φ` | believes whether |
| `?[a]φ` | `!K[a]φ & !K[a]!φ` | ignorant whether |
| `¿[a]φ` | `!B[a]φ & !B[a]!φ` | suspends judgement |
| `C[*]φ` | `C[g]φ` over all declared agents | common knowledge, everyone |
| `K[a,b]φ` | `K[a]φ & K[b]φ` | agent lists distribute over `K`, `B`, `□`, `Kw`, `Bw`, `?`, `¿` |

`K'`, `B'`, and `S'` already exist in `[J] Depl.g4` with no implementing class (§11 row 9); here
they cost one parser rule each. Agent-list distribution matches the `[J]` corpus, which writes
`K[alice, bob] φ`. The `?[a]φ` form generalises the `initially`-only usage of §7.3 to any formula
position.

An ASCII alternative is accepted for every non-ASCII operator (`[]` for `□`, `??` for `¿`), since
requiring `□` and `¿` at the keyboard would be hostile.

### 7.5 Compiler structure

Distinct stages — `lex → parse → AST → typecheck/ground → IR` — not `[J]`'s 996-line one-pass
`DeplToProblem` visitor. Diagnostics carry source spans. A second front-end can be added against
the IR without touching the semantics.

---

## 8. Using delhi: worked examples

This section is normative for the surface syntax and illustrative for the semantics. Every formula
here is a query delhi must accept and answer.

### 8.1 Reading the model: `Rᵢ`, `~ᵢᵘ`, and `→ᵢᵘ`

Every attitude is a quantifier over some set of worlds, so the notation is worth ten lines.

There is **one relation per agent**, `Rᵢ`, and everything else is derived from it.
`u Rᵢ v` means *"agent i considers world v **at least as plausible as** world u."* It is a
preorder — reflexive (`u Rᵢ u`) and transitive — but deliberately **not symmetric**, because that
asymmetry is what encodes preference: if `u Rᵢ v` but not `v Rᵢ u`, then `i` strictly prefers `v`.

Three derived sets, all read at a world `u` (the superscript is *where you're standing*, the
subscript is *whose mind you're in*):

| notation | definition | what it is |
|---|---|---|
| `Rᵢ(u)` | `{v \| u Rᵢ v}` | worlds `i` ranks **at least as plausible as** `u` |
| `~ᵢᵘ` | `{v \| u Rᵢ v` **or** `v Rᵢ u}` | worlds `i` can **compare** with `u` at all — her *epistemic range*, everything she hasn't ruled out |
| `→ᵢᵘ` | the most plausible worlds inside `~ᵢᵘ` | her **best guess** |

`~ᵢ` is the one that looks strange, so: two worlds are `~ᵢ`-related when `i` can rank them against
each other *in either direction*. Worlds she cannot rank at all are ones she has excluded outright.
So "comparable" and "still considered possible" are the same thing, and `~ᵢᵘ` is her whole picture
of what might be the case. It is an equivalence relation, which is why `K` behaves like S5
knowledge.

These three sets are **nested**, and that single fact generates the whole attitude hierarchy:

```
~ᵢᵘ   ─ everything i considers possible ───────────── K[i] quantifies here
 └─ Rᵢ(u)  ─ ...at least as plausible as now ──────── □[i] quantifies here
     └─ →ᵢᵘ  ─ ...the very most plausible ─────────── B[i] quantifies here
```

`→ᵢᵘ ⊆ Rᵢ(u)` because `u` itself is in `~ᵢᵘ`, so anything maximal in that class is in particular
at least as plausible as `u`. `Rᵢ(u) ⊆ ~ᵢᵘ` by definition. Hence `K[i]φ → □[i]φ → B[i]φ`: the
smaller the set you quantify over, the weaker the claim.

**Worked instance** — Coin Lie `s0` ([T] Fig. 5.4), two worlds `u` (where `h`, designated) and `v`
(where `!h`). The figure draws exactly one edge, **`v ──C──► u`**, i.e. `v R_C u`: C considers the
heads-world at least as plausible as the tails-world. Reflexive edges are omitted, as in every
figure.

| | A and B | C |
|---|---|---|
| `~ᵢᵘ` | `{u}` — no edges either way, so nothing to compare | `{u, v}` — she can rank them, so both are live |
| `Rᵢ(u)` | `{u}` | `{u}` — `u R_C v` does **not** hold; only the reflexive edge leaves `u` |
| `→ᵢᵘ` | `{u}` | `{u}` — `u` is ranked at-least-as-good by *everything* in the class (`u R_C u`, `v R_C u`); `v` is not, since `u R_C v` fails |
| verdict | `K[a] h`, `K[b] h` | `!K[c] h & !K[c] !h`, and `B[c] h` |

So A and B *know* the coin is heads; C does not know, but correctly *leans* toward heads. Note this
falls out of the *shape* of the relation, with no extra machinery.

**Arrow direction matters, and it is easy to get backwards.** `u Rᵢ v` means *v* is the preferred
world, so an arrow drawn `x ──► y` puts the agent's belief at `y`. In `s0` the arrow runs `v ──► u`,
so C believes `h`. Applying `announce_not_heads` **reverses it** to `u' ──► v'` ([T] Fig. 5.6),
which is precisely what makes the lie land: C moves from correctly believing `h` to wrongly
believing `!h`. §8.5 traces the consequence. A spec draft of this table had the edge reversed and
concluded `B[c] !h` at `s0`, which would have made the lie a no-op and seeded a wrong expected value
into the figure tests — the reason §9 snapshots every figure rather than trusting prose.

### 8.2 The five attitudes, in plain terms

| you write | it means | can it be wrong? |
|---|---|---|
| `h` | the coin *is* heads-up | — it's the fact itself |
| `K[a] h` | **a knows** it | **No** — `K[a]φ → φ` |
| `□[a] h` | **a safely believes** it | **No** — `□[a]φ → φ` as well; see below |
| `B[a] h` | **a believes** it | **Yes.** This is the point of the system. |
| `B^ψ[a] h` | **if a learned ψ**, she would believe it | it's a hypothetical, so neither |
| `C[g] h` | **common knowledge** in `g`: all know it, all know all know it, forever | No |

**Why belief is not just "weak knowledge."** `B[c] !h & h` says C believes tails while it is in
fact heads. No amount of `K` can express that, because `K[c] !h` would entail `!h`. Every
false-belief task in the literature lives in that gap.

#### Knowledge vs. safe belief — the difference is *lying*

The natural objection: if no truth can dislodge a safe belief, hasn't the agent effectively got
knowledge? Both are factive — `□[a]φ` really does entail `φ`, because `Rᵢ` is reflexive so `u`
itself is among the worlds `□` quantifies over. So the difference is **not** that one can be false.

The difference is *which* worlds get ignored. `K` looks at **all** of `~ᵢᵘ`. `□` looks only at
`Rᵢ(u)` — the worlds at least as plausible as the current one — and therefore **ignores the worlds
the agent considers possible but less likely**. Safe belief is what you get by disregarding your
own long shots.

That gap is exactly where deception lives:

- **`K[a]φ`** — there is no `!φ` world anywhere in `a`'s picture. Nothing anyone says, true or
  false, can make her believe `!φ`. She will simply reject it. (This is why mB's announcement
  construction has full observers discard announcements contradicting prior knowledge, §8.7.)
- **`□[a]φ`** — `φ` is true and `a` believes it, and no *honest* information will change that,
  because conditioning on anything true can only ever move her among worlds at least as plausible
  as the actual one. But she *does* still carry a live `!φ` world, ranked below. **A lie can
  promote it.**

There is a formula that makes this exact, and it is the cleanest way to see the distinction:

```
B^{!φ}[a] φ   ≡   K[a] φ
```

*"a would still believe φ even if she were told the opposite"* is precisely *"a knows φ."* The
proof is one line: `B^ψ` quantifies over the most plausible `ψ`-worlds in `~ᵢᵘ`; with `ψ = !φ`
those are `!φ`-worlds, so `φ` fails there — **unless there are none**, i.e. unless every world in
`~ᵢᵘ` satisfies `φ`, which is `K[a]φ`.

Safe belief satisfies the weaker version, quantified over true news only:

```
□[a]φ   holds iff   B^ψ[a]φ  for every ψ that is actually true
```

So, in one line each: **knowledge is immune to any message; safe belief is immune to honest
messages but flippable by a lie; plain belief is flippable by anything.**

The useful predicate that falls out:

```
□[a] φ & !K[a] φ        // a is right about φ, and only a LIE could change that
```

which is the deception-exposure test — the set of an agent's correct beliefs that an adversary
could still overturn. `K[a]φ` marks the ones that are safe from that.

**Why conditional belief earns its place.** `B^ψ[a] φ` asks what `a` *would* believe on learning
`ψ`, reading the plausibility ordering *underneath* the top layer. Its practical use is
**previewing belief revision before acting**:

```
B^{!heads}[carol] distracted_alice
```

"If Carol were told the coin is tails, she would conclude Alice was distracted." A planner can ask
this *before* choosing to announce, rather than announcing and inspecting the wreckage. Note
`B[a]φ ≡ B^⊤[a]φ`, which §9 records as a property test.

### 8.3 Nesting: the attitudes that actually matter

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

### 8.4 The full attitude catalogue

Only the six operators of §4.2 are primitive. Everything below is a **boolean combination of
them** — which matters, because it means the surface language can grow without the semantics
growing at all. Entries marked **sugar** are desugared by the parser (§7) into the middle column;
`delhi-mb` never sees them, and §4.2 stays exactly as specified.

#### Ignorance and certainty

| attitude | formula | sugar | plain reading |
|---|---|---|---|
| knows whether | `K[a]φ \| K[a]!φ` | `Kw[a]φ` | she has settled the question, either way |
| **ignorant of whether** | `!K[a]φ & !K[a]!φ` | `?[a]φ` | she genuinely does not know which — the standard uncertainty idiom |
| believes whether | `B[a]φ \| B[a]!φ` | `Bw[a]φ` | she is committed one way or the other |
| **suspends judgement** | `!B[a]φ & !B[a]!φ` | `¿[a]φ` | stronger than ignorance: she cannot even lean. Her most-plausible worlds disagree. |
| considers possible | `!K[a]!φ` | `K'[a]φ` | compatible with everything she knows |
| does not rule out | `!B[a]!φ` | `B'[a]φ` | compatible with what she believes |

Note `?[a]φ` and `¿[a]φ` are different and the gap between them is real: an agent can be ignorant
*whether* φ while still believing φ — that is ordinary uncertain opinion, `?[a]φ & B[a]φ`.
Suspension of judgement is the rarer case where she has no opinion at all. `?[a]φ` already exists
in `initially` blocks (§7.3); this generalises it to any formula position.

#### Getting it right and getting it wrong

| attitude | formula | plain reading |
|---|---|---|
| **false belief** | `B[a]φ & !φ` | she believes something untrue — the core phenomenon |
| correct belief | `B[a]φ & φ` | she happens to be right |
| true belief that is not knowledge | `B[a]φ & φ & !K[a]φ` | right, but for all she knows she might not have been |
| **wrong about whether** | `(B[a]φ & !φ) \| (B[a]!φ & φ)` | she has taken a definite position and it is the wrong one |
| believes but isn't certain | `B[a]φ & !K[a]φ` | committed, but could be mistaken |
| **vulnerable to deception** | `□[a]φ & !K[a]φ` | right, immune to honest correction, but a lie would flip her (§8.2) |
| immune to deception | `K[a]φ` | no message of any kind can move her off φ |

#### Attitudes about other agents

| attitude | formula | plain reading |
|---|---|---|
| **2nd-order false belief** | `B[a]B[b]φ & !B[b]φ` | a is wrong about what b believes — the Sally-Anne shape |
| knows that b knows whether | `K[a](K[b]φ \| K[b]!φ)` | "she knows I found out, but not what I found" |
| knows that b is ignorant | `K[a](!K[b]φ & !K[b]!φ)` | a is sure b is still in the dark |
| **wrongly thinks b is ignorant** | `B[a](?[b]φ) & K[b]φ` | the classic setup for being outmanoeuvred |
| mutual knowledge, one level | `K[a]φ & K[b]φ` | both know — but neither need know that the other does |
| **common knowledge** | `C[a,b]φ` | strictly stronger: infinitely many levels. `C[*]` for all agents |

The distinction in the last two rows is the one people most often collapse. `K[a]φ & K[b]φ` is
consistent with each believing the other is ignorant; `C[a,b]φ` rules that out at every depth. A
public announcement is *supposed* to produce the latter — and §4.7(a) records that mB's
construction does not quite manage it, which is why this is a test rather than an assumption.

#### Hypothetical and dynamic

| attitude | formula | plain reading |
|---|---|---|
| would be persuaded by ψ | `!B[a]φ & B^ψ[a]φ` | telling her ψ would win her over to φ |
| would be unmoved by ψ | `B[a]φ & B^ψ[a]φ` | φ survives learning ψ |
| **would still believe φ if told otherwise** | `B^{!φ}[a]φ` | **equivalent to `K[a]φ`** (§8.2) |
| entrenched against honest news | `□[a]φ` | no true information changes her mind |

**Not available in mB+:** *common belief* — the transitive closure over `Belᵢ` rather than `~ᵢ`.
[KR24] uses `C_g` for exactly that, so the same symbol means different things across the two
papers (§4.2). It would be cheap to add — one more closure over a derived relation — but it is a
genuine seventh primitive rather than sugar, so it is recorded as an open question (§12) rather than
slipped in.

### 8.5 A full trace: the Coin Lie scenario

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
whole reason `announces` confers belief rather than knowledge (§8.7).

### 8.6 Observability: `observes`, `aware`, and neither

Every action assigns each agent to exactly one of three classes, per world:

| clause | class | intuition |
|---|---|---|
| `i observes a if φ` | **full observer** (where φ holds) | sees *what* happened |
| `i aware a if φ` | **partial observer** | knows *that* something happened, not what |
| neither clause fires | **oblivious** | believes nothing happened at all |

`observes` and `aware` must never both hold for the same agent in the same world — §4.3 makes that
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
not. See [T] Prop. 5.2.8 — and §4.8 for the limitation that she does not consider *other* actions
that might have occurred instead.

### 8.7 The three action types, and when to use which

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
each conditioned on that actor's location. Note the single `pre` with an explicit `&` (D8, §4.3).

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
conditions" machinery of §4.7(b), specified in [KR21] §4.1.

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
**propositional** formula only (§4.3).

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

Unlike `determines`, `announces` takes a **modal** formula (§4.3), so agents can talk about mental
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

## 9. Testing

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
| **[KR21] Figs 7–8** | **Bicycle-3 — `#[should_panic]`, the §4.8 gap** |
| [MBD] Figs 4–10 | Coin Lie under the [MBD] transition rule (§4.5.1 differential) |

Each asserts both the entailments the text claims and a pretty-printed model snapshot, so a
semantics change surfaces as a readable diff.

Three notes on this table. [T] Ch. 3 examples are mA-local and [KR21]'s are mA-revise; under mB+
they must be **re-derived**, and any divergence from the published figure is itself a finding to
record, not a test failure to suppress. [KR21] Fig. 6 and Fig. 8 are deliberately *incorrect*
outputs of prior formalisms — they are negative tests, asserting mB+ does **not** reproduce them.
And [MBD] Figs 4–10 depict the same Coin Lie scenario as [T] Figs 5.4–5.10, so running both is the
§4.5.1 differential test.

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
- [KR21] Theorem 1 — all acquired knowledge is true: `∀α, i, u. ⟨M,u⟩ ⊨ 𝒦^α_iu` (§4.7(b)).

**[KR21-S] as a ready-made frame suite.** The supplementary appendix proves, for mA-revise's
*separate* `Kᵢ`/`Bᵢ`, that update preserves S5 (Thm. 2), KD45 (Thms. 5, Lemma 6), KB1 (Thm. 3), and
KB2 (Thm. 4), enumerating each frame property individually — reflexivity, symmetry, transitivity,
seriality, Euclideanness. Two uses:

1. **Now:** mB derives `Kᵢ` and `Bᵢ` from a single preorder, so KB1 (`Belᵢ ⊆ ~ᵢ`) and KB2
   (`(u,v) ∈ ~ᵢ ∧ (v,w) ∈ Belᵢ ⇒ (u,w) ∈ Belᵢ`) should hold **by construction**. That makes them
   theorems to *verify*, not axioms to assert — cheap property tests with the exact statements
   given by [KR21-S] lines 7–9.
2. **If mA-revise becomes a backend (§12):** Thms. 2–5 transcribe directly into its property suite.

**L4 — Algebraic and metamorphic properties.**

- `s ⊨ φ ⟺ ¬(s ⊨ ¬φ)`
- `Bᵢφ ≡ Bᵢ^⊤ φ`
- KB1 (`Kᵢφ ⇒ Bᵢφ`), KB2, seriality of belief
- contraction preserves entailment: `s ⊨ φ ⟺ contract(s) ⊨ φ`
- bisimilar states agree on random formulas
- canonical keys, **both directions** — they mean different things and both are needed:
  - `key(s) == key(s')` ⇒ `bisimilar(s, s')` — **soundness**. Failure means wrongly merging distinct
    states, which corrupts results.
  - `bisimilar(s, s')` ⇒ `key(s) == key(s')` — **no false negatives**. Failure only costs
    performance (duplicate entries), but silently defeats the entire point of §5.1.
- `→ᵢ` non-emptiness precondition (§4.1): assert `→ᵢ C ≠ ∅` for single-class `C`, and assert it
  *can* be empty for a generated multi-class `C` — the latter guards against reintroducing [T]'s
  unqualified claim
- the tier-1 oracle as a differential test

**Generators** are the real work: producing *valid* locally-well-preordered frames with useful
coverage (construct from random total preorders over random partitions, rather than generating
relations and rejecting), bounded-depth random formulas, and well-formed random action theories.

---

## 10. Interfaces

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

## 11. Defects in mecaPlanner this design addresses

| # | Defect | Addressed by |
|---|---|---|
| 1 | No tests whatsoever | §9 |
| 2 | `Depl.g4` action syntax matches none of the ~90 corpus files | §7, new language |
| 3 | `PlausibilityState.hashCode()` returns `1`; `equals()` returns `false` for any two distinct objects | §5.1 canonical keys |
| ~~4~~ | ~~Event models built without the edge conditions Defs. 3–4 require~~ — **retracted, this was wrong.** `EventModel.addEdge(agent, from, to, Formula)` stores a per-`(agent, from, to)` condition, and `Action.buildEventModel` passes `full.negate()` (= `PN`), `Literal(true)` (= `FPN`), and `AndFormula.make(full.negate(), aware.negate())` (= `N`). All three constructions match [T] Def. 3, Fig. 5.2, and Def. 4 edge-for-edge. | — |
| 5 | Dead commented-out mA-revise code in `Action.java` | not ported; its idea reused in §4.7(b) |
| 6 | Well-formedness as runtime `assert`, often disabled | §4.3 lowering-time diagnostics |
| 7 | Environment models as compiled Java classes | deferred to v0.2 with a registry design |
| 8 | 996-line one-pass parser visitor | §7.5 staged compiler |
| 9 | `C[g]` documented in the README but absent from both `Depl.g4` and `formulae/`; `S'` (safe belief) reserved in the grammar with no implementing class. `[J] todo` lists "common knowledge" as future work | §4.2 |
| 10 | No visualisation | §10 `delhi dot` |
| 11 | `[J]` treats repeated `precondition{…}` clauses as a **conjunction** while [T] eq. 5.1 defines `a_pre` as a **disjunction** — the implementation and the paper contradict each other | §4.3 D8: one `pre` clause, explicit `&` |
| 12 | `intermediateTransition` in `Action.java` is an abandoned transcription of [KR21] eqs. 4.6–4.12, containing the typo `m.get(m).add(c)` (should be `m.get(f)`), which is why it never worked | §4.7(b), ported properly from [KR21] §4.1 |

---

## 12. Open questions for v0.2

**Hypothetical actions (§4.8)** — the live one, informed by whatever the Bicycle-3 test shows:

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
- Tier-3 bounded-depth merging (§6.4).
- **Common belief** as a seventh primitive: the transitive closure over `Belᵢ` rather than `~ᵢ`
  (§8.4). mB has only common *knowledge*; [KR24] uses `C_g` for common belief, so the symbol is
  overloaded across the two papers. Implementation is one more closure over a derived relation, but
  it is a genuine new operator rather than sugar, and it interacts with §6's completeness analysis
  (`Belᵢ` is derived, so its closure is derived twice over). Deliberately not slipped into v0.1.
- Whether mB+'s announcement `ψ ∈ L^P_GB` (§4.3) interacts badly with `Hᵢ`, since a modal
  announcement precondition evaluated across hypothetical sub-models may not be well defined.
- **Formula-valued postconditions** — generalising `causes l if φ` to `causes f <- φ` as
  `[J] Depl.g4` and `[J] Assignment` already do (§4.3). Strictly more expressive, but [KR21] §4.1's
  observer machinery is specified for the conditional form, so what a full observer should *learn*
  from an assignment needs deriving before it can be adopted.
- **Is `~D` a congruence for product update?** (§6.3) The last piece of the bisimulation story.
  Resolving it turns `contract_dynamic` into `contract_full` and applies the ~10 % merge
  improvement to search. Needs product update implemented first, so it is a v0.1 task, not a
  precondition. *(§6 as a whole was the previous blocker; it was resolved on 2026-07-27 —
  `research/bisimulation/FINDINGS.md`.)*
