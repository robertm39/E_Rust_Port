use crate::terms::termtypes::{term_identity_id, Term, TP_IS_GROUND};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VarSet {
    term: Option<Term>,
    valid: bool,
    vars: BTreeMap<usize, Term>,
}

impl VarSet {
    #[must_use]
    pub fn new(term: Option<Term>) -> Self {
        Self {
            term,
            valid: false,
            vars: BTreeMap::new(),
        }
    }

    #[must_use]
    pub fn term(&self) -> Option<Term> {
        self.term.clone()
    }

    #[must_use]
    pub const fn is_valid(&self) -> bool {
        self.valid
    }

    pub fn set_valid(&mut self, valid: bool) {
        self.valid = valid;
    }

    pub fn reset(&mut self) {
        self.vars.clear();
    }

    pub fn insert(&mut self, var: &Term) -> bool {
        self.vars
            .insert(term_identity_id(var), var.clone())
            .is_none()
    }

    pub fn insert_var_set(&mut self, vars: &Self) {
        for var in vars.vars.values() {
            self.insert(var);
        }
    }

    pub fn delete_var(&mut self, var: &Term) -> bool {
        self.vars.remove(&term_identity_id(var)).is_some()
    }

    #[must_use]
    pub fn contains(&self, var: &Term) -> bool {
        self.vars.contains_key(&term_identity_id(var))
    }

    /// Collects free variables from the covered term.
    ///
    /// # Panics
    ///
    /// Panics if this set has no covered term.
    pub fn collect_vars(&mut self) -> i64 {
        let term = self
            .term
            .clone()
            .expect("varset collection requires a term");
        self.reset();
        collect_variables(&term, &mut self.vars)
    }

    pub fn union(&mut self, set1: &Self, set2: &Self) {
        self.reset();
        self.vars = set1.vars.clone();
        self.insert_var_set(set2);
    }

    pub fn merge(&mut self, set1: Self) {
        for var in set1.vars.into_values() {
            self.insert(&var);
        }
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.vars.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.vars.is_empty()
    }

    #[must_use]
    pub fn variables(&self) -> Vec<Term> {
        self.vars.values().cloned().collect()
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct VarSetStore {
    sets: BTreeMap<usize, VarSet>,
}

impl VarSetStore {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            sets: BTreeMap::new(),
        }
    }

    #[must_use]
    pub fn find_var_set(&self, key: &Term) -> Option<&VarSet> {
        self.sets.get(&term_identity_id(key))
    }

    pub fn get_var_set(&mut self, key: &Term) -> &mut VarSet {
        self.sets
            .entry(term_identity_id(key))
            .or_insert_with(|| VarSet::new(Some(key.clone())))
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.sets.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.sets.is_empty()
    }

    pub fn clear(&mut self) {
        self.sets.clear();
    }
}

fn collect_variables(term: &Term, vars: &mut BTreeMap<usize, Term>) -> i64 {
    let mut count = 0;
    let mut stack = vec![term.clone()];
    while let Some(current) = stack.pop() {
        if current.is_free_var() {
            if vars.insert(term_identity_id(&current), current).is_none() {
                count += 1;
            }
        } else {
            for arg in current.argument_clones().into_iter().flatten() {
                if !arg.query_prop(TP_IS_GROUND) {
                    stack.push(arg);
                }
            }
        }
    }
    count
}

#[cfg(test)]
mod tests {
    use super::{VarSet, VarSetStore};
    use crate::terms::termtypes::{Term, TP_IS_GROUND};

    fn sample_term() -> (Term, Term, Term, Term) {
        let root = Term::top_alloc(10, 2);
        let x = Term::const_cell_alloc(-2);
        let nested = Term::top_alloc(11, 1);
        let y = Term::const_cell_alloc(-4);
        nested.set_argument(0, y.clone());
        root.set_argument(0, x.clone());
        root.set_argument(1, nested.clone());
        (root, x, nested, y)
    }

    #[test]
    fn insert_delete_contains_and_reset_use_identity() {
        let (_, x, _, _) = sample_term();
        let same_code = Term::const_cell_alloc(-2);
        let mut set = VarSet::new(None);

        assert!(set.insert(&x));
        assert!(!set.insert(&x));
        assert!(set.contains(&x));
        assert!(!set.contains(&same_code));
        assert_eq!(set.len(), 1);
        assert!(set.delete_var(&x));
        assert!(!set.delete_var(&x));
        set.set_valid(true);
        set.reset();
        assert!(set.is_valid());
        assert!(set.is_empty());
    }

    #[test]
    fn collect_vars_skips_ground_subterms_and_counts_new_vars() {
        let (root, x, nested, y) = sample_term();
        nested.set_prop(TP_IS_GROUND);
        let mut set = VarSet::new(Some(root.clone()));

        assert_eq!(set.collect_vars(), 1);
        assert!(set.contains(&x));
        assert!(!set.contains(&y));

        nested.del_prop(TP_IS_GROUND);
        assert_eq!(set.collect_vars(), 2);
        assert!(set.contains(&y));
        assert_eq!(set.term(), Some(root));
    }

    #[test]
    fn set_union_insert_var_set_and_merge_follow_tree_set_semantics() {
        let (_, x, _, y) = sample_term();
        let mut left = VarSet::new(None);
        let mut right = VarSet::new(None);
        left.insert(&x);
        right.insert(&y);

        let mut union = VarSet::new(None);
        union.union(&left, &right);
        assert!(union.contains(&x));
        assert!(union.contains(&y));

        left.insert_var_set(&right);
        assert!(left.contains(&y));
        right.merge(union);
        assert!(right.contains(&x));
        assert!(right.contains(&y));
    }

    #[test]
    fn varset_store_finds_or_creates_sets_by_term_identity() {
        let (root, _, _, _) = sample_term();
        let same_shape = Term::top_alloc(10, 2);
        let mut store = VarSetStore::new();

        assert!(store.find_var_set(&root).is_none());
        store.get_var_set(&root).set_valid(true);
        assert!(store.find_var_set(&root).unwrap().is_valid());
        assert!(store.find_var_set(&same_shape).is_none());
        assert_eq!(store.len(), 1);
        store.clear();
        assert!(store.is_empty());
    }
}
