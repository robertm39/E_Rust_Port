//! Port of `PROPOSITIONAL/cpr_dpllformula`.

use crate::basics::defines::DEFAULT_COMCHAR_RAW;
use crate::basics::error::Diagnostic;
use crate::basics::simple_stuff::ProblemType;
use crate::clauses::clause::{clause_parse, clause_starts_maybe};
use crate::inout::scanner::Scanner;
use crate::propositional::propclauses::{DpllClause, DpllOutputFormat};
use crate::propositional::propsig::PropSig;
use crate::propositional::PLiteralCode;
use crate::terms::termbanks::TermBank;
use std::collections::BTreeSet;
use std::fmt::Write as _;

pub const DEFAULT_ATOM_NUMBER: usize = 500;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct AtomCell {
    pos_occur: i64,
    neg_occur: i64,
    pos_active: BTreeSet<usize>,
    neg_active: BTreeSet<usize>,
}

impl AtomCell {
    #[must_use]
    pub const fn pos_occur(&self) -> i64 {
        self.pos_occur
    }

    #[must_use]
    pub const fn neg_occur(&self) -> i64 {
        self.neg_occur
    }

    #[must_use]
    pub const fn pos_active(&self) -> &BTreeSet<usize> {
        &self.pos_active
    }

    #[must_use]
    pub const fn neg_active(&self) -> &BTreeSet<usize> {
        &self.neg_active
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DpllFormula {
    sig: PropSig,
    clauses: Vec<DpllClause>,
    atoms: Vec<AtomCell>,
}

impl Default for DpllFormula {
    fn default() -> Self {
        Self::new()
    }
}

impl DpllFormula {
    /// C `DPLLFormulaAlloc`.
    #[must_use]
    pub fn new() -> Self {
        Self {
            sig: PropSig::new(),
            clauses: Vec::new(),
            atoms: Vec::new(),
        }
    }

    #[must_use]
    pub const fn sig(&self) -> &PropSig {
        &self.sig
    }

    pub fn sig_mut(&mut self) -> &mut PropSig {
        &mut self.sig
    }

    #[must_use]
    pub fn clauses(&self) -> &[DpllClause] {
        &self.clauses
    }

    /// C `form->atom_no`, the allocated atom-table size rather than the number
    /// of atoms that occur.
    #[must_use]
    pub fn atom_no(&self) -> usize {
        self.atoms.len()
    }

    #[must_use]
    pub fn atom(&self, atom: usize) -> Option<&AtomCell> {
        self.atoms.get(atom)
    }

    /// C `DPLLRegisterClauseLiteral`.
    ///
    /// # Panics
    ///
    /// Panics if registering the same clause index at the same signed atom more
    /// than once, matching the C `PTreeStore` duplicate assertion.
    pub fn register_clause_literal(&mut self, clause_index: usize, lit: PLiteralCode) {
        let atom = literal_atom_index(lit);
        self.ensure_atom_space(atom);
        let cell = &mut self.atoms[atom];
        if lit > 0 {
            cell.pos_occur += 1;
            assert!(
                cell.pos_active.insert(clause_index),
                "Duplicate entry of a clause!"
            );
        } else {
            cell.neg_occur += 1;
            assert!(
                cell.neg_active.insert(clause_index),
                "Duplicate entry of a clause!"
            );
        }
    }

    /// C `DPLLFormulaInsertClause`.
    ///
    /// The clause is expected to be normalized before insertion, just as in C.
    pub fn insert_clause(&mut self, clause: DpllClause) {
        let clause_index = self.clauses.len();
        let literals = clause.literals().to_vec();
        self.clauses.push(clause);
        for literal in literals {
            self.register_clause_literal(clause_index, literal);
        }
    }

    /// C `DPLLFormulaPrint`.
    ///
    /// # Panics
    ///
    /// Panics for `DpllOutputFormat::NoFormat`, matching the C assertion in the
    /// default switch arm.
    #[must_use]
    pub fn print_string(&self, format: DpllOutputFormat, print_atoms: bool) -> String {
        let mut output = String::new();
        if print_atoms {
            output.push_str(&self.sig.print_string());
            for (atom, cell) in self.atoms.iter().enumerate() {
                if cell.pos_occur != 0 || cell.neg_occur != 0 {
                    let _ = writeln!(
                        output,
                        "{DEFAULT_COMCHAR_RAW} {atom:4}: {:4} {:4}",
                        cell.pos_occur, cell.pos_occur
                    );
                }
            }
        }
        for clause in &self.clauses {
            match format {
                DpllOutputFormat::Lop => {
                    output.push_str(&clause.print_lop_string(&self.sig));
                    output.push('\n');
                }
                DpllOutputFormat::Dimacs => output.push_str(&clause.print_dimacs_string()),
                DpllOutputFormat::NoFormat => panic!("Not a valid DPLLOutputFormat"),
            }
        }
        output
    }

    /// C `DPLLFormulaParseLOP`, adapted to Rust's explicit `TermBank`.
    ///
    /// The returned string is the C progress text that the original writes to
    /// `GlobalOut`.
    ///
    /// # Errors
    ///
    /// Returns parser diagnostics or DPLL clause-conversion diagnostics.
    pub fn parse_lop(
        &mut self,
        scanner: &mut Scanner,
        bank: &mut TermBank,
        problem_type: ProblemType,
    ) -> Result<String, Diagnostic> {
        let mut trace = String::new();
        self.parse_lop_with_trace(scanner, bank, problem_type, |line| {
            trace.push_str(line);
            Ok(())
        })?;
        Ok(trace)
    }

    /// C `DPLLFormulaParseLOP` with incremental trace delivery.
    ///
    /// Unlike [`Self::parse_lop`], this preserves trace lines already emitted
    /// when parsing a later clause fails, matching C's writes to `GlobalOut`.
    ///
    /// # Errors
    ///
    /// Returns parser, clause-conversion, or trace-sink diagnostics.
    pub fn parse_lop_with_trace(
        &mut self,
        scanner: &mut Scanner,
        bank: &mut TermBank,
        problem_type: ProblemType,
        mut trace_sink: impl FnMut(&str) -> Result<(), Diagnostic>,
    ) -> Result<(), Diagnostic> {
        while clause_starts_maybe(scanner) {
            let clause = clause_parse(scanner, bank, problem_type)?;
            let mut pclause = DpllClause::from_clause(&mut self.sig, bank, &clause)?;
            let mut trace = String::from("New clause: ");
            trace.push_str(&pclause.print_lop_string(&self.sig));
            if pclause.normalize() {
                trace.push_str("...discarded (tautology)\n");
            } else {
                self.insert_clause(pclause);
                trace.push_str("...accepted\n");
            }
            trace_sink(&trace)?;
        }
        Ok(())
    }

    fn ensure_atom_space(&mut self, atom: usize) {
        while self.atoms.len() <= atom {
            self.add_atom_space();
        }
    }

    /// C `dpll_form_add_atom_space`.
    fn add_atom_space(&mut self) {
        let old_limit = self.atoms.len();
        let new_limit = if old_limit == 0 {
            DEFAULT_ATOM_NUMBER
        } else {
            old_limit + (old_limit / 2)
        };
        self.atoms.resize_with(new_limit, AtomCell::default);
    }
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
    use super::{DpllFormula, DEFAULT_ATOM_NUMBER};
    use crate::basics::simple_stuff::ProblemType;
    use crate::inout::scanner::Scanner;
    use crate::propositional::propclauses::{DpllClause, DpllOutputFormat};
    use crate::terms::signature::Signature;
    use crate::terms::termbanks::TermBank;
    use crate::terms::typebanks::TypeBank;

    fn test_bank() -> TermBank {
        let mut signature = Signature::new(TypeBank::new());
        signature.insert_internal_codes().unwrap();
        TermBank::new(signature).unwrap()
    }

    #[test]
    fn new_formula_is_empty_and_allocates_atoms_lazily() {
        let formula = DpllFormula::new();

        assert_eq!(formula.sig().atom_number(), 1);
        assert!(formula.clauses().is_empty());
        assert_eq!(formula.atom_no(), 0);
    }

    #[test]
    fn register_clause_literal_grows_atom_table_and_records_sign_specific_sets() {
        let mut formula = DpllFormula::new();

        formula.register_clause_literal(4, 3);
        formula.register_clause_literal(7, -3);

        assert_eq!(formula.atom_no(), DEFAULT_ATOM_NUMBER);
        let atom = formula.atom(3).unwrap();
        assert_eq!(atom.pos_occur(), 1);
        assert_eq!(atom.neg_occur(), 1);
        assert!(atom.pos_active().contains(&4));
        assert!(atom.neg_active().contains(&7));
    }

    #[test]
    #[should_panic(expected = "Duplicate entry of a clause!")]
    fn duplicate_clause_registration_panics_like_c_ptree_store_assertion() {
        let mut formula = DpllFormula::new();

        formula.register_clause_literal(1, 2);
        formula.register_clause_literal(1, 2);
    }

    #[test]
    fn insert_clause_pushes_clause_and_registers_all_literals() {
        let mut formula = DpllFormula::new();
        let clause = DpllClause::from_literals(vec![1, -2]);

        formula.insert_clause(clause);

        assert_eq!(formula.clauses().len(), 1);
        assert_eq!(formula.atom(1).unwrap().pos_occur(), 1);
        assert_eq!(formula.atom(2).unwrap().neg_occur(), 1);
        assert!(formula.atom(1).unwrap().pos_active().contains(&0));
        assert!(formula.atom(2).unwrap().neg_active().contains(&0));
    }

    #[test]
    fn print_string_renders_atoms_and_clauses_in_c_shape() {
        let mut formula = DpllFormula::new();
        formula.sig_mut().insert_atom("p");
        formula.sig_mut().insert_atom("q");
        formula.insert_clause(DpllClause::from_literals(vec![1, -2]));

        assert_eq!(
            formula.print_string(DpllOutputFormat::Lop, true),
            "% Propositional signature:\n\
             % ------------------------\n\
             %      1 : p\n\
             %      2 : q\n\n\
             %    1:    1    1\n\
             %    2:    0    0\n\
             p<-q.\n"
        );
        assert_eq!(
            formula.print_string(DpllOutputFormat::Dimacs, false),
            "1 -2 0\n"
        );
    }

    #[test]
    #[should_panic(expected = "Not a valid DPLLOutputFormat")]
    fn print_string_rejects_no_format_like_c_assertion() {
        let mut formula = DpllFormula::new();
        formula.insert_clause(DpllClause::from_literals(vec![1]));

        let _ = formula.print_string(DpllOutputFormat::NoFormat, false);
    }

    #[test]
    fn parse_lop_accepts_normalized_clauses_and_discards_tautologies() {
        let mut formula = DpllFormula::new();
        let mut bank = test_bank();
        let mut scanner = Scanner::from_user_string("p <- q. r <- r.", false).unwrap();

        let trace = formula
            .parse_lop(&mut scanner, &mut bank, ProblemType::FirstOrder)
            .unwrap();

        assert_eq!(
            trace,
            "New clause: p<-q....accepted\nNew clause: r<-r....discarded (tautology)\n"
        );
        assert_eq!(formula.clauses().len(), 1);
        assert_eq!(formula.clauses()[0].literals(), &[1, -2]);
        assert_eq!(formula.sig().atom_name(1), "p");
        assert_eq!(formula.sig().atom_name(2), "q");
        assert_eq!(formula.sig().atom_name(3), "r");
    }
}
