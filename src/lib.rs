#![doc = include_str!("../README.md")]
#![doc(html_playground_url = "https://play.rust-lang.org")]

mod order_by;
use indexmap::IndexMap;
pub use order_by::OrdBy;

mod watcher;
pub use watcher::Watcher;

use std::{
    collections::{BTreeMap, HashSet},
    hash::Hash,
    ops::{Deref, DerefMut},
    sync::Arc,
};
use tokio::sync::watch;
use tracing::trace;

pub struct ValordMap<T, K, V: OrdBy<Target = T>> {
    map: IndexMap<K, Option<V>>,
    sorted_indexs: BTreeMap<T, HashSet<usize>>,
    sender: watch::Sender<Option<Arc<V>>>,
}

pub struct RefMut<'a, T, K, V>
where
    T: Ord + Clone,
    K: Hash + Eq,
    V: OrdBy<Target = T>,
{
    // key: &'k K,
    index: usize,
    valord: &'a mut ValordMap<T, K, V>,
}

impl<'a, T, K, V> RefMut<'a, T, K, V>
where
    T: Ord + Clone,
    K: Hash + Eq,
    V: OrdBy<Target = T>,
{
    fn tyr_new(valord: &'a mut ValordMap<T, K, V>, key: &K) -> Option<RefMut<'a, T, K, V>> {
        // TODO:
        let (index, _, v) = valord.map.get_full(key)?;
        let ord_by = v.as_ref().map(|v| v.ord_by())?;
        ValordMap::<T, K, V>::remove_from_indexs(&mut valord.sorted_indexs, ord_by, index);
        Some(Self { index, valord })
    }
}

impl<'a, T, K, V> Deref for RefMut<'a, T, K, V>
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

impl<'a, T, K, V> DerefMut for RefMut<'a, T, K, V>
where
    T: Ord + Clone,
    K: Hash + Eq,
    V: OrdBy<Target = T>,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        // Safety: if value is not exist, try_new() will return None
        let v = self
            .valord
            .map
            .get_index_mut(self.index)
            .and_then(|(_, maybe_val)| maybe_val.as_mut())
            .unwrap();
        ValordMap::<T, K, V>::remove_from_indexs(
            &mut self.valord.sorted_indexs,
            v.ord_by(),
            self.index,
        );
        v
    }
}

impl<'a, T, K, V> Drop for RefMut<'a, T, K, V>
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
            sender: watch::channel(None).0,
        }
    }

    /// Watch a key, trigger a notification when the maximum value changes.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use valord_map::ValordMap;
    /// use std::time::Duration;
    ///
    ///#[tokio::main]
    ///async fn main() {
    ///    let mut sorted_map = ValordMap::new();
    ///    let mut watcher = sorted_map.watcher();
    ///    let handle = tokio::spawn(async move {
    ///        tokio::time::sleep(Duration::from_secs(1)).await;
    ///        sorted_map.insert("qians", 1);
    ///        tokio::time::sleep(Duration::from_secs(1)).await;
    ///        sorted_map.insert("tedious", 2);
    ///        tokio::time::sleep(Duration::from_secs(1)).await;
    ///        sorted_map.insert("sheng", 3);
    ///        tokio::time::sleep(Duration::from_secs(1)).await;
    ///        sorted_map.insert("xuandu", 4);
    ///        tokio::time::sleep(Duration::from_secs(1)).await;
    ///        sorted_map.insert("xuandu2", 5);
    ///        tokio::time::sleep(Duration::from_secs(1)).await;
    ///        sorted_map.insert("xuandu3", 6);
    ///    });
    ///
    ///    println!("watching...");
    ///    for v in 1..=6 {
    ///        let header = watcher.head_changed().await.unwrap().unwrap();
    ///        assert_eq!(&v, header.as_ref());
    ///    }
    ///
    ///   let _ = handle.await;
    /// }
    /// ```
    pub fn watcher(&self) -> Watcher<V> {
        self.sender.subscribe().into()
    }

    /// insert into ValordMap
    ///
    /// # Example
    ///
    /// ```
    /// use valord_map::ValordMap;
    ///
    /// let mut sorted_map = ValordMap::new();
    /// sorted_map.insert("qians", 1);
    /// sorted_map.insert("tedious", 2);
    /// sorted_map.insert("xuandu", 3);
    /// sorted_map.insert("xuandu", 1);
    ///
    /// let sorted_pairs: Vec<_> = sorted_map.iter().collect();
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
        let ord_by = value.ord_by().clone();

        let mut changed = true;
        if let Some((curr_head, _)) = self.sorted_indexs.last_key_value() {
            if curr_head >= &ord_by {
                changed = false
            }
        };

        let (index, old_val) = self.map.insert_full(key, Some(value));
        if let Some(old_val) = old_val.flatten() {
            Self::remove_from_indexs(&mut self.sorted_indexs, old_val.ord_by(), index)
        }

        self.sorted_indexs.entry(ord_by).or_default().insert(index);

        if changed {
            // TODO:
            trace!("head changed");
            // let _ = self.sender.send(Some(value.clone().0));
        }
    }

    /// Returns an iterator over the ValordMap.
    /// The iterator yields all items from start to end order by value.ord_by().
    ///
    /// # Example
    ///
    /// ```
    /// use valord_map::ValordMap;
    ///
    /// let mut sorted_map = ValordMap::new();
    /// sorted_map.insert("qians", 1);
    /// sorted_map.insert("tedious", 2);
    /// sorted_map.insert("xuandu", 3);
    /// sorted_map.insert("xuandu", 1);
    ///
    /// let mut iter = sorted_map.iter();
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

    /// Returns the first vector of key-value pairs in the map. The value in this pair is the minimum values in the map.
    ///
    /// # Example
    ///
    /// ```
    /// use valord_map::ValordMap;
    ///
    /// let mut sorted_map = ValordMap::new();
    /// sorted_map.insert("qians", 1);
    /// sorted_map.insert("tedious", 2);
    /// sorted_map.insert("xuandu", 3);
    /// sorted_map.insert("xuandu", 1);
    ///
    /// let min_list = sorted_map.first();
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

    /// Returns the last vector of key-value pairs in the map. The value in this pair is the maximum values in the map.
    ///
    /// # Example
    ///
    /// ```
    /// use valord_map::ValordMap;
    ///
    /// let mut sorted_map = ValordMap::new();
    /// sorted_map.insert("qians", 1);
    /// sorted_map.insert("tedious", 2);
    /// sorted_map.insert("xuandu", 3);
    /// sorted_map.insert("xuandu", 1);
    ///
    /// let max_list = sorted_map.last();
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

    /// remove from ValordMap
    ///
    /// # Example
    ///
    /// ```
    /// use valord_map::ValordMap;
    ///
    /// let mut sorted_map = ValordMap::new();
    /// sorted_map.insert("qians", 1);
    /// sorted_map.insert("tedious", 2);
    /// sorted_map.insert("sheng", 3);
    /// sorted_map.insert("xuandu", 4);
    /// sorted_map.insert("xuandu2", 5);
    /// sorted_map.insert("xuandu3", 6);
    /// assert_eq!(sorted_map.range(4..).last().unwrap(), (&"xuandu3", &6));
    /// assert_eq!(
    ///     sorted_map
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
    /// let mut sorted_map = ValordMap::new();
    /// sorted_map.insert("key1", 1);
    /// sorted_map.insert("key2", 2);
    /// sorted_map.insert("key3", 3);
    ///
    /// let mut val = sorted_map.get_mut(&"key2").unwrap();
    /// *val = 4;
    /// drop(val);
    /// assert_eq!(sorted_map.get(&"key2").unwrap(), &4);
    /// assert_eq!(sorted_map.last(), vec![(&"key2", &4)]);
    /// ```
    pub fn get_mut<'a>(&'a mut self, key: &K) -> Option<RefMut<'a, T, K, V>> {
        RefMut::tyr_new(self, key)
    }

    /// Modify value in map, if exist return true, else return false
    ///
    /// # Example
    ///
    /// ```
    /// use valord_map::ValordMap;
    ///
    /// let mut sorted_map = ValordMap::new();
    /// sorted_map.insert("qians", 1);
    ///
    /// assert!(sorted_map.modify(&"qians", |v| *v = 2));
    /// assert_eq!(sorted_map.iter().next().unwrap(), (&"qians", &2));
    /// ```
    pub fn modify<F>(&mut self, key: &K, op: F) -> bool
    where
        F: Fn(&mut V),
    {
        if let Some((index, _, v)) = Self::get_full_mut(&mut self.map, key) {
            Self::remove_from_indexs(&mut self.sorted_indexs, v.ord_by(), index);
            op(v);
            self.sorted_indexs
                .entry(v.ord_by().clone())
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
    /// let mut sorted_map = ValordMap::new();
    /// sorted_map.insert(1, "a");
    /// sorted_map.insert(2, "b");
    ///
    /// let removed_value = sorted_map.remove(&1);
    /// assert_eq!(removed_value, Some("a"));
    /// assert_eq!(sorted_map.get(&1), None);
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
    /// let mut sorted_map = ValordMap::new();
    /// sorted_map.insert(1, "a");
    /// sorted_map.insert(2, "b");
    ///
    /// let removed_entry = sorted_map.remove_entry(&1);
    /// assert_eq!(removed_entry, Some((&1, "a")));
    /// assert_eq!(sorted_map.get(&1), None);
    /// ```
    pub fn remove_entry<'a>(&'a mut self, key: &'a K) -> Option<(&K, V)> {
        if let Some((i, k, v)) = self.map.get_full_mut(key) {
            if let Some(old) = v.take() {
                Self::remove_from_indexs(&mut self.sorted_indexs, old.ord_by(), i);
                return Some((k, old));
            };
        }
        None
    }

    fn get_by_index(&self, index: usize) -> Option<(&K, &V)> {
        self.map
            .get_index(index)
            .and_then(|(k, maybe_val)| maybe_val.as_ref().map(|v| (k, v)))
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

    // fn iter_mut_from_indexs<'a>(
    //     &'a mut self,
    //     indexs: &'a HashSet<usize>,
    // ) -> impl Iterator<Item = (&K, &'a mut V)> {
    //     indexs.iter().filter_map(|index| self.get_mut_by_index(*index))
    // }

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
    use super::*;

    // use std::time::Duration;

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

        fn ord_by(&self) -> &Self::Target {
            &self.order_by
        }
    }

    #[test]
    fn test_sorted_map_insert_order_by() {
        let mut sorted_map = ValordMap::new();
        sorted_map.insert("qians", OrdByValue::new(123, 1));
        sorted_map.insert("tedious", OrdByValue::new(412, 2));
        sorted_map.insert("xuandu", OrdByValue::new(125, 3));
        sorted_map.insert("xuandu", OrdByValue::new(938, 1));

        let sorted_pairs: Vec<_> = sorted_map.iter().collect();

        assert_eq!(sorted_pairs.len(), 3);
        assert_eq!(sorted_pairs[0].1.order_by, 1);
        assert_eq!(sorted_pairs[1].1.order_by, 1);
        assert_eq!(sorted_pairs[2], (&"tedious", &OrdByValue::new(412, 2)));
    }

    #[test]
    fn test_sorted_map_remove_non_existent() {
        let mut sorted_map = ValordMap::new();
        sorted_map.insert(1, "a");
        sorted_map.insert(2, "b");

        let removed_value = sorted_map.remove(&3);
        assert_eq!(removed_value, None);
        assert_eq!(sorted_map.get(&3), None);
    }

    #[test]
    fn test_sorted_map_multiple_insert_and_remove() {
        let mut sorted_map = ValordMap::new();
        sorted_map.insert("qians", 1);
        sorted_map.insert("tedious", 2);
        sorted_map.insert("xuandu", 3);

        assert_eq!(sorted_map.remove(&"tedious"), Some(2));
        assert_eq!(sorted_map.remove(&"qians"), Some(1));

        sorted_map.insert("x", 2);
        sorted_map.insert("y", 4);

        let sorted_pairs: Vec<_> = sorted_map.iter().collect();
        // println!("sorted_map: {sorted_map:?}");
        println!("sorted_pairs: {sorted_pairs:?}");
        assert_eq!(sorted_pairs.len(), 3);
        assert_eq!(sorted_pairs[0], (&"x", &2));
        assert_eq!(sorted_pairs[1], (&"xuandu", &3));
        assert_eq!(sorted_pairs[2], (&"y", &4));
    }

    // #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    // async fn watch() {
    //     let mut sorted_map = ValordMap::new();
    //     let mut watcher = sorted_map.watcher();
    //     let handle = tokio::spawn(async move {
    //         tokio::time::sleep(Duration::from_secs(1)).await;
    //         sorted_map.insert("qians", 1);
    //         tokio::time::sleep(Duration::from_secs(1)).await;
    //         sorted_map.insert("tedious", 2);
    //         tokio::time::sleep(Duration::from_secs(1)).await;
    //         sorted_map.insert("sheng", 3);
    //         tokio::time::sleep(Duration::from_secs(1)).await;
    //         sorted_map.insert("xuandu", 4);
    //         tokio::time::sleep(Duration::from_secs(1)).await;
    //         sorted_map.insert("xuandu2", 5);
    //         tokio::time::sleep(Duration::from_secs(1)).await;
    //         sorted_map.insert("xuandu3", 6);
    //     });

    //     println!("watching...");
    //     for v in 1..=6 {
    //         let header = watcher.head_changed().await.unwrap().unwrap();
    //         assert_eq!(&v, header.as_ref());
    //     }

    //     let _ = handle.await;
    // }
}
