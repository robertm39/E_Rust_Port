use crate::clauses::clause::Clause;

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

#[cfg(test)]
mod tests {
    use super::{lifo_eval_compute, lifo_eval_init};
    use crate::clauses::clause::Clause;

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
}
