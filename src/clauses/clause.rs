use crate::basics::defines::DEFAULT_COMCHAR_RAW;
use crate::basics::error::{Diagnostic, ErrorCode};
use crate::basics::partial_orderings::CompareResult;
use crate::basics::pdarrays::PDIntArray;
use crate::basics::pstacks::PStack;
use crate::basics::simple_stuff::ProblemType;
use crate::basics::sysdate::SysDate;
use crate::clauses::clause_props::{
    clause_type_from_identifier, FormulaProperties, CP_IGNORE_PROPS, CP_INITIAL, CP_INPUT_FORMULA,
    CP_IS_D_INDEXED, CP_IS_ORIENTED, CP_IS_SOS, CP_TYPE_AXIOM, CP_TYPE_CONJECTURE,
    CP_TYPE_HYPOTHESIS, CP_TYPE_LEMMA, CP_TYPE_NEG_CONJECTURE, CP_TYPE_QUESTION,
    CP_TYPE_WATCH_CLAUSE,
};
use crate::clauses::clauseinfo::ClauseInfo;
use crate::clauses::clausepos::RewriteSequenceEntry;
use crate::clauses::eqn::{
    eqn_tstp_string, eqn_write, eqn_write_debug, eqn_write_fof, Eqn, EqnFofPrintOptions,
    EqnPrintOptions,
};
use crate::clauses::eqn_props::{
    EqnProperties, EqnSide, EP_DOMINATES, EP_HAS_EQUIV, EP_IS_DOMINATED, EP_PSEUDO_LIT,
};
use crate::clauses::eqnlist::{EqnList, EQN_LIST_LONG_LIMIT};
use crate::clauses::neweval::{EvalCell, EvalObjectHandle};
use crate::inout::basicparser::parse_skip_parenthesized_expr;
use crate::inout::scanner::{test_tok, IoFormat, Scanner, TokenType};
use crate::orderings::ocb::OrderControlBlock;
use crate::terms::functypes::func_symb_start_token;
use crate::terms::functypes::FunCode;
use crate::terms::signature::Signature;
use crate::terms::simpletypes::Type;
use crate::terms::subst::Substitution;
use crate::terms::termbanks::TermBank;
use crate::terms::termtypes::{Term, TermProperties, DEFAULT_FWEIGHT, DEFAULT_VWEIGHT, TP_OP_FLAG};
use crate::terms::termvars::VarBank;
use crate::terms::termweightext::TermWeightExtension;
use std::cmp::{Ordering, Reverse};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::sync::atomic::{AtomicI64, Ordering as AtomicOrdering};

static GLOBAL_CLAUSE_COUNTER: AtomicI64 = AtomicI64::new(i64::MIN);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ClauseParseOptions {
    pub clauses_have_local_variables: bool,
    pub clauses_have_disjoint_variables: bool,
}

impl Default for ClauseParseOptions {
    fn default() -> Self {
        Self {
            clauses_have_local_variables: true,
            clauses_have_disjoint_variables: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Clause {
    ident: i64,
    date: SysDate,
    literals: EqnList,
    neg_lit_no: usize,
    pos_lit_no: usize,
    properties: FormulaProperties,
    weight: i64,
    evaluations: Option<EvalCell>,
    info: Option<ClauseInfo>,
    create_date: i64,
    proof_depth: i64,
    proof_size: i64,
    derivation: Option<PStack<RewriteSequenceEntry>>,
}

impl Clause {
    #[must_use]
    pub fn empty() -> Self {
        Self {
            ident: 0,
            date: SysDate::creation_time(),
            literals: EqnList::new(),
            neg_lit_no: 0,
            pos_lit_no: 0,
            properties: CP_IGNORE_PROPS,
            weight: 0,
            evaluations: None,
            info: None,
            create_date: 0,
            proof_depth: 0,
            proof_size: 0,
            derivation: None,
        }
    }

    #[must_use]
    pub fn alloc(literals: EqnList) -> Self {
        let mut positive = Vec::new();
        let mut negative = Vec::new();
        for literal in literals.into_vec() {
            if literal.is_positive() {
                positive.push(literal);
            } else {
                negative.push(literal);
            }
        }

        let mut clause = Self::empty();
        clause.ident = next_clause_ident();
        clause.pos_lit_no = positive.len();
        clause.neg_lit_no = negative.len();
        positive.append(&mut negative);
        clause.literals = EqnList::from_vec(positive);
        clause
    }

    #[must_use]
    pub const fn ident(&self) -> i64 {
        self.ident
    }

    pub const fn set_ident(&mut self, ident: i64) {
        self.ident = ident;
    }

    #[must_use]
    pub const fn date(&self) -> SysDate {
        self.date
    }

    pub const fn set_date(&mut self, date: SysDate) {
        self.date = date;
    }

    #[must_use]
    pub const fn literals(&self) -> &EqnList {
        &self.literals
    }

    pub fn literals_mut(&mut self) -> &mut EqnList {
        &mut self.literals
    }

    #[must_use]
    pub fn into_literals(self) -> EqnList {
        self.literals
    }

    pub fn replace_literals(&mut self, literals: EqnList) {
        self.literals = literals;
        self.recompute_lit_counts();
    }

    /// Orient literals and mark maximal literals, matching C
    /// `ClauseMarkMaximalTerms`.
    pub fn mark_maximal_terms(&mut self, ocb: &mut OrderControlBlock, bank: &TermBank) {
        self.orient_literals(ocb, bank);
        self.mark_maximal_literals(ocb, bank);
        self.set_prop(CP_IS_ORIENTED);
    }

    /// Orient literals and mark maximal literals using bank-backed ordering
    /// preparation when needed.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if bank-backed term ordering preparation fails.
    pub fn mark_maximal_terms_with_bank(
        &mut self,
        ocb: &mut OrderControlBlock,
        bank: &mut TermBank,
    ) -> Result<(), Diagnostic> {
        self.orient_literals_with_bank(ocb, bank)?;
        self.mark_maximal_literals_with_bank(ocb, bank)?;
        self.set_prop(CP_IS_ORIENTED);
        Ok(())
    }

    /// Conditionally mark maximal terms, matching C
    /// `ClauseCondMarkMaximalTerms`.
    ///
    /// Returns whether marking was performed.
    pub fn cond_mark_maximal_terms(
        &mut self,
        ocb: &mut OrderControlBlock,
        bank: &TermBank,
    ) -> bool {
        if self.query_prop(CP_IS_ORIENTED) {
            false
        } else {
            self.mark_maximal_terms(ocb, bank);
            true
        }
    }

    /// Conditionally mark maximal terms using bank-backed ordering preparation
    /// when needed.
    ///
    /// Returns whether marking was performed.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if bank-backed term ordering preparation fails.
    pub fn cond_mark_maximal_terms_with_bank(
        &mut self,
        ocb: &mut OrderControlBlock,
        bank: &mut TermBank,
    ) -> Result<bool, Diagnostic> {
        if self.query_prop(CP_IS_ORIENTED) {
            Ok(false)
        } else {
            self.mark_maximal_terms_with_bank(ocb, bank)?;
            Ok(true)
        }
    }

    /// Orient all literals, matching C `ClauseOrientLiterals`.
    pub fn orient_literals(&mut self, ocb: &mut OrderControlBlock, bank: &TermBank) -> usize {
        self.literals.orient(ocb, bank)
    }

    /// Orient all literals using bank-backed ordering preparation when needed.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if bank-backed term ordering preparation fails.
    pub fn orient_literals_with_bank(
        &mut self,
        ocb: &mut OrderControlBlock,
        bank: &mut TermBank,
    ) -> Result<usize, Diagnostic> {
        self.literals.orient_with_bank(ocb, bank)
    }

    /// Mark maximal literals, matching C `ClauseMarkMaximalLiterals`.
    pub fn mark_maximal_literals(&mut self, ocb: &mut OrderControlBlock, bank: &TermBank) -> usize {
        self.literals.mark_maximal_literals(ocb, bank)
    }

    /// Mark maximal literals using bank-backed ordering preparation when
    /// needed.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if bank-backed term ordering preparation fails.
    pub fn mark_maximal_literals_with_bank(
        &mut self,
        ocb: &mut OrderControlBlock,
        bank: &mut TermBank,
    ) -> Result<usize, Diagnostic> {
        self.literals.mark_maximal_literals_with_bank(ocb, bank)
    }

    #[must_use]
    pub const fn positive_literal_count(&self) -> usize {
        self.pos_lit_no
    }

    #[must_use]
    pub const fn negative_literal_count(&self) -> usize {
        self.neg_lit_no
    }

    #[must_use]
    pub const fn properties(&self) -> FormulaProperties {
        self.properties
    }

    pub fn set_properties(&mut self, properties: FormulaProperties) {
        self.properties = properties;
    }

    pub fn set_prop(&mut self, prop: FormulaProperties) {
        self.properties.set(prop);
    }

    pub fn del_prop(&mut self, prop: FormulaProperties) {
        self.properties.delete(prop);
    }

    #[must_use]
    pub const fn give_props(&self, prop: FormulaProperties) -> FormulaProperties {
        self.properties.give(prop)
    }

    #[must_use]
    pub const fn query_prop(&self, prop: FormulaProperties) -> bool {
        self.properties.query(prop)
    }

    #[must_use]
    pub const fn is_any_prop_set(&self, prop: FormulaProperties) -> bool {
        self.properties.is_any_set(prop)
    }

    #[must_use]
    pub const fn any_prop_set(&self, prop: FormulaProperties) -> FormulaProperties {
        self.properties.any_set(prop)
    }

    pub fn set_tptp_type(&mut self, type_: FormulaProperties) {
        self.properties.set_tptp_type(type_);
    }

    #[must_use]
    pub const fn query_tptp_type(&self) -> FormulaProperties {
        self.properties.query_tptp_type()
    }

    pub fn set_csscpa_source(&mut self, source: u64) {
        self.properties.set_csscpa_source(source);
    }

    #[must_use]
    pub const fn query_csscpa_source(&self) -> u64 {
        self.properties.query_csscpa_source()
    }

    #[must_use]
    pub const fn weight(&self) -> i64 {
        self.weight
    }

    pub const fn set_weight(&mut self, weight: i64) {
        self.weight = weight;
    }

    #[must_use]
    pub const fn evaluations(&self) -> Option<&EvalCell> {
        self.evaluations.as_ref()
    }

    pub fn evaluations_mut(&mut self) -> Option<&mut EvalCell> {
        self.evaluations.as_mut()
    }

    pub fn add_eval_cell(&mut self, evaluation: EvalCell) {
        self.add_eval_cell_with_object(evaluation, None);
    }

    pub fn add_eval_cell_with_object(
        &mut self,
        mut evaluation: EvalCell,
        object: Option<EvalObjectHandle>,
    ) {
        evaluation.set_object(object);
        self.evaluations = Some(evaluation);
    }

    pub fn remove_evaluations(&mut self) {
        self.evaluations = None;
    }

    pub fn take_evaluations(&mut self) -> Option<EvalCell> {
        self.evaluations.take()
    }

    #[must_use]
    pub const fn info(&self) -> Option<&ClauseInfo> {
        self.info.as_ref()
    }

    pub fn set_info(&mut self, info: Option<ClauseInfo>) {
        self.info = info;
    }

    pub fn take_info(&mut self) -> Option<ClauseInfo> {
        self.info.take()
    }

    #[must_use]
    pub const fn create_date(&self) -> i64 {
        self.create_date
    }

    pub const fn set_create_date(&mut self, create_date: i64) {
        self.create_date = create_date;
    }

    #[must_use]
    pub const fn proof_depth(&self) -> i64 {
        self.proof_depth
    }

    pub const fn set_proof_depth(&mut self, proof_depth: i64) {
        self.proof_depth = proof_depth;
    }

    #[must_use]
    pub const fn proof_size(&self) -> i64 {
        self.proof_size
    }

    pub const fn set_proof_size(&mut self, proof_size: i64) {
        self.proof_size = proof_size;
    }

    #[must_use]
    pub const fn derivation(&self) -> Option<&PStack<RewriteSequenceEntry>> {
        self.derivation.as_ref()
    }

    pub fn ensure_derivation(&mut self) -> &mut PStack<RewriteSequenceEntry> {
        self.derivation.get_or_insert_with(PStack::new)
    }

    pub fn set_derivation(&mut self, derivation: Option<PStack<RewriteSequenceEntry>>) {
        self.derivation = derivation;
    }

    pub fn take_derivation(&mut self) -> Option<PStack<RewriteSequenceEntry>> {
        self.derivation.take()
    }

    #[must_use]
    pub fn derivation_stack_pointer(&self) -> isize {
        self.derivation.as_ref().map_or(0, PStack::stack_pointer)
    }

    pub fn recompute_lit_counts(&mut self) {
        self.pos_lit_no = self
            .literals
            .as_slice()
            .iter()
            .filter(|literal| literal.is_positive())
            .count();
        self.neg_lit_no = self.literals.len() - self.pos_lit_no;
    }

    pub fn gc_mark_terms(&self, bank: &TermBank) {
        self.literals.gc_mark_terms(bank);
    }

    #[must_use]
    pub const fn literal_number(&self) -> usize {
        self.pos_lit_no + self.neg_lit_no
    }

    #[must_use]
    pub fn prop_lit_number(&self, prop: EqnProperties) -> usize {
        self.literals.query_prop_number(prop)
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.literal_number() == 0
    }

    #[must_use]
    pub const fn is_goal(&self) -> bool {
        self.pos_lit_no == 0
    }

    #[must_use]
    pub const fn is_horn(&self) -> bool {
        self.pos_lit_no <= 1
    }

    #[must_use]
    pub const fn is_unit(&self) -> bool {
        self.literal_number() == 1
    }

    #[must_use]
    pub const fn is_demodulator(&self) -> bool {
        self.pos_lit_no == 1 && self.neg_lit_no == 0
    }

    #[must_use]
    pub fn is_rw_rule(&self) -> bool {
        self.is_demodulator()
            && self
                .literals
                .as_slice()
                .first()
                .is_some_and(Eqn::is_oriented)
    }

    #[must_use]
    pub fn is_ground(&self) -> bool {
        self.literals.is_ground()
    }

    #[must_use]
    pub const fn is_positive(&self) -> bool {
        self.neg_lit_no == 0
    }

    #[must_use]
    pub const fn is_negative(&self) -> bool {
        self.pos_lit_no == 0
    }

    #[must_use]
    pub const fn is_mixed(&self) -> bool {
        !(self.is_positive() || self.is_negative())
    }

    #[must_use]
    pub const fn is_hypothesis(&self) -> bool {
        matches!(self.query_tptp_type(), CP_TYPE_HYPOTHESIS)
    }

    #[must_use]
    pub const fn is_conjecture(&self) -> bool {
        matches!(
            self.query_tptp_type(),
            CP_TYPE_NEG_CONJECTURE | CP_TYPE_CONJECTURE
        )
    }

    #[must_use]
    pub fn find_neg_pure_var_lit(&self) -> Option<&Eqn> {
        self.literals.find_neg_pure_var_lit()
    }

    #[must_use]
    pub fn is_trivial(&self, bank: &TermBank) -> bool {
        if self.literals.find_true(bank).is_some() {
            return true;
        }
        if self.pos_lit_no != 0 && self.neg_lit_no != 0 {
            if self.literal_number() > EQN_LIST_LONG_LIMIT {
                return self.literals.long_is_trivial(bank);
            }
            return self.literals.is_trivial();
        }
        false
    }

    #[must_use]
    pub fn has_max_pos_eq_lit(&self, bank: &TermBank) -> bool {
        self.literals.as_slice().iter().any(|literal| {
            literal.is_maximal() && literal.is_equ_lit(bank) && literal.is_positive()
        })
    }

    #[must_use]
    pub fn is_ac_redundant(&self, bank: &TermBank) -> bool {
        if self.is_unit()
            && self.is_positive()
            && self.literals.as_slice().first().is_some_and(|literal| {
                literal.standard_weight() <= 4 * DEFAULT_FWEIGHT + 6 * DEFAULT_VWEIGHT
            })
        {
            return false;
        }
        self.literals.is_ac_trivial(bank)
    }

    #[must_use]
    pub fn is_sem_false(&self) -> bool {
        self.literals
            .as_slice()
            .iter()
            .all(|literal| literal.query_prop(EP_PSEUDO_LIT))
    }

    #[must_use]
    pub fn is_sem_empty(&self, bank: &TermBank) -> bool {
        self.literals
            .as_slice()
            .iter()
            .all(|literal| literal.is_simple_answer(bank))
    }

    pub fn evaluate_answer_literals(&mut self, bank: &TermBank) -> usize {
        if !self.is_sem_false() {
            return 0;
        }
        let removed = self.literals.remove_simple_answers(bank);
        if removed != 0 {
            self.recompute_lit_counts();
        }
        removed
    }

    #[must_use]
    pub fn answer_output_string(&self, bank: &TermBank) -> Option<String> {
        clause_answer_output_string(bank, self)
    }

    #[must_use]
    pub fn is_equational(&self, bank: &TermBank) -> bool {
        self.literals.is_equational(bank)
    }

    #[must_use]
    pub fn is_pure_equational(&self, bank: &TermBank) -> bool {
        self.literals.is_pure_equational(bank)
    }

    pub fn term_set_prop(&self, prop: TermProperties) {
        self.literals.term_set_prop(prop);
    }

    #[must_use]
    pub fn tb_term_del_prop_count(&self, prop: TermProperties) -> i64 {
        self.literals.tb_term_del_prop_count(prop)
    }

    pub fn term_del_prop(&self, prop: TermProperties) {
        self.literals.term_del_prop(prop);
    }

    #[must_use]
    pub const fn is_sos(&self) -> bool {
        self.query_prop(CP_IS_SOS)
    }

    #[must_use]
    pub fn is_range_restricted(&self) -> bool {
        if self.is_positive() || self.is_ground() {
            return true;
        }
        if self.is_negative() {
            return false;
        }
        let (positive, negative) = self.collect_posneg_vars();
        is_key_subset(&negative, &positive)
    }

    #[must_use]
    pub fn is_anti_range_restricted(&self) -> bool {
        if self.is_negative() || self.is_ground() {
            return true;
        }
        if self.is_positive() {
            return false;
        }
        let (positive, negative) = self.collect_posneg_vars();
        is_key_subset(&positive, &negative)
    }

    #[must_use]
    pub fn is_strongly_range_restricted(&self) -> bool {
        if self.is_empty() || self.is_ground() {
            return true;
        }
        if self.is_positive() || self.is_negative() {
            return false;
        }
        let (positive, negative) = self.collect_posneg_vars();
        positive.keys().eq(negative.keys())
    }

    #[must_use]
    pub fn is_eq_definition(&self, bank: &TermBank, min_arity: usize) -> EqnSide {
        if self.is_unit() {
            self.literals.as_slice()[0].is_definition(bank, min_arity)
        } else {
            EqnSide::NoSide
        }
    }

    pub fn sort_literals_by<F>(&mut self, mut compare: F)
    where
        F: FnMut(&Eqn, &Eqn) -> i64,
    {
        if self.literal_number() > 1 {
            let mut literals = std::mem::take(&mut self.literals).into_vec();
            for (index, literal) in literals.iter_mut().enumerate() {
                literal.set_position(index_to_i32(index));
            }
            literals.sort_by(|left, right| cmp_i64_to_order(compare(left, right)));
            self.literals = EqnList::from_vec(literals);
        }
    }

    pub fn canonize(&mut self, bank: &TermBank) {
        for literal in self.literals.as_mut_slice() {
            literal.canonize();
        }
        self.sort_literals_by(|left, right| left.struct_weight_lex_compare(right, bank));
    }

    pub fn subsume_order_sort_literals(&mut self, bank: &TermBank) {
        self.sort_literals_by(|left, right| {
            i64::from(left.subsume_inverse_refined_compare(right, bank))
        });
    }

    #[must_use]
    pub fn is_subsume_ordered(&self, bank: &TermBank) -> bool {
        self.is_sorted_by(|left, right| i64::from(left.subsume_inverse_compare(right, bank)))
    }

    #[must_use]
    pub fn is_sorted_by<F>(&self, mut compare: F) -> bool
    where
        F: FnMut(&Eqn, &Eqn) -> i64,
    {
        self.literals
            .as_slice()
            .windows(2)
            .all(|window| compare(&window[0], &window[1]) <= 0)
    }

    #[must_use]
    pub fn struct_weight_compare(&self, other: &Self, bank: &TermBank) -> i64 {
        let self_class = clause_polarity_class(self);
        let other_class = clause_polarity_class(other);
        let mut result = self_class - other_class;
        if result != 0 {
            return result;
        }
        result = usize_diff(self.neg_lit_no, other.neg_lit_no);
        if result != 0 {
            return result;
        }
        result = usize_diff(self.pos_lit_no, other.pos_lit_no);
        if result != 0 {
            return result;
        }
        result = self.weight - other.weight;
        if result != 0 {
            return result;
        }
        for (left, right) in self
            .literals
            .as_slice()
            .iter()
            .zip(other.literals.as_slice())
        {
            result = left.struct_weight_compare(right, bank);
            if result != 0 {
                return result;
            }
        }
        0
    }

    #[must_use]
    pub fn struct_weight_lex_compare(&self, other: &Self, bank: &TermBank) -> i64 {
        let mut result = self.struct_weight_compare(other, bank);
        if result != 0 {
            return result;
        }
        for (left, right) in self
            .literals
            .as_slice()
            .iter()
            .zip(other.literals.as_slice())
        {
            result = left.struct_weight_lex_compare(right, bank);
            if result != 0 {
                return result;
            }
        }
        self.ident - other.ident
    }

    #[must_use]
    pub fn compare_fun(&self, other: &Self) -> i32 {
        let mut result = usize_diff(other.pos_lit_no, self.pos_lit_no);
        if result == 0 {
            result = usize_diff(other.neg_lit_no, self.neg_lit_no);
        }
        if result != 0 {
            return cmp_i64(result);
        }
        for (left, right) in self
            .literals
            .as_slice()
            .iter()
            .zip(other.literals.as_slice())
        {
            let literal_cmp = left.literal_compare_fun(right);
            if literal_cmp != 0 {
                return literal_cmp;
            }
        }
        0
    }

    #[must_use]
    pub fn cmp_by_id(&self, other: &Self) -> i32 {
        cmp_i64(self.ident - other.ident)
    }

    #[must_use]
    pub fn cmp_by_struct_weight(&self, other: &Self, bank: &TermBank) -> i32 {
        cmp_i64(self.struct_weight_lex_compare(other, bank))
    }

    pub fn copy_to_bank(&self, bank: &mut TermBank) -> Result<Self, Diagnostic> {
        Ok(self.copy_with_literals(self.literals.copy_to_bank(bank)?))
    }

    pub fn flat_copy(&self, bank: &mut TermBank) -> Result<Self, Diagnostic> {
        Ok(self.copy_with_literals(self.literals.flat_copy(bank)?))
    }

    pub fn copy_opt(&self, bank: &mut TermBank) -> Result<Self, Diagnostic> {
        Ok(self.copy_with_literals(self.literals.copy_opt(bank)?))
    }

    pub fn copy_disjoint(&self, bank: &mut TermBank) -> Result<Self, Diagnostic> {
        Ok(self.copy_with_literals(self.literals.copy_disjoint(bank)?))
    }

    /// Return a skolemized copy, matching C `ClauseSkolemize`.
    ///
    /// The source clause is left unchanged after the temporary variable
    /// bindings are backtracked.
    pub fn skolemize(&self, bank: &mut TermBank) -> Result<Self, Diagnostic> {
        let mut subst = Substitution::new();
        for literal in self.literals.as_slice() {
            if let Err(err) = skolemize_term_in_bank(literal.left(), &mut subst, bank)
                .and_then(|()| skolemize_term_in_bank(literal.right(), &mut subst, bank))
            {
                subst.backtrack_skolem();
                return Err(err);
            }
        }

        let result = self.copy_to_bank(bank);
        subst.backtrack_skolem();
        result
    }

    /// Destructively normalizes variables and rewrites the literal list through
    /// the provided term bank when normalization created bindings.
    ///
    /// # Panics
    ///
    /// Panics if the clause carries `CP_IS_D_INDEXED`, matching the C assertion
    /// that indexed clauses cannot be rewritten in place.
    pub fn normalize_vars(
        &mut self,
        bank: &mut TermBank,
        fresh_vars: &VarBank,
    ) -> Result<(), Diagnostic> {
        assert!(
            !self.query_prop(CP_IS_D_INDEXED),
            "indexed clauses cannot be normalized in place"
        );
        if self.is_empty() {
            return Ok(());
        }

        let mut subst = Substitution::new();
        fresh_vars.reset_v_counts();
        let _ = self.literals.subst_norm(&mut subst, fresh_vars);
        if !subst.is_empty() {
            self.literals = self.literals.copy_to_bank(bank)?;
        }
        subst.delete();
        Ok(())
    }

    #[must_use]
    #[expect(
        clippy::too_many_arguments,
        reason = "C-compatible helper mirrors ccl_clauses argument list"
    )]
    pub fn literal_weight(
        &self,
        bank: &TermBank,
        max_term_multiplier: f64,
        max_literal_multiplier: f64,
        pos_multiplier: f64,
        vweight: i64,
        fweight: i64,
        app_var_mult: f64,
        count_eq_encoding: bool,
    ) -> f64 {
        self.literals
            .as_slice()
            .iter()
            .map(|literal| {
                literal.literal_weight(
                    bank,
                    max_term_multiplier,
                    max_literal_multiplier,
                    pos_multiplier,
                    vweight,
                    fweight,
                    app_var_mult,
                    count_eq_encoding,
                )
            })
            .sum()
    }

    #[must_use]
    #[expect(
        clippy::too_many_arguments,
        reason = "C-compatible helper mirrors ccl_clauses argument list"
    )]
    pub fn fun_weight(
        &self,
        max_term_multiplier: f64,
        max_literal_multiplier: f64,
        pos_multiplier: f64,
        vweight: i64,
        flimit: FunCode,
        fweights: &[i64],
        default_fweight: i64,
        app_var_mult: f64,
        typefreqs: Option<&BTreeMap<i64, i64>>,
    ) -> f64 {
        self.literals
            .as_slice()
            .iter()
            .map(|literal| {
                literal.literal_fun_weight(
                    max_term_multiplier,
                    max_literal_multiplier,
                    pos_multiplier,
                    vweight,
                    flimit,
                    fweights,
                    default_fweight,
                    app_var_mult,
                    typefreqs,
                )
            })
            .sum()
    }

    #[must_use]
    pub fn term_ext_weight<Data, WeightFun>(
        &self,
        extension: &TermWeightExtension<Data, WeightFun>,
    ) -> f64
    where
        WeightFun: Fn(&Term, &Data) -> f64,
    {
        self.literals
            .as_slice()
            .iter()
            .map(|literal| literal.literal_term_ext_weight(extension))
            .sum()
    }

    #[must_use]
    #[expect(
        clippy::too_many_arguments,
        reason = "C-compatible helper mirrors ccl_clauses argument list"
    )]
    pub fn non_linear_weight(
        &self,
        bank: &TermBank,
        max_term_multiplier: f64,
        max_literal_multiplier: f64,
        pos_multiplier: f64,
        first_var_weight: i64,
        repeat_var_weight: i64,
        fweight: i64,
        app_var_mult: f64,
        count_eq_encoding: bool,
    ) -> f64 {
        self.literals
            .as_slice()
            .iter()
            .map(|literal| {
                literal.literal_non_linear_weight(
                    bank,
                    max_term_multiplier,
                    max_literal_multiplier,
                    pos_multiplier,
                    first_var_weight,
                    repeat_var_weight,
                    fweight,
                    app_var_mult,
                    count_eq_encoding,
                )
            })
            .sum()
    }

    #[must_use]
    #[expect(
        clippy::too_many_arguments,
        reason = "C-compatible helper mirrors ccl_clauses argument list"
    )]
    pub fn sym_type_weight(
        &self,
        max_term_multiplier: f64,
        max_literal_multiplier: f64,
        pos_multiplier: f64,
        vweight: i64,
        fweight: i64,
        cweight: i64,
        pweight: i64,
        app_var_mult: f64,
    ) -> f64 {
        self.literals
            .as_slice()
            .iter()
            .map(|literal| {
                literal.literal_sym_type_weight(
                    max_term_multiplier,
                    max_literal_multiplier,
                    pos_multiplier,
                    vweight,
                    fweight,
                    cweight,
                    pweight,
                    app_var_mult,
                )
            })
            .sum()
    }

    #[must_use]
    pub fn standard_weight(&self) -> i64 {
        self.literals
            .as_slice()
            .iter()
            .map(Eqn::standard_weight)
            .sum()
    }

    #[must_use]
    #[expect(
        clippy::too_many_arguments,
        reason = "C-compatible helper mirrors ccl_clauses argument list"
    )]
    pub fn orient_weight(
        &self,
        bank: &TermBank,
        unorientable_literal_multiplier: f64,
        max_literal_multiplier: f64,
        pos_multiplier: f64,
        vweight: i64,
        fweight: i64,
        app_var_mult: f64,
        count_eq_encoding: bool,
    ) -> f64 {
        self.literals
            .as_slice()
            .iter()
            .map(|literal| {
                let mut weight = literal.literal_weight(
                    bank,
                    1.0,
                    max_literal_multiplier,
                    pos_multiplier,
                    vweight,
                    fweight,
                    app_var_mult,
                    count_eq_encoding,
                );
                if !literal.is_oriented() {
                    weight *= unorientable_literal_multiplier;
                }
                weight
            })
            .sum()
    }

    #[must_use]
    pub fn depth(&self) -> i64 {
        self.literals.depth()
    }

    /// Return whether this clause is not greater-or-equal to `other` under
    /// the multiset extension of the selected literal ordering.
    ///
    /// This preserves C `ClauseNotGreaterEqual`, including its temporary
    /// mutation of `EPHasEquiv`, `EPDominates`, and `EPIsDominated` flags.
    ///
    /// # Panics
    ///
    /// Panics if literal comparison returns one of the C-unexpected partial
    /// ordering sentinels.
    pub fn not_greater_equal(
        &mut self,
        ocb: &mut OrderControlBlock,
        bank: &TermBank,
        other: &mut Self,
    ) -> bool {
        self.literals.del_prop(EP_HAS_EQUIV | EP_DOMINATES);

        let left_literals = self.literals.as_mut_slice();
        let right_literals = other.literals.as_mut_slice();
        let mut left_index = 0;

        for right_index in 0..right_literals.len() {
            let (through_current, later_right) = right_literals.split_at_mut(right_index + 1);
            let current_right = &mut through_current[right_index];
            current_right.del_prop(EP_HAS_EQUIV | EP_IS_DOMINATED);

            let mut found_equal = false;
            let mut found_greater = false;

            while left_index < left_literals.len() && !current_right.has_equiv() {
                if !left_literals[left_index].has_equiv() {
                    let relation =
                        left_literals[left_index].literal_compare(ocb, bank, current_right);
                    match relation {
                        CompareResult::Greater => {
                            let mut found_equal_later = false;
                            if !left_literals[left_index].dominates() {
                                found_equal_later = found_eq_lit_later(
                                    ocb,
                                    bank,
                                    &mut left_literals[left_index],
                                    later_right,
                                );
                            }
                            if !found_equal_later {
                                left_literals[left_index].set_prop(EP_DOMINATES);
                                current_right.set_prop(EP_IS_DOMINATED);
                                found_greater = true;
                            }
                        }
                        CompareResult::Equal => {
                            left_literals[left_index].set_prop(EP_HAS_EQUIV);
                            current_right.set_prop(EP_HAS_EQUIV);
                            found_equal = true;
                        }
                        CompareResult::Lesser | CompareResult::Uncomparable => {}
                        CompareResult::Unknown
                        | CompareResult::NotGreaterEqual
                        | CompareResult::NotLessEqual => {
                            panic!("unexpected literal comparison relation: {relation:?}");
                        }
                    }
                }
                left_index += 1;
            }

            if !found_equal && !found_greater {
                return true;
            }
        }
        false
    }

    #[must_use]
    pub fn norm_subst(&self, subst: &mut Substitution, vars: &VarBank) -> usize {
        self.literals.subst_norm(subst, vars)
    }

    pub fn add_symbol_distribution(&self, dist_array: &mut [i64]) {
        self.literals.add_symbol_distribution(dist_array);
    }

    pub fn add_type_distribution(&self, sig: &mut Signature, type_array: &mut [i64]) {
        self.literals.add_type_distribution(sig, type_array);
    }

    pub fn add_symbol_dist_exist(&self, dist_array: &mut [i64], exists: &mut Vec<FunCode>) {
        self.literals.add_symbol_dist_exist(dist_array, exists);
    }

    pub fn add_symbol_features(&self, mod_stack: &mut Vec<usize>, feature_array: &mut [i64]) {
        self.literals.add_symbol_features(mod_stack, feature_array);
    }

    pub fn compute_function_ranks(&self, rank_array: &mut [i64], count: &mut i64) {
        self.literals.compute_function_ranks(rank_array, count);
    }

    pub fn collect_variables(&self, vars: &mut BTreeMap<usize, Term>) -> i64 {
        self.literals.collect_variables(vars)
    }

    pub fn collect_fcodes(&self, fcodes: &mut BTreeSet<FunCode>) -> i64 {
        self.literals.collect_fcodes(fcodes)
    }

    pub fn add_fun_occs(&self, f_occur: &mut PDIntArray, res_stack: &mut Vec<FunCode>) -> i64 {
        self.literals.add_fun_occs(f_occur, res_stack)
    }

    pub fn collect_subterms(&self, collector: &mut PStack<Term>) -> i64 {
        let start = collector.len();
        let result = self.literals.collect_subterms(collector);
        for term in &collector.as_slice()[start..] {
            term.del_prop(TP_OP_FLAG);
        }
        result
    }

    pub fn return_fcodes(&self, f_codes: &mut Vec<FunCode>) -> i64 {
        let start = f_codes.len();
        let mut subterms = PStack::new();
        self.collect_subterms(&mut subterms);
        let mut seen = BTreeSet::new();
        for term in subterms.as_slice() {
            if !term.is_any_var() && seen.insert(term.f_code()) {
                f_codes.push(term.f_code());
            }
        }
        usize_to_i64(f_codes.len() - start)
    }

    #[must_use]
    pub fn is_untyped(&self) -> bool {
        self.literals.as_slice().iter().all(Eqn::is_untyped)
    }

    #[must_use]
    pub fn query_literal<F>(&self, query: F) -> bool
    where
        F: FnMut(&Eqn) -> bool,
    {
        self.literals.as_slice().iter().any(query)
    }

    pub fn collect_ground_terms(
        &self,
        result: &mut BTreeMap<usize, Term>,
        pos_lits: bool,
        neg_lits: bool,
        all_subterms: bool,
    ) -> i64 {
        self.literals
            .collect_ground_terms(result, pos_lits, neg_lits, all_subterms)
    }

    fn copy_with_literals(&self, literals: EqnList) -> Self {
        let mut copy = Self {
            ident: self.ident,
            date: self.date,
            literals,
            neg_lit_no: self.neg_lit_no,
            pos_lit_no: self.pos_lit_no,
            properties: self.properties,
            weight: self.weight,
            evaluations: None,
            info: None,
            create_date: self.create_date,
            proof_depth: self.proof_depth,
            proof_size: self.proof_size,
            derivation: None,
        };
        copy.recompute_lit_counts();
        copy
    }

    fn collect_posneg_vars(&self) -> (BTreeMap<usize, Term>, BTreeMap<usize, Term>) {
        let mut positive = BTreeMap::new();
        let mut negative = BTreeMap::new();
        for literal in self.literals.as_slice() {
            if literal.is_positive() {
                literal.collect_variables(&mut positive);
            } else {
                literal.collect_variables(&mut negative);
            }
        }
        (positive, negative)
    }
}

#[must_use]
pub fn clause_answer_output_string(bank: &TermBank, clause: &Clause) -> Option<String> {
    if !clause.is_sem_false() || clause.is_empty() {
        return None;
    }

    let mut output = format!("{DEFAULT_COMCHAR_RAW} SZS answers Tuple [");
    if clause.literal_number() > 1 {
        output.push('(');
    }
    if let Some((first, rest)) = clause.literals().as_slice().split_first() {
        output.push_str(&answer_literal_string(bank, first));
        for literal in rest {
            output.push('|');
            output.push_str(&answer_literal_string(bank, literal));
        }
    }
    if clause.literal_number() > 1 {
        output.push(')');
    }
    output.push_str("|_]\n");
    Some(output)
}

fn answer_literal_string(bank: &TermBank, literal: &Eqn) -> String {
    let mut output = String::from("[");
    if literal.query_prop(EP_PSEUDO_LIT)
        && bank
            .signature()
            .is_simple_answer_pred(literal.left().f_code())
        && literal
            .left()
            .argument(0)
            .as_ref()
            .is_some_and(|answer_term| answer_term.f_code() > 0)
    {
        let answer_term = literal
            .left()
            .argument(0)
            .expect("checked answer literal must have an answer term");
        for index in 0..answer_term.arity() {
            if index != 0 {
                output.push_str(", ");
            }
            let argument = answer_term
                .argument(index)
                .expect("answer tuple term must have initialized arguments");
            let _ = bank.write_term(&mut output, &argument, true);
        }
    } else {
        output.push_str(&eqn_tstp_string(bank, literal, true, false));
    }
    output.push(']');
    output
}

#[must_use]
pub fn clause_starts_maybe(scanner: &Scanner) -> bool {
    if scanner.test_tok(func_symb_start_token() | TokenType::TILDE_SIGN) {
        return true;
    }
    if scanner.test_tok(TokenType::LESSER_SIGN | TokenType::QUESTION_MARK) {
        let look = scanner.look_token(1);
        return !look.skipped() && test_tok(look, TokenType::HYPHEN);
    }
    false
}

pub fn clause_parse(
    scanner: &mut Scanner,
    bank: &mut TermBank,
    problem_type: ProblemType,
) -> Result<Clause, Diagnostic> {
    clause_parse_with_options(scanner, bank, problem_type, ClauseParseOptions::default())
}

/// Parses the C `ClauseParse` control flow over the currently ported simple
/// term/equation parser.
///
/// # Panics
///
/// Panics if the scanner format is `IoFormat::Auto`, matching the C
/// precondition that scanner format has already been resolved.
pub fn clause_parse_with_options(
    scanner: &mut Scanner,
    bank: &mut TermBank,
    problem_type: ProblemType,
    options: ClauseParseOptions,
) -> Result<Clause, Diagnostic> {
    apply_clause_parse_var_scope(bank, options);
    let start_source = token_source_string(scanner.current_token().source_bytes());
    let start_line = usize_to_i64(scanner.current_token().line());
    let start_column = usize_to_i64(scanner.current_token().column());
    let mut type_ = CP_TYPE_AXIOM;
    let (literals, name) = match scanner.format() {
        IoFormat::Tptp => clause_parse_tptp(scanner, bank, problem_type, &mut type_)?,
        IoFormat::Tstp => clause_parse_tstp(scanner, bank, problem_type, &mut type_)?,
        IoFormat::Lop => (
            clause_parse_lop(scanner, bank, problem_type, &mut type_)?,
            None,
        ),
        IoFormat::Auto => panic!("format not supported"),
    };
    scanner.accept_tok(TokenType::FULLSTOP)?;

    let mut clause = Clause::alloc(literals);
    clause.set_tptp_type(type_);
    clause.set_prop(CP_INITIAL | CP_INPUT_FORMULA);
    clause.set_info(Some(ClauseInfo::new(
        name.as_deref(),
        Some(start_source.as_str()),
        start_line,
        start_column,
    )));
    Ok(clause)
}

pub fn clause_pcl_parse(
    scanner: &mut Scanner,
    bank: &mut TermBank,
    problem_type: ProblemType,
) -> Result<Clause, Diagnostic> {
    clause_pcl_parse_with_options(scanner, bank, problem_type, ClauseParseOptions::default())
}

/// Parses the C `ClausePCLParse` shape over the currently ported simple
/// term/equation parser.
///
/// # Panics
///
/// Panics if the scanner format is not `IoFormat::Tptp`, matching the C
/// assertion.
pub fn clause_pcl_parse_with_options(
    scanner: &mut Scanner,
    bank: &mut TermBank,
    problem_type: ProblemType,
    options: ClauseParseOptions,
) -> Result<Clause, Diagnostic> {
    assert_eq!(scanner.format(), IoFormat::Tptp);
    if options.clauses_have_local_variables {
        bank.vars().clear_ext_names();
    }
    scanner.accept_tok(TokenType::OPEN_SQUARE)?;
    let literals = EqnList::parse(scanner, bank, TokenType::COMMA, problem_type)?;
    scanner.accept_tok(TokenType::CLOSE_SQUARE)?;
    let mut clause = Clause::alloc(literals);
    clause.set_tptp_type(if clause.positive_literal_count() != 0 {
        CP_TYPE_AXIOM
    } else {
        CP_TYPE_CONJECTURE
    });
    Ok(clause)
}

fn clause_parse_tptp(
    scanner: &mut Scanner,
    bank: &mut TermBank,
    problem_type: ProblemType,
    type_: &mut FormulaProperties,
) -> Result<(EqnList, Option<String>), Diagnostic> {
    scanner.accept_id("input_clause")?;
    scanner.accept_tok(TokenType::OPEN_BRACKET)?;
    let name = scanner.current_token().literal();
    scanner.accept_tok(TokenType::NAME)?;
    scanner.accept_tok(TokenType::COMMA)?;
    *type_ = clause_type_parse(
        scanner,
        "axiom|hypothesis|conjecture|lemma|unknown|watchlist",
        problem_type,
    )?;
    if *type_ == CP_TYPE_CONJECTURE {
        *type_ = CP_TYPE_NEG_CONJECTURE;
    }
    scanner.accept_tok(TokenType::COMMA)?;
    scanner.accept_tok(TokenType::OPEN_SQUARE)?;
    let literals = EqnList::parse(scanner, bank, TokenType::COMMA, problem_type)?;
    scanner.accept_tok(TokenType::CLOSE_SQUARE)?;
    scanner.accept_tok(TokenType::CLOSE_BRACKET)?;
    Ok((literals, Some(name)))
}

fn clause_parse_tstp(
    scanner: &mut Scanner,
    bank: &mut TermBank,
    problem_type: ProblemType,
    type_: &mut FormulaProperties,
) -> Result<(EqnList, Option<String>), Diagnostic> {
    scanner.accept_id("cnf")?;
    scanner.accept_tok(TokenType::OPEN_BRACKET)?;
    let name = scanner.current_token().literal();
    scanner.accept_tok(TokenType::NAME | TokenType::POS_INT | TokenType::SQ_STRING)?;
    scanner.accept_tok(TokenType::COMMA)?;
    *type_ = clause_type_parse(
        scanner,
        "axiom|definition|theorem|assumption|hypothesis|negated_conjecture|lemma|unknown|plain|watchlist",
        problem_type,
    )?;
    scanner.accept_tok(TokenType::COMMA)?;
    let literals = if scanner.test_tok(TokenType::OPEN_BRACKET) {
        scanner.accept_tok(TokenType::OPEN_BRACKET)?;
        let parsed = EqnList::parse(scanner, bank, TokenType::PIPE, problem_type)?;
        scanner.accept_tok(TokenType::CLOSE_BRACKET)?;
        parsed
    } else {
        EqnList::parse(scanner, bank, TokenType::PIPE, problem_type)?
    };
    if scanner.test_tok(TokenType::COMMA) {
        scanner.accept_tok(TokenType::COMMA)?;
        tstp_skip_source(scanner)?;
        if scanner.test_tok(TokenType::COMMA) {
            scanner.accept_tok(TokenType::COMMA)?;
            scanner.check_tok(TokenType::OPEN_SQUARE)?;
            parse_skip_parenthesized_expr(scanner)?;
        }
    }
    scanner.accept_tok(TokenType::CLOSE_BRACKET)?;
    Ok((literals, Some(name)))
}

fn clause_parse_lop(
    scanner: &mut Scanner,
    bank: &mut TermBank,
    problem_type: ProblemType,
    type_: &mut FormulaProperties,
) -> Result<EqnList, Diagnostic> {
    let mut conclusion = EqnList::parse(scanner, bank, TokenType::SEMICOLON, problem_type)?;
    let mut procedural = false;
    if scanner.test_tok(TokenType::COLON) {
        if conclusion.len() > 1 {
            return Err(syntax_error(
                "Procedural rule cannot have more than one head literal",
            ));
        }
        procedural = true;
    } else if scanner.test_tok(TokenType::QUESTION_MARK) {
        if !conclusion.is_empty() {
            return Err(syntax_error("Query should consist only of tail literals"));
        }
        *type_ = CP_TYPE_NEG_CONJECTURE;
    }

    if scanner.test_tok(TokenType::FULLSTOP) {
        if conclusion.len() > 1 {
            return Err(syntax_error(
                "Procedural fact cannot have more than one literal",
            ));
        }
        return Ok(conclusion);
    }

    scanner.accept_tok(TokenType::LESSER_SIGN | TokenType::COLON | TokenType::QUESTION_MARK)?;
    scanner.accept_tok_no_skip(TokenType::HYPHEN)?;
    let mut preconditions = EqnList::parse(scanner, bank, TokenType::COMMA, problem_type)?;
    if procedural && preconditions.is_empty() {
        return Err(syntax_error(
            "Procedural rule or query needs at least one tail literal",
        ));
    }
    preconditions.negate_eqns();
    conclusion.append(preconditions);
    Ok(conclusion)
}

fn clause_type_parse(
    scanner: &mut Scanner,
    legal_types: &str,
    problem_type: ProblemType,
) -> Result<FormulaProperties, Diagnostic> {
    scanner.check_id(legal_types)?;
    let identifier = scanner.current_token().literal();
    scanner.accept_tok(TokenType::IDENT)?;
    Ok(clause_type_from_identifier(&identifier, problem_type))
}

fn tstp_skip_source(scanner: &mut Scanner) -> Result<(), Diagnostic> {
    if scanner.test_tok(TokenType::OPEN_SQUARE) {
        parse_skip_parenthesized_expr(scanner)
    } else {
        scanner.accept_tok(TokenType::IDENTIFIER | TokenType::POS_INT)?;
        if scanner.test_tok(TokenType::OPEN_BRACKET) {
            parse_skip_parenthesized_expr(scanner)?;
        }
        Ok(())
    }
}

fn apply_clause_parse_var_scope(bank: &TermBank, options: ClauseParseOptions) {
    if options.clauses_have_local_variables {
        bank.vars().clear_ext_names();
    }
    if options.clauses_have_disjoint_variables {
        bank.vars().clear_ext_names_no_reset();
    }
}

fn syntax_error(message: &str) -> Diagnostic {
    Diagnostic::new(ErrorCode::SYNTAX_ERROR, message)
}

pub fn clause_write_list(
    output: &mut impl fmt::Write,
    bank: &TermBank,
    clause: &Clause,
    full_terms: bool,
    options: EqnPrintOptions,
) -> fmt::Result {
    clause
        .literals()
        .write_print(output, bank, "; ", false, full_terms, options)?;
    output.write_str(" <-.")
}

#[must_use]
pub fn clause_print_list_string(bank: &TermBank, clause: &Clause, full_terms: bool) -> String {
    let mut output = String::new();
    let _ = clause_write_list(
        &mut output,
        bank,
        clause,
        full_terms,
        EqnPrintOptions::default(),
    );
    output
}

pub fn clause_write_axiom(
    output: &mut impl fmt::Write,
    bank: &TermBank,
    clause: &Clause,
    full_terms: bool,
    options: EqnPrintOptions,
) -> fmt::Result {
    let mut printed = 0;
    for literal in clause
        .literals()
        .as_slice()
        .iter()
        .filter(|literal| literal.is_positive())
    {
        eqn_write(output, bank, literal, false, full_terms, options)?;
        printed += 1;
        if printed < clause.positive_literal_count() {
            output.write_str("; ")?;
        }
    }

    output.write_str(" <- ")?;

    printed = 0;
    for literal in clause
        .literals()
        .as_slice()
        .iter()
        .filter(|literal| literal.is_negative())
    {
        eqn_write(output, bank, literal, true, full_terms, options)?;
        printed += 1;
        if printed < clause.negative_literal_count() {
            output.write_str(", ")?;
        }
    }

    output.write_char('.')
}

#[must_use]
pub fn clause_print_axiom_string(bank: &TermBank, clause: &Clause, full_terms: bool) -> String {
    clause_print_axiom_string_with_options(bank, clause, full_terms, EqnPrintOptions::default())
}

#[must_use]
pub fn clause_print_axiom_string_with_options(
    bank: &TermBank,
    clause: &Clause,
    full_terms: bool,
    options: EqnPrintOptions,
) -> String {
    let mut output = String::new();
    let _ = clause_write_axiom(&mut output, bank, clause, full_terms, options);
    output
}

pub fn clause_write_rule(
    output: &mut impl fmt::Write,
    bank: &TermBank,
    clause: &Clause,
    full_terms: bool,
    options: EqnPrintOptions,
) -> fmt::Result {
    if let Some((first, rest)) = clause.literals().as_slice().split_first() {
        eqn_write(output, bank, first, false, full_terms, options)?;
        if !rest.is_empty() {
            output.write_str(" <- ")?;
            write_literal_tail(output, bank, rest, full_terms, options)?;
        }
    } else {
        output.write_str(" <- ")?;
    }
    output.write_char('.')
}

#[must_use]
pub fn clause_print_rule_string(bank: &TermBank, clause: &Clause, full_terms: bool) -> String {
    let mut output = String::new();
    let _ = clause_write_rule(
        &mut output,
        bank,
        clause,
        full_terms,
        EqnPrintOptions::default(),
    );
    output
}

pub fn clause_write_goal(
    output: &mut impl fmt::Write,
    bank: &TermBank,
    clause: &Clause,
    full_terms: bool,
    options: EqnPrintOptions,
) -> fmt::Result {
    output.write_str("<- ")?;
    clause
        .literals()
        .write_print(output, bank, ", ", true, full_terms, options)?;
    output.write_char('.')
}

#[must_use]
pub fn clause_print_goal_string(bank: &TermBank, clause: &Clause, full_terms: bool) -> String {
    let mut output = String::new();
    let _ = clause_write_goal(
        &mut output,
        bank,
        clause,
        full_terms,
        EqnPrintOptions::default(),
    );
    output
}

/// # Panics
///
/// Panics if `clause` is empty, matching the C `ClausePrintQuery` assertion.
pub fn clause_write_query(
    output: &mut impl fmt::Write,
    bank: &TermBank,
    clause: &Clause,
    full_terms: bool,
    options: EqnPrintOptions,
) -> fmt::Result {
    assert!(
        !clause.is_empty(),
        "ClausePrintQuery requires at least one literal"
    );
    output.write_str("?- ")?;
    clause
        .literals()
        .write_print(output, bank, ", ", true, full_terms, options)?;
    output.write_char('.')
}

/// # Panics
///
/// Panics if `clause` is empty, matching the C `ClausePrintQuery` assertion.
#[must_use]
pub fn clause_print_query_string(bank: &TermBank, clause: &Clause, full_terms: bool) -> String {
    let mut output = String::new();
    let _ = clause_write_query(
        &mut output,
        bank,
        clause,
        full_terms,
        EqnPrintOptions::default(),
    );
    output
}

pub fn clause_write_tptp_format(
    output: &mut impl fmt::Write,
    bank: &TermBank,
    clause: &Clause,
) -> fmt::Result {
    clause_write_tptp_format_with_options(output, bank, clause, EqnPrintOptions::tptp())
}

pub fn clause_write_tptp_format_with_options(
    output: &mut impl fmt::Write,
    bank: &TermBank,
    clause: &Clause,
    options: EqnPrintOptions,
) -> fmt::Result {
    write!(
        output,
        "input_clause({},{},[",
        clause_tptp_identifier(clause),
        clause_tptp_role(clause)
    )?;
    clause
        .literals()
        .write_print(output, bank, ",", false, true, options)?;
    output.write_str("]).")
}

#[must_use]
pub fn clause_print_tptp_format_string(bank: &TermBank, clause: &Clause) -> String {
    clause_print_tptp_format_string_with_options(bank, clause, EqnPrintOptions::tptp())
}

#[must_use]
pub fn clause_print_tptp_format_string_with_options(
    bank: &TermBank,
    clause: &Clause,
    options: EqnPrintOptions,
) -> String {
    let mut output = String::new();
    let _ = clause_write_tptp_format_with_options(&mut output, bank, clause, options);
    output
}

pub fn clause_write_tstp_core(
    output: &mut impl fmt::Write,
    bank: &TermBank,
    clause: &Clause,
    full_terms: bool,
    print_oriented: bool,
) -> fmt::Result {
    clause_write_tstp_core_with_type_suffixes(
        output,
        bank,
        clause,
        full_terms,
        print_oriented,
        false,
    )
}

pub fn clause_write_tstp_core_with_type_suffixes(
    output: &mut impl fmt::Write,
    bank: &TermBank,
    clause: &Clause,
    full_terms: bool,
    print_oriented: bool,
    print_types: bool,
) -> fmt::Result {
    output.write_char('(')?;
    if clause.is_empty() {
        output.write_str("$false")?;
    } else {
        clause.literals().write_tstp_print_with_type_suffixes(
            output,
            bank,
            "|",
            full_terms,
            print_oriented,
            print_types,
        )?;
    }
    output.write_char(')')
}

#[must_use]
pub fn clause_print_tstp_core_string(
    bank: &TermBank,
    clause: &Clause,
    full_terms: bool,
    print_oriented: bool,
) -> String {
    let mut output = String::new();
    let _ = clause_write_tstp_core(&mut output, bank, clause, full_terms, print_oriented);
    output
}

/// Writes the C `ClauseTSTPPrint` shape for represented clause literals.
///
/// # Errors
///
/// Returns a diagnostic if the output writer reports a formatting error.
///
/// # Panics
///
/// Panics if any literal or term violates the C printing preconditions.
pub fn clause_write_tstp(
    output: &mut impl fmt::Write,
    bank: &TermBank,
    clause: &Clause,
    full_terms: bool,
    complete: bool,
    problem_type: ProblemType,
) -> Result<(), Diagnostic> {
    clause_write_tstp_with_type_suffixes(
        output,
        bank,
        clause,
        full_terms,
        complete,
        problem_type,
        false,
    )
}

/// Writes the C `ClauseTSTPPrint` shape with optional term type suffixes.
///
/// # Errors
///
/// Returns a diagnostic under the same conditions as [`clause_write_tstp`].
///
/// # Panics
///
/// Panics if any literal or term violates the C printing preconditions.
pub fn clause_write_tstp_with_type_suffixes(
    output: &mut impl fmt::Write,
    bank: &TermBank,
    clause: &Clause,
    full_terms: bool,
    complete: bool,
    problem_type: ProblemType,
    print_types: bool,
) -> Result<(), Diagnostic> {
    let is_untyped = clause.is_untyped();
    write!(
        output,
        "{}({}, {}, ",
        clause_tstp_kind(is_untyped, problem_type),
        clause_tptp_identifier(clause),
        clause_tstp_role(clause)
    )
    .map_err(tstp_write_error)?;

    if clause.is_empty() || (is_untyped && problem_type != ProblemType::HigherOrder) {
        clause_write_tstp_core_with_type_suffixes(
            output,
            bank,
            clause,
            full_terms,
            false,
            print_types,
        )
        .map_err(tstp_write_error)?;
    } else {
        clause_write_tstp_formula_closure_with_type_suffixes(
            output,
            bank,
            clause,
            full_terms,
            problem_type,
            print_types,
        )
        .map_err(tstp_write_error)?;
    }

    if complete {
        output.write_str(").").map_err(tstp_write_error)?;
    }
    Ok(())
}

/// Returns the C `ClauseTSTPPrint` shape for represented clause literals.
///
/// # Errors
///
/// Returns a diagnostic under the same conditions as [`clause_write_tstp`].
///
/// # Panics
///
/// Panics if any literal or term violates the C printing preconditions.
pub fn clause_tstp_string(
    bank: &TermBank,
    clause: &Clause,
    full_terms: bool,
    complete: bool,
    problem_type: ProblemType,
) -> Result<String, Diagnostic> {
    let mut output = String::new();
    clause_write_tstp(
        &mut output,
        bank,
        clause,
        full_terms,
        complete,
        problem_type,
    )?;
    Ok(output)
}

/// Returns the C `ClausePrint` shape with explicit output-format dispatch.
///
/// C dispatches from the process-global `OutputFormat`: TPTP uses
/// `ClausePrintTPTPFormat`, TSTP uses `ClauseTSTPPrint`, and every other
/// format falls back to LOP printing.
///
/// # Errors
///
/// Returns a diagnostic if TSTP rendering rejects the clause.
///
/// # Panics
///
/// Panics if the selected underlying printer would panic for the given clause,
/// matching the corresponding C assertion path.
pub fn clause_print_format_string(
    bank: &TermBank,
    clause: &Clause,
    full_terms: bool,
    output_format: IoFormat,
    problem_type: ProblemType,
) -> Result<String, Diagnostic> {
    let options = match output_format {
        IoFormat::Tptp => EqnPrintOptions::tptp(),
        IoFormat::Lop | IoFormat::Tstp | IoFormat::Auto => EqnPrintOptions::lop(),
    };
    clause_print_format_string_with_options(
        bank,
        clause,
        full_terms,
        output_format,
        problem_type,
        options,
    )
}

/// Returns the C `ClausePrint` shape with caller-provided equation options.
///
/// # Errors
///
/// Returns a diagnostic if TSTP rendering rejects the clause.
pub fn clause_print_format_string_with_options(
    bank: &TermBank,
    clause: &Clause,
    full_terms: bool,
    output_format: IoFormat,
    problem_type: ProblemType,
    options: EqnPrintOptions,
) -> Result<String, Diagnostic> {
    match output_format {
        IoFormat::Tptp => Ok(clause_print_tptp_format_string_with_options(
            bank, clause, options,
        )),
        IoFormat::Tstp => {
            let mut output = String::new();
            clause_write_tstp_with_type_suffixes(
                &mut output,
                bank,
                clause,
                full_terms,
                true,
                problem_type,
                options.print_types,
            )?;
            Ok(output)
        }
        IoFormat::Lop | IoFormat::Auto => Ok(clause_print_lop_format_string_with_options(
            bank, clause, full_terms, options,
        )),
    }
}

pub fn clause_write_lop_format(
    output: &mut impl fmt::Write,
    bank: &TermBank,
    clause: &Clause,
    full_terms: bool,
    options: EqnPrintOptions,
) -> fmt::Result {
    if matches!(
        clause.query_tptp_type(),
        CP_TYPE_CONJECTURE | CP_TYPE_NEG_CONJECTURE
    ) && !clause.is_empty()
    {
        clause_write_query(output, bank, clause, full_terms, options)
    } else {
        clause_write_axiom(output, bank, clause, full_terms, options)
    }
}

#[must_use]
pub fn clause_print_lop_format_string(
    bank: &TermBank,
    clause: &Clause,
    full_terms: bool,
) -> String {
    clause_print_lop_format_string_with_options(
        bank,
        clause,
        full_terms,
        EqnPrintOptions::default(),
    )
}

#[must_use]
pub fn clause_print_lop_format_string_with_options(
    bank: &TermBank,
    clause: &Clause,
    full_terms: bool,
    options: EqnPrintOptions,
) -> String {
    let mut output = String::new();
    let _ = clause_write_lop_format(&mut output, bank, clause, full_terms, options);
    output
}

pub fn clause_write_pcl(
    output: &mut impl fmt::Write,
    bank: &TermBank,
    clause: &Clause,
    full_terms: bool,
) -> fmt::Result {
    clause_write_pcl_with_options(output, bank, clause, full_terms, EqnPrintOptions::tptp())
}

pub fn clause_write_pcl_with_options(
    output: &mut impl fmt::Write,
    bank: &TermBank,
    clause: &Clause,
    full_terms: bool,
    options: EqnPrintOptions,
) -> fmt::Result {
    output.write_char('[')?;
    clause
        .literals()
        .write_print(output, bank, ",", false, full_terms, options)?;
    output.write_char(']')
}

#[must_use]
pub fn clause_pcl_string(bank: &TermBank, clause: &Clause, full_terms: bool) -> String {
    let mut output = String::new();
    let _ = clause_write_pcl(&mut output, bank, clause, full_terms);
    output
}

pub fn clause_write_debug(
    output: &mut impl fmt::Write,
    bank: &TermBank,
    clause: &Clause,
    problem_type: ProblemType,
) -> fmt::Result {
    write!(output, "thf({}, plain, ", clause_debug_identifier(clause))?;
    if let Some((first, rest)) = clause.literals().as_slice().split_first() {
        eqn_write_debug(output, bank, first, problem_type)?;
        for literal in rest {
            output.write_str(" | ")?;
            eqn_write_debug(output, bank, literal, problem_type)?;
        }
    }
    output.write_str(" ).")
}

#[must_use]
pub fn clause_debug_string(bank: &TermBank, clause: &Clause, problem_type: ProblemType) -> String {
    let mut output = String::new();
    let _ = clause_write_debug(&mut output, bank, clause, problem_type);
    output
}

fn write_literal_tail(
    output: &mut impl fmt::Write,
    bank: &TermBank,
    literals: &[Eqn],
    full_terms: bool,
    options: EqnPrintOptions,
) -> fmt::Result {
    for (index, literal) in literals.iter().enumerate() {
        if index != 0 {
            output.write_str(", ")?;
        }
        eqn_write(output, bank, literal, true, full_terms, options)?;
    }
    Ok(())
}

fn clause_tptp_identifier(clause: &Clause) -> String {
    let source = clause.query_csscpa_source();
    if clause.ident() >= 0 {
        format!("c_{source}_{}", clause.ident())
    } else {
        let offset = i128::from(clause.ident()) - i128::from(i64::MIN);
        format!("i_{source}_{offset}")
    }
}

fn clause_debug_identifier(clause: &Clause) -> String {
    if clause.ident() >= 0 {
        format!("cl{}", clause.ident())
    } else {
        let offset = i128::from(clause.ident()) - i128::from(i64::MIN);
        format!("cl{offset}")
    }
}

fn clause_tptp_role(clause: &Clause) -> &'static str {
    match clause.query_tptp_type() {
        CP_TYPE_AXIOM => "axiom",
        CP_TYPE_HYPOTHESIS => "hypothesis",
        CP_TYPE_CONJECTURE | CP_TYPE_NEG_CONJECTURE => "conjecture",
        CP_TYPE_QUESTION => "question",
        CP_TYPE_LEMMA => "lemma",
        CP_TYPE_WATCH_CLAUSE => "watchlist",
        _ => "unknown",
    }
}

fn clause_tstp_kind(is_untyped: bool, problem_type: ProblemType) -> &'static str {
    if !is_untyped && problem_type == ProblemType::FirstOrder {
        "tcf"
    } else if problem_type == ProblemType::HigherOrder {
        "thf"
    } else {
        "cnf"
    }
}

fn clause_write_tstp_formula_closure_with_type_suffixes(
    output: &mut impl fmt::Write,
    bank: &TermBank,
    clause: &Clause,
    full_terms: bool,
    problem_type: ProblemType,
    print_types: bool,
) -> fmt::Result {
    let mut variables = BTreeMap::new();
    let _ = clause.collect_variables(&mut variables);
    let mut variables: Vec<_> = variables.into_values().collect();
    variables.sort_by_key(|variable| Reverse(variable.f_code()));

    if variables.is_empty() {
        return clause_write_tstp_formula_body_with_type_suffixes(
            output,
            bank,
            clause,
            full_terms,
            print_types,
        );
    }

    output.write_str("![")?;
    for (index, variable) in variables.iter().enumerate() {
        if index != 0 {
            output.write_str(", ")?;
        }
        bank.write_term_with_type_suffixes(output, variable, true, print_types)?;
        let type_ = variable
            .type_()
            .expect("quantified variable printing requires a known type");
        if problem_type == ProblemType::HigherOrder || !type_.is_individual() {
            output.write_char(':')?;
            write_tstp_type(output, bank, &type_, problem_type)?;
        }
    }
    output.write_str("]:(")?;
    clause_write_tstp_formula_body_with_type_suffixes(
        output,
        bank,
        clause,
        full_terms,
        print_types,
    )?;
    output.write_char(')')
}

fn clause_write_tstp_formula_body_with_type_suffixes(
    output: &mut impl fmt::Write,
    bank: &TermBank,
    clause: &Clause,
    full_terms: bool,
    print_types: bool,
) -> fmt::Result {
    let literals = clause.literals().as_slice();
    if literals.len() > 1 {
        output.write_char('(')?;
    }

    let options = EqnFofPrintOptions::tstp().with_print_types(print_types);
    for (index, literal) in literals.iter().enumerate() {
        if index != 0 {
            output.write_char('|')?;
        }
        eqn_write_fof(output, bank, literal, false, full_terms, options)?;
    }

    if literals.len() > 1 {
        output.write_char(')')?;
    }
    Ok(())
}

fn write_tstp_type(
    output: &mut impl fmt::Write,
    bank: &TermBank,
    type_: &Type,
    problem_type: ProblemType,
) -> fmt::Result {
    let mut rendered = Vec::new();
    bank.signature()
        .type_bank()
        .print_tstp(&mut rendered, type_, problem_type)
        .map_err(|_| fmt::Error)?;
    let rendered = String::from_utf8(rendered).map_err(|_| fmt::Error)?;
    output.write_str(&rendered)
}

fn clause_tstp_role(clause: &Clause) -> &'static str {
    match clause.query_tptp_type() {
        CP_TYPE_AXIOM if clause.query_prop(CP_INPUT_FORMULA) => "axiom",
        CP_TYPE_HYPOTHESIS => "hypothesis",
        CP_TYPE_CONJECTURE => "conjecture",
        CP_TYPE_QUESTION => "question",
        CP_TYPE_LEMMA => "lemma",
        CP_TYPE_WATCH_CLAUSE => "watchlist",
        CP_TYPE_NEG_CONJECTURE => "negated_conjecture",
        _ => "plain",
    }
}

fn tstp_write_error(_error: fmt::Error) -> Diagnostic {
    Diagnostic::new(ErrorCode::OTHER_ERROR, "failed to write TSTP clause")
}

fn token_source_string(source: &[u8]) -> String {
    String::from_utf8_lossy(source).into_owned()
}

fn next_clause_ident() -> i64 {
    GLOBAL_CLAUSE_COUNTER
        .fetch_add(1, AtomicOrdering::SeqCst)
        .saturating_add(1)
}

fn cmp_i64(value: i64) -> i32 {
    match value.cmp(&0) {
        Ordering::Less => -1,
        Ordering::Equal => 0,
        Ordering::Greater => 1,
    }
}

fn cmp_i64_to_order(value: i64) -> Ordering {
    value.cmp(&0)
}

fn index_to_i32(index: usize) -> i32 {
    i32::try_from(index).unwrap_or(i32::MAX)
}

fn usize_to_i64(value: usize) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

fn usize_diff(left: usize, right: usize) -> i64 {
    usize_to_i64(left) - usize_to_i64(right)
}

fn clause_polarity_class(clause: &Clause) -> i64 {
    if clause.is_positive() {
        0
    } else if clause.is_negative() {
        2
    } else {
        1
    }
}

fn found_eq_lit_later(
    ocb: &mut OrderControlBlock,
    bank: &TermBank,
    left: &mut Eqn,
    later_right: &mut [Eqn],
) -> bool {
    for right in later_right {
        if !right.has_equiv() && left.literal_compare(ocb, bank, right) == CompareResult::Equal {
            left.set_prop(EP_HAS_EQUIV);
            right.set_prop(EP_HAS_EQUIV);
            return true;
        }
    }
    false
}

fn is_key_subset(left: &BTreeMap<usize, Term>, right: &BTreeMap<usize, Term>) -> bool {
    left.keys().all(|key| right.contains_key(key))
}

fn skolemize_term_in_bank(
    term: &Term,
    subst: &mut Substitution,
    bank: &mut TermBank,
) -> Result<(), Diagnostic> {
    if term.is_free_var() {
        if term.binding().is_none() {
            let type_ = term.type_();
            let skolem = bank.alloc_new_skolem(&[], type_.as_ref())?;
            subst.add_binding(term, &skolem);
        }
    } else {
        for arg in term.argument_clones().into_iter().flatten() {
            skolemize_term_in_bank(&arg, subst, bank)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        clause_debug_string, clause_parse, clause_pcl_parse, clause_pcl_string,
        clause_print_axiom_string, clause_print_format_string, clause_print_goal_string,
        clause_print_lop_format_string, clause_print_query_string, clause_print_rule_string,
        clause_print_tptp_format_string, clause_print_tstp_core_string, clause_starts_maybe,
        clause_tstp_string, clause_write_tstp_with_type_suffixes, Clause,
    };
    use crate::basics::partial_orderings::HoOrderKind;
    use crate::basics::pdarrays::{PDIntArray, GROW_EXPONENTIAL};
    use crate::basics::pstacks::PStack;
    use crate::basics::simple_stuff::{reset_problem_type, set_problem_type, ProblemType};
    use crate::basics::sysdate::SysDate;
    use crate::clauses::clause_props::{
        CP_INITIAL, CP_INPUT_FORMULA, CP_IS_D_INDEXED, CP_IS_ORIENTED, CP_IS_SOS, CP_TYPE_AXIOM,
        CP_TYPE_CONJECTURE, CP_TYPE_HYPOTHESIS, CP_TYPE_NEG_CONJECTURE,
    };
    use crate::clauses::clauseinfo::ClauseInfo;
    use crate::clauses::derivation::{DerivationEntry, DC_CNF_EVAL_GC};
    use crate::clauses::eqn::Eqn;
    use crate::clauses::eqn_props::{
        EqnSide, EP_DOMINATES, EP_HAS_EQUIV, EP_IS_DOMINATED, EP_IS_MAXIMAL, EP_IS_ORIENTED,
        EP_PSEUDO_LIT,
    };
    use crate::clauses::eqnlist::EqnList;
    use crate::clauses::neweval::evals_alloc;
    use crate::heuristics::to_params::TermOrdering;
    use crate::inout::scanner::{IoFormat, Scanner};
    use crate::orderings::ocb::OrderControlBlock;
    use crate::terms::lambda::{apply_terms, close_with_db_var};
    use crate::terms::signature::{Signature, FP_ASSOCIATIVE, FP_COMMUTATIVE, FP_SKOLEM_SYMBOL};
    use crate::terms::simpletypes::{alloc_arrow_type, alloc_simple_sort, Type};
    use crate::terms::subst::Substitution;
    use crate::terms::termbanks::TermBank;
    use crate::terms::termtypes::{DerefType, Term, TP_OP_FLAG, TP_SPECIAL_FLAG};
    use crate::terms::termvars::VarBank;
    use crate::terms::termweightext::{TermWeightExtension, TermWeightExtensionStyle};
    use crate::terms::typebanks::TypeBank;
    use crate::test_support::global_state_lock;
    use std::collections::{BTreeMap, BTreeSet};

    fn test_bank() -> TermBank {
        let mut signature = Signature::new(TypeBank::new());
        signature.insert_internal_codes().unwrap();
        TermBank::new(signature).unwrap()
    }

    struct ProblemTypeReset;

    impl Drop for ProblemTypeReset {
        fn drop(&mut self) {
            reset_problem_type();
        }
    }

    fn set_problem_type_for_test(problem_type: ProblemType) -> ProblemTypeReset {
        reset_problem_type();
        set_problem_type(problem_type).unwrap_or_else(|err| panic!("{err}"));
        ProblemTypeReset
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

    fn typed_var_with_type(bank: &TermBank, f_code: i64, type_: &Type) -> Term {
        bank.vars().var_assert_alloc(f_code, type_)
    }

    fn typed_const_with_type(bank: &mut TermBank, name: &str, type_: &Type) -> Term {
        let f_code = bank.signature_mut().insert_id(name, 0, false);
        bank.signature_mut()
            .declare_final_type(f_code, type_.clone())
            .unwrap();
        let term = Term::const_cell_alloc(f_code);
        term.set_type(Some(type_.clone()));
        bank.insert(&term, DerefType::Never).unwrap()
    }

    fn typed_pred_unary_with_arg_type(bank: &mut TermBank, name: &str, arg: &Term) -> Term {
        let arg_type = arg.type_().expect("test argument has a type");
        let bool_type = bank.signature().type_bank().bool_type();
        let f_code = bank.signature_mut().insert_id(name, 1, false);
        let predicate_type = bank
            .signature_mut()
            .type_bank_mut()
            .insert_type_shared(alloc_arrow_type(vec![arg_type, bool_type.clone()]));
        bank.signature_mut()
            .declare_final_type(f_code, predicate_type)
            .unwrap();
        let term = Term::top_alloc(f_code, 1);
        term.set_type(Some(bool_type));
        term.set_argument(0, arg.clone());
        bank.insert(&term, DerefType::Never).unwrap()
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

    fn kbo_ocb(bank: &TermBank) -> OrderControlBlock {
        OrderControlBlock::alloc(
            TermOrdering::Kbo,
            true,
            bank.signature(),
            HoOrderKind::LfhoOrder,
        )
    }

    fn kbo6_lambda_ocb(bank: &TermBank) -> OrderControlBlock {
        OrderControlBlock::alloc(
            TermOrdering::Kbo6,
            true,
            bank.signature(),
            HoOrderKind::LambdaOrder,
        )
    }

    #[test]
    fn allocation_sorts_positive_literals_before_negative_literals() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let c = typed_const(&mut bank, "c");
        let negative = eqn(&mut bank, &a, &b, false);
        let positive = eqn(&mut bank, &b, &c, true);

        let clause = Clause::alloc(EqnList::from_vec(vec![negative.clone(), positive.clone()]));

        assert_eq!(clause.positive_literal_count(), 1);
        assert_eq!(clause.negative_literal_count(), 1);
        assert!(clause.literals().as_slice()[0].is_positive());
        assert!(clause.literals().as_slice()[1].is_negative());
        assert_eq!(clause.literals().as_slice(), &[positive, negative]);
        assert!(clause.ident() > i64::MIN);
        assert_eq!(clause.date(), SysDate::creation_time());
        assert!(clause.evaluations().is_none());
    }

    #[test]
    fn mark_maximal_terms_orients_literals_and_marks_clause() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let f_a = typed_unary(&mut bank, "f", &a);
        let mut clause = Clause::alloc(EqnList::from_vec(vec![
            eqn(&mut bank, &a, &f_a, true),
            eqn(&mut bank, &a, &a, true),
        ]));
        let mut ocb = kbo_ocb(&bank);

        clause.mark_maximal_terms(&mut ocb, &bank);

        assert!(clause.query_prop(CP_IS_ORIENTED));
        assert!(clause.literals().as_slice()[0].is_oriented());
        assert_eq!(clause.literals().as_slice()[0].left(), &f_a);
        assert!(clause.literals().as_slice()[0].is_maximal());
        assert!(!clause.literals().as_slice()[1].is_maximal());
    }

    #[test]
    fn mark_maximal_terms_with_bank_accepts_lambda_order_beta_surface() {
        let _guard = global_state_lock();
        let _problem_type = set_problem_type_for_test(ProblemType::HigherOrder);
        let mut bank = test_bank();
        let binder_type = bank.signature().type_bank().default_type();
        let db0 = bank.request_db_var(&binder_type, 0);
        let lambda =
            close_with_db_var(&mut bank, &binder_type, &db0).unwrap_or_else(|err| panic!("{err}"));
        let arg = typed_const(&mut bank, "clause_lambda_order_arg");
        let applied = apply_terms(&mut bank, &lambda, std::slice::from_ref(&arg))
            .unwrap_or_else(|err| panic!("{err}"));
        let mut clause = Clause::alloc(EqnList::from_vec(vec![eqn(
            &mut bank, &applied, &arg, true,
        )]));
        let mut ocb = kbo6_lambda_ocb(&bank);

        clause
            .mark_maximal_terms_with_bank(&mut ocb, &mut bank)
            .unwrap_or_else(|err| panic!("{err}"));

        assert!(clause.query_prop(CP_IS_ORIENTED));
        assert!(clause.literals().as_slice()[0].is_maximal());
        assert!(!clause.literals().as_slice()[0].is_oriented());
    }

    #[test]
    fn clause_ordering_wrappers_match_c_macro_behaviour() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let f_a = typed_unary(&mut bank, "f", &a);
        let mut clause = Clause::alloc(EqnList::from_vec(vec![
            eqn(&mut bank, &a, &f_a, true),
            eqn(&mut bank, &a, &a, true),
        ]));
        let mut ocb = kbo_ocb(&bank);

        assert_eq!(clause.orient_literals(&mut ocb, &bank), 1);
        assert!(!clause.query_prop(CP_IS_ORIENTED));
        assert!(clause.literals().as_slice()[0].is_oriented());
        assert_eq!(clause.literals().as_slice()[0].left(), &f_a);

        assert_eq!(clause.mark_maximal_literals(&mut ocb, &bank), 1);
        assert!(clause.literals().as_slice()[0].is_maximal());
        assert!(!clause.literals().as_slice()[1].is_maximal());

        let mut conditional = Clause::alloc(EqnList::from_vec(vec![
            eqn(&mut bank, &a, &f_a, true),
            eqn(&mut bank, &a, &a, true),
        ]));
        assert!(conditional.cond_mark_maximal_terms(&mut ocb, &bank));
        assert!(conditional.query_prop(CP_IS_ORIENTED));
        assert_eq!(conditional.literals().as_slice()[0].left(), &f_a);
        assert!(conditional.literals().as_slice()[0].is_maximal());

        let mut stale_flag = Clause::alloc(EqnList::from_vec(vec![eqn(&mut bank, &a, &f_a, true)]));
        stale_flag.set_prop(CP_IS_ORIENTED);
        assert!(!stale_flag.cond_mark_maximal_terms(&mut ocb, &bank));
        assert_eq!(stale_flag.literals().as_slice()[0].left(), &a);
        assert!(!stale_flag.literals().as_slice()[0].is_maximal());
    }

    #[test]
    fn not_greater_equal_preserves_c_multiset_scan_and_flags() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let f_a = typed_unary(&mut bank, "f", &a);
        let mut ocb = kbo_ocb(&bank);

        let mut greater = Clause::alloc(EqnList::from_vec(vec![eqn(&mut bank, &f_a, &a, true)]));
        let mut lesser = Clause::alloc(EqnList::from_vec(vec![eqn(&mut bank, &a, &a, true)]));
        assert!(!greater.not_greater_equal(&mut ocb, &bank, &mut lesser));
        assert!(greater.literals().as_slice()[0].query_prop(EP_DOMINATES));
        assert!(lesser.literals().as_slice()[0].query_prop(EP_IS_DOMINATED));

        let mut equal_left = Clause::alloc(EqnList::from_vec(vec![eqn(&mut bank, &f_a, &a, true)]));
        let mut equal_right =
            Clause::alloc(EqnList::from_vec(vec![eqn(&mut bank, &f_a, &a, true)]));
        assert!(!equal_left.not_greater_equal(&mut ocb, &bank, &mut equal_right));
        assert!(equal_left.literals().as_slice()[0].query_prop(EP_HAS_EQUIV));
        assert!(equal_right.literals().as_slice()[0].query_prop(EP_HAS_EQUIV));

        let mut smaller = Clause::alloc(EqnList::from_vec(vec![eqn(&mut bank, &a, &a, true)]));
        let mut larger = Clause::alloc(EqnList::from_vec(vec![eqn(&mut bank, &f_a, &a, true)]));
        assert!(smaller.not_greater_equal(&mut ocb, &bank, &mut larger));

        let mut reserved_for_later =
            Clause::alloc(EqnList::from_vec(vec![eqn(&mut bank, &f_a, &a, true)]));
        let mut later_equal = Clause::alloc(EqnList::from_vec(vec![
            eqn(&mut bank, &a, &a, true),
            eqn(&mut bank, &f_a, &a, true),
        ]));
        assert!(reserved_for_later.not_greater_equal(&mut ocb, &bank, &mut later_equal));
        assert!(reserved_for_later.literals().as_slice()[0].query_prop(EP_HAS_EQUIV));
        assert!(later_equal.literals().as_slice()[1].query_prop(EP_HAS_EQUIV));
    }

    #[test]
    fn clause_print_strings_match_c_lop_tptp_and_pcl_shapes() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "print_a");
        let b = typed_const(&mut bank, "print_b");
        let p = typed_pred_const(&mut bank, "print_p");
        let true_term = bank.true_term().clone();
        let positive_equality = eqn(&mut bank, &a, &b, true);
        let positive_predicate = eqn(&mut bank, &p, &true_term, true);
        let negative_equality = eqn(&mut bank, &b, &a, false);
        let mut clause = Clause::alloc(EqnList::from_vec(vec![
            negative_equality,
            positive_equality,
            positive_predicate,
        ]));
        clause.set_ident(77);
        clause.set_csscpa_source(5);

        assert_eq!(
            clause_print_axiom_string(&bank, &clause, true),
            "print_a=print_b; print_p <- print_b=print_a."
        );
        assert_eq!(
            clause_print_rule_string(&bank, &clause, true),
            "print_a=print_b <- ~print_p, print_b=print_a."
        );
        assert_eq!(
            clause_print_goal_string(&bank, &clause, true),
            "<- print_a!=print_b, ~print_p, print_b=print_a."
        );
        assert_eq!(
            clause_print_query_string(&bank, &clause, true),
            "?- print_a!=print_b, ~print_p, print_b=print_a."
        );

        clause.set_tptp_type(CP_TYPE_NEG_CONJECTURE);
        assert_eq!(
            clause_print_lop_format_string(&bank, &clause, true),
            "?- print_a!=print_b, ~print_p, print_b=print_a."
        );
        assert_eq!(
            clause_print_tptp_format_string(&bank, &clause),
            "input_clause(c_5_77,conjecture,[++equal(print_a, print_b),++print_p,--equal(print_b, print_a)])."
        );
        assert_eq!(
            clause_print_format_string(
                &bank,
                &clause,
                true,
                IoFormat::Lop,
                ProblemType::FirstOrder
            )
            .unwrap(),
            clause_print_lop_format_string(&bank, &clause, true)
        );
        assert_eq!(
            clause_print_format_string(
                &bank,
                &clause,
                true,
                IoFormat::Auto,
                ProblemType::FirstOrder
            )
            .unwrap(),
            clause_print_lop_format_string(&bank, &clause, true)
        );
        assert_eq!(
            clause_print_format_string(
                &bank,
                &clause,
                true,
                IoFormat::Tptp,
                ProblemType::FirstOrder
            )
            .unwrap(),
            clause_print_tptp_format_string(&bank, &clause)
        );
        assert_eq!(
            clause_print_format_string(
                &bank,
                &clause,
                true,
                IoFormat::Tstp,
                ProblemType::FirstOrder
            )
            .unwrap(),
            clause_tstp_string(&bank, &clause, true, true, ProblemType::FirstOrder).unwrap()
        );
        assert_eq!(
            clause_print_tstp_core_string(&bank, &clause, true, false),
            "(print_a=print_b|print_p|print_b!=print_a)"
        );
        clause.literals_mut().as_mut_slice()[0].set_prop(EP_IS_ORIENTED);
        assert_eq!(
            clause_print_tstp_core_string(&bank, &clause, true, true),
            "(print_a->print_b|print_p|print_b!=print_a)"
        );
        assert_eq!(
            clause_pcl_string(&bank, &clause, true),
            "[++equal(print_a, print_b),++print_p,--equal(print_b, print_a)]"
        );
        assert_eq!(
            clause_print_tstp_core_string(&bank, &Clause::empty(), true, false),
            "($false)"
        );
    }

    #[test]
    fn clause_debug_string_matches_c_dbg_shape() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "dbg_a");
        let b = typed_const(&mut bank, "dbg_b");
        let positive = eqn(&mut bank, &a, &b, true);
        let negative = eqn(&mut bank, &b, &a, false);
        let mut clause = Clause::alloc(EqnList::from_vec(vec![negative, positive]));
        clause.set_ident(42);

        assert_eq!(
            clause_debug_string(&bank, &clause, ProblemType::FirstOrder),
            "thf(cl42, plain, dbg_a=dbg_b%% | dbg_b!=dbg_a%% )."
        );

        let mut long_min_clause = Clause::empty();
        long_min_clause.set_ident(i64::MIN + 3);
        assert_eq!(
            clause_debug_string(&bank, &long_min_clause, ProblemType::FirstOrder),
            "thf(cl3, plain,  )."
        );
    }

    #[test]
    fn clause_tstp_string_wraps_supported_core_branch_like_c() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "tstp_a");
        let b = typed_const(&mut bank, "tstp_b");
        let positive = eqn(&mut bank, &a, &b, true);
        let negative = eqn(&mut bank, &b, &a, false);
        let mut clause = Clause::alloc(EqnList::from_vec(vec![negative, positive]));
        clause.set_ident(77);
        clause.set_csscpa_source(5);
        clause.set_tptp_type(CP_TYPE_AXIOM);

        assert_eq!(
            clause_tstp_string(&bank, &clause, true, true, ProblemType::FirstOrder).unwrap(),
            "cnf(c_5_77, plain, (tstp_a=tstp_b|tstp_b!=tstp_a))."
        );

        clause.set_prop(CP_INPUT_FORMULA);
        assert_eq!(
            clause_tstp_string(&bank, &clause, true, true, ProblemType::FirstOrder).unwrap(),
            "cnf(c_5_77, axiom, (tstp_a=tstp_b|tstp_b!=tstp_a))."
        );

        clause.set_tptp_type(CP_TYPE_NEG_CONJECTURE);
        assert_eq!(
            clause_tstp_string(&bank, &clause, true, false, ProblemType::FirstOrder).unwrap(),
            "cnf(c_5_77, negated_conjecture, (tstp_a=tstp_b|tstp_b!=tstp_a)"
        );

        let mut empty = Clause::empty();
        empty.set_tptp_type(CP_TYPE_HYPOTHESIS);
        assert_eq!(
            clause_tstp_string(&bank, &empty, true, true, ProblemType::HigherOrder).unwrap(),
            "thf(c_0_0, hypothesis, ($false))."
        );

        assert_eq!(
            clause_tstp_string(&bank, &clause, true, true, ProblemType::HigherOrder).unwrap(),
            "thf(c_5_77, negated_conjecture, (tstp_a=tstp_b|tstp_b!=tstp_a))."
        );
    }

    #[test]
    fn clause_tstp_string_closes_typed_first_order_clause_like_c() {
        let mut bank = test_bank();
        let person_code = bank
            .signature_mut()
            .type_bank_mut()
            .define_simple_sort("person")
            .unwrap();
        let person = bank
            .signature_mut()
            .type_bank_mut()
            .insert_type_shared(alloc_simple_sort(person_code));
        let x = typed_var(&bank, -2);
        let y = typed_var_with_type(&bank, -4, &person);
        let p_x = typed_pred_unary_with_arg_type(&mut bank, "typed_p", &x);
        let q_y = typed_pred_unary_with_arg_type(&mut bank, "typed_q", &y);
        let mut clause = Clause::alloc(EqnList::from_vec(vec![
            Eqn::alloc_flatten(p_x, &mut bank, true).unwrap(),
            Eqn::alloc_flatten(q_y, &mut bank, true).unwrap(),
        ]));
        clause.set_ident(13);
        clause.set_csscpa_source(2);
        clause.set_tptp_type(CP_TYPE_AXIOM);
        clause.set_prop(CP_INPUT_FORMULA);

        assert_eq!(
            clause_tstp_string(&bank, &clause, true, true, ProblemType::FirstOrder).unwrap(),
            "tcf(c_2_13, axiom, ![X1, X2:person]:((typed_p(X1)|typed_q(X2))))."
        );

        let mut typed_output = String::new();
        clause_write_tstp_with_type_suffixes(
            &mut typed_output,
            &bank,
            &clause,
            true,
            true,
            ProblemType::FirstOrder,
            true,
        )
        .unwrap();
        assert_eq!(
            typed_output,
            "tcf(c_2_13, axiom, ![X1:$i, X2:person:person]:((typed_p(X1:$i):$o|typed_q(X2:person):$o)))."
        );

        assert_eq!(
            clause_tstp_string(&bank, &clause, true, true, ProblemType::HigherOrder).unwrap(),
            "thf(c_2_13, axiom, ![X1:$i, X2:person]:((typed_p(X1)|typed_q(X2))))."
        );
    }

    #[test]
    fn clause_tstp_string_prints_ground_typed_formula_without_core_parens() {
        let mut bank = test_bank();
        let person_code = bank
            .signature_mut()
            .type_bank_mut()
            .define_simple_sort("person")
            .unwrap();
        let person = bank
            .signature_mut()
            .type_bank_mut()
            .insert_type_shared(alloc_simple_sort(person_code));
        let alice = typed_const_with_type(&mut bank, "alice", &person);
        let q_alice = typed_pred_unary_with_arg_type(&mut bank, "holds", &alice);
        let mut clause = Clause::alloc(EqnList::from_vec(vec![Eqn::alloc_flatten(
            q_alice, &mut bank, true,
        )
        .unwrap()]));
        clause.set_ident(14);
        clause.set_csscpa_source(2);

        assert_eq!(
            clause_tstp_string(&bank, &clause, true, true, ProblemType::FirstOrder).unwrap(),
            "tcf(c_2_14, plain, holds(alice))."
        );
    }

    #[test]
    fn clause_starts_maybe_matches_c_token_and_lookahead_rule() {
        let term = Scanner::from_user_string("p.", false).unwrap();
        let query = Scanner::from_user_string("?- p.", false).unwrap();
        let spaced_query = Scanner::from_user_string("? - p.", false).unwrap();
        let other = Scanner::from_user_string(").", false).unwrap();

        assert!(clause_starts_maybe(&term));
        assert!(clause_starts_maybe(&query));
        assert!(!clause_starts_maybe(&spaced_query));
        assert!(!clause_starts_maybe(&other));
    }

    #[test]
    fn clause_parse_reads_lop_rules_queries_and_facts() {
        let mut bank = test_bank();
        let mut rule = Scanner::from_user_string("p(a) <- q(a), r(a). tail", false).unwrap();
        rule.set_format(IoFormat::Lop);
        let parsed_rule = clause_parse(&mut rule, &mut bank, ProblemType::FirstOrder).unwrap();

        assert_eq!(
            clause_print_lop_format_string(&bank, &parsed_rule, true),
            "p(a) <- q(a), r(a)."
        );
        assert_eq!(parsed_rule.query_tptp_type(), CP_TYPE_AXIOM);
        assert!(parsed_rule.query_prop(CP_INITIAL | CP_INPUT_FORMULA));
        assert_eq!(
            parsed_rule.info().and_then(ClauseInfo::source),
            Some("p(a) <- q(a), r(a). tail")
        );
        assert_eq!(rule.current_token().literal(), "tail");

        let mut query = Scanner::from_user_string("?- goal(a).", false).unwrap();
        query.set_format(IoFormat::Lop);
        let parsed_query = clause_parse(&mut query, &mut bank, ProblemType::FirstOrder).unwrap();
        assert_eq!(parsed_query.query_tptp_type(), CP_TYPE_NEG_CONJECTURE);
        assert_eq!(
            clause_print_lop_format_string(&bank, &parsed_query, true),
            "?- goal(a)."
        );

        let mut invalid = Scanner::from_user_string("p(a); q(a).", false).unwrap();
        invalid.set_format(IoFormat::Lop);
        let error = clause_parse(&mut invalid, &mut bank, ProblemType::FirstOrder).unwrap_err();
        assert!(error
            .message()
            .contains("Procedural fact cannot have more than one literal"));
    }

    #[test]
    fn clause_parse_reads_old_tptp_and_tstp_wrappers() {
        let mut bank = test_bank();
        let mut old_tptp =
            Scanner::from_user_string("input_clause(c_0_1,conjecture,[--p(a)]).", false).unwrap();
        old_tptp.set_format(IoFormat::Tptp);
        let parsed_tptp = clause_parse(&mut old_tptp, &mut bank, ProblemType::FirstOrder).unwrap();

        assert_eq!(parsed_tptp.query_tptp_type(), CP_TYPE_NEG_CONJECTURE);
        assert_eq!(parsed_tptp.info().and_then(ClauseInfo::name), Some("c_0_1"));
        assert_eq!(
            clause_print_lop_format_string(&bank, &parsed_tptp, true),
            "?- p(a)."
        );

        let mut tstp = Scanner::from_user_string(
            "cnf(c2, negated_conjecture, (p(a)|~q(a)), file('x.p',unknown)).",
            false,
        )
        .unwrap();
        tstp.set_format(IoFormat::Tstp);
        let parsed_tstp = clause_parse(&mut tstp, &mut bank, ProblemType::FirstOrder).unwrap();

        assert_eq!(parsed_tstp.query_tptp_type(), CP_TYPE_NEG_CONJECTURE);
        assert_eq!(parsed_tstp.info().and_then(ClauseInfo::name), Some("c2"));
        assert_eq!(parsed_tstp.literal_number(), 2);
        assert_eq!(parsed_tstp.positive_literal_count(), 1);
        assert_eq!(parsed_tstp.negative_literal_count(), 1);
    }

    #[test]
    fn clause_pcl_parse_reads_tptp_literal_lists_and_sets_type_by_polarity() {
        let mut bank = test_bank();
        let mut scanner = Scanner::from_user_string("[++p(a),--q(a)] tail", false).unwrap();
        scanner.set_format(IoFormat::Tptp);

        let parsed = clause_pcl_parse(&mut scanner, &mut bank, ProblemType::FirstOrder).unwrap();

        assert_eq!(parsed.query_tptp_type(), CP_TYPE_AXIOM);
        assert_eq!(parsed.positive_literal_count(), 1);
        assert_eq!(parsed.negative_literal_count(), 1);
        assert_eq!(scanner.current_token().literal(), "tail");

        let mut negative = Scanner::from_user_string("[--goal(a)]", false).unwrap();
        negative.set_format(IoFormat::Tptp);
        let parsed_negative =
            clause_pcl_parse(&mut negative, &mut bank, ProblemType::FirstOrder).unwrap();
        assert_eq!(parsed_negative.query_tptp_type(), CP_TYPE_CONJECTURE);
    }

    #[test]
    fn property_and_metadata_helpers_follow_clause_macros() {
        let mut clause = Clause::empty();

        clause.set_ident(42);
        clause.set_date(SysDate::from_raw(7));
        clause.set_prop(CP_INITIAL | CP_INPUT_FORMULA);
        clause.set_tptp_type(CP_TYPE_HYPOTHESIS);
        clause.set_csscpa_source(3);
        clause.set_weight(11);
        clause.set_info(Some(ClauseInfo::new(Some("c"), Some("file.p"), 1, 2)));
        clause.set_create_date(5);
        clause.set_proof_depth(6);
        clause.set_proof_size(9);

        assert_eq!(clause.ident(), 42);
        assert_eq!(clause.date(), SysDate::from_raw(7));
        assert!(clause.query_prop(CP_INITIAL));
        assert!(clause.is_any_prop_set(CP_INITIAL | CP_IS_SOS));
        assert_eq!(clause.any_prop_set(CP_INITIAL | CP_IS_SOS), CP_INITIAL);
        assert_eq!(clause.give_props(CP_INITIAL | CP_IS_SOS), CP_INITIAL);
        assert!(clause.is_hypothesis());
        assert_eq!(clause.query_csscpa_source(), 3);
        assert_eq!(clause.weight(), 11);
        assert_eq!(clause.info().and_then(ClauseInfo::name), Some("c"));
        assert_eq!(clause.create_date(), 5);
        assert_eq!(clause.proof_depth(), 6);
        assert_eq!(clause.proof_size(), 9);

        clause.del_prop(CP_INITIAL);
        assert!(!clause.query_prop(CP_INITIAL));
        clause.set_properties(CP_IS_SOS);
        assert!(clause.is_sos());
    }

    #[test]
    fn evaluation_storage_attach_take_and_remove_match_clause_shape() {
        let mut clause = Clause::empty();
        let mut evaluations = evals_alloc(2);
        evaluations.eval_mut(0).set_heuristic(1.5);
        evaluations.eval_mut(1).set_heuristic(2.5);

        clause.add_eval_cell_with_object(evaluations, Some(17));

        let stored = clause.evaluations().expect("evaluation cell is attached");
        assert_eq!(stored.eval_no(), 2);
        assert_eq!(stored.object(), Some(17));
        assert_eq!(stored.eval(1).heuristic().to_bits(), 2.5_f32.to_bits());

        let mut taken = clause.take_evaluations().expect("evaluation cell is taken");
        assert!(clause.evaluations().is_none());
        taken.set_object(Some(19));
        clause.add_eval_cell(taken);
        assert!(matches!(
            clause.evaluations(),
            Some(evaluation) if evaluation.object().is_none()
        ));

        clause.remove_evaluations();
        assert!(clause.evaluations().is_none());
    }

    #[test]
    fn basic_classification_uses_cached_literal_counts() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let positive = Clause::alloc(EqnList::from_vec(vec![eqn(&mut bank, &a, &b, true)]));
        let negative = Clause::alloc(EqnList::from_vec(vec![eqn(&mut bank, &a, &b, false)]));
        let mixed = Clause::alloc(EqnList::from_vec(vec![
            eqn(&mut bank, &a, &b, true),
            eqn(&mut bank, &a, &b, false),
        ]));

        assert!(Clause::empty().is_empty());
        assert!(Clause::empty().is_goal());
        assert!(positive.is_unit());
        assert!(positive.is_horn());
        assert!(positive.is_demodulator());
        assert!(!positive.is_rw_rule());
        assert!(positive.is_positive());
        assert!(!positive.is_negative());
        assert!(negative.is_negative());
        assert!(mixed.is_mixed());
        assert_eq!(mixed.literal_number(), 2);
    }

    #[test]
    fn semantic_and_triviality_checks_delegate_to_literals() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let answer = answer_term(&mut bank, &a);
        let mut answer_lit = Eqn::alloc(answer, bank.true_term().clone(), &mut bank, true).unwrap();
        let mut pseudo = eqn(&mut bank, &a, &b, true);
        pseudo.set_prop(EP_PSEUDO_LIT);
        let true_lit = eqn(&mut bank, &a, &a, true);
        let pos = eqn(&mut bank, &a, &b, true);
        let neg = eqn(&mut bank, &b, &a, false);

        assert!(Clause::alloc(EqnList::from_vec(vec![pseudo.clone()])).is_sem_false());
        assert!(Clause::alloc(EqnList::from_vec(vec![answer_lit.clone()])).is_sem_empty(&bank));
        assert!(Clause::alloc(EqnList::from_vec(vec![true_lit])).is_trivial(&bank));
        assert!(Clause::alloc(EqnList::from_vec(vec![pos, neg])).is_trivial(&bank));

        answer_lit.set_prop(EP_PSEUDO_LIT);
        let mut answer_clause = Clause::alloc(EqnList::from_vec(vec![answer_lit, pseudo]));
        assert_eq!(answer_clause.evaluate_answer_literals(&bank), 1);
        assert_eq!(answer_clause.literal_number(), 1);
        assert_eq!(answer_clause.positive_literal_count(), 1);
    }

    #[test]
    fn answer_output_string_prints_szs_tuple_shape() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "answer_a");
        let b = typed_const(&mut bank, "answer_b");
        let ans_a = typed_unary(&mut bank, "answer_payload_a", &a);
        let ans_b = typed_unary(&mut bank, "answer_payload_b", &b);
        let answer_a = answer_term(&mut bank, &ans_a);
        let answer_b = answer_term(&mut bank, &ans_b);
        let truth = bank.true_term().clone();
        let literal_a = Eqn::alloc(answer_a, truth.clone(), &mut bank, true).unwrap();
        let literal_b = Eqn::alloc(answer_b, truth, &mut bank, true).unwrap();
        let clause = Clause::alloc(EqnList::from_vec(vec![literal_a, literal_b]));

        assert_eq!(
            clause.answer_output_string(&bank).as_deref(),
            Some("% SZS answers Tuple [([answer_a]|[answer_b])|_]\n")
        );
    }

    #[test]
    fn range_restriction_variants_compare_positive_and_negative_variables() {
        let mut bank = test_bank();
        let x = typed_var(&bank, -10);
        let y = typed_var(&bank, -12);
        let a = typed_const(&mut bank, "a");
        let pos_x = eqn(&mut bank, &x, &a, true);
        let neg_x = eqn(&mut bank, &x, &a, false);
        let neg_y = eqn(&mut bank, &y, &a, false);

        let restricted = Clause::alloc(EqnList::from_vec(vec![pos_x.clone(), neg_x.clone()]));
        assert!(restricted.is_range_restricted());
        assert!(restricted.is_anti_range_restricted());
        assert!(restricted.is_strongly_range_restricted());

        let not_restricted = Clause::alloc(EqnList::from_vec(vec![pos_x, neg_y]));
        assert!(!not_restricted.is_range_restricted());
        assert!(!not_restricted.is_anti_range_restricted());
        assert!(!Clause::alloc(EqnList::from_vec(vec![neg_x])).is_range_restricted());
    }

    #[test]
    fn canonicalization_and_structural_comparison_match_literal_ordering() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let c = typed_const(&mut bank, "c");
        let mut clause = Clause::alloc(EqnList::from_vec(vec![
            eqn(&mut bank, &c, &b, false),
            eqn(&mut bank, &b, &a, true),
        ]));

        clause.canonize(&bank);
        assert!(clause.is_sorted_by(|left, right| left.struct_weight_lex_compare(right, &bank)));
        assert_eq!(clause.literals().as_slice()[0].position(), 0);

        let mut other = clause.clone();
        other.set_ident(clause.ident() + 10);
        other.set_weight(other.standard_weight());
        clause.set_weight(clause.standard_weight());
        assert!(clause.struct_weight_lex_compare(&other, &bank) < 0);
        assert_eq!(clause.cmp_by_struct_weight(&other, &bank), -1);
    }

    #[test]
    fn subsume_order_wrappers_sort_and_check_like_c_macros() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let f_a = typed_unary(&mut bank, "f", &a);
        let mut clause = Clause::alloc(EqnList::from_vec(vec![
            eqn(&mut bank, &a, &a, true),
            eqn(&mut bank, &f_a, &a, true),
        ]));

        assert!(!clause.is_subsume_ordered(&bank));
        clause.subsume_order_sort_literals(&bank);

        assert!(clause.is_subsume_ordered(&bank));
        assert_eq!(clause.literals().as_slice()[0].left(), &f_a);
        assert_eq!(clause.literals().as_slice()[0].position(), 1);
        assert_eq!(clause.literals().as_slice()[1].position(), 0);
    }

    #[test]
    fn copy_helpers_preserve_metadata_but_drop_source_info_like_c() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let x = typed_var(&bank, -10);
        let mut clause = Clause::alloc(EqnList::from_vec(vec![
            eqn(&mut bank, &a, &b, true),
            eqn(&mut bank, &x, &b, false),
        ]));
        clause.set_tptp_type(CP_TYPE_AXIOM);
        clause.set_info(Some(ClauseInfo::new(Some("input"), None, -1, -1)));
        clause.set_create_date(7);
        clause.add_eval_cell_with_object(evals_alloc(1), Some(23));
        clause
            .ensure_derivation()
            .push(DerivationEntry::Operation(DC_CNF_EVAL_GC));

        let flat = clause.flat_copy(&mut bank).unwrap();
        assert_eq!(flat.ident(), clause.ident());
        assert_eq!(flat.query_tptp_type(), CP_TYPE_AXIOM);
        assert!(flat.info().is_none());
        assert!(flat.evaluations().is_none());
        assert!(flat.derivation().is_none());
        assert_eq!(flat.literals(), clause.literals());

        let copied = clause.copy_opt(&mut bank).unwrap();
        assert_eq!(copied.literal_number(), clause.literal_number());
        assert!(copied.evaluations().is_none());
        assert!(copied.derivation().is_none());
        let disjoint = clause.copy_disjoint(&mut bank).unwrap();
        assert_ne!(disjoint.literals().as_slice()[1].left(), &x);
        assert!(disjoint.evaluations().is_none());
        assert!(disjoint.derivation().is_none());
    }

    #[test]
    fn skolemize_copy_uses_temporary_bindings_and_backtracks_like_c() {
        let mut bank = test_bank();
        let x = typed_var(&bank, -10);
        let f_x = typed_unary(&mut bank, "f", &x);
        let source = Clause::alloc(EqnList::from_vec(vec![eqn(&mut bank, &f_x, &x, true)]));

        let skolemized = source.skolemize(&mut bank).unwrap();

        assert!(x.binding().is_none());
        assert_eq!(source.literals().as_slice()[0].right(), &x);

        let literal = &skolemized.literals().as_slice()[0];
        let skolem = literal.right();
        assert!(!skolem.is_any_var());
        assert!(bank
            .signature()
            .query_prop(skolem.f_code(), FP_SKOLEM_SYMBOL));
        assert_eq!(skolem.type_(), x.type_());
        assert_eq!(literal.left().argument(0).as_ref(), Some(skolem));
    }

    #[test]
    fn weight_and_collection_wrappers_sum_literal_helpers() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let f_of_a = typed_unary(&mut bank, "f", &a);
        let mut positive = eqn(&mut bank, &f_of_a, &b, true);
        positive.set_prop(EP_IS_MAXIMAL);
        let negative = eqn(&mut bank, &a, &b, false);
        let clause = Clause::alloc(EqnList::from_vec(vec![positive, negative]));

        assert_eq!(
            clause.standard_weight(),
            clause
                .literals()
                .as_slice()
                .iter()
                .map(Eqn::standard_weight)
                .sum::<i64>()
        );
        assert!(clause.literal_weight(&bank, 2.0, 3.0, 4.0, 1, 2, 1.0, false) > 0.0);
        assert!(clause.fun_weight(2.0, 3.0, 4.0, 1, 100, &[1; 101], 2, 1.0, None) > 0.0);
        assert!(clause.non_linear_weight(&bank, 2.0, 3.0, 4.0, 1, 2, 3, 1.0, false) > 0.0);
        assert!(clause.sym_type_weight(2.0, 3.0, 4.0, 1, 2, 3, 4, 1.0) > 0.0);
        let extension = TermWeightExtension::new(
            2.0,
            3.0,
            4.0,
            TermWeightExtensionStyle::Simple,
            |_term: &Term, _data: &()| 1.0,
            (),
        );
        assert_eq!(
            clause.term_ext_weight(&extension).to_bits(),
            52.0_f64.to_bits()
        );
        assert!(clause.orient_weight(&bank, 5.0, 3.0, 4.0, 1, 2, 1.0, false) > 0.0);
        assert_eq!(clause.depth(), 2);

        let mut dist = vec![0; usize::try_from(bank.signature().f_count() + 1).unwrap()];
        clause.add_symbol_distribution(&mut dist);
        assert!(dist[usize::try_from(f_of_a.f_code()).unwrap()] > 0);

        let mut exists = Vec::new();
        let mut exists_dist = vec![0; dist.len()];
        clause.add_symbol_dist_exist(&mut exists_dist, &mut exists);
        assert!(exists.contains(&f_of_a.f_code()));

        let mut features = vec![0; usize::try_from((bank.signature().f_count() + 1) * 4).unwrap()];
        let mut modified = Vec::new();
        clause.add_symbol_features(&mut modified, &mut features);
        assert!(!modified.is_empty());

        let mut ranks = vec![0; dist.len()];
        let mut count = 1;
        clause.compute_function_ranks(&mut ranks, &mut count);
        assert!(ranks[usize::try_from(f_of_a.f_code()).unwrap()] > 0);

        let mut vars = BTreeMap::new();
        assert_eq!(clause.collect_variables(&mut vars), 0);
        let mut fcodes = BTreeSet::new();
        assert!(clause.collect_fcodes(&mut fcodes) >= 3);
        let mut occur = PDIntArray::new_int(2, GROW_EXPONENTIAL);
        let mut occur_stack = Vec::new();
        assert!(clause.add_fun_occs(&mut occur, &mut occur_stack) >= 3);

        let mut subterms = PStack::new();
        assert!(clause.collect_subterms(&mut subterms) >= 3);
        assert!(subterms
            .as_slice()
            .iter()
            .all(|term| !term.query_prop(TP_OP_FLAG)));

        let mut returned = Vec::new();
        assert!(clause.return_fcodes(&mut returned) >= 3);
        let mut ground_terms = BTreeMap::new();
        assert!(clause.collect_ground_terms(&mut ground_terms, true, true, false) >= 1);
    }

    #[test]
    fn ac_redundant_definition_untyped_and_query_helpers_match_shapes() {
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
        let ac_clause = Clause::alloc(EqnList::from_vec(vec![
            eqn(&mut bank, &left, &right, true),
            eqn(&mut bank, &a, &b, false),
        ]));
        assert!(ac_clause.is_ac_redundant(&bank));

        let x = typed_var(&bank, -10);
        let y = typed_var(&bank, -12);
        let def_left = typed_binary_with_code(&mut bank, f_code, &x, &y);
        let def = Clause::alloc(EqnList::from_vec(vec![eqn(&mut bank, &def_left, &a, true)]));
        assert_eq!(def.is_eq_definition(&bank, 1), EqnSide::LeftSide);
        assert!(def.is_untyped());
        assert!(def.query_literal(Eqn::is_positive));
        assert!(def.is_equational(&bank));
        assert!(def.is_pure_equational(&bank));
    }

    #[test]
    fn normalize_vars_replaces_bound_variables_with_fresh_bank_variables() {
        let mut bank = test_bank();
        let x = typed_var(&bank, -10);
        let a = typed_const(&mut bank, "a");
        let mut clause = Clause::alloc(EqnList::from_vec(vec![eqn(&mut bank, &x, &a, true)]));
        let fresh = VarBank::new(bank.signature().type_bank());

        clause.normalize_vars(&mut bank, &fresh).unwrap();

        assert!(clause.literals().as_slice()[0].left().is_free_var());
        assert_ne!(clause.literals().as_slice()[0].left(), &x);
        let mut subst = Substitution::new();
        assert_eq!(clause.norm_subst(&mut subst, &fresh), 0);
        subst.backtrack();
    }

    #[test]
    #[should_panic(expected = "indexed clauses cannot be normalized in place")]
    fn normalize_vars_rejects_d_indexed_clauses() {
        let mut bank = test_bank();
        let x = typed_var(&bank, -10);
        let a = typed_const(&mut bank, "a");
        let mut clause = Clause::alloc(EqnList::from_vec(vec![eqn(&mut bank, &x, &a, true)]));
        clause.set_prop(CP_IS_D_INDEXED);
        let fresh = VarBank::new(bank.signature().type_bank());

        let _ = clause.normalize_vars(&mut bank, &fresh);
    }

    #[test]
    fn term_property_and_comparison_helpers_match_c_shapes() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let mut oriented = eqn(&mut bank, &a, &b, true);
        oriented.set_prop(EP_IS_ORIENTED);
        let rw_clause = Clause::alloc(EqnList::from_vec(vec![oriented]));
        assert!(rw_clause.is_rw_rule());

        rw_clause.term_set_prop(TP_SPECIAL_FLAG);
        assert!(rw_clause.literals().as_slice()[0]
            .left()
            .query_prop(TP_SPECIAL_FLAG));
        assert!(rw_clause.tb_term_del_prop_count(TP_SPECIAL_FLAG) > 0);
        rw_clause.term_del_prop(TP_SPECIAL_FLAG);

        let more_positive = Clause::alloc(EqnList::from_vec(vec![
            eqn(&mut bank, &a, &b, true),
            eqn(&mut bank, &b, &a, true),
        ]));
        assert!(more_positive.compare_fun(&rw_clause) < 0);
        assert_eq!(rw_clause.cmp_by_id(&rw_clause), 0);
        assert_eq!(rw_clause.query_tptp_type().bits(), 0);
    }

    #[test]
    fn conjecture_type_helper_accepts_positive_and_negative_conjectures() {
        let mut clause = Clause::empty();
        clause.set_tptp_type(CP_TYPE_CONJECTURE);
        assert!(clause.is_conjecture());
        clause.set_tptp_type(CP_TYPE_NEG_CONJECTURE);
        assert!(clause.is_conjecture());
        clause.set_prop(CP_IS_ORIENTED);
        assert!(clause.query_prop(CP_IS_ORIENTED));
    }
}
