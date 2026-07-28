//! The explicit `state` form (§7.3): worlds and edges written out directly.

use crate::ast::{Cmp, EdgeDecl, WorldDecl};
use crate::{Ctx, Diagnostics, Span};
use delhi_mb::{Model, State};
use delhi_syntax::Store;
use std::collections::HashMap;

/// Builds a state from explicitly written worlds and edges.
///
/// Reflexive edges are implicit and the relation is transitively closed before the
/// frame is validated, so the source states the shape rather than its closure.
pub fn build_explicit(
    worlds: &[WorldDecl],
    edges: &[EdgeDecl],
    ctx: &Ctx,
    _store: &mut Store,
    diags: &mut Diagnostics,
) -> Option<State> {
    let n_atoms = ctx.sig.n_atoms();
    let n_agents = ctx.sig.n_agents();

    if worlds.is_empty() {
        diags.push(Span::new(0, 0), "a `state` block needs at least one world");
        return None;
    }

    let mut index: HashMap<&str, usize> = HashMap::new();
    for (i, w) in worlds.iter().enumerate() {
        if index.insert(w.name.as_str(), i).is_some() {
            diags.push(w.span, format!("duplicate world `{}`", w.name));
        }
    }

    let designated: Vec<usize> = worlds
        .iter()
        .enumerate()
        .filter(|(_, w)| w.designated)
        .map(|(i, _)| i)
        .collect();
    if designated.len() != 1 {
        diags.push(
            worlds[0].span,
            format!(
                "exactly one world must be designated with `*`; found {}",
                designated.len()
            ),
        );
        return None;
    }

    let mut model = Model::new(worlds.len(), n_agents, n_atoms.max(1));
    for (i, w) in worlds.iter().enumerate() {
        for t in &w.facts {
            let args: Vec<String> = t
                .args
                .iter()
                .map(|a| match a {
                    crate::ast::Arg::Obj(o) => o.clone(),
                    other => {
                        diags.push(t.span, format!("a world's facts must be ground: {other:?}"));
                        String::new()
                    }
                })
                .collect();
            match ctx.sig.atom_id(&t.pred, &args) {
                Some(a) => model.val[i].set(a as usize),
                None => diags.push(
                    t.span,
                    format!("no proposition `{}`", crate::ground::atom_key(&t.pred, &args)),
                ),
            }
        }
    }

    // Record the strict edges so their converses can be checked after closure.
    let mut strict: Vec<(usize, usize, usize, Span)> = Vec::new();
    for e in edges {
        let Some(agent) = ctx.sig.agent_id(&e.agent) else {
            diags.push(e.span, format!("`{}` is not a declared agent", e.agent));
            continue;
        };
        let (Some(&from), Some(&to)) =
            (index.get(e.from.as_str()), index.get(e.to.as_str()))
        else {
            for name in [&e.from, &e.to] {
                if !index.contains_key(name.as_str()) {
                    diags.push(e.span, format!("unknown world `{name}`"));
                }
            }
            continue;
        };
        let a = agent as usize;
        model.relate(a, from, to);
        match e.cmp {
            Cmp::Equi => model.relate(a, to, from),
            Cmp::Le => {}
            Cmp::Lt => strict.push((a, from, to, e.span)),
        }
    }

    // Transitive closure. `Model::new` already installed the reflexive edges.
    loop {
        let mut changed = false;
        for a in 0..n_agents {
            for u in 0..worlds.len() {
                let reach = model.rel[a][u].ones();
                for v in reach {
                    for w in model.rel[a][v].ones() {
                        if !model.rel[a][u].get(w) {
                            model.relate(a, u, w);
                            changed = true;
                        }
                    }
                }
            }
        }
        if !changed {
            break;
        }
    }

    for (a, from, to, span) in strict {
        if model.rel[a][to].get(from) {
            diags.push(
                span,
                format!(
                    "`{} < {}` is strict, but the converse follows from the other edges",
                    worlds[from].name, worlds[to].name
                ),
            );
        }
    }

    if let Err(e) = model.validate() {
        diags.push(worlds[0].span, format!("the declared frame is invalid: {e:?}"));
        return None;
    }

    Some(State { model, designated: designated[0] })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{parse_file, Constants, Ctx, Diagnostics, Sig};
    use delhi_syntax::Store;

    fn build(src: &str) -> Result<(Sig, Store, State), String> {
        let mut d = Diagnostics::default();
        let ast = parse_file(src, &mut d);
        let sig = Sig::build(&ast, &mut d);
        let consts = Constants::build(&ast, &sig, &mut d);
        let ctx = Ctx { sig: &sig, consts: &consts };
        let (worlds, edges) = match &ast.init {
            Some(crate::ast::Init::Explicit { worlds, edges, .. }) => (worlds.clone(), edges.clone()),
            other => panic!("expected an explicit state, got {other:?}"),
        };
        let mut store = Store::default();
        match build_explicit(&worlds, &edges, &ctx, &mut store, &mut d) {
            Some(st) if d.is_empty() => Ok((sig, store, st)),
            _ => Err(d.render(src)),
        }
    }

    const COIN: &str = r#"
        types   { Actor - Object }
        objects { alice, bob, carol - Actor }
        agents  { alice, bob, carol }
        props   { h }
        state {
            *u <- { h }
             v <- { }
            carol: v < u
        }
        actions {}
    "#;

    #[test]
    fn reproduces_the_published_coin_lie_start_state() {
        let (sig, _, st) = build(COIN).expect("should build");
        assert_eq!(st.model.validate(), Ok(()));
        assert_eq!(st.model.n_worlds, 2);
        let carol = sig.agent_id("carol").unwrap() as usize;
        let h = sig.atom_id("h", &[]).unwrap() as usize;
        let d = st.designated;
        assert!(st.model.val[d].get(h));
        let other = if d == 0 { 1 } else { 0 };
        assert!(st.model.rel[carol][other].get(d), "`v < u` means u is the more plausible");
        assert!(!st.model.rel[carol][d].get(other));
    }

    #[test]
    fn tilde_adds_both_directions() {
        let (sig, _, st) = build(r#"
            types{ Actor - Object } objects{ a - Actor } agents{ a } props{ h }
            state { *u <- { h }, v <- { }, a: u ~ v }
            actions{}
        "#).expect("should build");
        let a = sig.agent_id("a").unwrap() as usize;
        assert!(st.model.rel[a][0].get(1) && st.model.rel[a][1].get(0));
    }

    #[test]
    fn the_relation_is_transitively_closed() {
        let (sig, _, st) = build(r#"
            types{ Actor - Object } objects{ a - Actor } agents{ a } props{ p, q }
            state { *u <- { }, v <- { p }, w <- { q }, a: u <= v, a: v <= w }
            actions{}
        "#).expect("should build");
        let a = sig.agent_id("a").unwrap() as usize;
        assert!(st.model.rel[a][0].get(2), "u <= v <= w implies u <= w");
        assert_eq!(st.model.validate(), Ok(()));
    }

    #[test]
    fn strict_edges_whose_converse_is_derivable_are_rejected() {
        // `a: u < v` together with `a: v ~ u` is contradictory.
        let err = build(r#"
            types{ Actor - Object } objects{ a - Actor } agents{ a } props{ p }
            state { *u <- { }, v <- { p }, a: u < v, a: v ~ u }
            actions{}
        "#).unwrap_err();
        assert!(err.contains("strict"), "expected a strictness complaint, got:\n{err}");
    }

    #[test]
    fn exactly_one_designated_world_is_required() {
        let none = build(r#"
            types{ Actor - Object } objects{ a - Actor } agents{ a } props{ p }
            state { u <- { }, v <- { p } }
            actions{}
        "#).unwrap_err();
        assert!(none.contains("designated"));

        let two = build(r#"
            types{ Actor - Object } objects{ a - Actor } agents{ a } props{ p }
            state { *u <- { }, *v <- { p } }
            actions{}
        "#).unwrap_err();
        assert!(two.contains("designated"));
    }

    #[test]
    fn unknown_world_and_agent_names_are_reported() {
        let err = build(r#"
            types{ Actor - Object } objects{ a - Actor } agents{ a } props{ p }
            state { *u <- { }, a: u ~ nosuch }
            actions{}
        "#).unwrap_err();
        assert!(err.contains("nosuch"));

        let err = build(r#"
            types{ Actor - Object } objects{ a - Actor } agents{ a } props{ p }
            state { *u <- { }, v <- { p }, ghost: u ~ v }
            actions{}
        "#).unwrap_err();
        assert!(err.contains("ghost"));
    }

    #[test]
    fn a_frame_that_is_not_locally_connected_is_rejected() {
        // u and v both reach w but cannot be compared with each other.
        let err = build(r#"
            types{ Actor - Object } objects{ a - Actor } agents{ a } props{ p, q }
            state { *u <- { }, v <- { p }, w <- { q }, a: u <= w, a: v <= w }
            actions{}
        "#).unwrap_err();
        assert!(err.contains("connected") || err.contains("frame"),
                "expected a frame-condition complaint, got:\n{err}");
    }
}
