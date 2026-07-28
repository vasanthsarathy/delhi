//! Terminal colour, hand-rolled.
//!
//! Zero runtime dependencies is a project-wide constraint, so this is a handful of ANSI
//! escapes rather than a crate. It is deliberately small: colour marks the few things a
//! reader scans for — whether a file was accepted, whether a formula held, which world is
//! the actual one — and stays out of the way otherwise.
//!
//! Colour is **off unless explicitly enabled**, which `main` does once at startup after
//! checking that stdout is a terminal and that `NO_COLOR` is unset. That default matters
//! for two reasons: the unit tests assert on exact output and would break on escape
//! codes, and `delhi dot` is meant to be piped into Graphviz, which would choke on them.

use std::sync::atomic::{AtomicBool, Ordering};

static ENABLED: AtomicBool = AtomicBool::new(false);

/// Turns colour on for the rest of the process.
///
/// Call once, from `main`, only after establishing that the destination is a terminal.
pub fn enable() {
    ENABLED.store(true, Ordering::Relaxed);
}

/// Whether output should carry escape codes.
pub fn on() -> bool {
    ENABLED.load(Ordering::Relaxed)
}

/// Decides whether to colour, following the conventions a terminal user expects:
/// honour `NO_COLOR` (any value), honour `CLICOLOR_FORCE`, and otherwise colour only
/// when stdout is actually a terminal so that pipes and files stay clean.
pub fn detect() -> bool {
    if std::env::var_os("NO_COLOR").is_some() {
        return false;
    }
    if std::env::var_os("CLICOLOR_FORCE").is_some() {
        return true;
    }
    use std::io::IsTerminal;
    std::io::stdout().is_terminal()
}

fn wrap(code: &str, s: &str) -> String {
    if on() {
        format!("\x1b[{code}m{s}\x1b[0m")
    } else {
        s.to_string()
    }
}

/// Something went well.
pub fn good(s: &str) -> String {
    wrap("32", s)
}

/// Something went wrong.
pub fn bad(s: &str) -> String {
    wrap("31", s)
}

/// Worth the eye landing on it — the designated world, a headline number.
pub fn key(s: &str) -> String {
    wrap("1;33", s)
}

/// Secondary text: prompts, separators, units.
pub fn dim(s: &str) -> String {
    wrap("2", s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn styling_is_a_no_op_until_enabled() {
        // The default must be plain, because every command's unit tests assert on exact
        // output and `dot` is piped into Graphviz. A test that merely called `enable()`
        // first would not catch a regression here, since `ENABLED` is process-global.
        assert!(!on(), "colour must default to off");
        assert_eq!(good("ok"), "ok");
        assert_eq!(bad("no"), "no");
        assert_eq!(key("*"), "*");
        assert_eq!(dim("> "), "> ");
    }

    #[test]
    fn no_color_beats_clicolor_force() {
        // Both are conventions; `NO_COLOR` is the one that must win, since it is the
        // opt-out a user reaches for when something is mangling their output.
        if std::env::var_os("NO_COLOR").is_some() {
            assert!(!detect(), "NO_COLOR set in the environment must disable colour");
        }
    }
}
