use crate::basics::defines::DEFAULT_COMCHAR_RAW;
use crate::basics::error::{Diagnostic, ErrorCode};
use crate::basics::simple_stuff::ProblemType;
use crate::clauses::clause::{clause_answer_output_string, Clause};
use crate::clauses::clause_props::{
    CP_IS_DEAD, CP_IS_PROOF_CLAUSE, CP_TYPE_CONJECTURE, CP_TYPE_NEG_CONJECTURE,
    CP_TYPE_WATCH_CLAUSE, CP_WATCH_ONLY,
};
use crate::clauses::clausefunc::tformula_expand_distinct;
use crate::clauses::clausesets::ClauseSet;
use crate::clauses::derivation::{
    clause_dummy_quote_parent_ref, clause_is_dummy_quote, clause_is_eval_gc,
    demodulator_clause_refs, deriv_stack_count_search_inferences, deriv_stack_extract_parents,
    deriv_stack_indicates_initial_clause, op_has_arg1, op_has_arg2, op_has_cnf_arg1,
    op_has_cnf_arg2, op_has_fof_arg1, op_has_fof_arg2, ClauseDerivationRef, DerivationEntry,
    DerivationParentRef, FormulaDerivationRef, DC_AC_RES, DC_CNF_QUOTE, DC_EXPAND_DISTINCT,
};
use crate::clauses::fcvindexing::{
    fvi_param_init_anchors, fvi_param_init_specs, FvIndexInitTargetSets, FvIndexParams,
};
use crate::clauses::formulasets::{wformula_deriv_find_first, FormulaSet, WrappedFormula};
use crate::clauses::freqvectors::FvCollect;
use crate::clauses::global_indices::GlobalIndices;
use crate::clauses::rewrite::REWRITE_UNCACHED;
use crate::inout::scanner::{IoFormat, Scanner, TokenType};
use crate::orderings::ocb::OrderControlBlock;
use crate::terms::functypes::FunCode;
use crate::terms::garbage_coll::GcSetHandle;
use crate::terms::signature::{FunctionProperties, Signature};
use crate::terms::termbanks::TermBank;
use crate::terms::termvars::VarBank;
use crate::terms::typebanks::TypeBank;
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
    path::Path,
    sync::atomic::Ordering,
};

pub const WATCHLIST_INLINE_STRING: &str = "Use inline watchlist type";
pub const WATCHLIST_INLINE_QSTRING: &str = "'Use inline watchlist type'";

const GC_AXIOMS: GcSetHandle = GcSetHandle::new(1);
const GC_AX_ARCHIVE: GcSetHandle = GcSetHandle::new(2);
const GC_PROCESSED_POS_RULES: GcSetHandle = GcSetHandle::new(3);
const GC_PROCESSED_POS_EQNS: GcSetHandle = GcSetHandle::new(4);
const GC_PROCESSED_NEG_UNITS: GcSetHandle = GcSetHandle::new(5);
const GC_PROCESSED_NON_UNITS: GcSetHandle = GcSetHandle::new(6);
const GC_UNPROCESSED: GcSetHandle = GcSetHandle::new(7);
const GC_TMP_STORE: GcSetHandle = GcSetHandle::new(8);
const GC_EVAL_STORE: GcSetHandle = GcSetHandle::new(9);
const GC_ARCHIVE: GcSetHandle = GcSetHandle::new(10);
const GC_WATCHLIST: GcSetHandle = GcSetHandle::new(11);
const GC_DEFINITION_STORE: GcSetHandle = GcSetHandle::new(12);

const GC_F_AXIOMS: GcSetHandle = GcSetHandle::new(101);
const GC_F_AX_ARCHIVE: GcSetHandle = GcSetHandle::new(102);
const GC_DEFINITION_FORMULA_ARCHIVE: GcSetHandle = GcSetHandle::new(103);
const GC_F_ARCHIVE: GcSetHandle = GcSetHandle::new(104);

const PROOF_STATE_CLAUSE_GC_ROOTS: [GcSetHandle; 12] = [
    GC_AXIOMS,
    GC_AX_ARCHIVE,
    GC_PROCESSED_POS_RULES,
    GC_PROCESSED_POS_EQNS,
    GC_PROCESSED_NEG_UNITS,
    GC_PROCESSED_NON_UNITS,
    GC_UNPROCESSED,
    GC_TMP_STORE,
    GC_EVAL_STORE,
    GC_ARCHIVE,
    GC_WATCHLIST,
    GC_DEFINITION_STORE,
];

const PROOF_STATE_FORMULA_GC_ROOTS: [GcSetHandle; 4] = [
    GC_F_AXIOMS,
    GC_F_AX_ARCHIVE,
    GC_DEFINITION_FORMULA_ARCHIVE,
    GC_F_ARCHIVE,
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WatchlistSource<'a> {
    Disabled,
    Inline,
    File(&'a Path),
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ProofStateStatistics {
    pub status_reported: bool,
    pub answer_count: i64,
    pub processed_count: u64,
    pub proc_trivial_count: u64,
    pub proc_forward_subsumed_count: u64,
    pub proc_non_trivial_count: u64,
    pub other_redundant_count: u64,
    pub non_redundant_deleted: u64,
    pub backward_subsumed_count: u64,
    pub backward_rewritten_count: u64,
    pub backward_rewritten_lit_count: u64,
    pub rw_count: u64,
    pub generated_count: u64,
    pub aggressive_forward_subsumed_count: u64,
    pub generated_lit_count: u64,
    pub non_trivial_generated_count: u64,
    pub context_sr_count: u64,
    pub paramod_count: u64,
    pub factor_count: u64,
    pub neg_ext_count: u64,
    pub resolv_count: u64,
    pub disequ_deco_count: u64,
    pub satcheck_count: u64,
    pub satcheck_success: u64,
    pub satcheck_satisfiable: u64,
    pub satcheck_full_size: u64,
    pub satcheck_actual_size: u64,
    pub satcheck_core_size: u64,
    pub satcheck_preproc_time: f64,
    pub satcheck_encoding_time: f64,
    pub satcheck_solver_time: f64,
    pub satcheck_preproc_stime: f64,
    pub satcheck_encoding_stime: f64,
    pub satcheck_solver_stime: f64,
    pub filter_orphans_base: u64,
    pub forward_contract_base: u64,
    pub gc_count: u64,
    pub gc_used_count: u64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RawFormulaFeatures {
    pub has_formula_input: bool,
    pub sentence_no: i64,
    pub term_size: i64,
    pub lowered_clause_no: i64,
    pub lowered_clause_term_size: i64,
    pub conjecture_count: i64,
    pub hypothesis_count: i64,
    pub lowered_conjecture_count: i64,
    pub lowered_hypothesis_count: i64,
    pub order: i32,
    pub conj_order: i32,
    pub num_lambdas: i32,
    pub app_var_lits: bool,
}

impl RawFormulaFeatures {
    pub fn add(&mut self, other: Self) {
        self.has_formula_input |= other.has_formula_input;
        self.sentence_no += other.sentence_no;
        self.term_size += other.term_size;
        self.lowered_clause_no += other.lowered_clause_no;
        self.lowered_clause_term_size += other.lowered_clause_term_size;
        self.conjecture_count += other.conjecture_count;
        self.hypothesis_count += other.hypothesis_count;
        self.lowered_conjecture_count += other.lowered_conjecture_count;
        self.lowered_hypothesis_count += other.lowered_hypothesis_count;
        self.order = self.order.max(other.order);
        self.conj_order = self.conj_order.max(other.conj_order);
        self.num_lambdas += other.num_lambdas;
        self.app_var_lits |= other.app_var_lits;
    }
}

#[derive(Debug)]
pub struct ProofStateProcessedSets<'a> {
    pub pos_rules: &'a mut ClauseSet,
    pub pos_eqns: &'a mut ClauseSet,
    pub neg_units: &'a mut ClauseSet,
    pub non_units: &'a mut ClauseSet,
}

#[derive(Debug)]
pub struct ProofStateGenerationContext<'a> {
    pub fresh_vars: &'a VarBank,
    pub processed_pos_rules: &'a ClauseSet,
    pub processed_pos_eqns: &'a ClauseSet,
    pub processed_neg_units: &'a ClauseSet,
    pub processed_non_units: &'a ClauseSet,
    pub tmp_store: &'a mut ClauseSet,
    pub archive: &'a mut ClauseSet,
    pub choice_opcodes: &'a mut BTreeMap<FunCode, Clause>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ProofStateGcAnalysis {
    pub clause_count: u64,
    pub given_count: u64,
    pub used_given_count: u64,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ProofStateProcessDistinctResult {
    pub distinct_formulas_processed: i64,
    pub expanded_formula_sources: Vec<FormulaDerivationRef>,
    pub formula_derivation_ops: Vec<i64>,
}

#[derive(Clone, Copy, Debug)]
pub enum DerivedView<'a> {
    Clause(&'a Clause),
    Formula(&'a WrappedFormula),
}

pub enum DerivedViewMut<'a> {
    Clause(&'a mut Clause),
    Formula(&'a mut WrappedFormula),
}

#[must_use]
pub fn derived_in_proof(derived: DerivedView<'_>) -> bool {
    match derived {
        DerivedView::Clause(clause) => clause.is_empty() || clause.query_prop(CP_IS_PROOF_CLAUSE),
        DerivedView::Formula(formula) => formula.query_prop(CP_IS_PROOF_CLAUSE),
    }
}

pub fn derived_set_in_proof(derived: DerivedViewMut<'_>, in_proof: bool) {
    match derived {
        DerivedViewMut::Clause(clause) => {
            if in_proof {
                clause.set_prop(CP_IS_PROOF_CLAUSE);
            } else {
                clause.del_prop(CP_IS_PROOF_CLAUSE);
            }
        }
        DerivedViewMut::Formula(formula) => {
            if in_proof {
                formula.set_prop(CP_IS_PROOF_CLAUSE);
            } else {
                formula.del_prop(CP_IS_PROOF_CLAUSE);
            }
        }
    }
}

#[must_use]
pub fn derived_is_eval_gc(derived: DerivedView<'_>) -> bool {
    match derived {
        DerivedView::Clause(clause) => clause_is_eval_gc(clause),
        DerivedView::Formula(_) => false,
    }
}

const DOT_NODE_GRAY: &str = ",color=gray, fillcolor=gray";
const DOT_NODE_AX_GRAY: &str = ",color=gray, fillcolor=gray66";
const DOT_NODE_GREEN: &str = ",color=green,fillcolor=palegreen";
const DOT_NODE_AX_GREEN: &str = ",color=green,fillcolor=forestgreen";
const DOT_NODE_RED: &str = ",color=red,fillcolor=lightpink1";
const DOT_NODE_AX_RED: &str = ",color=red,fillcolor=firebrick1";
const DOT_NODE_PURPLE: &str = ",color=blue,fillcolor=darkorchid1";
const DOT_NODE_BLUE: &str = ",color=blue,fillcolor=lightskyblue1";
const DOT_NODE_AX_BLUE: &str = ",color=blue,fillcolor=dodgerblue";

#[must_use]
pub fn derived_dot_node_colour(derived: DerivedView<'_>) -> &'static str {
    derived_dot_node_colour_with_proof_status(derived, derived_in_proof(derived))
}

#[must_use]
pub fn derived_dot_node_colour_for_proof_member(derived: DerivedView<'_>) -> &'static str {
    derived_dot_node_colour_with_proof_status(derived, true)
}

#[must_use]
fn derived_dot_node_colour_with_proof_status(
    derived: DerivedView<'_>,
    in_proof: bool,
) -> &'static str {
    if !in_proof {
        return if derived_has_derivation(derived) {
            DOT_NODE_GRAY
        } else {
            DOT_NODE_AX_GRAY
        };
    }

    match derived {
        DerivedView::Clause(clause) => {
            if clause.is_empty() {
                DOT_NODE_PURPLE
            } else if matches!(
                clause.query_tptp_type(),
                CP_TYPE_CONJECTURE | CP_TYPE_NEG_CONJECTURE
            ) {
                if derived_has_derivation(derived) {
                    DOT_NODE_BLUE
                } else {
                    DOT_NODE_AX_BLUE
                }
            } else if derived_has_derivation(derived) {
                DOT_NODE_GREEN
            } else {
                DOT_NODE_AX_GREEN
            }
        }
        DerivedView::Formula(formula) => match formula.query_tptp_type() {
            CP_TYPE_CONJECTURE => {
                if derived_has_derivation(derived) {
                    DOT_NODE_RED
                } else {
                    DOT_NODE_AX_RED
                }
            }
            CP_TYPE_NEG_CONJECTURE => {
                if derived_has_derivation(derived) {
                    DOT_NODE_BLUE
                } else {
                    DOT_NODE_AX_BLUE
                }
            }
            _ => {
                if derived_has_derivation(derived) {
                    DOT_NODE_GREEN
                } else {
                    DOT_NODE_AX_GREEN
                }
            }
        },
    }
}

#[must_use]
pub fn derived_dot_clause_link_colour(child: DerivedView<'_>, parent: &Clause) -> &'static str {
    if !parent.query_prop(CP_IS_PROOF_CLAUSE) || !derived_in_proof(child) {
        DOT_NODE_GRAY
    } else {
        derived_dot_node_colour(child)
    }
}

#[must_use]
pub fn derived_dot_formula_link_colour(
    child: DerivedView<'_>,
    parent: &WrappedFormula,
) -> &'static str {
    if !parent.query_prop(CP_IS_PROOF_CLAUSE) || !derived_in_proof(child) {
        DOT_NODE_GRAY
    } else {
        derived_dot_node_colour(child)
    }
}

#[must_use]
fn derived_has_derivation(derived: DerivedView<'_>) -> bool {
    match derived {
        DerivedView::Clause(clause) => clause.derivation().is_some(),
        DerivedView::Formula(formula) => formula.derivation().is_some(),
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ProofObjectAnalysis {
    pub clause_step_count: u64,
    pub formula_step_count: u64,
    pub clause_conjecture_count: u64,
    pub formula_conjecture_count: u64,
    pub initial_clause_count: u64,
    pub initial_formula_count: u64,
    pub generating_inference_count: u64,
    pub simplifying_inference_count: u64,
}

impl ProofObjectAnalysis {
    #[must_use]
    pub const fn total_step_count(self) -> u64 {
        self.clause_step_count + self.formula_step_count
    }

    #[must_use]
    pub const fn conjecture_count(self) -> u64 {
        self.clause_conjecture_count + self.formula_conjecture_count
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProofObjectGraphEdge {
    pub parent_index: usize,
    pub child_index: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProofObjectGraphNode {
    Clause(usize),
    Formula(usize),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProofObjectGraphMixedEdge {
    pub parent: ProofObjectGraphNode,
    pub child: ProofObjectGraphNode,
}

#[derive(Debug, Default, PartialEq)]
pub struct ProofObjectGraph<'a> {
    pub clauses: Vec<&'a Clause>,
    pub formulas: Vec<&'a WrappedFormula>,
    pub clause_aliases: BTreeMap<ClauseDerivationRef, usize>,
    pub edges: Vec<ProofObjectGraphEdge>,
    pub mixed_edges: Vec<ProofObjectGraphMixedEdge>,
    pub root_indices: Vec<usize>,
    pub formula_root_indices: Vec<usize>,
}

#[derive(Debug, Default, PartialEq)]
pub struct ProofStateTrainingExamples<'a> {
    pub positive: Vec<&'a Clause>,
    pub negative: Vec<&'a Clause>,
}

#[derive(Clone, Copy)]
enum ProofObjectParentResolution {
    ProofStep,
    QuoteSource,
    AcAxiom,
}

#[derive(Clone, Copy)]
struct ProofObjectParentEdge {
    parent: DerivationParentRef,
    resolution: ProofObjectParentResolution,
}

#[derive(Clone, Debug)]
pub struct ProofState {
    terms: TermBank,
    tmp_terms: TermBank,
    fresh_vars: VarBank,
    original_symbols: usize,
    axioms: ClauseSet,
    f_axioms: FormulaSet,
    ax_archive: ClauseSet,
    f_ax_archive: FormulaSet,
    processed_pos_rules: ClauseSet,
    processed_pos_eqns: ClauseSet,
    processed_neg_units: ClauseSet,
    processed_non_units: ClauseSet,
    unprocessed: ClauseSet,
    tmp_store: ClauseSet,
    eval_store: ClauseSet,
    archive: ClauseSet,
    f_archive: FormulaSet,
    choice_opcodes: BTreeMap<FunCode, Clause>,
    extract_roots: Vec<Clause>,
    extract_formula_roots: Vec<WrappedFormula>,
    watchlist: Option<ClauseSet>,
    watchlist_activation: WatchlistActivation,
    definition_store: ClauseSet,
    definition_formula_archive: FormulaSet,
    definition_assocs: BTreeMap<i64, FunCode>,
    definition_formula_assocs: BTreeMap<i64, FormulaDerivationRef>,
    global_indices: GlobalIndices,
    watchlist_indices: GlobalIndices,
    fvi_initialized: bool,
    fvi_cspec: Option<FvCollect>,
    def_store_cspec: Option<FvCollect>,
    state_is_complete: bool,
    has_interpreted_symbols: bool,
    raw_formula_features: RawFormulaFeatures,
    statistics: ProofStateStatistics,
    answer_outputs: Vec<String>,
}

pub type ProofStateDefinitionStoreMut<'a> = (
    &'a mut TermBank,
    &'a mut ClauseSet,
    &'a mut BTreeMap<i64, FunCode>,
    &'a mut BTreeMap<i64, FormulaDerivationRef>,
    &'a mut FormulaSet,
);

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum WatchlistActivation {
    #[default]
    Inactive,
    Active,
}

fn register_proof_state_gc_roots(terms: &mut TermBank) {
    for root in PROOF_STATE_CLAUSE_GC_ROOTS {
        terms.register_gc_clause_set(root);
    }
    for root in PROOF_STATE_FORMULA_GC_ROOTS {
        terms.register_gc_formula_set(root);
    }
}

fn gc_mark_clause_set_terms(set: &ClauseSet, terms: &TermBank) {
    for clause in set.iter() {
        clause.gc_mark_terms(terms);
    }
}

impl ProofState {
    /// Allocates the currently ported proof-state owner fields.
    ///
    /// The clause-set, global-index, demodulator-index, formula-set, FV-index,
    /// distinct-symbol, and statistic initialization mirrors C
    /// `ProofStateAlloc`. SAT integration is added by later slices.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if internal signature or term-bank setup fails.
    pub fn new(free_symbol_props: FunctionProperties) -> Result<Self, Diagnostic> {
        let mut signature = Signature::new(TypeBank::new());
        signature.insert_internal_codes()?;
        signature.remove_distinct_props(free_symbol_props);
        let mut terms = TermBank::new(signature)?;
        let tmp_terms = TermBank::new(terms.signature().clone())?;
        let fresh_vars = VarBank::new(terms.signature().type_bank());
        terms.vars().pair_shadow(&fresh_vars);
        register_proof_state_gc_roots(&mut terms);

        Ok(Self {
            terms,
            tmp_terms,
            fresh_vars,
            original_symbols: 0,
            axioms: ClauseSet::new(),
            f_axioms: FormulaSet::new(),
            ax_archive: ClauseSet::new(),
            f_ax_archive: FormulaSet::new(),
            processed_pos_rules: ClauseSet::new_demod_indexed(),
            processed_pos_eqns: ClauseSet::new_demod_indexed(),
            processed_neg_units: ClauseSet::new_demod_indexed(),
            processed_non_units: ClauseSet::new_position_indexed(),
            unprocessed: ClauseSet::new(),
            tmp_store: ClauseSet::new(),
            eval_store: ClauseSet::new(),
            archive: ClauseSet::new(),
            f_archive: FormulaSet::new(),
            choice_opcodes: BTreeMap::new(),
            extract_roots: Vec::new(),
            extract_formula_roots: Vec::new(),
            watchlist: Some(ClauseSet::new()),
            watchlist_activation: WatchlistActivation::Inactive,
            definition_store: ClauseSet::new(),
            definition_formula_archive: FormulaSet::new(),
            definition_assocs: BTreeMap::new(),
            definition_formula_assocs: BTreeMap::new(),
            global_indices: GlobalIndices::null(),
            watchlist_indices: GlobalIndices::null(),
            fvi_initialized: false,
            fvi_cspec: None,
            def_store_cspec: None,
            state_is_complete: true,
            has_interpreted_symbols: false,
            raw_formula_features: RawFormulaFeatures::default(),
            statistics: ProofStateStatistics::default(),
            answer_outputs: Vec::new(),
        })
    }

    #[must_use]
    pub const fn terms(&self) -> &TermBank {
        &self.terms
    }

    pub fn terms_mut(&mut self) -> &mut TermBank {
        &mut self.terms
    }

    #[must_use]
    pub const fn tmp_terms(&self) -> &TermBank {
        &self.tmp_terms
    }

    pub fn tmp_terms_mut(&mut self) -> &mut TermBank {
        let main_signature = self.terms.signature();
        let tmp_signature = self.tmp_terms.signature();
        if main_signature.f_count() != tmp_signature.f_count()
            || main_signature.type_bank().types_count() != tmp_signature.type_bank().types_count()
        {
            *self.tmp_terms.signature_mut() = main_signature.clone();
        }
        &mut self.tmp_terms
    }

    pub fn replace_tmp_terms(&mut self, tmp_terms: TermBank) {
        self.tmp_terms = tmp_terms;
    }

    #[must_use]
    pub const fn global_indices(&self) -> &GlobalIndices {
        &self.global_indices
    }

    pub fn global_indices_mut(&mut self) -> &mut GlobalIndices {
        &mut self.global_indices
    }

    #[must_use]
    pub const fn watchlist_indices(&self) -> &GlobalIndices {
        &self.watchlist_indices
    }

    pub fn watchlist_indices_mut(&mut self) -> &mut GlobalIndices {
        &mut self.watchlist_indices
    }

    pub(crate) fn with_global_indices<R>(
        &mut self,
        operation: impl FnOnce(&mut Self, &mut GlobalIndices) -> R,
    ) -> R {
        let mut indices = std::mem::take(&mut self.global_indices);
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            operation(self, &mut indices)
        }));
        self.global_indices = indices;
        match result {
            Ok(result) => result,
            Err(payload) => std::panic::resume_unwind(payload),
        }
    }

    pub(crate) fn with_watchlist_indices<R>(
        &mut self,
        operation: impl FnOnce(&mut Self, &mut GlobalIndices) -> R,
    ) -> R {
        let mut indices = std::mem::take(&mut self.watchlist_indices);
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            operation(self, &mut indices)
        }));
        self.watchlist_indices = indices;
        match result {
            Ok(result) => result,
            Err(payload) => std::panic::resume_unwind(payload),
        }
    }

    pub(crate) fn with_global_and_watchlist_indices<R>(
        &mut self,
        operation: impl FnOnce(&mut Self, &mut GlobalIndices, &mut GlobalIndices) -> R,
    ) -> R {
        let mut indices = std::mem::take(&mut self.global_indices);
        let mut watchlist_indices = std::mem::take(&mut self.watchlist_indices);
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            operation(self, &mut indices, &mut watchlist_indices)
        }));
        self.global_indices = indices;
        self.watchlist_indices = watchlist_indices;
        match result {
            Ok(result) => result,
            Err(payload) => std::panic::resume_unwind(payload),
        }
    }

    #[must_use]
    pub const fn fresh_vars(&self) -> &VarBank {
        &self.fresh_vars
    }

    pub fn terms_and_axioms_mut(&mut self) -> (&mut TermBank, &mut ClauseSet) {
        let Self { terms, axioms, .. } = self;
        (terms, axioms)
    }

    pub fn terms_axioms_f_axioms_mut(&mut self) -> (&mut TermBank, &mut ClauseSet, &FormulaSet) {
        let Self {
            terms,
            axioms,
            f_axioms,
            ..
        } = self;
        (terms, axioms, f_axioms)
    }

    pub fn terms_axioms_ax_archive_mut(
        &mut self,
    ) -> (&mut TermBank, &mut ClauseSet, &mut ClauseSet) {
        let Self {
            terms,
            axioms,
            ax_archive,
            ..
        } = self;
        (terms, axioms, ax_archive)
    }

    pub fn terms_and_watchlist_mut(&mut self) -> (&mut TermBank, Option<&mut ClauseSet>) {
        let Self {
            terms, watchlist, ..
        } = self;
        (terms, watchlist.as_mut())
    }

    pub fn terms_and_unprocessed_mut(&mut self) -> (&mut TermBank, &mut ClauseSet) {
        let Self {
            terms, unprocessed, ..
        } = self;
        (terms, unprocessed)
    }

    pub fn terms_and_archive_mut(&mut self) -> (&mut TermBank, &mut ClauseSet) {
        let Self { terms, archive, .. } = self;
        (terms, archive)
    }

    pub fn terms_axioms_archive_mut(&mut self) -> (&mut TermBank, &mut ClauseSet, &mut ClauseSet) {
        let Self {
            terms,
            axioms,
            archive,
            ..
        } = self;
        (terms, axioms, archive)
    }

    pub fn terms_axioms_watchlist_archive_mut(
        &mut self,
    ) -> (
        &mut TermBank,
        &mut ClauseSet,
        Option<&mut ClauseSet>,
        &mut ClauseSet,
    ) {
        let Self {
            terms,
            axioms,
            watchlist,
            archive,
            ..
        } = self;
        (terms, axioms, watchlist.as_mut(), archive)
    }

    pub fn terms_axioms_choice_opcodes_mut(
        &mut self,
    ) -> (
        &mut TermBank,
        &mut ClauseSet,
        &mut BTreeMap<FunCode, Clause>,
    ) {
        let Self {
            terms,
            axioms,
            choice_opcodes,
            ..
        } = self;
        (terms, axioms, choice_opcodes)
    }

    pub fn terms_axioms_formula_sets_mut(
        &mut self,
    ) -> (&mut TermBank, &ClauseSet, &FormulaSet, &FormulaSet) {
        let Self {
            terms,
            axioms,
            f_axioms,
            f_ax_archive,
            ..
        } = self;
        (terms, axioms, f_axioms, f_ax_archive)
    }

    pub fn terms_axioms_formula_sets_cnf_mut(
        &mut self,
    ) -> (
        &mut TermBank,
        &mut ClauseSet,
        &mut FormulaSet,
        &mut FormulaSet,
    ) {
        let Self {
            terms,
            axioms,
            f_axioms,
            f_ax_archive,
            ..
        } = self;
        (terms, axioms, f_axioms, f_ax_archive)
    }

    pub fn terms_f_axioms_watchlist_mut(
        &mut self,
    ) -> (&mut TermBank, &mut FormulaSet, Option<&mut ClauseSet>) {
        let Self {
            terms,
            f_axioms,
            watchlist,
            ..
        } = self;
        (terms, f_axioms, watchlist.as_mut())
    }

    pub fn terms_and_processed_sets_mut(&mut self) -> (&mut TermBank, ProofStateProcessedSets<'_>) {
        let Self {
            terms,
            processed_pos_rules,
            processed_pos_eqns,
            processed_neg_units,
            processed_non_units,
            ..
        } = self;
        (
            terms,
            ProofStateProcessedSets {
                pos_rules: processed_pos_rules,
                pos_eqns: processed_pos_eqns,
                neg_units: processed_neg_units,
                non_units: processed_non_units,
            },
        )
    }

    pub fn terms_and_generation_context_mut(
        &mut self,
    ) -> (&mut TermBank, ProofStateGenerationContext<'_>) {
        let Self {
            terms,
            processed_pos_rules,
            processed_pos_eqns,
            processed_neg_units,
            processed_non_units,
            tmp_store,
            archive,
            choice_opcodes,
            fresh_vars,
            ..
        } = self;
        (
            terms,
            ProofStateGenerationContext {
                fresh_vars,
                processed_pos_rules,
                processed_pos_eqns,
                processed_neg_units,
                processed_non_units,
                tmp_store,
                archive,
                choice_opcodes,
            },
        )
    }

    #[must_use]
    pub const fn original_symbols(&self) -> usize {
        self.original_symbols
    }

    #[must_use]
    pub const fn axioms(&self) -> &ClauseSet {
        &self.axioms
    }

    pub fn axioms_mut(&mut self) -> &mut ClauseSet {
        &mut self.axioms
    }

    #[must_use]
    pub const fn f_axioms(&self) -> &FormulaSet {
        &self.f_axioms
    }

    pub fn f_axioms_mut(&mut self) -> &mut FormulaSet {
        &mut self.f_axioms
    }

    #[must_use]
    pub const fn ax_archive(&self) -> &ClauseSet {
        &self.ax_archive
    }

    pub fn ax_archive_mut(&mut self) -> &mut ClauseSet {
        &mut self.ax_archive
    }

    #[must_use]
    pub const fn f_ax_archive(&self) -> &FormulaSet {
        &self.f_ax_archive
    }

    pub fn f_ax_archive_mut(&mut self) -> &mut FormulaSet {
        &mut self.f_ax_archive
    }

    #[must_use]
    pub const fn processed_pos_rules(&self) -> &ClauseSet {
        &self.processed_pos_rules
    }

    pub fn processed_pos_rules_mut(&mut self) -> &mut ClauseSet {
        &mut self.processed_pos_rules
    }

    #[must_use]
    pub const fn processed_pos_eqns(&self) -> &ClauseSet {
        &self.processed_pos_eqns
    }

    pub fn processed_pos_eqns_mut(&mut self) -> &mut ClauseSet {
        &mut self.processed_pos_eqns
    }

    #[must_use]
    pub const fn processed_neg_units(&self) -> &ClauseSet {
        &self.processed_neg_units
    }

    pub fn processed_neg_units_mut(&mut self) -> &mut ClauseSet {
        &mut self.processed_neg_units
    }

    #[must_use]
    pub const fn processed_non_units(&self) -> &ClauseSet {
        &self.processed_non_units
    }

    pub fn processed_non_units_mut(&mut self) -> &mut ClauseSet {
        &mut self.processed_non_units
    }

    #[must_use]
    pub const fn unprocessed(&self) -> &ClauseSet {
        &self.unprocessed
    }

    pub fn unprocessed_mut(&mut self) -> &mut ClauseSet {
        &mut self.unprocessed
    }

    #[must_use]
    pub const fn tmp_store(&self) -> &ClauseSet {
        &self.tmp_store
    }

    pub fn tmp_store_mut(&mut self) -> &mut ClauseSet {
        &mut self.tmp_store
    }

    #[must_use]
    pub const fn eval_store(&self) -> &ClauseSet {
        &self.eval_store
    }

    pub fn eval_store_mut(&mut self) -> &mut ClauseSet {
        &mut self.eval_store
    }

    #[must_use]
    pub const fn archive(&self) -> &ClauseSet {
        &self.archive
    }

    pub fn archive_mut(&mut self) -> &mut ClauseSet {
        &mut self.archive
    }

    #[must_use]
    pub const fn f_archive(&self) -> &FormulaSet {
        &self.f_archive
    }

    pub fn f_archive_mut(&mut self) -> &mut FormulaSet {
        &mut self.f_archive
    }

    /// Processes `$distinct(...)` formula axioms like C
    /// `ProofStateProcessDistinct`.
    ///
    /// Matching formulas are discovered in current `f_axioms` order and then
    /// processed in stack-pop order, so later matching formulas are expanded
    /// first. The original wrappers move to `f_ax_archive`, and fresh expanded
    /// disequality wrappers with `DCExpandDistinct` derivations are appended to
    /// `f_axioms`.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if expanding a `$distinct` formula fails.
    ///
    /// # Panics
    ///
    /// Panics if a matching wrapper has no formula term.
    pub fn process_distinct(&mut self) -> Result<ProofStateProcessDistinctResult, Diagnostic> {
        let distinct_code = self.terms.signature().distinct_code();
        let mut pending = self
            .f_axioms
            .iter()
            .filter(|formula| formula.formula().f_code() == distinct_code)
            .map(WrappedFormula::entry_id)
            .collect::<Vec<_>>();
        let mut result = ProofStateProcessDistinctResult::default();

        while let Some(entry_id) = pending.pop() {
            let Some(distinct) = self.f_axioms.extract_entry(entry_id) else {
                continue;
            };
            let source = distinct.derivation_ref();
            let diseq_form = tformula_expand_distinct(&mut self.terms, distinct.formula())?;
            self.f_ax_archive.insert(distinct);
            let mut expanded = WrappedFormula::wt_formula_alloc(diseq_form);
            expanded.push_formula_derivation(DC_EXPAND_DISTINCT, Some(source), None);
            self.f_axioms.insert(expanded);
            result.distinct_formulas_processed += 1;
            result.expanded_formula_sources.push(source);
            result.formula_derivation_ops.push(DC_EXPAND_DISTINCT);
        }

        Ok(result)
    }

    #[must_use]
    pub const fn choice_opcodes(&self) -> &BTreeMap<FunCode, Clause> {
        &self.choice_opcodes
    }

    pub fn choice_opcodes_mut(&mut self) -> &mut BTreeMap<FunCode, Clause> {
        &mut self.choice_opcodes
    }

    #[must_use]
    pub fn extract_roots(&self) -> &[Clause] {
        &self.extract_roots
    }

    pub fn push_extract_root(&mut self, clause: Clause) {
        self.extract_roots.push(clause);
    }

    #[must_use]
    pub fn extract_formula_roots(&self) -> &[WrappedFormula] {
        &self.extract_formula_roots
    }

    pub fn push_extract_formula_root(&mut self, formula: WrappedFormula) {
        self.extract_formula_roots.push(formula);
    }

    pub fn terms_watchlist_archive_mut(
        &mut self,
    ) -> (&mut TermBank, Option<&mut ClauseSet>, &mut ClauseSet) {
        let Self {
            terms,
            watchlist,
            archive,
            ..
        } = self;
        (terms, watchlist.as_mut(), archive)
    }

    #[must_use]
    pub const fn watchlist(&self) -> Option<&ClauseSet> {
        self.watchlist.as_ref()
    }

    #[must_use]
    pub const fn watchlist_active(&self) -> bool {
        matches!(self.watchlist_activation, WatchlistActivation::Active)
    }

    pub fn watchlist_mut(&mut self) -> Option<&mut ClauseSet> {
        self.watchlist.as_mut()
    }

    pub fn discard_watchlist(&mut self) -> Option<ClauseSet> {
        self.watchlist_activation = WatchlistActivation::Inactive;
        self.terms.deregister_gc_clause_set(GC_WATCHLIST);
        self.watchlist.take()
    }

    #[must_use]
    pub const fn definition_store(&self) -> &ClauseSet {
        &self.definition_store
    }

    pub fn definition_store_mut(&mut self) -> &mut ClauseSet {
        &mut self.definition_store
    }

    #[must_use]
    pub const fn definition_formula_archive(&self) -> &FormulaSet {
        &self.definition_formula_archive
    }

    pub fn definition_formula_archive_mut(&mut self) -> &mut FormulaSet {
        &mut self.definition_formula_archive
    }

    #[must_use]
    pub const fn definition_assocs(&self) -> &BTreeMap<i64, FunCode> {
        &self.definition_assocs
    }

    pub fn definition_assocs_mut(&mut self) -> &mut BTreeMap<i64, FunCode> {
        &mut self.definition_assocs
    }

    #[must_use]
    pub const fn definition_formula_assocs(&self) -> &BTreeMap<i64, FormulaDerivationRef> {
        &self.definition_formula_assocs
    }

    pub fn definition_formula_assocs_mut(&mut self) -> &mut BTreeMap<i64, FormulaDerivationRef> {
        &mut self.definition_formula_assocs
    }

    pub fn terms_and_definition_store_mut(&mut self) -> ProofStateDefinitionStoreMut<'_> {
        let Self {
            terms,
            definition_store,
            definition_formula_archive,
            definition_assocs,
            definition_formula_assocs,
            ..
        } = self;
        (
            terms,
            definition_store,
            definition_assocs,
            definition_formula_assocs,
            definition_formula_archive,
        )
    }

    #[must_use]
    pub fn clause_by_derivation_ref(&self, parent: ClauseDerivationRef) -> Option<&Clause> {
        self.live_and_archive_clause_sets()
            .into_iter()
            .find_map(|set| set.find_by_derivation_ref(parent))
    }

    #[must_use]
    pub fn proof_clause_by_derivation_ref(&self, parent: ClauseDerivationRef) -> Option<&Clause> {
        find_by_derivation_ref_or_sourceless_id(&self.proof_clause_sets(), parent)
    }

    #[must_use]
    pub fn proof_formula_by_derivation_ref(
        &self,
        parent: FormulaDerivationRef,
    ) -> Option<&WrappedFormula> {
        self.proof_formula_sets()
            .into_iter()
            .find_map(|set| find_formula_by_derivation_ref(set, parent))
    }

    #[must_use]
    pub fn proof_quote_source_by_derivation_ref(
        &self,
        parent: ClauseDerivationRef,
    ) -> Option<&Clause> {
        find_by_derivation_ref_or_sourceless_id(&self.proof_quote_source_clause_sets(), parent)
    }

    #[must_use]
    pub fn clause_by_derivation_ref_mut(
        &mut self,
        parent: ClauseDerivationRef,
    ) -> Option<&mut Clause> {
        if let Some(clause) = self.axioms.find_by_derivation_ref_mut(parent) {
            return Some(clause);
        }
        if let Some(clause) = self.ax_archive.find_by_derivation_ref_mut(parent) {
            return Some(clause);
        }
        if let Some(clause) = self.processed_pos_rules.find_by_derivation_ref_mut(parent) {
            return Some(clause);
        }
        if let Some(clause) = self.processed_pos_eqns.find_by_derivation_ref_mut(parent) {
            return Some(clause);
        }
        if let Some(clause) = self.processed_neg_units.find_by_derivation_ref_mut(parent) {
            return Some(clause);
        }
        if let Some(clause) = self.processed_non_units.find_by_derivation_ref_mut(parent) {
            return Some(clause);
        }
        if let Some(clause) = self.unprocessed.find_by_derivation_ref_mut(parent) {
            return Some(clause);
        }
        if let Some(clause) = self.tmp_store.find_by_derivation_ref_mut(parent) {
            return Some(clause);
        }
        if let Some(clause) = self.eval_store.find_by_derivation_ref_mut(parent) {
            return Some(clause);
        }
        if let Some(clause) = self.archive.find_by_derivation_ref_mut(parent) {
            return Some(clause);
        }
        if let Some(clause) = self.definition_store.find_by_derivation_ref_mut(parent) {
            return Some(clause);
        }
        self.watchlist
            .as_mut()
            .and_then(|watchlist| watchlist.find_by_derivation_ref_mut(parent))
    }

    #[must_use]
    pub fn clause_parent_is_dead(&self, parent: DerivationParentRef) -> bool {
        match parent {
            DerivationParentRef::Clause(parent) => {
                let mut saw_live_parent = false;
                for set in self.live_and_archive_clause_sets() {
                    if let Some(clause) = set.find_by_derivation_ref(parent) {
                        if clause.query_prop(CP_IS_DEAD) {
                            return true;
                        }
                        saw_live_parent = true;
                    }
                }
                !saw_live_parent
            }
            DerivationParentRef::Demodulator(_) | DerivationParentRef::Formula(_) => false,
        }
    }

    fn live_and_archive_clause_sets(&self) -> Vec<&ClauseSet> {
        let mut sets = vec![
            &self.axioms,
            &self.ax_archive,
            &self.processed_pos_rules,
            &self.processed_pos_eqns,
            &self.processed_neg_units,
            &self.processed_non_units,
            &self.unprocessed,
            &self.tmp_store,
            &self.eval_store,
            &self.archive,
            &self.definition_store,
        ];
        if let Some(watchlist) = self.watchlist.as_ref() {
            sets.push(watchlist);
        }
        sets
    }

    fn proof_clause_sets(&self) -> Vec<&ClauseSet> {
        let mut sets = vec![
            &self.archive,
            &self.processed_pos_rules,
            &self.processed_pos_eqns,
            &self.processed_neg_units,
            &self.processed_non_units,
            &self.ax_archive,
            &self.axioms,
            &self.unprocessed,
            &self.tmp_store,
            &self.eval_store,
            &self.definition_store,
        ];
        if let Some(watchlist) = self.watchlist.as_ref() {
            sets.push(watchlist);
        }
        sets
    }

    fn proof_quote_source_clause_sets(&self) -> Vec<&ClauseSet> {
        let mut sets = vec![
            &self.ax_archive,
            &self.axioms,
            &self.archive,
            &self.processed_pos_rules,
            &self.processed_pos_eqns,
            &self.processed_neg_units,
            &self.processed_non_units,
            &self.unprocessed,
            &self.tmp_store,
            &self.eval_store,
            &self.definition_store,
        ];
        if let Some(watchlist) = self.watchlist.as_ref() {
            sets.push(watchlist);
        }
        sets
    }

    fn proof_formula_sets(&self) -> Vec<&FormulaSet> {
        vec![
            &self.definition_formula_archive,
            &self.f_archive,
            &self.f_ax_archive,
            &self.f_axioms,
        ]
    }

    #[must_use]
    pub const fn fvi_initialized(&self) -> bool {
        self.fvi_initialized
    }

    #[must_use]
    pub const fn fvi_cspec(&self) -> Option<&FvCollect> {
        self.fvi_cspec.as_ref()
    }

    #[must_use]
    pub const fn def_store_cspec(&self) -> Option<&FvCollect> {
        self.def_store_cspec.as_ref()
    }

    #[must_use]
    pub const fn state_is_complete(&self) -> bool {
        self.state_is_complete
    }

    pub const fn set_state_is_complete(&mut self, complete: bool) {
        self.state_is_complete = complete;
    }

    #[must_use]
    pub const fn has_interpreted_symbols(&self) -> bool {
        self.has_interpreted_symbols
    }

    #[must_use]
    pub const fn raw_formula_features(&self) -> &RawFormulaFeatures {
        &self.raw_formula_features
    }

    pub fn add_raw_formula_features(&mut self, features: RawFormulaFeatures) {
        self.raw_formula_features.add(features);
    }

    #[must_use]
    pub const fn statistics(&self) -> &ProofStateStatistics {
        &self.statistics
    }

    pub fn statistics_mut(&mut self) -> &mut ProofStateStatistics {
        &mut self.statistics
    }

    /// Marks represented clause ancestors that participate in a successful proof.
    ///
    /// This ports the clause-side marking side effect of C
    /// `DerivationMarkProofSteps`: an empty or already proof-marked root makes
    /// reachable clause parents proof clauses. Rust also accepts semantically
    /// false roots while supported answer-limit proof roots still use that
    /// executable bridge representation. Formula parents and ordered proof
    /// graph construction stay with the future formula/archive derivation
    /// slice.
    #[must_use]
    pub fn mark_proof_clause_ancestors(&mut self, root: &Clause) -> u64 {
        if !root.is_empty() && !root.is_sem_false() && !root.query_prop(CP_IS_PROOF_CLAUSE) {
            return 0;
        }

        let ac_axioms = self.terms().signature().ac_axioms().to_vec();
        let (parents, _) = deriv_stack_extract_parents(root.derivation(), &ac_axioms);
        self.mark_proof_clause_parent_refs(parents, &ac_axioms)
    }

    /// Counts EvalGC-selected clauses like C `ProofStateAnalyseGC`.
    ///
    /// C accumulates the result into `gc_count` and `gc_used_count` instead of
    /// replacing existing values. Rust preserves that behavior while also
    /// returning the one-shot analysis for callers that need a snapshot.
    #[must_use]
    pub fn analyse_gc(&mut self) -> ProofStateGcAnalysis {
        let analysis = self.gc_analysis();
        self.statistics.gc_count = self
            .statistics
            .gc_count
            .saturating_add(analysis.given_count);
        self.statistics.gc_used_count = self
            .statistics
            .gc_used_count
            .saturating_add(analysis.used_given_count);
        analysis
    }

    /// Selects positive and negative training examples like C
    /// `ProofStatePickTrainingExamples`.
    #[must_use]
    pub fn pick_training_examples(&self) -> ProofStateTrainingExamples<'_> {
        let mut examples = ProofStateTrainingExamples::default();
        for set in self.gc_clause_sets() {
            for clause in set.iter().filter(|clause| clause_is_eval_gc(clause)) {
                if clause.query_prop(CP_IS_PROOF_CLAUSE) {
                    examples.positive.push(clause);
                } else {
                    examples.negative.push(clause);
                }
            }
        }
        examples
    }

    #[must_use]
    pub fn proof_object_analysis_for_roots<'a, I>(&self, roots: I) -> ProofObjectAnalysis
    where
        I: IntoIterator<Item = &'a Clause>,
    {
        self.proof_object_analysis_for_mixed_roots(roots, std::iter::empty::<&'a WrappedFormula>())
    }

    #[must_use]
    pub fn proof_object_analysis_for_mixed_roots<'a, C, F>(
        &self,
        clause_roots: C,
        formula_roots: F,
    ) -> ProofObjectAnalysis
    where
        C: IntoIterator<Item = &'a Clause>,
        F: IntoIterator<Item = &'a WrappedFormula>,
    {
        let mut analysis = ProofObjectAnalysis::default();
        let mut visited = Vec::new();
        let mut pending_edges = Vec::new();
        let mut formula_visited = Vec::new();
        let ac_axioms = self.terms().signature().ac_axioms();

        for root in clause_roots {
            let root = self.proof_object_first_clause(root);
            Self::analyse_proof_object_clause(
                root,
                &mut analysis,
                &mut visited,
                &mut pending_edges,
                ac_axioms,
            );
        }
        for root in formula_roots {
            let root = self.proof_object_first_formula(root);
            Self::analyse_proof_object_formula(
                root,
                &mut analysis,
                &mut formula_visited,
                &mut pending_edges,
                ac_axioms,
            );
        }

        while let Some(edge) = pending_edges.pop() {
            match edge.parent {
                DerivationParentRef::Clause(_) | DerivationParentRef::Demodulator(_) => {
                    for clause in self.proof_object_edge_clauses(edge) {
                        let clause = self.proof_object_edge_first_clause(edge, clause);
                        Self::analyse_proof_object_clause(
                            clause,
                            &mut analysis,
                            &mut visited,
                            &mut pending_edges,
                            ac_axioms,
                        );
                    }
                }
                DerivationParentRef::Formula(parent) => {
                    if let Some(formula) = self.proof_object_formula_by_derivation_ref(parent) {
                        Self::analyse_proof_object_formula(
                            formula,
                            &mut analysis,
                            &mut formula_visited,
                            &mut pending_edges,
                            ac_axioms,
                        );
                    }
                }
            }
        }

        analysis
    }

    #[must_use]
    pub fn proof_object_graph_for_roots<'a, I>(&'a self, roots: I) -> ProofObjectGraph<'a>
    where
        I: IntoIterator<Item = &'a Clause>,
    {
        self.proof_object_graph_for_mixed_roots(roots, std::iter::empty::<&'a WrappedFormula>())
    }

    #[must_use]
    pub fn proof_object_graph_for_mixed_roots<'a, C, F>(
        &'a self,
        clause_roots: C,
        formula_roots: F,
    ) -> ProofObjectGraph<'a>
    where
        C: IntoIterator<Item = &'a Clause>,
        F: IntoIterator<Item = &'a WrappedFormula>,
    {
        let mut graph = ProofObjectGraph::default();
        let mut clause_visited = Vec::new();
        let mut formula_visited = Vec::new();
        let mut pending_edges = Vec::new();
        let ac_axioms = self.terms().signature().ac_axioms();

        for root in clause_roots {
            let root = self.proof_object_first_clause(root);
            let root_index = Self::collect_proof_object_graph_clause(
                root,
                &mut graph,
                &mut clause_visited,
                &mut pending_edges,
                ac_axioms,
            );
            graph
                .clause_aliases
                .entry(ClauseDerivationRef::from(root))
                .or_insert(root_index);
            if !graph.root_indices.contains(&root_index) {
                graph.root_indices.push(root_index);
            }
        }
        for root in formula_roots {
            let root = self.proof_object_first_formula(root);
            let root_index = Self::collect_proof_object_graph_formula(
                root,
                &mut graph,
                &mut formula_visited,
                &mut pending_edges,
                ac_axioms,
            );
            if !graph.formula_root_indices.contains(&root_index) {
                graph.formula_root_indices.push(root_index);
            }
        }

        while let Some((child, edge)) = pending_edges.pop() {
            match edge.parent {
                DerivationParentRef::Clause(_) | DerivationParentRef::Demodulator(_) => {
                    for clause in self.proof_object_edge_clauses(edge) {
                        let clause = self.proof_object_edge_first_clause(edge, clause);
                        let parent_index = Self::collect_proof_object_graph_clause(
                            clause,
                            &mut graph,
                            &mut clause_visited,
                            &mut pending_edges,
                            ac_axioms,
                        );
                        if let DerivationParentRef::Clause(parent_ref) = edge.parent {
                            graph
                                .clause_aliases
                                .entry(parent_ref)
                                .or_insert(parent_index);
                        }
                        let parent = ProofObjectGraphNode::Clause(parent_index);
                        graph
                            .mixed_edges
                            .push(ProofObjectGraphMixedEdge { parent, child });
                        if let ProofObjectGraphNode::Clause(child_index) = child {
                            graph.edges.push(ProofObjectGraphEdge {
                                parent_index,
                                child_index,
                            });
                        }
                    }
                }
                DerivationParentRef::Formula(parent_ref) => {
                    if let Some(formula) = self.proof_object_formula_by_derivation_ref(parent_ref) {
                        let parent_index = Self::collect_proof_object_graph_formula(
                            formula,
                            &mut graph,
                            &mut formula_visited,
                            &mut pending_edges,
                            ac_axioms,
                        );
                        graph.mixed_edges.push(ProofObjectGraphMixedEdge {
                            parent: ProofObjectGraphNode::Formula(parent_index),
                            child,
                        });
                    }
                }
            }
        }

        graph
    }

    fn gc_analysis(&self) -> ProofStateGcAnalysis {
        let mut analysis = ProofStateGcAnalysis::default();
        for set in self.gc_clause_sets() {
            for clause in set.iter() {
                analysis.clause_count += 1;
                if clause_is_eval_gc(clause) {
                    analysis.given_count += 1;
                    if clause.query_prop(CP_IS_PROOF_CLAUSE) {
                        analysis.used_given_count += 1;
                    }
                }
            }
        }
        analysis
    }

    fn analyse_proof_object_clause(
        clause: &Clause,
        analysis: &mut ProofObjectAnalysis,
        visited: &mut Vec<*const Clause>,
        pending_edges: &mut Vec<ProofObjectParentEdge>,
        ac_axioms: &[ClauseDerivationRef],
    ) {
        let key = std::ptr::from_ref(clause);
        if visited.contains(&key) {
            return;
        }
        visited.push(key);

        analysis.clause_step_count += 1;
        if clause.is_conjecture() {
            analysis.clause_conjecture_count += 1;
        }
        if deriv_stack_indicates_initial_clause(clause.derivation()) {
            analysis.initial_clause_count += 1;
        }
        let (generating, simplifying) = deriv_stack_count_search_inferences(clause.derivation());
        analysis.generating_inference_count += generating;
        analysis.simplifying_inference_count += simplifying;

        pending_edges.extend(proof_object_parent_edges(clause.derivation(), ac_axioms));
    }

    fn analyse_proof_object_formula(
        formula: &WrappedFormula,
        analysis: &mut ProofObjectAnalysis,
        visited: &mut Vec<*const WrappedFormula>,
        pending_edges: &mut Vec<ProofObjectParentEdge>,
        ac_axioms: &[ClauseDerivationRef],
    ) {
        let key = std::ptr::from_ref(formula);
        if visited.contains(&key) {
            return;
        }
        visited.push(key);

        analysis.formula_step_count += 1;
        if formula.is_conjecture() {
            analysis.formula_conjecture_count += 1;
        }
        if formula.derivation().is_none() {
            analysis.initial_formula_count += 1;
        }

        pending_edges.extend(proof_object_parent_edges(formula.derivation(), ac_axioms));
    }

    fn collect_proof_object_graph_clause<'a>(
        clause: &'a Clause,
        graph: &mut ProofObjectGraph<'a>,
        visited: &mut Vec<(*const Clause, usize)>,
        pending_edges: &mut Vec<(ProofObjectGraphNode, ProofObjectParentEdge)>,
        ac_axioms: &[ClauseDerivationRef],
    ) -> usize {
        let key = std::ptr::from_ref(clause);
        if let Some((_, index)) = visited.iter().find(|(visited, _)| *visited == key) {
            return *index;
        }
        let index = graph.clauses.len();
        visited.push((key, index));
        graph.clauses.push(clause);
        graph
            .clause_aliases
            .entry(ClauseDerivationRef::from(clause))
            .or_insert(index);
        pending_edges.extend(
            proof_object_parent_edges(clause.derivation(), ac_axioms)
                .into_iter()
                .map(|edge| (ProofObjectGraphNode::Clause(index), edge)),
        );
        index
    }

    fn collect_proof_object_graph_formula<'a>(
        formula: &'a WrappedFormula,
        graph: &mut ProofObjectGraph<'a>,
        visited: &mut Vec<(*const WrappedFormula, usize)>,
        pending_edges: &mut Vec<(ProofObjectGraphNode, ProofObjectParentEdge)>,
        ac_axioms: &[ClauseDerivationRef],
    ) -> usize {
        let key = std::ptr::from_ref(formula);
        if let Some((_, index)) = visited.iter().find(|(visited, _)| *visited == key) {
            return *index;
        }
        let index = graph.formulas.len();
        visited.push((key, index));
        graph.formulas.push(formula);
        pending_edges.extend(
            proof_object_parent_edges(formula.derivation(), ac_axioms)
                .into_iter()
                .map(|edge| (ProofObjectGraphNode::Formula(index), edge)),
        );
        index
    }

    fn proof_object_first_clause<'a>(&'a self, clause: &'a Clause) -> &'a Clause {
        let mut current = clause;
        let mut visited = Vec::new();
        while clause_is_dummy_quote(current) {
            let key = std::ptr::from_ref(current);
            if visited.contains(&key) {
                break;
            }
            visited.push(key);
            let Some(parent) = clause_dummy_quote_parent_ref(current)
                .and_then(|parent| self.proof_quote_source_by_derivation_ref(parent))
            else {
                break;
            };
            if std::ptr::eq(parent, current) {
                break;
            }
            current = parent;
        }
        current
    }

    fn proof_object_edge_first_clause<'a>(
        &'a self,
        edge: ProofObjectParentEdge,
        clause: &'a Clause,
    ) -> &'a Clause {
        if matches!(edge.resolution, ProofObjectParentResolution::AcAxiom) {
            clause
        } else {
            self.proof_object_first_clause(clause)
        }
    }

    fn proof_object_first_formula<'a>(&'a self, formula: &'a WrappedFormula) -> &'a WrappedFormula {
        wformula_deriv_find_first(formula, |parent| {
            self.proof_formula_by_derivation_ref(parent)
        })
    }

    fn proof_object_formula_by_derivation_ref(
        &self,
        parent: FormulaDerivationRef,
    ) -> Option<&WrappedFormula> {
        self.proof_formula_by_derivation_ref(parent)
            .map(|formula| self.proof_object_first_formula(formula))
    }

    fn proof_object_edge_clauses(&self, edge: ProofObjectParentEdge) -> Vec<&Clause> {
        match edge.parent {
            DerivationParentRef::Clause(parent) => {
                let clause = match edge.resolution {
                    ProofObjectParentResolution::ProofStep
                    | ProofObjectParentResolution::AcAxiom => {
                        self.proof_clause_by_derivation_ref(parent)
                    }
                    ProofObjectParentResolution::QuoteSource => {
                        self.proof_quote_source_by_derivation_ref(parent)
                    }
                };
                clause.into_iter().collect()
            }
            DerivationParentRef::Demodulator(demodulator) => demodulator_clause_refs(demodulator)
                .into_iter()
                .filter_map(|parent| self.proof_clause_by_derivation_ref(parent))
                .collect(),
            DerivationParentRef::Formula(_) => Vec::new(),
        }
    }

    fn mark_proof_clause_parent_refs(
        &mut self,
        parents: Vec<DerivationParentRef>,
        ac_axioms: &[ClauseDerivationRef],
    ) -> u64 {
        let mut pending = parents;
        let mut visited = BTreeSet::new();
        let mut marked = 0;

        while let Some(parent) = pending.pop() {
            let DerivationParentRef::Clause(parent) = parent else {
                continue;
            };
            if !visited.insert(parent) {
                continue;
            }
            let Some((newly_marked, parent_parents)) =
                self.mark_proof_clause_by_derivation_ref(parent, ac_axioms)
            else {
                continue;
            };
            if newly_marked {
                marked += 1;
                pending.extend(parent_parents);
            }
        }

        marked
    }

    fn mark_proof_clause_by_derivation_ref(
        &mut self,
        parent: ClauseDerivationRef,
        ac_axioms: &[ClauseDerivationRef],
    ) -> Option<(bool, Vec<DerivationParentRef>)> {
        let clause = self.proof_clause_by_derivation_ref_mut(parent)?;
        if clause.query_prop(CP_IS_PROOF_CLAUSE) {
            return Some((false, Vec::new()));
        }
        clause.set_prop(CP_IS_PROOF_CLAUSE);
        let (parents, _) = deriv_stack_extract_parents(clause.derivation(), ac_axioms);
        Some((true, parents))
    }

    fn proof_clause_by_derivation_ref_mut(
        &mut self,
        parent: ClauseDerivationRef,
    ) -> Option<&mut Clause> {
        // C derivations store parent pointers. Until Rust has stable clause
        // handles, selected archive copies must win over same-id axioms for
        // proof-object GC/training marking.
        if let Some(clause) = self.archive.find_by_derivation_ref_mut(parent) {
            return Some(clause);
        }
        if let Some(clause) = self.processed_pos_rules.find_by_derivation_ref_mut(parent) {
            return Some(clause);
        }
        if let Some(clause) = self.processed_pos_eqns.find_by_derivation_ref_mut(parent) {
            return Some(clause);
        }
        if let Some(clause) = self.processed_neg_units.find_by_derivation_ref_mut(parent) {
            return Some(clause);
        }
        if let Some(clause) = self.processed_non_units.find_by_derivation_ref_mut(parent) {
            return Some(clause);
        }
        if let Some(clause) = self.ax_archive.find_by_derivation_ref_mut(parent) {
            return Some(clause);
        }
        if let Some(clause) = self.axioms.find_by_derivation_ref_mut(parent) {
            return Some(clause);
        }
        if let Some(clause) = self.unprocessed.find_by_derivation_ref_mut(parent) {
            return Some(clause);
        }
        if let Some(clause) = self.tmp_store.find_by_derivation_ref_mut(parent) {
            return Some(clause);
        }
        if let Some(clause) = self.eval_store.find_by_derivation_ref_mut(parent) {
            return Some(clause);
        }
        if let Some(clause) = self.definition_store.find_by_derivation_ref_mut(parent) {
            return Some(clause);
        }
        self.watchlist
            .as_mut()
            .and_then(|watchlist| watchlist.find_by_derivation_ref_mut(parent))
    }

    fn gc_clause_sets(&self) -> [&ClauseSet; 6] {
        [
            &self.ax_archive,
            &self.processed_pos_rules,
            &self.processed_pos_eqns,
            &self.processed_neg_units,
            &self.processed_non_units,
            &self.archive,
        ]
    }

    pub fn record_answer_clause(&mut self, clause: &Clause) {
        let Some(answer_output) = clause_answer_output_string(&self.terms, clause) else {
            return;
        };
        if !self.statistics.status_reported {
            self.answer_outputs
                .push(format!("{DEFAULT_COMCHAR_RAW} SZS status Theorem\n"));
            self.statistics.status_reported = true;
        }
        self.answer_outputs.push(answer_output);
    }

    #[must_use]
    pub fn answer_outputs(&self) -> &[String] {
        &self.answer_outputs
    }

    pub fn take_answer_outputs(&mut self) -> Vec<String> {
        std::mem::take(&mut self.answer_outputs)
    }

    #[must_use]
    pub fn processed_cardinality(&self) -> i64 {
        self.processed_pos_rules.members()
            + self.processed_pos_eqns.members()
            + self.processed_neg_units.members()
            + self.processed_non_units.members()
    }

    #[must_use]
    pub fn unprocessed_cardinality(&self) -> i64 {
        self.unprocessed.members()
    }

    #[must_use]
    pub fn cardinality(&self) -> i64 {
        self.processed_cardinality() + self.unprocessed_cardinality()
    }

    /// Counts clause and formula axioms like C `ProofStateAxNo`.
    #[must_use]
    pub fn axiom_count(&self) -> i64 {
        self.axioms.members() + self.f_axioms.cardinality()
    }

    #[must_use]
    pub fn is_untyped(&self) -> bool {
        self.processed_pos_rules.is_untyped()
            && self.processed_pos_eqns.is_untyped()
            && self.processed_neg_units.is_untyped()
            && self.processed_non_units.is_untyped()
            && self.unprocessed.is_untyped()
    }

    /// Clears the clause/formula sets covered by C `ProofStateResetClauseSets`.
    ///
    /// The C helper does not clear `definition_store`, its definition formula
    /// archive, or `f_archive`, despite its comment saying all clause and
    /// formula sets are emptied. Rust preserves that until reset semantics are
    /// audited with callers.
    pub fn reset_clause_sets(&mut self) {
        self.axioms.clear();
        self.f_axioms.clear();
        self.ax_archive.clear();
        self.f_ax_archive.clear();
        self.processed_pos_rules.clear();
        self.processed_pos_eqns.clear();
        self.processed_neg_units.clear();
        self.processed_non_units.clear();
        self.unprocessed.clear();
        self.tmp_store.clear();
        self.eval_store.clear();
        self.archive.clear();
        self.raw_formula_features = RawFormulaFeatures::default();
        if let Some(watchlist) = self.watchlist.as_mut() {
            watchlist.clear();
        }
    }

    /// Marks proof-state clause terms and sweeps unreachable term-bank entries.
    ///
    /// C `TBGCCollect(state->terms)` marks registered clause/formula sets through
    /// the term bank's GC admin. Rust stores stable handles for the represented
    /// proof-state owners and resolves those handles to the owned sets here.
    pub fn collect_term_garbage(&mut self) -> i64 {
        let clause_handles = self.terms.gc().clause_set_handles().collect::<Vec<_>>();
        let formula_handles = self.terms.gc().formula_set_handles().collect::<Vec<_>>();
        let clause_roots = [
            (GC_AXIOMS, Some(&self.axioms)),
            (GC_AX_ARCHIVE, Some(&self.ax_archive)),
            (GC_PROCESSED_POS_RULES, Some(&self.processed_pos_rules)),
            (GC_PROCESSED_POS_EQNS, Some(&self.processed_pos_eqns)),
            (GC_PROCESSED_NEG_UNITS, Some(&self.processed_neg_units)),
            (GC_PROCESSED_NON_UNITS, Some(&self.processed_non_units)),
            (GC_UNPROCESSED, Some(&self.unprocessed)),
            (GC_TMP_STORE, Some(&self.tmp_store)),
            (GC_EVAL_STORE, Some(&self.eval_store)),
            (GC_ARCHIVE, Some(&self.archive)),
            (GC_WATCHLIST, self.watchlist.as_ref()),
            (GC_DEFINITION_STORE, Some(&self.definition_store)),
        ];
        let formula_roots = [
            (GC_F_AXIOMS, &self.f_axioms),
            (GC_F_AX_ARCHIVE, &self.f_ax_archive),
            (
                GC_DEFINITION_FORMULA_ARCHIVE,
                &self.definition_formula_archive,
            ),
            (GC_F_ARCHIVE, &self.f_archive),
        ];

        for handle in clause_handles {
            for (registered, set) in &clause_roots {
                if *registered == handle {
                    if let Some(set) = *set {
                        gc_mark_clause_set_terms(set, &self.terms);
                    }
                    break;
                }
            }
        }

        for handle in formula_handles {
            for (registered, set) in &formula_roots {
                if *registered == handle {
                    (*set).gc_mark_cells(&self.terms);
                    break;
                }
            }
        }

        self.terms.gc_sweep()
    }

    /// Loads or disables the proof-state watchlist like C
    /// `ProofStateLoadWatchlist`.
    ///
    /// File sources are parsed into the existing watchlist set and then require
    /// end-of-file. Inline sources skip parsing but still activate the current
    /// watchlist. Disabled sources drop the optional watchlist. C's initial
    /// documentation output is emitted by the executable compatibility layer.
    ///
    /// # Errors
    ///
    /// Returns diagnostics from file opening, parsing, trailing-token checks, or
    /// attempting to activate a watchlist that was already disabled.
    pub fn load_watchlist(
        &mut self,
        source: WatchlistSource<'_>,
        parse_format: IoFormat,
    ) -> Result<i64, Diagnostic> {
        if source == WatchlistSource::Disabled {
            self.discard_watchlist();
            return Ok(0);
        }

        let Self {
            terms, watchlist, ..
        } = self;
        let watchlist = watchlist.as_mut().ok_or_else(|| {
            Diagnostic::new(
                ErrorCode::OTHER_ERROR,
                "Cannot activate a proof-state watchlist after it has been disabled",
            )
        })?;

        let parsed = match source {
            WatchlistSource::Disabled => unreachable!("disabled watchlist handled above"),
            WatchlistSource::Inline => 0,
            WatchlistSource::File(path) => {
                let mut scanner = Scanner::from_file(path, true)?;
                scanner.set_format(parse_format);
                let parsed = watchlist.parse_list(&mut scanner, terms, ProblemType::FirstOrder)?;
                scanner.check_tok(TokenType::NO_TOKEN)?;
                parsed
            }
        };

        activate_watchlist(watchlist, terms);
        self.watchlist_activation = WatchlistActivation::Active;
        Ok(parsed)
    }

    /// Prints the main proof-state clause sets like C `ProofStatePrint`.
    ///
    /// The C output intentionally prints processed positive rewrite rules and
    /// processed positive equations under the same heading. Rust preserves that
    /// shape for compatibility.
    ///
    /// # Errors
    ///
    /// Returns any formatting error from `output`.
    pub fn write_print(&self, output: &mut impl fmt::Write) -> fmt::Result {
        writeln!(
            output,
            "\n{DEFAULT_COMCHAR_RAW} Processed positive unit clauses:"
        )?;
        output.write_str(&self.processed_pos_rules.print_lop_string(&self.terms, true))?;
        output.write_str(&self.processed_pos_eqns.print_lop_string(&self.terms, true))?;
        writeln!(
            output,
            "\n{DEFAULT_COMCHAR_RAW} Processed negative unit clauses:"
        )?;
        output.write_str(&self.processed_neg_units.print_lop_string(&self.terms, true))?;
        writeln!(
            output,
            "\n{DEFAULT_COMCHAR_RAW} Processed non-unit clauses:"
        )?;
        output.write_str(&self.processed_non_units.print_lop_string(&self.terms, true))?;
        writeln!(output, "\n{DEFAULT_COMCHAR_RAW} Unprocessed clauses:")?;
        output.write_str(&self.unprocessed.print_lop_string(&self.terms, true))
    }

    #[must_use]
    pub fn print_string(&self) -> String {
        let mut output = String::new();
        let _ = self.write_print(&mut output);
        output
    }

    /// Prints proof-state counters like C `ProofStateStatisticsPrint`.
    ///
    /// The represented detailed-statistics subset currently covers generated
    /// literal, term-node, and demodulator-index counters.
    ///
    /// # Errors
    ///
    /// Returns any formatting error from `output`.
    pub fn write_statistics(
        &self,
        output: &mut impl fmt::Write,
        record_gc_selection: bool,
        print_details: bool,
    ) -> fmt::Result {
        self.write_processed_statistics(output)?;
        self.write_generation_statistics(output)?;
        self.write_satcheck_statistics(output)?;
        self.write_clause_set_statistics(output, record_gc_selection)?;
        if print_details {
            self.write_detailed_statistics(output)?;
        }
        Ok(())
    }

    fn write_processed_statistics(&self, output: &mut impl fmt::Write) -> fmt::Result {
        let statistics = self.statistics();
        writeln!(
            output,
            "{DEFAULT_COMCHAR_RAW} Initial clauses in saturation        : {}",
            self.axioms.members()
        )?;
        writeln!(
            output,
            "{DEFAULT_COMCHAR_RAW} Processed clauses                    : {}",
            statistics.processed_count
        )?;
        writeln!(
            output,
            "{DEFAULT_COMCHAR_RAW} ...of these trivial                  : {}",
            statistics.proc_trivial_count
        )?;
        writeln!(
            output,
            "{DEFAULT_COMCHAR_RAW} ...subsumed                          : {}",
            statistics.proc_forward_subsumed_count
        )?;
        writeln!(
            output,
            "{DEFAULT_COMCHAR_RAW} ...remaining for further processing  : {}",
            statistics.proc_non_trivial_count
        )?;
        writeln!(
            output,
            "{DEFAULT_COMCHAR_RAW} Other redundant clauses eliminated   : {}",
            statistics.other_redundant_count
        )?;
        writeln!(
            output,
            "{DEFAULT_COMCHAR_RAW} Clauses deleted for lack of memory   : {}",
            statistics.non_redundant_deleted
        )?;
        writeln!(
            output,
            "{DEFAULT_COMCHAR_RAW} Backward-subsumed                    : {}",
            statistics.backward_subsumed_count
        )?;
        writeln!(
            output,
            "{DEFAULT_COMCHAR_RAW} Backward-rewritten                   : {}",
            statistics.backward_rewritten_count
        )
    }

    fn write_generation_statistics(&self, output: &mut impl fmt::Write) -> fmt::Result {
        let statistics = self.statistics();
        let generated_clause_count = generated_clause_statistics_count(
            statistics.generated_count,
            statistics.backward_rewritten_count,
        );
        let cached_rewrite_steps = cached_rewrite_steps(
            statistics.rw_count,
            REWRITE_UNCACHED.load(Ordering::Relaxed),
        );
        writeln!(
            output,
            "{DEFAULT_COMCHAR_RAW} Generated clauses                    : {generated_clause_count}"
        )?;
        writeln!(
            output,
            "{DEFAULT_COMCHAR_RAW} ...of the previous two non-redundant : {}",
            statistics.non_trivial_generated_count
        )?;
        writeln!(
            output,
            "{DEFAULT_COMCHAR_RAW} ...aggressively subsumed             : {}",
            statistics.aggressive_forward_subsumed_count
        )?;
        writeln!(
            output,
            "{DEFAULT_COMCHAR_RAW} Contextual simplify-reflections      : {}",
            statistics.context_sr_count
        )?;
        writeln!(
            output,
            "{DEFAULT_COMCHAR_RAW} Paramodulations                      : {}",
            statistics.paramod_count
        )?;
        writeln!(
            output,
            "{DEFAULT_COMCHAR_RAW} Factorizations                       : {}",
            statistics.factor_count
        )?;
        writeln!(
            output,
            "{DEFAULT_COMCHAR_RAW} NegExts                              : {}",
            statistics.neg_ext_count
        )?;
        writeln!(
            output,
            "{DEFAULT_COMCHAR_RAW} Equation resolutions                 : {}",
            statistics.resolv_count
        )?;
        writeln!(
            output,
            "{DEFAULT_COMCHAR_RAW} Disequality decompositions           : {}",
            statistics.disequ_deco_count
        )?;
        writeln!(
            output,
            "{DEFAULT_COMCHAR_RAW} Total rewrite steps                  : {}",
            statistics.rw_count
        )?;
        writeln!(
            output,
            "{DEFAULT_COMCHAR_RAW} ...of those cached                   : {cached_rewrite_steps}"
        )
    }

    fn write_satcheck_statistics(&self, output: &mut impl fmt::Write) -> fmt::Result {
        let statistics = self.statistics();
        writeln!(
            output,
            "{DEFAULT_COMCHAR_RAW} Propositional unsat checks           : {}",
            statistics.satcheck_count
        )?;
        writeln!(
            output,
            "{DEFAULT_COMCHAR_RAW}    Propositional check models        : {}",
            statistics.satcheck_satisfiable
        )?;
        writeln!(
            output,
            "{DEFAULT_COMCHAR_RAW}    Propositional check unsatisfiable : {}",
            statistics.satcheck_success
        )?;
        writeln!(
            output,
            "{DEFAULT_COMCHAR_RAW}    Propositional clauses             : {}",
            statistics.satcheck_full_size
        )?;
        writeln!(
            output,
            "{DEFAULT_COMCHAR_RAW}    Propositional clauses after purity: {}",
            statistics.satcheck_actual_size
        )?;
        writeln!(
            output,
            "{DEFAULT_COMCHAR_RAW}    Propositional unsat core size     : {}",
            statistics.satcheck_core_size
        )?;
        writeln!(
            output,
            "{DEFAULT_COMCHAR_RAW}    Propositional preprocessing time  : {:.3}",
            statistics.satcheck_preproc_time
        )?;
        writeln!(
            output,
            "{DEFAULT_COMCHAR_RAW}    Propositional encoding time       : {:.3}",
            statistics.satcheck_encoding_time
        )?;
        writeln!(
            output,
            "{DEFAULT_COMCHAR_RAW}    Propositional solver time         : {:.3}",
            statistics.satcheck_solver_time
        )?;
        writeln!(
            output,
            "{DEFAULT_COMCHAR_RAW}    Success case prop preproc time    : {:.3}",
            statistics.satcheck_preproc_stime
        )?;
        writeln!(
            output,
            "{DEFAULT_COMCHAR_RAW}    Success case prop encoding time   : {:.3}",
            statistics.satcheck_encoding_stime
        )?;
        writeln!(
            output,
            "{DEFAULT_COMCHAR_RAW}    Success case prop solver time     : {:.3}",
            statistics.satcheck_solver_stime
        )
    }

    fn write_clause_set_statistics(
        &self,
        output: &mut impl fmt::Write,
        record_gc_selection: bool,
    ) -> fmt::Result {
        let statistics = self.statistics();
        writeln!(
            output,
            "{DEFAULT_COMCHAR_RAW} Current number of processed clauses  : {}",
            self.processed_cardinality()
        )?;
        writeln!(
            output,
            "{DEFAULT_COMCHAR_RAW}    Positive orientable unit clauses  : {}",
            self.processed_pos_rules.members()
        )?;
        writeln!(
            output,
            "{DEFAULT_COMCHAR_RAW}    Positive unorientable unit clauses: {}",
            self.processed_pos_eqns.members()
        )?;
        writeln!(
            output,
            "{DEFAULT_COMCHAR_RAW}    Negative unit clauses             : {}",
            self.processed_neg_units.members()
        )?;
        writeln!(
            output,
            "{DEFAULT_COMCHAR_RAW}    Non-unit-clauses                  : {}",
            self.processed_non_units.members()
        )?;
        writeln!(
            output,
            "{DEFAULT_COMCHAR_RAW} Current number of unprocessed clauses: {}",
            self.unprocessed.members()
        )?;
        writeln!(
            output,
            "{DEFAULT_COMCHAR_RAW} ...number of literals in the above   : {}",
            self.unprocessed.literals()
        )?;
        writeln!(
            output,
            "{DEFAULT_COMCHAR_RAW} Current number of archived formulas  : {}",
            self.f_archive.cardinality()
        )?;
        writeln!(
            output,
            "{DEFAULT_COMCHAR_RAW} Current number of archived clauses   : {}",
            self.archive.members()
        )?;
        if record_gc_selection {
            writeln!(
                output,
                "{DEFAULT_COMCHAR_RAW} Proof object given clauses           : {}",
                statistics.gc_used_count
            )?;
            writeln!(
                output,
                "{DEFAULT_COMCHAR_RAW} Proof search given clauses           : {}",
                statistics.gc_count
            )?;
        }
        Ok(())
    }

    fn write_detailed_statistics(&self, output: &mut impl fmt::Write) -> fmt::Result {
        let statistics = self.statistics();
        let generated_lit_count = generated_literal_statistics_count(
            statistics.generated_lit_count,
            statistics.backward_rewritten_lit_count,
        );
        writeln!(
            output,
            "{DEFAULT_COMCHAR_RAW} Total literals in generated clauses  : {generated_lit_count}"
        )?;
        writeln!(
            output,
            "{DEFAULT_COMCHAR_RAW} Shared term nodes                    : {}",
            self.terms.term_nodes()
        )?;
        writeln!(
            output,
            "{DEFAULT_COMCHAR_RAW} ...corresponding unshared nodes      : {}",
            self.detailed_unshared_term_nodes()
        )?;
        writeln!(
            output,
            "{DEFAULT_COMCHAR_RAW} Match attempts with oriented units   : {}",
            self.processed_pos_rules.demod_index_match_count()
        )?;
        writeln!(
            output,
            "{DEFAULT_COMCHAR_RAW} Match attempts with unoriented units : {}",
            self.processed_pos_eqns.demod_index_match_count()
        )?;
        #[cfg(feature = "measure-expensive")]
        {
            self.write_measure_expensive_statistics(output)?;
        }
        Ok(())
    }

    #[cfg(feature = "measure-expensive")]
    fn write_measure_expensive_statistics(&self, output: &mut impl fmt::Write) -> fmt::Result {
        writeln!(
            output,
            "{DEFAULT_COMCHAR_RAW} Oriented PDT nodes visited           : {}",
            self.processed_pos_rules.demod_index_visited_count()
        )?;
        writeln!(
            output,
            "{DEFAULT_COMCHAR_RAW} Unoriented PDT nodes visited         : {}",
            self.processed_pos_eqns.demod_index_visited_count()
        )
    }

    fn detailed_unshared_term_nodes(&self) -> i64 {
        [
            &self.tmp_store,
            &self.eval_store,
            &self.processed_pos_rules,
            &self.processed_pos_eqns,
            &self.processed_neg_units,
            &self.processed_non_units,
            &self.unprocessed,
        ]
        .into_iter()
        .map(|set| set.term_nodes(&self.terms))
        .sum()
    }

    #[must_use]
    pub fn statistics_string(&self, record_gc_selection: bool, print_details: bool) -> String {
        let mut output = String::new();
        let _ = self.write_statistics(&mut output, record_gc_selection, print_details);
        output
    }

    /// Initializes the preloaded watchlist clauses like the local clause-set
    /// portion of C `ProofStateInitWatchlist`.
    ///
    /// This orients and marks maximal terms, drains the watchlist through a
    /// temporary set, and reinserts it through the owned FV index when one is
    /// installed. C also inserts the result into `state->wlindices`; Rust keeps
    /// that caller-owned bridge in proof-control until long-lived global-index
    /// ownership can move into `ProofState`.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if bank-backed term ordering preparation fails.
    pub fn init_watchlist(&mut self, ocb: &mut OrderControlBlock) -> Result<i64, Diagnostic> {
        let Self {
            terms, watchlist, ..
        } = self;
        let Some(watchlist) = watchlist.as_mut() else {
            return Ok(0);
        };

        watchlist.mark_maximal_terms_with_bank(ocb, terms)?;
        let mut temp = ClauseSet::new();
        while let Some(clause) = watchlist.extract_first() {
            temp.insert(clause);
        }

        let inserted = watchlist.indexed_insert_clause_set_owned_with_bank(&mut temp, terms)?;
        debug_assert!(temp.is_empty());
        Ok(inserted)
    }

    /// Initializes and installs the FV-index anchors attached by C
    /// `fvi_param_init`.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if the current signature cannot be represented in
    /// the feature-vector index layout.
    pub fn init_fvi_anchors(&mut self, params: &FvIndexParams) -> Result<(), Diagnostic> {
        let specs = fvi_param_init_specs(self.terms.signature(), params)?;
        let anchors =
            fvi_param_init_anchors(&self.axioms, &specs, params, self.watchlist.is_some());
        self.fvi_cspec = Some(specs.cspec().clone());
        self.def_store_cspec = Some(specs.def_store_cspec().clone());

        let Self {
            processed_non_units,
            processed_pos_rules,
            processed_pos_eqns,
            processed_neg_units,
            watchlist,
            definition_store,
            ..
        } = self;
        anchors.install(FvIndexInitTargetSets {
            processed_non_units,
            processed_pos_rules,
            processed_pos_eqns,
            processed_neg_units,
            watchlist: watchlist.as_mut(),
            def_store: definition_store,
        });
        self.fvi_initialized = true;
        Ok(())
    }
}

fn find_by_derivation_ref_or_sourceless_id<'a>(
    sets: &[&'a ClauseSet],
    parent: ClauseDerivationRef,
) -> Option<&'a Clause> {
    sets.iter()
        .find_map(|set| set.find_by_derivation_ref(parent))
        .or_else(|| {
            if parent.source() == 0 && parent.generation() == 0 {
                sets.iter().find_map(|set| set.find_by_id(parent.ident()))
            } else {
                None
            }
        })
}

fn find_formula_by_derivation_ref(
    set: &FormulaSet,
    parent: FormulaDerivationRef,
) -> Option<&WrappedFormula> {
    set.iter()
        .find(|formula| parent.matches(formula.ident(), formula.entry_id()))
}

fn proof_object_parent_edges(
    derivation: Option<&crate::basics::pstacks::PStack<DerivationEntry>>,
    ac_axioms: &[ClauseDerivationRef],
) -> Vec<ProofObjectParentEdge> {
    let Some(derivation) = derivation else {
        return Vec::new();
    };

    let entries = derivation.as_slice();
    let mut edges = Vec::new();
    let mut index = 0;
    while index < entries.len() {
        let DerivationEntry::Operation(op) = entries[index] else {
            index += 1;
            continue;
        };
        index += 1;
        let resolution = if op == DC_CNF_QUOTE {
            ProofObjectParentResolution::QuoteSource
        } else {
            ProofObjectParentResolution::ProofStep
        };
        let mut numarg1 = None;

        if op_has_cnf_arg1(op) || op_has_fof_arg1(op) {
            push_proof_object_parent_edge(entries, &mut index, resolution, &mut edges);
        } else if op == DC_AC_RES {
            numarg1 = Some(read_proof_object_numeric_arg(entries, &mut index));
        } else if op_has_arg1(op) {
            index += 1;
        }

        if op_has_cnf_arg2(op) || op_has_fof_arg2(op) {
            push_proof_object_parent_edge(entries, &mut index, resolution, &mut edges);
        } else if op_has_arg2(op) {
            index += 1;
        }

        if op == DC_AC_RES {
            push_proof_object_ac_axiom_edges(numarg1.unwrap_or(0), ac_axioms, &mut edges);
        }
    }
    edges
}

fn push_proof_object_parent_edge(
    entries: &[DerivationEntry],
    index: &mut usize,
    resolution: ProofObjectParentResolution,
    edges: &mut Vec<ProofObjectParentEdge>,
) {
    if let Some(entry) = entries.get(*index) {
        match entry {
            DerivationEntry::ClauseParent(parent) => edges.push(ProofObjectParentEdge {
                parent: DerivationParentRef::Clause(*parent),
                resolution,
            }),
            DerivationEntry::Demodulator(demodulator) => edges.push(ProofObjectParentEdge {
                parent: DerivationParentRef::Demodulator(*demodulator),
                resolution: ProofObjectParentResolution::ProofStep,
            }),
            DerivationEntry::FormulaParent(parent) => edges.push(ProofObjectParentEdge {
                parent: DerivationParentRef::Formula(*parent),
                resolution,
            }),
            DerivationEntry::Operation(_) | DerivationEntry::NumericArg(_) => {}
        }
    }
    *index += 1;
}

fn read_proof_object_numeric_arg(entries: &[DerivationEntry], index: &mut usize) -> i64 {
    let entry = entries
        .get(*index)
        .unwrap_or_else(|| panic!("derivation numeric argument is missing"));
    *index += 1;
    match entry {
        DerivationEntry::NumericArg(value) => *value,
        DerivationEntry::Operation(_)
        | DerivationEntry::ClauseParent(_)
        | DerivationEntry::FormulaParent(_)
        | DerivationEntry::Demodulator(_) => {
            panic!("derivation numeric argument has the wrong entry shape")
        }
    }
}

fn push_proof_object_ac_axiom_edges(
    count: i64,
    ac_axioms: &[ClauseDerivationRef],
    edges: &mut Vec<ProofObjectParentEdge>,
) {
    let ac_count = usize::try_from(count)
        .unwrap_or_else(|_| panic!("DCACRes parent count must be non-negative"));
    assert!(
        ac_count <= ac_axioms.len(),
        "DCACRes parent count exceeds supplied AC axioms"
    );
    edges.extend(
        ac_axioms[..ac_count]
            .iter()
            .copied()
            .map(|parent| ProofObjectParentEdge {
                parent: DerivationParentRef::Clause(parent),
                resolution: ProofObjectParentResolution::AcAxiom,
            }),
    );
}

fn cached_rewrite_steps(rw_count: u64, rewrite_uncached: u64) -> u64 {
    rw_count.saturating_sub(rewrite_uncached)
}

fn c_signed_wrapping_difference(left: u64, right: u64) -> i64 {
    i64::from_ne_bytes(left.wrapping_sub(right).to_ne_bytes())
}

fn generated_clause_statistics_count(generated_count: u64, backward_rewritten_count: u64) -> i64 {
    c_signed_wrapping_difference(generated_count, backward_rewritten_count)
}

fn generated_literal_statistics_count(
    generated_lit_count: u64,
    backward_rewritten_lit_count: u64,
) -> i64 {
    c_signed_wrapping_difference(generated_lit_count, backward_rewritten_lit_count)
}

pub fn proof_state_alloc(free_symbol_props: FunctionProperties) -> Result<ProofState, Diagnostic> {
    ProofState::new(free_symbol_props)
}

fn activate_watchlist(watchlist: &mut ClauseSet, terms: &TermBank) {
    watchlist.set_tptp_type(CP_TYPE_WATCH_CLAUSE);
    watchlist.set_prop(CP_WATCH_ONLY);
    watchlist.default_weigh_clauses();
    watchlist.sort_literals_by(|left, right| i64::from(left.subsume_inverse_compare(right, terms)));
}

#[cfg(test)]
mod tests {
    use super::{
        cached_rewrite_steps, derived_dot_clause_link_colour, derived_dot_formula_link_colour,
        derived_dot_node_colour, derived_in_proof, derived_is_eval_gc, derived_set_in_proof,
        generated_clause_statistics_count, generated_literal_statistics_count, proof_state_alloc,
        DerivedView, DerivedViewMut, ProofObjectAnalysis, ProofObjectGraphEdge,
        ProofObjectGraphMixedEdge, ProofObjectGraphNode, ProofState, ProofStateGcAnalysis,
        ProofStateStatistics, WatchlistSource, GC_DEFINITION_FORMULA_ARCHIVE, GC_F_ARCHIVE,
        GC_F_AXIOMS, GC_F_AX_ARCHIVE, GC_WATCHLIST, PROOF_STATE_CLAUSE_GC_ROOTS,
        PROOF_STATE_FORMULA_GC_ROOTS,
    };
    use crate::basics::error::ErrorCode;
    use crate::basics::partial_orderings::HoOrderKind;
    use crate::basics::simple_stuff::ProblemType;
    use crate::clauses::clause::{clause_print_lop_format_string, Clause};
    use crate::clauses::clause_props::{
        CP_IS_DEAD, CP_IS_ORIENTED, CP_IS_PROOF_CLAUSE, CP_IS_S_INDEXED, CP_TYPE_AXIOM,
        CP_TYPE_CONJECTURE, CP_TYPE_NEG_CONJECTURE, CP_TYPE_QUESTION, CP_TYPE_WATCH_CLAUSE,
        CP_WATCH_ONLY,
    };
    use crate::clauses::derivation::{
        clause_push_ac_res_derivation, clause_push_derivation, clause_push_formula_derivation,
        ClauseDerivationRef, DerivationEntry, DerivationParentRef, FormulaDerivationRef,
        DC_APPLY_DEF, DC_CNF_EVAL_GC, DC_CNF_QUOTE, DC_EQ_RES, DC_EXPAND_DISTINCT, DC_FOF_QUOTE,
        DC_FOF_SIMPLIFY,
    };
    use crate::clauses::eqn::Eqn;
    use crate::clauses::eqn_props::EP_IS_MAXIMAL;
    use crate::clauses::eqnlist::EqnList;
    use crate::clauses::fcvindexing::FvIndexParams;
    use crate::clauses::formulasets::WrappedFormula;
    use crate::clauses::freqvectors::FvIndexType;
    use crate::clauses::proofstate::{WATCHLIST_INLINE_QSTRING, WATCHLIST_INLINE_STRING};
    use crate::heuristics::to_params::TermOrdering;
    use crate::inout::scanner::IoFormat;
    use crate::orderings::ocb::OrderControlBlock;
    use crate::terms::signature::{FP_DISTINCT_PROP, FP_IGNORE_PROPS};
    use crate::terms::termbanks::TermBank;
    use crate::terms::termtypes::{DerefType, Term};
    use std::path::PathBuf;

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

    fn typed_unary(bank: &mut TermBank, name: &str, arg: &Term) -> Term {
        let type_ = bank.signature().type_bank().default_type();
        let f_code = bank.signature_mut().insert_id(name, 1, false);
        if bank.signature().get_type(f_code).is_none() {
            bank.signature_mut()
                .declare_final_type(
                    f_code,
                    crate::terms::simpletypes::alloc_arrow_type(vec![type_.clone(), type_]),
                )
                .unwrap();
        }
        let term = Term::top_alloc(f_code, 1);
        term.set_type(Some(bank.signature().type_bank().default_type()));
        term.set_argument(0, arg.clone());
        bank.insert(&term, DerefType::Never).unwrap()
    }

    fn literal(bank: &mut TermBank, left: &Term, right: &Term, positive: bool) -> Eqn {
        Eqn::alloc(left.clone(), right.clone(), bank, positive).unwrap()
    }

    fn clause_from(literals: Vec<Eqn>, ident: i64) -> Clause {
        let mut clause = Clause::alloc(EqnList::from_vec(literals));
        clause.set_ident(ident);
        clause.set_weight(clause.standard_weight());
        clause
    }

    fn simple_clause(state: &mut ProofState, stem: &str, ident: i64) -> Clause {
        let bank = state.terms_mut();
        let left = typed_const(bank, &format!("{stem}_left"));
        let right = typed_const(bank, &format!("{stem}_right"));
        clause_from(vec![literal(bank, &left, &right, true)], ident)
    }

    fn eval_gc_clause(state: &mut ProofState, stem: &str, ident: i64, proof: bool) -> Clause {
        let mut clause = simple_clause(state, stem, ident);
        clause_push_derivation(&mut clause, DC_CNF_EVAL_GC, None, None);
        if proof {
            clause.set_prop(CP_IS_PROOF_CLAUSE);
        }
        clause
    }

    fn nontrivial_clause(state: &mut ProofState, stem: &str, ident: i64) -> Clause {
        let bank = state.terms_mut();
        let left = typed_const(bank, &format!("{stem}_left"));
        let right_const = typed_const(bank, &format!("{stem}_right"));
        let right = typed_unary(bank, &format!("{stem}_f"), &right_const);
        clause_from(vec![literal(bank, &left, &right, true)], ident)
    }

    fn wrapped_formula(state: &mut ProofState, name: &str) -> WrappedFormula {
        let formula = typed_const(state.terms_mut(), name);
        WrappedFormula::wt_formula_alloc(formula)
    }

    fn distinct_formula(bank: &mut TermBank, args: &[Term]) -> Term {
        let term = Term::top_alloc(bank.signature().distinct_code(), args.len());
        for (index, arg) in args.iter().enumerate() {
            term.set_argument(index, arg.clone());
        }
        bank.term_top_insert(term).unwrap()
    }

    fn temp_path(stem: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!(
            "e_rust_port_proofstate_{stem}_{}.p",
            std::process::id()
        ));
        path
    }

    fn assert_watchlist_clause_shape(state: &ProofState) {
        for clause in state.watchlist().unwrap().iter() {
            assert_eq!(clause.query_tptp_type(), CP_TYPE_WATCH_CLAUSE);
            assert!(clause.query_prop(CP_WATCH_ONLY));
            assert_eq!(clause.weight(), clause.standard_weight());
            assert!(clause.is_subsume_ordered(state.terms()));
        }
    }

    #[test]
    fn derived_view_helpers_follow_c_proof_and_eval_gc_rules() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();

        let empty = Clause::empty();
        assert!(derived_in_proof(DerivedView::Clause(&empty)));
        assert!(!derived_is_eval_gc(DerivedView::Clause(&empty)));

        let mut clause = simple_clause(&mut state, "derived_clause", 10);
        assert!(!derived_in_proof(DerivedView::Clause(&clause)));
        derived_set_in_proof(DerivedViewMut::Clause(&mut clause), true);
        assert!(clause.query_prop(CP_IS_PROOF_CLAUSE));
        assert!(derived_in_proof(DerivedView::Clause(&clause)));
        derived_set_in_proof(DerivedViewMut::Clause(&mut clause), false);
        assert!(!clause.query_prop(CP_IS_PROOF_CLAUSE));
        assert!(!derived_in_proof(DerivedView::Clause(&clause)));

        let eval_gc = eval_gc_clause(&mut state, "derived_eval_gc", 11, false);
        assert!(derived_is_eval_gc(DerivedView::Clause(&eval_gc)));

        let mut formula = wrapped_formula(&mut state, "derived_formula");
        assert!(!derived_in_proof(DerivedView::Formula(&formula)));
        assert!(!derived_is_eval_gc(DerivedView::Formula(&formula)));
        derived_set_in_proof(DerivedViewMut::Formula(&mut formula), true);
        assert!(formula.query_prop(CP_IS_PROOF_CLAUSE));
        assert!(derived_in_proof(DerivedView::Formula(&formula)));
        derived_set_in_proof(DerivedViewMut::Formula(&mut formula), false);
        assert!(!formula.query_prop(CP_IS_PROOF_CLAUSE));
        assert!(!derived_in_proof(DerivedView::Formula(&formula)));
    }

    #[test]
    fn derived_dot_node_colours_follow_c_clause_rules() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();

        let out_of_proof_axiom = simple_clause(&mut state, "dot_out_axiom", 20);
        assert_eq!(
            derived_dot_node_colour(DerivedView::Clause(&out_of_proof_axiom)),
            ",color=gray, fillcolor=gray66"
        );

        let mut out_of_proof_derived = simple_clause(&mut state, "dot_out_derived", 21);
        clause_push_derivation(&mut out_of_proof_derived, DC_CNF_EVAL_GC, None, None);
        assert_eq!(
            derived_dot_node_colour(DerivedView::Clause(&out_of_proof_derived)),
            ",color=gray, fillcolor=gray"
        );

        let empty = Clause::empty();
        assert_eq!(
            derived_dot_node_colour(DerivedView::Clause(&empty)),
            ",color=blue,fillcolor=darkorchid1"
        );

        let mut initial_axiom = simple_clause(&mut state, "dot_initial_axiom", 22);
        initial_axiom.set_tptp_type(CP_TYPE_AXIOM);
        derived_set_in_proof(DerivedViewMut::Clause(&mut initial_axiom), true);
        assert_eq!(
            derived_dot_node_colour(DerivedView::Clause(&initial_axiom)),
            ",color=green,fillcolor=forestgreen"
        );

        let mut derived_axiom = simple_clause(&mut state, "dot_derived_axiom", 23);
        derived_axiom.set_tptp_type(CP_TYPE_AXIOM);
        clause_push_derivation(&mut derived_axiom, DC_CNF_EVAL_GC, None, None);
        derived_set_in_proof(DerivedViewMut::Clause(&mut derived_axiom), true);
        assert_eq!(
            derived_dot_node_colour(DerivedView::Clause(&derived_axiom)),
            ",color=green,fillcolor=palegreen"
        );

        let mut initial_conjecture = simple_clause(&mut state, "dot_initial_conj", 24);
        initial_conjecture.set_tptp_type(CP_TYPE_CONJECTURE);
        derived_set_in_proof(DerivedViewMut::Clause(&mut initial_conjecture), true);
        assert_eq!(
            derived_dot_node_colour(DerivedView::Clause(&initial_conjecture)),
            ",color=blue,fillcolor=dodgerblue"
        );

        let mut derived_neg_conjecture = simple_clause(&mut state, "dot_derived_neg_conj", 25);
        derived_neg_conjecture.set_tptp_type(CP_TYPE_NEG_CONJECTURE);
        clause_push_derivation(&mut derived_neg_conjecture, DC_CNF_EVAL_GC, None, None);
        derived_set_in_proof(DerivedViewMut::Clause(&mut derived_neg_conjecture), true);
        assert_eq!(
            derived_dot_node_colour(DerivedView::Clause(&derived_neg_conjecture)),
            ",color=blue,fillcolor=lightskyblue1"
        );
    }

    #[test]
    fn derived_dot_node_colours_follow_c_formula_rules() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();

        let out_of_proof_formula = wrapped_formula(&mut state, "dot_out_formula");
        assert_eq!(
            derived_dot_node_colour(DerivedView::Formula(&out_of_proof_formula)),
            ",color=gray, fillcolor=gray66"
        );

        let mut out_of_proof_derived = wrapped_formula(&mut state, "dot_out_formula_derived");
        out_of_proof_derived.push_formula_derivation(DC_FOF_SIMPLIFY, None, None);
        assert_eq!(
            derived_dot_node_colour(DerivedView::Formula(&out_of_proof_derived)),
            ",color=gray, fillcolor=gray"
        );

        let mut initial_conjecture = wrapped_formula(&mut state, "dot_formula_conj");
        initial_conjecture.set_tptp_type(CP_TYPE_CONJECTURE);
        derived_set_in_proof(DerivedViewMut::Formula(&mut initial_conjecture), true);
        assert_eq!(
            derived_dot_node_colour(DerivedView::Formula(&initial_conjecture)),
            ",color=red,fillcolor=firebrick1"
        );

        let mut derived_conjecture = wrapped_formula(&mut state, "dot_formula_derived_conj");
        derived_conjecture.set_tptp_type(CP_TYPE_CONJECTURE);
        derived_conjecture.push_formula_derivation(DC_FOF_SIMPLIFY, None, None);
        derived_set_in_proof(DerivedViewMut::Formula(&mut derived_conjecture), true);
        assert_eq!(
            derived_dot_node_colour(DerivedView::Formula(&derived_conjecture)),
            ",color=red,fillcolor=lightpink1"
        );

        let mut initial_neg_conjecture = wrapped_formula(&mut state, "dot_formula_neg_conj");
        initial_neg_conjecture.set_tptp_type(CP_TYPE_NEG_CONJECTURE);
        derived_set_in_proof(DerivedViewMut::Formula(&mut initial_neg_conjecture), true);
        assert_eq!(
            derived_dot_node_colour(DerivedView::Formula(&initial_neg_conjecture)),
            ",color=blue,fillcolor=dodgerblue"
        );

        let mut question = wrapped_formula(&mut state, "dot_formula_question");
        question.set_tptp_type(CP_TYPE_QUESTION);
        derived_set_in_proof(DerivedViewMut::Formula(&mut question), true);
        assert_eq!(
            derived_dot_node_colour(DerivedView::Formula(&question)),
            ",color=green,fillcolor=forestgreen"
        );
    }

    #[test]
    fn derived_dot_link_colours_follow_c_parent_proof_rules() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();

        let mut parent = simple_clause(&mut state, "dot_clause_parent", 26);
        let mut child = simple_clause(&mut state, "dot_clause_child", 27);
        child.set_tptp_type(CP_TYPE_AXIOM);
        derived_set_in_proof(DerivedViewMut::Clause(&mut child), true);
        assert_eq!(
            derived_dot_clause_link_colour(DerivedView::Clause(&child), &parent),
            ",color=gray, fillcolor=gray"
        );

        parent.set_prop(CP_IS_PROOF_CLAUSE);
        assert_eq!(
            derived_dot_clause_link_colour(DerivedView::Clause(&child), &parent),
            ",color=green,fillcolor=forestgreen"
        );

        derived_set_in_proof(DerivedViewMut::Clause(&mut child), false);
        assert_eq!(
            derived_dot_clause_link_colour(DerivedView::Clause(&child), &parent),
            ",color=gray, fillcolor=gray"
        );

        let mut formula_parent = wrapped_formula(&mut state, "dot_formula_parent");
        let mut formula_child = wrapped_formula(&mut state, "dot_formula_child");
        formula_child.set_tptp_type(CP_TYPE_NEG_CONJECTURE);
        derived_set_in_proof(DerivedViewMut::Formula(&mut formula_child), true);
        assert_eq!(
            derived_dot_formula_link_colour(DerivedView::Formula(&formula_child), &formula_parent),
            ",color=gray, fillcolor=gray"
        );

        formula_parent.set_prop(CP_IS_PROOF_CLAUSE);
        assert_eq!(
            derived_dot_formula_link_colour(DerivedView::Formula(&formula_child), &formula_parent),
            ",color=blue,fillcolor=dodgerblue"
        );
    }

    fn test_ocb(state: &ProofState) -> OrderControlBlock {
        OrderControlBlock::alloc(
            TermOrdering::Kbo,
            true,
            state.terms().signature(),
            HoOrderKind::LfhoOrder,
        )
    }

    #[test]
    fn proof_state_alloc_initializes_c_shape_clause_sets_and_flags() {
        let state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();

        assert_eq!(WATCHLIST_INLINE_STRING, "Use inline watchlist type");
        assert_eq!(WATCHLIST_INLINE_QSTRING, "'Use inline watchlist type'");
        assert_eq!(state.original_symbols(), 0);
        assert!(state.watchlist().is_some());
        assert_eq!(state.axioms().members(), 0);
        assert_eq!(state.f_axioms().cardinality(), 0);
        assert_eq!(state.ax_archive().members(), 0);
        assert_eq!(state.f_ax_archive().cardinality(), 0);
        assert_eq!(state.processed_pos_rules().members(), 0);
        assert_eq!(state.processed_pos_eqns().members(), 0);
        assert_eq!(state.processed_neg_units().members(), 0);
        assert_eq!(state.processed_non_units().members(), 0);
        assert!(state.processed_pos_rules().demod_index().is_some());
        assert!(state.processed_pos_eqns().demod_index().is_some());
        assert!(state.processed_neg_units().demod_index().is_some());
        assert!(state.processed_non_units().demod_index().is_none());
        assert_eq!(state.unprocessed().members(), 0);
        assert_eq!(state.tmp_store().members(), 0);
        assert_eq!(state.eval_store().members(), 0);
        assert_eq!(state.archive().members(), 0);
        assert_eq!(state.f_archive().cardinality(), 0);
        assert_eq!(state.definition_store().members(), 0);
        assert_eq!(state.definition_formula_archive().cardinality(), 0);
        assert_eq!(state.definition_assocs().len(), 0);
        assert_eq!(state.definition_formula_assocs().len(), 0);
        assert_eq!(
            state.global_indices().problem_type(),
            ProblemType::NotInitialized
        );
        assert_eq!(
            state.watchlist_indices().problem_type(),
            ProblemType::NotInitialized
        );
        assert!(state.state_is_complete());
        assert!(!state.has_interpreted_symbols());
        assert!(!state.fvi_initialized());
        assert!(state.fvi_cspec().is_none());
        assert!(state.def_store_cspec().is_none());
        assert_eq!(state.statistics(), &ProofStateStatistics::default());
        assert!(state.terms().signature().distinct_code() > 0);
    }

    #[test]
    fn proof_state_scoped_index_access_restores_both_owners() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        state.global_indices_mut().init_for_problem(
            "FP1",
            "FP2",
            "FP3",
            4,
            ProblemType::HigherOrder,
        );
        state.watchlist_indices_mut().init_for_problem(
            "FP4",
            "NoIndex",
            "NoIndex",
            4,
            ProblemType::HigherOrder,
        );

        let observed = state.with_global_and_watchlist_indices(|state, global, watchlist| {
            assert_eq!(
                state.global_indices().problem_type(),
                ProblemType::NotInitialized
            );
            assert_eq!(
                state.watchlist_indices().problem_type(),
                ProblemType::NotInitialized
            );
            (
                global.pm_from_index_type().to_owned(),
                watchlist.rw_bw_index_type().to_owned(),
            )
        });

        assert_eq!(observed, ("FP2".to_owned(), "FP4".to_owned()));
        assert_eq!(state.global_indices().pm_into_index_type(), "FP3");
        assert_eq!(state.watchlist_indices().rw_bw_index_type(), "FP4");

        let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            state.with_global_and_watchlist_indices(|_, _, _| panic!("index access test"));
        }));
        assert!(panic.is_err());
        assert_eq!(state.global_indices().pm_into_index_type(), "FP3");
        assert_eq!(state.watchlist_indices().rw_bw_index_type(), "FP4");
    }

    #[test]
    fn proof_state_alloc_registers_represented_gc_roots() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let gc = state.terms().gc();

        assert_eq!(gc.clause_set_count(), PROOF_STATE_CLAUSE_GC_ROOTS.len());
        assert_eq!(gc.formula_set_count(), PROOF_STATE_FORMULA_GC_ROOTS.len());
        for root in PROOF_STATE_CLAUSE_GC_ROOTS {
            assert!(gc.has_clause_set(root));
        }
        for root in PROOF_STATE_FORMULA_GC_ROOTS {
            assert!(gc.has_formula_set(root));
        }
        assert!(gc.has_clause_set(GC_WATCHLIST));
        assert!(gc.has_formula_set(GC_F_AXIOMS));
        assert!(gc.has_formula_set(GC_F_AX_ARCHIVE));
        assert!(gc.has_formula_set(GC_DEFINITION_FORMULA_ARCHIVE));
        assert!(gc.has_formula_set(GC_F_ARCHIVE));

        assert!(state.discard_watchlist().is_some());
        assert!(!state.terms().gc().has_clause_set(GC_WATCHLIST));
        assert_eq!(
            state.terms().gc().clause_set_count(),
            PROOF_STATE_CLAUSE_GC_ROOTS.len() - 1
        );
    }

    #[test]
    fn proof_state_parent_lookup_uses_ident_and_source() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let mut first = simple_clause(&mut state, "parent_lookup_first", 20_001);
        first.set_csscpa_source(1);
        let mut second = simple_clause(&mut state, "parent_lookup_second", 20_001);
        second.set_csscpa_source(2);
        state.unprocessed_mut().insert(first);
        state.archive_mut().insert(second);

        assert_eq!(
            state
                .clause_by_derivation_ref(ClauseDerivationRef::new(20_001, 1))
                .map(Clause::query_csscpa_source),
            Some(1)
        );
        assert_eq!(
            state
                .clause_by_derivation_ref(ClauseDerivationRef::new(20_001, 2))
                .map(Clause::query_csscpa_source),
            Some(2)
        );
        assert!(!state.clause_parent_is_dead(DerivationParentRef::Clause(
            ClauseDerivationRef::new(20_001, 1)
        )));
        assert!(state.clause_parent_is_dead(DerivationParentRef::Clause(
            ClauseDerivationRef::new(20_001, 3)
        )));
    }

    #[test]
    fn proof_state_formula_parent_lookup_uses_definition_formula_archive() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let formula = wrapped_formula(&mut state, "definition_formula_parent_lookup");
        let formula_ref = FormulaDerivationRef::new(formula.ident());

        state.definition_formula_archive_mut().insert(formula);

        assert_eq!(
            state
                .proof_formula_by_derivation_ref(formula_ref)
                .map(WrappedFormula::ident),
            Some(formula_ref.ident())
        );
    }

    #[test]
    fn proof_state_formula_parent_lookup_distinguishes_flat_copy_sources() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let original = wrapped_formula(&mut state, "formula_parent_original");
        let copy = original.flat_copy();
        assert_eq!(original.ident(), copy.ident());
        let original_ref = original.derivation_ref();
        let copy_ref = copy.derivation_ref();

        state.f_ax_archive_mut().insert(original);
        state.f_ax_archive_mut().insert(copy);

        assert_eq!(
            state
                .proof_formula_by_derivation_ref(original_ref)
                .map(WrappedFormula::entry_id),
            Some(original_ref.source())
        );
        assert_eq!(
            state
                .proof_formula_by_derivation_ref(copy_ref)
                .map(WrappedFormula::entry_id),
            Some(copy_ref.source())
        );
    }

    #[test]
    fn proof_state_formula_parent_lookup_survives_storage_growth_and_set_move() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let original = wrapped_formula(&mut state, "stable_formula_parent");
        let original_ref = original.derivation_ref();
        let mut quote = original.flat_copy();
        quote.push_formula_derivation(DC_FOF_QUOTE, Some(original_ref), None);
        let quote_ref = quote.derivation_ref();

        state.f_axioms_mut().insert(original);
        for _ in 0..4_096 {
            state.f_axioms_mut().insert(WrappedFormula::default_alloc());
        }
        let original = state
            .f_axioms_mut()
            .extract_entry(original_ref.source())
            .expect("stable entry id must resolve after formula-set growth");
        state.f_ax_archive_mut().insert(original);
        for _ in 0..4_096 {
            state
                .f_ax_archive_mut()
                .insert(WrappedFormula::default_alloc());
        }
        state.f_axioms_mut().insert(quote);

        assert_eq!(
            state
                .proof_formula_by_derivation_ref(original_ref)
                .map(WrappedFormula::entry_id),
            Some(original_ref.source())
        );
        let quote = state
            .proof_formula_by_derivation_ref(quote_ref)
            .expect("flat-copy source must remain independently addressable");
        assert_eq!(quote.entry_id(), quote_ref.source());
        assert_eq!(
            state.proof_object_first_formula(quote).entry_id(),
            original_ref.source()
        );
    }

    #[test]
    fn proof_state_records_formula_extraction_roots() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let formula = wrapped_formula(&mut state, "formula_extraction_root");
        let formula_ident = formula.ident();

        assert!(state.extract_formula_roots().is_empty());
        state.push_extract_formula_root(formula);

        assert_eq!(state.extract_formula_roots().len(), 1);
        assert_eq!(state.extract_formula_roots()[0].ident(), formula_ident);
    }

    #[test]
    fn proof_state_parent_liveness_treats_matching_dead_archive_as_dead() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let mut live = simple_clause(&mut state, "parent_live_copy", 20_002);
        live.set_csscpa_source(7);
        let mut dead = simple_clause(&mut state, "parent_dead_copy", 20_002);
        dead.set_csscpa_source(7);
        dead.set_prop(CP_IS_DEAD);
        state.unprocessed_mut().insert(live);
        state.archive_mut().insert(dead);

        assert!(state
            .clause_by_derivation_ref(ClauseDerivationRef::new(20_002, 7))
            .is_some());
        assert!(state.clause_parent_is_dead(DerivationParentRef::Clause(
            ClauseDerivationRef::new(20_002, 7)
        )));
    }

    #[test]
    fn proof_state_mark_proof_clause_ancestors_sets_reachable_clause_prop() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let mut source = simple_clause(&mut state, "proof_mark_source", 20_003);
        source.set_csscpa_source(11);
        let mut selected = simple_clause(&mut state, "proof_mark_selected", 20_004);
        selected.set_csscpa_source(12);
        clause_push_derivation(&mut selected, DC_CNF_QUOTE, Some(&source), None);
        clause_push_derivation(&mut selected, DC_CNF_EVAL_GC, None, None);
        let mut root = Clause::alloc(EqnList::new());
        clause_push_derivation(&mut root, DC_EQ_RES, Some(&selected), None);

        state.ax_archive_mut().insert(source);
        state.archive_mut().insert(selected);

        assert_eq!(state.mark_proof_clause_ancestors(&root), 2);
        assert!(state
            .ax_archive()
            .find_by_id(20_003)
            .unwrap()
            .query_prop(CP_IS_PROOF_CLAUSE));
        assert!(state
            .archive()
            .find_by_id(20_004)
            .unwrap()
            .query_prop(CP_IS_PROOF_CLAUSE));
        assert_eq!(
            state.analyse_gc(),
            ProofStateGcAnalysis {
                clause_count: 2,
                given_count: 1,
                used_given_count: 1,
            }
        );
        assert_eq!(state.mark_proof_clause_ancestors(&root), 0);
    }

    #[test]
    fn proof_state_mark_proof_clause_ancestors_prefers_archive_copy_for_duplicate_refs() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let original = simple_clause(&mut state, "proof_mark_duplicate_original", 20_005);
        let mut selected_copy = simple_clause(&mut state, "proof_mark_duplicate_selected", 20_005);
        clause_push_derivation(&mut selected_copy, DC_CNF_EVAL_GC, None, None);
        let mut root = Clause::alloc(EqnList::new());
        clause_push_derivation(&mut root, DC_EQ_RES, Some(&selected_copy), None);

        state.ax_archive_mut().insert(original);
        state.archive_mut().insert(selected_copy);

        assert_eq!(state.mark_proof_clause_ancestors(&root), 1);
        assert!(!state
            .ax_archive()
            .find_by_id(20_005)
            .unwrap()
            .query_prop(CP_IS_PROOF_CLAUSE));
        assert!(state
            .archive()
            .find_by_id(20_005)
            .unwrap()
            .query_prop(CP_IS_PROOF_CLAUSE));
        assert_eq!(
            state.analyse_gc(),
            ProofStateGcAnalysis {
                clause_count: 2,
                given_count: 1,
                used_given_count: 1,
            }
        );
    }

    #[test]
    fn proof_state_mark_proof_clause_ancestors_follows_signature_ac_axioms() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let first = simple_clause(&mut state, "proof_mark_ac_first", 20_050);
        let second = simple_clause(&mut state, "proof_mark_ac_second", 20_051);
        let first_ref = ClauseDerivationRef::from(&first);
        let second_ref = ClauseDerivationRef::from(&second);
        let mut root = Clause::alloc(EqnList::new());
        clause_push_ac_res_derivation(&mut root, 2);

        state.terms_mut().signature_mut().push_ac_axiom(first_ref);
        state.terms_mut().signature_mut().push_ac_axiom(second_ref);
        state.axioms_mut().insert(first);
        state.archive_mut().insert(second);

        assert_eq!(state.mark_proof_clause_ancestors(&root), 2);
        assert!(state
            .axioms()
            .find_by_id(20_050)
            .unwrap()
            .query_prop(CP_IS_PROOF_CLAUSE));
        assert!(state
            .archive()
            .find_by_id(20_051)
            .unwrap()
            .query_prop(CP_IS_PROOF_CLAUSE));
    }

    #[test]
    fn proof_state_proof_object_analysis_counts_reachable_clause_steps() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let mut source = simple_clause(&mut state, "proof_analysis_source", 20_006);
        source.set_csscpa_source(21);
        let mut selected = simple_clause(&mut state, "proof_analysis_selected", 20_007);
        selected.set_csscpa_source(22);
        clause_push_derivation(&mut selected, DC_CNF_QUOTE, Some(&source), None);
        clause_push_derivation(&mut selected, DC_CNF_EVAL_GC, None, None);
        let mut root = Clause::alloc(EqnList::new());
        clause_push_derivation(&mut root, DC_EQ_RES, Some(&selected), None);

        state.axioms_mut().insert(source);
        state.archive_mut().insert(selected);

        assert_eq!(
            state.proof_object_analysis_for_roots([&root]),
            ProofObjectAnalysis {
                clause_step_count: 3,
                formula_step_count: 0,
                clause_conjecture_count: 0,
                formula_conjecture_count: 0,
                initial_clause_count: 1,
                initial_formula_count: 0,
                generating_inference_count: 1,
                simplifying_inference_count: 0,
            }
        );
    }

    #[test]
    fn proof_state_proof_object_analysis_follows_quote_source_for_duplicate_refs() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let original = simple_clause(&mut state, "proof_analysis_duplicate_original", 20_008);
        let mut selected_copy =
            simple_clause(&mut state, "proof_analysis_duplicate_selected", 20_008);
        clause_push_derivation(&mut selected_copy, DC_CNF_QUOTE, Some(&original), None);
        clause_push_derivation(&mut selected_copy, DC_CNF_EVAL_GC, None, None);
        let mut root = Clause::alloc(EqnList::new());
        clause_push_derivation(&mut root, DC_EQ_RES, Some(&selected_copy), None);

        state.axioms_mut().insert(original);
        state.archive_mut().insert(selected_copy);

        assert_eq!(
            state.proof_object_analysis_for_roots([&root]),
            ProofObjectAnalysis {
                clause_step_count: 3,
                formula_step_count: 0,
                clause_conjecture_count: 0,
                formula_conjecture_count: 0,
                initial_clause_count: 1,
                initial_formula_count: 0,
                generating_inference_count: 1,
                simplifying_inference_count: 0,
            }
        );
    }

    #[test]
    fn proof_clause_lookup_distinguishes_requeued_generation_from_archive() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let archived = simple_clause(&mut state, "proof_generation_archived", 20_049);
        let archived_ref = ClauseDerivationRef::from(&archived);
        let mut requeued = archived.clone();
        requeued.refresh_derivation_generation();
        let requeued_ref = ClauseDerivationRef::from(&requeued);

        state.archive_mut().insert(archived);
        state.processed_neg_units_mut().insert(requeued);

        assert_ne!(archived_ref, requeued_ref);
        assert_eq!(
            state
                .proof_clause_by_derivation_ref(archived_ref)
                .map(ClauseDerivationRef::from),
            Some(archived_ref)
        );
        assert_eq!(
            state
                .proof_clause_by_derivation_ref(requeued_ref)
                .map(ClauseDerivationRef::from),
            Some(requeued_ref)
        );
    }

    #[test]
    fn proof_clause_lookup_checks_all_exact_refs_before_legacy_id_fallback() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let current = simple_clause(&mut state, "proof_generation_current", 20_054);
        let current_ref = ClauseDerivationRef::from(&current);
        let mut archived_copy = current.clone();
        archived_copy.refresh_derivation_generation();

        state.ax_archive_mut().insert(archived_copy);
        state.axioms_mut().insert(current);

        let expected = state.axioms().find_by_derivation_ref(current_ref).unwrap();
        assert!(std::ptr::eq(
            state.proof_clause_by_derivation_ref(current_ref).unwrap(),
            expected
        ));
        assert!(std::ptr::eq(
            state
                .proof_quote_source_by_derivation_ref(current_ref)
                .unwrap(),
            expected
        ));
    }

    #[test]
    fn proof_state_proof_object_analysis_follows_signature_ac_axioms() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let first = simple_clause(&mut state, "proof_analysis_ac_first", 20_052);
        let second = simple_clause(&mut state, "proof_analysis_ac_second", 20_053);
        let first_ref = ClauseDerivationRef::from(&first);
        let second_ref = ClauseDerivationRef::from(&second);
        let mut root = Clause::alloc(EqnList::new());
        clause_push_ac_res_derivation(&mut root, 2);

        state.terms_mut().signature_mut().push_ac_axiom(first_ref);
        state.terms_mut().signature_mut().push_ac_axiom(second_ref);
        state.axioms_mut().insert(first);
        state.axioms_mut().insert(second);

        assert_eq!(
            state.proof_object_analysis_for_roots([&root]),
            ProofObjectAnalysis {
                clause_step_count: 3,
                formula_step_count: 0,
                clause_conjecture_count: 0,
                formula_conjecture_count: 0,
                initial_clause_count: 2,
                initial_formula_count: 0,
                generating_inference_count: 0,
                simplifying_inference_count: 1,
            }
        );
    }

    #[test]
    fn proof_state_proof_object_analysis_counts_formula_parents() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let mut formula = wrapped_formula(&mut state, "proof_analysis_formula_parent");
        formula.set_tptp_type(CP_TYPE_CONJECTURE);
        let formula_ref = FormulaDerivationRef::new(formula.ident());
        let mut root = Clause::alloc(EqnList::new());
        clause_push_formula_derivation(&mut root, DC_FOF_QUOTE, Some(formula_ref), None);

        state.f_ax_archive_mut().insert(formula);

        assert_eq!(
            state.proof_object_analysis_for_roots([&root]),
            ProofObjectAnalysis {
                clause_step_count: 1,
                formula_step_count: 1,
                clause_conjecture_count: 0,
                formula_conjecture_count: 1,
                initial_clause_count: 1,
                initial_formula_count: 1,
                generating_inference_count: 0,
                simplifying_inference_count: 0,
            }
        );
    }

    #[test]
    fn proof_state_proof_object_analysis_counts_formula_archive_parents() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let mut formula = wrapped_formula(&mut state, "proof_analysis_f_archive_parent");
        formula.set_tptp_type(CP_TYPE_CONJECTURE);
        let formula_ref = FormulaDerivationRef::new(formula.ident());
        let mut root = Clause::alloc(EqnList::new());
        clause_push_formula_derivation(&mut root, DC_FOF_QUOTE, Some(formula_ref), None);

        state.f_archive_mut().insert(formula);

        assert_eq!(
            state.proof_object_analysis_for_roots([&root]),
            ProofObjectAnalysis {
                clause_step_count: 1,
                formula_step_count: 1,
                clause_conjecture_count: 0,
                formula_conjecture_count: 1,
                initial_clause_count: 1,
                initial_formula_count: 1,
                generating_inference_count: 0,
                simplifying_inference_count: 0,
            }
        );
    }

    #[test]
    fn proof_state_proof_object_analysis_follows_formula_quote_source() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let mut original = wrapped_formula(&mut state, "proof_analysis_original_formula");
        original.set_tptp_type(CP_TYPE_CONJECTURE);
        let original_ref = FormulaDerivationRef::new(original.ident());
        let mut quote = wrapped_formula(&mut state, "proof_analysis_formula_quote");
        quote.set_tptp_type(CP_TYPE_AXIOM);
        quote.push_formula_derivation(DC_FOF_QUOTE, Some(original_ref), None);
        let quote_ref = FormulaDerivationRef::new(quote.ident());
        let mut root = Clause::alloc(EqnList::new());
        clause_push_formula_derivation(&mut root, DC_FOF_QUOTE, Some(quote_ref), None);

        state.f_ax_archive_mut().insert(original);
        state.f_axioms_mut().insert(quote);

        assert_eq!(
            state.proof_object_analysis_for_roots([&root]),
            ProofObjectAnalysis {
                clause_step_count: 1,
                formula_step_count: 1,
                clause_conjecture_count: 0,
                formula_conjecture_count: 1,
                initial_clause_count: 1,
                initial_formula_count: 1,
                generating_inference_count: 0,
                simplifying_inference_count: 0,
            }
        );
    }

    #[test]
    fn proof_state_proof_object_analysis_accepts_formula_roots() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let mut formula = wrapped_formula(&mut state, "proof_analysis_formula_root");
        formula.set_tptp_type(CP_TYPE_CONJECTURE);

        assert_eq!(
            state.proof_object_analysis_for_mixed_roots([], [&formula]),
            ProofObjectAnalysis {
                clause_step_count: 0,
                formula_step_count: 1,
                clause_conjecture_count: 0,
                formula_conjecture_count: 1,
                initial_clause_count: 0,
                initial_formula_count: 1,
                generating_inference_count: 0,
                simplifying_inference_count: 0,
            }
        );
    }

    #[test]
    fn proof_state_proof_object_graph_collects_reachable_clause_edges() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let mut source = simple_clause(&mut state, "proof_graph_source", 20_009);
        source.set_csscpa_source(31);
        let mut selected = simple_clause(&mut state, "proof_graph_selected", 20_010);
        selected.set_csscpa_source(32);
        clause_push_derivation(&mut selected, DC_CNF_QUOTE, Some(&source), None);
        clause_push_derivation(&mut selected, DC_CNF_EVAL_GC, None, None);
        let mut root = Clause::alloc(EqnList::new());
        root.set_ident(20_011);
        clause_push_derivation(&mut root, DC_EQ_RES, Some(&selected), None);

        state.axioms_mut().insert(source);
        state.archive_mut().insert(selected);

        let graph = state.proof_object_graph_for_roots([&root]);
        assert_eq!(
            graph
                .clauses
                .iter()
                .map(|clause| clause.ident())
                .collect::<Vec<_>>(),
            vec![20_011, 20_010, 20_009]
        );
        assert_eq!(graph.root_indices, vec![0]);
        assert_eq!(
            graph.edges,
            vec![
                ProofObjectGraphEdge {
                    parent_index: 1,
                    child_index: 0,
                },
                ProofObjectGraphEdge {
                    parent_index: 2,
                    child_index: 1,
                },
            ]
        );
    }

    #[test]
    fn proof_state_proof_object_graph_uses_quote_source_for_duplicate_refs() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let original = simple_clause(&mut state, "proof_graph_duplicate_original", 20_012);
        let mut selected_copy = simple_clause(&mut state, "proof_graph_duplicate_selected", 20_012);
        selected_copy.set_ident(20_013);
        clause_push_derivation(&mut selected_copy, DC_CNF_QUOTE, Some(&original), None);
        clause_push_derivation(&mut selected_copy, DC_CNF_EVAL_GC, None, None);
        let mut root = Clause::alloc(EqnList::new());
        root.set_ident(20_014);
        clause_push_derivation(&mut root, DC_EQ_RES, Some(&selected_copy), None);

        state.axioms_mut().insert(original);
        state.archive_mut().insert(selected_copy);

        let graph = state.proof_object_graph_for_roots([&root]);
        assert_eq!(graph.root_indices, vec![0]);
        assert_eq!(
            graph.edges,
            vec![
                ProofObjectGraphEdge {
                    parent_index: 1,
                    child_index: 0,
                },
                ProofObjectGraphEdge {
                    parent_index: 2,
                    child_index: 1,
                },
            ]
        );
    }

    #[test]
    fn proof_state_proof_object_graph_collapses_mutated_active_quote_like_c() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let original = simple_clause(&mut state, "proof_graph_ax_archive_original", 20_015);
        let mut active_quote = simple_clause(&mut state, "proof_graph_active_quote", 20_015);
        clause_push_derivation(&mut active_quote, DC_CNF_QUOTE, Some(&original), None);

        state.ax_archive_mut().insert(original);
        state.axioms_mut().insert(active_quote);

        let root = state.axioms().find_by_id(20_015).unwrap();
        let graph = state.proof_object_graph_for_roots([root]);

        assert_eq!(
            graph
                .clauses
                .iter()
                .map(|clause| clause.derivation().is_some())
                .collect::<Vec<_>>(),
            vec![false]
        );
        assert_eq!(graph.root_indices, vec![0]);
        assert!(graph.edges.is_empty());
    }

    #[test]
    fn proof_state_proof_object_graph_records_distinct_requested_roots() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let first_root = simple_clause(&mut state, "proof_graph_first_root", 20_016);
        let parent = simple_clause(&mut state, "proof_graph_second_parent", 20_017);
        let mut second_root = Clause::alloc(EqnList::new());
        second_root.set_ident(20_018);
        clause_push_derivation(&mut second_root, DC_EQ_RES, Some(&parent), None);

        state.axioms_mut().insert(parent);

        let graph = state.proof_object_graph_for_roots([&first_root, &second_root, &first_root]);
        assert_eq!(
            graph
                .clauses
                .iter()
                .map(|clause| clause.ident())
                .collect::<Vec<_>>(),
            vec![20_016, 20_018, 20_017]
        );
        assert_eq!(graph.root_indices, vec![0, 1]);
        assert_eq!(
            graph.edges,
            vec![ProofObjectGraphEdge {
                parent_index: 2,
                child_index: 1,
            }]
        );
    }

    #[test]
    fn proof_state_proof_object_graph_follows_signature_ac_axioms() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let first = simple_clause(&mut state, "proof_graph_ac_first", 20_054);
        let second = simple_clause(&mut state, "proof_graph_ac_second", 20_055);
        let first_ref = ClauseDerivationRef::from(&first);
        let second_ref = ClauseDerivationRef::from(&second);
        let mut root = Clause::alloc(EqnList::new());
        root.set_ident(20_056);
        clause_push_ac_res_derivation(&mut root, 2);

        state.terms_mut().signature_mut().push_ac_axiom(first_ref);
        state.terms_mut().signature_mut().push_ac_axiom(second_ref);
        state.axioms_mut().insert(first);
        state.axioms_mut().insert(second);

        let graph = state.proof_object_graph_for_roots([&root]);
        assert_eq!(
            graph
                .clauses
                .iter()
                .map(|clause| clause.ident())
                .collect::<Vec<_>>(),
            vec![20_056, 20_055, 20_054]
        );
        assert_eq!(graph.root_indices, vec![0]);
        assert_eq!(
            graph.edges,
            vec![
                ProofObjectGraphEdge {
                    parent_index: 1,
                    child_index: 0,
                },
                ProofObjectGraphEdge {
                    parent_index: 2,
                    child_index: 0,
                },
            ]
        );
    }

    #[test]
    fn proof_state_proof_object_graph_preserves_ac_axiom_quote_node_like_c() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let original = simple_clause(&mut state, "proof_graph_ac_original", 20_057);
        let mut selected = simple_clause(&mut state, "proof_graph_ac_selected", 20_058);
        clause_push_derivation(&mut selected, DC_CNF_QUOTE, Some(&original), None);
        let selected_ref = ClauseDerivationRef::from(&selected);
        let mut root = Clause::alloc(EqnList::new());
        root.set_ident(20_059);
        clause_push_ac_res_derivation(&mut root, 1);

        state
            .terms_mut()
            .signature_mut()
            .push_ac_axiom(selected_ref);
        state.axioms_mut().insert(original);
        state.archive_mut().insert(selected);

        let graph = state.proof_object_graph_for_roots([&root]);
        assert_eq!(
            graph
                .clauses
                .iter()
                .map(|clause| clause.ident())
                .collect::<Vec<_>>(),
            vec![20_059, 20_058, 20_057]
        );
        assert_eq!(
            graph.edges,
            vec![
                ProofObjectGraphEdge {
                    parent_index: 1,
                    child_index: 0,
                },
                ProofObjectGraphEdge {
                    parent_index: 2,
                    child_index: 1,
                },
            ]
        );
    }

    #[test]
    fn proof_state_proof_object_graph_collects_formula_parent_nodes() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let formula = wrapped_formula(&mut state, "proof_graph_formula_parent");
        let formula_ref = FormulaDerivationRef::new(formula.ident());
        let mut root = Clause::alloc(EqnList::new());
        root.set_ident(20_019);
        clause_push_formula_derivation(&mut root, DC_FOF_QUOTE, Some(formula_ref), None);

        state.f_ax_archive_mut().insert(formula);

        let graph = state.proof_object_graph_for_roots([&root]);
        assert_eq!(
            graph
                .clauses
                .iter()
                .map(|clause| clause.ident())
                .collect::<Vec<_>>(),
            vec![20_019]
        );
        assert_eq!(
            graph
                .formulas
                .iter()
                .map(|formula| formula.ident())
                .collect::<Vec<_>>(),
            vec![formula_ref.ident()]
        );
        assert_eq!(graph.edges, Vec::new());
        assert_eq!(
            graph.mixed_edges,
            vec![ProofObjectGraphMixedEdge {
                parent: ProofObjectGraphNode::Formula(0),
                child: ProofObjectGraphNode::Clause(0),
            }]
        );
    }

    #[test]
    fn proof_state_proof_object_graph_collects_formula_archive_parent_nodes() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let formula = wrapped_formula(&mut state, "proof_graph_f_archive_parent");
        let formula_ref = FormulaDerivationRef::new(formula.ident());
        let mut root = Clause::alloc(EqnList::new());
        root.set_ident(20_022);
        clause_push_formula_derivation(&mut root, DC_FOF_QUOTE, Some(formula_ref), None);

        state.f_archive_mut().insert(formula);

        let graph = state.proof_object_graph_for_roots([&root]);
        assert_eq!(
            graph
                .clauses
                .iter()
                .map(|clause| clause.ident())
                .collect::<Vec<_>>(),
            vec![20_022]
        );
        assert_eq!(
            graph
                .formulas
                .iter()
                .map(|formula| formula.ident())
                .collect::<Vec<_>>(),
            vec![formula_ref.ident()]
        );
        assert_eq!(
            graph.mixed_edges,
            vec![ProofObjectGraphMixedEdge {
                parent: ProofObjectGraphNode::Formula(0),
                child: ProofObjectGraphNode::Clause(0),
            }]
        );
    }

    #[test]
    fn proof_state_proof_object_graph_records_formula_roots() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let formula = wrapped_formula(&mut state, "proof_graph_formula_root");
        let formula_ref = FormulaDerivationRef::new(formula.ident());

        let graph = state.proof_object_graph_for_mixed_roots([], [&formula]);

        assert_eq!(graph.root_indices, Vec::<usize>::new());
        assert_eq!(graph.formula_root_indices, vec![0]);
        assert_eq!(
            graph
                .formulas
                .iter()
                .map(|formula| formula.ident())
                .collect::<Vec<_>>(),
            vec![formula_ref.ident()]
        );
        assert_eq!(graph.mixed_edges, Vec::new());
    }

    #[test]
    fn proof_state_proof_object_graph_uses_formula_quote_source() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let original = wrapped_formula(&mut state, "proof_graph_formula_original");
        let original_ref = FormulaDerivationRef::new(original.ident());
        let mut quote = wrapped_formula(&mut state, "proof_graph_formula_quote");
        quote.push_formula_derivation(DC_FOF_QUOTE, Some(original_ref), None);
        let quote_ref = FormulaDerivationRef::new(quote.ident());
        let mut root = Clause::alloc(EqnList::new());
        root.set_ident(20_020);
        clause_push_formula_derivation(&mut root, DC_FOF_QUOTE, Some(quote_ref), None);

        state.f_ax_archive_mut().insert(original);
        state.f_axioms_mut().insert(quote);

        let graph = state.proof_object_graph_for_roots([&root]);
        assert_eq!(
            graph
                .formulas
                .iter()
                .map(|formula| formula.ident())
                .collect::<Vec<_>>(),
            vec![original_ref.ident()]
        );
        assert_eq!(
            graph.mixed_edges,
            vec![ProofObjectGraphMixedEdge {
                parent: ProofObjectGraphNode::Formula(0),
                child: ProofObjectGraphNode::Clause(0),
            }]
        );
    }

    #[test]
    fn proof_state_proof_object_graph_collects_formula_to_formula_edges() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let definition = wrapped_formula(&mut state, "proof_graph_formula_definition");
        let definition_ref = FormulaDerivationRef::new(definition.ident());
        let mut rewritten = wrapped_formula(&mut state, "proof_graph_formula_rewritten");
        rewritten.push_formula_derivation(DC_APPLY_DEF, Some(definition_ref), None);
        let rewritten_ref = FormulaDerivationRef::new(rewritten.ident());
        let mut root = Clause::alloc(EqnList::new());
        root.set_ident(20_021);
        clause_push_formula_derivation(&mut root, DC_FOF_QUOTE, Some(rewritten_ref), None);

        state.f_ax_archive_mut().insert(definition);
        state.f_axioms_mut().insert(rewritten);

        let graph = state.proof_object_graph_for_roots([&root]);
        assert_eq!(
            graph
                .formulas
                .iter()
                .map(|formula| formula.ident())
                .collect::<Vec<_>>(),
            vec![rewritten_ref.ident(), definition_ref.ident()]
        );
        assert_eq!(
            graph.mixed_edges,
            vec![
                ProofObjectGraphMixedEdge {
                    parent: ProofObjectGraphNode::Formula(0),
                    child: ProofObjectGraphNode::Clause(0),
                },
                ProofObjectGraphMixedEdge {
                    parent: ProofObjectGraphNode::Formula(1),
                    child: ProofObjectGraphNode::Formula(0),
                },
            ]
        );
    }

    #[test]
    fn proof_state_alloc_applies_free_symbol_distinct_mask() {
        let default_state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        assert_eq!(
            default_state.terms().signature().distinct_props(),
            FP_DISTINCT_PROP
        );

        let free_state = proof_state_alloc(FP_DISTINCT_PROP).unwrap();
        assert_eq!(
            free_state.terms().signature().distinct_props(),
            FP_IGNORE_PROPS
        );
    }

    #[test]
    fn proof_state_cardinalities_follow_c_macros() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let rule = simple_clause(&mut state, "card_rule", 10);
        let equation = simple_clause(&mut state, "card_eqn", 11);
        let negative = simple_clause(&mut state, "card_neg", 12);
        let non_unit = simple_clause(&mut state, "card_nonunit", 13);
        let unprocessed = simple_clause(&mut state, "card_unproc", 14);
        let axiom = simple_clause(&mut state, "card_axiom", 15);
        let formula_axiom = wrapped_formula(&mut state, "card_formula_axiom");

        state.processed_pos_rules_mut().insert(rule);
        state.processed_pos_eqns_mut().insert(equation);
        state.processed_neg_units_mut().insert(negative);
        state.processed_non_units_mut().insert(non_unit);
        state.unprocessed_mut().insert(unprocessed);
        state.axioms_mut().insert(axiom);
        state.f_axioms_mut().insert(formula_axiom);

        assert_eq!(state.processed_cardinality(), 4);
        assert_eq!(state.unprocessed_cardinality(), 1);
        assert_eq!(state.cardinality(), 5);
        assert_eq!(state.axiom_count(), 2);
    }

    #[test]
    fn proof_state_statistics_reports_formula_archive_cardinality() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let archived = wrapped_formula(&mut state, "stats_archived_formula");
        state.f_archive_mut().insert(archived);

        assert!(state
            .statistics_string(false, false)
            .contains("Current number of archived formulas  : 1"));
    }

    #[test]
    fn proof_state_cached_rewrite_steps_follow_c_max_correction() {
        assert_eq!(cached_rewrite_steps(7, 3), 4);
        assert_eq!(cached_rewrite_steps(3, 7), 0);
    }

    #[test]
    fn proof_state_generated_clause_statistics_use_c_signed_display() {
        assert_eq!(generated_clause_statistics_count(7, 3), 4);
        assert_eq!(generated_clause_statistics_count(3, 7), -4);
    }

    #[test]
    fn proof_state_generated_literal_statistics_use_c_signed_display() {
        assert_eq!(generated_literal_statistics_count(7, 3), 4);
        assert_eq!(generated_literal_statistics_count(3, 7), -4);
    }

    #[test]
    fn proof_state_statistics_reports_detailed_generated_literals_and_demod_index_attempts() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        state.statistics_mut().generated_lit_count = 7;
        state.statistics_mut().backward_rewritten_lit_count = 3;
        let included = simple_clause(&mut state, "stats_included_unshared", 70_001);
        state.unprocessed_mut().insert(included);
        let included_unshared = state.unprocessed().term_nodes(state.terms());
        let excluded = simple_clause(&mut state, "stats_excluded_unshared", 70_002);
        state.archive_mut().insert(excluded);
        let included_plus_excluded_unshared =
            included_unshared + state.archive().term_nodes(state.terms());
        let shared_term_nodes = state.terms().term_nodes();

        state
            .processed_pos_rules()
            .record_demod_index_search_attempt();
        state
            .processed_pos_eqns()
            .record_demod_index_search_attempt();
        state
            .processed_pos_eqns()
            .record_demod_index_search_attempt();
        state
            .processed_pos_rules()
            .record_demod_index_nodes_visited(5);
        state
            .processed_pos_eqns()
            .record_demod_index_nodes_visited(11);

        let ordinary_statistics = state.statistics_string(false, false);
        assert!(!ordinary_statistics.contains("Total literals in generated clauses"));
        assert!(!ordinary_statistics.contains("Shared term nodes"));
        assert!(!ordinary_statistics.contains("...corresponding unshared nodes"));
        assert!(!ordinary_statistics.contains("Match attempts with oriented units"));
        assert!(!ordinary_statistics.contains("Match attempts with unoriented units"));
        assert!(!ordinary_statistics.contains("PDT nodes visited"));

        let detailed_statistics = state.statistics_string(false, true);
        assert!(detailed_statistics.contains("Total literals in generated clauses  : 4"));
        assert!(detailed_statistics.contains(&format!(
            "Shared term nodes                    : {shared_term_nodes}"
        )));
        assert!(detailed_statistics.contains(&format!(
            "...corresponding unshared nodes      : {included_unshared}"
        )));
        assert!(!detailed_statistics.contains(&format!(
            "...corresponding unshared nodes      : {included_plus_excluded_unshared}"
        )));
        assert!(detailed_statistics.contains("Match attempts with oriented units   : 1"));
        assert!(detailed_statistics.contains("Match attempts with unoriented units : 2"));
        #[cfg(feature = "measure-expensive")]
        {
            assert!(detailed_statistics.contains("Oriented PDT nodes visited           : 5"));
            assert!(detailed_statistics.contains("Unoriented PDT nodes visited         : 11"));
        }
        #[cfg(not(feature = "measure-expensive"))]
        {
            assert!(!detailed_statistics.contains("PDT nodes visited"));
        }
    }

    #[test]
    fn proof_state_process_distinct_archives_and_expands_in_c_stack_order() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let first = {
            let bank = state.terms_mut();
            let a = typed_const(bank, "distinct_first_a");
            let b = typed_const(bank, "distinct_first_b");
            WrappedFormula::wt_formula_alloc(distinct_formula(bank, &[a, b]))
        };
        let first_ident = first.ident();
        let first_ref = first.derivation_ref();
        let middle = wrapped_formula(&mut state, "distinct_middle");
        let middle_ident = middle.ident();
        let second = {
            let bank = state.terms_mut();
            let a = typed_const(bank, "distinct_second_a");
            let b = typed_const(bank, "distinct_second_b");
            let c = typed_const(bank, "distinct_second_c");
            WrappedFormula::wt_formula_alloc(distinct_formula(bank, &[a, b, c]))
        };
        let second_ident = second.ident();
        let second_ref = second.derivation_ref();

        state.f_axioms_mut().insert(first);
        state.f_axioms_mut().insert(middle);
        state.f_axioms_mut().insert(second);

        let result = state.process_distinct().unwrap();

        assert_eq!(result.distinct_formulas_processed, 2);
        assert_eq!(result.expanded_formula_sources, vec![second_ref, first_ref]);
        assert_eq!(
            result.formula_derivation_ops,
            vec![DC_EXPAND_DISTINCT, DC_EXPAND_DISTINCT]
        );
        assert_eq!(
            state
                .f_ax_archive()
                .iter()
                .map(WrappedFormula::ident)
                .collect::<Vec<_>>(),
            vec![second_ident, first_ident]
        );

        let active = state.f_axioms().iter().collect::<Vec<_>>();
        assert_eq!(active.len(), 3);
        assert_eq!(active[0].ident(), middle_ident);
        assert_ne!(active[1].ident(), second_ident);
        assert_eq!(
            active[1].derivation_entries(),
            &[
                DerivationEntry::Operation(DC_EXPAND_DISTINCT),
                DerivationEntry::FormulaParent(second_ref),
            ]
        );
        assert_eq!(
            active[1].formula().f_code(),
            state.terms().signature().and_code()
        );
        assert_ne!(active[2].ident(), first_ident);
        assert_eq!(
            active[2].derivation_entries(),
            &[
                DerivationEntry::Operation(DC_EXPAND_DISTINCT),
                DerivationEntry::FormulaParent(first_ref),
            ]
        );
        assert_eq!(
            active[2].formula().f_code(),
            state.terms().signature().neqn_code()
        );
    }

    #[test]
    fn proof_state_collect_term_garbage_marks_registered_formula_roots() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let (formula_arg, formula_term, definition_arg, definition_term, dropped) = {
            let bank = state.terms_mut();
            let formula_arg = typed_const(bank, "registered_gc_formula_arg");
            let formula_term = typed_unary(bank, "registered_gc_formula", &formula_arg);
            let definition_arg = typed_const(bank, "registered_gc_definition_arg");
            let definition_term =
                typed_unary(bank, "registered_gc_definition_formula", &definition_arg);
            let dropped = typed_const(bank, "registered_gc_dropped");
            (
                formula_arg,
                formula_term,
                definition_arg,
                definition_term,
                dropped,
            )
        };

        state
            .f_axioms_mut()
            .insert(WrappedFormula::wt_formula_alloc(formula_term.clone()));
        state
            .definition_formula_archive_mut()
            .insert(WrappedFormula::wt_formula_alloc(definition_term.clone()));

        assert_eq!(state.collect_term_garbage(), 1);
        assert!(state.terms_mut().find(&formula_arg).is_some());
        assert!(state.terms_mut().find(&formula_term).is_some());
        assert!(state.terms_mut().find(&definition_arg).is_some());
        assert!(state.terms_mut().find(&definition_term).is_some());
        assert!(state.terms_mut().find(&dropped).is_none());
    }

    #[test]
    fn proof_state_analyse_gc_counts_c_clause_domains_only() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let ax_archive = eval_gc_clause(&mut state, "gc_ax_archive", 40, true);
        state.ax_archive_mut().insert(ax_archive);
        let pos_rule = eval_gc_clause(&mut state, "gc_pos_rule", 41, false);
        state.processed_pos_rules_mut().insert(pos_rule);
        let plain_pos_eqn = simple_clause(&mut state, "gc_plain_pos_eqn", 42);
        state.processed_pos_eqns_mut().insert(plain_pos_eqn);
        let neg_unit = eval_gc_clause(&mut state, "gc_neg_unit", 43, true);
        state.processed_neg_units_mut().insert(neg_unit);
        let non_unit = eval_gc_clause(&mut state, "gc_non_unit", 44, false);
        state.processed_non_units_mut().insert(non_unit);
        let archive = eval_gc_clause(&mut state, "gc_archive", 45, true);
        state.archive_mut().insert(archive);
        let ignored_axiom = eval_gc_clause(&mut state, "gc_ignored_axiom", 46, true);
        state.axioms_mut().insert(ignored_axiom);
        let ignored_unprocessed = eval_gc_clause(&mut state, "gc_ignored_unprocessed", 47, true);
        state.unprocessed_mut().insert(ignored_unprocessed);
        let ignored_tmp = eval_gc_clause(&mut state, "gc_ignored_tmp", 48, true);
        state.tmp_store_mut().insert(ignored_tmp);
        let ignored_eval = eval_gc_clause(&mut state, "gc_ignored_eval", 49, true);
        state.eval_store_mut().insert(ignored_eval);
        let ignored_watch = eval_gc_clause(&mut state, "gc_ignored_watch", 50, true);
        state.watchlist_mut().unwrap().insert(ignored_watch);

        let expected = ProofStateGcAnalysis {
            clause_count: 6,
            given_count: 5,
            used_given_count: 3,
        };

        assert_eq!(state.analyse_gc(), expected);
        assert_eq!(state.statistics().gc_count, 5);
        assert_eq!(state.statistics().gc_used_count, 3);

        assert_eq!(state.analyse_gc(), expected);
        assert_eq!(state.statistics().gc_count, 10);
        assert_eq!(state.statistics().gc_used_count, 6);
    }

    #[test]
    fn proof_state_pick_training_examples_uses_eval_gc_and_proof_clause_prop() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let ax_pos = eval_gc_clause(&mut state, "train_ax_pos", 60, true);
        state.ax_archive_mut().insert(ax_pos);
        let rule_neg = eval_gc_clause(&mut state, "train_rule_neg", 61, false);
        state.processed_pos_rules_mut().insert(rule_neg);
        let plain = simple_clause(&mut state, "train_plain", 62);
        state.processed_pos_eqns_mut().insert(plain);
        let neg_pos = eval_gc_clause(&mut state, "train_neg_pos", 63, true);
        state.processed_neg_units_mut().insert(neg_pos);
        let nonunit_neg = eval_gc_clause(&mut state, "train_nonunit_neg", 64, false);
        state.processed_non_units_mut().insert(nonunit_neg);
        let archive_pos = eval_gc_clause(&mut state, "train_archive_pos", 65, true);
        state.archive_mut().insert(archive_pos);
        let ignored = eval_gc_clause(&mut state, "train_ignored", 66, true);
        state.unprocessed_mut().insert(ignored);

        let examples = state.pick_training_examples();

        assert_eq!(
            examples
                .positive
                .iter()
                .map(|clause| clause.ident())
                .collect::<Vec<_>>(),
            vec![60, 63, 65]
        );
        assert_eq!(
            examples
                .negative
                .iter()
                .map(|clause| clause.ident())
                .collect::<Vec<_>>(),
            vec![61, 64]
        );
    }

    #[test]
    fn proof_state_print_preserves_c_section_order() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let rule = simple_clause(&mut state, "print_rule", 20);
        let equation = simple_clause(&mut state, "print_eqn", 21);
        let negative = simple_clause(&mut state, "print_neg", 22);
        let non_unit = simple_clause(&mut state, "print_nonunit", 23);
        let unprocessed = simple_clause(&mut state, "print_unproc", 24);

        let rule_print = clause_print_lop_format_string(state.terms(), &rule, true);
        let equation_print = clause_print_lop_format_string(state.terms(), &equation, true);
        let negative_print = clause_print_lop_format_string(state.terms(), &negative, true);
        let non_unit_print = clause_print_lop_format_string(state.terms(), &non_unit, true);
        let unprocessed_print = clause_print_lop_format_string(state.terms(), &unprocessed, true);

        state.processed_pos_rules_mut().insert(rule);
        state.processed_pos_eqns_mut().insert(equation);
        state.processed_neg_units_mut().insert(negative);
        state.processed_non_units_mut().insert(non_unit);
        state.unprocessed_mut().insert(unprocessed);

        assert_eq!(
            state.print_string(),
            format!(
                "\n% Processed positive unit clauses:\n{rule_print}\n{equation_print}\n\n\
                 % Processed negative unit clauses:\n{negative_print}\n\n\
                 % Processed non-unit clauses:\n{non_unit_print}\n\n\
                 % Unprocessed clauses:\n{unprocessed_print}\n"
            )
        );
    }

    #[test]
    fn proof_state_is_untyped_checks_processed_and_unprocessed_only() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let processed = simple_clause(&mut state, "untyped_proc", 20);
        let axiom_only = simple_clause(&mut state, "untyped_axiom", 21);

        state.processed_non_units_mut().insert(processed);
        state.axioms_mut().insert(axiom_only);

        assert!(state.is_untyped());
    }

    #[test]
    fn proof_state_reset_clause_sets_preserves_definition_store_and_fvi_state() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let axiom = nontrivial_clause(&mut state, "reset_axiom", 30);
        let processed = nontrivial_clause(&mut state, "reset_proc", 31);
        let unprocessed = nontrivial_clause(&mut state, "reset_unproc", 32);
        let watch = nontrivial_clause(&mut state, "reset_watch", 33);
        let def = nontrivial_clause(&mut state, "reset_def", 34);
        let formula_axiom = wrapped_formula(&mut state, "reset_formula_axiom");
        let formula_ax_archive = wrapped_formula(&mut state, "reset_formula_ax_archive");
        let formula_archive = wrapped_formula(&mut state, "reset_formula_archive");
        let definition_formula_archive =
            wrapped_formula(&mut state, "reset_definition_formula_archive");
        let definition_formula_archive_ident = definition_formula_archive.ident();
        let params = FvIndexParams::new(FvIndexType::AcFold, false, true, 9, 1);

        state.axioms_mut().insert(axiom);
        state.f_axioms_mut().insert(formula_axiom);
        state.f_ax_archive_mut().insert(formula_ax_archive);
        state.f_archive_mut().insert(formula_archive);
        state
            .definition_formula_archive_mut()
            .insert(definition_formula_archive);
        state.processed_non_units_mut().insert(processed);
        state.unprocessed_mut().insert(unprocessed);
        state.watchlist_mut().unwrap().insert(watch);
        state.definition_store_mut().insert(def);
        state.definition_formula_assocs_mut().insert(
            34,
            FormulaDerivationRef::new(definition_formula_archive_ident),
        );
        state.init_fvi_anchors(&params).unwrap();

        state.reset_clause_sets();

        assert!(state.fvi_initialized());
        assert_eq!(state.axioms().members(), 0);
        assert_eq!(state.f_axioms().cardinality(), 0);
        assert_eq!(state.f_ax_archive().cardinality(), 0);
        assert_eq!(state.f_archive().cardinality(), 1);
        assert_eq!(state.definition_formula_archive().cardinality(), 1);
        assert_eq!(state.processed_non_units().members(), 0);
        assert_eq!(state.unprocessed().members(), 0);
        assert_eq!(state.watchlist().unwrap().members(), 0);
        assert_eq!(state.definition_store().members(), 1);
        assert_eq!(state.definition_formula_assocs().len(), 1);
        assert_eq!(
            state
                .processed_non_units()
                .fv_anchor()
                .unwrap()
                .index()
                .clause_count(),
            0
        );
    }

    #[test]
    fn proof_state_init_fvi_anchors_installs_processed_watchlist_and_definition_store_anchors() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let axiom = nontrivial_clause(&mut state, "fvi_axiom", 40);
        let params = FvIndexParams::new(FvIndexType::AcFold, false, true, 9, 1);

        state.axioms_mut().insert(axiom);
        state.init_fvi_anchors(&params).unwrap();

        assert!(state.fvi_initialized());
        assert!(state.fvi_cspec().is_some());
        assert!(state.def_store_cspec().is_some());
        assert!(state.processed_non_units().fv_anchor().is_some());
        assert!(state.processed_pos_rules().fv_anchor().is_some());
        assert!(state.processed_pos_eqns().fv_anchor().is_some());
        assert!(state.processed_neg_units().fv_anchor().is_some());
        assert!(state.watchlist().unwrap().fv_anchor().is_some());
        assert!(state.definition_store().fv_anchor().is_some());
    }

    #[test]
    fn proof_state_init_fvi_anchors_omits_watchlist_after_discard() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let params = FvIndexParams::new(FvIndexType::AcFold, false, false, 9, 1);

        assert!(state.discard_watchlist().is_some());
        state.init_fvi_anchors(&params).unwrap();

        assert!(state.watchlist().is_none());
        assert!(state.processed_non_units().fv_anchor().is_some());
        assert!(state.definition_store().fv_anchor().is_some());
    }

    #[test]
    fn proof_state_load_watchlist_inline_marks_existing_watchlist() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let watch = nontrivial_clause(&mut state, "inline_watch", 50);

        state.watchlist_mut().unwrap().insert(watch);

        assert_eq!(
            state
                .load_watchlist(WatchlistSource::Inline, IoFormat::Lop)
                .unwrap(),
            0
        );
        assert_eq!(state.watchlist().unwrap().members(), 1);
        assert_watchlist_clause_shape(&state);
    }

    #[test]
    fn proof_state_load_watchlist_file_parses_marks_and_requires_eof() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let path = temp_path("file");
        let _ = std::fs::remove_file(&path);
        std::fs::write(&path, b"p(a).\nq(a) <- r(a).\n").unwrap();

        assert_eq!(
            state
                .load_watchlist(WatchlistSource::File(&path), IoFormat::Lop)
                .unwrap(),
            2
        );

        assert_eq!(state.watchlist().unwrap().members(), 2);
        assert_watchlist_clause_shape(&state);
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn proof_state_load_watchlist_disabled_discards_watchlist() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();

        assert_eq!(
            state
                .load_watchlist(WatchlistSource::Disabled, IoFormat::Lop)
                .unwrap(),
            0
        );

        assert!(state.watchlist().is_none());
    }

    #[test]
    fn proof_state_load_watchlist_active_after_disable_reports_error() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();

        state
            .load_watchlist(WatchlistSource::Disabled, IoFormat::Lop)
            .unwrap();
        let error = state
            .load_watchlist(WatchlistSource::Inline, IoFormat::Lop)
            .unwrap_err();

        assert_eq!(error.code(), ErrorCode::OTHER_ERROR);
    }

    #[test]
    fn proof_state_init_watchlist_marks_and_reindexes_watchlist_clauses() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let first = nontrivial_clause(&mut state, "init_watch_first", 90);
        let second = nontrivial_clause(&mut state, "init_watch_second", 91);
        let params = FvIndexParams::new(FvIndexType::AcFold, false, true, 9, 1);

        state.watchlist_mut().unwrap().insert(first);
        state.watchlist_mut().unwrap().insert(second);
        state.init_fvi_anchors(&params).unwrap();
        let mut ocb = test_ocb(&state);

        assert_eq!(state.init_watchlist(&mut ocb).unwrap(), 2);

        let watchlist = state.watchlist().unwrap();
        assert_eq!(watchlist.members(), 2);
        assert_eq!(
            watchlist.iter().map(Clause::ident).collect::<Vec<_>>(),
            vec![90, 91]
        );
        assert!(watchlist.iter().all(|clause| {
            clause.query_prop(CP_IS_ORIENTED)
                && clause.query_prop(CP_IS_S_INDEXED)
                && clause.literals().query_prop_number(EP_IS_MAXIMAL) == 1
        }));
    }

    #[test]
    fn proof_state_init_watchlist_without_watchlist_is_noop() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        state.discard_watchlist();
        let mut ocb = test_ocb(&state);

        assert_eq!(state.init_watchlist(&mut ocb).unwrap(), 0);
        assert!(state.watchlist().is_none());
    }
}
