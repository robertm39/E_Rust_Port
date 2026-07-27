use crate::basics::pdarrays::GROW_EXPONENTIAL;

pub type PDRangeArrIndex = isize;
pub type PDRangeArrInt = i64;
pub type PDIntRangeArr = PDRangeArr<PDRangeArrInt>;
pub type PDPointerRangeArr<T> = PDRangeArr<Option<T>>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PDRangeArr<T> {
    offset: PDRangeArrIndex,
    size: usize,
    grow: usize,
    default: T,
    array: Vec<T>,
}

impl<T: Clone> PDRangeArr<T> {
    /// Return an initialized range array covering `idx`.
    ///
    /// The C constructor uses `grow` as the initial size when it is non-zero
    /// and otherwise starts with one slot.
    #[must_use]
    pub fn with_default(idx: PDRangeArrIndex, grow: usize, default: T) -> Self {
        let size = if grow == GROW_EXPONENTIAL { 1 } else { grow };
        Self {
            offset: idx,
            size,
            grow,
            default: default.clone(),
            array: vec![default; size],
        }
    }

    #[must_use]
    pub const fn low_key(&self) -> PDRangeArrIndex {
        self.offset
    }

    #[must_use]
    pub fn limit_key(&self) -> PDRangeArrIndex {
        self.offset
            .checked_add(index_from_usize(self.size))
            .unwrap_or(PDRangeArrIndex::MAX)
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

    #[must_use]
    pub fn index_is_covered(&self, idx: PDRangeArrIndex) -> bool {
        idx >= self.low_key() && idx < self.limit_key()
    }

    /// Enlarge the range enough to cover `idx`.
    ///
    /// # Panics
    ///
    /// Panics if computing the new logical range overflows `usize` or
    /// `isize`.
    pub fn enlarge(&mut self, idx: PDRangeArrIndex) {
        if self.index_is_covered(idx) {
            return;
        }
        self.enlarge_c_raw(idx);
    }

    /// Enlarge using the exported C `PDRangeArrEnlarge` branch choice.
    ///
    /// Unlike [`PDRangeArr::enlarge`], this does not return early for already
    /// covered indices; it dispatches solely on `idx < offset`, matching the C
    /// helper's assertion-sensitive public surface.
    ///
    /// # Panics
    ///
    /// Panics if computing the new logical range overflows `usize` or
    /// `isize`, or if the C-shaped branch choice fails to cover `idx`.
    pub fn enlarge_c_raw(&mut self, idx: PDRangeArrIndex) {
        if idx < self.offset {
            self.expand_down(idx);
        } else {
            self.expand_up(idx);
        }
        assert!(
            self.index_is_covered(idx),
            "PDRangeArrEnlarge failed to cover index {idx}"
        );
    }

    /// Return a mutable slot for `idx`, growing the covered range if needed.
    ///
    /// # Panics
    ///
    /// Panics if range growth overflows the represented index or capacity
    /// space, or if the post-growth coverage invariant is broken.
    pub fn element_ref(&mut self, idx: PDRangeArrIndex) -> &mut T {
        self.enlarge(idx);
        let index = self.storage_index_or_panic(idx, "PDRangeArrElementRef");
        self.array
            .get_mut(index)
            .unwrap_or_else(|| panic!("PDRangeArrElementRef lost covered slot"))
    }

    /// Return the slot for `idx`, growing the covered range if needed.
    ///
    /// # Panics
    ///
    /// Panics if range growth overflows the represented index or capacity
    /// space, or if the post-growth coverage invariant is broken.
    pub fn element(&mut self, idx: PDRangeArrIndex) -> &T {
        self.enlarge(idx);
        let index = self.storage_index_or_panic(idx, "PDRangeArrElement");
        self.array
            .get(index)
            .unwrap_or_else(|| panic!("PDRangeArrElement lost covered slot"))
    }

    #[must_use]
    pub fn existing_element(&self, idx: PDRangeArrIndex) -> Option<&T> {
        let index = self.storage_index(idx)?;
        self.array.get(index)
    }

    /// Assign `value` to `idx`, growing the covered range if needed.
    ///
    /// # Panics
    ///
    /// Panics if range growth overflows the represented index or capacity
    /// space, or if the post-growth coverage invariant is broken.
    pub fn assign(&mut self, idx: PDRangeArrIndex, value: T) {
        let element = self.element_ref(idx);
        *element = value;
    }

    pub fn delete(&mut self, idx: PDRangeArrIndex) -> bool {
        let Some(index) = self.storage_index(idx) else {
            return false;
        };
        self.array[index] = self.default.clone();
        true
    }

    #[must_use]
    pub fn copy_array(&self) -> Self {
        Self {
            offset: self.offset,
            size: self.size,
            grow: self.grow,
            default: self.default.clone(),
            array: self.array.clone(),
        }
    }

    fn expand_down(&mut self, idx: PDRangeArrIndex) {
        let old_size = self.size;
        let old_offset = self.offset;
        let distance = checked_distance(old_offset, idx);
        let min_size = checked_add_usize(distance, old_size);
        let new_size = range_arr_size(min_size, old_size, self.grow);
        let added = checked_sub_usize(new_size, old_size);
        let new_offset = old_offset
            .checked_sub(index_from_usize(added))
            .unwrap_or_else(|| panic!("PDRangeArr offset overflow"));

        let mut new_array = vec![self.default.clone(); new_size];
        let old_start = checked_distance(old_offset, new_offset);
        new_array[old_start..old_start + old_size].clone_from_slice(&self.array);

        self.offset = new_offset;
        self.size = new_size;
        self.array = new_array;
    }

    fn expand_up(&mut self, idx: PDRangeArrIndex) {
        let min_size = checked_add_usize(checked_distance(idx, self.offset), 1);
        let new_size = range_arr_size(min_size, self.size, self.grow);
        self.array.resize(new_size, self.default.clone());
        self.size = new_size;
    }

    fn storage_index(&self, idx: PDRangeArrIndex) -> Option<usize> {
        if !self.index_is_covered(idx) {
            return None;
        }
        usize::try_from(idx.checked_sub(self.offset)?).ok()
    }

    fn storage_index_or_panic(&self, idx: PDRangeArrIndex, caller: &str) -> usize {
        self.storage_index(idx)
            .unwrap_or_else(|| panic!("{caller} failed to cover index {idx}"))
    }
}

impl<T: Clone> PDPointerRangeArr<T> {
    #[must_use]
    pub fn new_pointer(idx: PDRangeArrIndex, grow: usize) -> Self {
        Self::with_default(idx, grow, None)
    }

    #[must_use]
    pub fn members(&self) -> usize {
        self.array.iter().filter(|value| value.is_some()).count()
    }

    pub fn delete_pointer(&mut self, idx: PDRangeArrIndex) -> bool {
        self.delete(idx)
    }
}

impl PDIntRangeArr {
    #[must_use]
    pub fn new_int(idx: PDRangeArrIndex, grow: usize) -> Self {
        Self::with_default(idx, grow, 0)
    }

    pub fn delete_int(&mut self, idx: PDRangeArrIndex) -> bool {
        self.delete(idx)
    }

    /// Increment `idx` by `value`, growing the covered range if needed.
    ///
    /// # Panics
    ///
    /// Panics if range growth overflows the represented index or capacity
    /// space, or if the post-growth coverage invariant is broken.
    pub fn inc_int(&mut self, idx: PDRangeArrIndex, value: PDRangeArrInt) -> PDRangeArrInt {
        let element = self.element_ref(idx);
        *element += value;
        *element
    }

    /// Return the integer slot for `idx`, growing the covered range if needed.
    ///
    /// # Panics
    ///
    /// Panics if range growth overflows the represented index or capacity
    /// space, or if the post-growth coverage invariant is broken.
    pub fn element_int(&mut self, idx: PDRangeArrIndex) -> PDRangeArrInt {
        *self.element(idx)
    }
}

fn range_arr_size(min_size: usize, size: usize, grow: usize) -> usize {
    if grow == GROW_EXPONENTIAL {
        let mut new_size = size;
        while new_size <= min_size {
            let Some(doubled) = new_size.checked_mul(2) else {
                panic!("PDRangeArr capacity overflow");
            };
            new_size = doubled;
        }
        new_size
    } else {
        let Some(block) = min_size
            .checked_div(grow)
            .and_then(|value| value.checked_add(1))
        else {
            panic!("PDRangeArr capacity overflow");
        };
        let Some(new_size) = block.checked_mul(grow) else {
            panic!("PDRangeArr capacity overflow");
        };
        new_size
    }
}

fn checked_distance(high: PDRangeArrIndex, low: PDRangeArrIndex) -> usize {
    high.checked_sub(low)
        .and_then(|value| usize::try_from(value).ok())
        .unwrap_or_else(|| panic!("PDRangeArr index overflow"))
}

fn checked_add_usize(left: usize, right: usize) -> usize {
    left.checked_add(right)
        .unwrap_or_else(|| panic!("PDRangeArr capacity overflow"))
}

fn checked_sub_usize(left: usize, right: usize) -> usize {
    left.checked_sub(right)
        .unwrap_or_else(|| panic!("PDRangeArr capacity overflow"))
}

fn index_from_usize(value: usize) -> PDRangeArrIndex {
    match PDRangeArrIndex::try_from(value) {
        Ok(converted) => converted,
        Err(error) => panic!("PDRangeArr index overflow: {error}"),
    }
}

#[cfg(test)]
mod tests {
    use super::{PDIntRangeArr, PDPointerRangeArr};
    use crate::basics::pdarrays::GROW_EXPONENTIAL;

    #[test]
    fn exponential_growth_expands_up_and_down_with_c_offset_rule() {
        let mut array = PDPointerRangeArr::new_pointer(10, GROW_EXPONENTIAL);
        assert_eq!(
            (array.low_key(), array.limit_key(), array.size()),
            (10, 11, 1)
        );

        array.assign(12, Some("up"));
        assert_eq!(
            (array.low_key(), array.limit_key(), array.size()),
            (10, 14, 4)
        );
        assert_eq!(array.existing_element(12), Some(&Some("up")));

        array.assign(7, Some("down"));
        assert_eq!(
            (array.low_key(), array.limit_key(), array.size()),
            (6, 14, 8)
        );
        assert_eq!(array.existing_element(7), Some(&Some("down")));
        assert_eq!(array.existing_element(12), Some(&Some("up")));
        assert_eq!(array.existing_element(6), Some(&None));
    }

    #[test]
    fn fixed_growth_uses_covering_multiples_and_shifts_down() {
        let mut array = PDPointerRangeArr::<usize>::new_pointer(0, 5);
        assert_eq!(
            (array.low_key(), array.limit_key(), array.size()),
            (0, 5, 5)
        );

        array.assign(7, Some(7));
        assert_eq!(
            (array.low_key(), array.limit_key(), array.size()),
            (0, 10, 10)
        );

        array.assign(-3, Some(3));
        assert_eq!(
            (array.low_key(), array.limit_key(), array.size()),
            (-5, 10, 15)
        );
        assert_eq!(array.existing_element(-3), Some(&Some(3)));
        assert_eq!(array.existing_element(7), Some(&Some(7)));
        assert_eq!(array.existing_element(-5), Some(&None));
    }

    #[test]
    fn element_access_expands_and_returns_slots_like_c_macros() {
        let mut array = PDPointerRangeArr::<usize>::new_pointer(0, 2);

        assert_eq!(array.element(-3), &None);
        assert_eq!((array.low_key(), array.limit_key()), (-4, 2));

        *array.element_ref(4) = Some(4);
        assert_eq!((array.low_key(), array.limit_key()), (-4, 6));
        assert_eq!(array.existing_element(4), Some(&Some(4)));
    }

    #[test]
    fn raw_enlarge_uses_c_branch_choice_even_for_covered_indices() {
        let mut array = PDPointerRangeArr::new_pointer(0, GROW_EXPONENTIAL);
        array.assign(3, Some("high"));
        array.assign(0, Some("low"));
        let before = array.copy_array();

        array.enlarge_c_raw(1);

        assert_eq!(array.low_key(), before.low_key());
        assert_eq!(array.limit_key(), before.limit_key());
        assert_eq!(array.size(), before.size());
        assert_eq!(array.as_slice(), before.as_slice());

        array.enlarge_c_raw(-2);
        assert_eq!((array.low_key(), array.limit_key()), (-8, 8));
        assert_eq!(array.existing_element(0), Some(&Some("low")));
        assert_eq!(array.existing_element(3), Some(&Some("high")));
    }

    #[test]
    fn delete_only_clears_covered_indices() {
        let mut array = PDPointerRangeArr::new_pointer(4, GROW_EXPONENTIAL);
        array.assign(4, Some("value"));
        assert!(!array.delete_pointer(9));
        assert_eq!(array.size(), 1);
        assert!(array.delete_pointer(4));
        assert_eq!(array.existing_element(4), Some(&None));
    }

    #[test]
    fn members_count_non_null_pointer_slots() {
        let mut array = PDPointerRangeArr::new_pointer(-1, 3);
        array.assign(-1, Some("a"));
        array.assign(0, Some("b"));
        array.assign(4, Some("c"));
        assert_eq!(array.members(), 3);
    }

    #[test]
    fn integer_range_arrays_zero_fill_and_increment() {
        let mut array = PDIntRangeArr::new_int(0, GROW_EXPONENTIAL);
        assert_eq!(array.inc_int(-2, 5), 5);
        assert_eq!((array.low_key(), array.limit_key()), (-3, 1));
        assert_eq!(array.inc_int(3, 7), 7);
        assert_eq!((array.low_key(), array.limit_key()), (-3, 5));
        assert_eq!(array.element_int(-2), 5);
        assert_eq!(array.element_int(3), 7);
        assert_eq!(array.element_int(0), 0);
        assert!(array.delete_int(3));
        assert_eq!(array.element_int(3), 0);
    }

    #[test]
    fn copy_preserves_offset_size_growth_and_contents() {
        let mut array = PDPointerRangeArr::new_pointer(2, 4);
        array.assign(-2, Some("low"));
        array.assign(5, Some("high"));

        let copy = array.copy_array();
        assert_eq!(copy.low_key(), array.low_key());
        assert_eq!(copy.limit_key(), array.limit_key());
        assert_eq!(copy.grow(), 4);
        assert_eq!(copy.as_slice(), array.as_slice());
    }
}
