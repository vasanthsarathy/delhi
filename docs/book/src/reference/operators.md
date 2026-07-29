# Operators

Six primitives and nine sugar forms. Every one works in `eval`, in `ask` patterns, in
`goal`, in `invariants`, and at the prompt.

## Primitives

| | Written | Holds when |
|---|---|---|
| Knowledge | `K[a] φ` | φ holds in every world `a` considers possible |
| Belief | `B[a] φ` | φ holds in every world `a` finds **most** plausible |
| Safe belief | `[][a] φ` or `□[a] φ` | φ holds in every world at least as plausible as this one |
| Conditional belief | `B^ψ[a] φ` | `a` **would** believe φ on learning ψ |
| Common knowledge | `C[*] φ` | φ survives any chain of any agents' arrows |
| Atoms | `p`, `q(x)` | as valued in the world |

Closed under `!`, `&`, `|`, and nested arbitrarily. `C[*]` takes all agents; `C[a,b]` takes
a group.

## Sugar

| | Written | Expands to |
|---|---|---|
| Knows whether | `Kw[a] φ` | `K[a] φ \| K[a] !φ` |
| Ignorance | `?[a] φ` | `!Kw[a] φ` |
| Believes whether | `Bw[a] φ` | `B[a] φ \| B[a] !φ` |
| Belief-ignorance | `??[a] φ` | `!Bw[a] φ` |

The disjunction is why these exist as operators. "Does Alice know whether the coin is
heads?" is not `K[alice] h` and not `!K[alice] h` — it is the *or* of two knowledge claims,
and writing it out every time is how mistakes get made.

## The laws each obeys

delhi verifies these as frame properties rather than assuming them.

**`K` is S5** — `K[a] φ → φ` (T, factivity), `K[a] φ → K[a] K[a] φ` (4),
`!K[a] φ → K[a] !K[a] φ` (5).

**`B` is KD45** — `B[a] φ → !B[a] !φ` (D, consistency), plus 4 and 5. Notably **not** T: an
agent can believe something false. That is the entire point.

**`[]` is factive** — `[][a] φ → φ`.

**Bridges** — `K[a] φ → B[a] φ`, `B[a] φ → K[a] B[a] φ`, `K[a] φ → [][a] φ`.

## Strength ordering

```
K[a] φ    ⟹    [][a] φ    ⟹    B[a] φ
```

Knowledge is strongest, then safe belief, then belief. Each step reads over fewer worlds,
so each is easier to satisfy. Safe belief is the useful middle: it survives learning any
true fact, where a mere belief can be overturned by one.

```bash
$ delhi eval examples/coin_lie.delhi -f "[][carol] h"    # true
$ delhi eval examples/coin_lie.delhi -f "K[carol] h"     # false
```

Carol's belief that the coin is heads is *safe* — stable under any true news — without
being knowledge, because she has not actually established it.

## Plausibility direction

`u R[i] v` means **v is at least as plausible as u**. Plausibility increases along the
arrow, and in the surface syntax it increases to the right:

```
carol: w0 < w1        // w1 is the more plausible
```

Worth pinning down, because an inverted ordering is still a well-formed model. Nothing will
complain, and every answer will be quietly backwards.
