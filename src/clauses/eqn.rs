use crate::basics::error::{Diagnostic, ErrorCode};
use crate::basics::partial_orderings::CompareResult;
use crate::basics::simple_stuff::{problem_type, ProblemType};
use crate::basics::{pdarrays::PDIntArray, pstacks::PStack};
use crate::clauses::eqn_props::{
    EqnProperties, EqnSide, PatEqnDirection, EP_IS_EQU_LITERAL, EP_IS_ORIENTED, EP_IS_POSITIVE,
    EP_MAX_IS_UP_TO_DATE, EP_NO_PROPS, EP_PSEUDO_LIT, EQUAL_PREDICATE,
};
use crate::heuristics::to_params::LiteralCmp;
use crate::inout::scanner::{IoFormat, Scanner, TokenType};
use crate::orderings::cto_orderings::to_compare;
use crate::orderings::ocb::OrderControlBlock;
use crate::terms::acterms::term_ac_equal;
use crate::terms::functypes::FunCode;
use crate::terms::match_mgu::{subst_match_complete, subst_mgu_complete};
use crate::terms::signature::{Signature, FP_CL_SPLIT_DEF, FP_PSEUDO_PRED};
use crate::terms::simpletypes::type_is_predicate;
use crate::terms::subst::Substitution;
use crate::terms::termbanks::{
    tb_term_collect_subterms, tb_term_del_prop_count, tb_term_is_ground, tb_term_is_type_term,
    tb_term_is_x_type_term, TermBank,
};
use crate::terms::termfunc::{
    term_add_fun_occ, term_add_symbol_dist_exist, term_add_symbol_distribution_limited,
    term_add_symbol_features, term_add_symbol_features_limited, term_add_type_distribution,
    term_app_encode, term_collect_fcodes, term_collect_ground_terms, term_collect_prop_variables,
    term_collect_variables, term_compute_function_ranks, term_dag_weight, term_depth,
    term_fsum_weight, term_has_f_code, term_is_def_term, term_is_untyped, term_lex_compare,
    term_non_linear_weight, term_standard_weight, term_struct_equal_deref,
    term_struct_weight_compare, term_sym_type_weight, term_weight_compute,
};
use crate::terms::termtypes::{
    term_del_prop, term_del_prop_opt, term_deref, term_identity_cmp, term_set_prop,
    term_var_del_prop, term_var_search_prop, term_var_set_prop, DerefType, Term, TermProperties,
    TP_OP_FLAG, TP_PRED_POS,
};
use crate::terms::termvars::VarBank;
use crate::terms::termweightext::TermWeightExtension;
use crate::terms::typecheck::type_declare_is_predicate;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

const DEFAULT_COMCHAR: &str = "%%";

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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[allow(clippy::struct_excessive_bools)]
pub struct EqnPrintOptions {
    pub output_format: IoFormat,
    pub use_infix: bool,
    pub full_equational_rep: bool,
    pub print_oriented: bool,
    pub higher_order_parentheses: bool,
    pub print_types: bool,
}

impl EqnPrintOptions {
    #[must_use]
    pub const fn lop() -> Self {
        Self {
            output_format: IoFormat::Lop,
            use_infix: true,
            full_equational_rep: false,
            print_oriented: false,
            higher_order_parentheses: false,
            print_types: false,
        }
    }

    #[must_use]
    pub const fn tptp() -> Self {
        Self {
            output_format: IoFormat::Tptp,
            use_infix: false,
            full_equational_rep: false,
            print_oriented: false,
            higher_order_parentheses: false,
            print_types: false,
        }
    }

    #[must_use]
    pub const fn with_print_types(mut self, print_types: bool) -> Self {
        self.print_types = print_types;
        self
    }
}

impl Default for EqnPrintOptions {
    fn default() -> Self {
        Self::lop()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EqnFofPrintOptions {
    pub output_format: IoFormat,
    pub pcl: bool,
    pub higher_order_parentheses: bool,
    pub print_types: bool,
}

impl EqnFofPrintOptions {
    #[must_use]
    pub const fn lop() -> Self {
        Self {
            output_format: IoFormat::Lop,
            pcl: false,
            higher_order_parentheses: false,
            print_types: false,
        }
    }

    #[must_use]
    pub const fn tptp() -> Self {
        Self {
            output_format: IoFormat::Tptp,
            pcl: false,
            higher_order_parentheses: false,
            print_types: false,
        }
    }

    #[must_use]
    pub const fn tstp() -> Self {
        Self {
            output_format: IoFormat::Tstp,
            pcl: false,
            higher_order_parentheses: false,
            print_types: false,
        }
    }

    #[must_use]
    pub const fn with_print_types(mut self, print_types: bool) -> Self {
        self.print_types = print_types;
        self
    }
}

impl Default for EqnFofPrintOptions {
    fn default() -> Self {
        Self::lop()
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

    pub(crate) fn set_left_raw(&mut self, term: Term) {
        self.lterm = term;
    }

    pub(crate) fn set_right_raw(&mut self, term: Term) {
        self.rterm = term;
    }

    pub fn swap_sides(&mut self) {
        self.del_prop(EP_IS_ORIENTED);
        self.del_prop(EP_MAX_IS_UP_TO_DATE);
        self.swap_sides_simple();
    }

    /// Orient this equation with the selected term ordering.
    ///
    /// Returns `true` if the sides were swapped, matching C `EqnOrient`.
    ///
    /// # Panics
    ///
    /// Panics if the selected ordering cannot compare terms or if it returns an
    /// internal cache-only relation where C asserts in the default switch arm.
    pub fn orient(&mut self, ocb: &mut OrderControlBlock, bank: &TermBank) -> bool {
        if self.query_prop(EP_MAX_IS_UP_TO_DATE) {
            return false;
        }

        let relation = if self.lterm == self.rterm {
            CompareResult::Equal
        } else if self.lterm == *bank.true_term() {
            CompareResult::Lesser
        } else if self.rterm == *bank.true_term() {
            CompareResult::Greater
        } else {
            compare_terms(ocb, bank, &self.lterm, &self.rterm)
        };

        let swapped = match relation {
            CompareResult::Uncomparable | CompareResult::Equal => {
                self.del_prop(EP_IS_ORIENTED);
                false
            }
            CompareResult::Greater => {
                self.set_prop(EP_IS_ORIENTED);
                false
            }
            CompareResult::Lesser => {
                self.swap_sides();
                self.set_prop(EP_IS_ORIENTED);
                true
            }
            CompareResult::Unknown
            | CompareResult::NotGreaterEqual
            | CompareResult::NotLessEqual => {
                panic!("unexpected equation orientation relation: {relation:?}")
            }
        };
        self.set_prop(EP_MAX_IS_UP_TO_DATE);
        swapped
    }

    /// Compare two equations as multisets of terms, matching C `EqnCompare`.
    ///
    /// # Panics
    ///
    /// Panics if the equations do not have equivalent polarity, matching the C
    /// assertion in the shared positive-equation comparison helper.
    #[must_use]
    pub fn order_compare(
        &self,
        ocb: &mut OrderControlBlock,
        bank: &TermBank,
        other: &Self,
    ) -> CompareResult {
        compare_pos_eqns(ocb, bank, self, other)
    }

    /// Return whether this equation is greater than `other` under `ocb`.
    ///
    /// # Panics
    ///
    /// Panics under the same invariants as [`Self::order_compare`].
    #[must_use]
    pub fn order_greater(
        &self,
        ocb: &mut OrderControlBlock,
        bank: &TermBank,
        other: &Self,
    ) -> bool {
        self.order_compare(ocb, bank, other) == CompareResult::Greater
    }

    /// Compare two signed literals under the selected literal comparison mode.
    ///
    /// # Panics
    ///
    /// Panics under the selected term ordering's internal invariants, or if a
    /// literal/equational property bit is inconsistent with its `$true` shape.
    #[must_use]
    pub fn literal_compare(
        &self,
        ocb: &mut OrderControlBlock,
        bank: &TermBank,
        other: &Self,
    ) -> CompareResult {
        let self_pseudo = self.query_prop(EP_PSEUDO_LIT);
        let other_pseudo = other.query_prop(EP_PSEUDO_LIT);
        if self_pseudo && !other_pseudo {
            return CompareResult::Lesser;
        }
        if other_pseudo && !self_pseudo {
            return CompareResult::Greater;
        }

        if !self.is_selected() {
            if other.is_selected() {
                return CompareResult::Lesser;
            }
        } else if !other.is_selected() {
            return CompareResult::Greater;
        } else if self.is_positive() != other.is_positive() {
            return CompareResult::Uncomparable;
        }

        if ocb.lit_cmp == LiteralCmp::NoCmp {
            return CompareResult::Uncomparable;
        }

        let tfo_result = tfo_literal_compare(ocb, bank, self, other);
        if matches!(tfo_result, CompareResult::Greater | CompareResult::Lesser) {
            return tfo_result;
        }

        if self.is_positive() == other.is_positive() {
            compare_pos_eqns(ocb, bank, self, other)
        } else if self.is_positive() {
            compare_poseqn_negeqn(ocb, bank, self, other)
        } else {
            compare_poseqn_negeqn(ocb, bank, other, self)
                .inverse()
                .unwrap_or_else(|| panic!("literal comparison produced unknown inverse"))
        }
    }

    /// Return whether this signed literal is greater than `other` under `ocb`.
    ///
    /// # Panics
    ///
    /// Panics under the same invariants as [`Self::literal_compare`].
    #[must_use]
    pub fn literal_greater(
        &self,
        ocb: &mut OrderControlBlock,
        bank: &TermBank,
        other: &Self,
    ) -> bool {
        self.literal_compare(ocb, bank, other) == CompareResult::Greater
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
    pub fn term_ext_weight<Data, WeightFun>(
        &self,
        extension: &TermWeightExtension<Data, WeightFun>,
    ) -> f64
    where
        WeightFun: Fn(&Term, &Data) -> f64,
    {
        let mut result = extension.term_weight(&self.rterm);
        if !self.is_oriented() {
            result *= extension.max_term_multiplier();
        }
        result += extension.term_weight(&self.lterm) * extension.max_term_multiplier();
        result
    }

    #[must_use]
    pub fn literal_term_ext_weight<Data, WeightFun>(
        &self,
        extension: &TermWeightExtension<Data, WeightFun>,
    ) -> f64
    where
        WeightFun: Fn(&Term, &Data) -> f64,
    {
        let mut result = self.term_ext_weight(extension);
        if self.is_maximal() {
            result *= extension.max_literal_multiplier();
        }
        if self.is_positive() {
            result *= extension.pos_eq_multiplier();
        }
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

    pub fn subst_norm(&self, subst: &mut Substitution, vars: &VarBank) -> usize {
        let result = subst.norm_term(&self.lterm, vars);
        subst.norm_term(&self.rterm, vars);
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

    pub fn add_type_distribution(&self, sig: &mut Signature, type_array: &mut [i64]) {
        term_add_type_distribution(&self.lterm, sig, type_array);
        term_add_type_distribution(&self.rterm, sig, type_array);
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
        let shape_props = EqnProperties::from_bits(
            EP_IS_POSITIVE.bits() | EP_IS_EQU_LITERAL.bits() | EP_PSEUDO_LIT.bits(),
        );
        self.properties = self.give_props(shape_props) | (source.properties & !shape_props);
    }
}

fn compare_terms(
    ocb: &mut OrderControlBlock,
    bank: &TermBank,
    left: &Term,
    right: &Term,
) -> CompareResult {
    to_compare(
        ocb,
        bank.signature(),
        left,
        right,
        DerefType::Always,
        DerefType::Always,
    )
}

fn is_greater_or_equal(relation: CompareResult) -> bool {
    matches!(relation, CompareResult::Greater | CompareResult::Equal)
}

fn is_lesser_or_equal(relation: CompareResult) -> bool {
    matches!(relation, CompareResult::Lesser | CompareResult::Equal)
}

fn compare_pos_eqns(
    ocb: &mut OrderControlBlock,
    bank: &TermBank,
    left: &Eqn,
    right: &Eqn,
) -> CompareResult {
    assert_eq!(
        left.is_positive(),
        right.is_positive(),
        "EqnCompare requires equivalent literal polarity"
    );

    let left_left_relation = compare_terms(ocb, bank, left.left(), right.left());
    let right_right_relation = compare_terms(ocb, bank, left.right(), right.right());

    if left_left_relation == CompareResult::Equal && right_right_relation == CompareResult::Equal {
        return CompareResult::Equal;
    }
    if is_greater_or_equal(left_left_relation) && is_greater_or_equal(right_right_relation) {
        return CompareResult::Greater;
    }
    if is_lesser_or_equal(left_left_relation) && is_lesser_or_equal(right_right_relation) {
        return CompareResult::Lesser;
    }

    let left_right_relation = compare_terms(ocb, bank, left.left(), right.right());

    if left_left_relation == CompareResult::Greater && left_right_relation == CompareResult::Greater
    {
        return CompareResult::Greater;
    }
    if left_right_relation == CompareResult::Lesser && right_right_relation == CompareResult::Lesser
    {
        return CompareResult::Lesser;
    }

    let right_left_relation = compare_terms(ocb, bank, left.right(), right.left());

    if left_right_relation == CompareResult::Equal && right_left_relation == CompareResult::Equal {
        return CompareResult::Equal;
    }
    if is_greater_or_equal(right_left_relation) && is_greater_or_equal(left_right_relation) {
        return CompareResult::Greater;
    }
    if right_left_relation == CompareResult::Greater
        && right_right_relation == CompareResult::Greater
    {
        return CompareResult::Greater;
    }
    if left_left_relation == CompareResult::Lesser && right_left_relation == CompareResult::Lesser {
        return CompareResult::Lesser;
    }
    if is_lesser_or_equal(right_left_relation) && is_lesser_or_equal(left_right_relation) {
        return CompareResult::Lesser;
    }

    CompareResult::Uncomparable
}

fn compare_poseqn_negeqn(
    ocb: &mut OrderControlBlock,
    bank: &TermBank,
    positive: &Eqn,
    negative: &Eqn,
) -> CompareResult {
    assert!(positive.is_positive(), "left literal must be positive");
    assert!(negative.is_negative(), "right literal must be negative");

    let left_left_relation = compare_terms(ocb, bank, positive.left(), negative.left());

    if positive.is_oriented() {
        if is_lesser_or_equal(left_left_relation) {
            return CompareResult::Lesser;
        }

        let left_right_relation = compare_terms(ocb, bank, positive.left(), negative.right());

        if is_lesser_or_equal(left_right_relation) {
            return CompareResult::Lesser;
        }
        if left_left_relation == CompareResult::Greater
            && left_right_relation == CompareResult::Greater
        {
            return CompareResult::Greater;
        }
    } else {
        let left_right_relation = compare_terms(ocb, bank, positive.left(), negative.right());

        if left_left_relation == CompareResult::Greater
            && left_right_relation == CompareResult::Greater
        {
            return CompareResult::Greater;
        }

        let right_left_relation = compare_terms(ocb, bank, positive.right(), negative.left());
        let right_right_relation = compare_terms(ocb, bank, positive.right(), negative.right());

        if right_left_relation == CompareResult::Greater
            && right_right_relation == CompareResult::Greater
        {
            return CompareResult::Greater;
        }
        if (is_lesser_or_equal(left_left_relation) || is_lesser_or_equal(left_right_relation))
            && (is_lesser_or_equal(right_left_relation) || is_lesser_or_equal(right_right_relation))
        {
            return CompareResult::Lesser;
        }
    }

    CompareResult::Uncomparable
}

fn tfo_literal_compare(
    ocb: &OrderControlBlock,
    bank: &TermBank,
    left: &Eqn,
    right: &Eqn,
) -> CompareResult {
    if ocb.lit_cmp == LiteralCmp::TfoEqMax {
        if left.is_equ_lit(bank) && !right.is_equ_lit(bank) {
            return CompareResult::Greater;
        }
        if !left.is_equ_lit(bank) && right.is_equ_lit(bank) {
            return CompareResult::Lesser;
        }
        if !left.is_equ_lit(bank) && !left.left().is_free_var() && !right.left().is_free_var() {
            return ocb.fun_compare(
                bank.signature(),
                left.left().f_code(),
                right.left().f_code(),
            );
        }
    } else if ocb.lit_cmp == LiteralCmp::TfoEqMin {
        if left.is_equ_lit(bank) && !right.is_equ_lit(bank) {
            return CompareResult::Lesser;
        }
        if !left.is_equ_lit(bank) && right.is_equ_lit(bank) {
            return CompareResult::Greater;
        }
        if !left.is_equ_lit(bank) && !left.left().is_free_var() && !right.left().is_free_var() {
            return ocb.fun_compare(
                bank.signature(),
                left.left().f_code(),
                right.left().f_code(),
            );
        }
    }
    CompareResult::Unknown
}

/// Writes the C `EqnPrint` shape with explicit term-bank and format options.
///
/// # Panics
///
/// Panics if the literal's equational-property bit and right-hand `$true`
/// shape are inconsistent, or if a printed term has an uninitialized
/// argument, matching the C preconditions.
pub fn eqn_write(
    output: &mut impl fmt::Write,
    bank: &TermBank,
    eqn: &Eqn,
    negated: bool,
    full_terms: bool,
    options: EqnPrintOptions,
) -> fmt::Result {
    let positive = eqn.is_positive() ^ negated;
    if options.output_format == IoFormat::Tptp {
        output.write_str(if positive { "++" } else { "--" })?;
        if eqn.is_equ_lit(bank) {
            write!(output, "{EQUAL_PREDICATE}(")?;
            bank.write_term_with_type_suffixes(
                output,
                eqn.left(),
                full_terms,
                options.print_types,
            )?;
            output.write_str(", ")?;
            bank.write_term_with_type_suffixes(
                output,
                eqn.right(),
                full_terms,
                options.print_types,
            )?;
            output.write_char(')')
        } else {
            bank.write_term_with_type_suffixes(output, eqn.left(), full_terms, options.print_types)
        }
    } else if options.use_infix && (options.full_equational_rep || eqn.right() != bank.true_term())
    {
        write_ho_paren(output, '(', options)?;
        bank.write_term_with_type_suffixes(output, eqn.left(), full_terms, options.print_types)?;
        if !positive {
            output.write_char('!')?;
        }
        output.write_str(if eqn.is_oriented() && options.print_oriented {
            "->"
        } else {
            "="
        })?;
        bank.write_term_with_type_suffixes(output, eqn.right(), full_terms, options.print_types)?;
        write_ho_paren(output, ')', options)
    } else {
        if !positive {
            output.write_char('~')?;
        }
        if eqn.right() != bank.true_term() || options.full_equational_rep {
            write!(output, "{EQUAL_PREDICATE}(")?;
            bank.write_term_with_type_suffixes(
                output,
                eqn.left(),
                full_terms,
                options.print_types,
            )?;
            output.write_str(", ")?;
            bank.write_term_with_type_suffixes(
                output,
                eqn.right(),
                full_terms,
                options.print_types,
            )?;
            output.write_char(')')
        } else {
            write_ho_paren(output, '(', options)?;
            bank.write_term_with_type_suffixes(
                output,
                eqn.left(),
                full_terms,
                options.print_types,
            )?;
            write_ho_paren(output, ')', options)
        }
    }
}

/// Writes the C `EqnPrintDeref` shape.
///
/// # Panics
///
/// Panics if dereferencing reaches an unsupported applied-variable binding or
/// if a printed term has an uninitialized argument, matching the current term
/// dereference and printing preconditions.
pub fn eqn_write_deref(
    output: &mut impl fmt::Write,
    bank: &TermBank,
    eqn: &Eqn,
    deref: DerefType,
) -> fmt::Result {
    let mut left_deref = deref;
    let left = term_deref(eqn.left(), &mut left_deref);
    bank.write_term(output, &left, true)?;
    output.write_str(if eqn.is_positive() { "=" } else { "!=" })?;
    let mut right_deref = deref;
    let right = term_deref(eqn.right(), &mut right_deref);
    bank.write_term(output, &right, true)
}

/// Writes the C `EqnPrintDBG` shape.
///
/// # Panics
///
/// Panics if a printed term violates the C debug term printing preconditions.
pub fn eqn_write_debug(
    output: &mut impl fmt::Write,
    bank: &TermBank,
    eqn: &Eqn,
    problem_type: ProblemType,
) -> fmt::Result {
    bank.write_term_debug(output, eqn.left(), problem_type)?;
    output.write_str(if eqn.is_positive() { "=" } else { "!=" })?;
    bank.write_term_debug(output, eqn.right(), problem_type)?;
    if eqn.is_maximal() {
        output.write_char('*')?;
    }
    if eqn.is_oriented() {
        output.write_char('>')?;
    }
    if eqn.query_prop(EP_IS_EQU_LITERAL) {
        output.write_str(DEFAULT_COMCHAR)?;
    }
    Ok(())
}

/// Parses the C `EqnParseInfix` shape using the currently ported term parser.
///
/// # Panics
///
/// Panics if predicate declaration invariants are violated, matching the C
/// parser assertions.
pub fn eqn_parse_infix(
    scanner: &mut Scanner,
    bank: &mut TermBank,
    problem_type: ProblemType,
) -> Result<Eqn, Diagnostic> {
    let (positive, left, right) = eqn_parse_infix_terms(scanner, bank, problem_type)?;
    Eqn::alloc(left, right, bank, positive)
}

/// Parses the C `EqnParse` shape using the currently ported term parser.
///
/// # Panics
///
/// Panics if the scanner format is `IoFormat::Auto`, matching the C assertion
/// that a concrete input format has been selected before literal parsing.
pub fn eqn_parse(
    scanner: &mut Scanner,
    bank: &mut TermBank,
    problem_type: ProblemType,
) -> Result<Eqn, Diagnostic> {
    eqn_parse_real(scanner, bank, false, problem_type)
}

/// Parses the C `EqnFOFParse` shape using the currently ported term parser.
///
/// # Panics
///
/// Panics if the scanner format is `IoFormat::Auto`, matching the C assertion
/// that a concrete input format has been selected before literal parsing.
pub fn eqn_fof_parse(
    scanner: &mut Scanner,
    bank: &mut TermBank,
    problem_type: ProblemType,
) -> Result<Eqn, Diagnostic> {
    eqn_parse_real(scanner, bank, true, problem_type)
}

fn eqn_parse_real(
    scanner: &mut Scanner,
    bank: &mut TermBank,
    fof: bool,
    problem_type: ProblemType,
) -> Result<Eqn, Diagnostic> {
    let mut negate = false;
    let (mut positive, left, right) = match scanner.format() {
        IoFormat::Lop => {
            if scanner.test_tok(TokenType::TILDE_SIGN) {
                negate = true;
                scanner.accept_tok(TokenType::TILDE_SIGN)?;
            }
            eqn_parse_mixfix_terms(scanner, bank, problem_type)?
        }
        IoFormat::Tptp => {
            if fof {
                if scanner.test_tok(TokenType::TILDE_SIGN) {
                    negate = true;
                    scanner.accept_tok(TokenType::TILDE_SIGN)?;
                }
            } else {
                scanner.check_tok(TokenType::PLUS | TokenType::HYPHEN)?;
                if scanner.test_tok(TokenType::HYPHEN) {
                    negate = true;
                    scanner.next_token()?;
                    scanner.accept_tok_no_skip(TokenType::HYPHEN)?;
                } else {
                    scanner.next_token()?;
                    scanner.accept_tok_no_skip(TokenType::PLUS)?;
                }
            }
            eqn_parse_prefix_terms(scanner, bank)?
        }
        IoFormat::Tstp => {
            if scanner.test_tok(TokenType::TILDE_SIGN) {
                negate = true;
                scanner.accept_tok(TokenType::TILDE_SIGN)?;
            }
            eqn_parse_infix_terms(scanner, bank, problem_type)?
        }
        IoFormat::Auto => panic!("format not supported"),
    };
    if negate {
        positive = !positive;
    }
    Eqn::alloc(left, right, bank, positive)
}

fn eqn_parse_mixfix_terms(
    scanner: &mut Scanner,
    bank: &mut TermBank,
    problem_type: ProblemType,
) -> Result<(bool, Term, Term), Diagnostic> {
    if scanner.test_id(EQUAL_PREDICATE) {
        eqn_parse_prefix_terms(scanner, bank)
    } else {
        eqn_parse_infix_terms(scanner, bank, problem_type)
    }
}

fn eqn_parse_prefix_terms(
    scanner: &mut Scanner,
    bank: &mut TermBank,
) -> Result<(bool, Term, Term), Diagnostic> {
    if scanner.test_id(EQUAL_PREDICATE) {
        scanner.accept_id(EQUAL_PREDICATE)?;
        scanner.accept_tok(TokenType::OPEN_BRACKET)?;
        let left = bank.parse_term_with_distinct_checks(scanner)?;
        scanner.accept_tok(TokenType::COMMA)?;
        let right = bank.parse_term_with_distinct_checks(scanner)?;
        scanner.accept_tok(TokenType::CLOSE_BRACKET)?;
        Ok((true, left, right))
    } else {
        let left = bank.parse_term_with_distinct_checks(scanner)?;
        prepare_predicate_literal(bank, &left)?;
        Ok((true, left, bank.true_term().clone()))
    }
}

fn eqn_parse_infix_terms(
    scanner: &mut Scanner,
    bank: &mut TermBank,
    problem_type: ProblemType,
) -> Result<(bool, Term, Term), Diagnostic> {
    let mut in_parens = false;
    if problem_type == ProblemType::HigherOrder && scanner.test_tok(TokenType::OPEN_BRACKET) {
        scanner.accept_tok(TokenType::OPEN_BRACKET)?;
        in_parens = true;
    }

    let left = bank.parse_term_with_distinct_checks(scanner)?;
    if in_parens && scanner.test_tok(TokenType::CLOSE_BRACKET) {
        scanner.accept_tok(TokenType::CLOSE_BRACKET)?;
        in_parens = false;
    }

    let mut positive = true;
    let right = if scanner.test_tok(TokenType::NEG_EQUAL_SIGN | TokenType::EQUAL_SIGN) {
        if scanner.test_tok(TokenType::NEG_EQUAL_SIGN) {
            positive = false;
        }
        scanner.accept_tok(TokenType::NEG_EQUAL_SIGN | TokenType::EQUAL_SIGN)?;
        bank.parse_term_with_distinct_checks(scanner)?
    } else {
        prepare_predicate_literal(bank, &left)?;
        bank.true_term().clone()
    };

    if in_parens {
        scanner.accept_tok(TokenType::CLOSE_BRACKET)?;
    }
    Ok((positive, left, right))
}

fn prepare_predicate_literal(bank: &mut TermBank, term: &Term) -> Result<(), Diagnostic> {
    if term.is_free_var() {
        if term.type_().as_ref().is_some_and(type_is_predicate) {
            return Ok(());
        }
        return Err(Diagnostic::new(
            ErrorCode::SYNTAX_ERROR,
            "Individual variable used at predicate position",
        ));
    }
    type_declare_is_predicate(bank.signature_mut(), term)
}

/// Writes the C `EqnAppEncode` shape without mutating the source literal.
///
/// # Errors
///
/// Returns a diagnostic if app-encoding/type inference fails, or if the output
/// writer reports a formatting error.
///
/// # Panics
///
/// Panics if the app-encoded term printer sees an uninitialized argument,
/// matching the C term printing preconditions.
pub fn eqn_write_app_encode(
    output: &mut impl fmt::Write,
    bank: &mut TermBank,
    eqn: &Eqn,
    negated: bool,
) -> Result<(), Diagnostic> {
    let positive = eqn.is_positive() ^ negated;
    let left = term_app_encode(eqn.left(), bank.signature_mut())?;
    if eqn.is_equ_lit(bank) {
        let right = term_app_encode(eqn.right(), bank.signature_mut())?;
        bank.write_term(output, &left, true)
            .map_err(|_| Diagnostic::new(ErrorCode::OTHER_ERROR, "failed to write equation"))?;
        if !positive {
            output
                .write_char('!')
                .map_err(|_| Diagnostic::new(ErrorCode::OTHER_ERROR, "failed to write equation"))?;
        }
        output
            .write_char('=')
            .map_err(|_| Diagnostic::new(ErrorCode::OTHER_ERROR, "failed to write equation"))?;
        bank.write_term(output, &right, true)
            .map_err(|_| Diagnostic::new(ErrorCode::OTHER_ERROR, "failed to write equation"))?;
    } else {
        if !positive {
            output
                .write_char('~')
                .map_err(|_| Diagnostic::new(ErrorCode::OTHER_ERROR, "failed to write equation"))?;
        }
        bank.write_term(output, &left, true)
            .map_err(|_| Diagnostic::new(ErrorCode::OTHER_ERROR, "failed to write equation"))?;
    }
    Ok(())
}

/// Writes the C `EqnFOFPrint` shape with explicit output-format switches.
///
/// # Panics
///
/// Panics if `output_format` is `IoFormat::Auto`, matching the C assertion
/// that only concrete LOP/TPTP/TSTP output formats are supported here, or if a
/// printed term violates the C term printing preconditions.
pub fn eqn_write_fof(
    output: &mut impl fmt::Write,
    bank: &TermBank,
    eqn: &Eqn,
    negated: bool,
    full_terms: bool,
    fof_options: EqnFofPrintOptions,
) -> fmt::Result {
    let positive = eqn.is_positive() ^ negated;
    let infix = match fof_options.output_format {
        IoFormat::Tptp => false,
        IoFormat::Tstp => true,
        IoFormat::Lop => !fof_options.pcl,
        IoFormat::Auto => panic!("format not supported"),
    };
    let options = EqnPrintOptions {
        output_format: fof_options.output_format,
        higher_order_parentheses: fof_options.higher_order_parentheses,
        print_types: fof_options.print_types,
        ..EqnPrintOptions::default()
    };

    if infix {
        if eqn.is_equ_lit(bank) {
            write_ho_paren(output, '(', options)?;
            write_ho_paren(output, '(', options)?;
            bank.write_term_with_type_suffixes(
                output,
                eqn.left(),
                full_terms,
                fof_options.print_types,
            )?;
            write_ho_paren(output, ')', options)?;
            if !positive {
                output.write_char('!')?;
            }
            output.write_char('=')?;
            write_ho_paren(output, '(', options)?;
            bank.write_term_with_type_suffixes(
                output,
                eqn.right(),
                full_terms,
                fof_options.print_types,
            )?;
            write_ho_paren(output, ')', options)?;
            write_ho_paren(output, ')', options)
        } else {
            if !positive {
                output.write_char('~')?;
            }
            write_ho_paren(output, '(', options)?;
            bank.write_term_with_type_suffixes(
                output,
                eqn.left(),
                full_terms,
                fof_options.print_types,
            )?;
            write_ho_paren(output, ')', options)
        }
    } else {
        if !positive {
            output.write_char('~')?;
        }
        if eqn.is_equ_lit(bank) {
            write!(output, "{EQUAL_PREDICATE}(")?;
            bank.write_term_with_type_suffixes(
                output,
                eqn.left(),
                full_terms,
                fof_options.print_types,
            )?;
            output.write_str(", ")?;
            bank.write_term_with_type_suffixes(
                output,
                eqn.right(),
                full_terms,
                fof_options.print_types,
            )?;
            output.write_char(')')
        } else {
            bank.write_term_with_type_suffixes(
                output,
                eqn.left(),
                full_terms,
                fof_options.print_types,
            )
        }
    }
}

/// Writes the C `EqnTSTPPrint` shape.
///
/// # Panics
///
/// Panics if the literal's equational-property bit and right-hand `$true`
/// shape are inconsistent, or if a printed term has an uninitialized
/// argument, matching the C preconditions.
pub fn eqn_write_tstp(
    output: &mut impl fmt::Write,
    bank: &TermBank,
    eqn: &Eqn,
    full_terms: bool,
    print_oriented: bool,
) -> fmt::Result {
    eqn_write_tstp_with_type_suffixes(output, bank, eqn, full_terms, print_oriented, false)
}

/// Writes the C `EqnTSTPPrint` shape with optional `TermPrintTypes` suffixes.
///
/// # Panics
///
/// Panics if the literal's equational-property bit and right-hand `$true`
/// shape are inconsistent, or if a printed term has an uninitialized argument
/// or missing type, matching the C preconditions.
pub fn eqn_write_tstp_with_type_suffixes(
    output: &mut impl fmt::Write,
    bank: &TermBank,
    eqn: &Eqn,
    full_terms: bool,
    print_oriented: bool,
    print_types: bool,
) -> fmt::Result {
    if eqn.is_prop_false() {
        return output.write_str("$false");
    }
    if eqn.is_equ_lit(bank) {
        bank.write_term_with_type_suffixes(output, eqn.left(), full_terms, print_types)?;
        if print_oriented && eqn.is_oriented() {
            output.write_str(if eqn.is_negative() { "!->" } else { "->" })?;
        } else {
            output.write_str(if eqn.is_negative() { "!=" } else { "=" })?;
        }
        bank.write_term_with_type_suffixes(output, eqn.right(), full_terms, print_types)
    } else {
        if eqn.is_negative() {
            output.write_char('~')?;
        }
        bank.write_term_with_type_suffixes(output, eqn.left(), full_terms, print_types)
    }
}

#[must_use]
pub fn eqn_string(
    bank: &TermBank,
    eqn: &Eqn,
    negated: bool,
    full_terms: bool,
    options: EqnPrintOptions,
) -> String {
    let mut output = String::new();
    let _ = eqn_write(&mut output, bank, eqn, negated, full_terms, options);
    output
}

#[must_use]
pub fn eqn_deref_string(bank: &TermBank, eqn: &Eqn, deref: DerefType) -> String {
    let mut output = String::new();
    let _ = eqn_write_deref(&mut output, bank, eqn, deref);
    output
}

#[must_use]
pub fn eqn_debug_string(bank: &TermBank, eqn: &Eqn, problem_type: ProblemType) -> String {
    let mut output = String::new();
    let _ = eqn_write_debug(&mut output, bank, eqn, problem_type);
    output
}

/// Returns the C `EqnAppEncode` rendering without mutating the source literal.
///
/// # Errors
///
/// Returns a diagnostic if app-encoding/type inference fails.
///
/// # Panics
///
/// Panics if the app-encoded term printer sees an uninitialized argument,
/// matching the C term printing preconditions.
pub fn eqn_app_encode_string(
    bank: &mut TermBank,
    eqn: &Eqn,
    negated: bool,
) -> Result<String, Diagnostic> {
    let mut output = String::new();
    eqn_write_app_encode(&mut output, bank, eqn, negated)?;
    Ok(output)
}

#[must_use]
pub fn eqn_fof_string(
    bank: &TermBank,
    eqn: &Eqn,
    negated: bool,
    full_terms: bool,
    fof_options: EqnFofPrintOptions,
) -> String {
    let mut output = String::new();
    let _ = eqn_write_fof(&mut output, bank, eqn, negated, full_terms, fof_options);
    output
}

#[must_use]
pub fn eqn_tstp_string(
    bank: &TermBank,
    eqn: &Eqn,
    full_terms: bool,
    print_oriented: bool,
) -> String {
    let mut output = String::new();
    let _ = eqn_write_tstp(&mut output, bank, eqn, full_terms, print_oriented);
    output
}

fn write_ho_paren(output: &mut impl fmt::Write, ch: char, options: EqnPrintOptions) -> fmt::Result {
    if options.higher_order_parentheses {
        output.write_char(ch)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        eqn_app_encode_string, eqn_debug_string, eqn_deref_string, eqn_fof_parse, eqn_fof_string,
        eqn_parse, eqn_string, eqn_tstp_string, Eqn, EqnFofPrintOptions, EqnPrintOptions,
    };
    use crate::basics::partial_orderings::{CompareResult, HoOrderKind};
    use crate::basics::pdarrays::{PDIntArray, GROW_EXPONENTIAL};
    use crate::basics::pstacks::PStack;
    use crate::basics::simple_stuff::ProblemType;
    use crate::clauses::eqn_props::{
        EqnSide, PatEqnDirection, EP_FROM_CLAUSE_LIT, EP_IS_EQU_LITERAL, EP_IS_MAXIMAL,
        EP_IS_ORIENTED, EP_IS_PM_INTO_LIT, EP_IS_POSITIVE, EP_IS_SELECTED, EP_MAX_IS_UP_TO_DATE,
        EP_PSEUDO_LIT,
    };
    use crate::heuristics::to_params::{LiteralCmp, TermOrdering};
    use crate::inout::scanner::{IoFormat, Scanner};
    use crate::orderings::ocb::OrderControlBlock;
    use crate::terms::signature::SIG_PHONY_APP_CODE;
    use crate::terms::signature::{
        FunctionProperties, Signature, FP_CL_SPLIT_DEF, FP_IS_INTEGER, FP_PSEUDO_PRED,
    };
    use crate::terms::simpletypes::alloc_arrow_type;
    use crate::terms::subst::Substitution;
    use crate::terms::termbanks::TermBank;
    use crate::terms::termfunc::term_standard_weight;
    use crate::terms::termtypes::{
        DerefType, Term, TP_CHECK_FLAG, TP_OP_FLAG, TP_PRED_POS, TP_SPECIAL_FLAG,
    };
    use crate::terms::termweightext::{TermWeightExtension, TermWeightExtensionStyle};
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

    fn kbo_ocb(bank: &TermBank) -> OrderControlBlock {
        OrderControlBlock::alloc(
            TermOrdering::Kbo,
            true,
            bank.signature(),
            HoOrderKind::LfhoOrder,
        )
    }

    fn matrix_kbo_ocb(bank: &TermBank) -> OrderControlBlock {
        OrderControlBlock::alloc(
            TermOrdering::Kbo,
            false,
            bank.signature(),
            HoOrderKind::LfhoOrder,
        )
    }

    #[test]
    fn eqn_parse_uses_lop_mixfix_and_tptp_sign_shapes() {
        let mut bank = test_bank();
        let mut lop_infix = Scanner::from_user_string("parse_a=parse_b", false).unwrap();
        lop_infix.set_format(IoFormat::Lop);
        let equality = eqn_parse(&mut lop_infix, &mut bank, ProblemType::FirstOrder).unwrap();
        assert_eq!(
            eqn_string(&bank, &equality, false, true, EqnPrintOptions::default()),
            "parse_a=parse_b"
        );

        let mut lop_prefix = Scanner::from_user_string("~equal(parse_a,parse_b)", false).unwrap();
        lop_prefix.set_format(IoFormat::Lop);
        let negated_prefix =
            eqn_parse(&mut lop_prefix, &mut bank, ProblemType::FirstOrder).unwrap();
        assert_eq!(
            eqn_string(
                &bank,
                &negated_prefix,
                false,
                true,
                EqnPrintOptions::default()
            ),
            "parse_a!=parse_b"
        );

        let mut predicate = Scanner::from_user_string("~parse_p", false).unwrap();
        predicate.set_format(IoFormat::Lop);
        let predicate = eqn_parse(&mut predicate, &mut bank, ProblemType::FirstOrder).unwrap();
        assert_eq!(
            eqn_string(&bank, &predicate, false, true, EqnPrintOptions::default()),
            "~parse_p"
        );

        let mut tptp = Scanner::from_user_string("--equal(tptp_a,tptp_b)", false).unwrap();
        tptp.set_format(IoFormat::Tptp);
        let tptp = eqn_parse(&mut tptp, &mut bank, ProblemType::FirstOrder).unwrap();
        assert_eq!(
            eqn_string(&bank, &tptp, false, true, EqnPrintOptions::default()),
            "tptp_a!=tptp_b"
        );
    }

    #[test]
    fn eqn_fof_parse_uses_tptp_tilde_negation() {
        let mut bank = test_bank();
        let mut scanner = Scanner::from_user_string("~equal(fof_a,fof_b)", false).unwrap();
        scanner.set_format(IoFormat::Tptp);

        let literal = eqn_fof_parse(&mut scanner, &mut bank, ProblemType::FirstOrder).unwrap();

        assert_eq!(
            eqn_fof_string(&bank, &literal, false, true, EqnFofPrintOptions::tptp()),
            "~equal(fof_a, fof_b)"
        );
    }

    #[test]
    fn eqn_print_string_matches_c_lop_tptp_and_option_shapes() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "print_a");
        let b = typed_const(&mut bank, "print_b");
        let equality = Eqn::alloc(a.clone(), b.clone(), &mut bank, true).unwrap();

        assert_eq!(
            eqn_string(&bank, &equality, false, true, EqnPrintOptions::default()),
            "print_a=print_b"
        );
        assert_eq!(
            eqn_string(&bank, &equality, true, true, EqnPrintOptions::default()),
            "print_a!=print_b"
        );
        assert_eq!(
            eqn_string(&bank, &equality, false, true, EqnPrintOptions::tptp()),
            "++equal(print_a, print_b)"
        );

        let mut oriented = equality.clone();
        oriented.set_prop(EP_IS_ORIENTED);
        let oriented_options = EqnPrintOptions {
            print_oriented: true,
            ..EqnPrintOptions::default()
        };
        assert_eq!(
            eqn_string(&bank, &oriented, false, true, oriented_options),
            "print_a->print_b"
        );

        let atom = typed_pred_const(&mut bank, "print_p");
        let true_term = bank.true_term().clone();
        let predicate = Eqn::alloc(atom, true_term, &mut bank, true).unwrap();
        assert_eq!(
            eqn_string(&bank, &predicate, false, true, EqnPrintOptions::default()),
            "print_p"
        );
        assert_eq!(
            eqn_string(&bank, &predicate, true, true, EqnPrintOptions::default()),
            "~print_p"
        );
        assert_eq!(
            eqn_string(&bank, &predicate, false, true, EqnPrintOptions::tptp()),
            "++print_p"
        );
    }

    #[test]
    fn eqn_fof_print_string_matches_c_format_switches() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "fof_a");
        let b = typed_const(&mut bank, "fof_b");
        let equality = Eqn::alloc(a.clone(), b.clone(), &mut bank, true).unwrap();

        assert_eq!(
            eqn_fof_string(&bank, &equality, false, true, EqnFofPrintOptions::lop()),
            "fof_a=fof_b"
        );
        assert_eq!(
            eqn_fof_string(&bank, &equality, true, true, EqnFofPrintOptions::tstp()),
            "fof_a!=fof_b"
        );

        let pcl_options = EqnFofPrintOptions {
            pcl: true,
            ..EqnFofPrintOptions::lop()
        };
        assert_eq!(
            eqn_fof_string(&bank, &equality, false, true, pcl_options),
            "equal(fof_a, fof_b)"
        );
        assert_eq!(
            eqn_fof_string(&bank, &equality, true, true, EqnFofPrintOptions::tptp()),
            "~equal(fof_a, fof_b)"
        );

        let ho_options = EqnFofPrintOptions {
            higher_order_parentheses: true,
            ..EqnFofPrintOptions::tstp()
        };
        assert_eq!(
            eqn_fof_string(&bank, &equality, false, true, ho_options),
            "((fof_a)=(fof_b))"
        );

        let atom = typed_pred_const(&mut bank, "fof_p");
        let true_term = bank.true_term().clone();
        let predicate = Eqn::alloc(atom, true_term, &mut bank, true).unwrap();
        assert_eq!(
            eqn_fof_string(&bank, &predicate, false, true, EqnFofPrintOptions::lop()),
            "fof_p"
        );
        assert_eq!(
            eqn_fof_string(&bank, &predicate, true, true, EqnFofPrintOptions::lop()),
            "~fof_p"
        );
        assert_eq!(
            eqn_fof_string(&bank, &predicate, true, true, ho_options),
            "~(fof_p)"
        );
    }

    #[test]
    fn eqn_tstp_print_string_matches_c_literal_shapes() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "tstp_a");
        let b = typed_const(&mut bank, "tstp_b");
        let positive = Eqn::alloc(a.clone(), b.clone(), &mut bank, true).unwrap();
        let negative = Eqn::alloc(a.clone(), b.clone(), &mut bank, false).unwrap();

        assert_eq!(
            eqn_tstp_string(&bank, &positive, true, false),
            "tstp_a=tstp_b"
        );
        assert_eq!(
            eqn_tstp_string(&bank, &negative, true, false),
            "tstp_a!=tstp_b"
        );

        let mut oriented_positive = positive.clone();
        oriented_positive.set_prop(EP_IS_ORIENTED);
        let mut oriented_negative = negative.clone();
        oriented_negative.set_prop(EP_IS_ORIENTED);
        assert_eq!(
            eqn_tstp_string(&bank, &oriented_positive, true, true),
            "tstp_a->tstp_b"
        );
        assert_eq!(
            eqn_tstp_string(&bank, &oriented_negative, true, true),
            "tstp_a!->tstp_b"
        );
        assert_eq!(
            eqn_tstp_string(&bank, &oriented_positive, true, false),
            "tstp_a=tstp_b"
        );

        let atom = typed_pred_const(&mut bank, "tstp_p");
        let true_term = bank.true_term().clone();
        let predicate = Eqn::alloc(atom, true_term, &mut bank, false).unwrap();
        assert_eq!(eqn_tstp_string(&bank, &predicate, true, false), "~tstp_p");

        let false_literal = Eqn::alloc(a.clone(), a, &mut bank, false).unwrap();
        assert_eq!(
            eqn_tstp_string(&bank, &false_literal, true, false),
            "$false"
        );
    }

    #[test]
    fn eqn_deref_print_string_matches_c_infix_shape() {
        let mut bank = test_bank();
        let type_ = bank.signature().type_bank().default_type();
        let x = bank.vars().var_assert_alloc(-2, &type_);
        let y = bank.vars().var_assert_alloc(-4, &type_);
        let a = typed_const(&mut bank, "deref_a");
        let b = typed_const(&mut bank, "deref_b");
        x.set_binding(Some(y.clone()));
        y.set_binding(Some(a.clone()));
        let negative = Eqn::alloc(x.clone(), b.clone(), &mut bank, false).unwrap();

        assert_eq!(
            eqn_deref_string(&bank, &negative, DerefType::Never),
            "X1!=deref_b"
        );
        assert_eq!(
            eqn_deref_string(&bank, &negative, DerefType::Once),
            "X2!=deref_b"
        );
        assert_eq!(
            eqn_deref_string(&bank, &negative, DerefType::Always),
            "deref_a!=deref_b"
        );

        let positive = Eqn::alloc(y.clone(), b, &mut bank, true).unwrap();
        assert_eq!(
            eqn_deref_string(&bank, &positive, DerefType::Always),
            "deref_a=deref_b"
        );
    }

    #[test]
    fn eqn_debug_print_string_matches_c_suffix_shape() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "debug_a");
        let b = typed_const(&mut bank, "debug_b");
        let mut equality = Eqn::alloc(a.clone(), b, &mut bank, true).unwrap();
        equality.set_prop(EP_IS_MAXIMAL | EP_IS_ORIENTED);

        assert_eq!(
            eqn_debug_string(&bank, &equality, ProblemType::FirstOrder),
            "debug_a=debug_b*>%%"
        );

        let predicate = typed_pred_const(&mut bank, "debug_p");
        let true_term = bank.true_term().clone();
        let negative = Eqn::alloc(predicate, true_term, &mut bank, false).unwrap();
        assert_eq!(
            eqn_debug_string(&bank, &negative, ProblemType::FirstOrder),
            "debug_p!=$true"
        );

        let unary = typed_unary(&mut bank, "debug_f", &a);
        let ho_equality = Eqn::alloc(unary, a, &mut bank, true).unwrap();
        assert_eq!(
            eqn_debug_string(&bank, &ho_equality, ProblemType::HigherOrder),
            "debug_f debug_a=debug_a%%"
        );
    }

    #[test]
    fn eqn_app_encode_string_prints_temporary_encoded_terms() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "app_a");
        let b = typed_const(&mut bank, "app_b");
        let f_ab = typed_binary(&mut bank, "app_f", &a, &b);
        let declared_f_type = bank
            .signature()
            .get_type(f_ab.f_code())
            .expect("typed function has a type")
            .clone();
        let individual = bank.signature().type_bank().default_type();
        let f_type = bank
            .signature_mut()
            .type_bank_mut()
            .insert_type_shared(declared_f_type);
        let prefix_type =
            bank.signature_mut()
                .type_bank_mut()
                .insert_type_shared(alloc_arrow_type(vec![
                    individual.clone(),
                    individual.clone(),
                ]));
        let inner_app = format!(
            "app_{}_{}_{}",
            f_type.type_uid(),
            individual.type_uid(),
            prefix_type.type_uid()
        );
        let outer_app = format!(
            "app_{}_{}_{}",
            prefix_type.type_uid(),
            individual.type_uid(),
            individual.type_uid()
        );
        let encoded_left = format!("{outer_app}({inner_app}(app_f,app_a),app_b)");
        let equality = Eqn::alloc(f_ab.clone(), a.clone(), &mut bank, true).unwrap();

        assert_eq!(
            eqn_app_encode_string(&mut bank, &equality, false).unwrap(),
            format!("{encoded_left}=app_a")
        );
        assert_eq!(
            eqn_app_encode_string(&mut bank, &equality, true).unwrap(),
            format!("{encoded_left}!=app_a")
        );
        assert_eq!(bank.term_string(&f_ab, true), "app_f(app_a,app_b)");

        let predicate = typed_pred_const(&mut bank, "app_p");
        let true_term = bank.true_term().clone();
        let pred_lit = Eqn::alloc(predicate, true_term, &mut bank, true).unwrap();
        assert_eq!(
            eqn_app_encode_string(&mut bank, &pred_lit, true).unwrap(),
            "~app_p"
        );
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
    fn orient_sets_and_swaps_sides_like_c_eqn_orient() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let f_a = typed_unary(&mut bank, "f", &a);
        let mut greater = Eqn::alloc(f_a.clone(), a.clone(), &mut bank, true).unwrap();
        let mut ocb = kbo_ocb(&bank);

        assert!(!greater.orient(&mut ocb, &bank));
        assert_eq!(greater.left(), &f_a);
        assert_eq!(greater.right(), &a);
        assert!(greater.is_oriented());
        assert!(greater.query_prop(EP_MAX_IS_UP_TO_DATE));

        let mut lesser = Eqn::alloc(a.clone(), f_a.clone(), &mut bank, true).unwrap();
        assert!(lesser.orient(&mut ocb, &bank));
        assert_eq!(lesser.left(), &f_a);
        assert_eq!(lesser.right(), &a);
        assert!(lesser.is_oriented());
        assert!(lesser.query_prop(EP_MAX_IS_UP_TO_DATE));
    }

    #[test]
    fn orient_preserves_c_true_and_equal_special_cases() {
        let mut bank = test_bank();
        let atom = typed_pred_const(&mut bank, "p");
        let mut right_true =
            Eqn::alloc(atom.clone(), bank.true_term().clone(), &mut bank, true).unwrap();
        let mut ocb = kbo_ocb(&bank);

        assert!(!right_true.orient(&mut ocb, &bank));
        assert_eq!(right_true.left(), &atom);
        assert_eq!(right_true.right(), bank.true_term());
        assert!(right_true.is_oriented());
        assert!(right_true.query_prop(EP_MAX_IS_UP_TO_DATE));

        let mut left_true =
            Eqn::alloc(atom.clone(), bank.true_term().clone(), &mut bank, true).unwrap();
        left_true.set_left_raw(bank.true_term().clone());
        left_true.set_right_raw(atom.clone());
        assert!(left_true.orient(&mut ocb, &bank));
        assert_eq!(left_true.left(), &atom);
        assert_eq!(left_true.right(), bank.true_term());
        assert!(left_true.is_oriented());

        let a = typed_const(&mut bank, "a");
        let mut equal = Eqn::alloc(a.clone(), a, &mut bank, true).unwrap();
        equal.set_prop(EP_IS_ORIENTED);
        assert!(!equal.orient(&mut ocb, &bank));
        assert!(!equal.is_oriented());
        assert!(equal.query_prop(EP_MAX_IS_UP_TO_DATE));
    }

    #[test]
    fn orient_respects_max_up_to_date_cache_flag() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let f_a = typed_unary(&mut bank, "f", &a);
        let mut eq = Eqn::alloc(a.clone(), f_a.clone(), &mut bank, true).unwrap();
        let mut ocb = kbo_ocb(&bank);

        eq.set_prop(EP_MAX_IS_UP_TO_DATE);

        assert!(!eq.orient(&mut ocb, &bank));
        assert_eq!(eq.left(), &a);
        assert_eq!(eq.right(), &f_a);
        assert!(!eq.is_oriented());
        assert!(eq.query_prop(EP_MAX_IS_UP_TO_DATE));
    }

    #[test]
    fn order_compare_matches_c_equation_multiset_cases() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let f_a = typed_unary(&mut bank, "f", &a);
        let greater = Eqn::alloc(f_a.clone(), a.clone(), &mut bank, true).unwrap();
        let mut smaller = Eqn::alloc(a.clone(), a.clone(), &mut bank, true).unwrap();
        let mut equal = Eqn::alloc(f_a.clone(), a, &mut bank, true).unwrap();
        let mut ocb = kbo_ocb(&bank);

        assert_eq!(
            greater.order_compare(&mut ocb, &bank, &smaller),
            CompareResult::Greater
        );
        assert!(greater.order_greater(&mut ocb, &bank, &smaller));
        assert_eq!(
            smaller.order_compare(&mut ocb, &bank, &greater),
            CompareResult::Lesser
        );
        assert_eq!(
            greater.order_compare(&mut ocb, &bank, &equal),
            CompareResult::Equal
        );

        smaller.flip_prop(EP_IS_POSITIVE);
        equal.flip_prop(EP_IS_POSITIVE);
        assert_eq!(
            smaller.order_compare(&mut ocb, &bank, &equal),
            CompareResult::Lesser
        );
    }

    #[test]
    fn literal_compare_honors_pseudo_selected_and_no_cmp_priority() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let f_a = typed_unary(&mut bank, "f", &a);
        let normal = Eqn::alloc(f_a.clone(), a.clone(), &mut bank, true).unwrap();
        let mut pseudo = Eqn::alloc(f_a.clone(), a.clone(), &mut bank, true).unwrap();
        let mut selected = Eqn::alloc(f_a.clone(), a.clone(), &mut bank, true).unwrap();
        let mut selected_negative = Eqn::alloc(f_a.clone(), a.clone(), &mut bank, false).unwrap();
        let mut ocb = kbo_ocb(&bank);

        pseudo.set_prop(EP_PSEUDO_LIT);
        assert_eq!(
            pseudo.literal_compare(&mut ocb, &bank, &normal),
            CompareResult::Lesser
        );
        assert_eq!(
            normal.literal_compare(&mut ocb, &bank, &pseudo),
            CompareResult::Greater
        );

        selected.set_prop(EP_IS_SELECTED);
        assert_eq!(
            normal.literal_compare(&mut ocb, &bank, &selected),
            CompareResult::Lesser
        );
        assert!(selected.literal_greater(&mut ocb, &bank, &normal));

        selected_negative.set_prop(EP_IS_SELECTED);
        assert_eq!(
            selected.literal_compare(&mut ocb, &bank, &selected_negative),
            CompareResult::Uncomparable
        );

        ocb.lit_cmp = LiteralCmp::NoCmp;
        assert_eq!(
            normal.literal_compare(&mut ocb, &bank, &selected),
            CompareResult::Lesser
        );
        let other = Eqn::alloc(a.clone(), a, &mut bank, true).unwrap();
        assert_eq!(
            normal.literal_compare(&mut ocb, &bank, &other),
            CompareResult::Uncomparable
        );
    }

    #[test]
    fn literal_compare_ports_tfo_modes_and_signed_equation_cases() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let f_a = typed_unary(&mut bank, "f", &a);
        let equality = Eqn::alloc(a.clone(), b.clone(), &mut bank, true).unwrap();
        let predicate = typed_pred_const(&mut bank, "p");
        let other_predicate = typed_pred_const(&mut bank, "q");
        let p_lit =
            Eqn::alloc(predicate.clone(), bank.true_term().clone(), &mut bank, true).unwrap();
        let q_lit = Eqn::alloc(
            other_predicate.clone(),
            bank.true_term().clone(),
            &mut bank,
            true,
        )
        .unwrap();

        let mut eq_max = kbo_ocb(&bank);
        eq_max.lit_cmp = LiteralCmp::TfoEqMax;
        assert_eq!(
            equality.literal_compare(&mut eq_max, &bank, &p_lit),
            CompareResult::Greater
        );

        let mut eq_min = kbo_ocb(&bank);
        eq_min.lit_cmp = LiteralCmp::TfoEqMin;
        assert_eq!(
            equality.literal_compare(&mut eq_min, &bank, &p_lit),
            CompareResult::Lesser
        );

        let mut pred_prec = matrix_kbo_ocb(&bank);
        pred_prec.lit_cmp = LiteralCmp::TfoEqMax;
        pred_prec.precedence_add_tuple(
            bank.signature(),
            predicate.f_code(),
            other_predicate.f_code(),
            CompareResult::Greater,
        );
        assert_eq!(
            p_lit.literal_compare(&mut pred_prec, &bank, &q_lit),
            CompareResult::Greater
        );

        let mut positive = Eqn::alloc(f_a, a.clone(), &mut bank, true).unwrap();
        positive.set_prop(EP_IS_ORIENTED);
        let negative = Eqn::alloc(a, b, &mut bank, false).unwrap();
        let mut normal = kbo_ocb(&bank);
        assert_eq!(
            positive.literal_compare(&mut normal, &bank, &negative),
            CompareResult::Greater
        );
        assert_eq!(
            negative.literal_compare(&mut normal, &bank, &positive),
            CompareResult::Lesser
        );
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
    fn copy_opt_preserves_recomputed_boolean_literal_shape() {
        let mut bank = test_bank();
        let bool_type = bank.signature().type_bank().bool_type();
        let x = bank.vars().var_assert_alloc(-2, &bool_type);
        let y = bank.vars().var_assert_alloc(-4, &bool_type);
        let eq = Eqn::alloc(y.clone(), x.clone(), &mut bank, true).unwrap();
        assert!(eq.query_prop(EP_IS_EQU_LITERAL));

        x.set_binding(Some(bank.false_term().clone()));
        let opt = eq.copy_opt(&mut bank).unwrap();
        x.set_binding(None);

        assert!(!opt.query_prop(EP_IS_EQU_LITERAL));
        assert!(!opt.is_equ_lit(&bank));
        assert!(opt.is_negative());
        assert_eq!(opt.left(), &y);
        assert_eq!(opt.right(), bank.true_term());
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
    fn term_extension_weight_helpers_apply_term_literal_and_polarity_multipliers() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let f_of_a = typed_unary(&mut bank, "f", &a);
        let mut lit = Eqn::alloc(f_of_a, b, &mut bank, true).unwrap();
        let simple = TermWeightExtension::new(
            2.0,
            3.0,
            4.0,
            TermWeightExtensionStyle::Simple,
            |term: &Term, base: &f64| base + if term.arity() == 0 { 0.0 } else { 1.0 },
            10.0,
        );

        assert_f64_bits_eq(lit.term_ext_weight(&simple), 42.0);
        lit.set_prop(EP_IS_MAXIMAL);
        assert_f64_bits_eq(lit.literal_term_ext_weight(&simple), 504.0);

        lit.set_prop(EP_IS_ORIENTED);
        assert_f64_bits_eq(lit.term_ext_weight(&simple), 32.0);
        assert_f64_bits_eq(lit.literal_term_ext_weight(&simple), 384.0);
    }

    #[test]
    fn term_extension_weight_uses_real_term_subterm_traversal() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let f_of_a = typed_unary(&mut bank, "f", &a);
        let lit = Eqn::alloc(f_of_a, b, &mut bank, false).unwrap();
        let subterms = TermWeightExtension::new(
            2.0,
            3.0,
            4.0,
            TermWeightExtensionStyle::SubtermsSum,
            |_term: &Term, _data: &()| 1.0,
            (),
        );

        assert_f64_bits_eq(lit.term_ext_weight(&subterms), 6.0);
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
    fn substitution_normalization_binds_variables_on_both_equation_sides() {
        let mut bank = test_bank();
        let type_ = bank.signature().type_bank().default_type();
        let left_var = bank.vars().var_assert_alloc(-2, &type_);
        let right_var = bank.vars().var_assert_alloc(-4, &type_);
        let existing_var = bank.vars().var_assert_alloc(-6, &type_);
        let existing_binding = typed_const(&mut bank, "a");
        let right_const = typed_const(&mut bank, "b");
        let left = typed_unary(&mut bank, "f", &left_var);
        let right = typed_binary(&mut bank, "g", &right_var, &right_const);
        let eq = Eqn::alloc(left, right, &mut bank, true).unwrap();
        let mut subst = Substitution::new();
        subst.add_binding(&existing_var, &existing_binding);

        let backtrack = eq.subst_norm(&mut subst, bank.vars());

        assert_eq!(backtrack, 1);
        assert_eq!(subst.len(), 3);
        assert!(existing_var.binding().is_some());
        let left_binding = left_var.binding().unwrap();
        let right_binding = right_var.binding().unwrap();
        assert!(left_binding.is_free_var());
        assert!(right_binding.is_free_var());
        assert_ne!(left_binding, right_binding);
        assert!(left_binding.query_prop(TP_SPECIAL_FLAG));
        assert!(right_binding.query_prop(TP_SPECIAL_FLAG));

        subst.backtrack_to_pos(backtrack);
        assert!(existing_var.binding().is_some());
        assert!(left_var.binding().is_none());
        assert!(right_var.binding().is_none());
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
