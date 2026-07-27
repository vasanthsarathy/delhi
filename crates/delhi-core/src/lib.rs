//! Backend-agnostic interface. v0.2's cooperation-agnostic search ([T] §6.1) is
//! generic over this, so a second semantics can be added without touching the planner.
#![deny(missing_docs)]

/// What a planner needs from an epistemic state representation.
///
/// The perspective shift `sⁱ` from [T] §6.1 needs multi-pointed states and is deferred
/// to v0.2 along with the planner itself.
pub trait EpistemicState: Clone + Sized {
    /// The action representation this backend consumes.
    type Action;
    /// The formula store this backend evaluates against.
    type Store;
    /// The query language handle.
    type Formula: Copy;

    /// Whether the designated world models `f`.
    fn entails(&self, store: &Self::Store, f: Self::Formula) -> bool;
    /// Applies an action, or `None` when it is not applicable.
    fn apply(&self, store: &Self::Store, action: &Self::Action) -> Option<Self>;
    /// Quotient by the congruence-safe bisimulation.
    fn contract_dynamic(&self) -> Self;
    /// Whether two states satisfy the same formulas.
    fn equivalent(&self, other: &Self) -> bool;
    /// A canonical key for hashing.
    fn key(&self) -> Vec<u8>;
}
