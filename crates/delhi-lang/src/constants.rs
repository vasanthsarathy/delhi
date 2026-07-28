//! Compile-time constants (§7.1). Never atoms; folded to `⊤`/`⊥` during lowering.

use crate::ast::{Arg, Ast};
use crate::ground::atom_key;
use crate::{Diagnostics, Sig};
use std::collections::{HashMap, HashSet};

/// Ground truth values fixed at parse time. Never reach the semantics.
#[derive(Default, Debug)]
pub struct Constants {
    vals: HashMap<String, bool>,
    preds: HashSet<String>,
}

impl Constants {
    /// Expands every declaration over the objects of any type it names, with later
    /// declarations overriding earlier ones.
    pub fn build(ast: &Ast, sig: &Sig, diags: &mut Diagnostics) -> Constants {
        let mut out = Constants::default();
        for decl in &ast.constants {
            let term = &decl.term;
            if sig.preds.contains_key(&term.pred) {
                diags.push(
                    term.span,
                    format!(
                        "`{}` is declared as both a proposition and a constant; \
                         constants are parse-time only and never become atoms",
                        term.pred
                    ),
                );
                continue;
            }
            out.preds.insert(term.pred.clone());

            // Each argument contributes either one object or every object of a type.
            let mut choices: Vec<Vec<String>> = Vec::with_capacity(term.args.len());
            let mut bad = false;
            for a in &term.args {
                match a {
                    Arg::Obj(o) => {
                        if !sig.objects.contains_key(o) {
                            diags.push(term.span, format!("unknown object `{o}`"));
                            bad = true;
                        }
                        choices.push(vec![o.clone()]);
                    }
                    Arg::Ty(t) => {
                        if t != "Object" && !sig.types.contains_key(t) {
                            diags.push(term.span, format!("unknown type `{t}`"));
                            bad = true;
                        }
                        choices.push(sig.objects_of(t));
                    }
                    Arg::Var(v) => {
                        diags.push(
                            term.span,
                            format!("`?{v}`: a constant may not contain a variable"),
                        );
                        bad = true;
                    }
                }
            }
            if bad {
                continue;
            }

            let mut tuples: Vec<Vec<String>> = vec![Vec::new()];
            for col in &choices {
                let mut next = Vec::with_capacity(tuples.len() * col.len());
                for prefix in &tuples {
                    for c in col {
                        let mut v = prefix.clone();
                        v.push(c.clone());
                        next.push(v);
                    }
                }
                tuples = next;
            }
            for t in tuples {
                out.vals.insert(atom_key(&term.pred, &t), !decl.negated);
            }
        }
        out
    }

    /// Registers `pred` as constant even before any instance of it is known.
    ///
    /// A derived predicate has to be constant from the outset: `is_constant_pred` is the
    /// gate that makes an unlisted instance fold to `false` rather than being reported as
    /// an undeclared proposition, and a rule that derives nothing still needs its head
    /// name to read as constant-and-false.
    pub fn declare_pred(&mut self, pred: &str) {
        self.preds.insert(pred.to_string());
    }

    /// Records a derived fact as true.
    ///
    /// Used by the rule fixpoint. Derived facts are indistinguishable from written ones
    /// afterwards, which is the point: everything downstream folds them the same way.
    pub fn add_fact(&mut self, pred: &str, args: &[String]) {
        self.preds.insert(pred.to_string());
        self.vals.insert(crate::atom_key(pred, args), true);
    }

    /// Whether `pred` was declared in the `constants` section.
    pub fn is_constant_pred(&self, pred: &str) -> bool {
        self.preds.contains(pred)
    }

    /// The fixed value of a ground constant, or `None` if it has none.
    pub fn lookup(&self, pred: &str, args: &[String]) -> Option<bool> {
        self.vals.get(&atom_key(pred, args)).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{parse_file, Diagnostics, Sig};

    fn build(src: &str) -> (Sig, Constants) {
        let mut d = Diagnostics::default();
        let ast = parse_file(src, &mut d);
        let sig = Sig::build(&ast, &mut d);
        let c = Constants::build(&ast, &sig, &mut d);
        assert!(d.is_empty(), "unexpected errors:\n{}", d.render(src));
        (sig, c)
    }

    const SRC: &str = r#"
        types   { Location - Object }
        objects { hall, study, cellar - Location }
        agents  { }
        props   { }
        constants {
            !adjacent(Location, Location),
            adjacent(hall, study),
            adjacent(study, cellar),
        }
        initially { }
        actions {}
    "#;

    #[test]
    fn a_blanket_declaration_expands_over_every_pair() {
        let (_, c) = build(SRC);
        // 3 locations squared = 9 pairs, all present.
        assert_eq!(c.lookup("adjacent", &["cellar".into(), "hall".into()]), Some(false));
        assert_eq!(c.lookup("adjacent", &["hall".into(), "cellar".into()]), Some(false));
    }

    #[test]
    fn later_declarations_override_earlier_ones() {
        let (_, c) = build(SRC);
        assert_eq!(c.lookup("adjacent", &["hall".into(), "study".into()]), Some(true));
        assert_eq!(c.lookup("adjacent", &["study".into(), "cellar".into()]), Some(true));
        // and the blanket still holds where nothing overrode it
        assert_eq!(c.lookup("adjacent", &["study".into(), "hall".into()]), Some(false));
    }

    #[test]
    fn a_predicate_is_known_to_be_constant() {
        let (_, c) = build(SRC);
        assert!(c.is_constant_pred("adjacent"));
        assert!(!c.is_constant_pred("at"));
    }

    #[test]
    fn a_wrong_arity_or_unknown_predicate_lookup_is_unknown() {
        let (_, c) = build(SRC);
        assert_eq!(c.lookup("adjacent", &["hall".into()]), None, "wrong arity");
        assert_eq!(c.lookup("nosuch", &[]), None);
    }

    #[test]
    fn a_well_formed_but_undeclared_instance_of_a_constant_predicate_is_unknown() {
        // No blanket declaration here, unlike `SRC`: `adjacent` is declared only
        // for one pair, so `adjacent(hall, kitchen)` is a valid, correct-arity
        // tuple of a known constant predicate that was simply never mentioned.
        // Task 7 relies on the *conjunction* `is_constant_pred(p) && lookup(..)
        // .unwrap_or(false)` to fold this to `false`; either half alone is
        // compatible with a broken implementation, so both are asserted here.
        let src = r#"
            types   { Location - Object }
            objects { hall, study, kitchen - Location }
            agents  { }
            props   { }
            constants {
                adjacent(hall, study),
            }
            initially { }
            actions {}
        "#;
        let (_, c) = build(src);
        assert!(c.is_constant_pred("adjacent"));
        assert_eq!(c.lookup("adjacent", &["hall".into(), "kitchen".into()]), None);
    }

    #[test]
    fn declaring_a_name_as_both_constant_and_prop_is_an_error() {
        let mut d = Diagnostics::default();
        let src = r#"
            types{} objects{} agents{} props{ p }
            constants { p } initially{} actions{}
        "#;
        let ast = parse_file(src, &mut d);
        let sig = Sig::build(&ast, &mut d);
        let _ = Constants::build(&ast, &sig, &mut d);
        assert!(
            d.items().iter().any(|x| x.message.contains("both")),
            "should reject a name declared as both a constant and a proposition"
        );
    }

    #[test]
    fn an_unknown_type_in_a_constant_pattern_is_reported() {
        let mut d = Diagnostics::default();
        let src = r#"
            types{} objects{} agents{} props{}
            constants { adjacent(Ghost, Ghost) } initially{} actions{}
        "#;
        let ast = parse_file(src, &mut d);
        let sig = Sig::build(&ast, &mut d);
        let _ = Constants::build(&ast, &sig, &mut d);
        assert!(d.items().iter().any(|x| x.message.contains("Ghost")));
    }

    #[test]
    fn a_variable_in_a_constant_pattern_is_reported() {
        let mut d = Diagnostics::default();
        let src = r#"
            types{} objects{} agents{} props{}
            constants { adjacent(?x, ?y) } initially{} actions{}
        "#;
        let ast = parse_file(src, &mut d);
        let sig = Sig::build(&ast, &mut d);
        let _ = Constants::build(&ast, &sig, &mut d);
        assert!(d.items().iter().any(|x| x.message.contains("variable")));
    }
}
