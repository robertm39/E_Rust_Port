use std::cmp::Ordering;

use crate::basics::partial_orderings::CompareResult;
use crate::basics::pdarrays::PDIntArray;
use crate::clauses::eqn::Eqn;
use crate::clauses::eqn_props::PatEqnDirection;
use crate::terms::functypes::FunCode;
use crate::terms::signature::{Signature, DEFAULT_SIGNATURE_SIZE};
use crate::terms::termfunc::term_standard_weight;
use crate::terms::termtypes::Term;
use crate::terms::termvars::{f_code_is_alt_code, DEFAULT_VARBANK_SIZE};

pub const DEFAULT_LITERAL_NO: usize = 8;
pub const PATTERN_SEARCH_BRANCHLIMIT: usize = 3;
pub const NORM_ARITY_LIMIT: i64 = 16_384 / 8;
pub const NORM_SYMBOL_LIMIT: i64 = 65_536 * 8;
pub const NORM_VAR_INIT: i64 = -536_870_912;

#[derive(Clone, Debug)]
pub struct PatternSubst {
    used_idents: PDIntArray,
    fun_subst: PDIntArray,
    used_vars: i64,
    var_subst: PDIntArray,
    backtrack: Vec<FunCode>,
    sig: Signature,
}

impl PatternSubst {
    #[must_use]
    pub fn new(sig: &Signature) -> Self {
        Self {
            used_idents: PDIntArray::new_int(DEFAULT_SIGNATURE_SIZE, DEFAULT_SIGNATURE_SIZE),
            fun_subst: PDIntArray::new_int(DEFAULT_SIGNATURE_SIZE, DEFAULT_SIGNATURE_SIZE),
            used_vars: NORM_VAR_INIT,
            var_subst: PDIntArray::new_int(DEFAULT_VARBANK_SIZE, DEFAULT_VARBANK_SIZE),
            backtrack: Vec::new(),
            sig: sig.clone(),
        }
    }

    #[must_use]
    pub fn default_subst(sig: &Signature) -> Self {
        let mut result = Self::new(sig);
        for f_code in 1..=sig.f_count() {
            if sig.is_special(f_code) {
                result.fun_subst.assign(pd_index(f_code), f_code);
            }
        }
        result
    }

    #[must_use]
    pub const fn used_vars(&self) -> i64 {
        self.used_vars
    }

    #[must_use]
    pub fn backtrack_len(&self) -> usize {
        self.backtrack.len()
    }

    pub fn used_ident_count(&mut self, arity: i32) -> i64 {
        self.used_idents.element_int(pd_index(i64::from(arity)))
    }

    pub fn fun_binding(&mut self, f_code: FunCode) -> FunCode {
        self.fun_subst.element_int(pd_index(f_code))
    }

    /// # Panics
    ///
    /// Panics if `f_code` is not a negative variable f-code.
    pub fn var_binding(&mut self, f_code: FunCode) -> FunCode {
        assert!(f_code < 0, "variable f-code must be negative");
        self.var_subst.element_int(pd_index(-f_code))
    }

    /// # Panics
    ///
    /// Panics if `symbol` is zero or if the normalized per-arity id range is
    /// exhausted.
    pub fn extend(&mut self, symbol: FunCode, arity: usize) -> bool {
        if symbol > 0 {
            let existing = self.fun_subst.element_int(pd_index(symbol));
            if existing == 0 {
                let replacement = self.get_new_fun_symbol(arity);
                self.fun_subst.assign(pd_index(symbol), replacement);
                self.backtrack.push(symbol);
                return true;
            }
        } else {
            assert!(symbol < 0, "pattern symbols must be non-zero");
            if !f_code_is_alt_code(symbol) {
                let index = pd_index(-symbol);
                if self.var_subst.element_int(index) == 0 {
                    self.used_vars -= 1;
                    self.var_subst.assign(index, self.used_vars);
                    self.backtrack.push(symbol);
                    return true;
                }
            }
        }
        false
    }

    #[must_use]
    /// # Panics
    ///
    /// Panics if `f_code` is zero.
    pub fn symbol_value(&mut self, f_code: FunCode) -> FunCode {
        assert_ne!(f_code, 0, "pattern symbols must be non-zero");
        if f_code > 0 {
            if self.sig.is_special(f_code) {
                return f_code;
            }
            self.fun_subst.element_int(pd_index(f_code))
        } else if f_code_is_alt_code(f_code) {
            f_code
        } else {
            self.var_subst.element_int(pd_index(-f_code))
        }
    }

    #[must_use]
    pub fn symbol_is_bound(&mut self, f_code: FunCode) -> bool {
        self.symbol_value(f_code) != 0
    }

    /// # Panics
    ///
    /// Panics if `old_state` is past the current backtrack stack end or if the
    /// substitution state no longer matches the C-style LIFO allocation order.
    pub fn backtrack_to(&mut self, old_state: usize) -> bool {
        assert!(
            old_state <= self.backtrack.len(),
            "cannot backtrack past stack end"
        );
        let mut changed = false;
        while self.backtrack.len() > old_state {
            let symbol = self.backtrack.pop().expect("backtrack stack is non-empty");
            if symbol < 0 {
                let index = pd_index(-symbol);
                let rep_symbol = self.var_subst.element_int(index);
                assert_eq!(self.used_vars, rep_symbol);
                self.used_vars += 1;
                self.var_subst.assign(index, 0);
            } else {
                let rep_symbol = self.fun_subst.element_int(pd_index(symbol));
                let arity = pattern_id_get_arity(rep_symbol);
                let count = pattern_id_get_ident(rep_symbol);
                let used = self.used_idents.element_int(pd_index(arity));
                assert_eq!(used, count);
                self.used_idents.assign(pd_index(arity), used - 1);
                self.fun_subst.assign(pd_index(symbol), 0);
            }
            changed = true;
        }
        changed
    }

    fn comparison_value(&mut self, f_code: FunCode) -> FunCode {
        assert_ne!(f_code, 0, "pattern symbols must be non-zero");
        if f_code > 0 {
            let value = self.fun_subst.element_int(pd_index(f_code));
            if value == f_code {
                i64::from(self.sig.get_alpha_rank(f_code))
            } else {
                value
            }
        } else if f_code_is_alt_code(f_code) {
            f_code
        } else {
            self.var_subst.element_int(pd_index(-f_code))
        }
    }

    fn get_new_fun_symbol(&mut self, arity: usize) -> FunCode {
        let arity = i64::try_from(arity).unwrap_or(i64::MAX);
        let index = pd_index(arity);
        let base = self.used_idents.element_int(index) + 1;
        self.used_idents.assign(index, base);
        assert!(base <= NORM_SYMBOL_LIMIT, "too many pattern symbols");
        pattern_norm_code(base, arity)
    }
}

#[must_use]
pub const fn pattern_norm_code(symbol: FunCode, arity: i64) -> FunCode {
    NORM_SYMBOL_LIMIT * (arity + 1) + symbol
}

#[must_use]
pub const fn pattern_id_get_arity(ident: FunCode) -> i64 {
    ident / NORM_SYMBOL_LIMIT - 1
}

#[must_use]
pub const fn pattern_id_get_ident(ident: FunCode) -> FunCode {
    ident % NORM_SYMBOL_LIMIT
}

#[must_use]
pub const fn pat_id_is_norm_id(symbol: FunCode) -> bool {
    symbol >= NORM_SYMBOL_LIMIT
}

/// # Panics
///
/// Panics if a free variable with arguments is encountered, or if a term
/// reports an arity whose argument slot is absent.
pub fn pattern_term_compute(subst: &mut PatternSubst, term: &Term) -> bool {
    let mut changed = subst.extend(term.f_code(), term.arity());
    for index in 0..term.arity() {
        assert!(
            !term.is_free_var(),
            "free variables with arguments cannot be traversed"
        );
        let arg = required_argument(term, index);
        changed |= pattern_term_compute(subst, &arg);
    }
    changed
}

#[must_use]
/// # Panics
///
/// Panics if the structural size comparison returns an impossible state, or if
/// same-size terms expose mismatched arities or missing arguments.
pub fn pattern_term_compare(
    subst1: &mut PatternSubst,
    left: &Term,
    subst2: &mut PatternSubst,
    right: &Term,
) -> CompareResult {
    let size_result = pat_term_size_compare(left, right);
    assert_ne!(size_result, CompareResult::Uncomparable);
    assert_ne!(size_result, CompareResult::Unknown);
    if size_result != CompareResult::Equal {
        return size_result;
    }

    let mut stack = vec![(left.clone(), right.clone())];
    while let Some((left, right)) = stack.pop() {
        assert_eq!(left.arity(), right.arity());
        let result = pat_symbol_compare(subst1, left.f_code(), subst2, right.f_code());
        if result != CompareResult::Equal {
            return result;
        }
        for index in 0..left.arity() {
            stack.push((
                required_argument(&left, index),
                required_argument(&right, index),
            ));
        }
    }
    CompareResult::Equal
}

pub fn pattern_term_pair_compute(
    subst: &mut PatternSubst,
    eqn: &Eqn,
    direction: PatEqnDirection,
) -> bool {
    let (left, right) = pat_eqn_terms(eqn, direction);
    let changed = pattern_term_compute(subst, left);
    pattern_term_compute(subst, right) || changed
}

#[must_use]
pub fn pattern_term_pair_compare(
    subst1: &mut PatternSubst,
    eqn1: &Eqn,
    dir1: PatEqnDirection,
    subst2: &mut PatternSubst,
    eqn2: &Eqn,
    dir2: PatEqnDirection,
) -> CompareResult {
    let weight_cmp = eqn2.standard_weight() - eqn1.standard_weight();
    if weight_cmp != 0 {
        return q_to_part_i64(weight_cmp);
    }

    let (left1, right1) = pat_eqn_terms(eqn1, dir1);
    let (left2, right2) = pat_eqn_terms(eqn2, dir2);

    let result = pat_term_size_compare(left1, left2);
    if result != CompareResult::Equal {
        return result;
    }
    let result = pat_term_size_compare(right1, right2);
    if result != CompareResult::Equal {
        return result;
    }

    if eqn1.is_positive() && eqn2.is_negative() {
        return CompareResult::Greater;
    }
    if eqn1.is_negative() && eqn2.is_positive() {
        return CompareResult::Lesser;
    }

    let result = pattern_term_compare(subst1, left1, subst2, left2);
    if result != CompareResult::Equal {
        return result;
    }
    pattern_term_compare(subst1, right1, subst2, right2)
}

pub fn pattern_lit_list_compute(
    subst: &mut PatternSubst,
    listrep: &[(&Eqn, PatEqnDirection)],
) -> bool {
    let mut changed = false;
    for (eqn, direction) in listrep {
        changed |= pattern_term_pair_compute(subst, eqn, *direction);
    }
    changed
}

#[must_use]
pub fn pattern_lit_list_compare(
    subst1: &mut PatternSubst,
    listrep1: &[(&Eqn, PatEqnDirection)],
    subst2: &mut PatternSubst,
    listrep2: &[(&Eqn, PatEqnDirection)],
) -> CompareResult {
    let len_cmp = i64::try_from(listrep1.len()).unwrap_or(i64::MAX)
        - i64::try_from(listrep2.len()).unwrap_or(i64::MAX);
    if len_cmp != 0 {
        return q_to_part_i64(len_cmp);
    }

    for ((eqn1, dir1), (eqn2, dir2)) in listrep1.iter().zip(listrep2) {
        let result = pattern_term_pair_compare(subst1, eqn1, *dir1, subst2, eqn2, *dir2);
        if result != CompareResult::Equal {
            return result;
        }
    }
    CompareResult::Equal
}

fn pat_symbol_compare(
    subst1: &mut PatternSubst,
    f1: FunCode,
    subst2: &mut PatternSubst,
    f2: FunCode,
) -> CompareResult {
    if subst1.symbol_is_bound(f1) && subst2.symbol_is_bound(f2) {
        let cmp = subst1.comparison_value(f1) - subst2.comparison_value(f2);
        q_to_part_i64(cmp)
    } else if subst1.symbol_is_bound(f1) {
        CompareResult::Lesser
    } else if subst1.symbol_is_bound(f2) {
        CompareResult::Greater
    } else {
        CompareResult::Uncomparable
    }
}

fn pat_term_size_compare(left: &Term, right: &Term) -> CompareResult {
    if left.f_code() == crate::terms::signature::SIG_TRUE_CODE
        && right.f_code() == crate::terms::signature::SIG_TRUE_CODE
    {
        return CompareResult::Equal;
    }
    if left.f_code() == crate::terms::signature::SIG_TRUE_CODE {
        return CompareResult::Greater;
    }
    if right.f_code() == crate::terms::signature::SIG_TRUE_CODE {
        return CompareResult::Lesser;
    }

    let mut stack = vec![(left.clone(), right.clone())];
    while let Some((left, right)) = stack.pop() {
        if left == right {
            continue;
        }

        let weight_cmp = term_standard_weight(&left) - term_standard_weight(&right);
        if weight_cmp < 0 {
            return CompareResult::Greater;
        }
        if weight_cmp > 0 {
            return CompareResult::Lesser;
        }

        let arity_cmp = i64::try_from(left.arity()).unwrap_or(i64::MAX)
            - i64::try_from(right.arity()).unwrap_or(i64::MAX);
        if arity_cmp < 0 {
            return CompareResult::Greater;
        }
        if arity_cmp > 0 {
            return CompareResult::Lesser;
        }

        for index in 0..left.arity() {
            stack.push((
                required_argument(&left, index),
                required_argument(&right, index),
            ));
        }
    }
    CompareResult::Equal
}

fn pat_eqn_terms(eqn: &Eqn, direction: PatEqnDirection) -> (&Term, &Term) {
    if direction == PatEqnDirection::Normal {
        (eqn.left(), eqn.right())
    } else {
        (eqn.right(), eqn.left())
    }
}

fn required_argument(term: &Term, index: usize) -> Term {
    match term.argument(index) {
        Some(arg) => arg,
        None => panic!("pattern term has no argument at index {index}"),
    }
}

fn q_to_part_i64(value: i64) -> CompareResult {
    match value.cmp(&0) {
        Ordering::Less => CompareResult::Lesser,
        Ordering::Equal => CompareResult::Equal,
        Ordering::Greater => CompareResult::Greater,
    }
}

fn pd_index(value: i64) -> isize {
    isize::try_from(value).unwrap_or(isize::MAX)
}

#[cfg(test)]
mod tests {
    use super::{
        pat_id_is_norm_id, pattern_id_get_arity, pattern_id_get_ident, pattern_lit_list_compare,
        pattern_lit_list_compute, pattern_norm_code, pattern_term_compare, pattern_term_compute,
        pattern_term_pair_compare, pattern_term_pair_compute, PatternSubst, DEFAULT_LITERAL_NO,
        NORM_ARITY_LIMIT, NORM_SYMBOL_LIMIT, NORM_VAR_INIT, PATTERN_SEARCH_BRANCHLIMIT,
    };
    use crate::basics::partial_orderings::CompareResult;
    use crate::clauses::eqn::Eqn;
    use crate::clauses::eqn_props::PatEqnDirection;
    use crate::terms::signature::Signature;
    use crate::terms::termbanks::TermBank;
    use crate::terms::termtypes::{Term, TP_IS_SHARED};
    use crate::terms::typebanks::TypeBank;

    #[test]
    fn constants_and_norm_id_helpers_match_c_macros() {
        assert_eq!(DEFAULT_LITERAL_NO, 8);
        assert_eq!(PATTERN_SEARCH_BRANCHLIMIT, 3);
        assert_eq!(NORM_ARITY_LIMIT, 2048);
        assert_eq!(NORM_SYMBOL_LIMIT, 524_288);
        assert_eq!(NORM_VAR_INIT, -536_870_912);

        let ident = pattern_norm_code(7, 3);
        assert!(pat_id_is_norm_id(ident));
        assert_eq!(pattern_id_get_arity(ident), 3);
        assert_eq!(pattern_id_get_ident(ident), 7);
    }

    #[test]
    fn default_subst_binds_special_symbols_to_themselves() {
        let mut sig = Signature::new(TypeBank::new());
        let special = sig.insert_id("special", 0, true);
        let ordinary = sig.insert_id("ordinary", 0, false);
        let mut subst = PatternSubst::default_subst(&sig);

        assert_eq!(subst.symbol_value(special), special);
        assert!(subst.symbol_is_bound(special));
        assert_eq!(subst.symbol_value(ordinary), 0);
        assert!(!subst.symbol_is_bound(ordinary));
    }

    #[test]
    fn extension_and_backtracking_follow_c_stacks() {
        let mut sig = Signature::new(TypeBank::new());
        let f = sig.insert_id("f", 2, false);
        let mut subst = PatternSubst::new(&sig);

        assert!(subst.extend(f, 2));
        assert_eq!(subst.fun_binding(f), pattern_norm_code(1, 2));
        assert_eq!(subst.used_ident_count(2), 1);
        assert_eq!(subst.backtrack_len(), 1);
        assert!(!subst.extend(f, 2));

        let old_state = subst.backtrack_len();
        assert!(subst.extend(-2, 0));
        assert_eq!(subst.var_binding(-2), NORM_VAR_INIT - 1);
        assert_eq!(subst.used_vars(), NORM_VAR_INIT - 1);
        assert!(!subst.extend(-1, 0));
        assert!(subst.backtrack_to(old_state));
        assert_eq!(subst.var_binding(-2), 0);
        assert_eq!(subst.used_vars(), NORM_VAR_INIT);
        assert!(subst.backtrack_to(0));
        assert_eq!(subst.fun_binding(f), 0);
        assert_eq!(subst.used_ident_count(2), 0);
    }

    #[test]
    fn term_compute_binds_function_symbols_and_normal_variables() {
        let mut sig = Signature::new(TypeBank::new());
        let f = sig.insert_id("f", 2, false);
        let a = sig.insert_id("a", 0, false);
        let term = fun(f, &[constant(a), variable(-2)]);
        let mut subst = PatternSubst::new(&sig);

        assert!(pattern_term_compute(&mut subst, &term));

        assert_eq!(subst.fun_binding(f), pattern_norm_code(1, 2));
        assert_eq!(subst.fun_binding(a), pattern_norm_code(1, 0));
        assert_eq!(subst.var_binding(-2), NORM_VAR_INIT - 1);
    }

    #[test]
    fn term_compare_uses_size_then_pattern_symbol_order() {
        let mut sig = Signature::new(TypeBank::new());
        let f = sig.insert_id("f", 1, false);
        let g = sig.insert_id("g", 1, false);
        let a = sig.insert_id("a", 0, false);
        let left = fun(f, &[constant(a)]);
        let right = fun(g, &[constant(a)]);
        let mut left_subst = PatternSubst::new(&sig);
        let mut right_subst = PatternSubst::new(&sig);
        pattern_term_compute(&mut left_subst, &left);
        pattern_term_compute(&mut right_subst, &right);

        assert_eq!(
            pattern_term_compare(&mut left_subst, &left, &mut right_subst, &right),
            CompareResult::Equal
        );

        let larger = fun(f, &[fun(g, &[constant(a)])]);
        assert_eq!(
            pattern_term_compare(&mut left_subst, &left, &mut right_subst, &larger),
            CompareResult::Greater
        );
    }

    #[test]
    fn term_pair_and_list_compute_respect_directions() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let eqn = Eqn::alloc(a.clone(), b.clone(), &mut bank, true).unwrap();
        let mut subst = PatternSubst::new(bank.signature());

        assert!(pattern_term_pair_compute(
            &mut subst,
            &eqn,
            PatEqnDirection::Reverse
        ));
        assert_eq!(subst.fun_binding(b.f_code()), pattern_norm_code(1, 0));
        assert_eq!(subst.fun_binding(a.f_code()), pattern_norm_code(2, 0));

        let old_state = subst.backtrack_len();
        assert!(!pattern_lit_list_compute(
            &mut subst,
            &[(&eqn, PatEqnDirection::Reverse)]
        ));
        assert_eq!(subst.backtrack_len(), old_state);
    }

    #[test]
    fn term_pair_and_list_compare_match_c_ordering_shape() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let c = typed_const(&mut bank, "c");
        let positive = Eqn::alloc(a.clone(), b.clone(), &mut bank, true).unwrap();
        let negative = Eqn::alloc(a, c, &mut bank, false).unwrap();
        let mut left_subst = PatternSubst::new(bank.signature());
        let mut right_subst = PatternSubst::new(bank.signature());
        pattern_term_pair_compute(&mut left_subst, &positive, PatEqnDirection::Normal);
        pattern_term_pair_compute(&mut right_subst, &negative, PatEqnDirection::Normal);

        assert_eq!(
            pattern_term_pair_compare(
                &mut left_subst,
                &positive,
                PatEqnDirection::Normal,
                &mut right_subst,
                &negative,
                PatEqnDirection::Normal,
            ),
            CompareResult::Greater
        );
        assert_eq!(
            pattern_lit_list_compare(
                &mut left_subst,
                &[(&positive, PatEqnDirection::Normal)],
                &mut right_subst,
                &[
                    (&positive, PatEqnDirection::Normal),
                    (&negative, PatEqnDirection::Normal),
                ],
            ),
            CompareResult::Lesser
        );
    }

    fn test_bank() -> TermBank {
        TermBank::new(Signature::new(TypeBank::new())).expect("term bank allocation")
    }

    fn typed_const(bank: &mut TermBank, name: &str) -> Term {
        let f_code = bank.signature_mut().insert_id(name, 0, false);
        let type_ = bank.signature().type_bank().i_type();
        bank.signature_mut()
            .declare_type(f_code, type_)
            .expect("constant type declaration");
        bank.create_const_term(f_code).expect("constant insertion")
    }

    fn constant(f_code: i64) -> Term {
        let term = Term::const_cell_alloc(f_code);
        term.set_prop(TP_IS_SHARED);
        term
    }

    fn variable(f_code: i64) -> Term {
        let term = Term::const_cell_alloc(f_code);
        term.set_prop(TP_IS_SHARED);
        term
    }

    fn fun(f_code: i64, args: &[Term]) -> Term {
        let term = Term::top_alloc(f_code, args.len());
        term.set_prop(TP_IS_SHARED);
        for (index, arg) in args.iter().enumerate() {
            term.set_argument(index, arg.clone());
        }
        term
    }
}
