use std::collections::btree_map::Entry;
use std::collections::BTreeMap;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObjMap<K, V> {
    entries: BTreeMap<K, Option<V>>,
    root_key: Option<K>,
}

impl<K, V> Default for ObjMap<K, V>
where
    K: Ord + Clone,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<K, V> ObjMap<K, V>
where
    K: Ord + Clone,
{
    #[must_use]
    pub const fn new() -> Self {
        Self {
            entries: BTreeMap::new(),
            root_key: None,
        }
    }

    #[must_use]
    pub fn nodes(&self) -> usize {
        self.entries.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    #[must_use]
    pub const fn root_key(&self) -> Option<&K> {
        self.root_key.as_ref()
    }

    pub fn store(&mut self, key: K, value: V) -> Option<V> {
        let (slot, created) = self.get_ref(key);
        let old_value = slot.replace(value);
        if created {
            None
        } else {
            old_value
        }
    }

    pub fn get_ref(&mut self, key: K) -> (&mut Option<V>, bool) {
        self.root_key = Some(key.clone());
        match self.entries.entry(key) {
            Entry::Vacant(slot) => (slot.insert(None), true),
            Entry::Occupied(slot) => (slot.into_mut(), false),
        }
    }

    pub fn find(&mut self, key: &K) -> Option<&V> {
        if self.entries.contains_key(key) {
            self.root_key = Some(key.clone());
        }
        self.entries.get(key).and_then(Option::as_ref)
    }

    pub fn extract(&mut self, key: &K) -> Option<V> {
        let value = self.entries.remove(key)?;
        self.root_key = self.entries.keys().next().cloned();
        value
    }

    pub fn extract_slot(&mut self, key: &K) -> Option<Option<V>> {
        let value = self.entries.remove(key)?;
        self.root_key = self.entries.keys().next().cloned();
        Some(value)
    }

    pub fn iter_entries(&self) -> impl Iterator<Item = (&K, &Option<V>)> {
        self.entries.iter()
    }

    #[must_use]
    pub fn traverse_values(&self) -> Vec<(&K, Option<&V>)> {
        self.entries
            .iter()
            .map(|(key, value)| (key, value.as_ref()))
            .collect()
    }

    pub fn free_with<F>(self, mut del_fun: F)
    where
        F: FnMut(K, Option<V>),
    {
        for (key, value) in self.entries {
            del_fun(key, value);
        }
    }
}

#[must_use]
pub const fn size_of_obj_map_node_estimate() -> usize {
    std::mem::size_of::<usize>() * 4
}

#[cfg(test)]
mod tests {
    use super::{size_of_obj_map_node_estimate, ObjMap};

    #[test]
    fn store_returns_old_value_and_updates_mapping() {
        let mut map = ObjMap::new();

        assert_eq!(map.store(2, "two"), None);
        assert_eq!(map.store(1, "one"), None);
        assert_eq!(map.store(2, "TWO"), Some("two"));
        assert_eq!(map.nodes(), 2);
        assert_eq!(map.find(&2), Some(&"TWO"));
        assert_eq!(map.root_key(), Some(&2));
    }

    #[test]
    fn get_ref_creates_null_slot_and_reports_creation() {
        let mut map: ObjMap<&str, i32> = ObjMap::new();

        let (slot, created) = map.get_ref("x");
        assert!(created);
        assert_eq!(slot, &None);
        assert_eq!(map.find(&"x"), None);
        assert_eq!(map.nodes(), 1);

        let (slot, created) = map.get_ref("x");
        assert!(!created);
        *slot = Some(7);
        assert_eq!(map.find(&"x"), Some(&7));
    }

    #[test]
    fn store_over_null_slot_returns_none_like_c() {
        let mut map = ObjMap::new();
        map.get_ref("x");

        assert_eq!(map.store("x", 10), None);
        assert_eq!(map.find(&"x"), Some(&10));
    }

    #[test]
    fn extract_removes_node_and_returns_value_shape() {
        let mut map = ObjMap::new();
        map.store(1, "one");
        map.get_ref(2);

        assert_eq!(map.extract(&1), Some("one"));
        assert_eq!(map.find(&1), None);
        assert_eq!(map.extract(&2), None);
        assert_eq!(map.nodes(), 0);
        assert_eq!(map.extract_slot(&99), None);
    }

    #[test]
    fn traversal_is_sorted_and_preserves_null_slots() {
        let mut map = ObjMap::new();
        map.store(3, "three");
        map.get_ref(1);
        map.store(2, "two");

        let traversed = map
            .traverse_values()
            .into_iter()
            .map(|(key, value)| (*key, value.copied()))
            .collect::<Vec<_>>();
        assert_eq!(
            traversed,
            vec![(1, None), (2, Some("two")), (3, Some("three"))]
        );
    }

    #[test]
    fn free_with_visits_key_value_pairs_in_order() {
        let mut map = ObjMap::new();
        map.store(2, "two");
        map.store(1, "one");
        let mut freed = Vec::new();

        map.free_with(|key, value| freed.push((key, value)));
        assert_eq!(freed, vec![(1, Some("one")), (2, Some("two"))]);
    }

    #[test]
    fn node_size_estimate_is_nonzero() {
        assert!(size_of_obj_map_node_estimate() > 0);
    }
}
