use crate::clauses::clause::Clause;
use crate::clauses::subterm_tree::SubtermOcc;
use crate::terms::fp_index::{FPIndex, FPTree};
use crate::terms::idx_fp::FingerprintIndexFunction;
use crate::terms::signature::Signature;
use crate::terms::termfunc::term_is_db_closed;
use crate::terms::termtypes::{term_identity_id, Term};
use std::collections::{btree_map::Entry, BTreeMap};

pub type TermIdentitySet = BTreeMap<usize, Term>;

pub struct SubtermIndex<'sig> {
    index: FPIndex<'sig, SubtermOcc>,
}

impl<'sig> SubtermIndex<'sig> {
    #[must_use]
    pub fn new(fp_fun: FingerprintIndexFunction, sig: &'sig Signature) -> Self {
        Self {
            index: FPIndex::new(fp_fun, sig),
        }
    }

    #[must_use]
    pub const fn root(&self) -> &FPTree<SubtermOcc> {
        self.index.root()
    }

    pub fn root_mut(&mut self) -> &mut FPTree<SubtermOcc> {
        self.index.root_mut()
    }

    #[must_use]
    pub fn find_leaf(&self, term: &Term) -> Option<&FPTree<SubtermOcc>> {
        self.index.find(term)
    }

    #[must_use]
    pub fn find_occurrence(&self, term: &Term) -> Option<&SubtermOcc> {
        self.index
            .find(term)
            .and_then(FPTree::payload)
            .and_then(|payload| payload.find(&SubtermOcc::new(term)))
    }

    #[must_use]
    pub fn collect_leaves<'idx>(&'idx self, result: &mut Vec<&'idx FPTree<SubtermOcc>>) -> usize {
        self.index.collect_leaves(result)
    }

    pub fn collect_matchable_occurrences<'idx>(
        &'idx self,
        term: &Term,
        result: &mut Vec<&'idx SubtermOcc>,
    ) -> usize {
        let start = result.len();
        let mut payloads = Vec::new();
        self.index.find_matchable(term, &mut payloads);
        for payload in payloads.into_iter().rev().flatten() {
            result.extend(payload.iter());
        }
        result.len() - start
    }

    pub fn collect_unifiable_occurrences<'idx>(
        &'idx self,
        term: &Term,
        result: &mut Vec<&'idx SubtermOcc>,
    ) -> usize {
        let start = result.len();
        let mut payloads = Vec::new();
        self.index.find_unifiable(term, &mut payloads);
        for payload in payloads.into_iter().rev().flatten() {
            result.extend(payload.iter());
        }
        result.len() - start
    }

    pub fn insert_occurrence(&mut self, clause: &Clause, term: &Term, restricted: bool) -> bool {
        let leaf = self.index.insert(term);
        let payload = leaf.ensure_payload();
        let mut occurrence = payload
            .extract_object(&SubtermOcc::new(term))
            .unwrap_or_else(|| SubtermOcc::new(term));
        let inserted = occurrence.insert_occurrence(clause, restricted);
        let duplicate = payload.store(occurrence);
        debug_assert!(duplicate.is_none());
        inserted
    }

    pub fn delete_occurrence(&mut self, clause: &Clause, term: &Term, restricted: bool) -> bool {
        let (deleted, payload_empty) = {
            let Some(leaf) = self.index.find_mut(term) else {
                return false;
            };
            let Some(payload) = leaf.payload_mut() else {
                return false;
            };
            let Some(mut occurrence) = payload.extract_object(&SubtermOcc::new(term)) else {
                return false;
            };
            let deleted = occurrence.delete_occurrence(clause, restricted);
            if !occurrence.is_unused() {
                let duplicate = payload.store(occurrence);
                debug_assert!(duplicate.is_none());
            }
            let payload_empty = payload.is_empty();
            if payload_empty {
                leaf.clear_payload();
            }
            (deleted, payload_empty)
        };
        if payload_empty {
            self.index.delete(term);
        }
        deleted
    }

    pub fn insert_clause(&mut self, clause: &Clause, lambda_demod: bool) {
        let mut rest = TermIdentitySet::new();
        let mut full = TermIdentitySet::new();
        let _ = clause_collect_idx_subterms(clause, &mut rest, &mut full, lambda_demod);
        self.insert_set(clause, &rest, true);
        self.insert_set(clause, &full, false);
    }

    pub fn delete_clause(&mut self, clause: &Clause, lambda_demod: bool) {
        let mut rest = TermIdentitySet::new();
        let mut full = TermIdentitySet::new();
        let _ = clause_collect_idx_subterms(clause, &mut rest, &mut full, lambda_demod);
        self.delete_set(clause, &rest, true);
        self.delete_set(clause, &full, false);
    }

    fn insert_set(&mut self, clause: &Clause, terms: &TermIdentitySet, restricted: bool) {
        for term in terms.values() {
            self.insert_occurrence(clause, term, restricted);
        }
    }

    fn delete_set(&mut self, clause: &Clause, terms: &TermIdentitySet, restricted: bool) {
        for term in terms.values() {
            self.delete_occurrence(clause, term, restricted);
        }
    }
}

#[must_use]
pub fn clause_collect_idx_subterms(
    clause: &Clause,
    rest: &mut TermIdentitySet,
    full: &mut TermIdentitySet,
    lambda_demod: bool,
) -> i64 {
    clause
        .literals()
        .as_slice()
        .iter()
        .map(|literal| {
            let restricted_rw =
                literal.is_maximal() && literal.is_positive() && literal.is_oriented();
            term_collect_idx_subterms(literal.left(), rest, full, restricted_rw, lambda_demod)
                + term_collect_idx_subterms(literal.right(), rest, full, false, lambda_demod)
        })
        .sum()
}

fn term_collect_idx_subterms(
    term: &Term,
    rest: &mut TermIdentitySet,
    full: &mut TermIdentitySet,
    restricted: bool,
    lambda_demod: bool,
) -> i64 {
    if term.is_free_var() {
        return 0;
    }

    let mut result = 0;
    if (!lambda_demod || term_is_db_closed(term))
        && store_term(target_set(rest, full, restricted), term)
    {
        result += 1;
    }
    if !lambda_demod && !term.is_lambda() {
        for arg in term.argument_clones().into_iter().flatten() {
            result += term_collect_idx_subterms(&arg, rest, full, false, true);
        }
    }
    result
}

fn target_set<'sets>(
    rest: &'sets mut TermIdentitySet,
    full: &'sets mut TermIdentitySet,
    restricted: bool,
) -> &'sets mut TermIdentitySet {
    if restricted {
        rest
    } else {
        full
    }
}

fn store_term(target: &mut TermIdentitySet, term: &Term) -> bool {
    match target.entry(term_identity_id(term)) {
        Entry::Occupied(_) => false,
        Entry::Vacant(entry) => {
            entry.insert(term.clone());
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{clause_collect_idx_subterms, SubtermIndex, TermIdentitySet};
    use crate::clauses::clause::Clause;
    use crate::clauses::eqn::Eqn;
    use crate::clauses::eqn_props::{EP_IS_MAXIMAL, EP_IS_ORIENTED};
    use crate::clauses::eqnlist::EqnList;
    use crate::terms::idx_fp::index_fp1_create;
    use crate::terms::signature::{Signature, SIG_DB_LAMBDA_CODE};
    use crate::terms::simpletypes::alloc_arrow_type;
    use crate::terms::termbanks::TermBank;
    use crate::terms::termtypes::{
        term_identity_id, DerefType, Term, TP_HAS_DB_SUBTERM, TP_IS_DB_VAR,
    };
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
        let term = Term::const_cell_alloc(f_code);
        term.set_type(Some(type_));
        bank.insert(&term, DerefType::Never).unwrap()
    }

    fn typed_unary(bank: &mut TermBank, name: &str, arg: &Term) -> Term {
        let type_ = bank.signature().type_bank().default_type();
        let f_code = bank.signature_mut().insert_id(name, 1, false);
        if bank.signature().get_type(f_code).is_none() {
            bank.signature_mut()
                .declare_final_type(f_code, alloc_arrow_type(vec![type_.clone(), type_.clone()]))
                .unwrap();
        }
        let term = Term::top_alloc(f_code, 1);
        term.set_type(Some(type_));
        term.set_argument(0, arg.clone());
        bank.insert(&term, DerefType::Never).unwrap()
    }

    fn eqn(bank: &mut TermBank, left: &Term, right: &Term, positive: bool) -> Eqn {
        Eqn::alloc(left.clone(), right.clone(), bank, positive).unwrap()
    }

    fn singleton_clause(literal: Eqn, ident: i64) -> Clause {
        let mut clause = Clause::alloc(EqnList::from_vec(vec![literal]));
        clause.set_ident(ident);
        clause
    }

    fn contains_term(terms: &TermIdentitySet, term: &Term) -> bool {
        terms.contains_key(&term_identity_id(term))
    }

    fn full_clause_idents(occurrences: &[&crate::clauses::subterm_tree::SubtermOcc]) -> Vec<i64> {
        occurrences
            .iter()
            .flat_map(|occurrence| occurrence.full_clauses().values())
            .map(Clause::ident)
            .collect()
    }

    #[test]
    fn direct_occurrence_insert_delete_updates_fingerprint_payload() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let left = typed_unary(&mut bank, "f", &a);
        let clause = singleton_clause(eqn(&mut bank, &left, &b, true), 1);
        let mut index = SubtermIndex::new(index_fp1_create, bank.signature());

        assert!(index.insert_occurrence(&clause, &left, true));
        assert!(!index.insert_occurrence(&clause, &left, true));
        let occurrence = index.find_occurrence(&left).unwrap();
        assert_eq!(occurrence.restricted_clauses().len(), 1);
        assert!(occurrence.full_clauses().is_empty());

        assert!(index.delete_occurrence(&clause, &left, true));
        assert!(index.find_occurrence(&left).is_none());
        assert!(index.find_leaf(&left).is_none());
        assert!(!index.delete_occurrence(&clause, &left, true));
    }

    #[test]
    fn clause_collection_splits_restricted_left_sides_from_full_subterms() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let left = typed_unary(&mut bank, "f", &a);
        let right = typed_unary(&mut bank, "g", &b);
        let mut literal = eqn(&mut bank, &left, &right, true);
        literal.set_prop(EP_IS_MAXIMAL | EP_IS_ORIENTED);
        let clause = singleton_clause(literal, 1);
        let mut rest = TermIdentitySet::new();
        let mut full = TermIdentitySet::new();

        assert_eq!(
            clause_collect_idx_subterms(&clause, &mut rest, &mut full, false),
            4
        );

        assert!(contains_term(&rest, &left));
        assert_eq!(rest.len(), 1);
        assert!(contains_term(&full, &a));
        assert!(contains_term(&full, &right));
        assert!(contains_term(&full, &b));
        assert!(!contains_term(&full, &left));
    }

    #[test]
    fn lambda_demod_collection_filters_open_db_terms_and_stops_recursion() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let f_of_a = typed_unary(&mut bank, "f", &a);
        let type_ = bank.signature().type_bank().default_type();
        let open_db = Term::const_cell_alloc(0);
        open_db.set_type(Some(type_.clone()));
        open_db.set_prop(TP_IS_DB_VAR | TP_HAS_DB_SUBTERM);
        let clause = singleton_clause(eqn(&mut bank, &f_of_a, &open_db, true), 1);
        let mut rest = TermIdentitySet::new();
        let mut full = TermIdentitySet::new();

        assert_eq!(
            clause_collect_idx_subterms(&clause, &mut rest, &mut full, true),
            1
        );
        assert!(contains_term(&full, &f_of_a));
        assert!(!contains_term(&full, &a));
        assert!(!contains_term(&full, &open_db));

        let lambda = Term::top_alloc(SIG_DB_LAMBDA_CODE, 2);
        lambda.set_type(Some(type_));
        lambda.set_prop(TP_HAS_DB_SUBTERM);
        lambda.set_argument(0, typed_const(&mut bank, "binder"));
        lambda.set_argument(1, open_db);
        let lambda_clause = singleton_clause(eqn(&mut bank, &lambda, &f_of_a, true), 2);
        rest.clear();
        full.clear();

        assert_eq!(
            clause_collect_idx_subterms(&lambda_clause, &mut rest, &mut full, true),
            2
        );
        assert!(contains_term(&full, &lambda));
        assert!(contains_term(&full, &f_of_a));
    }

    #[test]
    fn clause_insert_delete_uses_collected_restricted_and_full_sets() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let left = typed_unary(&mut bank, "f", &a);
        let right = typed_unary(&mut bank, "g", &b);
        let mut literal = eqn(&mut bank, &left, &right, true);
        literal.set_prop(EP_IS_MAXIMAL | EP_IS_ORIENTED);
        let clause = singleton_clause(literal, 9);
        let mut index = SubtermIndex::new(index_fp1_create, bank.signature());

        index.insert_clause(&clause, false);
        assert_eq!(
            index
                .find_occurrence(&left)
                .unwrap()
                .restricted_clauses()
                .len(),
            1
        );
        assert_eq!(
            index.find_occurrence(&right).unwrap().full_clauses().len(),
            1
        );
        index.delete_clause(&clause, false);
        assert!(index.find_occurrence(&left).is_none());
        assert!(index.find_occurrence(&right).is_none());
    }

    #[test]
    fn matchable_occurrence_query_flattens_fingerprint_payloads() {
        let mut bank = test_bank();
        let x = Term::const_cell_alloc(-2);
        x.set_type(Some(bank.signature().type_bank().default_type()));
        let a = typed_const(&mut bank, "matchable_a");
        let b = typed_const(&mut bank, "matchable_b");
        let f_x = typed_unary(&mut bank, "matchable_f", &x);
        let f_a = typed_unary(&mut bank, "matchable_f", &a);
        let f_b = typed_unary(&mut bank, "matchable_f", &b);
        let first = singleton_clause(eqn(&mut bank, &f_a, &a, true), 21);
        let second = singleton_clause(eqn(&mut bank, &f_b, &b, true), 22);
        let mut index = SubtermIndex::new(index_fp1_create, bank.signature());
        index.insert_occurrence(&first, &f_a, false);
        index.insert_occurrence(&second, &f_b, false);
        let mut occurrences = Vec::new();

        assert_eq!(
            index.collect_matchable_occurrences(&f_x, &mut occurrences),
            2
        );
        let mut identifiers = occurrences
            .iter()
            .flat_map(|occurrence| occurrence.full_clauses().values())
            .map(Clause::ident)
            .collect::<Vec<_>>();
        identifiers.sort_unstable();
        assert_eq!(identifiers, vec![21, 22]);
    }

    #[test]
    fn occurrence_queries_flatten_fingerprint_candidates_in_c_stack_pop_order() {
        let mut bank = test_bank();
        let x = Term::const_cell_alloc(-3);
        x.set_type(Some(bank.signature().type_bank().default_type()));
        let a = typed_const(&mut bank, "stack_order_a");
        let f_a = typed_unary(&mut bank, "stack_order_f", &a);
        let g_a = typed_unary(&mut bank, "stack_order_g", &a);
        let first = singleton_clause(eqn(&mut bank, &x, &a, true), 31);
        let second = singleton_clause(eqn(&mut bank, &f_a, &a, true), 32);
        let third = singleton_clause(eqn(&mut bank, &g_a, &a, true), 33);
        let mut index = SubtermIndex::new(index_fp1_create, bank.signature());
        index.insert_occurrence(&first, &x, false);
        index.insert_occurrence(&second, &f_a, false);
        index.insert_occurrence(&third, &g_a, false);

        let mut matchable = Vec::new();
        assert_eq!(index.collect_matchable_occurrences(&x, &mut matchable), 3);
        assert_eq!(full_clause_idents(&matchable), vec![33, 32, 31]);

        let mut unifiable = Vec::new();
        assert_eq!(index.collect_unifiable_occurrences(&x, &mut unifiable), 3);
        assert_eq!(full_clause_idents(&unifiable), vec![33, 32, 31]);
    }
}
