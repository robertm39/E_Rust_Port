use crate::basics::pdarrays::{PDArrayIndex, PDIntArray};
use crate::clauses::clause::Clause;
use crate::terms::functypes::FunCode;
use crate::terms::signature::Signature;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[repr(i32)]
pub enum AxiomType {
    NoAxiom = 0,
    ClauseAxiom = 1,
    FormulaAxiom = 2,
}

#[derive(Clone, Debug, PartialEq)]
enum WAxiomPayload {
    Clause(Box<Clause>),
    FormulaFCodes(Vec<FunCode>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct WAxiom {
    axiom_type: AxiomType,
    weight: f64,
    payload: WAxiomPayload,
}

impl WAxiom {
    #[must_use]
    pub fn alloc_clause(clause: &Clause) -> Self {
        Self {
            axiom_type: AxiomType::ClauseAxiom,
            weight: 0.0,
            payload: WAxiomPayload::Clause(Box::new(clause.clone())),
        }
    }

    #[must_use]
    pub fn alloc_formula_fcodes(f_codes: Vec<FunCode>) -> Self {
        Self {
            axiom_type: AxiomType::FormulaAxiom,
            weight: 0.0,
            payload: WAxiomPayload::FormulaFCodes(f_codes),
        }
    }

    #[must_use]
    pub const fn axiom_type(&self) -> AxiomType {
        self.axiom_type
    }

    #[must_use]
    pub const fn weight(&self) -> f64 {
        self.weight
    }

    pub const fn set_weight(&mut self, weight: f64) {
        self.weight = weight;
    }

    #[must_use]
    pub fn fcodes(&self) -> Vec<FunCode> {
        match &self.payload {
            WAxiomPayload::Clause(clause) => {
                let mut f_codes = Vec::new();
                clause.return_fcodes(&mut f_codes);
                f_codes
            }
            WAxiomPayload::FormulaFCodes(f_codes) => f_codes.clone(),
        }
    }

    /// Assigns this axiom the average nonzero relevance of its nonspecial
    /// function symbols.
    ///
    /// If no relevant nonspecial symbol is found, the old weight is preserved,
    /// matching `WAxiomAddRelEval`.
    #[expect(
        clippy::cast_precision_loss,
        reason = "C averages long relevance values as double"
    )]
    pub fn add_rel_eval(&mut self, sig: &Signature, rel_vec: &mut PDIntArray) {
        let mut sum = 0.0;
        let mut count = 0_i64;

        for f_code in self.fcodes() {
            if sig.is_special(f_code) {
                continue;
            }
            let rel = rel_vec.element_int(pd_index(f_code));
            if rel != 0 {
                sum += rel as f64;
                count += 1;
            }
        }
        if count != 0 {
            self.weight = sum / count as f64;
        }
    }
}

#[must_use]
pub fn w_axiom_cmp(left: &WAxiom, right: &WAxiom) -> i32 {
    if left.weight < right.weight {
        return -1;
    }
    if left.weight > right.weight {
        return 1;
    }
    if left.axiom_type < right.axiom_type {
        return -1;
    }
    if left.axiom_type > right.axiom_type {
        return 1;
    }
    match w_axiom_key(left).cmp(&w_axiom_key(right)) {
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Greater => 1,
    }
}

#[must_use]
pub fn w_axiom_key(axiom: &WAxiom) -> usize {
    std::ptr::from_ref(axiom) as usize
}

fn pd_index(f_code: FunCode) -> PDArrayIndex {
    PDArrayIndex::try_from(f_code).unwrap_or(PDArrayIndex::MAX)
}

#[cfg(test)]
mod tests {
    use super::{w_axiom_cmp, AxiomType, WAxiom};
    use crate::basics::pdarrays::{PDIntArray, GROW_EXPONENTIAL};
    use crate::clauses::clause::Clause;
    use crate::clauses::eqn::Eqn;
    use crate::clauses::eqnlist::EqnList;
    use crate::terms::functypes::FunCode;
    use crate::terms::signature::Signature;
    use crate::terms::termbanks::TermBank;
    use crate::terms::termtypes::Term;
    use crate::terms::typebanks::TypeBank;

    fn test_bank() -> TermBank {
        let mut signature = Signature::new(TypeBank::new());
        signature.insert_internal_codes().unwrap();
        TermBank::new(signature).unwrap()
    }

    fn typed_const(bank: &mut TermBank, name: &str, special: bool) -> Term {
        let type_ = bank.signature().type_bank().default_type();
        let f_code = bank.signature_mut().insert_id(name, 0, special);
        bank.signature_mut()
            .declare_final_type(f_code, type_)
            .unwrap();
        bank.create_const_term(f_code).unwrap()
    }

    fn unit_clause(bank: &mut TermBank, left: &Term, right: &Term, ident: i64) -> Clause {
        let literal = Eqn::alloc(left.clone(), right.clone(), bank, true).unwrap();
        let mut clause = Clause::alloc(EqnList::from_vec(vec![literal]));
        clause.set_ident(ident);
        clause
    }

    fn store_rel(rel_vec: &mut PDIntArray, f_code: FunCode, value: i64) {
        rel_vec.assign(isize::try_from(f_code).unwrap(), value);
    }

    fn assert_weight_eq(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < f64::EPSILON,
            "actual weight {actual} differs from expected {expected}"
        );
    }

    #[test]
    fn axiom_type_discriminants_match_c_enum() {
        assert_eq!(AxiomType::NoAxiom as i32, 0);
        assert_eq!(AxiomType::ClauseAxiom as i32, 1);
        assert_eq!(AxiomType::FormulaAxiom as i32, 2);
    }

    #[test]
    fn allocation_initializes_type_and_zero_weight() {
        let mut bank = test_bank();
        let left = typed_const(&mut bank, "a", false);
        let right = typed_const(&mut bank, "b", false);
        let clause = unit_clause(&mut bank, &left, &right, 10);
        let axiom = WAxiom::alloc_clause(&clause);
        let formula = WAxiom::alloc_formula_fcodes(vec![left.f_code()]);

        assert_eq!(axiom.axiom_type(), AxiomType::ClauseAxiom);
        assert_weight_eq(axiom.weight(), 0.0);
        assert_eq!(axiom.fcodes().len(), 2);
        assert_eq!(formula.axiom_type(), AxiomType::FormulaAxiom);
        assert_eq!(formula.fcodes(), vec![left.f_code()]);
    }

    #[test]
    fn relevance_eval_averages_nonzero_nonspecial_symbol_relevances() {
        let mut bank = test_bank();
        let relevant = typed_const(&mut bank, "a", false);
        let other_relevant = typed_const(&mut bank, "b", false);
        let special = typed_const(&mut bank, "ignored", true);
        let clause = Clause::alloc(EqnList::from_vec(vec![
            Eqn::alloc(relevant.clone(), other_relevant.clone(), &mut bank, true).unwrap(),
            Eqn::alloc(special.clone(), relevant.clone(), &mut bank, true).unwrap(),
        ]));
        let mut axiom = WAxiom::alloc_clause(&clause);
        let mut rel_vec = PDIntArray::new_int(2, GROW_EXPONENTIAL);
        store_rel(&mut rel_vec, relevant.f_code(), 2);
        store_rel(&mut rel_vec, other_relevant.f_code(), 4);
        store_rel(&mut rel_vec, special.f_code(), 100);

        axiom.add_rel_eval(bank.signature(), &mut rel_vec);

        assert_weight_eq(axiom.weight(), 3.0);
    }

    #[test]
    fn relevance_eval_preserves_old_weight_when_no_symbol_contributes() {
        let mut bank = test_bank();
        let special = typed_const(&mut bank, "ignored", true);
        let mut axiom = WAxiom::alloc_formula_fcodes(vec![special.f_code()]);
        axiom.set_weight(7.5);
        let mut rel_vec = PDIntArray::new_int(2, GROW_EXPONENTIAL);
        store_rel(&mut rel_vec, special.f_code(), 100);

        axiom.add_rel_eval(bank.signature(), &mut rel_vec);

        assert_weight_eq(axiom.weight(), 7.5);
    }

    #[test]
    fn comparison_orders_by_weight_type_and_pointer_identity() {
        let mut bank = test_bank();
        let left_term = typed_const(&mut bank, "a", false);
        let right_term = typed_const(&mut bank, "b", false);
        let clause = unit_clause(&mut bank, &left_term, &right_term, 10);
        let mut light = WAxiom::alloc_clause(&clause);
        let mut heavy = WAxiom::alloc_clause(&clause);
        heavy.set_weight(1.0);

        assert_eq!(w_axiom_cmp(&light, &heavy), -1);
        light.set_weight(1.0);
        let mut formula = WAxiom::alloc_formula_fcodes(vec![left_term.f_code()]);
        formula.set_weight(1.0);
        assert_eq!(w_axiom_cmp(&light, &formula), -1);

        let same_type_left = Box::new(WAxiom::alloc_clause(&clause));
        let same_type_right = Box::new(WAxiom::alloc_clause(&clause));
        assert_eq!(w_axiom_cmp(&same_type_left, &same_type_left), 0);
        assert_ne!(w_axiom_cmp(&same_type_left, &same_type_right), 0);
    }
}
