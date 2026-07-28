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

## What validates it

194 tests, plus 2 that fail by design and are marked ignored — they pin known gaps rather
than pretending they are absent.

The load-bearing one is `examples/coin_lie.delhi`, which reproduces the published figures
end to end. The same scenario exists twice: once built through the Rust API in
`crates/delhi-mb/tests/coin_lie.rs`, and once as the text file above. They must agree
assertion for assertion. If the two ever diverge, the front end is wrong and the semantics
is right.

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
