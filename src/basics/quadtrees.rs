use crate::basics::defines::{IntOrP, IntOrPInt};
use std::cmp::Ordering;
use std::collections::btree_map::Entry;
use std::collections::BTreeMap;

pub type QuadInt = i32;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct QuadKey<P> {
    p1: P,
    i1: QuadInt,
    p2: P,
    i2: QuadInt,
}

impl<P> QuadKey<P> {
    #[must_use]
    pub const fn new(p1: P, i1: QuadInt, p2: P, i2: QuadInt) -> Self {
        Self { p1, i1, p2, i2 }
    }

    #[must_use]
    pub const fn p1(&self) -> &P {
        &self.p1
    }

    #[must_use]
    pub const fn i1(&self) -> QuadInt {
        self.i1
    }

    #[must_use]
    pub const fn p2(&self) -> &P {
        &self.p2
    }

    #[must_use]
    pub const fn i2(&self) -> QuadInt {
        self.i2
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QuadTreeEntry<P, V> {
    key: QuadKey<P>,
    value: V,
}

impl<P, V> QuadTreeEntry<P, V> {
    #[must_use]
    pub const fn new(key: QuadKey<P>, value: V) -> Self {
        Self { key, value }
    }

    #[must_use]
    pub const fn key(&self) -> &QuadKey<P> {
        &self.key
    }

    #[must_use]
    pub const fn value(&self) -> &V {
        &self.value
    }

    pub fn value_mut(&mut self) -> &mut V {
        &mut self.value
    }

    #[must_use]
    pub fn into_parts(self) -> (QuadKey<P>, V) {
        (self.key, self.value)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QuadTree<P, V> {
    entries: BTreeMap<QuadKey<P>, V>,
    root_key: Option<QuadKey<P>>,
}

impl<P, V> Default for QuadTree<P, V>
where
    P: Ord + Clone,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<P, V> QuadTree<P, V>
where
    P: Ord + Clone,
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
    pub const fn root_key(&self) -> Option<&QuadKey<P>> {
        self.root_key.as_ref()
    }

    pub fn insert_entry(&mut self, entry: QuadTreeEntry<P, V>) -> Option<&V> {
        let (key, value) = entry.into_parts();
        self.root_key = Some(key.clone());
        match self.entries.entry(key) {
            Entry::Vacant(slot) => {
                slot.insert(value);
                None
            }
            Entry::Occupied(slot) => Some(slot.into_mut()),
        }
    }

    pub fn store(&mut self, key: QuadKey<P>, value: V) -> bool {
        self.root_key = Some(key.clone());
        match self.entries.entry(key) {
            Entry::Vacant(slot) => {
                slot.insert(value);
                true
            }
            Entry::Occupied(_) => false,
        }
    }

    pub fn find(&mut self, key: &QuadKey<P>) -> Option<&V> {
        if self.entries.contains_key(key) {
            self.root_key = Some(key.clone());
            self.entries.get(key)
        } else {
            None
        }
    }

    pub fn find_mut(&mut self, key: &QuadKey<P>) -> Option<&mut V> {
        if self.entries.contains_key(key) {
            self.root_key = Some(key.clone());
            self.entries.get_mut(key)
        } else {
            None
        }
    }

    pub fn extract_entry(&mut self, key: &QuadKey<P>) -> Option<QuadTreeEntry<P, V>> {
        let (key, value) = self.entries.remove_entry(key)?;
        self.root_key = self.entries.keys().next().cloned();
        Some(QuadTreeEntry::new(key, value))
    }

    pub fn delete_entry(&mut self, key: &QuadKey<P>) -> bool {
        self.extract_entry(key).is_some()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&QuadKey<P>, &V)> {
        self.entries.iter()
    }

    #[must_use]
    pub fn entries_vec(&self) -> Vec<(&QuadKey<P>, &V)> {
        self.iter().collect()
    }
}

impl<P> QuadTree<P, IntOrP<P>>
where
    P: Ord + Clone,
{
    pub fn store_int(&mut self, key: QuadKey<P>, value: IntOrPInt) -> bool {
        self.store(key, IntOrP::Int(value))
    }

    pub fn store_pointer(&mut self, key: QuadKey<P>, value: P) -> bool {
        self.store(key, IntOrP::Pointer(value))
    }
}

pub fn double_key_cmp<P>(p1: &P, i1: QuadInt, p2: &P, i2: QuadInt) -> Ordering
where
    P: Ord,
{
    p1.cmp(p2).then_with(|| i1.cmp(&i2))
}

pub fn quad_key_cmp<P>(key1: &QuadKey<P>, key2: &QuadKey<P>) -> Ordering
where
    P: Ord,
{
    double_key_cmp(&key1.p1, key1.i1, &key2.p1, key2.i1)
        .then_with(|| double_key_cmp(&key1.p2, key1.i2, &key2.p2, key2.i2))
}

#[cfg(test)]
mod tests {
    use super::{double_key_cmp, quad_key_cmp, QuadKey, QuadTree, QuadTreeEntry};
    use crate::basics::defines::IntOrP;
    use std::cmp::Ordering;

    fn key(p1: usize, i1: i32, p2: usize, i2: i32) -> QuadKey<usize> {
        QuadKey::new(p1, i1, p2, i2)
    }

    #[test]
    fn double_and_quad_key_comparisons_match_c_field_order() {
        assert_eq!(double_key_cmp(&1, 99, &2, -99), Ordering::Less);
        assert_eq!(double_key_cmp(&2, -1, &2, 1), Ordering::Less);
        assert_eq!(double_key_cmp(&2, 7, &2, 7), Ordering::Equal);

        assert_eq!(
            quad_key_cmp(&key(1, 0, 9, 0), &key(1, 0, 10, -9)),
            Ordering::Less
        );
        assert_eq!(
            quad_key_cmp(&key(1, 0, 9, 4), &key(1, 0, 9, 3)),
            Ordering::Greater
        );
    }

    #[test]
    fn store_find_and_duplicates_preserve_existing_value() {
        let mut tree = QuadTree::new();
        let first_key = key(10, 1, 20, 2);

        assert!(tree.store(first_key.clone(), "old"));
        assert!(!tree.store(first_key.clone(), "new"));
        assert_eq!(tree.nodes(), 1);
        assert_eq!(tree.root_key(), Some(&first_key));
        assert_eq!(tree.find(&first_key), Some(&"old"));
    }

    #[test]
    fn insert_entry_returns_existing_value_for_duplicate_key() {
        let mut tree = QuadTree::new();
        let first_key = key(3, 1, 4, 1);

        assert_eq!(
            tree.insert_entry(QuadTreeEntry::new(first_key.clone(), 100)),
            None
        );
        assert_eq!(
            tree.insert_entry(QuadTreeEntry::new(first_key.clone(), 200)),
            Some(&100)
        );
        assert_eq!(tree.find(&first_key), Some(&100));
    }

    #[test]
    fn find_mut_rewrites_value_and_tracks_root_like_recent_access() {
        let mut tree = QuadTree::new();
        let first_key = key(1, 0, 2, 0);
        let second_key = key(2, 0, 3, 0);
        tree.store(first_key.clone(), 10);
        tree.store(second_key.clone(), 20);

        *tree.find_mut(&first_key).unwrap() = 11;
        assert_eq!(tree.root_key(), Some(&first_key));
        assert_eq!(tree.find(&first_key), Some(&11));
    }

    #[test]
    fn extract_and_delete_remove_entries_without_touching_others() {
        let mut tree = QuadTree::new();
        let first_key = key(1, 1, 1, 1);
        let second_key = key(2, 2, 2, 2);
        tree.store(first_key.clone(), "first");
        tree.store(second_key.clone(), "second");

        let extracted = tree
            .extract_entry(&first_key)
            .map(QuadTreeEntry::into_parts);
        assert_eq!(extracted, Some((first_key.clone(), "first")));
        assert_eq!(tree.find(&first_key), None);
        assert_eq!(tree.find(&second_key), Some(&"second"));
        assert!(!tree.delete_entry(&first_key));
        assert!(tree.delete_entry(&second_key));
        assert!(tree.is_empty());
    }

    #[test]
    fn traversal_is_sorted_by_full_quad_key() {
        let mut tree = QuadTree::new();
        for key in [key(2, 0, 0, 0), key(1, 2, 0, 0), key(1, 1, 9, 9)] {
            tree.store(key.clone(), key.i1());
        }

        let keys = tree
            .iter()
            .map(|(key, _value)| key.clone())
            .collect::<Vec<_>>();
        assert_eq!(
            keys,
            vec![key(1, 1, 9, 9), key(1, 2, 0, 0), key(2, 0, 0, 0)]
        );
    }

    #[test]
    fn int_or_pointer_helpers_match_c_union_payload_shape() {
        let mut tree = QuadTree::<usize, IntOrP<usize>>::new();
        let int_key = key(1, 0, 2, 0);
        let ptr_key = key(1, 0, 3, 0);

        assert!(tree.store_int(int_key.clone(), 5));
        assert!(tree.store_pointer(ptr_key.clone(), 99));
        assert_eq!(tree.find(&int_key).and_then(IntOrP::as_int), Some(5));
        assert_eq!(tree.find(&ptr_key).and_then(IntOrP::as_pointer), Some(&99));
    }
}
