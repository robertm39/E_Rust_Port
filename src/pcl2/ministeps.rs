//! Port of `PCL2/pcl_ministeps`.

use std::fmt::Write as _;

use crate::basics::error::Diagnostic;
use crate::basics::simple_stuff::ProblemType;
use crate::clauses::clause::clause_pcl_parse;
use crate::clauses::clausefunc::{
    tformula_tptp_parse, tformula_tptp_string, TFormulaTptpPrintOptions,
};
use crate::clauses::inferencedoc::ProofDocOutputFormat;
use crate::inout::basicparser::parse_int;
use crate::inout::scanner::{Scanner, TokenType};
use crate::pcl2::current_error;
use crate::pcl2::expressions::{PclExpression, PclOpCode};
use crate::pcl2::miniclauses::MiniClause;
use crate::pcl2::steps::{
    external_type_string, parse_external_type, prop_to_tstp_type, PclStepProperties,
    PCL_IS_FOF_STEP, PCL_IS_INITIAL, PCL_IS_PROOF_STEP, PCL_IS_SHELL_STEP,
};
use crate::terms::termbanks::TermBank;
use crate::terms::termtypes::Term;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PclMiniStepParseOptions {
    pub problem_type: ProblemType,
    pub support_shell_pcl: bool,
}

impl Default for PclMiniStepParseOptions {
    fn default() -> Self {
        Self {
            problem_type: ProblemType::FirstOrder,
            support_shell_pcl: false,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PclMiniStepLogic {
    Shell,
    Clause(MiniClause),
    Formula(Term),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PclMiniStep {
    id: i64,
    logic: PclMiniStepLogic,
    properties: PclStepProperties,
    just: PclExpression,
    extra: Option<String>,
}

impl PclMiniStep {
    #[must_use]
    pub fn new(
        id: i64,
        logic: PclMiniStepLogic,
        properties: PclStepProperties,
        just: PclExpression,
        extra: Option<String>,
    ) -> Self {
        Self {
            id,
            logic,
            properties,
            just,
            extra,
        }
    }

    #[must_use]
    pub const fn id(&self) -> i64 {
        self.id
    }

    #[must_use]
    pub const fn logic(&self) -> &PclMiniStepLogic {
        &self.logic
    }

    #[must_use]
    pub const fn properties(&self) -> PclStepProperties {
        self.properties
    }

    #[must_use]
    pub const fn just(&self) -> &PclExpression {
        &self.just
    }

    #[must_use]
    pub fn extra(&self) -> Option<&str> {
        self.extra.as_deref()
    }

    #[must_use]
    pub const fn is_fof(&self) -> bool {
        self.properties.is_fof()
    }

    #[must_use]
    pub const fn is_shell(&self) -> bool {
        self.properties.is_shell()
    }

    #[must_use]
    pub const fn is_clausal(&self) -> bool {
        self.properties.is_clausal()
    }

    pub fn set_property(&mut self, properties: PclStepProperties) {
        self.properties.set(properties);
    }

    pub fn delete_property(&mut self, properties: PclStepProperties) {
        self.properties.delete(properties);
    }

    #[must_use]
    pub fn is_empty_clause(&self) -> bool {
        matches!(&self.logic, PclMiniStepLogic::Clause(clause) if clause.is_empty())
    }

    /// C `PCLMiniStepParse`.
    ///
    /// # Errors
    ///
    /// Returns scanner diagnostics for invalid mini-step syntax or diagnostics
    /// from the underlying clause/formula/expression parsers.
    ///
    /// # Panics
    ///
    /// Panics if a clausal mini-step is parsed while the scanner is not in
    /// TPTP format, matching the currently ported `ClausePCLParse` assertion.
    pub fn parse(
        scanner: &mut Scanner,
        bank: &mut TermBank,
        options: PclMiniStepParseOptions,
    ) -> Result<Self, Diagnostic> {
        let id = parse_int(scanner)?;
        if scanner.test_tok(TokenType::FULLSTOP) {
            return Err(current_error(
                scanner,
                "No compound PCL identifiers allowed in this mode",
            ));
        }
        scanner.accept_tok(TokenType::COLON)?;
        let mut properties = parse_external_type(scanner)?;
        scanner.accept_tok(TokenType::COLON)?;

        let logic = if options.support_shell_pcl && scanner.test_tok(TokenType::COLON) {
            properties.set(PCL_IS_SHELL_STEP);
            PclMiniStepLogic::Shell
        } else if scanner.test_tok(TokenType::OPEN_SQUARE) {
            let clause = clause_pcl_parse(scanner, bank, options.problem_type)?;
            properties.delete(PCL_IS_FOF_STEP);
            PclMiniStepLogic::Clause(MiniClause::minify_clause(clause))
        } else {
            let formula = tformula_tptp_parse(scanner, bank)?;
            properties.set(PCL_IS_FOF_STEP);
            PclMiniStepLogic::Formula(formula)
        };

        scanner.accept_tok(TokenType::COLON)?;
        let just = PclExpression::parse(scanner, true)?;
        let extra = if scanner.test_tok(TokenType::COLON) {
            scanner.next_token()?;
            scanner.check_tok(TokenType::SQ_STRING)?;
            let value = scanner.current_token().literal();
            scanner.next_token()?;
            Some(value)
        } else {
            None
        };

        properties.delete(PCL_IS_PROOF_STEP);
        if just.op() == PclOpCode::Initial {
            properties.set(PCL_IS_INITIAL);
        }

        Ok(Self {
            id,
            logic,
            properties,
            just,
            extra,
        })
    }

    /// C `PCLMiniStepPrint`.
    ///
    /// # Errors
    ///
    /// Returns diagnostics from temporary clause/formula rendering.
    pub fn print_string(
        &self,
        bank: &mut TermBank,
        problem_type: ProblemType,
    ) -> Result<String, Diagnostic> {
        let mut output = format!("{:6} : ", self.id);
        output.push_str(&external_type_string(self.properties));
        output.push_str(" : ");
        if !self.is_shell() {
            match &self.logic {
                PclMiniStepLogic::Shell => {}
                PclMiniStepLogic::Clause(clause) => {
                    output.push_str(&clause.print_pcl_string(bank)?);
                }
                PclMiniStepLogic::Formula(formula) => {
                    output.push_str(&tformula_tptp_string(
                        bank,
                        formula,
                        true,
                        TFormulaTptpPrintOptions::tptp(problem_type),
                    )?);
                }
            }
        }
        output.push_str(" : ");
        output.push_str(&self.just.print_string(true));
        if let Some(extra) = &self.extra {
            output.push_str(" : ");
            output.push_str(extra);
        }
        Ok(output)
    }

    /// C `PCLMiniStepPrintTSTP`.
    ///
    /// # Errors
    ///
    /// Returns diagnostics from temporary clause/formula rendering.
    pub fn print_tstp_string(
        &self,
        bank: &mut TermBank,
        problem_type: ProblemType,
    ) -> Result<String, Diagnostic> {
        let mut output = String::new();
        if self.is_clausal() {
            let _ = write!(
                output,
                "cnf({},{},",
                self.id,
                prop_to_tstp_type(self.properties)
            );
            if !self.is_shell() {
                if let PclMiniStepLogic::Clause(clause) = &self.logic {
                    output.push_str(&clause.print_tstp_core_string(bank)?);
                }
            }
        } else {
            let _ = write!(
                output,
                "fof({}, {},",
                self.id,
                prop_to_tstp_type(self.properties)
            );
            if !self.is_shell() {
                if let PclMiniStepLogic::Formula(formula) = &self.logic {
                    output.push_str(&tformula_tptp_string(
                        bank,
                        formula,
                        true,
                        TFormulaTptpPrintOptions::tptp(problem_type),
                    )?);
                }
            }
        }
        output.push(',');
        output.push_str(&self.just.print_tstp_string(true));
        if let Some(extra) = &self.extra {
            output.push_str(",[");
            output.push_str(extra);
            output.push(']');
        }
        output.push_str(").");
        Ok(output)
    }

    /// C `PCLMiniStepPrintFormat`.
    ///
    /// # Errors
    ///
    /// Returns diagnostics from the selected printer.
    ///
    /// # Panics
    ///
    /// Panics for unsupported formats, matching the C assertion in the default
    /// switch branch.
    pub fn print_format_string(
        &self,
        bank: &mut TermBank,
        problem_type: ProblemType,
        format: ProofDocOutputFormat,
    ) -> Result<String, Diagnostic> {
        match format {
            ProofDocOutputFormat::Pcl => self.print_string(bank, problem_type),
            ProofDocOutputFormat::Tstp => self.print_tstp_string(bank, problem_type),
            _ => panic!("PCLMiniStepPrintFormat supports only PCL and TSTP"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{PclMiniStep, PclMiniStepLogic, PclMiniStepParseOptions};
    use crate::basics::simple_stuff::ProblemType;
    use crate::clauses::inferencedoc::ProofDocOutputFormat;
    use crate::inout::scanner::{IoFormat, Scanner, TokenType};
    use crate::pcl2::steps::{
        PCL_IS_FOF_STEP, PCL_IS_INITIAL, PCL_IS_LEMMA, PCL_IS_SHELL_STEP, PCL_TYPE_AXIOM,
        PCL_TYPE_CONJECTURE, PCL_TYPE_NEG_CONJECTURE,
    };
    use crate::terms::signature::Signature;
    use crate::terms::termbanks::TermBank;
    use crate::terms::typebanks::TypeBank;

    fn test_bank() -> TermBank {
        let mut signature = Signature::new(TypeBank::new());
        signature.insert_internal_codes().unwrap();
        TermBank::new(signature).unwrap()
    }

    fn parse(source: &str, support_shell_pcl: bool) -> (PclMiniStep, TermBank, Scanner) {
        let mut bank = test_bank();
        let mut scanner = Scanner::from_user_string(source, false).unwrap();
        scanner.set_format(IoFormat::Tptp);
        let step = PclMiniStep::parse(
            &mut scanner,
            &mut bank,
            PclMiniStepParseOptions {
                problem_type: ProblemType::FirstOrder,
                support_shell_pcl,
            },
        )
        .unwrap();
        (step, bank, scanner)
    }

    #[test]
    fn parses_clause_ministep_and_prints_pcl_and_tstp_shapes() {
        let (step, mut bank, scanner) = parse(
            "42 : lemma,conj : [++p(a),--q(a)] : initial : 'extra' tail",
            false,
        );

        assert_eq!(step.id(), 42);
        assert!(matches!(step.logic(), PclMiniStepLogic::Clause(_)));
        assert!(step.properties().query(PCL_IS_LEMMA | PCL_IS_INITIAL));
        assert_eq!(step.properties().query_type(), PCL_TYPE_CONJECTURE);
        assert!(!step.properties().is_fof());
        assert_eq!(step.extra(), Some("'extra'"));
        assert_eq!(scanner.current_token().literal(), "tail");

        assert_eq!(
            step.print_string(&mut bank, ProblemType::FirstOrder)
                .unwrap(),
            "    42 : lemma,conj : [++p(a),--q(a)] : initial : 'extra'"
        );
        assert_eq!(
            step.print_tstp_string(&mut bank, ProblemType::FirstOrder)
                .unwrap(),
            "cnf(42,conjecture,(p(a)|~q(a)),unknown(),['extra'])."
        );
    }

    #[test]
    fn parses_formula_ministep_and_sets_fof_property() {
        let (step, mut bank, scanner) = parse("7 : neg : p(a)|q(a) : 3 tail", false);

        assert_eq!(step.id(), 7);
        assert!(matches!(step.logic(), PclMiniStepLogic::Formula(_)));
        assert!(step.properties().query(PCL_IS_FOF_STEP));
        assert_eq!(step.properties().query_type(), PCL_TYPE_NEG_CONJECTURE);
        assert_eq!(scanner.current_token().literal(), "tail");

        assert_eq!(
            step.print_string(&mut bank, ProblemType::FirstOrder)
                .unwrap(),
            "     7 : neg : (p(a)|q(a)) : 3"
        );
        assert_eq!(
            step.print_tstp_string(&mut bank, ProblemType::FirstOrder)
                .unwrap(),
            "fof(7, negated_conjecture,(p(a)|q(a)),3)."
        );
    }

    #[test]
    fn parses_shell_ministep_only_when_option_is_enabled() {
        let (step, mut bank, _) = parse("3 : : : 2", true);

        assert!(matches!(step.logic(), PclMiniStepLogic::Shell));
        assert!(step.properties().query(PCL_IS_SHELL_STEP));
        assert_eq!(step.properties().query_type(), PCL_TYPE_AXIOM);
        assert!(step.is_clausal());

        assert_eq!(
            step.print_string(&mut bank, ProblemType::FirstOrder)
                .unwrap(),
            "     3 :  :  : 2"
        );
        assert_eq!(
            step.print_tstp_string(&mut bank, ProblemType::FirstOrder)
                .unwrap(),
            "cnf(3,plain,,2)."
        );
    }

    #[test]
    fn shell_shape_without_option_falls_through_to_formula_parser() {
        let mut bank = test_bank();
        let mut scanner = Scanner::from_user_string("3 : : : 2", false).unwrap();
        scanner.set_format(IoFormat::Tptp);

        let error = PclMiniStep::parse(&mut scanner, &mut bank, PclMiniStepParseOptions::default())
            .unwrap_err();

        assert!(error.message().contains("expected"));
    }

    #[test]
    fn rejects_compound_identifiers_in_mini_mode() {
        let mut bank = test_bank();
        let mut scanner = Scanner::from_user_string("1.2 : : [++p] : 3", false).unwrap();
        scanner.set_format(IoFormat::Tptp);

        let error = PclMiniStep::parse(&mut scanner, &mut bank, PclMiniStepParseOptions::default())
            .unwrap_err();

        assert!(error
            .message()
            .contains("No compound PCL identifiers allowed in this mode"));
        assert!(scanner.test_tok(TokenType::FULLSTOP));
    }

    #[test]
    fn mini_extra_field_accepts_only_single_quoted_strings() {
        let mut bank = test_bank();
        let mut scanner = Scanner::from_user_string("1 : : [++p] : 3 : name", false).unwrap();
        scanner.set_format(IoFormat::Tptp);

        let error = PclMiniStep::parse(&mut scanner, &mut bank, PclMiniStepParseOptions::default())
            .unwrap_err();

        assert!(error.message().contains("String enclosed in single quote"));
    }

    #[test]
    fn print_format_dispatches_only_pcl_and_tstp() {
        let (step, mut bank, _) = parse("4 : : [++p] : 3", false);

        assert_eq!(
            step.print_format_string(
                &mut bank,
                ProblemType::FirstOrder,
                ProofDocOutputFormat::Pcl
            )
            .unwrap(),
            "     4 :  : [++p] : 3"
        );
        assert_eq!(
            step.print_format_string(
                &mut bank,
                ProblemType::FirstOrder,
                ProofDocOutputFormat::Tstp
            )
            .unwrap(),
            "cnf(4,plain,(p),3)."
        );
    }
}
