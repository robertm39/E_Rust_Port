use std::cmp::Ordering;

pub const PSTACK_DEFAULT_SIZE: usize = 128;

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
    /// # Panics
    ///
    /// Panics when `size` is zero. The C implementation assumes a positive
    /// allocation size and its growth rule cannot recover from zero.
    #[must_use]
    pub fn with_size(size: usize) -> Self {
        assert!(size > 0, "PStack initial size must be non-zero");
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
    pub fn stack_pointer(&self) -> PStackPointer {
        match PStackPointer::try_from(self.stack.len()) {
            Ok(value) => value,
            Err(_) => PStackPointer::MAX,
        }
    }

    #[must_use]
    pub fn top_stack_pointer(&self) -> Option<PStackPointer> {
        self.stack_pointer().checked_sub(1)
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

    pub fn discard_top(&mut self) -> bool {
        self.stack.pop().is_some()
    }

    #[must_use]
    pub fn top(&self) -> Option<&T> {
        self.stack.last()
    }

    pub fn top_mut(&mut self) -> Option<&mut T> {
        self.stack.last_mut()
    }

    #[must_use]
    pub fn below_top(&self) -> Option<&T> {
        self.stack.get(self.stack.len().checked_sub(2)?)
    }

    #[must_use]
    pub fn element(&self, pos: PStackPointer) -> Option<&T> {
        self.element_index(pos)
            .and_then(|index| self.stack.get(index))
    }

    pub fn element_mut(&mut self, pos: PStackPointer) -> Option<&mut T> {
        self.element_index(pos)
            .and_then(|index| self.stack.get_mut(index))
    }

    pub fn assign(&mut self, pos: PStackPointer, value: T) -> bool {
        let Some(element) = self.element_mut(pos) else {
            return false;
        };
        *element = value;
        true
    }

    #[must_use]
    pub fn contains_value(&self, value: &T) -> bool
    where
        T: PartialEq,
    {
        self.stack.iter().any(|element| element == value)
    }

    pub fn discard_element(&mut self, pos: PStackPointer) -> Option<T> {
        self.element_index(pos)
            .map(|index| self.stack.swap_remove(index))
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
            let Some(element) = self.element(index) else {
                break;
            };
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
            let duplicate = res
                .top()
                .is_some_and(|top| compare(top, &candidate) == Ordering::Equal);
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
        let take_first = match (st1.top(), st2.top()) {
            (Some(left), Some(right)) => compare(left, right) == Ordering::Less,
            _ => false,
        };
        if take_first {
            st1.pop()
        } else {
            st2.pop()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{PStack, PStackInt, PSTACK_DEFAULT_SIZE};

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
        assert_eq!(stack.top_stack_pointer(), Some(2));
        assert_eq!(stack.top(), Some(&30));
        assert_eq!(stack.below_top(), Some(&20));
        assert_eq!(stack.pop(), Some(30));

        stack.reset();
        assert!(stack.is_empty());
        assert_eq!(stack.allocated_size(), 4);
    }

    #[test]
    fn default_stack_starts_at_c_default_size() {
        let stack = PStack::<usize>::new();
        assert_eq!(stack.allocated_size(), PSTACK_DEFAULT_SIZE);
    }

    #[test]
    fn element_assignment_and_discard_match_c_index_behavior() {
        let mut stack = PStack::with_size(4);
        for value in [1, 2, 3, 4] {
            stack.push(value);
        }

        assert_eq!(stack.element(2), Some(&3));
        assert_eq!(stack.element(-1), None);
        assert!(stack.assign(2, 30));
        assert_eq!(stack.element(2), Some(&30));
        assert!(stack.contains_value(&30));
        assert!(!stack.contains_value(&3));

        assert_eq!(stack.discard_element(1), Some(2));
        assert_eq!(stack.as_slice(), &[1, 4, 30]);
        assert!(!stack.assign(10, 0));
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
