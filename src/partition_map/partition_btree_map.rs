use std::collections::btree_map::{self, BTreeMap};

partition_map![
    /// This is a `PartitionBTreeMap`.
    PartitionBTreeMap<K, V>
    btree_map
    BTreeMap
    Ord
];
/*
impl<K, V> PartitionBTreeMap<K, V> where
    K: Ord,
{
    pub fn range<Q, R>(&self, range: R) -> Range<K, V> where
        K: Borrow<Q>,
        R: ops::RangeBounds<Q>,
        Q: Ord + ?Sized,
    {
        Range {
            iter: self.map.range((coerce(range.start()), coerce(range.end()))),
            vec: &self.vec,
        }
    }

    pub fn range_mut<Q, R>(&mut self, range: R) -> RangeMut<K, V> where
        K: Borrow<Q>,
        R: ops::RangeBounds<Q>,
        Q: Ord + ?Sized,
    {
        RangeMut {
            iter: self.map.range((coerce(range.start()), coerce(range.end()))),
            vec: &mut self.vec,
        }
    }
}

#[derive(Clone)]
pub struct Range<'a, K: 'a, V: 'a> {
    iter: btree_map::Range<'a, NonNull<K>, usize>,
    vec: &'a PartitionVec<(K, V)>,
}

impl<'a, K, V> Iterator for Range<'a, K, V> {
    type Item = (&'a K, &'a V);

    #[inline]
    fn next(&mut self) -> Option<(&'a K, &'a V)> {
        let (key, &index) = self.iter.next()?;

        Some((key, &self.vec[index].1))
    }
}

impl<'a, K, V> DoubleEndedIterator for Range<'a, K, V> {
    #[inline]
    fn next_back(&mut self) -> Option<(&'a K, &'a V)> {
        let (key, &index) = self.iter.next_back()?;

        Some((key, &self.vec[index].1))
    }
}

impl<'a, K, V> FusedIterator for Range<'a, K, V> {}

pub struct RangeMut<'a, K: 'static, V: 'a> {
    iter: btree_map::Range<'a, NonNull<K>, usize>,
    vec: &'a mut PartitionVec<(K, V)>,
}

impl<'a, K, V> Iterator for RangeMut<'a, K, V> {
    type Item = (&'a K, &'a mut V);

    #[inline]
    fn next(&mut self) -> Option<(&'a K, &'a mut V)> {
        let (key, &index) = self.iter.next()?;

        unsafe { Some((key, crate::extend_mut(&mut self.vec[index].1))) }
    }
}

impl<'a, K, V> DoubleEndedIterator for RangeMut<'a, K, V> {
    #[inline]
    fn next_back(&mut self) -> Option<(&'a K, &'a mut V)> {
        let (key, &index) = self.iter.next_back()?;

        unsafe { Some((key, crate::extend_mut(&mut self.vec[index].1))) }
    }
}

impl<'a, K, V> FusedIterator for RangeMut<'a, K, V> {}
*/
