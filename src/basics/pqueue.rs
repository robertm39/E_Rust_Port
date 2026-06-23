pub const PQUEUE_DEFAULT_SIZE: usize = 128;

pub type PQueueInt = i64;
pub type PQueueIndex = isize;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IntOrP<P> {
    Int(PQueueInt),
    Pointer(P),
}

impl<P> IntOrP<P> {
    #[must_use]
    pub const fn int(value: PQueueInt) -> Self {
        Self::Int(value)
    }

    #[must_use]
    pub const fn pointer(value: P) -> Self {
        Self::Pointer(value)
    }

    #[must_use]
    pub const fn as_int(&self) -> Option<PQueueInt> {
        match self {
            Self::Int(value) => Some(*value),
            Self::Pointer(_) => None,
        }
    }

    #[must_use]
    pub const fn as_pointer(&self) -> Option<&P> {
        match self {
            Self::Int(_) => None,
            Self::Pointer(value) => Some(value),
        }
    }

    pub fn into_int(self) -> Option<PQueueInt> {
        match self {
            Self::Int(value) => Some(value),
            Self::Pointer(_) => None,
        }
    }

    pub fn into_pointer(self) -> Option<P> {
        match self {
            Self::Int(_) => None,
            Self::Pointer(value) => Some(value),
        }
    }
}

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
        while self.get_next().is_some() {}
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
            self.grow_after_full_wrap();
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
            self.grow_after_full_wrap();
        }
    }

    pub fn get_next(&mut self) -> Option<T> {
        if self.is_empty() {
            return None;
        }

        let result = self.queue[self.tail].take();
        self.tail += 1;
        if self.tail == self.size {
            self.tail = 0;
        }
        result
    }

    pub fn get_last(&mut self) -> Option<T> {
        if self.is_empty() {
            return None;
        }

        self.head = if self.head == 0 {
            self.size - 1
        } else {
            self.head - 1
        };
        self.queue[self.head].take()
    }

    #[must_use]
    pub fn look(&self) -> Option<&T> {
        if self.is_empty() {
            None
        } else {
            self.queue[self.tail].as_ref()
        }
    }

    #[must_use]
    pub fn look_last(&self) -> Option<&T> {
        if self.is_empty() {
            return None;
        }
        let index = if self.head == 0 {
            self.size - 1
        } else {
            self.head - 1
        };
        self.queue[index].as_ref()
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
    pub fn inc_index(&self, index: PQueueIndex) -> PQueueIndex {
        let Some(index) = checked_index(index, self.size) else {
            return -1;
        };
        let next = (index + 1) % self.size;
        if next == self.head {
            -1
        } else {
            usize_to_queue_index(next)
        }
    }

    #[must_use]
    pub fn element(&self, index: PQueueIndex) -> Option<&T> {
        checked_index(index, self.size)
            .and_then(|index| self.queue.get(index))
            .and_then(Option::as_ref)
    }

    fn grow_after_full_wrap(&mut self) {
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

    pub fn get_next_int(&mut self) -> Option<PQueueInt> {
        self.get_next().and_then(IntOrP::into_int)
    }

    pub fn get_next_pointer(&mut self) -> Option<P> {
        self.get_next().and_then(IntOrP::into_pointer)
    }

    pub fn get_last_int(&mut self) -> Option<PQueueInt> {
        self.get_last().and_then(IntOrP::into_int)
    }

    pub fn get_last_pointer(&mut self) -> Option<P> {
        self.get_last().and_then(IntOrP::into_pointer)
    }

    #[must_use]
    pub fn look_int(&self) -> Option<PQueueInt> {
        self.look().and_then(IntOrP::as_int)
    }

    #[must_use]
    pub fn look_pointer(&self) -> Option<&P> {
        self.look().and_then(IntOrP::as_pointer)
    }

    #[must_use]
    pub fn look_last_int(&self) -> Option<PQueueInt> {
        self.look_last().and_then(IntOrP::as_int)
    }

    #[must_use]
    pub fn look_last_pointer(&self) -> Option<&P> {
        self.look_last().and_then(IntOrP::as_pointer)
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
        assert_eq!(queue.look(), Some(&10));
        assert_eq!(queue.look_last(), Some(&20));
        assert_eq!(queue.get_next(), Some(10));
        assert_eq!(queue.get_next(), Some(20));
        assert_eq!(queue.get_next(), None);
    }

    #[test]
    fn get_last_views_queue_as_stack() {
        let mut queue = PQueue::with_size(4);
        for value in [1, 2, 3] {
            queue.store(value);
        }

        assert_eq!(queue.get_last(), Some(3));
        assert_eq!(queue.look_last(), Some(&2));
        assert_eq!(queue.get_next(), Some(1));
        assert_eq!(queue.get_last(), Some(2));
        assert!(queue.is_empty());
    }

    #[test]
    fn bury_places_values_at_the_queue_front() {
        let mut queue = PQueue::with_size(4);
        queue.store(1);
        queue.store(2);
        queue.bury(0);

        assert_eq!(queue.cardinality(), 3);
        assert_eq!(queue.get_next(), Some(0));
        assert_eq!(queue.get_next(), Some(1));
        assert_eq!(queue.get_next(), Some(2));
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
        assert_eq!(queue.element(4), Some(&0));
        assert_eq!(queue.element(7), Some(&3));
        assert_eq!(queue.get_next(), Some(0));
        assert_eq!(queue.get_next(), Some(1));
        assert_eq!(queue.get_next(), Some(2));
        assert_eq!(queue.get_next(), Some(3));
    }

    #[test]
    fn wrapped_full_growth_keeps_fifo_order() {
        let mut queue = PQueue::with_size(4);
        for value in [1, 2, 3] {
            queue.store(value);
        }
        assert_eq!(queue.get_next(), Some(1));
        queue.store(4);
        queue.store(5);

        assert_eq!(queue.allocated_size(), 8);
        assert_eq!(queue.tail_index(), 5);
        assert_eq!(queue.get_next(), Some(2));
        assert_eq!(queue.get_next(), Some(3));
        assert_eq!(queue.get_next(), Some(4));
        assert_eq!(queue.get_next(), Some(5));
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
            if let Some(value) = queue.element(index) {
                values.push(*value);
            }
            index = queue.inc_index(index);
        }

        assert_eq!(values, vec![10, 20, 30, 40]);
    }

    #[test]
    fn reset_preserves_allocation_and_empties_logical_contents() {
        let mut queue = PQueue::with_size(2);
        queue.store("a");
        queue.store("b");
        assert_eq!(queue.allocated_size(), 4);

        queue.reset();
        assert!(queue.is_empty());
        assert_eq!(queue.allocated_size(), 4);
        assert_eq!(queue.get_next(), None);
        queue.store("c");
        assert_eq!(queue.get_next(), Some("c"));
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
    }
}
