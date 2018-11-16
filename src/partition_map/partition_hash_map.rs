use std::{
    hash::{Hash, BuildHasher},
    collections::hash_map::{self, HashMap, RandomState},
};

partition_map![
    /// This is a `PartitionHashMap`.
    PartitionHashMap<K, V, S: BuildHasher = RandomState>
    hash_map
    HashMap
    Eq + Hash
];

impl<K, V> PartitionHashMap<K, V, std::collections::hash_map::RandomState> where
    K: Eq + Hash,
{
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            map: std::collections::HashMap::with_capacity(capacity),
            vec: PartitionVec::with_capacity(capacity),
            last_removed: !0,
        }
    }
}

impl<K, V, S> PartitionHashMap<K, V, S> where
    K: Eq + Hash,
    S: BuildHasher,
{
    pub fn with_hasher(hash_builder: S) -> Self {
        Self {
            map: std::collections::HashMap::with_hasher(hash_builder),
            vec: PartitionVec::new(),
            last_removed: !0,
        }
    }

    pub fn with_capacity_and_hasher(capacity: usize, hash_builder: S) -> Self {
        Self {
            map: std::collections::HashMap::with_capacity_and_hasher(capacity, hash_builder),
            vec: PartitionVec::with_capacity(capacity),
            last_removed: !0,
        }
    }

    pub fn capacity(&self) -> usize {
        usize::min(self.map.capacity(), self.vec.capacity())
    }

    pub fn reserve(&mut self, additional: usize) {
        self.map.reserve(additional);
        self.vec.reserve(additional);
    }

    pub fn shrink_to_fit(&mut self) {
        self.map.shrink_to_fit();
        self.vec.shrink_to_fit();
    }

    pub fn hasher(&self) -> &S {
        self.map.hasher()
    }
}
