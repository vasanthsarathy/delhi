//! What a state *means*, as data: every agent's attitude to every proposition.
//!
//! [`print_state`](crate::print_state) renders the model — worlds and plausibility edges
//! — which is exact and round-trips through the parser, but leaves the reader to work out
//! what any of it implies. This is the complementary view.
//!
//! It lives here rather than in a front end because two of them need it: the CLI prints
//! it as text and the web UI renders it as HTML. Returning structured data rather than a
//! formatted string is what lets both do that without either growing its own copy of the
//! logic.

use crate::Problem;
use delhi_mb::State;

/// One agent's stance on every proposition, each landing in exactly one list.
///
/// The three cases are the ones the attitude table distinguishes, collapsed by polarity:
/// an agent that knows `¬p` has `!p` in `knows`, and one that believes `p` without
/// knowing it has `p` in `believes`. `undecided` is the remainder — no lean either way.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AgentView {
    /// The agent's name.
    pub agent: String,
    /// Signed propositions it knows, e.g. `h`, `!d`.
    pub knows: Vec<String>,
    /// Signed propositions it believes without knowing.
    pub believes: Vec<String>,
    /// Propositions it has no view on, unsigned.
    pub undecided: Vec<String>,
}

/// The actual world's facts, and each agent's attitudes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StateView {
    /// Signed propositions true in the designated world, e.g. `h`, `!d`.
    pub facts: Vec<String>,
    /// One entry per declared agent, in declaration order.
    pub agents: Vec<AgentView>,
}

/// Computes the attitude of every agent to every proposition in `state`.
///
/// First-order by construction: one entry per agent, one attitude per proposition.
/// Nested attitudes do not fit that shape and are not reported — evaluate a formula
/// instead.
///
/// Takes `&mut Problem` because the queries have to be interned into its store, and
/// takes `state` separately so a front end can pass a state it has stepped forward
/// rather than the problem's initial one.
pub fn state_view(p: &mut Problem, state: &State) -> StateView {
    let n_atoms = p.sig.n_atoms();
    let n_agents = p.sig.n_agents();

    let names: Vec<String> = (0..n_atoms).map(|a| p.sig.atom_name(a as u32).to_string()).collect();
    let agent_names: Vec<String> =
        (0..n_agents).map(|i| p.sig.agent_name(i as u32).to_string()).collect();

    let signed = |a: usize, positive: bool| {
        if positive {
            names[a].clone()
        } else {
            format!("!{}", names[a])
        }
    };

    let facts = (0..n_atoms)
        .map(|a| signed(a, state.model.val[state.designated].get(a)))
        .collect();

    let mut agents = Vec::with_capacity(n_agents);
    for (i, agent) in agent_names.iter().enumerate() {
        // Every query is interned first: `entails` borrows the store immutably while
        // `knows`/`believes` need it mutably, so the two phases cannot interleave.
        let mut queries = Vec::with_capacity(n_atoms);
        for a in 0..n_atoms {
            let atom = p.store.atom(a as u32);
            let neg = p.store.not(atom);
            let kp = p.store.knows(i as u32, atom);
            let kn = p.store.knows(i as u32, neg);
            let bp = p.store.believes(i as u32, atom);
            let bn = p.store.believes(i as u32, neg);
            queries.push((kp, kn, bp, bn));
        }

        let mut view = AgentView {
            agent: agent.clone(),
            knows: Vec::new(),
            believes: Vec::new(),
            undecided: Vec::new(),
        };
        for (a, (kp, kn, bp, bn)) in queries.into_iter().enumerate() {
            if state.entails(&p.store, kp) {
                view.knows.push(signed(a, true));
            } else if state.entails(&p.store, kn) {
                view.knows.push(signed(a, false));
            } else if state.entails(&p.store, bp) {
                view.believes.push(signed(a, true));
            } else if state.entails(&p.store, bn) {
                view.believes.push(signed(a, false));
            } else {
                view.undecided.push(names[a].clone());
            }
        }
        agents.push(view);
    }

    StateView { facts, agents }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn view(src: &str) -> StateView {
        let mut p = Problem::parse(src).unwrap_or_else(|e| panic!("{e}"));
        let state = p.state.clone();
        state_view(&mut p, &state)
    }

    const COIN: &str = r#"
        types{ Actor - Object } objects{ a, b - Actor } agents{ a, b } props{ h }
        initially { h, ?[a] h, B[a] h }
        actions {}
    "#;

    #[test]
    fn knowing_and_merely_believing_land_in_different_lists() {
        // `b` knows h outright; `a` only leans that way. A version reporting both as
        // known would still look plausible, so the distinction is asserted directly.
        let v = view(COIN);
        assert_eq!(v.facts, vec!["h"]);
        assert_eq!(v.agents[0].agent, "a");
        assert_eq!(v.agents[0].believes, vec!["h"]);
        assert!(v.agents[0].knows.is_empty(), "a does not know h");
        assert_eq!(v.agents[1].agent, "b");
        assert_eq!(v.agents[1].knows, vec!["h"]);
        assert!(v.agents[1].believes.is_empty());
    }

    #[test]
    fn a_flat_plausibility_order_reads_as_undecided() {
        // Uncertainty with no belief declaration leaves both worlds maximal, so the
        // agent believes neither h nor !h. Without this branch such a proposition would
        // vanish from the report entirely rather than being called out.
        let v = view(r#"
            types{ Actor - Object } objects{ a - Actor } agents{ a } props{ h }
            initially { h, ?[a] h }
            actions {}
        "#);
        assert_eq!(v.agents[0].undecided, vec!["h"]);
        assert!(v.agents[0].knows.is_empty() && v.agents[0].believes.is_empty());
    }

    #[test]
    fn a_false_proposition_is_reported_negated_not_omitted() {
        // `d` is false everywhere, so both the facts line and the attitudes must carry
        // `!d`. Reporting only the true propositions would make a state that knows a lot
        // look like one that knows little.
        let v = view(r#"
            types{ Actor - Object } objects{ a - Actor } agents{ a } props{ h, d }
            initially { h }
            actions {}
        "#);
        assert!(v.facts.contains(&"!d".to_string()), "got {:?}", v.facts);
        assert!(v.agents[0].knows.contains(&"!d".to_string()), "got {:?}", v.agents[0]);
    }

    #[test]
    fn every_proposition_lands_in_exactly_one_list() {
        // The three lists partition the propositions. If a case were dropped or
        // double-counted the totals would not add up, and no single-scenario assertion
        // above would necessarily notice.
        for src in [COIN, include_str!("../../../examples/muddy_children.delhi")] {
            let mut p = Problem::parse(src).unwrap_or_else(|e| panic!("{e}"));
            let n = p.sig.n_atoms();
            let state = p.state.clone();
            let v = state_view(&mut p, &state);
            assert_eq!(v.facts.len(), n);
            for a in &v.agents {
                assert_eq!(
                    a.knows.len() + a.believes.len() + a.undecided.len(),
                    n,
                    "{}'s attitudes must cover every proposition once: {a:?}",
                    a.agent
                );
            }
        }
    }
}
