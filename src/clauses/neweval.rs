use std::cmp::Ordering;
use std::num::NonZeroUsize;
use std::sync::atomic::{AtomicI64, Ordering as AtomicOrdering};

pub type EvalPriority = i64;
pub type EvalObjectHandle = usize;
pub type EvalNodeHandle = usize;

pub const PRIO_BEST: EvalPriority = 0;
pub const PRIO_PREFER: EvalPriority = 30;
pub const PRIO_NORMAL: EvalPriority = 40;
pub const PRIO_DEFER: EvalPriority = 50;
pub const PRIO_LARGEST_REASONABLE: EvalPriority = 1_048_576;

static EVALUATION_COUNTER: AtomicI64 = AtomicI64::new(0);

#[derive(Clone, Debug, PartialEq)]
pub struct SimpleEvalCell {
    priority: EvalPriority,
    heuristic: f32,
    left: Option<NonZeroUsize>,
    right: Option<NonZeroUsize>,
}

impl Default for SimpleEvalCell {
    fn default() -> Self {
        Self::new()
    }
}

impl SimpleEvalCell {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            priority: 0,
            heuristic: 0.0,
            left: None,
            right: None,
        }
    }

    #[must_use]
    pub const fn priority(&self) -> EvalPriority {
        self.priority
    }

    pub const fn set_priority(&mut self, priority: EvalPriority) {
        self.priority = priority;
    }

    #[must_use]
    pub const fn heuristic(&self) -> f32 {
        self.heuristic
    }

    pub const fn set_heuristic(&mut self, heuristic: f32) {
        self.heuristic = heuristic;
    }

    #[allow(clippy::cast_possible_truncation)]
    pub fn set_heuristic_from_eval(&mut self, heuristic: f64) {
        self.heuristic = heuristic as f32;
    }

    #[must_use]
    pub fn left(&self) -> Option<EvalNodeHandle> {
        self.left.map(unpack_handle)
    }

    pub fn set_left(&mut self, left: Option<EvalNodeHandle>) {
        self.left = left.map(pack_handle);
    }

    #[must_use]
    pub fn right(&self) -> Option<EvalNodeHandle> {
        self.right.map(unpack_handle)
    }

    pub fn set_right(&mut self, right: Option<EvalNodeHandle>) {
        self.right = right.map(pack_handle);
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct EvalCell {
    eval_count: EvalPriority,
    object: Option<NonZeroUsize>,
    evals: Box<[SimpleEvalCell]>,
}

impl EvalCell {
    #[must_use]
    pub fn alloc(eval_no: usize) -> Self {
        let mut eval = evals_alloc_raw(eval_no);
        eval.eval_count = EVALUATION_COUNTER.fetch_add(1, AtomicOrdering::Relaxed);
        eval
    }

    #[must_use]
    pub fn eval_no(&self) -> usize {
        self.evals.len()
    }

    #[must_use]
    pub const fn eval_count(&self) -> EvalPriority {
        self.eval_count
    }

    #[must_use]
    pub fn object(&self) -> Option<EvalObjectHandle> {
        self.object.map(unpack_handle)
    }

    pub fn set_object(&mut self, object: Option<EvalObjectHandle>) {
        self.object = object.map(pack_handle);
    }

    /// Returns the simple evaluation at `pos`.
    ///
    /// # Panics
    ///
    /// Panics if `pos` is outside this cell's evaluation array, matching the
    /// unchecked C flexible-array access contract.
    #[must_use]
    pub fn eval(&self, pos: usize) -> &SimpleEvalCell {
        &self.evals[pos]
    }

    /// Returns the mutable simple evaluation at `pos`.
    ///
    /// # Panics
    ///
    /// Panics if `pos` is outside this cell's evaluation array, matching the
    /// unchecked C flexible-array access contract.
    pub fn eval_mut(&mut self, pos: usize) -> &mut SimpleEvalCell {
        &mut self.evals[pos]
    }

    pub fn set_priority(&mut self, priority: EvalPriority) {
        for eval in &mut self.evals {
            eval.set_priority(priority);
        }
    }

    pub fn change_priority(&mut self, diff: EvalPriority) {
        for eval in &mut self.evals {
            eval.set_priority(eval.priority() + diff);
        }
    }

    #[must_use]
    pub fn list_print_string(&self) -> String {
        (0..self.eval_no())
            .map(|pos| self.print_string(pos))
            .collect()
    }

    #[must_use]
    pub fn list_print_comment_string(&self) -> String {
        format!("/*{}*/", self.list_print_string())
    }

    /// Returns the C `EvalPrint` string for evaluation position `pos`.
    ///
    /// # Panics
    ///
    /// Panics if `pos` is outside this cell's evaluation array, matching the
    /// unchecked C flexible-array access contract.
    #[must_use]
    pub fn print_string(&self, pos: usize) -> String {
        let eval = self.eval(pos);
        format!(
            "[{:3}:{:.10}:{}]",
            eval.priority(),
            eval.heuristic(),
            self.eval_count()
        )
    }

    /// Returns the C `EvalPrintComment` string for evaluation position `pos`.
    ///
    /// # Panics
    ///
    /// Panics if `pos` is outside this cell's evaluation array, matching the
    /// unchecked C flexible-array access contract.
    #[must_use]
    pub fn print_comment_string(&self, pos: usize) -> String {
        format!("/*{}*/", self.print_string(pos))
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct EvalTree {
    root: Option<EvalNodeHandle>,
    nodes: Vec<Option<EvalCell>>,
    free: Vec<EvalNodeHandle>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EvalTreeTraverseState {
    stack: Vec<EvalNodeHandle>,
}

impl EvalTree {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            root: None,
            nodes: Vec::new(),
            free: Vec::new(),
        }
    }

    #[must_use]
    pub const fn root(&self) -> Option<EvalNodeHandle> {
        self.root
    }

    pub fn alloc_node(&mut self, mut eval: EvalCell) -> EvalNodeHandle {
        for pos in 0..eval.eval_no() {
            eval.eval_mut(pos).set_left(None);
            eval.eval_mut(pos).set_right(None);
        }

        if let Some(handle) = self.free.pop() {
            self.nodes[handle] = Some(eval);
            handle
        } else {
            let handle = self.nodes.len();
            self.nodes.push(Some(eval));
            handle
        }
    }

    /// Removes a node from the arena without checking for tree dependencies.
    ///
    /// This matches the C `EvalsFree` contract: callers must only free detached
    /// entries or accept dangling tree references.
    pub fn take_node(&mut self, handle: EvalNodeHandle) -> Option<EvalCell> {
        let node = self.nodes.get_mut(handle)?.take()?;
        self.free.push(handle);
        Some(node)
    }

    #[must_use]
    pub fn node(&self, handle: EvalNodeHandle) -> Option<&EvalCell> {
        self.nodes.get(handle).and_then(Option::as_ref)
    }

    pub fn node_mut(&mut self, handle: EvalNodeHandle) -> Option<&mut EvalCell> {
        self.nodes.get_mut(handle).and_then(Option::as_mut)
    }

    /// Inserts `newnode` by evaluation position `pos`.
    ///
    /// Returns the existing matching handle on duplicate keys and leaves
    /// `newnode` detached, matching the C `EvalTreeInsert` return convention.
    ///
    /// # Panics
    ///
    /// Panics if either `newnode` or any traversed handle is absent, or if `pos`
    /// is outside a node's evaluation array.
    pub fn insert(&mut self, newnode: EvalNodeHandle, pos: usize) -> Option<EvalNodeHandle> {
        self.assert_node(newnode);
        if self.root.is_none() {
            self.set_left(newnode, pos, None);
            self.set_right(newnode, pos, None);
            self.root = Some(newnode);
            return None;
        }

        self.root = self.splay_tree(self.root, newnode, pos);
        let root = self.root.expect("non-empty tree should keep a root");
        let cmpres = self.compare_handles(newnode, root, pos);

        match cmpres.cmp(&0) {
            Ordering::Less => {
                let root_left = self.left(root, pos);
                self.set_left(newnode, pos, root_left);
                self.set_right(newnode, pos, Some(root));
                self.set_left(root, pos, None);
                self.root = Some(newnode);
                None
            }
            Ordering::Greater => {
                let root_right = self.right(root, pos);
                self.set_right(newnode, pos, root_right);
                self.set_left(newnode, pos, Some(root));
                self.set_right(root, pos, None);
                self.root = Some(newnode);
                None
            }
            Ordering::Equal => Some(root),
        }
    }

    /// Finds `key`, splaying the nearest node to the root as in C.
    ///
    /// # Panics
    ///
    /// Panics if `key` or any traversed handle is absent, or if `pos` is outside
    /// a node's evaluation array.
    pub fn find(&mut self, key: EvalNodeHandle, pos: usize) -> Option<EvalNodeHandle> {
        self.root?;
        self.assert_node(key);
        self.root = self.splay_tree(self.root, key, pos);
        let root = self.root.expect("non-empty tree should keep a root");
        (self.compare_handles(root, key, pos) == 0).then_some(root)
    }

    /// Extracts the matching node, detaches its tree links, and returns it.
    ///
    /// # Panics
    ///
    /// Panics if `key` or any traversed handle is absent, or if `pos` is outside
    /// a node's evaluation array.
    pub fn extract_entry(&mut self, key: EvalNodeHandle, pos: usize) -> Option<EvalNodeHandle> {
        self.root?;
        self.assert_node(key);
        self.root = self.splay_tree(self.root, key, pos);
        let root = self.root.expect("non-empty tree should keep a root");

        if self.compare_handles(key, root, pos) != 0 {
            return None;
        }

        let next_root = if self.left(root, pos).is_none() {
            self.right(root, pos)
        } else {
            let left_root = self.left(root, pos);
            let x = self
                .splay_tree(left_root, key, pos)
                .expect("left subtree should splay to a root");
            self.set_right(x, pos, self.right(root, pos));
            Some(x)
        };

        self.set_left(root, pos, None);
        self.set_right(root, pos, None);
        self.root = next_root;
        Some(root)
    }

    /// Deletes the matching node.
    ///
    /// # Panics
    ///
    /// Panics if `key` or any traversed handle is absent, or if `pos` is outside
    /// a node's evaluation array.
    pub fn delete_entry(&mut self, key: EvalNodeHandle, pos: usize) -> bool {
        if let Some(cell) = self.extract_entry(key, pos) {
            let _ = self.take_node(cell);
            true
        } else {
            false
        }
    }

    /// Finds the smallest node without splaying.
    ///
    /// # Panics
    ///
    /// Panics if any traversed handle is absent or if `pos` is outside a node's
    /// evaluation array.
    #[must_use]
    pub fn find_smallest(&self, pos: usize) -> Option<EvalNodeHandle> {
        let mut root = self.root?;
        while let Some(left) = self.left(root, pos) {
            root = left;
        }
        Some(root)
    }

    /// Initializes an in-order traversal stack.
    ///
    /// # Panics
    ///
    /// Panics if any traversed handle is absent or if `pos` is outside a node's
    /// evaluation array.
    #[must_use]
    pub fn traverse_init(&self, pos: usize) -> EvalTreeTraverseState {
        let mut state = EvalTreeTraverseState::default();
        self.push_left_spine(&mut state.stack, self.root, pos);
        state
    }

    /// Advances an in-order traversal state.
    ///
    /// # Panics
    ///
    /// Panics if any traversed handle is absent or if `pos` is outside a node's
    /// evaluation array.
    pub fn traverse_next(
        &self,
        state: &mut EvalTreeTraverseState,
        pos: usize,
    ) -> Option<EvalNodeHandle> {
        let result = state.stack.pop()?;
        self.push_left_spine(&mut state.stack, self.right(result, pos), pos);
        Some(result)
    }

    /// Returns the C debug print shape used by `EvalTreePrintInOrder`.
    ///
    /// # Panics
    ///
    /// Panics if any traversed handle is absent or if `pos` is outside a node's
    /// evaluation array.
    #[must_use]
    pub fn print_in_order_string(&self, pos: usize) -> String {
        let mut state = self.traverse_init(pos);
        let mut out = String::new();
        while let Some(handle) = self.traverse_next(&mut state, pos) {
            out.push_str(&self.eval(handle).list_print_comment_string());
            out.push('\n');
        }
        out
    }

    fn splay_tree(
        &mut self,
        tree: Option<EvalNodeHandle>,
        splay: EvalNodeHandle,
        pos: usize,
    ) -> Option<EvalNodeHandle> {
        let mut tree = tree?;
        let dummy = self.alloc_node(evals_alloc_raw(self.eval(splay).eval_no()));
        self.set_left(dummy, pos, None);
        self.set_right(dummy, pos, None);
        let mut left = dummy;
        let mut right = dummy;

        loop {
            let cmpres = self.compare_handles(splay, tree, pos);
            match cmpres.cmp(&0) {
                Ordering::Less => {
                    if self.left(tree, pos).is_none() {
                        break;
                    }
                    let tree_left = self.left(tree, pos).expect("left child checked");
                    if self.compare_handles(splay, tree_left, pos) < 0 {
                        let tmp = tree_left;
                        self.set_left(tree, pos, self.right(tmp, pos));
                        self.set_right(tmp, pos, Some(tree));
                        tree = tmp;
                        if self.left(tree, pos).is_none() {
                            break;
                        }
                    }
                    self.set_left(right, pos, Some(tree));
                    right = tree;
                    tree = self.left(tree, pos).expect("left child checked after link");
                }
                Ordering::Greater => {
                    if self.right(tree, pos).is_none() {
                        break;
                    }
                    let tree_right = self.right(tree, pos).expect("right child checked");
                    if self.compare_handles(splay, tree_right, pos) > 0 {
                        let tmp = tree_right;
                        self.set_right(tree, pos, self.left(tmp, pos));
                        self.set_left(tmp, pos, Some(tree));
                        tree = tmp;
                        if self.right(tree, pos).is_none() {
                            break;
                        }
                    }
                    self.set_right(left, pos, Some(tree));
                    left = tree;
                    tree = self
                        .right(tree, pos)
                        .expect("right child checked after link");
                }
                Ordering::Equal => break,
            }
        }

        self.set_right(left, pos, self.left(tree, pos));
        self.set_left(right, pos, self.right(tree, pos));
        self.set_left(tree, pos, self.right(dummy, pos));
        self.set_right(tree, pos, self.left(dummy, pos));
        let _ = self.take_node(dummy);
        Some(tree)
    }

    fn push_left_spine(
        &self,
        stack: &mut Vec<EvalNodeHandle>,
        mut root: Option<EvalNodeHandle>,
        pos: usize,
    ) {
        while let Some(handle) = root {
            stack.push(handle);
            root = self.left(handle, pos);
        }
    }

    fn assert_node(&self, handle: EvalNodeHandle) {
        assert!(
            self.node(handle).is_some(),
            "unknown evaluation tree handle"
        );
    }

    fn eval(&self, handle: EvalNodeHandle) -> &EvalCell {
        self.node(handle)
            .expect("evaluation tree handle should reference a live node")
    }

    fn eval_mut(&mut self, handle: EvalNodeHandle) -> &mut EvalCell {
        self.node_mut(handle)
            .expect("evaluation tree handle should reference a live node")
    }

    fn compare_handles(
        &self,
        left: EvalNodeHandle,
        right: EvalNodeHandle,
        pos: usize,
    ) -> EvalPriority {
        eval_compare(self.eval(left), self.eval(right), pos)
    }

    fn left(&self, handle: EvalNodeHandle, pos: usize) -> Option<EvalNodeHandle> {
        self.eval(handle).eval(pos).left()
    }

    fn set_left(&mut self, handle: EvalNodeHandle, pos: usize, left: Option<EvalNodeHandle>) {
        self.eval_mut(handle).eval_mut(pos).set_left(left);
    }

    fn right(&self, handle: EvalNodeHandle, pos: usize) -> Option<EvalNodeHandle> {
        self.eval(handle).eval(pos).right()
    }

    fn set_right(&mut self, handle: EvalNodeHandle, pos: usize, right: Option<EvalNodeHandle>) {
        self.eval_mut(handle).eval_mut(pos).set_right(right);
    }
}

#[must_use]
pub fn evals_alloc(eval_no: usize) -> EvalCell {
    EvalCell::alloc(eval_no)
}

#[must_use]
pub fn evaluation_counter() -> EvalPriority {
    EVALUATION_COUNTER.load(AtomicOrdering::Relaxed)
}

/// Compares two evaluation cells at evaluation position `pos`.
///
/// # Panics
///
/// Panics if `pos` is outside either cell's evaluation array, matching the
/// unchecked C flexible-array access contract.
#[must_use]
pub fn eval_compare(ev1: &EvalCell, ev2: &EvalCell, pos: usize) -> EvalPriority {
    let eval1 = ev1.eval(pos);
    let eval2 = ev2.eval(pos);

    let priority_diff = eval1.priority() - eval2.priority();
    if priority_diff != 0 {
        return priority_diff;
    }

    let count_diff = ev1.eval_count() - ev2.eval_count();
    if count_diff == 0 {
        return count_diff;
    }

    let heuristic_diff = cmp_f32_c(eval1.heuristic(), eval2.heuristic());
    if heuristic_diff != 0 {
        return heuristic_diff;
    }
    count_diff
}

/// Returns whether `ev1` is greater than `ev2` at evaluation position `pos`.
///
/// # Panics
///
/// Panics if `pos` is outside either cell's evaluation array, matching the
/// unchecked C flexible-array access contract.
#[must_use]
pub fn eval_greater(ev1: &EvalCell, ev2: &EvalCell, pos: usize) -> bool {
    let eval1 = ev1.eval(pos);
    let eval2 = ev2.eval(pos);

    if eval1.priority() > eval2.priority() {
        return true;
    }
    if eval1.priority() == eval2.priority() {
        if ev1.eval_count() == ev2.eval_count() {
            return false;
        }
        if eval1.heuristic() > eval2.heuristic() {
            return true;
        }
        if c_float_eq(eval1.heuristic(), eval2.heuristic()) && ev1.eval_count() > ev2.eval_count() {
            return true;
        }
    }
    false
}

fn evals_alloc_raw(eval_no: usize) -> EvalCell {
    EvalCell {
        eval_count: 0,
        object: None,
        evals: vec![SimpleEvalCell::new(); eval_no].into_boxed_slice(),
    }
}

fn cmp_f32_c(left: f32, right: f32) -> EvalPriority {
    EvalPriority::from(left > right) - EvalPriority::from(left < right)
}

fn c_float_eq(left: f32, right: f32) -> bool {
    matches!(left.partial_cmp(&right), Some(Ordering::Equal))
}

fn pack_handle(handle: usize) -> NonZeroUsize {
    let encoded = handle
        .checked_add(1)
        .expect("evaluation handle space exhausted");
    NonZeroUsize::new(encoded).expect("encoded evaluation handle must be nonzero")
}

fn unpack_handle(handle: NonZeroUsize) -> usize {
    handle.get() - 1
}

#[cfg(test)]
mod tests {
    use super::{
        eval_compare, eval_greater, evals_alloc, evals_alloc_raw, evaluation_counter, EvalCell,
        EvalTree, SimpleEvalCell, PRIO_BEST, PRIO_DEFER, PRIO_LARGEST_REASONABLE, PRIO_NORMAL,
        PRIO_PREFER,
    };
    use std::num::NonZeroUsize;

    fn eval_with_count(eval_count: i64, priority: i64, heuristic: f32) -> EvalCell {
        let mut eval = evals_alloc_raw(1);
        eval.eval_count = eval_count;
        eval.eval_mut(0).set_priority(priority);
        eval.eval_mut(0).set_heuristic(heuristic);
        eval
    }

    #[test]
    fn priority_constants_match_c_defines() {
        assert_eq!(PRIO_BEST, 0);
        assert_eq!(PRIO_PREFER, 30);
        assert_eq!(PRIO_NORMAL, 40);
        assert_eq!(PRIO_DEFER, 50);
        assert_eq!(PRIO_LARGEST_REASONABLE, 1_048_576);
    }

    #[test]
    fn allocation_initializes_eval_cells_and_advances_global_counter() {
        let start = evaluation_counter();
        let first = evals_alloc(2);
        let second = evals_alloc(1);

        assert_eq!(first.eval_count(), start);
        assert_eq!(second.eval_count(), start + 1);
        assert_eq!(first.eval_no(), 2);
        assert_eq!(first.object(), None);
        assert_eq!(first.eval(0).priority(), 0);
        assert_eq!(first.eval(0).heuristic().to_bits(), 0.0_f32.to_bits());
        assert_eq!(first.eval(0).left(), None);
        assert_eq!(first.eval(0).right(), None);
    }

    #[test]
    fn object_and_tree_link_slots_preserve_c_cell_shape() {
        let mut eval = evals_alloc_raw(1);

        assert_eq!(
            std::mem::size_of::<Option<NonZeroUsize>>(),
            std::mem::size_of::<usize>()
        );
        assert_eq!(std::mem::size_of::<SimpleEvalCell>(), 32);
        assert_eq!(std::mem::size_of::<EvalCell>(), 32);

        eval.set_object(Some(17));
        eval.eval_mut(0).set_left(Some(3));
        eval.eval_mut(0).set_right(Some(4));

        assert_eq!(eval.object(), Some(17));
        assert_eq!(eval.eval(0).left(), Some(3));
        assert_eq!(eval.eval(0).right(), Some(4));
    }

    #[test]
    fn heuristic_assignment_from_eval_truncates_to_c_float_storage() {
        let mut eval = evals_alloc_raw(1);
        let value = 1.0_f64 / 3.0;

        eval.eval_mut(0).set_heuristic_from_eval(value);

        assert_eq!(
            eval.eval(0).heuristic().to_bits(),
            0.333_333_34_f32.to_bits()
        );
    }

    #[test]
    fn printing_matches_c_eval_formats() {
        let mut eval = evals_alloc_raw(2);
        eval.eval_count = 7;
        eval.eval_mut(0).set_priority(PRIO_PREFER);
        eval.eval_mut(0).set_heuristic(2.5);
        eval.eval_mut(1).set_priority(PRIO_DEFER);
        eval.eval_mut(1).set_heuristic(-1.25);

        assert_eq!(eval.print_string(0), "[ 30:2.5000000000:7]");
        assert_eq!(eval.print_comment_string(0), "/*[ 30:2.5000000000:7]*/");
        assert_eq!(
            eval.list_print_string(),
            "[ 30:2.5000000000:7][ 50:-1.2500000000:7]"
        );
        assert_eq!(
            eval.list_print_comment_string(),
            "/*[ 30:2.5000000000:7][ 50:-1.2500000000:7]*/"
        );
    }

    #[test]
    fn priority_mutators_touch_all_simple_evals() {
        let mut eval = evals_alloc_raw(3);

        eval.set_priority(PRIO_NORMAL);
        eval.change_priority(5);

        assert_eq!(eval.eval(0).priority(), 45);
        assert_eq!(eval.eval(1).priority(), 45);
        assert_eq!(eval.eval(2).priority(), 45);
    }

    #[test]
    fn compare_uses_priority_then_count_and_heuristic_like_c() {
        let lower_priority = eval_with_count(1, PRIO_PREFER, 100.0);
        let higher_priority = eval_with_count(2, PRIO_NORMAL, 1.0);
        assert_eq!(eval_compare(&lower_priority, &higher_priority, 0), -10);
        assert_eq!(eval_compare(&higher_priority, &lower_priority, 0), 10);

        let better_heuristic = eval_with_count(1, PRIO_NORMAL, 10.0);
        let worse_heuristic = eval_with_count(2, PRIO_NORMAL, 5.0);
        assert_eq!(eval_compare(&better_heuristic, &worse_heuristic, 0), 1);
        assert_eq!(eval_compare(&worse_heuristic, &better_heuristic, 0), -1);

        let older = eval_with_count(1, PRIO_NORMAL, 5.0);
        let newer = eval_with_count(2, PRIO_NORMAL, 5.0);
        assert_eq!(eval_compare(&older, &newer, 0), -1);
        assert_eq!(eval_compare(&newer, &older, 0), 1);

        let same_count_left = eval_with_count(3, PRIO_NORMAL, 100.0);
        let same_count_right = eval_with_count(3, PRIO_NORMAL, 1.0);
        assert_eq!(eval_compare(&same_count_left, &same_count_right, 0), 0);
    }

    #[test]
    fn compare_treats_nan_heuristics_like_c_cmp_macro() {
        let left = eval_with_count(1, PRIO_NORMAL, f32::NAN);
        let right = eval_with_count(2, PRIO_NORMAL, 5.0);

        assert_eq!(eval_compare(&left, &right, 0), -1);
        assert!(!eval_greater(&left, &right, 0));
    }

    #[test]
    fn eval_greater_matches_c_branch_order() {
        let high_priority = eval_with_count(1, PRIO_DEFER, 1.0);
        let low_priority = eval_with_count(2, PRIO_NORMAL, 100.0);
        assert!(eval_greater(&high_priority, &low_priority, 0));

        let better_heuristic = eval_with_count(1, PRIO_NORMAL, 10.0);
        let worse_heuristic = eval_with_count(2, PRIO_NORMAL, 5.0);
        assert!(eval_greater(&better_heuristic, &worse_heuristic, 0));

        let newer = eval_with_count(3, PRIO_NORMAL, 5.0);
        let older = eval_with_count(2, PRIO_NORMAL, 5.0);
        assert!(eval_greater(&newer, &older, 0));

        let same_count_left = eval_with_count(4, PRIO_NORMAL, 100.0);
        let same_count_right = eval_with_count(4, PRIO_NORMAL, 1.0);
        assert!(!eval_greater(&same_count_left, &same_count_right, 0));
    }

    fn tree_eval(eval_count: i64, heuristic: f32) -> EvalCell {
        eval_with_count(eval_count, PRIO_NORMAL, heuristic)
    }

    fn traversal_counts(tree: &EvalTree) -> Vec<i64> {
        let mut state = tree.traverse_init(0);
        let mut counts = Vec::new();
        while let Some(handle) = tree.traverse_next(&mut state, 0) {
            counts.push(tree.node(handle).expect("live node").eval_count());
        }
        counts
    }

    #[test]
    fn eval_tree_insert_splays_and_traverses_in_c_order() {
        let mut tree = EvalTree::new();
        let first = tree.alloc_node(tree_eval(1, 30.0));
        let second = tree.alloc_node(tree_eval(2, 10.0));
        let third = tree.alloc_node(tree_eval(3, 20.0));

        assert_eq!(tree.insert(first, 0), None);
        assert_eq!(tree.root(), Some(first));
        assert_eq!(tree.insert(second, 0), None);
        assert_eq!(tree.root(), Some(second));
        assert_eq!(tree.insert(third, 0), None);
        assert_eq!(tree.root(), Some(third));

        assert_eq!(traversal_counts(&tree), vec![2, 3, 1]);
        assert_eq!(tree.find_smallest(0), Some(second));
        assert_eq!(
            tree.print_in_order_string(0),
            "/*[ 40:10.0000000000:2]*/\n/*[ 40:20.0000000000:3]*/\n/*[ 40:30.0000000000:1]*/\n"
        );
    }

    #[test]
    fn eval_tree_find_splays_found_or_nearest_node() {
        let mut tree = EvalTree::new();
        let low = tree.alloc_node(tree_eval(1, 10.0));
        let mid = tree.alloc_node(tree_eval(2, 20.0));
        let high = tree.alloc_node(tree_eval(3, 30.0));
        let missing = tree.alloc_node(tree_eval(4, 25.0));

        assert_eq!(tree.insert(low, 0), None);
        assert_eq!(tree.insert(mid, 0), None);
        assert_eq!(tree.insert(high, 0), None);

        assert_eq!(tree.find(mid, 0), Some(mid));
        assert_eq!(tree.root(), Some(mid));
        assert_eq!(tree.find(missing, 0), None);
        assert_eq!(tree.root(), Some(high));
        assert_eq!(traversal_counts(&tree), vec![1, 2, 3]);
    }

    #[test]
    fn eval_tree_insert_duplicate_returns_existing_and_keeps_newnode_detached() {
        let mut tree = EvalTree::new();
        let original = tree.alloc_node(tree_eval(7, 10.0));
        let duplicate = tree.alloc_node(tree_eval(7, 99.0));

        assert_eq!(tree.insert(original, 0), None);
        assert_eq!(tree.insert(duplicate, 0), Some(original));

        assert_eq!(tree.root(), Some(original));
        assert_eq!(traversal_counts(&tree), vec![7]);
        assert_eq!(
            tree.node(duplicate)
                .expect("duplicate remains allocated")
                .eval(0)
                .left(),
            None
        );
        assert_eq!(
            tree.node(duplicate)
                .expect("duplicate remains allocated")
                .eval(0)
                .right(),
            None
        );
    }

    #[test]
    fn eval_tree_extract_detaches_entry_and_reassembles_children() {
        let mut tree = EvalTree::new();
        let low = tree.alloc_node(tree_eval(1, 10.0));
        let mid = tree.alloc_node(tree_eval(2, 20.0));
        let high = tree.alloc_node(tree_eval(3, 30.0));

        assert_eq!(tree.insert(low, 0), None);
        assert_eq!(tree.insert(mid, 0), None);
        assert_eq!(tree.insert(high, 0), None);

        assert_eq!(tree.extract_entry(mid, 0), Some(mid));
        assert_eq!(
            tree.node(mid)
                .expect("extracted node remains owned")
                .eval(0)
                .left(),
            None
        );
        assert_eq!(
            tree.node(mid)
                .expect("extracted node remains owned")
                .eval(0)
                .right(),
            None
        );
        assert_eq!(traversal_counts(&tree), vec![1, 3]);
        assert_eq!(tree.find(mid, 0), None);
    }

    #[test]
    fn eval_tree_delete_removes_and_frees_matching_entry() {
        let mut tree = EvalTree::new();
        let low = tree.alloc_node(tree_eval(1, 10.0));
        let high = tree.alloc_node(tree_eval(2, 20.0));
        let missing = tree.alloc_node(tree_eval(3, 30.0));

        assert_eq!(tree.insert(low, 0), None);
        assert_eq!(tree.insert(high, 0), None);

        assert!(tree.delete_entry(low, 0));
        assert!(tree.node(low).is_none());
        assert_eq!(traversal_counts(&tree), vec![2]);
        assert!(!tree.delete_entry(missing, 0));
        assert_eq!(traversal_counts(&tree), vec![2]);
    }
}
