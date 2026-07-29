# From Python

Most epistemic-reasoning work sits inside an ML or cognitive-modelling stack written in
Python. `python/delhi.py` wraps the CLI: standard library only, nothing to install. It
ships inside every release archive, or take it from
[the repository](https://github.com/vasanthsarathy/delhi/blob/master/python/delhi.py).

Put `delhi` on your `PATH`, drop the file beside your code:

```python
from delhi import Domain

d = Domain("examples/coin_lie.delhi")
d.do("distract_a()", "peek_c()")          # apply a trace

d.eval("K[bob] Kw[carol] h")              # True  — bob heard the peek
d.eval("K[alice] Kw[carol] h")            # False — alice was distracted
d.eval("?[alice] Kw[carol] h")            # True  — she cannot even say

s = d.state()
s.facts                                   # ['d', 'h']
s.agents[0].agent, s.agents[0].knows      # ('alice', ['d', 'h'])

d.reset().do("announce_not_heads()", "distract_a()", "peek_c()")
d.ask("B[alice] B[carol] _")              # ['B[alice] B[carol] (d)',
                                          #  'B[alice] B[carol] (!h)']
d.eval("B[alice] B[carol] !h & K[carol] h")   # True — the false belief
```

## What the API gives you

| | |
|---|---|
| `Domain(path)` | parse and check; raises `DelhiError` on a bad file |
| `.do(*actions)` · `.undo(n)` · `.reset()` | manage the trace; all return `self` |
| `.actions` | every ground action name |
| `.eval(formula)` → `bool` | raises on a malformed formula |
| `.eval_many(formulas)` → `dict` | |
| `.holds(*formulas)` → `bool` | short-circuits |
| `.ask(pattern, depth=0)` → `list[str]` | |
| `.ask_full(...)` → `dict` | with `considered` and `truncated` |
| `.state()` → `StateView` | `.facts`, `.agents`, `.worlds`, `.violated` |

Two behaviours worth knowing. `Domain` replays the trace from the initial state on every
call rather than holding a live model, so `undo()` is exact and two `Domain`s over one file
cannot drift. And a malformed formula **raises** rather than returning `False` — a typo
must not read as a refuted hypothesis.

## Underneath

Every command takes `--json` and emits exactly one object on stdout, **errors included**,
so a caller never has to decide whether what it read was an answer or a diagnostic:

```bash
$ delhi eval examples/coin_lie.delhi -f "B[carol] h" --json
{"ok":true,"value":true}
$ delhi eval examples/coin_lie.delhi -f "K[nobody] h" --json
{"ok":false,"error":"1:1: `nobody` is not a declared agent\n  K[nobody] h\n  ^^^^^^^^^^^"}
```

Exit codes are unchanged by `--json`, so both signals stay available.

## How fast, and when this is the wrong tool

Each call is one process launch: **≈3–5 ms on Linux, ≈20–25 ms on Windows**. The model
checking itself is microseconds, so at that rate you are timing `fork`, not delhi.

Fine for scripting, dataset generation and batch evaluation — a few thousand checks is
seconds. **Not** fine inside a training loop that queries per step: at 20 ms a call, a
million queries is six hours of process creation.

If that is your shape, two ways out. `delhi gui` serves `/api/eval`, `/api/ask` and
`/api/state` over loopback HTTP, and one long-lived process answering many requests avoids
the launch entirely — though it is built as a debugging UI, so treat that surface as
unstable. Otherwise
[open an issue](https://github.com/vasanthsarathy/delhi/issues): real PyO3 bindings are the
answer, and knowing which calls sit in your hot path is what would shape them.
