use crate::clauses::clause::Clause;
use crate::clauses::clausecpos::CompactPos;
use crate::clauses::clausepos_tree::ClauseTPosTree;
use crate::clauses::eqn::Eqn;
use crate::terms::functypes::FunCode;
use crate::terms::termfunc::term_standard_weight;
use crate::terms::termtypes::{Term, DEFAULT_FWEIGHT};
use std::collections::{btree_map::Entry, BTreeMap};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExtIndexPos {
    f_code: FunCode,
    pos: CompactPos,
}

impl ExtIndexPos {
    #[must_use]
    pub const fn new(f_code: FunCode, pos: CompactPos) -> Self {
        Self { f_code, pos }
    }

    #[must_use]
    pub const fn f_code(self) -> FunCode {
        self.f_code
    }

    #[must_use]
    pub const fn pos(self) -> CompactPos {
        self.pos
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ExtIndex {
    entries: BTreeMap<FunCode, ClauseTPosTree>,
}

impl ExtIndex {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            entries: BTreeMap::new(),
        }
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    #[must_use]
    pub fn find(&self, f_code: FunCode) -> Option<&ClauseTPosTree> {
        self.entries.get(&f_code)
    }

    pub fn insert_pos(&mut self, clause: &Clause, f_code: FunCode, pos: CompactPos) -> bool {
        match self.entries.entry(f_code) {
            Entry::Occupied(mut entry) => entry.get_mut().insert_pos(clause, pos),
            Entry::Vacant(entry) => {
                let mut tree = ClauseTPosTree::new();
                let inserted = tree.insert_pos(clause, pos);
                entry.insert(tree);
                inserted
            }
        }
    }

    pub fn delete_clause_for_symbol(&mut self, clause: &Clause, f_code: FunCode) -> bool {
        let Some(tree) = self.entries.get_mut(&f_code) else {
            return false;
        };
        let deleted = tree.delete_clause(clause);
        if tree.is_empty() {
            self.entries.remove(&f_code);
        }
        deleted
    }

    pub fn insert_into_clause(&mut self, clause: &Clause, max_depth: i32) -> i64 {
        if clause.proof_depth() > i64::from(max_depth) {
            return 0;
        }
        let mut positions = Vec::new();
        collect_ext_sup_into_pos(clause, &mut positions);
        self.insert_collected(clause, &positions);
        i64::try_from(positions.len()).unwrap_or(i64::MAX)
    }

    pub fn delete_into_clause(&mut self, clause: &Clause) -> i64 {
        let mut positions = Vec::new();
        collect_ext_sup_into_pos(clause, &mut positions);
        self.delete_collected(clause, &positions);
        i64::try_from(positions.len()).unwrap_or(i64::MAX)
    }

    pub fn insert_from_clause(&mut self, clause: &Clause, max_depth: i32) -> i64 {
        if clause.proof_depth() > i64::from(max_depth) {
            return 0;
        }
        let mut positions = Vec::new();
        collect_ext_sup_from_pos(clause, &mut positions);
        self.insert_collected(clause, &positions);
        i64::try_from(positions.len()).unwrap_or(i64::MAX)
    }

    pub fn delete_from_clause(&mut self, clause: &Clause) -> i64 {
        let mut positions = Vec::new();
        collect_ext_sup_from_pos(clause, &mut positions);
        self.delete_collected(clause, &positions);
        i64::try_from(positions.len()).unwrap_or(i64::MAX)
    }

    fn insert_collected(&mut self, clause: &Clause, positions: &[ExtIndexPos]) {
        for entry in positions.iter().rev() {
            self.insert_pos(clause, entry.f_code(), entry.pos());
        }
    }

    fn delete_collected(&mut self, clause: &Clause, positions: &[ExtIndexPos]) {
        for entry in positions.iter().rev() {
            self.delete_clause_for_symbol(clause, entry.f_code());
        }
    }
}

#[must_use]
pub fn type_ext_eligible(term: &Term) -> bool {
    term.type_()
        .is_some_and(|type_| type_.is_bool() || type_.is_arrow())
}

#[must_use]
pub fn term_has_ext_eligible_subterm(term: &Term) -> bool {
    term.argument_clones().into_iter().flatten().any(|arg| {
        (type_ext_eligible(&arg) && !arg.is_top_level_any_var())
            || term_has_ext_eligible_subterm(&arg)
    })
}

pub fn collect_ext_sup_into_pos(clause: &Clause, positions: &mut Vec<ExtIndexPos>) {
    let mut pos = 0;
    for literal in clause.literals().as_slice() {
        build_into_pos_stack(literal, pos, positions);
        pos += literal.standard_weight();
    }
}

pub fn collect_ext_sup_from_pos(clause: &Clause, positions: &mut Vec<ExtIndexPos>) {
    let mut pos = 0;
    for literal in clause.literals().as_slice() {
        if !term_type_is_arrow(literal.left()) && literal.is_positive() {
            if !literal.left().is_top_level_free_var()
                && !maybe_normalize_app_var(literal.left())
                && term_has_ext_eligible_subterm(literal.left())
            {
                positions.push(ExtIndexPos::new(literal.left().f_code(), pos));
            }
            pos += term_standard_weight(literal.left());
            if !literal.right().is_top_level_free_var()
                && !maybe_normalize_app_var(literal.left())
                && term_has_ext_eligible_subterm(literal.right())
            {
                positions.push(ExtIndexPos::new(literal.right().f_code(), pos));
            }
            pos += term_standard_weight(literal.right());
        } else {
            pos += literal.standard_weight();
        }
    }
}

fn build_into_pos_stack(literal: &Eqn, pos: CompactPos, positions: &mut Vec<ExtIndexPos>) {
    collect_into_pos_term(literal.left(), pos, positions);
    collect_into_pos_term(
        literal.right(),
        pos + term_standard_weight(literal.left()),
        positions,
    );
}

fn collect_into_pos_term(term: &Term, mut pos: CompactPos, positions: &mut Vec<ExtIndexPos>) {
    let term_pos = pos;
    let old_len = positions.len();
    let mut has_func_subterm = false;
    let normalized_pattern = maybe_normalize_app_var(term);

    if !term.is_lambda() && !normalized_pattern {
        if !term.is_phony_app() {
            pos += DEFAULT_FWEIGHT;
        }
        for arg in term.argument_clones().into_iter().flatten() {
            collect_into_pos_term(&arg, pos, positions);
            has_func_subterm |= type_ext_eligible(&arg) && !arg.is_top_level_any_var();
            pos += term_standard_weight(&arg);
        }
    }

    if !term_type_is_arrow(term)
        && !term.is_top_level_any_var()
        && !normalized_pattern
        && (has_func_subterm || positions.len() != old_len)
    {
        positions.push(ExtIndexPos::new(term.f_code(), term_pos));
    }
}

fn maybe_normalize_app_var(term: &Term) -> bool {
    term.is_applied_free_var()
}

fn term_type_is_arrow(term: &Term) -> bool {
    term.type_().is_some_and(|type_| type_.is_arrow())
}

#[cfg(test)]
mod tests {
    use super::{
        collect_ext_sup_from_pos, collect_ext_sup_into_pos, term_has_ext_eligible_subterm, ExtIndex,
    };
    use crate::clauses::clause::Clause;
    use crate::clauses::eqn::Eqn;
    use crate::clauses::eqnlist::EqnList;
    use crate::terms::signature::Signature;
    use crate::terms::simpletypes::{alloc_arrow_type, Type};
    use crate::terms::termbanks::TermBank;
    use crate::terms::termfunc::term_standard_weight;
    use crate::terms::termtypes::{DerefType, Term, DEFAULT_FWEIGHT};
    use crate::terms::typebanks::TypeBank;

    fn test_bank() -> TermBank {
        let mut signature = Signature::new(TypeBank::new());
        signature.insert_internal_codes().unwrap();
        TermBank::new(signature).unwrap()
    }

    fn typed_const(bank: &mut TermBank, name: &str, type_: Type) -> Term {
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

    fn typed_unary(bank: &mut TermBank, name: &str, arg: &Term, return_type: Type) -> Term {
        let arg_type = arg
            .type_()
            .unwrap_or_else(|| bank.signature().type_bank().default_type());
        let f_code = bank.signature_mut().insert_id(name, 1, false);
        if bank.signature().get_type(f_code).is_none() {
            bank.signature_mut()
                .declare_final_type(
                    f_code,
                    alloc_arrow_type(vec![arg_type, return_type.clone()]),
                )
                .unwrap();
        }
        let term = Term::top_alloc(f_code, 1);
        term.set_type(Some(return_type));
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

    #[test]
    fn term_has_ext_eligible_subterm_detects_bool_or_arrow_children() {
        let mut bank = test_bank();
        let individual = bank.signature().type_bank().default_type();
        let bool_type = bank.signature().type_bank().bool_type();
        let arrow_type = alloc_arrow_type(vec![individual.clone(), individual.clone()]);
        let plain = typed_const(&mut bank, "ext_plain", individual.clone());
        let arrow = typed_const(&mut bank, "ext_arrow", arrow_type);
        let bool_term = typed_const(&mut bank, "ext_bool", bool_type);
        let with_plain = typed_unary(&mut bank, "ext_plain_parent", &plain, individual.clone());
        let with_arrow = typed_unary(&mut bank, "ext_arrow_parent", &arrow, individual.clone());
        let with_bool = typed_unary(&mut bank, "ext_bool_parent", &bool_term, individual);

        assert!(!term_has_ext_eligible_subterm(&with_plain));
        assert!(term_has_ext_eligible_subterm(&with_arrow));
        assert!(term_has_ext_eligible_subterm(&with_bool));
    }

    #[test]
    fn collect_into_positions_pushes_terms_with_eligible_descendants() {
        let mut bank = test_bank();
        let individual = bank.signature().type_bank().default_type();
        let arrow_type = alloc_arrow_type(vec![individual.clone(), individual.clone()]);
        let arrow = typed_const(&mut bank, "ext_into_arrow", arrow_type);
        let inner = typed_unary(&mut bank, "ext_into_inner", &arrow, individual.clone());
        let outer = typed_unary(&mut bank, "ext_into_outer", &inner, individual.clone());
        let rhs = typed_const(&mut bank, "ext_into_rhs", individual);
        let clause = singleton_clause(eqn(&mut bank, &outer, &rhs, true), 1);
        let mut positions = Vec::new();

        collect_ext_sup_into_pos(&clause, &mut positions);

        assert_eq!(positions.len(), 2);
        assert_eq!(positions[0].f_code(), inner.f_code());
        assert_eq!(positions[0].pos(), DEFAULT_FWEIGHT);
        assert_eq!(positions[1].f_code(), outer.f_code());
        assert_eq!(positions[1].pos(), 0);
    }

    #[test]
    fn collect_from_positions_uses_positive_left_type_gate_and_both_sides() {
        let mut bank = test_bank();
        let individual = bank.signature().type_bank().default_type();
        let arrow_type = alloc_arrow_type(vec![individual.clone(), individual.clone()]);
        let arrow = typed_const(&mut bank, "ext_from_arrow", arrow_type);
        let left = typed_unary(&mut bank, "ext_from_left", &arrow, individual.clone());
        let right = typed_unary(&mut bank, "ext_from_right", &arrow, individual);
        let clause = singleton_clause(eqn(&mut bank, &left, &right, true), 2);
        let mut positions = Vec::new();

        collect_ext_sup_from_pos(&clause, &mut positions);

        assert_eq!(positions.len(), 2);
        assert_eq!(positions[0].f_code(), left.f_code());
        assert_eq!(positions[0].pos(), 0);
        assert_eq!(positions[1].f_code(), right.f_code());
        assert_eq!(positions[1].pos(), term_standard_weight(&left));
    }

    #[test]
    fn insert_and_delete_apply_proof_depth_gate() {
        let mut bank = test_bank();
        let individual = bank.signature().type_bank().default_type();
        let arrow_type = alloc_arrow_type(vec![individual.clone(), individual.clone()]);
        let arrow = typed_const(&mut bank, "ext_idx_arrow", arrow_type);
        let left = typed_unary(&mut bank, "ext_idx_left", &arrow, individual.clone());
        let right = typed_const(&mut bank, "ext_idx_rhs", individual);
        let mut clause = singleton_clause(eqn(&mut bank, &left, &right, true), 3);
        clause.set_proof_depth(5);
        let mut index = ExtIndex::new();

        assert_eq!(index.insert_into_clause(&clause, 4), 0);
        assert!(index.is_empty());
        assert_eq!(index.insert_into_clause(&clause, 5), 1);
        assert_eq!(index.len(), 1);
        assert!(index.find(left.f_code()).unwrap().find(&clause).is_some());

        assert_eq!(index.delete_into_clause(&clause), 1);
        assert!(index.find(left.f_code()).is_none());
    }
}
