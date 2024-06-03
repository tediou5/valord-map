mod raw;
pub use raw::RawEntry;

use crate::OrdBy;
use std::hash::Hash;

/// Entry for an existing key-value pair in an [`ValordMap`][crate::ValordMap]
/// or a vacant location to insert one.
pub enum Entry<'v, T, K, V>
where
    T: Ord + Clone,
    K: Hash + Eq,
    V: OrdBy<Target = T>,
{
    /// Existing slot with equivalent key.
    Occupied(RawEntry<'v, T, K, V>),
    /// Vacant slot (no equivalent key in the map).
    Vacant(RawEntry<'v, T, K, V>),
}

impl<'v, T, K, V> Entry<'v, T, K, V>
where
    T: Ord + Clone,
    K: Hash + Eq,
    V: OrdBy<Target = T>,
{
    /// Inserts `default` value if the entry is vacant, and returns a mutable reference to the value.
    ///
    /// # Examples
    ///
    /// ```
    /// use valord_map::ValordMap;
    ///
    /// let mut map = ValordMap::new();
    /// map.entry("key").or_insert("value");
    ///
    /// assert_eq!(map.get(&"key"), Some(&"value"));
    /// ```
    pub fn or_insert(&mut self, default: V) -> &mut V {
        match self {
            Entry::Occupied(entry) => entry.get_mut_with_key().1,
            Entry::Vacant(entry) => {
                entry.valord.free_indexs.pop_front();
                entry.insert(default)
            }
        }
    }

    /// Inserts a value produced by the function `default` if the entry is vacant,
    /// and returns a mutable reference to the value.
    ///
    /// # Examples
    ///
    /// ```
    /// use valord_map::ValordMap;
    ///
    /// let mut map = ValordMap::new();
    /// map.entry("key").or_insert_with(|| "value");
    ///
    /// assert_eq!(map.get(&"key"), Some(&"value"));
    /// ```
    pub fn or_insert_with<F: FnOnce() -> V>(&mut self, default: F) -> &mut V {
        match self {
            Entry::Occupied(entry) => entry.get_mut_with_key().1,
            Entry::Vacant(entry) => {
                entry.valord.free_indexs.pop_front();
                entry.insert(default())
            }
        }
    }

    /// Inserts a value produced by the function `default` with the given key if the entry is vacant,
    /// and returns a mutable reference to the value.
    ///
    /// # Examples
    ///
    /// ```
    /// use valord_map::ValordMap;
    ///
    /// let mut map = ValordMap::new();
    /// map.entry("key").or_insert_with_key(|key| format!("value for {}", key));
    ///
    /// assert_eq!(map.get(&"key"), Some(&"value for key".to_string()));
    /// ```
    pub fn or_insert_with_key<F: FnOnce(&K) -> V>(&mut self, default: F) -> &mut V {
        match self {
            Entry::Occupied(entry) => entry.get_mut_with_key().1,
            Entry::Vacant(entry) => {
                entry.valord.free_indexs.pop_front();
                entry.insert_with_key(default)
            }
        }
    }

    /// Modifies the entry if it is occupied with the function `f`, and returns the entry.
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
    pub fn and_modify<F>(mut self, f: F) -> Self
    where
        F: FnOnce(&mut V),
    {
        if let Entry::Occupied(entry) = &mut self {
            f(entry.get_mut_with_key().1);
        }
        self
    }
}

impl<'v, T, K, V> Entry<'v, T, K, V>
where
    T: Ord + Clone,
    K: Hash + Eq,
    V: OrdBy<Target = T> + Default,
{
    /// Inserts the default value if the entry is vacant, and returns a mutable reference to the value.
    ///
    /// # Examples
    ///
    /// ```
    /// use valord_map::ValordMap;
    ///
    /// let mut map: ValordMap<usize, &str, usize> = ValordMap::new();
    /// map.entry("key").or_default();
    ///
    /// assert_eq!(map.get(&"key"), Some(&Default::default()));
    /// ```
    pub fn or_default(&mut self) -> &mut V {
        match self {
            Entry::Occupied(entry) => entry.get_mut_with_key().1,
            Entry::Vacant(entry) => {
                entry.valord.free_indexs.pop_front();
                entry.insert(V::default())
            }
        }
    }
}
