//! Hash-consed formulas. Identical subterms share a `FormulaId`, so structural
//! equality is integer equality and entailment can memoise on `(FormulaId, WorldId)`.

use std::collections::HashMap;

/// Index of a ground atomic proposition.
pub type AtomId = u32;
/// Index of an agent.
pub type AgentId = u32;
/// Bitset over agents, used as the group argument of `C_g`. Caps agents at 32.
pub type AgentMask = u32;
/// Handle into a [`Store`].
pub type FormulaId = u32;

/// One node of the formula DAG. `§4.2` fixes the operator set at six.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum Node {
    /// Verum.
    True,
    /// An atomic proposition.
    Atom(AtomId),
    /// Negation.
    Not(FormulaId),
    /// Conjunction.
    And(FormulaId, FormulaId),
    /// `K[i] φ` — box over `~ᵢ`.
    Knows(AgentId, FormulaId),
    /// `B[i] φ` — box over `Belᵢ`.
    Believes(AgentId, FormulaId),
    /// `□[i] φ` — box over `Rᵢ`.
    Safe(AgentId, FormulaId),
    /// `B^ψ[i] φ` — arguments are `(agent, ψ, φ)`.
    CondBel(AgentId, FormulaId, FormulaId),
    /// `C[g] φ` — box over the reflexive-transitive closure of `∪_{i∈g} ~ᵢ`.
    Common(AgentMask, FormulaId),
}

/// Arena of hash-consed formula nodes.
#[derive(Default, Debug, Clone)]
pub struct Store {
    nodes: Vec<Node>,
    map: HashMap<Node, FormulaId>,
}

impl Store {
    /// Interns a node, returning an existing id when the node is already present.
    pub fn mk(&mut self, n: Node) -> FormulaId {
        if let Some(&i) = self.map.get(&n) {
            return i;
        }
        let i = self.nodes.len() as FormulaId;
        self.nodes.push(n.clone());
        self.map.insert(n, i);
        i
    }

    /// The node behind an id.
    pub fn node(&self, f: FormulaId) -> &Node {
        &self.nodes[f as usize]
    }

    /// How many distinct nodes are interned.
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Whether the arena is empty.
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// `⊤`
    pub fn tru(&mut self) -> FormulaId {
        self.mk(Node::True)
    }
    /// `⊥`
    pub fn fls(&mut self) -> FormulaId {
        let t = self.tru();
        self.mk(Node::Not(t))
    }
    /// An atom.
    pub fn atom(&mut self, a: AtomId) -> FormulaId {
        self.mk(Node::Atom(a))
    }
    /// `!f`
    pub fn not(&mut self, f: FormulaId) -> FormulaId {
        self.mk(Node::Not(f))
    }
    /// `a & b`
    pub fn and(&mut self, a: FormulaId, b: FormulaId) -> FormulaId {
        self.mk(Node::And(a, b))
    }
    /// `a | b`, as `!(!a & !b)`.
    pub fn or(&mut self, a: FormulaId, b: FormulaId) -> FormulaId {
        let na = self.not(a);
        let nb = self.not(b);
        let c = self.and(na, nb);
        self.not(c)
    }
    /// `a -> b`
    pub fn implies(&mut self, a: FormulaId, b: FormulaId) -> FormulaId {
        let na = self.not(a);
        self.or(na, b)
    }
    /// Conjunction of a slice; `⊤` when empty.
    pub fn all(&mut self, fs: &[FormulaId]) -> FormulaId {
        match fs.split_first() {
            None => self.tru(),
            Some((&h, rest)) => rest.iter().fold(h, |acc, &f| self.and(acc, f)),
        }
    }
    /// Disjunction of a slice; `⊥` when empty.
    pub fn any(&mut self, fs: &[FormulaId]) -> FormulaId {
        match fs.split_first() {
            None => self.fls(),
            Some((&h, rest)) => rest.iter().fold(h, |acc, &f| self.or(acc, f)),
        }
    }
    /// `K[i] f`
    pub fn knows(&mut self, i: AgentId, f: FormulaId) -> FormulaId {
        self.mk(Node::Knows(i, f))
    }
    /// `B[i] f`
    pub fn believes(&mut self, i: AgentId, f: FormulaId) -> FormulaId {
        self.mk(Node::Believes(i, f))
    }
    /// `□[i] f`
    pub fn safe(&mut self, i: AgentId, f: FormulaId) -> FormulaId {
        self.mk(Node::Safe(i, f))
    }
    /// `B^psi[i] phi`
    pub fn cond_bel(&mut self, i: AgentId, psi: FormulaId, phi: FormulaId) -> FormulaId {
        self.mk(Node::CondBel(i, psi, phi))
    }
    /// `C[g] f`
    pub fn common(&mut self, g: AgentMask, f: FormulaId) -> FormulaId {
        self.mk(Node::Common(g, f))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_subterms_share_one_id() {
        let mut s = Store::default();
        let p = s.atom(0);
        let a = s.believes(0, p);
        let b = s.believes(0, p);
        assert_eq!(a, b, "hash-consing must return the same id");

        let before = s.len();
        let _ = s.believes(0, p);
        assert_eq!(s.len(), before, "re-making a node must not grow the arena");
    }

    #[test]
    fn or_is_built_from_not_and_and() {
        let mut s = Store::default();
        let p = s.atom(0);
        let q = s.atom(1);
        let disj = s.or(p, q);
        // !(!p & !q)
        match s.node(disj) {
            Node::Not(inner) => match s.node(*inner) {
                Node::And(x, y) => {
                    assert!(matches!(s.node(*x), Node::Not(f) if *f == p));
                    assert!(matches!(s.node(*y), Node::Not(f) if *f == q));
                }
                other => panic!("expected And, got {other:?}"),
            },
            other => panic!("expected Not, got {other:?}"),
        }
    }
}
