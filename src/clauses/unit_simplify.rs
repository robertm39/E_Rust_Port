use crate::basics::simple_stuff::string_index_c;
use crate::clauses::clause::Clause;
use crate::clauses::clause_props::{CP_INITIAL, CP_IS_PROTECTED, CP_IS_SOS, CP_LIMITED_RW};
use crate::clauses::clausefunc::clause_remove_literal_index;
use crate::clauses::clausesets::ClauseSet;
use crate::clauses::eqn::Eqn;
use crate::clauses::eqn_props::EqnSide;
use crate::clauses::pdtrees::{PdtIndexedOccurrence, PDTREE_IGNORE_NF_DATE};
use crate::clauses::subsumption::eqn_topsubsumes_termpair;
use crate::terms::match_mgu::subst_match_complete;
use crate::terms::subst::Substitution;
use crate::terms::termtypes::Term;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i32)]
pub enum UnitSimplifyType {
    NoUnitSimplify = 0,
    TopLevelUnitSimplify = 1,
    FullUnitSimplify = 2,
}

pub const UNIT_SIMPLIFY_NAMES: [Option<&str>; 4] = [
    Some("NoSimplify"),
    Some("TopSimplify"),
    Some("FullSimplify"),
    None,
];

#[derive(Clone, Copy, Debug)]
pub struct SimplifyingUnit<'set> {
    clause: &'set Clause,
    literal_index: usize,
}

impl<'set> SimplifyingUnit<'set> {
    #[must_use]
    pub const fn clause(self) -> &'set Clause {
        self.clause
    }

    #[must_use]
    pub const fn literal_index(self) -> usize {
        self.literal_index
    }

    #[must_use]
    pub fn literal(self) -> &'set Eqn {
        &self.clause.literals().as_slice()[self.literal_index]
    }
}

#[must_use]
pub fn trans_unit_simplify_string(name: &str) -> Option<UnitSimplifyType> {
    match string_index_c(name, &UNIT_SIMPLIFY_NAMES) {
        0 => Some(UnitSimplifyType::NoUnitSimplify),
        1 => Some(UnitSimplifyType::TopLevelUnitSimplify),
        2 => Some(UnitSimplifyType::FullUnitSimplify),
        _ => None,
    }
}

#[must_use]
pub fn find_top_simplifying_unit<'set>(
    units: &'set ClauseSet,
    left: &Term,
    right: &Term,
) -> Option<SimplifyingUnit<'set>> {
    find_top_simplifying_unit_with_sign(units, left, right, None)
}

#[must_use]
pub fn find_signed_top_simplifying_unit<'set>(
    units: &'set ClauseSet,
    left: &Term,
    right: &Term,
    sign: bool,
) -> Option<SimplifyingUnit<'set>> {
    find_top_simplifying_unit_with_sign(units, left, right, Some(sign))
}

#[must_use]
/// # Panics
///
/// Panics if matching nonvariable terms report an arity but do not expose
/// initialized arguments. This is an internal term-bank invariant.
pub fn find_simplifying_unit<'set>(
    units: &'set ClauseSet,
    left: &Term,
    right: &Term,
    positive_only: bool,
) -> Option<SimplifyingUnit<'set>> {
    if positive_only {
        if let Some(result) = find_signed_top_simplifying_unit(units, left, right, true) {
            return Some(result);
        }
    } else if let Some(result) = find_top_simplifying_unit(units, left, right) {
        return Some(result);
    }

    let mut current_left = left.clone();
    let mut current_right = right.clone();
    while !current_left.is_top_level_free_var()
        && !current_right.is_top_level_free_var()
        && !current_left.is_lambda()
        && !current_right.is_lambda()
        && current_left.f_code() == current_right.f_code()
        && current_left.arity() != 0
    {
        debug_assert_ne!(current_left, current_right);
        let mut differing_pair = None;
        for index in 0..current_left.arity() {
            let next_left = current_left
                .argument(index)
                .expect("left term arguments must be initialized");
            let next_right = current_right
                .argument(index)
                .expect("right term arguments must be initialized");
            if next_left != next_right {
                if differing_pair.is_some() {
                    return None;
                }
                differing_pair = Some((next_left, next_right));
            }
        }

        let (next_left, next_right) = differing_pair?;
        current_left = next_left;
        current_right = next_right;
        if let Some(result) =
            find_signed_top_simplifying_unit(units, &current_left, &current_right, true)
        {
            return Some(result);
        }
    }
    None
}

/// Simplifies `clause` with a unit set, matching C
/// `ClauseSimplifyWithUnitSet` aside from full `PDTreeFindNextDemodulator`
/// traversal over live clause positions.
///
/// Returns `false` when a same-signed unit subsumes the clause, otherwise
/// returns `true` after applying all opposite-signed unit cuts.
///
/// # Panics
///
/// Panics for [`UnitSimplifyType::NoUnitSimplify`], matching the C assertion
/// that the caller selects either top-level or full unit simplification.
#[must_use]
pub fn clause_simplify_with_unit_set(
    clause: &mut Clause,
    unit_set: &mut ClauseSet,
    how: UnitSimplifyType,
) -> bool {
    assert_ne!(
        how,
        UnitSimplifyType::NoUnitSimplify,
        "unit simplification mode must not be NoUnitSimplify"
    );

    let mut index = 0;
    while index < clause.literal_number() {
        let (left, right, sign) = {
            let literal = &clause.literals().as_slice()[index];
            (
                literal.left().clone(),
                literal.right().clone(),
                literal.is_positive(),
            )
        };
        let simplifier_index = match how {
            UnitSimplifyType::NoUnitSimplify => unreachable!(),
            UnitSimplifyType::TopLevelUnitSimplify => {
                find_top_simplifying_unit_index(unit_set, &left, &right, None)
            }
            UnitSimplifyType::FullUnitSimplify => {
                find_simplifying_unit_index(unit_set, &left, &right, false)
            }
        };

        let Some(simplifier_index) = simplifier_index else {
            index += 1;
            continue;
        };
        let simplifier_sign = unit_set
            .iter()
            .nth(simplifier_index)
            .and_then(unit_literal)
            .expect("simplifying unit index must select a unit literal")
            .is_positive();

        if sign == simplifier_sign {
            let protect_unit = !clause.is_unit()
                && clause.standard_weight()
                    == unit_set
                        .iter()
                        .nth(simplifier_index)
                        .expect("simplifying unit index must select a clause")
                        .standard_weight();
            let c_sos_as_property = if clause.query_prop(CP_IS_SOS) {
                CP_INITIAL
            } else {
                crate::clauses::clause_props::CP_IGNORE_PROPS
            };
            let simplifier = unit_set
                .iter_mut()
                .nth(simplifier_index)
                .expect("simplifying unit index must select a mutable clause");
            if protect_unit {
                simplifier.set_prop(CP_IS_PROTECTED);
            }
            simplifier.set_prop(c_sos_as_property);
            return false;
        }

        clause.del_prop(CP_LIMITED_RW);
        let removed = clause_remove_literal_index(clause, index);
        debug_assert!(removed.is_some(), "current literal must be removable");
    }
    true
}

fn find_top_simplifying_unit_with_sign<'set>(
    units: &'set ClauseSet,
    left: &Term,
    right: &Term,
    sign: Option<bool>,
) -> Option<SimplifyingUnit<'set>> {
    units.record_demod_index_search_init(left, PDTREE_IGNORE_NF_DATE, false);
    let result = if units.demod_index_search_may_have_match() {
        let candidate_sides = units.demod_index_search_candidate_sides();
        if let Some(candidate_sides) = candidate_sides.as_deref() {
            find_indexed_top_simplifying_unit(units, candidate_sides, left, right, sign)
        } else {
            find_plain_top_simplifying_unit(units, left, right, sign)
        }
    } else {
        None
    };
    units.record_demod_index_search_exit();
    result
}

fn find_indexed_top_simplifying_unit<'set>(
    units: &'set ClauseSet,
    candidate_sides: &[PdtIndexedOccurrence],
    left: &Term,
    right: &Term,
    sign: Option<bool>,
) -> Option<SimplifyingUnit<'set>> {
    candidate_sides.iter().find_map(|&candidate| {
        let clause = units.find_by_id(candidate.clause_id)?;
        let literal = unit_literal(clause)?;
        if sign.is_some_and(|required| literal.is_positive() != required) {
            return None;
        }
        unit_literal_occurrence_matches_top_pair(candidate, literal, left, right).then_some(
            SimplifyingUnit {
                clause,
                literal_index: 0,
            },
        )
    })
}

fn find_plain_top_simplifying_unit<'set>(
    units: &'set ClauseSet,
    left: &Term,
    right: &Term,
    sign: Option<bool>,
) -> Option<SimplifyingUnit<'set>> {
    units.iter().find_map(|clause| {
        let literal = unit_literal(clause)?;
        if sign.is_some_and(|required| literal.is_positive() != required) {
            return None;
        }
        eqn_topsubsumes_termpair(literal, left, right).then_some(SimplifyingUnit {
            clause,
            literal_index: 0,
        })
    })
}

fn unit_literal_occurrence_matches_top_pair(
    occurrence: PdtIndexedOccurrence,
    literal: &Eqn,
    left: &Term,
    right: &Term,
) -> bool {
    match occurrence.side {
        EqnSide::NoSide => false,
        EqnSide::LeftSide => {
            unit_literal_side_matches_top_pair(literal.left(), literal.right(), left, right)
        }
        EqnSide::RightSide => {
            unit_literal_side_matches_top_pair(literal.right(), literal.left(), left, right)
        }
        EqnSide::BothSides => {
            unit_literal_side_matches_top_pair(literal.left(), literal.right(), left, right)
                || unit_literal_side_matches_top_pair(literal.right(), literal.left(), left, right)
        }
    }
}

fn unit_literal_side_matches_top_pair(
    indexed_side: &Term,
    other_side: &Term,
    left: &Term,
    right: &Term,
) -> bool {
    let mut subst = Substitution::new();
    let result = subst_match_complete(indexed_side, left, &mut subst)
        && subst_match_complete(other_side, right, &mut subst);
    subst.backtrack();
    result
}

fn find_indexed_top_simplifying_unit_index(
    units: &ClauseSet,
    candidate_sides: &[PdtIndexedOccurrence],
    left: &Term,
    right: &Term,
    sign: Option<bool>,
) -> Option<usize> {
    candidate_sides.iter().find_map(|&candidate| {
        let (index, clause) = units
            .iter()
            .enumerate()
            .find(|(_, clause)| clause.ident() == candidate.clause_id)?;
        let literal = unit_literal(clause)?;
        if sign.is_some_and(|required| literal.is_positive() != required) {
            return None;
        }
        unit_literal_occurrence_matches_top_pair(candidate, literal, left, right).then_some(index)
    })
}

fn find_plain_top_simplifying_unit_index(
    units: &ClauseSet,
    left: &Term,
    right: &Term,
    sign: Option<bool>,
) -> Option<usize> {
    units.iter().enumerate().find_map(|(index, candidate)| {
        let literal = unit_literal(candidate)?;
        if sign.is_some_and(|required| literal.is_positive() != required) {
            return None;
        }
        eqn_topsubsumes_termpair(literal, left, right).then_some(index)
    })
}

fn find_top_simplifying_unit_index(
    units: &ClauseSet,
    left: &Term,
    right: &Term,
    sign: Option<bool>,
) -> Option<usize> {
    units.record_demod_index_search_init(left, PDTREE_IGNORE_NF_DATE, false);
    let result = if units.demod_index_search_may_have_match() {
        let candidate_sides = units.demod_index_search_candidate_sides();
        if let Some(candidate_sides) = candidate_sides.as_deref() {
            find_indexed_top_simplifying_unit_index(units, candidate_sides, left, right, sign)
        } else {
            find_plain_top_simplifying_unit_index(units, left, right, sign)
        }
    } else {
        None
    };
    units.record_demod_index_search_exit();
    result
}

fn find_simplifying_unit_index(
    units: &ClauseSet,
    left: &Term,
    right: &Term,
    positive_only: bool,
) -> Option<usize> {
    if positive_only {
        if let Some(result) = find_top_simplifying_unit_index(units, left, right, Some(true)) {
            return Some(result);
        }
    } else if let Some(result) = find_top_simplifying_unit_index(units, left, right, None) {
        return Some(result);
    }

    let mut current_left = left.clone();
    let mut current_right = right.clone();
    while !current_left.is_top_level_free_var()
        && !current_right.is_top_level_free_var()
        && !current_left.is_lambda()
        && !current_right.is_lambda()
        && current_left.f_code() == current_right.f_code()
        && current_left.arity() != 0
    {
        debug_assert_ne!(current_left, current_right);
        let mut differing_pair = None;
        for index in 0..current_left.arity() {
            let next_left = current_left
                .argument(index)
                .expect("left term arguments must be initialized");
            let next_right = current_right
                .argument(index)
                .expect("right term arguments must be initialized");
            if next_left != next_right {
                if differing_pair.is_some() {
                    return None;
                }
                differing_pair = Some((next_left, next_right));
            }
        }

        let (next_left, next_right) = differing_pair?;
        current_left = next_left;
        current_right = next_right;
        if let Some(result) =
            find_top_simplifying_unit_index(units, &current_left, &current_right, Some(true))
        {
            return Some(result);
        }
    }
    None
}

fn unit_literal(clause: &Clause) -> Option<&Eqn> {
    clause
        .is_unit()
        .then(|| clause.literals().as_slice().first())
        .flatten()
}

#[cfg(test)]
mod tests {
    use super::{
        clause_simplify_with_unit_set, find_signed_top_simplifying_unit, find_simplifying_unit,
        find_top_simplifying_unit, trans_unit_simplify_string, UnitSimplifyType,
    };
    use crate::clauses::clause::Clause;
    use crate::clauses::clause_props::{CP_INITIAL, CP_IS_PROTECTED, CP_IS_SOS, CP_LIMITED_RW};
    use crate::clauses::clausesets::ClauseSet;
    use crate::clauses::eqn::Eqn;
    use crate::clauses::eqn_props::EP_IS_ORIENTED;
    use crate::clauses::eqnlist::EqnList;
    use crate::clauses::pdtrees::{
        prefix_compute_term_code, PdtTraversalOrder, PDTREE_IGNORE_NF_DATE,
    };
    use crate::terms::signature::Signature;
    use crate::terms::simpletypes::alloc_arrow_type;
    use crate::terms::termbanks::TermBank;
    use crate::terms::termfunc::term_standard_weight;
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

    fn typed_unary(bank: &mut TermBank, name: &str, arg: &Term) -> Term {
        let type_ = bank.signature().type_bank().default_type();
        let f_code = bank.signature_mut().insert_id(name, 1, false);
        if bank.signature().get_type(f_code).is_none() {
            bank.signature_mut()
                .declare_final_type(f_code, alloc_arrow_type(vec![type_.clone(), type_.clone()]))
                .unwrap();
        }
        let term = Term::top_alloc(f_code, 1);
        term.set_type(Some(type_));
        term.set_argument(0, arg.clone());
        bank.insert(&term, DerefType::Never).unwrap()
    }

    fn typed_var(bank: &TermBank, code: i64) -> Term {
        let type_ = bank.signature().type_bank().default_type();
        bank.vars().var_assert_alloc(code, &type_)
    }

    fn literal(bank: &mut TermBank, left: &Term, right: &Term, positive: bool) -> Eqn {
        Eqn::alloc(left.clone(), right.clone(), bank, positive).unwrap()
    }

    fn clause_from(literals: Vec<Eqn>) -> Clause {
        Clause::alloc(EqnList::from_vec(literals))
    }

    #[test]
    fn unit_simplify_names_match_c_table() {
        assert_eq!(
            trans_unit_simplify_string("NoSimplify"),
            Some(UnitSimplifyType::NoUnitSimplify)
        );
        assert_eq!(
            trans_unit_simplify_string("TopSimplify"),
            Some(UnitSimplifyType::TopLevelUnitSimplify)
        );
        assert_eq!(
            trans_unit_simplify_string("FullSimplify"),
            Some(UnitSimplifyType::FullUnitSimplify)
        );
        assert_eq!(trans_unit_simplify_string("missing"), None);
    }

    #[test]
    fn find_simplifying_unit_descends_only_after_top_lookup_fails() {
        let mut bank = test_bank();
        let variable = typed_var(&bank, -10);
        let a = typed_const(&mut bank, "unit_a");
        let b = typed_const(&mut bank, "unit_b");
        let left = typed_unary(&mut bank, "unit_f", &b);
        let right = typed_unary(&mut bank, "unit_f", &a);
        let positive_unit = clause_from(vec![literal(&mut bank, &variable, &a, true)]);
        let set = ClauseSet::from_clauses([positive_unit]);

        assert!(find_top_simplifying_unit(&set, &left, &right).is_none());
        assert_eq!(
            find_simplifying_unit(&set, &left, &right, false)
                .map(|unit| unit.literal().is_positive()),
            Some(true)
        );
        assert_eq!(
            find_signed_top_simplifying_unit(&set, &b, &a, false).map(|unit| unit.clause().ident()),
            None
        );
    }

    #[test]
    fn indexed_top_lookup_records_demodulator_search_attempts() {
        let mut bank = test_bank();
        let variable = typed_var(&bank, -10);
        let a = typed_const(&mut bank, "unit_count_a");
        let b = typed_const(&mut bank, "unit_count_b");
        let mut positive_unit = clause_from(vec![literal(&mut bank, &variable, &a, true)]);
        let mut set = ClauseSet::new_demod_indexed();

        positive_unit.set_weight(positive_unit.standard_weight());
        set.indexed_insert_clause_owned(positive_unit, &bank);

        assert_eq!(set.demod_index_match_count(), 0);
        assert!(find_top_simplifying_unit(&set, &b, &a).is_some());
        assert_eq!(set.demod_index_match_count(), 1);
        assert_eq!(
            set.demod_index_traversal_order(),
            Some(PdtTraversalOrder::variables_first())
        );
        assert!(!set.demod_index_search_active());
        let state = set
            .demod_index_search_state()
            .expect("unit lookup records search state");
        assert_eq!(state.term_code, prefix_compute_term_code(&b));
        assert_eq!(state.term_weight, term_standard_weight(&b));
        assert_eq!(state.term_date, PDTREE_IGNORE_NF_DATE);
        assert_eq!(state.traversal_order, PdtTraversalOrder::variables_first());
        assert!(find_signed_top_simplifying_unit(&set, &b, &a, false).is_none());
        assert_eq!(set.demod_index_match_count(), 2);
        assert_eq!(
            set.demod_index_traversal_order(),
            Some(PdtTraversalOrder::variables_first())
        );
        assert!(!set.demod_index_search_active());
    }

    #[test]
    fn indexed_top_lookup_uses_pdt_candidate_side_direction() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "unit_side_a");
        let b = typed_const(&mut bank, "unit_side_b");
        let mut unit_lit = literal(&mut bank, &a, &b, true);
        unit_lit.set_prop(EP_IS_ORIENTED);
        let mut unit = clause_from(vec![unit_lit]);
        unit.set_weight(unit.standard_weight());
        let mut indexed = ClauseSet::new_demod_indexed();

        indexed.indexed_insert_clause_owned(unit.clone(), &bank);

        assert!(find_top_simplifying_unit(&indexed, &a, &b).is_some());
        assert!(find_top_simplifying_unit(&indexed, &b, &a).is_none());

        let plain = ClauseSet::from_clauses([unit]);
        assert!(find_top_simplifying_unit(&plain, &b, &a).is_some());
    }

    #[test]
    fn indexed_top_lookup_uses_pdt_candidate_order_before_set_order() {
        let mut bank = test_bank();
        let variable = typed_var(&bank, -11);
        let a = typed_const(&mut bank, "unit_order_a");
        let rhs = typed_const(&mut bank, "unit_order_rhs");
        let mut specific = clause_from(vec![literal(&mut bank, &a, &rhs, true)]);
        let specific_id = specific.ident();
        let mut general = clause_from(vec![literal(&mut bank, &variable, &rhs, true)]);
        let general_id = general.ident();
        let mut indexed = ClauseSet::new_demod_indexed();

        specific.set_weight(specific.standard_weight());
        general.set_weight(general.standard_weight());
        indexed.indexed_insert_clause_owned(specific.clone(), &bank);
        indexed.indexed_insert_clause_owned(general.clone(), &bank);

        assert_eq!(
            find_top_simplifying_unit(&indexed, &a, &rhs).map(|unit| unit.clause().ident()),
            Some(general_id)
        );

        let plain = ClauseSet::from_clauses([specific, general]);
        assert_eq!(
            find_top_simplifying_unit(&plain, &a, &rhs).map(|unit| unit.clause().ident()),
            Some(specific_id)
        );
    }

    #[test]
    fn clause_simplify_with_unit_set_removes_opposite_signed_literals() {
        let mut bank = test_bank();
        let variable = typed_var(&bank, -10);
        let a = typed_const(&mut bank, "cut_a");
        let b = typed_const(&mut bank, "cut_b");
        let c = typed_const(&mut bank, "cut_c");
        let left = typed_unary(&mut bank, "cut_f", &b);
        let right = typed_unary(&mut bank, "cut_f", &a);
        let positive_unit = clause_from(vec![literal(&mut bank, &variable, &a, true)]);
        let mut unit_set = ClauseSet::from_clauses([positive_unit]);
        let mut target = clause_from(vec![
            literal(&mut bank, &left, &right, false),
            literal(&mut bank, &c, &b, true),
        ]);
        target.set_weight(target.standard_weight());
        target.set_prop(CP_INITIAL | CP_LIMITED_RW);

        assert!(clause_simplify_with_unit_set(
            &mut target,
            &mut unit_set,
            UnitSimplifyType::FullUnitSimplify
        ));

        assert_eq!(target.literal_number(), 1);
        assert!(target.literals().as_slice()[0].is_positive());
        assert!(target.query_prop(CP_INITIAL));
        assert!(!target.query_prop(CP_LIMITED_RW));
        assert_eq!(target.weight(), target.standard_weight());
    }

    #[test]
    fn top_level_unit_simplify_does_not_descend() {
        let mut bank = test_bank();
        let variable = typed_var(&bank, -10);
        let a = typed_const(&mut bank, "top_a");
        let b = typed_const(&mut bank, "top_b");
        let left = typed_unary(&mut bank, "top_f", &b);
        let right = typed_unary(&mut bank, "top_f", &a);
        let positive_unit = clause_from(vec![literal(&mut bank, &variable, &a, true)]);
        let mut unit_set = ClauseSet::from_clauses([positive_unit]);
        let mut target = clause_from(vec![literal(&mut bank, &left, &right, false)]);

        assert!(clause_simplify_with_unit_set(
            &mut target,
            &mut unit_set,
            UnitSimplifyType::TopLevelUnitSimplify
        ));

        assert_eq!(target.literal_number(), 1);
        assert!(target.literals().as_slice()[0].is_negative());
    }

    #[test]
    fn same_signed_unit_subsumes_and_preserves_c_sos_property_bug() {
        let mut bank = test_bank();
        let variable = typed_var(&bank, -10);
        let a = typed_const(&mut bank, "same_a");
        let b = typed_const(&mut bank, "same_b");
        let positive_unit = clause_from(vec![literal(&mut bank, &variable, &a, true)]);
        let unit_id = positive_unit.ident();
        let mut unit_set = ClauseSet::from_clauses([positive_unit]);
        let mut target = clause_from(vec![literal(&mut bank, &b, &a, true)]);
        target.set_prop(CP_IS_SOS);

        assert!(!clause_simplify_with_unit_set(
            &mut target,
            &mut unit_set,
            UnitSimplifyType::TopLevelUnitSimplify
        ));

        let unit = unit_set.find_by_id(unit_id).unwrap();
        assert!(unit.query_prop(CP_INITIAL));
        assert!(!unit.query_prop(CP_IS_SOS));
        assert!(!unit.query_prop(CP_IS_PROTECTED));
        assert_eq!(target.literal_number(), 1);
    }
}
