# Install

`delhi` is a single self-contained binary. Pick whichever route asks least of you.

## From crates.io

Needs a Rust toolchain ([rustup.rs](https://rustup.rs)), and compiles in a couple of
minutes. Installs into `~/.cargo/bin`, which rustup already puts on your `PATH`.

```bash
cargo install delhi            # install
cargo install delhi --force    # update to the newest release
```

`--force` is how you update: without it, cargo declines to overwrite an existing install.

## A prebuilt binary

No Rust needed. Download the archive for your platform from
[Releases](https://github.com/vasanthsarathy/delhi/releases), unpack it, and put `delhi`
somewhere on your `PATH`. Each archive carries the binary, the examples, the Python
wrapper, and both licences, with `SHA256SUMS` published beside it.

Or let a script do it — it verifies the checksum and unpacks to `~/.local/bin`
(`%LOCALAPPDATA%\delhi\bin` on Windows). Neither script edits your shell profile; each
reports whether the directory is on `PATH` and leaves the change to you.

```bash
curl -fsSL https://raw.githubusercontent.com/vasanthsarathy/delhi/master/install.sh | sh
```
```powershell
irm https://raw.githubusercontent.com/vasanthsarathy/delhi/master/install.ps1 | iex
```

## From source

```bash
git clone https://github.com/vasanthsarathy/delhi && cd delhi
cargo install --path crates/delhi
```

## Check it worked

```bash
delhi --version
delhi --help
```

If you installed a prebuilt archive, the examples are beside the binary. If you installed
with cargo there is no `examples/` directory — but the ten examples are compiled *into* the
binary, so `delhi gui` can still open them from anywhere.

## As a library

The semantics and the language are separate crates, usable without the CLI:

```toml
[dependencies]
delhi-lang = "0.1"     # parse, check and query .delhi source
delhi-mb  = "0.1"      # the model checker itself
```

All the library crates have zero external dependencies. See
[docs.rs/delhi-lang](https://docs.rs/delhi-lang).

## Minimum Rust version

**1.78**, checked in CI. That is a promise about *using* delhi — building its own test
suite wants something newer, because a test-only dependency does.

## A note on version numbers

**crates.io history starts at 0.1.4.** Versions before that exist as GitHub releases and
git tags, but were never published: the binary package was named `delhi-cli` until 0.1.4,
and `delhi-gui` could not be packaged at all — it reached outside its own directory for the
bundled examples, which cargo will not carry into a tarball.

So `cargo install delhi@0.1.2` finds nothing, while the prebuilt archives on
[Releases](https://github.com/vasanthsarathy/delhi/releases) go back to 0.1.0. From 0.1.4
onward the git tag and the published crate are the same thing.
