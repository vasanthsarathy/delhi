# Command line

```
delhi check <FILE>                          parse, ground, and validate
delhi state <FILE> [-a ACTION]…             facts, and each agent's attitudes
delhi show  <FILE>                          the model itself, in explicit form
delhi eval  <FILE> [-a ACTION]… -f φ        evaluate one formula
delhi ask   <FILE> [-a ACTION]… -q π        enumerate what holds; `_` is the hole
delhi step  <FILE> -a <ACTION>…             apply actions in sequence
delhi dot   <FILE>                          Graphviz
delhi repl  <FILE>                          explore interactively
delhi gui   [DIR] [-p PORT]                 browser UI over a folder of .delhi files
delhi bench <FILE> [-n CYCLES] -a ACTION…   model growth and timing
delhi --version | --help
```

## Flags

- **`-a ACTION…`** — apply a trace before answering. Takes any number of ground action
  names. Available on `state`, `eval`, `ask`, `step` and `bench`.
- **`-f FORMULA`** — the formula for `eval`.
- **`-q PATTERN`** — the pattern for `ask`; `_` marks the hole.
- **`-d DEPTH`** — modal nesting depth for `ask` candidates. Default 0.
- **`--json`** — one JSON object on stdout, errors included. On `check`, `state`, `eval`
  and `ask`.

## Exit codes

| | |
|---|---|
| `0` | success, or the formula holds |
| `1` | the file was rejected, or the formula does not hold |
| `2` | usage error, a malformed formula, or an unknown action |

The `1` / `2` split is deliberate: `2` means the question was wrong, `1` means the answer
was no. A script that conflates them turns a typo into a refutation.

## Colour

On when stdout is a terminal, off otherwise — so `delhi dot … | dot -Tpng` stays
byte-clean. `NO_COLOR=1` forces it off, `CLICOLOR_FORCE=1` forces it on through a pipe.

## The REPL

```bash
$ delhi repl examples/coin_lie.delhi
> B[carol] h              a bare formula evaluates
> :do peek_c()            apply an action
> :undo                   drop the last
> :reset                  clear the trace
> :state                  the state view
> :ask B[alice] _         enumerate; `:ask 2 …` sets the depth
> :actions                what can be applied
> :help
```

## The browser UI

`delhi gui` serves the current directory — your own folder of `.delhi` files, not the
repository. The ten examples are compiled into the binary, so a fresh install still opens
on something readable.

```bash
cd ~/my-domains && delhi gui        # or: delhi gui ~/my-domains -p 9000
```

Editor with syntax highlighting and clickable errors, an actions rail, a live state view, a
model graph, and a console with command history. Every divider is draggable. It binds to
loopback and has no authentication, because it is a debugging tool for the machine it runs
on — do not expose it.
