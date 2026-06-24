use crate::clauses::clause::Clause;
use crate::clauses::clausesets::ClauseSet;
use crate::terms::functypes::FunCode;
use crate::terms::signature::Signature;
use std::collections::BTreeSet;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RelevanceData {
    max_level: i64,
    f_code_relevance: Vec<i64>,
}

impl RelevanceData {
    #[must_use]
    pub fn compute(signature: &Signature, axioms: &ClauseSet) -> Self {
        let mut core = Vec::new();
        let mut rest = Vec::new();
        for clause in axioms.iter() {
            if clause.is_conjecture() {
                core.push(clause.clone());
            } else {
                rest.push(clause.clone());
            }
        }

        let mut f_code_relevance = vec![0; signature_f_limit(signature)];
        let mut level = 1;
        while !core.is_empty() {
            let new_codes = find_level_f_codes(signature, &core, level, &mut f_code_relevance);
            let (next_core, next_rest) = extract_new_core(rest, &new_codes);
            core = next_core;
            rest = next_rest;
            level += 1;
        }

        Self {
            max_level: level,
            f_code_relevance,
        }
    }

    #[must_use]
    pub const fn max_level(&self) -> i64 {
        self.max_level
    }

    #[must_use]
    pub fn f_code_relevance(&self, f_code: FunCode) -> i64 {
        usize::try_from(f_code)
            .ok()
            .and_then(|index| self.f_code_relevance.get(index))
            .copied()
            .unwrap_or(0)
    }
}

fn find_level_f_codes(
    signature: &Signature,
    core: &[Clause],
    level: i64,
    f_code_relevance: &mut Vec<i64>,
) -> BTreeSet<FunCode> {
    let mut new_codes = BTreeSet::new();
    for clause in core {
        let mut f_codes = Vec::new();
        clause.return_fcodes(&mut f_codes);
        for f_code in f_codes {
            if signature.is_special(f_code) {
                continue;
            }
            let index = f_code_index(f_code);
            if index >= f_code_relevance.len() {
                f_code_relevance.resize(index + 1, 0);
            }
            if f_code_relevance[index] == 0 {
                f_code_relevance[index] = level;
                new_codes.insert(f_code);
            }
        }
    }
    new_codes
}

fn extract_new_core(
    rest: Vec<Clause>,
    new_codes: &BTreeSet<FunCode>,
) -> (Vec<Clause>, Vec<Clause>) {
    let mut core = Vec::new();
    let mut remaining = Vec::new();
    for clause in rest {
        if clause_mentions_any(&clause, new_codes) {
            core.push(clause);
        } else {
            remaining.push(clause);
        }
    }
    (core, remaining)
}

fn clause_mentions_any(clause: &Clause, f_codes: &BTreeSet<FunCode>) -> bool {
    if f_codes.is_empty() {
        return false;
    }
    let mut clause_f_codes = Vec::new();
    clause.return_fcodes(&mut clause_f_codes);
    clause_f_codes
        .into_iter()
        .any(|f_code| f_codes.contains(&f_code))
}

fn signature_f_limit(signature: &Signature) -> usize {
    usize::try_from(signature.f_count() + 1)
        .unwrap_or_else(|_| panic!("signature f-count must fit relevance vector length"))
}

fn f_code_index(f_code: FunCode) -> usize {
    usize::try_from(f_code).unwrap_or_else(|_| panic!("positive f-code must fit vector index"))
}

#[cfg(test)]
mod tests {
    use super::RelevanceData;
    use crate::clauses::clause::Clause;
    use crate::clauses::clause_props::{FormulaProperties, CP_TYPE_AXIOM, CP_TYPE_NEG_CONJECTURE};
    use crate::clauses::clausesets::ClauseSet;
    use crate::clauses::eqn::Eqn;
    use crate::clauses::eqnlist::EqnList;
    use crate::terms::signature::Signature;
    use crate::terms::simpletypes::alloc_arrow_type;
    use crate::terms::termbanks::TermBank;
    use crate::terms::termtypes::{DerefType, Term};
    use crate::terms::typebanks::TypeBank;

    fn test_bank() -> TermBank {
        let mut signature = Signature::new(TypeBank::new());
        signature.insert_internal_codes().unwrap();
        TermBank::new(signature).unwrap()
    }

    fn typed_const(bank: &mut TermBank, name: &str, special: bool) -> Term {
        let type_ = bank.signature().type_bank().default_type();
        let f_code = bank.signature_mut().insert_id(name, 0, special);
        bank.signature_mut()
            .declare_final_type(f_code, type_.clone())
            .unwrap();
        let term = Term::const_cell_alloc(f_code);
        term.set_type(Some(type_));
        bank.insert(&term, DerefType::Never).unwrap()
    }

    fn typed_unary(bank: &mut TermBank, name: &str, arg: &Term) -> Term {
        let type_ = bank.signature().type_bank().default_type();
        let arrow = bank
            .signature_mut()
            .type_bank_mut()
            .insert_type_shared(alloc_arrow_type(vec![type_.clone(), type_.clone()]));
        let f_code = bank.signature_mut().insert_id(name, 1, false);
        bank.signature_mut()
            .declare_final_type(f_code, arrow)
            .unwrap();
        let term = Term::top_alloc(f_code, 1);
        term.set_type(Some(type_));
        term.set_argument(0, arg.clone());
        bank.insert(&term, DerefType::Never).unwrap()
    }

    fn unit_clause(
        bank: &mut TermBank,
        left: &Term,
        right: &Term,
        tptp_type: FormulaProperties,
    ) -> Clause {
        let literal = Eqn::alloc(left.clone(), right.clone(), bank, true).unwrap();
        let mut clause = Clause::alloc(EqnList::from_vec(vec![literal]));
        clause.set_tptp_type(tptp_type);
        clause
    }

    fn f_code(bank: &TermBank, name: &str) -> i64 {
        bank.signature().find_f_code(name)
    }

    #[test]
    fn relevance_data_expands_from_conjectures_by_shared_symbols() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a", false);
        let b = typed_const(&mut bank, "b", false);
        let c = typed_const(&mut bank, "c", false);
        let f_of_a = typed_unary(&mut bank, "f", &a);
        let f_of_b = typed_unary(&mut bank, "f", &b);
        let g_of_b = typed_unary(&mut bank, "g", &b);
        let g_of_c = typed_unary(&mut bank, "g", &c);
        let h_of_c = typed_unary(&mut bank, "h", &c);

        let conjecture = unit_clause(&mut bank, &f_of_a, &a, CP_TYPE_NEG_CONJECTURE);
        let first_rest = unit_clause(&mut bank, &f_of_b, &g_of_b, CP_TYPE_AXIOM);
        let second_rest = unit_clause(&mut bank, &g_of_c, &h_of_c, CP_TYPE_AXIOM);
        let data = RelevanceData::compute(
            bank.signature(),
            &ClauseSet::from_clauses([conjecture, first_rest, second_rest]),
        );

        assert_eq!(data.f_code_relevance(f_code(&bank, "f")), 1);
        assert_eq!(data.f_code_relevance(f_code(&bank, "a")), 1);
        assert_eq!(data.f_code_relevance(f_code(&bank, "g")), 2);
        assert_eq!(data.f_code_relevance(f_code(&bank, "b")), 2);
        assert_eq!(data.f_code_relevance(f_code(&bank, "h")), 3);
        assert_eq!(data.f_code_relevance(f_code(&bank, "c")), 3);
        assert_eq!(data.max_level(), 4);
    }

    #[test]
    fn relevance_data_skips_special_symbols() {
        let mut bank = test_bank();
        let special = typed_const(&mut bank, "special", true);
        let other = typed_const(&mut bank, "other", false);
        let conjecture = unit_clause(&mut bank, &special, &special, CP_TYPE_NEG_CONJECTURE);
        let rest = unit_clause(&mut bank, &special, &other, CP_TYPE_AXIOM);
        let data = RelevanceData::compute(
            bank.signature(),
            &ClauseSet::from_clauses([conjecture, rest]),
        );

        assert_eq!(data.f_code_relevance(f_code(&bank, "special")), 0);
        assert_eq!(data.f_code_relevance(f_code(&bank, "other")), 0);
        assert_eq!(data.max_level(), 2);
    }
}
