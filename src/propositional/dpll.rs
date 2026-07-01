//! Port of the implemented parts of `PROPOSITIONAL/cpr_dpll`.

use crate::propositional::dpllformula::DpllFormula;
use crate::propositional::varset::AtomSet;
use crate::propositional::PLiteralCode;
use std::collections::BTreeSet;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DpllState {
    form: DpllFormula,
    assignment: Vec<PLiteralCode>,
    deactivated: Vec<Option<usize>>,
    unproc_units: Vec<usize>,
    open_atoms: AtomSet,
}

impl DpllState {
    /// C `DPLLStateAlloc`.
    #[must_use]
    pub fn new(form: DpllFormula) -> Self {
        let mut open_atoms = AtomSet::new();
        for atom in 1..form.atom_no() {
            if form
                .atom(atom)
                .is_some_and(|cell| cell.pos_occur() + cell.neg_occur() != 0)
            {
                open_atoms.insert(atom_code(atom));
            }
        }

        let mut unproc_units = Vec::new();
        for (index, clause) in form.clauses().iter().enumerate() {
            if clause.is_unit() {
                unproc_units.push(index);
            }
        }

        Self {
            form,
            assignment: Vec::new(),
            deactivated: Vec::new(),
            unproc_units,
            open_atoms,
        }
    }

    #[must_use]
    pub const fn form(&self) -> &DpllFormula {
        &self.form
    }

    #[must_use]
    pub fn assignment(&self) -> &[PLiteralCode] {
        &self.assignment
    }

    #[must_use]
    pub fn deactivated(&self) -> &[Option<usize>] {
        &self.deactivated
    }

    #[must_use]
    pub fn unprocessed_units(&self) -> &[usize] {
        &self.unproc_units
    }

    #[must_use]
    pub const fn open_atoms(&self) -> &AtomSet {
        &self.open_atoms
    }

    /// C `DPLLAssignVar`.
    ///
    /// The referenced C `deactivate_clauses` and `shorten_clauses` helpers are
    /// stubs that return zero without mutating the clause trees. Assigning that
    /// zero to the C `bool res` makes every currently implemented assignment
    /// return `false`.
    ///
    /// # Panics
    ///
    /// Panics if `assignment` does not index an allocated formula atom, matching
    /// the C direct `form->atoms[assignment]` access.
    pub fn assign_var(&mut self, assignment: PLiteralCode) -> bool {
        self.assignment.push(assignment);
        self.deactivated.push(None);

        let atom = literal_atom_index(assignment);
        let atom_cell = self
            .form
            .atom(atom)
            .unwrap_or_else(|| panic!("DPLL assignment atom is outside the formula atom table"));
        let pos_active = atom_cell.pos_active().clone();
        let neg_active = atom_cell.neg_active().clone();
        let _ = deactivate_clauses(self, &pos_active);
        shorten_clauses(self, &neg_active) != 0
    }
}

fn deactivate_clauses(_state: &mut DpllState, _clauses: &BTreeSet<usize>) -> i64 {
    0
}

fn shorten_clauses(_state: &mut DpllState, _clauses: &BTreeSet<usize>) -> i64 {
    0
}

fn atom_code(atom: usize) -> PLiteralCode {
    PLiteralCode::try_from(atom)
        .unwrap_or_else(|error| panic!("DPLL atom index does not fit PLiteralCode: {error}"))
}

fn literal_atom_index(lit: PLiteralCode) -> usize {
    let atom = lit
        .checked_abs()
        .unwrap_or_else(|| panic!("DPLL literal atom code overflowed while taking absolute value"));
    usize::try_from(atom)
        .unwrap_or_else(|error| panic!("DPLL literal atom code does not fit usize: {error}"))
}

#[cfg(test)]
mod tests {
    use super::DpllState;
    use crate::propositional::dpllformula::DpllFormula;
    use crate::propositional::propclauses::DpllClause;

    fn formula_with_units() -> DpllFormula {
        let mut formula = DpllFormula::new();
        formula.sig_mut().insert_atom("p");
        formula.sig_mut().insert_atom("q");
        formula.insert_clause(DpllClause::from_literals(vec![1]));
        formula.insert_clause(DpllClause::from_literals(vec![-2, 1]));
        formula
    }

    #[test]
    fn state_allocation_collects_open_atoms_and_unit_clauses() {
        let state = DpllState::new(formula_with_units());

        assert!(state.assignment().is_empty());
        assert!(state.deactivated().is_empty());
        assert_eq!(state.unprocessed_units(), &[0]);
        assert_eq!(state.open_atoms().iter().collect::<Vec<_>>(), vec![2, 1]);
    }

    #[test]
    fn assign_var_pushes_assignment_and_marker_and_returns_stub_false() {
        let mut state = DpllState::new(formula_with_units());

        assert!(!state.assign_var(1));

        assert_eq!(state.assignment(), &[1]);
        assert_eq!(state.deactivated(), &[None]);
        assert_eq!(state.form().atom(1).unwrap().pos_occur(), 2);
        assert_eq!(state.form().atom(2).unwrap().neg_occur(), 1);
    }

    #[test]
    fn assign_negative_var_preserves_current_c_stub_result_shape() {
        let mut state = DpllState::new(formula_with_units());

        assert!(!state.assign_var(-2));

        assert_eq!(state.assignment(), &[-2]);
        assert_eq!(state.deactivated(), &[None]);
    }

    #[test]
    #[should_panic(expected = "DPLL assignment atom is outside the formula atom table")]
    fn assign_var_panics_for_atom_outside_allocated_table() {
        let mut state = DpllState::new(DpllFormula::new());

        let _ = state.assign_var(1);
    }
}
