# delhi

An epistemic model checker and reasoning system. It represents what agents know and
believe, how those attitudes change when things happen, and — the part that makes it
interesting — how an agent can end up confidently wrong about what another agent believes.

Written in Rust, no runtime dependencies.

```
$ delhi eval examples/coin_lie.delhi -f "B[carol] h"
true
```

## Why

Most planning systems track facts. delhi tracks *attitudes toward* facts, per agent, and
keeps them straight through announcements, sensing, and actions that some agents witness
and others miss. It is built on **mB+**: plausibility models where each agent orders
possible worlds by how believable they are, which is what lets an agent be wrong now and
recover later when better evidence arrives.

That ordering is the whole trick. Knowledge is what holds across everything an agent
considers possible; belief is what holds across the ones it finds *most* plausible. When a
lie lands, the agent's ordering shifts without its knowledge changing — and when the truth
arrives, the ordering shifts back.

## Quick start

```bash
cargo build --release
cargo run -p delhi-cli -- check examples/coin_lie.delhi
```

Or explore interactively:

```bash
cargo run -p delhi-cli -- repl examples/coin_lie.delhi
```

## The language

A `.delhi` file declares a signature, an initial state, an optional goal, and actions.
Here is the whole of `examples/coin_lie.delhi`, which reproduces Figures 5.4–5.10 of
Buckingham's thesis:

```
types   { Actor - Object }
objects { alice, bob, carol - Actor }
agents  { alice, bob, carol }
props   { h, d }                    // heads; distracted

initially {
    h                    // the coin really is heads
    ?[carol] h           // carol cannot tell
    B[carol] h           // but she correctly leans that way
}

goal { B[alice] B[carol] !h & K[carol] h }

actions {
    announce_not_heads() {
        actor     alice
        announces !h                  // a lie; truth is not required
        alice observes, bob observes, carol observes
    }

    distract_a() {
        actor  bob
        causes d
        alice observes, bob observes, carol observes
    }

    peek_c() {
        actor      carol
        determines h
        carol observes                // sees the coin: comes to KNOW
        bob   aware                   // hears it: knows carol learned something
        alice aware if !d             // only notices if she is not distracted
    }
}
```

Alice lies that the coin is tails. Bob distracts her. Carol peeks and learns the truth —
but Alice, being distracted, never sees that happen, so her picture of Carol goes stale.
At the end Alice believes Carol believes ¬h, while Carol *knows* h. That is the goal
formula, and it is a second-order false belief: Alice is wrong not about the world but
about someone else's mind.

### Three kinds of action

| Clause | Meaning |
|---|---|
| `causes p, !q` | changes the world. Add `if φ` for a conditional effect. |
| `determines p` | sensing. The observer comes to **know**. Propositional only. |
| `announces φ` | speech. The hearer comes to **believe** — and it may be a lie. |

### Three observer classes

| Clause | The agent… |
|---|---|
| `a observes` | sees exactly what happened |
| `a aware` | knows *something* happened, not what |
| *(neither)* | is oblivious; nothing changes for it |

Both take a condition: `alice aware if !d` makes the class depend on the state.

### Initial state

`initially { … }` is declarative — state the facts and attitudes, and the model is
constructed and then *verified* against every declaration you wrote. For cases where you
want the model itself, `state { … }` writes worlds and edges out by hand:

```
state {
  *w1 <- { h }          // `*` marks the designated (actual) world
   w0 <- { }
  carol: w0 < w1        // w1 is the more plausible of the two
}
```

`<` and `<=` point toward the *more plausible* world; `~` relates two worlds both ways.
This is also exactly what `delhi show` prints, so you can inspect a declaratively-built
state and paste the result back.

## Examples

Six domains in `examples/`, each runnable and each pinned by a test in
`crates/delhi-lang/tests/` so the file, the test, and this README cannot drift apart.

| File | What it is for |
|---|---|
| `coin_lie.delhi` | Second-order false belief, from Buckingham's thesis (Figs 5.4–5.10). The reference trace. |
| `sally_anne.delhi` | The canonical false-belief task — Wimmer & Perner (1983) |
| `ice_cream_van.delhi` | The second-order follow-up — Perner & Wimmer (1985) |
| `bicycle.delhi` | Belief revision: a lie lands, then evidence overturns it |
| `coin_in_the_box.delhi` | The standard epistemic-planning benchmark |
| `muddy_children.delhi` | The canonical multi-agent puzzle |

**Sally-Anne** is the whole reason a system like this exists. Sally puts her marble in the
basket and leaves; Anne moves it to the box; Sally returns. Asked where Sally will look,
children under about four say "the box" — they answer where the marble *is*, having no
machinery for a belief that is false. The entire task turns on one clause:

```
anne_moves() {
    causes box, !basket
    anne  observes
    sally observes if present    // she is not, so she misses it entirely
}
```

and it ends with `B[sally] basket` true while `box` is true. It also ends with
`B[anne] B[sally] basket` — Anne passing the task herself.

**The ice-cream van** goes one level up. John watches the van leave the park, so he is not
wrong about the van. Then the driver tells Mary, and John does not see that happen. He is
wrong about *Mary's mind*, which is a different and later-developing competence:
`B[john] B[mary] at_park` alongside `K[mary] !at_park`.

**The bicycle** is the argument for plausibility orderings in three lines. Mira lies that
Theo's bicycle is broken; he believes her; he looks and sees it is fine. Under a flat belief
set that sequence is a contradiction with nowhere to put the correction. Here it is just a
reordering, twice:

```
> B[theo] !broken          true      # he assumes it is fine
> :do mira_lies()
> B[theo] broken           true      # the lie lands
> K[theo] broken           false     #   ...as belief, not knowledge
> [][theo] broken          false     #   ...and not safely: evidence can dislodge it
> :do theo_looks()
> K[theo] !broken          true      # and now he knows
```

**Coin in the Box** is the benchmark, and it exists to separate three epistemic positions:
seeing, hearing, and missing entirely. Alice peeks while Bob is in earshot but not looking,
and the result is `K[alice] tail`, `!Kw[bob] tail`, and `K[bob] Kw[alice] tail` — Bob does
not learn the coin, but does learn that Alice learned it. That middle position is exactly
what `aware` is for.

**Muddy Children** ends one notch weaker than the textbook, and the file explains why at
length rather than hiding it. The timing is exactly classical — with three muddy children,
ignorance is announced twice and on the third round all three conclude together, and
deleting either round breaks it. But it ends in belief, not knowledge. An announcement in
this language reorders which worlds an agent finds plausible rather than deleting any,
because in this language announcements can be lies. No reordering can produce knowledge
while the ¬φ worlds are all still there to be considered. That is the price of being able to
model the Coin Lie at all.

## Querying

Every operator below is available in `eval`, in `goal`, and at the REPL prompt.

| Write | Reads as | True when |
|---|---|---|
| `K[a] φ` | a knows φ | φ holds everywhere a considers possible |
| `B[a] φ` | a believes φ | φ holds where a finds it most plausible |
| `[][a] φ` | a *safely* believes φ | belief no true evidence can dislodge |
| `B^ψ[a] φ` | a would believe φ given ψ | conditional belief |
| `C[a,b] φ` | a and b commonly know φ | and each knows the other knows, without end |
| `C[*] φ` | everyone commonly knows φ | `[*]` is shorthand, and only `C` takes it |
| `Kw[a] φ` | a knows whether φ | a knows which way it went |
| `Bw[a] φ` | a believes whether φ | a has taken a side |
| `?[a] φ` | a is ignorant of φ | a knows neither φ nor ¬φ |
| `??[a] φ` | a is undecided about φ | a does not even lean |
| `K'[a] φ` | a considers φ possible | dual of `K` |
| `B'[a] φ` | a has not ruled φ out | dual of `B` |
| `S'[a] φ` | — | dual of safe belief |

Connectives are `!`, `&`, `|`, `->`, with `->` loosest and right-associative, and
modalities binding tightest. Both `□` and `[]` work for safe belief, `¿` and `??` for
undecided.

The distinction between `K` and `[]` is the subtle one. Safe belief is belief that no *true*
announcement can dislodge — it is factive, so `[][a] φ` does imply `φ` — but knowledge is
strictly stronger, because it quantifies over everything the agent finds comparable rather
than only over what it finds at least as plausible as the actual world. The example above
shows both at once, in its initial state:

```
$ delhi eval examples/coin_lie.delhi -f "[][carol] h"    # true
$ delhi eval examples/coin_lie.delhi -f "K[carol] h"     # false
```

Carol safely believes the coin is heads, and no truth will talk her out of it. She still
does not know it.

## The tool

```
delhi check <FILE>              parse, ground, and validate
delhi show  <FILE>              print the initial state
delhi eval  <FILE> -f <FORMULA> evaluate a formula
delhi step  <FILE> -a <ACTION>… apply actions in sequence
delhi dot   <FILE>              Graphviz
delhi repl  <FILE>              explore interactively
```

Exit codes are scriptable: `0` success or the formula holds, `1` the file was rejected or
the formula is false, `2` a usage error or a malformed formula.

`dot` is not decoration. A model with four agents and sixteen worlds is unreadable as text
and obvious as a picture — the figures in the source papers *are* the debugging medium:

```bash
delhi dot examples/coin_lie.delhi | dot -Tpng > state.png
```

## Layout

| Crate | Holds |
|---|---|
| `delhi-syntax` | hash-consed formulas over six primitive operators, plus the derived attitudes |
| `delhi-mb` | the mB+ semantics: bitset models, frame validation, entailment, bisimulation, product update |
| `delhi-core` | the backend-agnostic trait a planner would be generic over |
| `delhi-lang` | the front end: lex → parse → ground → lower |
| `delhi-cli` | the `delhi` binary |

`delhi-lang` depends on the semantics; the semantics does not depend on the front end.

## What is new here

delhi is a reimplementation, so most of it is transcription — the mB semantics, the action
types, the observability model, and the Coin Lie figures all come from Buckingham's thesis
and the KR papers. Five things do not.

**A soundness proof and a measurement where the thesis leaves a gap.** The thesis notes on
p. 68 that its bisimulation algorithm "is not complete in the multi-agent case" and attaches
no number. The soundness question is the one that actually matters — an unsound contraction
merges states that are not interchangeable, which means wrong plans rather than slow ones —
and it was left open. `research/bisimulation/` settles both. Soundness is proved, by a
level-increasing argument using local connectedness and finiteness, then checked over 454,290
exhaustive models and 24 million random ones — zero violations. A separate sweep over 451,730
exhaustive models confirms the containment `~R ⊆ ~D` directly. The incompleteness is then
measured:

| worlds | agents | models | incomplete |
|---|---|---|---|
| 3 | 1 | 115 | **5.22 %** |
| 3 | 2 | 2,645 | **5.44 %** |
| 4 | 1 | 2,595 | **10.17 %** |
| 4 | 2 | 448,935 | **9.38 %** |

**And a diagnosis, which turned up something about the original implementation.** The rate
is as high with one agent as with two — but the thesis attributes its incompleteness to a
technique (Andersen, Bolander, van Ditmarsch et al.) that is *complete* in the single-agent
case. So the Java implementation is not running the algorithm the thesis says it runs; it is
running plain Kripke partition refinement over `Rᵢ` and its converse. The cause of the loss
is refining against `Rᵢ⁻¹` — a relation that no operator in the language is a box over. The
smallest witness is three worlds and one agent, and it is in the findings file.

**Two bisimulation notions, side by side.** `~R` is what the thesis describes and the Java
implements; `~D` refines against one relation per operator and is exactly modal equivalence
for the K/B/□/C fragment. `~R ⊆ ~D` is verified. Whether `~D` is a congruence for product
update is open, and it is worth answering: if it is, the ~10 % merge improvement applies
directly to search.

**Two transition rules, and evidence for which is authoritative.** The thesis and the mB
draft define product update differently. Both are implemented (`UpdateRule::Thesis` and
`UpdateRule::MbDraft`), and the Coin Lie turns out to be the differential case: under the
draft rule the lie does not land at all. That is a concrete argument for the thesis rule,
found by running both rather than by reading.

**A construction the source material does not have.** `initially { … }` lets you declare
facts and attitudes and get a model, rather than writing worlds and edges by hand. Facts fix
the designated valuation, `?[a] p` declarations fix which atoms vary, belief declarations
score worlds, and `u Rᵢ v` holds when the worlds are comparable and `score(v) ≥ score(u)`.
The scoring heuristic is not obviously complete, so the construction does not trust itself:
it re-checks by entailment that every declaration it was given actually holds in the model
it built, and reports the ones that do not. Its limits — nested belief, disjunction and
conditional belief are assertion-only — are reported to the author rather than silently
tolerated.

Beyond those: `B^ψ` and `□` are first-class operators here (standard in the Baltag–Smets
belief-revision tradition, but not in this action language); the surface language is delhi's
own rather than DEPL; and two known semantic gaps are pinned by tests that fail *by design*
and are marked ignored, rather than going undocumented.

## Why Rust

The honest short answer is that this problem is small-but-brutal, and constant factors decide
what is feasible.

**The state space multiplies.** Product update crosses worlds with events at every step. The
Coin Lie runs 2 → 4 → 8 → 16 worlds in three actions, and that is a toy with three agents and
two atoms. Plan search compounds it further. Nothing about the asymptotics changes with
language choice — but the point at which a problem stops fitting on a laptop moves a long way,
and that point is where the research happens.

**The hot code is exactly what interpreters are worst at.** Valuations and relation rows are
bitsets over `u64` words; entailment is memoised on `(FormulaId, WorldId)` pairs; contraction
is partition refinement over those bitsets. This is tight loops of index arithmetic and word
operations, with no vectorisable numeric work to hand off to a library. Python was the initial
plan and would have been the wrong tool once the ideas settled — not by 2×, but by the kind of
factor that changes which experiments you can run.

**Algebraic data types with exhaustive matching, which is not a nicety here.** The language has
six primitive operators, nine derived attitudes, three action kinds, and three observer
classes. Adding an operator should fail to compile in every place that must handle it. That
consideration ruled out C: the arena is hash-consed with interned ids and raw index arithmetic
over bitsets, which is precisely the code where manual memory management produces
use-after-free and out-of-bounds bugs, and where a missing switch case is silent. Odin and Zig
were considered and are decent fits, but have smaller ecosystems and no borrow checking for
aliasing-heavy arena code. OCaml or Haskell would have given the type system, at the cost of GC
pauses and less predictable memory when holding large state sets.

**Zero dependencies, deliberately.** Every crate, including the CLI — argument parsing is
hand-rolled because six subcommands do not justify a supply chain. A research artifact should
still build in five years, and a reviewer should be able to audit all of it.

The cost was real and worth naming: the borrow discipline showed up repeatedly during
implementation, mostly around holding a `&Problem` while needing `&mut Problem.store`. It slowed
things down. It also never once produced a wrong answer at runtime.

## What validates it

202 tests, plus 2 that fail by design and are marked ignored — they pin known gaps rather
than pretending they are absent.

The load-bearing one is `examples/coin_lie.delhi`, which reproduces the published figures
end to end. The same scenario exists twice: once built through the Rust API in
`crates/delhi-mb/tests/coin_lie.rs`, and once as the text file above. They must agree
assertion for assertion. If the two ever diverge, the front end is wrong and the semantics
is right. Every other example is pinned the same way, by its headline claim rather than by
merely parsing — `tests/examples.rs` asserts that Sally looks in the basket, that John is
wrong about Mary, that the muddy children conclude on the third round *and not the second*.

Beyond that: the pretty-printer round-trips through the parser under full bisimulation, so
it cannot quietly invert an edge; property tests cover frame preservation across update and
the bridge axioms between knowledge and belief; and a regression test reproduces a measured
incompleteness result from `research/bisimulation/`.

## Status

v0.1. Model checking and reasoning work. **Planning does not exist yet** — `delhi-core`
declares the interface a search would use, and nothing implements it.

`todo.md` carries the open questions and triaged follow-ons. The design is specified in
`docs/superpowers/specs/2026-07-25-delhi-core-design.md`, which is worth reading before
changing anything in `delhi-mb`.

## Background

delhi is a rewrite of mecaPlanner (Java, DEPL) with its own surface language. The semantics
comes from:

- Buckingham, *Epistemic Planning with Attitudes and Revision* (thesis) — the mB+ semantics
  and the Coin Lie figures
- Buckingham, Sarathy, Scheutz & Son, *Epistemic Planning with Perspectives* (KR 2021)
- Buckingham et al. (KR 2024)

PDFs and the original Java source are in `refs/`.
