use crate::basics::error::Diagnostic;
use crate::clauses::clausefunc::{
    clause_set_canonize, clause_set_remove_superfluous_literals,
    clause_set_replace_injectivity_defs,
};
use crate::clauses::clausesets::ClauseSet;
use crate::terms::termbanks::TermBank;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clauses::clause::Clause;
    use crate::clauses::eqn::Eqn;
    use crate::clauses::eqnlist::EqnList;
    use crate::terms::signature::Signature;
    use crate::terms::termbanks::TermBank;
    use crate::terms::termtypes::Term;
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

    fn literal(bank: &mut TermBank, left: &Term, right: &Term, positive: bool) -> Eqn {
        Eqn::alloc(left.clone(), right.clone(), bank, positive).unwrap()
    }

    fn clause(literals: Vec<Eqn>) -> Clause {
        let mut clause = Clause::alloc(EqnList::from_vec(literals));
        clause.set_weight(clause.standard_weight());
        clause
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
}
