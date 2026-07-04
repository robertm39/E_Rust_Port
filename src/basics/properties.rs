use std::ops::{BitAnd, BitOr, BitXor, Not};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Properties(u64);

impl Properties {
    pub const NONE: Self = Self(0);

    #[must_use]
    pub const fn from_bits(bits: u64) -> Self {
        Self(bits)
    }

    #[must_use]
    pub const fn bits(self) -> u64 {
        self.0
    }

    pub fn set(&mut self, prop: Self) {
        self.0 |= prop.0;
    }

    pub fn delete(&mut self, prop: Self) {
        self.0 &= !prop.0;
    }

    pub fn flip(&mut self, prop: Self) {
        self.0 ^= prop.0;
    }

    pub fn assign(&mut self, selector: Self, prop: Self) {
        self.delete(selector);
        self.set(selector & prop);
    }

    #[must_use]
    pub const fn query(self, prop: Self) -> bool {
        (self.0 & prop.0) == prop.0
    }

    #[must_use]
    pub const fn is_any_set(self, prop: Self) -> bool {
        (self.0 & prop.0) != 0
    }

    #[must_use]
    pub const fn any_set(self, prop: Self) -> Self {
        self.give(prop)
    }

    #[must_use]
    pub const fn give(self, prop: Self) -> Self {
        Self(self.0 & prop.0)
    }

    #[must_use]
    pub const fn are_equiv(left: Self, right: Self, props: Self) -> bool {
        (left.0 & props.0) == (right.0 & props.0)
    }
}

impl BitOr for Properties {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitAnd for Properties {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl BitXor for Properties {
    type Output = Self;

    fn bitxor(self, rhs: Self) -> Self::Output {
        Self(self.0 ^ rhs.0)
    }
}

impl Not for Properties {
    type Output = Self;

    fn not(self) -> Self::Output {
        Self(!self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::Properties;

    const A: Properties = Properties::from_bits(0b0001);
    const B: Properties = Properties::from_bits(0b0010);
    const C: Properties = Properties::from_bits(0b0100);
    const ALL: Properties = Properties::from_bits(0b0111);

    #[test]
    fn set_delete_flip_and_query_match_c_macros() {
        let mut props = Properties::NONE;
        props.set(A | B);
        assert_eq!(props.bits(), 0b0011);
        assert!(props.query(A | B));
        assert!(props.query(Properties::NONE));
        assert!(props.is_any_set(B | C));
        assert_eq!(props.any_set(B | C), B);

        props.delete(A);
        assert_eq!(props.bits(), 0b0010);
        assert!(!props.query(A | B));
        props.flip(B | C);
        assert_eq!(props.bits(), 0b0100);
    }

    #[test]
    fn assign_clears_selected_bits_and_sets_selected_new_bits() {
        let mut props = A | B | C;
        props.assign(A | B, B | C);
        assert_eq!(props, B | C);
    }

    #[test]
    fn give_and_equivalence_mask_properties() {
        let left = A | C;
        let right = B | C;
        assert_eq!(left.give(ALL), A | C);
        assert!(Properties::are_equiv(left, right, C));
        assert!(!Properties::are_equiv(left, right, A | B));
    }
}
