use crate::basics::defines::DEFAULT_COMCHAR_RAW;
use crate::basics::objtrees::ObjTree;
use crate::basics::simple_stuff::ProblemType;
use crate::clauses::clause::Clause;
use crate::clauses::clausecpos::{clause_cpos_get_subterm, CompactPos};
use crate::clauses::clausepos_tree::ClauseTPosTree;
use crate::clauses::eqn::Eqn;
use crate::clauses::subterm_index::TermIdentitySet;
use crate::clauses::subterm_tree::SubtermOcc;
use crate::terms::fp_index::{FPIndex, FPTree};
use crate::terms::functypes::FunCode;
use crate::terms::idx_fp::FingerprintIndexFunction;
use crate::terms::signature::Signature;
use crate::terms::termbanks::TermBank;
use crate::terms::termfunc::term_standard_weight;
use crate::terms::termtypes::{term_identity_id, DerefType, Term, DEFAULT_FWEIGHT};
use std::collections::btree_map::Entry;
use std::fmt::{self, Write};

#[derive(Clone, Debug, PartialEq)]
pub struct OverlapTermPos {
    term: Term,
    pos: CompactPos,
}

impl OverlapTermPos {
    #[must_use]
    pub const fn new(term: Term, pos: CompactPos) -> Self {
        Self { term, pos }
    }

    #[must_use]
    pub const fn term(&self) -> &Term {
        &self.term
    }

    #[must_use]
    pub const fn pos(&self) -> CompactPos {
        self.pos
    }
}

pub struct OverlapIndex<'sig> {
    index: FPIndex<'sig, SubtermOcc>,
}

impl<'sig> OverlapIndex<'sig> {
    #[must_use]
    pub fn new(fp_fun: FingerprintIndexFunction, sig: &'sig Signature) -> Self {
        Self {
            index: FPIndex::new(fp_fun, sig),
        }
    }

    #[must_use]
    pub const fn root(&self) -> &FPTree<SubtermOcc> {
        self.index.root()
    }

    pub fn root_mut(&mut self) -> &mut FPTree<SubtermOcc> {
        self.index.root_mut()
    }

    #[must_use]
    pub fn find_leaf(&self, term: &Term) -> Option<&FPTree<SubtermOcc>> {
        self.index.find(term)
    }

    #[must_use]
    pub fn find_occurrence(&self, term: &Term) -> Option<&SubtermOcc> {
        self.index
            .find(term)
            .and_then(FPTree::payload)
            .and_then(|payload| payload.find(&SubtermOcc::new(term)))
    }

    #[must_use]
    pub fn find_unifiable_occurrences<'idx>(
        &'idx self,
        term: &Term,
        result: &mut Vec<&'idx SubtermOcc>,
    ) -> usize {
        let start = result.len();
        let mut leaves = Vec::new();
        self.index.find_unifiable(term, &mut leaves);
        for payload in leaves.into_iter().rev().flatten() {
            result.extend(payload.iter());
        }
        result.len() - start
    }

    #[must_use]
    pub fn collect_leaves<'idx>(&'idx self, result: &mut Vec<&'idx FPTree<SubtermOcc>>) -> usize {
        self.index.collect_leaves(result)
    }

    #[must_use]
    pub fn leaf_debug_string(&self, bank: &TermBank, problem_type: ProblemType) -> String {
        let mut output = String::new();
        let _ = self.write_leaf_debug(&mut output, bank, problem_type);
        output
    }

    pub fn write_leaf_debug(
        &self,
        output: &mut impl Write,
        bank: &TermBank,
        problem_type: ProblemType,
    ) -> fmt::Result {
        self.index.write_print_with(output, |path, leaf, output| {
            write_overlap_index_fp_leaf_debug(output, path, leaf, bank, problem_type)
        })
    }

    #[cfg(feature = "print-index-stats")]
    #[must_use]
    pub fn distrib_data_string(&self) -> String {
        self.index.collect_distrib().data_string()
    }

    #[cfg(feature = "print-index-stats")]
    pub fn write_distrib_data(&self, output: &mut impl Write) -> fmt::Result {
        self.index.collect_distrib().write_data(output)
    }

    #[cfg(feature = "print-index-stats")]
    #[must_use]
    pub fn dot_string<F>(&self, name: &str, print_payload: F) -> String
    where
        F: FnMut(&ObjTree<SubtermOcc>, &Signature) -> String,
    {
        self.index.dot_string(name, print_payload)
    }

    pub fn insert_pos(&mut self, clause: &Clause, pos: CompactPos, term: Option<&Term>) -> bool {
        let computed;
        let term = if let Some(term) = term {
            term
        } else {
            computed = clause_cpos_get_subterm(clause, pos);
            &computed
        };
        let leaf = self.index.insert(term);
        let payload = leaf.ensure_payload();
        let mut occurrence = payload
            .extract_object(&SubtermOcc::new(term))
            .unwrap_or_else(|| SubtermOcc::new(term));
        let inserted = occurrence.position_clauses_mut().insert_pos(clause, pos);
        let duplicate = payload.store(occurrence);
        debug_assert!(duplicate.is_none());
        inserted
    }

    pub fn delete_pos(&mut self, clause: &Clause, pos: CompactPos, term: Option<&Term>) -> bool {
        let computed;
        let term = if let Some(term) = term {
            term
        } else {
            computed = clause_cpos_get_subterm(clause, pos);
            &computed
        };
        self.delete_with(term, |occurrence| {
            occurrence.position_clauses_mut().delete_pos(clause, pos)
        })
    }

    pub fn delete_clause_occ(&mut self, clause: &Clause, term: &Term) -> bool {
        self.delete_with(term, |occurrence| {
            occurrence.position_clauses_mut().delete_clause(clause)
        })
    }

    pub fn insert_into_clause(&mut self, clause: &Clause) -> i64 {
        let mut collector = Vec::new();
        let count = clause_collect_into_terms_pos(clause, &mut collector);
        for entry in collector.iter().rev() {
            self.insert_pos(clause, entry.pos(), Some(entry.term()));
        }
        count
    }

    pub fn delete_into_clause(&mut self, clause: &Clause) -> i64 {
        let mut collector = TermIdentitySet::new();
        let count = clause_collect_into_terms(clause, &mut collector);
        for term in collector.values() {
            self.delete_clause_occ(clause, term);
        }
        count
    }

    pub fn insert_from_clause(&mut self, clause: &Clause) -> i64 {
        let mut collector = Vec::new();
        let count = clause_collect_from_terms_pos(clause, &mut collector);
        for entry in collector.iter().rev() {
            self.insert_pos(clause, entry.pos(), Some(entry.term()));
        }
        count
    }

    pub fn delete_from_clause(&mut self, clause: &Clause) -> i64 {
        let mut collector = TermIdentitySet::new();
        let count = clause_collect_from_terms(clause, &mut collector);
        for term in collector.values() {
            self.delete_clause_occ(clause, term);
        }
        count
    }

    fn delete_with(
        &mut self,
        term: &Term,
        delete_occurrence: impl FnOnce(&mut SubtermOcc) -> bool,
    ) -> bool {
        let (deleted, payload_empty) = {
            let Some(leaf) = self.index.find_mut(term) else {
                return false;
            };
            let Some(payload) = leaf.payload_mut() else {
                return false;
            };
            let Some(mut occurrence) = payload.extract_object(&SubtermOcc::new(term)) else {
                return false;
            };
            let deleted = delete_occurrence(&mut occurrence);
            if !occurrence.is_unused() {
                let duplicate = payload.store(occurrence);
                debug_assert!(duplicate.is_none());
            }
            let payload_empty = payload.is_empty();
            if payload_empty {
                leaf.clear_payload();
            }
            (deleted, payload_empty)
        };
        if payload_empty {
            self.index.delete(term);
        }
        deleted
    }
}

pub fn write_overlap_index_fp_leaf_debug(
    output: &mut impl Write,
    path: &[FunCode],
    leaf: &FPTree<SubtermOcc>,
    bank: &TermBank,
    problem_type: ProblemType,
) -> fmt::Result {
    write!(output, "{DEFAULT_COMCHAR_RAW} ")?;
    for sample in path {
        write!(output, "{sample:4}.")?;
    }
    writeln!(output, ":{} terms", leaf.payload_nodes())?;
    if let Some(payload) = leaf.payload() {
        write_overlap_index_subterm_payload_debug(output, payload, bank, problem_type)?;
    }
    Ok(())
}

pub fn write_overlap_index_subterm_payload_debug(
    output: &mut impl Write,
    payload: &ObjTree<SubtermOcc>,
    bank: &TermBank,
    problem_type: ProblemType,
) -> fmt::Result {
    for occurrence in payload.iter() {
        writeln!(output, "Node: {occurrence:p} data={occurrence:p}")?;
        write!(output, "Key: {} = ", occurrence.term().entry_no())?;
        bank.write_term_deref_for_problem(
            output,
            occurrence.term(),
            problem_type,
            DerefType::Always,
        )?;
        writeln!(output)?;
        write_overlap_index_clause_tree_debug(output, occurrence.position_clauses(), bank)?;
    }
    Ok(())
}

pub fn write_overlap_index_clause_tree_debug(
    output: &mut impl Write,
    tree: &ClauseTPosTree,
    bank: &TermBank,
) -> fmt::Result {
    for entry in tree.entries() {
        entry.write_lop_debug(output, bank)?;
    }
    Ok(())
}

#[must_use]
pub fn clause_collect_into_terms(clause: &Clause, terms: &mut TermIdentitySet) -> i64 {
    clause
        .literals()
        .as_slice()
        .iter()
        .filter(|literal| literal.is_maximal())
        .map(|literal| eqn_collect_into_terms(literal, terms))
        .sum()
}

#[must_use]
pub fn clause_collect_into_terms_pos(clause: &Clause, terms: &mut Vec<OverlapTermPos>) -> i64 {
    let mut pos = 0;
    let mut result = 0;
    for literal in clause.literals().as_slice() {
        if literal.is_maximal() {
            result += eqn_collect_into_terms_pos(literal, pos, terms);
        }
        pos += literal.standard_weight();
    }
    result
}

#[must_use]
pub fn clause_collect_from_terms(clause: &Clause, terms: &mut TermIdentitySet) -> i64 {
    clause
        .literals()
        .as_slice()
        .iter()
        .filter(|literal| literal.is_maximal() && literal.is_positive() && !literal.is_selected())
        .map(|literal| {
            let mut result = 1;
            store_term(terms, literal.left());
            if !literal.is_oriented() {
                result += 1;
                store_term(terms, literal.right());
            }
            result
        })
        .sum()
}

#[must_use]
pub fn clause_collect_from_terms_pos(clause: &Clause, terms: &mut Vec<OverlapTermPos>) -> i64 {
    let mut pos = 0;
    let mut result = 0;
    for literal in clause.literals().as_slice() {
        if literal.is_maximal() && literal.is_positive() && !literal.is_selected() {
            result += 1;
            terms.push(OverlapTermPos::new(literal.left().clone(), pos));
            if !literal.is_oriented() {
                result += 1;
                terms.push(OverlapTermPos::new(
                    literal.right().clone(),
                    pos + term_standard_weight(literal.left()),
                ));
            }
        }
        pos += literal.standard_weight();
    }
    result
}

#[must_use]
pub fn clause_collect_into_terms2(
    clause: &Clause,
    bank: &TermBank,
    terms: &mut TermIdentitySet,
    natoms: &mut TermIdentitySet,
) -> i64 {
    clause
        .literals()
        .as_slice()
        .iter()
        .filter(|literal| literal.is_maximal())
        .map(|literal| eqn_collect_into_terms2(literal, bank, terms, natoms))
        .sum()
}

#[must_use]
pub fn clause_collect_into_terms_pos2(
    clause: &Clause,
    bank: &TermBank,
    terms: &mut Vec<OverlapTermPos>,
    natoms: &mut Vec<OverlapTermPos>,
) -> i64 {
    let mut pos = 0;
    let mut result = 0;
    for literal in clause.literals().as_slice() {
        if literal.is_maximal() {
            result += eqn_collect_into_terms_pos2(literal, bank, pos, terms, natoms);
        }
        pos += literal.standard_weight();
    }
    result
}

pub fn overlap_index_insert_into_clause2(
    tindex: &mut OverlapIndex<'_>,
    naindex: &mut OverlapIndex<'_>,
    clause: &Clause,
    bank: &TermBank,
) -> i64 {
    let mut terms = Vec::new();
    let mut natoms = Vec::new();
    let count = clause_collect_into_terms_pos2(clause, bank, &mut terms, &mut natoms);
    for entry in terms.iter().rev() {
        tindex.insert_pos(clause, entry.pos(), Some(entry.term()));
    }
    for entry in natoms.iter().rev() {
        naindex.insert_pos(clause, entry.pos(), Some(entry.term()));
    }
    count
}

pub fn overlap_index_delete_into_clause2(
    tindex: &mut OverlapIndex<'_>,
    naindex: &mut OverlapIndex<'_>,
    clause: &Clause,
    bank: &TermBank,
) -> i64 {
    let mut terms = TermIdentitySet::new();
    let mut natoms = TermIdentitySet::new();
    let count = clause_collect_into_terms2(clause, bank, &mut terms, &mut natoms);
    for term in terms.values() {
        tindex.delete_clause_occ(clause, term);
    }
    for term in natoms.values() {
        naindex.delete_clause_occ(clause, term);
    }
    count
}

fn eqn_collect_into_terms(literal: &Eqn, terms: &mut TermIdentitySet) -> i64 {
    let mut result = term_collect_into_terms(literal.left(), terms);
    if !literal.is_oriented() {
        result += term_collect_into_terms(literal.right(), terms);
    }
    result
}

fn eqn_collect_into_terms2(
    literal: &Eqn,
    bank: &TermBank,
    terms: &mut TermIdentitySet,
    natoms: &mut TermIdentitySet,
) -> i64 {
    let mut result = if literal.is_negative() && !literal.is_equ_lit(bank) {
        term_collect_into_terms2(literal.left(), terms, natoms)
    } else {
        term_collect_into_terms(literal.left(), terms)
    };
    if !literal.is_oriented() {
        result += term_collect_into_terms(literal.right(), terms);
    }
    result
}

fn eqn_collect_into_terms_pos(
    literal: &Eqn,
    litpos: CompactPos,
    terms: &mut Vec<OverlapTermPos>,
) -> i64 {
    let mut result = term_collect_into_terms_pos(literal.left(), litpos, terms);
    if !literal.is_oriented() {
        result += term_collect_into_terms_pos(
            literal.right(),
            litpos + term_standard_weight(literal.left()),
            terms,
        );
    }
    result
}

fn eqn_collect_into_terms_pos2(
    literal: &Eqn,
    bank: &TermBank,
    litpos: CompactPos,
    terms: &mut Vec<OverlapTermPos>,
    natoms: &mut Vec<OverlapTermPos>,
) -> i64 {
    let mut result = if literal.is_negative() && !literal.is_equ_lit(bank) {
        term_collect_into_terms_pos2(literal.left(), litpos, terms, natoms)
    } else {
        term_collect_into_terms_pos(literal.left(), litpos, terms)
    };
    if !literal.is_oriented() {
        result += term_collect_into_terms_pos(
            literal.right(),
            litpos + term_standard_weight(literal.left()),
            terms,
        );
    }
    result
}

fn term_collect_into_terms(term: &Term, terms: &mut TermIdentitySet) -> i64 {
    if term.is_free_var() {
        return 0;
    }
    store_term(terms, term);
    let mut result = 1;
    for arg in term.argument_clones().into_iter().flatten() {
        result += term_collect_into_terms(&arg, terms);
    }
    result
}

fn term_collect_into_terms2(
    term: &Term,
    terms: &mut TermIdentitySet,
    natoms: &mut TermIdentitySet,
) -> i64 {
    if term.is_free_var() {
        return 0;
    }
    store_term(natoms, term);
    let mut result = 1;
    for arg in term.argument_clones().into_iter().flatten() {
        result += term_collect_into_terms(&arg, terms);
    }
    result
}

fn term_collect_into_terms_pos(
    term: &Term,
    mut pos: CompactPos,
    terms: &mut Vec<OverlapTermPos>,
) -> i64 {
    if term.is_free_var() {
        return 0;
    }
    terms.push(OverlapTermPos::new(term.clone(), pos));
    let mut result = 1;
    if !term.is_phony_app() {
        pos += DEFAULT_FWEIGHT;
    }
    if !term.is_lambda() {
        for arg in term.argument_clones().into_iter().flatten() {
            result += term_collect_into_terms_pos(&arg, pos, terms);
            pos += term_standard_weight(&arg);
        }
    }
    result
}

fn term_collect_into_terms_pos2(
    term: &Term,
    mut pos: CompactPos,
    terms: &mut Vec<OverlapTermPos>,
    natoms: &mut Vec<OverlapTermPos>,
) -> i64 {
    if term.is_free_var() {
        return 0;
    }
    natoms.push(OverlapTermPos::new(term.clone(), pos));
    let mut result = 1;
    if !term.is_phony_app() {
        pos += DEFAULT_FWEIGHT;
    }
    if !term.is_lambda() {
        for arg in term.argument_clones().into_iter().flatten() {
            result += term_collect_into_terms_pos(&arg, pos, terms);
            pos += term_standard_weight(&arg);
        }
    }
    result
}

fn store_term(terms: &mut TermIdentitySet, term: &Term) -> bool {
    match terms.entry(term_identity_id(term)) {
        Entry::Occupied(_) => false,
        Entry::Vacant(entry) => {
            entry.insert(term.clone());
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        clause_collect_from_terms, clause_collect_from_terms_pos, clause_collect_into_terms,
        clause_collect_into_terms2, clause_collect_into_terms_pos, clause_collect_into_terms_pos2,
        overlap_index_delete_into_clause2, overlap_index_insert_into_clause2, OverlapIndex,
    };
    use crate::basics::simple_stuff::ProblemType;
    use crate::clauses::clause::Clause;
    use crate::clauses::eqn::Eqn;
    use crate::clauses::eqn_props::{EP_IS_MAXIMAL, EP_IS_ORIENTED, EP_IS_SELECTED};
    use crate::clauses::eqnlist::EqnList;
    use crate::clauses::subterm_index::TermIdentitySet;
    use crate::terms::idx_fp::index_fp1_create;
    use crate::terms::signature::Signature;
    use crate::terms::simpletypes::alloc_arrow_type;
    use crate::terms::termbanks::TermBank;
    use crate::terms::termfunc::term_standard_weight;
    use crate::terms::termtypes::{term_identity_id, DerefType, Term};
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
        let term = Term::const_cell_alloc(f_code);
        term.set_type(Some(type_));
        bank.insert(&term, DerefType::Never).unwrap()
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

    fn typed_predicate(bank: &mut TermBank, name: &str, arg: &Term) -> Term {
        let individual = bank.signature().type_bank().default_type();
        let bool_type = bank.signature().type_bank().bool_type();
        let p_type = bank
            .signature_mut()
            .type_bank_mut()
            .insert_type_shared(alloc_arrow_type(vec![individual, bool_type]));
        let f_code = bank.signature_mut().insert_id(name, 1, false);
        if bank.signature().get_type(f_code).is_none() {
            bank.signature_mut()
                .declare_final_type(f_code, p_type)
                .unwrap();
        }
        let term = Term::top_alloc(f_code, 1);
        term.set_type(Some(bank.signature().type_bank().bool_type()));
        term.set_argument(0, arg.clone());
        bank.insert(&term, DerefType::Never).unwrap()
    }

    fn eqn(bank: &mut TermBank, left: &Term, right: &Term, positive: bool) -> Eqn {
        Eqn::alloc(left.clone(), right.clone(), bank, positive).unwrap()
    }

    fn singleton_clause(literal: Eqn, ident: i64) -> Clause {
        let mut clause = Clause::alloc(EqnList::from_vec(vec![literal]));
        clause.set_ident(ident);
        clause
    }

    fn contains_term(terms: &TermIdentitySet, term: &Term) -> bool {
        terms.contains_key(&term_identity_id(term))
    }

    #[test]
    fn into_collectors_visit_maximal_non_variable_subterms_with_compact_positions() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "oi_a");
        let b = typed_const(&mut bank, "oi_b");
        let left = typed_unary(&mut bank, "oi_f", &a);
        let right = typed_unary(&mut bank, "oi_g", &b);
        let mut literal = eqn(&mut bank, &left, &right, true);
        literal.set_prop(EP_IS_MAXIMAL);
        let clause = singleton_clause(literal, 1);
        let mut terms = TermIdentitySet::new();
        let mut positions = Vec::new();

        assert_eq!(clause_collect_into_terms(&clause, &mut terms), 4);
        assert!(contains_term(&terms, &left));
        assert!(contains_term(&terms, &a));
        assert!(contains_term(&terms, &right));
        assert!(contains_term(&terms, &b));

        assert_eq!(clause_collect_into_terms_pos(&clause, &mut positions), 4);
        assert_eq!(positions[0].term(), &left);
        assert_eq!(positions[0].pos(), 0);
        assert_eq!(positions[1].term(), &a);
        assert_eq!(
            positions[1].pos(),
            term_standard_weight(&Term::const_cell_alloc(0))
        );
        assert_eq!(positions[2].term(), &right);
        assert_eq!(positions[2].pos(), term_standard_weight(&left));
    }

    #[test]
    fn from_collectors_keep_only_unselected_positive_maximal_rule_sides() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "oi_from_a");
        let b = typed_const(&mut bank, "oi_from_b");
        let c = typed_const(&mut bank, "oi_from_c");
        let left = typed_unary(&mut bank, "oi_from_f", &a);
        let right = typed_unary(&mut bank, "oi_from_g", &b);
        let mut rule = eqn(&mut bank, &left, &right, true);
        rule.set_prop(EP_IS_MAXIMAL);
        let mut selected = eqn(&mut bank, &right, &c, true);
        selected.set_prop(EP_IS_MAXIMAL | EP_IS_SELECTED);
        let negative = eqn(&mut bank, &c, &a, false);
        let clause = Clause::alloc(EqnList::from_vec(vec![rule, selected, negative]));
        let mut terms = TermIdentitySet::new();
        let mut positions = Vec::new();

        assert_eq!(clause_collect_from_terms(&clause, &mut terms), 2);
        assert!(contains_term(&terms, &left));
        assert!(contains_term(&terms, &right));
        assert_eq!(terms.len(), 2);

        assert_eq!(clause_collect_from_terms_pos(&clause, &mut positions), 2);
        assert_eq!(positions[0].term(), &left);
        assert_eq!(positions[0].pos(), 0);
        assert_eq!(positions[1].term(), &right);
        assert_eq!(positions[1].pos(), term_standard_weight(&left));
    }

    #[test]
    fn split_into_collectors_route_negative_atom_heads_to_natom_index() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "oi_split_a");
        let f_a = typed_unary(&mut bank, "oi_split_f", &a);
        let atom = typed_predicate(&mut bank, "oi_split_p", &f_a);
        let mut literal =
            Eqn::alloc(atom.clone(), bank.true_term().clone(), &mut bank, false).unwrap();
        literal.set_prop(EP_IS_MAXIMAL | EP_IS_ORIENTED);
        let clause = singleton_clause(literal, 4);
        let mut terms = TermIdentitySet::new();
        let mut natoms = TermIdentitySet::new();
        let mut term_positions = Vec::new();
        let mut natom_positions = Vec::new();

        assert_eq!(
            clause_collect_into_terms2(&clause, &bank, &mut terms, &mut natoms),
            3
        );
        assert!(contains_term(&natoms, &atom));
        assert!(contains_term(&terms, &f_a));
        assert!(contains_term(&terms, &a));

        assert_eq!(
            clause_collect_into_terms_pos2(
                &clause,
                &bank,
                &mut term_positions,
                &mut natom_positions
            ),
            3
        );
        assert_eq!(natom_positions.len(), 1);
        assert_eq!(natom_positions[0].term(), &atom);
        assert_eq!(term_positions.len(), 2);
    }

    #[test]
    fn direct_position_insert_delete_updates_fingerprint_payload() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "oi_idx_a");
        let b = typed_const(&mut bank, "oi_idx_b");
        let left = typed_unary(&mut bank, "oi_idx_f", &a);
        let literal = eqn(&mut bank, &left, &b, true);
        let clause = singleton_clause(literal, 10);
        let mut index = OverlapIndex::new(index_fp1_create, bank.signature());

        assert!(index.insert_pos(&clause, 0, Some(&left)));
        assert!(!index.insert_pos(&clause, 0, Some(&left)));
        let occurrence = index.find_occurrence(&left).unwrap();
        assert_eq!(
            occurrence
                .position_clauses()
                .find(&clause)
                .unwrap()
                .positions()
                .iter()
                .copied()
                .collect::<Vec<_>>(),
            vec![0]
        );

        assert!(index.delete_pos(&clause, 0, Some(&left)));
        assert!(index.find_occurrence(&left).is_none());
        assert!(index.find_leaf(&left).is_none());
        assert!(!index.delete_pos(&clause, 0, Some(&left)));
    }

    #[test]
    fn leaf_debug_string_matches_c_leaf_header_and_payload_shape() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "oi_leaf_a");
        let b = typed_const(&mut bank, "oi_leaf_b");
        let left = typed_unary(&mut bank, "oi_leaf_f", &a);
        let literal = eqn(&mut bank, &left, &b, true);
        let clause = singleton_clause(literal, 12);
        let mut index = OverlapIndex::new(index_fp1_create, bank.signature());

        assert!(index.insert_pos(&clause, 0, Some(&left)));

        let debug = index.leaf_debug_string(&bank, ProblemType::FirstOrder);

        assert!(debug.starts_with("% "));
        assert!(debug.contains(&format!("{:4}.:1 terms\n", left.f_code())));
        assert!(debug.contains("Node: "));
        assert!(debug.contains(" data="));
        assert!(debug.contains("Key: "));
        assert!(debug.contains("oi_leaf_f(oi_leaf_a)"));
        assert!(debug.contains("OLs: "));
        assert!(debug.contains("occ: 0\n"));
    }

    #[expect(
        clippy::needless_pass_by_value,
        reason = "regression exercises cloned clause values passing through a by-value helper"
    )]
    fn insert_position_by_value(
        index: &mut OverlapIndex<'_>,
        clause: Clause,
        term: &Term,
        pos: i64,
    ) {
        assert!(index.insert_pos(&clause, pos, Some(term)));
    }

    #[test]
    fn direct_position_insert_keys_cloned_clause_payloads_by_stable_identity() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "oi_stable_a");
        let b = typed_const(&mut bank, "oi_stable_b");
        let c = typed_const(&mut bank, "oi_stable_c");
        let left = typed_unary(&mut bank, "oi_stable_f", &a);
        let first = singleton_clause(eqn(&mut bank, &left, &b, true), 40);
        let second = singleton_clause(eqn(&mut bank, &left, &c, true), 41);
        let mut index = OverlapIndex::new(index_fp1_create, bank.signature());

        insert_position_by_value(&mut index, first.clone(), &left, 0);
        insert_position_by_value(&mut index, second.clone(), &left, 2);

        let occurrence = index.find_occurrence(&left).unwrap();
        assert_eq!(occurrence.position_clauses().len(), 2);
        assert_eq!(
            occurrence
                .position_clauses()
                .find(&first)
                .unwrap()
                .positions()
                .iter()
                .copied()
                .collect::<Vec<_>>(),
            vec![0]
        );
        assert_eq!(
            occurrence
                .position_clauses()
                .find(&second)
                .unwrap()
                .positions()
                .iter()
                .copied()
                .collect::<Vec<_>>(),
            vec![2]
        );
    }

    #[test]
    fn clause_insert_delete_uses_collected_into_and_from_terms() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "oi_clause_a");
        let b = typed_const(&mut bank, "oi_clause_b");
        let left = typed_unary(&mut bank, "oi_clause_f", &a);
        let right = typed_unary(&mut bank, "oi_clause_g", &b);
        let mut literal = eqn(&mut bank, &left, &right, true);
        literal.set_prop(EP_IS_MAXIMAL);
        let clause = singleton_clause(literal, 20);
        let mut into_index = OverlapIndex::new(index_fp1_create, bank.signature());
        let mut from_index = OverlapIndex::new(index_fp1_create, bank.signature());

        assert_eq!(into_index.insert_into_clause(&clause), 4);
        assert!(into_index.find_occurrence(&a).is_some());
        assert_eq!(into_index.delete_into_clause(&clause), 4);
        assert!(into_index.find_occurrence(&a).is_none());

        assert_eq!(from_index.insert_from_clause(&clause), 2);
        assert!(from_index.find_occurrence(&left).is_some());
        assert!(from_index.find_occurrence(&right).is_some());
        assert_eq!(from_index.delete_from_clause(&clause), 2);
        assert!(from_index.find_occurrence(&left).is_none());
        assert!(from_index.find_occurrence(&right).is_none());
    }

    #[test]
    fn split_insert_delete_routes_negative_atom_occurrences() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "oi_route_a");
        let f_a = typed_unary(&mut bank, "oi_route_f", &a);
        let atom = typed_predicate(&mut bank, "oi_route_p", &f_a);
        let mut literal =
            Eqn::alloc(atom.clone(), bank.true_term().clone(), &mut bank, false).unwrap();
        literal.set_prop(EP_IS_MAXIMAL | EP_IS_ORIENTED);
        let clause = singleton_clause(literal, 30);
        let mut term_index = OverlapIndex::new(index_fp1_create, bank.signature());
        let mut natom_index = OverlapIndex::new(index_fp1_create, bank.signature());

        assert_eq!(
            overlap_index_insert_into_clause2(&mut term_index, &mut natom_index, &clause, &bank),
            3
        );
        assert!(natom_index.find_occurrence(&atom).is_some());
        assert!(term_index.find_occurrence(&atom).is_none());
        assert!(term_index.find_occurrence(&f_a).is_some());

        assert_eq!(
            overlap_index_delete_into_clause2(&mut term_index, &mut natom_index, &clause, &bank),
            3
        );
        assert!(natom_index.find_occurrence(&atom).is_none());
        assert!(term_index.find_occurrence(&f_a).is_none());
    }

    #[test]
    fn unifiable_occurrence_query_uses_c_candidate_stack_pop_order() {
        let mut bank = test_bank();
        let x = Term::const_cell_alloc(-4);
        x.set_type(Some(bank.signature().type_bank().default_type()));
        let a = typed_const(&mut bank, "oi_stack_order_a");
        let f_a = typed_unary(&mut bank, "oi_stack_order_f", &a);
        let g_a = typed_unary(&mut bank, "oi_stack_order_g", &a);
        let first = singleton_clause(eqn(&mut bank, &x, &a, true), 51);
        let second = singleton_clause(eqn(&mut bank, &f_a, &a, true), 52);
        let third = singleton_clause(eqn(&mut bank, &g_a, &a, true), 53);
        let mut index = OverlapIndex::new(index_fp1_create, bank.signature());
        index.insert_pos(&first, 0, Some(&x));
        index.insert_pos(&second, 0, Some(&f_a));
        index.insert_pos(&third, 0, Some(&g_a));

        let mut occurrences = Vec::new();
        assert_eq!(index.find_unifiable_occurrences(&x, &mut occurrences), 3);
        let idents = occurrences
            .iter()
            .flat_map(|occurrence| occurrence.position_clauses().entries())
            .map(|entry| entry.clause().ident())
            .collect::<Vec<_>>();
        assert_eq!(idents, vec![53, 52, 51]);
    }
}
