//! `rules` — Horn clauses over constants, saturated before anything runs.
//!
//! ```text
//! constants { adjacent(hall, study)  adjacent(study, attic) }
//! rules {
//!     reach(?x, ?y) :- adjacent(?x, ?y)
//!     reach(?x, ?z) :- adjacent(?x, ?y), reach(?y, ?z)
//! }
//! ```
//!
//! `reach(hall, attic)` then folds to `true` like any other constant, and nothing about
//! the semantics changes: derived facts join the constant table, so by the time a formula
//! is lowered a derived atom is already a literal truth value and never occupies a bit in
//! any world.
//!
//! # Why constants only
//!
//! The fixpoint is computed once, at parse time, which is only sound because the constant
//! table is static. A rule whose body mentioned a *fluent* would have a derived extension
//! that varies per world and per action, and computing it would mean either a fixpoint
//! per world at evaluation time or maintaining derived atoms through product update —
//! the frame problem again. That is a genuine extension, not an oversight, and it is
//! refused with a message rather than half-supported.
//!
//! # Restrictions, and why each is there
//!
//! * **No negation in bodies.** Rules stay monotone, so the least fixpoint exists and is
//!   reached by iterating to saturation. With negation the program would need
//!   stratifying, and an unstratifiable program would have no single right answer.
//! * **Range restriction.** Every variable in the head must appear in the body, or the
//!   head would derive facts about objects the body never constrained.
//! * **A bound on derivations**, since a rule over `n` objects with `k` variables has
//!   `n^k` groundings per round.

use crate::ast::{Arg, Ast, Term};
use crate::{atom_key, Constants, Diagnostics, Sig};
use std::collections::HashSet;

/// Ceiling on derived facts, so a careless rule cannot exhaust memory at parse time.
pub const MAX_DERIVED: usize = 100_000;

/// Ceiling on the groundings tried for one rule in one round.
const MAX_GROUNDINGS: usize = 2_000_000;

/// The variables appearing in a term, in order of first appearance.
fn vars_of(t: &Term, out: &mut Vec<String>) {
    for a in &t.args {
        if let Arg::Var(v) = a {
            if !out.contains(v) {
                out.push(v.clone());
            }
        }
    }
}

/// Resolves a term's arguments under one assignment, or `None` if an argument is a type
/// name — never meaningful in a rule.
fn ground(t: &Term, assign: &[(String, String)]) -> Option<Vec<String>> {
    let mut out = Vec::with_capacity(t.args.len());
    for a in &t.args {
        match a {
            Arg::Obj(o) => out.push(o.clone()),
            Arg::Var(v) => {
                let o = assign.iter().find(|(name, _)| name == v)?;
                out.push(o.1.clone());
            }
            Arg::Ty(_) => return None,
        }
    }
    Some(out)
}

/// Checks the rules and reports anything that would make the fixpoint ill-defined.
///
/// Returns the set of predicate names the rules derive, which is what tells the caller
/// which names are legitimate in a body even though no `constants` entry mentions them.
fn check(ast: &Ast, sig: &Sig, diags: &mut Diagnostics) -> HashSet<String> {
    let mut derived: HashSet<String> = HashSet::new();
    let mut arity: Vec<(String, usize)> = Vec::new();

    for r in &ast.rules {
        // A fluent's truth varies per world; a parse-time fixpoint cannot speak about it.
        if sig.preds.contains_key(&r.head.pred) {
            diags.push(
                r.span,
                format!(
                    "`{}` is a declared proposition, so it cannot be derived by a rule; \
                     rules run once, over constants",
                    r.head.pred
                ),
            );
            continue;
        }
        match arity.iter().find(|(n, _)| *n == r.head.pred) {
            Some((_, k)) if *k != r.head.args.len() => diags.push(
                r.span,
                format!("`{}` is derived with {k} arguments elsewhere", r.head.pred),
            ),
            None => arity.push((r.head.pred.clone(), r.head.args.len())),
            _ => {}
        }
        derived.insert(r.head.pred.clone());
    }

    for r in &ast.rules {
        if r.body.is_empty() {
            diags.push(r.span, "a rule needs at least one body atom");
        }
        for b in &r.body {
            if sig.preds.contains_key(&b.pred) {
                diags.push(
                    b.span,
                    format!(
                        "`{}` is a declared proposition; a rule body may only mention \
                         constants and other derived predicates",
                        b.pred
                    ),
                );
            }
        }
        // Range restriction: a head variable absent from the body would range over
        // everything, deriving facts the body never justified.
        let mut head_vars = Vec::new();
        vars_of(&r.head, &mut head_vars);
        let mut body_vars = Vec::new();
        for b in &r.body {
            vars_of(b, &mut body_vars);
        }
        for v in head_vars {
            if !body_vars.contains(&v) {
                diags.push(
                    r.span,
                    format!("`?{v}` is in the head but not the body, so it is unconstrained"),
                );
            }
        }
    }
    derived
}

/// Saturates the rules against `consts`, adding every derived fact.
///
/// Naive iteration to a fixpoint: each round re-derives everything and stops when a round
/// adds nothing. Semi-naive evaluation would avoid the repeated work, but these programs
/// run once over a handful of objects, and the simpler loop is easier to be sure of.
pub fn saturate(ast: &Ast, sig: &Sig, consts: &mut Constants, diags: &mut Diagnostics) {
    if ast.rules.is_empty() {
        return;
    }
    let derived = check(ast, sig, diags);
    if !diags.is_empty() {
        return;
    }
    for name in &derived {
        consts.declare_pred(name);
    }

    let objects: Vec<String> = {
        let mut v: Vec<String> = sig.objects.keys().cloned().collect();
        v.sort();
        v
    };
    let mut added = 0usize;

    loop {
        let mut changed = false;
        for r in &ast.rules {
            let mut vars = Vec::new();
            vars_of(&r.head, &mut vars);
            for b in &r.body {
                vars_of(b, &mut vars);
            }
            let combos = objects.len().checked_pow(vars.len() as u32).unwrap_or(usize::MAX);
            if combos > MAX_GROUNDINGS {
                diags.push(
                    r.span,
                    format!(
                        "this rule has {} variables over {} objects, which is too many \
                         groundings to try",
                        vars.len(),
                        objects.len()
                    ),
                );
                return;
            }

            for i in 0..combos.max(1) {
                // Decode `i` as a base-|objects| numeral, one digit per variable.
                let mut assign = Vec::with_capacity(vars.len());
                let mut n = i;
                for v in &vars {
                    if objects.is_empty() {
                        break;
                    }
                    assign.push((v.clone(), objects[n % objects.len()].clone()));
                    n /= objects.len();
                }
                if assign.len() != vars.len() {
                    break;
                }

                let holds = r.body.iter().all(|b| {
                    ground(b, &assign)
                        .is_some_and(|args| consts.lookup(&b.pred, &args) == Some(true))
                });
                if !holds {
                    continue;
                }
                let Some(head_args) = ground(&r.head, &assign) else {
                    continue;
                };
                if consts.lookup(&r.head.pred, &head_args) == Some(true) {
                    continue;
                }
                consts.add_fact(&r.head.pred, &head_args);
                added += 1;
                changed = true;
                if added > MAX_DERIVED {
                    diags.push(
                        r.span,
                        format!("the rules derived more than {MAX_DERIVED} facts"),
                    );
                    return;
                }
            }
        }
        if !changed {
            break;
        }
    }
    debug_assert!(
        ast.rules.iter().all(|r| !r.head.pred.is_empty()),
        "every rule must name a head predicate"
    );
    let _ = atom_key; // the key format lives in `ground`; kept imported for the doc link
}

#[cfg(test)]
mod tests {
    use crate::Problem;

    const MAP: &str = r#"
        types   { Room - Object }
        objects { hall, study, attic - Room }
        agents  { }
        props   { here(Room) }
        constants {
            !adjacent(Room, Room)
            adjacent(hall, study)
            adjacent(study, attic)
        }
        rules {
            reach(?x, ?y) :- adjacent(?x, ?y)
            reach(?x, ?z) :- adjacent(?x, ?y), reach(?y, ?z)
        }
        initially { here(hall) }
        actions {}
    "#;

    fn holds(src: &str, f: &str) -> bool {
        let mut p = Problem::parse(src).unwrap_or_else(|e| panic!("{e}"));
        let mut d = crate::Diagnostics::default();
        let toks = crate::lex(f, &mut d);
        let e = crate::Parser::new(&toks).parse_expr(&mut d);
        let id = crate::lower_formula(
            &e,
            &p.sig,
            &p.consts,
            &crate::Bindings::default(),
            &mut p.store,
            &mut d,
        );
        assert!(d.is_empty(), "{}", d.render(f));
        p.entails(id)
    }

    #[test]
    fn transitive_closure_is_reached_and_nothing_more() {
        // The one-step cases, the two-step case that only recursion gives, and the
        // absence of a path that does not exist — all three, because a rule that derived
        // everything would pass the first two.
        assert!(holds(MAP, "adjacent(hall, study)"));
        assert!(holds(MAP, "reach(hall, study)"), "one step");
        assert!(holds(MAP, "reach(study, attic)"), "one step");
        assert!(holds(MAP, "reach(hall, attic)"), "two steps — only the recursive rule gives this");
        assert!(!holds(MAP, "reach(attic, hall)"), "the relation is not symmetric");
        assert!(!holds(MAP, "reach(hall, hall)"), "and not reflexive");
    }

    #[test]
    fn a_derived_predicate_folds_away_rather_than_becoming_an_atom() {
        // The point of deriving into the constant table: `reach` is gone by the time the
        // semantics runs, so it costs no bit in any world.
        let p = Problem::parse(MAP).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(p.sig.n_atoms(), 3, "only `here(Room)` expands into atoms");
        assert!(p.consts.is_constant_pred("reach"));
    }

    #[test]
    fn a_rule_over_a_fluent_is_refused_in_either_position() {
        // The fixpoint runs once at parse time, which is sound only because constants are
        // static. A fluent's extension varies per world and per action, so it can be
        // neither derived nor depended on — and the two cases get different messages,
        // because the reason differs.
        let in_body = Problem::parse(
            r#"
            types{ Room - Object } objects{ hall - Room } agents{} props{ here(Room) }
            rules { near(?x) :- here(?x) }
            initially { here(hall) } actions{}
        "#,
        )
        .unwrap_err();
        assert!(in_body.contains("a rule body may only mention"), "got {in_body}");

        let in_head = Problem::parse(
            r#"
            types{ Room - Object } objects{ hall - Room } agents{} props{ here(Room) }
            constants { !adj(Room, Room) adj(hall, hall) }
            rules { here(?x) :- adj(?x, ?x) }
            initially { here(hall) } actions{}
        "#,
        )
        .unwrap_err();
        assert!(in_head.contains("cannot be derived by a rule"), "got {in_head}");
        assert!(in_head.contains("rules run once"), "and says why: {in_head}");
    }

    #[test]
    fn an_unconstrained_head_variable_is_rejected() {
        // `?y` ranges over everything, so this would assert `linked(hall, *)` for every
        // object — almost never what was meant, and silently enormous.
        let e = Problem::parse(
            r#"
            types{ Room - Object } objects{ hall, study - Room } agents{} props{ x }
            constants { !adjacent(Room, Room) adjacent(hall, study) }
            rules { linked(?a, ?y) :- adjacent(?a, ?a) }
            initially { x } actions{}
        "#,
        )
        .unwrap_err();
        assert!(e.contains("not the body"), "got {e}");
    }

    #[test]
    fn a_rule_deriving_a_name_at_two_arities_is_rejected() {
        let e = Problem::parse(
            r#"
            types{ Room - Object } objects{ hall, study - Room } agents{} props{ x }
            constants { !adjacent(Room, Room) adjacent(hall, study) }
            rules {
                p(?a)     :- adjacent(?a, ?a)
                p(?a, ?b) :- adjacent(?a, ?b)
            }
            initially { x } actions{}
        "#,
        )
        .unwrap_err();
        assert!(e.contains("arguments elsewhere"), "got {e}");
    }

    #[test]
    fn rules_compose_with_definitions_and_queries() {
        // A derived predicate is an ordinary constant afterwards, so everything that
        // works on constants works on it.
        let src = r#"
            types   { Room - Object }
            objects { hall, study, attic - Room }
            agents  { }
            props   { here(Room) }
            constants { !adjacent(Room, Room) adjacent(hall, study) adjacent(study, attic) }
            rules {
                reach(?x, ?y) :- adjacent(?x, ?y)
                reach(?x, ?z) :- adjacent(?x, ?y), reach(?y, ?z)
            }
            define { stranded(?r) = !reach(?r, attic) }
            initially { here(hall) }
            goal { stranded(attic) & !stranded(hall) }
            actions {}
        "#;
        let p = Problem::parse(src).unwrap_or_else(|e| panic!("{e}"));
        assert!(p.entails(p.goal.expect("declared")), "attic reaches nothing; hall reaches attic");
    }
}
