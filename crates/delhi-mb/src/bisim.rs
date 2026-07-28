//! Two bisimulation notions (§6.3).
//!
//! * `~R` — over `{Rᵢ, Rᵢ⁻¹}`. What `[J]` implements. Sound (§6.1.2), incomplete.
//! * `~D` — over `{Rᵢ, ~ᵢ, Belᵢ, C}`. Exactly modal equivalence for `K/B/□/C`.

use crate::{common_closure, Bits, Derived, Model, State};
use std::collections::HashMap;

/// Partition refinement to the coarsest bisimulation over `rels`, starting from `init`.
///
/// `rels` is a flat list of relations, each a row per world. `init` assigns an initial
/// block to each world (normally the valuation class).
///
/// # Panics
/// If `init.len() != n_worlds`, or any relation in `rels` does not have exactly
/// `n_worlds` rows.
pub fn refine(rels: &[Vec<Bits>], init: &[u32], n_worlds: usize) -> Vec<u32> {
    debug_assert_eq!(init.len(), n_worlds, "refine: init must have one entry per world");
    debug_assert!(
        rels.iter().all(|r| r.len() == n_worlds),
        "refine: every relation must have exactly n_worlds rows"
    );
    let mut block = canonicalise(init);
    loop {
        let mut seen: HashMap<(u32, Vec<Vec<u32>>), u32> = HashMap::new();
        let mut next = vec![0u32; n_worlds];
        for u in 0..n_worlds {
            let mut sig = Vec::with_capacity(rels.len());
            for rel in rels {
                let mut reached: Vec<u32> = rel[u].ones().into_iter().map(|v| block[v]).collect();
                reached.sort_unstable();
                reached.dedup();
                sig.push(reached);
            }
            let key = (block[u], sig);
            let id = seen.len() as u32;
            next[u] = *seen.entry(key).or_insert(id);
        }
        let next = canonicalise(&next);
        if next == block {
            return block;
        }
        block = next;
    }
}

fn canonicalise(p: &[u32]) -> Vec<u32> {
    let mut map: HashMap<u32, u32> = HashMap::new();
    let mut out = Vec::with_capacity(p.len());
    for &x in p {
        let id = map.len() as u32;
        out.push(*map.entry(x).or_insert(id));
    }
    out
}

fn transpose(rows: &[Bits], n: usize) -> Vec<Bits> {
    let mut t = vec![Bits::new(n); n];
    for (u, row) in rows.iter().enumerate() {
        for v in row.ones() {
            t[v].set(u);
        }
    }
    t
}

fn valuation_classes(m: &Model) -> Vec<u32> {
    let mut seen: HashMap<&Bits, u32> = HashMap::new();
    let mut out = Vec::with_capacity(m.n_worlds);
    for w in 0..m.n_worlds {
        let id = seen.len() as u32;
        out.push(*seen.entry(&m.val[w]).or_insert(id));
    }
    out
}

/// `{Rᵢ, Rᵢ⁻¹}` for every agent — the `~R` relation set.
pub fn rels_dynamic(m: &Model) -> Vec<Vec<Bits>> {
    let mut out = Vec::with_capacity(m.n_agents * 2);
    for i in 0..m.n_agents {
        out.push(m.rel[i].clone());
        out.push(transpose(&m.rel[i], m.n_worlds));
    }
    out
}

/// `{Rᵢ, ~ᵢ, Belᵢ}` for every agent plus the all-agent `C` closure — the `~D` set.
pub fn rels_full(m: &Model) -> Vec<Vec<Bits>> {
    let d = Derived::of(m);
    let mut out = Vec::with_capacity(m.n_agents * 3 + 1);
    for i in 0..m.n_agents {
        out.push(m.rel[i].clone());
        out.push(d.comp[i].clone());
        out.push(d.bel[i].clone());
    }
    let all: u32 = if m.n_agents >= 32 { u32::MAX } else { (1u32 << m.n_agents) - 1 };
    out.push(common_closure(m, all));
    out
}

/// Block ids under `~R`.
pub fn blocks_dynamic(m: &Model) -> Vec<u32> {
    refine(&rels_dynamic(m), &valuation_classes(m), m.n_worlds)
}

/// Block ids under `~D`.
pub fn blocks_full(m: &Model) -> Vec<u32> {
    refine(&rels_full(m), &valuation_classes(m), m.n_worlds)
}

fn quotient(m: &Model, blocks: &[u32]) -> (Model, Vec<u32>) {
    let n_new = blocks.iter().copied().max().map_or(0, |x| x as usize + 1);
    let mut rep = vec![usize::MAX; n_new];
    for (w, &b) in blocks.iter().enumerate() {
        if rep[b as usize] == usize::MAX {
            rep[b as usize] = w;
        }
    }
    let mut out = Model::new(n_new, m.n_agents, m.n_atoms);
    for (b, &r) in rep.iter().enumerate() {
        out.val[b] = m.val[r].clone();
    }
    for i in 0..m.n_agents {
        for u in 0..m.n_worlds {
            for v in m.rel[i][u].ones() {
                out.rel[i][blocks[u] as usize].set(blocks[v] as usize);
            }
        }
    }
    (out, blocks.to_vec())
}

impl Model {
    /// Quotient by `~R`. Sound and a congruence for product update; incomplete.
    pub fn contract_dynamic(&self) -> (Model, Vec<u32>) {
        let b = blocks_dynamic(self);
        quotient(self, &b)
    }
    /// Quotient by `~D`. Complete for `K/B/□/C`; congruence status open (§6.3).
    pub fn contract_full(&self) -> (Model, Vec<u32>) {
        let b = blocks_full(self);
        quotient(self, &b)
    }
}

fn joint(a: &State, b: &State) -> (Model, usize, usize) {
    debug_assert_eq!(a.model.n_agents, b.model.n_agents, "joint: states must share an agent count");
    let n = a.model.n_worlds + b.model.n_worlds;
    let agents = a.model.n_agents.max(b.model.n_agents);
    let atoms = a.model.n_atoms.max(b.model.n_atoms);
    let mut m = Model::new(n, agents, atoms);
    // NOTE: `Bits` derives `PartialEq`/`Hash` over its backing `Vec<u64>`, so two `Bits`
    // with identical members but different capacities compare unequal and hash
    // differently. Cloning `a.model.val[w]`/`b.model.val[w]` directly here would keep
    // the *source* capacity, not `m`'s (`atoms`), which corrupts `valuation_classes`'
    // `HashMap<&Bits, u32>` keying whenever the two models have different `n_atoms`.
    // Rebuild each valuation at the target capacity via `.ones()` + `.set()` instead.
    for w in 0..a.model.n_worlds {
        for atom in a.model.val[w].ones() {
            m.val[w].set(atom);
        }
    }
    for w in 0..b.model.n_worlds {
        for atom in b.model.val[w].ones() {
            m.val[a.model.n_worlds + w].set(atom);
        }
    }
    for i in 0..agents {
        for u in 0..a.model.n_worlds {
            for v in a.model.rel[i][u].ones() {
                m.rel[i][u].set(v);
            }
        }
        for u in 0..b.model.n_worlds {
            for v in b.model.rel[i][u].ones() {
                m.rel[i][a.model.n_worlds + u].set(a.model.n_worlds + v);
            }
        }
    }
    (m, a.designated, a.model.n_worlds + b.designated)
}

impl State {
    /// Whether the two designated worlds fall in one `~R` block of the disjoint union.
    ///
    /// # Panics
    /// If `self` and `other` do not have the same `n_agents`.
    pub fn bisimilar_dynamic(&self, other: &State) -> bool {
        let (m, x, y) = joint(self, other);
        let b = blocks_dynamic(&m);
        b[x] == b[y]
    }
    /// Whether the two states satisfy exactly the same `K/B/□/C` formulas (§6.3).
    ///
    /// # Panics
    /// If `self` and `other` do not have the same `n_agents`.
    pub fn equivalent(&self, other: &State) -> bool {
        let (m, x, y) = joint(self, other);
        let b = blocks_full(&m);
        b[x] == b[y]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Model;

    /// The smallest witness from §6.1.3, verbatim: three worlds, one agent.
    /// `0 ⇄ 1` tie at the top; `2` strictly below both. Valuations: 0 ↦ a, 1,2 ↦ b.
    fn witness() -> Model {
        let mut m = Model::new(3, 1, 1);
        m.val[0].set(0);
        m.relate(0, 0, 1);
        m.relate(0, 1, 0);
        m.relate(0, 2, 0);
        m.relate(0, 2, 1);
        m
    }

    #[test]
    fn witness_is_a_valid_frame() {
        assert_eq!(witness().validate(), Ok(()));
    }

    #[test]
    fn tilde_d_merges_worlds_one_and_two_but_tilde_r_does_not() {
        let m = witness();
        let br = blocks_dynamic(&m);
        let bd = blocks_full(&m);
        assert_ne!(br[1], br[2], "~R splits them, because R⁻¹(1) != R⁻¹(2)");
        assert_eq!(bd[1], bd[2], "~D merges them: no operator sees the difference");
    }

    #[test]
    fn tilde_r_refines_tilde_d() {
        // §6.1.2/§6.3: ~R ⊆ ~D, with zero exceptions across 451_730 models.
        let m = witness();
        let br = blocks_dynamic(&m);
        let bd = blocks_full(&m);
        for u in 0..m.n_worlds {
            for v in 0..m.n_worlds {
                if br[u] == br[v] {
                    assert_eq!(bd[u], bd[v], "~R merged {u},{v} but ~D split them");
                }
            }
        }
    }

    #[test]
    fn contraction_shrinks_the_witness_under_full_but_not_dynamic() {
        let m = witness();
        assert_eq!(m.contract_dynamic().0.n_worlds, 3);
        assert_eq!(m.contract_full().0.n_worlds, 2);
    }

    #[test]
    fn joint_union_rebuilds_valuations_at_the_target_capacity() {
        // Two states with the same single atom true, but different atom counts, so the
        // source `Bits` have different backing widths. They must still compare equivalent.
        let mut a = Model::new(1, 1, 1);
        a.val[0].set(0);
        let mut b = Model::new(1, 1, 100);
        b.val[0].set(0);
        let sa = State { model: a, designated: 0 };
        let sb = State { model: b, designated: 0 };
        assert!(sa.equivalent(&sb), "same atom set must compare equivalent regardless of capacity");
        assert!(sa.bisimilar_dynamic(&sb));
    }
}
