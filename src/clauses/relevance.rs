use crate::basics::plist::{PListArena, PListHandle};
use crate::clauses::clause::Clause;
use crate::clauses::clausesets::ClauseSet;
use crate::clauses::findex::FIndex;
use crate::terms::functypes::FunCode;
use crate::terms::signature::Signature;

#[derive(Clone, Debug, PartialEq)]
pub struct RelevanceData {
    max_level: i64,
    f_code_relevance: Vec<i64>,
    clause_levels: Vec<Vec<Clause>>,
    clauses_rest: Vec<Clause>,
}

impl RelevanceData {
    #[must_use]
    pub fn compute(signature: &Signature, axioms: &ClauseSet) -> Self {
        let mut work = ClauseRelevanceWork::new(axioms);
        let mut f_code_relevance = vec![0; signature_f_limit(signature)];
        let mut level = 1;

        while !work.clauses.is_empty(work.clauses_core) {
            find_level_f_codes(
                signature,
                &work.clauses,
                work.clauses_core,
                level,
                &mut f_code_relevance,
                &mut work.new_codes,
            );

            work.relevance_levels.push(work.clauses_core);
            work.clauses_core = work.clauses.alloc_list();
            extract_new_core(&mut work);
            level += 1;
        }

        let clause_levels = work
            .relevance_levels
            .iter()
            .map(|&anchor| clone_list_clauses(&work.clauses, anchor))
            .collect();
        let clauses_rest = clone_list_clauses(&work.clauses, work.clauses_rest);

        Self {
            max_level: level,
            f_code_relevance,
            clause_levels,
            clauses_rest,
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

    #[must_use]
    pub fn clause_levels(&self) -> &[Vec<Clause>] {
        &self.clause_levels
    }

    #[must_use]
    pub fn clauses_rest(&self) -> &[Clause] {
        &self.clauses_rest
    }

    #[must_use]
    pub fn pruned_clause_set(&self, level: i64) -> ClauseSet {
        let mut result = ClauseSet::new();
        if level <= 0 {
            return result;
        }

        let requested = usize::try_from(level).unwrap_or(usize::MAX);
        for clauses in self.clause_levels.iter().take(requested) {
            for clause in clauses {
                result.insert(clause.clone());
            }
        }
        if requested > self.clause_levels.len() {
            for clause in &self.clauses_rest {
                result.insert(clause.clone());
            }
        }
        result
    }
}

#[must_use]
pub fn clause_set_relevance_prune(
    signature: &Signature,
    axioms: &ClauseSet,
    level: i64,
) -> (ClauseSet, i64) {
    if level == 0 {
        return (axioms.clone(), 0);
    }

    let reldata = RelevanceData::compute(signature, axioms);
    let pruned = reldata.pruned_clause_set(level);
    let removed = axioms.members() - pruned.members();
    (pruned, removed)
}

#[derive(Clone, Debug, PartialEq)]
struct ClauseRelevanceWork {
    clauses: PListArena<Clause>,
    clauses_core: PListHandle,
    clauses_rest: PListHandle,
    clauses_index: FIndex,
    new_codes: Vec<FunCode>,
    relevance_levels: Vec<PListHandle>,
}

impl ClauseRelevanceWork {
    fn new(axioms: &ClauseSet) -> Self {
        let mut clauses = PListArena::new();
        let clauses_core = clauses.alloc_list();
        let clauses_rest = clauses.alloc_list();
        let mut conjecture_count = 0;

        for clause in axioms.iter() {
            if clause.is_conjecture() {
                store_clause_after_anchor(&mut clauses, clauses_core, clause);
                conjecture_count += 1;
            } else {
                store_clause_after_anchor(&mut clauses, clauses_rest, clause);
            }
        }

        let mut clauses_index = FIndex::new();
        let _ = clauses_index.add_pl_clause_set(&clauses, clauses_rest);

        Self {
            clauses,
            clauses_core,
            clauses_rest,
            clauses_index,
            new_codes: Vec::new(),
            relevance_levels: Vec::with_capacity(conjecture_count),
        }
    }
}

fn find_level_f_codes(
    signature: &Signature,
    clauses: &PListArena<Clause>,
    clauses_core: PListHandle,
    level: i64,
    f_code_relevance: &mut Vec<i64>,
    new_codes: &mut Vec<FunCode>,
) {
    debug_assert!(new_codes.is_empty());
    for (_handle, clause) in clauses.entries(clauses_core) {
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
                new_codes.push(f_code);
            }
        }
    }
}

fn extract_new_core(work: &mut ClauseRelevanceWork) {
    while let Some(f_code) = work.new_codes.pop() {
        while let Some(entry) = work.clauses_index.first_pl_clause(f_code) {
            let _ = work.clauses_index.remove_pl_clause(&work.clauses, entry);
            let _ = work.clauses.extract(entry);
            work.clauses.insert_after(work.clauses_core, entry);
        }
    }
}

fn clone_list_clauses(clauses: &PListArena<Clause>, anchor: PListHandle) -> Vec<Clause> {
    clauses
        .entries(anchor)
        .into_iter()
        .map(|(_handle, clause)| clause.clone())
        .collect()
}

fn store_clause_after_anchor(
    clauses: &mut PListArena<Clause>,
    anchor: PListHandle,
    clause: &Clause,
) {
    let _stored = clauses.store_after(anchor, clause.clone());
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
    use super::{clause_set_relevance_prune, RelevanceData};
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

    fn clause_ids(clauses: &[Clause]) -> Vec<i64> {
        clauses.iter().map(Clause::ident).collect()
    }

    fn set_ids(clauses: &ClauseSet) -> Vec<i64> {
        clauses.iter().map(Clause::ident).collect()
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

    #[test]
    fn relevance_data_retains_c_style_clause_levels_and_rest() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a", false);
        let b = typed_const(&mut bank, "b", false);
        let c = typed_const(&mut bank, "c", false);
        let f_of_a = typed_unary(&mut bank, "f", &a);
        let f_of_b = typed_unary(&mut bank, "f", &b);
        let g_of_b = typed_unary(&mut bank, "g", &b);
        let unrelated = typed_unary(&mut bank, "u", &c);
        let conjecture = unit_clause(&mut bank, &f_of_a, &a, CP_TYPE_NEG_CONJECTURE);
        let relevant = unit_clause(&mut bank, &f_of_b, &g_of_b, CP_TYPE_AXIOM);
        let irrelevant = unit_clause(&mut bank, &unrelated, &c, CP_TYPE_AXIOM);
        let conjecture_id = conjecture.ident();
        let relevant_id = relevant.ident();
        let irrelevant_id = irrelevant.ident();

        let data = RelevanceData::compute(
            bank.signature(),
            &ClauseSet::from_clauses([conjecture, relevant, irrelevant]),
        );

        assert_eq!(data.clause_levels().len(), 2);
        assert_eq!(clause_ids(&data.clause_levels()[0]), vec![conjecture_id]);
        assert_eq!(clause_ids(&data.clause_levels()[1]), vec![relevant_id]);
        assert_eq!(clause_ids(data.clauses_rest()), vec![irrelevant_id]);
    }

    #[test]
    fn relevance_pruning_keeps_requested_levels_and_reports_removed_count() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a", false);
        let b = typed_const(&mut bank, "b", false);
        let c = typed_const(&mut bank, "c", false);
        let f_of_a = typed_unary(&mut bank, "f", &a);
        let f_of_b = typed_unary(&mut bank, "f", &b);
        let g_of_b = typed_unary(&mut bank, "g", &b);
        let unrelated = typed_unary(&mut bank, "u", &c);
        let conjecture = unit_clause(&mut bank, &f_of_a, &a, CP_TYPE_NEG_CONJECTURE);
        let relevant = unit_clause(&mut bank, &f_of_b, &g_of_b, CP_TYPE_AXIOM);
        let irrelevant = unit_clause(&mut bank, &unrelated, &c, CP_TYPE_AXIOM);
        let conjecture_id = conjecture.ident();
        let relevant_id = relevant.ident();
        let irrelevant_id = irrelevant.ident();
        let axioms = ClauseSet::from_clauses([conjecture, relevant, irrelevant]);

        let (level_one, removed) = clause_set_relevance_prune(bank.signature(), &axioms, 1);
        assert_eq!(set_ids(&level_one), vec![conjecture_id]);
        assert_eq!(removed, 2);

        let (level_two, removed) = clause_set_relevance_prune(bank.signature(), &axioms, 2);
        assert_eq!(set_ids(&level_two), vec![conjecture_id, relevant_id]);
        assert_eq!(removed, 1);

        let (all, removed) = clause_set_relevance_prune(bank.signature(), &axioms, 99);
        assert_eq!(
            set_ids(&all),
            vec![conjecture_id, relevant_id, irrelevant_id]
        );
        assert_eq!(removed, 0);
    }

    #[test]
    fn relevance_pruning_level_zero_returns_original_set() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a", false);
        let conjecture = unit_clause(&mut bank, &a, &a, CP_TYPE_NEG_CONJECTURE);
        let axiom = unit_clause(&mut bank, &a, &a, CP_TYPE_AXIOM);
        let original_ids = vec![conjecture.ident(), axiom.ident()];
        let axioms = ClauseSet::from_clauses([conjecture, axiom]);

        let (same, removed) = clause_set_relevance_prune(bank.signature(), &axioms, 0);

        assert_eq!(set_ids(&same), original_ids);
        assert_eq!(removed, 0);
    }
}
