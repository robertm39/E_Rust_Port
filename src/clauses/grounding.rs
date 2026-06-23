use crate::basics::error::Diagnostic;
use crate::clauses::clause::Clause;
use crate::clauses::eqn::Eqn;
use crate::clauses::eqn_props::EP_IS_EQU_LITERAL;
use crate::terms::termbanks::TermBank;
use crate::terms::termtypes::{DerefType, Term};
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
        clause_cmp_by_len, clause_eqlit_recode, clause_print_dimacs_string, eqn_eqlit_recode,
        print_dimacs_header_string, GcuEncoding, GroundSetState, DEFAULT_LIT_GROW, DEFAULT_LIT_NO,
    };
    use crate::clauses::clause::Clause;
    use crate::clauses::eqn::Eqn;
    use crate::clauses::eqn_props::{EP_IS_EQU_LITERAL, EP_IS_SELECTED};
    use crate::clauses::eqnlist::EqnList;
    use crate::terms::signature::Signature;
    use crate::terms::termbanks::TermBank;
    use crate::terms::termtypes::Term;
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
    }
}
