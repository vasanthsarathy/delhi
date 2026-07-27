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
