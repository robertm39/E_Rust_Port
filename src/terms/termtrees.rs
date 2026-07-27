use crate::basics::simple_stuff::{problem_type, ProblemType};
use crate::terms::termtypes::{Term, TermProperties};
use std::cmp::Ordering;

// Intrusive left/right links live in each `Term`, so independently cloned or
// externally assembled trees could alias and relink the same cells. Keep the
// owner crate-private and non-cloneable; production construction is confined
// to `TermCellStore`, which assigns each shared term to exactly one bucket.
#[derive(Debug, Default)]
pub(crate) struct TermTree {
    root: Option<Term>,
}

impl TermTree {
    #[must_use]
    pub const fn new() -> Self {
        Self { root: None }
    }

    #[cfg(test)]
    #[must_use]
    fn root(&self) -> Option<Term> {
        self.root.clone()
    }

    pub fn clear(&mut self) {
        self.root = None;
    }

    #[must_use]
    pub fn find(&mut self, key: &Term) -> Option<Term> {
        if let Some(root) = self.root.take() {
            let problem_type = problem_type();
            let root = splay_term_tree(root, key, problem_type);
            let found = term_top_order_for_problem(&root, key, problem_type)
                .is_eq()
                .then_some(root.clone());
            self.root = Some(root);
            found
        } else {
            None
        }
    }

    pub fn insert(&mut self, new: Term) -> Option<Term> {
        let Some(root) = self.root.take() else {
            new.clear_tree_links();
            self.root = Some(new);
            return None;
        };

        let problem_type = problem_type();
        let root = splay_term_tree(root, &new, problem_type);
        match term_top_order_for_problem(&new, &root, problem_type) {
            Ordering::Less => {
                new.set_left_son(root.take_left_son());
                new.set_right_son(Some(root));
                self.root = Some(new);
                None
            }
            Ordering::Greater => {
                new.set_right_son(root.take_right_son());
                new.set_left_son(Some(root));
                self.root = Some(new);
                None
            }
            Ordering::Equal => {
                self.root = Some(root.clone());
                Some(root)
            }
        }
    }

    pub fn extract(&mut self, key: &Term) -> Option<Term> {
        let root = self.root.take()?;
        let problem_type = problem_type();
        let root = splay_term_tree(root, key, problem_type);
        if !term_top_order_for_problem(key, &root, problem_type).is_eq() {
            self.root = Some(root);
            return None;
        }

        let next_root = if let Some(left) = root.left_son() {
            let new_root = splay_term_tree(left, key, problem_type);
            new_root.set_right_son(root.right_son());
            Some(new_root)
        } else {
            root.right_son()
        };
        root.clear_tree_links();
        self.root = next_root;
        Some(root)
    }

    pub fn delete(&mut self, key: &Term) -> bool {
        self.extract(key).is_some()
    }

    pub fn set_prop(&self, props: TermProperties) {
        walk_tree(self.root.as_ref(), |term| term.set_prop(props));
    }

    pub fn del_prop(&self, props: TermProperties) {
        walk_tree(self.root.as_ref(), |term| term.del_prop(props));
    }

    #[must_use]
    pub fn nodes(&self) -> i64 {
        let mut count = 0;
        walk_tree(self.root.as_ref(), |_| count += 1);
        count
    }

    #[must_use]
    pub fn terms(&self) -> Vec<Term> {
        let mut terms = Vec::new();
        walk_tree(self.root.as_ref(), |term| terms.push(term.clone()));
        terms
    }

    pub(crate) fn collect_matching(
        &self,
        result: &mut Vec<Term>,
        mut predicate: impl FnMut(&Term) -> bool,
    ) {
        walk_tree(self.root.as_ref(), |term| {
            if predicate(term) {
                result.push(term.clone());
            }
        });
    }
}

/// Compares top-level term cells by the C term-tree key.
///
/// # Panics
///
/// In higher-order mode, panics if either term has no type. In debug builds,
/// also panics if first-order terms lack types or have distinct type handles.
/// The C function encodes the first-order preconditions as assertions.
#[must_use]
pub fn term_top_compare(left: &Term, right: &Term) -> i64 {
    term_top_compare_for_problem(left, right, problem_type())
}

/// Compares top-level term cells by the C term-tree key for a selected syntax mode.
///
/// # Panics
///
/// In higher-order mode, panics if either term has no type. In debug builds,
/// also panics if first-order terms lack types or have distinct type handles.
/// The C function encodes the first-order preconditions as assertions.
#[must_use]
pub fn term_top_compare_for_problem(left: &Term, right: &Term, problem_type: ProblemType) -> i64 {
    match term_top_order_for_problem(left, right, problem_type) {
        Ordering::Less => -1,
        Ordering::Equal => 0,
        Ordering::Greater => 1,
    }
}

#[expect(
    clippy::inline_always,
    reason = "pinned whole-prover Callgrind improves when this hot comparator is forced inline"
)]
#[allow(
    unsafe_code,
    reason = "measured private comparison over stable term-tree inputs"
)]
#[inline(always)]
fn term_top_order_for_problem(left: &Term, right: &Term, problem_type: ProblemType) -> Ordering {
    // SAFETY: Both owned handles keep their cells and initialized argument
    // handles live for this synchronous comparison. Term-tree operations
    // mutate only the disjoint intrusive left/right fields; every production
    // argument guard is dropped before store entry, and types are complete.
    unsafe {
        left.borrowed_cell().compare_top_order(
            right.borrowed_cell(),
            problem_type == ProblemType::HigherOrder,
        )
    }
}

#[expect(
    clippy::inline_always,
    reason = "pinned whole-prover Callgrind improves when this hot splay is forced inline"
)]
#[inline(always)]
fn splay_term_tree(mut tree: Term, key: &Term, problem_type: ProblemType) -> Term {
    let mut left_root = None;
    let mut left_tail: Option<Term> = None;
    let mut right_root = None;
    let mut right_tail: Option<Term> = None;

    loop {
        match term_top_order_for_problem(key, &tree, problem_type) {
            Ordering::Less => {
                let Some(mut next) = tree.take_left_son() else {
                    break;
                };
                if term_top_order_for_problem(key, &next, problem_type) == Ordering::Less {
                    let tmp = next;
                    tree.set_left_son(tmp.take_right_son());
                    tmp.set_right_son(Some(tree));
                    tree = tmp;
                    let Some(left_child) = tree.take_left_son() else {
                        break;
                    };
                    next = left_child;
                }
                if let Some(tail) = right_tail.as_ref() {
                    tail.set_left_son(Some(tree.clone()));
                } else {
                    right_root = Some(tree.clone());
                }
                right_tail = Some(tree);
                tree = next;
            }
            Ordering::Greater => {
                let Some(mut next) = tree.take_right_son() else {
                    break;
                };
                if term_top_order_for_problem(key, &next, problem_type) == Ordering::Greater {
                    let tmp = next;
                    tree.set_right_son(tmp.take_left_son());
                    tmp.set_left_son(Some(tree));
                    tree = tmp;
                    let Some(right_child) = tree.take_right_son() else {
                        break;
                    };
                    next = right_child;
                }
                if let Some(tail) = left_tail.as_ref() {
                    tail.set_right_son(Some(tree.clone()));
                } else {
                    left_root = Some(tree.clone());
                }
                left_tail = Some(tree);
                tree = next;
            }
            Ordering::Equal => break,
        }
    }

    if let Some(left_tail) = left_tail {
        left_tail.set_right_son(tree.take_left_son());
        tree.set_left_son(left_root);
    }
    if let Some(right_tail) = right_tail {
        right_tail.set_left_son(tree.take_right_son());
        tree.set_right_son(right_root);
    }
    tree
}

fn walk_tree(root: Option<&Term>, mut visit: impl FnMut(&Term)) {
    let mut stack = Vec::new();
    if let Some(root) = root {
        stack.push(root.clone());
    }
    while let Some(term) = stack.pop() {
        visit(&term);
        if let Some(left) = term.left_son() {
            stack.push(left);
        }
        if let Some(right) = term.right_son() {
            stack.push(right);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{term_top_compare, term_top_compare_for_problem, TermTree};
    use crate::basics::simple_stuff::ProblemType;
    use crate::terms::simpletypes::{alloc_simple_sort, type_identity_cmp};
    use crate::terms::termtypes::{
        term_identity_cmp, Term, TP_CHECK_FLAG, TP_GARBAGE_FLAG, TP_TOP_POS,
    };
    use crate::terms::typebanks::TypeBank;
    use std::cmp::Ordering;

    fn typed_const(f_code: i64, type_: &crate::terms::simpletypes::Type) -> Term {
        let term = Term::const_cell_alloc(f_code);
        term.set_type(Some(type_.clone()));
        term
    }

    #[allow(
        unsafe_code,
        reason = "the comparison test owns both immutable term graphs for the complete argument traversal"
    )]
    fn owned_top_order(left: &Term, right: &Term, problem_type: ProblemType) -> Ordering {
        let mut result = left.f_code().cmp(&right.f_code());
        if result != Ordering::Equal {
            return result;
        }
        if problem_type == ProblemType::HigherOrder {
            result = type_identity_cmp(&left.type_().unwrap(), &right.type_().unwrap()).cmp(&0);
            if result != Ordering::Equal {
                return result;
            }
        }
        result = left.arity().cmp(&right.arity());
        if result != Ordering::Equal {
            return result;
        }
        // SAFETY: Both term graphs remain owned and structurally unchanged
        // throughout this synchronous comparison.
        let left_arguments = unsafe { left.arguments() };
        // SAFETY: The same owner and no-mutation scope covers `right`.
        let right_arguments = unsafe { right.arguments() };
        for (left_arg, right_arg) in left_arguments.iter().zip(right_arguments.iter()) {
            result =
                term_identity_cmp(left_arg.as_ref().unwrap(), right_arg.as_ref().unwrap()).cmp(&0);
            if result != Ordering::Equal {
                return result;
            }
        }
        result
    }

    #[test]
    fn term_top_compare_uses_f_code_type_arity_and_arg_identity() {
        let types = TypeBank::new();
        let left = typed_const(1, &types.i_type());
        let right = typed_const(2, &types.i_type());
        assert!(term_top_compare(&left, &right) < 0);

        let binary = Term::top_alloc(3, 1);
        binary.set_type(Some(types.i_type()));
        binary.set_argument(0, left.clone());
        let same_top_different_arg = Term::top_alloc(3, 1);
        same_top_different_arg.set_type(Some(types.i_type()));
        same_top_different_arg.set_argument(0, right.clone());
        assert_eq!(
            term_top_compare(&binary, &same_top_different_arg),
            i64::from(term_identity_cmp(&left, &right))
        );

        let arity_two = Term::top_alloc(3, 2);
        arity_two.set_type(Some(types.i_type()));
        assert!(term_top_compare(&binary, &arity_two) < 0);
    }

    #[test]
    fn higher_order_comparison_uses_type_identity_before_arity() {
        let type_a = alloc_simple_sort(20);
        let type_b = alloc_simple_sort(20);
        let left = typed_const(1, &type_a);
        let right = typed_const(1, &type_b);

        assert_ne!(
            term_top_compare_for_problem(&left, &right, ProblemType::HigherOrder),
            0
        );
    }

    #[test]
    fn borrowed_top_cursor_matches_owned_comparison_boundaries() {
        let types = TypeBank::new();
        let i_type = types.i_type();
        let one = typed_const(1, &i_type);
        let two = typed_const(2, &i_type);
        let left = Term::top_alloc(3, 1);
        left.set_type(Some(i_type.clone()));
        left.set_argument(0, one.clone());
        let right = Term::top_alloc(3, 1);
        right.set_type(Some(i_type.clone()));
        right.set_argument(0, two.clone());
        let binary = Term::top_alloc(3, 2);
        binary.set_type(Some(i_type.clone()));
        binary.set_argument(0, one.clone());
        binary.set_argument(1, two.clone());
        let ternary_left = Term::top_alloc(3, 3);
        ternary_left.set_type(Some(i_type.clone()));
        ternary_left.set_argument(0, one.clone());
        ternary_left.set_argument(1, two.clone());
        ternary_left.set_argument(2, one.clone());
        let ternary_right = Term::top_alloc(3, 3);
        ternary_right.set_type(Some(i_type.clone()));
        ternary_right.set_argument(0, one.clone());
        ternary_right.set_argument(1, two.clone());
        ternary_right.set_argument(2, two.clone());

        for (left, right) in [
            (&one, &two),
            (&left, &right),
            (&right, &left),
            (&left, &binary),
            (&binary, &left),
            (&binary, &ternary_left),
            (&ternary_left, &binary),
            (&ternary_left, &ternary_right),
            (&ternary_right, &ternary_left),
            (&ternary_left, &ternary_left),
            (&left, &left),
        ] {
            let expected = owned_top_order(left, right, ProblemType::FirstOrder);
            assert_eq!(
                term_top_compare_for_problem(left, right, ProblemType::FirstOrder),
                match expected {
                    Ordering::Less => -1,
                    Ordering::Equal => 0,
                    Ordering::Greater => 1,
                }
            );
        }

        let type_a = alloc_simple_sort(20);
        let type_b = alloc_simple_sort(20);
        let left = typed_const(4, &type_a);
        let right = typed_const(4, &type_b);
        let expected = owned_top_order(&left, &right, ProblemType::HigherOrder);
        assert_eq!(
            term_top_compare_for_problem(&left, &right, ProblemType::HigherOrder),
            match expected {
                Ordering::Less => -1,
                Ordering::Equal => 0,
                Ordering::Greater => 1,
            }
        );
    }

    #[test]
    fn insert_find_extract_and_delete_follow_splay_tree_contract() {
        let types = TypeBank::new();
        let mut term_tree = TermTree::new();
        let one = typed_const(1, &types.i_type());
        let two = typed_const(2, &types.i_type());
        let third = typed_const(3, &types.i_type());

        assert!(term_tree.insert(two.clone()).is_none());
        assert!(term_tree.insert(one.clone()).is_none());
        assert!(term_tree.insert(third.clone()).is_none());
        assert_eq!(term_tree.nodes(), 3);
        assert_eq!(term_tree.find(&one), Some(one.clone()));
        assert_eq!(term_tree.root(), Some(one.clone()));

        let duplicate = typed_const(2, &types.i_type());
        assert_eq!(term_tree.insert(duplicate), Some(two.clone()));
        assert_eq!(term_tree.nodes(), 3);

        assert_eq!(term_tree.extract(&two), Some(two.clone()));
        assert_eq!(term_tree.nodes(), 2);
        assert!(!term_tree.delete(&two));
        assert!(term_tree.delete(&third));
        assert_eq!(term_tree.nodes(), 1);
    }

    #[test]
    fn tree_property_helpers_visit_all_nodes() {
        let types = TypeBank::new();
        let mut tree = TermTree::new();
        let one = typed_const(1, &types.i_type());
        let two = typed_const(2, &types.i_type());
        tree.insert(one.clone());
        tree.insert(two.clone());

        tree.set_prop(TP_CHECK_FLAG | TP_TOP_POS);
        assert!(one.query_prop(TP_CHECK_FLAG | TP_TOP_POS));
        assert!(two.query_prop(TP_CHECK_FLAG | TP_TOP_POS));
        tree.del_prop(TP_TOP_POS);
        assert!(one.query_prop(TP_CHECK_FLAG));
        assert!(!one.query_prop(TP_TOP_POS));
        assert_eq!(tree.terms().len(), 2);
    }

    #[test]
    fn matching_collection_clones_only_selected_terms() {
        let types = TypeBank::new();
        let mut term_tree = TermTree::new();
        let one = typed_const(1, &types.i_type());
        let two = typed_const(2, &types.i_type());
        let three = typed_const(3, &types.i_type());
        term_tree.insert(one.clone());
        term_tree.insert(two.clone());
        term_tree.insert(three.clone());
        one.set_prop(TP_GARBAGE_FLAG);
        three.set_prop(TP_GARBAGE_FLAG);

        let mut matching = Vec::new();
        term_tree.collect_matching(&mut matching, |term| term.query_prop(TP_GARBAGE_FLAG));

        assert_eq!(matching.len(), 2);
        assert!(matching.contains(&one));
        assert!(matching.contains(&three));
        assert!(!matching.contains(&two));
    }
}
