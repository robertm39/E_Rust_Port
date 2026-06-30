use crate::basics::defines::IntOrP;
use crate::basics::error::{Diagnostic, ErrorCode};
use crate::basics::pqueue::{PQueue, PQueueInt};
use crate::basics::pstacks::PStack;
use crate::basics::simple_stuff::ProblemType;
use crate::clauses::clause::{clause_write_tstp, Clause};
use crate::clauses::clause_props::FormulaProperties;
use crate::clauses::clausesets::{clause_set_ref_stack_cardinality, ClauseSet};
use crate::clauses::f_generality::{clause_compute_d_rel, GenDistrib, GeneralityMeasure};
use crate::clauses::formulasets::{FormulaSet, WrappedFormula};
use crate::terms::functypes::FunCode;
use crate::terms::signature::Signature;
use crate::terms::termbanks::TermBank;
use std::collections::BTreeSet;
use std::fmt;

const DEFAULT_COMCHAR_RAW: &str = "%";

#[derive(Debug)]
pub struct DRel<'a> {
    f_code: FunCode,
    activated: bool,
    d_clauses: PStack<&'a Clause>,
}

impl<'a> DRel<'a> {
    #[must_use]
    pub fn new(f_code: FunCode) -> Self {
        Self {
            f_code,
            activated: false,
            d_clauses: PStack::new(),
        }
    }

    #[must_use]
    pub const fn f_code(&self) -> FunCode {
        self.f_code
    }

    #[must_use]
    pub const fn is_activated(&self) -> bool {
        self.activated
    }

    pub const fn set_activated(&mut self, activated: bool) {
        self.activated = activated;
    }

    #[must_use]
    pub const fn d_clauses(&self) -> &PStack<&'a Clause> {
        &self.d_clauses
    }

    pub fn d_clauses_mut(&mut self) -> &mut PStack<&'a Clause> {
        &mut self.d_clauses
    }

    pub fn write_debug(
        &self,
        output: &mut impl fmt::Write,
        stderr: &mut impl fmt::Write,
        signature: &Signature,
    ) -> fmt::Result {
        let name = signature.find_name(self.f_code).unwrap_or("<unknown>");
        writeln!(
            output,
            "{DEFAULT_COMCHAR_RAW} {f_code:6} {name:<15}: {clause_count:6} clauses, {formula_count:6} formulas",
            f_code = self.f_code,
            clause_count = self.d_clauses.len(),
            formula_count = 0
        )?;
        output.write_str(DEFAULT_COMCHAR_RAW)?;
        output.write_str("formulas: ")?;
        stderr.write_char('\n')
    }

    #[must_use]
    pub fn debug_string(&self, signature: &Signature) -> (String, String) {
        let mut output = String::new();
        let mut stderr = String::new();
        let _ = self.write_debug(&mut output, &mut stderr, signature);
        (output, stderr)
    }
}

#[derive(Debug)]
pub struct DRelation<'a> {
    relation: Vec<Option<DRel<'a>>>,
}

impl Default for DRelation<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> DRelation<'a> {
    #[must_use]
    pub fn new() -> Self {
        let mut relation = Vec::with_capacity(10);
        relation.resize_with(10, || None);
        Self { relation }
    }

    #[must_use]
    pub fn allocated_size(&self) -> usize {
        self.relation.len()
    }

    /// Returns the `DRel` entry for `f_code`, creating it when absent.
    ///
    /// # Panics
    ///
    /// Panics when `f_code` is negative or does not fit in `usize`, matching
    /// the C assumption that function codes are valid array indexes here.
    pub fn get_f_entry(&mut self, f_code: FunCode) -> &mut DRel<'a> {
        let index = f_code_index(f_code);
        if index >= self.relation.len() {
            self.relation.resize_with(index + 1, || None);
        }
        if self.relation[index].is_none() {
            self.relation[index] = Some(DRel::new(f_code));
        }
        self.relation[index]
            .as_mut()
            .expect("DRel entry must exist after insertion")
    }

    #[must_use]
    pub fn entry(&self, f_code: FunCode) -> Option<&DRel<'a>> {
        self.relation
            .get(f_code_index(f_code))
            .and_then(Option::as_ref)
    }

    pub fn entry_mut(&mut self, f_code: FunCode) -> Option<&mut DRel<'a>> {
        self.relation
            .get_mut(f_code_index(f_code))
            .and_then(Option::as_mut)
    }

    pub fn add_clause(
        &mut self,
        generality: &mut GenDistrib,
        gentype: GeneralityMeasure,
        benevolence: f64,
        generosity: i64,
        clause: &'a Clause,
    ) {
        let mut symbols = PStack::new();
        clause_compute_d_rel(
            generality,
            gentype,
            benevolence,
            generosity,
            clause,
            &mut symbols,
        );
        if symbols.is_empty() {
            self.get_f_entry(0).d_clauses_mut().push(clause);
        } else {
            while let Some(symbol) = symbols.pop() {
                self.get_f_entry(symbol).d_clauses_mut().push(clause);
            }
        }
    }

    pub fn add_clause_set(
        &mut self,
        generality: &mut GenDistrib,
        gentype: GeneralityMeasure,
        benevolence: f64,
        generosity: i64,
        set: &'a ClauseSet,
    ) {
        for clause in set.iter() {
            self.add_clause(generality, gentype, benevolence, generosity, clause);
        }
    }

    pub fn add_clause_sets(
        &mut self,
        generality: &mut GenDistrib,
        gentype: GeneralityMeasure,
        benevolence: f64,
        generosity: i64,
        sets: &PStack<&'a ClauseSet>,
    ) {
        for set in sets.as_slice() {
            self.add_clause_set(generality, gentype, benevolence, generosity, set);
        }
    }

    #[must_use]
    pub fn total_entries(&self) -> i64 {
        usize_to_i64(
            self.relation
                .iter()
                .skip(1)
                .filter_map(Option::as_ref)
                .map(|entry| entry.d_clauses().len())
                .sum(),
        )
    }

    pub fn write_debug(
        &self,
        output: &mut impl fmt::Write,
        stderr: &mut impl fmt::Write,
        signature: &Signature,
    ) -> fmt::Result {
        for entry in self.relation.iter().skip(1).filter_map(Option::as_ref) {
            entry.write_debug(output, stderr, signature)?;
        }
        Ok(())
    }

    #[must_use]
    pub fn debug_string(&self, signature: &Signature) -> (String, String) {
        let mut output = String::new();
        let mut stderr = String::new();
        let _ = self.write_debug(&mut output, &mut stderr, signature);
        (output, stderr)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i64)]
pub enum AxiomType {
    NoType = 0,
    Clause = 1,
    Formula = 2,
}

impl AxiomType {
    #[must_use]
    pub const fn queue_tag(self) -> PQueueInt {
        match self {
            Self::NoType => 0,
            Self::Clause => 1,
            Self::Formula => 2,
        }
    }

    #[must_use]
    pub const fn from_queue_tag(tag: PQueueInt) -> Option<Self> {
        match tag {
            0 => Some(Self::NoType),
            1 => Some(Self::Clause),
            2 => Some(Self::Formula),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ClauseSineParams {
    pub gen_measure: GeneralityMeasure,
    pub use_hypotheses: bool,
    pub benevolence: f64,
    pub generosity: i64,
    pub max_recursion_depth: i64,
    pub max_set_size: i64,
    pub max_set_fraction: f64,
    pub add_no_symbol_axioms: bool,
}

impl ClauseSineParams {
    #[must_use]
    pub const fn g_sine(gen_measure: GeneralityMeasure) -> Self {
        Self {
            gen_measure,
            use_hypotheses: false,
            benevolence: 1.0,
            generosity: i64::MAX,
            max_recursion_depth: i32::MAX as i64,
            max_set_size: i64::MAX,
            max_set_fraction: 1.0,
            add_no_symbol_axioms: false,
        }
    }
}

pub fn pstack_clause_del_prop(stack: &mut PStack<&mut Clause>, prop: FormulaProperties) {
    for clause in stack.as_mut_slice() {
        (*clause).del_prop(prop);
    }
}

/// Staged equivalent of C `PStackClausesMove`.
///
/// C stacks store raw `Clause_p` values whose current owner set is embedded in
/// the clause cell. Rust uses clause identifiers for this staged helper, so
/// callers provide the expected old owner set explicitly. The destination is
/// also searched to preserve C's behavior for duplicate stack entries that
/// move an already-selected clause to the destination tail again.
///
/// # Panics
///
/// Panics when a stacked clause identifier is not present in either `from` or
/// `to`, matching C's assertion that every stacked clause pointer is set-owned.
#[must_use]
pub fn pstack_clauses_move(stack: &PStack<i64>, from: &mut ClauseSet, to: &mut ClauseSet) -> i64 {
    let mut moved = 0;
    for ident in stack.as_slice() {
        let clause = from
            .extract_by_id(*ident)
            .or_else(|| to.extract_by_id(*ident))
            .unwrap_or_else(|| panic!("stacked clause identifier is not set-owned: {ident}"));
        to.insert(clause);
        moved += 1;
    }
    moved
}

pub fn pstack_formula_del_prop(stack: &mut PStack<&mut WrappedFormula>, prop: FormulaProperties) {
    for formula in stack.as_mut_slice() {
        (*formula).del_prop(prop);
    }
}

/// Staged equivalent of C `PStackFormulasMove`.
///
/// C stores raw `WFormula_p` values whose current owner set is embedded in the
/// formula cell. Rust formula stacks currently store stable `WrappedFormula`
/// entry ids, so callers provide the expected old owner set explicitly. The
/// destination is also searched to preserve C's behavior for duplicate stack
/// entries that move an already-selected formula to the destination tail again.
///
/// # Panics
///
/// Panics when a stacked entry id is not present in either `from` or `to`,
/// matching C's assertion that every stacked formula pointer is set-owned.
#[must_use]
pub fn pstack_formulas_move(
    stack: &PStack<u64>,
    from: &mut FormulaSet,
    to: &mut FormulaSet,
) -> i64 {
    let mut moved = 0;
    for entry_id in stack.as_slice() {
        let formula = from
            .extract_entry(*entry_id)
            .or_else(|| to.extract_entry(*entry_id))
            .unwrap_or_else(|| panic!("stacked formula entry id is not set-owned: {entry_id}"));
        to.insert(formula);
        moved += 1;
    }
    moved
}

pub fn pqueue_store_clause<'a>(axioms: &mut PQueue<IntOrP<&'a Clause>>, clause: &'a Clause) {
    axioms.store_int(AxiomType::Clause.queue_tag());
    axioms.store_pointer(clause);
}

pub fn clause_set_find_ax_selection_seeds<'a>(
    set: &'a ClauseSet,
    res: &mut PQueue<IntOrP<&'a Clause>>,
    inc_hypos: bool,
) -> i64 {
    let mut found = 0;
    for clause in set.iter() {
        if clause.is_conjecture() || (inc_hypos && clause.is_hypothesis()) {
            pqueue_store_clause(res, clause);
            found += 1;
        }
    }
    found
}

pub fn select_threshold_clause_sets<'a>(
    clause_sets: &PStack<&'a ClauseSet>,
    formula_cardinality: i64,
    threshold: i64,
    res_clauses: &mut PStack<&'a Clause>,
) -> i64 {
    let ax_cardinality = clause_set_ref_stack_cardinality(clause_sets) + formula_cardinality;

    if ax_cardinality <= threshold {
        for set in clause_sets.as_slice() {
            set.push_clause_refs(res_clauses);
        }
    }

    usize_to_i64(res_clauses.len())
}

/// Clause-only equivalent of C `SelectDefiningAxioms`.
///
/// Returns the number of newly selected clauses pushed by this defining-axiom
/// traversal. The queue must contain C-shaped adjacent type/pointer entries
/// produced by [`pqueue_store_clause`].
///
/// # Panics
///
/// Panics if the queue is malformed, if it contains formula entries, or if a
/// selected clause contains a function code outside `signature_size`.
pub fn select_defining_axioms_clause_sets<'a>(
    drel: &mut DRelation<'a>,
    internal_symbols: FunCode,
    signature_size: usize,
    max_recursion_depth: i64,
    max_set_size: i64,
    axioms: &mut PQueue<IntOrP<&'a Clause>>,
    res_clauses: &mut PStack<&'a Clause>,
) -> i64 {
    let mut dist_array = vec![0; signature_size];
    let mut selected = BTreeSet::new();
    let mut symbol_stack = Vec::new();
    let mut selected_count = 0;
    let mut recursion_level = 0;
    axioms.store_int(AxiomType::NoType.queue_tag());

    while !axioms.is_empty() {
        if selected_count > max_set_size || recursion_level > max_recursion_depth {
            break;
        }
        let type_ = axioms
            .get_next_int()
            .and_then(AxiomType::from_queue_tag)
            .expect("SInE axiom queue must contain an axiom type tag");
        match type_ {
            AxiomType::NoType => {
                recursion_level += 1;
                if !axioms.is_empty() {
                    axioms.store_int(AxiomType::NoType.queue_tag());
                }
            }
            AxiomType::Clause => {
                let clause = axioms
                    .get_next_pointer()
                    .expect("SInE clause queue tag must be followed by a clause");
                if !selected.insert(std::ptr::from_ref(clause)) {
                    continue;
                }
                res_clauses.push(clause);
                clause.add_symbol_dist_exist(&mut dist_array, &mut symbol_stack);
                selected_count += 1;
                enqueue_new_symbol_relations(
                    drel,
                    internal_symbols,
                    &mut dist_array,
                    &mut symbol_stack,
                    axioms,
                );
            }
            AxiomType::Formula => {
                panic!("formula SInE selection is not represented in the clause-only queue");
            }
        }
    }

    selected_count
}

/// Clause-only equivalent of C `SelectAxioms`.
///
/// `generality` must already contain the symbol distribution for `clause_sets`,
/// matching the C `StructFOFSpecInitDistrib`/`StructFOFSpecAddProblem` setup.
///
/// # Panics
///
/// Panics under the same clause-only queue and function-code preconditions as
/// [`select_defining_axioms_clause_sets`].
pub fn select_axioms_clause_sets<'a>(
    generality: &mut GenDistrib,
    clause_sets: &PStack<&'a ClauseSet>,
    seed_start: usize,
    params: ClauseSineParams,
    res_clauses: &mut PStack<&'a Clause>,
) -> i64 {
    let mut drel = DRelation::new();
    let mut selq = PQueue::new();

    drel.add_clause_sets(
        generality,
        params.gen_measure,
        params.benevolence,
        params.generosity,
        clause_sets,
    );
    let mut seeds = 0;
    for set in clause_sets.as_slice().iter().skip(seed_start) {
        seeds += clause_set_find_ax_selection_seeds(set, &mut selq, params.use_hypotheses);
    }
    if seeds == 0 {
        return 0;
    }

    let ax_cardinality = clause_set_ref_stack_cardinality(clause_sets);
    let max_result_size =
        max_sine_result_size(ax_cardinality, params.max_set_size, params.max_set_fraction);
    let mut selected_count = 0;
    if params.add_no_symbol_axioms {
        if let Some(no_symbol_axioms) = drel.entry(0) {
            for clause in no_symbol_axioms.d_clauses().as_slice() {
                res_clauses.push(*clause);
            }
        }
        selected_count = usize_to_i64(res_clauses.len());
    }

    selected_count
        + select_defining_axioms_clause_sets(
            &mut drel,
            generality.internal_symbols(),
            generality.size(),
            params.max_recursion_depth,
            max_result_size,
            &mut selq,
            res_clauses,
        )
}

/// Writes the C `PStackClausePrintTSTP` shape.
///
/// # Errors
///
/// Returns a diagnostic if a clause needs an unported `ClauseTSTPPrint` branch,
/// or if the output writer reports a formatting error.
///
/// # Panics
///
/// Panics if a printed clause violates the C clause/literal/term printing
/// preconditions.
pub fn pstack_clause_write_tstp(
    output: &mut impl fmt::Write,
    bank: &TermBank,
    stack: &PStack<&Clause>,
    problem_type: ProblemType,
) -> Result<(), Diagnostic> {
    for clause in stack.as_slice() {
        clause_write_tstp(output, bank, clause, true, true, problem_type)?;
        output.write_char('\n').map_err(tstp_stack_write_error)?;
    }
    Ok(())
}

/// Returns the C `PStackClausePrintTSTP` shape.
///
/// # Errors
///
/// Returns a diagnostic under the same conditions as
/// [`pstack_clause_write_tstp`].
///
/// # Panics
///
/// Panics if a printed clause violates the C clause/literal/term printing
/// preconditions.
pub fn pstack_clause_print_tstp_string(
    bank: &TermBank,
    stack: &PStack<&Clause>,
    problem_type: ProblemType,
) -> Result<String, Diagnostic> {
    let mut output = String::new();
    pstack_clause_write_tstp(&mut output, bank, stack, problem_type)?;
    Ok(output)
}

/// Writes the C `PStackFormulaPrintTSTP` shape.
///
/// # Errors
///
/// Returns a diagnostic if a wrapped formula cannot be rendered, or if the
/// output writer reports a formatting error.
///
/// # Panics
///
/// Panics if a stacked wrapper has no formula term or violates formula printing
/// preconditions.
pub fn pstack_formula_write_tstp(
    output: &mut impl fmt::Write,
    bank: &mut TermBank,
    stack: &PStack<&WrappedFormula>,
    problem_type: ProblemType,
    keep_input_names: bool,
) -> Result<(), Diagnostic> {
    for formula in stack.as_slice() {
        output
            .write_str(&formula.tstp_string(bank, true, true, problem_type, keep_input_names)?)
            .map_err(tstp_formula_stack_write_error)?;
        output
            .write_char('\n')
            .map_err(tstp_formula_stack_write_error)?;
    }
    Ok(())
}

/// Returns the C `PStackFormulaPrintTSTP` shape.
///
/// # Errors
///
/// Returns a diagnostic under the same conditions as
/// [`pstack_formula_write_tstp`].
///
/// # Panics
///
/// Panics if a stacked wrapper has no formula term or violates formula printing
/// preconditions.
pub fn pstack_formula_print_tstp_string(
    bank: &mut TermBank,
    stack: &PStack<&WrappedFormula>,
    problem_type: ProblemType,
    keep_input_names: bool,
) -> Result<String, Diagnostic> {
    let mut output = String::new();
    pstack_formula_write_tstp(&mut output, bank, stack, problem_type, keep_input_names)?;
    Ok(output)
}

fn tstp_stack_write_error(_error: fmt::Error) -> Diagnostic {
    Diagnostic::new(ErrorCode::OTHER_ERROR, "failed to write TSTP clause stack")
}

fn tstp_formula_stack_write_error(_error: fmt::Error) -> Diagnostic {
    Diagnostic::new(ErrorCode::OTHER_ERROR, "failed to write TSTP formula stack")
}

fn enqueue_new_symbol_relations<'a>(
    drel: &mut DRelation<'a>,
    internal_symbols: FunCode,
    dist_array: &mut [i64],
    symbol_stack: &mut Vec<FunCode>,
    axioms: &mut PQueue<IntOrP<&'a Clause>>,
) {
    for &f_code in symbol_stack.iter() {
        if f_code > internal_symbols {
            if let Some(frel) = drel.entry_mut(f_code) {
                if !frel.is_activated() {
                    frel.set_activated(true);
                    for clause in frel.d_clauses().as_slice() {
                        pqueue_store_clause(axioms, clause);
                    }
                }
            }
        }
        dist_array[f_code_index(f_code)] = 0;
    }
    symbol_stack.clear();
}

#[expect(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    reason = "C assigns the double max-set fraction product to a long"
)]
fn max_sine_result_size(ax_cardinality: i64, max_set_size: i64, max_set_fraction: f64) -> i64 {
    let fraction_size = (max_set_fraction * ax_cardinality as f64) as i64;
    max_set_size.min(fraction_size)
}

fn f_code_index(f_code: FunCode) -> usize {
    usize::try_from(f_code).unwrap_or_else(|_| panic!("function code must fit usize: {f_code}"))
}

fn usize_to_i64(value: usize) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

#[cfg(test)]
mod tests {
    use super::{
        clause_set_find_ax_selection_seeds, pqueue_store_clause, pstack_clause_del_prop,
        pstack_clause_print_tstp_string, pstack_clauses_move, pstack_formula_del_prop,
        pstack_formula_print_tstp_string, pstack_formulas_move, select_axioms_clause_sets,
        select_threshold_clause_sets, AxiomType, ClauseSineParams, DRel, DRelation,
    };
    use crate::basics::defines::IntOrP;
    use crate::basics::pqueue::PQueue;
    use crate::basics::pstacks::PStack;
    use crate::basics::simple_stuff::ProblemType;
    use crate::clauses::clause::Clause;
    use crate::clauses::clause_props::{
        CP_INPUT_FORMULA, CP_IS_RELEVANT, CP_TYPE_AXIOM, CP_TYPE_CONJECTURE, CP_TYPE_HYPOTHESIS,
        CP_TYPE_NEG_CONJECTURE,
    };
    use crate::clauses::clauseinfo::ClauseInfo;
    use crate::clauses::clausesets::ClauseSet;
    use crate::clauses::eqn::Eqn;
    use crate::clauses::eqnlist::EqnList;
    use crate::clauses::f_generality::{GenDistrib, GeneralityMeasure};
    use crate::clauses::formulasets::{FormulaSet, WrappedFormula};
    use crate::terms::signature::Signature;
    use crate::terms::termbanks::TermBank;
    use crate::terms::termtypes::{DerefType, Term};
    use crate::terms::typebanks::TypeBank;

    fn test_bank() -> TermBank {
        let mut signature = Signature::new(TypeBank::new());
        signature.insert_internal_codes().unwrap();
        TermBank::new(signature).unwrap()
    }

    #[test]
    fn drel_and_drelation_initialize_and_count_clause_entries_like_c() {
        let mut rel = DRel::new(7);
        assert_eq!(rel.f_code(), 7);
        assert!(!rel.is_activated());
        assert!(rel.d_clauses().is_empty());
        rel.set_activated(true);
        assert!(rel.is_activated());

        let zero_clause = Clause::empty();
        let first = Clause::empty();
        let second = Clause::empty();
        let mut relation = DRelation::new();
        assert_eq!(relation.allocated_size(), 10);
        relation.get_f_entry(0).d_clauses_mut().push(&zero_clause);
        relation.get_f_entry(3).d_clauses_mut().push(&first);
        relation.get_f_entry(12).d_clauses_mut().push(&second);

        assert_eq!(relation.allocated_size(), 13);
        assert_eq!(relation.get_f_entry(12).f_code(), 12);
        assert_eq!(relation.total_entries(), 2);
    }

    #[test]
    fn drel_debug_print_preserves_clause_counts_and_stderr_newline_quirk() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "debug_a");
        let clause = clause_from(vec![literal(&mut bank, &a, &a, true)]);
        let mut rel = DRel::new(a.f_code());
        rel.d_clauses_mut().push(&clause);

        let (output, stderr) = rel.debug_string(bank.signature());

        assert_eq!(
            output,
            format!(
                "% {f_code:6} {name:<15}: {clauses:6} clauses, {formulas:6} formulas\n%formulas: ",
                f_code = a.f_code(),
                name = "debug_a",
                clauses = 1,
                formulas = 0
            )
        );
        assert_eq!(stderr, "\n");
    }

    #[test]
    fn drelation_debug_print_scans_entries_from_index_one_in_array_order() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "debug_rel_a");
        let b = typed_const(&mut bank, "debug_rel_b");
        let clause_a = clause_from(vec![literal(&mut bank, &a, &a, true)]);
        let clause_b = clause_from(vec![literal(&mut bank, &b, &b, true)]);
        let zero_clause = Clause::empty();
        let mut relation = DRelation::new();
        relation
            .get_f_entry(b.f_code())
            .d_clauses_mut()
            .push(&clause_b);
        relation.get_f_entry(0).d_clauses_mut().push(&zero_clause);
        relation
            .get_f_entry(a.f_code())
            .d_clauses_mut()
            .push(&clause_a);

        let (output, stderr) = relation.debug_string(bank.signature());

        assert!(output.starts_with(&format!("% {:6} {:<15}", a.f_code(), "debug_rel_a")));
        assert!(output.contains(&format!("% {:6} {:<15}", b.f_code(), "debug_rel_b")));
        assert!(!output.contains("UNNAMED_DB"));
        assert_eq!(stderr, "\n\n");
    }

    #[test]
    fn drelation_add_clause_uses_drel_symbols_and_zero_fallback_like_c() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "sine_drel_a");
        let b = typed_const(&mut bank, "sine_drel_b");
        let selected = clause_from(vec![literal(&mut bank, &a, &b, true)]);
        let empty = Clause::empty();
        let set = ClauseSet::from_clauses([selected, empty]);
        let mut generality = GenDistrib::new(bank.signature());
        generality.add_clause_set(&set, 1);
        let mut relation = DRelation::new();

        relation.add_clause_set(&mut generality, GeneralityMeasure::Terms, 10.0, 1, &set);

        assert_eq!(
            relation
                .entry(a.f_code())
                .map(|entry| entry.d_clauses().len()),
            Some(1)
        );
        assert_eq!(
            relation
                .entry(b.f_code())
                .map(|entry| entry.d_clauses().len()),
            Some(1)
        );
        assert_eq!(
            relation.entry(0).map(|entry| entry.d_clauses().len()),
            Some(1)
        );
        assert_eq!(relation.total_entries(), 2);
    }

    #[test]
    fn drelation_add_clause_sets_preserves_clause_set_stack_order() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "sine_set_a");
        let b = typed_const(&mut bank, "sine_set_b");
        let c = typed_const(&mut bank, "sine_set_c");
        let mut first_clause = clause_from(vec![literal(&mut bank, &a, &a, true)]);
        let mut second_clause = clause_from(vec![literal(&mut bank, &b, &b, true)]);
        let mut third_clause = clause_from(vec![literal(&mut bank, &c, &c, true)]);
        first_clause.set_ident(10);
        second_clause.set_ident(20);
        third_clause.set_ident(30);
        let first = ClauseSet::from_clauses([first_clause]);
        let second = ClauseSet::from_clauses([second_clause, third_clause]);
        let mut set_stack = PStack::new();
        set_stack.push(&first);
        set_stack.push(&second);
        let mut generality = GenDistrib::new(bank.signature());
        generality.add_clause_sets(&set_stack);
        let mut relation = DRelation::new();

        relation.add_clause_sets(
            &mut generality,
            GeneralityMeasure::Terms,
            10.0,
            0,
            &set_stack,
        );

        let ids = [a.f_code(), b.f_code(), c.f_code()]
            .into_iter()
            .map(|f_code| relation.entry(f_code).unwrap().d_clauses().as_slice()[0].ident())
            .collect::<Vec<_>>();
        assert_eq!(ids, vec![10, 20, 30]);
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

    fn literal(bank: &mut TermBank, left: &Term, right: &Term, positive: bool) -> Eqn {
        Eqn::alloc(left.clone(), right.clone(), bank, positive).unwrap()
    }

    fn clause_from(literals: Vec<Eqn>) -> Clause {
        let mut clause = Clause::alloc(EqnList::from_vec(literals));
        clause.set_weight(clause.standard_weight());
        clause
    }

    #[test]
    fn pstack_clause_print_tstp_string_preserves_stack_order_and_newlines() {
        let mut bank = test_bank();
        let first = typed_const(&mut bank, "sine_a");
        let second = typed_const(&mut bank, "sine_b");
        let third = typed_const(&mut bank, "sine_c");
        let mut unit = clause_from(vec![literal(&mut bank, &first, &second, true)]);
        unit.set_ident(1);
        unit.set_tptp_type(CP_TYPE_AXIOM);
        unit.set_prop(CP_INPUT_FORMULA);
        let mut mixed = clause_from(vec![
            literal(&mut bank, &second, &third, true),
            literal(&mut bank, &third, &first, false),
        ]);
        mixed.set_ident(2);
        mixed.set_tptp_type(CP_TYPE_NEG_CONJECTURE);
        let mut stack = PStack::new();
        stack.push(&unit);
        stack.push(&mixed);

        assert_eq!(
            pstack_clause_print_tstp_string(&bank, &stack, ProblemType::FirstOrder).unwrap(),
            concat!(
                "cnf(c_0_1, axiom, (sine_a=sine_b)).\n",
                "cnf(c_0_2, negated_conjecture, (sine_b=sine_c|sine_c!=sine_a)).\n",
            )
        );
    }

    #[test]
    fn pstack_clause_print_tstp_string_handles_empty_stack() {
        let bank = test_bank();
        let stack = PStack::new();

        assert_eq!(
            pstack_clause_print_tstp_string(&bank, &stack, ProblemType::FirstOrder).unwrap(),
            ""
        );
    }

    #[test]
    fn pstack_formula_print_tstp_string_preserves_stack_order_and_newlines() {
        let mut bank = test_bank();
        let first_left = typed_const(&mut bank, "sine_form_a");
        let first_right = typed_const(&mut bank, "sine_form_b");
        let second_left = typed_const(&mut bank, "sine_form_c");
        let second_right = typed_const(&mut bank, "sine_form_d");
        let first_clause = clause_from(vec![literal(&mut bank, &first_left, &first_right, true)]);
        let second_clause =
            clause_from(vec![literal(&mut bank, &second_left, &second_right, false)]);
        let mut first =
            WrappedFormula::of_clause(&mut bank, &first_clause, ProblemType::FirstOrder)
                .expect("first clause can be encoded as a formula");
        first.set_tptp_type(CP_TYPE_AXIOM);
        first.set_prop(CP_INPUT_FORMULA);
        first.set_info(Some(ClauseInfo::new(
            Some("sine_formula_first"),
            None,
            1,
            1,
        )));
        let mut second =
            WrappedFormula::of_clause(&mut bank, &second_clause, ProblemType::FirstOrder)
                .expect("second clause can be encoded as a formula");
        second.set_tptp_type(CP_TYPE_NEG_CONJECTURE);
        second.set_info(Some(ClauseInfo::new(
            Some("sine_formula_second"),
            None,
            2,
            1,
        )));
        let mut stack = PStack::new();
        stack.push(&first);
        stack.push(&second);

        let rendered =
            pstack_formula_print_tstp_string(&mut bank, &stack, ProblemType::FirstOrder, true)
                .unwrap();

        assert_eq!(
            rendered,
            concat!(
                "fof(sine_formula_first, axiom, sine_form_a=sine_form_b).\n",
                "fof(sine_formula_second, negated_conjecture, sine_form_c!=sine_form_d).\n",
            )
        );
    }

    #[test]
    fn pstack_formula_print_tstp_string_handles_empty_stack() {
        let mut bank = test_bank();
        let stack = PStack::new();

        assert_eq!(
            pstack_formula_print_tstp_string(&mut bank, &stack, ProblemType::FirstOrder, true)
                .unwrap(),
            ""
        );
    }

    #[test]
    fn pstack_clause_del_prop_clears_property_on_each_stacked_clause() {
        let mut first = Clause::empty();
        first.set_prop(CP_INPUT_FORMULA);
        let mut second = Clause::empty();
        second.set_prop(CP_INPUT_FORMULA);
        let mut stack = PStack::new();
        stack.push(&mut first);
        stack.push(&mut second);

        pstack_clause_del_prop(&mut stack, CP_INPUT_FORMULA);
        drop(stack);

        assert!(!first.query_prop(CP_INPUT_FORMULA));
        assert!(!second.query_prop(CP_INPUT_FORMULA));
    }

    #[test]
    fn pstack_clauses_move_preserves_stack_order_and_relinks_duplicates() {
        let mut first = Clause::empty();
        first.set_ident(10);
        let mut second = Clause::empty();
        second.set_ident(20);
        let mut third = Clause::empty();
        third.set_ident(30);
        let mut from = ClauseSet::from_clauses([first, second, third]);
        let mut to = ClauseSet::new();
        let mut stack = PStack::new();
        stack.push(30);
        stack.push(10);
        stack.push(30);

        assert_eq!(pstack_clauses_move(&stack, &mut from, &mut to), 3);

        assert_eq!(from.members(), 1);
        assert_eq!(from.iter().map(Clause::ident).collect::<Vec<_>>(), vec![20]);
        assert_eq!(
            to.iter().map(Clause::ident).collect::<Vec<_>>(),
            vec![10, 30]
        );
    }

    #[test]
    fn pstack_formula_del_prop_clears_property_on_each_stacked_formula() {
        let mut bank = test_bank();
        let mut first =
            WrappedFormula::wt_formula_alloc(typed_const(&mut bank, "sine_formula_prop_first"));
        first.set_prop(CP_INPUT_FORMULA | CP_IS_RELEVANT);
        let mut second =
            WrappedFormula::wt_formula_alloc(typed_const(&mut bank, "sine_formula_prop_second"));
        second.set_prop(CP_INPUT_FORMULA | CP_IS_RELEVANT);
        let mut stack = PStack::new();
        stack.push(&mut first);
        stack.push(&mut second);

        pstack_formula_del_prop(&mut stack, CP_INPUT_FORMULA);
        drop(stack);

        assert!(!first.query_prop(CP_INPUT_FORMULA));
        assert!(!second.query_prop(CP_INPUT_FORMULA));
        assert!(first.query_prop(CP_IS_RELEVANT));
        assert!(second.query_prop(CP_IS_RELEVANT));
    }

    #[test]
    fn pstack_formulas_move_preserves_stack_order_and_relinks_duplicates() {
        let mut bank = test_bank();
        let first = WrappedFormula::wt_formula_alloc(typed_const(&mut bank, "sine_move_first"));
        let second = WrappedFormula::wt_formula_alloc(typed_const(&mut bank, "sine_move_second"));
        let third = WrappedFormula::wt_formula_alloc(typed_const(&mut bank, "sine_move_third"));
        let first_id = first.entry_id();
        let second_id = second.entry_id();
        let third_id = third.entry_id();
        let mut from = FormulaSet::new();
        from.insert(first);
        from.insert(second);
        from.insert(third);
        let mut to = FormulaSet::new();
        let mut stack = PStack::new();
        stack.push(third_id);
        stack.push(first_id);
        stack.push(third_id);

        assert_eq!(pstack_formulas_move(&stack, &mut from, &mut to), 3);

        assert_eq!(from.cardinality(), 1);
        assert_eq!(
            from.iter()
                .map(WrappedFormula::entry_id)
                .collect::<Vec<_>>(),
            vec![second_id]
        );
        assert_eq!(
            to.iter().map(WrappedFormula::entry_id).collect::<Vec<_>>(),
            vec![first_id, third_id]
        );
    }

    #[test]
    fn pqueue_store_clause_writes_c_tag_pointer_tuple() {
        let clause = Clause::empty();
        let mut queue = PQueue::<IntOrP<&Clause>>::new();

        pqueue_store_clause(&mut queue, &clause);

        assert_eq!(queue.get_next_int(), Some(AxiomType::Clause.queue_tag()));
        let stored = queue.get_next_pointer().expect("stored clause pointer");
        assert_eq!(std::ptr::from_ref(stored), std::ptr::from_ref(&clause));
        assert!(queue.is_empty());
    }

    #[test]
    fn clause_set_find_ax_selection_seeds_keeps_set_order_and_optional_hypotheses() {
        let mut axiom = Clause::empty();
        axiom.set_ident(1);
        axiom.set_tptp_type(CP_TYPE_AXIOM);
        let mut conjecture = Clause::empty();
        conjecture.set_ident(2);
        conjecture.set_tptp_type(CP_TYPE_CONJECTURE);
        let mut hypothesis = Clause::empty();
        hypothesis.set_ident(3);
        hypothesis.set_tptp_type(CP_TYPE_HYPOTHESIS);
        let mut neg_conjecture = Clause::empty();
        neg_conjecture.set_ident(4);
        neg_conjecture.set_tptp_type(CP_TYPE_NEG_CONJECTURE);
        let set = ClauseSet::from_clauses([axiom, conjecture, hypothesis, neg_conjecture]);

        let mut without_hypotheses = PQueue::<IntOrP<&Clause>>::new();
        assert_eq!(
            clause_set_find_ax_selection_seeds(&set, &mut without_hypotheses, false),
            2
        );
        assert_eq!(
            without_hypotheses.get_next_int(),
            Some(AxiomType::Clause.queue_tag())
        );
        assert_eq!(
            without_hypotheses.get_next_pointer().map(Clause::ident),
            Some(2)
        );
        assert_eq!(
            without_hypotheses.get_next_int(),
            Some(AxiomType::Clause.queue_tag())
        );
        assert_eq!(
            without_hypotheses.get_next_pointer().map(Clause::ident),
            Some(4)
        );
        assert!(without_hypotheses.is_empty());

        let mut with_hypotheses = PQueue::<IntOrP<&Clause>>::new();
        assert_eq!(
            clause_set_find_ax_selection_seeds(&set, &mut with_hypotheses, true),
            3
        );
        assert_eq!(with_hypotheses.get_next_int(), Some(1));
        assert_eq!(
            with_hypotheses.get_next_pointer().map(Clause::ident),
            Some(2)
        );
        assert_eq!(with_hypotheses.get_next_int(), Some(1));
        assert_eq!(
            with_hypotheses.get_next_pointer().map(Clause::ident),
            Some(3)
        );
        assert_eq!(with_hypotheses.get_next_int(), Some(1));
        assert_eq!(
            with_hypotheses.get_next_pointer().map(Clause::ident),
            Some(4)
        );
        assert!(with_hypotheses.is_empty());
    }

    #[test]
    fn select_threshold_clause_sets_pushes_all_clause_refs_under_combined_limit() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "threshold_a");
        let b = typed_const(&mut bank, "threshold_b");
        let c = typed_const(&mut bank, "threshold_c");
        let mut first_clause = clause_from(vec![literal(&mut bank, &a, &a, true)]);
        let mut second_clause = clause_from(vec![literal(&mut bank, &b, &b, true)]);
        let mut third_clause = clause_from(vec![literal(&mut bank, &c, &c, true)]);
        first_clause.set_ident(10);
        second_clause.set_ident(20);
        third_clause.set_ident(30);
        let first = ClauseSet::from_clauses([first_clause, second_clause]);
        let second = ClauseSet::from_clauses([third_clause]);
        let mut sets = PStack::new();
        sets.push(&first);
        sets.push(&second);
        let mut result = PStack::new();

        assert_eq!(select_threshold_clause_sets(&sets, 0, 3, &mut result), 3);
        assert_eq!(
            result
                .as_slice()
                .iter()
                .map(|clause| clause.ident())
                .collect::<Vec<_>>(),
            vec![10, 20, 30]
        );
    }

    #[test]
    fn select_threshold_clause_sets_returns_existing_result_len_when_over_limit() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "threshold_kept");
        let b = typed_const(&mut bank, "threshold_blocked");
        let existing = clause_from(vec![literal(&mut bank, &a, &a, true)]);
        let candidate = clause_from(vec![literal(&mut bank, &b, &b, true)]);
        let set = ClauseSet::from_clauses([candidate]);
        let mut sets = PStack::new();
        sets.push(&set);
        let mut result = PStack::new();
        result.push(&existing);

        assert_eq!(select_threshold_clause_sets(&sets, 1, 1, &mut result), 1);
        assert_eq!(result.as_slice()[0].ident(), existing.ident());
    }

    #[test]
    fn select_axioms_clause_sets_follows_drelation_layers_and_recursion_limit() {
        let mut bank = test_bank();
        let goal_symbol = typed_const(&mut bank, "gsine_goal");
        let bridge_symbol = typed_const(&mut bank, "gsine_bridge");
        let far_symbol = typed_const(&mut bank, "gsine_far");
        let unrelated_symbol = typed_const(&mut bank, "gsine_unrelated");
        let mut goal = clause_from(vec![literal(&mut bank, &goal_symbol, &goal_symbol, true)]);
        goal.set_ident(10);
        goal.set_tptp_type(CP_TYPE_CONJECTURE);
        let mut bridge = clause_from(vec![literal(&mut bank, &goal_symbol, &bridge_symbol, true)]);
        bridge.set_ident(20);
        bridge.set_tptp_type(CP_TYPE_AXIOM);
        let mut far = clause_from(vec![literal(&mut bank, &bridge_symbol, &far_symbol, true)]);
        far.set_ident(30);
        far.set_tptp_type(CP_TYPE_AXIOM);
        let mut unrelated = clause_from(vec![literal(
            &mut bank,
            &unrelated_symbol,
            &unrelated_symbol,
            true,
        )]);
        unrelated.set_ident(40);
        unrelated.set_tptp_type(CP_TYPE_AXIOM);
        let set = ClauseSet::from_clauses([goal, bridge, far, unrelated]);
        let mut sets = PStack::new();
        sets.push(&set);
        let mut generality = GenDistrib::new(bank.signature());
        generality.add_clause_sets(&sets);
        let mut params = ClauseSineParams::g_sine(GeneralityMeasure::Terms);
        params.benevolence = 10.0;
        params.max_recursion_depth = 2;
        let mut selected = PStack::new();

        let selected_count =
            select_axioms_clause_sets(&mut generality, &sets, 0, params, &mut selected);

        assert_eq!(selected_count, 3);
        assert_eq!(
            selected
                .as_slice()
                .iter()
                .map(|clause| clause.ident())
                .collect::<Vec<_>>(),
            vec![10, 20, 30]
        );
    }

    #[test]
    fn select_axioms_clause_sets_stops_after_c_recursion_marker_limit() {
        let mut bank = test_bank();
        let goal_symbol = typed_const(&mut bank, "gsine_limited_goal");
        let bridge_symbol = typed_const(&mut bank, "gsine_limited_bridge");
        let far_symbol = typed_const(&mut bank, "gsine_limited_far");
        let mut goal = clause_from(vec![literal(&mut bank, &goal_symbol, &goal_symbol, true)]);
        goal.set_ident(10);
        goal.set_tptp_type(CP_TYPE_CONJECTURE);
        let mut bridge = clause_from(vec![literal(&mut bank, &goal_symbol, &bridge_symbol, true)]);
        bridge.set_ident(20);
        bridge.set_tptp_type(CP_TYPE_AXIOM);
        let mut far = clause_from(vec![literal(&mut bank, &bridge_symbol, &far_symbol, true)]);
        far.set_ident(30);
        far.set_tptp_type(CP_TYPE_AXIOM);
        let set = ClauseSet::from_clauses([goal, bridge, far]);
        let mut sets = PStack::new();
        sets.push(&set);
        let mut generality = GenDistrib::new(bank.signature());
        generality.add_clause_sets(&sets);
        let mut params = ClauseSineParams::g_sine(GeneralityMeasure::Terms);
        params.benevolence = 10.0;
        params.max_recursion_depth = 1;
        let mut selected = PStack::new();

        let selected_count =
            select_axioms_clause_sets(&mut generality, &sets, 0, params, &mut selected);

        assert_eq!(selected_count, 2);
        assert_eq!(
            selected
                .as_slice()
                .iter()
                .map(|clause| clause.ident())
                .collect::<Vec<_>>(),
            vec![10, 20]
        );
    }

    #[test]
    fn select_axioms_clause_sets_returns_empty_selection_without_seeds() {
        let mut bank = test_bank();
        let axiom_symbol = typed_const(&mut bank, "gsine_no_seed");
        let mut axiom = clause_from(vec![literal(&mut bank, &axiom_symbol, &axiom_symbol, true)]);
        axiom.set_ident(10);
        axiom.set_tptp_type(CP_TYPE_AXIOM);
        let set = ClauseSet::from_clauses([axiom]);
        let mut sets = PStack::new();
        sets.push(&set);
        let mut generality = GenDistrib::new(bank.signature());
        generality.add_clause_sets(&sets);
        let params = ClauseSineParams::g_sine(GeneralityMeasure::Terms);
        let mut selected = PStack::new();

        assert_eq!(
            select_axioms_clause_sets(&mut generality, &sets, 0, params, &mut selected),
            0
        );
        assert!(selected.is_empty());
    }

    #[test]
    fn select_axioms_clause_sets_preserves_no_symbol_duplicate_quirk() {
        let mut goal = Clause::empty();
        goal.set_ident(10);
        goal.set_tptp_type(CP_TYPE_CONJECTURE);
        let set = ClauseSet::from_clauses([goal]);
        let bank = test_bank();
        let mut sets = PStack::new();
        sets.push(&set);
        let mut generality = GenDistrib::new(bank.signature());
        generality.add_clause_sets(&sets);
        let mut params = ClauseSineParams::g_sine(GeneralityMeasure::Terms);
        params.add_no_symbol_axioms = true;
        let mut selected = PStack::new();

        let selected_count =
            select_axioms_clause_sets(&mut generality, &sets, 0, params, &mut selected);

        assert_eq!(selected_count, 2);
        assert_eq!(
            selected
                .as_slice()
                .iter()
                .map(|clause| clause.ident())
                .collect::<Vec<_>>(),
            vec![10, 10]
        );
    }
}
