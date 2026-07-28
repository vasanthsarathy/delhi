//! Grounding action declarations into one `ActionDef` per parameter assignment.

use crate::ast::{ActionDecl, Arg, Clause, Expr};
use crate::lower_formula::{lower_formula, Bindings};
use crate::{Constants, Diagnostics, Sig, Span};
use delhi_mb::{ActionDef, Effect, Kind};
use delhi_syntax::{AgentId, FormulaId, Store};

/// The read-only context every lowering step needs. Bundled so that grounding functions
/// stay under clippy's argument-count limit without reaching for an `allow`.
pub struct Ctx<'a> {
    /// The checked signature.
    pub sig: &'a Sig,
    /// Parse-time constants.
    pub consts: &'a Constants,
}

/// One fully ground action, plus the surface information the semantics does not carry.
#[derive(Debug)]
pub struct GroundAction {
    /// Display name including arguments, e.g. `move(alice,hall,study)`.
    pub name: String,
    /// Who performs it. Recorded for tooling only — [KR21] §3 keeps observability
    /// independent of agency, so `ActionDef` has no actor field.
    pub actor: Option<AgentId>,
    /// What the semantics consumes.
    pub def: ActionDef,
}

/// Grounds every declaration into one action per parameter assignment, dropping any
/// whose precondition folds to `⊥`.
pub fn ground_actions(
    decls: &[ActionDecl],
    sig: &Sig,
    consts: &Constants,
    store: &mut Store,
    diags: &mut Diagnostics,
) -> Vec<GroundAction> {
    let ctx = Ctx { sig, consts };
    let mut out = Vec::new();
    for decl in decls {
        // Every combination of objects for the declared parameters.
        let mut assignments: Vec<Vec<(String, String)>> = vec![Vec::new()];
        for p in &decl.params {
            if p.ty != "Object" && !ctx.sig.types.contains_key(&p.ty) {
                diags.push(p.span, format!("unknown type `{}`", p.ty));
            }
            let objs = ctx.sig.objects_of(&p.ty);
            let mut next = Vec::with_capacity(assignments.len() * objs.len());
            for prefix in &assignments {
                for o in &objs {
                    let mut v = prefix.clone();
                    v.push((p.name.clone(), o.clone()));
                    next.push(v);
                }
            }
            assignments = next;
        }
        for assign in assignments {
            if let Some(g) = ground_one(decl, &assign, &ctx, store, diags) {
                out.push(g);
            }
        }
    }
    out
}

fn display_name(decl: &ActionDecl, assign: &[(String, String)]) -> String {
    let args: Vec<&str> = assign.iter().map(|(_, o)| o.as_str()).collect();
    format!("{}({})", decl.name, args.join(","))
}

/// Grounds one declaration under one assignment. `None` when the action is impossible
/// or malformed.
fn ground_one(
    decl: &ActionDecl,
    assign: &[(String, String)],
    ctx: &Ctx,
    store: &mut Store,
    diags: &mut Diagnostics,
) -> Option<GroundAction> {
    let binds = Bindings::from(assign.to_vec());
    let name = display_name(decl, assign);

    let mut actor: Option<AgentId> = None;
    let mut pre: Option<FormulaId> = None;
    let mut kinds: Vec<(Kind, Span)> = Vec::new();
    let mut effects: Vec<Effect> = Vec::new();
    let mut effect_span: Option<Span> = None;
    let mut observes: Vec<(AgentId, FormulaId)> = Vec::new();
    let mut aware: Vec<(AgentId, FormulaId)> = Vec::new();

    for clause in &decl.clauses {
        match clause {
            Clause::Actor(a, sp) => {
                let who = resolve_agent_arg(a, &binds, ctx.sig, *sp, diags);
                if let Some(id) = who {
                    actor = Some(id);
                }
            }
            Clause::Pre(e) => {
                if pre.is_some() {
                    diags.push(e.span(), "at most one `pre` clause; write a conjunction instead");
                }
                pre = Some(lower_formula(e, ctx.sig, ctx.consts, &binds, store, diags));
            }
            Clause::Determines(e) => {
                let f = lower_formula(e, ctx.sig, ctx.consts, &binds, store, diags);
                kinds.push((Kind::Sensing(f), e.span()));
            }
            Clause::Announces(e) => {
                let f = lower_formula(e, ctx.sig, ctx.consts, &binds, store, diags);
                kinds.push((Kind::Announce(f), e.span()));
            }
            Clause::Causes { lits, cond, span } => {
                let cond_f = match cond {
                    Some(c) => lower_formula(c, ctx.sig, ctx.consts, &binds, store, diags),
                    None => store.tru(),
                };
                let mut signed = Vec::with_capacity(lits.len());
                for (term, positive) in lits {
                    let e = Expr::Atom(term.clone());
                    let f = lower_formula(&e, ctx.sig, ctx.consts, &binds, store, diags);
                    // A `causes` literal must name a real atom, not a constant or a
                    // folded value — the semantics updates atoms, and nothing else.
                    match atom_of(store, f) {
                        Some(id) => signed.push((id, *positive)),
                        None => diags.push(
                            term.span,
                            "`causes` needs a proposition; constants cannot be changed",
                        ),
                    }
                }
                effects.push(Effect { lits: signed, cond: cond_f });
                effect_span = Some(*span);
            }
            Clause::Observes { who, cond, span } => {
                for id in expand_agent_arg(who, &binds, ctx.sig, *span, diags) {
                    let f = observer_condition(ctx, cond, who, id, &binds, store, diags);
                    observes.push((id, f));
                }
            }
            Clause::Aware { who, cond, span } => {
                for id in expand_agent_arg(who, &binds, ctx.sig, *span, diags) {
                    let f = observer_condition(ctx, cond, who, id, &binds, store, diags);
                    aware.push((id, f));
                }
            }
        }
    }

    if !effects.is_empty() {
        kinds.push((Kind::Ontic(effects), effect_span.unwrap_or(decl.span)));
    }
    if kinds.len() != 1 {
        diags.push(
            decl.span,
            format!(
                "`{}` must have exactly one of `causes`, `determines`, or `announces`; found {}",
                decl.name,
                kinds.len()
            ),
        );
        return None;
    }
    let kind = kinds.pop().expect("checked len == 1").0;

    let pre = pre.unwrap_or_else(|| store.tru());
    // §7.1: an action that can never fire is not generated at all.
    if pre == store.fls() {
        return None;
    }

    let def = ActionDef { name: name.clone(), pre, kind, observes, aware };
    if let Err(e) = def.validate(store) {
        diags.push(decl.span, format!("`{name}` is not well formed: {e:?}"));
        return None;
    }
    Some(GroundAction { name, actor, def })
}

/// The atom behind a lowered formula, when it is exactly an atom.
fn atom_of(store: &Store, f: FormulaId) -> Option<delhi_syntax::AtomId> {
    match store.node(f) {
        delhi_syntax::Node::Atom(a) => Some(*a),
        _ => None,
    }
}

/// Resolves an argument that must name a single declared agent.
fn resolve_agent_arg(
    a: &Arg,
    binds: &Bindings,
    sig: &Sig,
    span: Span,
    diags: &mut Diagnostics,
) -> Option<AgentId> {
    let name = match a {
        Arg::Obj(o) => o.clone(),
        Arg::Var(v) => match binds.get(v) {
            Some(o) => o.to_string(),
            None => {
                diags.push(span, format!("`?{v}` is not an action parameter"));
                return None;
            }
        },
        Arg::Ty(t) => {
            diags.push(span, format!("type name `{t}` is not an agent"));
            return None;
        }
    };
    match sig.agent_id(&name) {
        Some(i) => Some(i),
        None => {
            diags.push(span, format!("`{name}` is not a declared agent"));
            None
        }
    }
}

/// The agents an observability clause applies to. A variable that is not an action
/// parameter is clause-scoped and ranges over every declared agent.
fn expand_agent_arg(
    a: &Arg,
    binds: &Bindings,
    sig: &Sig,
    span: Span,
    diags: &mut Diagnostics,
) -> Vec<AgentId> {
    match a {
        Arg::Var(v) if binds.get(v).is_none() => (0..sig.n_agents() as AgentId).collect(),
        _ => resolve_agent_arg(a, binds, sig, span, diags).into_iter().collect(),
    }
}

/// Lowers an observability guard, binding a clause-scoped variable to the agent it
/// currently stands for so guards like `?o observes if at(?o, ?f)` work.
fn observer_condition(
    ctx: &Ctx,
    cond: &Option<Expr>,
    who: &Arg,
    id: AgentId,
    binds: &Bindings,
    store: &mut Store,
    diags: &mut Diagnostics,
) -> FormulaId {
    let Some(c) = cond else {
        return store.tru();
    };
    let scoped = match who {
        Arg::Var(v) if binds.get(v).is_none() => binds.with(v, ctx.sig.agent_name(id)),
        _ => binds.clone(),
    };
    lower_formula(c, ctx.sig, ctx.consts, &scoped, store, diags)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{parse_file, Constants, Diagnostics, Sig};
    use delhi_mb::Kind;
    use delhi_syntax::Store;

    fn ground(src: &str) -> (Sig, Store, Vec<GroundAction>) {
        let mut d = Diagnostics::default();
        let ast = parse_file(src, &mut d);
        let sig = Sig::build(&ast, &mut d);
        let consts = Constants::build(&ast, &sig, &mut d);
        let mut store = Store::default();
        let acts = ground_actions(&ast.actions, &sig, &consts, &mut store, &mut d);
        assert!(d.is_empty(), "unexpected errors:\n{}", d.render(src));
        (sig, store, acts)
    }

    const MOVE: &str = r#"
        types   { Actor - Object, Location - Object }
        objects { alice - Actor, hall, study - Location }
        agents  { alice }
        props   { at(Actor, Location) }
        constants { !adjacent(Location, Location), adjacent(hall, study) }
        initially { }
        actions {
            move(?a - Actor, ?f - Location, ?t - Location) {
                actor  ?a
                pre    at(?a, ?f) & adjacent(?f, ?t)
                causes at(?a, ?t), !at(?a, ?f)
                ?o observes if at(?o, ?f)
            }
        }
    "#;

    #[test]
    fn impossible_ground_actions_are_never_generated() {
        // 1 actor x 2 froms x 2 tos = 4 candidates, but `adjacent` is true for exactly
        // one ordered pair, so only that one survives (§7.1).
        let (_, _, acts) = ground(MOVE);
        assert_eq!(acts.len(), 1, "only move(alice,hall,study) has a satisfiable precondition");
        assert_eq!(acts[0].name, "move(alice,hall,study)");
    }

    #[test]
    fn the_actor_is_recorded_but_does_not_reach_the_action_def() {
        let (sig, _, acts) = ground(MOVE);
        assert_eq!(acts[0].actor, Some(sig.agent_id("alice").unwrap()));
        // `ActionDef` has no actor field at all — nothing to assert beyond that the
        // observability lists are what carry agent information into the semantics.
        assert_eq!(acts[0].def.observes.len(), 1, "one entry per declared agent");
    }

    #[test]
    fn clause_scoped_variables_expand_over_the_declared_agents() {
        let src = r#"
            types   { Actor - Object }
            objects { alice, bob, carol - Actor }
            agents  { alice, bob, carol }
            props   { p }
            initially { }
            actions {
                shout() { actor alice, announces p, ?o observes }
            }
        "#;
        let (_, _, acts) = ground(src);
        assert_eq!(acts.len(), 1);
        assert_eq!(acts[0].def.observes.len(), 3, "?o ranges over every declared agent");
    }

    #[test]
    fn causes_becomes_an_ontic_kind_with_signed_literals() {
        let (sig, _, acts) = ground(MOVE);
        let at_study = sig.atom_id("at", &["alice".into(), "study".into()]).unwrap();
        let at_hall = sig.atom_id("at", &["alice".into(), "hall".into()]).unwrap();
        match &acts[0].def.kind {
            Kind::Ontic(effects) => {
                assert_eq!(effects.len(), 1, "one `causes` clause is one Effect");
                let lits = &effects[0].lits;
                assert!(lits.contains(&(at_study, true)));
                assert!(lits.contains(&(at_hall, false)));
            }
            other => panic!("expected an ontic action, got {other:?}"),
        }
    }

    #[test]
    fn determines_and_announces_become_their_own_kinds() {
        let src = r#"
            types{ Actor - Object } objects{ a - Actor } agents{ a } props{ p }
            initially{}
            actions {
                look()  { actor a, determines p, a observes }
                tell()  { actor a, announces  p, a observes }
            }
        "#;
        let (_, _, acts) = ground(src);
        let look = acts.iter().find(|x| x.name == "look()").unwrap();
        let tell = acts.iter().find(|x| x.name == "tell()").unwrap();
        assert!(matches!(look.def.kind, Kind::Sensing(_)));
        assert!(matches!(tell.def.kind, Kind::Announce(_)));
    }

    #[test]
    fn a_missing_precondition_defaults_to_top() {
        let (_, store, acts) = ground(r#"
            types{ Actor - Object } objects{ a - Actor } agents{ a } props{ p }
            initially{} actions { go() { actor a, causes p, a observes } }
        "#);
        let mut s = store;
        assert_eq!(acts[0].def.pre, s.tru());
    }

    #[test]
    fn two_precondition_clauses_are_rejected() {
        let mut d = Diagnostics::default();
        let src = r#"
            types{ Actor - Object } objects{ a - Actor } agents{ a } props{ p, q }
            initially{} actions { go() { actor a, pre p, pre q, causes p, a observes } }
        "#;
        let ast = parse_file(src, &mut d);
        let sig = Sig::build(&ast, &mut d);
        let c = Constants::build(&ast, &sig, &mut d);
        let mut s = Store::default();
        let _ = ground_actions(&ast.actions, &sig, &c, &mut s, &mut d);
        assert!(d.items().iter().any(|x| x.message.contains("at most one")),
                "the surface language allows one `pre`; write a conjunction instead");
    }

    #[test]
    fn an_action_with_no_effect_clause_is_rejected() {
        let mut d = Diagnostics::default();
        let src = r#"
            types{ Actor - Object } objects{ a - Actor } agents{ a } props{ p }
            initially{} actions { go() { actor a, a observes } }
        "#;
        let ast = parse_file(src, &mut d);
        let sig = Sig::build(&ast, &mut d);
        let c = Constants::build(&ast, &sig, &mut d);
        let mut s = Store::default();
        let _ = ground_actions(&ast.actions, &sig, &c, &mut s, &mut d);
        assert!(d.items().iter().any(|x| x.message.contains("exactly one")));
    }

    #[test]
    fn mixing_two_effect_kinds_is_rejected() {
        let mut d = Diagnostics::default();
        let src = r#"
            types{ Actor - Object } objects{ a - Actor } agents{ a } props{ p, q }
            initially{} actions { go() { actor a, causes p, determines q, a observes } }
        "#;
        let ast = parse_file(src, &mut d);
        let sig = Sig::build(&ast, &mut d);
        let c = Constants::build(&ast, &sig, &mut d);
        let mut s = Store::default();
        let _ = ground_actions(&ast.actions, &sig, &c, &mut s, &mut d);
        assert!(d.items().iter().any(|x| x.message.contains("exactly one")));
    }

    #[test]
    fn well_formedness_from_the_semantics_layer_is_surfaced() {
        // An agent may not be both a full and a partial observer — `delhi-mb`'s
        // `ActionDef::validate` catches it; we must report it with a span rather
        // than letting it reach the semantics.
        let mut d = Diagnostics::default();
        let src = r#"
            types{ Actor - Object } objects{ a - Actor } agents{ a } props{ p }
            initially{} actions { go() { actor a, causes p, a observes, a aware } }
        "#;
        let ast = parse_file(src, &mut d);
        let sig = Sig::build(&ast, &mut d);
        let c = Constants::build(&ast, &sig, &mut d);
        let mut s = Store::default();
        let _ = ground_actions(&ast.actions, &sig, &c, &mut s, &mut d);
        assert!(!d.is_empty(), "the observer-class overlap must be reported");
    }
}
