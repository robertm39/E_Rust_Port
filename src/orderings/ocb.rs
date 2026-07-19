//! Ordering-control block basics from `cto_ocb`.

use crate::basics::error::Diagnostic;
use crate::basics::partial_orderings::{CompareResult, HoOrderKind};
use crate::basics::pstacks::PStackPointer;
use crate::heuristics::to_params::{
    LiteralCmp, TermOrdering, DEFAULT_DB_WEIGHT, DEFAULT_LAMBDA_WEIGHT,
};
use crate::terms::functypes::FunCode;
use crate::terms::signature::{Signature, SIG_TRUE_CODE};
use crate::terms::simpletypes::{Type, TypeUniqueId};
use crate::terms::termbanks::TermBank;
use crate::terms::termtypes::{term_deref, term_identity_id, DerefType, Term};
use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::io::{self, Write};

pub const OCB_FUN_DEFAULT_WEIGHT: i64 = 1;
pub const W_DEFAULT_WEIGHT: i64 = 1;

pub const TO_NAMES: [&str; 10] = [
    "NoOrdering",
    "Optimize",
    "KBO",
    "KBO6",
    "LPO",
    "LPOCopy",
    "LPO4",
    "LPO4Copy",
    "RPO",
    "Empty",
];

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OrderControlBlock {
    pub ordering_type: TermOrdering,
    pub sig_size: FunCode,
    pub weights: Option<Vec<i64>>,
    pub var_weight: i64,
    pub lam_weight: i64,
    pub db_weight: i64,
    pub prec_weights: Option<Vec<i64>>,
    pub precedence: Option<Vec<CompareResult>>,
    pub lit_cmp: LiteralCmp,
    pub rewrite_strong_rhs_inst: bool,
    pub wb: i64,
    pub pos_bal: i64,
    pub neg_bal: i64,
    pub max_var: i64,
    pub vb_size: usize,
    pub vb: Vec<i32>,
    pub ho_vb: BTreeMap<usize, i64>,
    pub ho_order_kind: HoOrderKind,
    min_constants: BTreeMap<TypeUniqueId, FunCode>,
    state_stack: Vec<FunCode>,
}

impl OrderControlBlock {
    /// Allocate an order-control block for a concrete ordering type.
    ///
    /// # Panics
    ///
    /// Panics if called with `NoOrdering` or `Optimize`, matching the C
    /// assertion that OCB allocation receives a concrete ordering.
    #[must_use]
    pub fn alloc(
        ordering_type: TermOrdering,
        prec_by_weight: bool,
        signature: &Signature,
        ho_order_kind: HoOrderKind,
    ) -> Self {
        let sig_size = signature.f_count();
        let vb_size = if ho_order_kind == HoOrderKind::LambdaOrder {
            1
        } else {
            64
        };
        let mut handle = Self {
            ordering_type,
            sig_size,
            weights: None,
            var_weight: 1,
            lam_weight: DEFAULT_LAMBDA_WEIGHT,
            db_weight: DEFAULT_DB_WEIGHT,
            prec_weights: None,
            precedence: None,
            lit_cmp: LiteralCmp::Normal,
            rewrite_strong_rhs_inst: false,
            wb: 0,
            pos_bal: 0,
            neg_bal: 0,
            max_var: 0,
            vb_size,
            vb: vec![0; vb_size],
            ho_vb: BTreeMap::new(),
            ho_order_kind,
            min_constants: BTreeMap::new(),
            state_stack: Vec::new(),
        };

        match ordering_type {
            TermOrdering::Kbo | TermOrdering::Kbo6 => {
                handle.weights = Some(vec![OCB_FUN_DEFAULT_WEIGHT; weights_size(sig_size)]);
                handle.alloc_precedence(prec_by_weight);
            }
            TermOrdering::Lpo
            | TermOrdering::LpoCopy
            | TermOrdering::Lpo4
            | TermOrdering::Lpo4Copy
            | TermOrdering::Rpo => handle.alloc_precedence(prec_by_weight),
            TermOrdering::Empty => {}
            TermOrdering::NoOrdering | TermOrdering::Optimize => {
                panic!("OCBAlloc called with non-concrete ordering type")
            }
        }

        handle
    }

    #[must_use]
    pub fn precedence_state(&self) -> PStackPointer {
        PStackPointer::try_from(self.state_stack.len()).unwrap_or(PStackPointer::MAX)
    }

    #[must_use]
    pub fn fun_weight(&self, symbol: FunCode) -> i64 {
        let Some(weights) = &self.weights else {
            return OCB_FUN_DEFAULT_WEIGHT;
        };
        if symbol <= self.sig_size {
            weights[fcode_index(symbol)]
        } else {
            OCB_FUN_DEFAULT_WEIGHT
        }
    }

    /// Set a stored KBO function-symbol weight.
    ///
    /// # Panics
    ///
    /// Panics if this OCB has no weight vector, or if `symbol` is outside the
    /// saved signature range.
    pub fn set_fun_weight(&mut self, symbol: FunCode, weight: i64) {
        let Some(weights) = &mut self.weights else {
            panic!("function weights are not allocated for this ordering")
        };
        weights[fcode_index(symbol)] = weight;
    }

    /// Copy generated KBO weights into this OCB.
    ///
    /// # Panics
    ///
    /// Panics if this OCB has no weight vector, or if `weights` does not match
    /// the saved f-code-indexed signature size.
    pub fn install_weights(&mut self, weights: &[i64]) {
        let Some(target) = &mut self.weights else {
            panic!("function weights are not allocated for this ordering")
        };
        assert_eq!(
            target.len(),
            weights.len(),
            "generated weight vector must match OCB signature size"
        );
        target.copy_from_slice(weights);
    }

    #[must_use]
    pub fn fun_prec_weight(&self, symbol: FunCode) -> i64 {
        if let Some(prec_weights) = &self.prec_weights {
            if symbol <= self.sig_size {
                return prec_weights[fcode_index(symbol)];
            }
        }
        -symbol
    }

    /// Set a total-precedence weight for one stored symbol.
    ///
    /// # Panics
    ///
    /// Panics if this OCB has no precedence-weight vector, or if `symbol` is
    /// outside the saved signature range.
    pub fn set_fun_prec_weight(&mut self, symbol: FunCode, weight: i64) {
        let Some(prec_weights) = &mut self.prec_weights else {
            panic!("precedence weights are not allocated for this ordering")
        };
        prec_weights[fcode_index(symbol)] = weight;
    }

    /// Copy generated total-precedence weights into this OCB.
    ///
    /// # Panics
    ///
    /// Panics if this OCB has no precedence-weight vector, or if `weights` does
    /// not match the saved f-code-indexed signature size.
    pub fn install_prec_weights(&mut self, weights: &[i64]) {
        let Some(target) = &mut self.prec_weights else {
            panic!("precedence weights are not allocated for this ordering")
        };
        assert_eq!(
            target.len(),
            weights.len(),
            "generated precedence vector must match OCB signature size"
        );
        target.copy_from_slice(weights);
    }

    /// Compare two function symbols through the OCB precedence state.
    ///
    /// # Panics
    ///
    /// Panics if either f-code is non-positive, or if this OCB has neither
    /// precedence weights nor a precedence matrix for distinct non-special
    /// symbols.
    #[must_use]
    pub fn fun_compare(
        &self,
        signature: &Signature,
        left: FunCode,
        right: FunCode,
    ) -> CompareResult {
        assert!(left > 0 && right > 0, "f-codes must be positive");
        if left == right {
            return CompareResult::Equal;
        }
        if left == SIG_TRUE_CODE {
            return CompareResult::Lesser;
        }
        if right == SIG_TRUE_CODE {
            return CompareResult::Greater;
        }

        let distinct_delta = i64::from(is_distinct_symbol(signature, right))
            - i64::from(is_distinct_symbol(signature, left));
        if distinct_delta != 0 {
            return q_to_part_i64(distinct_delta);
        }

        if self.prec_weights.is_some() {
            return q_to_part_i64(self.fun_prec_weight(left) - self.fun_prec_weight(right));
        }
        self.fun_compare_matrix(left, right)
    }

    /// Compare two unequal symbols through the precedence matrix.
    ///
    /// # Panics
    ///
    /// Panics if the symbols are equal, or if this OCB was allocated with
    /// precedence weights instead of a matrix.
    #[must_use]
    pub fn fun_compare_matrix(&self, left: FunCode, right: FunCode) -> CompareResult {
        assert!(left != right, "OCBFunCompare handles equal symbols first");
        let Some(precedence) = &self.precedence else {
            panic!("precedence matrix is not allocated for this ordering")
        };

        if left <= self.sig_size {
            if right <= self.sig_size {
                return precedence[matrix_index(self.sig_size, left, right)];
            }
            return CompareResult::Greater;
        }
        if right <= self.sig_size {
            return CompareResult::Lesser;
        }
        q_to_part_i64(right - left)
    }

    /// Insert a matrix precedence tuple and close it transitively.
    ///
    /// # Panics
    ///
    /// Panics if no precedence matrix is allocated, if either f-code is outside
    /// the saved signature range, or if `relation` is not a concrete
    /// equal/greater/lesser relation.
    pub fn precedence_add_tuple(
        &mut self,
        signature: &Signature,
        left: FunCode,
        right: FunCode,
        relation: CompareResult,
    ) -> PStackPointer {
        assert!(
            self.precedence.is_some(),
            "precedence matrix is required for tuple insertion"
        );
        assert!(left <= self.sig_size, "left symbol must fit OCB signature");
        assert!(
            right <= self.sig_size,
            "right symbol must fit OCB signature"
        );
        assert!(
            relation != CompareResult::Uncomparable,
            "uncomparable tuples are not inserted"
        );
        assert!(
            matches!(
                relation,
                CompareResult::Equal | CompareResult::Greater | CompareResult::Lesser
            ),
            "only concrete precedence relations are inserted"
        );

        let old = self.precedence_state();
        let current = self.fun_compare(signature, left, right);
        if current == relation {
            return old;
        }
        if current != CompareResult::Uncomparable {
            return 0;
        }

        self.state_stack.push(left);
        self.state_stack.push(right);
        self.set_matrix_pair(left, right, relation);

        let mut ok = true;
        for symbol in 1..=self.sig_size {
            ok = self.trans_compute(signature, left, right, symbol);
            if !ok {
                break;
            }
            ok = self.trans_compute(signature, symbol, left, right);
            if !ok {
                break;
            }
        }

        if ok {
            1
        } else {
            let failed_right = self
                .state_stack
                .pop()
                .unwrap_or_else(|| panic!("state stack stores right symbol"));
            let failed_left = self
                .state_stack
                .pop()
                .unwrap_or_else(|| panic!("state stack stores left symbol"));
            self.set_matrix_pair(failed_left, failed_right, CompareResult::Uncomparable);
            0
        }
    }

    /// Backtrack the precedence matrix to a previous stack state.
    ///
    /// # Panics
    ///
    /// Panics if `state` is negative, odd, larger than the current state stack,
    /// or does not refer to comparable matrix entries.
    pub fn precedence_backtrack(&mut self, state: PStackPointer) -> bool {
        assert!(state >= 0, "precedence state must be non-negative");
        let target = usize::try_from(state).unwrap_or_else(|_| panic!("state must fit usize"));
        assert!(
            target <= self.state_stack.len() && target % 2 == 0,
            "precedence state must be a previous stack pointer"
        );

        while self.state_stack.len() != target {
            let right = self
                .state_stack
                .pop()
                .unwrap_or_else(|| panic!("state stack stores right symbol"));
            let left = self
                .state_stack
                .pop()
                .unwrap_or_else(|| panic!("state stack stores left symbol"));
            assert_ne!(
                self.fun_compare_matrix(left, right),
                CompareResult::Uncomparable
            );
            self.set_matrix_pair(left, right, CompareResult::Uncomparable);
        }
        !self.state_stack.is_empty()
    }

    #[must_use]
    pub fn min_const(&self, type_: &Type) -> FunCode {
        self.min_constants
            .get(&type_.type_uid())
            .copied()
            .unwrap_or(0)
    }

    pub fn cond_set_min_const(&mut self, type_: &Type, candidate: FunCode) {
        self.min_constants
            .entry(type_.type_uid())
            .or_insert(candidate);
    }

    pub fn set_min_const(&mut self, type_: &Type, candidate: FunCode) {
        self.min_constants.insert(type_.type_uid(), candidate);
    }

    /// Return or create the designated constant for `type_`.
    ///
    /// # Panics
    ///
    /// Panics if a newly-created Skolem symbol cannot be assigned the requested
    /// final type.
    pub fn find_min_const(&mut self, signature: &mut Signature, type_: &Type) -> FunCode {
        let mut candidate = self.min_const(type_);
        if candidate == 0 {
            for symbol in (signature.internal_symbols() + 1)..=signature.f_count() {
                if signature.is_fun_const(symbol)
                    && !signature.is_special(symbol)
                    && signature
                        .get_type(symbol)
                        .is_some_and(|symbol_type| symbol_type == type_)
                    && (candidate == 0
                        || self.fun_compare(signature, symbol, candidate) == CompareResult::Greater)
                {
                    candidate = symbol;
                }
            }
            if candidate == 0 {
                candidate = signature.get_new_skolem_code(0);
                signature
                    .declare_final_type(candidate, type_.clone())
                    .unwrap_or_else(|err| panic!("{err}"));
            }
            self.cond_set_min_const(type_, candidate);
        }
        candidate
    }

    /// Return the designated minimum term for `type_` from a term bank.
    ///
    /// # Errors
    ///
    /// Returns any term-bank diagnostic produced while creating or retrieving
    /// the minimum term.
    ///
    /// # Panics
    ///
    /// Panics under the same Skolem type-declaration failure as
    /// [`Self::find_min_const`].
    pub fn designated_min_term(
        &mut self,
        terms: &mut TermBank,
        type_: &Type,
    ) -> Result<Term, Diagnostic> {
        let min_const = self.find_min_const(terms.signature_mut(), type_);
        terms.create_min_term(min_const)
    }

    /// Return one maximal function symbol occurring in `term`.
    ///
    /// This intentionally preserves the C loop bound that starts traversal at
    /// argument index 1, so argument zero is not inspected.
    ///
    /// # Panics
    ///
    /// Panics if this OCB has no precedence storage, if an inspected argument
    /// slot is uninitialized, or if a variable appears in an inspected
    /// non-root argument and triggers the C-shaped positive-f-code assertion.
    #[must_use]
    pub fn term_max_fun_code(&self, signature: &Signature, term: &Term) -> FunCode {
        assert!(
            self.precedence.is_some() || self.prec_weights.is_some(),
            "term maximum lookup requires OCB precedence storage"
        );
        let mut deref = DerefType::Once;
        let term = term_deref(term, &mut deref);
        if term.is_any_var() {
            return 0;
        }

        let mut result = term.f_code();
        for index in 1..term.arity() {
            let arg = term
                .argument(index)
                .unwrap_or_else(|| panic!("term argument slot must be initialized"));
            let candidate = self.term_max_fun_code(signature, &arg);
            if self.fun_compare(signature, candidate, result) == CompareResult::Greater {
                result = candidate;
            }
        }
        result
    }

    pub fn inc_ho_var_balance(&mut self, term: &Term) {
        let balance = self.ho_vb.entry(term_identity_id(term)).or_insert(0);
        if *balance == 0 {
            self.pos_bal += 1;
        } else if *balance == -1 {
            self.neg_bal -= 1;
        }
        *balance += 1;
        self.wb += self.var_weight;
    }

    pub fn dec_ho_var_balance(&mut self, term: &Term) {
        let balance = self.ho_vb.entry(term_identity_id(term)).or_insert(0);
        if *balance == 0 {
            self.neg_bal += 1;
        } else if *balance == 1 {
            self.pos_bal -= 1;
        }
        *balance -= 1;
        self.wb -= self.var_weight;
    }

    pub fn reset_ho_var_map(&mut self) {
        self.ho_vb.clear();
    }

    /// Print this OCB in C's debug-comment shape.
    ///
    /// C stores the signature pointer in the OCB. Rust keeps that ownership
    /// outside the OCB, so callers pass the live signature explicitly when
    /// they want symbol names and full `OCBFunCompare` behavior.
    ///
    /// # Errors
    ///
    /// Returns any error reported by the output writer or by signature
    /// printing.
    ///
    /// # Panics
    ///
    /// Panics if the stored precedence matrix contains a comparison relation
    /// with no C debug symbol.
    pub fn debug_print(
        &self,
        output: &mut impl Write,
        signature: Option<&Signature>,
    ) -> io::Result<()> {
        writeln!(output, "% ==============OCB-Debug-Information============")?;
        writeln!(output, "% ===============================================")?;
        if let Some(signature) = signature {
            signature.print(output)?;
        } else {
            writeln!(output, "% No sig!")?;
        }
        writeln!(output, "% -----------------------------------------------")?;
        if self.weights.is_some() {
            write!(output, "% Weights:")?;
            for symbol in 1..=self.sig_size {
                if (symbol - 1) % 8 == 0 {
                    write!(output, "\n% ")?;
                }
                if let Some(signature) = signature {
                    let name = signature.find_name(symbol).unwrap_or("<unnamed>");
                    write!(output, " ({name} = {}) ", self.fun_weight(symbol))?;
                } else {
                    write!(output, " ({symbol} = {}) ", self.fun_weight(symbol))?;
                }
            }
            writeln!(output, "\n")?;
        } else {
            writeln!(output, "% No weights!")?;
        }
        writeln!(output, "% -----------------------------------------------")?;
        if self.precedence.is_some() {
            write!(output, "% Precedence Matrix:\n%       ")?;
            for symbol in 1..=self.sig_size {
                write!(output, " {symbol:2} ")?;
            }
            writeln!(output)?;
            for left in 1..=self.sig_size {
                write!(output, "% {left:2}  | ")?;
                for right in 1..=self.sig_size {
                    let relation = self.debug_fun_compare(signature, left, right);
                    let symbol = relation
                        .symbol()
                        .unwrap_or_else(|| panic!("relation {relation:?} has no debug symbol"));
                    write!(output, " {symbol}")?;
                }
                writeln!(output)?;
            }
        } else {
            writeln!(output, "% No precedence!")?;
        }
        writeln!(output, "% ===============================================")
    }

    #[must_use]
    pub fn state_stack(&self) -> &[FunCode] {
        &self.state_stack
    }

    fn alloc_precedence(&mut self, prec_by_weight: bool) {
        if prec_by_weight {
            self.prec_weights = Some(vec![0; weights_size(self.sig_size)]);
            self.precedence = None;
        } else {
            self.precedence = Some(initial_precedence_matrix(self.sig_size));
            self.prec_weights = None;
        }
    }

    fn trans_compute(
        &mut self,
        signature: &Signature,
        left: FunCode,
        middle: FunCode,
        right: FunCode,
    ) -> bool {
        let rel12 = self.fun_compare(signature, left, middle);
        let rel23 = self.fun_compare(signature, middle, right);
        match rel12 {
            CompareResult::Uncomparable => true,
            CompareResult::Equal => {
                rel23 == CompareResult::Uncomparable
                    || self.precedence_add_tuple(signature, left, right, rel23) != 0
            }
            CompareResult::Greater => {
                !(rel23 == CompareResult::Equal || rel23 == CompareResult::Greater)
                    || self.precedence_add_tuple(signature, left, right, CompareResult::Greater)
                        != 0
            }
            CompareResult::Lesser => {
                !(rel23 == CompareResult::Equal || rel23 == CompareResult::Lesser)
                    || self.precedence_add_tuple(signature, left, right, CompareResult::Lesser) != 0
            }
            CompareResult::Unknown
            | CompareResult::NotGreaterEqual
            | CompareResult::NotLessEqual => {
                panic!("unexpected comparison relation in OCB transitive closure")
            }
        }
    }

    fn set_matrix_pair(&mut self, left: FunCode, right: FunCode, relation: CompareResult) {
        let Some(precedence) = &mut self.precedence else {
            panic!("precedence matrix is not allocated for this ordering")
        };
        precedence[matrix_index(self.sig_size, left, right)] = relation;
        precedence[matrix_index(self.sig_size, right, left)] = relation
            .inverse()
            .unwrap_or_else(|| panic!("inserted relation has an inverse"));
    }

    fn debug_fun_compare(
        &self,
        signature: Option<&Signature>,
        left: FunCode,
        right: FunCode,
    ) -> CompareResult {
        if let Some(signature) = signature {
            self.fun_compare(signature, left, right)
        } else if left == right {
            CompareResult::Equal
        } else {
            self.precedence
                .as_ref()
                .unwrap_or_else(|| panic!("precedence matrix is not allocated for this ordering"))
                [matrix_index(self.sig_size, left, right)]
        }
    }
}

fn initial_precedence_matrix(sig_size: FunCode) -> Vec<CompareResult> {
    let size = fcode_index(sig_size)
        .checked_mul(fcode_index(sig_size))
        .unwrap_or_else(|| panic!("precedence matrix size overflow"));
    let mut matrix = vec![CompareResult::Uncomparable; size];
    for symbol in 1..=sig_size {
        matrix[matrix_index(sig_size, symbol, symbol)] = CompareResult::Equal;
    }
    matrix
}

fn matrix_index(sig_size: FunCode, left: FunCode, right: FunCode) -> usize {
    let row = fcode_index(right - 1);
    let col = fcode_index(left - 1);
    row.checked_mul(fcode_index(sig_size))
        .and_then(|base| base.checked_add(col))
        .unwrap_or_else(|| panic!("precedence matrix index overflow"))
}

fn weights_size(sig_size: FunCode) -> usize {
    usize::try_from(
        sig_size
            .checked_add(1)
            .unwrap_or_else(|| panic!("signature size must leave index-zero room")),
    )
    .unwrap_or_else(|_| panic!("signature size must fit usize"))
}

fn fcode_index(symbol: FunCode) -> usize {
    usize::try_from(symbol).unwrap_or_else(|_| panic!("f-code must fit vector index"))
}

fn q_to_part_i64(value: i64) -> CompareResult {
    match value.cmp(&0) {
        Ordering::Less => CompareResult::Lesser,
        Ordering::Equal => CompareResult::Equal,
        Ordering::Greater => CompareResult::Greater,
    }
}

fn is_distinct_symbol(signature: &Signature, symbol: FunCode) -> bool {
    symbol <= signature.f_count()
        && signature.is_any_func_prop_set(symbol, signature.distinct_props())
}

#[cfg(test)]
mod tests {
    use super::{
        weights_size, OrderControlBlock, OCB_FUN_DEFAULT_WEIGHT, TO_NAMES, W_DEFAULT_WEIGHT,
    };
    use crate::basics::partial_orderings::{CompareResult, HoOrderKind};
    use crate::heuristics::to_params::{LiteralCmp, TermOrdering, DEFAULT_DB_WEIGHT};
    use crate::terms::functypes::FunCode;
    use crate::terms::signature::{
        Signature, FP_IS_INTEGER, FP_IS_OBJECT, SIG_FALSE_CODE, SIG_TRUE_CODE,
    };
    use crate::terms::simpletypes::{alloc_simple_sort, Type};
    use crate::terms::termbanks::TermBank;
    use crate::terms::termtypes::{term_identity_id, Term};
    use crate::terms::typebanks::TypeBank;

    fn signature() -> Signature {
        let mut signature = Signature::new(TypeBank::new());
        signature
            .insert_internal_codes()
            .unwrap_or_else(|err| panic!("{err}"));
        signature
    }

    fn typed_symbol(signature: &mut Signature, name: &str, type_: &Type) -> FunCode {
        let code = signature.insert_id(name, 0, false);
        signature
            .declare_final_type(code, type_.clone())
            .unwrap_or_else(|err| panic!("{err}"));
        code
    }

    #[test]
    fn constants_and_names_match_c_header() {
        assert_eq!(OCB_FUN_DEFAULT_WEIGHT, 1);
        assert_eq!(W_DEFAULT_WEIGHT, 1);
        assert_eq!(
            TO_NAMES,
            [
                "NoOrdering",
                "Optimize",
                "KBO",
                "KBO6",
                "LPO",
                "LPOCopy",
                "LPO4",
                "LPO4Copy",
                "RPO",
                "Empty"
            ]
        );
    }

    #[test]
    fn allocation_matches_c_storage_defaults() {
        let signature = signature();
        let kbo =
            OrderControlBlock::alloc(TermOrdering::Kbo6, true, &signature, HoOrderKind::LfhoOrder);
        let lpo = OrderControlBlock::alloc(
            TermOrdering::Lpo,
            false,
            &signature,
            HoOrderKind::LambdaOrder,
        );
        let empty = OrderControlBlock::alloc(
            TermOrdering::Empty,
            false,
            &signature,
            HoOrderKind::LfhoOrder,
        );

        assert_eq!(kbo.sig_size, signature.f_count());
        assert_eq!(
            kbo.weights.as_ref().map(Vec::len),
            Some(weights_size(kbo.sig_size))
        );
        assert!(kbo.prec_weights.is_some());
        assert!(kbo.precedence.is_none());
        assert_eq!(kbo.var_weight, 1);
        assert_eq!(kbo.db_weight, DEFAULT_DB_WEIGHT);
        assert_eq!(kbo.lit_cmp, LiteralCmp::Normal);
        assert_eq!(kbo.vb_size, 64);
        assert!(kbo.vb.iter().all(|entry| *entry == 0));
        assert!(lpo.weights.is_none());
        assert!(lpo.precedence.is_some());
        assert_eq!(lpo.vb_size, 1);
        assert!(empty.weights.is_none());
        assert!(empty.precedence.is_none());
        assert!(empty.prec_weights.is_none());
    }

    #[test]
    fn higher_order_variable_map_uses_term_identity_and_c_reset_boundary() {
        let signature = signature();
        let mut ocb = OrderControlBlock::alloc(
            TermOrdering::Kbo6,
            true,
            &signature,
            HoOrderKind::LambdaOrder,
        );
        ocb.var_weight = 3;
        let first = Term::const_cell_alloc(-2);
        let first_alias = first.clone();
        let same_code_distinct_owner = Term::const_cell_alloc(-2);

        ocb.inc_ho_var_balance(&first);
        ocb.inc_ho_var_balance(&first_alias);
        ocb.dec_ho_var_balance(&first_alias);
        ocb.dec_ho_var_balance(&same_code_distinct_owner);

        assert_ne!(
            term_identity_id(&first),
            term_identity_id(&same_code_distinct_owner)
        );
        assert_eq!(ocb.ho_vb.len(), 2);
        assert_eq!(ocb.ho_vb[&term_identity_id(&first)], 1);
        assert_eq!(ocb.ho_vb[&term_identity_id(&same_code_distinct_owner)], -1);
        assert_eq!(ocb.pos_bal, 1);
        assert_eq!(ocb.neg_bal, 1);
        assert_eq!(ocb.wb, 0);

        ocb.reset_ho_var_map();

        assert!(ocb.ho_vb.is_empty());
        assert_eq!(ocb.pos_bal, 1);
        assert_eq!(ocb.neg_bal, 1);
        assert_eq!(ocb.wb, 0);
    }

    #[test]
    fn function_weight_lookup_defaults_for_new_symbols() {
        let mut signature = signature();
        let mut ocb =
            OrderControlBlock::alloc(TermOrdering::Kbo, true, &signature, HoOrderKind::LfhoOrder);
        ocb.set_fun_weight(SIG_TRUE_CODE, 7);
        let later = signature.insert_id("later", 0, false);

        assert_eq!(ocb.fun_weight(SIG_TRUE_CODE), 7);
        assert_eq!(ocb.fun_weight(later), OCB_FUN_DEFAULT_WEIGHT);
    }

    #[test]
    fn total_precedence_weights_drive_symbol_comparison() {
        let mut signature = signature();
        let a = signature.insert_id("a", 0, false);
        let b = signature.insert_id("b", 0, false);
        let mut ocb =
            OrderControlBlock::alloc(TermOrdering::Kbo, true, &signature, HoOrderKind::LfhoOrder);

        ocb.set_fun_prec_weight(SIG_TRUE_CODE, i64::MIN / 2);
        ocb.set_fun_prec_weight(a, 10);
        ocb.set_fun_prec_weight(b, 20);

        assert_eq!(ocb.fun_prec_weight(a), 10);
        assert_eq!(ocb.fun_compare(&signature, a, b), CompareResult::Lesser);
        assert_eq!(ocb.fun_compare(&signature, b, a), CompareResult::Greater);
        assert_eq!(
            ocb.fun_compare(&signature, SIG_TRUE_CODE, b),
            CompareResult::Lesser
        );
        assert_eq!(
            ocb.fun_prec_weight(signature.f_count() + 1),
            -(signature.f_count() + 1)
        );
    }

    #[test]
    fn distinct_symbol_properties_override_precedence_weights() {
        let mut signature = signature();
        let ordinary = signature.insert_id("ordinary", 0, false);
        let distinct = signature.insert_id("distinct", 0, false);
        signature.set_func_prop(distinct, FP_IS_INTEGER);
        let mut ocb =
            OrderControlBlock::alloc(TermOrdering::Kbo, true, &signature, HoOrderKind::LfhoOrder);
        ocb.set_fun_prec_weight(ordinary, 100);
        ocb.set_fun_prec_weight(distinct, 1);

        assert_eq!(
            ocb.fun_compare(&signature, ordinary, distinct),
            CompareResult::Greater
        );
        assert_eq!(
            ocb.fun_compare(&signature, distinct, ordinary),
            CompareResult::Lesser
        );
    }

    #[test]
    fn matrix_precedence_adds_transitive_relations_and_backtracks() {
        let mut signature = signature();
        let a = signature.insert_id("a", 0, false);
        let b = signature.insert_id("b", 0, false);
        let c = signature.insert_id("c", 0, false);
        let mut ocb =
            OrderControlBlock::alloc(TermOrdering::Lpo, false, &signature, HoOrderKind::LfhoOrder);
        let state = ocb.precedence_state();

        assert_eq!(
            ocb.precedence_add_tuple(&signature, a, b, CompareResult::Greater),
            1
        );
        assert_eq!(
            ocb.precedence_add_tuple(&signature, b, c, CompareResult::Greater),
            1
        );

        assert_eq!(ocb.fun_compare(&signature, a, b), CompareResult::Greater);
        assert_eq!(ocb.fun_compare(&signature, b, c), CompareResult::Greater);
        assert_eq!(ocb.fun_compare(&signature, a, c), CompareResult::Greater);
        assert!(ocb.precedence_state() > state);
        assert!(!ocb.precedence_backtrack(state));
        assert_eq!(
            ocb.fun_compare(&signature, a, b),
            CompareResult::Uncomparable
        );
    }

    #[test]
    #[should_panic(expected = "only concrete precedence relations are inserted")]
    fn matrix_precedence_rejects_not_greater_equal_cache_result() {
        let mut signature = signature();
        let a = signature.insert_id("a", 0, false);
        let b = signature.insert_id("b", 0, false);
        let mut ocb =
            OrderControlBlock::alloc(TermOrdering::Lpo, false, &signature, HoOrderKind::LfhoOrder);

        let _state = ocb.precedence_add_tuple(&signature, a, b, CompareResult::NotGreaterEqual);
    }

    #[test]
    #[should_panic(expected = "only concrete precedence relations are inserted")]
    fn matrix_precedence_rejects_not_less_equal_cache_result() {
        let mut signature = signature();
        let a = signature.insert_id("a", 0, false);
        let b = signature.insert_id("b", 0, false);
        let mut ocb =
            OrderControlBlock::alloc(TermOrdering::Lpo, false, &signature, HoOrderKind::LfhoOrder);

        let _state = ocb.precedence_add_tuple(&signature, a, b, CompareResult::NotLessEqual);
    }

    #[test]
    fn incompatible_matrix_tuple_returns_zero_without_rewriting_existing_relation() {
        let mut signature = signature();
        let a = signature.insert_id("a", 0, false);
        let b = signature.insert_id("b", 0, false);
        let mut ocb =
            OrderControlBlock::alloc(TermOrdering::Lpo, false, &signature, HoOrderKind::LfhoOrder);

        assert_eq!(
            ocb.precedence_add_tuple(&signature, a, b, CompareResult::Greater),
            1
        );
        assert_eq!(
            ocb.precedence_add_tuple(&signature, b, a, CompareResult::Greater),
            0
        );
        assert_eq!(ocb.fun_compare(&signature, a, b), CompareResult::Greater);
    }

    #[test]
    fn matrix_compare_orders_symbols_added_after_ocb_creation_as_lower() {
        let mut signature = signature();
        let old = signature.insert_id("old", 0, false);
        let ocb =
            OrderControlBlock::alloc(TermOrdering::Lpo, false, &signature, HoOrderKind::LfhoOrder);
        let new = signature.insert_id("new", 0, false);

        assert_eq!(
            ocb.fun_compare(&signature, old, new),
            CompareResult::Greater
        );
        assert_eq!(ocb.fun_compare(&signature, new, old), CompareResult::Lesser);
        assert_eq!(
            ocb.fun_compare(&signature, new + 1, new),
            CompareResult::Lesser
        );
    }

    #[test]
    fn min_constant_helpers_use_type_uid_slots() {
        let mut signature = signature();
        let individual = signature.type_bank().i_type();
        let first = typed_symbol(&mut signature, "first", &individual);
        let second = typed_symbol(&mut signature, "second", &individual);
        let mut ocb =
            OrderControlBlock::alloc(TermOrdering::Lpo, false, &signature, HoOrderKind::LfhoOrder);
        ocb.precedence_add_tuple(&signature, first, second, CompareResult::Lesser);

        let found = ocb.find_min_const(&mut signature, &individual);

        assert_eq!(found, second);
        assert_eq!(ocb.min_const(&individual), second);
        ocb.cond_set_min_const(&individual, first);
        assert_eq!(ocb.min_const(&individual), second);
        ocb.set_min_const(&individual, first);
        assert_eq!(ocb.min_const(&individual), first);
    }

    #[test]
    fn find_min_const_creates_typed_skolem_when_no_constant_exists() {
        let mut signature = signature();
        let animal_code = signature
            .type_bank_mut()
            .define_simple_sort("animal")
            .unwrap_or_else(|err| panic!("{err}"));
        let animal = signature
            .type_bank_mut()
            .insert_type_shared(alloc_simple_sort(animal_code));
        let before = signature.f_count();
        let mut ocb =
            OrderControlBlock::alloc(TermOrdering::Lpo, false, &signature, HoOrderKind::LfhoOrder);

        let skolem = ocb.find_min_const(&mut signature, &animal);

        assert!(skolem > before);
        assert_eq!(signature.get_type(skolem), Some(&animal));
        assert_eq!(ocb.min_const(&animal), skolem);
    }

    #[test]
    fn designated_min_term_uses_term_bank_min_term_cache() {
        let mut bank = TermBank::new(signature()).unwrap_or_else(|err| panic!("{err}"));
        let individual = bank.signature().type_bank().i_type();
        let constant = typed_symbol(bank.signature_mut(), "a", &individual);
        let mut ocb = OrderControlBlock::alloc(
            TermOrdering::Lpo,
            false,
            bank.signature(),
            HoOrderKind::LfhoOrder,
        );

        let first = ocb
            .designated_min_term(&mut bank, &individual)
            .unwrap_or_else(|err| panic!("{err}"));
        let second = ocb
            .designated_min_term(&mut bank, &individual)
            .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(first.f_code(), constant);
        assert_eq!(second, first);
    }

    #[test]
    fn term_max_fun_code_preserves_c_argument_zero_skip_and_deref_once() {
        let mut signature = signature();
        let root = signature.insert_id("f", 2, false);
        let skipped = signature.insert_id("a", 0, false);
        let inspected = signature.insert_id("b", 0, false);
        let mut ocb =
            OrderControlBlock::alloc(TermOrdering::Lpo, false, &signature, HoOrderKind::LfhoOrder);
        ocb.precedence_add_tuple(&signature, root, inspected, CompareResult::Lesser);
        ocb.precedence_add_tuple(&signature, inspected, skipped, CompareResult::Lesser);
        let term = Term::top_alloc(root, 2);
        term.set_argument(0, Term::const_cell_alloc(skipped));
        term.set_argument(1, Term::const_cell_alloc(inspected));
        let variable = Term::const_cell_alloc(-2);
        variable.set_binding(Some(term.clone()));

        assert_eq!(ocb.term_max_fun_code(&signature, &term), inspected);
        assert_eq!(ocb.term_max_fun_code(&signature, &variable), inspected);
        assert_eq!(
            ocb.term_max_fun_code(&signature, &Term::const_cell_alloc(-4)),
            0
        );
    }

    #[test]
    fn install_helpers_copy_generated_weight_vectors() {
        let signature = signature();
        let mut ocb =
            OrderControlBlock::alloc(TermOrdering::Kbo6, true, &signature, HoOrderKind::LfhoOrder);
        let mut weights = vec![0; weights_size(signature.f_count())];
        let mut prec_weights = weights.clone();
        weights[usize::try_from(SIG_FALSE_CODE).unwrap()] = 11;
        prec_weights[usize::try_from(SIG_FALSE_CODE).unwrap()] = 22;

        ocb.install_weights(&weights);
        ocb.install_prec_weights(&prec_weights);

        assert_eq!(ocb.fun_weight(SIG_FALSE_CODE), 11);
        assert_eq!(ocb.fun_prec_weight(SIG_FALSE_CODE), 22);
    }

    #[test]
    fn debug_print_reports_signature_weights_and_absent_matrix() {
        let mut signature = signature();
        let a = signature.insert_id("a", 0, false);
        let b = signature.insert_id("b", 0, false);
        let mut ocb =
            OrderControlBlock::alloc(TermOrdering::Kbo, true, &signature, HoOrderKind::LfhoOrder);
        ocb.set_fun_weight(a, 7);
        ocb.set_fun_weight(b, 3);

        let mut output = Vec::new();
        ocb.debug_print(&mut output, Some(&signature))
            .unwrap_or_else(|err| panic!("{err}"));
        let printed = String::from_utf8(output).unwrap_or_else(|err| panic!("{err}"));

        assert!(printed.starts_with("% ==============OCB-Debug-Information============\n"));
        assert!(printed.contains("% Signature"));
        assert!(printed.contains(" (a = 7) "));
        assert!(printed.contains(" (b = 3) "));
        assert!(printed.contains("% No precedence!\n"));
    }

    #[test]
    fn debug_print_reports_precedence_matrix_symbols() {
        let mut signature = signature();
        let a = signature.insert_id("a", 0, false);
        let b = signature.insert_id("b", 0, false);
        let mut ocb =
            OrderControlBlock::alloc(TermOrdering::Lpo, false, &signature, HoOrderKind::LfhoOrder);
        ocb.precedence_add_tuple(&signature, a, b, CompareResult::Greater);

        let mut output = Vec::new();
        ocb.debug_print(&mut output, Some(&signature))
            .unwrap_or_else(|err| panic!("{err}"));
        let printed = String::from_utf8(output).unwrap_or_else(|err| panic!("{err}"));

        assert!(printed.contains("% No weights!\n"));
        assert!(printed.contains("% Precedence Matrix:\n"));
        assert!(printed.contains(" = "));
        assert!(printed.contains(" > "));
        assert!(printed.contains(" < "));
        assert!(printed.contains("=/="));
    }

    #[test]
    fn debug_print_can_render_without_signature_when_no_lookup_is_needed() {
        let signature = signature();
        let ocb = OrderControlBlock::alloc(
            TermOrdering::Empty,
            false,
            &signature,
            HoOrderKind::LfhoOrder,
        );

        let mut output = Vec::new();
        ocb.debug_print(&mut output, None)
            .unwrap_or_else(|err| panic!("{err}"));
        let printed = String::from_utf8(output).unwrap_or_else(|err| panic!("{err}"));

        assert!(printed.contains("% No sig!\n"));
        assert!(printed.contains("% No weights!\n"));
        assert!(printed.contains("% No precedence!\n"));
    }

    #[test]
    fn object_distinct_property_has_same_precedence_override_sign() {
        let mut signature = signature();
        let ordinary = signature.insert_id("ordinary", 0, false);
        let object = signature.insert_id("object", 0, false);
        signature.set_func_prop(object, FP_IS_OBJECT);
        let ocb =
            OrderControlBlock::alloc(TermOrdering::Lpo, false, &signature, HoOrderKind::LfhoOrder);

        assert_eq!(
            ocb.fun_compare(&signature, ordinary, object),
            CompareResult::Greater
        );
    }
}
