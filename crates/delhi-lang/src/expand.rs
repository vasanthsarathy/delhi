//! `define` — named formulas, expanded before anything is lowered.
//!
//! A definition is a macro over the surface syntax:
//!
//! ```text
//! define {
//!     blocked(?r)  = !lit(?r) | locked(?r)
//!     agree(?x,?y) = K[?x] safe & K[?y] safe
//! }
//! ```
//!
//! Expansion happens on the AST, not during lowering, and that choice is what keeps the
//! feature cheap: `lower_formula` never learns definitions exist, so nothing about the
//! semantics, the constant folding or the action grounding changes. A defined name is
//! gone by the time any of them run.
//!
//! # Not recursive, by construction
//!
//! The call graph is checked for cycles when the table is built, so expansion cannot
//! diverge — it is not guarded by a depth limit that could be hit legitimately. Recursion
//! would need a least fixpoint, which is a different feature with a different cost; see
//! `rules` for the constant-only case where that fixpoint is affordable.

use crate::ast::{ActionDecl, Arg, Ast, Clause, Expr, Init, Term};
use crate::Diagnostics;
use std::collections::{HashMap, HashSet};

/// The checked definition table.
#[derive(Debug, Default)]
pub struct Defs {
    by_name: HashMap<String, (Vec<String>, Expr)>,
}

impl Defs {
    /// Whether `name` is defined.
    pub fn contains(&self, name: &str) -> bool {
        self.by_name.contains_key(name)
    }

    /// How many definitions there are.
    pub fn len(&self) -> usize {
        self.by_name.len()
    }

    /// Whether there are none.
    pub fn is_empty(&self) -> bool {
        self.by_name.is_empty()
    }

    /// Builds the table, rejecting duplicates, clashes with declared propositions, and
    /// any cycle in the call graph.
    pub fn build(ast: &Ast, diags: &mut Diagnostics) -> Defs {
        let mut by_name: HashMap<String, (Vec<String>, Expr)> = HashMap::new();
        let declared: HashSet<&str> = ast.props.iter().map(|p| p.name.as_str()).collect();

        for d in &ast.defines {
            if declared.contains(d.name.as_str()) {
                diags.push(d.span, format!("`{}` is already a declared proposition", d.name));
                continue;
            }
            if by_name.contains_key(&d.name) {
                diags.push(d.span, format!("duplicate definition `{}`", d.name));
                continue;
            }
            let mut seen = HashSet::new();
            for param in &d.params {
                if !seen.insert(param.clone()) {
                    diags.push(d.span, format!("`?{param}` appears twice in the parameter list"));
                }
            }
            by_name.insert(d.name.clone(), (d.params.clone(), d.body.clone()));
        }

        // Cycle check up front. Expansion is then guaranteed to terminate, so it needs no
        // depth limit — and a limit would be the wrong tool anyway, since it cannot tell
        // a cycle from a legitimately deep nesting.
        let defs = Defs { by_name };
        for d in &ast.defines {
            if defs.reaches(&d.name, &d.name, &mut HashSet::new()) {
                diags.push(d.span, format!("`{}` is defined in terms of itself", d.name));
            }
        }
        defs
    }

    /// Whether expanding `from` can reach `target`, following definition calls.
    fn reaches(&self, from: &str, target: &str, visited: &mut HashSet<String>) -> bool {
        let Some((_, body)) = self.by_name.get(from) else {
            return false;
        };
        let mut called = Vec::new();
        collect_calls(body, &mut called);
        for name in called {
            if name == target {
                return true;
            }
            if visited.insert(name.clone()) && self.reaches(&name, target, visited) {
                return true;
            }
        }
        false
    }
}

/// Names of the predicates applied anywhere in `e`.
fn collect_calls(e: &Expr, out: &mut Vec<String>) {
    match e {
        Expr::Hole(_) | Expr::True(_) | Expr::False(_) => {}
        Expr::Atom(t) => out.push(t.pred.clone()),
        Expr::Not(a, _) => collect_calls(a, out),
        Expr::And(a, b, _) | Expr::Or(a, b, _) | Expr::Implies(a, b, _) => {
            collect_calls(a, out);
            collect_calls(b, out);
        }
        Expr::Modality { cond, body, .. } => {
            if let Some(c) = cond {
                collect_calls(c, out);
            }
            collect_calls(body, out);
        }
    }
}

/// Replaces the parameter variables in a definition body with the call's arguments.
///
/// Substitution is over `Arg`, before names are resolved, so an argument may itself be a
/// variable — an action parameter passed straight through — and so a definition may be
/// used inside a modality's agent list.
fn subst(e: &Expr, map: &HashMap<String, Arg>) -> Expr {
    let sub_arg = |a: &Arg| -> Arg {
        match a {
            Arg::Var(v) => map.get(v).cloned().unwrap_or_else(|| a.clone()),
            other => other.clone(),
        }
    };
    let sub_term = |t: &Term| Term {
        pred: t.pred.clone(),
        args: t.args.iter().map(&sub_arg).collect(),
        span: t.span,
    };
    match e {
        Expr::Hole(_) | Expr::True(_) | Expr::False(_) => e.clone(),
        Expr::Atom(t) => Expr::Atom(sub_term(t)),
        Expr::Not(a, s) => Expr::Not(Box::new(subst(a, map)), *s),
        Expr::And(a, b, s) => Expr::And(Box::new(subst(a, map)), Box::new(subst(b, map)), *s),
        Expr::Or(a, b, s) => Expr::Or(Box::new(subst(a, map)), Box::new(subst(b, map)), *s),
        Expr::Implies(a, b, s) => {
            Expr::Implies(Box::new(subst(a, map)), Box::new(subst(b, map)), *s)
        }
        Expr::Modality { op, agents, cond, body, span } => Expr::Modality {
            op: op.clone(),
            agents: agents.as_ref().map(|v| v.iter().map(&sub_arg).collect()),
            cond: cond.as_ref().map(|c| Box::new(subst(c, map))),
            body: Box::new(subst(body, map)),
            span: *span,
        },
    }
}

/// Expands every definition call in `e`.
///
/// The result contains no defined names, so lowering sees only declared propositions and
/// constants. Spans of the *call* are kept for the expanded body, so a diagnostic inside
/// a definition points at the line that used it rather than at the definition.
pub fn expand(e: &Expr, defs: &Defs, diags: &mut Diagnostics) -> Expr {
    match e {
        Expr::Hole(_) | Expr::True(_) | Expr::False(_) => e.clone(),
        Expr::Atom(t) => match defs.by_name.get(&t.pred) {
            None => e.clone(),
            Some((params, body)) => {
                if params.len() != t.args.len() {
                    diags.push(
                        t.span,
                        format!(
                            "`{}` takes {} argument{}, given {}",
                            t.pred,
                            params.len(),
                            if params.len() == 1 { "" } else { "s" },
                            t.args.len()
                        ),
                    );
                    return Expr::False(t.span);
                }
                let map: HashMap<String, Arg> =
                    params.iter().cloned().zip(t.args.iter().cloned()).collect();
                // Expanded again: a body may call other definitions. Terminates because
                // the call graph was checked acyclic when the table was built.
                expand(&subst(body, &map), defs, diags)
            }
        },
        Expr::Not(a, s) => Expr::Not(Box::new(expand(a, defs, diags)), *s),
        Expr::And(a, b, s) => {
            Expr::And(Box::new(expand(a, defs, diags)), Box::new(expand(b, defs, diags)), *s)
        }
        Expr::Or(a, b, s) => {
            Expr::Or(Box::new(expand(a, defs, diags)), Box::new(expand(b, defs, diags)), *s)
        }
        Expr::Implies(a, b, s) => {
            Expr::Implies(Box::new(expand(a, defs, diags)), Box::new(expand(b, defs, diags)), *s)
        }
        Expr::Modality { op, agents, cond, body, span } => Expr::Modality {
            op: op.clone(),
            agents: agents.clone(),
            cond: cond.as_ref().map(|c| Box::new(expand(c, defs, diags))),
            body: Box::new(expand(body, defs, diags)),
            span: *span,
        },
    }
}

/// Rejects a defined name where only a real proposition will do.
///
/// `causes blocked(r)` and a world's fact list both need an atom the semantics can set or
/// store. A definition is a formula, so there is nothing to update — saying so is better
/// than expanding it and failing later with a confusing message.
fn reject_defined_term(t: &Term, defs: &Defs, what: &str, diags: &mut Diagnostics) {
    if defs.contains(&t.pred) {
        diags.push(t.span, format!("`{}` is a definition, not a proposition; {what}", t.pred));
    }
}

/// Expands every formula in the file.
pub fn expand_ast(ast: &mut Ast, defs: &Defs, diags: &mut Diagnostics) {
    if let Some(g) = ast.goal.take() {
        ast.goal = Some(expand(&g, defs, diags));
    }
    let invariants = std::mem::take(&mut ast.invariants);
    ast.invariants =
        invariants.into_iter().map(|(e, s)| (expand(&e, defs, diags), s)).collect();

    match ast.init.take() {
        Some(Init::Declarative(items, span)) => {
            let items = items.iter().map(|e| expand(e, defs, diags)).collect();
            ast.init = Some(Init::Declarative(items, span));
        }
        Some(Init::Explicit { mut worlds, edges, span }) => {
            for w in &mut worlds {
                for f in &w.facts {
                    reject_defined_term(f, defs, "a world's facts must be propositions", diags);
                }
            }
            ast.init = Some(Init::Explicit { worlds, edges, span });
        }
        None => {}
    }

    let actions = std::mem::take(&mut ast.actions);
    ast.actions = actions
        .into_iter()
        .map(|a| expand_action(a, defs, diags))
        .collect();
}

fn expand_action(mut a: ActionDecl, defs: &Defs, diags: &mut Diagnostics) -> ActionDecl {
    a.clauses = a
        .clauses
        .into_iter()
        .map(|c| match c {
            Clause::Pre(e) => Clause::Pre(expand(&e, defs, diags)),
            Clause::Determines(e) => Clause::Determines(expand(&e, defs, diags)),
            Clause::Announces(e) => Clause::Announces(expand(&e, defs, diags)),
            Clause::Causes { lits, cond, span } => {
                for (t, _) in &lits {
                    reject_defined_term(t, defs, "`causes` needs something to set", diags);
                }
                Clause::Causes {
                    lits,
                    cond: cond.map(|c| expand(&c, defs, diags)),
                    span,
                }
            }
            Clause::Observes { who, cond, span } => Clause::Observes {
                who,
                cond: cond.map(|c| expand(&c, defs, diags)),
                span,
            },
            Clause::Aware { who, cond, span } => Clause::Aware {
                who,
                cond: cond.map(|c| expand(&c, defs, diags)),
                span,
            },
            other => other,
        })
        .collect();
    a
}

#[cfg(test)]
mod tests {
    use crate::Problem;

    #[test]
    fn a_definition_stands_for_its_body_wherever_it_is_used() {
        let p = Problem::parse(
            r#"
            types{ Actor - Object } objects{ a - Actor } agents{ a } props{ lit, locked }
            define { blocked = !lit | locked }
            initially { locked }
            goal { blocked }
            actions {}
        "#,
        )
        .unwrap_or_else(|e| panic!("{e}"));
        let goal = p.goal.expect("declared");
        assert!(p.entails(goal), "`locked` holds, so `blocked` should");
    }

    #[test]
    fn parameters_are_substituted_including_inside_modalities() {
        // The substitution runs over `Arg`, before names are resolved, which is what
        // lets a parameter stand where an agent name does.
        let mut p = Problem::parse(
            r#"
            types{ Actor - Object } objects{ a, b - Actor } agents{ a, b } props{ q }
            define { both(?x, ?y) = K[?x] q & K[?y] q }
            initially { q }
            goal { both(a, b) }
            actions {}
        "#,
        )
        .unwrap_or_else(|e| panic!("{e}"));
        let goal = p.goal.expect("declared");
        assert!(p.entails(goal), "both know q in a one-world model");
        // And it really expanded: the same thing written out is the same formula id,
        // since the store is hash-consed.
        let written = {
            let s = &mut p.store;
            let q = s.atom(p.sig.atom_id("q", &[]).unwrap());
            let ka = s.knows(p.sig.agent_id("a").unwrap(), q);
            let kb = s.knows(p.sig.agent_id("b").unwrap(), q);
            s.and(ka, kb)
        };
        assert_eq!(goal, written, "expansion must produce the identical formula");
    }

    #[test]
    fn definitions_may_call_definitions() {
        let p = Problem::parse(
            r#"
            types{} objects{} agents{} props{ x, y }
            define {
                either = x | y
                neither = !either
            }
            initially { }
            goal { neither }
            actions {}
        "#,
        )
        .unwrap_or_else(|e| panic!("{e}"));
        assert!(p.entails(p.goal.expect("declared")), "neither x nor y holds");
    }

    #[test]
    fn a_cycle_is_rejected_rather_than_expanded_forever() {
        // The guard is a cycle check on the call graph, not a depth limit — a limit
        // cannot tell a loop from legitimately deep nesting, and would hang or
        // mis-report depending on which way it was tuned.
        let e = Problem::parse(
            r#"
            types{} objects{} agents{} props{ x }
            define { a = b   b = a }
            initially { x } goal { x } actions{}
        "#,
        )
        .unwrap_err();
        assert!(e.contains("defined in terms of itself"), "got {e}");

        let direct = Problem::parse(
            r#"
            types{} objects{} agents{} props{ x }
            define { loop = loop | x }
            initially { x } goal { x } actions{}
        "#,
        )
        .unwrap_err();
        assert!(direct.contains("defined in terms of itself"), "got {direct}");
    }

    #[test]
    fn arity_and_name_clashes_are_reported() {
        let arity = Problem::parse(
            r#"
            types{} objects{} agents{} props{ x }
            define { pair(?u, ?v) = x }
            initially { x } goal { pair(x) } actions{}
        "#,
        )
        .unwrap_err();
        assert!(arity.contains("takes 2 arguments, given 1"), "got {arity}");

        let clash = Problem::parse(
            r#"
            types{} objects{} agents{} props{ x }
            define { x = x }
            initially { x } goal { x } actions{}
        "#,
        )
        .unwrap_err();
        assert!(clash.contains("already a declared proposition"), "got {clash}");
    }

    #[test]
    fn a_definition_works_in_a_query_as_well_as_in_the_file() {
        // A name usable in a goal but not at the prompt would be the kind of split that
        // makes a feature feel half-finished, so the query paths expand too.
        let mut p = Problem::parse(
            r#"
            types{} objects{} agents{} props{ lit, locked }
            define { blocked = !lit | locked }
            initially { locked }
            actions {}
        "#,
        )
        .unwrap_or_else(|e| panic!("{e}"));

        let mut diags = crate::Diagnostics::default();
        let toks = crate::lex("blocked", &mut diags);
        let e = crate::Parser::new(&toks).parse_expr(&mut diags);
        let e = crate::expand(&e, &p.defs, &mut diags);
        assert!(diags.is_empty(), "the query expands cleanly");

        let f = crate::lower_formula(
            &e,
            &p.sig,
            &p.consts,
            &crate::Bindings::default(),
            &mut p.store,
            &mut diags,
        );
        assert!(diags.is_empty());
        assert!(p.entails(f), "`blocked` holds because `locked` does");
    }

    #[test]
    fn a_definition_cannot_be_caused_or_written_as_a_world_fact() {
        // Both need an atom the semantics can set or store; a definition is a formula,
        // so there is nothing to update.
        let caused = Problem::parse(
            r#"
            types{ Actor - Object } objects{ a - Actor } agents{ a } props{ x, y }
            define { both = x & y }
            initially { x }
            actions { go() { actor a, causes both, a observes } }
        "#,
        )
        .unwrap_err();
        assert!(caused.contains("is a definition, not a proposition"), "got {caused}");

        let fact = Problem::parse(
            r#"
            types{ Actor - Object } objects{ a - Actor } agents{ a } props{ x, y }
            define { both = x & y }
            state { *w0 <- { both } }
            actions {}
        "#,
        )
        .unwrap_err();
        assert!(fact.contains("is a definition, not a proposition"), "got {fact}");
    }
}
