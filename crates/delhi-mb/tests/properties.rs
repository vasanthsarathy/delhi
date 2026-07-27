//! §9 L3/L4. Frame preservation, the KB bridge axioms, and algebraic laws.

use delhi_mb::{build, ActionDef, Effect, Kind, Model, State};
use delhi_syntax::Store;
use proptest::prelude::*;

/// Builds a valid locally-well-preordered model by construction: assign each world a
/// comparability class and a level, then relate within a class by level order (§9).
fn model_strategy(n: usize, agents: usize, atoms: usize) -> impl Strategy<Value = Model> {
    let per_agent = prop::collection::vec((0..n, 0..n), n);
    (
        prop::collection::vec(per_agent, agents),
        prop::collection::vec(prop::collection::vec(any::<bool>(), atoms), n),
    )
        .prop_map(move |(agent_specs, vals)| {
            let mut m = Model::new(n, agents, atoms);
            for (w, bits) in vals.iter().enumerate() {
                for (a, &on) in bits.iter().enumerate() {
                    if on {
                        m.val[w].set(a);
                    }
                }
            }
            for (i, spec) in agent_specs.iter().enumerate() {
                for u in 0..n {
                    for v in 0..n {
                        let (cu, lu) = spec[u];
                        let (cv, lv) = spec[v];
                        if cu == cv && lu <= lv {
                            m.relate(i, u, v);
                        }
                    }
                }
            }
            m
        })
}

proptest! {
    #[test]
    fn generated_frames_are_valid(m in model_strategy(5, 2, 2)) {
        prop_assert_eq!(m.validate(), Ok(()));
    }

    /// KB1: `Belᵢ ⊆ ~ᵢ`, hence `K[i]φ → B[i]φ` (§3.5, §9).
    #[test]
    fn kb1_holds_by_construction(m in model_strategy(5, 2, 2)) {
        let d = delhi_mb::Derived::of(&m);
        for i in 0..m.n_agents {
            for u in 0..m.n_worlds {
                prop_assert!(d.comp[i][u].contains_all(&d.bel[i][u]));
            }
        }
    }

    /// KB2: `u ~ᵢ v` and `w ∈ →ᵢᵛ` imply `w ∈ →ᵢᵘ` (§3.5).
    #[test]
    fn kb2_holds_by_construction(m in model_strategy(5, 2, 2)) {
        let d = delhi_mb::Derived::of(&m);
        for i in 0..m.n_agents {
            for u in 0..m.n_worlds {
                for v in d.comp[i][u].ones() {
                    prop_assert_eq!(d.bel[i][u].clone(), d.bel[i][v].clone());
                }
            }
        }
    }

    /// Belief is serial: `→ᵢᵘ` is never empty (§4.1's corrected precondition).
    #[test]
    fn belief_is_serial(m in model_strategy(5, 2, 2)) {
        let d = delhi_mb::Derived::of(&m);
        for i in 0..m.n_agents {
            for u in 0..m.n_worlds {
                prop_assert!(!d.bel[i][u].is_empty());
            }
        }
    }

    /// §6.1.2: `~R ⊆ ~D`, with no exceptions. This is the soundness regression.
    #[test]
    fn tilde_r_never_merges_more_than_tilde_d(m in model_strategy(5, 2, 2)) {
        let br = delhi_mb::blocks_dynamic(&m);
        let bd = delhi_mb::blocks_full(&m);
        for u in 0..m.n_worlds {
            for v in 0..m.n_worlds {
                if br[u] == br[v] {
                    prop_assert_eq!(bd[u], bd[v]);
                }
            }
        }
    }

    /// §4.2: `B[i]φ ≡ B^⊤[i]φ`.
    #[test]
    fn conditional_belief_on_top_is_belief(m in model_strategy(4, 2, 2)) {
        let st = State { model: m, designated: 0 };
        let mut s = Store::default();
        let p = s.atom(0);
        let t = s.tru();
        let b = s.believes(0, p);
        let cb = s.cond_bel(0, t, p);
        prop_assert_eq!(st.entails(&s, b), st.entails(&s, cb));
    }

    /// §9 L4: entailment is two-valued.
    #[test]
    fn bivalence(m in model_strategy(4, 2, 2)) {
        let st = State { model: m, designated: 0 };
        let mut s = Store::default();
        let p = s.atom(0);
        let k = s.knows(0, p);
        let nk = s.not(k);
        prop_assert_ne!(st.entails(&s, k), st.entails(&s, nk));
    }

    /// [T] §9.2.1: product update preserves the frame conditions.
    #[test]
    fn update_preserves_frame_conditions(m in model_strategy(4, 2, 2)) {
        let st = State { model: m, designated: 0 };
        let mut s = Store::default();
        let t = s.tru();
        let p = s.atom(0);
        let am = build(
            &ActionDef {
                name: "sense_p".into(),
                pre: t,
                kind: Kind::Sensing(p),
                observes: vec![(0, t)],
                aware: vec![(1, t)],
            },
            &mut s,
            2,
        );
        if let Some(out) = st.apply(&s, &am) {
            prop_assert_eq!(out.model.validate(), Ok(()));
        }
    }

    /// [T] Prop. 5.2.5: `a causes l` ⇒ the result entails `l`.
    #[test]
    fn ontic_effects_update_the_state(m in model_strategy(4, 2, 2)) {
        let st = State { model: m, designated: 0 };
        let mut s = Store::default();
        let t = s.tru();
        let am = build(
            &ActionDef {
                name: "set_p".into(),
                pre: t,
                kind: Kind::Ontic(vec![Effect { lits: vec![(0, true)], cond: t }]),
                observes: vec![(0, t), (1, t)],
                aware: vec![],
            },
            &mut s,
            2,
        );
        if let Some(out) = st.apply(&s, &am) {
            let p = s.atom(0);
            prop_assert!(out.entails(&s, p));
        }
    }

    /// Contraction preserves entailment, and keys are sound in both directions (§9 L4).
    #[test]
    fn contraction_and_keys_agree(m in model_strategy(4, 2, 2)) {
        let st = State { model: m.clone(), designated: 0 };
        let (cm, map) = m.contract_full();
        let ct = State { model: cm, designated: map[0] as usize };
        let mut s = Store::default();
        let p = s.atom(0);
        let k = s.knows(0, p);
        prop_assert_eq!(st.entails(&s, k), ct.entails(&s, k));
        prop_assert!(st.equivalent(&ct));
        prop_assert_eq!(st.key(), ct.key());
    }
}
