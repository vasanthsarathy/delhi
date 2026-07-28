//! Action theories (§4.3) and the well-formedness rules `[J]` left as runtime asserts.

use delhi_syntax::{AgentId, AtomId, FormulaId, Node, Store};

/// One `causes` statement: a literal list under an optional condition.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Effect {
    /// `(atom, true)` makes it true; `(atom, false)` makes it false.
    pub lits: Vec<(AtomId, bool)>,
    /// The `if` guard; `⊤` when unconditional.
    pub cond: FormulaId,
}

/// Which of the three mutually exclusive forms this action takes (§4.3 forms 2–4).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Kind {
    /// `a causes l₀,…,lₙ if φ`
    Ontic(Vec<Effect>),
    /// `a determines φ` — propositional only.
    Sensing(FormulaId),
    /// `a announces ψ` — may be modal, and need not be true.
    Announce(FormulaId),
}

/// A ground action.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActionDef {
    /// Display name.
    pub name: String,
    /// `a_pre`. A single field, which is how D8's at-most-one rule is enforced.
    pub pre: FormulaId,
    /// The action's form.
    pub kind: Kind,
    /// `i observes a if φ`.
    pub observes: Vec<(AgentId, FormulaId)>,
    /// `i aware_of a if φ`.
    pub aware: Vec<(AgentId, FormulaId)>,
}

/// A violated well-formedness rule.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TheoryError {
    /// A `causes` list holds both `p` and `!p`.
    ContradictoryLiterals {
        /// The offending atom.
        atom: AtomId,
    },
    /// An agent appears in both `observes` and `aware`.
    ObserverClassOverlap {
        /// The offending agent.
        agent: AgentId,
    },
    /// `determines` was given a modal formula.
    ModalSensingFormula,
}

fn is_propositional(store: &Store, f: FormulaId) -> bool {
    match store.node(f) {
        Node::True | Node::Atom(_) => true,
        Node::Not(g) => is_propositional(store, *g),
        Node::And(a, b) => is_propositional(store, *a) && is_propositional(store, *b),
        _ => false,
    }
}

impl ActionDef {
    /// Checks the §4.3 rules. `[J]` left these as runtime asserts that are often disabled.
    pub fn validate(&self, store: &Store) -> Result<(), TheoryError> {
        for (i, _) in &self.observes {
            if self.aware.iter().any(|(j, _)| j == i) {
                return Err(TheoryError::ObserverClassOverlap { agent: *i });
            }
        }
        match &self.kind {
            Kind::Ontic(effects) => {
                for e in effects {
                    for (atom, sign) in &e.lits {
                        if e.lits.iter().any(|(b, s2)| b == atom && s2 != sign) {
                            return Err(TheoryError::ContradictoryLiterals { atom: *atom });
                        }
                    }
                }
            }
            Kind::Sensing(phi) => {
                if !is_propositional(store, *phi) {
                    return Err(TheoryError::ModalSensingFormula);
                }
            }
            // `announces` takes `ψ ∈ L^P_GB` — modal is allowed (§4.3).
            Kind::Announce(_) => {}
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use delhi_syntax::Store;

    #[test]
    fn a_well_formed_ontic_action_validates() {
        let mut s = Store::default();
        let t = s.tru();
        let a = ActionDef {
            name: "distract_a".into(),
            pre: t,
            kind: Kind::Ontic(vec![Effect { lits: vec![(0, true)], cond: t }]),
            observes: vec![(0, t), (1, t), (2, t)],
            aware: vec![],
        };
        assert_eq!(a.validate(&s), Ok(()));
    }

    #[test]
    fn contradictory_literals_are_rejected() {
        // §4.6: [T] forbids these; [MBD] silently let `add` win. We follow [T].
        let mut s = Store::default();
        let t = s.tru();
        let a = ActionDef {
            name: "bad".into(),
            pre: t,
            kind: Kind::Ontic(vec![Effect { lits: vec![(0, true), (0, false)], cond: t }]),
            observes: vec![],
            aware: vec![],
        };
        assert_eq!(a.validate(&s), Err(TheoryError::ContradictoryLiterals { atom: 0 }));
    }

    #[test]
    fn an_agent_cannot_be_both_observer_and_aware() {
        let mut s = Store::default();
        let t = s.tru();
        let a = ActionDef {
            name: "bad".into(),
            pre: t,
            kind: Kind::Sensing(t),
            observes: vec![(0, t)],
            aware: vec![(0, t)],
        };
        assert_eq!(a.validate(&s), Err(TheoryError::ObserverClassOverlap { agent: 0 }));
    }

    #[test]
    fn sensing_rejects_a_modal_formula() {
        // `determines` confers knowledge, which cannot be false, so it is
        // restricted to propositional formulas.
        let mut s = Store::default();
        let t = s.tru();
        let p = s.atom(0);
        let modal = s.knows(0, p);
        let a = ActionDef {
            name: "peek".into(),
            pre: t,
            kind: Kind::Sensing(modal),
            observes: vec![(0, t)],
            aware: vec![],
        };
        assert_eq!(a.validate(&s), Err(TheoryError::ModalSensingFormula));
    }

    #[test]
    fn announcement_accepts_a_modal_formula() {
        // `announces` confers belief and may be a lie, so agents can talk about
        // each other's mental states. Rejecting this would forbid exactly the
        // announcements this system exists to model.
        let mut s = Store::default();
        let t = s.tru();
        let p = s.atom(0);
        let modal = s.knows(0, p);
        let a = ActionDef {
            name: "tell".into(),
            pre: t,
            kind: Kind::Announce(modal),
            observes: vec![(0, t)],
            aware: vec![],
        };
        assert_eq!(a.validate(&s), Ok(()));
    }
}
