//! The delhi surface language: lexer, parser, grounding, and lowering to `delhi-mb`.
#![deny(missing_docs)]

pub mod lex;
pub mod span;

pub use lex::{lex, Tok, Token};
pub use span::{Diagnostic, Diagnostics, Span};
