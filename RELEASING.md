# Releasing

A release is **one command** once the changelog is written: push a tag, and CI builds the
binaries, publishes to crates.io, and redeploys the docs.

This file exists for the parts that are not obvious, most of which were learned the hard
way during 0.1.4. Read [Things that will bite you](#things-that-will-bite-you) before your
first one.

---

## The checklist

### 1. Decide the version

delhi is pre-1.0, so `0.1.x` for anything that does not break a published API and `0.2.0`
if it does. `cargo install delhi --force` gets whatever is newest either way.

### 2. Bump it — one place

**`Cargo.toml` at the repository root.** Two spots, adjacent:

```toml
[workspace.package]
version = "0.1.5"          # <- here

[workspace.dependencies]
delhi-syntax = { path = "crates/delhi-syntax", version = "0.1.5" }   # <- and these five
delhi-core   = { path = "crates/delhi-core",   version = "0.1.5" }
delhi-mb     = { path = "crates/delhi-mb",     version = "0.1.5" }
delhi-lang   = { path = "crates/delhi-lang",   version = "0.1.5" }
delhi-gui    = { path = "crates/delhi-gui",    version = "0.1.5" }
```

Nothing else carries a version. The per-crate manifests all say `version.workspace = true`.

> These were once repeated across six manifests, and the very first bump missed one —
> the `delhi-gui` line, whose `optional = true` sat after the version and slipped past the
> pattern that caught the other five. It resolved anyway, because `^0.1.0` accepts `0.1.1`,
> which is exactly why nobody would have noticed until a publish. Keep them in one block.

Then refresh the lockfile and confirm nothing was left behind:

```bash
cargo build --workspace
grep -rn '0\.1\.4' Cargo.toml crates/*/Cargo.toml    # should print nothing
```

### 3. Write the changelog

Add a section to `CHANGELOG.md` under `## <version> — <date>`, grouped into **Added /
Changed / Fixed / Internal**. Say what changed for a *user*, and why, not which files moved.

### 4. Run the whole gate locally

CI runs all of this, but finding out here is faster:

```bash
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
cargo build -p delhi --no-default-features        # the zero-dependency path
cargo +1.78 check --workspace                     # MSRV
DELHI_BIN="$PWD/target/release/delhi" python python/test_delhi.py
```

### 5. Pre-flight the publish

**Do this. It is the only check that catches a crate which publishes cleanly and then
fails to compile for whoever installs it.**

```bash
cargo publish --workspace --dry-run --locked
```

It builds every crate from its own packaged tarball against a temporary registry holding
the others — the same thing crates.io will do, without the irreversible part.

### 6. Tag and push

```bash
git add -A && git commit -m "release: 0.1.5"
git push origin master
git tag -a v0.1.5 -m "delhi 0.1.5

<one or two lines>. See CHANGELOG.md."
git push origin v0.1.5
```

### 7. Watch it

```bash
gh run list --limit 2
gh run view <id> --json jobs -q '.jobs[] | .name + ": " + (.conclusion // .status)'
```

The `release` workflow builds four targets, publishes the GitHub release, then publishes to
crates.io. Takes about five minutes.

### 8. Verify like a stranger

Not from this repository:

```bash
cargo install delhi --force
delhi --version
cd /tmp && mkdir t && cd t
delhi eval <(curl -sL https://raw.githubusercontent.com/vasanthsarathy/delhi/master/examples/coin_lie.delhi) -f "B[carol] h"
```

---

## Things that will bite you

### Publishing is irreversible

A version on crates.io can be **yanked but never deleted or replaced**, and a crate *name*
can never be reused. There is no undo. This is why step 5 exists.

### A crate may not read files outside its own directory

`cargo package` cannot carry files from beyond the package root. `delhi-gui` once did
`include_str!("../../../examples/…")` — it would have published cleanly and then failed to
compile for **everyone who installed it**, and no test in the repository would have noticed,
because inside the repository that path is always valid.

The examples are inlined into `crates/delhi-gui/src/builtin.rs` instead. **If you change
anything in `examples/`, regenerate it:**

```bash
sh tools/bundle-examples.sh
```

The test `bundled_examples_match_the_repository_byte_for_byte` fails until you do, so
forgetting is a red suite rather than a binary quietly serving stale text.

The same rule applies to `#[cfg(test)]` blocks in `src/`, which ship in the tarball. Before
adding an `include_str!`, ask whether the path leaves the crate.

### crates.io rate-limits *new crate names*

A first publish of several crates runs straight into it. 0.1.4 stopped with one of six left
to upload. Publishing new *versions* of existing crates is not affected, so this only bites
on a first publish or when adding a crate.

### `cargo publish --workspace` is not resumable

If it stops partway, a re-run aborts with `already exists on crates.io index` for every
crate that did land, and there is no flag to continue. Both the release job and
`.github/workflows/publish.yml` therefore go crate by crate and treat "already exists" as
success.

**To finish or repair a stuck publish**, without cutting a new tag:

```bash
gh workflow run publish.yml --ref master
gh workflow run publish.yml --ref master -f dry_run=true     # verify only
```

### The first publish needed two account-level things

Neither is a repository setting, and both fail with a clear message:

- **A verified email on crates.io** — https://crates.io/settings/profile
- **`CARGO_REGISTRY_TOKEN`** as a repository secret, scoped to *publish-new* and
  *publish-update*. Without it the publish step skips with a warning rather than failing.

### MSRV deliberately skips `--all-targets`

The MSRV job runs `cargo check --workspace`, not `--all-targets`. The test-only dependency
tree pulls a `getrandom` needing edition 2024, which would drag the stated floor from 1.78
to 1.85 for a dependency no consumer ever builds. Do not "fix" this by adding the flag —
raise `rust-version` only with a CI job proving the new floor.

---

## What CI does on a tag

| Workflow | Trigger | Does |
|---|---|---|
| `ci.yml` | every push and PR | fmt, clippy, tests, no-default-features build, MSRV, docs, an installed-binary smoke test, the Python wrapper — on Linux, macOS and Windows |
| `release.yml` | tag `v*` | builds 4 platform archives, publishes the GitHub release with `SHA256SUMS`, then publishes to crates.io |
| `publish.yml` | manual | publishes any crate not yet up, for finishing a stuck release |
| `docs.yml` | push to master | builds the landing page and book, checks every internal link, deploys to GitHub Pages |

Release archives carry the binary, `examples/`, `python/delhi.py`, `README.md`,
`CHANGELOG.md` and both licences.
