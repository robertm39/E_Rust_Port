use crate::basics::error::Diagnostic;
use crate::clauses::clause::Clause;
use crate::clauses::clause_props::CP_IS_ORIENTED;
use crate::clauses::clausesets::ClauseSet;
use crate::clauses::eqn_props::{EP_IS_PM_INTO_LIT, EP_IS_SELECTED};
use crate::clauses::fcvindexing::FvIndexParams;
use crate::heuristics::clausesetfeatures::SpecFeatureCell;
use crate::heuristics::hcb::{HeuristicParmsCell, SplitClassType};
use crate::heuristics::hcbadmin::HcbAdmin;
use crate::heuristics::heuristic_lookup::get_heuristic_handle_with_context;
use crate::heuristics::litselection::{
    apply_ported_literal_selector_with_bank, UnsupportedLiteralSelection,
};
use crate::heuristics::to_autoselect::to_select_ordering;
use crate::heuristics::wfcbadmin::{WeightParseContext, WfcbAdmin};
use crate::inout::scanner::{Scanner, TokenType};
use crate::orderings::ocb::OrderControlBlock;
use crate::terms::termbanks::TermBank;

pub const DEFAULT_WEIGHT_FUNCTIONS: &str = concat!(
    "\n",
    "weight21_ugg  = Clauseweight(PreferUnitGroundGoals,2,1,1)      \n",
    "rweight21_g   = Refinedweight(PreferGoals,    2,1,1.5,1.1,1) \n",
    "rweight11_g   = Refinedweight(PreferGoals,    1,1,1.5,1.1,1.1) \n",
    "rweight21_a   = Refinedweight(PreferNonGoals, 2,1,1.5,1.1,1.1) \n",
    "rweight21_ugg = Refinedweight(PreferUnitGroundGoals, 2,1,1.5,1.1,1.1) \n",
    "fifo_f        = FIFOWeight(ConstPrio)                      \n",
    "lifo_f        = LIFOWeight(ConstPrio)                      \n",
    "weight11_f    = Clauseweight(ConstPrio,1,1,1)              \n",
    "weight11_ugg  = Clauseweight(PreferUnitGroundGoals,1,1,1)  \n",
    "weight21_f    = Clauseweight(ConstPrio,2,1,1)              \n",
    "TSMRDefault   = TSMWeight(ConstPrio, 1, 1, 2, flat, E_KNOWLEDGE,",
    "100000,1.0,1.0,Flat,IndexIdentity,100000,-20,20,-2,-1,0,2)\n",
);

pub const DEFAULT_HEURISTICS: &str = concat!(
    "Weight     = (1*weight21_ugg)                       \n",
    "WeightC1   = (1*weight11_ugg)                       \n",
    "StandardWeight = (1*weight21_f)                     \n",
    "StandardPG = (5*weight21_f,1*fifo_f)                \n",
    "RWeight    = (1*rweight21_ugg)                      \n",
    "FIFO       = (1*fifo_f)                             \n",
    "LIFO       = (1*lifo_f)                             \n",
    "Default    = (3*rweight21_a, 1*rweight21_g)         \n",
    "Uniq       = (1*Uniqweight(ConstPrio))\n",
    "UseWatchlist = \n",
    "(10*Refinedweight(PreferGoals,1,2,2,2,0.5),",
    " 10*Refinedweight(PreferNonGoals,2,1,2,2,2),",
    " 5*OrientLMaxWeight(PreferWatchlist,2,1,2,1,1),",
    " 1*FIFOWeight(PreferWatchlist))\n",
    "UseWatchlistPure=\n",
    "(1*Defaultweight(PreferWatchlist))\n",
    "UseWatchlistPG10=\n",
    "(10*Defaultweight(PreferWatchlist),\n",
    " 1*FIFOWeight(ConstPrio))\n",
    "UseWatchlistEvo=\n",
    "(1*ConjectureRelativeSymbolWeight(SimulateSOS,0.5, 100, 100,\n",
    "                                  100, 100, 1.5, 1.5, 1),\n",
    " 4*ConjectureRelativeSymbolWeight(PreferWatchlist,0.1, 100, \n",
    "                                  100, 100, 100, 1.5, 1.5, 1.5),\n",
    " 1*FIFOWeight(PreferProcessed),\n",
    " 1*ConjectureRelativeSymbolWeight(PreferWatchlist,0.5, 100, 100, \n",
    "                                  100, 100, 1.5, 1.5, 1),\n",
    " 4*Refinedweight(SimulateSOS,3,2,2,1.5,2))\n",
    "UseTSM1 = \n",
    "(10*Refinedweight(PreferGoals,1,2,2,2,0.5),",
    " 10*Refinedweight(PreferNonGoals,2,1,2,2,2),",
    " 5*TSMRDefault,",
    " 1*FIFOWeight(PreferWatchlist))\n",
    "UseTSM2 = \n",
    "(20*TSMRDefault,",
    " 5*OrientLMaxWeight(PreferWatchlist,2,1,2,1,1),",
    " 1*FIFOWeight(PreferWatchlist)).",
);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SatSolverState {
    generation: u64,
    trace_generation_enabled: bool,
}

impl SatSolverState {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            generation: 1,
            trace_generation_enabled: true,
        }
    }

    #[must_use]
    pub const fn generation(self) -> u64 {
        self.generation
    }

    #[must_use]
    pub const fn trace_generation_enabled(self) -> bool {
        self.trace_generation_enabled
    }

    pub fn reset(&mut self) {
        self.generation = self.generation.saturating_add(1);
        self.trace_generation_enabled = true;
    }
}

impl Default for SatSolverState {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LiteralSelectionOutcome {
    Inherited,
    SelectorApplied,
    SelectionSkipped,
}

pub struct ProofControl {
    ocb: Option<OrderControlBlock>,
    active_hcb: Option<usize>,
    wfcbs: WfcbAdmin,
    hcbs: HcbAdmin,
    ac_handling_active: bool,
    heuristic_parms: HeuristicParmsCell,
    fvi_parms: FvIndexParams,
    problem_specs: SpecFeatureCell,
    solver: SatSolverState,
}

impl ProofControl {
    #[must_use]
    pub fn new() -> Self {
        Self {
            ocb: None,
            active_hcb: None,
            wfcbs: WfcbAdmin::new(),
            hcbs: HcbAdmin::new(),
            ac_handling_active: false,
            heuristic_parms: HeuristicParmsCell::default(),
            fvi_parms: FvIndexParams::default(),
            problem_specs: SpecFeatureCell::default(),
            solver: SatSolverState::new(),
        }
    }

    #[must_use]
    pub const fn ocb(&self) -> Option<&OrderControlBlock> {
        self.ocb.as_ref()
    }

    pub fn set_ocb(&mut self, ocb: OrderControlBlock) {
        self.ocb = Some(ocb);
    }

    #[must_use]
    pub fn take_ocb(&mut self) -> Option<OrderControlBlock> {
        self.ocb.take()
    }

    #[must_use]
    pub const fn active_hcb(&self) -> Option<usize> {
        self.active_hcb
    }

    pub const fn set_active_hcb(&mut self, active_hcb: Option<usize>) {
        self.active_hcb = active_hcb;
    }

    #[must_use]
    pub const fn wfcbs(&self) -> &WfcbAdmin {
        &self.wfcbs
    }

    pub const fn wfcbs_mut(&mut self) -> &mut WfcbAdmin {
        &mut self.wfcbs
    }

    #[must_use]
    pub const fn hcbs(&self) -> &HcbAdmin {
        &self.hcbs
    }

    pub const fn hcbs_mut(&mut self) -> &mut HcbAdmin {
        &mut self.hcbs
    }

    #[must_use]
    pub const fn ac_handling_active(&self) -> bool {
        self.ac_handling_active
    }

    pub const fn set_ac_handling_active(&mut self, active: bool) {
        self.ac_handling_active = active;
    }

    #[must_use]
    pub const fn heuristic_parms(&self) -> &HeuristicParmsCell {
        &self.heuristic_parms
    }

    pub const fn heuristic_parms_mut(&mut self) -> &mut HeuristicParmsCell {
        &mut self.heuristic_parms
    }

    pub fn set_heuristic_parms(&mut self, heuristic_parms: HeuristicParmsCell) {
        self.heuristic_parms = heuristic_parms;
    }

    #[must_use]
    pub const fn fvi_parms(&self) -> &FvIndexParams {
        &self.fvi_parms
    }

    pub fn set_fvi_parms(&mut self, fvi_parms: FvIndexParams) {
        self.fvi_parms = fvi_parms;
    }

    #[must_use]
    pub const fn problem_specs(&self) -> &SpecFeatureCell {
        &self.problem_specs
    }

    pub fn set_problem_specs(&mut self, problem_specs: SpecFeatureCell) {
        self.problem_specs = problem_specs;
    }

    #[must_use]
    pub const fn solver(&self) -> SatSolverState {
        self.solver
    }

    pub fn reset_sat_solver(&mut self) {
        self.solver.reset();
    }
}

impl Default for ProofControl {
    fn default() -> Self {
        Self::new()
    }
}

#[must_use]
pub fn proof_control_alloc() -> ProofControl {
    ProofControl::new()
}

pub fn proof_control_reset_sat_solver(control: &mut ProofControl) {
    control.reset_sat_solver();
}

/// Initializes the currently ported proof-control state handled by C
/// `ProofControlInit`.
///
/// The later proof-state setup in C `ProofStateInit`, including FV-index anchor
/// creation and clause-set insertion, is kept outside this helper until the
/// Rust proof-state owner is available.
///
/// # Errors
///
/// Returns diagnostics from ordering selection, built-in or user-supplied
/// weight/heuristic definition parsing, or active heuristic lookup.
#[expect(
    clippy::too_many_arguments,
    reason = "C-compatible ProofControlInit bridge keeps the original inputs visible"
)]
pub fn proof_control_init(
    control: &mut ProofControl,
    bank: &mut TermBank,
    axioms: &mut ClauseSet,
    params: &mut HeuristicParmsCell,
    fvi_params: &FvIndexParams,
    wfcb_defs: &[String],
    hcb_defs: &mut Vec<String>,
    higher_order_problem: bool,
) -> Result<(), Diagnostic> {
    debug_assert!(control.ocb.is_none());
    debug_assert!(control.active_hcb.is_none());

    let ocb = to_select_ordering(bank, axioms, params, higher_order_problem)?;
    control.ocb = Some(ocb);
    proof_control_init_heuristics(control, axioms, params, fvi_params, wfcb_defs, hcb_defs)
}

/// Installs the heuristic definition state handled by C `ProofControlInit`.
///
/// This helper is available separately for tests and staged integration points
/// that already own an OCB.
///
/// # Errors
///
/// Returns a diagnostic when a built-in or user-supplied weight/heuristic
/// definition cannot be parsed, or when the requested active heuristic is
/// unknown.
pub fn proof_control_init_heuristics(
    control: &mut ProofControl,
    axioms: &ClauseSet,
    params: &mut HeuristicParmsCell,
    fvi_params: &FvIndexParams,
    wfcb_defs: &[String],
    hcb_defs: &mut Vec<String>,
) -> Result<(), Diagnostic> {
    debug_assert!(control.active_hcb.is_none());

    let context = WeightParseContext::new(axioms);
    install_default_weight_functions(control, context)?;
    for definition in wfcb_defs {
        install_option_weight_functions(control, definition, context)?;
    }

    install_default_heuristics(control, context)?;
    if let Some(heuristic_def) = params.heuristic_def.clone() {
        hcb_defs.push(heuristic_def);
    } else if let Some(heuristic_def) = hcb_defs.last() {
        params.heuristic_def = Some(heuristic_def.clone());
    }
    for definition in hcb_defs.iter() {
        install_option_heuristics(control, definition, context)?;
    }

    control.heuristic_parms = params.clone();
    control.active_hcb = Some(get_heuristic_handle_with_context(
        &params.heuristic_name,
        &mut control.hcbs,
        &mut control.wfcbs,
        context,
    )?);
    control.fvi_parms = fvi_params.clone();
    if control.heuristic_parms.split_clauses == SplitClassType::NONE {
        control.fvi_parms.set_symbol_slack(0);
    }
    *params = control.heuristic_parms.clone();

    Ok(())
}

fn install_default_weight_functions(
    control: &mut ProofControl,
    context: WeightParseContext<'_>,
) -> Result<(), Diagnostic> {
    let mut scanner = Scanner::from_internal_string(DEFAULT_WEIGHT_FUNCTIONS, true)?;
    control
        .wfcbs
        .weight_fun_def_list_parse_with_context(&mut scanner, context)?;
    scanner.check_tok(TokenType::NO_TOKEN)
}

fn install_option_weight_functions(
    control: &mut ProofControl,
    definition: &str,
    context: WeightParseContext<'_>,
) -> Result<(), Diagnostic> {
    let mut scanner = Scanner::from_option_string(definition, true)?;
    control
        .wfcbs
        .weight_fun_def_list_parse_with_context(&mut scanner, context)?;
    scanner.check_tok(TokenType::NO_TOKEN)
}

fn install_default_heuristics(
    control: &mut ProofControl,
    context: WeightParseContext<'_>,
) -> Result<(), Diagnostic> {
    let mut scanner = Scanner::from_internal_string(DEFAULT_HEURISTICS, true)?;
    control.hcbs.heuristic_def_list_parse_with_context(
        &mut scanner,
        &mut control.wfcbs,
        context,
    )?;
    scanner.accept_tok(TokenType::FULLSTOP)?;
    scanner.check_tok(TokenType::NO_TOKEN)
}

fn install_option_heuristics(
    control: &mut ProofControl,
    definition: &str,
    context: WeightParseContext<'_>,
) -> Result<(), Diagnostic> {
    let mut scanner = Scanner::from_option_string(definition, true)?;
    control.hcbs.heuristic_def_list_parse_with_context(
        &mut scanner,
        &mut control.wfcbs,
        context,
    )?;
    scanner.check_tok(TokenType::NO_TOKEN)
}

#[must_use]
pub fn select_inherited_literal(clause: &mut Clause) -> bool {
    if clause.negative_literal_count() == 0 {
        return false;
    }
    let found = clause
        .literals()
        .as_slice()
        .iter()
        .any(|literal| literal.is_negative() && literal.query_prop(EP_IS_PM_INTO_LIT));
    if !found {
        return false;
    }
    for literal in clause.literals_mut().as_mut_slice() {
        if literal.query_prop(EP_IS_PM_INTO_LIT) {
            literal.set_prop(EP_IS_SELECTED);
        }
    }
    true
}

/// Runs the C `DoLiteralSelection` wrapper using a caller-supplied selector
/// body.
pub fn do_literal_selection_with_selector<S>(
    control: &mut ProofControl,
    clause: &mut Clause,
    mut selector: S,
) -> LiteralSelectionOutcome
where
    S: FnMut(Option<&mut OrderControlBlock>, &mut Clause),
{
    clear_literal_selection_state(clause);
    let parms = control.heuristic_parms();
    if should_try_inherited_selection(parms, clause) && select_inherited_literal(clause) {
        return LiteralSelectionOutcome::Inherited;
    }
    if literal_selection_conditions_hold(parms, clause) {
        debug_assert_eq!(clause.prop_lit_number(EP_IS_SELECTED), 0);
        selector(control.ocb.as_mut(), clause);
        LiteralSelectionOutcome::SelectorApplied
    } else {
        crate::heuristics::litselection::select_no_literals(control.ocb.as_mut(), clause);
        LiteralSelectionOutcome::SelectionSkipped
    }
}

/// Runs the C `DoLiteralSelection` wrapper using the literal-selection bodies
/// that have already been ported.
///
/// # Errors
///
/// Returns `UnsupportedLiteralSelection` if the configured selector body has
/// not been ported yet and the wrapper reaches the selector call.
pub fn do_literal_selection(
    control: &mut ProofControl,
    clause: &mut Clause,
) -> Result<LiteralSelectionOutcome, UnsupportedLiteralSelection> {
    do_literal_selection_impl(control, None, clause)
}

/// Runs the C `DoLiteralSelection` wrapper using ported selector bodies,
/// including selector bodies whose Rust implementation needs the term bank for
/// maximality marking.
///
/// # Errors
///
/// Returns `UnsupportedLiteralSelection` if the configured selector body has
/// not been ported yet and the wrapper reaches the selector call.
pub fn do_literal_selection_with_bank(
    control: &mut ProofControl,
    bank: &TermBank,
    clause: &mut Clause,
) -> Result<LiteralSelectionOutcome, UnsupportedLiteralSelection> {
    do_literal_selection_impl(control, Some(bank), clause)
}

fn do_literal_selection_impl(
    control: &mut ProofControl,
    bank: Option<&TermBank>,
    clause: &mut Clause,
) -> Result<LiteralSelectionOutcome, UnsupportedLiteralSelection> {
    clear_literal_selection_state(clause);
    let parms = control.heuristic_parms();
    if should_try_inherited_selection(parms, clause) && select_inherited_literal(clause) {
        return Ok(LiteralSelectionOutcome::Inherited);
    }
    if literal_selection_conditions_hold(parms, clause) {
        debug_assert_eq!(clause.prop_lit_number(EP_IS_SELECTED), 0);
        apply_ported_literal_selector_with_bank(
            control.heuristic_parms.selection_strategy.as_str(),
            control.ocb.as_mut(),
            bank,
            clause,
        )?;
        Ok(LiteralSelectionOutcome::SelectorApplied)
    } else {
        crate::heuristics::litselection::select_no_literals(control.ocb.as_mut(), clause);
        Ok(LiteralSelectionOutcome::SelectionSkipped)
    }
}

fn clear_literal_selection_state(clause: &mut Clause) {
    clause.literals_mut().del_prop(EP_IS_SELECTED);
    debug_assert_eq!(clause.prop_lit_number(EP_IS_SELECTED), 0);
    clause.del_prop(CP_IS_ORIENTED);
    debug_assert_eq!(clause.prop_lit_number(EP_IS_SELECTED), 0);
}

fn should_try_inherited_selection(parms: &HeuristicParmsCell, clause: &Clause) -> bool {
    parms.inherit_paramod_lit
        || (parms.inherit_goal_pm_lit && clause.is_goal())
        || (parms.inherit_conj_pm_lit && clause.is_conjecture())
}

fn literal_selection_conditions_hold(parms: &HeuristicParmsCell, clause: &Clause) -> bool {
    clause.negative_literal_count() != 0
        && count_in_range(
            clause.positive_literal_count(),
            parms.pos_lit_sel_min,
            parms.pos_lit_sel_max,
        )
        && count_in_range(
            clause.negative_literal_count(),
            parms.neg_lit_sel_min,
            parms.neg_lit_sel_max,
        )
        && count_in_range(
            clause.literal_number(),
            parms.all_lit_sel_min,
            parms.all_lit_sel_max,
        )
        && (parms.weight_sel_min == 0 || parms.weight_sel_min <= clause.standard_weight())
}

fn count_in_range(count: usize, min: i64, max: i64) -> bool {
    let count = i64::try_from(count).unwrap_or(i64::MAX);
    count >= min && count <= max
}

#[cfg(test)]
mod tests {
    use super::{
        do_literal_selection, do_literal_selection_with_bank, do_literal_selection_with_selector,
        proof_control_alloc, proof_control_init, proof_control_init_heuristics,
        proof_control_reset_sat_solver, select_inherited_literal, LiteralSelectionOutcome,
        DEFAULT_HEURISTICS, DEFAULT_WEIGHT_FUNCTIONS,
    };
    use crate::basics::partial_orderings::HoOrderKind;
    use crate::clauses::clause::Clause;
    use crate::clauses::clause_props::{CP_IS_ORIENTED, CP_TYPE_CONJECTURE};
    use crate::clauses::clausesets::ClauseSet;
    use crate::clauses::eqn::Eqn;
    use crate::clauses::eqn_props::{EP_IS_PM_INTO_LIT, EP_IS_SELECTED};
    use crate::clauses::eqnlist::EqnList;
    use crate::clauses::fcvindexing::FvIndexParams;
    use crate::clauses::freqvectors::{FvIndexType, FVINDEX_MAX_FEATURES_DEFAULT};
    use crate::heuristics::hcb::{HeuristicParmsCell, SplitClassType, HCB_DEFAULT_HEURISTIC};
    use crate::heuristics::litselection::SELECT_UNLESS_POS_MAX;
    use crate::heuristics::to_params::TermOrdering;
    use crate::orderings::ocb::OrderControlBlock;
    use crate::terms::signature::Signature;
    use crate::terms::termbanks::TermBank;
    use crate::terms::termtypes::Term;
    use crate::terms::typebanks::TypeBank;

    fn test_bank() -> TermBank {
        let mut signature = Signature::new(TypeBank::new());
        signature.insert_internal_codes().unwrap();
        TermBank::new(signature).unwrap()
    }

    fn typed_const(bank: &mut TermBank, name: &str) -> Term {
        let type_ = bank.signature().type_bank().default_type();
        let f_code = bank.signature_mut().insert_id(name, 0, false);
        if bank.signature().get_type(f_code).is_none() {
            bank.signature_mut()
                .declare_final_type(f_code, type_)
                .unwrap();
        }
        bank.create_const_term(f_code).unwrap()
    }

    fn literal(bank: &mut TermBank, left: &Term, right: &Term, positive: bool) -> Eqn {
        Eqn::alloc(left.clone(), right.clone(), bank, positive).unwrap()
    }

    fn mixed_clause() -> Clause {
        let mut bank = test_bank();
        let first = typed_const(&mut bank, "pc_a");
        let second = typed_const(&mut bank, "pc_b");
        Clause::alloc(EqnList::from_vec(vec![
            literal(&mut bank, &first, &second, true),
            literal(&mut bank, &second, &first, false),
        ]))
    }

    fn positive_clause() -> Clause {
        let mut bank = test_bank();
        let first = typed_const(&mut bank, "pc_pos_a");
        let second = typed_const(&mut bank, "pc_pos_b");
        Clause::alloc(EqnList::from_vec(vec![literal(
            &mut bank, &first, &second, true,
        )]))
    }

    fn negative_clause(bank: &mut TermBank) -> Clause {
        let first = typed_const(bank, "pc_neg_a");
        let second = typed_const(bank, "pc_neg_b");
        Clause::alloc(EqnList::from_vec(vec![literal(
            bank, &first, &second, false,
        )]))
    }

    fn kbo_ocb(bank: &TermBank) -> OrderControlBlock {
        OrderControlBlock::alloc(
            TermOrdering::Kbo,
            true,
            bank.signature(),
            HoOrderKind::LfhoOrder,
        )
    }

    #[test]
    fn proof_control_alloc_initializes_c_shape() {
        let control = proof_control_alloc();

        assert!(control.ocb().is_none());
        assert!(control.active_hcb().is_none());
        assert!(control.wfcbs().is_empty());
        assert!(control.hcbs().is_empty());
        assert!(!control.ac_handling_active());
        assert_eq!(control.heuristic_parms(), &HeuristicParmsCell::default());
        assert_eq!(
            control.heuristic_parms().heuristic_name,
            HCB_DEFAULT_HEURISTIC
        );
        assert_eq!(control.fvi_parms().cspec().features(), FvIndexType::AcFold);
        assert!(!control.fvi_parms().use_perm_vectors());
        assert_eq!(
            control.fvi_parms().max_symbols(),
            FVINDEX_MAX_FEATURES_DEFAULT
        );
        assert_eq!(control.problem_specs().clauses, 0);
        assert_eq!(control.solver().generation(), 1);
        assert!(control.solver().trace_generation_enabled());
    }

    #[test]
    fn proof_control_reset_sat_solver_reinitializes_trace_state() {
        let mut control = proof_control_alloc();

        proof_control_reset_sat_solver(&mut control);

        assert_eq!(control.solver().generation(), 2);
        assert!(control.solver().trace_generation_enabled());
    }

    #[test]
    fn default_definition_strings_match_c_surface() {
        assert!(DEFAULT_WEIGHT_FUNCTIONS.starts_with('\n'));
        assert!(DEFAULT_WEIGHT_FUNCTIONS.contains("weight21_ugg  = Clauseweight"));
        assert!(DEFAULT_WEIGHT_FUNCTIONS.contains("TSMRDefault   = TSMWeight"));
        assert!(DEFAULT_HEURISTICS.contains("Default    = (3*rweight21_a, 1*rweight21_g)"));
        assert!(DEFAULT_HEURISTICS.contains("UseWatchlist = \n"));
        assert!(DEFAULT_HEURISTICS.ends_with(" 1*FIFOWeight(PreferWatchlist))."));
    }

    #[test]
    fn proof_control_init_installs_default_definitions_and_active_hcb() {
        let mut control = proof_control_alloc();
        let axioms = ClauseSet::new();
        let mut params = HeuristicParmsCell::default();
        let fvi_params = FvIndexParams::new(FvIndexType::AcFold, true, true, 19, 7);
        let mut hcb_defs = Vec::new();

        proof_control_init_heuristics(
            &mut control,
            &axioms,
            &mut params,
            &fvi_params,
            &[],
            &mut hcb_defs,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        assert!(control.wfcbs().find_wfcb_handle("rweight21_a").is_some());
        assert!(control.wfcbs().find_wfcb_handle("TSMRDefault").is_some());
        let default_hcb = control
            .hcbs()
            .find_hcb_handle(HCB_DEFAULT_HEURISTIC)
            .unwrap_or_else(|| panic!("Default HCB should be installed"));
        assert_eq!(control.active_hcb(), Some(default_hcb));
        assert_eq!(control.fvi_parms().symbol_slack(), 0);
        assert_eq!(control.fvi_parms().max_symbols(), 19);
        assert!(control.fvi_parms().use_perm_vectors());
        assert!(control.fvi_parms().eliminate_uninformative());
        assert!(params.heuristic_def.is_none());
        assert!(hcb_defs.is_empty());
    }

    #[test]
    fn proof_control_init_preserves_symbol_slack_when_splitting_is_enabled() {
        let mut control = proof_control_alloc();
        let axioms = ClauseSet::new();
        let mut params = HeuristicParmsCell {
            split_clauses: SplitClassType::ALL,
            ..HeuristicParmsCell::default()
        };
        let fvi_params = FvIndexParams::new(FvIndexType::AcFold, false, false, 23, 11);
        let mut hcb_defs = Vec::new();

        proof_control_init_heuristics(
            &mut control,
            &axioms,
            &mut params,
            &fvi_params,
            &[],
            &mut hcb_defs,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(control.fvi_parms().symbol_slack(), 11);
    }

    #[test]
    fn proof_control_init_uses_user_definition_stack_like_c() {
        let mut control = proof_control_alloc();
        let axioms = ClauseSet::new();
        let mut params = HeuristicParmsCell {
            heuristic_name: "Alt".to_owned(),
            ..HeuristicParmsCell::default()
        };
        let fvi_params = FvIndexParams::default();
        let wfcb_defs = vec!["custom = FIFOWeight(ConstPrio)".to_owned()];
        let mut hcb_defs = vec!["Alt=(1*custom)".to_owned()];

        proof_control_init_heuristics(
            &mut control,
            &axioms,
            &mut params,
            &fvi_params,
            &wfcb_defs,
            &mut hcb_defs,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        assert!(control.wfcbs().find_wfcb_handle("custom").is_some());
        let alt_hcb = control
            .hcbs()
            .find_hcb_handle("Alt")
            .unwrap_or_else(|| panic!("user HCB should be installed"));
        assert_eq!(control.active_hcb(), Some(alt_hcb));
        assert_eq!(params.heuristic_def.as_deref(), Some("Alt=(1*custom)"));
        assert_eq!(hcb_defs, ["Alt=(1*custom)"]);
    }

    #[test]
    fn proof_control_init_pushes_direct_heuristic_def_like_c() {
        let mut control = proof_control_alloc();
        let axioms = ClauseSet::new();
        let mut params = HeuristicParmsCell {
            heuristic_name: "Injected".to_owned(),
            heuristic_def: Some("Injected=(1*fifo_f)".to_owned()),
            ..HeuristicParmsCell::default()
        };
        let fvi_params = FvIndexParams::default();
        let mut hcb_defs = Vec::new();

        proof_control_init_heuristics(
            &mut control,
            &axioms,
            &mut params,
            &fvi_params,
            &[],
            &mut hcb_defs,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        let injected_hcb = control
            .hcbs()
            .find_hcb_handle("Injected")
            .unwrap_or_else(|| panic!("direct HCB should be installed"));
        assert_eq!(control.active_hcb(), Some(injected_hcb));
        assert_eq!(hcb_defs, ["Injected=(1*fifo_f)"]);
        assert_eq!(params.heuristic_def.as_deref(), Some("Injected=(1*fifo_f)"));
    }

    #[test]
    fn proof_control_init_selects_ordering_then_initializes_heuristics() {
        let mut control = proof_control_alloc();
        let mut bank = test_bank();
        typed_const(&mut bank, "pc_init_order_a");
        let mut axioms = ClauseSet::new();
        let mut params = HeuristicParmsCell::default();
        params.order_params.ordertype = TermOrdering::NoOrdering;
        let fvi_params = FvIndexParams::default();
        let mut hcb_defs = Vec::new();

        proof_control_init(
            &mut control,
            &mut bank,
            &mut axioms,
            &mut params,
            &fvi_params,
            &[],
            &mut hcb_defs,
            false,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(
            control.ocb().map(|ocb| ocb.ordering_type),
            Some(TermOrdering::Kbo)
        );
        assert_eq!(
            control.active_hcb(),
            control.hcbs().find_hcb_handle(HCB_DEFAULT_HEURISTIC)
        );
        assert!(control.wfcbs().find_wfcb_handle("weight21_ugg").is_some());
    }

    #[test]
    fn inherited_literal_selection_requires_negative_pm_into_literal() {
        let mut clause = mixed_clause();
        clause.literals_mut().as_mut_slice()[0].set_prop(EP_IS_PM_INTO_LIT);
        assert!(!select_inherited_literal(&mut clause));
        assert_eq!(clause.prop_lit_number(EP_IS_SELECTED), 0);

        clause.literals_mut().as_mut_slice()[1].set_prop(EP_IS_PM_INTO_LIT);
        assert!(select_inherited_literal(&mut clause));
        assert_eq!(clause.prop_lit_number(EP_IS_SELECTED), 2);
    }

    #[test]
    fn do_literal_selection_clears_state_and_applies_selector_when_enabled() {
        let mut control = proof_control_alloc();
        let mut clause = mixed_clause();
        clause.set_prop(CP_IS_ORIENTED);
        clause.literals_mut().as_mut_slice()[0].set_prop(EP_IS_SELECTED);
        let mut selector_calls = 0;

        let outcome =
            do_literal_selection_with_selector(&mut control, &mut clause, |_ocb, clause| {
                selector_calls += 1;
                assert!(!clause.query_prop(CP_IS_ORIENTED));
                assert_eq!(clause.prop_lit_number(EP_IS_SELECTED), 0);
                clause.literals_mut().as_mut_slice()[1].set_prop(EP_IS_SELECTED);
            });

        assert_eq!(outcome, LiteralSelectionOutcome::SelectorApplied);
        assert_eq!(selector_calls, 1);
        assert_eq!(clause.prop_lit_number(EP_IS_SELECTED), 1);
    }

    #[test]
    fn do_literal_selection_inherited_path_bypasses_selector() {
        let mut control = proof_control_alloc();
        control.heuristic_parms_mut().inherit_paramod_lit = true;
        let mut clause = mixed_clause();
        clause.set_prop(CP_IS_ORIENTED);
        clause.literals_mut().as_mut_slice()[0].set_prop(EP_IS_PM_INTO_LIT);
        clause.literals_mut().as_mut_slice()[1].set_prop(EP_IS_PM_INTO_LIT);

        let outcome =
            do_literal_selection_with_selector(&mut control, &mut clause, |_ocb, _clause| {
                panic!("selector must not run after inherited selection");
            });

        assert_eq!(outcome, LiteralSelectionOutcome::Inherited);
        assert!(!clause.query_prop(CP_IS_ORIENTED));
        assert_eq!(clause.prop_lit_number(EP_IS_SELECTED), 2);
    }

    #[test]
    fn do_literal_selection_goal_and_conjecture_inheritance_follow_c_gate() {
        let mut goal_control = proof_control_alloc();
        goal_control.heuristic_parms_mut().inherit_goal_pm_lit = true;
        let mut goal = mixed_clause();
        goal.literals_mut().as_mut_slice()[1].set_prop(EP_IS_PM_INTO_LIT);
        assert_ne!(goal.negative_literal_count(), 0);
        goal.replace_literals(EqnList::from_vec(
            goal.literals()
                .as_slice()
                .iter()
                .filter(|literal| literal.is_negative())
                .cloned()
                .collect(),
        ));

        let outcome =
            do_literal_selection_with_selector(&mut goal_control, &mut goal, |_ocb, _clause| {
                panic!("goal inherited selection should bypass selector");
            });
        assert_eq!(outcome, LiteralSelectionOutcome::Inherited);

        let mut conjecture_control = proof_control_alloc();
        conjecture_control.heuristic_parms_mut().inherit_conj_pm_lit = true;
        let mut conjecture = mixed_clause();
        conjecture.set_tptp_type(CP_TYPE_CONJECTURE);
        conjecture.literals_mut().as_mut_slice()[1].set_prop(EP_IS_PM_INTO_LIT);

        let outcome = do_literal_selection_with_selector(
            &mut conjecture_control,
            &mut conjecture,
            |_ocb, _clause| {
                panic!("conjecture inherited selection should bypass selector");
            },
        );
        assert_eq!(outcome, LiteralSelectionOutcome::Inherited);
    }

    #[test]
    fn do_literal_selection_skips_selector_when_c_limits_fail() {
        let mut control = proof_control_alloc();
        let mut clause = positive_clause();
        clause.set_prop(CP_IS_ORIENTED);
        clause.literals_mut().as_mut_slice()[0].set_prop(EP_IS_SELECTED);

        let outcome =
            do_literal_selection_with_selector(&mut control, &mut clause, |_ocb, _clause| {
                panic!("selector must not run for clauses without negative literals");
            });

        assert_eq!(outcome, LiteralSelectionOutcome::SelectionSkipped);
        assert!(!clause.query_prop(CP_IS_ORIENTED));
        assert_eq!(clause.prop_lit_number(EP_IS_SELECTED), 0);
    }

    #[test]
    fn do_literal_selection_named_noop_selectors_are_available() {
        let mut control = proof_control_alloc();
        let mut clause = mixed_clause();

        let outcome = do_literal_selection(&mut control, &mut clause).unwrap_or_else(|err| {
            panic!("{err}");
        });

        assert_eq!(outcome, LiteralSelectionOutcome::SelectorApplied);
        assert_eq!(clause.prop_lit_number(EP_IS_SELECTED), 0);

        control.heuristic_parms_mut().selection_strategy = "NoGeneration".to_owned();
        let outcome = do_literal_selection(&mut control, &mut clause).unwrap_or_else(|err| {
            panic!("{err}");
        });

        assert_eq!(outcome, LiteralSelectionOutcome::SelectorApplied);
        assert_eq!(clause.prop_lit_number(EP_IS_SELECTED), 0);
    }

    #[test]
    fn do_literal_selection_reports_unported_strategy_only_if_reached() {
        let mut control = proof_control_alloc();
        control.heuristic_parms_mut().selection_strategy = "SelectUnlessUniqMax".to_owned();

        let mut positive = positive_clause();
        let outcome = do_literal_selection(&mut control, &mut positive).unwrap_or_else(|err| {
            panic!("{err}");
        });
        assert_eq!(outcome, LiteralSelectionOutcome::SelectionSkipped);

        let mut mixed = mixed_clause();
        let error = do_literal_selection(&mut control, &mut mixed).unwrap_err();
        assert_eq!(error.strategy(), "SelectUnlessUniqMax");
    }

    #[test]
    fn do_literal_selection_with_bank_applies_ordering_dependent_selector() {
        let mut bank = test_bank();
        let mut control = proof_control_alloc();
        control.heuristic_parms_mut().selection_strategy = SELECT_UNLESS_POS_MAX.to_owned();
        let mut clause = negative_clause(&mut bank);
        control.set_ocb(kbo_ocb(&bank));

        let outcome = do_literal_selection_with_bank(&mut control, &bank, &mut clause)
            .unwrap_or_else(|err| {
                panic!("{err}");
            });

        assert_eq!(outcome, LiteralSelectionOutcome::SelectorApplied);
        assert_eq!(clause.prop_lit_number(EP_IS_SELECTED), 1);
    }
}
