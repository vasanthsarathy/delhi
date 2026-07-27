//! Product update ([T] Def. 2) and the non-equivalent [MBD] variant (§4.5.1).

use crate::{ActionModel, Bits, Evaluator, Model, State};
use delhi_syntax::Store;

/// Which transition rule to use. [T] is authoritative; [MBD] exists for the
/// differential test in §4.5.1 and diverges when `e` is strictly preferred to `f`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UpdateRule {
    /// [T] Def. 2.
    Thesis,
    /// [MBD] line 129.
    MbDraft,
}

impl State {
    /// Product update under [T] Def. 2.
    pub fn apply(&self, store: &Store, am: &ActionModel) -> Option<State> {
        self.apply_with(store, am, UpdateRule::Thesis)
    }

    /// Product update under the chosen rule. Returns `None` when the action is not
    /// applicable, i.e. when the designated world satisfies no designated event's
    /// precondition, or more than one.
    pub fn apply_with(
        &self,
        store: &Store,
        am: &ActionModel,
        rule: UpdateRule,
    ) -> Option<State> {
        let m = &self.model;
        let mut ev = Evaluator::new(store, m);

        // W' = {⟨u,e⟩ | M,u ⊨ pre(e)}
        let mut pairs: Vec<(usize, usize)> = Vec::new();
        let mut index = vec![vec![usize::MAX; am.n_events]; m.n_worlds];
        for (u, row) in index.iter_mut().enumerate() {
            for (e, slot) in row.iter_mut().enumerate() {
                if ev.eval(am.pre[e], u) {
                    *slot = pairs.len();
                    pairs.push((u, e));
                }
            }
        }

        // d' = ⟨d,e⟩ for the unique designated e whose precondition holds at d.
        let mut chosen = None;
        for &e in &am.designated {
            if ev.eval(am.pre[e], self.designated) {
                if chosen.is_some() {
                    return None; // more than one: not applicable
                }
                chosen = Some(e);
            }
        }
        let d_event = chosen?;

        let n_new = pairs.len();
        let mut out = Model::new(n_new, m.n_agents, m.n_atoms);

        // V'(⟨u,e⟩) = (V(u) ∪ add(e)) \ del(e)
        for (k, &(u, e)) in pairs.iter().enumerate() {
            let mut v = m.val[u].clone();
            for &a in &am.add[e] {
                v.set(a as usize);
            }
            for &a in &am.del[e] {
                v.unset(a as usize);
            }
            out.val[k] = v;
        }

        // Q(e,f)(i) evaluated at BOTH u and v (§4.5).
        let arrow = |ev: &mut Evaluator, i: usize, e: usize, f: usize, u: usize, v: usize| {
            match am.q[i][e][f] {
                None => false,
                Some(cond) => ev.eval(cond, u) && ev.eval(cond, v),
            }
        };

        let comp: Vec<Vec<Bits>> = (0..m.n_agents).map(|i| m.comparability_rows(i)).collect();

        for (k, &(u, e)) in pairs.iter().enumerate() {
            for (l, &(v, f)) in pairs.iter().enumerate() {
                for (i, crow) in comp.iter().enumerate() {
                    let ef = arrow(&mut ev, i, e, f, u, v);
                    let fe = arrow(&mut ev, i, f, e, u, v);
                    let sim = crow[u].get(v);
                    let ruv = m.rel[i][u].get(v);
                    let rvu = m.rel[i][v].get(u);
                    let related = match rule {
                        UpdateRule::Thesis => ef && ((!fe && sim) || (fe && ruv)),
                        UpdateRule::MbDraft => (ruv && (ef || fe)) || (rvu && ef && !fe),
                    };
                    if related {
                        out.rel[i][k].set(l);
                    }
                }
            }
        }

        // Def. 2 yields reflexivity by construction: the implicit `Q(e,e)(i) = ⊤` self-edge
        // makes both arrow tests true, and the second disjunct then needs only `u Rᵢ u`,
        // which the source frame guarantees. Assert it rather than forcing it — forcing
        // would mask a defect in the rule above.
        #[cfg(debug_assertions)]
        for i in 0..m.n_agents {
            for k in 0..n_new {
                debug_assert!(
                    out.rel[i][k].get(k),
                    "product update lost reflexivity at agent {i}, world {k}"
                );
            }
        }

        let designated = index[self.designated][d_event];
        debug_assert_ne!(designated, usize::MAX);
        Some(State { model: out, designated })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{build, ActionDef, Kind, Model, State};
    use delhi_syntax::Store;

    /// Coin Lie s0: agents 0=A, 1=B, 2=C; atom 0 = h. Edge `v R_C u`.
    fn s0() -> State {
        let mut m = Model::new(2, 3, 1);
        m.val[0].set(0);
        m.relate(2, 1, 0);
        State { model: m, designated: 0 }
    }

    #[test]
    fn announcing_not_heads_reverses_cs_arrow() {
        // [T] Figs 5.4 → 5.6. The lie flips C from believing h to believing !h.
        let mut s = Store::default();
        let t = s.tru();
        let h = s.atom(0);
        let nh = s.not(h);
        let act = ActionDef {
            name: "announce_not_heads".into(),
            pre: t,
            kind: Kind::Announce(nh),
            observes: vec![(0, t), (1, t), (2, t)],
            aware: vec![],
        };
        let am = build(&act, &mut s, 3);
        let s1 = s0().apply(&s, &am).expect("action is applicable");

        assert_eq!(s1.model.validate(), Ok(()), "frame conditions must survive update");

        // A and B still know h — the lie contradicts what they know, so it is rejected.
        let ka = s.knows(0, h);
        let kb = s.knows(1, h);
        assert!(s1.entails(&s, ka));
        assert!(s1.entails(&s, kb));

        // C now wrongly believes !h, without knowing it.
        let bc = s.believes(2, nh);
        let kc = s.knows(2, nh);
        assert!(s1.entails(&s, bc));
        assert!(!s1.entails(&s, kc));

        // And A knows the lie landed.
        let inner = s.believes(2, nh);
        let ka_b = s.knows(0, inner);
        assert!(s1.entails(&s, ka_b));
    }

    #[test]
    fn the_two_update_rules_diverge_on_coin_lie() {
        // §4.5.1: the rules agree in three of four event-comparability configurations
        // and diverge when `e` is strictly preferred to `f`. Coin Lie REACHES that
        // case, so this is the differential test the design doc asks for — the doc's
        // claim that the rules agree on every worked example is false.
        //
        // At agent C, from <world 1, e^psi> to <world 0, e^!psi>:
        //   e->f is PN = bottom (C fully observes)  => false
        //   f->e is FPN = top                        => true
        //   u ~ v and u R v hold; v R u does not
        // Thesis : both disjuncts need e->f, so the edge is ABSENT — C keeps a strict
        //          preference and comes to believe the announcement.
        // MbDraft: the first disjunct (u R v AND (e->f OR f->e)) fires, so the edge is
        //          present both ways — the worlds become equiplausible and the lie
        //          fails to land. This is exactly the "destroys a belief [T] retains"
        //          failure §4.5.1 predicts.
        let mut s = Store::default();
        let t = s.tru();
        let h = s.atom(0);
        let nh = s.not(h);
        let act = ActionDef {
            name: "announce_not_heads".into(),
            pre: t,
            kind: Kind::Announce(nh),
            observes: vec![(0, t), (1, t), (2, t)],
            aware: vec![],
        };
        let am = build(&act, &mut s, 3);
        let thesis = s0().apply_with(&s, &am, UpdateRule::Thesis).unwrap();
        let draft = s0().apply_with(&s, &am, UpdateRule::MbDraft).unwrap();

        assert!(
            !thesis.equivalent(&draft),
            "Coin Lie reaches the divergent configuration, so the rules must differ"
        );

        // The concrete consequence: under [T] the lie lands; under [MBD] it does not.
        let b_c_nh = s.believes(2, nh);
        assert!(thesis.entails(&s, b_c_nh), "[T]: C comes to believe the announcement");
        assert!(!draft.entails(&s, b_c_nh), "[MBD]: the belief is destroyed");
    }
}
