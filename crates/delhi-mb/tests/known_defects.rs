//! Tests that currently FAIL by design. Each pins a documented limitation so that
//! a future fix has an acceptance criterion. Run with `cargo test -- --ignored`.

use delhi_mb::{build, ActionDef, Kind, Model, State};
use delhi_syntax::Store;

/// §4.7(a) — [T] §5.3's announcement defect. A full observer `j` should end up knowing
/// that a partial observer `i` believes *whether* φ, not *that* φ.
#[test]
#[ignore = "known defect: [T] §5.3. Un-ignore when the θ/τ split lands (§4.7(a))."]
fn announcement_does_not_overinform_full_observers() {
    let mut s = Store::default();
    let t = s.tru();
    let p = s.atom(0);

    // Two worlds so that φ is genuinely open. Agent 0 = j (full), 1 = i (partial).
    let mut m = Model::new(2, 2, 1);
    m.val[0].set(0);
    m.relate(0, 0, 1);
    m.relate(0, 1, 0);
    m.relate(1, 0, 1);
    m.relate(1, 1, 0);
    assert_eq!(m.validate(), Ok(()));
    let st = State { model: m, designated: 0 };

    let act = ActionDef {
        name: "announce_p".into(),
        pre: t,
        kind: Kind::Announce(p),
        observes: vec![(0, t)],
        aware: vec![(1, t)],
    };
    let am = build(&act, &mut s, 2);
    let out = st.apply(&s, &am).expect("applicable");

    let bw = s.believes_whether(1, p);
    let k_j_bw = s.knows(0, bw);
    assert!(out.entails(&s, k_j_bw), "j must know that i believes WHETHER p");

    let b_i_p = s.believes(1, p);
    let k_j_b_i_p = s.knows(0, b_i_p);
    assert!(
        !out.entails(&s, k_j_b_i_p),
        "j must NOT know that i believes THAT p — this is the [T] §5.3 defect"
    );
}

/// §4.8 — mB has no hypothetical actions ([KR21] eq. 23). Bicycle-3: M is oblivious to
/// whether T looked or played, so M must not come to know `!p`.
#[test]
#[ignore = "known gap: mB lacks hypothetical actions ([KR21] §7). See §4.8."]
fn oblivious_agent_does_not_learn_that_the_other_action_did_not_happen() {
    let mut s = Store::default();
    let t = s.tru();
    let b = s.atom(0); // bicycle in the basement
    let p = s.atom(1); // T is playing

    // Agents: 0 = M (mother), 1 = T (Timmy). M is oblivious to tim_look.
    let mut m = Model::new(2, 2, 2);
    m.val[0].set(0);
    m.relate(0, 0, 1);
    m.relate(0, 1, 0);
    m.relate(1, 0, 1);
    m.relate(1, 1, 0);
    assert_eq!(m.validate(), Ok(()));
    let st = State { model: m, designated: 0 };

    // T looks; M observes nothing. tim_play (which would cause p) is never applied.
    let look = build(
        &ActionDef {
            name: "tim_look".into(),
            pre: t,
            kind: Kind::Sensing(b),
            observes: vec![(1, t)],
            aware: vec![],
        },
        &mut s,
        2,
    );
    let out = st.apply(&s, &look).expect("applicable");

    let np = s.not(p);
    let k_m_np = s.knows(0, np);
    assert!(
        !out.entails(&s, k_m_np),
        "M must not know !p — she cannot rule out that T played instead"
    );
    // tim_play, for reference: the hypothetical alternative action M cannot rule out.
    // Effect { lits: vec![(1, true)], cond: t } — never applied in this scenario.
}
