use {
    std::{
        ops,
        borrow::Borrow,
        hash::{Hash, BuildHasher},
    },
    crate::{PartitionVec, extend_mut},
};

macro_rules! partition_map {
    (
        #[$mod_doc: meta]
        $mod: tt,
        #[$struct_doc: meta]
        $struct: tt,
        $map: ident <
            K,
            V
            $(, $generic: tt : $bound: tt = $default: tt)*
        >,
        $($key_bounds: tt)*
    ) => {
        #[$mod_doc]
        pub mod $mod {
            #[allow(unused_imports)]
            use {
                std::{
                    ops,
                    borrow::Borrow,
                    hash::{Hash, BuildHasher},
                    collections::hash_map::{self, HashMap, RandomState},
                    collections::btree_map::BTreeMap,
                },
                crate::PartitionVec,
            };

            #[$struct_doc]
            pub struct $struct<K, V $(, $generic = $default)*> {
                pub(crate) map: $map<K, usize $(, $generic)*>,
                pub(crate) vec: PartitionVec<V>,
                pub(crate) last_removed: usize,
            }

            impl<K, V> $struct<K, V $(, $default)*> where
                K: $($key_bounds)*,
            {
                pub fn new() -> Self {
                    Self {
                        map: $map::new(),
                        vec: PartitionVec::new(),
                        last_removed: !0,
                    }
                }
            }

            impl<K, V $(, $generic)*> $struct<K, V $(, $generic)*> where
                K: $($key_bounds)*,
                $($generic: $bound,)*
            {
                pub fn union<Q1, Q2>(&mut self, first_key: &Q1, second_key: &Q2) where
                    K: Borrow<Q1> + Borrow<Q2>,
                    Q1: $($key_bounds)* + ?Sized,
                    Q2: $($key_bounds)* + ?Sized,
                {
                    self.vec.union(self.map[first_key], self.map[second_key]);
                }

                pub fn same_set<Q1, Q2>(&self, first_key: &Q1, second_key: &Q2) -> bool where
                    K: Borrow<Q1> + Borrow<Q2>,
                    Q1: $($key_bounds)* + ?Sized,
                    Q2: $($key_bounds)* + ?Sized,
                {
                    self.vec.same_set(self.map[first_key], self.map[second_key])
                }

                pub fn other_sets<Q1, Q2>(&self, first_key: &Q1, second_key: &Q2) -> bool where
                    K: Borrow<Q1> + Borrow<Q2>,
                    Q1: $($key_bounds)* + ?Sized,
                    Q2: $($key_bounds)* + ?Sized,
                {
                    self.vec.other_sets(self.map[first_key], self.map[second_key])
                }

                pub fn make_singleton<Q>(&mut self, key: &Q) where
                    K: Borrow<Q>,
                    Q: $($key_bounds)* + ?Sized,
                {
                    self.vec.make_singleton(self.map[key]);
                }

                pub fn is_singleton<Q>(&self, key: &Q) -> bool where
                    K: Borrow<Q>,
                    Q: $($key_bounds)* + ?Sized,
                {
                    self.vec.is_singleton(self.map[key])
                }

                pub fn len_of_set<Q>(&self, key: &Q) -> usize where
                    K: Borrow<Q>,
                    Q: $($key_bounds)* + ?Sized,
                {
                    self.vec.len_of_set(self.map[key])
                }

                pub fn amount_of_sets(&self) -> usize {
                    let mut done = bit_vec![false; self.vec.len()];
                    let mut count = 0;

                    for &i in self.map.values() {
                        if !done.get(self.vec.find(i)).unwrap() {
                            done.set(self.vec.find(i), true);
                            count += 1;
                        }
                    }

                    count
                }

                pub fn len(&self) -> usize {
                    self.map.len()
                }

                pub fn is_empty(&self) -> bool {
                    self.map.len() == 0
                }

                pub fn clear(&mut self) {
                    self.map.clear();

                    unsafe {
                        for i in 0 .. self.vec.len() {
                            if !self.vec.meta[i].is_marked() {
                                drop(std::ptr::read(&self.vec.data[i]));
                            }
                        }
                        self.vec.data.set_len(0);
                        self.vec.meta.set_len(0);
                    }
                }

                pub fn get<Q>(&self, key: &Q) -> Option<&V> where
                    K: Borrow<Q>,
                    Q: $($key_bounds)* + ?Sized,
                {
                    self.vec.get(*self.map.get(key)?)
                }

                pub fn get_mut<Q>(&mut self, key: &Q) -> Option<&mut V> where
                    K: Borrow<Q>,
                    Q: $($key_bounds)* + ?Sized,
                {
                    self.vec.get_mut(*self.map.get(key)?)
                }

                pub fn contains_key<Q>(&self, key: &Q) -> bool where
                    K: Borrow<Q>,
                    Q: $($key_bounds)* + ?Sized,
                {
                    self.map.contains_key(key)
                }

                pub fn insert(&mut self, key: K, mut value: V) -> Option<V> {
                    let index = self.map.get(&key).cloned();

                    if let Some(index) = index {
                        std::mem::swap(&mut self.vec[index], &mut value);
                        Some(value)
                    } else {
                        if self.last_removed == !0 {
                            self.map.insert(key, self.vec.len());
                            self.vec.push(value);
                        } else {
                            let index = self.last_removed;
                            self.map.insert(key, index);
                            unsafe { self.vec_lazy_insert(index, value) };
                        }
                        None
                    }
                }

                pub fn remove<Q>(&mut self, key: &Q) -> Option<V> where
                    K: Borrow<Q>,
                    Q: $($key_bounds)* + ?Sized,
                {
                    let index = self.map.remove(key)?;

                    unsafe { Some(self.vec_lazy_remove(index)) }
                }

                pub(crate) unsafe fn vec_lazy_insert(&mut self, index: usize, value: V) {
                    self.last_removed = self.vec.meta[index].marked_value();

                    std::ptr::write(&mut self.vec.data[index], value);
                    self.vec.meta[index] = crate::metadata::Metadata::new(index);
                }

                pub(crate) unsafe fn vec_lazy_remove(&mut self, index: usize) -> V {
                    self.vec.make_singleton(index);

                    let value = std::ptr::read(&self.vec.data[index]);
                    self.vec.meta[index].set_marked_value(self.last_removed);

                    self.last_removed = index;

                    value
                }
            }

            impl<K, V $(, $generic)*> Default for $struct<K, V $(, $generic)*> where
                K: $($key_bounds)*,
                $($generic: $bound + Default,)*
            {
                fn default() -> Self {
                    Self {
                        map: $map::default(),
                        vec: PartitionVec::new(),
                        last_removed: !0,
                    }
                }
            }

            impl<'a, K, Q, V $(, $generic)*> ops::Index<&'a Q> for $struct<K, V $(, $generic)*> where
                K: $($key_bounds)* + Borrow<Q>,
                Q: $($key_bounds)* + ?Sized,
                $($generic: $bound,)*
            {
                type Output = V;

                fn index(&self, key: &Q) -> &V {
                    &self.vec[self.map[key]]
                }
            }

            impl<K, V $(, $generic)*> Drop for $struct<K, V $(, $generic)*> {
                fn drop(&mut self) {
                    unsafe {
                        for i in 0 .. self.vec.len() {
                            if !self.vec.meta[i].is_marked() {
                                drop(std::ptr::read(&self.vec.data[i]));
                            }
                        }
                        self.vec.data.set_len(0);
                        self.vec.meta.set_len(0);
                    }
                }
            }
        }
    };
}

partition_map![
    ///This is a `partition_hash_map`.
    partition_hash_map,
    /// This is a `PartitionHashMap`.
    PartitionHashMap,
    HashMap<K, V, S: BuildHasher = RandomState>,
    Eq + Hash
];

impl<K, V> partition_hash_map::PartitionHashMap<K, V, std::collections::hash_map::RandomState> where
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

impl<K, V, S> partition_hash_map::PartitionHashMap<K, V, S> where
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

    pub fn remove_entry<Q>(&mut self, key: &Q) -> Option<(K, V)> where
        K: Borrow<Q>,
        Q: Eq + Hash + ?Sized,
    {
        let (key, index) = self.map.remove_entry(key)?;

        unsafe { Some((key, self.vec_lazy_remove(index))) }
    }
}

partition_map![
    ///This is a `partition_hash_map`.
    partition_btree_map,
    /// This is a `PartitionHashMap`.
    PartitionBTreeMap,
    BTreeMap<K, V>,
    Ord
];

impl<K, V> partition_btree_map::PartitionBTreeMap<K, V> where
    K: Ord,
{
    pub fn range<Q, R>(&self, range: R) -> impl Iterator<Item = (&K, &V)> where
        K: Borrow<Q>,
        R: ops::RangeBounds<Q>,
        Q: Ord + ?Sized,
    {
        let vec = &self.vec;
        self.map.range(range).map(move |(key, &index)|
            (key, &vec[index])
        )
    }

    pub fn range_mut<Q, R>(&mut self, range: R) -> impl Iterator<Item = (&K, &mut V)> where
        K: Borrow<Q>,
        R: ops::RangeBounds<Q>,
        Q: Ord + ?Sized,
    {
        let vec = &mut self.vec;
        self.map.range(range).map(move |(key, &index)|
            (key, unsafe { extend_mut(&mut vec[index]) })
        )
    }
}
