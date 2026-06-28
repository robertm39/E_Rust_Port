use crate::basics::pstacks::PStack;
use crate::clauses::clause::Clause;
use crate::terms::termtypes::RewriteDemodulator;
use std::fmt::Write as _;

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
pub const DO_INTRO_DEF: i64 = 33;

pub const DC_CNF_QUOTE: i64 = DO_QUOTE | ARG1_CNF;
pub const DC_CNF_ADD_ARG: i64 = DO_ADD_CNF_ARG | ARG1_CNF;
pub const DC_CNF_EVAL_GC: i64 = DO_EVAL_GC;
pub const DC_REWRITE: i64 = DO_REWRITE | ARG1_CNF;
pub const DC_LOCAL_REWRITE: i64 = DO_LOCAL_REWRITE;
pub const DC_UNFOLD: i64 = DO_UNFOLD | ARG1_CNF;
pub const DC_APPLY_DEF: i64 = DO_APPLY_DEF | ARG1_FOF;
pub const DC_CONTEXT_SR: i64 = DO_CONTEXT_SR | ARG1_CNF;
pub const DC_DES_EQ_RES: i64 = DO_DES_EQ_RES;
pub const DC_SR: i64 = DO_SR | ARG1_CNF;
pub const DC_AC_RES: i64 = DO_AC_RES | ARG1_NUM;
pub const DC_CONDENSE: i64 = DO_CONDENSE;
pub const DC_NORMALIZE: i64 = DO_NORMALIZE;
pub const DC_EVAL_ANSWERS: i64 = DO_EVAL_ANSWERS;
pub const DC_PARAMOD: i64 = DO_PARAMOD | ARG1_CNF | ARG2_CNF;
pub const DC_SIM_PARAMOD: i64 = DO_SIM_PARAMOD | ARG1_CNF | ARG2_CNF;
pub const DC_ORDERED_FACTOR: i64 = DO_ORDERED_FACTOR | ARG1_CNF;
pub const DC_EQ_FACTOR: i64 = DO_EQ_FACTOR | ARG1_CNF;
pub const DC_EQ_RES: i64 = DO_EQ_RES | ARG1_CNF;
pub const DC_DIS_EQ_DECOMPOSE: i64 = DO_DIS_EQ_DECOMPOSE | ARG1_CNF;
pub const DC_SAT_GEN: i64 = DO_SAT_GEN | ARG1_CNF;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DerivationParentRef {
    Clause(ClauseDerivationRef),
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

/// Extracts the clause-side parent references from a C-shaped derivation stack.
///
/// The returned count matches C `DerivStackExtractParents`: it counts only
/// parents that are direct opcode arguments. Additional `DCACRes` parents from
/// `ac_axioms` are appended but not included in the count.
///
/// # Panics
///
/// Panics if an opcode-declared argument is missing, has the wrong Rust stack
/// entry shape, or if a `DCACRes` numeric argument requests more AC axioms than
/// the caller supplied.
#[must_use]
pub fn deriv_stack_extract_parents(
    derivation: Option<&PStack<DerivationEntry>>,
    ac_axioms: &[ClauseDerivationRef],
) -> (Vec<DerivationParentRef>, usize) {
    let Some(derivation) = derivation else {
        return (Vec::new(), 0);
    };

    let mut parents = Vec::new();
    let mut direct_parent_count = 0;
    let mut numarg1 = 0;
    let entries = derivation.as_slice();
    let mut index = 0;

    while index < entries.len() {
        let DerivationEntry::Operation(op) = entries[index] else {
            index += 1;
            continue;
        };
        index += 1;

        if op_has_cnf_arg1(op) {
            parents.push(read_parent_arg(entries, &mut index));
            direct_parent_count += 1;
        } else if op_has_fof_arg1(op) {
            skip_arg(entries, &mut index);
            direct_parent_count += 1;
        } else if op_has_num_arg1(op) {
            numarg1 = read_numeric_arg(entries, &mut index);
        }

        if op_has_cnf_arg2(op) {
            parents.push(read_parent_arg(entries, &mut index));
            direct_parent_count += 1;
        } else if op_has_fof_arg2(op) {
            skip_arg(entries, &mut index);
            direct_parent_count += 1;
        } else if op_has_num_arg2(op) {
            skip_arg(entries, &mut index);
        }

        if op == DC_AC_RES {
            let ac_count = usize::try_from(numarg1)
                .unwrap_or_else(|_| panic!("DCACRes parent count must be non-negative"));
            assert!(
                ac_count <= ac_axioms.len(),
                "DCACRes parent count exceeds supplied AC axioms"
            );
            parents.extend(
                ac_axioms[..ac_count]
                    .iter()
                    .copied()
                    .map(DerivationParentRef::Clause),
            );
        }
    }

    (parents, direct_parent_count)
}

#[must_use]
pub fn deriv_stack_indicates_initial_clause(derivation: Option<&PStack<DerivationEntry>>) -> bool {
    let Some(derivation) = derivation else {
        return true;
    };

    let entries = derivation.as_slice();
    let mut index = 0;

    while index < entries.len() {
        let DerivationEntry::Operation(op) = entries[index] else {
            index += 1;
            continue;
        };
        index += 1;

        if op_has_cnf_arg1(op) {
            return false;
        } else if op_has_arg1(op) {
            skip_arg(entries, &mut index);
        }

        if op_has_cnf_arg2(op) {
            return false;
        } else if op_has_arg2(op) {
            skip_arg(entries, &mut index);
        }

        if op == DC_AC_RES {
            return false;
        }
    }

    true
}

#[must_use]
pub fn deriv_stack_count_search_inferences(
    derivation: Option<&PStack<DerivationEntry>>,
) -> (u64, u64) {
    let Some(derivation) = derivation else {
        return (0, 0);
    };

    let entries = derivation.as_slice();
    let mut index = 0;
    let mut generating_count = 0;
    let mut simplifying_count = 0;

    while index < entries.len() {
        let DerivationEntry::Operation(op) = entries[index] else {
            index += 1;
            continue;
        };
        index += 1;

        if op_has_arg1(op) {
            skip_arg(entries, &mut index);
        }
        if op_has_arg2(op) {
            skip_arg(entries, &mut index);
        }

        match op {
            DC_PARAMOD | DC_SIM_PARAMOD | DC_ORDERED_FACTOR | DC_EQ_FACTOR | DC_EQ_RES => {
                generating_count += 1;
            }
            DC_REWRITE | DC_UNFOLD | DC_APPLY_DEF | DC_CONTEXT_SR | DC_DES_EQ_RES | DC_SR
            | DC_AC_RES | DC_CONDENSE | DC_NORMALIZE | DC_EVAL_ANSWERS => {
                simplifying_count += 1;
            }
            _ => {}
        }
    }

    (generating_count, simplifying_count)
}

/// Returns the C `DerivationStackTSTPPrint` expression for represented
/// clause-side derivation stacks.
///
/// Formula parents and signature-owned AC axiom expansion remain with the
/// later formula/signature proof-object owner.
///
/// # Panics
///
/// Panics if an opcode-declared clause argument is missing or has the wrong
/// Rust stack entry shape.
#[must_use]
pub fn deriv_stack_tstp_string(derivation: Option<&PStack<DerivationEntry>>) -> Option<String> {
    let derivation = derivation?;
    let entries = derivation.as_slice();
    let mut subexpressions = Vec::new();
    let mut index = 0;
    while index < entries.len() {
        subexpressions.push(index);
        let DerivationEntry::Operation(op) = entries[index] else {
            index += 1;
            continue;
        };
        index += 1;
        if op_has_arg1(op) {
            index += 1;
        }
        if op_has_arg2(op) {
            index += 1;
        }
    }

    let mut rendered = String::new();
    let mut extra_cnf_args = Vec::new();
    for &start in subexpressions.iter().rev() {
        let Some(DerivationEntry::Operation(op)) = entries.get(start) else {
            continue;
        };
        match *op {
            DC_CNF_QUOTE => {}
            DC_CNF_ADD_ARG => {
                extra_cnf_args.push(derivation_clause_arg(entries, start + 1));
            }
            op if op_code(op) == DO_INTRO_DEF => {
                rendered.push_str(derivation_op_id(op));
            }
            op => {
                let status = derivation_op_status(op).unwrap_or("unknown");
                write!(
                    &mut rendered,
                    "inference({},[status({})],[",
                    derivation_op_id(op),
                    status
                )
                .expect("writing to String cannot fail");
            }
        }
    }

    for &start in &subexpressions {
        let Some(DerivationEntry::Operation(op)) = entries.get(start) else {
            continue;
        };
        if *op == DC_CNF_ADD_ARG {
            continue;
        }
        if op_has_cnf_arg1(*op) {
            if start != 0 {
                rendered.push_str(", ");
            }
            write_derivation_clause_ref(&mut rendered, derivation_clause_arg(entries, start + 1));
            if op_has_cnf_arg2(*op) {
                rendered.push_str(", ");
                write_derivation_clause_ref(
                    &mut rendered,
                    derivation_clause_arg(entries, start + 2),
                );
            }
        }
        while let Some(parent) = extra_cnf_args.pop() {
            rendered.push_str(", ");
            write_derivation_clause_ref(&mut rendered, parent);
        }
        match *op {
            DC_CNF_QUOTE => {}
            op if op_code(op) == DO_INTRO_DEF => {}
            op => {
                if let Some(theory) = derivation_op_theory(op) {
                    write!(&mut rendered, ", theory({theory})")
                        .expect("writing to String cannot fail");
                }
                rendered.push_str("])");
            }
        }
    }

    Some(rendered)
}

#[must_use]
pub fn demodulator_clause_refs(demodulator: RewriteDemodulator) -> Vec<ClauseDerivationRef> {
    let id = demodulator.id();
    let mut refs = Vec::with_capacity(2);
    if let Ok(ident) = i64::try_from(id) {
        refs.push(ClauseDerivationRef::new(ident, 0));
    }
    if let Ok(id) = i128::try_from(id) {
        if let Ok(negative_ident) = i64::try_from(1_i128 - id) {
            refs.push(ClauseDerivationRef::new(negative_ident, 0));
        }
    }
    refs
}

fn derivation_clause_arg(entries: &[DerivationEntry], index: usize) -> ClauseDerivationRef {
    match entries
        .get(index)
        .unwrap_or_else(|| panic!("derivation clause argument is missing"))
    {
        DerivationEntry::ClauseParent(parent) => *parent,
        DerivationEntry::Demodulator(demodulator) => {
            ClauseDerivationRef::new(demodulator.id().try_into().unwrap_or(i64::MAX), 0)
        }
        DerivationEntry::Operation(_) | DerivationEntry::NumericArg(_) => {
            panic!("derivation clause argument has the wrong entry shape")
        }
    }
}

fn write_derivation_clause_ref(output: &mut String, parent: ClauseDerivationRef) {
    write!(output, "c_0_{}", parent.ident()).expect("writing to String cannot fail");
}

const fn derivation_op_id(op: i64) -> &'static str {
    match op_code(op) {
        DO_NOP => "NOP",
        DO_QUOTE => "QUOTE",
        DO_ADD_CNF_ARG => "AddArg",
        DO_EVAL_GC => "evalgc",
        DO_REWRITE | DO_UNFOLD => "rw",
        DO_LOCAL_REWRITE => "local_rw",
        DO_APPLY_DEF => "apply_def",
        DO_CONTEXT_SR => "csr",
        DO_DES_EQ_RES | DO_EQ_RES => "er",
        DO_SR => "sr",
        DO_AC_RES => "ar",
        DO_CONDENSE => "condense",
        DO_NORMALIZE => "cn",
        DO_EVAL_ANSWERS => "eval_answer_literal",
        15 => "assume_negation",
        16 => "fof_simplification",
        17 => "fof_nnf",
        18 => "shift_quantors",
        19 => "variable_rename",
        20 => "skolemize",
        21 => "distribute",
        22 => "add_answer_literal",
        23 => "epxand_distinct",
        DO_PARAMOD => "pm",
        DO_SIM_PARAMOD => "spm",
        DO_ORDERED_FACTOR => "of",
        DO_EQ_FACTOR => "ef",
        DO_DIS_EQ_DECOMPOSE => "diseq_decomp",
        DO_SAT_GEN => "cdclpropres",
        31 => "pred_elim_resolve",
        32 => "split_equiv",
        DO_INTRO_DEF => "introduced(definition)",
        34 => "split_conjunct",
        35 => "lift_bool_eq",
        36 => "lift_lambdas",
        37 => "fool_unroll",
        38 => "lift_ite",
        39 => "eliminate_boolean_vars",
        40 => "dynamic_cnf",
        41 => "flex_resolve",
        42 => "arg_cong",
        43 => "neg_ext",
        44 => "pos_ext",
        45 => "ext_sup",
        46 => "ext_eqres",
        47 => "ext_eqfact",
        48 => "recognize_injectivity",
        49 => "introduce_choice_axiom",
        50 => "eliminate_leibniz_eq",
        51 => "primitive_enumeration",
        52 => "choice_inst",
        53 => "trigger",
        54 => "prune_arg",
        _ => "unknown",
    }
}

const fn derivation_op_status(op: i64) -> Option<&'static str> {
    match op_code(op) {
        DO_NOP | DO_QUOTE | DO_INTRO_DEF => None,
        DO_ADD_CNF_ARG => Some("NA"),
        15 => Some("cth"),
        20 => Some("esa"),
        3..=14 | 16..=19 | 21..=32 | 34..=54 => Some("thm"),
        _ => Some("unknown"),
    }
}

const fn derivation_op_theory(op: i64) -> Option<&'static str> {
    match op_code(op) {
        DO_ADD_CNF_ARG => Some("NA"),
        DO_EVAL_ANSWERS | 22 => Some("answers"),
        23 => Some("distinct"),
        _ => None,
    }
}

fn read_parent_arg(entries: &[DerivationEntry], index: &mut usize) -> DerivationParentRef {
    let entry = entries
        .get(*index)
        .unwrap_or_else(|| panic!("derivation parent argument is missing"));
    *index += 1;
    match entry {
        DerivationEntry::ClauseParent(parent) => DerivationParentRef::Clause(*parent),
        DerivationEntry::Demodulator(demodulator) => DerivationParentRef::Demodulator(*demodulator),
        DerivationEntry::Operation(_) | DerivationEntry::NumericArg(_) => {
            panic!("derivation parent argument has the wrong entry shape")
        }
    }
}

fn read_numeric_arg(entries: &[DerivationEntry], index: &mut usize) -> i64 {
    let entry = entries
        .get(*index)
        .unwrap_or_else(|| panic!("derivation numeric argument is missing"));
    *index += 1;
    match entry {
        DerivationEntry::NumericArg(value) => *value,
        DerivationEntry::Operation(_)
        | DerivationEntry::ClauseParent(_)
        | DerivationEntry::Demodulator(_) => {
            panic!("derivation numeric argument has the wrong entry shape")
        }
    }
}

fn skip_arg(entries: &[DerivationEntry], index: &mut usize) {
    assert!(
        *index < entries.len(),
        "derivation opcode argument is missing"
    );
    *index += 1;
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
        clause_push_numeric_derivation, deriv_stack_count_search_inferences,
        deriv_stack_extract_parents, deriv_stack_indicates_initial_clause, deriv_stack_tstp_string,
        derivation_entries, get_is_ho, op_code, op_is_generating, set_is_ho, ClauseDerivationRef,
        DerivationEntry, DerivationParentRef, ARG1_CNF, ARG1_NUM, ARG2_CNF, ARG_IS_HO, DC_AC_RES,
        DC_APPLY_DEF, DC_CNF_ADD_ARG, DC_CNF_EVAL_GC, DC_CNF_QUOTE, DC_CONDENSE, DC_CONTEXT_SR,
        DC_DIS_EQ_DECOMPOSE, DC_EQ_FACTOR, DC_EQ_RES, DC_EVAL_ANSWERS, DC_LOCAL_REWRITE,
        DC_ORDERED_FACTOR, DC_PARAMOD, DC_REWRITE, DC_UNFOLD,
    };
    use crate::basics::pstacks::PStack;
    use crate::clauses::clause::Clause;
    use crate::clauses::eqnlist::EqnList;
    use crate::terms::termtypes::RewriteDemodulator;

    #[test]
    fn derivation_code_values_match_c_bit_layout() {
        assert_eq!(ARG1_CNF, 512);
        assert_eq!(ARG1_NUM, 1024);
        assert_eq!(ARG2_CNF, 4096);
        assert_eq!(ARG_IS_HO, 16384);
        assert_eq!(DC_LOCAL_REWRITE, 5);
        assert_eq!(DC_REWRITE, 516);
        assert_eq!(DC_UNFOLD, 518);
        assert_eq!(DC_APPLY_DEF, 263);
        assert_eq!(DC_CNF_QUOTE, 513);
        assert_eq!(DC_ORDERED_FACTOR, 538);
        assert_eq!(DC_EQ_FACTOR, 539);
        assert_eq!(DC_EQ_RES, 540);
        assert_eq!(DC_AC_RES, 1035);
        assert_eq!(DC_EVAL_ANSWERS, 14);
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

    #[test]
    fn deriv_stack_extract_parents_preserves_direct_count_and_ac_axiom_shape() {
        let first = ClauseDerivationRef::new(10, 1);
        let second = ClauseDerivationRef::new(11, 2);
        let ac_first = ClauseDerivationRef::new(20, 3);
        let ac_second = ClauseDerivationRef::new(21, 4);
        let demodulator = RewriteDemodulator::new(99);
        let mut derivation = PStack::new();
        derivation.push(DerivationEntry::Operation(DC_PARAMOD));
        derivation.push(DerivationEntry::ClauseParent(first));
        derivation.push(DerivationEntry::ClauseParent(second));
        derivation.push(DerivationEntry::Operation(DC_REWRITE));
        derivation.push(DerivationEntry::Demodulator(demodulator));
        derivation.push(DerivationEntry::Operation(DC_AC_RES));
        derivation.push(DerivationEntry::NumericArg(2));

        let (parents, direct_count) =
            deriv_stack_extract_parents(Some(&derivation), &[ac_first, ac_second]);

        assert_eq!(direct_count, 3);
        assert_eq!(
            parents,
            vec![
                DerivationParentRef::Clause(first),
                DerivationParentRef::Clause(second),
                DerivationParentRef::Demodulator(demodulator),
                DerivationParentRef::Clause(ac_first),
                DerivationParentRef::Clause(ac_second),
            ]
        );
    }

    #[test]
    fn deriv_stack_indicates_initial_clause_matches_c_cnf_parent_scan() {
        assert!(deriv_stack_indicates_initial_clause(None));
        let empty = PStack::new();
        assert!(deriv_stack_indicates_initial_clause(Some(&empty)));

        let mut no_parent_simplification = PStack::new();
        no_parent_simplification.push(DerivationEntry::Operation(DC_CONDENSE));
        assert!(deriv_stack_indicates_initial_clause(Some(
            &no_parent_simplification
        )));

        let mut cnf_parent = PStack::new();
        cnf_parent.push(DerivationEntry::Operation(DC_CNF_QUOTE));
        cnf_parent.push(DerivationEntry::ClauseParent(ClauseDerivationRef::new(
            30, 5,
        )));
        assert!(!deriv_stack_indicates_initial_clause(Some(&cnf_parent)));

        let mut ac_res = PStack::new();
        ac_res.push(DerivationEntry::Operation(DC_AC_RES));
        ac_res.push(DerivationEntry::NumericArg(0));
        assert!(!deriv_stack_indicates_initial_clause(Some(&ac_res)));
    }

    #[test]
    fn deriv_stack_count_search_inferences_uses_c_exact_switch_cases() {
        let parent = ClauseDerivationRef::new(40, 6);
        let demodulator = RewriteDemodulator::new(100);
        let mut derivation = PStack::new();
        derivation.push(DerivationEntry::Operation(DC_ORDERED_FACTOR));
        derivation.push(DerivationEntry::ClauseParent(parent));
        derivation.push(DerivationEntry::Operation(DC_REWRITE));
        derivation.push(DerivationEntry::Demodulator(demodulator));
        derivation.push(DerivationEntry::Operation(DC_CONTEXT_SR));
        derivation.push(DerivationEntry::ClauseParent(parent));
        derivation.push(DerivationEntry::Operation(DC_CONDENSE));
        derivation.push(DerivationEntry::Operation(DC_DIS_EQ_DECOMPOSE));
        derivation.push(DerivationEntry::ClauseParent(parent));
        derivation.push(DerivationEntry::Operation(set_is_ho(DC_EQ_RES)));
        derivation.push(DerivationEntry::ClauseParent(parent));

        assert_eq!(
            deriv_stack_count_search_inferences(Some(&derivation)),
            (1, 3)
        );
    }

    #[test]
    fn deriv_stack_tstp_string_matches_c_nested_shape() {
        let mut derivation = PStack::new();
        derivation.push(DerivationEntry::Operation(DC_CNF_QUOTE));
        derivation.push(DerivationEntry::ClauseParent(ClauseDerivationRef::new(
            101, 0,
        )));
        derivation.push(DerivationEntry::Operation(DC_CNF_EVAL_GC));
        derivation.push(DerivationEntry::Operation(DC_EQ_RES));
        derivation.push(DerivationEntry::ClauseParent(ClauseDerivationRef::new(
            102, 0,
        )));

        assert_eq!(
            deriv_stack_tstp_string(Some(&derivation)).as_deref(),
            Some(
                "inference(er,[status(thm)],[inference(evalgc,[status(thm)],[c_0_101]), c_0_102])"
            )
        );
    }

    #[test]
    fn deriv_stack_tstp_string_preserves_cnf_add_arg_stack_order() {
        let mut derivation = PStack::new();
        derivation.push(DerivationEntry::Operation(DC_EQ_RES));
        derivation.push(DerivationEntry::ClauseParent(ClauseDerivationRef::new(
            202, 0,
        )));
        derivation.push(DerivationEntry::Operation(DC_CNF_ADD_ARG));
        derivation.push(DerivationEntry::ClauseParent(ClauseDerivationRef::new(
            201, 0,
        )));

        assert_eq!(
            deriv_stack_tstp_string(Some(&derivation)).as_deref(),
            Some("inference(er,[status(thm)],[c_0_202, c_0_201])")
        );
    }
}
