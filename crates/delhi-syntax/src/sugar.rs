//! Derived attitudes (§8.4). Every one is a boolean combination of the six
//! primitives, so `delhi-mb` never sees them.

use crate::{AgentId, FormulaId, Store};

impl Store {
    /// `Kw[i]φ` — agent knows whether φ is true: `K[i]φ | K[i]!φ`.
    pub fn knows_whether(&mut self, i: AgentId, f: FormulaId) -> FormulaId {
        let nf = self.not(f);
        let a = self.knows(i, f);
        let b = self.knows(i, nf);
        self.or(a, b)
    }
    /// `Bw[i]φ` — agent has a belief either way: `B[i]φ | B[i]!φ`.
    pub fn believes_whether(&mut self, i: AgentId, f: FormulaId) -> FormulaId {
        let nf = self.not(f);
        let a = self.believes(i, f);
        let b = self.believes(i, nf);
        self.or(a, b)
    }
    /// `?[i]φ` — agent is ignorant whether: `!K[i]φ & !K[i]!φ`.
    pub fn ignorant(&mut self, i: AgentId, f: FormulaId) -> FormulaId {
        let kw = self.knows_whether(i, f);
        self.not(kw)
    }
    /// `¿[i]φ` — agent suspends judgement: `!B[i]φ & !B[i]!φ`.
    pub fn undecided(&mut self, i: AgentId, f: FormulaId) -> FormulaId {
        let bw = self.believes_whether(i, f);
        self.not(bw)
    }
    /// `K'[i]φ` — agent considers φ possible: `!K[i]!φ`.
    pub fn considers_possible(&mut self, i: AgentId, f: FormulaId) -> FormulaId {
        let nf = self.not(f);
        let k = self.knows(i, nf);
        self.not(k)
    }
    /// `B'[i]φ` — agent has not ruled out φ: `!B[i]!φ`.
    pub fn not_ruled_out(&mut self, i: AgentId, f: FormulaId) -> FormulaId {
        let nf = self.not(f);
        let b = self.believes(i, nf);
        self.not(b)
    }
    /// `S'[i]φ` — φ is safe for agent to act on: `!□[i]!φ`.
    pub fn safe_dual(&mut self, i: AgentId, f: FormulaId) -> FormulaId {
        let nf = self.not(f);
        let s = self.safe(i, nf);
        self.not(s)
    }
    /// `K[a,b,…]φ` — all agents in the list know φ.
    pub fn knows_all(&mut self, agents: &[AgentId], f: FormulaId) -> FormulaId {
        let mut parts = Vec::with_capacity(agents.len());
        for &i in agents {
            parts.push(self.knows(i, f));
        }
        self.all(&parts)
    }
    /// `B[a,b,…]φ` — all agents in the list believe φ.
    pub fn believes_all(&mut self, agents: &[AgentId], f: FormulaId) -> FormulaId {
        let mut parts = Vec::with_capacity(agents.len());
        for &i in agents {
            parts.push(self.believes(i, f));
        }
        self.all(&parts)
    }
}

#[cfg(test)]
mod tests {
    use crate::{Node, Store};

    #[test]
    fn knows_whether_is_a_disjunction_of_knows() {
        let mut s = Store::default();
        let p = s.atom(0);
        let kw = s.knows_whether(0, p);
        let np = s.not(p);
        let expect = {
            let a = s.knows(0, p);
            let b = s.knows(0, np);
            s.or(a, b)
        };
        assert_eq!(kw, expect);
    }

    #[test]
    fn ignorant_is_neither_knows() {
        let mut s = Store::default();
        let p = s.atom(0);
        let ig = s.ignorant(0, p);
        // ignorant is exactly !knows_whether
        let kw = s.knows_whether(0, p);
        let expect = s.not(kw);
        assert_eq!(ig, expect);
    }

    #[test]
    fn agent_lists_distribute_over_knows() {
        let mut s = Store::default();
        let p = s.atom(0);
        let ka = s.knows(0, p);
        let kb = s.knows(1, p);
        let expect = s.and(ka, kb);
        assert_eq!(s.knows_all(&[0, 1], p), expect);
    }

    #[test]
    fn cond_bel_on_top_is_plain_belief() {
        // §4.2: `B[i]φ ≡ B^⊤[i]φ` is asserted here syntactically only;
        // Task 14 checks it semantically.
        let mut s = Store::default();
        let p = s.atom(0);
        let t = s.tru();
        let cb = s.cond_bel(0, t, p);
        assert!(matches!(s.node(cb), Node::CondBel(0, _, _)));
    }

    #[test]
    fn duals_negate_on_both_sides() {
        let mut s = Store::default();
        let p = s.atom(0);
        let np = s.not(p);

        let expect_k = {
            let k = s.knows(0, np);
            s.not(k)
        };
        assert_eq!(s.considers_possible(0, p), expect_k, "K'[a]p is !K[a]!p");

        let expect_b = {
            let b = s.believes(0, np);
            s.not(b)
        };
        assert_eq!(s.not_ruled_out(0, p), expect_b, "B'[a]p is !B[a]!p");

        let expect_s = {
            let sf = s.safe(0, np);
            s.not(sf)
        };
        assert_eq!(s.safe_dual(0, p), expect_s, "S'[a]p is ![][a]!p");
    }

    #[test]
    fn believes_whether_and_undecided_are_complementary() {
        let mut s = Store::default();
        let p = s.atom(0);
        let np = s.not(p);
        let expect_bw = {
            let a = s.believes(0, p);
            let b = s.believes(0, np);
            s.or(a, b)
        };
        assert_eq!(s.believes_whether(0, p), expect_bw);
        let bw = s.believes_whether(0, p);
        let expect_un = s.not(bw);
        assert_eq!(s.undecided(0, p), expect_un, "undecided is the negation of believes-whether");
    }

    #[test]
    fn believes_all_distributes_over_agents() {
        let mut s = Store::default();
        let p = s.atom(0);
        let expect = {
            let a = s.believes(0, p);
            let b = s.believes(1, p);
            s.and(a, b)
        };
        assert_eq!(s.believes_all(&[0, 1], p), expect);
    }
}
