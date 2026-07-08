use crate::terms::termtypes::{term_deref, DerefType, Term};
use std::collections::BTreeMap;

pub const VAR_HASH_SIZE: usize = 16;
pub const VAR_HASH_MASK: i64 = 15;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VarHashEntry {
    key: Term,
    value: i64,
}

impl VarHashEntry {
    #[must_use]
    pub fn key(&self) -> Term {
        self.key.clone()
    }

    #[must_use]
    pub const fn value(&self) -> i64 {
        self.value
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VarHash {
    buckets: Vec<Vec<VarHashEntry>>,
}

impl Default for VarHash {
    fn default() -> Self {
        Self::new()
    }
}

impl VarHash {
    #[must_use]
    pub fn new() -> Self {
        Self {
            buckets: vec![Vec::new(); VAR_HASH_SIZE],
        }
    }

    /// Finds an entry for `var`.
    ///
    /// # Panics
    ///
    /// Panics if `var` is not a negative-code variable, matching the C hash
    /// function assertion.
    #[must_use]
    pub fn find(&self, var: &Term) -> Option<&VarHashEntry> {
        let index = var_hash_function(var);
        var_hash_list_find(&self.buckets[index], var)
    }

    /// Adds `value` to the stored variable count, inserting if needed.
    ///
    /// # Panics
    ///
    /// Panics if `var` is not a negative-code variable, matching the C hash
    /// function assertion.
    pub fn add_value(&mut self, var: &Term, value: i64) -> i64 {
        let index = var_hash_function(var);
        if let Some(entry) = self.buckets[index]
            .iter_mut()
            .find(|entry| entry.key == *var)
        {
            entry.value += value;
            entry.value
        } else {
            self.buckets[index].insert(
                0,
                VarHashEntry {
                    key: var.clone(),
                    value,
                },
            );
            value
        }
    }

    /// Adds variable occurrences from `term`.
    ///
    /// # Panics
    ///
    /// Panics if a counted variable has a non-negative f-code, matching the C
    /// hash function assertion.
    pub fn add_var_distrib(&mut self, term: &Term, deref: DerefType, add: i64) {
        let mut stack = vec![(term.clone(), deref)];
        while let Some((candidate, mut current_deref)) = stack.pop() {
            let current = term_deref(&candidate, &mut current_deref);
            if current.is_free_var() {
                self.add_value(&current, add);
            } else {
                for arg in current.argument_clones().into_iter().flatten() {
                    stack.push((arg, current_deref));
                }
            }
        }
    }

    #[must_use]
    pub fn bucket_len(&self, index: usize) -> Option<usize> {
        self.buckets.get(index).map(Vec::len)
    }

    #[must_use]
    pub fn entries(&self) -> Vec<VarHashEntry> {
        self.buckets
            .iter()
            .flat_map(|bucket| bucket.iter().cloned())
            .collect()
    }
}

/// Hashes a variable by negative f-code.
///
/// # Panics
///
/// Panics if `var` is not a negative-code variable.
#[must_use]
pub fn var_hash_function(var: &Term) -> usize {
    assert!(var.f_code() < 0, "variable f-code must be negative");
    usize::try_from((-var.f_code()) & VAR_HASH_MASK).expect("var hash index fits usize")
}

#[must_use]
pub fn var_hash_list_find<'a>(list: &'a [VarHashEntry], var: &Term) -> Option<&'a VarHashEntry> {
    list.iter().find(|entry| entry.key == *var)
}

/// Adds variable occurrences to an integer distribution keyed by `-f_code`.
///
/// # Panics
///
/// Panics if Rust integer arithmetic overflows while accumulating the
/// distribution in debug builds.
pub fn add_var_distrib_to_map(
    distrib: &mut BTreeMap<i64, i64>,
    term: &Term,
    deref: DerefType,
    add: i64,
) {
    let mut stack = vec![(term.clone(), deref)];
    while let Some((candidate, mut current_deref)) = stack.pop() {
        let current = term_deref(&candidate, &mut current_deref);
        if current.is_free_var() {
            let key = -current.f_code();
            *distrib.entry(key).or_insert(0) += add;
        } else {
            for arg in current.argument_clones().into_iter().flatten() {
                stack.push((arg, current_deref));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        add_var_distrib_to_map, var_hash_function, var_hash_list_find, VarHash, VAR_HASH_MASK,
        VAR_HASH_SIZE,
    };
    use crate::terms::termtypes::{DerefType, Term};
    use std::collections::BTreeMap;

    #[test]
    fn constants_and_hash_function_match_c_header() {
        let var = Term::const_cell_alloc(-17);
        assert_eq!(VAR_HASH_SIZE, 16);
        assert_eq!(VAR_HASH_MASK, 15);
        assert_eq!(var_hash_function(&var), 1);
    }

    #[test]
    fn add_value_inserts_at_bucket_head_and_accumulates() {
        let mut hash = VarHash::new();
        let first = Term::const_cell_alloc(-1);
        let colliding = Term::const_cell_alloc(-17);

        assert_eq!(hash.add_value(&first, 2), 2);
        assert_eq!(hash.add_value(&colliding, 3), 3);
        assert_eq!(hash.bucket_len(1), Some(2));
        assert_eq!(hash.find(&first).unwrap().value(), 2);
        assert_eq!(hash.add_value(&first, 5), 7);
        assert_eq!(hash.find(&first).unwrap().value(), 7);

        let entries = hash.entries();
        assert_eq!(entries[0].key(), colliding);
        assert_eq!(var_hash_list_find(&entries, &first).unwrap().value(), 7);
    }

    #[test]
    fn var_distribution_counts_dereferenced_free_variables() {
        let root = Term::top_alloc(10, 2);
        let x = Term::const_cell_alloc(-2);
        let y = Term::const_cell_alloc(-4);
        let bound = Term::const_cell_alloc(-6);
        y.set_binding(Some(bound.clone()));
        root.set_argument(0, x.clone());
        root.set_argument(1, y);

        let mut hash = VarHash::new();
        hash.add_var_distrib(&root, DerefType::Always, 2);
        assert_eq!(hash.find(&x).unwrap().value(), 2);
        assert_eq!(hash.find(&bound).unwrap().value(), 2);

        let mut distrib = BTreeMap::new();
        add_var_distrib_to_map(&mut distrib, &root, DerefType::Always, 3);
        assert_eq!(distrib.get(&2), Some(&3));
        assert_eq!(distrib.get(&6), Some(&3));
    }
}
