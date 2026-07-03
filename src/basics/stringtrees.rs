use std::collections::btree_map::Entry;
use std::collections::BTreeMap;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StrTreeEntry<V1, V2> {
    pub val1: V1,
    pub val2: V2,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StrTree<V1, V2> {
    entries: BTreeMap<String, StrTreeEntry<V1, V2>>,
    root_key: Option<String>,
}

impl<V1, V2> Default for StrTree<V1, V2> {
    fn default() -> Self {
        Self::new()
    }
}

impl<V1, V2> StrTree<V1, V2> {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            entries: BTreeMap::new(),
            root_key: None,
        }
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    #[must_use]
    pub fn root_key(&self) -> Option<&str> {
        self.root_key.as_deref()
    }

    pub fn store(&mut self, key: &str, val1: V1, val2: V2) -> bool {
        let owned_key = key.to_owned();
        match self.entries.entry(owned_key.clone()) {
            Entry::Vacant(entry) => {
                entry.insert(StrTreeEntry { val1, val2 });
                self.root_key = Some(owned_key);
                true
            }
            Entry::Occupied(_) => {
                self.root_key = Some(owned_key);
                false
            }
        }
    }

    #[must_use]
    pub fn find(&self, key: &str) -> Option<&StrTreeEntry<V1, V2>> {
        self.entries.get(key)
    }

    pub fn find_splayed(&mut self, key: &str) -> Option<&StrTreeEntry<V1, V2>> {
        if self.entries.contains_key(key) {
            self.root_key = Some(key.to_owned());
            self.entries.get(key)
        } else {
            None
        }
    }

    pub fn find_mut(&mut self, key: &str) -> Option<&mut StrTreeEntry<V1, V2>> {
        if self.entries.contains_key(key) {
            self.root_key = Some(key.to_owned());
            self.entries.get_mut(key)
        } else {
            None
        }
    }

    pub fn extract_entry(&mut self, key: &str) -> Option<(String, StrTreeEntry<V1, V2>)> {
        let result = self.entries.remove_entry(key);
        if result.is_some() {
            self.root_key = self.entries.keys().next().cloned();
        }
        result
    }

    pub fn delete_entry(&mut self, key: &str) -> bool {
        self.entries.remove(key).is_some()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &StrTreeEntry<V1, V2>)> {
        self.entries
            .iter()
            .map(|(key, entry)| (key.as_str(), entry))
    }
}

#[cfg(test)]
mod tests {
    use super::{StrTree, StrTreeEntry};

    #[test]
    fn store_find_and_duplicate_handling_match_c_contract() {
        let mut tree = StrTree::new();
        assert!(tree.store("gamma", 3, 30));
        assert!(tree.store("alpha", 1, 10));
        assert!(!tree.store("gamma", 99, 99));
        assert_eq!(tree.len(), 2);
        assert_eq!(tree.root_key(), Some("gamma"));
        assert_eq!(
            tree.find("gamma"),
            Some(&StrTreeEntry { val1: 3, val2: 30 })
        );
        assert_eq!(tree.find("missing"), None);
    }

    #[test]
    fn find_mut_allows_signature_style_value_rewrite() {
        let mut tree = StrTree::new();
        tree.store("name", 1, 0);
        tree.find_mut("name").unwrap().val1 = 7;
        assert_eq!(tree.root_key(), Some("name"));
        assert_eq!(tree.find("name").unwrap().val1, 7);
    }

    #[test]
    fn splayed_find_tracks_recent_root_like_c() {
        let mut tree = StrTree::new();
        tree.store("alpha", 1, 0);
        tree.store("beta", 2, 0);
        tree.store("gamma", 3, 0);

        assert_eq!(tree.find_splayed("alpha").map(|entry| entry.val1), Some(1));
        assert_eq!(tree.root_key(), Some("alpha"));
        assert_eq!(tree.find_splayed("missing"), None);
        assert_eq!(tree.root_key(), Some("alpha"));
    }

    #[test]
    fn traversal_is_sorted_by_key_like_tree_traversal() {
        let mut tree = StrTree::new();
        tree.store("gamma", 3, 0);
        tree.store("alpha", 1, 0);
        tree.store("beta", 2, 0);

        let visited = tree
            .iter()
            .map(|(key, entry)| (key.to_owned(), entry.val1))
            .collect::<Vec<_>>();
        assert_eq!(
            visited,
            vec![
                ("alpha".to_owned(), 1),
                ("beta".to_owned(), 2),
                ("gamma".to_owned(), 3)
            ]
        );
    }

    #[test]
    fn extract_and_delete_remove_entries_without_touching_others() {
        let mut tree = StrTree::new();
        tree.store("alpha", 1, 10);
        tree.store("beta", 2, 20);
        tree.store("gamma", 3, 30);

        assert_eq!(
            tree.extract_entry("beta"),
            Some(("beta".to_owned(), StrTreeEntry { val1: 2, val2: 20 }))
        );
        assert_eq!(tree.find("beta"), None);
        assert!(tree.delete_entry("alpha"));
        assert!(!tree.delete_entry("missing"));
        assert_eq!(
            tree.find("gamma"),
            Some(&StrTreeEntry { val1: 3, val2: 30 })
        );
    }
}
