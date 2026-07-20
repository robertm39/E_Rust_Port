use crate::basics::error::Diagnostic;
use crate::basics::partial_orderings::CompareResult;
use crate::basics::simple_stuff::ProblemType;
use crate::basics::{pdarrays::PDIntArray, pstacks::PStack};
use crate::clauses::eqn::{
    eqn_parse, eqn_write, eqn_write_deref, eqn_write_tstp_with_type_suffixes, Eqn, EqnPrintOptions,
};
use crate::clauses::eqn_props::{
    EqnProperties, EP_IS_MAXIMAL, EP_IS_POSITIVE, EP_IS_STRICTLY_MAXIMAL,
};
use crate::inout::scanner::{IoFormat, Scanner, TokenType};
use crate::orderings::ocb::OrderControlBlock;
use crate::terms::functypes::func_symb_start_token;
use crate::terms::functypes::FunCode;
use crate::terms::lambda::lambda_normalize_db;
use crate::terms::signature::Signature;
use crate::terms::subst::Substitution;
use crate::terms::termbanks::TermBank;
use crate::terms::termtypes::{DerefType, Term, TermProperties};
use crate::terms::termvars::VarBank;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

pub const EQN_LIST_LONG_LIMIT: usize = 15;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct EqnList {
    literals: Vec<Eqn>,
}

impl EqnList {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            literals: Vec::new(),
        }
    }

    #[must_use]
    pub fn from_vec(literals: Vec<Eqn>) -> Self {
        Self { literals }
    }

    #[must_use]
    pub fn from_array(array: Vec<Eqn>) -> Self {
        Self::from_vec(array)
    }

    #[must_use]
    pub fn as_slice(&self) -> &[Eqn] {
        &self.literals
    }

    #[must_use]
    pub fn as_mut_slice(&mut self) -> &mut [Eqn] {
        &mut self.literals
    }

    #[must_use]
    pub fn into_vec(self) -> Vec<Eqn> {
        self.literals
    }

    #[must_use]
    pub const fn len(&self) -> usize {
        self.literals.len()
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.literals.is_empty()
    }

    pub fn push(&mut self, literal: Eqn) {
        self.literals.push(literal);
    }

    /// Parses the C `EqnListParse` shape using the banked equation and
    /// `TBTermParse`-equivalent term parsers.
    ///
    /// # Panics
    ///
    /// Panics if the scanner format is `IoFormat::Auto`, matching the C
    /// assertion that a concrete input format has been selected before list
    /// parsing.
    pub fn parse(
        scanner: &mut Scanner,
        bank: &mut TermBank,
        sep: TokenType,
        problem_type: ProblemType,
    ) -> Result<Self, Diagnostic> {
        let term_start = func_symb_start_token();
        let starts_literal = match scanner.format() {
            IoFormat::Tptp => scanner.test_tok(TokenType::PLUS | TokenType::HYPHEN),
            IoFormat::Lop | IoFormat::Tstp => scanner.test_tok(term_start | TokenType::TILDE_SIGN),
            IoFormat::Auto => panic!("format not supported"),
        };
        if !starts_literal {
            return Ok(Self::new());
        }

        let mut result = Self::new();
        result.push(eqn_parse(scanner, bank, problem_type)?);
        while scanner.test_tok(sep) {
            scanner.next_token()?;
            result.push(eqn_parse(scanner, bank, problem_type)?);
        }
        Ok(result)
    }

    pub fn gc_mark_terms(&self, bank: &TermBank) {
        for literal in &self.literals {
            literal.gc_mark_terms(bank);
        }
    }

    pub fn set_prop(&mut self, prop: EqnProperties) -> usize {
        for literal in &mut self.literals {
            literal.set_prop(prop);
        }
        self.len()
    }

    pub fn del_prop(&mut self, prop: EqnProperties) -> usize {
        for literal in &mut self.literals {
            literal.del_prop(prop);
        }
        self.len()
    }

    pub fn flip_prop(&mut self, prop: EqnProperties) -> usize {
        for literal in &mut self.literals {
            literal.flip_prop(prop);
        }
        self.len()
    }

    #[must_use]
    pub fn query_prop_number(&self, prop: EqnProperties) -> usize {
        self.literals
            .iter()
            .filter(|literal| literal.query_prop(prop))
            .count()
    }

    #[must_use]
    pub fn exists_term_except<F>(&self, except_index: Option<usize>, mut predicate: F) -> bool
    where
        F: FnMut(&Term) -> bool,
    {
        self.literals.iter().enumerate().any(|(index, literal)| {
            Some(index) != except_index && (predicate(literal.left()) || predicate(literal.right()))
        })
    }

    #[must_use]
    pub fn exists_term<F>(&self, predicate: F) -> bool
    where
        F: FnMut(&Term) -> bool,
    {
        self.exists_term_except(None, predicate)
    }

    pub fn map_terms<F>(&mut self, bank: &TermBank, mut mapper: F)
    where
        F: FnMut(&Term) -> Term,
    {
        for literal in &mut self.literals {
            literal.map_terms(bank, &mut mapper);
        }
    }

    /// Applies C `EqnListLambdaNormalize` to every literal side.
    ///
    /// The underlying literal mapper preserves C `EqnMap` side effects:
    /// normalized `$false` sides are rewritten through `$true` with polarity
    /// flips, `$true` is swapped away from the left side, and the equational
    /// literal flag is refreshed.
    ///
    /// # Errors
    ///
    /// Returns diagnostics from DB-lambda beta/eta normalization.
    pub fn lambda_normalize(&mut self, bank: &mut TermBank) -> Result<usize, Diagnostic> {
        let mut changed_sides = 0;
        for literal in &mut self.literals {
            let old_left = literal.left().clone();
            let old_right = literal.right().clone();
            let normalized_left = lambda_normalize_db(bank, &old_left)?;
            let normalized_right = lambda_normalize_db(bank, &old_right)?;

            let mut mapped_left = false;
            let mut mapped_right = false;
            literal.map_terms(bank, |term| {
                if !mapped_left && term == &old_left {
                    mapped_left = true;
                    normalized_left.clone()
                } else if !mapped_right && term == &old_right {
                    mapped_right = true;
                    normalized_right.clone()
                } else {
                    term.clone()
                }
            });

            changed_sides += usize::from(literal.left() != &old_left);
            changed_sides += usize::from(literal.right() != &old_right);
        }
        Ok(changed_sides)
    }

    /// Orient every literal in the list and return the number of swaps.
    pub fn orient(&mut self, ocb: &mut OrderControlBlock, bank: &TermBank) -> usize {
        let mut swaps = 0;
        for literal in &mut self.literals {
            if literal.orient(ocb, bank) {
                swaps += 1;
            }
        }
        swaps
    }

    /// Orient every literal using a bank-backed ordering path when needed.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if bank-backed term ordering preparation fails.
    pub fn orient_with_bank(
        &mut self,
        ocb: &mut OrderControlBlock,
        bank: &mut TermBank,
    ) -> Result<usize, Diagnostic> {
        let mut swaps = 0;
        for literal in &mut self.literals {
            if literal.orient_with_bank(ocb, bank)? {
                swaps += 1;
            }
        }
        Ok(swaps)
    }

    /// Mark maximal and strictly maximal literals under the selected ordering.
    ///
    /// This preserves C `EqnListMaximalLiterals` while keeping the Rust list
    /// order stable instead of destructively relinking candidates.
    pub fn mark_maximal_literals(&mut self, ocb: &mut OrderControlBlock, bank: &TermBank) -> usize {
        self.set_prop(EP_IS_STRICTLY_MAXIMAL);
        self.del_prop(EP_IS_MAXIMAL);

        let mut candidates: Vec<usize> = (0..self.len()).collect();
        let mut maximal = Vec::new();

        while !candidates.is_empty() {
            let candidate = candidates.remove(0);
            let mut candidate_survives = true;
            let mut step = 0;

            while step < candidates.len() {
                let current = candidates[step];
                match self.literals[candidate].literal_compare(ocb, bank, &self.literals[current]) {
                    CompareResult::Greater => {
                        self.literals[current].del_prop(EP_IS_STRICTLY_MAXIMAL);
                        candidates.remove(step);
                    }
                    CompareResult::Lesser => {
                        self.literals[candidate].del_prop(EP_IS_STRICTLY_MAXIMAL);
                        candidate_survives = false;
                        break;
                    }
                    CompareResult::Equal => {
                        self.literals[current].del_prop(EP_IS_STRICTLY_MAXIMAL);
                        self.literals[candidate].del_prop(EP_IS_STRICTLY_MAXIMAL);
                        step += 1;
                    }
                    CompareResult::Unknown
                    | CompareResult::Uncomparable
                    | CompareResult::NotGreaterEqual
                    | CompareResult::NotLessEqual => {
                        step += 1;
                    }
                }
            }

            if candidate_survives {
                maximal.push(candidate);
            }
        }

        for index in &maximal {
            self.literals[*index].set_prop(EP_IS_MAXIMAL);
        }
        maximal.len()
    }

    /// Mark maximal and strictly maximal literals using a bank-backed ordering
    /// path when needed.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if bank-backed term ordering preparation fails.
    pub fn mark_maximal_literals_with_bank(
        &mut self,
        ocb: &mut OrderControlBlock,
        bank: &mut TermBank,
    ) -> Result<usize, Diagnostic> {
        self.set_prop(EP_IS_STRICTLY_MAXIMAL);
        self.del_prop(EP_IS_MAXIMAL);

        let mut candidates: Vec<usize> = (0..self.len()).collect();
        let mut maximal = Vec::new();

        while !candidates.is_empty() {
            let candidate = candidates.remove(0);
            let mut candidate_survives = true;
            let mut step = 0;

            while step < candidates.len() {
                let current = candidates[step];
                match self.literals[candidate].literal_compare_with_bank(
                    ocb,
                    bank,
                    &self.literals[current],
                )? {
                    CompareResult::Greater => {
                        self.literals[current].del_prop(EP_IS_STRICTLY_MAXIMAL);
                        candidates.remove(step);
                    }
                    CompareResult::Lesser => {
                        self.literals[candidate].del_prop(EP_IS_STRICTLY_MAXIMAL);
                        candidate_survives = false;
                        break;
                    }
                    CompareResult::Equal => {
                        self.literals[current].del_prop(EP_IS_STRICTLY_MAXIMAL);
                        self.literals[candidate].del_prop(EP_IS_STRICTLY_MAXIMAL);
                        step += 1;
                    }
                    CompareResult::Unknown
                    | CompareResult::Uncomparable
                    | CompareResult::NotGreaterEqual
                    | CompareResult::NotLessEqual => {
                        step += 1;
                    }
                }
            }

            if candidate_survives {
                maximal.push(candidate);
            }
        }

        for index in &maximal {
            self.literals[*index].set_prop(EP_IS_MAXIMAL);
        }
        Ok(maximal.len())
    }

    /// Return whether the literal at `eqn_index` is maximal with respect to
    /// currently marked maximal literals in this list.
    ///
    /// This mirrors C `EqnListEqnIsMaximal`, including its reliance on
    /// existing `EPIsMaximal` flags rather than recomputing maximality.
    ///
    /// # Panics
    ///
    /// Panics under the same invariants as [`Eqn::literal_compare`].
    #[must_use]
    pub fn eqn_is_maximal_index(
        &self,
        ocb: &mut OrderControlBlock,
        bank: &TermBank,
        eqn_index: usize,
    ) -> Option<bool> {
        let eqn = self.literals.get(eqn_index)?;
        Some(self.literals.iter().enumerate().all(|(index, literal)| {
            index == eqn_index
                || !literal.is_maximal()
                || literal.literal_compare(ocb, bank, eqn) != CompareResult::Greater
        }))
    }

    /// Bank-backed variant of [`Self::eqn_is_maximal_index`].
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if bank-backed term ordering preparation fails.
    ///
    /// # Panics
    ///
    /// Panics under the same invariants as [`Eqn::literal_compare_with_bank`].
    pub fn eqn_is_maximal_index_with_bank(
        &self,
        ocb: &mut OrderControlBlock,
        bank: &mut TermBank,
        eqn_index: usize,
    ) -> Result<Option<bool>, Diagnostic> {
        let Some(eqn) = self.literals.get(eqn_index) else {
            return Ok(None);
        };
        for (index, literal) in self.literals.iter().enumerate() {
            if index != eqn_index
                && literal.is_maximal()
                && literal.literal_compare_with_bank(ocb, bank, eqn)? == CompareResult::Greater
            {
                return Ok(Some(false));
            }
        }
        Ok(Some(true))
    }

    /// Return whether the literal at `eqn_index` is strictly maximal with
    /// respect to currently marked maximal literals in this list.
    ///
    /// This mirrors C `EqnListEqnIsStrictlyMaximal`: another marked maximal
    /// literal that is equal to or greater than the candidate makes the
    /// candidate non-strict.
    ///
    /// # Panics
    ///
    /// Panics under the same invariants as [`Eqn::literal_compare`].
    #[must_use]
    pub fn eqn_is_strictly_maximal_index(
        &self,
        ocb: &mut OrderControlBlock,
        bank: &TermBank,
        eqn_index: usize,
    ) -> Option<bool> {
        let eqn = self.literals.get(eqn_index)?;
        Some(self.literals.iter().enumerate().all(|(index, literal)| {
            index == eqn_index
                || !literal.is_maximal()
                || !matches!(
                    literal.literal_compare(ocb, bank, eqn),
                    CompareResult::Equal | CompareResult::Greater
                )
        }))
    }

    /// Bank-backed variant of [`Self::eqn_is_strictly_maximal_index`].
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if bank-backed term ordering preparation fails.
    ///
    /// # Panics
    ///
    /// Panics under the same invariants as [`Eqn::literal_compare_with_bank`].
    pub fn eqn_is_strictly_maximal_index_with_bank(
        &self,
        ocb: &mut OrderControlBlock,
        bank: &mut TermBank,
        eqn_index: usize,
    ) -> Result<Option<bool>, Diagnostic> {
        let Some(eqn) = self.literals.get(eqn_index) else {
            return Ok(None);
        };
        for (index, literal) in self.literals.iter().enumerate() {
            if index != eqn_index
                && literal.is_maximal()
                && matches!(
                    literal.literal_compare_with_bank(ocb, bank, eqn)?,
                    CompareResult::Equal | CompareResult::Greater
                )
            {
                return Ok(Some(false));
            }
        }
        Ok(Some(true))
    }

    #[must_use]
    pub fn to_stack(&self) -> PStack<&Eqn> {
        let mut stack = PStack::new();
        for literal in &self.literals {
            stack.push(literal);
        }
        stack
    }

    /// Moves all literals into a stack without copying their cells.
    ///
    /// This is the owned Rust counterpart to consuming C pointers with
    /// `EqnListFromStack`; [`Self::to_stack`] provides C's non-owning pointer
    /// view when the list must remain intact.
    #[must_use]
    pub fn into_stack(self) -> PStack<Eqn> {
        let mut stack = PStack::new();
        for literal in self.literals {
            stack.push(literal);
        }
        stack
    }

    #[must_use]
    pub fn from_stack(mut stack: PStack<Eqn>) -> Self {
        let mut literals = Vec::with_capacity(stack.len());
        while let Some(literal) = stack.pop() {
            literals.push(literal);
        }
        literals.reverse();
        Self::from_vec(literals)
    }

    /// Writes the C `EqnListPrint` shape with an explicit separator and
    /// negation flag.
    ///
    /// # Panics
    ///
    /// Panics if any literal or term violates the C printing preconditions.
    pub fn write_print(
        &self,
        output: &mut impl fmt::Write,
        bank: &TermBank,
        sep: &str,
        negated: bool,
        full_terms: bool,
        options: EqnPrintOptions,
    ) -> fmt::Result {
        let mut iter = self.literals.iter();
        if let Some(first) = iter.next() {
            eqn_write(output, bank, first, negated, full_terms, options)?;
            for literal in iter {
                output.write_str(sep)?;
                eqn_write(output, bank, literal, negated, full_terms, options)?;
            }
        }
        Ok(())
    }

    #[must_use]
    pub fn print_string(
        &self,
        bank: &TermBank,
        sep: &str,
        negated: bool,
        full_terms: bool,
        options: EqnPrintOptions,
    ) -> String {
        let mut output = String::new();
        let _ = self.write_print(&mut output, bank, sep, negated, full_terms, options);
        output
    }

    /// Writes the C `EqnListPrintDeref` shape with an explicit separator.
    ///
    /// # Panics
    ///
    /// Panics if any printed term violates the C printing preconditions.
    pub fn write_deref_print(
        &self,
        output: &mut impl fmt::Write,
        bank: &TermBank,
        sep: &str,
        deref: DerefType,
    ) -> fmt::Result {
        let mut iter = self.literals.iter();
        if let Some(first) = iter.next() {
            eqn_write_deref(output, bank, first, deref)?;
            for literal in iter {
                output.write_str(sep)?;
                eqn_write_deref(output, bank, literal, deref)?;
            }
        }
        Ok(())
    }

    #[must_use]
    pub fn deref_print_string(&self, bank: &TermBank, sep: &str, deref: DerefType) -> String {
        let mut output = String::new();
        let _ = self.write_deref_print(&mut output, bank, sep, deref);
        output
    }

    /// Writes the C `EqnListTSTPPrint` shape with an explicit separator.
    ///
    /// # Panics
    ///
    /// Panics if any literal or term violates the C printing preconditions.
    pub fn write_tstp_print(
        &self,
        output: &mut impl fmt::Write,
        bank: &TermBank,
        sep: &str,
        full_terms: bool,
        print_oriented: bool,
    ) -> fmt::Result {
        self.write_tstp_print_with_type_suffixes(
            output,
            bank,
            sep,
            full_terms,
            print_oriented,
            false,
        )
    }

    /// Writes the C `EqnListTSTPPrint` shape with optional term type suffixes.
    ///
    /// # Panics
    ///
    /// Panics if any literal or term violates the C printing preconditions.
    pub fn write_tstp_print_with_type_suffixes(
        &self,
        output: &mut impl fmt::Write,
        bank: &TermBank,
        sep: &str,
        full_terms: bool,
        print_oriented: bool,
        print_types: bool,
    ) -> fmt::Result {
        let mut iter = self.literals.iter();
        if let Some(first) = iter.next() {
            eqn_write_tstp_with_type_suffixes(
                output,
                bank,
                first,
                full_terms,
                print_oriented,
                print_types,
            )?;
            for literal in iter {
                output.write_str(sep)?;
                eqn_write_tstp_with_type_suffixes(
                    output,
                    bank,
                    literal,
                    full_terms,
                    print_oriented,
                    print_types,
                )?;
            }
        }
        Ok(())
    }

    #[must_use]
    pub fn tstp_print_string(
        &self,
        bank: &TermBank,
        sep: &str,
        full_terms: bool,
        print_oriented: bool,
    ) -> String {
        let mut output = String::new();
        let _ = self.write_tstp_print(&mut output, bank, sep, full_terms, print_oriented);
        output
    }

    #[must_use]
    pub fn split_to_stacks(&self, prop: EqnProperties) -> (PStack<&Eqn>, PStack<&Eqn>) {
        let mut matching = PStack::new();
        let mut non_matching = PStack::new();
        for literal in &self.literals {
            if literal.query_prop(prop) {
                matching.push(literal);
            } else {
                non_matching.push(literal);
            }
        }
        (matching, non_matching)
    }

    pub fn extract_element(&mut self, index: usize) -> Option<Eqn> {
        if index < self.len() {
            Some(self.literals.remove(index))
        } else {
            None
        }
    }

    #[must_use]
    pub fn extract_by_props(&mut self, props: EqnProperties, negate: bool) -> Self {
        let mut kept = Vec::with_capacity(self.len());
        let mut extracted = Vec::new();
        for literal in self.literals.drain(..) {
            if literal.query_prop(props) ^ negate {
                extracted.push(literal);
            } else {
                kept.push(literal);
            }
        }
        extracted.reverse();
        self.literals = kept;
        Self::from_vec(extracted)
    }

    pub fn delete_element(&mut self, index: usize) -> bool {
        self.extract_element(index).is_some()
    }

    pub fn insert_element(&mut self, index: usize, literal: Eqn) -> bool {
        if index > self.len() {
            return false;
        }
        self.literals.insert(index, literal);
        true
    }

    pub fn insert_first(&mut self, literal: Eqn) {
        self.literals.insert(0, literal);
    }

    pub fn append(&mut self, mut newpart: Self) {
        self.literals.append(&mut newpart.literals);
    }

    pub fn flat_copy(&self, bank: &mut TermBank) -> Result<Self, Diagnostic> {
        let mut copy = Vec::with_capacity(self.len());
        for literal in &self.literals {
            copy.push(literal.flat_copy(bank)?);
        }
        Ok(Self::from_vec(copy))
    }

    pub fn copy_to_bank(&self, bank: &mut TermBank) -> Result<Self, Diagnostic> {
        let mut copy = Vec::with_capacity(self.len());
        for literal in &self.literals {
            copy.push(literal.copy_to_bank(bank)?);
        }
        Ok(Self::from_vec(copy))
    }

    pub fn copy_except_index(
        &self,
        except_index: Option<usize>,
        bank: &mut TermBank,
    ) -> Result<Self, Diagnostic> {
        let mut copy = Vec::with_capacity(self.len());
        for (index, literal) in self.literals.iter().enumerate() {
            if Some(index) != except_index {
                copy.push(literal.copy_to_bank(bank)?);
            }
        }
        Ok(Self::from_vec(copy))
    }

    pub fn copy_opt(&self, bank: &mut TermBank) -> Result<Self, Diagnostic> {
        let mut copy = Vec::with_capacity(self.len());
        for literal in &self.literals {
            copy.push(literal.copy_opt(bank)?);
        }
        Ok(Self::from_vec(copy))
    }

    pub fn copy_opt_except_index(
        &self,
        except_index: Option<usize>,
        bank: &mut TermBank,
    ) -> Result<Self, Diagnostic> {
        let mut copy = Vec::with_capacity(self.len());
        for (index, literal) in self.literals.iter().enumerate() {
            if Some(index) != except_index {
                copy.push(literal.copy_opt(bank)?);
            }
        }
        Ok(Self::from_vec(copy))
    }

    pub fn copy_disjoint(&self, bank: &mut TermBank) -> Result<Self, Diagnostic> {
        let mut copy = Vec::with_capacity(self.len());
        for literal in &self.literals {
            copy.push(literal.copy_disjoint(bank)?);
        }
        Ok(Self::from_vec(copy))
    }

    pub fn copy_repl(
        &self,
        bank: &mut TermBank,
        old: &Term,
        repl: &Term,
    ) -> Result<Self, Diagnostic> {
        let mut copy = Vec::with_capacity(self.len());
        for literal in &self.literals {
            copy.push(literal.copy_repl(bank, old, repl)?);
        }
        Ok(Self::from_vec(copy))
    }

    pub fn copy_repl_plain(
        &self,
        bank: &mut TermBank,
        old: &Term,
        repl: &Term,
    ) -> Result<Self, Diagnostic> {
        let mut copy = Vec::with_capacity(self.len());
        for literal in &self.literals {
            copy.push(literal.copy_repl_plain(bank, old, repl)?);
        }
        Ok(Self::from_vec(copy))
    }

    pub fn negate_eqns(&mut self) {
        for literal in &mut self.literals {
            literal.flip_prop(EP_IS_POSITIVE);
        }
    }

    pub fn remove_duplicates(&mut self, bank: &TermBank) -> usize {
        let mut seen = BTreeSet::new();
        let old_len = self.len();
        self.literals
            .retain(|literal| seen.insert(literal_syntax_key(literal, bank)));
        old_len - self.len()
    }

    pub fn remove_resolved(&mut self, bank: &TermBank) -> usize {
        let old_len = self.len();
        self.literals.retain(|literal| !literal.is_false(bank));
        old_len - self.len()
    }

    pub fn remove_ac_resolved(&mut self, bank: &TermBank) -> usize {
        let old_len = self.len();
        self.literals
            .retain(|literal| literal.is_positive() || !literal.is_ac_trivial(bank));
        old_len - self.len()
    }

    pub fn remove_simple_answers(&mut self, bank: &TermBank) -> usize {
        let old_len = self.len();
        self.literals
            .retain(|literal| !literal.is_simple_answer(bank));
        old_len - self.len()
    }

    #[must_use]
    pub fn find_neg_pure_var_lit_index(&self) -> Option<usize> {
        self.literals
            .iter()
            .position(|literal| literal.is_negative() && literal.is_pure_var())
    }

    #[must_use]
    pub fn find_neg_pure_var_lit(&self) -> Option<&Eqn> {
        self.find_neg_pure_var_lit_index()
            .and_then(|index| self.literals.get(index))
    }

    #[must_use]
    pub fn find_true_index(&self, bank: &TermBank) -> Option<usize> {
        self.literals
            .iter()
            .position(|literal| literal.is_true(bank))
    }

    #[must_use]
    pub fn find_true(&self, bank: &TermBank) -> Option<&Eqn> {
        self.find_true_index(bank)
            .and_then(|index| self.literals.get(index))
    }

    #[must_use]
    pub fn is_trivial(&self) -> bool {
        for index in 0..self.len() {
            for other in &self.literals[index + 1..] {
                let literal = &self.literals[index];
                if !EqnProperties::are_equiv(
                    literal.properties(),
                    other.properties(),
                    EP_IS_POSITIVE,
                ) && literal.equal(other)
                {
                    return true;
                }
            }
        }
        false
    }

    #[must_use]
    pub fn long_is_trivial(&self, bank: &TermBank) -> bool {
        let mut positives = PStack::new();
        let mut negatives = PStack::new();
        for literal in &self.literals {
            if literal.is_positive() {
                positives.push(literal);
            } else {
                negatives.push(literal);
            }
        }

        positives.sort_by(|left, right| left.syntax_compare(right, bank).cmp(&0));
        negatives.sort_by(|left, right| left.syntax_compare(right, bank).cmp(&0));

        let mut positive_pos = 0;
        let mut negative_pos = 0;
        while positive_pos < positives.stack_pointer() {
            let key = *positives.element(positive_pos);
            negative_pos = negatives.bin_search_by_key(
                key,
                negative_pos,
                negatives.stack_pointer(),
                |search, element| search.syntax_compare(element, bank).cmp(&0),
            );
            if negative_pos >= negatives.stack_pointer() {
                break;
            }
            let other = *negatives.element(negative_pos);
            if key.syntax_compare(other, bank) == 0 {
                return true;
            }
            positive_pos = positives.bin_search_by_key(
                other,
                positive_pos,
                positives.stack_pointer(),
                |search, element| search.syntax_compare(element, bank).cmp(&0),
            );
        }
        false
    }

    #[must_use]
    pub fn is_ac_trivial(&self, bank: &TermBank) -> bool {
        self.literals
            .iter()
            .any(|literal| literal.is_positive() && literal.is_ac_trivial(bank))
    }

    #[must_use]
    pub fn is_ground(&self) -> bool {
        self.literals.iter().all(Eqn::is_ground)
    }

    #[must_use]
    pub fn is_equational(&self, bank: &TermBank) -> bool {
        self.literals.iter().any(|literal| literal.is_equ_lit(bank))
    }

    #[must_use]
    pub fn is_pure_equational(&self, bank: &TermBank) -> bool {
        self.literals.iter().all(|literal| literal.is_equ_lit(bank))
    }

    #[must_use]
    pub fn subst_norm_except(
        &self,
        except_index: Option<usize>,
        subst: &mut Substitution,
        vars: &VarBank,
    ) -> usize {
        let result = subst.len();
        for (index, literal) in self.literals.iter().enumerate() {
            if Some(index) != except_index {
                literal.subst_norm(subst, vars);
            }
        }
        result
    }

    #[must_use]
    pub fn subst_norm(&self, subst: &mut Substitution, vars: &VarBank) -> usize {
        self.subst_norm_except(None, subst, vars)
    }

    #[must_use]
    pub fn depth(&self) -> i64 {
        self.literals
            .iter()
            .map(Eqn::depth)
            .max()
            .unwrap_or_default()
    }

    pub fn add_symbol_distribution(&self, dist_array: &mut [i64]) {
        for literal in &self.literals {
            literal.add_symbol_distribution(dist_array);
        }
    }

    pub fn add_type_distribution(&self, sig: &mut Signature, type_array: &mut [i64]) {
        for literal in &self.literals {
            literal.add_type_distribution(sig, type_array);
        }
    }

    pub fn add_symbol_dist_exist(&self, dist_array: &mut [i64], exists: &mut Vec<FunCode>) {
        for literal in &self.literals {
            literal.add_symbol_dist_exist(dist_array, exists);
        }
    }

    pub fn add_symbol_features(&self, mod_stack: &mut Vec<usize>, feature_array: &mut [i64]) {
        for literal in &self.literals {
            literal.add_symbol_features(mod_stack, feature_array);
        }
    }

    pub fn compute_function_ranks(&self, rank_array: &mut [i64], count: &mut i64) {
        for literal in &self.literals {
            literal.compute_function_ranks(rank_array, count);
        }
    }

    pub fn collect_variables(&self, vars: &mut BTreeMap<usize, Term>) -> i64 {
        self.literals
            .iter()
            .map(|literal| literal.collect_variables(vars))
            .sum()
    }

    pub fn collect_fcodes(&self, fcodes: &mut BTreeSet<FunCode>) -> i64 {
        self.literals
            .iter()
            .map(|literal| literal.collect_fcodes(fcodes))
            .sum()
    }

    pub fn add_fun_occs(&self, f_occur: &mut PDIntArray, res_stack: &mut Vec<FunCode>) -> i64 {
        self.literals
            .iter()
            .map(|literal| literal.add_fun_occs(f_occur, res_stack))
            .sum()
    }

    pub fn signed_term_set_prop(&self, prop: TermProperties, pos: bool, neg: bool) {
        for literal in &self.literals {
            if (pos && literal.is_positive()) || (neg && literal.is_negative()) {
                literal.term_set_prop(prop);
            }
        }
    }

    pub fn term_set_prop(&self, prop: TermProperties) {
        self.signed_term_set_prop(prop, true, true);
    }

    pub fn signed_term_del_prop(&self, prop: TermProperties, pos: bool, neg: bool) {
        for literal in &self.literals {
            if (pos && literal.is_positive()) || (neg && literal.is_negative()) {
                literal.term_del_prop(prop);
            }
        }
    }

    pub fn term_del_prop(&self, prop: TermProperties) {
        self.signed_term_del_prop(prop, true, true);
    }

    #[must_use]
    pub fn tb_term_del_prop_count(&self, prop: TermProperties) -> i64 {
        self.literals
            .iter()
            .map(|literal| literal.tb_term_del_prop_count(prop))
            .sum()
    }

    pub fn collect_subterms(&self, collector: &mut PStack<Term>) -> i64 {
        self.literals
            .iter()
            .map(|literal| literal.collect_subterms(collector))
            .sum()
    }

    pub fn collect_ground_terms(
        &self,
        result: &mut BTreeMap<usize, Term>,
        pos_lits: bool,
        neg_lits: bool,
        all_subterms: bool,
    ) -> i64 {
        self.literals
            .iter()
            .filter(|literal| {
                (literal.is_positive() && pos_lits) || (literal.is_negative() && neg_lits)
            })
            .map(|literal| literal.collect_ground_terms(result, all_subterms))
            .sum()
    }

    #[must_use]
    pub fn find_comp_lit_except(
        &self,
        except_index: Option<usize>,
        other: &Eqn,
        left_deref: DerefType,
        right_deref: DerefType,
    ) -> bool {
        self.literals.iter().enumerate().any(|(index, literal)| {
            Some(index) != except_index
                && literal.is_positive() != other.is_positive()
                && literal.equal_deref(other, left_deref, right_deref)
        })
    }
}

fn literal_syntax_key(literal: &Eqn, bank: &TermBank) -> (u8, u8, i64, i64) {
    let sign = u8::from(!literal.is_positive());
    let (equational, max_entry, min_entry) = eqn_syntax_key(literal, bank);
    (sign, equational, max_entry, min_entry)
}

fn eqn_syntax_key(literal: &Eqn, bank: &TermBank) -> (u8, i64, i64) {
    let equational = u8::from(!literal.is_equ_lit(bank));
    let left = literal.left().entry_no();
    let right = literal.right().entry_no();
    (equational, left.max(right), left.min(right))
}

#[cfg(test)]
mod tests {
    use super::{EqnList, EQN_LIST_LONG_LIMIT};
    use crate::basics::partial_orderings::HoOrderKind;
    use crate::basics::pdarrays::{PDIntArray, GROW_EXPONENTIAL};
    use crate::basics::pstacks::PStack;
    use crate::basics::simple_stuff::ProblemType;
    use crate::clauses::eqn::{Eqn, EqnPrintOptions};
    use crate::clauses::eqn_props::{
        EP_IS_EQU_LITERAL, EP_IS_MAXIMAL, EP_IS_ORIENTED, EP_IS_POSITIVE, EP_IS_SELECTED,
        EP_IS_STRICTLY_MAXIMAL, EP_MAX_IS_UP_TO_DATE,
    };
    use crate::heuristics::to_params::TermOrdering;
    use crate::inout::scanner::{IoFormat, Scanner, TokenType};
    use crate::orderings::ocb::OrderControlBlock;
    use crate::terms::lambda::{apply_terms, close_with_type_prefix};
    use crate::terms::signature::{Signature, FP_ASSOCIATIVE, FP_COMMUTATIVE};
    use crate::terms::simpletypes::{alloc_arrow_type, Type};
    use crate::terms::subst::Substitution;
    use crate::terms::termbanks::TermBank;
    use crate::terms::termtypes::{DerefType, Term, TP_OP_FLAG, TP_SPECIAL_FLAG};
    use crate::terms::typebanks::TypeBank;
    use std::collections::{BTreeMap, BTreeSet};

    fn test_bank() -> TermBank {
        let mut signature = Signature::new(TypeBank::new());
        signature.insert_internal_codes().unwrap();
        TermBank::new(signature).unwrap()
    }

    fn typed_const(bank: &mut TermBank, name: &str) -> Term {
        let type_ = bank.signature().type_bank().default_type();
        let f_code = bank.signature_mut().insert_id(name, 0, false);
        bank.signature_mut()
            .declare_final_type(f_code, type_.clone())
            .unwrap();
        let term = Term::const_cell_alloc(f_code);
        term.set_type(Some(type_));
        bank.insert(&term, DerefType::Never).unwrap()
    }

    fn typed_const_with_type(bank: &mut TermBank, name: &str, type_: Type) -> Term {
        let f_code = bank.signature_mut().insert_id(name, 0, false);
        bank.signature_mut()
            .declare_final_type(f_code, type_.clone())
            .unwrap();
        let term = Term::const_cell_alloc(f_code);
        term.set_type(Some(type_));
        bank.insert(&term, DerefType::Never).unwrap()
    }

    fn typed_pred_const(bank: &mut TermBank, name: &str) -> Term {
        let bool_type = bank.signature().type_bank().bool_type();
        let f_code = bank.signature_mut().insert_id(name, 0, false);
        bank.signature_mut()
            .declare_final_type(f_code, bool_type.clone())
            .unwrap();
        let term = Term::const_cell_alloc(f_code);
        term.set_type(Some(bool_type));
        bank.insert(&term, DerefType::Never).unwrap()
    }

    fn typed_unary(bank: &mut TermBank, name: &str, arg: &Term) -> Term {
        let type_ = bank.signature().type_bank().default_type();
        let f_code = bank.signature_mut().insert_id(name, 1, false);
        bank.signature_mut()
            .declare_final_type(f_code, alloc_arrow_type(vec![type_.clone(), type_.clone()]))
            .unwrap();
        let term = Term::top_alloc(f_code, 1);
        term.set_type(Some(type_));
        term.set_argument(0, arg.clone());
        bank.insert(&term, DerefType::Never).unwrap()
    }

    fn typed_binary_with_code(bank: &mut TermBank, f_code: i64, left: &Term, right: &Term) -> Term {
        let type_ = bank.signature().type_bank().default_type();
        let term = Term::top_alloc(f_code, 2);
        term.set_type(Some(type_));
        term.set_argument(0, left.clone());
        term.set_argument(1, right.clone());
        bank.insert(&term, DerefType::Never).unwrap()
    }

    fn typed_var(bank: &TermBank, f_code: i64) -> Term {
        let type_ = bank.signature().type_bank().default_type();
        bank.vars().var_assert_alloc(f_code, &type_)
    }

    fn answer_term(bank: &mut TermBank, arg: &Term) -> Term {
        let term = Term::top_alloc(bank.signature().answer_code(), 1);
        term.set_type(Some(bank.signature().type_bank().bool_type()));
        term.set_argument(0, arg.clone());
        bank.insert(&term, DerefType::Never).unwrap()
    }

    fn eqn(bank: &mut TermBank, left: &Term, right: &Term, positive: bool) -> Eqn {
        Eqn::alloc(left.clone(), right.clone(), bank, positive).unwrap()
    }

    fn kbo_ocb(bank: &TermBank) -> OrderControlBlock {
        OrderControlBlock::alloc(
            TermOrdering::Kbo,
            true,
            bank.signature(),
            HoOrderKind::LfhoOrder,
        )
    }

    #[test]
    fn parse_reads_separated_equation_list_and_empty_start() {
        let mut bank = test_bank();
        let mut scanner = Scanner::from_user_string("list_a=list_b;~list_p", false).unwrap();
        scanner.set_format(IoFormat::Lop);

        let list = EqnList::parse(
            &mut scanner,
            &mut bank,
            TokenType::SEMICOLON,
            ProblemType::FirstOrder,
        )
        .unwrap();

        assert_eq!(list.len(), 2);
        assert_eq!(
            list.print_string(&bank, ";", false, true, EqnPrintOptions::default()),
            "list_a=list_b;~list_p"
        );

        let mut empty = Scanner::from_user_string("]", false).unwrap();
        empty.set_format(IoFormat::Lop);
        assert!(EqnList::parse(
            &mut empty,
            &mut bank,
            TokenType::SEMICOLON,
            ProblemType::FirstOrder
        )
        .unwrap()
        .is_empty());
    }

    #[test]
    fn parse_routes_distinct_term_tokens_through_the_banked_term_parser() {
        let mut bank = test_bank();
        let mut scanner = Scanner::from_user_string(
            "++equal(42,42);--equal(3/4,3/4);++equal(1.5,1.5);--equal(\"obj\",\"obj\")",
            false,
        )
        .unwrap();
        scanner.set_format(IoFormat::Tptp);

        let list = EqnList::parse(
            &mut scanner,
            &mut bank,
            TokenType::SEMICOLON,
            ProblemType::FirstOrder,
        )
        .unwrap();

        assert_eq!(list.len(), 4);
        assert!(list.as_slice()[0].is_positive());
        assert!(list.as_slice()[1].is_negative());
        assert!(list.as_slice()[2].is_positive());
        assert!(list.as_slice()[3].is_negative());
        assert_eq!(
            list.as_slice()
                .iter()
                .map(|literal| bank.signature().find_name(literal.left().f_code()).unwrap())
                .collect::<Vec<_>>(),
            vec!["42", "3/4", "1.500000", "\"obj\""]
        );
    }

    #[test]
    fn property_helpers_apply_to_each_literal() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let c = typed_const(&mut bank, "c");
        let mut list = EqnList::from_vec(vec![
            eqn(&mut bank, &a, &b, true),
            eqn(&mut bank, &b, &c, false),
        ]);

        assert_eq!(EQN_LIST_LONG_LIMIT, 15);
        assert_eq!(list.set_prop(EP_IS_SELECTED), 2);
        assert_eq!(list.query_prop_number(EP_IS_SELECTED), 2);
        assert_eq!(list.del_prop(EP_IS_SELECTED), 2);
        assert_eq!(list.query_prop_number(EP_IS_SELECTED), 0);
        assert_eq!(list.flip_prop(EP_IS_MAXIMAL), 2);
        assert!(list.as_slice().iter().all(Eqn::is_maximal));
    }

    #[test]
    fn orient_orients_all_literals_and_counts_swaps() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let f_a = typed_unary(&mut bank, "f", &a);
        let mut list = EqnList::from_vec(vec![
            eqn(&mut bank, &a, &f_a, true),
            eqn(&mut bank, &f_a, &a, true),
        ]);
        let mut ocb = kbo_ocb(&bank);

        assert_eq!(list.orient(&mut ocb, &bank), 1);

        assert_eq!(list.as_slice()[0].left(), &f_a);
        assert_eq!(list.as_slice()[0].right(), &a);
        assert!(list.as_slice().iter().all(Eqn::is_oriented));
        assert!(list
            .as_slice()
            .iter()
            .all(|literal| literal.query_prop(EP_MAX_IS_UP_TO_DATE)));
    }

    #[test]
    fn mark_maximal_literals_preserves_c_candidate_semantics() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let f_a = typed_unary(&mut bank, "f", &a);
        let dominant = eqn(&mut bank, &f_a, &a, true);
        let equal_dominant = eqn(&mut bank, &f_a, &a, true);
        let dominated = eqn(&mut bank, &a, &a, true);
        let mut list = EqnList::from_vec(vec![dominant, equal_dominant, dominated]);
        let mut ocb = kbo_ocb(&bank);

        assert_eq!(list.mark_maximal_literals(&mut ocb, &bank), 2);

        assert!(list.as_slice()[0].is_maximal());
        assert!(list.as_slice()[1].is_maximal());
        assert!(!list.as_slice()[2].is_maximal());
        assert!(!list.as_slice()[0].is_strictly_maximal());
        assert!(!list.as_slice()[1].is_strictly_maximal());
        assert!(!list.as_slice()[2].query_prop(EP_IS_STRICTLY_MAXIMAL));
        assert_eq!(list.query_prop_number(EP_IS_MAXIMAL), 2);

        assert_eq!(list.eqn_is_maximal_index(&mut ocb, &bank, 0), Some(true));
        assert_eq!(list.eqn_is_maximal_index(&mut ocb, &bank, 1), Some(true));
        assert_eq!(list.eqn_is_maximal_index(&mut ocb, &bank, 2), Some(false));
        assert_eq!(list.eqn_is_maximal_index(&mut ocb, &bank, 3), None);
        assert_eq!(
            list.eqn_is_strictly_maximal_index(&mut ocb, &bank, 0),
            Some(false)
        );
        assert_eq!(
            list.eqn_is_strictly_maximal_index(&mut ocb, &bank, 1),
            Some(false)
        );
        assert_eq!(
            list.eqn_is_strictly_maximal_index(&mut ocb, &bank, 2),
            Some(false)
        );

        assert_eq!(
            list.eqn_is_maximal_index_with_bank(&mut ocb, &mut bank, 2)
                .unwrap(),
            Some(false)
        );
    }

    #[test]
    fn direct_strict_maximal_query_uses_existing_maximal_flags_like_c() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let f_a = typed_unary(&mut bank, "f", &a);
        let dominant = eqn(&mut bank, &f_a, &a, true);
        let dominated = eqn(&mut bank, &a, &a, true);
        let mut list = EqnList::from_vec(vec![dominant, dominated]);
        let mut ocb = kbo_ocb(&bank);

        assert_eq!(list.mark_maximal_literals(&mut ocb, &bank), 1);
        assert_eq!(
            list.eqn_is_strictly_maximal_index(&mut ocb, &bank, 0),
            Some(true)
        );
        assert_eq!(
            list.eqn_is_strictly_maximal_index(&mut ocb, &bank, 1),
            Some(false)
        );

        list.as_mut_slice()[0].del_prop(EP_IS_MAXIMAL);
        assert_eq!(list.eqn_is_maximal_index(&mut ocb, &bank, 1), Some(true));
        assert_eq!(
            list.eqn_is_strictly_maximal_index_with_bank(&mut ocb, &mut bank, 1)
                .unwrap(),
            Some(true)
        );
    }

    #[test]
    fn conversions_and_link_operations_preserve_c_ordering() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let c = typed_const(&mut bank, "c");
        let first = eqn(&mut bank, &a, &b, true);
        let second = eqn(&mut bank, &b, &c, true);
        let third = eqn(&mut bank, &c, &a, true);
        let mut list = EqnList::from_array(vec![first.clone(), second.clone(), third.clone()]);

        let stack = list.to_stack();
        assert!(stack
            .as_slice()
            .iter()
            .zip(list.as_slice())
            .all(|(stack_literal, list_literal)| std::ptr::eq(*stack_literal, list_literal)));
        drop(stack);

        let transfer = EqnList::from_array(vec![first.clone(), second.clone(), third.clone()]);
        let rebuilt = EqnList::from_stack(transfer.into_stack());
        assert_eq!(
            rebuilt.as_slice(),
            &[first.clone(), second.clone(), third.clone()]
        );

        let extracted = list.extract_element(1).unwrap();
        assert_eq!(extracted, second);
        assert_eq!(list.as_slice(), &[first.clone(), third.clone()]);
        assert!(list.insert_element(1, extracted));
        assert_eq!(
            list.as_slice(),
            &[first.clone(), second.clone(), third.clone()]
        );
        assert!(!list.insert_element(4, first.clone()));

        let mut tail = EqnList::new();
        tail.push(first.clone());
        list.append(tail);
        assert_eq!(
            list.as_slice(),
            &[first.clone(), second, third, first.clone()]
        );
        assert!(list.delete_element(3));
    }

    #[test]
    fn print_helpers_preserve_separator_order_and_tstp_literal_shape() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "list_print_a");
        let b = typed_const(&mut bank, "list_print_b");
        let c = typed_const(&mut bank, "list_print_c");
        let predicate = typed_pred_const(&mut bank, "list_print_p");
        let true_term = bank.true_term().clone();
        let mut oriented = eqn(&mut bank, &a, &b, true);
        oriented.set_prop(EP_IS_ORIENTED);
        let list = EqnList::from_vec(vec![
            oriented,
            eqn(&mut bank, &b, &c, false),
            eqn(&mut bank, &predicate, &true_term, false),
        ]);

        assert_eq!(
            EqnList::new().print_string(&bank, ";", false, true, EqnPrintOptions::default()),
            ""
        );
        assert_eq!(
            EqnList::new().deref_print_string(&bank, ";", DerefType::Always),
            ""
        );
        assert_eq!(
            list.print_string(&bank, ";", true, true, EqnPrintOptions::default()),
            "list_print_a!=list_print_b;list_print_b=list_print_c;list_print_p"
        );
        assert_eq!(
            list.tstp_print_string(&bank, " | ", true, true),
            "list_print_a->list_print_b | list_print_b!=list_print_c | ~list_print_p"
        );

        let type_ = bank.signature().type_bank().default_type();
        let x = bank.vars().var_assert_alloc(-2, &type_);
        x.set_binding(Some(a.clone()));
        let deref_list = EqnList::from_vec(vec![
            eqn(&mut bank, &x, &b, true),
            eqn(&mut bank, &c, &x, false),
        ]);
        assert_eq!(
            deref_list.deref_print_string(&bank, " / ", DerefType::Always),
            "list_print_a=list_print_b / list_print_c!=list_print_a"
        );
    }

    #[test]
    fn extract_by_props_reverses_extracted_literals_like_c_insert_first() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let c = typed_const(&mut bank, "c");
        let mut first = eqn(&mut bank, &a, &b, true);
        let mut second = eqn(&mut bank, &b, &c, true);
        let mut third = eqn(&mut bank, &c, &a, true);
        first.set_position(1);
        second.set_position(2);
        third.set_position(3);
        first.set_prop(EP_IS_SELECTED);
        third.set_prop(EP_IS_SELECTED);
        let mut list = EqnList::from_vec(vec![first, second, third]);

        let extracted = list.extract_by_props(EP_IS_SELECTED, false);

        assert_eq!(
            extracted
                .as_slice()
                .iter()
                .map(Eqn::position)
                .collect::<Vec<_>>(),
            vec![3, 1]
        );
        assert_eq!(list.as_slice()[0].position(), 2);
    }

    #[test]
    fn copy_helpers_forward_to_equation_copy_variants() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let c = typed_const(&mut bank, "c");
        let x = typed_var(&bank, -10);
        let list = EqnList::from_vec(vec![
            eqn(&mut bank, &a, &b, true),
            eqn(&mut bank, &x, &b, false),
        ]);

        let flat = list.flat_copy(&mut bank).unwrap();
        assert_eq!(flat, list);
        let copied = list.copy_except_index(Some(0), &mut bank).unwrap();
        assert_eq!(copied.len(), 1);
        assert!(copied.as_slice()[0].is_negative());

        let replaced = list.copy_repl(&mut bank, &b, &c).unwrap();
        assert_eq!(replaced.as_slice()[0].right(), &c);
        let plain_replaced = list.copy_repl_plain(&mut bank, &b, &a).unwrap();
        assert_eq!(plain_replaced.as_slice()[0].right(), &a);

        let disjoint = list.copy_disjoint(&mut bank).unwrap();
        assert_eq!(disjoint.len(), 2);
        assert_ne!(disjoint.as_slice()[1].left(), &x);
    }

    #[test]
    fn duplicate_and_resolved_removal_match_literal_predicates() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let answer = answer_term(&mut bank, &a);
        let answer_lit = Eqn::alloc(answer, bank.true_term().clone(), &mut bank, true).unwrap();
        let positive = eqn(&mut bank, &a, &b, true);
        let duplicate = eqn(&mut bank, &b, &a, true);
        let negative = eqn(&mut bank, &a, &b, false);
        let false_lit = eqn(&mut bank, &a, &a, false);
        let mut list =
            EqnList::from_vec(vec![positive, duplicate, negative, false_lit, answer_lit]);

        assert_eq!(list.remove_duplicates(&bank), 1);
        assert_eq!(list.remove_resolved(&bank), 1);
        assert_eq!(list.remove_simple_answers(&bank), 1);
        assert_eq!(list.len(), 2);
        assert!(!list.as_slice()[0].literal_equal(&list.as_slice()[1]));
    }

    #[test]
    fn truth_triviality_groundness_and_complement_search_match_c_shapes() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let x = typed_var(&bank, -10);
        let pos = eqn(&mut bank, &a, &b, true);
        let neg = eqn(&mut bank, &b, &a, false);
        let y = typed_var(&bank, -12);
        let pure_var = eqn(&mut bank, &x, &y, false);
        let true_lit = eqn(&mut bank, &a, &a, true);
        let list = EqnList::from_vec(vec![pos.clone(), pure_var, true_lit]);

        assert!(list.find_neg_pure_var_lit().is_some());
        assert_eq!(list.find_true_index(&bank), Some(2));
        assert!(EqnList::from_vec(vec![pos.clone(), neg.clone()]).is_trivial());
        assert!(EqnList::from_vec(vec![pos.clone(), neg.clone()]).long_is_trivial(&bank));
        assert!(!list.is_ground());
        assert!(EqnList::from_vec(vec![pos.clone()]).is_ground());
        assert!(EqnList::from_vec(vec![pos.clone()]).is_equational(&bank));
        assert!(EqnList::from_vec(vec![pos.clone()]).is_pure_equational(&bank));
        assert!(EqnList::from_vec(vec![pos]).find_comp_lit_except(
            None,
            &neg,
            DerefType::Never,
            DerefType::Never
        ));
    }

    #[test]
    fn long_triviality_preserves_c_binary_search_false_negative() {
        let mut bank = test_bank();
        let terms = (1..=14)
            .map(|index| typed_pred_const(&mut bank, &format!("long_pred_{index}")))
            .collect::<Vec<_>>();
        let truth = bank.true_term().clone();
        let positive_indices = [2, 5, 6, 7, 8];
        let negative_indices = [0, 1, 3, 4, 5, 7, 9, 10, 11, 12, 13];
        let mut literals = Vec::with_capacity(16);
        for index in positive_indices {
            literals
                .push(Eqn::alloc(terms[index].clone(), truth.clone(), &mut bank, true).unwrap());
        }
        for index in negative_indices {
            literals
                .push(Eqn::alloc(terms[index].clone(), truth.clone(), &mut bank, false).unwrap());
        }
        let list = EqnList::from_vec(literals);

        assert_eq!(list.len(), EQN_LIST_LONG_LIMIT + 1);
        assert!(list.is_trivial());
        assert!(!list.long_is_trivial(&bank));
    }

    #[test]
    fn ac_resolved_and_term_property_helpers_delegate_to_literals() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let type_ = bank.signature().type_bank().default_type();
        let f_code = bank.signature_mut().insert_id("f", 2, false);
        bank.signature_mut()
            .declare_final_type(
                f_code,
                alloc_arrow_type(vec![type_.clone(), type_.clone(), type_]),
            )
            .unwrap();
        bank.signature_mut()
            .set_func_prop(f_code, FP_ASSOCIATIVE | FP_COMMUTATIVE);
        let left = typed_binary_with_code(&mut bank, f_code, &a, &b);
        let right = typed_binary_with_code(&mut bank, f_code, &b, &a);
        let positive = eqn(&mut bank, &left, &right, true);
        let negative = eqn(&mut bank, &left, &right, false);
        let mut list = EqnList::from_vec(vec![positive, negative]);

        assert!(list.is_ac_trivial(&bank));
        assert_eq!(list.remove_ac_resolved(&bank), 1);
        list.signed_term_set_prop(TP_SPECIAL_FLAG, true, false);
        assert!(list.as_slice()[0].left().query_prop(TP_SPECIAL_FLAG));
        list.term_del_prop(TP_SPECIAL_FLAG);
        assert!(!list.as_slice()[0].left().query_prop(TP_SPECIAL_FLAG));

        list.term_set_prop(TP_OP_FLAG);
        assert!(list.tb_term_del_prop_count(TP_OP_FLAG) > 0);
    }

    #[test]
    fn substitution_and_collection_wrappers_accumulate_c_style_counts() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let f_of_a = typed_unary(&mut bank, "f", &a);
        let x = typed_var(&bank, -10);
        let variable_lit = eqn(&mut bank, &x, &b, true);
        let ground_lit = eqn(&mut bank, &f_of_a, &b, false);
        let list = EqnList::from_vec(vec![variable_lit, ground_lit]);

        let mut subst = Substitution::new();
        let vars = crate::terms::termvars::VarBank::new(bank.signature().type_bank());
        assert_eq!(list.subst_norm_except(Some(1), &mut subst, &vars), 0);
        assert_eq!(subst.len(), 1);
        subst.backtrack();

        assert_eq!(list.depth(), 2);
        let mut dist = vec![0; usize::try_from(bank.signature().f_count() + 1).unwrap()];
        list.add_symbol_distribution(&mut dist);
        assert!(dist[usize::try_from(f_of_a.f_code()).unwrap()] > 0);

        let mut exists_dist = vec![0; dist.len()];
        let mut exists = Vec::new();
        list.add_symbol_dist_exist(&mut exists_dist, &mut exists);
        assert!(exists.contains(&f_of_a.f_code()));

        let mut features = vec![0; usize::try_from((bank.signature().f_count() + 1) * 4).unwrap()];
        let mut modified = Vec::new();
        list.add_symbol_features(&mut modified, &mut features);
        assert!(!modified.is_empty());

        let mut ranks = vec![0; dist.len()];
        let mut count = 1;
        list.compute_function_ranks(&mut ranks, &mut count);
        assert!(ranks[usize::try_from(f_of_a.f_code()).unwrap()] > 0);

        let mut variables = BTreeMap::new();
        assert_eq!(list.collect_variables(&mut variables), 1);
        let mut fcodes = BTreeSet::new();
        assert!(list.collect_fcodes(&mut fcodes) >= 3);

        let mut occur = PDIntArray::new_int(2, GROW_EXPONENTIAL);
        let mut occurrence_stack = Vec::new();
        assert!(list.add_fun_occs(&mut occur, &mut occurrence_stack) >= 3);

        let mut subterms = PStack::new();
        assert!(list.collect_subterms(&mut subterms) >= 3);

        let mut ground_terms = BTreeMap::new();
        assert_eq!(
            list.collect_ground_terms(&mut ground_terms, false, true, false),
            1
        );
    }

    #[test]
    fn predicate_literals_affect_equational_classification() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let pred = typed_pred_const(&mut bank, "p");
        let predicate_lit = Eqn::alloc(pred, bank.true_term().clone(), &mut bank, true).unwrap();
        let equation_lit = eqn(&mut bank, &a, &a, true);
        let mut list = EqnList::from_vec(vec![predicate_lit, equation_lit]);

        assert!(list.is_equational(&bank));
        assert!(!list.is_pure_equational(&bank));
        list.negate_eqns();
        assert!(list.as_slice().iter().all(Eqn::is_negative));

        let (matching, non_matching) = list.split_to_stacks(EP_IS_POSITIVE);
        assert!(matching.is_empty());
        assert_eq!(non_matching.len(), 2);
        assert!(non_matching
            .as_slice()
            .iter()
            .zip(list.as_slice())
            .all(|(stack_literal, list_literal)| std::ptr::eq(*stack_literal, list_literal)));
    }

    #[test]
    fn map_terms_forwards_literal_normalization() {
        let mut bank = test_bank();
        let atom = typed_pred_const(&mut bank, "p");
        let mut list = EqnList::from_vec(vec![Eqn::alloc(
            atom.clone(),
            bank.true_term().clone(),
            &mut bank,
            true,
        )
        .unwrap()]);
        let false_term = bank.false_term().clone();

        list.map_terms(&bank, |term| {
            if term == &atom {
                false_term.clone()
            } else {
                term.clone()
            }
        });

        assert!(list.as_slice()[0].is_negative());
        assert_eq!(list.as_slice()[0].right(), bank.true_term());
    }

    #[test]
    fn lambda_normalize_maps_literal_sides_through_beta_eta_normalization() {
        let mut bank = test_bank();
        let i_type = bank.signature().type_bank().default_type();
        let unary_type = alloc_arrow_type(vec![i_type.clone(), i_type.clone()]);
        let f = typed_const_with_type(&mut bank, "eqnlist_lambda_f", unary_type);
        let db0 = bank.request_db_var(&i_type, 0);
        let matrix = apply_terms(&mut bank, &f, std::slice::from_ref(&db0)).unwrap();
        let lambda =
            close_with_type_prefix(&mut bank, std::slice::from_ref(&i_type), &matrix).unwrap();
        let a = typed_const(&mut bank, "eqnlist_lambda_a");
        let b = typed_const(&mut bank, "eqnlist_lambda_b");
        let applied = apply_terms(&mut bank, &lambda, std::slice::from_ref(&a)).unwrap();
        let expected = apply_terms(&mut bank, &f, std::slice::from_ref(&a)).unwrap();
        let mut list = EqnList::from_vec(vec![eqn(&mut bank, &applied, &b, true)]);

        let changed = list.lambda_normalize(&mut bank).unwrap();

        assert_eq!(changed, 1);
        assert_eq!(list.as_slice()[0].left(), &expected);
        assert_eq!(list.as_slice()[0].right(), &b);
        assert!(list.as_slice()[0].is_positive());
    }

    #[test]
    fn lambda_normalize_preserves_eqn_map_false_and_polarity_normalization() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "eqnlist_map_false_a");
        let b = typed_const(&mut bank, "eqnlist_map_false_b");
        let mut literal = eqn(&mut bank, &a, &b, true);
        literal.set_left_raw(bank.false_term().clone());
        literal.set_right_raw(b.clone());
        literal.set_prop(EP_IS_EQU_LITERAL);
        let mut list = EqnList::from_vec(vec![literal]);

        let changed = list.lambda_normalize(&mut bank).unwrap();

        assert_eq!(changed, 2);
        let literal = &list.as_slice()[0];
        assert_eq!(literal.left(), &b);
        assert_eq!(literal.right(), bank.true_term());
        assert!(!literal.is_positive());
        assert!(!literal.is_equ_lit(&bank));
    }

    #[test]
    fn orientation_flags_survive_stack_copies_and_copy_opt_clears_stale_metadata() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let mut literal = eqn(&mut bank, &a, &b, true);
        literal.set_prop(EP_IS_ORIENTED | EP_MAX_IS_UP_TO_DATE);
        let list = EqnList::from_vec(vec![literal]);

        let roundtrip = EqnList::from_stack(list.into_stack());
        assert!(roundtrip.as_slice()[0].is_oriented());

        let mut unoriented = roundtrip.as_slice()[0].clone();
        unoriented.del_prop(EP_IS_ORIENTED);
        let copied = EqnList::from_vec(vec![unoriented])
            .copy_opt(&mut bank)
            .unwrap();
        assert!(!copied.as_slice()[0].query_prop(EP_MAX_IS_UP_TO_DATE));
    }
}
