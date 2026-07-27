//! Abstract syntax. Types only — no logic lives here.

use crate::Span;

/// An argument to a predicate.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Arg {
    /// A declared object, e.g. `alice`.
    Obj(String),
    /// A variable, e.g. `?a`.
    Var(String),
    /// A type name, e.g. `Location`. Legal **only** inside `constants`, where it
    /// expands over that type's objects (§7.1). Task 7 rejects it anywhere else.
    Ty(String),
}

/// An applied predicate, e.g. `at(?a, study)`.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Term {
    /// Predicate name.
    pub pred: String,
    /// Arguments, possibly empty.
    pub args: Vec<Arg>,
    /// Source location.
    pub span: Span,
}

/// Which modal operator, including the sugar forms of §7.4.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Modal {
    /// `K[..]`
    Knows,
    /// `B[..]`
    Believes,
    /// `[][..]` or `□[..]`
    Safe,
    /// `C[..]` — `None` agents means `C[*]`, every declared agent.
    Common,
    /// `K'[..]`
    KnowsDual,
    /// `B'[..]`
    BelievesDual,
    /// `S'[..]`
    SafeDual,
    /// `Kw[..]`
    KnowsWhether,
    /// `Bw[..]`
    BelievesWhether,
    /// `?[..]`
    Ignorant,
    /// `??[..]` or `¿[..]`
    Undecided,
}

/// A formula as written.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Expr {
    /// `true`
    True(Span),
    /// `false`
    False(Span),
    /// An applied predicate.
    Atom(Term),
    /// `!e`
    Not(Box<Expr>, Span),
    /// `a & b`
    And(Box<Expr>, Box<Expr>, Span),
    /// `a | b`
    Or(Box<Expr>, Box<Expr>, Span),
    /// `a -> b`
    Implies(Box<Expr>, Box<Expr>, Span),
    /// A modality applied to agents and a body. `agents` is `None` only for `C[*]`.
    Modality {
        /// Which operator.
        op: Modal,
        /// The agent list, or `None` for `C[*]`.
        agents: Option<Vec<String>>,
        /// The `ψ` of a conditional belief `B^ψ[a]φ`, otherwise `None`.
        cond: Option<Box<Expr>>,
        /// The operand.
        body: Box<Expr>,
        /// Source location.
        span: Span,
    },
}

impl Expr {
    /// Where this expression came from.
    pub fn span(&self) -> Span {
        match self {
            Expr::True(s) | Expr::False(s) => *s,
            Expr::Atom(t) => t.span,
            Expr::Not(_, s) | Expr::And(_, _, s) | Expr::Or(_, _, s) | Expr::Implies(_, _, s) => *s,
            Expr::Modality { span, .. } => *span,
        }
    }
}
