# Changelog

Notable changes per release. Dates are the tag date.

## 0.1.3 — 2026-07-29

### Added

- `--json` on `check`, `state`, `eval` and `ask`. Each emits exactly one JSON object on
  stdout, **errors included**, so a caller never has to decide whether what it read was an
  answer or a diagnostic. Exit codes are unchanged, so both signals remain available.
- `python/delhi.py` — a `Domain` class wrapping the CLI. Standard library only, nothing to
  install, and shipped inside the release archives. A malformed formula raises rather than
  returning `False`, since a typo must not read as a refuted hypothesis.
- A README section on calling delhi from Python, including the limit that matters: one
  process per call is ≈3–5 ms on Linux and ≈20–25 ms on Windows — fine for batch work,
  wrong inside a training loop.

### Internal

- The JSON emitter is hand-rolled rather than `serde_json`, so `--json` works in the
  `--no-default-features` build whose whole point is an empty dependency graph.
- `python/test_delhi.py` drives the real binary and runs in CI on all three platforms, so
  the wrapper and the JSON schema cannot drift apart unnoticed.

## 0.1.2 — 2026-07-28

Browser UI only. The CLI is unchanged from 0.1.1.

### Added

- Command history at the prompt. Up and Down walk previous commands, `esc` clears the
  line, and the list survives a reload. A half-typed line is stashed when the walk begins
  and restored on the way back, so going to look does not eat what you were writing.

### Changed

- The prompt moved from the foot of the window to just under the toolbar. It is a control
  and belongs with the controls; at the bottom edge it was the furthest thing on screen
  from both the file it queries and the panel its answers land in.

### Fixed

- Scrollbars were the browser's default light-grey chrome on a dark page. The root element
  now declares `color-scheme: dark`, which also re-skins the `<select>` and the caret, and
  the bars are styled down from ~17px to 10px — width that matters in panels only a few
  hundred pixels wide.

## 0.1.1 — 2026-07-28

### Added

- `delhi eval` and `delhi state` take `-a ACTION…`, so a formula or a state view can be
  asked about the situation a trace reaches rather than only about the initial state.
  The README had claimed this of all three query commands; it had only ever been true of
  `delhi ask`.

### Fixed

- Documentation of the three observer classes. `aware` means the agent knows the action
  occurred but not how it turned out — so an aware bystander to a peek comes to know that
  the peeker *knows whether*, without learning which way. The behaviour was already
  correct; the README described it in three words and the mechanism not at all.

### Internal

- The trace loop is shared between `ask`, `eval` and `state` instead of copied, so one
  place contracts after each step and one place decides which failures are usage errors
  (exit 2) and which are answers (exit 1).
- A test pinning `aware` against *oblivious*. The existing coverage compared `observes`
  against `aware`, but every non-peeker in Coin in the Box is aware, so nothing stated the
  distinction that makes three classes rather than two.

## 0.1.0 — 2026-07-28

First release.

- **Semantics** — mB+ plausibility models: knowledge (S5), belief (KD45), safe belief,
  conditional belief and common knowledge; product update; `~R` and `~D` bisimulation
  with contraction wired into every trace.
- **Language** — a declarative `.delhi` file format: signature, initial state written as
  attitudes or as an explicit model, goals, invariants, non-recursive definitions, Horn
  rules over constants, and three action kinds against three observer classes.
- **Queries** — evaluation of any mB+ formula, and `ask` patterns that enumerate every
  formula of a given shape that holds, to a chosen modal depth.
- **Tools** — `delhi` CLI with `check`, `state`, `show`, `eval`, `ask`, `step`, `dot`,
  `repl` and `bench`; and `delhi gui`, a local browser UI over a directory of `.delhi`
  files.
- **Examples** — ten domains, from the Coin Lie of Buckingham's thesis through
  Sally-Anne and the Birthday Bicycle Story to Grapevine and the EFP benchmarks.
