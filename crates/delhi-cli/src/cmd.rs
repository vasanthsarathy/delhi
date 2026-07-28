//! The subcommands. Each returns an exit code and writes to the provided sink, so the
//! tests can drive them without spawning a process.

use crate::style;
use delhi_lang::{print_state, Problem};
use delhi_mb::State;
use std::fmt::Write;

/// Parses `src`, or writes the diagnostics and returns `None`.
fn open(src: &str, out: &mut String) -> Option<Problem> {
    match Problem::parse(src) {
        Ok(p) => Some(p),
        Err(e) => {
            let _ = write!(out, "{}", style::bad(&e));
            None
        }
    }
}

/// `delhi check` — parse, ground, and validate. `0` if the file is accepted.
pub fn cmd_check(src: &str, out: &mut String) -> i32 {
    match open(src, out) {
        None => 1,
        Some(p) => {
            let _ = writeln!(
                out,
                "{} {} atoms, {} agents, {} ground actions, {} worlds",
                style::good("ok:"),
                p.sig.n_atoms(),
                p.sig.n_agents(),
                p.actions.len(),
                p.state.model.n_worlds
            );
            0
        }
    }
}

/// Renders what a state *means*: the facts of the actual world, and every agent's
/// attitude to every proposition.
///
/// `print_state` shows the model — worlds and plausibility edges — which is exact and
/// round-trips through the parser, but leaves the reader to work out what any of it
/// implies. This is the complementary view, and the one worth having open while
/// stepping through a scenario.
///
/// Each proposition falls into exactly one of the five cases the attitude table
/// distinguishes: the agent knows it, knows its negation, believes it without knowing,
/// believes its negation without knowing, or is undecided.
fn attitudes(p: &mut Problem, state: &State) -> String {
    let n_atoms = p.sig.n_atoms();
    let n_agents = p.sig.n_agents();

    let names: Vec<String> = (0..n_atoms).map(|a| p.sig.atom_name(a as u32).to_string()).collect();
    let agents: Vec<String> =
        (0..n_agents).map(|i| p.sig.agent_name(i as u32).to_string()).collect();

    let mut out = String::new();
    let facts: Vec<String> = (0..n_atoms)
        .map(|a| {
            if state.model.val[state.designated].get(a) {
                names[a].clone()
            } else {
                format!("!{}", names[a])
            }
        })
        .collect();
    let _ = writeln!(out, "{} {}", style::dim("actual world"), facts.join(", "));
    if n_agents == 0 || n_atoms == 0 {
        return out;
    }
    let _ = writeln!(out);

    // Build every query first: `entails` needs `&Store` while `knows`/`believes` need
    // `&mut Store`, so the two phases cannot interleave.
    let width = agents.iter().map(String::len).max().unwrap_or(0);
    for (i, agent) in agents.iter().enumerate() {
        let mut queries = Vec::with_capacity(n_atoms);
        for a in 0..n_atoms {
            let atom = p.store.atom(a as u32);
            let neg = p.store.not(atom);
            let kp = p.store.knows(i as u32, atom);
            let kn = p.store.knows(i as u32, neg);
            let bp = p.store.believes(i as u32, atom);
            let bn = p.store.believes(i as u32, neg);
            queries.push((kp, kn, bp, bn));
        }

        let (mut known, mut believed, mut undecided) = (Vec::new(), Vec::new(), Vec::new());
        for (a, (kp, kn, bp, bn)) in queries.into_iter().enumerate() {
            let pos = names[a].clone();
            let neg = format!("!{}", names[a]);
            if state.entails(&p.store, kp) {
                known.push(pos);
            } else if state.entails(&p.store, kn) {
                known.push(neg);
            } else if state.entails(&p.store, bp) {
                believed.push(pos);
            } else if state.entails(&p.store, bn) {
                believed.push(neg);
            } else {
                undecided.push(pos);
            }
        }

        let mut parts = Vec::new();
        if !known.is_empty() {
            parts.push(format!("{} {}", style::dim("knows"), known.join(", ")));
        }
        if !believed.is_empty() {
            parts.push(format!("{} {}", style::key("believes"), believed.join(", ")));
        }
        if !undecided.is_empty() {
            parts.push(format!("{} {}", style::dim("undecided"), undecided.join(", ")));
        }
        let _ = writeln!(out, "  {agent:<width$}  {}", parts.join("   "));
    }
    out
}

/// `delhi state` — the actual world's facts and every agent's attitude to every
/// proposition. The readable counterpart to `show`, which prints the model itself.
pub fn cmd_state(src: &str, out: &mut String) -> i32 {
    let Some(mut p) = open(src, out) else {
        return 1;
    };
    let state = p.state.clone();
    let text = attitudes(&mut p, &state);
    let _ = write!(out, "{text}");
    0
}

/// `delhi show` — print the initial state in the explicit form.
pub fn cmd_show(src: &str, out: &mut String) -> i32 {
    match open(src, out) {
        None => 1,
        Some(p) => {
            let _ = write!(out, "{}", print_state(&p.state, &p.sig));
            0
        }
    }
}

/// `delhi eval` — evaluate a formula in the initial state.
///
/// Exit code is `0` when the formula holds, `1` when it does not, and `2` when the
/// formula itself is malformed — so shell scripts can branch on the answer.
pub fn cmd_eval(src: &str, formula: &str, out: &mut String) -> i32 {
    let Some(mut p) = open(src, out) else {
        return 1;
    };
    match parse_query(&mut p, formula) {
        Err(e) => {
            let _ = write!(out, "{}", style::bad(&e));
            2
        }
        Ok(f) => {
            // Through `Problem::entails`, not `state.entails` directly: that is where
            // the "formula must come from this problem's store" precondition is stated
            // and checked, and `parse_query` lowers into exactly that store.
            let holds = p.entails(f);
            let _ = writeln!(out, "{}", verdict(holds));
            i32::from(!holds)
        }
    }
}

/// `true` or `false`, coloured — the one word a reader is scanning for.
fn verdict(b: bool) -> String {
    if b {
        style::good("true")
    } else {
        style::key("false")
    }
}

/// Parses a formula written on the command line against an already-checked problem.
///
/// The query shares the problem's signature and constants, so it can name any
/// declared proposition or agent.
pub fn parse_query(p: &mut Problem, text: &str) -> Result<delhi_syntax::FormulaId, String> {
    // A query is lowered exactly like a `goal`, by wrapping it in a throwaway file
    // whose other sections are copied from the problem. Rather than re-serialise the
    // whole problem, reuse the pieces already checked.
    let mut diags = delhi_lang::Diagnostics::default();
    let toks = delhi_lang::lex(text, &mut diags);
    let mut parser = delhi_lang::Parser::new(&toks);
    let expr = parser.parse_expr(&mut diags);
    let f = delhi_lang::lower_formula(
        &expr,
        &p.sig,
        &p.consts,
        &delhi_lang::Bindings::default(),
        &mut p.store,
        &mut diags,
    );
    if diags.is_empty() {
        Ok(f)
    } else {
        Err(diags.render(text))
    }
}

/// `delhi step` — apply a sequence of actions and print the resulting state.
pub fn cmd_step(src: &str, actions: &[String], out: &mut String) -> i32 {
    let Some(mut p) = open(src, out) else {
        return 1;
    };
    let n_agents = p.sig.n_agents();
    let mut state = p.state.clone();

    for name in actions {
        let Some(g) = p.actions.iter().find(|a| &a.name == name) else {
            let mut names: Vec<&str> = p.actions.iter().map(|a| a.name.as_str()).collect();
            names.sort_unstable();
            let _ = writeln!(out, "no action `{name}`; available: {}", names.join(", "));
            return 2;
        };
        let def = g.def.clone();
        let model = delhi_mb::build(&def, &mut p.store, n_agents);
        match state.apply(&p.store, &model) {
            Some(next) => {
                state = contracted(&next);
                let _ = writeln!(out, "{} {name}", style::dim("applied"));
            }
            None => {
                let _ = writeln!(out, "`{name}` is not applicable in the current state");
                return 1;
            }
        }
    }

    let _ = write!(out, "{}", print_state(&state, &p.sig));
    0
}

/// `delhi dot` — Graphviz for the initial state.
///
/// One node per world, doubled for the designated one, labelled with the atoms true
/// there. One edge per agent relation, reflexive edges omitted.
pub fn cmd_dot(src: &str, out: &mut String) -> i32 {
    let Some(p) = open(src, out) else {
        return 1;
    };
    let m = &p.state.model;
    let _ = writeln!(out, "digraph delhi {{");
    let _ = writeln!(out, "  rankdir=LR;");
    for w in 0..m.n_worlds {
        let facts: Vec<&str> = m.val[w]
            .ones()
            .into_iter()
            // A `Model` pads its valuation to at least one bit, so a signature with no
            // atoms still has a set bit with no name behind it. Skipping the unnamed
            // ones is the defined behaviour here, exactly as in `print_state`.
            .filter(|a| *a < p.sig.n_atoms())
            .map(|a| p.sig.atom_name(a as u32))
            .collect();
        let label = if facts.is_empty() { "∅".to_string() } else { facts.join(",") };
        let peripheries = if w == p.state.designated { " peripheries=2" } else { "" };
        let _ = writeln!(out, "  w{w} [shape=circle label=\"{label}\"{peripheries}];");
    }
    for i in 0..m.n_agents {
        let agent = p.sig.agent_name(i as u32);
        for u in 0..m.n_worlds {
            for v in m.rel[i][u].ones() {
                if u != v {
                    let _ = writeln!(out, "  w{u} -> w{v} [label=\"{agent}\"];");
                }
            }
        }
    }
    let _ = writeln!(out, "}}");
    0
}

/// Whether the interactive loop should keep going.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ReplOutcome {
    /// Read another line.
    Continue,
    /// Stop.
    Quit,
}

/// Handles one line of interactive input. Pure, so the loop can be tested without a
/// terminal — the reason command handling is separated from the loop at all.
pub fn repl_step(
    p: &mut Problem,
    state: &mut State,
    line: &str,
    out: &mut String,
) -> ReplOutcome {
    let line = line.trim();
    if line.is_empty() {
        return ReplOutcome::Continue;
    }
    if let Some(rest) = line.strip_prefix(':') {
        let (cmd, arg) = match rest.split_once(char::is_whitespace) {
            Some((c, a)) => (c, a.trim()),
            None => (rest, ""),
        };
        match cmd {
            "quit" | "q" => return ReplOutcome::Quit,
            "show" => {
                let _ = write!(out, "{}", print_state(state, &p.sig));
            }
            "state" | "s" => {
                let snapshot = state.clone();
                let text = attitudes(p, &snapshot);
                let _ = write!(out, "{text}");
            }
            "reset" => {
                *state = p.state.clone();
                let _ = writeln!(out, "reset to the initial state");
            }
            "actions" => {
                let mut names: Vec<&str> = p.actions.iter().map(|a| a.name.as_str()).collect();
                names.sort_unstable();
                let _ = writeln!(out, "{}", names.join("\n"));
            }
            "do" => {
                let n_agents = p.sig.n_agents();
                match p.actions.iter().find(|a| a.name == arg) {
                    None => {
                        let _ = writeln!(out, "no action `{arg}`; try :actions");
                    }
                    Some(g) => {
                        let def = g.def.clone();
                        let model = delhi_mb::build(&def, &mut p.store, n_agents);
                        match state.apply(&p.store, &model) {
                            Some(next) => {
                                *state = contracted(&next);
                                let _ = writeln!(out, "{} {arg}", style::dim("applied"));
                            }
                            None => {
                                let _ = writeln!(out, "`{arg}` is not applicable here");
                            }
                        }
                    }
                }
            }
            "help" | "h" => {
                let _ = writeln!(
                    out,
                    "<formula>     evaluate in the current state — any operator:\n\
                     \x20                K B [] B^psi C Kw Bw ? ?? K' B' S'\n\
                     :state        facts, and each agent's attitude to each proposition\n\
                     :do <action>  apply an action and keep the result\n\
                     :actions      list the ground actions\n\
                     :show         print the model itself, in the explicit form\n\
                     :reset        return to the initial state\n\
                     :quit         exit"
                );
            }
            other => {
                let _ = writeln!(out, "unknown command `:{other}` — try :help");
            }
        }
        return ReplOutcome::Continue;
    }

    match parse_query(p, line) {
        Ok(f) => {
            let _ = writeln!(out, "{}", verdict(state.entails(&p.store, f)));
        }
        Err(e) => {
            let _ = write!(out, "{e}");
        }
    }
    ReplOutcome::Continue
}

/// `delhi repl` — the interactive loop.
pub fn cmd_repl(src: &str) -> i32 {
    let mut buf = String::new();
    let Some(mut p) = open(src, &mut buf) else {
        print!("{buf}");
        return 1;
    };
    let mut state = p.state.clone();
    println!("{}", style::dim("delhi — :help for commands, :quit to exit"));
    loop {
        use std::io::Write as _;
        print!("{}", style::dim("> "));
        let _ = std::io::stdout().flush();
        let mut line = String::new();
        match std::io::stdin().read_line(&mut line) {
            Ok(0) => return 0, // EOF
            Ok(_) => {}
            Err(e) => {
                eprintln!("{e}");
                return 1;
            }
        }
        let mut out = String::new();
        let outcome = repl_step(&mut p, &mut state, &line, &mut out);
        print!("{out}");
        if outcome == ReplOutcome::Quit {
            return 0;
        }
    }
}

/// Quotients `state` by `~R`, remapping the designated world.
///
/// Applied after every update in `step` and the REPL. Product update multiplies worlds
/// by events, so without this a handful of actions produces thousands of worlds that are
/// pairwise indistinguishable — `delhi bench` shows Coin Lie reaching 8,192 worlds and
/// 9.5 seconds by its fourth cycle, against 16 worlds and 4.5 ms with this in place.
///
/// `~R` rather than `~D` because it is proved sound *and* a congruence for product
/// update, so it cannot change the answer to any query. `~D` merges more but its
/// congruence status is open (spec §6.3), which makes it unsafe to apply between updates.
fn contracted(state: &State) -> State {
    let (model, blocks) = state.model.contract_dynamic();
    let designated = blocks[state.designated] as usize;
    State { model, designated }
}

/// Which contraction to apply between updates, if any.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Contract {
    /// Never contract. Worlds accumulate as product update produces them.
    None,
    /// Quotient by `~R` after every update. Sound, a congruence, incomplete.
    Dynamic,
    /// Quotient by `~D` after every update. Complete for `K`/`B`/`□`/`C`;
    /// whether it is a congruence for product update is open (spec §6.3), which is
    /// exactly what running it here probes.
    Full,
}

impl Contract {
    fn label(self) -> &'static str {
        match self {
            Contract::None => "none",
            Contract::Dynamic => "~R",
            Contract::Full => "~D",
        }
    }
}

/// Worlds and elapsed nanoseconds after each cycle of the action list.
struct Trajectory {
    worlds: Vec<usize>,
    nanos: Vec<u128>,
    /// `false` if a cycle could not be completed (inapplicable, or the cap was hit).
    complete: bool,
}

/// Stops a run before it exhausts memory, and is checked after every *update* rather
/// than every cycle — a cycle of three actions can multiply worlds eightfold, so a
/// per-cycle check overshoots badly.
///
/// The bound that matters is the relation, which is `n_agents × n_worlds²` bits: at
/// 6,000 worlds and three agents that is already ~13 MB, and it grows quadratically.
const WORLD_CAP: usize = 6_000;

/// Runs `cycles` repetitions of `actions`, contracting as directed after each update.
fn trajectory(
    p: &mut Problem,
    actions: &[String],
    cycles: usize,
    how: Contract,
) -> (Trajectory, State) {
    let n_agents = p.sig.n_agents();
    let mut state = p.state.clone();
    let mut t = Trajectory { worlds: vec![state.model.n_worlds], nanos: vec![0], complete: true };

    // Event models depend only on the action, not the state, so build them once —
    // otherwise the benchmark measures `build` repeatedly rather than `apply`.
    let mut models = Vec::with_capacity(actions.len());
    for name in actions {
        let Some(g) = p.actions.iter().find(|a| &a.name == name) else {
            t.complete = false;
            return (t, state);
        };
        let def = g.def.clone();
        models.push(delhi_mb::build(&def, &mut p.store, n_agents));
    }

    let mut total = 0u128;
    for _ in 0..cycles {
        let start = std::time::Instant::now();
        for am in &models {
            let Some(next) = state.apply(&p.store, am) else {
                t.complete = false;
                t.nanos.push(total + start.elapsed().as_nanos());
                t.worlds.push(state.model.n_worlds);
                return (t, state);
            };
            state = next;
            if how != Contract::None {
                let (m, blocks) = match how {
                    Contract::Dynamic => state.model.contract_dynamic(),
                    _ => state.model.contract_full(),
                };
                let d = blocks[state.designated] as usize;
                state = State { model: m, designated: d };
            }
            // Checked here, not once per cycle: one cycle can multiply worlds by the
            // number of actions in it, so a per-cycle check overshoots the cap badly.
            if state.model.n_worlds > WORLD_CAP {
                t.complete = false;
                t.nanos.push(total + start.elapsed().as_nanos());
                t.worlds.push(state.model.n_worlds);
                return (t, state);
            }
        }
        total += start.elapsed().as_nanos();
        t.worlds.push(state.model.n_worlds);
        t.nanos.push(total);
    }
    (t, state)
}

/// Formats nanoseconds at a readable scale.
fn dur(n: u128) -> String {
    if n < 1_000 {
        format!("{n}ns")
    } else if n < 1_000_000 {
        format!("{:.1}us", n as f64 / 1e3)
    } else if n < 1_000_000_000 {
        format!("{:.1}ms", n as f64 / 1e6)
    } else {
        format!("{:.2}s", n as f64 / 1e9)
    }
}

/// `delhi bench` — how model size and update cost behave as actions accumulate.
///
/// Runs the same action list three times over: without contraction, quotienting by
/// `~R` after each update, and quotienting by `~D`. The first answers whether models
/// grow without bound; the other two answer how much of that growth is redundancy.
///
/// It also checks the three trajectories agree on every formula in the file's goal, so
/// a contraction that changed an answer would be visible rather than silent.
pub fn cmd_bench(src: &str, actions: &[String], cycles: usize, out: &mut String) -> i32 {
    let Some(mut p) = open(src, out) else {
        return 1;
    };
    if actions.is_empty() {
        let _ = writeln!(out, "nothing to benchmark: pass at least one action with -a");
        return 2;
    }
    for name in actions {
        if !p.actions.iter().any(|a| &a.name == name) {
            let mut names: Vec<&str> = p.actions.iter().map(|a| a.name.as_str()).collect();
            names.sort_unstable();
            let _ = writeln!(out, "no action `{name}`; available: {}", names.join(", "));
            return 2;
        }
    }

    let modes = [Contract::None, Contract::Dynamic, Contract::Full];
    let mut runs = Vec::new();
    for how in modes {
        runs.push(trajectory(&mut p, actions, cycles, how));
    }

    let _ = writeln!(
        out,
        "{} agents, {} atoms, {} worlds initially; one cycle = {}\n",
        p.sig.n_agents(),
        p.sig.n_atoms(),
        p.state.model.n_worlds,
        actions.join(" -> ")
    );
    let _ = writeln!(
        out,
        "{:>5}  {:>10} {:>10}  {:>10} {:>10}  {:>10} {:>10}",
        "cycle", "worlds", "cumul", "worlds ~R", "cumul", "worlds ~D", "cumul"
    );

    let longest = runs.iter().map(|(t, _)| t.worlds.len()).max().unwrap_or(0);
    for step in 0..longest {
        let _ = write!(out, "{step:>5}");
        for (t, _) in &runs {
            match (t.worlds.get(step), t.nanos.get(step)) {
                (Some(w), Some(n)) if step > 0 => {
                    let _ = write!(out, "  {:>10} {:>10}", w, dur(*n));
                }
                (Some(w), _) => {
                    let _ = write!(out, "  {:>10} {:>10}", w, "-");
                }
                _ => {
                    let _ = write!(out, "  {:>10} {:>10}", "-", "-");
                }
            }
        }
        let _ = writeln!(out);
    }

    for ((t, _), how) in runs.iter().zip(modes) {
        if !t.complete {
            let _ = writeln!(
                out,
                "\n{}: stopped after {} cycles (inapplicable action, or past the {WORLD_CAP}-world cap)",
                how.label(),
                t.worlds.len() - 1
            );
        }
    }

    // Do the three agree? A disagreement between `none` and `~R` would contradict a
    // proved congruence; one between `none` and `~D` is the open §6.3 question.
    if let Some(goal) = p.goal {
        let answers: Vec<bool> = runs.iter().map(|(_, s)| s.entails(&p.store, goal)).collect();
        let depths: Vec<usize> = runs.iter().map(|(t, _)| t.worlds.len()).collect();
        let comparable = depths.iter().all(|d| *d == depths[0]);
        let _ = write!(out, "\ngoal after the run:");
        for (a, how) in answers.iter().zip(modes) {
            let _ = write!(out, "  {}={}", how.label(), a);
        }
        let _ = writeln!(out);
        if comparable && answers.iter().any(|a| *a != answers[0]) {
            let _ = writeln!(
                out,
                "DISAGREEMENT: contraction changed the answer. For ~R that contradicts a proved\n\
                 congruence and means a bug; for ~D it is evidence on the open question in §6.3."
            );
            return 1;
        }
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOOD: &str = r#"
        types{ Actor - Object } objects{ a - Actor } agents{ a } props{ h }
        initially { h, ?[a] h, B[a] h }
        actions { look() { actor a, determines h, a observes } }
    "#;

    const BAD: &str = "types{} objects{} agents{ ghost } props{} initially{} actions{}";

    fn run(f: impl FnOnce(&mut String) -> i32) -> (i32, String) {
        let mut out = String::new();
        let code = f(&mut out);
        (code, out)
    }

    #[test]
    fn check_accepts_a_valid_file() {
        let (code, out) = run(|o| cmd_check(GOOD, o));
        assert_eq!(code, 0);
        assert!(out.to_lowercase().contains("ok"), "expected a success line, got: {out}");
    }

    #[test]
    fn check_rejects_and_explains() {
        let (code, out) = run(|o| cmd_check(BAD, o));
        assert_eq!(code, 1);
        assert!(out.contains("ghost"), "the diagnostic must name the problem");
    }

    #[test]
    fn show_emits_the_explicit_form() {
        let (code, out) = run(|o| cmd_show(GOOD, o));
        assert_eq!(code, 0);
        assert!(out.starts_with("state {"), "got: {out}");
        assert!(out.contains('*'), "the designated world is marked");
    }

    #[test]
    fn eval_reports_true_and_false_with_different_codes() {
        let (code, out) = run(|o| cmd_eval(GOOD, "K[a] h", o));
        assert_eq!(code, 1, "a is uncertain, so K[a]h is false");
        assert!(out.contains("false"));

        let (code, out) = run(|o| cmd_eval(GOOD, "B[a] h", o));
        assert_eq!(code, 0, "a believes h");
        assert!(out.contains("true"));
    }

    #[test]
    fn eval_reports_a_malformed_formula_rather_than_panicking() {
        let (code, out) = run(|o| cmd_eval(GOOD, "K[nobody] h", o));
        assert_eq!(code, 2);
        assert!(out.contains("nobody"));
    }

    const COIN: &str = r#"
        types{ Actor - Object } objects{ a, b - Actor } agents{ a, b } props{ h }
        initially { h, ?[a] h, B[a] h }
        actions {
            tell() { actor b, announces !h, a observes, b observes }
            look() { actor a, determines h, a observes }
        }
    "#;

    #[test]
    fn step_applies_a_sequence_and_prints_the_result() {
        let (code, out) = run(|o| cmd_step(COIN, &["tell()".to_string()], o));
        assert_eq!(code, 0, "got: {out}");
        assert!(out.contains("state {"), "the resulting state is printed");
    }

    #[test]
    fn step_reports_an_unknown_action_by_name() {
        let (code, out) = run(|o| cmd_step(COIN, &["nosuch()".to_string()], o));
        assert_eq!(code, 2);
        assert!(out.contains("nosuch()"));
        assert!(out.contains("tell()"), "the message should list what IS available");
    }

    #[test]
    fn step_reports_an_inapplicable_action_rather_than_panicking() {
        // `look()` senses h, whose two designated events have exhaustive preconditions,
        // so it always applies. Use a precondition that cannot hold instead.
        let src = r#"
            types{ Actor - Object } objects{ a - Actor } agents{ a } props{ h, q }
            initially { h }
            actions { go() { actor a, pre q, causes h, a observes } }
        "#;
        let (code, out) = run(|o| cmd_step(src, &["go()".to_string()], o));
        assert_eq!(code, 1);
        assert!(out.to_lowercase().contains("not applicable"), "got: {out}");
    }

    #[test]
    fn dot_emits_a_digraph_with_one_node_per_world() {
        let (code, out) = run(|o| cmd_dot(COIN, o));
        assert_eq!(code, 0);
        assert!(out.starts_with("digraph"), "got: {out}");
        assert_eq!(out.matches("shape=").count(), 2, "two worlds, two nodes");
        assert!(out.contains("peripheries=2"), "the designated world is doubled");
        assert!(out.contains("->"), "edges are drawn");
        assert!(out.trim_end().ends_with('}'));
    }

    #[test]
    fn dot_omits_reflexive_edges() {
        // Every world in COIN is reflexively related to itself for both agents (a
        // modal frame condition), so if the filter that skips `u == v` were removed,
        // self-loops like `w0 -> w0` would appear. `starts_with("digraph")` alone
        // would not catch this, so check for the absence of every self-loop directly.
        let (code, out) = run(|o| cmd_dot(COIN, o));
        assert_eq!(code, 0);
        for w in 0..2 {
            assert!(
                !out.contains(&format!("w{w} -> w{w}")),
                "reflexive edges must not be drawn:\n{out}"
            );
        }
    }

    fn repl_on(src: &str, lines: &[&str]) -> (Vec<ReplOutcome>, String) {
        let mut p = Problem::parse(src).expect("parses");
        let mut state = p.state.clone();
        let mut out = String::new();
        let mut outcomes = Vec::new();
        for l in lines {
            outcomes.push(repl_step(&mut p, &mut state, l, &mut out));
        }
        (outcomes, out)
    }

    #[test]
    fn repl_evaluates_a_bare_formula() {
        let (o, out) = repl_on(COIN, &["B[a] h"]);
        assert_eq!(o, vec![ReplOutcome::Continue]);
        assert!(out.contains("true"), "got: {out}");
    }

    #[test]
    fn repl_applies_an_action_and_the_state_persists() {
        // After `tell()` announces !h, a should believe !h rather than h.
        let (_, out) = repl_on(COIN, &[":do tell()", "B[a] h", "B[a] !h"]);
        let lines: Vec<&str> = out.lines().filter(|l| *l == "true" || *l == "false").collect();
        assert_eq!(lines, vec!["false", "true"], "the applied action must persist; got:\n{out}");
    }

    #[test]
    fn repl_reset_returns_to_the_initial_state() {
        let (_, out) = repl_on(COIN, &[":do tell()", ":reset", "B[a] h"]);
        assert!(out.lines().any(|l| l == "true"), "after reset a believes h again:\n{out}");
    }

    #[test]
    fn repl_quit_stops_and_unknown_commands_do_not() {
        let (o, _) = repl_on(COIN, &[":quit"]);
        assert_eq!(o, vec![ReplOutcome::Quit]);
        let (o, out) = repl_on(COIN, &[":nonsense"]);
        assert_eq!(o, vec![ReplOutcome::Continue]);
        assert!(out.contains(":help"), "an unknown command should point at help");
    }

    #[test]
    fn repl_reports_a_bad_formula_without_stopping() {
        let (o, out) = repl_on(COIN, &["K[nobody] h"]);
        assert_eq!(o, vec![ReplOutcome::Continue], "a bad query must not end the session");
        assert!(out.contains("nobody"));
    }

    #[test]
    fn repl_lists_the_available_actions() {
        let (_, out) = repl_on(COIN, &[":actions"]);
        assert!(out.contains("tell()") && out.contains("look()"));
    }

    #[test]
    fn step_contracts_so_worlds_do_not_accumulate() {
        // Two applications of an action that keeps producing distinguishable events
        // would give 2 -> 4 -> 8 worlds uncontracted. `tell()` announces the same thing
        // twice, so the second application adds nothing an agent can tell apart and the
        // quotient collapses it back.
        let mut p = Problem::parse(COIN).expect("parses");
        let n_agents = p.sig.n_agents();
        let def = p.action("tell()").expect("action").def.clone();
        let am = delhi_mb::build(&def, &mut p.store, n_agents);

        let once = p.state.apply(&p.store, &am).expect("applicable");
        let twice = once.apply(&p.store, &am).expect("applicable");
        let twice_contracted = contracted(&contracted(&once).apply(&p.store, &am).expect("ok"));

        assert!(
            twice_contracted.model.n_worlds < twice.model.n_worlds,
            "contraction should shrink the model: {} vs {}",
            twice_contracted.model.n_worlds,
            twice.model.n_worlds
        );
        // `~R` is a congruence, so the contracted run must answer every query the same.
        assert!(
            twice.equivalent(&twice_contracted),
            "contraction must not change what the state models"
        );
    }

    #[test]
    fn state_separates_knowing_from_merely_believing() {
        // In COIN, `b` knows h outright while `a` only believes it — the whole point of
        // the view is that those read differently. A version that reported both as
        // "knows" would still look plausible, so assert the distinction directly.
        let (code, out) = run(|o| cmd_state(COIN, o));
        assert_eq!(code, 0, "got: {out}");
        assert!(out.contains("actual world"), "got: {out}");

        let line_a = out.lines().find(|l| l.trim_start().starts_with("a ")).expect("a's line");
        let line_b = out.lines().find(|l| l.trim_start().starts_with("b ")).expect("b's line");
        assert!(line_a.contains("believes h"), "a believes h without knowing: {line_a}");
        assert!(!line_a.contains("knows h"), "a must not be reported as knowing: {line_a}");
        assert!(line_b.contains("knows h"), "b knows h: {line_b}");
    }

    #[test]
    fn state_reports_undecided_when_an_agent_leans_neither_way() {
        // Uncertainty with no belief declaration leaves the plausibility order flat, so
        // both worlds are maximal and the agent believes neither h nor !h. Without this
        // branch such an agent would be silently omitted from its own line.
        let src = r#"
            types{ Actor - Object } objects{ a - Actor } agents{ a } props{ h }
            initially { h, ?[a] h }
            actions {}
        "#;
        let (code, out) = run(|o| cmd_state(src, o));
        assert_eq!(code, 0, "got: {out}");
        assert!(out.contains("undecided h"), "got: {out}");
        assert!(!out.contains("believes"), "a flat order is not a belief: {out}");
    }

    #[test]
    fn repl_state_follows_the_current_state_not_the_initial_one() {
        // The bug this guards is reading `p.state` instead of the live `state` — which
        // would look right on the first call and never change afterwards.
        let (_, out) = repl_on(COIN, &[":state", ":do look()", ":state"]);
        let believes = out.matches("believes h").count();
        let knows = out.matches("knows h").count();
        assert_eq!(believes, 1, "only the first snapshot has a believing: {out}");
        assert!(knows >= 2, "after look(), a knows h too: {out}");
    }

    #[test]
    fn state_rejects_a_bad_file() {
        let (code, out) = run(|o| cmd_state(BAD, o));
        assert_eq!(code, 1);
        assert!(out.contains("ghost"), "got: {out}");
    }

    #[test]
    fn bench_shows_contraction_bounding_a_run_that_would_otherwise_grow() {
        // The headline claim, asserted rather than eyeballed: uncontracted the model
        // grows, contracted it does not. Three cycles is enough for the gap to open.
        let (code, out) = run(|o| cmd_bench(COIN, &["tell()".to_string()], 3, o));
        assert_eq!(code, 0, "got: {out}");
        assert!(out.contains("worlds ~R") && out.contains("worlds ~D"), "got: {out}");

        let mut p = Problem::parse(COIN).expect("parses");
        let acts = vec!["tell()".to_string()];
        let (plain, _) = trajectory(&mut p, &acts, 3, Contract::None);
        let (small, _) = trajectory(&mut p, &acts, 3, Contract::Dynamic);
        assert!(
            small.worlds.last() < plain.worlds.last(),
            "contraction should bound growth: {:?} vs {:?}",
            small.worlds,
            plain.worlds
        );
        // And bounded means *bounded*, not merely smaller — the last two cycles agree.
        let n = small.worlds.len();
        assert_eq!(
            small.worlds[n - 1],
            small.worlds[n - 2],
            "contracted size should reach a fixed point: {:?}",
            small.worlds
        );
    }

    #[test]
    fn bench_rejects_bad_input_without_running_anything() {
        let (code, out) = run(|o| cmd_bench(COIN, &["nosuch()".to_string()], 2, o));
        assert_eq!(code, 2);
        assert!(out.contains("nosuch()") && out.contains("tell()"), "got: {out}");

        let (code, out) = run(|o| cmd_bench(COIN, &[], 2, o));
        assert_eq!(code, 2, "an empty action list is a usage error");
        assert!(out.contains("-a"), "got: {out}");
    }

    #[test]
    fn contracted_preserves_the_designated_world() {
        // The quotient renumbers worlds, so the designated index must be remapped
        // through the block map rather than carried over. If it were not, the starred
        // world would drift to whichever block happened to take that index.
        let mut p = Problem::parse(COIN).expect("parses");
        let n_agents = p.sig.n_agents();
        let def = p.action("tell()").expect("action").def.clone();
        let am = delhi_mb::build(&def, &mut p.store, n_agents);
        let after = p.state.apply(&p.store, &am).expect("applicable");

        let c = contracted(&after);
        assert_eq!(
            after.model.val[after.designated].ones(),
            c.model.val[c.designated].ones(),
            "the designated world's facts must survive the quotient"
        );
    }
}
