use crate::terms::functypes::FunCode;
use crate::terms::termfunc::term_weight_compute;
use crate::terms::termtypes::Term;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TreeDistanceCosts {
    pub ins_cost: i64,
    pub del_cost: i64,
    pub ch_cost: i64,
}

impl TreeDistanceCosts {
    #[must_use]
    pub const fn new(ins_cost: i64, del_cost: i64, ch_cost: i64) -> Self {
        Self {
            ins_cost,
            del_cost,
            ch_cost,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct TedTraversal {
    leftmost: Vec<usize>,
    code: Vec<FunCode>,
    key_roots: Vec<usize>,
    fresh: usize,
}

/// Computes C `ted_term_distance` for already-normalized terms.
///
/// # Panics
///
/// Panics if a traversed non-free, non-constant term has no argument 0, or if
/// any traversed compound term has an uninitialized argument. This mirrors the
/// unchecked C traversal in `ted_lmld_kr`.
#[must_use]
pub fn ted_term_distance(left: &Term, right: &Term, costs: TreeDistanceCosts) -> f64 {
    let left_traversal = ted_traversal(left);
    let right_traversal = ted_traversal(right);
    let left_len = left_traversal.code.len() - 1;
    let right_len = right_traversal.code.len() - 1;
    let mut tree_distance = vec![vec![0_i64; right_len + 1]; left_len + 1];

    for &left_root in &left_traversal.key_roots {
        for &right_root in &right_traversal.key_roots {
            ted_forest_distance(
                left_root,
                right_root,
                &left_traversal,
                &right_traversal,
                &mut tree_distance,
                costs,
            );
        }
    }

    i64_to_f64(tree_distance[left_len][right_len])
}

/// Scores `term` against normalized conjecture terms using tree edit distance.
///
/// # Panics
///
/// Panics under the same conditions as [`ted_term_distance`].
#[must_use]
pub fn ted_term_weight(term: &Term, conjecture_terms: &[Term], costs: TreeDistanceCosts) -> f64 {
    let mut minimum = f64::MAX;
    for conjecture in conjecture_terms {
        minimum = minimum.min(ted_term_distance(term, conjecture, costs));
    }
    minimum
}

fn ted_traversal(term: &Term) -> TedTraversal {
    let len = term_node_count(term);
    let mut traversal = TedTraversal {
        leftmost: vec![0; len + 1],
        code: vec![0; len + 1],
        key_roots: Vec::new(),
        fresh: 1,
    };
    let _ = ted_lmld_key_roots(term, &mut traversal, true);
    debug_assert_eq!(traversal.fresh, len + 1);
    traversal
}

fn ted_lmld_key_roots(term: &Term, traversal: &mut TedTraversal, is_root: bool) -> usize {
    let (idx, leftmost_idx) = if term.is_free_var() || term.is_const() {
        let idx = traversal.fresh;
        traversal.fresh += 1;
        traversal.leftmost[idx] = idx;
        (idx, idx)
    } else {
        let first = term
            .argument(0)
            .unwrap_or_else(|| panic!("tree-distance term argument 0 is uninitialized"));
        let first_leftmost = ted_lmld_key_roots(&first, traversal, false);
        for index in 1..term.arity() {
            let arg = term
                .argument(index)
                .unwrap_or_else(|| panic!("tree-distance term argument {index} is uninitialized"));
            let _ = ted_lmld_key_roots(&arg, traversal, true);
        }

        let idx = traversal.fresh;
        traversal.fresh += 1;
        traversal.leftmost[idx] = traversal.leftmost[first_leftmost];
        (idx, first_leftmost)
    };

    traversal.code[idx] = term.f_code();
    if is_root {
        traversal.key_roots.push(idx);
    }
    leftmost_idx
}

fn ted_forest_distance(
    i: usize,
    j: usize,
    left: &TedTraversal,
    right: &TedTraversal,
    tree_distance: &mut [Vec<i64>],
    costs: TreeDistanceCosts,
) {
    let mut forest_distance = vec![vec![0_i64; j + 1]; i + 1];
    let left_base = left.leftmost[i] - 1;
    let right_base = right.leftmost[j] - 1;

    forest_distance[left_base][right_base] = 0;
    for di in left.leftmost[i]..=i {
        forest_distance[di][right_base] = forest_distance[di - 1][right_base] + costs.del_cost;
    }
    for dj in right.leftmost[j]..=j {
        forest_distance[left_base][dj] = forest_distance[left_base][dj - 1] + costs.ins_cost;
    }
    for di in left.leftmost[i]..=i {
        for dj in right.leftmost[j]..=j {
            if left.leftmost[di] == left.leftmost[i] && right.leftmost[dj] == right.leftmost[j] {
                forest_distance[di][dj] = min3(
                    forest_distance[di - 1][dj] + costs.del_cost,
                    forest_distance[di][dj - 1] + costs.ins_cost,
                    forest_distance[di - 1][dj - 1]
                        + if left.code[di] == right.code[dj] {
                            0
                        } else {
                            costs.ch_cost
                        },
                );
                tree_distance[di][dj] = forest_distance[di][dj];
            } else {
                forest_distance[di][dj] = min3(
                    forest_distance[di - 1][dj] + costs.del_cost,
                    forest_distance[di][dj - 1] + costs.ins_cost,
                    forest_distance[left.leftmost[di] - 1][right.leftmost[dj] - 1]
                        + tree_distance[di][dj],
                );
            }
        }
    }
}

fn term_node_count(term: &Term) -> usize {
    usize::try_from(term_weight_compute(term, 1, 1))
        .expect("tree-distance term node count fits usize")
}

fn min3(left: i64, middle: i64, right: i64) -> i64 {
    left.min(middle).min(right)
}

#[allow(clippy::cast_precision_loss)]
fn i64_to_f64(value: i64) -> f64 {
    value as f64
}

#[cfg(test)]
mod tests {
    use super::{ted_term_distance, ted_term_weight, ted_traversal, TreeDistanceCosts};
    use crate::terms::functypes::FunCode;
    use crate::terms::termtypes::{Term, TP_IS_DB_VAR};

    fn costs(ins_cost: i64, del_cost: i64, ch_cost: i64) -> TreeDistanceCosts {
        TreeDistanceCosts::new(ins_cost, del_cost, ch_cost)
    }

    fn assert_f64_bits_eq(actual: f64, expected: f64) {
        assert_eq!(actual.to_bits(), expected.to_bits());
    }

    fn unary(code: FunCode, arg: &Term) -> Term {
        let term = Term::top_alloc(code, 1);
        term.set_argument(0, arg.clone());
        term
    }

    fn binary(code: FunCode, left: &Term, right: &Term) -> Term {
        let term = Term::top_alloc(code, 2);
        term.set_argument(0, left.clone());
        term.set_argument(1, right.clone());
        term
    }

    #[test]
    fn traversal_matches_c_leftmost_leaf_and_key_root_shape() {
        let a = Term::const_cell_alloc(1);
        let b = Term::const_cell_alloc(2);
        let g = unary(20, &a);
        let root = binary(10, &g, &b);
        let traversal = ted_traversal(&root);

        assert_eq!(traversal.code, vec![0, 1, 20, 2, 10]);
        assert_eq!(traversal.leftmost, vec![0, 1, 1, 3, 1]);
        assert_eq!(traversal.key_roots, vec![3, 4]);
    }

    #[test]
    fn term_distance_handles_leaf_change_insert_and_delete_costs() {
        let a = Term::const_cell_alloc(1);
        let b = Term::const_cell_alloc(2);
        let fa = unary(10, &a);

        assert_f64_bits_eq(ted_term_distance(&a, &a, costs(2, 3, 5)), 0.0);
        assert_f64_bits_eq(ted_term_distance(&a, &b, costs(2, 3, 7)), 5.0);
        assert_f64_bits_eq(ted_term_distance(&a, &fa, costs(2, 3, 7)), 2.0);
        assert_f64_bits_eq(ted_term_distance(&fa, &a, costs(2, 3, 7)), 3.0);
    }

    #[test]
    fn term_distance_uses_tree_structure_and_root_change() {
        let a = Term::const_cell_alloc(1);
        let b = Term::const_cell_alloc(2);
        let c = Term::const_cell_alloc(3);
        let fab = binary(10, &a, &b);
        let fac = binary(10, &a, &c);
        let gab = binary(11, &a, &b);

        assert_f64_bits_eq(ted_term_distance(&fab, &fac, costs(2, 3, 4)), 4.0);
        assert_f64_bits_eq(ted_term_distance(&fab, &gab, costs(2, 3, 4)), 4.0);
    }

    #[test]
    fn term_weight_returns_minimum_or_dbl_max() {
        let term = binary(10, &Term::const_cell_alloc(1), &Term::const_cell_alloc(2));
        let exact = binary(10, &Term::const_cell_alloc(1), &Term::const_cell_alloc(2));
        let close = binary(10, &Term::const_cell_alloc(1), &Term::const_cell_alloc(3));

        assert_f64_bits_eq(ted_term_weight(&term, &[close], costs(2, 3, 4)), 4.0);
        assert_f64_bits_eq(
            ted_term_weight(&term, &[Term::const_cell_alloc(99), exact], costs(2, 3, 4)),
            0.0,
        );
        assert_f64_bits_eq(ted_term_weight(&term, &[], costs(2, 3, 4)), f64::MAX);
    }

    #[test]
    #[should_panic(expected = "tree-distance term argument 0 is uninitialized")]
    fn bare_db_variable_follows_c_non_leaf_path_and_panics() {
        let db = Term::const_cell_alloc(0);
        db.set_prop(TP_IS_DB_VAR);

        let _ = ted_term_distance(&db, &Term::const_cell_alloc(1), costs(1, 1, 1));
    }
}
