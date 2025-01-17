/*
 * Copyright 2019 The Starlark in Rust Authors.
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     https://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

use std::mem;

use gazebo::prelude::*;
use indexmap::Equivalent;

use crate::collections::hash::{BorrowHashed, Hashed, SmallHashResult};

// We define a lot of iterators on top of other iterators
// so define a helper macro for that
macro_rules! def_iter {
    () => {
        fn next(&mut self) -> Option<Self::Item> {
            self.iter.next().map(Self::map)
        }

        fn nth(&mut self, n: usize) -> Option<Self::Item> {
            self.iter.nth(n).map(Self::map)
        }

        fn last(mut self) -> Option<Self::Item> {
            // Since these are all double-ended iterators we can skip to the end quickly
            self.iter.next_back().map(Self::map)
        }

        fn size_hint(&self) -> (usize, Option<usize>) {
            self.iter.size_hint()
        }

        fn count(self) -> usize {
            self.iter.len()
        }

        fn collect<C>(self) -> C
        where
            C: std::iter::FromIterator<Self::Item>,
        {
            self.iter.map(Self::map).collect()
        }
    };
}

/// Bucket in [`VecMap`].
#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) struct Bucket<K, V> {
    pub(crate) hash: SmallHashResult,
    pub(crate) key: K,
    pub(crate) value: V,
}

#[derive(Debug, Clone, Eq, PartialEq, Default_)]
pub struct VecMap<K, V> {
    pub(crate) buckets: Vec<Bucket<K, V>>,
}

#[derive(Clone_)]
pub struct VMKeys<'a, K: 'a, V: 'a> {
    iter: std::slice::Iter<'a, Bucket<K, V>>,
}

impl<'a, K: 'a, V: 'a> VMKeys<'a, K, V> {
    fn map(b: &'a Bucket<K, V>) -> <Self as Iterator>::Item {
        &b.key
    }
}

impl<'a, K: 'a, V: 'a> Iterator for VMKeys<'a, K, V> {
    type Item = &'a K;

    def_iter!();
}

impl<'a, K: 'a, V: 'a> ExactSizeIterator for VMKeys<'a, K, V> {
    fn len(&self) -> usize {
        self.iter.len()
    }
}

#[derive(Clone_)]
pub struct VMValues<'a, K: 'a, V: 'a> {
    iter: std::slice::Iter<'a, Bucket<K, V>>,
}

impl<'a, K: 'a, V: 'a> VMValues<'a, K, V> {
    fn map(b: &'a Bucket<K, V>) -> <Self as Iterator>::Item {
        &b.value
    }
}

impl<'a, K: 'a, V: 'a> Iterator for VMValues<'a, K, V> {
    type Item = &'a V;

    def_iter!();
}

impl<'a, K: 'a, V: 'a> ExactSizeIterator for VMValues<'a, K, V> {
    fn len(&self) -> usize {
        self.iter.len()
    }
}

pub struct VMValuesMut<'a, K: 'a, V: 'a> {
    iter: std::slice::IterMut<'a, Bucket<K, V>>,
}

impl<'a, K: 'a, V: 'a> VMValuesMut<'a, K, V> {
    fn map(b: &'a mut Bucket<K, V>) -> <Self as Iterator>::Item {
        &mut b.value
    }
}

impl<'a, K: 'a, V: 'a> Iterator for VMValuesMut<'a, K, V> {
    type Item = &'a mut V;

    def_iter!();
}

impl<'a, K: 'a, V: 'a> ExactSizeIterator for VMValuesMut<'a, K, V> {
    fn len(&self) -> usize {
        self.iter.len()
    }
}

#[derive(Clone_)]
pub struct VMIter<'a, K: 'a, V: 'a> {
    iter: std::slice::Iter<'a, Bucket<K, V>>,
}

impl<'a, K: 'a, V: 'a> Iterator for VMIter<'a, K, V> {
    type Item = (&'a K, &'a V);

    def_iter!();
}

impl<'a, K: 'a, V: 'a> ExactSizeIterator for VMIter<'a, K, V> {}

impl<'a, K: 'a, V: 'a> VMIter<'a, K, V> {
    fn map(b: &Bucket<K, V>) -> (&K, &V) {
        (&b.key, &b.value)
    }
}

pub struct VMIterHash<'a, K: 'a, V: 'a> {
    iter: std::slice::Iter<'a, Bucket<K, V>>,
}

impl<'a, K: 'a, V: 'a> VMIterHash<'a, K, V> {
    fn map(b: &'a Bucket<K, V>) -> (BorrowHashed<'a, K>, &'a V) {
        (BorrowHashed::new_unchecked(b.hash, &b.key), &b.value)
    }
}

impl<'a, K: 'a, V: 'a> Iterator for VMIterHash<'a, K, V> {
    type Item = (BorrowHashed<'a, K>, &'a V);

    def_iter!();
}

impl<'a, K: 'a, V: 'a> ExactSizeIterator for VMIterHash<'a, K, V> {
    fn len(&self) -> usize {
        self.iter.len()
    }
}

pub struct VMIterMut<'a, K: 'a, V: 'a> {
    iter: std::slice::IterMut<'a, Bucket<K, V>>,
}

impl<'a, K: 'a, V: 'a> VMIterMut<'a, K, V> {
    fn map(b: &mut Bucket<K, V>) -> (&K, &mut V) {
        (&b.key, &mut b.value)
    }
}

impl<'a, K: 'a, V: 'a> Iterator for VMIterMut<'a, K, V> {
    type Item = (&'a K, &'a mut V);

    def_iter!();
}

impl<'a, K: 'a, V: 'a> ExactSizeIterator for VMIterMut<'a, K, V> {
    fn len(&self) -> usize {
        self.iter.len()
    }
}

pub struct VMIntoIterHash<K, V> {
    iter: std::vec::IntoIter<Bucket<K, V>>,
}

impl<K, V> Iterator for VMIntoIterHash<K, V> {
    type Item = (Hashed<K>, V);

    fn next(&mut self) -> Option<Self::Item> {
        self.iter
            .next()
            .map(|b| (Hashed::new_unchecked(b.hash, b.key), b.value))
    }

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        self.iter
            .nth(n)
            .map(|b| (Hashed::new_unchecked(b.hash, b.key), b.value))
    }

    fn last(mut self) -> Option<Self::Item> {
        // Since these are all double-ended iterators we can skip to the end quickly
        self.iter
            .next_back()
            .map(|b| (Hashed::new_unchecked(b.hash, b.key), b.value))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }

    fn count(self) -> usize {
        self.iter.len()
    }

    fn collect<C>(self) -> C
    where
        C: std::iter::FromIterator<Self::Item>,
    {
        self.iter
            .map(|b| (Hashed::new_unchecked(b.hash, b.key), b.value))
            .collect()
    }
}

impl<K, V> ExactSizeIterator for VMIntoIterHash<K, V> {
    fn len(&self) -> usize {
        self.iter.len()
    }
}

pub struct VMIntoIter<K, V> {
    iter: std::vec::IntoIter<Bucket<K, V>>,
}

impl<K, V> VMIntoIter<K, V> {
    fn map(b: Bucket<K, V>) -> (K, V) {
        (b.key, b.value)
    }
}

impl<'a, K: 'a, V: 'a> Iterator for VMIntoIter<K, V> {
    type Item = (K, V);

    def_iter!();
}

impl<'a, K: 'a, V: 'a> ExactSizeIterator for VMIntoIter<K, V> {
    fn len(&self) -> usize {
        self.iter.len()
    }
}

impl<K, V> VecMap<K, V> {
    pub fn with_capacity(n: usize) -> Self {
        VecMap {
            buckets: Vec::with_capacity(n),
        }
    }

    pub fn reserve(&mut self, additional: usize) {
        self.buckets.reserve(additional);
    }

    pub fn capacity(&self) -> usize {
        self.buckets.capacity()
    }

    pub(crate) fn extra_memory(&self) -> usize {
        self.buckets.capacity() * mem::size_of::<Bucket<K, V>>()
    }

    pub fn get_full<Q>(&self, key: BorrowHashed<Q>) -> Option<(usize, &K, &V)>
    where
        Q: ?Sized + Equivalent<K>,
    {
        // This method is _very_ hot. There are three ways to implement this scan:
        // 1) Checked index operations.
        // 2) Unchecked index operations.
        // 3) Iterators.
        // Iterators would be best, but is significantly slower, so go with unchecked.
        // (25% on a benchmark which did a lot of other stuff too).
        let mut i = 0;
        #[allow(clippy::explicit_counter_loop)] // we are paranoid about performance
        for b in &self.buckets {
            // We always have at least as many hashes as value, so this index is safe.
            if b.hash == key.hash() && key.key().equivalent(&b.key) {
                return Some((i, &b.key, &b.value));
            }
            i += 1;
        }
        None
    }

    pub fn get_index_of_hashed<Q>(&self, key: BorrowHashed<Q>) -> Option<usize>
    where
        Q: ?Sized + Equivalent<K>,
    {
        self.get_full(key).map(|(i, _, _)| i)
    }

    pub fn get_index(&self, index: usize) -> Option<(&K, &V)> {
        self.buckets.get(index).map(|x| (&x.key, &x.value))
    }

    pub(crate) unsafe fn get_unchecked(&self, index: usize) -> &Bucket<K, V> {
        debug_assert!(index < self.buckets.len());
        self.buckets.get_unchecked(index)
    }

    pub(crate) unsafe fn get_unchecked_mut(&mut self, index: usize) -> &mut Bucket<K, V> {
        debug_assert!(index < self.buckets.len());
        self.buckets.get_unchecked_mut(index)
    }

    pub(crate) fn insert_unique_unchecked(&mut self, key: Hashed<K>, value: V) {
        self.buckets.push(Bucket {
            hash: key.hash(),
            key: key.into_key(),
            value,
        });
    }

    pub fn remove_hashed_entry<Q>(&mut self, key: BorrowHashed<Q>) -> Option<(K, V)>
    where
        Q: ?Sized + Equivalent<K>,
    {
        let len = self.buckets.len();
        if len == 0 {
            return None;
        }

        for i in 0..len {
            if self.buckets[i].hash == key.hash() && key.key().equivalent(&self.buckets[i].key) {
                let b = self.buckets.remove(i);
                return Some((b.key, b.value));
            }
        }
        None
    }

    pub fn len(&self) -> usize {
        self.buckets.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buckets.is_empty()
    }

    pub fn clear(&mut self) {
        self.buckets.clear();
    }

    pub fn values(&self) -> VMValues<K, V> {
        VMValues {
            iter: self.buckets.iter(),
        }
    }

    pub fn values_mut(&mut self) -> VMValuesMut<K, V> {
        VMValuesMut {
            iter: self.buckets.iter_mut(),
        }
    }

    pub fn keys(&self) -> VMKeys<K, V> {
        VMKeys {
            iter: self.buckets.iter(),
        }
    }

    pub fn into_iter(self) -> VMIntoIter<K, V> {
        VMIntoIter {
            iter: self.buckets.into_iter(),
        }
    }

    pub fn iter(&self) -> VMIter<K, V> {
        VMIter {
            iter: self.buckets.iter(),
        }
    }

    pub fn iter_hashed(&self) -> VMIterHash<K, V> {
        VMIterHash {
            // Values go first since they terminate first and we can short-circuit
            iter: self.buckets.iter(),
        }
    }

    pub fn into_iter_hashed(self) -> VMIntoIterHash<K, V> {
        // See the comments on VMIntoIterHash for why this one looks different
        VMIntoIterHash {
            iter: self.buckets.into_iter(),
        }
    }

    pub fn iter_mut(&mut self) -> VMIterMut<K, V> {
        VMIterMut {
            iter: self.buckets.iter_mut(),
        }
    }
}
