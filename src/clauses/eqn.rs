use crate::basics::error::{Diagnostic, ErrorCode};
use crate::clauses::eqn_props::{
    EqnProperties, EqnSide, PatEqnDirection, EP_IS_EQU_LITERAL, EP_IS_ORIENTED, EP_IS_POSITIVE,
    EP_MAX_IS_UP_TO_DATE, EP_NO_PROPS, EP_PSEUDO_LIT,
};
use crate::terms::acterms::term_ac_equal;
use crate::terms::signature::{FP_CL_SPLIT_DEF, FP_PSEUDO_PRED};
use crate::terms::simpletypes::type_is_predicate;
use crate::terms::termbanks::{
    tb_term_del_prop_count, tb_term_is_ground, tb_term_is_type_term, tb_term_is_x_type_term,
    TermBank,
};
use crate::terms::termfunc::{term_has_f_code, term_is_def_term};
use crate::terms::termtypes::{
    term_del_prop, term_set_prop, term_var_del_prop, term_var_search_prop, term_var_set_prop,
    DerefType, Term, TermProperties, TP_OP_FLAG, TP_PRED_POS,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Eqn {
    properties: EqnProperties,
    pos: i32,
    lterm: Term,
    rterm: Term,
}

impl Eqn {
    /// Allocates an equation/literal cell from already-shared terms.
    ///
    /// # Panics
    ///
    /// Panics if the C assertions around true predicate literals, equality
    /// right-hand sides, DB predicate declarations, or initialized equality
    /// arguments are violated.
    pub fn alloc(
        mut lterm: Term,
        mut rterm: Term,
        bank: &mut TermBank,
        mut positive: bool,
    ) -> Result<Self, Diagnostic> {
        let true_term = bank.true_term().clone();
        let false_term = bank.false_term().clone();

        if lterm == false_term {
            lterm = true_term.clone();
            positive = !positive;
        }
        if rterm == false_term {
            rterm = true_term.clone();
            positive = !positive;
        }
        if lterm == true_term {
            std::mem::swap(&mut lterm, &mut rterm);
        }

        let mut properties = EP_NO_PROPS;
        if positive {
            properties.set(EP_IS_POSITIVE);
        }
        if rterm == true_term {
            assert!(
                rterm.query_prop(TP_PRED_POS),
                "$true term must carry predicate-position property"
            );
            if lterm.f_code() > bank.signature().internal_symbols() {
                assert!(
                    !lterm.is_db_var(),
                    "DB variables are not declared as predicate symbols"
                );
                bank.signature_mut().declare_is_predicate(lterm.f_code())?;
            }
            lterm.set_prop(TP_PRED_POS);
            if !lterm.is_any_var() && bank.signature().query_prop(lterm.f_code(), FP_PSEUDO_PRED) {
                properties.set(EP_PSEUDO_LIT);
            }
        } else {
            assert_ne!(
                rterm.f_code(),
                crate::terms::signature::SIG_TRUE_CODE,
                "equality right side must not be a distinct $true cell"
            );
            properties.set(EP_IS_EQU_LITERAL);
        }

        let ltype = lterm.type_();
        let rtype = rterm.type_();
        let predicate_true_literal =
            ltype.as_ref().is_some_and(type_is_predicate) && rterm == true_term;
        if ltype != rtype && !predicate_true_literal {
            return Err(Diagnostic::new(ErrorCode::SYNTAX_ERROR, "Type error"));
        }

        Ok(Self {
            properties,
            pos: 0,
            lterm,
            rterm,
        })
    }

    /// Allocates a predicate literal, lifting `$eq`/`$neq` applications to
    /// equation-literal shape.
    ///
    /// # Panics
    ///
    /// Panics if `lterm` is not typed as boolean, or if an equality term lacks
    /// both arguments, matching the C assertions and argument access.
    pub fn alloc_flatten(
        lterm: Term,
        bank: &mut TermBank,
        mut sign: bool,
    ) -> Result<Self, Diagnostic> {
        assert!(
            lterm
                .type_()
                .as_ref()
                .is_some_and(crate::terms::simpletypes::Type::is_bool),
            "flattened literal input must be boolean"
        );
        let f_code = lterm.f_code();
        if f_code == bank.signature().eqn_code() || f_code == bank.signature().neqn_code() {
            if f_code == bank.signature().neqn_code() {
                sign = !sign;
            }
            let left = lterm
                .argument(0)
                .unwrap_or_else(|| panic!("equality literal left argument is uninitialized"));
            let right = lterm
                .argument(1)
                .unwrap_or_else(|| panic!("equality literal right argument is uninitialized"));
            Self::alloc(left, right, bank, sign)
        } else {
            Self::alloc(lterm, bank.true_term().clone(), bank, sign)
        }
    }

    pub fn create_true_lit(bank: &mut TermBank) -> Result<Self, Diagnostic> {
        Self::alloc(
            bank.true_term().clone(),
            bank.true_term().clone(),
            bank,
            true,
        )
    }

    /// Encodes a literal as a shared `$eq`/`$neq` term-bank term.
    ///
    /// # Panics
    ///
    /// Panics if either side is not present in the bank, or if term-bank top
    /// insertion invariants are violated, matching the C assertions.
    pub fn terms_tb_term_encode(
        bank: &mut TermBank,
        lterm: &Term,
        rterm: &Term,
        positive: bool,
        direction: PatEqnDirection,
    ) -> Result<Term, Diagnostic> {
        assert!(
            bank.find(lterm).is_some(),
            "left term must already be in the term bank"
        );
        assert!(
            bank.find(rterm).is_some(),
            "right term must already be in the term bank"
        );

        let f_code = bank.signature_mut().get_eqn_code(positive);
        assert_ne!(f_code, 0, "equality code allocation must succeed");
        let term = Term::top_alloc(f_code, 2);
        term.set_type(Some(bank.signature().type_bank().bool_type()));
        if direction == PatEqnDirection::Normal {
            term.set_argument(0, lterm.clone());
            term.set_argument(1, rterm.clone());
        } else {
            term.set_argument(0, rterm.clone());
            term.set_argument(1, lterm.clone());
        }
        bank.term_top_insert(term)
    }

    pub fn tb_term_encode(
        &self,
        bank: &mut TermBank,
        direction: PatEqnDirection,
    ) -> Result<Term, Diagnostic> {
        Self::terms_tb_term_encode(
            bank,
            &self.lterm,
            &self.rterm,
            self.is_positive(),
            direction,
        )
    }

    /// Decodes a shared `$eq`/`$neq` term into an equation/literal cell.
    ///
    /// # Panics
    ///
    /// Panics if the term is not headed by the bank's equality/inequality code
    /// or if either argument slot is uninitialized, matching the C assertions
    /// and direct argument access.
    pub fn tb_term_decode(bank: &mut TermBank, eqn: &Term) -> Result<Self, Diagnostic> {
        assert!(
            eqn.f_code() == bank.signature().eqn_code()
                || eqn.f_code() == bank.signature().neqn_code(),
            "encoded equation term must use equality or inequality code"
        );
        let positive = eqn.f_code() == bank.signature().eqn_code();
        let left = eqn
            .argument(0)
            .unwrap_or_else(|| panic!("encoded equation left argument is uninitialized"));
        let right = eqn
            .argument(1)
            .unwrap_or_else(|| panic!("encoded equation right argument is uninitialized"));
        Self::alloc(left, right, bank, positive)
    }

    #[must_use]
    pub const fn properties(&self) -> EqnProperties {
        self.properties
    }

    pub fn set_properties(&mut self, properties: EqnProperties) {
        self.properties = properties;
    }

    pub fn set_prop(&mut self, prop: EqnProperties) {
        self.properties.set(prop);
    }

    pub fn del_prop(&mut self, prop: EqnProperties) {
        self.properties.delete(prop);
    }

    pub fn flip_prop(&mut self, prop: EqnProperties) {
        self.properties.flip(prop);
    }

    #[must_use]
    pub const fn query_prop(&self, prop: EqnProperties) -> bool {
        self.properties.query(prop)
    }

    #[must_use]
    pub const fn is_any_prop_set(&self, prop: EqnProperties) -> bool {
        self.properties.is_any_set(prop)
    }

    #[must_use]
    pub const fn give_props(&self, prop: EqnProperties) -> EqnProperties {
        self.properties.give(prop)
    }

    #[must_use]
    pub const fn position(&self) -> i32 {
        self.pos
    }

    pub const fn set_position(&mut self, pos: i32) {
        self.pos = pos;
    }

    #[must_use]
    pub const fn left(&self) -> &Term {
        &self.lterm
    }

    #[must_use]
    pub const fn right(&self) -> &Term {
        &self.rterm
    }

    #[must_use]
    pub const fn is_oriented(&self) -> bool {
        self.properties.is_oriented()
    }

    #[must_use]
    pub const fn is_positive(&self) -> bool {
        self.properties.is_positive()
    }

    #[must_use]
    pub const fn is_negative(&self) -> bool {
        self.properties.is_negative()
    }

    #[must_use]
    /// # Panics
    ///
    /// Panics if the equation property bit and right-hand `$true` shape are
    /// inconsistent, matching the C macro assertions.
    pub fn is_equ_lit(&self, bank: &TermBank) -> bool {
        assert!(
            self.query_prop(EP_IS_EQU_LITERAL) || self.rterm == *bank.true_term(),
            "non-equational literals must have $true on the right"
        );
        assert!(
            !self.query_prop(EP_IS_EQU_LITERAL) || self.rterm != *bank.true_term(),
            "equational literals must not have $true on the right"
        );
        self.query_prop(EP_IS_EQU_LITERAL)
    }

    #[must_use]
    pub const fn is_maximal(&self) -> bool {
        self.properties.is_maximal()
    }

    #[must_use]
    pub const fn is_strictly_maximal(&self) -> bool {
        self.properties.is_strictly_maximal()
    }

    #[must_use]
    pub fn pred_code_fo(&self, bank: &TermBank) -> i64 {
        if self.is_equ_lit(bank) {
            0
        } else {
            self.lterm.f_code()
        }
    }

    #[must_use]
    pub fn pred_code_ho(&self, bank: &TermBank) -> i64 {
        if self.is_equ_lit(bank) || self.lterm.is_any_var() || self.lterm.is_phony_app() {
            0
        } else {
            self.lterm.f_code()
        }
    }

    #[must_use]
    pub fn is_split_lit(&self, bank: &TermBank) -> bool {
        !self.is_equ_lit(bank)
            && bank
                .signature()
                .query_prop(self.pred_code_fo(bank), FP_CL_SPLIT_DEF)
    }

    #[must_use]
    pub const fn has_equiv(&self) -> bool {
        self.properties.has_equiv()
    }

    #[must_use]
    pub const fn is_dominated(&self) -> bool {
        self.properties.is_dominated()
    }

    #[must_use]
    pub const fn dominates(&self) -> bool {
        self.properties.dominates()
    }

    #[must_use]
    pub const fn is_selected(&self) -> bool {
        self.properties.is_selected()
    }

    #[must_use]
    pub fn is_prop_true(&self) -> bool {
        self.lterm == self.rterm && self.is_positive()
    }

    #[must_use]
    pub fn is_prop_false(&self) -> bool {
        self.lterm == self.rterm && self.is_negative()
    }

    #[must_use]
    pub fn is_bool_var(&self, bank: &TermBank) -> bool {
        self.lterm.is_free_var() && self.rterm == *bank.true_term()
    }

    #[must_use]
    pub fn is_ground(&self) -> bool {
        tb_term_is_ground(&self.lterm) && tb_term_is_ground(&self.rterm)
    }

    #[must_use]
    pub fn is_pure_var(&self) -> bool {
        self.lterm.is_free_var() && self.rterm.is_free_var()
    }

    #[must_use]
    pub fn is_part_var(&self) -> bool {
        self.lterm.is_free_var() || self.rterm.is_free_var()
    }

    #[must_use]
    pub fn is_propositional(&self, bank: &TermBank) -> bool {
        !self.is_equ_lit(bank) && self.lterm.is_const()
    }

    #[must_use]
    pub fn is_type_pred(&self, bank: &TermBank) -> bool {
        !self.is_equ_lit(bank) && tb_term_is_type_term(&self.lterm)
    }

    #[must_use]
    pub fn is_x_type_pred(&self, bank: &TermBank) -> bool {
        !self.is_equ_lit(bank) && tb_term_is_x_type_term(&self.lterm)
    }

    #[must_use]
    pub fn is_real_x_type_pred(&self, bank: &TermBank) -> bool {
        !self.is_equ_lit(bank) && term_is_def_term(&self.lterm, 1)
    }

    #[must_use]
    pub fn is_simple_answer(&self, bank: &TermBank) -> bool {
        !self.lterm.is_db_var() && bank.signature().is_simple_answer_pred(self.lterm.f_code())
    }

    pub fn term_set_prop(&self, prop: TermProperties) {
        term_set_prop(&self.lterm, DerefType::Never, prop);
        term_set_prop(&self.rterm, DerefType::Never, prop);
    }

    #[must_use]
    pub fn tb_term_del_prop_count(&self, prop: TermProperties) -> i64 {
        tb_term_del_prop_count(&self.lterm, prop) + tb_term_del_prop_count(&self.rterm, prop)
    }

    pub fn term_del_prop(&self, prop: TermProperties) {
        term_del_prop(&self.lterm, DerefType::Never, prop);
        term_del_prop(&self.rterm, DerefType::Never, prop);
    }

    #[must_use]
    pub fn is_clausifiable(&self, bank: &TermBank) -> bool {
        self.lterm.type_().is_some_and(|type_| type_.is_bool())
            && (self.rterm != *bank.true_term()
                || (!self.lterm.is_any_var()
                    && bank.signature().is_logical_symbol(self.lterm.f_code())))
    }

    pub fn gc_mark_terms(&self, bank: &TermBank) {
        bank.gc_mark_term(&self.lterm);
        bank.gc_mark_term(&self.rterm);
    }

    pub fn swap_sides_simple(&mut self) {
        std::mem::swap(&mut self.lterm, &mut self.rterm);
    }

    pub fn swap_sides(&mut self) {
        self.del_prop(EP_IS_ORIENTED);
        self.del_prop(EP_MAX_IS_UP_TO_DATE);
        self.swap_sides_simple();
    }

    pub fn copy_to_bank(&self, bank: &mut TermBank) -> Result<Self, Diagnostic> {
        let lterm = bank.insert_no_props_cached(&self.lterm, DerefType::Always)?;
        let rterm = bank.insert_no_props_cached(&self.rterm, DerefType::Always)?;
        let mut handle = Self::alloc(lterm, rterm, bank, self.is_positive())?;
        handle.copy_properties_from(self);
        if !handle.is_oriented() {
            handle.del_prop(EP_MAX_IS_UP_TO_DATE);
        }
        Ok(handle)
    }

    pub fn flat_copy(&self, bank: &mut TermBank) -> Result<Self, Diagnostic> {
        let mut handle = Self::alloc(
            self.lterm.clone(),
            self.rterm.clone(),
            bank,
            self.is_positive(),
        )?;
        handle.copy_properties_from(self);
        if !handle.is_oriented() {
            handle.del_prop(EP_MAX_IS_UP_TO_DATE);
        }
        Ok(handle)
    }

    pub fn copy_repl(
        &self,
        bank: &mut TermBank,
        old: &Term,
        repl: &Term,
    ) -> Result<Self, Diagnostic> {
        let lterm = bank.insert_repl(&self.lterm, DerefType::Always, old, repl)?;
        let rterm = bank.insert_repl(&self.rterm, DerefType::Always, old, repl)?;
        let mut handle = Self::alloc(lterm, rterm, bank, self.is_positive())?;
        handle.copy_properties_from(self);
        handle.del_prop(EP_MAX_IS_UP_TO_DATE);
        handle.del_prop(EP_IS_ORIENTED);
        Ok(handle)
    }

    pub fn copy_repl_plain(
        &self,
        bank: &mut TermBank,
        old: &Term,
        repl: &Term,
    ) -> Result<Self, Diagnostic> {
        let lterm = bank.insert_repl_plain(&self.lterm, old, repl)?;
        let rterm = bank.insert_repl_plain(&self.rterm, old, repl)?;
        let mut handle = Self::alloc(lterm, rterm, bank, self.is_positive())?;
        handle.copy_properties_from(self);
        handle.del_prop(EP_MAX_IS_UP_TO_DATE);
        handle.del_prop(EP_IS_ORIENTED);
        Ok(handle)
    }

    pub fn copy_opt(&self, bank: &mut TermBank) -> Result<Self, Diagnostic> {
        let lterm = bank.insert_opt(&self.lterm, DerefType::Always)?;
        let rterm = bank.insert_opt(&self.rterm, DerefType::Always)?;
        let mut handle = Self::alloc(lterm, rterm, bank, self.is_positive())?;
        handle.copy_properties_from(self);
        handle.del_prop(EP_MAX_IS_UP_TO_DATE);
        handle.del_prop(EP_IS_ORIENTED);
        Ok(handle)
    }

    pub fn copy_disjoint(&self, bank: &mut TermBank) -> Result<Self, Diagnostic> {
        let lterm = bank.insert_disjoint(&self.lterm)?;
        let rterm = bank.insert_disjoint(&self.rterm)?;
        let mut handle = Self::alloc(lterm, rterm, bank, self.is_positive())?;
        handle.copy_properties_from(self);
        Ok(handle)
    }

    #[must_use]
    pub fn is_trivial(&self) -> bool {
        self.lterm == self.rterm
    }

    #[must_use]
    pub fn is_ac_trivial(&self, bank: &TermBank) -> bool {
        term_ac_equal(bank.signature(), &self.lterm, &self.rterm)
    }

    #[must_use]
    pub fn terms_are_distinct(&self, bank: &TermBank) -> bool {
        self.lterm.is_const()
            && self.rterm.is_const()
            && bank
                .signature()
                .is_any_func_prop_set(self.lterm.f_code(), bank.signature().distinct_props())
            && bank
                .signature()
                .is_any_func_prop_set(self.rterm.f_code(), bank.signature().distinct_props())
            && self.lterm.f_code() != self.rterm.f_code()
    }

    #[must_use]
    pub fn is_true(&self, bank: &TermBank) -> bool {
        if self.is_positive() {
            self.is_trivial()
        } else {
            self.terms_are_distinct(bank)
        }
    }

    #[must_use]
    pub fn is_false(&self, bank: &TermBank) -> bool {
        if self.is_negative() {
            self.is_trivial()
        } else {
            self.terms_are_distinct(bank)
        }
    }

    #[must_use]
    pub fn has_unbound_vars(&self, dom_side: EqnSide) -> bool {
        let (domain, range) = if dom_side == EqnSide::LeftSide {
            (&self.lterm, &self.rterm)
        } else {
            (&self.rterm, &self.lterm)
        };

        term_var_set_prop(range, DerefType::Never, TP_OP_FLAG);
        term_var_del_prop(domain, DerefType::Never, TP_OP_FLAG);
        term_var_search_prop(range, DerefType::Never, TP_OP_FLAG)
    }

    #[must_use]
    pub fn is_definition(&self, bank: &TermBank, min_arity: usize) -> EqnSide {
        if self.is_negative() {
            return EqnSide::NoSide;
        }
        if term_is_def_term(&self.lterm, min_arity)
            && !bank
                .signature()
                .query_prop(self.lterm.f_code(), FP_PSEUDO_PRED)
            && !term_has_f_code(&self.rterm, self.lterm.f_code())
            && !self.has_unbound_vars(EqnSide::LeftSide)
        {
            return EqnSide::LeftSide;
        }
        if term_is_def_term(&self.rterm, min_arity)
            && !bank
                .signature()
                .query_prop(self.rterm.f_code(), FP_PSEUDO_PRED)
            && !term_has_f_code(&self.lterm, self.rterm.f_code())
            && !self.has_unbound_vars(EqnSide::RightSide)
        {
            return EqnSide::RightSide;
        }
        EqnSide::NoSide
    }

    fn copy_properties_from(&mut self, source: &Self) {
        self.properties = self.give_props(EP_IS_POSITIVE) | (source.properties & !EP_IS_POSITIVE);
    }
}

#[cfg(test)]
mod tests {
    use super::Eqn;
    use crate::clauses::eqn_props::{
        EqnSide, PatEqnDirection, EP_FROM_CLAUSE_LIT, EP_IS_EQU_LITERAL, EP_IS_MAXIMAL,
        EP_IS_ORIENTED, EP_IS_PM_INTO_LIT, EP_IS_POSITIVE, EP_IS_SELECTED, EP_MAX_IS_UP_TO_DATE,
    };
    use crate::terms::signature::{
        FunctionProperties, Signature, FP_CL_SPLIT_DEF, FP_IS_INTEGER, FP_PSEUDO_PRED,
    };
    use crate::terms::simpletypes::alloc_arrow_type;
    use crate::terms::termbanks::TermBank;
    use crate::terms::termfunc::term_standard_weight;
    use crate::terms::termtypes::{Term, TP_CHECK_FLAG, TP_PRED_POS};
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
            .declare_final_type(f_code, type_.clone())
            .unwrap();
        let term = Term::const_cell_alloc(f_code);
        term.set_type(Some(type_));
        bank.insert(&term, crate::terms::termtypes::DerefType::Never)
            .unwrap()
    }

    fn typed_pred_const(bank: &mut TermBank, name: &str) -> Term {
        let bool_type = bank.signature().type_bank().bool_type();
        let f_code = bank.signature_mut().insert_id(name, 0, false);
        bank.signature_mut()
            .declare_final_type(f_code, bool_type.clone())
            .unwrap();
        let term = Term::const_cell_alloc(f_code);
        term.set_type(Some(bool_type));
        bank.insert(&term, crate::terms::termtypes::DerefType::Never)
            .unwrap()
    }

    fn typed_unary(bank: &mut TermBank, name: &str, arg: &Term) -> Term {
        let type_ = bank.signature().type_bank().default_type();
        let f_code = bank.signature_mut().insert_id(name, 1, false);
        bank.signature_mut()
            .declare_final_type(f_code, alloc_arrow_type(vec![type_.clone(), type_.clone()]))
            .unwrap();
        let term = Term::top_alloc(f_code, 1);
        term.set_type(Some(type_));
        term.set_argument(0, arg.clone());
        bank.insert(&term, crate::terms::termtypes::DerefType::Never)
            .unwrap()
    }

    #[test]
    fn alloc_normalizes_false_and_true_like_c() {
        let mut bank = test_bank();
        let atom = typed_pred_const(&mut bank, "p");
        let false_term = bank.false_term().clone();

        let eq = Eqn::alloc(false_term, atom.clone(), &mut bank, true).unwrap();

        assert_eq!(eq.left(), &atom);
        assert_eq!(eq.right(), bank.true_term());
        assert!(eq.is_negative());
        assert!(!eq.is_equ_lit(&bank));
        assert!(atom.query_prop(TP_PRED_POS));
        assert_eq!(eq.position(), 0);
    }

    #[test]
    fn alloc_marks_equational_and_pseudo_predicate_literals() {
        let mut bank = test_bank();
        let left = typed_const(&mut bank, "a");
        let right = typed_const(&mut bank, "b");

        let eq = Eqn::alloc(left.clone(), right.clone(), &mut bank, true).unwrap();

        assert!(eq.is_positive());
        assert!(eq.is_equ_lit(&bank));
        assert_eq!(eq.left(), &left);
        assert_eq!(eq.right(), &right);

        let pseudo = typed_pred_const(&mut bank, "answer_like");
        bank.signature_mut()
            .set_func_prop(pseudo.f_code(), FP_PSEUDO_PRED);
        let lit = Eqn::alloc(pseudo, bank.true_term().clone(), &mut bank, true).unwrap();
        assert!(lit.query_prop(crate::clauses::eqn_props::EP_PSEUDO_LIT));
    }

    #[test]
    fn alloc_flatten_lifts_eq_and_flips_neq_sign() {
        let mut bank = test_bank();
        let left = typed_const(&mut bank, "a");
        let right = typed_const(&mut bank, "b");
        let eqn_type = bank.signature().type_bank().bool_type();
        let eqn_code = bank.signature().eqn_code();
        let neqn_code = bank.signature().neqn_code();

        let eq_term = Term::top_alloc(eqn_code, 2);
        eq_term.set_type(Some(eqn_type.clone()));
        eq_term.set_argument(0, left.clone());
        eq_term.set_argument(1, right.clone());
        let eq = Eqn::alloc_flatten(eq_term, &mut bank, true).unwrap();
        assert!(eq.is_positive());
        assert!(eq.is_equ_lit(&bank));
        assert_eq!(eq.left(), &left);

        let neq_term = Term::top_alloc(neqn_code, 2);
        neq_term.set_type(Some(eqn_type));
        neq_term.set_argument(0, left);
        neq_term.set_argument(1, right);
        let neq = Eqn::alloc_flatten(neq_term, &mut bank, true).unwrap();
        assert!(neq.is_negative());
        assert!(neq.is_equ_lit(&bank));
    }

    #[test]
    fn term_bank_encoding_and_decoding_match_c_shape() {
        let mut bank = test_bank();
        let left = typed_const(&mut bank, "a");
        let right = typed_const(&mut bank, "b");

        let encoded =
            Eqn::terms_tb_term_encode(&mut bank, &left, &right, true, PatEqnDirection::Normal)
                .unwrap();
        assert_eq!(encoded.f_code(), bank.signature().eqn_code());
        assert_eq!(
            encoded.type_(),
            Some(bank.signature().type_bank().bool_type())
        );
        assert_eq!(encoded.argument(0), Some(left.clone()));
        assert_eq!(encoded.argument(1), Some(right.clone()));
        assert!(encoded.is_shared());

        let decoded = Eqn::tb_term_decode(&mut bank, &encoded).unwrap();
        assert!(decoded.is_positive());
        assert!(decoded.is_equ_lit(&bank));
        assert_eq!(decoded.left(), &left);
        assert_eq!(decoded.right(), &right);

        let reversed =
            Eqn::terms_tb_term_encode(&mut bank, &left, &right, false, PatEqnDirection::Reverse)
                .unwrap();
        assert_eq!(reversed.f_code(), bank.signature().neqn_code());
        assert_eq!(reversed.argument(0), Some(right.clone()));
        assert_eq!(reversed.argument(1), Some(left.clone()));
        let decoded_reversed = Eqn::tb_term_decode(&mut bank, &reversed).unwrap();
        assert!(decoded_reversed.is_negative());
        assert_eq!(decoded_reversed.left(), &right);
        assert_eq!(decoded_reversed.right(), &left);
    }

    #[test]
    fn instance_term_bank_encoding_uses_current_polarity() {
        let mut bank = test_bank();
        let left = typed_const(&mut bank, "a");
        let right = typed_const(&mut bank, "b");
        let mut eq = Eqn::alloc(left.clone(), right.clone(), &mut bank, true).unwrap();
        eq.flip_prop(EP_IS_POSITIVE);

        let encoded = eq
            .tb_term_encode(&mut bank, PatEqnDirection::Normal)
            .unwrap();

        assert_eq!(encoded.f_code(), bank.signature().neqn_code());
        assert_eq!(encoded.argument(0), Some(left));
        assert_eq!(encoded.argument(1), Some(right));
    }

    #[test]
    fn property_forwarders_and_swap_match_c_macros() {
        let mut bank = test_bank();
        let left = typed_const(&mut bank, "a");
        let right = typed_const(&mut bank, "b");
        let mut eq = Eqn::alloc(left.clone(), right.clone(), &mut bank, true).unwrap();

        eq.set_prop(EP_IS_ORIENTED | EP_MAX_IS_UP_TO_DATE | EP_IS_SELECTED);
        assert!(eq.is_oriented());
        assert!(eq.is_selected());
        assert!(eq.is_any_prop_set(EP_IS_SELECTED | EP_IS_MAXIMAL));
        assert_eq!(
            eq.give_props(EP_IS_SELECTED | EP_IS_MAXIMAL),
            EP_IS_SELECTED
        );
        eq.flip_prop(EP_IS_SELECTED);
        assert!(!eq.is_selected());

        eq.swap_sides();
        assert_eq!(eq.left(), &right);
        assert_eq!(eq.right(), &left);
        assert!(!eq.is_oriented());
        assert!(!eq.query_prop(EP_MAX_IS_UP_TO_DATE));
    }

    #[test]
    fn copy_variants_preserve_and_clear_properties_like_c() {
        let mut bank = test_bank();
        let left = typed_const(&mut bank, "a");
        let right = typed_const(&mut bank, "b");
        let replacement = typed_const(&mut bank, "c");
        let mut eq = Eqn::alloc(left.clone(), right.clone(), &mut bank, true).unwrap();
        eq.set_prop(EP_IS_ORIENTED | EP_MAX_IS_UP_TO_DATE | EP_IS_SELECTED | EP_FROM_CLAUSE_LIT);

        let copied = eq.copy_to_bank(&mut bank).unwrap();
        assert!(copied.is_oriented());
        assert!(copied.query_prop(EP_MAX_IS_UP_TO_DATE));
        assert!(copied.is_selected());
        assert!(copied.query_prop(EP_IS_EQU_LITERAL));

        let flat = eq.flat_copy(&mut bank).unwrap();
        assert_eq!(flat.left(), &left);
        assert!(flat.query_prop(EP_FROM_CLAUSE_LIT));

        let repl = eq.copy_repl_plain(&mut bank, &left, &replacement).unwrap();
        assert_eq!(repl.left(), &replacement);
        assert!(!repl.is_oriented());
        assert!(!repl.query_prop(EP_MAX_IS_UP_TO_DATE));
        assert!(repl.is_selected());

        let repl_instantiated = eq.copy_repl(&mut bank, &right, &replacement).unwrap();
        assert_eq!(repl_instantiated.right(), &replacement);
        assert!(!repl_instantiated.is_oriented());

        let opt = eq.copy_opt(&mut bank).unwrap();
        assert!(!opt.is_oriented());
        assert!(opt.query_prop(EP_IS_SELECTED));
    }

    #[test]
    fn copy_disjoint_replaces_free_variables_with_alternates() {
        let mut bank = test_bank();
        let type_ = bank.signature().type_bank().default_type();
        let x = bank.vars().var_assert_alloc(-2, &type_);
        let y = bank.vars().var_assert_alloc(-4, &type_);
        let eq = Eqn::alloc(x.clone(), y.clone(), &mut bank, true).unwrap();

        let disjoint = eq.copy_disjoint(&mut bank).unwrap();

        assert_eq!(disjoint.left().f_code(), -1);
        assert_eq!(disjoint.right().f_code(), -3);
        assert_ne!(disjoint.left(), &x);
        assert_ne!(disjoint.right(), &y);
    }

    #[test]
    fn simple_term_predicates_match_macros() {
        let mut bank = test_bank();
        let left = typed_const(&mut bank, "a");
        let right = typed_const(&mut bank, "b");
        let eq = Eqn::alloc(left.clone(), right.clone(), &mut bank, true).unwrap();
        assert!(eq.is_ground());
        assert!(!eq.is_part_var());
        assert!(!eq.is_trivial());

        let true_lit = Eqn::create_true_lit(&mut bank).unwrap();
        assert!(true_lit.is_prop_true());
        assert!(!true_lit.is_prop_false());
        assert!(true_lit.is_trivial());

        let bool_type = bank.signature().type_bank().bool_type();
        let x = bank.vars().var_assert_alloc(-2, &bool_type);
        let bool_lit = Eqn::alloc(x.clone(), bank.true_term().clone(), &mut bank, true).unwrap();
        assert!(bool_lit.is_bool_var(&bank));
        assert!(bool_lit.is_part_var());
        assert!(!bool_lit.is_pure_var());

        let type_ = bank.signature().type_bank().default_type();
        let x = bank.vars().var_assert_alloc(-6, &type_);
        let y = bank.vars().var_assert_alloc(-4, &type_);
        let var_eq = Eqn::alloc(x, y, &mut bank, true).unwrap();
        assert!(var_eq.is_pure_var());
    }

    #[test]
    fn predicate_code_split_and_distinct_helpers_match_c_shape() {
        let mut bank = test_bank();
        let atom = typed_pred_const(&mut bank, "split_pred");
        bank.signature_mut()
            .set_func_prop(atom.f_code(), FP_CL_SPLIT_DEF);
        let lit = Eqn::alloc(atom.clone(), bank.true_term().clone(), &mut bank, true).unwrap();

        assert_eq!(lit.pred_code_fo(&bank), atom.f_code());
        assert_eq!(lit.pred_code_ho(&bank), atom.f_code());
        assert!(lit.is_split_lit(&bank));
        assert!(lit.is_propositional(&bank));

        let left = typed_const(&mut bank, "1");
        let right = typed_const(&mut bank, "2");
        bank.signature_mut()
            .set_func_prop(left.f_code(), FP_IS_INTEGER);
        bank.signature_mut()
            .set_func_prop(right.f_code(), FP_IS_INTEGER);
        let distinct = Eqn::alloc(left, right, &mut bank, true).unwrap();
        assert!(distinct.terms_are_distinct(&bank));
        assert!(!distinct.is_split_lit(&bank));
    }

    #[test]
    fn truth_and_falsehood_helpers_match_c_cases() {
        let mut bank = test_bank();
        let left = typed_const(&mut bank, "a");
        let right = typed_const(&mut bank, "b");

        let positive_trivial = Eqn::alloc(left.clone(), left.clone(), &mut bank, true).unwrap();
        assert!(positive_trivial.is_true(&bank));
        assert!(!positive_trivial.is_false(&bank));

        let negative_trivial = Eqn::alloc(left.clone(), left.clone(), &mut bank, false).unwrap();
        assert!(!negative_trivial.is_true(&bank));
        assert!(negative_trivial.is_false(&bank));

        bank.signature_mut()
            .set_func_prop(left.f_code(), FP_IS_INTEGER);
        bank.signature_mut()
            .set_func_prop(right.f_code(), FP_IS_INTEGER);
        let positive_distinct = Eqn::alloc(left.clone(), right.clone(), &mut bank, true).unwrap();
        assert!(!positive_distinct.is_true(&bank));
        assert!(positive_distinct.is_false(&bank));

        let negative_distinct = Eqn::alloc(left, right, &mut bank, false).unwrap();
        assert!(negative_distinct.is_true(&bank));
        assert!(!negative_distinct.is_false(&bank));
    }

    #[test]
    fn unbound_variable_check_preserves_c_domain_side_shape() {
        let mut bank = test_bank();
        let type_ = bank.signature().type_bank().default_type();
        let x = bank.vars().var_assert_alloc(-2, &type_);
        let y = bank.vars().var_assert_alloc(-4, &type_);
        let left = typed_unary(&mut bank, "f", &x);
        let right_with_x = typed_unary(&mut bank, "g", &x);
        let right_with_y = typed_unary(&mut bank, "h", &y);

        let bound = Eqn::alloc(left.clone(), right_with_x, &mut bank, true).unwrap();
        assert!(!bound.has_unbound_vars(EqnSide::LeftSide));
        assert!(!bound.has_unbound_vars(EqnSide::RightSide));

        let unbound = Eqn::alloc(left.clone(), right_with_y, &mut bank, true).unwrap();
        assert!(unbound.has_unbound_vars(EqnSide::LeftSide));

        let fallback_right_side =
            Eqn::alloc(typed_unary(&mut bank, "q", &y), left, &mut bank, true).unwrap();
        assert!(fallback_right_side.has_unbound_vars(EqnSide::RightSide));
        assert!(fallback_right_side.has_unbound_vars(EqnSide::NoSide));
        assert!(fallback_right_side.has_unbound_vars(EqnSide::BothSides));
    }

    #[test]
    fn definition_detection_matches_positive_def_term_rules() {
        let mut bank = test_bank();
        let type_ = bank.signature().type_bank().default_type();
        let x = bank.vars().var_assert_alloc(-2, &type_);
        let y = bank.vars().var_assert_alloc(-4, &type_);

        let def_head = typed_unary(&mut bank, "f", &x);
        let body = typed_unary(&mut bank, "g", &x);
        let def = Eqn::alloc(def_head.clone(), body, &mut bank, true).unwrap();
        assert_eq!(def.is_definition(&bank, 1), EqnSide::LeftSide);
        assert_eq!(def.is_definition(&bank, 2), EqnSide::NoSide);

        let right_head = typed_unary(&mut bank, "h", &x);
        let body = typed_const(&mut bank, "body");
        let right_def = Eqn::alloc(body, right_head, &mut bank, true).unwrap();
        assert_eq!(right_def.is_definition(&bank, 1), EqnSide::RightSide);

        let negative = Eqn::alloc(
            def_head.clone(),
            typed_unary(&mut bank, "m", &x),
            &mut bank,
            false,
        )
        .unwrap();
        assert_eq!(negative.is_definition(&bank, 1), EqnSide::NoSide);

        let unbound_body = typed_unary(&mut bank, "n", &y);
        let unbound = Eqn::alloc(def_head.clone(), unbound_body, &mut bank, true).unwrap();
        assert_eq!(unbound.is_definition(&bank, 1), EqnSide::NoSide);

        bank.signature_mut()
            .set_func_prop(def_head.f_code(), FP_PSEUDO_PRED);
        let pseudo = Eqn::alloc(
            def_head,
            typed_const(&mut bank, "pseudo_body"),
            &mut bank,
            true,
        )
        .unwrap();
        assert_eq!(pseudo.is_definition(&bank, 1), EqnSide::NoSide);
    }

    #[test]
    fn term_property_helpers_touch_both_sides() {
        let mut bank = test_bank();
        let left = typed_const(&mut bank, "a");
        let right = typed_const(&mut bank, "b");
        let eq = Eqn::alloc(left.clone(), right.clone(), &mut bank, true).unwrap();

        eq.term_set_prop(TP_CHECK_FLAG);
        assert!(left.query_prop(TP_CHECK_FLAG));
        assert!(right.query_prop(TP_CHECK_FLAG));
        assert_eq!(eq.tb_term_del_prop_count(TP_CHECK_FLAG), 2);
        assert!(!left.query_prop(TP_CHECK_FLAG));
        assert!(!right.query_prop(TP_CHECK_FLAG));

        eq.term_set_prop(TP_CHECK_FLAG);
        eq.term_del_prop(TP_CHECK_FLAG);
        assert!(!left.query_prop(TP_CHECK_FLAG));
        assert!(!right.query_prop(TP_CHECK_FLAG));
    }

    #[test]
    fn ac_trivial_uses_ac_normalization() {
        let mut bank = test_bank();
        let type_ = bank.signature().type_bank().default_type();
        let f_code = bank.signature_mut().insert_id("f", 2, false);
        bank.signature_mut()
            .declare_final_type(
                f_code,
                alloc_arrow_type(vec![type_.clone(), type_.clone(), type_]),
            )
            .unwrap();
        bank.signature_mut()
            .set_func_prop(f_code, FunctionProperties::IS_AC);
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");

        let left = Term::top_alloc(f_code, 2);
        left.set_argument(0, a.clone());
        left.set_argument(1, b.clone());
        let left = bank
            .insert(&left, crate::terms::termtypes::DerefType::Never)
            .unwrap();
        let right = Term::top_alloc(f_code, 2);
        right.set_argument(0, b);
        right.set_argument(1, a);
        let right = bank
            .insert(&right, crate::terms::termtypes::DerefType::Never)
            .unwrap();

        assert_eq!(term_standard_weight(&left), term_standard_weight(&right));
        let eq = Eqn::alloc(left, right, &mut bank, true).unwrap();
        assert!(eq.is_ac_trivial(&bank));
    }

    #[test]
    fn clausifiable_rejects_non_logical_predicate_literals() {
        let mut bank = test_bank();
        let atom = typed_pred_const(&mut bank, "p");
        let lit = Eqn::alloc(atom, bank.true_term().clone(), &mut bank, true).unwrap();

        assert!(!lit.is_clausifiable(&bank));

        let true_lit = Eqn::create_true_lit(&mut bank).unwrap();
        assert!(true_lit.is_clausifiable(&bank));
    }

    #[test]
    fn rarely_used_property_shortcuts_follow_bits() {
        let mut bank = test_bank();
        let left = typed_const(&mut bank, "a");
        let right = typed_const(&mut bank, "b");
        let mut eq = Eqn::alloc(left, right, &mut bank, true).unwrap();

        eq.set_prop(
            EP_IS_MAXIMAL
                | crate::clauses::eqn_props::EP_IS_STRICTLY_MAXIMAL
                | crate::clauses::eqn_props::EP_HAS_EQUIV
                | crate::clauses::eqn_props::EP_IS_DOMINATED
                | EP_IS_PM_INTO_LIT,
        );

        assert!(eq.is_maximal());
        assert!(eq.is_strictly_maximal());
        assert!(eq.has_equiv());
        assert!(eq.is_dominated());
        assert!(eq.dominates());
        assert!(eq.query_prop(EP_IS_PM_INTO_LIT));
    }
}
