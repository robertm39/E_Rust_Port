//! Port of `PROPOSITIONAL/cpr_propclauses`.

use crate::basics::error::{Diagnostic, ErrorCode};
use crate::clauses::clause::Clause;
use crate::propositional::propsig::PropSig;
use crate::propositional::PLiteralCode;
use crate::terms::termbanks::TermBank;
use std::cmp::Ordering;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i32)]
pub enum DpllOutputFormat {
    NoFormat = 0,
    Lop = 1,
    Dimacs = 2,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DpllClause {
    lit_no: usize,
    active_no: usize,
    literals: Vec<PLiteralCode>,
}

impl DpllClause {
    #[must_use]
    pub fn from_literals(literals: Vec<PLiteralCode>) -> Self {
        let lit_no = literals.len();
        Self {
            lit_no,
            active_no: lit_no,
            literals,
        }
    }

    /// C `DPLLClauseFromClause`.
    ///
    /// # Errors
    ///
    /// Returns a syntax diagnostic if a literal is not a real propositional
    /// predicate literal, or an internal diagnostic if the predicate symbol has
    /// no signature name.
    pub fn from_clause(
        psig: &mut PropSig,
        bank: &TermBank,
        clause: &Clause,
    ) -> Result<Self, Diagnostic> {
        let mut literals = Vec::with_capacity(clause.literal_number());
        for literal in clause.literals().as_slice() {
            if literal.is_equ_lit(bank) || !literal.left().is_const() {
                return Err(Diagnostic::new(
                    ErrorCode::SYNTAX_ERROR,
                    "Only real propositional clauses can be converted by DPLLClauseFromClause()!",
                ));
            }
            let name = bank
                .signature()
                .find_name(literal.left().f_code())
                .ok_or_else(|| {
                    Diagnostic::new(
                        ErrorCode::OTHER_ERROR,
                        "propositional literal has no signature name",
                    )
                })?;
            let atom = psig.insert_atom(name);
            literals.push(if literal.is_negative() { -atom } else { atom });
        }
        Ok(Self::from_literals(literals))
    }

    #[must_use]
    pub const fn lit_no(&self) -> usize {
        self.lit_no
    }

    #[must_use]
    pub const fn active_no(&self) -> usize {
        self.active_no
    }

    #[must_use]
    pub const fn is_unit(&self) -> bool {
        self.active_no == 1
    }

    #[must_use]
    pub fn literals(&self) -> &[PLiteralCode] {
        &self.literals[..self.lit_no]
    }

    #[must_use]
    pub fn storage_len(&self) -> usize {
        self.literals.len()
    }

    /// C `DPLLClauseNormalize`.
    ///
    /// Sorts the active literals by atom code with positive literals before
    /// negative literals for the same atom, removes duplicate literals, keeps
    /// complementary pairs, and reports whether any complementary pair was
    /// found.
    ///
    /// # Panics
    ///
    /// Panics if `lit_no != active_no`, matching the C assertion.
    pub fn normalize(&mut self) -> bool {
        if self.lit_no <= 1 {
            return false;
        }
        assert_eq!(
            self.lit_no, self.active_no,
            "DPLLClauseNormalize requires all literals to be active"
        );

        self.literals[..self.lit_no].sort_by(|left, right| compare_literals(*left, *right));
        let mut tautology = false;
        let mut to = 0_usize;
        let mut from = 1_usize;
        while from < self.lit_no {
            if self.literals[from] != self.literals[to] {
                if self.literals[from] == -self.literals[to] {
                    tautology = true;
                }
                to += 1;
                self.literals[to] = self.literals[from];
            }
            from += 1;
        }
        self.lit_no = to + 1;
        self.active_no = self.lit_no;
        tautology
    }

    /// C `DPLLClausePrintLOP`.
    ///
    /// # Panics
    ///
    /// Panics if the clause contains an atom code unknown to `psig`, matching
    /// `PropSigGetAtomName` assertions.
    #[must_use]
    pub fn print_lop_string(&self, psig: &PropSig) -> String {
        let mut output = String::new();
        let mut sep = "";
        for literal in self.literals() {
            if *literal > 0 {
                output.push_str(sep);
                output.push_str(psig.atom_name(*literal));
                sep = ";";
            }
        }
        output.push_str("<-");
        sep = "";
        for literal in self.literals() {
            if *literal < 0 {
                output.push_str(sep);
                output.push_str(psig.atom_name(-*literal));
                sep = ",";
            }
        }
        output.push('.');
        output
    }

    /// C `DPLLClausePrintDimacs`.
    #[must_use]
    pub fn print_dimacs_string(&self) -> String {
        let mut output = String::new();
        for literal in self.literals() {
            output.push_str(&literal.to_string());
            output.push(' ');
        }
        output.push_str("0\n");
        output
    }
}

fn compare_literals(left: PLiteralCode, right: PLiteralCode) -> Ordering {
    match left.abs().cmp(&right.abs()) {
        Ordering::Equal => match (left > 0, right > 0) {
            (true, false) => Ordering::Less,
            (false, true) => Ordering::Greater,
            _ => Ordering::Equal,
        },
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::{DpllClause, DpllOutputFormat};
    use crate::clauses::clause::Clause;
    use crate::clauses::eqn::Eqn;
    use crate::clauses::eqnlist::EqnList;
    use crate::propositional::propsig::PropSig;
    use crate::terms::signature::Signature;
    use crate::terms::termbanks::TermBank;
    use crate::terms::termtypes::{DerefType, Term};
    use crate::terms::typebanks::TypeBank;

    fn test_bank() -> TermBank {
        let mut signature = Signature::new(TypeBank::new());
        signature.insert_internal_codes().unwrap();
        TermBank::new(signature).unwrap()
    }

    fn predicate_atom(bank: &mut TermBank, name: &str) -> Term {
        let bool_type = bank.signature().type_bank().bool_type();
        let f_code = bank.signature_mut().insert_id(name, 0, false);
        bank.signature_mut()
            .declare_final_type(f_code, bool_type)
            .unwrap();
        bank.create_const_term(f_code).unwrap()
    }

    fn predicate_literal(bank: &mut TermBank, atom: &Term, positive: bool) -> Eqn {
        Eqn::alloc(atom.clone(), bank.true_term().clone(), bank, positive).unwrap()
    }

    fn clause_from(literals: Vec<Eqn>) -> Clause {
        Clause::alloc(EqnList::from_vec(literals))
    }

    #[test]
    fn output_format_discriminants_match_c_enum() {
        assert_eq!(DpllOutputFormat::NoFormat as i32, 0);
        assert_eq!(DpllOutputFormat::Lop as i32, 1);
        assert_eq!(DpllOutputFormat::Dimacs as i32, 2);
    }

    #[test]
    fn from_clause_encodes_predicate_literals_and_updates_signature() {
        let mut bank = test_bank();
        let p = predicate_atom(&mut bank, "dpll_p");
        let q = predicate_atom(&mut bank, "dpll_q");
        let clause = clause_from(vec![
            predicate_literal(&mut bank, &p, true),
            predicate_literal(&mut bank, &q, false),
        ]);
        let mut psig = PropSig::new();

        let dpll = DpllClause::from_clause(&mut psig, &bank, &clause).unwrap();

        assert_eq!(dpll.lit_no(), 2);
        assert_eq!(dpll.active_no(), 2);
        assert_eq!(dpll.literals(), &[1, -2]);
        assert_eq!(psig.atom_name(1), "dpll_p");
        assert_eq!(psig.atom_name(2), "dpll_q");
    }

    #[test]
    fn from_clause_reuses_existing_atom_codes() {
        let mut bank = test_bank();
        let p = predicate_atom(&mut bank, "dpll_reuse_p");
        let q = predicate_atom(&mut bank, "dpll_reuse_q");
        let clause = clause_from(vec![
            predicate_literal(&mut bank, &q, true),
            predicate_literal(&mut bank, &p, false),
        ]);
        let mut psig = PropSig::new();
        assert_eq!(psig.insert_atom("dpll_reuse_p"), 1);

        let dpll = DpllClause::from_clause(&mut psig, &bank, &clause).unwrap();

        assert_eq!(dpll.literals(), &[2, -1]);
        assert_eq!(psig.atom_name(1), "dpll_reuse_p");
        assert_eq!(psig.atom_name(2), "dpll_reuse_q");
    }

    #[test]
    fn from_clause_rejects_non_propositional_literals() {
        let mut bank = test_bank();
        let type_ = bank.signature().type_bank().default_type();
        let a_code = bank.signature_mut().insert_id("dpll_a", 0, false);
        let b_code = bank.signature_mut().insert_id("dpll_b", 0, false);
        bank.signature_mut()
            .declare_final_type(a_code, type_.clone())
            .unwrap();
        bank.signature_mut()
            .declare_final_type(b_code, type_)
            .unwrap();
        let left = bank
            .insert(&Term::const_cell_alloc(a_code), DerefType::Never)
            .unwrap();
        let right = bank
            .insert(&Term::const_cell_alloc(b_code), DerefType::Never)
            .unwrap();
        let equation = Eqn::alloc(left, right, &mut bank, true).unwrap();
        let clause = clause_from(vec![equation]);
        let mut psig = PropSig::new();

        let error = DpllClause::from_clause(&mut psig, &bank, &clause).unwrap_err();

        assert_eq!(error.code(), crate::basics::error::ErrorCode::SYNTAX_ERROR);
        assert_eq!(
            error.message(),
            "Only real propositional clauses can be converted by DPLLClauseFromClause()!"
        );
    }

    #[test]
    fn normalize_sorts_deduplicates_and_keeps_storage() {
        let mut clause = DpllClause::from_literals(vec![-3, 2, -1, 2, 1, -3, 3]);

        let tautology = clause.normalize();

        assert!(tautology);
        assert_eq!(clause.literals(), &[1, -1, 2, 3, -3]);
        assert_eq!(clause.lit_no(), 5);
        assert_eq!(clause.active_no(), 5);
        assert_eq!(clause.storage_len(), 7);
    }

    #[test]
    fn normalize_leaves_unit_and_empty_clauses_unchanged() {
        let mut unit = DpllClause::from_literals(vec![-7]);
        let mut empty = DpllClause::from_literals(Vec::new());

        assert!(!unit.normalize());
        assert!(!empty.normalize());
        assert_eq!(unit.literals(), &[-7]);
        assert!(empty.literals().is_empty());
        assert!(unit.is_unit());
        assert!(!empty.is_unit());
    }

    #[test]
    fn printers_match_lop_and_dimacs_shapes() {
        let mut psig = PropSig::new();
        psig.insert_atom("p");
        psig.insert_atom("q");
        psig.insert_atom("r");
        psig.insert_atom("s");
        let clause = DpllClause::from_literals(vec![1, 2, -3, -4]);

        assert_eq!(clause.print_lop_string(&psig), "p;q<-r,s.");
        assert_eq!(clause.print_dimacs_string(), "1 2 -3 -4 0\n");
    }

    #[test]
    fn printers_handle_empty_and_one_sided_clauses() {
        let mut psig = PropSig::new();
        psig.insert_atom("p");
        let empty = DpllClause::from_literals(Vec::new());
        let positive = DpllClause::from_literals(vec![1]);
        let negative = DpllClause::from_literals(vec![-1]);

        assert_eq!(empty.print_lop_string(&psig), "<-.");
        assert_eq!(empty.print_dimacs_string(), "0\n");
        assert_eq!(positive.print_lop_string(&psig), "p<-.");
        assert_eq!(negative.print_lop_string(&psig), "<-p.");
    }
}
