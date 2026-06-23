pub trait BinaryTreeAccess {
    type Handle: Copy;

    fn left_child(&self, handle: Self::Handle) -> Option<Self::Handle>;

    fn right_child(&self, handle: Self::Handle) -> Option<Self::Handle>;
}

#[derive(Clone, Debug)]
pub struct AvlTraverseState<'a, T>
where
    T: BinaryTreeAccess,
{
    tree: &'a T,
    stack: Vec<T::Handle>,
}

impl<'a, T> AvlTraverseState<'a, T>
where
    T: BinaryTreeAccess,
{
    #[must_use]
    pub fn new(tree: &'a T, root: Option<T::Handle>) -> Self {
        let mut state = Self {
            tree,
            stack: Vec::new(),
        };
        state.push_left_spine(root);
        state
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.stack.is_empty()
    }

    #[must_use]
    pub fn pending_len(&self) -> usize {
        self.stack.len()
    }

    fn push_left_spine(&mut self, mut handle: Option<T::Handle>) {
        while let Some(current) = handle {
            self.stack.push(current);
            handle = self.tree.left_child(current);
        }
    }

    pub fn traverse_next(&mut self) -> Option<T::Handle> {
        let result = self.stack.pop()?;
        self.push_left_spine(self.tree.right_child(result));
        Some(result)
    }
}

impl<T> Iterator for AvlTraverseState<'_, T>
where
    T: BinaryTreeAccess,
{
    type Item = T::Handle;

    fn next(&mut self) -> Option<Self::Item> {
        self.traverse_next()
    }
}

#[must_use]
pub fn avl_traverse_init<T>(tree: &T, root: Option<T::Handle>) -> AvlTraverseState<'_, T>
where
    T: BinaryTreeAccess,
{
    AvlTraverseState::new(tree, root)
}

pub fn avl_traverse_next<T>(state: &mut AvlTraverseState<'_, T>) -> Option<T::Handle>
where
    T: BinaryTreeAccess,
{
    state.traverse_next()
}

#[cfg(test)]
mod tests {
    use super::{avl_traverse_init, avl_traverse_next, BinaryTreeAccess};

    #[derive(Clone, Debug)]
    struct Node {
        key: i32,
        left: Option<usize>,
        right: Option<usize>,
    }

    #[derive(Clone, Debug)]
    struct Tree {
        nodes: Vec<Node>,
    }

    impl BinaryTreeAccess for Tree {
        type Handle = usize;

        fn left_child(&self, handle: Self::Handle) -> Option<Self::Handle> {
            self.nodes[handle].left
        }

        fn right_child(&self, handle: Self::Handle) -> Option<Self::Handle> {
            self.nodes[handle].right
        }
    }

    fn sample_tree() -> Tree {
        Tree {
            nodes: vec![
                Node {
                    key: 1,
                    left: None,
                    right: None,
                },
                Node {
                    key: 2,
                    left: Some(0),
                    right: Some(2),
                },
                Node {
                    key: 3,
                    left: None,
                    right: None,
                },
                Node {
                    key: 4,
                    left: Some(1),
                    right: Some(5),
                },
                Node {
                    key: 5,
                    left: None,
                    right: None,
                },
                Node {
                    key: 6,
                    left: Some(4),
                    right: None,
                },
            ],
        }
    }

    #[test]
    fn traversal_init_pushes_left_spine_like_c_macro() {
        let tree = sample_tree();
        let state = avl_traverse_init(&tree, Some(3));
        assert_eq!(state.pending_len(), 3);
        assert!(!state.is_empty());
    }

    #[test]
    fn traversal_next_returns_nodes_in_left_root_right_order() {
        let tree = sample_tree();
        let ordered_keys = avl_traverse_init(&tree, Some(3))
            .map(|handle| tree.nodes[handle].key)
            .collect::<Vec<_>>();
        assert_eq!(ordered_keys, vec![1, 2, 3, 4, 5, 6]);
    }

    #[test]
    fn function_shaped_next_matches_generated_c_api() {
        let tree = sample_tree();
        let mut state = avl_traverse_init(&tree, Some(3));

        assert_eq!(avl_traverse_next(&mut state), Some(0));
        assert_eq!(avl_traverse_next(&mut state), Some(1));
        assert_eq!(avl_traverse_next(&mut state), Some(2));
    }

    #[test]
    fn empty_root_returns_empty_state_and_no_next_node() {
        let tree = sample_tree();
        let mut state = avl_traverse_init(&tree, None);
        assert!(state.is_empty());
        assert_eq!(avl_traverse_next(&mut state), None);
    }
}
