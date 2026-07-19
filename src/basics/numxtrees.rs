use std::cmp::Ordering;

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
struct NumXTreeNode<V> {
    key: NumXTreeKey,
    entry: NumXTreeEntry<V>,
    left: Option<usize>,
    right: Option<usize>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NumXTree<V> {
    nodes: Vec<Option<NumXTreeNode<V>>>,
    free: Vec<usize>,
    root: Option<usize>,
    len: usize,
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
    pub fn root_key(&self) -> Option<NumXTreeKey> {
        self.root.map(|root| self.node(root).key)
    }

    pub fn insert_entry(&mut self, key: NumXTreeKey, entry: NumXTreeEntry<V>) -> bool {
        let Some(root) = self.root else {
            self.root = Some(self.alloc_node(key, entry));
            return true;
        };

        let root = self.splay(root, key);
        self.root = Some(root);
        match key.cmp(&self.node(root).key) {
            Ordering::Less => {
                let left = self.node(root).left;
                let new_root = self.alloc_node(key, entry);
                self.node_mut(new_root).left = left;
                self.node_mut(new_root).right = Some(root);
                self.node_mut(root).left = None;
                self.root = Some(new_root);
                true
            }
            Ordering::Greater => {
                let right = self.node(root).right;
                let new_root = self.alloc_node(key, entry);
                self.node_mut(new_root).right = right;
                self.node_mut(new_root).left = Some(root);
                self.node_mut(root).right = None;
                self.root = Some(new_root);
                true
            }
            Ordering::Equal => false,
        }
    }

    pub fn find(&mut self, key: NumXTreeKey) -> Option<&NumXTreeEntry<V>> {
        let root = self.splay(self.root?, key);
        self.root = Some(root);
        (self.node(root).key == key).then(|| &self.node(root).entry)
    }

    pub fn find_splayed(&mut self, key: NumXTreeKey) -> Option<&NumXTreeEntry<V>> {
        self.find(key)
    }

    #[must_use]
    pub fn find_binary(&self, key: NumXTreeKey) -> Option<&NumXTreeEntry<V>> {
        self.find_index(key).map(|index| &self.node(index).entry)
    }

    pub fn find_mut(&mut self, key: NumXTreeKey) -> Option<&mut NumXTreeEntry<V>> {
        let root = self.splay(self.root?, key);
        self.root = Some(root);
        (self.node(root).key == key).then(|| &mut self.node_mut(root).entry)
    }

    pub fn extract_entry(&mut self, key: NumXTreeKey) -> Option<(NumXTreeKey, NumXTreeEntry<V>)> {
        let root = self.splay(self.root?, key);
        self.root = Some(root);
        if self.node(root).key != key {
            return None;
        }
        let removed = self.remove_root(root);
        Some((removed.key, removed.entry))
    }

    pub fn extract_root(&mut self) -> Option<(NumXTreeKey, NumXTreeEntry<V>)> {
        let root = self.root?;
        let removed = self.remove_root(root);
        Some((removed.key, removed.entry))
    }

    pub fn delete_entry(&mut self, key: NumXTreeKey) -> bool {
        self.extract_entry(key).is_some()
    }

    #[must_use]
    pub fn max_node(&self) -> Option<(NumXTreeKey, &NumXTreeEntry<V>)> {
        let mut current = self.root?;
        while let Some(right) = self.node(current).right {
            current = right;
        }
        let node = self.node(current);
        Some((node.key, &node.entry))
    }

    #[must_use]
    pub fn max_key(&self) -> Option<NumXTreeKey> {
        self.max_node().map(|(key, _entry)| key)
    }

    pub fn iter(&self) -> impl Iterator<Item = (NumXTreeKey, &NumXTreeEntry<V>)> {
        NumXTreeIter::new(self).map(|node| (node.key, &node.entry))
    }

    pub fn limited_iter(
        &self,
        limit: NumXTreeKey,
    ) -> impl Iterator<Item = (NumXTreeKey, &NumXTreeEntry<V>)> {
        NumXTreeIter::new_limited(self, limit).map(|node| (node.key, &node.entry))
    }

    fn alloc_node(&mut self, key: NumXTreeKey, entry: NumXTreeEntry<V>) -> usize {
        let node = NumXTreeNode {
            key,
            entry,
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

    fn node(&self, index: usize) -> &NumXTreeNode<V> {
        self.nodes[index]
            .as_ref()
            .expect("NumXTree link must refer to a live node")
    }

    fn node_mut(&mut self, index: usize) -> &mut NumXTreeNode<V> {
        self.nodes[index]
            .as_mut()
            .expect("NumXTree link must refer to a live node")
    }

    fn find_index(&self, key: NumXTreeKey) -> Option<usize> {
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

    fn remove_root(&mut self, root: usize) -> NumXTreeNode<V> {
        debug_assert_eq!(self.root, Some(root));
        let removed = self.nodes[root]
            .take()
            .expect("NumXTree root must refer to a live node");
        let new_root = if let Some(left) = removed.left {
            let left = self.splay(left, removed.key);
            self.node_mut(left).right = removed.right;
            Some(left)
        } else {
            removed.right
        };
        self.free.push(root);
        self.len -= 1;
        self.root = new_root;
        removed
    }

    fn splay(&mut self, root: usize, key: NumXTreeKey) -> usize {
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
                    if key < self.node(left).key {
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
                    if key > self.node(right).key {
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

struct NumXTreeIter<'tree, V> {
    tree: &'tree NumXTree<V>,
    pending: Vec<usize>,
    current: Option<usize>,
}

impl<'tree, V> NumXTreeIter<'tree, V> {
    fn new(tree: &'tree NumXTree<V>) -> Self {
        Self {
            tree,
            pending: Vec::new(),
            current: tree.root,
        }
    }

    fn new_limited(tree: &'tree NumXTree<V>, limit: NumXTreeKey) -> Self {
        let mut pending = Vec::new();
        let mut current = tree.root;
        while let Some(index) = current {
            let node = tree.node(index);
            if node.key < limit {
                current = node.right;
            } else {
                pending.push(index);
                current = (node.key != limit).then_some(node.left).flatten();
            }
        }
        Self {
            tree,
            pending,
            current: None,
        }
    }
}

impl<'tree, V> Iterator for NumXTreeIter<'tree, V> {
    type Item = &'tree NumXTreeNode<V>;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(current) = self.current {
            self.pending.push(current);
            self.current = self.tree.node(current).left;
        }
        let next = self.pending.pop()?;
        self.current = self.tree.node(next).right;
        Some(self.tree.node(next))
    }
}

#[cfg(test)]
mod tests {
    use super::{NumXTree, NumXTreeEntry};
    use std::fmt::Write as _;

    fn shape<V>(tree: &NumXTree<V>) -> String {
        fn write_node<V>(tree: &NumXTree<V>, current: Option<usize>, output: &mut String) {
            let Some(current) = current else {
                output.push('.');
                return;
            };
            let node = tree.node(current);
            write!(output, "[{}](", node.key).unwrap();
            write_node(tree, node.left, output);
            output.push(',');
            write_node(tree, node.right, output);
            output.push(')');
        }

        let mut output = String::new();
        write_node(tree, tree.root, &mut output);
        output
    }

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
        assert_eq!(tree.root_key(), Some(10));
    }

    #[test]
    fn insert_entry_supports_all_four_values_without_rewriting_duplicates() {
        let mut tree = NumXTree::new();
        assert!(tree.insert_entry(4, NumXTreeEntry::new([1, 2, 3, 4])));
        assert!(!tree.insert_entry(4, NumXTreeEntry::new([9, 9, 9, 9])));
        assert_eq!(tree.find(4).unwrap().values(), &[1, 2, 3, 4]);
    }

    #[test]
    fn find_mut_rewrites_value_slots_and_splays() {
        let mut tree = NumXTree::new();
        tree.store(1, 10, 100);
        tree.store(2, 20, 200);
        *tree.find_mut(1).unwrap().value_mut(2).unwrap() = 101;
        assert_eq!(tree.root_key(), Some(1));
        assert_eq!(tree.find_binary(1).unwrap().value(2), Some(&101));
    }

    #[test]
    fn binary_find_and_max_node_do_not_reorganize_the_tree() {
        let mut tree = NumXTree::new();
        for key in [4, 2, 6, 3] {
            tree.store(key, key * 10, 0);
        }
        let root = tree.root_key();

        assert!(tree.find_binary(2).is_some());
        assert!(tree.find_binary(5).is_none());
        assert_eq!(tree.max_key(), Some(6));
        assert_eq!(tree.root_key(), root);
    }

    #[test]
    fn operation_trace_matches_unchanged_c_splay_topology() {
        let mut tree = NumXTree::new();

        assert_eq!(shape(&tree), ".");
        assert!(tree.store(4, 40, 400));
        assert_eq!(shape(&tree), "[4](.,.)");
        assert!(tree.store(2, 20, 200));
        assert_eq!(shape(&tree), "[2](.,[4](.,.))");
        assert!(tree.store(6, 60, 600));
        assert_eq!(shape(&tree), "[6]([4]([2](.,.),.),.)");
        assert!(tree.store(3, 30, 300));
        assert_eq!(shape(&tree), "[3]([2](.,.),[4](.,[6](.,.)))");
        assert!(!tree.store(4, 44, 444));
        assert_eq!(shape(&tree), "[4]([3]([2](.,.),.),[6](.,.))");

        assert!(tree.find(2).is_some());
        assert_eq!(shape(&tree), "[2](.,[3](.,[4](.,[6](.,.))))");
        assert!(tree.find_binary(6).is_some());
        assert_eq!(shape(&tree), "[2](.,[3](.,[4](.,[6](.,.))))");
        assert!(tree.find(1).is_none());
        assert_eq!(shape(&tree), "[2](.,[3](.,[4](.,[6](.,.))))");
        assert!(tree.find(9).is_none());
        assert_eq!(shape(&tree), "[6]([3]([2](.,.),[4](.,.)),.)");
        assert!(tree.find(4).is_some());
        assert_eq!(shape(&tree), "[4]([3]([2](.,.),.),[6](.,.))");

        assert_eq!(tree.extract_entry(5), None);
        assert_eq!(shape(&tree), "[6]([4]([3]([2](.,.),.),.),.)");
        assert_eq!(
            tree.extract_entry(3),
            Some((3, NumXTreeEntry::new([30, 300, 0, 0])))
        );
        assert_eq!(shape(&tree), "[2](.,[4](.,[6](.,.)))");
        assert_eq!(
            tree.extract_root(),
            Some((2, NumXTreeEntry::new([20, 200, 0, 0])))
        );
        assert_eq!(shape(&tree), "[4](.,[6](.,.))");
    }

    #[test]
    fn extract_delete_and_slot_reuse_preserve_owned_entries() {
        let mut tree = NumXTree::new();
        tree.store(1, 10, 100);
        tree.store(2, 20, 200);
        tree.store(3, 30, 300);
        let allocated = tree.nodes.len();

        assert_eq!(
            tree.extract_entry(2),
            Some((2, NumXTreeEntry::new([20, 200, 0, 0])))
        );
        assert!(tree.find_binary(2).is_none());
        assert!(tree.delete_entry(1));
        assert!(!tree.delete_entry(9));
        assert!(tree.store(4, 40, 400));
        assert_eq!(tree.nodes.len(), allocated);
        assert_eq!(tree.extract_root().map(|(key, _entry)| key), Some(4));
    }

    #[test]
    fn max_node_is_non_destructive() {
        let mut tree = NumXTree::new();
        tree.store(-1, -10, 0);
        tree.store(4, 40, 0);
        tree.store(2, 20, 0);
        let root = tree.root_key();

        assert_eq!(
            tree.max_node(),
            Some((4, &NumXTreeEntry::new([40, 0, 0, 0])))
        );
        assert_eq!(tree.max_key(), Some(4));
        assert_eq!(tree.nodes(), 3);
        assert_eq!(tree.root_key(), root);
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

        let above_all = tree
            .limited_iter(8)
            .map(|(key, _entry)| key)
            .collect::<Vec<_>>();
        assert!(above_all.is_empty());
    }
}
