pub const PLOCALSTACK_DEFAULT_SIZE: usize = 64;
pub const PLOCALSTACK_TAG_BITS: usize = 2;
pub const PLOCALSTACK_TAG_MASK: usize = (1_usize << PLOCALSTACK_TAG_BITS) - 1;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PLocalStack<T> {
    size: usize,
    data: Vec<T>,
}

impl<T> Default for PLocalStack<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> PLocalStack<T> {
    #[must_use]
    pub fn new() -> Self {
        Self::with_size(PLOCALSTACK_DEFAULT_SIZE)
    }

    /// Allocate a local stack with selectable initial slot count.
    ///
    /// # Panics
    ///
    /// Panics when `size` is zero. The C macros allocate a positive number of
    /// pointer slots and cannot grow coherently from zero.
    #[must_use]
    pub fn with_size(size: usize) -> Self {
        assert!(size > 0, "PLocalStack initial size must be non-zero");
        Self {
            size,
            data: Vec::with_capacity(size),
        }
    }

    #[must_use]
    pub const fn allocated_size(&self) -> usize {
        self.size
    }

    #[must_use]
    pub fn current(&self) -> usize {
        self.data.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    #[must_use]
    pub fn as_slice(&self) -> &[T] {
        &self.data
    }

    /// Grow when `current + space >= size`, matching `PLocalStackEnsureSpace`.
    ///
    /// # Panics
    ///
    /// Panics if computing the requested size or doubled allocation size would
    /// overflow `usize`.
    pub fn ensure_space(&mut self, space: usize) {
        let should_grow = match self.data.len().checked_add(space) {
            Some(required) => required >= self.size,
            None => true,
        };
        if should_grow {
            self.grow(space);
        }
    }

    /// Push without growing, matching the C macro's caller-managed contract.
    pub fn push(&mut self, value: T) -> bool {
        if self.data.len() >= self.size {
            return false;
        }
        self.data.push(value);
        true
    }

    /// Ensure room for one slot, then push.
    ///
    /// # Panics
    ///
    /// Panics if stack growth would overflow `usize`.
    pub fn push_growing(&mut self, value: T) {
        self.ensure_space(1);
        let pushed = self.push(value);
        debug_assert!(pushed);
    }

    pub fn pop(&mut self) -> Option<T> {
        self.data.pop()
    }

    pub fn push_slice(&mut self, values: &[T])
    where
        T: Clone,
    {
        self.ensure_space(values.len());
        for value in values {
            let pushed = self.push(value.clone());
            debug_assert!(pushed);
        }
    }

    pub fn push_slice_reversed(&mut self, values: &[T])
    where
        T: Clone,
    {
        self.ensure_space(values.len());
        for value in values.iter().rev() {
            let pushed = self.push(value.clone());
            debug_assert!(pushed);
        }
    }

    fn grow(&mut self, space: usize) {
        let old_size = self.size;
        let Some(required) = old_size.checked_add(space) else {
            panic!("PLocalStack capacity overflow");
        };
        let mut new_size = old_size;
        while new_size <= required {
            let Some(doubled) = new_size.checked_mul(2) else {
                panic!("PLocalStack capacity overflow");
            };
            new_size = doubled;
        }
        if self.data.capacity() < new_size {
            self.data.reserve_exact(new_size - self.data.capacity());
        }
        self.size = new_size;
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PLocalTaggedStack<T, Tag> {
    size: usize,
    data: Vec<(T, Tag)>,
}

impl<T, Tag> Default for PLocalTaggedStack<T, Tag> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T, Tag> PLocalTaggedStack<T, Tag> {
    #[must_use]
    pub fn new() -> Self {
        Self::with_size(PLOCALSTACK_DEFAULT_SIZE)
    }

    /// Allocate a tagged stack with a C pointer-slot capacity.
    ///
    /// # Panics
    ///
    /// Panics when `size` is zero.
    #[must_use]
    pub fn with_size(size: usize) -> Self {
        assert!(size > 0, "PLocalTaggedStack initial size must be non-zero");
        Self {
            size,
            data: Vec::with_capacity(size / 2),
        }
    }

    #[must_use]
    pub const fn allocated_slots(&self) -> usize {
        self.size
    }

    #[must_use]
    pub fn current_slots(&self) -> usize {
        self.data.len().saturating_mul(2)
    }

    #[must_use]
    pub fn current_entries(&self) -> usize {
        self.data.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Grow using the non-`TAGGED_POINTERS` C slot accounting.
    ///
    /// # Panics
    ///
    /// Panics if converting requested entries into pointer slots or growing the
    /// stack would overflow `usize`.
    pub fn ensure_space(&mut self, entries: usize) {
        let Some(slot_space) = entries.checked_mul(2) else {
            panic!("PLocalTaggedStack capacity overflow");
        };
        let should_grow = match self.current_slots().checked_add(slot_space) {
            Some(required) => required >= self.size,
            None => true,
        };
        if should_grow {
            self.grow_slots(slot_space);
        }
    }

    /// Push one tagged value without growing.
    pub fn push(&mut self, value: T, tag: Tag) -> bool {
        let Some(required_slots) = self.current_slots().checked_add(2) else {
            return false;
        };
        if required_slots > self.size {
            return false;
        }
        self.data.push((value, tag));
        true
    }

    /// Ensure room for one tagged entry, then push.
    ///
    /// # Panics
    ///
    /// Panics if stack growth would overflow `usize`.
    pub fn push_growing(&mut self, value: T, tag: Tag) {
        self.ensure_space(1);
        let pushed = self.push(value, tag);
        debug_assert!(pushed);
    }

    pub fn pop(&mut self) -> Option<(T, Tag)> {
        self.data.pop()
    }

    pub fn push_slice(&mut self, values: &[T], tag: Tag)
    where
        T: Clone,
        Tag: Clone,
    {
        self.ensure_space(values.len());
        for value in values {
            let pushed = self.push(value.clone(), tag.clone());
            debug_assert!(pushed);
        }
    }

    pub fn push_slice_reversed(&mut self, values: &[T], tag: Tag)
    where
        T: Clone,
        Tag: Clone,
    {
        self.ensure_space(values.len());
        for value in values.iter().rev() {
            let pushed = self.push(value.clone(), tag.clone());
            debug_assert!(pushed);
        }
    }

    fn grow_slots(&mut self, slot_space: usize) {
        let old_size = self.size;
        let Some(required) = old_size.checked_add(slot_space) else {
            panic!("PLocalTaggedStack capacity overflow");
        };
        let mut new_size = old_size;
        while new_size <= required {
            let Some(doubled) = new_size.checked_mul(2) else {
                panic!("PLocalTaggedStack capacity overflow");
            };
            new_size = doubled;
        }
        let new_entry_capacity = new_size / 2;
        if self.data.capacity() < new_entry_capacity {
            self.data
                .reserve_exact(new_entry_capacity - self.data.capacity());
        }
        self.size = new_size;
    }
}

#[cfg(test)]
mod tests {
    use super::{PLocalStack, PLocalTaggedStack, PLOCALSTACK_DEFAULT_SIZE};

    #[test]
    fn default_stack_starts_empty_with_c_default_size() {
        let stack = PLocalStack::<usize>::new();
        assert_eq!(stack.allocated_size(), PLOCALSTACK_DEFAULT_SIZE);
        assert!(stack.is_empty());
        assert_eq!(stack.current(), 0);
    }

    #[test]
    fn push_does_not_grow_but_ensure_space_uses_c_equality_rule() {
        let mut stack = PLocalStack::with_size(2);

        assert!(stack.push(1));
        assert!(stack.push(2));
        assert!(!stack.push(3));
        assert_eq!(stack.allocated_size(), 2);

        stack.ensure_space(1);
        assert_eq!(stack.allocated_size(), 4);
        assert!(stack.push(3));
        assert_eq!(stack.as_slice(), &[1, 2, 3]);
    }

    #[test]
    fn ensure_space_grows_against_old_size_plus_space() {
        let mut stack = PLocalStack::<usize>::with_size(4);
        stack.ensure_space(4);
        assert_eq!(stack.allocated_size(), 16);
    }

    #[test]
    fn pop_and_push_growing_are_lifo() {
        let mut stack = PLocalStack::with_size(2);
        stack.push_growing("a");
        stack.push_growing("b");
        stack.push_growing("c");

        assert_eq!(stack.allocated_size(), 4);
        assert_eq!(stack.pop(), Some("c"));
        assert_eq!(stack.pop(), Some("b"));
        assert_eq!(stack.pop(), Some("a"));
        assert_eq!(stack.pop(), None);
    }

    #[test]
    fn slice_helpers_match_term_argument_push_order() {
        let args = [1, 2, 3];
        let mut forward = PLocalStack::with_size(2);
        forward.push_slice(&args);
        assert_eq!(forward.as_slice(), &[1, 2, 3]);

        let mut reversed = PLocalStack::with_size(2);
        reversed.push_slice_reversed(&args);
        assert_eq!(reversed.as_slice(), &[3, 2, 1]);
        assert_eq!(reversed.pop(), Some(1));
    }

    #[test]
    fn tagged_stack_uses_two_slot_non_tagged_pointer_accounting() {
        let mut stack = PLocalTaggedStack::with_size(4);

        stack.push_growing("term-a", 1_u8);
        assert_eq!(stack.allocated_slots(), 4);
        assert_eq!(stack.current_slots(), 2);

        stack.push_growing("term-b", 2_u8);
        assert_eq!(stack.allocated_slots(), 8);
        assert_eq!(stack.current_entries(), 2);
        assert_eq!(stack.pop(), Some(("term-b", 2)));
        assert_eq!(stack.pop(), Some(("term-a", 1)));
        assert_eq!(stack.pop(), None);
    }

    #[test]
    fn tagged_slice_helpers_preserve_tags_and_order() {
        let args = ["x", "y", "z"];
        let mut stack = PLocalTaggedStack::with_size(4);
        stack.push_slice_reversed(&args, 9_u8);

        assert_eq!(stack.current_entries(), 3);
        assert_eq!(stack.pop(), Some(("x", 9)));
        assert_eq!(stack.pop(), Some(("y", 9)));
        assert_eq!(stack.pop(), Some(("z", 9)));
    }
}
