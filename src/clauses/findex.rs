use crate::clauses::clause::Clause;
use crate::clauses::clausepos_tree::clause_key;
use crate::terms::functypes::FunCode;
use std::collections::{btree_map::Entry, BTreeMap};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct FIndex {
    index: BTreeMap<FunCode, BTreeMap<usize, Clause>>,
}

impl FIndex {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            index: BTreeMap::new(),
        }
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.index.values().all(BTreeMap::is_empty)
    }

    #[must_use]
    pub fn bucket(&self, f_code: FunCode) -> Option<&BTreeMap<usize, Clause>> {
        self.index.get(&f_code)
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
}

#[cfg(test)]
mod tests {
    use super::FIndex;
    use crate::clauses::clause::Clause;
    use crate::clauses::clausepos_tree::clause_key;
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
}
