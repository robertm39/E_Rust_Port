use std::cmp::Ordering;

#[derive(Clone, Debug, PartialEq)]
pub struct FloatTreeEntry<V1, V2> {
    pub val1: V1,
    pub val2: V2,
}

#[derive(Clone, Debug, PartialEq)]
struct FloatTreeNode<V1, V2> {
    key: f64,
    entry: FloatTreeEntry<V1, V2>,
    left: Option<usize>,
    right: Option<usize>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FloatTree<V1, V2> {
    nodes: Vec<Option<FloatTreeNode<V1, V2>>>,
    free: Vec<usize>,
    root: Option<usize>,
    len: usize,
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
    pub fn root_key(&self) -> Option<f64> {
        self.root.map(|root| self.node(root).key)
    }

    pub fn store(&mut self, key: f64, val1: V1, val2: V2) -> bool {
        self.insert_entry(key, FloatTreeEntry { val1, val2 })
    }

    pub fn insert_entry(&mut self, key: f64, entry: FloatTreeEntry<V1, V2>) -> bool {
        let Some(root) = self.root else {
            self.root = Some(self.alloc_node(key, entry));
            return true;
        };

        let root = self.splay(root, key);
        self.root = Some(root);
        match c_splay_cmp(key, self.node(root).key) {
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

    pub fn find(&mut self, key: f64) -> Option<&FloatTreeEntry<V1, V2>> {
        let root = self.splay(self.root?, key);
        self.root = Some(root);
        c_keys_equal(self.node(root).key, key).then(|| &self.node(root).entry)
    }

    pub fn find_splayed(&mut self, key: f64) -> Option<&FloatTreeEntry<V1, V2>> {
        self.find(key)
    }

    #[must_use]
    pub fn find_binary(&self, key: f64) -> Option<&FloatTreeEntry<V1, V2>> {
        self.find_index(key).map(|index| &self.node(index).entry)
    }

    pub fn find_mut(&mut self, key: f64) -> Option<&mut FloatTreeEntry<V1, V2>> {
        let root = self.splay(self.root?, key);
        self.root = Some(root);
        c_keys_equal(self.node(root).key, key).then(|| &mut self.node_mut(root).entry)
    }

    pub fn extract_entry(&mut self, key: f64) -> Option<(f64, FloatTreeEntry<V1, V2>)> {
        let root = self.splay(self.root?, key);
        self.root = Some(root);
        if !c_keys_equal(self.node(root).key, key) {
            return None;
        }
        let removed = self.remove_root(root);
        Some((removed.key, removed.entry))
    }

    pub fn delete_entry(&mut self, key: f64) -> bool {
        self.extract_entry(key).is_some()
    }

    pub fn iter(&self) -> impl Iterator<Item = (f64, &FloatTreeEntry<V1, V2>)> {
        FloatTreeIter::new(self).map(|node| (node.key, &node.entry))
    }

    fn alloc_node(&mut self, key: f64, entry: FloatTreeEntry<V1, V2>) -> usize {
        let node = FloatTreeNode {
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

    fn node(&self, index: usize) -> &FloatTreeNode<V1, V2> {
        self.nodes[index]
            .as_ref()
            .expect("FloatTree link must refer to a live node")
    }

    fn node_mut(&mut self, index: usize) -> &mut FloatTreeNode<V1, V2> {
        self.nodes[index]
            .as_mut()
            .expect("FloatTree link must refer to a live node")
    }

    fn find_index(&self, key: f64) -> Option<usize> {
        let mut current = self.root;
        while let Some(index) = current {
            current = match c_splay_cmp(key, self.node(index).key) {
                Ordering::Less => self.node(index).left,
                Ordering::Greater => self.node(index).right,
                Ordering::Equal => {
                    return c_keys_equal(self.node(index).key, key).then_some(index);
                }
            };
        }
        None
    }

    fn remove_root(&mut self, root: usize) -> FloatTreeNode<V1, V2> {
        debug_assert_eq!(self.root, Some(root));
        let removed = self.nodes[root]
            .take()
            .expect("FloatTree root must refer to a live node");
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

    fn splay(&mut self, root: usize, key: f64) -> usize {
        let mut tree = root;
        let mut lower_root = None;
        let mut lower_tail = None;
        let mut upper_root = None;
        let mut upper_tail = None;

        loop {
            match c_splay_cmp(key, self.node(tree).key) {
                Ordering::Less => {
                    let Some(left) = self.node(tree).left else {
                        break;
                    };
                    if c_splay_cmp(key, self.node(left).key).is_lt() {
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
                    if c_splay_cmp(key, self.node(right).key).is_gt() {
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

fn c_splay_cmp(left: f64, right: f64) -> Ordering {
    left.partial_cmp(&right).unwrap_or(Ordering::Equal)
}

fn c_keys_equal(left: f64, right: f64) -> bool {
    matches!(left.partial_cmp(&right), Some(Ordering::Equal))
}

struct FloatTreeIter<'tree, V1, V2> {
    tree: &'tree FloatTree<V1, V2>,
    pending: Vec<usize>,
    current: Option<usize>,
}

impl<'tree, V1, V2> FloatTreeIter<'tree, V1, V2> {
    fn new(tree: &'tree FloatTree<V1, V2>) -> Self {
        Self {
            tree,
            pending: Vec::new(),
            current: tree.root,
        }
    }
}

impl<'tree, V1, V2> Iterator for FloatTreeIter<'tree, V1, V2> {
    type Item = &'tree FloatTreeNode<V1, V2>;

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
    use super::{FloatTree, FloatTreeEntry};
    use std::fmt::Write as _;

    fn shape<V1, V2>(tree: &FloatTree<V1, V2>) -> String {
        fn write_node<V1, V2>(
            tree: &FloatTree<V1, V2>,
            current: Option<usize>,
            output: &mut String,
        ) {
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
        assert_eq!(tree.root_key(), Some(10.5));
    }

    #[test]
    fn find_mut_rewrites_values_and_splays() {
        let mut tree = FloatTree::new();
        tree.store(1.0, 10, 100);
        tree.store(2.0, 20, 200);
        tree.find_mut(1.0).unwrap().val2 = 101;
        assert_eq!(tree.root_key(), Some(1.0));
        assert_eq!(tree.find_binary(1.0).unwrap().val2, 101);
    }

    #[test]
    fn operation_trace_matches_unchanged_c_splay_topology() {
        let mut tree = FloatTree::new();

        assert_eq!(shape(&tree), ".");
        assert!(tree.store(4.0, 40, 400));
        assert_eq!(shape(&tree), "[4](.,.)");
        assert!(tree.store(2.0, 20, 200));
        assert_eq!(shape(&tree), "[2](.,[4](.,.))");
        assert!(tree.store(6.0, 60, 600));
        assert_eq!(shape(&tree), "[6]([4]([2](.,.),.),.)");
        assert!(tree.store(3.0, 30, 300));
        assert_eq!(shape(&tree), "[3]([2](.,.),[4](.,[6](.,.)))");
        assert!(!tree.store(4.0, 44, 444));
        assert_eq!(shape(&tree), "[4]([3]([2](.,.),.),[6](.,.))");
        assert_eq!(tree.find_binary(4.0).unwrap().val1, 40);

        assert!(tree.find(2.0).is_some());
        assert_eq!(shape(&tree), "[2](.,[3](.,[4](.,[6](.,.))))");
        assert!(tree.find_binary(6.0).is_some());
        assert_eq!(shape(&tree), "[2](.,[3](.,[4](.,[6](.,.))))");
        assert!(tree.find(1.0).is_none());
        assert_eq!(shape(&tree), "[2](.,[3](.,[4](.,[6](.,.))))");
        assert!(tree.find(9.0).is_none());
        assert_eq!(shape(&tree), "[6]([3]([2](.,.),[4](.,.)),.)");
        assert!(tree.find(4.0).is_some());
        assert_eq!(shape(&tree), "[4]([3]([2](.,.),.),[6](.,.))");

        assert_eq!(tree.extract_entry(5.0), None);
        assert_eq!(shape(&tree), "[6]([4]([3]([2](.,.),.),.),.)");
        assert_eq!(
            tree.extract_entry(3.0),
            Some((
                3.0,
                FloatTreeEntry {
                    val1: 30,
                    val2: 300
                }
            ))
        );
        assert_eq!(shape(&tree), "[2](.,[4](.,[6](.,.)))");
    }

    #[test]
    fn extract_delete_and_slot_reuse_preserve_owned_entries() {
        let mut tree = FloatTree::new();
        tree.store(1.0, "one", 1);
        tree.store(2.5, "two", 2);
        tree.store(3.0, "three", 3);
        let allocated = tree.nodes.len();

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
        assert!(tree.find_binary(2.5).is_none());
        assert!(tree.delete_entry(1.0));
        assert!(!tree.delete_entry(9.0));
        assert!(tree.store(4.0, "four", 4));
        assert_eq!(tree.nodes.len(), allocated);
    }

    #[test]
    fn traversal_is_sorted_and_signed_zero_is_a_duplicate() {
        let mut tree = FloatTree::new();
        tree.store(3.0, 30, 0);
        tree.store(-0.0, 0, 0);
        assert!(!tree.store(0.0, 999, 999));
        assert_eq!(tree.root_key().unwrap().to_bits(), (-0.0_f64).to_bits());
        tree.store(f64::INFINITY, 100, 0);
        tree.store(f64::NEG_INFINITY, -100, 0);
        tree.store(-2.0, -20, 0);

        let visited = tree
            .iter()
            .map(|(key, entry)| (key, entry.val1))
            .collect::<Vec<_>>();
        assert_eq!(
            visited,
            vec![
                (f64::NEG_INFINITY, -100),
                (-2.0, -20),
                (-0.0, 0),
                (3.0, 30),
                (f64::INFINITY, 100)
            ]
        );
    }

    #[test]
    fn signed_zero_accesses_preserve_the_stored_representation() {
        let mut tree = FloatTree::new();
        assert!(tree.store(-0.0, 10, 0));
        assert!(!tree.store(0.0, 99, 99));
        assert_eq!(tree.root_key().unwrap().to_bits(), (-0.0_f64).to_bits());

        assert_eq!(tree.find(0.0).unwrap().val1, 10);
        assert_eq!(tree.root_key().unwrap().to_bits(), (-0.0_f64).to_bits());

        tree.find_mut(0.0).unwrap().val1 = 20;
        assert_eq!(tree.root_key().unwrap().to_bits(), (-0.0_f64).to_bits());
        assert_eq!(tree.find_binary(-0.0).unwrap().val1, 20);

        let (key, entry) = tree.extract_entry(0.0).unwrap();
        assert_eq!(key.to_bits(), (-0.0_f64).to_bits());
        assert_eq!(entry.val1, 20);
    }

    #[test]
    fn nan_query_on_numeric_tree_stops_at_root_and_never_matches() {
        let mut tree = FloatTree::new();
        assert!(tree.store(1.0, 10, 100));
        assert!(tree.store(2.0, 20, 200));
        let expected_shape = shape(&tree);

        assert!(!tree.store(f64::NAN, 99, 999));
        assert_eq!(tree.nodes(), 2);
        assert_eq!(shape(&tree), expected_shape);
        assert_eq!(tree.find(f64::NAN), None);
        assert_eq!(tree.find_binary(f64::NAN), None);
        assert_eq!(tree.extract_entry(f64::NAN), None);
        assert_eq!(shape(&tree), expected_shape);
        assert_eq!(tree.find_binary(2.0).unwrap().val1, 20);
    }

    #[test]
    fn nan_inserted_into_empty_tree_is_unfindable_and_blocks_later_inserts() {
        let mut tree = FloatTree::new();
        let nan = f64::from_bits(0x7ff8_0000_0000_0001);
        assert!(tree.store(nan, 20, 200));
        assert_eq!(tree.nodes(), 1);
        assert_eq!(tree.root_key().unwrap().to_bits(), nan.to_bits());

        assert_eq!(tree.find(nan), None);
        assert_eq!(tree.find(1.0), None);
        assert!(!tree.store(1.0, 10, 100));
        assert!(!tree.store(f64::NAN, 99, 999));
        assert_eq!(tree.extract_entry(nan), None);
        assert!(!tree.delete_entry(nan));
        assert_eq!(tree.nodes(), 1);

        let entries = tree.iter().collect::<Vec<_>>();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].0.to_bits(), nan.to_bits());
        assert_eq!(entries[0].1.val1, 20);
    }
}
