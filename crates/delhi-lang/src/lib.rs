//! The delhi surface language: lexer, parser, grounding, and lowering to `delhi-mb`.
#![deny(missing_docs)]

pub mod ast;
pub mod lex;
pub mod parse_expr;
pub mod span;

pub use ast::{Arg, Expr, Modal, Term};
pub use lex::{lex, Tok, Token};
pub use parse_expr::Parser;
pub use span::{Diagnostic, Diagnostics, Span};
