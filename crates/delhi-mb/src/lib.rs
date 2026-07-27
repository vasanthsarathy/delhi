//! The mB+ backend: plausibility models, entailment, bisimulation, and the
//! action layer. See `docs/superpowers/specs/2026-07-25-delhi-core-design.md`.
#![deny(missing_docs)]

pub mod bits;
pub mod derived;
pub mod model;

pub use bits::Bits;
pub use derived::{common_closure, maxima, Derived};
pub use model::{FrameError, Model, State, WorldId};
