//! The subcommands. Each returns an exit code and writes to the provided sink, so the
//! tests can drive them without spawning a process.

use delhi_lang::{print_state, Problem};
use std::fmt::Write;

/// Parses `src`, or writes the diagnostics and returns `None`.
fn open(src: &str, out: &mut String) -> Option<Problem> {
    match Problem::parse(src) {
        Ok(p) => Some(p),
        Err(e) => {
            let _ = write!(out, "{e}");
            None
        }
    }
}

/// `delhi check` — parse, ground, and validate. `0` if the file is accepted.
pub fn cmd_check(src: &str, out: &mut String) -> i32 {
    match open(src, out) {
        None => 1,
        Some(p) => {
            let _ = writeln!(
                out,
                "ok: {} atoms, {} agents, {} ground actions, {} worlds",
                p.sig.n_atoms(),
                p.sig.n_agents(),
                p.actions.len(),
                p.state.model.n_worlds
            );
            0
        }
    }
}

/// `delhi show` — print the initial state in the explicit form.
pub fn cmd_show(src: &str, out: &mut String) -> i32 {
    match open(src, out) {
        None => 1,
        Some(p) => {
            let _ = write!(out, "{}", print_state(&p.state, &p.sig));
            0
        }
    }
}

/// `delhi eval` — evaluate a formula in the initial state.
///
/// Exit code is `0` when the formula holds, `1` when it does not, and `2` when the
/// formula itself is malformed — so shell scripts can branch on the answer.
pub fn cmd_eval(src: &str, formula: &str, out: &mut String) -> i32 {
    let Some(mut p) = open(src, out) else {
        return 1;
    };
    match parse_query(&mut p, formula) {
        Err(e) => {
            let _ = write!(out, "{e}");
            2
        }
        Ok(f) => {
            let holds = p.state.entails(&p.store, f);
            let _ = writeln!(out, "{holds}");
            i32::from(!holds)
        }
    }
}

/// Parses a formula written on the command line against an already-checked problem.
///
/// The query shares the problem's signature and constants, so it can name any
/// declared proposition or agent.
pub fn parse_query(p: &mut Problem, text: &str) -> Result<delhi_syntax::FormulaId, String> {
    // A query is lowered exactly like a `goal`, by wrapping it in a throwaway file
    // whose other sections are copied from the problem. Rather than re-serialise the
    // whole problem, reuse the pieces already checked.
    let mut diags = delhi_lang::Diagnostics::default();
    let toks = delhi_lang::lex(text, &mut diags);
    let mut parser = delhi_lang::Parser::new(&toks);
    let expr = parser.parse_expr(&mut diags);
    let f = delhi_lang::lower_formula(
        &expr,
        &p.sig,
        &p.consts,
        &delhi_lang::Bindings::default(),
        &mut p.store,
        &mut diags,
    );
    if diags.is_empty() {
        Ok(f)
    } else {
        Err(diags.render(text))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOOD: &str = r#"
        types{ Actor - Object } objects{ a - Actor } agents{ a } props{ h }
        initially { h, ?[a] h, B[a] h }
        actions { look() { actor a, determines h, a observes } }
    "#;

    const BAD: &str = "types{} objects{} agents{ ghost } props{} initially{} actions{}";

    fn run(f: impl FnOnce(&mut String) -> i32) -> (i32, String) {
        let mut out = String::new();
        let code = f(&mut out);
        (code, out)
    }

    #[test]
    fn check_accepts_a_valid_file() {
        let (code, out) = run(|o| cmd_check(GOOD, o));
        assert_eq!(code, 0);
        assert!(out.to_lowercase().contains("ok"), "expected a success line, got: {out}");
    }

    #[test]
    fn check_rejects_and_explains() {
        let (code, out) = run(|o| cmd_check(BAD, o));
        assert_eq!(code, 1);
        assert!(out.contains("ghost"), "the diagnostic must name the problem");
    }

    #[test]
    fn show_emits_the_explicit_form() {
        let (code, out) = run(|o| cmd_show(GOOD, o));
        assert_eq!(code, 0);
        assert!(out.starts_with("state {"), "got: {out}");
        assert!(out.contains('*'), "the designated world is marked");
    }

    #[test]
    fn eval_reports_true_and_false_with_different_codes() {
        let (code, out) = run(|o| cmd_eval(GOOD, "K[a] h", o));
        assert_eq!(code, 1, "a is uncertain, so K[a]h is false");
        assert!(out.contains("false"));

        let (code, out) = run(|o| cmd_eval(GOOD, "B[a] h", o));
        assert_eq!(code, 0, "a believes h");
        assert!(out.contains("true"));
    }

    #[test]
    fn eval_reports_a_malformed_formula_rather_than_panicking() {
        let (code, out) = run(|o| cmd_eval(GOOD, "K[nobody] h", o));
        assert_eq!(code, 2);
        assert!(out.contains("nobody"));
    }
}
