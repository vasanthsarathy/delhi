#!/bin/sh
# Regenerates crates/delhi-gui/src/builtin.rs from examples/.
#
# Run after adding, removing or editing anything in examples/. The test
# `bundled_examples_match_the_repository_byte_for_byte` in delhi-gui fails until you do,
# so forgetting is a red suite rather than a binary that quietly ships stale text.
#
#     sh tools/bundle-examples.sh
set -eu

cd "$(dirname "$0")/.."
OUT=crates/delhi-gui/src/builtin.rs

# The sources are inlined with r#"…"#, which breaks if an example ever contains `"#`.
# Checked rather than assumed: the failure would otherwise be a confusing parse error in
# generated code.
if grep -l '"#' examples/*.delhi 2>/dev/null; then
    echo "error: the files above contain \`\"#\`, which would terminate the raw string" >&2
    exit 1
fi

{
    cat <<'HEADER'
//! The examples that ship inside the binary.
//!
//! A downloaded or `cargo install`ed `delhi` has no repository beside it, so without these
//! the UI opens on an empty directory and a new user has nowhere to learn what a `.delhi`
//! file looks like. They are read-only: the copy in the binary is the one that is served,
//! and saving one writes a new file into the served directory under whatever name the user
//! chooses.
//!
//! **Generated — do not edit by hand.** Regenerate with `tools/bundle-examples.sh`.
//!
//! The sources are inlined rather than `include_str!`d from `examples/`, which sits
//! outside this crate. `cargo package` cannot carry files from beyond the package root, so
//! the path form produced a crate that published cleanly and then failed to compile for
//! anyone who installed it. `lib.rs` asserts this file matches `examples/` byte for byte,
//! so the copy cannot drift.

/// Bundled examples as `(file name, source)`, sorted by name.
pub const BUILTIN: &[(&str, &str)] = &[
HEADER
    for f in $(ls examples/*.delhi | sort); do
        n=$(basename "$f")
        printf '    (\n        "%s",\n        r#"' "$n"
        cat "$f"
        printf '"#,\n    ),\n'
    done
    echo '];'
} > "$OUT"

echo "wrote $OUT from $(ls examples/*.delhi | wc -l) examples"
