use crate::basics::error::Diagnostic;
use crate::clauses::clause::Clause;
use crate::clauses::eqn::Eqn;
use crate::clauses::eqn_props::EP_IS_EQU_LITERAL;
use crate::terms::functypes::FunCode;
use crate::terms::signature::Signature;
use crate::terms::termbanks::TermBank;
use crate::terms::termtypes::{term_identity_id, Term};
use std::collections::BTreeMap;

pub type TermIdentitySet = BTreeMap<usize, Term>;
pub type VarConstraintMap = BTreeMap<i64, TermIdentitySet>;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct LitConstrCell {
    constrained: bool,
    constraints: TermIdentitySet,
}

impl LitConstrCell {
    #[must_use]
    pub fn constrained() -> Self {
        Self {
            constrained: true,
            constraints: TermIdentitySet::new(),
        }
    }

    #[must_use]
    pub const fn is_constrained(&self) -> bool {
        self.constrained
    }

    pub const fn set_constrained(&mut self, value: bool) {
        self.constrained = value;
    }

    #[must_use]
    pub const fn constraints(&self) -> &TermIdentitySet {
        &self.constraints
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LitOccTable {
    sig_size: usize,
    maxarity: usize,
    matrix: Vec<LitConstrCell>,
}

impl LitOccTable {
    /// Allocates a literal-occurrence table sized for the current signature.
    ///
    /// # Panics
    ///
    /// Panics if the signature symbol count or predicate arity cannot be
    /// represented as `usize`.
    #[must_use]
    pub fn alloc(sig: &Signature) -> Self {
        let sig_size = usize::try_from(sig.f_count() + 1).expect("signature size fits usize");
        let max_predicate_arity =
            usize::try_from(sig.find_max_predicate_arity()).expect("predicate arity fits usize");
        let maxarity = max_predicate_arity.max(2);
        let matrix = vec![LitConstrCell::constrained(); sig_size * (maxarity + 1)];
        Self {
            sig_size,
            maxarity,
            matrix,
        }
    }

    #[must_use]
    pub const fn sig_size(&self) -> usize {
        self.sig_size
    }

    #[must_use]
    pub const fn maxarity(&self) -> usize {
        self.maxarity
    }

    #[must_use]
    pub fn constr_state(&self, pred: FunCode, pos: usize) -> bool {
        self.entry(pred, pos).is_constrained()
    }

    pub fn set_constr_state(&mut self, pred: FunCode, pos: usize, value: bool) {
        self.entry_mut(pred, pos).set_constrained(value);
    }

    /// Returns the constraint set for a constrained literal position.
    ///
    /// # Panics
    ///
    /// Panics if the position is currently unconstrained, matching the C
    /// assertion in `LitPosGetConstraints`.
    #[must_use]
    pub fn constraints(&self, pred: FunCode, pos: usize) -> &TermIdentitySet {
        let entry = self.entry(pred, pos);
        assert!(
            entry.is_constrained(),
            "constraints can only be read for constrained literal positions"
        );
        entry.constraints()
    }

    #[must_use]
    pub fn add_constraint(&mut self, pred: FunCode, pos: usize, term: &Term) -> bool {
        if term.is_free_var() {
            self.set_constr_state(pred, pos, false);
            true
        } else {
            self.entry_mut(pred, pos)
                .constraints
                .insert(term_identity_id(term), term.clone());
            false
        }
    }

    fn entry(&self, pred: FunCode, pos: usize) -> &LitConstrCell {
        let index = self.index(pred, pos);
        &self.matrix[index]
    }

    fn entry_mut(&mut self, pred: FunCode, pos: usize) -> &mut LitConstrCell {
        let index = self.index(pred, pos);
        &mut self.matrix[index]
    }

    fn index(&self, pred: FunCode, pos: usize) -> usize {
        assert!(pred >= 0, "literal predicate code must be non-negative");
        let pred = usize::try_from(pred).expect("literal predicate code fits usize");
        assert!(pred < self.sig_size, "literal predicate code out of range");
        assert!(
            pos < self.maxarity,
            "literal argument position out of range"
        );
        (self.sig_size * pos) + pred
    }
}

/// Adds constraints induced by one predicate literal into the matching sign table.
///
/// # Panics
///
/// Panics if `eqn` is still an equational literal, if any literal argument is
/// uninitialized, or if the predicate/position is outside the table bounds.
pub fn lit_occ_add_lit_alt(
    positive_table: &mut LitOccTable,
    negative_table: &mut LitOccTable,
    eqn: &Eqn,
) {
    assert!(
        !eqn.query_prop(EP_IS_EQU_LITERAL),
        "literal-occurrence tables require predicate literals"
    );
    let target = if eqn.is_positive() {
        positive_table
    } else {
        negative_table
    };
    let lit = eqn.left();
    for index in 0..lit.arity() {
        let arg = lit
            .argument(index)
            .unwrap_or_else(|| panic!("literal argument {index} is uninitialized"));
        let _ = target.add_constraint(lit.f_code(), index, &arg);
    }
}

/// Adds constraints induced by every literal in one clause.
///
/// # Panics
///
/// Panics under the same preconditions as [`lit_occ_add_lit_alt`].
pub fn lit_occ_add_clause_alt(
    positive_table: &mut LitOccTable,
    negative_table: &mut LitOccTable,
    clause: &Clause,
) {
    for literal in clause.literals().as_slice() {
        lit_occ_add_lit_alt(positive_table, negative_table, literal);
    }
}

/// Adds constraints induced by a slice of clauses.
///
/// # Panics
///
/// Panics under the same preconditions as [`lit_occ_add_lit_alt`].
pub fn lit_occ_add_clause_slice_alt(
    positive_table: &mut LitOccTable,
    negative_table: &mut LitOccTable,
    clauses: &[Clause],
) {
    for clause in clauses {
        lit_occ_add_clause_alt(positive_table, negative_table, clause);
    }
}

/// Pushes all usable constant terms, or one requested constant, into `stack`.
///
/// # Errors
///
/// Returns a diagnostic if inserting a collected constant into the term bank
/// fails.
///
/// # Panics
///
/// Panics if `uniq` names no existing arity-zero symbol, matching the C
/// assertions in `SigCollectConstantTerms`.
pub fn sig_collect_constant_terms(
    bank: &mut TermBank,
    stack: &mut Vec<Term>,
    uniq: Option<FunCode>,
) -> Result<i64, Diagnostic> {
    if let Some(f_code) = uniq {
        assert!(
            f_code > 0 && f_code <= bank.signature().f_count(),
            "unique constant code must name a signature symbol"
        );
        assert_eq!(
            bank.signature().find_arity(f_code),
            Some(0),
            "unique constant code must have arity zero"
        );
        stack.push(bank.create_const_term(f_code)?);
        return Ok(1);
    }

    let constants = collect_signature_constant_codes(bank.signature());
    for f_code in &constants {
        stack.push(bank.create_const_term(*f_code)?);
    }
    let mut result = i64::try_from(constants.len()).unwrap_or(i64::MAX);
    if result == 0 {
        stack.push(bank.alloc_new_skolem(&[], None)?);
        result = 1;
    }
    Ok(result)
}

/// Intersects variable alternatives with constraints from opposite-sign literals.
///
/// # Panics
///
/// Panics if a literal argument is uninitialized or if a predicate/position is
/// outside either table's bounds.
pub fn eqn_collect_var_constr(
    positive_table: &LitOccTable,
    negative_table: &LitOccTable,
    var_constr: &mut VarConstraintMap,
    eqn: &Eqn,
) {
    let constraints = if eqn.is_positive() {
        negative_table
    } else {
        positive_table
    };
    let lit = eqn.left();
    for index in 0..lit.arity() {
        let arg = lit
            .argument(index)
            .unwrap_or_else(|| panic!("literal argument {index} is uninitialized"));
        if arg.is_free_var() && constraints.constr_state(lit.f_code(), index) {
            let allowed = constraints.constraints(lit.f_code(), index);
            let var_key = -arg.f_code();
            let alternatives = var_constr.entry(var_key).or_default();
            intersect_identity_sets(alternatives, allowed);
        }
    }
}

/// Applies variable-constraint collection to every literal in a clause.
///
/// # Panics
///
/// Panics under the same preconditions as [`eqn_collect_var_constr`].
pub fn clause_collect_var_constr(
    positive_table: &LitOccTable,
    negative_table: &LitOccTable,
    clause: &Clause,
    _ground_terms: &TermIdentitySet,
    var_constr: &mut VarConstraintMap,
) {
    for literal in clause.literals().as_slice() {
        eqn_collect_var_constr(positive_table, negative_table, var_constr, literal);
    }
}

#[must_use]
pub fn term_identity_set_from_terms(terms: &[Term]) -> TermIdentitySet {
    let mut result = TermIdentitySet::new();
    for term in terms {
        result.insert(term_identity_id(term), term.clone());
    }
    result
}

fn collect_signature_constant_codes(sig: &Signature) -> Vec<FunCode> {
    let mut result = Vec::new();
    for f_code in (sig.internal_symbols() + 1)..=sig.f_count() {
        if !sig.is_predicate(f_code) && !sig.is_special(f_code) && sig.find_arity(f_code) == Some(0)
        {
            result.push(f_code);
        }
    }
    result
}

fn intersect_identity_sets(alternatives: &mut TermIdentitySet, allowed: &TermIdentitySet) {
    alternatives.retain(|key, _| allowed.contains_key(key));
}

#[cfg(test)]
mod tests {
    use super::{
        clause_collect_var_constr, eqn_collect_var_constr, lit_occ_add_clause_alt,
        lit_occ_add_clause_slice_alt, lit_occ_add_lit_alt, sig_collect_constant_terms,
        term_identity_set_from_terms, LitOccTable, TermIdentitySet, VarConstraintMap,
    };
    use crate::clauses::clause::Clause;
    use crate::clauses::eqn::Eqn;
    use crate::clauses::eqnlist::EqnList;
    use crate::terms::functypes::FunCode;
    use crate::terms::signature::{FunctionProperties, Signature, FP_SPECIAL};
    use crate::terms::simpletypes::alloc_arrow_type;
    use crate::terms::termbanks::TermBank;
    use crate::terms::termtypes::{DerefType, Term};
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

    fn typed_var(bank: &TermBank, f_code: FunCode) -> Term {
        let type_ = bank.signature().type_bank().default_type();
        bank.vars().var_assert_alloc(f_code, &type_)
    }

    fn predicate_atom(bank: &mut TermBank, name: &str, args: &[Term]) -> Term {
        let bool_type = bank.signature().type_bank().bool_type();
        let f_code =
            bank.signature_mut()
                .insert_id(name, i32::try_from(args.len()).unwrap(), false);
        let mut type_args = Vec::with_capacity(args.len() + 1);
        for arg in args {
            type_args.push(arg.type_().expect("test argument must be typed"));
        }
        type_args.push(bool_type.clone());
        if bank.signature().get_type(f_code).is_none() {
            bank.signature_mut()
                .declare_type(f_code, alloc_arrow_type(type_args))
                .unwrap();
        }
        let term = Term::top_alloc(f_code, args.len());
        term.set_type(Some(bool_type.clone()));
        for (index, arg) in args.iter().enumerate() {
            term.set_argument(index, arg.clone());
        }
        let shared = bank.insert(&term, DerefType::Never).unwrap();
        shared.set_type(Some(bool_type));
        shared
    }

    fn predicate_literal(bank: &mut TermBank, atom: &Term, positive: bool) -> Eqn {
        Eqn::alloc(atom.clone(), bank.true_term().clone(), bank, positive).unwrap()
    }

    fn clause_from(literals: Vec<Eqn>) -> Clause {
        Clause::alloc(EqnList::from_vec(literals))
    }

    fn var_key(variable: &Term) -> i64 {
        -variable.f_code()
    }

    #[test]
    fn lit_occ_table_allocates_constrained_matrix_with_c_dimensions() {
        let mut bank = test_bank();
        let first = typed_const(&mut bank, "a");
        let second = typed_const(&mut bank, "b");
        let third = typed_const(&mut bank, "c");
        let atom = predicate_atom(&mut bank, "p", &[first, second, third]);
        let table = LitOccTable::alloc(bank.signature());

        assert_eq!(
            table.sig_size(),
            usize::try_from(bank.signature().f_count() + 1).unwrap()
        );
        assert_eq!(table.maxarity(), 3);
        assert!(table.constr_state(atom.f_code(), 0));
        assert!(table.constraints(atom.f_code(), 0).is_empty());
    }

    #[test]
    fn lit_pos_add_constraint_collects_terms_and_free_vars_unconstrain() {
        let mut bank = test_bank();
        let ground = typed_const(&mut bank, "a");
        let variable = typed_var(&bank, -2);
        let atom = predicate_atom(&mut bank, "p", std::slice::from_ref(&ground));
        let mut table = LitOccTable::alloc(bank.signature());

        assert!(!table.add_constraint(atom.f_code(), 0, &ground));
        assert_eq!(table.constraints(atom.f_code(), 0).len(), 1);
        assert!(!table.add_constraint(atom.f_code(), 0, &ground));
        assert_eq!(table.constraints(atom.f_code(), 0).len(), 1);
        assert!(table.add_constraint(atom.f_code(), 0, &variable));
        assert!(!table.constr_state(atom.f_code(), 0));
        assert_eq!(table.entry(atom.f_code(), 0).constraints().len(), 1);
    }

    #[test]
    fn literal_and_clause_addition_route_by_sign_and_argument_position() {
        let mut bank = test_bank();
        let ground = typed_const(&mut bank, "a");
        let variable = typed_var(&bank, -2);
        let positive_atom = predicate_atom(&mut bank, "p", &[ground.clone(), variable.clone()]);
        let negative_atom = predicate_atom(&mut bank, "p", &[variable, ground.clone()]);
        let mut positive_table = LitOccTable::alloc(bank.signature());
        let mut negative_table = LitOccTable::alloc(bank.signature());

        lit_occ_add_lit_alt(
            &mut positive_table,
            &mut negative_table,
            &predicate_literal(&mut bank, &positive_atom, true),
        );
        assert_eq!(
            positive_table.constraints(positive_atom.f_code(), 0).len(),
            1
        );
        assert!(!positive_table.constr_state(positive_atom.f_code(), 1));
        assert!(negative_table
            .constraints(positive_atom.f_code(), 0)
            .is_empty());

        let negative_clause =
            clause_from(vec![predicate_literal(&mut bank, &negative_atom, false)]);
        lit_occ_add_clause_alt(&mut positive_table, &mut negative_table, &negative_clause);
        assert!(!negative_table.constr_state(negative_atom.f_code(), 0));
        assert_eq!(
            negative_table.constraints(negative_atom.f_code(), 1).len(),
            1
        );

        lit_occ_add_clause_slice_alt(&mut positive_table, &mut negative_table, &[negative_clause]);
        assert_eq!(
            negative_table.constraints(negative_atom.f_code(), 1).len(),
            1
        );
    }

    #[test]
    fn collect_constant_terms_scans_functions_skips_predicates_and_adds_skolem_if_needed() {
        let mut bank = test_bank();
        let first = typed_const(&mut bank, "a");
        let second = typed_const(&mut bank, "b");
        let special_code = bank.signature_mut().insert_id("special", 0, false);
        let default_type = bank.signature().type_bank().default_type();
        bank.signature_mut()
            .declare_final_type(special_code, default_type)
            .unwrap();
        bank.signature_mut()
            .set_func_prop(special_code, FP_SPECIAL | FunctionProperties::TYPE_FIXED);
        let _predicate = predicate_atom(&mut bank, "p", &[]);
        let mut stack = Vec::new();

        assert_eq!(
            sig_collect_constant_terms(&mut bank, &mut stack, None).unwrap(),
            2
        );
        assert_eq!(stack, vec![first.clone(), second]);

        stack.clear();
        assert_eq!(
            sig_collect_constant_terms(&mut bank, &mut stack, Some(first.f_code())).unwrap(),
            1
        );
        assert_eq!(stack, vec![first]);

        let mut empty_bank = test_bank();
        let mut skolem_stack = Vec::new();
        assert_eq!(
            sig_collect_constant_terms(&mut empty_bank, &mut skolem_stack, None).unwrap(),
            1
        );
        assert_eq!(skolem_stack.len(), 1);
        assert_eq!(skolem_stack[0].arity(), 0);
    }

    #[test]
    fn variable_constraints_intersect_with_opposite_sign_literal_positions() {
        let mut bank = test_bank();
        let first = typed_const(&mut bank, "a");
        let second = typed_const(&mut bank, "b");
        let third = typed_const(&mut bank, "c");
        let x = typed_var(&bank, -2);
        let y = typed_var(&bank, -4);
        let negative_atom = predicate_atom(&mut bank, "p", &[first.clone(), second.clone()]);
        let query_atom = predicate_atom(&mut bank, "p", &[x.clone(), y.clone()]);
        let mut positive_table = LitOccTable::alloc(bank.signature());
        let mut negative_table = LitOccTable::alloc(bank.signature());
        lit_occ_add_lit_alt(
            &mut positive_table,
            &mut negative_table,
            &predicate_literal(&mut bank, &negative_atom, false),
        );
        let all_terms =
            term_identity_set_from_terms(&[first.clone(), second.clone(), third.clone()]);
        let mut var_constr = VarConstraintMap::new();
        var_constr.insert(var_key(&x), all_terms.clone());
        var_constr.insert(var_key(&y), all_terms);

        eqn_collect_var_constr(
            &positive_table,
            &negative_table,
            &mut var_constr,
            &predicate_literal(&mut bank, &query_atom, true),
        );

        assert_eq!(
            var_constr[&var_key(&x)],
            term_identity_set_from_terms(std::slice::from_ref(&first))
        );
        assert_eq!(
            var_constr[&var_key(&y)],
            term_identity_set_from_terms(std::slice::from_ref(&second))
        );
    }

    #[test]
    fn clause_constraint_collection_preserves_c_unused_ground_terms_argument() {
        let mut bank = test_bank();
        let first = typed_const(&mut bank, "a");
        let x = typed_var(&bank, -2);
        let negative_atom = predicate_atom(&mut bank, "p", std::slice::from_ref(&first));
        let query_atom = predicate_atom(&mut bank, "p", std::slice::from_ref(&x));
        let mut positive_table = LitOccTable::alloc(bank.signature());
        let mut negative_table = LitOccTable::alloc(bank.signature());
        lit_occ_add_lit_alt(
            &mut positive_table,
            &mut negative_table,
            &predicate_literal(&mut bank, &negative_atom, false),
        );
        let ground_terms = term_identity_set_from_terms(std::slice::from_ref(&first));
        let mut var_constr = VarConstraintMap::new();
        var_constr.insert(var_key(&x), ground_terms.clone());

        clause_collect_var_constr(
            &positive_table,
            &negative_table,
            &clause_from(vec![predicate_literal(&mut bank, &query_atom, true)]),
            &TermIdentitySet::new(),
            &mut var_constr,
        );

        assert_eq!(var_constr[&var_key(&x)], ground_terms);
    }
}
