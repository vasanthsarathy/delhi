# How attitudes change

A static model says what agents think right now. The field is called *dynamic* epistemic
logic because the real subject is what happens next — and specifically, how one event can
leave different agents in different epistemic positions.

## The core idea

An event is modelled the same way a state is: as a little model of its own. It has
**events** in place of worlds, a **precondition** on each saying when it can occur, and one
relation per agent saying which events that agent cannot tell apart.

Then the new state is the **product** of the two. Every (world, event) pair whose
precondition holds becomes a new world, and an agent connects two new worlds when it
connected both the worlds *and* the events they came from.

That product is the entire mechanism. Announcements, sensing, lying, and acting unobserved
are all the same operation with different little models.

## Three kinds of thing that can happen

delhi gives you three, because they change different things:

| Clause | What changes |
|---|---|
| `causes p, !q` | **the world.** The marble moves. Add `if φ` for a conditional effect. |
| `determines p` | **knowledge.** The observer looks and comes to *know*. |
| `announces φ` | **belief.** The hearer comes to *believe* — and it may be a lie. |

The distinction between the last two is the factivity line from
[Knowledge and belief](./knowledge-and-belief.md). Looking in the box tells you how things
are; being told is only as good as the teller.

```
peek_c() {
    actor      carol
    determines h          // she looks: she will KNOW
    carol observes
}

announce_not_heads() {
    actor     alice
    announces !h          // she says it: hearers will BELIEVE
    alice observes, bob observes, carol observes
}
```

`announces !h` does not require `!h` to be true. That is what makes it a lie rather than a
fact, and it is why the hearer ends up *believing* something false while still not knowing
it.

## Who saw it

This is the part that does the work, and it is where most of the interesting modelling
lives. Three positions an agent can be in:

| Clause | The agent… |
|---|---|
| `a observes` | sees exactly what happened, outcome included |
| `a aware` | knows the action occurred, but **not how it turned out** |
| *(neither)* | is oblivious — does not even learn that anything happened |

The middle one is easy to overlook and does a great deal. If Bob peeks into a box and Alice
is `aware`, Alice does not learn the coin — but she learns that *Bob* has. She comes to
know that he knows whether:

```bash
$ delhi eval examples/coin_lie.delhi -a "distract_a()" "peek_c()" -f "K[bob] Kw[carol] h"
true      # bob heard the peek: he knows carol settled it
$ delhi eval examples/coin_lie.delhi -a "distract_a()" "peek_c()" -f "K[alice] Kw[carol] h"
false     # alice was distracted: she does not even know it happened
```

**Mechanically**, a sensing or announcing action builds three events — `ψ`, `¬ψ`, and a `⊤`
event standing for *nothing observable happened*. Each agent gets two edge labels:

- `ψ ↔ ¬ψ` is labelled `¬observes(i)`
- the edges to the `⊤` event are labelled `¬(observes(i) ∨ aware(i))`

An `aware` agent keeps the first — the outcomes stay indistinguishable — and loses the
second. That is precisely "I know it happened, I don't know how it went". An oblivious
agent keeps both and cannot rule out that the world simply carried on.

Conditions make this dynamic. `alice aware if !d` means her class depends on the state at
the time, which is how one `distract_a()` earlier in the trace turns her from aware into
oblivious.

## Why belief revision needs the ordering

When an agent learns something that contradicts what it believed, it must not simply be
left with nothing. The plausibility ordering from
[Knowledge and belief](./knowledge-and-belief.md) is what makes this work: an announcement
does not *delete* the worlds where the announcement is false, it **reorders** them,
promoting the ones consistent with what was said.

The agent's knowledge does not change — every world it considered possible is still
possible. Only the leaning moves. And because the disfavoured worlds are still there, a
later truthful announcement can promote them back:

```bash
$ delhi eval examples/coin_lie.delhi -a "announce_not_heads()" -f "B[carol] !h"
true      # the lie landed
$ delhi eval examples/coin_lie.delhi -a "announce_not_heads()" "peek_c()" -f "K[carol] h"
true      # she looks, and recovers the truth
```

A system that deleted worlds on announcement would have made the first step irreversible.
Carol would have been stuck believing the lie with no way back, which is not what happens
when someone lies to you and you then check.

## Models grow — and what to do about it

Product update multiplies. Each action crosses every world with every distinguishable
event, so an uncontracted model grows exponentially:

```
cycle      worlds      cumul
    1          16     72.9us
    2         128      2.2ms
    3        1024    133.6ms
    4        8192      9.47s
```

The fix is **bisimulation contraction**: worlds that no agent can distinguish, and that no
formula could tell apart, are merged. delhi contracts after every action, which in most
domains holds the model at a fixed point — Coin Lie settles at 16 worlds whether you run 2
cycles or 8, taking about 0.6 ms each.

It is not a fixed point in general. Grapevine's cycle creates a genuinely new distinction
every time round, because each repetition adds another layer of who-was-present-for-what,
and contraction cannot merge what is really different. See the benchmark section of the
[README](https://github.com/vasanthsarathy/delhi#how-fast-and-does-it-blow-up) for numbers.

## Where to go next

You now have the whole conceptual picture: worlds, an ordering, two attitudes plus two
refinements, nesting, and product update against three observer classes.

[Your first domain](../guide/first-domain.md) builds one from scratch.
