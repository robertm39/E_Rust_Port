use crate::basics::pstacks::PStack;
use crate::clauses::clause::Clause;
use crate::terms::termtypes::RewriteDemodulator;

pub const ARG1_FOF: i64 = 1 << 8;
pub const ARG1_CNF: i64 = 1 << 9;
pub const ARG1_NUM: i64 = 1 << 10;
pub const ARG2_FOF: i64 = 1 << 11;
pub const ARG2_CNF: i64 = 1 << 12;
pub const ARG2_NUM: i64 = 1 << 13;
pub const ARG_IS_HO: i64 = 1 << 14;

pub const DO_NOP: i64 = 0;
pub const DO_QUOTE: i64 = 1;
pub const DO_ADD_CNF_ARG: i64 = 2;
pub const DO_EVAL_GC: i64 = 3;
pub const DO_REWRITE: i64 = 4;
pub const DO_LOCAL_REWRITE: i64 = 5;
pub const DO_UNFOLD: i64 = 6;
pub const DO_APPLY_DEF: i64 = 7;
pub const DO_CONTEXT_SR: i64 = 8;
pub const DO_DES_EQ_RES: i64 = 9;
pub const DO_SR: i64 = 10;
pub const DO_AC_RES: i64 = 11;
pub const DO_CONDENSE: i64 = 12;
pub const DO_NORMALIZE: i64 = 13;
pub const DO_EVAL_ANSWERS: i64 = 14;
pub const DO_PARAMOD: i64 = 24;
pub const DO_SIM_PARAMOD: i64 = 25;
pub const DO_ORDERED_FACTOR: i64 = 26;
pub const DO_EQ_FACTOR: i64 = 27;
pub const DO_EQ_RES: i64 = 28;
pub const DO_DIS_EQ_DECOMPOSE: i64 = 29;
pub const DO_SAT_GEN: i64 = 30;

pub const DC_CNF_QUOTE: i64 = DO_QUOTE | ARG1_CNF;
pub const DC_CNF_ADD_ARG: i64 = DO_ADD_CNF_ARG | ARG1_CNF;
pub const DC_CNF_EVAL_GC: i64 = DO_EVAL_GC;
pub const DC_REWRITE: i64 = DO_REWRITE | ARG1_CNF;
pub const DC_LOCAL_REWRITE: i64 = DO_LOCAL_REWRITE;
pub const DC_CONTEXT_SR: i64 = DO_CONTEXT_SR | ARG1_CNF;
pub const DC_DES_EQ_RES: i64 = DO_DES_EQ_RES;
pub const DC_SR: i64 = DO_SR | ARG1_CNF;
pub const DC_AC_RES: i64 = DO_AC_RES | ARG1_NUM;
pub const DC_CONDENSE: i64 = DO_CONDENSE;
pub const DC_NORMALIZE: i64 = DO_NORMALIZE;
pub const DC_PARAMOD: i64 = DO_PARAMOD | ARG1_CNF | ARG2_CNF;
pub const DC_SIM_PARAMOD: i64 = DO_SIM_PARAMOD | ARG1_CNF | ARG2_CNF;
pub const DC_ORDERED_FACTOR: i64 = DO_ORDERED_FACTOR | ARG1_CNF;
pub const DC_EQ_FACTOR: i64 = DO_EQ_FACTOR | ARG1_CNF;
pub const DC_EQ_RES: i64 = DO_EQ_RES | ARG1_CNF;
pub const DC_DIS_EQ_DECOMPOSE: i64 = DO_DIS_EQ_DECOMPOSE | ARG1_CNF;
pub const DC_SAT_GEN: i64 = DO_SAT_GEN | ARG1_CNF;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ClauseDerivationRef {
    ident: i64,
    source: u64,
}

impl ClauseDerivationRef {
    #[must_use]
    pub const fn new(ident: i64, source: u64) -> Self {
        Self { ident, source }
    }

    #[must_use]
    pub const fn ident(self) -> i64 {
        self.ident
    }

    #[must_use]
    pub const fn source(self) -> u64 {
        self.source
    }
}

impl From<&Clause> for ClauseDerivationRef {
    fn from(clause: &Clause) -> Self {
        Self::new(clause.ident(), clause.query_csscpa_source())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DerivationEntry {
    Operation(i64),
    ClauseParent(ClauseDerivationRef),
    NumericArg(i64),
    Demodulator(RewriteDemodulator),
}

#[must_use]
pub const fn op_has_cnf_arg1(op: i64) -> bool {
    (op & ARG1_CNF) != 0
}

#[must_use]
pub const fn op_has_fof_arg1(op: i64) -> bool {
    (op & ARG1_FOF) != 0
}

#[must_use]
pub const fn op_has_num_arg1(op: i64) -> bool {
    (op & ARG1_NUM) != 0
}

#[must_use]
pub const fn op_has_arg1(op: i64) -> bool {
    (op & (ARG1_CNF | ARG1_FOF | ARG1_NUM)) != 0
}

#[must_use]
pub const fn op_has_cnf_arg2(op: i64) -> bool {
    (op & ARG2_CNF) != 0
}

#[must_use]
pub const fn op_has_fof_arg2(op: i64) -> bool {
    (op & ARG2_FOF) != 0
}

#[must_use]
pub const fn op_has_num_arg2(op: i64) -> bool {
    (op & ARG2_NUM) != 0
}

#[must_use]
pub const fn op_has_arg2(op: i64) -> bool {
    (op & (ARG2_CNF | ARG2_FOF | ARG2_NUM)) != 0
}

#[must_use]
pub const fn op_code(op: i64) -> i64 {
    op & 127
}

#[must_use]
pub const fn op_is_generating(op: i64) -> bool {
    let code = op_code(op);
    code >= DO_PARAMOD && code <= DO_SAT_GEN
}

#[must_use]
pub const fn set_is_ho(op: i64) -> i64 {
    op | ARG_IS_HO
}

#[must_use]
pub const fn get_is_ho(op: i64) -> bool {
    (op & ARG_IS_HO) != 0
}

/// Pushes a clause derivation operation with optional clause parents.
///
/// # Panics
///
/// Panics if `op` is `DCNop`, if a provided parent is not permitted by the
/// opcode argument bits, if a second CNF parent is requested without a first
/// CNF parent, or if the opcode requires formula parents. Formula derivation
/// entries are intentionally left to the later formula-owner slice.
pub fn clause_push_derivation(
    clause: &mut Clause,
    op: i64,
    arg1: Option<&Clause>,
    arg2: Option<&Clause>,
) {
    assert!(op != 0, "derivation opcode must not be DCNop");
    assert!(
        op_has_cnf_arg1(op) || !op_has_cnf_arg2(op),
        "C derivation stack permits CNF arg2 only after CNF arg1"
    );
    assert!(
        op_has_cnf_arg1(op) || op_has_fof_arg1(op) || arg1.is_none(),
        "derivation arg1 is not permitted by opcode"
    );
    assert!(
        op_has_cnf_arg2(op) || op_has_fof_arg2(op) || arg2.is_none(),
        "derivation arg2 is not permitted by opcode"
    );
    assert!(
        op_has_cnf_arg1(op) || !op_has_fof_arg1(op),
        "FOF derivation parents are not represented for clauses yet"
    );
    assert!(
        op_has_cnf_arg2(op) || !op_has_fof_arg2(op),
        "FOF derivation parents are not represented for clauses yet"
    );

    let stack = clause.ensure_derivation();
    stack.push(DerivationEntry::Operation(op));
    if let Some(parent) = arg1 {
        stack.push(DerivationEntry::ClauseParent(parent.into()));
        if let Some(parent) = arg2 {
            stack.push(DerivationEntry::ClauseParent(parent.into()));
        }
    }
}

/// Pushes a clause derivation operation with one numeric argument.
///
/// # Panics
///
/// Panics if `op` is `DCNop` or if the opcode does not permit a numeric first
/// argument.
pub fn clause_push_numeric_derivation(clause: &mut Clause, op: i64, arg1: i64) {
    assert!(op != 0, "derivation opcode must not be DCNop");
    assert!(
        op_has_num_arg1(op),
        "numeric derivation arg1 is not permitted by opcode"
    );
    let stack = clause.ensure_derivation();
    stack.push(DerivationEntry::Operation(op));
    stack.push(DerivationEntry::NumericArg(arg1));
}

#[must_use]
pub fn clause_is_eval_gc(clause: &Clause) -> bool {
    derivation_top_operation(clause) == Some(DC_CNF_EVAL_GC)
}

#[must_use]
pub fn clause_is_dummy_quote(clause: &Clause) -> bool {
    let Some(derivation) = clause.derivation() else {
        return false;
    };
    matches!(
        derivation.as_slice(),
        [
            DerivationEntry::Operation(DC_CNF_QUOTE),
            DerivationEntry::ClauseParent(_)
        ]
    )
}

fn derivation_top_operation(clause: &Clause) -> Option<i64> {
    let derivation = clause.derivation()?;
    match derivation.as_slice().last() {
        Some(DerivationEntry::Operation(op)) => Some(*op),
        _ => None,
    }
}

#[must_use]
pub fn derivation_entries(clause: &Clause) -> &[DerivationEntry] {
    clause.derivation().map_or(&[], PStack::as_slice)
}

#[cfg(test)]
mod tests {
    use super::{
        clause_is_dummy_quote, clause_is_eval_gc, clause_push_derivation,
        clause_push_numeric_derivation, derivation_entries, get_is_ho, op_code, op_is_generating,
        set_is_ho, ClauseDerivationRef, DerivationEntry, ARG1_CNF, ARG1_NUM, ARG2_CNF, ARG_IS_HO,
        DC_AC_RES, DC_CNF_EVAL_GC, DC_CNF_QUOTE, DC_EQ_FACTOR, DC_EQ_RES, DC_LOCAL_REWRITE,
        DC_ORDERED_FACTOR, DC_REWRITE,
    };
    use crate::clauses::clause::Clause;
    use crate::clauses::eqnlist::EqnList;

    #[test]
    fn derivation_code_values_match_c_bit_layout() {
        assert_eq!(ARG1_CNF, 512);
        assert_eq!(ARG1_NUM, 1024);
        assert_eq!(ARG2_CNF, 4096);
        assert_eq!(ARG_IS_HO, 16384);
        assert_eq!(DC_LOCAL_REWRITE, 5);
        assert_eq!(DC_REWRITE, 516);
        assert_eq!(DC_CNF_QUOTE, 513);
        assert_eq!(DC_ORDERED_FACTOR, 538);
        assert_eq!(DC_EQ_FACTOR, 539);
        assert_eq!(DC_EQ_RES, 540);
        assert_eq!(DC_AC_RES, 1035);
        assert_eq!(op_code(DC_EQ_RES), 28);
        assert!(op_is_generating(DC_EQ_FACTOR));
        assert!(get_is_ho(set_is_ho(DC_EQ_RES)));
    }

    #[test]
    fn clause_push_derivation_records_opcode_and_clause_parent() {
        let mut parent = Clause::alloc(EqnList::new());
        parent.set_ident(42);
        parent.set_csscpa_source(7);
        let mut child = Clause::alloc(EqnList::new());

        clause_push_derivation(&mut child, DC_EQ_RES, Some(&parent), None);

        assert_eq!(
            derivation_entries(&child),
            &[
                DerivationEntry::Operation(DC_EQ_RES),
                DerivationEntry::ClauseParent(ClauseDerivationRef::new(42, 7)),
            ]
        );
    }

    #[test]
    fn clause_push_numeric_derivation_records_numeric_arg() {
        let mut clause = Clause::alloc(EqnList::new());

        clause_push_numeric_derivation(&mut clause, DC_AC_RES, 3);

        assert_eq!(
            derivation_entries(&clause),
            &[
                DerivationEntry::Operation(DC_AC_RES),
                DerivationEntry::NumericArg(3),
            ]
        );
    }

    #[test]
    fn clause_derivation_shape_predicates_follow_c_stack_checks() {
        let mut quoted = Clause::alloc(EqnList::new());
        let parent = Clause::alloc(EqnList::new());
        clause_push_derivation(&mut quoted, DC_CNF_QUOTE, Some(&parent), None);
        assert!(clause_is_dummy_quote(&quoted));
        assert!(!clause_is_eval_gc(&quoted));

        let mut eval_gc = Clause::alloc(EqnList::new());
        clause_push_derivation(&mut eval_gc, DC_CNF_EVAL_GC, None, None);
        assert!(clause_is_eval_gc(&eval_gc));
        assert!(!clause_is_dummy_quote(&eval_gc));
    }
}
