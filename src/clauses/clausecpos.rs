use crate::clauses::clause::Clause;
use crate::clauses::clausepos::ClausePos;
use crate::clauses::eqn::Eqn;
use crate::clauses::eqn_props::EqnSide;
use crate::terms::termfunc::term_standard_weight;
use crate::terms::termpos::TermPos;
use crate::terms::termtypes::{Term, DEFAULT_FWEIGHT};

pub type CompactPos = i64;

/// Packs a full term position into E's weighted compact-position encoding.
///
/// # Panics
///
/// Panics if a stored superterm is a free variable or lambda, if a component
/// index is outside the superterm arity, or if a traversed argument slot is
/// uninitialized, matching the C assertions.
#[must_use]
pub fn pack_term_pos(pos: &TermPos) -> CompactPos {
    let mut result = 0;
    for (term, index) in pos.components() {
        assert!(
            !term.is_free_var(),
            "free variables cannot have subpositions"
        );
        assert!(!term.is_lambda(), "lambda term positions are not packed");
        assert!(
            index < term.arity(),
            "term-position index must select an existing argument"
        );

        if !term.is_phony_app() && !term.is_lambda() {
            result += DEFAULT_FWEIGHT;
        }
        for arg_index in 0..index {
            let arg = term
                .argument(arg_index)
                .expect("packed term position requires initialized arguments");
            result += term_standard_weight(&arg);
        }
    }
    result
}

/// Packs a clause position into E's weighted compact-position encoding.
///
/// # Panics
///
/// Panics if `pos` is not backed by a clause and current literal index, or if
/// the index no longer selects a literal in that clause.
#[must_use]
pub fn pack_clause_pos<T>(pos: &ClausePos<T>) -> CompactPos {
    let clause = pos
        .clause()
        .expect("compact clause position packing requires a clause");
    let literal_index = pos
        .literal_index()
        .expect("compact clause position packing requires a literal index");
    assert!(
        literal_index < clause.literals().len(),
        "literal index must select a clause literal"
    );

    let mut result = clause
        .literals()
        .as_slice()
        .iter()
        .take(literal_index)
        .map(Eqn::standard_weight)
        .sum::<i64>();
    let literal = &clause.literals().as_slice()[literal_index];
    if pos.side() == EqnSide::RightSide {
        result += term_standard_weight(literal.left());
    }
    result + pack_term_pos(pos.term_pos())
}

/// Unpacks a compact term position into an existing full position.
///
/// # Panics
///
/// Panics if the compact position descends below a lambda, if traversal reaches
/// a free variable, if the compact value is outside the term, or if a traversed
/// argument slot is uninitialized.
pub fn unpack_term_pos(pos: &mut TermPos, term: &Term, mut cpos: CompactPos) {
    assert!(
        !term.is_lambda() || cpos == 0,
        "lambda terms only support their top compact position"
    );
    pos.clear();

    let mut current = term.clone();
    while cpos > 0 {
        assert!(
            !current.is_free_var(),
            "free variables cannot have subpositions"
        );
        if !current.is_phony_app() && !current.is_lambda() {
            cpos -= DEFAULT_FWEIGHT;
        }
        assert!(cpos >= 0, "compact position underflows the current term");

        let mut selected = None;
        for index in 0..current.arity() {
            let arg = current
                .argument(index)
                .expect("unpacked term position requires initialized arguments");
            let weight = term_standard_weight(&arg);
            if cpos < weight {
                selected = Some((index, arg));
                break;
            }
            cpos -= weight;
            assert!(cpos >= 0, "compact position underflows term arguments");
        }

        let (index, next) = selected.expect("compact position must select an argument");
        pos.push_component(current, index);
        current = next;
    }
}

/// Unpacks a compact clause position into an existing clause-position cursor.
///
/// # Panics
///
/// Panics if the clause is empty, if the compact position does not select a
/// literal/side/subterm in the clause, or under the same conditions as
/// [`unpack_term_pos`].
pub fn unpack_clause_pos_into<T>(mut cpos: CompactPos, clause: Clause, pos: &mut ClausePos<T>) {
    let (literal_index, _) =
        clause_cpos_split(&clause, &mut cpos).expect("compact clause position needs a literal");
    pos.set_clause(Some(clause));
    assert!(pos.set_literal_index(Some(literal_index)));

    let (left_weight, left, right) = {
        let literal = pos
            .literal()
            .expect("literal index must select an unpacked clause literal");
        (
            term_standard_weight(literal.left()),
            literal.left().clone(),
            literal.right().clone(),
        )
    };
    let side_term = if cpos >= left_weight {
        cpos -= left_weight;
        pos.set_side(EqnSide::RightSide);
        right
    } else {
        pos.set_side(EqnSide::LeftSide);
        left
    };
    unpack_term_pos(pos.term_pos_mut(), &side_term, cpos);
}

/// Unpacks a compact clause position into a new clause-position cursor.
///
/// # Panics
///
/// Panics under the same conditions as [`unpack_clause_pos_into`].
#[must_use]
pub fn unpack_clause_pos(cpos: CompactPos, clause: Clause) -> ClausePos<()> {
    let mut pos = ClausePos::new();
    unpack_clause_pos_into(cpos, clause, &mut pos);
    pos
}

/// Returns the subterm selected by a compact clause position.
///
/// # Panics
///
/// Panics under the same conditions as [`unpack_clause_pos`].
#[must_use]
pub fn clause_cpos_get_subterm(clause: &Clause, cpos: CompactPos) -> Term {
    unpack_clause_pos(cpos, clause.clone())
        .get_subterm()
        .expect("unpacked clause position must select a subterm")
}

pub fn clause_cpos_first_lit<'clause>(
    clause: &'clause Clause,
    cpos: &mut CompactPos,
) -> Option<(usize, &'clause Eqn)> {
    *cpos = 0;
    clause
        .literals()
        .as_slice()
        .first()
        .map(|literal| (0, literal))
}

pub fn clause_cpos_next_lit<'clause>(
    clause: &'clause Clause,
    literal_index: usize,
    cpos: &mut CompactPos,
) -> Option<(usize, &'clause Eqn)> {
    let literal = clause.literals().as_slice().get(literal_index)?;
    let next_index = literal_index.saturating_add(1);
    if let Some(next) = clause.literals().as_slice().get(next_index) {
        *cpos += literal.standard_weight();
        Some((next_index, next))
    } else {
        *cpos = 0;
        None
    }
}

/// Splits a compact clause position into a literal and relative position.
///
/// # Panics
///
/// Panics if `*cpos` is at or beyond the end of a non-empty clause, matching
/// the C assertion after advancing past the last literal.
pub fn clause_cpos_split<'clause>(
    clause: &'clause Clause,
    cpos: &mut CompactPos,
) -> Option<(usize, &'clause Eqn)> {
    let literals = clause.literals().as_slice();
    let mut index = 0;
    while let Some(literal) = literals.get(index) {
        if literal.standard_weight() > *cpos {
            return Some((index, literal));
        }
        *cpos -= literal.standard_weight();
        index += 1;
        assert!(
            index < literals.len(),
            "compact clause position must point inside a clause literal"
        );
    }
    None
}

#[cfg(test)]
mod tests {
    use super::{
        clause_cpos_first_lit, clause_cpos_get_subterm, clause_cpos_next_lit, clause_cpos_split,
        pack_clause_pos, pack_term_pos, unpack_clause_pos, unpack_term_pos, CompactPos,
    };
    use crate::clauses::clause::Clause;
    use crate::clauses::clausepos::ClausePos;
    use crate::clauses::eqn::Eqn;
    use crate::clauses::eqn_props::EqnSide;
    use crate::clauses::eqnlist::EqnList;
    use crate::terms::signature::Signature;
    use crate::terms::simpletypes::alloc_arrow_type;
    use crate::terms::termbanks::TermBank;
    use crate::terms::termfunc::term_standard_weight;
    use crate::terms::termpos::TermPos;
    use crate::terms::termtypes::{Term, DEFAULT_FWEIGHT};
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

    fn typed_unary(bank: &mut TermBank, name: &str, arg: &Term) -> Term {
        let type_ = bank.signature().type_bank().default_type();
        let f_code = bank.signature_mut().insert_id(name, 1, false);
        bank.signature_mut()
            .declare_final_type(f_code, alloc_arrow_type(vec![type_.clone(), type_]))
            .unwrap();
        let term = Term::top_alloc(f_code, 1);
        term.set_type(arg.type_());
        term.set_argument(0, arg.clone());
        bank.term_top_insert(term).unwrap()
    }

    fn typed_binary(bank: &mut TermBank, name: &str, left: &Term, right: &Term) -> Term {
        let type_ = bank.signature().type_bank().default_type();
        let f_code = bank.signature_mut().insert_id(name, 2, false);
        bank.signature_mut()
            .declare_final_type(
                f_code,
                alloc_arrow_type(vec![type_.clone(), type_.clone(), type_]),
            )
            .unwrap();
        let term = Term::top_alloc(f_code, 2);
        term.set_type(left.type_());
        term.set_argument(0, left.clone());
        term.set_argument(1, right.clone());
        bank.term_top_insert(term).unwrap()
    }

    fn eqn(bank: &mut TermBank, left: &Term, right: &Term, positive: bool) -> Eqn {
        Eqn::alloc(left.clone(), right.clone(), bank, positive).unwrap()
    }

    #[test]
    fn term_position_packing_uses_c_weight_offsets() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let g_of_b = typed_unary(&mut bank, "g", &b);
        let f = typed_binary(&mut bank, "f", &a, &g_of_b);
        let mut pos = TermPos::new();
        pos.push_component(f.clone(), 1);
        pos.push_component(g_of_b.clone(), 0);

        let packed = pack_term_pos(&pos);
        assert_eq!(packed, 2 * DEFAULT_FWEIGHT + term_standard_weight(&a));

        let mut unpacked = TermPos::new();
        unpack_term_pos(&mut unpacked, &f, packed);
        assert_eq!(unpacked.print_string(), "1.0\n");
        assert_eq!(unpacked.get_subterm(&f), b);
        assert_eq!(pack_term_pos(&TermPos::new()), 0);
    }

    #[test]
    fn clause_position_pack_unpack_round_trips_literal_side_and_subterm() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let c = typed_const(&mut bank, "c");
        let g_of_b = typed_unary(&mut bank, "g", &b);
        let left = typed_binary(&mut bank, "f", &a, &g_of_b);
        let first = eqn(&mut bank, &left, &c, true);
        let second = eqn(&mut bank, &b, &c, false);
        let clause = Clause::alloc(EqnList::from_vec(vec![first.clone(), second]));

        let mut pos = ClausePos::<()>::for_clause(clause.clone());
        pos.term_pos_mut().push_component(left.clone(), 1);
        pos.term_pos_mut().push_component(g_of_b.clone(), 0);
        let packed_left = pack_clause_pos(&pos);
        assert_eq!(clause_cpos_get_subterm(&clause, packed_left), b);

        let unpacked = unpack_clause_pos(packed_left, clause.clone());
        assert_eq!(unpacked.literal_index(), Some(0));
        assert_eq!(unpacked.side(), EqnSide::LeftSide);
        assert_eq!(unpacked.term_pos().print_string(), "1.0\n");

        pos.set_side(EqnSide::RightSide);
        pos.term_pos_mut().clear();
        let packed_right = pack_clause_pos(&pos);
        assert_eq!(packed_right, term_standard_weight(&left));
        let unpacked_right = unpack_clause_pos(packed_right, clause);
        assert_eq!(unpacked_right.side(), EqnSide::RightSide);
        assert_eq!(unpacked_right.get_subterm(), Some(c.clone()));
        assert_eq!(
            first.standard_weight(),
            term_standard_weight(&left) + term_standard_weight(&c)
        );
    }

    #[test]
    fn literal_iteration_and_split_update_compact_positions_like_c() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let c = typed_const(&mut bank, "c");
        let first = eqn(&mut bank, &a, &b, true);
        let second = eqn(&mut bank, &b, &c, false);
        let clause = Clause::alloc(EqnList::from_vec(vec![first.clone(), second.clone()]));

        let mut cpos: CompactPos = 99;
        let (index, lit) = clause_cpos_first_lit(&clause, &mut cpos).unwrap();
        assert_eq!(index, 0);
        assert_eq!(lit, &first);
        assert_eq!(cpos, 0);

        let (index, lit) = clause_cpos_next_lit(&clause, index, &mut cpos).unwrap();
        assert_eq!(index, 1);
        assert_eq!(lit, &second);
        assert_eq!(cpos, first.standard_weight());
        assert!(clause_cpos_next_lit(&clause, index, &mut cpos).is_none());
        assert_eq!(cpos, 0);

        let mut split_pos = first.standard_weight() + term_standard_weight(second.left());
        let (index, lit) = clause_cpos_split(&clause, &mut split_pos).unwrap();
        assert_eq!(index, 1);
        assert_eq!(lit, &second);
        assert_eq!(split_pos, term_standard_weight(second.left()));
    }

    #[test]
    fn empty_clause_first_literal_and_split_match_null_shape() {
        let empty = Clause::empty();
        let mut cpos = 5;
        assert!(clause_cpos_first_lit(&empty, &mut cpos).is_none());
        assert_eq!(cpos, 0);
        cpos = 5;
        assert!(clause_cpos_split(&empty, &mut cpos).is_none());
        assert_eq!(cpos, 5);
    }
}
