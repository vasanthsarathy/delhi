//! Canonical byte keys for states, so equality is a memcmp and states can be hashed (§5.1).

use crate::{rels_full, Model, State};
use std::collections::{BTreeMap, HashSet};

fn encode(m: &Model, order: &[usize], designated_pos: usize) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&(m.n_worlds as u32).to_le_bytes());
    out.extend_from_slice(&(m.n_agents as u32).to_le_bytes());
    out.extend_from_slice(&(designated_pos as u32).to_le_bytes());
    for &w in order {
        for v in m.val[w].ones() {
            out.extend_from_slice(&(v as u32).to_le_bytes());
        }
        out.push(0xff);
    }
    let pos: Vec<usize> = {
        let mut p = vec![0usize; m.n_worlds];
        for (i, &w) in order.iter().enumerate() {
            p[w] = i;
        }
        p
    };
    for i in 0..m.n_agents {
        for &w in order {
            let mut targets: Vec<u32> =
                m.rel[i][w].ones().into_iter().map(|v| pos[v] as u32).collect();
            targets.sort_unstable();
            for t in targets {
                out.extend_from_slice(&t.to_le_bytes());
            }
            out.push(0xfe);
        }
    }
    out
}

/// Ranks each key by its position in sorted order, so ids do not depend on
/// the order worlds happen to be visited in.
fn rank_by_sorted_key(keys: &[Vec<u64>]) -> Vec<usize> {
    let mut rank: BTreeMap<&Vec<u64>, usize> = BTreeMap::new();
    for k in keys {
        rank.insert(k, 0);
    }
    for (i, slot) in rank.values_mut().enumerate() {
        *slot = i;
    }
    keys.iter().map(|k| rank[k]).collect()
}

/// A canonical ordering of the worlds of an already-contracted model.
///
/// Runs the same colour refinement `blocks_full` uses, but assigns colour ids by
/// sorting the signatures rather than by first occurrence, so the result is
/// invariant under renaming of worlds.
///
/// # Preconditions
/// `m` must already be `~D`-contracted. On a contracted model no two worlds are
/// bisimilar, so refinement ends with every world in its own colour and the colour
/// order is a total order. The `debug_assert!` below enforces that.
fn canonical_order(m: &Model) -> Vec<usize> {
    let rels = rels_full(m);
    let mut keys: Vec<Vec<u64>> = (0..m.n_worlds)
        .map(|w| m.val[w].ones().into_iter().map(|a| a as u64).collect())
        .collect();
    let mut colour = rank_by_sorted_key(&keys);
    loop {
        keys = (0..m.n_worlds)
            .map(|w| {
                let mut sig = vec![colour[w] as u64];
                for rel in &rels {
                    sig.push(u64::MAX); // separator between relations
                    let mut nbr: Vec<u64> =
                        rel[w].ones().into_iter().map(|v| colour[v] as u64).collect();
                    nbr.sort_unstable();
                    sig.extend(nbr);
                }
                sig
            })
            .collect();
        let next = rank_by_sorted_key(&keys);
        if next == colour {
            break;
        }
        colour = next;
    }
    debug_assert!(
        colour.iter().collect::<HashSet<_>>().len() == m.n_worlds,
        "canonical_order: model was not contracted — two worlds share a colour"
    );
    let mut order: Vec<usize> = (0..m.n_worlds).collect();
    order.sort_by_key(|&w| colour[w]);
    order
}

impl State {
    /// A canonical byte key. Two states share a key exactly when they are
    /// `~D`-equivalent.
    pub fn key(&self) -> Vec<u8> {
        let (contracted, map) = self.model.contract_full();
        let d = map[self.designated] as usize;
        let order = canonical_order(&contracted);
        let dpos = order
            .iter()
            .position(|&w| w == d)
            .expect("designated world must appear in the canonical order");
        encode(&contracted, &order, dpos)
    }
}

#[cfg(test)]
mod tests {
    use crate::{Model, State};

    fn two_world(swap: bool) -> State {
        // Same state, world ids swapped. Keys must agree.
        let mut m = Model::new(2, 1, 1);
        let (top, bottom) = if swap { (1, 0) } else { (0, 1) };
        m.val[top].set(0);
        m.relate(0, bottom, top);
        State { model: m, designated: top }
    }

    #[test]
    fn key_is_invariant_under_world_renaming() {
        assert_eq!(two_world(false).key(), two_world(true).key());
    }

    #[test]
    fn key_soundness_equal_keys_imply_equivalent() {
        let a = two_world(false);
        let b = two_world(true);
        assert_eq!(a.key(), b.key());
        assert!(a.equivalent(&b), "equal keys must imply equivalence");
    }

    #[test]
    fn key_has_no_false_negatives_for_equivalent_states() {
        let a = two_world(false);
        let b = two_world(true);
        assert!(a.equivalent(&b));
        assert_eq!(a.key(), b.key(), "equivalent states must share a key");
    }

    #[test]
    fn different_states_get_different_keys() {
        let a = two_world(false);
        let mut m = Model::new(2, 1, 1);
        m.val[1].set(0); // belief now lands on the atom-free world
        m.relate(0, 0, 1);
        let b = State { model: m, designated: 0 };
        assert_ne!(a.key(), b.key());
    }
}
