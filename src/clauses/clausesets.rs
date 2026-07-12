use crate::basics::defines::DEFAULT_COMCHAR_RAW;
use crate::basics::error::{Diagnostic, ErrorCode};
use crate::basics::pdarrays::PDIntArray;
use crate::basics::pstacks::PStack;
use crate::basics::simple_stuff::ProblemType;
use crate::basics::sysdate::SysDate;
use crate::clauses::clause::{
    clause_parse_with_options, clause_print_format_string_with_options,
    clause_print_lop_format_string, clause_print_lop_format_string_with_options,
    clause_print_tptp_format_string_with_options, clause_starts_maybe,
    clause_write_tstp_with_type_suffixes, Clause, ClauseParseOptions,
};
use crate::clauses::clause_props::{
    FormulaProperties, CP_DELETE_CLAUSE, CP_IS_D_INDEXED, CP_IS_SOS, CP_IS_S_INDEXED,
    CP_TYPE_CONJECTURE,
};
use crate::clauses::clausepos::ClausePos;
use crate::clauses::derivation::ClauseDerivationRef;
use crate::clauses::eqn::EqnPrintOptions;
use crate::clauses::eqn_props::EqnSide;
use crate::clauses::fcvindexing::{fv_index_pack_clause, fv_index_storage, FvIndexAnchor};
use crate::clauses::freqvectors::{
    fv_size, perm_vector_compute_internal, var_freq_vector_compute, FreqVector, FvCollect,
    FvIndexType, PermVector,
};
use crate::clauses::neweval::{EvalCell, EvalObjectHandle};
use crate::clauses::pdtrees::{PdTree, PdtIndexedOccurrence, PdtSearchState, PdtTraversalOrder};
use crate::clauses::tautologies::clause_is_tautology;
use crate::inout::scanner::{IoFormat, Scanner};
use crate::orderings::ocb::OrderControlBlock;
use crate::terms::functypes::FunCode;
use crate::terms::signature::Signature;
use crate::terms::termbanks::TermBank;
use crate::terms::termfunc::term_compute_order;
use crate::terms::termtypes::{Term, TermProperties};
use std::cell::Cell;
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::io::{self, Write};

const CLAUSECELL_MEM: i64 = 68;
const PTREE_CELL_MEM: i64 = 16;
const CLAUSECELL_DYN_MEM: i64 = CLAUSECELL_MEM + 3 * PTREE_CELL_MEM;
const EQN_CELL_MEM: i64 = 24;

#[derive(Clone, Copy, Debug)]
struct EvalIndexEntry {
    object: EvalObjectHandle,
    priority: i64,
    eval_count: i64,
    heuristic: f32,
}

impl EvalIndexEntry {
    fn from_eval(object: EvalObjectHandle, evaluations: &EvalCell, pos: usize) -> Self {
        let eval = evaluations.eval(pos);
        Self {
            object,
            priority: eval.priority(),
            eval_count: evaluations.eval_count(),
            heuristic: eval.heuristic(),
        }
    }
}

impl PartialEq for EvalIndexEntry {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for EvalIndexEntry {}

impl PartialOrd for EvalIndexEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for EvalIndexEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        let eval_order = match self.priority.cmp(&other.priority) {
            Ordering::Equal => match self.eval_count.cmp(&other.eval_count) {
                Ordering::Equal => Ordering::Equal,
                ordering => {
                    let heuristic_order = cmp_f32_c(self.heuristic, other.heuristic);
                    if heuristic_order == Ordering::Equal {
                        ordering
                    } else {
                        heuristic_order
                    }
                }
            },
            ordering => return ordering,
        };

        eval_order.then_with(|| self.object.cmp(&other.object))
    }
}

type ClauseSlot = usize;
const SPARSE_STORE_COMPACT_MIN_HOLES: usize = 64;

#[derive(Clone, Debug, Default)]
struct SparseClauseStore {
    slots: Vec<Option<Clause>>,
    len: usize,
    first_occupied: usize,
}

impl SparseClauseStore {
    fn len(&self) -> usize {
        self.len
    }

    fn is_empty(&self) -> bool {
        self.len == 0
    }

    fn iter(&self) -> impl Iterator<Item = &Clause> {
        self.slots.iter().flatten()
    }

    fn iter_mut(&mut self) -> impl Iterator<Item = &mut Clause> {
        self.slots.iter_mut().flatten()
    }

    fn occupied_slots(&self) -> impl Iterator<Item = ClauseSlot> + '_ {
        self.slots
            .iter()
            .enumerate()
            .skip(self.first_occupied)
            .filter_map(|(slot, clause)| clause.as_ref().map(|_| slot))
    }

    fn push_back(&mut self, clause: Clause) -> ClauseSlot {
        let slot = self.slots.len();
        if self.len == 0 {
            self.first_occupied = slot;
        }
        self.slots.push(Some(clause));
        self.len += 1;
        slot
    }

    fn get_slot(&self, slot: ClauseSlot) -> Option<&Clause> {
        self.slots.get(slot)?.as_ref()
    }

    fn get_slot_mut(&mut self, slot: ClauseSlot) -> Option<&mut Clause> {
        self.slots.get_mut(slot)?.as_mut()
    }

    fn position_of_slot(&self, slot: ClauseSlot) -> Option<usize> {
        self.get_slot(slot)?;
        Some(
            self.slots[self.first_occupied..slot]
                .iter()
                .filter(|clause| clause.is_some())
                .count(),
        )
    }

    fn first_slot(&self) -> Option<ClauseSlot> {
        self.get_slot(self.first_occupied)
            .map(|_| self.first_occupied)
    }

    fn remove_slot(&mut self, slot: ClauseSlot) -> Option<Clause> {
        let clause = self.slots.get_mut(slot)?.take()?;
        self.len -= 1;
        if self.len == 0 {
            self.slots.clear();
            self.first_occupied = 0;
        } else if slot == self.first_occupied {
            self.first_occupied = self.slots[slot + 1..]
                .iter()
                .position(Option::is_some)
                .map_or(self.slots.len(), |offset| slot + 1 + offset);
        }
        Some(clause)
    }

    fn compact_if_sparse(&mut self) -> bool {
        let holes = self.slots.len().saturating_sub(self.len);
        if holes < SPARSE_STORE_COMPACT_MIN_HOLES || holes <= self.len {
            return false;
        }

        self.slots = std::mem::take(&mut self.slots)
            .into_iter()
            .flatten()
            .map(Some)
            .collect();
        self.first_occupied = 0;
        true
    }

    fn sort_unstable_by(&mut self, mut compare: impl FnMut(&Clause, &Clause) -> Ordering) {
        let mut clauses = std::mem::take(&mut self.slots)
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();
        clauses.sort_unstable_by(|left, right| compare(left, right));
        self.len = clauses.len();
        self.slots = clauses.into_iter().map(Some).collect();
        self.first_occupied = 0;
    }
}

impl PartialEq for SparseClauseStore {
    fn eq(&self, other: &Self) -> bool {
        self.len == other.len && self.iter().eq(other.iter())
    }
}

impl<'a> IntoIterator for &'a SparseClauseStore {
    type Item = &'a Clause;
    type IntoIter = std::iter::Flatten<std::slice::Iter<'a, Option<Clause>>>;

    fn into_iter(self) -> Self::IntoIter {
        self.slots.iter().flatten()
    }
}

impl<'a> IntoIterator for &'a mut SparseClauseStore {
    type Item = &'a mut Clause;
    type IntoIter = std::iter::Flatten<std::slice::IterMut<'a, Option<Clause>>>;

    fn into_iter(self) -> Self::IntoIter {
        self.slots.iter_mut().flatten()
    }
}

impl IntoIterator for SparseClauseStore {
    type Item = Clause;
    type IntoIter = std::iter::Flatten<std::vec::IntoIter<Option<Clause>>>;

    fn into_iter(self) -> Self::IntoIter {
        self.slots.into_iter().flatten()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ClauseSet {
    clauses: SparseClauseStore,
    literals: i64,
    date: SysDate,
    identifier: String,
    demod_index: Option<PdTree>,
    demod_index_coverage: Cell<Option<bool>>,
    indexed_clause_positions: BTreeMap<i64, usize>,
    fv_anchor: Option<FvIndexAnchor>,
    eval_indices: Vec<BTreeSet<EvalIndexEntry>>,
    eval_object_slots: Vec<Option<ClauseSlot>>,
    eval_no: usize,
    next_eval_object: EvalObjectHandle,
}

impl Default for ClauseSet {
    fn default() -> Self {
        Self::new()
    }
}

impl ClauseSet {
    #[must_use]
    pub fn new() -> Self {
        let mut date = SysDate::creation_time();
        let _ = date.increment();
        Self {
            clauses: SparseClauseStore::default(),
            literals: 0,
            date,
            identifier: String::new(),
            demod_index: None,
            demod_index_coverage: Cell::new(None),
            indexed_clause_positions: BTreeMap::new(),
            fv_anchor: None,
            eval_indices: Vec::new(),
            eval_object_slots: Vec::new(),
            eval_no: 0,
            next_eval_object: 0,
        }
    }

    #[must_use]
    pub fn new_demod_indexed() -> Self {
        let mut set = Self::new();
        set.init_demod_index();
        set
    }

    #[must_use]
    pub fn from_clauses(clauses: impl IntoIterator<Item = Clause>) -> Self {
        let mut set = Self::new();
        for clause in clauses {
            set.insert(clause);
        }
        set
    }

    #[must_use]
    pub fn into_clauses(self) -> Vec<Clause> {
        self.clauses.into_iter().collect()
    }

    #[must_use]
    pub const fn date(&self) -> SysDate {
        self.date
    }

    pub const fn set_date(&mut self, date: SysDate) {
        self.date = date;
    }

    #[must_use]
    pub fn identifier(&self) -> &str {
        &self.identifier
    }

    pub fn set_identifier(&mut self, identifier: impl Into<String>) {
        self.identifier = identifier.into();
    }

    pub fn init_demod_index(&mut self) {
        if self.demod_index.is_none() {
            self.demod_index = Some(PdTree::new());
            self.demod_index_coverage.set(None);
            self.rebuild_indexed_clause_positions();
        }
    }

    #[must_use]
    pub const fn demod_index(&self) -> Option<&PdTree> {
        self.demod_index.as_ref()
    }

    #[must_use]
    pub fn demod_index_match_count(&self) -> u64 {
        self.demod_index.as_ref().map_or(0, PdTree::match_count)
    }

    #[must_use]
    pub fn demod_index_visited_count(&self) -> u64 {
        self.demod_index.as_ref().map_or(0, PdTree::visited_count)
    }

    #[must_use]
    pub fn demod_index_traversal_order(&self) -> Option<PdtTraversalOrder> {
        self.demod_index
            .as_ref()
            .map(PdTree::search_traversal_order)
    }

    #[must_use]
    pub fn demod_index_search_state(&self) -> Option<PdtSearchState> {
        self.demod_index.as_ref().and_then(PdTree::search_state)
    }

    #[must_use]
    pub fn demod_index_search_active(&self) -> bool {
        self.demod_index
            .as_ref()
            .is_some_and(PdTree::search_is_active)
    }

    #[must_use]
    pub fn demod_index_search_may_have_match(&self) -> bool {
        let Some(index) = &self.demod_index else {
            return true;
        };
        if !self.demod_index_covers_units() {
            return true;
        }
        index.search_root_satisfies_constraints() && index.search_root_may_have_matchable_path()
    }

    #[must_use]
    pub fn demod_index_search_uses_compact_candidates(&self) -> bool {
        self.demod_index.is_some() && self.demod_index_covers_units()
    }

    #[must_use]
    pub fn demod_index_search_candidate_sides(&self) -> Option<Vec<PdtIndexedOccurrence>> {
        let index = self.demod_index.as_ref()?;
        if !self.demod_index_covers_units() {
            return None;
        }
        index.search_matching_occurrences()
    }

    pub fn demod_index_search_next_candidate_side(&self) -> Option<PdtIndexedOccurrence> {
        if !self.demod_index_covers_units() {
            return None;
        }
        self.demod_index
            .as_ref()
            .and_then(PdTree::search_next_matching_occurrence)
    }

    pub fn record_demod_index_search_attempt(&self) {
        if let Some(index) = &self.demod_index {
            index.record_search_attempt();
        }
    }

    pub fn record_demod_index_search_init(
        &self,
        term: &Term,
        age_constraint: SysDate,
        prefer_general: bool,
    ) {
        if let Some(index) = &self.demod_index {
            index.record_search_init(term, age_constraint, prefer_general);
        }
    }

    pub fn record_demod_index_search_exit(&self) {
        if let Some(index) = &self.demod_index {
            index.record_search_exit();
        }
    }

    pub fn record_demod_index_nodes_visited(&self, count: u64) {
        if let Some(index) = &self.demod_index {
            index.record_nodes_visited(count);
        }
    }

    #[must_use]
    pub fn demod_index_storage_estimate(&self) -> usize {
        self.demod_index
            .as_ref()
            .map_or(0, PdTree::storage_estimate)
    }

    #[must_use]
    pub const fn fv_anchor(&self) -> Option<&FvIndexAnchor> {
        self.fv_anchor.as_ref()
    }

    #[must_use]
    pub const fn fv_anchor_mut(&mut self) -> Option<&mut FvIndexAnchor> {
        self.fv_anchor.as_mut()
    }

    pub fn set_fv_anchor(&mut self, fv_anchor: Option<FvIndexAnchor>) -> Option<FvIndexAnchor> {
        std::mem::replace(&mut self.fv_anchor, fv_anchor)
    }

    pub fn take_fv_anchor(&mut self) -> Option<FvIndexAnchor> {
        self.fv_anchor.take()
    }

    #[must_use]
    pub fn members(&self) -> i64 {
        usize_to_i64(self.clauses.len())
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.clauses.len()
    }

    #[must_use]
    pub const fn literals(&self) -> i64 {
        self.literals
    }

    #[must_use]
    pub const fn eval_no(&self) -> usize {
        self.eval_no
    }

    /// Returns the C `ClauseSetStorage` constant-memory estimate.
    ///
    /// Includes the demodulator `PDTreeStorage` component when this set owns a
    /// demodulator index.
    #[must_use]
    pub fn storage_estimate(&self) -> i64 {
        let clause_cell_mem = CLAUSECELL_DYN_MEM.saturating_add(eval_mem(self.eval_no));
        clause_cell_mem
            .saturating_mul(self.members())
            .saturating_add(EQN_CELL_MEM.saturating_mul(self.literals()))
            .saturating_add(usize_to_i64(self.demod_index_storage_estimate()))
            .saturating_add(usize_to_i64(fv_index_storage(self.fv_anchor())))
    }

    /// Writes C `ClauseSetDerivationStackStatistics` histogram output.
    ///
    /// C counts missing derivation stacks in bucket zero, uses an integer
    /// `PDArray` with initial/grow size eight, and prints every allocated
    /// bucket including zero-count buckets.
    ///
    /// # Errors
    ///
    /// Returns any write error reported by `output`.
    #[allow(clippy::cast_precision_loss)]
    pub fn write_derivation_stack_statistics(&self, output: &mut impl Write) -> io::Result<()> {
        let mut distribution = PDIntArray::with_default(8, 8, 0);
        for clause in &self.clauses {
            distribution.inc_int(clause.derivation_stack_pointer(), 1);
        }

        let mut sum = 0.0;
        for (index, &count) in distribution.as_slice().iter().enumerate() {
            writeln!(output, "{DEFAULT_COMCHAR_RAW} {index:5}: {count:6}")?;
            sum += (count as f64) * (index as f64);
        }

        let average = sum / (self.members() as f64);
        writeln!(
            output,
            "{DEFAULT_COMCHAR_RAW} Average over {} clauses: {average:.6}",
            self.members()
        )
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.clauses.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Clause> {
        self.clauses.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Clause> {
        self.demod_index_coverage.set(None);
        self.clauses.iter_mut()
    }

    #[must_use]
    pub fn print_lop_string(&self, bank: &TermBank, full_terms: bool) -> String {
        self.print_lop_string_with_options(bank, full_terms, EqnPrintOptions::default())
    }

    #[must_use]
    pub fn print_lop_string_with_options(
        &self,
        bank: &TermBank,
        full_terms: bool,
        options: EqnPrintOptions,
    ) -> String {
        let mut output = String::new();
        for clause in &self.clauses {
            output.push_str(&clause_print_lop_format_string_with_options(
                bank, clause, full_terms, options,
            ));
            output.push('\n');
        }
        output
    }

    #[must_use]
    pub fn print_lop_prefix_string(&self, bank: &TermBank, prefix: &str) -> String {
        let mut output = String::new();
        for clause in &self.clauses {
            output.push_str(prefix);
            output.push_str(&clause_print_lop_format_string(bank, clause, true));
            output.push('\n');
        }
        output
    }

    #[must_use]
    pub fn print_tptp_format_string(&self, bank: &TermBank) -> String {
        self.print_tptp_format_string_with_options(bank, EqnPrintOptions::tptp())
    }

    #[must_use]
    pub fn print_tptp_format_string_with_options(
        &self,
        bank: &TermBank,
        options: EqnPrintOptions,
    ) -> String {
        let mut output = String::new();
        for clause in &self.clauses {
            output.push_str(&clause_print_tptp_format_string_with_options(
                bank, clause, options,
            ));
            output.push('\n');
        }
        output
    }

    /// Returns the C `ClauseSetPrint` shape with explicit `ClausePrint` dispatch.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if TSTP rendering rejects a stored clause.
    pub fn print_format_string(
        &self,
        bank: &TermBank,
        full_terms: bool,
        output_format: IoFormat,
        problem_type: ProblemType,
    ) -> Result<String, Diagnostic> {
        let options = match output_format {
            IoFormat::Tptp => EqnPrintOptions::tptp(),
            IoFormat::Lop | IoFormat::Tstp | IoFormat::Auto => EqnPrintOptions::lop(),
        };
        self.print_format_string_with_options(
            bank,
            full_terms,
            output_format,
            problem_type,
            options,
        )
    }

    /// Returns the C `ClauseSetPrint` shape with caller-provided equation options.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if TSTP rendering rejects a stored clause.
    pub fn print_format_string_with_options(
        &self,
        bank: &TermBank,
        full_terms: bool,
        output_format: IoFormat,
        problem_type: ProblemType,
        options: EqnPrintOptions,
    ) -> Result<String, Diagnostic> {
        let mut output = String::new();
        for clause in &self.clauses {
            output.push_str(&clause_set_render_clause_string(
                bank,
                clause,
                full_terms,
                output_format,
                problem_type,
                options,
            )?);
            output.push('\n');
        }
        Ok(output)
    }

    /// Returns the C `ClauseSetPrintPrefix` shape with explicit `ClausePrint` dispatch.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if TSTP rendering rejects a stored clause.
    pub fn print_prefix_format_string(
        &self,
        bank: &TermBank,
        prefix: &str,
        output_format: IoFormat,
        problem_type: ProblemType,
        options: EqnPrintOptions,
    ) -> Result<String, Diagnostic> {
        let mut output = String::new();
        for clause in &self.clauses {
            output.push_str(prefix);
            output.push_str(&clause_set_render_clause_string(
                bank,
                clause,
                true,
                output_format,
                problem_type,
                options,
            )?);
            output.push('\n');
        }
        Ok(output)
    }

    /// Writes the C `ClauseSetTSTPPrint` shape through the shared clause TSTP
    /// renderer.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if rendering or writing any clause fails.
    pub fn write_tstp(
        &self,
        output: &mut impl fmt::Write,
        bank: &TermBank,
        full_terms: bool,
        problem_type: ProblemType,
    ) -> Result<(), Diagnostic> {
        self.write_tstp_with_type_suffixes(output, bank, full_terms, problem_type, false)
    }

    /// Writes the C `ClauseSetTSTPPrint` shape with optional term type suffixes.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if rendering or writing any clause fails.
    pub fn write_tstp_with_type_suffixes(
        &self,
        output: &mut impl fmt::Write,
        bank: &TermBank,
        full_terms: bool,
        problem_type: ProblemType,
        print_types: bool,
    ) -> Result<(), Diagnostic> {
        for clause in &self.clauses {
            clause_write_tstp_with_type_suffixes(
                output,
                bank,
                clause,
                full_terms,
                true,
                problem_type,
                print_types,
            )?;
            output
                .write_str("\n")
                .map_err(clause_set_tstp_write_error)?;
        }
        Ok(())
    }

    /// Returns the C `ClauseSetTSTPPrint` shape through the shared clause TSTP
    /// renderer.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic under the same conditions as [`Self::write_tstp`].
    pub fn tstp_print_string(
        &self,
        bank: &TermBank,
        full_terms: bool,
        problem_type: ProblemType,
    ) -> Result<String, Diagnostic> {
        self.tstp_print_string_with_type_suffixes(bank, full_terms, problem_type, false)
    }

    /// Returns the C `ClauseSetTSTPPrint` shape with optional term type suffixes.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic under the same conditions as [`Self::write_tstp`].
    pub fn tstp_print_string_with_type_suffixes(
        &self,
        bank: &TermBank,
        full_terms: bool,
        problem_type: ProblemType,
        print_types: bool,
    ) -> Result<String, Diagnostic> {
        let mut output = String::new();
        self.write_tstp_with_type_suffixes(
            &mut output,
            bank,
            full_terms,
            problem_type,
            print_types,
        )?;
        Ok(output)
    }

    /// Parses the C `ClauseSetParseList` loop over the currently ported simple
    /// clause parser.
    ///
    /// # Panics
    ///
    /// Panics if `scanner` is in `IoFormat::Auto`, matching the clause parser's
    /// concrete-format precondition.
    pub fn parse_list(
        &mut self,
        scanner: &mut Scanner,
        bank: &mut TermBank,
        problem_type: ProblemType,
    ) -> Result<i64, Diagnostic> {
        self.parse_list_with_options(scanner, bank, problem_type, ClauseParseOptions::default())
    }

    /// Parses clauses until the current token no longer starts a clause,
    /// inserting each parsed clause in set order.
    ///
    /// # Panics
    ///
    /// Panics if `scanner` is in `IoFormat::Auto`, matching the clause parser's
    /// concrete-format precondition.
    pub fn parse_list_with_options(
        &mut self,
        scanner: &mut Scanner,
        bank: &mut TermBank,
        problem_type: ProblemType,
        options: ClauseParseOptions,
    ) -> Result<i64, Diagnostic> {
        let mut count = 0;
        while clause_starts_maybe(scanner) {
            let clause = clause_parse_with_options(scanner, bank, problem_type, options)?;
            self.insert(clause);
            count += 1;
        }
        Ok(count)
    }

    pub fn insert(&mut self, mut clause: Clause) {
        self.demod_index_coverage.set(None);
        self.compact_clause_store_if_sparse();
        self.literals += usize_to_i64(clause.literal_number());
        index_clause_evaluations(
            &mut self.eval_indices,
            &mut self.eval_no,
            &mut self.next_eval_object,
            &mut clause,
        );
        let eval_object = clause.evaluations().and_then(EvalCell::object);
        let ident = clause.ident();
        let slot = self.clauses.push_back(clause);
        if let Some(object) = eval_object {
            if self.eval_object_slots.len() <= object {
                self.eval_object_slots.resize(object + 1, None);
            }
            self.eval_object_slots[object] = Some(slot);
        }
        if self.demod_index.is_some() {
            self.indexed_clause_positions.entry(ident).or_insert(slot);
        }
    }

    pub fn insert_set(&mut self, source: &mut Self) -> i64 {
        let mut moved = 0;
        while let Some(clause) = source.extract_first() {
            self.insert(clause);
            moved += 1;
        }
        moved
    }

    pub fn clear(&mut self) {
        while self.extract_first().is_some() {}
    }

    pub fn indexed_insert_clause(
        &mut self,
        clause: Clause,
        fv_anchor: Option<&mut FvIndexAnchor>,
        bank: &TermBank,
    ) {
        let mut clause = indexed_clause_for_anchor(clause, fv_anchor, bank);
        self.index_clause_demodulator(&mut clause);
        self.insert(clause);
    }

    pub fn indexed_insert_clause_owned(&mut self, clause: Clause, bank: &TermBank) {
        let mut clause = indexed_clause_for_anchor(clause, self.fv_anchor.as_mut(), bank);
        self.index_clause_demodulator(&mut clause);
        self.insert(clause);
    }

    pub fn indexed_insert_clause_set(
        &mut self,
        source: &mut Self,
        mut fv_anchor: Option<&mut FvIndexAnchor>,
        bank: &TermBank,
    ) -> i64 {
        let mut moved = 0;
        while let Some(mut clause) = source.extract_first() {
            clause.set_weight(clause.standard_weight());
            self.indexed_insert_clause(clause, fv_anchor.as_deref_mut(), bank);
            moved += 1;
        }
        moved
    }

    pub fn indexed_insert_clause_set_owned(&mut self, source: &mut Self, bank: &TermBank) -> i64 {
        let mut moved = 0;
        while let Some(mut clause) = source.extract_first() {
            clause.set_weight(clause.standard_weight());
            self.indexed_insert_clause_owned(clause, bank);
            moved += 1;
        }
        moved
    }

    pub fn extract_first(&mut self) -> Option<Clause> {
        let slot = self.clauses.first_slot()?;
        self.extract_at_slot(slot)
    }

    pub fn extract_by_id(&mut self, ident: i64) -> Option<Clause> {
        let slot = self.slot_by_id(ident)?;
        self.extract_at_slot(slot)
    }

    pub fn delete_by_id(&mut self, ident: i64) -> bool {
        self.extract_by_id(ident).is_some()
    }

    #[must_use]
    pub fn find_same(&self, clause: &Clause) -> Option<&Clause> {
        self.clauses
            .iter()
            .find(|candidate| std::ptr::eq(*candidate, clause))
    }

    #[must_use]
    pub fn verify_demod_clause_side(&self, clause: &Clause, side: EqnSide) -> bool {
        if self.find_same(clause).is_none() {
            return false;
        }
        if !clause.is_demodulator() {
            return false;
        }
        if side == EqnSide::RightSide
            && clause
                .literals()
                .as_slice()
                .first()
                .is_some_and(crate::clauses::eqn::Eqn::is_oriented)
        {
            return false;
        }
        true
    }

    #[must_use]
    pub fn find_by_id(&self, ident: i64) -> Option<&Clause> {
        self.clauses.iter().find(|clause| clause.ident() == ident)
    }

    #[must_use]
    pub(crate) fn find_indexed_by_id(&self, ident: i64) -> Option<&Clause> {
        let slot = *self.indexed_clause_positions.get(&ident)?;
        self.clauses.get_slot(slot)
    }

    #[must_use]
    pub(crate) fn find_indexed_position_by_id(&self, ident: i64) -> Option<(usize, &Clause)> {
        let slot = *self.indexed_clause_positions.get(&ident)?;
        let position = self.clauses.position_of_slot(slot)?;
        self.clauses.get_slot(slot).map(|clause| (position, clause))
    }

    #[must_use]
    pub fn find_by_derivation_ref(&self, parent: ClauseDerivationRef) -> Option<&Clause> {
        self.clauses
            .iter()
            .find(|clause| ClauseDerivationRef::from(*clause) == parent)
    }

    #[must_use]
    pub fn find_by_derivation_ref_mut(
        &mut self,
        parent: ClauseDerivationRef,
    ) -> Option<&mut Clause> {
        self.demod_index_coverage.set(None);
        self.clauses
            .iter_mut()
            .find(|clause| ClauseDerivationRef::from(&**clause) == parent)
    }

    pub fn find_by_id_mut(&mut self, ident: i64) -> Option<&mut Clause> {
        self.demod_index_coverage.set(None);
        self.clauses
            .iter_mut()
            .find(|clause| clause.ident() == ident)
    }

    #[must_use]
    pub fn find_best(&self, idx: usize) -> Option<&Clause> {
        let object = self.eval_indices.get(idx)?.first()?.object;
        self.find_by_eval_object(object)
    }

    pub fn extract_best(&mut self, idx: usize) -> Option<Clause> {
        let object = self.eval_indices.get(idx)?.first()?.object;
        self.extract_by_eval_object(object)
    }

    pub fn extract_by_eval_object(&mut self, object: EvalObjectHandle) -> Option<Clause> {
        let slot = self.eval_object_slots.get(object).copied().flatten()?;
        self.extract_at_slot(slot)
    }

    #[must_use]
    pub fn eval_order_objects(&self, idx: usize) -> Vec<EvalObjectHandle> {
        self.eval_indices.get(idx).map_or_else(Vec::new, |root| {
            root.iter().map(|entry| entry.object).collect()
        })
    }

    pub fn del_prop_by_eval_object(
        &mut self,
        object: EvalObjectHandle,
        prop: FormulaProperties,
    ) -> bool {
        let Some(clause) = self.find_by_eval_object_mut(object) else {
            return false;
        };
        if clause.query_prop(prop) {
            clause.del_prop(prop);
            true
        } else {
            false
        }
    }

    pub fn remove_evaluations(&mut self) {
        for root in &mut self.eval_indices {
            root.clear();
        }
        self.eval_object_slots.clear();
        self.next_eval_object = 0;
        for clause in &mut self.clauses {
            clause.remove_evaluations();
        }
    }

    pub fn rebuild_eval_indices(&mut self) {
        self.eval_indices.clear();
        self.eval_object_slots.clear();
        self.eval_no = 0;
        self.next_eval_object = 0;
        for clause in &mut self.clauses {
            index_clause_evaluations(
                &mut self.eval_indices,
                &mut self.eval_no,
                &mut self.next_eval_object,
                clause,
            );
        }
        self.rebuild_eval_object_slots();
    }

    #[must_use]
    pub fn eval_order_cloned(&self, idx: usize) -> Vec<Clause> {
        self.eval_indices.get(idx).map_or_else(Vec::new, |root| {
            root.iter()
                .filter_map(|entry| self.find_by_eval_object(entry.object).cloned())
                .collect()
        })
    }

    pub fn sort_by<F>(&mut self, mut compare: F)
    where
        F: FnMut(&Clause, &Clause) -> Ordering,
    {
        for clause in &mut self.clauses {
            clause.set_weight(clause.standard_weight());
        }
        self.clauses
            .sort_unstable_by(|left, right| compare(left, right));
        self.rebuild_indexed_clause_positions();
        self.rebuild_eval_object_slots();
    }

    pub fn sort_literals_by<F>(&mut self, mut compare: F)
    where
        F: FnMut(&crate::clauses::eqn::Eqn, &crate::clauses::eqn::Eqn) -> i64,
    {
        for clause in &mut self.clauses {
            clause.sort_literals_by(&mut compare);
        }
    }

    pub fn set_prop(&mut self, prop: FormulaProperties) {
        self.demod_index_coverage.set(None);
        for clause in &mut self.clauses {
            clause.set_prop(prop);
        }
    }

    pub fn del_prop(&mut self, prop: FormulaProperties) {
        self.demod_index_coverage.set(None);
        for clause in &mut self.clauses {
            clause.del_prop(prop);
        }
    }

    pub fn set_tptp_type(&mut self, type_: FormulaProperties) {
        for clause in &mut self.clauses {
            clause.set_tptp_type(type_);
        }
    }

    pub fn mark_copies(&mut self) -> i64 {
        let occupied_slots = self.clauses.occupied_slots().collect::<Vec<_>>();
        let duplicate_slots = occupied_slots
            .iter()
            .enumerate()
            .filter_map(|(index, &slot)| {
                let clause = self.clauses.get_slot(slot)?;
                occupied_slots[..index]
                    .iter()
                    .any(|&previous| {
                        self.clauses
                            .get_slot(previous)
                            .is_some_and(|candidate| candidate.compare_fun(clause) == 0)
                    })
                    .then_some(slot)
            })
            .collect::<Vec<_>>();
        for slot in &duplicate_slots {
            if let Some(clause) = self.clauses.get_slot_mut(*slot) {
                clause.set_prop(CP_DELETE_CLAUSE);
            }
        }
        usize_to_i64(duplicate_slots.len())
    }

    pub fn delete_marked_entries(&mut self) -> i64 {
        let marked_slots = self
            .clauses
            .occupied_slots()
            .filter(|&slot| {
                self.clauses
                    .get_slot(slot)
                    .is_some_and(|clause| clause.query_prop(CP_DELETE_CLAUSE))
            })
            .collect::<Vec<_>>();
        for slot in &marked_slots {
            let _ = self.extract_at_slot(*slot);
        }
        self.compact_clause_store_if_sparse();
        usize_to_i64(marked_slots.len())
    }

    pub fn delete_copies(&mut self) -> i64 {
        let marked = self.mark_copies();
        let deleted = self.delete_marked_entries();
        debug_assert_eq!(marked, deleted);
        marked
    }

    pub fn delete_non_units(&mut self) -> i64 {
        for clause in &mut self.clauses {
            if clause.literal_number() > 1 {
                clause.set_prop(CP_DELETE_CLAUSE);
            } else {
                clause.del_prop(CP_DELETE_CLAUSE);
            }
        }
        self.delete_marked_entries()
    }

    pub fn filter_trivial(&mut self, bank: &TermBank) -> i64 {
        let trivial_slots = self
            .clauses
            .occupied_slots()
            .filter(|&slot| {
                self.clauses
                    .get_slot(slot)
                    .is_some_and(|clause| clause.is_trivial(bank))
            })
            .collect::<Vec<_>>();
        for slot in &trivial_slots {
            let _ = self.extract_at_slot(*slot);
        }
        self.compact_clause_store_if_sparse();
        usize_to_i64(trivial_slots.len())
    }

    /// Removes clauses proved tautological by E's ground-completion check.
    ///
    /// This plain `ClauseSet` currently has no demodulator index, matching the
    /// C precondition asserted by `ClauseSetFilterTautologies`.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if copying a candidate clause into `work_bank` or
    /// inserting normalized terms fails.
    ///
    /// # Panics
    ///
    /// Panics if this set owns a demodulator index, matching the C
    /// `ClauseSetFilterTautologies` precondition for plain clause sets.
    pub fn filter_tautologies(&mut self, work_bank: &mut TermBank) -> Result<i64, Diagnostic> {
        assert!(
            self.demod_index.is_none(),
            "ClauseSetFilterTautologies expects a plain, non-demod-indexed set"
        );
        let slots = self.clauses.occupied_slots().collect::<Vec<_>>();
        let mut removed = 0;
        for slot in slots {
            let clause = self
                .clauses
                .get_slot(slot)
                .expect("occupied clause slot must contain a clause");
            if clause_is_tautology(work_bank, clause)? {
                let _ = self.extract_at_slot(slot);
                removed += 1;
            }
        }
        self.compact_clause_store_if_sparse();
        Ok(removed)
    }

    #[must_use]
    pub fn term_nodes(&self, bank: &TermBank) -> i64 {
        self.clauses
            .iter()
            .map(|clause| {
                clause_weight_to_i64(clause.literal_weight(bank, 1.0, 1.0, 1.0, 1, 1, 1.0, true))
            })
            .sum()
    }

    pub fn mark_sos(&mut self, tptp_types: bool) -> i64 {
        let mut result = 0;
        for clause in &mut self.clauses {
            if (tptp_types && clause.query_tptp_type() == CP_TYPE_CONJECTURE)
                || (!tptp_types && clause.is_goal())
            {
                clause.set_prop(CP_IS_SOS);
                result += 1;
            } else {
                clause.del_prop(CP_IS_SOS);
            }
        }
        result
    }

    /// Orient and mark maximal literals in every clause in set order.
    pub fn mark_maximal_terms(&mut self, ocb: &mut OrderControlBlock, bank: &TermBank) {
        for clause in &mut self.clauses {
            clause.mark_maximal_terms(ocb, bank);
        }
    }

    /// Orient and mark maximal literals in every clause in set order using
    /// bank-backed ordering preparation when needed.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if bank-backed term ordering preparation fails.
    pub fn mark_maximal_terms_with_bank(
        &mut self,
        ocb: &mut OrderControlBlock,
        bank: &mut TermBank,
    ) -> Result<(), Diagnostic> {
        for clause in &mut self.clauses {
            clause.mark_maximal_terms_with_bank(ocb, bank)?;
        }
        Ok(())
    }

    pub fn term_set_prop(&self, prop: TermProperties) {
        for clause in &self.clauses {
            clause.term_set_prop(prop);
        }
    }

    #[must_use]
    pub fn tb_term_prop_del_count(&self, prop: TermProperties) -> i64 {
        self.clauses
            .iter()
            .map(|clause| clause.tb_term_del_prop_count(prop))
            .sum()
    }

    #[must_use]
    pub fn shared_term_nodes(&self) -> i64 {
        self.term_set_prop(crate::terms::termtypes::TP_OP_FLAG);
        self.tb_term_prop_del_count(crate::terms::termtypes::TP_OP_FLAG)
    }

    pub fn add_symbol_distribution(&self, dist_array: &mut [i64]) {
        for clause in &self.clauses {
            clause.add_symbol_distribution(dist_array);
        }
    }

    pub fn add_type_distribution(&self, sig: &mut Signature, type_array: &mut [i64]) {
        for clause in &self.clauses {
            clause.add_type_distribution(sig, type_array);
        }
    }

    pub fn add_conj_symbol_distribution(&self, dist_array: &mut [i64]) {
        for clause in &self.clauses {
            if clause.is_conjecture() {
                clause.add_symbol_distribution(dist_array);
            }
        }
    }

    pub fn add_axiom_symbol_distribution(&self, dist_array: &mut [i64]) {
        for clause in &self.clauses {
            if !clause.is_conjecture() {
                clause.add_symbol_distribution(dist_array);
            }
        }
    }

    pub fn compute_function_ranks(&self, rank_array: &mut [i64], count: &mut i64) {
        for clause in &self.clauses {
            clause.compute_function_ranks(rank_array, count);
        }
    }

    pub fn find_char_freq_vectors(
        &self,
        fsum: &mut FreqVector,
        fmax: &mut FreqVector,
        fmin: &mut FreqVector,
        cspec: &FvCollect,
    ) -> i64 {
        fsum.initialize(0);
        fmax.initialize(0);
        fmin.initialize(i64::MAX);

        for clause in &self.clauses {
            let current = var_freq_vector_compute(clause, cspec);
            let old_sum = fsum.clone();
            fsum.add_from(&old_sum, &current);
            let old_max = fmax.clone();
            fmax.max_from(&old_max, &current);
            let old_min = fmin.clone();
            fmin.min_from(&old_min, &current);
        }
        self.members()
    }

    #[must_use]
    pub fn perm_vector_compute(
        &self,
        cspec: &FvCollect,
        eliminate_uninformative: bool,
    ) -> Option<PermVector> {
        if cspec.features() == FvIndexType::NoFeatures {
            return None;
        }

        let vector_len = if cspec.features() == FvIndexType::CollectFeatures {
            cspec.result_len()
        } else {
            fv_size(cspec.max_symbols(), cspec.features())
        };
        let mut fsum = FreqVector::new(vector_len);
        let mut fmax = FreqVector::new(vector_len);
        let mut fmin = FreqVector::new(vector_len);
        self.find_char_freq_vectors(&mut fsum, &mut fmax, &mut fmin, cspec);
        Some(perm_vector_compute_internal(
            &fmax,
            &fmin,
            &fsum,
            cspec.max_symbols(),
            eliminate_uninformative,
        ))
    }

    #[must_use]
    pub fn find_freq_symbol(&self, sig: &Signature, arity: i32, least: bool) -> FunCode {
        let Some(dist_size) = sig
            .f_count()
            .checked_add(1)
            .and_then(|size| usize::try_from(size).ok())
        else {
            return 0;
        };
        let mut dist_array = vec![0; dist_size];
        self.add_symbol_distribution(&mut dist_array);

        let mut selected = 0;
        let mut frequency = if least { i64::MAX } else { 0 };
        for f_code in (sig.internal_symbols() + 1)..=sig.f_count() {
            if sig.find_arity(f_code) == Some(arity)
                && !sig.is_predicate(f_code)
                && !sig.is_special(f_code)
            {
                let Some(index) = f_code_index(f_code) else {
                    continue;
                };
                let symbol_frequency = dist_array[index];
                if (least && symbol_frequency <= frequency)
                    || (!least && symbol_frequency >= frequency)
                {
                    frequency = symbol_frequency;
                    selected = f_code;
                }
            }
        }
        selected
    }

    pub fn apply_fun(&self, mut fun: impl FnMut(&Clause) -> i64) -> i64 {
        let mut result = false;
        for clause in &self.clauses {
            result = (i64::from(result) + fun(clause)) != 0;
        }
        i64::from(result)
    }

    #[must_use]
    pub fn max_var_number(&self) -> i64 {
        self.clauses
            .iter()
            .map(|clause| {
                let mut variables = BTreeMap::new();
                clause.collect_variables(&mut variables)
            })
            .max()
            .unwrap_or(0)
    }

    #[must_use]
    pub fn standard_weight(&self) -> i64 {
        self.clauses.iter().map(Clause::standard_weight).sum()
    }

    pub fn default_weigh_clauses(&mut self) {
        for clause in &mut self.clauses {
            clause.set_weight(clause.standard_weight());
        }
    }

    #[must_use]
    pub fn find_max_standard_weight(&self) -> Option<&Clause> {
        let mut max_weight = 0;
        let mut result = None;
        for clause in &self.clauses {
            let weight = clause.standard_weight();
            if weight > max_weight {
                max_weight = weight;
                result = Some(clause);
            }
        }
        result
    }

    #[must_use]
    pub fn find_eq_definition(&self, bank: &TermBank, min_arity: usize) -> Option<ClausePos> {
        self.find_eq_definition_from_index(bank, min_arity, 0)
    }

    #[must_use]
    pub fn find_eq_definition_from_id(
        &self,
        bank: &TermBank,
        min_arity: usize,
        start_ident: i64,
    ) -> Option<ClausePos> {
        let slot = self.slot_by_id(start_ident)?;
        let start = self.clauses.position_of_slot(slot)?;
        self.find_eq_definition_from_index(bank, min_arity, start)
    }

    pub fn new_terms(&mut self, bank: &mut TermBank) -> Result<i64, Diagnostic> {
        let mut stack = Vec::with_capacity(self.clauses.len());
        while let Some(clause) = self.extract_first() {
            stack.push(clause);
        }

        while let Some(clause) = stack.pop() {
            let mut copy = clause.copy_to_bank(bank)?;
            copy.set_weight(copy.standard_weight());
            debug_assert_eq!(copy.weight(), copy.standard_weight());
            self.insert(copy);
        }
        Ok(self.members())
    }

    pub fn fv_indexify(&mut self, anchor: &mut FvIndexAnchor, bank: &TermBank) -> i64 {
        let mut stack = Vec::with_capacity(self.clauses.len());
        while let Some(clause) = self.extract_first() {
            stack.push(clause);
        }

        while let Some(clause) = stack.pop() {
            self.indexed_insert_clause(clause, Some(&mut *anchor), bank);
        }
        self.members()
    }

    /// Rebuilds this set through its owned feature-vector index.
    ///
    /// # Panics
    ///
    /// Panics if no feature-vector anchor is installed on the set.
    pub fn fv_indexify_owned(&mut self, bank: &TermBank) -> i64 {
        assert!(
            self.fv_anchor.is_some(),
            "FV indexify requires an installed FV anchor"
        );
        let mut stack = Vec::with_capacity(self.clauses.len());
        while let Some(clause) = self.extract_first() {
            stack.push(clause);
        }

        while let Some(clause) = stack.pop() {
            self.indexed_insert_clause_owned(clause, bank);
        }
        self.members()
    }

    pub fn push_clause_refs<'a>(&'a self, stack: &mut PStack<&'a Clause>) -> i64 {
        let mut pushed = 0;
        for clause in &self.clauses {
            stack.push(clause);
            pushed += 1;
        }
        pushed
    }

    pub fn push_clauses<'a>(&'a self, stack: &mut PStack<&'a Clause>) -> i64 {
        self.push_clause_refs(stack)
    }

    pub fn split_conjecture_refs<'a>(
        &'a self,
        conjectures: &mut Vec<&'a Clause>,
        rest: &mut Vec<&'a Clause>,
    ) -> i64 {
        let mut found = 0;
        for clause in &self.clauses {
            if clause.is_conjecture() {
                conjectures.push(clause);
                found += 1;
            } else {
                rest.push(clause);
            }
        }
        found
    }

    pub fn count_conjectures(&self, hypos: &mut i64) -> i64 {
        let mut conjectures = 0;
        for clause in &self.clauses {
            if clause.is_conjecture() {
                conjectures += 1;
            }
            if clause.is_hypothesis() {
                *hypos += 1;
            }
        }
        conjectures
    }

    #[must_use]
    pub fn conjecture_order(&self, sig: &Signature) -> usize {
        let mut order = 0;
        for clause in &self.clauses {
            for literal in clause.literals().as_slice() {
                order = order.max(term_compute_order(sig, literal.left()));
                order = order.max(term_compute_order(sig, literal.right()));
            }
        }
        order
    }

    #[must_use]
    pub fn is_untyped(&self) -> bool {
        self.clauses.iter().all(Clause::is_untyped)
    }

    fn find_eq_definition_from_index(
        &self,
        bank: &TermBank,
        min_arity: usize,
        start: usize,
    ) -> Option<ClausePos> {
        for clause in self.clauses.iter().skip(start) {
            let side = clause.is_eq_definition(bank, min_arity);
            if side != EqnSide::NoSide {
                let mut pos = ClausePos::for_clause(clause.clone());
                pos.set_side(side);
                return Some(pos);
            }
        }
        None
    }

    fn slot_by_id(&self, ident: i64) -> Option<ClauseSlot> {
        self.clauses.occupied_slots().find(|&slot| {
            self.clauses
                .get_slot(slot)
                .is_some_and(|clause| clause.ident() == ident)
        })
    }

    fn find_by_eval_object(&self, object: EvalObjectHandle) -> Option<&Clause> {
        let slot = self.eval_object_slots.get(object).copied().flatten()?;
        self.clauses.get_slot(slot)
    }

    fn find_by_eval_object_mut(&mut self, object: EvalObjectHandle) -> Option<&mut Clause> {
        self.demod_index_coverage.set(None);
        let slot = self.eval_object_slots.get(object).copied().flatten()?;
        self.clauses.get_slot_mut(slot)
    }

    fn extract_at_slot(&mut self, slot: ClauseSlot) -> Option<Clause> {
        self.demod_index_coverage.set(None);
        let clause = self.clauses.get_slot(slot)?;
        let entries = eval_index_entries(clause);
        let eval_object = clause.evaluations().and_then(EvalCell::object);
        let ident = clause.ident();
        for (pos, entry) in entries {
            if let Some(root) = self.eval_indices.get_mut(pos) {
                root.remove(&entry);
            }
        }
        if let Some(object) = eval_object {
            if let Some(mapped_slot) = self.eval_object_slots.get_mut(object) {
                *mapped_slot = None;
            }
        }
        let mut clause = self.clauses.remove_slot(slot)?;
        self.delete_clause_demodulator_index(&mut clause);
        if clause.query_prop(CP_IS_S_INDEXED) {
            if let Some(anchor) = self.fv_anchor.as_mut() {
                anchor.delete(&clause);
            }
            clause.del_prop(CP_IS_S_INDEXED);
        }
        self.literals -= usize_to_i64(clause.literal_number());
        if self.indexed_clause_positions.get(&ident) == Some(&slot) {
            let next_slot = self.slot_by_id(ident);
            if let Some(next_slot) = next_slot {
                self.indexed_clause_positions.insert(ident, next_slot);
            } else {
                self.indexed_clause_positions.remove(&ident);
            }
        }
        Some(clause)
    }

    fn compact_clause_store_if_sparse(&mut self) {
        if self.clauses.compact_if_sparse() {
            self.rebuild_indexed_clause_positions();
            self.rebuild_eval_object_slots();
        }
    }

    fn index_clause_demodulator(&mut self, clause: &mut Clause) {
        let Some(index) = self.demod_index.as_mut() else {
            return;
        };
        index_demodulator_clause(index, clause);
        clause.set_prop(CP_IS_D_INDEXED);
    }

    fn delete_clause_demodulator_index(&mut self, clause: &mut Clause) {
        if !clause.query_prop(CP_IS_D_INDEXED) {
            return;
        }
        if let Some(index) = self.demod_index.as_mut() {
            delete_demodulator_clause(index, clause);
        }
        clause.del_prop(CP_IS_D_INDEXED);
    }

    fn demod_index_covers_units(&self) -> bool {
        if let Some(covers) = self.demod_index_coverage.get() {
            return covers;
        }
        let covers = self
            .clauses
            .iter()
            .all(|clause| !clause.is_unit() || clause.query_prop(CP_IS_D_INDEXED));
        self.demod_index_coverage.set(Some(covers));
        covers
    }

    fn rebuild_indexed_clause_positions(&mut self) {
        self.indexed_clause_positions.clear();
        if self.demod_index.is_none() {
            return;
        }
        for slot in self.clauses.occupied_slots() {
            let clause = self
                .clauses
                .get_slot(slot)
                .expect("occupied clause slot must contain a clause");
            self.indexed_clause_positions
                .entry(clause.ident())
                .or_insert(slot);
        }
    }

    fn rebuild_eval_object_slots(&mut self) {
        self.eval_object_slots.clear();
        for slot in self.clauses.occupied_slots() {
            let Some(object) = self
                .clauses
                .get_slot(slot)
                .and_then(Clause::evaluations)
                .and_then(EvalCell::object)
            else {
                continue;
            };
            if self.eval_object_slots.len() <= object {
                self.eval_object_slots.resize(object + 1, None);
            }
            self.eval_object_slots[object] = Some(slot);
        }
    }

    pub(crate) fn recompute_literals(&mut self) {
        self.literals = self
            .clauses
            .iter()
            .map(|clause| usize_to_i64(clause.literal_number()))
            .sum();
    }
}

fn index_demodulator_clause(index: &mut PdTree, clause: &Clause) {
    assert!(
        clause.is_unit(),
        "demodulator-indexed clauses must be units"
    );
    let literal = clause
        .literals()
        .as_slice()
        .first()
        .expect("unit clause has one literal");
    index.insert_term_occurrence(
        literal.left(),
        clause.date(),
        PdtIndexedOccurrence::new(clause.ident(), EqnSide::LeftSide),
    );
    if !literal.is_oriented() {
        index.insert_term_occurrence(
            literal.right(),
            clause.date(),
            PdtIndexedOccurrence::new(clause.ident(), EqnSide::RightSide),
        );
    }
}

fn delete_demodulator_clause(index: &mut PdTree, clause: &Clause) {
    assert!(
        clause.is_unit(),
        "demodulator-indexed clauses must be units"
    );
    let literal = clause
        .literals()
        .as_slice()
        .first()
        .expect("unit clause has one literal");
    let _ = index.delete_term_occurrence(
        literal.left(),
        clause.date(),
        PdtIndexedOccurrence::new(clause.ident(), EqnSide::LeftSide),
    );
    if !literal.is_oriented() {
        let _ = index.delete_term_occurrence(
            literal.right(),
            clause.date(),
            PdtIndexedOccurrence::new(clause.ident(), EqnSide::RightSide),
        );
    }
}

fn clause_set_render_clause_string(
    bank: &TermBank,
    clause: &Clause,
    full_terms: bool,
    output_format: IoFormat,
    problem_type: ProblemType,
    options: EqnPrintOptions,
) -> Result<String, Diagnostic> {
    clause_print_format_string_with_options(
        bank,
        clause,
        full_terms,
        output_format,
        problem_type,
        options,
    )
}

fn indexed_clause_for_anchor(
    clause: Clause,
    fv_anchor: Option<&mut FvIndexAnchor>,
    bank: &TermBank,
) -> Clause {
    debug_assert_eq!(clause.weight(), clause.standard_weight());
    let mut clause = clause;
    if let Some(anchor) = fv_anchor {
        let mut packed = fv_index_pack_clause(clause, Some(&*anchor));
        anchor.insert(&mut packed, bank);
        clause = packed.into_clause();
        clause.set_prop(CP_IS_S_INDEXED);
    }
    clause
}

fn index_clause_evaluations(
    eval_indices: &mut Vec<BTreeSet<EvalIndexEntry>>,
    eval_no: &mut usize,
    next_eval_object: &mut EvalObjectHandle,
    clause: &mut Clause,
) {
    let Some(evaluations) = clause.evaluations_mut() else {
        return;
    };
    let object = *next_eval_object;
    *next_eval_object += 1;
    evaluations.set_object(Some(object));
    *eval_no = (*eval_no).max(evaluations.eval_no());
    while eval_indices.len() < evaluations.eval_no() {
        eval_indices.push(BTreeSet::new());
    }
    for (pos, root) in eval_indices
        .iter_mut()
        .enumerate()
        .take(evaluations.eval_no())
    {
        let _ = root.insert(EvalIndexEntry::from_eval(object, evaluations, pos));
    }
}

fn eval_index_entries(clause: &Clause) -> Vec<(usize, EvalIndexEntry)> {
    let Some(evaluations) = clause.evaluations() else {
        return Vec::new();
    };
    let Some(object) = evaluations.object() else {
        return Vec::new();
    };
    (0..evaluations.eval_no())
        .map(|pos| (pos, EvalIndexEntry::from_eval(object, evaluations, pos)))
        .collect()
}

fn cmp_f32_c(left: f32, right: f32) -> Ordering {
    if left > right {
        Ordering::Greater
    } else if left < right {
        Ordering::Less
    } else {
        Ordering::Equal
    }
}

#[must_use]
pub fn clause_set_stack_cardinality(stack: &PStack<ClauseSet>) -> i64 {
    stack.as_slice().iter().map(ClauseSet::members).sum()
}

#[must_use]
pub fn clause_set_ref_stack_cardinality(stack: &PStack<&ClauseSet>) -> i64 {
    stack.as_slice().iter().map(|set| set.members()).sum()
}

/// Returns the maximum date among the first `limit` clause sets.
///
/// # Panics
///
/// Panics if `limit` exceeds `sets.len()`, matching the C caller contract for
/// the demodulator array prefix.
#[must_use]
pub fn clause_set_list_get_max_date(sets: &[&ClauseSet], limit: usize) -> SysDate {
    assert!(
        limit <= sets.len(),
        "ClauseSetListGetMaxDate limit must fit the set slice"
    );
    sets.iter()
        .take(limit)
        .fold(SysDate::creation_time(), |date, set| {
            date.maximum(set.date())
        })
}

/// Writes the C `EqAxiomsPrint` shape with an explicit output format.
///
/// # Errors
///
/// Returns a diagnostic for TSTP, matching the C fatal-error branch, or if the
/// formatter fails.
pub fn eq_axioms_write(
    output: &mut impl fmt::Write,
    sig: &Signature,
    format: IoFormat,
    single_subst: bool,
) -> Result<(), Diagnostic> {
    match format {
        IoFormat::Tptp => {
            output
                .write_str(
                    "input_clause(eq_reflexive, axiom, [++equal(X,X)]).\n\
                     input_clause(eq_symmetric, axiom, [++equal(X,Y),--equal(Y,X)]).\n\
                     input_clause(eq_transitive, axiom, [++equal(X,Z),--equal(X,Y),--equal(Y,Z)]).\n",
                )
                .map_err(eq_axioms_write_error)?;
            for f_code in (sig.internal_symbols() + 1)..=sig.f_count() {
                let Some(arity) = sig.find_arity(f_code) else {
                    continue;
                };
                if arity == 0 {
                    continue;
                }
                let Some(symbol) = sig.find_name(f_code) else {
                    continue;
                };
                if sig.is_predicate(f_code) {
                    tptp_eq_pred_axiom_write(output, symbol, arity, single_subst)?;
                } else {
                    tptp_eq_func_axiom_write(output, symbol, arity, single_subst)?;
                }
            }
        }
        IoFormat::Tstp => {
            return Err(Diagnostic::new(
                ErrorCode::OTHER_ERROR,
                "Adding of equality axioms not (yet) supported for TSTP/TPTP-3 format.",
            ));
        }
        IoFormat::Auto | IoFormat::Lop => {
            output
                .write_str(
                    "equal(X,X) <- .\n\
                     equal(X,Y) <- equal(Y,X).\n\
                     equal(X,Z) <- equal(X,Y), equal(Y,Z).\n",
                )
                .map_err(eq_axioms_write_error)?;
            for f_code in (sig.internal_symbols() + 1)..=sig.f_count() {
                let Some(arity) = sig.find_arity(f_code) else {
                    continue;
                };
                if arity == 0 {
                    continue;
                }
                let Some(symbol) = sig.find_name(f_code) else {
                    continue;
                };
                if sig.is_predicate(f_code) {
                    eq_pred_axiom_write(output, symbol, arity, single_subst)?;
                } else {
                    eq_func_axiom_write(output, symbol, arity, single_subst)?;
                }
            }
        }
    }
    Ok(())
}

/// Returns the C `EqAxiomsPrint` shape with an explicit output format.
///
/// # Errors
///
/// Returns a diagnostic under the same conditions as [`eq_axioms_write`].
pub fn eq_axioms_print_string(
    sig: &Signature,
    format: IoFormat,
    single_subst: bool,
) -> Result<String, Diagnostic> {
    let mut output = String::new();
    eq_axioms_write(&mut output, sig, format, single_subst)?;
    Ok(output)
}

fn usize_to_i64(value: usize) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

fn eval_mem(eval_no: usize) -> i64 {
    32_i64.saturating_add(usize_to_i64(eval_no).saturating_mul(4))
}

fn f_code_index(f_code: FunCode) -> Option<usize> {
    usize::try_from(f_code).ok()
}

#[allow(clippy::cast_possible_truncation)]
fn clause_weight_to_i64(weight: f64) -> i64 {
    weight as i64
}

fn clause_set_tstp_write_error(_error: fmt::Error) -> Diagnostic {
    Diagnostic::new(ErrorCode::OTHER_ERROR, "failed to write TSTP clause set")
}

fn eq_axioms_write_error(_error: fmt::Error) -> Diagnostic {
    Diagnostic::new(ErrorCode::OTHER_ERROR, "failed to write equality axioms")
}

fn print_var_pattern(
    output: &mut impl fmt::Write,
    symbol: &str,
    arity: i32,
    var: &str,
    alt_var: Option<&str>,
    exception: i32,
) -> Result<(), Diagnostic> {
    write!(output, "{symbol}(").map_err(eq_axioms_write_error)?;
    for i in 1..=arity {
        if i != 1 {
            output.write_str(",").map_err(eq_axioms_write_error)?;
        }
        if i == exception {
            output
                .write_str(alt_var.unwrap_or(""))
                .map_err(eq_axioms_write_error)?;
        } else {
            write!(output, "{var}{i}").map_err(eq_axioms_write_error)?;
        }
    }
    output.write_str(")").map_err(eq_axioms_write_error)
}

fn eq_func_axiom_write(
    output: &mut impl fmt::Write,
    symbol: &str,
    arity: i32,
    single_subst: bool,
) -> Result<(), Diagnostic> {
    if single_subst {
        for i in 1..=arity {
            output.write_str("equal(").map_err(eq_axioms_write_error)?;
            print_var_pattern(output, symbol, arity, "X", Some("Y"), i)?;
            output.write_str(",").map_err(eq_axioms_write_error)?;
            print_var_pattern(output, symbol, arity, "X", Some("Z"), i)?;
            output
                .write_str(") <- equal(Y,Z).\n")
                .map_err(eq_axioms_write_error)?;
        }
    } else {
        output.write_str("equal(").map_err(eq_axioms_write_error)?;
        print_var_pattern(output, symbol, arity, "X", None, 0)?;
        output.write_str(",").map_err(eq_axioms_write_error)?;
        print_var_pattern(output, symbol, arity, "Y", None, 0)?;
        output.write_str(") <- ").map_err(eq_axioms_write_error)?;
        for i in 1..=arity {
            if i != 1 {
                output.write_str(",").map_err(eq_axioms_write_error)?;
            }
            write!(output, "equal(X{i},Y{i})").map_err(eq_axioms_write_error)?;
        }
        output.write_str(".\n").map_err(eq_axioms_write_error)?;
    }
    Ok(())
}

fn eq_pred_axiom_write(
    output: &mut impl fmt::Write,
    symbol: &str,
    arity: i32,
    single_subst: bool,
) -> Result<(), Diagnostic> {
    if single_subst {
        for i in 1..=arity {
            print_var_pattern(output, symbol, arity, "X", Some("Y"), i)?;
            output.write_str(" <- ").map_err(eq_axioms_write_error)?;
            print_var_pattern(output, symbol, arity, "X", Some("Z"), i)?;
            output
                .write_str(", equal(Y,Z).\n")
                .map_err(eq_axioms_write_error)?;
        }
    } else {
        print_var_pattern(output, symbol, arity, "X", None, 0)?;
        output.write_str(" <- ").map_err(eq_axioms_write_error)?;
        print_var_pattern(output, symbol, arity, "Y", None, 0)?;
        for i in 1..=arity {
            write!(output, ",equal(X{i},Y{i})").map_err(eq_axioms_write_error)?;
        }
        output.write_str(".\n").map_err(eq_axioms_write_error)?;
    }
    Ok(())
}

fn tptp_eq_func_axiom_write(
    output: &mut impl fmt::Write,
    symbol: &str,
    arity: i32,
    single_subst: bool,
) -> Result<(), Diagnostic> {
    if single_subst {
        for i in 1..=arity {
            write!(
                output,
                "input_clause(eq_subst_{symbol}{i}, axiom, [++equal("
            )
            .map_err(eq_axioms_write_error)?;
            print_var_pattern(output, symbol, arity, "X", Some("Y"), i)?;
            output.write_str(",").map_err(eq_axioms_write_error)?;
            print_var_pattern(output, symbol, arity, "X", Some("Z"), i)?;
            output
                .write_str("),--equal(Y,Z)]).\n")
                .map_err(eq_axioms_write_error)?;
        }
    } else {
        write!(output, "input_clause(eq_subst_{symbol}, axiom, [++equal(")
            .map_err(eq_axioms_write_error)?;
        print_var_pattern(output, symbol, arity, "X", None, 0)?;
        output.write_str(",").map_err(eq_axioms_write_error)?;
        print_var_pattern(output, symbol, arity, "Y", None, 0)?;
        output.write_str(")").map_err(eq_axioms_write_error)?;
        for i in 1..=arity {
            write!(output, ",--equal(X{i},Y{i})").map_err(eq_axioms_write_error)?;
        }
        output.write_str("]).\n").map_err(eq_axioms_write_error)?;
    }
    Ok(())
}

fn tptp_eq_pred_axiom_write(
    output: &mut impl fmt::Write,
    symbol: &str,
    arity: i32,
    single_subst: bool,
) -> Result<(), Diagnostic> {
    if single_subst {
        for i in 1..=arity {
            write!(output, "input_clause(eq_subst_{symbol}{i}, axiom, [++")
                .map_err(eq_axioms_write_error)?;
            print_var_pattern(output, symbol, arity, "X", Some("Y"), i)?;
            output.write_str(",--").map_err(eq_axioms_write_error)?;
            print_var_pattern(output, symbol, arity, "X", Some("Z"), i)?;
            output
                .write_str(",--equal(Y,Z)]).\n")
                .map_err(eq_axioms_write_error)?;
        }
    } else {
        write!(output, "input_clause(eq_subst_{symbol}, axiom, [++")
            .map_err(eq_axioms_write_error)?;
        print_var_pattern(output, symbol, arity, "X", None, 0)?;
        output.write_str(",--").map_err(eq_axioms_write_error)?;
        print_var_pattern(output, symbol, arity, "Y", None, 0)?;
        for i in 1..=arity {
            write!(output, ",--equal(X{i},Y{i})").map_err(eq_axioms_write_error)?;
        }
        output.write_str("]).\n").map_err(eq_axioms_write_error)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        clause_set_list_get_max_date, clause_set_ref_stack_cardinality,
        clause_set_stack_cardinality, eq_axioms_print_string, eval_mem, ClauseSet,
        CLAUSECELL_DYN_MEM, EQN_CELL_MEM,
    };
    use crate::basics::partial_orderings::HoOrderKind;
    use crate::basics::pstacks::PStack;
    use crate::basics::simple_stuff::{reset_problem_type, set_problem_type, ProblemType};
    use crate::basics::sysdate::SysDate;
    use crate::clauses::clause::Clause;
    use crate::clauses::clause_props::{
        CP_DELETE_CLAUSE, CP_INITIAL, CP_IS_D_INDEXED, CP_IS_ORIENTED, CP_IS_SOS, CP_IS_S_INDEXED,
        CP_TYPE_AXIOM, CP_TYPE_CONJECTURE, CP_TYPE_HYPOTHESIS, CP_TYPE_NEG_CONJECTURE,
    };
    use crate::clauses::derivation::{DerivationEntry, DC_CNF_EVAL_GC};
    use crate::clauses::eqn::{Eqn, EqnPrintOptions};
    use crate::clauses::eqn_props::{EqnSide, EP_IS_MAXIMAL, EP_IS_ORIENTED};
    use crate::clauses::eqnlist::EqnList;
    use crate::clauses::fcvindexing::FvIndexAnchor;
    use crate::clauses::freqvectors::{
        fv_size, perm_vector_compute_internal, var_freq_vector_compute, FreqVector, FvCollect,
        FvCollectLayout, FvIndexType,
    };
    use crate::clauses::neweval::{evals_alloc, EvalCell};
    use crate::clauses::pdtrees::{PdtIndexedOccurrence, PDTREE_IGNORE_NF_DATE};
    use crate::heuristics::to_params::TermOrdering;
    use crate::inout::scanner::{IoFormat, Scanner};
    use crate::orderings::ocb::OrderControlBlock;
    use crate::terms::lambda::{apply_terms, close_with_db_var};
    use crate::terms::signature::Signature;
    use crate::terms::simpletypes::alloc_arrow_type;
    use crate::terms::termbanks::TermBank;
    use crate::terms::termfunc::term_standard_weight;
    use crate::terms::termtypes::{DerefType, Term, TP_CHECK_FLAG};
    use crate::terms::typebanks::TypeBank;
    use crate::test_support::global_state_lock;

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
        if bank.signature().get_type(f_code).is_none() {
            bank.signature_mut()
                .declare_final_type(f_code, type_.clone())
                .unwrap();
        }
        bank.create_const_term(f_code).unwrap()
    }

    fn typed_var(bank: &TermBank, f_code: i64) -> Term {
        let type_ = bank.signature().type_bank().default_type();
        bank.vars().var_assert_alloc(f_code, &type_)
    }

    fn typed_unary(bank: &mut TermBank, name: &str, arg: &Term) -> Term {
        let type_ = bank.signature().type_bank().default_type();
        let f_code = bank.signature_mut().insert_id(name, 1, false);
        if bank.signature().get_type(f_code).is_none() {
            bank.signature_mut()
                .declare_final_type(f_code, alloc_arrow_type(vec![type_.clone(), type_.clone()]))
                .unwrap();
        }
        let term = Term::top_alloc(f_code, 1);
        term.set_type(Some(type_));
        term.set_argument(0, arg.clone());
        bank.insert(&term, DerefType::Never).unwrap()
    }

    fn literal(bank: &mut TermBank, left: &Term, right: &Term, positive: bool) -> Eqn {
        Eqn::alloc(left.clone(), right.clone(), bank, positive).unwrap()
    }

    fn clause_from(literals: Vec<Eqn>) -> Clause {
        let mut clause = Clause::alloc(EqnList::from_vec(literals));
        clause.set_weight(clause.standard_weight());
        clause
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

    fn clause_with_evaluations(mut clause: Clause, values: &[(i64, f32)]) -> Clause {
        let mut evaluations = evals_alloc(values.len());
        for (pos, &(priority, heuristic)) in values.iter().enumerate() {
            evaluations.eval_mut(pos).set_priority(priority);
            evaluations.eval_mut(pos).set_heuristic(heuristic);
        }
        clause.add_eval_cell(evaluations);
        clause
    }

    fn declare_typed_function(bank: &mut TermBank, name: &str, arity: usize) {
        let type_ = bank.signature().type_bank().default_type();
        let f_code = bank
            .signature_mut()
            .insert_id(name, i32::try_from(arity).unwrap(), false);
        let mut args = vec![type_.clone(); arity];
        args.push(type_);
        bank.signature_mut()
            .declare_final_type(f_code, alloc_arrow_type(args))
            .unwrap();
    }

    fn declare_typed_predicate(bank: &mut TermBank, name: &str, arity: usize) {
        let arg_type = bank.signature().type_bank().default_type();
        let bool_type = bank.signature().type_bank().bool_type();
        let f_code = bank
            .signature_mut()
            .insert_id(name, i32::try_from(arity).unwrap(), false);
        let mut args = vec![arg_type; arity];
        args.push(bool_type);
        bank.signature_mut()
            .declare_final_type(f_code, alloc_arrow_type(args))
            .unwrap();
    }

    fn ac_anchor(max_symbols: usize) -> FvIndexAnchor {
        let mut cspec = FvCollect::new(FvCollectLayout::new(FvIndexType::AcFeatures, false, 0, 0));
        cspec.set_max_symbols(max_symbols);
        FvIndexAnchor::new(cspec, None)
    }

    #[test]
    fn insert_extract_and_transfer_preserve_order_and_accounting() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let c = typed_const(&mut bank, "c");
        let first = clause_from(vec![literal(&mut bank, &a, &b, true)]);
        let second = clause_from(vec![
            literal(&mut bank, &b, &c, true),
            literal(&mut bank, &c, &a, false),
        ]);
        let first_id = first.ident();
        let second_id = second.ident();
        let mut set = ClauseSet::new();

        assert_eq!(set.date(), SysDate::from_raw(1));
        assert!(set.is_empty());
        set.insert(first);
        set.insert(second);

        assert_eq!(set.members(), 2);
        assert_eq!(set.len(), 2);
        assert_eq!(set.literals(), 3);
        assert_eq!(
            set.iter().map(Clause::ident).collect::<Vec<_>>(),
            vec![first_id, second_id]
        );
        assert_eq!(
            set.find_by_id(second_id).map(Clause::ident),
            Some(second_id)
        );

        let extracted = set.extract_first().unwrap();
        assert_eq!(extracted.ident(), first_id);
        assert_eq!(set.members(), 1);
        assert_eq!(set.literals(), 2);

        let mut target = ClauseSet::from_clauses([extracted]);
        assert_eq!(target.insert_set(&mut set), 1);
        assert!(set.is_empty());
        assert_eq!(
            target.iter().map(Clause::ident).collect::<Vec<_>>(),
            vec![first_id, second_id]
        );
        assert_eq!(target.literals(), 3);
    }

    #[test]
    fn storage_estimate_uses_c_clause_set_storage_shape() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "storage_a");
        let b = typed_const(&mut bank, "storage_b");
        let c = typed_const(&mut bank, "storage_c");
        let first = clause_with_evaluations(
            clause_from(vec![literal(&mut bank, &a, &b, true)]),
            &[(10, 1.0), (20, 2.0)],
        );
        let second = clause_from(vec![
            literal(&mut bank, &b, &c, true),
            literal(&mut bank, &c, &a, false),
        ]);
        let mut set = ClauseSet::new();

        set.insert(first);
        set.insert(second);

        assert_eq!(set.eval_no(), 2);
        assert_eq!(
            set.storage_estimate(),
            (CLAUSECELL_DYN_MEM + eval_mem(2)) * 2 + EQN_CELL_MEM * 3
        );
    }

    #[test]
    fn storage_estimate_includes_owned_fv_index_storage() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "storage_idx_a");
        let b = typed_const(&mut bank, "storage_idx_b");
        let max_symbols = usize::try_from(bank.signature().f_count() + 1).unwrap();
        let mut set = ClauseSet::new();
        set.set_fv_anchor(Some(ac_anchor(max_symbols)));

        set.indexed_insert_clause_owned(clause_from(vec![literal(&mut bank, &a, &b, true)]), &bank);

        let fv_storage = i64::try_from(set.fv_anchor().unwrap().storage_estimate()).unwrap();
        assert_eq!(
            set.storage_estimate(),
            CLAUSECELL_DYN_MEM + eval_mem(0) + EQN_CELL_MEM + fv_storage
        );
    }

    #[test]
    fn storage_estimate_includes_owned_demod_index_storage() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "storage_demod_a");
        let b = typed_const(&mut bank, "storage_demod_b");
        let mut literal = literal(&mut bank, &a, &b, true);
        literal.set_prop(EP_IS_ORIENTED);
        let clause = clause_from(vec![literal]);
        let mut set = ClauseSet::new_demod_indexed();

        set.indexed_insert_clause_owned(clause, &bank);

        let stored = set.iter().next().unwrap();
        let demod_storage = i64::try_from(set.demod_index_storage_estimate()).unwrap();
        assert!(stored.query_prop(CP_IS_D_INDEXED));
        assert_eq!(set.demod_index().unwrap().term_count(), 1);
        assert!(demod_storage > 0);
        assert_eq!(
            set.storage_estimate(),
            CLAUSECELL_DYN_MEM + eval_mem(0) + EQN_CELL_MEM + demod_storage
        );
    }

    #[test]
    fn derivation_stack_statistics_preserves_c_pdarray_print_shape() {
        let no_derivation = Clause::empty();
        let mut depth_two = Clause::empty();
        depth_two
            .ensure_derivation()
            .push(DerivationEntry::Operation(DC_CNF_EVAL_GC));
        depth_two
            .ensure_derivation()
            .push(DerivationEntry::Operation(DC_CNF_EVAL_GC));
        let mut depth_eight = Clause::empty();
        {
            let derivation = depth_eight.ensure_derivation();
            for _ in 0..8 {
                derivation.push(DerivationEntry::Operation(DC_CNF_EVAL_GC));
            }
        }
        let set = ClauseSet::from_clauses([no_derivation, depth_two, depth_eight]);
        let mut output = Vec::new();

        set.write_derivation_stack_statistics(&mut output).unwrap();

        assert_eq!(
            String::from_utf8(output).unwrap(),
            "%     0:      1\n\
             %     1:      0\n\
             %     2:      1\n\
             %     3:      0\n\
             %     4:      0\n\
             %     5:      0\n\
             %     6:      0\n\
             %     7:      0\n\
             %     8:      1\n\
             %     9:      0\n\
             %    10:      0\n\
             %    11:      0\n\
             %    12:      0\n\
             %    13:      0\n\
             %    14:      0\n\
             %    15:      0\n\
             % Average over 3 clauses: 3.333333\n"
        );
    }

    #[test]
    fn extracting_demod_indexed_clause_removes_index_entry_and_flag() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "extract_demod_a");
        let b = typed_const(&mut bank, "extract_demod_b");
        let clause = clause_from(vec![literal(&mut bank, &a, &b, true)]);
        let mut set = ClauseSet::new_demod_indexed();

        set.indexed_insert_clause_owned(clause, &bank);
        assert_eq!(set.demod_index().unwrap().term_count(), 2);
        let indexed_storage = set.demod_index_storage_estimate();

        let extracted = set.extract_first().unwrap();

        assert!(!extracted.query_prop(CP_IS_D_INDEXED));
        assert_eq!(set.demod_index().unwrap().term_count(), 0);
        assert!(set.demod_index_storage_estimate() < indexed_storage);
        assert!(set.demod_index_storage_estimate() > 0);
    }

    #[test]
    fn indexed_clause_lookup_tracks_first_duplicate_after_removal() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "indexed_lookup_a");
        let b = typed_const(&mut bank, "indexed_lookup_b");
        let c = typed_const(&mut bank, "indexed_lookup_c");
        let mut first = clause_from(vec![literal(&mut bank, &a, &b, true)]);
        let mut second = clause_from(vec![literal(&mut bank, &a, &c, true)]);
        first.set_ident(7_001);
        second.set_ident(7_001);
        let first_generation = first.derivation_generation();
        let second_generation = second.derivation_generation();
        let mut set = ClauseSet::new_demod_indexed();

        set.indexed_insert_clause_owned(first, &bank);
        set.indexed_insert_clause_owned(second, &bank);

        assert_eq!(
            set.find_indexed_by_id(7_001)
                .map(Clause::derivation_generation),
            Some(first_generation)
        );
        assert_eq!(
            set.find_indexed_position_by_id(7_001)
                .map(|(position, clause)| (position, clause.derivation_generation())),
            Some((0, first_generation))
        );

        let extracted = set.extract_first().expect("first duplicate is present");
        assert_eq!(extracted.derivation_generation(), first_generation);
        assert_eq!(
            set.find_indexed_position_by_id(7_001)
                .map(|(position, clause)| (position, clause.derivation_generation())),
            Some((0, second_generation))
        );
    }

    #[test]
    fn demod_index_constraints_track_clause_dates_and_deletion() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "constraint_demod_a");
        let b = typed_const(&mut bank, "constraint_demod_b");
        let f_a = typed_unary(&mut bank, "constraint_demod_f", &a);
        let mut first = clause_from(vec![literal(&mut bank, &a, &b, true)]);
        let mut second_literal = literal(&mut bank, &f_a, &b, true);
        second_literal.set_prop(EP_IS_ORIENTED);
        let mut second = clause_from(vec![second_literal]);
        first.set_date(SysDate::from_raw(3));
        second.set_date(SysDate::from_raw(7));
        let second_id = second.ident();
        let mut set = ClauseSet::new_demod_indexed();

        set.indexed_insert_clause_owned(first, &bank);
        set.indexed_insert_clause_owned(second, &bank);

        let index = set.demod_index().expect("demod index initialized");
        assert_eq!(index.term_count(), 3);
        assert_eq!(index.size_constraint(), term_standard_weight(&a));
        assert_eq!(index.age_constraint(), SysDate::from_raw(7));

        let extracted = set
            .extract_by_id(second_id)
            .expect("second clause is indexed by identifier");

        assert!(!extracted.query_prop(CP_IS_D_INDEXED));
        let index = set.demod_index().expect("demod index remains initialized");
        assert_eq!(index.term_count(), 2);
        assert_eq!(index.size_constraint(), term_standard_weight(&a));
        assert_eq!(index.age_constraint(), SysDate::from_raw(3));
    }

    #[test]
    fn demod_index_search_may_have_match_uses_root_constraints() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "constraint_search_a");
        let b = typed_const(&mut bank, "constraint_search_b");
        let f_a = typed_unary(&mut bank, "constraint_search_f", &a);
        let mut literal = literal(&mut bank, &f_a, &b, true);
        literal.set_prop(EP_IS_ORIENTED);
        let mut clause = clause_from(vec![literal]);
        clause.set_date(SysDate::from_raw(7));
        let mut set = ClauseSet::new_demod_indexed();

        set.indexed_insert_clause_owned(clause, &bank);

        set.record_demod_index_search_init(&a, PDTREE_IGNORE_NF_DATE, false);
        assert!(!set.demod_index_search_may_have_match());
        set.record_demod_index_search_exit();

        set.record_demod_index_search_init(&f_a, SysDate::from_raw(6), false);
        assert!(set.demod_index_search_may_have_match());
        set.record_demod_index_search_exit();

        set.record_demod_index_search_init(&f_a, SysDate::from_raw(7), false);
        assert!(!set.demod_index_search_may_have_match());
        set.record_demod_index_search_exit();
    }

    #[test]
    fn demod_index_search_may_have_match_uses_trie_path_prune() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "path_constraint_a");
        let b = typed_const(&mut bank, "path_constraint_b");
        let f_a = typed_unary(&mut bank, "path_constraint_f", &a);
        let g_a = typed_unary(&mut bank, "path_constraint_g", &a);
        let mut literal = literal(&mut bank, &f_a, &b, true);
        literal.set_prop(EP_IS_ORIENTED);
        let clause = clause_from(vec![literal]);
        let mut set = ClauseSet::new_demod_indexed();

        set.indexed_insert_clause_owned(clause, &bank);

        set.record_demod_index_search_init(&g_a, PDTREE_IGNORE_NF_DATE, false);
        assert!(!set.demod_index_search_may_have_match());
        set.record_demod_index_search_exit();

        set.record_demod_index_search_init(&f_a, PDTREE_IGNORE_NF_DATE, false);
        assert!(set.demod_index_search_may_have_match());
        set.record_demod_index_search_exit();
    }

    #[test]
    fn demod_index_search_candidates_identify_current_clause_sides() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "candidate_side_a");
        let f_a = typed_unary(&mut bank, "candidate_side_f", &a);
        let g_a = typed_unary(&mut bank, "candidate_side_g", &a);
        let clause = clause_from(vec![literal(&mut bank, &f_a, &g_a, true)]);
        let clause_id = clause.ident();
        let mut set = ClauseSet::new_demod_indexed();

        set.indexed_insert_clause_owned(clause, &bank);

        set.record_demod_index_search_init(&f_a, PDTREE_IGNORE_NF_DATE, false);
        assert_eq!(
            set.demod_index_search_candidate_sides(),
            Some(vec![PdtIndexedOccurrence::new(
                clause_id,
                EqnSide::LeftSide
            )])
        );
        assert_eq!(
            set.demod_index_search_next_candidate_side(),
            Some(PdtIndexedOccurrence::new(clause_id, EqnSide::LeftSide))
        );
        assert_eq!(set.demod_index_search_next_candidate_side(), None);
        set.record_demod_index_search_exit();

        set.record_demod_index_search_init(&g_a, PDTREE_IGNORE_NF_DATE, false);
        assert_eq!(
            set.demod_index_search_candidate_sides(),
            Some(vec![PdtIndexedOccurrence::new(
                clause_id,
                EqnSide::RightSide
            )])
        );
        assert_eq!(
            set.demod_index_search_next_candidate_side(),
            Some(PdtIndexedOccurrence::new(clause_id, EqnSide::RightSide))
        );
        assert_eq!(set.demod_index_search_next_candidate_side(), None);
        set.record_demod_index_search_exit();

        let extracted = set
            .extract_by_id(clause_id)
            .expect("indexed clause remains extractable");
        assert!(!extracted.query_prop(CP_IS_D_INDEXED));

        set.record_demod_index_search_init(&f_a, PDTREE_IGNORE_NF_DATE, false);
        assert_eq!(set.demod_index_search_candidate_sides(), Some(Vec::new()));
        assert_eq!(set.demod_index_search_next_candidate_side(), None);
        set.record_demod_index_search_exit();
    }

    #[test]
    fn demod_index_search_may_have_match_falls_back_for_unindexed_demodulators() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "constraint_fallback_a");
        let b = typed_const(&mut bank, "constraint_fallback_b");
        let mut literal = literal(&mut bank, &a, &b, true);
        literal.set_prop(EP_IS_ORIENTED);
        let clause = clause_from(vec![literal]);
        let mut set = ClauseSet::new_demod_indexed();

        set.insert(clause);
        set.record_demod_index_search_init(&a, PDTREE_IGNORE_NF_DATE, false);

        assert!(set.demod_index_search_may_have_match());
        assert_eq!(set.demod_index_coverage.get(), Some(false));
        set.record_demod_index_search_exit();
    }

    #[test]
    fn demod_index_coverage_cache_invalidates_on_mutable_clause_access() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "coverage_cache_a");
        let b = typed_const(&mut bank, "coverage_cache_b");
        let mut literal = literal(&mut bank, &a, &b, true);
        literal.set_prop(EP_IS_ORIENTED);
        let clause = clause_from(vec![literal]);
        let mut set = ClauseSet::new_demod_indexed();

        set.indexed_insert_clause_owned(clause, &bank);
        assert_eq!(set.demod_index_coverage.get(), None);

        set.record_demod_index_search_init(&a, PDTREE_IGNORE_NF_DATE, false);
        assert!(set.demod_index_search_may_have_match());
        assert_eq!(set.demod_index_coverage.get(), Some(true));
        assert!(set.demod_index_search_uses_compact_candidates());
        assert_eq!(set.demod_index_coverage.get(), Some(true));
        set.record_demod_index_search_exit();

        let _ = set.iter_mut().next();
        assert_eq!(set.demod_index_coverage.get(), None);
        set.record_demod_index_search_init(&a, PDTREE_IGNORE_NF_DATE, false);
        assert!(set.demod_index_search_uses_compact_candidates());
        assert_eq!(set.demod_index_coverage.get(), Some(true));
        set.record_demod_index_search_exit();

        set.set_prop(CP_IS_SOS);
        assert_eq!(set.demod_index_coverage.get(), None);
    }

    #[test]
    fn indexed_insert_clause_set_reweighs_source_and_preserves_order() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "idx_a");
        let b = typed_const(&mut bank, "idx_b");
        let c = typed_const(&mut bank, "idx_c");
        let mut first = clause_from(vec![literal(&mut bank, &a, &b, true)]);
        let mut second = clause_from(vec![literal(&mut bank, &b, &c, true)]);
        let first_id = first.ident();
        let second_id = second.ident();
        first.set_weight(0);
        second.set_weight(0);
        let mut source = ClauseSet::from_clauses([first, second]);
        let mut target = ClauseSet::new();
        let max_symbols = usize::try_from(bank.signature().f_count() + 1).unwrap();
        let mut anchor = ac_anchor(max_symbols);

        assert_eq!(
            target.indexed_insert_clause_set(&mut source, Some(&mut anchor), &bank),
            2
        );
        assert!(source.is_empty());
        assert_eq!(
            target.iter().map(Clause::ident).collect::<Vec<_>>(),
            vec![first_id, second_id]
        );
        assert!(target
            .iter()
            .all(|clause| clause.weight() == clause.standard_weight()));
        assert!(target
            .iter()
            .all(|clause| clause.query_prop(CP_IS_S_INDEXED)));
        assert_eq!(anchor.count_nodes(true, false), 2);
    }

    #[test]
    fn owned_indexed_insert_without_anchor_behaves_like_plain_insert() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "owned_plain_a");
        let b = typed_const(&mut bank, "owned_plain_b");
        let clause = clause_from(vec![literal(&mut bank, &a, &b, true)]);
        let ident = clause.ident();
        let mut set = ClauseSet::new();

        set.indexed_insert_clause_owned(clause, &bank);

        assert_eq!(set.members(), 1);
        let stored = set.find_by_id(ident).unwrap();
        assert!(!stored.query_prop(CP_IS_S_INDEXED));
        assert!(set.fv_anchor().is_none());
    }

    #[test]
    fn owned_indexed_insert_marks_indexes_and_extracts_from_anchor() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "owned_idx_a");
        let b = typed_const(&mut bank, "owned_idx_b");
        let clause = clause_from(vec![literal(&mut bank, &a, &b, true)]);
        let ident = clause.ident();
        let max_symbols = usize::try_from(bank.signature().f_count() + 1).unwrap();
        let mut set = ClauseSet::new();
        set.set_fv_anchor(Some(ac_anchor(max_symbols)));

        set.indexed_insert_clause_owned(clause, &bank);

        let stored = set.find_by_id(ident).unwrap();
        assert!(stored.query_prop(CP_IS_S_INDEXED));
        assert_eq!(set.fv_anchor().unwrap().index().clause_count(), 1);
        assert_eq!(set.fv_anchor().unwrap().count_nodes(true, false), 1);

        let extracted = set.extract_first().unwrap();
        assert_eq!(extracted.ident(), ident);
        assert!(!extracted.query_prop(CP_IS_S_INDEXED));
        assert_eq!(set.fv_anchor().unwrap().index().clause_count(), 0);
        assert_eq!(set.fv_anchor().unwrap().count_nodes(true, true), 1);
        assert_eq!(set.fv_anchor().unwrap().count_nodes(true, false), 1);
    }

    #[test]
    fn owned_indexed_insert_clause_set_reweighs_source_and_preserves_order() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "owned_set_a");
        let b = typed_const(&mut bank, "owned_set_b");
        let c = typed_const(&mut bank, "owned_set_c");
        let mut first = clause_from(vec![literal(&mut bank, &a, &b, true)]);
        let mut second = clause_from(vec![literal(&mut bank, &b, &c, true)]);
        let first_id = first.ident();
        let second_id = second.ident();
        first.set_weight(0);
        second.set_weight(0);
        let mut source = ClauseSet::from_clauses([first, second]);
        let max_symbols = usize::try_from(bank.signature().f_count() + 1).unwrap();
        let mut target = ClauseSet::new();
        target.set_fv_anchor(Some(ac_anchor(max_symbols)));

        assert_eq!(
            target.indexed_insert_clause_set_owned(&mut source, &bank),
            2
        );

        assert!(source.is_empty());
        assert_eq!(
            target.iter().map(Clause::ident).collect::<Vec<_>>(),
            vec![first_id, second_id]
        );
        assert!(target
            .iter()
            .all(|clause| clause.weight() == clause.standard_weight()));
        assert!(target
            .iter()
            .all(|clause| clause.query_prop(CP_IS_S_INDEXED)));
        assert_eq!(target.fv_anchor().unwrap().count_nodes(true, false), 2);
    }

    #[test]
    fn fv_indexify_reinserts_from_stack_and_marks_s_indexed_clauses() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "fv_a");
        let b = typed_const(&mut bank, "fv_b");
        let c = typed_const(&mut bank, "fv_c");
        let first = clause_from(vec![literal(&mut bank, &a, &b, true)]);
        let second = clause_from(vec![literal(&mut bank, &b, &c, true)]);
        let first_id = first.ident();
        let second_id = second.ident();
        let mut set = ClauseSet::from_clauses([first, second]);
        let max_symbols = usize::try_from(bank.signature().f_count() + 1).unwrap();
        let mut anchor = ac_anchor(max_symbols);

        assert_eq!(set.fv_indexify(&mut anchor, &bank), 2);
        assert_eq!(
            set.iter().map(Clause::ident).collect::<Vec<_>>(),
            vec![second_id, first_id]
        );
        assert!(set.iter().all(|clause| clause.query_prop(CP_IS_S_INDEXED)));
        assert!(anchor.count_nodes(false, false) > 0);
        assert_eq!(anchor.count_nodes(true, false), 2);
    }

    #[test]
    fn fv_indexify_owned_reinserts_from_stack_and_marks_s_indexed_clauses() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "owned_fv_a");
        let b = typed_const(&mut bank, "owned_fv_b");
        let c = typed_const(&mut bank, "owned_fv_c");
        let first = clause_from(vec![literal(&mut bank, &a, &b, true)]);
        let second = clause_from(vec![literal(&mut bank, &b, &c, true)]);
        let first_id = first.ident();
        let second_id = second.ident();
        let mut set = ClauseSet::from_clauses([first, second]);
        let max_symbols = usize::try_from(bank.signature().f_count() + 1).unwrap();
        set.set_fv_anchor(Some(ac_anchor(max_symbols)));

        assert_eq!(set.fv_indexify_owned(&bank), 2);

        assert_eq!(
            set.iter().map(Clause::ident).collect::<Vec<_>>(),
            vec![second_id, first_id]
        );
        assert!(set.iter().all(|clause| clause.query_prop(CP_IS_S_INDEXED)));
        assert!(set.fv_anchor().unwrap().count_nodes(false, false) > 0);
        assert_eq!(set.fv_anchor().unwrap().count_nodes(true, false), 2);
    }

    #[test]
    fn find_same_and_demod_verification_preserve_c_pointer_and_side_rules() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "demod_a");
        let b = typed_const(&mut bank, "demod_b");
        let c = typed_const(&mut bank, "demod_c");
        let mut oriented_lit = literal(&mut bank, &a, &b, true);
        oriented_lit.set_prop(EP_IS_ORIENTED);
        let oriented = clause_from(vec![oriented_lit]);
        let oriented_id = oriented.ident();
        let oriented_copy = oriented.clone();
        let unoriented = clause_from(vec![literal(&mut bank, &b, &c, true)]);
        let unoriented_id = unoriented.ident();
        let non_demod = clause_from(vec![
            literal(&mut bank, &a, &b, true),
            literal(&mut bank, &b, &c, false),
        ]);
        let non_demod_id = non_demod.ident();
        let set = ClauseSet::from_clauses([oriented, unoriented, non_demod]);

        let stored_oriented = set.find_by_id(oriented_id).unwrap();
        assert!(set.find_same(stored_oriented).is_some());
        assert!(set.find_same(&oriented_copy).is_none());
        assert!(set.verify_demod_clause_side(stored_oriented, EqnSide::LeftSide));
        assert!(!set.verify_demod_clause_side(stored_oriented, EqnSide::RightSide));

        let stored_unoriented = set.find_by_id(unoriented_id).unwrap();
        assert!(set.verify_demod_clause_side(stored_unoriented, EqnSide::RightSide));

        let stored_non_demod = set.find_by_id(non_demod_id).unwrap();
        assert!(!set.verify_demod_clause_side(stored_non_demod, EqnSide::LeftSide));
    }

    #[test]
    fn lop_print_helpers_render_each_clause_in_set_order() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "set_print_a");
        let b = typed_const(&mut bank, "set_print_b");
        let c = typed_const(&mut bank, "set_print_c");
        let set = ClauseSet::from_clauses([
            clause_from(vec![literal(&mut bank, &a, &b, true)]),
            clause_from(vec![
                literal(&mut bank, &b, &c, true),
                literal(&mut bank, &c, &a, false),
            ]),
        ]);

        assert_eq!(
            set.print_lop_string(&bank, true),
            "set_print_a=set_print_b <- .\nset_print_b=set_print_c <- set_print_c=set_print_a.\n"
        );
        assert_eq!(
            set.print_lop_prefix_string(&bank, "# "),
            "# set_print_a=set_print_b <- .\n# set_print_b=set_print_c <- set_print_c=set_print_a.\n"
        );
    }

    #[test]
    fn format_print_helpers_dispatch_clause_output_in_set_order() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "set_format_a");
        let b = typed_const(&mut bank, "set_format_b");
        let c = typed_const(&mut bank, "set_format_c");
        let mut first = clause_from(vec![literal(&mut bank, &a, &b, true)]);
        first.set_ident(201);
        let mut second = clause_from(vec![
            literal(&mut bank, &b, &c, true),
            literal(&mut bank, &c, &a, false),
        ]);
        second.set_ident(202);
        let set = ClauseSet::from_clauses([first, second]);

        let input_clause_set = set
            .print_format_string(&bank, true, IoFormat::Tptp, ProblemType::FirstOrder)
            .unwrap_or_else(|err| panic!("{err}"));
        assert_eq!(input_clause_set.matches("input_clause(").count(), 2);
        assert!(input_clause_set.contains("c_0_201"));
        assert!(input_clause_set.contains("c_0_202"));
        assert!(input_clause_set.contains("++equal(set_format_a, set_format_b)"));
        assert!(input_clause_set.ends_with("]).\n"));
        assert!(!input_clause_set.contains("<-"));

        let wrapped_clause_set = set
            .print_prefix_format_string(
                &bank,
                "# ",
                IoFormat::Tstp,
                ProblemType::FirstOrder,
                EqnPrintOptions::lop(),
            )
            .unwrap_or_else(|err| panic!("{err}"));
        assert_eq!(
            wrapped_clause_set.matches("# cnf(").count()
                + wrapped_clause_set.matches("# tcf(").count(),
            2
        );
        assert!(wrapped_clause_set.contains("set_format_a"));
        assert!(!wrapped_clause_set.contains("<-"));

        assert_eq!(
            set.print_format_string(&bank, true, IoFormat::Auto, ProblemType::FirstOrder)
                .unwrap_or_else(|err| panic!("{err}")),
            set.print_lop_string(&bank, true)
        );
    }

    #[test]
    fn tstp_print_helper_renders_complete_clauses_in_set_order() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "set_tstp_a");
        let b = typed_const(&mut bank, "set_tstp_b");
        let mut first = clause_from(vec![literal(&mut bank, &a, &b, true)]);
        first.set_ident(101);
        first.set_tptp_type(CP_TYPE_AXIOM);
        let mut second = clause_from(vec![literal(&mut bank, &b, &a, false)]);
        second.set_ident(102);
        second.set_tptp_type(CP_TYPE_NEG_CONJECTURE);
        let set = ClauseSet::from_clauses([first, second]);

        assert_eq!(
            set.tstp_print_string(&bank, true, ProblemType::FirstOrder)
                .unwrap(),
            "cnf(c_0_101, plain, (set_tstp_a=set_tstp_b)).\n\
             cnf(c_0_102, negated_conjecture, (set_tstp_b!=set_tstp_a)).\n"
        );
    }

    #[test]
    fn equality_axiom_lop_printing_matches_c_substitutivity_shapes() {
        let mut bank = test_bank();
        declare_typed_function(&mut bank, "eq_lop_f", 2);
        declare_typed_predicate(&mut bank, "eq_lop_p", 1);

        assert_eq!(
            eq_axioms_print_string(bank.signature(), IoFormat::Lop, false).unwrap(),
            "equal(X,X) <- .\n\
             equal(X,Y) <- equal(Y,X).\n\
             equal(X,Z) <- equal(X,Y), equal(Y,Z).\n\
             equal(eq_lop_f(X1,X2),eq_lop_f(Y1,Y2)) <- equal(X1,Y1),equal(X2,Y2).\n\
             eq_lop_p(X1) <- eq_lop_p(Y1),equal(X1,Y1).\n"
        );
    }

    #[test]
    fn equality_axiom_tptp_single_substitution_and_tstp_error_match_c() {
        let mut bank = test_bank();
        declare_typed_function(&mut bank, "eq_tptp_f", 2);
        declare_typed_predicate(&mut bank, "eq_tptp_p", 1);

        assert_eq!(
            eq_axioms_print_string(bank.signature(), IoFormat::Tptp, true).unwrap(),
            "input_clause(eq_reflexive, axiom, [++equal(X,X)]).\n\
             input_clause(eq_symmetric, axiom, [++equal(X,Y),--equal(Y,X)]).\n\
             input_clause(eq_transitive, axiom, [++equal(X,Z),--equal(X,Y),--equal(Y,Z)]).\n\
             input_clause(eq_subst_eq_tptp_f1, axiom, [++equal(eq_tptp_f(Y,X2),eq_tptp_f(Z,X2)),--equal(Y,Z)]).\n\
             input_clause(eq_subst_eq_tptp_f2, axiom, [++equal(eq_tptp_f(X1,Y),eq_tptp_f(X1,Z)),--equal(Y,Z)]).\n\
             input_clause(eq_subst_eq_tptp_p1, axiom, [++eq_tptp_p(Y),--eq_tptp_p(Z),--equal(Y,Z)]).\n"
        );

        assert!(
            eq_axioms_print_string(bank.signature(), IoFormat::Tstp, false)
                .unwrap_err()
                .message()
                .contains("TSTP/TPTP-3")
        );
    }

    #[test]
    fn parse_list_reads_clauses_until_non_clause_start() {
        let mut bank = test_bank();
        let mut scanner = Scanner::from_user_string("p(a). q(a) <- r(a). )", false).unwrap();
        scanner.set_format(IoFormat::Lop);
        let mut set = ClauseSet::new();

        assert_eq!(
            set.parse_list(&mut scanner, &mut bank, ProblemType::FirstOrder)
                .unwrap(),
            2
        );

        assert_eq!(set.members(), 2);
        assert_eq!(set.literals(), 3);
        assert_eq!(scanner.current_token().literal(), ")");
        assert_eq!(
            set.print_lop_string(&bank, true),
            "p(a) <- .\nq(a) <- r(a).\n"
        );

        let mut empty = Scanner::from_user_string(")", false).unwrap();
        empty.set_format(IoFormat::Lop);
        assert_eq!(
            set.parse_list(&mut empty, &mut bank, ProblemType::FirstOrder)
                .unwrap(),
            0
        );
        assert_eq!(empty.current_token().literal(), ")");
        assert_eq!(set.members(), 2);
    }

    #[test]
    fn remove_evaluations_clears_all_clauses_without_changing_set_accounting() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let c = typed_const(&mut bank, "c");
        let mut first = clause_from(vec![literal(&mut bank, &a, &b, true)]);
        let mut second = clause_from(vec![
            literal(&mut bank, &b, &c, true),
            literal(&mut bank, &c, &a, false),
        ]);
        first.add_eval_cell(evals_alloc(1));
        second.add_eval_cell(evals_alloc(2));
        let expected_ids = vec![first.ident(), second.ident()];
        let mut set = ClauseSet::from_clauses([first, second]);

        set.remove_evaluations();

        assert_eq!(set.members(), 2);
        assert_eq!(set.literals(), 3);
        assert_eq!(
            set.iter().map(Clause::ident).collect::<Vec<_>>(),
            expected_ids
        );
        assert!(set.iter().all(|clause| clause.evaluations().is_none()));
    }

    #[test]
    fn eval_indices_track_best_clause_extraction_and_root_clearing() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let c = typed_const(&mut bank, "c");
        let first = clause_with_evaluations(
            clause_from(vec![literal(&mut bank, &a, &b, true)]),
            &[(40, 30.0), (40, 1.0)],
        );
        let second = clause_with_evaluations(
            clause_from(vec![literal(&mut bank, &b, &c, true)]),
            &[(40, 10.0), (40, 3.0)],
        );
        let first_id = first.ident();
        let second_id = second.ident();
        let mut set = ClauseSet::from_clauses([first, second]);

        assert_eq!(set.eval_no(), 2);
        assert_eq!(set.find_best(0).map(Clause::ident), Some(second_id));
        assert_eq!(set.find_best(1).map(Clause::ident), Some(first_id));
        assert!(set
            .iter()
            .all(|clause| clause.evaluations().and_then(EvalCell::object).is_some()));

        let extracted = set.extract_best(0).unwrap();

        assert_eq!(extracted.ident(), second_id);
        assert_eq!(set.find_best(0).map(Clause::ident), Some(first_id));
        assert_eq!(set.members(), 1);
        assert_eq!(set.literals(), 1);

        set.remove_evaluations();

        assert_eq!(set.eval_no(), 2);
        assert_eq!(set.find_best(0).map(Clause::ident), None);
        assert!(set.iter().all(|clause| clause.evaluations().is_none()));
    }

    #[test]
    fn eval_indices_keep_cloned_evaluation_cells_distinct() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let c = typed_const(&mut bank, "c");
        let first = clause_with_evaluations(
            clause_from(vec![literal(&mut bank, &a, &b, true)]),
            &[(40, 1.0)],
        );
        let mut second = clause_from(vec![literal(&mut bank, &b, &c, true)]);
        second.add_eval_cell(first.evaluations().unwrap().clone());
        let first_id = first.ident();
        let second_id = second.ident();
        let mut set = ClauseSet::from_clauses([first, second]);

        assert_eq!(set.eval_order_objects(0).len(), 2);
        assert_eq!(set.find_best(0).map(Clause::ident), Some(first_id));
        assert_eq!(
            set.extract_best(0).map(|clause| clause.ident()),
            Some(first_id)
        );
        assert_eq!(set.find_best(0).map(Clause::ident), Some(second_id));
        assert_eq!(set.members(), 1);
    }

    #[test]
    fn eval_object_slots_survive_holes_insertion_and_sorting() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "eval_slot_a");
        let b = typed_const(&mut bank, "eval_slot_b");
        let c = typed_const(&mut bank, "eval_slot_c");
        let d = typed_const(&mut bank, "eval_slot_d");
        let first = clause_with_evaluations(
            clause_from(vec![literal(&mut bank, &a, &b, true)]),
            &[(40, 30.0)],
        );
        let second = clause_with_evaluations(
            clause_from(vec![literal(&mut bank, &b, &c, true)]),
            &[(40, 10.0)],
        );
        let third = clause_with_evaluations(
            clause_from(vec![literal(&mut bank, &c, &d, true)]),
            &[(40, 20.0)],
        );
        let second_id = second.ident();
        let third_id = third.ident();
        let mut set = ClauseSet::from_clauses([first, second, third]);

        assert_eq!(
            set.extract_best(0).map(|clause| clause.ident()),
            Some(second_id)
        );

        let mut fourth = clause_with_evaluations(
            clause_from(vec![literal(&mut bank, &d, &a, true)]),
            &[(40, 5.0)],
        );
        fourth.set_prop(CP_INITIAL);
        let fourth_id = fourth.ident();
        set.insert(fourth);
        let fourth_object = set
            .find_by_id(fourth_id)
            .and_then(Clause::evaluations)
            .and_then(EvalCell::object)
            .expect("inserted evaluated clause must have an object handle");

        assert_eq!(set.find_best(0).map(Clause::ident), Some(fourth_id));
        set.sort_by(|left, right| right.ident().cmp(&left.ident()));
        assert_eq!(set.find_best(0).map(Clause::ident), Some(fourth_id));
        assert!(set.del_prop_by_eval_object(fourth_object, CP_INITIAL));
        assert!(!set
            .find_by_id(fourth_id)
            .expect("sorted clause remains in the set")
            .query_prop(CP_INITIAL));

        assert_eq!(
            set.extract_best(0).map(|clause| clause.ident()),
            Some(fourth_id)
        );
        assert_eq!(set.find_best(0).map(Clause::ident), Some(third_id));
    }

    #[test]
    fn eval_object_slots_rebuild_after_sparse_store_compaction() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "eval_compact_a");
        let b = typed_const(&mut bank, "eval_compact_b");
        let base_literal = literal(&mut bank, &a, &b, true);
        let clause_count = 2 * super::SPARSE_STORE_COMPACT_MIN_HOLES + 1;
        let mut ids = Vec::with_capacity(clause_count);
        let mut set = ClauseSet::new();

        for index in 0..clause_count {
            let heuristic = f32::from(i16::try_from(index).unwrap());
            let clause = clause_with_evaluations(
                clause_from(vec![base_literal.clone()]),
                &[(40, heuristic)],
            );
            ids.push(clause.ident());
            set.insert(clause);
        }

        let removed = super::SPARSE_STORE_COMPACT_MIN_HOLES + 1;
        for &expected in &ids[..removed] {
            assert_eq!(
                set.extract_best(0).map(|clause| clause.ident()),
                Some(expected)
            );
        }

        assert_eq!(set.find_best(0).map(Clause::ident), Some(ids[removed]));

        let preferred = clause_with_evaluations(clause_from(vec![base_literal]), &[(40, -1.0)]);
        let preferred_id = preferred.ident();
        set.insert(preferred);
        assert_eq!(set.clauses.slots.len(), set.len());
        assert_eq!(set.clauses.first_occupied, 0);
        assert_eq!(
            set.extract_best(0).map(|clause| clause.ident()),
            Some(preferred_id)
        );
        assert_eq!(set.find_best(0).map(Clause::ident), Some(ids[removed]));
    }

    #[test]
    fn delete_marked_non_units_and_copies_follow_plain_set_order() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let c = typed_const(&mut bank, "c");
        let unit = clause_from(vec![literal(&mut bank, &a, &b, true)]);
        let mut copy = unit.clone();
        copy.set_ident(unit.ident() + 1000);
        let non_unit = clause_from(vec![
            literal(&mut bank, &b, &c, true),
            literal(&mut bank, &c, &a, false),
        ]);
        let unit_id = unit.ident();
        let copy_id = copy.ident();
        let non_unit_id = non_unit.ident();
        let mut set = ClauseSet::from_clauses([unit, copy, non_unit]);

        assert_eq!(set.mark_copies(), 1);
        assert!(set
            .find_by_id(copy_id)
            .unwrap()
            .query_prop(CP_DELETE_CLAUSE));
        assert_eq!(set.delete_marked_entries(), 1);
        assert_eq!(
            set.iter().map(Clause::ident).collect::<Vec<_>>(),
            vec![unit_id, non_unit_id]
        );
        assert_eq!(set.literals(), 3);

        assert_eq!(set.delete_non_units(), 1);
        assert_eq!(
            set.iter().map(Clause::ident).collect::<Vec<_>>(),
            vec![unit_id]
        );
        assert_eq!(set.literals(), 1);

        let duplicate = set.find_by_id(unit_id).unwrap().clone();
        set.insert(duplicate);
        assert_eq!(set.delete_copies(), 1);
        assert_eq!(set.members(), 1);
    }

    #[test]
    fn filter_trivial_removes_true_and_conflicting_clauses() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let kept = clause_from(vec![literal(&mut bank, &a, &b, true)]);
        let kept_id = kept.ident();
        let true_clause = clause_from(vec![Eqn::create_true_lit(&mut bank).unwrap()]);
        let conflicting = clause_from(vec![
            literal(&mut bank, &a, &b, true),
            literal(&mut bank, &a, &b, false),
        ]);
        let mut set = ClauseSet::from_clauses([true_clause, kept, conflicting]);

        assert_eq!(set.literals(), 4);
        assert_eq!(set.filter_trivial(&bank), 2);

        assert_eq!(set.members(), 1);
        assert_eq!(set.literals(), 1);
        assert_eq!(
            set.iter().map(Clause::ident).collect::<Vec<_>>(),
            vec![kept_id]
        );
    }

    #[test]
    fn filter_tautologies_removes_ground_completion_tautologies() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let c = typed_const(&mut bank, "c");
        let kept = clause_from(vec![
            literal(&mut bank, &a, &c, true),
            literal(&mut bank, &a, &b, false),
        ]);
        let kept_id = kept.ident();
        let tautology = clause_from(vec![
            literal(&mut bank, &a, &c, true),
            literal(&mut bank, &a, &b, false),
            literal(&mut bank, &b, &c, false),
        ]);
        let mut set = ClauseSet::from_clauses([kept, tautology]);
        let mut work_bank = TermBank::new(bank.signature().clone()).unwrap();

        assert_eq!(set.filter_tautologies(&mut work_bank).unwrap(), 1);

        assert_eq!(set.members(), 1);
        assert_eq!(set.literals(), 2);
        assert_eq!(
            set.iter().map(Clause::ident).collect::<Vec<_>>(),
            vec![kept_id]
        );
    }

    #[test]
    fn set_properties_sos_and_conjecture_counts_match_c_rules() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let mut axiom = clause_from(vec![literal(&mut bank, &a, &b, true)]);
        let mut goal = clause_from(vec![literal(&mut bank, &a, &b, false)]);
        let mut conjecture = clause_from(vec![literal(&mut bank, &b, &a, true)]);
        let mut neg_conjecture = clause_from(vec![literal(&mut bank, &b, &a, false)]);
        axiom.set_tptp_type(CP_TYPE_AXIOM);
        goal.set_tptp_type(CP_TYPE_HYPOTHESIS);
        conjecture.set_tptp_type(CP_TYPE_CONJECTURE);
        neg_conjecture.set_tptp_type(CP_TYPE_NEG_CONJECTURE);
        let mut set = ClauseSet::from_clauses([axiom, goal, conjecture, neg_conjecture]);

        set.set_prop(CP_INITIAL);
        assert!(set.iter().all(|clause| clause.query_prop(CP_INITIAL)));
        set.del_prop(CP_INITIAL);
        assert!(set.iter().all(|clause| !clause.query_prop(CP_INITIAL)));
        set.set_tptp_type(CP_TYPE_AXIOM);
        assert!(set
            .iter()
            .all(|clause| clause.query_tptp_type() == CP_TYPE_AXIOM));

        let ids = set.iter().map(Clause::ident).collect::<Vec<_>>();
        set.find_by_id_mut(ids[1])
            .unwrap()
            .set_tptp_type(CP_TYPE_HYPOTHESIS);
        set.find_by_id_mut(ids[2])
            .unwrap()
            .set_tptp_type(CP_TYPE_CONJECTURE);
        set.find_by_id_mut(ids[3])
            .unwrap()
            .set_tptp_type(CP_TYPE_NEG_CONJECTURE);

        assert_eq!(set.mark_sos(false), 2);
        assert!(set.find_by_id(ids[1]).unwrap().query_prop(CP_IS_SOS));
        assert!(set.find_by_id(ids[3]).unwrap().query_prop(CP_IS_SOS));
        assert_eq!(set.mark_sos(true), 1);
        assert!(set.find_by_id(ids[2]).unwrap().query_prop(CP_IS_SOS));
        assert!(!set.find_by_id(ids[3]).unwrap().query_prop(CP_IS_SOS));

        let mut hypotheses = 10;
        assert_eq!(set.count_conjectures(&mut hypotheses), 2);
        assert_eq!(hypotheses, 11);

        let mut conjectures = Vec::new();
        let mut rest = Vec::new();
        assert_eq!(set.split_conjecture_refs(&mut conjectures, &mut rest), 2);
        assert_eq!(
            conjectures
                .iter()
                .map(|clause| clause.ident())
                .collect::<Vec<_>>(),
            vec![ids[2], ids[3]]
        );
        assert_eq!(
            rest.iter().map(|clause| clause.ident()).collect::<Vec<_>>(),
            vec![ids[0], ids[1]]
        );
    }

    #[test]
    fn mark_maximal_terms_updates_each_clause_in_set_order() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let f_a = typed_unary(&mut bank, "f", &a);
        let first = clause_from(vec![
            literal(&mut bank, &a, &f_a, true),
            literal(&mut bank, &a, &a, true),
        ]);
        let second = clause_from(vec![literal(&mut bank, &a, &f_a, true)]);
        let mut set = ClauseSet::from_clauses([first, second]);
        let mut ocb = kbo_ocb(&bank);

        set.mark_maximal_terms(&mut ocb, &bank);

        assert!(set.iter().all(|clause| clause.query_prop(CP_IS_ORIENTED)));
        let clauses = set.iter().collect::<Vec<_>>();
        assert_eq!(clauses[0].literals().query_prop_number(EP_IS_MAXIMAL), 1);
        assert_eq!(clauses[1].literals().query_prop_number(EP_IS_MAXIMAL), 1);
        assert_eq!(clauses[0].literals().as_slice()[0].left(), &f_a);
    }

    #[test]
    fn mark_maximal_terms_with_bank_accepts_lambda_order_beta_surface_set() {
        let _guard = global_state_lock();
        let _problem_type = set_problem_type_for_test(ProblemType::HigherOrder);
        let mut bank = test_bank();
        let binder_type = bank.signature().type_bank().default_type();
        let db0 = bank.request_db_var(&binder_type, 0);
        let lambda =
            close_with_db_var(&mut bank, &binder_type, &db0).unwrap_or_else(|err| panic!("{err}"));
        let arg = typed_const(&mut bank, "clauseset_lambda_order_arg");
        let applied = apply_terms(&mut bank, &lambda, std::slice::from_ref(&arg))
            .unwrap_or_else(|err| panic!("{err}"));
        let clause = clause_from(vec![literal(&mut bank, &applied, &arg, true)]);
        let mut set = ClauseSet::from_clauses([clause]);
        let mut ocb = kbo6_lambda_ocb(&bank);

        set.mark_maximal_terms_with_bank(&mut ocb, &mut bank)
            .unwrap_or_else(|err| panic!("{err}"));

        let clause = set.iter().next().expect("test set has one clause");
        assert!(clause.query_prop(CP_IS_ORIENTED));
        assert_eq!(clause.literals().query_prop_number(EP_IS_MAXIMAL), 1);
        assert!(!clause.literals().as_slice()[0].is_oriented());
    }

    #[test]
    fn aggregate_weights_variables_terms_and_stack_counts_use_clause_helpers() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let x = typed_var(&bank, -2);
        let fx = typed_unary(&mut bank, "f", &x);
        let unit = clause_from(vec![literal(&mut bank, &a, &b, true)]);
        let var_clause = clause_from(vec![literal(&mut bank, &fx, &a, false)]);
        let mut set = ClauseSet::from_clauses([unit, var_clause]);

        assert_eq!(
            set.standard_weight(),
            set.iter().map(Clause::standard_weight).sum()
        );
        set.iter_mut().for_each(|clause| clause.set_weight(-1));
        set.default_weigh_clauses();
        assert!(set
            .iter()
            .all(|clause| clause.weight() == clause.standard_weight()));
        assert_eq!(set.max_var_number(), 1);
        assert_eq!(set.find_max_standard_weight().map(Clause::weight), Some(5));
        assert!(set.term_nodes(&bank) > 0);

        assert_eq!(set.tb_term_prop_del_count(TP_CHECK_FLAG), 0);
        set.term_set_prop(TP_CHECK_FLAG);
        assert!(set.tb_term_prop_del_count(TP_CHECK_FLAG) > 0);
        assert!(set.shared_term_nodes() > 0);
        assert!(set.is_untyped());

        let mut owned_stack = PStack::new();
        owned_stack.push(set.clone());
        assert_eq!(clause_set_stack_cardinality(&owned_stack), 2);

        let mut ref_stack = PStack::new();
        ref_stack.push(&set);
        assert_eq!(clause_set_ref_stack_cardinality(&ref_stack), 2);

        let mut clause_stack = PStack::new();
        assert_eq!(set.push_clause_refs(&mut clause_stack), 2);
        assert_eq!(clause_stack.len(), 2);
        let mut c_named_clause_stack = PStack::new();
        assert_eq!(set.push_clauses(&mut c_named_clause_stack), 2);
        assert_eq!(
            c_named_clause_stack
                .as_slice()
                .iter()
                .map(|clause| clause.ident())
                .collect::<Vec<_>>(),
            set.iter().map(Clause::ident).collect::<Vec<_>>()
        );
        assert_eq!(set.apply_fun(|_| 1), 1);

        let default_type = bank.signature().type_bank().default_type();
        let arrow_type = bank
            .signature_mut()
            .type_bank_mut()
            .insert_type_shared(alloc_arrow_type(vec![default_type.clone(), default_type]));
        let higher_order = bank.vars().var_assert_alloc(-6, &arrow_type);
        set.insert(clause_from(vec![literal(
            &mut bank,
            &higher_order,
            &higher_order,
            true,
        )]));
        assert!(set.conjecture_order(bank.signature()) > 0);
    }

    #[test]
    fn clause_set_list_get_max_date_scans_requested_prefix() {
        let mut early = ClauseSet::new();
        let mut latest = ClauseSet::new();
        let mut ignored = ClauseSet::new();
        early.set_date(SysDate::from_raw(3));
        latest.set_date(SysDate::from_raw(7));
        ignored.set_date(SysDate::from_raw(11));

        assert_eq!(
            clause_set_list_get_max_date(&[&early, &latest, &ignored], 2),
            SysDate::from_raw(7)
        );
        assert_eq!(
            clause_set_list_get_max_date(&[&early, &latest, &ignored], 0),
            SysDate::creation_time()
        );
    }

    #[test]
    fn type_distribution_forwards_clause_terms_to_signature_types() {
        let mut bank = test_bank();
        let default_type = bank.signature().type_bank().default_type();
        let unary_type = bank
            .signature_mut()
            .type_bank_mut()
            .insert_type_shared(alloc_arrow_type(vec![
                default_type.clone(),
                default_type.clone(),
            ]));
        let f_code = bank.signature_mut().insert_id("typed_f", 1, false);
        bank.signature_mut()
            .declare_final_type(f_code, unary_type.clone())
            .unwrap();
        let a_code = bank.signature_mut().insert_id("typed_a", 0, false);
        bank.signature_mut()
            .declare_final_type(a_code, default_type.clone())
            .unwrap();
        let a = bank.create_const_term(a_code).unwrap();
        let fa = Term::top_alloc(f_code, 1);
        fa.set_type(Some(default_type.clone()));
        fa.set_argument(0, a.clone());
        let fa = bank.insert(&fa, DerefType::Never).unwrap();
        let set = ClauseSet::from_clauses([clause_from(vec![literal(&mut bank, &fa, &a, true)])]);

        let mut type_dist =
            vec![0; usize::try_from(bank.signature().type_bank().types_count() + 1).unwrap()];
        set.add_type_distribution(bank.signature_mut(), &mut type_dist);

        assert_eq!(
            type_dist[usize::try_from(unary_type.type_uid()).unwrap()],
            1
        );
        assert_eq!(
            type_dist[usize::try_from(default_type.type_uid()).unwrap()],
            2
        );
    }

    #[test]
    fn characteristic_freq_vectors_and_permutation_follow_clause_order() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let x = typed_var(&bank, -2);
        let fx = typed_unary(&mut bank, "f", &x);
        let first = clause_from(vec![literal(&mut bank, &a, &b, true)]);
        let second = clause_from(vec![literal(&mut bank, &fx, &a, false)]);
        let set = ClauseSet::from_clauses([first, second]);
        let mut cspec = FvCollect::new(FvCollectLayout::new(FvIndexType::AcFeatures, false, 0, 0));
        cspec.set_max_symbols(usize::try_from(bank.signature().f_count()).unwrap() + 1);
        let vector_len = fv_size(cspec.max_symbols(), cspec.features());
        let mut fsum = FreqVector::new(vector_len);
        let mut fmax = FreqVector::new(vector_len);
        let mut fmin = FreqVector::new(vector_len);

        assert_eq!(
            set.find_char_freq_vectors(&mut fsum, &mut fmax, &mut fmin, &cspec),
            2
        );

        let vectors = set
            .iter()
            .map(|clause| var_freq_vector_compute(clause, &cspec))
            .collect::<Vec<_>>();
        let mut expected_sum = FreqVector::new(vector_len);
        let mut expected_max = FreqVector::new(vector_len);
        let mut expected_min = FreqVector::new(vector_len);
        expected_min.initialize(i64::MAX);
        for vector in &vectors {
            let old_sum = expected_sum.clone();
            expected_sum.add_from(&old_sum, vector);
            let old_max = expected_max.clone();
            expected_max.max_from(&old_max, vector);
            let old_min = expected_min.clone();
            expected_min.min_from(&old_min, vector);
        }
        assert_eq!(fsum, expected_sum);
        assert_eq!(fmax, expected_max);
        assert_eq!(fmin, expected_min);

        let expected_perm = perm_vector_compute_internal(
            &expected_max,
            &expected_min,
            &expected_sum,
            cspec.max_symbols(),
            false,
        );
        assert_eq!(
            set.perm_vector_compute(&cspec, false).unwrap(),
            expected_perm
        );

        let no_features =
            FvCollect::new(FvCollectLayout::new(FvIndexType::NoFeatures, false, 0, 0));
        assert!(set.perm_vector_compute(&no_features, false).is_none());
    }

    #[test]
    fn new_terms_copies_into_target_bank_and_reinserts_from_stack() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let c = typed_const(&mut bank, "c");
        let mut first = clause_from(vec![literal(&mut bank, &a, &b, true)]);
        let mut second = clause_from(vec![literal(&mut bank, &b, &c, true)]);
        first.set_weight(-10);
        second.set_weight(-20);
        let first_id = first.ident();
        let second_id = second.ident();
        let original_first_left = first.literals().as_slice()[0].left().clone();
        let mut set = ClauseSet::from_clauses([first, second]);
        let mut target = TermBank::new(bank.signature().clone()).unwrap();

        assert_eq!(set.new_terms(&mut target).unwrap(), 2);

        let copied = set.iter().collect::<Vec<_>>();
        assert_eq!(
            copied
                .iter()
                .map(|clause| clause.ident())
                .collect::<Vec<_>>(),
            vec![second_id, first_id]
        );
        assert_eq!(set.literals(), 2);
        assert!(copied
            .iter()
            .all(|clause| clause.weight() == clause.standard_weight()));
        let copied_first_left = copied[1].literals().as_slice()[0].left();
        assert_ne!(copied_first_left, &original_first_left);
        assert_eq!(copied_first_left.f_code(), original_first_left.f_code());
    }

    #[test]
    fn frequency_symbol_selection_preserves_last_tie_wins_behavior() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let first = typed_unary(&mut bank, "f", &a);
        let second = typed_unary(&mut bank, "g", &a);
        let f_code = first.f_code();
        let g_code = second.f_code();
        let set = ClauseSet::from_clauses([clause_from(vec![
            literal(&mut bank, &first, &a, true),
            literal(&mut bank, &second, &a, true),
        ])]);

        let mut dist = vec![0; usize::try_from(bank.signature().f_count() + 1).unwrap()];
        set.add_symbol_distribution(&mut dist);
        assert_eq!(dist[usize::try_from(f_code).unwrap()], 1);
        assert_eq!(dist[usize::try_from(g_code).unwrap()], 1);

        assert_eq!(set.find_freq_symbol(bank.signature(), 1, false), g_code);
        assert_eq!(set.find_freq_symbol(bank.signature(), 1, true), g_code);
    }

    #[test]
    fn equality_definition_lookup_returns_reduced_clause_position_from_start() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let x = typed_var(&bank, -2);
        let fx = typed_unary(&mut bank, "f", &x);
        let non_definition = clause_from(vec![literal(&mut bank, &a, &b, true)]);
        let definition = clause_from(vec![literal(&mut bank, &fx, &a, true)]);
        let late_definition = clause_from(vec![literal(&mut bank, &fx, &b, true)]);
        let definition_id = definition.ident();
        let late_definition_id = late_definition.ident();
        let set = ClauseSet::from_clauses([non_definition, definition, late_definition]);

        let found = set.find_eq_definition(&bank, 1).unwrap();
        assert_eq!(found.clause().map(Clause::ident), Some(definition_id));
        assert_eq!(found.literal_index(), Some(0));
        assert_eq!(found.side(), EqnSide::LeftSide);
        assert!(found.term_pos().is_top_pos());

        let found_from_late = set
            .find_eq_definition_from_id(&bank, 1, late_definition_id)
            .unwrap();
        assert_eq!(
            found_from_late.clause().map(Clause::ident),
            Some(late_definition_id)
        );
        assert!(set.find_eq_definition_from_id(&bank, 1, i64::MAX).is_none());
    }
}
