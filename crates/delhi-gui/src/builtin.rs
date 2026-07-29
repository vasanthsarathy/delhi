//! The examples that ship inside the binary.
//!
//! A downloaded or `cargo install`ed `delhi` has no repository beside it, so without these
//! the UI opens on an empty directory and a new user has nowhere to learn what a `.delhi`
//! file looks like. They are read-only: the copy in the binary is the one that is served,
//! and saving one writes a new file into the served directory under whatever name the user
//! chooses.
//!
//! **Generated — do not edit by hand.** Regenerate with `tools/bundle-examples.sh`.
//!
//! The sources are inlined rather than `include_str!`d from `examples/`, which sits
//! outside this crate. `cargo package` cannot carry files from beyond the package root, so
//! the path form produced a crate that published cleanly and then failed to compile for
//! anyone who installed it. `lib.rs` asserts this file matches `examples/` byte for byte,
//! so the copy cannot drift.

/// Bundled examples as `(file name, source)`, sorted by name.
pub const BUILTIN: &[(&str, &str)] = &[
    (
        "bicycle.delhi",
        r#"// The Birthday Bicycle Story.
//
// Sullivan, Zaitchik & Tager-Flusberg (1994), a second-order false-belief task from
// the developmental literature. It is also the story the KR 2021 paper opens with,
// as the thing its action language was built to represent.
//
// It is Timmy's birthday and his mother has hidden a bicycle in the basement. To
// keep the surprise she tells him she does *not* intend to give him one. Timmy
// believes her. Then, unbeknownst to her, he goes down to the basement, sees the
// bicycle, and works out what she meant to do.
//
//     "What does Timmy's mother think Timmy believes?"
//
// She thinks he still believes there is no bicycle. She is wrong, and she is wrong
// about his mind rather than about the world -- she knows perfectly well the bicycle
// is down there.
//
// Two things are happening at once, and the story needs both:
//
//   * Timmy is lied to, believes it, and then revises when the evidence arrives.
//     Under a flat belief set that sequence is a contradiction with nowhere to put
//     the correction. Here it is a reordering of which world he finds most
//     plausible, and then a reordering back.
//
//   * His mother never sees him look, so her picture of him goes stale and stays
//     stale. That is the second-order part.
//
// Ported from the mecaPlanner corpus (problems/aamas/bicycle1.depl).

types   { Actor - Object }
objects { timmy, mom - Actor }
agents  { timmy, mom }
props   { bicycle }                // there is a bicycle in the basement for him

initially {
    bicycle                        // there really is one
    ?[timmy] bicycle               // but he cannot see the basement from here
    B[timmy] !bicycle              // and has no particular reason to expect it
}

// He works out the truth; she never learns that he did.
goal { K[timmy] bicycle & B[mom] B[timmy] !bicycle }

actions {
    mom_tells_him_no() {
        actor     mom
        announces !bicycle         // a lie, told to protect the surprise
        timmy observes, mom observes
    }

    timmy_looks_in_the_basement() {
        actor      timmy
        determines bicycle         // sensing: he comes to KNOW, not merely believe
        timmy observes
        // mom is not listed, so she is oblivious -- she has no idea he went down.
    }
}
"#,
    ),
    (
        "coin_in_the_box.delhi",
        r#"// Coin in the Box — the standard epistemic-planning benchmark.
//
// From van Ditmarsch's work on DEL, and one of the domains every epistemic planner
// gets measured on (it appears in the EFP suite, and in the mecaPlanner corpus as
// problems/efp/Coin_in_the_Box_*.depl).
//
// A coin lies in a locked box. Alice has the key. Anyone *looking* when the box is
// opened sees what happens; anyone merely within earshot knows something happened
// without learning what. So the domain separates three epistemic positions cleanly,
// which is why it is the benchmark: seeing, hearing, and missing entirely.
//
// This file shows the three features the other examples do not: a parameterised
// predicate, a parameterised action, and constants.
//
// The constants earn their place here. `peek(?p)` says the peeker sees the coin and
// everyone *else* merely hears it happen -- but with `?p` unbound, "everyone else"
// cannot be written down. `same` supplies it. It is declared false for every pair
// and true only on the diagonal, so `!same(?p, bob)` folds to a literal true or
// false the moment `?p` is bound, and never becomes a proposition occupying a bit
// in every world. Without it, grounding `peek(bob)` puts bob in two observer classes
// at once, and the well-formedness check rejects it.

types   { Actor - Object }
objects { alice, bob, carol - Actor }
agents  { alice, bob, carol }
props   { tail, opened, looking(Actor) }

constants {
    !same(Actor, Actor)            // by default no two actors are the same one
    same(alice, alice)             // and these are the exceptions
    same(bob, bob)
    same(carol, carol)
}

initially {
    tail                           // the coin is, in fact, tails
    ?[alice] tail                  // but nobody can see it through a closed box
    ?[bob]   tail
    ?[carol] tail
    looking(alice)                 // alice is watching the box; the others are not
}

// Alice learns the coin. Bob does not -- but he knows that she did.
goal { K[alice] tail & !Kw[bob] tail & K[bob] Kw[alice] tail }

actions {
    open_box() {
        actor  alice               // she has the key
        pre    !opened
        causes opened
        alice observes if looking(alice)
        bob   observes if looking(bob)
        carol observes if looking(carol)
    }

    peek(?p - Actor) {
        actor      ?p
        pre        opened & looking(?p)
        determines tail            // whoever looks comes to KNOW which way it is

        ?p observes                            // sees the coin itself
        alice aware if !same(?p, alice)        // everyone else is merely
        bob   aware if !same(?p, bob)          // in earshot: they learn that
        carol aware if !same(?p, carol)        // ?p now knows, not which way
    }
}
"#,
    ),
    (
        "coin_lie.delhi",
        r#"// The Coin Lie scenario. Reproduces [T] Figs 5.4-5.10.
//
// A lies that the coin is tails; B distracts A; C peeks and learns the truth.
// Because A is distracted she never sees the peek, so her picture of C goes
// stale and she ends up wrong about what C believes.

types   { Actor - Object }
objects { alice, bob, carol - Actor }
agents  { alice, bob, carol }
props   { h, d }

initially {
    h                    // the coin really is heads
    ?[carol] h           // carol cannot tell
    B[carol] h           // but she correctly leans that way
}

goal { B[alice] B[carol] !h & K[carol] h }

actions {
    announce_not_heads() {
        actor     alice
        announces !h                  // a lie; truth is not required
        alice observes, bob observes, carol observes
    }

    distract_a() {
        actor  bob
        causes d
        alice observes, bob observes, carol observes
    }

    peek_c() {
        actor      carol
        determines h
        carol observes                // sees the coin: comes to KNOW
        bob   aware                   // hears it: knows carol learned something
        alice aware if !d             // only notices if she is not distracted
    }
}
"#,
    ),
    (
        "grapevine.delhi",
        r#"// Grapevine — from the EFP benchmark suite.
//
// Ported from the mecaPlanner corpus (problems/efp/grapevine_config1.depl). Three
// agents, two rooms, and one secret each. You may tell a secret you believe, and
// whoever is in the room with you hears it. You may walk between rooms, and whoever is
// in the room you leave sees you go.
//
// The goal is the interesting part, because one conjunct is *negative*:
//
//     B[a] secret(b)            a learns b's secret
//     !B[c] secret(b)           and c does not
//     B[b] B[a] !B[c] secret(b) and b knows that a knows c is out of the loop
//
// So it is not enough to spread information; you have to spread it to some agents and
// not others, and then be sure the right agent knows about the gap. That is what makes
// gossip domains a planning benchmark rather than a broadcast exercise.
//
// **On the port.** The original enumerates all 24 actions by hand — `share_a_sb_2`,
// `left_c`, `right_a` and so on — because that encoding has no parameters. delhi does,
// so the same domain is two action declarations that ground to the same 24. The
// `same` constant is what makes `move` well formed: without it `move(a, r1, r1)`
// grounds to an action causing `at(a,r1)` and `!at(a,r1)` at once, which is rejected as
// contradictory. With it, the precondition folds to false at `?from == ?to` and those
// six groundings are dropped before they are ever built.
//
// The original also names `god` as every action's owner, a dummy required by mA*'s
// syntax; it observes nothing and appears in no goal, so the port drops it.

types   { Actor - Object, Room - Object }
objects { a, b, c - Actor
          r1, r2 - Room }
agents  { a, b, c }
props   { at(Actor, Room), secret(Actor) }

constants {
    !same(Room, Room)              // two rooms differ unless they are the same one
    same(r1, r1)
    same(r2, r2)
}

initially {
    secret(a) secret(b) secret(c)  // every secret is in fact true
    at(a, r1) at(b, r1) at(c, r1)  // and everyone starts together in room 1

    ?[b] secret(a)  ?[c] secret(a) // each agent knows her own secret and no other's:
    ?[a] secret(b)  ?[c] secret(b) // declaring uncertainty for everyone *except* the
    ?[a] secret(c)  ?[b] secret(c) // owner is what makes that true
}

goal { B[a] secret(b) & !B[c] secret(b) & B[b] B[a] !B[c] secret(b) }

actions {
    // Telling a secret you believe. Heard by whoever shares the room.
    share(?who - Actor, ?whose - Actor, ?room - Room) {
        actor     ?who
        pre       B[?who] secret(?whose) & at(?who, ?room)
        announces secret(?whose)
        ?o observes if at(?o, ?room)
    }

    // Walking out. Seen by whoever is in the room being left — including the mover.
    move(?who - Actor, ?from - Room, ?to - Room) {
        actor  ?who
        pre    at(?who, ?from) & !same(?from, ?to)
        causes at(?who, ?to), !at(?who, ?from)
        ?o observes if at(?o, ?from)
    }
}
"#,
    ),
    (
        "ice_cream_van.delhi",
        r#"// The ice-cream van — a *second-order* false belief.
//
// Perner & Wimmer (1985), the follow-up to Sally-Anne. Sally-Anne asks whether you
// can represent someone's false belief about the world. This asks whether you can
// represent someone's false belief about *someone else's belief*, which children
// typically pass several years later.
//
// The van is in the park. John and Mary are both there. Mary goes home. The van
// moves to the church, and John sees it go -- Mary does not. So far this is
// Sally-Anne: Mary is wrong about the van.
//
// Then the driver passes Mary's house and tells her. John does not see that happen.
//
//     "Where does John think Mary will go for ice cream?"
//
// The park. John is not wrong about the van -- he watched it leave. He is wrong
// about Mary's mind, and that is a different and harder thing to be wrong about.
//
// Adapted from the mecaPlanner corpus (problems/aamas/icecream.depl), extended with
// the telling step that makes the task genuinely second-order.

types   { Actor - Object }
objects { john, mary - Actor }
agents  { john, mary }
props   { at_park, mary_home }     // van is at the park; mary has gone home

initially {
    at_park                        // common knowledge: the van is in the park
}

// John's belief about Mary's belief is false, while Mary in fact knows better.
goal { B[john] B[mary] at_park & K[mary] !at_park }

actions {
    mary_goes_home() {
        actor  mary
        causes mary_home
        john observes, mary observes
    }

    van_moves_to_church() {
        actor  john                    // john happens to be the one watching
        pre    at_park
        causes !at_park
        john observes
        mary observes if !mary_home    // she is home, so she misses it
    }

    driver_tells_mary() {
        actor      mary
        determines at_park             // she learns which way it is: she now KNOWS
        mary observes
        // john is not listed at all -- he is oblivious, so his picture of Mary
        // goes stale and stays stale.
    }
}
"#,
    ),
    (
        "muddy_children.delhi",
        r#"// Muddy Children — the canonical multi-agent epistemic puzzle.
//
// Three children have been playing; all three have muddy foreheads. Each can see
// the others but not herself. Their father says "at least one of you is muddy",
// then asks repeatedly "do you know whether you are muddy?"
//
// The classic answer: nobody knows for the first two rounds, and then all three
// know at once. Nothing was said in those rounds -- and yet *that* is the
// information. Each child learns from the others' silence.
//
// Read the result of this file carefully, because delhi does not reproduce the
// textbook answer exactly, and the difference is the interesting part. See the
// note at the bottom.

types   { Kid - Object }
objects { alice, bob, carol - Kid }
agents  { alice, bob, carol }
props   { muddy(Kid) }

initially {
    muddy(alice)                   // all three really are muddy
    muddy(bob)
    muddy(carol)

    ?[alice] muddy(alice)          // and none can see her own forehead.
    ?[bob]   muddy(bob)            // Declaring uncertainty only about herself is
    ?[carol] muddy(carol)          // what makes each child know the other two:
}                                  // worlds are comparable for a child exactly
                                   // when they agree on everything she is not
                                   // uncertain about.

goal { Bw[alice] muddy(alice) & Bw[bob] muddy(bob) & Bw[carol] muddy(carol) }

actions {
    // The actor of an announcement is recorded for display only -- it never reaches
    // the semantics, since observability is independent of agency. So the father
    // need not be modelled as an agent.
    father_speaks() {
        actor     alice
        announces muddy(alice) | muddy(bob) | muddy(carol)
        alice observes, bob observes, carol observes
    }

    nobody_knows() {
        actor     alice
        announces !Bw[alice] muddy(alice)
                & !Bw[bob]   muddy(bob)
                & !Bw[carol] muddy(carol)
        alice observes, bob observes, carol observes
    }
}

// ----------------------------------------------------------------------------
// What actually happens, and where it departs from the textbook.
//
//     delhi repl examples/muddy_children.delhi
//     > :do father_speaks()
//     > :do nobody_knows()
//     > :do nobody_knows()
//     > Bw[alice] muddy(alice)     // true -- and likewise for bob and carol
//     > B[alice] muddy(alice)      // true: she concludes she IS muddy
//     > Kw[alice] muddy(alice)     // FALSE
//
// The timing is exactly classical: with three muddy children, ignorance is
// announced twice and on the third round all three conclude together. Comment out
// either `nobody_knows()` and the conclusion does not arrive -- the silence really
// is carrying the information.
//
// What differs is the *attitude*. The textbook version ends in knowledge; here it
// ends in belief, and `Kw` stays false however many rounds you run.
//
// That is not a bug, it is mB being consistent about what an announcement is. In
// plain public-announcement logic, announcing φ deletes every ¬φ world, so what
// survives is knowledge -- but that only works if announcements cannot be false.
// mB is built for a language where they can be, and the Coin Lie in this same
// directory turns on exactly that. So an announcement reorders which worlds an
// agent finds plausible instead of destroying any, and no reordering can ever
// produce knowledge, because the ¬φ worlds are all still there to be considered.
//
// The price is that this puzzle ends one notch weaker than the classical account.
// The thing bought is the ability to model an agent who is lied to, believes it,
// and later recovers -- which is the whole point of the system.

"#,
    ),
    (
        "reachability.delhi",
        r#"// Rules, definitions and invariants together.
//
// The map is static, so `reach` — the transitive closure of `adjacent` — can be
// computed once at parse time by a Horn fixpoint and folded away. It never becomes
// a proposition and never occupies a bit in any world.

types   { Actor - Object, Room - Object }
objects { alice - Actor
          hall, study, attic, cellar - Room }
agents  { alice }
props   { at(Actor, Room) }

constants {
    !adjacent(Room, Room)          // nothing is adjacent unless said otherwise
    adjacent(hall, study)
    adjacent(study, attic)
}

rules {
    reach(?x, ?y) :- adjacent(?x, ?y)
    reach(?x, ?z) :- adjacent(?x, ?y), reach(?y, ?z)
}

define {
    stuck(?w)     = !reach(hall, cellar) & at(?w, hall)
    can_get(?w, ?r) = at(?w, hall) & reach(hall, ?r)
}

initially { at(alice, hall) }

goal { can_get(alice, attic) }

invariants { !at(alice, cellar) }     // she can never get there, so she never is there

actions {
    walk(?w - Actor, ?from - Room, ?to - Room) {
        actor  ?w
        pre    at(?w, ?from) & adjacent(?from, ?to)
        causes at(?w, ?to), !at(?w, ?from)
        ?o observes
    }
}
"#,
    ),
    (
        "safe_belief.delhi",
        r#"// Safe belief: the attitude between knowledge and belief.
//
// Two agents face the same question and lean opposite ways. One is right, one is wrong,
// and neither *knows* — so plain belief cannot tell them apart. Safe belief can.
//
// The rule is that safe belief is measured from the actual world: `[][i] p` holds when p
// is true in every world i finds at least as plausible as the way things actually are.
// An agent who is right has almost nothing above the actual world, so its beliefs are
// safe. An agent who is wrong has its own favoured worlds sitting above reality, and p
// has to survive those too.
//
// The practical reading: a safe belief is one that no *true* information can overturn.
//
//     delhi eval examples/safe_belief.delhi -f "[][ada] up"      // true  — right
//     delhi eval examples/safe_belief.delhi -f "[][ben] !up"     // false — wrong
//     delhi eval examples/safe_belief.delhi -a "gossip()" -f "B[ben] !up"
//
// See https://vsarathy.com/delhi/book/logic/safe-belief.html

types   { Actor - Object }
objects { ada, ben, cleo - Actor }
agents  { ada, ben, cleo }
props   { up, rain }        // the server is up; it is raining

initially {
    up                      // it really is up
    rain                    // and it really is raining

    ?[ada] up               // ada cannot tell...
    B[ada] up               // ...but leans the right way

    ?[ben] up               // ben cannot tell either...
    B[ben] !up              // ...and leans the wrong way

    ?[cleo] up              // cleo has no idea and no leaning either way
}

actions {
    // True, and said to everyone. Ada's belief is untouched; ben's is overturned.
    gossip() {
        actor     cleo
        announces up
        ada observes, ben observes, cleo observes
    }

    // A lie. It can move belief, but never safe belief — `[]` is factive.
    deny() {
        actor     cleo
        announces !up
        ada observes, ben observes, cleo observes
    }

    // Sensing: the observer comes to KNOW, and so also to safely believe.
    check(?who - Actor) {
        actor      ?who
        determines up
        ?who observes
    }
}
"#,
    ),
    (
        "sally_anne.delhi",
        r#"// Sally-Anne — the canonical false-belief task.
//
// Wimmer & Perner (1983); Baron-Cohen, Leslie & Frith (1985) turned it into the
// test that made it famous. Sally puts her marble in the basket and leaves. Anne
// moves it to the box. Sally comes back.
//
//     "Where will Sally look for her marble?"
//
// The answer is the basket, and getting it right requires representing Sally's
// belief as separate from the facts. Children under about four say "the box" --
// they answer where the marble *is*, because they have no machinery for a belief
// that is false.
//
// Ported from the mecaPlanner corpus (problems/aamas/sallyanne.depl).
//
// The whole task turns on one clause: `sally observes if present`. Anne's move is
// witnessed only if Sally is in the room, and she is not.

types   { Actor - Object }
objects { sally, anne - Actor }
agents  { sally, anne }
props   { present, basket, box }   // sally in the room; marble in basket; in box

initially {
    present                        // sally is here, and the marble is
    basket                         // in the basket -- both agents can see it
}

// Where Sally will look, versus where the marble is.
goal { B[sally] basket & box }

actions {
    sally_leaves() {
        actor  sally
        causes !present
        sally observes, anne observes
    }

    anne_moves() {
        actor  anne
        pre    basket
        causes box, !basket
        anne  observes
        sally observes if present  // she is not, so she misses it entirely
    }

    sally_returns() {
        actor  sally
        causes present
        sally observes, anne observes
    }
}
"#,
    ),
    (
        "sally_anne_second_order.delhi",
        r#"// Sally-Anne, second-order variant.
//
// Bräuner, Blackburn & Polyanskaya (2016). This is Example 1 of the KR 2024 paper,
// used there to show what an action language needs before it can express the case.
//
// Same room, same marble, same box and basket. The difference is one line: Sally
// does not leave. She stays and watches secretly, and Anne does not realise she has
// been seen.
//
//     "Where does Anne expect Sally to look for the marble?"
//
// The basket. Sally in fact knows perfectly well the marble is in the box, because
// she watched it move. Anne is wrong about Sally's belief, which makes this a
// second-order task rather than the first-order one in `sally_anne.delhi`.
//
// What this file demonstrates about the language is narrower and more specific than
// "it does second-order beliefs" -- the ice-cream van already shows that, by having
// an agent simply miss an event. Here nobody misses anything. Anne's mistake is
// about *observability itself*: about whether Sally was in a position to see. That
// is what local dynamic observability buys, and it is why the observer clauses take
// conditions rather than being fixed per action:
//
//     sally observes if watching
//
// Anne's most plausible worlds are ones where `watching` is false, so in those
// worlds she computes Sally as oblivious, and her model of Sally never updates.

types   { Actor - Object }
objects { sally, anne - Actor }
agents  { sally, anne }
props   { watching, basket, box }

initially {
    basket                         // the marble starts in the basket
    watching                       // and sally really is watching

    ?[anne] watching               // anne cannot tell whether she is being watched
    B[anne] !watching              // and assumes she is not
}

// Sally knows where the marble is; Anne expects her to look in the basket anyway.
goal { K[sally] box & B[anne] B[sally] basket }

actions {
    anne_moves() {
        actor  anne
        pre    basket
        causes box, !basket
        anne  observes
        sally observes if watching     // true in fact, false in anne's picture
    }
}
"#,
    ),
    (
        "selective_communication.delhi",
        r#"// Selective Communication (SC_3_4) — from the EFP benchmark suite.
//
// Ported from the mecaPlanner corpus (problems/efp/SC_3_4.depl), which is the encoding
// used to measure epistemic planners. Three agents and four positions.
//
// Agent `a` can sense the truth of `q`, but only from position 2. Everyone can walk
// left and right along the four positions, and everyone sees them do it. Shouting `q`
// is possible from any position, but *who hears it depends on where you are*:
//
//     position 1 -> a, b        position 3 -> b, c
//     position 2 -> a, b, c     position 4 -> a, c
//
// That is the whole puzzle. To get a nested belief to a particular agent you have to
// walk to the position whose audience is the one you need. The goals are third-order:
//
//     B[a] B[c] B[a] q     and     B[c] B[a] B[c] q
//
// Two notes on the port. The original names `god` as the owner of every action, a dummy
// required by mA*'s syntax; `god` observes nothing and appears in no goal, so it is pure
// scaffolding. delhi records an actor for display only — it never reaches the semantics —
// so the port drops `god` and keeps the three agents that do epistemic work. And the
// original's `shout_2` lists `observes{a}` twice, which is harmless and not reproduced.

types   { Actor - Object }
objects { a, b, c - Actor }
agents  { a, b, c }
props   { q, at_1, at_2, at_3, at_4 }

initially {
    q                              // q is in fact true
    at_1                           // and everyone starts at position 1

    ?[a] q                         // but nobody can see whether q holds
    ?[b] q
    ?[c] q
}

goal { B[a] B[c] B[a] q & B[c] B[a] B[c] q }

actions {
    // Walking is public: everyone sees the position change.
    left() {
        actor  a
        pre    !at_1
        causes at_3, !at_4 if at_4
        causes at_2, !at_3 if at_3
        causes at_1, !at_2 if at_2
        a observes, b observes, c observes
    }

    right() {
        actor  a
        pre    !at_4
        causes at_2, !at_1 if at_1
        causes at_3, !at_2 if at_2
        causes at_4, !at_3 if at_3
        a observes, b observes, c observes
    }

    // Only `a`, and only from position 2, can find out whether q.
    sense() {
        actor      a
        pre        at_2
        determines q
        a observes
    }

    // Shouting requires already believing q. The audience is the position's.
    shout_1() {
        actor     a
        pre       B[a] q & at_1
        announces q
        a observes, b observes
    }

    shout_2() {
        actor     a
        pre       B[a] q & at_2
        announces q
        a observes, b observes, c observes
    }

    shout_3() {
        actor     a
        pre       B[a] q & at_3
        announces q
        b observes, c observes
    }

    shout_4() {
        actor     a
        pre       B[a] q & at_4
        announces q
        a observes, c observes
    }
}
"#,
    ),
];
