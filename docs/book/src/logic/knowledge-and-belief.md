# Knowledge and belief

English uses "know" and "believe" almost interchangeably. Logic cannot, because the two
obey different laws — and the difference is exactly what makes a system able to model an
agent that is confidently wrong.

## The one law that separates them

**Knowledge is factive. Belief is not.**

If Alice knows the coin is heads, then the coin is heads. If Alice merely believes it, the
coin may well be tails. Written out:

```
K[alice] h  →  h        holds always
B[alice] h  →  h        does NOT hold
```

Everything else follows from taking that seriously. A system with only one attitude has to
pick: make it factive and you cannot model mistakes, or make it non-factive and you cannot
model anything an agent has actually established.

## Adding the leaning

The [previous chapter](./possible-worlds.md) gave each agent a relation meaning "cannot
tell these apart". To get belief, delhi replaces it with an **ordering**: not just which
worlds are possible, but which are more *plausible*.

```
carol: w0 < w1
```

Read it left to right: `w1` is at least as plausible as `w0`. Plausibility increases along
the arrow, and in the surface syntax it increases to the right.

> This direction is worth pinning down, because an inverted ordering is still a perfectly
> valid model — nothing will complain, and every answer will be subtly backwards.

From that one ordering, both attitudes fall out:

| | Read over | Meaning |
|---|---|---|
| `K[a] φ` | **every** world the agent considers possible | knowledge |
| `B[a] φ` | only the **most plausible** ones | belief |

Carol considers both `w0` and `w1` possible, so she does not *know* which. But `w1` is her
most plausible world, so whatever is true there is what she *believes*.

## What each obeys

The two attitudes end up satisfying different axiom systems. You do not need these to use
delhi, but they are the standard names and they say precisely what each attitude promises:

**Knowledge is S5.**

- `K[a] φ → φ` — what you know is true *(factivity)*
- `K[a] φ → K[a] K[a] φ` — you know what you know *(positive introspection)*
- `¬K[a] φ → K[a] ¬K[a] φ` — you know what you don't know *(negative introspection)*

**Belief is KD45.** The same introspection, and consistency in place of factivity:

- `B[a] φ → ¬B[a] ¬φ` — you never believe a thing and its negation *(consistency)*
- `B[a] φ → B[a] B[a] φ`, `¬B[a] φ → B[a] ¬B[a] φ` — introspection, as above

Notice what belief keeps: an agent can be *wrong*, but not *incoherent*. Sally believes the
marble is in the basket when it is not; she does not simultaneously believe it is not.

delhi verifies these as frame properties rather than assuming them — see
[Operators](../reference/operators.md).

## Two more, and why

Knowledge and belief are the two you will reach for. delhi carries two more because the
gap between them turns out to be where the useful questions live.

**Safe belief, `[][a] φ`** — true in every world *at least as plausible* as the current
one. It sits between the other two: stronger than belief, weaker than knowledge. Its point
is stability. A safe belief survives learning any true fact; a mere belief can be
overturned by one.

**Conditional belief, `B^ψ[a] φ`** — what the agent *would* believe if it learned ψ. This
is the one that makes revision predictable: you can ask what Alice's belief would become
before telling her anything, and the answer is already determined by her ordering.

```bash
$ delhi eval examples/coin_lie.delhi -f "B[carol] h"        # believes heads
true
$ delhi eval examples/coin_lie.delhi -f "B^(!h)[carol] h"   # ...but would give it up
false
```

An agent that could not be told anything is not a believer, just a fact table with extra
steps. Conditional belief is what encodes the difference.

## The whole point

Put the pieces together and you can express a state no purely fact-based system can:

```
B[carol] h  &  !h  &  K[alice] !h
```

Carol believes heads, it is tails, and Alice knows it is tails. Three agents, one coin, and
a disagreement that is not a contradiction — because belief is not factive, and each
agent's ordering is its own.

The [next chapter](./higher-order.md) goes one level up: what Alice believes about what
Carol believes, and why that is where the genuinely hard cases start.
