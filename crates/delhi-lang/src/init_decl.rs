//! The declarative `initially` construction (§7.3).

use crate::ast::{Expr, Modal};
use crate::lower_formula::{lower_formula, Bindings};
use crate::{Ctx, Diagnostics};
use delhi_mb::{Bits, Model, State};
use delhi_syntax::{AgentId, AtomId, FormulaId, Node, Store};

/// Refuse rather than build a state this construction could not finish.
///
/// `n` uncertain atoms give `2ⁿ` worlds, and the cost is not the worlds themselves
/// but what is quadratic in them: the comparability-and-score loop runs `2ⁿ × 2ⁿ`
/// times with an inner walk over the uncertain atoms, `Model::validate` is worse
/// still, and `rel` holds `n_agents × 2ⁿ` rows of `2ⁿ` bits. At 12 that is 4096
/// worlds, ~17M iterations and ~2 MB of relation per agent — quick. Each further
/// atom quadruples the loop: 16 would mean 65536 worlds, ~4.3 billion iterations
/// and ~0.5 GB per agent, which does not complete, so a limit set there would
/// refuse only inputs that were already hopeless while admitting ones that hang.
const MAX_UNCERTAIN_ATOMS: usize = 12;

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

/// Whether `f` is free of modal operators, and so can be judged against a valuation
/// alone.
///
/// This is a structural question, deliberately separate from [`eval_prop`]: asking
/// whether `eval_prop` returned a value is *not* a purity test, because `&&`
/// short-circuits, so a false left conjunct hides whatever modality sits on the
/// right. Answering it here means the classification cannot depend on the valuation
/// — least of all on the half-built one it would have during classification.
fn is_propositional(store: &Store, f: FormulaId) -> bool {
    match store.node(f) {
        Node::True | Node::Atom(_) => true,
        Node::Not(g) => is_propositional(store, *g),
        Node::And(a, b) => is_propositional(store, *a) && is_propositional(store, *b),
        _ => false,
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
///
/// `block` spans the whole `initially { … }` block. Failures that are properties of the
/// block rather than of any one entry — the uncertainty limit, and a frame this
/// construction should never have produced — are reported against it; per-entry
/// failures keep the span of their entry.
pub fn build_declarative(
    items: &[Expr],
    block: crate::Span,
    ctx: &Ctx,
    store: &mut Store,
    diags: &mut Diagnostics,
) -> Option<State> {
    let n_atoms = ctx.sig.n_atoms();
    let n_agents = ctx.sig.n_agents();
    let binds = Bindings::default();

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

    // Which entries the construction actually acted on. An entry that drove nothing
    // and then fails verification is a limit of the construction, not necessarily an
    // error by the author, and the diagnostic has to say so.
    let mut drove = vec![false; items.len()];

    // Classify. Every entry is also kept as an assertion, checked at the end.
    for (k, (item, &(id, clean))) in items.iter().zip(lowered.iter()).enumerate() {
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
            Expr::Atom(_) | Expr::Not(_, _) => match store.node(id) {
                Node::Atom(a) => {
                    let a = *a;
                    v0.set(a as usize);
                    drove[k] = true;
                }
                // A negative literal drives too: it confirms the default.
                Node::Not(inner) => drove[k] = atom_of(store, *inner).is_some(),
                _ => {}
            },
            Expr::Modality { op: Modal::Ignorant, agents: Some(names), body, span, .. } => {
                let f = relower(body, ctx, &binds, store);
                match atom_of(store, f) {
                    Some(a) => {
                        drove[k] = true;
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
                if is_propositional(store, f) {
                    drove[k] = true;
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
            block,
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
            block,
            format!("constructed an invalid frame: {e:?} — this is a bug in delhi-lang"),
        );
        return None;
    }

    let state = State { model, designated };

    // Verify: every entry, whatever its shape, must hold in what we built. This is
    // what makes the construction trustworthy — the scoring heuristic is not
    // obviously complete, so the result proves itself rather than being assumed.
    for (k, (item, &(id, clean))) in items.iter().zip(lowered.iter()).enumerate() {
        if clean && !state.entails(store, id) {
            let mut msg = String::from("this declaration does not hold in the constructed state");
            if !drove[k] {
                msg.push_str(
                    "; entries of this shape do not drive the construction — only bare \
                     literals, `?[a] p`, and propositional `B[a] φ` do",
                );
            }
            diags.push(item.span(), msg);
        }
    }

    Some(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{parse_file, Constants, Ctx, Diagnostics, Sig};
    use delhi_syntax::Store;

    /// Runs the construction and hands back whatever it produced, diagnostics and
    /// all. Tests that expect errors use this; [`build`] wraps it for the rest.
    fn raw(src: &str) -> (Sig, Store, Option<State>, Diagnostics) {
        let mut d = Diagnostics::default();
        let ast = parse_file(src, &mut d);
        let sig = Sig::build(&ast, &mut d);
        let consts = Constants::build(&ast, &sig, &mut d);
        let ctx = Ctx { sig: &sig, consts: &consts };
        let (items, block) = match &ast.init {
            Some(crate::ast::Init::Declarative(v, block)) => (v.clone(), *block),
            other => panic!("expected a declarative initial state, got {other:?}"),
        };
        let mut store = Store::default();
        let st = build_declarative(&items, block, &ctx, &mut store, &mut d);
        (sig, store, st, d)
    }

    fn build(src: &str) -> (Sig, Store, State) {
        let (sig, store, st, d) = raw(src);
        let st = st.unwrap_or_else(|| panic!("construction failed:\n{}", d.render(src)));
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
        let (items, block) = match &ast.init {
            Some(crate::ast::Init::Declarative(v, block)) => (v.clone(), *block),
            _ => unreachable!(),
        };
        let mut store = Store::default();
        let _ = build_declarative(&items, block, &ctx, &mut store, &mut d);
        assert!(
            d.items().iter().any(|x| x.message.contains("does not hold")),
            "an unsatisfiable declaration must be reported, not ignored"
        );
    }

    /// `initially { ?[a] p0, ?[a] p1, … }` over `n` atoms, and the diagnostics it draws.
    fn uncertain_over(n: usize) -> Diagnostics {
        let atoms: Vec<String> = (0..n).map(|i| format!("p{i}")).collect();
        let src = format!(
            "types{{ Actor - Object }} objects{{ a - Actor }} agents{{ a }} props{{ {} }}
             initially {{ {} }} actions{{}}",
            atoms.join(", "),
            atoms.iter().map(|p| format!("?[a] {p}")).collect::<Vec<_>>().join(", ")
        );
        raw(&src).3
    }

    #[test]
    fn the_uncertainty_limit_refuses_a_size_that_would_not_finish() {
        // The bound has to sit where the construction is still quick, not merely
        // where it still fits in memory: the relation loop is quadratic in worlds,
        // so 13 uncertain atoms is 8192 worlds and ~67M iterations, and each further
        // atom quadruples that. 16 — a plausible-looking "memory" limit — would
        // permit 65536 worlds, ~4.3 billion iterations, and ~0.5 GB of `rel` per
        // agent, so the refusal test below would pass while a *permitted* input hung.
        let d = uncertain_over(13);
        assert!(
            d.items().iter().any(|x| x.message.contains("uncertain")),
            "13 uncertain atoms must be refused"
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
        let (items, block) = match &ast.init {
            Some(crate::ast::Init::Declarative(v, block)) => (v.clone(), *block),
            _ => unreachable!(),
        };
        let mut store = Store::default();
        let out = build_declarative(&items, block, &ctx, &mut store, &mut d);
        assert!(out.is_none());
        assert!(d.items().iter().any(|x| x.message.contains("uncertain")));
    }

    #[test]
    fn a_whole_block_refusal_is_blamed_on_the_block_not_its_first_entry() {
        // The uncertainty limit is a property of the block, not of whichever entry
        // happens to be written first, and the caret has to say so. Anchoring it to
        // `items.first()` pointed at one arbitrary declaration — here `?[a] p0`, on
        // the line after the keyword — and at byte zero of the file for a block with
        // no entries at all.
        let atoms: Vec<String> = (0..13).map(|i| format!("p{i}")).collect();
        let src = format!(
            "types{{ Actor - Object }} objects{{ a - Actor }} agents{{ a }} props{{ {} }}\n\
             initially {{\n{}\n}}\nactions{{}}",
            atoms.join(", "),
            atoms.iter().map(|p| format!("  ?[a] {p}")).collect::<Vec<_>>().join(",\n"),
        );
        let d = raw(&src).3;
        let limit = d
            .items()
            .iter()
            .find(|x| x.message.contains("uncertain"))
            .expect("the limit must be refused");
        // `initially` is the first token on line 2; the entries begin on line 3.
        assert_eq!(
            (limit.span.start, limit.span.end),
            (src.find("initially").unwrap(), src.rfind("}\nactions").unwrap() + 1),
            "the refusal must span the whole `initially` block:\n{}",
            d.render(&src)
        );
        assert!(d.render(&src).contains("2:1"), "and render at the keyword:\n{}", d.render(&src));
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
        // Two agents declare uncertainty about the *same* atom, so `U` must be
        // deduplicated: one atom, two worlds. Without the dedup this is four
        // worlds, two of each valuation, and every assertion below still holds.
        assert_eq!(st.model.n_worlds, 2, "one distinct uncertain atom gives two worlds");

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
    fn a_failing_entry_says_whether_the_construction_even_tried() {
        // Both entries below fail verification, but for different reasons, and the
        // author cannot act on either unless the two are told apart. `B[a] q` is a
        // shape the construction scores, so it genuinely does not hold. `p | q` is
        // a shape the construction never attempts, so its failure says nothing
        // about the author's intent — only that nothing was built to satisfy it.
        let (_, _, st, d) = raw(r#"
            types{ Actor - Object } objects{ a - Actor } agents{ a } props{ p, q }
            initially { ?[a] p, B[a] q, p | q }
            actions{}
        "#);
        assert!(st.is_some());
        let msgs: Vec<&str> = d.items().iter().map(|x| x.message.as_str()).collect();
        assert_eq!(msgs.len(), 2, "expected both entries reported, got {msgs:?}");

        let drove = msgs.iter().find(|m| !m.contains("do not drive")).expect("B[a] q drove");
        assert!(drove.contains("does not hold"));

        let did_not = msgs.iter().find(|m| m.contains("do not drive")).expect("`p | q` did not");
        assert!(did_not.contains("does not hold"), "still says what went wrong");
        assert!(
            did_not.contains("bare literals") && did_not.contains("`?[a] p`"),
            "and says what would drive the construction: {did_not}"
        );
    }

    #[test]
    fn a_modal_belief_body_never_ranks_worlds_whatever_the_entry_order() {
        // `B[a] (p | B[b] q)` has a modal body, so per §7.3 it must drive nothing.
        // Deciding that by asking whether `eval_prop` returned a value gets it
        // wrong: `|` desugars through `&`, whose `&&` short-circuits, so once `p`
        // is already true in the half-built `v0` the modal conjunct is never
        // visited and the entry is admitted as a scoring clause. The frame then
        // depends on whether the fact `p` was written before or after the belief.
        const BEFORE: &str = r#"
            types{ Actor - Object } objects{ a, b - Actor } agents{ a, b } props{ p, q }
            initially { p, ?[a] p, B[a] (p | B[b] q) }
            actions{}
        "#;
        const AFTER: &str = r#"
            types{ Actor - Object } objects{ a, b - Actor } agents{ a, b } props{ p, q }
            initially { ?[a] p, B[a] (p | B[b] q), p }
            actions{}
        "#;
        let (sig, _, before, _) = raw(BEFORE);
        let (_, _, after, _) = raw(AFTER);
        let before = before.expect("construction must still succeed");
        let after = after.expect("construction must still succeed");
        assert_eq!(before.model.rel, after.model.rel, "entry order must not change the frame");

        // And nothing was ranked: `a`'s two worlds stay mutually plausible.
        let a = sig.agent_id("a").unwrap() as usize;
        assert!(before.model.rel[a][0].get(1) && before.model.rel[a][1].get(0));
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
        let (items, block) = match &ast.init {
            Some(crate::ast::Init::Declarative(v, block)) => (v.clone(), *block),
            _ => unreachable!(),
        };
        let mut store = Store::default();
        let _ = build_declarative(&items, block, &ctx, &mut store, &mut d);
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
        let (items, block) = match &ast.init {
            Some(crate::ast::Init::Declarative(v, block)) => (v.clone(), *block),
            _ => unreachable!(),
        };
        let mut store = Store::default();
        let _ = build_declarative(&items, block, &ctx, &mut store, &mut d);
        assert!(
            d.items().iter().any(|x| x.message.contains("does not hold")),
            "a contradictory entry must still be reported:\n{}",
            d.render(src)
        );
    }
}
