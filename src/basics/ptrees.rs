use std::cmp::Ordering;
use std::fmt::Write as _;

#[derive(Clone, Debug, Eq, PartialEq)]
struct PTreeNode<K> {
    key: K,
    left: Option<usize>,
    right: Option<usize>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PTree<K> {
    nodes: Vec<Option<PTreeNode<K>>>,
    free: Vec<usize>,
    root: Option<usize>,
    len: usize,
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
    pub fn root_key(&self) -> Option<&K> {
        self.root.map(|root| &self.node(root).key)
    }

    pub fn store(&mut self, key: K) -> bool {
        let Some(root) = self.root else {
            self.root = Some(self.alloc_node(key));
            return true;
        };

        let root = self.splay(root, &key);
        self.root = Some(root);
        match key.cmp(&self.node(root).key) {
            Ordering::Less => {
                let left = self.node(root).left;
                let new_root = self.alloc_node(key);
                self.node_mut(new_root).left = left;
                self.node_mut(new_root).right = Some(root);
                self.node_mut(root).left = None;
                self.root = Some(new_root);
                true
            }
            Ordering::Greater => {
                let right = self.node(root).right;
                let new_root = self.alloc_node(key);
                self.node_mut(new_root).right = right;
                self.node_mut(new_root).left = Some(root);
                self.node_mut(root).right = None;
                self.root = Some(new_root);
                true
            }
            Ordering::Equal => false,
        }
    }

    #[must_use]
    pub fn find(&self, key: &K) -> Option<&K> {
        self.find_index(key).map(|index| &self.node(index).key)
    }

    pub fn find_splayed(&mut self, key: &K) -> Option<&K> {
        let root = self.splay(self.root?, key);
        self.root = Some(root);
        (self.node(root).key.cmp(key) == Ordering::Equal).then(|| &self.node(root).key)
    }

    #[must_use]
    pub fn find_binary(&self, key: &K) -> Option<&K> {
        self.find(key)
    }

    pub fn extract_key(&mut self, key: &K) -> Option<K> {
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
            .expect("PTree root must refer to a live node");
        self.free.push(root);
        self.len -= 1;
        self.root = new_root;
        Some(removed.key)
    }

    pub fn extract_root_key(&mut self) -> Option<K> {
        let key = self.root_key()?.clone();
        self.extract_key(&key)
    }

    pub fn delete_entry(&mut self, key: &K) -> bool {
        self.extract_key(key).is_some()
    }

    pub fn merge(&mut self, add: Self) -> bool {
        let before = self.len;
        for key in add.to_stack() {
            self.store(key);
        }
        self.len != before
    }

    pub fn insert_tree(&mut self, add: &Self) {
        for key in add.to_stack() {
            self.store(key);
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
        values
            .into_iter()
            .filter(|key| self.store(key.clone()))
            .count()
    }

    #[must_use]
    pub fn to_stack(&self) -> Vec<K> {
        self.c_stack_order()
            .into_iter()
            .map(|index| self.node(index).key.clone())
            .collect()
    }

    pub fn shared_element(&mut self, other: &Self) -> Option<K> {
        for key in other.to_stack() {
            if let Some(found) = self.find_splayed(&key) {
                return Some(found.clone());
            }
        }
        None
    }

    #[must_use]
    pub fn intersection(&self, other: &Self) -> Self {
        let mut result = Self::new();
        for key in other.to_stack() {
            if self.find_binary(&key).is_some() {
                result.store(key);
            }
        }
        result
    }

    #[must_use]
    pub fn copy_tree(&self) -> Self {
        let mut result = Self::new();
        for key in self.to_stack() {
            result.store(key);
        }
        result
    }

    pub fn destructive_intersection(&mut self, other: &Self) -> usize {
        let mut result = Self::new();
        let mut removed = 0;
        while let Some(key) = self.extract_root_key() {
            if other.find_binary(&key).is_some() {
                result.store(key);
            } else {
                removed += 1;
            }
        }
        *self = result;
        removed
    }

    #[must_use]
    pub fn equivalent(&self, other: &Self) -> bool {
        self.iter().eq(other.iter())
    }

    pub fn is_subset_of(&self, other: &mut Self) -> bool {
        self.iter().all(|key| other.find_splayed(key).is_some())
    }

    pub fn visit_in_order<F>(&self, visitor: F)
    where
        F: FnMut(&K),
    {
        self.iter().for_each(visitor);
    }

    pub fn iter(&self) -> impl Iterator<Item = &K> {
        PTreeIter::new(self)
    }

    #[must_use]
    pub fn debug_print_with_count(&self) -> (String, usize)
    where
        K: std::fmt::Display,
    {
        let ordered_keys = self.to_stack();
        let mut result = String::new();
        for (count, key) in ordered_keys.iter().enumerate() {
            if count.is_multiple_of(10) {
                result.push_str("\n%");
            }
            let write_result = write!(&mut result, " {key:>7}");
            debug_assert!(write_result.is_ok());
        }
        result.push('\n');
        (result, ordered_keys.len())
    }

    #[must_use]
    pub fn debug_print_string(&self) -> String
    where
        K: std::fmt::Display,
    {
        self.debug_print_with_count().0
    }

    fn alloc_node(&mut self, key: K) -> usize {
        let node = PTreeNode {
            key,
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

    fn node(&self, index: usize) -> &PTreeNode<K> {
        self.nodes[index]
            .as_ref()
            .expect("PTree link must refer to a live node")
    }

    fn node_mut(&mut self, index: usize) -> &mut PTreeNode<K> {
        self.nodes[index]
            .as_mut()
            .expect("PTree link must refer to a live node")
    }

    fn find_index(&self, key: &K) -> Option<usize> {
        let mut current = self.root;
        while let Some(index) = current {
            current = match key.cmp(&self.node(index).key) {
                Ordering::Less => self.node(index).left,
                Ordering::Greater => self.node(index).right,
                Ordering::Equal => return Some(index),
            };
        }
        None
    }

    fn splay(&mut self, root: usize, key: &K) -> usize {
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
                    tree = self.node(tree).left.expect("splay left link must exist");
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
                    tree = self.node(tree).right.expect("splay right link must exist");
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

    fn c_stack_order(&self) -> Vec<usize> {
        let mut result = Vec::with_capacity(self.len);
        let mut pending = Vec::new();
        if let Some(root) = self.root {
            pending.push(root);
        }
        while let Some(index) = pending.pop() {
            result.push(index);
            if let Some(left) = self.node(index).left {
                pending.push(left);
            }
            if let Some(right) = self.node(index).right {
                pending.push(right);
            }
        }
        result
    }
}

struct PTreeIter<'tree, K> {
    tree: &'tree PTree<K>,
    pending: Vec<usize>,
    current: Option<usize>,
}

impl<'tree, K> PTreeIter<'tree, K>
where
    K: Ord + Clone,
{
    fn new(tree: &'tree PTree<K>) -> Self {
        Self {
            tree,
            pending: Vec::new(),
            current: tree.root,
        }
    }
}

impl<'tree, K> Iterator for PTreeIter<'tree, K>
where
    K: Ord + Clone,
{
    type Item = &'tree K;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(current) = self.current {
            self.pending.push(current);
            self.current = self.tree.node(current).left;
        }
        let next = self.pending.pop()?;
        self.current = self.tree.node(next).right;
        Some(&self.tree.node(next).key)
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
    fn splayed_find_tracks_hits_and_nearest_miss_like_c() {
        let mut tree = tree(&[1, 2, 3]);
        assert_eq!(tree.root_key(), Some(&3));

        assert_eq!(tree.find_splayed(&1), Some(&1));
        assert_eq!(tree.root_key(), Some(&1));
        assert_eq!(tree.find_splayed(&99), None);
        assert_eq!(tree.root_key(), Some(&3));
    }

    #[test]
    fn extract_root_delete_and_stack_conversion_follow_splay_shape() {
        let (mut tree, inserted) = PTree::from_stack([3, 1, 3, 2]);
        assert_eq!(inserted, 3);
        assert_eq!(tree.insert_stack([2, 4, 4]), 1);
        assert_eq!(tree.to_stack(), vec![4, 3, 2, 1]);
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
        assert_eq!(base.iter().copied().collect::<Vec<_>>(), vec![1, 2, 3, 4]);
        assert!(!base.merge(tree(&[1, 2])));
    }

    #[test]
    fn insert_tree_preserves_source_and_intersections_match_sets() {
        let mut base = tree(&[1, 4]);
        let add = tree(&[2, 4]);
        base.insert_tree(&add);
        assert_eq!(base.iter().copied().collect::<Vec<_>>(), vec![1, 2, 4]);
        assert_eq!(add.iter().copied().collect::<Vec<_>>(), vec![2, 4]);

        let intersection = base.intersection(&tree(&[2, 3, 4]));
        assert_eq!(intersection.iter().copied().collect::<Vec<_>>(), vec![2, 4]);
        assert_eq!(intersection.root_key(), Some(&2));
        assert_eq!(base.shared_element(&tree(&[9, 4, 2])), Some(2));
    }

    #[test]
    fn copy_destructive_intersection_equivalence_and_subset_match_c_helpers() {
        let mut base = tree(&[1, 2, 3, 4]);
        let copied = base.copy_tree();
        assert_eq!(copied.iter().copied().collect::<Vec<_>>(), vec![1, 2, 3, 4]);
        let removed = base.destructive_intersection(&tree(&[2, 4, 6]));
        assert_eq!(removed, 2);
        assert_eq!(base.iter().copied().collect::<Vec<_>>(), vec![2, 4]);
        assert!(base.equivalent(&tree(&[4, 2])));
        let mut superset = tree(&[1, 2, 3, 4]);
        assert!(base.is_subset_of(&mut superset));
        assert!(!tree(&[1, 9]).is_subset_of(&mut base));
    }

    #[test]
    fn visit_in_order_and_debug_print_preserve_distinct_orders() {
        let tree = tree(&[3, 1, 2]);
        let mut visited = Vec::new();
        tree.visit_in_order(|key| visited.push(*key));
        assert_eq!(visited, vec![1, 2, 3]);
        assert_eq!(tree.to_stack(), vec![2, 3, 1]);
        assert_eq!(
            tree.debug_print_with_count(),
            ("\n%       2       3       1\n".to_owned(), 3)
        );
        assert_eq!(tree.debug_print_string(), "\n%       2       3       1\n");
    }
}
