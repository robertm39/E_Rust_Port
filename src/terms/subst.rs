use crate::basics::error::Diagnostic;
use crate::basics::simple_stuff::{problem_type, ProblemType};
use crate::terms::lambda::whnf_deref;
use crate::terms::signature::Signature;
use crate::terms::termbanks::TermBank;
use crate::terms::termfunc::term_create_prefix;
use crate::terms::termtypes::{
    term_deref, BorrowedTermCell, DerefType, Term, TP_OP_FLAG, TP_PRED_POS, TP_SPECIAL_FLAG,
};
use crate::terms::termvars::VarBank;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Substitution {
    bindings: Vec<Term>,
    norm_stack: Vec<BorrowedTermCell>,
    owned_norm_stack: Vec<Term>,
}

struct BorrowedNormStack<'a> {
    stack: &'a mut Vec<BorrowedTermCell>,
}

impl<'a> BorrowedNormStack<'a> {
    fn new(stack: &'a mut Vec<BorrowedTermCell>) -> Self {
        Self { stack }
    }

    fn push(&mut self, term: BorrowedTermCell) {
        self.stack.push(term);
    }

    fn pop(&mut self) -> Option<BorrowedTermCell> {
        self.stack.pop()
    }

    fn as_mut(&mut self) -> &mut Vec<BorrowedTermCell> {
        self.stack
    }
}

impl Drop for BorrowedNormStack<'_> {
    fn drop(&mut self) {
        self.stack.clear();
    }
}

struct OwnedNormStack<'a> {
    stack: &'a mut Vec<Term>,
}

impl<'a> OwnedNormStack<'a> {
    fn new(stack: &'a mut Vec<Term>) -> Self {
        Self { stack }
    }

    fn push(&mut self, term: Term) {
        self.stack.push(term);
    }

    fn pop(&mut self) -> Option<Term> {
        self.stack.pop()
    }
}

impl Drop for OwnedNormStack<'_> {
    fn drop(&mut self) {
        self.stack.clear();
    }
}

impl Substitution {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            bindings: Vec::new(),
            norm_stack: Vec::new(),
            owned_norm_stack: Vec::new(),
        }
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.bindings.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.bindings.is_empty()
    }

    #[must_use]
    pub fn bindings(&self) -> &[Term] {
        &self.bindings
    }

    /// Adds a free-variable binding and returns the previous stack position.
    ///
    /// # Panics
    ///
    /// Debug builds panic if `var` is not a free variable, is already bound,
    /// or has a different type from `bind`, matching the C `SubstAddBinding`
    /// assertions when `NDEBUG` is not defined.
    pub fn add_binding(&mut self, var: &Term, bind: &Term) -> usize {
        self.add_owned_binding(var.clone(), bind)
    }

    fn add_owned_binding(&mut self, var: Term, bind: &Term) -> usize {
        Self::add_owned_binding_to(&mut self.bindings, var, bind)
    }

    fn add_owned_binding_to(bindings: &mut Vec<Term>, var: Term, bind: &Term) -> usize {
        let previous = bindings.len();
        debug_assert!(var.is_free_var(), "only free variables can be bound");
        debug_assert!(var.binding().is_none(), "variable is already bound");
        debug_assert_eq!(var.type_(), bind.type_(), "binding type mismatch");

        var.set_binding(Some(bind.clone()));
        bindings.push(var);
        previous
    }

    /// Removes the newest binding, if one exists.
    ///
    /// # Panics
    ///
    /// Panics if the substitution stack contains something other than a free
    /// variable, matching the C representation invariant.
    pub fn backtrack_single(&mut self) -> bool {
        let Some(var) = self.bindings.pop() else {
            return false;
        };
        debug_assert!(
            var.is_free_var(),
            "substitution stack stores free variables"
        );
        var.set_binding(None);
        true
    }

    pub fn backtrack_to_pos(&mut self, pos: usize) -> usize {
        let mut count = 0;
        while self.len() > pos {
            self.backtrack_single();
            count += 1;
        }
        count
    }

    pub fn backtrack(&mut self) -> usize {
        let mut count = 0;
        while self.backtrack_single() {
            count += 1;
        }
        count
    }

    pub fn delete(mut self) {
        self.backtrack();
    }

    /// Instantiates unbound variables in `term` with fresh variables.
    ///
    /// # Panics
    ///
    /// Panics if a reachable free variable has no type, following the C
    /// precondition for `VarBankGetFreshVar`.
    #[allow(
        unsafe_code,
        reason = "measured private traversal over stable Rc term allocations"
    )]
    pub fn norm_term(&mut self, term: &Term, vars: &VarBank) -> usize {
        let previous = self.len();
        debug_assert!(
            self.norm_stack.is_empty(),
            "normalization scratch must be empty"
        );
        let bindings = &mut self.bindings;
        let mut expansion_roots = Vec::new();
        let mut norm_stack = BorrowedNormStack::new(&mut self.norm_stack);
        norm_stack.push(term.borrowed_cell());
        while let Some(candidate) = norm_stack.pop() {
            // SAFETY: `term` remains borrowed for the complete traversal and
            // owns every structural argument reachable from the initial
            // cursor. Normalization never replaces argument slots or removes
            // bindings: it only reads the graph, sets scalar property bits,
            // and changes a currently empty variable binding to `Some`.
            // Existing and newly installed bindings therefore retain every
            // followed target. Applied-variable expansion owners stay in
            // `expansion_roots` until the raw stack is empty. `TermCell` uses
            // interior mutability, so no mutable reference aliases these
            // shared reads, and all pointers preserve `Rc::as_ptr` provenance,
            // alignment, and initialization.
            unsafe {
                let current = candidate.deref_always(&mut expansion_roots);
                if current.is_free_var() {
                    if current.query_prop(TP_SPECIAL_FLAG) {
                        continue;
                    }
                    let type_ = current.type_().expect("free variable must have a type");
                    let new_var = vars.get_fresh_var(&type_);
                    new_var.set_prop(TP_SPECIAL_FLAG);
                    Self::add_owned_binding_to(bindings, current.to_owned(), &new_var);
                } else {
                    current.push_arguments_reversed(norm_stack.as_mut());
                }
            }
        }
        previous
    }

    /// Instantiates unbound variables after applying the C problem-specific
    /// root dereference policy.
    ///
    /// Higher-order traversal uses `WHNF_deref`; first-order traversal uses
    /// `TermDerefAlways`. The term bank and problem type are explicit instead
    /// of reproducing C's unused signature parameter and process-global policy
    /// inside `SubstNormTerm`.
    ///
    /// # Errors
    ///
    /// Returns diagnostics from higher-order weak-head normalization.
    ///
    /// # Panics
    ///
    /// Panics if a reachable free variable has no type, following the C
    /// precondition for `VarBankGetFreshVar`.
    pub fn norm_term_with_bank(
        &mut self,
        term: &Term,
        vars: &VarBank,
        bank: &mut TermBank,
        problem_type: ProblemType,
    ) -> Result<usize, Diagnostic> {
        if problem_type != ProblemType::HigherOrder {
            return Ok(self.norm_term(term, vars));
        }

        let previous = self.len();
        debug_assert!(
            self.owned_norm_stack.is_empty(),
            "higher-order normalization scratch must be empty"
        );
        let normalized = {
            let bindings = &mut self.bindings;
            let mut norm_stack = OwnedNormStack::new(&mut self.owned_norm_stack);
            norm_stack.push(term.clone());
            (|| {
                while let Some(candidate) = norm_stack.pop() {
                    let current = whnf_deref(bank, &candidate)?;
                    if current.is_free_var() {
                        if current.query_prop(TP_SPECIAL_FLAG) {
                            continue;
                        }
                        let type_ = current.type_().expect("free variable must have a type");
                        let new_var = vars.get_fresh_var(&type_);
                        new_var.set_prop(TP_SPECIAL_FLAG);
                        Self::add_owned_binding_to(bindings, current, &new_var);
                    } else {
                        for index in (0..current.arity()).rev() {
                            norm_stack.push(current.argument(index).unwrap_or_else(|| {
                                panic!("term argument {index} is uninitialized")
                            }));
                        }
                    }
                }
                Ok(())
            })()
        };
        if let Err(error) = normalized {
            self.backtrack_to_pos(previous);
            return Err(error);
        }
        Ok(previous)
    }

    /// Returns whether this substitution is a one-step variable renaming.
    ///
    /// # Panics
    ///
    /// Panics if the substitution stack does not contain bound free variables,
    /// matching the C assertions.
    #[must_use]
    pub fn is_renaming(&self) -> bool {
        for var in &self.bindings {
            assert!(
                var.is_free_var(),
                "substitution stack stores free variables"
            );
            assert!(var.binding().is_some(), "substitution variables are bound");
            let mut deref = DerefType::Once;
            let inst = term_deref(var, &mut deref);
            if !inst.is_free_var() {
                return false;
            }
            inst.del_prop(TP_OP_FLAG);
        }

        for var in &self.bindings {
            let mut deref = DerefType::Once;
            let inst = term_deref(var, &mut deref);
            if inst.query_prop(TP_OP_FLAG) {
                return false;
            }
            inst.set_prop(TP_OP_FLAG);
        }
        true
    }

    /// Backtracks skolem bindings.
    ///
    /// # Panics
    ///
    /// Panics if a recorded variable is not currently bound, matching the C
    /// skolem substitution invariant.
    pub fn backtrack_skolem(&mut self) {
        while let Some(var) = self.bindings.pop() {
            assert!(
                var.binding().is_some(),
                "skolem substitution variable is bound"
            );
            var.set_binding(None);
        }
    }

    pub fn skolemize_term(&mut self, term: &Term, sig: &mut Signature) {
        if term.is_free_var() {
            if term.binding().is_none() {
                let skolem = Term::const_cell_alloc(sig.get_new_skolem_code(0));
                term.set_binding(Some(skolem));
                self.bindings.push(term.clone());
            }
        } else {
            for arg in term.argument_clones().into_iter().flatten() {
                self.skolemize_term(&arg, sig);
            }
        }
    }

    pub fn complete_instance(&mut self, term: &Term, default_binding: &Term) {
        if term.is_free_var() {
            if term.binding().is_none() {
                self.add_binding(term, default_binding);
            }
        } else {
            for arg in term.argument_clones().into_iter().flatten() {
                self.complete_instance(&arg, default_binding);
            }
        }
    }

    /// Binds a variable to a prefix of another term.
    ///
    /// # Panics
    ///
    /// Panics if `var` is not an unbound free variable, the terms have missing
    /// types, or the first-order predicate-position assertion is violated.
    pub fn bind_app_var(
        &mut self,
        var: &Term,
        to_bind: &Term,
        up_to: usize,
        bank: &mut TermBank,
        problem_type: ProblemType,
    ) -> Result<usize, Diagnostic> {
        let previous = self.len();
        assert!(
            var.is_free_var(),
            "app-var binding target must be a free variable"
        );
        assert!(var.binding().is_none(), "variable is already bound");
        assert!(
            problem_type == ProblemType::HigherOrder || !to_bind.query_prop(TP_PRED_POS),
            "first-order app-var binding cannot bind predicate-position terms"
        );
        let var_type = var.type_().expect("variable must have a type");
        assert!(to_bind.type_().is_some(), "term to bind must have a type");

        let prefix = term_create_prefix(to_bind, up_to);
        prefix.set_type(Some(var_type));
        let binding = if prefix.is_shared() {
            prefix
        } else {
            bank.term_top_insert(prefix)?
        };
        var.set_binding(Some(binding));
        self.bindings.push(var.clone());
        Ok(previous)
    }

    #[must_use]
    pub fn has_ho_binding(&self) -> bool {
        self.has_ho_binding_for_problem(problem_type())
    }

    #[must_use]
    pub fn has_ho_binding_for_problem(&self, problem_type: ProblemType) -> bool {
        problem_type == ProblemType::HigherOrder
            && self
                .bindings
                .iter()
                .any(|var| var.type_().is_some_and(|type_| type_.is_arrow()))
    }
}

#[cfg(test)]
mod tests {
    use super::Substitution;
    use crate::basics::simple_stuff::ProblemType;
    use crate::terms::lambda::{apply_terms, close_with_type_prefix};
    use crate::terms::signature::{Signature, SIG_PHONY_APP_CODE};
    use crate::terms::simpletypes::alloc_arrow_type;
    use crate::terms::termbanks::TermBank;
    use crate::terms::termtypes::{DerefType, Term, TP_OP_FLAG, TP_SPECIAL_FLAG};
    use crate::terms::termvars::VarBank;
    use crate::terms::typebanks::TypeBank;

    fn typed_var(code: i64, bank: &TypeBank) -> Term {
        let var = Term::const_cell_alloc(code);
        var.set_type(Some(bank.i_type()));
        var
    }

    #[test]
    fn add_binding_and_backtracking_follow_stack_positions() {
        let type_bank = TypeBank::new();
        let x = typed_var(-2, &type_bank);
        let y = typed_var(-4, &type_bank);
        let a = Term::const_cell_alloc(10);
        a.set_type(Some(type_bank.i_type()));
        let b = Term::const_cell_alloc(11);
        b.set_type(Some(type_bank.i_type()));
        let mut subst = Substitution::new();

        let start = subst.add_binding(&x, &a);
        assert_eq!(start, 0);
        let after_first = subst.add_binding(&y, &b);
        assert_eq!(after_first, 1);
        assert_eq!(subst.len(), 2);
        assert_eq!(x.binding(), Some(a));
        assert_eq!(y.binding(), Some(b));

        assert_eq!(subst.backtrack_to_pos(after_first), 1);
        assert!(x.binding().is_some());
        assert!(y.binding().is_none());
        assert!(subst.backtrack_single());
        assert!(x.binding().is_none());
        assert!(!subst.backtrack_single());
    }

    #[test]
    fn norm_term_binds_each_unmarked_free_variable_to_fresh_marked_vars() {
        let type_bank = TypeBank::new();
        let vars = VarBank::new(&type_bank);
        let root = Term::top_alloc(20, 3);
        let x = typed_var(-2, &type_bank);
        let y = typed_var(-4, &type_bank);
        root.set_argument(0, x.clone());
        root.set_argument(1, x.clone());
        root.set_argument(2, y.clone());
        let mut subst = Substitution::new();

        let pos = subst.norm_term(&root, &vars);

        assert_eq!(pos, 0);
        assert_eq!(subst.len(), 2);
        assert_eq!(subst.bindings(), &[x.clone(), y.clone()]);
        assert!(x.binding().unwrap().query_prop(TP_SPECIAL_FLAG));
        assert!(y.binding().unwrap().query_prop(TP_SPECIAL_FLAG));
        assert_eq!(subst.backtrack(), 2);
        assert!(x.binding().is_none());
        assert!(y.binding().is_none());
    }

    #[test]
    fn norm_term_preserves_order_through_inline_argument_shapes() {
        let type_bank = TypeBank::new();
        let vars = VarBank::new(&type_bank);
        let x = typed_var(-2, &type_bank);
        let y = typed_var(-4, &type_bank);
        let unary = Term::top_alloc(20, 1);
        unary.set_argument(0, x.clone());
        let root = Term::top_alloc(21, 2);
        root.set_argument(0, unary);
        root.set_argument(1, y.clone());
        let mut subst = Substitution::new();

        assert_eq!(subst.norm_term(&root, &vars), 0);
        assert_eq!(subst.bindings(), &[x.clone(), y.clone()]);
        assert_eq!(subst.backtrack(), 2);
        assert!(x.binding().is_none());
        assert!(y.binding().is_none());
    }

    #[test]
    fn norm_term_follows_existing_binding_chain_before_freshening() {
        let type_bank = TypeBank::new();
        let vars = VarBank::new(&type_bank);
        let x = typed_var(-2, &type_bank);
        let y = typed_var(-4, &type_bank);
        let mut subst = Substitution::new();
        subst.add_binding(&x, &y);

        let pos = subst.norm_term(&x, &vars);

        assert_eq!(pos, 1);
        assert_eq!(subst.bindings(), &[x.clone(), y.clone()]);
        assert!(y.binding().unwrap().query_prop(TP_SPECIAL_FLAG));
        assert_eq!(subst.backtrack_to_pos(pos), 1);
        assert_eq!(x.binding(), Some(y.clone()));
        assert!(y.binding().is_none());
        assert_eq!(subst.backtrack(), 1);
        assert!(x.binding().is_none());
    }

    #[test]
    fn norm_term_keeps_applied_variable_expansion_owned_during_traversal() {
        let type_bank = TypeBank::new();
        let vars = VarBank::new(&type_bank);
        let individual = type_bank.i_type();
        let function_type = alloc_arrow_type(vec![individual.clone(), individual.clone()]);
        let head = Term::const_cell_alloc(-2);
        head.set_type(Some(function_type.clone()));
        let rigid = Term::const_cell_alloc(20);
        rigid.set_type(Some(function_type));
        let argument = typed_var(-4, &type_bank);
        let application = Term::top_alloc(SIG_PHONY_APP_CODE, 2);
        application.set_type(Some(individual));
        application.set_argument(0, head.clone());
        application.set_argument(1, argument.clone());
        let mut subst = Substitution::new();
        subst.add_binding(&head, &rigid);

        let pos = subst.norm_term(&application, &vars);

        assert_eq!(pos, 1);
        assert_eq!(subst.bindings(), &[head.clone(), argument.clone()]);
        assert!(argument.binding().unwrap().query_prop(TP_SPECIAL_FLAG));
        assert_eq!(subst.backtrack_to_pos(pos), 1);
        assert!(argument.binding().is_none());
        assert_eq!(head.binding(), Some(rigid));
        assert_eq!(subst.backtrack(), 1);
        assert!(head.binding().is_none());
    }

    #[test]
    fn norm_term_with_bank_weak_head_normalizes_higher_order_roots() {
        let mut sig = Signature::new(TypeBank::new());
        sig.insert_internal_codes().unwrap();
        let individual = sig.type_bank().i_type();
        let rigid_code = sig.insert_id("subst_norm_whnf_rigid", 0, false);
        sig.declare_type(rigid_code, individual.clone()).unwrap();
        let mut bank = TermBank::new(sig).unwrap();
        let rigid = bank.create_const_term(rigid_code).unwrap();
        let lambda =
            close_with_type_prefix(&mut bank, std::slice::from_ref(&individual), &rigid).unwrap();
        let arrow_type = lambda.type_().expect("closed lambda must have a type");
        let head = Term::const_cell_alloc(-2);
        head.set_type(Some(arrow_type));
        head.set_binding(Some(lambda));
        let discarded = Term::const_cell_alloc(-4);
        discarded.set_type(Some(individual));
        let application = apply_terms(&mut bank, &head, std::slice::from_ref(&discarded)).unwrap();
        let fresh = VarBank::new(bank.signature().type_bank());
        let mut subst = Substitution::new();

        let ordinary_pos = subst
            .norm_term_with_bank(&application, &fresh, &mut bank, ProblemType::NotInitialized)
            .unwrap();

        assert_eq!(ordinary_pos, 0);
        assert_eq!(subst.len(), 1);
        assert!(discarded.binding().is_some());
        subst.backtrack();

        let pos = subst
            .norm_term_with_bank(&application, &fresh, &mut bank, ProblemType::HigherOrder)
            .unwrap();

        assert_eq!(pos, 0);
        assert!(subst.is_empty());
        assert!(discarded.binding().is_none());
    }

    #[test]
    fn norm_term_clears_borrowed_scratch_when_a_panic_is_caught() {
        let type_bank = TypeBank::new();
        let vars = VarBank::new(&type_bank);
        let untyped = Term::const_cell_alloc(-2);
        let trailing = typed_var(-4, &type_bank);
        let invalid = Term::top_alloc(20, 2);
        invalid.set_argument(0, untyped);
        invalid.set_argument(1, trailing);
        let mut subst = Substitution::new();

        let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            subst.norm_term(&invalid, &vars);
        }));

        assert!(panic.is_err());
        assert!(subst.norm_stack.is_empty());

        let valid = typed_var(-6, &type_bank);
        assert_eq!(subst.norm_term(&valid, &vars), 0);
        assert_eq!(subst.bindings(), std::slice::from_ref(&valid));
        assert_eq!(subst.backtrack(), 1);
        assert!(valid.binding().is_none());
    }

    #[test]
    fn renaming_check_accepts_distinct_variable_bindings_only() {
        let type_bank = TypeBank::new();
        let x = typed_var(-2, &type_bank);
        let y = typed_var(-4, &type_bank);
        let a = typed_var(-6, &type_bank);
        let b = typed_var(-8, &type_bank);
        let mut subst = Substitution::new();

        subst.add_binding(&x, &a);
        subst.add_binding(&y, &b);
        assert!(subst.is_renaming());
        assert!(a.query_prop(TP_OP_FLAG));
        assert!(b.query_prop(TP_OP_FLAG));

        subst.backtrack();
        let non_var = Term::const_cell_alloc(10);
        non_var.set_type(Some(type_bank.i_type()));
        subst.add_binding(&x, &non_var);
        assert!(!subst.is_renaming());
    }

    #[test]
    fn renaming_check_rejects_duplicate_target_variables() {
        let type_bank = TypeBank::new();
        let x = typed_var(-2, &type_bank);
        let y = typed_var(-4, &type_bank);
        let target = typed_var(-6, &type_bank);
        let mut subst = Substitution::new();

        subst.add_binding(&x, &target);
        subst.add_binding(&y, &target);

        assert!(!subst.is_renaming());
    }

    #[test]
    fn complete_instance_binds_unbound_variables_to_default_binding() {
        let type_bank = TypeBank::new();
        let root = Term::top_alloc(30, 2);
        let x = typed_var(-2, &type_bank);
        let y = typed_var(-4, &type_bank);
        root.set_argument(0, x.clone());
        root.set_argument(1, y.clone());
        let default = Term::const_cell_alloc(12);
        default.set_type(Some(type_bank.i_type()));
        let mut subst = Substitution::new();

        subst.complete_instance(&root, &default);

        assert_eq!(subst.len(), 2);
        assert_eq!(x.binding(), Some(default.clone()));
        assert_eq!(y.binding(), Some(default));
    }

    #[test]
    fn skolemize_term_binds_unbound_variables_and_backtracks_them() {
        let type_bank = TypeBank::new();
        let mut sig = Signature::new(type_bank.clone());
        let root = Term::top_alloc(30, 1);
        let x = typed_var(-2, &type_bank);
        root.set_argument(0, x.clone());
        let mut subst = Substitution::new();

        subst.skolemize_term(&root, &mut sig);

        let binding = x.binding().unwrap();
        assert!(binding.f_code() > 0);
        assert_eq!(sig.find_name(binding.f_code()), Some("esk1_0"));
        subst.backtrack_skolem();
        assert!(x.binding().is_none());
    }

    #[test]
    fn bind_app_var_binds_to_prefix_and_records_position() {
        let mut sig = Signature::new(TypeBank::new());
        let i_type = sig.type_bank().i_type();
        let f_code = sig.insert_id("subst_prefix_f", 2, false);
        sig.declare_type(
            f_code,
            alloc_arrow_type(vec![i_type.clone(), i_type.clone(), i_type.clone()]),
        )
        .unwrap();
        let a_code = sig.insert_id("subst_prefix_a", 0, false);
        let b_code = sig.insert_id("subst_prefix_b", 0, false);
        sig.declare_type(a_code, i_type.clone()).unwrap();
        sig.declare_type(b_code, i_type.clone()).unwrap();
        let mut bank = TermBank::new(sig).unwrap();
        let var = Term::const_cell_alloc(-2);
        var.set_type(Some(i_type.clone()));
        let a = bank.create_const_term(a_code).unwrap();
        let b = bank.create_const_term(b_code).unwrap();
        let term = Term::top_alloc(f_code, 2);
        term.set_type(Some(i_type.clone()));
        term.set_argument(0, a.clone());
        term.set_argument(1, b);
        let shared_term = bank.insert_ignore_var(&term, DerefType::Never).unwrap();
        let mut subst = Substitution::new();

        let pos = subst
            .bind_app_var(&var, &shared_term, 1, &mut bank, ProblemType::HigherOrder)
            .unwrap();

        assert_eq!(pos, 0);
        let binding = var.binding().unwrap();
        assert_eq!(binding.f_code(), f_code);
        assert_eq!(binding.arity(), 1);
        assert_eq!(binding.argument(0), Some(a));
        assert_eq!(binding.type_(), Some(i_type));
        assert!(binding.is_shared());
        assert_eq!(bank.find(&binding), Some(binding));
    }

    #[test]
    fn bind_app_var_reuses_shared_full_prefix() {
        let mut sig = Signature::new(TypeBank::new());
        let i_type = sig.type_bank().i_type();
        let f_code = sig.insert_id("subst_full_f", 1, false);
        sig.declare_type(
            f_code,
            alloc_arrow_type(vec![i_type.clone(), i_type.clone()]),
        )
        .unwrap();
        let a_code = sig.insert_id("subst_full_a", 0, false);
        sig.declare_type(a_code, i_type.clone()).unwrap();
        let mut bank = TermBank::new(sig).unwrap();
        let a = bank.create_const_term(a_code).unwrap();
        let term = Term::top_alloc(f_code, 1);
        term.set_type(Some(i_type.clone()));
        term.set_argument(0, a);
        let shared_term = bank.insert_ignore_var(&term, DerefType::Never).unwrap();
        let var = Term::const_cell_alloc(-2);
        var.set_type(Some(i_type));
        let before = bank.term_nodes();
        let mut subst = Substitution::new();

        subst
            .bind_app_var(&var, &shared_term, 1, &mut bank, ProblemType::HigherOrder)
            .unwrap();

        assert_eq!(var.binding(), Some(shared_term));
        assert_eq!(bank.term_nodes(), before);
    }

    #[test]
    fn ho_binding_detection_is_problem_type_gated() {
        let mut type_bank = TypeBank::new();
        let arrow =
            type_bank.insert_type_shared(crate::terms::simpletypes::alloc_arrow_type(vec![
                type_bank.i_type(),
                type_bank.bool_type(),
            ]));
        let var = Term::const_cell_alloc(-2);
        var.set_type(Some(arrow.clone()));
        let bind = Term::const_cell_alloc(-4);
        bind.set_type(Some(arrow));
        let mut subst = Substitution::new();
        subst.add_binding(&var, &bind);

        assert!(!subst.has_ho_binding_for_problem(ProblemType::FirstOrder));
        assert!(subst.has_ho_binding_for_problem(ProblemType::HigherOrder));
    }
}
