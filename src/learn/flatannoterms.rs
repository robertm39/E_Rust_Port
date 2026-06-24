use crate::basics::numtrees::NumTree;
use crate::terms::termbanks::TermBank;
use crate::terms::termtypes::Term;
use std::fmt::Write as _;

#[derive(Clone, Debug, PartialEq)]
pub struct FlatAnnoTerm {
    term: Term,
    eval: f64,
    eval_weight: f64,
    sources: i64,
    next: Option<Box<FlatAnnoTerm>>,
}

pub type FlatAnnoSet = NumTree<FlatAnnoTerm, ()>;

impl FlatAnnoTerm {
    #[must_use]
    pub const fn new(term: Term, eval: f64, eval_weight: f64, sources: i64) -> Self {
        Self {
            term,
            eval,
            eval_weight,
            sources,
            next: None,
        }
    }

    #[must_use]
    pub const fn term(&self) -> &Term {
        &self.term
    }

    #[must_use]
    pub const fn eval(&self) -> f64 {
        self.eval
    }

    #[must_use]
    pub const fn eval_weight(&self) -> f64 {
        self.eval_weight
    }

    #[must_use]
    pub const fn sources(&self) -> i64 {
        self.sources
    }

    #[must_use]
    pub fn next(&self) -> Option<&FlatAnnoTerm> {
        self.next.as_deref()
    }

    pub fn set_next(&mut self, next: Option<FlatAnnoTerm>) {
        self.next = next.map(Box::new);
    }
}

#[must_use]
pub const fn flat_anno_set_alloc() -> FlatAnnoSet {
    FlatAnnoSet::new()
}

pub fn flat_anno_set_add_term(set: &mut FlatAnnoSet, term: FlatAnnoTerm) -> bool {
    let key = term.term.entry_no();
    if let Some(entry) = set.find_mut(key) {
        let existing = &mut entry.val1;
        existing.eval = (term.eval * term.eval_weight + existing.eval * existing.eval_weight)
            / (term.eval_weight + existing.eval_weight);
        existing.eval_weight += term.eval_weight;
        existing.sources += term.sources;
        false
    } else {
        set.store(key, term, ());
        true
    }
}

#[must_use]
pub fn flat_anno_term_print_string(term: &FlatAnnoTerm, bank: &TermBank) -> String {
    let mut result = String::new();
    let write_result = write!(
        &mut result,
        "{} : {:.6}. /* EvalWeight: {:.6}, Id: {} */",
        bank.term_string(&term.term, true),
        term.eval,
        term.eval_weight,
        term.term.entry_no()
    );
    debug_assert!(write_result.is_ok());
    result
}

#[must_use]
pub fn flat_anno_set_print_string(set: &FlatAnnoSet, bank: &TermBank) -> String {
    let mut result = String::new();
    for (_key, entry) in set.iter() {
        result.push_str(&flat_anno_term_print_string(&entry.val1, bank));
        result.push('\n');
    }
    result
}

#[must_use]
pub fn flat_anno_set_size(set: &FlatAnnoSet) -> i64 {
    let mut result = 0_i64;
    for (_key, entry) in set.iter() {
        result += entry.val1.sources;
    }
    result
}

/// Insert a flat annotated term for each subterm of `term`.
///
/// # Panics
///
/// Panics when a traversed term has an uninitialized argument, matching the C
/// assertion that `t->args` is available for every non-leaf term.
pub fn flat_anno_term_flatten(set: &mut FlatAnnoSet, term: &FlatAnnoTerm) -> i64 {
    let mut result = 0_i64;
    let mut stack = vec![term.term.clone()];
    while let Some(current) = stack.pop() {
        let handle = FlatAnnoTerm::new(current.clone(), term.eval, term.eval_weight, term.sources);
        flat_anno_set_add_term(set, handle);
        result += 1;
        for index in 0..current.arity() {
            let arg = current
                .argument(index)
                .unwrap_or_else(|| panic!("term argument {index} is uninitialized"));
            stack.push(arg);
        }
    }
    result
}

pub fn flat_anno_set_flatten(set: &mut FlatAnnoSet, to_flatten: &FlatAnnoSet) -> i64 {
    let mut result = 0_i64;
    let terms = to_flatten
        .iter()
        .map(|(_key, entry)| entry.val1.clone())
        .collect::<Vec<_>>();
    for term in terms {
        result += flat_anno_term_flatten(set, &term);
    }
    result
}

#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn flat_anno_set_eval_average(set: &FlatAnnoSet) -> f64 {
    if set.is_empty() {
        return 0.0;
    }

    let mut sources = 0_i64;
    let mut result = 0.0;
    for (_key, entry) in set.iter() {
        result += entry.val1.eval;
        sources += entry.val1.sources;
    }
    result / sources as f64
}

#[must_use]
pub fn flat_anno_set_eval_weighted_average(set: &FlatAnnoSet) -> f64 {
    if set.is_empty() {
        return 0.0;
    }

    let mut weight = 0.0;
    let mut result = 0.0;
    for (_key, entry) in set.iter() {
        result += entry.val1.eval_weight * entry.val1.eval;
        weight += entry.val1.eval_weight;
    }
    result / weight
}

#[cfg(test)]
mod tests {
    use super::{
        flat_anno_set_add_term, flat_anno_set_alloc, flat_anno_set_eval_average,
        flat_anno_set_eval_weighted_average, flat_anno_set_flatten, flat_anno_set_print_string,
        flat_anno_set_size, flat_anno_term_flatten, flat_anno_term_print_string, FlatAnnoSet,
        FlatAnnoTerm,
    };
    use crate::inout::scanner::Scanner;
    use crate::terms::signature::Signature;
    use crate::terms::termbanks::TermBank;
    use crate::terms::termtypes::Term;
    use crate::terms::typebanks::TypeBank;

    fn assert_close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < f64::EPSILON,
            "expected {expected}, got {actual}"
        );
    }

    fn term_bank() -> TermBank {
        let mut signature = Signature::new(TypeBank::new());
        signature.insert_internal_codes().unwrap();
        TermBank::new(signature).unwrap()
    }

    fn parse_in_bank(bank: &mut TermBank, source: &str) -> Term {
        let mut scanner =
            Scanner::from_user_string(source, false).unwrap_or_else(|err| panic!("{err}"));
        bank.parse_term_simple(&mut scanner)
            .unwrap_or_else(|err| panic!("{err}"))
    }

    fn find_term(set: &FlatAnnoSet, term: &Term) -> FlatAnnoTerm {
        set.find(term.entry_no())
            .unwrap_or_else(|| panic!("flat annotation for term {} not found", term.entry_no()))
            .val1
            .clone()
    }

    #[test]
    fn flat_anno_set_add_term_merges_duplicate_entry_numbers() {
        let mut bank = term_bank();
        let term = parse_in_bank(&mut bank, "f(a)");
        let mut set = flat_anno_set_alloc();

        assert!(flat_anno_set_add_term(
            &mut set,
            FlatAnnoTerm::new(term.clone(), 10.0, 2.0, 2)
        ));
        assert!(!flat_anno_set_add_term(
            &mut set,
            FlatAnnoTerm::new(term.clone(), 20.0, 3.0, 3)
        ));

        let merged = find_term(&set, &term);
        assert_close(merged.eval(), 16.0);
        assert_close(merged.eval_weight(), 5.0);
        assert_eq!(merged.sources(), 5);
    }

    #[test]
    fn flat_anno_term_flatten_visits_all_subterms_and_merges_duplicates() {
        let mut bank = term_bank();
        let root = parse_in_bank(&mut bank, "f(a,a)");
        let first_arg = root.argument(0).unwrap();
        let mut set = flat_anno_set_alloc();
        let term = FlatAnnoTerm::new(root.clone(), 7.0, 1.5, 1);

        assert_eq!(flat_anno_term_flatten(&mut set, &term), 3);
        assert_eq!(set.nodes(), 2);
        assert_eq!(flat_anno_set_size(&set), 3);

        let root_flat = find_term(&set, &root);
        assert_close(root_flat.eval(), 7.0);
        assert_close(root_flat.eval_weight(), 1.5);
        assert_eq!(root_flat.sources(), 1);

        let arg_flat = find_term(&set, &first_arg);
        assert_close(arg_flat.eval(), 7.0);
        assert_close(arg_flat.eval_weight(), 3.0);
        assert_eq!(arg_flat.sources(), 2);
    }

    #[test]
    fn flat_anno_set_flatten_flattens_sorted_source_terms() {
        let mut bank = term_bank();
        let left = parse_in_bank(&mut bank, "g(a)");
        let right = parse_in_bank(&mut bank, "h(b)");
        let mut source = flat_anno_set_alloc();
        flat_anno_set_add_term(&mut source, FlatAnnoTerm::new(left.clone(), 1.0, 1.0, 1));
        flat_anno_set_add_term(&mut source, FlatAnnoTerm::new(right.clone(), 2.0, 1.0, 1));

        let mut flattened = flat_anno_set_alloc();
        assert_eq!(flat_anno_set_flatten(&mut flattened, &source), 4);
        assert_eq!(flat_anno_set_size(&flattened), 4);
        assert!(flattened.find(left.entry_no()).is_some());
        assert!(flattened.find(right.entry_no()).is_some());
    }

    #[test]
    fn flat_anno_set_averages_preserve_c_formulas() {
        let mut bank = term_bank();
        let left = parse_in_bank(&mut bank, "a");
        let right = parse_in_bank(&mut bank, "b");
        let mut set = flat_anno_set_alloc();
        flat_anno_set_add_term(&mut set, FlatAnnoTerm::new(left, 10.0, 2.0, 2));
        flat_anno_set_add_term(&mut set, FlatAnnoTerm::new(right, 20.0, 3.0, 3));

        assert_close(flat_anno_set_eval_average(&FlatAnnoSet::new()), 0.0);
        assert_close(
            flat_anno_set_eval_weighted_average(&FlatAnnoSet::new()),
            0.0,
        );
        assert_close(flat_anno_set_eval_average(&set), 6.0);
        assert_close(flat_anno_set_eval_weighted_average(&set), 16.0);
    }

    #[test]
    fn flat_anno_printing_uses_c_debug_shape() {
        let mut bank = term_bank();
        let term = parse_in_bank(&mut bank, "f(a)");
        let flat = FlatAnnoTerm::new(term.clone(), 1.5, 2.5, 3);
        let expected = format!(
            "f(a) : 1.500000. /* EvalWeight: 2.500000, Id: {} */",
            term.entry_no()
        );

        assert_eq!(flat_anno_term_print_string(&flat, &bank), expected);

        let mut set = flat_anno_set_alloc();
        flat_anno_set_add_term(&mut set, flat);
        assert_eq!(
            flat_anno_set_print_string(&set, &bank),
            format!("{expected}\n")
        );
    }

    #[test]
    fn flat_anno_term_next_field_matches_c_list_surface() {
        let mut bank = term_bank();
        let term = parse_in_bank(&mut bank, "a");
        let mut flat = FlatAnnoTerm::new(term.clone(), 1.0, 1.0, 1);
        assert!(flat.next().is_none());

        flat.set_next(Some(FlatAnnoTerm::new(term, 2.0, 3.0, 4)));

        let next = flat.next().unwrap();
        assert_close(next.eval(), 2.0);
        assert_close(next.eval_weight(), 3.0);
        assert_eq!(next.sources(), 4);
    }
}
