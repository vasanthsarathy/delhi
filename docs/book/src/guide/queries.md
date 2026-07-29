# Asking questions

Three ways, depending on how much you already know about what you are looking for.

| | Answers | Reach for it when |
|---|---|---|
| `delhi eval -f φ` | is this one formula true? | you know what to check |
| `delhi ask -q π` | which formulas of this shape hold? | you do not yet know what to look for |
| `delhi state` | every agent's stance on every proposition | you want the lay of the land |

All three take `-a ACTION…` to run a trace first, and all three exist at the REPL prompt
and in the browser console as a bare formula, `:ask`, and `:state`.

## eval

```bash
$ delhi eval examples/coin_lie.delhi -a "peek_c()" -f "K[bob] Kw[carol] h"
true
```

Exit code `0` if it holds, `1` if not, `2` if the formula is malformed — so a shell can
branch on the answer, and a typo never looks like a refutation.

Any mB+ formula works: the operators, nested arbitrarily, closed under `&`, `|`, `!`.

## ask

`ask` takes a **pattern** with `_` marking a hole, and reports every formula of that shape
that holds:

```bash
$ delhi ask examples/coin_lie.delhi -q "B[carol] _"
  B[carol] (!d)
  B[carol] (h)
2 of 4 candidates at depth 0
```

The hole is filled from the *modal literals* of the domain — each proposition and its
negation, then attitudes about those, and so on. `-d N` sets how deep that nesting may go.

The hole may appear **more than once**, and all occurrences are filled with the same
formula. That is what makes the interesting recipes possible:

| The question | Write |
|---|---|
| What does alice believe? | `-q "B[alice] _"` |
| What can't she settle? | `-q "?[alice] _"` |
| What does she think carol believes? | `-q "B[alice] B[carol] _"` |
| **What does she believe that is false?** | `-q "B[alice] _ & !_"` |
| Where do alice and carol disagree? | `-q "B[alice] _ & B[carol] !_"` |
| What does alice know that carol doesn't? | `-q "K[alice] _ & !K[carol] _"` |
| What does carol believe without knowing? | `-q "B[carol] _ & !K[carol] _"` |

The false-belief recipe is the one to try first, because it finds things you would not have
thought to check.

**Depth costs.** Candidates grow fast — the count is reported so you can see it, and the
answer says `truncated` if the search hit its cap. Start at 0 or 1.

## state

```bash
$ delhi state examples/coin_lie.delhi -a "announce_not_heads()"
actual world !d, h

  alice  knows !d, h
  bob    knows !d, h
  carol  knows !d   believes !h
```

First-order by construction — one line per agent, one attitude per proposition. Nested
attitudes do not fit that shape and are not shown; use `eval` or `ask` for those.

The three lists partition the propositions: every proposition is in exactly one of `knows`,
`believes`, `undecided` for each agent.

## Ignorance and "knows whether"

Two sugar forms worth knowing, because they are what you actually want more often than `K`:

- `Kw[a] φ` — **knows whether**: `K[a] φ | K[a] !φ`. The agent has settled the question,
  either way.
- `?[a] φ` — **ignorance**: `!Kw[a] φ`. The agent has not.

`Bw` and `??` are the belief-level counterparts. Both genuinely need the disjunction —
"knows whether" is not expressible without it, which is why they exist as operators rather
than as something you write out each time.

## Machine-readable output

Every one of these takes `--json`, which emits exactly one object on stdout, errors
included:

```bash
$ delhi eval examples/coin_lie.delhi -f "B[carol] h" --json
{"ok":true,"value":true}
```

See [From Python](./python.md).
