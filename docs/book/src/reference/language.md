# The language

A `.delhi` file has a signature, an initial state, optional constraints, and actions.

```
types   { Actor - Object }          // a type hierarchy
objects { alice, bob - Actor }      // things, with types
agents  { alice, bob }              // which of them have minds
props   { h, d }                    // propositions

constants { adjacent(hall, study) } // optional, static
define    { … }                     // optional, named formulas
rules     { … }                     // optional, Horn clauses
initially { … }  or  state { … }    // required, one of the two
goal      { φ }                     // optional
invariants{ φ … }                   // optional
actions   { … }                     // required, may be empty
```

## Initial state

Two forms. **`initially`** is declarative — state facts and attitudes, and the model is
constructed and then verified against every line you wrote:

```
initially {
    h                    // a fact about the actual world
    ?[carol] h           // carol cannot tell
    B[carol] h           // but she leans that way
}
```

**`state`** writes the model out by hand, and is exactly what `delhi show` prints:

```
state {
  *w1 <- { h }          // `*` marks the actual world
   w0 <- { }
  carol: w0 < w1        // w1 is the more plausible
}
```

`<` and `<=` point toward the *more plausible* world; `~` relates two worlds both ways.

## Definitions

Named formulas, expanded before anything is lowered — the semantics never learns they
existed. Parameters substitute objects, and may stand where an agent name does:

```
define {
    blocked(?r)       = !lit(?r) | locked(?r)
    can_enter(?w, ?r) = !blocked(?r) & K[?w] !blocked(?r)
}
```

Usable anywhere a formula is. Definitions may call definitions; a cycle is rejected when
the table is built rather than caught by a depth limit.

Two things are refused deliberately. A definition cannot be `causes`d or written as a world
fact, since both need an atom the semantics can *set*. And parameters range over objects,
not formulas — `define f(?p) = K[a] ?p` is a second-order macro and is not supported.

## Rules

Horn clauses over constants, saturated to a least fixpoint at parse time:

```
constants { !adjacent(Room, Room)  adjacent(hall, study)  adjacent(study, attic) }
rules {
    reach(?x, ?y) :- adjacent(?x, ?y)
    reach(?x, ?z) :- adjacent(?x, ?y), reach(?y, ?z)
}
```

`reach(hall, attic)` folds to `true` like any other constant. Derived predicates never
become propositions, so they cost no bit in any world.

**Constants only, and that restriction is the interesting part.** The fixpoint runs once,
which is sound only because the constant table is static. A rule over a *fluent* would have
an extension varying per world and per action, so computing it would mean either a fixpoint
per world at evaluation or maintaining derived atoms through product update — the frame
problem again. It is refused with a message rather than half-supported.

Bodies carry no negation, which keeps the program monotone so the least fixpoint exists;
and every head variable must appear in the body, or the head would assert facts the body
never justified.

## Invariants

Claims that must hold in every state, not only the first:

```
invariants { !((B[a] p & B[b] !p) | (B[a] !p & B[b] p)) }
```

An `initially` entry that drives no construction is already an assertion about the start;
an invariant is the same claim made about the whole run, which is usually what a domain
constraint means.

## Comments

`//` to end of line, `/* … */` for blocks.
