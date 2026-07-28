//! The front end assembled: a checked [`Problem`], and [`load`] to read one from disk.

use crate::ast::Init;
use crate::lower_formula::{lower_formula, Bindings};
use crate::{
    build_declarative, build_explicit, ground_actions, parse_file, Constants, Ctx, Diagnostics,
    GroundAction, Sig,
};
use delhi_mb::State;
use delhi_syntax::{FormulaId, Store};

/// A fully checked problem: signature, initial state, goal, and ground actions.
///
/// `Debug` is derived because `Problem::parse` returns it in a `Result`, and
/// `Result::unwrap_err` requires the success type to be printable.
#[derive(Debug)]
pub struct Problem {
    /// Formula arena shared by everything below.
    pub store: Store,
    /// The checked signature.
    pub sig: Sig,
    /// The constant table, kept so later queries lower against the same one.
    pub consts: Constants,
    /// The initial state.
    pub state: State,
    /// The declared goal, if the file had one.
    pub goal: Option<FormulaId>,
    /// Declared invariants, each with the source text that wrote it.
    ///
    /// The text is kept so a violation can name the constraint as the author wrote it
    /// rather than as a formula id or a re-rendering.
    pub invariants: Vec<(FormulaId, String)>,
    /// Every ground action whose precondition is satisfiable.
    pub actions: Vec<GroundAction>,
}

impl Problem {
    /// Parses and checks a source file.
    ///
    /// On failure returns every diagnostic rendered against the source, so one call
    /// reports all the problems rather than only the first. A construction that
    /// produced a state but also raised a diagnostic is a failure too: the state is
    /// only as trustworthy as the checks that passed alongside it.
    pub fn parse(src: &str) -> Result<Problem, String> {
        let mut diags = Diagnostics::default();
        let ast = parse_file(src, &mut diags);
        let sig = Sig::build(&ast, &mut diags);
        let consts = Constants::build(&ast, &sig, &mut diags);
        let mut store = Store::default();

        let state = {
            let ctx = Ctx { sig: &sig, consts: &consts };
            match &ast.init {
                // The block's own span goes through: it is what whole-block failures
                // are reported against, and reconstructing one from the entries would
                // blame an arbitrary entry (or, for an empty block, byte zero).
                Some(Init::Declarative(items, block)) => {
                    build_declarative(items, *block, &ctx, &mut store, &mut diags)
                }
                Some(Init::Explicit { worlds, edges, span }) => {
                    build_explicit(worlds, edges, *span, &ctx, &mut store, &mut diags)
                }
                None => None,
            }
        };

        let goal = ast.goal.as_ref().map(|g| {
            lower_formula(g, &sig, &consts, &Bindings::default(), &mut store, &mut diags)
        });

        let invariants: Vec<(FormulaId, String)> = ast
            .invariants
            .iter()
            .map(|(e, sp)| {
                let f =
                    lower_formula(e, &sig, &consts, &Bindings::default(), &mut store, &mut diags);
                (f, src[sp.start.min(src.len())..sp.end.min(src.len())].trim().to_string())
            })
            .collect();

        let actions = ground_actions(&ast.actions, &sig, &consts, &mut store, &mut diags);

        match (state, diags.is_empty()) {
            (Some(state), true) => Ok(Problem { store, sig, consts, state, goal, invariants, actions }),
            _ => Err(diags.render(src)),
        }
    }

    /// A ground action by its display name, e.g. `move(alice,hall,study)`. A
    /// zero-parameter action keeps its empty argument list, so `peek_c` is `peek_c()`.
    pub fn action(&self, name: &str) -> Option<&GroundAction> {
        self.actions.iter().find(|a| a.name == name)
    }

    /// Whether the initial state models `f`.
    ///
    /// Precondition: `f` was produced by this problem's [`Problem::store`].
    pub fn entails(&self, f: FormulaId) -> bool {
        debug_assert!(
            (f as usize) < self.store.len(),
            "formula must come from this problem's store"
        );
        self.state.entails(&self.store, f)
    }

    /// The declared invariants that `state` violates, as the author wrote them.
    ///
    /// Takes the state rather than using `self.state`, because the point of an invariant
    /// is that it is checked *after every action* — a version that could only inspect the
    /// initial state would be a slower way of writing an `initially` entry.
    pub fn violated(&self, state: &State) -> Vec<&str> {
        self.invariants
            .iter()
            .filter(|(f, _)| !state.entails(&self.store, *f))
            .map(|(_, text)| text.as_str())
            .collect()
    }
}


/// Reads and parses a file from disk. Read errors are reported with the path, so a
/// missing file reads the same way as a malformed one.
pub fn load(path: &str) -> Result<Problem, String> {
    let src = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
    Problem::parse(&src)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SRC: &str = r#"
        types{ Actor - Object } objects{ a, b - Actor } agents{ a, b } props{ p }
        initially { p, ?[a] p, B[a] p }
        invariants {
            !((B[a] p & B[b] !p) | (B[a] !p & B[b] p))
            K[b] p
        }
        actions { lie() { actor b, announces !p, a observes, b observes } }
    "#;

    #[test]
    fn an_invariant_holding_initially_can_still_be_broken_by_an_action() {
        // The whole point of an invariant over an `initially` assertion: it is checked
        // against states the file never mentions.
        let mut p = Problem::parse(SRC).unwrap_or_else(|e| panic!("{e}"));
        assert!(p.violated(&p.state).is_empty(), "clean at the start");

        let n = p.sig.n_agents();
        let def = p.action("lie()").expect("action").def.clone();
        let am = delhi_mb::build(&def, &mut p.store, n);
        let after = p.state.clone().apply(&p.store, &am).expect("applicable");

        let bad = p.violated(&after);
        assert_eq!(bad.len(), 1, "exactly the disagreement one: {bad:?}");
        assert!(bad[0].starts_with("!(("), "got {:?}", bad[0]);
    }

    #[test]
    fn a_violation_quotes_the_constraint_exactly_as_written() {
        // Guards a real bug: `Expr::span()` of a parenthesised expression covers only its
        // contents, so slicing by it truncated `!(a | b)` to `!(a | b`. The span is taken
        // from the parser's token positions instead.
        let p = Problem::parse(SRC).unwrap_or_else(|e| panic!("{e}"));
        let texts: Vec<&str> = p.invariants.iter().map(|(_, t)| t.as_str()).collect();
        assert_eq!(texts[0], "!((B[a] p & B[b] !p) | (B[a] !p & B[b] p))");
        assert_eq!(texts[1], "K[b] p");
        for t in &texts {
            assert_eq!(
                t.matches('(').count(),
                t.matches(')').count(),
                "parens must balance in the quoted text: {t}"
            );
        }
    }

    #[test]
    fn a_file_with_no_invariants_section_has_none_and_violates_nothing() {
        let p = Problem::parse(
            r#"types{} objects{} agents{} props{ p } initially{ p } actions{}"#,
        )
        .unwrap_or_else(|e| panic!("{e}"));
        assert!(p.invariants.is_empty());
        assert!(p.violated(&p.state).is_empty());
    }
}
