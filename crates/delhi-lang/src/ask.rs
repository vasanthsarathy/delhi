//! Enumerating the formulas of a given shape that hold, rather than checking one.
//!
//! Evaluating a formula answers "is this true?". This answers "which of these are true?"
//! — what does alice believe, what is she ignorant of, what holds two levels down. The
//! difference matters when you are debugging a scenario and do not yet know what to ask.
//!
//! # What is enumerated
//!
//! Not "all formulas": there are infinitely many, since conjunction alone generates
//! without bound. The candidates are **modal literals** — a literal under some sequence
//! of `K`/`B` modalities, as in `B[alice] K[carol] !h`. That is a real and well-studied
//! restriction: it is the representation Muise et al.'s PDKB planner is built on, chosen
//! there for the same reason it is chosen here, that the set is finite and its size is
//! predictable from the signature and the depth.
//!
//! The count is `Σ_{k≤d} (2·agents)^k · 2·atoms`, which grows fast — three agents and
//! nine atoms reach ~3,900 at depth 3 — so [`MAX_CANDIDATES`] bounds it and the caller
//! is told when the bound bit.
//!
//! # The hole
//!
//! The caller supplies a pattern containing `_`, and each candidate is substituted for
//! it: `B[alice] _` asks what alice believes, `?[alice] _` what she is ignorant of,
//! `K[bob] K[alice] _` what bob knows alice knows. Substitution is textual and the
//! candidate is parenthesised, so the pattern needs no special parsing and any operator
//! the language has — including sugar — works in it for free.

use crate::lower_formula::{lower_formula, Bindings};
use crate::{Diagnostics, Parser, Problem, Sig};
use delhi_mb::State;

/// Ceiling on candidates, so a careless depth cannot hang the tool.
///
/// Each candidate costs a parse, a lowering and an evaluation — tens of microseconds —
/// so this is a fraction of a second, chosen to stay interactive rather than to be the
/// largest survivable number.
pub const MAX_CANDIDATES: usize = 20_000;

/// The placeholder a pattern must contain.
pub const HOLE: &str = "_";

/// What an enumeration found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Answer {
    /// The instantiated formulas that hold, in enumeration order: shallow before deep.
    pub matches: Vec<String>,
    /// How many candidates were tried.
    pub considered: usize,
    /// Whether [`MAX_CANDIDATES`] cut the enumeration short. When true, `matches` is a
    /// prefix of the real answer, not the whole of it.
    pub truncated: bool,
}

/// Every modal literal of depth at most `depth`, shallowest first.
///
/// Depth 0 is the bare literals. Each further level prefixes `K[i]` and `B[i]` for every
/// agent. Shallowest-first ordering matters for readability — the short, usually more
/// interesting answers come out on top — and it is what makes truncation a sensible
/// prefix rather than an arbitrary sample.
pub fn modal_literals(sig: &Sig, depth: usize) -> Vec<String> {
    let mut level: Vec<String> = Vec::new();
    for a in 0..sig.n_atoms() {
        let name = sig.atom_name(a as u32);
        level.push(name.to_string());
        level.push(format!("!{name}"));
    }
    let mut all = level.clone();
    for _ in 0..depth {
        let mut next = Vec::new();
        for inner in &level {
            for i in 0..sig.n_agents() {
                let who = sig.agent_name(i as u32);
                next.push(format!("K[{who}] {inner}"));
                next.push(format!("B[{who}] {inner}"));
            }
            if all.len() + next.len() > MAX_CANDIDATES {
                break;
            }
        }
        all.extend(next.iter().cloned());
        level = next;
        if all.len() > MAX_CANDIDATES {
            break;
        }
    }
    all.truncate(MAX_CANDIDATES);
    all
}

/// Substitutes one candidate into the pattern, parenthesised so precedence cannot bite.
fn instantiate(pattern: &str, candidate: &str) -> String {
    pattern.replace(HOLE, &format!("({candidate})"))
}

/// Flips the polarity of a candidate's innermost literal, leaving its modalities alone.
///
/// The literal is whatever follows the last `] `, since every modality ends in one.
fn complement(candidate: &str) -> String {
    let (prefix, lit) = match candidate.rfind("] ") {
        Some(i) => candidate.split_at(i + 2),
        None => ("", candidate),
    };
    match lit.strip_prefix('!') {
        Some(rest) => format!("{prefix}{rest}"),
        None => format!("{prefix}!{lit}"),
    }
}

/// Lowers `text` against an already-checked problem and evaluates it at `state`.
///
/// Returns `None` when the text does not lower, which during enumeration means the
/// *pattern* is malformed — the candidates are generated from the signature and always
/// lower — so the caller reports it once rather than once per candidate.
fn holds(p: &mut Problem, state: &State, text: &str) -> Option<bool> {
    let mut diags = Diagnostics::default();
    let toks = crate::lex(text, &mut diags);
    let expr = Parser::new(&toks).parse_expr(&mut diags);
    let f = lower_formula(&expr, &p.sig, &p.consts, &Bindings::default(), &mut p.store, &mut diags);
    if diags.is_empty() {
        Some(state.entails(&p.store, f))
    } else {
        None
    }
}

/// Enumerates the instantiations of `pattern` that hold at `state`.
///
/// `pattern` must contain [`HOLE`]. Returns the rendered diagnostics if the pattern does
/// not lower once a candidate is substituted — checked on the first candidate, so a typo
/// is reported immediately rather than after thousands of silent failures.
pub fn ask(
    p: &mut Problem,
    state: &State,
    pattern: &str,
    depth: usize,
) -> Result<Answer, String> {
    if !pattern.contains(HOLE) {
        return Err(format!(
            "the pattern needs a `{HOLE}` to fill — try `B[agent] {HOLE}`, or `{HOLE}` on its own"
        ));
    }
    let candidates = modal_literals(&p.sig, depth);
    if candidates.is_empty() {
        return Ok(Answer { matches: Vec::new(), considered: 0, truncated: false });
    }

    // Check the pattern once, on the first candidate. A pattern that does not lower is
    // the user's mistake and must be reported as itself, not as an empty result.
    let first = instantiate(pattern, &candidates[0]);
    if holds(p, state, &first).is_none() {
        let mut diags = Diagnostics::default();
        let toks = crate::lex(&first, &mut diags);
        let expr = Parser::new(&toks).parse_expr(&mut diags);
        let _ = lower_formula(
            &expr,
            &p.sig,
            &p.consts,
            &Bindings::default(),
            &mut p.store,
            &mut diags,
        );
        return Err(diags.render(&first));
    }

    let mut hit: std::collections::HashSet<&str> = std::collections::HashSet::new();
    let mut order: Vec<&String> = Vec::new();
    for c in &candidates {
        if holds(p, state, &instantiate(pattern, c)) == Some(true) {
            hit.insert(c.as_str());
            order.push(c);
        }
    }

    // Some patterns cannot see polarity: being ignorant of `h` *is* being ignorant of
    // `!h`, and likewise for `Kw`/`Bw`. Those return both twins, which reads as two
    // findings when there is one. Where both a candidate and its complement matched,
    // only the positive form is kept.
    //
    // The rule fires exactly when the pattern is polarity-blind, and never for a
    // consistent attitude — belief is KD, so `B[a] h` and `B[a] !h` cannot both hold.
    let matches = order
        .into_iter()
        .filter(|c| {
            let is_negative = complement(c).len() < c.len();
            !(is_negative && hit.contains(complement(c).as_str()))
        })
        .map(|c| instantiate(pattern, c))
        .collect();

    Ok(Answer {
        matches,
        considered: candidates.len(),
        truncated: candidates.len() >= MAX_CANDIDATES,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const COIN: &str = r#"
        types{ Actor - Object } objects{ a, b - Actor } agents{ a, b } props{ h }
        initially { h, ?[a] h, B[a] h }
        actions {}
    "#;

    fn problem(src: &str) -> (Problem, State) {
        let p = Problem::parse(src).unwrap_or_else(|e| panic!("{e}"));
        let s = p.state.clone();
        (p, s)
    }

    #[test]
    fn candidate_count_follows_the_signature_and_the_depth() {
        // 1 atom, 2 agents: 2 literals at depth 0, then x4 per level (2 ops x 2 agents).
        // Getting this wrong silently changes what "up to depth d" means.
        let (p, _) = problem(COIN);
        assert_eq!(modal_literals(&p.sig, 0).len(), 2);
        assert_eq!(modal_literals(&p.sig, 1).len(), 2 + 8);
        assert_eq!(modal_literals(&p.sig, 2).len(), 2 + 8 + 32);
    }

    #[test]
    fn candidates_are_ordered_shallowest_first() {
        // Truncation keeps a prefix, so the ordering is what makes a cut-short answer
        // useful rather than arbitrary.
        let (p, _) = problem(COIN);
        let c = modal_literals(&p.sig, 2);
        let depth_of = |s: &str| s.matches('[').count();
        let depths: Vec<usize> = c.iter().map(|s| depth_of(s)).collect();
        assert!(depths.windows(2).all(|w| w[0] <= w[1]), "not shallowest-first: {depths:?}");
    }

    #[test]
    fn asking_what_an_agent_believes_separates_belief_from_knowledge() {
        // `a` believes h without knowing it; `b` knows it. Both believe it, so the
        // belief query must return h for both while the knowledge query returns it
        // only for b — that contrast is the whole point of the tool.
        let (mut p, s) = problem(COIN);
        let believes_a = ask(&mut p, &s, "B[a] _", 0).expect("valid pattern");
        assert_eq!(believes_a.matches, vec!["B[a] (h)"]);

        let knows_a = ask(&mut p, &s, "K[a] _", 0).expect("valid pattern");
        assert!(knows_a.matches.is_empty(), "a knows nothing here: {:?}", knows_a.matches);

        let knows_b = ask(&mut p, &s, "K[b] _", 0).expect("valid pattern");
        assert_eq!(knows_b.matches, vec!["K[b] (h)"]);
    }

    #[test]
    fn asking_what_an_agent_is_ignorant_of_reports_the_atom_once() {
        // Ignorance is symmetric — being ignorant of h is being ignorant of !h — so a
        // naive enumeration returns both polarities and reads as two findings when
        // there is one. The positive literal is the one worth showing.
        let (mut p, s) = problem(COIN);
        let ignorant = ask(&mut p, &s, "?[a] _", 0).expect("valid pattern");
        assert!(ignorant.matches.iter().any(|m| m.contains("(h)")), "got {:?}", ignorant.matches);
        assert!(!ignorant.matches.iter().any(|m| m.contains("(!h)")),
                "the negated twin is redundant: {:?}", ignorant.matches);
    }

    #[test]
    fn depth_reaches_nested_attitudes_that_depth_zero_cannot() {
        // `b` knows h and knows that a is unsure, so `K[b] B[a] h` holds at depth 1 but
        // no depth-0 query can express it.
        let (mut p, s) = problem(COIN);
        let shallow = ask(&mut p, &s, "K[b] _", 0).expect("ok");
        assert!(!shallow.matches.iter().any(|m| m.contains("B[a]")));

        let deep = ask(&mut p, &s, "K[b] _", 1).expect("ok");
        assert!(deep.matches.iter().any(|m| m == "K[b] (B[a] h)"), "got {:?}", deep.matches);
        assert!(deep.considered > shallow.considered);
    }

    #[test]
    fn a_bare_hole_enumerates_what_simply_holds() {
        let (mut p, s) = problem(COIN);
        let a = ask(&mut p, &s, "_", 0).expect("ok");
        assert_eq!(a.matches, vec!["(h)"], "h is true, !h is not");
    }

    #[test]
    fn a_pattern_without_a_hole_is_rejected_as_such() {
        // Otherwise it would evaluate one fixed formula thousands of times and return
        // either everything or nothing, which looks like a broken query rather than a
        // misspelled one.
        let (mut p, s) = problem(COIN);
        let e = ask(&mut p, &s, "B[a] h", 0).unwrap_err();
        assert!(e.contains('_'), "the error should say what is missing: {e}");
    }

    #[test]
    fn a_malformed_pattern_reports_its_own_diagnostic() {
        let (mut p, s) = problem(COIN);
        let e = ask(&mut p, &s, "B[nobody] _", 0).unwrap_err();
        assert!(e.contains("nobody"), "got {e}");
    }

    #[test]
    fn the_candidate_bound_is_honoured_and_declared() {
        // A careless depth must degrade to a truncated answer, not to a hang. The flag
        // is what stops a partial answer being read as a complete one.
        let src = r#"
            types{ Actor - Object } objects{ a, b, c - Actor } agents{ a, b, c }
            props{ p, q, r, s }
            initially { p } actions {}
        "#;
        let (mut p, st) = problem(src);
        let a = ask(&mut p, &st, "_", 9).expect("ok");
        assert!(a.truncated, "depth 9 over 3 agents must hit the bound");
        assert!(a.considered <= MAX_CANDIDATES);
    }
}
