use crate::basics::error::Diagnostic;
use crate::basics::pdarrays::PDIntArray;
use crate::basics::pstacks::PStack;
use crate::basics::sysdate::SysDate;
use crate::clauses::clause_props::{
    FormulaProperties, CP_IGNORE_PROPS, CP_IS_D_INDEXED, CP_IS_SOS, CP_TYPE_CONJECTURE,
    CP_TYPE_HYPOTHESIS, CP_TYPE_NEG_CONJECTURE,
};
use crate::clauses::clauseinfo::ClauseInfo;
use crate::clauses::eqn::Eqn;
use crate::clauses::eqn_props::{EqnProperties, EqnSide, EP_PSEUDO_LIT};
use crate::clauses::eqnlist::{EqnList, EQN_LIST_LONG_LIMIT};
use crate::terms::functypes::FunCode;
use crate::terms::subst::Substitution;
use crate::terms::termbanks::TermBank;
use crate::terms::termtypes::{Term, TermProperties, DEFAULT_FWEIGHT, DEFAULT_VWEIGHT, TP_OP_FLAG};
use crate::terms::termvars::VarBank;
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicI64, Ordering as AtomicOrdering};

static GLOBAL_CLAUSE_COUNTER: AtomicI64 = AtomicI64::new(i64::MIN);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Clause {
    ident: i64,
    date: SysDate,
    literals: EqnList,
    neg_lit_no: usize,
    pos_lit_no: usize,
    properties: FormulaProperties,
    weight: i64,
    info: Option<ClauseInfo>,
    create_date: i64,
    proof_depth: i64,
    proof_size: i64,
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
            info: None,
            create_date: 0,
            proof_depth: 0,
            proof_size: 0,
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
    pub const fn info(&self) -> Option<&ClauseInfo> {
        self.info.as_ref()
    }

    pub fn set_info(&mut self, info: Option<ClauseInfo>) {
        self.info = info;
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

    #[must_use]
    pub fn is_not_greater_equal_deferred(&self) -> bool {
        false
    }

    #[must_use]
    pub fn norm_subst(&self, subst: &mut Substitution, vars: &VarBank) -> usize {
        self.literals.subst_norm(subst, vars)
    }

    pub fn add_symbol_distribution(&self, dist_array: &mut [i64]) {
        self.literals.add_symbol_distribution(dist_array);
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
            info: None,
            create_date: self.create_date,
            proof_depth: self.proof_depth,
            proof_size: self.proof_size,
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

fn is_key_subset(left: &BTreeMap<usize, Term>, right: &BTreeMap<usize, Term>) -> bool {
    left.keys().all(|key| right.contains_key(key))
}

#[cfg(test)]
mod tests {
    use super::Clause;
    use crate::basics::pdarrays::{PDIntArray, GROW_EXPONENTIAL};
    use crate::basics::pstacks::PStack;
    use crate::basics::sysdate::SysDate;
    use crate::clauses::clause_props::{
        CP_INITIAL, CP_INPUT_FORMULA, CP_IS_D_INDEXED, CP_IS_ORIENTED, CP_IS_SOS, CP_TYPE_AXIOM,
        CP_TYPE_CONJECTURE, CP_TYPE_HYPOTHESIS, CP_TYPE_NEG_CONJECTURE,
    };
    use crate::clauses::clauseinfo::ClauseInfo;
    use crate::clauses::eqn::Eqn;
    use crate::clauses::eqn_props::{EqnSide, EP_IS_MAXIMAL, EP_IS_ORIENTED, EP_PSEUDO_LIT};
    use crate::clauses::eqnlist::EqnList;
    use crate::terms::signature::{Signature, FP_ASSOCIATIVE, FP_COMMUTATIVE};
    use crate::terms::simpletypes::alloc_arrow_type;
    use crate::terms::subst::Substitution;
    use crate::terms::termbanks::TermBank;
    use crate::terms::termtypes::{DerefType, Term, TP_OP_FLAG, TP_SPECIAL_FLAG};
    use crate::terms::termvars::VarBank;
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
        let answer_lit = Eqn::alloc(answer, bank.true_term().clone(), &mut bank, true).unwrap();
        let mut pseudo = eqn(&mut bank, &a, &b, true);
        pseudo.set_prop(EP_PSEUDO_LIT);
        let true_lit = eqn(&mut bank, &a, &a, true);
        let pos = eqn(&mut bank, &a, &b, true);
        let neg = eqn(&mut bank, &b, &a, false);

        assert!(Clause::alloc(EqnList::from_vec(vec![pseudo])).is_sem_false());
        assert!(Clause::alloc(EqnList::from_vec(vec![answer_lit])).is_sem_empty(&bank));
        assert!(Clause::alloc(EqnList::from_vec(vec![true_lit])).is_trivial(&bank));
        assert!(Clause::alloc(EqnList::from_vec(vec![pos, neg])).is_trivial(&bank));
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

        let flat = clause.flat_copy(&mut bank).unwrap();
        assert_eq!(flat.ident(), clause.ident());
        assert_eq!(flat.query_tptp_type(), CP_TYPE_AXIOM);
        assert!(flat.info().is_none());
        assert_eq!(flat.literals(), clause.literals());

        let copied = clause.copy_opt(&mut bank).unwrap();
        assert_eq!(copied.literal_number(), clause.literal_number());
        let disjoint = clause.copy_disjoint(&mut bank).unwrap();
        assert_ne!(disjoint.literals().as_slice()[1].left(), &x);
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
        assert!(!rw_clause.is_not_greater_equal_deferred());
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
