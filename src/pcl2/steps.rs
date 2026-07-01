//! Port of `PCL2/pcl_steps`.

use crate::basics::error::Diagnostic;
use crate::basics::simple_stuff::ProblemType;
use crate::clauses::clause::{
    clause_pcl_parse, clause_pcl_string, clause_print_lop_format_string,
    clause_print_tptp_format_string, clause_print_tstp_core_string, Clause,
};
use crate::clauses::clause_props::{
    FormulaProperties, CP_TYPE_1, CP_TYPE_2, CP_TYPE_3, CP_TYPE_AXIOM, CP_TYPE_CONJECTURE,
    CP_TYPE_MASK, CP_TYPE_NEG_CONJECTURE, CP_TYPE_QUESTION, CP_TYPE_UNKNOWN,
};
use crate::clauses::clausefunc::{
    tformula_tptp_parse, tformula_tptp_string, TFormulaTptpPrintOptions,
};
use crate::clauses::inferencedoc::ProofDocOutputFormat;
use crate::inout::scanner::{Scanner, TokenType};
use crate::pcl2::expressions::{PclExpression, PclOpCode};
use crate::pcl2::idents::PclId;
use crate::terms::termbanks::TermBank;
use crate::terms::termtypes::Term;
use std::fmt::Write as _;
use std::ops::{BitAnd, BitOr, BitOrAssign, Not};

pub const PCL_PROOF_DIST_INFINITY: i64 = i64::MAX;
pub const PCL_PROOF_DIST_DEFAULT: i64 = 10;
pub const PCL_PROOF_DIST_UNKNOWN: i64 = -1;
pub const PCL_NO_WEIGHT: i64 = -1;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PclStepProperties(u64);

impl PclStepProperties {
    #[must_use]
    pub const fn from_bits(bits: u64) -> Self {
        Self(bits)
    }

    #[must_use]
    pub const fn from_formula_properties(properties: FormulaProperties) -> Self {
        Self(properties.bits())
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

    pub fn set_type(&mut self, type_: Self) {
        self.delete(PCL_TYPE_MASK);
        self.set(type_);
    }

    #[must_use]
    pub const fn query_type(self) -> Self {
        self.give(PCL_TYPE_MASK)
    }

    #[must_use]
    pub const fn is_fof(self) -> bool {
        self.query(PCL_IS_FOF_STEP)
    }

    #[must_use]
    pub const fn is_shell(self) -> bool {
        self.query(PCL_IS_SHELL_STEP)
    }

    #[must_use]
    pub const fn is_clausal(self) -> bool {
        !self.is_fof()
    }
}

impl BitOr for PclStepProperties {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for PclStepProperties {
    fn bitor_assign(&mut self, rhs: Self) {
        self.set(rhs);
    }
}

impl BitAnd for PclStepProperties {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl Not for PclStepProperties {
    type Output = Self;

    fn not(self) -> Self::Output {
        Self(!self.0)
    }
}

pub const PCL_NO_PROP: PclStepProperties = PclStepProperties::from_bits(0);
pub const PCL_IS_LEMMA: PclStepProperties = PclStepProperties::from_bits(1);
pub const PCL_IS_INITIAL: PclStepProperties = PclStepProperties::from_bits(2);
pub const PCL_IS_FINAL: PclStepProperties = PclStepProperties::from_bits(4);
pub const PCL_IS_MARKED: PclStepProperties = PclStepProperties::from_bits(8);
pub const PCL_IS_PROOF_STEP: PclStepProperties = PclStepProperties::from_bits(16);
pub const PCL_IS_EXAMPLE: PclStepProperties = PclStepProperties::from_bits(32);
pub const PCL_IS_FOF_STEP: PclStepProperties = PclStepProperties::from_bits(64);
pub const PCL_IS_SHELL_STEP: PclStepProperties = PclStepProperties::from_bits(128);
pub const PCL_TYPE_1: PclStepProperties = PclStepProperties::from_formula_properties(CP_TYPE_1);
pub const PCL_TYPE_2: PclStepProperties = PclStepProperties::from_formula_properties(CP_TYPE_2);
pub const PCL_TYPE_3: PclStepProperties = PclStepProperties::from_formula_properties(CP_TYPE_3);
pub const PCL_TYPE_MASK: PclStepProperties =
    PclStepProperties::from_formula_properties(CP_TYPE_MASK);
pub const PCL_TYPE_UNKNOWN: PclStepProperties =
    PclStepProperties::from_formula_properties(CP_TYPE_UNKNOWN);
pub const PCL_TYPE_AXIOM: PclStepProperties =
    PclStepProperties::from_formula_properties(CP_TYPE_AXIOM);
pub const PCL_TYPE_HYPOTHESIS: PclStepProperties =
    PclStepProperties::from_formula_properties(crate::clauses::clause_props::CP_TYPE_HYPOTHESIS);
pub const PCL_TYPE_CONJECTURE: PclStepProperties =
    PclStepProperties::from_formula_properties(CP_TYPE_CONJECTURE);
pub const PCL_TYPE_NEG_CONJECTURE: PclStepProperties =
    PclStepProperties::from_formula_properties(CP_TYPE_NEG_CONJECTURE);
pub const PCL_TYPE_QUESTION: PclStepProperties =
    PclStepProperties::from_formula_properties(CP_TYPE_QUESTION);

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PclStepTreeData {
    pub proof_dag_size: i64,
    pub proof_tree_size: i64,
    pub active_pm_refs: i64,
    pub other_generating_refs: i64,
    pub active_simpl_refs: i64,
    pub passive_simpl_refs: i64,
    pub pure_quote_refs: i64,
    pub lemma_quality: f32,
    pub contrib_simpl_refs: i64,
    pub contrib_gen_refs: i64,
    pub useless_simpl_refs: i64,
    pub useless_gen_refs: i64,
    pub proof_distance: i64,
}

impl Default for PclStepTreeData {
    fn default() -> Self {
        let mut data = Self {
            proof_dag_size: 0,
            proof_tree_size: 0,
            active_pm_refs: 0,
            other_generating_refs: 0,
            active_simpl_refs: 0,
            passive_simpl_refs: 0,
            pure_quote_refs: 0,
            lemma_quality: 0.0,
            contrib_simpl_refs: 0,
            contrib_gen_refs: 0,
            useless_simpl_refs: 0,
            useless_gen_refs: 0,
            proof_distance: 0,
        };
        let mut properties = PCL_NO_PROP;
        data.reset(&mut properties, false);
        data
    }
}

impl PclStepTreeData {
    /// C `PCLStepResetTreeData`.
    pub fn reset(&mut self, properties: &mut PclStepProperties, just_weights: bool) {
        self.proof_dag_size = PCL_NO_WEIGHT;
        self.proof_tree_size = PCL_NO_WEIGHT;
        if !just_weights {
            self.active_pm_refs = 0;
            self.other_generating_refs = 0;
            self.active_simpl_refs = 0;
            self.passive_simpl_refs = 0;
            self.pure_quote_refs = 0;
            self.lemma_quality = 0.0;
            self.contrib_simpl_refs = 0;
            self.contrib_gen_refs = 0;
            self.useless_simpl_refs = 0;
            self.useless_gen_refs = 0;
            self.proof_distance = PCL_PROOF_DIST_UNKNOWN;
            properties.delete(PCL_IS_LEMMA | PCL_IS_MARKED);
        }
    }
}

/// C `PCLParseExternalType`.
///
/// # Errors
///
/// Returns scanner diagnostics when the annotation list contains an
/// unsupported identifier or misses a comma between non-colon tokens.
pub fn parse_external_type(scanner: &mut Scanner) -> Result<PclStepProperties, Diagnostic> {
    let mut type_ = PCL_TYPE_AXIOM;
    let mut extra = PCL_NO_PROP;

    while !scanner.test_tok(TokenType::COLON) {
        if scanner.test_id("conj") {
            type_ = PCL_TYPE_CONJECTURE;
            scanner.next_token()?;
        } else if scanner.test_id("que") {
            type_ = PCL_TYPE_QUESTION;
            scanner.next_token()?;
        } else if scanner.test_id("neg") {
            type_ = PCL_TYPE_NEG_CONJECTURE;
            scanner.next_token()?;
        } else if scanner.test_id("lemma") {
            extra = PCL_IS_LEMMA;
            scanner.next_token()?;
        } else {
            scanner.check_id("conj|neg|lemma")?;
        }
        if !scanner.test_tok(TokenType::COLON) {
            scanner.accept_tok(TokenType::COMMA)?;
        }
    }
    Ok(type_ | extra)
}

/// C `PCLPrintExternalType`.
#[must_use]
pub fn external_type_string(props: PclStepProperties) -> String {
    let mut output = String::new();
    let mut prepend = "";
    if props.query(PCL_IS_LEMMA) {
        output.push_str("lemma");
        prepend = ",";
    }
    match props.query_type() {
        PCL_TYPE_NEG_CONJECTURE => {
            output.push_str(prepend);
            output.push_str("neg");
        }
        PCL_TYPE_CONJECTURE => {
            output.push_str(prepend);
            output.push_str("conj");
        }
        PCL_TYPE_QUESTION => {
            output.push_str(prepend);
            output.push_str("que");
        }
        _ => {}
    }
    output
}

/// C `PCLPropToTSTPType`.
#[must_use]
pub const fn prop_to_tstp_type(props: PclStepProperties) -> &'static str {
    match props.query_type() {
        PCL_TYPE_CONJECTURE => "conjecture",
        PCL_TYPE_QUESTION => "question",
        PCL_TYPE_NEG_CONJECTURE => "negated_conjecture",
        _ if props.query(PCL_IS_LEMMA) => "lemma",
        _ if props.query(PCL_IS_INITIAL) => "axiom",
        _ => "plain",
    }
}

/// C `PCLStepIdCompare`, parameterized over already-ported identifiers.
#[must_use]
pub fn step_id_compare(left: &PclId, right: &PclId) -> i32 {
    left.compare_c_value(right)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PclStepParseOptions {
    pub problem_type: ProblemType,
    pub support_shell_pcl: bool,
}

impl Default for PclStepParseOptions {
    fn default() -> Self {
        Self {
            problem_type: ProblemType::FirstOrder,
            support_shell_pcl: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum PclStepLogic {
    Shell,
    Clause(Box<Clause>),
    Formula(Term),
}

#[derive(Clone, Debug, PartialEq)]
pub struct PclStep {
    id: PclId,
    logic: PclStepLogic,
    just: PclExpression,
    extra: Option<String>,
    properties: PclStepProperties,
    tree_data: PclStepTreeData,
}

impl PclStep {
    #[must_use]
    pub fn new(
        id: PclId,
        logic: PclStepLogic,
        just: PclExpression,
        extra: Option<String>,
        properties: PclStepProperties,
    ) -> Self {
        Self {
            id,
            logic,
            just,
            extra,
            properties,
            tree_data: PclStepTreeData::default(),
        }
    }

    #[must_use]
    pub const fn id(&self) -> &PclId {
        &self.id
    }

    #[must_use]
    pub const fn logic(&self) -> &PclStepLogic {
        &self.logic
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
    pub const fn properties(&self) -> PclStepProperties {
        self.properties
    }

    #[must_use]
    pub const fn tree_data(&self) -> &PclStepTreeData {
        &self.tree_data
    }

    pub const fn tree_data_mut(&mut self) -> &mut PclStepTreeData {
        &mut self.tree_data
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

    pub fn set_justification(&mut self, just: PclExpression) {
        self.just = just;
    }

    pub fn reset_tree_data(&mut self, just_weights: bool) {
        self.tree_data.reset(&mut self.properties, just_weights);
    }

    #[must_use]
    pub fn is_empty_clause(&self) -> bool {
        matches!(&self.logic, PclStepLogic::Clause(clause) if clause.is_empty())
    }

    /// C `PCLStepParse`.
    ///
    /// # Errors
    ///
    /// Returns scanner diagnostics for invalid full-step syntax or diagnostics
    /// from the underlying clause/formula/expression parsers.
    ///
    /// # Panics
    ///
    /// Panics if a clausal step is parsed while the scanner is not in TPTP
    /// format, matching the currently ported `ClausePCLParse` assertion.
    pub fn parse(
        scanner: &mut Scanner,
        bank: &mut TermBank,
        options: PclStepParseOptions,
    ) -> Result<Self, Diagnostic> {
        let id = PclId::parse(scanner)?;
        scanner.accept_tok(TokenType::COLON)?;
        let mut properties = parse_external_type(scanner)?;
        scanner.accept_tok(TokenType::COLON)?;

        let logic = if options.support_shell_pcl && scanner.test_tok(TokenType::COLON) {
            properties.set(PCL_IS_SHELL_STEP);
            PclStepLogic::Shell
        } else if scanner.test_tok(TokenType::OPEN_SQUARE) {
            let clause = clause_pcl_parse(scanner, bank, options.problem_type)?;
            properties.delete(PCL_IS_FOF_STEP);
            PclStepLogic::Clause(Box::new(clause))
        } else {
            let formula = tformula_tptp_parse(scanner, bank)?;
            properties.set(PCL_IS_FOF_STEP);
            PclStepLogic::Formula(formula)
        };

        scanner.accept_tok(TokenType::COLON)?;
        let just = PclExpression::parse(scanner, false)?;
        let extra = if scanner.test_tok(TokenType::COLON) {
            scanner.next_token()?;
            scanner.check_tok(TokenType::SQ_STRING | TokenType::NAME | TokenType::POS_INT)?;
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
            just,
            extra,
            properties,
            tree_data: PclStepTreeData::default(),
        })
    }

    /// C `PCLStepPrintExtra`.
    ///
    /// # Errors
    ///
    /// Returns diagnostics from temporary formula rendering.
    pub fn print_extra_string(
        &self,
        bank: &mut TermBank,
        problem_type: ProblemType,
        data: bool,
    ) -> Result<String, Diagnostic> {
        let mut output = self.id.print_formatted_string(true);
        output.push_str(" : ");
        output.push_str(&external_type_string(self.properties));
        output.push_str(" : ");
        if !self.is_shell() {
            match &self.logic {
                PclStepLogic::Shell => {}
                PclStepLogic::Clause(clause) => {
                    output.push_str(&clause_pcl_string(bank, clause, true));
                }
                PclStepLogic::Formula(formula) => {
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
        output.push_str(&self.just.print_string(false));
        if let Some(extra) = &self.extra {
            output.push_str(" : ");
            output.push_str(extra);
        } else if self.properties.query(PCL_IS_LEMMA) {
            output.push_str(" : 'lemma'");
        }
        if data {
            let _ = write!(
                output,
                " /* {:3} {:3} {:3} {:3} {:3}  */",
                self.tree_data.contrib_simpl_refs,
                self.tree_data.contrib_gen_refs,
                self.tree_data.useless_simpl_refs,
                self.tree_data.useless_gen_refs,
                self.tree_data.proof_distance
            );
        }
        Ok(output)
    }

    /// C `PCLStepPrintTSTP`.
    ///
    /// # Errors
    ///
    /// Returns diagnostics from temporary formula rendering.
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
                self.id.print_tstp_string(),
                prop_to_tstp_type(self.properties)
            );
            if !self.is_shell() {
                if let PclStepLogic::Clause(clause) = &self.logic {
                    output.push_str(&clause_print_tstp_core_string(bank, clause, true, false));
                }
            }
        } else {
            let _ = write!(
                output,
                "fof({},{},",
                self.id.print_tstp_string(),
                prop_to_tstp_type(self.properties)
            );
            if !self.is_shell() {
                if let PclStepLogic::Formula(formula) = &self.logic {
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
        output.push_str(&self.just.print_tstp_string(false));
        if let Some(extra) = &self.extra {
            output.push_str(",[");
            output.push_str(extra);
            output.push(']');
        }
        output.push_str(").");
        Ok(output)
    }

    /// C `PCLStepPrintTPTP`.
    ///
    /// # Errors
    ///
    /// Returns diagnostics from temporary formula rendering.
    pub fn print_tptp_string(
        &self,
        bank: &mut TermBank,
        problem_type: ProblemType,
    ) -> Result<String, Diagnostic> {
        if self.is_shell() {
            return Ok(self.shell_omitted_string());
        }
        match &self.logic {
            PclStepLogic::Shell => Ok(self.shell_omitted_string()),
            PclStepLogic::Clause(clause) => Ok(clause_print_tptp_format_string(bank, clause)),
            PclStepLogic::Formula(formula) => Ok(format!(
                "input_formula({},{},{})",
                self.id.print_tstp_string(),
                prop_to_tstp_type(self.properties),
                tformula_tptp_string(
                    bank,
                    formula,
                    true,
                    TFormulaTptpPrintOptions::tptp(problem_type),
                )?
            )),
        }
    }

    /// C `PCLStepPrintLOP`.
    ///
    /// # Errors
    ///
    /// Returns diagnostics from temporary formula rendering.
    pub fn print_lop_string(
        &self,
        bank: &mut TermBank,
        problem_type: ProblemType,
    ) -> Result<String, Diagnostic> {
        if self.is_shell() {
            return Ok(self.shell_omitted_string());
        }
        match &self.logic {
            PclStepLogic::Shell => Ok(self.shell_omitted_string()),
            PclStepLogic::Clause(clause) => Ok(clause_print_lop_format_string(bank, clause, true)),
            PclStepLogic::Formula(formula) => tformula_tptp_string(
                bank,
                formula,
                true,
                TFormulaTptpPrintOptions::tptp(problem_type),
            ),
        }
    }

    /// C `PCLStepPrintFormat`.
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
        data: bool,
        format: ProofDocOutputFormat,
    ) -> Result<String, Diagnostic> {
        match format {
            ProofDocOutputFormat::Pcl => self.print_extra_string(bank, problem_type, data),
            ProofDocOutputFormat::Lop => self.print_lop_string(bank, problem_type),
            ProofDocOutputFormat::Tptp => self.print_tptp_string(bank, problem_type),
            ProofDocOutputFormat::Tstp => self.print_tstp_string(bank, problem_type),
            _ => panic!("PCLStepPrintFormat supports only PCL, LOP, TPTP, and TSTP"),
        }
    }

    /// C `PCLStepPrintExample`.
    ///
    /// # Panics
    ///
    /// Panics for FOF steps, matching the C assertion.
    #[must_use]
    pub fn print_example_string(
        &self,
        bank: &TermBank,
        id: i64,
        proof_steps: i64,
        total_steps: i64,
    ) -> String {
        assert!(
            !self.is_fof(),
            "PCLStepPrintExample requires a clausal step"
        );
        if self.is_shell() {
            return self.shell_omitted_string();
        }
        let PclStepLogic::Clause(clause) = &self.logic else {
            return self.shell_omitted_string();
        };
        format!(
            "{id:4}:({}, {:.6},{:.6},{:.6},{:.6}):{}",
            self.tree_data.proof_distance,
            c_example_ratio(self.tree_data.contrib_simpl_refs, proof_steps + 1),
            c_example_ratio(
                self.tree_data.useless_simpl_refs,
                total_steps - proof_steps + 1,
            ),
            c_example_ratio(self.tree_data.contrib_gen_refs, proof_steps + 1),
            c_example_ratio(
                self.tree_data.useless_gen_refs,
                total_steps - proof_steps + 1,
            ),
            clause_print_lop_format_string(bank, clause, true)
        )
    }

    #[must_use]
    fn shell_omitted_string(&self) -> String {
        format!("# Step {} omitted (Shell)\n", self.id.print_string())
    }
}

/// C `PCLStepIdCompare`, parameterized over already-ported step cells.
#[must_use]
pub fn pcl_step_id_compare(left: &PclStep, right: &PclStep) -> i32 {
    step_id_compare(left.id(), right.id())
}

#[allow(clippy::cast_precision_loss)]
fn c_example_ratio(numerator: i64, denominator: i64) -> f64 {
    // C computes these with `(float)` denominators and `%f` output.
    f64::from(numerator as f32 / denominator as f32)
}

#[cfg(test)]
mod tests {
    use super::{
        external_type_string, parse_external_type, pcl_step_id_compare, prop_to_tstp_type,
        step_id_compare, PclStep, PclStepLogic, PclStepParseOptions, PclStepTreeData,
        PCL_IS_EXAMPLE, PCL_IS_FINAL, PCL_IS_FOF_STEP, PCL_IS_INITIAL, PCL_IS_LEMMA, PCL_IS_MARKED,
        PCL_IS_PROOF_STEP, PCL_IS_SHELL_STEP, PCL_NO_PROP, PCL_NO_WEIGHT, PCL_PROOF_DIST_DEFAULT,
        PCL_PROOF_DIST_INFINITY, PCL_PROOF_DIST_UNKNOWN, PCL_TYPE_1, PCL_TYPE_2, PCL_TYPE_3,
        PCL_TYPE_AXIOM, PCL_TYPE_CONJECTURE, PCL_TYPE_HYPOTHESIS, PCL_TYPE_MASK,
        PCL_TYPE_NEG_CONJECTURE, PCL_TYPE_QUESTION, PCL_TYPE_UNKNOWN,
    };
    use crate::basics::simple_stuff::ProblemType;
    use crate::clauses::inferencedoc::ProofDocOutputFormat;
    use crate::inout::scanner::{IoFormat, Scanner, TokenType};
    use crate::pcl2::idents::PclId;
    use crate::terms::signature::Signature;
    use crate::terms::termbanks::TermBank;
    use crate::terms::typebanks::TypeBank;

    fn parse_id(source: &str) -> PclId {
        let mut scanner = Scanner::from_user_string(source, false).unwrap();
        PclId::parse(&mut scanner).unwrap()
    }

    fn parse_type(source: &str) -> (super::PclStepProperties, Scanner) {
        let mut scanner = Scanner::from_user_string(source, false).unwrap();
        let props = parse_external_type(&mut scanner).unwrap();
        (props, scanner)
    }

    fn test_bank() -> TermBank {
        let mut signature = Signature::new(TypeBank::new());
        signature.insert_internal_codes().unwrap();
        TermBank::new(signature).unwrap()
    }

    fn parse_step(source: &str, support_shell_pcl: bool) -> (PclStep, TermBank, Scanner) {
        let mut bank = test_bank();
        let mut scanner = Scanner::from_user_string(source, false).unwrap();
        scanner.set_format(IoFormat::Tptp);
        let step = PclStep::parse(
            &mut scanner,
            &mut bank,
            PclStepParseOptions {
                problem_type: ProblemType::FirstOrder,
                support_shell_pcl,
            },
        )
        .unwrap();
        (step, bank, scanner)
    }

    #[test]
    fn constants_match_c_pcl_step_property_bits() {
        assert_eq!(PCL_PROOF_DIST_INFINITY, i64::MAX);
        assert_eq!(PCL_PROOF_DIST_DEFAULT, 10);
        assert_eq!(PCL_PROOF_DIST_UNKNOWN, -1);
        assert_eq!(PCL_NO_WEIGHT, -1);
        assert_eq!(PCL_NO_PROP.bits(), 0);
        assert_eq!(PCL_IS_LEMMA.bits(), 1);
        assert_eq!(PCL_IS_INITIAL.bits(), 2);
        assert_eq!(PCL_IS_FINAL.bits(), 4);
        assert_eq!(PCL_IS_MARKED.bits(), 8);
        assert_eq!(PCL_IS_PROOF_STEP.bits(), 16);
        assert_eq!(PCL_IS_EXAMPLE.bits(), 32);
        assert_eq!(PCL_IS_FOF_STEP.bits(), 64);
        assert_eq!(PCL_IS_SHELL_STEP.bits(), 128);
        assert_eq!(PCL_TYPE_1.bits(), 1024);
        assert_eq!(PCL_TYPE_2.bits(), 2048);
        assert_eq!(PCL_TYPE_3.bits(), 4096);
        assert_eq!(PCL_TYPE_MASK.bits(), 7168);
        assert_eq!(PCL_TYPE_UNKNOWN.bits(), 0);
        assert_eq!(PCL_TYPE_AXIOM.bits(), 1024);
        assert_eq!(PCL_TYPE_HYPOTHESIS.bits(), 2048);
        assert_eq!(PCL_TYPE_CONJECTURE.bits(), 3072);
        assert_eq!(PCL_TYPE_NEG_CONJECTURE.bits(), 5120);
        assert_eq!(PCL_TYPE_QUESTION.bits(), 6144);
    }

    #[test]
    fn property_helpers_follow_c_macros() {
        let mut props = PCL_NO_PROP;
        props.set(PCL_IS_INITIAL | PCL_IS_FOF_STEP | PCL_TYPE_AXIOM);
        assert!(props.query(PCL_IS_INITIAL | PCL_IS_FOF_STEP));
        assert!(props.is_any_set(PCL_IS_SHELL_STEP | PCL_IS_FOF_STEP));
        assert!(props.is_fof());
        assert!(!props.is_clausal());
        assert!(!props.is_shell());
        assert_eq!(props.give(PCL_TYPE_MASK), PCL_TYPE_AXIOM);

        props.set_type(PCL_TYPE_NEG_CONJECTURE);
        assert_eq!(props.query_type(), PCL_TYPE_NEG_CONJECTURE);
        assert!(props.query(PCL_IS_INITIAL | PCL_IS_FOF_STEP));

        props.delete(PCL_IS_FOF_STEP);
        assert!(props.is_clausal());
    }

    #[test]
    fn parses_external_type_lists_until_colon() {
        let (props, scanner) = parse_type("lemma,conj: rest");
        assert_eq!(props.query_type(), PCL_TYPE_CONJECTURE);
        assert!(props.query(PCL_IS_LEMMA));
        assert!(scanner.test_tok(TokenType::COLON));

        let (question, scanner) = parse_type("que,: rest");
        assert_eq!(question, PCL_TYPE_QUESTION);
        assert!(scanner.test_tok(TokenType::COLON));

        let (empty, scanner) = parse_type(": rest");
        assert_eq!(empty, PCL_TYPE_AXIOM);
        assert!(scanner.test_tok(TokenType::COLON));
    }

    #[test]
    fn parse_external_type_error_surface_omits_accepted_question_token_like_c() {
        let mut scanner = Scanner::from_user_string("bad: rest", false).unwrap();
        let error = parse_external_type(&mut scanner).unwrap_err();
        assert!(error.message().contains("conj|neg|lemma"));
        assert!(!error.message().contains("que"));
    }

    #[test]
    fn external_type_print_matches_c_order_and_empty_defaults() {
        assert_eq!(external_type_string(PCL_TYPE_AXIOM), "");
        assert_eq!(external_type_string(PCL_TYPE_HYPOTHESIS), "");
        assert_eq!(external_type_string(PCL_TYPE_CONJECTURE), "conj");
        assert_eq!(external_type_string(PCL_TYPE_QUESTION), "que");
        assert_eq!(external_type_string(PCL_TYPE_NEG_CONJECTURE), "neg");
        assert_eq!(external_type_string(PCL_IS_LEMMA | PCL_TYPE_AXIOM), "lemma");
        assert_eq!(
            external_type_string(PCL_IS_LEMMA | PCL_TYPE_CONJECTURE),
            "lemma,conj"
        );
    }

    #[test]
    fn prop_to_tstp_type_preserves_initial_axiom_gate() {
        assert_eq!(prop_to_tstp_type(PCL_TYPE_CONJECTURE), "conjecture");
        assert_eq!(prop_to_tstp_type(PCL_TYPE_QUESTION), "question");
        assert_eq!(
            prop_to_tstp_type(PCL_TYPE_NEG_CONJECTURE),
            "negated_conjecture"
        );
        assert_eq!(prop_to_tstp_type(PCL_IS_LEMMA | PCL_TYPE_AXIOM), "lemma");
        assert_eq!(prop_to_tstp_type(PCL_IS_INITIAL | PCL_TYPE_AXIOM), "axiom");
        assert_eq!(prop_to_tstp_type(PCL_TYPE_AXIOM), "plain");
        assert_eq!(prop_to_tstp_type(PCL_TYPE_HYPOTHESIS), "plain");
    }

    #[test]
    fn reset_tree_data_resets_weights_or_all_analysis_fields_like_c() {
        let mut data = PclStepTreeData {
            proof_dag_size: 12,
            proof_tree_size: 13,
            active_pm_refs: 1,
            other_generating_refs: 2,
            active_simpl_refs: 3,
            passive_simpl_refs: 4,
            pure_quote_refs: 5,
            lemma_quality: 0.75,
            contrib_simpl_refs: 6,
            contrib_gen_refs: 7,
            useless_simpl_refs: 8,
            useless_gen_refs: 9,
            proof_distance: 10,
        };
        let mut props = PCL_IS_LEMMA | PCL_IS_MARKED | PCL_IS_INITIAL;

        data.reset(&mut props, true);
        assert_eq!(data.proof_dag_size, PCL_NO_WEIGHT);
        assert_eq!(data.proof_tree_size, PCL_NO_WEIGHT);
        assert_eq!(data.active_pm_refs, 1);
        assert!(props.query(PCL_IS_LEMMA | PCL_IS_MARKED));

        data.reset(&mut props, false);
        assert_eq!(data.active_pm_refs, 0);
        assert_eq!(data.other_generating_refs, 0);
        assert_eq!(data.lemma_quality.to_bits(), 0.0_f32.to_bits());
        assert_eq!(data.proof_distance, PCL_PROOF_DIST_UNKNOWN);
        assert!(!props.is_any_set(PCL_IS_LEMMA | PCL_IS_MARKED));
        assert!(props.query(PCL_IS_INITIAL));
    }

    #[test]
    fn default_tree_data_matches_full_reset_shape() {
        let data = PclStepTreeData::default();
        assert_eq!(data.proof_dag_size, PCL_NO_WEIGHT);
        assert_eq!(data.proof_tree_size, PCL_NO_WEIGHT);
        assert_eq!(data.proof_distance, PCL_PROOF_DIST_UNKNOWN);
    }

    #[test]
    fn step_id_compare_delegates_to_c_identifier_comparison() {
        assert_eq!(step_id_compare(&parse_id("1.2"), &parse_id("1.2")), 0);
        assert!(step_id_compare(&parse_id("1.2"), &parse_id("1.3")) < 0);
        assert!(step_id_compare(&parse_id("2"), &parse_id("1.999")) > 0);
    }

    #[test]
    fn parses_clause_step_and_prints_pcl_and_tstp_shapes() {
        let (step, mut bank, scanner) = parse_step(
            "42 : lemma,conj : [++p(a),--q(a)] : initial : 'extra' tail",
            false,
        );

        assert_eq!(step.id().elements(), [42]);
        assert!(matches!(step.logic(), PclStepLogic::Clause(_)));
        assert!(step.properties().query(PCL_IS_LEMMA | PCL_IS_INITIAL));
        assert_eq!(step.properties().query_type(), PCL_TYPE_CONJECTURE);
        assert!(!step.properties().is_fof());
        assert_eq!(step.extra(), Some("'extra'"));
        assert_eq!(scanner.current_token().literal(), "tail");

        assert_eq!(
            step.print_extra_string(&mut bank, ProblemType::FirstOrder, false)
                .unwrap(),
            "     42 : lemma,conj : [++p(a),--q(a)] : initial : 'extra'"
        );
        assert_eq!(
            step.print_tstp_string(&mut bank, ProblemType::FirstOrder)
                .unwrap(),
            "cnf(42,conjecture,(p(a)|~q(a)),unknown(),['extra'])."
        );
    }

    #[test]
    fn parses_formula_step_and_uses_full_identifier_quotes() {
        let (step, mut bank, scanner) = parse_step("7.2 : neg : p(a)|q(a) : 3.4 tail", false);

        assert_eq!(step.id().print_tstp_string(), "pclid7_2");
        assert!(matches!(step.logic(), PclStepLogic::Formula(_)));
        assert!(step.properties().query(PCL_IS_FOF_STEP));
        assert_eq!(step.properties().query_type(), PCL_TYPE_NEG_CONJECTURE);
        assert_eq!(scanner.current_token().literal(), "tail");

        assert_eq!(
            step.print_extra_string(&mut bank, ProblemType::FirstOrder, false)
                .unwrap(),
            "      7.2 : neg : (p(a)|q(a)) : 3.4"
        );
        assert_eq!(
            step.print_tstp_string(&mut bank, ProblemType::FirstOrder)
                .unwrap(),
            "fof(pclid7_2,negated_conjecture,(p(a)|q(a)),pclid3_4)."
        );
        assert_eq!(
            step.print_tptp_string(&mut bank, ProblemType::FirstOrder)
                .unwrap(),
            "input_formula(pclid7_2,negated_conjecture,(p(a)|q(a)))"
        );
    }

    #[test]
    fn lemma_without_extra_gets_implicit_pcl_lemma_comment_only() {
        let (step, mut bank, _) = parse_step("5 : lemma : [++p] : initial", false);

        assert_eq!(
            step.print_extra_string(&mut bank, ProblemType::FirstOrder, false)
                .unwrap(),
            "      5 : lemma : [++p] : initial : 'lemma'"
        );
        assert_eq!(
            step.print_tstp_string(&mut bank, ProblemType::FirstOrder)
                .unwrap(),
            "cnf(5,lemma,(p),unknown())."
        );
    }

    #[test]
    fn shell_steps_require_option_and_print_omitted_for_logical_formats() {
        let (step, mut bank, _) = parse_step("3 : : : 2 : final", true);

        assert!(matches!(step.logic(), PclStepLogic::Shell));
        assert!(step.properties().query(PCL_IS_SHELL_STEP));
        assert_eq!(step.extra(), Some("final"));
        assert_eq!(
            step.print_extra_string(&mut bank, ProblemType::FirstOrder, false)
                .unwrap(),
            "      3 :  :  : 2 : final"
        );
        assert_eq!(
            step.print_tstp_string(&mut bank, ProblemType::FirstOrder)
                .unwrap(),
            "cnf(3,plain,,2,[final])."
        );
        assert_eq!(
            step.print_lop_string(&mut bank, ProblemType::FirstOrder)
                .unwrap(),
            "# Step 3 omitted (Shell)\n"
        );
        assert_eq!(
            step.print_tptp_string(&mut bank, ProblemType::FirstOrder)
                .unwrap(),
            "# Step 3 omitted (Shell)\n"
        );
    }

    #[test]
    fn full_step_extra_accepts_names_and_positive_integers() {
        let (named, _, _) = parse_step("1 : : [++p] : initial : proof", false);
        assert_eq!(named.extra(), Some("proof"));

        let (numbered, _, _) = parse_step("2 : : [++q] : 1 : 99", false);
        assert_eq!(numbered.extra(), Some("99"));
    }

    #[test]
    fn print_format_dispatches_all_c_supported_full_step_formats() {
        let (step, mut bank, _) = parse_step("4 : : [++p] : initial", false);

        assert_eq!(
            step.print_format_string(
                &mut bank,
                ProblemType::FirstOrder,
                false,
                ProofDocOutputFormat::Pcl,
            )
            .unwrap(),
            "      4 :  : [++p] : initial"
        );
        assert_eq!(
            step.print_format_string(
                &mut bank,
                ProblemType::FirstOrder,
                false,
                ProofDocOutputFormat::Tstp,
            )
            .unwrap(),
            "cnf(4,axiom,(p),unknown())."
        );
        let tptp = step
            .print_format_string(
                &mut bank,
                ProblemType::FirstOrder,
                false,
                ProofDocOutputFormat::Tptp,
            )
            .unwrap();
        assert!(tptp.starts_with("input_clause(i_0_"));
        assert!(tptp.ends_with(",axiom,[++p])."));
        assert_eq!(
            step.print_format_string(
                &mut bank,
                ProblemType::FirstOrder,
                false,
                ProofDocOutputFormat::Lop,
            )
            .unwrap(),
            "p <- ."
        );
    }

    #[test]
    fn print_extra_data_and_example_use_analysis_counters() {
        let (mut step, mut bank, _) = parse_step("8 : : [++p] : initial", false);
        step.tree_data_mut().contrib_simpl_refs = 2;
        step.tree_data_mut().contrib_gen_refs = 4;
        step.tree_data_mut().useless_simpl_refs = 6;
        step.tree_data_mut().useless_gen_refs = 8;
        step.tree_data_mut().proof_distance = 3;

        assert_eq!(
            step.print_extra_string(&mut bank, ProblemType::FirstOrder, true)
                .unwrap(),
            "      8 :  : [++p] : initial /*   2   4   6   8   3  */"
        );
        assert_eq!(
            step.print_example_string(&bank, 11, 3, 9),
            "  11:(3, 0.500000,0.857143,1.000000,1.142857):p <- ."
        );
    }

    #[test]
    fn full_step_id_compare_uses_full_identifier_comparison() {
        let (left, _, _) = parse_step("1.2 : : [++p] : initial", false);
        let (right, _, _) = parse_step("1.3 : : [++p] : initial", false);

        assert!(pcl_step_id_compare(&left, &right) < 0);
    }
}
