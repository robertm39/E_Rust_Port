use crate::basics::pdarrays::{PDArrayIndex, PDIntArray};
use crate::basics::pstacks::PStack;
use crate::clauses::clausesets::ClauseSet;
use crate::inout::basicparser::{parse_float, parse_int};
use crate::inout::scanner::{Scanner, TokenType};
use crate::terms::signature::Signature;
use crate::terms::termfunc::{term_depth, term_weight_compute};
use crate::terms::termtypes::{DEFAULT_FWEIGHT, DEFAULT_VWEIGHT};
use std::fmt::Write as _;

pub const FEATURE_NUMBER: usize = 15;
pub const SEL_FEATURE_WEIGHTS: [f64; FEATURE_NUMBER] = [1.0; FEATURE_NUMBER];
pub const SEL_PRED_WEIGHT: f64 = 1.0;
pub const SEL_FUNC_WEIGHT: f64 = 1.0;

#[derive(Clone, Debug, PartialEq)]
pub struct Features {
    pred_max_arity: i32,
    pred_distrib: PDIntArray,
    func_max_arity: i32,
    func_distrib: PDIntArray,
    values: [f64; FEATURE_NUMBER],
}

impl Default for Features {
    fn default() -> Self {
        Self::new()
    }
}

impl Features {
    #[must_use]
    pub fn new() -> Self {
        Self {
            pred_max_arity: -1,
            pred_distrib: PDIntArray::new_int(5, 5),
            func_max_arity: -1,
            func_distrib: PDIntArray::new_int(5, 5),
            values: [0.0; FEATURE_NUMBER],
        }
    }

    #[must_use]
    pub const fn pred_max_arity(&self) -> i32 {
        self.pred_max_arity
    }

    #[must_use]
    pub const fn func_max_arity(&self) -> i32 {
        self.func_max_arity
    }

    #[must_use]
    pub const fn values(&self) -> &[f64; FEATURE_NUMBER] {
        &self.values
    }

    #[must_use]
    pub fn value(&self, index: usize) -> Option<f64> {
        self.values.get(index).copied()
    }

    pub fn set_value(&mut self, index: usize, value: f64) -> bool {
        let Some(slot) = self.values.get_mut(index) else {
            return false;
        };
        *slot = value;
        true
    }

    #[must_use]
    pub fn pred_distribution_value(&self, arity: i32) -> i64 {
        distrib_existing_value(&self.pred_distrib, arity)
    }

    #[must_use]
    pub fn func_distribution_value(&self, arity: i32) -> i64 {
        distrib_existing_value(&self.func_distrib, arity)
    }

    pub fn assign_pred_distribution_value(&mut self, arity: i32, value: i64) -> bool {
        self.pred_distrib.assign(arity_index(arity), value)
    }

    pub fn assign_func_distribution_value(&mut self, arity: i32, value: i64) -> bool {
        self.func_distrib.assign(arity_index(arity), value)
    }

    pub fn set_pred_max_arity(&mut self, max_arity: i32) {
        self.pred_max_arity = max_arity;
    }

    pub fn set_func_max_arity(&mut self, max_arity: i32) {
        self.func_max_arity = max_arity;
    }

    #[must_use]
    pub fn print_string(&self) -> String {
        let mut result = String::new();
        result.push_str("PA: (");
        append_distribution(&mut result, &self.pred_distrib, self.pred_max_arity);
        result.push_str(")  FA: (");
        append_distribution(&mut result, &self.func_distrib, self.func_max_arity);
        result.push_str(")\n");
        let write_result = write!(&mut result, "({:.6}", self.values[0]);
        debug_assert!(write_result.is_ok());
        for value in &self.values[1..] {
            let write_result = write!(&mut result, ", {value:.6}");
            debug_assert!(write_result.is_ok());
        }
        result.push_str(")\n");
        result
    }

    pub fn parse(scanner: &mut Scanner) -> Result<Self, crate::basics::error::Diagnostic> {
        let mut handle = Self::new();

        scanner.accept_id("PA")?;
        scanner.accept_tok(TokenType::COLON)?;
        handle.pred_max_arity = parse_sig_distrib(scanner, &mut handle.pred_distrib)?;

        scanner.accept_id("FA")?;
        scanner.accept_tok(TokenType::COLON)?;
        handle.func_max_arity = parse_sig_distrib(scanner, &mut handle.func_distrib)?;

        scanner.accept_tok(TokenType::OPEN_BRACKET)?;
        handle.values[0] = parse_float(scanner)?;
        for index in 1..FEATURE_NUMBER {
            scanner.accept_tok(TokenType::COMMA)?;
            handle.values[index] = parse_float(scanner)?;
        }
        scanner.accept_tok(TokenType::CLOSE_BRACKET)?;

        Ok(handle)
    }

    /// Return C's weighted relative distance between feature vectors.
    ///
    /// # Panics
    ///
    /// Panics when fewer than `FEATURE_NUMBER` feature weights are supplied.
    #[must_use]
    pub fn distance(&mut self, other: &mut Self, pred_w: f64, func_w: f64, weights: &[f64]) -> f64 {
        num_feature_distance(self, other, pred_w, func_w, weights)
    }
}

pub fn compute_clause_set_num_features(features: &mut Features, set: &ClauseSet, sig: &Signature) {
    let mut pos_tdepth = PStack::new();
    let mut neg_tdepth = PStack::new();
    let mut pos_tsize = PStack::new();
    let mut neg_tsize = PStack::new();
    let mut pos_lits = PStack::new();
    let mut neg_lits = PStack::new();

    let mut symbol_distrib = vec![0; sig.size()];
    set.add_symbol_distribution(&mut symbol_distrib);
    features.pred_max_arity =
        sig.add_symbol_arities(&mut features.pred_distrib, true, &symbol_distrib);
    features.func_max_arity =
        sig.add_symbol_arities(&mut features.func_distrib, false, &symbol_distrib);

    features.values[0] = 0.0;
    features.values[1] = 0.0;
    features.values[2] = 0.0;

    for clause in set.iter() {
        if clause.is_unit() {
            features.values[0] += 1.0;
        } else if clause.is_horn() {
            features.values[1] += 1.0;
        } else {
            features.values[2] += 1.0;
        }

        pos_lits.push(usize_to_i64(clause.positive_literal_count()));
        neg_lits.push(usize_to_i64(clause.negative_literal_count()));

        for literal in clause.literals().as_slice() {
            if literal.is_positive() {
                pos_tsize.push(term_weight_compute(
                    literal.left(),
                    DEFAULT_VWEIGHT,
                    DEFAULT_FWEIGHT,
                ));
                pos_tsize.push(term_weight_compute(
                    literal.right(),
                    DEFAULT_VWEIGHT,
                    DEFAULT_FWEIGHT,
                ));
                pos_tdepth.push(term_depth(literal.left()));
                pos_tdepth.push(term_depth(literal.right()));
            } else {
                neg_tsize.push(term_weight_compute(
                    literal.left(),
                    DEFAULT_VWEIGHT,
                    DEFAULT_FWEIGHT,
                ));
                neg_tsize.push(term_weight_compute(
                    literal.right(),
                    DEFAULT_VWEIGHT,
                    DEFAULT_FWEIGHT,
                ));
                neg_tdepth.push(term_depth(literal.left()));
                neg_tdepth.push(term_depth(literal.right()));
            }
        }
    }

    let (average, deviation) = pos_tdepth.compute_average();
    features.values[3] = average;
    features.values[4] = deviation;
    let (average, deviation) = neg_tdepth.compute_average();
    features.values[5] = average;
    features.values[6] = deviation;
    let (average, deviation) = pos_tsize.compute_average();
    features.values[7] = average;
    features.values[8] = deviation;
    let (average, deviation) = neg_tsize.compute_average();
    features.values[9] = average;
    features.values[10] = deviation;
    let (average, deviation) = pos_lits.compute_average();
    features.values[11] = average;
    features.values[12] = deviation;
    let (average, deviation) = neg_lits.compute_average();
    features.values[13] = average;
    features.values[14] = deviation;
}

#[must_use]
pub fn num_features_print_string(features: &Features) -> String {
    features.print_string()
}

pub fn num_features_parse(
    scanner: &mut Scanner,
) -> Result<Features, crate::basics::error::Diagnostic> {
    Features::parse(scanner)
}

/// Return C's weighted relative distance between feature vectors.
///
/// # Panics
///
/// Panics when fewer than `FEATURE_NUMBER` feature weights are supplied.
#[must_use]
pub fn num_feature_distance(
    left: &mut Features,
    right: &mut Features,
    pred_w: f64,
    func_w: f64,
    weights: &[f64],
) -> f64 {
    assert!(
        weights.len() >= FEATURE_NUMBER,
        "feature distance requires FEATURE_NUMBER weights"
    );

    let mut dist = arity_distr_distance(
        &mut left.pred_distrib,
        &mut right.pred_distrib,
        left.pred_max_arity.max(right.pred_max_arity),
    );
    let mut wsq = pred_w * pred_w;
    let mut result = dist * dist * wsq;
    let mut norm = wsq;

    dist = arity_distr_distance(
        &mut left.func_distrib,
        &mut right.func_distrib,
        left.func_max_arity.max(right.func_max_arity),
    );
    wsq = func_w * func_w;
    result += dist * dist * wsq;
    norm += wsq;

    for (index, weight) in weights.iter().take(FEATURE_NUMBER).enumerate() {
        dist = relative_difference(left.values[index], right.values[index]);
        wsq = weight * weight;
        result += dist * dist * wsq;
        norm += wsq;
    }
    (result / norm).sqrt()
}

fn parse_sig_distrib(
    scanner: &mut Scanner,
    distrib: &mut PDIntArray,
) -> Result<i32, crate::basics::error::Diagnostic> {
    let mut index = -1_i32;

    scanner.accept_tok(TokenType::OPEN_BRACKET)?;
    if !scanner.test_tok(TokenType::CLOSE_BRACKET) {
        index += 1;
        distrib.assign(arity_index(index), parse_int(scanner)?);
        while !scanner.test_tok(TokenType::CLOSE_BRACKET) {
            index += 1;
            scanner.accept_tok(TokenType::COMMA)?;
            distrib.assign(arity_index(index), parse_int(scanner)?);
        }
    }
    scanner.accept_tok(TokenType::CLOSE_BRACKET)?;

    Ok(index)
}

#[allow(clippy::cast_precision_loss)]
fn arity_distr_distance(left: &mut PDIntArray, right: &mut PDIntArray, max_arity: i32) -> f64 {
    assert!(max_arity >= -1);
    if max_arity == -1 {
        return 0.0;
    }

    let mut result = 0.0;
    for arity in 0..=max_arity {
        let val1 = left.element_int(arity_index(arity)) as f64;
        let val2 = right.element_int(arity_index(arity)) as f64;
        let diff = relative_difference(val1, val2);
        result += diff * diff;
    }
    result.sqrt() / f64::from(max_arity + 1)
}

#[allow(clippy::float_cmp)]
fn relative_difference(left: f64, right: f64) -> f64 {
    if left == 0.0 && right == 0.0 {
        return 0.0;
    }
    (left - right) / (2.0 * left.abs().max(right.abs()))
}

fn append_distribution(result: &mut String, distrib: &PDIntArray, max_arity: i32) {
    let mut sep = "";
    for arity in 0..=max_arity {
        result.push_str(sep);
        let write_result = write!(result, "{}", distrib_existing_value(distrib, arity));
        debug_assert!(write_result.is_ok());
        sep = ", ";
    }
}

fn distrib_existing_value(distrib: &PDIntArray, arity: i32) -> i64 {
    distrib
        .existing_element(arity_index(arity))
        .copied()
        .unwrap_or(0)
}

fn arity_index(arity: i32) -> PDArrayIndex {
    PDArrayIndex::try_from(arity).expect("arity fits dynamic-array index")
}

fn usize_to_i64(value: usize) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

#[cfg(test)]
mod tests {
    use super::{
        compute_clause_set_num_features, num_feature_distance, num_features_parse,
        num_features_print_string, relative_difference, Features, FEATURE_NUMBER,
    };
    use crate::clauses::clause::Clause;
    use crate::clauses::clausesets::ClauseSet;
    use crate::clauses::eqn::Eqn;
    use crate::clauses::eqnlist::EqnList;
    use crate::inout::scanner::Scanner;
    use crate::terms::signature::Signature;
    use crate::terms::simpletypes::alloc_arrow_type;
    use crate::terms::termbanks::TermBank;
    use crate::terms::termtypes::{DerefType, Term};
    use crate::terms::typebanks::TypeBank;
    use std::fmt::Write as _;

    fn assert_close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 1.0e-12,
            "expected {expected}, got {actual}"
        );
    }

    fn make_scanner(source: &str) -> Scanner {
        Scanner::from_user_string(source, false).unwrap_or_else(|err| panic!("{err}"))
    }

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
        bank.insert(&term, DerefType::Never).unwrap()
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
        bank.insert(&term, DerefType::Never).unwrap()
    }

    fn literal(bank: &mut TermBank, left: &Term, right: &Term, positive: bool) -> Eqn {
        Eqn::alloc(left.clone(), right.clone(), bank, positive).unwrap()
    }

    fn clause_from(literals: Vec<Eqn>) -> Clause {
        let mut clause = Clause::alloc(EqnList::from_vec(literals));
        clause.set_weight(clause.standard_weight());
        clause
    }

    fn parsed_feature_source(values: &[f64; FEATURE_NUMBER]) -> String {
        let mut result = String::from("PA: (1, 2) FA: () (");
        let write_result = write!(&mut result, "{}", values[0]);
        debug_assert!(write_result.is_ok());
        for value in &values[1..] {
            let write_result = write!(&mut result, ", {value}");
            debug_assert!(write_result.is_ok());
        }
        result.push(')');
        result
    }

    #[test]
    fn features_parse_and_print_preserve_c_layout() {
        let values = [
            1.0, 2.0, 3.5, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0,
        ];
        let mut scanner = make_scanner(&parsed_feature_source(&values));
        let parsed = num_features_parse(&mut scanner).unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(parsed.pred_max_arity(), 1);
        assert_eq!(parsed.func_max_arity(), -1);
        assert_eq!(parsed.pred_distribution_value(0), 1);
        assert_eq!(parsed.pred_distribution_value(1), 2);
        for (actual, expected) in parsed.values().iter().zip(values) {
            assert_close(*actual, expected);
        }
        assert_eq!(
            num_features_print_string(&parsed),
            "PA: (1, 2)  FA: ()\n(1.000000, 2.000000, 3.500000, 4.000000, 5.000000, 6.000000, 7.000000, 8.000000, 9.000000, 10.000000, 11.000000, 12.000000, 13.000000, 14.000000, 15.000000)\n"
        );
    }

    #[test]
    fn feature_distance_uses_signed_relative_difference_and_weight_norm() {
        let mut left = Features::new();
        left.set_value(0, 2.0);
        left.assign_pred_distribution_value(0, 2);
        left.set_pred_max_arity(0);
        let mut right = Features::new();
        right.set_value(0, 4.0);
        right.assign_pred_distribution_value(0, 1);
        right.set_pred_max_arity(0);

        assert_close(relative_difference(2.0, 4.0), -0.25);
        let distance = num_feature_distance(
            &mut left,
            &mut right,
            2.0,
            0.0,
            &[
                1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
            ],
        );
        assert_close(distance, 0.25);
    }

    #[test]
    fn compute_clause_set_num_features_matches_clause_pass_shape() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let fa = typed_unary(&mut bank, "f", &a);
        let unit = clause_from(vec![literal(&mut bank, &a, &b, true)]);
        let horn = clause_from(vec![
            literal(&mut bank, &fa, &b, true),
            literal(&mut bank, &a, &b, false),
        ]);
        let general = clause_from(vec![
            literal(&mut bank, &a, &fa, true),
            literal(&mut bank, &b, &fa, true),
        ]);
        let set = ClauseSet::from_clauses([unit, horn, general]);

        let mut features = Features::new();
        compute_clause_set_num_features(&mut features, &set, bank.signature());

        assert_close(features.value(0).unwrap(), 1.0);
        assert_close(features.value(1).unwrap(), 1.0);
        assert_close(features.value(2).unwrap(), 1.0);
        assert_close(features.value(11).unwrap(), 4.0 / 3.0);
        assert_close(features.value(13).unwrap(), 1.0 / 3.0);
        assert_eq!(features.pred_max_arity(), -1);
        assert_eq!(features.func_max_arity(), 1);
        assert_eq!(features.func_distribution_value(0), 2);
        assert_eq!(features.func_distribution_value(1), 1);
    }
}
