use crate::basics::simple_stuff::{problem_type, ProblemType};
use crate::terms::simpletypes::type_identity_cmp;
use crate::terms::termtypes::{term_identity_cmp, Term, TermProperties};
use std::cmp::Ordering;

#[derive(Clone, Debug, Default)]
pub struct TermTree {
    root: Option<Term>,
}

impl TermTree {
    #[must_use]
    pub const fn new() -> Self {
        Self { root: None }
    }

    #[must_use]
    pub fn root(&self) -> Option<Term> {
        self.root.clone()
    }

    pub fn clear(&mut self) {
        self.root = None;
    }

    #[must_use]
    pub fn find(&mut self, key: &Term) -> Option<Term> {
        if let Some(root) = self.root.take() {
            let root = splay_term_tree(root, key);
            let found = (term_top_compare(&root, key) == 0).then_some(root.clone());
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

        let root = splay_term_tree(root, &new);
        let cmp = term_top_compare(&new, &root);
        match cmp.cmp(&0) {
            Ordering::Less => {
                new.set_left_son(root.left_son());
                new.set_right_son(Some(root.clone()));
                root.set_left_son(None);
                self.root = Some(new);
                None
            }
            Ordering::Greater => {
                new.set_right_son(root.right_son());
                new.set_left_son(Some(root.clone()));
                root.set_right_son(None);
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
        let root = splay_term_tree(root, key);
        if term_top_compare(key, &root) != 0 {
            self.root = Some(root);
            return None;
        }

        let next_root = if let Some(left) = root.left_son() {
            let new_root = splay_term_tree(left, key);
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
}

/// Compares top-level term cells by the C term-tree key.
///
/// # Panics
///
/// Panics if either term has no type, or if first-order mode compares terms
/// with distinct type handles. The C function encodes these as assertions.
#[must_use]
pub fn term_top_compare(left: &Term, right: &Term) -> i64 {
    term_top_compare_for_problem(left, right, problem_type())
}

/// Compares top-level term cells by the C term-tree key for a selected syntax mode.
///
/// # Panics
///
/// Panics if either term has no type, or if first-order mode compares terms
/// with distinct type handles. The C function encodes these as assertions.
#[must_use]
pub fn term_top_compare_for_problem(left: &Term, right: &Term, problem_type: ProblemType) -> i64 {
    let mut result = left.f_code() - right.f_code();
    if result != 0 {
        return result;
    }

    let left_type = left.type_().expect("term top comparison requires types");
    let right_type = right.type_().expect("term top comparison requires types");
    if problem_type == ProblemType::HigherOrder {
        result = i64::from(type_identity_cmp(&left_type, &right_type));
        if result != 0 {
            return result;
        }
    } else {
        assert_eq!(left_type, right_type, "first-order term types must match");
    }

    result = i64::try_from(left.arity()).unwrap_or(i64::MAX)
        - i64::try_from(right.arity()).unwrap_or(i64::MAX);
    if result != 0 {
        return result;
    }

    for index in 0..left.arity() {
        let left_arg = left
            .argument(index)
            .expect("term top comparison requires initialized arguments");
        let right_arg = right
            .argument(index)
            .expect("term top comparison requires initialized arguments");
        result = i64::from(term_identity_cmp(&left_arg, &right_arg));
        if result != 0 {
            return result;
        }
    }
    result
}

fn splay_term_tree(mut tree: Term, key: &Term) -> Term {
    let mut left_root = None;
    let mut left_tail: Option<Term> = None;
    let mut right_root = None;
    let mut right_tail: Option<Term> = None;

    loop {
        let cmp = term_top_compare(key, &tree);
        match cmp.cmp(&0) {
            Ordering::Less => {
                let Some(mut next) = tree.take_left_son() else {
                    break;
                };
                if term_top_compare(key, &next) < 0 {
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
                if term_top_compare(key, &next) > 0 {
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
    use crate::terms::simpletypes::alloc_simple_sort;
    use crate::terms::termtypes::{Term, TP_CHECK_FLAG, TP_TOP_POS};
    use crate::terms::typebanks::TypeBank;

    fn typed_const(f_code: i64, type_: &crate::terms::simpletypes::Type) -> Term {
        let term = Term::const_cell_alloc(f_code);
        term.set_type(Some(type_.clone()));
        term
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
        same_top_different_arg.set_argument(0, right);
        assert_ne!(term_top_compare(&binary, &same_top_different_arg), 0);

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
}
