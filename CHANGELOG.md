# Changelog

Notable changes per release. Dates are the tag date.

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
