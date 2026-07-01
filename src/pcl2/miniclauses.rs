//! Port of `PCL2/pcl_miniclauses`.

use crate::basics::error::Diagnostic;
use crate::clauses::clause::{
    clause_pcl_string, clause_print_lop_format_string, clause_print_tstp_core_string, Clause,
};
use crate::clauses::eqn::Eqn;
use crate::clauses::eqnlist::EqnList;
use crate::terms::termbanks::TermBank;
use crate::terms::termtypes::Term;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MiniLiteral {
    positive: bool,
    left: Term,
    right: Term,
}

impl MiniLiteral {
    #[must_use]
    pub const fn new(positive: bool, left: Term, right: Term) -> Self {
        Self {
            positive,
            left,
            right,
        }
    }

    #[must_use]
    pub const fn is_positive(&self) -> bool {
        self.positive
    }

    #[must_use]
    pub const fn is_negative(&self) -> bool {
        !self.positive
    }

    #[must_use]
    pub const fn left(&self) -> &Term {
        &self.left
    }

    #[must_use]
    pub const fn right(&self) -> &Term {
        &self.right
    }

    fn from_eqn(literal: &Eqn) -> Self {
        Self {
            positive: literal.is_positive(),
            left: literal.left().clone(),
            right: literal.right().clone(),
        }
    }

    fn to_eqn(&self, bank: &mut TermBank) -> Result<Eqn, Diagnostic> {
        Eqn::alloc(self.left.clone(), self.right.clone(), bank, self.positive)
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct MiniClause {
    literals: Vec<MiniLiteral>,
}

impl MiniClause {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            literals: Vec::new(),
        }
    }

    /// C `ClauseToMiniClause`.
    #[must_use]
    pub fn from_clause(clause: &Clause) -> Self {
        Self::from_eqns(clause.literals().as_slice())
    }

    /// C `MinifyClause`.
    #[must_use]
    pub fn minify_clause(clause: Clause) -> Self {
        let literals = clause.into_literals().into_vec();
        Self {
            literals: literals.iter().map(MiniLiteral::from_eqn).collect(),
        }
    }

    fn from_eqns(literals: &[Eqn]) -> Self {
        Self {
            literals: literals.iter().map(MiniLiteral::from_eqn).collect(),
        }
    }

    #[must_use]
    pub fn literals(&self) -> &[MiniLiteral] {
        &self.literals
    }

    #[must_use]
    pub const fn literal_number(&self) -> usize {
        self.literals.len()
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.literals.is_empty()
    }

    /// C `MiniClauseToClause`.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if allocating any rebuilt literal fails.
    pub fn to_clause(&self, bank: &mut TermBank) -> Result<Clause, Diagnostic> {
        let mut literals = Vec::with_capacity(self.literals.len());
        for literal in &self.literals {
            literals.push(literal.to_eqn(bank)?);
        }
        Ok(Clause::alloc(EqnList::from_vec(literals)))
    }

    /// C `UnMinifyClause`.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if allocating any rebuilt literal fails.
    pub fn unminify_clause(self, bank: &mut TermBank) -> Result<Clause, Diagnostic> {
        self.to_clause(bank)
    }

    /// Renders through a temporary ordinary clause, matching C
    /// `MiniClausePrint`. Rust currently exposes the LOP branch explicitly
    /// because the process-global output-format dispatcher is not part of this
    /// PCL2 slice.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if allocating any rebuilt literal fails.
    pub fn print_lop_string(
        &self,
        bank: &mut TermBank,
        full_terms: bool,
    ) -> Result<String, Diagnostic> {
        let clause = self.to_clause(bank)?;
        Ok(clause_print_lop_format_string(bank, &clause, full_terms))
    }

    /// C `MiniClausePCLPrint`.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if allocating any rebuilt literal fails.
    pub fn print_pcl_string(&self, bank: &mut TermBank) -> Result<String, Diagnostic> {
        let clause = self.to_clause(bank)?;
        Ok(clause_pcl_string(bank, &clause, true))
    }

    /// C `MiniClauseTSTPCorePrint`.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if allocating any rebuilt literal fails.
    pub fn print_tstp_core_string(&self, bank: &mut TermBank) -> Result<String, Diagnostic> {
        let clause = self.to_clause(bank)?;
        Ok(clause_print_tstp_core_string(bank, &clause, true, false))
    }
}

#[cfg(test)]
mod tests {
    use super::{MiniClause, MiniLiteral};
    use crate::clauses::clause::{
        clause_pcl_string, clause_print_lop_format_string, clause_print_tstp_core_string, Clause,
    };
    use crate::clauses::clause_props::CP_TYPE_NEG_CONJECTURE;
    use crate::clauses::eqn::Eqn;
    use crate::clauses::eqnlist::EqnList;
    use crate::terms::signature::Signature;
    use crate::terms::termbanks::TermBank;
    use crate::terms::termtypes::{DerefType, Term};
    use crate::terms::typebanks::TypeBank;

    fn test_bank() -> TermBank {
        let mut signature = Signature::new(TypeBank::new());
        signature.insert_internal_codes().unwrap();
        TermBank::new(signature).unwrap()
    }

    fn typed_const(bank: &mut TermBank, name: &str) -> Term {
        let type_ = bank.signature().type_bank().default_type();
        let f_code = bank.signature_mut().insert_id(name, 0, false);
        bank.signature_mut()
            .declare_final_type(f_code, type_.clone())
            .unwrap();
        let term = Term::const_cell_alloc(f_code);
        term.set_type(Some(type_));
        bank.insert(&term, DerefType::Never).unwrap()
    }

    fn typed_pred_const(bank: &mut TermBank, name: &str) -> Term {
        let bool_type = bank.signature().type_bank().bool_type();
        let f_code = bank.signature_mut().insert_id(name, 0, false);
        bank.signature_mut()
            .declare_final_type(f_code, bool_type.clone())
            .unwrap();
        let term = Term::const_cell_alloc(f_code);
        term.set_type(Some(bool_type));
        bank.insert(&term, DerefType::Never).unwrap()
    }

    fn eqn(bank: &mut TermBank, left: &Term, right: &Term, positive: bool) -> Eqn {
        Eqn::alloc(left.clone(), right.clone(), bank, positive).unwrap()
    }

    fn sample_clause(bank: &mut TermBank) -> Clause {
        let a = typed_const(bank, "mini_a");
        let b = typed_const(bank, "mini_b");
        let p = typed_pred_const(bank, "mini_p");
        let true_term = bank.true_term().clone();
        Clause::alloc(EqnList::from_vec(vec![
            eqn(bank, &b, &a, false),
            eqn(bank, &a, &b, true),
            eqn(bank, &p, &true_term, true),
        ]))
    }

    #[test]
    fn from_clause_keeps_literal_signs_and_shared_term_handles() {
        let mut bank = test_bank();
        let clause = sample_clause(&mut bank);
        let mini = MiniClause::from_clause(&clause);

        assert_eq!(mini.literal_number(), 3);
        assert!(!mini.is_empty());
        assert!(mini.literals()[0].is_positive());
        assert!(mini.literals()[1].is_positive());
        assert!(mini.literals()[2].is_negative());
        assert_eq!(
            mini.literals()[0].left(),
            clause.literals().as_slice()[0].left()
        );
        assert_eq!(
            mini.literals()[0].right(),
            clause.literals().as_slice()[0].right()
        );
        assert_eq!(
            mini.literals()[2].left(),
            clause.literals().as_slice()[2].left()
        );
        assert_eq!(
            mini.literals()[2].right(),
            clause.literals().as_slice()[2].right()
        );
    }

    #[test]
    fn to_clause_round_trips_through_clause_allocation_shape() {
        let mut bank = test_bank();
        let mut clause = sample_clause(&mut bank);
        clause.set_tptp_type(CP_TYPE_NEG_CONJECTURE);
        let mini = MiniClause::from_clause(&clause);

        let rebuilt = mini.to_clause(&mut bank).unwrap();

        assert_eq!(rebuilt.literal_number(), clause.literal_number());
        assert_eq!(
            rebuilt.positive_literal_count(),
            clause.positive_literal_count()
        );
        assert_eq!(
            rebuilt.negative_literal_count(),
            clause.negative_literal_count()
        );
        assert_eq!(
            clause_pcl_string(&bank, &rebuilt, true),
            clause_pcl_string(&bank, &clause, true)
        );
        assert_eq!(
            clause_print_tstp_core_string(&bank, &rebuilt, true, false),
            clause_print_tstp_core_string(&bank, &clause, true, false)
        );
    }

    #[test]
    fn minify_and_unminify_consume_owned_shapes() {
        let mut bank = test_bank();
        let clause = sample_clause(&mut bank);
        let expected = clause_pcl_string(&bank, &clause, true);

        let mini = MiniClause::minify_clause(clause);
        let rebuilt = mini.unminify_clause(&mut bank).unwrap();

        assert_eq!(clause_pcl_string(&bank, &rebuilt, true), expected);
    }

    #[test]
    fn print_helpers_match_temporary_clause_rendering() {
        let mut bank = test_bank();
        let mut clause = sample_clause(&mut bank);
        clause.set_tptp_type(CP_TYPE_NEG_CONJECTURE);
        let mini = MiniClause::from_clause(&clause);
        let rebuilt_for_lop = mini.to_clause(&mut bank).unwrap();

        assert_eq!(
            mini.print_lop_string(&mut bank, true).unwrap(),
            clause_print_lop_format_string(&bank, &rebuilt_for_lop, true)
        );
        assert_eq!(
            mini.print_pcl_string(&mut bank).unwrap(),
            clause_pcl_string(&bank, &clause, true)
        );
        assert_eq!(
            mini.print_tstp_core_string(&mut bank).unwrap(),
            clause_print_tstp_core_string(&bank, &clause, true, false)
        );
    }

    #[test]
    fn empty_clause_rebuilds_and_prints_as_empty_core() {
        let mut bank = test_bank();
        let mini = MiniClause::new();

        let rebuilt = mini.to_clause(&mut bank).unwrap();

        assert_eq!(mini.literal_number(), 0);
        assert!(mini.is_empty());
        assert!(rebuilt.is_empty());
        assert_eq!(mini.print_pcl_string(&mut bank).unwrap(), "[]");
        assert_eq!(mini.print_tstp_core_string(&mut bank).unwrap(), "($false)");
    }

    #[test]
    fn mini_literal_constructor_exposes_terms_and_sign() {
        let mut bank = test_bank();
        let left = typed_const(&mut bank, "literal_left");
        let right = typed_const(&mut bank, "literal_right");
        let literal = MiniLiteral::new(false, left.clone(), right.clone());

        assert!(literal.is_negative());
        assert!(!literal.is_positive());
        assert_eq!(literal.left(), &left);
        assert_eq!(literal.right(), &right);
    }
}
