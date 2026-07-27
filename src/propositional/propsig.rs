//! Port of `PROPOSITIONAL/cpr_propsig`.

use crate::basics::defines::DEFAULT_COMCHAR_RAW;
use crate::propositional::{p_atom_p, PLiteralCode, PLITERAL_NO_LIT};
use std::collections::BTreeMap;
use std::fmt::Write as _;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PropSig {
    enc_to_name: Vec<Option<String>>,
    name_to_enc: BTreeMap<String, PLiteralCode>,
}

impl Default for PropSig {
    fn default() -> Self {
        Self::new()
    }
}

impl PropSig {
    /// C `PropSigAlloc`.
    #[must_use]
    pub fn new() -> Self {
        Self {
            enc_to_name: vec![None],
            name_to_enc: BTreeMap::new(),
        }
    }

    /// C `PropSigAtomNumber(psig)`.
    ///
    /// This includes the reserved `PLiteralNoLit` slot, so a fresh signature
    /// has atom number `1` and the first inserted atom receives encoding `1`.
    #[must_use]
    pub fn atom_number(&self) -> PLiteralCode {
        match PLiteralCode::try_from(self.enc_to_name.len()) {
            Ok(value) => value,
            Err(_overflow) => PLiteralCode::MAX,
        }
    }

    #[must_use]
    pub fn atom_count(&self) -> usize {
        self.enc_to_name.len().saturating_sub(1)
    }

    /// C `PropSigGetAtomEnc`.
    #[must_use]
    pub fn atom_encoding(&self, name: &str) -> PLiteralCode {
        self.name_to_enc
            .get(name)
            .copied()
            .unwrap_or(PLITERAL_NO_LIT)
    }

    /// C `PropSigInsertAtom`.
    pub fn insert_atom(&mut self, name: &str) -> PLiteralCode {
        let existing = self.atom_encoding(name);
        if existing != PLITERAL_NO_LIT {
            return existing;
        }

        let enc = self.atom_number();
        let owned_name = name.to_owned();
        self.enc_to_name.push(Some(owned_name.clone()));
        let replaced = self.name_to_enc.insert(owned_name, enc);
        debug_assert!(replaced.is_none());
        enc
    }

    /// C `PropSigGetAtomName`.
    ///
    /// # Panics
    ///
    /// Panics if `atom` is not a positive known atom code, matching the C
    /// assertions in `PropSigGetAtomName`.
    #[must_use]
    pub fn atom_name(&self, atom: PLiteralCode) -> &str {
        assert!(
            p_atom_p(atom),
            "PropSigGetAtomName requires a positive atom code"
        );
        assert!(
            atom < self.atom_number(),
            "PropSigGetAtomName requires a known atom code"
        );
        let index = match usize::try_from(atom) {
            Ok(index) => index,
            Err(error) => panic!("PropSigGetAtomName atom code does not fit usize: {error}"),
        };
        match self.enc_to_name.get(index).and_then(Option::as_deref) {
            Some(name) => name,
            None => panic!("PropSigGetAtomName found no name for atom code"),
        }
    }

    /// C `PropSigPrint`.
    ///
    /// # Panics
    ///
    /// Panics only if writing to an owned `String` fails.
    #[must_use]
    pub fn print_string(&self) -> String {
        let mut output = format!(
            "{DEFAULT_COMCHAR_RAW} Propositional signature:\n\
             {DEFAULT_COMCHAR_RAW} ------------------------\n"
        );
        for atom in 1..self.atom_number() {
            let name = self.atom_name(atom);
            match writeln!(output, "{DEFAULT_COMCHAR_RAW} {atom:6} : {name}") {
                Ok(()) => {}
                Err(error) => panic!("writing to a String failed: {error}"),
            }
        }
        output.push('\n');
        output
    }
}

#[cfg(test)]
mod tests {
    use super::PropSig;
    use crate::propositional::PLITERAL_NO_LIT;

    #[test]
    fn new_signature_reserves_literal_zero() {
        let sig = PropSig::new();

        assert_eq!(sig.atom_number(), 1);
        assert_eq!(sig.atom_count(), 0);
        assert_eq!(sig.atom_encoding("missing"), PLITERAL_NO_LIT);
        assert_eq!(
            sig.print_string(),
            "% Propositional signature:\n% ------------------------\n\n"
        );
    }

    #[test]
    fn insertion_assigns_codes_from_reserved_stack_top() {
        let mut sig = PropSig::new();

        assert_eq!(sig.insert_atom("p"), 1);
        assert_eq!(sig.insert_atom("q"), 2);
        assert_eq!(sig.insert_atom("p"), 1);

        assert_eq!(sig.atom_number(), 3);
        assert_eq!(sig.atom_count(), 2);
        assert_eq!(sig.atom_encoding("p"), 1);
        assert_eq!(sig.atom_encoding("q"), 2);
        assert_eq!(sig.atom_encoding("r"), PLITERAL_NO_LIT);
        assert_eq!(sig.atom_name(1), "p");
        assert_eq!(sig.atom_name(2), "q");
    }

    #[test]
    fn print_string_uses_encoding_order_not_name_order() {
        let mut sig = PropSig::new();
        sig.insert_atom("zeta");
        sig.insert_atom("alpha");
        sig.insert_atom("middle");

        assert_eq!(
            sig.print_string(),
            "% Propositional signature:\n\
             % ------------------------\n\
             %      1 : zeta\n\
             %      2 : alpha\n\
             %      3 : middle\n\n"
        );
    }

    #[test]
    #[should_panic(expected = "PropSigGetAtomName requires a positive atom code")]
    fn atom_name_rejects_zero_literal_code() {
        let sig = PropSig::new();

        let _ = sig.atom_name(PLITERAL_NO_LIT);
    }

    #[test]
    #[should_panic(expected = "PropSigGetAtomName requires a positive atom code")]
    fn atom_name_rejects_negative_literal_code() {
        let sig = PropSig::new();

        let _ = sig.atom_name(-1);
    }

    #[test]
    #[should_panic(expected = "PropSigGetAtomName requires a known atom code")]
    fn atom_name_rejects_unallocated_positive_code() {
        let mut sig = PropSig::new();
        sig.insert_atom("p");

        let _ = sig.atom_name(2);
    }
}
