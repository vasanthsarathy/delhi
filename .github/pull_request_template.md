<!-- Link the issue this closes, e.g. "Closes #12". -->

## What and why

<!-- What changed, and the reasoning. The diff shows what; this should say why. -->

## How it was verified

<!--
Beyond "tests pass". The most valuable line here is evidence a new test would FAIL
without the change — several tests in this repo passed against the very bug they were
meant to pin, and only a deliberate sabotage of the fix revealed it.
-->

## Checklist

- [ ] `cargo test --workspace`
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
- [ ] `cargo fmt --all`
- [ ] Ran `sh tools/bundle-examples.sh` — *only if `examples/` changed*
- [ ] Ran `python python/test_delhi.py` — *only if `--json` output changed*
- [ ] Docs updated (`README.md`, `docs/book/`) if behaviour changed
