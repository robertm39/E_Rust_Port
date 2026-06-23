use crate::basics::error::Diagnostic;
use crate::clauses::clause::Clause;
use crate::clauses::eqn::Eqn;
use crate::clauses::eqn_props::EqnProperties;
use crate::clauses::eqnlist::EqnList;
use crate::terms::signature::SIG_TRUE_CODE;
use crate::terms::termbanks::TermBank;
use crate::terms::termtypes::Term;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PropLit {
    properties: EqnProperties,
    lit: Term,
}

impl PropLit {
    #[must_use]
    pub const fn new(properties: EqnProperties, lit: Term) -> Self {
        Self { properties, lit }
    }

    #[must_use]
    pub const fn properties(&self) -> EqnProperties {
        self.properties
    }

    #[must_use]
    pub const fn literal(&self) -> &Term {
        &self.lit
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PropClause {
    literals: Vec<PropLit>,
}

impl PropClause {
    /// Allocates a compact propositional clause from an ordinary clause.
    ///
    /// # Panics
    ///
    /// Panics if any literal is not a predicate literal ending in `$true`,
    /// matching the C assertion that propositional clauses only store atom
    /// terms.
    #[must_use]
    pub fn alloc(clause: &Clause) -> Self {
        let mut literals = Vec::with_capacity(clause.literal_number());
        for literal in clause.literals().as_slice() {
            assert_eq!(
                literal.right().f_code(),
                SIG_TRUE_CODE,
                "propositional clauses require predicate literals"
            );
            literals.push(PropLit::new(literal.properties(), literal.left().clone()));
        }
        Self { literals }
    }

    #[must_use]
    pub fn literals(&self) -> &[PropLit] {
        &self.literals
    }

    #[must_use]
    pub const fn lit_no(&self) -> usize {
        self.literals.len()
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.literals.is_empty()
    }

    /// Rebuilds an ordinary clause from this compact propositional clause.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if allocating any rebuilt literal fails.
    pub fn to_clause(&self, bank: &mut TermBank) -> Result<Clause, Diagnostic> {
        let mut literals = Vec::with_capacity(self.literals.len());
        let true_term = bank.true_term().clone();
        for prop_lit in &self.literals {
            let mut literal = Eqn::alloc(prop_lit.lit.clone(), true_term.clone(), bank, false)?;
            literal.set_properties(prop_lit.properties);
            literals.push(literal);
        }
        Ok(Clause::alloc(EqnList::from_vec(literals)))
    }

    #[must_use]
    pub fn max_var(&self) -> i64 {
        self.literals
            .iter()
            .map(|literal| literal.lit.entry_no())
            .max()
            .unwrap_or(0)
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PropClauseSet {
    members: i64,
    literals: i64,
    empty_clauses: i64,
    clauses: Vec<PropClause>,
}

impl PropClauseSet {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            members: 0,
            literals: 0,
            empty_clauses: 0,
            clauses: Vec::new(),
        }
    }

    #[must_use]
    pub const fn members(&self) -> i64 {
        self.members
    }

    #[must_use]
    pub const fn literal_count(&self) -> i64 {
        self.literals
    }

    #[must_use]
    pub const fn empty_clause_count(&self) -> i64 {
        self.empty_clauses
    }

    #[must_use]
    pub fn clauses(&self) -> &[PropClause] {
        &self.clauses
    }

    pub fn insert_prop_clause(&mut self, clause: PropClause) -> i64 {
        self.members += 1;
        self.literals += usize_to_i64(clause.lit_no());
        if clause.is_empty() {
            self.empty_clauses += 1;
        }
        self.clauses.push(clause);
        self.members
    }

    pub fn insert_clause(&mut self, clause: Clause) -> i64 {
        let prop_clause = PropClause::alloc(&clause);
        drop(clause);
        self.insert_prop_clause(prop_clause)
    }

    #[must_use]
    pub fn max_var(&self) -> i64 {
        self.clauses
            .iter()
            .map(PropClause::max_var)
            .max()
            .unwrap_or(0)
    }
}

fn usize_to_i64(value: usize) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

#[cfg(test)]
mod tests {
    use super::{PropClause, PropClauseSet};
    use crate::clauses::clause::Clause;
    use crate::clauses::eqn::Eqn;
    use crate::clauses::eqn_props::{EP_IS_POSITIVE, EP_IS_SELECTED};
    use crate::clauses::eqnlist::EqnList;
    use crate::terms::signature::Signature;
    use crate::terms::termbanks::TermBank;
    use crate::terms::termtypes::{DerefType, Term};
    use crate::terms::typebanks::TypeBank;

    fn test_bank() -> TermBank {
        let mut signature = Signature::new(TypeBank::new());
        signature.insert_internal_codes().unwrap();
        TermBank::new(signature).unwrap()
    }

    fn predicate_atom(bank: &mut TermBank, name: &str) -> Term {
        let bool_type = bank.signature().type_bank().bool_type();
        let f_code = bank.signature_mut().insert_id(name, 0, false);
        bank.signature_mut()
            .declare_final_type(f_code, bool_type)
            .unwrap();
        bank.create_const_term(f_code).unwrap()
    }

    fn predicate_literal(bank: &mut TermBank, atom: &Term, positive: bool) -> Eqn {
        Eqn::alloc(atom.clone(), bank.true_term().clone(), bank, positive).unwrap()
    }

    fn clause_from(literals: Vec<Eqn>) -> Clause {
        Clause::alloc(EqnList::from_vec(literals))
    }

    #[test]
    fn prop_clause_alloc_snapshots_predicate_literals_in_clause_order() {
        let mut bank = test_bank();
        let first = predicate_atom(&mut bank, "p");
        let second = predicate_atom(&mut bank, "q");
        let mut negative = predicate_literal(&mut bank, &second, false);
        negative.set_prop(EP_IS_SELECTED);
        let clause = clause_from(vec![negative, predicate_literal(&mut bank, &first, true)]);

        let prop_clause = PropClause::alloc(&clause);

        assert_eq!(prop_clause.lit_no(), 2);
        assert_eq!(prop_clause.literals()[0].literal(), &first);
        assert!(prop_clause.literals()[0].properties().query(EP_IS_POSITIVE));
        assert_eq!(prop_clause.literals()[1].literal(), &second);
        assert!(prop_clause.literals()[1].properties().query(EP_IS_SELECTED));
    }

    #[test]
    fn prop_clause_to_clause_rebuilds_literals_and_preserves_properties() {
        let mut bank = test_bank();
        let first = predicate_atom(&mut bank, "p");
        let second = predicate_atom(&mut bank, "q");
        let mut negative = predicate_literal(&mut bank, &second, false);
        negative.set_prop(EP_IS_SELECTED);
        let source = clause_from(vec![predicate_literal(&mut bank, &first, true), negative]);
        let prop_clause = PropClause::alloc(&source);

        let rebuilt = prop_clause.to_clause(&mut bank).unwrap();

        assert_eq!(rebuilt.literal_number(), 2);
        let rebuilt_lits = rebuilt.literals().as_slice();
        assert_eq!(rebuilt_lits[0].left(), &first);
        assert_eq!(rebuilt_lits[0].right(), bank.true_term());
        assert!(rebuilt_lits[0].is_positive());
        assert_eq!(rebuilt_lits[1].left(), &second);
        assert_eq!(rebuilt_lits[1].right(), bank.true_term());
        assert!(rebuilt_lits[1].is_negative());
        assert!(rebuilt_lits[1].query_prop(EP_IS_SELECTED));
    }

    #[test]
    fn max_var_uses_current_literal_entry_numbers() {
        let mut bank = test_bank();
        let first = predicate_atom(&mut bank, "p");
        let second = predicate_atom(&mut bank, "q");
        first.set_entry_no(17);
        second.set_entry_no(41);
        let prop_clause = PropClause::alloc(&clause_from(vec![
            predicate_literal(&mut bank, &first, true),
            predicate_literal(&mut bank, &second, false),
        ]));

        assert_eq!(prop_clause.max_var(), 41);
        second.set_entry_no(9);
        assert_eq!(prop_clause.max_var(), 17);
    }

    #[test]
    fn prop_clause_set_insertion_updates_append_order_and_stats() {
        let mut bank = test_bank();
        let first = predicate_atom(&mut bank, "p");
        let second = predicate_atom(&mut bank, "q");
        first.set_entry_no(11);
        second.set_entry_no(29);
        let unit = PropClause::alloc(&clause_from(vec![predicate_literal(
            &mut bank, &first, true,
        )]));
        let binary = clause_from(vec![
            predicate_literal(&mut bank, &first, true),
            predicate_literal(&mut bank, &second, false),
        ]);
        let mut set = PropClauseSet::new();

        assert_eq!(set.insert_prop_clause(unit.clone()), 1);
        assert_eq!(set.insert_clause(binary), 2);
        assert_eq!(set.insert_prop_clause(PropClause::default()), 3);

        assert_eq!(set.members(), 3);
        assert_eq!(set.literal_count(), 3);
        assert_eq!(set.empty_clause_count(), 1);
        assert_eq!(set.clauses()[0], unit);
        assert_eq!(set.clauses()[1].lit_no(), 2);
        assert!(set.clauses()[2].is_empty());
        assert_eq!(set.max_var(), 29);
    }

    #[test]
    fn empty_prop_clause_round_trips_to_empty_clause() {
        let mut bank = test_bank();
        let prop_clause = PropClause::alloc(&Clause::empty());
        let rebuilt = prop_clause.to_clause(&mut bank).unwrap();

        assert!(prop_clause.is_empty());
        assert!(rebuilt.is_empty());
    }

    #[test]
    #[should_panic(expected = "propositional clauses require predicate literals")]
    fn prop_clause_alloc_rejects_equational_literals_like_c_assertion() {
        let mut bank = test_bank();
        let type_ = bank.signature().type_bank().default_type();
        let a_code = bank.signature_mut().insert_id("a", 0, false);
        let b_code = bank.signature_mut().insert_id("b", 0, false);
        bank.signature_mut()
            .declare_final_type(a_code, type_.clone())
            .unwrap();
        bank.signature_mut()
            .declare_final_type(b_code, type_)
            .unwrap();
        let left = bank
            .insert(&Term::const_cell_alloc(a_code), DerefType::Never)
            .unwrap();
        let right = bank
            .insert(&Term::const_cell_alloc(b_code), DerefType::Never)
            .unwrap();
        let equation = Eqn::alloc(left, right, &mut bank, true).unwrap();

        let _ = PropClause::alloc(&clause_from(vec![equation]));
    }
}
