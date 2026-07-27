//! Interning of agent and predicate names to dense `u32` ids.

use std::collections::HashMap;

/// Maps names to dense `u32` ids and back.
#[derive(Default, Debug, Clone)]
pub struct Interner {
    names: Vec<String>,
    map: HashMap<String, u32>,
}

impl Interner {
    /// Returns the id for `s`, assigning a fresh one if unseen.
    pub fn intern(&mut self, s: &str) -> u32 {
        if let Some(&i) = self.map.get(s) {
            return i;
        }
        let i = self.names.len() as u32;
        self.names.push(s.to_owned());
        self.map.insert(s.to_owned(), i);
        i
    }

    /// The name behind an id.
    ///
    /// # Panics
    /// If `i` was not produced by this interner.
    pub fn name(&self, i: u32) -> &str {
        &self.names[i as usize]
    }

    /// How many distinct names have been interned.
    pub fn len(&self) -> usize {
        self.names.len()
    }

    /// Whether nothing has been interned yet.
    pub fn is_empty(&self) -> bool {
        self.names.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interning_is_stable_and_deduplicates() {
        let mut i = Interner::default();
        let alice = i.intern("alice");
        let bob = i.intern("bob");
        assert_ne!(alice, bob);
        assert_eq!(i.intern("alice"), alice, "re-interning must return the same id");
        assert_eq!(i.name(alice), "alice");
        assert_eq!(i.len(), 2);
    }
}
