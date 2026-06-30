//! Propositional reasoning support ported from E's `PROPOSITIONAL` units.

pub mod varset;

/// C `PLiteralCode`.
pub type PLiteralCode = i64;

/// C `PLiteralNoLit`.
pub const PLITERAL_NO_LIT: PLiteralCode = 0;

/// C `PAtomP(code)`.
#[must_use]
pub const fn p_atom_p(code: PLiteralCode) -> bool {
    code > 0
}
