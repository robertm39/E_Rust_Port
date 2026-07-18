use crate::basics::defines::{IntOrP, IntOrPInt};
use std::cmp::Ordering;

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
struct QuadTreeNode<P, V> {
    key: QuadKey<P>,
    value: V,
    left: Option<usize>,
    right: Option<usize>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QuadTree<P, V> {
    nodes: Vec<Option<QuadTreeNode<P, V>>>,
    free: Vec<usize>,
    root: Option<usize>,
    len: usize,
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
            nodes: Vec::new(),
            free: Vec::new(),
            root: None,
            len: 0,
        }
    }

    #[must_use]
    pub const fn nodes(&self) -> usize {
        self.len
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    #[must_use]
    pub fn root_key(&self) -> Option<&QuadKey<P>> {
        self.root.map(|root| &self.node(root).key)
    }

    pub fn insert_entry(&mut self, entry: QuadTreeEntry<P, V>) -> Option<&V> {
        let (key, value) = entry.into_parts();
        let Some(root) = self.root else {
            self.root = Some(self.alloc_node(key, value));
            return None;
        };

        let root = self.splay(root, &key);
        self.root = Some(root);
        match key.cmp(&self.node(root).key) {
            Ordering::Less => {
                let left = self.node(root).left;
                let new_root = self.alloc_node(key, value);
                self.node_mut(new_root).left = left;
                self.node_mut(new_root).right = Some(root);
                self.node_mut(root).left = None;
                self.root = Some(new_root);
                None
            }
            Ordering::Greater => {
                let right = self.node(root).right;
                let new_root = self.alloc_node(key, value);
                self.node_mut(new_root).right = right;
                self.node_mut(new_root).left = Some(root);
                self.node_mut(root).right = None;
                self.root = Some(new_root);
                None
            }
            Ordering::Equal => Some(&self.node(root).value),
        }
    }

    pub fn store(&mut self, key: QuadKey<P>, value: V) -> bool {
        self.insert_entry(QuadTreeEntry::new(key, value)).is_none()
    }

    pub fn find(&mut self, key: &QuadKey<P>) -> Option<&V> {
        self.find_splayed(key)
    }

    pub fn find_splayed(&mut self, key: &QuadKey<P>) -> Option<&V> {
        let root = self.splay(self.root?, key);
        self.root = Some(root);
        (self.node(root).key.cmp(key) == Ordering::Equal).then(|| &self.node(root).value)
    }

    pub fn find_mut(&mut self, key: &QuadKey<P>) -> Option<&mut V> {
        let root = self.splay(self.root?, key);
        self.root = Some(root);
        (self.node(root).key.cmp(key) == Ordering::Equal).then(|| &mut self.node_mut(root).value)
    }

    /// # Panics
    ///
    /// Panics if the tree's internal root index does not refer to a live node.
    pub fn extract_entry(&mut self, key: &QuadKey<P>) -> Option<QuadTreeEntry<P, V>> {
        let root = self.splay(self.root?, key);
        self.root = Some(root);
        if self.node(root).key.cmp(key) != Ordering::Equal {
            return None;
        }

        let left = self.node(root).left;
        let right = self.node(root).right;
        let new_root = if let Some(left) = left {
            let left = self.splay(left, key);
            self.node_mut(left).right = right;
            Some(left)
        } else {
            right
        };
        let removed = self.nodes[root]
            .take()
            .expect("QuadTree root must refer to a live node");
        self.free.push(root);
        self.len -= 1;
        self.root = new_root;
        Some(QuadTreeEntry::new(removed.key, removed.value))
    }

    pub fn delete_entry(&mut self, key: &QuadKey<P>) -> bool {
        self.extract_entry(key).is_some()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&QuadKey<P>, &V)> {
        QuadTreeIter::new(self)
    }

    #[must_use]
    pub fn entries_vec(&self) -> Vec<(&QuadKey<P>, &V)> {
        self.iter().collect()
    }

    fn alloc_node(&mut self, key: QuadKey<P>, value: V) -> usize {
        let node = QuadTreeNode {
            key,
            value,
            left: None,
            right: None,
        };
        self.len += 1;
        if let Some(index) = self.free.pop() {
            self.nodes[index] = Some(node);
            index
        } else {
            self.nodes.push(Some(node));
            self.nodes.len() - 1
        }
    }

    fn node(&self, index: usize) -> &QuadTreeNode<P, V> {
        self.nodes[index]
            .as_ref()
            .expect("QuadTree link must refer to a live node")
    }

    fn node_mut(&mut self, index: usize) -> &mut QuadTreeNode<P, V> {
        self.nodes[index]
            .as_mut()
            .expect("QuadTree link must refer to a live node")
    }

    fn splay(&mut self, root: usize, key: &QuadKey<P>) -> usize {
        let mut tree = root;
        let mut lower_root = None;
        let mut lower_tail = None;
        let mut upper_root = None;
        let mut upper_tail = None;

        loop {
            match key.cmp(&self.node(tree).key) {
                Ordering::Less => {
                    let Some(left) = self.node(tree).left else {
                        break;
                    };
                    if key < &self.node(left).key {
                        let left_right = self.node(left).right;
                        self.node_mut(tree).left = left_right;
                        self.node_mut(left).right = Some(tree);
                        tree = left;
                        if self.node(tree).left.is_none() {
                            break;
                        }
                    }
                    if let Some(tail) = upper_tail {
                        self.node_mut(tail).left = Some(tree);
                    } else {
                        upper_root = Some(tree);
                    }
                    upper_tail = Some(tree);
                    tree = self
                        .node(tree)
                        .left
                        .expect("QuadTree splay left link must exist");
                }
                Ordering::Greater => {
                    let Some(right) = self.node(tree).right else {
                        break;
                    };
                    if key > &self.node(right).key {
                        let right_left = self.node(right).left;
                        self.node_mut(tree).right = right_left;
                        self.node_mut(right).left = Some(tree);
                        tree = right;
                        if self.node(tree).right.is_none() {
                            break;
                        }
                    }
                    if let Some(tail) = lower_tail {
                        self.node_mut(tail).right = Some(tree);
                    } else {
                        lower_root = Some(tree);
                    }
                    lower_tail = Some(tree);
                    tree = self
                        .node(tree)
                        .right
                        .expect("QuadTree splay right link must exist");
                }
                Ordering::Equal => break,
            }
        }

        let tree_left = self.node(tree).left;
        let tree_right = self.node(tree).right;
        if let Some(tail) = lower_tail {
            self.node_mut(tail).right = tree_left;
        } else {
            lower_root = tree_left;
        }
        if let Some(tail) = upper_tail {
            self.node_mut(tail).left = tree_right;
        } else {
            upper_root = tree_right;
        }
        self.node_mut(tree).left = lower_root;
        self.node_mut(tree).right = upper_root;
        tree
    }

    #[cfg(test)]
    fn pre_order_keys(&self) -> Vec<QuadKey<P>> {
        let mut result = Vec::with_capacity(self.len);
        let mut pending = Vec::new();
        if let Some(root) = self.root {
            pending.push(root);
        }
        while let Some(index) = pending.pop() {
            result.push(self.node(index).key.clone());
            if let Some(right) = self.node(index).right {
                pending.push(right);
            }
            if let Some(left) = self.node(index).left {
                pending.push(left);
            }
        }
        result
    }
}

struct QuadTreeIter<'tree, P, V> {
    tree: &'tree QuadTree<P, V>,
    pending: Vec<usize>,
    current: Option<usize>,
}

impl<'tree, P, V> QuadTreeIter<'tree, P, V>
where
    P: Ord + Clone,
{
    fn new(tree: &'tree QuadTree<P, V>) -> Self {
        Self {
            tree,
            pending: Vec::new(),
            current: tree.root,
        }
    }
}

impl<'tree, P, V> Iterator for QuadTreeIter<'tree, P, V>
where
    P: Ord + Clone,
{
    type Item = (&'tree QuadKey<P>, &'tree V);

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(current) = self.current {
            self.pending.push(current);
            self.current = self.tree.node(current).left;
        }
        let next = self.pending.pop()?;
        self.current = self.tree.node(next).right;
        let node = self.tree.node(next);
        Some((&node.key, &node.value))
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
    use std::collections::BTreeMap;

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
    fn splayed_find_tracks_recent_root_like_c() {
        let mut tree = QuadTree::new();
        let first_key = key(1, 0, 2, 0);
        let second_key = key(2, 0, 3, 0);
        tree.store(first_key.clone(), 10);
        tree.store(second_key.clone(), 20);

        assert_eq!(tree.find_splayed(&first_key), Some(&10));
        assert_eq!(tree.root_key(), Some(&first_key));
        assert_eq!(tree.find_splayed(&key(9, 0, 9, 0)), None);
        assert_eq!(tree.root_key(), Some(&second_key));
    }

    #[test]
    fn splayed_misses_move_nearest_boundary_key_to_root() {
        let mut tree = QuadTree::new();
        let keys = [key(4, 0, 0, 0), key(2, 0, 0, 0), key(6, 0, 0, 0)];
        for key in &keys {
            assert!(tree.store(key.clone(), key.p1));
        }

        assert_eq!(tree.find(&key(1, 0, 0, 0)), None);
        assert_eq!(tree.root_key(), Some(&keys[1]));
        assert_eq!(tree.find(&key(9, 0, 0, 0)), None);
        assert_eq!(tree.root_key(), Some(&keys[2]));
        assert_eq!(
            tree.pre_order_keys(),
            vec![keys[2].clone(), keys[0].clone(), keys[1].clone()]
        );
    }

    #[test]
    fn operation_trace_matches_unchanged_c_splay_topology() {
        let mut tree = QuadTree::new();
        let a = key(4, 0, 40, 0);
        let b = key(2, 0, 20, 0);
        let c = key(6, 0, 60, 0);
        let d = key(4, -1, 40, 0);
        let low = key(1, 0, 10, 0);
        let high = key(9, 0, 90, 0);
        let middle_miss = key(5, 0, 50, 0);

        assert!(tree.store(a.clone(), 40));
        assert_eq!(tree.pre_order_keys(), vec![a.clone()]);
        assert!(tree.store(b.clone(), 20));
        assert_eq!(tree.pre_order_keys(), vec![b.clone(), a.clone()]);
        assert!(tree.store(c.clone(), 60));
        assert_eq!(tree.pre_order_keys(), vec![c.clone(), a.clone(), b.clone()]);
        assert!(tree.store(d.clone(), 39));
        assert_eq!(
            tree.pre_order_keys(),
            vec![d.clone(), b.clone(), a.clone(), c.clone()]
        );
        assert!(!tree.store(a.clone(), 999));
        assert_eq!(
            tree.pre_order_keys(),
            vec![a.clone(), d.clone(), b.clone(), c.clone()]
        );

        assert_eq!(tree.find(&b), Some(&20));
        assert_eq!(
            tree.pre_order_keys(),
            vec![b.clone(), d.clone(), a.clone(), c.clone()]
        );
        assert_eq!(tree.find(&low), None);
        assert_eq!(
            tree.pre_order_keys(),
            vec![b.clone(), d.clone(), a.clone(), c.clone()]
        );
        assert_eq!(tree.find(&high), None);
        assert_eq!(
            tree.pre_order_keys(),
            vec![c.clone(), d.clone(), b.clone(), a.clone()]
        );
        assert_eq!(tree.find(&a), Some(&40));
        assert_eq!(
            tree.pre_order_keys(),
            vec![a.clone(), d.clone(), b.clone(), c.clone()]
        );

        assert!(tree.extract_entry(&middle_miss).is_none());
        assert_eq!(
            tree.pre_order_keys(),
            vec![c.clone(), a.clone(), d.clone(), b.clone()]
        );
        assert_eq!(
            tree.extract_entry(&d).map(QuadTreeEntry::into_parts),
            Some((d, 39))
        );
        assert_eq!(tree.pre_order_keys(), vec![b.clone(), a.clone(), c.clone()]);
        assert!(tree.delete_entry(&c));
        assert_eq!(tree.pre_order_keys(), vec![a, b]);
    }

    #[test]
    fn mixed_operations_preserve_ordered_map_invariants() {
        let mut tree = QuadTree::new();
        let mut expected = BTreeMap::new();
        let mut random = 0xD1B5_4A32_D192_ED03_u64;

        for step in 0..2_000_i64 {
            random = random
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            let candidate = key(
                usize::try_from((random >> 8) & 31).unwrap(),
                i32::try_from((random >> 16) & 7).unwrap() - 3,
                usize::try_from((random >> 24) & 31).unwrap(),
                i32::try_from((random >> 32) & 7).unwrap() - 3,
            );

            match random % 5 {
                0 => {
                    let inserted = !expected.contains_key(&candidate);
                    if inserted {
                        expected.insert(candidate.clone(), step);
                    }
                    assert_eq!(tree.store(candidate, step), inserted);
                }
                1 => {
                    assert_eq!(
                        tree.find(&candidate).copied(),
                        expected.get(&candidate).copied()
                    );
                }
                2 => {
                    assert_eq!(
                        tree.delete_entry(&candidate),
                        expected.remove(&candidate).is_some()
                    );
                }
                3 => {
                    let actual = tree
                        .extract_entry(&candidate)
                        .map(QuadTreeEntry::into_parts);
                    let modeled = expected.remove_entry(&candidate);
                    assert_eq!(actual, modeled);
                }
                _ => {
                    let modeled = expected.get_mut(&candidate);
                    let actual = tree.find_mut(&candidate);
                    assert_eq!(actual.is_some(), modeled.is_some());
                    if let (Some(actual), Some(modeled)) = (actual, modeled) {
                        *actual = actual.wrapping_add(1);
                        *modeled = modeled.wrapping_add(1);
                    }
                }
            }

            assert_eq!(tree.nodes(), expected.len());
            assert_eq!(tree.is_empty(), expected.is_empty());
            assert_eq!(
                tree.iter()
                    .map(|(key, value)| (key.clone(), *value))
                    .collect::<Vec<_>>(),
                expected
                    .iter()
                    .map(|(key, value)| (key.clone(), *value))
                    .collect::<Vec<_>>()
            );
            assert_eq!(tree.root_key().is_none(), expected.is_empty());
        }
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
