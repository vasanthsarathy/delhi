# Safe belief

Knowledge is what holds everywhere the agent considers possible. Belief is what holds where
it considers *most* plausible. Between them sits a third attitude that turns out to be the
interesting one, and the one hardest to guess at from its name.

Everything below is `examples/safe_belief.delhi`, so you can run it.

## Three nested sets

All three operators read φ off a set of worlds. The sets are nested:

```
        ┌───────────────────────────────────────┐
   K    │  every world the agent can't rule out  │
        │   ┌─────────────────────────────────┐  │
   □    │   │  worlds at least as plausible    │  │
        │   │      as the actual one           │  │
        │   │      ┌───────────────────┐       │  │
   B    │   │      │  the most         │       │  │
        │   │      │  plausible ones   │       │  │
        │   │      └───────────────────┘       │  │
        │   └─────────────────────────────────┘  │
        └───────────────────────────────────────┘
```

A bigger set is a stronger claim, so `K[a] φ → □[a] φ → B[a] φ`, and both arrows are
strict.

## It depends where you are standing

`K` and `B` are anchored to fixed sets — the whole class, the top of the ordering. **`□` is
measured from the actual world**, and that one difference is what gives it its character.

Three agents, one question, none of whom *knows* the answer:

```
initially {
    up                      // the server really is up
    ?[ada] up   B[ada] up   // ada cannot tell, but leans the right way
    ?[ben] up   B[ben] !up  // ben cannot tell, and leans the wrong way
    ?[cleo] up              // cleo has no leaning either way
}
```

```bash
$ delhi eval examples/safe_belief.delhi -f "[][ada] up"     # true
$ delhi eval examples/safe_belief.delhi -f "[][ben] !up"    # false
$ delhi eval examples/safe_belief.delhi -f "[][cleo] up"    # false
```

Plain belief cannot separate these — all three hold their views equally firmly, and
`B[ada] up` and `B[ben] !up` are both true. Safe belief separates them at once:

- **Ada is right.** The actual world is already her most plausible one, so there is almost
  nothing ranked above it, and her belief survives everything.
- **Ben is wrong.** His favoured worlds sit *above* reality in his own ordering, and `!up`
  has to hold in those too. It does not.
- **Cleo has no leaning.** Both worlds are equally plausible to her, so both sit above the
  actual one, and nothing non-trivial is safe.

A consequence worth naming: **`□` is factive.** `□[a] φ → φ`. You cannot safely believe
something false, because the actual world is always in the set being checked. Belief has no
such guarantee — that is the whole point of belief.

## What it actually means: undefeated by truth

The definition is geometric, but the characterisation is not:

> A safe belief is one that **no true information can overturn.**

Watch it. `gossip()` announces `up`, which is true:

```bash
$ delhi eval examples/safe_belief.delhi -a "gossip()" -f "[][ada] up"     # true — untouched
$ delhi eval examples/safe_belief.delhi -a "gossip()" -f "B[ben] up"      # true — overturned
```

Ada's belief was safe, and the truth left it alone. Ben's was not, and one true sentence
flipped it. That is not a coincidence about this domain — it is what `□` means.

The conditional-belief operator says the same thing without running anything:

```bash
$ delhi eval examples/safe_belief.delhi -f "B^up[ben] !up"     # false
```

Ben would give up `!up` on learning `up`. His belief was defeasible, and `□` is exactly the
operator that says so.

> **Undefeated by *truth*, not undefeated.** A safe belief can still be broken by a
> falsehood — see [A lie can destroy one](#a-lie-can-destroy-one-just-not-create-one)
> below. `□` promises stability against true information, and nothing more.

## How an agent acquires one

An agent safely believes φ exactly when **the actual world outranks every world where φ
fails**. Three ways to get there:

| Route | | `K` | `□` | `B` |
|---|---|---|---|---|
| **Sensing** | `check(ben)` — he looks | true | true | true |
| **True announcement** | `gossip()` — he is told, truthfully | false | **true** | true |
| **A lie** | `deny()` — ada is told a falsehood | false | **false** | true |

The middle row is the one to sit with. A truthful announcement took Ben from a *wrong*
belief to a **safe** one — without giving him knowledge. He is now right, and nothing true
can shake him, yet he still cannot rule out the alternative. That state has no name in
ordinary English, and it is what `□` is for.

The bottom row can never come out otherwise. A lie moves belief and can never make it safe,
because `□` is factive.

There is also a fourth, passive route: **already being right**. Ada acquired nothing. Her
belief was safe from the first line of the file, because it happened to match the world.

### A lie can destroy one, just not create one

```bash
$ delhi eval examples/safe_belief.delhi -f "[][ada] up"              # true
$ delhi eval examples/safe_belief.delhi -a "deny()" -f "[][ada] up"  # false
```

Safe belief is not permanent. `deny()` is false, and it still reorders Ada's worlds enough
to destroy the safety of a belief she had held safely. Being undefeatable by truth is no
protection at all against a convincing falsehood.

## Can an agent tell that its belief is safe?

Here the answer is genuinely surprising, and it is where `□` differs most from the other
two.

**No. `K[a] □[a] φ` and `K[a] φ` are equivalent.**

```bash
$ delhi eval examples/safe_belief.delhi -f "K[ada] [][ada] up"   # false
```

Ada's belief *is* safe, and she cannot establish that it is. The reason is short: `K`
quantifies over every world she cannot rule out, including her least plausible one — and
from there, "everything at least as plausible" is the entire class. So `K□φ` demands φ
throughout, which is just `Kφ`.

So an agent can never *know* it holds a safe belief that falls short of knowledge. Safe
belief and knowledge become certifiable at exactly the same moment.

**But the agent is not in the dark. It is overconfident.**

```bash
$ delhi eval examples/safe_belief.delhi -f "B[ada] [][ada] up"     # true
$ delhi eval examples/safe_belief.delhi -f "B[ben] [][ben] !up"    # true  ← but it is NOT
```

`B[a] □[a] φ` and `B[a] φ` are equivalent: **every agent believes every one of its beliefs
is safe.** From the top of your own ordering, everything looks unshakeable — that is what
being at the top means. Ben believes his belief is undefeatable, while one true sentence is
about to overturn it.

Third form, and this one behaves:

```bash
$ delhi eval examples/safe_belief.delhi -f "[][ada] [][ada] up"    # true
$ delhi eval examples/safe_belief.delhi -f "[][ben] [][ben] !up"   # false
```

`□[a] □[a] φ ↔ □[a] φ`. Safe belief *is* safely introspective, because the plausibility
relation is a preorder — transitivity gives one direction, reflexivity the other.

### What that means for modelling

`□` is, in a real sense, an **outside observer's operator**. It measures the fit between an
agent's ranking and the way things actually are, and the agent has no access to the second
half of that. Its self-report is useless, because it always says yes.

- **To let an agent verify its belief is stable**, it needs knowledge — sensing, not
  testimony.
- **To find out yourself whether a belief is stable**, ask `□[a] φ` from outside. That is
  the modeller's question, and the agent cannot answer it for you.

This is a genuine asymmetry with the other two operators, both of which an agent
introspects on perfectly: `K` is S5 and `B` is KD45, and both carry positive and negative
introspection.

## Why the operator exists at all

`K` here is S5 — infallible certainty, true in every world the agent cannot rule out. Many
epistemologists think that is too strong for the English word *knows*, and propose instead:

> knowledge is true belief that no further truth would overturn.

That is the **defeasibility analysis** of knowledge (Lehrer & Paxson; Stalnaker), and it is
exactly `□`. Safe belief is not a technical curiosity wedged between two real operators — it
is a serious candidate for what knowing *is*, sitting in the same model as the certainty
reading so you can ask for either and compare.

Baltag and Smets, whose plausibility models delhi's semantics are built on, introduce `□`
for precisely this reason. See [Further reading](../background/further-reading.md).
