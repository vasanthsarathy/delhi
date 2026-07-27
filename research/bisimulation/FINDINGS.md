# Bisimulation in mB: soundness, incompleteness, and its actual cause

**Date:** 2026-07-27
**Status:** Resolved. Unblocks §6 of the design spec.
**Programs:** `soundness_probe.rs`, `gap_measurement.rs` (`rustc -O -o x x.rs && ./x`)

Notation, following the spec:

- `~R` — bisimilarity over `{Rᵢ, Rᵢ⁻¹}`. **What [T] describes and `[J]` implements.**
- `~D` — bisimilarity over `{Rᵢ, ~ᵢ, Belᵢ, C-closure}`, one relation per operator.
- `≡` — modal equivalence for the `K / B / □ / C` fragment.

---

## 1. `~R` is sound. `[J]`'s contraction does not produce wrong answers.

This was the branch that would have been serious: if bisimulation failed to preserve `Bᵢ`, then
contraction would merge states that are not interchangeable and mecaPlanner could return wrong
plans, not merely slow ones.

**It does not happen.** `soundness_probe.rs` checks the exact criterion — two worlds in the same
bisimulation block must have `→ᵢ` sets that agree *up to blocks*, which is what makes `Bᵢφ` agree
for every `φ` including nested ones:

| scope | models | violations |
|---|---|---|
| exhaustive, n=3, 1 agent | 115 | 0 |
| exhaustive, n=3, 2 agents | 2 645 | 0 |
| exhaustive, n=4, 1 agent | 2 595 | 0 |
| exhaustive, n=4, 2 agents | 448 935 | 0 |
| random, n=5…8, 2–3 agents | 24 000 000 | 0 |

`gap_measurement.rs` independently confirms it: across 451 730 exhaustive models, **zero** pairs
merged by `~R` were separated by `~D`. So `~R ⊆ ~D`.

### Proof

Let `Z` be an `Rᵢ`-bisimulation with `u Z u'`, and let `w ∈ →ᵢᵘ`. We show some `w* ∈ →ᵢᵘ'` has
`w Z w*`.

Local connectedness makes each comparability class a **total** preorder, so "level" is well defined
and `→ᵢᵘ` is exactly the top level of `u`'s class.

1. `w ∈ →ᵢᵘ` and `u` is in the class, so `u Rᵢ w`. **Forth** gives `w'` with `u' Rᵢ w'`, `w Z w'`.
2. If `w'` is maximal, take `w* = w'`. Otherwise pick `y'` strictly above `w'`.
3. **Back** on `w Z w'` gives `y` with `w Rᵢ y` and `y Z y'`.
4. `w` is maximal, so `y` is at the top level too, hence `y Rᵢ w`.
5. **Forth** on `y Z y'` gives `w''` with `y' Rᵢ w''` and `w Z w''`.
6. `level(w'') ≥ level(y') > level(w')`, and `w''` is in `u'`'s class.

Replace `w'` by `w''` and repeat. Each round strictly increases the level; levels are finite, so it
terminates at a maximal `w*` bisimilar to `w`. Symmetric in the other direction. ∎

Note the proof uses **only** `Rᵢ` forth/back — not the converse. The converse is needed for `Kᵢ`
(which quantifies over the whole class), not for `Bᵢ`.

---

## 2. `~R` is incomplete, and here is the magnitude

[T] p. 68 says the algorithm "is not complete in the multi-agent case" and attaches no number.
Measured, as the fraction of models in which `~R` separates at least one pair that `≡` identifies:

| n | agents | models | incomplete | rate |
|---|---|---|---|---|
| 2 | 1 | 8 | 0 | 0 % |
| 2 | 2 | 32 | 0 | 0 % |
| 3 | 1 | 115 | 6 | **5.22 %** |
| 3 | 2 | 2 645 | 144 | **5.44 %** |
| 4 | 1 | 2 595 | 264 | **10.17 %** |
| 4 | 2 | 448 935 | 42 120 | **9.38 %** |

Two things stand out.

**It is not a multi-agent phenomenon.** The single-agent rates are as high as the multi-agent ones.
[T] attributes the incompleteness to Andersen, Bolander, van Ditmarsch et al. (2013), whose notion
is *complete* for the single-agent case — so `[J]` is **not implementing the technique [T] says it
uses**. It is running plain Kripke partition refinement over the two relations. Corroborating
detail: `[J] PlausibilityState.reduce()` opens with `//normalize();` — commented out — and
`normalize()` is the routine that rebuilds relations from per-class minima.

**It grows with model size**, roughly doubling from n=3 to n=4. Product update makes models grow,
so this is the regime that matters.

---

## 3. The actual cause: refining against `Rᵢ⁻¹`, which no operator uses

The smallest witness (n=3, one agent):

```
   worlds 0, 1, 2      valuations: 0 ↦ a,  1 ↦ b,  2 ↦ b

   Rᵢ :  0 ⇄ 1        (0 and 1 tie at the top level)
         2 → 0, 2 → 1  (2 strictly below both)

   levels:  {2}  <  {0, 1}          →ᵢ = {0,1} at every world
```

Worlds **1 and 2 are modally equivalent** for `K/B/□/C`:

| | world 1 | world 2 |
|---|---|---|
| valuation | `b` | `b` |
| `~ᵢ` class (`K`) | `{0,1,2}` | `{0,1,2}` |
| `→ᵢ` (`B`) | `{0,1}` | `{0,1}` |
| `Rᵢ(·)` (`□`) | `{0,1}` | `{0,1,2}` |

`□` reaches an extra world from 2 — but that world *is* 2, which is equivalent to 1, so no formula
sees the difference. `~D` merges them. **`~R` splits them**, because it refines against `Rᵢ⁻¹`:
`Rᵢ⁻¹(1) = {0,1,2}` while `Rᵢ⁻¹(2) = {2}`.

And that is the whole story: **no operator in `L_GB` is a box over `Rᵢ⁻¹`.** `Kᵢ` is a box over the
*union* `~ᵢ = Rᵢ ∪ Rᵢ⁻¹`, never over the converse alone. Refining against the converse separately
discriminates on structure the language cannot express, so `~R` over-refines.

**This has nothing to do with conditional belief.** An earlier draft of spec §6.1 blamed `Bᵢ^ψ`;
that was wrong, and could not have been right, since mB as published has no `Bᵢ^ψ`.

---

## 4. `~D` is exactly modal equivalence

Every operator in the fragment is a box over a relation in `~D`'s set — `□ᵢ` over `Rᵢ`, `Kᵢ` over
`~ᵢ`, `Bᵢ` over `Belᵢ`, `C_g` over the closure. Models are finite, hence image-finite. By
Hennessy–Milner, bisimilarity over that set **is** modal equivalence: `~D = ≡`.

So §6.3's tier-2 claim holds, and for a duller reason than expected — it is Hennessy–Milner applied
to the right set of relations, not a new result.

### The correction this forces

Spec §6.1.2 argued `~D` must be *finer* than `~R` (merging fewer states), and concluded tier 2
could only be a decision procedure rather than an optimisation. **That was backwards.** `~R ⊆ ~D`,
confirmed by 451 730 exhaustive models with zero exceptions. `~D` merges **more** — about 10 % more
models admit a merge at n=4. Tier 2 is a **performance improvement**, not merely a correctness tool.

The error was assuming `~D` respects `Rᵢ⁻¹` because it respects `~ᵢ`. It does not: back-and-forth
on a union does not imply back-and-forth on each part.

---

## 5. What remains open

**Is `~D` a congruence for product update?** Needed before `~D` may replace `~R` *inside* the
dynamics. [T] Def. 2 reads `u ~ᵢ v`, `u Rᵢ v`, and `Q(e,f)(i)` evaluated at `u` and `v`. The first
two are `~D` relations; the third is a propositional formula, preserved because `~D ⊆ ≡`. So it
plausibly holds, but it is not proved and not tested — testing needs product update implemented.

Until then the safe configuration stands: `~R` inside the dynamics (sound, congruence by the
standard DEL argument), `~D` at the boundary. If the congruence result goes through, `~D` can be
used throughout and the ~10 % merge improvement applies to search as well.

`Bᵢ^ψ` remains outside any fixed-relation bisimulation (spec §6.1.1). That part of the original
analysis was correct and is unaffected.
