use crate::basics::pstacks::PStack;
use crate::clauses::clause_props::{
    FormulaProperties, CP_IGNORE_PROPS, CP_IS_LAMBDA_DEF, CP_TYPE_CONJECTURE, CP_TYPE_QUESTION,
};
use crate::clauses::clausefunc::{tformula_is_literal, tformula_mark_polarity};
use crate::clauses::clauseinfo::ClauseInfo;
use crate::terms::functypes::FunCode;
use crate::terms::signature::Signature;
use crate::terms::simpletypes::type_is_predicate;
use crate::terms::termbanks::tb_term_collect_subterms;
use crate::terms::termbanks::TermBank;
use crate::terms::termfunc::{
    term_compute_order, term_has_f_code, term_is_untyped, term_standard_weight,
};
use crate::terms::termtypes::{term_has_interpreted_symbol, Term, TP_OP_FLAG};
use std::collections::BTreeSet;
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};

static WRAPPED_FORMULA_ENTRY_ID: AtomicU64 = AtomicU64::new(1);
static FORMULA_IDENT_COUNTER: AtomicI64 = AtomicI64::new(i64::MIN);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WrappedFormula {
    entry_id: u64,
    properties: FormulaProperties,
    is_clause: bool,
    ident: i64,
    info: Option<ClauseInfo>,
    formula: Option<Term>,
}

impl WrappedFormula {
    #[must_use]
    pub fn default_alloc() -> Self {
        Self {
            entry_id: next_entry_id(),
            properties: CP_IGNORE_PROPS,
            is_clause: false,
            ident: 0,
            info: None,
            formula: None,
        }
    }

    #[must_use]
    pub fn wt_formula_alloc(formula: Term) -> Self {
        Self {
            formula: Some(formula),
            ident: next_formula_ident(),
            ..Self::default_alloc()
        }
    }

    #[must_use]
    pub fn flat_copy(&self) -> Self {
        Self {
            entry_id: next_entry_id(),
            properties: self.properties,
            is_clause: self.is_clause,
            ident: self.ident,
            info: None,
            formula: self.formula.clone(),
        }
    }

    #[must_use]
    pub const fn entry_id(&self) -> u64 {
        self.entry_id
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
    pub const fn query_prop(&self, prop: FormulaProperties) -> bool {
        self.properties.query(prop)
    }

    #[must_use]
    pub const fn give_props(&self, prop: FormulaProperties) -> FormulaProperties {
        self.properties.give(prop)
    }

    pub fn set_tptp_type(&mut self, type_: FormulaProperties) {
        self.properties.set_tptp_type(type_);
    }

    #[must_use]
    pub const fn query_tptp_type(&self) -> FormulaProperties {
        self.properties.query_tptp_type()
    }

    #[must_use]
    pub const fn is_clause(&self) -> bool {
        self.is_clause
    }

    pub fn set_is_clause(&mut self, is_clause: bool) {
        self.is_clause = is_clause;
    }

    #[must_use]
    pub const fn ident(&self) -> i64 {
        self.ident
    }

    #[must_use]
    pub const fn info(&self) -> Option<&ClauseInfo> {
        self.info.as_ref()
    }

    pub fn set_info(&mut self, info: Option<ClauseInfo>) {
        self.info = info;
    }

    /// Returns the wrapped formula term.
    ///
    /// # Panics
    ///
    /// Panics if this wrapper was allocated without a formula term.
    #[must_use]
    pub fn formula(&self) -> &Term {
        self.formula
            .as_ref()
            .unwrap_or_else(|| panic!("wrapped formula has no term formula"))
    }

    pub fn set_formula(&mut self, formula: Term) {
        self.formula = Some(formula);
    }

    pub fn gc_mark_cells(&self, bank: &TermBank) {
        if let Some(formula) = &self.formula {
            bank.gc_mark_term(formula);
        }
    }

    pub fn mark_polarity(&self, bank: &TermBank) {
        if let Some(formula) = &self.formula {
            tformula_mark_polarity(bank, formula, 1);
        }
    }

    #[must_use]
    pub fn get_id(&self, keep_input_names: bool) -> String {
        if keep_input_names {
            if let Some(name) = self.info.as_ref().and_then(ClauseInfo::name) {
                return name.to_owned();
            }
        }
        if self.ident < 0 {
            format!("i_0_{}", i128::from(self.ident) - i128::from(i64::MIN))
        } else {
            format!("c_0_{}", self.ident)
        }
    }

    #[must_use]
    pub fn is_untyped(&self) -> bool {
        term_is_untyped(self.formula())
    }

    #[must_use]
    pub fn has_interpreted_symbol(&self) -> bool {
        term_has_interpreted_symbol(self.formula())
    }

    #[must_use]
    pub fn standard_weight(&self) -> i64 {
        term_standard_weight(self.formula())
    }

    #[must_use]
    pub fn is_hypothesis(&self) -> bool {
        self.properties.is_hypothesis()
    }

    #[must_use]
    pub fn is_conjecture(&self) -> bool {
        matches!(
            self.query_tptp_type(),
            CP_TYPE_CONJECTURE | CP_TYPE_QUESTION
        ) || self.properties.is_conjecture()
    }

    #[must_use]
    pub fn conjecture_order(&self, signature: &Signature) -> usize {
        term_compute_order(signature, self.formula())
    }

    #[must_use]
    pub fn contains_f_code(&self, f_code: FunCode) -> bool {
        term_has_f_code(self.formula(), f_code)
    }

    /// Pushes each distinct non-variable, non-phony-app f-code in traversal order.
    ///
    /// # Panics
    ///
    /// Panics if this wrapper has no formula term or if the formula is not
    /// shared by its term bank.
    pub fn return_f_codes(&self, f_codes: &mut Vec<FunCode>) -> i64 {
        let start = f_codes.len();
        let mut subterms = PStack::new();
        let _ = tb_term_collect_subterms(self.formula(), &mut subterms);
        let mut seen = BTreeSet::new();
        for term in subterms.as_slice() {
            term.del_prop(TP_OP_FLAG);
            if !term.is_any_var() && !term.is_phony_app() && seen.insert(term.f_code()) {
                f_codes.push(term.f_code());
            }
        }
        usize_to_i64(f_codes.len() - start)
    }

    /// Returns the count of distinct non-variable, non-phony-app f-codes.
    ///
    /// # Panics
    ///
    /// Panics under the same conditions as [`Self::return_f_codes`].
    #[must_use]
    pub fn symbol_diversity(&self) -> i64 {
        let mut f_codes = Vec::new();
        self.return_f_codes(&mut f_codes)
    }

    /// Extracts the defined symbol from C's lambda-definition formula shapes.
    ///
    /// # Panics
    ///
    /// Panics if this wrapper is tagged as a lambda definition but has no
    /// formula term.
    #[must_use]
    pub fn get_lambda_defined_symbol(&self, signature: &Signature) -> Option<FunCode> {
        if !self.query_prop(CP_IS_LAMBDA_DEF) {
            return None;
        }

        let mut formula = self.formula().clone();
        while formula.f_code() == signature.qall_code() && formula.arity() == 2 {
            formula = formula.argument(1)?;
        }

        let left = if formula.f_code() == signature.eqn_code() {
            formula.argument(0)
        } else if formula.f_code() == signature.equiv_code() {
            let equivalence_left = formula.argument(0)?;
            if equivalence_left.f_code() == signature.eqn_code() {
                equivalence_left.argument(0)
            } else {
                None
            }
        } else {
            None
        }?;

        (left.f_code() > signature.internal_symbols()).then_some(left.f_code())
    }

    /// Counts lambda cells that occur below a non-logical/non-lambda formula node.
    ///
    /// # Panics
    ///
    /// Panics if this wrapper has no formula term or if a malformed lambda or
    /// phony-application cell has uninitialized arguments.
    #[must_use]
    pub fn count_non_top_level_lambdas(&self, signature: &Signature) -> i32 {
        tformula_count_non_top_level_lambdas(signature, self.formula())
    }

    /// Returns whether any literal side is an applied free variable.
    ///
    /// # Panics
    ///
    /// Panics if this wrapper has no formula term.
    #[must_use]
    pub fn has_app_var_literal(&self, bank: &TermBank) -> bool {
        tformula_has_app_var_literal(bank, self.formula())
    }

    fn lambda_definition_head_for_statistics(&self, bank: &TermBank) -> Option<Term> {
        if !self.query_prop(CP_IS_LAMBDA_DEF) {
            return None;
        }

        let signature = bank.signature();
        let mut formula = self.formula().clone();
        while formula.f_code() == signature.qall_code() && formula.arity() == 2 {
            formula = formula.argument(1)?;
        }

        if formula.f_code() == signature.eqn_code() {
            return formula.argument(0);
        }
        if formula.f_code() != signature.equiv_code() {
            return None;
        }

        let equivalence_left = formula.argument(0)?;
        if equivalence_left.f_code() != signature.eqn_code()
            || equivalence_left.argument(1).as_ref() != Some(bank.true_term())
        {
            return None;
        }
        equivalence_left.argument(0)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FormulaDefinitionStatistics {
    pub num_defs: i32,
    pub percentage_form_defs: f64,
    pub num_lams: i32,
    pub has_app_var_lits: bool,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct FormulaSet {
    formulas: Vec<WrappedFormula>,
    identifier: String,
}

impl FormulaSet {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn cardinality(&self) -> i64 {
        usize_to_i64(self.formulas.len())
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.formulas.is_empty()
    }

    #[must_use]
    pub fn identifier(&self) -> &str {
        &self.identifier
    }

    pub fn set_identifier(&mut self, identifier: impl Into<String>) {
        self.identifier = identifier.into();
    }

    pub fn iter(&self) -> impl Iterator<Item = &WrappedFormula> {
        self.formulas.iter()
    }

    pub fn insert(&mut self, formula: WrappedFormula) -> u64 {
        let entry_id = formula.entry_id();
        self.formulas.push(formula);
        entry_id
    }

    pub fn insert_set(&mut self, from: &mut Self) -> i64 {
        let moved = from.cardinality();
        self.formulas.append(&mut from.formulas);
        moved
    }

    pub fn extract_first(&mut self) -> Option<WrappedFormula> {
        if self.formulas.is_empty() {
            None
        } else {
            Some(self.formulas.remove(0))
        }
    }

    pub fn extract_entry(&mut self, entry_id: u64) -> Option<WrappedFormula> {
        let position = self
            .formulas
            .iter()
            .position(|formula| formula.entry_id() == entry_id)?;
        Some(self.formulas.remove(position))
    }

    #[must_use]
    pub fn get(&self, entry_id: u64) -> Option<&WrappedFormula> {
        self.formulas
            .iter()
            .find(|formula| formula.entry_id() == entry_id)
    }

    pub fn delete_entry(&mut self, entry_id: u64) -> bool {
        self.extract_entry(entry_id).is_some()
    }

    pub fn move_formula_from(&mut self, from: &mut Self, entry_id: u64) -> bool {
        let Some(formula) = from.extract_entry(entry_id) else {
            return false;
        };
        self.insert(formula);
        true
    }

    #[must_use]
    pub fn is_untyped(&self) -> bool {
        self.formulas.iter().all(WrappedFormula::is_untyped)
    }

    #[must_use]
    pub fn has_interpreted_symbol(&self) -> bool {
        self.formulas
            .iter()
            .any(WrappedFormula::has_interpreted_symbol)
    }

    #[must_use]
    pub fn split_conjectures(&self) -> (i64, Vec<&WrappedFormula>, Vec<&WrappedFormula>) {
        let mut conjectures = Vec::new();
        let mut rest = Vec::new();
        for formula in &self.formulas {
            if formula.is_conjecture() {
                conjectures.push(formula);
            } else {
                rest.push(formula);
            }
        }
        (usize_to_i64(conjectures.len()), conjectures, rest)
    }

    #[must_use]
    pub fn standard_weight(&self) -> i64 {
        self.formulas
            .iter()
            .filter(|formula| !formula.query_prop(CP_IS_LAMBDA_DEF))
            .map(WrappedFormula::standard_weight)
            .sum()
    }

    pub fn count_conjectures(&self, hypotheses: &mut i64) -> i64 {
        let mut conjectures = 0;
        for formula in &self.formulas {
            if formula.is_conjecture() {
                conjectures += 1;
            }
            if formula.is_hypothesis() {
                *hypotheses += 1;
            }
        }
        conjectures
    }

    #[must_use]
    pub fn conjecture_order(&self, signature: &Signature) -> usize {
        self.formulas
            .iter()
            .filter(|formula| formula.is_conjecture() || formula.is_hypothesis())
            .map(|formula| formula.conjecture_order(signature))
            .max()
            .unwrap_or(0)
    }

    pub fn collect_f_code(&self, f_code: FunCode, result: &mut Vec<u64>) -> i64 {
        let start = result.len();
        result.extend(
            self.formulas
                .iter()
                .filter(|formula| formula.contains_f_code(f_code))
                .map(WrappedFormula::entry_id),
        );
        usize_to_i64(result.len() - start)
    }

    pub fn gc_mark_cells(&self, bank: &TermBank) {
        for formula in &self.formulas {
            formula.gc_mark_cells(bank);
        }
    }

    pub fn mark_polarity(&self, bank: &TermBank) {
        for formula in &self.formulas {
            formula.mark_polarity(bank);
        }
    }
}

#[must_use]
pub fn formula_set_stack_cardinality(stack: &[&FormulaSet]) -> i64 {
    stack.iter().map(|set| set.cardinality()).sum()
}

pub fn formula_stack_cond_set_type(stack: &mut [WrappedFormula], type_: FormulaProperties) {
    for formula in stack {
        if formula.query_tptp_type() != CP_TYPE_CONJECTURE || type_ == CP_TYPE_CONJECTURE {
            formula.set_tptp_type(type_);
        }
    }
}

#[must_use]
pub fn formula_set_definition_statistics(
    orig: &FormulaSet,
    arch: &FormulaSet,
    bank: &TermBank,
) -> FormulaDefinitionStatistics {
    let mut num_defs = 0_i32;
    let mut form_defs = 0_i32;
    let mut num_lams = 0_i32;
    let mut has_app_var_lits = false;

    for set in [orig, arch] {
        for formula in set.iter() {
            has_app_var_lits |= formula.has_app_var_literal(bank);
            num_lams =
                num_lams.saturating_add(formula.count_non_top_level_lambdas(bank.signature()));

            if let Some(head) = formula.lambda_definition_head_for_statistics(bank) {
                num_defs = num_defs.saturating_add(1);
                if head.type_().as_ref().is_some_and(type_is_predicate) {
                    form_defs = form_defs.saturating_add(1);
                }
            }
        }
    }

    FormulaDefinitionStatistics {
        num_defs,
        percentage_form_defs: if num_defs == 0 {
            0.0
        } else {
            f64::from(form_defs) / f64::from(num_defs)
        },
        num_lams,
        has_app_var_lits,
    }
}

fn next_entry_id() -> u64 {
    WRAPPED_FORMULA_ENTRY_ID.fetch_add(1, Ordering::Relaxed)
}

fn next_formula_ident() -> i64 {
    loop {
        let current = FORMULA_IDENT_COUNTER.load(Ordering::Relaxed);
        let next = current
            .checked_add(1)
            .unwrap_or_else(|| panic!("formula ident counter overflow"));
        if FORMULA_IDENT_COUNTER
            .compare_exchange(current, next, Ordering::Relaxed, Ordering::Relaxed)
            .is_ok()
        {
            return next;
        }
    }
}

fn usize_to_i64(value: usize) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

fn tformula_count_non_top_level_lambdas(signature: &Signature, formula: &Term) -> i32 {
    let mut stack = vec![(formula.clone(), true)];
    let mut result = 0_i32;

    while let Some((formula, mut is_at_top)) = stack.pop() {
        if !formula.has_lambda_subterm() {
            continue;
        }

        if is_at_top {
            is_at_top = !formula.is_free_var()
                && ((formula.f_code() > 0 && signature.is_logical_symbol(formula.f_code()))
                    || formula.is_lambda());
        } else if formula.is_lambda() {
            result = result.saturating_add(1);
        }

        let start = usize::from(formula.is_phony_app() || formula.is_lambda());
        for index in start..formula.arity() {
            let child = formula
                .argument(index)
                .unwrap_or_else(|| panic!("formula argument {index} is uninitialized"));
            if child.has_lambda_subterm() {
                stack.push((child, is_at_top));
            }
        }
    }

    result
}

fn tformula_has_app_var_literal(bank: &TermBank, formula: &Term) -> bool {
    let mut stack = vec![formula.clone()];

    while let Some(formula) = stack.pop() {
        if tformula_is_literal(bank, &formula) {
            let left_has_app_var = formula
                .argument(0)
                .is_some_and(|term| term.is_applied_free_var());
            let right_has_app_var = formula
                .argument(1)
                .is_some_and(|term| term.is_applied_free_var());
            if left_has_app_var || right_has_app_var {
                return true;
            }
        } else if formula.f_code() > 0 && bank.signature().is_logical_symbol(formula.f_code()) {
            stack.extend(formula.argument_clones().into_iter().flatten());
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::{
        formula_set_definition_statistics, formula_set_stack_cardinality,
        formula_stack_cond_set_type, FormulaDefinitionStatistics, FormulaSet, WrappedFormula,
    };
    use crate::clauses::clause_props::{
        CP_IGNORE_PROPS, CP_INPUT_FORMULA, CP_IS_LAMBDA_DEF, CP_TYPE_AXIOM, CP_TYPE_CONJECTURE,
        CP_TYPE_HYPOTHESIS, CP_TYPE_NEG_CONJECTURE, CP_TYPE_QUESTION,
    };
    use crate::clauses::clausefunc::tformula_decode_polarity;
    use crate::clauses::clauseinfo::ClauseInfo;
    use crate::terms::lambda::close_with_db_var;
    use crate::terms::signature::{Signature, SIG_DB_LAMBDA_CODE, SIG_PHONY_APP_CODE};
    use crate::terms::simpletypes::{alloc_arrow_type, alloc_simple_sort, ST_INTEGER};
    use crate::terms::termbanks::TermBank;
    use crate::terms::termfunc::term_standard_weight;
    use crate::terms::termtypes::{DerefType, Term};
    use crate::terms::typebanks::TypeBank;

    fn test_bank() -> TermBank {
        let mut signature = Signature::new(TypeBank::new());
        signature.insert_internal_codes().unwrap();
        TermBank::new(signature).unwrap()
    }

    fn typed_const(bank: &mut TermBank, name: &str) -> Term {
        let type_ = bank.signature().type_bank().default_type();
        let f_code = bank.signature_mut().insert_id(name, 0, false);
        bank.signature_mut()
            .declare_final_type(f_code, type_)
            .unwrap();
        bank.create_const_term(f_code).unwrap()
    }

    fn typed_var(bank: &TermBank, code: i64) -> Term {
        let type_ = bank.signature().type_bank().default_type();
        bank.vars().var_assert_alloc(code, &type_)
    }

    fn typed_unary(bank: &mut TermBank, name: &str, arg: &Term) -> Term {
        let type_ = bank.signature().type_bank().default_type();
        typed_unary_with_types(bank, name, arg, &type_, &type_)
    }

    fn typed_unary_with_types(
        bank: &mut TermBank,
        name: &str,
        arg: &Term,
        arg_type: &crate::terms::simpletypes::Type,
        ret_type: &crate::terms::simpletypes::Type,
    ) -> Term {
        let arrow = bank
            .signature_mut()
            .type_bank_mut()
            .insert_type_shared(alloc_arrow_type(vec![arg_type.clone(), ret_type.clone()]));
        let f_code = bank.signature_mut().insert_id(name, 1, false);
        bank.signature_mut()
            .declare_final_type(f_code, arrow)
            .unwrap();
        let term = Term::top_alloc(f_code, 1);
        term.set_type(Some(ret_type.clone()));
        term.set_argument(0, arg.clone());
        bank.insert(&term, DerefType::Never).unwrap()
    }

    fn typed_predicate_const(bank: &mut TermBank, name: &str) -> Term {
        let type_ = bank.signature().type_bank().bool_type();
        let f_code = bank.signature_mut().insert_id(name, 0, false);
        bank.signature_mut()
            .declare_final_type(f_code, type_)
            .unwrap();
        bank.signature_mut().declare_is_predicate(f_code).unwrap();
        bank.create_const_term(f_code).unwrap()
    }

    fn bool_binary_with_code(bank: &mut TermBank, f_code: i64, left: &Term, right: &Term) -> Term {
        let type_ = bank.signature().type_bank().bool_type();
        let term = Term::top_alloc(f_code, 2);
        term.set_type(Some(type_));
        term.set_argument(0, left.clone());
        term.set_argument(1, right.clone());
        bank.term_top_insert(term).unwrap()
    }

    fn quantified_with_code(
        bank: &mut TermBank,
        f_code: i64,
        variable: &Term,
        body: &Term,
    ) -> Term {
        let type_ = bank.signature().type_bank().bool_type();
        let term = Term::top_alloc(f_code, 2);
        term.set_type(Some(type_));
        term.set_argument(0, variable.clone());
        term.set_argument(1, body.clone());
        bank.term_top_insert(term).unwrap()
    }

    fn phony_app(bank: &mut TermBank, head: &Term, arg: &Term) -> Term {
        let type_ = bank.signature().type_bank().default_type();
        let term = Term::top_alloc(SIG_PHONY_APP_CODE, 2);
        term.set_type(Some(type_));
        term.set_argument(0, head.clone());
        term.set_argument(1, arg.clone());
        bank.term_top_insert(term).unwrap()
    }

    #[test]
    fn wrapped_formula_allocation_defaults_and_ids_match_c_shape() {
        let mut bank = test_bank();
        let atom = typed_const(&mut bank, "wf_atom");
        let default = WrappedFormula::default_alloc();

        assert_eq!(default.properties(), CP_IGNORE_PROPS);
        assert!(!default.is_clause());
        assert_eq!(default.ident(), 0);
        assert_eq!(default.info(), None);
        assert_eq!(default.get_id(false), "c_0_0");

        let mut formula = WrappedFormula::wt_formula_alloc(atom);
        assert!(formula.get_id(false).starts_with("i_0_"));
        formula.set_info(Some(ClauseInfo::new(
            Some("input_formula_name"),
            Some("problem.p"),
            7,
            3,
        )));
        assert_eq!(formula.get_id(true), "input_formula_name");
        assert!(formula.get_id(false).starts_with("i_0_"));
    }

    #[test]
    fn wrapped_formula_flat_copy_preserves_formula_state_but_not_source_info() {
        let mut bank = test_bank();
        let atom = typed_const(&mut bank, "wf_copy_atom");
        let mut formula = WrappedFormula::wt_formula_alloc(atom);
        formula.set_tptp_type(CP_TYPE_CONJECTURE);
        formula.set_prop(CP_INPUT_FORMULA);
        formula.set_is_clause(true);
        formula.set_info(Some(ClauseInfo::new(Some("copy_source"), None, 1, 1)));

        let copied = formula.flat_copy();

        assert_ne!(copied.entry_id(), formula.entry_id());
        assert_eq!(copied.ident(), formula.ident());
        assert_eq!(copied.properties(), formula.properties());
        assert!(copied.is_clause());
        assert_eq!(copied.info(), None);
        assert_eq!(copied.formula(), formula.formula());
    }

    #[test]
    fn formula_set_insert_extract_and_move_preserve_c_list_order() {
        let mut bank = test_bank();
        let first = WrappedFormula::wt_formula_alloc(typed_const(&mut bank, "set_first"));
        let second = WrappedFormula::wt_formula_alloc(typed_const(&mut bank, "set_second"));
        let third = WrappedFormula::wt_formula_alloc(typed_const(&mut bank, "set_third"));
        let first_id = first.entry_id();
        let second_id = second.entry_id();
        let third_id = third.entry_id();
        let mut set = FormulaSet::new();

        assert!(set.is_empty());
        assert_eq!(set.insert(first), first_id);
        assert_eq!(set.insert(second), second_id);
        assert_eq!(set.cardinality(), 2);
        assert_eq!(
            set.iter().map(WrappedFormula::entry_id).collect::<Vec<_>>(),
            vec![first_id, second_id]
        );

        assert_eq!(set.extract_first().unwrap().entry_id(), first_id);
        assert_eq!(set.cardinality(), 1);
        assert_eq!(set.extract_entry(second_id).unwrap().entry_id(), second_id);
        assert!(set.extract_first().is_none());

        let mut from = FormulaSet::new();
        from.insert(WrappedFormula::wt_formula_alloc(typed_const(
            &mut bank,
            "move_first",
        )));
        from.insert(third);
        let moved_first_id = from.iter().next().unwrap().entry_id();
        let mut to = FormulaSet::new();

        assert!(to.move_formula_from(&mut from, third_id));
        assert_eq!(from.cardinality(), 1);
        assert_eq!(to.cardinality(), 1);
        assert_eq!(to.iter().next().unwrap().entry_id(), third_id);

        assert_eq!(to.insert_set(&mut from), 1);
        assert!(from.is_empty());
        assert_eq!(
            to.iter().map(WrappedFormula::entry_id).collect::<Vec<_>>(),
            vec![third_id, moved_first_id]
        );
        assert_eq!(formula_set_stack_cardinality(&[&to, &from]), 2);
    }

    #[test]
    fn formula_set_type_queries_and_counts_match_c_macros() {
        let mut bank = test_bank();
        let mut axiom = WrappedFormula::wt_formula_alloc(typed_const(&mut bank, "count_axiom"));
        axiom.set_tptp_type(CP_TYPE_AXIOM);
        let mut hypothesis = WrappedFormula::wt_formula_alloc(typed_const(&mut bank, "count_hyp"));
        hypothesis.set_tptp_type(CP_TYPE_HYPOTHESIS);
        let mut conjecture = WrappedFormula::wt_formula_alloc(typed_const(&mut bank, "count_conj"));
        conjecture.set_tptp_type(CP_TYPE_CONJECTURE);
        let mut question =
            WrappedFormula::wt_formula_alloc(typed_const(&mut bank, "count_question"));
        question.set_tptp_type(CP_TYPE_QUESTION);
        let mut set = FormulaSet::new();
        set.insert(axiom);
        set.insert(hypothesis);
        set.insert(conjecture);
        set.insert(question);

        let (count, conjectures, rest) = set.split_conjectures();
        assert_eq!(count, 2);
        assert_eq!(conjectures.len(), 2);
        assert_eq!(rest.len(), 2);

        let mut hypothesis_count = 5;
        assert_eq!(set.count_conjectures(&mut hypothesis_count), 2);
        assert_eq!(hypothesis_count, 6);

        let mut stack = set.iter().cloned().collect::<Vec<_>>();
        formula_stack_cond_set_type(&mut stack, CP_TYPE_HYPOTHESIS);
        assert_eq!(stack[0].query_tptp_type(), CP_TYPE_HYPOTHESIS);
        assert_eq!(stack[2].query_tptp_type(), CP_TYPE_CONJECTURE);
        formula_stack_cond_set_type(&mut stack, CP_TYPE_CONJECTURE);
        assert!(stack
            .iter()
            .all(|formula| formula.query_tptp_type() == CP_TYPE_CONJECTURE));
    }

    #[test]
    fn formula_set_weight_fcode_and_symbol_queries_use_formula_terms() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "formula_set_a");
        let f_a = typed_unary(&mut bank, "formula_set_f", &a);
        let mut normal = WrappedFormula::wt_formula_alloc(f_a.clone());
        normal.set_tptp_type(CP_TYPE_NEG_CONJECTURE);
        let mut lambda_def = WrappedFormula::wt_formula_alloc(a.clone());
        lambda_def.set_prop(CP_IS_LAMBDA_DEF);
        let normal_id = normal.entry_id();
        let mut set = FormulaSet::new();
        set.insert(normal);
        set.insert(lambda_def);

        assert_eq!(set.standard_weight(), term_standard_weight(&f_a));
        let mut result = Vec::new();
        assert_eq!(set.collect_f_code(f_a.f_code(), &mut result), 1);
        assert_eq!(result, vec![normal_id]);
        assert_eq!(set.conjecture_order(bank.signature()), 0);
        assert!(!set.has_interpreted_symbol());
        assert!(set.is_untyped());

        let int_type = bank
            .signature_mut()
            .type_bank_mut()
            .insert_type_shared(alloc_simple_sort(ST_INTEGER));
        let int_code = bank.signature_mut().insert_id("formula_set_int", 0, false);
        bank.signature_mut()
            .declare_final_type(int_code, int_type)
            .unwrap();
        let int_term = bank.create_const_term(int_code).unwrap();
        set.insert(WrappedFormula::wt_formula_alloc(int_term));
        assert!(set.has_interpreted_symbol());
        assert!(!set.is_untyped());
    }

    #[test]
    fn formula_set_mark_polarity_marks_each_wrapped_formula_like_c() {
        let mut bank = test_bank();
        let first = typed_const(&mut bank, "pol_first");
        let second = typed_const(&mut bank, "pol_second");
        let third = typed_const(&mut bank, "pol_third");
        let fourth = typed_const(&mut bank, "pol_fourth");
        let fifth = typed_const(&mut bank, "pol_fifth");
        let sixth = typed_const(&mut bank, "pol_sixth");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let impl_code = bank.signature().impl_code();
        let or_code = bank.signature().or_code();
        let first_atom = bool_binary_with_code(&mut bank, eqn_code, &first, &second);
        let second_atom = bool_binary_with_code(&mut bank, eqn_code, &third, &fourth);
        let third_atom = bool_binary_with_code(&mut bank, eqn_code, &fifth, &sixth);
        let left_disjunction = bool_binary_with_code(&mut bank, or_code, &first_atom, &second_atom);
        let implication =
            bool_binary_with_code(&mut bank, impl_code, &left_disjunction, &third_atom);
        let disjunction = bool_binary_with_code(&mut bank, or_code, &second_atom, &third_atom);
        let mut set = FormulaSet::new();
        set.insert(WrappedFormula::default_alloc());
        set.insert(WrappedFormula::wt_formula_alloc(implication.clone()));
        set.insert(WrappedFormula::wt_formula_alloc(disjunction.clone()));

        set.mark_polarity(&bank);

        assert_eq!(tformula_decode_polarity(&implication), 1);
        assert_eq!(tformula_decode_polarity(&left_disjunction), -1);
        assert_eq!(tformula_decode_polarity(&disjunction), 1);
    }

    #[test]
    fn wrapped_formula_return_f_codes_preserves_order_and_skips_phony_apps() {
        let mut bank = test_bank();
        let first = typed_const(&mut bank, "fc_first");
        let second = typed_const(&mut bank, "fc_second");
        let third = typed_const(&mut bank, "fc_third");
        let var = typed_var(&bank, -41);
        let applied = phony_app(&mut bank, &var, &third);
        let and_code = bank.signature().and_code();
        let or_code = bank.signature().or_code();
        let pair = bool_binary_with_code(&mut bank, and_code, &first, &applied);
        let formula = bool_binary_with_code(&mut bank, or_code, &pair, &second);
        let wrapped = WrappedFormula::wt_formula_alloc(formula);
        let mut f_codes = vec![999_999];

        assert_eq!(wrapped.return_f_codes(&mut f_codes), 5);
        assert_eq!(
            f_codes,
            vec![
                999_999,
                or_code,
                and_code,
                first.f_code(),
                third.f_code(),
                second.f_code(),
            ]
        );
        assert_eq!(wrapped.symbol_diversity(), 5);
        assert!(!f_codes.contains(&SIG_PHONY_APP_CODE));
    }

    #[test]
    fn wrapped_formula_get_lambda_defined_symbol_matches_c_shapes() {
        let mut bank = test_bank();
        let head = typed_const(&mut bank, "lambda_defined_head");
        let rhs = typed_const(&mut bank, "lambda_defined_rhs");
        let quantified_var = typed_var(&bank, -42);
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let qall_code = bank.signature().qall_code();
        let equiv_code = bank.signature().equiv_code();
        let equation = bool_binary_with_code(&mut bank, eqn_code, &head, &rhs);
        let quantified = quantified_with_code(&mut bank, qall_code, &quantified_var, &equation);
        let mut equation_definition = WrappedFormula::wt_formula_alloc(quantified);
        equation_definition.set_prop(CP_IS_LAMBDA_DEF);

        assert_eq!(
            equation_definition.get_lambda_defined_symbol(bank.signature()),
            Some(head.f_code())
        );

        let true_term = bank.true_term().clone();
        let predicate_equation = bool_binary_with_code(&mut bank, eqn_code, &head, &true_term);
        let true_formula = bank.true_term().clone();
        let equivalence =
            bool_binary_with_code(&mut bank, equiv_code, &predicate_equation, &true_formula);
        let mut equivalence_definition = WrappedFormula::wt_formula_alloc(equivalence);
        equivalence_definition.set_prop(CP_IS_LAMBDA_DEF);
        assert_eq!(
            equivalence_definition.get_lambda_defined_symbol(bank.signature()),
            Some(head.f_code())
        );

        let mut untagged = equivalence_definition.flat_copy();
        untagged.del_prop(CP_IS_LAMBDA_DEF);
        assert_eq!(untagged.get_lambda_defined_symbol(bank.signature()), None);
    }

    #[test]
    fn formula_set_definition_statistics_matches_c_scan_shapes() {
        let mut bank = test_bank();
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let equiv_code = bank.signature().equiv_code();
        let or_code = bank.signature().or_code();

        let func_head = typed_const(&mut bank, "stats_function_head");
        let func_rhs = typed_const(&mut bank, "stats_function_rhs");
        let function_equation = bool_binary_with_code(&mut bank, eqn_code, &func_head, &func_rhs);
        let mut function_definition = WrappedFormula::wt_formula_alloc(function_equation);
        function_definition.set_prop(CP_IS_LAMBDA_DEF);

        let predicate_head = typed_predicate_const(&mut bank, "stats_predicate_head");
        let true_term = bank.true_term().clone();
        let predicate_equation =
            bool_binary_with_code(&mut bank, eqn_code, &predicate_head, &true_term);
        let true_formula = bank.true_term().clone();
        let predicate_equivalence =
            bool_binary_with_code(&mut bank, equiv_code, &predicate_equation, &true_formula);
        let mut predicate_definition = WrappedFormula::wt_formula_alloc(predicate_equivalence);
        predicate_definition.set_prop(CP_IS_LAMBDA_DEF);

        let loose_head = typed_predicate_const(&mut bank, "stats_loose_predicate_head");
        let loose_rhs = typed_predicate_const(&mut bank, "stats_loose_predicate_rhs");
        let loose_equation = bool_binary_with_code(&mut bank, eqn_code, &loose_head, &loose_rhs);
        let true_formula = bank.true_term().clone();
        let loose_equivalence =
            bool_binary_with_code(&mut bank, equiv_code, &loose_equation, &true_formula);
        let mut loose_definition = WrappedFormula::wt_formula_alloc(loose_equivalence);
        loose_definition.set_prop(CP_IS_LAMBDA_DEF);
        assert_eq!(
            loose_definition.get_lambda_defined_symbol(bank.signature()),
            Some(loose_head.f_code())
        );

        let binder_type = bank.signature().type_bank().default_type();
        let lambda_body = typed_const(&mut bank, "stats_lambda_body");
        let lambda = close_with_db_var(&mut bank, &binder_type, &lambda_body).unwrap();
        assert_eq!(lambda.f_code(), SIG_DB_LAMBDA_CODE);
        let lambda_type = lambda.type_().expect("lambda is typed");
        let container_ret_type = bank.signature().type_bank().default_type();
        let lambda_container = typed_unary_with_types(
            &mut bank,
            "stats_lambda_container",
            &lambda,
            &lambda_type,
            &container_ret_type,
        );
        let lambda_rhs = typed_const(&mut bank, "stats_lambda_rhs");
        let lambda_literal =
            bool_binary_with_code(&mut bank, eqn_code, &lambda_container, &lambda_rhs);

        let app_head = typed_var(&bank, -101);
        let app_arg = typed_const(&mut bank, "stats_app_arg");
        let app_var = phony_app(&mut bank, &app_head, &app_arg);
        let app_literal = bool_binary_with_code(&mut bank, eqn_code, &app_var, &true_term);
        let normal_literal = bool_binary_with_code(&mut bank, eqn_code, &func_rhs, &func_rhs);
        let app_formula = bool_binary_with_code(&mut bank, or_code, &app_literal, &normal_literal);

        let mut orig = FormulaSet::new();
        orig.insert(function_definition);
        orig.insert(loose_definition);
        orig.insert(WrappedFormula::wt_formula_alloc(lambda_literal));
        let mut arch = FormulaSet::new();
        arch.insert(predicate_definition);
        arch.insert(WrappedFormula::wt_formula_alloc(app_formula));

        let stats = formula_set_definition_statistics(&orig, &arch, &bank);

        assert_eq!(stats.num_defs, 2);
        assert!((stats.percentage_form_defs - 0.5).abs() < f64::EPSILON);
        assert_eq!(stats.num_lams, 1);
        assert!(stats.has_app_var_lits);

        let empty = FormulaSet::new();
        assert_eq!(
            formula_set_definition_statistics(&empty, &empty, &bank),
            FormulaDefinitionStatistics::default()
        );
    }
}
