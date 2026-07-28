//! Formula expressions. Precedence, loosest first: `->`, `|`, `&`, prefix `!`, modality.

use crate::ast::{Arg, Expr, Modal, Term};
use crate::{Diagnostics, Span, Tok, Token};

/// A cursor over tokens. Shared by the expression parser (this task) and the
/// section parser (Task 4).
pub struct Parser<'a> {
    toks: &'a [Token],
    pos: usize,
}

impl<'a> Parser<'a> {
    /// A parser positioned at the first token.
    ///
    /// # Panics
    /// If `toks` is empty; `lex` always appends [`Tok::Eof`], so this cannot happen
    /// for tokens it produced.
    pub fn new(toks: &'a [Token]) -> Self {
        debug_assert!(!toks.is_empty(), "token stream must end with Eof");
        Parser { toks, pos: 0 }
    }
    /// The current token without consuming it.
    pub fn peek(&self) -> &Tok {
        &self.toks[self.pos.min(self.toks.len() - 1)].tok
    }
    /// The token after the current one.
    pub fn peek2(&self) -> &Tok {
        &self.toks[(self.pos + 1).min(self.toks.len() - 1)].tok
    }
    /// The token `n` positions ahead of the cursor; `peek_at(0)` is [`Parser::peek`].
    /// Clamped to the trailing `Eof`, like `peek` and `peek2`.
    pub fn peek_at(&self, n: usize) -> &Tok {
        &self.toks[(self.pos + n).min(self.toks.len() - 1)].tok
    }
    /// The current token's span.
    pub fn span(&self) -> Span {
        self.toks[self.pos.min(self.toks.len() - 1)].span
    }
    /// Span of the token just consumed, or of the current one at the start of input.
    ///
    /// `Expr::span()` cannot serve here: a parenthesised expression carries the span of
    /// its *contents*, so `!(a | b)` ends before the closing parens. This gives the real
    /// end of whatever was last read, which is what quoting a construct back to the
    /// author needs.
    pub fn prev_span(&self) -> Span {
        self.toks[self.pos.saturating_sub(1).min(self.toks.len() - 1)].span
    }
    /// Consumes and returns the current token.
    pub fn bump(&mut self) -> Token {
        let t = self.toks[self.pos.min(self.toks.len() - 1)].clone();
        if self.pos < self.toks.len() - 1 {
            self.pos += 1;
        }
        t
    }
    /// Consumes the current token if it matches, reporting whether it did.
    pub fn eat(&mut self, want: &Tok) -> bool {
        if self.peek() == want {
            self.bump();
            true
        } else {
            false
        }
    }
    /// Consumes the current token if it matches, otherwise records a diagnostic.
    pub fn expect(&mut self, want: &Tok, what: &str, diags: &mut Diagnostics) -> bool {
        if self.eat(want) {
            true
        } else {
            diags.push(self.span(), format!("expected `{what}`"));
            false
        }
    }
    /// Whether input is exhausted.
    pub fn at_eof(&self) -> bool {
        matches!(self.peek(), Tok::Eof)
    }

    /// Parses a formula.
    pub fn parse_expr(&mut self, diags: &mut Diagnostics) -> Expr {
        self.parse_implies(diags)
    }

    fn parse_implies(&mut self, diags: &mut Diagnostics) -> Expr {
        let lhs = self.parse_or(diags);
        if self.eat(&Tok::Arrow) {
            let rhs = self.parse_implies(diags); // right-associative
            let sp = lhs.span().merge(rhs.span());
            return Expr::Implies(Box::new(lhs), Box::new(rhs), sp);
        }
        lhs
    }

    fn parse_or(&mut self, diags: &mut Diagnostics) -> Expr {
        let mut lhs = self.parse_and(diags);
        while self.eat(&Tok::Bar) {
            let rhs = self.parse_and(diags);
            let sp = lhs.span().merge(rhs.span());
            lhs = Expr::Or(Box::new(lhs), Box::new(rhs), sp);
        }
        lhs
    }

    fn parse_and(&mut self, diags: &mut Diagnostics) -> Expr {
        let mut lhs = self.parse_unary(diags);
        while self.eat(&Tok::Amp) {
            let rhs = self.parse_unary(diags);
            let sp = lhs.span().merge(rhs.span());
            lhs = Expr::And(Box::new(lhs), Box::new(rhs), sp);
        }
        lhs
    }

    fn parse_unary(&mut self, diags: &mut Diagnostics) -> Expr {
        if matches!(self.peek(), Tok::Bang) {
            let sp = self.span();
            self.bump();
            let inner = self.parse_unary(diags);
            let full = sp.merge(inner.span());
            return Expr::Not(Box::new(inner), full);
        }
        self.parse_primary(diags)
    }

    /// Recognises a modality keyword at the cursor, returning it and how many tokens
    /// it spans. `None` when the cursor is not at a modality.
    fn modal_at(&self) -> Option<(Modal, usize)> {
        match (self.peek(), self.peek2()) {
            (Tok::Box, _) => Some((Modal::Safe, 1)),
            (Tok::Question, _) => Some((Modal::Ignorant, 1)),
            (Tok::Undecided, _) => Some((Modal::Undecided, 1)),
            (Tok::Upper(k), Tok::Prime) => match k.as_str() {
                "K" => Some((Modal::KnowsDual, 2)),
                "B" => Some((Modal::BelievesDual, 2)),
                "S" => Some((Modal::SafeDual, 2)),
                _ => None,
            },
            (Tok::Upper(k), _) => match k.as_str() {
                "K" => Some((Modal::Knows, 1)),
                "B" => Some((Modal::Believes, 1)),
                "C" => Some((Modal::Common, 1)),
                "Kw" => Some((Modal::KnowsWhether, 1)),
                "Bw" => Some((Modal::BelievesWhether, 1)),
                _ => None,
            },
            _ => None,
        }
    }

    fn parse_primary(&mut self, diags: &mut Diagnostics) -> Expr {
        let start = self.span();

        if let Some((op, width)) = self.modal_at() {
            for _ in 0..width {
                self.bump();
            }
            // Optional `^psi` for conditional belief. `parse_unary`, not
            // `parse_primary`: the condition already reaches modalities through
            // `parse_primary`, so admitting prefix `!` adds no ambiguity — it only
            // stops `B^!q[a] p` collapsing into a cascade of unrelated complaints.
            // Anything looser would swallow the `[agents]` that has to follow.
            let cond =
                if self.eat(&Tok::Caret) { Some(Box::new(self.parse_unary(diags))) } else { None };
            self.expect(&Tok::LBracket, "[", diags);
            let agents = if self.eat(&Tok::Star) {
                None
            } else {
                let mut names = Vec::new();
                loop {
                    match self.peek().clone() {
                        Tok::Lower(n) => {
                            self.bump();
                            names.push(Arg::Obj(n));
                        }
                        // A variable is legal here so that a parameterised action can
                        // speak about its own parameter's beliefs, as in
                        // `share(?who) { pre B[?who] secret }`. It resolves through the
                        // same bindings as any other argument.
                        Tok::Var(n) => {
                            self.bump();
                            names.push(Arg::Var(n));
                        }
                        _ => {
                            diags.push(self.span(), "expected an agent name or `?variable`");
                            break;
                        }
                    }
                    if !self.eat(&Tok::Comma) {
                        break;
                    }
                }
                Some(names)
            };
            self.expect(&Tok::RBracket, "]", diags);
            let body = self.parse_unary(diags);
            let span = start.merge(body.span());
            return Expr::Modality { op, agents, cond, body: Box::new(body), span };
        }

        if self.eat(&Tok::LParen) {
            let e = self.parse_expr(diags);
            self.expect(&Tok::RParen, ")", diags);
            return e;
        }

        match self.peek().clone() {
            Tok::Hole => {
                self.bump();
                Expr::Hole(start)
            }
            Tok::Lower(name) if name == "true" => {
                self.bump();
                Expr::True(start)
            }
            Tok::Lower(name) if name == "false" => {
                self.bump();
                Expr::False(start)
            }
            Tok::Lower(name) => {
                self.bump();
                let mut args = Vec::new();
                let mut end = start;
                if self.eat(&Tok::LParen) {
                    if !matches!(self.peek(), Tok::RParen) {
                        loop {
                            match self.peek().clone() {
                                Tok::Lower(o) => {
                                    self.bump();
                                    args.push(Arg::Obj(o));
                                }
                                Tok::Var(v) => {
                                    self.bump();
                                    args.push(Arg::Var(v));
                                }
                                // A type name is only meaningful inside `constants`
                                // (§7.1). Accept it here so the one expression parser
                                // serves both; Task 7 rejects it elsewhere with a
                                // message that can name the offending argument.
                                Tok::Upper(t) => {
                                    self.bump();
                                    args.push(Arg::Ty(t));
                                }
                                _ => {
                                    diags.push(
                                        self.span(),
                                        "expected an object, `?variable`, or type name",
                                    );
                                    break;
                                }
                            }
                            if !self.eat(&Tok::Comma) {
                                break;
                            }
                        }
                    }
                    end = self.span();
                    self.expect(&Tok::RParen, ")", diags);
                }
                Expr::Atom(Term { pred: name, args, span: start.merge(end) })
            }
            _ => {
                diags.push(start, "expected a formula");
                self.bump();
                Expr::False(start)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ast::*, lex, Diagnostics};

    fn parse(src: &str) -> Expr {
        let mut d = Diagnostics::default();
        let toks = lex(src, &mut d);
        let mut p = Parser::new(&toks);
        let e = p.parse_expr(&mut d);
        assert!(d.is_empty(), "unexpected errors:\n{}", d.render(src));
        e
    }

    #[test]
    fn and_binds_tighter_than_or() {
        // a | b & c  ==  a | (b & c)
        match parse("a() | b() & c()") {
            Expr::Or(l, r, _) => {
                assert!(matches!(*l, Expr::Atom(_)));
                assert!(matches!(*r, Expr::And(_, _, _)), "rhs must be the conjunction");
            }
            other => panic!("expected Or, got {other:?}"),
        }
    }

    #[test]
    fn implication_is_right_associative_and_loosest() {
        // a -> b -> c  ==  a -> (b -> c)
        match parse("a() -> b() -> c()") {
            Expr::Implies(_, r, _) => assert!(matches!(*r, Expr::Implies(_, _, _))),
            other => panic!("expected Implies, got {other:?}"),
        }
    }

    #[test]
    fn negation_scopes_over_a_modality_not_inside_it() {
        // !K[a]p  ==  !(K[a]p)
        match parse("!K[a] p()") {
            Expr::Not(inner, _) => {
                assert!(matches!(*inner, Expr::Modality { op: Modal::Knows, .. }));
            }
            other => panic!("expected Not, got {other:?}"),
        }
    }

    #[test]
    fn agent_lists_are_preserved_for_lowering() {
        match parse("K[alice, bob] p()") {
            Expr::Modality { op: Modal::Knows, agents: Some(a), .. } => {
                assert_eq!(a, vec![Arg::Obj("alice".into()), Arg::Obj("bob".into())]);
            }
            other => panic!("expected Knows with two agents, got {other:?}"),
        }
    }

    #[test]
    fn a_variable_may_stand_where_an_agent_name_does() {
        // What makes `share(?who) { pre B[?who] secret(?whose) }` writable at all. The
        // parser must keep the variable rather than demanding a literal name, so that
        // grounding can substitute it like any other argument.
        match parse("B[?who] p()") {
            Expr::Modality { op: Modal::Believes, agents: Some(a), .. } => {
                assert_eq!(a, vec![Arg::Var("who".into())]);
            }
            other => panic!("expected Believes with a variable agent, got {other:?}"),
        }
        // Mixed lists too — a group modality may name some agents and bind others.
        match parse("C[alice, ?other] p()") {
            Expr::Modality { op: Modal::Common, agents: Some(a), .. } => {
                assert_eq!(a, vec![Arg::Obj("alice".into()), Arg::Var("other".into())]);
            }
            other => panic!("expected Common with a mixed list, got {other:?}"),
        }
    }

    #[test]
    fn common_knowledge_star_has_no_agent_list() {
        match parse("C[*] p()") {
            Expr::Modality { op: Modal::Common, agents: None, .. } => {}
            other => panic!("expected C[*], got {other:?}"),
        }
    }

    #[test]
    fn every_sugar_form_parses_to_its_own_operator() {
        let cases = [
            ("K'[a] p()", Modal::KnowsDual),
            ("B'[a] p()", Modal::BelievesDual),
            ("S'[a] p()", Modal::SafeDual),
            ("Kw[a] p()", Modal::KnowsWhether),
            ("Bw[a] p()", Modal::BelievesWhether),
            ("?[a] p()", Modal::Ignorant),
            ("??[a] p()", Modal::Undecided),
            ("[][a] p()", Modal::Safe),
        ];
        for (src, want) in cases {
            match parse(src) {
                Expr::Modality { op, .. } => assert_eq!(op, want, "for input {src}"),
                other => panic!("{src}: expected a modality, got {other:?}"),
            }
        }
    }

    #[test]
    fn conditional_belief_captures_both_operands() {
        // B^q[a] p  — the condition is q, the body is p.
        match parse("B^q()[a] p()") {
            Expr::Modality { op: Modal::Believes, cond: Some(c), body, .. } => {
                assert!(matches!(*c, Expr::Atom(ref t) if t.pred == "q"));
                assert!(matches!(*body, Expr::Atom(ref t) if t.pred == "p"));
            }
            other => panic!("expected conditional belief, got {other:?}"),
        }
    }

    #[test]
    fn a_negated_condition_needs_no_parentheses() {
        // The condition of a conditional belief parses with `parse_unary`, so prefix
        // `!` is admitted directly. With `parse_primary` there — which already reaches
        // modalities, so `B^K[b]q[a] p` worked — only `!` was excluded, and `B^!q[a] p`
        // produced a cascade of three unrelated diagnostics instead of one tree.
        //
        // `Expr` carries spans and derives `PartialEq`, so the two sources are padded
        // to put `!`, `q()`, and `p()` at identical byte offsets; the trees are then
        // equal outright rather than merely equal-up-to-spans.
        let bare = parse("B^ !q() [a] p()");
        let parens = parse("B^(!q())[a] p()");
        assert_eq!(bare, parens, "`B^!q[a] p` must parse as `B^(!q)[a] p`");
        match bare {
            Expr::Modality { op: Modal::Believes, cond: Some(c), .. } => match *c {
                Expr::Not(inner, _) => {
                    assert!(matches!(*inner, Expr::Atom(ref t) if t.pred == "q"));
                }
                other => panic!("expected the condition to be a negation, got {other:?}"),
            },
            other => panic!("expected a conditional belief, got {other:?}"),
        }
    }

    #[test]
    fn predicate_arguments_distinguish_objects_variables_and_types() {
        match parse("at(?a, study)") {
            Expr::Atom(t) => {
                assert_eq!(t.pred, "at");
                assert_eq!(t.args, vec![Arg::Var("a".into()), Arg::Obj("study".into())]);
            }
            other => panic!("expected an atom, got {other:?}"),
        }
        // Type names are accepted here so `constants { !adjacent(Location, Location) }`
        // parses; Task 7 rejects them outside `constants`.
        match parse("adjacent(Location, Location)") {
            Expr::Atom(t) => {
                assert_eq!(t.args, vec![Arg::Ty("Location".into()), Arg::Ty("Location".into())]);
            }
            other => panic!("expected an atom, got {other:?}"),
        }
    }

    #[test]
    fn a_missing_closing_paren_reports_a_span() {
        let mut d = Diagnostics::default();
        let toks = lex("(a() & b()", &mut d);
        let mut p = Parser::new(&toks);
        let _ = p.parse_expr(&mut d);
        assert_eq!(d.len(), 1);
        assert!(d.items()[0].message.contains(')'), "message should name the expected token");
    }
}
