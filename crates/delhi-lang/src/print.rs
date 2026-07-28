//! Rendering a state in the explicit `state` syntax (§7.3).

use crate::Sig;
use delhi_mb::State;

/// Names worlds `w0`, `w1`, … so the output parses back without needing the original
/// source's names.
fn world_name(i: usize) -> String {
    format!("w{i}")
}

/// Renders a state in the explicit `state` syntax of §7.3.
///
/// Reflexive edges are omitted because they are implicit, and a mutually related pair
/// prints once as `~`. Task 10 parses the result back to an equivalent state.
///
/// # Preconditions
/// `sig` must be the signature that built `st` — same agent count — or the printed
/// agent names will not match the state's actual structure.
///
/// Atom ids in `st` at or above `sig.n_atoms()` are *not* a precondition: they are
/// skipped, and that is the defined behaviour rather than a silent violation. A
/// `Model` pads its valuation to at least one bit, so a signature declaring no atoms
/// still yields a one-bit valuation with no name behind it, and a printer asked for
/// output must produce it rather than trip an assertion on a state it did not build.
pub fn print_state(st: &State, sig: &Sig) -> String {
    let m = &st.model;
    debug_assert!(
        m.n_agents == sig.n_agents(),
        "print_state: sig must be the signature that built st"
    );
    let mut out = String::from("state {\n");

    for w in 0..m.n_worlds {
        let star = if w == st.designated { "*" } else { " " };
        let facts: Vec<&str> = m.val[w]
            .ones()
            .into_iter()
            // See the doc comment: unnamed padding bits are skipped, not asserted away.
            .filter(|a| *a < sig.n_atoms())
            .map(|a| sig.atom_name(a as u32))
            .collect();
        out.push_str(&format!("  {star}{} <- {{ {} }}\n", world_name(w), facts.join(", ")));
    }

    if m.n_worlds > 1 {
        out.push('\n');
    }
    for i in 0..m.n_agents {
        let agent = sig.agent_name(i as u32);
        for u in 0..m.n_worlds {
            for v in (u + 1)..m.n_worlds {
                let uv = m.rel[i][u].get(v);
                let vu = m.rel[i][v].get(u);
                let line = match (uv, vu) {
                    (true, true) => Some(format!("{} ~ {}", world_name(u), world_name(v))),
                    (true, false) => Some(format!("{} < {}", world_name(u), world_name(v))),
                    (false, true) => Some(format!("{} < {}", world_name(v), world_name(u))),
                    (false, false) => None,
                };
                if let Some(l) = line {
                    out.push_str(&format!("  {agent}: {l}\n"));
                }
            }
        }
    }
    out.push_str("}\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{build_explicit, parse_file, Constants, Ctx, Diagnostics, Sig};
    use delhi_syntax::Store;

    const HEADER: &str = r#"
        types   { Actor - Object }
        objects { alice, carol - Actor }
        agents  { alice, carol }
        props   { h }
    "#;

    fn build(state_block: &str) -> (Sig, State) {
        let src = format!("{HEADER}\n{state_block}\nactions{{}}");
        let mut d = Diagnostics::default();
        let ast = parse_file(&src, &mut d);
        let sig = Sig::build(&ast, &mut d);
        let consts = Constants::build(&ast, &sig, &mut d);
        let ctx = Ctx { sig: &sig, consts: &consts };
        let (w, e, block) = match &ast.init {
            Some(crate::ast::Init::Explicit { worlds, edges, span }) => {
                (worlds.clone(), edges.clone(), *span)
            }
            _ => unreachable!(),
        };
        let mut store = Store::default();
        let st = build_explicit(&w, &e, block, &ctx, &mut store, &mut d).expect("builds");
        assert!(d.is_empty(), "{}", d.render(&src));
        (sig, st)
    }

    #[test]
    fn output_marks_the_designated_world_and_its_facts() {
        let (sig, st) = build("state { *u <- { h }, v <- { }, carol: v < u }");
        let out = print_state(&st, &sig);
        assert!(out.contains('*'), "the designated world is starred");
        assert!(out.contains('h'), "its facts are listed");
        assert!(out.starts_with("state {"), "output is the explicit form");
        assert!(out.trim_end().ends_with('}'));
    }

    #[test]
    fn a_mutual_pair_prints_once_as_tilde() {
        let (sig, st) = build("state { *u <- { h }, v <- { }, carol: u ~ v }");
        let out = print_state(&st, &sig);
        assert_eq!(out.matches('~').count(), 1, "one `~`, not two `<=` lines");
        assert!(!out.contains("<="), "a mutual pair must not also print as `<=`");
    }

    #[test]
    fn reflexive_edges_are_not_printed() {
        // A single-world state still has a reflexive edge for every agent (the
        // model constructor installs it), so this exercises the omission for real.
        // Printed worlds are renamed `w0`, `w1`, ... (see `world_name`), so check
        // for that name rather than the source name `u`.
        let (sig, st) = build("state { *u <- { h } }");
        let out = print_state(&st, &sig);
        assert!(
            !out.contains("w0 ~ w0") && !out.contains("w0 <= w0") && !out.contains("w0 < w0"),
            "reflexive edges are implicit:\n{out}"
        );
    }

    #[test]
    fn printing_then_reparsing_yields_an_equivalent_state() {
        // This is §7.3's actual requirement: the printed form must be readable back.
        let (sig, st) = build("state { *u <- { h }, v <- { }, carol: v < u }");
        let printed = print_state(&st, &sig);

        let src = format!("{HEADER}\n{printed}\nactions{{}}");
        let mut d = Diagnostics::default();
        let ast = parse_file(&src, &mut d);
        let sig2 = Sig::build(&ast, &mut d);
        let consts = Constants::build(&ast, &sig2, &mut d);
        let ctx = Ctx { sig: &sig2, consts: &consts };
        let (w, e, block) = match &ast.init {
            Some(crate::ast::Init::Explicit { worlds, edges, span }) => {
                (worlds.clone(), edges.clone(), *span)
            }
            other => panic!("printed output did not parse as a state: {other:?}\n{printed}"),
        };
        let mut store = Store::default();
        let round = build_explicit(&w, &e, block, &ctx, &mut store, &mut d)
            .unwrap_or_else(|| panic!("round trip failed:\n{}\n{}", printed, d.render(&src)));
        assert!(d.is_empty(), "round trip errors:\n{}", d.render(&src));
        assert!(st.equivalent(&round), "the round trip must preserve the state");
    }
}
