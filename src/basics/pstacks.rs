use std::{cmp::Ordering, fmt, mem};

pub const PSTACK_DEFAULT_SIZE: usize = 128;
const PSTACK_AVG_ENTRIES: usize = 6;

pub type PStackPointer = isize;
pub type PStackInt = i64;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PStack<T> {
    size: usize,
    stack: Vec<T>,
}

impl<T> Default for PStack<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> PStack<T> {
    #[must_use]
    pub fn new() -> Self {
        Self::with_size(PSTACK_DEFAULT_SIZE)
    }

    /// Allocate an empty stack with selectable initial size.
    ///
    /// C stores pointer-sized union elements. Rust keeps the same logical
    /// growth boundary while sizing the initial vector to the equivalent byte
    /// count when `T` is wider than that union.
    ///
    /// # Panics
    ///
    /// Panics when `size` is zero. The C implementation assumes a positive
    /// allocation size and its growth rule cannot recover from zero.
    #[must_use]
    pub fn with_size(size: usize) -> Self {
        assert!(size > 0, "PStack initial size must be non-zero");
        Self::with_size_and_capacity(size, Self::initial_capacity(size))
    }

    /// Allocate a default-sized stack using the average occupancy assumed by
    /// the C aggregate-memory estimates.
    #[must_use]
    pub(crate) fn with_average_capacity() -> Self {
        Self::with_size_and_capacity(PSTACK_DEFAULT_SIZE, PSTACK_AVG_ENTRIES)
    }

    fn with_size_and_capacity(size: usize, capacity: usize) -> Self {
        assert!(size > 0, "PStack initial size must be non-zero");
        assert!(
            capacity > 0 && capacity <= size,
            "PStack physical capacity must be within its logical size"
        );
        Self {
            size,
            stack: Vec::with_capacity(capacity),
        }
    }

    fn initial_capacity(size: usize) -> usize {
        let element_size = mem::size_of::<T>();
        if element_size == 0 {
            return size;
        }

        let c_bytes = size.saturating_mul(mem::size_of::<usize>());
        (c_bytes / element_size).clamp(1, size)
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
    pub fn stack_pointer(&self) -> PStackPointer {
        match PStackPointer::try_from(self.stack.len()) {
            Ok(value) => value,
            Err(_) => PStackPointer::MAX,
        }
    }

    #[must_use]
    pub fn top_stack_pointer(&self) -> PStackPointer {
        self.stack_pointer() - 1
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.stack.is_empty()
    }

    #[must_use]
    pub fn as_slice(&self) -> &[T] {
        &self.stack
    }

    pub fn as_mut_slice(&mut self) -> &mut [T] {
        &mut self.stack
    }

    pub fn reset(&mut self) {
        self.stack.clear();
    }

    pub fn push(&mut self, value: T) {
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
            panic!("PStack capacity overflow");
        };
        if self.stack.capacity() < new_size {
            self.stack.reserve_exact(new_size - self.stack.capacity());
        }
        self.size = new_size;
    }

    pub fn pop(&mut self) -> Option<T> {
        self.stack.pop()
    }

    /// Pop the top stack element.
    ///
    /// # Panics
    ///
    /// Panics when the stack is empty, matching the C `PStackPop`
    /// assertion.
    pub fn pop_nonempty(&mut self) -> T {
        assert!(!self.is_empty(), "PStackPop called on an empty stack");
        self.stack
            .pop()
            .unwrap_or_else(|| panic!("PStackPop lost non-empty top element"))
    }

    /// Discard the top stack element.
    ///
    /// # Panics
    ///
    /// Panics when the stack is empty, matching the C `PStackDiscardTop`
    /// assertion.
    pub fn discard_top(&mut self) {
        assert!(
            !self.is_empty(),
            "PStackDiscardTop called on an empty stack"
        );
        let _discarded = self.stack.pop();
    }

    /// Return the top stack element.
    ///
    /// # Panics
    ///
    /// Panics when the stack is empty, matching the C `PStackTop`
    /// assertion.
    #[must_use]
    pub fn top(&self) -> &T {
        assert!(!self.is_empty(), "PStackTop called on an empty stack");
        self.stack
            .last()
            .unwrap_or_else(|| panic!("PStackTop lost non-empty top element"))
    }

    /// Return a mutable reference to the top stack element.
    ///
    /// # Panics
    ///
    /// Panics when the stack is empty, matching the C `PStackTopAddr`
    /// assertion.
    pub fn top_mut(&mut self) -> &mut T {
        assert!(!self.is_empty(), "PStackTopAddr called on an empty stack");
        self.stack
            .last_mut()
            .unwrap_or_else(|| panic!("PStackTopAddr lost non-empty top element"))
    }

    /// Return the second item on the stack.
    ///
    /// # Panics
    ///
    /// Panics when the stack has fewer than two elements, matching the C
    /// `PStackBelowTop` assertion.
    #[must_use]
    pub fn below_top(&self) -> &T {
        assert!(
            self.stack.len() >= 2,
            "PStackBelowTop called with fewer than two elements"
        );
        &self.stack[self.stack.len() - 2]
    }

    /// Return the element at stack position `pos`.
    ///
    /// # Panics
    ///
    /// Panics when `pos` is negative or outside the current stack, matching
    /// the C `PStackElement` assertion.
    #[must_use]
    pub fn element(&self, pos: PStackPointer) -> &T {
        let index = self.element_index_or_panic(pos, "PStackElement");
        &self.stack[index]
    }

    /// Return a mutable reference to the element at stack position `pos`.
    ///
    /// # Panics
    ///
    /// Panics when `pos` is negative or outside the current stack, matching
    /// the C `PStackElementRef` assertion.
    pub fn element_mut(&mut self, pos: PStackPointer) -> &mut T {
        let index = self.element_index_or_panic(pos, "PStackElementRef");
        &mut self.stack[index]
    }

    /// Assign `value` at stack position `pos`.
    ///
    /// # Panics
    ///
    /// Panics when `pos` is negative or outside the current stack, matching
    /// the C `PStackAssign*` macro precondition.
    pub fn assign(&mut self, pos: PStackPointer, value: T) {
        let element = self.element_mut(pos);
        *element = value;
    }

    #[must_use]
    pub fn contains_value(&self, value: &T) -> bool
    where
        T: PartialEq,
    {
        self.stack.iter().any(|element| element == value)
    }

    /// Remove stack position `pos` by swapping in the current top element.
    ///
    /// # Panics
    ///
    /// Panics when `pos` is negative or outside the current stack, matching
    /// the C `PStackDiscardElement` assertion.
    pub fn discard_element(&mut self, pos: PStackPointer) -> T {
        let index = self.element_index_or_panic(pos, "PStackDiscardElement");
        self.stack.swap_remove(index)
    }

    pub fn sort_by<F>(&mut self, mut compare: F)
    where
        F: FnMut(&T, &T) -> Ordering,
    {
        self.stack
            .sort_unstable_by(|left, right| compare(left, right));
    }

    pub fn bin_search_by_key<K, F>(
        &self,
        key: &K,
        mut lower: PStackPointer,
        mut upper: PStackPointer,
        mut compare: F,
    ) -> PStackPointer
    where
        F: FnMut(&K, &T) -> Ordering,
    {
        while lower < upper {
            let index = isize::midpoint(lower, upper);
            let element = self.element(index);
            match compare(key, element) {
                Ordering::Less => upper = index - 1,
                Ordering::Greater => lower = index + 1,
                Ordering::Equal => return index,
            }
        }
        lower + 1
    }

    pub fn merge<F>(st1: &mut Self, st2: &mut Self, res: &mut Self, mut compare: F)
    where
        F: FnMut(&T, &T) -> Ordering,
    {
        while !st1.is_empty() || !st2.is_empty() {
            let Some(candidate) = next_merge_candidate(st1, st2, &mut compare) else {
                break;
            };
            let duplicate = !res.is_empty() && compare(res.top(), &candidate) == Ordering::Equal;
            if !duplicate {
                res.push(candidate);
            }
        }
    }

    pub fn push_stack(&mut self, source: &Self)
    where
        T: Clone,
    {
        for value in source.as_slice() {
            self.push(value.clone());
        }
    }

    pub fn write_elements_with<W, F>(&self, output: &mut W, mut render: F) -> fmt::Result
    where
        W: fmt::Write + ?Sized,
        F: FnMut(&mut W, &T) -> fmt::Result,
    {
        for value in &self.stack {
            render(output, value)?;
        }
        Ok(())
    }

    #[must_use]
    pub fn copy_stack(&self) -> Self
    where
        T: Clone,
    {
        let mut copy = Self::new();
        copy.push_stack(self);
        copy
    }

    fn element_index(&self, pos: PStackPointer) -> Option<usize> {
        let index = usize::try_from(pos).ok()?;
        (index < self.stack.len()).then_some(index)
    }

    fn element_index_or_panic(&self, pos: PStackPointer, caller: &str) -> usize {
        self.element_index(pos)
            .unwrap_or_else(|| panic!("{caller} called with invalid position {pos}"))
    }
}

impl PStack<PStackInt> {
    #[must_use]
    pub fn find_int(&self, value: PStackInt) -> bool {
        self.contains_value(&value)
    }

    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn compute_average(&self) -> (f64, f64) {
        let count = self.stack.len();
        if count == 0 {
            return (0.0, 0.0);
        }

        let sum = self.stack.iter().map(|value| *value as f64).sum::<f64>();
        let average = sum / count as f64;
        let variance = self
            .stack
            .iter()
            .map(|value| {
                let delta = *value as f64 - average;
                delta * delta
            })
            .sum::<f64>()
            / count as f64;

        (average, variance.sqrt())
    }

    pub fn write_ints_c_width4_dot<W>(&self, output: &mut W) -> fmt::Result
    where
        W: fmt::Write + ?Sized,
    {
        self.write_elements_with(output, |output, value| {
            let value = *value;
            write!(output, "{value:4}.")
        })
    }

    #[must_use]
    pub fn format_ints_c_width4_dot(&self) -> String {
        let mut output = String::new();
        match self.write_ints_c_width4_dot(&mut output) {
            Ok(()) => output,
            Err(error) => unreachable!("writing to String failed: {error}"),
        }
    }
}

impl<T> PStack<T>
where
    T: Copy + fmt::Pointer,
{
    pub fn write_pointers_c_percent_p<W>(&self, output: &mut W) -> fmt::Result
    where
        W: fmt::Write + ?Sized,
    {
        self.write_elements_with(output, |output, value| {
            let pointer = *value;
            write!(output, "{pointer:p}")
        })
    }

    #[must_use]
    pub fn format_pointers_c_percent_p(&self) -> String {
        let mut output = String::new();
        match self.write_pointers_c_percent_p(&mut output) {
            Ok(()) => output,
            Err(error) => unreachable!("writing to String failed: {error}"),
        }
    }
}

impl<T> PStack<&T> {
    #[must_use]
    pub fn find_pointer(&self, value: &T) -> bool {
        self.stack
            .iter()
            .any(|element| std::ptr::eq(*element, value))
    }
}

fn next_merge_candidate<T, F>(
    st1: &mut PStack<T>,
    st2: &mut PStack<T>,
    compare: &mut F,
) -> Option<T>
where
    F: FnMut(&T, &T) -> Ordering,
{
    if st1.is_empty() {
        st2.pop()
    } else if st2.is_empty() {
        st1.pop()
    } else {
        let take_first = compare(st1.top(), st2.top()) == Ordering::Less;
        if take_first {
            st1.pop()
        } else {
            st2.pop()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{PStack, PStackInt, PSTACK_AVG_ENTRIES, PSTACK_DEFAULT_SIZE};
    use std::fmt::Write as _;

    #[test]
    fn push_pop_and_reset_use_c_growth_rules() {
        let mut stack = PStack::with_size(2);
        assert_eq!(stack.allocated_size(), 2);
        assert!(stack.is_empty());

        stack.push(10);
        stack.push(20);
        assert_eq!(stack.allocated_size(), 2);
        stack.push(30);
        assert_eq!(stack.allocated_size(), 4);
        assert_eq!(stack.len(), 3);
        assert_eq!(stack.stack_pointer(), 3);
        assert_eq!(stack.top_stack_pointer(), 2);
        assert_eq!(stack.top(), &30);
        assert_eq!(stack.below_top(), &20);
        assert_eq!(stack.pop(), Some(30));
        assert_eq!(stack.pop_nonempty(), 20);

        stack.reset();
        assert!(stack.is_empty());
        assert_eq!(stack.allocated_size(), 4);
        assert_eq!(stack.top_stack_pointer(), -1);
    }

    #[test]
    fn default_stack_starts_at_c_default_size() {
        let stack = PStack::<usize>::new();
        assert_eq!(stack.allocated_size(), PSTACK_DEFAULT_SIZE);
    }

    #[test]
    fn wide_typed_stack_matches_c_initial_allocation_bytes() {
        let stack = PStack::<[usize; 4]>::new();
        assert_eq!(stack.allocated_size(), PSTACK_DEFAULT_SIZE);
        assert_eq!(stack.stack.capacity(), PSTACK_DEFAULT_SIZE / 4);
    }

    #[test]
    fn average_capacity_stack_keeps_c_logical_growth_boundary() {
        let mut stack = PStack::<[usize; 4]>::with_average_capacity();
        assert_eq!(stack.allocated_size(), PSTACK_DEFAULT_SIZE);
        assert_eq!(stack.stack.capacity(), PSTACK_AVG_ENTRIES);

        for value in 0..=PSTACK_AVG_ENTRIES {
            stack.push([value; 4]);
        }

        assert_eq!(stack.len(), PSTACK_AVG_ENTRIES + 1);
        assert_eq!(stack.allocated_size(), PSTACK_DEFAULT_SIZE);
    }

    #[test]
    fn element_assignment_and_discard_match_c_index_behavior() {
        let mut stack = PStack::with_size(4);
        for value in [1, 2, 3, 4] {
            stack.push(value);
        }

        assert_eq!(stack.element(2), &3);
        stack.assign(2, 30);
        assert_eq!(stack.element(2), &30);
        assert!(stack.contains_value(&30));
        assert!(!stack.contains_value(&3));

        assert_eq!(stack.discard_element(1), 2);
        assert_eq!(stack.as_slice(), &[1, 4, 30]);
    }

    #[test]
    #[should_panic(expected = "PStackTop called on an empty stack")]
    fn top_panics_on_empty_like_c_assertion() {
        let stack = PStack::<usize>::new();
        let _value = stack.top();
    }

    #[test]
    #[should_panic(expected = "PStackTopAddr called on an empty stack")]
    fn top_mut_panics_on_empty_like_c_assertion() {
        let mut stack = PStack::<usize>::new();
        let _value = stack.top_mut();
    }

    #[test]
    #[should_panic(expected = "PStackDiscardTop called on an empty stack")]
    fn discard_top_panics_on_empty_like_c_assertion() {
        let mut stack = PStack::<usize>::new();
        stack.discard_top();
    }

    #[test]
    #[should_panic(expected = "PStackPop called on an empty stack")]
    fn pop_nonempty_panics_on_empty_like_c_assertion() {
        let mut stack = PStack::<usize>::new();
        let _value = stack.pop_nonempty();
    }

    #[test]
    #[should_panic(expected = "PStackBelowTop called with fewer than two elements")]
    fn below_top_panics_with_fewer_than_two_elements_like_c_assertion() {
        let mut stack = PStack::new();
        stack.push(1);
        let _value = stack.below_top();
    }

    #[test]
    #[should_panic(expected = "PStackElement called with invalid position -1")]
    fn element_panics_on_negative_position_like_c_assertion() {
        let mut stack = PStack::new();
        stack.push(1);
        let _value = stack.element(-1);
    }

    #[test]
    #[should_panic(expected = "PStackElementRef called with invalid position 1")]
    fn element_mut_panics_on_position_at_stack_pointer_like_c_assertion() {
        let mut stack = PStack::new();
        stack.push(1);
        let _value = stack.element_mut(1);
    }

    #[test]
    #[should_panic(expected = "PStackDiscardElement called with invalid position 2")]
    fn discard_element_panics_on_position_at_stack_pointer_like_c_assertion() {
        let mut stack = PStack::new();
        stack.push(1);
        stack.push(2);
        let _value = stack.discard_element(2);
    }

    #[test]
    fn copy_stack_uses_default_capacity_and_pushes_in_order() {
        let mut stack = PStack::with_size(2);
        stack.push(1);
        stack.push(2);
        stack.push(3);
        assert_eq!(stack.allocated_size(), 4);

        let copy = stack.copy_stack();
        assert_eq!(copy.as_slice(), &[1, 2, 3]);
        assert_eq!(copy.allocated_size(), PSTACK_DEFAULT_SIZE);
    }

    #[test]
    fn sort_bin_search_and_push_stack_preserve_c_shapes() {
        let mut stack = PStack::new();
        for value in [30, 10, 20] {
            stack.push(value);
        }
        stack.sort_by(Ord::cmp);
        assert_eq!(stack.as_slice(), &[10, 20, 30]);

        let compare = |key: &i32, value: &i32| key.cmp(value);
        assert_eq!(stack.bin_search_by_key(&20, 0, 3, compare), 1);
        assert_eq!(stack.bin_search_by_key(&15, 0, 3, compare), 1);
        assert_eq!(stack.bin_search_by_key(&25, 0, 3, compare), 3);

        let mut target = PStack::with_size(1);
        target.push_stack(&stack);
        assert_eq!(target.as_slice(), &[10, 20, 30]);
        assert_eq!(target.allocated_size(), 4);
    }

    #[test]
    fn merge_consumes_inputs_from_their_tops_and_drops_duplicates() {
        let mut left = PStack::new();
        let mut right = PStack::new();
        let mut result = PStack::new();
        for value in [5, 3, 1] {
            left.push(value);
        }
        for value in [4, 3, 2] {
            right.push(value);
        }

        PStack::merge(&mut left, &mut right, &mut result, Ord::cmp);
        assert!(left.is_empty());
        assert!(right.is_empty());
        assert_eq!(result.as_slice(), &[1, 2, 3, 4, 5]);
    }

    #[test]
    fn integer_average_and_deviation_match_population_formula() {
        let mut stack = PStack::<PStackInt>::new();
        assert_eq!(stack.compute_average(), (0.0, 0.0));
        for value in [1, 2, 3] {
            stack.push(value);
        }
        assert!(stack.find_int(2));
        assert!(!stack.find_int(4));

        let (average, deviation) = stack.compute_average();
        assert!((average - 2.0).abs() < f64::EPSILON);
        assert!((deviation - (2.0_f64 / 3.0).sqrt()).abs() < f64::EPSILON);
        assert_eq!(stack.format_ints_c_width4_dot(), "   1.   2.   3.");
    }

    #[test]
    fn find_pointer_uses_c_pointer_identity_not_structural_equality() {
        let first = Box::new(String::from("same"));
        let second = Box::new(String::from("same"));
        let mut stack = PStack::new();
        stack.push(first.as_ref());

        assert!(stack.find_pointer(first.as_ref()));
        assert!(!stack.find_pointer(second.as_ref()));
        assert!(stack.contains_value(&second.as_ref()));

        let mut output = String::new();
        let print_result =
            stack.write_elements_with(&mut output, |output, value| write!(output, "{:p};", *value));
        assert!(print_result.is_ok());
        assert_eq!(output, format!("{:p};", first.as_ref()));
        assert_eq!(
            stack.format_pointers_c_percent_p(),
            format!("{:p}", first.as_ref())
        );
    }

    #[test]
    fn raw_pointer_printing_keeps_c_percent_p_shape_without_dereferencing() {
        let value = 7_i32;
        let value_ptr = std::ptr::from_ref(&value);
        let null_ptr = std::ptr::null::<i32>();
        let mut stack = PStack::new();
        stack.push(value_ptr);
        stack.push(null_ptr);

        assert_eq!(
            stack.format_pointers_c_percent_p(),
            format!("{value_ptr:p}{null_ptr:p}")
        );
    }

    #[test]
    fn merge_prefers_second_stack_when_tops_compare_equal() {
        let mut left = PStack::new();
        let mut right = PStack::new();
        let mut result = PStack::new();
        left.push((1, "left"));
        right.push((1, "right"));

        PStack::merge(&mut left, &mut right, &mut result, |left, right| {
            left.0.cmp(&right.0)
        });
        assert_eq!(result.as_slice(), &[(1, "right")]);
        assert!(left.is_empty());
        assert!(right.is_empty());
    }
}
