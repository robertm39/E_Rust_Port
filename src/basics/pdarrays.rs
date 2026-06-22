pub const GROW_EXPONENTIAL: usize = 0;

pub type PDArrayIndex = isize;
pub type PDArrayInt = i64;
pub type PDIntArray = PDArray<PDArrayInt>;
pub type PDPointerArray<T> = PDArray<Option<T>>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PDArray<T> {
    size: usize,
    grow: usize,
    default: T,
    array: Vec<T>,
}

impl<T: Clone> PDArray<T> {
    /// Return an initialized dynamic array with the requested logical size.
    ///
    /// # Panics
    ///
    /// Panics when `init_size` is zero. The C implementation asserts this
    /// invariant before allocating.
    #[must_use]
    pub fn with_default(init_size: usize, grow: usize, default: T) -> Self {
        assert!(init_size > 0, "PDArray initial size must be non-zero");
        Self {
            size: init_size,
            grow,
            default: default.clone(),
            array: vec![default; init_size],
        }
    }

    #[must_use]
    pub const fn size(&self) -> usize {
        self.size
    }

    #[must_use]
    pub const fn grow(&self) -> usize {
        self.grow
    }

    #[must_use]
    pub fn as_slice(&self) -> &[T] {
        &self.array
    }

    /// Enlarge the array enough to cover `idx`.
    ///
    /// # Panics
    ///
    /// Panics if computing the new logical size overflows `usize`.
    pub fn enlarge(&mut self, idx: PDArrayIndex) -> bool {
        let Some(index) = checked_index(idx) else {
            return false;
        };
        if index < self.size {
            return true;
        }

        let new_size = self.new_size_for(index);
        self.array.resize(new_size, self.default.clone());
        self.size = new_size;
        true
    }

    pub fn element_ref(&mut self, idx: PDArrayIndex) -> Option<&mut T> {
        self.enlarge(idx);
        let index = checked_index(idx)?;
        self.array.get_mut(index)
    }

    pub fn element(&mut self, idx: PDArrayIndex) -> Option<&T> {
        self.enlarge(idx);
        let index = checked_index(idx)?;
        self.array.get(index)
    }

    #[must_use]
    pub fn existing_element(&self, idx: PDArrayIndex) -> Option<&T> {
        let index = checked_index(idx)?;
        self.array.get(index)
    }

    pub fn assign(&mut self, idx: PDArrayIndex, value: T) -> bool {
        let Some(element) = self.element_ref(idx) else {
            return false;
        };
        *element = value;
        true
    }

    pub fn delete(&mut self, idx: PDArrayIndex) -> bool {
        let Some(index) = checked_index(idx) else {
            return false;
        };
        if index >= self.size {
            return false;
        }
        self.array[index] = self.default.clone();
        true
    }

    #[must_use]
    pub fn copy_array(&self) -> Self {
        Self {
            size: self.size,
            grow: self.grow,
            default: self.default.clone(),
            array: self.array.clone(),
        }
    }

    fn new_size_for(&self, index: usize) -> usize {
        if self.grow == GROW_EXPONENTIAL {
            let mut new_size = self.size;
            while new_size <= index {
                let Some(doubled) = new_size.checked_mul(2) else {
                    panic!("PDArray capacity overflow");
                };
                new_size = doubled;
            }
            new_size
        } else {
            let Some(block) = index
                .checked_div(self.grow)
                .and_then(|value| value.checked_add(1))
            else {
                panic!("PDArray capacity overflow");
            };
            let Some(new_size) = block.checked_mul(self.grow) else {
                panic!("PDArray capacity overflow");
            };
            new_size
        }
    }
}

impl<T: Clone> PDPointerArray<T> {
    /// Return an initialized pointer-style dynamic array filled with `NULL`.
    ///
    /// # Panics
    ///
    /// Panics when `init_size` is zero.
    #[must_use]
    pub fn new_pointer(init_size: usize, grow: usize) -> Self {
        Self::with_default(init_size, grow, None)
    }

    #[must_use]
    pub fn members(&self) -> usize {
        self.array.iter().filter(|value| value.is_some()).count()
    }

    #[must_use]
    pub fn first_unused(&self) -> usize {
        self.array
            .iter()
            .rposition(Option::is_some)
            .map_or(0, |index| index + 1)
    }

    pub fn store_pointer(&mut self, value: T) -> usize {
        let index = self.first_unused();
        self.assign(index_to_pd(index), Some(value));
        index
    }

    pub fn delete_pointer(&mut self, idx: PDArrayIndex) -> bool {
        self.delete(idx)
    }
}

impl PDIntArray {
    /// Return an initialized integer dynamic array filled with zero.
    ///
    /// # Panics
    ///
    /// Panics when `init_size` is zero.
    #[must_use]
    pub fn new_int(init_size: usize, grow: usize) -> Self {
        Self::with_default(init_size, grow, 0)
    }

    pub fn delete_int(&mut self, idx: PDArrayIndex) -> bool {
        self.delete(idx)
    }

    pub fn store_int(&mut self, value: PDArrayInt) -> usize {
        let index = self
            .array
            .iter()
            .rposition(|stored| *stored != 0)
            .map_or(0, |index| index + 1);
        self.assign(index_to_pd(index), value);
        index
    }

    pub fn add_prefix(&mut self, data: &mut Self, limit: usize) {
        for index in 0..limit {
            let idx = index_to_pd(index);
            let old = self.element_int(idx);
            let new = data.element_int(idx);
            self.assign(idx, old + new);
        }
    }

    pub fn inc_int(&mut self, idx: PDArrayIndex, value: PDArrayInt) -> Option<PDArrayInt> {
        let element = self.element_ref(idx)?;
        *element += value;
        Some(*element)
    }

    pub fn element_int(&mut self, idx: PDArrayIndex) -> PDArrayInt {
        self.element(idx).copied().unwrap_or(0)
    }
}

fn checked_index(idx: PDArrayIndex) -> Option<usize> {
    usize::try_from(idx).ok()
}

fn index_to_pd(index: usize) -> PDArrayIndex {
    match PDArrayIndex::try_from(index) {
        Ok(value) => value,
        Err(_) => PDArrayIndex::MAX,
    }
}

#[cfg(test)]
mod tests {
    use super::{PDArray, PDIntArray, PDPointerArray, GROW_EXPONENTIAL};

    #[test]
    fn pointer_arrays_initialize_to_none_and_grow_exponentially() {
        let mut array = PDPointerArray::new_pointer(2, GROW_EXPONENTIAL);
        assert_eq!(array.size(), 2);
        assert_eq!(array.members(), 0);

        assert!(array.assign(1, Some("one")));
        assert_eq!(array.members(), 1);
        assert_eq!(array.first_unused(), 2);
        assert_eq!(array.store_pointer("two"), 2);
        assert_eq!(array.size(), 4);
        assert_eq!(array.first_unused(), 3);
        assert_eq!(array.existing_element(0), Some(&None));
        assert_eq!(array.existing_element(2), Some(&Some("two")));
    }

    #[test]
    fn fixed_growth_uses_the_smallest_covering_multiple() {
        let mut array = PDPointerArray::<usize>::new_pointer(3, 5);
        assert!(array.assign(12, Some(99)));
        assert_eq!(array.size(), 15);
        assert_eq!(array.existing_element(12), Some(&Some(99)));
        assert_eq!(array.existing_element(14), Some(&None));
    }

    #[test]
    fn delete_ignores_out_of_range_indices_without_growing() {
        let mut array = PDPointerArray::new_pointer(2, GROW_EXPONENTIAL);
        assert!(array.assign(1, Some("value")));
        assert!(!array.delete_pointer(5));
        assert_eq!(array.size(), 2);
        assert!(array.delete_pointer(1));
        assert_eq!(array.existing_element(1), Some(&None));
    }

    #[test]
    fn copy_preserves_contents_size_and_growth() {
        let mut array = PDPointerArray::new_pointer(2, GROW_EXPONENTIAL);
        array.assign(3, Some("three"));

        let copy = array.copy_array();
        assert_eq!(copy.size(), 4);
        assert_eq!(copy.grow(), GROW_EXPONENTIAL);
        assert_eq!(copy.as_slice(), &[None, None, None, Some("three")]);
    }

    #[test]
    fn integer_arrays_zero_fill_and_add_prefix_like_c_macros() {
        let mut collect = PDIntArray::new_int(2, GROW_EXPONENTIAL);
        let mut data = PDIntArray::new_int(2, GROW_EXPONENTIAL);
        collect.assign(0, 3);
        collect.assign(3, 7);
        data.assign(0, 4);
        data.assign(4, 9);

        collect.add_prefix(&mut data, 5);
        assert_eq!(collect.element_int(0), 7);
        assert_eq!(collect.element_int(3), 7);
        assert_eq!(collect.element_int(4), 9);
        assert_eq!(collect.size(), 8);
        assert_eq!(data.size(), 8);
    }

    #[test]
    fn integer_increment_and_store_use_zero_as_unused_sentinel() {
        let mut array = PDIntArray::new_int(2, GROW_EXPONENTIAL);
        assert_eq!(array.store_int(11), 0);
        assert_eq!(array.store_int(22), 1);
        assert_eq!(array.inc_int(3, 5), Some(5));
        assert_eq!(array.size(), 4);
        assert!(array.delete_int(1));
        assert_eq!(array.element_int(1), 0);
    }

    #[test]
    fn generic_array_supports_custom_default_values() {
        let mut array = PDArray::with_default(1, 3, -1);
        assert_eq!(array.element(4), Some(&-1));
        assert_eq!(array.size(), 6);
        array.assign(4, 10);
        assert_eq!(array.element(4), Some(&10));
    }
}
