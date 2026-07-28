//! Section parsing. Sections may appear in any order, at most once each.

use crate::ast::*;
use crate::{lex, Diagnostics, Parser, Span, Tok};

/// Parses a whole source file. Always returns an [`Ast`]; consult `diags` for errors.
pub fn parse_file(src: &str, diags: &mut Diagnostics) -> Ast {
    let toks = lex(src, diags);
    let mut p = Parser::new(&toks);
    let mut ast = Ast::default();
    let mut seen: Vec<String> = Vec::new();

    while !p.at_eof() {
        let head_span = p.span();
        let name = match p.peek().clone() {
            Tok::Lower(n) => {
                p.bump();
                n
            }
            _ => {
                diags.push(head_span, "expected a section name");
                p.bump();
                continue;
            }
        };
        if seen.contains(&name) {
            diags.push(head_span, format!("duplicate section `{name}`"));
        }
        seen.push(name.clone());
        if (name == "initially" && seen.iter().any(|s| s == "state"))
            || (name == "state" && seen.iter().any(|s| s == "initially"))
        {
            diags.push(head_span, "use either `initially` or `state`, not both");
        }
        if !p.expect(&Tok::LBrace, "{", diags) {
            continue;
        }
        match name.as_str() {
            "types" => parse_types(&mut p, &mut ast, diags),
            "objects" => parse_objects(&mut p, &mut ast, diags),
            "agents" => parse_agents(&mut p, &mut ast, diags),
            "props" => parse_props(&mut p, &mut ast, diags),
            "constants" => parse_constants(&mut p, &mut ast, diags),
            "define" => parse_defines(&mut p, &mut ast, diags),
            "initially" => parse_initially(&mut p, &mut ast, diags, head_span),
            "state" => parse_state(&mut p, &mut ast, diags, head_span),
            "goal" => {
                if !matches!(p.peek(), Tok::RBrace) {
                    ast.goal = Some(p.parse_expr(diags));
                }
                p.expect(&Tok::RBrace, "}", diags);
            }
            "invariants" => {
                while !matches!(p.peek(), Tok::RBrace | Tok::Eof) {
                    // The entry's span runs from its first token to its last consumed
                    // one. `Expr::span()` will not do: a parenthesised expression carries
                    // the span of its *contents*, so quoting `!(a | b)` back to the
                    // author would drop the closing parens.
                    let from = p.span();
                    let e = p.parse_expr(diags);
                    let sp = from.merge(p.prev_span());
                    ast.invariants.push((e, sp));
                    p.eat(&Tok::Comma);
                }
                p.expect(&Tok::RBrace, "}", diags);
            }
            "actions" => parse_actions(&mut p, &mut ast, diags),
            other => {
                diags.push(head_span, format!("unknown section `{other}`"));
                skip_block(&mut p);
            }
        }
    }

    for required in ["types", "objects", "agents", "props", "actions"] {
        if !seen.iter().any(|s| s == required) {
            diags.push(Span::new(0, 0), format!("missing required section `{required}`"));
        }
    }
    if ast.init.is_none() {
        diags.push(Span::new(0, 0), "missing an initial state: use `initially` or `state`");
    }
    ast
}

/// Consumes tokens up to and including the matching `}`.
fn skip_block(p: &mut Parser) {
    let mut depth = 1;
    while !p.at_eof() && depth > 0 {
        match p.peek() {
            Tok::LBrace => depth += 1,
            Tok::RBrace => depth -= 1,
            _ => {}
        }
        p.bump();
    }
}

fn parse_types(p: &mut Parser, ast: &mut Ast, diags: &mut Diagnostics) {
    while !matches!(p.peek(), Tok::RBrace | Tok::Eof) {
        let sp = p.span();
        let name = match p.peek().clone() {
            Tok::Upper(n) => { p.bump(); n }
            _ => { diags.push(sp, "expected a type name (types start uppercase)"); p.bump(); continue; }
        };
        p.expect(&Tok::Dash, "-", diags);
        let parent = match p.peek().clone() {
            Tok::Upper(n) => { p.bump(); n }
            _ => { diags.push(p.span(), "expected a supertype name"); "Object".to_string() }
        };
        ast.types.push(TypeDecl { name, parent, span: sp.merge(p.span()) });
        p.eat(&Tok::Comma);
    }
    p.expect(&Tok::RBrace, "}", diags);
}

fn parse_objects(p: &mut Parser, ast: &mut Ast, diags: &mut Diagnostics) {
    while !matches!(p.peek(), Tok::RBrace | Tok::Eof) {
        // `a, b, c - Type` declares three objects of one type.
        let mut group: Vec<(String, Span)> = Vec::new();
        loop {
            let sp = p.span();
            match p.peek().clone() {
                Tok::Lower(n) => { p.bump(); group.push((n, sp)); }
                _ => { diags.push(sp, "expected an object name (objects start lowercase)"); p.bump(); break; }
            }
            if !p.eat(&Tok::Comma) {
                break;
            }
            if matches!(p.peek(), Tok::RBrace) {
                break; // trailing comma
            }
        }
        p.expect(&Tok::Dash, "-", diags);
        let ty = match p.peek().clone() {
            Tok::Upper(n) => { p.bump(); n }
            _ => { diags.push(p.span(), "expected a type name"); "Object".to_string() }
        };
        for (name, sp) in group {
            ast.objects.push(ObjDecl { name, ty: ty.clone(), span: sp });
        }
        p.eat(&Tok::Comma);
    }
    p.expect(&Tok::RBrace, "}", diags);
}

fn parse_agents(p: &mut Parser, ast: &mut Ast, diags: &mut Diagnostics) {
    while !matches!(p.peek(), Tok::RBrace | Tok::Eof) {
        let sp = p.span();
        match p.peek().clone() {
            Tok::Lower(n) => { p.bump(); ast.agents.push((n, sp)); }
            _ => { diags.push(sp, "expected an agent name"); p.bump(); }
        }
        p.eat(&Tok::Comma);
    }
    p.expect(&Tok::RBrace, "}", diags);
}

/// Words a clause head can start with. Reserved as proposition names (below) so
/// that `comma_starts_new_clause` never has to guess: once none of these can be a
/// proposition, seeing one right after a comma in a `causes` list is unambiguous.
const RESERVED_CLAUSE_WORDS: [&str; 7] =
    ["actor", "pre", "causes", "determines", "announces", "observes", "aware"];

fn parse_props(p: &mut Parser, ast: &mut Ast, diags: &mut Diagnostics) {
    while !matches!(p.peek(), Tok::RBrace | Tok::Eof) {
        let sp = p.span();
        let name = match p.peek().clone() {
            Tok::Lower(n) => { p.bump(); n }
            _ => { diags.push(sp, "expected a predicate name"); p.bump(); continue; }
        };
        if RESERVED_CLAUSE_WORDS.contains(&name.as_str()) {
            diags.push(
                sp,
                format!("`{name}` is a reserved clause keyword and cannot name a proposition"),
            );
        }
        let mut params = Vec::new();
        if p.eat(&Tok::LParen) {
            while !matches!(p.peek(), Tok::RParen | Tok::Eof) {
                match p.peek().clone() {
                    Tok::Upper(t) => { p.bump(); params.push(t); }
                    _ => { diags.push(p.span(), "predicate parameters must be type names"); p.bump(); }
                }
                if !p.eat(&Tok::Comma) {
                    break;
                }
            }
            p.expect(&Tok::RParen, ")", diags);
        }
        ast.props.push(PropDecl { name, params, span: sp.merge(p.span()) });
        p.eat(&Tok::Comma);
    }
    p.expect(&Tok::RBrace, "}", diags);
}

fn parse_constants(p: &mut Parser, ast: &mut Ast, diags: &mut Diagnostics) {
    while !matches!(p.peek(), Tok::RBrace | Tok::Eof) {
        let negated = p.eat(&Tok::Bang);
        match p.parse_expr(diags) {
            Expr::Atom(term) => ast.constants.push(ConstDecl { negated, term }),
            other => diags.push(other.span(), "a constant must be a predicate application"),
        }
        p.eat(&Tok::Comma);
    }
    p.expect(&Tok::RBrace, "}", diags);
}

/// `define { name(?a, ?b) = <formula>  other = <formula> }`
///
/// Entries need no separator: an entry ends where its formula does, and the next begins
/// with a name followed by `(` or `=`. Commas are accepted for those who want them.
fn parse_defines(p: &mut Parser, ast: &mut Ast, diags: &mut Diagnostics) {
    while !matches!(p.peek(), Tok::RBrace | Tok::Eof) {
        let span = p.span();
        let name = match p.peek().clone() {
            Tok::Lower(n) => {
                p.bump();
                n
            }
            _ => {
                diags.push(span, "expected a definition name");
                p.bump();
                continue;
            }
        };
        let mut params = Vec::new();
        if p.eat(&Tok::LParen) {
            while !matches!(p.peek(), Tok::RParen | Tok::Eof) {
                match p.peek().clone() {
                    Tok::Var(v) => {
                        p.bump();
                        params.push(v);
                    }
                    _ => {
                        diags.push(p.span(), "a definition's parameters must be `?variables`");
                        p.bump();
                    }
                }
                if !p.eat(&Tok::Comma) {
                    break;
                }
            }
            p.expect(&Tok::RParen, ")", diags);
        }
        if !p.expect(&Tok::Eq, "=", diags) {
            skip_block(p);
            return;
        }
        let body = p.parse_expr(diags);
        ast.defines.push(DefDecl { name, params, body, span });
        p.eat(&Tok::Comma);
    }
    p.expect(&Tok::RBrace, "}", diags);
}

fn parse_initially(p: &mut Parser, ast: &mut Ast, diags: &mut Diagnostics, head: Span) {
    let mut items = Vec::new();
    while !matches!(p.peek(), Tok::RBrace | Tok::Eof) {
        items.push(p.parse_expr(diags));
        p.eat(&Tok::Comma);
    }
    let end = p.span();
    p.expect(&Tok::RBrace, "}", diags);
    ast.init = Some(Init::Declarative(items, head.merge(end)));
}

fn parse_state(p: &mut Parser, ast: &mut Ast, diags: &mut Diagnostics, head: Span) {
    let mut worlds = Vec::new();
    let mut edges = Vec::new();
    while !matches!(p.peek(), Tok::RBrace | Tok::Eof) {
        let sp = p.span();
        let designated = p.eat(&Tok::Star);
        let name = match p.peek().clone() {
            Tok::Lower(n) => { p.bump(); n }
            _ => { diags.push(sp, "expected a world or agent name"); p.bump(); continue; }
        };
        if p.eat(&Tok::Colon) {
            // `agent: u <cmp> v`
            let from = match p.peek().clone() {
                Tok::Lower(n) => { p.bump(); n }
                _ => { diags.push(p.span(), "expected a world name"); continue; }
            };
            let cmp = match p.peek() {
                Tok::Tilde => { p.bump(); Cmp::Equi }
                Tok::Lt => { p.bump(); Cmp::Lt }
                Tok::Le => { p.bump(); Cmp::Le }
                _ => { diags.push(p.span(), "expected `~`, `<`, or `<=`"); p.bump(); Cmp::Le }
            };
            let to = match p.peek().clone() {
                Tok::Lower(n) => { p.bump(); n }
                _ => { diags.push(p.span(), "expected a world name"); continue; }
            };
            edges.push(EdgeDecl { agent: name, from, cmp, to, span: sp.merge(p.span()) });
        } else {
            // `*u <- { facts }`
            p.expect(&Tok::Gets, "<-", diags);
            p.expect(&Tok::LBrace, "{", diags);
            let mut facts = Vec::new();
            while !matches!(p.peek(), Tok::RBrace | Tok::Eof) {
                match p.parse_expr(diags) {
                    Expr::Atom(t) => facts.push(t),
                    other => diags.push(other.span(), "a world's facts must be predicate applications"),
                }
                p.eat(&Tok::Comma);
            }
            p.expect(&Tok::RBrace, "}", diags);
            worlds.push(WorldDecl { name, designated, facts, span: sp.merge(p.span()) });
        }
        p.eat(&Tok::Comma);
    }
    let end = p.span();
    p.expect(&Tok::RBrace, "}", diags);
    ast.init = Some(Init::Explicit { worlds, edges, span: head.merge(end) });
}

fn parse_arg(p: &mut Parser, diags: &mut Diagnostics) -> Arg {
    match p.peek().clone() {
        Tok::Lower(n) => { p.bump(); Arg::Obj(n) }
        Tok::Var(v) => { p.bump(); Arg::Var(v) }
        _ => {
            diags.push(p.span(), "expected an object or `?variable`");
            p.bump();
            Arg::Obj(String::new())
        }
    }
}

fn parse_actions(p: &mut Parser, ast: &mut Ast, diags: &mut Diagnostics) {
    while !matches!(p.peek(), Tok::RBrace | Tok::Eof) {
        let sp = p.span();
        let name = match p.peek().clone() {
            Tok::Lower(n) => { p.bump(); n }
            _ => { diags.push(sp, "expected an action name"); p.bump(); continue; }
        };
        let mut params = Vec::new();
        if p.eat(&Tok::LParen) {
            while !matches!(p.peek(), Tok::RParen | Tok::Eof) {
                let psp = p.span();
                let vn = match p.peek().clone() {
                    Tok::Var(v) => { p.bump(); v }
                    _ => { diags.push(psp, "expected `?parameter`"); p.bump(); continue; }
                };
                p.expect(&Tok::Dash, "-", diags);
                let ty = match p.peek().clone() {
                    Tok::Upper(t) => { p.bump(); t }
                    _ => { diags.push(p.span(), "expected a type name"); "Object".to_string() }
                };
                params.push(ParamDecl { name: vn, ty, span: psp.merge(p.span()) });
                if !p.eat(&Tok::Comma) {
                    break;
                }
            }
            p.expect(&Tok::RParen, ")", diags);
        }
        p.expect(&Tok::LBrace, "{", diags);
        let clauses = parse_clauses(p, diags);
        ast.actions.push(ActionDecl { name, params, clauses, span: sp });
        p.eat(&Tok::Comma);
    }
    p.expect(&Tok::RBrace, "}", diags);
}

/// Whether the tokens right after the cursor's current comma start a new clause,
/// rather than continuing a `causes` literal list. This is exact, not a heuristic:
/// `parse_props` rejects [`RESERVED_CLAUSE_WORDS`] as proposition names, so a bare
/// `actor`/`pre`/`causes`/`determines`/`announces` can never legally be a `causes`
/// literal, and `observes`/`aware` can never begin a clause on their own — the
/// grammar has no rule that lets a bare `observes`/`aware` stand as a literal, so
/// `<word> observes|aware` with no comma between them is always the start of an
/// `<arg> observes|aware` clause, never `word` followed by a new clause.
///
/// Precondition: `p.peek()` is the comma being considered.
fn comma_starts_new_clause(p: &Parser) -> bool {
    debug_assert!(matches!(p.peek(), Tok::Comma), "must be called at a comma");
    match p.peek_at(1) {
        Tok::Lower(k) if RESERVED_CLAUSE_WORDS[..5].contains(&k.as_str()) => true,
        Tok::Lower(_) | Tok::Var(_) => {
            matches!(p.peek_at(2), Tok::Lower(k2) if k2 == "observes" || k2 == "aware")
        }
        _ => false,
    }
}

fn parse_clauses(p: &mut Parser, diags: &mut Diagnostics) -> Vec<Clause> {
    let mut clauses = Vec::new();
    while !matches!(p.peek(), Tok::RBrace | Tok::Eof) {
        let sp = p.span();
        // A clause is either a keyword, or `<arg> observes|aware`.
        let kw = match p.peek().clone() {
            Tok::Lower(k) => Some(k),
            _ => None,
        };
        match kw.as_deref() {
            Some("actor") => {
                p.bump();
                let a = parse_arg(p, diags);
                clauses.push(Clause::Actor(a, sp));
            }
            Some("pre") => {
                p.bump();
                clauses.push(Clause::Pre(p.parse_expr(diags)));
            }
            Some("determines") => {
                p.bump();
                clauses.push(Clause::Determines(p.parse_expr(diags)));
            }
            Some("announces") => {
                p.bump();
                clauses.push(Clause::Announces(p.parse_expr(diags)));
            }
            Some("causes") => {
                p.bump();
                let mut lits = Vec::new();
                loop {
                    let neg = p.eat(&Tok::Bang);
                    match p.parse_expr(diags) {
                        Expr::Atom(t) => lits.push((t, !neg)),
                        other => diags.push(other.span(), "`causes` takes literals"),
                    }
                    // A comma normally continues the literal list, but a zero-arity
                    // literal (e.g. `p`) is syntactically identical to a bare object
                    // name, so `causes p, a observes` would otherwise swallow `a` as
                    // a second literal. Stop instead when what follows the comma is
                    // the start of a fresh clause.
                    if matches!(p.peek(), Tok::Comma) && comma_starts_new_clause(p) {
                        break;
                    }
                    if !p.eat(&Tok::Comma) {
                        break;
                    }
                }
                let cond = if matches!(p.peek(), Tok::Lower(k) if k == "if") {
                    p.bump();
                    Some(p.parse_expr(diags))
                } else {
                    None
                };
                clauses.push(Clause::Causes { lits, cond, span: sp.merge(p.span()) });
            }
            _ => {
                // `<arg> observes` / `<arg> aware`
                let who = parse_arg(p, diags);
                let which = match p.peek().clone() {
                    Tok::Lower(k) if k == "observes" || k == "aware" => { p.bump(); k }
                    _ => {
                        diags.push(p.span(), "expected `observes` or `aware`");
                        p.bump();
                        continue;
                    }
                };
                let cond = if matches!(p.peek(), Tok::Lower(k) if k == "if") {
                    p.bump();
                    Some(p.parse_expr(diags))
                } else {
                    None
                };
                let span = sp.merge(p.span());
                clauses.push(if which == "observes" {
                    Clause::Observes { who, cond, span }
                } else {
                    Clause::Aware { who, cond, span }
                });
            }
        }
        p.eat(&Tok::Comma);
    }
    p.expect(&Tok::RBrace, "}", diags);
    clauses
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Diagnostics;

    const COIN: &str = r#"
        types   { Actor - Object }
        objects { alice, bob, carol - Actor }
        agents  { alice, bob, carol }
        props   { h, d }

        initially {
            h
            ?[carol] h
            B[carol] h
        }

        actions {
            announce_not_heads {
                actor     alice
                announces !h
                alice observes, bob observes, carol observes
            }
            peek_c {
                actor      carol
                determines h
                carol observes
                bob   aware
                alice aware if !d
            }
        }
    "#;

    fn parse(src: &str) -> Ast {
        let mut d = Diagnostics::default();
        let a = parse_file(src, &mut d);
        assert!(d.is_empty(), "unexpected errors:\n{}", d.render(src));
        a
    }

    #[test]
    fn parses_every_section_of_a_realistic_file() {
        let a = parse(COIN);
        assert_eq!(a.types.len(), 1);
        assert_eq!(a.types[0].name, "Actor", "the subtype must be `Actor`, not swapped with its parent");
        assert_eq!(a.types[0].parent, "Object", "the supertype must be `Object`, not swapped with the subtype");
        assert_eq!(a.objects.len(), 3, "one declaration per object even when comma-grouped");
        assert_eq!(a.agents.len(), 3);
        let agent_names: Vec<&str> = a.agents.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(agent_names, vec!["alice", "bob", "carol"], "agent names must be preserved, not blanked");
        assert_eq!(a.props.len(), 2);
        let prop_names: Vec<&str> = a.props.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(prop_names, vec!["h", "d"], "prop names must be preserved, not blanked");
        assert_eq!(a.actions.len(), 2);
        assert!(matches!(a.init, Some(Init::Declarative(_, _))));
    }

    #[test]
    fn grouped_object_declarations_share_a_type() {
        let a = parse(COIN);
        assert!(a.objects.iter().all(|o| o.ty == "Actor"));
        let names: Vec<&str> = a.objects.iter().map(|o| o.name.as_str()).collect();
        assert_eq!(names, vec!["alice", "bob", "carol"]);
    }

    #[test]
    fn action_clauses_keep_source_order_and_conditions() {
        let a = parse(COIN);
        let peek = a.actions.iter().find(|x| x.name == "peek_c").unwrap();
        assert!(matches!(peek.clauses[0], Clause::Actor(_, _)));
        assert!(matches!(peek.clauses[1], Clause::Determines(_)));
        // `alice aware if !d` must carry its guard.
        let guarded = peek.clauses.iter().any(
            |c| matches!(c, Clause::Aware { who: Arg::Obj(w), cond: Some(_), .. } if w == "alice"),
        );
        assert!(guarded, "the conditional `aware` clause must retain its guard");
    }

    #[test]
    fn explicit_state_form_parses_worlds_and_edges() {
        let src = r#"
            types{} objects{} agents{ carol } props{ h }
            state {
                *u <- { h }
                 v <- { }
                carol: u ~ v
                carol: v < u
                carol: u <= v
            }
            actions{}
        "#;
        let a = parse(src);
        match a.init {
            Some(Init::Explicit { worlds, edges, .. }) => {
                assert_eq!(worlds.len(), 2);
                assert!(worlds[0].designated, "`*` marks the designated world");
                assert!(!worlds[1].designated);
                assert_eq!(edges.len(), 3);
                assert_eq!(edges[0].cmp, Cmp::Equi);
                assert_eq!(edges[1].cmp, Cmp::Lt);
                assert_eq!((edges[1].from.as_str(), edges[1].to.as_str()), ("v", "u"));
                // `<=` must parse to its own variant, not collapse onto `<`.
                assert_eq!(edges[2].cmp, Cmp::Le);
                assert_eq!((edges[2].from.as_str(), edges[2].to.as_str()), ("u", "v"));
            }
            other => panic!("expected an explicit state, got {other:?}"),
        }
    }

    #[test]
    fn causes_and_pre_clauses_carry_their_literals_and_guard() {
        let src = r#"
            types{} objects{} agents{ alice } props{ h, d }
            initially{ h }
            actions {
                flip(?c - Object) {
                    actor alice
                    pre !d
                    causes h, !d if d
                }
            }
        "#;
        let a = parse(src);
        let act = &a.actions[0];
        match &act.clauses[1] {
            Clause::Pre(Expr::Not(inner, _)) => {
                assert!(matches!(**inner, Expr::Atom(ref t) if t.pred == "d"), "`pre !d` must negate `d`, not something else");
            }
            other => panic!("expected Pre(!d), got {other:?}"),
        }
        match &act.clauses[2] {
            Clause::Causes { lits, cond, .. } => {
                assert_eq!(lits.len(), 2, "both `causes` literals must be kept");
                assert_eq!(lits[0].0.pred, "h");
                assert!(lits[0].1, "`h` (unmarked) must be positive");
                assert_eq!(lits[1].0.pred, "d");
                assert!(!lits[1].1, "`!d` must be negative");
                match cond {
                    Some(Expr::Atom(t)) => assert_eq!(t.pred, "d", "the `if` guard must be kept, not dropped"),
                    other => panic!("expected the `if d` guard, got {other:?}"),
                }
            }
            other => panic!("expected Causes, got {other:?}"),
        }
    }

    #[test]
    fn constants_section_parses_negation_and_term() {
        let src = r#"
            types{} objects{} agents{} props{ h, d }
            constants { h, !d }
            initially{}
            actions{}
        "#;
        let a = parse(src);
        assert_eq!(a.constants.len(), 2);
        assert!(!a.constants[0].negated, "`h` (unmarked) must not be negated");
        assert_eq!(a.constants[0].term.pred, "h");
        assert!(a.constants[1].negated, "`!d` must be negated");
        assert_eq!(a.constants[1].term.pred, "d", "the negated entry's term must still be `d`, not dropped");
    }

    #[test]
    fn action_parameters_parse_variable_name_and_type() {
        let src = r#"
            types{} objects{} agents{} props{ h }
            initially{}
            actions {
                flip(?c - Object, ?x - Actor) {
                    actor alice
                }
            }
        "#;
        let a = parse(src);
        let act = &a.actions[0];
        assert_eq!(act.params.len(), 2, "both parameters must be kept");
        assert_eq!(act.params[0].name, "c", "parameter name must not be swapped with its type");
        assert_eq!(act.params[0].ty, "Object", "parameter type must not be swapped with its name");
        assert_eq!(act.params[1].name, "x");
        assert_eq!(act.params[1].ty, "Actor");
    }

    #[test]
    fn both_initial_state_forms_at_once_is_an_error() {
        let src = "types{} objects{} agents{} props{} initially{} state{} actions{}";
        let mut d = Diagnostics::default();
        let _ = parse_file(src, &mut d);
        assert!(d.items().iter().any(|x| x.message.contains("initially")
            || x.message.contains("state")), "should complain about the duplicate form");
    }

    #[test]
    fn a_duplicated_section_is_an_error() {
        let src = "types{} types{} objects{} agents{} props{} initially{} actions{}";
        let mut d = Diagnostics::default();
        let _ = parse_file(src, &mut d);
        assert!(d.items().iter().any(|x| x.message.contains("duplicate")));
    }

    #[test]
    fn a_missing_required_section_is_reported_once_at_the_end() {
        // `actions` omitted.
        let src = "types{} objects{} agents{} props{} initially{}";
        let mut d = Diagnostics::default();
        let _ = parse_file(src, &mut d);
        assert!(d.items().iter().any(|x| x.message.contains("actions")));
    }

    // --- `comma_starts_new_clause` boundary tests -------------------------------
    //
    // These pin the comma boundary inside a `causes` literal list directly, rather
    // than relying on the grounding fixtures in `lower_action.rs` to exercise it
    // indirectly. Each keyword and both lookahead words get their own case.

    #[test]
    fn a_causes_list_continues_across_a_comma_before_another_bare_literal() {
        let a = parse(r#"
            types{} objects{} agents{} props{ h, d }
            initially{}
            actions { go() { causes h, d } }
        "#);
        match &a.actions[0].clauses[0] {
            Clause::Causes { lits, .. } => {
                assert_eq!(lits.len(), 2, "a comma before another bare literal must \
                    still extend the causes list, not stop at the first");
                assert_eq!(lits[0].0.pred, "h");
                assert_eq!(lits[1].0.pred, "d");
            }
            other => panic!("expected Causes, got {other:?}"),
        }
    }

    #[test]
    fn a_comma_before_actor_ends_the_causes_list() {
        let a = parse(r#"
            types{ Actor - Object } objects{ x - Actor } agents{} props{ p }
            initially{}
            actions { go() { causes p, actor x } }
        "#);
        assert_eq!(a.actions[0].clauses.len(), 2, "`actor` must not be swallowed as a second literal");
        match &a.actions[0].clauses[0] {
            Clause::Causes { lits, .. } => assert_eq!(lits.len(), 1),
            other => panic!("expected Causes, got {other:?}"),
        }
        assert!(matches!(&a.actions[0].clauses[1], Clause::Actor(Arg::Obj(n), _) if n == "x"));
    }

    #[test]
    fn a_comma_before_pre_ends_the_causes_list() {
        let a = parse(r#"
            types{} objects{} agents{} props{ p, q }
            initially{}
            actions { go() { causes p, pre q } }
        "#);
        assert_eq!(a.actions[0].clauses.len(), 2, "`pre` must not be swallowed as a second literal");
        match &a.actions[0].clauses[0] {
            Clause::Causes { lits, .. } => assert_eq!(lits.len(), 1),
            other => panic!("expected Causes, got {other:?}"),
        }
        assert!(matches!(&a.actions[0].clauses[1], Clause::Pre(_)));
    }

    #[test]
    fn a_comma_before_a_second_causes_ends_the_first_list() {
        let a = parse(r#"
            types{} objects{} agents{} props{ p, q }
            initially{}
            actions { go() { causes p, causes q } }
        "#);
        assert_eq!(a.actions[0].clauses.len(), 2, "the second `causes` must start its own clause");
        match &a.actions[0].clauses[0] {
            Clause::Causes { lits, .. } => assert_eq!(lits.len(), 1),
            other => panic!("expected the first Causes, got {other:?}"),
        }
        match &a.actions[0].clauses[1] {
            Clause::Causes { lits, .. } => assert_eq!(lits[0].0.pred, "q"),
            other => panic!("expected the second Causes, got {other:?}"),
        }
    }

    #[test]
    fn a_comma_before_determines_ends_the_causes_list() {
        let a = parse(r#"
            types{} objects{} agents{} props{ p, q }
            initially{}
            actions { go() { causes p, determines q } }
        "#);
        assert_eq!(a.actions[0].clauses.len(), 2, "`determines` must not be swallowed as a second literal");
        match &a.actions[0].clauses[0] {
            Clause::Causes { lits, .. } => assert_eq!(lits.len(), 1),
            other => panic!("expected Causes, got {other:?}"),
        }
        assert!(matches!(&a.actions[0].clauses[1], Clause::Determines(_)));
    }

    #[test]
    fn a_comma_before_announces_ends_the_causes_list() {
        let a = parse(r#"
            types{} objects{} agents{} props{ p, q }
            initially{}
            actions { go() { causes p, announces q } }
        "#);
        assert_eq!(a.actions[0].clauses.len(), 2, "`announces` must not be swallowed as a second literal");
        match &a.actions[0].clauses[0] {
            Clause::Causes { lits, .. } => assert_eq!(lits.len(), 1),
            other => panic!("expected Causes, got {other:?}"),
        }
        assert!(matches!(&a.actions[0].clauses[1], Clause::Announces(_)));
    }

    #[test]
    fn a_comma_before_an_observes_head_ends_the_causes_list() {
        let a = parse(r#"
            types{ Actor - Object } objects{ x - Actor } agents{ x } props{ p }
            initially{}
            actions { go() { causes p, x observes } }
        "#);
        assert_eq!(a.actions[0].clauses.len(), 2, "the `observes` head must not be swallowed as a second literal");
        match &a.actions[0].clauses[0] {
            Clause::Causes { lits, .. } => assert_eq!(lits.len(), 1),
            other => panic!("expected Causes, got {other:?}"),
        }
        assert!(matches!(
            &a.actions[0].clauses[1],
            Clause::Observes { who: Arg::Obj(n), .. } if n == "x"
        ));
    }

    #[test]
    fn a_comma_before_an_aware_head_ends_the_causes_list() {
        let a = parse(r#"
            types{ Actor - Object } objects{ x - Actor } agents{ x } props{ p }
            initially{}
            actions { go() { causes p, x aware } }
        "#);
        assert_eq!(a.actions[0].clauses.len(), 2, "the `aware` head must not be swallowed as a second literal");
        match &a.actions[0].clauses[0] {
            Clause::Causes { lits, .. } => assert_eq!(lits.len(), 1),
            other => panic!("expected Causes, got {other:?}"),
        }
        assert!(matches!(
            &a.actions[0].clauses[1],
            Clause::Aware { who: Arg::Obj(n), .. } if n == "x"
        ));
    }

    #[test]
    fn a_reserved_clause_word_cannot_name_a_proposition() {
        let mut d = Diagnostics::default();
        let src = "types{} objects{} agents{} props{ pre } initially{} actions{}";
        let _ = parse_file(src, &mut d);
        assert!(
            d.items().iter().any(|x| x.message.contains("pre") && x.message.contains("reserved")),
            "a proposition named after a clause keyword must be rejected"
        );
    }
}
