use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::rc::Rc;
use std::sync::atomic::{AtomicI64, Ordering as AtomicOrdering};

use crate::basics::error::Diagnostic;
use crate::basics::partial_orderings::CompareResult;
use crate::learn::patterns::{pattern_term_compare, pattern_term_print_string, PatternSubst};
use crate::learn::termtops::{alt_term_top, cs_term_top, es_term_top, term_top};
use crate::terms::functypes::FunCode;
use crate::terms::termbanks::TermBank;
use crate::terms::termtypes::{DerefType, Term};
use crate::terms::termvars::is_alt_var;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IndexType(i32);

impl IndexType {
    pub const NO_INDEX: Self = Self(0);
    pub const ARITY: Self = Self(1);
    pub const SYMBOL: Self = Self(2);
    pub const TOP: Self = Self(4);
    pub const ALT_TOP: Self = Self(8);
    pub const CS_TOP: Self = Self(16);
    pub const ES_TOP: Self = Self(32);
    pub const IDENTITY: Self = Self(64);
    pub const EMPTY: Self = Self(128);
    pub const DYNAMIC: Self = Self(
        Self::ARITY.0
            | Self::SYMBOL.0
            | Self::TOP.0
            | Self::ALT_TOP.0
            | Self::CS_TOP.0
            | Self::ES_TOP.0
            | Self::IDENTITY.0,
    );

    #[must_use]
    pub const fn bits(self) -> i32 {
        self.0
    }
}

pub const INDEX_DYNAMIC_DEPTH: i32 = 0;

pub const INDEX_FUN_NAMES: [&str; 10] = [
    "IndexNoIndex",
    "IndexDynamic",
    "IndexArity",
    "IndexSymbol",
    "IndexTop",
    "IndexAltTop",
    "IndexCSTop",
    "IndexESTop",
    "IndexIdentity",
    "IndexEmpty",
];

static INDEX_COUNTER: AtomicI64 = AtomicI64::new(0);

#[derive(Clone, Debug)]
pub struct IndexTerm {
    term: Term,
    subst: Rc<PatternSubst>,
    key: i64,
}

#[derive(Clone, Debug)]
pub struct TSMIndex {
    ident: i64,
    index_type: IndexType,
    depth: i32,
    count: i64,
    subst: Rc<PatternSubst>,
    symbol_index: BTreeMap<FunCode, i64>,
    term_index: BTreeSet<IndexTerm>,
}

impl IndexTerm {
    #[must_use]
    pub fn new(term: Term, subst: PatternSubst, key: i64) -> Self {
        Self::new_shared(term, Rc::new(subst), key)
    }

    fn new_shared(term: Term, subst: Rc<PatternSubst>, key: i64) -> Self {
        Self { term, subst, key }
    }

    #[must_use]
    pub const fn term(&self) -> &Term {
        &self.term
    }

    #[must_use]
    pub const fn key(&self) -> i64 {
        self.key
    }
}

impl PartialEq for IndexTerm {
    fn eq(&self, other: &Self) -> bool {
        index_term_order(self, other) == Ordering::Equal
    }
}

impl Eq for IndexTerm {}

impl PartialOrd for IndexTerm {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for IndexTerm {
    fn cmp(&self, other: &Self) -> Ordering {
        index_term_order(self, other)
    }
}

impl TSMIndex {
    /// Allocates a C-shaped `TSMIndexCell`.
    ///
    /// # Panics
    ///
    /// Panics if `index_type` is `IndexNoIndex`, if a top index receives a
    /// non-positive depth, or if a dynamic/composite type is used where the C
    /// code expects one concrete index kind.
    #[must_use]
    pub fn new(index_type: IndexType, depth: i32, subst: PatternSubst) -> Self {
        Self::new_shared(index_type, depth, Rc::new(subst))
    }

    pub(crate) fn new_shared(index_type: IndexType, depth: i32, subst: Rc<PatternSubst>) -> Self {
        assert_ne!(index_type, IndexType::NO_INDEX);
        match index_type {
            IndexType::ARITY | IndexType::SYMBOL | IndexType::IDENTITY | IndexType::EMPTY => {}
            IndexType::TOP | IndexType::ALT_TOP | IndexType::CS_TOP | IndexType::ES_TOP => {
                assert!(depth > 0, "term-top indexes require a positive depth");
            }
            _ => panic!("unknown or composite TSM index type"),
        }
        Self {
            ident: INDEX_COUNTER.fetch_add(1, AtomicOrdering::Relaxed),
            index_type,
            depth,
            count: 0,
            subst,
            symbol_index: BTreeMap::new(),
            term_index: BTreeSet::new(),
        }
    }

    #[must_use]
    pub const fn ident(&self) -> i64 {
        self.ident
    }

    #[must_use]
    pub const fn index_type(&self) -> IndexType {
        self.index_type
    }

    #[must_use]
    pub const fn depth(&self) -> i32 {
        self.depth
    }

    #[must_use]
    pub const fn count(&self) -> i64 {
        self.count
    }

    #[cfg(test)]
    pub(crate) fn shares_subst(&self, subst: &Rc<PatternSubst>) -> bool {
        Rc::ptr_eq(&self.subst, subst)
    }

    /// Returns the index key for `term`, or `-1` when the term is absent.
    ///
    /// # Panics
    ///
    /// Panics if a symbol/top/identity lookup uses a substitution that does not
    /// bind the compared symbols, matching the C assertion that only total
    /// substitutions are expected here.
    pub fn find(&mut self, term: &Term, subst: &PatternSubst, bank: &TermBank) -> i64 {
        match self.index_type {
            IndexType::ARITY => {
                let result = usize_to_i64(term.arity());
                self.count = self.count.max(result + 1);
                result
            }
            IndexType::SYMBOL => {
                let key = index_symbol_key(term, subst);
                self.symbol_index.get(&key).copied().unwrap_or(-1)
            }
            IndexType::TOP | IndexType::ALT_TOP | IndexType::CS_TOP | IndexType::ES_TOP => {
                let top = any_term_top(self.index_type, term, self.depth, bank);
                let query = IndexTerm::new(top, subst.clone(), -1);
                self.term_index.get(&query).map_or(-1, |entry| entry.key)
            }
            IndexType::IDENTITY => {
                let query = IndexTerm::new(term.clone(), subst.clone(), -1);
                self.term_index.get(&query).map_or(-1, |entry| entry.key)
            }
            IndexType::EMPTY => -1,
            _ => panic!("unknown or composite TSM index type"),
        }
    }

    /// Inserts `term` into this TSM index and returns its dense key.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic when the term bank rejects a term-top or identity
    /// representative during insertion.
    ///
    /// # Panics
    ///
    /// Panics if inserting into `IndexEmpty`, or if a symbol/top/identity
    /// insertion uses a substitution that does not bind the compared symbols,
    /// matching the C assertion that only total substitutions are expected here.
    pub fn insert(&mut self, term: &Term, bank: &mut TermBank) -> Result<i64, Diagnostic> {
        match self.index_type {
            IndexType::ARITY => {
                let result = usize_to_i64(term.arity());
                self.count = self.count.max(result + 1);
                Ok(result)
            }
            IndexType::SYMBOL => {
                let key = index_symbol_key(term, self.subst.as_ref());
                if let Some(existing) = self.symbol_index.get(&key) {
                    Ok(*existing)
                } else {
                    let result = self.count;
                    self.symbol_index.insert(key, result);
                    self.count += 1;
                    Ok(result)
                }
            }
            IndexType::TOP | IndexType::ALT_TOP | IndexType::CS_TOP | IndexType::ES_TOP => {
                let top = any_term_top(self.index_type, term, self.depth, bank);
                self.insert_term_index(&top, bank)
            }
            IndexType::IDENTITY => self.insert_term_index(term, bank),
            IndexType::EMPTY => panic!("cannot insert term into IndexEmpty index"),
            _ => panic!("unknown or composite TSM index type"),
        }
    }

    /// Prints the current index in the C debug-comment shape.
    ///
    /// # Panics
    ///
    /// Panics if stored index terms cannot be rendered as total pattern terms.
    #[must_use]
    pub fn print_string(&self, bank: &TermBank, depth: i32) -> String {
        let indent = " ".repeat(usize::try_from(3_i32.saturating_mul(depth)).unwrap_or(0));
        match self.index_type {
            IndexType::ARITY => format!("# {indent}Index {} is arity index!\n", self.ident),
            IndexType::SYMBOL => self.print_symbol_index(bank, &indent),
            IndexType::TOP
            | IndexType::ALT_TOP
            | IndexType::CS_TOP
            | IndexType::ES_TOP
            | IndexType::IDENTITY => self.print_term_index(bank),
            IndexType::EMPTY => "# Index is empty index!\n".to_string(),
            _ => panic!("unknown or composite TSM index type"),
        }
    }

    fn insert_term_index(&mut self, term: &Term, bank: &mut TermBank) -> Result<i64, Diagnostic> {
        let query = IndexTerm::new_shared(term.clone(), Rc::clone(&self.subst), -1);
        if let Some(entry) = self.term_index.get(&query) {
            return Ok(entry.key);
        }

        let shared = bank.insert(term, DerefType::Never)?;
        let result = self.count;
        let entry = IndexTerm::new_shared(shared, Rc::clone(&self.subst), result);
        assert!(self.term_index.insert(entry));
        self.count += 1;
        Ok(result)
    }

    fn print_symbol_index(&self, bank: &TermBank, indent: &str) -> String {
        let mut output = format!(
            "# {indent}Index {} is symbol index!\n# {indent}PSymbol         Index  FCode     (Symbol)\n",
            self.ident
        );
        let mut alternatives = 0_i64;
        for (symbol, index) in &self.symbol_index {
            let f_code = self.subst.original_symbol(*symbol);
            let name = if f_code > 0 && f_code <= bank.signature().f_count() {
                bank.signature().find_name(f_code).unwrap_or("variable")
            } else {
                "variable"
            };
            let _ = writeln!(
                output,
                "# {indent}#{symbol:10} :{index:7}  {f_code:7}     {name}"
            );
            alternatives += 1;
        }
        let _ = writeln!(output, "# {indent}{alternatives} alternatives in the index");
        output
    }

    fn print_term_index(&self, bank: &TermBank) -> String {
        let mut output = format!("# Index is {} index!\n", get_index_name(self.index_type));
        let mut alternatives = 0_i64;
        for entry in &self.term_index {
            let mut subst = self.subst.as_ref().clone();
            let _ = writeln!(
                output,
                "# {:3} : {}",
                entry.key,
                pattern_term_print_string(&mut subst, &entry.term, bank.signature())
            );
            alternatives += 1;
        }
        let _ = writeln!(output, "# {alternatives} alternatives in the index");
        output
    }
}

#[must_use]
pub fn get_index_type(name: &str) -> Option<IndexType> {
    let position = INDEX_FUN_NAMES
        .iter()
        .position(|candidate| *candidate == name)?;
    if position == 0 {
        return Some(IndexType::NO_INDEX);
    }
    if position == 1 {
        return Some(IndexType::DYNAMIC);
    }
    Some(IndexType(1 << (position - 2)))
}

#[must_use]
pub fn get_index_name(index_type: IndexType) -> &'static str {
    match index_type {
        IndexType::NO_INDEX => INDEX_FUN_NAMES[0],
        IndexType::ARITY => INDEX_FUN_NAMES[2],
        IndexType::SYMBOL => INDEX_FUN_NAMES[3],
        IndexType::TOP => INDEX_FUN_NAMES[4],
        IndexType::ALT_TOP => INDEX_FUN_NAMES[5],
        IndexType::CS_TOP => INDEX_FUN_NAMES[6],
        IndexType::ES_TOP => INDEX_FUN_NAMES[7],
        IndexType::IDENTITY => INDEX_FUN_NAMES[8],
        IndexType::EMPTY => INDEX_FUN_NAMES[9],
        _ => INDEX_FUN_NAMES[1],
    }
}

#[must_use]
pub fn index_term_alloc(term: Term, subst: PatternSubst, key: i64) -> IndexTerm {
    IndexTerm::new(term, subst, key)
}

/// Compares two index terms using the C `IndexTermCompareFun` contract.
///
/// # Panics
///
/// Panics if pattern comparison returns `to_uncomparable`, matching the C
/// assertion that only total substitutions are expected in TSM indexes.
#[must_use]
pub fn index_term_compare_fun(left: &IndexTerm, right: &IndexTerm) -> i32 {
    match index_term_order(left, right) {
        Ordering::Less => -1,
        Ordering::Equal => 0,
        Ordering::Greater => 1,
    }
}

#[must_use]
pub fn tsm_index_alloc(index_type: IndexType, depth: i32, subst: PatternSubst) -> TSMIndex {
    TSMIndex::new(index_type, depth, subst)
}

#[must_use]
pub(crate) fn tsm_index_alloc_shared(
    index_type: IndexType,
    depth: i32,
    subst: Rc<PatternSubst>,
) -> TSMIndex {
    TSMIndex::new_shared(index_type, depth, subst)
}

/// Returns the index key for `term`, or `-1` when absent.
///
/// # Panics
///
/// Panics under the same internal-invariant conditions as [`TSMIndex::find`].
pub fn tsm_index_find(
    index: &mut TSMIndex,
    term: &Term,
    subst: &PatternSubst,
    bank: &TermBank,
) -> i64 {
    index.find(term, subst, bank)
}

/// Inserts `term` into a TSM index and returns its dense key.
///
/// # Errors
///
/// Returns a diagnostic when the term bank rejects a term representative.
///
/// # Panics
///
/// Panics under the same internal-invariant conditions as [`TSMIndex::insert`].
pub fn tsm_index_insert(
    index: &mut TSMIndex,
    term: &Term,
    bank: &mut TermBank,
) -> Result<i64, Diagnostic> {
    index.insert(term, bank)
}

/// Prints the current index in the C debug-comment shape.
///
/// # Panics
///
/// Panics under the same internal-invariant conditions as
/// [`TSMIndex::print_string`].
#[must_use]
pub fn tsm_index_print_string(index: &TSMIndex, bank: &TermBank, depth: i32) -> String {
    index.print_string(bank, depth)
}

fn index_term_order(left: &IndexTerm, right: &IndexTerm) -> Ordering {
    let mut left_subst = left.subst.as_ref().clone();
    let mut right_subst = right.subst.as_ref().clone();
    compare_result_to_ordering(pattern_term_compare(
        &mut left_subst,
        &left.term,
        &mut right_subst,
        &right.term,
    ))
}

fn compare_result_to_ordering(result: CompareResult) -> Ordering {
    match result {
        CompareResult::Lesser => Ordering::Less,
        CompareResult::Equal => Ordering::Equal,
        CompareResult::Greater => Ordering::Greater,
        CompareResult::Uncomparable => panic!("only total substitutions expected here"),
        CompareResult::Unknown | CompareResult::NotGreaterEqual | CompareResult::NotLessEqual => {
            panic!("pattern term comparison returned non-C result")
        }
    }
}

fn index_symbol_key(term: &Term, subst: &PatternSubst) -> FunCode {
    if term.is_free_var() && is_alt_var(term) {
        return term.f_code();
    }
    let key = subst.symbol_value(term.f_code());
    assert_ne!(key, 0, "index symbol must be bound by pattern substitution");
    key
}

fn any_term_top(index_type: IndexType, term: &Term, depth: i32, bank: &TermBank) -> Term {
    match index_type {
        IndexType::TOP => term_top(term, depth, bank.vars()),
        IndexType::ALT_TOP => alt_term_top(term, depth, bank.vars()),
        IndexType::CS_TOP => cs_term_top(term, depth, bank.vars()),
        IndexType::ES_TOP => es_term_top(term, depth, bank.vars()),
        _ => panic!("wrong term-top index type"),
    }
}

fn usize_to_i64(value: usize) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

#[cfg(test)]
mod tests {
    use super::{
        get_index_name, get_index_type, index_term_alloc, index_term_compare_fun, tsm_index_alloc,
        tsm_index_find, tsm_index_insert, tsm_index_print_string, IndexType, TSMIndex,
        INDEX_DYNAMIC_DEPTH, INDEX_FUN_NAMES,
    };
    use crate::inout::scanner::Scanner;
    use crate::learn::patterns::{pattern_term_compute, PatternSubst};
    use crate::terms::signature::Signature;
    use crate::terms::termbanks::TermBank;
    use crate::terms::termtypes::Term;
    use crate::terms::typebanks::TypeBank;
    use std::rc::Rc;

    #[test]
    fn index_type_names_and_values_match_c_surface() {
        assert_eq!(INDEX_FUN_NAMES[0], "IndexNoIndex");
        assert_eq!(INDEX_FUN_NAMES[1], "IndexDynamic");
        assert_eq!(INDEX_DYNAMIC_DEPTH, 0);
        assert_eq!(IndexType::NO_INDEX.bits(), 0);
        assert_eq!(IndexType::ARITY.bits(), 1);
        assert_eq!(IndexType::SYMBOL.bits(), 2);
        assert_eq!(IndexType::TOP.bits(), 4);
        assert_eq!(IndexType::ALT_TOP.bits(), 8);
        assert_eq!(IndexType::CS_TOP.bits(), 16);
        assert_eq!(IndexType::ES_TOP.bits(), 32);
        assert_eq!(IndexType::IDENTITY.bits(), 64);
        assert_eq!(IndexType::EMPTY.bits(), 128);
    }

    #[test]
    fn get_index_type_expands_dynamic_like_c_helper() {
        assert_eq!(get_index_type("IndexNoIndex"), Some(IndexType::NO_INDEX));
        assert_eq!(get_index_type("IndexArity"), Some(IndexType::ARITY));
        assert_eq!(get_index_type("IndexEmpty"), Some(IndexType::EMPTY));
        assert_eq!(get_index_type("missing"), None);
        assert_eq!(
            get_index_type("IndexDynamic").map(IndexType::bits),
            Some(
                IndexType::ARITY.bits()
                    | IndexType::SYMBOL.bits()
                    | IndexType::TOP.bits()
                    | IndexType::ALT_TOP.bits()
                    | IndexType::CS_TOP.bits()
                    | IndexType::ES_TOP.bits()
                    | IndexType::IDENTITY.bits()
            )
        );
    }

    #[test]
    fn get_index_name_maps_composites_to_dynamic_name() {
        assert_eq!(get_index_name(IndexType::NO_INDEX), "IndexNoIndex");
        assert_eq!(get_index_name(IndexType::TOP), "IndexTop");
        assert_eq!(get_index_name(IndexType::DYNAMIC), "IndexDynamic");
    }

    #[test]
    fn shared_constructor_reuses_pattern_substitution() {
        let bank = test_bank();
        let subst = Rc::new(PatternSubst::new(bank.signature()));

        let index = TSMIndex::new_shared(IndexType::SYMBOL, 0, Rc::clone(&subst));

        assert!(Rc::ptr_eq(&index.subst, &subst));
        assert_eq!(Rc::strong_count(&subst), 2);
    }

    #[test]
    fn index_term_compare_uses_pattern_substitution() {
        let mut left_bank = test_bank();
        let left = parse_in_bank(&mut left_bank, "f(a)");
        let mut left_subst = PatternSubst::new(left_bank.signature());
        assert!(pattern_term_compute(&mut left_subst, &left));

        let mut right_bank = test_bank();
        let right = parse_in_bank(&mut right_bank, "g(b)");
        let mut right_subst = PatternSubst::new(right_bank.signature());
        assert!(pattern_term_compute(&mut right_subst, &right));

        let left_index = index_term_alloc(left, left_subst, 7);
        let right_index = index_term_alloc(right, right_subst, 9);

        assert_eq!(index_term_compare_fun(&left_index, &right_index), 0);
    }

    #[test]
    fn arity_index_find_and_insert_update_count_like_c() {
        let mut bank = test_bank();
        let binary = parse_in_bank(&mut bank, "f(a,b)");
        let constant = parse_in_bank(&mut bank, "a");
        let subst = bound_subst(&bank, &[&binary, &constant]);
        let mut index = tsm_index_alloc(IndexType::ARITY, 0, subst.clone());

        assert_eq!(tsm_index_insert(&mut index, &binary, &mut bank).unwrap(), 2);
        assert_eq!(index.count(), 3);
        assert_eq!(tsm_index_find(&mut index, &constant, &subst, &bank), 0);
        assert_eq!(index.count(), 3);
    }

    #[test]
    fn symbol_index_assigns_dense_keys_and_reuses_existing_symbols() {
        let mut bank = test_bank();
        let first = parse_in_bank(&mut bank, "f(a)");
        let same_symbol = parse_in_bank(&mut bank, "f(b)");
        let other = parse_in_bank(&mut bank, "g(a)");
        let subst = bound_subst(&bank, &[&first, &same_symbol, &other]);
        let mut index = tsm_index_alloc(IndexType::SYMBOL, 0, subst.clone());

        assert_eq!(tsm_index_insert(&mut index, &first, &mut bank).unwrap(), 0);
        assert_eq!(
            tsm_index_insert(&mut index, &same_symbol, &mut bank).unwrap(),
            0
        );
        assert_eq!(tsm_index_insert(&mut index, &other, &mut bank).unwrap(), 1);
        assert_eq!(tsm_index_find(&mut index, &first, &subst, &bank), 0);
        assert_eq!(index.count(), 2);
    }

    #[test]
    fn top_index_reuses_terms_with_same_selected_top() {
        let mut bank = test_bank();
        let first = parse_in_bank(&mut bank, "f(a)");
        let same_top = parse_in_bank(&mut bank, "f(b)");
        let other = parse_in_bank(&mut bank, "g(a)");
        let subst = bound_subst(&bank, &[&first, &same_top, &other]);
        let mut index = tsm_index_alloc(IndexType::TOP, 1, subst.clone());

        assert_eq!(tsm_index_insert(&mut index, &first, &mut bank).unwrap(), 0);
        assert_eq!(
            tsm_index_insert(&mut index, &same_top, &mut bank).unwrap(),
            0
        );
        assert_eq!(tsm_index_insert(&mut index, &other, &mut bank).unwrap(), 1);
        assert_eq!(tsm_index_find(&mut index, &same_top, &subst, &bank), 0);
        assert_eq!(index.count(), 2);
    }

    #[test]
    fn identity_index_distinguishes_full_pattern_terms() {
        let mut bank = test_bank();
        let first = parse_in_bank(&mut bank, "f(a)");
        let second = parse_in_bank(&mut bank, "f(b)");
        let subst = bound_subst(&bank, &[&first, &second]);
        let mut index = tsm_index_alloc(IndexType::IDENTITY, 0, subst.clone());

        assert_eq!(tsm_index_insert(&mut index, &first, &mut bank).unwrap(), 0);
        assert_eq!(tsm_index_insert(&mut index, &second, &mut bank).unwrap(), 1);
        assert_eq!(tsm_index_insert(&mut index, &first, &mut bank).unwrap(), 0);
        assert_eq!(tsm_index_find(&mut index, &second, &subst, &bank), 1);
    }

    #[test]
    fn index_print_string_uses_c_comment_shape() {
        let mut bank = test_bank();
        let first = parse_in_bank(&mut bank, "f(a)");
        let subst = bound_subst(&bank, &[&first]);
        let mut index = tsm_index_alloc(IndexType::SYMBOL, 0, subst);
        assert_eq!(tsm_index_insert(&mut index, &first, &mut bank).unwrap(), 0);

        let printed = tsm_index_print_string(&index, &bank, 1);

        assert!(printed.starts_with("#    Index "));
        assert!(printed.contains("is symbol index!"));
        assert!(printed.contains("alternatives in the index"));
    }

    fn test_bank() -> TermBank {
        TermBank::new(Signature::new(TypeBank::new())).expect("term bank allocation")
    }

    fn parse_in_bank(bank: &mut TermBank, source: &str) -> Term {
        let mut scanner = Scanner::from_user_string(source, false).expect("scanner allocation");
        bank.parse_term_simple(&mut scanner)
            .expect("simple term parse")
    }

    fn bound_subst(bank: &TermBank, terms: &[&Term]) -> PatternSubst {
        let mut subst = PatternSubst::new(bank.signature());
        for term in terms {
            pattern_term_compute(&mut subst, term);
        }
        subst
    }
}
