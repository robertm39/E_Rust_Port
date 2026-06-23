use crate::basics::error::Diagnostic;
use crate::basics::{pdarrays::PDIntArray, pstacks::PStack};
use crate::clauses::eqn::Eqn;
use crate::clauses::eqn_props::{EqnProperties, EP_IS_POSITIVE};
use crate::terms::functypes::FunCode;
use crate::terms::subst::Substitution;
use crate::terms::termbanks::TermBank;
use crate::terms::termtypes::{DerefType, Term, TermProperties};
use crate::terms::termvars::VarBank;
use std::collections::{BTreeMap, BTreeSet};

pub const EQN_LIST_LONG_LIMIT: usize = 15;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct EqnList {
    literals: Vec<Eqn>,
}

impl EqnList {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            literals: Vec::new(),
        }
    }

    #[must_use]
    pub fn from_vec(literals: Vec<Eqn>) -> Self {
        Self { literals }
    }

    #[must_use]
    pub fn from_array(array: Vec<Eqn>) -> Self {
        Self::from_vec(array)
    }

    #[must_use]
    pub fn as_slice(&self) -> &[Eqn] {
        &self.literals
    }

    #[must_use]
    pub fn as_mut_slice(&mut self) -> &mut [Eqn] {
        &mut self.literals
    }

    #[must_use]
    pub fn into_vec(self) -> Vec<Eqn> {
        self.literals
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.literals.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.literals.is_empty()
    }

    pub fn push(&mut self, literal: Eqn) {
        self.literals.push(literal);
    }

    pub fn gc_mark_terms(&self, bank: &TermBank) {
        for literal in &self.literals {
            literal.gc_mark_terms(bank);
        }
    }

    pub fn set_prop(&mut self, prop: EqnProperties) -> usize {
        for literal in &mut self.literals {
            literal.set_prop(prop);
        }
        self.len()
    }

    pub fn del_prop(&mut self, prop: EqnProperties) -> usize {
        for literal in &mut self.literals {
            literal.del_prop(prop);
        }
        self.len()
    }

    pub fn flip_prop(&mut self, prop: EqnProperties) -> usize {
        for literal in &mut self.literals {
            literal.flip_prop(prop);
        }
        self.len()
    }

    #[must_use]
    pub fn query_prop_number(&self, prop: EqnProperties) -> usize {
        self.literals
            .iter()
            .filter(|literal| literal.query_prop(prop))
            .count()
    }

    #[must_use]
    pub fn exists_term_except<F>(&self, except_index: Option<usize>, mut predicate: F) -> bool
    where
        F: FnMut(&Term) -> bool,
    {
        self.literals.iter().enumerate().any(|(index, literal)| {
            Some(index) != except_index && (predicate(literal.left()) || predicate(literal.right()))
        })
    }

    #[must_use]
    pub fn exists_term<F>(&self, predicate: F) -> bool
    where
        F: FnMut(&Term) -> bool,
    {
        self.exists_term_except(None, predicate)
    }

    pub fn map_terms<F>(&mut self, bank: &TermBank, mut mapper: F)
    where
        F: FnMut(&Term) -> Term,
    {
        for literal in &mut self.literals {
            literal.map_terms(bank, &mut mapper);
        }
    }

    #[must_use]
    pub fn to_stack(&self) -> PStack<Eqn> {
        let mut stack = PStack::new();
        for literal in &self.literals {
            stack.push(literal.clone());
        }
        stack
    }

    #[must_use]
    pub fn from_stack(mut stack: PStack<Eqn>) -> Self {
        let mut literals = Vec::with_capacity(stack.len());
        while let Some(literal) = stack.pop() {
            literals.push(literal);
        }
        literals.reverse();
        Self::from_vec(literals)
    }

    #[must_use]
    pub fn split_to_stacks(&self, prop: EqnProperties) -> (PStack<Eqn>, PStack<Eqn>) {
        let mut matching = PStack::new();
        let mut non_matching = PStack::new();
        for literal in &self.literals {
            if literal.query_prop(prop) {
                matching.push(literal.clone());
            } else {
                non_matching.push(literal.clone());
            }
        }
        (matching, non_matching)
    }

    pub fn extract_element(&mut self, index: usize) -> Option<Eqn> {
        if index < self.len() {
            Some(self.literals.remove(index))
        } else {
            None
        }
    }

    #[must_use]
    pub fn extract_by_props(&mut self, props: EqnProperties, negate: bool) -> Self {
        let mut kept = Vec::with_capacity(self.len());
        let mut extracted = Vec::new();
        for literal in self.literals.drain(..) {
            if literal.query_prop(props) ^ negate {
                extracted.push(literal);
            } else {
                kept.push(literal);
            }
        }
        extracted.reverse();
        self.literals = kept;
        Self::from_vec(extracted)
    }

    pub fn delete_element(&mut self, index: usize) -> bool {
        self.extract_element(index).is_some()
    }

    pub fn insert_element(&mut self, index: usize, literal: Eqn) -> bool {
        if index > self.len() {
            return false;
        }
        self.literals.insert(index, literal);
        true
    }

    pub fn insert_first(&mut self, literal: Eqn) {
        self.literals.insert(0, literal);
    }

    pub fn append(&mut self, mut newpart: Self) {
        self.literals.append(&mut newpart.literals);
    }

    pub fn flat_copy(&self, bank: &mut TermBank) -> Result<Self, Diagnostic> {
        let mut copy = Vec::with_capacity(self.len());
        for literal in &self.literals {
            copy.push(literal.flat_copy(bank)?);
        }
        Ok(Self::from_vec(copy))
    }

    pub fn copy_to_bank(&self, bank: &mut TermBank) -> Result<Self, Diagnostic> {
        let mut copy = Vec::with_capacity(self.len());
        for literal in &self.literals {
            copy.push(literal.copy_to_bank(bank)?);
        }
        Ok(Self::from_vec(copy))
    }

    pub fn copy_except_index(
        &self,
        except_index: Option<usize>,
        bank: &mut TermBank,
    ) -> Result<Self, Diagnostic> {
        let mut copy = Vec::with_capacity(self.len());
        for (index, literal) in self.literals.iter().enumerate() {
            if Some(index) != except_index {
                copy.push(literal.copy_to_bank(bank)?);
            }
        }
        Ok(Self::from_vec(copy))
    }

    pub fn copy_opt(&self, bank: &mut TermBank) -> Result<Self, Diagnostic> {
        let mut copy = Vec::with_capacity(self.len());
        for literal in &self.literals {
            copy.push(literal.copy_opt(bank)?);
        }
        Ok(Self::from_vec(copy))
    }

    pub fn copy_opt_except_index(
        &self,
        except_index: Option<usize>,
        bank: &mut TermBank,
    ) -> Result<Self, Diagnostic> {
        let mut copy = Vec::with_capacity(self.len());
        for (index, literal) in self.literals.iter().enumerate() {
            if Some(index) != except_index {
                copy.push(literal.copy_opt(bank)?);
            }
        }
        Ok(Self::from_vec(copy))
    }

    pub fn copy_disjoint(&self, bank: &mut TermBank) -> Result<Self, Diagnostic> {
        let mut copy = Vec::with_capacity(self.len());
        for literal in &self.literals {
            copy.push(literal.copy_disjoint(bank)?);
        }
        Ok(Self::from_vec(copy))
    }

    pub fn copy_repl(
        &self,
        bank: &mut TermBank,
        old: &Term,
        repl: &Term,
    ) -> Result<Self, Diagnostic> {
        let mut copy = Vec::with_capacity(self.len());
        for literal in &self.literals {
            copy.push(literal.copy_repl(bank, old, repl)?);
        }
        Ok(Self::from_vec(copy))
    }

    pub fn copy_repl_plain(
        &self,
        bank: &mut TermBank,
        old: &Term,
        repl: &Term,
    ) -> Result<Self, Diagnostic> {
        let mut copy = Vec::with_capacity(self.len());
        for literal in &self.literals {
            copy.push(literal.copy_repl_plain(bank, old, repl)?);
        }
        Ok(Self::from_vec(copy))
    }

    pub fn negate_eqns(&mut self) {
        for literal in &mut self.literals {
            literal.flip_prop(EP_IS_POSITIVE);
        }
    }

    pub fn remove_duplicates(&mut self, bank: &TermBank) -> usize {
        let mut seen = BTreeSet::new();
        let old_len = self.len();
        self.literals
            .retain(|literal| seen.insert(literal_syntax_key(literal, bank)));
        old_len - self.len()
    }

    pub fn remove_resolved(&mut self, bank: &TermBank) -> usize {
        let old_len = self.len();
        self.literals.retain(|literal| !literal.is_false(bank));
        old_len - self.len()
    }

    pub fn remove_ac_resolved(&mut self, bank: &TermBank) -> usize {
        let old_len = self.len();
        self.literals
            .retain(|literal| literal.is_positive() || !literal.is_ac_trivial(bank));
        old_len - self.len()
    }

    pub fn remove_simple_answers(&mut self, bank: &TermBank) -> usize {
        let old_len = self.len();
        self.literals
            .retain(|literal| !literal.is_simple_answer(bank));
        old_len - self.len()
    }

    #[must_use]
    pub fn find_neg_pure_var_lit_index(&self) -> Option<usize> {
        self.literals
            .iter()
            .position(|literal| literal.is_negative() && literal.is_pure_var())
    }

    #[must_use]
    pub fn find_neg_pure_var_lit(&self) -> Option<&Eqn> {
        self.find_neg_pure_var_lit_index()
            .and_then(|index| self.literals.get(index))
    }

    #[must_use]
    pub fn find_true_index(&self, bank: &TermBank) -> Option<usize> {
        self.literals
            .iter()
            .position(|literal| literal.is_true(bank))
    }

    #[must_use]
    pub fn find_true(&self, bank: &TermBank) -> Option<&Eqn> {
        self.find_true_index(bank)
            .and_then(|index| self.literals.get(index))
    }

    #[must_use]
    pub fn is_trivial(&self) -> bool {
        for index in 0..self.len() {
            for other in &self.literals[index + 1..] {
                let literal = &self.literals[index];
                if !EqnProperties::are_equiv(
                    literal.properties(),
                    other.properties(),
                    EP_IS_POSITIVE,
                ) && literal.equal(other)
                {
                    return true;
                }
            }
        }
        false
    }

    #[must_use]
    pub fn long_is_trivial(&self, bank: &TermBank) -> bool {
        let mut positives = BTreeSet::new();
        let mut negatives = BTreeSet::new();
        for literal in &self.literals {
            let key = eqn_syntax_key(literal, bank);
            if literal.is_positive() {
                if negatives.contains(&key) {
                    return true;
                }
                positives.insert(key);
            } else {
                if positives.contains(&key) {
                    return true;
                }
                negatives.insert(key);
            }
        }
        false
    }

    #[must_use]
    pub fn is_ac_trivial(&self, bank: &TermBank) -> bool {
        self.literals
            .iter()
            .any(|literal| literal.is_positive() && literal.is_ac_trivial(bank))
    }

    #[must_use]
    pub fn is_ground(&self) -> bool {
        self.literals.iter().all(Eqn::is_ground)
    }

    #[must_use]
    pub fn is_equational(&self, bank: &TermBank) -> bool {
        self.literals.iter().any(|literal| literal.is_equ_lit(bank))
    }

    #[must_use]
    pub fn is_pure_equational(&self, bank: &TermBank) -> bool {
        self.literals.iter().all(|literal| literal.is_equ_lit(bank))
    }

    #[must_use]
    pub fn subst_norm_except(
        &self,
        except_index: Option<usize>,
        subst: &mut Substitution,
        vars: &VarBank,
    ) -> usize {
        let result = subst.len();
        for (index, literal) in self.literals.iter().enumerate() {
            if Some(index) != except_index {
                literal.subst_norm(subst, vars);
            }
        }
        result
    }

    #[must_use]
    pub fn subst_norm(&self, subst: &mut Substitution, vars: &VarBank) -> usize {
        self.subst_norm_except(None, subst, vars)
    }

    #[must_use]
    pub fn depth(&self) -> i64 {
        self.literals
            .iter()
            .map(Eqn::depth)
            .max()
            .unwrap_or_default()
    }

    pub fn add_symbol_distribution(&self, dist_array: &mut [i64]) {
        for literal in &self.literals {
            literal.add_symbol_distribution(dist_array);
        }
    }

    pub fn add_symbol_dist_exist(&self, dist_array: &mut [i64], exists: &mut Vec<FunCode>) {
        for literal in &self.literals {
            literal.add_symbol_dist_exist(dist_array, exists);
        }
    }

    pub fn add_symbol_features(&self, mod_stack: &mut Vec<usize>, feature_array: &mut [i64]) {
        for literal in &self.literals {
            literal.add_symbol_features(mod_stack, feature_array);
        }
    }

    pub fn compute_function_ranks(&self, rank_array: &mut [i64], count: &mut i64) {
        for literal in &self.literals {
            literal.compute_function_ranks(rank_array, count);
        }
    }

    pub fn collect_variables(&self, vars: &mut BTreeMap<usize, Term>) -> i64 {
        self.literals
            .iter()
            .map(|literal| literal.collect_variables(vars))
            .sum()
    }

    pub fn collect_fcodes(&self, fcodes: &mut BTreeSet<FunCode>) -> i64 {
        self.literals
            .iter()
            .map(|literal| literal.collect_fcodes(fcodes))
            .sum()
    }

    pub fn add_fun_occs(&self, f_occur: &mut PDIntArray, res_stack: &mut Vec<FunCode>) -> i64 {
        self.literals
            .iter()
            .map(|literal| literal.add_fun_occs(f_occur, res_stack))
            .sum()
    }

    pub fn signed_term_set_prop(&self, prop: TermProperties, pos: bool, neg: bool) {
        for literal in &self.literals {
            if (pos && literal.is_positive()) || (neg && literal.is_negative()) {
                literal.term_set_prop(prop);
            }
        }
    }

    pub fn term_set_prop(&self, prop: TermProperties) {
        self.signed_term_set_prop(prop, true, true);
    }

    pub fn signed_term_del_prop(&self, prop: TermProperties, pos: bool, neg: bool) {
        for literal in &self.literals {
            if (pos && literal.is_positive()) || (neg && literal.is_negative()) {
                literal.term_del_prop(prop);
            }
        }
    }

    pub fn term_del_prop(&self, prop: TermProperties) {
        self.signed_term_del_prop(prop, true, true);
    }

    #[must_use]
    pub fn tb_term_del_prop_count(&self, prop: TermProperties) -> i64 {
        self.literals
            .iter()
            .map(|literal| literal.tb_term_del_prop_count(prop))
            .sum()
    }

    pub fn collect_subterms(&self, collector: &mut PStack<Term>) -> i64 {
        self.literals
            .iter()
            .map(|literal| literal.collect_subterms(collector))
            .sum()
    }

    pub fn collect_ground_terms(
        &self,
        result: &mut BTreeMap<usize, Term>,
        pos_lits: bool,
        neg_lits: bool,
        all_subterms: bool,
    ) -> i64 {
        self.literals
            .iter()
            .filter(|literal| {
                (literal.is_positive() && pos_lits) || (literal.is_negative() && neg_lits)
            })
            .map(|literal| literal.collect_ground_terms(result, all_subterms))
            .sum()
    }

    #[must_use]
    pub fn find_comp_lit_except(
        &self,
        except_index: Option<usize>,
        other: &Eqn,
        left_deref: DerefType,
        right_deref: DerefType,
    ) -> bool {
        self.literals.iter().enumerate().any(|(index, literal)| {
            Some(index) != except_index
                && literal.is_positive() != other.is_positive()
                && literal.equal_deref(other, left_deref, right_deref)
        })
    }
}

fn literal_syntax_key(literal: &Eqn, bank: &TermBank) -> (u8, u8, i64, i64) {
    let sign = u8::from(!literal.is_positive());
    let (equational, max_entry, min_entry) = eqn_syntax_key(literal, bank);
    (sign, equational, max_entry, min_entry)
}

fn eqn_syntax_key(literal: &Eqn, bank: &TermBank) -> (u8, i64, i64) {
    let equational = u8::from(!literal.is_equ_lit(bank));
    let left = literal.left().entry_no();
    let right = literal.right().entry_no();
    (equational, left.max(right), left.min(right))
}

#[cfg(test)]
mod tests {
    use super::{EqnList, EQN_LIST_LONG_LIMIT};
    use crate::basics::pdarrays::{PDIntArray, GROW_EXPONENTIAL};
    use crate::basics::pstacks::PStack;
    use crate::clauses::eqn::Eqn;
    use crate::clauses::eqn_props::{
        EP_IS_MAXIMAL, EP_IS_ORIENTED, EP_IS_POSITIVE, EP_IS_SELECTED, EP_MAX_IS_UP_TO_DATE,
    };
    use crate::terms::signature::{Signature, FP_ASSOCIATIVE, FP_COMMUTATIVE};
    use crate::terms::simpletypes::alloc_arrow_type;
    use crate::terms::subst::Substitution;
    use crate::terms::termbanks::TermBank;
    use crate::terms::termtypes::{DerefType, Term, TP_OP_FLAG, TP_SPECIAL_FLAG};
    use crate::terms::typebanks::TypeBank;
    use std::collections::{BTreeMap, BTreeSet};

    fn test_bank() -> TermBank {
        let mut signature = Signature::new(TypeBank::new());
        signature.insert_internal_codes().unwrap();
        TermBank::new(signature).unwrap()
    }

    fn typed_const(bank: &mut TermBank, name: &str) -> Term {
        let type_ = bank.signature().type_bank().default_type();
        let f_code = bank.signature_mut().insert_id(name, 0, false);
        bank.signature_mut()
            .declare_final_type(f_code, type_.clone())
            .unwrap();
        let term = Term::const_cell_alloc(f_code);
        term.set_type(Some(type_));
        bank.insert(&term, DerefType::Never).unwrap()
    }

    fn typed_pred_const(bank: &mut TermBank, name: &str) -> Term {
        let bool_type = bank.signature().type_bank().bool_type();
        let f_code = bank.signature_mut().insert_id(name, 0, false);
        bank.signature_mut()
            .declare_final_type(f_code, bool_type.clone())
            .unwrap();
        let term = Term::const_cell_alloc(f_code);
        term.set_type(Some(bool_type));
        bank.insert(&term, DerefType::Never).unwrap()
    }

    fn typed_unary(bank: &mut TermBank, name: &str, arg: &Term) -> Term {
        let type_ = bank.signature().type_bank().default_type();
        let f_code = bank.signature_mut().insert_id(name, 1, false);
        bank.signature_mut()
            .declare_final_type(f_code, alloc_arrow_type(vec![type_.clone(), type_.clone()]))
            .unwrap();
        let term = Term::top_alloc(f_code, 1);
        term.set_type(Some(type_));
        term.set_argument(0, arg.clone());
        bank.insert(&term, DerefType::Never).unwrap()
    }

    fn typed_binary_with_code(bank: &mut TermBank, f_code: i64, left: &Term, right: &Term) -> Term {
        let type_ = bank.signature().type_bank().default_type();
        let term = Term::top_alloc(f_code, 2);
        term.set_type(Some(type_));
        term.set_argument(0, left.clone());
        term.set_argument(1, right.clone());
        bank.insert(&term, DerefType::Never).unwrap()
    }

    fn typed_var(bank: &TermBank, f_code: i64) -> Term {
        let type_ = bank.signature().type_bank().default_type();
        bank.vars().var_assert_alloc(f_code, &type_)
    }

    fn answer_term(bank: &mut TermBank, arg: &Term) -> Term {
        let term = Term::top_alloc(bank.signature().answer_code(), 1);
        term.set_type(Some(bank.signature().type_bank().bool_type()));
        term.set_argument(0, arg.clone());
        bank.insert(&term, DerefType::Never).unwrap()
    }

    fn eqn(bank: &mut TermBank, left: &Term, right: &Term, positive: bool) -> Eqn {
        Eqn::alloc(left.clone(), right.clone(), bank, positive).unwrap()
    }

    #[test]
    fn property_helpers_apply_to_each_literal() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let c = typed_const(&mut bank, "c");
        let mut list = EqnList::from_vec(vec![
            eqn(&mut bank, &a, &b, true),
            eqn(&mut bank, &b, &c, false),
        ]);

        assert_eq!(EQN_LIST_LONG_LIMIT, 15);
        assert_eq!(list.set_prop(EP_IS_SELECTED), 2);
        assert_eq!(list.query_prop_number(EP_IS_SELECTED), 2);
        assert_eq!(list.del_prop(EP_IS_SELECTED), 2);
        assert_eq!(list.query_prop_number(EP_IS_SELECTED), 0);
        assert_eq!(list.flip_prop(EP_IS_MAXIMAL), 2);
        assert!(list.as_slice().iter().all(Eqn::is_maximal));
    }

    #[test]
    fn conversions_and_link_operations_preserve_c_ordering() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let c = typed_const(&mut bank, "c");
        let first = eqn(&mut bank, &a, &b, true);
        let second = eqn(&mut bank, &b, &c, true);
        let third = eqn(&mut bank, &c, &a, true);
        let mut list = EqnList::from_array(vec![first.clone(), second.clone(), third.clone()]);

        let stack = list.to_stack();
        assert_eq!(
            stack.as_slice(),
            &[first.clone(), second.clone(), third.clone()]
        );
        let rebuilt = EqnList::from_stack(stack);
        assert_eq!(
            rebuilt.as_slice(),
            &[first.clone(), second.clone(), third.clone()]
        );

        let extracted = list.extract_element(1).unwrap();
        assert_eq!(extracted, second);
        assert_eq!(list.as_slice(), &[first.clone(), third.clone()]);
        assert!(list.insert_element(1, extracted));
        assert_eq!(
            list.as_slice(),
            &[first.clone(), second.clone(), third.clone()]
        );
        assert!(!list.insert_element(4, first.clone()));

        let mut tail = EqnList::new();
        tail.push(first.clone());
        list.append(tail);
        assert_eq!(
            list.as_slice(),
            &[first.clone(), second, third, first.clone()]
        );
        assert!(list.delete_element(3));
    }

    #[test]
    fn extract_by_props_reverses_extracted_literals_like_c_insert_first() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let c = typed_const(&mut bank, "c");
        let mut first = eqn(&mut bank, &a, &b, true);
        let mut second = eqn(&mut bank, &b, &c, true);
        let mut third = eqn(&mut bank, &c, &a, true);
        first.set_position(1);
        second.set_position(2);
        third.set_position(3);
        first.set_prop(EP_IS_SELECTED);
        third.set_prop(EP_IS_SELECTED);
        let mut list = EqnList::from_vec(vec![first, second, third]);

        let extracted = list.extract_by_props(EP_IS_SELECTED, false);

        assert_eq!(
            extracted
                .as_slice()
                .iter()
                .map(Eqn::position)
                .collect::<Vec<_>>(),
            vec![3, 1]
        );
        assert_eq!(list.as_slice()[0].position(), 2);
    }

    #[test]
    fn copy_helpers_forward_to_equation_copy_variants() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let c = typed_const(&mut bank, "c");
        let x = typed_var(&bank, -10);
        let list = EqnList::from_vec(vec![
            eqn(&mut bank, &a, &b, true),
            eqn(&mut bank, &x, &b, false),
        ]);

        let flat = list.flat_copy(&mut bank).unwrap();
        assert_eq!(flat, list);
        let copied = list.copy_except_index(Some(0), &mut bank).unwrap();
        assert_eq!(copied.len(), 1);
        assert!(copied.as_slice()[0].is_negative());

        let replaced = list.copy_repl(&mut bank, &b, &c).unwrap();
        assert_eq!(replaced.as_slice()[0].right(), &c);
        let plain_replaced = list.copy_repl_plain(&mut bank, &b, &a).unwrap();
        assert_eq!(plain_replaced.as_slice()[0].right(), &a);

        let disjoint = list.copy_disjoint(&mut bank).unwrap();
        assert_eq!(disjoint.len(), 2);
        assert_ne!(disjoint.as_slice()[1].left(), &x);
    }

    #[test]
    fn duplicate_and_resolved_removal_match_literal_predicates() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let answer = answer_term(&mut bank, &a);
        let answer_lit = Eqn::alloc(answer, bank.true_term().clone(), &mut bank, true).unwrap();
        let positive = eqn(&mut bank, &a, &b, true);
        let duplicate = eqn(&mut bank, &b, &a, true);
        let negative = eqn(&mut bank, &a, &b, false);
        let false_lit = eqn(&mut bank, &a, &a, false);
        let mut list =
            EqnList::from_vec(vec![positive, duplicate, negative, false_lit, answer_lit]);

        assert_eq!(list.remove_duplicates(&bank), 1);
        assert_eq!(list.remove_resolved(&bank), 1);
        assert_eq!(list.remove_simple_answers(&bank), 1);
        assert_eq!(list.len(), 2);
        assert!(!list.as_slice()[0].literal_equal(&list.as_slice()[1]));
    }

    #[test]
    fn truth_triviality_groundness_and_complement_search_match_c_shapes() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let x = typed_var(&bank, -10);
        let pos = eqn(&mut bank, &a, &b, true);
        let neg = eqn(&mut bank, &b, &a, false);
        let y = typed_var(&bank, -12);
        let pure_var = eqn(&mut bank, &x, &y, false);
        let true_lit = eqn(&mut bank, &a, &a, true);
        let list = EqnList::from_vec(vec![pos.clone(), pure_var, true_lit]);

        assert!(list.find_neg_pure_var_lit().is_some());
        assert_eq!(list.find_true_index(&bank), Some(2));
        assert!(EqnList::from_vec(vec![pos.clone(), neg.clone()]).is_trivial());
        assert!(EqnList::from_vec(vec![pos.clone(), neg.clone()]).long_is_trivial(&bank));
        assert!(!list.is_ground());
        assert!(EqnList::from_vec(vec![pos.clone()]).is_ground());
        assert!(EqnList::from_vec(vec![pos.clone()]).is_equational(&bank));
        assert!(EqnList::from_vec(vec![pos.clone()]).is_pure_equational(&bank));
        assert!(EqnList::from_vec(vec![pos]).find_comp_lit_except(
            None,
            &neg,
            DerefType::Never,
            DerefType::Never
        ));
    }

    #[test]
    fn ac_resolved_and_term_property_helpers_delegate_to_literals() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let type_ = bank.signature().type_bank().default_type();
        let f_code = bank.signature_mut().insert_id("f", 2, false);
        bank.signature_mut()
            .declare_final_type(
                f_code,
                alloc_arrow_type(vec![type_.clone(), type_.clone(), type_]),
            )
            .unwrap();
        bank.signature_mut()
            .set_func_prop(f_code, FP_ASSOCIATIVE | FP_COMMUTATIVE);
        let left = typed_binary_with_code(&mut bank, f_code, &a, &b);
        let right = typed_binary_with_code(&mut bank, f_code, &b, &a);
        let positive = eqn(&mut bank, &left, &right, true);
        let negative = eqn(&mut bank, &left, &right, false);
        let mut list = EqnList::from_vec(vec![positive, negative]);

        assert!(list.is_ac_trivial(&bank));
        assert_eq!(list.remove_ac_resolved(&bank), 1);
        list.signed_term_set_prop(TP_SPECIAL_FLAG, true, false);
        assert!(list.as_slice()[0].left().query_prop(TP_SPECIAL_FLAG));
        list.term_del_prop(TP_SPECIAL_FLAG);
        assert!(!list.as_slice()[0].left().query_prop(TP_SPECIAL_FLAG));

        list.term_set_prop(TP_OP_FLAG);
        assert!(list.tb_term_del_prop_count(TP_OP_FLAG) > 0);
    }

    #[test]
    fn substitution_and_collection_wrappers_accumulate_c_style_counts() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let f_of_a = typed_unary(&mut bank, "f", &a);
        let x = typed_var(&bank, -10);
        let variable_lit = eqn(&mut bank, &x, &b, true);
        let ground_lit = eqn(&mut bank, &f_of_a, &b, false);
        let list = EqnList::from_vec(vec![variable_lit, ground_lit]);

        let mut subst = Substitution::new();
        let vars = crate::terms::termvars::VarBank::new(bank.signature().type_bank());
        assert_eq!(list.subst_norm_except(Some(1), &mut subst, &vars), 0);
        assert_eq!(subst.len(), 1);
        subst.backtrack();

        assert_eq!(list.depth(), 2);
        let mut dist = vec![0; usize::try_from(bank.signature().f_count() + 1).unwrap()];
        list.add_symbol_distribution(&mut dist);
        assert!(dist[usize::try_from(f_of_a.f_code()).unwrap()] > 0);

        let mut exists_dist = vec![0; dist.len()];
        let mut exists = Vec::new();
        list.add_symbol_dist_exist(&mut exists_dist, &mut exists);
        assert!(exists.contains(&f_of_a.f_code()));

        let mut features = vec![0; usize::try_from((bank.signature().f_count() + 1) * 4).unwrap()];
        let mut modified = Vec::new();
        list.add_symbol_features(&mut modified, &mut features);
        assert!(!modified.is_empty());

        let mut ranks = vec![0; dist.len()];
        let mut count = 1;
        list.compute_function_ranks(&mut ranks, &mut count);
        assert!(ranks[usize::try_from(f_of_a.f_code()).unwrap()] > 0);

        let mut variables = BTreeMap::new();
        assert_eq!(list.collect_variables(&mut variables), 1);
        let mut fcodes = BTreeSet::new();
        assert!(list.collect_fcodes(&mut fcodes) >= 3);

        let mut occur = PDIntArray::new_int(2, GROW_EXPONENTIAL);
        let mut occurrence_stack = Vec::new();
        assert!(list.add_fun_occs(&mut occur, &mut occurrence_stack) >= 3);

        let mut subterms = PStack::new();
        assert!(list.collect_subterms(&mut subterms) >= 3);

        let mut ground_terms = BTreeMap::new();
        assert_eq!(
            list.collect_ground_terms(&mut ground_terms, false, true, false),
            1
        );
    }

    #[test]
    fn predicate_literals_affect_equational_classification() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let pred = typed_pred_const(&mut bank, "p");
        let predicate_lit = Eqn::alloc(pred, bank.true_term().clone(), &mut bank, true).unwrap();
        let equation_lit = eqn(&mut bank, &a, &a, true);
        let mut list = EqnList::from_vec(vec![predicate_lit, equation_lit]);

        assert!(list.is_equational(&bank));
        assert!(!list.is_pure_equational(&bank));
        list.negate_eqns();
        assert!(list.as_slice().iter().all(Eqn::is_negative));

        let (negative, positive) = list.split_to_stacks(EP_IS_POSITIVE);
        assert!(negative.is_empty());
        assert_eq!(positive.len(), 2);
    }

    #[test]
    fn map_terms_forwards_literal_normalization() {
        let mut bank = test_bank();
        let atom = typed_pred_const(&mut bank, "p");
        let mut list = EqnList::from_vec(vec![Eqn::alloc(
            atom.clone(),
            bank.true_term().clone(),
            &mut bank,
            true,
        )
        .unwrap()]);
        let false_term = bank.false_term().clone();

        list.map_terms(&bank, |term| {
            if term == &atom {
                false_term.clone()
            } else {
                term.clone()
            }
        });

        assert!(list.as_slice()[0].is_negative());
        assert_eq!(list.as_slice()[0].right(), bank.true_term());
    }

    #[test]
    fn orientation_flags_survive_stack_copies_and_copy_opt_clears_stale_metadata() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let mut literal = eqn(&mut bank, &a, &b, true);
        literal.set_prop(EP_IS_ORIENTED | EP_MAX_IS_UP_TO_DATE);
        let list = EqnList::from_vec(vec![literal]);

        let roundtrip = EqnList::from_stack(list.to_stack());
        assert!(roundtrip.as_slice()[0].is_oriented());

        let mut unoriented = roundtrip.as_slice()[0].clone();
        unoriented.del_prop(EP_IS_ORIENTED);
        let copied = EqnList::from_vec(vec![unoriented])
            .copy_opt(&mut bank)
            .unwrap();
        assert!(!copied.as_slice()[0].query_prop(EP_MAX_IS_UP_TO_DATE));
    }
}
