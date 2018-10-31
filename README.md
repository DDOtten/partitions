[![Build Status](https://travis-ci.org/DDOtten/partitions.png?branch=master)](https://travis-ci.org/DDOtten/partitions)

# Partitions

A [disjoint-sets/union-find] implementation of a vector partitioned in sets that
allows for efficient iteration over the elements of a set.

The main struct of this crate is [`PartitionVec<T>`] which has the functionality
of a `Vec<T>` and in addition divides the elements of this vector in sets.
The elements each start in their own set and sets can be joined with the
[`union`] method.
You can check if elements share a set with the [`same_set`] method and iterate
on the elements in a set with the [`set`] method.
The [`union`] and [`same_set`] methods are extremely fast and have an amortized
complexity of `O(α(n))` where `α` is the inverse Ackermann function and `n` is
the length.
This complexity is proven to be optimal and `α(n)` has value below 5 for any `n`
that can be written in the observable universe.
The next element of the iterator returned by [`set`] is found in `O(1)` time.

The Disjoint-Sets algorithm is used in high-performance implementations of
unification.
It is also a key component in implementing Kruskal's algorithm to find the
minimum spanning tree of a graph.

[disjoint-sets/union-find]:
https://en.wikipedia.org/wiki/Disjoint-set_data_structure
[`PartitionVec<T>`]:
https://docs.rs/partitions/0.2.0/partitions/partition_vec/struct.PartitionVec.html
[`union`]:
https://docs.rs/partitions/0.2.0/partitions/partition_vec/struct.PartitionVec.html#method.union
[`same_set`]:
https://docs.rs/partitions/0.2.0/partitions/partition_vec/struct.PartitionVec.html#method.same_set
[`set`]:
https://docs.rs/partitions/0.2.0/partitions/partition_vec/struct.PartitionVec.html#method.set
[`make_singleton`]:
https://docs.rs/partitions/0.2.0/partitions/partition_vec/struct.PartitionVec.html#method.make_singleton

## Using Partitions

The recommended way to use this crate is to add a line into your `Cargo.toml`
such as:

```toml
[dependencies]
partitions = "0.2"
```

and then add the following to to your `lib.rs` or `main.rs`:

```rust
extern crate partitions;
```

## License

Partitions is distributed under the terms of the Apache License (Version 2.0).
