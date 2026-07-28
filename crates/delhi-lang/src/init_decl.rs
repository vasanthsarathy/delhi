//! The declarative `initially` construction (§7.3).

use crate::ast::{Expr, Modal};
use crate::lower_formula::{lower_formula, Bindings};
use crate::{Ctx, Diagnostics};
use delhi_mb::{Bits, Model, State};
use delhi_syntax::{AgentId, AtomId, FormulaId, Node, Store};

/// Refuse rather than allocate `2^n` worlds beyond this many uncertain atoms.
const MAX_UNCERTAIN_ATOMS: usize = 16;

/// Evaluates a purely propositional formula against a valuation.
///
/// `None` when the formula contains a modality, which cannot be judged before the
/// model exists.
fn eval_prop(store: &Store, f: FormulaId, val: &Bits) -> Option<bool> {
    match store.node(f) {
        Node::True => Some(true),
        Node::Atom(a) => Some(val.get(*a as usize)),
        Node::Not(g) => eval_prop(store, *g, val).map(|b| !b),
        Node::And(a, b) => Some(eval_prop(store, *a, val)? && eval_prop(store, *b, val)?),
        _ => None,
    }
}

/// The atom behind a formula, when it is exactly an atom.
fn atom_of(store: &Store, f: FormulaId) -> Option<AtomId> {
    match store.node(f) {
        Node::Atom(a) => Some(*a),
        _ => None,
    }
}

/// Lowers a sub-expression of an entry that has already lowered cleanly, so no new
/// diagnostic can arise. Keeping the errors on one pass is what stops a bad entry
/// being reported twice.
fn relower(body: &Expr, ctx: &Ctx, binds: &Bindings, store: &mut Store) -> FormulaId {
    let mut quiet = Diagnostics::default();
    let f = lower_formula(body, ctx.sig, ctx.consts, binds, store, &mut quiet);
    debug_assert!(quiet.is_empty(), "a sub-expression of a clean entry cannot fail to lower");
    f
}

/// Resolves an agent-name list, reporting any name that was never declared.
fn agent_ids(
    names: &[String],
    ctx: &Ctx,
    span: crate::Span,
    diags: &mut Diagnostics,
) -> Vec<AgentId> {
    let mut out = Vec::with_capacity(names.len());
    for n in names {
        match ctx.sig.agent_id(n) {
            Some(i) => out.push(i),
            None => diags.push(span, format!("`{n}` is not a declared agent")),
        }
    }
    out
}

/// Builds the initial state from a declarative block, then verifies every entry.
///
/// Facts fix the designated world's valuation; `?[a] p` entries decide which atoms
/// vary across worlds and who can tell them apart; propositional `B[a] φ` entries
/// score worlds, and `u Rᵢ v` is recorded exactly when `v` scores at least as high
/// as `u` — plausibility increases with the score, matching *"`v` is at least as
/// plausible as `u`"* (§4.1).
///
/// Every entry, whatever its shape, is re-checked against the finished state; a
/// declaration that does not hold there is reported rather than passed over.
///
/// Returns `None` if construction was impossible; diagnostics explain why.
pub fn build_declarative(
    items: &[Expr],
    ctx: &Ctx,
    store: &mut Store,
    diags: &mut Diagnostics,
) -> Option<State> {
    let n_atoms = ctx.sig.n_atoms();
    let n_agents = ctx.sig.n_agents();
    let binds = Bindings::default();
    let whole = items.first().map_or(crate::Span::new(0, 0), |e| e.span());

    let mut v0 = Bits::new(n_atoms.max(1));
    let mut uncertain: Vec<(AgentId, AtomId)> = Vec::new();
    let mut beliefs: Vec<(AgentId, FormulaId)> = Vec::new();

    // Lower every entry exactly once, up front, noting which lowered cleanly. An
    // entry that failed to lower has had its say; it drives nothing and is not
    // blamed a second time by the verification pass below.
    let lowered: Vec<(FormulaId, bool)> = items
        .iter()
        .map(|item| {
            let mark = diags.len();
            let f = lower_formula(item, ctx.sig, ctx.consts, &binds, store, diags);
            (f, diags.len() == mark)
        })
        .collect();

    // Classify. Every entry is also kept as an assertion, checked at the end.
    for (item, &(id, clean)) in items.iter().zip(lowered.iter()) {
        if !clean {
            continue;
        }
        match item {
            // Only a positive literal fixes a bit of the designated valuation. A
            // negative literal is already the default, and anything else — a negated
            // modality, a negated conjunction, a predicate that constant-folded away
            // — fixes no single bit, so it drives nothing and is left to the
            // verification pass. Rejecting those here would refuse entries that in
            // fact hold, and would be inconsistent with the un-negated shapes
            // (`p & q`, `p | q`), which are assertion-only already.
            Expr::Atom(_) | Expr::Not(_, _) => {
                if let Node::Atom(a) = store.node(id) {
                    v0.set(*a as usize);
                }
            }
            Expr::Modality { op: Modal::Ignorant, agents: Some(names), body, span, .. } => {
                let f = relower(body, ctx, &binds, store);
                match atom_of(store, f) {
                    Some(a) => {
                        uncertain.extend(
                            agent_ids(names, ctx, *span, diags).into_iter().map(|i| (i, a)),
                        );
                    }
                    None => diags.push(
                        body.span(),
                        "`?[..]` drives construction only for a single atom; \
                         write it over an atom, or state it as an assertion elsewhere",
                    ),
                }
            }
            Expr::Modality { op: Modal::Believes, agents: Some(names), cond: None, body, span } => {
                let f = relower(body, ctx, &binds, store);
                // Only propositional bodies can rank worlds before the model exists.
                if eval_prop(store, f, &v0).is_some() {
                    beliefs
                        .extend(agent_ids(names, ctx, *span, diags).into_iter().map(|i| (i, f)));
                }
            }
            _ => { /* assertion only */ }
        }
    }

    // The atoms that vary across worlds.
    let mut u_atoms: Vec<AtomId> = uncertain.iter().map(|(_, a)| *a).collect();
    u_atoms.sort_unstable();
    u_atoms.dedup();
    if u_atoms.len() > MAX_UNCERTAIN_ATOMS {
        diags.push(
            whole,
            format!(
                "{} uncertain atoms would need 2^{} worlds; the limit is {}",
                u_atoms.len(),
                u_atoms.len(),
                MAX_UNCERTAIN_ATOMS
            ),
        );
        return None;
    }

    // Worlds: every valuation agreeing with `v0` outside `u_atoms`.
    let n_worlds = 1usize << u_atoms.len();
    let mut vals: Vec<Bits> = Vec::with_capacity(n_worlds);
    let mut designated = 0usize;
    for mask in 0..n_worlds {
        let mut v = v0.clone();
        for (bit, &a) in u_atoms.iter().enumerate() {
            if (mask >> bit) & 1 == 1 {
                v.set(a as usize);
            } else {
                v.unset(a as usize);
            }
        }
        if v == v0 {
            designated = mask;
        }
        vals.push(v);
    }

    let mut model = Model::new(n_worlds, n_agents, n_atoms.max(1));
    model.val.clone_from(&vals);

    for i in 0..n_agents {
        let iu: Vec<AtomId> = uncertain
            .iter()
            .filter(|(a, _)| *a as usize == i)
            .map(|(_, at)| *at)
            .collect();
        // Score: how many of `i`'s belief declarations hold at each world.
        let score: Vec<usize> = vals
            .iter()
            .map(|v| {
                beliefs
                    .iter()
                    .filter(|(a, _)| *a as usize == i)
                    .filter(|(_, f)| eval_prop(store, *f, v) == Some(true))
                    .count()
            })
            .collect();
        for u in 0..n_worlds {
            for v in 0..n_worlds {
                // Comparable when they agree on every uncertain atom `i` can see,
                // i.e. on `U \ Uᵢ`.
                let comparable = u_atoms
                    .iter()
                    .filter(|a| !iu.contains(a))
                    .all(|&a| vals[u].get(a as usize) == vals[v].get(a as usize));
                // `v` at least as plausible as `u` exactly when it scores at least
                // as high. Swapping these arguments inverts every belief.
                if comparable && score[v] >= score[u] {
                    model.relate(i, u, v);
                }
            }
        }
    }

    if let Err(e) = model.validate() {
        diags.push(
            whole,
            format!("constructed an invalid frame: {e:?} — this is a bug in delhi-lang"),
        );
        return None;
    }

    let state = State { model, designated };

    // Verify: every entry, whatever its shape, must hold in what we built. This is
    // what makes the construction trustworthy — the scoring heuristic is not
    // obviously complete, so the result proves itself rather than being assumed.
    for (item, &(id, clean)) in items.iter().zip(lowered.iter()) {
        if clean && !state.entails(store, id) {
            diags.push(item.span(), "this declaration does not hold in the constructed state");
        }
    }

    Some(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{parse_file, Constants, Ctx, Diagnostics, Sig};
    use delhi_syntax::Store;

    fn build(src: &str) -> (Sig, Store, State) {
        let mut d = Diagnostics::default();
        let ast = parse_file(src, &mut d);
        let sig = Sig::build(&ast, &mut d);
        let consts = Constants::build(&ast, &sig, &mut d);
        let ctx = Ctx { sig: &sig, consts: &consts };
        let items = match &ast.init {
            Some(crate::ast::Init::Declarative(v, _)) => v.clone(),
            other => panic!("expected a declarative initial state, got {other:?}"),
        };
        let mut store = Store::default();
        let st = build_declarative(&items, &ctx, &mut store, &mut d)
            .unwrap_or_else(|| panic!("construction failed:\n{}", d.render(src)));
        assert!(d.is_empty(), "unexpected errors:\n{}", d.render(src));
        (sig, store, st)
    }

    const COIN: &str = r#"
        types   { Actor - Object }
        objects { alice, bob, carol - Actor }
        agents  { alice, bob, carol }
        props   { h }
        initially {
            h
            ?[carol] h
            B[carol] h
        }
        actions {}
    "#;

    #[test]
    fn reproduces_the_published_coin_lie_start_state() {
        let (sig, _, st) = build(COIN);
        assert_eq!(st.model.validate(), Ok(()), "the frame must be valid by construction");
        assert_eq!(st.model.n_worlds, 2, "one uncertain atom gives two worlds");

        let carol = sig.agent_id("carol").unwrap() as usize;
        let h = sig.atom_id("h", &[]).unwrap() as usize;
        let d = st.designated;
        assert!(st.model.val[d].get(h), "the designated world satisfies the declared fact");
        let other = if d == 0 { 1 } else { 0 };
        assert!(!st.model.val[other].get(h), "the other world is the one where h fails");

        // The published edge: the ¬h world holds the h world at least as plausible,
        // and NOT the converse. Plan 1 wrote this by hand as `relate(carol, 1, 0)`.
        assert!(st.model.rel[carol][other].get(d), "¬h world ranks the h world above it");
        assert!(!st.model.rel[carol][d].get(other), "and not the other way round");
    }

    #[test]
    fn agents_with_no_uncertainty_get_singleton_classes() {
        let (sig, _, st) = build(COIN);
        for name in ["alice", "bob"] {
            let a = sig.agent_id(name).unwrap() as usize;
            for w in 0..st.model.n_worlds {
                assert_eq!(st.model.rel[a][w].ones(), vec![w], "{name} distinguishes every world");
            }
        }
    }

    #[test]
    fn the_declared_entries_all_hold_in_the_result() {
        let (sig, mut store, st) = build(COIN);
        let h = store.atom(sig.atom_id("h", &[]).unwrap());
        let carol = sig.agent_id("carol").unwrap();
        let alice = sig.agent_id("alice").unwrap();

        assert!(st.entails(&store, h), "the fact holds");
        let ig = store.ignorant(carol, h);
        assert!(st.entails(&store, ig), "carol is uncertain");
        let bel = store.believes(carol, h);
        assert!(st.entails(&store, bel), "carol believes h");
        let k = store.knows(alice, h);
        assert!(st.entails(&store, k), "alice, having no uncertainty, knows h");
    }

    #[test]
    fn two_uncertain_atoms_give_four_worlds() {
        let (sig, _, st) = build(
            r#"
            types{ Actor - Object } objects{ a - Actor } agents{ a } props{ p, q }
            initially { p, q, ?[a] p, ?[a] q }
            actions{}
        "#,
        );
        assert_eq!(st.model.n_worlds, 4);
        assert_eq!(st.model.validate(), Ok(()));

        // A world count alone cannot tell a right world set from a wrong one of the
        // same size: pin the valuations. Every combination of `p` and `q` occurs
        // exactly once, and the designated world is the one matching the facts.
        let p = sig.atom_id("p", &[]).unwrap() as usize;
        let q = sig.atom_id("q", &[]).unwrap() as usize;
        let mut seen: Vec<(bool, bool)> =
            (0..4).map(|w| (st.model.val[w].get(p), st.model.val[w].get(q))).collect();
        seen.sort_unstable();
        assert_eq!(seen, vec![(false, false), (false, true), (true, false), (true, true)]);
        assert_eq!(
            (st.model.val[st.designated].get(p), st.model.val[st.designated].get(q)),
            (true, true),
            "the designated world is the one agreeing with the declared facts"
        );
    }

    #[test]
    fn unlisted_atoms_default_to_false() {
        let (sig, mut store, st) = build(
            r#"
            types{ Actor - Object } objects{ a - Actor } agents{ a } props{ p, q }
            initially { p }
            actions{}
        "#,
        );
        let q = store.atom(sig.atom_id("q", &[]).unwrap());
        let nq = store.not(q);
        assert!(st.entails(&store, nq));
    }

    #[test]
    fn a_declaration_that_does_not_hold_is_reported() {
        // Nothing makes `q` believed, so asserting it must fail rather than pass silently.
        let mut d = Diagnostics::default();
        let src = r#"
            types{ Actor - Object } objects{ a - Actor } agents{ a } props{ p, q }
            initially { p, ?[a] p, B[a] q }
            actions{}
        "#;
        let ast = parse_file(src, &mut d);
        let sig = Sig::build(&ast, &mut d);
        let consts = Constants::build(&ast, &sig, &mut d);
        let ctx = Ctx { sig: &sig, consts: &consts };
        let items = match &ast.init {
            Some(crate::ast::Init::Declarative(v, _)) => v.clone(),
            _ => unreachable!(),
        };
        let mut store = Store::default();
        let _ = build_declarative(&items, &ctx, &mut store, &mut d);
        assert!(
            d.items().iter().any(|x| x.message.contains("does not hold")),
            "an unsatisfiable declaration must be reported, not ignored"
        );
    }

    #[test]
    fn too_many_uncertain_atoms_is_refused_rather_than_hanging() {
        let atoms: Vec<String> = (0..25).map(|i| format!("p{i}")).collect();
        let src = format!(
            "types{{ Actor - Object }} objects{{ a - Actor }} agents{{ a }} props{{ {} }}
             initially {{ {} }} actions{{}}",
            atoms.join(", "),
            atoms.iter().map(|p| format!("?[a] {p}")).collect::<Vec<_>>().join(", ")
        );
        let mut d = Diagnostics::default();
        let ast = parse_file(&src, &mut d);
        let sig = Sig::build(&ast, &mut d);
        let consts = Constants::build(&ast, &sig, &mut d);
        let ctx = Ctx { sig: &sig, consts: &consts };
        let items = match &ast.init {
            Some(crate::ast::Init::Declarative(v, _)) => v.clone(),
            _ => unreachable!(),
        };
        let mut store = Store::default();
        let out = build_declarative(&items, &ctx, &mut store, &mut d);
        assert!(out.is_none());
        assert!(d.items().iter().any(|x| x.message.contains("uncertain")));
    }

    #[test]
    fn scoring_ranks_a_believed_world_above_an_unbelieved_one() {
        // Two agents disagree about the same uncertainty, so the direction of the
        // scoring edge is asserted for both a positive and a negative belief.
        let (sig, _, st) = build(
            r#"
            types{ Actor - Object } objects{ a, b - Actor } agents{ a, b } props{ p }
            initially { p, ?[a] p, ?[b] p, B[a] p, B[b] !p }
            actions{}
        "#,
        );
        let p = sig.atom_id("p", &[]).unwrap() as usize;
        let a = sig.agent_id("a").unwrap() as usize;
        let b = sig.agent_id("b").unwrap() as usize;
        let wp = (0..st.model.n_worlds).find(|&w| st.model.val[w].get(p)).unwrap();
        let wn = (0..st.model.n_worlds).find(|&w| !st.model.val[w].get(p)).unwrap();

        // `a` believes p: the p-world is the more plausible, so `wn Rₐ wp` only.
        assert!(st.model.rel[a][wn].get(wp), "a ranks the p-world above the ¬p-world");
        assert!(!st.model.rel[a][wp].get(wn), "and not the converse");
        // `b` believes !p: the direction is exactly reversed.
        assert!(st.model.rel[b][wp].get(wn), "b ranks the ¬p-world above the p-world");
        assert!(!st.model.rel[b][wn].get(wp), "and not the converse");
    }

    #[test]
    fn a_negated_entry_that_is_not_a_literal_is_an_assertion_not_an_error() {
        // `!B[a] q` and `!(p & q)` fix no bit of the designated valuation, so they
        // drive nothing — but both hold in what the construction builds, and an
        // entry that holds must never be reported. Only the verification pass gets
        // to judge these shapes. (The un-negated `p & q` was already assertion-only;
        // treating its negation as a hard error would be inconsistent.)
        let (_, _, _) = build(
            r#"
            types{ Actor - Object } objects{ a - Actor } agents{ a } props{ p, q }
            initially { p, ?[a] p, !B[a] q, !(p & q), !q }
            actions{}
        "#,
        );
    }

    #[test]
    fn an_entry_that_fails_to_lower_is_reported_once_and_not_chased() {
        // Each entry is lowered exactly once, so its error appears once; and an
        // entry that never lowered must not then be blamed for "not holding".
        let mut d = Diagnostics::default();
        let src = r#"
            types{ Actor - Object } objects{ a - Actor } agents{ a } props{ p }
            initially { nosuch }
            actions{}
        "#;
        let ast = parse_file(src, &mut d);
        let sig = Sig::build(&ast, &mut d);
        let consts = Constants::build(&ast, &sig, &mut d);
        let ctx = Ctx { sig: &sig, consts: &consts };
        let items = match &ast.init {
            Some(crate::ast::Init::Declarative(v, _)) => v.clone(),
            _ => unreachable!(),
        };
        let mut store = Store::default();
        let _ = build_declarative(&items, &ctx, &mut store, &mut d);
        assert_eq!(d.len(), 1, "expected exactly one diagnostic, got:\n{}", d.render(src));
        assert!(d.items()[0].message.contains("nosuch"));
    }

    #[test]
    fn a_negated_entry_that_fails_is_still_caught_by_verification() {
        // Relaxing the classifier must not lose the error: `!p` contradicts the
        // declared fact `p`, and the verification pass is what reports it.
        let mut d = Diagnostics::default();
        let src = r#"
            types{ Actor - Object } objects{ a - Actor } agents{ a } props{ p, q }
            initially { p, !(p | q) }
            actions{}
        "#;
        let ast = parse_file(src, &mut d);
        let sig = Sig::build(&ast, &mut d);
        let consts = Constants::build(&ast, &sig, &mut d);
        let ctx = Ctx { sig: &sig, consts: &consts };
        let items = match &ast.init {
            Some(crate::ast::Init::Declarative(v, _)) => v.clone(),
            _ => unreachable!(),
        };
        let mut store = Store::default();
        let _ = build_declarative(&items, &ctx, &mut store, &mut d);
        assert!(
            d.items().iter().any(|x| x.message.contains("does not hold")),
            "a contradictory entry must still be reported:\n{}",
            d.render(src)
        );
    }
}
