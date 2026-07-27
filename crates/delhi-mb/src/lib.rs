//! The mB+ backend: plausibility models, entailment, bisimulation, and the
//! action layer. See `docs/superpowers/specs/2026-07-25-delhi-core-design.md`.
#![deny(missing_docs)]

pub mod bisim;
pub mod bits;
pub mod build;
pub mod canon;
pub mod derived;
pub mod eval;
pub mod model;
pub mod theory;
pub mod update;

pub use bisim::{blocks_dynamic, blocks_full, refine, rels_dynamic, rels_full};
pub use bits::Bits;
pub use build::{build, ActionModel, EV_NPHI, EV_PHI, EV_TOP};
pub use derived::{common_closure, maxima, Derived};
pub use eval::Evaluator;
pub use model::{FrameError, Model, State, WorldId};
pub use theory::{ActionDef, Effect, Kind, TheoryError};
pub use update::UpdateRule;

impl delhi_core::EpistemicState for State {
    type Action = crate::ActionModel;
    type Store = delhi_syntax::Store;
    type Formula = delhi_syntax::FormulaId;

    fn entails(&self, store: &Self::Store, f: Self::Formula) -> bool {
        State::entails(self, store, f)
    }
    fn apply(&self, store: &Self::Store, action: &Self::Action) -> Option<Self> {
        State::apply(self, store, action)
    }
    fn contract_dynamic(&self) -> Self {
        let (m, map) = self.model.contract_dynamic();
        State { model: m, designated: map[self.designated] as usize }
    }
    fn equivalent(&self, other: &Self) -> bool {
        State::equivalent(self, other)
    }
    fn key(&self) -> Vec<u8> {
        State::key(self)
    }
}
