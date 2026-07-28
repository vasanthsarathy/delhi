# delhi

An epistemic model checker and reasoning system. It represents what agents know and
believe, how those attitudes change when things happen, and — the part that makes it
interesting — how an agent can end up confidently wrong about what another agent believes.

Written in Rust, no runtime dependencies.

```
$ delhi eval examples/coin_lie.delhi -f "B[carol] h"
true
```

## Why

Most planning systems track facts. delhi tracks *attitudes toward* facts, per agent, and
keeps them straight through announcements, sensing, and actions that some agents witness
and others miss. It is built on **mB+**: plausibility models where each agent orders
possible worlds by how believable they are, which is what lets an agent be wrong now and
recover later when better evidence arrives.

That ordering is the whole trick. Knowledge is what holds across everything an agent
considers possible; belief is what holds across the ones it finds *most* plausible. When a
lie lands, the agent's ordering shifts without its knowledge changing — and when the truth
arrives, the ordering shifts back.

## Quick start

Put `delhi` on your `PATH` once and stop typing `cargo run`:

```bash
cargo install --path crates/delhi-cli
delhi check examples/coin_lie.delhi
```

If you would rather not install, `cargo build --release` leaves the binary at
`target/release/delhi` — that is the same thing without the copy, and it is what you want
for timing anything, since `cargo run` defaults to an unoptimised build.

```bash
cargo build --release
./target/release/delhi repl examples/coin_lie.delhi
```

Output is coloured when stdout is a terminal, and plain otherwise — so
`delhi dot … | dot -Tpng` stays byte-clean. `NO_COLOR=1` turns it off, `CLICOLOR_FORCE=1`
turns it on through a pipe.

## The language

A `.delhi` file declares a signature, an initial state, an optional goal, and actions.
Here is the whole of `examples/coin_lie.delhi`, which reproduces Figures 5.4–5.10 of
Buckingham's thesis:

```
types   { Actor - Object }
objects { alice, bob, carol - Actor }
agents  { alice, bob, carol }
props   { h, d }                    // heads; distracted

initially {
    h                    // the coin really is heads
    ?[carol] h           // carol cannot tell
    B[carol] h           // but she correctly leans that way
}

goal { B[alice] B[carol] !h & K[carol] h }

actions {
    announce_not_heads() {
        actor     alice
        announces !h                  // a lie; truth is not required
        alice observes, bob observes, carol observes
    }

    distract_a() {
        actor  bob
        causes d
        alice observes, bob observes, carol observes
    }

    peek_c() {
        actor      carol
        determines h
        carol observes                // sees the coin: comes to KNOW
        bob   aware                   // hears it: knows carol learned something
        alice aware if !d             // only notices if she is not distracted
    }
}
```

Alice lies that the coin is tails. Bob distracts her. Carol peeks and learns the truth —
but Alice, being distracted, never sees that happen, so her picture of Carol goes stale.
At the end Alice believes Carol believes ¬h, while Carol *knows* h. That is the goal
formula, and it is a second-order false belief: Alice is wrong not about the world but
about someone else's mind.

### Three kinds of action

| Clause | Meaning |
|---|---|
| `causes p, !q` | changes the world. Add `if φ` for a conditional effect. |
| `determines p` | sensing. The observer comes to **know**. Propositional only. |
| `announces φ` | speech. The hearer comes to **believe** — and it may be a lie. |

### Three observer classes

| Clause | The agent… |
|---|---|
| `a observes` | sees exactly what happened |
| `a aware` | knows *something* happened, not what |
| *(neither)* | is oblivious; nothing changes for it |

Both take a condition: `alice aware if !d` makes the class depend on the state.

### Initial state

`initially { … }` is declarative — state the facts and attitudes, and the model is
constructed and then *verified* against every declaration you wrote. For cases where you
want the model itself, `state { … }` writes worlds and edges out by hand:

```
state {
  *w1 <- { h }          // `*` marks the designated (actual) world
   w0 <- { }
  carol: w0 < w1        // w1 is the more plausible of the two
}
```

`<` and `<=` point toward the *more plausible* world; `~` relates two worlds both ways.
This is also exactly what `delhi show` prints, so you can inspect a declaratively-built
state and paste the result back.

### Constraints, definitions and rules

Three optional sections, each answering a different need.

**`invariants`** — claims that must hold in every state, not only the first. An `initially`
entry that drives no construction is already an assertion about the start; an invariant is
the same claim made about the whole run, which is usually what a domain constraint means.

```
invariants { !((B[a] p & B[b] !p) | (B[a] !p & B[b] p)) }
```
```
$ delhi step domain.delhi -a "lie()"
applied lie()
invariant violated after lie():
  !((B[a] p & B[b] !p) | (B[a] !p & B[b] p))
```

Checked when the initial state is built and after every action, in `step`, the REPL and the
browser. `check` refuses a domain inconsistent with its own constraint; `step` exits 1; the
REPL reports and keeps going, since exploring past a break is usually what you want.

**`define`** — a named formula, expanded before anything is lowered, so the semantics never
learns it existed. Parameters substitute objects and may stand where an agent name does.

```
define {
    blocked(?r)       = !lit(?r) | locked(?r)
    can_enter(?w, ?r) = !blocked(?r) & K[?w] !blocked(?r)
}
```

Usable anywhere a formula is — preconditions, goals, invariants, and at the prompt.
Definitions may call definitions; a cycle is rejected when the table is built rather than
caught by a depth limit. Two things are refused deliberately: a definition cannot be
`causes`d or written as a world fact, since both need an atom the semantics can *set*; and
parameters range over objects, not formulas, so `define f(?p) = K[a] ?p` is a second-order
macro and not supported.

**`rules`** — Horn clauses over constants, saturated to a least fixpoint at parse time.

```
constants { !adjacent(Room, Room)  adjacent(hall, study)  adjacent(study, attic) }
rules {
    reach(?x, ?y) :- adjacent(?x, ?y)
    reach(?x, ?z) :- adjacent(?x, ?y), reach(?y, ?z)
}
```

`reach(hall, attic)` folds to `true` like any other constant. Derived predicates never
become propositions, so they cost no bit in any world — in `examples/reachability.delhi`
the signature stays at four atoms, and ten of the twelve `walk` groundings are pruned
before they are built because their `adjacent` guard folded to false.

**Constants only, and that is the interesting restriction.** The fixpoint runs once, which
is sound only because the constant table is static. A rule over a *fluent* would have an
extension that varies per world and per action, so computing it would mean either a
fixpoint per world at evaluation or maintaining derived atoms through product update — the
frame problem again. It is refused with a message rather than half-supported. Bodies carry
no negation, which keeps the program monotone so the least fixpoint exists; and every head
variable must appear in the body, or the head would assert facts the body never justified.

### Axioms are a different matter

Logical axioms are **not** assertable, deliberately. S5 for knowledge, KD45 for belief and
the bridges between them are properties of the *frame*, enforced by `Model::validate` and
relied on by the semantics and the soundness proof. Asserting one is a no-op because it
already holds; weakening one means a different frame class, which is a change to the
semantics rather than to a domain, and would invalidate results a file has no business
invalidating. Anything strictly *stronger* than the frame gives is a domain constraint —
write it as an invariant.

## Examples

Ten domains in `examples/`, each runnable and each pinned by a test in
`crates/delhi-lang/tests/` so the file, the test, and this README cannot drift apart.

| File | What it is for |
|---|---|
| `coin_lie.delhi` | Second-order false belief, from Buckingham's thesis (Figs 5.4–5.10). The reference trace. |
| `sally_anne.delhi` | The canonical false-belief task — Wimmer & Perner (1983) |
| `sally_anne_second_order.delhi` | Its second-order variant — Bräuner et al. (2016); Example 1 of KR 2024 |
| `ice_cream_van.delhi` | Second-order false belief by missing an event — Perner & Wimmer (1985) |
| `bicycle.delhi` | The Birthday Bicycle — Sullivan, Zaitchik & Tager-Flusberg (1994); the story KR 2021 opens with |
| `coin_in_the_box.delhi` | The standard epistemic-planning benchmark |
| `muddy_children.delhi` | The canonical multi-agent puzzle |
| `selective_communication.delhi` | SC_3_4 from the EFP suite — third-order goals, position-dependent audiences |
| `grapevine.delhi` | Grapevine from the EFP suite — gossip, with a *negative* goal conjunct |
| `reachability.delhi` | Rules, definitions and invariants together — a derived transitive closure |

The last two are ports of published benchmarks (from the mecaPlanner corpus in `refs/`), so
delhi's numbers on them can be set against EFP's and PDKB's. Grapevine is also the compactness
case: the original enumerates 24 actions by hand because that encoding has no parameters, and
the port is two declarations that ground to the same 24 — a test asserts exactly that, including
that the six `move(x, r, r)` groundings are pruned rather than built and rejected.

**Sally-Anne** is the whole reason a system like this exists. Sally puts her marble in the
basket and leaves; Anne moves it to the box; Sally returns. Asked where Sally will look,
children under about four say "the box" — they answer where the marble *is*, having no
machinery for a belief that is false. The entire task turns on one clause:

```
anne_moves() {
    causes box, !basket
    anne  observes
    sally observes if present    // she is not, so she misses it entirely
}
```

and it ends with `B[sally] basket` true while `box` is true. It also ends with
`B[anne] B[sally] basket` — Anne passing the task herself.

**The ice-cream van** goes one level up. John watches the van leave the park, so he is not
wrong about the van. Then the driver tells Mary, and John does not see that happen. He is
wrong about *Mary's mind*, which is a different and later-developing competence:
`B[john] B[mary] at_park` alongside `K[mary] !at_park`.

**The second-order Sally-Anne** changes one line and asks something harder. Sally does not
leave — she stays and watches secretly, and Anne does not realise she has been seen. Nobody
misses an event here; Anne's mistake is about *observability itself*, about whether Sally was
in a position to see. That is what the conditions on observer clauses are for:

```
sally observes if watching     // true in fact, false in anne's picture
```

Anne's most plausible worlds have `watching` false, so in those worlds she computes Sally as
oblivious and her model of Sally never updates: `K[sally] box` and `B[anne] B[sally] basket`
both hold.

**The Birthday Bicycle** is the story KR 2021 opens with, and it does two jobs at once. It is
Timmy's birthday; his mother has hidden a bicycle in the basement and tells him she is not
giving him one, to keep the surprise. He believes her — then goes down and sees it.

```
> B[timmy] !bicycle           true      # he has no reason to expect one
> :do mom_tells_him_no()
> B[timmy] !bicycle           true      # the lie holds
> K[timmy] !bicycle           false     #   ...as belief, not knowledge
> [][timmy] !bicycle          false     #   ...and not safely: evidence can dislodge it
> :do timmy_looks_in_the_basement()
> K[timmy] bicycle            true      # he works it out
> B[mom] B[timmy] !bicycle    true      # and she never learns that he did
```

The first half is the argument for plausibility orderings: under a flat belief set, believing
`!bicycle` and then learning `bicycle` is a contradiction with nowhere to put the correction.
Here it is a reordering, twice. The second half is the second-order false belief — his
mother knows exactly where the bicycle is, and is wrong only about him.

**Coin in the Box** is the benchmark, and it exists to separate three epistemic positions:
seeing, hearing, and missing entirely. Alice peeks while Bob is in earshot but not looking,
and the result is `K[alice] tail`, `!Kw[bob] tail`, and `K[bob] Kw[alice] tail` — Bob does
not learn the coin, but does learn that Alice learned it. That middle position is exactly
what `aware` is for.

**Muddy Children** ends one notch weaker than the textbook, and the file explains why at
length rather than hiding it. The timing is exactly classical — with three muddy children,
ignorance is announced twice and on the third round all three conclude together, and
deleting either round breaks it. But it ends in belief, not knowledge. An announcement in
this language reorders which worlds an agent finds plausible rather than deleting any,
because in this language announcements can be lies. No reordering can produce knowledge
while the ¬φ worlds are all still there to be considered. That is the price of being able to
model the Coin Lie at all.

## Querying

### Three ways to ask

| | Answers | Use when |
|---|---|---|
| `delhi eval -f φ` | is this one formula true? | you know what to check |
| `delhi ask -q π` | which formulas of this shape are true? | you don't yet know what to look for |
| `delhi state` | every agent's stance on every proposition | you want the lay of the land |

All three take `-a ACTION…` to run a trace first, and all three exist at the REPL prompt and
in the browser console as a bare formula, `:ask`, and `:state`.

### Recipes

The pattern language is where the leverage is, so start from the question rather than the
syntax. `_` is a hole; every hole in one pattern takes the same filler, which is what makes
the last four work.

| The question | Write |
|---|---|
| Does alice know the coin is heads? | `eval -f "K[alice] h"` |
| Is carol wrong about it? | `eval -f "B[carol] !h & h"` |
| Is it common knowledge? | `eval -f "C[*] h"` |
| What does alice believe? | `ask -q "B[alice] _"` |
| What can't she settle? | `ask -q "?[alice] _"` |
| What does she think carol believes? | `ask -q "B[alice] B[carol] _"` |
| What's commonly known? | `ask -q "C[*] _"` |
| **What does alice believe that is false?** | `ask -q "B[alice] _ & !_"` |
| Where do alice and carol disagree? | `ask -q "B[alice] _ & B[carol] !_"` |
| What does alice know that carol doesn't? | `ask -q "K[alice] _ & !K[carol] _"` |
| What does carol believe without knowing? | `ask -q "B[carol] _ & !K[carol] _"` |

The false-belief recipe is the one worth trying first, because it finds things you would not
have thought to check. Run the Coin Lie to its end and ask what alice believes that is not so:

```bash
$ delhi ask examples/coin_lie.delhi -d 1 -q "B[alice] _ & !_" \
      -a "announce_not_heads()" "distract_a()" "peek_c()"
  B[alice] (B[carol] !h) & !(B[carol] !h)
1 of 28 candidates at depth 1
```

That is the scenario's entire point — alice's second-order false belief — located without
naming it. And on grapevine, after b tells a a secret with c out of the room:

```bash
$ delhi ask examples/grapevine.delhi -q "K[a] _ & !K[c] _" \
      -a "move(c,r1,r2)" "share(b,b,r1)"
  K[a] (secret(a)) & !K[c] (secret(a))
  K[a] (secret(b)) & !K[c] (secret(b))
```

Her own secret, and the one she just heard.

### The operators

Every one is available in `eval`, in `ask` patterns, in `goal`, and at the prompt.

| Write | Reads as | True when |
|---|---|---|
| `K[a] φ` | a knows φ | φ holds everywhere a considers possible |
| `B[a] φ` | a believes φ | φ holds where a finds it most plausible |
| `[][a] φ` | a *safely* believes φ | belief no true evidence can dislodge |
| `B^ψ[a] φ` | a would believe φ given ψ | conditional belief |
| `C[a,b] φ` | a and b commonly know φ | and each knows the other knows, without end |
| `C[*] φ` | everyone commonly knows φ | `[*]` is shorthand, and only `C` takes it |
| `Kw[a] φ` | a knows whether φ | a knows which way it went |
| `Bw[a] φ` | a believes whether φ | a has taken a side |
| `?[a] φ` | a is ignorant of φ | a knows neither φ nor ¬φ |
| `??[a] φ` | a is undecided about φ | a does not even lean |
| `K'[a] φ` | a considers φ possible | dual of `K` — she cannot rule it out |
| `B'[a] φ` | a has not ruled φ out | dual of `B` — φ survives somewhere she finds plausible |
| `S'[a] φ` | a cannot safely rule φ out | dual of `[]` |

Connectives are `!`, `&`, `|`, `->`, with `->` loosest and right-associative, and
modalities binding tightest — so `K[a] p & B[b] q -> r` is `((K[a]p) & (B[b]q)) -> r`. Both
`□` and `[]` work for safe belief, `¿` and `??` for undecided.

Nothing expressible in mB+ is inexpressible as a query: all six primitives, all nine sugar
forms, closed under the connectives and nested arbitrarily. `eval` runs the same parser and
lowering that check a `goal`, so a query and a goal cannot disagree.

```bash
$ delhi eval examples/coin_lie.delhi \
      -f "(K[alice] h | B[carol] !h) -> (C[*] Kw[alice] h & [][carol] h & !??[carol] h)"
true
```

### Where the "or" sits

Several attitudes are disjunctions underneath, and it is worth knowing which:

| Sugar | Is really |
|---|---|
| `Kw[a] φ` | `K[a] φ \| K[a] !φ` |
| `Bw[a] φ` | `B[a] φ \| B[a] !φ` |
| `?[a] φ` | `!Kw[a] φ`, so `!K[a] φ & !K[a] !φ` |
| `??[a] φ` | `!Bw[a] φ` |

That disjunction sits **outside** the modality, which is why the query language handles it
without ceremony — it is an ordinary Boolean combination of modal formulas. You can write
`Kw` by hand and get the same answer, using one filler in two holes:

```bash
$ delhi ask examples/coin_lie.delhi -q "Kw[alice] _"
  Kw[alice] (d)
  Kw[alice] (h)

$ delhi ask examples/coin_lie.delhi -q "K[alice] _ | K[alice] !_"
  K[alice] (d) | K[alice] !(d)
  K[alice] (h) | K[alice] !(h)
```

A disjunction **inside** the modality is a different thing entirely. `K[a](p | q)` can hold
when neither `K[a]p` nor `K[a]q` does — knowing that one of two things is so without knowing
which. That is the case `ask` cannot reach, since its candidates are literals under
modalities and never disjunctions; `eval` checks such formulas perfectly well. When the note
below says disjunctive knowledge is the missing class, it means this inner kind only.

### Knowledge versus safe belief

The subtle pair. Safe belief is belief that no *true* announcement can dislodge — it is
factive, so `[][a] φ` does imply `φ` — but knowledge is strictly stronger, because it
quantifies over everything the agent finds comparable rather than only over what it finds at
least as plausible as the actual world. The Coin Lie shows both at once, in its initial state:

```
$ delhi eval examples/coin_lie.delhi -f "[][carol] h"    # true
$ delhi eval examples/coin_lie.delhi -f "K[carol] h"     # false
```

Carol safely believes the coin is heads, and no truth will talk her out of it. She still does
not know it.

### How `ask` chooses what to try

`-d` sets how deeply the hole may nest, so `-d 1 -q "B[alice] _"` reaches
`B[alice] (B[carol] !h)` without your naming `B[carol]` yourself. At the REPL and in the
browser it is `:ask [depth] <pattern>`.

**Enumeration is necessarily restricted.** `{φ : B[a]φ}` is infinite — conjunction alone
generates without bound — so `ask` ranges over *modal literals*: a literal under some sequence
of `K`/`B`. That is the representation Muise et al.'s PDKB planner is built on, chosen here for
the same reason: finite, with a size that follows from the signature and the depth
(`Σ_{k≤d} (2·agents)^k · 2·atoms`). There is a ceiling, and you are told when it bites.

What that costs is precise, and it concerns disjunction *inside* a modality only — the outer
kind, as in `Kw`, is already there. Conjunctive candidates would be *redundant*, since `K` and
`B` are normal and `K[a](φ&ψ) ≡ K[a]φ & K[a]ψ`. Disjunctive ones would **not** be — `B[a](p|q)`
can hold when neither `B[a]p` nor `B[a]q` does — so knowing-that-one-of-these-holds is the one
genuinely missing class, and the direction to extend if enumeration is ever widened.

Two smaller points. The pattern is itself the filter: `B[alice] _` at depth 1 also returns
introspective truths like `B[alice] (B[alice] d)`, valid in KD45 and uninformative, and naming
the inner agent cuts them out. And when a pattern cannot see polarity — ignorance of `h` *is*
ignorance of `!h`, likewise `Kw`/`Bw` — only the positive form is listed. That last one is a
rendering rule, not part of the answer set.

`_` is a real token, not a placeholder string: a lone `_` is the hole, while `at_park` and `_x`
are ordinary identifiers, and filling is a tree substitution. That is load-bearing rather than
tidy — substituting textually tore identifiers containing underscores, and `!_` now negates
whatever fills it without needing defensive parentheses. §7.6 of the design spec gives the
grammar and both semantics formally.

## The tool

```
delhi check <FILE>                        parse, ground, and validate
delhi state <FILE>                        facts, and each agent's attitudes
delhi show  <FILE>                        the model itself, in the explicit form
delhi eval  <FILE> -f <FORMULA>           evaluate one formula
delhi ask   <FILE> -q <PATTERN>           enumerate what holds; `_` is the hole
delhi step  <FILE> -a <ACTION>…           apply actions in sequence
delhi dot   <FILE>                        Graphviz
delhi repl  <FILE>                        explore interactively
delhi bench <FILE> [-n CYCLES] -a <ACTION>…   model growth and timing
```

Exit codes are scriptable: `0` success or the formula holds, `1` the file was rejected or
the formula is false, `2` a usage error or a malformed formula.

### Reading a state

`show` prints the model — worlds and plausibility edges — which is exact and round-trips
through the parser, but leaves you to work out what any of it implies. `state` prints what it
*means*: the facts of the actual world, and every agent's attitude to every proposition,
sorted into the ones it knows, the ones it merely believes, and the ones it has no view on.

```
$ delhi state examples/muddy_children.delhi
actual world muddy(alice), muddy(bob), muddy(carol)

  alice  knows muddy(bob), muddy(carol)   undecided muddy(alice)
  bob    knows muddy(alice), muddy(carol)   undecided muddy(bob)
  carol  knows muddy(alice), muddy(bob)   undecided muddy(carol)
```

That is the puzzle's whole setup in three lines. It is also the best way to watch it resolve,
since `:state` in the REPL tracks the current state rather than the initial one:

```
> :do father_speaks()
> :do nobody_knows()
> :do nobody_knows()
> :state
  alice  knows muddy(bob), muddy(carol)   believes muddy(alice)
```

`undecided` has become `believes`, which is the moment the puzzle turns.

The view is first-order by construction — one line per agent, one attitude per proposition.
Nested attitudes do not fit that shape and are not shown; type the formula at the prompt
instead. Every operator works there, against the current state:

```
> B[alice] B[carol] !h
true
```

### The browser UI

For working through a scenario, `delhi-gui` is easier than the REPL: it shows the file, the
state, and the model at once, and re-checks as you type.

```bash
cargo run -p delhi-gui        # then open http://127.0.0.1:8080
cargo run -p delhi-gui 9000   # a different port
```

Editor on the left, attitudes and the plausibility graph on the right, console along the
bottom. The example files are in a dropdown; the ground actions are buttons, so a trace is
built by clicking; and the console takes the same input the REPL does — a formula, or `:do`,
`:undo`, `:reset`. Diagnostics arrive with line, column and caret exactly as they do on the
command line, and every error at once rather than the first.

The editor is syntax-highlighted: sections, clause keywords, modalities, variables and types
each get a colour, and `?[a]` reads as the ignorance modality while `?who` reads as a
variable — so `B[?who] secret(?whose)` distinguishes all three parts. It is a coloured layer
behind a real `<textarea>`, which keeps undo, selection and the caret working natively.

The graph labels each world with only the propositions that **differ between worlds**. In
Grapevine that is the difference between nine atoms truncated to `at(a,r1),at…` and the three
secrets, which is what the worlds actually disagree about.

It binds to loopback and has no authentication, because it is a debugging tool for the
machine it runs on. Do not expose it.

**`delhi-gui` is the one crate exempt from the zero-dependency rule** — it uses `tiny_http`
and `serde_json`. The rule exists so that the crates carrying the semantics and the language
stay auditable and keep building years from now; a debugging UI makes no such claim, and
hand-rolling HTTP to honour a rule whose reason does not reach it would buy nothing. The
exemption is kept honest by giving the crate no logic of its own: every answer it renders
comes from `delhi-lang`, where it is tested. `cargo build` and `cargo test` at the workspace
root skip it, so the core stays fast to build.

### Pictures

`dot` is not decoration. A model with four agents and sixteen worlds is unreadable as text
and obvious as a picture — the figures in the source papers *are* the debugging medium:

```bash
delhi dot examples/coin_lie.delhi | dot -Tpng > state.png
```

## How fast, and does it blow up

Both questions have the same answer, and it is the most useful thing measurement turned up.

`delhi bench` runs an action list repeatedly and reports model size and elapsed time three
ways: never contracting, quotienting by `~R` after each update, and by `~D`. Coin Lie, one
cycle being its three actions:

```
$ delhi bench examples/coin_lie.delhi -n 8 -a "announce_not_heads()" "distract_a()" "peek_c()"

cycle      worlds      cumul   worlds ~R      cumul   worlds ~D      cumul
    0           2          -           2          -           2          -
    1          16     72.9us          14    205.7us          14    168.7us
    2         128      2.2ms          16    797.7us          16    903.3us
    3        1024    133.6ms          16      1.4ms          16      1.7ms
    4        8192      9.47s          16      2.0ms          16      2.5ms
```

**Uncontracted, models grow without bound.** Product update crosses worlds with events, so
each cycle multiplies size by the number of distinguishable events — ×8 here. Cost grows
faster still, because the relation is `n_agents × n_worlds²` bits and almost every operation
walks it: 73 µs, 2.2 ms, 134 ms, 9.5 s. Four cycles in it is already unusable.

**Contracted, they reach a fixed point and stay there.** Sixteen worlds at cycle 2, and
sixteen at cycle 8. The trajectory is flat, and cost per cycle becomes constant — about
0.6 ms — so total time is linear in the number of actions rather than exponential.

Muddy Children behaves the same way, settling at 32 worlds while the uncontracted run reaches
8,192 and 8.8 seconds.

**But a fixed point is not the general case, and Grapevine shows it.** That domain's cycle —
c leaves the room, b tells a secret, c comes back — creates a genuinely new distinction every
time round, because each repetition adds another layer of who-was-present-for-what. Contraction
cannot merge what is really different:

```
$ delhi bench examples/grapevine.delhi -n 6 \
      -a "move(c,r1,r2)" "share(b,b,r1)" "move(c,r2,r1)"

cycle      worlds      cumul   worlds ~R      cumul
    0           8          -           8          -
    1          36    378.6us          36    528.1us
    2         160      5.1ms          92      5.1ms
    3         756    103.6ms         188     26.1ms
    4        3740      2.50s         588    123.0ms
    5        6052      6.52s        2460      1.33s
    6           -          -        7276      9.71s
```

Contraction still earns its place — roughly 2.5–5× fewer worlds at each depth, and it buys
two extra cycles before the cap — but it converts an explosion into a slower explosion, not
into a flat line.

So the honest answer to "how fast is this" comes in two parts. **Quotient, always**: the
difference is thousands of times over a dozen actions, not a constant factor. **And whether
that is enough depends entirely on the domain**: when actions stop producing distinguishable
events you get a fixed point and effectively unlimited depth; when they keep producing them,
as in gossip domains, you get maybe five or six cycles before it stops being interesting. A
planner over this will need more than contraction — depth bounds, heuristics, or a restricted
representation of the kind PDKB uses.

One incidental observation from these runs: `~R` and `~D` produced *identical* world counts at
every step in every domain here. The 5–10 % merge gap measured over random models in
`research/bisimulation/` did not show up on any real domain, which is worth knowing before
anyone spends effort trying to exploit it.

That finding changed the tool. `step` and the REPL now quotient by `~R` after every update,
which took twelve Coin Lie actions from ~9.5 s and 8,192 worlds to **29 ms and 16 worlds**.
`~R` is used rather than the coarser `~D` because it is proved sound *and* a congruence for
product update, so it cannot change the answer to any query; `~D` merges more but its
congruence status is open (§6.3), which makes it unsafe to apply between updates.

`bench` checks that too. It evaluates the file's goal at the end of all three trajectories
and reports a disagreement loudly — for `~R` that would mean a bug against a proved
congruence, and for `~D` it would be evidence on the open question. Across these domains all
three agree, and `~R` and `~D` reach the same fixed point.

These are wall-clock numbers from one release build on one laptop, so read the ratios and the
shapes rather than the absolute microseconds. And none of this makes the underlying problem
easy — epistemic plan existence is undecidable in general, and no implementation detail
changes that.

## Layout

| Crate | Holds |
|---|---|
| `delhi-syntax` | hash-consed formulas over six primitive operators, plus the derived attitudes |
| `delhi-mb` | the mB+ semantics: bitset models, frame validation, entailment, bisimulation, product update |
| `delhi-core` | the backend-agnostic trait a planner would be generic over |
| `delhi-lang` | the front end: lex → parse → ground → lower |
| `delhi-cli` | the `delhi` binary |

`delhi-lang` depends on the semantics; the semantics does not depend on the front end.

## What is new here

delhi is a reimplementation, so most of it is transcription — the mB semantics, the action
types, the observability model, and the Coin Lie figures all come from Buckingham's thesis
and the KR papers. Five things do not.

**A soundness proof and a measurement where the thesis leaves a gap.** The thesis notes on
p. 68 that its bisimulation algorithm "is not complete in the multi-agent case" and attaches
no number. The soundness question is the one that actually matters — an unsound contraction
merges states that are not interchangeable, which means wrong plans rather than slow ones —
and it was left open. `research/bisimulation/` settles both. Soundness is proved, by a
level-increasing argument using local connectedness and finiteness, then checked over 454,290
exhaustive models and 24 million random ones — zero violations. A separate sweep over 451,730
exhaustive models confirms the containment `~R ⊆ ~D` directly. The incompleteness is then
measured:

| worlds | agents | models | incomplete |
|---|---|---|---|
| 3 | 1 | 115 | **5.22 %** |
| 3 | 2 | 2,645 | **5.44 %** |
| 4 | 1 | 2,595 | **10.17 %** |
| 4 | 2 | 448,935 | **9.38 %** |

**And a diagnosis, which turned up something about the original implementation.** The rate
is as high with one agent as with two — but the thesis attributes its incompleteness to a
technique (Andersen, Bolander, van Ditmarsch et al.) that is *complete* in the single-agent
case. So the Java implementation is not running the algorithm the thesis says it runs; it is
running plain Kripke partition refinement over `Rᵢ` and its converse. The cause of the loss
is refining against `Rᵢ⁻¹` — a relation that no operator in the language is a box over. The
smallest witness is three worlds and one agent, and it is in the findings file.

**Two bisimulation notions, side by side.** `~R` is what the thesis describes and the Java
implements; `~D` refines against one relation per operator and is exactly modal equivalence
for the K/B/□/C fragment. `~R ⊆ ~D` is verified. Whether `~D` is a congruence for product
update is open, and it is worth answering: if it is, the ~10 % merge improvement applies
directly to search.

**Two transition rules, and evidence for which is authoritative.** The thesis and the mB
draft define product update differently. Both are implemented (`UpdateRule::Thesis` and
`UpdateRule::MbDraft`), and the Coin Lie turns out to be the differential case: under the
draft rule the lie does not land at all. That is a concrete argument for the thesis rule,
found by running both rather than by reading.

**A construction the source material does not have.** `initially { … }` lets you declare
facts and attitudes and get a model, rather than writing worlds and edges by hand. Facts fix
the designated valuation, `?[a] p` declarations fix which atoms vary, belief declarations
score worlds, and `u Rᵢ v` holds when the worlds are comparable and `score(v) ≥ score(u)`.
The scoring heuristic is not obviously complete, so the construction does not trust itself:
it re-checks by entailment that every declaration it was given actually holds in the model
it built, and reports the ones that do not. Its limits — nested belief, disjunction and
conditional belief are assertion-only — are reported to the author rather than silently
tolerated.

Beyond those: `B^ψ` and `□` are first-class operators here, which they are not in mB's object
language — though they are Baltag & Smets's operators, not delhi's, and the models could
always interpret them (see **A note on the name** below, which is the honest accounting); the
surface language is delhi's own rather than DEPL; and two known semantic gaps are pinned by
tests that fail *by design* and are marked ignored, rather than going undocumented.

## Where this sits

Epistemic planning has two traditions that pull against each other. One starts from
**expressiveness** — dynamic epistemic logic will represent anything, at the cost of the
modeller hand-building event models and of undecidable plan existence. The other starts from
**tractability** — restrict the representation until an off-the-shelf planner can be pointed
at it. Action languages sit in between: keep a semantics grounded in DEL, but let the
modeller declare who observes what and derive the event models from that.

delhi is in the third camp, and specifically it implements **mB**, which is the branch of
that lineage that swapped knowledge for belief.

### Expressiveness

| | Belief ≠ knowledge | Revision on contradicting evidence | Second-order false belief | False belief about *who observed* | Conditional `B^ψ` / safe `□` |
|---|---|---|---|---|---|
| **DEL** (Baltag–Moss–Solecki; van Ditmarsch et al.) | yes | via specific update rules | yes | yes | in extensions |
| **Baltag & Smets** (2006, 2008) | yes | yes — this is where it comes from | yes | — | **yes, its home ground** |
| **mA\*** (Baral, Gelfond, Pontelli & Son) | limited | crude — collapses *all* uncertainty | no | no | no |
| **mA\* + higher-order observability** (KR 2024) | limited | as mA\* | yes | yes | no |
| **mB** (Buckingham thesis; KR 2021) | yes | yes, preserving other uncertainty | yes | yes (local dynamic observability) | in the models, not the language |
| **mB+ / delhi** | yes | yes | yes | yes | **yes, as query operators** |
| **EFP / EFP 2.0** (Le, Fabiano, Son & Pontelli) | knowledge-oriented | — | — | — | no |
| **PDKB / RP-MEP** (Muise et al.) | yes (in the belief work) | bounded | to the depth bound | no | no |

### Machinery

| | Event models | State representation | Planner | Notes |
|---|---|---|---|---|
| **DEL** | hand-built per problem | Kripke models | none inherent; plan existence undecidable in general | The complaint the action languages exist to answer |
| **Baltag & Smets** | action-priority update | plausibility models | none | A logic, not a planning system |
| **mA\*** | derived from observability declarations | Kripke (S5) | via EFP and others | The ancestor of everything below it |
| **mB** | derived, per possible world | plausibility models | mecaPlanner (Java) | Adds local dynamic observability and hypothetical actions |
| **mB+ / delhi** | derived, per possible world | plausibility models, bitset-backed | **none yet** | Model checking and reasoning only |
| **EFP 2.0** | derived | *two* — Kripke and possibilities | forward search (C++) | Possibilities sidestep bisimulation contraction entirely |
| **PDKB / RP-MEP** | compiled away | bounded-depth modal literals | any classical planner | Buys enormous scale by giving up disjunctive uncertainty |

### What the differences actually amount to

**mA\* → mB is the belief step**, and the sharpest way to see it is what each does when an
agent is contradicted. mA\*'s revision removes *all* of an agent's uncertainty and hands it
certain knowledge of the true state — so in the Birthday Bicycle story, Timmy peeking in the
basement would teach him not only that the bicycle is there but everything else besides. mB
revises what was actually contradicted and leaves the rest of the agent's uncertainty
standing. That is the difference between a system that can model a *specific* false belief
being corrected and one that can only reset an agent to omniscience.

**Baltag & Smets is where the semantics comes from, and it is not an action language.** The
plausibility preorders, the conditional belief `B^ψ`, the safe belief `□` — all of that is
theirs. What mB contributes is putting it under an action language, so a modeller writes
`announces !bicycle` and `mom observes` rather than drawing an event model.

### A note on the name, since it matters for citation

**"mB+" is delhi's own label, not Buckingham's.** It was coined for this project and appears
nowhere in the thesis or the KR papers. Cite mB; mB+ is a name for what this implementation
does on top of it.

What it denotes is deliberately narrow. The thesis defines its object language in Definition 1
(§5.1.1) with exactly six clauses — `p`, `¬φ`, `φ ∧ ψ`, `Kᵢφ`, `Bᵢφ`, `C_gφ` — and no more.
delhi adds two operators to that language:

| | in mB's models | in mB's object language | in delhi |
|---|---|---|---|
| `Bᵢ^ψ φ` conditional belief | yes — the models *are* Baltag–Smets models | no | yes |
| `□ᵢ φ` safe belief | yes, same reason | no | yes |

So the extension is not new logic and is not ours as logic. Both operators are Baltag & Smets's,
and both were already *interpretable* in mB's models — the thesis simply does not lift them into
the language it defines. delhi lifts them, implements them, and adds nine derived attitudes as
sugar over the six primitives. The surface language also allows a condition on effects
(`causes p if φ`), which the thesis's action tuple expresses by splitting events instead.

One item originally scoped into the `+` was **not** delivered: a fix to the announcement
construction. The design spec's §4.7(a) describes a defect that, on implementation, did not
manifest as described, so it is documented and pinned rather than fixed. Do not read "mB+" as
including it.

**EFP and PDKB are solving the problem delhi has not started on.** Both are planners; delhi
is not. They are also the two most interesting reference points for what a delhi planner
should do, and they answer the scaling question in opposite ways. EFP 2.0 carries two state
representations and can use "possibilities" to avoid bisimulation contraction altogether —
directly relevant, since `research/bisimulation/` measures what contraction costs and what it
leaves on the table. PDKB goes the other way and restricts the representation until a
classical planner applies, trading away disjunctive uncertainty and unbounded modal depth for
orders of magnitude of scale. delhi currently pays the full price of the general
representation and gets none of the planning benefit, which is exactly the gap to close.

**On the KR 2024 line.** delhi implements the thesis and KR 2021 semantics. The KR 2024
paper's Example 1 — the second-order Sally-Anne, where Anne is wrong about whether she was
seen — does work here, and `examples/sally_anne_second_order.delhi` is that example with its
trace pinned by a test. But KR 2024's treatment of higher-order action observability is more
general than what was implemented, and two related gaps are open and marked as such: the
hypothetical-actions treatment in `§4.8` of the design spec, and `§4.7(a)`. Do not read the
table as a claim that delhi covers KR 2024.

*This table is my reading of the literature and the code, not a surveyed claim. The rows for
EFP and PDKB in particular are the ones I am least sure of — corrections welcome, and they
should be made here rather than carried in someone's head.*

## Why Rust

The honest short answer is that this problem is small-but-brutal, and constant factors decide
what is feasible.

**The state space multiplies.** Product update crosses worlds with events at every step. The
Coin Lie runs 2 → 4 → 8 → 16 worlds in three actions, and that is a toy with three agents and
two atoms. Plan search compounds it further. Nothing about the asymptotics changes with
language choice — but the point at which a problem stops fitting on a laptop moves a long way,
and that point is where the research happens.

**The hot code is exactly what interpreters are worst at.** Valuations and relation rows are
bitsets over `u64` words; entailment is memoised on `(FormulaId, WorldId)` pairs; contraction
is partition refinement over those bitsets. This is tight loops of index arithmetic and word
operations, with no vectorisable numeric work to hand off to a library. Python was the initial
plan and would have been the wrong tool once the ideas settled — not by 2×, but by the kind of
factor that changes which experiments you can run.

**Algebraic data types with exhaustive matching, which is not a nicety here.** The language has
six primitive operators, nine derived attitudes, three action kinds, and three observer
classes. Adding an operator should fail to compile in every place that must handle it. That
consideration ruled out C: the arena is hash-consed with interned ids and raw index arithmetic
over bitsets, which is precisely the code where manual memory management produces
use-after-free and out-of-bounds bugs, and where a missing switch case is silent. Odin and Zig
were considered and are decent fits, but have smaller ecosystems and no borrow checking for
aliasing-heavy arena code. OCaml or Haskell would have given the type system, at the cost of GC
pauses and less predictable memory when holding large state sets.

**Zero dependencies, deliberately.** Every crate, including the CLI — argument parsing is
hand-rolled because six subcommands do not justify a supply chain. A research artifact should
still build in five years, and a reviewer should be able to audit all of it.

The cost was real and worth naming: the borrow discipline showed up repeatedly during
implementation, mostly around holding a `&Problem` while needing `&mut Problem.store`. It slowed
things down. It also never once produced a wrong answer at runtime.

## What validates it

274 tests, plus 2 that fail by design and are marked ignored — they pin known gaps rather
than pretending they are absent.

The load-bearing one is `examples/coin_lie.delhi`, which reproduces the published figures
end to end. The same scenario exists twice: once built through the Rust API in
`crates/delhi-mb/tests/coin_lie.rs`, and once as the text file above. They must agree
assertion for assertion. If the two ever diverge, the front end is wrong and the semantics
is right. Every other example is pinned the same way, by its headline claim rather than by
merely parsing — `tests/examples.rs` asserts that Sally looks in the basket, that John is
wrong about Mary, that the muddy children conclude on the third round *and not the second*.

Beyond that: the pretty-printer round-trips through the parser under full bisimulation, so
it cannot quietly invert an edge; property tests cover frame preservation across update and
the bridge axioms between knowledge and belief; and a regression test reproduces a measured
incompleteness result from `research/bisimulation/`.

## Status

v0.1. Model checking and reasoning work. **Planning does not exist yet** — `delhi-core`
declares the interface a search would use, and nothing implements it.

`todo.md` carries the open questions and triaged follow-ons. The design is specified in
`docs/superpowers/specs/2026-07-25-delhi-core-design.md`, which is worth reading before
changing anything in `delhi-mb`.

## Background

delhi is a rewrite of mecaPlanner (Java, DEPL) with its own surface language. The semantics
comes from:

- Buckingham, thesis — the mB semantics and the Coin Lie figures (5.4–5.10)
- Buckingham, Sarathy, Scheutz & Son, *A Multi-Agent Epistemic and Doxastic Action Language
  with Belief Revision and Local Dynamic Observability* (KR 2021)
- Buckingham, Scheutz, Son & Fabiano, *Action Language mA\* with Higher-Order Action
  Observability* (KR 2024) — not implemented; see the note in **Where this sits**
- Baral, Gelfond, Pontelli & Son — mA\*, the action language mB builds on
- Baltag & Smets (2006, 2008) — the plausibility models, conditional belief and safe belief

Referenced for comparison rather than implemented: Baltag, Moss & Solecki (1998) and van
Ditmarsch, van der Hoek & Kooi (2007) for DEL; Le, Fabiano, Son & Pontelli for EFP and
Fabiano et al. for EFP 2.0; Muise, Belle, Felli, McIlraith, Miller, Pearce & Sonenberg for
RP-MEP and the PDKB representation.

The examples cite their own sources: Wimmer & Perner (1983) and Baron-Cohen, Leslie & Frith
(1985) for Sally-Anne; Bräuner, Blackburn & Polyanskaya (2016) for its second-order variant;
Perner & Wimmer (1985) for the ice-cream van; Sullivan, Zaitchik & Tager-Flusberg (1994) for
the Birthday Bicycle.

PDFs and the original Java source are in `refs/`.
