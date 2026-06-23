use crate::clauses::clause::Clause;
use crate::clauses::eqn::Eqn;
use crate::terms::functypes::FunCode;
use crate::terms::signature::Signature;
use crate::terms::termbanks::TermBank;
use crate::terms::termtypes::{term_identity_id, Term};
use std::collections::{btree_map::Entry, BTreeMap};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LitDesc {
    lit_key: usize,
    lit: Term,
    clause: Option<Clause>,
}

impl LitDesc {
    #[must_use]
    pub fn new(lit: &Term, clause: &Clause) -> Self {
        Self {
            lit_key: term_identity_id(lit),
            lit: lit.clone(),
            clause: Some(clause.clone()),
        }
    }

    #[must_use]
    pub const fn lit_key(&self) -> usize {
        self.lit_key
    }

    #[must_use]
    pub const fn literal(&self) -> &Term {
        &self.lit
    }

    #[must_use]
    pub const fn clause(&self) -> Option<&Clause> {
        self.clause.as_ref()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LitHash {
    sig_size: usize,
    pos_lits: Vec<BTreeMap<usize, LitDesc>>,
    neg_lits: Vec<BTreeMap<usize, LitDesc>>,
}

impl LitHash {
    /// Allocates a literal hash sized to the current signature.
    ///
    /// # Panics
    ///
    /// Panics if the signature function-symbol count cannot be represented as
    /// a Rust vector size. The C allocation uses `f_count + 1` array slots.
    #[must_use]
    pub fn new(sig: &Signature) -> Self {
        let sig_size =
            usize::try_from(sig.f_count() + 1).expect("signature size must fit in usize");
        Self {
            sig_size,
            pos_lits: vec![BTreeMap::new(); sig_size],
            neg_lits: vec![BTreeMap::new(); sig_size],
        }
    }

    #[must_use]
    pub const fn sig_size(&self) -> usize {
        self.sig_size
    }

    #[must_use]
    pub fn positive_bucket(&self, f_code: FunCode) -> Option<&BTreeMap<usize, LitDesc>> {
        self.bucket(f_code, true)
    }

    #[must_use]
    pub fn negative_bucket(&self, f_code: FunCode) -> Option<&BTreeMap<usize, LitDesc>> {
        self.bucket(f_code, false)
    }

    #[must_use]
    pub fn find(&self, lit: &Term, positive: bool) -> Option<&LitDesc> {
        self.bucket(lit.f_code(), positive)
            .and_then(|bucket| bucket.get(&term_identity_id(lit)))
    }

    /// Inserts a non-equational literal into the hash.
    ///
    /// Returns `true` when a new literal descriptor is stored. Returns `false`
    /// when the same literal term was already present in the same sign bucket;
    /// in that case the stored unique-clause payload is cleared, matching
    /// `LitHashInsertEqn`.
    ///
    /// # Panics
    ///
    /// Panics if `eqn` is equational, if its left-hand predicate code is not a
    /// positive signature symbol, or if the hash was allocated for a smaller
    /// signature than the literal uses.
    #[must_use]
    pub fn insert_eqn(&mut self, eqn: &Eqn, clause: &Clause, bank: &TermBank) -> bool {
        assert!(
            !eqn.is_equ_lit(bank),
            "literal hash accepts only non-equational literals"
        );
        let f_code = eqn.left().f_code();
        assert!(f_code > 0, "literal hash predicate code must be positive");
        let index = fun_code_index(f_code);
        assert!(
            index < self.sig_size,
            "literal hash was allocated for a smaller signature"
        );

        let root = if eqn.is_positive() {
            &mut self.pos_lits[index]
        } else {
            &mut self.neg_lits[index]
        };
        let key = term_identity_id(eqn.left());
        match root.entry(key) {
            Entry::Occupied(mut entry) => {
                entry.get_mut().clause = None;
                false
            }
            Entry::Vacant(entry) => {
                entry.insert(LitDesc::new(eqn.left(), clause));
                true
            }
        }
    }

    /// Inserts every literal from `clause` into the hash.
    ///
    /// Returns the number of newly stored literal descriptors.
    ///
    /// # Panics
    ///
    /// Panics under the same conditions as [`Self::insert_eqn`] for any
    /// literal in the clause. This preserves `LitHashInsertClause`, whose
    /// per-literal helper asserts that the literal is non-equational.
    #[must_use]
    pub fn insert_clause(&mut self, clause: &Clause, bank: &TermBank) -> usize {
        clause
            .literals()
            .as_slice()
            .iter()
            .filter(|literal| self.insert_eqn(literal, clause, bank))
            .count()
    }

    /// Inserts every literal from every clause yielded by `clauses`.
    ///
    /// Returns the number of newly stored literal descriptors.
    ///
    /// # Panics
    ///
    /// Panics under the same conditions as [`Self::insert_eqn`] for any
    /// literal in the clauses.
    #[must_use]
    pub fn insert_clauses<'a>(
        &mut self,
        clauses: impl IntoIterator<Item = &'a Clause>,
        bank: &TermBank,
    ) -> usize {
        clauses
            .into_iter()
            .map(|clause| self.insert_clause(clause, bank))
            .sum()
    }

    fn bucket(&self, f_code: FunCode, positive: bool) -> Option<&BTreeMap<usize, LitDesc>> {
        if f_code <= 0 {
            return None;
        }
        let index = usize::try_from(f_code).ok()?;
        if index >= self.sig_size {
            return None;
        }
        if positive {
            self.pos_lits.get(index)
        } else {
            self.neg_lits.get(index)
        }
    }
}

#[must_use]
pub fn lit_desc_compare(left: &LitDesc, right: &LitDesc) -> i32 {
    match left.lit_key.cmp(&right.lit_key) {
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Greater => 1,
    }
}

fn fun_code_index(f_code: FunCode) -> usize {
    usize::try_from(f_code).expect("positive function code fits in usize")
}

#[cfg(test)]
mod tests {
    use super::{lit_desc_compare, LitDesc, LitHash};
    use crate::clauses::clause::Clause;
    use crate::clauses::eqn::Eqn;
    use crate::clauses::eqnlist::EqnList;
    use crate::terms::signature::Signature;
    use crate::terms::simpletypes::alloc_arrow_type;
    use crate::terms::termbanks::TermBank;
    use crate::terms::termtypes::{term_identity_id, Term};
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

    fn typed_predicate(bank: &mut TermBank, name: &str, arg: Option<&Term>) -> Term {
        let bool_type = bank.signature().type_bank().bool_type();
        let f_code = bank
            .signature_mut()
            .insert_id(name, i32::from(arg.is_some()), false);
        if let Some(argument) = arg {
            let arg_type = argument.type_().expect("test argument must be typed");
            bank.signature_mut()
                .declare_final_type(f_code, alloc_arrow_type(vec![arg_type, bool_type.clone()]))
                .unwrap();
            let term = Term::top_alloc(f_code, 1);
            term.set_type(Some(bool_type));
            term.set_argument(0, argument.clone());
            bank.insert(&term, crate::terms::termtypes::DerefType::Never)
                .unwrap()
        } else {
            bank.signature_mut()
                .declare_final_type(f_code, bool_type)
                .unwrap();
            bank.create_const_term(f_code).unwrap()
        }
    }

    fn predicate_literal(bank: &mut TermBank, lit: &Term, positive: bool) -> Eqn {
        Eqn::alloc(lit.clone(), bank.true_term().clone(), bank, positive).unwrap()
    }

    fn unit_clause(bank: &mut TermBank, lit: &Term, positive: bool, ident: i64) -> Clause {
        let mut clause = Clause::alloc(EqnList::from_vec(vec![predicate_literal(
            bank, lit, positive,
        )]));
        clause.set_ident(ident);
        clause
    }

    #[test]
    fn descriptors_compare_by_literal_identity_only() {
        let mut bank = test_bank();
        let predicate = typed_predicate(&mut bank, "p", None);
        let same_shape = Term::const_cell_alloc(predicate.f_code());
        same_shape.set_type(predicate.type_());
        let clause = unit_clause(&mut bank, &predicate, true, 1);
        let desc = LitDesc::new(&predicate, &clause);
        let same_desc = LitDesc::new(&predicate, &clause);
        let other_desc = LitDesc::new(&same_shape, &clause);

        assert_eq!(desc.lit_key(), term_identity_id(&predicate));
        assert_eq!(desc.literal(), &predicate);
        assert_eq!(desc.clause().unwrap().ident(), 1);
        assert_eq!(lit_desc_compare(&desc, &same_desc), 0);
        assert_ne!(lit_desc_compare(&desc, &other_desc), 0);
    }

    #[test]
    fn insertion_separates_sign_buckets_and_records_unique_clause() {
        let mut bank = test_bank();
        let predicate = typed_predicate(&mut bank, "p", None);
        let positive_clause = unit_clause(&mut bank, &predicate, true, 10);
        let negative_clause = unit_clause(&mut bank, &predicate, false, 11);
        let mut hash = LitHash::new(bank.signature());

        assert_eq!(
            hash.sig_size(),
            usize::try_from(bank.signature().f_count() + 1).unwrap()
        );
        assert_eq!(hash.insert_clause(&positive_clause, &bank), 1);
        assert_eq!(hash.insert_clause(&negative_clause, &bank), 1);

        assert_eq!(
            hash.find(&predicate, true)
                .unwrap()
                .clause()
                .unwrap()
                .ident(),
            10
        );
        assert_eq!(
            hash.find(&predicate, false)
                .unwrap()
                .clause()
                .unwrap()
                .ident(),
            11
        );
        assert_eq!(hash.positive_bucket(predicate.f_code()).unwrap().len(), 1);
        assert_eq!(hash.negative_bucket(predicate.f_code()).unwrap().len(), 1);
    }

    #[test]
    fn duplicate_literal_terms_clear_unique_clause_payload() {
        let mut bank = test_bank();
        let predicate = typed_predicate(&mut bank, "p", None);
        let first = predicate_literal(&mut bank, &predicate, true);
        let second = predicate_literal(&mut bank, &predicate, true);
        let mut clause = Clause::alloc(EqnList::from_vec(vec![first, second]));
        clause.set_ident(20);
        let mut hash = LitHash::new(bank.signature());

        assert_eq!(hash.insert_clause(&clause, &bank), 1);

        assert!(hash.find(&predicate, true).unwrap().clause().is_none());
    }

    #[test]
    fn structurally_equal_but_distinct_terms_are_distinct_literals() {
        let mut bank = test_bank();
        let predicate = typed_predicate(&mut bank, "p", None);
        let same_shape = Term::const_cell_alloc(predicate.f_code());
        same_shape.set_type(predicate.type_());
        let first = unit_clause(&mut bank, &predicate, true, 30);
        let second = unit_clause(&mut bank, &same_shape, true, 31);
        let mut hash = LitHash::new(bank.signature());

        assert_eq!(hash.insert_clauses([&first, &second], &bank), 2);

        assert_eq!(hash.positive_bucket(predicate.f_code()).unwrap().len(), 2);
        assert_eq!(
            hash.find(&predicate, true)
                .unwrap()
                .clause()
                .unwrap()
                .ident(),
            30
        );
        assert_eq!(
            hash.find(&same_shape, true)
                .unwrap()
                .clause()
                .unwrap()
                .ident(),
            31
        );
    }

    #[test]
    #[should_panic(expected = "literal hash accepts only non-equational literals")]
    fn insertion_rejects_equational_literals() {
        let mut bank = test_bank();
        let left = typed_const(&mut bank, "a");
        let right = typed_const(&mut bank, "b");
        let eqn = Eqn::alloc(left, right, &mut bank, true).unwrap();
        let clause = Clause::alloc(EqnList::from_vec(vec![eqn.clone()]));
        let mut hash = LitHash::new(bank.signature());

        let _ = hash.insert_eqn(&eqn, &clause, &bank);
    }
}
