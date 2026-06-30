use crate::clauses::clause::Clause;
use crate::clauses::clausesets::ClauseSet;
use crate::clauses::formulasets::FormulaSet;
use crate::terms::termbanks::TermBank;

pub trait ClauseSetMarker {
    fn mark_clause_terms(&self, bank: &TermBank);
}

pub trait FormulaSetMarker {
    fn mark_formula_cells(&self, bank: &TermBank);
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct EmptyFormulaSet;

impl FormulaSetMarker for EmptyFormulaSet {
    fn mark_formula_cells(&self, _bank: &TermBank) {}
}

impl FormulaSetMarker for FormulaSet {
    fn mark_formula_cells(&self, bank: &TermBank) {
        self.gc_mark_cells(bank);
    }
}

impl FormulaSetMarker for &FormulaSet {
    fn mark_formula_cells(&self, bank: &TermBank) {
        (*self).gc_mark_cells(bank);
    }
}

impl ClauseSetMarker for Vec<Clause> {
    fn mark_clause_terms(&self, bank: &TermBank) {
        mark_clause_slice_terms(bank, self);
    }
}

impl ClauseSetMarker for &[Clause] {
    fn mark_clause_terms(&self, bank: &TermBank) {
        mark_clause_slice_terms(bank, self);
    }
}

impl<const N: usize> ClauseSetMarker for [Clause; N] {
    fn mark_clause_terms(&self, bank: &TermBank) {
        mark_clause_slice_terms(bank, self);
    }
}

impl ClauseSetMarker for ClauseSet {
    fn mark_clause_terms(&self, bank: &TermBank) {
        for clause in self.iter() {
            clause.gc_mark_terms(bank);
        }
    }
}

#[must_use]
pub fn tb_gc_collect<C, F>(bank: &mut TermBank, clause_sets: &[C], formula_sets: &[F]) -> i64
where
    C: ClauseSetMarker,
    F: FormulaSetMarker,
{
    for clause_set in clause_sets {
        clause_set.mark_clause_terms(bank);
    }
    for formula_set in formula_sets {
        formula_set.mark_formula_cells(bank);
    }
    bank.gc_sweep()
}

fn mark_clause_slice_terms(bank: &TermBank, clauses: &[Clause]) {
    for clause in clauses {
        clause.gc_mark_terms(bank);
    }
}

#[cfg(test)]
mod tests {
    use super::{tb_gc_collect, ClauseSetMarker, EmptyFormulaSet, FormulaSetMarker};
    use crate::clauses::clause::Clause;
    use crate::clauses::clausesets::ClauseSet;
    use crate::clauses::eqn::Eqn;
    use crate::clauses::eqnlist::EqnList;
    use crate::clauses::formulasets::{FormulaSet, WrappedFormula};
    use crate::terms::signature::Signature;
    use crate::terms::simpletypes::alloc_arrow_type;
    use crate::terms::termbanks::TermBank;
    use crate::terms::termtypes::{DerefType, Term};
    use crate::terms::typebanks::TypeBank;
    use std::cell::RefCell;

    struct RecordingClauseSet<'log> {
        log: &'log RefCell<Vec<&'static str>>,
        clauses: Vec<Clause>,
    }

    impl ClauseSetMarker for RecordingClauseSet<'_> {
        fn mark_clause_terms(&self, bank: &TermBank) {
            self.log.borrow_mut().push("clause");
            for clause in &self.clauses {
                clause.gc_mark_terms(bank);
            }
        }
    }

    struct RecordingFormulaSet<'log> {
        log: &'log RefCell<Vec<&'static str>>,
        terms: Vec<Term>,
    }

    impl FormulaSetMarker for RecordingFormulaSet<'_> {
        fn mark_formula_cells(&self, bank: &TermBank) {
            self.log.borrow_mut().push("formula");
            for term in &self.terms {
                bank.gc_mark_term(term);
            }
        }
    }

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
        bank.insert(&term, DerefType::Never).unwrap()
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
        bank.insert(&term, DerefType::Never).unwrap()
    }

    fn unit_clause(bank: &mut TermBank, left: &Term, right: &Term) -> Clause {
        let literal = Eqn::alloc(left.clone(), right.clone(), bank, true).unwrap();
        Clause::alloc(EqnList::from_vec(vec![literal]))
    }

    #[test]
    fn collection_marks_clause_sets_before_sweeping_terms() {
        let mut bank = test_bank();
        let kept = typed_const(&mut bank, "kept");
        let right = typed_const(&mut bank, "right");
        let dropped = typed_const(&mut bank, "dropped");
        let clause = unit_clause(&mut bank, &kept, &right);
        let clause_sets = [vec![clause]];
        let formula_sets: [EmptyFormulaSet; 0] = [];

        assert_eq!(tb_gc_collect(&mut bank, &clause_sets, &formula_sets), 1);
        assert!(bank.find(&kept).is_some());
        assert!(bank.find(&right).is_some());
        assert!(bank.find(&dropped).is_none());
    }

    #[test]
    fn collection_marks_formula_sets_after_clause_sets_then_sweeps() {
        let mut bank = test_bank();
        let log = RefCell::new(Vec::new());
        let clause_term = typed_const(&mut bank, "clause_term");
        let formula_arg = typed_const(&mut bank, "formula_arg");
        let formula_term = typed_unary(&mut bank, "formula", &formula_arg);
        let dropped = typed_const(&mut bank, "dropped");
        let clause = unit_clause(&mut bank, &clause_term, &clause_term);
        let clause_sets = [RecordingClauseSet {
            log: &log,
            clauses: vec![clause],
        }];
        let formula_sets = [RecordingFormulaSet {
            log: &log,
            terms: vec![formula_term.clone()],
        }];

        assert_eq!(tb_gc_collect(&mut bank, &clause_sets, &formula_sets), 1);
        assert_eq!(log.borrow().as_slice(), ["clause", "formula"]);
        assert!(bank.find(&clause_term).is_some());
        assert!(bank.find(&formula_term).is_some());
        assert!(bank.find(&formula_arg).is_some());
        assert!(bank.find(&dropped).is_none());
    }

    #[test]
    fn collection_accepts_plain_clause_sets() {
        let mut bank = test_bank();
        let kept = typed_const(&mut bank, "kept");
        let right = typed_const(&mut bank, "right");
        let dropped = typed_const(&mut bank, "dropped");
        let clause = unit_clause(&mut bank, &kept, &right);
        let clause_sets = [ClauseSet::from_clauses([clause])];
        let formula_sets: [EmptyFormulaSet; 0] = [];

        assert_eq!(tb_gc_collect(&mut bank, &clause_sets, &formula_sets), 1);
        assert!(bank.find(&kept).is_some());
        assert!(bank.find(&right).is_some());
        assert!(bank.find(&dropped).is_none());
    }

    #[test]
    fn collection_accepts_plain_formula_sets() {
        let mut bank = test_bank();
        let formula_arg = typed_const(&mut bank, "plain_formula_arg");
        let formula_term = typed_unary(&mut bank, "plain_formula", &formula_arg);
        let dropped = typed_const(&mut bank, "plain_formula_dropped");
        let mut set = FormulaSet::new();
        set.insert(WrappedFormula::default_alloc());
        set.insert(WrappedFormula::wt_formula_alloc(formula_term.clone()));
        let clause_sets: [Vec<Clause>; 0] = [];
        let formula_sets = [&set];

        assert_eq!(tb_gc_collect(&mut bank, &clause_sets, &formula_sets), 1);
        assert!(bank.find(&formula_term).is_some());
        assert!(bank.find(&formula_arg).is_some());
        assert!(bank.find(&dropped).is_none());
    }
}
