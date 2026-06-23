use crate::basics::error::Diagnostic;
use crate::clauses::clause::Clause;
use crate::clauses::eqn::Eqn;
use crate::clauses::eqn_props::EP_IS_EQU_LITERAL;
use crate::clauses::groundconstr::{
    clause_collect_var_constr, LitOccTable, TermIdentitySet, VarConstraintMap,
};
use crate::terms::termbanks::TermBank;
use crate::terms::termtypes::{DerefType, Term};
use std::collections::BTreeMap;
use std::fmt::Write as _;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i32)]
pub enum GroundSetState {
    Complete = 0,
    LowMemory = 1,
    Timeout = 2,
    Unknown = 3,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i32)]
pub enum GcuEncoding {
    None = 0,
    Pos = 1,
    Neg = 2,
    Both = 3,
}

pub const DEFAULT_LIT_NO: usize = 4096;
pub const DEFAULT_LIT_GROW: usize = 8192;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VarInst {
    variable: Term,
    alternatives: Option<Vec<Term>>,
    position: Option<usize>,
}

impl VarInst {
    #[must_use]
    pub fn new(variable: Term) -> Self {
        Self {
            variable,
            alternatives: None,
            position: None,
        }
    }

    #[must_use]
    pub const fn variable(&self) -> &Term {
        &self.variable
    }

    #[must_use]
    pub fn alternatives(&self) -> Option<&[Term]> {
        self.alternatives.as_deref()
    }

    pub fn set_alternatives(&mut self, alternatives: Vec<Term>) {
        self.alternatives = Some(alternatives);
        self.position = None;
    }

    #[must_use]
    pub const fn position(&self) -> Option<usize> {
        self.position
    }

    #[must_use]
    pub fn current_alternative(&self) -> Option<&Term> {
        let alternatives = self.alternatives.as_ref()?;
        let position = self.position?;
        alternatives.get(position)
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct VarSetInst {
    cells: Vec<VarInst>,
}

impl VarSetInst {
    #[must_use]
    pub fn alloc(clause: &Clause) -> Self {
        let mut variables = BTreeMap::new();
        let _ = clause.collect_variables(&mut variables);
        Self {
            cells: variables.into_values().map(VarInst::new).collect(),
        }
    }

    #[must_use]
    pub fn constrained_alloc(
        positive_table: &LitOccTable,
        negative_table: &LitOccTable,
        clause: &Clause,
        ground_terms: &TermIdentitySet,
    ) -> Self {
        let mut handle = Self::alloc(clause);
        let mut var_constr = VarConstraintMap::new();
        for cell in &handle.cells {
            var_constr.insert(
                variable_constraint_key(cell.variable()),
                ground_terms.clone(),
            );
        }
        clause_collect_var_constr(
            positive_table,
            negative_table,
            clause,
            ground_terms,
            &mut var_constr,
        );
        for cell in &mut handle.cells {
            let alternatives = var_constr
                .remove(&variable_constraint_key(cell.variable()))
                .unwrap_or_default()
                .into_values()
                .collect();
            cell.set_alternatives(alternatives);
        }
        handle
    }

    #[must_use]
    pub fn cells(&self) -> &[VarInst] {
        &self.cells
    }

    #[must_use]
    pub fn cells_mut(&mut self) -> &mut [VarInst] {
        &mut self.cells
    }

    #[must_use]
    pub const fn len(&self) -> usize {
        self.cells.len()
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }

    pub fn set_all_alternatives(&mut self, alternatives: &[Term]) {
        for cell in &mut self.cells {
            cell.set_alternatives(alternatives.to_vec());
        }
    }

    /// Initializes the current substitution to the last alternative per variable.
    ///
    /// # Panics
    ///
    /// Panics if any variable cell has no alternatives stack, matching the C
    /// assertion in `varsetinstinitialize`.
    pub fn initialize(&mut self) -> bool {
        for cell in &mut self.cells {
            let alternatives = cell
                .alternatives
                .as_ref()
                .expect("variable instantiation requires alternatives");
            let Some(last) = alternatives.len().checked_sub(1) else {
                return false;
            };
            cell.position = Some(last);
        }
        true
    }

    /// Applies the current substitution by binding each variable to its alternative.
    ///
    /// # Panics
    ///
    /// Panics if the instance has not been initialized, if a current alternative
    /// is missing, if an alternative is a free variable, or if a cell variable is
    /// not a free variable.
    pub fn apply(&self) {
        for cell in &self.cells {
            let alternative = cell
                .current_alternative()
                .expect("variable instantiation must be initialized");
            assert!(
                !alternative.is_free_var(),
                "ground instantiation alternatives must not be free variables"
            );
            assert!(
                cell.variable.is_free_var(),
                "variable instantiation cells must hold free variables"
            );
            cell.variable.set_binding(Some(alternative.clone()));
        }
    }

    pub fn clear(&self) {
        for cell in &self.cells {
            cell.variable.set_binding(None);
        }
    }

    /// Advances to the next C-order substitution.
    ///
    /// # Panics
    ///
    /// Panics if the instance has not been initialized or if an alternatives
    /// stack is empty.
    pub fn advance(&mut self) -> bool {
        for cell in &mut self.cells {
            let position = cell
                .position
                .as_mut()
                .expect("variable instantiation must be initialized");
            if *position != 0 {
                *position -= 1;
                return true;
            }
            let alternatives = cell
                .alternatives
                .as_ref()
                .expect("variable instantiation requires alternatives");
            *position = alternatives
                .len()
                .checked_sub(1)
                .expect("variable alternatives must not be empty");
        }
        false
    }

    /// Estimates the number of substitutions represented by the alternatives.
    ///
    /// # Panics
    ///
    /// Panics if any cell has no alternatives stack, matching the C assertion
    /// in `varinstestimate`.
    #[allow(clippy::cast_precision_loss)]
    #[must_use]
    pub fn estimate(&self) -> f64 {
        self.cells.iter().fold(1.0, |estimate, cell| {
            let alternatives = cell
                .alternatives
                .as_ref()
                .expect("variable instantiation requires alternatives");
            estimate * alternatives.len() as f64
        })
    }
}

#[must_use]
pub fn clause_cmp_by_len(left: &Clause, right: &Clause) -> i32 {
    let literal_cmp = usize_diff_as_i32(left.literal_number(), right.literal_number());
    if literal_cmp != 0 {
        return literal_cmp;
    }
    usize_diff_as_i32(
        left.positive_literal_count(),
        right.positive_literal_count(),
    )
}

/// Recode an equational literal as a non-equational `$eq(left,right)=true` literal.
///
/// # Errors
///
/// Returns a diagnostic if the term-bank insertion or replacement literal
/// allocation fails.
///
/// # Panics
///
/// Panics if equality-code allocation fails or if the encoded equality term
/// cannot be represented in the term bank, matching the C invariant that the
/// equality symbol is available.
pub fn eqn_eqlit_recode(literal: &mut Eqn, bank: &mut TermBank) -> Result<bool, Diagnostic> {
    if !literal.is_equ_lit(bank) {
        return Ok(false);
    }

    let eqn_code = bank.signature_mut().get_eqn_code(true);
    assert_ne!(eqn_code, 0, "equality code allocation must succeed");
    let encoded = Term::top_alloc(eqn_code, 2);
    encoded.set_type(Some(bank.signature().type_bank().bool_type()));
    encoded.set_argument(0, literal.left().clone());
    encoded.set_argument(1, literal.right().clone());
    let encoded = bank.insert(&encoded, DerefType::Never)?;
    let true_term = bank.true_term().clone();
    let properties = literal.properties() & !EP_IS_EQU_LITERAL;
    let mut replacement = Eqn::alloc(encoded, true_term, bank, literal.is_positive())?;
    replacement.set_properties(properties);
    *literal = replacement;
    Ok(true)
}

/// Recode all equational literals in a clause.
///
/// # Errors
///
/// Returns a diagnostic if any literal recoding fails.
pub fn clause_eqlit_recode(clause: &mut Clause, bank: &mut TermBank) -> Result<bool, Diagnostic> {
    let mut recoded = false;
    for literal in clause.literals_mut().as_mut_slice() {
        recoded |= eqn_eqlit_recode(literal, bank)?;
    }
    Ok(recoded)
}

#[must_use]
pub fn print_dimacs_header_string(max_lit: i64, members: i64) -> String {
    let max_lit = if max_lit <= 0 { 1 } else { max_lit };
    format!("p cnf {max_lit} {members}\n")
}

#[must_use]
pub fn clause_print_dimacs_string(clause: &Clause) -> String {
    if clause.is_empty() {
        return " -1 0\n  1 0\n".to_owned();
    }

    let mut result = String::new();
    for literal in clause.literals().as_slice() {
        if literal.is_positive() {
            let _ = write!(&mut result, "  {}", literal.left().entry_no());
        } else {
            let _ = write!(&mut result, " -{}", literal.left().entry_no());
        }
    }
    result.push_str(" 0\n");
    result
}

#[must_use]
pub fn clause_get_max_lit(clause: &Clause) -> i64 {
    clause
        .literals()
        .as_slice()
        .iter()
        .map(|literal| literal.left().entry_no())
        .max()
        .unwrap_or(0)
}

fn variable_constraint_key(variable: &Term) -> i64 {
    -variable.f_code()
}

fn usize_diff_as_i32(left: usize, right: usize) -> i32 {
    let left = i64::try_from(left).unwrap_or(i64::MAX);
    let right = i64::try_from(right).unwrap_or(i64::MAX);
    let diff = left - right;
    i32::try_from(diff).unwrap_or_else(|_| {
        if diff.is_negative() {
            i32::MIN
        } else {
            i32::MAX
        }
    })
}

#[cfg(test)]
mod tests {
    use super::{
        clause_cmp_by_len, clause_eqlit_recode, clause_get_max_lit, clause_print_dimacs_string,
        eqn_eqlit_recode, print_dimacs_header_string, GcuEncoding, GroundSetState, VarSetInst,
        DEFAULT_LIT_GROW, DEFAULT_LIT_NO,
    };
    use crate::clauses::clause::Clause;
    use crate::clauses::eqn::Eqn;
    use crate::clauses::eqn_props::{EP_IS_EQU_LITERAL, EP_IS_SELECTED};
    use crate::clauses::eqnlist::EqnList;
    use crate::clauses::groundconstr::{
        lit_occ_add_lit_alt, term_identity_set_from_terms, LitOccTable,
    };
    use crate::terms::signature::Signature;
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

    fn predicate_atom(bank: &mut TermBank, name: &str, args: &[Term]) -> Term {
        let bool_type = bank.signature().type_bank().bool_type();
        let f_code =
            bank.signature_mut()
                .insert_id(name, i32::try_from(args.len()).unwrap(), false);
        if bank.signature().get_type(f_code).is_none() {
            let mut type_args = Vec::with_capacity(args.len() + 1);
            for arg in args {
                type_args.push(arg.type_().expect("test argument must be typed"));
            }
            type_args.push(bool_type.clone());
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

    fn literal(bank: &mut TermBank, left: &Term, right: &Term, positive: bool) -> Eqn {
        Eqn::alloc(left.clone(), right.clone(), bank, positive).unwrap()
    }

    fn clause_from(literals: Vec<Eqn>) -> Clause {
        let mut clause = Clause::alloc(EqnList::from_vec(literals));
        clause.set_weight(clause.standard_weight());
        clause
    }

    #[test]
    fn constants_and_discriminants_match_c_header() {
        assert_eq!(GroundSetState::Complete as i32, 0);
        assert_eq!(GroundSetState::LowMemory as i32, 1);
        assert_eq!(GroundSetState::Timeout as i32, 2);
        assert_eq!(GroundSetState::Unknown as i32, 3);
        assert_eq!(GcuEncoding::None as i32, 0);
        assert_eq!(GcuEncoding::Pos as i32, 1);
        assert_eq!(GcuEncoding::Neg as i32, 2);
        assert_eq!(GcuEncoding::Both as i32, 3);
        assert_eq!(DEFAULT_LIT_NO, 4096);
        assert_eq!(DEFAULT_LIT_GROW, 8192);
    }

    #[test]
    fn clause_compare_by_length_then_positive_count_matches_implementation() {
        let mut bank = test_bank();
        let first = typed_const(&mut bank, "a");
        let second = typed_const(&mut bank, "b");
        let unit = clause_from(vec![literal(&mut bank, &first, &second, true)]);
        let longer = clause_from(vec![
            literal(&mut bank, &first, &second, true),
            literal(&mut bank, &second, &first, false),
        ]);
        let negative_unit = clause_from(vec![literal(&mut bank, &first, &second, false)]);

        assert!(clause_cmp_by_len(&unit, &longer) < 0);
        assert!(clause_cmp_by_len(&unit, &negative_unit) > 0);
        assert_eq!(clause_cmp_by_len(&unit, &unit), 0);
    }

    #[test]
    fn var_set_inst_alloc_collects_clause_variables_without_alternatives() {
        let mut bank = test_bank();
        let first = typed_const(&mut bank, "a");
        let x = typed_var(&bank, -2);
        let y = typed_var(&bank, -4);
        let clause = clause_from(vec![
            literal(&mut bank, &x, &first, true),
            literal(&mut bank, &y, &first, false),
        ]);

        let inst = VarSetInst::alloc(&clause);

        assert_eq!(inst.len(), 2);
        assert!(inst
            .cells()
            .iter()
            .all(|cell| cell.variable().is_free_var()));
        assert!(inst
            .cells()
            .iter()
            .all(|cell| cell.alternatives().is_none()));
    }

    #[test]
    fn var_set_inst_initialize_apply_next_and_clear_follow_c_order() {
        let mut bank = test_bank();
        let first = typed_const(&mut bank, "a");
        let second = typed_const(&mut bank, "b");
        let x = typed_var(&bank, -2);
        let y = typed_var(&bank, -4);
        let mut inst = VarSetInst::alloc(&clause_from(vec![literal(&mut bank, &x, &y, true)]));
        inst.set_all_alternatives(&[first.clone(), second.clone()]);
        let first_variable = inst.cells()[0].variable().clone();
        let second_variable = inst.cells()[1].variable().clone();

        assert!(inst.initialize());
        assert_eq!(inst.cells()[0].position(), Some(1));
        assert_eq!(inst.cells()[1].position(), Some(1));
        inst.apply();
        assert_eq!(x.binding(), Some(second.clone()));
        assert_eq!(y.binding(), Some(second.clone()));

        assert!(inst.advance());
        inst.apply();
        assert_eq!(first_variable.binding(), Some(first));
        assert_eq!(second_variable.binding(), Some(second));
        inst.clear();
        assert_eq!(x.binding(), None);
        assert_eq!(y.binding(), None);
    }

    #[test]
    fn var_set_inst_empty_alternatives_and_estimate_match_c_shape() {
        let mut bank = test_bank();
        let x = typed_var(&bank, -2);
        let y = typed_var(&bank, -4);
        let first = typed_const(&mut bank, "a");
        let second = typed_const(&mut bank, "b");
        let third = typed_const(&mut bank, "c");
        let mut inst = VarSetInst::alloc(&clause_from(vec![literal(&mut bank, &x, &y, true)]));

        inst.cells_mut()[0].set_alternatives(Vec::new());
        inst.cells_mut()[1].set_alternatives(vec![first.clone()]);
        assert!(!inst.initialize());

        inst.cells_mut()[0].set_alternatives(vec![first, second]);
        inst.cells_mut()[1].set_alternatives(vec![third]);
        assert!((inst.estimate() - 2.0).abs() < f64::EPSILON);
    }

    #[test]
    fn constrained_var_set_inst_uses_grounding_constraints() {
        let mut bank = test_bank();
        let first = typed_const(&mut bank, "a");
        let second = typed_const(&mut bank, "b");
        let third = typed_const(&mut bank, "c");
        let x = typed_var(&bank, -2);
        let y = typed_var(&bank, -4);
        let negative_atom = predicate_atom(&mut bank, "p", &[first.clone(), second.clone()]);
        let query_atom = predicate_atom(&mut bank, "p", &[x.clone(), y.clone()]);
        let query_clause = clause_from(vec![predicate_literal(&mut bank, &query_atom, true)]);
        let mut positive_table = LitOccTable::alloc(bank.signature());
        let mut negative_table = LitOccTable::alloc(bank.signature());
        lit_occ_add_lit_alt(
            &mut positive_table,
            &mut negative_table,
            &predicate_literal(&mut bank, &negative_atom, false),
        );
        let ground_terms = term_identity_set_from_terms(&[first.clone(), second.clone(), third]);

        let inst = VarSetInst::constrained_alloc(
            &positive_table,
            &negative_table,
            &query_clause,
            &ground_terms,
        );

        assert_eq!(inst.len(), 2);
        let x_cell = inst
            .cells()
            .iter()
            .find(|cell| cell.variable().f_code() == x.f_code())
            .unwrap();
        let y_cell = inst
            .cells()
            .iter()
            .find(|cell| cell.variable().f_code() == y.f_code())
            .unwrap();
        assert_eq!(
            term_identity_set_from_terms(x_cell.alternatives().unwrap()),
            term_identity_set_from_terms(std::slice::from_ref(&first))
        );
        assert_eq!(
            term_identity_set_from_terms(y_cell.alternatives().unwrap()),
            term_identity_set_from_terms(std::slice::from_ref(&second))
        );
    }

    #[test]
    fn equality_literal_recode_wraps_terms_in_positive_equality_predicate() {
        let mut bank = test_bank();
        let first = typed_const(&mut bank, "a");
        let second = typed_const(&mut bank, "b");
        let mut eq = literal(&mut bank, &first, &second, true);
        eq.set_prop(EP_IS_SELECTED);
        assert!(eq.is_equ_lit(&bank));

        assert!(eqn_eqlit_recode(&mut eq, &mut bank).unwrap());

        assert!(!eq.is_equ_lit(&bank));
        assert!(eq.is_positive());
        assert!(eq.query_prop(EP_IS_SELECTED));
        assert!(!eq.query_prop(EP_IS_EQU_LITERAL));
        assert_eq!(eq.right(), bank.true_term());
        assert_eq!(eq.left().f_code(), bank.signature().eqn_code());
        assert_eq!(eq.left().argument(0).unwrap(), first);
        assert_eq!(eq.left().argument(1).unwrap(), second);
    }

    #[test]
    fn clause_recode_reports_whether_any_literal_changed() {
        let mut bank = test_bank();
        let first = typed_const(&mut bank, "a");
        let second = typed_const(&mut bank, "b");
        let true_lit = Eqn::create_true_lit(&mut bank).unwrap();
        let mut clause = clause_from(vec![literal(&mut bank, &first, &second, true), true_lit]);

        assert!(clause_eqlit_recode(&mut clause, &mut bank).unwrap());
        assert!(clause
            .literals()
            .as_slice()
            .iter()
            .all(|literal| !literal.is_equ_lit(&bank)));
        assert!(!clause_eqlit_recode(&mut clause, &mut bank).unwrap());
    }

    #[test]
    fn dimacs_helpers_match_c_spacing_and_empty_clause_workaround() {
        let mut bank = test_bank();
        let first = typed_const(&mut bank, "a");
        let second = typed_const(&mut bank, "b");
        let mut positive = literal(&mut bank, &first, &second, true);
        let mut negative = literal(&mut bank, &second, &first, false);
        eqn_eqlit_recode(&mut positive, &mut bank).unwrap();
        eqn_eqlit_recode(&mut negative, &mut bank).unwrap();
        let clause = clause_from(vec![positive, negative]);
        let pos_entry = clause.literals().as_slice()[0].left().entry_no();
        let neg_entry = clause.literals().as_slice()[1].left().entry_no();

        assert_eq!(print_dimacs_header_string(0, 5), "p cnf 1 5\n");
        assert_eq!(
            clause_print_dimacs_string(&clause),
            format!("  {pos_entry} -{neg_entry} 0\n")
        );
        assert_eq!(
            clause_print_dimacs_string(&Clause::empty()),
            " -1 0\n  1 0\n"
        );
        assert_eq!(clause_get_max_lit(&clause), pos_entry.max(neg_entry));
    }
}
