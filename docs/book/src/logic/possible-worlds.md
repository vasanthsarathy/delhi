# Possible worlds

The whole field rests on one move. To say what an agent knows, do not describe the agent's
head — describe the **situations it cannot rule out**.

## The move

Suppose a coin has been flipped and covered. It landed heads. Alice did not see it.

We could try to model Alice's ignorance by giving her a "knows-heads" flag set to false.
That works until you ask a second question — does she know it is *tails*? Also false. Does
she know it is heads-or-tails? True. Three flags now, and they have to be kept consistent
with each other by hand.

Instead, list the situations Alice considers possible:

```
    w1: heads          w0: tails
```

She cannot tell them apart, so both are live for her. Now every question answers itself:

- **Does Alice know it is heads?** Only if `heads` is true in *every* world she considers
  possible. It is false in `w0`, so no.
- **Does she know it is heads-or-tails?** That holds in both, so yes — for free, with no
  extra bookkeeping.
- **Is she ignorant of which?** Yes: neither `heads` nor `¬heads` holds throughout.

The consistency comes from the structure rather than from discipline. That is the payoff.

## The three parts

A model is three things:

- **Worlds** — the situations in play. One of them is the *actual* one; the rest are there
  because somebody cannot rule them out.
- **A valuation** — which propositions are true in each world.
- **A relation, per agent** — which worlds that agent connects to which. Alice's relation
  links `w1` and `w0` because she cannot distinguish them.

Each agent gets its **own** relation. That is what lets Bob know the coin while Alice does
not: Bob's relation links each world only to itself, so from `w1` he sees only `w1`.

```
state {
  *w1 <- { heads }      // `*` marks the actual world
   w0 <- { }
  alice: w0 ~ w1        // alice cannot tell these apart
}                       // bob relates each world only to itself
```

That is real delhi syntax — `delhi show` prints models in exactly this form. Bob needs no
line at all: with nothing declared, he distinguishes everything.

## Reading the picture

Two habits to build, because they are what make the diagrams say anything.

**Knowledge is what survives every arrow.** To evaluate "Alice knows φ", stand in the
actual world, follow every one of Alice's arrows, and check φ in each place you land. One
counterexample is enough to defeat the claim. This is why knowledge is expensive: it takes
*all* the possibilities agreeing.

**More arrows mean less knowledge.** An agent who connects everything to everything knows
nothing beyond what is true everywhere. An agent whose arrows go only from each world to
itself knows the actual world exactly. Learning something means *deleting* arrows.

That second habit is worth sitting with, because it inverts the intuition. Information does
not add to the model. It cuts it down.

## Where this is going

So far every arrow has meant "cannot tell these apart" — a symmetric, all-or-nothing sort
of uncertainty. That gives you knowledge, and only knowledge.

Belief needs more, because a believer is not merely uncertain: they *lean*. Alice may
consider tails possible while finding heads far more plausible, and if you tell her the
coin was tails she should be able to change her mind without ever having been logically
wrong. Plain arrows cannot express leaning.

The [next chapter](./knowledge-and-belief.md) adds the missing structure — an ordering on
the arrows — and shows why knowledge and belief end up obeying genuinely different laws.
