use std::cmp::Ordering;
use std::collections::btree_map::Entry;
use std::collections::BTreeMap;

#[derive(Copy, Clone, Debug)]
struct FloatTreeKey(f64);

impl FloatTreeKey {
    const fn new(key: f64) -> Self {
        Self(key)
    }

    const fn get(self) -> f64 {
        self.0
    }
}

impl PartialEq for FloatTreeKey {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other).is_eq()
    }
}

impl Eq for FloatTreeKey {}

impl PartialOrd for FloatTreeKey {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for FloatTreeKey {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.0.partial_cmp(&other.0) {
            Some(ordering) => ordering,
            None => match (self.0.is_nan(), other.0.is_nan()) {
                (true, false) => Ordering::Greater,
                (false, true) => Ordering::Less,
                (true, true) | (false, false) => Ordering::Equal,
            },
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct FloatTreeEntry<V1, V2> {
    pub val1: V1,
    pub val2: V2,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FloatTree<V1, V2> {
    entries: BTreeMap<FloatTreeKey, FloatTreeEntry<V1, V2>>,
    root_key: Option<FloatTreeKey>,
}

impl<V1, V2> Default for FloatTree<V1, V2> {
    fn default() -> Self {
        Self::new()
    }
}

impl<V1, V2> FloatTree<V1, V2> {
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
    pub fn root_key(&self) -> Option<f64> {
        self.root_key.map(FloatTreeKey::get)
    }

    pub fn store(&mut self, key: f64, val1: V1, val2: V2) -> bool {
        let key = FloatTreeKey::new(key);
        match self.entries.entry(key) {
            Entry::Vacant(entry) => {
                self.root_key = Some(key);
                entry.insert(FloatTreeEntry { val1, val2 });
                true
            }
            Entry::Occupied(entry) => {
                self.root_key = Some(*entry.key());
                false
            }
        }
    }

    #[must_use]
    pub fn find(&self, key: f64) -> Option<&FloatTreeEntry<V1, V2>> {
        self.entries.get(&FloatTreeKey::new(key))
    }

    pub fn find_splayed(&mut self, key: f64) -> Option<&FloatTreeEntry<V1, V2>> {
        let key = FloatTreeKey::new(key);
        let found = self.entries.get_key_value(&key);
        if let Some((stored_key, _entry)) = found {
            self.root_key = Some(*stored_key);
        }
        found.map(|(_stored_key, entry)| entry)
    }

    pub fn find_mut(&mut self, key: f64) -> Option<&mut FloatTreeEntry<V1, V2>> {
        let key = FloatTreeKey::new(key);
        if let Some((stored_key, _entry)) = self.entries.get_key_value(&key) {
            self.root_key = Some(*stored_key);
            self.entries.get_mut(&key)
        } else {
            None
        }
    }

    pub fn extract_entry(&mut self, key: f64) -> Option<(f64, FloatTreeEntry<V1, V2>)> {
        let result = self
            .entries
            .remove_entry(&FloatTreeKey::new(key))
            .map(|(key, entry)| (key.get(), entry));
        if result.is_some() {
            self.root_key = self.entries.keys().next().copied();
        }
        result
    }

    pub fn delete_entry(&mut self, key: f64) -> bool {
        self.extract_entry(key).is_some()
    }

    pub fn iter(&self) -> impl Iterator<Item = (f64, &FloatTreeEntry<V1, V2>)> {
        self.entries.iter().map(|(key, entry)| (key.get(), entry))
    }
}

#[cfg(test)]
mod tests {
    use super::{FloatTree, FloatTreeEntry};

    #[test]
    fn store_find_and_duplicates_match_c_contract_for_ordered_keys() {
        let mut tree = FloatTree::new();
        assert!(tree.store(10.5, "ten", 100));
        assert!(tree.store(-1.25, "minus", -10));
        assert!(!tree.store(10.5, "ignored", 999));
        assert_eq!(tree.nodes(), 2);
        assert_eq!(tree.root_key(), Some(10.5));
        assert_eq!(
            tree.find(10.5),
            Some(&FloatTreeEntry {
                val1: "ten",
                val2: 100
            })
        );
        assert_eq!(tree.find(99.0), None);
    }

    #[test]
    fn find_mut_rewrites_values_and_tracks_root_like_recent_access() {
        let mut tree = FloatTree::new();
        tree.store(1.0, 10, 100);
        tree.store(2.0, 20, 200);
        tree.find_mut(1.0).unwrap().val2 = 101;
        assert_eq!(tree.root_key(), Some(1.0));
        assert_eq!(tree.find(1.0).unwrap().val2, 101);
    }

    #[test]
    fn splayed_find_tracks_recent_root_like_c() {
        let mut tree = FloatTree::new();
        tree.store(1.0, "one", 1);
        tree.store(2.0, "two", 2);
        tree.store(3.0, "three", 3);
        assert_eq!(tree.root_key(), Some(3.0));

        assert_eq!(tree.find_splayed(1.0).unwrap().val1, "one");
        assert_eq!(tree.root_key(), Some(1.0));
        assert_eq!(tree.find_splayed(99.0), None);
        assert_eq!(tree.root_key(), Some(1.0));
    }

    #[test]
    fn extract_and_delete_remove_entries_without_touching_others() {
        let mut tree = FloatTree::new();
        tree.store(1.0, "one", 1);
        tree.store(2.5, "two", 2);
        tree.store(3.0, "three", 3);

        assert_eq!(
            tree.extract_entry(2.5),
            Some((
                2.5,
                FloatTreeEntry {
                    val1: "two",
                    val2: 2
                }
            ))
        );
        assert_eq!(tree.find(2.5), None);
        assert!(tree.delete_entry(1.0));
        assert!(!tree.delete_entry(9.0));
        assert_eq!(tree.nodes(), 1);
        assert_eq!(
            tree.find(3.0),
            Some(&FloatTreeEntry {
                val1: "three",
                val2: 3
            })
        );
    }

    #[test]
    fn traversal_is_sorted_and_treats_signed_zero_as_duplicate() {
        let mut tree = FloatTree::new();
        tree.store(3.0, 30, 0);
        tree.store(-0.0, 0, 0);
        assert!(!tree.store(0.0, 999, 999));
        assert_eq!(tree.root_key().unwrap().to_bits(), (-0.0_f64).to_bits());
        tree.store(f64::INFINITY, 100, 0);
        tree.store(-2.0, -20, 0);

        let visited = tree
            .iter()
            .map(|(key, entry)| (key, entry.val1))
            .collect::<Vec<_>>();
        assert_eq!(
            visited,
            vec![(-2.0, -20), (-0.0, 0), (3.0, 30), (f64::INFINITY, 100)]
        );
    }

    #[test]
    fn signed_zero_accesses_track_the_stored_key_representation() {
        let mut tree = FloatTree::new();
        assert!(tree.store(-0.0, 10, 0));
        assert!(!tree.store(0.0, 99, 99));
        assert_eq!(tree.root_key().unwrap().to_bits(), (-0.0_f64).to_bits());

        assert_eq!(tree.find_splayed(0.0).unwrap().val1, 10);
        assert_eq!(tree.root_key().unwrap().to_bits(), (-0.0_f64).to_bits());

        tree.find_mut(0.0).unwrap().val1 = 20;
        assert_eq!(tree.root_key().unwrap().to_bits(), (-0.0_f64).to_bits());
        assert_eq!(tree.find(-0.0).unwrap().val1, 20);

        let (key, entry) = tree.extract_entry(0.0).unwrap();
        assert_eq!(key.to_bits(), (-0.0_f64).to_bits());
        assert_eq!(entry.val1, 20);
    }

    #[test]
    fn nan_keys_have_a_deterministic_single_bucket() {
        let mut tree = FloatTree::new();
        assert!(tree.store(1.0, 10, 0));
        assert!(tree.store(f64::NAN, 20, 0));
        assert!(!tree.store(f64::from_bits(0x7ff8_0000_0000_0001), 99, 0));
        assert_eq!(tree.nodes(), 2);
        assert_eq!(tree.find(f64::NAN).unwrap().val1, 20);

        let keys = tree.iter().map(|(key, _entry)| key).collect::<Vec<_>>();
        assert_eq!(keys.len(), 2);
        assert_eq!(keys[0].to_bits(), 1.0_f64.to_bits());
        assert!(keys[1].is_nan());
    }
}
