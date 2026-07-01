use crate::basics::error::Diagnostic;
use crate::basics::simple_stuff::ProblemType;
use crate::clauses::clause::{
    clause_print_lop_format_string, clause_print_tptp_format_string, clause_tstp_string, Clause,
};
use crate::clauses::clausecpos::CompactPos;
use crate::inout::scanner::IoFormat;
use crate::terms::termbanks::TermBank;
use std::collections::{btree_map::Entry, BTreeMap, BTreeSet};
use std::fmt::{self, Write};

#[derive(Clone, Debug, PartialEq)]
pub struct ClauseTPos {
    clause_key: i64,
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
    pub const fn clause_key(&self) -> i64 {
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

    pub fn write_lop_debug(&self, output: &mut impl Write, bank: &TermBank) -> fmt::Result {
        writeln!(
            output,
            "OLs: {}",
            clause_print_lop_format_string(bank, &self.clause, true)
        )?;
        write!(output, "occ:")?;
        for pos in &self.positions {
            write!(output, " {pos}")?;
        }
        writeln!(output)
    }

    #[must_use]
    pub fn lop_debug_string(&self, bank: &TermBank) -> String {
        let mut output = String::new();
        let _ = self.write_lop_debug(&mut output, bank);
        output
    }

    /// Returns the C `ClauseTPosTreePrint` shape with explicit `ClausePrint` dispatch.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if TSTP rendering rejects the stored clause.
    pub fn format_debug_string(
        &self,
        bank: &TermBank,
        output_format: IoFormat,
        problem_type: ProblemType,
    ) -> Result<String, Diagnostic> {
        let mut output = String::new();
        writeln!(
            output,
            "OLs: {}",
            clause_tpos_rendered_clause_string(bank, &self.clause, output_format, problem_type,)?
        )
        .expect("writing to String cannot fail");
        write!(output, "occ:").expect("writing to String cannot fail");
        for pos in &self.positions {
            write!(output, " {pos}").expect("writing to String cannot fail");
        }
        writeln!(output).expect("writing to String cannot fail");
        Ok(output)
    }
}

fn clause_tpos_rendered_clause_string(
    bank: &TermBank,
    clause: &Clause,
    output_format: IoFormat,
    problem_type: ProblemType,
) -> Result<String, Diagnostic> {
    match output_format {
        IoFormat::Tptp => Ok(clause_print_tptp_format_string(bank, clause)),
        IoFormat::Tstp => clause_tstp_string(bank, clause, true, true, problem_type),
        IoFormat::Lop | IoFormat::Auto => Ok(clause_print_lop_format_string(bank, clause, true)),
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ClauseTPosTree {
    entries: BTreeMap<i64, ClauseTPos>,
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
pub const fn clause_key(clause: &Clause) -> i64 {
    clause.ident()
}

#[cfg(test)]
mod tests {
    use super::{clause_key, cmp_clause_tpos_cells, ClauseTPos, ClauseTPosTree};
    use crate::basics::simple_stuff::ProblemType;
    use crate::clauses::clause::Clause;
    use crate::clauses::eqn::Eqn;
    use crate::clauses::eqnlist::EqnList;
    use crate::inout::scanner::IoFormat;
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

    #[test]
    fn lop_debug_print_uses_explicit_clause_rendering() {
        let mut bank = test_bank();
        let left = typed_const(&mut bank, "clause_tpos_a");
        let right = typed_const(&mut bank, "clause_tpos_b");
        let literal = Eqn::alloc(left, right, &mut bank, true).unwrap();
        let clause = Clause::alloc(EqnList::from_vec(vec![literal]));
        let mut cell = ClauseTPos::new(&clause);
        cell.insert_pos(5);
        cell.insert_pos(1);

        assert_eq!(
            cell.lop_debug_string(&bank),
            "OLs: clause_tpos_a=clause_tpos_b <- .\nocc: 1 5\n"
        );
    }

    #[test]
    fn format_debug_print_dispatches_clause_output() {
        let mut bank = test_bank();
        let left = typed_const(&mut bank, "clause_tpos_format_a");
        let right = typed_const(&mut bank, "clause_tpos_format_b");
        let literal = Eqn::alloc(left, right, &mut bank, true).unwrap();
        let clause = Clause::alloc(EqnList::from_vec(vec![literal]));
        let mut cell = ClauseTPos::new(&clause);
        cell.insert_pos(2);
        cell.insert_pos(7);

        let input_clause_debug = cell
            .format_debug_string(&bank, IoFormat::Tptp, ProblemType::FirstOrder)
            .unwrap_or_else(|err| panic!("{err}"));
        assert!(input_clause_debug.starts_with("OLs: input_clause("));
        assert!(input_clause_debug.contains("++equal(clause_tpos_format_a, clause_tpos_format_b)"));
        assert!(input_clause_debug.ends_with("\nocc: 2 7\n"));
        assert!(!input_clause_debug.contains("<-"));

        let wrapped_clause_debug = cell
            .format_debug_string(&bank, IoFormat::Tstp, ProblemType::FirstOrder)
            .unwrap_or_else(|err| panic!("{err}"));
        assert!(
            wrapped_clause_debug.starts_with("OLs: cnf(")
                || wrapped_clause_debug.starts_with("OLs: tcf(")
        );
        assert!(wrapped_clause_debug.contains("clause_tpos_format_a"));
        assert!(wrapped_clause_debug.ends_with("\nocc: 2 7\n"));
        assert!(!wrapped_clause_debug.contains("<-"));

        assert_eq!(
            cell.format_debug_string(&bank, IoFormat::Auto, ProblemType::FirstOrder)
                .unwrap_or_else(|err| panic!("{err}")),
            cell.lop_debug_string(&bank)
        );
    }
}
