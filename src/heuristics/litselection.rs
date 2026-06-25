use crate::clauses::clause::Clause;
use crate::clauses::clause_props::CP_IS_ORIENTED;
use crate::clauses::eqn::Eqn;
use crate::clauses::eqn_props::{
    EP_IS_EQU_LITERAL, EP_IS_MAXIMAL, EP_IS_PM_INTO_LIT, EP_IS_POSITIVE, EP_IS_SELECTED,
};
use crate::orderings::ocb::OrderControlBlock;
use crate::terms::termbanks::TermBank;
use crate::terms::termfunc::{term_standard_weight, term_weight_compute};
use std::sync::atomic::{AtomicI64, Ordering as AtomicOrdering};

pub const NO_SELECTION: &str = "NoSelection";
pub const NO_GENERATION: &str = "NoGeneration";
pub const SELECT_NEGATIVE_LITERALS: &str = "SelectNegativeLiterals";
pub const P_SELECT_NEGATIVE_LITERALS: &str = "PSelectNegativeLiterals";
pub const SELECT_PURE_VAR_NEG_LITERALS: &str = "SelectPureVarNegLiterals";
pub const P_SELECT_PURE_VAR_NEG_LITERALS: &str = "PSelectPureVarNegLiterals";
pub const SELECT_LARGEST_NEG_LIT: &str = "SelectLargestNegLit";
pub const P_SELECT_LARGEST_NEG_LIT: &str = "PSelectLargestNegLit";
pub const SELECT_SMALLEST_NEG_LIT: &str = "SelectSmallestNegLit";
pub const P_SELECT_SMALLEST_NEG_LIT: &str = "PSelectSmallestNegLit";
pub const SELECT_LARGEST_ORIENTABLE: &str = "SelectLargestOrientable";
pub const P_SELECT_LARGEST_ORIENTABLE: &str = "PSelectLargestOrientable";
pub const M_SELECT_LARGEST_ORIENTABLE: &str = "MSelectLargestOrientable";
pub const SELECT_SMALLEST_ORIENTABLE: &str = "SelectSmallestOrientable";
pub const P_SELECT_SMALLEST_ORIENTABLE: &str = "PSelectSmallestOrientable";
pub const M_SELECT_SMALLEST_ORIENTABLE: &str = "MSelectSmallestOrientable";
pub const SELECT_DIFF_NEG_LIT: &str = "SelectDiffNegLit";
pub const P_SELECT_DIFF_NEG_LIT: &str = "PSelectDiffNegLit";
pub const SELECT_GROUND_NEG_LIT: &str = "SelectGroundNegLit";
pub const P_SELECT_GROUND_NEG_LIT: &str = "PSelectGroundNegLit";
pub const SELECT_OPTIMAL_LIT: &str = "SelectOptimalLit";
pub const P_SELECT_OPTIMAL_LIT: &str = "PSelectOptimalLit";
pub const SELECT_MIN_OPTIMAL_LIT: &str = "SelectMinOptimalLit";
pub const P_SELECT_MIN_OPTIMAL_LIT: &str = "PSelectMinOptimalLit";
pub const SELECT_NON_RR_OPTIMAL_LIT: &str = "SelectNonRROptimalLit";
pub const P_SELECT_NON_RR_OPTIMAL_LIT: &str = "PSelectNonRROptimalLit";
pub const SELECT_NON_STRONG_RR_OPTIMAL_LIT: &str = "SelectNonStrongRROptimalLit";
pub const P_SELECT_NON_STRONG_RR_OPTIMAL_LIT: &str = "PSelectNonStrongRROptimalLit";
pub const SELECT_ANTI_RR_OPTIMAL_LIT: &str = "SelectAntiRROptimalLit";
pub const P_SELECT_ANTI_RR_OPTIMAL_LIT: &str = "PSelectAntiRROptimalLit";
pub const SELECT_NON_ANTI_RR_OPTIMAL_LIT: &str = "SelectNonAntiRROptimalLit";
pub const P_SELECT_NON_ANTI_RR_OPTIMAL_LIT: &str = "PSelectNonAntiRROptimalLit";
pub const SELECT_STRONG_RR_NON_RR_OPTIMAL_LIT: &str = "SelectStrongRRNonRROptimalLit";
pub const P_SELECT_STRONG_RR_NON_RR_OPTIMAL_LIT: &str = "PSelectStrongRRNonRROptimalLit";
pub const SELECT_COND_OPTIMAL_LIT: &str = "SelectCondOptimalLit";
pub const P_SELECT_COND_OPTIMAL_LIT: &str = "PSelectCondOptimalLit";
pub const SELECT_ALL_COND_OPTIMAL_LIT: &str = "SelectAllCondOptimalLit";
pub const P_SELECT_ALL_COND_OPTIMAL_LIT: &str = "PSelectAllCondOptimalLit";
pub const SELECT_OPTIMAL_RESTR_DEPTH2: &str = "SelectOptimalRestrDepth2";
pub const P_SELECT_OPTIMAL_RESTR_DEPTH2: &str = "PSelectOptimalRestrDepth2";
pub const SELECT_OPTIMAL_RESTR_P_DEPTH2: &str = "SelectOptimalRestrPDepth2";
pub const P_SELECT_OPTIMAL_RESTR_P_DEPTH2: &str = "PSelectOptimalRestrPDepth2";
pub const SELECT_OPTIMAL_RESTR_N_DEPTH2: &str = "SelectOptimalRestrNDepth2";
pub const P_SELECT_OPTIMAL_RESTR_N_DEPTH2: &str = "PSelectOptimalRestrNDepth2";
pub const SELECT_UNLESS_UNIQ_MAX: &str = "SelectUnlessUniqMax";
pub const P_SELECT_UNLESS_UNIQ_MAX: &str = "PSelectUnlessUniqMax";
pub const SELECT_UNLESS_POS_MAX: &str = "SelectUnlessPosMax";
pub const P_SELECT_UNLESS_POS_MAX: &str = "PSelectUnlessPosMax";
pub const SELECT_UNLESS_UNIQ_POS_MAX: &str = "SelectUnlessUniqPosMax";
pub const P_SELECT_UNLESS_UNIQ_POS_MAX: &str = "PSelectUnlessUniqPosMax";
pub const SELECT_UNLESS_UNIQ_MAX_POS: &str = "SelectUnlessUniqMaxPos";
pub const P_SELECT_UNLESS_UNIQ_MAX_POS: &str = "PSelectUnlessUniqMaxPos";
pub const SELECT_COMPLEX: &str = "SelectComplex";
pub const P_SELECT_COMPLEX: &str = "PSelectComplex";
pub const SELECT_COMPLEX_EXCEPT_RR_HORN: &str = "SelectComplexExceptRRHorn";
pub const P_SELECT_COMPLEX_EXCEPT_RR_HORN: &str = "PSelectComplexExceptRRHorn";
pub const SELECT_L_COMPLEX: &str = "SelectLComplex";
pub const P_SELECT_L_COMPLEX: &str = "PSelectLComplex";
pub const SELECT_COMPLEX_PREFER_NEQ: &str = "SelectComplexPreferNEQ";
pub const P_SELECT_COMPLEX_PREFER_NEQ: &str = "PSelectComplexPreferNEQ";
pub const SELECT_COMPLEX_PREFER_EQ: &str = "SelectComplexPreferEQ";
pub const P_SELECT_COMPLEX_PREFER_EQ: &str = "PSelectComplexPreferEQ";
pub const SELECT_DIV_LITS: &str = "SelectDivLits";
pub const SELECT_DIV_PREFER_INTO_LITS: &str = "SelectDivPreferIntoLits";

const VAR_FACTOR: i64 = 3;
static LITERAL_WEIGHT_COUNTER: AtomicI64 = AtomicI64::new(0);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum BasicLiteralSelector {
    NoSelection,
    NoGeneration,
    NegativeLiterals,
    PNegativeLiterals,
    PureVarNegativeLiterals,
    PPureVarNegativeLiterals,
    LargestNegativeLiteral,
    PLargestNegativeLiteral,
    SmallestNegativeLiteral,
    PSmallestNegativeLiteral,
    DiffNegativeLiteral,
    PDiffNegativeLiteral,
    GroundNegativeLiteral,
    PGroundNegativeLiteral,
}

impl BasicLiteralSelector {
    fn from_name(name: &str) -> Option<Self> {
        match name {
            NO_SELECTION => Some(Self::NoSelection),
            NO_GENERATION => Some(Self::NoGeneration),
            SELECT_NEGATIVE_LITERALS => Some(Self::NegativeLiterals),
            P_SELECT_NEGATIVE_LITERALS => Some(Self::PNegativeLiterals),
            SELECT_PURE_VAR_NEG_LITERALS => Some(Self::PureVarNegativeLiterals),
            P_SELECT_PURE_VAR_NEG_LITERALS => Some(Self::PPureVarNegativeLiterals),
            SELECT_LARGEST_NEG_LIT => Some(Self::LargestNegativeLiteral),
            P_SELECT_LARGEST_NEG_LIT => Some(Self::PLargestNegativeLiteral),
            SELECT_SMALLEST_NEG_LIT => Some(Self::SmallestNegativeLiteral),
            P_SELECT_SMALLEST_NEG_LIT => Some(Self::PSmallestNegativeLiteral),
            SELECT_DIFF_NEG_LIT => Some(Self::DiffNegativeLiteral),
            P_SELECT_DIFF_NEG_LIT => Some(Self::PDiffNegativeLiteral),
            SELECT_GROUND_NEG_LIT => Some(Self::GroundNegativeLiteral),
            P_SELECT_GROUND_NEG_LIT => Some(Self::PGroundNegativeLiteral),
            _ => None,
        }
    }

    fn apply(self, ocb: Option<&mut OrderControlBlock>, clause: &mut Clause) {
        match self {
            Self::NoSelection => select_no_literals(ocb, clause),
            Self::NoGeneration => select_no_generation(ocb, clause),
            Self::NegativeLiterals => select_negative_literals(ocb, clause),
            Self::PNegativeLiterals => p_select_negative_literals(ocb, clause),
            Self::PureVarNegativeLiterals => select_first_variable_literal(ocb, clause),
            Self::PPureVarNegativeLiterals => p_select_first_variable_literal(ocb, clause),
            Self::LargestNegativeLiteral => select_largest_negative_literal(ocb, clause),
            Self::PLargestNegativeLiteral => p_select_largest_negative_literal(ocb, clause),
            Self::SmallestNegativeLiteral => select_smallest_negative_literal(ocb, clause),
            Self::PSmallestNegativeLiteral => p_select_smallest_negative_literal(ocb, clause),
            Self::DiffNegativeLiteral => select_diff_negative_literal(ocb, clause),
            Self::PDiffNegativeLiteral => p_select_diff_negative_literal(ocb, clause),
            Self::GroundNegativeLiteral => select_ground_negative_literal(ocb, clause),
            Self::PGroundNegativeLiteral => p_select_ground_negative_literal(ocb, clause),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum OptimalLiteralSelector {
    Optimal,
    POptimal,
    MinOptimal,
    PMinOptimal,
    NonRROptimal,
    PNonRROptimal,
    NonStrongRROptimal,
    PNonStrongRROptimal,
    AntiRROptimal,
    PAntiRROptimal,
    NonAntiRROptimal,
    PNonAntiRROptimal,
    StrongRRNonRROptimal,
    PStrongRRNonRROptimal,
    CondOptimal,
    PCondOptimal,
    AllCondOptimal,
    PAllCondOptimal,
    RestrDepth2,
    PRestrDepth2,
    RestrPDepth2,
    PRestrPDepth2,
    RestrNDepth2,
    PRestrNDepth2,
}

impl OptimalLiteralSelector {
    fn from_name(name: &str) -> Option<Self> {
        match name {
            SELECT_OPTIMAL_LIT => Some(Self::Optimal),
            P_SELECT_OPTIMAL_LIT => Some(Self::POptimal),
            SELECT_MIN_OPTIMAL_LIT => Some(Self::MinOptimal),
            P_SELECT_MIN_OPTIMAL_LIT => Some(Self::PMinOptimal),
            SELECT_NON_RR_OPTIMAL_LIT => Some(Self::NonRROptimal),
            P_SELECT_NON_RR_OPTIMAL_LIT => Some(Self::PNonRROptimal),
            SELECT_NON_STRONG_RR_OPTIMAL_LIT => Some(Self::NonStrongRROptimal),
            P_SELECT_NON_STRONG_RR_OPTIMAL_LIT => Some(Self::PNonStrongRROptimal),
            SELECT_ANTI_RR_OPTIMAL_LIT => Some(Self::AntiRROptimal),
            P_SELECT_ANTI_RR_OPTIMAL_LIT => Some(Self::PAntiRROptimal),
            SELECT_NON_ANTI_RR_OPTIMAL_LIT => Some(Self::NonAntiRROptimal),
            P_SELECT_NON_ANTI_RR_OPTIMAL_LIT => Some(Self::PNonAntiRROptimal),
            SELECT_STRONG_RR_NON_RR_OPTIMAL_LIT => Some(Self::StrongRRNonRROptimal),
            P_SELECT_STRONG_RR_NON_RR_OPTIMAL_LIT => Some(Self::PStrongRRNonRROptimal),
            SELECT_COND_OPTIMAL_LIT => Some(Self::CondOptimal),
            P_SELECT_COND_OPTIMAL_LIT => Some(Self::PCondOptimal),
            SELECT_ALL_COND_OPTIMAL_LIT => Some(Self::AllCondOptimal),
            P_SELECT_ALL_COND_OPTIMAL_LIT => Some(Self::PAllCondOptimal),
            SELECT_OPTIMAL_RESTR_DEPTH2 => Some(Self::RestrDepth2),
            P_SELECT_OPTIMAL_RESTR_DEPTH2 => Some(Self::PRestrDepth2),
            SELECT_OPTIMAL_RESTR_P_DEPTH2 => Some(Self::RestrPDepth2),
            P_SELECT_OPTIMAL_RESTR_P_DEPTH2 => Some(Self::PRestrPDepth2),
            SELECT_OPTIMAL_RESTR_N_DEPTH2 => Some(Self::RestrNDepth2),
            P_SELECT_OPTIMAL_RESTR_N_DEPTH2 => Some(Self::PRestrNDepth2),
            _ => None,
        }
    }

    fn apply(self, ocb: Option<&mut OrderControlBlock>, clause: &mut Clause) {
        match self {
            Self::Optimal => select_optimal_literal(ocb, clause),
            Self::POptimal => p_select_optimal_literal(ocb, clause),
            Self::MinOptimal => select_min_optimal_literal(ocb, clause),
            Self::PMinOptimal => p_select_min_optimal_literal(ocb, clause),
            Self::NonRROptimal => select_non_rr_optimal_literal(ocb, clause),
            Self::PNonRROptimal => p_select_non_rr_optimal_literal(ocb, clause),
            Self::NonStrongRROptimal => select_non_strong_rr_optimal_literal(ocb, clause),
            Self::PNonStrongRROptimal => p_select_non_strong_rr_optimal_literal(ocb, clause),
            Self::AntiRROptimal => select_anti_rr_optimal_literal(ocb, clause),
            Self::PAntiRROptimal => p_select_anti_rr_optimal_literal(ocb, clause),
            Self::NonAntiRROptimal => select_non_anti_rr_optimal_literal(ocb, clause),
            Self::PNonAntiRROptimal => p_select_non_anti_rr_optimal_literal(ocb, clause),
            Self::StrongRRNonRROptimal => select_strong_rr_non_rr_optimal_literal(ocb, clause),
            Self::PStrongRRNonRROptimal => p_select_strong_rr_non_rr_optimal_literal(ocb, clause),
            Self::CondOptimal => select_cond_optimal_literal(ocb, clause),
            Self::PCondOptimal => p_select_cond_optimal_literal(ocb, clause),
            Self::AllCondOptimal => select_all_cond_optimal_literal(ocb, clause),
            Self::PAllCondOptimal => p_select_all_cond_optimal_literal(ocb, clause),
            Self::RestrDepth2 => select_depth2_optimal_literal(ocb, clause),
            Self::PRestrDepth2 => p_select_depth2_optimal_literal(ocb, clause),
            Self::RestrPDepth2 => select_p_depth2_optimal_literal(ocb, clause),
            Self::PRestrPDepth2 => p_select_p_depth2_optimal_literal(ocb, clause),
            Self::RestrNDepth2 => select_n_depth2_optimal_literal(ocb, clause),
            Self::PRestrNDepth2 => p_select_n_depth2_optimal_literal(ocb, clause),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Depth2Scope {
    All,
    Positive,
    Negative,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum OrientableLiteralSelector {
    Largest,
    PLargest,
    MLargest,
    Smallest,
    PSmallest,
    MSmallest,
}

impl OrientableLiteralSelector {
    fn from_name(name: &str) -> Option<Self> {
        match name {
            SELECT_LARGEST_ORIENTABLE => Some(Self::Largest),
            P_SELECT_LARGEST_ORIENTABLE => Some(Self::PLargest),
            M_SELECT_LARGEST_ORIENTABLE => Some(Self::MLargest),
            SELECT_SMALLEST_ORIENTABLE => Some(Self::Smallest),
            P_SELECT_SMALLEST_ORIENTABLE => Some(Self::PSmallest),
            M_SELECT_SMALLEST_ORIENTABLE => Some(Self::MSmallest),
            _ => None,
        }
    }

    fn apply(self, ocb: &mut OrderControlBlock, bank: &TermBank, clause: &mut Clause) {
        match self {
            Self::Largest => select_largest_orientable_literal(ocb, bank, clause),
            Self::PLargest => p_select_largest_orientable_literal(ocb, bank, clause),
            Self::MLargest => m_select_largest_orientable_literal(ocb, bank, clause),
            Self::Smallest => select_smallest_orientable_literal(ocb, bank, clause),
            Self::PSmallest => p_select_smallest_orientable_literal(ocb, bank, clause),
            Self::MSmallest => m_select_smallest_orientable_literal(ocb, bank, clause),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum MaximalGateSelector {
    UnlessUniqMax,
    PUnlessUniqMax,
    UnlessPosMax,
    PUnlessPosMax,
    UnlessUniqPosMax,
    PUnlessUniqPosMax,
    UnlessUniqMaxPos,
    PUnlessUniqMaxPos,
}

impl MaximalGateSelector {
    fn from_name(name: &str) -> Option<Self> {
        match name {
            SELECT_UNLESS_UNIQ_MAX => Some(Self::UnlessUniqMax),
            P_SELECT_UNLESS_UNIQ_MAX => Some(Self::PUnlessUniqMax),
            SELECT_UNLESS_POS_MAX => Some(Self::UnlessPosMax),
            P_SELECT_UNLESS_POS_MAX => Some(Self::PUnlessPosMax),
            SELECT_UNLESS_UNIQ_POS_MAX => Some(Self::UnlessUniqPosMax),
            P_SELECT_UNLESS_UNIQ_POS_MAX => Some(Self::PUnlessUniqPosMax),
            SELECT_UNLESS_UNIQ_MAX_POS => Some(Self::UnlessUniqMaxPos),
            P_SELECT_UNLESS_UNIQ_MAX_POS => Some(Self::PUnlessUniqMaxPos),
            _ => None,
        }
    }

    fn apply(self, ocb: &mut OrderControlBlock, bank: &TermBank, clause: &mut Clause) {
        match self {
            Self::UnlessUniqMax => select_unless_uniq_max_optimal_literal(ocb, bank, clause),
            Self::PUnlessUniqMax => p_select_unless_uniq_max_optimal_literal(ocb, bank, clause),
            Self::UnlessPosMax => select_unless_pos_max_optimal_literal(ocb, bank, clause),
            Self::PUnlessPosMax => p_select_unless_pos_max_optimal_literal(ocb, bank, clause),
            Self::UnlessUniqPosMax => {
                select_unless_uniq_pos_max_optimal_literal(ocb, bank, clause);
            }
            Self::PUnlessUniqPosMax => {
                p_select_unless_uniq_pos_max_optimal_literal(ocb, bank, clause);
            }
            Self::UnlessUniqMaxPos => {
                select_unless_uniq_max_pos_optimal_literal(ocb, bank, clause);
            }
            Self::PUnlessUniqMaxPos => {
                p_select_unless_uniq_max_pos_optimal_literal(ocb, bank, clause);
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum MaximalGate {
    MoreThanOne,
    NoPositive,
    NotUniquePositive,
    NotUniquePositiveOnly,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum OrientableWeightChoice {
    Largest,
    Smallest,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ComplexLiteralSelector {
    Complex,
    PComplex,
    ComplexExceptRRHorn,
    PComplexExceptRRHorn,
    LComplex,
    PLComplex,
    PreferNEQ,
    PPreferNEQ,
    PreferEQ,
    PPreferEQ,
}

impl ComplexLiteralSelector {
    fn from_name(name: &str) -> Option<Self> {
        match name {
            SELECT_COMPLEX => Some(Self::Complex),
            P_SELECT_COMPLEX => Some(Self::PComplex),
            SELECT_COMPLEX_EXCEPT_RR_HORN => Some(Self::ComplexExceptRRHorn),
            P_SELECT_COMPLEX_EXCEPT_RR_HORN => Some(Self::PComplexExceptRRHorn),
            SELECT_L_COMPLEX => Some(Self::LComplex),
            P_SELECT_L_COMPLEX => Some(Self::PLComplex),
            SELECT_COMPLEX_PREFER_NEQ => Some(Self::PreferNEQ),
            P_SELECT_COMPLEX_PREFER_NEQ => Some(Self::PPreferNEQ),
            SELECT_COMPLEX_PREFER_EQ => Some(Self::PreferEQ),
            P_SELECT_COMPLEX_PREFER_EQ => Some(Self::PPreferEQ),
            _ => None,
        }
    }

    fn apply(self, ocb: Option<&mut OrderControlBlock>, clause: &mut Clause) {
        match self {
            Self::Complex => select_complex(ocb, clause),
            Self::PComplex => p_select_complex(ocb, clause),
            Self::ComplexExceptRRHorn => select_complex_except_rr_horn(ocb, clause),
            Self::PComplexExceptRRHorn => p_select_complex_except_rr_horn(ocb, clause),
            Self::LComplex => select_l_complex(ocb, clause),
            Self::PLComplex => p_select_l_complex(ocb, clause),
            Self::PreferNEQ => select_complex_prefer_neq(ocb, clause),
            Self::PPreferNEQ => p_select_complex_prefer_neq(ocb, clause),
            Self::PreferEQ => select_complex_prefer_eq(ocb, clause),
            Self::PPreferEQ => p_select_complex_prefer_eq(ocb, clause),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ComplexGroundChoice {
    SmallestStandard,
    LargestDiff,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum EquationalPreference {
    Equation,
    NonEquation,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DiversificationLiteralSelector {
    Diversification,
    PreferInto,
}

impl DiversificationLiteralSelector {
    fn from_name(name: &str) -> Option<Self> {
        match name {
            SELECT_DIV_LITS => Some(Self::Diversification),
            SELECT_DIV_PREFER_INTO_LITS => Some(Self::PreferInto),
            _ => None,
        }
    }

    fn apply(self, ocb: Option<&mut OrderControlBlock>, clause: &mut Clause) {
        match self {
            Self::Diversification => select_diversification_literals(ocb, clause),
            Self::PreferInto => select_diversification_prefer_into_literals(ocb, clause),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct LitEval {
    is_positive: bool,
    forbidden: bool,
    w1: i64,
    w2: i64,
    w3: i64,
}

impl LitEval {
    const fn new(literal: &Eqn) -> Self {
        Self {
            is_positive: literal.is_positive(),
            forbidden: false,
            w1: 0,
            w2: 0,
            w3: 0,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnsupportedLiteralSelection {
    strategy: String,
}

impl UnsupportedLiteralSelection {
    #[must_use]
    pub fn new(strategy: impl Into<String>) -> Self {
        Self {
            strategy: strategy.into(),
        }
    }

    #[must_use]
    pub fn strategy(&self) -> &str {
        &self.strategy
    }
}

impl std::fmt::Display for UnsupportedLiteralSelection {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "literal selection strategy '{}' is not ported yet",
            self.strategy
        )
    }
}

/// C `SelectNoLiterals`: assert that no literal is selected and otherwise do
/// nothing.
///
/// # Panics
///
/// Panics in debug builds if the caller has not already cleared selected
/// literal properties.
pub fn select_no_literals(_ocb: Option<&mut OrderControlBlock>, clause: &mut Clause) {
    debug_assert_eq!(clause.prop_lit_number(EP_IS_SELECTED), 0);
}

/// C `SelectNoGeneration`: same no-op body as `SelectNoLiterals`.
///
/// # Panics
///
/// Panics in debug builds if the caller has not already cleared selected
/// literal properties.
pub fn select_no_generation(_ocb: Option<&mut OrderControlBlock>, clause: &mut Clause) {
    debug_assert_eq!(clause.prop_lit_number(EP_IS_SELECTED), 0);
}

pub fn select_negative_literals(_ocb: Option<&mut OrderControlBlock>, clause: &mut Clause) {
    for literal in clause.literals_mut().as_mut_slice() {
        if literal.is_negative() {
            literal.set_prop(EP_IS_SELECTED);
        }
    }
}

pub fn p_select_negative_literals(_ocb: Option<&mut OrderControlBlock>, clause: &mut Clause) {
    clause.literals_mut().set_prop(EP_IS_SELECTED);
}

pub fn select_first_variable_literal(_ocb: Option<&mut OrderControlBlock>, clause: &mut Clause) {
    if let Some(index) = clause.literals().find_neg_pure_var_lit_index() {
        clause.literals_mut().as_mut_slice()[index].set_prop(EP_IS_SELECTED);
    }
}

pub fn p_select_first_variable_literal(_ocb: Option<&mut OrderControlBlock>, clause: &mut Clause) {
    if clause.literals().find_neg_pure_var_lit_index().is_some() {
        select_positive_literals(clause);
    }
}

pub fn select_largest_negative_literal(_ocb: Option<&mut OrderControlBlock>, clause: &mut Clause) {
    let mut selected = None;
    let mut select_weight = 0;

    for (index, literal) in clause.literals().as_slice().iter().enumerate() {
        if literal.is_negative() && literal.standard_weight() > select_weight {
            select_weight = literal.standard_weight();
            selected = Some(index);
        }
    }

    debug_assert!(
        selected.is_some(),
        "literal-selection wrapper guarantees a negative literal"
    );
    if let Some(index) = selected {
        clause.literals_mut().as_mut_slice()[index].set_prop(EP_IS_SELECTED);
    }
}

pub fn p_select_largest_negative_literal(
    _ocb: Option<&mut OrderControlBlock>,
    clause: &mut Clause,
) {
    let mut selected = None;
    let mut select_weight = 0;

    for (index, literal) in clause.literals_mut().as_mut_slice().iter_mut().enumerate() {
        if literal.is_positive() {
            literal.set_prop(EP_IS_SELECTED);
        } else if literal.standard_weight() > select_weight {
            select_weight = literal.standard_weight();
            selected = Some(index);
        }
    }

    debug_assert!(
        selected.is_some(),
        "literal-selection wrapper guarantees a negative literal"
    );
    if let Some(index) = selected {
        clause.literals_mut().as_mut_slice()[index].set_prop(EP_IS_SELECTED);
    }
}

pub fn select_smallest_negative_literal(_ocb: Option<&mut OrderControlBlock>, clause: &mut Clause) {
    let mut selected = None;
    let mut select_weight = i64::MAX;

    for (index, literal) in clause.literals().as_slice().iter().enumerate() {
        if literal.is_negative() && literal.standard_weight() < select_weight {
            select_weight = literal.standard_weight();
            selected = Some(index);
        }
    }

    debug_assert!(
        selected.is_some(),
        "literal-selection wrapper guarantees a negative literal"
    );
    if let Some(index) = selected {
        clause.literals_mut().as_mut_slice()[index].set_prop(EP_IS_SELECTED);
    }
}

pub fn p_select_smallest_negative_literal(
    _ocb: Option<&mut OrderControlBlock>,
    clause: &mut Clause,
) {
    let mut selected = None;
    let mut select_weight = i64::MAX;

    for (index, literal) in clause.literals_mut().as_mut_slice().iter_mut().enumerate() {
        if literal.is_positive() {
            literal.set_prop(EP_IS_SELECTED);
        } else if literal.standard_weight() < select_weight {
            select_weight = literal.standard_weight();
            selected = Some(index);
        }
    }

    debug_assert!(
        selected.is_some(),
        "literal-selection wrapper guarantees a negative literal"
    );
    if let Some(index) = selected {
        clause.literals_mut().as_mut_slice()[index].set_prop(EP_IS_SELECTED);
    }
}

pub fn select_largest_orientable_literal(
    ocb: &mut OrderControlBlock,
    bank: &TermBank,
    clause: &mut Clause,
) {
    select_orientable_literal_impl(ocb, bank, clause, false, OrientableWeightChoice::Largest);
}

pub fn p_select_largest_orientable_literal(
    ocb: &mut OrderControlBlock,
    bank: &TermBank,
    clause: &mut Clause,
) {
    select_orientable_literal_impl(ocb, bank, clause, true, OrientableWeightChoice::Largest);
}

pub fn m_select_largest_orientable_literal(
    ocb: &mut OrderControlBlock,
    bank: &TermBank,
    clause: &mut Clause,
) {
    if clause.is_horn() {
        p_select_largest_orientable_literal(ocb, bank, clause);
    } else {
        select_largest_orientable_literal(ocb, bank, clause);
    }
}

pub fn select_smallest_orientable_literal(
    ocb: &mut OrderControlBlock,
    bank: &TermBank,
    clause: &mut Clause,
) {
    select_orientable_literal_impl(ocb, bank, clause, false, OrientableWeightChoice::Smallest);
}

pub fn p_select_smallest_orientable_literal(
    ocb: &mut OrderControlBlock,
    bank: &TermBank,
    clause: &mut Clause,
) {
    select_orientable_literal_impl(ocb, bank, clause, true, OrientableWeightChoice::Smallest);
}

pub fn m_select_smallest_orientable_literal(
    ocb: &mut OrderControlBlock,
    bank: &TermBank,
    clause: &mut Clause,
) {
    if clause.is_horn() {
        p_select_smallest_orientable_literal(ocb, bank, clause);
    } else {
        select_smallest_orientable_literal(ocb, bank, clause);
    }
}

pub fn select_diff_negative_literal(_ocb: Option<&mut OrderControlBlock>, clause: &mut Clause) {
    let selected = find_max_diff_negative_literal(clause, false);

    if let Some(index) = selected {
        clause.literals_mut().as_mut_slice()[index].set_prop(EP_IS_SELECTED);
    }
}

pub fn p_select_diff_negative_literal(_ocb: Option<&mut OrderControlBlock>, clause: &mut Clause) {
    let selected = find_max_diff_negative_literal(clause, false);
    select_positive_literals(clause);

    debug_assert!(
        selected.is_some(),
        "literal-selection wrapper guarantees a negative literal"
    );
    if let Some(index) = selected {
        clause.literals_mut().as_mut_slice()[index].set_prop(EP_IS_SELECTED);
    }
}

pub fn select_ground_negative_literal(_ocb: Option<&mut OrderControlBlock>, clause: &mut Clause) {
    let selected = find_max_diff_negative_literal(clause, true);

    if let Some(index) = selected {
        clause.literals_mut().as_mut_slice()[index].set_prop(EP_IS_SELECTED);
    }
}

pub fn p_select_ground_negative_literal(_ocb: Option<&mut OrderControlBlock>, clause: &mut Clause) {
    let selected = find_max_diff_negative_literal(clause, true);

    if let Some(index) = selected {
        select_positive_literals(clause);
        clause.literals_mut().as_mut_slice()[index].set_prop(EP_IS_SELECTED);
    } else {
        clause.literals_mut().del_prop(EP_IS_SELECTED);
    }
}

pub fn select_optimal_literal(ocb: Option<&mut OrderControlBlock>, clause: &mut Clause) {
    if let Some(index) = find_max_diff_negative_literal(clause, true) {
        clause.literals_mut().as_mut_slice()[index].set_prop(EP_IS_SELECTED);
    } else {
        select_diff_negative_literal(ocb, clause);
    }
}

pub fn p_select_optimal_literal(ocb: Option<&mut OrderControlBlock>, clause: &mut Clause) {
    if let Some(index) = find_max_diff_negative_literal(clause, true) {
        clause.literals_mut().as_mut_slice()[index].set_prop(EP_IS_SELECTED);
    } else {
        p_select_diff_negative_literal(ocb, clause);
    }
}

pub fn select_min_optimal_literal(ocb: Option<&mut OrderControlBlock>, clause: &mut Clause) {
    if let Some(index) = find_min_weight_negative_literal(clause, true) {
        clause.literals_mut().as_mut_slice()[index].set_prop(EP_IS_SELECTED);
    } else {
        select_smallest_negative_literal(ocb, clause);
    }
}

pub fn p_select_min_optimal_literal(ocb: Option<&mut OrderControlBlock>, clause: &mut Clause) {
    if let Some(index) = find_min_weight_negative_literal(clause, true) {
        clause.literals_mut().as_mut_slice()[index].set_prop(EP_IS_SELECTED);
    } else {
        p_select_smallest_negative_literal(ocb, clause);
    }
}

pub fn select_non_rr_optimal_literal(ocb: Option<&mut OrderControlBlock>, clause: &mut Clause) {
    if !clause.is_range_restricted() {
        select_optimal_literal(ocb, clause);
    }
}

pub fn p_select_non_rr_optimal_literal(ocb: Option<&mut OrderControlBlock>, clause: &mut Clause) {
    if !clause.is_range_restricted() {
        p_select_optimal_literal(ocb, clause);
    }
}

pub fn select_non_strong_rr_optimal_literal(
    ocb: Option<&mut OrderControlBlock>,
    clause: &mut Clause,
) {
    if !clause.is_strongly_range_restricted() {
        select_optimal_literal(ocb, clause);
    }
}

pub fn p_select_non_strong_rr_optimal_literal(
    ocb: Option<&mut OrderControlBlock>,
    clause: &mut Clause,
) {
    if !clause.is_strongly_range_restricted() {
        p_select_optimal_literal(ocb, clause);
    }
}

pub fn select_anti_rr_optimal_literal(ocb: Option<&mut OrderControlBlock>, clause: &mut Clause) {
    debug_assert_eq!(clause.prop_lit_number(EP_IS_SELECTED), 0);
    if clause.negative_literal_count() == 0 {
        return;
    }
    if clause.is_anti_range_restricted() {
        select_optimal_literal(ocb, clause);
    }
}

pub fn p_select_anti_rr_optimal_literal(ocb: Option<&mut OrderControlBlock>, clause: &mut Clause) {
    if clause.is_anti_range_restricted() {
        p_select_optimal_literal(ocb, clause);
    }
}

pub fn select_non_anti_rr_optimal_literal(
    ocb: Option<&mut OrderControlBlock>,
    clause: &mut Clause,
) {
    if !clause.is_anti_range_restricted() {
        select_optimal_literal(ocb, clause);
    }
}

pub fn p_select_non_anti_rr_optimal_literal(
    ocb: Option<&mut OrderControlBlock>,
    clause: &mut Clause,
) {
    if !clause.is_anti_range_restricted() {
        p_select_optimal_literal(ocb, clause);
    }
}

pub fn select_strong_rr_non_rr_optimal_literal(
    ocb: Option<&mut OrderControlBlock>,
    clause: &mut Clause,
) {
    if !clause.is_range_restricted() || clause.is_strongly_range_restricted() {
        select_optimal_literal(ocb, clause);
    }
}

pub fn p_select_strong_rr_non_rr_optimal_literal(
    ocb: Option<&mut OrderControlBlock>,
    clause: &mut Clause,
) {
    if !clause.is_range_restricted() || clause.is_strongly_range_restricted() {
        p_select_optimal_literal(ocb, clause);
    } else {
        clause.literals_mut().del_prop(EP_IS_SELECTED);
    }
}

pub fn select_cond_optimal_literal(ocb: Option<&mut OrderControlBlock>, clause: &mut Clause) {
    select_cond_optimal_literal_impl(ocb, clause, false, false);
}

pub fn p_select_cond_optimal_literal(ocb: Option<&mut OrderControlBlock>, clause: &mut Clause) {
    select_cond_optimal_literal_impl(ocb, clause, true, false);
}

pub fn select_all_cond_optimal_literal(ocb: Option<&mut OrderControlBlock>, clause: &mut Clause) {
    select_cond_optimal_literal_impl(ocb, clause, false, true);
}

pub fn p_select_all_cond_optimal_literal(ocb: Option<&mut OrderControlBlock>, clause: &mut Clause) {
    select_cond_optimal_literal_impl(ocb, clause, true, true);
}

pub fn select_depth2_optimal_literal(ocb: Option<&mut OrderControlBlock>, clause: &mut Clause) {
    select_depth2_optimal_literal_impl(ocb, clause, false, Depth2Scope::All);
}

pub fn p_select_depth2_optimal_literal(ocb: Option<&mut OrderControlBlock>, clause: &mut Clause) {
    select_depth2_optimal_literal_impl(ocb, clause, true, Depth2Scope::All);
}

pub fn select_p_depth2_optimal_literal(ocb: Option<&mut OrderControlBlock>, clause: &mut Clause) {
    select_depth2_optimal_literal_impl(ocb, clause, false, Depth2Scope::Positive);
}

pub fn p_select_p_depth2_optimal_literal(ocb: Option<&mut OrderControlBlock>, clause: &mut Clause) {
    select_depth2_optimal_literal_impl(ocb, clause, true, Depth2Scope::Positive);
}

pub fn select_n_depth2_optimal_literal(ocb: Option<&mut OrderControlBlock>, clause: &mut Clause) {
    select_depth2_optimal_literal_impl(ocb, clause, false, Depth2Scope::Negative);
}

pub fn p_select_n_depth2_optimal_literal(ocb: Option<&mut OrderControlBlock>, clause: &mut Clause) {
    select_depth2_optimal_literal_impl(ocb, clause, true, Depth2Scope::Negative);
}

pub fn select_unless_uniq_max_optimal_literal(
    ocb: &mut OrderControlBlock,
    bank: &TermBank,
    clause: &mut Clause,
) {
    select_unless_maximal_gate_optimal_literal(ocb, bank, clause, false, MaximalGate::MoreThanOne);
}

pub fn p_select_unless_uniq_max_optimal_literal(
    ocb: &mut OrderControlBlock,
    bank: &TermBank,
    clause: &mut Clause,
) {
    select_unless_maximal_gate_optimal_literal(ocb, bank, clause, true, MaximalGate::MoreThanOne);
}

pub fn select_unless_pos_max_optimal_literal(
    ocb: &mut OrderControlBlock,
    bank: &TermBank,
    clause: &mut Clause,
) {
    select_unless_maximal_gate_optimal_literal(ocb, bank, clause, false, MaximalGate::NoPositive);
}

pub fn p_select_unless_pos_max_optimal_literal(
    ocb: &mut OrderControlBlock,
    bank: &TermBank,
    clause: &mut Clause,
) {
    select_unless_maximal_gate_optimal_literal(ocb, bank, clause, true, MaximalGate::NoPositive);
}

pub fn select_unless_uniq_pos_max_optimal_literal(
    ocb: &mut OrderControlBlock,
    bank: &TermBank,
    clause: &mut Clause,
) {
    select_unless_maximal_gate_optimal_literal(
        ocb,
        bank,
        clause,
        false,
        MaximalGate::NotUniquePositive,
    );
}

pub fn p_select_unless_uniq_pos_max_optimal_literal(
    ocb: &mut OrderControlBlock,
    bank: &TermBank,
    clause: &mut Clause,
) {
    select_unless_maximal_gate_optimal_literal(
        ocb,
        bank,
        clause,
        true,
        MaximalGate::NotUniquePositive,
    );
}

pub fn select_unless_uniq_max_pos_optimal_literal(
    ocb: &mut OrderControlBlock,
    bank: &TermBank,
    clause: &mut Clause,
) {
    select_unless_maximal_gate_optimal_literal(
        ocb,
        bank,
        clause,
        false,
        MaximalGate::NotUniquePositiveOnly,
    );
}

pub fn p_select_unless_uniq_max_pos_optimal_literal(
    ocb: &mut OrderControlBlock,
    bank: &TermBank,
    clause: &mut Clause,
) {
    select_unless_maximal_gate_optimal_literal(
        ocb,
        bank,
        clause,
        true,
        MaximalGate::NotUniquePositiveOnly,
    );
}

pub fn select_complex(ocb: Option<&mut OrderControlBlock>, clause: &mut Clause) {
    select_complex_impl(ocb, clause, false, ComplexGroundChoice::SmallestStandard);
}

pub fn p_select_complex(ocb: Option<&mut OrderControlBlock>, clause: &mut Clause) {
    select_complex_impl(ocb, clause, true, ComplexGroundChoice::SmallestStandard);
}

pub fn select_complex_except_rr_horn(ocb: Option<&mut OrderControlBlock>, clause: &mut Clause) {
    if !(clause.is_horn() && clause.is_range_restricted()) {
        select_complex(ocb, clause);
    }
}

pub fn p_select_complex_except_rr_horn(ocb: Option<&mut OrderControlBlock>, clause: &mut Clause) {
    if !(clause.is_horn() && clause.is_range_restricted()) {
        p_select_complex(ocb, clause);
    }
}

pub fn select_l_complex(ocb: Option<&mut OrderControlBlock>, clause: &mut Clause) {
    select_complex_impl(ocb, clause, false, ComplexGroundChoice::LargestDiff);
}

pub fn p_select_l_complex(ocb: Option<&mut OrderControlBlock>, clause: &mut Clause) {
    select_complex_impl(ocb, clause, true, ComplexGroundChoice::LargestDiff);
}

pub fn select_complex_prefer_neq(_ocb: Option<&mut OrderControlBlock>, clause: &mut Clause) {
    select_complex_prefer_impl(clause, false, EquationalPreference::NonEquation);
}

pub fn p_select_complex_prefer_neq(_ocb: Option<&mut OrderControlBlock>, clause: &mut Clause) {
    select_complex_prefer_impl(clause, true, EquationalPreference::NonEquation);
}

pub fn select_complex_prefer_eq(_ocb: Option<&mut OrderControlBlock>, clause: &mut Clause) {
    select_complex_prefer_impl(clause, false, EquationalPreference::Equation);
}

pub fn p_select_complex_prefer_eq(_ocb: Option<&mut OrderControlBlock>, clause: &mut Clause) {
    select_complex_prefer_impl(clause, true, EquationalPreference::Equation);
}

pub fn select_diversification_literals(_ocb: Option<&mut OrderControlBlock>, clause: &mut Clause) {
    generic_uniq_selection_no_ordering(clause, false, diversification_weight);
}

pub fn select_diversification_prefer_into_literals(
    _ocb: Option<&mut OrderControlBlock>,
    clause: &mut Clause,
) {
    generic_uniq_selection_no_ordering(clause, false, diversification_prefer_into_weight);
}

/// Applies the subset of literal-selection functions that has been ported.
///
/// # Errors
///
/// Returns `UnsupportedLiteralSelection` for valid C selector names whose
/// selector bodies have not been ported yet.
pub fn apply_ported_literal_selector(
    name: &str,
    ocb: Option<&mut OrderControlBlock>,
    clause: &mut Clause,
) -> Result<(), UnsupportedLiteralSelection> {
    apply_ported_literal_selector_with_bank(name, ocb, None, clause)
}

/// Applies the subset of literal-selection functions that has been ported,
/// including selector bodies that need the term bank for C maximality marking.
///
/// # Errors
///
/// Returns `UnsupportedLiteralSelection` for valid C selector names whose
/// selector bodies have not been ported yet, and for bank-aware selectors when
/// the required `OCB` or term bank is not supplied.
pub fn apply_ported_literal_selector_with_bank(
    name: &str,
    ocb: Option<&mut OrderControlBlock>,
    bank: Option<&TermBank>,
    clause: &mut Clause,
) -> Result<(), UnsupportedLiteralSelection> {
    if let Some(selector) = BasicLiteralSelector::from_name(name) {
        selector.apply(ocb, clause);
        Ok(())
    } else if let Some(selector) = OptimalLiteralSelector::from_name(name) {
        selector.apply(ocb, clause);
        Ok(())
    } else if let Some(selector) = ComplexLiteralSelector::from_name(name) {
        selector.apply(ocb, clause);
        Ok(())
    } else if let Some(selector) = DiversificationLiteralSelector::from_name(name) {
        selector.apply(ocb, clause);
        Ok(())
    } else if let Some(selector) = OrientableLiteralSelector::from_name(name) {
        let Some(ocb) = ocb else {
            return Err(UnsupportedLiteralSelection::new(name));
        };
        let Some(bank) = bank else {
            return Err(UnsupportedLiteralSelection::new(name));
        };
        selector.apply(ocb, bank, clause);
        Ok(())
    } else if let Some(selector) = MaximalGateSelector::from_name(name) {
        let Some(ocb) = ocb else {
            return Err(UnsupportedLiteralSelection::new(name));
        };
        let Some(bank) = bank else {
            return Err(UnsupportedLiteralSelection::new(name));
        };
        selector.apply(ocb, bank, clause);
        Ok(())
    } else {
        Err(UnsupportedLiteralSelection::new(name))
    }
}

fn select_positive_literals(clause: &mut Clause) {
    debug_assert_ne!(clause.negative_literal_count(), 0);
    for literal in clause.literals_mut().as_mut_slice() {
        if literal.is_positive() {
            literal.set_prop(EP_IS_SELECTED);
        }
    }
}

fn select_orientable_literal_impl(
    ocb: &mut OrderControlBlock,
    bank: &TermBank,
    clause: &mut Clause,
    positive_variant: bool,
    weight_choice: OrientableWeightChoice,
) {
    clause.cond_mark_maximal_terms(ocb, bank);

    let selected = find_orientable_negative_literal(clause, weight_choice);
    debug_assert!(
        selected.is_some(),
        "literal-selection wrapper guarantees a negative literal"
    );
    if positive_variant {
        select_positive_literals(clause);
    }
    if let Some(index) = selected {
        clause.literals_mut().as_mut_slice()[index].set_prop(EP_IS_SELECTED);
        clause.del_prop(CP_IS_ORIENTED);
    }
}

fn find_orientable_negative_literal(
    clause: &Clause,
    weight_choice: OrientableWeightChoice,
) -> Option<usize> {
    let mut selected = None;
    let mut selected_oriented = false;
    let mut selected_weight = match weight_choice {
        OrientableWeightChoice::Largest => 0,
        OrientableWeightChoice::Smallest => i64::MAX,
    };

    for (index, literal) in clause.literals().as_slice().iter().enumerate() {
        if !literal.is_negative() {
            continue;
        }
        let literal_oriented = literal.is_oriented();
        let same_orientation_class = selected_oriented == literal_oriented;
        let better_weight = match weight_choice {
            OrientableWeightChoice::Largest => literal.standard_weight() > selected_weight,
            OrientableWeightChoice::Smallest => literal.standard_weight() < selected_weight,
        };
        if (!selected_oriented && literal_oriented) || (same_orientation_class && better_weight) {
            selected = Some(index);
            selected_oriented = literal_oriented;
            selected_weight = literal.standard_weight();
        }
    }

    selected
}

fn select_unless_maximal_gate_optimal_literal(
    ocb: &mut OrderControlBlock,
    bank: &TermBank,
    clause: &mut Clause,
    positive_variant: bool,
    gate: MaximalGate,
) {
    clause.cond_mark_maximal_terms(ocb, bank);

    if maximal_gate_allows_selection(clause, gate) {
        apply_optimal_variant(Some(ocb), clause, positive_variant);
        clause.del_prop(CP_IS_ORIENTED);
    }
}

fn maximal_gate_allows_selection(clause: &Clause, gate: MaximalGate) -> bool {
    match gate {
        MaximalGate::MoreThanOne => clause.literals().query_prop_number(EP_IS_MAXIMAL) > 1,
        MaximalGate::NoPositive => {
            clause
                .literals()
                .query_prop_number(EP_IS_MAXIMAL | EP_IS_POSITIVE)
                == 0
        }
        MaximalGate::NotUniquePositive => {
            clause
                .literals()
                .query_prop_number(EP_IS_MAXIMAL | EP_IS_POSITIVE)
                != 1
        }
        MaximalGate::NotUniquePositiveOnly => !has_unique_positive_only_maximal_literal(clause),
    }
}

fn has_unique_positive_only_maximal_literal(clause: &Clause) -> bool {
    let mut found_positive_maximal = false;
    for literal in clause.literals().as_slice() {
        if literal.is_maximal() {
            if literal.is_negative() || found_positive_maximal {
                return false;
            }
            found_positive_maximal = true;
        }
    }
    found_positive_maximal
}

fn generic_uniq_selection_no_ordering(
    clause: &mut Clause,
    positive: bool,
    weight_fun: fn(&mut LitEval, &Eqn, &Clause),
) {
    debug_assert_ne!(clause.negative_literal_count(), 0);
    debug_assert_eq!(clause.prop_lit_number(EP_IS_SELECTED), 0);

    let mut evals = clause
        .literals()
        .as_slice()
        .iter()
        .map(LitEval::new)
        .collect::<Vec<_>>();

    for (eval, literal) in evals.iter_mut().zip(clause.literals().as_slice()) {
        weight_fun(eval, literal, clause);
    }

    let mut selected_index = 0;
    for (index, eval) in evals.iter().enumerate().skip(1) {
        if lit_eval_compare(eval, &evals[selected_index]).is_lt() {
            selected_index = index;
        }
    }

    debug_assert!(
        !evals[selected_index].is_positive,
        "generic literal selection candidate must be negative"
    );

    if !evals[selected_index].forbidden {
        clause.literals_mut().as_mut_slice()[selected_index].set_prop(EP_IS_SELECTED);
        clause.del_prop(CP_IS_ORIENTED);
        if positive {
            select_positive_literals(clause);
        }
    }
}

fn lit_eval_compare(left: &LitEval, right: &LitEval) -> std::cmp::Ordering {
    left.is_positive
        .cmp(&right.is_positive)
        .then_with(|| left.w1.cmp(&right.w1))
        .then_with(|| left.w2.cmp(&right.w2))
        .then_with(|| left.w3.cmp(&right.w3))
}

fn diversification_weight(eval: &mut LitEval, literal: &Eqn, clause: &Clause) {
    let counter = next_literal_weight_counter();
    if literal.is_negative() {
        eval.w1 = counter % negative_literal_count_i64(clause);
    }
}

fn diversification_prefer_into_weight(eval: &mut LitEval, literal: &Eqn, clause: &Clause) {
    let counter = next_literal_weight_counter();
    eval.w1 = if literal.query_prop(EP_IS_PM_INTO_LIT) {
        -1
    } else {
        0
    };
    if literal.is_negative() {
        eval.w2 = counter % negative_literal_count_i64(clause);
    }
}

fn next_literal_weight_counter() -> i64 {
    LITERAL_WEIGHT_COUNTER.fetch_add(1, AtomicOrdering::Relaxed)
}

fn negative_literal_count_i64(clause: &Clause) -> i64 {
    i64::try_from(clause.negative_literal_count()).expect("negative literal count fits in i64")
}

#[cfg(test)]
fn reset_literal_weight_counter_for_tests() {
    LITERAL_WEIGHT_COUNTER.store(0, AtomicOrdering::Relaxed);
}

fn select_cond_optimal_literal_impl(
    ocb: Option<&mut OrderControlBlock>,
    clause: &mut Clause,
    positive_variant: bool,
    all_positive: bool,
) {
    if conditional_gate_blocks_selection(clause, all_positive) {
        clause.literals_mut().del_prop(EP_IS_SELECTED);
    } else {
        apply_optimal_variant(ocb, clause, positive_variant);
    }
}

fn conditional_gate_blocks_selection(clause: &Clause, all_positive: bool) -> bool {
    if all_positive {
        !clause
            .literals()
            .as_slice()
            .iter()
            .any(|literal| literal.is_positive() && !positive_conditional_literal_blocks(literal))
    } else {
        clause
            .literals()
            .as_slice()
            .iter()
            .any(|literal| literal.is_positive() && positive_conditional_literal_blocks(literal))
    }
}

fn positive_conditional_literal_blocks(literal: &Eqn) -> bool {
    let mut weight = term_weight_compute(literal.left(), 0, VAR_FACTOR);
    let mut standard_weight = term_standard_weight(literal.left());
    if literal.query_prop(EP_IS_EQU_LITERAL) {
        weight += term_weight_compute(literal.right(), 0, VAR_FACTOR);
        standard_weight += term_standard_weight(literal.right());
    }
    standard_weight <= weight
}

fn select_depth2_optimal_literal_impl(
    ocb: Option<&mut OrderControlBlock>,
    clause: &mut Clause,
    positive_variant: bool,
    scope: Depth2Scope,
) {
    if has_depth_at_most(clause, scope, 2) {
        clause.literals_mut().del_prop(EP_IS_SELECTED);
    } else {
        apply_optimal_variant(ocb, clause, positive_variant);
    }
}

fn has_depth_at_most(clause: &Clause, scope: Depth2Scope, max_depth: i64) -> bool {
    clause.literals().as_slice().iter().any(|literal| {
        let scope_matches = match scope {
            Depth2Scope::All => true,
            Depth2Scope::Positive => literal.is_positive(),
            Depth2Scope::Negative => literal.is_negative(),
        };
        scope_matches && literal.depth() <= max_depth
    })
}

fn apply_optimal_variant(
    ocb: Option<&mut OrderControlBlock>,
    clause: &mut Clause,
    positive_variant: bool,
) {
    if positive_variant {
        p_select_optimal_literal(ocb, clause);
    } else {
        select_optimal_literal(ocb, clause);
    }
}

fn select_complex_impl(
    ocb: Option<&mut OrderControlBlock>,
    clause: &mut Clause,
    positive_variant: bool,
    ground_choice: ComplexGroundChoice,
) {
    let selected = clause
        .literals()
        .find_neg_pure_var_lit_index()
        .or_else(|| find_ground_negative_for_complex(clause, ground_choice));

    if let Some(index) = selected {
        clause.literals_mut().as_mut_slice()[index].set_prop(EP_IS_SELECTED);
    } else if positive_variant {
        p_select_diff_negative_literal(ocb, clause);
    } else {
        select_diff_negative_literal(ocb, clause);
    }
}

fn find_ground_negative_for_complex(
    clause: &Clause,
    ground_choice: ComplexGroundChoice,
) -> Option<usize> {
    let mut selected = None;
    let mut selected_weight = match ground_choice {
        ComplexGroundChoice::SmallestStandard => i64::MAX,
        ComplexGroundChoice::LargestDiff => -1,
    };

    for (index, literal) in clause.literals().as_slice().iter().enumerate() {
        if literal.is_negative() && literal.is_ground() {
            let current_weight = match ground_choice {
                ComplexGroundChoice::SmallestStandard => literal.standard_weight(),
                ComplexGroundChoice::LargestDiff => literal_selection_diff_weight(literal),
            };
            let is_better = match ground_choice {
                ComplexGroundChoice::SmallestStandard => current_weight < selected_weight,
                ComplexGroundChoice::LargestDiff => current_weight > selected_weight,
            };
            if is_better {
                selected_weight = current_weight;
                selected = Some(index);
            }
        }
    }

    selected
}

fn select_complex_prefer_impl(
    clause: &mut Clause,
    positive_variant: bool,
    preference: EquationalPreference,
) {
    if let Some(index) = find_preferred_complex_negative_literal(clause, preference) {
        if positive_variant {
            select_positive_literals(clause);
        }
        clause.literals_mut().as_mut_slice()[index].set_prop(EP_IS_SELECTED);
    }
}

fn find_preferred_complex_negative_literal(
    clause: &Clause,
    preference: EquationalPreference,
) -> Option<usize> {
    let mut selected = None;
    let mut selected_preferred = false;
    let mut selected_var = false;
    let mut selected_ground = false;
    let mut selected_weight = -1;

    for (index, literal) in clause.literals().as_slice().iter().enumerate() {
        if literal.is_negative() {
            let current_preferred = match preference {
                EquationalPreference::Equation => literal.query_prop(EP_IS_EQU_LITERAL),
                EquationalPreference::NonEquation => !literal.query_prop(EP_IS_EQU_LITERAL),
            };
            let current_var = literal.is_pure_var();
            let current_ground = literal.is_ground();
            let mut current_weight = -1;

            if selected_preferred && !current_preferred {
                break;
            }
            if current_preferred == selected_preferred {
                if selected_var && !current_var {
                    break;
                }
                if current_var == selected_var {
                    if selected_ground && !current_ground {
                        break;
                    }
                    if current_ground == selected_ground {
                        current_weight = literal_selection_diff_weight(literal);
                        if current_weight <= selected_weight {
                            break;
                        }
                    }
                }
            }

            if current_weight == -1 {
                current_weight = literal_selection_diff_weight(literal);
            }
            selected = Some(index);
            selected_weight = current_weight;
            selected_ground = current_ground;
            selected_var = current_var;
            selected_preferred = current_preferred;
        }
    }

    selected
}

fn find_max_diff_negative_literal(clause: &Clause, ground_only: bool) -> Option<usize> {
    let mut selected = None;
    let mut select_weight = -1;

    for (index, literal) in clause.literals().as_slice().iter().enumerate() {
        if literal.is_negative() && (!ground_only || literal.is_ground()) {
            let weight = literal_selection_diff_weight(literal);
            if weight > select_weight {
                select_weight = weight;
                selected = Some(index);
            }
        }
    }

    selected
}

fn literal_selection_diff_weight(literal: &crate::clauses::eqn::Eqn) -> i64 {
    100 * literal.standard_diff() + literal.standard_weight()
}

fn find_min_weight_negative_literal(clause: &Clause, ground_only: bool) -> Option<usize> {
    let mut selected = None;
    let mut select_weight = i64::MAX;

    for (index, literal) in clause.literals().as_slice().iter().enumerate() {
        if literal.is_negative() && (!ground_only || literal.is_ground()) {
            let weight = literal.standard_weight();
            if weight < select_weight {
                select_weight = weight;
                selected = Some(index);
            }
        }
    }

    selected
}

#[cfg(test)]
mod tests {
    use super::{
        apply_ported_literal_selector, apply_ported_literal_selector_with_bank,
        m_select_largest_orientable_literal, m_select_smallest_orientable_literal,
        p_select_all_cond_optimal_literal, p_select_complex, p_select_complex_except_rr_horn,
        p_select_complex_prefer_eq, p_select_complex_prefer_neq, p_select_cond_optimal_literal,
        p_select_depth2_optimal_literal, p_select_diff_negative_literal,
        p_select_first_variable_literal, p_select_ground_negative_literal, p_select_l_complex,
        p_select_largest_negative_literal, p_select_largest_orientable_literal,
        p_select_min_optimal_literal, p_select_negative_literals, p_select_optimal_literal,
        p_select_smallest_negative_literal, p_select_smallest_orientable_literal,
        p_select_strong_rr_non_rr_optimal_literal, p_select_unless_uniq_max_optimal_literal,
        reset_literal_weight_counter_for_tests, select_all_cond_optimal_literal,
        select_anti_rr_optimal_literal, select_complex, select_complex_except_rr_horn,
        select_complex_prefer_eq, select_complex_prefer_neq, select_cond_optimal_literal,
        select_depth2_optimal_literal, select_diff_negative_literal,
        select_diversification_literals, select_diversification_prefer_into_literals,
        select_first_variable_literal, select_ground_negative_literal, select_l_complex,
        select_largest_negative_literal, select_largest_orientable_literal,
        select_min_optimal_literal, select_n_depth2_optimal_literal, select_negative_literals,
        select_non_anti_rr_optimal_literal, select_non_rr_optimal_literal,
        select_non_strong_rr_optimal_literal, select_optimal_literal,
        select_p_depth2_optimal_literal, select_smallest_negative_literal,
        select_smallest_orientable_literal, select_strong_rr_non_rr_optimal_literal,
        select_unless_pos_max_optimal_literal, select_unless_uniq_max_optimal_literal,
        select_unless_uniq_max_pos_optimal_literal, select_unless_uniq_pos_max_optimal_literal,
        M_SELECT_LARGEST_ORIENTABLE, M_SELECT_SMALLEST_ORIENTABLE, NO_GENERATION, NO_SELECTION,
        P_SELECT_ALL_COND_OPTIMAL_LIT, P_SELECT_ANTI_RR_OPTIMAL_LIT, P_SELECT_COMPLEX,
        P_SELECT_COMPLEX_EXCEPT_RR_HORN, P_SELECT_COMPLEX_PREFER_EQ, P_SELECT_COMPLEX_PREFER_NEQ,
        P_SELECT_COND_OPTIMAL_LIT, P_SELECT_DIFF_NEG_LIT, P_SELECT_GROUND_NEG_LIT,
        P_SELECT_LARGEST_NEG_LIT, P_SELECT_LARGEST_ORIENTABLE, P_SELECT_L_COMPLEX,
        P_SELECT_MIN_OPTIMAL_LIT, P_SELECT_NEGATIVE_LITERALS, P_SELECT_NON_ANTI_RR_OPTIMAL_LIT,
        P_SELECT_NON_RR_OPTIMAL_LIT, P_SELECT_NON_STRONG_RR_OPTIMAL_LIT, P_SELECT_OPTIMAL_LIT,
        P_SELECT_OPTIMAL_RESTR_DEPTH2, P_SELECT_OPTIMAL_RESTR_N_DEPTH2,
        P_SELECT_OPTIMAL_RESTR_P_DEPTH2, P_SELECT_PURE_VAR_NEG_LITERALS, P_SELECT_SMALLEST_NEG_LIT,
        P_SELECT_SMALLEST_ORIENTABLE, P_SELECT_STRONG_RR_NON_RR_OPTIMAL_LIT,
        P_SELECT_UNLESS_POS_MAX, P_SELECT_UNLESS_UNIQ_MAX, P_SELECT_UNLESS_UNIQ_MAX_POS,
        P_SELECT_UNLESS_UNIQ_POS_MAX, SELECT_ALL_COND_OPTIMAL_LIT, SELECT_ANTI_RR_OPTIMAL_LIT,
        SELECT_COMPLEX, SELECT_COMPLEX_EXCEPT_RR_HORN, SELECT_COMPLEX_PREFER_EQ,
        SELECT_COMPLEX_PREFER_NEQ, SELECT_COND_OPTIMAL_LIT, SELECT_DIFF_NEG_LIT, SELECT_DIV_LITS,
        SELECT_DIV_PREFER_INTO_LITS, SELECT_GROUND_NEG_LIT, SELECT_LARGEST_NEG_LIT,
        SELECT_LARGEST_ORIENTABLE, SELECT_L_COMPLEX, SELECT_MIN_OPTIMAL_LIT,
        SELECT_NEGATIVE_LITERALS, SELECT_NON_ANTI_RR_OPTIMAL_LIT, SELECT_NON_RR_OPTIMAL_LIT,
        SELECT_NON_STRONG_RR_OPTIMAL_LIT, SELECT_OPTIMAL_LIT, SELECT_OPTIMAL_RESTR_DEPTH2,
        SELECT_OPTIMAL_RESTR_N_DEPTH2, SELECT_OPTIMAL_RESTR_P_DEPTH2, SELECT_PURE_VAR_NEG_LITERALS,
        SELECT_SMALLEST_NEG_LIT, SELECT_SMALLEST_ORIENTABLE, SELECT_STRONG_RR_NON_RR_OPTIMAL_LIT,
        SELECT_UNLESS_POS_MAX, SELECT_UNLESS_UNIQ_MAX, SELECT_UNLESS_UNIQ_MAX_POS,
        SELECT_UNLESS_UNIQ_POS_MAX,
    };
    use crate::basics::partial_orderings::HoOrderKind;
    use crate::clauses::clause::Clause;
    use crate::clauses::clause_props::CP_IS_ORIENTED;
    use crate::clauses::eqn::Eqn;
    use crate::clauses::eqn_props::{EP_IS_MAXIMAL, EP_IS_PM_INTO_LIT, EP_IS_SELECTED};
    use crate::clauses::eqnlist::EqnList;
    use crate::heuristics::to_params::TermOrdering;
    use crate::orderings::ocb::OrderControlBlock;
    use crate::terms::signature::Signature;
    use crate::terms::termbanks::TermBank;
    use crate::terms::termtypes::{DerefType, Term};
    use crate::terms::typebanks::TypeBank;

    fn test_bank() -> TermBank {
        TermBank::new(Signature::new(TypeBank::new())).unwrap_or_else(|err| panic!("{err}"))
    }

    fn const_term(code: i64) -> Term {
        Term::const_cell_alloc(code)
    }

    fn var_term(code: i64) -> Term {
        Term::const_cell_alloc(code)
    }

    fn typed_var(bank: &TermBank, code: i64) -> Term {
        let var = var_term(code);
        var.set_type(Some(bank.signature().type_bank().default_type()));
        var
    }

    fn unary(code: i64, arg: &Term) -> Term {
        let term = Term::top_alloc(code, 1);
        term.set_argument(0, arg.clone());
        term
    }

    fn binary(code: i64, left: &Term, right: &Term) -> Term {
        let term = Term::top_alloc(code, 2);
        term.set_argument(0, left.clone());
        term.set_argument(1, right.clone());
        term
    }

    fn shared_const(bank: &mut TermBank, name: &str) -> Term {
        let f_code = bank.signature_mut().insert_id(name, 0, false);
        bank.create_const_term(f_code).unwrap()
    }

    fn shared_unary(bank: &mut TermBank, name: &str, arg: &Term) -> Term {
        let f_code = bank.signature_mut().insert_id(name, 1, false);
        bank.insert(&unary(f_code, arg), DerefType::Never).unwrap()
    }

    fn literal(bank: &mut TermBank, left: &Term, right: &Term, positive: bool) -> Eqn {
        Eqn::alloc(left.clone(), right.clone(), bank, positive).unwrap()
    }

    fn predicate_const_atom(bank: &mut TermBank, name: &str) -> Term {
        let bool_type = bank.signature().type_bank().bool_type();
        let f_code = bank.signature_mut().insert_id(name, 0, false);
        bank.signature_mut()
            .declare_final_type(f_code, bool_type.clone())
            .unwrap();
        let atom = Term::const_cell_alloc(f_code);
        atom.set_type(Some(bool_type));
        bank.insert(&atom, DerefType::Never).unwrap()
    }

    fn predicate_literal(bank: &mut TermBank, atom: &Term, positive: bool) -> Eqn {
        let true_term = bank.true_term().clone();
        literal(bank, atom, &true_term, positive)
    }

    fn kbo_ocb(bank: &TermBank) -> OrderControlBlock {
        OrderControlBlock::alloc(
            TermOrdering::Kbo,
            true,
            bank.signature(),
            HoOrderKind::LfhoOrder,
        )
    }

    fn select_mask(clause: &Clause) -> Vec<bool> {
        clause
            .literals()
            .as_slice()
            .iter()
            .map(|literal| literal.query_prop(EP_IS_SELECTED))
            .collect()
    }

    fn selected_indices(clause: &Clause) -> Vec<usize> {
        clause
            .literals()
            .as_slice()
            .iter()
            .enumerate()
            .filter_map(|(index, literal)| literal.query_prop(EP_IS_SELECTED).then_some(index))
            .collect()
    }

    fn first_smallest_ground_negative_index(clause: &Clause) -> usize {
        let mut selected = None;
        let mut selected_weight = i64::MAX;
        for (index, literal) in clause.literals().as_slice().iter().enumerate() {
            if literal.is_negative() && literal.is_ground() {
                let current_weight = literal.standard_weight();
                if current_weight < selected_weight {
                    selected_weight = current_weight;
                    selected = Some(index);
                }
            }
        }
        selected.unwrap()
    }

    fn first_largest_diff_ground_negative_index(clause: &Clause) -> usize {
        let mut selected = None;
        let mut selected_weight = -1;
        for (index, literal) in clause.literals().as_slice().iter().enumerate() {
            if literal.is_negative() && literal.is_ground() {
                let current_weight = 100 * literal.standard_diff() + literal.standard_weight();
                if current_weight > selected_weight {
                    selected_weight = current_weight;
                    selected = Some(index);
                }
            }
        }
        selected.unwrap()
    }

    fn clear_selection(clause: &mut Clause) {
        clause.literals_mut().del_prop(EP_IS_SELECTED);
    }

    fn mark_maximal_literals(clause: &mut Clause, indices: &[usize]) {
        clause.literals_mut().del_prop(EP_IS_MAXIMAL);
        for index in indices {
            clause.literals_mut().as_mut_slice()[*index].set_prop(EP_IS_MAXIMAL);
        }
        clause.set_prop(CP_IS_ORIENTED);
    }

    fn simple_mixed_clause() -> Clause {
        let mut bank = test_bank();
        let a = const_term(10);
        let b = const_term(11);
        let c = const_term(12);
        let d = const_term(13);
        Clause::alloc(EqnList::from_vec(vec![
            literal(&mut bank, &a, &b, false),
            literal(&mut bank, &c, &d, true),
            literal(&mut bank, &b, &c, false),
        ]))
    }

    fn weighted_mixed_clause() -> Clause {
        let mut bank = test_bank();
        let a = const_term(20);
        let b = const_term(21);
        let c = const_term(22);
        let d = const_term(23);
        let f_a = unary(30, &a);
        Clause::alloc(EqnList::from_vec(vec![
            literal(&mut bank, &a, &b, false),
            literal(&mut bank, &c, &d, true),
            literal(&mut bank, &f_a, &b, false),
        ]))
    }

    fn pure_var_clause() -> Clause {
        let mut bank = test_bank();
        let a = const_term(40);
        let b = const_term(41);
        let x = var_term(-2);
        let y = var_term(-4);
        Clause::alloc(EqnList::from_vec(vec![
            literal(&mut bank, &a, &b, false),
            literal(&mut bank, &a, &b, true),
            literal(&mut bank, &x, &y, false),
        ]))
    }

    fn ground_and_nonground_clause(include_ground_negative: bool) -> Clause {
        let mut bank = test_bank();
        let a = shared_const(&mut bank, "ls_ground_a");
        let b = shared_const(&mut bank, "ls_ground_b");
        let c = shared_const(&mut bank, "ls_ground_c");
        let x = var_term(-2);
        x.set_type(Some(bank.signature().type_bank().default_type()));
        let f_a = shared_unary(&mut bank, "ls_ground_f", &a);
        let mut literals = vec![
            literal(&mut bank, &a, &b, true),
            literal(&mut bank, &x, &b, false),
        ];
        if include_ground_negative {
            literals.push(literal(&mut bank, &f_a, &c, false));
        }
        Clause::alloc(EqnList::from_vec(literals))
    }

    fn range_restricted_clause() -> Clause {
        let mut bank = test_bank();
        let x = typed_var(&bank, -10);
        let a = shared_const(&mut bank, "ls_rr_a");
        Clause::alloc(EqnList::from_vec(vec![
            literal(&mut bank, &x, &a, true),
            literal(&mut bank, &x, &a, false),
        ]))
    }

    fn non_range_restricted_clause() -> Clause {
        let mut bank = test_bank();
        let x = typed_var(&bank, -20);
        let y = typed_var(&bank, -22);
        let a = shared_const(&mut bank, "ls_non_rr_a");
        Clause::alloc(EqnList::from_vec(vec![
            literal(&mut bank, &x, &a, true),
            literal(&mut bank, &y, &a, false),
        ]))
    }

    fn anti_range_restricted_clause() -> Clause {
        let mut bank = test_bank();
        let x = typed_var(&bank, -30);
        let y = typed_var(&bank, -32);
        let a = shared_const(&mut bank, "ls_anti_rr_a");
        Clause::alloc(EqnList::from_vec(vec![
            literal(&mut bank, &x, &a, true),
            literal(&mut bank, &x, &y, false),
        ]))
    }

    fn weakly_range_restricted_clause() -> Clause {
        let mut bank = test_bank();
        let x = typed_var(&bank, -40);
        let y = typed_var(&bank, -42);
        let a = shared_const(&mut bank, "ls_weak_rr_a");
        Clause::alloc(EqnList::from_vec(vec![
            literal(&mut bank, &x, &y, true),
            literal(&mut bank, &x, &a, false),
        ]))
    }

    fn conditional_clause(blocking_positive: bool) -> Clause {
        let mut bank = test_bank();
        let x = var_term(-50);
        let y = var_term(-52);
        let z = var_term(-54);
        let a = const_term(120);
        let positive_left = if blocking_positive {
            unary(90, &x)
        } else {
            binary(91, &x, &y)
        };
        let positive_right = if blocking_positive { a.clone() } else { z };
        let neg_left = unary(92, &x);
        Clause::alloc(EqnList::from_vec(vec![
            literal(&mut bank, &positive_left, &positive_right, true),
            literal(&mut bank, &neg_left, &a, false),
        ]))
    }

    fn deep_nonground_clause() -> Clause {
        let mut bank = test_bank();
        let x = var_term(-60);
        let y = var_term(-62);
        let left = unary(100, &unary(101, &x));
        let right = unary(102, &unary(103, &y));
        Clause::alloc(EqnList::from_vec(vec![
            literal(&mut bank, &right, &left, true),
            literal(&mut bank, &left, &x, false),
        ]))
    }

    fn positive_deep_negative_shallow_clause() -> Clause {
        let mut bank = test_bank();
        let x = var_term(-70);
        let y = var_term(-72);
        let a = const_term(130);
        let deep = unary(110, &unary(111, &x));
        Clause::alloc(EqnList::from_vec(vec![
            literal(&mut bank, &deep, &y, true),
            literal(&mut bank, &x, &a, false),
        ]))
    }

    fn complex_diff_fallback_clause() -> Clause {
        let mut bank = test_bank();
        let x = var_term(-90);
        let a = const_term(150);
        let b = const_term(151);
        let deeper = unary(152, &unary(153, &x));
        Clause::alloc(EqnList::from_vec(vec![
            literal(&mut bank, &a, &b, true),
            literal(&mut bank, &x, &a, false),
            literal(&mut bank, &deeper, &a, false),
        ]))
    }

    fn complex_prefer_order_clause() -> Clause {
        let mut bank = test_bank();
        let x = var_term(-100);
        x.set_type(Some(bank.signature().type_bank().default_type()));
        let y = var_term(-102);
        y.set_type(Some(bank.signature().type_bank().default_type()));
        let a = shared_const(&mut bank, "complex_pref_a");
        let b = shared_const(&mut bank, "complex_pref_b");
        let pos = predicate_const_atom(&mut bank, "complex_pref_pos");
        let neq_first = predicate_const_atom(&mut bank, "complex_pref_neq_first");
        let neq_ignored = predicate_const_atom(&mut bank, "complex_pref_neq_ignored");
        Clause::alloc(EqnList::from_vec(vec![
            predicate_literal(&mut bank, &pos, true),
            predicate_literal(&mut bank, &neq_first, false),
            literal(&mut bank, &y, &a, false),
            predicate_literal(&mut bank, &neq_ignored, false),
            literal(&mut bank, &x, &b, false),
        ]))
    }

    fn diversification_clause() -> Clause {
        let mut bank = test_bank();
        let pos = predicate_const_atom(&mut bank, "div_pos");
        let a = shared_const(&mut bank, "div_a");
        let b = shared_const(&mut bank, "div_b");
        let c = shared_const(&mut bank, "div_c");
        let d = shared_const(&mut bank, "div_d");
        Clause::alloc(EqnList::from_vec(vec![
            predicate_literal(&mut bank, &pos, true),
            literal(&mut bank, &a, &b, false),
            literal(&mut bank, &b, &c, false),
            literal(&mut bank, &c, &d, false),
        ]))
    }

    fn orientable_clause(bank: &mut TermBank) -> Clause {
        let pos = predicate_const_atom(bank, "orient_pos");
        let a = shared_const(bank, "orient_a");
        let f_of_a = shared_unary(bank, "orient_f", &a);
        let f_of_f_of_a = shared_unary(bank, "orient_f", &f_of_a);
        Clause::alloc(EqnList::from_vec(vec![
            predicate_literal(bank, &pos, true),
            literal(bank, &f_of_a, &a, false),
            literal(bank, &f_of_f_of_a, &f_of_f_of_a, false),
        ]))
    }

    fn unorientable_clause(bank: &mut TermBank) -> Clause {
        let pos = predicate_const_atom(bank, "unorient_pos");
        let a = shared_const(bank, "unorient_a");
        let f_a = shared_unary(bank, "unorient_f", &a);
        Clause::alloc(EqnList::from_vec(vec![
            predicate_literal(bank, &pos, true),
            literal(bank, &a, &a, false),
            literal(bank, &f_a, &f_a, false),
        ]))
    }

    fn maximal_gate_clause(bank: &mut TermBank) -> Clause {
        let pos = predicate_const_atom(bank, "max_gate_pos");
        let a = shared_const(bank, "max_gate_a");
        let b = shared_const(bank, "max_gate_b");
        let c = shared_const(bank, "max_gate_c");
        Clause::alloc(EqnList::from_vec(vec![
            predicate_literal(bank, &pos, true),
            literal(bank, &a, &b, false),
            literal(bank, &b, &c, false),
        ]))
    }

    #[test]
    fn no_selection_and_no_generation_are_noop_selectors() {
        let mut clause = Clause::empty();

        apply_ported_literal_selector(NO_SELECTION, None, &mut clause).unwrap_or_else(|err| {
            panic!("{err}");
        });
        apply_ported_literal_selector(NO_GENERATION, None, &mut clause).unwrap_or_else(|err| {
            panic!("{err}");
        });

        assert_eq!(clause.prop_lit_number(EP_IS_SELECTED), 0);
    }

    #[test]
    fn negative_literal_selectors_mark_negative_or_all_literals() {
        let mut clause = simple_mixed_clause();

        select_negative_literals(None, &mut clause);
        assert_eq!(select_mask(&clause), vec![false, true, true]);

        clear_selection(&mut clause);
        p_select_negative_literals(None, &mut clause);
        assert_eq!(select_mask(&clause), vec![true, true, true]);
    }

    #[test]
    fn pure_variable_selectors_preserve_c_positive_variant_shape() {
        let mut clause = pure_var_clause();

        select_first_variable_literal(None, &mut clause);
        assert_eq!(select_mask(&clause), vec![false, false, true]);

        clear_selection(&mut clause);
        p_select_first_variable_literal(None, &mut clause);
        assert_eq!(select_mask(&clause), vec![true, false, false]);
    }

    #[test]
    fn largest_and_smallest_negative_selectors_use_first_standard_weight_best() {
        let mut clause = weighted_mixed_clause();

        select_largest_negative_literal(None, &mut clause);
        assert_eq!(select_mask(&clause), vec![false, false, true]);

        clear_selection(&mut clause);
        p_select_largest_negative_literal(None, &mut clause);
        assert_eq!(select_mask(&clause), vec![true, false, true]);

        clear_selection(&mut clause);
        select_smallest_negative_literal(None, &mut clause);
        assert_eq!(select_mask(&clause), vec![false, true, false]);

        clear_selection(&mut clause);
        p_select_smallest_negative_literal(None, &mut clause);
        assert_eq!(select_mask(&clause), vec![true, true, false]);
    }

    #[test]
    fn orientable_selectors_prefer_oriented_negative_literals() {
        let mut bank = test_bank();
        let mut clause = orientable_clause(&mut bank);
        let mut ocb = kbo_ocb(&bank);

        select_largest_orientable_literal(&mut ocb, &bank, &mut clause);
        assert_eq!(select_mask(&clause), vec![false, true, false]);
        assert!(!clause.query_prop(CP_IS_ORIENTED));

        let mut clause = orientable_clause(&mut bank);
        select_smallest_orientable_literal(&mut ocb, &bank, &mut clause);
        assert_eq!(select_mask(&clause), vec![false, true, false]);

        let mut clause = orientable_clause(&mut bank);
        p_select_largest_orientable_literal(&mut ocb, &bank, &mut clause);
        assert_eq!(select_mask(&clause), vec![true, true, false]);

        let mut clause = orientable_clause(&mut bank);
        p_select_smallest_orientable_literal(&mut ocb, &bank, &mut clause);
        assert_eq!(select_mask(&clause), vec![true, true, false]);
    }

    #[test]
    fn orientable_selectors_fall_back_to_weight_when_none_orient() {
        let mut bank = test_bank();
        let mut largest = unorientable_clause(&mut bank);
        let mut ocb = kbo_ocb(&bank);

        select_largest_orientable_literal(&mut ocb, &bank, &mut largest);
        assert_eq!(select_mask(&largest), vec![false, false, true]);

        let mut smallest = unorientable_clause(&mut bank);
        select_smallest_orientable_literal(&mut ocb, &bank, &mut smallest);
        assert_eq!(select_mask(&smallest), vec![false, true, false]);
    }

    #[test]
    fn mixed_orientable_selectors_use_positive_variant_only_for_horn() {
        let mut bank = test_bank();
        let mut horn = orientable_clause(&mut bank);
        let mut ocb = kbo_ocb(&bank);

        m_select_largest_orientable_literal(&mut ocb, &bank, &mut horn);
        assert_eq!(select_mask(&horn), vec![true, true, false]);

        let second_pos = predicate_const_atom(&mut bank, "orient_second_pos");
        let a = shared_const(&mut bank, "orient_nonhorn_a");
        let f_a = shared_unary(&mut bank, "orient_nonhorn_f", &a);
        let mut non_horn = Clause::alloc(EqnList::from_vec(vec![
            predicate_literal(&mut bank, &second_pos, true),
            predicate_literal(&mut bank, &second_pos, true),
            literal(&mut bank, &f_a, &a, false),
        ]));
        let mut ocb = kbo_ocb(&bank);

        m_select_smallest_orientable_literal(&mut ocb, &bank, &mut non_horn);
        assert_eq!(select_mask(&non_horn), vec![false, false, true]);
    }

    #[test]
    fn diff_selectors_prefer_unbalanced_negative_literal() {
        let mut clause = weighted_mixed_clause();

        select_diff_negative_literal(None, &mut clause);
        assert_eq!(select_mask(&clause), vec![false, false, true]);

        clear_selection(&mut clause);
        p_select_diff_negative_literal(None, &mut clause);
        assert_eq!(select_mask(&clause), vec![true, false, true]);
    }

    #[test]
    fn ground_negative_selectors_skip_or_clear_without_ground_candidate() {
        let mut clause = ground_and_nonground_clause(true);

        select_ground_negative_literal(None, &mut clause);
        assert_eq!(select_mask(&clause), vec![false, false, true]);

        clear_selection(&mut clause);
        p_select_ground_negative_literal(None, &mut clause);
        assert_eq!(select_mask(&clause), vec![true, false, true]);

        let mut no_ground = ground_and_nonground_clause(false);
        no_ground.literals_mut().set_prop(EP_IS_SELECTED);

        select_ground_negative_literal(None, &mut no_ground);
        assert_eq!(select_mask(&no_ground), vec![true, true]);

        p_select_ground_negative_literal(None, &mut no_ground);
        assert_eq!(select_mask(&no_ground), vec![false, false]);
    }

    #[test]
    fn optimal_selectors_prefer_ground_negative_or_diff_fallback() {
        let mut clause = ground_and_nonground_clause(true);

        select_optimal_literal(None, &mut clause);
        assert_eq!(select_mask(&clause), vec![false, false, true]);

        clear_selection(&mut clause);
        p_select_optimal_literal(None, &mut clause);
        assert_eq!(select_mask(&clause), vec![false, false, true]);

        let mut no_ground = ground_and_nonground_clause(false);
        select_optimal_literal(None, &mut no_ground);
        assert_eq!(select_mask(&no_ground), vec![false, true]);

        clear_selection(&mut no_ground);
        p_select_optimal_literal(None, &mut no_ground);
        assert_eq!(select_mask(&no_ground), vec![true, true]);
    }

    #[test]
    fn min_optimal_selectors_prefer_ground_negative_or_smallest_fallback() {
        let mut clause = ground_and_nonground_clause(true);

        select_min_optimal_literal(None, &mut clause);
        assert_eq!(select_mask(&clause), vec![false, false, true]);

        clear_selection(&mut clause);
        p_select_min_optimal_literal(None, &mut clause);
        assert_eq!(select_mask(&clause), vec![false, false, true]);

        let mut no_ground = ground_and_nonground_clause(false);
        select_min_optimal_literal(None, &mut no_ground);
        assert_eq!(select_mask(&no_ground), vec![false, true]);

        clear_selection(&mut no_ground);
        p_select_min_optimal_literal(None, &mut no_ground);
        assert_eq!(select_mask(&no_ground), vec![true, true]);
    }

    #[test]
    fn range_restricted_optimal_wrappers_preserve_c_gates() {
        let mut non_rr = non_range_restricted_clause();
        select_non_rr_optimal_literal(None, &mut non_rr);
        assert_eq!(select_mask(&non_rr), vec![false, true]);

        let mut rr = range_restricted_clause();
        select_non_rr_optimal_literal(None, &mut rr);
        assert_eq!(select_mask(&rr), vec![false, false]);

        let mut weak_rr = weakly_range_restricted_clause();
        select_non_strong_rr_optimal_literal(None, &mut weak_rr);
        assert_eq!(select_mask(&weak_rr), vec![false, true]);

        let mut strong_rr = range_restricted_clause();
        select_non_strong_rr_optimal_literal(None, &mut strong_rr);
        assert_eq!(select_mask(&strong_rr), vec![false, false]);

        let mut anti_rr = anti_range_restricted_clause();
        select_anti_rr_optimal_literal(None, &mut anti_rr);
        assert_eq!(select_mask(&anti_rr), vec![false, true]);

        let mut non_anti = non_range_restricted_clause();
        select_anti_rr_optimal_literal(None, &mut non_anti);
        assert_eq!(select_mask(&non_anti), vec![false, false]);

        select_non_anti_rr_optimal_literal(None, &mut non_anti);
        assert_eq!(select_mask(&non_anti), vec![false, true]);
    }

    #[test]
    fn strong_rr_non_rr_wrapper_selects_extremes_and_positive_variant_clears_middle() {
        let mut non_rr = non_range_restricted_clause();
        select_strong_rr_non_rr_optimal_literal(None, &mut non_rr);
        assert_eq!(select_mask(&non_rr), vec![false, true]);

        let mut strong_rr = range_restricted_clause();
        select_strong_rr_non_rr_optimal_literal(None, &mut strong_rr);
        assert_eq!(select_mask(&strong_rr), vec![false, true]);

        let mut weak_rr = weakly_range_restricted_clause();
        weak_rr.literals_mut().set_prop(EP_IS_SELECTED);
        p_select_strong_rr_non_rr_optimal_literal(None, &mut weak_rr);
        assert_eq!(select_mask(&weak_rr), vec![false, false]);
    }

    #[test]
    fn conditional_optimal_selectors_clear_or_fall_back_to_optimal() {
        let mut blocked = conditional_clause(true);
        blocked.literals_mut().set_prop(EP_IS_SELECTED);
        select_cond_optimal_literal(None, &mut blocked);
        assert_eq!(select_mask(&blocked), vec![false, false]);

        let mut allowed = conditional_clause(false);
        select_cond_optimal_literal(None, &mut allowed);
        assert_eq!(select_mask(&allowed), vec![false, true]);

        clear_selection(&mut allowed);
        p_select_cond_optimal_literal(None, &mut allowed);
        assert_eq!(select_mask(&allowed), vec![true, true]);
    }

    #[test]
    fn all_conditional_optimal_selectors_require_a_nonblocking_positive_to_select() {
        let mut all_blocked = conditional_clause(true);
        all_blocked.literals_mut().set_prop(EP_IS_SELECTED);
        select_all_cond_optimal_literal(None, &mut all_blocked);
        assert_eq!(select_mask(&all_blocked), vec![false, false]);

        let mut no_positive = Clause::alloc(EqnList::from_vec(vec![all_blocked
            .literals()
            .as_slice()[1]
            .clone()]));
        no_positive.literals_mut().set_prop(EP_IS_SELECTED);
        select_all_cond_optimal_literal(None, &mut no_positive);
        assert_eq!(select_mask(&no_positive), vec![false]);

        let mut allowed = conditional_clause(false);
        p_select_all_cond_optimal_literal(None, &mut allowed);
        assert_eq!(select_mask(&allowed), vec![true, true]);
    }

    #[test]
    fn depth2_optimal_selectors_gate_by_literal_scope() {
        let mut deep = deep_nonground_clause();
        select_depth2_optimal_literal(None, &mut deep);
        assert_eq!(select_mask(&deep), vec![false, true]);

        clear_selection(&mut deep);
        p_select_depth2_optimal_literal(None, &mut deep);
        assert_eq!(select_mask(&deep), vec![true, true]);

        let mut shallow = ground_and_nonground_clause(false);
        shallow.literals_mut().set_prop(EP_IS_SELECTED);
        select_depth2_optimal_literal(None, &mut shallow);
        assert_eq!(select_mask(&shallow), vec![false, false]);

        let mut positive_deep_negative_shallow = positive_deep_negative_shallow_clause();
        select_p_depth2_optimal_literal(None, &mut positive_deep_negative_shallow);
        assert_eq!(
            select_mask(&positive_deep_negative_shallow),
            vec![false, true]
        );

        clear_selection(&mut positive_deep_negative_shallow);
        positive_deep_negative_shallow
            .literals_mut()
            .set_prop(EP_IS_SELECTED);
        select_n_depth2_optimal_literal(None, &mut positive_deep_negative_shallow);
        assert_eq!(
            select_mask(&positive_deep_negative_shallow),
            vec![false, false]
        );
    }

    #[test]
    fn complex_selectors_preserve_pure_var_and_ground_choice_priority() {
        let mut pure = pure_var_clause();
        select_complex(None, &mut pure);
        assert_eq!(select_mask(&pure), vec![false, false, true]);

        clear_selection(&mut pure);
        p_select_complex(None, &mut pure);
        assert_eq!(select_mask(&pure), vec![false, false, true]);

        let mut ground = ground_and_nonground_clause(true);
        let smallest_ground = first_smallest_ground_negative_index(&ground);
        let largest_diff_ground = first_largest_diff_ground_negative_index(&ground);
        select_complex(None, &mut ground);
        assert_eq!(selected_indices(&ground), vec![smallest_ground]);

        clear_selection(&mut ground);
        p_select_complex(None, &mut ground);
        assert_eq!(selected_indices(&ground), vec![smallest_ground]);

        clear_selection(&mut ground);
        select_l_complex(None, &mut ground);
        assert_eq!(selected_indices(&ground), vec![largest_diff_ground]);

        clear_selection(&mut ground);
        p_select_l_complex(None, &mut ground);
        assert_eq!(selected_indices(&ground), vec![largest_diff_ground]);
    }

    #[test]
    fn complex_positive_variants_select_positives_only_in_diff_fallback() {
        let mut fallback = complex_diff_fallback_clause();
        select_complex(None, &mut fallback);
        assert_eq!(select_mask(&fallback), vec![false, false, true]);

        clear_selection(&mut fallback);
        p_select_complex(None, &mut fallback);
        assert_eq!(select_mask(&fallback), vec![true, false, true]);

        clear_selection(&mut fallback);
        p_select_l_complex(None, &mut fallback);
        assert_eq!(select_mask(&fallback), vec![true, false, true]);
    }

    #[test]
    fn complex_rr_horn_wrappers_skip_only_range_restricted_horn_clauses() {
        let mut rr_horn = range_restricted_clause();
        rr_horn.literals_mut().set_prop(EP_IS_SELECTED);
        select_complex_except_rr_horn(None, &mut rr_horn);
        assert_eq!(select_mask(&rr_horn), vec![true, true]);

        clear_selection(&mut rr_horn);
        p_select_complex_except_rr_horn(None, &mut rr_horn);
        assert_eq!(select_mask(&rr_horn), vec![false, false]);

        let mut non_rr = non_range_restricted_clause();
        select_complex_except_rr_horn(None, &mut non_rr);
        assert_eq!(select_mask(&non_rr), vec![false, true]);

        clear_selection(&mut non_rr);
        p_select_complex_except_rr_horn(None, &mut non_rr);
        assert_eq!(select_mask(&non_rr), vec![true, true]);
    }

    #[test]
    fn complex_prefer_selectors_keep_c_early_break_scan() {
        let mut non_equation_preferred = complex_prefer_order_clause();
        select_complex_prefer_neq(None, &mut non_equation_preferred);
        assert_eq!(
            select_mask(&non_equation_preferred),
            vec![false, true, false, false, false]
        );

        clear_selection(&mut non_equation_preferred);
        p_select_complex_prefer_neq(None, &mut non_equation_preferred);
        assert_eq!(
            select_mask(&non_equation_preferred),
            vec![true, true, false, false, false]
        );

        let mut equation_preferred = complex_prefer_order_clause();
        select_complex_prefer_eq(None, &mut equation_preferred);
        assert_eq!(
            select_mask(&equation_preferred),
            vec![false, false, true, false, false]
        );

        clear_selection(&mut equation_preferred);
        p_select_complex_prefer_eq(None, &mut equation_preferred);
        assert_eq!(
            select_mask(&equation_preferred),
            vec![true, false, true, false, false]
        );
    }

    #[test]
    fn diversification_selectors_preserve_c_counter_and_into_priority() {
        reset_literal_weight_counter_for_tests();
        let mut clause = diversification_clause();
        clause.set_prop(CP_IS_ORIENTED);

        select_diversification_literals(None, &mut clause);

        assert_eq!(selected_indices(&clause), vec![3]);
        assert!(!clause.query_prop(CP_IS_ORIENTED));

        reset_literal_weight_counter_for_tests();
        let mut by_name = diversification_clause();
        apply_ported_literal_selector(SELECT_DIV_LITS, None, &mut by_name).unwrap_or_else(|err| {
            panic!("{err}");
        });
        assert_eq!(selected_indices(&by_name), vec![3]);

        reset_literal_weight_counter_for_tests();
        let mut clause = diversification_clause();
        clause.literals_mut().as_mut_slice()[2].set_prop(EP_IS_PM_INTO_LIT);

        select_diversification_prefer_into_literals(None, &mut clause);

        assert_eq!(selected_indices(&clause), vec![2]);

        reset_literal_weight_counter_for_tests();
        let mut by_name = diversification_clause();
        by_name.literals_mut().as_mut_slice()[2].set_prop(EP_IS_PM_INTO_LIT);
        apply_ported_literal_selector(SELECT_DIV_PREFER_INTO_LITS, None, &mut by_name)
            .unwrap_or_else(|err| {
                panic!("{err}");
            });
        assert_eq!(selected_indices(&by_name), vec![2]);
    }

    #[test]
    fn unless_uniq_max_selectors_gate_on_total_maximal_count() {
        let mut bank = test_bank();
        let mut ocb = kbo_ocb(&bank);
        let mut blocked = maximal_gate_clause(&mut bank);
        mark_maximal_literals(&mut blocked, &[0]);

        select_unless_uniq_max_optimal_literal(&mut ocb, &bank, &mut blocked);

        assert_eq!(selected_indices(&blocked), Vec::<usize>::new());
        assert!(blocked.query_prop(CP_IS_ORIENTED));

        let mut allowed = maximal_gate_clause(&mut bank);
        mark_maximal_literals(&mut allowed, &[0, 1]);

        select_unless_uniq_max_optimal_literal(&mut ocb, &bank, &mut allowed);

        assert_eq!(selected_indices(&allowed), vec![1]);
        assert!(!allowed.query_prop(CP_IS_ORIENTED));

        let mut positive_variant = maximal_gate_clause(&mut bank);
        mark_maximal_literals(&mut positive_variant, &[0, 1]);

        p_select_unless_uniq_max_optimal_literal(&mut ocb, &bank, &mut positive_variant);

        assert_eq!(selected_indices(&positive_variant), vec![1]);
    }

    #[test]
    fn unless_positive_max_and_unique_positive_max_gates_match_c() {
        let mut bank = test_bank();
        let mut ocb = kbo_ocb(&bank);
        let mut positive_max = maximal_gate_clause(&mut bank);
        mark_maximal_literals(&mut positive_max, &[0]);

        select_unless_pos_max_optimal_literal(&mut ocb, &bank, &mut positive_max);

        assert_eq!(selected_indices(&positive_max), Vec::<usize>::new());

        let mut no_positive_max = maximal_gate_clause(&mut bank);
        mark_maximal_literals(&mut no_positive_max, &[1]);

        select_unless_pos_max_optimal_literal(&mut ocb, &bank, &mut no_positive_max);

        assert_eq!(selected_indices(&no_positive_max), vec![1]);

        let mut one_positive_plus_negative = maximal_gate_clause(&mut bank);
        mark_maximal_literals(&mut one_positive_plus_negative, &[0, 1]);

        select_unless_uniq_pos_max_optimal_literal(
            &mut ocb,
            &bank,
            &mut one_positive_plus_negative,
        );

        assert_eq!(
            selected_indices(&one_positive_plus_negative),
            Vec::<usize>::new()
        );

        let mut one_positive_not_only_maximal = maximal_gate_clause(&mut bank);
        mark_maximal_literals(&mut one_positive_not_only_maximal, &[0, 1]);

        select_unless_uniq_max_pos_optimal_literal(
            &mut ocb,
            &bank,
            &mut one_positive_not_only_maximal,
        );

        assert_eq!(selected_indices(&one_positive_not_only_maximal), vec![1]);
    }

    #[test]
    fn bank_aware_unless_max_selectors_are_available_by_c_strategy_name() {
        for name in [
            SELECT_UNLESS_UNIQ_MAX,
            P_SELECT_UNLESS_UNIQ_MAX,
            SELECT_UNLESS_POS_MAX,
            P_SELECT_UNLESS_POS_MAX,
            SELECT_UNLESS_UNIQ_POS_MAX,
            P_SELECT_UNLESS_UNIQ_POS_MAX,
            SELECT_UNLESS_UNIQ_MAX_POS,
            P_SELECT_UNLESS_UNIQ_MAX_POS,
        ] {
            let mut bank = test_bank();
            let mut ocb = kbo_ocb(&bank);
            let mut clause = maximal_gate_clause(&mut bank);
            mark_maximal_literals(&mut clause, &[1, 2]);

            apply_ported_literal_selector_with_bank(name, Some(&mut ocb), Some(&bank), &mut clause)
                .unwrap_or_else(|err| {
                    panic!("{err}");
                });
            assert_eq!(selected_indices(&clause), vec![1]);
        }
    }

    #[test]
    fn bank_aware_orientable_selectors_are_available_by_c_strategy_name() {
        for name in [
            SELECT_LARGEST_ORIENTABLE,
            P_SELECT_LARGEST_ORIENTABLE,
            M_SELECT_LARGEST_ORIENTABLE,
            SELECT_SMALLEST_ORIENTABLE,
            P_SELECT_SMALLEST_ORIENTABLE,
            M_SELECT_SMALLEST_ORIENTABLE,
        ] {
            let mut bank = test_bank();
            let mut clause = orientable_clause(&mut bank);
            let mut ocb = kbo_ocb(&bank);

            apply_ported_literal_selector_with_bank(name, Some(&mut ocb), Some(&bank), &mut clause)
                .unwrap_or_else(|err| {
                    panic!("{err}");
                });
            assert!(clause.prop_lit_number(EP_IS_SELECTED) >= 1);
        }
    }

    #[test]
    fn simple_selectors_are_available_by_c_strategy_name() {
        for name in [
            SELECT_NEGATIVE_LITERALS,
            P_SELECT_NEGATIVE_LITERALS,
            SELECT_PURE_VAR_NEG_LITERALS,
            P_SELECT_PURE_VAR_NEG_LITERALS,
            SELECT_LARGEST_NEG_LIT,
            P_SELECT_LARGEST_NEG_LIT,
            SELECT_SMALLEST_NEG_LIT,
            P_SELECT_SMALLEST_NEG_LIT,
            SELECT_DIFF_NEG_LIT,
            P_SELECT_DIFF_NEG_LIT,
            SELECT_GROUND_NEG_LIT,
            P_SELECT_GROUND_NEG_LIT,
            SELECT_OPTIMAL_LIT,
            P_SELECT_OPTIMAL_LIT,
            SELECT_MIN_OPTIMAL_LIT,
            P_SELECT_MIN_OPTIMAL_LIT,
            SELECT_NON_RR_OPTIMAL_LIT,
            P_SELECT_NON_RR_OPTIMAL_LIT,
            SELECT_NON_STRONG_RR_OPTIMAL_LIT,
            P_SELECT_NON_STRONG_RR_OPTIMAL_LIT,
            SELECT_ANTI_RR_OPTIMAL_LIT,
            P_SELECT_ANTI_RR_OPTIMAL_LIT,
            SELECT_NON_ANTI_RR_OPTIMAL_LIT,
            P_SELECT_NON_ANTI_RR_OPTIMAL_LIT,
            SELECT_STRONG_RR_NON_RR_OPTIMAL_LIT,
            P_SELECT_STRONG_RR_NON_RR_OPTIMAL_LIT,
            SELECT_COND_OPTIMAL_LIT,
            P_SELECT_COND_OPTIMAL_LIT,
            SELECT_ALL_COND_OPTIMAL_LIT,
            P_SELECT_ALL_COND_OPTIMAL_LIT,
            SELECT_OPTIMAL_RESTR_DEPTH2,
            P_SELECT_OPTIMAL_RESTR_DEPTH2,
            SELECT_OPTIMAL_RESTR_P_DEPTH2,
            P_SELECT_OPTIMAL_RESTR_P_DEPTH2,
            SELECT_OPTIMAL_RESTR_N_DEPTH2,
            P_SELECT_OPTIMAL_RESTR_N_DEPTH2,
            SELECT_COMPLEX,
            P_SELECT_COMPLEX,
            SELECT_COMPLEX_EXCEPT_RR_HORN,
            P_SELECT_COMPLEX_EXCEPT_RR_HORN,
            SELECT_L_COMPLEX,
            P_SELECT_L_COMPLEX,
            SELECT_COMPLEX_PREFER_NEQ,
            P_SELECT_COMPLEX_PREFER_NEQ,
            SELECT_COMPLEX_PREFER_EQ,
            P_SELECT_COMPLEX_PREFER_EQ,
        ] {
            let mut clause = ground_and_nonground_clause(true);
            apply_ported_literal_selector(name, None, &mut clause).unwrap_or_else(|err| {
                panic!("{err}");
            });
        }
    }

    #[test]
    fn unported_selector_reports_name() {
        let mut clause = Clause::empty();
        let error =
            apply_ported_literal_selector("SelectMaxLComplex", None, &mut clause).unwrap_err();

        assert_eq!(error.strategy(), "SelectMaxLComplex");
        assert!(error.to_string().contains("not ported yet"));
    }
}
