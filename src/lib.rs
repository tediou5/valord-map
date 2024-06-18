#![feature(let_chains)]
#![doc = include_str!("../README.md")]
#![doc(html_playground_url = "https://play.rust-lang.org")]
mod order_by;
pub use order_by::OrdBy;

mod entry;
pub use entry::{Entry, RawEntry};

use indexmap::{map::MutableKeys, IndexMap};
use std::{
    collections::{BTreeMap, HashSet, VecDeque},
    hash::Hash,
};

pub struct ValordMap<T, K, V: OrdBy<Target = T>> {
    map: IndexMap<K, Option<V>>,
    sorted_indexs: BTreeMap<T, HashSet<usize>>,

    free_indexs: VecDeque<usize>,
}

impl<T, K, V> ValordMap<T, K, V>
where
    T: Ord + Clone,
    K: Hash + Eq,
    V: OrdBy<Target = T>,
{
    pub fn new() -> Self {
        ValordMap {
            map: IndexMap::new(),
            sorted_indexs: BTreeMap::new(),
            free_indexs: VecDeque::new(),
        }
    }

    /// insert into ValordMap
    ///
    /// # Example
    ///
    /// ```
    /// use valord_map::ValordMap;
    ///
    /// let mut valord = ValordMap::new();
    /// valord.insert("qians", 1);
    /// valord.insert("tedious", 2);
    /// valord.insert("xuandu", 3);
    /// valord.insert("xuandu", 1);
    ///
    /// let sorted_pairs: Vec<_> = valord.iter().collect();
    ///
    /// println!("{:?}", sorted_pairs);
    ///
    /// assert_eq!(sorted_pairs.len(), 3);
    /// assert_eq!(sorted_pairs[0].1, &1);
    /// assert_eq!(sorted_pairs[1].1, &1);
    /// assert_eq!(sorted_pairs[2], (&"tedious", &2));
    /// ```
    pub fn insert(&mut self, key: K, value: V) {
        // let key: Arc<_> = key.into();
        self._insert(key, value)
    }

    fn _insert(&mut self, key: K, value: V) {
        let ord_by = value.ord_by();

        let index = if let Some((index, _k, old_val)) = self.map.get_full_mut(&key) {
            if let Some(old_val) = old_val {
                Self::remove_from_indexs(&mut self.sorted_indexs, &old_val.ord_by(), index);
                *old_val = value;
            }
            index
        } else if let Some(free_index) = self.free_indexs.front().copied()
            && let Some((k, v)) = self.map.get_index_mut2(free_index)
        {
            *k = key;
            *v = Some(value);
            self.free_indexs.pop_front();
            free_index
        } else {
            self.map.insert_full(key, Some(value)).0
        };

        self.sorted_indexs.entry(ord_by).or_default().insert(index);
    }

    /// Get the given key’s corresponding entry in the map for insertion and/or
    /// in-place manipulation
    ///
    /// # Examples
    ///
    /// ```
    /// use valord_map::ValordMap;
    ///
    /// let mut map = ValordMap::new();
    /// map.entry("key").and_modify(|v| *v = "new value").or_insert("value");
    ///
    /// assert_eq!(map.get(&"key"), Some(&"value"));
    ///
    /// map.entry("key").and_modify(|v| *v = "new value").or_insert("value");
    ///
    /// assert_eq!(map.get(&"key"), Some(&"new value"));
    /// ```
    pub fn entry(&mut self, key: K) -> Entry<'_, T, K, V> {
        let valord = self;
        match valord.map.get_full(&key) {
            Some((index, _, Some(_))) => return Entry::Occupied(RawEntry { index, valord }),
            Some((index, _, None)) => return Entry::Vacant(RawEntry { index, valord }),
            None => {}
        }

        let index = if let Some(free_index) = valord.free_indexs.front().copied() {
            free_index
        } else {
            let index_entry = valord.map.entry(key);
            let index = index_entry.index();
            index_entry.or_insert(None);
            valord.free_indexs.push_front(index);
            index
        };

        Entry::Vacant(RawEntry { index, valord })
    }

    /// Returns an iterator over the ValordMap.
    /// The iterator yields all items from start to end order by value.ord_by().
    ///
    /// # Example
    ///
    /// ```
    /// use valord_map::ValordMap;
    ///
    /// let mut valord = ValordMap::new();
    /// valord.insert("qians", 1);
    /// valord.insert("tedious", 2);
    /// valord.insert("xuandu", 3);
    /// valord.insert("xuandu", 1);
    ///
    /// let mut iter = valord.iter();
    ///
    /// assert_eq!(iter.next().unwrap().1, &1);
    /// assert_eq!(iter.next().unwrap().1, &1);
    /// assert_eq!(iter.next().unwrap(), (&"tedious", &2));
    /// ```
    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.sorted_indexs
            .iter()
            .flat_map(|(_, indexs)| indexs.iter().filter_map(|index| self.get_by_index(*index)))
    }

    /// Returns an reversesed iterator over the ValordMap.
    /// The iterator yields all items from start to end order by value.ord_by().
    ///
    /// # Example
    ///
    /// ```
    /// use valord_map::ValordMap;
    ///
    /// let mut valord = ValordMap::new();
    /// valord.insert("qians", 1);
    /// valord.insert("tedious", 2);
    /// valord.insert("xuandu", 3);
    /// valord.insert("xuandu", 1);
    ///
    /// let mut iter = valord.rev_iter();
    ///
    /// assert_eq!(iter.next().unwrap(), (&"tedious", &2));
    /// assert_eq!(iter.next().unwrap().1, &1);
    /// assert_eq!(iter.next().unwrap().1, &1);
    /// ```
    pub fn rev_iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.sorted_indexs
            .iter()
            .rev()
            .flat_map(|(_, indexs)| indexs.iter().filter_map(|index| self.get_by_index(*index)))
    }

    /// Returns an mut iterator over the ValordMap.
    /// The iterator yields all items from start to end order by value.ord_by().
    ///
    /// # Example
    ///
    /// ```
    /// use valord_map::ValordMap;
    ///
    /// let mut valord = ValordMap::new();
    /// valord.insert("qians", 1);
    /// valord.insert("tedious", 2);
    /// valord.insert("xuandu", 3);
    ///
    ///
    /// let mut iter = valord.iter_mut();
    ///
    /// let mut item1 = iter.next().unwrap();
    /// let (k, v) = item1.get_mut_with_key();
    /// assert_eq!(v, &mut 1);
    /// *v = 4;
    /// drop(item1);
    ///
    /// assert_eq!(iter.next().unwrap().get_mut_with_key(), (&"tedious", &mut 2));
    /// assert_eq!(iter.next().unwrap().get_mut_with_key(), (&"xuandu", &mut 3));
    /// assert!(iter.next().is_none());
    /// drop(iter);
    ///
    /// let max_list = valord.last();
    /// assert_eq!(max_list.len(), 1);
    /// assert_eq!(max_list, vec![(&"qians", &4)]);
    /// ```
    pub fn iter_mut(&mut self) -> impl Iterator<Item = RawEntry<'_, T, K, V>> {
        let indexs: Vec<_> = self
            .sorted_indexs
            .iter()
            .flat_map(|(_, indexs)| indexs.iter())
            .copied()
            .collect();
        let valord: *mut ValordMap<T, K, V> = self;
        indexs.into_iter().filter_map(move |index| {
            let vm = unsafe { valord.as_mut()? };
            vm.get_mut_by_index(index)
        })
    }

    /// Returns an reversesed mut iterator over the ValordMap.
    /// The iterator yields all items from start to end order by value.ord_by().
    ///
    /// # Example
    ///
    /// ```
    /// use valord_map::ValordMap;
    ///
    /// let mut valord = ValordMap::new();
    /// valord.insert("qians", 1);
    /// valord.insert("tedious", 2);
    /// valord.insert("xuandu", 3);
    ///
    ///
    /// let mut iter = valord.rev_iter_mut();
    ///
    /// let mut item1 = iter.next().unwrap();
    /// let (k, v) = item1.get_mut_with_key();
    /// assert_eq!(v, &mut 3);
    /// *v = 0;
    /// drop(item1);
    ///
    /// assert_eq!(iter.next().unwrap().get_mut_with_key(), (&"tedious", &mut 2));
    /// assert_eq!(iter.next().unwrap().get_mut_with_key(), (&"qians", &mut 1));
    /// assert!(iter.next().is_none());
    /// drop(iter);
    ///
    /// let max_list = valord.first();
    /// assert_eq!(max_list.len(), 1);
    /// assert_eq!(max_list, vec![(&"xuandu", &0)]);
    /// ```
    pub fn rev_iter_mut(&mut self) -> impl Iterator<Item = RawEntry<'_, T, K, V>> {
        let indexs: Vec<_> = self
            .sorted_indexs
            .iter()
            .rev()
            .flat_map(|(_, indexs)| indexs.iter())
            .copied()
            .collect();
        let valord: *mut ValordMap<T, K, V> = self;
        indexs.into_iter().filter_map(move |index| {
            let vm = unsafe { valord.as_mut()? };
            vm.get_mut_by_index(index)
        })
    }

    /// Returns the first vector of key-value pairs in the map. The value in this pair is the minimum values in the map.
    ///
    /// # Example
    ///
    /// ```
    /// use valord_map::ValordMap;
    ///
    /// let mut valord = ValordMap::new();
    /// valord.insert("qians", 1);
    /// valord.insert("tedious", 2);
    /// valord.insert("xuandu", 3);
    /// valord.insert("xuandu", 1);
    ///
    /// let min_list = valord.first();
    ///
    /// assert_eq!(min_list.len(), 2);
    /// assert!(min_list.iter().all(|(_, v)| **v == 1));
    /// ```
    pub fn first(&self) -> Vec<(&K, &V)> {
        self.sorted_indexs
            .first_key_value()
            .map(|(_, indexs)| self.iter_from_indexs(indexs).collect())
            .unwrap_or_default()
    }

    /// Returns the first vector of key-value mut pairs in the map. The value in this pair is the minimum values in the map.
    ///
    /// # Example
    ///
    /// ```
    /// use valord_map::ValordMap;
    ///
    /// let mut valord = ValordMap::new();
    /// valord.insert("qians", 1);
    /// valord.insert("tedious", 2);
    /// valord.insert("xuandu", 3);
    /// valord.insert("xuandu", 1);
    ///
    /// let mut min_list = valord.first_mut();
    /// assert_eq!(min_list.len(), 2);
    /// min_list.iter_mut().for_each(|entry| {
    ///     let (_k, v) = entry.get_mut_with_key();
    ///     *v = 0;
    /// });
    /// drop(min_list);
    ///
    /// let min_list = valord.first();
    /// assert!(min_list.iter().all(|(_, v)| **v == 0));
    /// ```
    pub fn first_mut(&mut self) -> Vec<RawEntry<'_, T, K, V>> {
        let valord: *mut ValordMap<T, K, V> = self;
        self.sorted_indexs
            .first_key_value()
            .map(|(_, indexs)| Self::iter_mut_from_indexs(valord, indexs.clone()).collect())
            .unwrap_or_default()
    }

    /// Returns the last vector of key-value pairs in the map. The value in this pair is the maximum values in the map.
    ///
    /// # Example
    ///
    /// ```
    /// use valord_map::ValordMap;
    ///
    /// let mut valord = ValordMap::new();
    /// valord.insert("qians", 1);
    /// valord.insert("tedious", 2);
    /// valord.insert("xuandu", 3);
    /// valord.insert("xuandu", 1);
    ///
    /// let max_list = valord.last();
    ///
    /// assert_eq!(max_list.len(), 1);
    /// assert_eq!(max_list, vec![(&"tedious", &2)]);
    /// ```
    pub fn last(&self) -> Vec<(&K, &V)> {
        self.sorted_indexs
            .last_key_value()
            .map(|(_, indexs)| self.iter_from_indexs(indexs).collect())
            .unwrap_or_default()
    }

    /// Returns the last vector of key-value mut pairs in the map. The value in this pair is the minimum values in the map.
    ///
    /// # Example
    ///
    /// ```
    /// use valord_map::ValordMap;
    ///
    /// let mut valord = ValordMap::new();
    /// valord.insert("qians", 1);
    /// valord.insert("tedious", 2);
    /// valord.insert("xuandu", 3);
    /// valord.insert("sheng", 4);
    ///
    /// let mut max_list = valord.last_mut();
    /// assert_eq!(max_list.len(), 1);
    /// let (k, v) = max_list[0].get_mut_with_key();
    /// assert_eq!((&k, &v), (&&"sheng", &&mut 4));
    ///
    /// *v = 2;
    /// drop(max_list);
    ///
    /// let max_list = valord.last();
    /// assert_eq!(max_list.len(), 1);
    /// assert_eq!(max_list, vec![(&"xuandu", &3)]);
    /// ```
    pub fn last_mut(&mut self) -> Vec<RawEntry<'_, T, K, V>> {
        let valord: *mut ValordMap<T, K, V> = self;
        self.sorted_indexs
            .last_key_value()
            .map(|(_, indexs)| Self::iter_mut_from_indexs(valord, indexs.clone()).collect())
            .unwrap_or_default()
    }

    /// get range from ValordMap
    ///
    /// # Example
    ///
    /// ```
    /// use valord_map::ValordMap;
    ///
    /// let mut valord = ValordMap::new();
    /// valord.insert("qians", 1);
    /// valord.insert("tedious", 2);
    /// valord.insert("sheng", 3);
    /// valord.insert("xuandu", 4);
    /// valord.insert("xuandu2", 5);
    /// valord.insert("xuandu3", 6);
    /// assert_eq!(valord.range(4..).last().unwrap(), (&"xuandu3", &6));
    /// assert_eq!(
    ///     valord
    ///         .range(4..)
    ///         .filter(|(_, v)| **v != 6)
    ///         .last()
    ///         .unwrap(),
    ///     (&"xuandu2", &5)
    /// );
    /// ```
    pub fn range<R>(&self, range: R) -> impl Iterator<Item = (&K, &V)>
    where
        R: std::ops::RangeBounds<V::Target>,
    {
        self.sorted_indexs
            .range(range)
            .flat_map(|(_, indexs)| self.iter_from_indexs(indexs))
    }

    /// get range mut from ValordMap
    ///
    /// # Example
    ///
    /// ```
    /// use valord_map::ValordMap;
    ///
    /// let mut valord = ValordMap::new();
    /// valord.insert("qians", 1);
    /// valord.insert("tedious", 2);
    /// valord.insert("sheng", 3);
    /// valord.insert("xuandu", 4);
    /// valord.insert("xuandu2", 5);
    /// valord.insert("xuandu3", 6);
    ///
    /// let mut range_iter = valord.range_mut(4..);
    ///
    /// let mut item1 = range_iter.next().unwrap();
    /// let (k, v) = item1.get_mut_with_key();
    /// assert_eq!(k, &"xuandu");
    /// assert_eq!(v, &mut 4);
    /// *v += 4;
    /// drop(item1);
    /// drop(range_iter);
    ///
    /// assert_eq!(
    ///     valord
    ///         .range(4..)
    ///         .last(),
    ///     Some((&"xuandu", &8))
    /// );
    /// ```
    pub fn range_mut<R>(&mut self, range: R) -> impl Iterator<Item = RawEntry<'_, T, K, V>>
    where
        R: std::ops::RangeBounds<V::Target>,
    {
        let range: Vec<_> = self
            .sorted_indexs
            .range(range)
            .flat_map(|(_, indexs)| indexs.iter())
            .copied()
            .collect();
        let valord: *mut ValordMap<T, K, V> = self;
        range.into_iter().filter_map(move |index| {
            let vm = unsafe { valord.as_mut()? };
            vm.get_mut_by_index(index)
        })
    }

    /// Get the ref value by given key, or return `None` if not found
    ///
    /// # Example
    ///
    /// ```
    /// use valord_map::ValordMap;
    ///
    /// let mut valord = ValordMap::new();
    /// valord.insert("key1", 1);
    /// valord.insert("key2", 2);
    /// valord.insert("key3", 3);
    ///
    /// let mut val1 = valord.get(&"key2");
    /// let mut val2 = valord.get(&"key4");
    /// assert_eq!(val1.unwrap(), &2);
    /// assert_eq!(val2, None);
    /// ```
    pub fn get(&self, key: &K) -> Option<&V> {
        self.map.get(key).and_then(|v| v.as_ref())
    }

    /// Get the ref mut value by given key, or return `None` if not found
    ///
    /// # Example
    ///
    /// ```
    /// use valord_map::ValordMap;
    ///
    /// let mut valord = ValordMap::new();
    /// valord.insert("key1", 1);
    /// valord.insert("key2", 2);
    /// valord.insert("key3", 3);
    ///
    /// let mut val = valord.get_mut(&"key2").unwrap();
    /// *val = 4;
    /// drop(val);
    /// assert_eq!(valord.get(&"key2").unwrap(), &4);
    /// assert_eq!(valord.last(), vec![(&"key2", &4)]);
    /// ```
    pub fn get_mut<'a>(&'a mut self, key: &K) -> Option<RawEntry<'a, T, K, V>> {
        RawEntry::try_new_by_key(self, key)
    }

    /// Modify value in map, if exist return true, else return false
    ///
    /// # Example
    ///
    /// ```
    /// use valord_map::ValordMap;
    ///
    /// let mut valord = ValordMap::new();
    /// valord.insert("qians", 1);
    ///
    /// assert!(valord.modify(&"qians", |v| *v = 2));
    /// assert_eq!(valord.iter().next().unwrap(), (&"qians", &2));
    /// ```
    pub fn modify<F>(&mut self, key: &K, op: F) -> bool
    where
        F: Fn(&mut V),
    {
        if let Some((index, _, v)) = Self::get_full_mut(&mut self.map, key) {
            Self::remove_from_indexs(&mut self.sorted_indexs, &v.ord_by(), index);
            op(v);
            self.sorted_indexs
                .entry(v.ord_by())
                .or_default()
                .insert(index);
            true
        } else {
            false
        }
    }

    /// remove from ValordMap
    ///
    /// # Example
    ///
    /// ```
    /// use valord_map::ValordMap;
    ///
    /// let mut valord = ValordMap::new();
    /// valord.insert(1, "a");
    /// valord.insert(2, "b");
    ///
    /// let removed_value = valord.remove(&1);
    /// assert_eq!(removed_value, Some("a"));
    /// assert_eq!(valord.get(&1), None);
    /// ```
    pub fn remove(&mut self, key: &K) -> Option<V> {
        self.remove_entry(key).map(|v| v.1)
    }

    /// Removes a key from the map, returning the stored key and value if the
    /// key was previously in the map.
    ///
    /// # Example
    ///
    /// ```
    /// use valord_map::ValordMap;
    ///
    /// let mut valord = ValordMap::new();
    /// valord.insert(1, "a");
    /// valord.insert(2, "b");
    ///
    /// let removed_entry = valord.remove_entry(&1);
    /// assert_eq!(removed_entry, Some((&1, "a")));
    /// assert_eq!(valord.get(&1), None);
    /// ```
    pub fn remove_entry<'a>(&'a mut self, key: &'a K) -> Option<(&K, V)> {
        if let Some((i, k, v)) = self.map.get_full_mut(key) {
            if let Some(old) = v.take() {
                self.free_indexs.push_back(i);
                Self::remove_from_indexs(&mut self.sorted_indexs, &old.ord_by(), i);
                return Some((k, old));
            };
        }
        None
    }

    /// Return the number of key-value pairs in the map.
    ///
    /// # Example
    ///
    /// ```
    /// use valord_map::ValordMap;
    ///
    /// let mut valord = ValordMap::new();
    /// valord.insert(1, "a");
    /// valord.insert(2, "b");
    /// valord.insert(3, "c");
    /// valord.insert(2, "d");
    ///
    /// assert_eq!(valord.len(), 3);
    ///
    /// let removed_value = valord.remove(&1);
    /// assert_eq!(removed_value, Some("a"));
    /// assert_eq!(valord.len(), 2);
    /// ```
    pub fn len(&self) -> usize {
        self.map.len() - self.free_indexs.len()
    }

    /// Re-order the ValordMap by value.ord_by().
    ///
    /// # Example
    ///
    /// ```
    /// use std::cell::Cell;
    /// use valord_map::ValordMap;
    ///
    /// let mut valord = ValordMap::new();
    /// valord.insert("qians", Cell::new(1));
    /// valord.insert("tedious", Cell::new(2));
    /// valord.insert("xuandu", Cell::new(3));
    ///
    /// valord
    ///     .iter()
    ///     .enumerate()
    ///     .for_each(|(i, (_, v))| v.set(5 - i));
    ///
    /// assert_eq!(
    ///     valord.iter().collect::<Vec<_>>(),
    ///     vec![
    ///         (&"qians", &Cell::new(5)),
    ///         (&"tedious", &Cell::new(4)),
    ///         (&"xuandu", &Cell::new(3))
    ///     ]
    /// );
    ///
    /// valord.re_order();
    ///
    /// assert_eq!(
    ///     valord.iter().collect::<Vec<_>>(),
    ///     vec![
    ///         (&"xuandu", &Cell::new(3)),
    ///         (&"tedious", &Cell::new(4)),
    ///         (&"qians", &Cell::new(5)),
    ///     ]
    /// );
    /// ```
    pub fn re_order(&mut self) {
        let mut sorted = BTreeMap::<T, HashSet<usize>>::new();
        self.map
            .iter()
            .enumerate()
            .filter_map(|(i, (_, v))| v.as_ref().map(|v| (v.ord_by(), i)))
            .for_each(|(t, i)| {
                sorted.entry(t).or_default().insert(i);
            });
        self.sorted_indexs = sorted;
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn get_by_index(&self, index: usize) -> Option<(&K, &V)> {
        self.map
            .get_index(index)
            .and_then(|(k, maybe_val)| maybe_val.as_ref().map(|v| (k, v)))
    }

    fn get_mut_by_index(&mut self, index: usize) -> Option<RawEntry<'_, T, K, V>> {
        RawEntry::try_new_by_index(self, index)
    }

    fn get_full_mut<'a>(
        map: &'a mut IndexMap<K, Option<V>>,
        key: &'a K,
    ) -> Option<(usize, &'a K, &'a mut V)> {
        map.get_full_mut(key)
            .and_then(|(i, k, v)| v.as_mut().map(|v| (i, k, v)))
    }

    fn iter_from_indexs<'a>(
        &'a self,
        indexs: &'a HashSet<usize>,
    ) -> impl Iterator<Item = (&K, &V)> {
        indexs.iter().filter_map(|index| self.get_by_index(*index))
    }

    fn iter_mut_from_indexs<'a>(
        valord: *mut ValordMap<T, K, V>,
        indexs: HashSet<usize>,
    ) -> impl Iterator<Item = RawEntry<'a, T, K, V>>
    where
        T: 'a,
        K: 'a,
        V: 'a,
    {
        indexs.into_iter().filter_map(move |index| {
            let vm = unsafe { valord.as_mut()? };
            vm.get_mut_by_index(index)
        })
    }

    fn remove_from_indexs(sorted_indexs: &mut BTreeMap<T, HashSet<usize>>, key: &T, index: usize) {
        if let Some(indexs) = sorted_indexs.get_mut(key) {
            indexs.remove(&index);
            if indexs.is_empty() {
                sorted_indexs.remove(key);
            }
        }
    }
}

impl<T, K, V> Default for ValordMap<T, K, V>
where
    T: Ord + Clone,
    K: std::hash::Hash + Eq,
    V: OrdBy<Target = T>,
{
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use super::*;

    #[derive(Debug, PartialEq, Eq)]
    struct OrdByValue {
        sth: usize,
        order_by: usize,
    }

    impl OrdByValue {
        fn new(sth: usize, order_by: usize) -> Self {
            Self { sth, order_by }
        }
    }

    impl OrdBy for OrdByValue {
        type Target = usize;

        fn ord_by(&self) -> Self::Target {
            self.order_by
        }
    }

    #[test]
    fn test_valord_insert_order_by() {
        let mut valord = ValordMap::new();
        valord.insert("qians", OrdByValue::new(123, 1));
        valord.insert("tedious", OrdByValue::new(412, 2));
        valord.insert("xuandu", OrdByValue::new(125, 3));
        valord.insert("xuandu", OrdByValue::new(938, 1));

        let sorted_pairs: Vec<_> = valord.iter().collect();

        assert_eq!(sorted_pairs.len(), 3);
        assert_eq!(sorted_pairs[0].1.order_by, 1);
        assert_eq!(sorted_pairs[1].1.order_by, 1);
        assert_eq!(sorted_pairs[2], (&"tedious", &OrdByValue::new(412, 2)));
    }

    #[test]
    fn test_valord_remove_non_existent() {
        let mut valord = ValordMap::new();
        valord.insert(1, "a");
        valord.insert(2, "b");

        let removed_value = valord.remove(&3);
        assert_eq!(removed_value, None);
        assert_eq!(valord.get(&3), None);
    }

    #[test]
    fn test_valord_multiple_insert_and_remove() {
        let mut valord = ValordMap::new();
        valord.insert("qians", 1);
        valord.insert("tedious", 2);
        valord.insert("xuandu", 3);

        assert_eq!(valord.remove(&"tedious"), Some(2));
        assert_eq!(valord.remove(&"qians"), Some(1));

        valord.insert("x", 2);
        valord.insert("y", 4);

        let sorted_pairs: Vec<_> = valord.iter().collect();
        println!("sorted_pairs: {sorted_pairs:?}");
        assert_eq!(sorted_pairs.len(), 3);
        assert_eq!(sorted_pairs[0], (&"x", &2));
        assert_eq!(sorted_pairs[1], (&"xuandu", &3));
        assert_eq!(sorted_pairs[2], (&"y", &4));
    }

    #[test]
    fn re_order() {
        let mut valord = ValordMap::new();
        valord.insert("qians", Cell::new(1));
        valord.insert("tedious", Cell::new(2));
        valord.insert("xuandu", Cell::new(3));

        valord
            .iter()
            .enumerate()
            .for_each(|(i, (_, v))| v.set(5 - i));

        assert_eq!(
            valord.iter().collect::<Vec<_>>(),
            vec![
                (&"qians", &Cell::new(5)),
                (&"tedious", &Cell::new(4)),
                (&"xuandu", &Cell::new(3))
            ]
        );

        valord.re_order();

        assert_eq!(
            valord.iter().collect::<Vec<_>>(),
            vec![
                (&"xuandu", &Cell::new(3)),
                (&"tedious", &Cell::new(4)),
                (&"qians", &Cell::new(5)),
            ]
        );
    }
}
