use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct GcSetHandle(usize);

impl GcSetHandle {
    /// Creates an opaque clause/formula-set handle.
    ///
    /// # Panics
    ///
    /// Panics if `id` is zero. The C registration helpers assert that set
    /// pointers are non-null.
    #[must_use]
    pub const fn new(id: usize) -> Self {
        assert!(id != 0, "GC set handle must be nonzero");
        Self(id)
    }

    #[must_use]
    pub const fn id(self) -> usize {
        self.0
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct GcAdmin {
    clause_sets: BTreeSet<GcSetHandle>,
    formula_sets: BTreeSet<GcSetHandle>,
}

impl GcAdmin {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            clause_sets: BTreeSet::new(),
            formula_sets: BTreeSet::new(),
        }
    }

    #[must_use]
    pub fn clause_set_count(&self) -> usize {
        self.clause_sets.len()
    }

    #[must_use]
    pub fn formula_set_count(&self) -> usize {
        self.formula_sets.len()
    }

    #[must_use]
    pub fn has_clause_set(&self, set: GcSetHandle) -> bool {
        self.clause_sets.contains(&set)
    }

    #[must_use]
    pub fn has_formula_set(&self, set: GcSetHandle) -> bool {
        self.formula_sets.contains(&set)
    }

    pub fn clause_set_handles(&self) -> impl Iterator<Item = GcSetHandle> + '_ {
        self.clause_sets.iter().copied()
    }

    pub fn formula_set_handles(&self) -> impl Iterator<Item = GcSetHandle> + '_ {
        self.formula_sets.iter().copied()
    }
}

#[must_use]
pub const fn gc_admin_alloc() -> GcAdmin {
    GcAdmin::new()
}

pub fn gc_admin_free(_junk: GcAdmin) {}

pub fn gc_register_formula_set(gc: &mut GcAdmin, set: GcSetHandle) {
    gc.formula_sets.insert(set);
}

pub fn gc_register_clause_set(gc: &mut GcAdmin, set: GcSetHandle) {
    gc.clause_sets.insert(set);
}

pub fn gc_deregister_formula_set(gc: &mut GcAdmin, set: GcSetHandle) {
    gc.formula_sets.remove(&set);
}

pub fn gc_deregister_clause_set(gc: &mut GcAdmin, set: GcSetHandle) {
    gc.clause_sets.remove(&set);
}

#[cfg(test)]
mod tests {
    use super::{
        gc_admin_alloc, gc_admin_free, gc_deregister_clause_set, gc_deregister_formula_set,
        gc_register_clause_set, gc_register_formula_set, GcSetHandle,
    };

    #[test]
    fn allocation_starts_with_empty_registries() {
        let gc = gc_admin_alloc();

        assert_eq!(gc.clause_set_count(), 0);
        assert_eq!(gc.formula_set_count(), 0);
        gc_admin_free(gc);
    }

    #[test]
    fn registration_uses_pointer_identity_sets() {
        let mut gc = gc_admin_alloc();
        let clause = GcSetHandle::new(1);
        let formula = GcSetHandle::new(2);

        gc_register_clause_set(&mut gc, clause);
        gc_register_clause_set(&mut gc, clause);
        gc_register_formula_set(&mut gc, formula);
        gc_register_formula_set(&mut gc, formula);

        assert_eq!(gc.clause_set_count(), 1);
        assert_eq!(gc.formula_set_count(), 1);
        assert!(gc.has_clause_set(clause));
        assert!(gc.has_formula_set(formula));
    }

    #[test]
    fn deregistration_removes_existing_sets_and_ignores_missing_ones() {
        let mut gc = gc_admin_alloc();
        let clause = GcSetHandle::new(1);
        let formula = GcSetHandle::new(2);

        gc_register_clause_set(&mut gc, clause);
        gc_register_formula_set(&mut gc, formula);
        gc_deregister_clause_set(&mut gc, GcSetHandle::new(3));
        gc_deregister_formula_set(&mut gc, GcSetHandle::new(4));

        assert!(gc.has_clause_set(clause));
        assert!(gc.has_formula_set(formula));

        gc_deregister_clause_set(&mut gc, clause);
        gc_deregister_formula_set(&mut gc, formula);

        assert!(!gc.has_clause_set(clause));
        assert!(!gc.has_formula_set(formula));
        assert_eq!(gc.clause_set_count(), 0);
        assert_eq!(gc.formula_set_count(), 0);
    }
}
