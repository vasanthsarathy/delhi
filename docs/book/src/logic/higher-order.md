# Higher-order attitudes

Everything so far has been *first-order*: what an agent thinks about the world. The
interesting cases are one level up — what an agent thinks about what another agent thinks.

## Nesting

The operators nest, and that is all there is to the syntax:

```
K[alice] h                          alice knows the coin is heads
B[alice] K[bob] h                   alice believes bob knows it
B[alice] B[bob] B[carol] !h         alice thinks bob thinks carol thinks it is tails
```

Semantically nothing new happens. `B[alice] B[bob] h` means: in every world Alice finds
most plausible, `B[bob] h` holds — which in turn means that in every world *Bob* finds most
plausible from there, `h` holds. Follow the arrows, then follow more arrows.

What *is* new is that these can come apart from the first-order facts in ways that matter.

## The false-belief task

Developmental psychology has a canonical test for whether a child can represent someone
else's mind separately from reality. It is called Sally-Anne, and it runs like this:

> Sally puts her marble in the basket and leaves the room. While she is gone, Anne moves
> the marble to the box. Sally comes back.
>
> **Where will Sally look for her marble?**

The answer is *the basket*. Children under about four say "the box" — they know where the
marble is, and they have no machinery for a belief that is false. Getting it right requires
holding two incompatible pictures at once: where the marble is, and where Sally thinks it
is.

That is exactly the structure of `examples/sally_anne.delhi`:

```bash
$ delhi eval examples/sally_anne.delhi \
      -a "sally_leaves()" "anne_moves()" "sally_returns()" \
      -f "B[sally] basket & !basket"
true
```

The conjunction is the whole task. `!basket` is the world; `B[sally] basket` is Sally.
Neither is negotiable and they disagree.

The single clause that makes it work is in the action that moves the marble:

```
sally observes if present
```

Sally sees Anne's move *only if* she is in the room — and `sally_leaves()` has already made
`present` false. Take the condition away and the whole phenomenon vanishes: she witnesses
the move and updates like anyone else.

## Second-order false belief

Now go one level further. It is possible to be wrong not about the world, but about
*someone else's mind* — and to be right about the world at the same time.

`examples/coin_lie.delhi` builds exactly that. Alice lies that the coin is tails, Bob
distracts her, Carol peeks and learns the truth. Alice, being distracted, never sees the
peek happen, so her picture of Carol goes stale:

```bash
$ delhi eval examples/coin_lie.delhi \
      -a "announce_not_heads()" "distract_a()" "peek_c()" \
      -f "B[alice] B[carol] !h & K[carol] h"
true
```

Alice believes Carol believes tails. Carol *knows* heads. Alice is not mistaken about the
coin — she is mistaken about Carol.

This is where the possible-worlds machinery earns its keep. There is no flag you could set
that would represent "Alice's model of Carol is two events out of date"; it falls out of
Alice's arrows pointing at worlds where the peek never happened.

## Common knowledge

One more operator, and it is not just "everybody knows".

`C[*] φ` — **common knowledge** — means everyone knows φ, and everyone knows that everyone
knows it, and so on without end. It is the standard precondition for coordination: you and
I can meet at noon without further discussion only if the arrangement is common knowledge,
not merely known to us both.

The infinite regress is not a problem to compute. `C` is evaluated over the *transitive
closure* of every agent's relation at once: φ is common knowledge exactly when it holds in
every world reachable by any chain of any agents' arrows.

`examples/muddy_children.delhi` is the classic demonstration. Three children can each see
the others' foreheads but not their own. The father announces "at least one of you is
muddy" — telling nobody anything they did not already see. But it makes the fact *common*,
and that alone lets them deduce their own state after two rounds of nobody speaking up.

```bash
$ delhi state examples/muddy_children.delhi \
      -a "father_speaks()" "nobody_knows()" "nobody_knows()"
```

`undecided` becomes `believes`. Nothing was said in those last two rounds — the *silence*
was the information.

## What to take away

Higher-order attitudes are not a decoration on top of the first-order ones. They are where
lying, deception, coordination, teaching and pretence all live, and every one of them
requires an agent's model of another agent to be able to go stale or be wrong.

The [next chapter](./dynamics.md) covers how these attitudes change when something happens
— which, given that staleness is the whole story here, is where the observability rules
turn out to matter more than anything else.
