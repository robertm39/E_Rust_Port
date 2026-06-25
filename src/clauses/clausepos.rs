use crate::basics::pstacks::PStack;
use crate::clauses::clause::Clause;
use crate::clauses::eqn::Eqn;
use crate::clauses::eqn_props::EqnSide;
use crate::terms::termpos::TermPos;
use crate::terms::termtypes::{RewriteDemodulator, Term};
use std::fmt::{self, Write};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RewriteSequenceEntry {
    Operation(i64),
    Demodulator(RewriteDemodulator),
}

#[derive(Clone, Debug, PartialEq)]
pub struct ClausePos<T = ()> {
    clause: Option<Clause>,
    literal_index: Option<usize>,
    literal: Option<Eqn>,
    side: EqnSide,
    pos: TermPos,
    data: Option<T>,
}

impl<T> Default for ClausePos<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> ClausePos<T> {
    #[must_use]
    pub fn new() -> Self {
        Self {
            clause: None,
            literal_index: None,
            literal: None,
            side: EqnSide::LeftSide,
            pos: TermPos::new(),
            data: None,
        }
    }

    #[must_use]
    pub fn for_clause(clause: Clause) -> Self {
        let literal_index = (!clause.literals().is_empty()).then_some(0);
        Self {
            clause: Some(clause),
            literal_index,
            literal: None,
            side: EqnSide::LeftSide,
            pos: TermPos::new(),
            data: None,
        }
    }

    #[must_use]
    pub fn for_literal(literal: Eqn) -> Self {
        Self {
            clause: None,
            literal_index: None,
            literal: Some(literal),
            side: EqnSide::LeftSide,
            pos: TermPos::new(),
            data: None,
        }
    }

    #[must_use]
    pub const fn clause(&self) -> Option<&Clause> {
        self.clause.as_ref()
    }

    pub fn set_clause(&mut self, clause: Option<Clause>) {
        self.literal_index = clause
            .as_ref()
            .and_then(|clause| (!clause.literals().is_empty()).then_some(0));
        self.clause = clause;
        if self.clause.is_some() {
            self.literal = None;
        }
    }

    #[must_use]
    pub const fn literal_index(&self) -> Option<usize> {
        self.literal_index
    }

    pub fn set_literal_index(&mut self, literal_index: Option<usize>) -> bool {
        if let (Some(clause), Some(index)) = (&self.clause, literal_index) {
            if index >= clause.literals().len() {
                return false;
            }
        }
        self.literal_index = literal_index;
        if literal_index.is_some() {
            self.literal = None;
        }
        true
    }

    #[must_use]
    pub fn literal(&self) -> Option<&Eqn> {
        if let (Some(clause), Some(index)) = (&self.clause, self.literal_index) {
            clause.literals().as_slice().get(index)
        } else {
            self.literal.as_ref()
        }
    }

    pub fn set_literal(&mut self, literal: Option<Eqn>) {
        self.clause = None;
        self.literal_index = None;
        self.literal = literal;
    }

    #[must_use]
    pub const fn side(&self) -> EqnSide {
        self.side
    }

    pub const fn set_side(&mut self, side: EqnSide) {
        self.side = side;
    }

    #[must_use]
    pub const fn term_pos(&self) -> &TermPos {
        &self.pos
    }

    pub fn term_pos_mut(&mut self) -> &mut TermPos {
        &mut self.pos
    }

    #[must_use]
    pub const fn data(&self) -> Option<&T> {
        self.data.as_ref()
    }

    pub fn set_data(&mut self, data: Option<T>) {
        self.data = data;
    }

    #[must_use]
    pub fn is_top(&self) -> bool {
        self.pos.is_top_pos()
    }

    #[must_use]
    pub fn get_side(&self) -> Option<Term> {
        let literal = self.literal()?;
        if self.side == EqnSide::LeftSide {
            Some(literal.left().clone())
        } else {
            Some(literal.right().clone())
        }
    }

    #[must_use]
    pub fn get_other_side(&self) -> Option<Term> {
        let literal = self.literal()?;
        if self.side == EqnSide::LeftSide {
            Some(literal.right().clone())
        } else {
            Some(literal.left().clone())
        }
    }

    #[must_use]
    pub fn get_subterm(&self) -> Option<Term> {
        let side = self.get_side()?;
        Some(self.pos.get_subterm(&side))
    }

    /// Writes the C clause-position shape `<clause>.<literal>.<side>.<pos>`.
    ///
    /// # Panics
    ///
    /// Panics if the position is not backed by a clause and a current literal
    /// index. The C printer assumes both pointers are initialized.
    pub fn write_to(&self, output: &mut impl Write) -> fmt::Result {
        let clause = self
            .clause
            .as_ref()
            .expect("clause position printer requires a clause");
        let literal_index = self
            .literal_index
            .expect("clause position printer requires a literal index");
        assert!(
            literal_index < clause.literals().len(),
            "literal index must select a clause literal"
        );

        let side_char = if self.side == EqnSide::RightSide {
            'R'
        } else {
            'L'
        };
        write!(output, "{}.{}.{side_char}.", clause.ident(), literal_index)?;
        self.pos.write_to(output)
    }

    /// Returns the C clause-position rendering.
    ///
    /// # Panics
    ///
    /// Panics under the same conditions as [`Self::write_to`].
    #[must_use]
    pub fn print_string(&self) -> String {
        let mut output = String::new();
        let _ = self.write_to(&mut output);
        output
    }

    pub fn find_pos_literal(&mut self, maximal: bool) -> Option<&Eqn> {
        self.find_literal_from_current(|literal| {
            literal.is_positive() && (!maximal || literal.is_maximal())
        });
        self.literal()
    }

    pub fn find_max_literal(&mut self, positive: bool) -> Option<&Eqn> {
        self.find_literal_from_current(|literal| {
            literal.is_maximal() && (!positive || literal.is_positive())
        });
        self.literal()
    }

    pub fn find_first_maximal_side(&mut self, positive: bool) -> Option<Term> {
        self.find_max_literal(positive)?;
        self.side = EqnSide::LeftSide;
        self.pos.clear();
        self.get_side()
    }

    pub fn find_next_maximal_side(&mut self, positive: bool) -> Option<Term> {
        let use_right_side = self
            .literal()
            .is_some_and(|literal| self.side == EqnSide::LeftSide && !literal.is_oriented());
        self.pos.clear();
        if use_right_side {
            self.side = EqnSide::RightSide;
            return self.get_side();
        }

        self.advance_to_next_literal();
        self.find_max_literal(positive)?;
        self.side = EqnSide::LeftSide;
        self.get_side()
    }

    /// Finds the first leftmost-innermost subterm on a maximal literal side.
    ///
    /// # Panics
    ///
    /// Panics if a traversed term argument slot is uninitialized, matching the
    /// C term-position traversal invariant.
    pub fn find_first_maximal_subterm(&mut self) -> Option<Term> {
        let side = self.find_first_maximal_side(false)?;
        Some(self.pos.first_li_position(&side))
    }

    /// Finds the next leftmost-innermost subterm on maximal literal sides.
    ///
    /// # Panics
    ///
    /// Panics if a traversed term argument slot is uninitialized, matching the
    /// C term-position traversal invariant.
    pub fn find_next_maximal_subterm(&mut self) -> Option<Term> {
        self.get_side()?;
        if let Some(term) = self.pos.next_li_position() {
            return Some(term);
        }

        let side = self.find_next_maximal_side(false)?;
        Some(self.pos.first_li_position(&side))
    }

    fn find_literal_from_current<F>(&mut self, mut predicate: F)
    where
        F: FnMut(&Eqn) -> bool,
    {
        if let Some(clause) = &self.clause {
            let Some(start) = self.literal_index else {
                return;
            };
            self.literal_index = (start..clause.literals().len())
                .find(|&index| predicate(&clause.literals().as_slice()[index]));
            if self.literal_index.is_some() {
                self.literal = None;
            }
        } else if self
            .literal
            .as_ref()
            .is_none_or(|literal| !predicate(literal))
        {
            self.literal = None;
        }
    }

    fn advance_to_next_literal(&mut self) {
        if let Some(clause) = &self.clause {
            self.literal_index = self.literal_index.and_then(|index| {
                let next = index.saturating_add(1);
                (next < clause.literals().len()).then_some(next)
            });
        } else {
            self.literal = None;
        }
    }
}

/// Computes the rewrite-chain entries needed to transform `to` into `from`.
///
/// Returns `true` if at least one rewrite link was followed, preserving the C
/// implementation's behavior despite the opposite wording in its comment.
///
/// # Panics
///
/// Panics if a non-identical `from` term is not marked rewritten, if a rewrite
/// replacement is missing, or if a structural rewrite link connects terms with
/// incompatible root symbols or uninitialized argument slots.
pub fn term_compute_rw_sequence(
    stack: &mut PStack<RewriteSequenceEntry>,
    from: &Term,
    to: &Term,
    inject_op: i32,
) -> bool {
    let mut current = from.clone();
    let mut changed = false;

    while current != *to {
        assert!(
            current.is_rewritten(),
            "rewrite chain source must be rewritten"
        );
        let replacement = current
            .rw_replace_field()
            .expect("rewritten term must have a replacement");
        if let Some(demodulator) = current.rw_demod_field() {
            if inject_op != 0 {
                stack.push(RewriteSequenceEntry::Operation(i64::from(inject_op)));
            }
            stack.push(RewriteSequenceEntry::Demodulator(demodulator));
        } else {
            assert_eq!(
                current.f_code(),
                replacement.f_code(),
                "structural rewrite link must preserve the root symbol"
            );
            assert!(
                current.arity() != 0,
                "structural rewrite link must have arguments"
            );
            for index in 0..current.arity() {
                let from_arg = current
                    .argument(index)
                    .expect("rewrite source argument must be initialized");
                let to_arg = replacement
                    .argument(index)
                    .expect("rewrite replacement argument must be initialized");
                term_compute_rw_sequence(stack, &from_arg, &to_arg, inject_op);
            }
        }
        current = replacement;
        changed = true;
    }

    changed
}

#[cfg(test)]
mod tests {
    use super::{term_compute_rw_sequence, ClausePos, RewriteSequenceEntry};
    use crate::basics::pstacks::PStack;
    use crate::clauses::clause::Clause;
    use crate::clauses::eqn::Eqn;
    use crate::clauses::eqn_props::{EqnSide, EP_IS_MAXIMAL, EP_IS_ORIENTED};
    use crate::clauses::eqnlist::EqnList;
    use crate::terms::replace::{term_add_rw_link, RwResultType};
    use crate::terms::signature::Signature;
    use crate::terms::simpletypes::alloc_arrow_type;
    use crate::terms::termbanks::TermBank;
    use crate::terms::termtypes::{RewriteDemodulator, Term};
    use crate::terms::typebanks::TypeBank;

    fn test_bank() -> TermBank {
        let types = TypeBank::new();
        TermBank::new(Signature::new(types)).unwrap()
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
        typed_binary_with_code(bank, f_code, left, right)
    }

    fn typed_binary_with_code(bank: &mut TermBank, f_code: i64, left: &Term, right: &Term) -> Term {
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
    fn allocation_side_access_and_printing_match_clause_position_shape() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let f_of_a = typed_unary(&mut bank, "f", &a);
        let mut literal = eqn(&mut bank, &f_of_a, &b, true);
        literal.set_prop(EP_IS_MAXIMAL);
        let mut clause = Clause::alloc(EqnList::from_vec(vec![literal]));
        clause.set_ident(42);

        let mut pos = ClausePos::<usize>::for_clause(clause);
        pos.set_data(Some(7));
        assert_eq!(pos.data(), Some(&7));
        assert!(pos.is_top());
        assert_eq!(pos.get_side(), Some(f_of_a.clone()));
        assert_eq!(pos.get_other_side(), Some(b.clone()));

        pos.term_pos_mut().push_component(f_of_a.clone(), 0);
        assert_eq!(pos.get_subterm(), Some(a));
        assert_eq!(pos.print_string(), "42.0.L.0");

        pos.set_side(EqnSide::RightSide);
        pos.term_pos_mut().clear();
        assert_eq!(pos.get_side(), Some(b));
        assert_eq!(pos.print_string(), "42.0.R.");
    }

    #[test]
    fn literal_search_and_maximal_side_iteration_follow_c_cursor_rules() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let c = typed_const(&mut bank, "c");
        let d = typed_const(&mut bank, "d");
        let mut positive_max = eqn(&mut bank, &a, &b, true);
        positive_max.set_prop(EP_IS_MAXIMAL);
        let positive_plain = eqn(&mut bank, &b, &c, true);
        let mut negative_max = eqn(&mut bank, &c, &d, false);
        negative_max.set_prop(EP_IS_MAXIMAL | EP_IS_ORIENTED);
        let clause = Clause::alloc(EqnList::from_vec(vec![
            negative_max.clone(),
            positive_plain,
            positive_max.clone(),
        ]));

        let mut pos = ClausePos::<()>::for_clause(clause);
        assert_eq!(pos.find_pos_literal(true), Some(&positive_max));
        assert_eq!(pos.literal_index(), Some(1));
        assert_eq!(pos.find_first_maximal_side(false), Some(a.clone()));
        assert_eq!(pos.find_next_maximal_side(false), Some(b));
        assert_eq!(pos.find_next_maximal_side(false), Some(c));
        assert_eq!(pos.side(), EqnSide::LeftSide);
        assert_eq!(pos.find_next_maximal_side(false), None);

        let mut standalone = ClausePos::<()>::for_literal(negative_max);
        assert!(standalone.find_pos_literal(false).is_none());
    }

    #[test]
    fn maximal_subterm_iteration_uses_leftmost_innermost_term_positions() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let c = typed_const(&mut bank, "c");
        let g_of_b = typed_unary(&mut bank, "g", &b);
        let f = typed_binary(&mut bank, "f", &a, &g_of_b);
        let mut literal = eqn(&mut bank, &f, &c, true);
        literal.set_prop(EP_IS_MAXIMAL);
        let clause = Clause::alloc(EqnList::from_vec(vec![literal]));
        let mut pos = ClausePos::<()>::for_clause(clause);

        assert_eq!(pos.find_first_maximal_subterm(), Some(a));
        assert_eq!(pos.term_pos().print_string(), "0");
        assert_eq!(pos.find_next_maximal_subterm(), Some(b));
        assert_eq!(pos.term_pos().print_string(), "1.0\n");
        assert_eq!(pos.find_next_maximal_subterm(), Some(g_of_b));
        assert_eq!(pos.term_pos().print_string(), "1");
        assert_eq!(pos.find_next_maximal_subterm(), Some(f));
        assert!(pos.is_top());
        assert_eq!(pos.find_next_maximal_subterm(), Some(c));
        assert!(pos.is_top());
        assert_eq!(pos.find_next_maximal_subterm(), None);
    }

    #[test]
    fn rewrite_sequence_preserves_recursive_demodulator_stack_shape() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let c = typed_const(&mut bank, "c");
        let type_ = bank.signature().type_bank().default_type();
        let f_code = bank.signature_mut().insert_id("f", 2, false);
        bank.signature_mut()
            .declare_final_type(
                f_code,
                alloc_arrow_type(vec![type_.clone(), type_.clone(), type_]),
            )
            .unwrap();
        let from = typed_binary_with_code(&mut bank, f_code, &a, &b);
        let to = typed_binary_with_code(&mut bank, f_code, &c, &b);
        let demod = RewriteDemodulator::new(17);

        term_add_rw_link(&a, &c, Some(demod), false, RwResultType::LimitedRewritable);
        term_add_rw_link(&from, &to, None, false, RwResultType::LimitedRewritable);

        let mut stack = PStack::new();
        assert!(term_compute_rw_sequence(&mut stack, &from, &to, 99));
        assert_eq!(
            stack.as_slice(),
            &[
                RewriteSequenceEntry::Operation(99),
                RewriteSequenceEntry::Demodulator(demod)
            ]
        );

        let mut no_steps = PStack::new();
        assert!(!term_compute_rw_sequence(&mut no_steps, &to, &to, 99));
        assert!(no_steps.is_empty());
    }
}
