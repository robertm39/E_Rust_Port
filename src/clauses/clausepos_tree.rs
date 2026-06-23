use crate::clauses::clause::Clause;
use crate::clauses::clausecpos::CompactPos;
use std::collections::{btree_map::Entry, BTreeMap, BTreeSet};
use std::fmt::{self, Write};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClauseTPos {
    clause_key: usize,
    clause: Clause,
    positions: BTreeSet<CompactPos>,
}

impl ClauseTPos {
    #[must_use]
    pub fn new(clause: &Clause) -> Self {
        Self {
            clause_key: clause_key(clause),
            clause: clause.clone(),
            positions: BTreeSet::new(),
        }
    }

    #[must_use]
    pub const fn clause_key(&self) -> usize {
        self.clause_key
    }

    #[must_use]
    pub const fn clause(&self) -> &Clause {
        &self.clause
    }

    #[must_use]
    pub const fn positions(&self) -> &BTreeSet<CompactPos> {
        &self.positions
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.positions.is_empty()
    }

    pub fn insert_pos(&mut self, pos: CompactPos) -> bool {
        self.positions.insert(pos)
    }

    pub fn delete_pos(&mut self, pos: CompactPos) -> bool {
        self.positions.remove(&pos)
    }

    pub fn write_debug(&self, output: &mut impl Write) -> fmt::Result {
        writeln!(output, "OLs: clause#{}", self.clause.ident())?;
        write!(output, "occ:")?;
        for pos in &self.positions {
            write!(output, " {pos}")?;
        }
        writeln!(output)
    }

    #[must_use]
    pub fn debug_string(&self) -> String {
        let mut output = String::new();
        let _ = self.write_debug(&mut output);
        output
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ClauseTPosTree {
    entries: BTreeMap<usize, ClauseTPos>,
}

impl ClauseTPosTree {
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
    pub fn find(&self, clause: &Clause) -> Option<&ClauseTPos> {
        self.entries.get(&clause_key(clause))
    }

    pub fn insert_pos(&mut self, clause: &Clause, pos: CompactPos) -> bool {
        match self.entries.entry(clause_key(clause)) {
            Entry::Occupied(mut entry) => entry.get_mut().insert_pos(pos),
            Entry::Vacant(entry) => {
                let mut cell = ClauseTPos::new(clause);
                let inserted = cell.insert_pos(pos);
                entry.insert(cell);
                inserted
            }
        }
    }

    pub fn delete_pos(&mut self, clause: &Clause, pos: CompactPos) -> bool {
        let key = clause_key(clause);
        let Some(cell) = self.entries.get_mut(&key) else {
            return false;
        };
        let removed = cell.delete_pos(pos);
        if cell.is_empty() {
            self.entries.remove(&key);
        }
        removed
    }

    pub fn delete_clause(&mut self, clause: &Clause) -> bool {
        self.entries.remove(&clause_key(clause)).is_some()
    }

    pub fn entries(&self) -> impl Iterator<Item = &ClauseTPos> {
        self.entries.values()
    }
}

#[must_use]
pub fn cmp_clause_tpos_cells(left: &ClauseTPos, right: &ClauseTPos) -> i32 {
    match left.clause_key.cmp(&right.clause_key) {
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Greater => 1,
    }
}

#[must_use]
pub fn clause_key(clause: &Clause) -> usize {
    std::ptr::from_ref(clause) as usize
}

#[cfg(test)]
mod tests {
    use super::{clause_key, cmp_clause_tpos_cells, ClauseTPos, ClauseTPosTree};
    use crate::clauses::clause::Clause;
    use crate::clauses::eqn::Eqn;
    use crate::clauses::eqnlist::EqnList;
    use crate::terms::signature::Signature;
    use crate::terms::termbanks::TermBank;
    use crate::terms::termtypes::Term;
    use crate::terms::typebanks::TypeBank;

    fn test_bank() -> TermBank {
        TermBank::new(Signature::new(TypeBank::new())).unwrap()
    }

    fn typed_const(bank: &mut TermBank, name: &str) -> Term {
        let type_ = bank.signature().type_bank().default_type();
        let f_code = bank.signature_mut().insert_id(name, 0, false);
        bank.signature_mut()
            .declare_final_type(f_code, type_)
            .unwrap();
        bank.create_const_term(f_code).unwrap()
    }

    fn unit_clause(name: &str, ident: i64) -> Clause {
        let mut bank = test_bank();
        let left = typed_const(&mut bank, name);
        let right = typed_const(&mut bank, "rhs");
        let literal = Eqn::alloc(left, right, &mut bank, true).unwrap();
        let mut clause = Clause::alloc(EqnList::from_vec(vec![literal]));
        clause.set_ident(ident);
        clause
    }

    #[test]
    fn cells_store_clause_pointer_key_and_sorted_positions() {
        let clause = Box::new(unit_clause("a", 11));
        let mut cell = ClauseTPos::new(&clause);

        assert_eq!(cell.clause_key(), clause_key(&clause));
        assert_eq!(cell.clause().ident(), 11);
        assert!(cell.insert_pos(9));
        assert!(cell.insert_pos(3));
        assert!(!cell.insert_pos(9));
        assert_eq!(
            cell.positions().iter().copied().collect::<Vec<_>>(),
            vec![3, 9]
        );
        assert!(cell.delete_pos(3));
        assert_eq!(
            cell.positions().iter().copied().collect::<Vec<_>>(),
            vec![9]
        );
    }

    #[test]
    fn tree_insert_delete_and_duplicate_handling_match_clause_position_map() {
        let clause = Box::new(unit_clause("a", 1));
        let other = Box::new(unit_clause("b", 2));
        let mut tree = ClauseTPosTree::new();

        assert!(tree.is_empty());
        assert!(tree.insert_pos(&clause, 4));
        assert!(!tree.insert_pos(&clause, 4));
        assert!(tree.insert_pos(&clause, 8));
        assert!(tree.insert_pos(&other, 2));
        assert_eq!(tree.len(), 2);
        assert_eq!(
            tree.find(&clause)
                .unwrap()
                .positions()
                .iter()
                .copied()
                .collect::<Vec<_>>(),
            vec![4, 8]
        );

        assert!(tree.delete_pos(&clause, 4));
        assert_eq!(
            tree.find(&clause)
                .unwrap()
                .positions()
                .iter()
                .copied()
                .collect::<Vec<_>>(),
            vec![8]
        );
        assert!(tree.delete_pos(&clause, 8));
        assert!(tree.find(&clause).is_none());
        assert!(tree.find(&other).is_some());
        assert!(tree.delete_clause(&other));
        assert!(tree.is_empty());
    }

    #[test]
    fn comparison_and_debug_print_use_clause_identity_and_positions() {
        let left = Box::new(unit_clause("a", 7));
        let right = Box::new(unit_clause("b", 8));
        let mut left_cell = ClauseTPos::new(&left);
        let right_cell = ClauseTPos::new(&right);
        left_cell.insert_pos(5);
        left_cell.insert_pos(1);

        assert_eq!(cmp_clause_tpos_cells(&left_cell, &left_cell), 0);
        assert_ne!(cmp_clause_tpos_cells(&left_cell, &right_cell), 0);
        assert_eq!(left_cell.debug_string(), "OLs: clause#7\nocc: 1 5\n");
    }
}
