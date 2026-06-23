use crate::basics::error::{Diagnostic, ErrorCode};
use crate::basics::simple_stuff::{problem_type, ProblemType};
use crate::basics::{pdarrays::PDIntArray, pstacks::PStack};
use crate::clauses::eqn_props::{
    EqnProperties, EqnSide, PatEqnDirection, EP_IS_EQU_LITERAL, EP_IS_ORIENTED, EP_IS_POSITIVE,
    EP_MAX_IS_UP_TO_DATE, EP_NO_PROPS, EP_PSEUDO_LIT,
};
use crate::terms::acterms::term_ac_equal;
use crate::terms::functypes::FunCode;
use crate::terms::match_mgu::{subst_match_complete, subst_mgu_complete};
use crate::terms::signature::{FP_CL_SPLIT_DEF, FP_PSEUDO_PRED};
use crate::terms::simpletypes::type_is_predicate;
use crate::terms::subst::Substitution;
use crate::terms::termbanks::{
    tb_term_collect_subterms, tb_term_del_prop_count, tb_term_is_ground, tb_term_is_type_term,
    tb_term_is_x_type_term, TermBank,
};
use crate::terms::termfunc::{
    term_add_fun_occ, term_add_symbol_dist_exist, term_add_symbol_distribution_limited,
    term_add_symbol_features, term_add_symbol_features_limited, term_collect_fcodes,
    term_collect_ground_terms, term_collect_prop_variables, term_collect_variables,
    term_compute_function_ranks, term_dag_weight, term_depth, term_fsum_weight, term_has_f_code,
    term_is_def_term, term_is_untyped, term_lex_compare, term_non_linear_weight,
    term_standard_weight, term_struct_equal_deref, term_struct_weight_compare,
    term_sym_type_weight, term_weight_compute,
};
use crate::terms::termtypes::{
    term_del_prop, term_del_prop_opt, term_identity_cmp, term_set_prop, term_var_del_prop,
    term_var_search_prop, term_var_set_prop, DerefType, Term, TermProperties, TP_OP_FLAG,
    TP_PRED_POS,
};
use std::collections::{BTreeMap, BTreeSet};

fn cmp_bool_as_c(left: bool, right: bool) -> i32 {
    match (left, right) {
        (true, false) => 1,
        (false, true) => -1,
        _ => 0,
    }
}

fn cmp_i64(left: i64, right: i64) -> i32 {
    match left.cmp(&right) {
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Greater => 1,
    }
}

fn cmp_i32(left: i32, right: i32) -> i32 {
    match left.cmp(&right) {
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Greater => 1,
    }
}

fn subsume_term_pair_directed(
    pattern_left: &Term,
    pattern_right: &Term,
    target_left: &Term,
    target_right: &Term,
    subst: &mut Substitution,
) -> bool {
    let backtrack = subst.len();
    let mut result = subst_match_complete(pattern_left, target_left, subst);
    if result {
        result = subst_match_complete(pattern_right, target_right, subst);
    }
    if !result {
        subst.backtrack_to_pos(backtrack);
    }
    result
}

fn unify_term_pair_directed(
    source_terms: [&Term; 2],
    target_terms: [&Term; 2],
    subst: &mut Substitution,
) -> bool {
    let backtrack = subst.len();
    let mut result = subst_mgu_complete(source_terms[0], target_terms[0], subst);
    if result {
        result = subst_mgu_complete(source_terms[1], target_terms[1], subst);
    }
    if !result {
        subst.backtrack_to_pos(backtrack);
    }
    result
}

#[allow(clippy::cast_precision_loss)]
fn i64_to_f64(value: i64) -> f64 {
    value as f64
}

fn apply_app_var_mult(weight: f64, term: &Term, app_var_mult: f64) -> f64 {
    if term.is_applied_free_var() {
        weight * app_var_mult
    } else {
        weight
    }
}

fn identity_ordered_terms<'term>(
    left: &'term Term,
    right: &'term Term,
) -> (&'term Term, &'term Term) {
    if term_identity_cmp(left, right) >= 0 {
        (left, right)
    } else {
        (right, left)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Eqn {
    properties: EqnProperties,
    pos: i32,
    lterm: Term,
    rterm: Term,
}

impl Eqn {
    /// Allocates an equation/literal cell from already-shared terms.
    ///
    /// # Panics
    ///
    /// Panics if the C assertions around true predicate literals, equality
    /// right-hand sides, DB predicate declarations, or initialized equality
    /// arguments are violated.
    pub fn alloc(
        mut lterm: Term,
        mut rterm: Term,
        bank: &mut TermBank,
        mut positive: bool,
    ) -> Result<Self, Diagnostic> {
        let true_term = bank.true_term().clone();
        let false_term = bank.false_term().clone();

        if lterm == false_term {
            lterm = true_term.clone();
            positive = !positive;
        }
        if rterm == false_term {
            rterm = true_term.clone();
            positive = !positive;
        }
        if lterm == true_term {
            std::mem::swap(&mut lterm, &mut rterm);
        }

        let mut properties = EP_NO_PROPS;
        if positive {
            properties.set(EP_IS_POSITIVE);
        }
        if rterm == true_term {
            assert!(
                rterm.query_prop(TP_PRED_POS),
                "$true term must carry predicate-position property"
            );
            if lterm.f_code() > bank.signature().internal_symbols() {
                assert!(
                    !lterm.is_db_var(),
                    "DB variables are not declared as predicate symbols"
                );
                bank.signature_mut().declare_is_predicate(lterm.f_code())?;
            }
            lterm.set_prop(TP_PRED_POS);
            if !lterm.is_any_var() && bank.signature().query_prop(lterm.f_code(), FP_PSEUDO_PRED) {
                properties.set(EP_PSEUDO_LIT);
            }
        } else {
            assert_ne!(
                rterm.f_code(),
                crate::terms::signature::SIG_TRUE_CODE,
                "equality right side must not be a distinct $true cell"
            );
            properties.set(EP_IS_EQU_LITERAL);
        }

        let ltype = lterm.type_();
        let rtype = rterm.type_();
        let predicate_true_literal =
            ltype.as_ref().is_some_and(type_is_predicate) && rterm == true_term;
        if ltype != rtype && !predicate_true_literal {
            return Err(Diagnostic::new(ErrorCode::SYNTAX_ERROR, "Type error"));
        }

        Ok(Self {
            properties,
            pos: 0,
            lterm,
            rterm,
        })
    }

    /// Allocates a predicate literal, lifting `$eq`/`$neq` applications to
    /// equation-literal shape.
    ///
    /// # Panics
    ///
    /// Panics if `lterm` is not typed as boolean, or if an equality term lacks
    /// both arguments, matching the C assertions and argument access.
    pub fn alloc_flatten(
        lterm: Term,
        bank: &mut TermBank,
        mut sign: bool,
    ) -> Result<Self, Diagnostic> {
        assert!(
            lterm
                .type_()
                .as_ref()
                .is_some_and(crate::terms::simpletypes::Type::is_bool),
            "flattened literal input must be boolean"
        );
        let f_code = lterm.f_code();
        if f_code == bank.signature().eqn_code() || f_code == bank.signature().neqn_code() {
            if f_code == bank.signature().neqn_code() {
                sign = !sign;
            }
            let left = lterm
                .argument(0)
                .unwrap_or_else(|| panic!("equality literal left argument is uninitialized"));
            let right = lterm
                .argument(1)
                .unwrap_or_else(|| panic!("equality literal right argument is uninitialized"));
            Self::alloc(left, right, bank, sign)
        } else {
            Self::alloc(lterm, bank.true_term().clone(), bank, sign)
        }
    }

    pub fn create_true_lit(bank: &mut TermBank) -> Result<Self, Diagnostic> {
        Self::alloc(
            bank.true_term().clone(),
            bank.true_term().clone(),
            bank,
            true,
        )
    }

    /// Encodes a literal as a shared `$eq`/`$neq` term-bank term.
    ///
    /// # Panics
    ///
    /// Panics if either side is not present in the bank, or if term-bank top
    /// insertion invariants are violated, matching the C assertions.
    pub fn terms_tb_term_encode(
        bank: &mut TermBank,
        lterm: &Term,
        rterm: &Term,
        positive: bool,
        direction: PatEqnDirection,
    ) -> Result<Term, Diagnostic> {
        assert!(
            bank.find(lterm).is_some(),
            "left term must already be in the term bank"
        );
        assert!(
            bank.find(rterm).is_some(),
            "right term must already be in the term bank"
        );

        let f_code = bank.signature_mut().get_eqn_code(positive);
        assert_ne!(f_code, 0, "equality code allocation must succeed");
        let term = Term::top_alloc(f_code, 2);
        term.set_type(Some(bank.signature().type_bank().bool_type()));
        if direction == PatEqnDirection::Normal {
            term.set_argument(0, lterm.clone());
            term.set_argument(1, rterm.clone());
        } else {
            term.set_argument(0, rterm.clone());
            term.set_argument(1, lterm.clone());
        }
        bank.term_top_insert(term)
    }

    pub fn tb_term_encode(
        &self,
        bank: &mut TermBank,
        direction: PatEqnDirection,
    ) -> Result<Term, Diagnostic> {
        Self::terms_tb_term_encode(
            bank,
            &self.lterm,
            &self.rterm,
            self.is_positive(),
            direction,
        )
    }

    /// Decodes a shared `$eq`/`$neq` term into an equation/literal cell.
    ///
    /// # Panics
    ///
    /// Panics if the term is not headed by the bank's equality/inequality code
    /// or if either argument slot is uninitialized, matching the C assertions
    /// and direct argument access.
    pub fn tb_term_decode(bank: &mut TermBank, eqn: &Term) -> Result<Self, Diagnostic> {
        assert!(
            eqn.f_code() == bank.signature().eqn_code()
                || eqn.f_code() == bank.signature().neqn_code(),
            "encoded equation term must use equality or inequality code"
        );
        let positive = eqn.f_code() == bank.signature().eqn_code();
        let left = eqn
            .argument(0)
            .unwrap_or_else(|| panic!("encoded equation left argument is uninitialized"));
        let right = eqn
            .argument(1)
            .unwrap_or_else(|| panic!("encoded equation right argument is uninitialized"));
        Self::alloc(left, right, bank, positive)
    }

    #[must_use]
    pub const fn properties(&self) -> EqnProperties {
        self.properties
    }

    pub fn set_properties(&mut self, properties: EqnProperties) {
        self.properties = properties;
    }

    pub fn set_prop(&mut self, prop: EqnProperties) {
        self.properties.set(prop);
    }

    pub fn del_prop(&mut self, prop: EqnProperties) {
        self.properties.delete(prop);
    }

    pub fn flip_prop(&mut self, prop: EqnProperties) {
        self.properties.flip(prop);
    }

    #[must_use]
    pub const fn query_prop(&self, prop: EqnProperties) -> bool {
        self.properties.query(prop)
    }

    #[must_use]
    pub const fn is_any_prop_set(&self, prop: EqnProperties) -> bool {
        self.properties.is_any_set(prop)
    }

    #[must_use]
    pub const fn give_props(&self, prop: EqnProperties) -> EqnProperties {
        self.properties.give(prop)
    }

    #[must_use]
    pub const fn position(&self) -> i32 {
        self.pos
    }

    pub const fn set_position(&mut self, pos: i32) {
        self.pos = pos;
    }

    #[must_use]
    pub const fn left(&self) -> &Term {
        &self.lterm
    }

    #[must_use]
    pub const fn right(&self) -> &Term {
        &self.rterm
    }

    #[must_use]
    pub const fn is_oriented(&self) -> bool {
        self.properties.is_oriented()
    }

    #[must_use]
    pub const fn is_positive(&self) -> bool {
        self.properties.is_positive()
    }

    #[must_use]
    pub const fn is_negative(&self) -> bool {
        self.properties.is_negative()
    }

    #[must_use]
    /// # Panics
    ///
    /// Panics if the equation property bit and right-hand `$true` shape are
    /// inconsistent, matching the C macro assertions.
    pub fn is_equ_lit(&self, bank: &TermBank) -> bool {
        assert!(
            self.query_prop(EP_IS_EQU_LITERAL) || self.rterm == *bank.true_term(),
            "non-equational literals must have $true on the right"
        );
        assert!(
            !self.query_prop(EP_IS_EQU_LITERAL) || self.rterm != *bank.true_term(),
            "equational literals must not have $true on the right"
        );
        self.query_prop(EP_IS_EQU_LITERAL)
    }

    #[must_use]
    pub const fn is_maximal(&self) -> bool {
        self.properties.is_maximal()
    }

    #[must_use]
    pub const fn is_strictly_maximal(&self) -> bool {
        self.properties.is_strictly_maximal()
    }

    #[must_use]
    pub fn pred_code_fo(&self, bank: &TermBank) -> i64 {
        if self.is_equ_lit(bank) {
            0
        } else {
            self.lterm.f_code()
        }
    }

    #[must_use]
    pub fn pred_code_ho(&self, bank: &TermBank) -> i64 {
        if self.is_equ_lit(bank) || self.lterm.is_any_var() || self.lterm.is_phony_app() {
            0
        } else {
            self.lterm.f_code()
        }
    }

    #[must_use]
    pub fn is_split_lit(&self, bank: &TermBank) -> bool {
        !self.is_equ_lit(bank)
            && bank
                .signature()
                .query_prop(self.pred_code_fo(bank), FP_CL_SPLIT_DEF)
    }

    #[must_use]
    pub const fn has_equiv(&self) -> bool {
        self.properties.has_equiv()
    }

    #[must_use]
    pub const fn is_dominated(&self) -> bool {
        self.properties.is_dominated()
    }

    #[must_use]
    pub const fn dominates(&self) -> bool {
        self.properties.dominates()
    }

    #[must_use]
    pub const fn is_selected(&self) -> bool {
        self.properties.is_selected()
    }

    #[must_use]
    pub fn is_prop_true(&self) -> bool {
        self.lterm == self.rterm && self.is_positive()
    }

    #[must_use]
    pub fn is_prop_false(&self) -> bool {
        self.lterm == self.rterm && self.is_negative()
    }

    #[must_use]
    pub fn is_bool_var(&self, bank: &TermBank) -> bool {
        self.lterm.is_free_var() && self.rterm == *bank.true_term()
    }

    #[must_use]
    pub fn is_ground(&self) -> bool {
        tb_term_is_ground(&self.lterm) && tb_term_is_ground(&self.rterm)
    }

    #[must_use]
    pub fn is_pure_var(&self) -> bool {
        self.lterm.is_free_var() && self.rterm.is_free_var()
    }

    #[must_use]
    pub fn is_part_var(&self) -> bool {
        self.lterm.is_free_var() || self.rterm.is_free_var()
    }

    #[must_use]
    pub fn is_propositional(&self, bank: &TermBank) -> bool {
        !self.is_equ_lit(bank) && self.lterm.is_const()
    }

    #[must_use]
    pub fn is_type_pred(&self, bank: &TermBank) -> bool {
        !self.is_equ_lit(bank) && tb_term_is_type_term(&self.lterm)
    }

    #[must_use]
    pub fn is_x_type_pred(&self, bank: &TermBank) -> bool {
        !self.is_equ_lit(bank) && tb_term_is_x_type_term(&self.lterm)
    }

    #[must_use]
    pub fn is_real_x_type_pred(&self, bank: &TermBank) -> bool {
        !self.is_equ_lit(bank) && term_is_def_term(&self.lterm, 1)
    }

    #[must_use]
    pub fn is_simple_answer(&self, bank: &TermBank) -> bool {
        !self.lterm.is_db_var() && bank.signature().is_simple_answer_pred(self.lterm.f_code())
    }

    pub fn term_set_prop(&self, prop: TermProperties) {
        term_set_prop(&self.lterm, DerefType::Never, prop);
        term_set_prop(&self.rterm, DerefType::Never, prop);
    }

    #[must_use]
    pub fn tb_term_del_prop_count(&self, prop: TermProperties) -> i64 {
        tb_term_del_prop_count(&self.lterm, prop) + tb_term_del_prop_count(&self.rterm, prop)
    }

    pub fn term_del_prop(&self, prop: TermProperties) {
        term_del_prop(&self.lterm, DerefType::Never, prop);
        term_del_prop(&self.rterm, DerefType::Never, prop);
    }

    #[must_use]
    pub fn is_clausifiable(&self, bank: &TermBank) -> bool {
        self.lterm.type_().is_some_and(|type_| type_.is_bool())
            && (self.rterm != *bank.true_term()
                || (!self.lterm.is_any_var()
                    && bank.signature().is_logical_symbol(self.lterm.f_code())))
    }

    pub fn gc_mark_terms(&self, bank: &TermBank) {
        bank.gc_mark_term(&self.lterm);
        bank.gc_mark_term(&self.rterm);
    }

    pub fn swap_sides_simple(&mut self) {
        std::mem::swap(&mut self.lterm, &mut self.rterm);
    }

    pub fn swap_sides(&mut self) {
        self.del_prop(EP_IS_ORIENTED);
        self.del_prop(EP_MAX_IS_UP_TO_DATE);
        self.swap_sides_simple();
    }

    pub fn copy_to_bank(&self, bank: &mut TermBank) -> Result<Self, Diagnostic> {
        let lterm = bank.insert_no_props_cached(&self.lterm, DerefType::Always)?;
        let rterm = bank.insert_no_props_cached(&self.rterm, DerefType::Always)?;
        let mut handle = Self::alloc(lterm, rterm, bank, self.is_positive())?;
        handle.copy_properties_from(self);
        if !handle.is_oriented() {
            handle.del_prop(EP_MAX_IS_UP_TO_DATE);
        }
        Ok(handle)
    }

    pub fn flat_copy(&self, bank: &mut TermBank) -> Result<Self, Diagnostic> {
        let mut handle = Self::alloc(
            self.lterm.clone(),
            self.rterm.clone(),
            bank,
            self.is_positive(),
        )?;
        handle.copy_properties_from(self);
        if !handle.is_oriented() {
            handle.del_prop(EP_MAX_IS_UP_TO_DATE);
        }
        Ok(handle)
    }

    pub fn copy_repl(
        &self,
        bank: &mut TermBank,
        old: &Term,
        repl: &Term,
    ) -> Result<Self, Diagnostic> {
        let lterm = bank.insert_repl(&self.lterm, DerefType::Always, old, repl)?;
        let rterm = bank.insert_repl(&self.rterm, DerefType::Always, old, repl)?;
        let mut handle = Self::alloc(lterm, rterm, bank, self.is_positive())?;
        handle.copy_properties_from(self);
        handle.del_prop(EP_MAX_IS_UP_TO_DATE);
        handle.del_prop(EP_IS_ORIENTED);
        Ok(handle)
    }

    pub fn copy_repl_plain(
        &self,
        bank: &mut TermBank,
        old: &Term,
        repl: &Term,
    ) -> Result<Self, Diagnostic> {
        let lterm = bank.insert_repl_plain(&self.lterm, old, repl)?;
        let rterm = bank.insert_repl_plain(&self.rterm, old, repl)?;
        let mut handle = Self::alloc(lterm, rterm, bank, self.is_positive())?;
        handle.copy_properties_from(self);
        handle.del_prop(EP_MAX_IS_UP_TO_DATE);
        handle.del_prop(EP_IS_ORIENTED);
        Ok(handle)
    }

    pub fn copy_opt(&self, bank: &mut TermBank) -> Result<Self, Diagnostic> {
        let lterm = bank.insert_opt(&self.lterm, DerefType::Always)?;
        let rterm = bank.insert_opt(&self.rterm, DerefType::Always)?;
        let mut handle = Self::alloc(lterm, rterm, bank, self.is_positive())?;
        handle.copy_properties_from(self);
        handle.del_prop(EP_MAX_IS_UP_TO_DATE);
        handle.del_prop(EP_IS_ORIENTED);
        Ok(handle)
    }

    pub fn copy_disjoint(&self, bank: &mut TermBank) -> Result<Self, Diagnostic> {
        let lterm = bank.insert_disjoint(&self.lterm)?;
        let rterm = bank.insert_disjoint(&self.rterm)?;
        let mut handle = Self::alloc(lterm, rterm, bank, self.is_positive())?;
        handle.copy_properties_from(self);
        Ok(handle)
    }

    #[must_use]
    pub fn is_trivial(&self) -> bool {
        self.lterm == self.rterm
    }

    #[must_use]
    pub fn is_ac_trivial(&self, bank: &TermBank) -> bool {
        term_ac_equal(bank.signature(), &self.lterm, &self.rterm)
    }

    #[must_use]
    pub fn terms_are_distinct(&self, bank: &TermBank) -> bool {
        self.lterm.is_const()
            && self.rterm.is_const()
            && bank
                .signature()
                .is_any_func_prop_set(self.lterm.f_code(), bank.signature().distinct_props())
            && bank
                .signature()
                .is_any_func_prop_set(self.rterm.f_code(), bank.signature().distinct_props())
            && self.lterm.f_code() != self.rterm.f_code()
    }

    #[must_use]
    pub fn is_true(&self, bank: &TermBank) -> bool {
        if self.is_positive() {
            self.is_trivial()
        } else {
            self.terms_are_distinct(bank)
        }
    }

    #[must_use]
    pub fn is_false(&self, bank: &TermBank) -> bool {
        if self.is_negative() {
            self.is_trivial()
        } else {
            self.terms_are_distinct(bank)
        }
    }

    #[must_use]
    pub fn has_unbound_vars(&self, dom_side: EqnSide) -> bool {
        let (domain, range) = if dom_side == EqnSide::LeftSide {
            (&self.lterm, &self.rterm)
        } else {
            (&self.rterm, &self.lterm)
        };

        term_var_set_prop(range, DerefType::Never, TP_OP_FLAG);
        term_var_del_prop(domain, DerefType::Never, TP_OP_FLAG);
        term_var_search_prop(range, DerefType::Never, TP_OP_FLAG)
    }

    #[must_use]
    pub fn is_definition(&self, bank: &TermBank, min_arity: usize) -> EqnSide {
        if self.is_negative() {
            return EqnSide::NoSide;
        }
        if term_is_def_term(&self.lterm, min_arity)
            && !bank
                .signature()
                .query_prop(self.lterm.f_code(), FP_PSEUDO_PRED)
            && !term_has_f_code(&self.rterm, self.lterm.f_code())
            && !self.has_unbound_vars(EqnSide::LeftSide)
        {
            return EqnSide::LeftSide;
        }
        if term_is_def_term(&self.rterm, min_arity)
            && !bank
                .signature()
                .query_prop(self.rterm.f_code(), FP_PSEUDO_PRED)
            && !term_has_f_code(&self.lterm, self.rterm.f_code())
            && !self.has_unbound_vars(EqnSide::RightSide)
        {
            return EqnSide::RightSide;
        }
        EqnSide::NoSide
    }

    #[must_use]
    pub fn standard_weight(&self) -> i64 {
        term_standard_weight(&self.lterm) + term_standard_weight(&self.rterm)
    }

    #[must_use]
    pub fn standard_diff(&self) -> i64 {
        let left = term_standard_weight(&self.lterm);
        let right = term_standard_weight(&self.rterm);
        left.max(right) - left.min(right)
    }

    #[must_use]
    pub const fn count_maximal_literals(&self) -> i64 {
        if self.is_oriented() {
            1
        } else {
            2
        }
    }

    #[must_use]
    pub fn weight(
        &self,
        max_multiplier: f64,
        vweight: i64,
        fweight: i64,
        app_var_mult: f64,
    ) -> f64 {
        let mut result = i64_to_f64(term_weight_compute(&self.rterm, vweight, fweight));
        if !self.is_oriented() {
            result *= max_multiplier;
        }
        result = apply_app_var_mult(result, &self.rterm, app_var_mult);
        result += apply_app_var_mult(
            i64_to_f64(term_weight_compute(&self.lterm, vweight, fweight)) * max_multiplier,
            &self.lterm,
            app_var_mult,
        );
        result
    }

    #[must_use]
    #[expect(
        clippy::too_many_arguments,
        reason = "C-compatible helper mirrors ccl_eqn argument list"
    )]
    pub fn dag_weight(
        &self,
        uniqmax_multiplier: f64,
        max_multiplier: f64,
        vweight: i64,
        fweight: i64,
        dup_weight: i64,
        new_eqn: bool,
        new_terms: bool,
    ) -> f64 {
        if new_eqn {
            self.term_del_prop(TP_OP_FLAG);
        } else if new_terms {
            term_del_prop_opt(&self.lterm, TP_OP_FLAG);
        }

        let lweight = term_dag_weight(&self.lterm, fweight, vweight, dup_weight, false);
        let rweight = term_dag_weight(&self.rterm, fweight, vweight, dup_weight, new_terms);

        if self.is_oriented() {
            uniqmax_multiplier * max_multiplier * i64_to_f64(lweight) + i64_to_f64(rweight)
        } else {
            max_multiplier * i64_to_f64(lweight) + max_multiplier * i64_to_f64(rweight)
        }
    }

    #[must_use]
    pub fn dag_weight2(
        &self,
        maxw_multiplier: f64,
        vweight: i64,
        fweight: i64,
        dup_weight: i64,
    ) -> f64 {
        let mut lweight = term_dag_weight(&self.lterm, fweight, vweight, dup_weight, true);
        let mut rweight = term_dag_weight(&self.rterm, fweight, vweight, dup_weight, true);
        if rweight > lweight {
            std::mem::swap(&mut lweight, &mut rweight);
        }
        maxw_multiplier * i64_to_f64(lweight) + i64_to_f64(rweight)
    }

    #[must_use]
    #[expect(
        clippy::too_many_arguments,
        reason = "C-compatible helper mirrors ccl_eqn argument list"
    )]
    pub fn fun_weight(
        &self,
        max_multiplier: f64,
        vweight: i64,
        flimit: FunCode,
        fweights: &[i64],
        default_fweight: i64,
        app_var_mult: f64,
        typefreqs: Option<&BTreeMap<i64, i64>>,
    ) -> f64 {
        let mut result = i64_to_f64(term_fsum_weight(
            &self.rterm,
            vweight,
            flimit,
            fweights,
            default_fweight,
            typefreqs,
        ));
        result = apply_app_var_mult(result, &self.rterm, app_var_mult);
        if !self.is_oriented() {
            result *= max_multiplier;
        }
        result += apply_app_var_mult(
            i64_to_f64(term_fsum_weight(
                &self.lterm,
                vweight,
                flimit,
                fweights,
                default_fweight,
                typefreqs,
            )) * max_multiplier,
            &self.lterm,
            app_var_mult,
        );
        result
    }

    #[must_use]
    pub fn non_linear_weight(
        &self,
        max_multiplier: f64,
        first_var_weight: i64,
        repeat_var_weight: i64,
        fweight: i64,
        app_var_mult: f64,
    ) -> f64 {
        let mut result = i64_to_f64(term_non_linear_weight(
            &self.rterm,
            first_var_weight,
            repeat_var_weight,
            fweight,
        ));
        if !self.is_oriented() {
            result *= max_multiplier;
        }
        result = apply_app_var_mult(result, &self.rterm, app_var_mult);
        result += apply_app_var_mult(
            i64_to_f64(term_non_linear_weight(
                &self.lterm,
                first_var_weight,
                repeat_var_weight,
                fweight,
            )) * max_multiplier,
            &self.lterm,
            app_var_mult,
        );
        result
    }

    #[must_use]
    pub fn sym_type_weight(
        &self,
        max_multiplier: f64,
        vweight: i64,
        fweight: i64,
        cweight: i64,
        pweight: i64,
        app_var_mult: f64,
    ) -> f64 {
        let mut result = i64_to_f64(term_sym_type_weight(
            &self.rterm,
            vweight,
            fweight,
            cweight,
            pweight,
        ));
        if !self.is_oriented() {
            result *= max_multiplier;
        }
        result = apply_app_var_mult(result, &self.rterm, app_var_mult);
        result += apply_app_var_mult(
            i64_to_f64(term_sym_type_weight(
                &self.lterm,
                vweight,
                fweight,
                cweight,
                pweight,
            )) * max_multiplier,
            &self.lterm,
            app_var_mult,
        );
        result
    }

    #[must_use]
    pub fn max_weight(&self, vweight: i64, fweight: i64, app_var_mult: f64) -> f64 {
        let left = apply_app_var_mult(
            i64_to_f64(term_weight_compute(&self.lterm, vweight, fweight)),
            &self.lterm,
            app_var_mult,
        );
        let right = apply_app_var_mult(
            i64_to_f64(term_weight_compute(&self.rterm, vweight, fweight)),
            &self.rterm,
            app_var_mult,
        );
        left.max(right)
    }

    #[must_use]
    pub fn corrected_weight(
        &self,
        bank: &TermBank,
        max_multiplier: f64,
        vweight: i64,
        fweight: i64,
        app_var_mult: f64,
    ) -> f64 {
        let mut result = if self.is_equ_lit(bank) {
            let mut right = i64_to_f64(term_weight_compute(&self.rterm, vweight, fweight));
            if !self.is_oriented() {
                right *= max_multiplier;
            }
            right += i64_to_f64(fweight);
            apply_app_var_mult(right, &self.rterm, app_var_mult)
        } else {
            0.0
        };
        result += apply_app_var_mult(
            i64_to_f64(term_weight_compute(&self.lterm, vweight, fweight)) * max_multiplier,
            &self.lterm,
            app_var_mult,
        );
        result
    }

    #[must_use]
    pub fn corrected_non_linear_weight(
        &self,
        bank: &TermBank,
        max_multiplier: f64,
        first_var_weight: i64,
        repeat_var_weight: i64,
        fweight: i64,
        app_var_mult: f64,
    ) -> f64 {
        let mut result = if self.is_equ_lit(bank) {
            let mut right = i64_to_f64(term_non_linear_weight(
                &self.rterm,
                first_var_weight,
                repeat_var_weight,
                fweight,
            ));
            if !self.is_oriented() {
                right *= max_multiplier;
            }
            apply_app_var_mult(right, &self.rterm, app_var_mult) + i64_to_f64(fweight)
        } else {
            0.0
        };
        result += apply_app_var_mult(
            i64_to_f64(term_non_linear_weight(
                &self.lterm,
                first_var_weight,
                repeat_var_weight,
                fweight,
            )) * max_multiplier,
            &self.lterm,
            app_var_mult,
        );
        result
    }

    #[must_use]
    pub fn max_term_positions(&self) -> i64 {
        let mut result = term_weight_compute(&self.lterm, 1, 1);
        if !self.is_oriented() {
            result += term_weight_compute(&self.rterm, 1, 1);
        }
        result
    }

    #[must_use]
    pub fn inference_positions(&self) -> i64 {
        let mut result = term_weight_compute(&self.lterm, 0, 1);
        if self.is_oriented() {
            result += term_weight_compute(&self.rterm, 0, 1);
        }
        result
    }

    #[must_use]
    #[expect(
        clippy::too_many_arguments,
        reason = "C-compatible helper mirrors ccl_eqn argument list"
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
        let mut result = if count_eq_encoding {
            self.weight(max_term_multiplier, vweight, fweight, app_var_mult)
        } else {
            self.corrected_weight(bank, max_term_multiplier, vweight, fweight, app_var_mult)
        };
        if self.is_maximal() {
            result *= max_literal_multiplier;
        }
        if self.is_positive() {
            result *= pos_multiplier;
        }
        result
    }

    #[must_use]
    #[expect(
        clippy::too_many_arguments,
        reason = "C-compatible helper mirrors ccl_eqn argument list"
    )]
    pub fn literal_fun_weight(
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
        let mut result = self.fun_weight(
            max_term_multiplier,
            vweight,
            flimit,
            fweights,
            default_fweight,
            app_var_mult,
            typefreqs,
        );
        if self.is_maximal() {
            result *= max_literal_multiplier;
        }
        if self.is_positive() {
            result *= pos_multiplier;
        }
        result
    }

    #[must_use]
    #[expect(
        clippy::too_many_arguments,
        reason = "C-compatible helper mirrors ccl_eqn argument list"
    )]
    pub fn literal_non_linear_weight(
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
        let mut result = if count_eq_encoding {
            self.non_linear_weight(
                max_term_multiplier,
                first_var_weight,
                repeat_var_weight,
                fweight,
                app_var_mult,
            )
        } else {
            self.corrected_non_linear_weight(
                bank,
                max_term_multiplier,
                first_var_weight,
                repeat_var_weight,
                fweight,
                app_var_mult,
            )
        };
        if self.is_maximal() {
            result *= max_literal_multiplier;
        }
        if self.is_positive() {
            result *= pos_multiplier;
        }
        result
    }

    #[must_use]
    #[expect(
        clippy::too_many_arguments,
        reason = "C-compatible helper mirrors ccl_eqn argument list"
    )]
    pub fn literal_sym_type_weight(
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
        let mut result = self.sym_type_weight(
            max_term_multiplier,
            vweight,
            fweight,
            cweight,
            pweight,
            app_var_mult,
        );
        if self.is_maximal() {
            result *= max_literal_multiplier;
        }
        if self.is_positive() {
            result *= pos_multiplier;
        }
        result
    }

    pub fn canonize(&mut self) {
        if term_struct_weight_compare(&self.lterm, &self.rterm) == 0
            && term_lex_compare(&self.lterm, &self.rterm) < 0
        {
            self.swap_sides();
        }
    }

    #[must_use]
    pub fn struct_weight_compare(&self, other: &Self, bank: &TermBank) -> i64 {
        if self.is_positive() && !other.is_positive() {
            return -1;
        }
        if other.is_positive() && !self.is_positive() {
            return 1;
        }
        if self.is_equ_lit(bank) && !other.is_equ_lit(bank) {
            return -1;
        }
        if other.is_equ_lit(bank) && !self.is_equ_lit(bank) {
            return 1;
        }
        let weight_cmp = self.standard_weight() - other.standard_weight();
        if weight_cmp != 0 {
            return weight_cmp;
        }
        let left_cmp = term_struct_weight_compare(&self.lterm, &other.lterm);
        if left_cmp != 0 {
            return left_cmp;
        }
        term_struct_weight_compare(&self.rterm, &other.rterm)
    }

    #[must_use]
    pub fn struct_weight_lex_compare(&self, other: &Self, bank: &TermBank) -> i64 {
        let structural = self.struct_weight_compare(other, bank);
        if structural != 0 {
            return structural;
        }
        let left_cmp = term_lex_compare(&self.lterm, &other.lterm);
        if left_cmp != 0 {
            return left_cmp;
        }
        term_lex_compare(&self.rterm, &other.rterm)
    }

    #[must_use]
    pub fn equal_directed(&self, other: &Self) -> bool {
        self.lterm == other.lterm && self.rterm == other.rterm
    }

    #[must_use]
    pub fn equal_directed_deref(
        &self,
        other: &Self,
        left_deref: DerefType,
        right_deref: DerefType,
    ) -> bool {
        if left_deref == DerefType::Never && right_deref == DerefType::Never {
            self.equal_directed(other)
        } else {
            term_struct_equal_deref(&self.lterm, &other.lterm, left_deref, right_deref)
                && term_struct_equal_deref(&self.rterm, &other.rterm, left_deref, right_deref)
        }
    }

    #[must_use]
    pub fn equal_deref(&self, other: &Self, left_deref: DerefType, right_deref: DerefType) -> bool {
        let directed = self.equal_directed_deref(other, left_deref, right_deref);
        if directed || (self.is_oriented() && other.is_oriented()) {
            return directed;
        }
        if left_deref == DerefType::Never && right_deref == DerefType::Never {
            self.lterm == other.rterm && self.rterm == other.lterm
        } else {
            term_struct_equal_deref(&self.lterm, &other.rterm, left_deref, right_deref)
                && term_struct_equal_deref(&self.rterm, &other.lterm, left_deref, right_deref)
        }
    }

    #[must_use]
    pub fn equal(&self, other: &Self) -> bool {
        self.equal_deref(other, DerefType::Never, DerefType::Never)
    }

    #[must_use]
    pub fn literal_equal(&self, other: &Self) -> bool {
        EqnProperties::are_equiv(self.properties, other.properties, EP_IS_POSITIVE)
            && self.equal(other)
    }

    #[must_use]
    pub fn subsume_q_order_compare(&self, other: &Self, bank: &TermBank) -> i32 {
        self.subsume_q_order_compare_for_problem_type(other, bank, problem_type())
    }

    #[must_use]
    fn subsume_q_order_compare_for_problem_type(
        &self,
        other: &Self,
        bank: &TermBank,
        problem_type: ProblemType,
    ) -> i32 {
        let mut result = cmp_bool_as_c(self.is_positive(), other.is_positive());
        if result != 0 {
            return result;
        }

        let self_is_equ_lit = self.is_equ_lit(bank);
        let other_is_equ_lit = other.is_equ_lit(bank);
        result = cmp_bool_as_c(self_is_equ_lit, other_is_equ_lit);
        if result != 0 {
            return result;
        }

        if problem_type == ProblemType::FirstOrder && !self_is_equ_lit {
            result = cmp_i64(self.lterm.f_code(), other.lterm.f_code());
        }
        result
    }

    #[must_use]
    pub fn subsume_inverse_compare(&self, other: &Self, bank: &TermBank) -> i32 {
        let result = other.subsume_q_order_compare(self, bank);
        if result != 0 {
            result
        } else {
            cmp_i64(other.standard_weight(), self.standard_weight())
        }
    }

    #[must_use]
    pub fn subsume_inverse_refined_compare(&self, other: &Self, bank: &TermBank) -> i32 {
        let result = self.subsume_inverse_compare(other, bank);
        if result != 0 {
            result
        } else {
            cmp_i32(self.pos, other.pos)
        }
    }

    #[must_use]
    pub fn subsume_compare(&self, other: &Self, bank: &TermBank) -> i32 {
        let result = self.subsume_q_order_compare(other, bank);
        if result != 0 {
            result
        } else {
            cmp_i64(self.standard_weight(), other.standard_weight())
        }
    }

    pub fn subsume_directed(&self, subsumed: &Self, subst: &mut Substitution) -> bool {
        subsume_term_pair_directed(
            &self.lterm,
            &self.rterm,
            &subsumed.lterm,
            &subsumed.rterm,
            subst,
        )
    }

    pub fn subsume(&self, subsumed: &Self, subst: &mut Substitution) -> bool {
        if self.is_oriented() && !subsumed.is_oriented() {
            return false;
        }
        let result = self.subsume_directed(subsumed, subst);
        if result || self.is_oriented() {
            return result;
        }
        subsume_term_pair_directed(
            &self.rterm,
            &self.lterm,
            &subsumed.lterm,
            &subsumed.rterm,
            subst,
        )
    }

    #[must_use]
    pub fn subsume_p(&self, subsumed: &Self) -> bool {
        let mut subst = Substitution::new();
        let result = self.subsume(subsumed, &mut subst);
        subst.backtrack();
        result
    }

    #[must_use]
    pub fn literal_subsume_p(&self, subsumed: &Self) -> bool {
        EqnProperties::are_equiv(self.properties, subsumed.properties, EP_IS_POSITIVE)
            && self.subsume_p(subsumed)
    }

    pub fn unify_directed(&self, other: &Self, subst: &mut Substitution) -> bool {
        unify_term_pair_directed(
            [&self.lterm, &self.rterm],
            [&other.lterm, &other.rterm],
            subst,
        )
    }

    pub fn unify(&self, other: &Self, subst: &mut Substitution) -> bool {
        let result = self.unify_directed(other, subst);
        if result || (self.is_oriented() && other.is_oriented()) {
            return result;
        }
        unify_term_pair_directed(
            [&self.rterm, &self.lterm],
            [&other.lterm, &other.rterm],
            subst,
        )
    }

    #[must_use]
    pub fn unify_p(&self, other: &Self) -> bool {
        let mut subst = Substitution::new();
        let result = self.unify(other, &mut subst);
        subst.backtrack();
        result
    }

    pub fn literal_unify_one_way(
        &self,
        other: &mut Self,
        subst: &mut Substitution,
        swapped: bool,
    ) -> bool {
        if self.is_positive() != other.is_positive() {
            return false;
        }
        if swapped {
            other.swap_sides();
        }
        let result = self.unify_directed(other, subst);
        if swapped {
            other.swap_sides();
        }
        result
    }

    #[must_use]
    pub fn syntax_compare(&self, other: &Self, bank: &TermBank) -> i32 {
        if self.is_equ_lit(bank) && !other.is_equ_lit(bank) {
            return -1;
        }
        if other.is_equ_lit(bank) && !self.is_equ_lit(bank) {
            return 1;
        }

        let self_max = self.lterm.entry_no().max(self.rterm.entry_no());
        let other_max = other.lterm.entry_no().max(other.rterm.entry_no());
        let max_cmp = cmp_i64(self_max, other_max);
        if max_cmp != 0 {
            return max_cmp;
        }

        let self_min = self.lterm.entry_no().min(self.rterm.entry_no());
        let other_min = other.lterm.entry_no().min(other.rterm.entry_no());
        cmp_i64(self_min, other_min)
    }

    #[must_use]
    pub fn literal_syntax_compare(&self, other: &Self, bank: &TermBank) -> i32 {
        if self.is_positive() && !other.is_positive() {
            return -1;
        }
        if other.is_positive() && !self.is_positive() {
            return 1;
        }
        self.syntax_compare(other, bank)
    }

    #[must_use]
    pub fn literal_compare_fun(&self, other: &Self) -> i32 {
        if self.is_positive() && !other.is_positive() {
            return 1;
        }
        if other.is_positive() && !self.is_positive() {
            return -1;
        }

        let (self_max, self_min) = identity_ordered_terms(&self.lterm, &self.rterm);
        let (other_max, other_min) = identity_ordered_terms(&other.lterm, &other.rterm);
        let max_cmp = term_identity_cmp(self_max, other_max);
        if max_cmp != 0 {
            return max_cmp;
        }
        term_identity_cmp(self_min, other_min)
    }

    #[must_use]
    pub fn depth(&self) -> i64 {
        term_depth(&self.lterm).max(term_depth(&self.rterm))
    }

    pub fn add_symbol_distribution(&self, dist_array: &mut [i64]) {
        term_add_symbol_distribution_limited(&self.lterm, dist_array, usize::MAX);
        term_add_symbol_distribution_limited(&self.rterm, dist_array, usize::MAX);
    }

    pub fn add_symbol_dist_exist(&self, dist_array: &mut [i64], exists: &mut Vec<FunCode>) {
        term_add_symbol_dist_exist(&self.lterm, dist_array, exists);
        term_add_symbol_dist_exist(&self.rterm, dist_array, exists);
    }

    pub fn add_symbol_distribution_limited(&self, dist_array: &mut [i64], limit: usize) {
        term_add_symbol_distribution_limited(&self.lterm, dist_array, limit);
        term_add_symbol_distribution_limited(&self.rterm, dist_array, limit);
    }

    pub fn add_symbol_features_limited(
        &self,
        freq_array: &mut [i64],
        depth_array: &mut [i64],
        limit: usize,
    ) {
        term_add_symbol_features_limited(&self.lterm, 0, freq_array, depth_array, limit);
        term_add_symbol_features_limited(&self.rterm, 0, freq_array, depth_array, limit);
    }

    pub fn add_symbol_features(&self, mod_stack: &mut Vec<usize>, feature_array: &mut [i64]) {
        let offset = if self.is_negative() { 2 } else { 0 };
        term_add_symbol_features(&self.lterm, mod_stack, 0, feature_array, offset);
        term_add_symbol_features(&self.rterm, mod_stack, 0, feature_array, offset);
    }

    pub fn compute_function_ranks(&self, rank_array: &mut [i64], count: &mut i64) {
        term_compute_function_ranks(&self.lterm, rank_array, count);
        term_compute_function_ranks(&self.rterm, rank_array, count);
    }

    pub fn collect_variables(&self, vars: &mut BTreeMap<usize, Term>) -> i64 {
        term_collect_variables(&self.lterm, vars) + term_collect_variables(&self.rterm, vars)
    }

    pub fn collect_fcodes(&self, fcodes: &mut BTreeSet<FunCode>) -> i64 {
        term_collect_fcodes(&self.lterm, fcodes) + term_collect_fcodes(&self.rterm, fcodes)
    }

    pub fn collect_prop_variables(
        &self,
        vars: &mut BTreeMap<usize, Term>,
        prop: TermProperties,
    ) -> i64 {
        term_collect_prop_variables(&self.lterm, vars, prop)
            + term_collect_prop_variables(&self.rterm, vars, prop)
    }

    pub fn add_fun_occs(&self, f_occur: &mut PDIntArray, res_stack: &mut Vec<FunCode>) -> i64 {
        term_add_fun_occ(&self.lterm, f_occur, res_stack)
            + term_add_fun_occ(&self.rterm, f_occur, res_stack)
    }

    pub fn collect_subterms(&self, collector: &mut PStack<Term>) -> i64 {
        tb_term_collect_subterms(&self.lterm, collector)
            + tb_term_collect_subterms(&self.rterm, collector)
    }

    pub fn collect_ground_terms(
        &self,
        result: &mut BTreeMap<usize, Term>,
        all_subterms: bool,
    ) -> i64 {
        term_collect_ground_terms(&self.lterm, result, all_subterms)
            + term_collect_ground_terms(&self.rterm, result, all_subterms)
    }

    #[must_use]
    pub fn has_app_var(&self) -> bool {
        self.lterm.is_applied_free_var() || self.rterm.is_applied_free_var()
    }

    #[must_use]
    pub fn is_untyped(&self) -> bool {
        term_is_untyped(&self.lterm) && term_is_untyped(&self.rterm)
    }

    pub fn map_terms<F>(&mut self, bank: &TermBank, mut mapper: F)
    where
        F: FnMut(&Term) -> Term,
    {
        let old_left = self.lterm.clone();
        let mut lterm = mapper(&self.lterm);
        let mut rterm = mapper(&self.rterm);
        let mut negate = false;

        if lterm == *bank.false_term() {
            lterm = bank.true_term().clone();
            negate = !negate;
        }
        if rterm == *bank.false_term() {
            rterm = bank.true_term().clone();
            negate = !negate;
        }
        if lterm == *bank.true_term() {
            std::mem::swap(&mut lterm, &mut rterm);
        }
        if rterm == *bank.true_term() {
            self.del_prop(EP_IS_EQU_LITERAL);
        } else {
            self.set_prop(EP_IS_EQU_LITERAL);
        }
        if negate {
            self.flip_prop(EP_IS_POSITIVE);
        }
        if lterm != old_left {
            self.del_prop(EP_MAX_IS_UP_TO_DATE);
            self.del_prop(EP_IS_ORIENTED);
        }

        self.lterm = lterm;
        self.rterm = rterm;
    }

    fn copy_properties_from(&mut self, source: &Self) {
        self.properties = self.give_props(EP_IS_POSITIVE) | (source.properties & !EP_IS_POSITIVE);
    }
}

#[cfg(test)]
mod tests {
    use super::Eqn;
    use crate::basics::pdarrays::{PDIntArray, GROW_EXPONENTIAL};
    use crate::basics::pstacks::PStack;
    use crate::basics::simple_stuff::ProblemType;
    use crate::clauses::eqn_props::{
        EqnSide, PatEqnDirection, EP_FROM_CLAUSE_LIT, EP_IS_EQU_LITERAL, EP_IS_MAXIMAL,
        EP_IS_ORIENTED, EP_IS_PM_INTO_LIT, EP_IS_POSITIVE, EP_IS_SELECTED, EP_MAX_IS_UP_TO_DATE,
    };
    use crate::terms::signature::SIG_PHONY_APP_CODE;
    use crate::terms::signature::{
        FunctionProperties, Signature, FP_CL_SPLIT_DEF, FP_IS_INTEGER, FP_PSEUDO_PRED,
    };
    use crate::terms::simpletypes::alloc_arrow_type;
    use crate::terms::subst::Substitution;
    use crate::terms::termbanks::TermBank;
    use crate::terms::termfunc::term_standard_weight;
    use crate::terms::termtypes::{DerefType, Term, TP_CHECK_FLAG, TP_OP_FLAG, TP_PRED_POS};
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
        bank.insert(&term, crate::terms::termtypes::DerefType::Never)
            .unwrap()
    }

    fn typed_pred_const(bank: &mut TermBank, name: &str) -> Term {
        let bool_type = bank.signature().type_bank().bool_type();
        let f_code = bank.signature_mut().insert_id(name, 0, false);
        bank.signature_mut()
            .declare_final_type(f_code, bool_type.clone())
            .unwrap();
        let term = Term::const_cell_alloc(f_code);
        term.set_type(Some(bool_type));
        bank.insert(&term, crate::terms::termtypes::DerefType::Never)
            .unwrap()
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
        bank.insert(&term, crate::terms::termtypes::DerefType::Never)
            .unwrap()
    }

    fn typed_binary(bank: &mut TermBank, name: &str, left: &Term, right: &Term) -> Term {
        let type_ = bank.signature().type_bank().default_type();
        let f_code = bank.signature_mut().insert_id(name, 2, false);
        bank.signature_mut()
            .declare_final_type(
                f_code,
                alloc_arrow_type(vec![type_.clone(), type_.clone(), type_.clone()]),
            )
            .unwrap();
        let term = Term::top_alloc(f_code, 2);
        term.set_type(Some(type_));
        term.set_argument(0, left.clone());
        term.set_argument(1, right.clone());
        bank.insert(&term, crate::terms::termtypes::DerefType::Never)
            .unwrap()
    }

    fn applied_free_var(bank: &mut TermBank, variable: &Term, arg: &Term) -> Term {
        let term = Term::top_alloc(SIG_PHONY_APP_CODE, 2);
        term.set_type(Some(bank.signature().type_bank().default_type()));
        term.set_argument(0, variable.clone());
        term.set_argument(1, arg.clone());
        term
    }

    fn assert_f64_bits_eq(actual: f64, expected: f64) {
        assert_eq!(actual.to_bits(), expected.to_bits());
    }

    #[test]
    fn alloc_normalizes_false_and_true_like_c() {
        let mut bank = test_bank();
        let atom = typed_pred_const(&mut bank, "p");
        let false_term = bank.false_term().clone();

        let eq = Eqn::alloc(false_term, atom.clone(), &mut bank, true).unwrap();

        assert_eq!(eq.left(), &atom);
        assert_eq!(eq.right(), bank.true_term());
        assert!(eq.is_negative());
        assert!(!eq.is_equ_lit(&bank));
        assert!(atom.query_prop(TP_PRED_POS));
        assert_eq!(eq.position(), 0);
    }

    #[test]
    fn alloc_marks_equational_and_pseudo_predicate_literals() {
        let mut bank = test_bank();
        let left = typed_const(&mut bank, "a");
        let right = typed_const(&mut bank, "b");

        let eq = Eqn::alloc(left.clone(), right.clone(), &mut bank, true).unwrap();

        assert!(eq.is_positive());
        assert!(eq.is_equ_lit(&bank));
        assert_eq!(eq.left(), &left);
        assert_eq!(eq.right(), &right);

        let pseudo = typed_pred_const(&mut bank, "answer_like");
        bank.signature_mut()
            .set_func_prop(pseudo.f_code(), FP_PSEUDO_PRED);
        let lit = Eqn::alloc(pseudo, bank.true_term().clone(), &mut bank, true).unwrap();
        assert!(lit.query_prop(crate::clauses::eqn_props::EP_PSEUDO_LIT));
    }

    #[test]
    fn alloc_flatten_lifts_eq_and_flips_neq_sign() {
        let mut bank = test_bank();
        let left = typed_const(&mut bank, "a");
        let right = typed_const(&mut bank, "b");
        let eqn_type = bank.signature().type_bank().bool_type();
        let eqn_code = bank.signature().eqn_code();
        let neqn_code = bank.signature().neqn_code();

        let eq_term = Term::top_alloc(eqn_code, 2);
        eq_term.set_type(Some(eqn_type.clone()));
        eq_term.set_argument(0, left.clone());
        eq_term.set_argument(1, right.clone());
        let eq = Eqn::alloc_flatten(eq_term, &mut bank, true).unwrap();
        assert!(eq.is_positive());
        assert!(eq.is_equ_lit(&bank));
        assert_eq!(eq.left(), &left);

        let neq_term = Term::top_alloc(neqn_code, 2);
        neq_term.set_type(Some(eqn_type));
        neq_term.set_argument(0, left);
        neq_term.set_argument(1, right);
        let neq = Eqn::alloc_flatten(neq_term, &mut bank, true).unwrap();
        assert!(neq.is_negative());
        assert!(neq.is_equ_lit(&bank));
    }

    #[test]
    fn term_bank_encoding_and_decoding_match_c_shape() {
        let mut bank = test_bank();
        let left = typed_const(&mut bank, "a");
        let right = typed_const(&mut bank, "b");

        let encoded =
            Eqn::terms_tb_term_encode(&mut bank, &left, &right, true, PatEqnDirection::Normal)
                .unwrap();
        assert_eq!(encoded.f_code(), bank.signature().eqn_code());
        assert_eq!(
            encoded.type_(),
            Some(bank.signature().type_bank().bool_type())
        );
        assert_eq!(encoded.argument(0), Some(left.clone()));
        assert_eq!(encoded.argument(1), Some(right.clone()));
        assert!(encoded.is_shared());

        let decoded = Eqn::tb_term_decode(&mut bank, &encoded).unwrap();
        assert!(decoded.is_positive());
        assert!(decoded.is_equ_lit(&bank));
        assert_eq!(decoded.left(), &left);
        assert_eq!(decoded.right(), &right);

        let reversed =
            Eqn::terms_tb_term_encode(&mut bank, &left, &right, false, PatEqnDirection::Reverse)
                .unwrap();
        assert_eq!(reversed.f_code(), bank.signature().neqn_code());
        assert_eq!(reversed.argument(0), Some(right.clone()));
        assert_eq!(reversed.argument(1), Some(left.clone()));
        let decoded_reversed = Eqn::tb_term_decode(&mut bank, &reversed).unwrap();
        assert!(decoded_reversed.is_negative());
        assert_eq!(decoded_reversed.left(), &right);
        assert_eq!(decoded_reversed.right(), &left);
    }

    #[test]
    fn instance_term_bank_encoding_uses_current_polarity() {
        let mut bank = test_bank();
        let left = typed_const(&mut bank, "a");
        let right = typed_const(&mut bank, "b");
        let mut eq = Eqn::alloc(left.clone(), right.clone(), &mut bank, true).unwrap();
        eq.flip_prop(EP_IS_POSITIVE);

        let encoded = eq
            .tb_term_encode(&mut bank, PatEqnDirection::Normal)
            .unwrap();

        assert_eq!(encoded.f_code(), bank.signature().neqn_code());
        assert_eq!(encoded.argument(0), Some(left));
        assert_eq!(encoded.argument(1), Some(right));
    }

    #[test]
    fn property_forwarders_and_swap_match_c_macros() {
        let mut bank = test_bank();
        let left = typed_const(&mut bank, "a");
        let right = typed_const(&mut bank, "b");
        let mut eq = Eqn::alloc(left.clone(), right.clone(), &mut bank, true).unwrap();

        eq.set_prop(EP_IS_ORIENTED | EP_MAX_IS_UP_TO_DATE | EP_IS_SELECTED);
        assert!(eq.is_oriented());
        assert!(eq.is_selected());
        assert!(eq.is_any_prop_set(EP_IS_SELECTED | EP_IS_MAXIMAL));
        assert_eq!(
            eq.give_props(EP_IS_SELECTED | EP_IS_MAXIMAL),
            EP_IS_SELECTED
        );
        eq.flip_prop(EP_IS_SELECTED);
        assert!(!eq.is_selected());

        eq.swap_sides();
        assert_eq!(eq.left(), &right);
        assert_eq!(eq.right(), &left);
        assert!(!eq.is_oriented());
        assert!(!eq.query_prop(EP_MAX_IS_UP_TO_DATE));
    }

    #[test]
    fn copy_variants_preserve_and_clear_properties_like_c() {
        let mut bank = test_bank();
        let left = typed_const(&mut bank, "a");
        let right = typed_const(&mut bank, "b");
        let replacement = typed_const(&mut bank, "c");
        let mut eq = Eqn::alloc(left.clone(), right.clone(), &mut bank, true).unwrap();
        eq.set_prop(EP_IS_ORIENTED | EP_MAX_IS_UP_TO_DATE | EP_IS_SELECTED | EP_FROM_CLAUSE_LIT);

        let copied = eq.copy_to_bank(&mut bank).unwrap();
        assert!(copied.is_oriented());
        assert!(copied.query_prop(EP_MAX_IS_UP_TO_DATE));
        assert!(copied.is_selected());
        assert!(copied.query_prop(EP_IS_EQU_LITERAL));

        let flat = eq.flat_copy(&mut bank).unwrap();
        assert_eq!(flat.left(), &left);
        assert!(flat.query_prop(EP_FROM_CLAUSE_LIT));

        let repl = eq.copy_repl_plain(&mut bank, &left, &replacement).unwrap();
        assert_eq!(repl.left(), &replacement);
        assert!(!repl.is_oriented());
        assert!(!repl.query_prop(EP_MAX_IS_UP_TO_DATE));
        assert!(repl.is_selected());

        let repl_instantiated = eq.copy_repl(&mut bank, &right, &replacement).unwrap();
        assert_eq!(repl_instantiated.right(), &replacement);
        assert!(!repl_instantiated.is_oriented());

        let opt = eq.copy_opt(&mut bank).unwrap();
        assert!(!opt.is_oriented());
        assert!(opt.query_prop(EP_IS_SELECTED));
    }

    #[test]
    fn copy_disjoint_replaces_free_variables_with_alternates() {
        let mut bank = test_bank();
        let type_ = bank.signature().type_bank().default_type();
        let x = bank.vars().var_assert_alloc(-2, &type_);
        let y = bank.vars().var_assert_alloc(-4, &type_);
        let eq = Eqn::alloc(x.clone(), y.clone(), &mut bank, true).unwrap();

        let disjoint = eq.copy_disjoint(&mut bank).unwrap();

        assert_eq!(disjoint.left().f_code(), -1);
        assert_eq!(disjoint.right().f_code(), -3);
        assert_ne!(disjoint.left(), &x);
        assert_ne!(disjoint.right(), &y);
    }

    #[test]
    fn simple_term_predicates_match_macros() {
        let mut bank = test_bank();
        let left = typed_const(&mut bank, "a");
        let right = typed_const(&mut bank, "b");
        let eq = Eqn::alloc(left.clone(), right.clone(), &mut bank, true).unwrap();
        assert!(eq.is_ground());
        assert!(!eq.is_part_var());
        assert!(!eq.is_trivial());

        let true_lit = Eqn::create_true_lit(&mut bank).unwrap();
        assert!(true_lit.is_prop_true());
        assert!(!true_lit.is_prop_false());
        assert!(true_lit.is_trivial());

        let bool_type = bank.signature().type_bank().bool_type();
        let x = bank.vars().var_assert_alloc(-2, &bool_type);
        let bool_lit = Eqn::alloc(x.clone(), bank.true_term().clone(), &mut bank, true).unwrap();
        assert!(bool_lit.is_bool_var(&bank));
        assert!(bool_lit.is_part_var());
        assert!(!bool_lit.is_pure_var());

        let type_ = bank.signature().type_bank().default_type();
        let x = bank.vars().var_assert_alloc(-6, &type_);
        let y = bank.vars().var_assert_alloc(-4, &type_);
        let var_eq = Eqn::alloc(x, y, &mut bank, true).unwrap();
        assert!(var_eq.is_pure_var());
    }

    #[test]
    fn predicate_code_split_and_distinct_helpers_match_c_shape() {
        let mut bank = test_bank();
        let atom = typed_pred_const(&mut bank, "split_pred");
        bank.signature_mut()
            .set_func_prop(atom.f_code(), FP_CL_SPLIT_DEF);
        let lit = Eqn::alloc(atom.clone(), bank.true_term().clone(), &mut bank, true).unwrap();

        assert_eq!(lit.pred_code_fo(&bank), atom.f_code());
        assert_eq!(lit.pred_code_ho(&bank), atom.f_code());
        assert!(lit.is_split_lit(&bank));
        assert!(lit.is_propositional(&bank));

        let left = typed_const(&mut bank, "1");
        let right = typed_const(&mut bank, "2");
        bank.signature_mut()
            .set_func_prop(left.f_code(), FP_IS_INTEGER);
        bank.signature_mut()
            .set_func_prop(right.f_code(), FP_IS_INTEGER);
        let distinct = Eqn::alloc(left, right, &mut bank, true).unwrap();
        assert!(distinct.terms_are_distinct(&bank));
        assert!(!distinct.is_split_lit(&bank));
    }

    #[test]
    fn truth_and_falsehood_helpers_match_c_cases() {
        let mut bank = test_bank();
        let left = typed_const(&mut bank, "a");
        let right = typed_const(&mut bank, "b");

        let positive_trivial = Eqn::alloc(left.clone(), left.clone(), &mut bank, true).unwrap();
        assert!(positive_trivial.is_true(&bank));
        assert!(!positive_trivial.is_false(&bank));

        let negative_trivial = Eqn::alloc(left.clone(), left.clone(), &mut bank, false).unwrap();
        assert!(!negative_trivial.is_true(&bank));
        assert!(negative_trivial.is_false(&bank));

        bank.signature_mut()
            .set_func_prop(left.f_code(), FP_IS_INTEGER);
        bank.signature_mut()
            .set_func_prop(right.f_code(), FP_IS_INTEGER);
        let positive_distinct = Eqn::alloc(left.clone(), right.clone(), &mut bank, true).unwrap();
        assert!(!positive_distinct.is_true(&bank));
        assert!(positive_distinct.is_false(&bank));

        let negative_distinct = Eqn::alloc(left, right, &mut bank, false).unwrap();
        assert!(negative_distinct.is_true(&bank));
        assert!(!negative_distinct.is_false(&bank));
    }

    #[test]
    fn unbound_variable_check_preserves_c_domain_side_shape() {
        let mut bank = test_bank();
        let type_ = bank.signature().type_bank().default_type();
        let x = bank.vars().var_assert_alloc(-2, &type_);
        let y = bank.vars().var_assert_alloc(-4, &type_);
        let left = typed_unary(&mut bank, "f", &x);
        let right_with_x = typed_unary(&mut bank, "g", &x);
        let right_with_y = typed_unary(&mut bank, "h", &y);

        let bound = Eqn::alloc(left.clone(), right_with_x, &mut bank, true).unwrap();
        assert!(!bound.has_unbound_vars(EqnSide::LeftSide));
        assert!(!bound.has_unbound_vars(EqnSide::RightSide));

        let unbound = Eqn::alloc(left.clone(), right_with_y, &mut bank, true).unwrap();
        assert!(unbound.has_unbound_vars(EqnSide::LeftSide));

        let fallback_right_side =
            Eqn::alloc(typed_unary(&mut bank, "q", &y), left, &mut bank, true).unwrap();
        assert!(fallback_right_side.has_unbound_vars(EqnSide::RightSide));
        assert!(fallback_right_side.has_unbound_vars(EqnSide::NoSide));
        assert!(fallback_right_side.has_unbound_vars(EqnSide::BothSides));
    }

    #[test]
    fn definition_detection_matches_positive_def_term_rules() {
        let mut bank = test_bank();
        let type_ = bank.signature().type_bank().default_type();
        let x = bank.vars().var_assert_alloc(-2, &type_);
        let y = bank.vars().var_assert_alloc(-4, &type_);

        let def_head = typed_unary(&mut bank, "f", &x);
        let body = typed_unary(&mut bank, "g", &x);
        let def = Eqn::alloc(def_head.clone(), body, &mut bank, true).unwrap();
        assert_eq!(def.is_definition(&bank, 1), EqnSide::LeftSide);
        assert_eq!(def.is_definition(&bank, 2), EqnSide::NoSide);

        let right_head = typed_unary(&mut bank, "h", &x);
        let body = typed_const(&mut bank, "body");
        let right_def = Eqn::alloc(body, right_head, &mut bank, true).unwrap();
        assert_eq!(right_def.is_definition(&bank, 1), EqnSide::RightSide);

        let negative = Eqn::alloc(
            def_head.clone(),
            typed_unary(&mut bank, "m", &x),
            &mut bank,
            false,
        )
        .unwrap();
        assert_eq!(negative.is_definition(&bank, 1), EqnSide::NoSide);

        let unbound_body = typed_unary(&mut bank, "n", &y);
        let unbound = Eqn::alloc(def_head.clone(), unbound_body, &mut bank, true).unwrap();
        assert_eq!(unbound.is_definition(&bank, 1), EqnSide::NoSide);

        bank.signature_mut()
            .set_func_prop(def_head.f_code(), FP_PSEUDO_PRED);
        let pseudo = Eqn::alloc(
            def_head,
            typed_const(&mut bank, "pseudo_body"),
            &mut bank,
            true,
        )
        .unwrap();
        assert_eq!(pseudo.is_definition(&bank, 1), EqnSide::NoSide);
    }

    #[test]
    fn canonicalization_and_structural_comparison_match_c_order() {
        let mut bank = test_bank();
        let left = typed_const(&mut bank, "a");
        let right = typed_const(&mut bank, "b");
        let mut eq = Eqn::alloc(left.clone(), right.clone(), &mut bank, true).unwrap();
        eq.set_prop(EP_IS_ORIENTED | EP_MAX_IS_UP_TO_DATE);

        assert_eq!(
            eq.standard_weight(),
            term_standard_weight(&left) + term_standard_weight(&right)
        );
        eq.canonize();
        assert_eq!(eq.left(), &right);
        assert_eq!(eq.right(), &left);
        assert!(!eq.is_oriented());
        assert!(!eq.query_prop(EP_MAX_IS_UP_TO_DATE));

        let positive = Eqn::alloc(left.clone(), right.clone(), &mut bank, true).unwrap();
        let negative = Eqn::alloc(left.clone(), right.clone(), &mut bank, false).unwrap();
        assert!(positive.struct_weight_compare(&negative, &bank) < 0);
        assert!(negative.struct_weight_compare(&positive, &bank) > 0);

        let atom = typed_pred_const(&mut bank, "p");
        let predicate_lit = Eqn::alloc(atom, bank.true_term().clone(), &mut bank, true).unwrap();
        assert!(positive.struct_weight_compare(&predicate_lit, &bank) < 0);

        let heavier_left = typed_unary(&mut bank, "f", &left);
        let heavier = Eqn::alloc(heavier_left, right.clone(), &mut bank, true).unwrap();
        assert!(heavier.struct_weight_compare(&positive, &bank) > 0);

        let lex_a = Eqn::alloc(left.clone(), right.clone(), &mut bank, true).unwrap();
        let lex_b = Eqn::alloc(right, left, &mut bank, true).unwrap();
        assert!(lex_a.struct_weight_lex_compare(&lex_b, &bank) < 0);
    }

    #[test]
    fn ordinary_weight_helpers_apply_orientation_and_applied_variable_multipliers() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let fa = typed_unary(&mut bank, "f", &a);
        let mut eq = Eqn::alloc(fa, b, &mut bank, true).unwrap();

        assert_f64_bits_eq(eq.weight(2.0, 3, 5, 7.0), 30.0);
        assert_eq!(eq.standard_diff(), 2);
        assert_eq!(eq.count_maximal_literals(), 2);

        eq.set_prop(EP_IS_ORIENTED);
        assert_f64_bits_eq(eq.weight(2.0, 3, 5, 7.0), 25.0);
        assert_eq!(eq.count_maximal_literals(), 1);

        let type_ = bank.signature().type_bank().default_type();
        let x = bank.vars().var_assert_alloc(-2, &type_);
        let a = typed_const(&mut bank, "app_a");
        let b = typed_const(&mut bank, "app_b");
        let app = applied_free_var(&mut bank, &x, &a);
        let eq = Eqn::alloc(app, b, &mut bank, true).unwrap();

        assert_f64_bits_eq(eq.weight(2.0, 3, 5, 10.0), 170.0);
        assert_f64_bits_eq(eq.max_weight(3, 5, 10.0), 80.0);
    }

    #[test]
    fn weighted_sum_nonlinear_symbol_type_and_corrected_weights_match_c_shape() {
        let mut bank = test_bank();
        let type_ = bank.signature().type_bank().default_type();
        let x = bank.vars().var_assert_alloc(-2, &type_);
        let left = typed_binary(&mut bank, "f", &x, &x);
        let right = typed_const(&mut bank, "a");
        let mut eq = Eqn::alloc(left.clone(), right.clone(), &mut bank, true).unwrap();

        assert_f64_bits_eq(eq.non_linear_weight(3.0, 7, 2, 5, 1.0), 57.0);
        assert_f64_bits_eq(eq.sym_type_weight(2.0, 1, 2, 3, 11, 1.0), 14.0);

        let flimit = left.f_code().max(right.f_code()) + 1;
        let mut fweights = vec![0; usize::try_from(flimit).unwrap()];
        fweights[usize::try_from(left.f_code()).unwrap()] = 13;
        fweights[usize::try_from(right.f_code()).unwrap()] = 17;
        assert_f64_bits_eq(eq.fun_weight(2.0, 3, flimit, &fweights, 5, 1.0, None), 72.0);

        eq.set_prop(EP_IS_ORIENTED);
        assert_f64_bits_eq(eq.non_linear_weight(3.0, 7, 2, 5, 1.0), 47.0);
        assert_f64_bits_eq(eq.sym_type_weight(2.0, 1, 2, 3, 11, 1.0), 11.0);

        let a = typed_const(&mut bank, "eq_a");
        let b = typed_const(&mut bank, "eq_b");
        let eq = Eqn::alloc(a, b, &mut bank, true).unwrap();
        assert_f64_bits_eq(eq.corrected_weight(&bank, 2.0, 3, 5, 1.0), 25.0);
        assert_f64_bits_eq(
            eq.corrected_non_linear_weight(&bank, 2.0, 7, 2, 5, 1.0),
            25.0,
        );

        let pred = typed_pred_const(&mut bank, "p");
        let lit = Eqn::alloc(pred, bank.true_term().clone(), &mut bank, true).unwrap();
        assert_f64_bits_eq(lit.corrected_weight(&bank, 2.0, 3, 5, 1.0), 10.0);
    }

    #[test]
    fn literal_weight_helpers_apply_literal_and_polarity_multipliers() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let mut lit = Eqn::alloc(a.clone(), b.clone(), &mut bank, true).unwrap();
        lit.set_prop(EP_IS_MAXIMAL);

        assert_f64_bits_eq(
            lit.literal_weight(&bank, 2.0, 3.0, 4.0, 3, 5, 1.0, false),
            300.0,
        );
        assert_f64_bits_eq(
            lit.literal_non_linear_weight(&bank, 2.0, 3.0, 4.0, 7, 2, 5, 1.0, false),
            300.0,
        );
        assert_f64_bits_eq(
            lit.literal_sym_type_weight(2.0, 3.0, 4.0, 1, 2, 3, 11, 1.0),
            144.0,
        );

        let flimit = a.f_code().max(b.f_code()) + 1;
        let mut fweights = vec![0; usize::try_from(flimit).unwrap()];
        fweights[usize::try_from(a.f_code()).unwrap()] = 11;
        fweights[usize::try_from(b.f_code()).unwrap()] = 13;
        assert_f64_bits_eq(
            lit.literal_fun_weight(2.0, 3.0, 4.0, 3, flimit, &fweights, 5, 1.0, None),
            576.0,
        );

        let mut neg = Eqn::alloc(a, b, &mut bank, false).unwrap();
        neg.set_prop(EP_IS_MAXIMAL);
        assert_f64_bits_eq(
            neg.literal_weight(&bank, 2.0, 3.0, 4.0, 3, 5, 1.0, false),
            75.0,
        );
    }

    #[test]
    fn dag_weight_helpers_share_flags_like_c_term_dag_weight() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let fa = typed_unary(&mut bank, "f", &a);
        let ga = typed_unary(&mut bank, "g", &a);
        let mut eq = Eqn::alloc(fa, ga, &mut bank, true).unwrap();

        assert_f64_bits_eq(eq.dag_weight(5.0, 2.0, 3, 10, 1, true, false), 62.0);
        assert!(a.query_prop(TP_OP_FLAG));

        assert_f64_bits_eq(eq.dag_weight(5.0, 2.0, 3, 10, 1, true, true), 80.0);
        assert_f64_bits_eq(eq.dag_weight2(4.0, 3, 10, 1), 100.0);

        eq.set_prop(EP_IS_ORIENTED);
        assert_f64_bits_eq(eq.dag_weight(5.0, 2.0, 3, 10, 1, true, false), 211.0);
    }

    #[test]
    fn position_count_helpers_preserve_c_orientation_asymmetry() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let fa = typed_unary(&mut bank, "f", &a);
        let gb = typed_unary(&mut bank, "g", &b);
        let mut eq = Eqn::alloc(fa, gb, &mut bank, true).unwrap();

        assert_eq!(eq.max_term_positions(), 4);
        assert_eq!(eq.inference_positions(), 2);

        eq.set_prop(EP_IS_ORIENTED);
        assert_eq!(eq.max_term_positions(), 2);
        assert_eq!(eq.inference_positions(), 4);
    }

    #[test]
    fn technical_compare_depth_app_var_untyped_and_map_helpers_match_c_shape() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let fa = typed_unary(&mut bank, "f", &a);
        let mut eq = Eqn::alloc(fa, b.clone(), &mut bank, true).unwrap();
        let reversed = Eqn::alloc(b.clone(), eq.left().clone(), &mut bank, true).unwrap();
        let negative = Eqn::alloc(eq.left().clone(), b.clone(), &mut bank, false).unwrap();

        assert_eq!(eq.literal_compare_fun(&reversed), 0);
        assert_eq!(eq.literal_compare_fun(&negative), 1);
        assert_eq!(negative.literal_compare_fun(&eq), -1);
        assert_eq!(eq.depth(), 2);
        assert!(eq.is_untyped());

        let type_ = bank.signature().type_bank().default_type();
        let x = bank.vars().var_assert_alloc(-2, &type_);
        let app = applied_free_var(&mut bank, &x, &a);
        let app_eq = Eqn::alloc(app, b.clone(), &mut bank, true).unwrap();
        assert!(app_eq.has_app_var());

        let c = typed_const(&mut bank, "c");
        eq.set_prop(EP_IS_ORIENTED | EP_MAX_IS_UP_TO_DATE);
        eq.map_terms(
            &bank,
            |term| {
                if term == &b {
                    c.clone()
                } else {
                    term.clone()
                }
            },
        );
        assert_eq!(eq.left(), reversed.right());
        assert_eq!(eq.right(), &c);
        assert!(eq.is_oriented());
        assert!(eq.query_prop(EP_MAX_IS_UP_TO_DATE));

        let old_left = eq.left().clone();
        let false_term = bank.false_term().clone();
        eq.map_terms(&bank, |term| {
            if term == &old_left {
                false_term.clone()
            } else {
                term.clone()
            }
        });
        assert_eq!(eq.left(), &c);
        assert_eq!(eq.right(), bank.true_term());
        assert!(eq.is_negative());
        assert!(!eq.is_oriented());
        assert!(!eq.query_prop(EP_MAX_IS_UP_TO_DATE));
        assert!(!eq.query_prop(EP_IS_EQU_LITERAL));
    }

    #[test]
    fn collection_and_symbol_feature_wrappers_apply_both_equation_sides() {
        let mut bank = test_bank();
        let type_ = bank.signature().type_bank().default_type();
        let x = bank.vars().var_assert_alloc(-2, &type_);
        x.set_prop(TP_CHECK_FLAG);
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let left = typed_binary(&mut bank, "f", &x, &a);
        let right = typed_unary(&mut bank, "g", &b);
        let eq = Eqn::alloc(left.clone(), right.clone(), &mut bank, true).unwrap();
        let max_code = [left.f_code(), right.f_code(), a.f_code(), b.f_code()]
            .into_iter()
            .max()
            .unwrap();
        let array_len = usize::try_from(max_code + 1).unwrap();

        let mut vars = BTreeMap::new();
        assert_eq!(eq.collect_variables(&mut vars), 1);
        assert_eq!(vars.values().next(), Some(&x));

        let mut prop_vars = BTreeMap::new();
        assert_eq!(eq.collect_prop_variables(&mut prop_vars, TP_CHECK_FLAG), 1);
        assert_eq!(prop_vars.values().next(), Some(&x));

        let mut fcodes = BTreeSet::new();
        assert_eq!(eq.collect_fcodes(&mut fcodes), 4);
        assert!(fcodes.contains(&left.f_code()));
        assert!(fcodes.contains(&right.f_code()));
        assert!(fcodes.contains(&a.f_code()));
        assert!(fcodes.contains(&b.f_code()));

        let mut dist = vec![0; array_len];
        eq.add_symbol_distribution(&mut dist);
        assert_eq!(dist[usize::try_from(left.f_code()).unwrap()], 1);
        assert_eq!(dist[usize::try_from(a.f_code()).unwrap()], 1);
        assert_eq!(dist[usize::try_from(right.f_code()).unwrap()], 1);
        assert_eq!(dist[usize::try_from(b.f_code()).unwrap()], 1);

        let mut limited = vec![0; array_len];
        eq.add_symbol_distribution_limited(&mut limited, usize::try_from(right.f_code()).unwrap());
        assert_eq!(limited[usize::try_from(left.f_code()).unwrap()], 1);
        assert_eq!(limited[usize::try_from(right.f_code()).unwrap()], 0);

        let mut exists_dist = vec![0; array_len];
        let mut exists = Vec::new();
        eq.add_symbol_dist_exist(&mut exists_dist, &mut exists);
        let exists_set = exists.into_iter().collect::<BTreeSet<_>>();
        assert_eq!(exists_set, fcodes);

        let mut freq = vec![0; array_len];
        let mut depth = vec![0; array_len];
        eq.add_symbol_features_limited(&mut freq, &mut depth, array_len);
        assert_eq!(freq[usize::try_from(left.f_code()).unwrap()], 1);
        assert_eq!(depth[usize::try_from(left.f_code()).unwrap()], 0);
        assert_eq!(freq[usize::try_from(a.f_code()).unwrap()], 1);
        assert_eq!(depth[usize::try_from(a.f_code()).unwrap()], 1);

        let mut feature_array = vec![0; 4 * array_len + 4];
        let mut mod_stack = Vec::new();
        eq.add_symbol_features(&mut mod_stack, &mut feature_array);
        let left_feature = usize::try_from(4 * left.f_code()).unwrap();
        assert!(mod_stack.contains(&left_feature));
        assert_eq!(feature_array[left_feature], 1);
        assert_eq!(feature_array[left_feature + 1], 0);

        let mut ranks = vec![0; array_len];
        let mut count = 1;
        eq.compute_function_ranks(&mut ranks, &mut count);
        assert_eq!(count, 5);
        assert_ne!(ranks[usize::try_from(left.f_code()).unwrap()], 0);
        assert_ne!(ranks[usize::try_from(right.f_code()).unwrap()], 0);

        let mut f_occur = PDIntArray::new_int(1, GROW_EXPONENTIAL);
        let mut occ_stack = Vec::new();
        assert_eq!(eq.add_fun_occs(&mut f_occur, &mut occ_stack), 4);
        assert!(occ_stack.contains(&left.f_code()));
        assert!(occ_stack.contains(&right.f_code()));

        let mut ground = BTreeMap::new();
        assert_eq!(eq.collect_ground_terms(&mut ground, false), 1);
        assert_eq!(ground.values().next(), Some(&right));

        let ground_left = typed_unary(&mut bank, "h", &a);
        let ground_eq = Eqn::alloc(ground_left, right, &mut bank, true).unwrap();
        let mut collector = PStack::new();
        assert_eq!(ground_eq.collect_subterms(&mut collector), 4);
        assert_eq!(ground_eq.collect_subterms(&mut collector), 0);
    }

    #[test]
    fn equality_helpers_follow_commutative_and_deref_rules() {
        let mut bank = test_bank();
        let left = typed_const(&mut bank, "a");
        let right = typed_const(&mut bank, "b");
        let eq = Eqn::alloc(left.clone(), right.clone(), &mut bank, true).unwrap();
        let directed = Eqn::alloc(left.clone(), right.clone(), &mut bank, true).unwrap();
        let swapped = Eqn::alloc(right.clone(), left.clone(), &mut bank, true).unwrap();

        assert!(eq.equal_directed(&directed));
        assert!(eq.equal(&swapped));
        assert!(eq.literal_equal(&swapped));

        let negative_swapped = Eqn::alloc(right.clone(), left.clone(), &mut bank, false).unwrap();
        assert!(eq.equal(&negative_swapped));
        assert!(!eq.literal_equal(&negative_swapped));

        let mut oriented_left = Eqn::alloc(left.clone(), right.clone(), &mut bank, true).unwrap();
        let mut oriented_right = Eqn::alloc(right.clone(), left.clone(), &mut bank, true).unwrap();
        oriented_left.set_prop(EP_IS_ORIENTED);
        oriented_right.set_prop(EP_IS_ORIENTED);
        assert!(!oriented_left.equal(&oriented_right));

        let type_ = bank.signature().type_bank().default_type();
        let x = bank.vars().var_assert_alloc(-2, &type_);
        x.set_binding(Some(left.clone()));
        let with_var = Eqn::alloc(x, right.clone(), &mut bank, true).unwrap();
        let concrete = Eqn::alloc(left, right, &mut bank, true).unwrap();
        assert!(!with_var.equal_directed_deref(&concrete, DerefType::Never, DerefType::Never));
        assert!(with_var.equal_directed_deref(&concrete, DerefType::Once, DerefType::Never));
    }

    #[test]
    fn subsumption_ordering_helpers_match_c_quasi_orders() {
        let mut bank = test_bank();
        let left = typed_const(&mut bank, "a");
        let right = typed_const(&mut bank, "b");

        let positive = Eqn::alloc(left.clone(), right.clone(), &mut bank, true).unwrap();
        let negative = Eqn::alloc(left.clone(), right.clone(), &mut bank, false).unwrap();
        assert_eq!(
            positive.subsume_q_order_compare_for_problem_type(
                &negative,
                &bank,
                ProblemType::FirstOrder
            ),
            1
        );
        assert_eq!(
            negative.subsume_q_order_compare_for_problem_type(
                &positive,
                &bank,
                ProblemType::FirstOrder
            ),
            -1
        );

        let p = typed_pred_const(&mut bank, "p");
        let q = typed_pred_const(&mut bank, "q");
        let p_lit = Eqn::alloc(p.clone(), bank.true_term().clone(), &mut bank, true).unwrap();
        let q_lit = Eqn::alloc(q.clone(), bank.true_term().clone(), &mut bank, true).unwrap();

        assert_eq!(
            positive.subsume_q_order_compare_for_problem_type(
                &p_lit,
                &bank,
                ProblemType::FirstOrder
            ),
            1
        );
        assert_eq!(
            p_lit.subsume_q_order_compare_for_problem_type(&q_lit, &bank, ProblemType::FirstOrder),
            super::cmp_i64(p.f_code(), q.f_code())
        );
        assert_eq!(
            p_lit.subsume_q_order_compare_for_problem_type(&q_lit, &bank, ProblemType::HigherOrder),
            0
        );
        assert_eq!(
            p_lit.subsume_q_order_compare_for_problem_type(
                &q_lit,
                &bank,
                ProblemType::NotInitialized
            ),
            0
        );

        let heavy_left = typed_unary(&mut bank, "f", &left);
        let heavy = Eqn::alloc(heavy_left, right, &mut bank, true).unwrap();
        assert!(positive.subsume_inverse_compare(&heavy, &bank) > 0);
        assert!(positive.subsume_compare(&heavy, &bank) < 0);

        let mut first = Eqn::alloc(left.clone(), left.clone(), &mut bank, true).unwrap();
        let mut second = Eqn::alloc(left.clone(), left, &mut bank, true).unwrap();
        first.set_position(10);
        second.set_position(20);
        assert!(first.subsume_inverse_refined_compare(&second, &bank) < 0);
        assert!(second.subsume_inverse_refined_compare(&first, &bank) > 0);
    }

    #[test]
    fn substitution_subsumption_helpers_backtrack_like_c() {
        let mut bank = test_bank();
        let type_ = bank.signature().type_bank().default_type();
        let x = bank.vars().var_assert_alloc(-2, &type_);
        let y = bank.vars().var_assert_alloc(-4, &type_);
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");

        let pattern = Eqn::alloc(x.clone(), b.clone(), &mut bank, true).unwrap();
        let target = Eqn::alloc(a.clone(), b.clone(), &mut bank, true).unwrap();
        let mut subst = Substitution::new();
        assert!(pattern.subsume_directed(&target, &mut subst));
        assert_eq!(subst.len(), 1);
        assert_eq!(x.binding(), Some(a.clone()));
        subst.backtrack();

        let failing = Eqn::alloc(y.clone(), y.clone(), &mut bank, true).unwrap();
        assert!(!failing.subsume_directed(&target, &mut subst));
        assert!(subst.is_empty());
        assert!(y.binding().is_none());

        let swapped_subsumed = Eqn::alloc(b.clone(), a.clone(), &mut bank, true).unwrap();
        assert!(pattern.subsume(&swapped_subsumed, &mut subst));
        assert_eq!(x.binding(), Some(a.clone()));
        subst.backtrack();

        let mut oriented = pattern.clone();
        oriented.set_prop(EP_IS_ORIENTED);
        assert!(!oriented.subsume(&swapped_subsumed, &mut subst));
        assert!(subst.is_empty());
        assert!(x.binding().is_none());

        assert!(pattern.subsume_p(&target));
        assert!(x.binding().is_none());
        assert!(pattern.literal_subsume_p(&target));

        let negative = Eqn::alloc(a, b, &mut bank, false).unwrap();
        assert!(!pattern.literal_subsume_p(&negative));
    }

    #[test]
    fn unification_helpers_backtrack_and_preserve_literal_side_effects() {
        let mut bank = test_bank();
        let type_ = bank.signature().type_bank().default_type();
        let x = bank.vars().var_assert_alloc(-2, &type_);
        let y = bank.vars().var_assert_alloc(-4, &type_);
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");

        let first = Eqn::alloc(x.clone(), b.clone(), &mut bank, true).unwrap();
        let second = Eqn::alloc(a.clone(), b.clone(), &mut bank, true).unwrap();
        let mut subst = Substitution::new();
        assert!(first.unify_directed(&second, &mut subst));
        assert_eq!(subst.len(), 1);
        assert_eq!(x.binding(), Some(a.clone()));
        subst.backtrack();

        let failing = Eqn::alloc(y.clone(), y.clone(), &mut bank, true).unwrap();
        assert!(!failing.unify_directed(&second, &mut subst));
        assert!(subst.is_empty());
        assert!(y.binding().is_none());

        let swapped_second = Eqn::alloc(b.clone(), a.clone(), &mut bank, true).unwrap();
        assert!(first.unify(&swapped_second, &mut subst));
        assert_eq!(x.binding(), Some(a.clone()));
        subst.backtrack();

        let mut oriented_first = first.clone();
        let mut oriented_second = swapped_second.clone();
        oriented_first.set_prop(EP_IS_ORIENTED);
        oriented_second.set_prop(EP_IS_ORIENTED);
        assert!(!oriented_first.unify(&oriented_second, &mut subst));
        assert!(subst.is_empty());
        assert!(x.binding().is_none());

        assert!(first.unify_p(&second));
        assert!(x.binding().is_none());

        let mut literal = Eqn::alloc(b.clone(), a, &mut bank, true).unwrap();
        literal.set_prop(EP_IS_ORIENTED | EP_MAX_IS_UP_TO_DATE);
        assert!(first.literal_unify_one_way(&mut literal, &mut subst, true));
        assert_eq!(literal.left(), &b);
        assert!(!literal.is_oriented());
        assert!(!literal.query_prop(EP_MAX_IS_UP_TO_DATE));
        assert!(x.binding().is_some());
        subst.backtrack();

        let mut negative = Eqn::alloc(b, second.left().clone(), &mut bank, false).unwrap();
        negative.set_prop(EP_IS_ORIENTED);
        assert!(!first.literal_unify_one_way(&mut negative, &mut subst, true));
        assert!(negative.is_oriented());
    }

    #[test]
    fn syntax_comparison_uses_equational_class_and_entry_numbers() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let c = typed_const(&mut bank, "c");

        let base_literal = Eqn::alloc(a.clone(), b.clone(), &mut bank, true).unwrap();
        let reversed_literal = Eqn::alloc(b.clone(), a.clone(), &mut bank, false).unwrap();
        assert_eq!(base_literal.syntax_compare(&reversed_literal, &bank), 0);
        assert_eq!(
            base_literal.literal_syntax_compare(&reversed_literal, &bank),
            -1
        );

        let larger_literal = Eqn::alloc(a, c, &mut bank, true).unwrap();
        assert!(base_literal.syntax_compare(&larger_literal, &bank) < 0);
        assert!(larger_literal.syntax_compare(&base_literal, &bank) > 0);

        let atom = typed_pred_const(&mut bank, "p");
        let predicate = Eqn::alloc(atom, bank.true_term().clone(), &mut bank, true).unwrap();
        assert!(base_literal.syntax_compare(&predicate, &bank) < 0);
        assert!(predicate.syntax_compare(&base_literal, &bank) > 0);
    }

    #[test]
    fn term_property_helpers_touch_both_sides() {
        let mut bank = test_bank();
        let left = typed_const(&mut bank, "a");
        let right = typed_const(&mut bank, "b");
        let eq = Eqn::alloc(left.clone(), right.clone(), &mut bank, true).unwrap();

        eq.term_set_prop(TP_CHECK_FLAG);
        assert!(left.query_prop(TP_CHECK_FLAG));
        assert!(right.query_prop(TP_CHECK_FLAG));
        assert_eq!(eq.tb_term_del_prop_count(TP_CHECK_FLAG), 2);
        assert!(!left.query_prop(TP_CHECK_FLAG));
        assert!(!right.query_prop(TP_CHECK_FLAG));

        eq.term_set_prop(TP_CHECK_FLAG);
        eq.term_del_prop(TP_CHECK_FLAG);
        assert!(!left.query_prop(TP_CHECK_FLAG));
        assert!(!right.query_prop(TP_CHECK_FLAG));
    }

    #[test]
    fn ac_trivial_uses_ac_normalization() {
        let mut bank = test_bank();
        let type_ = bank.signature().type_bank().default_type();
        let f_code = bank.signature_mut().insert_id("f", 2, false);
        bank.signature_mut()
            .declare_final_type(
                f_code,
                alloc_arrow_type(vec![type_.clone(), type_.clone(), type_]),
            )
            .unwrap();
        bank.signature_mut()
            .set_func_prop(f_code, FunctionProperties::IS_AC);
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");

        let left = Term::top_alloc(f_code, 2);
        left.set_argument(0, a.clone());
        left.set_argument(1, b.clone());
        let left = bank
            .insert(&left, crate::terms::termtypes::DerefType::Never)
            .unwrap();
        let right = Term::top_alloc(f_code, 2);
        right.set_argument(0, b);
        right.set_argument(1, a);
        let right = bank
            .insert(&right, crate::terms::termtypes::DerefType::Never)
            .unwrap();

        assert_eq!(term_standard_weight(&left), term_standard_weight(&right));
        let eq = Eqn::alloc(left, right, &mut bank, true).unwrap();
        assert!(eq.is_ac_trivial(&bank));
    }

    #[test]
    fn clausifiable_rejects_non_logical_predicate_literals() {
        let mut bank = test_bank();
        let atom = typed_pred_const(&mut bank, "p");
        let lit = Eqn::alloc(atom, bank.true_term().clone(), &mut bank, true).unwrap();

        assert!(!lit.is_clausifiable(&bank));

        let true_lit = Eqn::create_true_lit(&mut bank).unwrap();
        assert!(true_lit.is_clausifiable(&bank));
    }

    #[test]
    fn rarely_used_property_shortcuts_follow_bits() {
        let mut bank = test_bank();
        let left = typed_const(&mut bank, "a");
        let right = typed_const(&mut bank, "b");
        let mut eq = Eqn::alloc(left, right, &mut bank, true).unwrap();

        eq.set_prop(
            EP_IS_MAXIMAL
                | crate::clauses::eqn_props::EP_IS_STRICTLY_MAXIMAL
                | crate::clauses::eqn_props::EP_HAS_EQUIV
                | crate::clauses::eqn_props::EP_IS_DOMINATED
                | EP_IS_PM_INTO_LIT,
        );

        assert!(eq.is_maximal());
        assert!(eq.is_strictly_maximal());
        assert!(eq.has_equiv());
        assert!(eq.is_dominated());
        assert!(eq.dominates());
        assert!(eq.query_prop(EP_IS_PM_INTO_LIT));
    }
}
