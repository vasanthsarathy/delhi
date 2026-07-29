# Your first domain

We will build a scenario from nothing, one piece at a time, and end up with a
**second-order false belief** — an agent that is wrong not about the world but about
someone else's mind.

The story: Ana is planning a surprise party for Cleo. Ben knows. Cleo suspects something
and might go and check.

Create `party.delhi` and follow along.

## 1. Who and what

Every file starts by declaring its vocabulary.

```
types   { Actor - Object }
objects { ana, ben, cleo - Actor }
agents  { ana, ben, cleo }
props   { party }
```

- **`types`** — a small type hierarchy. `Actor - Object` reads "Actor is a kind of Object".
- **`objects`** — the things that exist, each with a type.
- **`agents`** — which of them have minds. Only these can appear inside `K[…]` or `B[…]`.
- **`props`** — the propositions. Here just one: whether a party is being planned.

## 2. What is true, and who knows it

```
initially {
    party
    ?[cleo] party
}

actions {}
```

`initially` is **declarative**. You state facts and attitudes, and delhi constructs a model
satisfying them — then checks the model against every line you wrote.

- `party` — a bare proposition is a fact about the actual world.
- `?[cleo] party` — Cleo cannot tell whether there is a party. `?` is the ignorance
  operator.

Anything not mentioned is known by everyone, so Ana and Ben know about the party without
being named.

> `actions {}` is required even when empty. Leave it out and you get
> `missing required section 'actions'`.

Check it:

```bash
$ delhi check party.delhi
ok: 1 atoms, 3 agents, 0 ground actions, 2 worlds
```

**Two worlds** — one where the party is on, one where it is not. Cleo's ignorance is what
created the second. `delhi state` shows the consequences:

```bash
$ delhi state party.delhi
actual world party

  ana   knows party
  ben   knows party
  cleo  undecided party
```

And `delhi show` prints the model itself:

```
state {
   w0 <- {  }
  *w1 <- { party }

  cleo: w0 ~ w1
}
```

`*` marks the actual world. Cleo relates the two, so she cannot tell them apart. Ana and
Ben need no line at all — with nothing declared they distinguish everything, which is
exactly what knowing means.

## 3. Things that happen

Now the actions. Add a second proposition and three of them:

```
props   { party, suspicious }

goal { K[cleo] party }

actions {
    ana_denies() {
        actor     ana
        announces !party
        ana observes, ben observes, cleo observes
    }

    ben_hints() {
        actor  ben
        causes suspicious
        ana observes, ben observes, cleo observes
    }

    cleo_checks() {
        actor      cleo
        determines party
        cleo observes
        ben  aware
        ana  aware if !suspicious
    }
}
```

Three actions, three different kinds of change:

- **`announces !party`** — Ana says there is no party. She is lying, and nothing requires
  otherwise. Hearers come to *believe*.
- **`causes suspicious`** — Ben changes the world.
- **`determines party`** — Cleo goes and looks. An observer comes to *know*.

And three observer positions in `cleo_checks()`:

- `cleo observes` — she sees the outcome.
- `ben aware` — he notices her checking but not what she found.
- `ana aware if !suspicious` — she only notices **if she is not preoccupied**. Ben's hint
  sets `suspicious`, so after `ben_hints()` this clause drops and Ana is oblivious.

That last line is the hinge of the whole scenario.

## 4. Watch it happen

Apply the lie alone:

```bash
$ delhi state party.delhi -a "ana_denies()"
actual world party, !suspicious

  ana   knows party, !suspicious
  ben   knows party, !suspicious
  cleo  knows !suspicious   believes !party
```

The lie landed. Cleo **believes** there is no party — and note she does not *know* it,
because knowledge is factive and there *is* a party. Being lied to moves belief without
touching knowledge.

Now the whole sequence:

```bash
$ delhi state party.delhi -a "ana_denies()" "ben_hints()" "cleo_checks()"
actual world party, suspicious

  ana   knows party, suspicious
  ben   knows party, suspicious
  cleo  knows party, suspicious
```

Cleo checked and now *knows*. First-order, everyone agrees — the state view is first-order
by construction, so it looks like nothing interesting happened.

## 5. The interesting part

The disagreement is one level up, where the state view cannot show it:

```bash
$ delhi eval party.delhi -a "ana_denies()" "ben_hints()" "cleo_checks()" \
      -f "K[ben] Kw[cleo] party"
true
$ delhi eval party.delhi -a "ana_denies()" "ben_hints()" "cleo_checks()" \
      -f "K[ana] Kw[cleo] party"
false
$ delhi eval party.delhi -a "ana_denies()" "ben_hints()" "cleo_checks()" \
      -f "B[ana] B[cleo] !party"
true
```

There it is. **Ben** was `aware`, so he knows Cleo settled the question — without knowing
what she found. **Ana** was oblivious, because Ben's hint had made her `suspicious` and her
`aware if !suspicious` clause dropped. So Ana's picture of Cleo is two events out of date:
she still believes Cleo believes the lie.

Ana is not wrong about the party. She is wrong about Cleo.

## 6. Let delhi find it for you

You had to guess that formula. `ask` searches instead — `_` is a hole to fill:

```bash
$ delhi ask party.delhi -a "ana_denies()" "ben_hints()" "cleo_checks()" \
      -q "B[ana] B[cleo] _"
  B[ana] B[cleo] (!party)
  B[ana] B[cleo] (suspicious)
2 of 4 candidates at depth 0
```

Better, ask what Ana believes **that is not so**, with the hole appearing twice:

```bash
$ delhi ask party.delhi -a "ana_denies()" "ben_hints()" "cleo_checks()" \
      -d 1 -q "B[ana] _ & !_"
  B[ana] (B[cleo] !party) & !(B[cleo] !party)
1 of 28 candidates at depth 1
```

One false belief, found rather than guessed: Ana believes Cleo believes there is no party,
and Cleo believes no such thing. `-d 1` allows one level of modal nesting in the candidates,
which is what lets the hole be filled by `B[cleo] !party` rather than a bare proposition.

## What you built

- Two worlds from a single `?`
- A lie that moved belief without touching knowledge
- A conditional observer clause that turned one agent oblivious mid-trace
- A second-order false belief, and a query that discovers it

Next: [Actions and who sees them](./actions.md) for the full observability rules, or
[Asking questions](./queries.md) for what `ask` can do.
