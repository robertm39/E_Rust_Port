use crate::basics::pdrangearrays::{PDPointerRangeArr, PDRangeArrIndex};
use std::collections::BTreeMap;
use std::fmt::Write as _;

pub type IntMapKey = PDRangeArrIndex;

pub const MAX_TREE_DENSITY: usize = 8;
pub const MIN_TREE_DENSITY: usize = 4;
pub const IM_ARRAY_SIZE: usize = MAX_TREE_DENSITY;

pub const INTMAPCELL_MEM: usize = 20;
pub const NUMTREECELL_MEM: usize = 24;
pub const PDARRAYCELL_MEM: usize = 20;
pub const INTORP_MEM: usize = 4;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum IntMapType {
    Empty = 0,
    Single = 1,
    Array = 2,
    Tree = 3,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum IntMapRepr<V: Clone> {
    Empty,
    Single { key: IntMapKey, value: Option<V> },
    Array(PDPointerRangeArr<V>),
    Tree(BTreeMap<IntMapKey, Option<V>>),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IntMap<V: Clone> {
    repr: IntMapRepr<V>,
    entry_no: usize,
    min_key: IntMapKey,
    max_key: IntMapKey,
}

impl<V: Clone> Default for IntMap<V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<V: Clone> IntMap<V> {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            repr: IntMapRepr::Empty,
            entry_no: 0,
            min_key: 0,
            max_key: 0,
        }
    }

    #[must_use]
    pub const fn map_type(&self) -> IntMapType {
        match self.repr {
            IntMapRepr::Empty => IntMapType::Empty,
            IntMapRepr::Single { .. } => IntMapType::Single,
            IntMapRepr::Array(_) => IntMapType::Array,
            IntMapRepr::Tree(_) => IntMapType::Tree,
        }
    }

    #[must_use]
    pub const fn entry_count_estimate(&self) -> usize {
        self.entry_no
    }

    #[must_use]
    pub const fn min_key(&self) -> Option<IntMapKey> {
        match self.repr {
            IntMapRepr::Empty => None,
            _ => Some(self.min_key),
        }
    }

    #[must_use]
    pub const fn max_key(&self) -> Option<IntMapKey> {
        match self.repr {
            IntMapRepr::Empty => None,
            _ => Some(self.max_key),
        }
    }

    #[must_use]
    pub fn get_val(&mut self, key: IntMapKey) -> Option<&V> {
        match &mut self.repr {
            IntMapRepr::Empty => None,
            IntMapRepr::Single {
                key: single_key,
                value,
            } => {
                if *single_key == key {
                    value.as_ref()
                } else {
                    None
                }
            }
            IntMapRepr::Array(array) => {
                if key <= self.max_key {
                    array.element(key).as_ref()
                } else {
                    None
                }
            }
            IntMapRepr::Tree(tree) => {
                if key <= self.max_key {
                    tree.get(&key).and_then(Option::as_ref)
                } else {
                    None
                }
            }
        }
    }

    /// Return a mutable slot for `key`, creating an empty slot if needed.
    ///
    /// # Panics
    ///
    /// Panics only if an internal representation invariant is broken after
    /// slot creation.
    pub fn get_ref(&mut self, key: IntMapKey) -> &mut Option<V> {
        self.ensure_ref_slot(key);
        match &mut self.repr {
            IntMapRepr::Single {
                key: single_key,
                value,
            } => {
                debug_assert_eq!(*single_key, key);
                value
            }
            IntMapRepr::Array(array) => array.element_ref(key),
            IntMapRepr::Tree(tree) => match tree.get_mut(&key) {
                Some(value) => value,
                None => panic!("IntMap tree slot missing after ensure_ref_slot"),
            },
            IntMapRepr::Empty => panic!("IntMap remained empty after ensure_ref_slot"),
        }
    }

    pub fn assign(&mut self, key: IntMapKey, value: V) {
        *self.get_ref(key) = Some(value);
    }

    pub fn del_key(&mut self, key: IntMapKey) -> Option<V> {
        match &mut self.repr {
            IntMapRepr::Empty => None,
            IntMapRepr::Single {
                key: single_key,
                value,
            } => {
                if *single_key == key {
                    let result = value.take();
                    self.repr = IntMapRepr::Empty;
                    self.entry_no = 0;
                    result
                } else {
                    None
                }
            }
            IntMapRepr::Array(array) => {
                if key > self.max_key {
                    return None;
                }
                let result = array.element(key).clone();
                if result.is_some() {
                    array.assign(key, None);
                    self.entry_no = self.entry_no.saturating_sub(1);
                    if switch_to_tree(self.min_key, self.max_key, self.max_key, self.entry_no) {
                        self.array_to_tree();
                    }
                }
                result
            }
            IntMapRepr::Tree(tree) => {
                let removed = tree.remove(&key)?;
                self.entry_no = self.entry_no.saturating_sub(1);
                if key == self.max_key {
                    self.max_key = tree.keys().next_back().copied().unwrap_or(self.min_key);
                    if switch_to_array(self.min_key, self.max_key, self.max_key, self.entry_no) {
                        self.tree_to_array();
                    }
                }
                removed
            }
        }
    }

    #[must_use]
    pub fn iter_range(&self, lower_key: IntMapKey, upper_key: IntMapKey) -> Vec<(IntMapKey, &V)> {
        if lower_key > upper_key {
            return Vec::new();
        }
        match &self.repr {
            IntMapRepr::Empty => Vec::new(),
            IntMapRepr::Single { key, value } => {
                if *key >= lower_key && *key <= upper_key {
                    value
                        .as_ref()
                        .map_or_else(Vec::new, |value| vec![(*key, value)])
                } else {
                    Vec::new()
                }
            }
            IntMapRepr::Array(array) => {
                let lower = lower_key.max(self.min_key);
                let upper = upper_key.min(self.max_key);
                if lower > upper {
                    return Vec::new();
                }

                (lower..=upper)
                    .filter_map(|key| {
                        array
                            .existing_element(key)
                            .and_then(Option::as_ref)
                            .map(|value| (key, value))
                    })
                    .collect()
            }
            IntMapRepr::Tree(tree) => tree
                .range(lower_key..=upper_key)
                .filter_map(|(key, value)| value.as_ref().map(|value| (*key, value)))
                .collect(),
        }
    }

    #[must_use]
    pub fn iter_range_c_mut(
        &mut self,
        lower_key: IntMapKey,
        upper_key: IntMapKey,
    ) -> Vec<(IntMapKey, V)> {
        if lower_key > upper_key {
            return Vec::new();
        }

        match &mut self.repr {
            IntMapRepr::Empty => Vec::new(),
            IntMapRepr::Single { key, value } => {
                if *key >= lower_key && *key <= upper_key {
                    value
                        .as_ref()
                        .map_or_else(Vec::new, |value| vec![(*key, value.clone())])
                } else {
                    Vec::new()
                }
            }
            IntMapRepr::Array(array) => {
                let upper = upper_key.min(self.max_key);
                if lower_key > upper {
                    return Vec::new();
                }

                let mut entries = Vec::new();
                let mut key = lower_key;
                loop {
                    if let Some(value) = array.element(key).as_ref() {
                        entries.push((key, value.clone()));
                    }
                    if key == upper || key == IntMapKey::MAX {
                        break;
                    }
                    key += 1;
                }
                entries
            }
            IntMapRepr::Tree(tree) => {
                let upper = upper_key.min(self.max_key);
                if lower_key > upper {
                    return Vec::new();
                }
                tree.range(lower_key..=upper)
                    .filter_map(|(key, value)| value.as_ref().map(|value| (*key, value.clone())))
                    .collect()
            }
        }
    }

    #[must_use]
    pub fn entries(&self) -> Vec<(IntMapKey, &V)> {
        match self.map_type() {
            IntMapType::Empty => Vec::new(),
            _ => self.iter_range(self.min_key, self.max_key),
        }
    }

    #[must_use]
    pub fn debug_print_string(&self) -> String
    where
        V: std::fmt::Display,
    {
        let mut result = String::new();
        let write_result = writeln!(
            &mut result,
            "% ==== IntMapType {} Size = {}",
            self.map_type() as u8,
            self.storage_estimate()
        );
        debug_assert!(write_result.is_ok());
        for (key, value) in self.entries() {
            let write_result = writeln!(&mut result, "% {key:5} : {value}");
            debug_assert!(write_result.is_ok());
        }
        result.push_str("% ==== IntMap End\n");
        result
    }

    #[must_use]
    pub fn storage_estimate(&self) -> usize {
        match &self.repr {
            IntMapRepr::Array(array) => 1 + array.size(),
            _ => 1 + self.entry_no,
        }
    }

    #[must_use]
    pub fn constant_mem_storage_estimate(&self) -> usize {
        INTMAPCELL_MEM.saturating_add(match &self.repr {
            IntMapRepr::Array(array) => pdarray_storage_estimate(array.size()),
            IntMapRepr::Tree(_) => self.entry_no.saturating_mul(NUMTREECELL_MEM),
            IntMapRepr::Empty | IntMapRepr::Single { .. } => 0,
        })
    }

    fn ensure_ref_slot(&mut self, key: IntMapKey) {
        match &self.repr {
            IntMapRepr::Empty => {
                self.repr = IntMapRepr::Single { key, value: None };
                self.min_key = key;
                self.max_key = key;
                self.entry_no = 1;
            }
            IntMapRepr::Single {
                key: single_key,
                value,
            } => {
                if *single_key == key {
                    return;
                }

                let old_key = *single_key;
                let old_value = value.clone();
                if switch_to_array(key, self.min_key, self.max_key, 2) {
                    let mut array = PDPointerRangeArr::new_pointer(old_key.min(key), IM_ARRAY_SIZE);
                    array.assign(old_key, old_value);
                    array.assign(key, None);
                    self.repr = IntMapRepr::Array(array);
                } else {
                    let mut tree = BTreeMap::new();
                    tree.insert(old_key, old_value);
                    tree.insert(key, None);
                    self.repr = IntMapRepr::Tree(tree);
                }
                self.entry_no = 2;
                self.min_key = self.min_key.min(key);
                self.max_key = self.max_key.max(key);
            }
            IntMapRepr::Array(_) => {
                if ((key > self.max_key) || (key < self.min_key))
                    && switch_to_tree(self.min_key, self.max_key, key, self.entry_no + 1)
                {
                    self.array_to_tree();
                    self.ensure_ref_slot(key);
                    return;
                }

                let was_none = match &mut self.repr {
                    IntMapRepr::Array(array) => array.element_ref(key).is_none(),
                    _ => unreachable!("IntMap representation changed unexpectedly"),
                };
                if was_none {
                    self.entry_no += 1;
                }
                self.min_key = self.min_key.min(key);
                self.max_key = self.max_key.max(key);
            }
            IntMapRepr::Tree(tree) => {
                if tree.contains_key(&key) {
                    return;
                }
                if switch_to_array(self.min_key, self.max_key, key, self.entry_no + 1) {
                    self.tree_to_array();
                    self.ensure_ref_slot(key);
                } else if let IntMapRepr::Tree(tree) = &mut self.repr {
                    tree.insert(key, None);
                    self.entry_no += 1;
                    self.max_key = self.max_key.max(key);
                    self.min_key = self.min_key.min(key);
                }
            }
        }
    }

    fn array_to_tree(&mut self) {
        let IntMapRepr::Array(array) = &self.repr else {
            debug_assert!(false, "array_to_tree called for non-array IntMap");
            return;
        };

        let mut tree = BTreeMap::new();
        let mut entry_no = 0_usize;
        let mut max_key = self.min_key;
        let mut min_key = self.max_key;
        let mut key = array.low_key();
        while key <= self.max_key {
            if let Some(Some(value)) = array.existing_element(key) {
                tree.insert(key, Some(value.clone()));
                entry_no += 1;
                max_key = key;
                min_key = min_key.min(key);
            }
            if key == IntMapKey::MAX {
                break;
            }
            key += 1;
        }
        self.max_key = max_key;
        self.min_key = min_key.min(max_key);
        self.entry_no = entry_no;
        self.repr = IntMapRepr::Tree(tree);
    }

    fn tree_to_array(&mut self) {
        let IntMapRepr::Tree(tree) = &self.repr else {
            debug_assert!(false, "tree_to_array called for non-tree IntMap");
            return;
        };

        let mut array = PDPointerRangeArr::new_pointer(self.min_key, IM_ARRAY_SIZE);
        let mut entry_no = 0_usize;
        let mut max_key = self.min_key;
        let mut min_key = self.max_key;
        for (key, value) in tree {
            if let Some(value) = value {
                array.assign(*key, Some(value.clone()));
                entry_no += 1;
                max_key = *key;
                min_key = min_key.min(*key);
            }
        }
        self.max_key = max_key;
        self.min_key = min_key.min(max_key);
        self.entry_no = entry_no;
        self.repr = IntMapRepr::Array(array);
    }
}

fn switch_to_array(
    old_min: IntMapKey,
    old_max: IntMapKey,
    new_key: IntMapKey,
    entries: usize,
) -> bool {
    let max_key = old_max.max(new_key);
    let min_key = old_min.min(new_key);
    let span = max_key.saturating_sub(min_key);
    entries_as_i128(entries) * entries_as_i128(MIN_TREE_DENSITY) > key_distance_as_i128(span)
}

fn switch_to_tree(
    old_min: IntMapKey,
    old_max: IntMapKey,
    new_key: IntMapKey,
    entries: usize,
) -> bool {
    let max_key = old_max.max(new_key);
    let min_key = old_min.min(new_key);
    let span = max_key.saturating_sub(min_key);
    entries_as_i128(entries) * entries_as_i128(MAX_TREE_DENSITY) < key_distance_as_i128(span)
}

fn entries_as_i128(entries: usize) -> i128 {
    i128::try_from(entries).unwrap_or(i128::MAX)
}

fn key_distance_as_i128(distance: IntMapKey) -> i128 {
    i128::try_from(distance).unwrap_or(i128::MAX)
}

const fn pdarray_storage_estimate(size: usize) -> usize {
    PDARRAYCELL_MEM
        .saturating_add(INTORP_MEM)
        .saturating_add(size.saturating_mul(INTORP_MEM))
}

#[cfg(test)]
mod tests {
    use super::{IntMap, IntMapType, INTMAPCELL_MEM, INTORP_MEM, NUMTREECELL_MEM, PDARRAYCELL_MEM};

    #[test]
    fn get_ref_creates_single_slot_and_assign_overwrites() {
        let mut map = IntMap::new();
        assert_eq!(map.map_type(), IntMapType::Empty);

        let slot = map.get_ref(10);
        assert_eq!(slot, &None);
        *slot = Some("ten");

        assert_eq!(map.map_type(), IntMapType::Single);
        assert_eq!(map.entry_count_estimate(), 1);
        assert_eq!(map.get_val(10), Some(&"ten"));
        map.assign(10, "TEN");
        assert_eq!(map.get_val(10), Some(&"TEN"));
    }

    #[test]
    fn dense_second_key_switches_single_to_array() {
        let mut map = IntMap::new();
        map.assign(10, "ten");
        assert_eq!(map.map_type(), IntMapType::Single);

        assert_eq!(map.get_ref(11), &None);
        assert_eq!(map.map_type(), IntMapType::Array);
        assert_eq!(map.min_key(), Some(10));
        assert_eq!(map.max_key(), Some(11));
        assert_eq!(map.entry_count_estimate(), 2);
    }

    #[test]
    fn sparse_array_insertion_switches_to_tree_and_iteration_skips_empty_slots() {
        let mut map = IntMap::new();
        map.assign(10, "ten");
        map.assign(11, "eleven");
        assert_eq!(map.map_type(), IntMapType::Array);

        assert_eq!(map.get_ref(100), &None);
        assert_eq!(map.map_type(), IntMapType::Tree);
        assert_eq!(map.get_val(100), None);

        let entries = map.entries();
        assert_eq!(entries, vec![(10, &"ten"), (11, &"eleven")]);
    }

    #[test]
    fn tree_switches_back_to_array_when_density_increases() {
        let mut map = IntMap::new();
        map.assign(0, "zero");
        map.assign(1, "one");
        assert_eq!(map.map_type(), IntMapType::Array);

        map.assign(100, "hundred");
        assert_eq!(map.map_type(), IntMapType::Tree);

        for key in 2..=25 {
            map.assign(key, "dense");
        }
        assert_eq!(map.map_type(), IntMapType::Array);
        assert_eq!(map.get_val(100), Some(&"hundred"));
    }

    #[test]
    fn repeated_array_get_ref_on_null_slot_inflates_entry_estimate_like_c() {
        let mut map: IntMap<&str> = IntMap::new();
        map.assign(0, "zero");
        assert_eq!(map.get_ref(1), &None);
        assert_eq!(map.entry_count_estimate(), 2);
        assert_eq!(map.get_ref(1), &None);
        assert_eq!(map.entry_count_estimate(), 3);
    }

    #[test]
    fn array_get_val_below_low_key_grows_backing_range_like_c() {
        let mut map = IntMap::new();
        map.assign(10, "ten");
        map.assign(11, "eleven");
        assert_eq!(map.map_type(), IntMapType::Array);
        assert_eq!(map.min_key(), Some(10));
        assert_eq!(map.max_key(), Some(11));
        let before_storage = map.storage_estimate();

        assert_eq!(map.get_val(9), None);

        assert!(map.storage_estimate() > before_storage);
        assert_eq!(map.min_key(), Some(10));
        assert_eq!(map.max_key(), Some(11));
        assert_eq!(map.entry_count_estimate(), 2);
    }

    #[test]
    fn array_delete_miss_below_low_key_grows_backing_range_like_c() {
        let mut map = IntMap::new();
        map.assign(10, "ten");
        map.assign(11, "eleven");
        assert_eq!(map.map_type(), IntMapType::Array);
        let before_storage = map.storage_estimate();

        assert_eq!(map.del_key(9), None);

        assert!(map.storage_estimate() > before_storage);
        assert_eq!(map.min_key(), Some(10));
        assert_eq!(map.max_key(), Some(11));
        assert_eq!(map.entry_count_estimate(), 2);
        assert_eq!(map.get_val(10), Some(&"ten"));
    }

    #[test]
    fn deletion_returns_existing_values_and_can_switch_array_to_tree() {
        let mut map = IntMap::new();
        for key in 0..=20 {
            map.assign(key, key);
        }
        assert_eq!(map.map_type(), IntMapType::Array);

        for key in 0..=18 {
            assert_eq!(map.del_key(key), Some(key));
        }
        assert_eq!(map.map_type(), IntMapType::Tree);
        assert_eq!(map.del_key(99), None);
        assert_eq!(map.del_key(20), Some(20));
        assert_eq!(map.get_val(20), None);
    }

    #[test]
    fn deleting_empty_single_slot_returns_none_but_clears_map() {
        let mut map: IntMap<&str> = IntMap::new();
        assert_eq!(map.get_ref(4), &None);
        assert_eq!(map.del_key(4), None);
        assert_eq!(map.map_type(), IntMapType::Empty);
        assert_eq!(map.entry_count_estimate(), 0);
    }

    #[test]
    fn range_iteration_is_inclusive_and_sorted() {
        let mut map = IntMap::new();
        for key in [5, 1, 3, 7] {
            map.assign(key, key * 10);
        }

        let entries = map.iter_range(3, 6);
        assert_eq!(entries, vec![(3, &30), (5, &50)]);
    }

    #[test]
    fn c_iterator_for_array_starts_at_raw_lower_key_and_grows_backing_range() {
        let mut map = IntMap::new();
        map.assign(10, "ten");
        map.assign(11, "eleven");
        assert_eq!(map.map_type(), IntMapType::Array);
        let before_storage = map.storage_estimate();

        let entries = map.iter_range_c_mut(9, 11);

        assert_eq!(entries, vec![(10, "ten"), (11, "eleven")]);
        assert!(map.storage_estimate() > before_storage);
        assert_eq!(map.min_key(), Some(10));
        assert_eq!(map.max_key(), Some(11));
        assert_eq!(map.entry_count_estimate(), 2);
    }

    #[test]
    fn ordinary_iter_range_stays_non_mutating_for_array_lower_miss() {
        let mut map = IntMap::new();
        map.assign(10, "ten");
        map.assign(11, "eleven");
        assert_eq!(map.map_type(), IntMapType::Array);
        let before_storage = map.storage_estimate();

        let entries = map.iter_range(9, 11);

        assert_eq!(entries, vec![(10, &"ten"), (11, &"eleven")]);
        assert_eq!(map.storage_estimate(), before_storage);
        assert_eq!(map.min_key(), Some(10));
        assert_eq!(map.max_key(), Some(11));
        assert_eq!(map.entry_count_estimate(), 2);
    }

    #[test]
    fn c_iterator_returns_empty_for_range_above_tree_bounds() {
        let mut map = IntMap::new();
        map.assign(10, "ten");
        map.assign(11, "eleven");
        map.assign(100, "hundred");
        assert_eq!(map.map_type(), IntMapType::Tree);

        let entries = map.iter_range_c_mut(101, 200);

        assert!(entries.is_empty());
        assert_eq!(map.min_key(), Some(10));
        assert_eq!(map.max_key(), Some(100));
        assert_eq!(map.entry_count_estimate(), 3);
    }

    #[test]
    fn debug_print_uses_comment_prefixed_c_shape() {
        let mut map = IntMap::new();
        map.assign(2, "two");
        assert_eq!(
            map.debug_print_string(),
            "% ==== IntMapType 1 Size = 2\n%     2 : two\n% ==== IntMap End\n"
        );
    }

    #[test]
    fn constant_mem_storage_estimate_matches_c_macro_shapes() {
        let mut dense = IntMap::new();
        assert_eq!(dense.constant_mem_storage_estimate(), INTMAPCELL_MEM);

        dense.assign(0, "zero");
        assert_eq!(dense.constant_mem_storage_estimate(), INTMAPCELL_MEM);

        dense.assign(1, "one");
        assert_eq!(
            dense.constant_mem_storage_estimate(),
            INTMAPCELL_MEM + PDARRAYCELL_MEM + INTORP_MEM + 8 * INTORP_MEM
        );

        let mut sparse = IntMap::new();
        sparse.assign(100, "hundred");
        sparse.assign(0, "zero");
        assert_eq!(
            sparse.constant_mem_storage_estimate(),
            INTMAPCELL_MEM + 2 * NUMTREECELL_MEM
        );
    }

    #[test]
    fn sparse_second_key_preserves_c_single_insertion_order_asymmetry() {
        let mut ascending = IntMap::new();
        ascending.assign(0, "zero");
        ascending.assign(100, "hundred");

        let mut descending = IntMap::new();
        descending.assign(100, "hundred");
        descending.assign(0, "zero");

        assert_eq!(ascending.entries(), descending.entries());
        assert_eq!(ascending.map_type(), IntMapType::Array);
        assert_eq!(descending.map_type(), IntMapType::Tree);
        assert_eq!(
            ascending.constant_mem_storage_estimate(),
            INTMAPCELL_MEM + PDARRAYCELL_MEM + INTORP_MEM + 104 * INTORP_MEM
        );
        assert_eq!(
            descending.constant_mem_storage_estimate(),
            INTMAPCELL_MEM + 2 * NUMTREECELL_MEM
        );
    }
}
