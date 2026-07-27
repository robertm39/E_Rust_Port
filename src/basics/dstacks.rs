pub const DSTACK_DEFAULT_SIZE: usize = 32;

pub type DStackPointer = isize;

#[derive(Clone, Debug, PartialEq)]
pub struct DStack {
    size: usize,
    stack: Vec<f64>,
}

impl Default for DStack {
    fn default() -> Self {
        Self::new()
    }
}

impl DStack {
    #[must_use]
    pub fn new() -> Self {
        Self::with_size(DSTACK_DEFAULT_SIZE)
    }

    /// Allocate an empty stack with selectable initial size.
    ///
    /// # Panics
    ///
    /// Panics when `size` is zero. The C implementation assumes a positive
    /// allocation size and its growth rule cannot recover from zero.
    #[must_use]
    pub fn with_size(size: usize) -> Self {
        assert!(size > 0, "DStack initial size must be non-zero");
        Self {
            size,
            stack: Vec::with_capacity(size),
        }
    }

    #[must_use]
    pub const fn allocated_size(&self) -> usize {
        self.size
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.stack.len()
    }

    #[must_use]
    pub fn stack_pointer(&self) -> DStackPointer {
        match DStackPointer::try_from(self.stack.len()) {
            Ok(value) => value,
            Err(_) => DStackPointer::MAX,
        }
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.stack.is_empty()
    }

    #[must_use]
    pub fn as_slice(&self) -> &[f64] {
        &self.stack
    }

    pub fn reset(&mut self) {
        self.stack.clear();
    }

    pub fn push(&mut self, value: f64) {
        if self.stack.len() == self.size {
            self.grow();
        }
        self.stack.push(value);
    }

    /// Double the logical allocation size used by the C stack.
    ///
    /// # Panics
    ///
    /// Panics if doubling the stack size would overflow `usize`.
    pub fn grow(&mut self) {
        let old_size = self.size;
        let Some(new_size) = old_size.checked_mul(2) else {
            panic!("DStack capacity overflow");
        };
        if self.stack.capacity() < new_size {
            self.stack.reserve_exact(new_size - self.stack.capacity());
        }
        self.size = new_size;
    }

    /// Pop the top value from a non-empty stack.
    ///
    /// # Panics
    ///
    /// Panics when the stack is empty, matching the C `DStackPop`
    /// assertion.
    pub fn pop(&mut self) -> f64 {
        assert!(!self.is_empty(), "DStackPop called on an empty stack");
        let Some(value) = self.stack.pop() else {
            unreachable!("DStack top slot was empty");
        };
        value
    }

    /// Return the top value from a non-empty stack.
    ///
    /// # Panics
    ///
    /// Panics when the stack is empty, matching the C `DStackTop`
    /// assertion.
    #[must_use]
    pub fn top(&self) -> f64 {
        assert!(!self.is_empty(), "DStackTop called on an empty stack");
        let Some(value) = self.stack.last().copied() else {
            unreachable!("DStack top slot was empty");
        };
        value
    }

    /// Return the second value from the top of a stack with at least two items.
    ///
    /// # Panics
    ///
    /// Panics when the stack has fewer than two values, matching the C
    /// `DStackBelowTop` assertion.
    #[must_use]
    pub fn below_top(&self) -> f64 {
        assert!(
            self.stack.len() >= 2,
            "DStackBelowTop called with fewer than two values"
        );
        self.stack[self.stack.len() - 2]
    }

    /// Return the value at a valid stack position.
    ///
    /// # Panics
    ///
    /// Panics when `pos` is negative or outside the current stack, matching the
    /// C `DStackElement` assertions.
    #[must_use]
    pub fn element(&self, pos: DStackPointer) -> f64 {
        assert!(pos >= 0, "DStackElement called with a negative index");
        let Ok(index) = usize::try_from(pos) else {
            panic!("DStack index overflow");
        };
        assert!(
            index < self.stack.len(),
            "DStackElement index out of bounds"
        );
        self.stack[index]
    }
}

#[cfg(test)]
mod tests {
    use super::{DStack, DSTACK_DEFAULT_SIZE};

    fn assert_same_f64(actual: f64, expected: f64) {
        assert_eq!(actual.to_bits(), expected.to_bits());
    }

    #[test]
    fn default_stack_starts_at_c_default_size() {
        let stack = DStack::new();
        assert_eq!(stack.allocated_size(), DSTACK_DEFAULT_SIZE);
        assert!(stack.is_empty());
    }

    #[test]
    fn push_pop_and_reset_use_c_growth_rules() {
        let mut stack = DStack::with_size(2);
        stack.push(1.25);
        stack.push(2.5);
        assert_eq!(stack.allocated_size(), 2);
        stack.push(3.75);
        assert_eq!(stack.allocated_size(), 4);
        assert_eq!(stack.stack_pointer(), 3);
        assert_same_f64(stack.top(), 3.75);
        assert_same_f64(stack.below_top(), 2.5);
        assert_same_f64(stack.element(0), 1.25);
        assert_same_f64(stack.pop(), 3.75);

        stack.reset();
        assert!(stack.is_empty());
        assert_eq!(stack.allocated_size(), 4);
    }

    #[test]
    #[should_panic(expected = "DStackPop called on an empty stack")]
    fn pop_panics_on_empty_like_c_assertion() {
        let mut stack = DStack::new();
        let _value = stack.pop();
    }

    #[test]
    #[should_panic(expected = "DStackTop called on an empty stack")]
    fn top_panics_on_empty_like_c_assertion() {
        let stack = DStack::new();
        let _value = stack.top();
    }

    #[test]
    #[should_panic(expected = "DStackBelowTop called with fewer than two values")]
    fn below_top_panics_without_two_values_like_c_assertion() {
        let mut stack = DStack::new();
        stack.push(1.0);
        let _value = stack.below_top();
    }

    #[test]
    #[should_panic(expected = "DStackElement called with a negative index")]
    fn element_panics_on_negative_index_like_c_assertion() {
        let mut stack = DStack::new();
        stack.push(1.0);
        let _value = stack.element(-1);
    }

    #[test]
    #[should_panic(expected = "DStackElement index out of bounds")]
    fn element_panics_on_out_of_bounds_index_like_c_assertion() {
        let mut stack = DStack::new();
        stack.push(1.0);
        let _value = stack.element(1);
    }
}
