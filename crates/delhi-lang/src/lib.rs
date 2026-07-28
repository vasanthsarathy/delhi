//! The delhi surface language: lexer, parser, grounding, and lowering to `delhi-mb`.
#![deny(missing_docs)]

pub mod ast;
pub mod constants;
pub mod ground;
pub mod lex;
pub mod parse_expr;
pub mod parse_file;
pub mod span;

pub use ast::{Arg, Ast, Expr, Modal, Term};
pub use constants::Constants;
pub use ground::{atom_key, Sig};
pub use lex::{lex, Tok, Token};
pub use parse_expr::Parser;
pub use parse_file::parse_file;
pub use span::{Diagnostic, Diagnostics, Span};
