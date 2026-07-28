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

use crate::ast::Expr;
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

/// Replaces every hole in `pattern` with `filler`, structurally.
///
/// Substitution is on the tree, not the text. Textual replacement was wrong twice over:
/// it corrupted any identifier containing an underscore — `at_park` became
/// `at(candidate)park` — and it needed defensive parentheses to keep precedence, which
/// structure gives for free.
///
/// Every hole receives the same filler. One hole is the common case; repeating it asks
/// "where does this same formula appear twice", as in `B[a] _ & !B[b] _`.
fn fill(pattern: &Expr, filler: &Expr) -> Expr {
    match pattern {
        Expr::Hole(_) => filler.clone(),
        Expr::True(_) | Expr::False(_) | Expr::Atom(_) => pattern.clone(),
        Expr::Not(a, s) => Expr::Not(Box::new(fill(a, filler)), *s),
        Expr::And(a, b, s) => {
            Expr::And(Box::new(fill(a, filler)), Box::new(fill(b, filler)), *s)
        }
        Expr::Or(a, b, s) => Expr::Or(Box::new(fill(a, filler)), Box::new(fill(b, filler)), *s),
        Expr::Implies(a, b, s) => {
            Expr::Implies(Box::new(fill(a, filler)), Box::new(fill(b, filler)), *s)
        }
        Expr::Modality { op, agents, cond, body, span } => Expr::Modality {
            op: op.clone(),
            agents: agents.clone(),
            cond: cond.as_ref().map(|c| Box::new(fill(c, filler))),
            body: Box::new(fill(body, filler)),
            span: *span,
        },
    }
}

/// Whether an expression contains a hole.
fn has_hole(e: &Expr) -> bool {
    match e {
        Expr::Hole(_) => true,
        Expr::True(_) | Expr::False(_) | Expr::Atom(_) => false,
        Expr::Not(a, _) => has_hole(a),
        Expr::And(a, b, _) | Expr::Or(a, b, _) | Expr::Implies(a, b, _) => {
            has_hole(a) || has_hole(b)
        }
        Expr::Modality { cond, body, .. } => {
            cond.as_ref().is_some_and(|c| has_hole(c)) || has_hole(body)
        }
    }
}

/// Parses one formula, returning it with any diagnostics raised.
fn parse(text: &str) -> (Expr, Diagnostics) {
    let mut diags = Diagnostics::default();
    let toks = crate::lex(text, &mut diags);
    let expr = Parser::new(&toks).parse_expr(&mut diags);
    (expr, diags)
}

/// Byte ranges of the `_` tokens in `pattern`, ascending.
fn hole_spans(pattern: &str) -> Vec<(usize, usize)> {
    let mut diags = Diagnostics::default();
    crate::lex(pattern, &mut diags)
        .iter()
        .filter(|t| t.tok == crate::Tok::Hole)
        .map(|t| (t.span.start, t.span.end))
        .collect()
}

/// Renders a filled pattern for display, splicing at the holes' byte ranges.
///
/// Splicing at spans rather than replacing the text `_` is the same correctness point as
/// [`fill`]: a pattern like `_ & at_park` has one hole, not two, and only the lexer knows
/// which underscore is which.
fn render(pattern: &str, holes: &[(usize, usize)], candidate: &str) -> String {
    let mut out = String::with_capacity(pattern.len() + candidate.len());
    let mut last = 0;
    for &(start, end) in holes {
        out.push_str(&pattern[last..start]);
        out.push('(');
        out.push_str(candidate);
        out.push(')');
        last = end;
    }
    out.push_str(&pattern[last..]);
    out
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

/// Enumerates the instantiations of `pattern` that hold at `state`.
///
/// `pattern` must contain [`HOLE`]. Returns rendered diagnostics if the pattern does not
/// parse or does not lower once filled — checked on the first candidate, so a typo is
/// reported as itself rather than as an empty answer.
pub fn ask(
    p: &mut Problem,
    state: &State,
    pattern: &str,
    depth: usize,
) -> Result<Answer, String> {
    let (pat, diags) = parse(pattern);
    if !diags.is_empty() {
        return Err(diags.render(pattern));
    }
    if !has_hole(&pat) {
        return Err(format!(
            "the pattern needs a `{HOLE}` to fill — try `B[agent] {HOLE}`, or `{HOLE}` on its own"
        ));
    }
    let holes = hole_spans(pattern);
    let candidates = modal_literals(&p.sig, depth);
    if candidates.is_empty() {
        return Ok(Answer { matches: Vec::new(), considered: 0, truncated: false });
    }

    // Parse each candidate once, not once per pattern: the pattern is filled with the
    // parsed tree, so a candidate's text is scanned a single time however many holes
    // the pattern has.
    let parsed: Vec<Expr> = candidates.iter().map(|c| parse(c).0).collect();

    // Check the filled pattern once. A pattern naming an undeclared agent is the user's
    // mistake and must surface as its own diagnostic, not as "nothing matched".
    let mut probe = Diagnostics::default();
    let first = fill(&pat, &parsed[0]);
    let _ = lower_formula(
        &first,
        &p.sig,
        &p.consts,
        &Bindings::default(),
        &mut p.store,
        &mut probe,
    );
    if !probe.is_empty() {
        return Err(probe.render(pattern));
    }

    let mut hit: std::collections::HashSet<&str> = std::collections::HashSet::new();
    let mut order: Vec<&String> = Vec::new();
    for (c, tree) in candidates.iter().zip(&parsed) {
        let mut quiet = Diagnostics::default();
        let f = lower_formula(
            &fill(&pat, tree),
            &p.sig,
            &p.consts,
            &Bindings::default(),
            &mut p.store,
            &mut quiet,
        );
        if quiet.is_empty() && state.entails(&p.store, f) {
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
        // Presentation only — the answer was decided on the trees. Parenthesised so the
        // printed form reparses to exactly what was evaluated.
        .map(|c| render(pattern, &holes, c))
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
    fn an_underscore_inside_an_identifier_is_not_a_hole() {
        // The bug that forced the hole to become a real token. Under textual
        // substitution `_ & at_park` had *two* holes as far as `str::replace` was
        // concerned, and the second one tore an atom in half: `at(cand)park`.
        let src = r#"
            types{ Actor - Object } objects{ a - Actor } agents{ a }
            props{ at_park, mary_home }
            initially { at_park }
            actions {}
        "#;
        let (mut p, s) = problem(src);
        let a = ask(&mut p, &s, "_ & at_park", 0).expect("the pattern is valid");
        assert!(
            a.matches.iter().any(|m| m == "(at_park) & at_park"),
            "the atom must survive intact: {:?}",
            a.matches
        );
        assert!(
            !a.matches.iter().any(|m| m.contains("at(")),
            "no match may contain a torn identifier: {:?}",
            a.matches
        );
        // And the candidates themselves are the underscored atoms, unmangled.
        let c = modal_literals(&p.sig, 0);
        assert!(c.contains(&"at_park".to_string()) && c.contains(&"!mary_home".to_string()));
    }

    #[test]
    fn every_hole_in_a_pattern_takes_the_same_filler() {
        // Repeating the hole asks "where does this same formula appear twice". Both
        // occurrences must receive the same candidate, or the question is meaningless.
        let (mut p, s) = problem(COIN);
        let a = ask(&mut p, &s, "_ & _", 0).expect("valid");
        assert_eq!(a.matches, vec!["(h) & (h)"], "got {:?}", a.matches);

        // `a` believes h and `b` knows it, so this holds for h and for nothing else.
        let both = ask(&mut p, &s, "B[a] _ & K[b] _", 0).expect("valid");
        assert_eq!(both.matches, vec!["B[a] (h) & K[b] (h)"], "got {:?}", both.matches);
    }

    #[test]
    fn substitution_is_structural_so_precedence_cannot_bite() {
        // Filling by tree means `!_` negates the candidate, whatever it is. Under a
        // careless textual splice `!_` with candidate `K[a] h` could read as `(!K[a]) h`.
        let (mut p, s) = problem(COIN);
        let a = ask(&mut p, &s, "!_", 1).expect("valid");
        assert!(a.matches.iter().any(|m| m == "!(K[a] h)"), "got {:?}", a.matches);
        assert!(!a.matches.iter().any(|m| m == "!(B[a] h)"), "a does believe h: {:?}", a.matches);
    }

    #[test]
    fn a_hole_written_in_a_file_is_rejected_with_a_diagnostic() {
        // A hole means nothing outside a query. Lowering must say so rather than
        // quietly treating it as false, which would make a goal silently unsatisfiable.
        let e = Problem::parse(
            r#"types{} objects{} agents{} props{ h } initially{ h } goal { _ } actions{}"#,
        )
        .unwrap_err();
        assert!(e.contains("query hole"), "got {e}");
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
