//! Lowering surface formulas to `delhi-syntax` ids, desugaring §7.4 on the way.

use crate::ast::{Arg, Expr, Modal, Term};
use crate::{Constants, Diagnostics, Sig, Span};
use delhi_syntax::{AgentId, FormulaId, Store};

/// Variable-to-object bindings in scope while lowering.
#[derive(Default, Clone, Debug)]
pub struct Bindings(Vec<(String, String)>);

impl Bindings {
    /// The object bound to `var`, if any.
    pub fn get(&self, var: &str) -> Option<&str> {
        self.0.iter().rev().find(|(v, _)| v == var).map(|(_, o)| o.as_str())
    }
    /// Returns a copy extended with one more binding. Later entries shadow earlier.
    pub fn with(&self, var: &str, obj: &str) -> Bindings {
        let mut v = self.0.clone();
        v.push((var.to_string(), obj.to_string()));
        Bindings(v)
    }
}

impl From<Vec<(String, String)>> for Bindings {
    fn from(v: Vec<(String, String)>) -> Self {
        Bindings(v)
    }
}

/// Simplifying constructors. Folding a constant to `⊥` is pointless unless `φ & ⊥`
/// collapses to `⊥` — §7.1's scale argument is that impossible actions are *never
/// generated*, and Task 8 detects that by comparing the lowered precondition against
/// `⊥`. `Store` deliberately does not simplify, so it happens here.
fn mk_not(store: &mut Store, a: FormulaId) -> FormulaId {
    let (t, f) = (store.tru(), store.fls());
    if a == t {
        f
    } else if a == f {
        t
    } else {
        store.not(a)
    }
}

fn mk_and(store: &mut Store, a: FormulaId, b: FormulaId) -> FormulaId {
    let (t, f) = (store.tru(), store.fls());
    if a == f || b == f {
        f
    } else if a == t {
        b
    } else if b == t {
        a
    } else {
        store.and(a, b)
    }
}

fn mk_or(store: &mut Store, a: FormulaId, b: FormulaId) -> FormulaId {
    let (t, f) = (store.tru(), store.fls());
    if a == t || b == t {
        t
    } else if a == f {
        b
    } else if b == f {
        a
    } else {
        store.or(a, b)
    }
}

fn mk_implies(store: &mut Store, a: FormulaId, b: FormulaId) -> FormulaId {
    let na = mk_not(store, a);
    mk_or(store, na, b)
}

/// Resolves a term's arguments to concrete object names.
fn resolve_args(
    term: &Term,
    binds: &Bindings,
    diags: &mut Diagnostics,
) -> Option<Vec<String>> {
    let mut out = Vec::with_capacity(term.args.len());
    for a in &term.args {
        match a {
            Arg::Obj(o) => out.push(o.clone()),
            Arg::Var(v) => match binds.get(v) {
                Some(o) => out.push(o.to_string()),
                None => {
                    diags.push(term.span, format!("`?{v}` is not bound here"));
                    return None;
                }
            },
            Arg::Ty(t) => {
                diags.push(
                    term.span,
                    format!("type name `{t}` is only allowed inside `constants`"),
                );
                return None;
            }
        }
    }
    Some(out)
}

/// Resolves an agent list to ids, reporting any that were never declared.
fn resolve_agents(
    names: &[String],
    sig: &Sig,
    span: Span,
    diags: &mut Diagnostics,
) -> Vec<AgentId> {
    let mut out = Vec::with_capacity(names.len());
    for n in names {
        match sig.agent_id(n) {
            Some(i) => out.push(i),
            None => diags.push(span, format!("`{n}` is not a declared agent")),
        }
    }
    out
}

/// Lowers a surface formula, folding constants and desugaring §7.4.
///
/// Always returns an id; on error it returns `⊥` and records a diagnostic, so one run
/// reports every problem rather than stopping at the first.
pub fn lower_formula(
    e: &Expr,
    sig: &Sig,
    consts: &Constants,
    binds: &Bindings,
    store: &mut Store,
    diags: &mut Diagnostics,
) -> FormulaId {
    match e {
        Expr::True(_) => store.tru(),
        Expr::False(_) => store.fls(),
        Expr::Not(inner, _) => {
            let f = lower_formula(inner, sig, consts, binds, store, diags);
            mk_not(store, f)
        }
        Expr::And(a, b, _) => {
            let x = lower_formula(a, sig, consts, binds, store, diags);
            let y = lower_formula(b, sig, consts, binds, store, diags);
            mk_and(store, x, y)
        }
        Expr::Or(a, b, _) => {
            let x = lower_formula(a, sig, consts, binds, store, diags);
            let y = lower_formula(b, sig, consts, binds, store, diags);
            mk_or(store, x, y)
        }
        Expr::Implies(a, b, _) => {
            let x = lower_formula(a, sig, consts, binds, store, diags);
            let y = lower_formula(b, sig, consts, binds, store, diags);
            mk_implies(store, x, y)
        }
        Expr::Atom(term) => {
            let Some(args) = resolve_args(term, binds, diags) else {
                return store.fls();
            };
            // Constants are folded away here and never become atoms (§7.1).
            //
            // `is_constant_pred` is the gate — it, not `lookup`, decides whether this
            // predicate is compile-time-constant at all. `lookup(..).unwrap_or(false)`
            // is the default within that gate: a well-formed, correct-arity instance of
            // a declared constant predicate that was simply never mentioned means
            // "not declared", which folds to `false` silently, not an error. Trusting
            // `lookup` alone (dropping the gate) would wrongly fold ordinary
            // propositions; reporting the `None` case as a diagnostic (rather than
            // defaulting it) would defeat the fold for the exact case it exists to
            // handle.
            if consts.is_constant_pred(&term.pred) {
                return if consts.lookup(&term.pred, &args).unwrap_or(false) {
                    store.tru()
                } else {
                    store.fls()
                };
            }
            match sig.atom_id(&term.pred, &args) {
                Some(id) => store.atom(id),
                None => {
                    diags.push(
                        term.span,
                        format!(
                            "no proposition `{}`; check the name, arity, and argument types",
                            crate::ground::atom_key(&term.pred, &args)
                        ),
                    );
                    store.fls()
                }
            }
        }
        Expr::Modality { op, agents, cond, body, span } => {
            let inner = lower_formula(body, sig, consts, binds, store, diags);

            if matches!(op, Modal::Common) {
                // Common knowledge takes a GROUP mask. It must not distribute.
                let mask: u32 = match agents {
                    None => {
                        let n = sig.n_agents();
                        if n >= 32 { u32::MAX } else { (1u32 << n) - 1 }
                    }
                    Some(names) => resolve_agents(names, sig, *span, diags)
                        .into_iter()
                        .fold(0u32, |m, i| m | (1u32 << i)),
                };
                return store.common(mask, inner);
            }

            let Some(names) = agents else {
                diags.push(*span, "only `C` accepts `[*]`; name the agents explicitly");
                return store.fls();
            };
            let ids = resolve_agents(names, sig, *span, diags);
            if ids.is_empty() {
                return store.fls();
            }

            // Everything else distributes over the agent list (§7.4).
            let mut parts = Vec::with_capacity(ids.len());
            for i in ids {
                let f = match op {
                    Modal::Knows => store.knows(i, inner),
                    Modal::Believes => match cond {
                        Some(psi) => {
                            let c = lower_formula(psi, sig, consts, binds, store, diags);
                            store.cond_bel(i, c, inner)
                        }
                        None => store.believes(i, inner),
                    },
                    Modal::Safe => store.safe(i, inner),
                    Modal::KnowsDual => store.considers_possible(i, inner),
                    Modal::BelievesDual => store.not_ruled_out(i, inner),
                    Modal::SafeDual => store.safe_dual(i, inner),
                    Modal::KnowsWhether => store.knows_whether(i, inner),
                    Modal::BelievesWhether => store.believes_whether(i, inner),
                    Modal::Ignorant => store.ignorant(i, inner),
                    Modal::Undecided => store.undecided(i, inner),
                    Modal::Common => unreachable!("handled above"),
                };
                parts.push(f);
            }
            store.all(&parts)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{parse_file, Constants, Diagnostics, Parser, Sig};
    use delhi_syntax::Store;

    const HEADER: &str = r#"
        types   { Location - Object }
        objects { alice, bob - Location }
        agents  { alice, bob }
        props   { p, q }
        constants { !adjacent(Location, Location), adjacent(alice, bob) }
        initially { }
        actions {}
    "#;

    fn setup() -> (Sig, Constants) {
        let mut d = Diagnostics::default();
        let ast = parse_file(HEADER, &mut d);
        let sig = Sig::build(&ast, &mut d);
        let c = Constants::build(&ast, &sig, &mut d);
        assert!(d.is_empty(), "setup errors:\n{}", d.render(HEADER));
        (sig, c)
    }

    fn lower(src: &str, s: &mut Store) -> FormulaId {
        let (sig, c) = setup();
        let mut d = Diagnostics::default();
        let toks = crate::lex(src, &mut d);
        let mut p = Parser::new(&toks);
        let e = p.parse_expr(&mut d);
        let f = lower_formula(&e, &sig, &c, &Bindings::default(), s, &mut d);
        assert!(d.is_empty(), "lowering errors for `{src}`:\n{}", d.render(src));
        f
    }

    #[test]
    fn sugar_lowers_to_the_same_id_as_its_expansion() {
        let mut s = Store::default();
        let (sig, _) = setup();
        let a = sig.agent_id("alice").unwrap();
        let p_atom = sig.atom_id("p", &[]).unwrap();

        let got = lower("Kw[alice] p", &mut s);
        let want = { let x = s.atom(p_atom); s.knows_whether(a, x) };
        assert_eq!(got, want, "Kw must lower through knows_whether");

        let got = lower("?[alice] p", &mut s);
        let want = { let x = s.atom(p_atom); s.ignorant(a, x) };
        assert_eq!(got, want);

        let got = lower("S'[alice] p", &mut s);
        let want = { let x = s.atom(p_atom); s.safe_dual(a, x) };
        assert_eq!(got, want);
    }

    #[test]
    fn agent_lists_distribute_over_knowledge() {
        let mut s = Store::default();
        let (sig, _) = setup();
        let (a, b) = (sig.agent_id("alice").unwrap(), sig.agent_id("bob").unwrap());
        let p_atom = sig.atom_id("p", &[]).unwrap();
        let got = lower("K[alice, bob] p", &mut s);
        let want = { let x = s.atom(p_atom); s.knows_all(&[a, b], x) };
        assert_eq!(got, want);
    }

    #[test]
    fn agent_lists_do_not_distribute_over_common_knowledge() {
        // C[a,b] is common knowledge among the GROUP — strictly stronger than
        // C[a] & C[b]. This is the one place distribution would be wrong.
        let mut s = Store::default();
        let (sig, _) = setup();
        let (a, b) = (sig.agent_id("alice").unwrap(), sig.agent_id("bob").unwrap());
        let p_atom = sig.atom_id("p", &[]).unwrap();
        let got = lower("C[alice, bob] p", &mut s);
        let mask = (1u32 << a) | (1u32 << b);
        let want = { let x = s.atom(p_atom); s.common(mask, x) };
        assert_eq!(got, want);

        let wrong = { let x = s.atom(p_atom); let ca = s.common(1 << a, x);
                      let cb = s.common(1 << b, x); s.and(ca, cb) };
        assert_ne!(got, wrong, "C must NOT distribute over its agent list");
    }

    #[test]
    fn c_star_covers_every_declared_agent() {
        let mut s = Store::default();
        let (sig, _) = setup();
        let p_atom = sig.atom_id("p", &[]).unwrap();
        let got = lower("C[*] p", &mut s);
        let want = { let x = s.atom(p_atom); s.common(0b11, x) };
        assert_eq!(got, want);
    }

    #[test]
    fn constants_fold_to_top_and_bottom() {
        let mut s = Store::default();
        let got_true = lower("adjacent(alice, bob)", &mut s);
        assert_eq!(got_true, s.tru());
        let got_false = lower("adjacent(bob, alice)", &mut s);
        assert_eq!(got_false, s.fls());
    }

    #[test]
    fn a_declared_but_unlisted_constant_instance_folds_silently_to_false() {
        // `lookup` returning `None` here means "not declared", not "unknown": a
        // well-formed, correct-arity instance of a declared constant predicate that
        // was simply never mentioned. It must fold to `false` exactly like an
        // explicit `Some(false)` would, with no diagnostic and no atom — trusting
        // `lookup` alone (without a value) would be a diagnostic-worthy error, but
        // the composition here is `is_constant_pred(p) && lookup(..).unwrap_or(false)`.
        let src = r#"
            types   { Location - Object }
            objects { hall, study, kitchen - Location }
            agents  { }
            props   { }
            constants { adjacent(hall, study) }
            initially { }
            actions {}
        "#;
        let mut setup_d = Diagnostics::default();
        let ast = parse_file(src, &mut setup_d);
        let sig = Sig::build(&ast, &mut setup_d);
        let c = Constants::build(&ast, &sig, &mut setup_d);
        assert!(setup_d.is_empty(), "setup errors:\n{}", setup_d.render(src));
        assert!(c.is_constant_pred("adjacent"));
        assert_eq!(c.lookup("adjacent", &["hall".to_string(), "kitchen".to_string()]), None);

        let mut s = Store::default();
        let mut d = Diagnostics::default();
        let expr_src = "adjacent(hall, kitchen)";
        let toks = crate::lex(expr_src, &mut d);
        let mut p = Parser::new(&toks);
        let e = p.parse_expr(&mut d);
        let f = lower_formula(&e, &sig, &c, &Bindings::default(), &mut s, &mut d);
        assert!(
            d.is_empty(),
            "a declared-but-unlisted instance must fold silently, not report:\n{}",
            d.render(expr_src)
        );
        assert_eq!(f, s.fls(), "unlisted instance of a declared constant folds to false");
    }

    #[test]
    fn folding_propagates_so_an_impossible_precondition_collapses_to_bottom() {
        // Without this, `p & adjacent(bob,alice)` would lower to `And(p, ⊥)` and Task 8
        // could not tell an impossible action from a merely-unsatisfied one. §7.1's
        // scale claim depends on this collapsing.
        let mut s = Store::default();
        assert_eq!(lower("p & adjacent(bob, alice)", &mut s), s.fls());
        assert_eq!(lower("adjacent(bob, alice) & p", &mut s), s.fls());
        // and the dual: a true constant disappears rather than lingering as `⊤ & p`
        let p_only = { let (sig, _) = setup(); s.atom(sig.atom_id("p", &[]).unwrap()) };
        assert_eq!(lower("p & adjacent(alice, bob)", &mut s), p_only);
        assert_eq!(lower("p | adjacent(bob, alice)", &mut s), p_only);
        assert_eq!(lower("p | adjacent(alice, bob)", &mut s), s.tru());
        assert_eq!(lower("!adjacent(alice, bob)", &mut s), s.fls());
        // The dual of negation: `!` over a folded-false constant must collapse all
        // the way to the canonical `⊤` id, not merely to some node that happens to
        // be logically equivalent. `Store::not` alone would build `Not(Not(True))`
        // here — a distinct, deeper id from `Store::tru()` — so this only passes if
        // `mk_not` actually simplifies rather than always delegating to `store.not`.
        assert_eq!(lower("!adjacent(bob, alice)", &mut s), s.tru());
    }

    #[test]
    fn variables_resolve_through_the_bindings() {
        let (sig, c) = setup();
        let mut s = Store::default();
        let mut d = Diagnostics::default();
        let src = "adjacent(?x, bob)";
        let toks = crate::lex(src, &mut d);
        let mut p = Parser::new(&toks);
        let e = p.parse_expr(&mut d);
        let b = Bindings::from(vec![("x".to_string(), "alice".to_string())]);
        let f = lower_formula(&e, &sig, &c, &b, &mut s, &mut d);
        assert!(d.is_empty(), "{}", d.render(src));
        assert_eq!(f, s.tru(), "?x bound to alice makes adjacent(alice,bob) true");
    }

    #[test]
    fn an_unbound_variable_is_reported() {
        let (sig, c) = setup();
        let mut s = Store::default();
        let mut d = Diagnostics::default();
        let toks = crate::lex("p(?nope)", &mut d);
        let mut p = Parser::new(&toks);
        let e = p.parse_expr(&mut d);
        let _ = lower_formula(&e, &sig, &c, &Bindings::default(), &mut s, &mut d);
        // `p` is zero-arity, so silently substituting `"nope"` as an object name for
        // the unbound `?nope` would still trip the "no such proposition" arity
        // error, whose message also happens to contain the bare word `nope`.
        // Checking for `"nope"` alone would pass for that wrong reason; requiring
        // `"?nope"` (with the sigil, as only the unbound-variable message writes it)
        // pins this to the rejection this test claims to protect.
        assert!(d.items().iter().any(|x| x.message.contains("?nope")));
    }

    #[test]
    fn a_type_name_outside_constants_is_reported() {
        let (sig, c) = setup();
        let mut s = Store::default();
        let mut d = Diagnostics::default();
        let toks = crate::lex("p(Location)", &mut d);
        let mut p = Parser::new(&toks);
        let e = p.parse_expr(&mut d);
        let _ = lower_formula(&e, &sig, &c, &Bindings::default(), &mut s, &mut d);
        // Both `"Location"` and `"constants"` must appear in the SAME message: `p`
        // is zero-arity, so silently accepting `Location` as an object name would
        // still trip the "no such proposition" arity error, whose rendered message
        // also happens to mention `Location` (it echoes the argument list). Checking
        // for `"Location"` alone would pass for that wrong reason; the mention of
        // `constants` pins this down to the actual type-name-outside-constants
        // rejection this test claims to protect.
        assert!(d.items().iter().any(|x| x.message.contains("Location")
            && x.message.contains("constants")));
    }

    #[test]
    fn an_undeclared_agent_is_reported() {
        let (sig, c) = setup();
        let mut s = Store::default();
        let mut d = Diagnostics::default();
        let toks = crate::lex("K[nobody] p", &mut d);
        let mut p = Parser::new(&toks);
        let e = p.parse_expr(&mut d);
        let _ = lower_formula(&e, &sig, &c, &Bindings::default(), &mut s, &mut d);
        assert!(d.items().iter().any(|x| x.message.contains("nobody")));
    }
}
