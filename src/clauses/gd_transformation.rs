use crate::basics::error::{Diagnostic, ErrorCode};
use crate::clauses::clause::Clause;
use crate::clauses::clausesets::ClauseSet;
use crate::clauses::derivation::{clause_push_derivation, DC_INTRO_DEF};
use crate::clauses::eqn::Eqn;
use crate::clauses::eqnlist::EqnList;
use crate::terms::termbanks::TermBank;
use crate::terms::termfunc::term_is_ground;
use crate::terms::termtypes::Term;
use std::collections::BTreeMap;

type DefinitionMap = BTreeMap<i64, Term>;

/// Adds TWEE-style goal definitions for ground terms from conjecture clauses.
///
/// This ports C `ClauseSetGDTransform`: it collects non-constant ground terms
/// from selected positive/negative literals in conjecture clauses, introduces
/// fresh typed constants for their definition normal forms, and inserts the
/// resulting positive unit equations into `clauses`.
///
/// # Errors
///
/// Returns a diagnostic if a term has no inferred type, if a generated typed
/// definition symbol cannot be declared, or if a shared definition term cannot
/// be inserted into the term bank.
pub fn clause_set_gd_transform(
    bank: &mut TermBank,
    clauses: &mut ClauseSet,
    add_goal_defs_pos: bool,
    add_goal_defs_neg: bool,
    add_goal_defs_subterms: bool,
) -> Result<i64, Diagnostic> {
    let mut goal_terms = BTreeMap::new();
    for clause in clauses.iter() {
        if clause.is_conjecture() {
            clause.collect_ground_terms(
                &mut goal_terms,
                add_goal_defs_pos,
                add_goal_defs_neg,
                add_goal_defs_subterms,
            );
        }
    }

    let mut result = 0;
    let mut defs = DefinitionMap::new();
    let terms = goal_terms.values().cloned().collect::<Vec<_>>();
    for term in terms {
        debug_assert!(
            term_is_ground(&term),
            "goal-definition collection only returns ground terms"
        );
        if !term.is_const() && !defs.contains_key(&term.entry_no()) {
            if add_goal_defs_subterms {
                result += gd_term_rek_define(bank, &term, &mut defs, clauses)?;
            } else {
                result += gd_term_define(bank, &term, &mut defs, clauses)?;
            }
        }
    }

    Ok(result)
}

fn gd_def_nf(bank: &mut TermBank, term: &Term, defs: &DefinitionMap) -> Result<Term, Diagnostic> {
    if term.is_const() {
        return Ok(term.clone());
    }
    if let Some(rhs) = defs.get(&term.entry_no()) {
        return Ok(rhs.clone());
    }

    let def_lhs = Term::top_copy_without_args(term);
    for index in 0..term.arity() {
        let arg = term.argument(index).ok_or_else(|| {
            Diagnostic::new(
                ErrorCode::OTHER_ERROR,
                format!("goal definition term argument {index} is uninitialized"),
            )
        })?;
        def_lhs.set_argument(index, gd_def_nf(bank, &arg, defs)?);
    }
    let def_lhs = bank.term_top_insert(def_lhs)?;
    if let Some(rhs) = defs.get(&def_lhs.entry_no()) {
        return Ok(rhs.clone());
    }
    Ok(def_lhs)
}

fn gd_term_define(
    bank: &mut TermBank,
    term: &Term,
    defs: &mut DefinitionMap,
    clauses: &mut ClauseSet,
) -> Result<i64, Diagnostic> {
    if term.is_const() || defs.contains_key(&term.entry_no()) {
        return Ok(0);
    }

    let lhs = gd_def_nf(bank, term, defs)?;
    let ret_type = term.type_().ok_or_else(|| {
        Diagnostic::new(
            ErrorCode::TYPE_ERROR,
            "goal definition term has no inferred type",
        )
    })?;
    let new_const = bank
        .signature_mut()
        .get_new_typed_def_code(&[], &ret_type)?;
    let rhs = bank.create_const_term(new_const)?;
    defs.insert(lhs.entry_no(), rhs.clone());

    let def_eqn = Eqn::alloc(lhs, rhs, bank, true)?;
    let mut clause = Clause::alloc(EqnList::from_vec(vec![def_eqn]));
    clause_push_derivation(&mut clause, DC_INTRO_DEF, None, None);
    clauses.insert(clause);
    Ok(1)
}

fn gd_term_rek_define(
    bank: &mut TermBank,
    term: &Term,
    defs: &mut DefinitionMap,
    clauses: &mut ClauseSet,
) -> Result<i64, Diagnostic> {
    if term.is_const() {
        return Ok(0);
    }
    debug_assert!(
        term_is_ground(term),
        "recursive goal-definition insertion expects ground terms"
    );

    let mut result = 0;
    for index in 0..term.arity() {
        let arg = term.argument(index).ok_or_else(|| {
            Diagnostic::new(
                ErrorCode::OTHER_ERROR,
                format!("goal definition term argument {index} is uninitialized"),
            )
        })?;
        result += gd_term_rek_define(bank, &arg, defs, clauses)?;
    }

    result += gd_term_define(bank, term, defs, clauses)?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clauses::clause_props::{CP_TYPE_AXIOM, CP_TYPE_NEG_CONJECTURE};
    use crate::clauses::derivation::{derivation_entries, DerivationEntry};
    use crate::terms::signature::Signature;
    use crate::terms::simpletypes::alloc_arrow_type;
    use crate::terms::termtypes::DerefType;
    use crate::terms::typebanks::TypeBank;

    fn test_bank() -> TermBank {
        let mut signature = Signature::new(TypeBank::new());
        signature.insert_internal_codes().unwrap();
        TermBank::new(signature).unwrap()
    }

    fn object_const(bank: &mut TermBank, name: &str) -> Term {
        let type_ = bank.signature().type_bank().i_type();
        let f_code = bank.signature_mut().insert_id(name, 0, false);
        bank.signature_mut()
            .declare_final_type(f_code, type_)
            .unwrap();
        bank.create_const_term(f_code).unwrap()
    }

    fn object_unary(bank: &mut TermBank, name: &str, arg: &Term) -> Term {
        let i_type = bank.signature().type_bank().i_type();
        let fn_type = bank
            .signature_mut()
            .type_bank_mut()
            .insert_type_shared(alloc_arrow_type(vec![i_type.clone(), i_type.clone()]));
        let f_code = bank.signature_mut().insert_id(name, 1, false);
        bank.signature_mut()
            .declare_final_type(f_code, fn_type)
            .unwrap();
        let term = Term::top_alloc(f_code, 1);
        term.set_type(Some(i_type));
        term.set_argument(0, arg.clone());
        bank.insert(&term, DerefType::Never).unwrap()
    }

    fn equation_literal(bank: &mut TermBank, left: &Term, right: &Term, positive: bool) -> Eqn {
        Eqn::alloc(left.clone(), right.clone(), bank, positive).unwrap()
    }

    fn conjecture_clause(literal: Eqn) -> Clause {
        let mut clause = Clause::alloc(EqnList::from_vec(vec![literal]));
        clause.set_tptp_type(CP_TYPE_NEG_CONJECTURE);
        clause
    }

    fn axiom_clause(literal: Eqn) -> Clause {
        let mut clause = Clause::alloc(EqnList::from_vec(vec![literal]));
        clause.set_tptp_type(CP_TYPE_AXIOM);
        clause
    }

    fn generated_def_count(bank: &TermBank) -> i64 {
        bank.signature().newdef_count()
    }

    #[test]
    fn gd_transform_ignores_non_conjecture_clauses() {
        let mut bank = test_bank();
        let a = object_const(&mut bank, "gd_axiom_a");
        let f_a = object_unary(&mut bank, "gd_axiom_f", &a);
        let mut clauses =
            ClauseSet::from_clauses([axiom_clause(equation_literal(&mut bank, &f_a, &a, true))]);

        let added = clause_set_gd_transform(&mut bank, &mut clauses, true, true, false).unwrap();

        assert_eq!(added, 0);
        assert_eq!(clauses.members(), 1);
    }

    #[test]
    fn gd_transform_adds_definition_for_selected_goal_term() {
        let mut bank = test_bank();
        let a = object_const(&mut bank, "gd_goal_a");
        let f_a = object_unary(&mut bank, "gd_goal_f", &a);
        let mut clauses = ClauseSet::from_clauses([conjecture_clause(equation_literal(
            &mut bank, &f_a, &a, true,
        ))]);

        let added = clause_set_gd_transform(&mut bank, &mut clauses, true, false, false).unwrap();

        assert_eq!(added, 1);
        assert_eq!(clauses.members(), 2);
        assert_eq!(generated_def_count(&bank), 1);
        let def_clause = clauses.iter().last().unwrap();
        assert_eq!(
            derivation_entries(def_clause),
            &[DerivationEntry::Operation(DC_INTRO_DEF)]
        );
    }

    #[test]
    fn gd_transform_respects_literal_sign_selection() {
        let mut bank = test_bank();
        let a = object_const(&mut bank, "gd_sign_a");
        let f_a = object_unary(&mut bank, "gd_sign_f", &a);
        let mut clauses = ClauseSet::from_clauses([conjecture_clause(equation_literal(
            &mut bank, &f_a, &a, true,
        ))]);

        let added = clause_set_gd_transform(&mut bank, &mut clauses, false, true, false).unwrap();

        assert_eq!(added, 0);
        assert_eq!(clauses.members(), 1);
    }

    #[test]
    fn gd_transform_can_define_subterms_before_parent_terms() {
        let mut bank = test_bank();
        let a = object_const(&mut bank, "gd_subterm_a");
        let f_a = object_unary(&mut bank, "gd_subterm_f", &a);
        let g_f_a = object_unary(&mut bank, "gd_subterm_g", &f_a);
        let mut clauses = ClauseSet::from_clauses([conjecture_clause(equation_literal(
            &mut bank, &g_f_a, &a, true,
        ))]);

        let added = clause_set_gd_transform(&mut bank, &mut clauses, true, false, true).unwrap();

        assert_eq!(added, 2);
        assert_eq!(clauses.members(), 3);
        assert_eq!(generated_def_count(&bank), 2);
    }
}
