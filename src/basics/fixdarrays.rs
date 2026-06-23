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

    #[must_use]
    pub fn element(&self, index: usize) -> Option<FixedDArrayInt> {
        self.array.get(index).copied()
    }

    pub fn assign(&mut self, index: usize, value: FixedDArrayInt) -> bool {
        let Some(element) = self.array.get_mut(index) else {
            return false;
        };
        *element = value;
        true
    }

    pub fn initialize(&mut self, value: FixedDArrayInt) {
        self.array.fill(value);
    }

    pub fn add_from(&mut self, s1: &Self, s2: &Self) -> bool {
        self.mul_add_from(s1, 1, s2, 1)
    }

    pub fn sub_from(&mut self, s1: &Self, s2: &Self) -> bool {
        self.mul_add_from(s1, 1, s2, -1)
    }

    pub fn mul_add_from(
        &mut self,
        s1: &Self,
        f1: FixedDArrayInt,
        s2: &Self,
        f2: FixedDArrayInt,
    ) -> bool {
        if !self.compatible_with(s1, s2) {
            return false;
        }
        for ((dest, left), right) in self.array.iter_mut().zip(&s1.array).zip(&s2.array) {
            *dest = f1 * *left + f2 * *right;
        }
        true
    }

    pub fn max_from(&mut self, s1: &Self, s2: &Self) -> bool {
        if !self.compatible_with(s1, s2) {
            return false;
        }
        for ((dest, left), right) in self.array.iter_mut().zip(&s1.array).zip(&s2.array) {
            *dest = (*left).max(*right);
        }
        true
    }

    pub fn min_from(&mut self, s1: &Self, s2: &Self) -> bool {
        if !self.compatible_with(s1, s2) {
            return false;
        }
        for ((dest, left), right) in self.array.iter_mut().zip(&s1.array).zip(&s2.array) {
            *dest = (*left).min(*right);
        }
        true
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

    fn compatible_with(&self, s1: &Self, s2: &Self) -> bool {
        s1.size() == self.size() && s2.size() == self.size()
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
        assert_eq!(array.element(1), Some(7));
        assert_eq!(array.element(9), None);
        assert!(array.assign(1, 4));
        assert_eq!(array.as_slice(), &[7, 4, 7]);
    }

    #[test]
    fn componentwise_arithmetic_matches_c_helpers() {
        let left = array(&[1, 4, -2]);
        let right = array(&[3, -5, 6]);
        let mut dest = FixedDArray::new(3);

        assert!(dest.add_from(&left, &right));
        assert_eq!(dest.as_slice(), &[4, -1, 4]);

        assert!(dest.sub_from(&left, &right));
        assert_eq!(dest.as_slice(), &[-2, 9, -8]);

        assert!(dest.mul_add_from(&left, 2, &right, -3));
        assert_eq!(dest.as_slice(), &[-7, 23, -22]);
    }

    #[test]
    fn componentwise_min_max_and_size_checks_match_asserted_contract() {
        let left = array(&[1, 4, -2]);
        let right = array(&[3, -5, 6]);
        let mut dest = FixedDArray::new(3);

        assert!(dest.max_from(&left, &right));
        assert_eq!(dest.as_slice(), &[3, 4, 6]);

        assert!(dest.min_from(&left, &right));
        assert_eq!(dest.as_slice(), &[1, -5, -2]);

        let wrong_size = FixedDArray::new(2);
        assert!(!dest.add_from(&left, &wrong_size));
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
