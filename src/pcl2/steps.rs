//! Shared primitives from `PCL2/pcl_steps`.

use crate::basics::error::Diagnostic;
use crate::clauses::clause_props::{
    FormulaProperties, CP_TYPE_1, CP_TYPE_2, CP_TYPE_3, CP_TYPE_AXIOM, CP_TYPE_CONJECTURE,
    CP_TYPE_MASK, CP_TYPE_NEG_CONJECTURE, CP_TYPE_QUESTION, CP_TYPE_UNKNOWN,
};
use crate::inout::scanner::{Scanner, TokenType};
use crate::pcl2::idents::PclId;
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

#[cfg(test)]
mod tests {
    use super::{
        external_type_string, parse_external_type, prop_to_tstp_type, step_id_compare,
        PclStepTreeData, PCL_IS_EXAMPLE, PCL_IS_FINAL, PCL_IS_FOF_STEP, PCL_IS_INITIAL,
        PCL_IS_LEMMA, PCL_IS_MARKED, PCL_IS_PROOF_STEP, PCL_IS_SHELL_STEP, PCL_NO_PROP,
        PCL_NO_WEIGHT, PCL_PROOF_DIST_DEFAULT, PCL_PROOF_DIST_INFINITY, PCL_PROOF_DIST_UNKNOWN,
        PCL_TYPE_1, PCL_TYPE_2, PCL_TYPE_3, PCL_TYPE_AXIOM, PCL_TYPE_CONJECTURE,
        PCL_TYPE_HYPOTHESIS, PCL_TYPE_MASK, PCL_TYPE_NEG_CONJECTURE, PCL_TYPE_QUESTION,
        PCL_TYPE_UNKNOWN,
    };
    use crate::inout::scanner::{Scanner, TokenType};
    use crate::pcl2::idents::PclId;

    fn parse_id(source: &str) -> PclId {
        let mut scanner = Scanner::from_user_string(source, false).unwrap();
        PclId::parse(&mut scanner).unwrap()
    }

    fn parse_type(source: &str) -> (super::PclStepProperties, Scanner) {
        let mut scanner = Scanner::from_user_string(source, false).unwrap();
        let props = parse_external_type(&mut scanner).unwrap();
        (props, scanner)
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
}
