# Related systems

Epistemic planning has two traditions pulling against each other. One starts from
**expressiveness** — dynamic epistemic logic will represent anything, at the cost of the
modeller hand-building event models and of undecidable plan existence. The other starts
from **tractability** — restrict the representation until an off-the-shelf planner can be
pointed at it.

Action languages sit in between: keep a semantics grounded in DEL, but let the modeller
declare *who observes what* and derive the event models from that. delhi is in this third
camp, and specifically implements **mB** — the branch of that lineage that swapped
knowledge for belief.

## Expressiveness

| | Belief ≠ knowledge | Revision on contradicting evidence | Second-order false belief | False belief about *who observed* | Conditional `B^ψ` / safe `□` |
|---|---|---|---|---|---|
| **DEL** (Baltag–Moss–Solecki; van Ditmarsch et al.) | yes | via specific update rules | yes | yes | in extensions |
| **Baltag & Smets** (2006, 2008) | yes | yes — this is where it comes from | yes | — | **yes, its home ground** |
| **mA\*** (Baral, Gelfond, Pontelli & Son) | limited | crude — collapses *all* uncertainty | no | no | no |
| **mA\* + higher-order observability** (KR 2024) | limited | as mA\* | yes | yes | no |
| **mB** (Buckingham thesis; KR 2021) | yes | yes, preserving other uncertainty | yes | yes (local dynamic observability) | in the models, not the language |
| **mB+ / delhi** | yes | yes | yes | yes | **yes, as query operators** |
| **EFP / EFP 2.0** (Le, Fabiano, Son & Pontelli) | knowledge-oriented | — | — | — | no |
| **PDKB / RP-MEP** (Muise et al.) | yes (in the belief work) | bounded | to the depth bound | no | no |

## Machinery

| | Event models | State representation | Planner |
|---|---|---|---|
| **DEL** | hand-built per problem | Kripke models | none inherent; plan existence undecidable in general |
| **Baltag & Smets** | action-priority update | plausibility models | none — a logic, not a planning system |
| **mA\*** | derived from observability | Kripke models | yes, via ASP or forward search |
| **mB** | derived from observability | plausibility models | yes (thesis Ch. 6) |
| **delhi** | derived from observability | plausibility models, bitset-backed | **not yet** |
| **EFP 2.0** | derived | possibilities / Kripke | yes, heavily optimised |
| **PDKB** | — | proper epistemic knowledge bases, depth-bounded | yes, compiles to classical planning |

## What "mB+" means

**The name is delhi's own.** Buckingham's mB defines its object language with six clauses:
atoms, negation, conjunction, knowledge, belief, and common knowledge. Safe belief and
conditional belief are genuinely absent from it.

They are not new to the world, though — they are Baltag and Smets's operators, and mB's
plausibility models already contain everything needed to evaluate them. delhi adds them to
the *query language* rather than to the semantics, and calls the result mB+ to be clear
about which parts came from where.

What else is delhi's own rather than inherited:

- **The `ask` query system.** Patterns with a repeated hole, enumerated over modal literals
  — the PDKB representation used as a search space rather than a state representation.
- **Invariants, definitions and Horn rules** as language features.
- **The performance work.** Hash-consed formulas, bitset models and relations, memoised
  entailment, canonical state keys, and contraction wired into every trace.
- **`~R` proved sound and a congruence**, with its incompleteness measured rather than
  assumed.

## Reading the comparison honestly

Two caveats worth stating.

**The planner column is where delhi is behind.** EFP 2.0 and PDKB are planning systems with
years of optimisation; delhi is a model checker with the pieces for a planner sitting idle.
If you need plans, not answers about states, they are the mature tools today.

**"Limited" for mA\* is not a criticism.** mA\* deliberately trades expressiveness for
tractability, and the trade buys real planning performance. The table records what each
system chose, not how well it did it.
