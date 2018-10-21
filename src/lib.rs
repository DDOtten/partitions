//! A [disjoint-sets/union-find] implementation of a vector partitioned in sets that allows
//! for efficient iteration over the elements of a set.
//!
//! The main struct of this crate is [`PartitionVec<T>`] which has the functionality of a `Vec<T>`
//! and in addition devides the elements of this vector in sets.
//! The elements each start in their own set and these sets can be joined with the `union` method.
//! You can check if elements share a set with the `same_set` method and iterate on the elements
//! in a set with the `set` method.
//! The `union` and `same_set` methods are extremely fast and have an amortized complexity of
//! `O(α(n))` where 'α' is the inverse Ackermann function and length `n`.
//! The `α(n)` has value below 5 for any `n` that can be written in the observable universe.
//! The next element of the iterator returned by `set` is found in `O(1)` time.
//!
//! This can be used for exampte to keep track of the connected components of an undirected graph.
//! This struct can then be used to determine whether two vertices belong to the same component,
//! or whether adding an edge between them would result in a cycle.
//! The Union–Find algorithm is used in high-performance implementations of unification.
//! It is also a key component in implementing Kruskal's algorithm to find the minimum spanning
//! tree of a graph.
//!
//! For each element of a [`PartitionVec<T>`] we need to store three additional `usize` values.
//! A more compact implementation is included that has the same functionality but only needs to
//! store an additional two `usize` values.
//! This is done by using a few bits of these two values to store the third.
//! This is a feature and can be enabled by adding the following to your `Cargo.toml` file:
//! ```
//! [dependencies.partitions]
//! version = "0.1"
//! features = ["compact"]
//! ```
//!
//! [disjoint-sets/union-find]: https://en.wikipedia.org/wiki/Disjoint-set_data_structure
//! [`PartitionVec<T>`]: struct.PartitionVec.html

extern crate bit_vec;
#[cfg(feature = "rayon")]
extern crate rayon;

/// We count the amount of expresions given to this macro.
#[doc(hidden)]
#[macro_export]
macro_rules! partitions_count_expr {
    () => { 0usize };
    ($_single: expr) => { 1usize };
    // Even amount of expresions.
    ($($first: expr, $_second: expr),*) => {
        (partitions_count_expr![$($first),*] << 1usize)
    };
    // Odd amount of expresions.
    ($_single: expr, $($first: expr, $_second: expr),*) => {
        (partitions_count_expr![$($first),*] << 1usize) | 1
    };
}

/// A convenient macro to create a `BitVec` similar to `vec!`.
macro_rules! bit_vec {
    ($element: expr; $len: expr) => {
        bit_vec::BitVec::from_elem($len, $element);
    };
    ($($value: expr),*) => {
        {
            let len = partitions_count_expr![$($value),*];
            let mut bit_vec = bit_vec::BitVec::with_capacity(len);

            $(
                bit_vec.push($value);
            )*

            bit_vec
        }
    };
    ($($value: expr,)*) => {
        bit_vec![$($value),*];
    };
}

mod metadata;
pub mod partition_vec;

pub use partition_vec::PartitionVec;

/// This takes an mutable reference and return a mutable reference with a different lifetime.
///
/// This is highly unsafe and every use of this function will have a
/// comment explaining why it is necessary.
/// The main motivation for making a function for this is that the code is not
/// intuitive and this makes the intend clearer.
unsafe fn extend_mut<'a, 'b, T>(ptr: &'a mut T) -> &'b mut T {
    &mut *(ptr as *mut T)
}
