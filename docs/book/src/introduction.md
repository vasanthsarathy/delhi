# Introduction

Most software that reasons about the world tracks *facts*: the coin is heads, the marble is
in the basket, the robot is in the hall. delhi tracks something harder — what each agent
**thinks** about those facts, what they think everyone else thinks, and how all of that
changes when things happen that some agents witness and others miss.

That extra layer is where the interesting failures live:

```
$ delhi eval examples/sally_anne.delhi \
      -a "sally_leaves()" "anne_moves()" "sally_returns()" \
      -f "B[sally] basket & !basket"
true
```

Sally believes the marble is in the basket. It is not in the basket. Both at once — and
that is not a bug in the model, it is the point. Sally left the room before Anne moved the
marble, so her belief is *stale* rather than wrong-headed, and a system that could not
represent the gap could not represent her at all.

## Who this is for

Two audiences, and you can skip half of this book depending on which you are.

**If you have not met epistemic logic before**, start with
[Possible worlds](./logic/possible-worlds.md). Four short chapters cover what it means to
say an agent "knows" or "believes" something formally, why those two words need separate
machinery, and what happens to both when the world changes. No prior logic is assumed
beyond `and`, `or`, `not`.

**If you know the field already**, skip to [Install](./guide/install.md) and
[Your first domain](./guide/first-domain.md). delhi implements **mB+**, a plausibility-model
semantics from Buckingham's thesis extended with safe and conditional belief; the
[Related systems](./background/related-work.md) chapter places it against DEL, mA\*, EFP and
PDKB, and says plainly what is transcription and what is new.

## What delhi is

A model checker, a small declarative language, and a browser UI for exploring both.

- **A language.** A `.delhi` file declares agents, propositions, what is true and believed
  at the start, and a set of actions. It reads like a description of a scenario rather than
  a set of equations.
- **A model checker.** Given that file, delhi builds a plausibility model and answers
  questions about it — including questions three or four levels deep, like "does Alice
  believe that Bob knows whether Carol is lying".
- **A tool.** A single self-contained binary. `delhi eval` for one question, `delhi ask` to
  enumerate every formula of a given shape that holds, `delhi repl` to poke at a scenario,
  `delhi gui` for a browser view of the model as it changes.

## What delhi is not

A planner. delhi tells you what holds in a state and how a state changes when you apply an
action you name. It does not search for a sequence of actions that would achieve a goal.
That is the obvious next thing to build on top, and the pieces for it exist, but it is not
here yet.

It is also not a theorem prover. Questions are answered against a *specific finite model*,
not proved valid across all models.

## A word on the name

delhi is a Rust reimplementation of `mecaPlanner`, a Java epistemic planner. The semantics,
the action types, and several examples come from that lineage; the language, the query
system, the tooling and the performance work do not. Where this book states something as
delhi's own rather than inherited, it says so.
