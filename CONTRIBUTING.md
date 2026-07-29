# Contributing

Thanks for looking. delhi is a small project, so the process is short.

**Releasing is a separate document: [RELEASING.md](RELEASING.md).**

---

## The shape of it

```
issue  →  triage  →  branch  →  PR  →  CI  →  review  →  merge to master
                                                              │
                                                    (when there is enough
                                                     to warrant one)
                                                              ↓
                                                      tag  →  release
```

Two things worth naming up front, because they are the parts people guess wrong:

**`master` is always releasable.** Anything merged should be safe to tag. That is what
lets a release be one command, and why CI gates every PR on the full suite across three
platforms rather than trusting a local run.

**Merging and releasing are separate decisions.** Work lands on `master` when it is done;
a release happens when there is enough to be worth one. Several merges usually accumulate
into one version. There is no need to release per change, and no harm in releasing often —
`cargo install delhi --force` is the update path either way.

---

## 1. Open an issue first

For anything beyond a typo. It is cheaper to be told "that is already possible, here is
how" than to write the patch first.

Use the templates — [bug report](https://github.com/vasanthsarathy/delhi/issues/new?template=bug_report.yml)
or [feature request](https://github.com/vasanthsarathy/delhi/issues/new?template=feature_request.yml).
For a bug, the single most useful thing is **a `.delhi` file small enough to paste** plus
the exact command and what you expected instead. Most epistemic-logic bugs are impossible
to guess at without the domain.

If you are unsure whether something is a bug or a misunderstanding of the semantics, open
it as a bug. Several "bugs" have turned out to be the documentation not explaining what
`aware` means, which is still a defect worth fixing.

## 2. Set up

```bash
git clone https://github.com/vasanthsarathy/delhi && cd delhi
cargo build --workspace
cargo test --workspace
```

No system dependencies, no code generation step, nothing to install beyond a Rust
toolchain. The browser UI needs no build step — it is one HTML file compiled into the
binary.

## 3. Branch and work

Branch from `master`. Name it after the thing, not the ticket number: `aware-condition-fix`
reads better than `issue-42` in six months.

### What good looks like here

- **Tests that would fail without the change.** The habit that has caught the most in this
  project is checking that a new test *fails* against the bug it names — several tests
  written here passed against the very defect they were meant to pin, and only a deliberate
  sabotage of the fix revealed it. If a test cannot fail, it is documentation.
- **Comments that say why, not what.** The code says what. Existing comments explain the
  reasoning behind a choice, especially where the obvious alternative is wrong. Match that.
- **Small, focused changes.** One concern per PR.

### The core is deliberately stable

`crates/delhi-mb/` and `crates/delhi-syntax/` hold the semantics — the model checker,
entailment, product update, bisimulation. They are validated against Buckingham's thesis
figures, and changes there need a stronger argument than changes to the front end. Not
off-limits, just not casual.

### Before you push

The whole CI gate, locally:

```bash
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all
cargo build -p delhi --no-default-features
```

Two extras if you touched the relevant thing:

```bash
sh tools/bundle-examples.sh                            # if you changed examples/
DELHI_BIN="$PWD/target/release/delhi" python python/test_delhi.py   # if you changed --json
```

## 4. Open a PR

Link the issue. Say what changed and why; the template asks for what it needs.

CI runs everything above on Linux, macOS and Windows, plus an MSRV check, a docs build, and
a smoke test that runs the release binary from a directory outside the checkout — the case
a repository-local test cannot see.

Commit messages: a short imperative subject, then a body explaining the reasoning if there
is any to explain. `git log` in this repo is the house style.

## 5. Merge

Squash or merge, either is fine. Once it is on `master`, CI has already proved it and the
docs site redeploys automatically.

---

## Where things live

| | |
|---|---|
| `crates/delhi-syntax` | hash-consed formulas, the operators |
| `crates/delhi-core` | the trait a planner would be generic over |
| `crates/delhi-mb` | the mB+ semantics — models, entailment, update, bisimulation |
| `crates/delhi-lang` | the front end: lex → parse → ground → lower, plus queries |
| `crates/delhi` | the `delhi` binary |
| `crates/delhi-gui` | the browser UI, behind a default-on `gui` feature |
| `examples/` | the ten domains; **regenerate the bundle after editing** |
| `python/` | the Python wrapper and its tests |
| `docs/book/` | the mdBook manual |
| `docs/site/` | the landing page |
| `docs/superpowers/` | the original design spec and implementation plans |

## Conventions that are not obvious

- **Zero external dependencies** in every crate except `delhi-gui`, which needs an HTTP
  server. `delhi` itself builds dependency-free with `--no-default-features`, and CI proves
  it on every push. Adding a dependency to a core crate needs a real argument.
- **No crate reads files outside its own directory.** `include_str!("../../..")` produces a
  crate that publishes and then fails to compile for whoever installs it. See
  [RELEASING.md](RELEASING.md#a-crate-may-not-read-files-outside-its-own-directory).
- **`rustfmt.toml` is tuned to the existing style** — `max_width = 100`,
  `use_small_heuristics = "Max"`. Run `cargo fmt --all`; CI checks it.
- **Plausibility direction**: `u R[i] v` means *v is at least as plausible as u*. It
  increases along the arrow and, in surface syntax, to the right. An inverted ordering is
  still a well-formed model, so nothing will complain — every answer will just be quietly
  backwards.

## Questions

Open an issue, or a [discussion](https://github.com/vasanthsarathy/delhi/discussions) if it
is more of a conversation than a task.
