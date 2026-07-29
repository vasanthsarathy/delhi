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
        state =
            state.apply(&p.store, &model).unwrap_or_else(|| panic!("`{name}` was not applicable"));
    }
    p.state = state;
    p
}

/// Lowers a formula against an already-checked problem, so tests read as the surface
/// language rather than as store calls.
///
/// Two things here are load-bearing, and an earlier version had neither. It **expands
/// definitions**, so a test may use a `define` name exactly as the file and the prompt
/// do. And it **panics on any diagnostic** rather than discarding them: lowering returns
/// `⊥` on failure, so a mistyped or unexpanded name would quietly read as `false` and
/// make a `("…", false)` assertion pass for entirely the wrong reason.
fn q(p: &mut Problem, text: &str) -> FormulaId {
    let mut diags = delhi_lang::Diagnostics::default();
    let toks = delhi_lang::lex(text, &mut diags);
    let expr = delhi_lang::Parser::new(&toks).parse_expr(&mut diags);
    let expr = delhi_lang::expand(&expr, &p.defs, &mut diags);
    let f = delhi_lang::lower_formula(
        &expr,
        &p.sig,
        &p.consts,
        &delhi_lang::Bindings::default(),
        &mut p.store,
        &mut diags,
    );
    assert!(diags.is_empty(), "`{text}` did not lower:\n{}", diags.render(text));
    f
}

/// Asserts each `(formula, expected)` pair in the problem's current state.
fn expect(p: &mut Problem, cases: &[(&str, bool)]) {
    let state: State = p.state.clone();
    for (text, want) in cases {
        let f = q(p, text);
        assert_eq!(state.entails(&p.store, f), *want, "expected `{text}` to be {want}");
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
            ("K[mary] !at_park", true),         // mary was told
            ("B[john] !at_park", true),         // john watched it leave
            ("B[john] B[mary] at_park", true),  // ...and is wrong about her
            ("B[mary] B[john] !at_park", true), // while she has him right
        ],
    );
    let goal = p.goal.expect("the file declares a goal");
    assert!(p.state.entails(&p.store, goal), "the declared goal holds");
}

#[test]
fn sally_anne_second_order_makes_anne_wrong_about_being_seen() {
    // The variant where nobody misses an event: Sally watches, and Anne's mistake is
    // about *observability itself*. Anne's most plausible worlds have `watching`
    // false, so in those worlds she computes Sally as oblivious.
    let mut p =
        run(include_str!("../../../examples/sally_anne_second_order.delhi"), &["anne_moves()"]);
    expect(
        &mut p,
        &[
            ("box", true),
            ("K[sally] box", true),            // she watched it happen
            ("K[anne] box", true),             // and anne moved it herself
            ("B[anne] B[sally] basket", true), // yet expects sally to look in the basket
            ("B[anne] B[sally] box", false),
        ],
    );
    let goal = p.goal.expect("the file declares a goal");
    assert!(p.state.entails(&p.store, goal), "the declared goal holds");
}

#[test]
fn the_birthday_bicycle_revises_timmy_and_leaves_his_mother_wrong() {
    let src = include_str!("../../../examples/bicycle.delhi");

    // Before anything: belief without knowledge.
    let mut p = run(src, &[]);
    expect(&mut p, &[("B[timmy] !bicycle", true), ("K[timmy] !bicycle", false)]);

    // The lie holds his belief in place but confers no knowledge — and crucially is
    // not *safe* belief, since a true announcement can dislodge it.
    let mut p = run(src, &["mom_tells_him_no()"]);
    expect(
        &mut p,
        &[("B[timmy] !bicycle", true), ("K[timmy] !bicycle", false), ("[][timmy] !bicycle", false)],
    );

    // Looking reorders it back — and his mother, who never saw him go, is left
    // holding a second-order false belief.
    let mut p = run(src, &["mom_tells_him_no()", "timmy_looks_in_the_basement()"]);
    expect(
        &mut p,
        &[
            ("K[timmy] bicycle", true),
            ("K[mom] bicycle", true), // she hid it; she is not confused
            ("B[mom] B[timmy] !bicycle", true), // ...about the bicycle
        ],
    );
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
fn the_three_observer_classes_are_three_distinct_epistemic_positions() {
    // The test above shows `observes` against `aware`, but in Coin in the Box every
    // non-peeker is aware, so it never shows `aware` against *oblivious*. That is the
    // half that makes three classes rather than two, and this pins it: one action, one
    // sensed proposition, three agents, three different answers.
    //
    // `distract_a()` first, because `peek_c` declares `alice aware if !d` — with `d`
    // true that clause drops and alice is left out of the action entirely.
    //
    // Every assertion below is false in the initial state and only the peek makes it
    // true, except alice's pair, which is the point: the *same* action that gives bob
    // `K[bob] Kw[carol] h` leaves alice without it. Note what is deliberately absent —
    // `Kw[bob] h` is already true before anything happens, because Coin Lie declares
    // only carol uncertain, so asserting it here would test the file's initial state
    // rather than what `aware` does.
    let mut p =
        run(include_str!("../../../examples/coin_lie.delhi"), &["distract_a()", "peek_c()"]);
    expect(
        &mut p,
        &[
            // carol observes: she learns the value itself.
            ("Kw[carol] h", true),
            // bob is aware: he knows the peek happened, so he knows carol settled it —
            // without seeing which way she settled it.
            ("K[bob] Kw[carol] h", true),
            // alice is oblivious, having been distracted. She does not learn the value,
            // and — the part `aware` alone would not tell you — she does not learn that
            // anyone else did either. Before the peek she knew carol could not tell;
            // now she cannot even say that.
            ("K[alice] Kw[carol] h", false),
            ("?[alice] Kw[carol] h", true),
        ],
    );
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
fn selective_communication_reaches_its_third_order_goals() {
    // SC_3_4 from the EFP suite. `a` senses q at position 2, then shouts from a
    // position whose audience is everyone — three actions for two third-order goals.
    let mut p = run(
        include_str!("../../../examples/selective_communication.delhi"),
        &["right()", "sense()", "shout_2()"],
    );
    expect(&mut p, &[("B[a] q", true), ("B[a] B[c] B[a] q", true), ("B[c] B[a] B[c] q", true)]);
    let goal = p.goal.expect("the file declares a goal");
    assert!(p.state.entails(&p.store, goal), "the declared goal holds");
}

#[test]
fn grapevine_grounds_to_the_originals_action_count() {
    // The port replaces 24 hand-enumerated actions with two parameterised ones. That is
    // only faithful if it grounds to the same 24 — 3 actors x 3 secrets x 2 rooms for
    // `share`, and 3 actors x 2 ordered distinct room pairs for `move`. The six
    // same-room groundings must be pruned, not merely rejected later: `move(a,r1,r1)`
    // would both add and delete `at(a,r1)`.
    let p = Problem::parse(include_str!("../../../examples/grapevine.delhi"))
        .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(p.actions.len(), 24, "18 share + 6 move");
    assert!(p.action("move(a,r1,r1)").is_none(), "same-room moves must be pruned");
    assert!(p.action("move(a,r1,r2)").is_some());
    assert!(p.action("share(b,b,r1)").is_some());
}

#[test]
fn grapevine_spreads_a_secret_to_one_agent_and_not_another() {
    // Two actions: c steps out, b tells the room. The negative conjunct is the point —
    // it is not enough to spread information, you have to withhold it too, and then
    // know that you withheld it.
    let mut p =
        run(include_str!("../../../examples/grapevine.delhi"), &["move(c,r1,r2)", "share(b,b,r1)"]);
    expect(
        &mut p,
        &[
            ("B[a] secret(b)", true),            // a hears it
            ("B[c] secret(b)", false),           // c was out of the room
            ("B[a] !B[c] secret(b)", true),      // a knows c missed it
            ("B[b] B[a] !B[c] secret(b)", true), // and b knows that a knows
        ],
    );
    let goal = p.goal.expect("the file declares a goal");
    assert!(p.state.entails(&p.store, goal), "the declared goal holds");
}

#[test]
fn reachability_derives_a_closure_that_prunes_impossible_actions() {
    // Rules, definitions and invariants in one file. The map is static, so `reach` is a
    // parse-time fixpoint that folds away: it never becomes a proposition, and the
    // `walk` groundings whose `adjacent` guard folded to false are never built.
    let mut p = run(include_str!("../../../examples/reachability.delhi"), &[]);
    assert_eq!(p.sig.n_atoms(), 4, "only at(Actor, Room) expands into atoms");
    assert_eq!(p.actions.len(), 2, "10 of 12 walk groundings are impossible");
    assert!(p.action("walk(alice,hall,study)").is_some());
    assert!(p.action("walk(alice,hall,cellar)").is_none(), "not adjacent");

    expect(
        &mut p,
        &[
            ("reach(hall, study)", true),    // one step
            ("reach(hall, attic)", true),    // two — only the recursive rule gives this
            ("reach(hall, cellar)", false),  // no path
            ("reach(attic, hall)", false),   // not symmetric
            ("can_get(alice, attic)", true), // a definition over a derived predicate
            ("can_get(alice, cellar)", false),
        ],
    );
    let goal = p.goal.expect("the file declares a goal");
    assert!(p.state.entails(&p.store, goal), "the declared goal holds");
    assert!(p.violated(&p.state).is_empty(), "and the invariant holds");
}

#[test]
fn muddy_children_conclude_on_the_third_round_and_not_before() {
    let src = include_str!("../../../examples/muddy_children.delhi");

    // Each child knows the others from the start, and herself not at all.
    let mut p = run(src, &[]);
    expect(&mut p, &[("K[alice] muddy(bob)", true), ("Bw[alice] muddy(alice)", false)]);

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

#[test]
fn safe_belief_separates_being_right_from_merely_believing() {
    // The headline claim: plain belief cannot tell ada and ben apart — both hold their
    // view equally firmly and neither knows — and safe belief separates them at once,
    // because it is measured from the actual world rather than from the top of the
    // agent's own ordering.
    let mut p = run(include_str!("../../../examples/safe_belief.delhi"), &[]);
    expect(
        &mut p,
        &[
            // Indistinguishable at the level of belief.
            ("B[ada] up", true),
            ("B[ben] !up", true),
            ("Kw[ada] up", false),
            ("Kw[ben] up", false),
            // Told apart by safe belief. `[]` is factive, so ben's cannot be safe.
            ("[][ada] up", true),
            ("[][ben] !up", false),
            // Cleo leans neither way, so both worlds sit above the actual one and
            // nothing non-trivial is safe for her — a third distinct position.
            ("B[cleo] up", false),
            ("[][cleo] up", false),
            // An agent cannot certify its own safe belief: `K[i] [][i] p` collapses to
            // `K[i] p`. It *believes* the belief is safe either way — ben included, and
            // ben is wrong.
            ("K[ada] [][ada] up", false),
            ("B[ada] [][ada] up", true),
            ("B[ben] [][ben] !up", true),
            ("[][ben] [][ben] !up", false),
        ],
    );
}

#[test]
fn a_true_announcement_makes_a_belief_safe_and_a_lie_never_can() {
    // The three acquisition routes, which is what the chapter on safe belief turns on.
    let mut told = run(include_str!("../../../examples/safe_belief.delhi"), &["gossip()"]);
    expect(
        &mut told,
        &[
            // Ben was wrong; a *true* announcement makes his new belief safe without
            // giving him knowledge. That middle state is the reason `[]` exists.
            ("B[ben] up", true),
            ("[][ben] up", true),
            ("K[ben] up", false),
        ],
    );

    let mut sensed = run(include_str!("../../../examples/safe_belief.delhi"), &["check(ben)"]);
    expect(&mut sensed, &[("K[ben] up", true), ("[][ben] up", true)]);

    let mut lied = run(include_str!("../../../examples/safe_belief.delhi"), &["deny()"]);
    expect(
        &mut lied,
        &[
            // A lie moves belief and can never make it safe, since `[]` is factive...
            ("B[ada] !up", true),
            ("[][ada] !up", false),
            // ...but it can *destroy* one. Ada held `up` safely before this.
            ("[][ada] up", false),
        ],
    );
}

#[test]
fn the_handover_capability_needs_safe_belief_to_state_its_goal() {
    // The paper's running example. Its headline claim is that the success criterion is
    // *safe* belief rather than belief: telling the truth achieves it, telling a lie
    // cannot, and showing achieves knowledge as well. Those three rows are the
    // acquisition table of the safe-belief section, on the domain that motivates it.
    let src = include_str!("../../../examples/handover.delhi");

    // She is wrong, and the robot knows she is wrong.
    let mut p = run(src, &[]);
    expect(
        &mut p,
        &[
            ("B[nurse] given & !given", true),
            ("K[robot] (B[nurse] given & !given)", true),
            // The goal does not hold yet -- and `[]` is what lets it be stated at all.
            ("[][nurse] !given", false),
        ],
    );

    // Telling: belief, safely, but not knowledge. This is the row that motivates `[]`.
    let mut told = run(src, &["robot_tells()"]);
    expect(
        &mut told,
        &[("B[nurse] !given", true), ("[][nurse] !given", true), ("K[nurse] !given", false)],
    );
    let goal = told.goal.expect("the file declares a goal");
    assert!(told.state.entails(&told.store, goal), "telling the truth discharges the goal");

    // Showing: knowledge, hence also safe belief.
    let mut shown = run(src, &["robot_shows()"]);
    expect(&mut shown, &[("K[nurse] !given", true), ("[][nurse] !given", true)]);

    // Lying moves belief and can never make it safe, `[]` being factive.
    let mut lied = run(src, &["robot_lies()"]);
    expect(&mut lied, &[("B[nurse] given", true), ("[][nurse] given", false)]);
}

#[test]
fn the_paged_doctor_ends_up_with_a_stale_belief_about_the_nurse() {
    // `doctor aware if !busy` is the conditional-observability clause. Paging her away
    // makes her oblivious to the correction, so her model of the nurse goes stale -- a
    // false belief about a mind, arising from an observability condition rather than
    // from anything declared. This is what the enumeration example in the paper finds.
    let src = include_str!("../../../examples/handover.delhi");

    // In the room: she learns the nurse was corrected.
    let mut heard = run(src, &["robot_tells()"]);
    expect(&mut heard, &[("K[doctor] B[nurse] !given", true)]);

    // Paged away first: she does not, and still takes the nurse to be mistaken.
    let mut missed = run(src, &["doctor_paged()", "robot_tells()"]);
    expect(
        &mut missed,
        &[
            ("K[doctor] B[nurse] !given", false),
            ("B[doctor] B[nurse] given", true),
            ("B[nurse] !given", true), // ...while the nurse has in fact been put right
        ],
    );
}
