//! Plausibility models (§4.1, [T] §5.1.1) and their frame conditions.

use crate::Bits;

/// Index of a world.
pub type WorldId = usize;

/// A plausibility model `⟨W, R, V⟩` (§4.1).
///
/// # Equality
/// The derived `PartialEq` compares raw structure, including the backing width of
/// each [`Bits`]. Two models with the same worlds and valuations but different
/// `n_atoms`/`n_worlds` capacities therefore compare UNEQUAL. For semantic
/// comparison use [`State::equivalent`] (modal equivalence) or [`State::key`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Model {
    /// `|W|`.
    pub n_worlds: usize,
    /// `|G|`.
    pub n_agents: usize,
    /// `|P|`.
    pub n_atoms: usize,
    /// `val[w]` is the set of atoms true at `w`.
    pub val: Vec<Bits>,
    /// `rel[i][u]` is `{v | u Rᵢ v}` — the worlds `i` holds at least as plausible as `u`.
    pub rel: Vec<Vec<Bits>>,
}

/// A pointed model `⟨M, u⟩`.
///
/// # Equality
/// The derived `PartialEq` compares raw structure, including the backing width of
/// each [`Bits`]. Two models with the same worlds and valuations but different
/// `n_atoms`/`n_worlds` capacities therefore compare UNEQUAL. For semantic
/// comparison use [`State::equivalent`] (modal equivalence) or [`State::key`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct State {
    /// The model.
    pub model: Model,
    /// The actual world.
    pub designated: WorldId,
}

/// A violated frame condition, with a concrete witness.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FrameError {
    /// `u Rᵢ u` is missing.
    NotReflexive {
        /// Offending agent.
        agent: usize,
        /// Offending world.
        world: usize,
    },
    /// `u Rᵢ v` and `v Rᵢ w` hold but `u Rᵢ w` does not.
    NotTransitive {
        /// Offending agent.
        agent: usize,
        /// First world.
        u: usize,
        /// Second world.
        v: usize,
        /// Third world.
        w: usize,
    },
    /// `u` and `v` are joined by a chain of `Rᵢ` edges but are not comparable.
    NotLocallyConnected {
        /// Offending agent.
        agent: usize,
        /// First world.
        u: usize,
        /// Second world.
        v: usize,
    },
}

impl Model {
    /// A model with every relation the identity and every valuation empty.
    pub fn new(n_worlds: usize, n_agents: usize, n_atoms: usize) -> Self {
        let val = vec![Bits::new(n_atoms.max(1)); n_worlds];
        let mut rel = Vec::with_capacity(n_agents);
        for _ in 0..n_agents {
            let mut rows = Vec::with_capacity(n_worlds);
            for u in 0..n_worlds {
                let mut r = Bits::new(n_worlds);
                r.set(u);
                rows.push(r);
            }
            rel.push(rows);
        }
        Model { n_worlds, n_agents, n_atoms, val, rel }
    }

    /// Records `u Rᵢ v` — *"`v` is at least as plausible as `u`"*.
    ///
    /// # Panics
    /// If `agent`, `u`, or `v` is out of range for this model.
    pub fn relate(&mut self, agent: usize, u: WorldId, v: WorldId) {
        debug_assert!(
            agent < self.n_agents && u < self.n_worlds && v < self.n_worlds,
            "relate index out of range"
        );
        self.rel[agent][u].set(v);
    }

    /// `~ᵢ` as a row per world: `{v | u Rᵢ v or v Rᵢ u}`.
    ///
    /// # Panics
    /// If `agent` is out of range for this model.
    pub fn comparability_rows(&self, agent: usize) -> Vec<Bits> {
        debug_assert!(agent < self.n_agents, "comparability_rows: agent out of range");
        let mut c = vec![Bits::new(self.n_worlds); self.n_worlds];
        for (u, cu) in c.iter_mut().enumerate() {
            for v in 0..self.n_worlds {
                if self.rel[agent][u].get(v) || self.rel[agent][v].get(u) {
                    cu.set(v);
                }
            }
        }
        c
    }

    /// Checks reflexivity, transitivity, and local connectedness for every agent.
    pub fn validate(&self) -> Result<(), FrameError> {
        for i in 0..self.n_agents {
            for u in 0..self.n_worlds {
                if !self.rel[i][u].get(u) {
                    return Err(FrameError::NotReflexive { agent: i, world: u });
                }
            }
            for u in 0..self.n_worlds {
                for v in self.rel[i][u].ones() {
                    for w in self.rel[i][v].ones() {
                        if !self.rel[i][u].get(w) {
                            return Err(FrameError::NotTransitive { agent: i, u, v, w });
                        }
                    }
                }
            }
            // Local connectedness holds exactly when `~ᵢ` is transitive (§3.5).
            let c = self.comparability_rows(i);
            for u in 0..self.n_worlds {
                for v in c[u].ones() {
                    if !c[u].contains_all(&c[v]) {
                        return Err(FrameError::NotLocallyConnected { agent: i, u, v });
                    }
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The Coin Lie start state, [T] Fig. 5.4. One agent (C), two worlds.
    /// The drawn edge is `v ──C──► u`, so `v R_C u`: C holds the heads-world
    /// at least as plausible. §8.1 records why this direction is easy to invert.
    fn coin_lie_s0() -> Model {
        let mut m = Model::new(2, 1, 1);
        m.val[0].set(0); // world 0 = u, where h holds
        m.relate(0, 1, 0); // v R_C u
        m
    }

    #[test]
    fn valid_frame_passes() {
        assert_eq!(coin_lie_s0().validate(), Ok(()));
    }

    #[test]
    fn missing_reflexive_edge_is_rejected() {
        let mut m = Model::new(2, 1, 1);
        m.rel[0][0].unset(0);
        assert_eq!(m.validate(), Err(FrameError::NotReflexive { agent: 0, world: 0 }));
    }

    #[test]
    fn non_transitive_frame_is_rejected() {
        let mut m = Model::new(3, 1, 1);
        m.relate(0, 0, 1);
        m.relate(0, 1, 2);
        // 0 R 2 is missing
        assert_eq!(
            m.validate(),
            Err(FrameError::NotTransitive { agent: 0, u: 0, v: 1, w: 2 })
        );
    }

    #[test]
    fn non_locally_connected_frame_is_rejected() {
        // 0 ──► 2 ◄── 1 : 0 and 1 are linked through 2 but not comparable (§3.5).
        let mut m = Model::new(3, 1, 1);
        m.relate(0, 0, 2);
        m.relate(0, 1, 2);
        assert!(matches!(m.validate(), Err(FrameError::NotLocallyConnected { .. })));
    }
}
