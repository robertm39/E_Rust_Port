use std::collections::btree_map::Entry;
use std::collections::BTreeMap;

pub const NUM_X_TREE_VALUES: usize = 4;
pub type NumXTreeKey = i64;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NumXTreeEntry<V> {
    vals: [V; NUM_X_TREE_VALUES],
}

impl<V> NumXTreeEntry<V> {
    #[must_use]
    pub const fn new(vals: [V; NUM_X_TREE_VALUES]) -> Self {
        Self { vals }
    }

    #[must_use]
    pub const fn values(&self) -> &[V; NUM_X_TREE_VALUES] {
        &self.vals
    }

    pub fn values_mut(&mut self) -> &mut [V; NUM_X_TREE_VALUES] {
        &mut self.vals
    }

    #[must_use]
    pub fn value(&self, index: usize) -> Option<&V> {
        self.vals.get(index)
    }

    pub fn value_mut(&mut self, index: usize) -> Option<&mut V> {
        self.vals.get_mut(index)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NumXTree<V> {
    entries: BTreeMap<NumXTreeKey, NumXTreeEntry<V>>,
    root_key: Option<NumXTreeKey>,
}

impl<V> Default for NumXTree<V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<V> NumXTree<V> {
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
    pub const fn root_key(&self) -> Option<NumXTreeKey> {
        self.root_key
    }

    pub fn insert_entry(&mut self, key: NumXTreeKey, entry: NumXTreeEntry<V>) -> bool {
        self.root_key = Some(key);
        match self.entries.entry(key) {
            Entry::Vacant(slot) => {
                slot.insert(entry);
                true
            }
            Entry::Occupied(_) => false,
        }
    }

    #[must_use]
    pub fn find(&self, key: NumXTreeKey) -> Option<&NumXTreeEntry<V>> {
        self.entries.get(&key)
    }

    pub fn find_splayed(&mut self, key: NumXTreeKey) -> Option<&NumXTreeEntry<V>> {
        if self.entries.contains_key(&key) {
            self.root_key = Some(key);
            self.entries.get(&key)
        } else {
            None
        }
    }

    pub fn find_mut(&mut self, key: NumXTreeKey) -> Option<&mut NumXTreeEntry<V>> {
        let found = self.entries.get_mut(&key);
        if found.is_some() {
            self.root_key = Some(key);
        }
        found
    }

    pub fn extract_entry(&mut self, key: NumXTreeKey) -> Option<(NumXTreeKey, NumXTreeEntry<V>)> {
        let result = self.entries.remove_entry(&key);
        if result.is_some() {
            self.root_key = self.entries.keys().next().copied();
        }
        result
    }

    pub fn extract_root(&mut self) -> Option<(NumXTreeKey, NumXTreeEntry<V>)> {
        let key = match self.root_key {
            Some(key) if self.entries.contains_key(&key) => key,
            _ => *self.entries.keys().next()?,
        };
        self.extract_entry(key)
    }

    pub fn delete_entry(&mut self, key: NumXTreeKey) -> bool {
        self.extract_entry(key).is_some()
    }

    #[must_use]
    pub fn max_node(&self) -> Option<(NumXTreeKey, &NumXTreeEntry<V>)> {
        self.entries
            .iter()
            .next_back()
            .map(|(key, entry)| (*key, entry))
    }

    #[must_use]
    pub fn max_key(&self) -> Option<NumXTreeKey> {
        self.max_node().map(|(key, _entry)| key)
    }

    pub fn iter(&self) -> impl Iterator<Item = (NumXTreeKey, &NumXTreeEntry<V>)> {
        self.entries.iter().map(|(key, entry)| (*key, entry))
    }

    pub fn limited_iter(
        &self,
        limit: NumXTreeKey,
    ) -> impl Iterator<Item = (NumXTreeKey, &NumXTreeEntry<V>)> {
        self.entries
            .range(limit..)
            .map(|(key, entry)| (*key, entry))
    }
}

impl<V> NumXTree<V>
where
    V: Default,
{
    pub fn store(&mut self, key: NumXTreeKey, val1: V, val2: V) -> bool {
        self.insert_entry(
            key,
            NumXTreeEntry::new([val1, val2, V::default(), V::default()]),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{NumXTree, NumXTreeEntry};

    #[test]
    fn store_find_and_duplicate_handling_match_c_contract() {
        let mut tree = NumXTree::new();
        assert!(tree.store(10, 100, 200));
        assert!(tree.store(-1, -10, -20));
        assert!(!tree.store(10, 999, 999));
        assert_eq!(tree.nodes(), 2);
        assert_eq!(tree.root_key(), Some(10));
        assert_eq!(tree.find(10).unwrap().values(), &[100, 200, 0, 0]);
        assert_eq!(tree.find(99), None);
    }

    #[test]
    fn insert_entry_supports_all_four_values_without_rewriting_duplicates() {
        let mut tree = NumXTree::new();
        assert!(tree.insert_entry(4, NumXTreeEntry::new([1, 2, 3, 4])));
        assert!(!tree.insert_entry(4, NumXTreeEntry::new([9, 9, 9, 9])));
        assert_eq!(tree.find(4).unwrap().values(), &[1, 2, 3, 4]);
    }

    #[test]
    fn find_mut_rewrites_value_slots_and_tracks_root_like_recent_access() {
        let mut tree = NumXTree::new();
        tree.store(1, 10, 100);
        tree.store(2, 20, 200);
        *tree.find_mut(1).unwrap().value_mut(2).unwrap() = 101;
        assert_eq!(tree.root_key(), Some(1));
        assert_eq!(tree.find(1).unwrap().value(2), Some(&101));
    }

    #[test]
    fn splayed_find_changes_root_for_later_root_extraction_like_c() {
        let mut tree = NumXTree::new();
        tree.store(1, 10, 100);
        tree.store(2, 20, 200);
        tree.store(3, 30, 300);
        assert_eq!(tree.root_key(), Some(3));

        assert_eq!(tree.find_splayed(1).unwrap().values(), &[10, 100, 0, 0]);
        assert_eq!(tree.root_key(), Some(1));
        assert_eq!(
            tree.extract_root(),
            Some((1, NumXTreeEntry::new([10, 100, 0, 0])))
        );

        assert_eq!(tree.find_splayed(99), None);
        assert_ne!(tree.root_key(), Some(99));
    }

    #[test]
    fn extract_delete_and_extract_root_remove_nodes() {
        let mut tree = NumXTree::new();
        tree.store(1, 10, 100);
        tree.store(2, 20, 200);
        tree.store(3, 30, 300);
        assert_eq!(tree.root_key(), Some(3));

        assert_eq!(
            tree.extract_entry(2),
            Some((2, NumXTreeEntry::new([20, 200, 0, 0])))
        );
        assert_eq!(tree.find(2), None);
        assert!(tree.delete_entry(1));
        assert!(!tree.delete_entry(9));

        assert_eq!(
            tree.extract_root(),
            Some((3, NumXTreeEntry::new([30, 300, 0, 0])))
        );
        assert!(tree.is_empty());
    }

    #[test]
    fn max_node_is_non_destructive() {
        let mut tree = NumXTree::new();
        tree.store(-1, -10, 0);
        tree.store(4, 40, 0);
        tree.store(2, 20, 0);

        assert_eq!(
            tree.max_node(),
            Some((4, &NumXTreeEntry::new([40, 0, 0, 0])))
        );
        assert_eq!(tree.max_key(), Some(4));
        assert_eq!(tree.nodes(), 3);
    }

    #[test]
    fn traversal_and_limited_traversal_are_ascending() {
        let mut tree = NumXTree::new();
        for key in [5, 1, 3, 7] {
            tree.store(key, key * 10, 0);
        }

        let all_keys = tree.iter().map(|(key, _entry)| key).collect::<Vec<_>>();
        assert_eq!(all_keys, vec![1, 3, 5, 7]);

        let limited_keys = tree
            .limited_iter(4)
            .map(|(key, _entry)| key)
            .collect::<Vec<_>>();
        assert_eq!(limited_keys, vec![5, 7]);

        let exact_limit = tree
            .limited_iter(5)
            .map(|(key, _entry)| key)
            .collect::<Vec<_>>();
        assert_eq!(exact_limit, vec![5, 7]);
    }
}
