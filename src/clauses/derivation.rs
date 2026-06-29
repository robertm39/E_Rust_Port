use crate::basics::pstacks::PStack;
use crate::clauses::clause::Clause;
use crate::terms::termtypes::RewriteDemodulator;
use std::fmt::Write as _;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i32)]
pub enum ProofOutput {
    None = 0,
    List = 1,
    Graph1 = 2,
    Graph2 = 3,
    Graph3 = 4,
}

impl ProofOutput {
    #[must_use]
    pub const fn from_c_value(value: i32) -> Option<Self> {
        match value {
            0 => Some(Self::None),
            1 => Some(Self::List),
            2 => Some(Self::Graph1),
            3 => Some(Self::Graph2),
            4 => Some(Self::Graph3),
            _ => None,
        }
    }

    #[must_use]
    pub const fn c_value(self) -> i32 {
        self as i32
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i32)]
pub enum ProofObjectType {
    InvalidObject = -1,
    NoObject = 0,
    SimpleDeriviation = 1,
    DetailedDerivation = 2,
    SingleStepDerivation = 3,
}

impl ProofObjectType {
    #[must_use]
    pub const fn from_c_value(value: i32) -> Option<Self> {
        match value {
            -1 => Some(Self::InvalidObject),
            0 => Some(Self::NoObject),
            1 => Some(Self::SimpleDeriviation),
            2 => Some(Self::DetailedDerivation),
            3 => Some(Self::SingleStepDerivation),
            _ => None,
        }
    }

    #[must_use]
    pub const fn c_value(self) -> i32 {
        self as i32
    }
}

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
pub const DO_NEGATE_CONJECTURE: i64 = 15;
pub const DO_FOF_SIMPLIFY: i64 = 16;
pub const DO_FNNF: i64 = 17;
pub const DO_SHIFT_QUANTORS: i64 = 18;
pub const DO_VAR_RENAME: i64 = 19;
pub const DO_SKOLEMIZE: i64 = 20;
pub const DO_DIST_DISJUNCTIONS: i64 = 21;
pub const DO_ANNO_QUESTION: i64 = 22;
pub const DO_EXPAND_DISTINCT: i64 = 23;
pub const DO_PARAMOD: i64 = 24;
pub const DO_SIM_PARAMOD: i64 = 25;
pub const DO_ORDERED_FACTOR: i64 = 26;
pub const DO_EQ_FACTOR: i64 = 27;
pub const DO_EQ_RES: i64 = 28;
pub const DO_DIS_EQ_DECOMPOSE: i64 = 29;
pub const DO_SAT_GEN: i64 = 30;
pub const DO_PE_RESOLVE: i64 = 31;
pub const DO_SPLIT_EQUIV: i64 = 32;
pub const DO_INTRO_DEF: i64 = 33;
pub const DO_SPLIT_CONJUNCT: i64 = 34;
pub const DO_EQ_TO_EQ: i64 = 35;
pub const DO_LIFT_LAMBDAS: i64 = 36;
pub const DO_FOOL_UNROLL: i64 = 37;
pub const DO_LIFT_ITE: i64 = 38;
pub const DO_ELIMINATE_BVAR: i64 = 39;
pub const DO_DYNAMIC_CNF: i64 = 40;
pub const DO_FLEX_RESOLVE: i64 = 41;
pub const DO_ARG_CONG: i64 = 42;
pub const DO_NEG_EXT: i64 = 43;
pub const DO_POS_EXT: i64 = 44;
pub const DO_EXT_SUP: i64 = 45;
pub const DO_EXT_EQ_RES: i64 = 46;
pub const DO_EXT_EQ_FACT: i64 = 47;
pub const DO_INV_REC: i64 = 48;
pub const DO_CHOICE_AX: i64 = 49;
pub const DO_LEIBNIZ_ELIM: i64 = 50;
pub const DO_PRIM_ENUM: i64 = 51;
pub const DO_CHOICE_INST: i64 = 52;
pub const DO_TRIGGER: i64 = 53;
pub const DO_PRUNE_ARG: i64 = 54;

pub const DC_NOP: i64 = DO_NOP;
pub const DC_CNF_QUOTE: i64 = DO_QUOTE | ARG1_CNF;
pub const DC_FOF_QUOTE: i64 = DO_QUOTE | ARG1_FOF;
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
pub const DC_NEGATE_CONJECTURE: i64 = DO_NEGATE_CONJECTURE;
pub const DC_FOF_SIMPLIFY: i64 = DO_FOF_SIMPLIFY;
pub const DC_FNNF: i64 = DO_FNNF;
pub const DC_SHIFT_QUANTORS: i64 = DO_SHIFT_QUANTORS;
pub const DC_VAR_RENAME: i64 = DO_VAR_RENAME;
pub const DC_SKOLEMIZE: i64 = DO_SKOLEMIZE;
pub const DC_DIST_DISJUNCTIONS: i64 = DO_DIST_DISJUNCTIONS;
pub const DC_ANNO_QUESTION: i64 = DO_ANNO_QUESTION;
pub const DC_EXPAND_DISTINCT: i64 = DO_EXPAND_DISTINCT | ARG1_FOF;
pub const DC_PARAMOD: i64 = DO_PARAMOD | ARG1_CNF | ARG2_CNF;
pub const DC_SIM_PARAMOD: i64 = DO_SIM_PARAMOD | ARG1_CNF | ARG2_CNF;
pub const DC_ORDERED_FACTOR: i64 = DO_ORDERED_FACTOR | ARG1_CNF;
pub const DC_EQ_FACTOR: i64 = DO_EQ_FACTOR | ARG1_CNF;
pub const DC_EQ_RES: i64 = DO_EQ_RES | ARG1_CNF;
pub const DC_DIS_EQ_DECOMPOSE: i64 = DO_DIS_EQ_DECOMPOSE | ARG1_CNF;
pub const DC_SAT_GEN: i64 = DO_SAT_GEN | ARG1_CNF;
pub const DC_PE_RESOLVE: i64 = DO_PE_RESOLVE | ARG1_CNF | ARG2_CNF;
pub const DC_SPLIT_EQUIV: i64 = DO_SPLIT_EQUIV | ARG1_FOF;
pub const DC_INTRO_DEF: i64 = DO_INTRO_DEF;
pub const DC_SPLIT_CONJUNCT: i64 = DO_SPLIT_CONJUNCT | ARG1_FOF;
pub const DC_EQ_TO_EQ: i64 = DO_EQ_TO_EQ;
pub const DC_LIFT_LAMBDAS: i64 = DO_LIFT_LAMBDAS | ARG1_FOF;
pub const DC_FOOL_UNROLL: i64 = DO_FOOL_UNROLL;
pub const DC_LIFT_ITE: i64 = DO_LIFT_ITE;
pub const DC_ELIMINATE_BVAR: i64 = DO_ELIMINATE_BVAR;
pub const DC_DYNAMIC_CNF: i64 = DO_DYNAMIC_CNF | ARG1_CNF | ARG_IS_HO;
pub const DC_FLEX_RESOLVE: i64 = DO_FLEX_RESOLVE | ARG_IS_HO;
pub const DC_ARG_CONG: i64 = DO_ARG_CONG | ARG1_CNF | ARG_IS_HO;
pub const DC_NEG_EXT: i64 = DO_NEG_EXT | ARG1_CNF | ARG_IS_HO;
pub const DC_POS_EXT: i64 = DO_POS_EXT | ARG1_CNF | ARG_IS_HO;
pub const DC_EXT_SUP: i64 = DO_EXT_SUP | ARG1_CNF | ARG2_CNF | ARG_IS_HO;
pub const DC_EXT_EQ_RES: i64 = DO_EXT_EQ_RES | ARG1_CNF | ARG_IS_HO;
pub const DC_EXT_EQ_FACT: i64 = DO_EXT_EQ_FACT | ARG1_CNF | ARG_IS_HO;
pub const DC_INV_REC: i64 = DO_INV_REC | ARG1_CNF | ARG_IS_HO;
pub const DC_CHOICE_AX: i64 = DO_CHOICE_AX | ARG_IS_HO;
pub const DC_LEIBNIZ_ELIM: i64 = DO_LEIBNIZ_ELIM | ARG1_CNF | ARG_IS_HO;
pub const DC_PRIM_ENUM: i64 = DO_PRIM_ENUM | ARG1_CNF | ARG_IS_HO;
pub const DC_CHOICE_INST: i64 = DO_CHOICE_INST | ARG1_CNF | ARG2_CNF | ARG_IS_HO;
pub const DC_TRIGGER: i64 = DO_TRIGGER | ARG1_CNF | ARG2_CNF | ARG_IS_HO;
pub const DC_PRUNE_ARG: i64 = DO_PRUNE_ARG | ARG_IS_HO;

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

/// Pushes the C `ClausePushACResDerivation` stack entries.
///
/// The C helper stores only `PStackGetSP(sig->ac_axioms)` as the numeric
/// argument. Rust keeps the signature-owned AC axiom list outside this helper
/// until signature/proof-state ownership is complete.
///
/// # Panics
///
/// Panics if `ac_axiom_count` does not fit in a C signed long equivalent.
pub fn clause_push_ac_res_derivation(clause: &mut Clause, ac_axiom_count: usize) {
    let count = i64::try_from(ac_axiom_count)
        .unwrap_or_else(|_| panic!("AC axiom count must fit in signed long"));
    clause_push_numeric_derivation(clause, DC_AC_RES, count);
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

/// Returns the C `DerivationStackPCLPrint` expression for represented
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
pub fn deriv_stack_pcl_string(derivation: Option<&PStack<DerivationEntry>>) -> Option<String> {
    let derivation = derivation?;
    let entries = derivation.as_slice();
    let subexpressions = derivation_subexpression_starts(entries);

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
            op if op_code(op) == DO_INTRO_DEF => rendered.push_str("introduced"),
            op => {
                write!(&mut rendered, "{}(", derivation_op_id(op))
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
            write_derivation_clause_ident(&mut rendered, derivation_clause_arg(entries, start + 1));
            if op_has_cnf_arg2(*op) {
                rendered.push_str(", ");
                write_derivation_clause_ident(
                    &mut rendered,
                    derivation_clause_arg(entries, start + 2),
                );
            }
        }
        while let Some(parent) = extra_cnf_args.pop() {
            rendered.push_str(", ");
            write_derivation_clause_ident(&mut rendered, parent);
        }
        match *op {
            DC_CNF_QUOTE => {}
            op if op_code(op) == DO_INTRO_DEF => {}
            _ => rendered.push(')'),
        }
    }

    Some(rendered)
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
    let subexpressions = derivation_subexpression_starts(entries);

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

fn derivation_subexpression_starts(entries: &[DerivationEntry]) -> Vec<usize> {
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
    subexpressions
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

fn write_derivation_clause_ident(output: &mut String, parent: ClauseDerivationRef) {
    write!(output, "{}", parent.ident()).expect("writing to String cannot fail");
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
        DO_NEGATE_CONJECTURE => "assume_negation",
        DO_FOF_SIMPLIFY => "fof_simplification",
        DO_FNNF => "fof_nnf",
        DO_SHIFT_QUANTORS => "shift_quantors",
        DO_VAR_RENAME => "variable_rename",
        DO_SKOLEMIZE => "skolemize",
        DO_DIST_DISJUNCTIONS => "distribute",
        DO_ANNO_QUESTION => "add_answer_literal",
        DO_EXPAND_DISTINCT => "epxand_distinct",
        DO_PARAMOD => "pm",
        DO_SIM_PARAMOD => "spm",
        DO_ORDERED_FACTOR => "of",
        DO_EQ_FACTOR => "ef",
        DO_DIS_EQ_DECOMPOSE => "diseq_decomp",
        DO_SAT_GEN => "cdclpropres",
        DO_PE_RESOLVE => "pred_elim_resolve",
        DO_SPLIT_EQUIV => "split_equiv",
        DO_INTRO_DEF => "introduced(definition)",
        DO_SPLIT_CONJUNCT => "split_conjunct",
        DO_EQ_TO_EQ => "lift_bool_eq",
        DO_LIFT_LAMBDAS => "lift_lambdas",
        DO_FOOL_UNROLL => "fool_unroll",
        DO_LIFT_ITE => "lift_ite",
        DO_ELIMINATE_BVAR => "eliminate_boolean_vars",
        DO_DYNAMIC_CNF => "dynamic_cnf",
        DO_FLEX_RESOLVE => "flex_resolve",
        DO_ARG_CONG => "arg_cong",
        DO_NEG_EXT => "neg_ext",
        DO_POS_EXT => "pos_ext",
        DO_EXT_SUP => "ext_sup",
        DO_EXT_EQ_RES => "ext_eqres",
        DO_EXT_EQ_FACT => "ext_eqfact",
        DO_INV_REC => "recognize_injectivity",
        DO_CHOICE_AX => "introduce_choice_axiom",
        DO_LEIBNIZ_ELIM => "eliminate_leibniz_eq",
        DO_PRIM_ENUM => "primitive_enumeration",
        DO_CHOICE_INST => "choice_inst",
        DO_TRIGGER => "trigger",
        DO_PRUNE_ARG => "prune_arg",
        _ => "unknown",
    }
}

const fn derivation_op_status(op: i64) -> Option<&'static str> {
    match op_code(op) {
        DO_NOP | DO_QUOTE | DO_INTRO_DEF => None,
        DO_ADD_CNF_ARG => Some("NA"),
        DO_NEGATE_CONJECTURE => Some("cth"),
        DO_SKOLEMIZE => Some("esa"),
        DO_EVAL_GC..=DO_EVAL_ANSWERS
        | DO_FOF_SIMPLIFY..=DO_VAR_RENAME
        | DO_DIST_DISJUNCTIONS..=DO_SPLIT_EQUIV
        | DO_SPLIT_CONJUNCT..=DO_PRUNE_ARG => Some("thm"),
        _ => Some("unknown"),
    }
}

const fn derivation_op_theory(op: i64) -> Option<&'static str> {
    match op_code(op) {
        DO_ADD_CNF_ARG => Some("NA"),
        DO_EVAL_ANSWERS | DO_ANNO_QUESTION => Some("answers"),
        DO_EXPAND_DISTINCT => Some("distinct"),
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
        clause_is_dummy_quote, clause_is_eval_gc, clause_push_ac_res_derivation,
        clause_push_derivation, clause_push_numeric_derivation,
        deriv_stack_count_search_inferences, deriv_stack_extract_parents,
        deriv_stack_indicates_initial_clause, deriv_stack_pcl_string, deriv_stack_tstp_string,
        derivation_entries, get_is_ho, op_code, op_is_generating, set_is_ho, ClauseDerivationRef,
        DerivationEntry, DerivationParentRef, ProofObjectType, ProofOutput, ARG1_CNF, ARG1_FOF,
        ARG1_NUM, ARG2_CNF, ARG2_FOF, ARG2_NUM, ARG_IS_HO, DC_AC_RES, DC_ANNO_QUESTION,
        DC_APPLY_DEF, DC_ARG_CONG, DC_CHOICE_AX, DC_CHOICE_INST, DC_CNF_ADD_ARG, DC_CNF_EVAL_GC,
        DC_CNF_QUOTE, DC_CONDENSE, DC_CONTEXT_SR, DC_DES_EQ_RES, DC_DIST_DISJUNCTIONS,
        DC_DIS_EQ_DECOMPOSE, DC_DYNAMIC_CNF, DC_ELIMINATE_BVAR, DC_EQ_FACTOR, DC_EQ_RES,
        DC_EQ_TO_EQ, DC_EVAL_ANSWERS, DC_EXPAND_DISTINCT, DC_EXT_EQ_FACT, DC_EXT_EQ_RES,
        DC_EXT_SUP, DC_FLEX_RESOLVE, DC_FNNF, DC_FOF_QUOTE, DC_FOF_SIMPLIFY, DC_FOOL_UNROLL,
        DC_INTRO_DEF, DC_INV_REC, DC_LEIBNIZ_ELIM, DC_LIFT_ITE, DC_LIFT_LAMBDAS, DC_LOCAL_REWRITE,
        DC_NEGATE_CONJECTURE, DC_NEG_EXT, DC_NOP, DC_NORMALIZE, DC_ORDERED_FACTOR, DC_PARAMOD,
        DC_PE_RESOLVE, DC_POS_EXT, DC_PRIM_ENUM, DC_PRUNE_ARG, DC_REWRITE, DC_SAT_GEN,
        DC_SHIFT_QUANTORS, DC_SIM_PARAMOD, DC_SKOLEMIZE, DC_SPLIT_CONJUNCT, DC_SPLIT_EQUIV, DC_SR,
        DC_TRIGGER, DC_UNFOLD, DC_VAR_RENAME, DO_AC_RES, DO_ADD_CNF_ARG, DO_ANNO_QUESTION,
        DO_APPLY_DEF, DO_ARG_CONG, DO_CHOICE_AX, DO_CHOICE_INST, DO_CONDENSE, DO_CONTEXT_SR,
        DO_DES_EQ_RES, DO_DIST_DISJUNCTIONS, DO_DIS_EQ_DECOMPOSE, DO_DYNAMIC_CNF,
        DO_ELIMINATE_BVAR, DO_EQ_FACTOR, DO_EQ_RES, DO_EQ_TO_EQ, DO_EVAL_ANSWERS, DO_EVAL_GC,
        DO_EXPAND_DISTINCT, DO_EXT_EQ_FACT, DO_EXT_EQ_RES, DO_EXT_SUP, DO_FLEX_RESOLVE, DO_FNNF,
        DO_FOF_SIMPLIFY, DO_FOOL_UNROLL, DO_INTRO_DEF, DO_INV_REC, DO_LEIBNIZ_ELIM, DO_LIFT_ITE,
        DO_LIFT_LAMBDAS, DO_LOCAL_REWRITE, DO_NEGATE_CONJECTURE, DO_NEG_EXT, DO_NOP, DO_NORMALIZE,
        DO_ORDERED_FACTOR, DO_PARAMOD, DO_PE_RESOLVE, DO_POS_EXT, DO_PRIM_ENUM, DO_PRUNE_ARG,
        DO_QUOTE, DO_REWRITE, DO_SAT_GEN, DO_SHIFT_QUANTORS, DO_SIM_PARAMOD, DO_SKOLEMIZE,
        DO_SPLIT_CONJUNCT, DO_SPLIT_EQUIV, DO_SR, DO_TRIGGER, DO_UNFOLD, DO_VAR_RENAME,
    };
    use crate::basics::pstacks::PStack;
    use crate::clauses::clause::Clause;
    use crate::clauses::eqnlist::EqnList;
    use crate::terms::termtypes::RewriteDemodulator;

    #[test]
    fn proof_output_discriminants_match_c_enum() {
        assert_eq!(ProofOutput::None.c_value(), 0);
        assert_eq!(ProofOutput::List.c_value(), 1);
        assert_eq!(ProofOutput::Graph1.c_value(), 2);
        assert_eq!(ProofOutput::Graph2.c_value(), 3);
        assert_eq!(ProofOutput::Graph3.c_value(), 4);
        assert_eq!(ProofOutput::from_c_value(2), Some(ProofOutput::Graph1));
        assert_eq!(ProofOutput::from_c_value(5), None);
    }

    #[test]
    fn proof_object_type_discriminants_match_c_enum_typo() {
        assert_eq!(ProofObjectType::InvalidObject.c_value(), -1);
        assert_eq!(ProofObjectType::NoObject.c_value(), 0);
        assert_eq!(ProofObjectType::SimpleDeriviation.c_value(), 1);
        assert_eq!(ProofObjectType::DetailedDerivation.c_value(), 2);
        assert_eq!(ProofObjectType::SingleStepDerivation.c_value(), 3);
        assert_eq!(
            ProofObjectType::from_c_value(1),
            Some(ProofObjectType::SimpleDeriviation)
        );
        assert_eq!(ProofObjectType::from_c_value(4), None);
    }

    #[test]
    fn derivation_argument_bits_match_c_bit_layout() {
        assert_eq!(ARG1_FOF, 256);
        assert_eq!(ARG1_CNF, 512);
        assert_eq!(ARG1_NUM, 1024);
        assert_eq!(ARG2_FOF, 2048);
        assert_eq!(ARG2_CNF, 4096);
        assert_eq!(ARG2_NUM, 8192);
        assert_eq!(ARG_IS_HO, 16384);
    }

    #[test]
    fn derivation_opcodes_match_c_enum_order() {
        let opcode_values = [
            (DO_NOP, 0),
            (DO_QUOTE, 1),
            (DO_ADD_CNF_ARG, 2),
            (DO_EVAL_GC, 3),
            (DO_REWRITE, 4),
            (DO_LOCAL_REWRITE, 5),
            (DO_UNFOLD, 6),
            (DO_APPLY_DEF, 7),
            (DO_CONTEXT_SR, 8),
            (DO_DES_EQ_RES, 9),
            (DO_SR, 10),
            (DO_AC_RES, 11),
            (DO_CONDENSE, 12),
            (DO_NORMALIZE, 13),
            (DO_EVAL_ANSWERS, 14),
            (DO_NEGATE_CONJECTURE, 15),
            (DO_FOF_SIMPLIFY, 16),
            (DO_FNNF, 17),
            (DO_SHIFT_QUANTORS, 18),
            (DO_VAR_RENAME, 19),
            (DO_SKOLEMIZE, 20),
            (DO_DIST_DISJUNCTIONS, 21),
            (DO_ANNO_QUESTION, 22),
            (DO_EXPAND_DISTINCT, 23),
            (DO_PARAMOD, 24),
            (DO_SIM_PARAMOD, 25),
            (DO_ORDERED_FACTOR, 26),
            (DO_EQ_FACTOR, 27),
            (DO_EQ_RES, 28),
            (DO_DIS_EQ_DECOMPOSE, 29),
            (DO_SAT_GEN, 30),
            (DO_PE_RESOLVE, 31),
            (DO_SPLIT_EQUIV, 32),
            (DO_INTRO_DEF, 33),
            (DO_SPLIT_CONJUNCT, 34),
            (DO_EQ_TO_EQ, 35),
            (DO_LIFT_LAMBDAS, 36),
            (DO_FOOL_UNROLL, 37),
            (DO_LIFT_ITE, 38),
            (DO_ELIMINATE_BVAR, 39),
            (DO_DYNAMIC_CNF, 40),
            (DO_FLEX_RESOLVE, 41),
            (DO_ARG_CONG, 42),
            (DO_NEG_EXT, 43),
            (DO_POS_EXT, 44),
            (DO_EXT_SUP, 45),
            (DO_EXT_EQ_RES, 46),
            (DO_EXT_EQ_FACT, 47),
            (DO_INV_REC, 48),
            (DO_CHOICE_AX, 49),
            (DO_LEIBNIZ_ELIM, 50),
            (DO_PRIM_ENUM, 51),
            (DO_CHOICE_INST, 52),
            (DO_TRIGGER, 53),
            (DO_PRUNE_ARG, 54),
        ];
        for (actual, expected) in opcode_values {
            assert_eq!(actual, expected);
        }
    }

    #[test]
    fn derivation_codes_match_c_bit_layout() {
        let derivation_code_values = [
            (DC_NOP, 0),
            (DC_CNF_QUOTE, 513),
            (DC_FOF_QUOTE, 257),
            (DC_CNF_ADD_ARG, 514),
            (DC_CNF_EVAL_GC, 3),
            (DC_REWRITE, 516),
            (DC_LOCAL_REWRITE, 5),
            (DC_UNFOLD, 518),
            (DC_APPLY_DEF, 263),
            (DC_CONTEXT_SR, 520),
            (DC_SR, 522),
            (DC_DES_EQ_RES, 9),
            (DC_AC_RES, 1035),
            (DC_CONDENSE, 12),
            (DC_NORMALIZE, 13),
            (DC_EVAL_ANSWERS, 14),
            (DC_NEGATE_CONJECTURE, 15),
            (DC_FOF_SIMPLIFY, 16),
            (DC_FNNF, 17),
            (DC_SHIFT_QUANTORS, 18),
            (DC_VAR_RENAME, 19),
            (DC_SKOLEMIZE, 20),
            (DC_DIST_DISJUNCTIONS, 21),
            (DC_ANNO_QUESTION, 22),
            (DC_EXPAND_DISTINCT, 279),
            (DC_PARAMOD, 4632),
            (DC_SIM_PARAMOD, 4633),
            (DC_ORDERED_FACTOR, 538),
            (DC_EQ_FACTOR, 539),
            (DC_EQ_RES, 540),
            (DC_DIS_EQ_DECOMPOSE, 541),
            (DC_SAT_GEN, 542),
            (DC_PE_RESOLVE, 4639),
            (DC_SPLIT_EQUIV, 288),
            (DC_INTRO_DEF, 33),
            (DC_SPLIT_CONJUNCT, 290),
            (DC_EQ_TO_EQ, 35),
            (DC_LIFT_LAMBDAS, 292),
            (DC_FOOL_UNROLL, 37),
            (DC_LIFT_ITE, 38),
            (DC_ELIMINATE_BVAR, 39),
            (DC_DYNAMIC_CNF, 16936),
            (DC_FLEX_RESOLVE, 16425),
            (DC_ARG_CONG, 16938),
            (DC_NEG_EXT, 16939),
            (DC_POS_EXT, 16940),
            (DC_EXT_SUP, 21037),
            (DC_EXT_EQ_RES, 16942),
            (DC_EXT_EQ_FACT, 16943),
            (DC_INV_REC, 16944),
            (DC_CHOICE_AX, 16433),
            (DC_LEIBNIZ_ELIM, 16946),
            (DC_PRIM_ENUM, 16947),
            (DC_CHOICE_INST, 21044),
            (DC_TRIGGER, 21045),
            (DC_PRUNE_ARG, 16438),
        ];
        for (actual, expected) in derivation_code_values {
            assert_eq!(actual, expected);
        }
    }

    #[test]
    fn derivation_opcode_helpers_match_c_macros() {
        assert_eq!(op_code(DC_EQ_RES), 28);
        assert!(op_is_generating(DC_EQ_FACTOR));
        assert!(op_is_generating(DC_SAT_GEN));
        assert!(!op_is_generating(DC_PE_RESOLVE));
        assert!(get_is_ho(set_is_ho(DC_EQ_RES)));
        assert!(get_is_ho(DC_EXT_SUP));
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
    fn clause_push_ac_res_derivation_records_current_ac_axiom_count() {
        let mut clause = Clause::alloc(EqnList::new());

        clause_push_ac_res_derivation(&mut clause, 2);

        assert_eq!(
            derivation_entries(&clause),
            &[
                DerivationEntry::Operation(DC_AC_RES),
                DerivationEntry::NumericArg(2),
            ]
        );
    }

    #[test]
    fn clause_push_ac_res_derivation_allows_empty_ac_axiom_stack() {
        let mut clause = Clause::alloc(EqnList::new());

        clause_push_ac_res_derivation(&mut clause, 0);

        assert_eq!(
            derivation_entries(&clause),
            &[
                DerivationEntry::Operation(DC_AC_RES),
                DerivationEntry::NumericArg(0),
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
    fn deriv_stack_pcl_string_matches_c_nested_shape() {
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
            deriv_stack_pcl_string(Some(&derivation)).as_deref(),
            Some("er(evalgc(101), 102)")
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

    #[test]
    fn deriv_stack_pcl_string_preserves_cnf_add_arg_stack_order() {
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
            deriv_stack_pcl_string(Some(&derivation)).as_deref(),
            Some("er(202, 201)")
        );
    }
}
