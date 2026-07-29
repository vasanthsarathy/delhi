# Actions and who sees them

An action declaration has three parts: who does it, what changes, and who notices. The
third is where most modelling effort actually goes.

```
peek_c() {
    actor      carol       // who performs it
    determines h           // what changes
    carol observes         // who notices, and how much
    bob   aware
    alice aware if !d
}
```

## Parameters

Actions can be parameterised over declared objects. Every well-typed grounding is
generated, and any whose precondition folds to false is dropped before it is built:

```
move(?who, ?from, ?to) {
    actor  ?who
    pre    at(?who, ?from) & adjacent(?from, ?to)
    causes at(?who, ?to), !at(?who, ?from)
    ?who observes
}
```

Ground names are what you pass to `-a`: `move(alice,hall,study)`.

## What changes

| Clause | Effect |
|---|---|
| `causes p, !q` | Ontic — changes the world. `causes p if φ` for a conditional effect. |
| `determines p` | Sensing — the observer comes to **know**. Propositional only. |
| `announces φ` | Speech — the hearer comes to **believe**. May be false. |

An action has exactly one of these. `pre φ` guards any of them.

**Why `determines` is propositional only.** Sensing settles a fact by looking at the world,
and there is nothing in the world to look at that would settle `K[bob] p`. If you want an
agent to learn about another's mind, that is `announces` — someone tells them — or `aware`,
which is the subject of the rest of this chapter.

## Who notices

| Clause | The agent… |
|---|---|
| `a observes` | sees exactly what happened, outcome included |
| `a aware` | knows the action occurred, but **not how it turned out** |
| *(neither)* | is oblivious — does not even learn that anything happened |

Both clauses take a condition — `alice aware if !d` — which is evaluated in the state at
the time, so an agent's class can change during a trace.

### What `aware` buys you

It is the class people skip, and it does the most interesting work. An aware agent learns
that the actor *settled* something, without learning what:

```bash
$ delhi eval examples/coin_lie.delhi -a "distract_a()" "peek_c()" -f "K[bob] Kw[carol] h"
true      # bob heard the peek, so he knows carol knows whether
$ delhi eval examples/coin_lie.delhi -a "distract_a()" "peek_c()" -f "K[alice] Kw[carol] h"
false     # alice was distracted: she does not know it happened at all
$ delhi eval examples/coin_lie.delhi -a "distract_a()" "peek_c()" -f "?[alice] Kw[carol] h"
true      # she cannot even say whether carol knows
```

Three agents, one action, three genuinely different epistemic positions.

**How it works.** A sensing or announcing action builds three events: `ψ`, `¬ψ`, and a `⊤`
event meaning *nothing observable happened*. Each agent gets two labels:

- `ψ ↔ ¬ψ` labelled `¬observes(i)` — can the agent tell the outcomes apart?
- edges to `⊤` labelled `¬(observes(i) ∨ aware(i))` — can it tell that anything happened?

`observes` loses both. `aware` keeps the first and loses the second. Oblivious keeps both.

## An agent cannot be in two classes

Declaring the same agent as both `observes` and `aware` is an error. This matters for
parameterised actions, where a naive grounding easily produces both:

```
peek(?p) {
    actor      ?p
    determines h
    ?p observes
    ?o aware if !same(?p, ?o)      // every OTHER agent
}
```

Without that guard, grounding `?o` to `?p` would put the peeker in both classes. A clause
whose condition folds to `⊥` is dropped rather than recorded, which is what makes the
guarded form legal.

## Invariants

Constraints that must hold in every state, not only the first:

```
invariants { !((B[a] p & B[b] !p) | (B[a] !p & B[b] p)) }
```

Checked when the initial state is built and after every action. `check` refuses a domain
inconsistent with its own constraint, `step` exits 1, and the REPL reports and carries on —
exploring past a break is usually what you want.
