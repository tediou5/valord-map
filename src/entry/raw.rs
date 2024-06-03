use crate::{OrdBy, ValordMap};

use std::hash::Hash;
use std::ops::{Deref, DerefMut};

pub struct RawEntry<'v, T, K, V>
where
    T: Ord + Clone,
    K: Hash + Eq,
    V: OrdBy<Target = T>,
{
    pub(crate) index: usize,
    pub(crate) valord: &'v mut ValordMap<T, K, V>,
}

impl<'v, T, K, V> RawEntry<'v, T, K, V>
where
    T: Ord + Clone,
    K: Hash + Eq,
    V: OrdBy<Target = T>,
{
    pub(crate) fn try_new_by_key<'a: 'v>(
        valord: &'a mut ValordMap<T, K, V>,
        key: &K,
    ) -> Option<RawEntry<'v, T, K, V>> {
        let (index, _, v) = valord.map.get_full(key)?;
        v.as_ref()?;
        Some(Self { index, valord })
    }

    pub(crate) fn try_new_by_index<'a: 'v>(
        valord: &'a mut ValordMap<T, K, V>,
        index: usize,
    ) -> Option<RawEntry<'v, T, K, V>> {
        valord.get_by_index(index)?;
        Some(Self { index, valord })
    }

    pub(crate) fn insert(&mut self, value: V) -> &mut V {
        let v = self
            .valord
            .map
            .get_index_mut(self.index)
            .map(|(_, v)| {
                if let Some(ref v) = v {
                    ValordMap::<T, K, V>::remove_from_indexs(
                        &mut self.valord.sorted_indexs,
                        v.ord_by(),
                        self.index,
                    );
                }
                v.insert(value)
            })
            .unwrap();
        v
    }

    pub(crate) fn insert_with_key<F: FnOnce(&K) -> V>(&mut self, value: F) -> &mut V {
        let v = self
            .valord
            .map
            .get_index_mut(self.index)
            .map(|(k, v)| {
                if let Some(ref v) = v {
                    ValordMap::<T, K, V>::remove_from_indexs(
                        &mut self.valord.sorted_indexs,
                        v.ord_by(),
                        self.index,
                    );
                }
                v.insert(value(k))
            })
            .unwrap();
        v
    }

    pub fn get_mut_with_key(&mut self) -> (&K, &mut V) {
        let (k, v) = self
            .valord
            .map
            .get_index_mut(self.index)
            .map(|(k, v)| (k, v.as_mut().unwrap()))
            .unwrap();
        ValordMap::<T, K, V>::remove_from_indexs(
            &mut self.valord.sorted_indexs,
            v.ord_by(),
            self.index,
        );

        (k, v)
    }
}

impl<'a, T, K, V> Deref for RawEntry<'a, T, K, V>
where
    T: Ord + Clone,
    K: Hash + Eq,
    V: OrdBy<Target = T>,
{
    type Target = V;

    fn deref(&self) -> &Self::Target {
        // Safety: if value is not exist, try_new() will return None
        self.valord
            .get_by_index(self.index)
            .map(|(_, v)| v)
            .unwrap()
    }
}

impl<'a, T, K, V> DerefMut for RawEntry<'a, T, K, V>
where
    T: Ord + Clone,
    K: Hash + Eq,
    V: OrdBy<Target = T>,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        // Safety: if value is not exist, try_new() will return None
        self.get_mut_with_key().1
    }
}

impl<'a, T, K, V> Drop for RawEntry<'a, T, K, V>
where
    T: Ord + Clone,
    K: Hash + Eq,
    V: OrdBy<Target = T>,
{
    fn drop(&mut self) {
        if let Some(ord_by) = self
            .valord
            .get_by_index(self.index)
            .map(|(_, v)| v.ord_by().clone())
        {
            self.valord
                .sorted_indexs
                .entry(ord_by)
                .or_default()
                .insert(self.index);
        };
    }
}
