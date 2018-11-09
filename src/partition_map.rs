macro_rules! partition_map {
    (
        #[$doc: meta]
        $struct: tt <K, V$(, $generic: tt: $bound: tt = $default: tt)*>
        $map_mod: ident
        $map_struct: ident
        $($key_bounds: tt)*
    ) => {
        #[$doc]
        #[derive(Clone)]
        pub struct $struct<K, V$(, $generic = $default)*> {
            map: $map_struct<K, usize $(, $generic)*>,
            vec: PartitionVec<V>,
            last_removed: usize,
        }

        impl<K, V> $struct<K, V$(, $default)*> where
            K: $($key_bounds)*,
        {
            pub fn new() -> Self {
                Self {
                    map: $map_struct::new(),
                    vec: PartitionVec::new(),
                    last_removed: !0,
                }
            }
        }

        impl<K, V$(, $generic)*> $struct<K, V$(, $generic)*> where
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

            pub fn keys(&self) -> Keys<K, V> {
                Keys {
                    iter: self.map.keys(),
                    phantom: std::marker::PhantomData,
                }
            }

            pub fn values(&self) -> Values<K, V> {
                Values {
                    iter: self.map.values(),
                    vec: &self.vec,
                }
            }

            pub fn values_mut(&mut self) -> ValuesMut<K, V> {
                ValuesMut {
                    iter: self.map.values(),
                    vec: &mut self.vec,
                }
            }

            pub fn iter(&self) -> Iter<K, V> {
                Iter {
                    iter: self.map.iter(),
                    vec: &self.vec,
                }
            }

            pub fn iter_mut(&mut self) -> IterMut<K, V> {
                IterMut {
                    iter: self.map.iter(),
                    vec: &mut self.vec,
                }
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

        impl<K, V$(, $generic)*> Default for $struct<K, V$(, $generic)*> where
            K: $($key_bounds)*,
            $($generic: $bound + Default,)*
        {
            fn default() -> Self {
                Self {
                    map: $map_struct::default(),
                    vec: PartitionVec::new(),
                    last_removed: !0,
                }
            }
        }

        impl<'a, K, Q, V$(, $generic)*> ops::Index<&'a Q> for $struct<K, V$(, $generic)*> where
            K: $($key_bounds)* + Borrow<Q>,
            Q: $($key_bounds)* + ?Sized,
            $($generic: $bound,)*
        {
            type Output = V;

            fn index(&self, key: &Q) -> &V {
                &self.vec[self.map[key]]
            }
        }

        impl<K, V$(, $generic)*> Extend<(K, V)> for $struct<K, V$(, $generic)*> where
            K: $($key_bounds)*,
            $($generic: $bound,)*
        {
            fn extend<I>(&mut self, iter: I) where
                I: IntoIterator<Item = (K, V)>,
            {
                for (key, value) in iter {
                    self.insert(key, value);
                }
            }
        }

        impl<'a, K, V$(, $generic)*> Extend<(&'a K, &'a V)> for $struct<K, V$(, $generic)*> where
            K: $($key_bounds)* + Copy + 'a,
            V: Copy + 'a,
            $($generic: $bound,)*
        {
            fn extend<I>(&mut self, iter: I) where
                I: IntoIterator<Item = (&'a K, &'a V)>,
            {
                for (&key, &value) in iter {
                    self.insert(key, value);
                }
            }
        }

        impl<K, V$(, $generic)*> IntoIterator for $struct<K, V$(, $generic)*> where
            K: $($key_bounds)*,
            $($generic: $bound,)*
        {
            type Item = (K, V);
            type IntoIter = IntoIter<K, V>;

            fn into_iter(self) -> IntoIter<K, V> {
                let into_iter = unsafe {
                    IntoIter {
                        iter: std::ptr::read(&self.map).into_iter(),
                        vec: std::ptr::read(&self.vec),
                    }
                };

                std::mem::forget(self);

                into_iter
            }
        }

        impl<'a, K, V$(, $generic)*> IntoIterator for &'a $struct<K, V$(, $generic)*> where
            K: $($key_bounds)*,
            $($generic: $bound,)*
        {
            type Item = (&'a K, &'a V);
            type IntoIter = Iter<'a, K, V>;

            fn into_iter(self) -> Iter<'a, K, V> {
                self.iter()
            }
        }

        impl<'a, K, V$(, $generic)*> IntoIterator for &'a mut $struct<K, V$(, $generic)*> where
            K: $($key_bounds)*,
            $($generic: $bound,)*
        {
            type Item = (&'a K, &'a mut V);
            type IntoIter = IterMut<'a, K, V>;

            fn into_iter(self) -> IterMut<'a, K, V> {
                self.iter_mut()
            }
        }

        impl<K, V$(, $generic)*> Drop for $struct<K, V$(, $generic)*> {
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

        #[derive(Clone)]
        pub struct Keys<'a, K: 'a, V: 'a> {
            iter: $map_mod::Keys<'a, K, usize>,
            phantom: std::marker::PhantomData<&'a V>,
        }

        impl<'a, K, V> Iterator for Keys<'a, K, V> {
            type Item = &'a K;

            #[inline]
            fn next(&mut self) -> Option<&'a K> {
                self.iter.next()
            }

            #[inline]
            fn size_hint(&self) -> (usize, Option<usize>) {
                self.iter.size_hint()
            }
        }

        impl<'a, K, V> ExactSizeIterator for Keys<'a, K, V> {
            #[inline]
            fn len(&self) -> usize {
                self.iter.len()
            }
        }

        impl<'a, K, V> FusedIterator for Keys<'a, K, V> {}

        #[derive(Clone)]
        pub struct Values<'a, K: 'a, V: 'a> {
            iter: $map_mod::Values<'a, K, usize>,
            vec: &'a PartitionVec<V>,
        }

        impl<'a, K, V> Iterator for Values<'a, K, V> {
            type Item = &'a V;

            #[inline]
            fn next(&mut self) -> Option<&'a V> {
                Some(&self.vec[*self.iter.next()?])
            }

            #[inline]
            fn size_hint(&self) -> (usize, Option<usize>) {
                self.iter.size_hint()
            }
        }

        impl<'a, K, V> ExactSizeIterator for Values<'a, K, V> {
            #[inline]
            fn len(&self) -> usize {
                self.iter.len()
            }
        }

        impl<'a, K, V> FusedIterator for Values<'a, K, V> {}

        pub struct ValuesMut<'a, K: 'a, V: 'a> {
            iter: $map_mod::Values<'a, K, usize>,
            vec: &'a mut PartitionVec<V>,
        }

        impl<'a, K, V> Iterator for ValuesMut<'a, K, V> {
            type Item = &'a mut V;

            #[inline]
            fn next(&mut self) -> Option<&'a mut V> {
                unsafe { Some(crate::extend_mut(&mut self.vec[*self.iter.next()?])) }
            }

            #[inline]
            fn size_hint(&self) -> (usize, Option<usize>) {
                self.iter.size_hint()
            }
        }

        impl<'a, K, V> ExactSizeIterator for ValuesMut<'a, K, V> {
            #[inline]
            fn len(&self) -> usize {
                self.iter.len()
            }
        }

        impl<'a, K, V> FusedIterator for ValuesMut<'a, K, V> {}

        pub struct IntoIter<K, V> {
            iter: $map_mod::IntoIter<K, usize>,
            vec: PartitionVec<V>,
        }

        impl<K, V> Iterator for IntoIter<K, V> {
            type Item = (K, V);

            #[inline]
            fn next(&mut self) -> Option<(K, V)> {
                let (key, index) = self.iter.next()?;

                unsafe { Some((key, std::ptr::read(&self.vec[index]))) }
            }

            fn size_hint(&self) -> (usize, Option<usize>) {
                self.iter.size_hint()
            }
        }

        impl<K, V> ExactSizeIterator for IntoIter<K, V> {
            #[inline]
            fn len(&self) -> usize {
                self.iter.len()
            }
        }

        impl<K, V> FusedIterator for IntoIter<K, V> {}

        impl<K, V> Drop for IntoIter<K, V> {
            fn drop(&mut self) {
                while let Some(_) = self.next() {}

                unsafe {
                    self.vec.data.set_len(0);
                    self.vec.meta.set_len(0);
                }
            }
        }

        #[derive(Clone)]
        pub struct Iter<'a, K: 'a, V: 'a> {
            iter: $map_mod::Iter<'a, K, usize>,
            vec: &'a PartitionVec<V>,
        }

        impl<'a, K, V> Iterator for Iter<'a, K, V> {
            type Item = (&'a K, &'a V);

            #[inline]
            fn next(&mut self) -> Option<(&'a K, &'a V)> {
                let (key, &index) = self.iter.next()?;

                Some((key, &self.vec[index]))
            }

            fn size_hint(&self) -> (usize, Option<usize>) {
                self.iter.size_hint()
            }
        }

        impl<'a, K, V> ExactSizeIterator for Iter<'a, K, V> {
            #[inline]
            fn len(&self) -> usize {
                self.iter.len()
            }
        }

        impl<'a, K, V> FusedIterator for Iter<'a, K, V> {}

        pub struct IterMut<'a, K: 'a, V: 'a> {
            iter: $map_mod::Iter<'a, K, usize>,
            vec: &'a mut PartitionVec<V>,
        }

        impl<'a, K, V> Iterator for IterMut<'a, K, V> {
            type Item = (&'a K, &'a mut V);

            #[inline]
            fn next(&mut self) -> Option<(&'a K, &'a mut V)> {
                let (key, &index) = self.iter.next()?;

                unsafe { Some((key, crate::extend_mut(&mut self.vec[index]))) }
            }

            fn size_hint(&self) -> (usize, Option<usize>) {
                self.iter.size_hint()
            }
        }

        impl<'a, K, V> ExactSizeIterator for IterMut<'a, K, V> {
            #[inline]
            fn len(&self) -> usize {
                self.iter.len()
            }
        }

        impl<'a, K, V> FusedIterator for IterMut<'a, K, V> {}
    };
}

/// This is a mod.
pub mod partition_hash_map {
    use {
        std::{
            ops,
            borrow::Borrow,
            iter::FusedIterator,
            hash::{Hash, BuildHasher},
            collections::hash_map::{self, HashMap, RandomState},
        },
        crate::PartitionVec,
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

        pub fn remove_entry<Q>(&mut self, key: &Q) -> Option<(K, V)> where
            K: Borrow<Q>,
            Q: Eq + Hash + ?Sized,
        {
            let (key, index) = self.map.remove_entry(key)?;

            unsafe { Some((key, self.vec_lazy_remove(index))) }
        }
    }
}

pub mod partition_btree_map {
    use {
        std::{
            ops,
            borrow::Borrow,
            iter::FusedIterator,
            collections::btree_map::{self, BTreeMap},
        },
        crate::PartitionVec,
    };

    partition_map![
        /// This is a `PartitionBTreeMap`.
        PartitionBTreeMap<K, V>
        btree_map
        BTreeMap
        Ord
    ];

    impl<K, V> PartitionBTreeMap<K, V> where
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
                (key, unsafe { crate::extend_mut(&mut vec[index]) })
            )
        }
    }

    #[derive(Clone)]
    pub struct Range<'a, K: 'a, V: 'a> {
        iter: btree_map::Range<'a, K, usize>,
        vec: &'a PartitionVec<V>,
    }

    impl<'a, K, V> Iterator for Range<'a, K, V> {
        type Item = (&'a K, &'a V);

        #[inline]
        fn next(&mut self) -> Option<(&'a K, &'a V)> {
            let (key, &index) = self.iter.next()?;

            Some((key, &self.vec[index]))
        }
    }

    impl<'a, K, V> DoubleEndedIterator for Range<'a, K, V> {
        #[inline]
        fn next_back(&mut self) -> Option<(&'a K, &'a V)> {
            let (key, &index) = self.iter.next_back()?;

            Some((key, &self.vec[index]))
        }
    }

    impl<'a, K, V> FusedIterator for Range<'a, K, V> {}

    pub struct RangeMut<'a, K: 'a, V: 'a> {
        iter: btree_map::Range<'a, K, usize>,
        vec: &'a mut PartitionVec<V>,
    }

    impl<'a, K, V> Iterator for RangeMut<'a, K, V> {
        type Item = (&'a K, &'a mut V);

        #[inline]
        fn next(&mut self) -> Option<(&'a K, &'a mut V)> {
            let (key, &index) = self.iter.next()?;

            unsafe { Some((key, crate::extend_mut(&mut self.vec[index]))) }
        }
    }

    impl<'a, K, V> DoubleEndedIterator for RangeMut<'a, K, V> {
        #[inline]
        fn next_back(&mut self) -> Option<(&'a K, &'a mut V)> {
            let (key, &index) = self.iter.next_back()?;

            unsafe { Some((key, crate::extend_mut(&mut self.vec[index]))) }
        }
    }

    impl<'a, K, V> FusedIterator for RangeMut<'a, K, V> {}
}
