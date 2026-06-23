use crate::terms::functypes::FunCode;
use crate::terms::simpletypes::{Type, INVALID_TYPE_UID};
use crate::terms::termtypes::{
    Term, DEFAULT_FWEIGHT, TP_HAS_DB_SUBTERM, TP_HAS_ETA_EXPANDABLE_SUBTERM, TP_IS_DB_VAR,
    TP_IS_SHARED,
};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DbVarBank {
    vars: BTreeMap<FunCode, BTreeMap<i64, Term>>,
}

impl DbVarBank {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            vars: BTreeMap::new(),
        }
    }

    /// Creates or returns the unique DB variable for `db_index` and `type_`.
    ///
    /// # Panics
    ///
    /// Panics if `db_index` is negative or if `type_` is not shared, matching
    /// the C assertions in `_RequestDBVar`.
    pub fn request_db_var(&mut self, type_: &Type, db_index: FunCode) -> Term {
        assert!(db_index >= 0, "DB variable index must be non-negative");
        assert_ne!(
            type_.type_uid(),
            INVALID_TYPE_UID,
            "DB variable type must be shared"
        );

        self.vars
            .entry(db_index)
            .or_default()
            .entry(type_.type_uid())
            .or_insert_with(|| mk_db(db_index, type_))
            .clone()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.vars.values().map(BTreeMap::len).sum()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.vars.is_empty()
    }

    pub fn clear(&mut self) {
        self.vars.clear();
    }
}

/// Allocates a DB variable term for a shared type/index pair.
///
/// # Panics
///
/// Panics if `db_index` is negative.
#[must_use]
pub fn mk_db(db_index: FunCode, type_: &Type) -> Term {
    assert!(db_index >= 0, "DB variable index must be non-negative");
    let var = Term::default_cell_alloc();
    let mut props = TP_IS_SHARED | TP_IS_DB_VAR | TP_HAS_DB_SUBTERM;
    if type_.is_arrow() {
        props |= TP_HAS_ETA_EXPANDABLE_SUBTERM;
    }
    var.set_prop(props);
    var.set_weight(DEFAULT_FWEIGHT);
    var.set_v_count(0);
    var.set_f_count(1);
    var.set_entry_no(db_index);
    var.set_f_code(db_index);
    var.set_type(Some(type_.clone()));
    var
}

#[cfg(test)]
mod tests {
    use super::{mk_db, DbVarBank};
    use crate::terms::simpletypes::{alloc_arrow_type, alloc_simple_sort, ST_INDIVIDUALS};
    use crate::terms::termtypes::{
        DEFAULT_FWEIGHT, TP_HAS_DB_SUBTERM, TP_HAS_ETA_EXPANDABLE_SUBTERM, TP_IS_DB_VAR,
        TP_IS_SHARED,
    };
    use crate::terms::typebanks::TypeBank;

    #[test]
    fn mk_db_sets_c_term_cell_shape() {
        let mut bank = TypeBank::new();
        let arrow =
            bank.insert_type_shared(alloc_arrow_type(vec![bank.i_type(), bank.bool_type()]));

        let var = mk_db(3, &arrow);

        assert_eq!(var.f_code(), 3);
        assert_eq!(var.entry_no(), 3);
        assert_eq!(var.weight(), DEFAULT_FWEIGHT);
        assert_eq!(var.v_count(), 0);
        assert_eq!(var.f_count(), 1);
        assert_eq!(var.type_(), Some(arrow));
        assert!(var.query_prop(TP_IS_SHARED | TP_IS_DB_VAR | TP_HAS_DB_SUBTERM));
        assert!(var.query_prop(TP_HAS_ETA_EXPANDABLE_SUBTERM));
    }

    #[test]
    fn request_db_var_reuses_identity_for_same_index_and_shared_type() {
        let mut type_bank = TypeBank::new();
        let individual = type_bank.insert_type_shared(alloc_simple_sort(ST_INDIVIDUALS));
        let bool_type = type_bank.bool_type();
        let mut db_bank = DbVarBank::new();

        let first = db_bank.request_db_var(&individual, 0);
        let repeated = db_bank.request_db_var(&individual, 0);
        let different_index = db_bank.request_db_var(&individual, 1);
        let different_type = db_bank.request_db_var(&bool_type, 0);

        assert_eq!(first, repeated);
        assert_ne!(first, different_index);
        assert_ne!(first, different_type);
        assert_eq!(db_bank.len(), 3);
        db_bank.clear();
        assert!(db_bank.is_empty());
    }
}
