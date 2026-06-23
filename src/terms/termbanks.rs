use crate::basics::error::Diagnostic;
use crate::basics::pstacks::PStack;
use crate::terms::dbvars::DbVarBank;
use crate::terms::functypes::FunCode;
use crate::terms::garbage_coll::GcAdmin;
use crate::terms::signature::{Signature, SIG_TRUE_CODE};
use crate::terms::signature::{SIG_DB_LAMBDA_CODE, SIG_FALSE_CODE, SIG_NAMED_LAMBDA_CODE};
use crate::terms::simpletypes::{Type, TypeUniqueId};
use crate::terms::termcellstore::TermCellStore;
use crate::terms::termfunc::{
    term_apply_arg as term_apply_arg_unshared, term_is_ground_compute, term_standard_weight,
};
use crate::terms::termtypes::{
    term_deref, DerefType, Term, TermProperties, DEFAULT_FWEIGHT, DEFAULT_VWEIGHT, TP_GARBAGE_FLAG,
    TP_HAS_APP_VAR, TP_HAS_BOOL_SUBTERM, TP_HAS_DB_SUBTERM, TP_HAS_EQ_NEQ_SYM,
    TP_HAS_ETA_EXPANDABLE_SUBTERM, TP_HAS_LAMBDA_SUBTERM, TP_HAS_NON_PATTERN_VAR, TP_IGNORE_PROPS,
    TP_IS_BETA_REDUCIBLE, TP_IS_GROUND, TP_IS_SHARED, TP_OP_FLAG, TP_PRED_POS,
};
use crate::terms::termvars::VarBank;
use crate::terms::typecheck::type_infer_sort;
use std::collections::BTreeMap;

#[derive(Clone, Debug)]
pub struct TermBank {
    in_count: u64,
    insertions: u64,
    recovered: u64,
    sig: Signature,
    vars: VarBank,
    db_vars: DbVarBank,
    true_term: Term,
    false_term: Term,
    min_terms: BTreeMap<TypeUniqueId, Term>,
    garbage_state: TermProperties,
    gc: GcAdmin,
    term_store: TermCellStore,
}

impl TermBank {
    pub fn new(sig: Signature) -> Result<Self, Diagnostic> {
        let vars = VarBank::new(sig.type_bank());
        let true_term = Term::const_cell_alloc(SIG_TRUE_CODE);
        let false_term = Term::const_cell_alloc(SIG_FALSE_CODE);
        let mut bank = Self {
            in_count: 0,
            insertions: 0,
            recovered: 0,
            sig,
            vars,
            db_vars: DbVarBank::new(),
            true_term: true_term.clone(),
            false_term: false_term.clone(),
            min_terms: BTreeMap::new(),
            garbage_state: TP_IGNORE_PROPS,
            gc: GcAdmin::new(),
            term_store: TermCellStore::new(),
        };

        true_term.set_type(Some(bank.sig.type_bank().bool_type()));
        true_term.set_prop(TP_PRED_POS);
        bank.true_term = bank.insert(&true_term, DerefType::Never)?;

        false_term.set_type(Some(bank.sig.type_bank().bool_type()));
        false_term.set_prop(TP_PRED_POS);
        bank.false_term = bank.insert(&false_term, DerefType::Never)?;

        Ok(bank)
    }

    #[must_use]
    pub const fn in_count(&self) -> u64 {
        self.in_count
    }

    #[must_use]
    pub const fn insertions(&self) -> u64 {
        self.insertions
    }

    #[must_use]
    pub const fn recovered(&self) -> u64 {
        self.recovered
    }

    #[must_use]
    pub const fn signature(&self) -> &Signature {
        &self.sig
    }

    #[must_use]
    pub fn signature_mut(&mut self) -> &mut Signature {
        &mut self.sig
    }

    #[must_use]
    pub const fn vars(&self) -> &VarBank {
        &self.vars
    }

    #[must_use]
    pub const fn gc(&self) -> &GcAdmin {
        &self.gc
    }

    #[must_use]
    pub const fn true_term(&self) -> &Term {
        &self.true_term
    }

    #[must_use]
    pub const fn false_term(&self) -> &Term {
        &self.false_term
    }

    #[must_use]
    pub const fn garbage_state(&self) -> TermProperties {
        self.garbage_state
    }

    #[must_use]
    pub const fn non_var_term_nodes(&self) -> i64 {
        self.term_store.entries()
    }

    /// Returns non-variable term nodes plus bank-owned variables.
    ///
    /// # Panics
    ///
    /// Panics if the term-cell store's maintained entry count differs from a
    /// full node count, matching the C consistency assertion in `TBTermNodes`.
    #[must_use]
    pub fn term_nodes(&self) -> i64 {
        assert_eq!(self.term_store.entries(), self.term_store.count_nodes());
        self.term_store.entries() + self.vars.cardinality()
    }

    /// Inserts a term, recursively sharing all non-variable subterms.
    ///
    /// # Panics
    ///
    /// Panics if a free or DB variable has no type, matching the C
    /// precondition for `TBInsert`.
    pub fn insert(&mut self, term: &Term, deref: DerefType) -> Result<Term, Diagnostic> {
        self.insert_with_mode(term, deref, InsertMode::ShareVariables)
    }

    /// Inserts a term without replacing free variables by bank-owned variables.
    ///
    /// # Panics
    ///
    /// Panics if a DB variable has no type, matching the C precondition.
    pub fn insert_ignore_var(&mut self, term: &Term, deref: DerefType) -> Result<Term, Diagnostic> {
        self.insert_with_mode(term, deref, InsertMode::KeepVariables)
    }

    /// Inserts a term after clearing copied top-cell properties.
    ///
    /// # Panics
    ///
    /// Panics if a free or DB variable has no type, matching the C
    /// precondition for `TBInsertNoProps`.
    pub fn insert_no_props(&mut self, term: &Term, deref: DerefType) -> Result<Term, Diagnostic> {
        self.insert_with_mode(term, deref, InsertMode::NoProperties)
    }

    /// Inserts a term while reusing shared ground subterms unchanged.
    ///
    /// # Panics
    ///
    /// Panics if a ground term is not shared, or if a free/DB variable has no
    /// type, matching the C preconditions for `TBInsertOpt`.
    pub fn insert_opt(&mut self, term: &Term, deref: DerefType) -> Result<Term, Diagnostic> {
        let mut current_deref = deref;
        let term = term_deref(term, &mut current_deref);
        if term_is_ground_for_insert(&term) {
            assert!(
                term.is_shared(),
                "optimized ground insertion expects sharing"
            );
            return Ok(term);
        }
        self.insert_with_mode(&term, current_deref, InsertMode::ShareVariables)
    }

    /// Inserts a term with one subterm replaced after instantiation.
    ///
    /// # Panics
    ///
    /// Panics if `repl` is not already present in this bank, or if a free/DB
    /// variable has no type, matching `TBInsertRepl`.
    pub fn insert_repl(
        &mut self,
        term: &Term,
        deref: DerefType,
        old: &Term,
        repl: &Term,
    ) -> Result<Term, Diagnostic> {
        if term == old {
            assert!(
                self.find(repl).is_some(),
                "replacement must already be in the term bank"
            );
            return Ok(repl.clone());
        }

        let mut current_deref = deref;
        let term = term_deref(term, &mut current_deref);
        if term.is_free_var() {
            let type_ = term.type_().expect("free variable must have a type");
            return Ok(self.vars.var_assert_alloc(term.f_code(), &type_));
        }
        if term.is_db_var() {
            let type_ = term.type_().expect("DB variable must have a type");
            return Ok(self.db_vars.request_db_var(&type_, term.f_code()));
        }

        let copy = Term::top_copy_without_args(&term);
        copy.set_properties(TP_IGNORE_PROPS);
        for (index, arg) in term.argument_clones().into_iter().enumerate() {
            let arg = arg.unwrap_or_else(|| panic!("term argument {index} is uninitialized"));
            let shared = self.insert_repl(&arg, current_deref, old, repl)?;
            copy.set_argument(index, shared);
        }
        self.term_top_insert(copy)
    }

    /// Inserts a plain, non-instantiated replacement when a subterm changes.
    ///
    /// # Panics
    ///
    /// Panics if `repl` is not already present in this bank, matching
    /// `TBInsertReplPlain`.
    pub fn insert_repl_plain(
        &mut self,
        term: &Term,
        old: &Term,
        repl: &Term,
    ) -> Result<Term, Diagnostic> {
        if term == old {
            assert!(
                self.find(repl).is_some(),
                "replacement must already be in the term bank"
            );
            return Ok(repl.clone());
        }
        if term_standard_weight(term) <= term_standard_weight(old) || term.is_any_var() {
            return Ok(term.clone());
        }

        let copy = Term::top_copy_without_args(term);
        copy.set_properties(TP_IGNORE_PROPS);
        let mut changed = false;
        for (index, arg) in term.argument_clones().into_iter().enumerate() {
            let arg = arg.unwrap_or_else(|| panic!("term argument {index} is uninitialized"));
            let replaced = self.insert_repl_plain(&arg, old, repl)?;
            if replaced != arg {
                changed = true;
            }
            copy.set_argument(index, replaced);
        }

        if changed {
            self.term_top_insert(copy)
        } else {
            Ok(term.clone())
        }
    }

    /// Inserts an instantiated term while preserving already-shared ground terms.
    ///
    /// # Panics
    ///
    /// Panics if `deref` exposes a free/DB variable without a type, or if an
    /// inserted parent receives an unshared ground child, matching the C
    /// caller preconditions for `TBInsertInstantiatedDeref`.
    pub fn insert_instantiated_deref(
        &mut self,
        term: &Term,
        deref: DerefType,
    ) -> Result<Term, Diagnostic> {
        if deref == DerefType::Never {
            return Ok(term.clone());
        }

        let mut current_deref = deref;
        let term = term_deref(term, &mut current_deref);
        if term.is_any_var() || term_is_ground_for_insert(&term) {
            return Ok(term);
        }

        let copy = Term::top_copy_without_args(&term);
        copy.set_properties(TP_IGNORE_PROPS);
        for (index, arg) in term.argument_clones().into_iter().enumerate() {
            let arg = arg.unwrap_or_else(|| panic!("term argument {index} is uninitialized"));
            let shared = self.insert_instantiated_deref(&arg, current_deref)?;
            copy.set_argument(index, shared);
        }
        self.term_top_insert(copy)
    }

    /// Inserts a first-order instantiated term.
    ///
    /// # Panics
    ///
    /// Panics if a ground or bound term is not already present in the bank, or
    /// if a free/DB variable has no type, matching `TBInsertInstantiatedFO`.
    pub fn insert_instantiated_fo(&mut self, term: &Term) -> Result<Term, Diagnostic> {
        if term_is_ground_for_insert(term) {
            assert!(
                self.find(term).is_some(),
                "ground instantiated terms must already be in the bank"
            );
            return Ok(term.clone());
        }
        if let Some(binding) = term.binding() {
            assert!(
                self.find(&binding).is_some(),
                "variable binding must already be in the bank"
            );
            return Ok(binding);
        }
        if term.is_free_var() {
            let type_ = term.type_().expect("free variable must have a type");
            return Ok(self.vars.var_assert_alloc(term.f_code(), &type_));
        }
        if term.is_db_var() {
            let type_ = term.type_().expect("DB variable must have a type");
            return Ok(self.db_vars.request_db_var(&type_, term.f_code()));
        }

        let copy = Term::top_copy_without_args(term);
        copy.set_properties(TP_IGNORE_PROPS);
        for (index, arg) in term.argument_clones().into_iter().enumerate() {
            let arg = arg.unwrap_or_else(|| panic!("term argument {index} is uninitialized"));
            let shared = self.insert_instantiated_fo(&arg)?;
            copy.set_argument(index, shared);
        }
        self.term_top_insert(copy)
    }

    /// Inserts a copy whose free variables are replaced by alternative vars.
    ///
    /// # Panics
    ///
    /// Panics if an input free variable is already an alternative variable, or
    /// if a free/DB variable has no type, matching the C `TBInsertDisjoint`
    /// preconditions.
    pub fn insert_disjoint(&mut self, term: &Term) -> Result<Term, Diagnostic> {
        if term_is_ground_for_insert(term) {
            return Ok(term.clone());
        }
        if term.is_free_var() {
            return Ok(self.vars.get_alt_var(term));
        }
        if term.is_db_var() {
            let type_ = term.type_().expect("DB variable must have a type");
            return Ok(self.db_vars.request_db_var(&type_, term.f_code()));
        }

        let copy = Term::top_copy_without_args(term);
        for (index, arg) in term.argument_clones().into_iter().enumerate() {
            let arg = arg.unwrap_or_else(|| panic!("term argument {index} is uninitialized"));
            let shared = self.insert_disjoint(&arg)?;
            copy.set_argument(index, shared);
        }
        self.term_top_insert(copy)
    }

    /// Inserts a top cell whose arguments are already shared.
    ///
    /// # Panics
    ///
    /// Panics if `term` is a variable, lambda/application invariants are
    /// violated, or a top-cell child is neither shared nor a free variable.
    pub fn term_top_insert(&mut self, term: Term) -> Result<Term, Diagnostic> {
        assert!(!term.is_any_var(), "top insertion expects a non-variable");
        assert!(
            term.f_code() != SIG_NAMED_LAMBDA_CODE
                || (term.arity() == 2 && term.argument(0).is_some_and(|arg| arg.is_free_var())),
            "named lambda top cell has invalid binder shape"
        );
        assert!(
            term.f_code() != SIG_DB_LAMBDA_CODE
                || (term.arity() == 2 && term.argument(0).is_some_and(|arg| arg.is_db_var())),
            "DB lambda top cell has invalid binder shape"
        );
        assert!(
            !term.is_phony_app()
                || term
                    .argument(0)
                    .is_some_and(|arg| arg.is_any_var() || arg.is_lambda()),
            "phony application must apply a variable or lambda"
        );
        assert!(
            !term.is_phony_app() || term.arity() > 1,
            "phony application needs at least one argument"
        );
        for arg in term.argument_clones().into_iter().flatten() {
            assert!(
                arg.is_shared() || arg.is_free_var(),
                "term-bank top insertion requires shared arguments"
            );
        }

        if term.type_().is_none() {
            type_infer_sort(&mut self.sig, &term)?;
            assert!(term.type_().is_some(), "type inference assigned a sort");
        }

        self.insertions += 1;
        if let Some(existing) = self.term_store.insert(term.clone()) {
            existing.set_prop(term.properties());
            return Ok(existing);
        }

        self.in_count += 1;
        term.set_entry_no(i64::try_from(self.in_count).unwrap_or(i64::MAX));
        term.assign_prop(TP_GARBAGE_FLAG, self.garbage_state);
        term.set_prop(TP_IS_SHARED);
        self.set_top_insert_metadata(&term);
        assert_eq!(self.find(&term), Some(term.clone()));
        Ok(term)
    }

    /// Finds a term representation in this bank.
    ///
    /// # Panics
    ///
    /// Panics if a DB variable has no type, matching `TBFind`/`TBRequestDBVar`.
    pub fn find(&mut self, term: &Term) -> Option<Term> {
        if term.is_free_var() {
            self.vars.f_code_find(term.f_code())
        } else if term.is_db_var() {
            let type_ = term.type_().expect("DB variable must have a type");
            Some(self.db_vars.request_db_var(&type_, term.f_code()))
        } else {
            self.term_store.find(term)
        }
    }

    pub fn create_const_term(&mut self, f_code: FunCode) -> Result<Term, Diagnostic> {
        let term = Term::const_cell_alloc(f_code);
        self.insert(&term, DerefType::Never)
    }

    /// Returns the cached minimal term for the constant's type.
    ///
    /// # Panics
    ///
    /// Panics if the signature has no type for `min_const`, matching the C
    /// assertion in `TBCreateMinTerm`.
    pub fn create_min_term(&mut self, min_const: FunCode) -> Result<Term, Diagnostic> {
        let type_ = self
            .sig
            .get_type(min_const)
            .expect("minimal constant must have a declared type")
            .clone();
        let type_uid = type_.type_uid();
        if let Some(term) = self.min_terms.get(&type_uid) {
            return Ok(term.clone());
        }
        let term = self.create_const_term(min_const)?;
        assert!(term.type_().is_some(), "minimal term has a type");
        self.min_terms.insert(type_uid, term.clone());
        Ok(term)
    }

    pub fn get_first_const_term(&mut self, sort: &Type) -> Result<Option<Term>, Diagnostic> {
        self.sig
            .collect_sort_consts(sort)
            .first()
            .copied()
            .map(|f_code| self.create_const_term(f_code))
            .transpose()
    }

    /// Returns a shared constant selected by the supplied C-style comparator.
    ///
    /// # Panics
    ///
    /// Panics if either distribution array is not large enough to address all
    /// currently known function codes, matching the C precondition that these
    /// arrays are sized to `sig->f_count + 1`.
    pub fn get_freq_const_term<F>(
        &mut self,
        sort: &Type,
        conj_dist_array: &[i64],
        dist_array: &[i64],
        mut is_better: F,
    ) -> Result<Option<Term>, Diagnostic>
    where
        F: FnMut(FunCode, FunCode, &[i64], &[i64]) -> bool,
    {
        let candidates = self.sig.collect_sort_consts(sort);
        let Some((&first, rest)) = candidates.split_first() else {
            return Ok(None);
        };

        let required_len = usize::try_from(self.sig.f_count())
            .unwrap_or(usize::MAX)
            .saturating_add(1);
        assert!(
            conj_dist_array.len() >= required_len,
            "conjecture distribution must cover all function codes"
        );
        assert!(
            dist_array.len() >= required_len,
            "global distribution must cover all function codes"
        );

        let mut best = first;
        for &candidate in rest {
            if is_better(candidate, best, conj_dist_array, dist_array) {
                best = candidate;
            }
        }
        self.create_const_term(best).map(Some)
    }

    /// Applies `mapper` recursively like C `TermMap`.
    ///
    /// `mapper` must return either `None` to stop recursion at `term`, or a
    /// shared term with the same type as the input term. Returning a different
    /// term restarts mapping at that replacement; returning the same term maps
    /// the arguments recursively.
    ///
    /// # Panics
    ///
    /// Panics if `mapper` returns an unshared or differently typed term, or if
    /// recursive argument mapping produces an unshared or differently typed
    /// argument, matching the C assertions in `TermMap`.
    pub fn map_term<F>(&mut self, term: &Term, map_fn: &mut F) -> Result<Term, Diagnostic>
    where
        F: FnMut(&mut Self, &Term) -> Result<Option<Term>, Diagnostic>,
    {
        let Some(mapped_term) = map_fn(self, term)? else {
            return Ok(term.clone());
        };
        assert!(
            mapped_term.is_shared(),
            "term mapper must return shared terms"
        );
        assert_eq!(
            mapped_term.type_(),
            term.type_(),
            "term mapper must preserve term type"
        );

        if mapped_term != term.clone() {
            return self.map_term(&mapped_term, map_fn);
        }
        if term.is_any_var() {
            return Ok(term.clone());
        }

        let copy = Term::top_copy_without_args(term);
        let mut changed = false;
        for (index, arg) in term.argument_clones().into_iter().enumerate() {
            let arg = arg.unwrap_or_else(|| panic!("term argument {index} is uninitialized"));
            let mapped_arg = self.map_term(&arg, map_fn)?;
            assert!(
                mapped_arg.is_shared(),
                "term mapper must return shared arguments"
            );
            assert_eq!(
                mapped_arg.type_(),
                arg.type_(),
                "term mapper must preserve argument type"
            );
            if mapped_arg != arg {
                changed = true;
            }
            copy.set_argument(index, mapped_arg);
        }

        if changed {
            self.term_top_insert(copy)
        } else {
            Ok(term.clone())
        }
    }

    #[must_use]
    pub fn term_apply_arg(&mut self, source: &Term, arg: &Term) -> Term {
        term_apply_arg_unshared(self.sig.type_bank_mut(), source, arg)
    }

    /// Sets properties by repointing `term_ref` to the banked top-cell variant.
    ///
    /// # Panics
    ///
    /// Panics if `term_ref` is a variable, matching the C assertion in
    /// `TBRefSetProp`.
    pub fn ref_set_prop(
        &mut self,
        term_ref: &mut Term,
        prop: TermProperties,
    ) -> Result<(), Diagnostic> {
        assert!(
            !term_ref.is_any_var(),
            "properties do not work for variables"
        );
        if term_ref.query_prop(prop) {
            return Ok(());
        }
        let new = Term::top_copy(term_ref);
        new.set_prop(prop);
        *term_ref = self.term_top_insert(new)?;
        Ok(())
    }

    pub fn ref_del_prop(
        &mut self,
        term_ref: &mut Term,
        prop: TermProperties,
    ) -> Result<(), Diagnostic> {
        if !term_ref.is_any_prop_set(prop) || term_ref.is_any_var() {
            return Ok(());
        }
        let new = Term::top_copy(term_ref);
        new.del_prop(prop);
        *term_ref = self.term_top_insert(new)?;
        Ok(())
    }

    pub fn gc_mark_term(&self, term: &Term) {
        let mut stack = vec![term.clone()];
        while let Some(current) = stack.pop() {
            if current.give_props(TP_GARBAGE_FLAG) == self.garbage_state {
                current.flip_prop(TP_GARBAGE_FLAG);
                stack.extend(current.argument_clones().into_iter().flatten());
                if current.is_rewritten() {
                    if let Some(replacement) = current.rw_replace_field() {
                        stack.push(replacement);
                    }
                }
            }
        }
    }

    /// Sweeps unmarked terms and flips the bank garbage state.
    ///
    /// # Panics
    ///
    /// Panics if the bank's `$true` term is currently marked rewritten,
    /// matching the C assertion before `TBGCSweep` marks roots.
    #[must_use]
    pub fn gc_sweep(&mut self) -> i64 {
        assert!(
            !self.true_term.is_rewritten(),
            "true term must not be rewritten during GC"
        );
        self.gc_mark_term(&self.true_term);
        self.gc_mark_term(&self.false_term);
        for term in self.min_terms.values() {
            self.gc_mark_term(term);
        }
        let recovered = self.term_store.gc_sweep(self.garbage_state);
        self.garbage_state = if self.garbage_state == TP_IGNORE_PROPS {
            TP_GARBAGE_FLAG
        } else {
            TP_IGNORE_PROPS
        };
        self.recovered += u64::try_from(recovered).unwrap_or(0);
        recovered
    }

    #[must_use]
    pub fn find_repr(&mut self, term: &Term) -> Option<Term> {
        if term.is_any_var() || term.is_const() {
            return self.find(term);
        }

        let work = Term::top_copy(term);
        for (index, arg) in term.argument_clones().into_iter().enumerate() {
            let arg = arg?;
            let repr = self.find_repr(&arg)?;
            work.set_argument(index, repr);
        }
        self.find(&work)
    }

    fn insert_with_mode(
        &mut self,
        term: &Term,
        deref: DerefType,
        mode: InsertMode,
    ) -> Result<Term, Diagnostic> {
        let mut current_deref = deref;
        let term = term_deref(term, &mut current_deref);

        if term.is_free_var() {
            if mode == InsertMode::KeepVariables {
                return Ok(term);
            }
            let type_ = term.type_().expect("free variable must have a type");
            return Ok(self.vars.var_assert_alloc(term.f_code(), &type_));
        }
        if term.is_db_var() {
            let type_ = term.type_().expect("DB variable must have a type");
            return Ok(self.db_vars.request_db_var(&type_, term.f_code()));
        }

        let copy = Term::top_copy_without_args(&term);
        if mode == InsertMode::NoProperties {
            copy.set_properties(TP_IGNORE_PROPS);
        }
        for (index, arg) in term.argument_clones().into_iter().enumerate() {
            let arg = arg.unwrap_or_else(|| panic!("term argument {index} is uninitialized"));
            let shared = self.insert_with_mode(&arg, current_deref, mode)?;
            copy.set_argument(index, shared);
        }
        self.term_top_insert(copy)
    }

    fn set_top_insert_metadata(&self, term: &Term) {
        let type_ = term.type_().expect("shared terms have types");
        if term.is_db_var() {
            term.set_prop(TP_HAS_DB_SUBTERM);
        }
        if type_.is_bool() {
            term.set_prop(TP_HAS_BOOL_SUBTERM);
        }
        if term.is_phony_app() && term.argument(0).is_some_and(|arg| arg.is_lambda()) {
            term.set_prop(TP_IS_BETA_REDUCIBLE);
        }
        if term.is_lambda() {
            term.set_prop(TP_HAS_LAMBDA_SUBTERM);
        }
        if type_.is_arrow() && !term.is_lambda() {
            term.set_prop(TP_HAS_ETA_EXPANDABLE_SUBTERM);
        }
        if matches!(term.f_code(), code if code == self.sig.eqn_code() || code == self.sig.neqn_code())
        {
            term.set_prop(TP_HAS_EQ_NEQ_SYM);
        }

        let mut v_count = 0_u32;
        let mut f_count = u32::from(!term.is_phony_app());
        let mut weight = DEFAULT_FWEIGHT * i64::from(f_count);
        for (index, arg) in term.argument_clones().into_iter().enumerate() {
            let arg = arg.unwrap_or_else(|| panic!("term argument {index} is uninitialized"));
            term.set_prop(arg.give_props(TP_IS_BETA_REDUCIBLE));
            term.set_prop(arg.give_props(TP_HAS_DB_SUBTERM));
            term.set_prop(arg.give_props(TP_HAS_EQ_NEQ_SYM));
            term.set_prop(arg.give_props(TP_HAS_BOOL_SUBTERM));
            if arg.type_().is_some_and(|type_| type_.is_bool()) {
                term.set_prop(TP_HAS_BOOL_SUBTERM);
            }
            term.set_prop(arg.give_props(TP_HAS_LAMBDA_SUBTERM));
            if (!(term.is_phony_app() || term.is_lambda())) || index != 0 {
                term.set_prop(arg.give_props(TP_HAS_ETA_EXPANDABLE_SUBTERM));
            }
            term.set_prop(arg.give_props(TP_HAS_NON_PATTERN_VAR));
            term.set_prop(arg.give_props(TP_HAS_APP_VAR));

            if arg.is_free_var() {
                v_count += 1;
                weight += DEFAULT_VWEIGHT;
            } else {
                v_count += arg.v_count();
                f_count += arg.f_count();
                weight += arg.weight();
            }
        }

        if term.f_code() == SIG_DB_LAMBDA_CODE {
            f_count = f_count
                .checked_sub(2)
                .expect("DB lambda count includes binder and lambda sign");
            weight -= 2 * DEFAULT_FWEIGHT;
        }

        if term.is_applied_free_var() {
            term.set_prop(TP_HAS_APP_VAR | TP_HAS_NON_PATTERN_VAR);
        }

        term.set_v_count(v_count);
        term.set_f_count(f_count);
        term.set_weight(weight);
        if v_count == 0 {
            term.set_prop(TP_IS_GROUND);
        }
    }
}

#[must_use]
pub fn tb_term_del_prop_count(term: &Term, prop: TermProperties) -> i64 {
    let mut count = 0;
    let mut stack = vec![term.clone()];
    while let Some(current) = stack.pop() {
        if current.query_prop(prop) {
            current.del_prop(prop);
            count += 1;
            stack.extend(current.argument_clones().into_iter().flatten());
        }
    }
    count
}

#[must_use]
pub fn tb_term_set_prop_count(term: &Term, prop: TermProperties) -> i64 {
    let mut count = 0;
    let mut stack = vec![term.clone()];
    while let Some(current) = stack.pop() {
        if !current.query_prop(prop) {
            current.set_prop(prop);
            count += 1;
            stack.extend(current.argument_clones().into_iter().flatten());
        }
    }
    count
}

/// Collects shared subterms, using `TPOpFlag` as the visited marker.
///
/// # Panics
///
/// Panics if `term` is not shared, matching the C assertion in
/// `TBTermCollectSubterms`.
pub fn tb_term_collect_subterms(term: &Term, collector: &mut PStack<Term>) -> i64 {
    assert!(term.is_shared(), "subterm collection expects shared terms");
    if term.query_prop(TP_OP_FLAG) {
        return 0;
    }

    let mut count = 1;
    term.set_prop(TP_OP_FLAG);
    collector.push(term.clone());
    for arg in term.argument_clones().into_iter().flatten() {
        count += tb_term_collect_subterms(&arg, collector);
    }
    count
}

#[must_use]
pub fn tb_cell_ident(term: &Term) -> i64 {
    if term.is_free_var() {
        term.f_code()
    } else {
        term.entry_no()
    }
}

#[must_use]
pub fn term_is_true_term(term: &Term) -> bool {
    term.f_code() == SIG_TRUE_CODE
}

#[must_use]
pub fn term_is_false_term(term: &Term) -> bool {
    term.f_code() == SIG_FALSE_CODE
}

#[must_use]
pub fn tb_term_is_ground(term: &Term) -> bool {
    term.query_prop(TP_IS_GROUND)
}

#[must_use]
pub fn tb_term_is_type_term(term: &Term) -> bool {
    term.weight() == DEFAULT_VWEIGHT + DEFAULT_FWEIGHT
}

#[must_use]
pub fn tb_term_is_x_type_term(term: &Term) -> bool {
    term.arity() != 0
        && term.weight()
            == DEFAULT_FWEIGHT + i64::try_from(term.arity()).unwrap_or(0) * DEFAULT_VWEIGHT
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum InsertMode {
    ShareVariables,
    KeepVariables,
    NoProperties,
}

#[must_use]
fn term_is_ground_for_insert(term: &Term) -> bool {
    if term.is_shared() {
        tb_term_is_ground(term)
    } else {
        term_is_ground_compute(term)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        tb_cell_ident, tb_term_collect_subterms, tb_term_del_prop_count, tb_term_is_ground,
        tb_term_set_prop_count, term_is_false_term, term_is_true_term, TermBank,
    };
    use crate::basics::pstacks::PStack;
    use crate::terms::replace::{term_add_rw_link, RwResultType};
    use crate::terms::signature::{Signature, SIG_FALSE_CODE, SIG_PHONY_APP_CODE, SIG_TRUE_CODE};
    use crate::terms::simpletypes::{alloc_arrow_type, alloc_simple_sort};
    use crate::terms::termtypes::{
        DerefType, Term, DEFAULT_FWEIGHT, DEFAULT_VWEIGHT, TP_CHECK_FLAG, TP_GARBAGE_FLAG,
        TP_HAS_APP_VAR, TP_HAS_NON_PATTERN_VAR, TP_IS_GROUND, TP_IS_SHARED, TP_OP_FLAG,
        TP_PRED_POS,
    };
    use crate::terms::typebanks::TypeBank;

    fn bank_with_symbol(name: &str, arity: i32) -> (TermBank, i64) {
        let mut sig = Signature::new(TypeBank::new());
        let f_code = sig.insert_id(name, arity, false);
        let type_ = if arity == 0 {
            sig.type_bank().i_type()
        } else {
            let mut args = vec![sig.type_bank().i_type(); usize::try_from(arity).unwrap_or(0)];
            args.push(sig.type_bank().i_type());
            sig.type_bank_mut()
                .insert_type_shared(alloc_arrow_type(args))
        };
        sig.declare_type(f_code, type_).unwrap();
        (TermBank::new(sig).unwrap(), f_code)
    }

    #[test]
    fn allocation_inserts_true_and_false_constants() {
        let bank = TermBank::new(Signature::new(TypeBank::new())).unwrap();

        assert_eq!(bank.in_count(), 2);
        assert_eq!(bank.insertions(), 2);
        assert_eq!(bank.non_var_term_nodes(), 2);
        assert_eq!(bank.term_nodes(), 2);
        assert!(term_is_true_term(bank.true_term()));
        assert!(term_is_false_term(bank.false_term()));
        assert_eq!(bank.true_term().f_code(), SIG_TRUE_CODE);
        assert_eq!(bank.false_term().f_code(), SIG_FALSE_CODE);
        assert!(bank
            .true_term()
            .query_prop(TP_IS_SHARED | TP_IS_GROUND | TP_PRED_POS));
        assert_eq!(bank.true_term().weight(), DEFAULT_FWEIGHT);
    }

    #[test]
    fn insert_recursively_shares_non_variable_terms_and_computes_counts() {
        let (mut bank, f_code) = bank_with_symbol("f", 1);
        let a_code = bank.signature_mut().insert_id("a", 0, false);
        let i_type = bank.signature().type_bank().i_type();
        bank.signature_mut().declare_type(a_code, i_type).unwrap();
        let a = Term::const_cell_alloc(a_code);
        let f = Term::top_alloc(f_code, 1);
        f.set_argument(0, a);

        let shared = bank.insert(&f, DerefType::Never).unwrap();

        assert!(shared.query_prop(TP_IS_SHARED | TP_IS_GROUND));
        assert_eq!(shared.v_count(), 0);
        assert_eq!(shared.f_count(), 2);
        assert_eq!(shared.weight(), DEFAULT_FWEIGHT * 2);
        assert!(shared.argument(0).unwrap().is_shared());
        assert_eq!(bank.in_count(), 4);
        assert_eq!(bank.term_nodes(), 4);
        assert_eq!(bank.find(&shared), Some(shared.clone()));
        assert_eq!(tb_cell_ident(&shared), shared.entry_no());
        assert!(tb_term_is_ground(&shared));
    }

    #[test]
    fn duplicate_top_insertion_reuses_existing_cell_and_merges_properties() {
        let (mut bank, a_code) = bank_with_symbol("a", 0);
        let first = Term::const_cell_alloc(a_code);
        first.set_prop(TP_PRED_POS);
        let shared_first = bank.insert(&first, DerefType::Never).unwrap();
        let duplicate = Term::const_cell_alloc(a_code);

        let shared_duplicate = bank.insert(&duplicate, DerefType::Never).unwrap();

        assert_eq!(shared_first, shared_duplicate);
        assert_eq!(bank.in_count(), 3);
        assert_eq!(bank.non_var_term_nodes(), 3);
        assert!(shared_duplicate.query_prop(TP_PRED_POS));
    }

    #[test]
    fn insert_uses_bank_variables_unless_asked_to_keep_variables() {
        let (mut bank, _f_code) = bank_with_symbol("a", 0);
        let var = Term::const_cell_alloc(-2);
        var.set_type(Some(bank.signature().type_bank().i_type()));

        let shared = bank.insert(&var, DerefType::Never).unwrap();
        let kept = bank.insert_ignore_var(&var, DerefType::Never).unwrap();

        assert_ne!(shared, var);
        assert_eq!(kept, var);
        assert_eq!(shared.f_code(), -2);
        assert_eq!(tb_cell_ident(&shared), -2);
        assert_eq!(bank.term_nodes(), 3);
    }

    #[test]
    fn optimized_insertion_reuses_shared_ground_terms_and_follows_bindings() {
        let (mut bank, f_code) = bank_with_symbol("f", 1);
        let a_code = bank.signature_mut().insert_id("a", 0, false);
        let i_type = bank.signature().type_bank().i_type();
        bank.signature_mut()
            .declare_type(a_code, i_type.clone())
            .unwrap();
        let a = bank.create_const_term(a_code).unwrap();
        let before = bank.term_nodes();

        assert_eq!(bank.insert_opt(&a, DerefType::Never).unwrap(), a);
        assert_eq!(bank.term_nodes(), before);

        let var = Term::const_cell_alloc(-2);
        var.set_type(Some(i_type));
        var.set_binding(Some(a.clone()));
        assert_eq!(bank.insert_opt(&var, DerefType::Always).unwrap(), a);

        let unshared = Term::top_alloc(f_code, 1);
        unshared.set_argument(0, var);
        let inserted = bank.insert_opt(&unshared, DerefType::Always).unwrap();
        assert_eq!(inserted.argument(0), Some(a));
        assert!(inserted.is_shared());
    }

    #[test]
    fn replacement_insertions_clear_new_top_properties_and_skip_plain_noops() {
        let (mut bank, f_code) = bank_with_symbol("f", 2);
        let old_code = bank.signature_mut().insert_id("old", 0, false);
        let repl_code = bank.signature_mut().insert_id("repl", 0, false);
        let keep_code = bank.signature_mut().insert_id("keep", 0, false);
        let i_type = bank.signature().type_bank().i_type();
        bank.signature_mut()
            .declare_type(old_code, i_type.clone())
            .unwrap();
        bank.signature_mut()
            .declare_type(repl_code, i_type.clone())
            .unwrap();
        bank.signature_mut()
            .declare_type(keep_code, i_type)
            .unwrap();
        let old = bank.create_const_term(old_code).unwrap();
        let repl = bank.create_const_term(repl_code).unwrap();
        let keep = bank.create_const_term(keep_code).unwrap();
        let root = Term::top_alloc(f_code, 2);
        root.set_prop(TP_CHECK_FLAG);
        root.set_argument(0, old.clone());
        root.set_argument(1, keep.clone());

        let instantiated = bank
            .insert_repl(&root, DerefType::Never, &old, &repl)
            .unwrap();
        assert_eq!(instantiated.argument(0), Some(repl.clone()));
        assert_eq!(instantiated.argument(1), Some(keep.clone()));
        assert!(!instantiated.query_prop(TP_CHECK_FLAG));

        let shared_root = bank.insert_no_props(&root, DerefType::Never).unwrap();
        let plain = bank.insert_repl_plain(&shared_root, &old, &repl).unwrap();
        assert_eq!(plain.argument(0), Some(repl));
        assert_eq!(plain.argument(1), Some(keep.clone()));
        assert!(plain.is_shared());

        let no_change = bank.insert_repl_plain(&keep, &old, &plain).unwrap();
        assert_eq!(no_change, keep);
    }

    #[test]
    fn instantiated_insertions_reuse_bound_and_ground_bank_terms() {
        let (mut bank, f_code) = bank_with_symbol("f", 1);
        let a_code = bank.signature_mut().insert_id("a", 0, false);
        let i_type = bank.signature().type_bank().i_type();
        bank.signature_mut()
            .declare_type(a_code, i_type.clone())
            .unwrap();
        let a = bank.create_const_term(a_code).unwrap();
        let var = Term::const_cell_alloc(-2);
        var.set_type(Some(i_type));
        var.set_binding(Some(a.clone()));

        assert_eq!(
            bank.insert_instantiated_deref(&var, DerefType::Never)
                .unwrap(),
            var
        );
        assert_eq!(
            bank.insert_instantiated_deref(&var, DerefType::Once)
                .unwrap(),
            a
        );

        let root = Term::top_alloc(f_code, 1);
        root.set_prop(TP_CHECK_FLAG);
        root.set_argument(0, var.clone());
        let derefed = bank
            .insert_instantiated_deref(&root, DerefType::Once)
            .unwrap();
        assert_eq!(derefed.argument(0), Some(a.clone()));
        assert!(!derefed.query_prop(TP_CHECK_FLAG));

        let fo = bank.insert_instantiated_fo(&root).unwrap();
        assert_eq!(fo.argument(0), Some(a.clone()));
        assert!(!fo.query_prop(TP_CHECK_FLAG));
        assert_eq!(bank.insert_instantiated_fo(&a).unwrap(), a);
    }

    #[test]
    fn disjoint_insertion_maps_normal_variables_to_alternative_variables() {
        let (mut bank, f_code) = bank_with_symbol("f", 1);
        let i_type = bank.signature().type_bank().i_type();
        let var = Term::const_cell_alloc(-2);
        var.set_type(Some(i_type));
        let root = Term::top_alloc(f_code, 1);
        root.set_argument(0, var);

        let disjoint = bank.insert_disjoint(&root).unwrap();
        let alt = disjoint.argument(0).unwrap();

        assert_eq!(alt.f_code(), -1);
        assert!(alt.is_free_var());
        assert!(disjoint.is_shared());
    }

    #[test]
    fn create_min_term_caches_by_declared_type() {
        let mut sig = Signature::new(TypeBank::new());
        let a = sig.insert_id("a", 0, false);
        let b = sig.insert_id("b", 0, false);
        let i_type = sig.type_bank().i_type();
        sig.declare_type(a, i_type.clone()).unwrap();
        sig.declare_type(b, i_type).unwrap();
        let mut bank = TermBank::new(sig).unwrap();

        let first = bank.create_min_term(a).unwrap();
        let second = bank.create_min_term(b).unwrap();

        assert_eq!(first, second);
        assert_eq!(first.f_code(), a);
    }

    #[test]
    fn sort_constant_selection_uses_signature_candidate_order_and_frequency_comparator() {
        let mut sig = Signature::new(TypeBank::new());
        let individual = sig.type_bank().i_type();
        let animal_code = sig.type_bank_mut().define_simple_sort("$animal").unwrap();
        let animal = sig
            .type_bank_mut()
            .insert_type_shared(alloc_simple_sort(animal_code));
        let mineral_code = sig.type_bank_mut().define_simple_sort("$mineral").unwrap();
        let mineral = sig
            .type_bank_mut()
            .insert_type_shared(alloc_simple_sort(mineral_code));

        let first_individual = sig.insert_id("first_individual", 0, false);
        let second_individual = sig.insert_id("second_individual", 0, false);
        sig.declare_final_type(second_individual, individual.clone())
            .unwrap();
        let animal_const = sig.insert_id("animal_const", 0, false);
        sig.declare_final_type(animal_const, animal.clone())
            .unwrap();
        let unary = sig.insert_id("unary", 1, false);
        sig.declare_final_type(
            unary,
            alloc_arrow_type(vec![individual.clone(), individual.clone()]),
        )
        .unwrap();

        let mut bank = TermBank::new(sig).unwrap();
        assert_eq!(
            bank.get_first_const_term(&individual)
                .unwrap()
                .unwrap()
                .f_code(),
            first_individual
        );
        assert_eq!(
            bank.get_first_const_term(&animal)
                .unwrap()
                .unwrap()
                .f_code(),
            animal_const
        );
        assert!(bank.get_first_const_term(&mineral).unwrap().is_none());

        let len = usize::try_from(bank.signature().f_count()).unwrap() + 1;
        let conj_dist_array = vec![0; len];
        let mut dist_array = vec![0; len];
        dist_array[usize::try_from(first_individual).unwrap()] = 7;
        dist_array[usize::try_from(second_individual).unwrap()] = 2;

        let selected = bank
            .get_freq_const_term(
                &individual,
                &conj_dist_array,
                &dist_array,
                |candidate, best, _, dist| {
                    dist[usize::try_from(candidate).unwrap()] < dist[usize::try_from(best).unwrap()]
                },
            )
            .unwrap()
            .unwrap();
        assert_eq!(selected.f_code(), second_individual);
        assert!(selected.is_shared());
    }

    #[test]
    fn term_map_recurses_only_when_mapper_returns_the_same_shared_term() {
        let (mut bank, f_code) = bank_with_symbol("f", 1);
        let a_code = bank.signature_mut().insert_id("a", 0, false);
        let b_code = bank.signature_mut().insert_id("b", 0, false);
        let i_type = bank.signature().type_bank().i_type();
        bank.signature_mut()
            .declare_type(a_code, i_type.clone())
            .unwrap();
        bank.signature_mut().declare_type(b_code, i_type).unwrap();
        let a = bank.create_const_term(a_code).unwrap();
        let b = bank.create_const_term(b_code).unwrap();
        let root = Term::top_alloc(f_code, 1);
        root.set_argument(0, a.clone());
        let shared_root = bank.insert(&root, DerefType::Never).unwrap();

        let mut stopped_visits = 0;
        let stopped = bank
            .map_term(&shared_root, &mut |_, _| {
                stopped_visits += 1;
                Ok(None)
            })
            .unwrap();
        assert_eq!(stopped, shared_root);
        assert_eq!(stopped_visits, 1);

        let mut replace_a = |_: &mut TermBank, term: &Term| {
            if term == &a {
                Ok(Some(b.clone()))
            } else {
                Ok(Some(term.clone()))
            }
        };
        let mapped_root = bank.map_term(&shared_root, &mut replace_a).unwrap();

        assert_ne!(mapped_root, shared_root);
        assert_eq!(mapped_root.argument(0), Some(b));
        assert!(mapped_root.is_shared());
    }

    #[test]
    fn term_apply_arg_uses_bank_type_storage_and_preserves_application_shape() {
        let mut type_bank = TypeBank::new();
        let i_type = type_bank.i_type();
        let arrow =
            type_bank.insert_type_shared(alloc_arrow_type(vec![i_type.clone(), i_type.clone()]));
        let mut sig = Signature::new(type_bank);
        let f_code = sig.insert_id("f", 0, false);
        sig.declare_type(f_code, arrow).unwrap();
        let a_code = sig.insert_id("a", 0, false);
        sig.declare_type(a_code, i_type.clone()).unwrap();
        let mut bank = TermBank::new(sig).unwrap();
        let function = bank.create_const_term(f_code).unwrap();
        let arg = bank.create_const_term(a_code).unwrap();

        let applied = bank.term_apply_arg(&function, &arg);

        assert_eq!(applied.f_code(), f_code);
        assert_eq!(applied.arity(), 1);
        assert_eq!(applied.argument(0), Some(arg));
        assert_eq!(applied.type_(), Some(i_type));
    }

    #[test]
    fn applied_free_var_is_marked_non_pattern_until_normalization_is_ported() {
        let mut type_bank = TypeBank::new();
        let arrow = type_bank.insert_type_shared(alloc_arrow_type(vec![
            type_bank.i_type(),
            type_bank.i_type(),
        ]));
        let mut sig = Signature::new(type_bank);
        let arg_code = sig.insert_id("a", 0, false);
        sig.declare_type(arg_code, sig.type_bank().i_type())
            .unwrap();
        let mut bank = TermBank::new(sig).unwrap();
        let head = Term::const_cell_alloc(-2);
        head.set_type(Some(arrow));
        let arg = bank.create_const_term(arg_code).unwrap();
        let app = Term::top_alloc(SIG_PHONY_APP_CODE, 2);
        app.set_argument(0, head);
        app.set_argument(1, arg);

        let shared = bank.insert_ignore_var(&app, DerefType::Never).unwrap();

        assert!(shared.query_prop(TP_HAS_APP_VAR | TP_HAS_NON_PATTERN_VAR));
        assert_eq!(shared.weight(), DEFAULT_VWEIGHT + DEFAULT_FWEIGHT);
    }

    #[test]
    fn reference_property_helpers_follow_current_top_key_behavior() {
        let (mut bank, a_code) = bank_with_symbol("a", 0);
        let mut term = bank.create_const_term(a_code).unwrap();

        bank.ref_set_prop(&mut term, TP_CHECK_FLAG).unwrap();

        assert!(term.query_prop(TP_CHECK_FLAG));
        let same = term.clone();
        bank.ref_del_prop(&mut term, TP_CHECK_FLAG).unwrap();

        assert_eq!(term, same);
        assert!(term.query_prop(TP_CHECK_FLAG));
    }

    #[test]
    fn property_count_helpers_prune_by_current_property_state() {
        let (mut bank, f_code) = bank_with_symbol("f", 1);
        let a_code = bank.signature_mut().insert_id("a", 0, false);
        let i_type = bank.signature().type_bank().i_type();
        bank.signature_mut().declare_type(a_code, i_type).unwrap();
        let a = bank.create_const_term(a_code).unwrap();
        let f = Term::top_alloc(f_code, 1);
        f.set_argument(0, a.clone());
        let shared = bank.insert(&f, DerefType::Never).unwrap();

        assert_eq!(tb_term_set_prop_count(&shared, TP_CHECK_FLAG), 2);
        assert!(shared.query_prop(TP_CHECK_FLAG));
        assert!(a.query_prop(TP_CHECK_FLAG));
        assert_eq!(tb_term_set_prop_count(&shared, TP_CHECK_FLAG), 0);
        assert_eq!(tb_term_del_prop_count(&shared, TP_CHECK_FLAG), 2);
        assert!(!shared.query_prop(TP_CHECK_FLAG));
        assert!(!a.query_prop(TP_CHECK_FLAG));
    }

    #[test]
    fn gc_mark_and_sweep_keep_roots_min_terms_and_rewrite_targets() {
        let mut sig = Signature::new(TypeBank::new());
        let keep_code = sig.insert_id("keep", 0, false);
        let dead_code = sig.insert_id("dead", 0, false);
        let repl_code = sig.insert_id("repl", 0, false);
        let i_type = sig.type_bank().i_type();
        sig.declare_type(keep_code, i_type.clone()).unwrap();
        sig.declare_type(dead_code, i_type.clone()).unwrap();
        sig.declare_type(repl_code, i_type).unwrap();
        let mut bank = TermBank::new(sig).unwrap();
        let keep = bank.create_min_term(keep_code).unwrap();
        let dead = bank.create_const_term(dead_code).unwrap();
        let replacement = bank.create_const_term(repl_code).unwrap();
        term_add_rw_link(
            &keep,
            &replacement,
            None,
            false,
            RwResultType::LimitedRewritable,
        );

        let recovered = bank.gc_sweep();

        assert_eq!(recovered, 1);
        assert_eq!(bank.recovered(), 1);
        assert_eq!(bank.garbage_state(), TP_GARBAGE_FLAG);
        assert_eq!(bank.find(&dead), None);
        assert_eq!(bank.find(&keep), Some(keep.clone()));
        assert_eq!(bank.find(&replacement), Some(replacement));
    }

    #[test]
    fn find_repr_uses_this_banks_shared_arguments() {
        let (mut bank, f_code) = bank_with_symbol("f", 1);
        let a_code = bank.signature_mut().insert_id("a", 0, false);
        let i_type = bank.signature().type_bank().i_type();
        bank.signature_mut().declare_type(a_code, i_type).unwrap();
        let a = bank.create_const_term(a_code).unwrap();
        let f = Term::top_alloc(f_code, 1);
        f.set_argument(0, a.clone());
        let shared = bank.insert(&f, DerefType::Never).unwrap();
        let external_a = Term::const_cell_alloc(a_code);
        external_a.set_type(a.type_());
        let external_f = Term::top_alloc(f_code, 1);
        external_f.set_type(shared.type_());
        external_f.set_argument(0, external_a);

        assert_eq!(bank.find_repr(&external_f), Some(shared));
    }

    #[test]
    fn collect_subterms_sets_op_flag_and_skips_existing_marks() {
        let (mut bank, f_code) = bank_with_symbol("f", 1);
        let a_code = bank.signature_mut().insert_id("a", 0, false);
        let i_type = bank.signature().type_bank().i_type();
        bank.signature_mut().declare_type(a_code, i_type).unwrap();
        let a = bank.create_const_term(a_code).unwrap();
        let f = Term::top_alloc(f_code, 1);
        f.set_argument(0, a.clone());
        let shared = bank.insert(&f, DerefType::Never).unwrap();
        let mut collector = PStack::new();

        assert_eq!(tb_term_collect_subterms(&shared, &mut collector), 2);
        assert_eq!(collector.len(), 2);
        assert!(shared.query_prop(TP_OP_FLAG));
        assert!(a.query_prop(TP_OP_FLAG));
        assert_eq!(tb_term_collect_subterms(&shared, &mut collector), 0);
    }
}
