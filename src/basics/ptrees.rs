use std::collections::BTreeSet;
use std::fmt::Write as _;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PTree<K> {
    keys: BTreeSet<K>,
    root_key: Option<K>,
}

impl<K> Default for PTree<K>
where
    K: Ord + Clone,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<K> PTree<K>
where
    K: Ord + Clone,
{
    #[must_use]
    pub const fn new() -> Self {
        Self {
            keys: BTreeSet::new(),
            root_key: None,
        }
    }

    #[must_use]
    pub fn nodes(&self) -> usize {
        self.keys.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }

    #[must_use]
    pub const fn root_key(&self) -> Option<&K> {
        self.root_key.as_ref()
    }

    pub fn store(&mut self, key: K) -> bool {
        self.root_key = Some(key.clone());
        self.keys.insert(key)
    }

    #[must_use]
    pub fn find(&self, key: &K) -> Option<&K> {
        self.keys.get(key)
    }

    #[must_use]
    pub fn find_binary(&self, key: &K) -> Option<&K> {
        self.find(key)
    }

    pub fn extract_key(&mut self, key: &K) -> Option<K> {
        let removed = self.keys.take(key);
        if removed.is_some() {
            self.root_key = self.keys.iter().next().cloned();
        }
        removed
    }

    pub fn extract_root_key(&mut self) -> Option<K> {
        let key = match self.root_key.as_ref() {
            Some(key) if self.keys.contains(key) => key.clone(),
            _ => self.keys.iter().next()?.clone(),
        };
        self.extract_key(&key)
    }

    pub fn delete_entry(&mut self, key: &K) -> bool {
        self.extract_key(key).is_some()
    }

    pub fn merge(&mut self, add: Self) -> bool {
        let before = self.keys.len();
        for key in add.keys {
            self.store(key);
        }
        self.keys.len() != before
    }

    pub fn insert_tree(&mut self, add: &Self) {
        for key in &add.keys {
            self.store(key.clone());
        }
    }

    pub fn from_stack<I>(values: I) -> (Self, usize)
    where
        I: IntoIterator<Item = K>,
    {
        let mut tree = Self::new();
        let inserted = tree.insert_stack(values);
        (tree, inserted)
    }

    pub fn insert_stack<I>(&mut self, values: I) -> usize
    where
        I: IntoIterator<Item = K>,
    {
        let mut inserted = 0_usize;
        for key in values {
            if self.store(key) {
                inserted += 1;
            }
        }
        inserted
    }

    #[must_use]
    pub fn to_stack(&self) -> Vec<K> {
        self.keys.iter().cloned().collect()
    }

    #[must_use]
    pub fn shared_element(&self, other: &Self) -> Option<K> {
        other
            .keys
            .iter()
            .find(|key| self.keys.contains(*key))
            .cloned()
    }

    #[must_use]
    pub fn intersection(&self, other: &Self) -> Self {
        let mut result = Self::new();
        for key in self.keys.intersection(&other.keys) {
            result.store(key.clone());
        }
        result
    }

    #[must_use]
    pub fn copy_tree(&self) -> Self {
        self.clone()
    }

    pub fn destructive_intersection(&mut self, other: &Self) -> usize {
        let before = self.keys.len();
        self.keys.retain(|key| other.keys.contains(key));
        self.root_key = self.keys.iter().next().cloned();
        before - self.keys.len()
    }

    #[must_use]
    pub fn equivalent(&self, other: &Self) -> bool {
        self.keys == other.keys
    }

    #[must_use]
    pub fn is_subset_of(&self, other: &Self) -> bool {
        self.keys.is_subset(&other.keys)
    }

    pub fn visit_in_order<F>(&self, mut visitor: F)
    where
        F: FnMut(&K),
    {
        for key in &self.keys {
            visitor(key);
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &K> {
        self.keys.iter()
    }

    #[must_use]
    pub fn debug_print_string(&self) -> String
    where
        K: std::fmt::Display,
    {
        let mut result = String::new();
        for (count, key) in self.keys.iter().enumerate() {
            if count.is_multiple_of(10) {
                result.push_str("\n%");
            }
            let write_result = write!(&mut result, " {key:>7}");
            debug_assert!(write_result.is_ok());
        }
        result.push('\n');
        result
    }
}

#[cfg(test)]
mod tests {
    use super::PTree;

    fn tree(values: &[i32]) -> PTree<i32> {
        let (tree, _inserted) = PTree::from_stack(values.iter().copied());
        tree
    }

    #[test]
    fn store_find_and_duplicates_match_c_contract() {
        let mut tree = PTree::new();
        assert!(tree.store(10));
        assert!(tree.store(3));
        assert!(!tree.store(10));
        assert_eq!(tree.nodes(), 2);
        assert_eq!(tree.root_key(), Some(&10));
        assert_eq!(tree.find(&3), Some(&3));
        assert_eq!(tree.find_binary(&99), None);
    }

    #[test]
    fn extract_root_delete_and_stack_conversion_work() {
        let (mut tree, inserted) = PTree::from_stack([3, 1, 3, 2]);
        assert_eq!(inserted, 3);
        assert_eq!(tree.insert_stack([2, 4, 4]), 1);
        assert_eq!(tree.to_stack(), vec![1, 2, 3, 4]);
        assert_eq!(tree.extract_key(&4), Some(4));
        assert!(!tree.delete_entry(&9));
        assert!(tree.delete_entry(&1));
        assert_eq!(tree.extract_root_key(), Some(2));
        assert_eq!(tree.to_stack(), vec![3]);
    }

    #[test]
    fn merge_consumes_source_and_reports_new_elements() {
        let mut base = tree(&[1, 2]);
        assert!(base.merge(tree(&[2, 3, 4])));
        assert_eq!(base.to_stack(), vec![1, 2, 3, 4]);
        assert!(!base.merge(tree(&[1, 2])));
    }

    #[test]
    fn insert_tree_preserves_source_and_intersections_match_sets() {
        let mut base = tree(&[1, 4]);
        let add = tree(&[2, 4]);
        base.insert_tree(&add);
        assert_eq!(base.to_stack(), vec![1, 2, 4]);
        assert_eq!(add.to_stack(), vec![2, 4]);

        let intersection = base.intersection(&tree(&[2, 3, 4]));
        assert_eq!(intersection.to_stack(), vec![2, 4]);
        assert_eq!(intersection.root_key(), Some(&4));
        assert_eq!(base.shared_element(&tree(&[9, 4, 2])), Some(2));
    }

    #[test]
    fn copy_destructive_intersection_equivalence_and_subset_match_c_helpers() {
        let mut base = tree(&[1, 2, 3, 4]);
        let copied = base.copy_tree();
        assert_eq!(copied.to_stack(), vec![1, 2, 3, 4]);
        let removed = base.destructive_intersection(&tree(&[2, 4, 6]));
        assert_eq!(removed, 2);
        assert_eq!(base.to_stack(), vec![2, 4]);
        assert!(base.equivalent(&tree(&[4, 2])));
        assert!(base.is_subset_of(&tree(&[1, 2, 3, 4])));
        assert!(!tree(&[1, 9]).is_subset_of(&base));
    }

    #[test]
    fn visit_in_order_and_debug_print_are_deterministic() {
        let tree = tree(&[3, 1, 2]);
        let mut visited = Vec::new();
        tree.visit_in_order(|key| visited.push(*key));
        assert_eq!(visited, vec![1, 2, 3]);
        assert_eq!(tree.debug_print_string(), "\n%       1       2       3\n");
    }
}
