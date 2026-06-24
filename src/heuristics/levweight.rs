use crate::terms::functypes::FunCode;
use crate::terms::termtypes::Term;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LevDistanceCosts {
    pub ins_cost: i32,
    pub del_cost: i32,
    pub ch_cost: i32,
}

impl LevDistanceCosts {
    #[must_use]
    pub const fn new(ins_cost: i32, del_cost: i32, ch_cost: i32) -> Self {
        Self {
            ins_cost,
            del_cost,
            ch_cost,
        }
    }
}

/// Extracts the f-code sequence produced by C `TermLRTraverseNext`.
///
/// # Panics
///
/// Panics if a traversed non-leaf term has an uninitialized argument, matching
/// the C traversal precondition that all argument slots contain valid terms.
#[must_use]
pub fn lev_compute_term_code(term: &Term) -> Vec<FunCode> {
    let mut code = Vec::new();
    let mut stack = vec![term.clone()];

    while let Some(current) = stack.pop() {
        code.push(current.f_code());
        if current.is_top_level_free_var() {
            continue;
        }

        let start = usize::from(current.is_lambda() || current.is_applied_db_var());
        for index in (start..current.arity()).rev() {
            let arg = current
                .argument(index)
                .unwrap_or_else(|| panic!("term argument {index} is uninitialized"));
            stack.push(arg);
        }
    }

    code
}

/// Computes the C Levenshtein distance over term-code sequences.
///
/// # Panics
///
/// Panics if either code sequence length does not fit C's `unsigned int`
/// loop counters.
#[must_use]
pub fn lev_codes_distance(code1: &[FunCode], code2: &[FunCode], costs: LevDistanceCosts) -> f64 {
    let ins_cost = c_int_to_uint(costs.ins_cost);
    let del_cost = c_int_to_uint(costs.del_cost);
    let ch_cost = c_int_to_uint(costs.ch_cost);
    let s1_len = code1.len();
    let s2_len = code2.len();
    let mut column = vec![0_u32; s1_len + 1];

    for (index, value) in column.iter_mut().enumerate() {
        *value = usize_to_c_uint(index).wrapping_mul(del_cost);
    }
    for x in 1..=s2_len {
        column[0] = usize_to_c_uint(x).wrapping_mul(ins_cost);
        let mut last_diag = usize_to_c_uint(x - 1).wrapping_mul(ins_cost);
        for y in 1..=s1_len {
            let old_diag = column[y];
            let del = column[y].wrapping_add(del_cost);
            let ins = column[y - 1].wrapping_add(ins_cost);
            let ch = last_diag.wrapping_add(if code1[y - 1] == code2[x - 1] {
                0
            } else {
                ch_cost
            });
            column[y] = del.min(ins).min(ch);
            last_diag = old_diag;
        }
    }

    f64::from(column[s1_len])
}

/// Computes the C Levenshtein distance between two terms' LR traversal codes.
///
/// # Panics
///
/// Panics under the same conditions as [`lev_compute_term_code`] and
/// [`lev_codes_distance`].
#[must_use]
pub fn lev_term_distance(left: &Term, right: &Term, costs: LevDistanceCosts) -> f64 {
    let left_code = lev_compute_term_code(left);
    let right_code = lev_compute_term_code(right);
    lev_codes_distance(&left_code, &right_code, costs)
}

/// Scores `term` against precomputed conjecture term-code sequences.
///
/// # Panics
///
/// Panics under the same conditions as [`lev_compute_term_code`] and
/// [`lev_codes_distance`].
#[must_use]
pub fn lev_term_weight(
    term: &Term,
    conjecture_codes: &[Vec<FunCode>],
    costs: LevDistanceCosts,
) -> f64 {
    let term_code = lev_compute_term_code(term);
    let mut minimum = f64::MAX;
    for conj_code in conjecture_codes {
        minimum = minimum.min(lev_codes_distance(&term_code, conj_code, costs));
    }
    minimum
}

fn c_int_to_uint(value: i32) -> u32 {
    u32::from_ne_bytes(value.to_ne_bytes())
}

fn usize_to_c_uint(value: usize) -> u32 {
    u32::try_from(value).expect("C Levenshtein sequence length fits unsigned int")
}

#[cfg(test)]
mod tests {
    use super::{
        lev_codes_distance, lev_compute_term_code, lev_term_distance, lev_term_weight,
        LevDistanceCosts,
    };
    use crate::terms::functypes::FunCode;
    use crate::terms::signature::{SIG_DB_LAMBDA_CODE, SIG_PHONY_APP_CODE};
    use crate::terms::termtypes::{Term, TP_IS_DB_VAR};

    fn costs(ins_cost: i32, del_cost: i32, ch_cost: i32) -> LevDistanceCosts {
        LevDistanceCosts::new(ins_cost, del_cost, ch_cost)
    }

    fn assert_f64_bits_eq(actual: f64, expected: f64) {
        assert_eq!(actual.to_bits(), expected.to_bits());
    }

    fn binary(code: FunCode, left: &Term, right: &Term) -> Term {
        let term = Term::top_alloc(code, 2);
        term.set_argument(0, left.clone());
        term.set_argument(1, right.clone());
        term
    }

    #[test]
    fn lev_term_code_uses_c_left_to_right_preorder() {
        let a = Term::const_cell_alloc(1);
        let b = Term::const_cell_alloc(2);
        let g = Term::top_alloc(20, 1);
        g.set_argument(0, a);
        let h = Term::top_alloc(21, 1);
        h.set_argument(0, b);
        let root = binary(10, &g, &h);

        assert_eq!(lev_compute_term_code(&root), vec![10, 20, 1, 21, 2]);
    }

    #[test]
    fn lev_term_code_preserves_c_top_level_variable_skips() {
        let applied_free = binary(
            SIG_PHONY_APP_CODE,
            &Term::const_cell_alloc(-2),
            &Term::const_cell_alloc(8),
        );
        assert_eq!(
            lev_compute_term_code(&applied_free),
            vec![SIG_PHONY_APP_CODE]
        );

        let db_head = Term::const_cell_alloc(0);
        db_head.set_prop(TP_IS_DB_VAR);
        let applied_db = binary(SIG_PHONY_APP_CODE, &db_head, &Term::const_cell_alloc(9));
        assert_eq!(
            lev_compute_term_code(&applied_db),
            vec![SIG_PHONY_APP_CODE, 9]
        );

        let lambda = binary(
            SIG_DB_LAMBDA_CODE,
            &Term::const_cell_alloc(0),
            &Term::const_cell_alloc(10),
        );
        assert_eq!(lev_compute_term_code(&lambda), vec![SIG_DB_LAMBDA_CODE, 10]);
    }

    #[test]
    fn lev_codes_distance_matches_c_dynamic_program_shape() {
        assert_f64_bits_eq(
            lev_codes_distance(&[1, 2, 3], &[1, 2, 3], costs(2, 3, 5)),
            0.0,
        );
        assert_f64_bits_eq(
            lev_codes_distance(&[1, 2, 3], &[1, 4, 3], costs(3, 4, 2)),
            2.0,
        );
        assert_f64_bits_eq(lev_codes_distance(&[1, 2], &[1, 2, 3], costs(2, 3, 5)), 3.0);
        assert_f64_bits_eq(lev_codes_distance(&[1, 2, 3], &[1, 3], costs(2, 3, 5)), 2.0);
    }

    #[test]
    fn lev_codes_distance_preserves_unsigned_negative_cost_wrap() {
        assert_f64_bits_eq(
            lev_codes_distance(&[1], &[], costs(1, -1, 1)),
            4_294_967_295.0,
        );
    }

    #[test]
    fn lev_term_distance_uses_extracted_codes() {
        let left = binary(10, &Term::const_cell_alloc(1), &Term::const_cell_alloc(2));
        let right = binary(10, &Term::const_cell_alloc(1), &Term::const_cell_alloc(3));

        assert_f64_bits_eq(lev_term_distance(&left, &right, costs(2, 3, 7)), 5.0);
    }

    #[test]
    fn lev_term_weight_returns_minimum_or_dbl_max() {
        let term = binary(10, &Term::const_cell_alloc(1), &Term::const_cell_alloc(2));
        let exact = lev_compute_term_code(&term);
        let close = vec![10, 1, 3];

        assert_f64_bits_eq(lev_term_weight(&term, &[close], costs(2, 3, 7)), 5.0);
        assert_f64_bits_eq(
            lev_term_weight(&term, &[vec![99], exact], costs(2, 3, 7)),
            0.0,
        );
        assert_f64_bits_eq(lev_term_weight(&term, &[], costs(2, 3, 7)), f64::MAX);
    }
}
