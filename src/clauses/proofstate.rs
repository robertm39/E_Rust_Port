use crate::basics::defines::DEFAULT_COMCHAR_RAW;
use crate::basics::error::{Diagnostic, ErrorCode};
use crate::basics::simple_stuff::ProblemType;
use crate::clauses::clause::{clause_answer_output_string, Clause};
use crate::clauses::clause_props::{CP_TYPE_WATCH_CLAUSE, CP_WATCH_ONLY};
use crate::clauses::clausesets::ClauseSet;
use crate::clauses::fcvindexing::{
    fvi_param_init_anchors, fvi_param_init_specs, FvIndexInitTargetSets, FvIndexParams,
};
use crate::clauses::freqvectors::FvCollect;
use crate::inout::scanner::{IoFormat, Scanner, TokenType};
use crate::orderings::ocb::OrderControlBlock;
use crate::terms::functypes::FunCode;
use crate::terms::signature::{FunctionProperties, Signature};
use crate::terms::termbanks::TermBank;
use crate::terms::termvars::VarBank;
use crate::terms::typebanks::TypeBank;
use std::{collections::BTreeMap, fmt, path::Path};

pub const WATCHLIST_INLINE_STRING: &str = "Use inline watchlist type";
pub const WATCHLIST_INLINE_QSTRING: &str = "'Use inline watchlist type'";

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

#[derive(Debug)]
pub struct ProofStateProcessedSets<'a> {
    pub pos_rules: &'a mut ClauseSet,
    pub pos_eqns: &'a mut ClauseSet,
    pub neg_units: &'a mut ClauseSet,
    pub non_units: &'a mut ClauseSet,
}

#[derive(Debug)]
pub struct ProofStateGenerationContext<'a> {
    pub processed_pos_rules: &'a ClauseSet,
    pub processed_pos_eqns: &'a ClauseSet,
    pub processed_neg_units: &'a ClauseSet,
    pub processed_non_units: &'a ClauseSet,
    pub tmp_store: &'a mut ClauseSet,
}

#[derive(Clone, Debug)]
pub struct ProofState {
    terms: TermBank,
    fresh_vars: VarBank,
    original_symbols: usize,
    axioms: ClauseSet,
    ax_archive: ClauseSet,
    processed_pos_rules: ClauseSet,
    processed_pos_eqns: ClauseSet,
    processed_neg_units: ClauseSet,
    processed_non_units: ClauseSet,
    unprocessed: ClauseSet,
    tmp_store: ClauseSet,
    eval_store: ClauseSet,
    archive: ClauseSet,
    watchlist: Option<ClauseSet>,
    watchlist_activation: WatchlistActivation,
    definition_store: ClauseSet,
    definition_assocs: BTreeMap<i64, FunCode>,
    fvi_initialized: bool,
    fvi_cspec: Option<FvCollect>,
    def_store_cspec: Option<FvCollect>,
    state_is_complete: bool,
    has_interpreted_symbols: bool,
    statistics: ProofStateStatistics,
    answer_outputs: Vec<String>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum WatchlistActivation {
    #[default]
    Inactive,
    Active,
}

impl ProofState {
    /// Allocates the currently ported proof-state owner fields.
    ///
    /// Formula sets, global indices, demodulator trees, the temporary term bank,
    /// and SAT integration are added by later slices. The clause-set, FV-index,
    /// distinct-symbol, and statistic initialization mirrors C `ProofStateAlloc`.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if internal signature or term-bank setup fails.
    pub fn new(free_symbol_props: FunctionProperties) -> Result<Self, Diagnostic> {
        let mut signature = Signature::new(TypeBank::new());
        signature.insert_internal_codes()?;
        signature.remove_distinct_props(free_symbol_props);
        let terms = TermBank::new(signature)?;
        let fresh_vars = VarBank::new(terms.signature().type_bank());
        terms.vars().pair_shadow(&fresh_vars);

        Ok(Self {
            terms,
            fresh_vars,
            original_symbols: 0,
            axioms: ClauseSet::new(),
            ax_archive: ClauseSet::new(),
            processed_pos_rules: ClauseSet::new(),
            processed_pos_eqns: ClauseSet::new(),
            processed_neg_units: ClauseSet::new(),
            processed_non_units: ClauseSet::new(),
            unprocessed: ClauseSet::new(),
            tmp_store: ClauseSet::new(),
            eval_store: ClauseSet::new(),
            archive: ClauseSet::new(),
            watchlist: Some(ClauseSet::new()),
            watchlist_activation: WatchlistActivation::Inactive,
            definition_store: ClauseSet::new(),
            definition_assocs: BTreeMap::new(),
            fvi_initialized: false,
            fvi_cspec: None,
            def_store_cspec: None,
            state_is_complete: true,
            has_interpreted_symbols: false,
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
    pub const fn fresh_vars(&self) -> &VarBank {
        &self.fresh_vars
    }

    pub fn terms_and_axioms_mut(&mut self) -> (&mut TermBank, &mut ClauseSet) {
        let Self { terms, axioms, .. } = self;
        (terms, axioms)
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
            ..
        } = self;
        (
            terms,
            ProofStateGenerationContext {
                processed_pos_rules,
                processed_pos_eqns,
                processed_neg_units,
                processed_non_units,
                tmp_store,
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
    pub const fn ax_archive(&self) -> &ClauseSet {
        &self.ax_archive
    }

    pub fn ax_archive_mut(&mut self) -> &mut ClauseSet {
        &mut self.ax_archive
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
    pub const fn definition_assocs(&self) -> &BTreeMap<i64, FunCode> {
        &self.definition_assocs
    }

    pub fn definition_assocs_mut(&mut self) -> &mut BTreeMap<i64, FunCode> {
        &mut self.definition_assocs
    }

    pub fn terms_and_definition_store_mut(
        &mut self,
    ) -> (&mut TermBank, &mut ClauseSet, &mut BTreeMap<i64, FunCode>) {
        let Self {
            terms,
            definition_store,
            definition_assocs,
            ..
        } = self;
        (terms, definition_store, definition_assocs)
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
    pub const fn statistics(&self) -> &ProofStateStatistics {
        &self.statistics
    }

    pub fn statistics_mut(&mut self) -> &mut ProofStateStatistics {
        &mut self.statistics
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

    /// Counts clause axioms currently represented in Rust.
    ///
    /// C `ProofStateAxNo` also includes formula axioms; this returns the clause
    /// side until `FormulaSet` is part of the proof-state owner.
    #[must_use]
    pub fn axiom_count(&self) -> i64 {
        self.axioms.members()
    }

    #[must_use]
    pub fn is_untyped(&self) -> bool {
        self.processed_pos_rules.is_untyped()
            && self.processed_pos_eqns.is_untyped()
            && self.processed_neg_units.is_untyped()
            && self.processed_non_units.is_untyped()
            && self.unprocessed.is_untyped()
    }

    /// Clears the clause sets covered by C `ProofStateResetClauseSets`.
    ///
    /// The C helper does not clear `definition_store`, despite its comment
    /// saying all clause and formula sets are emptied. Rust preserves that until
    /// definition-store reset semantics are audited with callers.
    pub fn reset_clause_sets(&mut self) {
        self.axioms.clear();
        self.ax_archive.clear();
        self.processed_pos_rules.clear();
        self.processed_pos_eqns.clear();
        self.processed_neg_units.clear();
        self.processed_non_units.clear();
        self.unprocessed.clear();
        self.tmp_store.clear();
        self.eval_store.clear();
        self.archive.clear();
        if let Some(watchlist) = self.watchlist.as_mut() {
            watchlist.clear();
        }
    }

    /// Marks proof-state clause terms and sweeps unreachable term-bank entries.
    ///
    /// C `TBGCCollect(state->terms)` marks registered clause/formula sets
    /// through the term bank's GC admin. The current Rust proof state owns the
    /// clause sets directly, so this marks every currently represented
    /// proof-state clause owner before sweeping. Formula-set participation is
    /// added when formula owners are ported.
    pub fn collect_term_garbage(&mut self) -> i64 {
        let Self {
            terms,
            axioms,
            ax_archive,
            processed_pos_rules,
            processed_pos_eqns,
            processed_neg_units,
            processed_non_units,
            unprocessed,
            tmp_store,
            eval_store,
            archive,
            watchlist,
            definition_store,
            ..
        } = self;

        for set in [
            axioms,
            ax_archive,
            processed_pos_rules,
            processed_pos_eqns,
            processed_neg_units,
            processed_non_units,
            unprocessed,
            tmp_store,
            eval_store,
            archive,
            definition_store,
        ] {
            for clause in set.iter() {
                clause.gc_mark_terms(terms);
            }
        }
        if let Some(watchlist) = watchlist.as_ref() {
            for clause in watchlist.iter() {
                clause.gc_mark_terms(terms);
            }
        }

        terms.gc_sweep()
    }

    /// Loads or disables the proof-state watchlist like C
    /// `ProofStateLoadWatchlist`.
    ///
    /// File sources are parsed into the existing watchlist set and then require
    /// end-of-file. Inline sources skip parsing but still activate the current
    /// watchlist. Disabled sources drop the optional watchlist.
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
            self.watchlist = None;
            self.watchlist_activation = WatchlistActivation::Inactive;
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
    /// Formula-archive ownership and term-bank detail mode are not represented
    /// here yet, so archived formulas remain zero and optional term-detail
    /// lines stay with the later global proof-output integration.
    ///
    /// # Errors
    ///
    /// Returns any formatting error from `output`.
    pub fn write_statistics(
        &self,
        output: &mut impl fmt::Write,
        record_gc_selection: bool,
    ) -> fmt::Result {
        self.write_processed_statistics(output)?;
        self.write_generation_statistics(output)?;
        self.write_satcheck_statistics(output)?;
        self.write_clause_set_statistics(output, record_gc_selection)
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
        writeln!(
            output,
            "{DEFAULT_COMCHAR_RAW} Generated clauses                    : {}",
            statistics
                .generated_count
                .wrapping_sub(statistics.backward_rewritten_count)
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
            "{DEFAULT_COMCHAR_RAW} ...of those cached                   : {}",
            statistics.rw_count
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
            "{DEFAULT_COMCHAR_RAW} Current number of archived formulas  : 0"
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

    #[must_use]
    pub fn statistics_string(&self, record_gc_selection: bool) -> String {
        let mut output = String::new();
        let _ = self.write_statistics(&mut output, record_gc_selection);
        output
    }

    /// Initializes the preloaded watchlist clauses like the local clause-set
    /// portion of C `ProofStateInitWatchlist`.
    ///
    /// This orients and marks maximal terms, drains the watchlist through a
    /// temporary set, and reinserts it through the owned FV index when one is
    /// installed. The C helper also inserts the result into `state->wlindices`;
    /// that global-index side effect remains pending until global indices are
    /// represented in Rust.
    pub fn init_watchlist(&mut self, ocb: &mut OrderControlBlock) -> i64 {
        let Self {
            terms, watchlist, ..
        } = self;
        let Some(watchlist) = watchlist.as_mut() else {
            return 0;
        };

        watchlist.mark_maximal_terms(ocb, terms);
        let mut temp = ClauseSet::new();
        while let Some(clause) = watchlist.extract_first() {
            temp.insert(clause);
        }

        let inserted = watchlist.indexed_insert_clause_set_owned(&mut temp, terms);
        debug_assert!(temp.is_empty());
        inserted
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
    use super::{proof_state_alloc, ProofState, ProofStateStatistics, WatchlistSource};
    use crate::basics::error::ErrorCode;
    use crate::basics::partial_orderings::HoOrderKind;
    use crate::clauses::clause::{clause_print_lop_format_string, Clause};
    use crate::clauses::clause_props::{
        CP_IS_ORIENTED, CP_IS_S_INDEXED, CP_TYPE_WATCH_CLAUSE, CP_WATCH_ONLY,
    };
    use crate::clauses::eqn::Eqn;
    use crate::clauses::eqn_props::EP_IS_MAXIMAL;
    use crate::clauses::eqnlist::EqnList;
    use crate::clauses::fcvindexing::FvIndexParams;
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

    fn nontrivial_clause(state: &mut ProofState, stem: &str, ident: i64) -> Clause {
        let bank = state.terms_mut();
        let left = typed_const(bank, &format!("{stem}_left"));
        let right_const = typed_const(bank, &format!("{stem}_right"));
        let right = typed_unary(bank, &format!("{stem}_f"), &right_const);
        clause_from(vec![literal(bank, &left, &right, true)], ident)
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
        assert_eq!(state.ax_archive().members(), 0);
        assert_eq!(state.processed_pos_rules().members(), 0);
        assert_eq!(state.processed_pos_eqns().members(), 0);
        assert_eq!(state.processed_neg_units().members(), 0);
        assert_eq!(state.processed_non_units().members(), 0);
        assert_eq!(state.unprocessed().members(), 0);
        assert_eq!(state.tmp_store().members(), 0);
        assert_eq!(state.eval_store().members(), 0);
        assert_eq!(state.archive().members(), 0);
        assert_eq!(state.definition_store().members(), 0);
        assert!(state.state_is_complete());
        assert!(!state.has_interpreted_symbols());
        assert!(!state.fvi_initialized());
        assert!(state.fvi_cspec().is_none());
        assert!(state.def_store_cspec().is_none());
        assert_eq!(state.statistics(), &ProofStateStatistics::default());
        assert!(state.terms().signature().distinct_code() > 0);
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

        state.processed_pos_rules_mut().insert(rule);
        state.processed_pos_eqns_mut().insert(equation);
        state.processed_neg_units_mut().insert(negative);
        state.processed_non_units_mut().insert(non_unit);
        state.unprocessed_mut().insert(unprocessed);
        state.axioms_mut().insert(axiom);

        assert_eq!(state.processed_cardinality(), 4);
        assert_eq!(state.unprocessed_cardinality(), 1);
        assert_eq!(state.cardinality(), 5);
        assert_eq!(state.axiom_count(), 1);
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
        let params = FvIndexParams::new(FvIndexType::AcFold, false, true, 9, 1);

        state.axioms_mut().insert(axiom);
        state.processed_non_units_mut().insert(processed);
        state.unprocessed_mut().insert(unprocessed);
        state.watchlist_mut().unwrap().insert(watch);
        state.definition_store_mut().insert(def);
        state.init_fvi_anchors(&params).unwrap();

        state.reset_clause_sets();

        assert!(state.fvi_initialized());
        assert_eq!(state.axioms().members(), 0);
        assert_eq!(state.processed_non_units().members(), 0);
        assert_eq!(state.unprocessed().members(), 0);
        assert_eq!(state.watchlist().unwrap().members(), 0);
        assert_eq!(state.definition_store().members(), 1);
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

        assert_eq!(state.init_watchlist(&mut ocb), 2);

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

        assert_eq!(state.init_watchlist(&mut ocb), 0);
        assert!(state.watchlist().is_none());
    }
}
