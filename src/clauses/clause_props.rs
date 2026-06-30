use crate::basics::simple_stuff::ProblemType;
use std::ops::{BitAnd, BitOr, BitOrAssign, Not};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct FormulaProperties(u64);

impl FormulaProperties {
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

    pub fn set_tptp_type(&mut self, type_: Self) {
        self.delete(CP_TYPE_MASK);
        self.set(type_);
    }

    #[must_use]
    pub const fn query_tptp_type(self) -> Self {
        self.give(CP_TYPE_MASK)
    }

    pub fn set_csscpa_source(&mut self, source: u64) {
        self.delete(CP_CSSCPA_MASK);
        self.set(Self(source.saturating_mul(CP_CSSCPA_1.bits())));
    }

    #[must_use]
    pub const fn query_csscpa_source(self) -> u64 {
        self.give(CP_CSSCPA_MASK).bits() / CP_CSSCPA_1.bits()
    }

    #[must_use]
    pub const fn is_hypothesis(self) -> bool {
        matches!(self.query_tptp_type(), CP_TYPE_HYPOTHESIS)
    }

    #[must_use]
    pub const fn is_conjecture(self) -> bool {
        matches!(
            self.query_tptp_type(),
            CP_TYPE_CONJECTURE | CP_TYPE_NEG_CONJECTURE | CP_TYPE_QUESTION
        )
    }
}

impl BitOr for FormulaProperties {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for FormulaProperties {
    fn bitor_assign(&mut self, rhs: Self) {
        self.set(rhs);
    }
}

impl BitAnd for FormulaProperties {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl Not for FormulaProperties {
    type Output = Self;

    fn not(self) -> Self::Output {
        Self(!self.0)
    }
}

pub const CP_IGNORE_PROPS: FormulaProperties = FormulaProperties::from_bits(0);
pub const CP_INITIAL: FormulaProperties = FormulaProperties::from_bits(1);
pub const CP_INPUT_FORMULA: FormulaProperties = FormulaProperties::from_bits(2);
pub const CP_IS_DEAD: FormulaProperties = FormulaProperties::from_bits(4);
pub const CP_IS_PROCESSED: FormulaProperties = FormulaProperties::from_bits(8);
pub const CP_IS_ORIENTED: FormulaProperties = FormulaProperties::from_bits(16);
pub const CP_IS_D_INDEXED: FormulaProperties = FormulaProperties::from_bits(32);
pub const CP_IS_S_INDEXED: FormulaProperties = FormulaProperties::from_bits(64);
pub const CP_IS_GLOBAL_INDEXED: FormulaProperties = FormulaProperties::from_bits(128);
pub const CP_RW_DETECTED: FormulaProperties = FormulaProperties::from_bits(256);
pub const CP_DELETE_CLAUSE: FormulaProperties = FormulaProperties::from_bits(512);
pub const CP_TYPE_1: FormulaProperties = FormulaProperties::from_bits(1024);
pub const CP_TYPE_2: FormulaProperties = FormulaProperties::from_bits(2048);
pub const CP_TYPE_3: FormulaProperties = FormulaProperties::from_bits(4096);
pub const CP_TYPE_MASK: FormulaProperties =
    FormulaProperties::from_bits(CP_TYPE_1.bits() | CP_TYPE_2.bits() | CP_TYPE_3.bits());
pub const CP_TYPE_UNKNOWN: FormulaProperties = FormulaProperties::from_bits(0);
pub const CP_TYPE_AXIOM: FormulaProperties = CP_TYPE_1;
pub const CP_TYPE_HYPOTHESIS: FormulaProperties = CP_TYPE_2;
pub const CP_TYPE_CONJECTURE: FormulaProperties =
    FormulaProperties::from_bits(CP_TYPE_1.bits() | CP_TYPE_2.bits());
pub const CP_TYPE_LEMMA: FormulaProperties = CP_TYPE_3;
pub const CP_TYPE_NEG_CONJECTURE: FormulaProperties =
    FormulaProperties::from_bits(CP_TYPE_1.bits() | CP_TYPE_3.bits());
pub const CP_TYPE_QUESTION: FormulaProperties =
    FormulaProperties::from_bits(CP_TYPE_2.bits() | CP_TYPE_3.bits());
pub const CP_TYPE_WATCH_CLAUSE: FormulaProperties = CP_TYPE_MASK;
pub const CP_IS_IR_VICTIM: FormulaProperties = FormulaProperties::from_bits(8192);
pub const CP_OP_FLAG: FormulaProperties = FormulaProperties::from_bits(16_384);
pub const CP_IS_SELECTED: FormulaProperties = FormulaProperties::from_bits(32_768);
pub const CP_IS_FINAL: FormulaProperties = FormulaProperties::from_bits(65_536);
pub const CP_IS_PROOF_CLAUSE: FormulaProperties = FormulaProperties::from_bits(131_072);
pub const CP_IS_SOS: FormulaProperties = FormulaProperties::from_bits(262_144);
pub const CP_NO_GENERATION: FormulaProperties = FormulaProperties::from_bits(524_288);
pub const CP_CSSCPA_1: FormulaProperties = FormulaProperties::from_bits(1_048_576);
pub const CP_CSSCPA_2: FormulaProperties = FormulaProperties::from_bits(2_097_152);
pub const CP_CSSCPA_4: FormulaProperties = FormulaProperties::from_bits(4_194_304);
pub const CP_CSSCPA_8: FormulaProperties = FormulaProperties::from_bits(8_388_608);
pub const CP_CSSCPA_MASK: FormulaProperties = FormulaProperties::from_bits(
    CP_CSSCPA_1.bits() | CP_CSSCPA_2.bits() | CP_CSSCPA_4.bits() | CP_CSSCPA_8.bits(),
);
pub const CP_CSSCPA_UNKNOWN: FormulaProperties = FormulaProperties::from_bits(0);
pub const CP_IS_PROTECTED: FormulaProperties = FormulaProperties::from_bits(16_777_216);
pub const CP_WATCH_ONLY: FormulaProperties = FormulaProperties::from_bits(33_554_432);
pub const CP_SUBSUMES_WATCH: FormulaProperties = FormulaProperties::from_bits(67_108_864);
pub const CP_LIMITED_RW: FormulaProperties = FormulaProperties::from_bits(134_217_728);
pub const CP_IS_RELEVANT: FormulaProperties = FormulaProperties::from_bits(268_435_456);
pub const CP_IS_PURE_INJECTIVITY: FormulaProperties = FormulaProperties::from_bits(536_870_912);
pub const CP_IS_LAMBDA_DEF: FormulaProperties = FormulaProperties::from_bits(1_073_741_824);

#[must_use]
pub const fn tptp_types_combine(
    type1: FormulaProperties,
    type2: FormulaProperties,
) -> FormulaProperties {
    if matches!(type1, CP_TYPE_AXIOM) {
        type2
    } else if matches!(type2, CP_TYPE_CONJECTURE) {
        CP_TYPE_CONJECTURE
    } else {
        type1
    }
}

#[must_use]
pub fn clause_type_from_identifier(
    identifier: &str,
    problem_type: ProblemType,
) -> FormulaProperties {
    match identifier {
        "axiom" | "theorem" => CP_TYPE_AXIOM,
        "definition" => {
            if problem_type == ProblemType::HigherOrder {
                CP_TYPE_AXIOM | CP_IS_LAMBDA_DEF
            } else {
                CP_TYPE_AXIOM
            }
        }
        "question" => CP_TYPE_QUESTION,
        "conjecture" => CP_TYPE_CONJECTURE,
        "assumption" | "negated_conjecture" => CP_TYPE_NEG_CONJECTURE,
        "hypothesis" => CP_TYPE_HYPOTHESIS,
        "lemma" => CP_TYPE_LEMMA,
        "watchlist" => CP_TYPE_WATCH_CLAUSE,
        _ => CP_TYPE_UNKNOWN,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        clause_type_from_identifier, tptp_types_combine, FormulaProperties, CP_CSSCPA_1,
        CP_CSSCPA_2, CP_CSSCPA_4, CP_CSSCPA_8, CP_CSSCPA_MASK, CP_CSSCPA_UNKNOWN, CP_DELETE_CLAUSE,
        CP_IGNORE_PROPS, CP_INITIAL, CP_INPUT_FORMULA, CP_IS_DEAD, CP_IS_D_INDEXED, CP_IS_FINAL,
        CP_IS_GLOBAL_INDEXED, CP_IS_IR_VICTIM, CP_IS_LAMBDA_DEF, CP_IS_ORIENTED, CP_IS_PROCESSED,
        CP_IS_PROOF_CLAUSE, CP_IS_PROTECTED, CP_IS_PURE_INJECTIVITY, CP_IS_RELEVANT,
        CP_IS_SELECTED, CP_IS_SOS, CP_IS_S_INDEXED, CP_LIMITED_RW, CP_NO_GENERATION, CP_OP_FLAG,
        CP_RW_DETECTED, CP_SUBSUMES_WATCH, CP_TYPE_1, CP_TYPE_2, CP_TYPE_3, CP_TYPE_AXIOM,
        CP_TYPE_CONJECTURE, CP_TYPE_HYPOTHESIS, CP_TYPE_LEMMA, CP_TYPE_MASK,
        CP_TYPE_NEG_CONJECTURE, CP_TYPE_QUESTION, CP_TYPE_UNKNOWN, CP_TYPE_WATCH_CLAUSE,
        CP_WATCH_ONLY,
    };
    use crate::basics::simple_stuff::ProblemType;

    #[test]
    fn constants_match_c_formula_property_bits() {
        assert_eq!(CP_IGNORE_PROPS.bits(), 0);
        assert_eq!(CP_INITIAL.bits(), 1);
        assert_eq!(CP_INPUT_FORMULA.bits(), 2);
        assert_eq!(CP_IS_DEAD.bits(), 4);
        assert_eq!(CP_IS_PROCESSED.bits(), 8);
        assert_eq!(CP_IS_ORIENTED.bits(), 16);
        assert_eq!(CP_IS_D_INDEXED.bits(), 32);
        assert_eq!(CP_IS_S_INDEXED.bits(), 64);
        assert_eq!(CP_IS_GLOBAL_INDEXED.bits(), 128);
        assert_eq!(CP_RW_DETECTED.bits(), 256);
        assert_eq!(CP_DELETE_CLAUSE.bits(), 512);
        assert_eq!(CP_TYPE_1.bits(), 1024);
        assert_eq!(CP_TYPE_2.bits(), 2048);
        assert_eq!(CP_TYPE_3.bits(), 4096);
        assert_eq!(CP_TYPE_MASK.bits(), 7168);
        assert_eq!(CP_TYPE_UNKNOWN.bits(), 0);
        assert_eq!(CP_TYPE_AXIOM.bits(), 1024);
        assert_eq!(CP_TYPE_HYPOTHESIS.bits(), 2048);
        assert_eq!(CP_TYPE_CONJECTURE.bits(), 3072);
        assert_eq!(CP_TYPE_LEMMA.bits(), 4096);
        assert_eq!(CP_TYPE_NEG_CONJECTURE.bits(), 5120);
        assert_eq!(CP_TYPE_QUESTION.bits(), 6144);
        assert_eq!(CP_TYPE_WATCH_CLAUSE.bits(), 7168);
        assert_eq!(CP_IS_IR_VICTIM.bits(), 8192);
        assert_eq!(CP_OP_FLAG.bits(), 16_384);
        assert_eq!(CP_IS_SELECTED.bits(), 32_768);
        assert_eq!(CP_IS_FINAL.bits(), 65_536);
        assert_eq!(CP_IS_PROOF_CLAUSE.bits(), 131_072);
        assert_eq!(CP_IS_SOS.bits(), 262_144);
        assert_eq!(CP_NO_GENERATION.bits(), 524_288);
        assert_eq!(CP_CSSCPA_1.bits(), 1_048_576);
        assert_eq!(CP_CSSCPA_2.bits(), 2_097_152);
        assert_eq!(CP_CSSCPA_4.bits(), 4_194_304);
        assert_eq!(CP_CSSCPA_8.bits(), 8_388_608);
        assert_eq!(CP_CSSCPA_MASK.bits(), 15_728_640);
        assert_eq!(CP_CSSCPA_UNKNOWN.bits(), 0);
        assert_eq!(CP_IS_PROTECTED.bits(), 16_777_216);
        assert_eq!(CP_WATCH_ONLY.bits(), 33_554_432);
        assert_eq!(CP_SUBSUMES_WATCH.bits(), 67_108_864);
        assert_eq!(CP_LIMITED_RW.bits(), 134_217_728);
        assert_eq!(CP_IS_RELEVANT.bits(), 268_435_456);
        assert_eq!(CP_IS_PURE_INJECTIVITY.bits(), 536_870_912);
        assert_eq!(CP_IS_LAMBDA_DEF.bits(), 1_073_741_824);
    }

    #[test]
    fn property_helpers_match_c_macros() {
        let mut props = CP_IGNORE_PROPS;
        props.set(CP_INITIAL | CP_IS_SOS);

        assert!(props.query(CP_INITIAL | CP_IS_SOS));
        assert!(props.is_any_set(CP_IS_SOS | CP_IS_DEAD));
        assert_eq!(props.give(CP_INITIAL | CP_IS_DEAD), CP_INITIAL);

        props.delete(CP_INITIAL);
        assert!(!props.query(CP_INITIAL));
        props |= CP_IS_FINAL;
        assert!(props.query(CP_IS_FINAL));
        assert_eq!((props & CP_IS_FINAL), CP_IS_FINAL);
        assert_eq!((!CP_IGNORE_PROPS).give(CP_IS_FINAL), CP_IS_FINAL);
    }

    #[test]
    fn tptp_type_helpers_clear_only_type_bits() {
        let mut props = CP_INITIAL | CP_TYPE_AXIOM | CP_IS_SOS;

        props.set_tptp_type(CP_TYPE_NEG_CONJECTURE);

        assert_eq!(props.query_tptp_type(), CP_TYPE_NEG_CONJECTURE);
        assert!(props.query(CP_INITIAL | CP_IS_SOS));
        assert!(props.is_conjecture());
        assert!(!props.is_hypothesis());

        props.set_tptp_type(CP_TYPE_QUESTION);
        assert!(props.is_conjecture());

        props.set_tptp_type(CP_TYPE_HYPOTHESIS);
        assert!(props.is_hypothesis());
        assert!(!props.is_conjecture());
    }

    #[test]
    fn csscpa_source_encoding_preserves_unchecked_c_multiplication() {
        let mut props = CP_INITIAL | CP_CSSCPA_1 | CP_CSSCPA_8;
        props.set_csscpa_source(9);

        assert_eq!(props.query_csscpa_source(), 9);
        assert!(props.query(CP_INITIAL));

        props.set_csscpa_source(16);
        assert_eq!(props.query_csscpa_source(), 0);
        assert!(props.query(CP_IS_PROTECTED));
    }

    #[test]
    fn tptp_type_combination_matches_macro_precedence() {
        assert_eq!(
            tptp_types_combine(CP_TYPE_AXIOM, CP_TYPE_HYPOTHESIS),
            CP_TYPE_HYPOTHESIS
        );
        assert_eq!(
            tptp_types_combine(CP_TYPE_LEMMA, CP_TYPE_CONJECTURE),
            CP_TYPE_CONJECTURE
        );
        assert_eq!(
            tptp_types_combine(CP_TYPE_LEMMA, CP_TYPE_HYPOTHESIS),
            CP_TYPE_LEMMA
        );
        assert_eq!(
            tptp_types_combine(CP_TYPE_AXIOM | CP_IS_LAMBDA_DEF, CP_TYPE_CONJECTURE),
            CP_TYPE_CONJECTURE
        );
    }

    #[test]
    fn clause_type_identifier_mapping_matches_c_parser_table() {
        assert_eq!(
            clause_type_from_identifier("axiom", ProblemType::FirstOrder),
            CP_TYPE_AXIOM
        );
        assert_eq!(
            clause_type_from_identifier("theorem", ProblemType::FirstOrder),
            CP_TYPE_AXIOM
        );
        assert_eq!(
            clause_type_from_identifier("definition", ProblemType::FirstOrder),
            CP_TYPE_AXIOM
        );
        assert_eq!(
            clause_type_from_identifier("definition", ProblemType::HigherOrder),
            CP_TYPE_AXIOM | CP_IS_LAMBDA_DEF
        );
        assert_eq!(
            clause_type_from_identifier("question", ProblemType::FirstOrder),
            CP_TYPE_QUESTION
        );
        assert_eq!(
            clause_type_from_identifier("conjecture", ProblemType::FirstOrder),
            CP_TYPE_CONJECTURE
        );
        assert_eq!(
            clause_type_from_identifier("assumption", ProblemType::FirstOrder),
            CP_TYPE_NEG_CONJECTURE
        );
        assert_eq!(
            clause_type_from_identifier("negated_conjecture", ProblemType::FirstOrder),
            CP_TYPE_NEG_CONJECTURE
        );
        assert_eq!(
            clause_type_from_identifier("hypothesis", ProblemType::FirstOrder),
            CP_TYPE_HYPOTHESIS
        );
        assert_eq!(
            clause_type_from_identifier("lemma", ProblemType::FirstOrder),
            CP_TYPE_LEMMA
        );
        assert_eq!(
            clause_type_from_identifier("watchlist", ProblemType::FirstOrder),
            CP_TYPE_WATCH_CLAUSE
        );
        assert_eq!(
            clause_type_from_identifier("unknown", ProblemType::FirstOrder),
            FormulaProperties::default()
        );
    }
}
