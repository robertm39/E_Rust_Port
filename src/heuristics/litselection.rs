use crate::clauses::clause::Clause;
use crate::clauses::clause_props::CP_IS_ORIENTED;
use crate::clauses::eqn::Eqn;
use crate::clauses::eqn_props::{
    EP_IS_EQU_LITERAL, EP_IS_MAXIMAL, EP_IS_PM_INTO_LIT, EP_IS_POSITIVE, EP_IS_SELECTED,
};
use crate::orderings::ocb::OrderControlBlock;
use crate::terms::termbanks::TermBank;
use crate::terms::termfunc::{term_standard_weight, term_weight_compute};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicI64, Ordering as AtomicOrdering};
#[cfg(test)]
use std::sync::{Mutex, MutexGuard};

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
pub const SELECT_MIN_OPTIMAL_NO_TYPE_PRED: &str = "SelectMinOptimalNoTypePred";
pub const P_SELECT_MIN_OPTIMAL_NO_TYPE_PRED: &str = "PSelectMinOptimalNoTypePred";
pub const SELECT_MIN_OPTIMAL_NO_X_TYPE_PRED: &str = "SelectMinOptimalNoXTypePred";
pub const P_SELECT_MIN_OPTIMAL_NO_X_TYPE_PRED: &str = "PSelectMinOptimalNoXTypePred";
pub const SELECT_MIN_OPTIMAL_NO_RX_TYPE_PRED: &str = "SelectMinOptimalNoRXTypePred";
pub const P_SELECT_MIN_OPTIMAL_NO_RX_TYPE_PRED: &str = "PSelectMinOptimalNoRXTypePred";
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
pub const SELECT_UNLESS_UNIQ_MAX_SMALLEST_ORIENTABLE: &str =
    "SelectUnlessUniqMaxSmallestOrientable";
pub const P_SELECT_UNLESS_UNIQ_MAX_SMALLEST_ORIENTABLE: &str =
    "PSelectUnlessUniqMaxSmallestOrientable";
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
pub const SELECT_COMPLEX_AHP: &str = "SelectComplexAHP";
pub const P_SELECT_COMPLEX_AHP: &str = "PSelectComplexAHP";
pub const SELECT_COMPLEX_AHP_EXCEPT_RR_HORN: &str = "SelectComplexAHPExceptRRHorn";
pub const P_SELECT_COMPLEX_AHP_EXCEPT_RR_HORN: &str = "PSelectComplexAHPExceptRRHorn";
pub const SELECT_COMPLEX_EXCEPT_UNIQ_MAX_HORN: &str = "SelectComplexExceptUniqMaxHorn";
pub const P_SELECT_COMPLEX_EXCEPT_UNIQ_MAX_HORN: &str = "PSelectComplexExceptUniqMaxHorn";
pub const M_SELECT_COMPLEX_EXCEPT_UNIQ_MAX_HORN: &str = "MSelectComplexExceptUniqMaxHorn";
pub const SELECT_COMPLEX_EXCEPT_UNIQ_MAX_POS_HORN: &str = "SelectComplexExceptUniqMaxPosHorn";
pub const P_SELECT_COMPLEX_EXCEPT_UNIQ_MAX_POS_HORN: &str = "PSelectComplexExceptUniqMaxPosHorn";
pub const SELECT_L_COMPLEX: &str = "SelectLComplex";
pub const P_SELECT_L_COMPLEX: &str = "PSelectLComplex";
pub const SELECT_COMPLEX_PREFER_NEQ: &str = "SelectComplexPreferNEQ";
pub const P_SELECT_COMPLEX_PREFER_NEQ: &str = "PSelectComplexPreferNEQ";
pub const SELECT_COMPLEX_PREFER_EQ: &str = "SelectComplexPreferEQ";
pub const P_SELECT_COMPLEX_PREFER_EQ: &str = "PSelectComplexPreferEQ";
pub const SELECT_MAX_L_COMPLEX: &str = "SelectMaxLComplex";
pub const P_SELECT_MAX_L_COMPLEX: &str = "PSelectMaxLComplex";
pub const SELECT_MAX_L_COMPLEX_NO_TYPE_PRED: &str = "SelectMaxLComplexNoTypePred";
pub const P_SELECT_MAX_L_COMPLEX_NO_TYPE_PRED: &str = "PSelectMaxLComplexNoTypePred";
pub const SELECT_MAX_L_COMPLEX_NO_X_TYPE_PRED: &str = "SelectMaxLComplexNoXTypePred";
pub const P_SELECT_MAX_L_COMPLEX_NO_X_TYPE_PRED: &str = "PSelectMaxLComplexNoXTypePred";
pub const SELECT_MAX_L_COMPLEX_G: &str = "SelectMaxLComplexG";
pub const SELECT_MAX_L_COMPLEX_AVOID_POS_PRED: &str = "SelectMaxLComplexAvoidPosPred";
pub const SELECT_MAX_L_COMPLEX_APP_NT_NP: &str = "SelectMaxLComplexAPPNTNp";
pub const SELECT_MAX_L_COMPLEX_APP_NO_TYPE: &str = "SelectMaxLComplexAPPNoType";
pub const SELECT_MAX_L_COMPLEX_AVOID_POS_U_PRED: &str = "SelectMaxLComplexAvoidPosUPred";
pub const SELECT_MAX_L_COMPLEX_AVOID_APP_VAR: &str = "SelectMaxLComplexAvoidAppVar";
pub const SELECT_MAX_L_COMPLEX_STRONGLY_AVOID_APP_VAR: &str =
    "SelectMaxLComplexStronglyAvoidAppVar";
pub const SELECT_MAX_L_COMPLEX_PREFER_APP_VAR: &str = "SelectMaxLComplexPreferAppVar";
pub const SELECT_NEW_COMPLEX: &str = "SelectNewComplex";
pub const P_SELECT_NEW_COMPLEX: &str = "PSelectNewComplex";
pub const SELECT_NEW_COMPLEX_EXCEPT_UNIQ_MAX_HORN: &str = "SelectNewComplexExceptUniqMaxHorn";
pub const P_SELECT_NEW_COMPLEX_EXCEPT_UNIQ_MAX_HORN: &str = "PSelectNewComplexExceptUniqMaxHorn";
pub const SELECT_MIN_INFPOS: &str = "SelectMinInfpos";
pub const P_SELECT_MIN_INFPOS: &str = "PSelectMinInfpos";
pub const H_SELECT_MIN_INFPOS: &str = "HSelectMinInfpos";
pub const G_SELECT_MIN_INFPOS: &str = "GSelectMinInfpos";
pub const SELECT_MIN_INFPOS_NO_TYPE_PRED: &str = "SelectMinInfposNoTypePred";
pub const P_SELECT_MIN_INFPOS_NO_TYPE_PRED: &str = "PSelectMinInfposNoTypePred";
pub const SELECT_MIN2_INFPOS: &str = "SelectMin2Infpos";
pub const P_SELECT_MIN2_INFPOS: &str = "PSelectMin2Infpos";
pub const SELECT_NEW_COMPLEX_AHP: &str = "SelectNewComplexAHP";
pub const P_SELECT_NEW_COMPLEX_AHP: &str = "PSelectNewComplexAHP";
pub const SELECT_NEW_COMPLEX_AHP_EXCEPT_RR_HORN: &str = "SelectNewComplexAHPExceptRRHorn";
pub const P_SELECT_NEW_COMPLEX_AHP_EXCEPT_RR_HORN: &str = "PSelectNewComplexAHPExceptRRHorn";
pub const SELECT_NEW_COMPLEX_AHP_EXCEPT_UNIQ_MAX_HORN: &str =
    "SelectNewComplexAHPExceptUniqMaxHorn";
pub const P_SELECT_NEW_COMPLEX_AHP_EXCEPT_UNIQ_MAX_HORN: &str =
    "PSelectNewComplexAHPExceptUniqMaxHorn";
pub const SELECT_NEW_COMPLEX_AHP_NS: &str = "SelectNewComplexAHPNS";
pub const SELECT_VG_NON_CR: &str = "SelectVGNonCR";
pub const SELECT_CQ_AR_EQ_LAST: &str = "SelectCQArEqLast";
pub const SELECT_CQ_AR_EQ_FIRST: &str = "SelectCQArEqFirst";
pub const SELECT_CQI_AR_EQ_LAST: &str = "SelectCQIArEqLast";
pub const SELECT_CQI_AR_EQ_FIRST: &str = "SelectCQIArEqFirst";
pub const SELECT_CQ_AR: &str = "SelectCQAr";
pub const SELECT_CQI_AR: &str = "SelectCQIAr";
pub const SELECT_CQ_AR_NP_EQ_FIRST: &str = "SelectCQArNpEqFirst";
pub const SELECT_CQI_AR_NP_EQ_FIRST: &str = "SelectCQIArNpEqFirst";
pub const SELECT_CQ_GR_AR_EQ_FIRST: &str = "SelectCQGrArEqFirst";
pub const SELECT_CQ_AR_NT_EQ_FIRST: &str = "SelectCQArNTEqFirst";
pub const SELECT_CQI_AR_NT_EQ_FIRST: &str = "SelectCQIArNTEqFirst";
pub const SELECT_CQ_AR_NT_NP_EQ_FIRST: &str = "SelectCQArNTNpEqFirst";
pub const SELECT_CQI_AR_NT_NP_EQ_FIRST: &str = "SelectCQIArNTNpEqFirst";
pub const SELECT_CQ_AR_NXT_EQ_FIRST: &str = "SelectCQArNXTEqFirst";
pub const SELECT_CQI_AR_NXT_EQ_FIRST: &str = "SelectCQIArNXTEqFirst";
pub const SELECT_CQ_AR_NT_NP: &str = "SelectCQArNTNp";
pub const SELECT_CQI_AR_NT_NP: &str = "SelectCQIArNTNp";
pub const SELECT_CQ_AR_NT: &str = "SelectCQArNT";
pub const SELECT_CQI_AR_NT: &str = "SelectCQIArNT";
pub const SELECT_CQ_AR_NP: &str = "SelectCQArNp";
pub const SELECT_CQI_AR_NP: &str = "SelectCQIArNp";
pub const SELECT_CQ_AR_NP_EQ_FIRST_UNLESS_PDOM: &str = "SelectCQArNpEqFirstUnlessPDom";
pub const SELECT_CQ_AR_NT_EQ_FIRST_UNLESS_PDOM: &str = "SelectCQArNTEqFirstUnlessPDom";
pub const SELECT_CQ_PREC_W: &str = "SelectCQPrecW";
pub const SELECT_CQI_PREC_W: &str = "SelectCQIPrecW";
pub const SELECT_CQ_PREC_W_NT_NP: &str = "SelectCQPrecWNTNp";
pub const SELECT_CQI_PREC_W_NT_NP: &str = "SelectCQIPrecWNTNp";
pub const SELECT_DIV_LITS: &str = "SelectDivLits";
pub const SELECT_DIV_PREFER_INTO_LITS: &str = "SelectDivPreferIntoLits";

const VAR_FACTOR: i64 = 3;
const CQ_FORBIDDEN_WEIGHT: i64 = 100_000;
const CQ_GROUND_BIAS: i64 = 2_000_000;
static LITERAL_WEIGHT_COUNTER: AtomicI64 = AtomicI64::new(0);
#[cfg(test)]
static LITERAL_WEIGHT_COUNTER_TEST_LOCK: Mutex<()> = Mutex::new(());

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
enum MinOptimalTypeSelector {
    RejectType,
    PositiveRejectType,
    RejectX,
    PositiveRejectX,
    RejectRealX,
    PositiveRejectRealX,
}

impl MinOptimalTypeSelector {
    fn from_name(name: &str) -> Option<Self> {
        match name {
            SELECT_MIN_OPTIMAL_NO_TYPE_PRED => Some(Self::RejectType),
            P_SELECT_MIN_OPTIMAL_NO_TYPE_PRED => Some(Self::PositiveRejectType),
            SELECT_MIN_OPTIMAL_NO_X_TYPE_PRED => Some(Self::RejectX),
            P_SELECT_MIN_OPTIMAL_NO_X_TYPE_PRED => Some(Self::PositiveRejectX),
            SELECT_MIN_OPTIMAL_NO_RX_TYPE_PRED => Some(Self::RejectRealX),
            P_SELECT_MIN_OPTIMAL_NO_RX_TYPE_PRED => Some(Self::PositiveRejectRealX),
            _ => None,
        }
    }

    fn apply(self, ocb: Option<&mut OrderControlBlock>, bank: &TermBank, clause: &mut Clause) {
        match self {
            Self::RejectType => select_min_optimal_no_type_pred(ocb, bank, clause),
            Self::PositiveRejectType => p_select_min_optimal_no_type_pred(ocb, bank, clause),
            Self::RejectX => select_min_optimal_no_x_type_pred(ocb, bank, clause),
            Self::PositiveRejectX => p_select_min_optimal_no_x_type_pred(ocb, bank, clause),
            Self::RejectRealX => select_min_optimal_no_rx_type_pred(ocb, bank, clause),
            Self::PositiveRejectRealX => {
                p_select_min_optimal_no_rx_type_pred(ocb, bank, clause);
            }
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
enum MinOptimalTypeFilter {
    Predicate,
    Extended,
    RealExtended,
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
    UnlessUniqMaxSmallestOrientable,
    PUnlessUniqMaxSmallestOrientable,
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
            SELECT_UNLESS_UNIQ_MAX_SMALLEST_ORIENTABLE => {
                Some(Self::UnlessUniqMaxSmallestOrientable)
            }
            P_SELECT_UNLESS_UNIQ_MAX_SMALLEST_ORIENTABLE => {
                Some(Self::PUnlessUniqMaxSmallestOrientable)
            }
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
            Self::UnlessUniqMaxSmallestOrientable => {
                select_unless_uniq_max_smallest_orientable(ocb, bank, clause);
            }
            Self::PUnlessUniqMaxSmallestOrientable => {
                p_select_unless_uniq_max_smallest_orientable(ocb, bank, clause);
            }
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
enum MaximalComplexSelector {
    Standard,
    Positive,
    Mixed,
    StandardPositiveMax,
    PositivePositiveMax,
}

impl MaximalComplexSelector {
    fn from_name(name: &str) -> Option<Self> {
        match name {
            SELECT_COMPLEX_EXCEPT_UNIQ_MAX_HORN => Some(Self::Standard),
            P_SELECT_COMPLEX_EXCEPT_UNIQ_MAX_HORN => Some(Self::Positive),
            M_SELECT_COMPLEX_EXCEPT_UNIQ_MAX_HORN => Some(Self::Mixed),
            SELECT_COMPLEX_EXCEPT_UNIQ_MAX_POS_HORN => Some(Self::StandardPositiveMax),
            P_SELECT_COMPLEX_EXCEPT_UNIQ_MAX_POS_HORN => Some(Self::PositivePositiveMax),
            _ => None,
        }
    }

    fn apply(self, ocb: &mut OrderControlBlock, bank: &TermBank, clause: &mut Clause) {
        match self {
            Self::Standard => select_complex_except_uniq_max_horn(ocb, bank, clause),
            Self::Positive => p_select_complex_except_uniq_max_horn(ocb, bank, clause),
            Self::Mixed => m_select_complex_except_uniq_max_horn(ocb, bank, clause),
            Self::StandardPositiveMax => {
                select_complex_except_uniq_max_pos_horn(ocb, bank, clause);
            }
            Self::PositivePositiveMax => {
                p_select_complex_except_uniq_max_pos_horn(ocb, bank, clause);
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum MaxLComplexSelector {
    Standard,
    Positive,
    NoTypePred,
    PositiveNoTypePred,
    NoXTypePred,
    PositiveNoXTypePred,
}

impl MaxLComplexSelector {
    fn from_name(name: &str) -> Option<Self> {
        match name {
            SELECT_MAX_L_COMPLEX => Some(Self::Standard),
            P_SELECT_MAX_L_COMPLEX => Some(Self::Positive),
            SELECT_MAX_L_COMPLEX_NO_TYPE_PRED => Some(Self::NoTypePred),
            P_SELECT_MAX_L_COMPLEX_NO_TYPE_PRED => Some(Self::PositiveNoTypePred),
            SELECT_MAX_L_COMPLEX_NO_X_TYPE_PRED => Some(Self::NoXTypePred),
            P_SELECT_MAX_L_COMPLEX_NO_X_TYPE_PRED => Some(Self::PositiveNoXTypePred),
            _ => None,
        }
    }

    fn apply(self, ocb: &mut OrderControlBlock, bank: &TermBank, clause: &mut Clause) {
        match self {
            Self::Standard => select_max_l_complex(ocb, bank, clause),
            Self::Positive => p_select_max_l_complex(ocb, bank, clause),
            Self::NoTypePred => select_max_l_complex_no_type_pred(ocb, bank, clause),
            Self::PositiveNoTypePred => p_select_max_l_complex_no_type_pred(ocb, bank, clause),
            Self::NoXTypePred => select_max_l_complex_no_x_type_pred(ocb, bank, clause),
            Self::PositiveNoXTypePred => p_select_max_l_complex_no_x_type_pred(ocb, bank, clause),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum GenericMaxLComplexSelector {
    Generic,
    AvoidPositivePredicate,
    AvoidPropositionalTypePredicate,
    AvoidTypePredicate,
    AvoidPositiveUninterpretedPredicate,
    AvoidAppVar,
    StronglyAvoidAppVar,
    PreferAppVar,
}

impl GenericMaxLComplexSelector {
    fn from_name(name: &str) -> Option<Self> {
        match name {
            SELECT_MAX_L_COMPLEX_G => Some(Self::Generic),
            SELECT_MAX_L_COMPLEX_AVOID_POS_PRED => Some(Self::AvoidPositivePredicate),
            SELECT_MAX_L_COMPLEX_APP_NT_NP => Some(Self::AvoidPropositionalTypePredicate),
            SELECT_MAX_L_COMPLEX_APP_NO_TYPE => Some(Self::AvoidTypePredicate),
            SELECT_MAX_L_COMPLEX_AVOID_POS_U_PRED => {
                Some(Self::AvoidPositiveUninterpretedPredicate)
            }
            SELECT_MAX_L_COMPLEX_AVOID_APP_VAR => Some(Self::AvoidAppVar),
            SELECT_MAX_L_COMPLEX_STRONGLY_AVOID_APP_VAR => Some(Self::StronglyAvoidAppVar),
            SELECT_MAX_L_COMPLEX_PREFER_APP_VAR => Some(Self::PreferAppVar),
            _ => None,
        }
    }

    fn apply(self, ocb: &mut OrderControlBlock, bank: &TermBank, clause: &mut Clause) {
        match self {
            Self::Generic => select_max_l_complex_g(ocb, bank, clause),
            Self::AvoidPositivePredicate => {
                select_max_l_complex_avoid_pos_pred(ocb, bank, clause);
            }
            Self::AvoidPropositionalTypePredicate => {
                select_max_l_complex_app_nt_np(ocb, bank, clause);
            }
            Self::AvoidTypePredicate => select_max_l_complex_app_no_type(ocb, bank, clause),
            Self::AvoidPositiveUninterpretedPredicate => {
                select_max_l_complex_avoid_pos_u_pred(ocb, bank, clause);
            }
            Self::AvoidAppVar => select_max_l_complex_avoid_app_var(ocb, bank, clause),
            Self::StronglyAvoidAppVar => {
                select_max_l_complex_strongly_avoid_app_var(ocb, bank, clause);
            }
            Self::PreferAppVar => select_max_l_complex_prefer_app_var(ocb, bank, clause),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum NewComplexSelector {
    Standard,
    Positive,
    ExceptUniqueMaxHorn,
    PositiveExceptUniqueMaxHorn,
}

impl NewComplexSelector {
    fn from_name(name: &str) -> Option<Self> {
        match name {
            SELECT_NEW_COMPLEX => Some(Self::Standard),
            P_SELECT_NEW_COMPLEX => Some(Self::Positive),
            SELECT_NEW_COMPLEX_EXCEPT_UNIQ_MAX_HORN => Some(Self::ExceptUniqueMaxHorn),
            P_SELECT_NEW_COMPLEX_EXCEPT_UNIQ_MAX_HORN => Some(Self::PositiveExceptUniqueMaxHorn),
            _ => None,
        }
    }

    fn apply(self, ocb: &mut OrderControlBlock, bank: &TermBank, clause: &mut Clause) {
        match self {
            Self::Standard => select_new_complex(ocb, bank, clause),
            Self::Positive => p_select_new_complex(ocb, bank, clause),
            Self::ExceptUniqueMaxHorn => {
                select_new_complex_except_uniq_max_horn(ocb, bank, clause);
            }
            Self::PositiveExceptUniqueMaxHorn => {
                p_select_new_complex_except_uniq_max_horn(ocb, bank, clause);
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum MinInfposSelector {
    Standard,
    Positive,
    PositiveIfNonGround,
    PositiveIfGround,
    NoTypePred,
    PositiveNoTypePred,
    Min2,
    PMin2,
}

impl MinInfposSelector {
    fn from_name(name: &str) -> Option<Self> {
        match name {
            SELECT_MIN_INFPOS => Some(Self::Standard),
            P_SELECT_MIN_INFPOS => Some(Self::Positive),
            H_SELECT_MIN_INFPOS => Some(Self::PositiveIfNonGround),
            G_SELECT_MIN_INFPOS => Some(Self::PositiveIfGround),
            SELECT_MIN_INFPOS_NO_TYPE_PRED => Some(Self::NoTypePred),
            P_SELECT_MIN_INFPOS_NO_TYPE_PRED => Some(Self::PositiveNoTypePred),
            SELECT_MIN2_INFPOS => Some(Self::Min2),
            P_SELECT_MIN2_INFPOS => Some(Self::PMin2),
            _ => None,
        }
    }

    fn apply(self, ocb: &mut OrderControlBlock, bank: &TermBank, clause: &mut Clause) {
        match self {
            Self::Standard => select_min_infpos(ocb, bank, clause),
            Self::Positive => p_select_min_infpos(ocb, bank, clause),
            Self::PositiveIfNonGround => h_select_min_infpos(ocb, bank, clause),
            Self::PositiveIfGround => g_select_min_infpos(ocb, bank, clause),
            Self::NoTypePred => select_min_infpos_no_type_pred(ocb, bank, clause),
            Self::PositiveNoTypePred => p_select_min_infpos_no_type_pred(ocb, bank, clause),
            Self::Min2 => select_min2_infpos(ocb, bank, clause),
            Self::PMin2 => p_select_min2_infpos(ocb, bank, clause),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum AhpLiteralSelector {
    Complex,
    PositiveComplex,
    ComplexExceptRRHorn,
    PositiveComplexExceptRRHorn,
    NewComplex,
    PositiveNewComplex,
    NewComplexExceptRRHorn,
    PositiveNewComplexExceptRRHorn,
    NewComplexExceptUniqueMaxHorn,
    PositiveNewComplexExceptUniqueMaxHorn,
    NewComplexNoSplit,
}

impl AhpLiteralSelector {
    fn from_name(name: &str) -> Option<Self> {
        match name {
            SELECT_COMPLEX_AHP => Some(Self::Complex),
            P_SELECT_COMPLEX_AHP => Some(Self::PositiveComplex),
            SELECT_COMPLEX_AHP_EXCEPT_RR_HORN => Some(Self::ComplexExceptRRHorn),
            P_SELECT_COMPLEX_AHP_EXCEPT_RR_HORN => Some(Self::PositiveComplexExceptRRHorn),
            SELECT_NEW_COMPLEX_AHP => Some(Self::NewComplex),
            P_SELECT_NEW_COMPLEX_AHP => Some(Self::PositiveNewComplex),
            SELECT_NEW_COMPLEX_AHP_EXCEPT_RR_HORN => Some(Self::NewComplexExceptRRHorn),
            P_SELECT_NEW_COMPLEX_AHP_EXCEPT_RR_HORN => Some(Self::PositiveNewComplexExceptRRHorn),
            SELECT_NEW_COMPLEX_AHP_EXCEPT_UNIQ_MAX_HORN => {
                Some(Self::NewComplexExceptUniqueMaxHorn)
            }
            P_SELECT_NEW_COMPLEX_AHP_EXCEPT_UNIQ_MAX_HORN => {
                Some(Self::PositiveNewComplexExceptUniqueMaxHorn)
            }
            SELECT_NEW_COMPLEX_AHP_NS => Some(Self::NewComplexNoSplit),
            _ => None,
        }
    }

    fn apply(self, ocb: &mut OrderControlBlock, bank: &TermBank, clause: &mut Clause) {
        match self {
            Self::Complex => select_complex_ahp(ocb, bank, clause),
            Self::PositiveComplex => p_select_complex_ahp(ocb, bank, clause),
            Self::ComplexExceptRRHorn => select_complex_ahp_except_rr_horn(ocb, bank, clause),
            Self::PositiveComplexExceptRRHorn => {
                p_select_complex_ahp_except_rr_horn(ocb, bank, clause);
            }
            Self::NewComplex => select_new_complex_ahp(ocb, bank, clause),
            Self::PositiveNewComplex => p_select_new_complex_ahp(ocb, bank, clause),
            Self::NewComplexExceptRRHorn => {
                select_new_complex_ahp_except_rr_horn(ocb, bank, clause);
            }
            Self::PositiveNewComplexExceptRRHorn => {
                p_select_new_complex_ahp_except_rr_horn(ocb, bank, clause);
            }
            Self::NewComplexExceptUniqueMaxHorn => {
                select_new_complex_ahp_except_uniq_max_horn(ocb, bank, clause);
            }
            Self::PositiveNewComplexExceptUniqueMaxHorn => {
                p_select_new_complex_ahp_except_uniq_max_horn(ocb, bank, clause);
            }
            Self::NewComplexNoSplit => select_new_complex_ahp_ns(ocb, bank, clause),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CqLiteralSelector {
    VgNonCr,
    ArEqLast,
    ArEqFirst,
    IArEqLast,
    IArEqFirst,
    Ar,
    IAr,
    ArNpEqFirst,
    IArNpEqFirst,
    GrArEqFirst,
    ArNtEqFirst,
    IArNtEqFirst,
    ArNtNpEqFirst,
    IArNtNpEqFirst,
    ArNxtEqFirst,
    IArNxtEqFirst,
    ArNtNp,
    IArNtNp,
    ArNt,
    IArNt,
    ArNp,
    IArNp,
    ArNpEqFirstUnlessPDom,
    ArNtEqFirstUnlessPDom,
    PrecW,
    IPrecW,
    PrecWNtNp,
    IPrecWNtNp,
}

impl CqLiteralSelector {
    fn from_name(name: &str) -> Option<Self> {
        match name {
            SELECT_VG_NON_CR => Some(Self::VgNonCr),
            SELECT_CQ_AR_EQ_LAST => Some(Self::ArEqLast),
            SELECT_CQ_AR_EQ_FIRST => Some(Self::ArEqFirst),
            SELECT_CQI_AR_EQ_LAST => Some(Self::IArEqLast),
            SELECT_CQI_AR_EQ_FIRST => Some(Self::IArEqFirst),
            SELECT_CQ_AR => Some(Self::Ar),
            SELECT_CQI_AR => Some(Self::IAr),
            SELECT_CQ_AR_NP_EQ_FIRST => Some(Self::ArNpEqFirst),
            SELECT_CQI_AR_NP_EQ_FIRST => Some(Self::IArNpEqFirst),
            SELECT_CQ_GR_AR_EQ_FIRST => Some(Self::GrArEqFirst),
            SELECT_CQ_AR_NT_EQ_FIRST => Some(Self::ArNtEqFirst),
            SELECT_CQI_AR_NT_EQ_FIRST => Some(Self::IArNtEqFirst),
            SELECT_CQ_AR_NT_NP_EQ_FIRST => Some(Self::ArNtNpEqFirst),
            SELECT_CQI_AR_NT_NP_EQ_FIRST => Some(Self::IArNtNpEqFirst),
            SELECT_CQ_AR_NXT_EQ_FIRST => Some(Self::ArNxtEqFirst),
            SELECT_CQI_AR_NXT_EQ_FIRST => Some(Self::IArNxtEqFirst),
            SELECT_CQ_AR_NT_NP => Some(Self::ArNtNp),
            SELECT_CQI_AR_NT_NP => Some(Self::IArNtNp),
            SELECT_CQ_AR_NT => Some(Self::ArNt),
            SELECT_CQI_AR_NT => Some(Self::IArNt),
            SELECT_CQ_AR_NP => Some(Self::ArNp),
            SELECT_CQI_AR_NP => Some(Self::IArNp),
            SELECT_CQ_AR_NP_EQ_FIRST_UNLESS_PDOM => Some(Self::ArNpEqFirstUnlessPDom),
            SELECT_CQ_AR_NT_EQ_FIRST_UNLESS_PDOM => Some(Self::ArNtEqFirstUnlessPDom),
            SELECT_CQ_PREC_W => Some(Self::PrecW),
            SELECT_CQI_PREC_W => Some(Self::IPrecW),
            SELECT_CQ_PREC_W_NT_NP => Some(Self::PrecWNtNp),
            SELECT_CQI_PREC_W_NT_NP => Some(Self::IPrecWNtNp),
            _ => None,
        }
    }

    fn apply(self, ocb: &mut OrderControlBlock, bank: &TermBank, clause: &mut Clause) {
        match self {
            Self::VgNonCr => select_vg_non_cr(ocb, bank, clause),
            Self::ArEqLast => select_cq_ar_eq_last(ocb, bank, clause),
            Self::ArEqFirst => select_cq_ar_eq_first(ocb, bank, clause),
            Self::IArEqLast => select_cqi_ar_eq_last(ocb, bank, clause),
            Self::IArEqFirst => select_cqi_ar_eq_first(ocb, bank, clause),
            Self::Ar => select_cq_ar(ocb, bank, clause),
            Self::IAr => select_cqi_ar(ocb, bank, clause),
            Self::ArNpEqFirst => select_cq_ar_np_eq_first(ocb, bank, clause),
            Self::IArNpEqFirst => select_cqi_ar_np_eq_first(ocb, bank, clause),
            Self::GrArEqFirst => select_cq_gr_ar_eq_first(ocb, bank, clause),
            Self::ArNtEqFirst => select_cq_ar_nt_eq_first(ocb, bank, clause),
            Self::IArNtEqFirst => select_cqi_ar_nt_eq_first(ocb, bank, clause),
            Self::ArNtNpEqFirst => select_cq_ar_nt_np_eq_first(ocb, bank, clause),
            Self::IArNtNpEqFirst => select_cqi_ar_nt_np_eq_first(ocb, bank, clause),
            Self::ArNxtEqFirst => select_cq_ar_nxt_eq_first(ocb, bank, clause),
            Self::IArNxtEqFirst => select_cqi_ar_nxt_eq_first(ocb, bank, clause),
            Self::ArNtNp => select_cq_ar_nt_np(ocb, bank, clause),
            Self::IArNtNp => select_cqi_ar_nt_np(ocb, bank, clause),
            Self::ArNt => select_cq_ar_nt(ocb, bank, clause),
            Self::IArNt => select_cqi_ar_nt(ocb, bank, clause),
            Self::ArNp => select_cq_ar_np(ocb, bank, clause),
            Self::IArNp => select_cqi_ar_np(ocb, bank, clause),
            Self::ArNpEqFirstUnlessPDom => {
                select_cq_ar_np_eq_first_unless_pdom(ocb, bank, clause);
            }
            Self::ArNtEqFirstUnlessPDom => {
                select_cq_ar_nt_eq_first_unless_pdom(ocb, bank, clause);
            }
            Self::PrecW => select_cq_prec_w(ocb, bank, clause),
            Self::IPrecW => select_cqi_prec_w(ocb, bank, clause),
            Self::PrecWNtNp => select_cq_prec_w_nt_np(ocb, bank, clause),
            Self::IPrecWNtNp => select_cqi_prec_w_nt_np(ocb, bank, clause),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum OrientableWeightChoice {
    Largest,
    Smallest,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ComplexMaxGate {
    UniqueMaximal,
    UniquePositiveMaximal,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum MaxLComplexTypeFilter {
    TypePred,
    XTypePred,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum GenericMaxLComplexWeight {
    Generic,
    AvoidPositivePredicate,
    AvoidAppVar,
    StronglyAvoidAppVar,
    PreferAppVar,
    AvoidPropositionalTypePredicate,
    AvoidTypePredicate,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum MinInfposPositivePolicy {
    Never,
    BeforeSelection,
    IfSelectedNonGround,
    IfSelectedGround,
    AfterSelection,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum NewComplexAhpMode {
    Standard,
    NoSplit,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CqArityPreference {
    High,
    Low,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CqFilter {
    None,
    NoPropositional,
    NoType,
    NoTypeOrPropositional,
    NoXType,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CqGroundBias {
    None,
    PreferGroundWithinSymbol,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct CqWeightSpec {
    arity: CqArityPreference,
    eq_w1: Option<i64>,
    filter: CqFilter,
    ground_bias: CqGroundBias,
    forbidden_w1: i64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct CqPrecedenceWeightSpec {
    inverted: bool,
    filter: CqFilter,
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

fn require_selector_bank<'bank>(
    name: &str,
    bank: Option<&'bank TermBank>,
) -> Result<&'bank TermBank, UnsupportedLiteralSelection> {
    bank.ok_or_else(|| UnsupportedLiteralSelection::new(name))
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

pub fn select_min_optimal_no_type_pred(
    _ocb: Option<&mut OrderControlBlock>,
    bank: &TermBank,
    clause: &mut Clause,
) {
    select_min_optimal_filtered_impl(bank, clause, MinOptimalTypeFilter::Predicate, false);
}

pub fn p_select_min_optimal_no_type_pred(
    _ocb: Option<&mut OrderControlBlock>,
    bank: &TermBank,
    clause: &mut Clause,
) {
    select_min_optimal_filtered_impl(bank, clause, MinOptimalTypeFilter::Predicate, true);
}

pub fn select_min_optimal_no_x_type_pred(
    _ocb: Option<&mut OrderControlBlock>,
    bank: &TermBank,
    clause: &mut Clause,
) {
    select_min_optimal_filtered_impl(bank, clause, MinOptimalTypeFilter::Extended, false);
}

pub fn p_select_min_optimal_no_x_type_pred(
    _ocb: Option<&mut OrderControlBlock>,
    bank: &TermBank,
    clause: &mut Clause,
) {
    select_min_optimal_filtered_impl(bank, clause, MinOptimalTypeFilter::Extended, true);
}

pub fn select_min_optimal_no_rx_type_pred(
    _ocb: Option<&mut OrderControlBlock>,
    bank: &TermBank,
    clause: &mut Clause,
) {
    select_min_optimal_filtered_impl(bank, clause, MinOptimalTypeFilter::RealExtended, false);
}

pub fn p_select_min_optimal_no_rx_type_pred(
    _ocb: Option<&mut OrderControlBlock>,
    bank: &TermBank,
    clause: &mut Clause,
) {
    select_min_optimal_filtered_impl(bank, clause, MinOptimalTypeFilter::RealExtended, true);
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

pub fn select_unless_uniq_max_smallest_orientable(
    ocb: &mut OrderControlBlock,
    bank: &TermBank,
    clause: &mut Clause,
) {
    select_unless_maximal_gate_smallest_orientable(ocb, bank, clause, false);
}

pub fn p_select_unless_uniq_max_smallest_orientable(
    ocb: &mut OrderControlBlock,
    bank: &TermBank,
    clause: &mut Clause,
) {
    select_unless_maximal_gate_smallest_orientable(ocb, bank, clause, true);
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

pub fn select_complex_except_uniq_max_horn(
    ocb: &mut OrderControlBlock,
    bank: &TermBank,
    clause: &mut Clause,
) {
    select_complex_except_max_horn_impl(ocb, bank, clause, false, ComplexMaxGate::UniqueMaximal);
}

pub fn p_select_complex_except_uniq_max_horn(
    ocb: &mut OrderControlBlock,
    bank: &TermBank,
    clause: &mut Clause,
) {
    select_complex_except_max_horn_impl(ocb, bank, clause, true, ComplexMaxGate::UniqueMaximal);
}

pub fn m_select_complex_except_uniq_max_horn(
    ocb: &mut OrderControlBlock,
    bank: &TermBank,
    clause: &mut Clause,
) {
    if clause.is_horn() {
        p_select_complex_except_uniq_max_horn(ocb, bank, clause);
    } else {
        select_complex_except_uniq_max_horn(ocb, bank, clause);
    }
}

pub fn select_complex_except_uniq_max_pos_horn(
    ocb: &mut OrderControlBlock,
    bank: &TermBank,
    clause: &mut Clause,
) {
    select_complex_except_max_horn_impl(
        ocb,
        bank,
        clause,
        false,
        ComplexMaxGate::UniquePositiveMaximal,
    );
}

pub fn p_select_complex_except_uniq_max_pos_horn(
    ocb: &mut OrderControlBlock,
    bank: &TermBank,
    clause: &mut Clause,
) {
    select_complex_except_max_horn_impl(
        ocb,
        bank,
        clause,
        true,
        ComplexMaxGate::UniquePositiveMaximal,
    );
}

pub fn select_max_l_complex(ocb: &mut OrderControlBlock, bank: &TermBank, clause: &mut Clause) {
    select_max_l_complex_impl(ocb, bank, clause, false, None);
}

pub fn p_select_max_l_complex(ocb: &mut OrderControlBlock, bank: &TermBank, clause: &mut Clause) {
    select_max_l_complex_impl(ocb, bank, clause, true, None);
}

pub fn select_max_l_complex_no_type_pred(
    ocb: &mut OrderControlBlock,
    bank: &TermBank,
    clause: &mut Clause,
) {
    select_max_l_complex_impl(
        ocb,
        bank,
        clause,
        false,
        Some(MaxLComplexTypeFilter::TypePred),
    );
}

pub fn p_select_max_l_complex_no_type_pred(
    ocb: &mut OrderControlBlock,
    bank: &TermBank,
    clause: &mut Clause,
) {
    select_max_l_complex_impl(
        ocb,
        bank,
        clause,
        true,
        Some(MaxLComplexTypeFilter::TypePred),
    );
}

pub fn select_max_l_complex_no_x_type_pred(
    ocb: &mut OrderControlBlock,
    bank: &TermBank,
    clause: &mut Clause,
) {
    select_max_l_complex_impl(
        ocb,
        bank,
        clause,
        false,
        Some(MaxLComplexTypeFilter::XTypePred),
    );
}

pub fn p_select_max_l_complex_no_x_type_pred(
    ocb: &mut OrderControlBlock,
    bank: &TermBank,
    clause: &mut Clause,
) {
    select_max_l_complex_impl(
        ocb,
        bank,
        clause,
        true,
        Some(MaxLComplexTypeFilter::XTypePred),
    );
}

pub fn select_max_l_complex_g(ocb: &mut OrderControlBlock, bank: &TermBank, clause: &mut Clause) {
    select_generic_max_l_complex_impl(ocb, bank, clause, true, GenericMaxLComplexWeight::Generic);
}

pub fn select_max_l_complex_avoid_pos_pred(
    ocb: &mut OrderControlBlock,
    bank: &TermBank,
    clause: &mut Clause,
) {
    select_generic_max_l_complex_impl(
        ocb,
        bank,
        clause,
        true,
        GenericMaxLComplexWeight::AvoidPositivePredicate,
    );
}

pub fn select_max_l_complex_app_nt_np(
    ocb: &mut OrderControlBlock,
    bank: &TermBank,
    clause: &mut Clause,
) {
    select_generic_max_l_complex_impl(
        ocb,
        bank,
        clause,
        true,
        GenericMaxLComplexWeight::AvoidPropositionalTypePredicate,
    );
}

pub fn select_max_l_complex_app_no_type(
    ocb: &mut OrderControlBlock,
    bank: &TermBank,
    clause: &mut Clause,
) {
    select_generic_max_l_complex_impl(
        ocb,
        bank,
        clause,
        true,
        GenericMaxLComplexWeight::AvoidTypePredicate,
    );
}

pub fn select_max_l_complex_avoid_pos_u_pred(
    ocb: &mut OrderControlBlock,
    bank: &TermBank,
    clause: &mut Clause,
) {
    select_generic_max_l_complex_impl(
        ocb,
        bank,
        clause,
        false,
        GenericMaxLComplexWeight::AvoidPositivePredicate,
    );
}

pub fn select_max_l_complex_avoid_app_var(
    ocb: &mut OrderControlBlock,
    bank: &TermBank,
    clause: &mut Clause,
) {
    select_generic_max_l_complex_impl(
        ocb,
        bank,
        clause,
        true,
        GenericMaxLComplexWeight::AvoidAppVar,
    );
}

pub fn select_max_l_complex_strongly_avoid_app_var(
    ocb: &mut OrderControlBlock,
    bank: &TermBank,
    clause: &mut Clause,
) {
    select_generic_max_l_complex_impl(
        ocb,
        bank,
        clause,
        true,
        GenericMaxLComplexWeight::StronglyAvoidAppVar,
    );
}

pub fn select_max_l_complex_prefer_app_var(
    ocb: &mut OrderControlBlock,
    bank: &TermBank,
    clause: &mut Clause,
) {
    select_generic_max_l_complex_impl(
        ocb,
        bank,
        clause,
        true,
        GenericMaxLComplexWeight::PreferAppVar,
    );
}

pub fn select_new_complex(ocb: &mut OrderControlBlock, bank: &TermBank, clause: &mut Clause) {
    select_new_complex_impl(ocb, bank, clause, false);
}

pub fn p_select_new_complex(ocb: &mut OrderControlBlock, bank: &TermBank, clause: &mut Clause) {
    select_new_complex_impl(ocb, bank, clause, true);
}

pub fn select_new_complex_except_uniq_max_horn(
    ocb: &mut OrderControlBlock,
    bank: &TermBank,
    clause: &mut Clause,
) {
    select_new_complex_except_uniq_max_horn_impl(ocb, bank, clause, false);
}

pub fn p_select_new_complex_except_uniq_max_horn(
    ocb: &mut OrderControlBlock,
    bank: &TermBank,
    clause: &mut Clause,
) {
    select_new_complex_except_uniq_max_horn_impl(ocb, bank, clause, true);
}

pub fn select_complex_ahp(ocb: &mut OrderControlBlock, bank: &TermBank, clause: &mut Clause) {
    select_complex_ahp_impl(ocb, bank, clause, false);
}

pub fn p_select_complex_ahp(ocb: &mut OrderControlBlock, bank: &TermBank, clause: &mut Clause) {
    select_complex_ahp_impl(ocb, bank, clause, true);
}

pub fn select_complex_ahp_except_rr_horn(
    ocb: &mut OrderControlBlock,
    bank: &TermBank,
    clause: &mut Clause,
) {
    if !(clause.is_horn() && clause.is_range_restricted()) {
        select_complex_ahp(ocb, bank, clause);
    }
}

pub fn p_select_complex_ahp_except_rr_horn(
    ocb: &mut OrderControlBlock,
    bank: &TermBank,
    clause: &mut Clause,
) {
    if !(clause.is_horn() && clause.is_range_restricted()) {
        p_select_complex_ahp(ocb, bank, clause);
    }
}

pub fn select_new_complex_ahp(ocb: &mut OrderControlBlock, bank: &TermBank, clause: &mut Clause) {
    select_new_complex_ahp_impl(ocb, bank, clause, false, NewComplexAhpMode::Standard);
}

pub fn p_select_new_complex_ahp(ocb: &mut OrderControlBlock, bank: &TermBank, clause: &mut Clause) {
    select_new_complex_ahp_impl(ocb, bank, clause, true, NewComplexAhpMode::Standard);
}

pub fn select_new_complex_ahp_except_rr_horn(
    ocb: &mut OrderControlBlock,
    bank: &TermBank,
    clause: &mut Clause,
) {
    if !(clause.is_horn() && clause.is_range_restricted()) {
        select_new_complex_ahp(ocb, bank, clause);
    }
}

pub fn p_select_new_complex_ahp_except_rr_horn(
    ocb: &mut OrderControlBlock,
    bank: &TermBank,
    clause: &mut Clause,
) {
    if !(clause.is_horn() && clause.is_range_restricted()) {
        p_select_new_complex_ahp(ocb, bank, clause);
    }
}

pub fn select_new_complex_ahp_except_uniq_max_horn(
    ocb: &mut OrderControlBlock,
    bank: &TermBank,
    clause: &mut Clause,
) {
    select_new_complex_ahp_except_uniq_max_horn_impl(ocb, bank, clause, false);
}

pub fn p_select_new_complex_ahp_except_uniq_max_horn(
    ocb: &mut OrderControlBlock,
    bank: &TermBank,
    clause: &mut Clause,
) {
    select_new_complex_ahp_except_uniq_max_horn_impl(ocb, bank, clause, true);
}

pub fn select_new_complex_ahp_ns(
    ocb: &mut OrderControlBlock,
    bank: &TermBank,
    clause: &mut Clause,
) {
    select_new_complex_ahp_impl(ocb, bank, clause, false, NewComplexAhpMode::NoSplit);
}

pub fn select_vg_non_cr(ocb: &mut OrderControlBlock, bank: &TermBank, clause: &mut Clause) {
    select_vg_non_cr_impl(ocb, bank, clause);
}

pub fn select_cq_ar_eq_last(ocb: &mut OrderControlBlock, bank: &TermBank, clause: &mut Clause) {
    select_cq_with_spec(
        ocb,
        bank,
        clause,
        cq_spec(CqArityPreference::High, Some(1_000_000), CqFilter::None),
    );
}

pub fn select_cq_ar_eq_first(ocb: &mut OrderControlBlock, bank: &TermBank, clause: &mut Clause) {
    select_cq_with_spec(
        ocb,
        bank,
        clause,
        cq_spec(CqArityPreference::High, Some(-100_000), CqFilter::None),
    );
}

pub fn select_cqi_ar_eq_last(ocb: &mut OrderControlBlock, bank: &TermBank, clause: &mut Clause) {
    select_cq_with_spec(
        ocb,
        bank,
        clause,
        cq_spec(CqArityPreference::Low, Some(100_000), CqFilter::None),
    );
}

pub fn select_cqi_ar_eq_first(ocb: &mut OrderControlBlock, bank: &TermBank, clause: &mut Clause) {
    select_cq_with_spec(
        ocb,
        bank,
        clause,
        cq_spec(CqArityPreference::Low, Some(-100_000), CqFilter::None),
    );
}

pub fn select_cq_ar(ocb: &mut OrderControlBlock, bank: &TermBank, clause: &mut Clause) {
    select_cq_with_spec(
        ocb,
        bank,
        clause,
        cq_spec(CqArityPreference::High, None, CqFilter::None),
    );
}

pub fn select_cqi_ar(ocb: &mut OrderControlBlock, bank: &TermBank, clause: &mut Clause) {
    select_cq_with_spec(
        ocb,
        bank,
        clause,
        cq_spec(CqArityPreference::Low, None, CqFilter::None),
    );
}

pub fn select_cq_ar_np_eq_first(ocb: &mut OrderControlBlock, bank: &TermBank, clause: &mut Clause) {
    select_cq_with_spec(
        ocb,
        bank,
        clause,
        cq_spec(
            CqArityPreference::High,
            Some(-100_000),
            CqFilter::NoPropositional,
        ),
    );
}

pub fn select_cqi_ar_np_eq_first(
    ocb: &mut OrderControlBlock,
    bank: &TermBank,
    clause: &mut Clause,
) {
    select_cq_with_spec(
        ocb,
        bank,
        clause,
        CqWeightSpec {
            arity: CqArityPreference::Low,
            eq_w1: Some(-1_000_000),
            filter: CqFilter::NoPropositional,
            ground_bias: CqGroundBias::None,
            forbidden_w1: 1_000_000,
        },
    );
}

pub fn select_cq_gr_ar_eq_first(ocb: &mut OrderControlBlock, bank: &TermBank, clause: &mut Clause) {
    select_cq_with_spec(
        ocb,
        bank,
        clause,
        CqWeightSpec {
            arity: CqArityPreference::High,
            eq_w1: Some(-1_000_000),
            filter: CqFilter::None,
            ground_bias: CqGroundBias::PreferGroundWithinSymbol,
            forbidden_w1: CQ_FORBIDDEN_WEIGHT,
        },
    );
}

pub fn select_cq_ar_nt_eq_first(ocb: &mut OrderControlBlock, bank: &TermBank, clause: &mut Clause) {
    select_cq_with_spec(
        ocb,
        bank,
        clause,
        cq_spec(CqArityPreference::High, Some(-100_000), CqFilter::NoType),
    );
}

pub fn select_cqi_ar_nt_eq_first(
    ocb: &mut OrderControlBlock,
    bank: &TermBank,
    clause: &mut Clause,
) {
    select_cq_with_spec(
        ocb,
        bank,
        clause,
        cq_spec(CqArityPreference::Low, Some(-100_000), CqFilter::NoType),
    );
}

pub fn select_cq_ar_nt_np_eq_first(
    ocb: &mut OrderControlBlock,
    bank: &TermBank,
    clause: &mut Clause,
) {
    select_cq_with_spec(
        ocb,
        bank,
        clause,
        cq_spec(
            CqArityPreference::High,
            Some(-100_000),
            CqFilter::NoTypeOrPropositional,
        ),
    );
}

pub fn select_cqi_ar_nt_np_eq_first(
    ocb: &mut OrderControlBlock,
    bank: &TermBank,
    clause: &mut Clause,
) {
    select_cq_with_spec(
        ocb,
        bank,
        clause,
        cq_spec(
            CqArityPreference::Low,
            Some(-100_000),
            CqFilter::NoTypeOrPropositional,
        ),
    );
}

pub fn select_cq_ar_nxt_eq_first(
    ocb: &mut OrderControlBlock,
    bank: &TermBank,
    clause: &mut Clause,
) {
    select_cq_with_spec(
        ocb,
        bank,
        clause,
        cq_spec(CqArityPreference::High, Some(-100_000), CqFilter::NoXType),
    );
}

pub fn select_cqi_ar_nxt_eq_first(
    ocb: &mut OrderControlBlock,
    bank: &TermBank,
    clause: &mut Clause,
) {
    select_cq_with_spec(
        ocb,
        bank,
        clause,
        cq_spec(CqArityPreference::Low, Some(-100_000), CqFilter::NoXType),
    );
}

pub fn select_cq_ar_nt_np(ocb: &mut OrderControlBlock, bank: &TermBank, clause: &mut Clause) {
    select_cq_with_spec(
        ocb,
        bank,
        clause,
        cq_spec(
            CqArityPreference::High,
            None,
            CqFilter::NoTypeOrPropositional,
        ),
    );
}

pub fn select_cqi_ar_nt_np(ocb: &mut OrderControlBlock, bank: &TermBank, clause: &mut Clause) {
    select_cq_with_spec(
        ocb,
        bank,
        clause,
        cq_spec(
            CqArityPreference::Low,
            None,
            CqFilter::NoTypeOrPropositional,
        ),
    );
}

pub fn select_cq_ar_nt(ocb: &mut OrderControlBlock, bank: &TermBank, clause: &mut Clause) {
    select_cq_with_spec(
        ocb,
        bank,
        clause,
        cq_spec(CqArityPreference::High, None, CqFilter::NoType),
    );
}

pub fn select_cqi_ar_nt(ocb: &mut OrderControlBlock, bank: &TermBank, clause: &mut Clause) {
    select_cq_with_spec(
        ocb,
        bank,
        clause,
        cq_spec(CqArityPreference::Low, None, CqFilter::NoType),
    );
}

pub fn select_cq_ar_np(ocb: &mut OrderControlBlock, bank: &TermBank, clause: &mut Clause) {
    select_cq_with_spec(
        ocb,
        bank,
        clause,
        cq_spec(CqArityPreference::High, None, CqFilter::NoPropositional),
    );
}

pub fn select_cqi_ar_np(ocb: &mut OrderControlBlock, bank: &TermBank, clause: &mut Clause) {
    select_cq_with_spec(
        ocb,
        bank,
        clause,
        cq_spec(CqArityPreference::Low, None, CqFilter::NoPropositional),
    );
}

pub fn select_cq_ar_np_eq_first_unless_pdom(
    ocb: &mut OrderControlBlock,
    bank: &TermBank,
    clause: &mut Clause,
) {
    select_unless_pdom(ocb, bank, clause, select_cq_ar_np_eq_first);
}

pub fn select_cq_ar_nt_eq_first_unless_pdom(
    ocb: &mut OrderControlBlock,
    bank: &TermBank,
    clause: &mut Clause,
) {
    select_unless_pdom(ocb, bank, clause, select_cq_ar_nt_eq_first);
}

pub fn select_cq_prec_w(ocb: &mut OrderControlBlock, bank: &TermBank, clause: &mut Clause) {
    select_cq_precedence_with_spec(
        ocb,
        bank,
        clause,
        CqPrecedenceWeightSpec {
            inverted: false,
            filter: CqFilter::None,
        },
    );
}

pub fn select_cqi_prec_w(ocb: &mut OrderControlBlock, bank: &TermBank, clause: &mut Clause) {
    select_cq_precedence_with_spec(
        ocb,
        bank,
        clause,
        CqPrecedenceWeightSpec {
            inverted: true,
            filter: CqFilter::None,
        },
    );
}

pub fn select_cq_prec_w_nt_np(ocb: &mut OrderControlBlock, bank: &TermBank, clause: &mut Clause) {
    select_cq_precedence_with_spec(
        ocb,
        bank,
        clause,
        CqPrecedenceWeightSpec {
            inverted: false,
            filter: CqFilter::NoTypeOrPropositional,
        },
    );
}

pub fn select_cqi_prec_w_nt_np(ocb: &mut OrderControlBlock, bank: &TermBank, clause: &mut Clause) {
    select_cq_precedence_with_spec(
        ocb,
        bank,
        clause,
        CqPrecedenceWeightSpec {
            inverted: true,
            filter: CqFilter::NoTypeOrPropositional,
        },
    );
}

pub fn select_min_infpos(ocb: &mut OrderControlBlock, bank: &TermBank, clause: &mut Clause) {
    select_min_infpos_impl(ocb, bank, clause, MinInfposPositivePolicy::Never, false);
}

pub fn p_select_min_infpos(ocb: &mut OrderControlBlock, bank: &TermBank, clause: &mut Clause) {
    select_min_infpos_impl(
        ocb,
        bank,
        clause,
        MinInfposPositivePolicy::BeforeSelection,
        false,
    );
}

pub fn h_select_min_infpos(ocb: &mut OrderControlBlock, bank: &TermBank, clause: &mut Clause) {
    select_min_infpos_impl(
        ocb,
        bank,
        clause,
        MinInfposPositivePolicy::IfSelectedNonGround,
        false,
    );
}

pub fn g_select_min_infpos(ocb: &mut OrderControlBlock, bank: &TermBank, clause: &mut Clause) {
    select_min_infpos_impl(
        ocb,
        bank,
        clause,
        MinInfposPositivePolicy::IfSelectedGround,
        false,
    );
}

pub fn select_min_infpos_no_type_pred(
    ocb: &mut OrderControlBlock,
    bank: &TermBank,
    clause: &mut Clause,
) {
    select_min_infpos_impl(ocb, bank, clause, MinInfposPositivePolicy::Never, true);
}

pub fn p_select_min_infpos_no_type_pred(
    ocb: &mut OrderControlBlock,
    bank: &TermBank,
    clause: &mut Clause,
) {
    select_min_infpos_impl(
        ocb,
        bank,
        clause,
        MinInfposPositivePolicy::AfterSelection,
        true,
    );
}

pub fn select_min2_infpos(ocb: &mut OrderControlBlock, bank: &TermBank, clause: &mut Clause) {
    select_min_infpos_weighted_impl(
        ocb,
        bank,
        clause,
        MinInfposPositivePolicy::Never,
        false,
        2,
        1,
    );
}

pub fn p_select_min2_infpos(ocb: &mut OrderControlBlock, bank: &TermBank, clause: &mut Clause) {
    select_min_infpos_weighted_impl(
        ocb,
        bank,
        clause,
        MinInfposPositivePolicy::BeforeSelection,
        false,
        2,
        1,
    );
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
    } else if let Some(selector) = MinOptimalTypeSelector::from_name(name) {
        let bank = require_selector_bank(name, bank)?;
        selector.apply(ocb, bank, clause);
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
    } else if let Some(selector) = MaximalComplexSelector::from_name(name) {
        let Some(ocb) = ocb else {
            return Err(UnsupportedLiteralSelection::new(name));
        };
        let Some(bank) = bank else {
            return Err(UnsupportedLiteralSelection::new(name));
        };
        selector.apply(ocb, bank, clause);
        Ok(())
    } else if let Some(selector) = MaxLComplexSelector::from_name(name) {
        let Some(ocb) = ocb else {
            return Err(UnsupportedLiteralSelection::new(name));
        };
        let Some(bank) = bank else {
            return Err(UnsupportedLiteralSelection::new(name));
        };
        selector.apply(ocb, bank, clause);
        Ok(())
    } else if let Some(selector) = GenericMaxLComplexSelector::from_name(name) {
        let Some(ocb) = ocb else {
            return Err(UnsupportedLiteralSelection::new(name));
        };
        let Some(bank) = bank else {
            return Err(UnsupportedLiteralSelection::new(name));
        };
        selector.apply(ocb, bank, clause);
        Ok(())
    } else if let Some(selector) = NewComplexSelector::from_name(name) {
        let Some(ocb) = ocb else {
            return Err(UnsupportedLiteralSelection::new(name));
        };
        let Some(bank) = bank else {
            return Err(UnsupportedLiteralSelection::new(name));
        };
        selector.apply(ocb, bank, clause);
        Ok(())
    } else if let Some(selector) = MinInfposSelector::from_name(name) {
        let Some(ocb) = ocb else {
            return Err(UnsupportedLiteralSelection::new(name));
        };
        let Some(bank) = bank else {
            return Err(UnsupportedLiteralSelection::new(name));
        };
        selector.apply(ocb, bank, clause);
        Ok(())
    } else if let Some(selector) = AhpLiteralSelector::from_name(name) {
        let Some(ocb) = ocb else {
            return Err(UnsupportedLiteralSelection::new(name));
        };
        let Some(bank) = bank else {
            return Err(UnsupportedLiteralSelection::new(name));
        };
        selector.apply(ocb, bank, clause);
        Ok(())
    } else if let Some(selector) = CqLiteralSelector::from_name(name) {
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

fn select_complex_except_max_horn_impl(
    ocb: &mut OrderControlBlock,
    bank: &TermBank,
    clause: &mut Clause,
    positive_variant: bool,
    gate: ComplexMaxGate,
) {
    if clause.is_horn() {
        clause.cond_mark_maximal_terms(ocb, bank);
        if complex_max_horn_gate_blocks(clause, gate) {
            return;
        }
    }

    if positive_variant {
        p_select_complex(Some(ocb), clause);
    } else {
        select_complex(Some(ocb), clause);
    }
    clause.del_prop(CP_IS_ORIENTED);
}

fn complex_max_horn_gate_blocks(clause: &Clause, gate: ComplexMaxGate) -> bool {
    let maximal = clause.literals().query_prop_number(EP_IS_MAXIMAL);
    match gate {
        ComplexMaxGate::UniqueMaximal => maximal == 1,
        ComplexMaxGate::UniquePositiveMaximal => {
            maximal == 1
                && clause
                    .literals()
                    .query_prop_number(EP_IS_MAXIMAL | EP_IS_POSITIVE)
                    == 1
        }
    }
}

fn select_max_l_complex_impl(
    ocb: &mut OrderControlBlock,
    bank: &TermBank,
    clause: &mut Clause,
    positive_variant: bool,
    type_filter: Option<MaxLComplexTypeFilter>,
) {
    clause.cond_mark_maximal_terms(ocb, bank);

    let maximal_count = clause.literals().query_prop_number(EP_IS_MAXIMAL);
    if maximal_count <= 1 {
        return;
    }

    clause.del_prop(CP_IS_ORIENTED);
    if type_filter.is_none()
        && clause
            .literals()
            .query_prop_number(EP_IS_MAXIMAL | EP_IS_POSITIVE)
            == maximal_count
    {
        if positive_variant {
            p_select_l_complex(Some(ocb), clause);
        } else {
            select_l_complex(Some(ocb), clause);
        }
        return;
    }

    let has_negative_maximal = clause
        .literals()
        .query_prop_number(EP_IS_MAXIMAL | EP_IS_POSITIVE)
        != maximal_count;
    let mut selected = if has_negative_maximal {
        find_max_lcomplex_literal(clause)
    } else {
        None
    };

    if selected
        .is_some_and(|index| max_lcomplex_type_filter_rejects(clause, bank, index, type_filter))
    {
        selected = None;
    }
    if selected.is_none() && type_filter.is_some() {
        selected = find_lcomplex_literal(clause);
        if selected
            .is_some_and(|index| max_lcomplex_type_filter_rejects(clause, bank, index, type_filter))
        {
            selected = None;
        }
    }

    if let Some(index) = selected {
        if positive_variant && type_filter.is_some() {
            select_positive_literals(clause);
        }
        clause.literals_mut().as_mut_slice()[index].set_prop(EP_IS_SELECTED);
    }
}

fn max_lcomplex_type_filter_rejects(
    clause: &Clause,
    bank: &TermBank,
    index: usize,
    type_filter: Option<MaxLComplexTypeFilter>,
) -> bool {
    let literal = &clause.literals().as_slice()[index];
    match type_filter {
        None => false,
        Some(MaxLComplexTypeFilter::TypePred) => literal.is_type_pred(bank),
        Some(MaxLComplexTypeFilter::XTypePred) => literal.is_x_type_pred(bank),
    }
}

fn find_max_lcomplex_literal(clause: &Clause) -> Option<usize> {
    find_lcomplex_literal_by_maximality(clause, true)
}

fn find_lcomplex_literal(clause: &Clause) -> Option<usize> {
    find_lcomplex_literal_by_maximality(clause, false)
}

fn find_lcomplex_literal_by_maximality(clause: &Clause, maximal: bool) -> Option<usize> {
    clause
        .literals()
        .as_slice()
        .iter()
        .position(|literal| {
            literal.is_negative() && literal.is_maximal() == maximal && literal.is_pure_var()
        })
        .or_else(|| find_largest_diff_lcomplex_literal(clause, maximal, true))
        .or_else(|| find_largest_diff_lcomplex_literal(clause, maximal, false))
}

fn find_largest_diff_lcomplex_literal(
    clause: &Clause,
    maximal: bool,
    ground_only: bool,
) -> Option<usize> {
    let mut selected = None;
    let mut select_weight = -1;

    for (index, literal) in clause.literals().as_slice().iter().enumerate() {
        if literal.is_negative()
            && literal.is_maximal() == maximal
            && (!ground_only || literal.is_ground())
        {
            let weight = literal_selection_diff_weight(literal);
            if weight > select_weight {
                select_weight = weight;
                selected = Some(index);
            }
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

fn select_unless_maximal_gate_smallest_orientable(
    ocb: &mut OrderControlBlock,
    bank: &TermBank,
    clause: &mut Clause,
    positive_variant: bool,
) {
    clause.cond_mark_maximal_terms(ocb, bank);

    if maximal_gate_allows_selection(clause, MaximalGate::MoreThanOne) {
        if positive_variant {
            p_select_smallest_orientable_literal(ocb, bank, clause);
        } else {
            select_smallest_orientable_literal(ocb, bank, clause);
        }
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

fn generic_uniq_selection_with_ordering(
    ocb: &mut OrderControlBlock,
    bank: &TermBank,
    clause: &mut Clause,
    positive: bool,
    mut weight_fun: impl FnMut(&mut LitEval, &Eqn, &Clause),
) {
    debug_assert_ne!(clause.negative_literal_count(), 0);
    debug_assert_eq!(clause.prop_lit_number(EP_IS_SELECTED), 0);

    clause.cond_mark_maximal_terms(ocb, bank);

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

#[cfg(test)]
fn literal_weight_counter_test_guard() -> MutexGuard<'static, ()> {
    LITERAL_WEIGHT_COUNTER_TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
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

fn select_new_complex_impl(
    ocb: &mut OrderControlBlock,
    bank: &TermBank,
    clause: &mut Clause,
    positive_variant: bool,
) {
    clause.cond_mark_maximal_terms(ocb, bank);

    let selected = find_smallest_max_negative_ground_literal(clause)
        .or_else(|| find_non_ground_min11_infpos_no_x_type_literal(clause, bank))
        .or_else(|| find_max_x_type_no_type_literal(clause, bank));

    if let Some(index) = selected {
        clause.literals_mut().as_mut_slice()[index].set_prop(EP_IS_SELECTED);
        clause.del_prop(CP_IS_ORIENTED);
        if positive_variant {
            select_positive_literals(clause);
        }
    }
}

fn select_new_complex_except_uniq_max_horn_impl(
    ocb: &mut OrderControlBlock,
    bank: &TermBank,
    clause: &mut Clause,
    positive_variant: bool,
) {
    if clause.is_horn() {
        clause.cond_mark_maximal_terms(ocb, bank);
        if clause.literals().query_prop_number(EP_IS_MAXIMAL) == 1 {
            return;
        }
    }

    select_new_complex_impl(ocb, bank, clause, positive_variant);
}

fn find_smallest_max_negative_ground_literal(clause: &Clause) -> Option<usize> {
    let mut selected = None;
    let mut select_weight = i64::MAX;

    for (index, literal) in clause.literals().as_slice().iter().enumerate() {
        if literal.is_negative() && literal.is_ground() {
            let weight = literal.left().weight();
            if weight < select_weight {
                select_weight = weight;
                selected = Some(index);
            }
        }
    }

    selected
}

fn find_non_ground_min11_infpos_no_x_type_literal(
    clause: &Clause,
    bank: &TermBank,
) -> Option<usize> {
    let mut selected = None;
    let mut select_weight = i64::MAX;

    for (index, literal) in clause.literals().as_slice().iter().enumerate() {
        if literal.is_negative() && !literal.is_ground() && !literal.is_x_type_pred(bank) {
            let mut weight = term_weight_compute(literal.left(), 1, 1);
            if !literal.is_oriented() {
                weight += term_weight_compute(literal.right(), 1, 1);
            }
            if weight < select_weight {
                select_weight = weight;
                selected = Some(index);
            }
        }
    }

    selected
}

fn find_max_x_type_no_type_literal(clause: &Clause, bank: &TermBank) -> Option<usize> {
    let mut selected = None;
    let mut select_weight = -1;

    for (index, literal) in clause.literals().as_slice().iter().enumerate() {
        if literal.is_negative() && literal.is_x_type_pred(bank) && !literal.is_type_pred(bank) {
            let weight = literal.left().weight();
            if weight > select_weight {
                select_weight = weight;
                selected = Some(index);
            }
        }
    }

    selected
}

fn select_generic_max_l_complex_impl(
    ocb: &mut OrderControlBlock,
    bank: &TermBank,
    clause: &mut Clause,
    return_without_negatives: bool,
    weight: GenericMaxLComplexWeight,
) {
    debug_assert_eq!(clause.prop_lit_number(EP_IS_SELECTED), 0);

    if return_without_negatives && clause.negative_literal_count() == 0 {
        return;
    }

    clause.cond_mark_maximal_terms(ocb, bank);
    if clause.literals().query_prop_number(EP_IS_MAXIMAL) <= 1 {
        return;
    }

    let mut pred_dist = if weight == GenericMaxLComplexWeight::Generic {
        BTreeMap::new()
    } else {
        positive_predicate_distribution(clause, bank)
    };
    if !return_without_negatives && weight == GenericMaxLComplexWeight::AvoidPositivePredicate {
        pred_dist.insert(0, 0);
    }

    generic_uniq_selection_with_ordering(ocb, bank, clause, false, |eval, literal, clause| {
        generic_max_lcomplex_weight(eval, literal, clause, bank, &pred_dist, weight);
    });
}

fn generic_max_lcomplex_weight(
    eval: &mut LitEval,
    literal: &Eqn,
    clause: &Clause,
    bank: &TermBank,
    pred_dist: &BTreeMap<i64, i64>,
    weight: GenericMaxLComplexWeight,
) {
    match weight {
        GenericMaxLComplexWeight::Generic => {
            let counter = next_literal_weight_counter();
            if literal.is_negative() {
                max_lcomplex_base_weight(eval, literal);
                eval.w3 = counter % negative_literal_count_i64(clause);
            }
        }
        GenericMaxLComplexWeight::AvoidPositivePredicate => {
            max_lcomplex_avoid_pred_weight(eval, literal, bank, pred_dist);
        }
        GenericMaxLComplexWeight::AvoidAppVar
        | GenericMaxLComplexWeight::StronglyAvoidAppVar
        | GenericMaxLComplexWeight::PreferAppVar => {
            max_lcomplex_app_var_weight(eval, literal, bank, pred_dist, weight);
        }
        GenericMaxLComplexWeight::AvoidPropositionalTypePredicate => {
            max_lcomplex_app_nt_np_weight(eval, literal, bank, pred_dist);
        }
        GenericMaxLComplexWeight::AvoidTypePredicate => {
            max_lcomplex_avoid_pred_weight(eval, literal, bank, pred_dist);
            if literal.is_type_pred(bank) {
                eval.forbidden = true;
            }
        }
    }
}

fn max_lcomplex_base_weight(eval: &mut LitEval, literal: &Eqn) {
    eval.w1 = if literal.is_maximal() { 0 } else { 100 };
    if !literal.is_pure_var() {
        eval.w1 += 10;
    }
    if !literal.is_ground() {
        eval.w1 += 1;
    }
    eval.w2 = -literal_selection_diff_weight(literal);
}

fn max_lcomplex_avoid_pred_weight(
    eval: &mut LitEval,
    literal: &Eqn,
    bank: &TermBank,
    pred_dist: &BTreeMap<i64, i64>,
) {
    if literal.is_negative() {
        max_lcomplex_base_weight(eval, literal);
        let f_code = if literal.is_equ_lit(bank)
            || literal.left().is_any_var()
            || literal.left().is_phony_app()
        {
            0
        } else {
            literal.left().f_code()
        };
        eval.w3 = pred_dist_value(pred_dist, f_code);
    }
}

fn max_lcomplex_app_var_weight(
    eval: &mut LitEval,
    literal: &Eqn,
    bank: &TermBank,
    pred_dist: &BTreeMap<i64, i64>,
    weight: GenericMaxLComplexWeight,
) {
    if literal.is_negative() {
        max_lcomplex_base_weight(eval, literal);
        match weight {
            GenericMaxLComplexWeight::AvoidAppVar if literal.has_app_var() => {
                eval.w1 += 20;
            }
            GenericMaxLComplexWeight::StronglyAvoidAppVar if literal.has_app_var() => {
                eval.w1 += 200;
            }
            GenericMaxLComplexWeight::PreferAppVar if !literal.has_app_var() => {
                eval.w1 += 200;
            }
            _ => {}
        }
        let f_code = if literal.is_equ_lit(bank) {
            0
        } else {
            literal.left().f_code()
        };
        eval.w3 = pred_dist_value(pred_dist, f_code);
    }
}

fn max_lcomplex_app_nt_np_weight(
    eval: &mut LitEval,
    literal: &Eqn,
    bank: &TermBank,
    pred_dist: &BTreeMap<i64, i64>,
) {
    if literal.is_negative() {
        if literal.is_type_pred(bank) || literal.is_propositional(bank) {
            eval.w1 = 100_000;
            eval.forbidden = true;
        } else {
            eval.w1 = if literal.is_maximal() { 0 } else { 100 };
        }
        if !literal.is_pure_var() {
            eval.w1 += 10;
        }
        if !literal.is_ground() {
            eval.w1 += 1;
        }
        eval.w2 = -literal_selection_diff_weight(literal);
        let f_code = if literal.is_equ_lit(bank) {
            0
        } else {
            literal.left().f_code()
        };
        eval.w3 = pred_dist_value(pred_dist, f_code);
    }
}

fn select_complex_ahp_impl(
    ocb: &mut OrderControlBlock,
    bank: &TermBank,
    clause: &mut Clause,
    positive_variant: bool,
) {
    let pred_dist = positive_predicate_distribution(clause, bank);
    generic_uniq_selection_with_ordering(
        ocb,
        bank,
        clause,
        positive_variant,
        |eval, literal, _| {
            complex_ahp_weight(eval, literal, &pred_dist);
        },
    );
}

fn select_new_complex_ahp_impl(
    ocb: &mut OrderControlBlock,
    bank: &TermBank,
    clause: &mut Clause,
    positive_variant: bool,
    mode: NewComplexAhpMode,
) {
    let pred_dist = positive_predicate_distribution(clause, bank);
    generic_uniq_selection_with_ordering(
        ocb,
        bank,
        clause,
        positive_variant,
        |eval, literal, _| {
            new_complex_ahp_weight(eval, literal, bank, &pred_dist, mode);
        },
    );
}

fn select_new_complex_ahp_except_uniq_max_horn_impl(
    ocb: &mut OrderControlBlock,
    bank: &TermBank,
    clause: &mut Clause,
    positive_variant: bool,
) {
    if clause.is_horn() {
        clause.cond_mark_maximal_terms(ocb, bank);
        if clause.literals().query_prop_number(EP_IS_MAXIMAL) == 1 {
            return;
        }
    }

    select_new_complex_ahp_impl(
        ocb,
        bank,
        clause,
        positive_variant,
        NewComplexAhpMode::Standard,
    );
}

const fn cq_spec(arity: CqArityPreference, eq_w1: Option<i64>, filter: CqFilter) -> CqWeightSpec {
    CqWeightSpec {
        arity,
        eq_w1,
        filter,
        ground_bias: CqGroundBias::None,
        forbidden_w1: CQ_FORBIDDEN_WEIGHT,
    }
}

fn select_vg_non_cr_impl(ocb: &mut OrderControlBlock, bank: &TermBank, clause: &mut Clause) {
    debug_assert_ne!(clause.negative_literal_count(), 0);
    debug_assert_eq!(clause.prop_lit_number(EP_IS_SELECTED), 0);

    if let Some(index) = clause.literals().find_neg_pure_var_lit_index() {
        clause.literals_mut().as_mut_slice()[index].set_prop(EP_IS_SELECTED);
        return;
    }

    clause.cond_mark_maximal_terms(ocb, bank);
    if let Some(index) = find_min_weight_negative_literal(clause, true) {
        clause.literals_mut().as_mut_slice()[index].set_prop(EP_IS_SELECTED);
        return;
    }

    if clause.literals().query_prop_number(EP_IS_MAXIMAL) == 1
        && clause
            .literals()
            .query_prop_number(EP_IS_MAXIMAL | EP_IS_POSITIVE)
            == 1
    {
        return;
    }

    select_new_complex_ahp_impl(ocb, bank, clause, false, NewComplexAhpMode::NoSplit);
}

fn select_cq_with_spec(
    ocb: &mut OrderControlBlock,
    bank: &TermBank,
    clause: &mut Clause,
    spec: CqWeightSpec,
) {
    generic_uniq_selection_with_ordering(ocb, bank, clause, false, |eval, literal, _| {
        cq_weight(eval, literal, bank, spec);
    });
}

fn select_cq_precedence_with_spec(
    ocb: &mut OrderControlBlock,
    bank: &TermBank,
    clause: &mut Clause,
    spec: CqPrecedenceWeightSpec,
) {
    let sig_size = ocb.sig_size;
    let prec_weights = ocb.prec_weights.clone();
    generic_uniq_selection_with_ordering(ocb, bank, clause, false, |eval, literal, _| {
        cq_precedence_weight(eval, literal, sig_size, prec_weights.as_deref(), bank, spec);
    });
}

fn cq_weight(eval: &mut LitEval, literal: &Eqn, bank: &TermBank, spec: CqWeightSpec) {
    if !literal.is_negative() {
        return;
    }

    if literal.is_equ_lit(bank) || literal.left().is_free_var() {
        if let Some(eq_w1) = spec.eq_w1 {
            eval.w1 = eq_w1;
            eval.w2 = 0;
        } else {
            eval.w1 = match spec.arity {
                CqArityPreference::High => -2,
                CqArityPreference::Low => 2,
            };
            eval.w2 = cq_alpha_rank_for_left(bank, literal);
        }
    } else {
        let f_code = literal.left().f_code();
        eval.w1 = match spec.arity {
            CqArityPreference::High => -cq_symbol_arity(bank, f_code),
            CqArityPreference::Low => cq_symbol_arity(bank, f_code),
        };
        eval.w2 = cq_alpha_rank(bank, f_code);
        if cq_filter_rejects(literal, bank, spec.filter) {
            eval.w1 = spec.forbidden_w1;
            eval.forbidden = true;
        }
    }

    eval.w3 = literal_selection_diff_weight(literal);
    if spec.ground_bias == CqGroundBias::PreferGroundWithinSymbol && literal.is_ground() {
        eval.w2 -= CQ_GROUND_BIAS;
    }
}

fn cq_precedence_weight(
    eval: &mut LitEval,
    literal: &Eqn,
    sig_size: i64,
    prec_weights: Option<&[i64]>,
    bank: &TermBank,
    spec: CqPrecedenceWeightSpec,
) {
    if !literal.is_negative() {
        return;
    }

    if literal.left().is_free_var() {
        eval.w1 = 0;
        eval.w2 = 0;
    } else if cq_filter_rejects(literal, bank, spec.filter) {
        eval.w1 = CQ_FORBIDDEN_WEIGHT;
        eval.forbidden = true;
    } else {
        let f_code = literal.left().f_code();
        eval.w1 = cq_fun_prec_weight(sig_size, prec_weights, f_code);
        if spec.inverted {
            eval.w1 = -eval.w1;
        }
        eval.w2 = cq_alpha_rank(bank, f_code);
    }
    eval.w3 = literal_selection_diff_weight(literal);
}

fn cq_fun_prec_weight(sig_size: i64, prec_weights: Option<&[i64]>, symbol: i64) -> i64 {
    if symbol <= sig_size {
        if let Some(weight) = prec_weights.and_then(|weights| {
            usize::try_from(symbol)
                .ok()
                .and_then(|index| weights.get(index))
        }) {
            return *weight;
        }
    }
    -symbol
}

fn cq_symbol_arity(bank: &TermBank, f_code: i64) -> i64 {
    i64::from(bank.signature().find_arity(f_code).unwrap_or(0))
}

fn cq_alpha_rank(bank: &TermBank, f_code: i64) -> i64 {
    i64::from(bank.signature().alpha_rank(f_code))
}

fn cq_alpha_rank_for_left(bank: &TermBank, literal: &Eqn) -> i64 {
    let f_code = literal.left().f_code();
    if f_code > 0 {
        cq_alpha_rank(bank, f_code)
    } else {
        0
    }
}

fn cq_filter_rejects(literal: &Eqn, bank: &TermBank, filter: CqFilter) -> bool {
    match filter {
        CqFilter::None => false,
        CqFilter::NoPropositional => literal.is_propositional(bank),
        CqFilter::NoType => literal.is_type_pred(bank),
        CqFilter::NoTypeOrPropositional => {
            literal.is_type_pred(bank) || literal.is_propositional(bank)
        }
        CqFilter::NoXType => literal.is_x_type_pred(bank),
    }
}

fn select_unless_pdom(
    ocb: &mut OrderControlBlock,
    bank: &TermBank,
    clause: &mut Clause,
    selector: fn(&mut OrderControlBlock, &TermBank, &mut Clause),
) {
    debug_assert_ne!(clause.negative_literal_count(), 0);
    debug_assert_eq!(clause.prop_lit_number(EP_IS_SELECTED), 0);

    clause.cond_mark_maximal_terms(ocb, bank);

    let pos_max_predicates = clause
        .literals()
        .as_slice()
        .iter()
        .filter(|literal| literal.is_positive() && literal.is_maximal())
        .map(|literal| literal.pred_code_fo(bank))
        .collect::<BTreeSet<_>>();

    if clause.literals().as_slice().iter().any(|literal| {
        literal.is_negative() && pos_max_predicates.contains(&literal.pred_code_fo(bank))
    }) {
        return;
    }

    selector(ocb, bank, clause);
}

fn positive_predicate_distribution(clause: &Clause, bank: &TermBank) -> BTreeMap<i64, i64> {
    let mut pred_dist = BTreeMap::new();
    for literal in clause
        .literals()
        .as_slice()
        .iter()
        .take_while(|literal| literal.is_positive())
    {
        *pred_dist.entry(literal.pred_code_fo(bank)).or_insert(0) += 1;
    }
    pred_dist
}

fn pred_dist_value(pred_dist: &BTreeMap<i64, i64>, f_code: i64) -> i64 {
    pred_dist.get(&f_code).copied().unwrap_or(0)
}

fn complex_ahp_weight(eval: &mut LitEval, literal: &Eqn, pred_dist: &BTreeMap<i64, i64>) {
    if literal.is_negative() {
        if literal.is_pure_var() {
            eval.w1 = 0;
        } else if literal.is_ground() {
            eval.w1 = 10;
            eval.w2 = literal.standard_weight();
        } else {
            eval.w1 = 20;
            eval.w2 = -literal_selection_diff_weight(literal);
        }
    }
    if literal.left().f_code() > 0 {
        eval.w3 = pred_dist_value(pred_dist, literal.left().f_code());
    }
}

fn new_complex_ahp_weight(
    eval: &mut LitEval,
    literal: &Eqn,
    bank: &TermBank,
    pred_dist: &BTreeMap<i64, i64>,
    mode: NewComplexAhpMode,
) {
    if literal.is_negative() {
        if mode == NewComplexAhpMode::NoSplit && literal.is_split_lit(bank) {
            eval.w1 = 100_000;
            eval.forbidden = true;
        } else if literal.is_ground() {
            eval.w1 = 0;
            eval.w2 = term_standard_weight(literal.left());
        } else if !literal.is_x_type_pred(bank) {
            eval.w1 = 10;
            eval.w2 = literal.max_term_positions();
        } else if !literal.is_type_pred(bank) {
            eval.w1 = 20;
            eval.w2 = -term_standard_weight(literal.left());
        } else {
            eval.w1 = 100_000;
            eval.forbidden = true;
        }
    }
    if !literal.left().is_free_var() {
        eval.w3 = pred_dist_value(pred_dist, literal.left().f_code());
    }
}

fn select_min_infpos_impl(
    ocb: &mut OrderControlBlock,
    bank: &TermBank,
    clause: &mut Clause,
    positive_policy: MinInfposPositivePolicy,
    no_type_pred: bool,
) {
    select_min_infpos_weighted_impl(ocb, bank, clause, positive_policy, no_type_pred, 1, 1);
}

fn select_min_infpos_weighted_impl(
    ocb: &mut OrderControlBlock,
    bank: &TermBank,
    clause: &mut Clause,
    positive_policy: MinInfposPositivePolicy,
    no_type_pred: bool,
    vweight: i64,
    fweight: i64,
) {
    clause.cond_mark_maximal_terms(ocb, bank);

    if matches!(positive_policy, MinInfposPositivePolicy::BeforeSelection) {
        select_positive_literals(clause);
    }

    let selected = find_min_infpos_negative_literal(clause, bank, no_type_pred, vweight, fweight);
    debug_assert!(
        no_type_pred || selected.is_some(),
        "literal-selection wrapper guarantees a negative literal"
    );

    if let Some(index) = selected {
        let selected_is_ground = clause.literals().as_slice()[index].is_ground();
        clause.literals_mut().as_mut_slice()[index].set_prop(EP_IS_SELECTED);

        let select_positives = match positive_policy {
            MinInfposPositivePolicy::Never | MinInfposPositivePolicy::BeforeSelection => false,
            MinInfposPositivePolicy::AfterSelection => true,
            MinInfposPositivePolicy::IfSelectedNonGround => !selected_is_ground,
            MinInfposPositivePolicy::IfSelectedGround => selected_is_ground,
        };
        if select_positives {
            select_positive_literals(clause);
        }
        clause.del_prop(CP_IS_ORIENTED);
    }
}

fn find_min_infpos_negative_literal(
    clause: &Clause,
    bank: &TermBank,
    no_type_pred: bool,
    vweight: i64,
    fweight: i64,
) -> Option<usize> {
    let mut selected = None;
    let mut select_weight = i64::MAX;

    for (index, literal) in clause.literals().as_slice().iter().enumerate() {
        if literal.is_negative() && (!no_type_pred || !literal.is_type_pred(bank)) {
            let weight = min_infpos_weight(literal, vweight, fweight);
            if weight < select_weight {
                select_weight = weight;
                selected = Some(index);
            }
        }
    }

    selected
}

fn min_infpos_weight(literal: &Eqn, vweight: i64, fweight: i64) -> i64 {
    let mut weight = term_weight_compute(literal.left(), vweight, fweight);
    if !literal.is_oriented() {
        weight += term_weight_compute(literal.right(), vweight, fweight);
    }
    weight
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

fn select_min_optimal_filtered_impl(
    bank: &TermBank,
    clause: &mut Clause,
    filter: MinOptimalTypeFilter,
    select_positive: bool,
) {
    let selected = find_min_weight_negative_literal(clause, true)
        .or_else(|| find_min_weight_negative_literal_filtered(clause, bank, filter));

    if let Some(index) = selected {
        clause.literals_mut().as_mut_slice()[index].set_prop(EP_IS_SELECTED);
        if select_positive {
            select_positive_literals(clause);
        }
        clause.del_prop(CP_IS_ORIENTED);
    }
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

fn find_min_weight_negative_literal_filtered(
    clause: &Clause,
    bank: &TermBank,
    filter: MinOptimalTypeFilter,
) -> Option<usize> {
    let mut selected = None;
    let mut select_weight = i64::MAX;

    for (index, literal) in clause.literals().as_slice().iter().enumerate() {
        if literal.is_negative() && !min_optimal_filter_rejects(literal, bank, filter) {
            let weight = literal.standard_weight();
            if weight < select_weight {
                select_weight = weight;
                selected = Some(index);
            }
        }
    }

    selected
}

fn min_optimal_filter_rejects(
    literal: &Eqn,
    bank: &TermBank,
    filter: MinOptimalTypeFilter,
) -> bool {
    match filter {
        MinOptimalTypeFilter::Predicate => literal.is_type_pred(bank),
        MinOptimalTypeFilter::Extended => literal.is_x_type_pred(bank),
        MinOptimalTypeFilter::RealExtended => literal.is_real_x_type_pred(bank),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        apply_ported_literal_selector, apply_ported_literal_selector_with_bank,
        literal_weight_counter_test_guard, m_select_largest_orientable_literal,
        m_select_smallest_orientable_literal, p_select_all_cond_optimal_literal, p_select_complex,
        p_select_complex_except_rr_horn, p_select_complex_prefer_eq, p_select_complex_prefer_neq,
        p_select_cond_optimal_literal, p_select_depth2_optimal_literal,
        p_select_diff_negative_literal, p_select_first_variable_literal,
        p_select_ground_negative_literal, p_select_l_complex, p_select_largest_negative_literal,
        p_select_largest_orientable_literal, p_select_min2_infpos, p_select_min_optimal_literal,
        p_select_negative_literals, p_select_optimal_literal, p_select_smallest_negative_literal,
        p_select_smallest_orientable_literal, p_select_strong_rr_non_rr_optimal_literal,
        p_select_unless_uniq_max_optimal_literal, p_select_unless_uniq_max_smallest_orientable,
        reset_literal_weight_counter_for_tests, select_all_cond_optimal_literal,
        select_anti_rr_optimal_literal, select_complex, select_complex_except_rr_horn,
        select_complex_prefer_eq, select_complex_prefer_neq, select_cond_optimal_literal,
        select_depth2_optimal_literal, select_diff_negative_literal,
        select_diversification_literals, select_diversification_prefer_into_literals,
        select_first_variable_literal, select_ground_negative_literal, select_l_complex,
        select_largest_negative_literal, select_largest_orientable_literal, select_min2_infpos,
        select_min_optimal_literal, select_n_depth2_optimal_literal, select_negative_literals,
        select_non_anti_rr_optimal_literal, select_non_rr_optimal_literal,
        select_non_strong_rr_optimal_literal, select_optimal_literal,
        select_p_depth2_optimal_literal, select_smallest_negative_literal,
        select_smallest_orientable_literal, select_strong_rr_non_rr_optimal_literal,
        select_unless_pos_max_optimal_literal, select_unless_uniq_max_optimal_literal,
        select_unless_uniq_max_pos_optimal_literal, select_unless_uniq_max_smallest_orientable,
        select_unless_uniq_pos_max_optimal_literal, M_SELECT_LARGEST_ORIENTABLE,
        M_SELECT_SMALLEST_ORIENTABLE, NO_GENERATION, NO_SELECTION, P_SELECT_ALL_COND_OPTIMAL_LIT,
        P_SELECT_ANTI_RR_OPTIMAL_LIT, P_SELECT_COMPLEX, P_SELECT_COMPLEX_EXCEPT_RR_HORN,
        P_SELECT_COMPLEX_PREFER_EQ, P_SELECT_COMPLEX_PREFER_NEQ, P_SELECT_COND_OPTIMAL_LIT,
        P_SELECT_DIFF_NEG_LIT, P_SELECT_GROUND_NEG_LIT, P_SELECT_LARGEST_NEG_LIT,
        P_SELECT_LARGEST_ORIENTABLE, P_SELECT_L_COMPLEX, P_SELECT_MIN_OPTIMAL_LIT,
        P_SELECT_NEGATIVE_LITERALS, P_SELECT_NON_ANTI_RR_OPTIMAL_LIT, P_SELECT_NON_RR_OPTIMAL_LIT,
        P_SELECT_NON_STRONG_RR_OPTIMAL_LIT, P_SELECT_OPTIMAL_LIT, P_SELECT_OPTIMAL_RESTR_DEPTH2,
        P_SELECT_OPTIMAL_RESTR_N_DEPTH2, P_SELECT_OPTIMAL_RESTR_P_DEPTH2,
        P_SELECT_PURE_VAR_NEG_LITERALS, P_SELECT_SMALLEST_NEG_LIT, P_SELECT_SMALLEST_ORIENTABLE,
        P_SELECT_STRONG_RR_NON_RR_OPTIMAL_LIT, P_SELECT_UNLESS_POS_MAX, P_SELECT_UNLESS_UNIQ_MAX,
        P_SELECT_UNLESS_UNIQ_MAX_POS, P_SELECT_UNLESS_UNIQ_MAX_SMALLEST_ORIENTABLE,
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
        SELECT_UNLESS_UNIQ_MAX_SMALLEST_ORIENTABLE, SELECT_UNLESS_UNIQ_POS_MAX,
    };
    use crate::basics::partial_orderings::HoOrderKind;
    use crate::clauses::clause::Clause;
    use crate::clauses::clause_props::CP_IS_ORIENTED;
    use crate::clauses::eqn::Eqn;
    use crate::clauses::eqn_props::{EP_IS_MAXIMAL, EP_IS_PM_INTO_LIT, EP_IS_SELECTED};
    use crate::clauses::eqnlist::EqnList;
    use crate::heuristics::to_params::TermOrdering;
    use crate::orderings::ocb::OrderControlBlock;
    use crate::terms::signature::{Signature, FP_CL_SPLIT_DEF, SIG_PHONY_APP_CODE};
    use crate::terms::simpletypes::alloc_arrow_type;
    use crate::terms::termbanks::TermBank;
    use crate::terms::termtypes::{DerefType, Term, TP_IS_GROUND};
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

    fn applied_free_var(bank: &mut TermBank, code: i64, arg: &Term) -> Term {
        let app = Term::top_alloc(SIG_PHONY_APP_CODE, 2);
        app.set_type(Some(bank.signature().type_bank().default_type()));
        app.set_argument(0, typed_var(bank, code));
        app.set_argument(1, arg.clone());
        bank.insert(&app, DerefType::Never)
            .unwrap_or_else(|err| panic!("{err}"))
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

    fn weighted_predicate_const_atom(bank: &mut TermBank, name: &str, weight: i64) -> Term {
        let atom = predicate_const_atom(bank, name);
        atom.set_weight(weight);
        atom
    }

    fn weighted_predicate_unary_atom(
        bank: &mut TermBank,
        name: &str,
        arg: &Term,
        weight: i64,
    ) -> Term {
        let bool_type = bank.signature().type_bank().bool_type();
        let default_type = bank.signature().type_bank().default_type();
        let f_code = bank.signature_mut().insert_id(name, 1, false);
        let predicate_type = alloc_arrow_type(vec![default_type, bool_type]);
        bank.signature_mut()
            .declare_type(f_code, predicate_type.clone())
            .unwrap();
        let atom = unary(f_code, arg);
        atom.set_type(Some(predicate_type));
        atom.set_weight(weight);
        atom
    }

    fn weighted_predicate_binary_atom(
        bank: &mut TermBank,
        name: &str,
        left: &Term,
        right: &Term,
        weight: i64,
    ) -> Term {
        let bool_type = bank.signature().type_bank().bool_type();
        let default_type = bank.signature().type_bank().default_type();
        let f_code = bank.signature_mut().insert_id(name, 2, false);
        let predicate_type = alloc_arrow_type(vec![default_type.clone(), default_type, bool_type]);
        bank.signature_mut()
            .declare_type(f_code, predicate_type.clone())
            .unwrap();
        let atom = binary(f_code, left, right);
        atom.set_type(Some(predicate_type));
        atom.set_weight(weight);
        atom
    }

    fn shared_weighted_predicate_unary_atom(
        bank: &mut TermBank,
        name: &str,
        arg: &Term,
        weight: i64,
    ) -> Term {
        let atom = weighted_predicate_unary_atom(bank, name, arg, weight);
        let shared = bank.insert(&atom, DerefType::Never).unwrap();
        shared.set_weight(weight);
        if !arg.query_prop(TP_IS_GROUND) {
            shared.del_prop(TP_IS_GROUND);
        }
        shared
    }

    fn shared_weighted_predicate_binary_atom(
        bank: &mut TermBank,
        name: &str,
        left: &Term,
        right: &Term,
        weight: i64,
    ) -> Term {
        let atom = weighted_predicate_binary_atom(bank, name, left, right, weight);
        let shared = bank.insert(&atom, DerefType::Never).unwrap();
        shared.set_weight(weight);
        if !left.query_prop(TP_IS_GROUND) || !right.query_prop(TP_IS_GROUND) {
            shared.del_prop(TP_IS_GROUND);
        }
        shared
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

    fn min_optimal_ground_type_clause(bank: &mut TermBank) -> Clause {
        let pos = predicate_const_atom(bank, "min_opt_filter_ground_pos");
        let type_pred = weighted_predicate_const_atom(bank, "min_opt_filter_ground_type", 3);
        let ordinary = weighted_predicate_const_atom(bank, "min_opt_filter_ground_ordinary", 7);

        let mut clause = Clause::alloc(EqnList::from_vec(vec![
            predicate_literal(bank, &pos, true),
            predicate_literal(bank, &type_pred, false),
            predicate_literal(bank, &ordinary, false),
        ]));
        clause.set_prop(CP_IS_ORIENTED);
        clause
    }

    fn min_optimal_x_filter_fallback_clause(bank: &mut TermBank) -> Clause {
        let pos = predicate_const_atom(bank, "min_opt_x_filter_pos");
        let x = typed_var(bank, -411);
        let repeated_x =
            shared_weighted_predicate_binary_atom(bank, "min_opt_x_filter_repeated", &x, &x, 4);
        let g_x = shared_unary(bank, "min_opt_x_filter_arg", &x);
        let ordinary =
            shared_weighted_predicate_unary_atom(bank, "min_opt_x_filter_ordinary", &g_x, 8);

        let mut clause = Clause::alloc(EqnList::from_vec(vec![
            predicate_literal(bank, &pos, true),
            predicate_literal(bank, &repeated_x, false),
            predicate_literal(bank, &ordinary, false),
        ]));
        clause.set_prop(CP_IS_ORIENTED);
        clause
    }

    fn min_optimal_real_x_filter_fallback_clause(bank: &mut TermBank) -> Clause {
        let pos = predicate_const_atom(bank, "min_opt_rx_filter_pos");
        let x = typed_var(bank, -412);
        let y = typed_var(bank, -413);
        let real_x =
            shared_weighted_predicate_binary_atom(bank, "min_opt_rx_filter_real", &x, &y, 4);
        let g_x = shared_unary(bank, "min_opt_rx_filter_arg", &x);
        let ordinary =
            shared_weighted_predicate_unary_atom(bank, "min_opt_rx_filter_ordinary", &g_x, 8);

        let mut clause = Clause::alloc(EqnList::from_vec(vec![
            predicate_literal(bank, &pos, true),
            predicate_literal(bank, &real_x, false),
            predicate_literal(bank, &ordinary, false),
        ]));
        clause.set_prop(CP_IS_ORIENTED);
        clause
    }

    fn min_optimal_all_real_x_clause(bank: &mut TermBank) -> Clause {
        let pos = predicate_const_atom(bank, "min_opt_all_rx_pos");
        let x = typed_var(bank, -414);
        let y = typed_var(bank, -415);
        let real_x = shared_weighted_predicate_binary_atom(bank, "min_opt_all_rx_real", &x, &y, 4);

        let mut clause = Clause::alloc(EqnList::from_vec(vec![
            predicate_literal(bank, &pos, true),
            predicate_literal(bank, &real_x, false),
        ]));
        clause.set_prop(CP_IS_ORIENTED);
        clause
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

    fn max_lcomplex_priority_clause(bank: &mut TermBank) -> Clause {
        let pos = predicate_const_atom(bank, "max_lcomplex_pos");
        let a = shared_const(bank, "max_lcomplex_a");
        let f_a = shared_unary(bank, "max_lcomplex_f", &a);
        let x = var_term(-300);
        let y = var_term(-302);

        Clause::alloc(EqnList::from_vec(vec![
            predicate_literal(bank, &pos, true),
            literal(bank, &f_a, &a, false),
            literal(bank, &x, &y, false),
        ]))
    }

    fn max_lcomplex_positive_max_fallback_clause(bank: &mut TermBank) -> Clause {
        let pos_a = predicate_const_atom(bank, "max_lcomplex_pos_a");
        let pos_b = predicate_const_atom(bank, "max_lcomplex_pos_b");
        let x = var_term(-310);
        let f_x = unary(310, &x);
        let a = const_term(311);

        Clause::alloc(EqnList::from_vec(vec![
            predicate_literal(bank, &pos_a, true),
            predicate_literal(bank, &pos_b, true),
            literal(bank, &f_x, &a, false),
        ]))
    }

    fn max_lcomplex_no_type_rejected_fallback_clause(bank: &mut TermBank) -> Clause {
        let pos = predicate_const_atom(bank, "max_lcomplex_type_pos");
        let maximal_type = weighted_predicate_const_atom(bank, "max_lcomplex_type_max", 3);
        let fallback_type = weighted_predicate_const_atom(bank, "max_lcomplex_type_fallback", 3);
        let fallback_plain = predicate_const_atom(bank, "max_lcomplex_plain_fallback");

        Clause::alloc(EqnList::from_vec(vec![
            predicate_literal(bank, &pos, true),
            predicate_literal(bank, &maximal_type, false),
            predicate_literal(bank, &fallback_type, false),
            predicate_literal(bank, &fallback_plain, false),
        ]))
    }

    fn max_lcomplex_no_x_type_fallback_clause(bank: &mut TermBank) -> Clause {
        let pos = predicate_const_atom(bank, "max_lcomplex_x_pos");
        let x = var_term(-320);
        let default_type = bank.signature().type_bank().default_type();
        x.set_type(Some(default_type.clone()));
        let maximal_x_type = weighted_predicate_unary_atom(bank, "max_lcomplex_x_max", &x, 3);
        let f_x = unary(320, &x);
        f_x.set_type(Some(default_type.clone()));
        let a = const_term(321);
        a.set_type(Some(default_type));

        Clause::alloc(EqnList::from_vec(vec![
            predicate_literal(bank, &pos, true),
            predicate_literal(bank, &maximal_x_type, false),
            literal(bank, &f_x, &a, false),
        ]))
    }

    fn new_complex_ground_clause(bank: &mut TermBank) -> Clause {
        let pos = predicate_const_atom(bank, "new_complex_ground_pos");
        let heavy = shared_const(bank, "new_complex_heavy");
        let light = shared_const(bank, "new_complex_light");
        let right_a = shared_const(bank, "new_complex_right_a");
        let right_b = shared_const(bank, "new_complex_right_b");

        let mut clause = Clause::alloc(EqnList::from_vec(vec![
            predicate_literal(bank, &pos, true),
            literal(bank, &heavy, &right_a, false),
            literal(bank, &light, &right_b, false),
        ]));
        clause.literals().as_slice()[1].left().set_weight(30);
        clause.literals().as_slice()[2].left().set_weight(3);
        clause.set_prop(CP_IS_ORIENTED);
        clause
    }

    fn new_complex_infpos_clause(bank: &mut TermBank) -> Clause {
        let pos = predicate_const_atom(bank, "new_complex_infpos_pos");
        let x = var_term(-340);
        let y = var_term(-342);
        let larger = unary(340, &unary(341, &x));
        let smaller = unary(342, &y);
        let right = const_term(343);
        let x_type = weighted_predicate_binary_atom(bank, "new_complex_infpos_xtype", &x, &y, 9);

        let mut clause = Clause::alloc(EqnList::from_vec(vec![
            predicate_literal(bank, &pos, true),
            literal(bank, &larger, &right, false),
            literal(bank, &smaller, &right, false),
            predicate_literal(bank, &x_type, false),
        ]));
        clause.set_prop(CP_IS_ORIENTED);
        clause
    }

    fn new_complex_x_type_clause(bank: &mut TermBank) -> Clause {
        let pos = predicate_const_atom(bank, "new_complex_xtype_pos");
        let x = var_term(-350);
        let y = var_term(-352);
        let small = weighted_predicate_binary_atom(bank, "new_complex_xtype_small", &x, &y, 4);
        let large = weighted_predicate_binary_atom(bank, "new_complex_xtype_large", &x, &y, 8);

        let mut clause = Clause::alloc(EqnList::from_vec(vec![
            predicate_literal(bank, &pos, true),
            predicate_literal(bank, &small, false),
            predicate_literal(bank, &large, false),
        ]));
        clause.set_prop(CP_IS_ORIENTED);
        clause
    }

    fn min_infpos_ground_clause(bank: &mut TermBank) -> Clause {
        let pos = predicate_const_atom(bank, "min_infpos_ground_pos");
        let a = shared_const(bank, "min_infpos_a");
        let b = shared_const(bank, "min_infpos_b");
        let c = shared_const(bank, "min_infpos_c");
        let f_a = shared_unary(bank, "min_infpos_f", &a);
        let g_f_a = shared_unary(bank, "min_infpos_g", &f_a);

        let mut clause = Clause::alloc(EqnList::from_vec(vec![
            predicate_literal(bank, &pos, true),
            literal(bank, &g_f_a, &a, false),
            literal(bank, &b, &c, false),
        ]));
        clause.set_prop(CP_IS_ORIENTED);
        clause
    }

    fn min_infpos_nonground_clause(bank: &mut TermBank) -> Clause {
        let pos = predicate_const_atom(bank, "min_infpos_nonground_pos");
        let a = shared_const(bank, "min_infpos_ng_a");
        let f_a = shared_unary(bank, "min_infpos_ng_f", &a);
        let g_f_a = shared_unary(bank, "min_infpos_ng_g", &f_a);
        let x = typed_var(bank, -360);

        let mut clause = Clause::alloc(EqnList::from_vec(vec![
            predicate_literal(bank, &pos, true),
            literal(bank, &x, &a, false),
            literal(bank, &g_f_a, &a, false),
        ]));
        clause.set_prop(CP_IS_ORIENTED);
        clause
    }

    fn min2_infpos_variable_weight_clause(bank: &mut TermBank) -> Clause {
        let pos = predicate_const_atom(bank, "min2_infpos_pos");
        let x = typed_var(bank, -365);
        let y = typed_var(bank, -366);
        let a = shared_const(bank, "min2_infpos_a");
        let b = shared_const(bank, "min2_infpos_b");
        let f_a = shared_unary(bank, "min2_infpos_f", &a);

        let mut clause = Clause::alloc(EqnList::from_vec(vec![
            predicate_literal(bank, &pos, true),
            literal(bank, &x, &y, false),
            literal(bank, &f_a, &b, false),
        ]));
        clause.set_prop(CP_IS_ORIENTED);
        clause
    }

    fn min_infpos_type_filter_clause(bank: &mut TermBank) -> Clause {
        let pos = predicate_const_atom(bank, "min_infpos_type_pos");
        let type_pred = weighted_predicate_const_atom(bank, "min_infpos_type_pred", 3);
        let a = shared_const(bank, "min_infpos_type_a");
        let b = shared_const(bank, "min_infpos_type_b");

        let mut clause = Clause::alloc(EqnList::from_vec(vec![
            predicate_literal(bank, &pos, true),
            predicate_literal(bank, &type_pred, false),
            literal(bank, &a, &b, false),
        ]));
        clause.set_prop(CP_IS_ORIENTED);
        clause
    }

    fn min_infpos_all_type_clause(bank: &mut TermBank) -> Clause {
        let pos = predicate_const_atom(bank, "min_infpos_all_type_pos");
        let first = weighted_predicate_const_atom(bank, "min_infpos_all_type_first", 3);
        let second = weighted_predicate_const_atom(bank, "min_infpos_all_type_second", 3);

        let mut clause = Clause::alloc(EqnList::from_vec(vec![
            predicate_literal(bank, &pos, true),
            predicate_literal(bank, &first, false),
            predicate_literal(bank, &second, false),
        ]));
        clause.set_prop(CP_IS_ORIENTED);
        clause
    }

    fn ahp_head_sharing_clause(bank: &mut TermBank) -> Clause {
        let shared = predicate_const_atom(bank, "ahp_shared");
        let other = predicate_const_atom(bank, "ahp_other");

        let mut clause = Clause::alloc(EqnList::from_vec(vec![
            predicate_literal(bank, &shared, true),
            predicate_literal(bank, &shared, false),
            predicate_literal(bank, &other, false),
        ]));
        clause.set_prop(CP_IS_ORIENTED);
        clause
    }

    fn ahp_non_horn_head_sharing_clause(bank: &mut TermBank) -> Clause {
        let shared = predicate_const_atom(bank, "ahp_non_horn_shared");
        let extra = predicate_const_atom(bank, "ahp_non_horn_extra");
        let other = predicate_const_atom(bank, "ahp_non_horn_other");

        let mut clause = Clause::alloc(EqnList::from_vec(vec![
            predicate_literal(bank, &shared, true),
            predicate_literal(bank, &extra, true),
            predicate_literal(bank, &shared, false),
            predicate_literal(bank, &other, false),
        ]));
        clause.set_prop(CP_IS_ORIENTED);
        clause
    }

    fn ahp_split_clause(bank: &mut TermBank) -> Clause {
        let pos = predicate_const_atom(bank, "ahp_split_pos");
        let split = predicate_const_atom(bank, "ahp_split_bad");
        let ordinary = predicate_const_atom(bank, "ahp_split_ok");
        bank.signature_mut()
            .set_func_prop(split.f_code(), FP_CL_SPLIT_DEF);

        let mut clause = Clause::alloc(EqnList::from_vec(vec![
            predicate_literal(bank, &pos, true),
            predicate_literal(bank, &split, false),
            predicate_literal(bank, &ordinary, false),
        ]));
        clause.set_prop(CP_IS_ORIENTED);
        clause
    }

    fn generic_max_lcomplex_avoid_pos_pred_clause(bank: &mut TermBank) -> Clause {
        let shared = predicate_const_atom(bank, "generic_max_shared");
        let other = predicate_const_atom(bank, "generic_max_other");

        Clause::alloc(EqnList::from_vec(vec![
            predicate_literal(bank, &shared, true),
            predicate_literal(bank, &shared, false),
            predicate_literal(bank, &other, false),
        ]))
    }

    fn generic_max_lcomplex_app_var_clause(bank: &mut TermBank) -> Clause {
        let pos = predicate_const_atom(bank, "generic_max_app_pos");
        let a = shared_const(bank, "generic_max_app_a");
        let app_var = applied_free_var(bank, -380, &a);
        let ordinary_var = typed_var(bank, -382);

        Clause::alloc(EqnList::from_vec(vec![
            predicate_literal(bank, &pos, true),
            literal(bank, &app_var, &a, false),
            literal(bank, &ordinary_var, &a, false),
        ]))
    }

    fn generic_max_lcomplex_forbidden_type_clause(bank: &mut TermBank) -> Clause {
        let pos = predicate_const_atom(bank, "generic_max_type_pos");
        let type_pred = weighted_predicate_const_atom(bank, "generic_max_type_pred", 3);
        let a = shared_const(bank, "generic_max_type_a");
        let b = shared_const(bank, "generic_max_type_b");

        Clause::alloc(EqnList::from_vec(vec![
            predicate_literal(bank, &pos, true),
            predicate_literal(bank, &type_pred, false),
            literal(bank, &a, &b, false),
        ]))
    }

    fn cq_arity_clause(bank: &mut TermBank) -> Clause {
        let pos = predicate_const_atom(bank, "cq_arity_pos");
        let a = shared_const(bank, "cq_arity_a");
        let b = shared_const(bank, "cq_arity_b");
        let unary_atom = weighted_predicate_unary_atom(bank, "cq_arity_unary", &a, 4);
        let binary_atom = weighted_predicate_binary_atom(bank, "cq_arity_binary", &a, &b, 5);

        Clause::alloc(EqnList::from_vec(vec![
            predicate_literal(bank, &pos, true),
            predicate_literal(bank, &unary_atom, false),
            predicate_literal(bank, &binary_atom, false),
            literal(bank, &a, &b, false),
        ]))
    }

    fn cq_ground_bias_clause(bank: &mut TermBank) -> Clause {
        let pos = predicate_const_atom(bank, "cq_ground_pos");
        let a = shared_const(bank, "cq_ground_a");
        let x = typed_var(bank, -390);
        let nonground_atom = weighted_predicate_unary_atom(bank, "cq_ground_p", &x, 4);
        let nonground_atom = bank.insert(&nonground_atom, DerefType::Never).unwrap();
        let ground_atom = weighted_predicate_unary_atom(bank, "cq_ground_p", &a, 4);
        let ground_atom = bank.insert(&ground_atom, DerefType::Never).unwrap();

        Clause::alloc(EqnList::from_vec(vec![
            predicate_literal(bank, &pos, true),
            predicate_literal(bank, &nonground_atom, false),
            predicate_literal(bank, &ground_atom, false),
        ]))
    }

    fn cq_filter_clause(bank: &mut TermBank) -> Clause {
        let pos = predicate_const_atom(bank, "cq_filter_pos");
        let prop = predicate_const_atom(bank, "cq_filter_prop");
        let type_pred = weighted_predicate_const_atom(bank, "cq_filter_type", 3);
        let x = typed_var(bank, -400);
        let y = typed_var(bank, -402);
        let x_type = weighted_predicate_binary_atom(bank, "cq_filter_xtype", &x, &y, 4);
        let a = shared_const(bank, "cq_filter_a");
        let ordinary = weighted_predicate_unary_atom(bank, "cq_filter_ordinary", &a, 4);

        Clause::alloc(EqnList::from_vec(vec![
            predicate_literal(bank, &pos, true),
            predicate_literal(bank, &prop, false),
            predicate_literal(bank, &type_pred, false),
            predicate_literal(bank, &x_type, false),
            predicate_literal(bank, &ordinary, false),
        ]))
    }

    fn cq_precedence_clause(bank: &mut TermBank) -> Clause {
        let pos = predicate_const_atom(bank, "cq_prec_pos");
        let a = shared_const(bank, "cq_prec_a");
        let high_atom = weighted_predicate_unary_atom(bank, "cq_prec_high", &a, 4);
        let low_atom = weighted_predicate_unary_atom(bank, "cq_prec_low", &a, 4);

        Clause::alloc(EqnList::from_vec(vec![
            predicate_literal(bank, &pos, true),
            predicate_literal(bank, &high_atom, false),
            predicate_literal(bank, &low_atom, false),
        ]))
    }

    fn cq_unless_pdom_clause(bank: &mut TermBank, shared_predicate: bool) -> Clause {
        let pos = predicate_const_atom(bank, "cq_pdom_shared");
        let blocked = if shared_predicate {
            predicate_const_atom(bank, "cq_pdom_shared")
        } else {
            predicate_const_atom(bank, "cq_pdom_other")
        };
        let fallback = predicate_const_atom(bank, "cq_pdom_fallback");

        let mut clause = Clause::alloc(EqnList::from_vec(vec![
            predicate_literal(bank, &pos, true),
            predicate_literal(bank, &blocked, false),
            predicate_literal(bank, &fallback, false),
        ]));
        mark_maximal_literals(&mut clause, &[0]);
        clause
    }

    fn vg_non_cr_nonground_clause(bank: &mut TermBank) -> Clause {
        let pos = predicate_const_atom(bank, "vg_non_cr_pos");
        let x = typed_var(bank, -410);
        let y = typed_var(bank, -412);
        let p_x = weighted_predicate_unary_atom(bank, "vg_non_cr_p", &x, 4);
        let q_y = weighted_predicate_unary_atom(bank, "vg_non_cr_q", &y, 4);

        let mut clause = Clause::alloc(EqnList::from_vec(vec![
            predicate_literal(bank, &pos, true),
            predicate_literal(bank, &p_x, false),
            predicate_literal(bank, &q_y, false),
        ]));
        mark_maximal_literals(&mut clause, &[0]);
        clause
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
    fn min_optimal_type_filtered_selectors_preserve_unfiltered_ground_branch() {
        let mut bank = test_bank();
        let mut ordinary = min_optimal_ground_type_clause(&mut bank);

        super::select_min_optimal_no_type_pred(None, &bank, &mut ordinary);
        assert_eq!(selected_indices(&ordinary), vec![1]);
        assert!(!ordinary.query_prop(CP_IS_ORIENTED));

        let mut positive_variant = min_optimal_ground_type_clause(&mut bank);
        super::p_select_min_optimal_no_type_pred(None, &bank, &mut positive_variant);
        assert_eq!(selected_indices(&positive_variant), vec![0, 1]);
        assert!(!positive_variant.query_prop(CP_IS_ORIENTED));
    }

    #[test]
    fn min_optimal_type_filtered_fallbacks_apply_requested_filter() {
        let mut no_x_bank = test_bank();
        let mut no_x_type = min_optimal_x_filter_fallback_clause(&mut no_x_bank);
        super::select_min_optimal_no_x_type_pred(None, &no_x_bank, &mut no_x_type);
        assert_eq!(selected_indices(&no_x_type), vec![2]);
        assert!(!no_x_type.query_prop(CP_IS_ORIENTED));

        let mut no_real_x_bank = test_bank();
        let mut no_real_x_type = min_optimal_x_filter_fallback_clause(&mut no_real_x_bank);
        super::select_min_optimal_no_rx_type_pred(None, &no_real_x_bank, &mut no_real_x_type);
        assert_eq!(selected_indices(&no_real_x_type), vec![1]);

        let mut real_x_bank = test_bank();
        let mut real_x = min_optimal_real_x_filter_fallback_clause(&mut real_x_bank);
        super::p_select_min_optimal_no_rx_type_pred(None, &real_x_bank, &mut real_x);
        assert_eq!(selected_indices(&real_x), vec![0, 2]);
        assert!(!real_x.query_prop(CP_IS_ORIENTED));
    }

    #[test]
    fn min_optimal_type_filtered_no_selection_preserves_direct_call_state() {
        let mut bank = test_bank();
        let mut clause = min_optimal_all_real_x_clause(&mut bank);

        super::p_select_min_optimal_no_rx_type_pred(None, &bank, &mut clause);
        assert_eq!(selected_indices(&clause), Vec::<usize>::new());
        assert!(clause.query_prop(CP_IS_ORIENTED));
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
    fn complex_unique_max_horn_wrappers_preserve_c_gates() {
        let mut bank = test_bank();
        let mut ocb = kbo_ocb(&bank);
        let mut unique_negative = maximal_gate_clause(&mut bank);
        mark_maximal_literals(&mut unique_negative, &[1]);

        super::select_complex_except_uniq_max_horn(&mut ocb, &bank, &mut unique_negative);
        assert_eq!(selected_indices(&unique_negative), Vec::<usize>::new());

        let mut unique_positive = maximal_gate_clause(&mut bank);
        mark_maximal_literals(&mut unique_positive, &[0]);

        super::select_complex_except_uniq_max_pos_horn(&mut ocb, &bank, &mut unique_positive);
        assert_eq!(selected_indices(&unique_positive), Vec::<usize>::new());

        let mut unique_negative_allowed = maximal_gate_clause(&mut bank);
        mark_maximal_literals(&mut unique_negative_allowed, &[1]);

        super::select_complex_except_uniq_max_pos_horn(
            &mut ocb,
            &bank,
            &mut unique_negative_allowed,
        );
        assert_eq!(selected_indices(&unique_negative_allowed), vec![1]);
        assert!(!unique_negative_allowed.query_prop(CP_IS_ORIENTED));
    }

    #[test]
    fn complex_unique_max_horn_positive_and_mixed_wrappers_match_c() {
        let bank = test_bank();
        let mut ocb = kbo_ocb(&bank);
        let mut positive_variant = complex_diff_fallback_clause();
        mark_maximal_literals(&mut positive_variant, &[1, 2]);

        super::p_select_complex_except_uniq_max_horn(&mut ocb, &bank, &mut positive_variant);
        assert_eq!(select_mask(&positive_variant), vec![true, false, true]);

        let mut mixed_variant = complex_diff_fallback_clause();
        mark_maximal_literals(&mut mixed_variant, &[1, 2]);

        super::m_select_complex_except_uniq_max_horn(&mut ocb, &bank, &mut mixed_variant);
        assert_eq!(select_mask(&mixed_variant), vec![true, false, true]);
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
        let _guard = literal_weight_counter_test_guard();
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
    fn unless_uniq_max_smallest_orientable_uses_existing_orientable_selector() {
        let mut bank = test_bank();
        let mut ocb = kbo_ocb(&bank);
        let mut blocked = maximal_gate_clause(&mut bank);
        mark_maximal_literals(&mut blocked, &[0]);

        select_unless_uniq_max_smallest_orientable(&mut ocb, &bank, &mut blocked);

        assert_eq!(selected_indices(&blocked), Vec::<usize>::new());
        assert!(blocked.query_prop(CP_IS_ORIENTED));

        let mut allowed = maximal_gate_clause(&mut bank);
        mark_maximal_literals(&mut allowed, &[0, 1]);

        select_unless_uniq_max_smallest_orientable(&mut ocb, &bank, &mut allowed);

        assert_eq!(selected_indices(&allowed), vec![1]);
        assert!(!allowed.query_prop(CP_IS_ORIENTED));

        let mut positive_variant = maximal_gate_clause(&mut bank);
        mark_maximal_literals(&mut positive_variant, &[0, 1]);

        p_select_unless_uniq_max_smallest_orientable(&mut ocb, &bank, &mut positive_variant);

        assert_eq!(selected_indices(&positive_variant), vec![0, 1]);
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
    fn max_lcomplex_selects_maximal_pure_variable_before_ground() {
        let mut bank = test_bank();
        let mut clause = max_lcomplex_priority_clause(&mut bank);
        mark_maximal_literals(&mut clause, &[0, 1, 2]);
        let mut ocb = kbo_ocb(&bank);

        super::select_max_l_complex(&mut ocb, &bank, &mut clause);

        assert_eq!(selected_indices(&clause), vec![2]);
        assert!(!clause.query_prop(CP_IS_ORIENTED));
    }

    #[test]
    fn max_lcomplex_falls_back_to_lcomplex_when_all_maximal_are_positive() {
        let mut bank = test_bank();
        let mut ordinary = max_lcomplex_positive_max_fallback_clause(&mut bank);
        mark_maximal_literals(&mut ordinary, &[0, 1]);
        let mut ocb = kbo_ocb(&bank);

        super::select_max_l_complex(&mut ocb, &bank, &mut ordinary);
        assert_eq!(selected_indices(&ordinary), vec![2]);

        let mut positive_variant = max_lcomplex_positive_max_fallback_clause(&mut bank);
        mark_maximal_literals(&mut positive_variant, &[0, 1]);

        super::p_select_max_l_complex(&mut ocb, &bank, &mut positive_variant);
        assert_eq!(selected_indices(&positive_variant), vec![0, 1, 2]);
    }

    #[test]
    fn max_lcomplex_no_type_variants_filter_only_the_c_selected_candidate() {
        let mut bank = test_bank();
        let mut clause = max_lcomplex_no_type_rejected_fallback_clause(&mut bank);
        mark_maximal_literals(&mut clause, &[0, 1]);
        let mut ocb = kbo_ocb(&bank);

        super::select_max_l_complex_no_type_pred(&mut ocb, &bank, &mut clause);

        assert_eq!(selected_indices(&clause), Vec::<usize>::new());
        assert!(!clause.query_prop(CP_IS_ORIENTED));
    }

    #[test]
    fn max_lcomplex_no_x_type_positive_variant_uses_nonmaximal_fallback() {
        let mut bank = test_bank();
        let mut clause = max_lcomplex_no_x_type_fallback_clause(&mut bank);
        mark_maximal_literals(&mut clause, &[0, 1]);
        let mut ocb = kbo_ocb(&bank);

        super::p_select_max_l_complex_no_x_type_pred(&mut ocb, &bank, &mut clause);

        assert_eq!(selected_indices(&clause), vec![0, 2]);
    }

    #[test]
    fn generic_max_lcomplex_g_preserves_base_priority() {
        let _guard = literal_weight_counter_test_guard();
        reset_literal_weight_counter_for_tests();
        let mut bank = test_bank();
        let mut clause = max_lcomplex_priority_clause(&mut bank);
        mark_maximal_literals(&mut clause, &[0, 1, 2]);
        let mut ocb = kbo_ocb(&bank);

        super::select_max_l_complex_g(&mut ocb, &bank, &mut clause);

        assert_eq!(selected_indices(&clause), vec![2]);
        assert!(!clause.query_prop(CP_IS_ORIENTED));

        let mut blocked = max_lcomplex_priority_clause(&mut bank);
        mark_maximal_literals(&mut blocked, &[1]);

        super::select_max_l_complex_g(&mut ocb, &bank, &mut blocked);
        assert_eq!(selected_indices(&blocked), Vec::<usize>::new());
        assert!(blocked.query_prop(CP_IS_ORIENTED));
    }

    #[test]
    fn generic_max_lcomplex_avoid_predicates_use_positive_head_penalty() {
        let mut bank = test_bank();
        let mut clause = generic_max_lcomplex_avoid_pos_pred_clause(&mut bank);
        mark_maximal_literals(&mut clause, &[1, 2]);
        let mut ocb = kbo_ocb(&bank);

        super::select_max_l_complex_avoid_pos_pred(&mut ocb, &bank, &mut clause);

        assert_eq!(selected_indices(&clause), vec![2]);
        assert!(!clause.query_prop(CP_IS_ORIENTED));

        let mut uninterpreted = generic_max_lcomplex_avoid_pos_pred_clause(&mut bank);
        mark_maximal_literals(&mut uninterpreted, &[1, 2]);

        super::select_max_l_complex_avoid_pos_u_pred(&mut ocb, &bank, &mut uninterpreted);
        assert_eq!(selected_indices(&uninterpreted), vec![2]);
    }

    #[test]
    fn generic_max_lcomplex_app_var_variants_adjust_priority() {
        let mut bank = test_bank();
        let mut avoid = generic_max_lcomplex_app_var_clause(&mut bank);
        mark_maximal_literals(&mut avoid, &[1, 2]);
        let mut ocb = kbo_ocb(&bank);

        super::select_max_l_complex_avoid_app_var(&mut ocb, &bank, &mut avoid);
        assert_eq!(selected_indices(&avoid), vec![2]);

        let mut strongly_avoid = generic_max_lcomplex_app_var_clause(&mut bank);
        mark_maximal_literals(&mut strongly_avoid, &[1, 2]);

        super::select_max_l_complex_strongly_avoid_app_var(&mut ocb, &bank, &mut strongly_avoid);
        assert_eq!(selected_indices(&strongly_avoid), vec![2]);

        let mut prefer = generic_max_lcomplex_app_var_clause(&mut bank);
        mark_maximal_literals(&mut prefer, &[1, 2]);

        super::select_max_l_complex_prefer_app_var(&mut ocb, &bank, &mut prefer);
        assert_eq!(selected_indices(&prefer), vec![1]);
    }

    #[test]
    fn generic_max_lcomplex_type_filters_preserve_forbidden_candidate_semantics() {
        let mut bank = test_bank();
        let mut app_nt_np = generic_max_lcomplex_forbidden_type_clause(&mut bank);
        mark_maximal_literals(&mut app_nt_np, &[1, 2]);
        let mut ocb = kbo_ocb(&bank);

        super::select_max_l_complex_app_nt_np(&mut ocb, &bank, &mut app_nt_np);

        assert_eq!(selected_indices(&app_nt_np), vec![2]);
        assert!(!app_nt_np.query_prop(CP_IS_ORIENTED));

        let mut app_no_type = generic_max_lcomplex_forbidden_type_clause(&mut bank);
        mark_maximal_literals(&mut app_no_type, &[1, 2]);

        super::select_max_l_complex_app_no_type(&mut ocb, &bank, &mut app_no_type);
        assert_eq!(selected_indices(&app_no_type), Vec::<usize>::new());
        assert!(app_no_type.query_prop(CP_IS_ORIENTED));
    }

    #[test]
    fn cq_arity_variants_preserve_equality_ordering_constants() {
        let mut bank = test_bank();
        let mut eq_last = cq_arity_clause(&mut bank);
        let mut ocb = kbo_ocb(&bank);

        super::select_cq_ar_eq_last(&mut ocb, &bank, &mut eq_last);
        assert_eq!(selected_indices(&eq_last), vec![2]);
        assert!(!eq_last.query_prop(CP_IS_ORIENTED));

        let mut eq_first_bank = test_bank();
        let mut eq_first = cq_arity_clause(&mut eq_first_bank);
        let mut eq_first_ocb = kbo_ocb(&eq_first_bank);
        super::select_cq_ar_eq_first(&mut eq_first_ocb, &eq_first_bank, &mut eq_first);
        assert_eq!(selected_indices(&eq_first), vec![3]);

        let mut inverse_eq_last_bank = test_bank();
        let mut inverse_eq_last = cq_arity_clause(&mut inverse_eq_last_bank);
        let mut inverse_eq_last_ocb = kbo_ocb(&inverse_eq_last_bank);
        super::select_cqi_ar_eq_last(
            &mut inverse_eq_last_ocb,
            &inverse_eq_last_bank,
            &mut inverse_eq_last,
        );
        assert_eq!(selected_indices(&inverse_eq_last), vec![1]);

        let mut inverse_eq_first_bank = test_bank();
        let mut inverse_eq_first = cq_arity_clause(&mut inverse_eq_first_bank);
        let mut inverse_eq_first_ocb = kbo_ocb(&inverse_eq_first_bank);
        super::select_cqi_ar_eq_first(
            &mut inverse_eq_first_ocb,
            &inverse_eq_first_bank,
            &mut inverse_eq_first,
        );
        assert_eq!(selected_indices(&inverse_eq_first), vec![3]);
    }

    #[test]
    fn cq_ground_bias_prefers_ground_only_within_the_same_symbol() {
        let mut bank = test_bank();
        let mut clause = cq_ground_bias_clause(&mut bank);
        let mut ocb = kbo_ocb(&bank);

        super::select_cq_gr_ar_eq_first(&mut ocb, &bank, &mut clause);

        assert_eq!(selected_indices(&clause), vec![2]);
    }

    #[test]
    fn cq_filters_mark_rejected_best_candidate_as_forbidden() {
        let mut bank = test_bank();
        let mut no_prop = cq_filter_clause(&mut bank);
        let mut ocb = kbo_ocb(&bank);

        super::select_cq_ar_np(&mut ocb, &bank, &mut no_prop);
        assert_eq!(selected_indices(&no_prop), vec![3]);

        let mut no_x_type_bank = test_bank();
        let mut no_x_type = cq_filter_clause(&mut no_x_type_bank);
        let mut no_x_type_ocb = kbo_ocb(&no_x_type_bank);
        super::select_cq_ar_nxt_eq_first(&mut no_x_type_ocb, &no_x_type_bank, &mut no_x_type);
        assert_eq!(selected_indices(&no_x_type), vec![4]);
    }

    #[test]
    fn cq_precedence_variants_use_ocb_precedence_weights() {
        let mut bank = test_bank();
        let mut direct = cq_precedence_clause(&mut bank);
        let high = bank.signature().find_f_code("cq_prec_high");
        let low = bank.signature().find_f_code("cq_prec_low");
        let mut ocb = kbo_ocb(&bank);
        ocb.set_fun_prec_weight(high, 30);
        ocb.set_fun_prec_weight(low, 10);

        super::select_cq_prec_w(&mut ocb, &bank, &mut direct);
        assert_eq!(selected_indices(&direct), vec![2]);

        let mut inverted_bank = test_bank();
        let mut inverted = cq_precedence_clause(&mut inverted_bank);
        let high = inverted_bank.signature().find_f_code("cq_prec_high");
        let low = inverted_bank.signature().find_f_code("cq_prec_low");
        let mut inverted_ocb = kbo_ocb(&inverted_bank);
        inverted_ocb.set_fun_prec_weight(high, 30);
        inverted_ocb.set_fun_prec_weight(low, 10);

        super::select_cqi_prec_w(&mut inverted_ocb, &inverted_bank, &mut inverted);
        assert_eq!(selected_indices(&inverted), vec![1]);
    }

    #[test]
    fn cq_unless_pdom_gates_on_maximal_positive_predicate_domination() {
        let mut bank = test_bank();
        let mut blocked = cq_unless_pdom_clause(&mut bank, true);
        let mut ocb = kbo_ocb(&bank);

        super::select_cq_ar_np_eq_first_unless_pdom(&mut ocb, &bank, &mut blocked);

        assert_eq!(selected_indices(&blocked), Vec::<usize>::new());
        assert!(blocked.query_prop(CP_IS_ORIENTED));

        let mut allowed_bank = test_bank();
        let mut allowed = cq_unless_pdom_clause(&mut allowed_bank, false);
        let mut allowed_ocb = kbo_ocb(&allowed_bank);
        super::select_cq_ar_nt_eq_first_unless_pdom(&mut allowed_ocb, &allowed_bank, &mut allowed);
        assert_eq!(selected_indices(&allowed), vec![2]);
        assert!(!allowed.query_prop(CP_IS_ORIENTED));
    }

    #[test]
    fn vg_non_cr_preserves_c_branch_order_and_side_effects() {
        let mut bank = test_bank();
        let mut pure_var = max_lcomplex_priority_clause(&mut bank);
        let mut ocb = kbo_ocb(&bank);

        super::select_vg_non_cr(&mut ocb, &bank, &mut pure_var);
        assert_eq!(selected_indices(&pure_var), vec![2]);

        let mut ground_bank = test_bank();
        let mut ground = new_complex_ground_clause(&mut ground_bank);
        let ground_index = first_smallest_ground_negative_index(&ground);
        let mut ground_ocb = kbo_ocb(&ground_bank);

        super::select_vg_non_cr(&mut ground_ocb, &ground_bank, &mut ground);
        assert_eq!(selected_indices(&ground), vec![ground_index]);
        assert!(ground.query_prop(CP_IS_ORIENTED));

        let mut positive_max_bank = test_bank();
        let mut positive_max = vg_non_cr_nonground_clause(&mut positive_max_bank);
        let mut positive_max_ocb = kbo_ocb(&positive_max_bank);
        super::select_vg_non_cr(&mut positive_max_ocb, &positive_max_bank, &mut positive_max);
        assert_eq!(selected_indices(&positive_max), Vec::<usize>::new());
        assert!(positive_max.query_prop(CP_IS_ORIENTED));
    }

    #[test]
    fn new_complex_selects_ground_literal_with_smallest_max_side() {
        let mut bank = test_bank();
        let mut clause = new_complex_ground_clause(&mut bank);
        let mut ocb = kbo_ocb(&bank);

        super::select_new_complex(&mut ocb, &bank, &mut clause);

        assert_eq!(selected_indices(&clause), vec![2]);
        assert!(!clause.query_prop(CP_IS_ORIENTED));

        let mut positive_variant = new_complex_ground_clause(&mut bank);

        super::p_select_new_complex(&mut ocb, &bank, &mut positive_variant);
        assert_eq!(selected_indices(&positive_variant), vec![0, 2]);
    }

    #[test]
    fn new_complex_uses_min_inference_position_before_x_type() {
        let mut bank = test_bank();
        let mut clause = new_complex_infpos_clause(&mut bank);
        let mut ocb = kbo_ocb(&bank);

        super::select_new_complex(&mut ocb, &bank, &mut clause);

        assert_eq!(selected_indices(&clause), vec![2]);
    }

    #[test]
    fn new_complex_falls_back_to_largest_non_type_x_type_literal() {
        let mut bank = test_bank();
        let mut clause = new_complex_x_type_clause(&mut bank);
        let mut ocb = kbo_ocb(&bank);

        super::select_new_complex(&mut ocb, &bank, &mut clause);

        assert_eq!(selected_indices(&clause), vec![2]);
    }

    #[test]
    fn new_complex_unique_max_horn_wrapper_preserves_c_gate() {
        let mut bank = test_bank();
        let mut blocked = new_complex_ground_clause(&mut bank);
        mark_maximal_literals(&mut blocked, &[1]);
        let mut ocb = kbo_ocb(&bank);

        super::select_new_complex_except_uniq_max_horn(&mut ocb, &bank, &mut blocked);

        assert_eq!(selected_indices(&blocked), Vec::<usize>::new());
        assert!(blocked.query_prop(CP_IS_ORIENTED));

        let mut allowed = new_complex_ground_clause(&mut bank);
        mark_maximal_literals(&mut allowed, &[1, 2]);

        super::p_select_new_complex_except_uniq_max_horn(&mut ocb, &bank, &mut allowed);
        assert_eq!(selected_indices(&allowed), vec![0, 2]);
    }

    #[test]
    fn min_infpos_selects_smallest_inference_position_weight() {
        let mut bank = test_bank();
        let mut clause = min_infpos_ground_clause(&mut bank);
        let mut ocb = kbo_ocb(&bank);

        super::select_min_infpos(&mut ocb, &bank, &mut clause);

        assert_eq!(selected_indices(&clause), vec![2]);
        assert!(!clause.query_prop(CP_IS_ORIENTED));

        let mut positive_variant = min_infpos_ground_clause(&mut bank);

        super::p_select_min_infpos(&mut ocb, &bank, &mut positive_variant);
        assert_eq!(selected_indices(&positive_variant), vec![0, 2]);
    }

    #[test]
    fn min2_infpos_uses_variable_weight_two() {
        let mut bank = test_bank();
        let mut standard = min2_infpos_variable_weight_clause(&mut bank);
        let mut ocb = kbo_ocb(&bank);

        super::select_min_infpos(&mut ocb, &bank, &mut standard);
        assert_eq!(selected_indices(&standard), vec![1]);

        let mut weighted = min2_infpos_variable_weight_clause(&mut bank);
        select_min2_infpos(&mut ocb, &bank, &mut weighted);
        assert_eq!(selected_indices(&weighted), vec![2]);
        assert!(!weighted.query_prop(CP_IS_ORIENTED));

        let mut positive_variant = min2_infpos_variable_weight_clause(&mut bank);
        p_select_min2_infpos(&mut ocb, &bank, &mut positive_variant);
        assert_eq!(selected_indices(&positive_variant), vec![0, 2]);
    }

    #[test]
    fn min_infpos_h_and_g_variants_gate_positive_selection_on_groundness() {
        let mut bank = test_bank();
        let mut nonground_h = min_infpos_nonground_clause(&mut bank);
        let mut ocb = kbo_ocb(&bank);

        super::h_select_min_infpos(&mut ocb, &bank, &mut nonground_h);
        assert_eq!(selected_indices(&nonground_h), vec![0, 1]);

        let mut nonground_g = min_infpos_nonground_clause(&mut bank);

        super::g_select_min_infpos(&mut ocb, &bank, &mut nonground_g);
        assert_eq!(selected_indices(&nonground_g), vec![1]);

        let mut ground_h = min_infpos_ground_clause(&mut bank);

        super::h_select_min_infpos(&mut ocb, &bank, &mut ground_h);
        assert_eq!(selected_indices(&ground_h), vec![2]);

        let mut ground_g = min_infpos_ground_clause(&mut bank);

        super::g_select_min_infpos(&mut ocb, &bank, &mut ground_g);
        assert_eq!(selected_indices(&ground_g), vec![0, 2]);
    }

    #[test]
    fn min_infpos_no_type_pred_variants_filter_and_allow_no_selection() {
        let mut bank = test_bank();
        let mut filtered = min_infpos_type_filter_clause(&mut bank);
        let mut ocb = kbo_ocb(&bank);

        super::p_select_min_infpos_no_type_pred(&mut ocb, &bank, &mut filtered);

        assert_eq!(selected_indices(&filtered), vec![0, 2]);
        assert!(!filtered.query_prop(CP_IS_ORIENTED));

        let mut all_type = min_infpos_all_type_clause(&mut bank);

        super::select_min_infpos_no_type_pred(&mut ocb, &bank, &mut all_type);
        assert_eq!(selected_indices(&all_type), Vec::<usize>::new());
        assert!(all_type.query_prop(CP_IS_ORIENTED));

        let mut all_type_positive = min_infpos_all_type_clause(&mut bank);

        super::p_select_min_infpos_no_type_pred(&mut ocb, &bank, &mut all_type_positive);
        assert_eq!(selected_indices(&all_type_positive), Vec::<usize>::new());
        assert!(all_type_positive.query_prop(CP_IS_ORIENTED));
    }

    #[test]
    fn complex_ahp_uses_positive_head_distribution_as_tiebreaker() {
        let mut bank = test_bank();
        let mut clause = ahp_head_sharing_clause(&mut bank);
        let mut ocb = kbo_ocb(&bank);

        super::select_complex_ahp(&mut ocb, &bank, &mut clause);

        assert_eq!(selected_indices(&clause), vec![2]);
        assert!(!clause.query_prop(CP_IS_ORIENTED));

        let mut positive_variant = ahp_head_sharing_clause(&mut bank);

        super::p_select_complex_ahp(&mut ocb, &bank, &mut positive_variant);
        assert_eq!(selected_indices(&positive_variant), vec![0, 2]);
    }

    #[test]
    fn complex_ahp_rr_horn_wrapper_preserves_c_noop_gate() {
        let bank = test_bank();
        let mut ocb = kbo_ocb(&bank);
        let mut blocked = range_restricted_clause();
        blocked.literals_mut().set_prop(EP_IS_SELECTED);

        super::select_complex_ahp_except_rr_horn(&mut ocb, &bank, &mut blocked);

        assert_eq!(selected_indices(&blocked), vec![0, 1]);

        let mut allowed_bank = test_bank();
        let mut allowed = ahp_non_horn_head_sharing_clause(&mut allowed_bank);
        let mut allowed_ocb = kbo_ocb(&allowed_bank);

        super::p_select_complex_ahp_except_rr_horn(&mut allowed_ocb, &allowed_bank, &mut allowed);
        assert_eq!(selected_indices(&allowed), vec![0, 1, 3]);
    }

    #[test]
    fn new_complex_ahp_uses_positive_head_distribution_and_filters_split() {
        let mut bank = test_bank();
        let mut clause = ahp_head_sharing_clause(&mut bank);
        let mut ocb = kbo_ocb(&bank);

        super::select_new_complex_ahp(&mut ocb, &bank, &mut clause);

        assert_eq!(selected_indices(&clause), vec![2]);
        assert!(!clause.query_prop(CP_IS_ORIENTED));

        let mut positive_variant = ahp_head_sharing_clause(&mut bank);

        super::p_select_new_complex_ahp(&mut ocb, &bank, &mut positive_variant);
        assert_eq!(selected_indices(&positive_variant), vec![0, 2]);

        let mut split_filtered = ahp_split_clause(&mut bank);

        super::select_new_complex_ahp_ns(&mut ocb, &bank, &mut split_filtered);
        assert_eq!(selected_indices(&split_filtered), vec![2]);
        assert!(!split_filtered.query_prop(CP_IS_ORIENTED));
    }

    #[test]
    fn new_complex_ahp_wrappers_preserve_rr_and_unique_max_gates() {
        let bank = test_bank();
        let mut ocb = kbo_ocb(&bank);
        let mut rr_blocked = range_restricted_clause();

        super::select_new_complex_ahp_except_rr_horn(&mut ocb, &bank, &mut rr_blocked);
        assert_eq!(selected_indices(&rr_blocked), Vec::<usize>::new());

        let mut allowed_bank = test_bank();
        let mut allowed = ahp_non_horn_head_sharing_clause(&mut allowed_bank);
        let mut allowed_ocb = kbo_ocb(&allowed_bank);

        super::p_select_new_complex_ahp_except_rr_horn(
            &mut allowed_ocb,
            &allowed_bank,
            &mut allowed,
        );
        assert_eq!(selected_indices(&allowed), vec![0, 1, 3]);

        let mut unique_bank = test_bank();
        let mut unique_blocked = ahp_head_sharing_clause(&mut unique_bank);
        mark_maximal_literals(&mut unique_blocked, &[1]);
        let mut unique_ocb = kbo_ocb(&unique_bank);

        super::select_new_complex_ahp_except_uniq_max_horn(
            &mut unique_ocb,
            &unique_bank,
            &mut unique_blocked,
        );
        assert_eq!(selected_indices(&unique_blocked), Vec::<usize>::new());
        assert!(unique_blocked.query_prop(CP_IS_ORIENTED));

        let mut unique_allowed = ahp_head_sharing_clause(&mut unique_bank);
        mark_maximal_literals(&mut unique_allowed, &[1, 2]);

        super::p_select_new_complex_ahp_except_uniq_max_horn(
            &mut unique_ocb,
            &unique_bank,
            &mut unique_allowed,
        );
        assert_eq!(selected_indices(&unique_allowed), vec![0, 2]);
        assert!(!unique_allowed.query_prop(CP_IS_ORIENTED));
    }

    #[test]
    fn bank_aware_unless_max_selectors_are_available_by_c_strategy_name() {
        for name in [
            SELECT_UNLESS_UNIQ_MAX,
            P_SELECT_UNLESS_UNIQ_MAX,
            SELECT_UNLESS_UNIQ_MAX_SMALLEST_ORIENTABLE,
            P_SELECT_UNLESS_UNIQ_MAX_SMALLEST_ORIENTABLE,
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
            if name == P_SELECT_UNLESS_UNIQ_MAX_SMALLEST_ORIENTABLE {
                assert_eq!(selected_indices(&clause), vec![0, 1]);
            } else {
                assert_eq!(selected_indices(&clause), vec![1]);
            }
        }
    }

    #[test]
    fn bank_aware_max_lcomplex_selectors_are_available_by_c_strategy_name() {
        for name in [
            super::SELECT_MAX_L_COMPLEX,
            super::P_SELECT_MAX_L_COMPLEX,
            super::SELECT_MAX_L_COMPLEX_NO_TYPE_PRED,
            super::P_SELECT_MAX_L_COMPLEX_NO_TYPE_PRED,
            super::SELECT_MAX_L_COMPLEX_NO_X_TYPE_PRED,
            super::P_SELECT_MAX_L_COMPLEX_NO_X_TYPE_PRED,
            super::SELECT_MAX_L_COMPLEX_G,
            super::SELECT_MAX_L_COMPLEX_AVOID_POS_PRED,
            super::SELECT_MAX_L_COMPLEX_APP_NT_NP,
            super::SELECT_MAX_L_COMPLEX_APP_NO_TYPE,
            super::SELECT_MAX_L_COMPLEX_AVOID_POS_U_PRED,
            super::SELECT_MAX_L_COMPLEX_AVOID_APP_VAR,
            super::SELECT_MAX_L_COMPLEX_STRONGLY_AVOID_APP_VAR,
            super::SELECT_MAX_L_COMPLEX_PREFER_APP_VAR,
        ] {
            let mut bank = test_bank();
            let mut clause = max_lcomplex_priority_clause(&mut bank);
            mark_maximal_literals(&mut clause, &[0, 1, 2]);
            let mut ocb = kbo_ocb(&bank);

            apply_ported_literal_selector_with_bank(name, Some(&mut ocb), Some(&bank), &mut clause)
                .unwrap_or_else(|err| {
                    panic!("{err}");
                });
            assert!(clause.prop_lit_number(EP_IS_SELECTED) >= 1);
        }
    }

    #[test]
    fn bank_aware_new_complex_selectors_are_available_by_c_strategy_name() {
        for name in [
            super::SELECT_NEW_COMPLEX,
            super::P_SELECT_NEW_COMPLEX,
            super::SELECT_NEW_COMPLEX_EXCEPT_UNIQ_MAX_HORN,
            super::P_SELECT_NEW_COMPLEX_EXCEPT_UNIQ_MAX_HORN,
        ] {
            let mut bank = test_bank();
            let mut clause = new_complex_ground_clause(&mut bank);
            let mut ocb = kbo_ocb(&bank);

            apply_ported_literal_selector_with_bank(name, Some(&mut ocb), Some(&bank), &mut clause)
                .unwrap_or_else(|err| {
                    panic!("{err}");
                });
            assert!(clause.prop_lit_number(EP_IS_SELECTED) >= 1);
        }
    }

    #[test]
    fn bank_aware_min_infpos_selectors_are_available_by_c_strategy_name() {
        for name in [
            super::SELECT_MIN_INFPOS,
            super::P_SELECT_MIN_INFPOS,
            super::H_SELECT_MIN_INFPOS,
            super::G_SELECT_MIN_INFPOS,
            super::SELECT_MIN_INFPOS_NO_TYPE_PRED,
            super::P_SELECT_MIN_INFPOS_NO_TYPE_PRED,
            super::SELECT_MIN2_INFPOS,
            super::P_SELECT_MIN2_INFPOS,
        ] {
            let mut bank = test_bank();
            let mut clause = min_infpos_ground_clause(&mut bank);
            let mut ocb = kbo_ocb(&bank);

            apply_ported_literal_selector_with_bank(name, Some(&mut ocb), Some(&bank), &mut clause)
                .unwrap_or_else(|err| {
                    panic!("{err}");
                });
            assert!(clause.prop_lit_number(EP_IS_SELECTED) >= 1);
        }
    }

    #[test]
    fn bank_aware_ahp_selectors_are_available_by_c_strategy_name() {
        for name in [
            super::SELECT_COMPLEX_AHP,
            super::P_SELECT_COMPLEX_AHP,
            super::SELECT_COMPLEX_AHP_EXCEPT_RR_HORN,
            super::P_SELECT_COMPLEX_AHP_EXCEPT_RR_HORN,
            super::SELECT_NEW_COMPLEX_AHP,
            super::P_SELECT_NEW_COMPLEX_AHP,
            super::SELECT_NEW_COMPLEX_AHP_EXCEPT_RR_HORN,
            super::P_SELECT_NEW_COMPLEX_AHP_EXCEPT_RR_HORN,
            super::SELECT_NEW_COMPLEX_AHP_EXCEPT_UNIQ_MAX_HORN,
            super::P_SELECT_NEW_COMPLEX_AHP_EXCEPT_UNIQ_MAX_HORN,
            super::SELECT_NEW_COMPLEX_AHP_NS,
        ] {
            let mut bank = test_bank();
            let mut clause = ahp_non_horn_head_sharing_clause(&mut bank);
            let mut ocb = kbo_ocb(&bank);

            apply_ported_literal_selector_with_bank(name, Some(&mut ocb), Some(&bank), &mut clause)
                .unwrap_or_else(|err| {
                    panic!("{err}");
                });
            assert!(clause.prop_lit_number(EP_IS_SELECTED) >= 1);
        }
    }

    #[test]
    fn bank_aware_cq_selectors_are_available_by_c_strategy_name() {
        for name in [
            super::SELECT_VG_NON_CR,
            super::SELECT_CQ_AR_EQ_LAST,
            super::SELECT_CQ_AR_EQ_FIRST,
            super::SELECT_CQI_AR_EQ_LAST,
            super::SELECT_CQI_AR_EQ_FIRST,
            super::SELECT_CQ_AR,
            super::SELECT_CQI_AR,
            super::SELECT_CQ_AR_NP_EQ_FIRST,
            super::SELECT_CQI_AR_NP_EQ_FIRST,
            super::SELECT_CQ_GR_AR_EQ_FIRST,
            super::SELECT_CQ_AR_NT_EQ_FIRST,
            super::SELECT_CQI_AR_NT_EQ_FIRST,
            super::SELECT_CQ_AR_NT_NP_EQ_FIRST,
            super::SELECT_CQI_AR_NT_NP_EQ_FIRST,
            super::SELECT_CQ_AR_NXT_EQ_FIRST,
            super::SELECT_CQI_AR_NXT_EQ_FIRST,
            super::SELECT_CQ_AR_NT_NP,
            super::SELECT_CQI_AR_NT_NP,
            super::SELECT_CQ_AR_NT,
            super::SELECT_CQI_AR_NT,
            super::SELECT_CQ_AR_NP,
            super::SELECT_CQI_AR_NP,
            super::SELECT_CQ_AR_NP_EQ_FIRST_UNLESS_PDOM,
            super::SELECT_CQ_AR_NT_EQ_FIRST_UNLESS_PDOM,
            super::SELECT_CQ_PREC_W,
            super::SELECT_CQI_PREC_W,
            super::SELECT_CQ_PREC_W_NT_NP,
            super::SELECT_CQI_PREC_W_NT_NP,
        ] {
            let mut bank = test_bank();
            let mut clause = cq_arity_clause(&mut bank);
            let mut ocb = kbo_ocb(&bank);

            apply_ported_literal_selector_with_bank(name, Some(&mut ocb), Some(&bank), &mut clause)
                .unwrap_or_else(|err| {
                    panic!("{err}");
                });
            assert!(clause.prop_lit_number(EP_IS_SELECTED) >= 1);
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
    fn bank_aware_complex_max_horn_wrappers_are_available_by_c_strategy_name() {
        for name in [
            super::SELECT_COMPLEX_EXCEPT_UNIQ_MAX_HORN,
            super::P_SELECT_COMPLEX_EXCEPT_UNIQ_MAX_HORN,
            super::M_SELECT_COMPLEX_EXCEPT_UNIQ_MAX_HORN,
            super::SELECT_COMPLEX_EXCEPT_UNIQ_MAX_POS_HORN,
            super::P_SELECT_COMPLEX_EXCEPT_UNIQ_MAX_POS_HORN,
        ] {
            let bank = test_bank();
            let mut clause = complex_diff_fallback_clause();
            mark_maximal_literals(&mut clause, &[1, 2]);
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
    fn bank_aware_min_optimal_type_filtered_selectors_are_available_by_c_strategy_name() {
        for name in [
            super::SELECT_MIN_OPTIMAL_NO_TYPE_PRED,
            super::P_SELECT_MIN_OPTIMAL_NO_TYPE_PRED,
            super::SELECT_MIN_OPTIMAL_NO_X_TYPE_PRED,
            super::P_SELECT_MIN_OPTIMAL_NO_X_TYPE_PRED,
            super::SELECT_MIN_OPTIMAL_NO_RX_TYPE_PRED,
            super::P_SELECT_MIN_OPTIMAL_NO_RX_TYPE_PRED,
        ] {
            let mut bank = test_bank();
            let mut clause = match name {
                super::SELECT_MIN_OPTIMAL_NO_TYPE_PRED
                | super::P_SELECT_MIN_OPTIMAL_NO_TYPE_PRED => {
                    min_optimal_ground_type_clause(&mut bank)
                }
                super::SELECT_MIN_OPTIMAL_NO_X_TYPE_PRED
                | super::P_SELECT_MIN_OPTIMAL_NO_X_TYPE_PRED => {
                    min_optimal_x_filter_fallback_clause(&mut bank)
                }
                _ => min_optimal_real_x_filter_fallback_clause(&mut bank),
            };

            apply_ported_literal_selector_with_bank(name, None, Some(&bank), &mut clause)
                .unwrap_or_else(|err| {
                    panic!("{err}");
                });
            assert!(clause.prop_lit_number(EP_IS_SELECTED) >= 1);
        }

        let mut clause = Clause::empty();
        let error = apply_ported_literal_selector(
            super::SELECT_MIN_OPTIMAL_NO_TYPE_PRED,
            None,
            &mut clause,
        )
        .unwrap_err();
        assert_eq!(error.strategy(), super::SELECT_MIN_OPTIMAL_NO_TYPE_PRED);
    }

    #[test]
    fn unsupported_selector_reports_name() {
        let mut clause = Clause::empty();
        let error =
            apply_ported_literal_selector("UnknownLiteralSelector", None, &mut clause).unwrap_err();

        assert_eq!(error.strategy(), "UnknownLiteralSelector");
        assert!(error.to_string().contains("not ported yet"));
    }
}
