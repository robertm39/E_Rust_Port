use std::cmp::Ordering;

#[derive(Clone, Debug, Eq, PartialEq)]
struct ObjTreeNode<T> {
    object: T,
    left: Option<usize>,
    right: Option<usize>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObjTree<T> {
    nodes: Vec<Option<ObjTreeNode<T>>>,
    free: Vec<usize>,
    root: Option<usize>,
    len: usize,
}

impl<T> Default for ObjTree<T>
where
    T: Ord,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<T> ObjTree<T>
where
    T: Ord,
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
    pub fn root_object(&self) -> Option<&T> {
        self.root.map(|root| &self.node(root).object)
    }

    pub fn store(&mut self, object: T) -> Option<&T> {
        let Some(root) = self.root else {
            self.root = Some(self.alloc_node(object));
            return None;
        };

        let root = self.splay(root, &object);
        self.root = Some(root);
        match object.cmp(&self.node(root).object) {
            Ordering::Less => {
                let left = self.node(root).left;
                let new_root = self.alloc_node(object);
                self.node_mut(new_root).left = left;
                self.node_mut(new_root).right = Some(root);
                self.node_mut(root).left = None;
                self.root = Some(new_root);
                None
            }
            Ordering::Greater => {
                let right = self.node(root).right;
                let new_root = self.alloc_node(object);
                self.node_mut(new_root).right = right;
                self.node_mut(new_root).left = Some(root);
                self.node_mut(root).right = None;
                self.root = Some(new_root);
                None
            }
            Ordering::Equal => Some(&self.node(root).object),
        }
    }

    pub fn find(&mut self, key: &T) -> Option<&T> {
        let root = self.splay(self.root?, key);
        self.root = Some(root);
        (self.node(root).object.cmp(key) == Ordering::Equal).then(|| &self.node(root).object)
    }

    pub fn find_splayed(&mut self, key: &T) -> Option<&T> {
        self.find(key)
    }

    #[must_use]
    pub fn find_binary(&self, key: &T) -> Option<&T> {
        self.find_index(key).map(|index| &self.node(index).object)
    }

    pub fn extract_object(&mut self, key: &T) -> Option<T> {
        let root = self.splay(self.root?, key);
        self.root = Some(root);
        if self.node(root).object.cmp(key) != Ordering::Equal {
            return None;
        }
        Some(self.remove_root(root))
    }

    pub fn extract_root_object(&mut self) -> Option<T> {
        self.root.map(|root| self.remove_root(root))
    }

    /// # Panics
    ///
    /// Panics if `add` contains an object already present in this tree. This
    /// mirrors the C `PTreeObjMerge` assertion that input trees are disjoint.
    pub fn merge_unique(&mut self, add: Self) {
        for object in add.into_c_stack_order() {
            assert!(
                self.store(object).is_none(),
                "ObjTree merge expects disjoint trees"
            );
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        ObjTreeIter::new(self)
    }

    #[must_use]
    pub fn to_vec(&self) -> Vec<T>
    where
        T: Clone,
    {
        self.iter().cloned().collect()
    }

    pub fn free_with<F>(self, mut del_fun: F)
    where
        F: FnMut(T),
    {
        for object in self.into_post_order() {
            del_fun(object);
        }
    }

    pub fn dummy_del_fun(_object: T) {}

    fn alloc_node(&mut self, object: T) -> usize {
        let node = ObjTreeNode {
            object,
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

    fn node(&self, index: usize) -> &ObjTreeNode<T> {
        self.nodes[index]
            .as_ref()
            .expect("ObjTree link must refer to a live node")
    }

    fn node_mut(&mut self, index: usize) -> &mut ObjTreeNode<T> {
        self.nodes[index]
            .as_mut()
            .expect("ObjTree link must refer to a live node")
    }

    fn find_index(&self, key: &T) -> Option<usize> {
        let mut current = self.root;
        while let Some(index) = current {
            current = match key.cmp(&self.node(index).object) {
                Ordering::Less => self.node(index).left,
                Ordering::Greater => self.node(index).right,
                Ordering::Equal => return Some(index),
            };
        }
        None
    }

    fn remove_root(&mut self, root: usize) -> T {
        debug_assert_eq!(self.root, Some(root));
        let removed = self.nodes[root]
            .take()
            .expect("ObjTree root must refer to a live node");
        let new_root = if let Some(left) = removed.left {
            let left = self.splay(left, &removed.object);
            self.node_mut(left).right = removed.right;
            Some(left)
        } else {
            removed.right
        };
        self.free.push(root);
        self.len -= 1;
        self.root = new_root;
        removed.object
    }

    fn splay(&mut self, root: usize, key: &T) -> usize {
        let mut tree = root;
        let mut lower_root = None;
        let mut lower_tail = None;
        let mut upper_root = None;
        let mut upper_tail = None;

        loop {
            match key.cmp(&self.node(tree).object) {
                Ordering::Less => {
                    let Some(left) = self.node(tree).left else {
                        break;
                    };
                    if key < &self.node(left).object {
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
                    if key > &self.node(right).object {
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

    fn post_order(&self) -> Vec<usize> {
        let mut result = Vec::with_capacity(self.len);
        let mut pending = Vec::new();
        if let Some(root) = self.root {
            pending.push((root, false));
        }
        while let Some((index, visited)) = pending.pop() {
            if visited {
                result.push(index);
            } else {
                pending.push((index, true));
                if let Some(right) = self.node(index).right {
                    pending.push((right, false));
                }
                if let Some(left) = self.node(index).left {
                    pending.push((left, false));
                }
            }
        }
        result
    }

    fn into_c_stack_order(self) -> Vec<T> {
        let order = self.c_stack_order();
        self.into_objects(order)
    }

    fn into_post_order(self) -> Vec<T> {
        let order = self.post_order();
        self.into_objects(order)
    }

    fn into_objects(self, order: Vec<usize>) -> Vec<T> {
        let mut nodes = self.nodes;
        order
            .into_iter()
            .map(|index| {
                nodes[index]
                    .take()
                    .expect("ObjTree traversal must refer to a live node")
                    .object
            })
            .collect()
    }
}

struct ObjTreeIter<'tree, T> {
    tree: &'tree ObjTree<T>,
    pending: Vec<usize>,
    current: Option<usize>,
}

impl<'tree, T> ObjTreeIter<'tree, T>
where
    T: Ord,
{
    fn new(tree: &'tree ObjTree<T>) -> Self {
        Self {
            tree,
            pending: Vec::new(),
            current: tree.root,
        }
    }
}

impl<'tree, T> Iterator for ObjTreeIter<'tree, T>
where
    T: Ord,
{
    type Item = &'tree T;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(current) = self.current {
            self.pending.push(current);
            self.current = self.tree.node(current).left;
        }
        let next = self.pending.pop()?;
        self.current = self.tree.node(next).right;
        Some(&self.tree.node(next).object)
    }
}

#[cfg(test)]
mod tests {
    use super::ObjTree;
    use std::fmt::Write as _;

    fn tree(values: &[i32]) -> ObjTree<i32> {
        let mut tree = ObjTree::new();
        for value in values {
            tree.store(*value);
        }
        tree
    }

    fn shape(tree: &ObjTree<i32>) -> String {
        fn write_node(tree: &ObjTree<i32>, current: Option<usize>, output: &mut String) {
            let Some(current) = current else {
                output.push('.');
                return;
            };
            let node = tree.node(current);
            write!(output, "[{}](", node.object).unwrap();
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
    fn store_find_and_duplicates_return_existing_object() {
        let mut tree = ObjTree::new();

        assert_eq!(tree.store(10), None);
        assert_eq!(tree.store(3), None);
        assert_eq!(tree.store(10), Some(&10));
        assert_eq!(tree.nodes(), 2);
        assert_eq!(tree.root_object(), Some(&10));
        assert_eq!(tree.find(&3), Some(&3));
        assert_eq!(tree.find_binary(&99), None);
    }

    #[test]
    fn splayed_find_tracks_hits_and_nearest_misses_like_c() {
        let mut tree = tree(&[3, 1, 2]);

        assert_eq!(tree.find_splayed(&1), Some(&1));
        assert_eq!(tree.root_object(), Some(&1));
        assert_eq!(tree.find_splayed(&99), None);
        assert_eq!(tree.root_object(), Some(&3));
    }

    #[test]
    fn binary_find_does_not_reorganize_the_tree() {
        let tree = tree(&[4, 2, 6, 3]);
        let root = tree.root_object().copied();

        assert_eq!(tree.find_binary(&2), Some(&2));
        assert_eq!(tree.find_binary(&5), None);
        assert_eq!(tree.root_object().copied(), root);
    }

    #[test]
    fn operation_trace_matches_unchanged_c_splay_topology() {
        let mut tree = ObjTree::new();

        assert_eq!(shape(&tree), ".");
        assert_eq!(tree.store(4), None);
        assert_eq!(shape(&tree), "[4](.,.)");
        assert_eq!(tree.store(2), None);
        assert_eq!(shape(&tree), "[2](.,[4](.,.))");
        assert_eq!(tree.store(6), None);
        assert_eq!(shape(&tree), "[6]([4]([2](.,.),.),.)");
        assert_eq!(tree.store(3), None);
        assert_eq!(shape(&tree), "[3]([2](.,.),[4](.,[6](.,.)))");
        assert_eq!(tree.store(4), Some(&4));
        assert_eq!(shape(&tree), "[4]([3]([2](.,.),.),[6](.,.))");

        assert_eq!(tree.find(&2), Some(&2));
        assert_eq!(shape(&tree), "[2](.,[3](.,[4](.,[6](.,.))))");
        assert_eq!(tree.find_binary(&6), Some(&6));
        assert_eq!(shape(&tree), "[2](.,[3](.,[4](.,[6](.,.))))");
        assert_eq!(tree.find(&1), None);
        assert_eq!(shape(&tree), "[2](.,[3](.,[4](.,[6](.,.))))");
        assert_eq!(tree.find(&9), None);
        assert_eq!(shape(&tree), "[6]([3]([2](.,.),[4](.,.)),.)");
        assert_eq!(tree.find(&4), Some(&4));
        assert_eq!(shape(&tree), "[4]([3]([2](.,.),.),[6](.,.))");

        assert_eq!(tree.extract_object(&5), None);
        assert_eq!(shape(&tree), "[6]([4]([3]([2](.,.),.),.),.)");
        assert_eq!(tree.extract_object(&3), Some(3));
        assert_eq!(shape(&tree), "[2](.,[4](.,[6](.,.)))");
        assert_eq!(tree.extract_root_object(), Some(2));
        assert_eq!(shape(&tree), "[4](.,[6](.,.))");
    }

    #[test]
    fn merge_unique_consumes_disjoint_source_in_c_stack_order() {
        let mut base = tree(&[1, 3]);
        base.merge_unique(tree(&[2, 4]));
        assert_eq!(base.to_vec(), vec![1, 2, 3, 4]);
        assert_eq!(shape(&base), "[2]([1](.,.),[3](.,[4](.,.)))");
    }

    #[test]
    #[should_panic(expected = "ObjTree merge expects disjoint trees")]
    fn merge_unique_asserts_on_duplicate_like_c() {
        tree(&[1, 3]).merge_unique(tree(&[3, 4]));
    }

    #[test]
    fn free_with_visits_objects_in_c_post_order() {
        let tree = tree(&[3, 1, 2]);
        let mut deleted = Vec::new();
        tree.free_with(|object| deleted.push(object));
        assert_eq!(deleted, vec![1, 3, 2]);
    }

    #[derive(Debug, Eq, Ord, PartialEq, PartialOrd)]
    struct NotClone(i32);

    #[test]
    fn root_tracking_and_extraction_do_not_require_payload_clones() {
        let mut tree = ObjTree::new();

        assert!(tree.store(NotClone(2)).is_none());
        assert!(tree.store(NotClone(1)).is_none());
        assert!(tree.store(NotClone(2)).is_some());
        assert_eq!(tree.find(&NotClone(1)), Some(&NotClone(1)));
        assert_eq!(tree.root_object(), Some(&NotClone(1)));
        assert_eq!(tree.extract_object(&NotClone(1)), Some(NotClone(1)));
    }
}
