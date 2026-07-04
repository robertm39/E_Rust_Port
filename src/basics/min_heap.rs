use std::cmp::Ordering;
use std::fmt::Write as _;

fn heap_index(index: usize) -> isize {
    isize::try_from(index).unwrap_or(isize::MAX)
}

pub struct MinHeap<T, C, S = fn(&T, isize)> {
    entries: Vec<T>,
    cmp: C,
    setter: Option<S>,
}

impl<T, C> MinHeap<T, C, fn(&T, isize)>
where
    C: Fn(&T, &T) -> Ordering,
{
    #[must_use]
    pub fn new(cmp: C) -> Self {
        Self {
            entries: Vec::new(),
            cmp,
            setter: None,
        }
    }
}

impl<T, C, S> MinHeap<T, C, S>
where
    C: Fn(&T, &T) -> Ordering,
    S: FnMut(&T, isize),
{
    #[must_use]
    pub fn with_index(cmp: C, setter: S) -> Self {
        Self {
            entries: Vec::new(),
            cmp,
            setter: Some(setter),
        }
    }

    #[must_use]
    pub fn size(&self) -> usize {
        self.entries.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    #[must_use]
    pub fn as_slice(&self) -> &[T] {
        &self.entries
    }

    #[must_use]
    pub fn peek_min(&self) -> Option<&T> {
        self.entries.first()
    }

    #[must_use]
    pub fn get(&self, index: usize) -> Option<&T> {
        self.entries.get(index)
    }

    #[must_use]
    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        self.entries.get_mut(index)
    }

    pub fn add(&mut self, value: T) {
        self.entries.push(value);
        let index = self.entries.len() - 1;
        self.call_setter_at(index, heap_index(index));
        self.bubble_up(index);
    }

    pub fn add_int(&mut self, value: T) {
        self.add(value);
    }

    pub fn add_ptr(&mut self, value: T) {
        self.add(value);
    }

    #[must_use]
    pub fn pop_min(&mut self) -> Option<T> {
        if self.entries.is_empty() {
            return None;
        }

        let result = self.entries.swap_remove(0);
        if !self.entries.is_empty() {
            self.call_setter_at(0, 0);
            self.drop_down(0);
        }
        self.call_setter_for_value(&result, -1);
        Some(result)
    }

    /// Pop the minimum element with the C `MinHeapPopMin` empty-heap contract.
    ///
    /// # Panics
    ///
    /// Panics when the heap is empty. The C implementation reports this path
    /// through `SysError("Trying to get an element from an empty heap", -1)`.
    #[must_use]
    pub fn pop_min_nonempty(&mut self) -> T {
        assert!(
            !self.entries.is_empty(),
            "Trying to get an element from an empty heap"
        );
        match self.pop_min() {
            Some(value) => value,
            None => panic!("MinHeapPopMin lost non-empty heap element"),
        }
    }

    pub fn update_element(&mut self, index: usize) {
        if self.entries.is_empty() && index == 0 {
            return;
        }
        self.assert_valid_index(index, "MinHeapUpdateElement");

        if index > 0 && self.compare_indices(index, parent(index)).is_lt() {
            self.bubble_up(index);
        } else {
            self.drop_down(index);
        }
    }

    pub fn update_element_c(&mut self, index: isize) {
        if index < 0 {
            return;
        }
        self.update_element(heap_index_from_signed(index, "MinHeapUpdateElement"));
    }

    pub fn remove_element(&mut self, index: usize) -> T {
        self.assert_valid_index(index, "MinHeapRemoveElement");

        let result = self.entries.swap_remove(index);
        self.call_setter_for_value(&result, -1);
        if index < self.entries.len() {
            self.call_setter_at(index, heap_index(index));
            self.update_element(index);
        }
        result
    }

    pub fn remove_element_c(&mut self, index: isize) -> T {
        self.remove_element(heap_index_from_signed(index, "MinHeapRemoveElement"))
    }

    pub fn decr_key(&mut self, index: usize) {
        self.drop_down(index);
    }

    pub fn decr_key_c(&mut self, index: isize) {
        self.decr_key(heap_index_from_signed(index, "MinHeapDecrKey"));
    }

    pub fn incr_key(&mut self, index: usize) {
        if self.entries.is_empty() && index == 0 {
            return;
        }
        self.assert_valid_index(index, "MinHeapIncrKey");
        self.bubble_up(index);
    }

    pub fn incr_key_c(&mut self, index: isize) {
        if index < 0 {
            return;
        }
        self.incr_key(heap_index_from_signed(index, "MinHeapIncrKey"));
    }

    #[must_use]
    pub fn debug_print_string(&self) -> String
    where
        T: std::fmt::Display,
    {
        let mut result = String::new();
        for entry in &self.entries {
            let write_result = write!(&mut result, "{entry}; ");
            debug_assert!(write_result.is_ok());
        }
        result
    }

    #[must_use]
    pub fn debug_print_pointer_string(&self) -> String
    where
        T: Copy + std::fmt::Pointer,
    {
        let mut result = String::new();
        for entry in &self.entries {
            let write_result = write!(&mut result, "{:p}; ", *entry);
            debug_assert!(write_result.is_ok());
        }
        result
    }

    fn compare_indices(&self, left: usize, right: usize) -> Ordering {
        (self.cmp)(&self.entries[left], &self.entries[right])
    }

    fn assert_valid_index(&self, index: usize, caller: &str) {
        assert!(
            index < self.entries.len(),
            "{caller} called with invalid index {index}"
        );
    }

    fn bubble_up(&mut self, mut child_index: usize) {
        while child_index > 0 {
            let parent_index = parent(child_index);
            if self.compare_indices(child_index, parent_index).is_lt() {
                self.swap_and_set(child_index, parent_index);
                child_index = parent_index;
            } else {
                break;
            }
        }
    }

    fn drop_down(&mut self, mut current_index: usize) {
        while let Some(left) = left_child_if_present(current_index, self.entries.len()) {
            let mut min_child_index = current_index;
            let right = right_child(current_index);

            if self.compare_indices(left, min_child_index).is_lt() {
                min_child_index = left;
            }
            if right < self.entries.len() && self.compare_indices(right, min_child_index).is_lt() {
                min_child_index = right;
            }

            if min_child_index == current_index {
                break;
            }
            self.swap_and_set(current_index, min_child_index);
            current_index = min_child_index;
        }
    }

    fn swap_and_set(&mut self, left: usize, right: usize) {
        self.entries.swap(left, right);
        self.call_setter_at(left, heap_index(left));
        self.call_setter_at(right, heap_index(right));
    }

    fn call_setter_at(&mut self, entry_index: usize, heap_index: isize) {
        if let Some(setter) = self.setter.as_mut() {
            setter(&self.entries[entry_index], heap_index);
        }
    }

    fn call_setter_for_value(&mut self, value: &T, heap_index: isize) {
        if let Some(setter) = self.setter.as_mut() {
            setter(value, heap_index);
        }
    }
}

const fn parent(index: usize) -> usize {
    (index - 1) / 2
}

const fn left_child(index: usize) -> usize {
    2 * index + 1
}

const fn right_child(index: usize) -> usize {
    left_child(index) + 1
}

fn left_child_if_present(index: usize, size: usize) -> Option<usize> {
    let left = index.checked_mul(2)?.checked_add(1)?;
    (left < size).then_some(left)
}

fn heap_index_from_signed(index: isize, caller: &str) -> usize {
    usize::try_from(index)
        .unwrap_or_else(|_error| panic!("{caller} called with negative index {index}"))
}

#[cfg(test)]
mod tests {
    use super::MinHeap;
    use std::cell::RefCell;
    use std::cmp::Ordering;
    use std::rc::Rc;

    #[derive(Clone, Debug, Eq, PartialEq)]
    struct Task {
        id: usize,
        priority: i32,
    }

    fn task_cmp(left: &Task, right: &Task) -> Ordering {
        left.priority
            .cmp(&right.priority)
            .then_with(|| left.id.cmp(&right.id))
    }

    #[test]
    fn integer_heap_pops_in_minimum_order() {
        let mut heap = MinHeap::new(i64::cmp);
        for value in [5, -1, 4, 10, 20, 12, 8, 99, 1] {
            heap.add_int(value);
        }

        assert_eq!(heap.size(), 9);
        assert_eq!(heap.peek_min(), Some(&-1));
        assert_eq!(
            heap.debug_print_string(),
            "-1; 1; 4; 5; 20; 12; 8; 99; 10; "
        );

        let mut popped = Vec::new();
        while let Some(value) = heap.pop_min() {
            popped.push(value);
        }
        assert_eq!(popped, vec![-1, 1, 4, 5, 8, 10, 12, 20, 99]);
        assert!(heap.is_empty());
        assert_eq!(heap.pop_min(), None);
    }

    #[test]
    fn setter_tracks_indices_through_add_and_pop() {
        let indices = Rc::new(RefCell::new(vec![-2_isize; 4]));
        let setter_indices = Rc::clone(&indices);
        let mut heap = MinHeap::with_index(task_cmp, move |task: &Task, index| {
            setter_indices.borrow_mut()[task.id] = index;
        });

        heap.add_ptr(Task {
            id: 1,
            priority: 30,
        });
        heap.add_ptr(Task {
            id: 2,
            priority: 10,
        });
        heap.add_ptr(Task {
            id: 3,
            priority: 20,
        });

        assert_eq!(indices.borrow()[2], 0);
        let popped = heap.pop_min_nonempty();
        assert_eq!(popped.id, 2);
        assert_eq!(indices.borrow()[2], -1);
        assert_eq!(heap.peek_min().unwrap().id, 3);
    }

    #[test]
    fn pop_min_nonempty_matches_c_minimum_pop_contract() {
        let mut heap = MinHeap::new(i64::cmp);
        heap.add_int(4);
        heap.add_int(1);
        heap.add_int(3);

        assert_eq!(heap.pop_min_nonempty(), 1);
        assert_eq!(heap.pop_min_nonempty(), 3);
        assert_eq!(heap.pop_min_nonempty(), 4);
    }

    #[test]
    #[should_panic(expected = "Trying to get an element from an empty heap")]
    fn pop_min_nonempty_panics_on_empty_like_c_sys_error() {
        let mut heap = MinHeap::new(i64::cmp);

        let _value = heap.pop_min_nonempty();
    }

    #[test]
    fn update_element_repairs_after_priority_changes() {
        let indices = Rc::new(RefCell::new(vec![-2_isize; 4]));
        let setter_indices = Rc::clone(&indices);
        let mut heap = MinHeap::with_index(task_cmp, move |task: &Task, index| {
            setter_indices.borrow_mut()[task.id] = index;
        });

        heap.add(Task {
            id: 1,
            priority: 30,
        });
        heap.add(Task {
            id: 2,
            priority: 10,
        });
        heap.add(Task {
            id: 3,
            priority: 20,
        });

        let id1_index = usize::try_from(indices.borrow()[1]).unwrap();
        heap.get_mut(id1_index).unwrap().priority = 5;
        heap.update_element(id1_index);
        assert_eq!(heap.peek_min().unwrap().id, 1);

        heap.get_mut(0).unwrap().priority = 50;
        heap.update_element(0);
        assert_eq!(heap.peek_min().unwrap().id, 2);
    }

    #[test]
    fn c_named_incr_and_decr_key_helpers_preserve_c_direction() {
        let mut heap = MinHeap::new(i64::cmp);
        heap.add(1);
        heap.add(5);
        heap.add(10);

        *heap.get_mut(0).unwrap() = 20;
        heap.decr_key(0);
        assert_eq!(heap.peek_min(), Some(&5));

        let ten_index = heap
            .as_slice()
            .iter()
            .position(|value| *value == 10)
            .unwrap();
        *heap.get_mut(ten_index).unwrap() = 2;
        heap.incr_key(ten_index);
        assert_eq!(heap.peek_min(), Some(&2));
    }

    #[test]
    fn remove_element_detaches_value_and_repairs_indices() {
        let indices = Rc::new(RefCell::new(vec![-2_isize; 4]));
        let setter_indices = Rc::clone(&indices);
        let mut heap = MinHeap::with_index(task_cmp, move |task: &Task, index| {
            setter_indices.borrow_mut()[task.id] = index;
        });

        heap.add(Task {
            id: 1,
            priority: 30,
        });
        heap.add(Task {
            id: 2,
            priority: 10,
        });
        heap.add(Task {
            id: 3,
            priority: 20,
        });

        let id3_index = usize::try_from(indices.borrow()[3]).unwrap();
        let removed = heap.remove_element(id3_index);
        assert_eq!(removed.id, 3);
        assert_eq!(indices.borrow()[3], -1);
        assert_eq!(heap.size(), 2);

        let remaining = heap
            .as_slice()
            .iter()
            .map(|task| task.id)
            .collect::<Vec<_>>();
        assert_eq!(remaining.len(), 2);
        assert!(remaining.contains(&1));
        assert!(remaining.contains(&2));
    }

    #[test]
    fn update_and_incr_preserve_c_empty_root_noop() {
        let mut heap = MinHeap::new(i64::cmp);

        heap.update_element(0);
        heap.incr_key(0);

        assert!(heap.is_empty());
    }

    #[test]
    fn signed_update_and_incr_negative_indices_match_c_noop() {
        let mut heap = MinHeap::new(i64::cmp);
        heap.add(3);
        heap.add(7);
        let before = heap.as_slice().to_vec();

        heap.update_element_c(-1);
        heap.incr_key_c(-2);

        assert_eq!(heap.as_slice(), before.as_slice());
    }

    #[test]
    #[should_panic(expected = "MinHeapRemoveElement called with negative index -1")]
    fn signed_remove_panics_on_negative_index_like_c_assertion() {
        let mut heap = MinHeap::new(i64::cmp);

        let _removed = heap.remove_element_c(-1);
    }

    #[test]
    #[should_panic(expected = "MinHeapDecrKey called with negative index -1")]
    fn signed_decr_panics_on_negative_index_like_c_stack_assertion() {
        let mut heap = MinHeap::new(i64::cmp);

        heap.decr_key_c(-1);
    }

    #[test]
    #[should_panic(expected = "MinHeapUpdateElement called with invalid index 1")]
    fn update_element_panics_on_positive_out_of_range_index() {
        let mut heap = MinHeap::new(i64::cmp);

        heap.update_element(1);
    }

    #[test]
    #[should_panic(expected = "MinHeapRemoveElement called with invalid index 0")]
    fn remove_element_panics_on_out_of_range_index() {
        let mut heap = MinHeap::new(i64::cmp);

        heap.remove_element(0);
    }

    #[test]
    #[should_panic(expected = "MinHeapIncrKey called with invalid index 1")]
    fn incr_key_panics_on_positive_out_of_range_index() {
        let mut heap = MinHeap::new(i64::cmp);

        heap.incr_key(1);
    }

    #[test]
    fn decr_key_preserves_c_out_of_range_leaf_noop() {
        let mut heap = MinHeap::new(i64::cmp);
        heap.add(3);
        heap.add(7);

        heap.decr_key(usize::MAX);

        assert_eq!(heap.as_slice(), &[3, 7]);
    }

    #[test]
    fn pointer_debug_print_uses_address_shape_in_heap_order() {
        let first = Task {
            id: 1,
            priority: 20,
        };
        let second = Task {
            id: 2,
            priority: 10,
        };
        let mut heap = MinHeap::new(|left: &&Task, right: &&Task| task_cmp(left, right));

        heap.add_ptr(&first);
        heap.add_ptr(&second);

        assert_eq!(
            heap.debug_print_pointer_string(),
            format!("{:p}; {:p}; ", &second, &first)
        );
    }
}
