use crate::terms::termtrees::TermTree;
use crate::terms::termtypes::{term_identity_id, Term, TermProperties, TP_GARBAGE_FLAG};
use std::io::{self, Write};

pub const TERM_STORE_HASH_SIZE: usize = 8192 * 4;
pub const TERM_STORE_HASH_MASK: usize = TERM_STORE_HASH_SIZE - 1;

#[derive(Debug)]
pub struct TermCellStore {
    entries: i64,
    arg_count: i64,
    store: Vec<TermTree>,
}

impl Default for TermCellStore {
    fn default() -> Self {
        Self::new()
    }
}

impl TermCellStore {
    #[must_use]
    pub fn new() -> Self {
        Self {
            entries: 0,
            arg_count: 0,
            store: std::iter::repeat_with(TermTree::new)
                .take(TERM_STORE_HASH_SIZE)
                .collect(),
        }
    }

    pub fn exit(&mut self) {
        for tree in &mut self.store {
            tree.clear();
        }
        self.entries = 0;
        self.arg_count = 0;
    }

    #[must_use]
    pub const fn entries(&self) -> i64 {
        self.entries
    }

    #[must_use]
    pub const fn arg_count(&self) -> i64 {
        self.arg_count
    }

    /// Finds a matching term cell in the hashed tree bucket.
    ///
    /// # Panics
    ///
    /// Panics if `term` has uninitialized arguments needed by the hash or
    /// comparison key.
    pub fn find(&mut self, term: &Term) -> Option<Term> {
        let hash = term_cell_hash(term);
        self.store[hash].find(term)
    }

    /// Inserts a term cell into the hashed tree bucket.
    ///
    /// # Panics
    ///
    /// Panics if `term` has uninitialized arguments needed by the hash or
    /// comparison key, or if the term-tree comparison preconditions are unmet.
    pub fn insert(&mut self, term: Term) -> Option<Term> {
        let hash = term_cell_hash(&term);
        let arity = i64::try_from(term.arity()).unwrap_or(i64::MAX);
        let duplicate = self.store[hash].insert(term);
        if duplicate.is_none() {
            self.entries += 1;
            self.arg_count += arity;
        }
        duplicate
    }

    /// Extracts a term cell from the hashed tree bucket.
    ///
    /// # Panics
    ///
    /// Panics if `term` has uninitialized arguments needed by the hash or
    /// comparison key, or if the term-tree comparison preconditions are unmet.
    pub fn extract(&mut self, term: &Term) -> Option<Term> {
        let hash = term_cell_hash(term);
        let ret = self.store[hash].extract(term);
        if ret.is_some() {
            self.entries -= 1;
            self.arg_count -= i64::try_from(term.arity()).unwrap_or(i64::MAX);
        }
        assert!(self.entries >= 0);
        ret
    }

    /// Deletes a term cell from the hashed tree bucket.
    ///
    /// # Panics
    ///
    /// Panics if `term` has uninitialized arguments needed by the hash or
    /// comparison key, or if the term-tree comparison preconditions are unmet.
    pub fn delete(&mut self, term: &Term) -> bool {
        let hash = term_cell_hash(term);
        let ret = self.store[hash].delete(term);
        if ret {
            self.entries -= 1;
            self.arg_count -= i64::try_from(term.arity()).unwrap_or(i64::MAX);
        }
        assert!(self.entries >= 0);
        ret
    }

    pub fn set_prop(&self, props: TermProperties) {
        for tree in &self.store {
            tree.set_prop(props);
        }
    }

    pub fn del_prop(&self, props: TermProperties) {
        for tree in &self.store {
            tree.del_prop(props);
        }
    }

    #[must_use]
    pub fn terms(&self) -> Vec<Term> {
        self.store.iter().flat_map(TermTree::terms).collect()
    }

    #[must_use]
    pub fn count_nodes(&self) -> i64 {
        self.store.iter().map(TermTree::nodes).sum()
    }

    pub fn gc_sweep(&mut self, gc_state: TermProperties) -> i64 {
        let mut recovered = 0;
        for index in 0..self.store.len() {
            let delete = self.store[index]
                .terms()
                .into_iter()
                .filter(|term| term.give_props(TP_GARBAGE_FLAG) == gc_state)
                .collect::<Vec<_>>();
            for term in delete {
                if self.delete(&term) {
                    recovered += 1;
                }
            }
        }
        recovered
    }

    pub fn print_distrib(&self, output: &mut impl Write) -> io::Result<()> {
        for (index, tree) in self.store.iter().enumerate() {
            writeln!(output, "# Hash {index:4}: {:6}", tree.nodes())?;
        }
        Ok(())
    }

    #[must_use]
    pub fn bucket_nodes(&self, index: usize) -> Option<i64> {
        self.store.get(index).map(TermTree::nodes)
    }
}

/// Computes the C term-cell-store hash for a term.
///
/// # Panics
///
/// Panics if a unary or n-ary term has uninitialized hash arguments.
#[must_use]
pub fn term_cell_hash(term: &Term) -> usize {
    let mut hash = f_code_hash_bits(term.f_code());
    let arguments = term.arguments();
    if !arguments.is_empty() {
        let arg = arguments[0]
            .as_ref()
            .expect("unary term hash requires arg 0");
        hash ^= term_identity_id(arg) >> 3;
    }
    if arguments.len() >= 2 {
        let arg = arguments[1]
            .as_ref()
            .expect("n-ary term hash requires arg 1");
        hash ^= term_identity_id(arg) >> 4;
    }
    hash & TERM_STORE_HASH_MASK
}

fn f_code_hash_bits(f_code: i64) -> usize {
    let modulus = i64::try_from(TERM_STORE_HASH_SIZE).expect("hash size fits in i64");
    usize::try_from(f_code.rem_euclid(modulus)).expect("positive remainder fits in usize")
}

#[cfg(test)]
mod tests {
    use super::{term_cell_hash, TermCellStore, TERM_STORE_HASH_MASK, TERM_STORE_HASH_SIZE};
    use crate::terms::termtypes::{
        Term, TP_CHECK_FLAG, TP_GARBAGE_FLAG, TP_IGNORE_PROPS, TP_TOP_POS,
    };
    use crate::terms::typebanks::TypeBank;

    fn typed_const(f_code: i64, type_: &crate::terms::simpletypes::Type) -> Term {
        let term = Term::const_cell_alloc(f_code);
        term.set_type(Some(type_.clone()));
        term
    }

    fn typed_unary(f_code: i64, arg: &Term, type_: &crate::terms::simpletypes::Type) -> Term {
        let term = Term::top_alloc(f_code, 1);
        term.set_type(Some(type_.clone()));
        term.set_argument(0, arg.clone());
        term
    }

    #[test]
    fn constants_and_hashing_match_c_shapes() {
        let types = TypeBank::new();
        let c = typed_const(-1, &types.i_type());
        assert_eq!(TERM_STORE_HASH_SIZE, 32_768);
        assert_eq!(TERM_STORE_HASH_MASK, 32_767);
        assert_eq!(term_cell_hash(&c), TERM_STORE_HASH_MASK);

        let arg = typed_const(2, &types.i_type());
        let unary = typed_unary(4, &arg, &types.i_type());
        assert_eq!(
            term_cell_hash(&unary),
            (4 ^ (crate::terms::termtypes::term_identity_id(&arg) >> 3)) & TERM_STORE_HASH_MASK
        );
    }

    #[test]
    #[should_panic(expected = "unary term hash requires arg 0")]
    fn hashing_rejects_uninitialized_argument_slots() {
        let types = TypeBank::new();
        let term = Term::top_alloc(4, 1);
        term.set_type(Some(types.i_type()));
        let _ = term_cell_hash(&term);
    }

    #[test]
    fn insert_find_extract_delete_and_accounting_match_store_contract() {
        let types = TypeBank::new();
        let mut store = TermCellStore::new();
        let one = typed_const(1, &types.i_type());
        let two = typed_unary(2, &one, &types.i_type());

        assert!(store.insert(one.clone()).is_none());
        assert!(store.insert(two.clone()).is_none());
        assert_eq!(store.entries(), 2);
        assert_eq!(store.arg_count(), 1);
        assert_eq!(store.count_nodes(), 2);
        assert_eq!(store.terms().len(), 2);
        assert_eq!(store.find(&one), Some(one.clone()));

        let duplicate = typed_const(1, &types.i_type());
        assert_eq!(store.insert(duplicate), Some(one.clone()));
        assert_eq!(store.entries(), 2);

        assert_eq!(store.extract(&two), Some(two.clone()));
        assert_eq!(store.entries(), 1);
        assert_eq!(store.arg_count(), 0);
        assert!(!store.delete(&two));
        assert!(store.delete(&one));
        assert_eq!(store.entries(), 0);
    }

    #[test]
    fn property_helpers_and_gc_sweep_visit_all_buckets() {
        let types = TypeBank::new();
        let mut store = TermCellStore::new();
        let one = typed_const(1, &types.i_type());
        let two = typed_const(2, &types.i_type());
        store.insert(one.clone());
        store.insert(two.clone());

        store.set_prop(TP_CHECK_FLAG | TP_TOP_POS);
        assert!(one.query_prop(TP_CHECK_FLAG | TP_TOP_POS));
        assert!(two.query_prop(TP_CHECK_FLAG | TP_TOP_POS));
        store.del_prop(TP_TOP_POS);
        assert!(!one.query_prop(TP_TOP_POS));

        one.set_prop(TP_GARBAGE_FLAG);
        let recovered = store.gc_sweep(TP_GARBAGE_FLAG);
        assert_eq!(recovered, 1);
        assert_eq!(store.entries(), 1);
        assert_eq!(store.find(&two), Some(two));
    }

    #[test]
    fn exit_clears_trees_and_distribution_prints_all_hashes() {
        let types = TypeBank::new();
        let mut store = TermCellStore::new();
        let term = typed_const(1, &types.i_type());
        let hash = term_cell_hash(&term);
        store.insert(term);
        assert_eq!(store.bucket_nodes(hash), Some(1));

        let mut output = Vec::new();
        store.print_distrib(&mut output).unwrap();
        let output = String::from_utf8(output).unwrap();
        assert!(output.contains("# Hash    1:      1"));
        assert_eq!(output.lines().count(), TERM_STORE_HASH_SIZE);

        store.exit();
        assert_eq!(store.entries(), 0);
        assert_eq!(store.arg_count(), 0);
        assert_eq!(store.count_nodes(), 0);
        assert_eq!(store.bucket_nodes(hash), Some(0));
    }

    #[test]
    fn gc_sweep_with_ignore_props_recovers_unmarked_terms() {
        let types = TypeBank::new();
        let mut store = TermCellStore::new();
        let term = typed_const(1, &types.i_type());
        store.insert(term);

        assert_eq!(store.gc_sweep(TP_IGNORE_PROPS), 1);
        assert_eq!(store.entries(), 0);
    }
}
