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
    /// `_` — the hole in a query pattern, standing for a formula to be filled in.
    ///
    /// Part of the expression grammar rather than a textual placeholder so that filling
    /// it is a tree substitution: an underscore inside an identifier such as `at_park`
    /// can never be mistaken for it, and a filled pattern needs no parentheses because
    /// structure, not text, decides precedence. Lowering rejects it, since a hole has no
    /// meaning outside a query.
    Hole(Span),
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
        ///
        /// `Arg` rather than `String` so an action parameter can appear here:
        /// `share(?who) { pre B[?who] secret }` needs the modality's agent to be
        /// resolved through the same bindings as the rest of the formula. `Arg::Ty` is
        /// never legal in this position and is rejected at lowering.
        agents: Option<Vec<Arg>>,
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
            Expr::Hole(s) | Expr::True(s) | Expr::False(s) => *s,
            Expr::Atom(t) => t.span,
            Expr::Not(_, s) | Expr::And(_, _, s) | Expr::Or(_, _, s) | Expr::Implies(_, _, s) => *s,
            Expr::Modality { span, .. } => *span,
        }
    }
}

/// `Sub - Super` in the `types` section.
#[derive(Clone, Debug)]
pub struct TypeDecl {
    /// The subtype.
    pub name: String,
    /// Its immediate supertype.
    pub parent: String,
    /// Source location.
    pub span: Span,
}

/// `name - Type` in the `objects` section.
#[derive(Clone, Debug)]
pub struct ObjDecl {
    /// The object.
    pub name: String,
    /// Its type.
    pub ty: String,
    /// Source location.
    pub span: Span,
}

/// `pred(Type, Type)` in the `props` section.
#[derive(Clone, Debug)]
pub struct PropDecl {
    /// Predicate name.
    pub name: String,
    /// Parameter types, possibly empty.
    pub params: Vec<String>,
    /// Source location.
    pub span: Span,
}

/// A `constants` entry: a possibly-negated pattern over objects or types.
#[derive(Clone, Debug)]
pub struct ConstDecl {
    /// Whether the entry is negated.
    pub negated: bool,
    /// The pattern; arguments may name a type, which expands over its objects.
    pub term: Term,
}

/// One clause inside an `action` body.
#[derive(Clone, Debug)]
pub enum Clause {
    /// `actor <arg>`
    Actor(Arg, Span),
    /// `pre <expr>`
    Pre(Expr),
    /// `causes l0, l1 [if cond]`
    Causes {
        /// Literals: `(term, positive?)`.
        lits: Vec<(Term, bool)>,
        /// The `if` guard, if written.
        cond: Option<Expr>,
        /// Source location.
        span: Span,
    },
    /// `determines <expr>`
    Determines(Expr),
    /// `announces <expr>`
    Announces(Expr),
    /// `<arg> observes [if cond]`
    Observes {
        /// The observing agent, possibly a clause-scoped variable.
        who: Arg,
        /// The `if` guard, if written.
        cond: Option<Expr>,
        /// Source location.
        span: Span,
    },
    /// `<arg> aware [if cond]`
    Aware {
        /// The partially-observing agent.
        who: Arg,
        /// The `if` guard, if written.
        cond: Option<Expr>,
        /// Source location.
        span: Span,
    },
}

/// `?v - Type` in an action's parameter list.
#[derive(Clone, Debug)]
pub struct ParamDecl {
    /// Variable name, without the `?`.
    pub name: String,
    /// The type it ranges over.
    pub ty: String,
    /// Source location.
    pub span: Span,
}

/// An `action` declaration.
#[derive(Clone, Debug)]
pub struct ActionDecl {
    /// Action name.
    pub name: String,
    /// Parameters, possibly empty.
    pub params: Vec<ParamDecl>,
    /// Body clauses in source order.
    pub clauses: Vec<Clause>,
    /// Source location of the name.
    pub span: Span,
}

/// A world in an explicit `state` block.
#[derive(Clone, Debug)]
pub struct WorldDecl {
    /// World name.
    pub name: String,
    /// Whether it is the designated world (`*` prefix).
    pub designated: bool,
    /// Atoms true here; everything else is false.
    pub facts: Vec<Term>,
    /// Source location.
    pub span: Span,
}

/// How two worlds compare for one agent, in an explicit `state` block.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Cmp {
    /// `u ~ v` — both directions.
    Equi,
    /// `u < v` — `v` strictly more plausible; the converse must not hold.
    Lt,
    /// `u <= v` — `v` at least as plausible; says nothing about the converse.
    Le,
}

/// `agent: u < v` in an explicit `state` block.
#[derive(Clone, Debug)]
pub struct EdgeDecl {
    /// Whose relation.
    pub agent: String,
    /// Left world.
    pub from: String,
    /// Comparison.
    pub cmp: Cmp,
    /// Right world.
    pub to: String,
    /// Source location.
    pub span: Span,
}

/// The initial state, in whichever of the two forms was written (§7.3).
#[derive(Clone, Debug)]
pub enum Init {
    /// `initially { ... }` — every entry is a formula, some of which drive construction.
    Declarative(Vec<Expr>, Span),
    /// `state { ... }` — worlds and edges given explicitly.
    Explicit {
        /// The worlds.
        worlds: Vec<WorldDecl>,
        /// The plausibility edges.
        edges: Vec<EdgeDecl>,
        /// Source location.
        span: Span,
    },
}

/// A whole parsed file, before any checking.
#[derive(Clone, Debug, Default)]
pub struct Ast {
    /// `types`
    pub types: Vec<TypeDecl>,
    /// `objects`
    pub objects: Vec<ObjDecl>,
    /// `agents`
    pub agents: Vec<(String, Span)>,
    /// `props`
    pub props: Vec<PropDecl>,
    /// `constants`
    pub constants: Vec<ConstDecl>,
    /// `initially` or `state`
    pub init: Option<Init>,
    /// `goal`, if written.
    pub goal: Option<Expr>,
    /// `actions`
    pub actions: Vec<ActionDecl>,
}
