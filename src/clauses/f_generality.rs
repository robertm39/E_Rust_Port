use crate::basics::pstacks::PStack;
use crate::clauses::clause::Clause;
use crate::clauses::clausesets::ClauseSet;
use crate::clauses::formulasets::{FormulaSet, WrappedFormula};
use crate::terms::functypes::FunCode;
use crate::terms::signature::Signature;
use crate::terms::termfunc::{term_add_symbol_dist_exist, term_trim_implications};
use crate::terms::termtypes::Term;
use std::cmp::Ordering;
use std::fmt;

const DEFAULT_COMCHAR_RAW: &str = "%";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FunGen {
    f_code: FunCode,
    term_freq: i64,
    fc_freq: i64,
}

impl FunGen {
    #[must_use]
    pub const fn new(f_code: FunCode) -> Self {
        Self {
            f_code,
            term_freq: 0,
            fc_freq: 0,
        }
    }

    #[must_use]
    pub const fn from_counts(f_code: FunCode, term_freq: i64, fc_freq: i64) -> Self {
        Self {
            f_code,
            term_freq,
            fc_freq,
        }
    }

    #[must_use]
    pub const fn f_code(&self) -> FunCode {
        self.f_code
    }

    #[must_use]
    pub const fn term_freq(&self) -> i64 {
        self.term_freq
    }

    #[must_use]
    pub const fn fc_freq(&self) -> i64 {
        self.fc_freq
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i32)]
pub enum GeneralityMeasure {
    NoMeasure = 0,
    Terms = 1,
    Literals = 2,
    Formulas = 3,
    PositiveFormula = 4,
    PositiveLiteral = 5,
    PositiveTerms = 6,
    NegativeFormula = 7,
    NegativeLiteral = 8,
    NegativeTerms = 9,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DRelSelectionParams {
    pub gen_measure: GeneralityMeasure,
    pub benevolence: f64,
    pub generosity: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GenDistrib {
    internal_symbols: FunCode,
    dist_array: Vec<FunGen>,
    f_distrib: Vec<i64>,
}

impl GenDistrib {
    #[must_use]
    pub fn new(signature: &Signature) -> Self {
        let size = signature_size(signature);
        Self {
            internal_symbols: signature.internal_symbols(),
            dist_array: init_dist_array(size),
            f_distrib: vec![0; size],
        }
    }

    #[must_use]
    pub fn size(&self) -> usize {
        self.dist_array.len()
    }

    #[must_use]
    pub const fn internal_symbols(&self) -> FunCode {
        self.internal_symbols
    }

    #[must_use]
    pub fn entry(&self, f_code: FunCode) -> Option<&FunGen> {
        self.dist_array.get(f_code_index(f_code))
    }

    #[must_use]
    pub fn scratch_value(&self, f_code: FunCode) -> Option<i64> {
        self.f_distrib.get(f_code_index(f_code)).copied()
    }

    pub fn size_adjust(&mut self, signature: &Signature) {
        self.internal_symbols = signature.internal_symbols();
        let new_size = signature_size(signature);
        if signature.f_count() >= usize_to_i64(self.size()) {
            self.dist_array.resize_with(new_size, || FunGen::new(0));
            for (index, entry) in self.dist_array.iter_mut().enumerate() {
                if entry.f_code != usize_to_i64(index) {
                    *entry = FunGen::new(usize_to_i64(index));
                }
            }
            self.f_distrib = vec![0; new_size];
        }
    }

    pub fn add_clause(&mut self, clause: &Clause, factor: i16) {
        let mut symbol_stack = Vec::new();
        clause.add_symbol_dist_exist(&mut self.f_distrib, &mut symbol_stack);
        self.merge_single_res(&symbol_stack, factor);
        self.clear_scratch_symbols(symbol_stack);
    }

    pub fn add_clause_set(&mut self, set: &ClauseSet, factor: i16) {
        for clause in set.iter() {
            self.add_clause(clause, factor);
        }
    }

    pub fn add_clause_set_stack(&mut self, stack: &PStack<&ClauseSet>, start: usize, factor: i16) {
        for set in stack.as_slice().iter().skip(start) {
            self.add_clause_set(set, factor);
        }
    }

    pub fn add_clause_sets(&mut self, stack: &PStack<&ClauseSet>) {
        self.add_clause_set_stack(stack, 0, 1);
    }

    pub fn backtrack_clause_sets(&mut self, stack: &PStack<&ClauseSet>, start: usize) {
        self.add_clause_set_stack(stack, start, -1);
    }

    pub fn add_formula(
        &mut self,
        signature: &Signature,
        formula: &WrappedFormula,
        trim_implications: bool,
        factor: i16,
    ) {
        let mut symbol_stack = Vec::new();
        formula_add_symbol_dist_exist(
            signature,
            formula,
            trim_implications,
            &mut self.f_distrib,
            &mut symbol_stack,
        );
        self.merge_single_res(&symbol_stack, factor);
        self.clear_scratch_symbols(symbol_stack);
    }

    pub fn add_formula_set(
        &mut self,
        signature: &Signature,
        set: &FormulaSet,
        trim_implications: bool,
        factor: i16,
    ) {
        for formula in set.iter() {
            self.add_formula(signature, formula, trim_implications, factor);
        }
    }

    pub fn add_formula_set_stack(
        &mut self,
        signature: &Signature,
        stack: &PStack<&FormulaSet>,
        start: usize,
        trim_implications: bool,
        factor: i16,
    ) {
        for set in stack.as_slice().iter().skip(start) {
            self.add_formula_set(signature, set, trim_implications, factor);
        }
    }

    pub fn add_formula_sets(
        &mut self,
        signature: &Signature,
        stack: &PStack<&FormulaSet>,
        trim_implications: bool,
    ) {
        self.add_formula_set_stack(signature, stack, 0, trim_implications, 1);
    }

    pub fn backtrack_formula_sets(
        &mut self,
        signature: &Signature,
        stack: &PStack<&FormulaSet>,
        start: usize,
    ) {
        self.add_formula_set_stack(signature, stack, start, false, -1);
    }

    pub fn write_debug(
        &self,
        output: &mut impl fmt::Write,
        signature: &Signature,
        limit: i64,
    ) -> fmt::Result {
        let (term_freq_total, fc_freq_total) = self.debug_totals(signature);
        writeln!(
            output,
            "{DEFAULT_COMCHAR_RAW} GenDist {:p} {term_freq_total} {fc_freq_total}",
            std::ptr::from_ref(self)
        )?;

        let start = debug_external_start(signature);
        let end = debug_row_end(signature, self.size(), limit);
        for f_code in start..end {
            let entry = self.dist_array[f_code_index(f_code)];
            let name = signature.find_name(f_code).unwrap_or("<unknown>");
            writeln!(
                output,
                "{DEFAULT_COMCHAR_RAW} {name:<30} ({f_code:8} = {entry_f_code:8}): {term_freq:8}  {fc_freq:8}",
                entry_f_code = entry.f_code(),
                term_freq = entry.term_freq(),
                fc_freq = entry.fc_freq()
            )?;
        }

        Ok(())
    }

    #[must_use]
    pub fn debug_string(&self, signature: &Signature, limit: i64) -> String {
        let mut output = String::new();
        let _ = self.write_debug(&mut output, signature, limit);
        output
    }

    fn compute_d_rel(
        &self,
        gentype: GeneralityMeasure,
        benevolence: f64,
        generosity: i64,
        symbol_stack: &[FunCode],
        res: &mut PStack<FunCode>,
    ) {
        let mut sort_stack = Vec::new();
        for &f_code in symbol_stack {
            if f_code >= self.internal_symbols {
                let Some(entry) = self.entry(f_code) else {
                    panic!("D-relation symbol f-code is outside GenDistrib");
                };
                sort_stack.push(*entry);
            }
        }
        if sort_stack.is_empty() {
            return;
        }

        match gentype {
            GeneralityMeasure::Terms => sort_stack.sort_unstable_by(fun_gen_tg_order),
            GeneralityMeasure::Formulas => sort_stack.sort_unstable_by(fun_gen_cg_order),
            _ => panic!("unsupported generality measure for D-relation: {gentype:?}"),
        }

        let least_gen = extract_generality(sort_stack[0], gentype);
        let mut gen_limit = c_benevolence_limit(least_gen, benevolence);
        let generosity_index = generosity_index(generosity, sort_stack.len());
        let aux_gen_limit = extract_generality(sort_stack[generosity_index], gentype);
        if aux_gen_limit < gen_limit {
            gen_limit = aux_gen_limit;
        }

        for gen in sort_stack {
            if extract_generality(gen, gentype) > gen_limit {
                break;
            }
            res.push(gen.f_code());
        }
    }

    fn merge_single_res(&mut self, symbol_stack: &[FunCode], factor: i16) {
        let factor = i64::from(factor);
        for &f_code in symbol_stack {
            let index = f_code_index(f_code);
            self.dist_array[index].term_freq += factor * self.f_distrib[index];
            self.dist_array[index].fc_freq += factor;
        }
    }

    fn clear_scratch_symbols(&mut self, symbol_stack: Vec<FunCode>) {
        for f_code in symbol_stack.into_iter().rev() {
            self.f_distrib[f_code_index(f_code)] = 0;
        }
    }

    fn debug_totals(&self, signature: &Signature) -> (i64, i64) {
        let mut term_freq_total = 0;
        let mut fc_freq_total = 0;
        for f_code in debug_external_start(signature)..usize_to_i64(self.size()) {
            let entry = self.dist_array[f_code_index(f_code)];
            term_freq_total += entry.term_freq();
            fc_freq_total += entry.fc_freq();
        }
        (term_freq_total, fc_freq_total)
    }
}

pub fn clause_compute_d_rel(
    generality: &mut GenDistrib,
    gentype: GeneralityMeasure,
    benevolence: f64,
    generosity: i64,
    clause: &Clause,
    res: &mut PStack<FunCode>,
) {
    let mut symbol_stack = Vec::new();
    clause.add_symbol_dist_exist(&mut generality.f_distrib, &mut symbol_stack);
    generality.compute_d_rel(gentype, benevolence, generosity, &symbol_stack, res);
    generality.clear_scratch_symbols(symbol_stack);
}

pub fn formula_compute_d_rel(
    generality: &mut GenDistrib,
    params: DRelSelectionParams,
    signature: &Signature,
    formula: &WrappedFormula,
    res: &mut PStack<FunCode>,
    trim_implications: bool,
) {
    let mut symbol_stack = Vec::new();
    formula_add_symbol_dist_exist(
        signature,
        formula,
        trim_implications,
        &mut generality.f_distrib,
        &mut symbol_stack,
    );
    generality.compute_d_rel(
        params.gen_measure,
        params.benevolence,
        params.generosity,
        &symbol_stack,
        res,
    );
    generality.clear_scratch_symbols(symbol_stack);
}

pub fn formula_add_symbol_dist_exist(
    signature: &Signature,
    formula: &WrappedFormula,
    trim_implications: bool,
    dist_array: &mut [i64],
    exists: &mut Vec<FunCode>,
) {
    let term = formula_d_rel_term(signature, formula, trim_implications);
    term_add_symbol_dist_exist(&term, dist_array, exists);
}

#[must_use]
pub fn fun_gen_tg_cmp(left: FunGen, right: FunGen) -> i32 {
    cmp_order(fun_gen_tg_order(&left, &right))
}

#[must_use]
pub fn fun_gen_cg_cmp(left: FunGen, right: FunGen) -> i32 {
    cmp_order(fun_gen_cg_order(&left, &right))
}

fn extract_generality(gen: FunGen, gentype: GeneralityMeasure) -> i64 {
    match gentype {
        GeneralityMeasure::Terms => gen.term_freq(),
        GeneralityMeasure::Formulas => gen.fc_freq(),
        _ => panic!("unsupported generality measure: {gentype:?}"),
    }
}

fn fun_gen_tg_order(left: &FunGen, right: &FunGen) -> Ordering {
    left.term_freq()
        .cmp(&right.term_freq())
        .then_with(|| left.fc_freq().cmp(&right.fc_freq()))
        .then_with(|| left.f_code().cmp(&right.f_code()))
}

fn fun_gen_cg_order(left: &FunGen, right: &FunGen) -> Ordering {
    left.fc_freq()
        .cmp(&right.fc_freq())
        .then_with(|| left.term_freq().cmp(&right.term_freq()))
        .then_with(|| left.f_code().cmp(&right.f_code()))
}

#[expect(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    reason = "C assigns the double benevolence product to a long"
)]
fn c_benevolence_limit(least_gen: i64, benevolence: f64) -> i64 {
    (least_gen as f64 * benevolence) as i64
}

fn generosity_index(generosity: i64, len: usize) -> usize {
    let last = len - 1;
    usize::try_from(generosity).map_or_else(
        |_| panic!("generosity must be non-negative"),
        |index| index.min(last),
    )
}

fn formula_d_rel_term(
    signature: &Signature,
    formula: &WrappedFormula,
    trim_implications: bool,
) -> Term {
    if trim_implications && formula.is_conjecture() {
        term_trim_implications(signature, formula.formula())
    } else {
        formula.formula().clone()
    }
}

fn cmp_order(ordering: Ordering) -> i32 {
    match ordering {
        Ordering::Less => -1,
        Ordering::Equal => 0,
        Ordering::Greater => 1,
    }
}

fn signature_size(signature: &Signature) -> usize {
    usize::try_from(signature.f_count() + 1)
        .unwrap_or_else(|_| panic!("signature f-count must fit GenDistrib size"))
}

fn init_dist_array(size: usize) -> Vec<FunGen> {
    (0..size)
        .map(|index| FunGen::new(usize_to_i64(index)))
        .collect()
}

fn debug_external_start(signature: &Signature) -> FunCode {
    signature
        .internal_symbols()
        .checked_add(1)
        .unwrap_or_else(|| panic!("internal-symbol boundary must fit f-code range"))
}

fn debug_row_end(signature: &Signature, dist_size: usize, limit: i64) -> FunCode {
    let limited_end = signature
        .internal_symbols()
        .checked_add(limit)
        .unwrap_or_else(|| panic!("GenDistrib print limit must fit f-code range"));
    limited_end.min(usize_to_i64(dist_size))
}

fn f_code_index(f_code: FunCode) -> usize {
    usize::try_from(f_code).unwrap_or_else(|_| panic!("function code must fit usize: {f_code}"))
}

fn usize_to_i64(value: usize) -> i64 {
    i64::try_from(value).unwrap_or_else(|_| panic!("usize value must fit i64: {value}"))
}

#[cfg(test)]
mod tests {
    use super::{
        clause_compute_d_rel, formula_compute_d_rel, fun_gen_cg_cmp, fun_gen_tg_cmp,
        DRelSelectionParams, FunGen, GenDistrib, GeneralityMeasure,
    };
    use crate::basics::pstacks::PStack;
    use crate::clauses::clause::Clause;
    use crate::clauses::clause_props::CP_TYPE_CONJECTURE;
    use crate::clauses::clausesets::ClauseSet;
    use crate::clauses::eqn::Eqn;
    use crate::clauses::eqnlist::EqnList;
    use crate::clauses::formulasets::{FormulaSet, WrappedFormula};
    use crate::terms::functypes::FunCode;
    use crate::terms::signature::Signature;
    use crate::terms::signature::{SIG_FALSE_CODE, SIG_TRUE_CODE};
    use crate::terms::simpletypes::alloc_arrow_type;
    use crate::terms::termbanks::TermBank;
    use crate::terms::termtypes::{DerefType, Term};
    use crate::terms::typebanks::TypeBank;

    fn test_bank() -> TermBank {
        let mut signature = Signature::new(TypeBank::new());
        signature.insert_internal_codes().unwrap();
        TermBank::new(signature).unwrap()
    }

    fn typed_const(bank: &mut TermBank, name: &str) -> Term {
        let type_ = bank.signature().type_bank().default_type();
        let f_code = bank.signature_mut().insert_id(name, 0, false);
        bank.signature_mut()
            .declare_final_type(f_code, type_)
            .unwrap();
        bank.create_const_term(f_code).unwrap()
    }

    fn typed_unary(bank: &mut TermBank, name: &str, arg: &Term) -> Term {
        let type_ = bank.signature().type_bank().default_type();
        let arrow = bank
            .signature_mut()
            .type_bank_mut()
            .insert_type_shared(alloc_arrow_type(vec![type_.clone(), type_.clone()]));
        let f_code = bank.signature_mut().insert_id(name, 1, false);
        bank.signature_mut()
            .declare_final_type(f_code, arrow)
            .unwrap();
        let term = Term::top_alloc(f_code, 1);
        term.set_type(Some(type_));
        term.set_argument(0, arg.clone());
        bank.insert(&term, DerefType::Never).unwrap()
    }

    fn literal(bank: &mut TermBank, left: &Term, right: &Term, positive: bool) -> Eqn {
        Eqn::alloc(left.clone(), right.clone(), bank, positive).unwrap()
    }

    fn clause_from(literals: Vec<Eqn>) -> Clause {
        let mut clause = Clause::alloc(EqnList::from_vec(literals));
        clause.set_weight(clause.standard_weight());
        clause
    }

    fn wrapped_formula(term: Term) -> WrappedFormula {
        WrappedFormula::wt_formula_alloc(term)
    }

    fn formula_node(f_code: FunCode, left: Term, right: Term) -> Term {
        let term = Term::top_alloc(f_code, 2);
        term.set_argument(0, left);
        term.set_argument(1, right);
        term
    }

    fn implication_chain(signature: &Signature, premises: &[Term], conclusion: &Term) -> Term {
        let mut current = conclusion.clone();
        for premise in premises.iter().rev() {
            current = formula_node(signature.impl_code(), premise.clone(), current);
        }
        current
    }

    fn entry_counts(dist: &GenDistrib, f_code: FunCode) -> (i64, i64) {
        let entry = dist.entry(f_code).unwrap();
        (entry.term_freq(), entry.fc_freq())
    }

    #[test]
    fn generality_measure_discriminants_match_c_enum() {
        assert_eq!(GeneralityMeasure::NoMeasure as i32, 0);
        assert_eq!(GeneralityMeasure::Terms as i32, 1);
        assert_eq!(GeneralityMeasure::Literals as i32, 2);
        assert_eq!(GeneralityMeasure::Formulas as i32, 3);
        assert_eq!(GeneralityMeasure::PositiveFormula as i32, 4);
        assert_eq!(GeneralityMeasure::PositiveLiteral as i32, 5);
        assert_eq!(GeneralityMeasure::PositiveTerms as i32, 6);
        assert_eq!(GeneralityMeasure::NegativeFormula as i32, 7);
        assert_eq!(GeneralityMeasure::NegativeLiteral as i32, 8);
        assert_eq!(GeneralityMeasure::NegativeTerms as i32, 9);
    }

    #[test]
    fn fun_gen_comparators_match_c_tie_breakers() {
        let low_terms = FunGen::from_counts(9, 1, 10);
        let high_terms = FunGen::from_counts(1, 2, 1);
        let low_formula = FunGen::from_counts(5, 4, 1);
        let high_formula = FunGen::from_counts(2, 1, 3);
        let lower_code = FunGen::from_counts(4, 7, 7);
        let higher_code = FunGen::from_counts(6, 7, 7);

        assert_eq!(fun_gen_tg_cmp(low_terms, high_terms), -1);
        assert_eq!(fun_gen_cg_cmp(low_formula, high_formula), -1);
        assert_eq!(fun_gen_tg_cmp(lower_code, higher_code), -1);
        assert_eq!(fun_gen_cg_cmp(higher_code, lower_code), 1);
    }

    #[test]
    fn gen_distrib_add_clause_counts_terms_and_resets_scratch_like_c() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let f_of_a = typed_unary(&mut bank, "f", &a);
        let clause = clause_from(vec![
            literal(&mut bank, &f_of_a, &a, true),
            literal(&mut bank, &a, &b, true),
        ]);
        let mut dist = GenDistrib::new(bank.signature());

        dist.add_clause(&clause, 1);

        assert_eq!(entry_counts(&dist, a.f_code()), (3, 1));
        assert_eq!(entry_counts(&dist, b.f_code()), (1, 1));
        assert_eq!(entry_counts(&dist, f_of_a.f_code()), (1, 1));
        assert_eq!(dist.scratch_value(a.f_code()), Some(0));
        assert_eq!(dist.scratch_value(b.f_code()), Some(0));
        assert_eq!(dist.scratch_value(f_of_a.f_code()), Some(0));

        dist.add_clause(&clause, -1);

        assert_eq!(entry_counts(&dist, a.f_code()), (0, 0));
        assert_eq!(entry_counts(&dist, b.f_code()), (0, 0));
        assert_eq!(entry_counts(&dist, f_of_a.f_code()), (0, 0));
    }

    #[test]
    fn gen_distrib_clause_set_stack_adds_and_backtracks_from_requested_start() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "stack_a");
        let b = typed_const(&mut bank, "stack_b");
        let first = ClauseSet::from_clauses([clause_from(vec![literal(&mut bank, &a, &a, true)])]);
        let second = ClauseSet::from_clauses([clause_from(vec![literal(&mut bank, &b, &b, true)])]);
        let mut sets = PStack::new();
        sets.push(&first);
        sets.push(&second);
        let mut dist = GenDistrib::new(bank.signature());

        dist.add_clause_sets(&sets);
        assert_eq!(entry_counts(&dist, a.f_code()), (2, 1));
        assert_eq!(entry_counts(&dist, b.f_code()), (2, 1));

        dist.backtrack_clause_sets(&sets, 1);
        assert_eq!(entry_counts(&dist, a.f_code()), (2, 1));
        assert_eq!(entry_counts(&dist, b.f_code()), (0, 0));
    }

    #[test]
    fn gen_distrib_add_formula_counts_terms_and_resets_scratch_like_c() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "form_a");
        let b = typed_const(&mut bank, "form_b");
        let f_of_a = typed_unary(&mut bank, "form_f", &a);
        let g_of_b = typed_unary(&mut bank, "form_g", &b);
        let formula = wrapped_formula(typed_unary(&mut bank, "form_h", &f_of_a));
        let second = wrapped_formula(g_of_b);
        let mut dist = GenDistrib::new(bank.signature());

        dist.add_formula(bank.signature(), &formula, false, 1);
        dist.add_formula(bank.signature(), &second, false, 1);

        assert_eq!(entry_counts(&dist, a.f_code()), (1, 1));
        assert_eq!(entry_counts(&dist, f_of_a.f_code()), (1, 1));
        assert_eq!(entry_counts(&dist, b.f_code()), (1, 1));
        assert_eq!(entry_counts(&dist, second.formula().f_code()), (1, 1));
        assert_eq!(dist.scratch_value(a.f_code()), Some(0));
        assert_eq!(dist.scratch_value(f_of_a.f_code()), Some(0));
        assert_eq!(dist.scratch_value(b.f_code()), Some(0));
        assert_eq!(dist.scratch_value(second.formula().f_code()), Some(0));

        dist.add_formula(bank.signature(), &formula, false, -1);
        assert_eq!(entry_counts(&dist, a.f_code()), (0, 0));
        assert_eq!(entry_counts(&dist, f_of_a.f_code()), (0, 0));
    }

    #[test]
    fn gen_distrib_formula_set_stack_and_backtrack_preserve_c_start_and_trim_shape() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "form_stack_a");
        let b = typed_const(&mut bank, "form_stack_b");
        let f_of_a = typed_unary(&mut bank, "form_stack_f", &a);
        let g_of_b = typed_unary(&mut bank, "form_stack_g", &b);
        let mut first = FormulaSet::new();
        let mut second = FormulaSet::new();
        first.insert(wrapped_formula(f_of_a.clone()));
        second.insert(wrapped_formula(g_of_b.clone()));
        let mut stack = PStack::new();
        stack.push(&first);
        stack.push(&second);
        let mut dist = GenDistrib::new(bank.signature());

        dist.add_formula_sets(bank.signature(), &stack, false);
        assert_eq!(entry_counts(&dist, a.f_code()), (1, 1));
        assert_eq!(entry_counts(&dist, f_of_a.f_code()), (1, 1));
        assert_eq!(entry_counts(&dist, b.f_code()), (1, 1));
        assert_eq!(entry_counts(&dist, g_of_b.f_code()), (1, 1));

        dist.backtrack_formula_sets(bank.signature(), &stack, 1);
        assert_eq!(entry_counts(&dist, a.f_code()), (1, 1));
        assert_eq!(entry_counts(&dist, f_of_a.f_code()), (1, 1));
        assert_eq!(entry_counts(&dist, b.f_code()), (0, 0));
        assert_eq!(entry_counts(&dist, g_of_b.f_code()), (0, 0));
    }

    #[test]
    fn clause_compute_d_rel_applies_generosity_limit_and_resets_scratch() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "drel_a");
        let b = typed_const(&mut bank, "drel_b");
        let c = typed_const(&mut bank, "drel_c");
        let clause_a = clause_from(vec![literal(&mut bank, &a, &a, true)]);
        let clause_b = clause_from(vec![literal(&mut bank, &b, &b, true)]);
        let clause_c = clause_from(vec![literal(&mut bank, &c, &c, true)]);
        let current = clause_from(vec![
            literal(&mut bank, &a, &b, true),
            literal(&mut bank, &c, &c, true),
        ]);
        let mut dist = GenDistrib::new(bank.signature());
        dist.add_clause(&clause_a, 1);
        for _ in 0..2 {
            dist.add_clause(&clause_b, 1);
        }
        for _ in 0..3 {
            dist.add_clause(&clause_c, 1);
        }

        let mut res = PStack::new();
        clause_compute_d_rel(
            &mut dist,
            GeneralityMeasure::Terms,
            10.0,
            1,
            &current,
            &mut res,
        );

        assert_eq!(res.as_slice(), &[a.f_code(), b.f_code()]);
        assert_eq!(dist.scratch_value(a.f_code()), Some(0));
        assert_eq!(dist.scratch_value(b.f_code()), Some(0));
        assert_eq!(dist.scratch_value(c.f_code()), Some(0));
    }

    #[test]
    fn formula_compute_d_rel_applies_generosity_limit_and_resets_scratch() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "form_drel_a");
        let b = typed_const(&mut bank, "form_drel_b");
        let c = typed_const(&mut bank, "form_drel_c");
        let form_a = wrapped_formula(a.clone());
        let form_b = wrapped_formula(b.clone());
        let form_c = wrapped_formula(c.clone());
        let current = wrapped_formula(typed_unary(&mut bank, "form_drel_f", &c));
        let mut dist = GenDistrib::new(bank.signature());
        dist.add_formula(bank.signature(), &form_a, false, 1);
        for _ in 0..2 {
            dist.add_formula(bank.signature(), &form_b, false, 1);
        }
        for _ in 0..3 {
            dist.add_formula(bank.signature(), &form_c, false, 1);
        }
        dist.add_formula(bank.signature(), &current, false, 1);

        let mut res = PStack::new();
        formula_compute_d_rel(
            &mut dist,
            DRelSelectionParams {
                gen_measure: GeneralityMeasure::Terms,
                benevolence: 10.0,
                generosity: 1,
            },
            bank.signature(),
            &current,
            &mut res,
            false,
        );

        assert_eq!(res.as_slice(), &[current.formula().f_code(), c.f_code()]);
        assert_eq!(dist.scratch_value(current.formula().f_code()), Some(0));
        assert_eq!(dist.scratch_value(c.f_code()), Some(0));
    }

    #[test]
    fn clause_compute_d_rel_filters_symbols_below_internal_boundary() {
        let mut bank = test_bank();
        let true_term = bank.create_const_term(SIG_TRUE_CODE).unwrap();
        let false_term = bank.create_const_term(SIG_FALSE_CODE).unwrap();
        let clause = clause_from(vec![literal(&mut bank, &true_term, &false_term, true)]);
        let mut dist = GenDistrib::new(bank.signature());
        dist.add_clause(&clause, 1);
        let mut res = PStack::new();

        clause_compute_d_rel(
            &mut dist,
            GeneralityMeasure::Terms,
            10.0,
            0,
            &clause,
            &mut res,
        );

        assert!(res.is_empty());
    }

    #[test]
    fn formula_compute_d_rel_filters_symbols_below_internal_boundary() {
        let mut bank = test_bank();
        let true_term = bank.create_const_term(SIG_TRUE_CODE).unwrap();
        let formula = wrapped_formula(true_term);
        let mut dist = GenDistrib::new(bank.signature());
        dist.add_formula(bank.signature(), &formula, false, 1);
        let mut res = PStack::new();

        formula_compute_d_rel(
            &mut dist,
            DRelSelectionParams {
                gen_measure: GeneralityMeasure::Terms,
                benevolence: 10.0,
                generosity: 0,
            },
            bank.signature(),
            &formula,
            &mut res,
            false,
        );

        assert!(res.is_empty());
    }

    #[test]
    fn formula_compute_d_rel_trims_deep_conjecture_implication_consequents_like_c() {
        let mut bank = test_bank();
        let conclusion = typed_const(&mut bank, "form_trim_conclusion");
        let premises = (0..10)
            .map(|index| typed_const(&mut bank, &format!("form_trim_premise_{index}")))
            .collect::<Vec<_>>();
        let formula_term = implication_chain(bank.signature(), &premises, &conclusion);
        let mut formula = wrapped_formula(formula_term);
        formula.set_tptp_type(CP_TYPE_CONJECTURE);
        let mut dist = GenDistrib::new(bank.signature());
        dist.add_formula(bank.signature(), &formula, true, 1);
        let mut res = PStack::new();

        formula_compute_d_rel(
            &mut dist,
            DRelSelectionParams {
                gen_measure: GeneralityMeasure::Terms,
                benevolence: 10.0,
                generosity: 0,
            },
            bank.signature(),
            &formula,
            &mut res,
            true,
        );

        assert_eq!(entry_counts(&dist, conclusion.f_code()), (1, 1));
        for premise in premises {
            assert_eq!(entry_counts(&dist, premise.f_code()), (0, 0));
        }
        assert_eq!(res.as_slice(), &[conclusion.f_code()]);
    }

    #[test]
    fn gen_distrib_debug_string_matches_c_totals_rows_and_limit_boundary() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "print_a");
        let b = typed_const(&mut bank, "print_b");
        let f_of_a = typed_unary(&mut bank, "print_f", &a);
        let clause = clause_from(vec![literal(&mut bank, &f_of_a, &b, true)]);
        let mut dist = GenDistrib::new(bank.signature());
        dist.add_clause(&clause, 1);

        let rendered = dist.debug_string(bank.signature(), 4);
        let mut lines = rendered.lines();
        let header = lines.next().unwrap();

        assert!(header.starts_with("% GenDist 0x"));
        assert!(header.ends_with(" 3 3"));
        assert_eq!(
            lines.collect::<Vec<_>>(),
            vec![
                format!(
                    "% {:<30} ({:8} = {:8}): {:8}  {:8}",
                    "print_a",
                    a.f_code(),
                    a.f_code(),
                    1,
                    1
                ),
                format!(
                    "% {:<30} ({:8} = {:8}): {:8}  {:8}",
                    "print_b",
                    b.f_code(),
                    b.f_code(),
                    1,
                    1
                ),
                format!(
                    "% {:<30} ({:8} = {:8}): {:8}  {:8}",
                    "print_f",
                    f_of_a.f_code(),
                    f_of_a.f_code(),
                    1,
                    1
                ),
            ]
        );

        let first_only = dist.debug_string(bank.signature(), 2);
        assert!(first_only.contains("print_a"));
        assert!(!first_only.contains("print_b"));
    }
}
