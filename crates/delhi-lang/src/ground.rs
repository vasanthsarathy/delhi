//! The signature: type hierarchy, objects, agents, and the ground atom table (§7.1).

use crate::ast::Ast;
use crate::Diagnostics;
use delhi_syntax::{AgentId, AtomId, Interner};
use std::collections::HashMap;

/// The declared signature, after checking: types, objects, agents, and every ground atom.
#[derive(Debug)]
pub struct Sig {
    /// Child type to its immediate parent. `Object` is implicit and absent.
    pub types: HashMap<String, String>,
    /// Object name to its declared type.
    pub objects: HashMap<String, String>,
    /// Agent names, in declaration order, giving dense [`AgentId`]s.
    pub agents: Interner,
    /// Canonical ground atom names, giving dense [`AtomId`]s.
    pub atoms: Interner,
    /// Predicate name to its parameter types.
    pub preds: HashMap<String, Vec<String>>,
}

/// The canonical printed form of a ground atom: `p` or `p(a,b)`.
pub fn atom_key(pred: &str, args: &[String]) -> String {
    if args.is_empty() {
        pred.to_string()
    } else {
        format!("{pred}({})", args.join(","))
    }
}

impl Sig {
    /// Builds and checks the signature, recording every problem in `diags`.
    pub fn build(ast: &Ast, diags: &mut Diagnostics) -> Sig {
        let mut types = HashMap::new();
        for t in &ast.types {
            if types.insert(t.name.clone(), t.parent.clone()).is_some() {
                diags.push(t.span, format!("duplicate type `{}`", t.name));
            }
        }
        // Every named parent must be declared, or be `Object`.
        for t in &ast.types {
            if t.parent != "Object" && !types.contains_key(&t.parent) {
                diags.push(t.span, format!("unknown supertype `{}`", t.parent));
            }
        }
        // A cycle would make `is_subtype` loop; detect by bounded walk.
        for t in ast.types.iter() {
            let mut seen = 0usize;
            let mut cur = t.name.clone();
            while let Some(p) = types.get(&cur) {
                seen += 1;
                if seen > types.len() {
                    diags.push(t.span, format!("cyclic type hierarchy at `{}`", t.name));
                    break;
                }
                cur = p.clone();
            }
        }

        let mut objects = HashMap::new();
        for o in &ast.objects {
            if o.ty != "Object" && !types.contains_key(&o.ty) {
                diags.push(o.span, format!("unknown type `{}`", o.ty));
            }
            if objects.insert(o.name.clone(), o.ty.clone()).is_some() {
                diags.push(o.span, format!("duplicate object `{}`", o.name));
            }
        }

        let mut agents = Interner::default();
        for (a, sp) in &ast.agents {
            if !objects.contains_key(a) {
                diags.push(*sp, format!("agent `{a}` is not a declared object"));
            }
            // `intern` merges a repeat into the existing id, so a duplicate would
            // otherwise pass unremarked while duplicate types, objects, and
            // predicates are all reported. Detect it by whether an id was added.
            let before = agents.len();
            agents.intern(a);
            if agents.len() == before {
                diags.push(*sp, format!("duplicate agent `{a}`"));
            }
        }

        let mut preds = HashMap::new();
        for p in &ast.props {
            for ty in &p.params {
                if ty != "Object" && !types.contains_key(ty) {
                    diags.push(p.span, format!("unknown type `{ty}` in predicate `{}`", p.name));
                }
            }
            if preds.insert(p.name.clone(), p.params.clone()).is_some() {
                diags.push(p.span, format!("duplicate predicate `{}`", p.name));
            }
        }

        let mut sig = Sig { types, objects, agents, atoms: Interner::default(), preds };

        // Expand each predicate over the objects of its parameter types (§7.1).
        let mut decls: Vec<(String, Vec<String>)> =
            sig.preds.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
        decls.sort(); // deterministic atom ids regardless of HashMap order
        for (name, params) in decls {
            for tuple in sig.tuples(&params) {
                sig.atoms.intern(&atom_key(&name, &tuple));
            }
        }
        sig
    }

    /// Every combination of objects matching `params`, in a deterministic order.
    fn tuples(&self, params: &[String]) -> Vec<Vec<String>> {
        let mut acc: Vec<Vec<String>> = vec![Vec::new()];
        for ty in params {
            let choices = self.objects_of(ty);
            let mut next = Vec::with_capacity(acc.len() * choices.len());
            for prefix in &acc {
                for c in &choices {
                    let mut v = prefix.clone();
                    v.push(c.clone());
                    next.push(v);
                }
            }
            acc = next;
        }
        acc
    }

    /// Whether `sub` is `sup` or descends from it. Reflexive; every type reaches `Object`.
    pub fn is_subtype(&self, sub: &str, sup: &str) -> bool {
        if sub == sup || sup == "Object" {
            return true;
        }
        let mut cur = sub.to_string();
        let mut steps = 0usize;
        while let Some(parent) = self.types.get(&cur) {
            if parent == sup {
                return true;
            }
            cur = parent.clone();
            steps += 1;
            if steps > self.types.len() {
                return false; // cycle; already reported at build time
            }
        }
        false
    }

    /// Declared objects whose type is `ty` or a subtype of it, sorted for determinism.
    pub fn objects_of(&self, ty: &str) -> Vec<String> {
        let mut out: Vec<String> = self
            .objects
            .iter()
            .filter(|(_, oty)| self.is_subtype(oty, ty))
            .map(|(name, _)| name.clone())
            .collect();
        out.sort();
        out
    }

    /// The id of a ground atom, or `None` if no such atom exists.
    pub fn atom_id(&self, pred: &str, args: &[String]) -> Option<AtomId> {
        let key = atom_key(pred, args);
        (0..self.atoms.len() as u32).find(|&i| self.atoms.name(i) == key)
    }

    /// The canonical name behind an atom id.
    ///
    /// # Panics
    /// If `id` was not produced by this signature.
    pub fn atom_name(&self, id: AtomId) -> &str {
        debug_assert!((id as usize) < self.atoms.len(), "atom id out of range");
        self.atoms.name(id)
    }

    /// The id of a declared agent, or `None`.
    pub fn agent_id(&self, name: &str) -> Option<AgentId> {
        (0..self.agents.len() as u32).find(|&i| self.agents.name(i) == name)
    }

    /// The name behind an agent id.
    ///
    /// # Panics
    /// If `id` was not produced by this signature.
    pub fn agent_name(&self, id: AgentId) -> &str {
        debug_assert!((id as usize) < self.agents.len(), "agent id out of range");
        self.agents.name(id)
    }

    /// How many ground atoms exist.
    pub fn n_atoms(&self) -> usize {
        self.atoms.len()
    }
    /// How many agents were declared.
    pub fn n_agents(&self) -> usize {
        self.agents.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{parse_file, Diagnostics};

    fn sig(src: &str) -> Sig {
        let mut d = Diagnostics::default();
        let ast = parse_file(src, &mut d);
        let s = Sig::build(&ast, &mut d);
        assert!(d.is_empty(), "unexpected errors:\n{}", d.render(src));
        s
    }

    const SRC: &str = r#"
        types   { Actor - Object, Location - Object, Robot - Actor, Droid - Robot }
        objects { alice - Actor, r2 - Robot, study, hall - Location }
        agents  { alice, r2 }
        props   { heads, at(Actor, Location) }
        initially { heads }
        actions {}
    "#;

    #[test]
    fn subtyping_is_transitive_and_rooted_at_object() {
        let s = sig(SRC);
        assert!(s.is_subtype("Robot", "Actor"));
        assert!(s.is_subtype("Robot", "Object"), "every type reaches Object");
        // Two hops through non-`Object` ancestors (Droid -> Robot -> Actor): a
        // transitivity bug that only special-cases `Object` as the target
        // would not be caught by the assertion above, since `is_subtype`
        // short-circuits whenever `sup == "Object"`.
        assert!(s.is_subtype("Droid", "Actor"), "transitive over two non-Object hops");
        assert!(s.is_subtype("Actor", "Actor"), "reflexive");
        assert!(!s.is_subtype("Actor", "Robot"), "not symmetric");
        assert!(!s.is_subtype("Location", "Actor"));
        assert!(!s.is_subtype("Actor", "Droid"));
    }

    #[test]
    fn objects_of_a_type_include_its_subtypes() {
        let s = sig(SRC);
        let mut actors = s.objects_of("Actor");
        actors.sort();
        assert_eq!(actors, vec!["alice".to_string(), "r2".to_string()],
                   "r2 is a Robot, and Robot is an Actor");
    }

    #[test]
    fn predicates_expand_over_the_objects_of_their_parameter_types() {
        let s = sig(SRC);
        // heads = 1 atom; at(Actor, Location) = 2 actors x 2 locations = 4
        assert_eq!(s.n_atoms(), 5);
        assert!(s.atom_id("heads", &[]).is_some());
        assert!(s.atom_id("at", &["alice".into(), "study".into()]).is_some());
        assert!(s.atom_id("at", &["r2".into(), "hall".into()]).is_some());
        assert!(s.atom_id("at", &["study".into(), "alice".into()]).is_none(),
                "arguments must respect the declared parameter types");
    }

    #[test]
    fn a_predicate_over_an_empty_type_yields_no_atoms() {
        let s = sig(r#"
            types { Ghost - Object }
            objects { }
            agents { }
            props { haunts(Ghost) }
            initially { }
            actions {}
        "#);
        assert_eq!(s.n_atoms(), 0);
    }

    #[test]
    fn agents_get_dense_ids_in_declaration_order() {
        let s = sig(SRC);
        assert_eq!(s.n_agents(), 2);
        assert_eq!(s.agent_id("alice"), Some(0));
        assert_eq!(s.agent_id("r2"), Some(1));
        assert_eq!(s.agent_id("nobody"), None);
    }

    #[test]
    fn an_undeclared_supertype_is_reported() {
        let mut d = Diagnostics::default();
        let ast = parse_file(
            "types{ Actor - Nope } objects{} agents{} props{} initially{} actions{}", &mut d);
        let _ = Sig::build(&ast, &mut d);
        assert!(d.items().iter().any(|x| x.message.contains("Nope")));
    }

    #[test]
    fn an_object_of_an_undeclared_type_is_reported() {
        let mut d = Diagnostics::default();
        let ast = parse_file(
            "types{} objects{ bob - Ghost } agents{} props{} initially{} actions{}", &mut d);
        let _ = Sig::build(&ast, &mut d);
        assert!(d.items().iter().any(|x| x.message.contains("Ghost")));
    }

    #[test]
    fn a_duplicate_agent_is_reported() {
        // `Interner::intern` merges a repeat into the existing id, so nothing about
        // the resulting signature records that `alice` was written twice: the agent
        // count is 1 either way. Duplicate types, objects, and predicates are all
        // diagnosed, and this must be too.
        let mut d = Diagnostics::default();
        let ast = parse_file(
            "types{} objects{ alice - Object } agents{ alice, alice } props{} initially{} actions{}",
            &mut d,
        );
        let s = Sig::build(&ast, &mut d);
        assert_eq!(s.n_agents(), 1, "the id space still dedups");
        assert_eq!(d.len(), 1, "exactly one complaint, and it is the duplicate");
        assert!(d.items()[0].message.contains("duplicate agent `alice`"),
                "got: {}", d.items()[0].message);
    }

    #[test]
    fn an_agent_that_is_not_an_object_is_reported() {
        let mut d = Diagnostics::default();
        let ast = parse_file(
            "types{} objects{} agents{ ghost } props{} initially{} actions{}", &mut d);
        let _ = Sig::build(&ast, &mut d);
        assert!(d.items().iter().any(|x| x.message.contains("ghost")));
    }
}
