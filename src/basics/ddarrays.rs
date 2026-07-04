pub type DDArrayIndex = isize;

use std::fmt::Write as _;

#[derive(Clone, Debug, PartialEq)]
pub struct DDArray {
    size: usize,
    grow: usize,
    array: Vec<f64>,
}

impl DDArray {
    /// Return an initialized dynamic double array filled with `0.0`.
    ///
    /// # Panics
    ///
    /// Panics when `init_size` or `grow` is zero. The C implementation asserts
    /// both invariants.
    #[must_use]
    pub fn new(init_size: usize, grow: usize) -> Self {
        assert!(init_size > 0, "DDArray initial size must be non-zero");
        assert!(grow > 0, "DDArray growth block must be non-zero");
        Self {
            size: init_size,
            grow,
            array: vec![0.0; init_size],
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
    pub fn as_slice(&self) -> &[f64] {
        &self.array
    }

    /// Enlarge the array enough to cover `idx`.
    ///
    /// # Panics
    ///
    /// Panics when `idx` is negative, or if computing the new logical size
    /// overflows `usize`.
    pub fn enlarge(&mut self, idx: DDArrayIndex) {
        let index = index_or_panic(idx, "DDArrayEnlarge");
        self.enlarge_index(index);
    }

    /// Enlarge using the exported C `DDArayEnlarge` target-size calculation.
    ///
    /// This helper deliberately does not return early for already covered
    /// indices. Direct C callers are expected to pass an uncovered index; when
    /// the C calculation would produce a smaller allocation and then overrun it
    /// during `memcpy`, Rust reports that broken precondition as a panic.
    ///
    /// # Panics
    ///
    /// Panics when `idx` is negative, if computing the new logical size
    /// overflows `usize`, or if the C target size is smaller than the current
    /// allocation.
    pub fn enlarge_c_raw(&mut self, idx: DDArrayIndex) {
        let index = index_or_panic(idx, "DDArayEnlarge");
        let new_size = self.new_size_for(index);
        assert!(
            new_size >= self.size,
            "DDArayEnlarge target size is smaller than current size"
        );
        self.array.resize(new_size, 0.0);
        self.size = new_size;
    }

    fn enlarge_index(&mut self, index: usize) {
        if index < self.size {
            return;
        }

        self.enlarge_c_raw(index_to_dd(index));
    }

    fn new_size_for(&self, index: usize) -> usize {
        let Some(block) = index
            .checked_div(self.grow)
            .and_then(|value| value.checked_add(1))
        else {
            panic!("DDArray capacity overflow");
        };
        let Some(new_size) = block.checked_mul(self.grow) else {
            panic!("DDArray capacity overflow");
        };
        new_size
    }

    /// Return a mutable reference to `idx`, enlarging the array if needed.
    ///
    /// # Panics
    ///
    /// Panics when `idx` is negative, matching the C `DDArrayElementRef`
    /// assertion.
    pub fn element_ref(&mut self, idx: DDArrayIndex) -> &mut f64 {
        let index = index_or_panic(idx, "DDArrayElementRef");
        self.enlarge_index(index);
        &mut self.array[index]
    }

    /// Return the value at `idx`, enlarging the array if needed.
    ///
    /// # Panics
    ///
    /// Panics when `idx` is negative, matching the C `DDArrayElement`
    /// assertion through `DDArrayElementRef`.
    #[must_use]
    pub fn element(&mut self, idx: DDArrayIndex) -> f64 {
        let index = index_or_panic(idx, "DDArrayElement");
        self.enlarge_index(index);
        self.array[index]
    }

    #[must_use]
    pub fn existing_element(&self, idx: DDArrayIndex) -> Option<f64> {
        let index = checked_index(idx)?;
        self.array.get(index).copied()
    }

    /// Assign `value` to `idx`, enlarging the array if needed.
    ///
    /// # Panics
    ///
    /// Panics when `idx` is negative, matching the C `DDArrayAssign`
    /// assertion through `DDArrayElementRef`.
    pub fn assign(&mut self, idx: DDArrayIndex, value: f64) {
        let element = self.element_ref(idx);
        *element = value;
    }

    pub fn add_prefix(&mut self, data: &mut Self, limit: usize) {
        for index in 0..limit {
            let idx = index_to_dd(index);
            let old = self.element(idx);
            let new = data.element(idx);
            self.assign(idx, old + new);
        }
    }

    #[must_use]
    pub fn debug_print_string(&mut self, size: usize) -> String {
        let mut result = String::new();
        for index in 0..size {
            let value = self.element(index_to_dd(index));
            let write_result = write!(&mut result, " {value:5.3} ");
            debug_assert!(write_result.is_ok());
            if (index + 1).is_multiple_of(10) {
                result.push('\n');
            }
        }
        result.push('\n');
        result
    }

    /// Return the C `DDArraySelectPart` partition value for the first `size`
    /// elements.
    ///
    /// # Panics
    ///
    /// Panics when `part` is outside the inclusive `0.0..=1.0` range, when
    /// `part` is NaN, when `size` is zero, or when `size` exceeds the current
    /// logical allocation. These are assertion failures in the C helper.
    #[must_use]
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss,
        clippy::cast_sign_loss
    )]
    pub fn select_part(&mut self, part: f64, size: usize) -> f64 {
        assert!(
            (0.0..=1.0).contains(&part),
            "DDArraySelectPart part must be in the inclusive 0.0..=1.0 range"
        );
        assert!(size > 0, "DDArraySelectPart size must be non-zero");
        assert!(
            size <= self.size,
            "DDArraySelectPart size exceeds allocated array size"
        );

        let rank_float = (size - 1) as f64 * part;
        let rank1 = rank_float as usize;
        let rank2 = (rank_float + 0.5) as usize;
        let mut start = 0_usize;
        let mut end = size - 1;

        while start != end {
            let midpoint = usize::midpoint(start, end);
            let pivot = (self.array[start] + self.array[midpoint] + self.array[end]) / 3.0;
            let mut left = start;
            let mut right = end;

            while left != right {
                while left < right && self.array[left] <= pivot {
                    left += 1;
                }
                while right > left && self.array[right] > pivot {
                    right -= 1;
                }
                self.array.swap(left, right);
            }

            if left > rank1 {
                end = left.saturating_sub(1);
            } else {
                start = left;
            }
        }

        let second = if rank2 == rank1 {
            self.array[start]
        } else {
            assert!(
                rank1 != size - 1,
                "DDArraySelectPart second rank must be inside array"
            );
            let mut minimum = self.array[start + 1];
            for index in start + 1..size {
                minimum = minimum.min(self.array[index]);
            }
            minimum
        };

        f64::midpoint(self.array[start], second)
    }
}

fn checked_index(idx: DDArrayIndex) -> Option<usize> {
    usize::try_from(idx).ok()
}

fn index_or_panic(idx: DDArrayIndex, caller: &str) -> usize {
    assert!(idx >= 0, "{caller} called with a negative index");
    match usize::try_from(idx) {
        Ok(value) => value,
        Err(error) => panic!("{caller} index overflow: {error}"),
    }
}

fn index_to_dd(index: usize) -> DDArrayIndex {
    match DDArrayIndex::try_from(index) {
        Ok(value) => value,
        Err(_error) => DDArrayIndex::MAX,
    }
}

#[cfg(test)]
mod tests {
    use super::{index_to_dd, DDArray};

    fn assert_same_f64(actual: f64, expected: f64) {
        assert_eq!(actual.to_bits(), expected.to_bits());
    }

    #[test]
    fn initializes_to_zero_and_grows_by_fixed_blocks() {
        let mut array = DDArray::new(3, 4);
        assert_eq!(array.size(), 3);
        assert_eq!(array.grow(), 4);
        assert_same_f64(array.element(2), 0.0);
        array.assign(6, 6.5);
        assert_eq!(array.size(), 8);
        assert_eq!(array.existing_element(6), Some(6.5));
        assert_eq!(array.existing_element(7), Some(0.0));
    }

    #[test]
    fn raw_enlarge_uses_misspelled_c_target_calculation_without_coverage_guard() {
        let mut array = DDArray::new(3, 4);

        array.enlarge_c_raw(6);
        assert_eq!(array.size(), 8);
        assert_eq!(array.existing_element(6), Some(0.0));

        array.enlarge_c_raw(7);
        assert_eq!(array.size(), 8);
    }

    #[test]
    #[should_panic(expected = "DDArayEnlarge target size is smaller than current size")]
    fn raw_enlarge_rejects_direct_call_that_would_underallocate_in_c() {
        let mut array = DDArray::new(3, 4);
        array.enlarge_c_raw(8);
        assert_eq!(array.size(), 12);

        array.enlarge_c_raw(1);
    }

    #[test]
    fn add_prefix_extends_both_arrays_like_element_macros() {
        let mut collect = DDArray::new(2, 3);
        let mut data = DDArray::new(2, 3);
        collect.assign(0, 1.5);
        data.assign(0, 2.25);
        data.assign(4, 4.0);

        collect.add_prefix(&mut data, 5);
        assert_eq!(collect.existing_element(0), Some(3.75));
        assert_eq!(collect.existing_element(4), Some(4.0));
        assert_eq!(collect.size(), 6);
        assert_eq!(data.size(), 6);
    }

    #[test]
    fn select_part_matches_c_partition_result_and_mutates_order() {
        let mut array = DDArray::new(6, 3);
        for (index, value) in [9.0, 1.0, 5.0, 3.0, 7.0, 11.0].into_iter().enumerate() {
            array.assign(index_to_dd(index), value);
        }

        assert_same_f64(array.select_part(0.5, 6), 6.0);
        let mut values = array.as_slice()[..6].to_vec();
        values.sort_by(f64::total_cmp);
        assert_eq!(values, vec![1.0, 3.0, 5.0, 7.0, 9.0, 11.0]);
    }

    #[test]
    #[should_panic(expected = "DDArraySelectPart part must be in the inclusive 0.0..=1.0 range")]
    fn select_part_panics_on_low_part_like_c_assertion() {
        let mut array = DDArray::new(2, 2);
        let _value = array.select_part(-0.1, 2);
    }

    #[test]
    #[should_panic(expected = "DDArraySelectPart part must be in the inclusive 0.0..=1.0 range")]
    fn select_part_panics_on_nan_part_like_c_assertion() {
        let mut array = DDArray::new(2, 2);
        let _value = array.select_part(f64::NAN, 2);
    }

    #[test]
    #[should_panic(expected = "DDArraySelectPart size must be non-zero")]
    fn select_part_panics_on_zero_size_like_c_assertion() {
        let mut array = DDArray::new(2, 2);
        let _value = array.select_part(0.5, 0);
    }

    #[test]
    #[should_panic(expected = "DDArraySelectPart size exceeds allocated array size")]
    fn select_part_panics_on_oversized_request_like_c_assertion() {
        let mut array = DDArray::new(2, 2);
        let _value = array.select_part(0.5, 3);
    }

    #[test]
    fn debug_print_matches_c_format_and_uses_mutating_element_access() {
        let mut array = DDArray::new(3, 4);
        array.assign(0, 1.0);
        array.assign(1, -2.25);
        array.assign(2, 12.5);

        assert_eq!(
            array.debug_print_string(12),
            " 1.000  -2.250  12.500  0.000  0.000  0.000  0.000  0.000  0.000  0.000 \n 0.000  0.000 \n"
        );
        assert_eq!(array.size(), 12);
        assert_eq!(array.existing_element(11), Some(0.0));
    }

    #[test]
    #[should_panic(expected = "DDArrayElementRef called with a negative index")]
    fn element_ref_panics_on_negative_index_like_c_assertion() {
        let mut array = DDArray::new(2, 2);
        let _value = array.element_ref(-1);
    }

    #[test]
    #[should_panic(expected = "DDArrayElement called with a negative index")]
    fn element_panics_on_negative_index_like_c_assertion() {
        let mut array = DDArray::new(2, 2);
        let _value = array.element(-1);
    }
}
