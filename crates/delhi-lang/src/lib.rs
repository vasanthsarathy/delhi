//! The delhi surface language: lexer, parser, grounding, and lowering to `delhi-mb`.
#![deny(missing_docs)]

pub mod ast;
pub mod constants;
pub mod ground;
pub mod init_decl;
pub mod init_explicit;
pub mod lex;
pub mod lower_action;
pub mod lower_formula;
pub mod parse_expr;
pub mod parse_file;
pub mod span;

pub use ast::{Arg, Ast, Expr, Modal, Term};
pub use constants::Constants;
pub use ground::{atom_key, Sig};
pub use init_decl::build_declarative;
pub use init_explicit::build_explicit;
pub use lex::{lex, Tok, Token};
pub use lower_action::{ground_actions, Ctx, GroundAction};
pub use lower_formula::{lower_formula, Bindings};
pub use parse_expr::Parser;
pub use parse_file::parse_file;
pub use span::{Diagnostic, Diagnostics, Span};
