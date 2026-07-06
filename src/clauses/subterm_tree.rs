use crate::basics::simple_stuff::ProblemType;
use crate::clauses::clause::Clause;
use crate::clauses::clausecpos::CompactPos;
use crate::clauses::clausepos_tree::{clause_key, ClauseTPosTree};
use crate::terms::termbanks::TermBank;
use crate::terms::termtypes::{term_identity_id, DerefType, Term};
use std::cmp::Ordering;
use std::collections::{btree_map::Entry, BTreeMap};
use std::fmt::{self, Write};

#[derive(Clone, Debug)]
pub struct SubtermOcc {
    term_key: usize,
    term: Term,
    rw_rest: BTreeMap<i64, Clause>,
    rw_full: BTreeMap<i64, Clause>,
    clauses: ClauseTPosTree,
}

impl SubtermOcc {
    #[must_use]
    pub fn new(term: &Term) -> Self {
        Self {
            term_key: term_identity_id(term),
            term: term.clone(),
            rw_rest: BTreeMap::new(),
            rw_full: BTreeMap::new(),
            clauses: ClauseTPosTree::new(),
        }
    }

    #[must_use]
    pub const fn term_key(&self) -> usize {
        self.term_key
    }

    #[must_use]
    pub const fn term(&self) -> &Term {
        &self.term
    }

    #[must_use]
    pub const fn restricted_clauses(&self) -> &BTreeMap<i64, Clause> {
        &self.rw_rest
    }

    #[must_use]
    pub const fn full_clauses(&self) -> &BTreeMap<i64, Clause> {
        &self.rw_full
    }

    #[must_use]
    pub const fn position_clauses(&self) -> &ClauseTPosTree {
        &self.clauses
    }

    pub fn position_clauses_mut(&mut self) -> &mut ClauseTPosTree {
        &mut self.clauses
    }

    pub fn insert_occurrence(&mut self, clause: &Clause, restricted: bool) -> bool {
        let target = if restricted {
            &mut self.rw_rest
        } else {
            &mut self.rw_full
        };
        store_clause(target, clause)
    }

    pub fn delete_occurrence(&mut self, clause: &Clause, restricted: bool) -> bool {
        let target = if restricted {
            &mut self.rw_rest
        } else {
            &mut self.rw_full
        };
        target.remove(&clause_key(clause)).is_some()
    }

    #[must_use]
    pub fn is_unused(&self) -> bool {
        self.rw_rest.is_empty() && self.rw_full.is_empty() && self.clauses.is_empty()
    }
}

impl PartialEq for SubtermOcc {
    fn eq(&self, other: &Self) -> bool {
        self.term_key == other.term_key
    }
}

impl Eq for SubtermOcc {}

impl PartialOrd for SubtermOcc {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SubtermOcc {
    fn cmp(&self, other: &Self) -> Ordering {
        self.term_key.cmp(&other.term_key)
    }
}

#[derive(Clone, Debug, Default)]
pub struct SubtermTree {
    entries: BTreeMap<usize, SubtermOcc>,
}

impl SubtermTree {
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

    pub fn insert_term(&mut self, term: &Term) -> &mut SubtermOcc {
        match self.entries.entry(term_identity_id(term)) {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => entry.insert(SubtermOcc::new(term)),
        }
    }

    #[must_use]
    pub fn find_term(&self, term: &Term) -> Option<&SubtermOcc> {
        self.entries.get(&term_identity_id(term))
    }

    pub fn find_term_mut(&mut self, term: &Term) -> Option<&mut SubtermOcc> {
        self.entries.get_mut(&term_identity_id(term))
    }

    pub fn delete_term(&mut self, term: &Term) -> Option<SubtermOcc> {
        self.entries.remove(&term_identity_id(term))
    }

    pub fn insert_term_occ(&mut self, term: &Term, clause: &Clause, restricted: bool) -> bool {
        self.insert_term(term).insert_occurrence(clause, restricted)
    }

    pub fn delete_term_occ(&mut self, term: &Term, clause: &Clause, restricted: bool) -> bool {
        let key = term_identity_id(term);
        let Some(entry) = self.entries.get_mut(&key) else {
            return false;
        };
        let removed = entry.delete_occurrence(clause, restricted);
        if entry.is_unused() {
            self.entries.remove(&key);
        }
        removed
    }

    pub fn insert_clause_pos(&mut self, term: &Term, clause: &Clause, pos: CompactPos) -> bool {
        self.insert_term(term)
            .position_clauses_mut()
            .insert_pos(clause, pos)
    }

    pub fn delete_clause_pos(&mut self, term: &Term, clause: &Clause, pos: CompactPos) -> bool {
        let key = term_identity_id(term);
        let Some(entry) = self.entries.get_mut(&key) else {
            return false;
        };
        let removed = entry.position_clauses_mut().delete_pos(clause, pos);
        if entry.is_unused() {
            self.entries.remove(&key);
        }
        removed
    }

    pub fn entries(&self) -> impl Iterator<Item = &SubtermOcc> {
        self.entries.values()
    }

    pub fn write_debug(&self, output: &mut impl Write) -> fmt::Result {
        for entry in self.entries.values() {
            writeln!(
                output,
                "Key: {} f_code={}",
                entry.term.entry_no(),
                entry.term.f_code()
            )?;
        }
        Ok(())
    }

    #[must_use]
    pub fn debug_string(&self) -> String {
        let mut output = String::new();
        let _ = self.write_debug(&mut output);
        output
    }

    pub fn write_term_debug(&self, output: &mut impl Write, bank: &TermBank) -> fmt::Result {
        for entry in self.entries.values() {
            write!(output, "Key: {} = ", entry.term.entry_no())?;
            bank.write_term(output, &entry.term, true)?;
            writeln!(output)?;
        }
        Ok(())
    }

    #[must_use]
    pub fn term_debug_string(&self, bank: &TermBank) -> String {
        let mut output = String::new();
        let _ = self.write_term_debug(&mut output, bank);
        output
    }

    /// Writes the flattened C `SubtermTreePrintDot` record-label branch.
    ///
    /// The C source uses raw tree pointers for the subgraph and node names.
    /// Rust keeps that diagnostic-only shape with this `SubtermTree` object's
    /// address while preserving the term list and `DEREF_ALWAYS` term printer.
    ///
    /// # Panics
    ///
    /// Panics if a printed non-constant term has an uninitialized argument.
    pub fn write_dot_record(
        &self,
        output: &mut impl Write,
        bank: &TermBank,
        problem_type: ProblemType,
    ) -> fmt::Result {
        let dot_id = format!("{self:p}");
        write_subterm_occurrences_dot_record(
            output,
            &dot_id,
            self.entries.values(),
            bank,
            problem_type,
        )
    }

    #[must_use]
    pub fn dot_record_string(&self, bank: &TermBank, problem_type: ProblemType) -> String {
        let mut output = String::new();
        let _ = self.write_dot_record(&mut output, bank, problem_type);
        output
    }

    pub fn write_dot_dummy(&self, output: &mut impl Write) -> fmt::Result {
        writeln!(
            output,
            "     t{self:p} [shape=box label=\"{} terms\"]",
            self.len()
        )
    }

    #[must_use]
    pub fn dot_dummy_string(&self) -> String {
        let mut output = String::new();
        let _ = self.write_dot_dummy(&mut output);
        output
    }
}

pub fn write_subterm_occurrences_dot_record<'a>(
    output: &mut impl Write,
    dot_id: &str,
    occurrences: impl IntoIterator<Item = &'a SubtermOcc>,
    bank: &TermBank,
    problem_type: ProblemType,
) -> fmt::Result {
    writeln!(output, "     subgraph g{dot_id}{{")?;
    output.write_str("     nodesep=0.05\n")?;
    output.write_str(
        "     node [shape=record,width=1.9,height=.1, penwidth=0, style=filled, fillcolor=gray80]\n",
    )?;
    write!(output, "     t{dot_id} [label=\"{{|{{")?;
    let mut sep = "";
    for occurrence in occurrences {
        output.write_str(sep)?;
        sep = "|";
        bank.write_term_deref_for_problem(
            output,
            occurrence.term(),
            problem_type,
            DerefType::Always,
        )?;
    }
    output.write_str("}}\"]\n")?;
    output.write_str("     }\n")
}

#[must_use]
pub fn subterm_occurrences_dot_record_string<'a>(
    dot_id: &str,
    occurrences: impl IntoIterator<Item = &'a SubtermOcc>,
    bank: &TermBank,
    problem_type: ProblemType,
) -> String {
    let mut output = String::new();
    let _ =
        write_subterm_occurrences_dot_record(&mut output, dot_id, occurrences, bank, problem_type);
    output
}

#[must_use]
pub fn cmp_subterm_cells(left: &SubtermOcc, right: &SubtermOcc) -> i32 {
    match left.term_key.cmp(&right.term_key) {
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Greater => 1,
    }
}

fn store_clause(target: &mut BTreeMap<i64, Clause>, clause: &Clause) -> bool {
    match target.entry(clause_key(clause)) {
        Entry::Occupied(_) => false,
        Entry::Vacant(entry) => {
            entry.insert(clause.clone());
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{cmp_subterm_cells, SubtermOcc, SubtermTree};
    use crate::basics::simple_stuff::ProblemType;
    use crate::clauses::clause::Clause;
    use crate::clauses::eqn::Eqn;
    use crate::clauses::eqnlist::EqnList;
    use crate::terms::signature::Signature;
    use crate::terms::termbanks::TermBank;
    use crate::terms::termtypes::{term_identity_id, Term};
    use crate::terms::typebanks::TypeBank;

    fn test_bank() -> TermBank {
        let mut signature = Signature::new(TypeBank::new());
        signature.insert_internal_codes().unwrap();
        TermBank::new(signature).unwrap()
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
    fn occurrence_cells_preserve_term_identity_and_payload_sets() {
        let term = Term::const_cell_alloc(10);
        let clause = Box::new(unit_clause("a", 1));
        let mut occ = SubtermOcc::new(&term);

        assert_eq!(occ.term_key(), term_identity_id(&term));
        assert_eq!(occ.term(), &term);
        assert!(occ.insert_occurrence(&clause, true));
        assert!(!occ.insert_occurrence(&clause, true));
        assert_eq!(occ.restricted_clauses().len(), 1);
        assert!(occ.full_clauses().is_empty());
        assert!(occ.delete_occurrence(&clause, true));
        assert!(occ.is_unused());
    }

    #[test]
    fn tree_insert_find_and_delete_terms_by_term_identity() {
        let term = Term::const_cell_alloc(10);
        let same_shape = Term::const_cell_alloc(10);
        let mut tree = SubtermTree::new();

        let first_key = tree.insert_term(&term).term_key();
        assert_eq!(first_key, term_identity_id(&term));
        assert!(tree.find_term(&term).is_some());
        assert!(tree.find_term(&same_shape).is_none());
        assert_eq!(tree.len(), 1);
        assert!(tree.delete_term(&term).is_some());
        assert!(tree.is_empty());
    }

    #[test]
    fn occurrence_insert_delete_removes_empty_nodes_like_c() {
        let term = Term::const_cell_alloc(20);
        let clause = Box::new(unit_clause("a", 1));
        let other = Box::new(unit_clause("b", 2));
        let mut tree = SubtermTree::new();

        assert!(tree.insert_term_occ(&term, &clause, true));
        assert!(!tree.insert_term_occ(&term, &clause, true));
        assert!(tree.insert_term_occ(&term, &other, false));
        assert_eq!(tree.find_term(&term).unwrap().restricted_clauses().len(), 1);
        assert_eq!(tree.find_term(&term).unwrap().full_clauses().len(), 1);

        assert!(tree.delete_term_occ(&term, &clause, true));
        assert!(tree.find_term(&term).is_some());
        assert!(tree.delete_term_occ(&term, &other, false));
        assert!(tree.find_term(&term).is_none());
        assert!(!tree.delete_term_occ(&term, &other, false));
    }

    #[test]
    fn overlap_positions_share_clause_position_tree_payload() {
        let term = Term::const_cell_alloc(30);
        let clause = Box::new(unit_clause("a", 1));
        let mut tree = SubtermTree::new();

        assert!(tree.insert_clause_pos(&term, &clause, 4));
        assert!(!tree.insert_clause_pos(&term, &clause, 4));
        assert!(tree.insert_clause_pos(&term, &clause, 8));
        let positions = tree
            .find_term(&term)
            .unwrap()
            .position_clauses()
            .find(&clause)
            .unwrap()
            .positions()
            .iter()
            .copied()
            .collect::<Vec<_>>();
        assert_eq!(positions, vec![4, 8]);
        assert!(tree.delete_clause_pos(&term, &clause, 4));
        assert!(tree.find_term(&term).is_some());
        assert!(tree.delete_clause_pos(&term, &clause, 8));
        assert!(tree.find_term(&term).is_none());
    }

    #[test]
    fn comparison_and_debug_output_use_identity_order_and_entry_numbers() {
        let left = Term::const_cell_alloc(40);
        left.set_entry_no(5);
        let right = Term::const_cell_alloc(41);
        let left_occ = SubtermOcc::new(&left);
        let right_occ = SubtermOcc::new(&right);
        let mut tree = SubtermTree::new();
        tree.insert_term(&left);

        assert_eq!(cmp_subterm_cells(&left_occ, &left_occ), 0);
        assert_ne!(cmp_subterm_cells(&left_occ, &right_occ), 0);
        assert_eq!(tree.debug_string(), "Key: 5 f_code=40\n");
    }

    #[test]
    fn term_debug_output_uses_explicit_term_bank_rendering() {
        let mut bank = test_bank();
        let left = typed_const(&mut bank, "subterm_print_a");
        left.set_entry_no(7);
        let mut tree = SubtermTree::new();
        tree.insert_term(&left);

        assert_eq!(tree.term_debug_string(&bank), "Key: 7 = subterm_print_a\n");
    }

    #[test]
    fn dot_record_output_matches_c_flattened_payload_shape() {
        let mut bank = test_bank();
        let left = typed_const(&mut bank, "subterm_dot_left");
        let right = typed_const(&mut bank, "subterm_dot_right");
        let equality = Term::top_alloc(bank.signature().eqn_code(), 2);
        equality.set_type(Some(bank.signature().type_bank().bool_type()));
        equality.set_argument(0, left.clone());
        equality.set_argument(1, right.clone());
        let equality = bank.term_top_insert(equality).unwrap();
        let mut tree = SubtermTree::new();
        tree.insert_term(&left);
        tree.insert_term(&equality);

        let dot = tree.dot_record_string(&bank, ProblemType::FirstOrder);

        assert!(dot.starts_with("     subgraph g"));
        assert!(dot.contains("     nodesep=0.05\n"));
        assert!(dot.contains(
            "     node [shape=record,width=1.9,height=.1, penwidth=0, \
             style=filled, fillcolor=gray80]\n"
        ));
        assert!(dot.contains(" [label=\"{|{"));
        assert!(dot.contains("subterm_dot_left"));
        assert!(dot.contains("subterm_dot_left=subterm_dot_right"));
        assert!(dot.ends_with("     }\n"));
    }

    #[test]
    fn dot_dummy_output_counts_terms_like_c_debug_helper() {
        let mut bank = test_bank();
        let left = typed_const(&mut bank, "subterm_dummy_left");
        let right = typed_const(&mut bank, "subterm_dummy_right");
        let mut tree = SubtermTree::new();
        tree.insert_term(&left);
        tree.insert_term(&right);

        let dummy = tree.dot_dummy_string();

        assert!(dummy.starts_with("     t"));
        assert!(dummy.ends_with(" [shape=box label=\"2 terms\"]\n"));
    }
}
