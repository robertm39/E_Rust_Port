use crate::basics::error::{Diagnostic, ErrorCode};
use crate::clauses::clause::Clause;
use crate::clauses::clause_props::{CP_IS_SOS, CP_TYPE_MASK};
use crate::clauses::eqn::Eqn;
use crate::clauses::eqn_props::EP_IS_SPLIT_LIT;
use crate::clauses::eqnlist::EqnList;
use crate::terms::signature::FP_CL_SPLIT_DEF;
use crate::terms::termbanks::TermBank;
use crate::terms::termtypes::{DerefType, Term};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClauseSplitType {
    GroundNone,
    GroundOne,
    GroundFull,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ClauseSplitOutcome {
    Unsplit(Box<Clause>),
    Split(Vec<Clause>),
}

impl ClauseSplitOutcome {
    #[must_use]
    pub fn split_count(&self) -> usize {
        match self {
            Self::Unsplit(_) => 0,
            Self::Split(clauses) => clauses.len(),
        }
    }
}

#[derive(Clone, Debug)]
struct LitSplitDesc {
    literal: Eqn,
    part: usize,
    varset: BTreeSet<usize>,
}

/// Returns whether the clause contains a C `EPIsSplitLit` marker.
///
/// C `ClauseHasSplitLiteral` checks the literal property, not the generated
/// predicate flag, even though other callers treat either marker as a split
/// literal.
#[must_use]
pub fn clause_has_split_literal(clause: &Clause) -> bool {
    clause
        .literals()
        .as_slice()
        .iter()
        .any(|literal| literal.query_prop(EP_IS_SPLIT_LIT))
}

/// Generates a C `GenDefLit`-style split definition literal.
///
/// This currently supports the arity-zero path used by ordinary
/// `ClauseSplit`. General splitting with explicit shared variables is handled
/// by a later slice.
///
/// # Errors
///
/// Returns a diagnostic if signature type declaration or term-bank insertion
/// fails.
pub fn gen_ground_def_lit(
    bank: &mut TermBank,
    pred: i64,
    positive: bool,
) -> Result<Eqn, Diagnostic> {
    if pred <= 0 {
        return Err(Diagnostic::new(
            ErrorCode::OTHER_ERROR,
            "split definition predicate must be positive",
        ));
    }

    let bool_type = bank.signature().type_bank().bool_type();
    bank.signature_mut().declare_type(pred, bool_type.clone())?;
    bank.signature_mut().set_func_prop(pred, FP_CL_SPLIT_DEF);

    let term = Term::const_cell_alloc(pred);
    term.set_type(Some(bool_type));
    let term = bank.insert(&term, DerefType::Never)?;
    let true_term = bank.true_term().clone();
    let mut literal = Eqn::alloc(term, true_term, bank, positive)?;
    literal.set_prop(EP_IS_SPLIT_LIT);
    Ok(literal)
}

/// Performs the fresh-definition C `ClauseSplit` path.
///
/// The returned `Split` vector contains the definition clauses followed by the
/// residual clause, matching the clauses inserted into the destination set by C.
/// Definition reuse (`fresh_defs == false`) and formula-archive side effects
/// are not represented here.
///
/// # Errors
///
/// Returns diagnostics from generated predicate typing, term-bank insertion, or
/// split-literal allocation.
pub fn clause_split_fresh(
    bank: &mut TermBank,
    mut clause: Clause,
    how: ClauseSplitType,
) -> Result<ClauseSplitOutcome, Diagnostic> {
    let lit_no = clause.literal_number();
    if lit_no <= 1 || clause_has_split_literal(&clause) {
        return Ok(ClauseSplitOutcome::Unsplit(Box::new(clause)));
    }

    let props = clause.give_props(CP_TYPE_MASK | CP_IS_SOS);
    let mut lit_table = initialize_lit_table(clause.literals().as_slice(), how);
    let mut part = 0;

    if how == ClauseSplitType::GroundOne && c_truthy_find_free_literal(&lit_table) {
        part += 1;
    }

    while let Some(index) = find_free_literal(&lit_table) {
        part += 1;
        build_part(&mut lit_table, index, part);
    }

    if part <= 1 {
        return Ok(ClauseSplitOutcome::Unsplit(Box::new(clause)));
    }

    let mut split_clauses = Vec::with_capacity(part + 1);
    let mut residual_literals = Vec::with_capacity(part);
    for part_index in 1..=part {
        let pred = bank.signature_mut().get_new_predicate_code(0);
        let mut clause_literals = Vec::with_capacity(lit_no + 1);
        clause_literals.push(gen_ground_def_lit(bank, pred, true)?);
        clause_literals.extend(assemble_part_literals(&lit_table, part_index));

        let mut new_clause = Clause::alloc(EqnList::from_vec(clause_literals));
        new_clause.set_properties(props);
        split_clauses.push(new_clause);

        residual_literals.push(gen_ground_def_lit(bank, pred, false)?);
    }

    residual_literals.reverse();
    clause.replace_literals(EqnList::from_vec(residual_literals));
    split_clauses.push(clause);
    Ok(ClauseSplitOutcome::Split(split_clauses))
}

fn initialize_lit_table(literals: &[Eqn], how: ClauseSplitType) -> Vec<LitSplitDesc> {
    literals
        .iter()
        .cloned()
        .map(|literal| {
            let varset = literal_varset(&literal);
            let part = usize::from(
                matches!(
                    how,
                    ClauseSplitType::GroundOne | ClauseSplitType::GroundNone
                ) && varset.is_empty(),
            );
            LitSplitDesc {
                literal,
                part,
                varset,
            }
        })
        .collect()
}

fn literal_varset(literal: &Eqn) -> BTreeSet<usize> {
    let mut variables = BTreeMap::new();
    let _ = literal.collect_variables(&mut variables);
    variables.keys().copied().collect()
}

fn find_free_literal(lit_table: &[LitSplitDesc]) -> Option<usize> {
    lit_table
        .iter()
        .enumerate()
        .find_map(|(index, desc)| (desc.part == 0).then_some(index))
}

fn c_truthy_find_free_literal(lit_table: &[LitSplitDesc]) -> bool {
    find_free_literal(lit_table) != Some(0)
}

fn build_part(lit_table: &mut [LitSplitDesc], lit_index: usize, part: usize) {
    lit_table[lit_index].part = part;
    let mut new_vars = true;
    while new_vars {
        new_vars = false;
        for index in lit_index + 1..lit_table.len() {
            if lit_table[index].part == 0
                && lit_table[lit_index]
                    .varset
                    .iter()
                    .any(|var| lit_table[index].varset.contains(var))
            {
                lit_table[index].part = part;
                let varset = std::mem::take(&mut lit_table[index].varset);
                for variable in varset {
                    new_vars |= lit_table[lit_index].varset.insert(variable);
                }
            }
        }
    }
}

fn assemble_part_literals(lit_table: &[LitSplitDesc], part: usize) -> Vec<Eqn> {
    let mut literals = Vec::new();
    for desc in lit_table {
        if desc.part == part {
            literals.push(desc.literal.clone());
        }
    }
    literals.reverse();
    literals
}

#[cfg(test)]
mod tests {
    use super::{
        clause_has_split_literal, clause_split_fresh, ClauseSplitOutcome, ClauseSplitType,
    };
    use crate::clauses::clause::Clause;
    use crate::clauses::clause_props::{CP_IS_SOS, CP_TYPE_AXIOM};
    use crate::clauses::eqn::Eqn;
    use crate::clauses::eqn_props::EP_IS_SPLIT_LIT;
    use crate::clauses::eqnlist::EqnList;
    use crate::terms::signature::{Signature, FP_CL_SPLIT_DEF};
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
                .declare_final_type(f_code, type_)
                .unwrap();
        }
        bank.create_const_term(f_code).unwrap()
    }

    fn typed_var(bank: &TermBank, f_code: i64) -> Term {
        let type_ = bank.signature().type_bank().default_type();
        bank.vars().var_assert_alloc(f_code, &type_)
    }

    fn lit(bank: &mut TermBank, left: &Term, right: &Term, positive: bool) -> Eqn {
        Eqn::alloc(left.clone(), right.clone(), bank, positive).unwrap()
    }

    fn split_literal_count(clause: &Clause, bank: &TermBank) -> usize {
        clause
            .literals()
            .as_slice()
            .iter()
            .filter(|literal| literal.query_prop(EP_IS_SPLIT_LIT) && literal.is_split_lit(bank))
            .count()
    }

    #[test]
    fn clause_split_fresh_splits_variable_disjoint_parts() {
        let mut bank = test_bank();
        let left_var = typed_var(&bank, -2);
        let right_var = typed_var(&bank, -4);
        let left_const = typed_const(&mut bank, "split_left");
        let right_const = typed_const(&mut bank, "split_right");
        let mut clause = Clause::alloc(EqnList::from_vec(vec![
            lit(&mut bank, &left_var, &left_const, true),
            lit(&mut bank, &right_var, &right_const, true),
        ]));
        clause.set_ident(7_001);
        clause.set_prop(CP_TYPE_AXIOM | CP_IS_SOS);

        let outcome = clause_split_fresh(&mut bank, clause, ClauseSplitType::GroundFull).unwrap();
        let ClauseSplitOutcome::Split(clauses) = outcome else {
            panic!("variable-disjoint clause should split");
        };

        assert_eq!(clauses.len(), 3);
        assert!(clauses[0].query_prop(CP_TYPE_AXIOM | CP_IS_SOS));
        assert!(clauses[1].query_prop(CP_TYPE_AXIOM | CP_IS_SOS));
        assert_eq!(split_literal_count(&clauses[0], &bank), 1);
        assert_eq!(split_literal_count(&clauses[1], &bank), 1);
        assert_eq!(clauses[2].ident(), 7_001);
        assert_eq!(clauses[2].literal_number(), 2);
        assert!(clauses[2]
            .literals()
            .as_slice()
            .iter()
            .all(Eqn::is_negative));
        assert_eq!(split_literal_count(&clauses[2], &bank), 2);
        for literal in clauses[2].literals().as_slice() {
            assert!(bank
                .signature()
                .query_prop(literal.pred_code_fo(&bank), FP_CL_SPLIT_DEF));
        }
    }

    #[test]
    fn clause_split_fresh_combines_ground_literals_for_ground_none() {
        let mut bank = test_bank();
        let variable = typed_var(&bank, -2);
        let first_const = typed_const(&mut bank, "split_ground_first");
        let second_const = typed_const(&mut bank, "split_ground_second");
        let third_const = typed_const(&mut bank, "split_ground_third");
        let clause = Clause::alloc(EqnList::from_vec(vec![
            lit(&mut bank, &first_const, &second_const, true),
            lit(&mut bank, &variable, &third_const, true),
        ]));

        let outcome = clause_split_fresh(&mut bank, clause, ClauseSplitType::GroundNone).unwrap();

        assert_eq!(outcome.split_count(), 0);
    }

    #[test]
    fn clause_split_fresh_splits_ground_literals_for_ground_full() {
        let mut bank = test_bank();
        let first_const = typed_const(&mut bank, "split_full_first");
        let second_const = typed_const(&mut bank, "split_full_second");
        let third_const = typed_const(&mut bank, "split_full_third");
        let fourth_const = typed_const(&mut bank, "split_full_fourth");
        let clause = Clause::alloc(EqnList::from_vec(vec![
            lit(&mut bank, &first_const, &second_const, true),
            lit(&mut bank, &third_const, &fourth_const, true),
        ]));

        let outcome = clause_split_fresh(&mut bank, clause, ClauseSplitType::GroundFull).unwrap();

        assert_eq!(outcome.split_count(), 3);
    }

    #[test]
    fn clause_split_fresh_refuses_existing_split_literal() {
        let mut bank = test_bank();
        let first_const = typed_const(&mut bank, "split_marked_first");
        let second_const = typed_const(&mut bank, "split_marked_second");
        let third_const = typed_const(&mut bank, "split_marked_third");
        let fourth_const = typed_const(&mut bank, "split_marked_fourth");
        let mut marked = lit(&mut bank, &first_const, &second_const, true);
        marked.set_prop(EP_IS_SPLIT_LIT);
        let clause = Clause::alloc(EqnList::from_vec(vec![
            marked,
            lit(&mut bank, &third_const, &fourth_const, true),
        ]));

        let outcome = clause_split_fresh(&mut bank, clause, ClauseSplitType::GroundFull).unwrap();

        assert_eq!(outcome.split_count(), 0);
        let ClauseSplitOutcome::Unsplit(clause) = outcome else {
            panic!("marked split literal should block resplitting");
        };
        assert!(clause_has_split_literal(&clause));
    }

    #[test]
    fn gen_ground_def_lit_inserts_shared_predicate_literal() {
        let mut bank = test_bank();
        let pred = bank.signature_mut().get_new_predicate_code(0);

        let literal = super::gen_ground_def_lit(&mut bank, pred, false).unwrap();

        assert!(literal.is_negative());
        assert!(literal.query_prop(EP_IS_SPLIT_LIT));
        assert!(literal.is_split_lit(&bank));
        assert_eq!(literal.right(), bank.true_term());
        assert!(bank.find(literal.left()).is_some());
    }

    #[test]
    fn split_definition_literal_reuses_existing_constant_cell() {
        let mut bank = test_bank();
        let pred = bank.signature_mut().get_new_predicate_code(0);
        let first = super::gen_ground_def_lit(&mut bank, pred, true).unwrap();
        let second_term = Term::const_cell_alloc(pred);
        second_term.set_type(Some(bank.signature().type_bank().bool_type()));
        let second_term = bank.insert(&second_term, DerefType::Never).unwrap();

        assert_eq!(first.left(), &second_term);
    }
}
