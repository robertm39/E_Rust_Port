use crate::basics::pstacks::PStack;
use crate::basics::sysdate::SysDate;
use crate::clauses::clause::Clause;
use crate::clauses::clause_props::{
    FormulaProperties, CP_DELETE_CLAUSE, CP_IS_SOS, CP_TYPE_CONJECTURE,
};
use crate::clauses::clausepos::ClausePos;
use crate::clauses::eqn_props::EqnSide;
use crate::terms::functypes::FunCode;
use crate::terms::signature::Signature;
use crate::terms::termbanks::TermBank;
use crate::terms::termfunc::term_compute_order;
use crate::terms::termtypes::TermProperties;
use std::cmp::Ordering;
use std::collections::{BTreeMap, VecDeque};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClauseSet {
    clauses: VecDeque<Clause>,
    literals: i64,
    date: SysDate,
    identifier: String,
}

impl Default for ClauseSet {
    fn default() -> Self {
        Self::new()
    }
}

impl ClauseSet {
    #[must_use]
    pub fn new() -> Self {
        let mut date = SysDate::creation_time();
        let _ = date.increment();
        Self {
            clauses: VecDeque::new(),
            literals: 0,
            date,
            identifier: String::new(),
        }
    }

    #[must_use]
    pub fn from_clauses(clauses: impl IntoIterator<Item = Clause>) -> Self {
        let mut set = Self::new();
        for clause in clauses {
            set.insert(clause);
        }
        set
    }

    #[must_use]
    pub fn into_clauses(self) -> Vec<Clause> {
        self.clauses.into_iter().collect()
    }

    #[must_use]
    pub const fn date(&self) -> SysDate {
        self.date
    }

    pub const fn set_date(&mut self, date: SysDate) {
        self.date = date;
    }

    #[must_use]
    pub fn identifier(&self) -> &str {
        &self.identifier
    }

    pub fn set_identifier(&mut self, identifier: impl Into<String>) {
        self.identifier = identifier.into();
    }

    #[must_use]
    pub fn members(&self) -> i64 {
        usize_to_i64(self.clauses.len())
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.clauses.len()
    }

    #[must_use]
    pub const fn literals(&self) -> i64 {
        self.literals
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.clauses.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Clause> {
        self.clauses.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Clause> {
        self.clauses.iter_mut()
    }

    pub fn insert(&mut self, clause: Clause) {
        self.literals += usize_to_i64(clause.literal_number());
        self.clauses.push_back(clause);
    }

    pub fn insert_set(&mut self, source: &mut Self) -> i64 {
        let mut moved = 0;
        while let Some(clause) = source.extract_first() {
            self.insert(clause);
            moved += 1;
        }
        moved
    }

    pub fn extract_first(&mut self) -> Option<Clause> {
        let clause = self.clauses.pop_front()?;
        self.literals -= usize_to_i64(clause.literal_number());
        Some(clause)
    }

    pub fn extract_by_id(&mut self, ident: i64) -> Option<Clause> {
        let position = self.position_by_id(ident)?;
        let clause = self.clauses.remove(position)?;
        self.literals -= usize_to_i64(clause.literal_number());
        Some(clause)
    }

    pub fn delete_by_id(&mut self, ident: i64) -> bool {
        self.extract_by_id(ident).is_some()
    }

    #[must_use]
    pub fn find_same(&self, clause: &Clause) -> Option<&Clause> {
        self.clauses
            .iter()
            .find(|candidate| std::ptr::eq(*candidate, clause))
    }

    #[must_use]
    pub fn find_by_id(&self, ident: i64) -> Option<&Clause> {
        self.clauses.iter().find(|clause| clause.ident() == ident)
    }

    pub fn find_by_id_mut(&mut self, ident: i64) -> Option<&mut Clause> {
        self.clauses
            .iter_mut()
            .find(|clause| clause.ident() == ident)
    }

    pub fn sort_by<F>(&mut self, mut compare: F)
    where
        F: FnMut(&Clause, &Clause) -> Ordering,
    {
        for clause in &mut self.clauses {
            clause.set_weight(clause.standard_weight());
        }
        self.clauses
            .make_contiguous()
            .sort_unstable_by(|left, right| compare(left, right));
    }

    pub fn sort_literals_by<F>(&mut self, mut compare: F)
    where
        F: FnMut(&crate::clauses::eqn::Eqn, &crate::clauses::eqn::Eqn) -> i64,
    {
        for clause in &mut self.clauses {
            clause.sort_literals_by(&mut compare);
        }
    }

    pub fn set_prop(&mut self, prop: FormulaProperties) {
        for clause in &mut self.clauses {
            clause.set_prop(prop);
        }
    }

    pub fn del_prop(&mut self, prop: FormulaProperties) {
        for clause in &mut self.clauses {
            clause.del_prop(prop);
        }
    }

    pub fn set_tptp_type(&mut self, type_: FormulaProperties) {
        for clause in &mut self.clauses {
            clause.set_tptp_type(type_);
        }
    }

    pub fn mark_copies(&mut self) -> i64 {
        let mut marked = 0;
        for index in 0..self.clauses.len() {
            let duplicate = (0..index)
                .any(|previous| self.clauses[previous].compare_fun(&self.clauses[index]) == 0);
            if duplicate {
                self.clauses[index].set_prop(CP_DELETE_CLAUSE);
                marked += 1;
            }
        }
        marked
    }

    pub fn delete_marked_entries(&mut self) -> i64 {
        let mut deleted = 0;
        let mut kept = VecDeque::with_capacity(self.clauses.len());
        while let Some(clause) = self.clauses.pop_front() {
            if clause.query_prop(CP_DELETE_CLAUSE) {
                deleted += 1;
            } else {
                kept.push_back(clause);
            }
        }
        self.clauses = kept;
        self.recompute_literals();
        deleted
    }

    pub fn delete_copies(&mut self) -> i64 {
        let marked = self.mark_copies();
        let deleted = self.delete_marked_entries();
        debug_assert_eq!(marked, deleted);
        marked
    }

    pub fn delete_non_units(&mut self) -> i64 {
        for clause in &mut self.clauses {
            if clause.literal_number() > 1 {
                clause.set_prop(CP_DELETE_CLAUSE);
            } else {
                clause.del_prop(CP_DELETE_CLAUSE);
            }
        }
        self.delete_marked_entries()
    }

    #[must_use]
    pub fn term_nodes(&self, bank: &TermBank) -> i64 {
        self.clauses
            .iter()
            .map(|clause| {
                clause_weight_to_i64(clause.literal_weight(bank, 1.0, 1.0, 1.0, 1, 1, 1.0, true))
            })
            .sum()
    }

    pub fn mark_sos(&mut self, tptp_types: bool) -> i64 {
        let mut result = 0;
        for clause in &mut self.clauses {
            if (tptp_types && clause.query_tptp_type() == CP_TYPE_CONJECTURE)
                || (!tptp_types && clause.is_goal())
            {
                clause.set_prop(CP_IS_SOS);
                result += 1;
            } else {
                clause.del_prop(CP_IS_SOS);
            }
        }
        result
    }

    pub fn term_set_prop(&self, prop: TermProperties) {
        for clause in &self.clauses {
            clause.term_set_prop(prop);
        }
    }

    #[must_use]
    pub fn tb_term_prop_del_count(&self, prop: TermProperties) -> i64 {
        self.clauses
            .iter()
            .map(|clause| clause.tb_term_del_prop_count(prop))
            .sum()
    }

    #[must_use]
    pub fn shared_term_nodes(&self) -> i64 {
        self.term_set_prop(crate::terms::termtypes::TP_OP_FLAG);
        self.tb_term_prop_del_count(crate::terms::termtypes::TP_OP_FLAG)
    }

    pub fn add_symbol_distribution(&self, dist_array: &mut [i64]) {
        for clause in &self.clauses {
            clause.add_symbol_distribution(dist_array);
        }
    }

    pub fn add_type_distribution(&self, sig: &mut Signature, type_array: &mut [i64]) {
        for clause in &self.clauses {
            clause.add_type_distribution(sig, type_array);
        }
    }

    pub fn add_conj_symbol_distribution(&self, dist_array: &mut [i64]) {
        for clause in &self.clauses {
            if clause.is_conjecture() {
                clause.add_symbol_distribution(dist_array);
            }
        }
    }

    pub fn add_axiom_symbol_distribution(&self, dist_array: &mut [i64]) {
        for clause in &self.clauses {
            if !clause.is_conjecture() {
                clause.add_symbol_distribution(dist_array);
            }
        }
    }

    pub fn compute_function_ranks(&self, rank_array: &mut [i64], count: &mut i64) {
        for clause in &self.clauses {
            clause.compute_function_ranks(rank_array, count);
        }
    }

    #[must_use]
    pub fn find_freq_symbol(&self, sig: &Signature, arity: i32, least: bool) -> FunCode {
        let Some(dist_size) = sig
            .f_count()
            .checked_add(1)
            .and_then(|size| usize::try_from(size).ok())
        else {
            return 0;
        };
        let mut dist_array = vec![0; dist_size];
        self.add_symbol_distribution(&mut dist_array);

        let mut selected = 0;
        let mut frequency = if least { i64::MAX } else { 0 };
        for f_code in (sig.internal_symbols() + 1)..=sig.f_count() {
            if sig.find_arity(f_code) == Some(arity)
                && !sig.is_predicate(f_code)
                && !sig.is_special(f_code)
            {
                let Some(index) = f_code_index(f_code) else {
                    continue;
                };
                let symbol_frequency = dist_array[index];
                if (least && symbol_frequency <= frequency)
                    || (!least && symbol_frequency >= frequency)
                {
                    frequency = symbol_frequency;
                    selected = f_code;
                }
            }
        }
        selected
    }

    #[must_use]
    pub fn max_var_number(&self) -> i64 {
        self.clauses
            .iter()
            .map(|clause| {
                let mut variables = BTreeMap::new();
                clause.collect_variables(&mut variables)
            })
            .max()
            .unwrap_or(0)
    }

    #[must_use]
    pub fn standard_weight(&self) -> i64 {
        self.clauses.iter().map(Clause::standard_weight).sum()
    }

    pub fn default_weigh_clauses(&mut self) {
        for clause in &mut self.clauses {
            clause.set_weight(clause.standard_weight());
        }
    }

    #[must_use]
    pub fn find_max_standard_weight(&self) -> Option<&Clause> {
        let mut max_weight = 0;
        let mut result = None;
        for clause in &self.clauses {
            let weight = clause.standard_weight();
            if weight > max_weight {
                max_weight = weight;
                result = Some(clause);
            }
        }
        result
    }

    #[must_use]
    pub fn find_eq_definition(&self, bank: &TermBank, min_arity: usize) -> Option<ClausePos> {
        self.find_eq_definition_from_index(bank, min_arity, 0)
    }

    #[must_use]
    pub fn find_eq_definition_from_id(
        &self,
        bank: &TermBank,
        min_arity: usize,
        start_ident: i64,
    ) -> Option<ClausePos> {
        let start = self.position_by_id(start_ident)?;
        self.find_eq_definition_from_index(bank, min_arity, start)
    }

    pub fn push_clause_refs<'a>(&'a self, stack: &mut PStack<&'a Clause>) -> i64 {
        let mut pushed = 0;
        for clause in &self.clauses {
            stack.push(clause);
            pushed += 1;
        }
        pushed
    }

    pub fn split_conjecture_refs<'a>(
        &'a self,
        conjectures: &mut Vec<&'a Clause>,
        rest: &mut Vec<&'a Clause>,
    ) -> i64 {
        let mut found = 0;
        for clause in &self.clauses {
            if clause.is_conjecture() {
                conjectures.push(clause);
                found += 1;
            } else {
                rest.push(clause);
            }
        }
        found
    }

    pub fn count_conjectures(&self, hypos: &mut i64) -> i64 {
        let mut conjectures = 0;
        for clause in &self.clauses {
            if clause.is_conjecture() {
                conjectures += 1;
            }
            if clause.is_hypothesis() {
                *hypos += 1;
            }
        }
        conjectures
    }

    #[must_use]
    pub fn conjecture_order(&self, sig: &Signature) -> usize {
        let mut order = 0;
        for clause in &self.clauses {
            for literal in clause.literals().as_slice() {
                order = order.max(term_compute_order(sig, literal.left()));
                order = order.max(term_compute_order(sig, literal.right()));
            }
        }
        order
    }

    #[must_use]
    pub fn is_untyped(&self) -> bool {
        self.clauses.iter().all(Clause::is_untyped)
    }

    fn find_eq_definition_from_index(
        &self,
        bank: &TermBank,
        min_arity: usize,
        start: usize,
    ) -> Option<ClausePos> {
        for clause in self.clauses.iter().skip(start) {
            let side = clause.is_eq_definition(bank, min_arity);
            if side != EqnSide::NoSide {
                let mut pos = ClausePos::for_clause(clause.clone());
                pos.set_side(side);
                return Some(pos);
            }
        }
        None
    }

    fn position_by_id(&self, ident: i64) -> Option<usize> {
        self.clauses
            .iter()
            .position(|clause| clause.ident() == ident)
    }

    fn recompute_literals(&mut self) {
        self.literals = self
            .clauses
            .iter()
            .map(|clause| usize_to_i64(clause.literal_number()))
            .sum();
    }
}

#[must_use]
pub fn clause_set_stack_cardinality(stack: &PStack<ClauseSet>) -> i64 {
    stack.as_slice().iter().map(ClauseSet::members).sum()
}

#[must_use]
pub fn clause_set_ref_stack_cardinality(stack: &PStack<&ClauseSet>) -> i64 {
    stack.as_slice().iter().map(|set| set.members()).sum()
}

fn usize_to_i64(value: usize) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

fn f_code_index(f_code: FunCode) -> Option<usize> {
    usize::try_from(f_code).ok()
}

#[allow(clippy::cast_possible_truncation)]
fn clause_weight_to_i64(weight: f64) -> i64 {
    weight as i64
}

#[cfg(test)]
mod tests {
    use super::{clause_set_ref_stack_cardinality, clause_set_stack_cardinality, ClauseSet};
    use crate::basics::pstacks::PStack;
    use crate::basics::sysdate::SysDate;
    use crate::clauses::clause::Clause;
    use crate::clauses::clause_props::{
        CP_DELETE_CLAUSE, CP_INITIAL, CP_IS_SOS, CP_TYPE_AXIOM, CP_TYPE_CONJECTURE,
        CP_TYPE_HYPOTHESIS, CP_TYPE_NEG_CONJECTURE,
    };
    use crate::clauses::eqn::Eqn;
    use crate::clauses::eqn_props::EqnSide;
    use crate::clauses::eqnlist::EqnList;
    use crate::terms::signature::Signature;
    use crate::terms::simpletypes::alloc_arrow_type;
    use crate::terms::termbanks::TermBank;
    use crate::terms::termtypes::{DerefType, Term, TP_CHECK_FLAG};
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

    fn typed_var(bank: &TermBank, f_code: i64) -> Term {
        let type_ = bank.signature().type_bank().default_type();
        bank.vars().var_assert_alloc(f_code, &type_)
    }

    fn typed_unary(bank: &mut TermBank, name: &str, arg: &Term) -> Term {
        let type_ = bank.signature().type_bank().default_type();
        let f_code = bank.signature_mut().insert_id(name, 1, false);
        if bank.signature().get_type(f_code).is_none() {
            bank.signature_mut()
                .declare_final_type(f_code, alloc_arrow_type(vec![type_.clone(), type_.clone()]))
                .unwrap();
        }
        let term = Term::top_alloc(f_code, 1);
        term.set_type(Some(type_));
        term.set_argument(0, arg.clone());
        bank.insert(&term, DerefType::Never).unwrap()
    }

    fn literal(bank: &mut TermBank, left: &Term, right: &Term, positive: bool) -> Eqn {
        Eqn::alloc(left.clone(), right.clone(), bank, positive).unwrap()
    }

    fn clause_from(literals: Vec<Eqn>) -> Clause {
        let mut clause = Clause::alloc(EqnList::from_vec(literals));
        clause.set_weight(clause.standard_weight());
        clause
    }

    #[test]
    fn insert_extract_and_transfer_preserve_order_and_accounting() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let c = typed_const(&mut bank, "c");
        let first = clause_from(vec![literal(&mut bank, &a, &b, true)]);
        let second = clause_from(vec![
            literal(&mut bank, &b, &c, true),
            literal(&mut bank, &c, &a, false),
        ]);
        let first_id = first.ident();
        let second_id = second.ident();
        let mut set = ClauseSet::new();

        assert_eq!(set.date(), SysDate::from_raw(1));
        assert!(set.is_empty());
        set.insert(first);
        set.insert(second);

        assert_eq!(set.members(), 2);
        assert_eq!(set.len(), 2);
        assert_eq!(set.literals(), 3);
        assert_eq!(
            set.iter().map(Clause::ident).collect::<Vec<_>>(),
            vec![first_id, second_id]
        );
        assert_eq!(
            set.find_by_id(second_id).map(Clause::ident),
            Some(second_id)
        );

        let extracted = set.extract_first().unwrap();
        assert_eq!(extracted.ident(), first_id);
        assert_eq!(set.members(), 1);
        assert_eq!(set.literals(), 2);

        let mut target = ClauseSet::from_clauses([extracted]);
        assert_eq!(target.insert_set(&mut set), 1);
        assert!(set.is_empty());
        assert_eq!(
            target.iter().map(Clause::ident).collect::<Vec<_>>(),
            vec![first_id, second_id]
        );
        assert_eq!(target.literals(), 3);
    }

    #[test]
    fn delete_marked_non_units_and_copies_follow_plain_set_order() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let c = typed_const(&mut bank, "c");
        let unit = clause_from(vec![literal(&mut bank, &a, &b, true)]);
        let mut copy = unit.clone();
        copy.set_ident(unit.ident() + 1000);
        let non_unit = clause_from(vec![
            literal(&mut bank, &b, &c, true),
            literal(&mut bank, &c, &a, false),
        ]);
        let unit_id = unit.ident();
        let copy_id = copy.ident();
        let non_unit_id = non_unit.ident();
        let mut set = ClauseSet::from_clauses([unit, copy, non_unit]);

        assert_eq!(set.mark_copies(), 1);
        assert!(set
            .find_by_id(copy_id)
            .unwrap()
            .query_prop(CP_DELETE_CLAUSE));
        assert_eq!(set.delete_marked_entries(), 1);
        assert_eq!(
            set.iter().map(Clause::ident).collect::<Vec<_>>(),
            vec![unit_id, non_unit_id]
        );
        assert_eq!(set.literals(), 3);

        assert_eq!(set.delete_non_units(), 1);
        assert_eq!(
            set.iter().map(Clause::ident).collect::<Vec<_>>(),
            vec![unit_id]
        );
        assert_eq!(set.literals(), 1);

        let duplicate = set.find_by_id(unit_id).unwrap().clone();
        set.insert(duplicate);
        assert_eq!(set.delete_copies(), 1);
        assert_eq!(set.members(), 1);
    }

    #[test]
    fn set_properties_sos_and_conjecture_counts_match_c_rules() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let mut axiom = clause_from(vec![literal(&mut bank, &a, &b, true)]);
        let mut goal = clause_from(vec![literal(&mut bank, &a, &b, false)]);
        let mut conjecture = clause_from(vec![literal(&mut bank, &b, &a, true)]);
        let mut neg_conjecture = clause_from(vec![literal(&mut bank, &b, &a, false)]);
        axiom.set_tptp_type(CP_TYPE_AXIOM);
        goal.set_tptp_type(CP_TYPE_HYPOTHESIS);
        conjecture.set_tptp_type(CP_TYPE_CONJECTURE);
        neg_conjecture.set_tptp_type(CP_TYPE_NEG_CONJECTURE);
        let mut set = ClauseSet::from_clauses([axiom, goal, conjecture, neg_conjecture]);

        set.set_prop(CP_INITIAL);
        assert!(set.iter().all(|clause| clause.query_prop(CP_INITIAL)));
        set.del_prop(CP_INITIAL);
        assert!(set.iter().all(|clause| !clause.query_prop(CP_INITIAL)));
        set.set_tptp_type(CP_TYPE_AXIOM);
        assert!(set
            .iter()
            .all(|clause| clause.query_tptp_type() == CP_TYPE_AXIOM));

        let ids = set.iter().map(Clause::ident).collect::<Vec<_>>();
        set.find_by_id_mut(ids[1])
            .unwrap()
            .set_tptp_type(CP_TYPE_HYPOTHESIS);
        set.find_by_id_mut(ids[2])
            .unwrap()
            .set_tptp_type(CP_TYPE_CONJECTURE);
        set.find_by_id_mut(ids[3])
            .unwrap()
            .set_tptp_type(CP_TYPE_NEG_CONJECTURE);

        assert_eq!(set.mark_sos(false), 2);
        assert!(set.find_by_id(ids[1]).unwrap().query_prop(CP_IS_SOS));
        assert!(set.find_by_id(ids[3]).unwrap().query_prop(CP_IS_SOS));
        assert_eq!(set.mark_sos(true), 1);
        assert!(set.find_by_id(ids[2]).unwrap().query_prop(CP_IS_SOS));
        assert!(!set.find_by_id(ids[3]).unwrap().query_prop(CP_IS_SOS));

        let mut hypotheses = 10;
        assert_eq!(set.count_conjectures(&mut hypotheses), 2);
        assert_eq!(hypotheses, 11);

        let mut conjectures = Vec::new();
        let mut rest = Vec::new();
        assert_eq!(set.split_conjecture_refs(&mut conjectures, &mut rest), 2);
        assert_eq!(
            conjectures
                .iter()
                .map(|clause| clause.ident())
                .collect::<Vec<_>>(),
            vec![ids[2], ids[3]]
        );
        assert_eq!(
            rest.iter().map(|clause| clause.ident()).collect::<Vec<_>>(),
            vec![ids[0], ids[1]]
        );
    }

    #[test]
    fn aggregate_weights_variables_terms_and_stack_counts_use_clause_helpers() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let x = typed_var(&bank, -2);
        let fx = typed_unary(&mut bank, "f", &x);
        let unit = clause_from(vec![literal(&mut bank, &a, &b, true)]);
        let var_clause = clause_from(vec![literal(&mut bank, &fx, &a, false)]);
        let mut set = ClauseSet::from_clauses([unit, var_clause]);

        assert_eq!(
            set.standard_weight(),
            set.iter().map(Clause::standard_weight).sum()
        );
        set.iter_mut().for_each(|clause| clause.set_weight(-1));
        set.default_weigh_clauses();
        assert!(set
            .iter()
            .all(|clause| clause.weight() == clause.standard_weight()));
        assert_eq!(set.max_var_number(), 1);
        assert_eq!(set.find_max_standard_weight().map(Clause::weight), Some(5));
        assert!(set.term_nodes(&bank) > 0);

        assert!(set.tb_term_prop_del_count(TP_CHECK_FLAG) == 0);
        set.term_set_prop(TP_CHECK_FLAG);
        assert!(set.tb_term_prop_del_count(TP_CHECK_FLAG) > 0);
        assert!(set.shared_term_nodes() > 0);
        assert!(set.is_untyped());

        let mut owned_stack = PStack::new();
        owned_stack.push(set.clone());
        assert_eq!(clause_set_stack_cardinality(&owned_stack), 2);

        let mut ref_stack = PStack::new();
        ref_stack.push(&set);
        assert_eq!(clause_set_ref_stack_cardinality(&ref_stack), 2);

        let mut clause_stack = PStack::new();
        assert_eq!(set.push_clause_refs(&mut clause_stack), 2);
        assert_eq!(clause_stack.len(), 2);

        let default_type = bank.signature().type_bank().default_type();
        let arrow_type = bank
            .signature_mut()
            .type_bank_mut()
            .insert_type_shared(alloc_arrow_type(vec![default_type.clone(), default_type]));
        let higher_order = bank.vars().var_assert_alloc(-6, &arrow_type);
        set.insert(clause_from(vec![literal(
            &mut bank,
            &higher_order,
            &higher_order,
            true,
        )]));
        assert!(set.conjecture_order(bank.signature()) > 0);
    }

    #[test]
    fn type_distribution_forwards_clause_terms_to_signature_types() {
        let mut bank = test_bank();
        let default_type = bank.signature().type_bank().default_type();
        let unary_type = bank
            .signature_mut()
            .type_bank_mut()
            .insert_type_shared(alloc_arrow_type(vec![
                default_type.clone(),
                default_type.clone(),
            ]));
        let f_code = bank.signature_mut().insert_id("typed_f", 1, false);
        bank.signature_mut()
            .declare_final_type(f_code, unary_type.clone())
            .unwrap();
        let a_code = bank.signature_mut().insert_id("typed_a", 0, false);
        bank.signature_mut()
            .declare_final_type(a_code, default_type.clone())
            .unwrap();
        let a = bank.create_const_term(a_code).unwrap();
        let fa = Term::top_alloc(f_code, 1);
        fa.set_type(Some(default_type.clone()));
        fa.set_argument(0, a.clone());
        let fa = bank.insert(&fa, DerefType::Never).unwrap();
        let set = ClauseSet::from_clauses([clause_from(vec![literal(&mut bank, &fa, &a, true)])]);

        let mut type_dist =
            vec![0; usize::try_from(bank.signature().type_bank().types_count() + 1).unwrap()];
        set.add_type_distribution(bank.signature_mut(), &mut type_dist);

        assert_eq!(
            type_dist[usize::try_from(unary_type.type_uid()).unwrap()],
            1
        );
        assert_eq!(
            type_dist[usize::try_from(default_type.type_uid()).unwrap()],
            2
        );
    }

    #[test]
    fn frequency_symbol_selection_preserves_last_tie_wins_behavior() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let first = typed_unary(&mut bank, "f", &a);
        let second = typed_unary(&mut bank, "g", &a);
        let f_code = first.f_code();
        let g_code = second.f_code();
        let set = ClauseSet::from_clauses([clause_from(vec![
            literal(&mut bank, &first, &a, true),
            literal(&mut bank, &second, &a, true),
        ])]);

        let mut dist = vec![0; usize::try_from(bank.signature().f_count() + 1).unwrap()];
        set.add_symbol_distribution(&mut dist);
        assert_eq!(dist[usize::try_from(f_code).unwrap()], 1);
        assert_eq!(dist[usize::try_from(g_code).unwrap()], 1);

        assert_eq!(set.find_freq_symbol(bank.signature(), 1, false), g_code);
        assert_eq!(set.find_freq_symbol(bank.signature(), 1, true), g_code);
    }

    #[test]
    fn equality_definition_lookup_returns_reduced_clause_position_from_start() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let x = typed_var(&bank, -2);
        let fx = typed_unary(&mut bank, "f", &x);
        let non_definition = clause_from(vec![literal(&mut bank, &a, &b, true)]);
        let definition = clause_from(vec![literal(&mut bank, &fx, &a, true)]);
        let late_definition = clause_from(vec![literal(&mut bank, &fx, &b, true)]);
        let definition_id = definition.ident();
        let late_definition_id = late_definition.ident();
        let set = ClauseSet::from_clauses([non_definition, definition, late_definition]);

        let found = set.find_eq_definition(&bank, 1).unwrap();
        assert_eq!(found.clause().map(Clause::ident), Some(definition_id));
        assert_eq!(found.literal_index(), Some(0));
        assert_eq!(found.side(), EqnSide::LeftSide);
        assert!(found.term_pos().is_top_pos());

        let found_from_late = set
            .find_eq_definition_from_id(&bank, 1, late_definition_id)
            .unwrap();
        assert_eq!(
            found_from_late.clause().map(Clause::ident),
            Some(late_definition_id)
        );
        assert!(set.find_eq_definition_from_id(&bank, 1, i64::MAX).is_none());
    }
}
