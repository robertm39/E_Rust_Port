use crate::basics::error::Diagnostic;
use crate::clauses::clause::Clause;
use crate::clauses::clause_props::CP_IS_ORIENTED;
use crate::clauses::clausefunc::clause_remove_superfluous_literals;
use crate::clauses::eqn::Eqn;
use crate::clauses::eqn_props::{
    EP_IS_EQU_LITERAL, EP_IS_ORIENTED, EP_IS_POSITIVE, EP_MAX_IS_UP_TO_DATE,
};
use crate::orderings::ocb::OrderControlBlock;
use crate::terms::termbanks::TermBank;
use crate::terms::termtypes::{term_identity_id, Term};
use std::collections::HashMap;

type LocalRwSystem = HashMap<usize, Term>;

/// Rewrites a clause with local rules extracted from that same clause.
///
/// This ports C `ClauseLocalRW`: orient literals, collect negative oriented
/// equalities `s != t` as `s -> t`, collect positive predicate literals as
/// `p -> $false`, then rewrite all non-rule-source literals with the resulting
/// pointer-identity map.
///
/// # Errors
///
/// Returns a diagnostic if term-bank insertion fails while recursively
/// rebuilding a changed term.
///
/// # Panics
///
/// Panics if term mapping violates the C term-bank sharing/type invariants, or
/// if literal cleanup is requested while the clause is still indexed.
pub fn clause_local_rw(
    ocb: &mut OrderControlBlock,
    bank: &mut TermBank,
    clause: &mut Clause,
) -> Result<bool, Diagnostic> {
    clause.orient_literals(ocb, bank);

    let rw_sys = collect_local_rw_system(bank, clause);
    let source_literals: Vec<bool> = clause
        .literals()
        .as_slice()
        .iter()
        .map(|literal| is_local_rw_source(literal, bank))
        .collect();
    let true_term = bank.true_term().clone();
    let false_term = bank.false_term().clone();
    let mut modified = false;

    for (literal, is_source) in clause
        .literals_mut()
        .as_mut_slice()
        .iter_mut()
        .zip(source_literals)
    {
        if is_source {
            continue;
        }

        let left = replace_term(&rw_sys, bank, literal.left())?;
        let right = replace_term(&rw_sys, bank, literal.right())?;
        modified |= map_literal_terms(literal, &true_term, &false_term, left, right);
    }

    if modified {
        clause.recompute_lit_counts();
        let _ = clause_remove_superfluous_literals(clause, bank);
        clause.del_prop(CP_IS_ORIENTED);
        clause.set_weight(clause.standard_weight());
    }

    Ok(modified)
}

fn collect_local_rw_system(bank: &TermBank, clause: &Clause) -> LocalRwSystem {
    let mut rw_sys = LocalRwSystem::new();

    for literal in clause.literals().as_slice() {
        if literal.is_negative() && literal.is_oriented() {
            rw_sys.insert(term_identity_id(literal.left()), literal.right().clone());
        } else if !literal.is_equ_lit(bank) && literal.is_positive() {
            debug_assert_eq!(literal.right(), bank.true_term());
            rw_sys.insert(term_identity_id(literal.left()), bank.false_term().clone());
        }
    }

    rw_sys
}

fn is_local_rw_source(literal: &Eqn, bank: &TermBank) -> bool {
    (literal.is_negative() && literal.is_oriented())
        || (!literal.is_equ_lit(bank) && literal.is_positive())
}

fn replace_term(
    rw_sys: &LocalRwSystem,
    bank: &mut TermBank,
    term: &Term,
) -> Result<Term, Diagnostic> {
    bank.map_term(term, &mut |_, candidate| {
        Ok(Some(
            rw_sys
                .get(&term_identity_id(candidate))
                .cloned()
                .unwrap_or_else(|| candidate.clone()),
        ))
    })
}

fn map_literal_terms(
    literal: &mut Eqn,
    true_term: &Term,
    false_term: &Term,
    mut left: Term,
    mut right: Term,
) -> bool {
    let old_left = literal.left().clone();
    let old_right = literal.right().clone();
    let mut negate = false;

    if left == *false_term {
        left = true_term.clone();
        negate = !negate;
    }
    if right == *false_term {
        right = true_term.clone();
        negate = !negate;
    }
    if left == *true_term {
        std::mem::swap(&mut left, &mut right);
    }
    if right == *true_term {
        literal.del_prop(EP_IS_EQU_LITERAL);
    } else {
        literal.set_prop(EP_IS_EQU_LITERAL);
    }

    if negate {
        literal.flip_prop(EP_IS_POSITIVE);
    }

    if left != old_left {
        literal.del_prop(EP_MAX_IS_UP_TO_DATE);
        literal.del_prop(EP_IS_ORIENTED);
    }

    literal.set_left_raw(left);
    literal.set_right_raw(right);

    literal.left() != &old_left || literal.right() != &old_right
}

#[cfg(test)]
mod tests {
    use super::clause_local_rw;
    use crate::basics::partial_orderings::HoOrderKind;
    use crate::clauses::clause::Clause;
    use crate::clauses::clause_props::CP_IS_ORIENTED;
    use crate::clauses::eqn::Eqn;
    use crate::clauses::eqn_props::{EP_IS_ORIENTED, EP_IS_POSITIVE};
    use crate::clauses::eqnlist::EqnList;
    use crate::heuristics::to_params::TermOrdering;
    use crate::orderings::ocb::OrderControlBlock;
    use crate::terms::signature::Signature;
    use crate::terms::simpletypes::{alloc_arrow_type, Type};
    use crate::terms::termbanks::TermBank;
    use crate::terms::termtypes::{DerefType, Term};
    use crate::terms::typebanks::TypeBank;

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
        bank.create_const_term(f_code).unwrap()
    }

    fn typed_unary_with_return(
        bank: &mut TermBank,
        name: &str,
        arg_type: &Type,
        return_type: Type,
        arg: &Term,
    ) -> Term {
        let f_code = bank.signature_mut().insert_id(name, 1, false);
        let term_type = return_type.clone();
        if bank.signature().get_type(f_code).is_none() {
            let fun_type = bank
                .signature_mut()
                .type_bank_mut()
                .insert_type_shared(alloc_arrow_type(vec![arg_type.clone(), return_type]));
            bank.signature_mut()
                .declare_final_type(f_code, fun_type)
                .unwrap();
        }
        let term = Term::top_alloc(f_code, 1);
        term.set_type(Some(term_type));
        term.set_argument(0, arg.clone());
        bank.insert(&term, DerefType::Never).unwrap()
    }

    fn typed_unary(bank: &mut TermBank, name: &str, arg: &Term) -> Term {
        let type_ = bank.signature().type_bank().default_type();
        typed_unary_with_return(bank, name, &type_, type_.clone(), arg)
    }

    fn bool_predicate(bank: &mut TermBank, name: &str, arg: &Term) -> Term {
        let individual = bank.signature().type_bank().default_type();
        let bool_type = bank.signature().type_bank().bool_type();
        typed_unary_with_return(bank, name, &individual, bool_type, arg)
    }

    fn bool_unary(bank: &mut TermBank, name: &str, arg: &Term) -> Term {
        let bool_type = bank.signature().type_bank().bool_type();
        typed_unary_with_return(bank, name, &bool_type, bool_type.clone(), arg)
    }

    fn eqn(bank: &mut TermBank, left: &Term, right: &Term, positive: bool) -> Eqn {
        Eqn::alloc(left.clone(), right.clone(), bank, positive).unwrap()
    }

    fn kbo_ocb(bank: &TermBank) -> OrderControlBlock {
        OrderControlBlock::alloc(
            TermOrdering::Kbo,
            true,
            bank.signature(),
            HoOrderKind::LfhoOrder,
        )
    }

    #[test]
    fn negative_oriented_literal_rewrites_other_literals() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "local_rw_a");
        let c = typed_const(&mut bank, "local_rw_c");
        let f_a = typed_unary(&mut bank, "local_rw_f", &a);
        let g_f_a = typed_unary(&mut bank, "local_rw_g", &f_a);
        let g_a = typed_unary(&mut bank, "local_rw_g", &a);
        let rule = eqn(&mut bank, &f_a, &a, false);
        let target = eqn(&mut bank, &g_f_a, &c, true);
        let mut clause = Clause::alloc(EqnList::from_vec(vec![rule, target]));
        clause.set_prop(CP_IS_ORIENTED);
        clause.set_weight(999);

        let modified = clause_local_rw(&mut kbo_ocb(&bank), &mut bank, &mut clause).unwrap();

        assert!(modified);
        let rewritten = &clause.literals().as_slice()[0];
        assert_eq!(rewritten.left(), &g_a);
        assert_eq!(rewritten.right(), &c);
        assert!(!rewritten.query_prop(EP_IS_ORIENTED));
        assert!(!clause.query_prop(CP_IS_ORIENTED));
        assert_eq!(clause.weight(), clause.standard_weight());
        let source = &clause.literals().as_slice()[1];
        assert_eq!(source.left(), &f_a);
        assert_eq!(source.right(), &a);
        assert!(source.query_prop(EP_IS_ORIENTED));
    }

    #[test]
    fn positive_atom_rule_rewrites_equational_target_subterms_to_false() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "local_rw_bool_a");
        let p_a = bool_predicate(&mut bank, "local_rw_p", &a);
        let q_p_a = bool_unary(&mut bank, "local_rw_q", &p_a);
        let false_term = bank.false_term().clone();
        let true_term = bank.true_term().clone();
        let q_false = bool_unary(&mut bank, "local_rw_q", &false_term);
        assert!(p_a.type_().as_ref().is_some_and(Type::is_bool));
        assert_eq!(p_a.type_(), bank.true_term().type_());
        let source = Eqn::alloc(p_a.clone(), true_term.clone(), &mut bank, true).unwrap();
        let target = Eqn::alloc(q_p_a, q_false.clone(), &mut bank, true).unwrap();
        let mut clause = Clause::alloc(EqnList::from_vec(vec![source, target]));

        let modified = clause_local_rw(&mut kbo_ocb(&bank), &mut bank, &mut clause).unwrap();

        assert!(modified);
        let source = &clause.literals().as_slice()[0];
        assert_eq!(source.left(), &p_a);
        assert!(source.query_prop(EP_IS_POSITIVE));
        let rewritten = &clause.literals().as_slice()[1];
        assert_eq!(rewritten.left(), &q_false);
        assert_eq!(rewritten.right(), &q_false);
        assert!(rewritten.is_positive());
        assert_eq!(clause.weight(), clause.standard_weight());
    }

    #[test]
    fn returns_false_when_only_rule_sources_are_present() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "local_rw_only_a");
        let f_a = typed_unary(&mut bank, "local_rw_only_f", &a);
        let rule = eqn(&mut bank, &f_a, &a, false);
        let mut clause = Clause::alloc(EqnList::from_vec(vec![rule]));

        let modified = clause_local_rw(&mut kbo_ocb(&bank), &mut bank, &mut clause).unwrap();

        assert!(!modified);
        assert_eq!(clause.literals().as_slice()[0].left(), &f_a);
        assert_eq!(clause.literals().as_slice()[0].right(), &a);
    }
}
