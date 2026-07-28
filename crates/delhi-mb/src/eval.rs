//! Entailment for `L_GB` (§4.2), memoised on `(FormulaId, WorldId)`.

use crate::{common_closure, maxima, Bits, Derived, Model, State, WorldId};
use delhi_syntax::{AgentMask, FormulaId, Node, Store};
use std::collections::HashMap;

/// Evaluates `L_GB` formulas against one model, caching per `(formula, world)`.
pub struct Evaluator<'a> {
    store: &'a Store,
    model: &'a Model,
    derived: Derived,
    closures: HashMap<AgentMask, Vec<Bits>>,
    memo: HashMap<(FormulaId, WorldId), bool>,
}

impl<'a> Evaluator<'a> {
    /// Builds an evaluator, computing `~ᵢ` and `Belᵢ` once.
    pub fn new(store: &'a Store, model: &'a Model) -> Self {
        Evaluator {
            store,
            model,
            derived: Derived::of(model),
            closures: HashMap::new(),
            memo: HashMap::new(),
        }
    }

    /// Whether `⟨M, w⟩ ⊨ f`.
    ///
    /// # Panics
    /// If `w` is out of range for the model this evaluator was built for.
    pub fn eval(&mut self, f: FormulaId, w: WorldId) -> bool {
        debug_assert!(w < self.model.n_worlds, "eval: world out of range");
        if let Some(&hit) = self.memo.get(&(f, w)) {
            return hit;
        }
        let out = self.eval_uncached(f, w);
        self.memo.insert((f, w), out);
        out
    }

    fn eval_uncached(&mut self, f: FormulaId, w: WorldId) -> bool {
        match self.store.node(f).clone() {
            Node::True => true,
            Node::Atom(a) => self.model.val[w].get(a as usize),
            Node::Not(g) => !self.eval(g, w),
            Node::And(a, b) => self.eval(a, w) && self.eval(b, w),
            Node::Knows(i, g) => {
                self.derived.comp[i as usize][w].ones().into_iter().all(|v| self.eval(g, v))
            }
            Node::Believes(i, g) => {
                self.derived.bel[i as usize][w].ones().into_iter().all(|v| self.eval(g, v))
            }
            Node::Safe(i, g) => {
                self.model.rel[i as usize][w].ones().into_iter().all(|v| self.eval(g, v))
            }
            Node::Common(g, inner) => {
                // Cloning the cached closure rows is a deliberate borrow-checker tradeoff:
                // the `&mut self` recursion into `self.eval` forces it, not an oversight.
                let rows = match self.closures.get(&g) {
                    Some(r) => r.clone(),
                    None => {
                        let r = common_closure(self.model, g);
                        self.closures.insert(g, r.clone());
                        r
                    }
                };
                rows[w].ones().into_iter().all(|v| self.eval(inner, v))
            }
            Node::CondBel(i, psi, phi) => {
                // `→ᵢ(~ᵢᵘ ∩ ⟦ψ⟧)`; vacuously true when that set is empty (§4.2).
                let class = self.derived.comp[i as usize][w].clone();
                let mut sat = Bits::new(self.model.n_worlds);
                for v in class.ones() {
                    if self.eval(psi, v) {
                        sat.set(v);
                    }
                }
                if sat.is_empty() {
                    return true;
                }
                maxima(self.model, i as usize, &sat).ones().into_iter().all(|v| self.eval(phi, v))
            }
        }
    }
}

impl State {
    /// Whether the designated world models `f`.
    pub fn entails(&self, store: &Store, f: FormulaId) -> bool {
        Evaluator::new(store, &self.model).eval(f, self.designated)
    }
}

#[cfg(test)]
mod tests {
    use crate::{Model, State};
    use delhi_syntax::Store;

    /// [T] Fig. 5.4. Atom 0 is `h`. Agents: 0 = A, 1 = B, 2 = C.
    /// A and B have only reflexive edges; C has `v R_C u`.
    fn coin_lie_s0() -> State {
        let mut m = Model::new(2, 3, 1);
        m.val[0].set(0);
        m.relate(2, 1, 0);
        State { model: m, designated: 0 }
    }

    #[test]
    fn coin_lie_s0_entailments_match_the_paper() {
        let s = coin_lie_s0();
        assert_eq!(s.model.validate(), Ok(()));
        let mut st = Store::default();
        let h = st.atom(0);

        // A and B know the coin is heads.
        let ka = st.knows(0, h);
        let kb = st.knows(1, h);
        assert!(s.entails(&st, ka));
        assert!(s.entails(&st, kb));

        // C does not know which way it lies...
        let ig = st.ignorant(2, h);
        assert!(s.entails(&st, ig));

        // ...but correctly leans toward heads (§8.1).
        let bc = st.believes(2, h);
        assert!(s.entails(&st, bc));

        // Common knowledge that A knows whether h.
        let kw = st.knows_whether(0, h);
        let c = st.common(0b111, kw);
        assert!(s.entails(&st, c));
    }

    #[test]
    fn safe_belief_is_factive_and_sits_between_k_and_b() {
        // §8.2: K → □ → B, and □ entails φ because Rᵢ is reflexive.
        let s = coin_lie_s0();
        let mut st = Store::default();
        let h = st.atom(0);
        let sq = st.safe(2, h);
        assert!(s.entails(&st, sq), "R_C(u) = {{u}} and h holds at u");
        assert!(s.entails(&st, h));
    }

    #[test]
    fn cond_bel_on_top_agrees_with_plain_belief() {
        // §4.2: `B[i]φ ≡ B^⊤[i]φ`.
        let s = coin_lie_s0();
        let mut st = Store::default();
        let h = st.atom(0);
        let t = st.tru();
        let b = st.believes(2, h);
        let cb = st.cond_bel(2, t, h);
        assert_eq!(s.entails(&st, b), s.entails(&st, cb));
    }

    #[test]
    fn cond_bel_on_an_impossible_condition_is_vacuous() {
        let s = coin_lie_s0();
        let mut st = Store::default();
        let h = st.atom(0);
        let f = st.fls();
        let cb = st.cond_bel(2, f, h);
        assert!(s.entails(&st, cb));
    }

    /// One agent, three worlds, atom 0 = `p`, true only at world 0.
    /// Relation: 0 R 1, 0 R 2, 1 R 2, 2 R 1 — worlds 1 and 2 tie at the top,
    /// world 0 sits strictly below both.
    ///
    ///   at world 0:  rel = {0,1,2}  comp = {0,1,2}  bel = {1,2}   -> B differs from []
    ///   at world 1:  rel = {1,2}    comp = {0,1,2}  bel = {1,2}   -> K differs from [] and B
    fn three_level() -> Model {
        let mut m = Model::new(3, 1, 1);
        m.val[0].set(0);
        m.relate(0, 0, 1);
        m.relate(0, 0, 2);
        m.relate(0, 1, 2);
        m.relate(0, 2, 1);
        m
    }

    #[test]
    fn knows_safe_and_believes_quantify_over_different_sets() {
        let m = three_level();
        assert_eq!(m.validate(), Ok(()));
        let mut st = Store::default();
        let p = st.atom(0);
        let np = st.not(p);
        let b = st.believes(0, np);
        let sq = st.safe(0, np);
        let k = st.knows(0, np);

        // World 0: belief looks only at the maxima {1,2}; safe belief also sees world 0.
        let at0 = State { model: m.clone(), designated: 0 };
        assert!(at0.entails(&st, b), "B quantifies over {{1,2}}, where !p holds");
        assert!(!at0.entails(&st, sq), "[] quantifies over {{0,1,2}}, and p holds at 0");

        // World 1: knowledge sees the whole class {0,1,2}; safe belief only {1,2}.
        let at1 = State { model: m, designated: 1 };
        assert!(!at1.entails(&st, k), "K quantifies over {{0,1,2}}, and p holds at 0");
        assert!(at1.entails(&st, sq), "[] quantifies over {{1,2}}, where !p holds");
    }
}
