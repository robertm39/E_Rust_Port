use crate::basics::error::Diagnostic;
use crate::clauses::clause::Clause;
use crate::heuristics::prio_funs::parse_prio_fun;
use crate::heuristics::wfcb::{wfcb_alloc, ClausePrioFun, Wfcb};
use crate::inout::scanner::{Scanner, TokenType};
use crate::terms::termbanks::TermBank;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LifoEvaluator {
    counter: f64,
}

impl Default for LifoEvaluator {
    fn default() -> Self {
        Self::new()
    }
}

impl LifoEvaluator {
    #[must_use]
    pub const fn new() -> Self {
        Self { counter: 0.0 }
    }

    #[must_use]
    pub const fn counter(self) -> f64 {
        self.counter
    }

    pub fn compute(&mut self, clause: &Clause) -> f64 {
        lifo_eval_compute(&mut self.counter, clause)
    }
}

#[must_use]
pub const fn lifo_eval_init() -> LifoEvaluator {
    LifoEvaluator::new()
}

pub fn lifo_eval_compute(counter: &mut f64, _clause: &Clause) -> f64 {
    *counter -= 1.0;
    *counter
}

#[must_use]
pub fn lifo_eval_wfcb_init(prio_fun: ClausePrioFun) -> Wfcb<LifoEvaluator> {
    wfcb_alloc(
        lifo_eval_wfcb_compute,
        prio_fun,
        lifo_eval_exit,
        Some(lifo_eval_init()),
    )
}

pub fn lifo_eval_parse(scanner: &mut Scanner) -> Result<Wfcb<LifoEvaluator>, Diagnostic> {
    scanner.accept_tok(TokenType::OPEN_BRACKET)?;
    let prio_fun = parse_prio_fun(scanner)?;
    scanner.accept_tok(TokenType::CLOSE_BRACKET)?;
    Ok(lifo_eval_wfcb_init(prio_fun))
}

fn lifo_eval_wfcb_compute(
    data: Option<&mut LifoEvaluator>,
    _bank: &TermBank,
    clause: &Clause,
) -> f64 {
    match data {
        Some(data) => data.compute(clause),
        None => panic!("LIFO WFCB requires initialized counter data"),
    }
}

fn lifo_eval_exit(_data: LifoEvaluator) {}

#[cfg(test)]
mod tests {
    use super::{lifo_eval_compute, lifo_eval_init, lifo_eval_wfcb_init};
    use crate::clauses::clause::Clause;
    use crate::clauses::neweval::{evals_alloc, EvalPriority, PRIO_NORMAL};
    use crate::heuristics::wfcb::clause_add_evaluation;
    use crate::inout::scanner::Scanner;
    use crate::terms::signature::Signature;
    use crate::terms::termbanks::TermBank;
    use crate::terms::typebanks::TypeBank;

    fn assert_close(actual: f64, expected: f64) {
        assert!((actual - expected).abs() < f64::EPSILON);
    }

    #[test]
    fn lifo_eval_decrements_before_returning_like_c() {
        let clause = Clause::empty();
        let mut evaluator = lifo_eval_init();

        assert_close(evaluator.compute(&clause), -1.0);
        assert_close(evaluator.compute(&clause), -2.0);
        assert_close(evaluator.counter(), -2.0);
    }

    #[test]
    fn lifo_eval_compute_uses_external_counter_state() {
        let clause = Clause::empty();
        let mut counter = 10.0;

        assert_close(lifo_eval_compute(&mut counter, &clause), 9.0);
        assert_close(counter, 9.0);
    }

    fn normal_priority(_bank: &TermBank, _clause: &Clause) -> EvalPriority {
        PRIO_NORMAL
    }

    fn term_bank() -> TermBank {
        TermBank::new(Signature::new(TypeBank::new())).unwrap_or_else(|err| panic!("{err}"))
    }

    #[test]
    fn lifo_wfcb_init_wraps_stateful_counter() {
        let clause = Clause::empty();
        let bank = term_bank();
        let mut wfcb = lifo_eval_wfcb_init(normal_priority);
        let mut evaluations = evals_alloc(2);

        clause_add_evaluation(&mut wfcb, &mut evaluations, &bank, &clause, 0, false);
        clause_add_evaluation(&mut wfcb, &mut evaluations, &bank, &clause, 1, false);

        assert_eq!(
            evaluations.eval(0).heuristic().to_bits(),
            (-1.0_f32).to_bits()
        );
        assert_eq!(
            evaluations.eval(1).heuristic().to_bits(),
            (-2.0_f32).to_bits()
        );
        assert_eq!(evaluations.eval(0).priority(), PRIO_NORMAL);
        assert_eq!(evaluations.eval(1).priority(), PRIO_NORMAL);
    }

    #[test]
    fn lifo_eval_parse_accepts_priority_function_in_brackets() {
        let clause = Clause::empty();
        let bank = term_bank();
        let mut scanner =
            Scanner::from_user_string("(ConstPrio) rest", false).unwrap_or_else(|err| {
                panic!("{err}");
            });
        let mut wfcb = super::lifo_eval_parse(&mut scanner).unwrap_or_else(|err| panic!("{err}"));
        let mut evaluations = evals_alloc(1);

        clause_add_evaluation(&mut wfcb, &mut evaluations, &bank, &clause, 0, false);

        assert_eq!(
            evaluations.eval(0).heuristic().to_bits(),
            (-1.0_f32).to_bits()
        );
        assert_eq!(evaluations.eval(0).priority(), PRIO_NORMAL);
        assert_eq!(scanner.current_token().literal(), "rest");
    }
}
