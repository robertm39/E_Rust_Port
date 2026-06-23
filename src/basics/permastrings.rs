use std::collections::BTreeSet;
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PermaStringRegistry {
    strings: BTreeSet<Arc<str>>,
}

impl PermaStringRegistry {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            strings: BTreeSet::new(),
        }
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.strings.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.strings.is_empty()
    }

    pub fn perma_string(&mut self, text: &str) -> Arc<str> {
        if let Some(existing) = self.strings.get(text) {
            return Arc::clone(existing);
        }

        let interned = Arc::<str>::from(text);
        self.strings.insert(Arc::clone(&interned));
        interned
    }

    pub fn perma_string_store(&mut self, text: String) -> Arc<str> {
        if let Some(existing) = self.strings.get(text.as_str()) {
            return Arc::clone(existing);
        }

        let interned = Arc::<str>::from(text);
        self.strings.insert(Arc::clone(&interned));
        interned
    }

    pub fn clear(&mut self) {
        self.strings.clear();
    }
}

static GLOBAL_REGISTRY: OnceLock<Mutex<PermaStringRegistry>> = OnceLock::new();

fn global_registry() -> &'static Mutex<PermaStringRegistry> {
    GLOBAL_REGISTRY.get_or_init(|| Mutex::new(PermaStringRegistry::new()))
}

fn lock_global_registry() -> MutexGuard<'static, PermaStringRegistry> {
    match global_registry().lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

#[must_use]
pub fn perma_string(text: &str) -> Arc<str> {
    lock_global_registry().perma_string(text)
}

pub fn maybe_perma_string(text: Option<&str>) -> Option<Arc<str>> {
    text.map(perma_string)
}

#[must_use]
pub fn perma_string_store(text: String) -> Arc<str> {
    lock_global_registry().perma_string_store(text)
}

pub fn perma_strings_free() {
    lock_global_registry().clear();
}

#[cfg(test)]
mod tests {
    use super::{
        maybe_perma_string, perma_string, perma_string_store, perma_strings_free,
        PermaStringRegistry,
    };
    use std::sync::Arc;

    #[test]
    fn registry_reuses_existing_allocation_for_equal_strings() {
        let mut registry = PermaStringRegistry::new();
        let first = registry.perma_string("alpha");
        let second = registry.perma_string("alpha");
        let third = registry.perma_string("beta");

        assert!(Arc::ptr_eq(&first, &second));
        assert!(!Arc::ptr_eq(&first, &third));
        assert_eq!(registry.len(), 2);
    }

    #[test]
    fn store_consumes_owned_string_and_shares_duplicates() {
        let mut registry = PermaStringRegistry::new();
        let first = registry.perma_string_store(String::from("alpha"));
        let second = registry.perma_string_store(String::from("alpha"));

        assert!(Arc::ptr_eq(&first, &second));
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn clear_drops_registry_references_without_invalidating_returned_arcs() {
        let mut registry = PermaStringRegistry::new();
        let first = registry.perma_string("alpha");
        registry.clear();
        assert!(registry.is_empty());

        let second = registry.perma_string("alpha");
        assert_eq!(&*first, "alpha");
        assert_eq!(&*second, "alpha");
        assert!(!Arc::ptr_eq(&first, &second));
    }

    #[test]
    fn global_helpers_match_c_null_and_free_shapes() {
        perma_strings_free();
        assert!(maybe_perma_string(None).is_none());

        let first = maybe_perma_string(Some("alpha")).unwrap();
        let second = perma_string_store(String::from("alpha"));
        assert!(Arc::ptr_eq(&first, &second));

        perma_strings_free();
        let third = perma_string("alpha");
        assert_eq!(&*third, "alpha");
        assert!(!Arc::ptr_eq(&first, &third));
        perma_strings_free();
    }
}
