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

    pub fn pop(&mut self) -> Option<f64> {
        self.stack.pop()
    }

    #[must_use]
    pub fn top(&self) -> Option<f64> {
        self.stack.last().copied()
    }

    #[must_use]
    pub fn below_top(&self) -> Option<f64> {
        self.stack.get(self.stack.len().checked_sub(2)?).copied()
    }

    #[must_use]
    pub fn element(&self, pos: DStackPointer) -> Option<f64> {
        let index = usize::try_from(pos).ok()?;
        self.stack.get(index).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::{DStack, DSTACK_DEFAULT_SIZE};

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
        assert_eq!(stack.top(), Some(3.75));
        assert_eq!(stack.below_top(), Some(2.5));
        assert_eq!(stack.element(0), Some(1.25));
        assert_eq!(stack.element(-1), None);
        assert_eq!(stack.pop(), Some(3.75));

        stack.reset();
        assert!(stack.is_empty());
        assert_eq!(stack.allocated_size(), 4);
    }
}
