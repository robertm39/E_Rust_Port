use std::cmp::Ordering;
use std::fmt;

use crate::basics::partial_orderings::CompareResult;
use crate::basics::pdarrays::PDIntArray;
use crate::clauses::clause::Clause;
use crate::clauses::eqn::Eqn;
use crate::clauses::eqn_props::PatEqnDirection;
use crate::terms::functypes::FunCode;
use crate::terms::signature::{Signature, DEFAULT_SIGNATURE_SIZE};
use crate::terms::termfunc::{term_copy, term_standard_weight};
use crate::terms::termtypes::{DerefType, Term};
use crate::terms::termvars::{f_code_is_alt_code, VarBank, DEFAULT_VARBANK_SIZE};

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

#[derive(Clone, Debug)]
pub struct PatternClauseResult<'a> {
    tries: i64,
    subst: PatternSubst,
    listrep: Vec<(&'a Eqn, PatEqnDirection)>,
}

impl<'a> PatternClauseResult<'a> {
    #[must_use]
    pub const fn tries(&self) -> i64 {
        self.tries
    }

    #[must_use]
    pub const fn subst(&self) -> &PatternSubst {
        &self.subst
    }

    #[must_use]
    pub fn subst_mut(&mut self) -> &mut PatternSubst {
        &mut self.subst
    }

    #[must_use]
    pub fn listrep(&self) -> &[(&'a Eqn, PatEqnDirection)] {
        &self.listrep
    }

    #[must_use]
    pub fn into_parts(self) -> (i64, PatternSubst, Vec<(&'a Eqn, PatEqnDirection)>) {
        (self.tries, self.subst, self.listrep)
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct LiteralMinimal {
    left: bool,
    right: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PatternSide {
    Left,
    Right,
}

#[derive(Clone, Debug)]
struct PatternSearchFrame {
    old_backtrack: usize,
    choices: Vec<(usize, PatEqnDirection)>,
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
    pub fn symbol_value(&self, f_code: FunCode) -> FunCode {
        assert_ne!(f_code, 0, "pattern symbols must be non-zero");
        if f_code > 0 {
            if self.sig.is_special(f_code) {
                return f_code;
            }
            subst_array_value(&self.fun_subst, pd_index(f_code))
        } else if f_code_is_alt_code(f_code) {
            f_code
        } else {
            subst_array_value(&self.var_subst, pd_index(-f_code))
        }
    }

    #[must_use]
    pub fn symbol_is_bound(&self, f_code: FunCode) -> bool {
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

    #[must_use]
    pub fn original_symbol(&self, f_code: FunCode) -> FunCode {
        if f_code > 0 {
            for index in 0..self.fun_subst.size() {
                if subst_array_value(&self.fun_subst, index_to_pd(index)) == f_code {
                    return usize_to_f_code(index);
                }
            }
            return 0;
        }

        if f_code_is_alt_code(f_code) {
            return f_code;
        }

        for index in 0..self.fun_subst.size() {
            if subst_array_value(&self.var_subst, index_to_pd(index)) == f_code {
                return usize_to_f_code(index);
            }
        }
        0
    }

    fn comparison_value(&mut self, f_code: FunCode) -> FunCode {
        assert_ne!(f_code, 0, "pattern symbols must be non-zero");
        if f_code > 0 {
            let value = subst_array_value(&self.fun_subst, pd_index(f_code));
            if value == f_code {
                i64::from(self.sig.get_alpha_rank(f_code))
            } else {
                value
            }
        } else if f_code_is_alt_code(f_code) {
            f_code
        } else {
            subst_array_value(&self.var_subst, pd_index(-f_code))
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
/// Panics if `f_code` is not a normalized pattern id.
#[must_use]
pub fn pattern_print_rep(f_code: FunCode) -> String {
    assert!(pat_id_is_norm_id(f_code));
    format!(
        "f{}_{}",
        pattern_id_get_arity(f_code),
        pattern_id_get_ident(f_code)
    )
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

/// # Panics
///
/// Panics if a term contains f-code zero, if a non-constant term has a missing
/// argument, or if a normalized non-variable id is malformed.
#[must_use]
pub fn pattern_term_print_string(subst: &mut PatternSubst, term: &Term, sig: &Signature) -> String {
    let mut output = String::new();
    let _ = pattern_term_write(&mut output, subst, term, sig);
    output
}

/// # Panics
///
/// Panics under the same conditions as `pattern_term_print_string`; additionally
/// panics if the literal shape and equational property bit disagree.
#[must_use]
pub fn pattern_eqn_print_string(
    subst: &mut PatternSubst,
    eqn: &Eqn,
    direction: PatEqnDirection,
    bank: &crate::terms::termbanks::TermBank,
) -> String {
    let mut output = String::new();
    let _ = pattern_eqn_write(&mut output, subst, eqn, direction, bank);
    output
}

/// # Panics
///
/// Panics under the same conditions as `pattern_eqn_print_string`.
#[must_use]
pub fn pattern_clause_print_string(
    subst: &mut PatternSubst,
    listrep: &[(&Eqn, PatEqnDirection)],
    bank: &crate::terms::termbanks::TermBank,
) -> String {
    let mut output = String::new();
    let mut prefix = "";
    for (eqn, direction) in listrep {
        output.push_str(prefix);
        let _ = pattern_eqn_write(&mut output, subst, eqn, *direction, bank);
        prefix = ";";
    }
    output.push_str(" <- .");
    output
}

#[must_use]
pub fn debug_pattern_clause_to_list(clause: &Clause) -> Vec<(&Eqn, PatEqnDirection)> {
    clause
        .literals()
        .as_slice()
        .iter()
        .map(|eqn| (eqn, PatEqnDirection::Normal))
        .collect()
}

/// # Panics
///
/// Panics under the same internal-invariant conditions as the term, equation,
/// and list pattern comparison helpers.
#[must_use]
pub fn pattern_clause_compute(clause: &Clause, subst: PatternSubst) -> PatternClauseResult<'_> {
    let literals = clause.literals().as_slice();
    let (tries, subst, order) = lit_list_rep_pattern(literals, subst);
    PatternClauseResult {
        tries,
        subst,
        listrep: order_to_listrep(literals, &order),
    }
}

/// # Panics
///
/// Panics if traversed variables have no type, if variable-bank invariants are
/// violated, if source argument slots are missing, or if normalized ids are
/// malformed.
#[must_use]
pub fn pattern_translate_sig(
    term: &Term,
    subst: &mut PatternSubst,
    old_sig: &Signature,
    new_sig: &mut Signature,
    new_vars: &VarBank,
) -> Term {
    let mut stack = vec![term.clone()];
    while let Some(current) = stack.pop() {
        if current.is_free_var() {
            let f_code = subst.symbol_value(current.f_code());
            if f_code != 0 {
                let type_ = current
                    .type_()
                    .expect("translated variables must have types");
                current.set_binding(Some(
                    new_vars.var_assert_alloc(f_code - NORM_VAR_INIT, &type_),
                ));
            }
        } else {
            push_arguments(&current, &mut stack);
        }
    }

    let copy = term_copy(term, new_vars, None, DerefType::Once);

    let mut stack = vec![term.clone()];
    while let Some(current) = stack.pop() {
        if current.is_free_var() {
            current.set_binding(None);
        } else {
            push_arguments(&current, &mut stack);
        }
    }

    let mut stack = vec![copy.clone()];
    while let Some(current) = stack.pop() {
        if current.is_free_var() {
            continue;
        }

        let f_code = subst.symbol_value(current.f_code());
        let new_name = if pat_id_is_norm_id(f_code) {
            pattern_print_rep(f_code)
        } else {
            old_sig
                .find_name(current.f_code())
                .expect("translated function symbol must have a source name")
                .to_string()
        };
        let arity = i32::try_from(current.arity()).unwrap_or(i32::MAX);
        let new_code = new_sig.insert_id(&new_name, arity, false);
        assert_ne!(new_code, 0, "signature insertion must return a valid code");
        current.set_f_code(new_code);
        push_arguments(&current, &mut stack);
    }
    copy
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

fn pat_symbol_compare_same_subst(
    subst: &mut PatternSubst,
    f1: FunCode,
    f2: FunCode,
) -> CompareResult {
    let f1_bound = subst.symbol_is_bound(f1);
    let f2_bound = subst.symbol_is_bound(f2);
    if f1_bound && f2_bound {
        let f1_value = subst.comparison_value(f1);
        let f2_value = subst.comparison_value(f2);
        q_to_part_i64(f1_value - f2_value)
    } else if f1_bound {
        CompareResult::Lesser
    } else if f2_bound {
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

fn pattern_term_compare_same_subst(
    subst: &mut PatternSubst,
    left: &Term,
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
        let result = pat_symbol_compare_same_subst(subst, left.f_code(), right.f_code());
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

fn pattern_term_pair_compare_same_subst(
    subst: &mut PatternSubst,
    eqn1: &Eqn,
    dir1: PatEqnDirection,
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

    let result = pattern_term_compare_same_subst(subst, left1, left2);
    if result != CompareResult::Equal {
        return result;
    }
    pattern_term_compare_same_subst(subst, right1, right2)
}

fn initialize_lit_list(
    subst: &mut PatternSubst,
    literals: &[Eqn],
    used: &[bool],
) -> Vec<LiteralMinimal> {
    let mut minimal = vec![LiteralMinimal::default(); literals.len()];
    for (index, literal) in literals.iter().enumerate() {
        if used[index] {
            continue;
        }
        match pattern_term_compare_same_subst(subst, literal.left(), literal.right()) {
            CompareResult::Equal | CompareResult::Lesser => minimal[index].left = true,
            CompareResult::Greater => minimal[index].right = true,
            CompareResult::Uncomparable => {
                minimal[index].left = true;
                minimal[index].right = true;
            }
            CompareResult::Unknown
            | CompareResult::NotGreaterEqual
            | CompareResult::NotLessEqual => {
                panic!("pattern term comparison returned non-C result")
            }
        }
    }
    minimal
}

fn mark_minimal_literals(
    subst: &mut PatternSubst,
    literals: &[Eqn],
    used: &[bool],
) -> Vec<LiteralMinimal> {
    let mut minimal = initialize_lit_list(subst, literals, used);

    for handle in 0..literals.len() {
        if used[handle] || !(minimal[handle].left || minimal[handle].right) {
            continue;
        }
        for compare in handle + 1..literals.len() {
            if used[compare] {
                continue;
            }
            if minimal[handle].left {
                if minimal[compare].left {
                    let cmpres = pattern_term_pair_compare_same_subst(
                        subst,
                        &literals[handle],
                        PatEqnDirection::Normal,
                        &literals[compare],
                        PatEqnDirection::Normal,
                    );
                    apply_minimal_compare(
                        &mut minimal,
                        cmpres,
                        (handle, PatternSide::Left),
                        (compare, PatternSide::Left),
                    );
                }
                if minimal[compare].right {
                    let cmpres = pattern_term_pair_compare_same_subst(
                        subst,
                        &literals[handle],
                        PatEqnDirection::Normal,
                        &literals[compare],
                        PatEqnDirection::Reverse,
                    );
                    apply_minimal_compare(
                        &mut minimal,
                        cmpres,
                        (handle, PatternSide::Left),
                        (compare, PatternSide::Right),
                    );
                }
            }
            if minimal[handle].right {
                if minimal[compare].left {
                    let cmpres = pattern_term_pair_compare_same_subst(
                        subst,
                        &literals[handle],
                        PatEqnDirection::Reverse,
                        &literals[compare],
                        PatEqnDirection::Normal,
                    );
                    apply_minimal_compare(
                        &mut minimal,
                        cmpres,
                        (handle, PatternSide::Right),
                        (compare, PatternSide::Left),
                    );
                }
                if minimal[compare].right {
                    let cmpres = pattern_term_pair_compare_same_subst(
                        subst,
                        &literals[handle],
                        PatEqnDirection::Reverse,
                        &literals[compare],
                        PatEqnDirection::Reverse,
                    );
                    apply_minimal_compare(
                        &mut minimal,
                        cmpres,
                        (handle, PatternSide::Right),
                        (compare, PatternSide::Right),
                    );
                }
            }
        }
    }
    minimal
}

fn apply_minimal_compare(
    minimal: &mut [LiteralMinimal],
    cmpres: CompareResult,
    first: (usize, PatternSide),
    second: (usize, PatternSide),
) {
    match cmpres {
        CompareResult::Equal | CompareResult::Lesser => clear_minimal(minimal, second),
        CompareResult::Greater => clear_minimal(minimal, first),
        CompareResult::Uncomparable => {}
        CompareResult::Unknown | CompareResult::NotGreaterEqual | CompareResult::NotLessEqual => {
            panic!("pattern literal comparison returned non-C result")
        }
    }
}

fn clear_minimal(minimal: &mut [LiteralMinimal], target: (usize, PatternSide)) {
    match target.1 {
        PatternSide::Left => minimal[target.0].left = false,
        PatternSide::Right => minimal[target.0].right = false,
    }
}

fn collect_choices(
    subst: &mut PatternSubst,
    literals: &[Eqn],
    used: &[bool],
) -> Vec<(usize, PatEqnDirection)> {
    let minimal = mark_minimal_literals(subst, literals, used);
    let mut choices = Vec::new();
    for (index, marks) in minimal.into_iter().enumerate() {
        if used[index] {
            continue;
        }
        if marks.left {
            choices.push((index, PatEqnDirection::Normal));
        }
        if marks.right {
            choices.push((index, PatEqnDirection::Reverse));
        }
    }
    choices
}

fn complete_state(
    subst: &mut PatternSubst,
    literals: &[Eqn],
    used: &mut [bool],
    order: &mut Vec<(usize, PatEqnDirection)>,
    state: &mut Vec<PatternSearchFrame>,
) -> bool {
    let mut choices = collect_choices(subst, literals, used);
    let mut choice_nr = choices.len();

    while choice_nr != 0 && choice_nr <= PATTERN_SEARCH_BRANCHLIMIT {
        let (picked, direction) = choices.pop().expect("choice stack is non-empty");

        state.push(PatternSearchFrame {
            old_backtrack: subst.backtrack_len(),
            choices,
        });
        used[picked] = true;
        pattern_term_pair_compute(subst, &literals[picked], direction);
        order.push((picked, direction));

        choices = collect_choices(subst, literals, used);
        choice_nr = choices.len();
    }

    choice_nr == 0
}

fn lit_list_rep_pattern(
    literals: &[Eqn],
    mut subst: PatternSubst,
) -> (i64, PatternSubst, Vec<(usize, PatEqnDirection)>) {
    let mut used = vec![false; literals.len()];
    let mut order = Vec::new();
    let mut state = Vec::new();
    let mut count = 1;
    let mut affordable = complete_state(&mut subst, literals, &mut used, &mut order, &mut state);
    let mut best_subst = subst.clone();
    let mut best_order = order.clone();

    while !state.is_empty() && affordable {
        let (picked, _direction) = order.pop().expect("state implies a picked literal");
        used[picked] = false;

        let frame = state.last_mut().expect("state is non-empty");
        subst.backtrack_to(frame.old_backtrack);
        if let Some((picked, direction)) = frame.choices.pop() {
            count += 1;
            used[picked] = true;
            pattern_term_pair_compute(&mut subst, &literals[picked], direction);
            order.push((picked, direction));
            affordable = complete_state(&mut subst, literals, &mut used, &mut order, &mut state);

            if affordable
                && pattern_lit_order_compare(
                    &mut subst,
                    literals,
                    &order,
                    &mut best_subst,
                    &best_order,
                ) == CompareResult::Lesser
            {
                best_subst = subst.clone();
                best_order = order.clone();
            }
        } else {
            state.pop();
        }
    }

    (if affordable { count } else { 0 }, best_subst, best_order)
}

fn pattern_lit_order_compare(
    subst1: &mut PatternSubst,
    literals: &[Eqn],
    order1: &[(usize, PatEqnDirection)],
    subst2: &mut PatternSubst,
    order2: &[(usize, PatEqnDirection)],
) -> CompareResult {
    let len_cmp = i64::try_from(order1.len()).unwrap_or(i64::MAX)
        - i64::try_from(order2.len()).unwrap_or(i64::MAX);
    if len_cmp != 0 {
        return q_to_part_i64(len_cmp);
    }

    for ((left_index, left_dir), (right_index, right_dir)) in order1.iter().zip(order2) {
        let result = pattern_term_pair_compare(
            subst1,
            &literals[*left_index],
            *left_dir,
            subst2,
            &literals[*right_index],
            *right_dir,
        );
        if result != CompareResult::Equal {
            return result;
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

fn pattern_term_write(
    output: &mut impl fmt::Write,
    subst: &mut PatternSubst,
    term: &Term,
    sig: &Signature,
) -> fmt::Result {
    let id = subst.symbol_value(term.f_code());
    if term.is_free_var() {
        if id == 0 {
            write!(output, "X{}", -term.f_code())
        } else {
            write!(output, "Xn{}", -(id - NORM_VAR_INIT))
        }
    } else {
        if pat_id_is_norm_id(id) {
            output.write_str(&pattern_print_rep(id))?;
        } else {
            output.write_str(sig.find_name(term.f_code()).unwrap_or("<unknown>"))?;
        }
        if term.arity() != 0 {
            output.write_char('(')?;
            pattern_term_write(output, subst, &required_argument(term, 0), sig)?;
            for index in 1..term.arity() {
                output.write_char(',')?;
                pattern_term_write(output, subst, &required_argument(term, index), sig)?;
            }
            output.write_char(')')?;
        }
        Ok(())
    }
}

fn pattern_eqn_write(
    output: &mut impl fmt::Write,
    subst: &mut PatternSubst,
    eqn: &Eqn,
    direction: PatEqnDirection,
    bank: &crate::terms::termbanks::TermBank,
) -> fmt::Result {
    if eqn.is_equ_lit(bank) {
        let (left, right) = pat_eqn_terms(eqn, direction);
        pattern_term_write(output, subst, left, bank.signature())?;
        if eqn.is_positive() {
            output.write_char('=')?;
        } else {
            output.write_str("!=")?;
        }
        pattern_term_write(output, subst, right, bank.signature())
    } else {
        if eqn.is_negative() {
            output.write_char('~')?;
        }
        pattern_term_write(output, subst, eqn.left(), bank.signature())
    }
}

fn push_arguments(term: &Term, stack: &mut Vec<Term>) {
    for index in 0..term.arity() {
        stack.push(required_argument(term, index));
    }
}

fn order_to_listrep<'a>(
    literals: &'a [Eqn],
    order: &[(usize, PatEqnDirection)],
) -> Vec<(&'a Eqn, PatEqnDirection)> {
    order
        .iter()
        .map(|(index, direction)| (&literals[*index], *direction))
        .collect()
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

fn subst_array_value(array: &PDIntArray, index: isize) -> i64 {
    array.existing_element(index).copied().unwrap_or(0)
}

fn index_to_pd(index: usize) -> isize {
    isize::try_from(index).unwrap_or(isize::MAX)
}

fn usize_to_f_code(index: usize) -> FunCode {
    FunCode::try_from(index).unwrap_or(FunCode::MAX)
}

#[cfg(test)]
mod tests {
    use super::{
        debug_pattern_clause_to_list, pat_id_is_norm_id, pattern_clause_compute,
        pattern_clause_print_string, pattern_eqn_print_string, pattern_id_get_arity,
        pattern_id_get_ident, pattern_lit_list_compare, pattern_lit_list_compute,
        pattern_norm_code, pattern_print_rep, pattern_term_compare, pattern_term_compute,
        pattern_term_pair_compare, pattern_term_pair_compute, pattern_term_print_string,
        pattern_translate_sig, PatternSubst, DEFAULT_LITERAL_NO, NORM_ARITY_LIMIT,
        NORM_SYMBOL_LIMIT, NORM_VAR_INIT, PATTERN_SEARCH_BRANCHLIMIT,
    };
    use crate::basics::partial_orderings::CompareResult;
    use crate::clauses::clause::Clause;
    use crate::clauses::eqn::Eqn;
    use crate::clauses::eqn_props::PatEqnDirection;
    use crate::clauses::eqnlist::EqnList;
    use crate::terms::signature::{Signature, DEFAULT_SIGNATURE_SIZE};
    use crate::terms::simpletypes::{alloc_arrow_type, Type};
    use crate::terms::termbanks::TermBank;
    use crate::terms::termtypes::{Term, DEFAULT_FWEIGHT, DEFAULT_VWEIGHT, TP_IS_SHARED};
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
        assert_eq!(pattern_print_rep(ident), "f3_7");
    }

    #[test]
    fn default_subst_binds_special_symbols_to_themselves() {
        let mut sig = Signature::new(TypeBank::new());
        let special = sig.insert_id("special", 0, true);
        let ordinary = sig.insert_id("ordinary", 0, false);
        let subst = PatternSubst::default_subst(&sig);

        assert_eq!(subst.symbol_value(special), special);
        assert!(subst.symbol_is_bound(special));
        assert_eq!(subst.symbol_value(ordinary), 0);
        assert!(!subst.symbol_is_bound(ordinary));
    }

    #[test]
    fn symbol_lookup_does_not_grow_substitution_arrays() {
        let mut sig = Signature::new(TypeBank::new());
        let mut outside = 0;
        for index in 0..=DEFAULT_SIGNATURE_SIZE {
            outside = sig.insert_id(&format!("outside{index}"), 0, false);
        }
        let subst = PatternSubst::new(&sig);
        let original_size = subst.fun_subst.size();
        assert!(usize::try_from(outside).is_ok_and(|index| index >= original_size));

        assert_eq!(subst.symbol_value(outside), 0);
        assert_eq!(subst.fun_subst.size(), original_size);
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
    fn original_symbol_lookup_preserves_c_scan_and_variable_index_quirk() {
        let mut sig = Signature::new(TypeBank::new());
        let f = sig.insert_id("f", 2, false);
        let mut subst = PatternSubst::new(&sig);

        assert!(subst.extend(f, 2));
        let f_rep = subst.fun_binding(f);
        assert_eq!(subst.original_symbol(f_rep), f);
        assert_eq!(subst.original_symbol(pattern_norm_code(99, 2)), 0);

        assert!(subst.extend(-2, 0));
        assert!(subst.extend(-4, 0));
        let odd_var_rep = subst.var_binding(-2);
        let even_var_rep = subst.var_binding(-4);
        assert_eq!(subst.original_symbol(odd_var_rep), odd_var_rep);
        assert_eq!(subst.original_symbol(even_var_rep), 4);
        assert_eq!(subst.original_symbol(-1), -1);

        let mut copy = subst.clone();
        assert!(subst.backtrack_to(0));
        assert_eq!(copy.fun_binding(f), f_rep);
        assert_eq!(copy.original_symbol(even_var_rep), 4);
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
    fn term_printing_uses_pattern_specific_variable_and_norm_names() {
        let mut sig = Signature::new(TypeBank::new());
        let f = sig.insert_id("f", 2, false);
        let a = sig.insert_id("a", 0, false);
        let term = fun(f, &[constant(a), variable(-2)]);
        let mut subst = PatternSubst::new(&sig);

        assert_eq!(
            pattern_term_print_string(&mut subst, &term, &sig),
            "f(a,X2)"
        );

        assert!(pattern_term_compute(&mut subst, &term));
        assert_eq!(
            pattern_term_print_string(&mut subst, &term, &sig),
            "f2_1(f0_1,Xn1)"
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

    #[test]
    fn equation_clause_printing_and_debug_list_match_c_surface() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let eqn = Eqn::alloc(a, b, &mut bank, true).unwrap();
        let mut subst = PatternSubst::new(bank.signature());

        assert!(pattern_term_pair_compute(
            &mut subst,
            &eqn,
            PatEqnDirection::Normal
        ));

        assert_eq!(
            pattern_eqn_print_string(&mut subst, &eqn, PatEqnDirection::Normal, &bank),
            "f0_1=f0_2"
        );
        assert_eq!(
            pattern_eqn_print_string(&mut subst, &eqn, PatEqnDirection::Reverse, &bank),
            "f0_2=f0_1"
        );
        assert_eq!(
            pattern_clause_print_string(&mut subst, &[(&eqn, PatEqnDirection::Normal)], &bank),
            "f0_1=f0_2 <- ."
        );

        let clause = Clause::alloc(EqnList::from_vec(vec![eqn]));
        let debug = debug_pattern_clause_to_list(&clause);
        assert_eq!(debug.len(), 1);
        assert_eq!(debug[0].1, PatEqnDirection::Normal);
    }

    #[test]
    fn clause_compute_uses_c_stack_choice_order_for_single_literal() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let eqn = Eqn::alloc(a, b, &mut bank, true).unwrap();
        let clause = Clause::alloc(EqnList::from_vec(vec![eqn]));
        let result = pattern_clause_compute(&clause, PatternSubst::new(bank.signature()));

        assert_eq!(result.tries(), 2);
        assert_eq!(result.listrep().len(), 1);
        assert_eq!(result.listrep()[0].1, PatEqnDirection::Reverse);

        let mut subst = result.subst().clone();
        assert_eq!(
            pattern_clause_print_string(&mut subst, result.listrep(), &bank),
            "f0_1=f0_2 <- ."
        );
    }

    #[test]
    fn clause_compute_returns_zero_when_initial_choices_exceed_branch_limit() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let c = typed_const(&mut bank, "c");
        let d = typed_const(&mut bank, "d");
        let first = Eqn::alloc(a, b, &mut bank, true).unwrap();
        let second = Eqn::alloc(c, d, &mut bank, true).unwrap();
        let clause = Clause::alloc(EqnList::from_vec(vec![first, second]));

        let result = pattern_clause_compute(&clause, PatternSubst::new(bank.signature()));

        assert_eq!(result.tries(), 0);
        assert!(result.listrep().is_empty());
    }

    #[test]
    fn translate_sig_maps_norm_symbols_and_clobbers_source_variable_bindings_like_c() {
        let mut old_sig = Signature::new(TypeBank::new());
        let type_ = old_sig.type_bank().i_type();
        let f = old_sig.insert_id("f", 2, false);
        old_sig
            .declare_final_type(
                f,
                alloc_arrow_type(vec![type_.clone(), type_.clone(), type_.clone()]),
            )
            .unwrap();
        let a_code = old_sig.insert_id("a", 0, false);
        old_sig.declare_final_type(a_code, type_.clone()).unwrap();

        let old_vars = crate::terms::termvars::VarBank::new(old_sig.type_bank());
        let var = old_vars.var_assert_alloc(-2, &type_);
        let a = typed_const_cell(a_code, &type_);
        let term = typed_fun_cell(f, &[a, var.clone()], &type_);
        let mut subst = PatternSubst::new(&old_sig);
        assert!(pattern_term_compute(&mut subst, &term));

        let stale_binding = typed_const_cell(a_code, &type_);
        var.set_binding(Some(stale_binding));
        let mut new_sig = Signature::new(TypeBank::new());
        let new_vars = crate::terms::termvars::VarBank::new(new_sig.type_bank());

        let translated =
            pattern_translate_sig(&term, &mut subst, &old_sig, &mut new_sig, &new_vars);

        assert!(var.binding().is_none());
        assert_eq!(new_sig.find_name(translated.f_code()), Some("f2_1"));
        let translated_const = translated.argument(0).unwrap();
        assert_eq!(new_sig.find_name(translated_const.f_code()), Some("f0_1"));
        let translated_var = translated.argument(1).unwrap();
        assert_eq!(translated_var.f_code(), -1);
        assert_eq!(new_vars.f_code_find(-1), Some(translated_var));
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

    fn typed_const_cell(f_code: i64, type_: &Type) -> Term {
        let term = Term::const_cell_alloc(f_code);
        term.set_type(Some(type_.clone()));
        term
    }

    fn typed_fun_cell(f_code: i64, args: &[Term], type_: &Type) -> Term {
        let term = Term::top_alloc(f_code, args.len());
        term.set_type(Some(type_.clone()));
        for (index, arg) in args.iter().enumerate() {
            term.set_argument(index, arg.clone());
        }
        term
    }

    fn constant(f_code: i64) -> Term {
        let term = Term::const_cell_alloc(f_code);
        term.set_f_count(1);
        term.set_weight(DEFAULT_FWEIGHT);
        term.set_prop(TP_IS_SHARED);
        term
    }

    fn variable(f_code: i64) -> Term {
        let term = Term::const_cell_alloc(f_code);
        term.set_v_count(1);
        term.set_weight(DEFAULT_VWEIGHT);
        term.set_prop(TP_IS_SHARED);
        term
    }

    fn fun(f_code: i64, args: &[Term]) -> Term {
        let term = Term::top_alloc(f_code, args.len());
        for (index, arg) in args.iter().enumerate() {
            term.set_argument(index, arg.clone());
        }
        let v_count = args.iter().map(Term::v_count).sum();
        let f_count = 1 + args.iter().map(Term::f_count).sum::<u32>();
        term.set_v_count(v_count);
        term.set_f_count(f_count);
        term.set_weight(
            i64::from(v_count) * DEFAULT_VWEIGHT + i64::from(f_count) * DEFAULT_FWEIGHT,
        );
        term.set_prop(TP_IS_SHARED);
        term
    }
}
