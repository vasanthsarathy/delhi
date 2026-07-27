//! The delhi query language: hash-consed formulas over `L_GB`.
#![deny(missing_docs)]

pub mod formula;
pub mod symbol;

pub use formula::{AgentId, AgentMask, AtomId, FormulaId, Node, Store};
pub use symbol::Interner;
