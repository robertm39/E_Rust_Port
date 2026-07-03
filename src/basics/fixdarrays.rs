use std::fmt::Write as _;

pub type FixedDArrayInt = i64;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FixedDArray {
    array: Vec<FixedDArrayInt>,
}

impl FixedDArray {
    #[must_use]
    pub fn new(size: usize) -> Self {
        Self {
            array: vec![0; size],
        }
    }

    #[must_use]
    pub fn size(&self) -> usize {
        self.array.len()
    }

    #[must_use]
    pub fn as_slice(&self) -> &[FixedDArrayInt] {
        &self.array
    }

    #[must_use]
    pub fn as_mut_slice(&mut self) -> &mut [FixedDArrayInt] {
        &mut self.array
    }

    /// Return the element at `index`.
    ///
    /// # Panics
    ///
    /// Panics when `index` is outside the fixed array. The C code exposes the
    /// raw fixed-size payload and relies on callers to satisfy this invariant.
    #[must_use]
    pub fn element(&self, index: usize) -> FixedDArrayInt {
        *self
            .array
            .get(index)
            .unwrap_or_else(|| panic!("FixedDArrayElement called with out-of-range index {index}"))
    }

    /// Assign `value` at `index`.
    ///
    /// # Panics
    ///
    /// Panics when `index` is outside the fixed array. The C code exposes the
    /// raw fixed-size payload and relies on callers to satisfy this invariant.
    pub fn assign(&mut self, index: usize, value: FixedDArrayInt) {
        let element = self
            .array
            .get_mut(index)
            .unwrap_or_else(|| panic!("FixedDArrayAssign called with out-of-range index {index}"));
        *element = value;
    }

    pub fn initialize(&mut self, value: FixedDArrayInt) {
        self.array.fill(value);
    }

    /// Store the component-wise sum of `s1` and `s2`.
    ///
    /// # Panics
    ///
    /// Panics if either source size differs from the destination size, matching
    /// the C `FixedDArrayAdd` assertions.
    pub fn add_from(&mut self, s1: &Self, s2: &Self) {
        self.mul_add_from(s1, 1, s2, 1);
    }

    /// Store the component-wise difference of `s1` and `s2`.
    ///
    /// # Panics
    ///
    /// Panics if either source size differs from the destination size, matching
    /// the C `FixedDArraySub`/`FixedDArrayMulAdd` assertions.
    pub fn sub_from(&mut self, s1: &Self, s2: &Self) {
        self.mul_add_from(s1, 1, s2, -1);
    }

    /// Store the component-wise weighted sum of `s1` and `s2`.
    ///
    /// # Panics
    ///
    /// Panics if either source size differs from the destination size, matching
    /// the C `FixedDArrayMulAdd` assertions.
    pub fn mul_add_from(&mut self, s1: &Self, f1: FixedDArrayInt, s2: &Self, f2: FixedDArrayInt) {
        self.assert_compatible_with(s1, s2, "FixedDArrayMulAdd");
        for ((dest, left), right) in self.array.iter_mut().zip(&s1.array).zip(&s2.array) {
            *dest = f1 * *left + f2 * *right;
        }
    }

    /// Store the component-wise maximum of `s1` and `s2`.
    ///
    /// # Panics
    ///
    /// Panics if either source size differs from the destination size, matching
    /// the C `FixedDArrayMax` assertions.
    pub fn max_from(&mut self, s1: &Self, s2: &Self) {
        self.assert_compatible_with(s1, s2, "FixedDArrayMax");
        for ((dest, left), right) in self.array.iter_mut().zip(&s1.array).zip(&s2.array) {
            *dest = (*left).max(*right);
        }
    }

    /// Store the component-wise minimum of `s1` and `s2`.
    ///
    /// # Panics
    ///
    /// Panics if either source size differs from the destination size, matching
    /// the C `FixedDArrayMin` assertions.
    pub fn min_from(&mut self, s1: &Self, s2: &Self) {
        self.assert_compatible_with(s1, s2, "FixedDArrayMin");
        for ((dest, left), right) in self.array.iter_mut().zip(&s1.array).zip(&s2.array) {
            *dest = (*left).min(*right);
        }
    }

    #[must_use]
    pub fn print_string(&self) -> String {
        let mut result = format!("% Size {}:", self.size());
        for value in &self.array {
            let write_result = write!(&mut result, " {value:4}");
            debug_assert!(write_result.is_ok());
        }
        result.push('\n');
        result
    }

    #[must_use]
    pub fn copy_array(&self) -> Self {
        self.clone()
    }

    fn assert_compatible_with(&self, s1: &Self, s2: &Self, caller: &str) {
        assert!(
            s1.size() == self.size(),
            "{caller} source 1 size {} differs from destination size {}",
            s1.size(),
            self.size()
        );
        assert!(
            s2.size() == self.size(),
            "{caller} source 2 size {} differs from destination size {}",
            s2.size(),
            self.size()
        );
    }
}

#[cfg(test)]
mod tests {
    use super::FixedDArray;

    fn array(values: &[i64]) -> FixedDArray {
        let mut array = FixedDArray::new(values.len());
        array.as_mut_slice().copy_from_slice(values);
        array
    }

    #[test]
    fn allocates_queryable_zeroed_storage_and_initializes() {
        let mut array = FixedDArray::new(3);
        assert_eq!(array.size(), 3);
        assert_eq!(array.as_slice(), &[0, 0, 0]);
        array.initialize(7);
        assert_eq!(array.as_slice(), &[7, 7, 7]);
        assert_eq!(array.element(1), 7);
        array.assign(1, 4);
        assert_eq!(array.as_slice(), &[7, 4, 7]);
    }

    #[test]
    #[should_panic(expected = "FixedDArrayElement called with out-of-range index 9")]
    fn element_panics_on_out_of_range_index_like_c_raw_access_precondition() {
        let array = FixedDArray::new(3);
        let _value = array.element(9);
    }

    #[test]
    #[should_panic(expected = "FixedDArrayAssign called with out-of-range index 3")]
    fn assign_panics_on_out_of_range_index_like_c_raw_access_precondition() {
        let mut array = FixedDArray::new(3);
        array.assign(3, 1);
    }

    #[test]
    fn componentwise_arithmetic_matches_c_helpers() {
        let left = array(&[1, 4, -2]);
        let right = array(&[3, -5, 6]);
        let mut dest = FixedDArray::new(3);

        dest.add_from(&left, &right);
        assert_eq!(dest.as_slice(), &[4, -1, 4]);

        dest.sub_from(&left, &right);
        assert_eq!(dest.as_slice(), &[-2, 9, -8]);

        dest.mul_add_from(&left, 2, &right, -3);
        assert_eq!(dest.as_slice(), &[-7, 23, -22]);
    }

    #[test]
    fn componentwise_min_max_and_size_checks_match_asserted_contract() {
        let left = array(&[1, 4, -2]);
        let right = array(&[3, -5, 6]);
        let mut dest = FixedDArray::new(3);

        dest.max_from(&left, &right);
        assert_eq!(dest.as_slice(), &[3, 4, 6]);

        dest.min_from(&left, &right);
        assert_eq!(dest.as_slice(), &[1, -5, -2]);
    }

    #[test]
    #[should_panic(expected = "FixedDArrayMulAdd source 2 size 2 differs from destination size 3")]
    fn componentwise_arithmetic_panics_on_size_mismatch_like_c_assertion() {
        let left = array(&[1, 4, -2]);
        let wrong_size = FixedDArray::new(2);
        let mut dest = FixedDArray::new(3);
        dest.add_from(&left, &wrong_size);
    }

    #[test]
    fn copy_is_independent_and_printing_matches_c_shape() {
        let mut original = array(&[1, 20, -3]);
        let copy = original.copy_array();
        original.assign(0, 99);

        assert_eq!(copy.as_slice(), &[1, 20, -3]);
        assert_eq!(copy.print_string(), "% Size 3:    1   20   -3\n");
    }
}
