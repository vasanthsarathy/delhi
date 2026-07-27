//! [T] §5.2.5, Figs 5.4–5.10 — the Coin Lie scenario, as traced in spec §8.5.
//! Agents: 0 = A, 1 = B, 2 = C. Atoms: 0 = h, 1 = d.

use delhi_mb::{build, ActionDef, ActionModel, Effect, Kind, Model, State};
use delhi_syntax::Store;

fn s0() -> State {
    // [T] Fig. 5.4: worlds u (h) and v; one edge `v ──C──► u`.
    let mut m = Model::new(2, 3, 2);
    m.val[0].set(0);
    m.relate(2, 1, 0);
    assert_eq!(m.validate(), Ok(()));
    State { model: m, designated: 0 }
}

struct Actions {
    announce: ActionModel,
    distract: ActionModel,
    peek: ActionModel,
}

fn actions(s: &mut Store) -> Actions {
    let t = s.tru();
    let h = s.atom(0);
    let d = s.atom(1);
    let nh = s.not(h);
    let nd = s.not(d);

    let announce = build(
        &ActionDef {
            name: "announce_not_heads".into(),
            pre: t,
            kind: Kind::Announce(nh),
            observes: vec![(0, t), (1, t), (2, t)],
            aware: vec![],
        },
        s,
        3,
    );
    let distract = build(
        &ActionDef {
            name: "distract_a".into(),
            pre: t,
            kind: Kind::Ontic(vec![Effect { lits: vec![(1, true)], cond: t }]),
            observes: vec![(0, t), (1, t), (2, t)],
            aware: vec![],
        },
        s,
        3,
    );
    let peek = build(
        &ActionDef {
            name: "peek_c".into(),
            pre: t,
            kind: Kind::Sensing(h),
            observes: vec![(2, t)],
            aware: vec![(1, t), (0, nd)], // A notices only if she is not distracted
        },
        s,
        3,
    );
    Actions { announce, distract, peek }
}

#[test]
fn coin_lie_full_trace() {
    let mut s = Store::default();
    let acts = actions(&mut s);
    let h = s.atom(0);
    let d = s.atom(1);
    let nh = s.not(h);

    // ---- s0 ----
    let st0 = s0();
    let ka = s.knows(0, h);
    let kb = s.knows(1, h);
    let ig_c = s.ignorant(2, h);
    let kw_a = s.knows_whether(0, h);
    let ck = s.common(0b111, kw_a);
    assert!(st0.entails(&s, ka), "s0: A knows h");
    assert!(st0.entails(&s, kb), "s0: B knows h");
    assert!(st0.entails(&s, ig_c), "s0: C does not know which way");
    assert!(st0.entails(&s, ck), "s0: common knowledge that A knows whether h");

    // ---- s1 ----
    let st1 = st0.apply(&s, &acts.announce).expect("announce applicable");
    assert_eq!(st1.model.validate(), Ok(()));
    let b_c_nh = s.believes(2, nh);
    let k_c_nh = s.knows(2, nh);
    let k_a_b_c = s.knows(0, b_c_nh);
    assert!(st1.entails(&s, b_c_nh), "s1: the lie worked, C believes !h");
    assert!(!st1.entails(&s, k_c_nh), "s1: but C does not know it");
    assert!(st1.entails(&s, ka), "s1: A's knowledge is untouched");
    assert!(st1.entails(&s, kb), "s1: B's knowledge is untouched");
    assert!(st1.entails(&s, k_a_b_c), "s1: A knows her lie landed");

    // ---- s2 ----
    let st2 = st1.apply(&s, &acts.distract).expect("distract applicable");
    assert_eq!(st2.model.validate(), Ok(()));
    assert!(st2.entails(&s, d), "s2: A is distracted");

    // ---- s3 ----
    let st3 = st2.apply(&s, &acts.peek).expect("peek applicable");
    assert_eq!(st3.model.validate(), Ok(()));
    let kc = s.knows(2, h);
    assert!(st3.entails(&s, ka), "s3: A knows h");
    assert!(st3.entails(&s, kb), "s3: B knows h");
    assert!(st3.entails(&s, kc), "s3: C now knows h");

    // The payoff: second-order false belief.
    let b_a_b_c = s.believes(0, b_c_nh);
    assert!(st3.entails(&s, b_a_b_c), "s3: A still believes C believes !h");
    let sofb = s.and(b_a_b_c, kc);
    assert!(st3.entails(&s, sofb), "s3: and A is wrong — C knows h");
}
