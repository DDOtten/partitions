use {
      std::{
          ops,
          fmt,
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
