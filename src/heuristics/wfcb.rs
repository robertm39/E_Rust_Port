use crate::clauses::clause::Clause;
use crate::clauses::neweval::{EvalCell, EvalPriority, PRIO_BEST};
use crate::terms::termbanks::TermBank;

pub type ClauseEvalFun<Data> = fn(Option<&mut Data>, &Clause) -> f64;
pub use crate::heuristics::prio_funs::ClausePrioFun;
pub type GenericExitFun<Data> = fn(Data);
pub type BoxedWfcb = Box<dyn WfcbOps>;

pub trait WfcbOps {
    fn compute_eval(&mut self, clause: &Clause) -> f64;
    fn compute_priority(&self, bank: &TermBank, clause: &Clause) -> EvalPriority;
    fn add_evaluation(
        &mut self,
        evaluations: &mut EvalCell,
        bank: &TermBank,
        clause: &Clause,
        pos: usize,
        empty: bool,
    );
}

pub struct Wfcb<Data> {
    eval_fun: ClauseEvalFun<Data>,
    exit_fun: GenericExitFun<Data>,
    priority_fun: ClausePrioFun,
    data: Option<Data>,
}

impl<Data> Wfcb<Data> {
    #[must_use]
    pub const fn new(
        wfcb_eval: ClauseEvalFun<Data>,
        prio_fun: ClausePrioFun,
        wfcb_exit: GenericExitFun<Data>,
        data: Option<Data>,
    ) -> Self {
        Self {
            eval_fun: wfcb_eval,
            exit_fun: wfcb_exit,
            priority_fun: prio_fun,
            data,
        }
    }

    #[must_use]
    pub const fn data(&self) -> Option<&Data> {
        self.data.as_ref()
    }

    #[must_use]
    pub fn compute_eval(&mut self, clause: &Clause) -> f64 {
        (self.eval_fun)(self.data.as_mut(), clause)
    }

    #[must_use]
    pub fn compute_priority(&self, bank: &TermBank, clause: &Clause) -> EvalPriority {
        (self.priority_fun)(bank, clause)
    }
}

impl<Data> Drop for Wfcb<Data> {
    fn drop(&mut self) {
        if let Some(data) = self.data.take() {
            (self.exit_fun)(data);
        }
    }
}

impl<Data> WfcbOps for Wfcb<Data> {
    fn compute_eval(&mut self, clause: &Clause) -> f64 {
        Self::compute_eval(self, clause)
    }

    fn compute_priority(&self, bank: &TermBank, clause: &Clause) -> EvalPriority {
        Self::compute_priority(self, bank, clause)
    }

    fn add_evaluation(
        &mut self,
        evaluations: &mut EvalCell,
        bank: &TermBank,
        clause: &Clause,
        pos: usize,
        empty: bool,
    ) {
        clause_add_evaluation(self, evaluations, bank, clause, pos, empty);
    }
}

#[must_use]
pub const fn wfcb_alloc<Data>(
    wfcb_eval: ClauseEvalFun<Data>,
    prio_fun: ClausePrioFun,
    wfcb_exit: GenericExitFun<Data>,
    data: Option<Data>,
) -> Wfcb<Data> {
    Wfcb::new(wfcb_eval, prio_fun, wfcb_exit, data)
}

/// Adds a WFCB-computed evaluation to an existing evaluation list.
///
/// # Panics
///
/// Panics if `pos` is outside `evaluations`, matching the C function's
/// unchecked `clause->evaluations->evals[pos]` access after its non-null
/// evaluation-list assertion.
pub fn clause_add_evaluation<Data>(
    wfcb: &mut Wfcb<Data>,
    evaluations: &mut EvalCell,
    bank: &TermBank,
    clause: &Clause,
    pos: usize,
    empty: bool,
) {
    let heuristic = wfcb.compute_eval(clause);
    let eval = evaluations.eval_mut(pos);
    eval.set_heuristic_from_eval(heuristic);
    if empty {
        eval.set_priority(PRIO_BEST);
    } else {
        eval.set_priority(wfcb.compute_priority(bank, clause));
    }
}

#[cfg(test)]
mod tests {
    use super::{clause_add_evaluation, wfcb_alloc, Wfcb};
    use crate::clauses::clause::Clause;
    use crate::clauses::neweval::{evals_alloc, EvalPriority, PRIO_BEST, PRIO_NORMAL};
    use crate::terms::signature::Signature;
    use crate::terms::termbanks::TermBank;
    use crate::terms::typebanks::TypeBank;
    use std::cell::Cell;
    use std::rc::Rc;

    #[derive(Debug)]
    struct EvalData {
        base: f64,
        exit_count: Rc<Cell<i32>>,
    }

    fn eval_with_data(data: Option<&mut EvalData>, _clause: &Clause) -> f64 {
        data.map_or(0.0, |data| data.base)
    }

    fn constant_priority(_bank: &TermBank, _clause: &Clause) -> EvalPriority {
        PRIO_NORMAL + 7
    }

    fn term_bank() -> TermBank {
        TermBank::new(Signature::new(TypeBank::new())).unwrap_or_else(|err| panic!("{err}"))
    }

    fn record_exit(data: EvalData) {
        let EvalData {
            base: _,
            exit_count,
        } = data;
        exit_count.set(exit_count.get() + 1);
    }

    #[test]
    fn allocation_preserves_callbacks_and_data_until_drop() {
        let exit_count = Rc::new(Cell::new(0));
        let mut wfcb = wfcb_alloc(
            eval_with_data,
            constant_priority,
            record_exit,
            Some(EvalData {
                base: 12.5,
                exit_count: Rc::clone(&exit_count),
            }),
        );
        let clause = Clause::empty();

        assert!(wfcb.data().is_some());
        assert_eq!(wfcb.compute_eval(&clause).to_bits(), 12.5_f64.to_bits());
        assert_eq!(
            wfcb.compute_priority(&term_bank(), &clause),
            PRIO_NORMAL + 7
        );
        assert_eq!(exit_count.get(), 0);

        drop(wfcb);

        assert_eq!(exit_count.get(), 1);
    }

    #[test]
    fn dropping_wfcb_with_no_data_does_not_call_exit() {
        let exit_count = Rc::new(Cell::new(0));
        let wfcb = wfcb_alloc::<EvalData>(eval_with_data, constant_priority, record_exit, None);

        drop(wfcb);

        assert_eq!(exit_count.get(), 0);
    }

    #[test]
    fn clause_add_evaluation_writes_heuristic_and_priority() {
        let exit_count = Rc::new(Cell::new(0));
        let mut wfcb = wfcb_alloc(
            eval_with_data,
            constant_priority,
            record_exit,
            Some(EvalData {
                base: 19.25,
                exit_count,
            }),
        );
        let clause = Clause::empty();
        let bank = term_bank();
        let mut evaluations = evals_alloc(2);

        clause_add_evaluation(&mut wfcb, &mut evaluations, &bank, &clause, 1, false);

        assert_eq!(
            evaluations.eval(1).heuristic().to_bits(),
            19.25_f32.to_bits()
        );
        assert_eq!(evaluations.eval(1).priority(), PRIO_NORMAL + 7);
    }

    #[test]
    fn empty_clause_evaluation_uses_best_priority_but_still_computes_weight() {
        let exit_count = Rc::new(Cell::new(0));
        let mut wfcb: Wfcb<EvalData> = wfcb_alloc(
            eval_with_data,
            constant_priority,
            record_exit,
            Some(EvalData {
                base: 3.5,
                exit_count,
            }),
        );
        let clause = Clause::empty();
        let bank = term_bank();
        let mut evaluations = evals_alloc(1);

        clause_add_evaluation(&mut wfcb, &mut evaluations, &bank, &clause, 0, true);

        assert_eq!(evaluations.eval(0).heuristic().to_bits(), 3.5_f32.to_bits());
        assert_eq!(evaluations.eval(0).priority(), PRIO_BEST);
    }
}
