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

        let actions = ground_actions(&ast.actions, &sig, &consts, &mut store, &mut diags);

        match (state, diags.is_empty()) {
            (Some(state), true) => Ok(Problem { store, sig, consts, state, goal, actions }),
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
}

/// Reads and parses a file from disk. Read errors are reported with the path, so a
/// missing file reads the same way as a malformed one.
pub fn load(path: &str) -> Result<Problem, String> {
    let src = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
    Problem::parse(&src)
}
