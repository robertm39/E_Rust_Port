use std::fmt;

use crate::basics::error::Diagnostic;
use crate::basics::simple_stuff::{problem_type, ProblemType};
use crate::clauses::clause::Clause;
use crate::clauses::clause_props::{CP_INPUT_FORMULA, CP_TYPE_CONJECTURE, CP_TYPE_NEG_CONJECTURE};
use crate::clauses::clausefunc::{
    clause_remove_superfluous_literals, clause_set_canonize,
    clause_set_remove_superfluous_literals, clause_set_replace_injectivity_defs,
};
use crate::clauses::clausesets::ClauseSet;
use crate::clauses::derivation::{clause_push_derivation, DC_UNFOLD};
use crate::clauses::eqn::Eqn;
use crate::clauses::eqn_props::{EqnSide, EP_IS_ORIENTED, EP_MAX_IS_UP_TO_DATE};
use crate::clauses::inferencedoc::ProofDocSession;
use crate::terms::lambda::{abstract_vars, apply_terms, whnf_step};
use crate::terms::match_mgu::subst_match_complete;
use crate::terms::simpletypes::Type;
use crate::terms::subst::Substitution;
use crate::terms::termbanks::TermBank;
use crate::terms::termfunc::term_standard_weight;
use crate::terms::termtypes::Term;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DefinitionSearchStart {
    Beginning,
    FromId(i64),
    End,
}

/// Performs the currently ported clause-set preprocessing pass.
///
/// This ports the non-unfolding portion of C `ClauseSetPreprocess`: remove
/// superfluous literals, filter tautologies, optionally replace injectivity
/// definitions, and canonize the remaining clause set.
///
/// # Errors
///
/// Returns a diagnostic if tautology filtering or injectivity-definition
/// replacement cannot construct required terms.
pub fn clause_set_preprocess(
    set: &mut ClauseSet,
    archive: &mut ClauseSet,
    tmp_terms: &mut TermBank,
    terms: &mut TermBank,
    replace_injectivity_defs: bool,
    _eqdef_incrlimit: i64,
    _eqdef_maxclauses: i64,
) -> Result<i64, Diagnostic> {
    let _removed_literals = clause_set_remove_superfluous_literals(set, terms);
    let removed_clauses = set.filter_tautologies(tmp_terms)?;
    if replace_injectivity_defs {
        let _replaced = clause_set_replace_injectivity_defs(set, archive, terms)?;
    }
    clause_set_canonize(set, terms);
    Ok(removed_clauses)
}

/// Applies one equational definition to all matching first-order subterms in a
/// clause.
///
/// Returns whether at least one subterm was unfolded.
///
/// # Errors
///
/// Returns a diagnostic if an unfolded replacement cannot be inserted into the
/// term bank.
pub fn clause_unfold_eq_def(
    clause: &mut Clause,
    demodulator: &Clause,
    lside: &Term,
    rside: &Term,
    bank: &mut TermBank,
) -> Result<bool, Diagnostic> {
    Ok(clause_unfold_eq_def_impl::<String>(clause, demodulator, lside, rside, bank, None)? != 0)
}

/// Applies one equational definition and emits C `DocClauseEqUnfold`
/// documentation when the session output level enables it.
///
/// # Errors
///
/// Returns a diagnostic if unfolding or proof-documentation rendering fails.
pub fn clause_unfold_eq_def_with_docs(
    output: &mut impl fmt::Write,
    session: &mut ProofDocSession,
    clause: &mut Clause,
    demodulator: &Clause,
    lside: &Term,
    rside: &Term,
    bank: &mut TermBank,
) -> Result<bool, Diagnostic> {
    Ok(clause_unfold_eq_def_impl(
        clause,
        demodulator,
        lside,
        rside,
        bank,
        Some((output, session)),
    )? != 0)
}

fn clause_unfold_eq_def_impl<W: fmt::Write>(
    clause: &mut Clause,
    demodulator: &Clause,
    lside: &Term,
    rside: &Term,
    bank: &mut TermBank,
    doc_context: Option<(&mut W, &mut ProofDocSession)>,
) -> Result<usize, Diagnostic> {
    if problem_type() == ProblemType::NotInitialized {
        return Ok(0);
    }

    let mut applications = 0;
    for literal in clause.literals_mut().as_mut_slice() {
        applications += eqn_unfold_def(literal, lside, rside, bank)?;
    }

    if applications == 0 {
        return Ok(0);
    }

    if demodulator.query_tptp_type() == CP_TYPE_CONJECTURE {
        clause.set_tptp_type(CP_TYPE_CONJECTURE);
    }
    clause.del_prop(CP_INPUT_FORMULA);
    if let Some((output, session)) = doc_context {
        let _result =
            session.doc_clause_eq_unfold(output, bank, clause, demodulator, applications)?;
    }
    for _ in 0..applications {
        clause_push_derivation(clause, DC_UNFOLD, Some(demodulator), None);
    }
    clause.set_weight(clause.standard_weight());
    Ok(applications)
}

/// Applies one equational definition to every clause in a set.
///
/// Returns whether at least one clause changed.
///
/// # Errors
///
/// Returns a diagnostic if an unfolded replacement cannot be inserted into the
/// term bank.
///
/// # Panics
///
/// Panics if `demodulator` is not a unit clause. The C caller reaches this
/// helper only with a definition position selected from a unit equational
/// clause.
pub fn clause_set_unfold_eq_def(
    set: &mut ClauseSet,
    demodulator: &Clause,
    demod_side: EqnSide,
    bank: &mut TermBank,
) -> Result<bool, Diagnostic> {
    clause_set_unfold_eq_def_impl::<String>(set, demodulator, demod_side, bank, None)
}

/// Applies one equational definition across a set while emitting C
/// `DocClauseEqUnfold` documentation for changed clauses.
///
/// # Errors
///
/// Returns a diagnostic if unfolding or proof-documentation rendering fails.
pub fn clause_set_unfold_eq_def_with_docs(
    output: &mut impl fmt::Write,
    session: &mut ProofDocSession,
    set: &mut ClauseSet,
    demodulator: &Clause,
    demod_side: EqnSide,
    bank: &mut TermBank,
) -> Result<bool, Diagnostic> {
    clause_set_unfold_eq_def_impl(set, demodulator, demod_side, bank, Some((output, session)))
}

fn clause_set_unfold_eq_def_impl<W: fmt::Write>(
    set: &mut ClauseSet,
    demodulator: &Clause,
    demod_side: EqnSide,
    bank: &mut TermBank,
    mut doc_context: Option<(&mut W, &mut ProofDocSession)>,
) -> Result<bool, Diagnostic> {
    if problem_type() == ProblemType::NotInitialized {
        return Ok(false);
    }

    let literal = demodulator
        .literals()
        .as_slice()
        .first()
        .expect("definition demodulator must be a unit clause");
    let (lside, rside) = if demod_side == EqnSide::LeftSide {
        (literal.left().clone(), literal.right().clone())
    } else {
        (literal.right().clone(), literal.left().clone())
    };
    let (lside, rside) = if problem_type() == ProblemType::HigherOrder {
        clause_extract_ho_definition(demodulator, demod_side, bank)?
    } else {
        (lside, rside)
    };
    let demod_is_conjecture = demodulator.is_conjecture();
    let mut changed = false;

    for clause in set.iter_mut() {
        let unfolded = if let Some((output, session)) = doc_context.as_mut() {
            clause_unfold_eq_def_impl(
                clause,
                demodulator,
                &lside,
                &rside,
                bank,
                Some((&mut **output, &mut **session)),
            )? != 0
        } else {
            clause_unfold_eq_def_impl::<W>(clause, demodulator, &lside, &rside, bank, None)? != 0
        };
        if unfolded {
            changed = true;
            let _removed_literals = clause_remove_superfluous_literals(clause, bank);
            if demod_is_conjecture {
                clause.set_tptp_type(CP_TYPE_NEG_CONJECTURE);
            }
            clause.set_weight(clause.standard_weight());
        }
    }

    if changed {
        set.recompute_literals();
    }
    Ok(changed)
}

/// Unfolds every eligible equational definition in set order and archives the
/// removed definition clauses.
///
/// Returns the number of removed definition clauses.
///
/// # Errors
///
/// Returns a diagnostic if an unfolded replacement cannot be inserted into the
/// term bank.
///
/// # Panics
///
/// Panics if the clause-set definition lookup returns a position that is not
/// attached to a clause or does not expose both equation sides. Those are
/// internal `ClauseSet::find_eq_definition` invariants.
pub fn clause_set_unfold_all_eq_defs(
    set: &mut ClauseSet,
    passive: Option<&mut ClauseSet>,
    archive: &mut ClauseSet,
    bank: &mut TermBank,
    min_arity: usize,
    eqdef_incrlimit: i64,
) -> Result<i64, Diagnostic> {
    clause_set_unfold_all_eq_defs_impl::<String>(
        set,
        passive,
        archive,
        bank,
        min_arity,
        eqdef_incrlimit,
        None,
    )
}

/// Unfolds eligible definitions while emitting C `DocClauseEqUnfold`
/// documentation for changed active and passive clauses.
///
/// # Errors
///
/// Returns a diagnostic if unfolding or proof-documentation rendering fails.
#[expect(
    clippy::too_many_arguments,
    reason = "C-compatible definition unfolding keeps caller state explicit"
)]
pub fn clause_set_unfold_all_eq_defs_with_docs(
    output: &mut impl fmt::Write,
    session: &mut ProofDocSession,
    set: &mut ClauseSet,
    passive: Option<&mut ClauseSet>,
    archive: &mut ClauseSet,
    bank: &mut TermBank,
    min_arity: usize,
    eqdef_incrlimit: i64,
) -> Result<i64, Diagnostic> {
    clause_set_unfold_all_eq_defs_impl(
        set,
        passive,
        archive,
        bank,
        min_arity,
        eqdef_incrlimit,
        Some((output, session)),
    )
}

fn clause_set_unfold_all_eq_defs_impl<W: fmt::Write>(
    set: &mut ClauseSet,
    passive: Option<&mut ClauseSet>,
    archive: &mut ClauseSet,
    bank: &mut TermBank,
    min_arity: usize,
    eqdef_incrlimit: i64,
    mut doc_context: Option<(&mut W, &mut ProofDocSession)>,
) -> Result<i64, Diagnostic> {
    if problem_type() == ProblemType::NotInitialized {
        return Ok(0);
    }

    let mut removed = 0;
    let mut start = DefinitionSearchStart::Beginning;
    let mut passive = passive;

    while let Some(demod_pos) = find_eq_definition_from_start(set, bank, min_arity, start) {
        let demod_clause = demod_pos
            .clause()
            .expect("definition position must carry a clause");
        let demod_id = demod_clause.ident();
        let demod_side = demod_pos.side();
        let lside = demod_pos
            .get_side()
            .expect("definition position must expose its selected side");
        let rside = demod_pos
            .get_other_side()
            .expect("definition position must expose its opposite side");

        start = next_clause_id_after(set, demod_id)
            .map_or(DefinitionSearchStart::End, DefinitionSearchStart::FromId);

        if term_standard_weight(&rside) - term_standard_weight(&lside) > eqdef_incrlimit {
            continue;
        }

        let demodulator = set
            .extract_by_id(demod_id)
            .expect("located definition clause must still be in the set");
        let _changed = if let Some((output, session)) = doc_context.as_mut() {
            clause_set_unfold_eq_def_impl(
                set,
                &demodulator,
                demod_side,
                bank,
                Some((&mut **output, &mut **session)),
            )?
        } else {
            clause_set_unfold_eq_def_impl::<W>(set, &demodulator, demod_side, bank, None)?
        };
        if let Some(passive_set) = passive.as_deref_mut() {
            let _changed = if let Some((output, session)) = doc_context.as_mut() {
                clause_set_unfold_eq_def_impl(
                    passive_set,
                    &demodulator,
                    demod_side,
                    bank,
                    Some((&mut **output, &mut **session)),
                )?
            } else {
                clause_set_unfold_eq_def_impl::<W>(
                    passive_set,
                    &demodulator,
                    demod_side,
                    bank,
                    None,
                )?
            };
        }
        archive.insert(demodulator);
        removed += 1;
    }

    Ok(removed)
}

/// Unfolds eligible equational definitions, then refilters tautologies and
/// canonizes the set when at least one definition was removed.
///
/// Returns the number of removed definition and tautology clauses.
///
/// # Errors
///
/// Returns a diagnostic if unfolding or tautology filtering cannot construct
/// required terms.
pub fn clause_set_unfold_eq_def_normalize(
    set: &mut ClauseSet,
    passive: Option<&mut ClauseSet>,
    archive: &mut ClauseSet,
    tmp_terms: &mut TermBank,
    terms: &mut TermBank,
    eqdef_incrlimit: i64,
    eqdef_maxclauses: i64,
) -> Result<i64, Diagnostic> {
    clause_set_unfold_eq_def_normalize_impl::<String>(
        set,
        passive,
        archive,
        tmp_terms,
        terms,
        eqdef_incrlimit,
        eqdef_maxclauses,
        None,
    )
}

/// Unfolds eligible definitions, refilters tautologies, and emits live C
/// equality-unfolding proof documentation for changed clauses.
///
/// # Errors
///
/// Returns a diagnostic if unfolding, tautology filtering, or
/// proof-documentation rendering fails.
#[expect(
    clippy::too_many_arguments,
    reason = "C-compatible preprocessing normalization keeps caller state explicit"
)]
pub fn clause_set_unfold_eq_def_normalize_with_docs(
    output: &mut impl fmt::Write,
    session: &mut ProofDocSession,
    set: &mut ClauseSet,
    passive: Option<&mut ClauseSet>,
    archive: &mut ClauseSet,
    tmp_terms: &mut TermBank,
    terms: &mut TermBank,
    eqdef_incrlimit: i64,
    eqdef_maxclauses: i64,
) -> Result<i64, Diagnostic> {
    clause_set_unfold_eq_def_normalize_impl(
        set,
        passive,
        archive,
        tmp_terms,
        terms,
        eqdef_incrlimit,
        eqdef_maxclauses,
        Some((output, session)),
    )
}

#[expect(
    clippy::too_many_arguments,
    reason = "Shared implementation mirrors C ClauseSetUnfoldEqDefNormalize state"
)]
fn clause_set_unfold_eq_def_normalize_impl<W: fmt::Write>(
    set: &mut ClauseSet,
    passive: Option<&mut ClauseSet>,
    archive: &mut ClauseSet,
    tmp_terms: &mut TermBank,
    terms: &mut TermBank,
    eqdef_incrlimit: i64,
    eqdef_maxclauses: i64,
    doc_context: Option<(&mut W, &mut ProofDocSession)>,
) -> Result<i64, Diagnostic> {
    if problem_type() == ProblemType::NotInitialized
        || eqdef_incrlimit == i64::MIN
        || set.members() > eqdef_maxclauses
    {
        return Ok(0);
    }

    let mut removed = 0;
    let unfolded = clause_set_unfold_all_eq_defs_impl(
        set,
        passive,
        archive,
        terms,
        1,
        eqdef_incrlimit,
        doc_context,
    )?;
    if unfolded != 0 {
        removed += unfolded;
        removed += set.filter_tautologies(tmp_terms)?;
        clause_set_canonize(set, terms);
    }
    Ok(removed)
}

fn eqn_unfold_def(
    eqn: &mut Eqn,
    lside: &Term,
    rside: &Term,
    bank: &mut TermBank,
) -> Result<usize, Diagnostic> {
    let mut applications = 0;
    let left = term_unfold_def(bank, eqn.left(), &mut applications, lside, rside)?;
    let right = term_unfold_def(bank, eqn.right(), &mut applications, lside, rside)?;

    if left != *eqn.left() || right != *eqn.right() {
        eqn.set_left_raw(left);
        eqn.set_right_raw(right);
        eqn.del_prop(EP_MAX_IS_UP_TO_DATE | EP_IS_ORIENTED);
    }

    Ok(applications)
}

fn term_unfold_def(
    bank: &mut TermBank,
    term: &Term,
    applications: &mut usize,
    lside: &Term,
    rside: &Term,
) -> Result<Term, Diagnostic> {
    if term.is_any_var() {
        return Ok(term.clone());
    }

    let top_copy = Term::top_copy_without_args(term);
    let mut changed = false;
    for (index, arg) in term.argument_clones().into_iter().enumerate() {
        let arg = arg.unwrap_or_else(|| panic!("term argument {index} is uninitialized"));
        let unfolded = term_unfold_def(bank, &arg, applications, lside, rside)?;
        if unfolded != arg {
            changed = true;
        }
        top_copy.set_argument(index, unfolded);
    }

    let candidate = if changed {
        bank.term_top_insert(top_copy)?
    } else {
        term.clone()
    };
    let result = match problem_type() {
        ProblemType::FirstOrder => term_top_unfold_def_fo(bank, &candidate, lside, rside)?,
        ProblemType::HigherOrder => term_top_unfold_def_ho(bank, &candidate, lside, rside)?,
        ProblemType::NotInitialized => candidate.clone(),
    };
    if result != candidate {
        *applications += 1;
    }
    Ok(result)
}

fn term_top_unfold_def_fo(
    bank: &mut TermBank,
    term: &Term,
    lside: &Term,
    rside: &Term,
) -> Result<Term, Diagnostic> {
    assert!(
        !lside.is_any_var(),
        "definition left side must not be a variable"
    );
    if lside.f_code() != term.f_code() {
        return Ok(term.clone());
    }

    let mut subst = Substitution::new();
    let matched = subst_match_complete(lside, term, &mut subst);
    if !matched {
        return Ok(term.clone());
    }

    let result = bank.insert_instantiated(rside)?;
    subst.backtrack();
    Ok(result)
}

fn term_top_unfold_def_ho(
    bank: &mut TermBank,
    term: &Term,
    lside: &Term,
    rside: &Term,
) -> Result<Term, Diagnostic> {
    assert!(
        !lside.is_top_level_any_var(),
        "higher-order definition left side must not be a variable"
    );
    assert!(
        !lside.is_lambda(),
        "higher-order definition left side must be a symbol"
    );
    if lside.f_code() != term.f_code() {
        return Ok(term.clone());
    }
    assert_eq!(
        lside.type_(),
        rside.type_(),
        "higher-order definition sides must have the same type"
    );
    if term.arity() == 0 {
        assert_eq!(
            term.type_(),
            rside.type_(),
            "constant definition replacement must preserve type"
        );
        return Ok(rside.clone());
    }

    let args = term
        .argument_clones()
        .into_iter()
        .enumerate()
        .map(|(index, arg)| arg.unwrap_or_else(|| panic!("term argument {index} is uninitialized")))
        .collect::<Vec<_>>();
    let applied = apply_terms(bank, rside, &args)?;
    let result = whnf_step(bank, &applied)?;
    assert_eq!(
        result.type_(),
        term.type_(),
        "higher-order unfolding must preserve target type"
    );
    Ok(result)
}

/// Extracts the symbol/lambda pair from a higher-order equational definition.
///
/// This mirrors C `ClauseExtractHODefinition`: from a unit equation whose
/// selected side is `f(X1,...,Xn)`, return the zero-arity symbol `f` as the
/// left side and the other equation side abstracted over `X1,...,Xn` as the
/// right side.
///
/// # Errors
///
/// Returns a diagnostic if abstracting the right side or inserting the
/// extracted symbol into the term bank fails.
///
/// # Panics
///
/// Panics if `def_side` is not an equation side, if `clause` is not a unit
/// equation demodulator, if any definition argument is not a free variable, or
/// if the abstracted definition type shape does not match the selected
/// symbol's signature type. These are the same caller-side invariants asserted
/// by the C helper after `ClauseIsEqDefinition` selects a valid definition
/// side; Rust compares type shape here because `Type` equality models sharing.
pub fn clause_extract_ho_definition(
    clause: &Clause,
    def_side: EqnSide,
    bank: &mut TermBank,
) -> Result<(Term, Term), Diagnostic> {
    assert!(
        matches!(def_side, EqnSide::LeftSide | EqnSide::RightSide),
        "higher-order definition side must select an equation side"
    );
    let literal = clause
        .literals()
        .as_slice()
        .first()
        .expect("definition demodulator must be a unit clause");
    let (def_term, other_term) = if def_side == EqnSide::LeftSide {
        (literal.left(), literal.right())
    } else {
        (literal.right(), literal.left())
    };

    let vars = def_term
        .argument_clones()
        .into_iter()
        .enumerate()
        .map(|(index, arg)| {
            let arg = arg.unwrap_or_else(|| panic!("definition argument {index} is uninitialized"));
            assert!(
                arg.is_free_var(),
                "higher-order definition arguments must be free variables"
            );
            arg
        })
        .collect::<Vec<_>>();
    let abstracted = abstract_vars(bank, other_term, &vars)?;
    let symbol = Term::top_alloc(def_term.f_code(), 0);
    let symbol_type = abstracted.type_();
    assert!(
        type_shapes_equal(
            symbol_type.as_ref(),
            bank.signature().get_type(symbol.f_code()),
        ),
        "higher-order definition symbol type must match its signature type"
    );
    symbol.set_type(symbol_type);
    let symbol = bank.term_top_insert(symbol)?;
    Ok((symbol, abstracted))
}

fn type_shapes_equal(left: Option<&Type>, right: Option<&Type>) -> bool {
    match (left, right) {
        (Some(left), Some(right)) => type_shape_equal(left, right),
        (None, None) => true,
        _ => false,
    }
}

fn type_shape_equal(left: &Type, right: &Type) -> bool {
    left.f_code() == right.f_code()
        && left.arity() == right.arity()
        && left
            .args()
            .iter()
            .zip(right.args())
            .all(|(left_arg, right_arg)| type_shape_equal(left_arg, right_arg))
}

fn find_eq_definition_from_start(
    set: &ClauseSet,
    bank: &TermBank,
    min_arity: usize,
    start: DefinitionSearchStart,
) -> Option<crate::clauses::clausepos::ClausePos> {
    match start {
        DefinitionSearchStart::Beginning => set.find_eq_definition(bank, min_arity),
        DefinitionSearchStart::FromId(ident) => {
            set.find_eq_definition_from_id(bank, min_arity, ident)
        }
        DefinitionSearchStart::End => None,
    }
}

fn next_clause_id_after(set: &ClauseSet, ident: i64) -> Option<i64> {
    let mut seen = false;
    for clause in set.iter() {
        if seen {
            return Some(clause.ident());
        }
        if clause.ident() == ident {
            seen = true;
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::basics::simple_stuff::{reset_problem_type, set_problem_type, ProblemType};
    use crate::clauses::clause::Clause;
    use crate::clauses::clause_props::{CP_INPUT_FORMULA, CP_TYPE_AXIOM};
    use crate::clauses::derivation::{ClauseDerivationRef, DerivationEntry, DC_UNFOLD};
    use crate::clauses::eqn::Eqn;
    use crate::clauses::eqnlist::EqnList;
    use crate::clauses::inferencedoc::{ProofDocOutputFormat, ProofDocSession};
    use crate::terms::signature::{Signature, SIG_DB_LAMBDA_CODE};
    use crate::terms::simpletypes::alloc_arrow_type;
    use crate::terms::termbanks::TermBank;
    use crate::terms::termfunc::term_has_f_code;
    use crate::terms::termtypes::Term;
    use crate::terms::typebanks::TypeBank;

    fn test_bank() -> TermBank {
        let mut signature = Signature::new(TypeBank::new());
        signature.insert_internal_codes().unwrap();
        TermBank::new(signature).unwrap()
    }

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

    fn object_const(bank: &mut TermBank, name: &str) -> Term {
        let type_ = bank.signature().type_bank().i_type();
        let f_code = bank.signature_mut().insert_id(name, 0, false);
        bank.signature_mut()
            .declare_final_type(f_code, type_)
            .unwrap();
        bank.create_const_term(f_code).unwrap()
    }

    fn object_var(bank: &TermBank, code: i64) -> Term {
        let type_ = bank.signature().type_bank().i_type();
        bank.vars().var_assert_alloc(code, &type_)
    }

    fn object_unary_code(bank: &mut TermBank, name: &str) -> i64 {
        let type_ = bank.signature().type_bank().i_type();
        let f_code = bank.signature_mut().insert_id(name, 1, false);
        bank.signature_mut()
            .declare_final_type(f_code, alloc_arrow_type(vec![type_.clone(), type_]))
            .unwrap();
        f_code
    }

    fn object_binary_code(bank: &mut TermBank, name: &str) -> i64 {
        let type_ = bank.signature().type_bank().i_type();
        let f_code = bank.signature_mut().insert_id(name, 2, false);
        bank.signature_mut()
            .declare_final_type(
                f_code,
                alloc_arrow_type(vec![type_.clone(), type_.clone(), type_]),
            )
            .unwrap();
        f_code
    }

    fn unary_with_code(bank: &mut TermBank, f_code: i64, arg: &Term) -> Term {
        let term = Term::top_alloc(f_code, 1);
        term.set_type(arg.type_());
        term.set_argument(0, arg.clone());
        bank.term_top_insert(term).unwrap()
    }

    fn binary_with_code(bank: &mut TermBank, f_code: i64, left: &Term, right: &Term) -> Term {
        let term = Term::top_alloc(f_code, 2);
        term.set_type(left.type_());
        term.set_argument(0, left.clone());
        term.set_argument(1, right.clone());
        bank.term_top_insert(term).unwrap()
    }

    fn literal(bank: &mut TermBank, left: &Term, right: &Term, positive: bool) -> Eqn {
        Eqn::alloc(left.clone(), right.clone(), bank, positive).unwrap()
    }

    fn clause(literals: Vec<Eqn>) -> Clause {
        let mut clause = Clause::alloc(EqnList::from_vec(literals));
        clause.set_weight(clause.standard_weight());
        clause
    }

    #[test]
    fn clause_extract_ho_definition_abstracts_left_side_arguments_in_c_order() {
        let mut terms = test_bank();
        let x = object_var(&terms, -2);
        let y = object_var(&terms, -3);
        let a = object_const(&mut terms, "extract_ho_left_a");
        let b = object_const(&mut terms, "extract_ho_left_b");
        let f_code = object_binary_code(&mut terms, "extract_ho_left_f");
        let h_code = object_binary_code(&mut terms, "extract_ho_left_h");
        let f_x_y = binary_with_code(&mut terms, f_code, &x, &y);
        let h_y_x = binary_with_code(&mut terms, h_code, &y, &x);
        let def = clause(vec![literal(&mut terms, &f_x_y, &h_y_x, true)]);

        let (symbol, lambda) =
            clause_extract_ho_definition(&def, EqnSide::LeftSide, &mut terms).unwrap();

        assert_eq!(symbol.f_code(), f_code);
        assert_eq!(symbol.arity(), 0);
        assert!(type_shapes_equal(
            symbol.type_().as_ref(),
            terms.signature().get_type(f_code),
        ));
        assert_eq!(lambda.f_code(), SIG_DB_LAMBDA_CODE);
        assert_eq!(lambda.type_(), symbol.type_());

        let applied = apply_terms(&mut terms, &lambda, &[a.clone(), b.clone()]).unwrap();
        let normalized = whnf_step(&mut terms, &applied).unwrap();
        let expected = binary_with_code(&mut terms, h_code, &b, &a);
        assert_eq!(normalized, expected);
    }

    #[test]
    fn clause_extract_ho_definition_uses_right_side_when_selected() {
        let mut terms = test_bank();
        let x = object_var(&terms, -2);
        let y = object_var(&terms, -3);
        let a = object_const(&mut terms, "extract_ho_right_a");
        let b = object_const(&mut terms, "extract_ho_right_b");
        let f_code = object_binary_code(&mut terms, "extract_ho_right_f");
        let h_code = object_binary_code(&mut terms, "extract_ho_right_h");
        let f_x_y = binary_with_code(&mut terms, f_code, &x, &y);
        let h_y_x = binary_with_code(&mut terms, h_code, &y, &x);
        let def = clause(vec![literal(&mut terms, &h_y_x, &f_x_y, true)]);

        let (symbol, lambda) =
            clause_extract_ho_definition(&def, EqnSide::RightSide, &mut terms).unwrap();

        assert_eq!(symbol.f_code(), f_code);
        assert_eq!(symbol.arity(), 0);
        assert!(type_shapes_equal(
            symbol.type_().as_ref(),
            terms.signature().get_type(f_code),
        ));
        assert_eq!(lambda.f_code(), SIG_DB_LAMBDA_CODE);
        assert_eq!(lambda.type_(), symbol.type_());

        let applied = apply_terms(&mut terms, &lambda, &[a.clone(), b.clone()]).unwrap();
        let normalized = whnf_step(&mut terms, &applied).unwrap();
        let expected = binary_with_code(&mut terms, h_code, &b, &a);
        assert_eq!(normalized, expected);
    }

    #[test]
    fn clause_set_preprocess_filters_tautologies_and_canonizes_survivors() {
        let mut terms = test_bank();
        let mut tmp_terms = TermBank::new(terms.signature().clone()).unwrap();
        let a = object_const(&mut terms, "preprocess_a");
        let b = object_const(&mut terms, "preprocess_b");
        let tautology = clause(vec![literal(&mut terms, &a, &a, true)]);
        let survivor = clause(vec![literal(&mut terms, &b, &a, true)]);
        let mut set = ClauseSet::from_clauses([tautology, survivor]);
        let mut archive = ClauseSet::new();

        let removed = clause_set_preprocess(
            &mut set,
            &mut archive,
            &mut tmp_terms,
            &mut terms,
            false,
            20,
            20_000,
        )
        .unwrap();

        assert_eq!(removed, 1);
        assert_eq!(set.members(), 1);
        assert_eq!(archive.members(), 0);
    }

    #[test]
    fn clause_set_preprocess_removes_duplicate_literals_without_counting_clause_removal() {
        let mut terms = test_bank();
        let mut tmp_terms = TermBank::new(terms.signature().clone()).unwrap();
        let a = object_const(&mut terms, "preprocess_dup_a");
        let b = object_const(&mut terms, "preprocess_dup_b");
        let duplicate = literal(&mut terms, &a, &b, true);
        let mut set = ClauseSet::from_clauses([clause(vec![duplicate.clone(), duplicate])]);
        let mut archive = ClauseSet::new();

        let removed = clause_set_preprocess(
            &mut set,
            &mut archive,
            &mut tmp_terms,
            &mut terms,
            false,
            20,
            20_000,
        )
        .unwrap();

        assert_eq!(removed, 0);
        assert_eq!(set.members(), 1);
        assert_eq!(set.iter().next().map(Clause::literal_number), Some(1));
    }

    #[test]
    fn clause_unfold_eq_def_rewrites_each_matching_occurrence() {
        let _problem_type = set_problem_type_for_test(ProblemType::FirstOrder);
        let mut terms = test_bank();
        let x = object_var(&terms, -2);
        let a = object_const(&mut terms, "unfold_a");
        let b = object_const(&mut terms, "unfold_b");
        let f_code = object_unary_code(&mut terms, "unfold_f");
        let g_code = object_unary_code(&mut terms, "unfold_g");
        let f_x = unary_with_code(&mut terms, f_code, &x);
        let g_x = unary_with_code(&mut terms, g_code, &x);
        let f_a = unary_with_code(&mut terms, f_code, &a);
        let f_b = unary_with_code(&mut terms, f_code, &b);
        let mut demodulator = clause(vec![literal(&mut terms, &f_x, &g_x, true)]);
        demodulator.set_tptp_type(CP_TYPE_AXIOM);
        let demodulator_id = demodulator.ident();
        let mut target = clause(vec![literal(&mut terms, &f_a, &f_b, true)]);

        assert!(clause_unfold_eq_def(&mut target, &demodulator, &f_x, &g_x, &mut terms).unwrap());

        let literal = &target.literals().as_slice()[0];
        assert!(!term_has_f_code(literal.left(), f_code));
        assert!(!term_has_f_code(literal.right(), f_code));
        assert!(term_has_f_code(literal.left(), g_code));
        assert!(term_has_f_code(literal.right(), g_code));
        assert_eq!(
            target.derivation().unwrap().as_slice(),
            &[
                DerivationEntry::Operation(DC_UNFOLD),
                DerivationEntry::ClauseParent(ClauseDerivationRef::new(demodulator_id, 0)),
                DerivationEntry::Operation(DC_UNFOLD),
                DerivationEntry::ClauseParent(ClauseDerivationRef::new(demodulator_id, 0)),
            ]
        );
    }

    #[test]
    fn clause_unfold_eq_def_with_docs_prints_live_unfold_step() {
        let _problem_type = set_problem_type_for_test(ProblemType::FirstOrder);
        let mut terms = test_bank();
        let x = object_var(&terms, -2);
        let a = object_const(&mut terms, "unfold_doc_a");
        let b = object_const(&mut terms, "unfold_doc_b");
        let f_code = object_unary_code(&mut terms, "unfold_doc_f");
        let g_code = object_unary_code(&mut terms, "unfold_doc_g");
        let f_x = unary_with_code(&mut terms, f_code, &x);
        let g_x = unary_with_code(&mut terms, g_code, &x);
        let f_a = unary_with_code(&mut terms, f_code, &a);
        let f_b = unary_with_code(&mut terms, f_code, &b);
        let mut demodulator = clause(vec![literal(&mut terms, &f_x, &g_x, true)]);
        demodulator.set_ident(30);
        demodulator.set_tptp_type(CP_TYPE_AXIOM);
        let mut target = clause(vec![literal(&mut terms, &f_a, &f_b, true)]);
        target.set_ident(7);
        target.set_prop(CP_INPUT_FORMULA);
        let mut session =
            ProofDocSession::new(ProofDocOutputFormat::Pcl, 2, ProblemType::FirstOrder);
        let mut rendered = String::new();

        assert!(clause_unfold_eq_def_with_docs(
            &mut rendered,
            &mut session,
            &mut target,
            &demodulator,
            &f_x,
            &g_x,
            &mut terms,
        )
        .unwrap());

        assert_eq!(target.ident(), 1);
        assert!(!target.query_prop(CP_INPUT_FORMULA));
        assert!(rendered.contains("rw(rw(7,30),30) : 'unfolding'"));
        assert_eq!(session.id_source.current_ident(), 1);
    }

    #[test]
    fn clause_unfold_eq_def_skips_same_head_nonmatching_occurrence() {
        let _problem_type = set_problem_type_for_test(ProblemType::FirstOrder);
        let mut terms = test_bank();
        let a = object_const(&mut terms, "unfold_nomatch_a");
        let b = object_const(&mut terms, "unfold_nomatch_b");
        let c = object_const(&mut terms, "unfold_nomatch_c");
        let f_code = object_unary_code(&mut terms, "unfold_nomatch_f");
        let g_code = object_unary_code(&mut terms, "unfold_nomatch_g");
        let f_a = unary_with_code(&mut terms, f_code, &a);
        let g_a = unary_with_code(&mut terms, g_code, &a);
        let f_b = unary_with_code(&mut terms, f_code, &b);
        let mut demodulator = clause(vec![literal(&mut terms, &f_a, &g_a, true)]);
        demodulator.set_tptp_type(CP_TYPE_AXIOM);
        let mut target = clause(vec![literal(&mut terms, &f_b, &c, true)]);

        assert!(!clause_unfold_eq_def(&mut target, &demodulator, &f_a, &g_a, &mut terms).unwrap());

        let literal = &target.literals().as_slice()[0];
        assert_eq!(literal.left(), &f_b);
        assert_eq!(literal.right(), &c);
        assert!(target.derivation().is_none());
    }

    #[test]
    fn clause_set_unfold_eq_def_normalize_archives_definition_and_rewrites_passive() {
        let _problem_type = set_problem_type_for_test(ProblemType::FirstOrder);
        let mut terms = test_bank();
        let mut tmp_terms = TermBank::new(terms.signature().clone()).unwrap();
        let x = object_var(&terms, -2);
        let a = object_const(&mut terms, "unfold_set_a");
        let b = object_const(&mut terms, "unfold_set_b");
        let c = object_const(&mut terms, "unfold_set_c");
        let f_code = object_unary_code(&mut terms, "unfold_set_f");
        let g_code = object_unary_code(&mut terms, "unfold_set_g");
        let f_x = unary_with_code(&mut terms, f_code, &x);
        let g_x = unary_with_code(&mut terms, g_code, &x);
        let f_a = unary_with_code(&mut terms, f_code, &a);
        let f_b = unary_with_code(&mut terms, f_code, &b);
        let def = clause(vec![literal(&mut terms, &f_x, &g_x, true)]);
        let def_id = def.ident();
        let target = clause(vec![literal(&mut terms, &f_a, &c, true)]);
        let target_id = target.ident();
        let mut set = ClauseSet::from_clauses([def, target]);
        let mut passive =
            ClauseSet::from_clauses([clause(vec![literal(&mut terms, &f_b, &c, true)])]);
        let mut archive = ClauseSet::new();

        let removed = clause_set_unfold_eq_def_normalize(
            &mut set,
            Some(&mut passive),
            &mut archive,
            &mut tmp_terms,
            &mut terms,
            20,
            20_000,
        )
        .unwrap();

        assert_eq!(removed, 1);
        assert_eq!(archive.find_by_id(def_id).map(Clause::ident), Some(def_id));
        assert_eq!(set.members(), 1);
        assert_eq!(set.iter().next().map(Clause::ident), Some(target_id));
        let target_literal = &set.iter().next().unwrap().literals().as_slice()[0];
        assert!(!term_has_f_code(target_literal.left(), f_code));
        assert!(!term_has_f_code(target_literal.right(), f_code));
        let passive_literal = &passive.iter().next().unwrap().literals().as_slice()[0];
        assert!(!term_has_f_code(passive_literal.left(), f_code));
        assert!(!term_has_f_code(passive_literal.right(), f_code));
    }

    #[test]
    fn clause_set_unfold_eq_def_normalize_higher_order_extracts_lambda_definition() {
        let _problem_type = set_problem_type_for_test(ProblemType::HigherOrder);
        let mut terms = test_bank();
        let mut tmp_terms = TermBank::new(terms.signature().clone()).unwrap();
        let x = object_var(&terms, -2);
        let a = object_const(&mut terms, "unfold_ho_a");
        let c = object_const(&mut terms, "unfold_ho_c");
        let f_code = object_unary_code(&mut terms, "unfold_ho_f");
        let g_code = object_unary_code(&mut terms, "unfold_ho_g");
        let f_x = unary_with_code(&mut terms, f_code, &x);
        let g_x = unary_with_code(&mut terms, g_code, &x);
        let f_a = unary_with_code(&mut terms, f_code, &a);
        let def = clause(vec![literal(&mut terms, &f_x, &g_x, true)]);
        let target = clause(vec![literal(&mut terms, &f_a, &c, true)]);
        let mut set = ClauseSet::from_clauses([def, target]);
        let mut archive = ClauseSet::new();

        let removed = clause_set_unfold_eq_def_normalize(
            &mut set,
            None,
            &mut archive,
            &mut tmp_terms,
            &mut terms,
            20,
            20_000,
        )
        .unwrap();

        assert_eq!(removed, 1);
        assert_eq!(archive.members(), 1);
        assert_eq!(set.members(), 1);
        let target_literal = &set.iter().next().unwrap().literals().as_slice()[0];
        assert!(!term_has_f_code(target_literal.left(), f_code));
        assert!(!term_has_f_code(target_literal.right(), f_code));
        assert!(
            term_has_f_code(target_literal.left(), g_code)
                || term_has_f_code(target_literal.right(), g_code)
        );
    }

    #[test]
    fn clause_set_unfold_eq_def_normalize_respects_disable_and_size_limits() {
        let _problem_type = set_problem_type_for_test(ProblemType::FirstOrder);
        let mut terms = test_bank();
        let mut tmp_terms = TermBank::new(terms.signature().clone()).unwrap();
        let x = object_var(&terms, -2);
        let a = object_const(&mut terms, "unfold_limit_a");
        let c = object_const(&mut terms, "unfold_limit_c");
        let f_code = object_unary_code(&mut terms, "unfold_limit_f");
        let g_code = object_unary_code(&mut terms, "unfold_limit_g");
        let f_x = unary_with_code(&mut terms, f_code, &x);
        let g_x = unary_with_code(&mut terms, g_code, &x);
        let f_a = unary_with_code(&mut terms, f_code, &a);
        let def = clause(vec![literal(&mut terms, &f_x, &g_x, true)]);
        let target = clause(vec![literal(&mut terms, &f_a, &c, true)]);
        let mut disabled_set = ClauseSet::from_clauses([def.clone(), target.clone()]);
        let mut oversized_set = ClauseSet::from_clauses([def, target]);
        let mut archive = ClauseSet::new();

        let disabled = clause_set_unfold_eq_def_normalize(
            &mut disabled_set,
            None,
            &mut archive,
            &mut tmp_terms,
            &mut terms,
            i64::MIN,
            20_000,
        )
        .unwrap();
        assert_eq!(disabled, 0);
        assert_eq!(disabled_set.members(), 2);
        assert_eq!(archive.members(), 0);

        let oversized = clause_set_unfold_eq_def_normalize(
            &mut oversized_set,
            None,
            &mut archive,
            &mut tmp_terms,
            &mut terms,
            20,
            1,
        )
        .unwrap();
        assert_eq!(oversized, 0);
        assert_eq!(oversized_set.members(), 2);
        assert_eq!(archive.members(), 0);
    }
}
