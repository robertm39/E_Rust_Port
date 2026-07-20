pub use crate::basics::defines::{IntOrP, IntOrPInt as PQueueInt};

pub const PQUEUE_DEFAULT_SIZE: usize = 128;

pub type PQueueIndex = isize;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PQueue<T> {
    size: usize,
    head: usize,
    tail: usize,
    queue: Vec<Option<T>>,
}

impl<T> Default for PQueue<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> PQueue<T> {
    #[must_use]
    pub fn new() -> Self {
        Self::with_size(PQUEUE_DEFAULT_SIZE)
    }

    /// Allocate an empty circular queue with selectable initial size.
    ///
    /// # Panics
    ///
    /// Panics when `size` is zero. The C queue uses `head == tail` as its
    /// empty sentinel, so a zero-sized allocation cannot represent progress.
    #[must_use]
    pub fn with_size(size: usize) -> Self {
        assert!(size > 0, "PQueue initial size must be non-zero");
        let mut queue = Vec::with_capacity(size);
        queue.resize_with(size, || None);
        Self {
            size,
            head: 0,
            tail: 0,
            queue,
        }
    }

    #[must_use]
    pub const fn allocated_size(&self) -> usize {
        self.size
    }

    #[must_use]
    pub fn head_index(&self) -> PQueueIndex {
        usize_to_queue_index(self.head)
    }

    #[must_use]
    pub fn raw_tail_index(&self) -> PQueueIndex {
        usize_to_queue_index(self.tail)
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.head == self.tail
    }

    #[must_use]
    pub const fn cardinality(&self) -> usize {
        if self.head >= self.tail {
            self.head - self.tail
        } else {
            self.head + (self.size - self.tail)
        }
    }

    pub fn reset(&mut self) {
        self.head = 0;
        self.tail = 0;
    }

    pub fn store(&mut self, value: T) {
        self.queue[self.head] = Some(value);
        self.head += 1;
        if self.head == self.size {
            self.head = 0;
        }
        if self.head == self.tail {
            self.grow_c_raw();
        }
    }

    pub fn bury(&mut self, value: T) {
        self.tail = if self.tail == 0 {
            self.size - 1
        } else {
            self.tail - 1
        };
        self.queue[self.tail] = Some(value);
        if self.head == self.tail {
            self.grow_c_raw();
        }
    }

    /// Extract the next queue value.
    ///
    /// # Panics
    ///
    /// Panics when the queue is empty, matching the C `PQueueGetNext`
    /// assertion.
    pub fn get_next(&mut self) -> T
    where
        T: Clone,
    {
        assert!(!self.is_empty(), "PQueueGetNext called on an empty queue");

        let result = self.queue[self.tail]
            .clone()
            .unwrap_or_else(|| panic!("PQueue used tail slot was empty"));
        self.tail += 1;
        if self.tail == self.size {
            self.tail = 0;
        }
        result
    }

    /// Extract the newest queue value, viewing the queue as a stack.
    ///
    /// # Panics
    ///
    /// Panics when the queue is empty, matching the C `PQueueGetLast`
    /// assertion.
    pub fn get_last(&mut self) -> T
    where
        T: Clone,
    {
        assert!(!self.is_empty(), "PQueueGetLast called on an empty queue");

        self.head = if self.head == 0 {
            self.size - 1
        } else {
            self.head - 1
        };
        self.queue[self.head]
            .clone()
            .unwrap_or_else(|| panic!("PQueue used head slot was empty"))
    }

    /// Move the newest queue value out, viewing the queue as a stack.
    ///
    /// This owned-consumption variant intentionally clears the consumed
    /// backing slot. The C-compatible [`Self::get_last`] retains that slot so
    /// callers using absolute queue indices can continue to inspect it.
    ///
    /// # Panics
    ///
    /// Panics when the queue is empty.
    pub(crate) fn take_last(&mut self) -> T {
        assert!(!self.is_empty(), "PQueueTakeLast called on an empty queue");

        self.head = if self.head == 0 {
            self.size - 1
        } else {
            self.head - 1
        };
        self.queue[self.head]
            .take()
            .unwrap_or_else(|| panic!("PQueue used head slot was empty"))
    }

    #[must_use]
    /// Return the next queue value without extracting it.
    ///
    /// # Panics
    ///
    /// Panics when the queue is empty, matching the C `PQueueLook` assertion.
    pub fn look(&self) -> &T {
        assert!(!self.is_empty(), "PQueueLook called on an empty queue");
        self.queue[self.tail]
            .as_ref()
            .unwrap_or_else(|| panic!("PQueue used tail slot was empty"))
    }

    #[must_use]
    /// Return the newest queue value without extracting it.
    ///
    /// # Panics
    ///
    /// Panics when the queue is empty, matching the C `PQueueLookLast`
    /// assertion.
    pub fn look_last(&self) -> &T {
        assert!(!self.is_empty(), "PQueueLookLast called on an empty queue");
        let index = if self.head == 0 {
            self.size - 1
        } else {
            self.head - 1
        };
        self.queue[index]
            .as_ref()
            .unwrap_or_else(|| panic!("PQueue used head slot was empty"))
    }

    #[must_use]
    pub fn tail_index(&self) -> PQueueIndex {
        if self.is_empty() {
            -1
        } else {
            usize_to_queue_index(self.tail)
        }
    }

    #[must_use]
    /// Advance an absolute queue slot with the C `PQueueIncIndex` arithmetic.
    ///
    /// # Panics
    ///
    /// Panics if the queue size cannot be represented as a C-shaped index or
    /// if advancing `index` would overflow the C-shaped index type.
    pub fn inc_index(&self, index: PQueueIndex) -> PQueueIndex {
        let size = PQueueIndex::try_from(self.size).expect("PQueue size fits C long");
        let next = index.checked_add(1).expect("PQueueIncIndex index overflow") % size;
        if next == self.head_index() {
            -1
        } else {
            next
        }
    }

    /// Return the backing slot at absolute `index`.
    ///
    /// # Panics
    ///
    /// Panics when `index` is negative, outside the allocated ring, or points
    /// at a slot that has never been initialized by `store`/`bury`. The C
    /// helper performs raw array access, so callers must supply a valid
    /// absolute slot.
    #[must_use]
    pub fn element(&self, index: PQueueIndex) -> &T {
        let index = checked_index(index, self.size)
            .unwrap_or_else(|| panic!("PQueueElement called with invalid index {index}"));
        self.queue[index]
            .as_ref()
            .unwrap_or_else(|| panic!("PQueueElement called on an uninitialized slot {index}"))
    }

    /// Increase the backing ring size using the exported C `PQueueGrow`
    /// layout.
    ///
    /// C normally reaches this only when `store`/`bury` has made the ring
    /// full. Direct calls on a non-full queue still double the allocation and
    /// shift `tail` by the old size, which can make old uninitialized slots
    /// appear live. Rust preserves that raw shape with `None` sentinel slots,
    /// so reading such slots through `element` remains an invariant failure.
    ///
    /// # Panics
    ///
    /// Panics if doubling the allocated size overflows.
    pub fn grow_c_raw(&mut self) {
        let old_size = self.size;
        let old_head = self.head;
        let Some(new_size) = old_size.checked_mul(2) else {
            panic!("PQueue capacity overflow");
        };
        let mut new_queue = Vec::with_capacity(new_size);
        new_queue.resize_with(new_size, || None);

        let mut index = 0;
        while index < old_head {
            new_queue[index] = self.queue[index].take();
            index += 1;
        }
        while index < old_size {
            new_queue[index + old_size] = self.queue[index].take();
            index += 1;
        }

        self.tail += old_size;
        self.queue = new_queue;
        self.size = new_size;
    }
}

impl<P> PQueue<IntOrP<P>> {
    pub fn store_int(&mut self, value: PQueueInt) {
        self.store(IntOrP::Int(value));
    }

    pub fn store_pointer(&mut self, value: P) {
        self.store(IntOrP::Pointer(value));
    }

    pub fn bury_int(&mut self, value: PQueueInt) {
        self.bury(IntOrP::Int(value));
    }

    pub fn bury_pointer(&mut self, value: P) {
        self.bury(IntOrP::Pointer(value));
    }

    pub fn get_next_int(&mut self) -> Option<PQueueInt>
    where
        P: Clone,
    {
        self.get_next().into_int()
    }

    pub fn get_next_pointer(&mut self) -> Option<P>
    where
        P: Clone,
    {
        self.get_next().into_pointer()
    }

    pub fn get_last_int(&mut self) -> Option<PQueueInt>
    where
        P: Clone,
    {
        self.get_last().into_int()
    }

    pub fn get_last_pointer(&mut self) -> Option<P>
    where
        P: Clone,
    {
        self.get_last().into_pointer()
    }

    #[must_use]
    pub fn look_int(&self) -> Option<PQueueInt> {
        self.look().as_int()
    }

    #[must_use]
    pub fn look_pointer(&self) -> Option<&P> {
        self.look().as_pointer()
    }

    #[must_use]
    pub fn look_last_int(&self) -> Option<PQueueInt> {
        self.look_last().as_int()
    }

    #[must_use]
    pub fn look_last_pointer(&self) -> Option<&P> {
        self.look_last().as_pointer()
    }
}

fn usize_to_queue_index(index: usize) -> PQueueIndex {
    match PQueueIndex::try_from(index) {
        Ok(value) => value,
        Err(_) => PQueueIndex::MAX,
    }
}

fn checked_index(index: PQueueIndex, size: usize) -> Option<usize> {
    let index = usize::try_from(index).ok()?;
    (index < size).then_some(index)
}

#[cfg(test)]
mod tests {
    use super::{IntOrP, PQueue, PQUEUE_DEFAULT_SIZE};

    #[test]
    fn default_queue_starts_empty_with_c_default_size() {
        let queue = PQueue::<usize>::new();
        assert_eq!(queue.allocated_size(), PQUEUE_DEFAULT_SIZE);
        assert!(queue.is_empty());
        assert_eq!(queue.cardinality(), 0);
        assert_eq!(queue.head_index(), 0);
        assert_eq!(queue.tail_index(), -1);
    }

    #[test]
    fn store_and_get_next_are_fifo() {
        let mut queue = PQueue::with_size(4);
        queue.store(10);
        queue.store(20);

        assert_eq!(queue.cardinality(), 2);
        assert_eq!(queue.look(), &10);
        assert_eq!(queue.look_last(), &20);
        assert_eq!(queue.get_next(), 10);
        assert_eq!(queue.element(0), &10);
        assert_eq!(queue.get_next(), 20);
        assert_eq!(queue.element(1), &20);
        assert!(queue.is_empty());
    }

    #[test]
    fn get_last_views_queue_as_stack() {
        let mut queue = PQueue::with_size(4);
        for value in [1, 2, 3] {
            queue.store(value);
        }

        assert_eq!(queue.get_last(), 3);
        assert_eq!(queue.element(2), &3);
        assert_eq!(queue.look_last(), &2);
        assert_eq!(queue.get_next(), 1);
        assert_eq!(queue.element(0), &1);
        assert_eq!(queue.get_last(), 2);
        assert_eq!(queue.element(1), &2);
        assert!(queue.is_empty());
    }

    #[test]
    fn take_last_moves_value_and_clears_backing_slot() {
        struct NonClone(&'static str);

        let mut queue = PQueue::with_size(4);
        queue.store(NonClone("first"));
        queue.store(NonClone("last"));

        assert_eq!(queue.take_last().0, "last");
        assert!(queue.queue[1].is_none());
        assert_eq!(queue.take_last().0, "first");
        assert!(queue.queue[0].is_none());
        assert!(queue.is_empty());
    }

    #[test]
    fn bury_places_values_at_the_queue_front() {
        let mut queue = PQueue::with_size(4);
        queue.store(1);
        queue.store(2);
        queue.bury(0);

        assert_eq!(queue.cardinality(), 3);
        assert_eq!(queue.get_next(), 0);
        assert_eq!(queue.get_next(), 1);
        assert_eq!(queue.get_next(), 2);
    }

    #[test]
    fn full_store_growth_preserves_c_absolute_layout() {
        let mut queue = PQueue::with_size(4);
        for value in [0, 1, 2, 3] {
            queue.store(value);
        }

        assert_eq!(queue.allocated_size(), 8);
        assert_eq!(queue.head_index(), 0);
        assert_eq!(queue.tail_index(), 4);
        assert_eq!(queue.cardinality(), 4);
        assert_eq!(queue.element(4), &0);
        assert_eq!(queue.element(7), &3);
        assert_eq!(queue.get_next(), 0);
        assert_eq!(queue.get_next(), 1);
        assert_eq!(queue.get_next(), 2);
        assert_eq!(queue.get_next(), 3);
    }

    #[test]
    fn direct_raw_grow_preserves_c_full_ring_layout() {
        let mut queue = PQueue {
            size: 4,
            head: 1,
            tail: 1,
            queue: vec![Some(4), Some(1), Some(2), Some(3)],
        };

        queue.grow_c_raw();

        assert_eq!(queue.allocated_size(), 8);
        assert_eq!(queue.head_index(), 1);
        assert_eq!(queue.tail_index(), 5);
        assert_eq!(queue.cardinality(), 4);
        assert_eq!(queue.element(5), &1);
        assert_eq!(queue.element(6), &2);
        assert_eq!(queue.element(7), &3);
        assert_eq!(queue.element(0), &4);
        assert_eq!(queue.get_next(), 1);
        assert_eq!(queue.get_next(), 2);
        assert_eq!(queue.get_next(), 3);
        assert_eq!(queue.get_next(), 4);
    }

    #[test]
    #[should_panic(expected = "PQueueElement called on an uninitialized slot 4")]
    fn direct_raw_grow_on_nonfull_queue_preserves_c_hazard_as_uninitialized_slot() {
        let mut queue = PQueue::with_size(4);
        queue.store(10);
        queue.store(20);

        queue.grow_c_raw();

        assert_eq!(queue.allocated_size(), 8);
        assert_eq!(queue.head_index(), 2);
        assert_eq!(queue.tail_index(), 4);
        assert_eq!(queue.cardinality(), 6);
        assert_eq!(queue.element(0), &10);
        assert_eq!(queue.element(1), &20);
        let _value = queue.element(4);
    }

    #[test]
    fn direct_raw_nonfull_growth_copies_stale_slots_around_new_live_holes() {
        let mut queue = PQueue {
            size: 4,
            head: 2,
            tail: 0,
            queue: vec![Some(10), Some(20), Some(30), Some(40)],
        };

        queue.grow_c_raw();

        assert_eq!(queue.allocated_size(), 8);
        assert_eq!(queue.head_index(), 2);
        assert_eq!(queue.tail_index(), 4);
        assert_eq!(queue.cardinality(), 6);
        assert_eq!(
            queue.queue,
            vec![
                Some(10),
                Some(20),
                None,
                None,
                None,
                None,
                Some(30),
                Some(40)
            ]
        );
    }

    #[test]
    fn wrapped_full_growth_keeps_fifo_order() {
        let mut queue = PQueue::with_size(4);
        for value in [1, 2, 3] {
            queue.store(value);
        }
        assert_eq!(queue.get_next(), 1);
        queue.store(4);
        queue.store(5);

        assert_eq!(queue.allocated_size(), 8);
        assert_eq!(queue.tail_index(), 5);
        assert_eq!(queue.get_next(), 2);
        assert_eq!(queue.get_next(), 3);
        assert_eq!(queue.get_next(), 4);
        assert_eq!(queue.get_next(), 5);
    }

    #[test]
    fn tail_and_increment_indices_iterate_absolute_slots() {
        let mut queue = PQueue::with_size(4);
        for value in [10, 20, 30, 40] {
            queue.store(value);
        }

        let mut index = queue.tail_index();
        let mut values = Vec::new();
        while index != -1 {
            values.push(*queue.element(index));
            index = queue.inc_index(index);
        }

        assert_eq!(values, vec![10, 20, 30, 40]);
    }

    #[test]
    fn increment_index_uses_c_raw_modulo_shape() {
        let mut queue = PQueue::with_size(4);
        queue.store(10);
        queue.store(20);

        assert_eq!(queue.head_index(), 2);
        assert_eq!(queue.inc_index(-1), 0);
        assert_eq!(queue.inc_index(1), -1);
        assert_eq!(queue.inc_index(3), 0);
        assert_eq!(queue.inc_index(4), 1);
    }

    #[test]
    fn reset_preserves_allocation_and_absolute_slots_like_c() {
        let mut queue = PQueue::with_size(4);
        queue.store("a");
        queue.store("b");
        assert_eq!(queue.allocated_size(), 4);

        queue.reset();
        assert!(queue.is_empty());
        assert_eq!(queue.allocated_size(), 4);
        assert_eq!(queue.element(0), &"a");
        assert_eq!(queue.element(1), &"b");
        queue.store("c");
        assert_eq!(queue.element(0), &"c");
        assert_eq!(queue.element(1), &"b");
        assert_eq!(queue.get_next(), "c");
    }

    #[test]
    #[should_panic(expected = "PQueueGetNext called on an empty queue")]
    fn get_next_panics_on_empty_like_c_assertion() {
        let mut queue = PQueue::<usize>::with_size(4);
        let _value = queue.get_next();
    }

    #[test]
    #[should_panic(expected = "PQueueLook called on an empty queue")]
    fn look_panics_on_empty_like_c_assertion() {
        let queue = PQueue::<usize>::with_size(4);
        let _value = queue.look();
    }

    #[test]
    fn mixed_int_or_pointer_helpers_preserve_c_union_call_shape() {
        let mut queue = PQueue::<IntOrP<&str>>::with_size(4);
        queue.store_int(7);
        queue.store_pointer("clause");
        queue.bury_int(3);

        assert_eq!(queue.look_int(), Some(3));
        assert_eq!(queue.get_next_int(), Some(3));
        assert_eq!(queue.get_next_int(), Some(7));
        assert_eq!(queue.look_pointer(), Some(&"clause"));
        assert_eq!(queue.get_next_pointer(), Some("clause"));
    }

    #[test]
    fn mixed_helpers_return_none_when_field_shape_does_not_match() {
        let mut queue = PQueue::<IntOrP<&str>>::with_size(2);
        queue.store_pointer("term");
        assert_eq!(queue.get_next_int(), None);
        assert!(queue.is_empty());
        assert_eq!(queue.element(0).as_pointer(), Some(&"term"));
    }

    #[test]
    #[should_panic(expected = "PQueueElement called with invalid index -1")]
    fn element_panics_on_negative_absolute_index() {
        let queue = PQueue::<usize>::with_size(4);
        let _value = queue.element(-1);
    }

    #[test]
    #[should_panic(expected = "PQueueElement called with invalid index 4")]
    fn element_panics_on_absolute_index_at_capacity() {
        let queue = PQueue::<usize>::with_size(4);
        let _value = queue.element(4);
    }

    #[test]
    #[should_panic(expected = "PQueueElement called on an uninitialized slot 0")]
    fn element_panics_on_never_initialized_absolute_slot() {
        let queue = PQueue::<usize>::with_size(4);
        let _value = queue.element(0);
    }
}
