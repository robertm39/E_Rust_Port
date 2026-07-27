//! Local comparison cache from `cto_cmpcache`.

use crate::basics::partial_orderings::CompareResult;
use crate::basics::quadtrees::{double_key_cmp, QuadKey, QuadTree};
use crate::terms::termtypes::{term_identity_id, DerefType, Term};
use std::cmp::Ordering;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CmpCache {
    entries: QuadTree<usize, CompareResult>,
}

impl CmpCache {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            entries: QuadTree::new(),
        }
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.nodes()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Clear a nullable comparison cache, matching `CmpCacheClear`.
pub fn cmp_cache_clear(cache: &mut Option<CmpCache>) {
    *cache = None;
}

/// Find a comparison in a nullable comparison cache.
///
/// Returns [`CompareResult::Unknown`] for absent entries and for comparisons
/// involving free variables.
#[must_use]
pub fn cmp_cache_find(
    cache: Option<&mut CmpCache>,
    t1: &Term,
    d1: DerefType,
    t2: &Term,
    d2: DerefType,
) -> CompareResult {
    if t1.is_free_var() || t2.is_free_var() {
        return CompareResult::Unknown;
    }
    let Some(cache) = cache else {
        return CompareResult::Unknown;
    };
    let (key, natural_order) = prepare_key(t1, d1, t2, d2);
    let Some(result) = cache.entries.find(&key).copied() else {
        return CompareResult::Unknown;
    };
    if natural_order {
        result
    } else {
        inverse_cached(result)
    }
}

/// Insert a comparison into a nullable comparison cache.
///
/// Returns `true` only when a new key is inserted. Existing keys may still have
/// their cached relation refined, matching `CmpCacheInsert`.
///
/// # Panics
///
/// Panics if `insert` is [`CompareResult::Unknown`] or if an existing cache
/// entry is incompatible with the inserted relation.
pub fn cmp_cache_insert(
    cache: &mut Option<CmpCache>,
    t1: &Term,
    d1: DerefType,
    t2: &Term,
    d2: DerefType,
    insert: CompareResult,
) -> bool {
    assert_ne!(insert, CompareResult::Unknown);
    if t1.is_free_var() || t2.is_free_var() {
        return false;
    }

    let (key, natural_order) = prepare_key(t1, d1, t2, d2);
    let value = if natural_order {
        insert
    } else {
        inverse_cached(insert)
    };
    let cache = cache.get_or_insert_with(CmpCache::new);
    if let Some(stored) = cache.entries.find_mut(&key) {
        merge_cached(stored, value);
        false
    } else {
        cache.entries.store(key, value)
    }
}

fn prepare_key(t1: &Term, d1: DerefType, t2: &Term, d2: DerefType) -> (QuadKey<usize>, bool) {
    let first = term_identity_id(t1);
    let second = term_identity_id(t2);
    let d1 = deref_key(d1);
    let d2 = deref_key(d2);
    if double_key_cmp(&first, d1, &second, d2) == Ordering::Greater {
        (QuadKey::new(first, d1, second, d2), true)
    } else {
        (QuadKey::new(second, d2, first, d1), false)
    }
}

fn deref_key(deref: DerefType) -> i32 {
    i32::from(deref as u8)
}

fn inverse_cached(result: CompareResult) -> CompareResult {
    result
        .inverse()
        .unwrap_or_else(|| panic!("cached comparison result must be invertible"))
}

fn merge_cached(stored: &mut CompareResult, value: CompareResult) {
    match *stored {
        CompareResult::NotGreaterEqual => {
            if value == CompareResult::NotLessEqual {
                *stored = CompareResult::Uncomparable;
            } else {
                assert!(
                    matches!(
                        value,
                        CompareResult::NotGreaterEqual
                            | CompareResult::Lesser
                            | CompareResult::Uncomparable
                    ),
                    "incompatible not-greater-equal cache refinement: {value:?}"
                );
                *stored = value;
            }
        }
        CompareResult::NotLessEqual => {
            if value == CompareResult::NotGreaterEqual {
                *stored = CompareResult::Uncomparable;
            } else {
                assert!(
                    matches!(
                        value,
                        CompareResult::NotLessEqual
                            | CompareResult::Greater
                            | CompareResult::Uncomparable
                    ),
                    "incompatible not-less-equal cache refinement: {value:?}"
                );
                *stored = value;
            }
        }
        existing => {
            assert!(
                existing == value
                    || (value == CompareResult::NotGreaterEqual
                        && existing == CompareResult::Lesser)
                    || (value == CompareResult::NotLessEqual && existing == CompareResult::Greater),
                "incompatible cached comparison refinement: existing={existing:?} value={value:?}"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{cmp_cache_clear, cmp_cache_find, cmp_cache_insert, CmpCache};
    use crate::basics::partial_orderings::CompareResult;
    use crate::terms::termtypes::{DerefType, Term};

    #[test]
    fn find_reports_unknown_for_empty_cache_and_free_variables() {
        let f = Term::const_cell_alloc(10);
        let g = Term::const_cell_alloc(11);
        let x = Term::const_cell_alloc(-2);
        let mut cache = Some(CmpCache::new());

        assert_eq!(
            cmp_cache_find(None, &f, DerefType::Never, &g, DerefType::Never),
            CompareResult::Unknown
        );
        assert_eq!(
            cmp_cache_find(cache.as_mut(), &x, DerefType::Never, &g, DerefType::Never),
            CompareResult::Unknown
        );
        assert!(!cmp_cache_insert(
            &mut cache,
            &f,
            DerefType::Never,
            &x,
            DerefType::Never,
            CompareResult::Greater
        ));
        assert_eq!(cache.as_ref().map(CmpCache::len), Some(0));
    }

    #[test]
    fn insert_and_find_handle_symmetric_relations() {
        let f = Term::const_cell_alloc(10);
        let g = Term::const_cell_alloc(11);
        let mut cache = None;

        assert!(cmp_cache_insert(
            &mut cache,
            &f,
            DerefType::Never,
            &g,
            DerefType::Always,
            CompareResult::Greater
        ));
        assert_eq!(cache.as_ref().map(CmpCache::len), Some(1));
        assert_eq!(
            cmp_cache_find(cache.as_mut(), &f, DerefType::Never, &g, DerefType::Always),
            CompareResult::Greater
        );
        assert_eq!(
            cmp_cache_find(cache.as_mut(), &g, DerefType::Always, &f, DerefType::Never),
            CompareResult::Lesser
        );
    }

    #[test]
    fn insert_merges_not_greater_and_not_less_into_uncomparable() {
        let f = Term::const_cell_alloc(10);
        let g = Term::const_cell_alloc(11);
        let mut cache = None;

        assert!(cmp_cache_insert(
            &mut cache,
            &f,
            DerefType::Never,
            &g,
            DerefType::Never,
            CompareResult::NotGreaterEqual
        ));
        assert!(!cmp_cache_insert(
            &mut cache,
            &f,
            DerefType::Never,
            &g,
            DerefType::Never,
            CompareResult::NotLessEqual
        ));
        assert_eq!(
            cmp_cache_find(cache.as_mut(), &f, DerefType::Never, &g, DerefType::Never),
            CompareResult::Uncomparable
        );
    }

    #[test]
    fn weaker_cache_refinement_preserves_stronger_existing_result() {
        let f = Term::const_cell_alloc(10);
        let g = Term::const_cell_alloc(11);
        let mut cache = None;

        assert!(cmp_cache_insert(
            &mut cache,
            &f,
            DerefType::Never,
            &g,
            DerefType::Never,
            CompareResult::Greater
        ));
        assert!(!cmp_cache_insert(
            &mut cache,
            &f,
            DerefType::Never,
            &g,
            DerefType::Never,
            CompareResult::NotLessEqual
        ));
        assert_eq!(
            cmp_cache_find(cache.as_mut(), &f, DerefType::Never, &g, DerefType::Never),
            CompareResult::Greater
        );

        cmp_cache_clear(&mut cache);
        assert!(cache.is_none());
    }
}
