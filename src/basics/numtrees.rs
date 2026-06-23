use std::collections::btree_map::Entry;
use std::collections::BTreeMap;
use std::fmt::Write as _;

pub type NumTreeKey = i64;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NumTreeEntry<V1, V2> {
    pub val1: V1,
    pub val2: V2,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NumTree<V1, V2> {
    entries: BTreeMap<NumTreeKey, NumTreeEntry<V1, V2>>,
    root_key: Option<NumTreeKey>,
}

impl<V1, V2> Default for NumTree<V1, V2> {
    fn default() -> Self {
        Self::new()
    }
}

impl<V1, V2> NumTree<V1, V2> {
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
    pub const fn root_key(&self) -> Option<NumTreeKey> {
        self.root_key
    }

    pub fn store(&mut self, key: NumTreeKey, val1: V1, val2: V2) -> bool {
        match self.entries.entry(key) {
            Entry::Vacant(entry) => {
                entry.insert(NumTreeEntry { val1, val2 });
                self.root_key = Some(key);
                true
            }
            Entry::Occupied(_) => {
                self.root_key = Some(key);
                false
            }
        }
    }

    #[must_use]
    pub fn find(&self, key: NumTreeKey) -> Option<&NumTreeEntry<V1, V2>> {
        self.entries.get(&key)
    }

    pub fn find_mut(&mut self, key: NumTreeKey) -> Option<&mut NumTreeEntry<V1, V2>> {
        let found = self.entries.get_mut(&key);
        if found.is_some() {
            self.root_key = Some(key);
        }
        found
    }

    pub fn extract_entry(&mut self, key: NumTreeKey) -> Option<(NumTreeKey, NumTreeEntry<V1, V2>)> {
        let result = self.entries.remove_entry(&key);
        if result.is_some() {
            self.root_key = self.entries.keys().next().copied();
        }
        result
    }

    pub fn extract_root(&mut self) -> Option<(NumTreeKey, NumTreeEntry<V1, V2>)> {
        let key = match self.root_key {
            Some(key) if self.entries.contains_key(&key) => key,
            _ => *self.entries.keys().next()?,
        };
        self.extract_entry(key)
    }

    pub fn delete_entry(&mut self, key: NumTreeKey) -> bool {
        self.extract_entry(key).is_some()
    }

    #[must_use]
    pub fn max_node(&self) -> Option<(NumTreeKey, &NumTreeEntry<V1, V2>)> {
        self.entries
            .iter()
            .next_back()
            .map(|(key, entry)| (*key, entry))
    }

    #[must_use]
    pub fn max_key(&self) -> Option<NumTreeKey> {
        self.max_node().map(|(key, _entry)| key)
    }

    pub fn iter(&self) -> impl Iterator<Item = (NumTreeKey, &NumTreeEntry<V1, V2>)> {
        self.entries.iter().map(|(key, entry)| (*key, entry))
    }

    pub fn limited_iter(
        &self,
        limit: NumTreeKey,
    ) -> impl Iterator<Item = (NumTreeKey, &NumTreeEntry<V1, V2>)> {
        self.entries
            .range(limit..)
            .map(|(key, entry)| (*key, entry))
    }

    #[must_use]
    pub fn debug_print_string(&self, keys_only: bool) -> String
    where
        V1: std::fmt::Display,
        V2: std::fmt::Display,
    {
        let mut result = String::new();
        for (key, entry) in &self.entries {
            let write_result = writeln!(&mut result, "{key}");
            debug_assert!(write_result.is_ok());
            if !keys_only {
                let write_result =
                    writeln!(&mut result, " Val1: {}  Val2: {}", entry.val1, entry.val2);
                debug_assert!(write_result.is_ok());
            }
        }
        let write_result = writeln!(&mut result, "Tree size: {}", self.entries.len());
        debug_assert!(write_result.is_ok());
        result
    }
}

#[cfg(test)]
mod tests {
    use super::{NumTree, NumTreeEntry};

    #[test]
    fn store_find_and_duplicates_match_c_contract() {
        let mut tree = NumTree::new();
        assert!(tree.store(10, "ten", 100));
        assert!(tree.store(-1, "minus", -10));
        assert!(!tree.store(10, "ignored", 999));
        assert_eq!(tree.nodes(), 2);
        assert_eq!(tree.root_key(), Some(10));
        assert_eq!(
            tree.find(10),
            Some(&NumTreeEntry {
                val1: "ten",
                val2: 100
            })
        );
        assert_eq!(tree.find(99), None);
    }

    #[test]
    fn find_mut_rewrites_values_and_tracks_root_like_recent_access() {
        let mut tree = NumTree::new();
        tree.store(1, 10, 100);
        tree.store(2, 20, 200);
        tree.find_mut(1).unwrap().val2 = 101;
        assert_eq!(tree.root_key(), Some(1));
        assert_eq!(tree.find(1).unwrap().val2, 101);
    }

    #[test]
    fn traversal_and_limited_traversal_are_ascending() {
        let mut tree = NumTree::new();
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

    #[test]
    fn extract_delete_and_extract_root_remove_nodes() {
        let mut tree = NumTree::new();
        tree.store(1, "one", 1);
        tree.store(2, "two", 2);
        tree.store(3, "three", 3);
        assert_eq!(tree.root_key(), Some(3));

        assert_eq!(
            tree.extract_entry(2),
            Some((
                2,
                NumTreeEntry {
                    val1: "two",
                    val2: 2
                }
            ))
        );
        assert_eq!(tree.find(2), None);
        assert!(tree.delete_entry(1));
        assert!(!tree.delete_entry(9));

        assert_eq!(
            tree.extract_root(),
            Some((
                3,
                NumTreeEntry {
                    val1: "three",
                    val2: 3
                }
            ))
        );
        assert!(tree.is_empty());
    }

    #[test]
    fn max_node_is_non_destructive() {
        let mut tree = NumTree::new();
        tree.store(-1, "minus", 0);
        tree.store(4, "four", 0);
        tree.store(2, "two", 0);

        assert_eq!(
            tree.max_node(),
            Some((
                4,
                &NumTreeEntry {
                    val1: "four",
                    val2: 0
                }
            ))
        );
        assert_eq!(tree.max_key(), Some(4));
        assert_eq!(tree.nodes(), 3);
    }

    #[test]
    fn debug_print_reports_keys_and_tree_size() {
        let mut tree = NumTree::new();
        tree.store(2, 20, 200);
        tree.store(1, 10, 100);

        assert_eq!(tree.debug_print_string(true), "1\n2\nTree size: 2\n");
        assert_eq!(
            tree.debug_print_string(false),
            "1\n Val1: 10  Val2: 100\n2\n Val1: 20  Val2: 200\nTree size: 2\n"
        );
    }
}
