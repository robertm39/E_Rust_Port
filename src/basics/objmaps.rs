use std::cmp::Ordering;

#[derive(Clone, Debug, Eq, PartialEq)]
struct ObjMapNode<K, V> {
    key: K,
    value: Option<V>,
    left: Option<usize>,
    right: Option<usize>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObjMap<K, V> {
    nodes: Vec<Option<ObjMapNode<K, V>>>,
    free: Vec<usize>,
    root: Option<usize>,
    len: usize,
}

impl<K, V> Default for ObjMap<K, V>
where
    K: Ord,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<K, V> ObjMap<K, V>
where
    K: Ord,
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

    pub fn store(&mut self, key: K, value: V) -> Option<V> {
        let (slot, created) = self.get_ref(key);
        let old_value = slot.replace(value);
        if created {
            debug_assert!(old_value.is_none());
            None
        } else {
            old_value
        }
    }

    pub fn get_ref(&mut self, key: K) -> (&mut Option<V>, bool) {
        let Some(root) = self.root else {
            let root = self.alloc_node(key, None);
            self.root = Some(root);
            return (&mut self.node_mut(root).value, true);
        };

        let root = self.splay(root, &key);
        self.root = Some(root);
        match key.cmp(&self.node(root).key) {
            Ordering::Less => {
                let left = self.node(root).left;
                let new_root = self.alloc_node(key, None);
                self.node_mut(new_root).left = left;
                self.node_mut(new_root).right = Some(root);
                self.node_mut(root).left = None;
                self.root = Some(new_root);
                (&mut self.node_mut(new_root).value, true)
            }
            Ordering::Greater => {
                let right = self.node(root).right;
                let new_root = self.alloc_node(key, None);
                self.node_mut(new_root).right = right;
                self.node_mut(new_root).left = Some(root);
                self.node_mut(root).right = None;
                self.root = Some(new_root);
                (&mut self.node_mut(new_root).value, true)
            }
            Ordering::Equal => (&mut self.node_mut(root).value, false),
        }
    }

    pub fn find(&mut self, key: &K) -> Option<&V> {
        let root = self.splay(self.root?, key);
        self.root = Some(root);
        if self.node(root).key.cmp(key) == Ordering::Equal {
            self.node(root).value.as_ref()
        } else {
            None
        }
    }

    pub fn find_splayed(&mut self, key: &K) -> Option<&V> {
        self.find(key)
    }

    pub fn extract(&mut self, key: &K) -> Option<V> {
        self.extract_slot(key).flatten()
    }

    pub fn extract_slot(&mut self, key: &K) -> Option<Option<V>> {
        let root = self.splay(self.root?, key);
        self.root = Some(root);
        if self.node(root).key.cmp(key) != Ordering::Equal {
            return None;
        }
        Some(self.remove_root(root).value)
    }

    pub fn iter_entries(&self) -> impl Iterator<Item = (&K, &Option<V>)> {
        ObjMapIter::new(self).map(|node| (&node.key, &node.value))
    }

    #[must_use]
    pub fn traverse_values(&self) -> Vec<(&K, Option<&V>)> {
        self.iter_entries()
            .map(|(key, value)| (key, value.as_ref()))
            .collect()
    }

    pub fn free_with<F>(self, mut del_fun: F)
    where
        F: FnMut(K, Option<V>),
    {
        for node in self.into_post_order() {
            del_fun(node.key, node.value);
        }
    }

    fn alloc_node(&mut self, key: K, value: Option<V>) -> usize {
        let node = ObjMapNode {
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

    fn node(&self, index: usize) -> &ObjMapNode<K, V> {
        self.nodes[index]
            .as_ref()
            .expect("ObjMap link must refer to a live node")
    }

    fn node_mut(&mut self, index: usize) -> &mut ObjMapNode<K, V> {
        self.nodes[index]
            .as_mut()
            .expect("ObjMap link must refer to a live node")
    }

    fn remove_root(&mut self, root: usize) -> ObjMapNode<K, V> {
        debug_assert_eq!(self.root, Some(root));
        let removed = self.nodes[root]
            .take()
            .expect("ObjMap root must refer to a live node");
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

    fn into_post_order(self) -> Vec<ObjMapNode<K, V>> {
        let order = self.post_order();
        let mut nodes = self.nodes;
        order
            .into_iter()
            .map(|index| {
                nodes[index]
                    .take()
                    .expect("ObjMap traversal must refer to a live node")
            })
            .collect()
    }
}

struct ObjMapIter<'map, K, V> {
    map: &'map ObjMap<K, V>,
    pending: Vec<usize>,
    current: Option<usize>,
}

impl<'map, K, V> ObjMapIter<'map, K, V>
where
    K: Ord,
{
    fn new(map: &'map ObjMap<K, V>) -> Self {
        Self {
            map,
            pending: Vec::new(),
            current: map.root,
        }
    }
}

impl<'map, K, V> Iterator for ObjMapIter<'map, K, V>
where
    K: Ord,
{
    type Item = &'map ObjMapNode<K, V>;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(current) = self.current {
            self.pending.push(current);
            self.current = self.map.node(current).left;
        }
        let next = self.pending.pop()?;
        self.current = self.map.node(next).right;
        Some(self.map.node(next))
    }
}

#[must_use]
pub const fn size_of_obj_map_node_estimate() -> usize {
    std::mem::size_of::<usize>() * 4
}

#[cfg(test)]
mod tests {
    use super::{size_of_obj_map_node_estimate, ObjMap};
    use std::{cmp::Ordering, fmt::Write as _};

    fn shape<V>(map: &ObjMap<i32, V>) -> String {
        fn write_node<V>(map: &ObjMap<i32, V>, current: Option<usize>, output: &mut String) {
            let Some(current) = current else {
                output.push('.');
                return;
            };
            let node = map.node(current);
            write!(output, "[{}](", node.key).unwrap();
            write_node(map, node.left, output);
            output.push(',');
            write_node(map, node.right, output);
            output.push(')');
        }

        let mut output = String::new();
        write_node(map, map.root, &mut output);
        output
    }

    #[test]
    fn store_returns_old_value_and_updates_mapping() {
        let mut map = ObjMap::new();

        assert_eq!(map.store(2, "two"), None);
        assert_eq!(map.store(1, "one"), None);
        assert_eq!(map.store(2, "TWO"), Some("two"));
        assert_eq!(map.nodes(), 2);
        assert_eq!(map.find(&2), Some(&"TWO"));
        assert_eq!(map.root_key(), Some(&2));
    }

    #[test]
    fn get_ref_creates_null_slot_and_reports_creation() {
        let mut map: ObjMap<&str, i32> = ObjMap::new();

        let (slot, created) = map.get_ref("x");
        assert!(created);
        assert_eq!(slot, &None);
        assert_eq!(map.find(&"x"), None);
        assert_eq!(map.nodes(), 1);

        let (slot, created) = map.get_ref("x");
        assert!(!created);
        *slot = Some(7);
        assert_eq!(map.find(&"x"), Some(&7));
    }

    #[test]
    fn splayed_find_tracks_null_slots_and_nearest_misses_like_c() {
        let mut map: ObjMap<i32, i32> = ObjMap::new();
        map.get_ref(1);
        map.store(3, 7);
        assert_eq!(map.root_key(), Some(&3));

        assert_eq!(map.find_splayed(&1), None);
        assert_eq!(map.root_key(), Some(&1));
        assert_eq!(map.find_splayed(&2), None);
        assert_eq!(map.root_key(), Some(&3));
    }

    #[test]
    fn store_over_null_slot_returns_none_like_c() {
        let mut map = ObjMap::new();
        map.get_ref("x");

        assert_eq!(map.store("x", 10), None);
        assert_eq!(map.find(&"x"), Some(&10));
    }

    #[test]
    fn operation_trace_matches_unchanged_c_splay_topology() {
        let mut map = ObjMap::new();

        assert_eq!(shape(&map), ".");
        assert_eq!(map.store(4, 40), None);
        assert_eq!(shape(&map), "[4](.,.)");
        assert_eq!(map.store(2, 20), None);
        assert_eq!(shape(&map), "[2](.,[4](.,.))");
        assert_eq!(map.store(6, 60), None);
        assert_eq!(shape(&map), "[6]([4]([2](.,.),.),.)");
        assert_eq!(map.store(3, 30), None);
        assert_eq!(shape(&map), "[3]([2](.,.),[4](.,[6](.,.)))");
        assert_eq!(map.store(4, 44), Some(40));
        assert_eq!(shape(&map), "[4]([3]([2](.,.),.),[6](.,.))");

        assert_eq!(map.find(&2), Some(&20));
        assert_eq!(shape(&map), "[2](.,[3](.,[4](.,[6](.,.))))");
        assert_eq!(map.find(&1), None);
        assert_eq!(shape(&map), "[2](.,[3](.,[4](.,[6](.,.))))");
        assert_eq!(map.find(&9), None);
        assert_eq!(shape(&map), "[6]([3]([2](.,.),[4](.,.)),.)");
        assert_eq!(map.find(&4), Some(&44));
        assert_eq!(shape(&map), "[4]([3]([2](.,.),.),[6](.,.))");

        assert_eq!(map.extract(&5), None);
        assert_eq!(shape(&map), "[6]([4]([3]([2](.,.),.),.),.)");
        assert_eq!(map.extract(&3), Some(30));
        assert_eq!(shape(&map), "[2](.,[4](.,[6](.,.)))");
        assert_eq!(map.extract_slot(&2), Some(Some(20)));
        assert_eq!(shape(&map), "[4](.,[6](.,.))");
    }

    #[test]
    fn extract_removes_null_value_nodes_despite_null_return_shape() {
        let mut map = ObjMap::new();
        map.store(1, "one");
        map.get_ref(2);

        assert_eq!(map.extract(&1), Some("one"));
        assert_eq!(map.find(&1), None);
        assert_eq!(map.extract(&2), None);
        assert_eq!(map.nodes(), 0);
        assert_eq!(map.extract_slot(&99), None);
    }

    #[test]
    fn traversal_is_sorted_and_preserves_null_slots() {
        let mut map = ObjMap::new();
        map.store(3, "three");
        map.get_ref(1);
        map.store(2, "two");

        let traversed = map
            .traverse_values()
            .into_iter()
            .map(|(key, value)| (*key, value.copied()))
            .collect::<Vec<_>>();
        assert_eq!(
            traversed,
            vec![(1, None), (2, Some("two")), (3, Some("three"))]
        );
    }

    #[test]
    fn free_with_visits_key_value_pairs_in_c_post_order() {
        let mut map = ObjMap::new();
        map.store(3, "three");
        map.store(1, "one");
        map.store(2, "two");
        let mut freed = Vec::new();

        map.free_with(|key, value| freed.push((key, value)));
        assert_eq!(
            freed,
            vec![(1, Some("one")), (3, Some("three")), (2, Some("two"))]
        );
    }

    #[derive(Debug)]
    struct EquivalentKey {
        order: i32,
        owner: &'static str,
    }

    impl PartialEq for EquivalentKey {
        fn eq(&self, other: &Self) -> bool {
            self.order == other.order
        }
    }

    impl Eq for EquivalentKey {}

    impl PartialOrd for EquivalentKey {
        fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
            Some(self.cmp(other))
        }
    }

    impl Ord for EquivalentKey {
        fn cmp(&self, other: &Self) -> Ordering {
            self.order.cmp(&other.order)
        }
    }

    #[test]
    fn equivalent_store_retains_the_original_owned_key_without_cloning() {
        let mut map = ObjMap::new();
        assert_eq!(
            map.store(
                EquivalentKey {
                    order: 1,
                    owner: "first",
                },
                "old",
            ),
            None
        );
        assert_eq!(
            map.store(
                EquivalentKey {
                    order: 1,
                    owner: "second",
                },
                "new",
            ),
            Some("old")
        );
        assert_eq!(map.root_key().map(|key| key.owner), Some("first"));
    }

    #[test]
    fn node_size_estimate_matches_the_four_pointer_c_node_shape() {
        assert_eq!(
            size_of_obj_map_node_estimate(),
            std::mem::size_of::<usize>() * 4
        );
    }
}
