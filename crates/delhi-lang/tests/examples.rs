//! Every file in `examples/` runs its documented trace and reaches its documented
//! conclusion.
//!
//! These exist so the examples and the README cannot drift from what the code does.
//! Each test asserts the *headline* claim of its file — the sentence a reader would
//! quote — not merely that the file parses.
//!
//! `coin_lie.delhi` is covered separately and more thoroughly by `coin_lie_file.rs`,
//! which matches it assertion-for-assertion against the API-driven reference.

use delhi_lang::Problem;
use delhi_mb::State;
use delhi_syntax::FormulaId;

/// Parses `src`, applies each named action in order, and returns the final state.
///
/// Panics with the diagnostic text if the file is rejected, and with the action name
/// if one is unknown or inapplicable — either is a broken example, not a soft failure.
fn run(src: &str, actions: &[&str]) -> Problem {
    let mut p = Problem::parse(src).unwrap_or_else(|e| panic!("{e}"));
    let n_agents = p.sig.n_agents();
    let mut state = p.state.clone();
    for name in actions {
        let g = p
            .actions
            .iter()
            .find(|a| &a.name == name)
            .unwrap_or_else(|| panic!("no action `{name}`"));
        let def = g.def.clone();
        let model = delhi_mb::build(&def, &mut p.store, n_agents);
        state = state
            .apply(&p.store, &model)
            .unwrap_or_else(|| panic!("`{name}` was not applicable"));
    }
    p.state = state;
    p
}

/// Lowers a formula against an already-checked problem, so tests read as the surface
/// language rather than as store calls.
fn q(p: &mut Problem, text: &str) -> FormulaId {
    delhi_lang::lower_formula(
        &delhi_lang::Parser::new(&delhi_lang::lex(text, &mut delhi_lang::Diagnostics::default()))
            .parse_expr(&mut delhi_lang::Diagnostics::default()),
        &p.sig,
        &p.consts,
        &delhi_lang::Bindings::default(),
        &mut p.store,
        &mut delhi_lang::Diagnostics::default(),
    )
}

/// Asserts each `(formula, expected)` pair in the problem's current state.
fn expect(p: &mut Problem, cases: &[(&str, bool)]) {
    let state: State = p.state.clone();
    for (text, want) in cases {
        let f = q(p, text);
        assert_eq!(
            state.entails(&p.store, f),
            *want,
            "expected `{text}` to be {want}"
        );
    }
}

#[test]
fn sally_anne_looks_in_the_basket() {
    // The false-belief task: Sally looks where she *believes* the marble is, not
    // where it is. `B[anne] B[sally] basket` is Anne passing the task herself.
    let mut p = run(
        include_str!("../../../examples/sally_anne.delhi"),
        &["sally_leaves()", "anne_moves()", "sally_returns()"],
    );
    expect(
        &mut p,
        &[
            ("box", true),                     // the marble really is in the box
            ("basket", false),                 //
            ("B[sally] basket", true),         // ...but that is where she will look
            ("B[sally] box", false),           //
            ("K[anne] box", true),             // anne saw it
            ("B[anne] B[sally] basket", true), // and models sally's mistake
        ],
    );
    let goal = p.goal.expect("the file declares a goal");
    assert!(p.state.entails(&p.store, goal), "the declared goal holds");
}

#[test]
fn the_ice_cream_van_produces_a_second_order_false_belief() {
    // John is right about the van and wrong about Mary's mind — which is the
    // distinction the second-order task exists to draw.
    let mut p = run(
        include_str!("../../../examples/ice_cream_van.delhi"),
        &["mary_goes_home()", "van_moves_to_church()", "driver_tells_mary()"],
    );
    expect(
        &mut p,
        &[
            ("at_park", false),
            ("K[mary] !at_park", true),          // mary was told
            ("B[john] !at_park", true),          // john watched it leave
            ("B[john] B[mary] at_park", true),   // ...and is wrong about her
            ("B[mary] B[john] !at_park", true),  // while she has him right
        ],
    );
    let goal = p.goal.expect("the file declares a goal");
    assert!(p.state.entails(&p.store, goal), "the declared goal holds");
}

#[test]
fn the_bicycle_lie_lands_and_then_loses_to_the_evidence() {
    let src = include_str!("../../../examples/bicycle.delhi");

    // Before anything: belief without knowledge.
    let mut p = run(src, &[]);
    expect(&mut p, &[("B[theo] !broken", true), ("K[theo] !broken", false)]);

    // The lie reorders his plausibility, but confers no knowledge — and crucially
    // is *not* safe belief, since a true announcement can dislodge it.
    let mut p = run(src, &["mira_lies()"]);
    expect(
        &mut p,
        &[
            ("B[theo] broken", true),
            ("K[theo] broken", false),
            ("[][theo] broken", false),
        ],
    );

    // Looking reorders it back, and this time he knows.
    let mut p = run(src, &["mira_lies()", "theo_looks()"]);
    expect(&mut p, &[("K[theo] !broken", true), ("B[theo] !broken", true)]);
    let goal = p.goal.expect("the file declares a goal");
    assert!(p.state.entails(&p.store, goal), "the declared goal holds");
}

#[test]
fn coin_in_the_box_separates_seeing_from_hearing() {
    // The benchmark's point: bob does not learn the coin, but does learn that alice
    // learned it. Those are different epistemic positions and `aware` is what
    // distinguishes them.
    let mut p = run(
        include_str!("../../../examples/coin_in_the_box.delhi"),
        &["open_box()", "peek(alice)"],
    );
    expect(
        &mut p,
        &[
            ("K[alice] tail", true),
            ("Kw[bob] tail", false),
            ("K[bob] Kw[alice] tail", true),
            ("Kw[carol] tail", false),
            ("K[carol] Kw[alice] tail", true),
        ],
    );
    let goal = p.goal.expect("the file declares a goal");
    assert!(p.state.entails(&p.store, goal), "the declared goal holds");
}

#[test]
fn coin_in_the_box_grounds_a_peek_per_actor_without_class_overlap() {
    // `?p observes` beside `?o aware if !same(?p, ?o)` is the shape that forced the
    // ⊥-conditioned observer clause to be dropped rather than recorded. If it were
    // recorded, every `peek` would be rejected for putting one agent in both classes.
    let p = Problem::parse(include_str!("../../../examples/coin_in_the_box.delhi"))
        .unwrap_or_else(|e| panic!("{e}"));
    for who in ["alice", "bob", "carol"] {
        let g = p
            .action(&format!("peek({who})"))
            .unwrap_or_else(|| panic!("peek({who}) should ground"));
        assert_eq!(g.def.observes.len(), 1, "peek({who}): only the peeker sees");
        assert_eq!(g.def.aware.len(), 2, "peek({who}): the other two hear");
    }
}

#[test]
fn muddy_children_conclude_on_the_third_round_and_not_before() {
    let src = include_str!("../../../examples/muddy_children.delhi");

    // Each child knows the others from the start, and herself not at all.
    let mut p = run(src, &[]);
    expect(
        &mut p,
        &[("K[alice] muddy(bob)", true), ("Bw[alice] muddy(alice)", false)],
    );

    // The father's announcement alone settles nothing, and neither does one round.
    let mut p = run(src, &["father_speaks()", "nobody_knows()"]);
    expect(&mut p, &[("Bw[alice] muddy(alice)", false)]);

    // Two rounds of silence, and all three conclude together.
    let mut p = run(src, &["father_speaks()", "nobody_knows()", "nobody_knows()"]);
    expect(
        &mut p,
        &[
            ("B[alice] muddy(alice)", true),
            ("B[bob] muddy(bob)", true),
            ("B[carol] muddy(carol)", true),
            // ...as belief, never as knowledge. An announcement reorders plausibility
            // rather than deleting worlds, because in this language it might be a lie.
            // The file documents why this departs from the textbook account.
            ("Kw[alice] muddy(alice)", false),
        ],
    );
    let goal = p.goal.expect("the file declares a goal");
    assert!(p.state.entails(&p.store, goal), "the declared goal holds");
}
