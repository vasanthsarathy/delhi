//! The request handlers, as pure functions.
//!
//! Every one takes the source text plus whatever the request carried and returns a JSON
//! string. Nothing here touches a socket, so the whole surface is unit-testable without
//! standing a server up — the same split that makes `delhi-cli`'s subcommands testable.
//!
//! The protocol is deliberately **stateless**: the browser holds the source and the list
//! of applied actions, and sends both with every request. Replaying a short trace costs
//! microseconds, and in exchange there are no sessions to expire, no way for the server's
//! idea of the state to drift from the editor's, and a reload cannot lose anything.

use delhi_lang::{print_state, Problem};
use delhi_mb::State;
use serde::Serialize;

/// One agent's stance, mirroring [`delhi_lang::AgentView`] for the wire.
#[derive(Serialize)]
pub struct Agent {
    pub agent: String,
    pub knows: Vec<String>,
    pub believes: Vec<String>,
    pub undecided: Vec<String>,
}

/// A world as the graph view needs it.
#[derive(Serialize)]
pub struct World {
    /// Index, which is also the label `w{id}`.
    pub id: usize,
    /// Propositions true here. Shown on hover.
    pub facts: Vec<String>,
    /// Signed propositions that *distinguish* this world from the others.
    ///
    /// A world node is a small circle, and in a domain like Grapevine nine atoms will
    /// not fit in one — but eight of the nine are the same in every world, so printing
    /// them says nothing. Restricting the label to the atoms that actually vary is what
    /// makes the picture readable: the positions drop out and the secrets remain, which
    /// is exactly what the worlds disagree about.
    pub label: Vec<String>,
    /// Whether this is the actual world.
    pub designated: bool,
}

/// One plausibility edge. `mutual` collapses the two directions into a single line.
#[derive(Serialize)]
pub struct Edge {
    pub agent: String,
    pub from: usize,
    pub to: usize,
    pub mutual: bool,
}

/// One diagnostic, positioned so the page can jump the editor to it.
#[derive(Serialize)]
pub struct Fault {
    pub line: usize,
    pub col: usize,
    /// Byte offsets, which is what a textarea selection needs.
    pub start: usize,
    pub end: usize,
    pub message: String,
}

/// Everything the page needs to render one state.
#[derive(Serialize)]
pub struct StateReply {
    pub ok: bool,
    /// Rendered diagnostics, when the file was rejected.
    pub error: Option<String>,
    /// The same diagnostics, positioned. Sent alongside the rendering rather than
    /// instead of it: the text reads well in a panel, and the offsets are what make an
    /// error clickable.
    pub faults: Vec<Fault>,
    pub facts: Vec<String>,
    pub agents: Vec<Agent>,
    pub worlds: Vec<World>,
    pub edges: Vec<Edge>,
    /// Ground action names, for the clickable list.
    pub actions: Vec<String>,
    /// The model in the explicit `state { … }` form.
    pub explicit: String,
    /// Actions that were applied, and any that could not be.
    pub applied: Vec<String>,
    /// Set when an action in the trace was unknown or inapplicable.
    pub trace_error: Option<String>,
    /// Declared invariants the current state violates, as the author wrote them.
    pub violated: Vec<String>,
    pub n_worlds: usize,
    /// Whether the file's declared goal holds here, if it declares one.
    pub goal: Option<bool>,
}

fn rejected(error: String, faults: Vec<Fault>) -> StateReply {
    StateReply {
        ok: false,
        error: Some(error),
        faults,
        facts: Vec::new(),
        agents: Vec::new(),
        worlds: Vec::new(),
        edges: Vec::new(),
        actions: Vec::new(),
        explicit: String::new(),
        applied: Vec::new(),
        trace_error: None,
        violated: Vec::new(),
        n_worlds: 0,
        goal: None,
    }
}

/// Applies `trace` to the problem's initial state, contracting after each step.
///
/// Contraction matters here for the same reason it does in the REPL: product update
/// multiplies worlds by events, and without quotienting a dozen actions produces
/// thousands of worlds that no agent can tell apart — which would also make the graph
/// unreadable. `~R` is used because it is proved a congruence, so it cannot change any
/// answer the page reports.
fn replay(p: &mut Problem, trace: &[String]) -> (State, Vec<String>, Option<String>) {
    let n_agents = p.sig.n_agents();
    let mut state = p.state.clone();
    let mut applied = Vec::new();
    for name in trace {
        let Some(g) = p.actions.iter().find(|a| &a.name == name) else {
            return (state, applied, Some(format!("no action `{name}`")));
        };
        let def = g.def.clone();
        let am = delhi_mb::build(&def, &mut p.store, n_agents);
        match state.apply(&p.store, &am) {
            Some(next) => {
                let (model, blocks) = next.model.contract_dynamic();
                let designated = blocks[next.designated] as usize;
                state = State { model, designated };
                applied.push(name.clone());
            }
            None => {
                return (
                    state,
                    applied,
                    Some(format!("`{name}` is not applicable in the current state")),
                );
            }
        }
    }
    (state, applied, None)
}

/// Positions every diagnostic against the source.
fn faults_of(diags: &delhi_lang::Diagnostics, src: &str) -> Vec<Fault> {
    diags
        .located(src)
        .into_iter()
        .map(|l| Fault { line: l.line, col: l.col, start: l.start, end: l.end, message: l.message })
        .collect()
}

/// `POST /api/state` — check the source, replay the trace, and describe where it lands.
pub fn state(src: &str, trace: &[String]) -> String {
    // `check` rather than `parse`, because a rendered error string has already discarded
    // the spans, and the page needs them to jump the editor to the fault.
    let (built, diags) = Problem::check(src);
    let mut p = match built {
        Some(p) if diags.is_empty() => p,
        _ => {
            let reply = rejected(diags.render(src), faults_of(&diags, src));
            return serde_json::to_string(&reply).expect("serialises");
        }
    };
    let (state, applied, trace_error) = replay(&mut p, trace);

    let view = delhi_lang::state_view(&mut p, &state);
    let m = &state.model;

    // An atom that holds in every world, or in none, cannot tell two worlds apart, so it
    // is dropped from the node labels. With one world nothing varies and the label falls
    // back to the full valuation, which is then the only thing there is to say.
    let varying: Vec<usize> = (0..p.sig.n_atoms())
        .filter(|a| {
            let first = m.val[0].get(*a);
            (1..m.n_worlds).any(|w| m.val[w].get(*a) != first)
        })
        .collect();
    let labelled: Vec<usize> =
        if varying.is_empty() { (0..p.sig.n_atoms()).collect() } else { varying };

    let worlds = (0..m.n_worlds)
        .map(|w| World {
            id: w,
            facts: (0..p.sig.n_atoms())
                .filter(|a| m.val[w].get(*a))
                .map(|a| p.sig.atom_name(a as u32).to_string())
                .collect(),
            label: labelled
                .iter()
                .map(|a| {
                    let name = p.sig.atom_name(*a as u32);
                    if m.val[w].get(*a) {
                        name.to_string()
                    } else {
                        format!("!{name}")
                    }
                })
                .collect(),
            designated: w == state.designated,
        })
        .collect();

    // Reflexive edges are implicit and never drawn; a mutually related pair is one
    // line, matching what `print_state` emits and keeping the picture readable.
    let mut edges = Vec::new();
    for i in 0..m.n_agents {
        let agent = p.sig.agent_name(i as u32).to_string();
        for u in 0..m.n_worlds {
            for v in (u + 1)..m.n_worlds {
                match (m.rel[i][u].get(v), m.rel[i][v].get(u)) {
                    (true, true) => {
                        edges.push(Edge { agent: agent.clone(), from: u, to: v, mutual: true })
                    }
                    (true, false) => {
                        edges.push(Edge { agent: agent.clone(), from: u, to: v, mutual: false })
                    }
                    (false, true) => {
                        edges.push(Edge { agent: agent.clone(), from: v, to: u, mutual: false })
                    }
                    (false, false) => {}
                }
            }
        }
    }

    let goal = p.goal.map(|g| state.entails(&p.store, g));
    let mut actions: Vec<String> = p.actions.iter().map(|a| a.name.clone()).collect();
    actions.sort();

    let reply = StateReply {
        ok: true,
        error: None,
        faults: Vec::new(),
        facts: view.facts,
        agents: view
            .agents
            .into_iter()
            .map(|a| Agent {
                agent: a.agent,
                knows: a.knows,
                believes: a.believes,
                undecided: a.undecided,
            })
            .collect(),
        worlds,
        edges,
        actions,
        explicit: print_state(&state, &p.sig),
        applied,
        trace_error,
        violated: p.violated(&state).into_iter().map(String::from).collect(),
        n_worlds: m.n_worlds,
        goal,
    };
    serde_json::to_string(&reply).expect("serialises")
}

/// The answer to one formula typed at the console.
#[derive(Serialize)]
pub struct EvalReply {
    pub ok: bool,
    pub value: Option<bool>,
    pub error: Option<String>,
}

/// `POST /api/eval` — evaluate `formula` in the state the trace reaches.
pub fn eval(src: &str, trace: &[String], formula: &str) -> String {
    let mut p = match Problem::parse(src) {
        Ok(p) => p,
        Err(e) => {
            return serde_json::to_string(&EvalReply { ok: false, value: None, error: Some(e) })
                .expect("serialises")
        }
    };
    let (state, _, trace_error) = replay(&mut p, trace);
    if let Some(e) = trace_error {
        return serde_json::to_string(&EvalReply { ok: false, value: None, error: Some(e) })
            .expect("serialises");
    }

    // Lowered against the checked problem's own signature and constants, so a query may
    // name anything the file declares.
    let mut diags = delhi_lang::Diagnostics::default();
    let toks = delhi_lang::lex(formula, &mut diags);
    let expr = delhi_lang::Parser::new(&toks).parse_expr(&mut diags);
    // Expanded like the file's own formulas, so a `define` name works here too.
    let expr = delhi_lang::expand(&expr, &p.defs, &mut diags);
    let f = delhi_lang::lower_formula(
        &expr,
        &p.sig,
        &p.consts,
        &delhi_lang::Bindings::default(),
        &mut p.store,
        &mut diags,
    );
    let reply = if diags.is_empty() {
        EvalReply { ok: true, value: Some(state.entails(&p.store, f)), error: None }
    } else {
        EvalReply { ok: false, value: None, error: Some(diags.render(formula)) }
    };
    serde_json::to_string(&reply).expect("serialises")
}

/// The result of an enumeration.
#[derive(Serialize)]
pub struct AskReply {
    pub ok: bool,
    pub matches: Vec<String>,
    pub considered: usize,
    pub truncated: bool,
    pub error: Option<String>,
}

fn ask_error(e: String) -> String {
    serde_json::to_string(&AskReply {
        ok: false,
        matches: Vec::new(),
        considered: 0,
        truncated: false,
        error: Some(e),
    })
    .expect("serialises")
}

/// `POST /api/ask` — which formulas of the given shape hold in the state the trace reaches.
pub fn ask(src: &str, trace: &[String], pattern: &str, depth: usize) -> String {
    let mut p = match Problem::parse(src) {
        Ok(p) => p,
        Err(e) => return ask_error(e),
    };
    let (state, _, trace_error) = replay(&mut p, trace);
    if let Some(e) = trace_error {
        return ask_error(e);
    }
    match delhi_lang::ask(&mut p, &state, pattern, depth) {
        Err(e) => ask_error(e),
        Ok(a) => serde_json::to_string(&AskReply {
            ok: true,
            matches: a.matches,
            considered: a.considered,
            truncated: a.truncated,
            error: None,
        })
        .expect("serialises"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const COIN: &str = include_str!("../../../examples/coin_lie.delhi");

    fn json(s: &str) -> serde_json::Value {
        serde_json::from_str(s).expect("valid json")
    }

    #[test]
    fn state_reports_attitudes_and_a_drawable_graph() {
        let v = json(&state(COIN, &[]));
        assert_eq!(v["ok"], true);
        assert_eq!(v["n_worlds"], 2);
        // Carol leans towards h without knowing it — the distinction the panel exists
        // to show, so it must survive the trip to the wire.
        let carol = v["agents"].as_array().unwrap().iter().find(|a| a["agent"] == "carol").unwrap();
        assert_eq!(carol["believes"][0], "h");
        assert!(carol["knows"].as_array().unwrap().iter().all(|k| k != "h"));
        // Two worlds, one designated, and an edge to draw between them.
        assert_eq!(v["worlds"].as_array().unwrap().len(), 2);
        assert_eq!(
            v["worlds"].as_array().unwrap().iter().filter(|w| w["designated"] == true).count(),
            1
        );
        // `d` is false in both worlds, so it says nothing about which world you are in
        // and must not clutter the label; `h` is the one that differs.
        let labels: Vec<&str> = v["worlds"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|w| w["label"].as_array().unwrap())
            .map(|s| s.as_str().unwrap())
            .collect();
        assert!(labels.contains(&"h") && labels.contains(&"!h"), "got {labels:?}");
        assert!(
            !labels.iter().any(|l| l.contains('d')),
            "invariant atoms must be dropped: {labels:?}"
        );
        assert_eq!(v["edges"].as_array().unwrap().len(), 1);
        assert_eq!(v["actions"].as_array().unwrap().len(), 3);
    }

    #[test]
    fn state_replays_a_trace_and_reaches_the_declared_goal() {
        let trace: Vec<String> = ["announce_not_heads()", "distract_a()", "peek_c()"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let v = json(&state(COIN, &trace));
        assert_eq!(v["ok"], true);
        assert_eq!(v["applied"].as_array().unwrap().len(), 3);
        assert_eq!(v["goal"], true, "the Coin Lie trace reaches its goal");
        assert!(v["trace_error"].is_null());
    }

    #[test]
    fn state_contracts_so_the_graph_stays_drawable() {
        // Twelve actions uncontracted is 8192 worlds — unreadable as a picture and slow
        // to produce. The quotient keeps it at 16.
        let cycle = ["announce_not_heads()", "distract_a()", "peek_c()"];
        let trace: Vec<String> = cycle.iter().cycle().take(12).map(|s| s.to_string()).collect();
        let v = json(&state(COIN, &trace));
        assert_eq!(v["ok"], true);
        let n = v["n_worlds"].as_u64().unwrap();
        assert!(n <= 32, "expected a contracted model, got {n} worlds");
    }

    #[test]
    fn a_rejected_file_carries_its_diagnostics_rather_than_failing_silently() {
        let v =
            json(&state("types{} objects{} agents{ ghost } props{} initially{} actions{}", &[]));
        assert_eq!(v["ok"], false);
        assert!(v["error"].as_str().unwrap().contains("ghost"));
    }

    #[test]
    fn an_unknown_or_inapplicable_action_is_reported_without_losing_the_state() {
        let v = json(&state(COIN, &["nosuch()".to_string()]));
        assert_eq!(v["ok"], true, "the file is fine; only the trace is not");
        assert!(v["trace_error"].as_str().unwrap().contains("nosuch()"));
        assert_eq!(v["applied"].as_array().unwrap().len(), 0);
        assert_eq!(v["n_worlds"], 2, "the state before the bad step is still reported");
    }

    #[test]
    fn eval_answers_against_the_state_the_trace_reaches_not_the_initial_one() {
        // The bug this guards is evaluating against `p.state`: `B[alice] B[carol] !h` is
        // false initially and true after the trace, so a stale evaluation is visible.
        let f = "B[alice] B[carol] !h";
        assert_eq!(json(&eval(COIN, &[], f))["value"], false);

        let trace: Vec<String> = ["announce_not_heads()", "distract_a()", "peek_c()"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert_eq!(json(&eval(COIN, &trace, f))["value"], true);
    }

    #[test]
    fn ask_enumerates_against_the_state_the_trace_reaches() {
        // The second-order false belief is found rather than guessed — which is the
        // reason to enumerate at all — and only after the trace, so this also pins
        // that `ask` uses the replayed state.
        let trace: Vec<String> = ["announce_not_heads()", "distract_a()", "peek_c()"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let v = json(&ask(COIN, &trace, "B[alice] B[carol] _", 0));
        assert_eq!(v["ok"], true);
        let ms: Vec<&str> =
            v["matches"].as_array().unwrap().iter().map(|m| m.as_str().unwrap()).collect();
        assert!(ms.iter().any(|m| m.contains("(!h)")), "got {ms:?}");
        assert!(!ms.contains(&"B[alice] B[carol] (h)"), "got {ms:?}");

        let before = json(&ask(COIN, &[], "B[alice] B[carol] _", 0));
        let bs: Vec<&str> =
            before["matches"].as_array().unwrap().iter().map(|m| m.as_str().unwrap()).collect();
        assert!(!bs.iter().any(|m| m.contains("(!h)")), "not yet true initially: {bs:?}");
    }

    #[test]
    fn ask_reports_a_pattern_without_a_hole_rather_than_returning_nothing() {
        let v = json(&ask(COIN, &[], "B[alice] h", 0));
        assert_eq!(v["ok"], false);
        assert!(v["error"].as_str().unwrap().contains('_'));
    }

    #[test]
    fn eval_reports_a_malformed_formula_with_its_span() {
        let v = json(&eval(COIN, &[], "K[nobody] h"));
        assert_eq!(v["ok"], false);
        assert!(v["error"].as_str().unwrap().contains("nobody"));
        assert!(v["value"].is_null());
    }
}
