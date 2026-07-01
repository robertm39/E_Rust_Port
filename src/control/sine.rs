use crate::basics::error::{Diagnostic, ErrorCode};
use crate::basics::pstacks::PStack;
use crate::clauses::clause::Clause;
use crate::clauses::clausesets::ClauseSet;
use crate::clauses::f_generality::GenDistrib;
use crate::clauses::formulasets::{FormulaSet, WrappedFormula};
use crate::clauses::sine::{
    select_axioms_clause_formula_sets, select_definitions_formula_sets,
    select_threshold_clause_formula_sets, ClauseSineParams, FormulaSineOptions, SineSetStacks,
};
use crate::heuristics::axfilter::{AxFilter, AxFilterType};
use crate::terms::functypes::FunCode;
use crate::terms::signature::Signature;
use std::collections::BTreeSet;

#[derive(Clone, Debug, PartialEq)]
pub struct StructFofSpec {
    clause_sets: Vec<ClauseSet>,
    formula_sets: Vec<FormulaSet>,
    parsed_includes: BTreeSet<String>,
    shared_ax_sp: usize,
    shared_ax_f_count: FunCode,
    f_distrib: GenDistrib,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StructFofSpecBacktrackReport {
    pub removed_clause_sets: usize,
    pub removed_formula_sets: usize,
    pub signature_backtrack_to: FunCode,
}

#[derive(Debug, PartialEq)]
pub struct StructFofSpecSelection<'a> {
    pub selected_count: i64,
    pub clauses: PStack<&'a Clause>,
    pub formulas: PStack<&'a WrappedFormula>,
}

impl StructFofSpec {
    #[must_use]
    pub fn new(signature: &Signature) -> Self {
        Self {
            clause_sets: Vec::new(),
            formula_sets: Vec::new(),
            parsed_includes: BTreeSet::new(),
            shared_ax_sp: 0,
            shared_ax_f_count: signature.f_count(),
            f_distrib: GenDistrib::new(signature),
        }
    }

    #[must_use]
    pub fn clause_set_count(&self) -> usize {
        self.clause_sets.len()
    }

    #[must_use]
    pub fn formula_set_count(&self) -> usize {
        self.formula_sets.len()
    }

    #[must_use]
    pub const fn shared_ax_sp(&self) -> usize {
        self.shared_ax_sp
    }

    #[must_use]
    pub const fn shared_ax_f_count(&self) -> FunCode {
        self.shared_ax_f_count
    }

    #[must_use]
    pub const fn f_distrib(&self) -> &GenDistrib {
        &self.f_distrib
    }

    #[must_use]
    pub fn has_parsed_include(&self, include: &str) -> bool {
        self.parsed_includes.contains(include)
    }

    pub fn mark_include_parsed(&mut self, include: impl Into<String>) -> bool {
        self.parsed_includes.insert(include.into())
    }

    pub fn reset_shared(&mut self) {
        self.shared_ax_sp = 0;
    }

    pub fn mark_shared_axioms(&mut self, signature: &Signature) {
        self.shared_ax_sp = self.clause_sets.len();
        self.shared_ax_f_count = signature.f_count();
    }

    pub fn init_distrib(&mut self, signature: &Signature, trim_implications: bool) {
        self.f_distrib.size_adjust(signature);
        let f_distrib = &mut self.f_distrib;
        for set in &self.clause_sets {
            f_distrib.add_clause_set(set, 1);
        }
        for set in &self.formula_sets {
            f_distrib.add_formula_set(signature, set, trim_implications, 1);
        }
    }

    pub fn add_problem(
        &mut self,
        signature: &Signature,
        clauses: ClauseSet,
        formulas: FormulaSet,
        trim_implications: bool,
    ) {
        self.f_distrib.size_adjust(signature);
        self.f_distrib.add_clause_set(&clauses, 1);
        self.f_distrib
            .add_formula_set(signature, &formulas, trim_implications, 1);
        self.clause_sets.push(clauses);
        self.formula_sets.push(formulas);
    }

    pub fn backtrack_to_spec(&mut self, signature: &Signature) -> StructFofSpecBacktrackReport {
        let removed_clause_sets = self.clause_sets.len().saturating_sub(self.shared_ax_sp);
        let removed_formula_sets = self.formula_sets.len().saturating_sub(self.shared_ax_sp);

        {
            let f_distrib = &mut self.f_distrib;
            for set in self.clause_sets.iter().skip(self.shared_ax_sp) {
                f_distrib.add_clause_set(set, -1);
            }
            for set in self.formula_sets.iter().skip(self.shared_ax_sp) {
                f_distrib.add_formula_set(signature, set, false, -1);
            }
        }

        self.clause_sets.truncate(self.shared_ax_sp);
        self.formula_sets.truncate(self.shared_ax_sp);

        StructFofSpecBacktrackReport {
            removed_clause_sets,
            removed_formula_sets,
            signature_backtrack_to: self.shared_ax_f_count,
        }
    }

    pub fn collect_f_code(&self, f_code: FunCode, result: &mut Vec<u64>) -> i64 {
        self.formula_sets
            .iter()
            .map(|set| set.collect_f_code(f_code, result))
            .sum()
    }

    pub fn get_problem<'a>(
        &'a mut self,
        signature: &Signature,
        filter: &AxFilter,
    ) -> Result<StructFofSpecSelection<'a>, Diagnostic> {
        let clause_sets = self.clause_set_stack();
        let formula_sets = self.formula_set_stack();
        let mut clauses = PStack::new();
        let mut formulas = PStack::new();

        let selected_count = match filter.type_ {
            AxFilterType::GSinE => {
                let mut selection_distrib = self.f_distrib.clone();
                select_axioms_clause_formula_sets(
                    &mut selection_distrib,
                    signature,
                    SineSetStacks {
                        clauses: &clause_sets,
                        formulas: &formula_sets,
                    },
                    self.shared_ax_sp,
                    clause_sine_params_from_filter(filter),
                    &mut clauses,
                    &mut formulas,
                )
            }
            AxFilterType::Threshold => select_threshold_clause_formula_sets(
                &clause_sets,
                &formula_sets,
                filter.threshold,
                &mut clauses,
                &mut formulas,
            ),
            AxFilterType::LambdaDefines => select_definitions_formula_sets(
                &clause_sets,
                &formula_sets,
                &mut clauses,
                &mut formulas,
            ),
            AxFilterType::NoFilter => {
                return Err(Diagnostic::new(
                    ErrorCode::INTERFACE_ERROR,
                    "Unknown AxFilter type in StructFofSpecGetProblem",
                ));
            }
        };

        Ok(StructFofSpecSelection {
            selected_count,
            clauses,
            formulas,
        })
    }

    fn clause_set_stack(&self) -> PStack<&ClauseSet> {
        let mut stack = PStack::new();
        for set in &self.clause_sets {
            stack.push(set);
        }
        stack
    }

    fn formula_set_stack(&self) -> PStack<&FormulaSet> {
        let mut stack = PStack::new();
        for set in &self.formula_sets {
            stack.push(set);
        }
        stack
    }
}

fn clause_sine_params_from_filter(filter: &AxFilter) -> ClauseSineParams {
    ClauseSineParams {
        gen_measure: filter.gen_measure,
        use_hypotheses: filter.use_hypotheses,
        benevolence: filter.benevolence,
        generosity: filter.generosity,
        max_recursion_depth: filter.max_recursion_depth,
        max_set_size: filter.max_set_size,
        max_set_fraction: filter.max_set_fraction,
        formula_options: FormulaSineOptions {
            trim_implications: filter.trim_implications,
            defined_symbols_in_drel: filter.defined_symbols_in_drel,
        },
        add_no_symbol_axioms: filter.add_no_symbol_axioms,
    }
}

#[cfg(test)]
mod tests {
    use super::StructFofSpec;
    use crate::basics::error::ErrorCode;
    use crate::clauses::clause::Clause;
    use crate::clauses::clause_props::{
        CP_IS_LAMBDA_DEF, CP_TYPE_AXIOM, CP_TYPE_CONJECTURE, CP_TYPE_HYPOTHESIS,
    };
    use crate::clauses::clausesets::ClauseSet;
    use crate::clauses::eqn::Eqn;
    use crate::clauses::eqnlist::EqnList;
    use crate::clauses::formulasets::{FormulaSet, WrappedFormula};
    use crate::heuristics::axfilter::AxFilter;
    use crate::terms::signature::Signature;
    use crate::terms::simpletypes::alloc_arrow_type;
    use crate::terms::termbanks::TermBank;
    use crate::terms::termtypes::{DerefType, Term};
    use crate::terms::typebanks::TypeBank;

    #[test]
    fn structured_spec_tracks_include_registry_and_shared_boundary() {
        let mut bank = test_bank();
        let initial_f_count = bank.signature().f_count();
        let mut spec = StructFofSpec::new(bank.signature());

        assert_eq!(spec.clause_set_count(), 0);
        assert_eq!(spec.formula_set_count(), 0);
        assert_eq!(spec.shared_ax_sp(), 0);
        assert_eq!(spec.shared_ax_f_count(), initial_f_count);
        assert!(spec.mark_include_parsed("Axioms/SET001.ax"));
        assert!(!spec.mark_include_parsed("Axioms/SET001.ax"));
        assert!(spec.has_parsed_include("Axioms/SET001.ax"));

        let shared = clause_set_with_clause(&mut bank, "shared", 1, CP_TYPE_AXIOM);
        spec.add_problem(bank.signature(), shared, FormulaSet::new(), false);
        spec.mark_shared_axioms(bank.signature());
        assert_eq!(spec.shared_ax_sp(), 1);

        spec.reset_shared();
        assert_eq!(spec.shared_ax_sp(), 0);
    }

    #[test]
    fn threshold_get_problem_preserves_clause_then_formula_stack_order() {
        let mut bank = test_bank();
        let mut spec = StructFofSpec::new(bank.signature());
        let shared = clause_set_with_clause(&mut bank, "threshold_shared", 10, CP_TYPE_AXIOM);
        let mut shared_forms = FormulaSet::new();
        let shared_formula = wrapped_formula(&mut bank, "threshold_shared_formula", CP_TYPE_AXIOM);
        let shared_formula_id = shared_formula.entry_id();
        shared_forms.insert(shared_formula);
        spec.add_problem(bank.signature(), shared, shared_forms, false);
        spec.mark_shared_axioms(bank.signature());

        let problem =
            clause_set_with_clause(&mut bank, "threshold_problem", 20, CP_TYPE_CONJECTURE);
        let mut problem_forms = FormulaSet::new();
        let problem_formula =
            wrapped_formula(&mut bank, "threshold_problem_formula", CP_TYPE_CONJECTURE);
        let problem_formula_id = problem_formula.entry_id();
        problem_forms.insert(problem_formula);
        spec.add_problem(bank.signature(), problem, problem_forms, false);

        let selection = spec
            .get_problem(bank.signature(), &AxFilter::threshold(10))
            .unwrap();

        assert_eq!(selection.selected_count, 4);
        let clause_ids = selection
            .clauses
            .as_slice()
            .iter()
            .map(|clause| clause.ident())
            .collect::<Vec<_>>();
        let formula_ids = selection
            .formulas
            .as_slice()
            .iter()
            .map(|formula| formula.entry_id())
            .collect::<Vec<_>>();
        assert_eq!(clause_ids, [10, 20]);
        assert_eq!(formula_ids, [shared_formula_id, problem_formula_id]);
    }

    #[test]
    fn gsine_get_problem_starts_seed_scan_after_shared_axioms() {
        let mut bank = test_bank();
        let mut spec = StructFofSpec::new(bank.signature());
        let shared = clause_set_with_clause(&mut bank, "gsine_shared", 100, CP_TYPE_AXIOM);
        spec.add_problem(bank.signature(), shared, FormulaSet::new(), false);
        spec.mark_shared_axioms(bank.signature());

        let goal_symbol = typed_const(&mut bank, "gsine_goal");
        let bridge_symbol = typed_unary(&mut bank, "gsine_bridge", &goal_symbol);
        let unrelated_symbol = typed_const(&mut bank, "gsine_unrelated");
        let mut goal = unit_clause(&mut bank, &goal_symbol, &goal_symbol, true);
        goal.set_ident(200);
        goal.set_tptp_type(CP_TYPE_CONJECTURE);
        let mut bridge = unit_clause(&mut bank, &bridge_symbol, &bridge_symbol, true);
        bridge.set_ident(201);
        bridge.set_tptp_type(CP_TYPE_AXIOM);
        let mut unrelated = unit_clause(&mut bank, &unrelated_symbol, &unrelated_symbol, true);
        unrelated.set_ident(202);
        unrelated.set_tptp_type(CP_TYPE_AXIOM);
        spec.add_problem(
            bank.signature(),
            ClauseSet::from_clauses([goal, bridge, unrelated]),
            FormulaSet::new(),
            false,
        );

        let mut filter = AxFilter::g_sine(crate::clauses::f_generality::GeneralityMeasure::Terms);
        filter.max_set_size = 10;
        filter.max_set_fraction = 1.0;
        filter.benevolence = 10.0;

        let selection = spec.get_problem(bank.signature(), &filter).unwrap();
        let clause_ids = selection
            .clauses
            .as_slice()
            .iter()
            .map(|clause| clause.ident())
            .collect::<Vec<_>>();

        assert!(clause_ids.contains(&200));
        assert!(clause_ids.contains(&201));
        assert!(!clause_ids.contains(&100));
        assert!(!clause_ids.contains(&202));
    }

    #[test]
    fn lambda_defines_selection_keeps_formula_defs_goals_and_hypotheses() {
        let mut bank = test_bank();
        let mut spec = StructFofSpec::new(bank.signature());
        let mut formulas = FormulaSet::new();
        let mut axiom = wrapped_formula(&mut bank, "lambda_axiom", CP_TYPE_AXIOM);
        axiom.set_prop(CP_IS_LAMBDA_DEF);
        let axiom_id = axiom.entry_id();
        let conjecture = wrapped_formula(&mut bank, "lambda_conjecture", CP_TYPE_CONJECTURE);
        let conjecture_id = conjecture.entry_id();
        let hypothesis = wrapped_formula(&mut bank, "lambda_hypothesis", CP_TYPE_HYPOTHESIS);
        let hypothesis_id = hypothesis.entry_id();
        let ignored = wrapped_formula(&mut bank, "lambda_ignored", CP_TYPE_AXIOM);
        let ignored_id = ignored.entry_id();
        formulas.insert(axiom);
        formulas.insert(conjecture);
        formulas.insert(hypothesis);
        formulas.insert(ignored);
        spec.add_problem(bank.signature(), ClauseSet::new(), formulas, false);

        let selection = spec
            .get_problem(bank.signature(), &AxFilter::lambda_defines())
            .unwrap();
        let formula_ids = selection
            .formulas
            .as_slice()
            .iter()
            .map(|formula| formula.entry_id())
            .collect::<Vec<_>>();

        assert_eq!(selection.selected_count, 3);
        assert_eq!(formula_ids, [axiom_id, conjecture_id, hypothesis_id]);
        assert!(!formula_ids.contains(&ignored_id));
    }

    #[test]
    fn backtrack_to_spec_removes_problem_sets_and_reports_signature_target() {
        let mut bank = test_bank();
        let mut spec = StructFofSpec::new(bank.signature());
        let shared = clause_set_with_clause(&mut bank, "backtrack_shared", 1, CP_TYPE_AXIOM);
        spec.add_problem(bank.signature(), shared, FormulaSet::new(), false);
        spec.mark_shared_axioms(bank.signature());
        let signature_target = spec.shared_ax_f_count();

        let problem = clause_set_with_clause(&mut bank, "backtrack_problem", 2, CP_TYPE_AXIOM);
        spec.add_problem(bank.signature(), problem, FormulaSet::new(), false);

        let report = spec.backtrack_to_spec(bank.signature());

        assert_eq!(report.removed_clause_sets, 1);
        assert_eq!(report.removed_formula_sets, 1);
        assert_eq!(report.signature_backtrack_to, signature_target);
        assert_eq!(spec.clause_set_count(), 1);
        assert_eq!(spec.formula_set_count(), 1);
    }

    #[test]
    fn collect_f_code_scans_formula_sets_in_order() {
        let mut bank = test_bank();
        let symbol = typed_const(&mut bank, "collect_symbol");
        let f_code = symbol.f_code();
        let other = typed_const(&mut bank, "collect_other");
        let mut first = FormulaSet::new();
        let first_match = WrappedFormula::wt_formula_alloc(symbol.clone());
        let first_id = first_match.entry_id();
        first.insert(first_match);
        let mut second = FormulaSet::new();
        second.insert(WrappedFormula::wt_formula_alloc(other));
        let second_match = WrappedFormula::wt_formula_alloc(symbol);
        let second_id = second_match.entry_id();
        second.insert(second_match);

        let mut spec = StructFofSpec::new(bank.signature());
        spec.add_problem(bank.signature(), ClauseSet::new(), first, false);
        spec.add_problem(bank.signature(), ClauseSet::new(), second, false);
        let mut result = Vec::new();

        assert_eq!(spec.collect_f_code(f_code, &mut result), 2);
        assert_eq!(result, [first_id, second_id]);
    }

    #[test]
    fn get_problem_rejects_no_filter_like_c_assertion_boundary() {
        let bank = test_bank();
        let mut spec = StructFofSpec::new(bank.signature());
        let error = spec
            .get_problem(bank.signature(), &AxFilter::new())
            .unwrap_err();
        assert_eq!(error.code(), ErrorCode::INTERFACE_ERROR);
    }

    fn test_bank() -> TermBank {
        let mut signature = Signature::new(TypeBank::new());
        signature.insert_internal_codes().unwrap();
        TermBank::new(signature).unwrap()
    }

    fn clause_set_with_clause(
        bank: &mut TermBank,
        name: &str,
        ident: i64,
        type_: crate::clauses::clause_props::FormulaProperties,
    ) -> ClauseSet {
        let term = typed_const(bank, name);
        let mut clause = unit_clause(bank, &term, &term, true);
        clause.set_ident(ident);
        clause.set_tptp_type(type_);
        ClauseSet::from_clauses([clause])
    }

    fn wrapped_formula(
        bank: &mut TermBank,
        name: &str,
        type_: crate::clauses::clause_props::FormulaProperties,
    ) -> WrappedFormula {
        let mut formula = WrappedFormula::wt_formula_alloc(typed_const(bank, name));
        formula.set_tptp_type(type_);
        formula
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
        let arrow = bank
            .signature_mut()
            .type_bank_mut()
            .insert_type_shared(alloc_arrow_type(vec![type_.clone(), type_.clone()]));
        let f_code = bank.signature_mut().insert_id(name, 1, false);
        bank.signature_mut()
            .declare_final_type(f_code, arrow)
            .unwrap();
        let term = Term::top_alloc(f_code, 1);
        term.set_type(Some(type_));
        term.set_argument(0, arg.clone());
        bank.insert(&term, DerefType::Never).unwrap()
    }

    fn unit_clause(bank: &mut TermBank, left: &Term, right: &Term, positive: bool) -> Clause {
        let literal = Eqn::alloc(left.clone(), right.clone(), bank, positive).unwrap();
        let mut clause = Clause::alloc(EqnList::from_vec(vec![literal]));
        clause.set_weight(clause.standard_weight());
        clause
    }
}
