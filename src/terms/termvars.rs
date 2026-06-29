use crate::basics::pstacks::PStack;
use crate::terms::functypes::FunCode;
use crate::terms::simpletypes::{Type, TypeUniqueId, INVALID_TYPE_UID};
use crate::terms::termtypes::{
    Term, TermProperties, DEFAULT_VWEIGHT, TP_HAS_ETA_EXPANDABLE_SUBTERM, TP_IS_SHARED,
};
use crate::terms::typebanks::TypeBank;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::{Rc, Weak};

pub const INITIAL_SORT_STACK_SIZE: usize = 10;
pub const DEFAULT_VARBANK_SIZE: usize = 30;

#[derive(Clone, Debug)]
pub struct VarBank(Rc<RefCell<VarBankCell>>);

#[derive(Debug)]
struct VarBankCell {
    id: &'static str,
    var_count: i64,
    fresh_count: FunCode,
    default_type: Type,
    max_var: FunCode,
    varstacks: BTreeMap<TypeUniqueId, Vec<Term>>,
    v_counts: BTreeMap<TypeUniqueId, usize>,
    variables: BTreeMap<FunCode, Term>,
    ext_index: BTreeMap<String, Term>,
    env: Vec<Option<VarBankNamedCell>>,
    shadow: Option<Weak<RefCell<VarBankCell>>>,
}

#[derive(Clone, Debug)]
struct VarBankNamedCell {
    var: Option<Term>,
    name: String,
}

impl VarBank {
    #[must_use]
    pub fn new(sort_table: &TypeBank) -> Self {
        Self(Rc::new(RefCell::new(VarBankCell {
            id: "Unpaired",
            var_count: 0,
            fresh_count: 0,
            default_type: sort_table.default_type(),
            max_var: 0,
            varstacks: BTreeMap::new(),
            v_counts: BTreeMap::new(),
            variables: BTreeMap::new(),
            ext_index: BTreeMap::new(),
            env: Vec::new(),
            shadow: None,
        })))
    }

    #[must_use]
    pub fn id(&self) -> &'static str {
        self.0.borrow().id
    }

    #[must_use]
    pub fn var_count(&self) -> i64 {
        self.0.borrow().var_count
    }

    #[must_use]
    pub fn fresh_count(&self) -> FunCode {
        self.0.borrow().fresh_count
    }

    #[must_use]
    pub fn max_var(&self) -> FunCode {
        self.0.borrow().max_var
    }

    #[must_use]
    pub fn default_type(&self) -> Type {
        self.0.borrow().default_type.clone()
    }

    #[must_use]
    pub fn env_depth(&self) -> usize {
        self.0.borrow().env.len()
    }

    /// Pairs two variable banks so later allocations keep matching f-codes.
    ///
    /// # Panics
    ///
    /// Panics if `secondary` already contains variables. The C function has
    /// the same assertion precondition.
    pub fn pair_shadow(&self, secondary: &Self) {
        assert_eq!(secondary.var_count(), 0, "secondary varbank must be empty");
        let existing = self.normal_variables_by_sort();
        {
            let mut primary = self.0.borrow_mut();
            primary.shadow = Some(Rc::downgrade(&secondary.0));
            primary.id = "Primary";
        }
        {
            let mut secondary_inner = secondary.0.borrow_mut();
            secondary_inner.shadow = Some(Rc::downgrade(&self.0));
            secondary_inner.id = "Secondary";
            for vars in existing.values() {
                for var in vars {
                    let type_ = var.type_().expect("varbank variables have types");
                    alloc_no_shadow(&mut secondary_inner, var.f_code(), &type_);
                }
            }
            secondary_inner.fresh_count = self.fresh_count();
        }
    }

    pub fn reset_v_counts(&self) {
        let mut inner = self.0.borrow_mut();
        for value in inner.v_counts.values_mut() {
            *value = 0;
        }
    }

    pub fn set_v_counts_to_used(&self) {
        let mut inner = self.0.borrow_mut();
        let sorts: Vec<_> = inner
            .varstacks
            .keys()
            .chain(inner.v_counts.keys())
            .copied()
            .collect();
        for sort in sorts {
            let used = inner.varstacks.get(&sort).map_or(0, Vec::len);
            inner.v_counts.insert(sort, used);
        }
    }

    pub fn set_fresh_count_to_used(&self) {
        {
            let mut inner = self.0.borrow_mut();
            let used = if inner.max_var % 2 == 0 {
                inner.max_var
            } else {
                inner.max_var + 1
            };
            inner.fresh_count = inner.fresh_count.max(used);
        }
        self.sync_shadow_fresh_count();
    }

    pub fn clear_ext_names(&self) {
        self.clear_ext_names_no_reset();
        self.reset_v_counts();
    }

    pub fn clear_ext_names_no_reset(&self) {
        let mut inner = self.0.borrow_mut();
        inner.ext_index.clear();
        inner.env.clear();
    }

    pub fn vars_set_prop(&self, prop: TermProperties) {
        for var in self.all_variables() {
            var.set_prop(prop);
        }
    }

    pub fn vars_del_prop(&self, prop: TermProperties) {
        for var in self.all_variables() {
            var.del_prop(prop);
        }
    }

    /// Finds a variable by negative function code.
    ///
    /// # Panics
    ///
    /// Panics if `f_code` is not negative, matching the C assertion.
    #[must_use]
    pub fn f_code_find(&self, f_code: FunCode) -> Option<Term> {
        assert!(f_code < 0, "variable f-code must be negative");
        self.0.borrow().variables.get(&f_code).cloned()
    }

    #[must_use]
    pub fn ext_name_find(&self, name: &str) -> Option<Term> {
        self.0.borrow().ext_index.get(name).cloned()
    }

    /// Returns an existing variable or allocates the requested code and type.
    ///
    /// # Panics
    ///
    /// Panics if `f_code` is non-negative, if `type_` has no shared UID, or if
    /// an existing variable with that code has a different type.
    #[must_use]
    pub fn var_assert_alloc(&self, f_code: FunCode, type_: &Type) -> Term {
        assert!(f_code < 0, "variable f-code must be negative");
        assert_shared_type(type_);
        if let Some(var) = self.f_code_find(f_code) {
            assert_eq!(var.v_count(), 1);
            assert_eq!(var.type_(), Some(type_.clone()));
            return var;
        }
        self.var_alloc(f_code, type_)
    }

    /// Allocates a new variable with the requested code and type.
    ///
    /// # Panics
    ///
    /// Panics if `f_code` is non-negative, if `type_` has no shared UID, or if
    /// a variable with the same code already exists.
    #[must_use]
    pub fn var_alloc(&self, f_code: FunCode, type_: &Type) -> Term {
        assert!(f_code < 0, "variable f-code must be negative");
        assert_shared_type(type_);
        let (var, shadow) = {
            let mut inner = self.0.borrow_mut();
            let var = alloc_no_shadow(&mut inner, f_code, type_);
            (var, inner.shadow.clone())
        };
        if let Some(shadow) = shadow.and_then(|weak| weak.upgrade()) {
            let mut shadow_inner = shadow.borrow_mut();
            if !shadow_inner.variables.contains_key(&f_code) {
                alloc_no_shadow(&mut shadow_inner, f_code, type_);
            }
        }
        var
    }

    /// Returns the next fresh even-numbered variable for `type_`.
    ///
    /// # Panics
    ///
    /// Panics if `type_` has no shared UID.
    #[must_use]
    pub fn get_fresh_var(&self, type_: &Type) -> Term {
        assert_shared_type(type_);
        let sort = type_.type_uid();
        let mut allocate_code = None;
        let mut existing = None;
        {
            let mut inner = self.0.borrow_mut();
            let v_count = *inner.v_counts.get(&sort).unwrap_or(&0);
            let stack_len = inner.varstacks.entry(sort).or_default().len();
            if stack_len <= v_count {
                inner.fresh_count += 2;
                allocate_code = Some(-inner.fresh_count);
            } else {
                existing = inner
                    .varstacks
                    .get(&sort)
                    .and_then(|stack| stack.get(v_count))
                    .cloned();
            }
            inner.v_counts.insert(sort, v_count + 1);
        }

        if let Some(var) = existing {
            return var;
        }

        let f_code = allocate_code.expect("fresh allocation code was set");
        let var = self.var_assert_alloc(f_code, type_);
        self.sync_shadow_fresh_count();
        var
    }

    /// Returns the alternative odd-numbered variable matching `term`.
    ///
    /// # Panics
    ///
    /// Panics if `term` is not a normal negative even-code variable with a
    /// shared type.
    #[must_use]
    pub fn get_alt_var(&self, term: &Term) -> Term {
        assert!(term.f_code() < 0, "variable f-code must be negative");
        assert!(!is_alt_var(term), "expected a normal variable");
        let type_ = term.type_().expect("varbank variables have types");
        self.var_assert_alloc(term.f_code() + 1, &type_)
    }

    /// Returns the alternative variable for the next fresh variable.
    ///
    /// # Panics
    ///
    /// Panics if `type_` has no shared UID.
    #[must_use]
    pub fn get_alt_fresh_var(&self, type_: &Type) -> Term {
        let fresh = self.get_fresh_var(type_);
        self.get_alt_var(&fresh)
    }

    /// Returns a variable associated with `name`, using the default sort.
    ///
    /// # Panics
    ///
    /// Panics if the default type stored in the variable bank has no shared UID.
    #[must_use]
    pub fn ext_name_assert_alloc(&self, name: &str) -> Term {
        if let Some(var) = self.ext_name_find(name) {
            return var;
        }
        let type_ = self.0.borrow().default_type.clone();
        let var = self.get_fresh_var(&type_);
        self.0
            .borrow_mut()
            .ext_index
            .insert(name.to_owned(), var.clone());
        var
    }

    /// Returns a variable associated with `name` and `type_`.
    ///
    /// # Panics
    ///
    /// Panics if `type_` has no shared UID.
    #[must_use]
    pub fn ext_name_assert_alloc_sort(&self, name: &str, type_: &Type) -> Term {
        assert_shared_type(type_);
        let old = self.ext_name_find(name);
        match old {
            None => {
                let var = self.get_fresh_var(type_);
                self.0
                    .borrow_mut()
                    .ext_index
                    .insert(name.to_owned(), var.clone());
                var
            }
            Some(var) if var.type_() == Some(type_.clone()) => var,
            Some(var) => {
                self.0.borrow_mut().env.push(Some(VarBankNamedCell {
                    var: Some(var.clone()),
                    name: name.to_owned(),
                }));
                let new_var = self.get_fresh_var(type_);
                self.0
                    .borrow_mut()
                    .ext_index
                    .insert(name.to_owned(), new_var.clone());
                new_var
            }
        }
    }

    /// Declares a new scoped external variable name using the default sort.
    ///
    /// Unlike [`Self::ext_name_assert_alloc`], this always allocates a fresh
    /// variable when `name` already exists, preserving the old binding for
    /// restoration by [`Self::pop_env`].
    ///
    /// # Panics
    ///
    /// Panics if the default type stored in the variable bank has no shared UID.
    #[must_use]
    pub fn ext_name_declare_alloc(&self, name: &str) -> Term {
        let type_ = self.0.borrow().default_type.clone();
        self.ext_name_declare_alloc_sort(name, &type_)
    }

    /// Declares a new scoped external variable name with `type_`.
    ///
    /// # Panics
    ///
    /// Panics if `type_` has no shared UID.
    #[must_use]
    pub fn ext_name_declare_alloc_sort(&self, name: &str, type_: &Type) -> Term {
        assert_shared_type(type_);
        let old = self.ext_name_find(name);
        if !self.0.borrow().env.is_empty() {
            self.0.borrow_mut().env.push(Some(VarBankNamedCell {
                var: old,
                name: name.to_owned(),
            }));
        }
        let new_var = self.get_fresh_var(type_);
        self.0
            .borrow_mut()
            .ext_index
            .insert(name.to_owned(), new_var.clone());
        new_var
    }

    pub fn push_env(&self) {
        self.0.borrow_mut().env.push(None);
    }

    pub fn pop_env(&self) {
        let mut inner = self.0.borrow_mut();
        while let Some(named) = inner.env.pop() {
            let Some(named) = named else {
                break;
            };
            if let Some(var) = named.var {
                inner.ext_index.insert(named.name, var);
            } else {
                inner.ext_index.remove(&named.name);
            }
        }
    }

    #[must_use]
    pub fn cardinality(&self) -> i64 {
        self.0.borrow().var_count
    }

    pub fn collect_vars(&self, into: &mut PStack<Term>) -> i64 {
        let inner = self.0.borrow();
        let mut count = 0;
        for index in 0..inner.max_var {
            if let Some(var) = inner.variables.get(&-index) {
                into.push(var.clone());
                count += 1;
            }
        }
        count
    }

    #[must_use]
    pub fn normal_stack_len(&self, type_: &Type) -> usize {
        let sort = type_.type_uid();
        self.0.borrow().varstacks.get(&sort).map_or(0, Vec::len)
    }

    #[must_use]
    pub fn normal_variables_by_sort(&self) -> BTreeMap<TypeUniqueId, Vec<Term>> {
        self.0.borrow().varstacks.clone()
    }

    #[must_use]
    pub fn v_count_for_type(&self, type_: &Type) -> usize {
        self.0
            .borrow()
            .v_counts
            .get(&type_.type_uid())
            .copied()
            .unwrap_or(0)
    }

    fn sync_shadow_fresh_count(&self) {
        let (fresh_count, shadow) = {
            let inner = self.0.borrow();
            (inner.fresh_count, inner.shadow.clone())
        };
        if let Some(shadow) = shadow.and_then(|weak| weak.upgrade()) {
            shadow.borrow_mut().fresh_count = fresh_count;
        }
    }

    fn all_variables(&self) -> Vec<Term> {
        self.0.borrow().variables.values().cloned().collect()
    }
}

#[must_use]
pub const fn f_code_is_alt_code(f_code: FunCode) -> bool {
    f_code % 2 != 0
}

#[must_use]
pub fn is_alt_var(var: &Term) -> bool {
    f_code_is_alt_code(var.f_code())
}

fn alloc_no_shadow(inner: &mut VarBankCell, f_code: FunCode, type_: &Type) -> Term {
    assert!(
        !inner.variables.contains_key(&f_code),
        "variable f-code already allocated"
    );
    let sort = type_.type_uid();
    let var = Term::default_cell_alloc();
    var.set_prop(TP_IS_SHARED);
    if type_.is_arrow() {
        var.set_prop(TP_HAS_ETA_EXPANDABLE_SUBTERM);
    }
    var.set_weight(DEFAULT_VWEIGHT);
    var.set_v_count(1);
    var.set_f_count(0);
    var.set_entry_no(f_code);
    var.set_f_code(f_code);
    var.set_type(Some(type_.clone()));

    inner.variables.insert(f_code, var.clone());
    if !is_alt_var(&var) {
        inner.varstacks.entry(sort).or_default().push(var.clone());
    }
    inner.max_var = inner.max_var.max(-f_code);
    inner.var_count += 1;
    var
}

fn assert_shared_type(type_: &Type) {
    assert_ne!(
        type_.type_uid(),
        INVALID_TYPE_UID,
        "varbank types must be shared through a TypeBank"
    );
}

#[cfg(test)]
mod tests {
    use super::{
        f_code_is_alt_code, is_alt_var, VarBank, DEFAULT_VARBANK_SIZE, INITIAL_SORT_STACK_SIZE,
    };
    use crate::basics::pstacks::PStack;
    use crate::terms::simpletypes::alloc_arrow_type;
    use crate::terms::termtypes::{TP_CHECK_FLAG, TP_HAS_ETA_EXPANDABLE_SUBTERM, TP_IS_SHARED};
    use crate::terms::typebanks::TypeBank;

    #[test]
    fn constants_and_alt_code_checks_match_c_header() {
        assert_eq!(INITIAL_SORT_STACK_SIZE, 10);
        assert_eq!(DEFAULT_VARBANK_SIZE, 30);
        assert!(f_code_is_alt_code(-1));
        assert!(!f_code_is_alt_code(-2));
    }

    #[test]
    fn fresh_variables_are_even_sorted_and_reused_by_v_count() {
        let types = TypeBank::new();
        let bank = VarBank::new(&types);
        let i_type = types.i_type();

        let first = bank.get_fresh_var(&i_type);
        let second = bank.get_fresh_var(&i_type);
        assert_eq!(first.f_code(), -2);
        assert_eq!(second.f_code(), -4);
        assert_eq!(bank.fresh_count(), 4);
        assert_eq!(bank.v_count_for_type(&i_type), 2);
        assert_eq!(bank.normal_stack_len(&i_type), 2);
        assert!(first.query_prop(TP_IS_SHARED));
        assert_eq!(first.weight(), 1);
        assert_eq!(first.v_count(), 1);
        assert_eq!(first.f_count(), 0);
        assert_eq!(first.entry_no(), -2);
        assert_eq!(first.type_(), Some(i_type.clone()));

        bank.reset_v_counts();
        let reused = bank.get_fresh_var(&i_type);
        assert_eq!(reused, first);
        assert_eq!(bank.fresh_count(), 4);
    }

    #[test]
    fn explicit_and_alt_allocations_update_indices_like_c() {
        let types = TypeBank::new();
        let bank = VarBank::new(&types);
        let i_type = types.i_type();

        let var = bank.var_assert_alloc(-8, &i_type);
        assert_eq!(bank.var_assert_alloc(-8, &i_type), var);
        assert_eq!(bank.f_code_find(-8), Some(var.clone()));
        assert_eq!(bank.normal_stack_len(&i_type), 1);
        assert_eq!(bank.max_var(), 8);
        assert_eq!(bank.cardinality(), 1);

        let alt = bank.get_alt_var(&var);
        assert_eq!(alt.f_code(), -7);
        assert!(is_alt_var(&alt));
        assert_eq!(bank.normal_stack_len(&i_type), 1);
        assert_eq!(bank.cardinality(), 2);
    }

    #[test]
    fn arrow_typed_variables_get_eta_expandable_flag() {
        let mut types = TypeBank::new();
        let arrow =
            types.insert_type_shared(alloc_arrow_type(vec![types.i_type(), types.bool_type()]));
        let bank = VarBank::new(&types);

        let var = bank.get_fresh_var(&arrow);
        assert!(var.query_prop(TP_HAS_ETA_EXPANDABLE_SUBTERM));
    }

    #[test]
    fn external_names_use_default_sort_and_clear_resets_v_counts() {
        let types = TypeBank::new();
        let bank = VarBank::new(&types);

        let x = bank.ext_name_assert_alloc("X");
        assert_eq!(x.f_code(), -2);
        assert_eq!(bank.ext_name_assert_alloc("X"), x);
        assert_eq!(bank.ext_name_find("X"), Some(x));
        assert_eq!(bank.v_count_for_type(&types.i_type()), 1);

        bank.clear_ext_names();
        assert!(bank.ext_name_find("X").is_none());
        assert_eq!(bank.v_count_for_type(&types.i_type()), 0);
    }

    #[test]
    fn sorted_external_name_scopes_restore_previous_binding() {
        let types = TypeBank::new();
        let mut user_types = TypeBank::new();
        let person_code = user_types.define_simple_sort("person").unwrap();
        let person = user_types
            .insert_type_shared(crate::terms::simpletypes::alloc_simple_sort(person_code));
        let bank = VarBank::new(&types);
        bank.push_env();

        let x_i = bank.ext_name_assert_alloc_sort("X", &types.i_type());
        let x_person = bank.ext_name_assert_alloc_sort("X", &person);
        assert_ne!(x_i, x_person);
        assert_eq!(bank.ext_name_find("X"), Some(x_person));
        assert_eq!(bank.env_depth(), 2);

        bank.pop_env();
        assert_eq!(bank.ext_name_find("X"), Some(x_i));
        assert_eq!(bank.env_depth(), 0);
    }

    #[test]
    fn scoped_external_name_declaration_shadows_same_sort_binding() {
        let types = TypeBank::new();
        let bank = VarBank::new(&types);
        bank.push_env();
        let outer = bank.ext_name_declare_alloc("X");

        bank.push_env();
        let inner = bank.ext_name_declare_alloc("X");
        assert_ne!(outer, inner);
        assert_eq!(outer.f_code(), -2);
        assert_eq!(inner.f_code(), -4);
        assert_eq!(bank.ext_name_find("X"), Some(inner));

        bank.pop_env();
        assert_eq!(bank.ext_name_find("X"), Some(outer));
        bank.pop_env();
        assert_eq!(bank.ext_name_find("X"), None);
    }

    #[test]
    fn sorted_scoped_external_name_declaration_shadows_same_sort_binding() {
        let types = TypeBank::new();
        let bank = VarBank::new(&types);
        bank.push_env();
        let outer = bank.ext_name_declare_alloc_sort("X", &types.i_type());

        bank.push_env();
        let inner = bank.ext_name_declare_alloc_sort("X", &types.i_type());
        assert_ne!(outer, inner);
        assert_eq!(bank.ext_name_find("X"), Some(inner));

        bank.pop_env();
        assert_eq!(bank.ext_name_find("X"), Some(outer));
        bank.pop_env();
        assert_eq!(bank.ext_name_find("X"), None);
    }

    #[test]
    fn set_v_counts_to_used_skips_existing_allocations() {
        let types = TypeBank::new();
        let bank = VarBank::new(&types);
        let i_type = types.i_type();
        let first = bank.get_fresh_var(&i_type);
        let _ = bank.get_fresh_var(&i_type);

        bank.reset_v_counts();
        assert_eq!(bank.get_fresh_var(&i_type), first);
        bank.set_v_counts_to_used();
        let third = bank.get_fresh_var(&i_type);
        assert_eq!(third.f_code(), -6);
    }

    #[test]
    fn set_fresh_count_to_used_skips_explicit_high_codes() {
        let types = TypeBank::new();
        let bank = VarBank::new(&types);
        let i_type = types.i_type();
        let _ = bank.var_assert_alloc(-8, &i_type);

        bank.set_v_counts_to_used();
        bank.set_fresh_count_to_used();
        let next = bank.get_fresh_var(&i_type);

        assert_eq!(next.f_code(), -10);
    }

    #[test]
    fn bank_property_helpers_touch_all_variables() {
        let types = TypeBank::new();
        let bank = VarBank::new(&types);
        let first = bank.get_fresh_var(&types.i_type());
        let alt = bank.get_alt_var(&first);

        bank.vars_set_prop(TP_CHECK_FLAG);
        assert!(first.query_prop(TP_CHECK_FLAG));
        assert!(alt.query_prop(TP_CHECK_FLAG));
        bank.vars_del_prop(TP_CHECK_FLAG);
        assert!(!first.query_prop(TP_CHECK_FLAG));
        assert!(!alt.query_prop(TP_CHECK_FLAG));
    }

    #[test]
    fn shadow_banks_copy_existing_vars_and_follow_new_allocations() {
        let types = TypeBank::new();
        let primary = VarBank::new(&types);
        let secondary = VarBank::new(&types);
        let first = primary.get_fresh_var(&types.i_type());

        primary.pair_shadow(&secondary);
        assert_eq!(primary.id(), "Primary");
        assert_eq!(secondary.id(), "Secondary");
        assert_eq!(secondary.f_code_find(first.f_code()).unwrap().f_code(), -2);
        assert_eq!(secondary.fresh_count(), primary.fresh_count());

        let second = primary.get_fresh_var(&types.i_type());
        assert_eq!(second.f_code(), -4);
        assert!(secondary.f_code_find(-4).is_some());
        assert_eq!(secondary.fresh_count(), primary.fresh_count());
    }

    #[test]
    fn collect_vars_preserves_c_loop_bound_quirk() {
        let types = TypeBank::new();
        let bank = VarBank::new(&types);
        let first = bank.get_fresh_var(&types.i_type());
        let second = bank.get_fresh_var(&types.i_type());
        let mut stack = PStack::new();

        let count = bank.collect_vars(&mut stack);

        assert_eq!(count, 1);
        assert_eq!(stack.as_slice(), &[first]);
        assert_ne!(stack.as_slice(), &[second]);
    }
}
