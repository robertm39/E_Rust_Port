//! Port of `PCL2/pcl_miniclauses`.

use crate::basics::error::Diagnostic;
use crate::basics::simple_stuff::ProblemType;
use crate::clauses::clause::{
    clause_pcl_string, clause_print_lop_format_string, clause_print_lop_format_string_with_options,
    clause_print_tptp_format_string_with_options, clause_print_tstp_core_string,
    clause_write_tstp_with_type_suffixes, Clause,
};
use crate::clauses::eqn::{Eqn, EqnPrintOptions};
use crate::clauses::eqnlist::EqnList;
use crate::inout::scanner::IoFormat;
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

/// Compact owned snapshot of a clause's literal signs and shared term handles.
///
/// Clause-level metadata is intentionally absent, matching C's commented-out
/// properties field and fresh `ClauseAlloc` reconstruction.
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

    /// Renders through a temporary ordinary clause, matching the default LOP
    /// branch of C `MiniClausePrint`.
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

    /// C `MiniClausePrint` with explicit `ClausePrint` dispatch.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if allocating any rebuilt literal fails, or if TSTP
    /// rendering rejects the rebuilt clause.
    pub fn print_format_string(
        &self,
        bank: &mut TermBank,
        full_terms: bool,
        output_format: IoFormat,
        problem_type: ProblemType,
    ) -> Result<String, Diagnostic> {
        let options = match output_format {
            IoFormat::Tptp => EqnPrintOptions::tptp(),
            IoFormat::Lop | IoFormat::Tstp | IoFormat::Auto => EqnPrintOptions::lop(),
        };
        self.print_format_string_with_options(
            bank,
            full_terms,
            output_format,
            problem_type,
            options,
        )
    }

    /// C `MiniClausePrint` with caller-provided equation options.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if allocating any rebuilt literal fails, or if TSTP
    /// rendering rejects the rebuilt clause.
    pub fn print_format_string_with_options(
        &self,
        bank: &mut TermBank,
        full_terms: bool,
        output_format: IoFormat,
        problem_type: ProblemType,
        options: EqnPrintOptions,
    ) -> Result<String, Diagnostic> {
        let clause = self.to_clause(bank)?;
        mini_clause_render_clause_string(
            bank,
            &clause,
            full_terms,
            output_format,
            problem_type,
            options,
        )
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

fn mini_clause_render_clause_string(
    bank: &TermBank,
    clause: &Clause,
    full_terms: bool,
    output_format: IoFormat,
    problem_type: ProblemType,
    options: EqnPrintOptions,
) -> Result<String, Diagnostic> {
    match output_format {
        IoFormat::Tptp => Ok(clause_print_tptp_format_string_with_options(
            bank, clause, options,
        )),
        IoFormat::Tstp => {
            let mut output = String::new();
            clause_write_tstp_with_type_suffixes(
                &mut output,
                bank,
                clause,
                full_terms,
                true,
                problem_type,
                options.print_types,
            )?;
            Ok(output)
        }
        IoFormat::Lop | IoFormat::Auto => Ok(clause_print_lop_format_string_with_options(
            bank, clause, full_terms, options,
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::{MiniClause, MiniLiteral};
    use crate::basics::simple_stuff::ProblemType;
    use crate::clauses::clause::{
        clause_pcl_string, clause_print_lop_format_string, clause_print_tstp_core_string, Clause,
    };
    use crate::clauses::clause_props::{CP_IGNORE_PROPS, CP_TYPE_NEG_CONJECTURE, CP_TYPE_UNKNOWN};
    use crate::clauses::eqn::{Eqn, EqnPrintOptions};
    use crate::clauses::eqnlist::EqnList;
    use crate::inout::scanner::IoFormat;
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

        assert_eq!(rebuilt.properties(), CP_IGNORE_PROPS);
        assert_eq!(rebuilt.query_tptp_type(), CP_TYPE_UNKNOWN);
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
    fn print_format_string_dispatches_rebuilt_clause_output() {
        let mut bank = test_bank();
        let clause = sample_clause(&mut bank);
        let mini = MiniClause::from_clause(&clause);

        let input_clause = mini
            .print_format_string(&mut bank, true, IoFormat::Tptp, ProblemType::FirstOrder)
            .unwrap_or_else(|err| panic!("{err}"));
        assert!(input_clause.starts_with("input_clause("));
        assert!(input_clause.contains("++equal(mini_a, mini_b)"));
        assert!(input_clause.contains("--equal(mini_b, mini_a)"));
        assert!(!input_clause.contains("<-"));

        let wrapped_clause = mini
            .print_format_string(&mut bank, true, IoFormat::Tstp, ProblemType::FirstOrder)
            .unwrap_or_else(|err| panic!("{err}"));
        assert!(wrapped_clause.starts_with("cnf(") || wrapped_clause.starts_with("tcf("));
        assert!(wrapped_clause.contains("mini_a"));
        assert!(!wrapped_clause.contains("<-"));

        assert_eq!(
            mini.print_format_string(&mut bank, true, IoFormat::Auto, ProblemType::FirstOrder)
                .unwrap_or_else(|err| panic!("{err}")),
            mini.print_lop_string(&mut bank, true)
                .unwrap_or_else(|err| panic!("{err}"))
        );
    }

    #[test]
    fn print_format_string_with_options_uses_caller_equation_options() {
        let mut bank = test_bank();
        let clause = sample_clause(&mut bank);
        let mini = MiniClause::from_clause(&clause);

        assert_eq!(
            mini.print_format_string_with_options(
                &mut bank,
                true,
                IoFormat::Lop,
                ProblemType::FirstOrder,
                EqnPrintOptions::lop()
            )
            .unwrap_or_else(|err| panic!("{err}")),
            mini.print_lop_string(&mut bank, true)
                .unwrap_or_else(|err| panic!("{err}"))
        );
    }

    #[test]
    fn output_format_is_local_to_each_print_call() {
        let mut bank = test_bank();
        let clause = sample_clause(&mut bank);
        let mini = MiniClause::from_clause(&clause);

        let lop_before = mini
            .print_format_string(&mut bank, true, IoFormat::Lop, ProblemType::FirstOrder)
            .unwrap();
        let tptp = mini
            .print_format_string(&mut bank, true, IoFormat::Tptp, ProblemType::FirstOrder)
            .unwrap();
        let lop_after = mini
            .print_format_string(&mut bank, true, IoFormat::Lop, ProblemType::FirstOrder)
            .unwrap();

        assert_eq!(lop_after, lop_before);
        assert!(tptp.starts_with("input_clause("));
        assert_ne!(tptp, lop_before);
    }

    #[test]
    fn literal_count_does_not_truncate_at_c_short_boundary() {
        let mut bank = test_bank();
        let left = typed_const(&mut bank, "wide_left");
        let right = typed_const(&mut bank, "wide_right");
        let literal = MiniLiteral::new(true, left, right);
        let expected = i16::MAX as usize + 1;
        let mini = MiniClause {
            literals: vec![literal; expected],
        };

        assert_eq!(mini.literal_number(), expected);
        assert!(!mini.is_empty());
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
