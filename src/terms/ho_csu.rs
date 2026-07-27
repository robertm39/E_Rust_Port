use crate::basics::error::{Diagnostic, ErrorCode};
use crate::basics::pqueue::PQueue;
use crate::basics::simple_stuff::{problem_type, ProblemType};
use crate::heuristics::hcb::{HeuristicParmsCell, UnifMode};
use crate::terms::fixpoint_unif::subst_compute_fixpoint_mgu;
use crate::terms::functypes::FunCode;
use crate::terms::ho_bindings::compute_next_binding;
use crate::terms::lambda::whnf_deref;
use crate::terms::match_mgu::{subst_mgu_complete_with_bank, OracleUnifResult};
use crate::terms::pattern_match_mgu::{prune_lambda_prefix, subst_compute_mgu_pattern};
use crate::terms::subst::Substitution;
use crate::terms::termbanks::TermBank;
use crate::terms::termfunc::term_is_ground;
use crate::terms::termtypes::Term;
use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

static HO_CSU_PARAMS: RwLock<Option<HoCsuParams>> = RwLock::new(None);

pub type StateTag = u64;
pub type Limits = u64;

pub const INIT_TAG: StateTag = 0;
pub const RIGID_PROCESSED_TAG: StateTag = 1;
pub const SOLVED_BY_ORACLE_TAG: StateTag = 2;
pub const DECOMPOSED_VAR: StateTag = 3;

pub const BT_STEP_SIZE: usize = 4;
pub const BURY_KIND: i32 = 0;
pub const STORE_KIND: i32 = 1;

#[derive(Clone, Debug, Eq, PartialEq)]
struct BacktrackFrame {
    constraints: PQueue<Term>,
    state: StateTag,
    limits: Limits,
    subst_pos: usize,
}

/// C-shaped iterator for higher-order complete-set-of-unifiers enumeration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CsuIterator {
    constraints: PQueue<Term>,
    backtrack_info: Vec<BacktrackFrame>,
    current_state: StateTag,
    current_limits: Limits,
    init_pos: usize,
    unifiers_returned: i32,
    steps: i32,
}

impl CsuIterator {
    /// C `CSUIterInit`.
    #[must_use]
    pub fn new(lhs: &Term, rhs: &Term, subst: &Substitution) -> Self {
        let mut constraints = PQueue::new();
        constraints.store(rhs.clone());
        constraints.store(lhs.clone());

        Self {
            constraints,
            backtrack_info: Vec::new(),
            current_state: INIT_TAG,
            current_limits: 0,
            init_pos: subst.len(),
            unifiers_returned: 0,
            steps: 0,
        }
    }

    /// C `CSUIterGetCurrentSubst`.
    #[must_use]
    pub const fn current_subst<'subst>(&self, subst: &'subst Substitution) -> &'subst Substitution {
        subst
    }

    /// C `NextCSUElement`, using the globally initialized CSU parameters.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if the global unification limits were not
    /// initialized or if a term-bank operation needed by higher-order
    /// normalization or binding construction fails.
    pub fn next_csu_element(
        &mut self,
        bank: &mut TermBank,
        subst: &mut Substitution,
    ) -> Result<bool, Diagnostic> {
        let Some(params) = current_unif_limits() else {
            return Err(Diagnostic::new(
                ErrorCode::OTHER_ERROR,
                "higher-order CSU limits have not been initialized",
            ));
        };
        self.next_csu_element_with_params(bank, subst, &params)
    }

    /// C `NextCSUElement`, with an explicit parameter snapshot for tests and
    /// callers that already own heuristic parameters.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if weak-head normalization, lambda-prefix pruning,
    /// oracle processing, or binding construction needs to rebuild a term and
    /// term-bank insertion fails.
    pub fn next_csu_element_with_params(
        &mut self,
        bank: &mut TermBank,
        subst: &mut Substitution,
        params: &HoCsuParams,
    ) -> Result<bool, Diagnostic> {
        let mut result = self.backtrack_iter(subst, params);
        self.steps = 0;

        if result {
            if self.should_use_first_order_mgu(params) {
                let lhs = self.constraints.get_last();
                let rhs = self.constraints.get_last();
                result = subst_mgu_complete_with_bank(bank, &lhs, &rhs, subst)?;
                self.backtrack_info.clear();
                self.unifiers_returned = 1;
            } else {
                result = self.forward_iter(bank, subst, params)?;
                if result {
                    self.unifiers_returned += 1;
                }
            }
        }

        if !result {
            subst.backtrack_to_pos(self.init_pos);
        }
        Ok(result)
    }

    /// C `CSUIterDestroy`.
    pub fn destroy(self, subst: &mut Substitution) {
        subst.backtrack_to_pos(self.init_pos);
    }

    #[must_use]
    pub const fn constraint_count(&self) -> usize {
        self.constraints.cardinality()
    }

    #[must_use]
    pub fn backtrack_frame_count(&self) -> usize {
        self.backtrack_info.len()
    }

    fn should_use_first_order_mgu(&self, params: &HoCsuParams) -> bool {
        (problem_type() != ProblemType::HigherOrder || params.unif_mode == UnifMode::Single)
            && self.unifiers_returned == 0
    }

    fn forward_iter(
        &mut self,
        bank: &mut TermBank,
        subst: &mut Substitution,
        params: &HoCsuParams,
    ) -> Result<bool, Diagnostic> {
        let mut result = true;
        while result && !self.constraints.is_empty() {
            assert_eq!(
                self.constraints.cardinality() % 2,
                0,
                "CSU constraints store term pairs"
            );
            let mut lhs = self.constraints.get_last();
            let mut rhs = self.constraints.get_last();

            if params.max_unif_steps > 0 && self.steps >= params.max_unif_steps {
                result = self.backtrack_iter(subst, params);
                continue;
            }
            if lhs.type_() != rhs.type_() {
                assert!(
                    self.constraints.is_empty(),
                    "C only hits type mismatch on the initial CSU pair"
                );
                result = false;
                continue;
            }

            (lhs, rhs) = whnf_and_prune(bank, &lhs, &rhs)?;
            let subst_pos = subst.len();
            if lhs == rhs {
                continue;
            }
            if term_is_ground(&lhs) && term_is_ground(&rhs) {
                result = self.backtrack_iter(subst, params);
                continue;
            }

            if rhs.is_top_level_free_var() && !lhs.is_top_level_free_var() {
                std::mem::swap(&mut lhs, &mut rhs);
            }

            result = if lhs.is_top_level_free_var() {
                self.process_flex_pair(bank, subst, params, lhs, rhs, subst_pos)?
            } else {
                self.process_rigid_pair(bank, subst, params, &lhs, &rhs)
            };
        }
        Ok(result)
    }

    fn process_flex_pair(
        &mut self,
        bank: &mut TermBank,
        subst: &mut Substitution,
        params: &HoCsuParams,
        lhs: Term,
        rhs: Term,
        subst_pos: usize,
    ) -> Result<bool, Diagnostic> {
        let oracle = Self::try_oracles(bank, subst, params, &lhs, &rhs)?;
        match oracle {
            OracleUnifResult::Unifiable => Ok(true),
            OracleUnifResult::NotUnifiable => Ok(self.backtrack_iter(subst, params)),
            OracleUnifResult::NotInFragment => {
                self.steps += 1;
                self.process_binding_step(bank, subst, params, lhs, rhs, subst_pos)
            }
        }
    }

    fn try_oracles(
        bank: &mut TermBank,
        subst: &mut Substitution,
        params: &HoCsuParams,
        lhs: &Term,
        rhs: &Term,
    ) -> Result<OracleUnifResult, Diagnostic> {
        let mut result = OracleUnifResult::NotInFragment;
        if params.fixpoint_oracle {
            result = subst_compute_fixpoint_mgu(bank, lhs, rhs, subst)?;
        }
        if result == OracleUnifResult::NotInFragment && params.pattern_oracle {
            result = subst_compute_mgu_pattern(bank, lhs, rhs, subst)?;
        }
        Ok(result)
    }

    fn process_binding_step(
        &mut self,
        bank: &mut TermBank,
        subst: &mut Substitution,
        params: &HoCsuParams,
        lhs: Term,
        rhs: Term,
        subst_pos: usize,
    ) -> Result<bool, Diagnostic> {
        let mut next_limits = self.current_limits;
        let (next_state, moved_forward) = compute_next_binding(
            bank,
            &lhs,
            &rhs,
            self.current_state,
            &mut next_limits,
            subst,
            params,
        )?;
        if moved_forward {
            assert_ne!(next_state, self.current_state);
            assert_ne!(next_state, DECOMPOSED_VAR);
            self.prepare_backtrack(&lhs, &rhs, next_state, self.current_limits, subst_pos);
            self.current_limits = next_limits;
            self.current_state = RIGID_PROCESSED_TAG;
            self.constraints.store(rhs);
            self.constraints.store(lhs);
            return Ok(true);
        }

        if head_id(&lhs) == head_id(&rhs) {
            assert_eq!(lhs.arity(), rhs.arity());
            assert_eq!(lhs.is_phony_app(), rhs.is_phony_app());
            let size = lhs.arity().saturating_sub(1);
            self.schedule_args(&lhs, &rhs, 1, size);
            self.current_state = RIGID_PROCESSED_TAG;
            self.current_limits = next_limits;
            Ok(true)
        } else {
            Ok(self.backtrack_iter(subst, params))
        }
    }

    fn process_rigid_pair(
        &mut self,
        bank: &TermBank,
        subst: &mut Substitution,
        params: &HoCsuParams,
        lhs: &Term,
        rhs: &Term,
    ) -> bool {
        if lhs.is_phony_app() {
            return self.process_phony_pair(subst, params, lhs, rhs);
        }
        if lhs.is_db_var() {
            return if rhs.is_db_var() && lhs.f_code() == rhs.f_code() {
                true
            } else {
                self.backtrack_iter(subst, params)
            };
        }
        if rhs.is_db_var() {
            assert!(
                !lhs.is_phony_app() && !lhs.is_db_var(),
                "right DB variable branch expects rigid left side"
            );
            return self.backtrack_iter(subst, params);
        }
        if lhs.f_code() != rhs.f_code() {
            return self.backtrack_iter(subst, params);
        }

        assert_eq!(lhs.arity(), rhs.arity());
        if bank.signature().is_polymorphic(lhs.f_code())
            && lhs.arity() != 0
            && required_arg(lhs, 0).type_() != required_arg(rhs, 0).type_()
        {
            self.backtrack_iter(subst, params)
        } else {
            self.schedule_args(lhs, rhs, 0, lhs.arity());
            true
        }
    }

    fn process_phony_pair(
        &mut self,
        subst: &mut Substitution,
        params: &HoCsuParams,
        lhs: &Term,
        rhs: &Term,
    ) -> bool {
        if !rhs.is_phony_app() {
            return self.backtrack_iter(subst, params);
        }
        let lhs_head = required_arg(lhs, 0);
        let rhs_head = required_arg(rhs, 0);
        assert!(lhs_head.is_db_var());
        assert!(rhs_head.is_db_var());
        if lhs_head != rhs_head {
            return self.backtrack_iter(subst, params);
        }
        assert_eq!(lhs.arity(), rhs.arity());
        self.schedule_args(lhs, rhs, 1, lhs.arity() - 1);
        true
    }

    fn prepare_backtrack(
        &mut self,
        lhs: &Term,
        rhs: &Term,
        next_state: StateTag,
        next_limits: Limits,
        subst_pos: usize,
    ) {
        self.backtrack_info.push(BacktrackFrame {
            constraints: build_new_queue(&self.constraints, lhs, rhs),
            state: next_state,
            limits: next_limits,
            subst_pos,
        });
    }

    fn backtrack_iter(&mut self, subst: &mut Substitution, params: &HoCsuParams) -> bool {
        if self.current_state == INIT_TAG {
            assert_eq!(self.constraints.cardinality(), 2);
            self.current_state = RIGID_PROCESSED_TAG;
            return true;
        }
        if self.backtrack_info.is_empty() || self.unifiers_returned >= params.max_unifiers {
            return false;
        }

        let frame = self
            .backtrack_info
            .pop()
            .expect("non-empty backtrack stack has a frame");
        subst.backtrack_to_pos(frame.subst_pos);
        self.current_limits = frame.limits;
        self.current_state = frame.state;
        self.constraints = frame.constraints;
        true
    }

    fn schedule_args(&mut self, lhs: &Term, rhs: &Term, start: usize, size: usize) {
        let mut flex = Vec::new();
        let mut rigid_same = Vec::new();
        let mut rigid_diff = Vec::new();

        for index in start..start + size {
            let left = required_arg(lhs, index);
            let right = required_arg(rhs, index);
            let left_code = unroll_fcode(&left);
            let right_code = unroll_fcode(&right);
            if left_code < 0 || right_code < 0 {
                flex.push(right);
                flex.push(left);
            } else if left_code == right_code {
                rigid_same.push(right);
                rigid_same.push(left);
            } else {
                rigid_diff.push(right);
                rigid_diff.push(left);
            }
        }

        move_stack(&mut self.constraints, &mut rigid_same, STORE_KIND);
        move_stack(&mut self.constraints, &mut rigid_diff, STORE_KIND);
        move_stack(&mut self.constraints, &mut flex, BURY_KIND);
    }
}

fn whnf_and_prune(bank: &mut TermBank, lhs: &Term, rhs: &Term) -> Result<(Term, Term), Diagnostic> {
    let lhs = whnf_deref(bank, lhs)?;
    let rhs = whnf_deref(bank, rhs)?;
    prune_lambda_prefix(bank, lhs, rhs)
}

fn build_new_queue(old: &PQueue<Term>, lhs: &Term, rhs: &Term) -> PQueue<Term> {
    let mut result = old.clone();
    result.store(rhs.clone());
    result.store(lhs.clone());
    result
}

fn move_stack(queue: &mut PQueue<Term>, stack: &mut Vec<Term>, move_kind: i32) {
    while let Some(term) = stack.pop() {
        if move_kind == BURY_KIND {
            queue.bury(term);
        } else {
            queue.store(term);
        }
    }
}

fn unroll_fcode(term: &Term) -> FunCode {
    let mut current = term.clone();
    while current.is_lambda() {
        current = required_arg(&current, 1);
    }

    while let Some(binding) = top_level_binding(&current) {
        current = binding;
        while current.is_lambda() {
            current = required_arg(&current, 1);
        }
    }

    head_id(&current)
}

fn top_level_binding(term: &Term) -> Option<Term> {
    if term.is_applied_free_var() {
        return required_arg(term, 0).binding();
    }
    if term.is_free_var() {
        return term.binding();
    }
    None
}

fn head_id(term: &Term) -> FunCode {
    if term.is_phony_app() {
        required_arg(term, 0).f_code()
    } else {
        term.f_code()
    }
}

fn required_arg(term: &Term, index: usize) -> Term {
    term.argument(index)
        .unwrap_or_else(|| panic!("CSU term argument {index} is uninitialized"))
}

/// Mirrors C `CONSTRAINT_STATE(c)`.
#[must_use]
pub const fn constraint_state(constraint: StateTag) -> StateTag {
    constraint & 3
}

/// Mirrors C `CONSTRAINT_COUNTER(c)`.
#[must_use]
pub const fn constraint_counter(constraint: StateTag) -> StateTag {
    constraint >> 2
}

/// Mirrors C `BUILD_CONSTR(c, s)`.
#[must_use]
pub const fn build_constraint(counter: StateTag, state: StateTag) -> StateTag {
    (counter << 2) | state
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HoCsuParams {
    pub func_proj_limit: i32,
    pub imit_limit: i32,
    pub ident_limit: i32,
    pub elim_limit: i32,
    pub unif_mode: UnifMode,
    pub pattern_oracle: bool,
    pub fixpoint_oracle: bool,
    pub max_unifiers: i32,
    pub max_unif_steps: i32,
}

impl HoCsuParams {
    #[must_use]
    pub const fn from_heuristic_parms(parms: &HeuristicParmsCell) -> Self {
        Self {
            func_proj_limit: parms.func_proj_limit,
            imit_limit: parms.imit_limit,
            ident_limit: parms.ident_limit,
            elim_limit: parms.elim_limit,
            unif_mode: parms.unif_mode,
            pattern_oracle: parms.pattern_oracle,
            fixpoint_oracle: parms.fixpoint_oracle,
            max_unifiers: parms.max_unifiers,
            max_unif_steps: parms.max_unif_steps,
        }
    }
}

pub fn init_unif_limits(parms: &HeuristicParmsCell) {
    *write_params() = Some(HoCsuParams::from_heuristic_parms(parms));
}

#[must_use]
pub fn current_unif_limits() -> Option<HoCsuParams> {
    *read_params()
}

fn read_params() -> RwLockReadGuard<'static, Option<HoCsuParams>> {
    match HO_CSU_PARAMS.read() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

fn write_params() -> RwLockWriteGuard<'static, Option<HoCsuParams>> {
    match HO_CSU_PARAMS.write() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        build_constraint, constraint_counter, constraint_state, whnf_and_prune, CsuIterator,
        HoCsuParams, BT_STEP_SIZE, BURY_KIND, DECOMPOSED_VAR, INIT_TAG, RIGID_PROCESSED_TAG,
        SOLVED_BY_ORACLE_TAG, STORE_KIND,
    };
    use crate::basics::simple_stuff::{reset_problem_type, set_problem_type, ProblemType};
    use crate::heuristics::hcb::{HeuristicParmsCell, UnifMode};
    use crate::terms::lambda::{apply_terms, close_with_type_prefix};
    use crate::terms::signature::Signature;
    use crate::terms::simpletypes::{alloc_arrow_type, Type};
    use crate::terms::subst::Substitution;
    use crate::terms::termbanks::TermBank;
    use crate::terms::termtypes::Term;
    use crate::terms::typebanks::TypeBank;

    struct ProblemTypeReset;

    impl Drop for ProblemTypeReset {
        fn drop(&mut self) {
            reset_problem_type();
        }
    }

    fn set_problem_type_for_test(problem_type: ProblemType) -> ProblemTypeReset {
        reset_problem_type();
        set_problem_type(problem_type).unwrap_or_else(|err| panic!("{err}"));
        ProblemTypeReset
    }

    fn test_bank() -> TermBank {
        let mut signature = Signature::new(TypeBank::new());
        signature.insert_internal_codes().unwrap();
        TermBank::new(signature).unwrap()
    }

    fn typed_const(bank: &mut TermBank, name: &str, type_: &Type) -> Term {
        let f_code = bank.signature_mut().insert_id(name, 0, false);
        bank.signature_mut()
            .declare_final_type(f_code, type_.clone())
            .unwrap();
        bank.create_const_term(f_code).unwrap()
    }

    fn ho_params(unif_mode: UnifMode) -> HoCsuParams {
        HoCsuParams {
            func_proj_limit: 0,
            imit_limit: 0,
            ident_limit: 0,
            elim_limit: 0,
            unif_mode,
            pattern_oracle: false,
            fixpoint_oracle: false,
            max_unifiers: 4,
            max_unif_steps: 32,
        }
    }

    #[test]
    fn state_tag_values_match_c_header() {
        assert_eq!(INIT_TAG, 0);
        assert_eq!(RIGID_PROCESSED_TAG, 1);
        assert_eq!(SOLVED_BY_ORACLE_TAG, 2);
        assert_eq!(DECOMPOSED_VAR, 3);
        assert_eq!(BT_STEP_SIZE, 4);
        assert_eq!(BURY_KIND, 0);
        assert_eq!(STORE_KIND, 1);
    }

    #[test]
    fn constraint_bit_packing_matches_c_macros() {
        let encoded = build_constraint(17, DECOMPOSED_VAR);
        assert_eq!(encoded, (17 << 2) | 3);
        assert_eq!(constraint_state(encoded), DECOMPOSED_VAR);
        assert_eq!(constraint_counter(encoded), 17);
    }

    #[test]
    fn constraint_build_does_not_mask_state_like_c_macro() {
        let encoded = build_constraint(0, 4);
        assert_eq!(encoded, 4);
        assert_eq!(constraint_state(encoded), INIT_TAG);
        assert_eq!(constraint_counter(encoded), 1);
    }

    #[test]
    fn ho_csu_params_snapshot_keeps_fields_read_by_c_csu_helpers() {
        let parms = HeuristicParmsCell {
            func_proj_limit: 1,
            imit_limit: 2,
            ident_limit: 3,
            elim_limit: 4,
            unif_mode: UnifMode::Multi,
            pattern_oracle: false,
            fixpoint_oracle: false,
            max_unifiers: 8,
            max_unif_steps: 9,
            ..HeuristicParmsCell::default()
        };

        assert_eq!(
            HoCsuParams::from_heuristic_parms(&parms),
            HoCsuParams {
                func_proj_limit: 1,
                imit_limit: 2,
                ident_limit: 3,
                elim_limit: 4,
                unif_mode: UnifMode::Multi,
                pattern_oracle: false,
                fixpoint_oracle: false,
                max_unifiers: 8,
                max_unif_steps: 9,
            }
        );
    }

    #[test]
    fn iterator_uses_complete_mgu_shortcut_for_first_order_problem() {
        let _problem_type = set_problem_type_for_test(ProblemType::FirstOrder);
        let mut bank = test_bank();
        let type_ = bank.signature().type_bank().default_type();
        let variable = bank.vars().get_fresh_var(&type_);
        let constant = typed_const(&mut bank, "csu_fo_a", &type_);
        let mut subst = Substitution::new();
        let mut iter = CsuIterator::new(&variable, &constant, &subst);

        assert!(iter
            .next_csu_element_with_params(&mut bank, &mut subst, &ho_params(UnifMode::Multi))
            .unwrap());
        assert_eq!(variable.binding(), Some(constant));
        assert_eq!(iter.constraint_count(), 0);

        assert!(!iter
            .next_csu_element_with_params(&mut bank, &mut subst, &ho_params(UnifMode::Multi))
            .unwrap());
        assert!(variable.binding().is_none());
    }

    #[test]
    fn iterator_uses_higher_order_complete_mgu_for_single_mode() {
        let _problem_type = set_problem_type_for_test(ProblemType::HigherOrder);
        let mut bank = test_bank();
        let type_ = bank.signature().type_bank().default_type();
        let left_arg = typed_const(&mut bank, "csu_ho_left_arg", &type_);
        let right_arg = typed_const(&mut bank, "csu_ho_right_arg", &type_);
        let f_code = bank.signature_mut().insert_id("csu_ho_same_head", 2, false);

        let left = Term::top_alloc(f_code, 2);
        left.set_type(Some(type_.clone()));
        left.set_argument(0, left_arg.clone());
        left.set_argument(1, right_arg.clone());
        let left = bank.term_top_insert(left).unwrap();

        let right = Term::top_alloc(f_code, 1);
        right.set_type(Some(type_));
        right.set_argument(0, left_arg);
        let right = bank.term_top_insert(right).unwrap();

        let mut subst = Substitution::new();
        let mut iter = CsuIterator::new(&left, &right, &subst);

        assert!(!iter
            .next_csu_element_with_params(&mut bank, &mut subst, &ho_params(UnifMode::Single))
            .unwrap());
        assert!(subst.is_empty());
    }

    #[test]
    fn iterator_uses_binding_dispatcher_for_higher_order_projection() {
        let _problem_type = set_problem_type_for_test(ProblemType::HigherOrder);
        let mut bank = test_bank();
        let type_ = bank.signature().type_bank().default_type();
        let constant = typed_const(&mut bank, "csu_ho_a", &type_);
        let flex_type = bank
            .signature_mut()
            .type_bank_mut()
            .insert_type_shared(alloc_arrow_type(vec![type_.clone(), type_]));
        let flex = bank.vars().get_fresh_var(&flex_type);
        let applied = apply_terms(&mut bank, &flex, std::slice::from_ref(&constant)).unwrap();
        let mut subst = Substitution::new();
        let mut iter = CsuIterator::new(&applied, &constant, &subst);

        assert!(iter
            .next_csu_element_with_params(&mut bank, &mut subst, &ho_params(UnifMode::Multi))
            .unwrap());
        assert!(flex.binding().is_some_and(|binding| binding.is_lambda()));
        assert_eq!(iter.constraint_count(), 0);
        assert_eq!(iter.backtrack_frame_count(), 1);

        assert!(!iter
            .next_csu_element_with_params(&mut bank, &mut subst, &ho_params(UnifMode::Multi))
            .unwrap());
        assert!(flex.binding().is_none());
    }

    #[test]
    fn whnf_and_prune_eta_expands_non_lambda_side() {
        let _problem_type = set_problem_type_for_test(ProblemType::HigherOrder);
        let mut bank = test_bank();
        let type_ = bank.signature().type_bank().default_type();
        let arrow = bank
            .signature_mut()
            .type_bank_mut()
            .insert_type_shared(alloc_arrow_type(vec![type_.clone(), type_.clone()]));
        let function = typed_const(&mut bank, "csu_prune_f", &arrow);
        let db0 = bank.request_db_var(&type_, 0);
        let lambda = close_with_type_prefix(&mut bank, std::slice::from_ref(&type_), &db0).unwrap();

        let (left, right) = whnf_and_prune(&mut bank, &lambda, &function).unwrap();

        assert!(left.is_db_var());
        assert_eq!(left.f_code(), 0);
        assert_eq!(right.f_code(), function.f_code());
        assert_eq!(right.argument(0).as_ref(), Some(&left));
    }
}
