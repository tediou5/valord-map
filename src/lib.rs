#![doc = "no_run"]
#![doc = include_str!("../README.md")]
#![doc(html_playground_url = "https://play.rust-lang.org")]

mod order_by;
pub use order_by::OrdBy;
pub(crate) use order_by::ValueOrdBy;

mod watcher;
pub use watcher::Watcher;

use std::{
    collections::{BTreeMap, HashMap, HashSet},
    sync::Arc,
};
use tokio::sync::watch;
use tracing::trace;

pub struct ValordMap<T, K, V: OrdBy<Target = T>> {
    map: HashMap<Arc<K>, ValueOrdBy<V>>,
    sorted_keys: BTreeMap<T, HashSet<Arc<K>>>,
    sender: watch::Sender<Option<Arc<V>>>,
}

impl<T, K, V> ValordMap<T, K, V>
where
    T: Ord + Clone,
    K: std::hash::Hash + Eq,
    V: OrdBy<Target = T>,
{
    pub fn new() -> Self {
        ValordMap {
            map: HashMap::new(),
            sorted_keys: BTreeMap::new(),
            sender: watch::channel(None).0,
        }
    }

    /// Watch a key, trigger a notification when the maximum value changes.
    ///
    /// Example
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
    /// Example
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
        let key: Arc<_> = key.into();
        self._insert(key, value)
    }

    fn _insert(&mut self, key: Arc<K>, value: V) {
        let ord_by = value.ord_by();

        let mut changed = true;
        if let Some((curr_head, _)) = self.sorted_keys.last_key_value() {
            if curr_head >= ord_by {
                changed = false
            }
        };

        self.remove(&key);
        self.sorted_keys
            .entry(ord_by.clone())
            .or_default()
            .insert(key.clone());
        let value: ValueOrdBy<V> = value.into();
        if changed {
            trace!("head changed");
            let _ = self.sender.send(Some(value.clone().0));
        }
        self.map.insert(key, value);
    }

    /// Returns an iterator over the ValordMap.
    /// The iterator yields all items from start to end order by value.ord_by().
    ///
    /// Example
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
        self.sorted_keys.iter().flat_map(|(_, keys)| {
            keys.iter().map(|key| {
                let k = key.as_ref();
                let v = self.map[k].as_ref();
                (k, v)
            })
        })
    }

    /// Returns the first vector of key-value pairs in the map. The value in this pair is the minimum values in the map.
    ///
    /// Example
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
        self.sorted_keys
            .first_key_value()
            .map(|(_, keys)| {
                keys.iter()
                    .map(|key| {
                        let k = key.as_ref();
                        let v = self.map[k].as_ref();
                        (k, v)
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Returns the last vector of key-value pairs in the map. The value in this pair is the maximum values in the map.
    ///
    /// Example
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
        self.sorted_keys
            .last_key_value()
            .map(|(_, keys)| {
                keys.iter()
                    .map(|key| {
                        let k = key.as_ref();
                        let v = self.map[k].as_ref();
                        (k, v)
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// remove from ValordMap
    ///
    /// Example
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
        self.sorted_keys.range(range).flat_map(|(_, keys)| {
            keys.iter().map(|key| {
                let k = key.as_ref();
                let v = self.map[k].as_ref();
                (k, v)
            })
        })
    }

    pub fn get(&self, key: &K) -> Option<&V> {
        self.map.get(key).map(|v| v.as_ref())
    }

    // TODO: move to entry
    /// Modify value in map, if exist return true, else return false
    ///
    /// Example
    ///
    /// ```
    /// use valord_map::ValordMap;
    ///
    /// let mut sorted_map = ValordMap::new();
    /// sorted_map.insert("qians", 1);
    ///
    /// assert!(sorted_map.modify(&"qians", |k, v| *v = 2));
    /// assert_eq!(sorted_map.iter().next().unwrap(), (&"qians", &2));
    /// ```
    pub fn modify<F>(&mut self, key: &K, op: F) -> bool
    where
        F: Fn(&K, &mut V),
    {
        if let Some((k, mut v)) = self.remove_entry(key) {
            op(&k, &mut v);
            self._insert(k, v);
            true
        } else {
            false
        }
    }

    /// remove from ValordMap
    ///
    /// Example
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
        if let Some(value) = self.map.remove(key) {
            let ord_by: &T = value.as_ref().ord_by();
            if let Some(keys) = self.sorted_keys.get_mut(ord_by) {
                keys.remove(key);
                if keys.is_empty() {
                    self.sorted_keys.remove(ord_by);
                }
            }
            return value.into_inner();
        };
        None
    }

    /// Removes a key from the map, returning the stored key and value if the
    /// key was previously in the map.
    ///
    /// Example
    ///
    /// ```
    /// use valord_map::ValordMap;
    ///
    /// let mut sorted_map = ValordMap::new();
    /// sorted_map.insert(1, "a");
    /// sorted_map.insert(2, "b");
    ///
    /// let removed_entry = sorted_map.remove_entry(&1);
    /// assert_eq!(removed_entry, Some((std::sync::Arc::new(1), "a")));
    /// assert_eq!(sorted_map.get(&1), None);
    /// ```
    pub fn remove_entry(&mut self, key: &K) -> Option<(Arc<K>, V)> {
        if let Some((k, val)) = self.map.remove_entry(key) {
            let ord_by: &T = val.as_ref().ord_by();
            if let Some(keys) = self.sorted_keys.get_mut(ord_by) {
                keys.remove(key);
                if keys.is_empty() {
                    self.sorted_keys.remove(ord_by);
                }
            }
            return Some((k, val.into_inner()?));
        };
        None
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

    use std::time::Duration;

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

        fn ord_by<'a>(&'a self) -> &Self::Target {
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

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn watch() {
        let mut sorted_map = ValordMap::new();
        let mut watcher = sorted_map.watcher();
        let handle = tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(1)).await;
            sorted_map.insert("qians", 1);
            tokio::time::sleep(Duration::from_secs(1)).await;
            sorted_map.insert("tedious", 2);
            tokio::time::sleep(Duration::from_secs(1)).await;
            sorted_map.insert("sheng", 3);
            tokio::time::sleep(Duration::from_secs(1)).await;
            sorted_map.insert("xuandu", 4);
            tokio::time::sleep(Duration::from_secs(1)).await;
            sorted_map.insert("xuandu2", 5);
            tokio::time::sleep(Duration::from_secs(1)).await;
            sorted_map.insert("xuandu3", 6);
        });

        println!("watching...");
        for v in 1..=6 {
            let header = watcher.head_changed().await.unwrap().unwrap();
            assert_eq!(&v, header.as_ref());
        }

        let _ = handle.await;
    }
}
