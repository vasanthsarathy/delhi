//! The gate: the same trace as Plan 1's `delhi-mb/tests/coin_lie.rs`, driven from a
//! text file rather than the Rust API. Divergence means the front-end is wrong.

use delhi_lang::Problem;

const SRC: &str = include_str!("../../../examples/coin_lie.delhi");

#[test]
fn coin_lie_from_a_file_matches_the_api_driven_trace() {
    let mut p = Problem::parse(SRC).unwrap_or_else(|e| panic!("{e}"));

    let s = &mut p.store;
    let h = s.atom(p.sig.atom_id("h", &[]).expect("atom h"));
    let d = s.atom(p.sig.atom_id("d", &[]).expect("atom d"));
    let nh = s.not(h);
    let (a, b, c) = (
        p.sig.agent_id("alice").unwrap(),
        p.sig.agent_id("bob").unwrap(),
        p.sig.agent_id("carol").unwrap(),
    );

    let ka = s.knows(a, h);
    let kb = s.knows(b, h);
    let kc = s.knows(c, h);
    let ig_c = s.ignorant(c, h);
    let b_c_h = s.believes(c, h);
    let b_c_nh = s.believes(c, nh);
    let k_c_nh = s.knows(c, nh);
    let k_a_b_c = s.knows(a, b_c_nh);
    let b_a_b_c = s.believes(a, b_c_nh);
    let kw_a = s.knows_whether(a, h);
    let ck = s.common(0b111, kw_a);

    // ---- s0 ----
    let s0 = p.state.clone();
    assert_eq!(s0.model.validate(), Ok(()));
    assert!(s0.entails(&p.store, ka), "s0: A knows h");
    assert!(s0.entails(&p.store, kb), "s0: B knows h");
    assert!(s0.entails(&p.store, ig_c), "s0: C does not know which way");
    assert!(s0.entails(&p.store, b_c_h), "s0: C correctly BELIEVES h — pins the arrow direction");
    assert!(s0.entails(&p.store, ck), "s0: common knowledge that A knows whether h");

    // ---- s1 ----
    // Clone the definition and build the event model in its own statement: `p.action`
    // borrows the whole problem, and `build` needs `&mut p.store`.
    let announce = p.action("announce_not_heads()").expect("action").def.clone();
    let am = delhi_mb::build(&announce, &mut p.store, 3);
    let s1 = s0.apply(&p.store, &am).expect("applicable");
    assert_eq!(s1.model.validate(), Ok(()));
    assert!(s1.entails(&p.store, b_c_nh), "s1: the lie worked");
    assert!(!s1.entails(&p.store, k_c_nh), "s1: but C does not KNOW it");
    assert!(s1.entails(&p.store, ka), "s1: the lie contradicts what A knows, so it is rejected");
    assert!(s1.entails(&p.store, kb), "s1: same for B");
    assert!(s1.entails(&p.store, k_a_b_c), "s1: A knows her lie landed");

    // ---- s2 ----
    let distract = p.action("distract_a()").expect("action").def.clone();
    let dm = delhi_mb::build(&distract, &mut p.store, 3);
    let s2 = s1.apply(&p.store, &dm).expect("applicable");
    assert_eq!(s2.model.validate(), Ok(()));
    assert!(s2.entails(&p.store, d), "s2: A is distracted");

    // ---- s3 ----
    let peek = p.action("peek_c()").expect("action").def.clone();
    let pm = delhi_mb::build(&peek, &mut p.store, 3);
    let s3 = s2.apply(&p.store, &pm).expect("applicable");
    assert_eq!(s3.model.validate(), Ok(()));
    assert!(
        s3.entails(&p.store, ka) && s3.entails(&p.store, kb) && s3.entails(&p.store, kc),
        "s3: everyone now knows h"
    );
    assert!(s3.entails(&p.store, b_a_b_c), "s3: A still believes C believes !h");

    // The declared goal is exactly the second-order false belief.
    //
    // Pin it structurally, not just by satisfaction. `Store` is hash-consed, so `mk`
    // returns the existing id for an identical node and `FormulaId` equality *is*
    // structural identity. Satisfaction alone is far too weak here: both conjuncts are
    // already asserted a few lines above, so a goal that folded to `⊤`, or that dropped
    // `& K[carol] h`, would still be entailed by s3 and pass unnoticed. The two
    // assertions check different things, so keep both.
    let goal = p.goal.expect("the file declares a goal");
    let want = {
        let s = &mut p.store;
        s.and(b_a_b_c, kc) // the reference's `sofb`
    };
    assert_eq!(goal, want, "the declared goal is exactly the second-order false belief");
    assert!(s3.entails(&p.store, goal), "s3 satisfies the declared goal");
}

#[test]
fn every_declared_action_grounds_to_exactly_one_ground_action() {
    let p = Problem::parse(SRC).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(p.actions.len(), 3, "three declarations, none parameterised");
}

#[test]
fn load_reads_the_example_from_disk_and_entails_queries_the_initial_state() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/coin_lie.delhi");
    let mut p = delhi_lang::load(path).unwrap_or_else(|e| panic!("{e}"));
    let h = p.store.atom(p.sig.atom_id("h", &[]).expect("atom h"));
    let a = p.sig.agent_id("alice").unwrap();
    let c = p.sig.agent_id("carol").unwrap();
    let ka = p.store.knows(a, h);
    let kc = p.store.knows(c, h);
    // `entails` must answer against the initial state, so it has to separate the
    // agent who knows from the one who does not — a constant `true` would not.
    assert!(p.entails(ka), "s0 models K[alice] h");
    assert!(!p.entails(kc), "s0 does not model K[carol] h");
}

#[test]
fn load_reports_a_missing_file_with_its_path() {
    let err = delhi_lang::load("no/such/file.delhi").unwrap_err();
    assert!(err.contains("no/such/file.delhi"), "the path must name the failure, got: {err}");
}

#[test]
fn a_file_with_errors_reports_them_all_with_spans() {
    let err =
        Problem::parse("types{} objects{} agents{ ghost } props{} initially{ nope } actions{}")
            .unwrap_err();
    assert!(err.contains("ghost"), "undeclared agent reported");
    assert!(err.contains("nope"), "unknown proposition reported");
    assert!(err.contains("1:"), "diagnostics carry line:col — this is all on line 1");
    assert!(err.contains('^'), "and a caret under the offending span");
}
