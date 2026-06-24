use crate::basics::plist::{PListArena, PListHandle};
use crate::clauses::clause::Clause;
use crate::clauses::clausepos_tree::clause_key;
use crate::clauses::clausesets::ClauseSet;
use crate::terms::functypes::FunCode;
use std::collections::{btree_map::Entry, BTreeMap};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct FIndex {
    index: BTreeMap<FunCode, BTreeMap<usize, Clause>>,
    plist_clause_index: BTreeMap<FunCode, BTreeMap<usize, PListHandle>>,
}

impl FIndex {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            index: BTreeMap::new(),
            plist_clause_index: BTreeMap::new(),
        }
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.index.values().all(BTreeMap::is_empty)
            && self.plist_clause_index.values().all(BTreeMap::is_empty)
    }

    #[must_use]
    pub fn bucket(&self, f_code: FunCode) -> Option<&BTreeMap<usize, Clause>> {
        self.index.get(&f_code)
    }

    #[must_use]
    pub fn plist_clause_bucket(&self, f_code: FunCode) -> Option<&BTreeMap<usize, PListHandle>> {
        self.plist_clause_index.get(&f_code)
    }

    /// Adds `clause` under every function symbol returned by
    /// `ClauseReturnFCodes`.
    ///
    /// Returns the number of new function-code/instance associations.
    #[must_use]
    pub fn add_clause(&mut self, clause: &Clause) -> usize {
        let mut f_codes = Vec::new();
        clause.return_fcodes(&mut f_codes);
        f_codes
            .into_iter()
            .filter(|&f_code| self.add_instance(f_code, clause))
            .count()
    }

    /// Removes `clause` from every function symbol returned by
    /// `ClauseReturnFCodes`.
    ///
    /// Returns the number of removed function-code/instance associations.
    #[must_use]
    pub fn remove_clause(&mut self, clause: &Clause) -> usize {
        let mut f_codes = Vec::new();
        clause.return_fcodes(&mut f_codes);
        f_codes
            .into_iter()
            .filter(|&f_code| self.remove_instance(f_code, clause))
            .count()
    }

    /// Adds every clause yielded by `clauses`.
    ///
    /// Returns the number of new function-code/instance associations.
    #[must_use]
    pub fn add_clauses<'a>(&mut self, clauses: impl IntoIterator<Item = &'a Clause>) -> usize {
        clauses
            .into_iter()
            .map(|clause| self.add_clause(clause))
            .sum()
    }

    /// Adds every clause from a plain clause set.
    ///
    /// Returns the number of new function-code/instance associations.
    #[must_use]
    pub fn add_clause_set(&mut self, clauses: &ClauseSet) -> usize {
        self.add_clauses(clauses.iter())
    }

    /// Adds a `PList` cell containing a clause under every function symbol
    /// returned by `ClauseReturnFCodes`.
    ///
    /// Returns the number of new function-code/list-cell associations.
    #[must_use]
    pub fn add_pl_clause(&mut self, clauses: &PListArena<Clause>, lclause: PListHandle) -> usize {
        let Some(clause) = clauses.value(lclause) else {
            return 0;
        };
        let mut f_codes = Vec::new();
        clause.return_fcodes(&mut f_codes);
        f_codes
            .into_iter()
            .filter(|&f_code| self.add_pl_clause_instance(f_code, lclause))
            .count()
    }

    /// Removes a `PList` cell containing a clause from every function symbol
    /// returned by `ClauseReturnFCodes`.
    ///
    /// Returns the number of removed function-code/list-cell associations.
    #[must_use]
    pub fn remove_pl_clause(
        &mut self,
        clauses: &PListArena<Clause>,
        lclause: PListHandle,
    ) -> usize {
        let Some(clause) = clauses.value(lclause) else {
            return 0;
        };
        let mut f_codes = Vec::new();
        clause.return_fcodes(&mut f_codes);
        f_codes
            .into_iter()
            .filter(|&f_code| self.remove_pl_clause_instance(f_code, lclause))
            .count()
    }

    /// Adds every clause cell from a `PList` clause set.
    ///
    /// Returns the number of new function-code/list-cell associations.
    #[must_use]
    pub fn add_pl_clause_set(&mut self, clauses: &PListArena<Clause>, set: PListHandle) -> usize {
        clauses
            .handles(set)
            .into_iter()
            .map(|lclause| self.add_pl_clause(clauses, lclause))
            .sum()
    }

    fn add_instance(&mut self, f_code: FunCode, clause: &Clause) -> bool {
        match self
            .index
            .entry(f_code)
            .or_default()
            .entry(clause_key(clause))
        {
            Entry::Occupied(_) => false,
            Entry::Vacant(entry) => {
                entry.insert(clause.clone());
                true
            }
        }
    }

    fn remove_instance(&mut self, f_code: FunCode, clause: &Clause) -> bool {
        let Some(bucket) = self.index.get_mut(&f_code) else {
            return false;
        };
        let removed = bucket.remove(&clause_key(clause)).is_some();
        if bucket.is_empty() {
            self.index.remove(&f_code);
        }
        removed
    }

    fn add_pl_clause_instance(&mut self, f_code: FunCode, lclause: PListHandle) -> bool {
        match self
            .plist_clause_index
            .entry(f_code)
            .or_default()
            .entry(lclause.index())
        {
            Entry::Occupied(_) => false,
            Entry::Vacant(entry) => {
                entry.insert(lclause);
                true
            }
        }
    }

    fn remove_pl_clause_instance(&mut self, f_code: FunCode, lclause: PListHandle) -> bool {
        let Some(bucket) = self.plist_clause_index.get_mut(&f_code) else {
            return false;
        };
        let removed = bucket.remove(&lclause.index()).is_some();
        if bucket.is_empty() {
            self.plist_clause_index.remove(&f_code);
        }
        removed
    }
}

#[cfg(test)]
mod tests {
    use super::FIndex;
    use crate::basics::plist::PListArena;
    use crate::clauses::clause::Clause;
    use crate::clauses::clausepos_tree::clause_key;
    use crate::clauses::clausesets::ClauseSet;
    use crate::clauses::eqn::Eqn;
    use crate::clauses::eqnlist::EqnList;
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
        bank.signature_mut()
            .declare_final_type(f_code, type_)
            .unwrap();
        bank.create_const_term(f_code).unwrap()
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

    fn unit_clause(bank: &mut TermBank, left: &Term, right: &Term, ident: i64) -> Clause {
        let literal = Eqn::alloc(left.clone(), right.clone(), bank, true).unwrap();
        let mut clause = Clause::alloc(EqnList::from_vec(vec![literal]));
        clause.set_ident(ident);
        clause
    }

    #[test]
    fn add_clause_indexes_each_unique_function_code_once() {
        let mut bank = test_bank();
        let argument = typed_const(&mut bank, "a");
        let left = typed_unary(&mut bank, "f", &argument);
        let right = typed_unary(&mut bank, "g", &argument);
        let clause = Box::new(unit_clause(&mut bank, &left, &right, 10));
        let mut index = FIndex::new();

        assert_eq!(index.add_clause(&clause), 3);
        assert_eq!(index.add_clause(&clause), 0);

        for term in [&argument, &left, &right] {
            let bucket = index.bucket(term.f_code()).unwrap();
            assert_eq!(bucket.len(), 1);
            assert_eq!(bucket.get(&clause_key(&clause)).unwrap().ident(), 10);
        }
    }

    #[test]
    fn remove_clause_deletes_all_function_code_associations() {
        let mut bank = test_bank();
        let argument = typed_const(&mut bank, "a");
        let left = typed_unary(&mut bank, "f", &argument);
        let right = typed_const(&mut bank, "b");
        let clause = Box::new(unit_clause(&mut bank, &left, &right, 11));
        let mut index = FIndex::new();

        assert_eq!(index.add_clause(&clause), 3);
        assert_eq!(index.remove_clause(&clause), 3);

        assert!(index.is_empty());
        assert_eq!(index.remove_clause(&clause), 0);
    }

    #[test]
    fn add_clauses_keeps_distinct_clause_identities_in_shared_buckets() {
        let mut bank = test_bank();
        let shared = typed_const(&mut bank, "a");
        let left_clause = Box::new(unit_clause(&mut bank, &shared, &shared, 20));
        let right_clause = Box::new(unit_clause(&mut bank, &shared, &shared, 21));
        let mut index = FIndex::new();

        assert_eq!(index.add_clauses([&*left_clause, &*right_clause]), 2);

        let bucket = index.bucket(shared.f_code()).unwrap();
        assert_eq!(bucket.len(), 2);
        assert_eq!(bucket.get(&clause_key(&left_clause)).unwrap().ident(), 20);
        assert_eq!(bucket.get(&clause_key(&right_clause)).unwrap().ident(), 21);
    }

    #[test]
    fn add_clause_set_uses_plain_clause_set_iteration() {
        let mut bank = test_bank();
        let shared = typed_const(&mut bank, "a");
        let set = ClauseSet::from_clauses([
            unit_clause(&mut bank, &shared, &shared, 30),
            unit_clause(&mut bank, &shared, &shared, 31),
        ]);
        let mut index = FIndex::new();

        assert_eq!(index.add_clause_set(&set), 2);
        assert_eq!(index.bucket(shared.f_code()).unwrap().len(), 2);
    }

    #[test]
    fn plist_clause_cells_are_indexed_by_cell_identity() {
        let mut bank = test_bank();
        let shared = typed_const(&mut bank, "a");
        let first_clause = unit_clause(&mut bank, &shared, &shared, 40);
        let second_clause = unit_clause(&mut bank, &shared, &shared, 41);
        let mut clauses = PListArena::new();
        let anchor = clauses.alloc_list();
        let first = clauses.store_after(anchor, first_clause).unwrap();
        let second = clauses.store_after(first, second_clause).unwrap();
        let mut index = FIndex::new();

        assert_eq!(index.add_pl_clause(&clauses, first), 1);
        assert_eq!(index.add_pl_clause(&clauses, first), 0);
        assert_eq!(index.add_pl_clause(&clauses, second), 1);

        let bucket = index.plist_clause_bucket(shared.f_code()).unwrap();
        assert_eq!(bucket.len(), 2);
        assert_eq!(bucket.get(&first.index()), Some(&first));
        assert_eq!(bucket.get(&second.index()), Some(&second));

        assert_eq!(index.remove_pl_clause(&clauses, first), 1);
        assert_eq!(index.remove_pl_clause(&clauses, first), 0);
        assert_eq!(index.plist_clause_bucket(shared.f_code()).unwrap().len(), 1);
    }

    #[test]
    fn plist_clause_set_addition_walks_list_cells() {
        let mut bank = test_bank();
        let shared = typed_const(&mut bank, "a");
        let first_clause = unit_clause(&mut bank, &shared, &shared, 50);
        let second_clause = unit_clause(&mut bank, &shared, &shared, 51);
        let mut clauses = PListArena::new();
        let anchor = clauses.alloc_list();
        let first = clauses.store_after(anchor, first_clause).unwrap();
        let second = clauses.store_after(first, second_clause).unwrap();
        let mut index = FIndex::new();

        assert_eq!(index.add_pl_clause_set(&clauses, anchor), 2);

        let bucket = index.plist_clause_bucket(shared.f_code()).unwrap();
        assert_eq!(bucket.len(), 2);
        assert!(bucket.contains_key(&first.index()));
        assert!(bucket.contains_key(&second.index()));
    }
}
