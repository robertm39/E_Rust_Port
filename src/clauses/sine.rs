use crate::basics::defines::IntOrP;
use crate::basics::error::{Diagnostic, ErrorCode};
use crate::basics::pqueue::{PQueue, PQueueInt};
use crate::basics::pstacks::PStack;
use crate::basics::simple_stuff::ProblemType;
use crate::clauses::clause::{clause_write_tstp, Clause};
use crate::clauses::clause_props::FormulaProperties;
use crate::clauses::clausesets::ClauseSet;
use crate::terms::functypes::FunCode;
use crate::terms::termbanks::TermBank;
use std::fmt;

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
}

pub fn pstack_clause_del_prop(stack: &mut PStack<&mut Clause>, prop: FormulaProperties) {
    for clause in stack.as_mut_slice() {
        (*clause).del_prop(prop);
    }
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

fn tstp_stack_write_error(_error: fmt::Error) -> Diagnostic {
    Diagnostic::new(ErrorCode::OTHER_ERROR, "failed to write TSTP clause stack")
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
        pstack_clause_print_tstp_string, AxiomType, DRel, DRelation,
    };
    use crate::basics::defines::IntOrP;
    use crate::basics::pqueue::PQueue;
    use crate::basics::pstacks::PStack;
    use crate::basics::simple_stuff::ProblemType;
    use crate::clauses::clause::Clause;
    use crate::clauses::clause_props::{
        CP_INPUT_FORMULA, CP_TYPE_AXIOM, CP_TYPE_CONJECTURE, CP_TYPE_HYPOTHESIS,
        CP_TYPE_NEG_CONJECTURE,
    };
    use crate::clauses::clausesets::ClauseSet;
    use crate::clauses::eqn::Eqn;
    use crate::clauses::eqnlist::EqnList;
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
}
