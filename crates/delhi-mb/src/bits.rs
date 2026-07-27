//! A small growable bitset. Used for valuations (over atoms) and relation rows
//! (over worlds), per §5.1.

/// A fixed-capacity bitset over `0..n`.
#[derive(Clone, PartialEq, Eq, Hash, Debug, Default, PartialOrd, Ord)]
pub struct Bits(Vec<u64>);

impl Bits {
    /// A set with capacity for `n` members, all absent.
    pub fn new(n: usize) -> Self {
        Bits(vec![0; n.div_ceil(64)])
    }
    /// Whether `i` is present.
    pub fn get(&self, i: usize) -> bool {
        self.0.get(i / 64).is_some_and(|w| w >> (i % 64) & 1 == 1)
    }
    /// Adds `i`.
    ///
    /// # Panics
    /// If `i` is beyond the capacity given to [`Bits::new`].
    pub fn set(&mut self, i: usize) {
        debug_assert!(i / 64 < self.0.len(), "set: index {} out of bounds (capacity {})", i, self.0.len() * 64);
        self.0[i / 64] |= 1u64 << (i % 64);
    }
    /// Removes `i`.
    pub fn unset(&mut self, i: usize) {
        if let Some(w) = self.0.get_mut(i / 64) {
            *w &= !(1u64 << (i % 64));
        }
    }
    /// Whether nothing is present.
    pub fn is_empty(&self) -> bool {
        self.0.iter().all(|&w| w == 0)
    }
    /// How many members are present.
    pub fn count(&self) -> usize {
        self.0.iter().map(|w| w.count_ones() as usize).sum()
    }
    /// The members, ascending.
    pub fn ones(&self) -> Vec<usize> {
        let mut out = Vec::with_capacity(self.count());
        for (wi, &word) in self.0.iter().enumerate() {
            let mut w = word;
            while w != 0 {
                out.push(wi * 64 + w.trailing_zeros() as usize);
                w &= w - 1;
            }
        }
        out
    }
    /// In-place union.
    pub fn union_with(&mut self, o: &Bits) {
        for (a, b) in self.0.iter_mut().zip(o.0.iter()) {
            *a |= *b;
        }
    }
    /// In-place intersection.
    pub fn intersect_with(&mut self, o: &Bits) {
        for (a, b) in self.0.iter_mut().zip(o.0.iter()) {
            *a &= *b;
        }
    }
    /// In-place difference.
    pub fn subtract(&mut self, o: &Bits) {
        for (a, b) in self.0.iter_mut().zip(o.0.iter()) {
            *a &= !*b;
        }
    }
    /// Whether every member of `o` is present here.
    pub fn contains_all(&self, o: &Bits) -> bool {
        o.0.iter().zip(self.0.iter()).all(|(b, a)| b & !a == 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_get_and_enumerate() {
        let mut b = Bits::new(130);
        assert!(b.is_empty());
        b.set(0);
        b.set(64);
        b.set(129);
        assert!(b.get(0) && b.get(64) && b.get(129));
        assert!(!b.get(1));
        assert_eq!(b.ones(), vec![0, 64, 129]);
        assert_eq!(b.count(), 3);
    }

    #[test]
    fn set_algebra() {
        let mut a = Bits::new(8);
        a.set(1);
        a.set(2);
        let mut b = Bits::new(8);
        b.set(2);
        b.set(3);

        let mut u = a.clone();
        u.union_with(&b);
        assert_eq!(u.ones(), vec![1, 2, 3]);

        let mut i = a.clone();
        i.intersect_with(&b);
        assert_eq!(i.ones(), vec![2]);

        let mut d = a.clone();
        d.subtract(&b);
        assert_eq!(d.ones(), vec![1]);

        assert!(u.contains_all(&a));
        assert!(!a.contains_all(&u));
    }
}
