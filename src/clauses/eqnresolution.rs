use crate::basics::error::Diagnostic;
use crate::basics::simple_stuff::{problem_type, ProblemType};
use crate::clauses::clause::Clause;
use crate::clauses::clause_props::{CP_IS_SOS, CP_NO_GENERATION};
use crate::clauses::clausesets::ClauseSet;
use crate::clauses::derivation::{clause_push_derivation, set_is_ho, DC_DES_EQ_RES, DC_EQ_RES};
use crate::clauses::eqn::Eqn;
use crate::clauses::inferencedoc::{
    ClauseCreationInference, ClauseCreationParents, ClauseModificationInference, ProofDocSession,
};
use crate::terms::ho_csu::CsuIterator;
use crate::terms::match_mgu::subst_mgu_complete_with_bank;
use crate::terms::subst::Substitution;
use crate::terms::termbanks::TermBank;
use crate::terms::termvars::VarBank;
use std::{collections::BTreeMap, fmt};

/// C `EqResOnMaximalLiteralsOnly` default.
pub const EQ_RES_ON_MAXIMAL_LITERALS_ONLY: bool = true;

/// Builds the single-clause C `ComputeEqRes` result.
///
/// This is the `res_cls == NULL` C branch: it uses the complete-MGU wrapper once
/// and does not enumerate higher-order CSU alternatives.
///
/// # Errors
///
/// Returns a diagnostic if term-bank insertion fails while copying the
/// resolvent.
///
/// # Panics
///
/// Panics if `literal_index` does not select a literal or if the selected
/// literal is positive, matching C's internal assertions.
pub fn compute_eq_res(
    bank: &mut TermBank,
    clause: &Clause,
    literal_index: usize,
) -> Result<Option<Clause>, Diagnostic> {
    let freshvars = fresh_var_bank_for_clause(bank, clause);
    let (resolvent, _) =
        compute_eq_res_with_ho_flag(bank, clause, literal_index, &freshvars, false)?;
    Ok(resolvent)
}

fn compute_eq_res_with_ho_flag(
    bank: &mut TermBank,
    clause: &Clause,
    literal_index: usize,
    freshvars: &VarBank,
    reset_freshvars: bool,
) -> Result<(Option<Clause>, bool), Diagnostic> {
    if reset_freshvars {
        freshvars.reset_v_counts();
    }
    let literal = eq_res_literal(clause, literal_index);
    let mut subst = Substitution::new();
    if !subst_mgu_complete_with_bank(bank, literal.left(), literal.right(), &mut subst)? {
        return Ok((None, false));
    }

    let subst_is_ho = subst.has_ho_binding();
    let resolvent = match build_resolvent(bank, clause, literal_index, freshvars, &mut subst) {
        Ok(resolvent) => resolvent,
        Err(err) => {
            subst.backtrack();
            return Err(err);
        }
    };
    subst.backtrack();
    Ok((Some(resolvent), subst_is_ho))
}

/// Builds the single-clause C `ComputeEqRes` result with a caller-owned
/// `freshvars` bank.
///
/// This mirrors C's proof-state-owned `freshvars` path by resetting variable
/// counts before the inference.
///
/// # Errors
///
/// Returns a diagnostic if term-bank insertion fails while copying the
/// resolvent.
pub fn compute_eq_res_with_fresh_vars(
    bank: &mut TermBank,
    clause: &Clause,
    literal_index: usize,
    freshvars: &VarBank,
) -> Result<Option<Clause>, Diagnostic> {
    let (resolvent, _) = compute_eq_res_with_ho_flag(bank, clause, literal_index, freshvars, true)?;
    Ok(resolvent)
}

fn compute_eq_res_csu_resolvents(
    bank: &mut TermBank,
    clause: &Clause,
    literal_index: usize,
    freshvars: &VarBank,
    reset_freshvars: bool,
) -> Result<(Vec<Clause>, bool), Diagnostic> {
    if reset_freshvars {
        freshvars.reset_v_counts();
    }
    let literal = eq_res_literal(clause, literal_index);
    let mut subst = Substitution::new();
    let mut iter = CsuIterator::new(literal.left(), literal.right(), &subst);
    let mut resolvents = Vec::new();
    let mut subst_is_ho = false;

    loop {
        let has_next = match iter.next_csu_element(bank, &mut subst) {
            Ok(has_next) => has_next,
            Err(err) => {
                iter.destroy(&mut subst);
                return Err(err);
            }
        };
        if !has_next {
            break;
        }

        subst_is_ho |= subst.has_ho_binding();
        let resolvent = match build_resolvent(bank, clause, literal_index, freshvars, &mut subst) {
            Ok(resolvent) => resolvent,
            Err(err) => {
                iter.destroy(&mut subst);
                return Err(err);
            }
        };
        let is_empty = resolvent.is_empty();
        resolvents.push(resolvent);
        if is_empty {
            break;
        }
    }

    iter.destroy(&mut subst);
    Ok((resolvents, subst_is_ho))
}

fn eq_res_literal(clause: &Clause, literal_index: usize) -> &Eqn {
    let literal = clause
        .literals()
        .as_slice()
        .get(literal_index)
        .expect("equality resolution literal index must be valid");
    assert!(
        literal.is_negative(),
        "equality resolution expects a negative literal"
    );
    literal
}

fn build_resolvent(
    bank: &mut TermBank,
    clause: &Clause,
    literal_index: usize,
    freshvars: &VarBank,
    subst: &mut Substitution,
) -> Result<Clause, Diagnostic> {
    let backtrack = subst.len();
    let result = (|| {
        let _ = clause.literals().subst_norm_except_with_bank(
            Some(literal_index),
            subst,
            freshvars,
            bank,
            problem_type(),
        )?;
        let mut new_literals = clause
            .literals()
            .copy_opt_except_index(Some(literal_index), bank)?;
        new_literals.lambda_normalize(bank)?;
        new_literals.remove_resolved(bank);
        new_literals.remove_duplicates(bank);
        Ok(Clause::alloc(new_literals))
    })();
    subst.backtrack_to_pos(backtrack);
    result
}

fn fresh_var_bank_for_clause(bank: &TermBank, clause: &Clause) -> VarBank {
    let mut variables = BTreeMap::new();
    let _ = clause.collect_variables(&mut variables);
    VarBank::fresh_normalization_bank(
        bank.signature().type_bank(),
        bank.vars(),
        variables.values(),
    )
}

/// Returns the first C `ClausePosFirstEqResLiteral` literal index.
#[must_use]
pub fn first_eq_res_literal_index(clause: &Clause, maximal_only: bool) -> Option<usize> {
    next_eq_res_literal_index_from(clause, 0, maximal_only)
}

/// Returns the next C `ClausePosNextEqResLiteral` literal index after `current`.
#[must_use]
pub fn next_eq_res_literal_index(
    clause: &Clause,
    current: usize,
    maximal_only: bool,
) -> Option<usize> {
    next_eq_res_literal_index_from(clause, current.saturating_add(1), maximal_only)
}

fn next_eq_res_literal_index_from(
    clause: &Clause,
    start: usize,
    maximal_only: bool,
) -> Option<usize> {
    clause
        .literals()
        .as_slice()
        .iter()
        .enumerate()
        .skip(start)
        .find_map(|(index, literal)| is_eq_res_candidate(literal, maximal_only).then_some(index))
}

fn is_eq_res_candidate(literal: &Eqn, maximal_only: bool) -> bool {
    literal.is_negative() && (!maximal_only || literal.is_maximal())
}

/// Computes all equality resolvents and inserts them into `store`.
///
/// This mirrors C `ComputeAllEqnResolvents`: first-order mode uses the single
/// complete-MGU path, while higher-order mode enumerates the CSU stack through
/// `CsuIterator`. Use [`compute_all_eqn_resolvents_with_docs`] for represented
/// proof-documentation output.
///
/// # Errors
///
/// Returns diagnostics from [`compute_eq_res`].
pub fn compute_all_eqn_resolvents(
    bank: &mut TermBank,
    clause: &Clause,
    store: &mut ClauseSet,
    maximal_only: bool,
) -> Result<i64, Diagnostic> {
    let freshvars = fresh_var_bank_for_clause(bank, clause);
    compute_all_eqn_resolvents_impl::<String>(
        bank,
        clause,
        store,
        maximal_only,
        &freshvars,
        false,
        None,
    )
}

/// Computes all equality resolvents using a caller-owned C `freshvars` bank.
///
/// # Errors
///
/// Returns diagnostics from [`compute_eq_res_with_fresh_vars`].
pub fn compute_all_eqn_resolvents_with_fresh_vars(
    bank: &mut TermBank,
    clause: &Clause,
    store: &mut ClauseSet,
    maximal_only: bool,
    freshvars: &VarBank,
) -> Result<i64, Diagnostic> {
    compute_all_eqn_resolvents_impl::<String>(
        bank,
        clause,
        store,
        maximal_only,
        freshvars,
        true,
        None,
    )
}

/// Computes all equality resolvents while emitting represented C
/// `DocClauseCreationDefault(..., inf_eres, ...)` output.
///
/// # Errors
///
/// Returns the same diagnostics as [`compute_all_eqn_resolvents`], plus any
/// proof-documentation write diagnostic.
pub fn compute_all_eqn_resolvents_with_docs(
    output: &mut impl fmt::Write,
    session: &mut ProofDocSession,
    bank: &mut TermBank,
    clause: &Clause,
    store: &mut ClauseSet,
    maximal_only: bool,
) -> Result<i64, Diagnostic> {
    let freshvars = fresh_var_bank_for_clause(bank, clause);
    compute_all_eqn_resolvents_impl(
        bank,
        clause,
        store,
        maximal_only,
        &freshvars,
        false,
        Some((output, session)),
    )
}

/// Computes all equality resolvents using a caller-owned C `freshvars` bank
/// while emitting represented C `DocClauseCreationDefault(..., inf_eres, ...)`
/// output.
///
/// # Errors
///
/// Returns the same diagnostics as
/// [`compute_all_eqn_resolvents_with_fresh_vars`], plus any
/// proof-documentation write diagnostic.
pub fn compute_all_eqn_resolvents_with_fresh_vars_and_docs(
    output: &mut impl fmt::Write,
    session: &mut ProofDocSession,
    bank: &mut TermBank,
    clause: &Clause,
    store: &mut ClauseSet,
    maximal_only: bool,
    freshvars: &VarBank,
) -> Result<i64, Diagnostic> {
    compute_all_eqn_resolvents_impl(
        bank,
        clause,
        store,
        maximal_only,
        freshvars,
        true,
        Some((output, session)),
    )
}

fn compute_all_eqn_resolvents_impl<W: fmt::Write>(
    bank: &mut TermBank,
    clause: &Clause,
    store: &mut ClauseSet,
    maximal_only: bool,
    freshvars: &VarBank,
    reset_freshvars: bool,
    mut doc_context: Option<(&mut W, &mut ProofDocSession)>,
) -> Result<i64, Diagnostic> {
    let mut resolv_count = 0;
    if clause.negative_literal_count() == 0 || clause.query_prop(CP_NO_GENERATION) {
        return Ok(resolv_count);
    }

    let higher_order_problem = problem_type() == ProblemType::HigherOrder;
    let mut next = first_eq_res_literal_index(clause, maximal_only);
    while let Some(index) = next {
        next = next_eq_res_literal_index(clause, index, maximal_only);
        let (mut resolvents, subst_is_ho) = if higher_order_problem {
            compute_eq_res_csu_resolvents(bank, clause, index, freshvars, reset_freshvars)?
        } else {
            let (resolvent, subst_is_ho) =
                compute_eq_res_with_ho_flag(bank, clause, index, freshvars, reset_freshvars)?;
            (resolvent.into_iter().collect(), subst_is_ho)
        };

        while let Some(mut resolvent) = resolvents.pop() {
            resolv_count += 1;
            resolvent.set_proof_depth(clause.proof_depth().saturating_add(1));
            resolvent.set_proof_size(clause.proof_size().saturating_add(1));
            resolvent.set_tptp_type(clause.query_tptp_type());
            resolvent.set_prop(clause.give_props(CP_IS_SOS));
            if let Some((output, session)) = doc_context.as_mut() {
                session.doc_clause_creation(
                    &mut **output,
                    bank,
                    &mut resolvent,
                    ClauseCreationInference::EqualityResolution,
                    ClauseCreationParents::unary(clause),
                    None,
                )?;
            }
            let operation = if subst_is_ho {
                set_is_ho(DC_EQ_RES)
            } else {
                DC_EQ_RES
            };
            clause_push_derivation(&mut resolvent, operation, Some(clause), None);
            store.insert(resolvent);
        }
    }
    Ok(resolv_count)
}

/// Performs C `ClauseERNormalizeVar` over one owned clause.
///
/// The returned count is the number of destructive equality-resolution
/// inferences. When it is non-zero, C inserts the final mutated clause into the
/// supplied store and the caller stops processing the current handle; Rust
/// returns the mutated clause so the caller can requeue it into the appropriate
/// `ClauseSet` owner.
///
/// # Errors
///
/// Returns diagnostics from [`compute_eq_res`].
pub fn clause_er_normalize_var(
    bank: &mut TermBank,
    clause: Clause,
    strong: bool,
) -> Result<(Clause, i64), Diagnostic> {
    let freshvars = fresh_var_bank_for_clause(bank, &clause);
    clause_er_normalize_var_impl::<String>(bank, clause, strong, &freshvars, false, None)
}

/// Performs C `ClauseERNormalizeVar` over one owned clause using a caller-owned
/// `freshvars` bank.
///
/// # Errors
///
/// Returns diagnostics from [`compute_eq_res_with_fresh_vars`].
pub fn clause_er_normalize_var_with_fresh_vars(
    bank: &mut TermBank,
    clause: Clause,
    strong: bool,
    freshvars: &VarBank,
) -> Result<(Clause, i64), Diagnostic> {
    clause_er_normalize_var_impl::<String>(bank, clause, strong, freshvars, true, None)
}

/// Performs C `ClauseERNormalizeVar` while emitting represented
/// `DocClauseModificationDefault(..., inf_eres, clause)` output.
///
/// # Errors
///
/// Returns the same diagnostics as [`clause_er_normalize_var`], plus any
/// proof-documentation write diagnostic.
pub fn clause_er_normalize_var_with_docs(
    output: &mut impl fmt::Write,
    session: &mut ProofDocSession,
    bank: &mut TermBank,
    clause: Clause,
    strong: bool,
) -> Result<(Clause, i64), Diagnostic> {
    let freshvars = fresh_var_bank_for_clause(bank, &clause);
    clause_er_normalize_var_impl(
        bank,
        clause,
        strong,
        &freshvars,
        false,
        Some((output, session)),
    )
}

/// Performs C `ClauseERNormalizeVar` with a caller-owned `freshvars` bank while
/// emitting represented `DocClauseModificationDefault(..., inf_eres, clause)`
/// output.
///
/// # Errors
///
/// Returns the same diagnostics as
/// [`clause_er_normalize_var_with_fresh_vars`], plus any proof-documentation
/// write diagnostic.
pub fn clause_er_normalize_var_with_fresh_vars_and_docs(
    output: &mut impl fmt::Write,
    session: &mut ProofDocSession,
    bank: &mut TermBank,
    clause: Clause,
    strong: bool,
    freshvars: &VarBank,
) -> Result<(Clause, i64), Diagnostic> {
    clause_er_normalize_var_impl(
        bank,
        clause,
        strong,
        freshvars,
        true,
        Some((output, session)),
    )
}

fn clause_er_normalize_var_impl<W: fmt::Write>(
    bank: &mut TermBank,
    mut clause: Clause,
    strong: bool,
    freshvars: &VarBank,
    reset_freshvars: bool,
    mut doc_context: Option<(&mut W, &mut ProofDocSession)>,
) -> Result<(Clause, i64), Diagnostic> {
    let mut count = 0;
    if clause.negative_literal_count() == 0 || clause.query_prop(CP_NO_GENERATION) {
        return Ok((clause, count));
    }

    loop {
        let mut resolved = None;
        for (index, literal) in clause.literals().as_slice().iter().enumerate() {
            if literal.is_negative() && (literal.is_pure_var() || (strong && literal.is_part_var()))
            {
                if let (Some(resolvent), subst_is_ho) =
                    compute_eq_res_with_ho_flag(bank, &clause, index, freshvars, reset_freshvars)?
                {
                    resolved = Some((resolvent, subst_is_ho));
                    break;
                }
            }
        }

        let Some((resolvent, subst_is_ho)) = resolved else {
            break;
        };
        count += 1;
        clause.set_proof_depth(clause.proof_depth().saturating_add(1));
        clause.set_proof_size(clause.proof_size().saturating_add(1));
        clause.replace_literals(resolvent.into_literals());
        if let Some((output, session)) = doc_context.as_mut() {
            let partner = clause.clone();
            session.doc_clause_modification(
                &mut **output,
                bank,
                &mut clause,
                ClauseModificationInference::DestructiveEqualityResolution,
                Some(&partner),
                None,
            )?;
        }
        let operation = if subst_is_ho {
            set_is_ho(DC_DES_EQ_RES)
        } else {
            DC_DES_EQ_RES
        };
        clause_push_derivation(&mut clause, operation, None, None);
    }

    Ok((clause, count))
}

#[cfg(test)]
mod tests {
    use super::{
        clause_er_normalize_var, clause_er_normalize_var_with_docs, compute_all_eqn_resolvents,
        compute_all_eqn_resolvents_with_docs, compute_eq_res, compute_eq_res_with_fresh_vars,
        first_eq_res_literal_index, next_eq_res_literal_index,
    };
    use crate::basics::simple_stuff::{reset_problem_type, set_problem_type, ProblemType};
    use crate::clauses::clause::Clause;
    use crate::clauses::clause_props::{CP_IS_SOS, CP_NO_GENERATION, CP_TYPE_NEG_CONJECTURE};
    use crate::clauses::clausesets::ClauseSet;
    use crate::clauses::derivation::{
        set_is_ho, ClauseDerivationRef, DerivationEntry, DC_DES_EQ_RES, DC_EQ_RES,
    };
    use crate::clauses::eqn::Eqn;
    use crate::clauses::eqn_props::EP_IS_MAXIMAL;
    use crate::clauses::eqnlist::EqnList;
    use crate::clauses::inferencedoc::{ProofDocOutputFormat, ProofDocSession};
    use crate::heuristics::hcb::{HeuristicParmsCell, UnifMode};
    use crate::terms::ho_csu::init_unif_limits;
    use crate::terms::lambda::{apply_terms, close_with_type_prefix};
    use crate::terms::signature::Signature;
    use crate::terms::simpletypes::{alloc_arrow_type, Type};
    use crate::terms::termbanks::TermBank;
    use crate::terms::termtypes::{DerefType, Term};
    use crate::terms::termvars::VarBank;
    use crate::terms::typebanks::TypeBank;
    use crate::test_support::global_state_lock;

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

    fn init_unif_limits_for_test(unif_mode: UnifMode) {
        let mut parms = HeuristicParmsCell {
            unif_mode,
            ..HeuristicParmsCell::default()
        };
        parms.max_unifiers = 8;
        parms.max_unif_steps = 64;
        init_unif_limits(&parms);
    }

    fn init_branching_unif_limits_for_test() {
        let parms = HeuristicParmsCell {
            func_proj_limit: 1,
            imit_limit: 1,
            unif_mode: UnifMode::Multi,
            pattern_oracle: false,
            fixpoint_oracle: false,
            max_unifiers: 4,
            max_unif_steps: 32,
            ..HeuristicParmsCell::default()
        };
        init_unif_limits(&parms);
    }

    fn test_bank() -> TermBank {
        let mut signature = Signature::new(TypeBank::new());
        signature.insert_internal_codes().unwrap();
        TermBank::new(signature).unwrap()
    }

    fn typed_const(bank: &mut TermBank, name: &str) -> Term {
        let type_ = bank.signature().type_bank().default_type();
        let f_code = bank.signature_mut().insert_id(name, 0, false);
        if bank.signature().get_type(f_code).is_none() {
            bank.signature_mut()
                .declare_final_type(f_code, type_.clone())
                .unwrap();
        }
        let term = Term::const_cell_alloc(f_code);
        term.set_type(Some(type_));
        bank.insert(&term, DerefType::Never).unwrap()
    }

    fn typed_const_with_type(bank: &mut TermBank, name: &str, type_: Type) -> Term {
        let f_code = bank.signature_mut().insert_id(name, 0, false);
        if bank.signature().get_type(f_code).is_none() {
            bank.signature_mut()
                .declare_final_type(f_code, type_.clone())
                .unwrap();
        }
        let term = Term::const_cell_alloc(f_code);
        term.set_type(Some(type_));
        bank.insert(&term, DerefType::Never).unwrap()
    }

    fn typed_var(bank: &TermBank, f_code: i64) -> Term {
        let type_ = bank.signature().type_bank().default_type();
        bank.vars().var_assert_alloc(f_code, &type_)
    }

    fn typed_arrow_type(bank: &mut TermBank) -> crate::terms::simpletypes::Type {
        let type_ = bank.signature().type_bank().default_type();
        bank.signature_mut()
            .type_bank_mut()
            .insert_type_shared(alloc_arrow_type(vec![type_.clone(), type_]))
    }

    fn typed_arrow_var(bank: &mut TermBank, f_code: i64) -> Term {
        let type_ = typed_arrow_type(bank);
        bank.vars().var_assert_alloc(f_code, &type_)
    }

    fn typed_arrow_const(bank: &mut TermBank, name: &str) -> Term {
        let type_ = typed_arrow_type(bank);
        let f_code = bank.signature_mut().insert_id(name, 0, false);
        if bank.signature().get_type(f_code).is_none() {
            bank.signature_mut()
                .declare_final_type(f_code, type_.clone())
                .unwrap();
        }
        let term = Term::const_cell_alloc(f_code);
        term.set_type(Some(type_));
        bank.insert(&term, DerefType::Never).unwrap()
    }

    fn typed_unary_code(bank: &mut TermBank, name: &str) -> i64 {
        let type_ = bank.signature().type_bank().default_type();
        let f_code = bank.signature_mut().insert_id(name, 1, false);
        if bank.signature().get_type(f_code).is_none() {
            bank.signature_mut()
                .declare_final_type(f_code, alloc_arrow_type(vec![type_.clone(), type_]))
                .unwrap();
        }
        f_code
    }

    fn typed_unary(bank: &mut TermBank, f_code: i64, arg: &Term) -> Term {
        let type_ = bank.signature().type_bank().default_type();
        let term = Term::top_alloc(f_code, 1);
        term.set_type(Some(type_));
        term.set_argument(0, arg.clone());
        bank.insert(&term, DerefType::Never).unwrap()
    }

    fn lit(bank: &mut TermBank, left: &Term, right: &Term, positive: bool) -> Eqn {
        Eqn::alloc(left.clone(), right.clone(), bank, positive).unwrap()
    }

    #[test]
    fn compute_eq_res_instantiates_remaining_literals() {
        let mut bank = test_bank();
        let x = typed_var(&bank, -2);
        let a = typed_const(&mut bank, "er_a");
        let b = typed_const(&mut bank, "er_b");
        let diseq = lit(&mut bank, &x, &a, false);
        let rest = lit(&mut bank, &x, &b, true);
        let clause = Clause::alloc(EqnList::from_vec(vec![rest, diseq]));

        let resolvent = compute_eq_res(&mut bank, &clause, 1)
            .unwrap()
            .expect("variable disequality should resolve");

        assert_eq!(resolvent.literal_number(), 1);
        let literal = &resolvent.literals().as_slice()[0];
        assert!(literal.is_positive());
        assert_eq!(literal.left(), &a);
        assert_eq!(literal.right(), &b);
    }

    #[test]
    fn compute_eq_res_with_fresh_vars_resets_caller_bank_like_c() {
        let mut bank = test_bank();
        let freshvars = VarBank::new(bank.signature().type_bank());
        bank.vars().pair_shadow(&freshvars);
        let x = typed_var(&bank, -2);
        let y = typed_var(&bank, -4);
        let type_ = bank.signature().type_bank().default_type();
        let _ = freshvars.get_fresh_var(&type_);
        let _ = freshvars.get_fresh_var(&type_);
        let a = typed_const(&mut bank, "er_fresh_a");
        let b = typed_const(&mut bank, "er_fresh_b");
        let rest = lit(&mut bank, &y, &b, true);
        let diseq = lit(&mut bank, &x, &a, false);
        let clause = Clause::alloc(EqnList::from_vec(vec![rest, diseq]));

        let resolvent = compute_eq_res_with_fresh_vars(&mut bank, &clause, 1, &freshvars)
            .unwrap()
            .expect("variable disequality should resolve");

        let literal = &resolvent.literals().as_slice()[0];
        assert_eq!(literal.left().f_code(), -2);
        assert_eq!(literal.right(), &b);
        assert_eq!(freshvars.v_count_for_type(&type_), 1);
    }

    #[test]
    fn compute_eq_res_removes_false_and_duplicate_literals() {
        let mut bank = test_bank();
        let x = typed_var(&bank, -2);
        let a = typed_const(&mut bank, "er_dup_a");
        let b = typed_const(&mut bank, "er_dup_b");
        let diseq = lit(&mut bank, &x, &a, false);
        let false_after_subst = lit(&mut bank, &x, &a, false);
        let first = lit(&mut bank, &x, &b, true);
        let second = lit(&mut bank, &x, &b, true);
        let clause = Clause::alloc(EqnList::from_vec(vec![
            first,
            second,
            diseq,
            false_after_subst,
        ]));

        let resolvent = compute_eq_res(&mut bank, &clause, 2)
            .unwrap()
            .expect("variable disequality should resolve");

        assert_eq!(resolvent.literal_number(), 1);
        let literal = &resolvent.literals().as_slice()[0];
        assert!(literal.is_positive());
        assert_eq!(literal.left(), &a);
        assert_eq!(literal.right(), &b);
    }

    #[test]
    fn compute_eq_res_lambda_normalizes_copied_resolvent_literals() {
        let mut bank = test_bank();
        let i_type = bank.signature().type_bank().default_type();
        let unary_type = alloc_arrow_type(vec![i_type.clone(), i_type.clone()]);
        let f = typed_const_with_type(&mut bank, "er_lambda_f", unary_type);
        let db0 = bank.request_db_var(&i_type, 0);
        let matrix = apply_terms(&mut bank, &f, std::slice::from_ref(&db0)).unwrap();
        let lambda =
            close_with_type_prefix(&mut bank, std::slice::from_ref(&i_type), &matrix).unwrap();
        let x = typed_var(&bank, -2);
        let a = typed_const(&mut bank, "er_lambda_a");
        let b = typed_const(&mut bank, "er_lambda_b");
        let applied = apply_terms(&mut bank, &lambda, std::slice::from_ref(&x)).unwrap();
        let expected = apply_terms(&mut bank, &f, std::slice::from_ref(&a)).unwrap();
        let rest = lit(&mut bank, &applied, &b, true);
        let diseq = lit(&mut bank, &x, &a, false);
        let clause = Clause::alloc(EqnList::from_vec(vec![rest, diseq]));

        let resolvent = compute_eq_res(&mut bank, &clause, 1)
            .unwrap()
            .expect("variable disequality should resolve");

        assert_eq!(resolvent.literal_number(), 1);
        let literal = &resolvent.literals().as_slice()[0];
        assert!(literal.is_positive());
        assert_eq!(literal.left(), &expected);
        assert_eq!(literal.right(), &b);
    }

    #[test]
    fn eq_res_literal_iteration_honors_maximal_filter() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "er_iter_a");
        let b = typed_const(&mut bank, "er_iter_b");
        let c = typed_const(&mut bank, "er_iter_c");
        let mut first = lit(&mut bank, &a, &b, false);
        let mut second = lit(&mut bank, &b, &c, false);
        first.set_prop(EP_IS_MAXIMAL);
        second.del_prop(EP_IS_MAXIMAL);
        let clause = Clause::alloc(EqnList::from_vec(vec![
            lit(&mut bank, &a, &c, true),
            first,
            second,
        ]));

        assert_eq!(first_eq_res_literal_index(&clause, true), Some(1));
        assert_eq!(next_eq_res_literal_index(&clause, 1, true), None);
        assert_eq!(first_eq_res_literal_index(&clause, false), Some(1));
        assert_eq!(next_eq_res_literal_index(&clause, 1, false), Some(2));
    }

    #[test]
    fn compute_all_eqn_resolvents_inserts_metadata_copies() {
        let mut bank = test_bank();
        let x = typed_var(&bank, -2);
        let y = typed_var(&bank, -4);
        let a = typed_const(&mut bank, "er_all_a");
        let b = typed_const(&mut bank, "er_all_b");
        let mut first_diseq = lit(&mut bank, &x, &a, false);
        let mut second_diseq = lit(&mut bank, &y, &b, false);
        first_diseq.set_prop(EP_IS_MAXIMAL);
        second_diseq.set_prop(EP_IS_MAXIMAL);
        let rest = lit(&mut bank, &x, &y, true);
        let mut clause = Clause::alloc(EqnList::from_vec(vec![rest, first_diseq, second_diseq]));
        clause.set_proof_depth(3);
        clause.set_proof_size(5);
        clause.set_tptp_type(CP_TYPE_NEG_CONJECTURE);
        clause.set_prop(CP_IS_SOS);
        let mut store = ClauseSet::new();

        let count = compute_all_eqn_resolvents(&mut bank, &clause, &mut store, true).unwrap();

        assert_eq!(count, 2);
        assert_eq!(store.members(), 2);
        for resolvent in store.iter() {
            assert_eq!(resolvent.proof_depth(), 4);
            assert_eq!(resolvent.proof_size(), 6);
            assert_eq!(resolvent.query_tptp_type(), CP_TYPE_NEG_CONJECTURE);
            assert!(resolvent.query_prop(CP_IS_SOS));
            assert_eq!(resolvent.literal_number(), 2);
            assert_eq!(
                resolvent.derivation().unwrap().as_slice(),
                &[
                    DerivationEntry::Operation(DC_EQ_RES),
                    DerivationEntry::ClauseParent(ClauseDerivationRef::from(&clause)),
                ]
            );
        }
    }

    #[test]
    fn compute_all_eqn_resolvents_with_docs_prints_creation_step() {
        let mut bank = test_bank();
        let x = typed_var(&bank, -2);
        let a = typed_const(&mut bank, "er_doc_a");
        let b = typed_const(&mut bank, "er_doc_b");
        let mut diseq = lit(&mut bank, &x, &a, false);
        diseq.set_prop(EP_IS_MAXIMAL);
        let rest = lit(&mut bank, &x, &b, true);
        let mut clause = Clause::alloc(EqnList::from_vec(vec![rest, diseq]));
        clause.set_ident(52);
        let mut store = ClauseSet::new();
        let mut output = String::new();
        let mut session =
            ProofDocSession::new(ProofDocOutputFormat::Pcl, 2, ProblemType::FirstOrder);

        let count = compute_all_eqn_resolvents_with_docs(
            &mut output,
            &mut session,
            &mut bank,
            &clause,
            &mut store,
            true,
        )
        .unwrap();

        assert_eq!(count, 1);
        assert!(output.contains(" : er(52)\n"));
        let stored = store
            .iter()
            .next()
            .expect("one equality resolvent inserted");
        assert_eq!(stored.ident(), 1);
        assert_eq!(
            stored.derivation().unwrap().as_slice(),
            &[
                DerivationEntry::Operation(DC_EQ_RES),
                DerivationEntry::ClauseParent(ClauseDerivationRef::from(&clause)),
            ]
        );
    }

    #[test]
    fn compute_all_eqn_resolvents_honors_generation_and_maximal_gates() {
        let mut bank = test_bank();
        let x = typed_var(&bank, -2);
        let y = typed_var(&bank, -4);
        let a = typed_const(&mut bank, "er_gate_a");
        let b = typed_const(&mut bank, "er_gate_b");
        let mut maximal = lit(&mut bank, &x, &a, false);
        let non_maximal = lit(&mut bank, &y, &b, false);
        maximal.set_prop(EP_IS_MAXIMAL);
        let clause = Clause::alloc(EqnList::from_vec(vec![maximal, non_maximal]));

        let mut maximal_store = ClauseSet::new();
        assert_eq!(
            compute_all_eqn_resolvents(&mut bank, &clause, &mut maximal_store, true).unwrap(),
            1
        );
        assert_eq!(maximal_store.members(), 1);

        let mut all_store = ClauseSet::new();
        assert_eq!(
            compute_all_eqn_resolvents(&mut bank, &clause, &mut all_store, false).unwrap(),
            2
        );
        assert_eq!(all_store.members(), 2);

        let mut blocked = clause.clone();
        blocked.set_prop(CP_NO_GENERATION);
        let mut blocked_store = ClauseSet::new();
        assert_eq!(
            compute_all_eqn_resolvents(&mut bank, &blocked, &mut blocked_store, false).unwrap(),
            0
        );
        assert!(blocked_store.is_empty());
    }

    #[test]
    fn compute_all_eqn_resolvents_higher_order_uses_first_order_subset() {
        let _guard = global_state_lock();
        let _problem_type = set_problem_type_for_test(ProblemType::HigherOrder);
        init_unif_limits_for_test(UnifMode::Single);
        let mut bank = test_bank();
        let x = typed_var(&bank, -2);
        let a = typed_const(&mut bank, "er_ho_fo_a");
        let b = typed_const(&mut bank, "er_ho_fo_b");
        let mut diseq = lit(&mut bank, &x, &a, false);
        diseq.set_prop(EP_IS_MAXIMAL);
        let rest = lit(&mut bank, &x, &b, true);
        let clause = Clause::alloc(EqnList::from_vec(vec![rest, diseq]));
        let mut store = ClauseSet::new();

        let count = compute_all_eqn_resolvents(&mut bank, &clause, &mut store, true).unwrap();

        assert_eq!(count, 1);
        let resolvent = store.iter().next().expect("first-order subset resolves");
        assert_eq!(resolvent.literal_number(), 1);
        assert_eq!(
            resolvent.derivation().unwrap().as_slice()[0],
            DerivationEntry::Operation(DC_EQ_RES)
        );
    }

    #[test]
    fn compute_all_eqn_resolvents_higher_order_enumerates_csu_pattern_result() {
        let _guard = global_state_lock();
        let _problem_type = set_problem_type_for_test(ProblemType::HigherOrder);
        init_unif_limits_for_test(UnifMode::Multi);
        let mut bank = test_bank();
        let i_type = bank.signature().type_bank().default_type();
        let function = typed_arrow_var(&mut bank, -2);
        let db0 = bank.request_db_var(&i_type, 0);
        let applied = apply_terms(&mut bank, &function, std::slice::from_ref(&db0)).unwrap();
        let a = typed_const(&mut bank, "er_ho_csu_a");
        let b = typed_const(&mut bank, "er_ho_csu_b");
        let mut diseq = lit(&mut bank, &applied, &a, false);
        diseq.set_prop(EP_IS_MAXIMAL);
        let rest = lit(&mut bank, &applied, &b, true);
        let clause = Clause::alloc(EqnList::from_vec(vec![rest, diseq]));
        let mut store = ClauseSet::new();

        let count = compute_all_eqn_resolvents(&mut bank, &clause, &mut store, true).unwrap();

        assert_eq!(count, 1);
        let resolvent = store.iter().next().expect("CSU resolvent inserted");
        assert_eq!(resolvent.literal_number(), 1);
        let literal = &resolvent.literals().as_slice()[0];
        assert!(literal.is_positive());
        assert_eq!(literal.left(), &a);
        assert_eq!(literal.right(), &b);
        assert_eq!(
            resolvent.derivation().unwrap().as_slice(),
            &[
                DerivationEntry::Operation(set_is_ho(DC_EQ_RES)),
                DerivationEntry::ClauseParent(ClauseDerivationRef::from(&clause)),
            ]
        );
    }

    #[test]
    fn compute_all_eqn_resolvents_preserves_multi_csu_pop_and_doc_order() {
        let _guard = global_state_lock();
        let _problem_type = set_problem_type_for_test(ProblemType::HigherOrder);
        init_branching_unif_limits_for_test();
        let mut bank = test_bank();
        let function = typed_arrow_var(&mut bank, -2);
        let imitation_argument = typed_const(&mut bank, "er_multi_a");
        let projection_argument = typed_const(&mut bank, "er_multi_b");
        let other_side = typed_const(&mut bank, "er_multi_e");
        let function_on_imitation = apply_terms(
            &mut bank,
            &function,
            std::slice::from_ref(&imitation_argument),
        )
        .unwrap();
        let function_on_projection = apply_terms(
            &mut bank,
            &function,
            std::slice::from_ref(&projection_argument),
        )
        .unwrap();
        let rest = lit(&mut bank, &function_on_projection, &other_side, true);
        let mut diseq = lit(
            &mut bank,
            &function_on_imitation,
            &imitation_argument,
            false,
        );
        diseq.set_prop(EP_IS_MAXIMAL);
        let mut clause = Clause::alloc(EqnList::from_vec(vec![rest, diseq]));
        clause.set_ident(45);
        let mut store = ClauseSet::new();
        let mut output = String::new();
        let mut session =
            ProofDocSession::new(ProofDocOutputFormat::Pcl, 2, ProblemType::HigherOrder);

        let count = compute_all_eqn_resolvents_with_docs(
            &mut output,
            &mut session,
            &mut bank,
            &clause,
            &mut store,
            true,
        )
        .unwrap();

        assert_eq!(count, 2);
        let resolvents = store.iter().collect::<Vec<_>>();
        assert_eq!(resolvents.len(), 2);
        assert_eq!(resolvents[0].ident(), 1);
        assert_eq!(resolvents[1].ident(), 2);
        assert_eq!(
            resolvents[0]
                .literals()
                .tstp_print_string(&bank, " | ", true, false),
            "er_multi_b=er_multi_e"
        );
        assert_eq!(
            resolvents[1]
                .literals()
                .tstp_print_string(&bank, " | ", true, false),
            "er_multi_a=er_multi_e"
        );
        for resolvent in &resolvents {
            assert_eq!(
                resolvent.derivation().unwrap().as_slice(),
                &[
                    DerivationEntry::Operation(set_is_ho(DC_EQ_RES)),
                    DerivationEntry::ClauseParent(ClauseDerivationRef::from(&clause)),
                ]
            );
        }
        assert_eq!(
            output,
            "     1 : :[++equal(er_multi_b, er_multi_e)] : er(45)\n     2 : :[++equal(er_multi_a, er_multi_e)] : er(45)\n"
        );
    }

    #[test]
    fn compute_eq_res_higher_order_arrow_binding_uses_single_mgu_path() {
        let _guard = global_state_lock();
        let _problem_type = set_problem_type_for_test(ProblemType::HigherOrder);
        let mut bank = test_bank();
        let x = typed_arrow_var(&mut bank, -2);
        let f = typed_arrow_const(&mut bank, "er_ho_arrow_f");
        let mut diseq = lit(&mut bank, &x, &f, false);
        diseq.set_prop(EP_IS_MAXIMAL);
        let clause = Clause::alloc(EqnList::from_vec(vec![diseq]));

        let resolvent = compute_eq_res(&mut bank, &clause, 0)
            .unwrap()
            .expect("single ComputeEqRes MGU path accepts arrow-variable binding");

        assert!(resolvent.is_empty());
    }

    #[test]
    fn compute_eq_res_higher_order_applied_variable_uses_rigid_prefix_mgu() {
        let _guard = global_state_lock();
        let _problem_type = set_problem_type_for_test(ProblemType::HigherOrder);
        let mut bank = test_bank();
        let function = typed_arrow_var(&mut bank, -2);
        let prefix = typed_const(&mut bank, "er_ho_mgu_prefix");
        let suffix = typed_const(&mut bank, "er_ho_mgu_suffix");
        let individual = bank.signature().type_bank().default_type();
        let binary = bank
            .signature_mut()
            .type_bank_mut()
            .insert_type_shared(alloc_arrow_type(vec![
                individual.clone(),
                individual.clone(),
                individual,
            ]));
        let rigid = typed_const_with_type(&mut bank, "er_ho_mgu_rigid", binary);
        let applied = apply_terms(&mut bank, &function, std::slice::from_ref(&suffix)).unwrap();
        let target = apply_terms(&mut bank, &rigid, &[prefix, suffix]).unwrap();
        let mut diseq = lit(&mut bank, &applied, &target, false);
        diseq.set_prop(EP_IS_MAXIMAL);
        let clause = Clause::alloc(EqnList::from_vec(vec![diseq]));

        let resolvent = compute_eq_res(&mut bank, &clause, 0)
            .unwrap()
            .expect("banked ComputeEqRes binds the applied variable to the rigid prefix");

        assert!(resolvent.is_empty());
        assert!(function.binding().is_none());
    }

    #[test]
    fn er_normalize_var_rewrites_until_no_variable_disequality_remains() {
        let mut bank = test_bank();
        let left_var = typed_var(&bank, -2);
        let middle_var = typed_var(&bank, -4);
        let right_var = typed_var(&bank, -6);
        let left_const = typed_const(&mut bank, "er_norm_a");
        let right_const = typed_const(&mut bank, "er_norm_b");
        let first_diseq = lit(&mut bank, &left_var, &middle_var, false);
        let second_diseq = lit(&mut bank, &middle_var, &right_var, false);
        let first_rest = lit(&mut bank, &left_var, &left_const, true);
        let second_rest = lit(&mut bank, &right_var, &right_const, true);
        let mut clause = Clause::alloc(EqnList::from_vec(vec![
            first_rest,
            second_rest,
            first_diseq,
            second_diseq,
        ]));
        clause.set_ident(44);
        clause.set_prop(CP_IS_SOS);

        let (clause, count) = clause_er_normalize_var(&mut bank, clause, false).unwrap();

        assert_eq!(count, 2);
        assert_eq!(clause.ident(), 44);
        assert!(clause.query_prop(CP_IS_SOS));
        assert_eq!(clause.proof_depth(), 2);
        assert_eq!(clause.proof_size(), 2);
        assert_eq!(clause.literal_number(), 2);
        assert!(clause.literals().as_slice().iter().all(Eqn::is_positive));
        let first_left = clause.literals().as_slice()[0].left().clone();
        assert!(first_left.is_free_var());
        assert!(clause
            .literals()
            .as_slice()
            .iter()
            .all(|literal| literal.left() == &first_left));
        assert!(clause
            .literals()
            .as_slice()
            .iter()
            .any(|literal| literal.right() == &left_const));
        assert!(clause
            .literals()
            .as_slice()
            .iter()
            .any(|literal| literal.right() == &right_const));
        assert_eq!(
            clause.derivation().unwrap().as_slice(),
            &[
                DerivationEntry::Operation(DC_DES_EQ_RES),
                DerivationEntry::Operation(DC_DES_EQ_RES),
            ]
        );
    }

    #[test]
    fn er_normalize_var_with_docs_prints_modification_steps() {
        let mut bank = test_bank();
        let x = typed_var(&bank, -2);
        let y = typed_var(&bank, -4);
        let rhs = typed_const(&mut bank, "er_norm_doc_rhs");
        let mut clause = Clause::alloc(EqnList::from_vec(vec![
            lit(&mut bank, &x, &rhs, true),
            lit(&mut bank, &x, &y, false),
        ]));
        clause.set_ident(61);
        let mut output = String::new();
        let mut session =
            ProofDocSession::new(ProofDocOutputFormat::Pcl, 2, ProblemType::FirstOrder);

        let (clause, count) =
            clause_er_normalize_var_with_docs(&mut output, &mut session, &mut bank, clause, false)
                .unwrap();

        assert_eq!(count, 1);
        assert_eq!(clause.ident(), 1);
        assert_eq!(session.id_source.current_ident(), 1);
        assert!(output.contains(" : er(61)\n"));
        assert_eq!(
            clause.derivation().unwrap().as_slice(),
            &[DerivationEntry::Operation(DC_DES_EQ_RES)]
        );
    }

    #[test]
    fn er_normalize_var_strong_option_allows_one_variable_side() {
        let mut bank = test_bank();
        let x = typed_var(&bank, -2);
        let a = typed_const(&mut bank, "er_strong_a");
        let b = typed_const(&mut bank, "er_strong_b");
        let f_code = typed_unary_code(&mut bank, "er_strong_f");
        let f_a = typed_unary(&mut bank, f_code, &a);
        let diseq = lit(&mut bank, &x, &f_a, false);
        let rest = lit(&mut bank, &x, &b, true);
        let clause = Clause::alloc(EqnList::from_vec(vec![rest, diseq]));

        let (weak_clause, weak_count) =
            clause_er_normalize_var(&mut bank, clause.clone(), false).unwrap();
        let (strong_clause, strong_count) =
            clause_er_normalize_var(&mut bank, clause, true).unwrap();

        assert_eq!(weak_count, 0);
        assert_eq!(weak_clause.literal_number(), 2);
        assert_eq!(strong_count, 1);
        assert_eq!(strong_clause.literal_number(), 1);
        assert_eq!(strong_clause.literals().as_slice()[0].left(), &f_a);
    }

    #[test]
    fn er_normalize_var_respects_no_generation_property() {
        let mut bank = test_bank();
        let x = typed_var(&bank, -2);
        let y = typed_var(&bank, -4);
        let rest = lit(&mut bank, &x, &y, false);
        let mut clause = Clause::alloc(EqnList::from_vec(vec![rest]));
        clause.set_prop(CP_NO_GENERATION);

        let (clause, count) = clause_er_normalize_var(&mut bank, clause, false).unwrap();

        assert_eq!(count, 0);
        assert_eq!(clause.literal_number(), 1);
    }
}
