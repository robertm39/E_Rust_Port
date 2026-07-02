use crate::basics::error::{Diagnostic, ErrorCode};
use crate::basics::simple_stuff::{problem_type, ProblemType};
use crate::clauses::clause::Clause;
use crate::clauses::clause_props::{CP_IS_SOS, CP_NO_GENERATION};
use crate::clauses::clausesets::ClauseSet;
use crate::clauses::derivation::{clause_push_derivation, DC_DES_EQ_RES, DC_EQ_RES};
use crate::clauses::eqn::Eqn;
use crate::clauses::inferencedoc::{
    ClauseCreationInference, ClauseCreationParents, ClauseModificationInference, ProofDocSession,
};
use crate::terms::match_mgu::{subst_mgu_complete, term_has_higher_order_unification_surface};
use crate::terms::subst::Substitution;
use crate::terms::termbanks::TermBank;
use crate::terms::termvars::VarBank;
use std::{collections::BTreeMap, fmt};

/// C `EqResOnMaximalLiteralsOnly` default.
pub const EQ_RES_ON_MAXIMAL_LITERALS_ONLY: bool = true;

/// Builds the first-order single-clause C `ComputeEqRes` result.
///
/// In higher-order problem mode, this uses the same first-order MGU path for
/// ordinary first-order subterms and reports an explicit diagnostic when the
/// candidate needs full CSU enumeration.
///
/// # Errors
///
/// Returns a diagnostic if a higher-order candidate needs full CSU enumeration,
/// or if term-bank insertion fails while copying the resolvent.
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
    let literal = clause
        .literals()
        .as_slice()
        .get(literal_index)
        .expect("equality resolution literal index must be valid");
    assert!(
        literal.is_negative(),
        "equality resolution expects a negative literal"
    );
    let higher_order_problem = problem_type() == ProblemType::HigherOrder;
    if higher_order_problem
        && (term_has_higher_order_unification_surface(literal.left())
            || term_has_higher_order_unification_surface(literal.right()))
    {
        return Err(higher_order_eq_res_diagnostic());
    }

    let mut subst = Substitution::new();
    if !subst_mgu_complete(literal.left(), literal.right(), &mut subst) {
        return Ok(None);
    }
    if higher_order_problem && subst.has_ho_binding_for_problem(ProblemType::HigherOrder) {
        subst.backtrack();
        return Err(higher_order_eq_res_diagnostic());
    }

    let freshvars = fresh_var_bank_for_clause(bank, clause);
    let resolvent = build_resolvent(bank, clause, literal_index, &freshvars, &mut subst)?;
    subst.backtrack();
    Ok(Some(resolvent))
}

fn higher_order_eq_res_diagnostic() -> Diagnostic {
    Diagnostic::new(
        ErrorCode::OTHER_ERROR,
        "higher-order equality-resolution CSU enumeration is not ported yet",
    )
}

fn build_resolvent(
    bank: &mut TermBank,
    clause: &Clause,
    literal_index: usize,
    freshvars: &VarBank,
    subst: &mut Substitution,
) -> Result<Clause, Diagnostic> {
    let backtrack = clause
        .literals()
        .subst_norm_except(Some(literal_index), subst, freshvars);
    let mut new_literals = clause
        .literals()
        .copy_opt_except_index(Some(literal_index), bank)?;
    subst.backtrack_to_pos(backtrack);
    new_literals.lambda_normalize(bank)?;
    new_literals.remove_resolved(bank);
    new_literals.remove_duplicates(bank);
    Ok(Clause::alloc(new_literals))
}

fn fresh_var_bank_for_clause(bank: &TermBank, clause: &Clause) -> VarBank {
    let freshvars = VarBank::new(bank.signature().type_bank());
    let mut variables = BTreeMap::new();
    let _ = clause.collect_variables(&mut variables);
    let max_var = variables
        .values()
        .map(|variable| -variable.f_code())
        .max()
        .unwrap_or(0);
    let default_type = bank.signature().type_bank().default_type();
    while freshvars.fresh_count() < max_var {
        let _ = freshvars.get_fresh_var(&default_type);
    }
    freshvars.set_v_counts_to_used();
    freshvars
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

/// Computes all first-order equality resolvents and inserts them into `store`.
///
/// This mirrors C `ComputeAllEqnResolvents` for the first-order MGU path. Higher
/// order CSU enumeration still belongs to a later slice, so higher-order problem
/// type is rejected through [`compute_eq_res`]. Use
/// [`compute_all_eqn_resolvents_with_docs`] for represented
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
    compute_all_eqn_resolvents_impl::<String>(bank, clause, store, maximal_only, None)
}

/// Computes all first-order equality resolvents while emitting represented C
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
    compute_all_eqn_resolvents_impl(bank, clause, store, maximal_only, Some((output, session)))
}

fn compute_all_eqn_resolvents_impl<W: fmt::Write>(
    bank: &mut TermBank,
    clause: &Clause,
    store: &mut ClauseSet,
    maximal_only: bool,
    mut doc_context: Option<(&mut W, &mut ProofDocSession)>,
) -> Result<i64, Diagnostic> {
    let mut resolv_count = 0;
    if clause.negative_literal_count() == 0 || clause.query_prop(CP_NO_GENERATION) {
        return Ok(resolv_count);
    }

    let mut next = first_eq_res_literal_index(clause, maximal_only);
    while let Some(index) = next {
        next = next_eq_res_literal_index(clause, index, maximal_only);
        if let Some(mut resolvent) = compute_eq_res(bank, clause, index)? {
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
            clause_push_derivation(&mut resolvent, DC_EQ_RES, Some(clause), None);
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
    clause_er_normalize_var_impl::<String>(bank, clause, strong, None)
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
    clause_er_normalize_var_impl(bank, clause, strong, Some((output, session)))
}

fn clause_er_normalize_var_impl<W: fmt::Write>(
    bank: &mut TermBank,
    mut clause: Clause,
    strong: bool,
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
                if let Some(resolvent) = compute_eq_res(bank, &clause, index)? {
                    resolved = Some(resolvent);
                    break;
                }
            }
        }

        let Some(resolvent) = resolved else {
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
        clause_push_derivation(&mut clause, DC_DES_EQ_RES, None, None);
    }

    Ok((clause, count))
}

#[cfg(test)]
mod tests {
    use super::{
        clause_er_normalize_var, clause_er_normalize_var_with_docs, compute_all_eqn_resolvents,
        compute_all_eqn_resolvents_with_docs, compute_eq_res, first_eq_res_literal_index,
        next_eq_res_literal_index,
    };
    use crate::basics::error::ErrorCode;
    use crate::basics::simple_stuff::{reset_problem_type, set_problem_type, ProblemType};
    use crate::clauses::clause::Clause;
    use crate::clauses::clause_props::{CP_IS_SOS, CP_NO_GENERATION, CP_TYPE_NEG_CONJECTURE};
    use crate::clauses::clausesets::ClauseSet;
    use crate::clauses::derivation::{
        ClauseDerivationRef, DerivationEntry, DC_DES_EQ_RES, DC_EQ_RES,
    };
    use crate::clauses::eqn::Eqn;
    use crate::clauses::eqn_props::EP_IS_MAXIMAL;
    use crate::clauses::eqnlist::EqnList;
    use crate::clauses::inferencedoc::{ProofDocOutputFormat, ProofDocSession};
    use crate::terms::lambda::{apply_terms, close_with_type_prefix};
    use crate::terms::signature::Signature;
    use crate::terms::simpletypes::{alloc_arrow_type, Type};
    use crate::terms::termbanks::TermBank;
    use crate::terms::termtypes::{DerefType, Term};
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
    fn compute_eq_res_higher_order_arrow_binding_remains_diagnostic() {
        let _guard = global_state_lock();
        let _problem_type = set_problem_type_for_test(ProblemType::HigherOrder);
        let mut bank = test_bank();
        let x = typed_arrow_var(&mut bank, -2);
        let f = typed_arrow_const(&mut bank, "er_ho_arrow_f");
        let mut diseq = lit(&mut bank, &x, &f, false);
        diseq.set_prop(EP_IS_MAXIMAL);
        let clause = Clause::alloc(EqnList::from_vec(vec![diseq]));

        let error = compute_eq_res(&mut bank, &clause, 0).unwrap_err();

        assert_eq!(error.code(), ErrorCode::OTHER_ERROR);
        assert!(error.message().contains("CSU enumeration"));
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
