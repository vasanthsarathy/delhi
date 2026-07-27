//! The mB+ backend: plausibility models, entailment, bisimulation, and the
//! action layer. See `docs/superpowers/specs/2026-07-25-delhi-core-design.md`.
#![deny(missing_docs)]

pub mod bisim;
pub mod bits;
pub mod canon;
pub mod derived;
pub mod eval;
pub mod model;

pub use bisim::{blocks_dynamic, blocks_full, refine, rels_dynamic, rels_full};
pub use bits::Bits;
pub use derived::{common_closure, maxima, Derived};
pub use eval::Evaluator;
pub use model::{FrameError, Model, State, WorldId};
