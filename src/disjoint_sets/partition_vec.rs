//! A [disjoint-sets/union-find] implementation of a vector partitioned in sets.
//!
//! See [`PartitionVec<T>`] for more information.
//!
//! [disjoint-sets/union-find]: https://en.wikipedia.org/wiki/Disjoint-set_data_structure
//! [`PartitionVec<T>`]: struct.PartitionVec.html

use {
    std::{
        ops,
        cmp::Ordering,
        iter::{
            FromIterator,
            FusedIterator,
        },
    },
    crate::{
        disjoint_sets::metadata::Metadata,
        extend_mut,
    },
};
#[cfg(feature = "rayon")]
use rayon::prelude::*;
#[cfg(feature = "proptest")]
use proptest::prelude::*;

/// A [disjoint-sets/union-find] implementation of a vector partitioned in sets.
///
/// Most methods that are defined on a `Vec` also work on a `PartitionVec`.
/// In addition to this each element stored in the `PartitionVec` is a member of a set.
/// Initially each element has its own set but sets can be joined with the `union` method.
///
/// In addition to the normal implementation we store an additional index for each element.
/// These indices form a circular linked list of the set the element is in.
/// This allows for fast iteration of the set using the `set` method
/// and is used to speed up the performance of other methods.
///
/// This implementation chooses not to expose the `find` method and instead has a `same_set` method.
/// This is so that the representative of the set stays an implementation detail which gives
/// us more freedom to change it behind the scenes for improved performance.
///
/// # Examples
///
/// ```
/// # #[macro_use]
/// # extern crate partitions;
/// #
/// # fn main() {
/// let mut partition_vec = partition_vec!['a', 'b', 'c', 'd'];
/// partition_vec.union(1, 2);
/// partition_vec.union(2, 3);
///
/// assert!(partition_vec.same_set(1, 3));
///
/// for (index, &value) in partition_vec.set(1) {
///     assert!(index >= 1);
///     assert!(index <= 3);
///     assert!(value != 'a');
/// }
/// # }
/// ```
///
/// [disjoint-sets/union-find]: https://en.wikipedia.org/wiki/Disjoint-set_data_structure
#[derive(Clone)]
pub struct PartitionVec<T> {
    /// Each index has a value.
    /// We store these in a separate `Vec` so we can easily dereference it to a slice.
    data: Vec<T>,
    /// The metadata for each value, this vec will always have the same size as `values`.
    meta: Vec<Metadata>,
}

/// Creates a [`PartitionVec`] containing the arguments.
///
/// There are tree forms of the `partition_vec!` macro:
///
/// - Create a [`PartitionVec`] containing a given list of elements all in distinct sets:
///
/// ```
/// # #[macro_use]
/// # extern crate partitions;
/// #
/// # fn main() {
/// let partition_vec = partition_vec!['a', 'b', 'c'];
///
/// assert!(partition_vec[0] == 'a');
/// assert!(partition_vec[1] == 'b');
/// assert!(partition_vec[2] == 'c');
///
/// assert!(partition_vec.is_singleton(0));
/// assert!(partition_vec.is_singleton(1));
/// assert!(partition_vec.is_singleton(2));
/// # }
/// ```
///
/// - Create a [`PartitionVec`] containing a given list of elements in the sets specified:
///
/// ```
/// # #[macro_use]
/// # extern crate partitions;
/// #
/// # fn main() {
/// let partition_vec = partition_vec![
///     'a' => 0,
///     'b' => 1,
///     'c' => 2,
///     'd' => 1,
///     'e' => 0,
/// ];
///
/// assert!(partition_vec[0] == 'a');
/// assert!(partition_vec[1] == 'b');
/// assert!(partition_vec[2] == 'c');
/// assert!(partition_vec[3] == 'd');
/// assert!(partition_vec[4] == 'e');
///
/// assert!(partition_vec.same_set(0, 4));
/// assert!(partition_vec.same_set(1, 3));
/// assert!(partition_vec.is_singleton(2));
/// # }
/// ```
///
/// You can use any identifiers that implement `Hash` and `Eq`.
/// Elements with the same set identifiers will be placed in the same set.
/// These identifiers will only be used when constructing a [`PartitionVec`]
/// and will not be stored further.
/// This means `println!("{:?}", partition_vec![3 => 'a', 1 => 'a'])` will display `[3 => 0, 1 => 0]`.
///
/// - Create a [`PartitionVec`] of distinct sets from a given element and size:
///
/// ```
/// # #[macro_use]
/// # extern crate partitions;
/// #
/// # fn main() {
/// let partition_vec = partition_vec!['a'; 3];
///
/// assert!(partition_vec[0] == 'a');
/// assert!(partition_vec[1] == 'a');
/// assert!(partition_vec[2] == 'a');
///
/// assert!(partition_vec.is_singleton(0));
/// assert!(partition_vec.is_singleton(1));
/// assert!(partition_vec.is_singleton(2));
/// # }
/// ```
///
/// [`PartitionVec`]: partition_vec/struct.PartitionVec.html
#[macro_export]
macro_rules! partition_vec {
    ($elem: expr; $len: expr) => {
        $crate::PartitionVec::from_elem($elem, $len);
    };
    ($($elem: expr),*) => {
        {
            let len = partitions_count_expr![$($elem),*];
            let mut partition_vec = $crate::PartitionVec::with_capacity(len);

            $(
                partition_vec.push($elem);
            )*

            partition_vec
        }
    };
    ($($elem: expr,)*) => {
        partition_vec![$($elem),*];
    };
    ($($elem: expr => $set: expr),*) => {
        {
            let len = partitions_count_expr![$($elem),*];
            let mut partition_vec = $crate::PartitionVec::with_capacity(len);
            let mut map = ::std::collections::HashMap::new();

            $(
                let last_index = partition_vec.len();
                partition_vec.push($elem);

                if let Some(&index) = map.get(&$set) {
                    partition_vec.union(index, last_index);
                } else {
                    map.insert($set, last_index);
                }
            )*

            partition_vec
        }
    };
    ($($elem: expr => $set: expr,)*) => {
        partition_vec![$($elem => $set),*];
    }
}

impl<T> PartitionVec<T> {
    /// Constructs a new, empty `PartitionVec<T>`.
    ///
    /// The `PartitionVec<T>` will not allocate until elements are pushed onto it.
    ///
    /// # Examples
    ///
    /// ```
    /// # #![allow(unused_mut)]
    /// use partitions::PartitionVec;
    ///
    /// let mut partition_vec: PartitionVec<()> = PartitionVec::new();
    /// ```
    #[inline]
    pub fn new() -> Self {
        Self {
            data: Vec::new(),
            meta: Vec::new(),
        }
    }

    /// Constructs a new, empty `PartitionVec<T>` with the specified capacity.
    ///
    /// The `PartitionVec<T>` will be able to hold exactly `capacity`
    /// elements without reallocating.
    /// If capacity is 0, the partition_vec will not allocate.
    ///
    /// # Examples
    ///
    /// ```
    /// use partitions::PartitionVec;
    ///
    /// let mut partition_vec = PartitionVec::with_capacity(10);
    ///
    /// assert!(partition_vec.len() == 0);
    /// assert!(partition_vec.capacity() == 10);
    ///
    /// // This can be done without reallocating.
    /// for i in 0 .. 10 {
    ///     partition_vec.push(i);
    /// }
    ///
    /// // We can add more elements but this will reallocate.
    /// partition_vec.push(11);
    /// ```
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            data: Vec::with_capacity(capacity),
            meta: Vec::with_capacity(capacity),
        }
    }

    /// Joins the sets of the `first_index` and the `second_index`.
    ///
    /// This method will be executed in `O(α(n))` time where `α` is the inverse
    /// Ackermann function. The inverse Ackermann function has value below 5
    /// for any value of `n` that can be written in the physical universe.
    ///
    /// # Panics
    ///
    /// If `first_index` or `second_index` is out of bounds.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[macro_use]
    /// # extern crate partitions;
    /// #
    /// # fn main() {
    /// let mut partition_vec = partition_vec![(); 4];
    ///
    /// // All elements start out in their own sets.
    /// assert!(partition_vec.len_of_set(0) == 1);
    /// assert!(partition_vec.len_of_set(1) == 1);
    /// assert!(partition_vec.len_of_set(2) == 1);
    /// assert!(partition_vec.len_of_set(3) == 1);
    ///
    /// partition_vec.union(1, 2);
    ///
    /// // Now 1 and 2 share a set.
    /// assert!(partition_vec.len_of_set(0) == 1);
    /// assert!(partition_vec.len_of_set(1) == 2);
    /// assert!(partition_vec.len_of_set(2) == 2);
    /// assert!(partition_vec.len_of_set(3) == 1);
    ///
    /// partition_vec.union(2, 3);
    ///
    /// // We added 3 to the existing set with 1 and 2.
    /// assert!(partition_vec.len_of_set(0) == 1);
    /// assert!(partition_vec.len_of_set(1) == 3);
    /// assert!(partition_vec.len_of_set(2) == 3);
    /// assert!(partition_vec.len_of_set(3) == 3);
    /// # }
    /// ```
    pub fn union(&mut self, first_index: usize, second_index: usize) {
        let i = self.find(first_index);
        let j = self.find(second_index);

        if i == j {
            return
        }

        // We swap the values of the links.
        let link_i = self.meta[i].link();
        let link_j = self.meta[j].link();
        self.meta[i].set_link(link_j);
        self.meta[j].set_link(link_i);

        // We add to the tree with the highest rank.
        match Ord::cmp(&self.meta[i].rank(), &self.meta[j].rank()) {
            Ordering::Less => {
                self.meta[i].set_parent(j);
            },
            Ordering::Equal => {
                // We add the first tree to the second tree.
                self.meta[i].set_parent(j);
                // The second tree becomes larger.
                self.meta[j].set_rank(self.meta[j].rank() + 1);
            },
            Ordering::Greater => {
                self.meta[j].set_parent(i);
            },
        }
    }

    /// Returns `true` if `first_index` and `second_index` are in the same set.
    ///
    /// This method will be executed in `O(α(n))` time where `α` is the inverse
    /// Ackermann function.
    ///
    /// # Panics
    ///
    /// If `first_index` or `second_index` are out of bounds.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[macro_use]
    /// # extern crate partitions;
    /// # fn main() {
    /// let mut partition_vec = partition_vec![(); 4];
    ///
    /// partition_vec.union(1, 3);
    /// partition_vec.union(0, 1);
    ///
    /// assert!(partition_vec.same_set(0, 1));
    /// assert!(!partition_vec.same_set(0, 2));
    /// assert!(partition_vec.same_set(0, 3));
    /// assert!(!partition_vec.same_set(1, 2));
    /// assert!(partition_vec.same_set(1, 3));
    /// assert!(!partition_vec.same_set(2, 3));
    /// # }
    /// ```
    #[inline]
    pub fn same_set(&self, first_index: usize, second_index: usize) -> bool {
        self.find(first_index) == self.find(second_index)
    }

    /// Returns `true` if `first_index` and `second_index` are in different sets.
    ///
    /// This method will be executed in `O(α(n))` time where `α` is the inverse
    /// Ackermann function.
    ///
    /// # Panics
    ///
    /// If `first_index` or `second_index` are out of bounds.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[macro_use]
    /// # extern crate partitions;
    /// # fn main() {
    /// let mut partition_vec = partition_vec![(); 4];
    ///
    /// partition_vec.union(1, 3);
    /// partition_vec.union(0, 1);
    ///
    /// assert!(!partition_vec.other_sets(0, 1));
    /// assert!(partition_vec.other_sets(0, 2));
    /// assert!(!partition_vec.other_sets(0, 3));
    /// assert!(partition_vec.other_sets(1, 2));
    /// assert!(!partition_vec.other_sets(1, 3));
    /// assert!(partition_vec.other_sets(2, 3));
    /// # }
    /// ```
    #[inline]
    pub fn other_sets(&self, first_index: usize, second_index: usize) -> bool {
        self.find(first_index) != self.find(second_index)
    }

    /// Will remove `index` from its set while leaving the other members in it.
    ///
    /// After this `index` will be the only element of its set.
    /// This won't change the `PartitionVec<T>` if `index` is already the only element.
    /// This method will be executed in `O(m)` time where `m` is the size of the set of `index`.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[macro_use]
    /// # extern crate partitions;
    /// #
    /// # fn main() {
    /// let mut partition_vec = partition_vec![
    ///     () => 'a',
    ///     () => 'a',
    ///     () => 'a',
    ///     () => 'b',
    /// ];
    ///
    /// // 0, 1, and 2 share a set.
    /// assert!(partition_vec.len_of_set(0) == 3);
    /// assert!(partition_vec.len_of_set(1) == 3);
    /// assert!(partition_vec.len_of_set(2) == 3);
    /// assert!(partition_vec.len_of_set(3) == 1);
    ///
    /// partition_vec.make_singleton(2);
    ///
    /// // Now 2 has its own set and 1, and 2 still share a set.
    /// assert!(partition_vec.len_of_set(0) == 2);
    /// assert!(partition_vec.len_of_set(1) == 2);
    /// assert!(partition_vec.len_of_set(2) == 1);
    /// assert!(partition_vec.len_of_set(3) == 1);
    /// # }
    /// ```
    pub fn make_singleton(&mut self, index: usize) {
        let mut current = self.meta[index].link();

        if current != index {
            // We make this the new root.
            let root = current;
            self.meta[root].set_rank(1);

            // All parents except for the last are updated.
            while self.meta[current].link() != index {
                self.meta[current].set_parent(root);

                current = self.meta[current].link();
            }

            // We change the last parent and link.
            self.meta[current].set_parent(root);
            self.meta[current].set_link(root);
        }

        self.meta[index] = Metadata::new(index);
    }

    /// Returns `true` if `index` is the only element of its set.
    ///
    /// This will be done in `O(1)` time.
    ///
    /// # Panics
    ///
    /// If `index` is out of bounds.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[macro_use]
    /// # extern crate partitions;
    /// #
    /// # fn main() {
    /// let mut partition_vec = partition_vec![(); 4];
    ///
    /// partition_vec.union(1, 3);
    ///
    /// assert!(partition_vec.is_singleton(0));
    /// assert!(!partition_vec.is_singleton(1));
    /// assert!(partition_vec.is_singleton(2));
    /// assert!(!partition_vec.is_singleton(3));
    /// # }
    /// ```
    #[inline]
    pub fn is_singleton(&self, index: usize) -> bool {
        self.meta[index].link() == index
    }

    /// Returns the amount of elements in the set that `index` belongs to.
    ///
    /// This will be done in `O(m)` time where `m` is the size of the set that `index` belongs to.
    ///
    /// # Panics
    ///
    /// If `index` is out of bounds.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[macro_use]
    /// # extern crate partitions;
    /// #
    /// # fn main() {
    /// let mut partition_vec = partition_vec![true; 3];
    ///
    /// assert!(partition_vec.len_of_set(0) == 1);
    /// assert!(partition_vec.len_of_set(1) == 1);
    /// assert!(partition_vec.len_of_set(2) == 1);
    ///
    /// partition_vec.union(0, 2);
    ///
    /// assert!(partition_vec.len_of_set(0) == 2);
    /// assert!(partition_vec.len_of_set(1) == 1);
    /// assert!(partition_vec.len_of_set(2) == 2);
    /// # }
    /// ```
    pub fn len_of_set(&self, index: usize) -> usize {
        let mut current = self.meta[index].link();
        let mut count = 1;

        while current != index {
            current = self.meta[current].link();
            count += 1;
        }

        count
    }

    /// Returns the amount of sets in the `PartitionVec<T>`.
    ///
    /// This method will be executed in `O(n α(n))` where `α` is the inverse Ackermann function.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[macro_use]
    /// # extern crate partitions;
    /// #
    /// # fn main() {
    /// let partition_vec = partition_vec![
    ///     8 => 0,
    ///     3 => 1,
    ///     4 => 0,
    ///     3 => 1,
    ///     7 => 2,
    /// ];
    ///
    /// assert!(partition_vec.amount_of_sets() == 3);
    /// # }
    /// ```
    pub fn amount_of_sets(&self) -> usize {
        let mut done = bit_vec![false; self.len()];
        let mut count = 0;

        for i in 0 .. self.len() {
            if !done.get(self.find(i)).unwrap() {
                done.set(self.find(i), true);
                count += 1;
            }
        }

        count
    }

    /// Gives the representative of the set that `index` belongs to.
    ///
    /// This method will be executed in `O(α(n))` time where `α` is the inverse
    /// Ackermann function. Each index of a set
    /// will give the same value. To see if two indexes point to values in
    /// the same subset compare the results of `find`.
    ///
    /// This method is private to keep the representative of the set an implementation
    /// detail, this gives greater freedom to change the representative of the set.
    ///
    /// # Panics
    ///
    /// If `index` is out of bounds.
    pub(crate) fn find(&self, index: usize) -> usize {
        // If the node is its own parent we have found the root.
        if self.meta[index].parent() == index {
            index
        } else {
            // This method is recursive so each parent on the way to the root is updated.
            let root = self.find(self.meta[index].parent());

            // We update the parent to the root for a lower tree.
            self.meta[index].set_parent(root);

            root
        }
    }

    /// Gives the representative of the set that `index` belongs to.
    ///
    /// This method is slightly faster than `find` but still `O(a(n))` time.
    /// This method wont update the parents while finding the representative and should
    /// only be used if the parents will be updated immediately afterwards.
    ///
    /// # Panics
    ///
    /// If `index` is out of bounds.
    #[inline]
    pub(crate) fn find_final(&self, mut index: usize) -> usize {
        while index != self.meta[index].parent() {
            index = self.meta[index].parent();
        }

        index
    }

    /// Returns the number of elements the `PartitionVec<T>` can hold without reallocating.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut partition_vec = partitions::PartitionVec::with_capacity(6);
    ///
    /// for i in 0 .. 6 {
    ///     partition_vec.push(i);
    /// }
    ///
    /// assert!(partition_vec.capacity() == 6);
    ///
    /// partition_vec.push(6);
    ///
    /// assert!(partition_vec.capacity() >= 7);
    /// ```
    #[inline]
    pub fn capacity(&self) -> usize {
        usize::min(self.data.capacity(), self.meta.capacity())
    }

    /// Appends an element to the back of the `PartitionVec<T>`.
    ///
    /// This element has its own disjoint set.
    ///
    /// # Panics
    ///
    /// Panics if the number of elements in the `PartitionVec<T>` overflows a `usize`.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[macro_use]
    /// # extern crate partitions;
    /// #
    /// # fn main() {
    /// let mut partition_vec = partition_vec![
    ///     'a' => 0,
    ///     'b' => 0,
    ///     'c' => 1,
    ///     'd' => 2,
    /// ];
    ///
    /// partition_vec.push('e');
    ///
    /// assert!(partition_vec.amount_of_sets() == 4);
    /// assert!(partition_vec[4] == 'e');
    /// # }
    /// ```
    #[inline]
    pub fn push(&mut self, elem: T) {
        let old_len = self.len();

        self.data.push(elem);
        self.meta.push(Metadata::new(old_len));
    }

    /// Removes the last element returns it, or `None` if it is empty.
    ///
    /// This will be done in `O(m)` time where `m` is the size of the set
    /// that `index` belongs to.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[macro_use]
    /// # extern crate partitions;
    /// #
    /// # fn main() {
    /// let mut partition_vec = partition_vec![
    ///     'a' => 0,
    ///     'b' => 0,
    ///     'c' => 1,
    ///     'd' => 0,
    /// ];
    ///
    /// assert!(partition_vec.pop() == Some('d'));
    ///
    /// assert!(partition_vec.amount_of_sets() == 2);
    /// assert!(partition_vec.len() == 3);
    /// # }
    /// ```
    pub fn pop(&mut self) -> Option<T> {
        let last_index = self.data.len() - 1;
        self.make_singleton(last_index);

        self.meta.pop()?;
        Some(self.data.pop().unwrap())
    }

    /// Inserts an element at `index` within the `PartitionVec<T>`, shifting all
    /// elements after it to the right.
    ///
    /// This will take `O(n)` time.
    ///
    /// # Panics
    ///
    /// Panics if `index` is out of bounds.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[macro_use]
    /// # extern crate partitions;
    /// #
    /// # fn main() {
    /// let mut partition_vec = partition_vec![
    ///     0 => 0,
    ///     1 => 1,
    ///     2 => 0,
    ///     3 => 2,
    /// ];
    ///
    /// partition_vec.insert(2, -1);
    ///
    /// assert!(partition_vec[2] == -1);
    /// assert!(partition_vec.amount_of_sets() == 4);
    /// # }
    /// ```
    pub fn insert(&mut self, index: usize, elem: T) {
        // We update the parents and links above the new value.
        for i in 0 .. self.meta.len() {
            let parent = self.meta[i].parent();
            if parent >= index {
                self.meta[i].set_parent(parent + 1);
            }

            let link = self.meta[i].link();
            if link >= index {
                self.meta[i].set_link(link + 1);
            }
        }

        self.data.insert(index, elem);
        self.meta.insert(index, Metadata::new(index));
    }

    /// Removes and returns the element at position index within the `PartitionVec<T>`,
    /// shifting all elements after it to the left.
    ///
    /// This will take `O(n + m)` time where `m` is the size of the set that `index` belongs to.
    ///
    /// # Panics
    ///
    /// Panics if `index` is out of bounds.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[macro_use]
    /// # extern crate partitions;
    /// #
    /// # fn main() {
    /// let mut partition_vec = partition_vec![
    ///     0 => 0,
    ///     1 => 1,
    ///     2 => 0,
    ///     3 => 2,
    /// ];
    ///
    /// assert!(partition_vec.remove(2) == 2);
    ///
    /// assert!(partition_vec[2] == 3);
    /// assert!(partition_vec.amount_of_sets() == 3);
    /// # }
    /// ```
    pub fn remove(&mut self, index: usize) -> T {
        self.make_singleton(index);

        self.meta.remove(index);

        // We lower all values that point above the index.
        for i in 0 .. self.meta.len() {
            let parent = self.meta[i].parent();
            if parent > index {
                self.meta[i].set_parent(parent - 1);
            }

            let link = self.meta[i].link();
            if link > index {
                self.meta[i].set_link(link - 1);
            }
        }

        self.data.remove(index)
    }

    /// Moves all the elements of `other` into `self`, leaving `other` empty.
    ///
    /// # Panics
    ///
    /// Panics if the number of elements in de `PartitionVec<T>` overflows a `usize`.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[macro_use]
    /// # extern crate partitions;
    /// #
    /// # fn main() {
    /// let mut first = partition_vec![
    ///     'a' => 0,
    ///     'b' => 1,
    ///     'c' => 1,
    /// ];
    /// let mut second = partition_vec![
    ///     'a' => 0,
    ///     'b' => 0,
    ///     'c' => 1,
    /// ];
    ///
    /// first.append(&mut second);
    ///
    /// assert!(first.len() == 6);
    /// assert!(second.len() == 0);
    ///
    /// assert!(first.amount_of_sets() == 4);
    /// assert!(second.amount_of_sets() == 0);
    /// # }
    /// ```
    pub fn append(&mut self, other: &mut Self) {
        let old_len = self.len();
        self.data.append(&mut other.data);
        self.meta.extend(other.meta.drain(..).map(|meta| {
            let old_parent = meta.parent();
            meta.set_parent(old_parent + old_len);
            let old_link = meta.link();
            meta.set_link(old_link + old_len);

            meta
        }));
    }

    /// Reserves capacity for at least `additional` more elements to be
    /// inserted in the given `PartitionVec<T>`.
    /// The collection may reserve more space to avoid frequent reallocation's.
    /// After calling `reserve`, capacity will be greater than
    /// or equal to `self.len() + additional`.
    /// Does nothing if capacity is already sufficient.
    ///
    /// # Panics
    ///
    /// Panics if the new capacity overflows a `usize`.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[macro_use]
    /// # extern crate partitions;
    /// #
    /// # fn main() {
    /// let mut partition_vec = partition_vec![1];
    /// partition_vec.reserve(10);
    /// assert!(partition_vec.capacity() >= 11);
    /// # }
    /// ```
    #[inline]
    pub fn reserve(&mut self, additional: usize) {
        self.data.reserve(additional);
        self.meta.reserve(additional);
    }

    /// Reserves the minimum capacity for exactly  `additional` more elements to be
    /// inserted in the given `PartitionVec<T>`.
    /// After calling `reserve_exact`, capacity will be greater than or
    /// equal to `self.len() + additional`.
    /// Does nothing if the capacity is already sufficient.
    ///
    /// Note that the allocator may give the collection more space than it requests.
    /// Therefore capacity can not be relied upon to be precisely minimal.
    /// Prefer `reserve` if future insertions are expected.
    ///
    /// # Panics
    ///
    /// Panics if the new capacity overflows a `usize`.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[macro_use]
    /// # extern crate partitions;
    /// #
    /// # fn main() {
    /// let mut partition_vec = partition_vec![1];
    /// partition_vec.reserve_exact(10);
    /// assert!(partition_vec.capacity() >= 11);
    /// # }
    /// ```
    #[inline]
    pub fn reserve_exact(&mut self, additional: usize) {
        self.data.reserve_exact(additional);
        self.meta.reserve_exact(additional);
    }

    /// Shrinks the capacity of the `PartitionVec<T>` as much as possible.
    ///
    /// It will drop down as close as possible to the length but the allocator
    /// may still inform the `PartitionVec<T>` that there is space for a few more
    /// elements.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut partition_vec = partitions::PartitionVec::with_capacity(10);
    ///
    /// partition_vec.extend([1, 2, 3].iter().cloned());
    ///
    /// assert!(partition_vec.capacity() == 10);
    ///
    /// partition_vec.shrink_to_fit();
    ///
    /// assert!(partition_vec.capacity() >= 3);
    /// ```
    #[inline]
    pub fn shrink_to_fit(&mut self) {
        self.data.shrink_to_fit();
        self.meta.shrink_to_fit();
    }

    /// Shortens the `PartitionVec<T>`, keeping the first `new_len` elements and
    /// dropping the rest.
    ///
    /// If `new_len` is greater than or equal to the collections current length,
    /// this has no effect.
    ///
    /// Note that this method has no effect on the allocated capacity of the
    /// collection.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[macro_use]
    /// # extern crate partitions;
    /// #
    /// # fn main() {
    /// let mut partition_vec = partition_vec![
    ///     'a' => 0,
    ///     'b' => 1,
    ///     'c' => 0,
    ///     'd' => 1,
    ///     'e' => 2,
    /// ];
    ///
    /// partition_vec.truncate(3);
    /// assert!(partition_vec.len() == 3);
    /// assert!(partition_vec.capacity() == 5);
    /// assert!(partition_vec.len_of_set(0) == 2);
    /// assert!(partition_vec.len_of_set(1) == 1);
    /// assert!(partition_vec.len_of_set(2) == 2);
    /// # }
    /// ```
    pub fn truncate(&mut self, new_len: usize) {
        if new_len >= self.len() {
            return
        }

        for i in 0 .. new_len {
            let parent = self.meta[i].parent();
            let mut current = self.meta[i].link();
            if parent >= new_len {
                // We make `i` the new root.
                self.meta[i].set_parent(i);
                self.meta[i].set_rank(1);

                let mut previous = i;
                // The last index we saw before we went out of the new bounds.
                let mut index_before_oob = if current >= new_len {
                    Some(previous)
                } else {
                    None
                };

                while current != i {
                    if current >= new_len {
                        // If the current is above the new length we update this value if needed.
                        if index_before_oob.is_none() {
                            index_before_oob = Some(previous);
                        }
                    } else if let Some(index) = index_before_oob {
                        // If we are back in bounds for the first time we update the link.
                        self.meta[index].set_link(current);
                        index_before_oob = None;
                    }

                    self.meta[current].set_parent(i);

                    previous = current;
                    current = self.meta[current].link();
                }

                if let Some(index) = index_before_oob {
                    self.meta[index].set_link(i);
                }
            } else if current >= new_len {
                while current >= new_len {
                    current = self.meta[current].link();
                }
                self.meta[i].set_link(current);
            }
        }

        self.data.truncate(new_len);
        self.meta.truncate(new_len);
    }

    /// Resizes the `PartitionVec<T>` in-place so that `len` is equal to `new_len`.
    ///
    /// If `new_len` is greater than `len`, the collection is extended by the
    /// difference, with each additional slot filled with `value`.
    /// If `new_len` is less than `len`, the collection is simply truncated.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[macro_use]
    /// # extern crate partitions;
    /// #
    /// # fn main() {
    /// let mut partition_vec = partition_vec![4, 9];
    /// partition_vec.resize(4, 0);
    /// assert!(partition_vec.as_slice() == &[4, 9, 0, 0]);
    ///
    /// let mut partition_vec = partition_vec![
    ///     4 => 0,
    ///     1 => 1,
    ///     3 => 5,
    ///     1 => 1,
    ///     1 => 3,
    /// ];
    /// partition_vec.resize(2, 0);
    /// assert!(partition_vec.as_slice() == &[4, 1]);
    /// # }
    /// ```
    #[inline]
    pub fn resize(&mut self, new_len: usize, value: T) where T: Clone {
        let len = self.len();
        match Ord::cmp(&new_len, &len) {
            Ordering::Less => self.truncate(new_len),
            Ordering::Equal => {},
            Ordering::Greater => {
                self.data.append(&mut vec![value; new_len - len]);
                self.meta.extend((len .. new_len).map(Metadata::new));
            }
        }
    }

    /// Clears the `PartitionVec<T>`, removing all values.
    ///
    /// Note that this method has no effect on the allocated capacity of the collection.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[macro_use]
    /// # extern crate partitions;
    /// #
    /// # fn main() {
    /// let mut partition_vec = partition_vec![2, 3, 4];
    /// assert!(!partition_vec.is_empty());
    /// partition_vec.clear();
    /// assert!(partition_vec.is_empty());
    /// # }
    /// ```
    #[inline]
    pub fn clear(&mut self) {
        self.data.clear();
        self.meta.clear();
    }

    /// Returns `true` if the partition_vec contains no elements.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut partition_vec = partitions::PartitionVec::new();
    /// assert!(partition_vec.is_empty());
    ///
    /// partition_vec.push(1);
    /// assert!(!partition_vec.is_empty());
    /// ```
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Converts the `PartitionVec<T>` into `Box<[T]>`.
    ///
    /// Note that this will drop any excess capacity.
    /// This will not take the sets of the `PartitionVec<T>` in to account at all.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut partition_vec = partitions::PartitionVec::with_capacity(10);
    /// partition_vec.extend([1, 2, 3].iter().cloned());
    ///
    /// assert!(partition_vec.capacity() == 10);
    /// let slice = partition_vec.into_boxed_slice();
    /// assert!(slice.into_vec().capacity() == 3);
    /// ```
    #[inline]
    pub fn into_boxed_slice(self) -> Box<[T]> {
        self.data.into_boxed_slice()
    }

    /// Extracts a slice containing the entire `PartitionVec<T>`.
    ///
    /// Equivalent to `&partition_vec[..]`.
    /// This will not take the sets of the `PartitionVec<T>` in to account at all.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[macro_use]
    /// # extern crate partitions;
    /// #
    /// # fn main() {
    /// use std::io::{self, Write};
    /// let buffer = partition_vec![1, 2, 3, 4, 5];
    /// io::sink().write(buffer.as_slice()).unwrap();
    /// # }
    /// ```
    #[inline]
    pub fn as_slice(&self) -> & [T] {
        self.data.as_slice()
    }

    /// Extracts a mutable slice containing the entire `PartitionVec<T>`.
    ///
    /// Equivalent to `&mut partition_vec[..]`.
    /// This will not take the sets of the `PartitionVec<T>` in to account at all.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[macro_use]
    /// # extern crate partitions;
    /// #
    /// # fn main() {
    /// use std::io::{self, Read};
    /// let mut buffer = partition_vec![0; 3];
    /// io::repeat(0b101).read_exact(buffer.as_mut_slice()).unwrap();
    /// # }
    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        self.data.as_mut_slice()
    }

    /// Returns an iterator over the elements of the set that `index` belongs to.
    ///
    /// The iterator returned yields pairs `(i, &value)` where `i` is the index of the value and
    /// `value` is the value itself.
    ///
    /// The order the elements are returned in is not specified.
    ///
    /// # Panics
    ///
    /// If `index` is out of bounds.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[macro_use]
    /// # extern crate partitions;
    /// #
    /// # fn main() {
    /// let partition_vec = partition_vec![
    ///     'a' => "first set",
    ///     'b' => "first set",
    ///     'c' => "second set",
    ///     'd' => "second set",
    /// ];
    ///
    /// let mut done = [0, 0, 0, 0];
    /// for (index, value) in partition_vec.set(0) {
    ///     assert!(*value == 'a' || *value == 'b');
    ///     done[index] += 1;
    /// }
    /// for (index, value) in partition_vec.set(1) {
    ///     assert!(*value == 'a' || *value == 'b');
    ///     done[index] += 1;
    /// }
    /// for (index, value) in partition_vec.set(2) {
    ///     assert!(*value == 'c' || *value == 'd');
    ///     done[index] += 1;
    /// }
    /// // We visited the first set twice and the second set once.
    /// assert!(done == [2, 2, 1, 1]);
    /// # }
    /// ```
    #[inline]
    pub fn set(&self, index: usize) -> Set<T> {
        let root = self.find_final(index);

        self.meta[root].set_rank(1);

        Set {
            partition_vec: self,
            current: Some(root),
            root,
        }
    }

    /// Returns an iterator over the elements of the set that `index` belongs to.
    ///
    /// The iterator returned yields pairs `(i, &mut value)` where `i` is the index of the value and
    /// `value` is the value itself.
    ///
    /// The order the elements are returned in is not specified.
    ///
    /// # Panics
    ///
    /// If `index` is out of bounds.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[macro_use]
    /// # extern crate partitions;
    /// #
    /// # fn main() {
    /// let mut partition_vec = partition_vec![
    ///     0 => 'a',
    ///     0 => 'b',
    ///     0 => 'b',
    ///     0 => 'c',
    /// ];
    ///
    /// assert!(partition_vec.as_slice() == &[0, 0, 0, 0]);
    /// for (index, value) in partition_vec.set_mut(2) {
    ///     assert!(index == 1 || index == 2);
    ///     *value += 1;
    /// }
    /// assert!(partition_vec.as_slice() == &[0, 1, 1, 0]);
    /// # }
    /// ```
    #[inline]
    pub fn set_mut(&mut self, index: usize) -> SetMut<T> {
        let root = self.find_final(index);

        self.meta[root].set_rank(1);

        SetMut {
            partition_vec: self,
            current: Some(root),
            root,
        }
    }

    /// Returns an iterator over all sets of the `PartitionVec<T>`.
    ///
    /// The iterator returned yields `Set` iterators.
    /// These `Set` iterators yield pairs `(i, &value)` where `i` is the index of
    /// the value and `value` is the value itself.
    ///
    /// The sets are returned in order by there first member.
    /// The order the elements of a `Set` are returned in is not specified.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[macro_use]
    /// # extern crate partitions;
    /// #
    /// # fn main() {
    /// let partition_vec = partition_vec![
    ///     0 => 'a',
    ///     0 => 'a',
    ///     2 => 'b',
    ///     2 => 'b',
    ///     4 => 'c',
    ///     4 => 'c',
    /// ];
    ///
    /// for set in partition_vec.all_sets() {
    ///     let mut count = 0;
    ///     for (index, value) in set {
    ///         assert!(index == *value || index == *value + 1);
    ///         count += 1;
    ///     }
    ///     assert!(count == 2);
    /// }
    /// # }
    /// ```
    #[inline]
    pub fn all_sets(&self) -> AllSets<T> {
        let len = self.len();

        AllSets {
            partition_vec: self,
            done: bit_vec![false; len],
            range: 0 .. len,
        }
    }

    /// Returns an iterator over all sets of the `PartitionVec<T>`.
    ///
    /// The iterator returned yields `SetMut` iterators.
    /// These `SetMut` iterators yield pairs `(i, &mut value)` where `i` is the index of
    /// the value and `value` is the value itself.
    ///
    /// The sets are returned in order by there first member.
    /// The order the elements of a `SetMut` are returned in is not specified.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[macro_use]
    /// # extern crate partitions;
    /// #
    /// # fn main() {
    /// let mut partition_vec = partition_vec![
    ///     0 => 'a',
    ///     0 => 'b',
    ///     0 => 'a',
    ///     0 => 'b',
    ///     0 => 'c',
    ///     0 => 'c',
    /// ];
    ///
    /// assert!(partition_vec.as_slice() == &[0, 0, 0, 0, 0, 0]);
    ///
    /// for (set_number, set_mut) in partition_vec.all_sets_mut().enumerate() {
    ///     for (index, value) in set_mut {
    ///         assert!(index < 6);
    ///         *value = set_number;
    ///     }
    /// }
    ///
    /// assert!(partition_vec.as_slice() == &[0, 1, 0, 1, 2, 2]);
    /// # }
    /// ```
    #[inline]
    pub fn all_sets_mut(&mut self) -> AllSetsMut<T> {
        let len = self.len();

        AllSetsMut {
            partition_vec: self,
            done: bit_vec![false; len],
            range: 0 .. len,
        }
    }

    /// This method is used by the `partition_vec!` macro.
    #[doc(hidden)]
    #[inline]
    pub fn from_elem(elem: T, len: usize) -> Self where T: Clone {
        Self {
            data: vec![elem; len],
            meta: (0 .. len).map(Metadata::new).collect(),
        }
    }

    pub(crate) unsafe fn set_len(&mut self, len: usize) {
        self.data.set_len(len);
        self.meta.set_len(len);
    }

    #[inline]
    pub(crate) unsafe fn lazy_insert(&mut self, index: usize, value: T) -> usize {
        let marked_value = self.meta[index].marked_value();

        std::ptr::write(&mut self.data[index], value);
        self.meta[index] = Metadata::new(index);

        marked_value
    }

    #[inline]
    pub(crate) unsafe fn lazy_remove(&mut self, index: usize, marked_value: usize) -> T {
        self.make_singleton(index);

        let value = std::ptr::read(&self.data[index]);
        self.meta[index].set_marked_value(marked_value);

        value
    }

    #[inline]
    pub(crate) fn lazy_clear(&mut self) {
        for i in 0 .. self.len() {
            if !self.meta[i].is_marked() {
                unsafe { drop(std::ptr::read(&self.data[i])); }
            }
        }

        unsafe {
            self.data.set_len(0);
            self.meta.set_len(0);
        }
    }
}

impl<T> Default for PartitionVec<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> std::fmt::Debug for PartitionVec<T> where T: std::fmt::Debug {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        // We map the roots to `usize` names.
        let mut map = std::collections::HashMap::with_capacity(self.len());
        let mut builder = formatter.debug_list();
        let mut names = 0;

        for i in 0 .. self.len() {
            let root = self.find(i);

            let name = if let Some(&name) = map.get(&root) {
                // If we already have a name we use it.
                name
            } else {
                // If we don't we make a new name.
                let new_name = names;
                map.insert(root, new_name);
                names += 1;

                new_name
            };

            builder.entry(&format_args!("{:?} => {}", self.data[i], name));
        }

        builder.finish()
    }
}

impl<T> PartialEq for PartitionVec<T> where T: PartialEq {
    fn eq(&self, other: &Self) -> bool {
        if self.len() != other.len() {
            return false
        }

        // We map the roots of self to the roots of other.
        let mut map = std::collections::HashMap::with_capacity(self.len());

        for i in 0 .. self.len() {
            if self.data[i] != other.data[i] {
                return false
            }

            let self_root = self.find(i);
            let other_root = other.find(i);

            if let Some(&root) = map.get(&self_root) {
                // If we have seen this root we check if we have the same map.
                if root != other_root {
                    return false
                }
            } else {
                // If we have not seen this root we add the relation to the map.
                map.insert(self_root, other_root);
            }
        }

        true
    }
}

impl<T> Eq for PartitionVec<T> where T: Eq {}

impl<T, I> ops::Index<I> for PartitionVec<T> where I: std::slice::SliceIndex<[T]> {
    type Output = I::Output;

    #[inline]
    fn index(&self, index: I) -> &I::Output {
        (**self).index(index)
    }
}

impl<T, I> ops::IndexMut<I> for PartitionVec<T> where I: std::slice::SliceIndex<[T]> {
    #[inline]
    fn index_mut(&mut self, index: I) -> &mut I::Output {
        (**self).index_mut(index)
    }
}

impl<T> ops::Deref for PartitionVec<T> {
    type Target = [T];

    fn deref(&self) -> &[T] {
        &self.data
    }
}

impl<T> ops::DerefMut for PartitionVec<T> {
    fn deref_mut(&mut self) -> &mut [T] {
        &mut self.data
    }
}

impl<T> From<Vec<T>> for PartitionVec<T> {
    fn from(vec: Vec<T>) -> Self {
        let len = vec.len();

        Self {
            data: vec,
            meta: (0 .. len).map(Metadata::new).collect(),
        }
    }
}

impl<T> FromIterator<T> for PartitionVec<T> {
    fn from_iter<I>(iter: I) -> Self where I: IntoIterator<Item = T> {
        let data = Vec::from_iter(iter);
        let len = data.len();

        Self {
            data,
            meta: (0 .. len).map(Metadata::new).collect(),
        }
    }
}

impl<'a, T> FromIterator<&'a T> for PartitionVec<T> where T: Copy + 'a {
    fn from_iter<I>(iter: I) -> Self where I: IntoIterator<Item = &'a T> {
        Self::from_iter(iter.into_iter().cloned())
    }
}

#[cfg(feature = "rayon")]
impl<T> FromParallelIterator<T> for PartitionVec<T> where T: Send {
    fn from_par_iter<I>(par_iter: I) -> Self where I: IntoParallelIterator<Item = T> {
        let par_iter = par_iter.into_par_iter();

        let mut partition = if let Some(len) = par_iter.opt_len() {
            Self::with_capacity(len)
        } else {
            Self::new()
        };

        partition.par_extend(par_iter);

        partition
    }
}

#[cfg(feature = "rayon")]
impl<'a, T> FromParallelIterator<&'a T> for PartitionVec<T> where T: Copy+ Send + Sync + 'a {
    fn from_par_iter<I>(par_iter: I) -> Self where I: IntoParallelIterator<Item = &'a T> {
        Self::from_par_iter(par_iter.into_par_iter().cloned())
    }
}

impl<T> IntoIterator for PartitionVec<T> {
    type Item = T;
    type IntoIter = std::vec::IntoIter<T>;

    fn into_iter(self) -> std::vec::IntoIter<T> {
        self.data.into_iter()
    }
}

impl<'a, T> IntoIterator for &'a PartitionVec<T> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;

    fn into_iter(self) -> std::slice::Iter<'a, T> {
        self.data.iter()
    }
}

impl<'a, T> IntoIterator for &'a mut PartitionVec<T> {
    type Item = &'a mut T;
    type IntoIter = std::slice::IterMut<'a, T>;

    fn into_iter(self) -> std::slice::IterMut<'a, T> {
        self.data.iter_mut()
    }
}

#[cfg(feature = "rayon")]
impl<T> IntoParallelIterator for PartitionVec<T> where T: Send {
    type Item = T;
    type Iter = rayon::vec::IntoIter<T>;

    fn into_par_iter(self) -> Self::Iter {
        self.data.into_par_iter()
    }
}

#[cfg(feature = "rayon")]
impl<'a, T> IntoParallelIterator for &'a PartitionVec<T> where T: Send + Sync {
    type Item = &'a T;
    type Iter = rayon::slice::Iter<'a, T>;

    fn into_par_iter(self) -> Self::Iter {
        self.data.par_iter()
    }
}

#[cfg(feature = "rayon")]
impl<'a, T> IntoParallelIterator for &'a mut PartitionVec<T> where T: Send + Sync {
    type Item = &'a mut T;
    type Iter = rayon::slice::IterMut<'a, T>;

    fn into_par_iter(self) -> Self::Iter {
        self.data.par_iter_mut()
    }
}

impl<T> Extend<T> for PartitionVec<T> {
    fn extend<I>(&mut self, iter: I) where I: IntoIterator<Item = T> {
        let len = self.len();
        self.data.extend(iter);
        let new_len = self.data.len();

        self.meta.extend((len .. new_len).map(Metadata::new));
    }
}

impl<'a, T> Extend<&'a T> for PartitionVec<T> where T: Copy + 'a {
    fn extend<I>(&mut self, iter: I) where I: IntoIterator<Item = &'a T> {
        let len = self.len();
        self.data.extend(iter);
        let new_len = self.data.len();

        self.meta.extend((len .. new_len).map(Metadata::new));
    }
}

#[cfg(feature = "rayon")]
impl<T> ParallelExtend<T> for PartitionVec<T> where T: Send {
    fn par_extend<I>(&mut self, par_iter: I) where I: IntoParallelIterator<Item = T>
    {
        let par_iter = par_iter.into_par_iter();

        self.data.par_extend(par_iter);
        self.meta.par_extend((0 .. self.data.len()).into_par_iter().map(Metadata::new));
    }
}

#[cfg(feature = "rayon")]
impl<'a, T> ParallelExtend<&'a T> for PartitionVec<T> where T: Copy + Send + Sync + 'a {
    fn par_extend<I>(&mut self, par_iter: I) where I: IntoParallelIterator<Item = &'a T> {
        self.par_extend(par_iter.into_par_iter().cloned())
    }
}

#[cfg(feature = "proptest")]
impl<T> Arbitrary for PartitionVec<T> where
    T: Arbitrary,
    T::Strategy: 'static,
{
    type Parameters = (proptest::collection::SizeRange, T::Parameters);
    type Strategy = BoxedStrategy<Self>;

    fn arbitrary_with(params: Self::Parameters) -> Self::Strategy {
        use std::collections::hash_map;

        let (size_range, params) = params;
        let params = (size_range, (params, ()));

        (Vec::<(T, usize)>::arbitrary_with(params)).prop_map(|vec| {
            let mut partition_vec = Self::with_capacity(vec.len());

            // We map a `set_number` to an `index` of that set.
            let mut map = hash_map::HashMap::with_capacity(vec.len());

            for (index, (value, mut set_number)) in vec.into_iter().enumerate() {
                partition_vec.push(value);

                let set_number = set_number.trailing_zeros();

                match map.entry(set_number) {
                    hash_map::Entry::Occupied(occupied) => {
                        partition_vec.union(index, *occupied.get());
                    },
                    hash_map::Entry::Vacant(vacant) => {
                        vacant.insert(index);
                    }
                }
            }

            partition_vec
        }).boxed()
    }
}

/// An iterator over a set in a `PartitionVec<T>`.
///
/// This struct is created by the [`set`] method on [`PartitionVec<T>`].
/// See its documentation for more.
///
/// [`set`]: struct.PartitionVec.html#method.set
/// [`PartitionVec<T>`]: struct.PartitionVec.html
#[derive(Clone, Debug)]
pub struct Set<'a, T: 'a> {
    partition_vec: &'a PartitionVec<T>,
    current: Option<usize>,
    root: usize,
}

impl<'a, T> Iterator for Set<'a, T> {
    type Item = (usize, &'a T);

    fn next(&mut self) -> Option<(usize, &'a T)> {
        let current = self.current?;

        self.partition_vec.meta[current].set_parent(self.root);

        let next = self.partition_vec.meta[current].link();

        // We started at the root.
        self.current = if next == self.root {
            None
        } else {
            Some(next)
        };

        Some((current, &self.partition_vec.data[current]))
    }
}

impl<'a, T> FusedIterator for Set<'a, T> {}

/// An iterator over a set in a `PartitionVec<T>` that allows mutating elements.
///
/// This struct is created by the [`set_mut`] method on [`PartitionVec<T>`].
/// See its documentation for more.
///
/// [`set_mut`]: struct.PartitionVec.html#method.set_mut
/// [`PartitionVec<T>`]: struct.PartitionVec.html
#[derive(Debug)]
pub struct SetMut<'a, T: 'a> {
    partition_vec: &'a mut PartitionVec<T>,
    current: Option<usize>,
    root: usize,
}

impl<'a, T> Iterator for SetMut<'a, T> {
    type Item = (usize, &'a mut T);

    fn next(&mut self) -> Option<(usize, &'a mut T)> {
        let current = self.current?;

        self.partition_vec.meta[current].set_parent(self.root);

        let next = self.partition_vec.meta[current].link();

        // We started at the root.
        self.current = if next == self.root {
            None
        } else {
            Some(next)
        };

        // This iterator wont give a reference to this value again so it is safe to extend
        // the lifetime of the mutable reference.
        unsafe {
            Some((current, extend_mut(&mut self.partition_vec.data[current])))
        }
    }
}

impl<'a, T> FusedIterator for SetMut<'a, T> {}

/// An iterator over all sets in a `PartitionVec<T>`.
///
/// This struct is created by the [`all_sets`] method on [`PartitionVec<T>`].
/// See its documentation for more information.
///
/// [`all_sets`]: struct.PartitionVec.html#method.all_sets
/// [`PartitionVec<T>`]: struct.PartitionVec.html
#[derive(Clone, Debug)]
pub struct AllSets<'a, T: 'a> {
    partition_vec: &'a PartitionVec<T>,
    done: bit_vec::BitVec,
    range: ops::Range<usize>,
}

impl<'a, T> Iterator for AllSets<'a, T> {
    type Item = Set<'a, T>;

    fn next(&mut self) -> Option<Set<'a, T>> {
        // We keep going until we find a set we have not returned yet.
        loop {
            let index = self.range.next()?;
            let root = self.partition_vec.find_final(index);

            // If we have not returned this set yet.
            if !self.done.get(root).unwrap() {
                self.done.set(root, true);

                return Some(Set {
                    partition_vec: self.partition_vec,
                    current: Some(root),
                    root,
                })
            }
        }
    }
}

impl<'a, T> DoubleEndedIterator for AllSets<'a, T> {
    fn next_back(&mut self) -> Option<Set<'a, T>> {
        // We keep going until we find a set we have not returned yet.
        loop {
            let index = self.range.next_back()?;
            let root = self.partition_vec.find_final(index);

            // If we have not returned this set yet.
            if !self.done.get(root).unwrap() {
                self.done.set(root, true);

                return Some(Set {
                    partition_vec: self.partition_vec,
                    current: Some(root),
                    root,
                })
            }
        }
    }
}

impl<'a, T> FusedIterator for AllSets<'a, T> {}

/// An iterator over all sets in a `PartitionVec<T>` that allows mutating elements.
///
/// This struct is created by the [`all_sets`] method on [`PartitionVec<T>`].
/// See its documentation for more information.
///
/// [`all_sets`]: struct.PartitionVec.html#method.all_sets
/// [`PartitionVec<T>`]: struct.PartitionVec.html
#[derive(Debug)]
pub struct AllSetsMut<'a, T: 'a> {
    partition_vec: &'a mut PartitionVec<T>,
    done: bit_vec::BitVec,
    range: ops::Range<usize>,
}

impl<'a, T> Iterator for AllSetsMut<'a, T> {
    type Item = SetMut<'a, T>;

    fn next(&mut self) -> Option<SetMut<'a, T>> {
        // We keep going until we find a set we have not returned yet.
        loop {
            let index = self.range.next()?;
            let root = self.partition_vec.find_final(index);

            // If we have not returned this set yet.
            if !self.done.get(root).unwrap() {
                self.done.set(root, true);

                // This is safe because we will not return this set again.
                unsafe { return Some(SetMut {
                    partition_vec: extend_mut(self).partition_vec,
                    current: Some(root),
                    root,
                })}
            }
        }
    }
}

impl<'a, T> DoubleEndedIterator for AllSetsMut<'a, T> {
    fn next_back(&mut self) -> Option<SetMut<'a, T>> {
        // We keep going until we find a set we have not returned yet.
        loop {
            let index = self.range.next_back()?;
            let root = self.partition_vec.find_final(index);

            // If we have not returned this set yet.
            if !self.done.get(root).unwrap() {
                self.done.set(root, true);

                // This is safe because we will not return this set again.
                unsafe { return Some(SetMut {
                    partition_vec: extend_mut(self).partition_vec,
                    current: Some(root),
                    root,
                })}
            }
        }
    }
}

impl<'a, T> FusedIterator for AllSetsMut<'a, T> {}
