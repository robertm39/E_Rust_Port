use std::ops::{BitAnd, BitOr, BitOrAssign, Not};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct EqnProperties(u64);

impl EqnProperties {
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

    #[must_use]
    pub const fn give(self, prop: Self) -> Self {
        Self(self.0 & prop.0)
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
    pub const fn are_equiv(left: Self, right: Self, prop: Self) -> bool {
        (left.0 & prop.0) == (right.0 & prop.0)
    }

    #[must_use]
    pub const fn is_oriented(self) -> bool {
        self.query(EP_IS_ORIENTED)
    }

    #[must_use]
    pub const fn is_positive(self) -> bool {
        self.query(EP_IS_POSITIVE)
    }

    #[must_use]
    pub const fn is_negative(self) -> bool {
        !self.query(EP_IS_POSITIVE)
    }

    #[must_use]
    pub const fn is_equ_literal(self) -> bool {
        self.query(EP_IS_EQU_LITERAL)
    }

    #[must_use]
    pub const fn is_maximal(self) -> bool {
        self.query(EP_IS_MAXIMAL)
    }

    #[must_use]
    pub const fn is_strictly_maximal(self) -> bool {
        self.query(EP_IS_STRICTLY_MAXIMAL)
    }

    #[must_use]
    pub const fn has_equiv(self) -> bool {
        self.query(EP_HAS_EQUIV)
    }

    #[must_use]
    pub const fn is_dominated(self) -> bool {
        self.query(EP_IS_DOMINATED)
    }

    #[must_use]
    pub const fn dominates(self) -> bool {
        self.query(EP_DOMINATES)
    }

    #[must_use]
    pub const fn is_selected(self) -> bool {
        self.query(EP_IS_SELECTED)
    }
}

impl BitOr for EqnProperties {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for EqnProperties {
    fn bitor_assign(&mut self, rhs: Self) {
        self.set(rhs);
    }
}

impl BitAnd for EqnProperties {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl Not for EqnProperties {
    type Output = Self;

    fn not(self) -> Self::Output {
        Self(!self.0)
    }
}

pub const EP_NO_PROPS: EqnProperties = EqnProperties::from_bits(0);
pub const EP_IS_POSITIVE: EqnProperties = EqnProperties::from_bits(1);
pub const EP_IS_MAXIMAL: EqnProperties = EqnProperties::from_bits(2);
pub const EP_IS_STRICTLY_MAXIMAL: EqnProperties = EqnProperties::from_bits(4);
pub const EP_IS_EQU_LITERAL: EqnProperties = EqnProperties::from_bits(8);
pub const EP_IS_ORIENTED: EqnProperties = EqnProperties::from_bits(16);
pub const EP_MAX_IS_UP_TO_DATE: EqnProperties = EqnProperties::from_bits(32);
pub const EP_HAS_EQUIV: EqnProperties = EqnProperties::from_bits(64);
pub const EP_IS_DOMINATED: EqnProperties = EqnProperties::from_bits(128);
pub const EP_DOMINATES: EqnProperties = EP_IS_DOMINATED;
pub const EP_IS_USED: EqnProperties = EqnProperties::from_bits(256);
pub const EP_GO_NATURAL: EqnProperties = EqnProperties::from_bits(512);
pub const EP_IS_SELECTED: EqnProperties = EqnProperties::from_bits(1024);
pub const EP_IS_PM_INTO_LIT: EqnProperties = EqnProperties::from_bits(2048);
pub const EP_FROM_CLAUSE_LIT: EqnProperties = EqnProperties::from_bits(4096);
pub const EP_PSEUDO_LIT: EqnProperties = EqnProperties::from_bits(8192);
pub const EP_L_PAT_MINIMAL: EqnProperties = EqnProperties::from_bits(16_384);
pub const EP_R_PAT_MINIMAL: EqnProperties = EqnProperties::from_bits(32_768);
pub const EP_IS_SPLIT_LIT: EqnProperties = EqnProperties::from_bits(65_636);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i32)]
pub enum EqnSide {
    NoSide = 0,
    LeftSide = 1,
    RightSide = 2,
    BothSides = 3,
}

pub const MAX_SIDE: EqnSide = EqnSide::LeftSide;
pub const MIN_SIDE: EqnSide = EqnSide::RightSide;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i32)]
pub enum PatEqnDirection {
    Normal = 0,
    Reverse = 1,
}

pub const EQUAL_PREDICATE: &str = "equal";

#[cfg(test)]
mod tests {
    use super::{
        EqnProperties, EqnSide, PatEqnDirection, EP_DOMINATES, EP_FROM_CLAUSE_LIT, EP_GO_NATURAL,
        EP_HAS_EQUIV, EP_IS_DOMINATED, EP_IS_EQU_LITERAL, EP_IS_MAXIMAL, EP_IS_ORIENTED,
        EP_IS_PM_INTO_LIT, EP_IS_POSITIVE, EP_IS_SELECTED, EP_IS_SPLIT_LIT, EP_IS_STRICTLY_MAXIMAL,
        EP_IS_USED, EP_L_PAT_MINIMAL, EP_MAX_IS_UP_TO_DATE, EP_NO_PROPS, EP_PSEUDO_LIT,
        EP_R_PAT_MINIMAL, EQUAL_PREDICATE, MAX_SIDE, MIN_SIDE,
    };

    #[test]
    fn constants_match_c_eqn_property_values() {
        assert_eq!(EP_NO_PROPS.bits(), 0);
        assert_eq!(EP_IS_POSITIVE.bits(), 1);
        assert_eq!(EP_IS_MAXIMAL.bits(), 2);
        assert_eq!(EP_IS_STRICTLY_MAXIMAL.bits(), 4);
        assert_eq!(EP_IS_EQU_LITERAL.bits(), 8);
        assert_eq!(EP_IS_ORIENTED.bits(), 16);
        assert_eq!(EP_MAX_IS_UP_TO_DATE.bits(), 32);
        assert_eq!(EP_HAS_EQUIV.bits(), 64);
        assert_eq!(EP_IS_DOMINATED.bits(), 128);
        assert_eq!(EP_DOMINATES.bits(), EP_IS_DOMINATED.bits());
        assert_eq!(EP_IS_USED.bits(), 256);
        assert_eq!(EP_GO_NATURAL.bits(), 512);
        assert_eq!(EP_IS_SELECTED.bits(), 1024);
        assert_eq!(EP_IS_PM_INTO_LIT.bits(), 2048);
        assert_eq!(EP_FROM_CLAUSE_LIT.bits(), 4096);
        assert_eq!(EP_PSEUDO_LIT.bits(), 8192);
        assert_eq!(EP_L_PAT_MINIMAL.bits(), 16_384);
        assert_eq!(EP_R_PAT_MINIMAL.bits(), 32_768);
        assert_eq!(EP_IS_SPLIT_LIT.bits(), 65_636);
    }

    #[test]
    fn split_literal_value_preserves_overlap_with_existing_flags() {
        let overlap = EP_IS_STRICTLY_MAXIMAL | EP_MAX_IS_UP_TO_DATE | EP_HAS_EQUIV;

        assert_eq!(EP_IS_SPLIT_LIT.give(overlap), overlap);
        assert_eq!(EP_IS_SPLIT_LIT.any_set(overlap), overlap);
        assert!(EP_IS_SPLIT_LIT.is_any_set(overlap));
    }

    #[test]
    fn property_helpers_match_c_macros() {
        let mut props = EP_NO_PROPS;
        props.set(EP_IS_POSITIVE | EP_IS_EQU_LITERAL);

        assert!(props.query(EP_IS_POSITIVE | EP_IS_EQU_LITERAL));
        assert!(props.is_any_set(EP_IS_EQU_LITERAL | EP_IS_ORIENTED));
        assert_eq!(
            props.any_set(EP_IS_EQU_LITERAL | EP_IS_ORIENTED),
            EP_IS_EQU_LITERAL
        );
        assert_eq!(props.give(EP_IS_POSITIVE | EP_IS_ORIENTED), EP_IS_POSITIVE);

        props.flip(EP_IS_POSITIVE | EP_IS_ORIENTED);
        assert!(props.is_negative());
        assert!(props.is_oriented());
        props.delete(EP_IS_ORIENTED);
        assert!(!props.is_oriented());

        props |= EP_IS_SELECTED;
        assert!(props.is_selected());
        assert_eq!((props & EP_IS_SELECTED), EP_IS_SELECTED);
        assert_eq!((!EP_NO_PROPS).give(EP_IS_MAXIMAL), EP_IS_MAXIMAL);
    }

    #[test]
    fn query_shortcuts_follow_property_bits() {
        let props = EP_IS_POSITIVE
            | EP_IS_EQU_LITERAL
            | EP_IS_MAXIMAL
            | EP_IS_STRICTLY_MAXIMAL
            | EP_HAS_EQUIV
            | EP_IS_DOMINATED;

        assert!(props.is_positive());
        assert!(!props.is_negative());
        assert!(props.is_equ_literal());
        assert!(props.is_maximal());
        assert!(props.is_strictly_maximal());
        assert!(props.has_equiv());
        assert!(props.is_dominated());
        assert!(props.dominates());
    }

    #[test]
    fn property_equivalence_checks_selected_mask_only() {
        let left = EP_IS_POSITIVE | EP_IS_ORIENTED;
        let right = EP_IS_POSITIVE | EP_IS_MAXIMAL;

        assert!(EqnProperties::are_equiv(left, right, EP_IS_POSITIVE));
        assert!(!EqnProperties::are_equiv(
            left,
            right,
            EP_IS_ORIENTED | EP_IS_MAXIMAL
        ));
    }

    #[test]
    fn side_and_direction_discriminants_match_c_enums() {
        assert_eq!(EqnSide::NoSide as i32, 0);
        assert_eq!(EqnSide::LeftSide as i32, 1);
        assert_eq!(MAX_SIDE, EqnSide::LeftSide);
        assert_eq!(EqnSide::RightSide as i32, 2);
        assert_eq!(MIN_SIDE, EqnSide::RightSide);
        assert_eq!(EqnSide::BothSides as i32, 3);
        assert_eq!(PatEqnDirection::Normal as i32, 0);
        assert_eq!(PatEqnDirection::Reverse as i32, 1);
        assert_eq!(EQUAL_PREDICATE, "equal");
    }
}
