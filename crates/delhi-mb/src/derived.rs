//! Relations derived from `Rᵢ`: `~ᵢ`, `→ᵢᵘ`, `Belᵢ`, and the `C_g` closure (§4.1).

use crate::{Bits, Model};
use delhi_syntax::AgentMask;

/// `~ᵢ` and `Belᵢ` for every agent, computed once per model.
#[derive(Clone, Debug)]
pub struct Derived {
    /// `comp[i][u]` is `~ᵢᵘ`.
    pub comp: Vec<Vec<Bits>>,
    /// `bel[i][u]` is `→ᵢᵘ`, the maxima of `~ᵢᵘ`.
    pub bel: Vec<Vec<Bits>>,
}

impl Derived {
    /// Computes both families.
    pub fn of(m: &Model) -> Self {
        let mut comp = Vec::with_capacity(m.n_agents);
        let mut bel = Vec::with_capacity(m.n_agents);
        for i in 0..m.n_agents {
            let c = m.comparability_rows(i);
            let b = (0..m.n_worlds).map(|u| maxima(m, i, &c[u])).collect();
            comp.push(c);
            bel.push(b);
        }
        Derived { comp, bel }
    }
}

/// The `Rᵢ`-maxima of `set`: `{w ∈ set | ∀x ∈ set. x Rᵢ w}`.
///
/// # Panics
/// If `agent` is out of range for `m`.
///
/// # Preconditions
/// `set` must lie within a single `~ᵢ` comparability class. Under that condition a
/// non-empty `set` yields a non-empty result, because local connectedness makes the
/// class a total preorder. For a `set` spanning several classes the result may be
/// empty — [T] §5.1.1 asserts non-emptiness unconditionally, which is wrong (§4.1).
pub fn maxima(m: &Model, agent: usize, set: &Bits) -> Bits {
    debug_assert!(agent < m.n_agents, "maxima: agent out of range");
    let members = set.ones();
    let mut out = Bits::new(m.n_worlds);
    for &w in &members {
        if members.iter().all(|&x| m.rel[agent][x].get(w)) {
            out.set(w);
        }
    }
    out
}

/// The reflexive-transitive closure of `∪_{i∈g} ~ᵢ`, one row per world.
pub fn common_closure(m: &Model, g: AgentMask) -> Vec<Bits> {
    let mut c = vec![Bits::new(m.n_worlds); m.n_worlds];
    for i in 0..m.n_agents {
        if g >> i & 1 == 0 {
            continue;
        }
        let rows = m.comparability_rows(i);
        for (u, c_u) in c.iter_mut().enumerate() {
            c_u.union_with(&rows[u]);
        }
    }
    for (u, c_u) in c.iter_mut().enumerate() {
        c_u.set(u);
    }
    // Jacobi iteration: collect all updates in a pass, then apply them together.
    // This avoids in-place mutation while iterating (which would conflict with the
    // borrow checker) and repeats until the closure reaches a fixed point.
    loop {
        let mut changed = false;
        let mut updates = Vec::new();
        for (u, _) in c.iter().enumerate() {
            let mut add = Bits::new(m.n_worlds);
            for v in c[u].ones() {
                add.union_with(&c[v]);
            }
            if !c[u].contains_all(&add) {
                updates.push((u, add));
                changed = true;
            }
        }
        for (u, add) in updates {
            c[u].union_with(&add);
        }
        if !changed {
            break;
        }
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Bits, Model};

    /// [T] Fig. 5.4 again: `v R_C u`, so `u` is the most plausible world.
    fn coin_lie_s0() -> Model {
        let mut m = Model::new(2, 1, 1);
        m.val[0].set(0);
        m.relate(0, 1, 0);
        m
    }

    #[test]
    fn coin_lie_s0_derived_sets_match_the_figure() {
        let m = coin_lie_s0();
        let d = Derived::of(&m);
        // ~_C^u = {u, v}: C can rank them, so both are live.
        assert_eq!(d.comp[0][0].ones(), vec![0, 1]);
        // R_C(u) = {u}: only the reflexive edge leaves u.
        assert_eq!(m.rel[0][0].ones(), vec![0]);
        // ->_C^u = {u}: everything in the class points to u.
        assert_eq!(d.bel[0][0].ones(), vec![0]);
    }

    #[test]
    fn maxima_of_a_multi_class_set_is_empty() {
        // §4.1: [T] asserts non-emptiness unconditionally; that is false when the
        // input spans two comparability classes. This pins the corrected statement.
        let m = Model::new(2, 1, 1); // no edges: two singleton classes
        let mut both = Bits::new(2);
        both.set(0);
        both.set(1);
        assert!(maxima(&m, 0, &both).is_empty());
    }

    #[test]
    fn common_closure_joins_agents() {
        // Agent 0 links {0,1}; agent 1 links {1,2}; together the closure is {0,1,2}.
        let mut m = Model::new(3, 2, 1);
        m.relate(0, 0, 1);
        m.relate(1, 1, 2);
        let c = common_closure(&m, 0b11);
        assert_eq!(c[0].ones(), vec![0, 1, 2]);
        let only_first = common_closure(&m, 0b01);
        assert_eq!(only_first[0].ones(), vec![0, 1]);
    }

    #[test]
    fn common_closure_iterates_to_a_fixpoint_not_just_one_pass() {
        // 0~1 (agent 0), 1~2 (agent 1), 2~3 (agent 2). Reaching world 3 from
        // world 0 takes three hops, so a single collect-and-apply pass is not
        // enough — this pins the repeat-until-stable loop.
        let mut m = Model::new(4, 3, 1);
        m.relate(0, 0, 1);
        m.relate(1, 1, 2);
        m.relate(2, 2, 3);
        let c = common_closure(&m, 0b111);
        assert_eq!(c[0].ones(), vec![0, 1, 2, 3]);
        assert_eq!(c[3].ones(), vec![0, 1, 2, 3]);
    }
}
