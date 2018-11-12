use std::cell::Cell;

/// This provides additional information about a given value in the `DisjointSets`.
///
/// For each value in the `DisjointSets` we store a `Metadata`.
#[cfg(not(feature = "compact"))]
#[derive(Clone, Debug, Default)]
pub(crate) struct Metadata {
    /// The parent of the value in its sets tree.
    /// These form an upside down tree where each child has the index of its parent.
    parent: Cell<usize>,
    /// A link to another index.
    /// These form a circular linked list in its subset.
    link: Cell<usize>,
    /// A maximum to the size of the tree of the set.
    rank: Cell<usize>,
}

#[cfg(not(feature = "compact"))]
impl Metadata {
    /// Create a new `Metadata` for an element with the given index.
    pub(crate) fn new(index: usize) -> Self {
        Self {
            parent: Cell::new(index),
            link: Cell::new(index),
            rank: Cell::new(0),
        }
    }

    /// Return the `parent` variable.
    pub(crate) fn parent(&self) -> usize {
        self.parent.get()
    }

    /// Set the `parent` variable.
    pub(crate) fn set_parent(&self, value: usize) {
        self.parent.set(value);
    }

    /// Return the `link` variable.
    pub(crate) fn link(&self) -> usize {
        self.link.get()
    }

    /// Set the `link` variable.
    pub(crate) fn set_link(&self, value: usize) {
        self.link.set(value);
    }

    /// Return the `rank` variable.
    pub(crate) fn rank(&self) -> usize {
        self.rank.get()
    }

    /// Set the `rank` variable.
    pub(crate) fn set_rank(&self, value: usize) {
        self.rank.set(value);
    }
}

#[cfg(feature = "compact")]
const USIZE_BITS: usize = 8 * ::std::mem::size_of::<usize>();
// The least amount of elements you need in a set to get a rank of 0 is 1.
// For a given n > 0 the least amount of elements you need to get a rank of n is
// double the least amount to get a rank of n - 1.
// This is because you need to join two sets of rank n - 1.
// With induction we see that the minimum amount of elements to get rank n is 2 ^ n.
//
// We write the amount of bytes a `usize` contains as 2 ^ B.
// For each element we store two times this amount of bytes which is 2 ^ (B + 1) bytes.
// There are 2 ^ (8 * 2 ^ B) = 2 ^ (2 ^ (3 + B)) memory addresses so a maximum for the amount of
// elements is given by 2 ^ (2 ^ (B + 3)) / 2 ^ (B + 1) = 2 ^ (2 ^ (B + 3) - B - 1).
// This means that a maximum for the rank is given by 2 ^ (B + 3) - B.
// To store this rank we need a maximum of B + 3 bits.
// Because we devide these bits over the parent and link we need a maximum of (B + 3) / 2 bits
// rounded up which is B / 2 + 2 bits rounded down.
#[cfg(all(feature = "compact", target_pointer_width = "8"))]
const RANK_BITS: usize = 2;
#[cfg(all(feature = "compact", target_pointer_width = "16"))]
const RANK_BITS: usize = 2;
#[cfg(all(feature = "compact", target_pointer_width = "32"))]
const RANK_BITS: usize = 3;
#[cfg(all(feature = "compact", target_pointer_width = "64"))]
const RANK_BITS: usize = 3;
#[cfg(all(feature = "compact", target_pointer_width = "128"))]
const RANK_BITS: usize = 4;
#[cfg(all(feature = "compact", target_pointer_width = "256"))]
const RANK_BITS: usize = 4;
// TODO: When possible replace with:
// const RANK_BITS: usize = std::mem::size_of::<usize>().trailing_zeros() as usize / 2 + 2;
#[cfg(feature = "compact")]
const MASK: usize = (1 << RANK_BITS) - 1;
#[cfg(feature = "compact")]
const MAX: usize = (1 << (USIZE_BITS - RANK_BITS)) - 2;

/// This provides additional information about a given value in the `DisjointSets`.
///
/// For each value in the `DisjointSets` we store a `Metadata`.
#[cfg(feature = "compact")]
#[derive(Clone, Debug, Default)]
pub(crate) struct Metadata {
    /// The parent of the value in its sets tree.
    /// These form an upside down tree where each child has the index of its parent.
    parent: Cell<usize>,
    /// A link to another index.
    /// These form a circular linked list in its subset.
    link: Cell<usize>,
}

#[cfg(feature = "compact")]
impl Metadata {
    /// Create a new `Metadata` for an element with the given index.
    ///
    /// # Panics
    ///
    /// Panics if the index is above the maximum amount of values a `PartitionVec<T>` can store
    /// with the compact representation.
    pub(crate) fn new(index: usize) -> Self {
        if index > MAX {
            panic!("A PartitionVec can only hold {} values.", MAX)
        }

        Self {
            parent: Cell::new(index << RANK_BITS),
            link: Cell::new(index << RANK_BITS),
        }
    }

    /// Return the `parent` variable.
    pub(crate) fn parent(&self) -> usize {
        self.parent.get() >> RANK_BITS
    }

    /// Set the `parent` variable.
    pub(crate) fn set_parent(&self, value: usize) {
        let old = self.parent.get();
        self.parent.set((old & MASK) | (value << RANK_BITS));
    }

    /// Return the `link` variable.
    pub(crate) fn link(&self) -> usize {
        self.link.get() >> RANK_BITS
    }

    /// Set the `link` variable.
    pub(crate) fn set_link(&self, value: usize) {
        let old = self.link.get();
        self.link.set((old & MASK) | (value << RANK_BITS));
    }

    /// Return the `rank` variable.
    pub(crate) fn rank(&self) -> usize {
        let left = self.link.get() & RANK_BITS;
        let right = self.parent.get() & RANK_BITS;
        (left << RANK_BITS) | right
    }

    /// Set the `rank` variable.
    pub(crate) fn set_rank(&self, value: usize) {
        let old = self.parent.get();
        self.parent.set((old & !MASK) | (value >> RANK_BITS));
        let old = self.link.get();
        self.link.set((old & !MASK) | (value & RANK_BITS));
    }
}

impl Metadata {
    pub(crate) fn is_marked(&self) -> bool {
        self.parent.get() == !0
    }

    pub(crate) unsafe fn set_marked_value(&mut self, value: usize) {
        self.parent.set(!0);
        self.link.set(value);
    }

    pub(crate) unsafe fn marked_value(&self) -> usize {
        self.link.get()
    }
}
