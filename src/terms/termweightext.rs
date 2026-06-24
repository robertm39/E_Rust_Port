use crate::terms::termtypes::Term;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[repr(i32)]
pub enum TermWeightExtensionStyle {
    #[default]
    Simple = 0,
    SubtermsSum = 1,
    SubtermsMax = 2,
}

impl TermWeightExtensionStyle {
    #[must_use]
    pub const fn c_value(self) -> i32 {
        self as i32
    }

    #[must_use]
    pub const fn from_c_value(value: i32) -> Option<Self> {
        match value {
            0 => Some(Self::Simple),
            1 => Some(Self::SubtermsSum),
            2 => Some(Self::SubtermsMax),
            _ => None,
        }
    }
}

pub type TermWeightExtenstionStyle = TermWeightExtensionStyle;

pub struct TermWeightExtension<Data, WeightFun> {
    max_term_multiplier: f64,
    max_literal_multiplier: f64,
    pos_eq_multiplier: f64,
    ext_style: TermWeightExtensionStyle,
    term_weight_fun: WeightFun,
    data: Data,
}

impl<Data, WeightFun> TermWeightExtension<Data, WeightFun> {
    #[must_use]
    pub const fn new(
        max_term_multiplier: f64,
        max_literal_multiplier: f64,
        pos_eq_multiplier: f64,
        ext_style: TermWeightExtensionStyle,
        term_weight_fun: WeightFun,
        data: Data,
    ) -> Self {
        Self {
            max_term_multiplier,
            max_literal_multiplier,
            pos_eq_multiplier,
            ext_style,
            term_weight_fun,
            data,
        }
    }

    #[must_use]
    pub const fn max_term_multiplier(&self) -> f64 {
        self.max_term_multiplier
    }

    #[must_use]
    pub const fn max_literal_multiplier(&self) -> f64 {
        self.max_literal_multiplier
    }

    #[must_use]
    pub const fn pos_eq_multiplier(&self) -> f64 {
        self.pos_eq_multiplier
    }

    #[must_use]
    pub const fn ext_style(&self) -> TermWeightExtensionStyle {
        self.ext_style
    }

    #[must_use]
    pub const fn data(&self) -> &Data {
        &self.data
    }

    pub fn term_ext_weight<'term, Term>(
        &self,
        term: &'term Term,
        is_free_var: impl Fn(&Term) -> bool,
        args: impl Fn(&'term Term) -> &'term [Term],
    ) -> f64
    where
        WeightFun: Fn(&Term, &Data) -> f64,
    {
        match self.ext_style {
            TermWeightExtensionStyle::Simple => (self.term_weight_fun)(term, &self.data),
            TermWeightExtensionStyle::SubtermsSum => {
                self.term_ext_weight_sum(term, is_free_var, args)
            }
            TermWeightExtensionStyle::SubtermsMax => {
                self.term_ext_weight_max(term, is_free_var, args)
            }
        }
    }

    fn term_ext_weight_sum<'term, Term>(
        &self,
        term: &'term Term,
        is_free_var: impl Fn(&Term) -> bool,
        args: impl Fn(&'term Term) -> &'term [Term],
    ) -> f64
    where
        WeightFun: Fn(&Term, &Data) -> f64,
    {
        let mut result = 0.0;
        let mut stack = vec![term];
        while let Some(subterm) = stack.pop() {
            result += (self.term_weight_fun)(subterm, &self.data);
            if !is_free_var(subterm) {
                stack.extend(args(subterm));
            }
        }
        result
    }

    fn term_ext_weight_max<'term, Term>(
        &self,
        term: &'term Term,
        is_free_var: impl Fn(&Term) -> bool,
        args: impl Fn(&'term Term) -> &'term [Term],
    ) -> f64
    where
        WeightFun: Fn(&Term, &Data) -> f64,
    {
        let mut result = -f64::MAX;
        let mut stack = vec![term];
        while let Some(subterm) = stack.pop() {
            result = result.max((self.term_weight_fun)(subterm, &self.data));
            if !is_free_var(subterm) {
                stack.extend(args(subterm));
            }
        }
        result
    }
}

impl<Data, WeightFun> TermWeightExtension<Data, WeightFun>
where
    WeightFun: Fn(&Term, &Data) -> f64,
{
    #[must_use]
    pub fn term_weight(&self, term: &Term) -> f64 {
        match self.ext_style {
            TermWeightExtensionStyle::Simple => (self.term_weight_fun)(term, &self.data),
            TermWeightExtensionStyle::SubtermsSum => self.term_weight_sum(term),
            TermWeightExtensionStyle::SubtermsMax => self.term_weight_max(term),
        }
    }

    fn term_weight_sum(&self, term: &Term) -> f64 {
        let mut result = 0.0;
        let mut stack = vec![term.clone()];
        while let Some(subterm) = stack.pop() {
            result += (self.term_weight_fun)(&subterm, &self.data);
            if !subterm.is_free_var() {
                stack.extend(subterm.argument_clones().into_iter().flatten());
            }
        }
        result
    }

    fn term_weight_max(&self, term: &Term) -> f64 {
        let mut result = -f64::MAX;
        let mut stack = vec![term.clone()];
        while let Some(subterm) = stack.pop() {
            result = result.max((self.term_weight_fun)(&subterm, &self.data));
            if !subterm.is_free_var() {
                stack.extend(subterm.argument_clones().into_iter().flatten());
            }
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::{TermWeightExtension, TermWeightExtensionStyle};
    use std::cell::RefCell;

    #[derive(Clone, Debug, PartialEq)]
    struct TestTerm {
        id: i32,
        weight: f64,
        free_var: bool,
        args: Vec<TestTerm>,
    }

    impl TestTerm {
        fn leaf(id: i32, weight: f64) -> Self {
            Self {
                id,
                weight,
                free_var: false,
                args: Vec::new(),
            }
        }

        fn free_var(id: i32, weight: f64, args: Vec<TestTerm>) -> Self {
            Self {
                id,
                weight,
                free_var: true,
                args,
            }
        }

        fn node(id: i32, weight: f64, args: Vec<TestTerm>) -> Self {
            Self {
                id,
                weight,
                free_var: false,
                args,
            }
        }
    }

    fn is_free_var(term: &TestTerm) -> bool {
        term.free_var
    }

    fn args(term: &TestTerm) -> &[TestTerm] {
        &term.args
    }

    fn assert_f64_bits_eq(actual: f64, expected: f64) {
        assert_eq!(actual.to_bits(), expected.to_bits());
    }

    #[test]
    fn style_discriminants_match_c_enum() {
        assert_eq!(TermWeightExtensionStyle::Simple.c_value(), 0);
        assert_eq!(TermWeightExtensionStyle::SubtermsSum.c_value(), 1);
        assert_eq!(TermWeightExtensionStyle::SubtermsMax.c_value(), 2);
        assert_eq!(
            TermWeightExtensionStyle::from_c_value(0),
            Some(TermWeightExtensionStyle::Simple)
        );
        assert_eq!(
            TermWeightExtensionStyle::from_c_value(1),
            Some(TermWeightExtensionStyle::SubtermsSum)
        );
        assert_eq!(
            TermWeightExtensionStyle::from_c_value(2),
            Some(TermWeightExtensionStyle::SubtermsMax)
        );
        assert_eq!(TermWeightExtensionStyle::from_c_value(3), None);
    }

    #[test]
    fn allocation_preserves_extension_fields() {
        let extension = TermWeightExtension::new(
            1.25,
            2.5,
            3.75,
            TermWeightExtensionStyle::Simple,
            |term: &TestTerm, multiplier: &f64| term.weight * multiplier,
            2.0,
        );

        assert_f64_bits_eq(extension.max_term_multiplier(), 1.25);
        assert_f64_bits_eq(extension.max_literal_multiplier(), 2.5);
        assert_f64_bits_eq(extension.pos_eq_multiplier(), 3.75);
        assert_eq!(extension.ext_style(), TermWeightExtensionStyle::Simple);
        assert_f64_bits_eq(*extension.data(), 2.0);
    }

    #[test]
    fn simple_style_applies_weight_function_only_to_root() {
        let term = TestTerm::node(
            1,
            10.0,
            vec![TestTerm::leaf(2, 20.0), TestTerm::leaf(3, 30.0)],
        );
        let visited = RefCell::new(Vec::new());
        let extension = TermWeightExtension::new(
            0.0,
            0.0,
            0.0,
            TermWeightExtensionStyle::Simple,
            |term: &TestTerm, visited: &RefCell<Vec<i32>>| {
                visited.borrow_mut().push(term.id);
                term.weight
            },
            visited,
        );

        assert_f64_bits_eq(extension.term_ext_weight(&term, is_free_var, args), 10.0);
        assert_eq!(extension.data().borrow().as_slice(), &[1]);
    }

    #[test]
    fn sum_style_visits_subterms_with_c_stack_order_and_skips_free_var_args() {
        let term = TestTerm::node(
            1,
            1.0,
            vec![
                TestTerm::free_var(2, 2.0, vec![TestTerm::leaf(20, 100.0)]),
                TestTerm::node(3, 3.0, vec![TestTerm::leaf(4, 4.0)]),
            ],
        );
        let visited = RefCell::new(Vec::new());
        let extension = TermWeightExtension::new(
            0.0,
            0.0,
            0.0,
            TermWeightExtensionStyle::SubtermsSum,
            |term: &TestTerm, visited: &RefCell<Vec<i32>>| {
                visited.borrow_mut().push(term.id);
                term.weight
            },
            visited,
        );

        assert_f64_bits_eq(extension.term_ext_weight(&term, is_free_var, args), 10.0);
        assert_eq!(extension.data().borrow().as_slice(), &[1, 3, 4, 2]);
    }

    #[test]
    fn max_style_uses_all_reachable_subterms_and_negative_dbl_max_initial_value() {
        let term = TestTerm::node(
            1,
            -20.0,
            vec![
                TestTerm::leaf(2, -10.0),
                TestTerm::node(3, -30.0, vec![TestTerm::leaf(4, -5.0)]),
            ],
        );
        let extension = TermWeightExtension::new(
            0.0,
            0.0,
            0.0,
            TermWeightExtensionStyle::SubtermsMax,
            |term: &TestTerm, (): &()| term.weight,
            (),
        );

        assert_f64_bits_eq(extension.term_ext_weight(&term, is_free_var, args), -5.0);
    }
}
