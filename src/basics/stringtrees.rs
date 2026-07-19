use std::cmp::Ordering;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StrTreeEntry<V1, V2> {
    pub val1: V1,
    pub val2: V2,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct StrTreeNode<V1, V2> {
    key: String,
    entry: StrTreeEntry<V1, V2>,
    left: Option<usize>,
    right: Option<usize>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StrTree<V1, V2> {
    nodes: Vec<Option<StrTreeNode<V1, V2>>>,
    free: Vec<usize>,
    root: Option<usize>,
    len: usize,
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
            nodes: Vec::new(),
            free: Vec::new(),
            root: None,
            len: 0,
        }
    }

    #[must_use]
    pub const fn len(&self) -> usize {
        self.len
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    #[must_use]
    pub fn root_key(&self) -> Option<&str> {
        self.root.map(|root| self.node(root).key.as_str())
    }

    pub fn store(&mut self, key: &str, val1: V1, val2: V2) -> bool {
        self.insert_entry(c_string_prefix(key).to_owned(), StrTreeEntry { val1, val2 })
    }

    pub fn insert_entry(&mut self, mut key: String, entry: StrTreeEntry<V1, V2>) -> bool {
        if let Some(nul) = key.as_bytes().iter().position(|byte| *byte == 0) {
            key.truncate(nul);
        }
        let Some(root) = self.root else {
            self.root = Some(self.alloc_node(key, entry));
            return true;
        };

        let root = self.splay(root, &key);
        self.root = Some(root);
        match c_string_cmp(&key, &self.node(root).key) {
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

    pub fn find(&mut self, key: &str) -> Option<&StrTreeEntry<V1, V2>> {
        let root = self.splay(self.root?, key);
        self.root = Some(root);
        c_string_cmp(&self.node(root).key, key)
            .is_eq()
            .then(|| &self.node(root).entry)
    }

    pub fn find_splayed(&mut self, key: &str) -> Option<&StrTreeEntry<V1, V2>> {
        self.find(key)
    }

    #[must_use]
    pub fn find_binary(&self, key: &str) -> Option<&StrTreeEntry<V1, V2>> {
        self.find_index(key).map(|index| &self.node(index).entry)
    }

    pub fn find_mut(&mut self, key: &str) -> Option<&mut StrTreeEntry<V1, V2>> {
        let root = self.splay(self.root?, key);
        self.root = Some(root);
        c_string_cmp(&self.node(root).key, key)
            .is_eq()
            .then(|| &mut self.node_mut(root).entry)
    }

    pub fn extract_entry(&mut self, key: &str) -> Option<(String, StrTreeEntry<V1, V2>)> {
        let root = self.splay(self.root?, key);
        self.root = Some(root);
        if !c_string_cmp(key, &self.node(root).key).is_eq() {
            return None;
        }
        let removed = self.remove_root(root);
        Some((removed.key, removed.entry))
    }

    pub fn delete_entry(&mut self, key: &str) -> bool {
        self.extract_entry(key).is_some()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &StrTreeEntry<V1, V2>)> {
        StrTreeIter::new(self).map(|node| (node.key.as_str(), &node.entry))
    }

    fn alloc_node(&mut self, key: String, entry: StrTreeEntry<V1, V2>) -> usize {
        let node = StrTreeNode {
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

    fn node(&self, index: usize) -> &StrTreeNode<V1, V2> {
        self.nodes[index]
            .as_ref()
            .expect("StrTree link must refer to a live node")
    }

    fn node_mut(&mut self, index: usize) -> &mut StrTreeNode<V1, V2> {
        self.nodes[index]
            .as_mut()
            .expect("StrTree link must refer to a live node")
    }

    fn find_index(&self, key: &str) -> Option<usize> {
        let mut current = self.root;
        while let Some(index) = current {
            current = match c_string_cmp(key, &self.node(index).key) {
                Ordering::Less => self.node(index).left,
                Ordering::Greater => self.node(index).right,
                Ordering::Equal => return Some(index),
            };
        }
        None
    }

    fn remove_root(&mut self, root: usize) -> StrTreeNode<V1, V2> {
        debug_assert_eq!(self.root, Some(root));
        let removed = self.nodes[root]
            .take()
            .expect("StrTree root must refer to a live node");
        let new_root = if let Some(left) = removed.left {
            let left = self.splay(left, &removed.key);
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

    fn splay(&mut self, root: usize, key: &str) -> usize {
        let mut tree = root;
        let mut lower_root = None;
        let mut lower_tail = None;
        let mut upper_root = None;
        let mut upper_tail = None;

        loop {
            match c_string_cmp(key, &self.node(tree).key) {
                Ordering::Less => {
                    let Some(left) = self.node(tree).left else {
                        break;
                    };
                    if c_string_cmp(key, &self.node(left).key).is_lt() {
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
                    if c_string_cmp(key, &self.node(right).key).is_gt() {
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

fn c_string_prefix(key: &str) -> &str {
    key.as_bytes()
        .iter()
        .position(|byte| *byte == 0)
        .map_or(key, |nul| &key[..nul])
}

fn c_string_cmp(left: &str, right: &str) -> Ordering {
    c_string_prefix(left)
        .as_bytes()
        .cmp(c_string_prefix(right).as_bytes())
}

struct StrTreeIter<'tree, V1, V2> {
    tree: &'tree StrTree<V1, V2>,
    pending: Vec<usize>,
    current: Option<usize>,
}

impl<'tree, V1, V2> StrTreeIter<'tree, V1, V2> {
    fn new(tree: &'tree StrTree<V1, V2>) -> Self {
        Self {
            tree,
            pending: Vec::new(),
            current: tree.root,
        }
    }
}

impl<'tree, V1, V2> Iterator for StrTreeIter<'tree, V1, V2> {
    type Item = &'tree StrTreeNode<V1, V2>;

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
    use super::{StrTree, StrTreeEntry};
    use std::fmt::Write as _;

    fn shape<V1, V2>(tree: &StrTree<V1, V2>) -> String {
        fn write_node<V1, V2>(tree: &StrTree<V1, V2>, current: Option<usize>, output: &mut String) {
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
        assert_eq!(tree.root_key(), Some("gamma"));
    }

    #[test]
    fn find_mut_allows_signature_style_value_rewrite_and_splays() {
        let mut tree = StrTree::new();
        tree.store("alpha", 1, 0);
        tree.store("name", 2, 0);
        tree.find_mut("alpha").unwrap().val1 = 7;
        assert_eq!(tree.root_key(), Some("alpha"));
        assert_eq!(tree.find_binary("alpha").unwrap().val1, 7);
    }

    #[test]
    fn operation_trace_matches_unchanged_c_splay_topology() {
        let mut tree = StrTree::new();

        assert_eq!(shape(&tree), ".");
        assert!(tree.store("d", 40, 400));
        assert_eq!(shape(&tree), "[d](.,.)");
        assert!(tree.store("b", 20, 200));
        assert_eq!(shape(&tree), "[b](.,[d](.,.))");
        assert!(tree.store("f", 60, 600));
        assert_eq!(shape(&tree), "[f]([d]([b](.,.),.),.)");
        assert!(tree.store("c", 30, 300));
        assert_eq!(shape(&tree), "[c]([b](.,.),[d](.,[f](.,.)))");
        assert!(!tree.store("d", 44, 444));
        assert_eq!(shape(&tree), "[d]([c]([b](.,.),.),[f](.,.))");
        assert_eq!(tree.find_binary("d").unwrap().val1, 40);

        assert!(tree.find("b").is_some());
        assert_eq!(shape(&tree), "[b](.,[c](.,[d](.,[f](.,.))))");
        assert!(tree.find_binary("f").is_some());
        assert_eq!(shape(&tree), "[b](.,[c](.,[d](.,[f](.,.))))");
        assert!(tree.find("a").is_none());
        assert_eq!(shape(&tree), "[b](.,[c](.,[d](.,[f](.,.))))");
        assert!(tree.find("z").is_none());
        assert_eq!(shape(&tree), "[f]([c]([b](.,.),[d](.,.)),.)");
        assert!(tree.find("d").is_some());
        assert_eq!(shape(&tree), "[d]([c]([b](.,.),.),[f](.,.))");

        assert_eq!(tree.extract_entry("e"), None);
        assert_eq!(shape(&tree), "[f]([d]([c]([b](.,.),.),.),.)");
        assert_eq!(
            tree.extract_entry("c"),
            Some((
                "c".to_owned(),
                StrTreeEntry {
                    val1: 30,
                    val2: 300
                }
            ))
        );
        assert_eq!(shape(&tree), "[b](.,[d](.,[f](.,.)))");
    }

    #[test]
    fn traversal_uses_unsigned_c_string_byte_order() {
        let mut tree = StrTree::new();
        tree.store("gamma", 3, 0);
        tree.store("alpha", 1, 0);
        tree.store("beta", 2, 0);
        tree.store("éclair", 4, 0);
        tree.store("zeta", 5, 0);

        let visited = tree
            .iter()
            .map(|(key, entry)| (key.to_owned(), entry.val1))
            .collect::<Vec<_>>();
        assert_eq!(
            visited,
            vec![
                ("alpha".to_owned(), 1),
                ("beta".to_owned(), 2),
                ("gamma".to_owned(), 3),
                ("zeta".to_owned(), 5),
                ("éclair".to_owned(), 4)
            ]
        );
    }

    #[test]
    fn embedded_nul_matches_c_string_termination_and_owned_key_copy() {
        let mut source = String::from("alpha\0ignored");
        let mut tree = StrTree::new();
        assert!(tree.store(&source, 1, 10));
        source.replace_range(..5, "omega");

        assert_eq!(tree.root_key(), Some("alpha"));
        assert!(!tree.store("alpha", 99, 99));
        assert_eq!(tree.find("alpha\0query").unwrap().val1, 1);
        assert_eq!(
            tree.extract_entry("alpha\0tail"),
            Some(("alpha".to_owned(), StrTreeEntry { val1: 1, val2: 10 }))
        );
        assert!(tree.is_empty());
    }

    #[test]
    fn extract_delete_and_slot_reuse_preserve_owned_entries() {
        let mut tree = StrTree::new();
        tree.store("alpha", 1, 10);
        tree.store("beta", 2, 20);
        tree.store("gamma", 3, 30);
        let allocated = tree.nodes.len();

        assert_eq!(
            tree.extract_entry("beta"),
            Some(("beta".to_owned(), StrTreeEntry { val1: 2, val2: 20 }))
        );
        assert!(tree.find_binary("beta").is_none());
        assert!(tree.delete_entry("alpha"));
        assert!(!tree.delete_entry("missing"));
        assert!(tree.store("delta", 4, 40));
        assert_eq!(tree.nodes.len(), allocated);
        assert_eq!(tree.find_binary("gamma").unwrap().val1, 3);
    }
}
