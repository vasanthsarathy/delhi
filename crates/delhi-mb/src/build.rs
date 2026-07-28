//! Compiling an `ActionDef` into an action plausibility model (§4.6).

use crate::{ActionDef, Kind};
use delhi_syntax::{AgentId, AtomId, FormulaId, Store};

/// Event index of `e^φ` in announcement and sensing models.
pub const EV_PHI: usize = 0;
/// Event index of `e^¬φ` in announcement and sensing models.
pub const EV_NPHI: usize = 1;
/// Event index of `e^⊤` in announcement and sensing models.
pub const EV_TOP: usize = 2;

/// An action plausibility model `⟨E, Q, pre, add, del, Γ⟩` (§4.4).
#[derive(Clone, Debug)]
pub struct ActionModel {
    /// `|E|`.
    pub n_events: usize,
    /// `pre(e)`. Typed `L^P_GB` — modal, per the §4.4 typing correction.
    pub pre: Vec<FormulaId>,
    /// `add(e)`.
    pub add: Vec<Vec<AtomId>>,
    /// `del(e)`.
    pub del: Vec<Vec<AtomId>>,
    /// `q[agent][from][to]`; `None` means `⊥`.
    pub q: Vec<Vec<Vec<Option<FormulaId>>>>,
    /// `Γ`.
    pub designated: Vec<usize>,
}

fn observes_disj(a: &ActionDef, s: &mut Store, i: AgentId) -> FormulaId {
    let parts: Vec<FormulaId> =
        a.observes.iter().filter(|(j, _)| *j == i).map(|(_, f)| *f).collect();
    s.any(&parts)
}

fn aware_disj(a: &ActionDef, s: &mut Store, i: AgentId) -> FormulaId {
    let parts: Vec<FormulaId> = a.aware.iter().filter(|(j, _)| *j == i).map(|(_, f)| *f).collect();
    s.any(&parts)
}

/// `PN(i) = ¬⋁ observes-conditions`.
fn pn(a: &ActionDef, s: &mut Store, i: AgentId) -> FormulaId {
    let o = observes_disj(a, s, i);
    s.not(o)
}

/// `N(i) = ¬((⋁ observes) | (⋁ aware))`.
fn n_label(a: &ActionDef, s: &mut Store, i: AgentId) -> FormulaId {
    let o = observes_disj(a, s, i);
    let w = aware_disj(a, s, i);
    let d = s.or(o, w);
    s.not(d)
}

fn empty_q(n_agents: usize, n_events: usize, top: FormulaId) -> Vec<Vec<Vec<Option<FormulaId>>>> {
    let mut q = vec![vec![vec![None; n_events]; n_events]; n_agents];
    for agent in q.iter_mut() {
        for (e, row) in agent.iter_mut().enumerate() {
            row[e] = Some(top); // implicit reflexive FPN edge (§4.4)
        }
    }
    q
}

/// Compiles an action definition into its action model (§4.6).
///
/// # Panics
/// If any agent id in `a.observes` or `a.aware` is `>= n_agents`.
pub fn build(a: &ActionDef, s: &mut Store, n_agents: usize) -> ActionModel {
    debug_assert!(
        a.observes.iter().chain(a.aware.iter()).all(|(i, _)| (*i as usize) < n_agents),
        "build: observer agent id out of range"
    );
    let top = s.tru();
    match &a.kind {
        Kind::Announce(psi) | Kind::Sensing(psi) => {
            let announcing = matches!(a.kind, Kind::Announce(_));
            let npsi = s.not(*psi);
            let pre_phi = s.and(a.pre, *psi);
            let pre_nphi = s.and(a.pre, npsi);
            let mut q = empty_q(n_agents, 3, top);
            for i in 0..n_agents as AgentId {
                let p = pn(a, s, i);
                let nn = n_label(a, s, i);
                q[i as usize][EV_PHI][EV_NPHI] = Some(p);
                q[i as usize][EV_NPHI][EV_PHI] = Some(if announcing { top } else { p });
                q[i as usize][EV_PHI][EV_TOP] = Some(nn);
                q[i as usize][EV_NPHI][EV_TOP] = Some(nn);
            }
            ActionModel {
                n_events: 3,
                pre: vec![pre_phi, pre_nphi, top],
                add: vec![vec![], vec![], vec![]],
                del: vec![vec![], vec![], vec![]],
                q,
                designated: vec![EV_PHI, EV_NPHI],
            }
        }
        Kind::Ontic(effects) => {
            // One event per realisable outcome, with mutually exclusive preconditions,
            // so exactly one designated event can fire at the designated world (§4.6).
            // k counts only conditional effects (cond ≠ ⊤); unconditional effects
            // always apply and don't contribute to the outcome split.
            let k = effects.iter().filter(|e| e.cond != top).count();
            let n_outcomes = 1usize << k;
            let mut pre = Vec::with_capacity(n_outcomes + 1);
            let mut add = Vec::with_capacity(n_outcomes + 1);
            let mut del = Vec::with_capacity(n_outcomes + 1);
            for mask in 0..n_outcomes {
                let mut guards = vec![a.pre];
                let mut adds = Vec::new();
                let mut dels = Vec::new();
                let mut cond_bit = 0;
                for e in effects.iter() {
                    if e.cond == top {
                        // Unconditional: always apply.
                        guards.push(top);
                        for (atom, sign) in &e.lits {
                            if *sign {
                                adds.push(*atom);
                            } else {
                                dels.push(*atom);
                            }
                        }
                    } else {
                        // Conditional: check mask bit.
                        if mask >> cond_bit & 1 == 1 {
                            guards.push(e.cond);
                            for (atom, sign) in &e.lits {
                                if *sign {
                                    adds.push(*atom);
                                } else {
                                    dels.push(*atom);
                                }
                            }
                        } else {
                            let ncond = s.not(e.cond);
                            guards.push(ncond);
                        }
                        cond_bit += 1;
                    }
                }
                pre.push(s.all(&guards));
                adds.sort_unstable();
                adds.dedup();
                dels.sort_unstable();
                dels.dedup();
                add.push(adds);
                del.push(dels);
            }
            let ev_top = n_outcomes;
            pre.push(top);
            add.push(vec![]);
            del.push(vec![]);

            let mut q = empty_q(n_agents, n_outcomes + 1, top);
            for i in 0..n_agents as AgentId {
                let nn = n_label(a, s, i);
                for e in 0..n_outcomes {
                    q[i as usize][e][ev_top] = Some(nn);
                }
            }
            ActionModel {
                n_events: n_outcomes + 1,
                pre,
                add,
                del,
                q,
                designated: (0..n_outcomes).collect(),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ActionDef, Effect, Kind};
    use delhi_syntax::Store;

    #[test]
    fn announcement_has_three_events_and_two_designated() {
        // [T] Def. 3.
        let mut s = Store::default();
        let t = s.tru();
        let h = s.atom(0);
        let nh = s.not(h);
        let a = ActionDef {
            name: "announce_not_heads".into(),
            pre: t,
            kind: Kind::Announce(nh),
            observes: vec![(0, t), (1, t), (2, t)],
            aware: vec![],
        };
        let am = build(&a, &mut s, 3);
        assert_eq!(am.n_events, 3);
        assert_eq!(am.designated, vec![0, 1]);
    }

    #[test]
    fn sensing_differs_from_announcement_only_in_one_edge() {
        // [T] Fig. 5.2: `⟨e^¬φ, e^φ⟩` is PN for sensing, FPN for announcement.
        let mut s = Store::default();
        let t = s.tru();
        let h = s.atom(0);
        let base = |kind| ActionDef {
            name: "x".into(),
            pre: t,
            kind,
            observes: vec![(0, t)],
            aware: vec![],
        };
        let ann = build(&base(Kind::Announce(h)), &mut s, 1);
        let sen = build(&base(Kind::Sensing(h)), &mut s, 1);
        assert_eq!(ann.q[0][EV_PHI][EV_NPHI], sen.q[0][EV_PHI][EV_NPHI], "both PN");
        assert_ne!(ann.q[0][EV_NPHI][EV_PHI], sen.q[0][EV_NPHI][EV_PHI]);
        // FPN is ⊤ for the announcement.
        assert_eq!(ann.q[0][EV_NPHI][EV_PHI], Some(t));
    }

    #[test]
    fn unconditional_ontic_action_has_two_events() {
        // [T] Def. 4.
        let mut s = Store::default();
        let t = s.tru();
        let a = ActionDef {
            name: "distract_a".into(),
            pre: t,
            kind: Kind::Ontic(vec![Effect { lits: vec![(0, true)], cond: t }]),
            observes: vec![(0, t)],
            aware: vec![],
        };
        let am = build(&a, &mut s, 1);
        assert_eq!(am.n_events, 2);
        assert_eq!(am.add[0], vec![0]);
        assert!(am.del[0].is_empty());
        assert_eq!(am.designated, vec![0]);
    }

    #[test]
    fn conditional_effects_split_into_one_event_per_outcome() {
        // §4.6: two conditional effects ⇒ 2² outcome events plus e^⊤.
        let mut s = Store::default();
        let t = s.tru();
        let broken = s.atom(2);
        let ok = s.not(broken);
        let a = ActionDef {
            name: "flip_switch".into(),
            pre: t,
            kind: Kind::Ontic(vec![
                Effect { lits: vec![(0, true)], cond: ok },
                Effect { lits: vec![(1, true)], cond: broken },
            ]),
            observes: vec![(0, t)],
            aware: vec![],
        };
        let am = build(&a, &mut s, 1);
        assert_eq!(am.n_events, 5, "4 outcome events + e^top");
        assert_eq!(am.designated.len(), 4);
    }

    #[test]
    fn mixed_conditional_and_unconditional_effects_split_only_on_the_conditional_ones() {
        // One unconditional effect and one conditional: 2^1 outcomes + e^top = 3 events.
        // The unconditional literal must appear in BOTH outcome events' add sets.
        let mut s = Store::default();
        let t = s.tru();
        let guard = s.atom(2);
        let a = ActionDef {
            name: "mixed".into(),
            pre: t,
            kind: Kind::Ontic(vec![
                Effect { lits: vec![(0, true)], cond: t }, // unconditional
                Effect { lits: vec![(1, true)], cond: guard }, // conditional
            ]),
            observes: vec![(0, t)],
            aware: vec![],
        };
        let am = build(&a, &mut s, 1);
        assert_eq!(am.n_events, 3, "2 outcomes + e^top");
        assert_eq!(am.designated.len(), 2);
        for &e in &am.designated {
            assert!(am.add[e].contains(&0), "unconditional effect must apply in every outcome");
        }
    }
}
