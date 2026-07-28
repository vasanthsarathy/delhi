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

/// `delhi step` — apply a sequence of actions and print the resulting state.
pub fn cmd_step(src: &str, actions: &[String], out: &mut String) -> i32 {
    let Some(mut p) = open(src, out) else {
        return 1;
    };
    let n_agents = p.sig.n_agents();
    let mut state = p.state.clone();

    for name in actions {
        let Some(g) = p.actions.iter().find(|a| &a.name == name) else {
            let mut names: Vec<&str> = p.actions.iter().map(|a| a.name.as_str()).collect();
            names.sort_unstable();
            let _ = writeln!(out, "no action `{name}`; available: {}", names.join(", "));
            return 2;
        };
        let def = g.def.clone();
        let model = delhi_mb::build(&def, &mut p.store, n_agents);
        match state.apply(&p.store, &model) {
            Some(next) => {
                state = next;
                let _ = writeln!(out, "applied {name}");
            }
            None => {
                let _ = writeln!(out, "`{name}` is not applicable in the current state");
                return 1;
            }
        }
    }

    let _ = write!(out, "{}", print_state(&state, &p.sig));
    0
}

/// `delhi dot` — Graphviz for the initial state.
///
/// One node per world, doubled for the designated one, labelled with the atoms true
/// there. One edge per agent relation, reflexive edges omitted.
pub fn cmd_dot(src: &str, out: &mut String) -> i32 {
    let Some(p) = open(src, out) else {
        return 1;
    };
    let m = &p.state.model;
    let _ = writeln!(out, "digraph delhi {{");
    let _ = writeln!(out, "  rankdir=LR;");
    for w in 0..m.n_worlds {
        let facts: Vec<&str> = m.val[w]
            .ones()
            .into_iter()
            .filter(|a| *a < p.sig.n_atoms())
            .map(|a| p.sig.atom_name(a as u32))
            .collect();
        let label = if facts.is_empty() { "∅".to_string() } else { facts.join(",") };
        let peripheries = if w == p.state.designated { " peripheries=2" } else { "" };
        let _ = writeln!(out, "  w{w} [shape=circle label=\"{label}\"{peripheries}];");
    }
    for i in 0..m.n_agents {
        let agent = p.sig.agent_name(i as u32);
        for u in 0..m.n_worlds {
            for v in m.rel[i][u].ones() {
                if u != v {
                    let _ = writeln!(out, "  w{u} -> w{v} [label=\"{agent}\"];");
                }
            }
        }
    }
    let _ = writeln!(out, "}}");
    0
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

    const COIN: &str = r#"
        types{ Actor - Object } objects{ a, b - Actor } agents{ a, b } props{ h }
        initially { h, ?[a] h, B[a] h }
        actions {
            tell() { actor b, announces !h, a observes, b observes }
            look() { actor a, determines h, a observes }
        }
    "#;

    #[test]
    fn step_applies_a_sequence_and_prints_the_result() {
        let (code, out) = run(|o| cmd_step(COIN, &["tell()".to_string()], o));
        assert_eq!(code, 0, "got: {out}");
        assert!(out.contains("state {"), "the resulting state is printed");
    }

    #[test]
    fn step_reports_an_unknown_action_by_name() {
        let (code, out) = run(|o| cmd_step(COIN, &["nosuch()".to_string()], o));
        assert_eq!(code, 2);
        assert!(out.contains("nosuch()"));
        assert!(out.contains("tell()"), "the message should list what IS available");
    }

    #[test]
    fn step_reports_an_inapplicable_action_rather_than_panicking() {
        // `look()` senses h, whose two designated events have exhaustive preconditions,
        // so it always applies. Use a precondition that cannot hold instead.
        let src = r#"
            types{ Actor - Object } objects{ a - Actor } agents{ a } props{ h, q }
            initially { h }
            actions { go() { actor a, pre q, causes h, a observes } }
        "#;
        let (code, out) = run(|o| cmd_step(src, &["go()".to_string()], o));
        assert_eq!(code, 1);
        assert!(out.to_lowercase().contains("not applicable"), "got: {out}");
    }

    #[test]
    fn dot_emits_a_digraph_with_one_node_per_world() {
        let (code, out) = run(|o| cmd_dot(COIN, o));
        assert_eq!(code, 0);
        assert!(out.starts_with("digraph"), "got: {out}");
        assert_eq!(out.matches("shape=").count(), 2, "two worlds, two nodes");
        assert!(out.contains("peripheries=2"), "the designated world is doubled");
        assert!(out.contains("->"), "edges are drawn");
        assert!(out.trim_end().ends_with('}'));
    }

    #[test]
    fn dot_omits_reflexive_edges() {
        // Every world in COIN is reflexively related to itself for both agents (a
        // modal frame condition), so if the filter that skips `u == v` were removed,
        // self-loops like `w0 -> w0` would appear. `starts_with("digraph")` alone
        // would not catch this, so check for the absence of every self-loop directly.
        let (code, out) = run(|o| cmd_dot(COIN, o));
        assert_eq!(code, 0);
        for w in 0..2 {
            assert!(
                !out.contains(&format!("w{w} -> w{w}")),
                "reflexive edges must not be drawn:\n{out}"
            );
        }
    }
}
