use crate::clauses::clause::Clause;
use crate::clauses::clausepos_tree::clause_key;
use crate::clauses::freqvectors::{
    fv_pack_clause, optimized_var_freq_vector_compute, FreqVector, FvCollect, FvCollectLayout,
    FvIndexType, FvPackedClause, PermVector, FVINDEX_MAX_FEATURES_DEFAULT,
    FVINDEX_SYMBOL_SLACK_DEFAULT,
};
use crate::clauses::subsumption::clause_subsume_order_sort_lits;
use crate::terms::termbanks::TermBank;
use std::collections::{btree_map::Entry, BTreeMap};
use std::fmt::Write as _;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FvIndexParams {
    cspec: FvCollect,
    use_perm_vectors: bool,
    eliminate_uninformative: bool,
    max_symbols: usize,
    symbol_slack: usize,
}

impl Default for FvIndexParams {
    fn default() -> Self {
        let mut cspec = FvCollect::new(FvCollectLayout::new(FvIndexType::AcFold, false, 0, 0));
        cspec.set_max_symbols(0);
        Self {
            cspec,
            use_perm_vectors: false,
            eliminate_uninformative: false,
            max_symbols: FVINDEX_MAX_FEATURES_DEFAULT,
            symbol_slack: FVINDEX_SYMBOL_SLACK_DEFAULT,
        }
    }
}

impl FvIndexParams {
    #[must_use]
    pub const fn cspec(&self) -> &FvCollect {
        &self.cspec
    }

    #[must_use]
    pub const fn use_perm_vectors(&self) -> bool {
        self.use_perm_vectors
    }

    #[must_use]
    pub const fn eliminate_uninformative(&self) -> bool {
        self.eliminate_uninformative
    }

    #[must_use]
    pub const fn max_symbols(&self) -> usize {
        self.max_symbols
    }

    #[must_use]
    pub const fn symbol_slack(&self) -> usize {
        self.symbol_slack
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct FvIndex {
    final_node: bool,
    clause_count: i64,
    successors: BTreeMap<i64, FvIndex>,
    clauses: BTreeMap<usize, Clause>,
}

impl FvIndex {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            final_node: false,
            clause_count: 0,
            successors: BTreeMap::new(),
            clauses: BTreeMap::new(),
        }
    }

    #[must_use]
    pub const fn is_final(&self) -> bool {
        self.final_node
    }

    #[must_use]
    pub const fn clause_count(&self) -> i64 {
        self.clause_count
    }

    #[must_use]
    pub const fn clauses(&self) -> &BTreeMap<usize, Clause> {
        &self.clauses
    }

    #[must_use]
    pub const fn successors(&self) -> &BTreeMap<i64, FvIndex> {
        &self.successors
    }

    /// Returns the successor for `key` only if its subtree is non-empty.
    ///
    /// # Panics
    ///
    /// Panics if called on a final node, matching the C assertion.
    #[must_use]
    pub fn get_next_non_empty_node(&self, key: i64) -> Option<&Self> {
        assert!(!self.final_node, "final FV-index nodes have no successors");
        self.successors
            .get(&key)
            .filter(|successor| successor.clause_count != 0)
    }

    /// Counts nodes using the C `FVIndexCountNodes` flags.
    ///
    /// # Panics
    ///
    /// Panics if a final node has a zero/nonzero clause count that disagrees
    /// with whether the leaf stores any clauses, matching the C `EQUIV`
    /// assertion.
    #[must_use]
    pub fn count_nodes(&self, leaves: bool, empty: bool) -> i64 {
        let mut result = 0;
        if self.final_node {
            if !empty || self.clauses.is_empty() {
                result += 1;
            }
            assert_eq!(
                self.clause_count != 0,
                !self.clauses.is_empty(),
                "final FV-index node count must match stored-clause presence"
            );
        } else {
            if !(empty || leaves) {
                result += 1;
            }
            for successor in self.successors.values() {
                result += successor.count_nodes(leaves, empty);
            }
        }
        result
    }

    #[must_use]
    pub fn debug_print_string(&self) -> String {
        let mut output = String::new();
        self.write_debug(&mut output, 0);
        output
    }

    fn insert_vector_clause(
        &mut self,
        vector: &FreqVector,
        clause_identity: usize,
        clause: Clause,
    ) -> bool {
        let mut node = self;
        node.clause_count += 1;
        for value in vector.as_slice() {
            assert!(
                !node.final_node,
                "final FV-index nodes cannot have successors"
            );
            node = match node.successors.entry(*value) {
                Entry::Occupied(entry) => entry.into_mut(),
                Entry::Vacant(entry) => entry.insert(Self::new()),
            };
            node.clause_count += 1;
        }
        node.final_node = true;
        node.clauses.insert(clause_identity, clause).is_none()
    }

    fn delete_vector_clause(&mut self, vector: &FreqVector, clause: &Clause) -> bool {
        let mut node = self;
        node.clause_count -= 1;
        for value in vector.as_slice() {
            assert!(
                !node.final_node,
                "final FV-index nodes cannot have successors"
            );
            let Some(next) = node.successors.get_mut(value) else {
                return false;
            };
            next.clause_count -= 1;
            node = next;
        }
        node.clauses.remove(&clause_key(clause)).is_some()
    }

    fn write_debug(&self, output: &mut String, level: usize) {
        if self.final_node {
            for clause in self.clauses.values() {
                for _ in 0..=level {
                    output.push_str("--");
                }
                let _ = writeln!(output, "clause#{}", clause.ident());
            }
        } else {
            for (key, successor) in &self.successors {
                for _ in 0..level {
                    output.push_str("--");
                }
                let _ = writeln!(output, "Alternative {key}: ");
                successor.write_debug(output, level + 1);
            }
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FvIndexAnchor {
    cspec: FvCollect,
    perm_vector: Option<PermVector>,
    index: FvIndex,
    storage: usize,
}

impl FvIndexAnchor {
    #[must_use]
    pub fn new(cspec: FvCollect, perm_vector: Option<PermVector>) -> Self {
        Self {
            cspec,
            perm_vector,
            index: FvIndex::new(),
            storage: 0,
        }
    }

    #[must_use]
    pub const fn cspec(&self) -> &FvCollect {
        &self.cspec
    }

    #[must_use]
    pub const fn perm_vector(&self) -> Option<&PermVector> {
        self.perm_vector.as_ref()
    }

    #[must_use]
    pub const fn index(&self) -> &FvIndex {
        &self.index
    }

    #[must_use]
    pub const fn storage_estimate(&self) -> usize {
        self.storage
    }

    /// Inserts a packed clause into the feature-vector index.
    ///
    /// # Panics
    ///
    /// Panics if `packed` does not contain a vector, or if an existing final
    /// node is encountered before all vector coordinates are consumed.
    pub fn insert(&mut self, packed: &mut FvPackedClause, bank: &TermBank) -> bool {
        let vector = packed
            .vector()
            .expect("FV-index insertion requires a packed frequency vector")
            .clone();
        clause_subsume_order_sort_lits(packed.clause_mut(), bank);
        let clause_identity = clause_key(packed.clause());
        let before_nodes = self.index.count_nodes(false, false);
        let inserted =
            self.index
                .insert_vector_clause(&vector, clause_identity, packed.clause().clone());
        let after_nodes = self.index.count_nodes(false, false);
        if after_nodes > before_nodes {
            self.storage += usize::try_from(after_nodes - before_nodes).unwrap_or(usize::MAX);
        }
        inserted
    }

    /// Deletes a clause from the feature-vector index.
    ///
    /// This preserves the C behavior of leaving empty leaves and interior nodes
    /// in place after deletion.
    ///
    /// # Panics
    ///
    /// Panics if an existing final node is encountered before all vector
    /// coordinates are consumed.
    pub fn delete(&mut self, clause: &Clause) -> bool {
        let vector =
            optimized_var_freq_vector_compute(clause, self.perm_vector.as_ref(), &self.cspec);
        self.index.delete_vector_clause(&vector, clause)
    }

    #[must_use]
    pub fn count_nodes(&self, leaves: bool, empty: bool) -> i64 {
        self.index.count_nodes(leaves, empty)
    }
}

#[must_use]
pub fn fv_index_storage(index: Option<&FvIndexAnchor>) -> usize {
    index.map_or(0, FvIndexAnchor::storage_estimate)
}

#[must_use]
pub fn fv_index_pack_clause(clause: Clause, anchor: Option<&FvIndexAnchor>) -> FvPackedClause {
    match anchor {
        Some(anchor) => fv_pack_clause(clause, anchor.perm_vector(), Some(anchor.cspec())),
        None => fv_pack_clause(clause, None, None),
    }
}

#[cfg(test)]
mod tests {
    use super::{fv_index_pack_clause, fv_index_storage, FvIndexAnchor, FvIndexParams};
    use crate::clauses::clause::Clause;
    use crate::clauses::eqn::Eqn;
    use crate::clauses::eqnlist::EqnList;
    use crate::clauses::freqvectors::{FvCollect, FvCollectLayout, FvIndexType};
    use crate::terms::signature::Signature;
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
        if bank.signature().get_type(f_code).is_none() {
            bank.signature_mut()
                .declare_final_type(f_code, type_.clone())
                .unwrap();
        }
        bank.create_const_term(f_code).unwrap()
    }

    fn typed_unary(bank: &mut TermBank, name: &str, arg: &Term) -> Term {
        let type_ = bank.signature().type_bank().default_type();
        let f_code = bank.signature_mut().insert_id(name, 1, false);
        if bank.signature().get_type(f_code).is_none() {
            bank.signature_mut()
                .declare_final_type(f_code, alloc_arrow_type(vec![type_.clone(), type_]))
                .unwrap();
        }
        let term = Term::top_alloc(f_code, 1);
        term.set_type(Some(bank.signature().type_bank().default_type()));
        term.set_argument(0, arg.clone());
        bank.insert(&term, DerefType::Never).unwrap()
    }

    fn literal(bank: &mut TermBank, left: &Term, right: &Term, positive: bool) -> Eqn {
        Eqn::alloc(left.clone(), right.clone(), bank, positive).unwrap()
    }

    fn clause_from(literals: Vec<Eqn>, ident: i64) -> Clause {
        let mut clause = Clause::alloc(EqnList::from_vec(literals));
        clause.set_ident(ident);
        clause.set_weight(clause.standard_weight());
        clause
    }

    fn ac_anchor(max_symbols: usize) -> FvIndexAnchor {
        let mut cspec = FvCollect::new(FvCollectLayout::new(FvIndexType::AcFeatures, false, 0, 0));
        cspec.set_max_symbols(max_symbols);
        FvIndexAnchor::new(cspec, None)
    }

    #[test]
    fn default_parameters_match_c_shape() {
        let params = FvIndexParams::default();
        assert_eq!(params.cspec().features(), FvIndexType::AcFold);
        assert!(!params.use_perm_vectors());
        assert!(!params.eliminate_uninformative());
        assert_eq!(params.max_symbols(), 17);
        assert_eq!(params.symbol_slack(), 0);
        assert_eq!(params.cspec().max_symbols(), 0);
    }

    #[test]
    fn pack_clause_uses_anchor_vector_when_available() {
        let mut bank = test_bank();
        let first = typed_const(&mut bank, "a");
        let second = typed_const(&mut bank, "b");
        let clause = clause_from(vec![literal(&mut bank, &first, &second, true)], 10);
        let anchor = ac_anchor(usize::try_from(second.f_code()).unwrap() + 1);

        let dummy = fv_index_pack_clause(clause.clone(), None);
        assert!(dummy.vector().is_none());

        let packed = fv_index_pack_clause(clause, Some(&anchor));
        assert!(packed.vector().is_some());
    }

    #[test]
    fn insert_tracks_counts_and_non_empty_successors() {
        let mut bank = test_bank();
        let first = typed_const(&mut bank, "a");
        let second = typed_const(&mut bank, "b");
        let f_of_first = typed_unary(&mut bank, "f", &first);
        let clause = clause_from(vec![literal(&mut bank, &f_of_first, &second, true)], 20);
        let mut anchor = ac_anchor(usize::try_from(f_of_first.f_code()).unwrap() + 1);
        let mut packed = fv_index_pack_clause(clause, Some(&anchor));
        let first_value = packed.vector().unwrap().as_slice()[0];
        let vector_len = i64::try_from(packed.vector().unwrap().len()).unwrap();

        assert!(anchor.insert(&mut packed, &bank));
        assert_eq!(
            fv_index_storage(Some(&anchor)),
            usize::try_from(vector_len).unwrap()
        );
        assert_eq!(fv_index_storage(None), 0);
        assert_eq!(anchor.index().clause_count(), 1);
        assert_eq!(anchor.count_nodes(true, false), 1);
        assert_eq!(anchor.count_nodes(false, false), vector_len + 1);
        assert!(anchor
            .index()
            .get_next_non_empty_node(first_value)
            .is_some());
    }

    #[test]
    fn delete_leaves_empty_leaf_and_suppresses_non_empty_lookup() {
        let mut bank = test_bank();
        let first = typed_const(&mut bank, "a");
        let second = typed_const(&mut bank, "b");
        let clause = clause_from(vec![literal(&mut bank, &first, &second, true)], 30);
        let mut anchor = ac_anchor(usize::try_from(second.f_code()).unwrap() + 1);
        let mut packed = fv_index_pack_clause(clause, Some(&anchor));
        let first_value = packed.vector().unwrap().as_slice()[0];

        assert!(anchor.insert(&mut packed, &bank));
        assert!(anchor.delete(packed.clause()));
        assert_eq!(anchor.index().clause_count(), 0);
        assert_eq!(anchor.count_nodes(true, true), 1);
        assert!(anchor
            .index()
            .get_next_non_empty_node(first_value)
            .is_none());
    }

    #[test]
    fn deleting_missing_clause_preserves_c_count_mutation_shape() {
        let mut bank = test_bank();
        let first = typed_const(&mut bank, "a");
        let second = typed_const(&mut bank, "b");
        let indexed = clause_from(vec![literal(&mut bank, &first, &second, true)], 40);
        let missing = clause_from(vec![literal(&mut bank, &first, &second, true)], 41);
        let mut anchor = ac_anchor(usize::try_from(second.f_code()).unwrap() + 1);
        let mut packed = fv_index_pack_clause(indexed, Some(&anchor));
        let first_value = packed.vector().unwrap().as_slice()[0];

        assert!(anchor.insert(&mut packed, &bank));
        assert!(!anchor.delete(&missing));
        assert_eq!(anchor.index().clause_count(), 0);
        assert!(anchor
            .index()
            .get_next_non_empty_node(first_value)
            .is_none());
    }
}
