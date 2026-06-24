use std::cmp::Ordering;
use std::sync::atomic::{AtomicI64, Ordering as AtomicOrdering};

pub type EvalPriority = i64;
pub type EvalObjectHandle = usize;
pub type EvalNodeHandle = usize;

pub const PRIO_BEST: EvalPriority = 0;
pub const PRIO_PREFER: EvalPriority = 30;
pub const PRIO_NORMAL: EvalPriority = 40;
pub const PRIO_DEFER: EvalPriority = 50;
pub const PRIO_LARGEST_REASONABLE: EvalPriority = 1_048_576;

static EVALUATION_COUNTER: AtomicI64 = AtomicI64::new(0);

#[derive(Clone, Debug, PartialEq)]
pub struct SimpleEvalCell {
    priority: EvalPriority,
    heuristic: f32,
    left: Option<EvalNodeHandle>,
    right: Option<EvalNodeHandle>,
}

impl Default for SimpleEvalCell {
    fn default() -> Self {
        Self::new()
    }
}

impl SimpleEvalCell {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            priority: 0,
            heuristic: 0.0,
            left: None,
            right: None,
        }
    }

    #[must_use]
    pub const fn priority(&self) -> EvalPriority {
        self.priority
    }

    pub const fn set_priority(&mut self, priority: EvalPriority) {
        self.priority = priority;
    }

    #[must_use]
    pub const fn heuristic(&self) -> f32 {
        self.heuristic
    }

    pub const fn set_heuristic(&mut self, heuristic: f32) {
        self.heuristic = heuristic;
    }

    #[allow(clippy::cast_possible_truncation)]
    pub fn set_heuristic_from_eval(&mut self, heuristic: f64) {
        self.heuristic = heuristic as f32;
    }

    #[must_use]
    pub const fn left(&self) -> Option<EvalNodeHandle> {
        self.left
    }

    pub const fn set_left(&mut self, left: Option<EvalNodeHandle>) {
        self.left = left;
    }

    #[must_use]
    pub const fn right(&self) -> Option<EvalNodeHandle> {
        self.right
    }

    pub const fn set_right(&mut self, right: Option<EvalNodeHandle>) {
        self.right = right;
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct EvalCell {
    eval_count: EvalPriority,
    object: Option<EvalObjectHandle>,
    evals: Vec<SimpleEvalCell>,
}

impl EvalCell {
    #[must_use]
    pub fn alloc(eval_no: usize) -> Self {
        let mut eval = evals_alloc_raw(eval_no);
        eval.eval_count = EVALUATION_COUNTER.fetch_add(1, AtomicOrdering::Relaxed);
        eval
    }

    #[must_use]
    pub fn eval_no(&self) -> usize {
        self.evals.len()
    }

    #[must_use]
    pub const fn eval_count(&self) -> EvalPriority {
        self.eval_count
    }

    #[must_use]
    pub const fn object(&self) -> Option<EvalObjectHandle> {
        self.object
    }

    pub const fn set_object(&mut self, object: Option<EvalObjectHandle>) {
        self.object = object;
    }

    /// Returns the simple evaluation at `pos`.
    ///
    /// # Panics
    ///
    /// Panics if `pos` is outside this cell's evaluation array, matching the
    /// unchecked C flexible-array access contract.
    #[must_use]
    pub fn eval(&self, pos: usize) -> &SimpleEvalCell {
        &self.evals[pos]
    }

    /// Returns the mutable simple evaluation at `pos`.
    ///
    /// # Panics
    ///
    /// Panics if `pos` is outside this cell's evaluation array, matching the
    /// unchecked C flexible-array access contract.
    pub fn eval_mut(&mut self, pos: usize) -> &mut SimpleEvalCell {
        &mut self.evals[pos]
    }

    pub fn set_priority(&mut self, priority: EvalPriority) {
        for eval in &mut self.evals {
            eval.set_priority(priority);
        }
    }

    pub fn change_priority(&mut self, diff: EvalPriority) {
        for eval in &mut self.evals {
            eval.set_priority(eval.priority() + diff);
        }
    }

    #[must_use]
    pub fn list_print_string(&self) -> String {
        (0..self.eval_no())
            .map(|pos| self.print_string(pos))
            .collect()
    }

    #[must_use]
    pub fn list_print_comment_string(&self) -> String {
        format!("/*{}*/", self.list_print_string())
    }

    /// Returns the C `EvalPrint` string for evaluation position `pos`.
    ///
    /// # Panics
    ///
    /// Panics if `pos` is outside this cell's evaluation array, matching the
    /// unchecked C flexible-array access contract.
    #[must_use]
    pub fn print_string(&self, pos: usize) -> String {
        let eval = self.eval(pos);
        format!(
            "[{:3}:{:.10}:{}]",
            eval.priority(),
            eval.heuristic(),
            self.eval_count()
        )
    }

    /// Returns the C `EvalPrintComment` string for evaluation position `pos`.
    ///
    /// # Panics
    ///
    /// Panics if `pos` is outside this cell's evaluation array, matching the
    /// unchecked C flexible-array access contract.
    #[must_use]
    pub fn print_comment_string(&self, pos: usize) -> String {
        format!("/*{}*/", self.print_string(pos))
    }
}

#[must_use]
pub fn evals_alloc(eval_no: usize) -> EvalCell {
    EvalCell::alloc(eval_no)
}

#[must_use]
pub fn evaluation_counter() -> EvalPriority {
    EVALUATION_COUNTER.load(AtomicOrdering::Relaxed)
}

/// Compares two evaluation cells at evaluation position `pos`.
///
/// # Panics
///
/// Panics if `pos` is outside either cell's evaluation array, matching the
/// unchecked C flexible-array access contract.
#[must_use]
pub fn eval_compare(ev1: &EvalCell, ev2: &EvalCell, pos: usize) -> EvalPriority {
    let eval1 = ev1.eval(pos);
    let eval2 = ev2.eval(pos);

    let priority_diff = eval1.priority() - eval2.priority();
    if priority_diff != 0 {
        return priority_diff;
    }

    let count_diff = ev1.eval_count() - ev2.eval_count();
    if count_diff == 0 {
        return count_diff;
    }

    let heuristic_diff = cmp_f32_c(eval1.heuristic(), eval2.heuristic());
    if heuristic_diff != 0 {
        return heuristic_diff;
    }
    count_diff
}

/// Returns whether `ev1` is greater than `ev2` at evaluation position `pos`.
///
/// # Panics
///
/// Panics if `pos` is outside either cell's evaluation array, matching the
/// unchecked C flexible-array access contract.
#[must_use]
pub fn eval_greater(ev1: &EvalCell, ev2: &EvalCell, pos: usize) -> bool {
    let eval1 = ev1.eval(pos);
    let eval2 = ev2.eval(pos);

    if eval1.priority() > eval2.priority() {
        return true;
    }
    if eval1.priority() == eval2.priority() {
        if ev1.eval_count() == ev2.eval_count() {
            return false;
        }
        if eval1.heuristic() > eval2.heuristic() {
            return true;
        }
        if c_float_eq(eval1.heuristic(), eval2.heuristic()) && ev1.eval_count() > ev2.eval_count() {
            return true;
        }
    }
    false
}

fn evals_alloc_raw(eval_no: usize) -> EvalCell {
    EvalCell {
        eval_count: 0,
        object: None,
        evals: vec![SimpleEvalCell::new(); eval_no],
    }
}

fn cmp_f32_c(left: f32, right: f32) -> EvalPriority {
    EvalPriority::from(left > right) - EvalPriority::from(left < right)
}

fn c_float_eq(left: f32, right: f32) -> bool {
    matches!(left.partial_cmp(&right), Some(Ordering::Equal))
}

#[cfg(test)]
mod tests {
    use super::{
        eval_compare, eval_greater, evals_alloc, evals_alloc_raw, evaluation_counter, EvalCell,
        PRIO_BEST, PRIO_DEFER, PRIO_LARGEST_REASONABLE, PRIO_NORMAL, PRIO_PREFER,
    };

    fn eval_with_count(eval_count: i64, priority: i64, heuristic: f32) -> EvalCell {
        let mut eval = evals_alloc_raw(1);
        eval.eval_count = eval_count;
        eval.eval_mut(0).set_priority(priority);
        eval.eval_mut(0).set_heuristic(heuristic);
        eval
    }

    #[test]
    fn priority_constants_match_c_defines() {
        assert_eq!(PRIO_BEST, 0);
        assert_eq!(PRIO_PREFER, 30);
        assert_eq!(PRIO_NORMAL, 40);
        assert_eq!(PRIO_DEFER, 50);
        assert_eq!(PRIO_LARGEST_REASONABLE, 1_048_576);
    }

    #[test]
    fn allocation_initializes_eval_cells_and_advances_global_counter() {
        let start = evaluation_counter();
        let first = evals_alloc(2);
        let second = evals_alloc(1);

        assert_eq!(first.eval_count(), start);
        assert_eq!(second.eval_count(), start + 1);
        assert_eq!(first.eval_no(), 2);
        assert_eq!(first.object(), None);
        assert_eq!(first.eval(0).priority(), 0);
        assert_eq!(first.eval(0).heuristic().to_bits(), 0.0_f32.to_bits());
        assert_eq!(first.eval(0).left(), None);
        assert_eq!(first.eval(0).right(), None);
    }

    #[test]
    fn object_and_tree_link_slots_preserve_c_cell_shape() {
        let mut eval = evals_alloc_raw(1);

        eval.set_object(Some(17));
        eval.eval_mut(0).set_left(Some(3));
        eval.eval_mut(0).set_right(Some(4));

        assert_eq!(eval.object(), Some(17));
        assert_eq!(eval.eval(0).left(), Some(3));
        assert_eq!(eval.eval(0).right(), Some(4));
    }

    #[test]
    fn heuristic_assignment_from_eval_truncates_to_c_float_storage() {
        let mut eval = evals_alloc_raw(1);
        let value = 1.0_f64 / 3.0;

        eval.eval_mut(0).set_heuristic_from_eval(value);

        assert_eq!(
            eval.eval(0).heuristic().to_bits(),
            0.333_333_34_f32.to_bits()
        );
    }

    #[test]
    fn printing_matches_c_eval_formats() {
        let mut eval = evals_alloc_raw(2);
        eval.eval_count = 7;
        eval.eval_mut(0).set_priority(PRIO_PREFER);
        eval.eval_mut(0).set_heuristic(2.5);
        eval.eval_mut(1).set_priority(PRIO_DEFER);
        eval.eval_mut(1).set_heuristic(-1.25);

        assert_eq!(eval.print_string(0), "[ 30:2.5000000000:7]");
        assert_eq!(eval.print_comment_string(0), "/*[ 30:2.5000000000:7]*/");
        assert_eq!(
            eval.list_print_string(),
            "[ 30:2.5000000000:7][ 50:-1.2500000000:7]"
        );
        assert_eq!(
            eval.list_print_comment_string(),
            "/*[ 30:2.5000000000:7][ 50:-1.2500000000:7]*/"
        );
    }

    #[test]
    fn priority_mutators_touch_all_simple_evals() {
        let mut eval = evals_alloc_raw(3);

        eval.set_priority(PRIO_NORMAL);
        eval.change_priority(5);

        assert_eq!(eval.eval(0).priority(), 45);
        assert_eq!(eval.eval(1).priority(), 45);
        assert_eq!(eval.eval(2).priority(), 45);
    }

    #[test]
    fn compare_uses_priority_then_count_and_heuristic_like_c() {
        let lower_priority = eval_with_count(1, PRIO_PREFER, 100.0);
        let higher_priority = eval_with_count(2, PRIO_NORMAL, 1.0);
        assert_eq!(eval_compare(&lower_priority, &higher_priority, 0), -10);
        assert_eq!(eval_compare(&higher_priority, &lower_priority, 0), 10);

        let better_heuristic = eval_with_count(1, PRIO_NORMAL, 10.0);
        let worse_heuristic = eval_with_count(2, PRIO_NORMAL, 5.0);
        assert_eq!(eval_compare(&better_heuristic, &worse_heuristic, 0), 1);
        assert_eq!(eval_compare(&worse_heuristic, &better_heuristic, 0), -1);

        let older = eval_with_count(1, PRIO_NORMAL, 5.0);
        let newer = eval_with_count(2, PRIO_NORMAL, 5.0);
        assert_eq!(eval_compare(&older, &newer, 0), -1);
        assert_eq!(eval_compare(&newer, &older, 0), 1);

        let same_count_left = eval_with_count(3, PRIO_NORMAL, 100.0);
        let same_count_right = eval_with_count(3, PRIO_NORMAL, 1.0);
        assert_eq!(eval_compare(&same_count_left, &same_count_right, 0), 0);
    }

    #[test]
    fn compare_treats_nan_heuristics_like_c_cmp_macro() {
        let left = eval_with_count(1, PRIO_NORMAL, f32::NAN);
        let right = eval_with_count(2, PRIO_NORMAL, 5.0);

        assert_eq!(eval_compare(&left, &right, 0), -1);
        assert!(!eval_greater(&left, &right, 0));
    }

    #[test]
    fn eval_greater_matches_c_branch_order() {
        let high_priority = eval_with_count(1, PRIO_DEFER, 1.0);
        let low_priority = eval_with_count(2, PRIO_NORMAL, 100.0);
        assert!(eval_greater(&high_priority, &low_priority, 0));

        let better_heuristic = eval_with_count(1, PRIO_NORMAL, 10.0);
        let worse_heuristic = eval_with_count(2, PRIO_NORMAL, 5.0);
        assert!(eval_greater(&better_heuristic, &worse_heuristic, 0));

        let newer = eval_with_count(3, PRIO_NORMAL, 5.0);
        let older = eval_with_count(2, PRIO_NORMAL, 5.0);
        assert!(eval_greater(&newer, &older, 0));

        let same_count_left = eval_with_count(4, PRIO_NORMAL, 100.0);
        let same_count_right = eval_with_count(4, PRIO_NORMAL, 1.0);
        assert!(!eval_greater(&same_count_left, &same_count_right, 0));
    }
}
