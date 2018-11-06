use {
    std::{
        borrow::Borrow,
        hash::{Hash, BuildHasher},
        collections::hash_map::{self, HashMap, RandomState},
    },
    crate::PartitionVec,
};

pub struct PartitionHashMap<K, V, S = RandomState> {
    map: HashMap<K, usize, S>,
    vec: PartitionVec<V>,
    last_removed: usize,
}

impl<K, V> PartitionHashMap<K, V, RandomState> where
    K: Eq + Hash
{
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
            vec: PartitionVec::new(),
            last_removed: !0,
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            map: HashMap::with_capacity(capacity),
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
            map: HashMap::with_hasher(hash_builder),
            vec: PartitionVec::new(),
            last_removed: !0,
        }
    }

    pub fn with_capacity_and_hasher(capacity: usize, hash_builder: S) -> Self {
        Self {
            map: HashMap::with_capacity_and_hasher(capacity, hash_builder),
            vec: PartitionVec::with_capacity(capacity),
            last_removed: !0,
        }
    }

    pub fn union<Q1, Q2>(&mut self, first_key: &Q1, second_key: &Q2) where
        K: Borrow<Q1> + Borrow<Q2>,
        Q1: Eq + Hash + ?Sized,
        Q2: Eq + Hash + ?Sized,
    {
        self.vec.union(self.map[first_key], self.map[second_key]);
    }

    pub fn same_set<Q1, Q2>(&self, first_key: &Q1, second_key: &Q2) -> bool where
        K: Borrow<Q1> + Borrow<Q2>,
        Q1: Eq + Hash + ?Sized,
        Q2: Eq + Hash + ?Sized,
    {
        self.vec.same_set(self.map[first_key], self.map[second_key])
    }

    pub fn other_sets<Q1, Q2>(&self, first_key: &Q1, second_key: &Q2) -> bool where
        K: Borrow<Q1> + Borrow<Q2>,
        Q1: Eq + Hash + ?Sized,
        Q2: Eq + Hash + ?Sized,
    {
        self.vec.other_sets(self.map[first_key], self.map[second_key])
    }

    pub fn make_singleton<Q>(&mut self, key: &Q) where
        K: Borrow<Q>,
        Q: Eq + Hash + ?Sized,
    {
        self.vec.make_singleton(self.map[key]);
    }

    pub fn is_singleton<Q>(&self, key: &Q) -> bool where
        K: Borrow<Q>,
        Q: Eq + Hash + ?Sized,
    {
        self.vec.is_singleton(self.map[key])
    }

    pub fn len_of_set<Q>(&self, key: &Q) -> usize where
        K: Borrow<Q>,
        Q: Eq + Hash + ?Sized,
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

    pub fn hasher(&self) -> &S {
        self.map.hasher()
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

    pub fn keys(&self) -> hash_map::Keys<K, usize> {
        self.map.keys()
    }

    pub fn values(&self) -> std::slice::Iter<V> {
        self.vec.iter()
    }

    pub fn values_mut(&mut self) -> std::slice::IterMut<V> {
        self.vec.iter_mut()
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
        Q: Eq + Hash + ?Sized,
    {
        self.vec.get(*self.map.get(key)?)
    }

    pub fn get_mut<Q>(&mut self, key: &Q) -> Option<&mut V> where
        K: Borrow<Q>,
        Q: Eq + Hash + ?Sized,
    {
        self.vec.get_mut(*self.map.get(key)?)
    }

    pub fn contains_key<Q>(&self, key: &Q) -> bool where
        K: Borrow<Q>,
        Q: Eq + Hash + ?Sized,
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
        Q: Eq + Hash + ?Sized,
    {
        let index = self.map.remove(key)?;

        unsafe { Some(self.vec_lazy_remove(index)) }
    }

    pub fn remove_entry<Q>(&mut self, key: &Q) -> Option<(K, V)> where
        K: Borrow<Q>,
        Q: Eq + Hash + ?Sized,
    {
        let (key, index) = self.map.remove_entry(key)?;

        unsafe { Some((key, self.vec_lazy_remove(index))) }
    }

    unsafe fn vec_lazy_insert(&mut self, index: usize, value: V) {
        self.last_removed = self.vec.meta[index].marked_value();

        std::ptr::write(&mut self.vec.data[index], value);
        self.vec.meta[index] = crate::metadata::Metadata::new(index);
    }

    unsafe fn vec_lazy_remove(&mut self, index: usize) -> V {
        self.vec.make_singleton(index);

        let value = std::ptr::read(&self.vec.data[index]);
        self.vec.meta[index].set_marked_value(self.last_removed);

        self.last_removed = index;

        value
    }
}

impl<K, V> Default for PartitionHashMap<K, V, RandomState> where
    K: Eq + Hash
{
    fn default() -> Self {
        Self::new()
    }
}

impl<K, V, S> Drop for PartitionHashMap<K, V, S> {
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
