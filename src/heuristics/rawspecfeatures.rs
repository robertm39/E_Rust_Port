use crate::basics::error::{Diagnostic, ErrorCode};
use crate::basics::simple_stuff::{problem_type, ProblemType};
use crate::clauses::formulasets::formula_set_definition_statistics;
use crate::clauses::proofstate::ProofState;
use crate::heuristics::clausesetfeatures::SpecLimits;
use crate::inout::basicparser::{parse_bool, parse_float, parse_int, parse_plain_filename};
use crate::inout::scanner::{Scanner, TokenType};

pub const RAW_CLASS_SIZE: usize = 16;
pub const RAW_CLASS_LEN: usize = RAW_CLASS_SIZE - 1;
pub const RAW_PARSE_CLASS_LEN: usize = 14;
pub const RAW_DEFAULT_MASK: &str = "aaaaaaaaaaaaa";

#[derive(Clone, Debug, Default, PartialEq)]
pub struct RawSpecFeatureCell {
    pub sentence_no: i64,
    pub term_size: i64,
    pub sig_size: i32,
    pub pred_size: i32,
    pub predc_size: i32,
    pub fun_size: i32,
    pub func_size: i32,
    pub conjecture_count: i64,
    pub hypothesis_count: i64,
    pub num_lambdas: i32,
    pub has_choice_sym: bool,
    pub num_of_definitions: i32,
    pub perc_of_form_defs: f64,
    pub order: i32,
    pub conj_order: i32,
    pub app_var_lits: bool,
    pub class: String,
}

pub fn raw_spec_features_compute(features: &mut RawSpecFeatureCell, state: &ProofState) {
    let signature = state.terms().signature();
    let axioms = state.axioms();
    let f_axioms = state.f_axioms();
    let f_ax_archive = state.f_ax_archive();
    let raw_formula_features = state.raw_formula_features();
    features.sentence_no = axioms.members() + f_axioms.cardinality()
        - raw_formula_features.lowered_clause_no
        + raw_formula_features.sentence_no;
    features.term_size = axioms.standard_weight() + f_axioms.standard_weight()
        - raw_formula_features.lowered_clause_term_size
        + raw_formula_features.term_size;
    features.hypothesis_count = 0;
    features.conjecture_count = axioms.count_conjectures(&mut features.hypothesis_count);
    features.conjecture_count += f_axioms.count_conjectures(&mut features.hypothesis_count);
    features.conjecture_count = features.conjecture_count
        - raw_formula_features.lowered_conjecture_count
        + raw_formula_features.conjecture_count;
    features.hypothesis_count = features.hypothesis_count
        - raw_formula_features.lowered_hypothesis_count
        + raw_formula_features.hypothesis_count;

    features.sig_size = signature.count_symbols(true) + signature.count_symbols(false);
    features.predc_size = signature.count_arity_symbols(0, true);
    features.func_size = signature.count_arity_symbols(0, false);
    features.pred_size = signature.count_symbols(true) - features.predc_size;
    features.fun_size = signature.count_symbols(false) - features.func_size;
    features.has_choice_sym = signature.has_choice_sym();

    let formula_order = f_axioms
        .iter()
        .map(|formula| usize_to_i32_saturating(formula.conjecture_order(signature)))
        .max()
        .unwrap_or(0);
    features.order = 1.max(formula_order).max(raw_formula_features.order);
    features.conj_order = 1
        .max(usize_to_i32_saturating(
            f_axioms.conjecture_order(signature),
        ))
        .max(raw_formula_features.conj_order);

    let definition_statistics =
        formula_set_definition_statistics(f_axioms, f_ax_archive, state.terms());
    features.num_of_definitions = definition_statistics.num_defs;
    features.perc_of_form_defs = definition_statistics.percentage_form_defs;
    features.num_lambdas = definition_statistics
        .num_lams
        .saturating_add(raw_formula_features.num_lambdas);
    features.app_var_lits =
        definition_statistics.has_app_var_lits || raw_formula_features.app_var_lits;
    features.class.clear();
}

pub fn raw_spec_features_classify(
    features: &mut RawSpecFeatureCell,
    limits: &SpecLimits,
    pattern: Option<&str>,
) {
    raw_spec_features_classify_for_problem_type(features, limits, pattern, problem_type());
}

pub fn raw_spec_features_classify_for_problem_type(
    features: &mut RawSpecFeatureCell,
    limits: &SpecLimits,
    pattern: Option<&str>,
    problem_type: ProblemType,
) {
    let mut class = [0_u8; RAW_CLASS_LEN];

    class[0] = if problem_type == ProblemType::HigherOrder {
        b'H'
    } else {
        b'F'
    };
    class[1] = raw_classify_i64(
        features.sentence_no,
        limits.ax_some_limit,
        limits.ax_many_limit,
    );
    class[2] = raw_classify_i64(
        features.term_size,
        limits.term_medium_limit,
        limits.term_large_limit,
    );
    class[3] = raw_classify_i32(
        features.sig_size,
        limits.symbols_medium_limit,
        limits.symbols_large_limit,
    );
    class[4] = raw_classify_i32(
        features.pred_size,
        limits.pred_medium_limit,
        limits.pred_large_limit,
    );
    class[5] = raw_classify_i32(
        features.predc_size,
        limits.predc_medium_limit,
        limits.predc_large_limit,
    );
    class[6] = raw_classify_i32(
        features.fun_size,
        limits.fun_medium_limit,
        limits.fun_large_limit,
    );
    class[7] = raw_classify_i32(
        features.func_size,
        limits.func_medium_limit,
        limits.func_large_limit,
    );
    class[8] = raw_classify_i32(
        features.num_of_definitions,
        limits.num_of_defs_medium_limit,
        limits.num_of_defs_large_limit,
    );
    class[9] = raw_classify_f64(
        features.perc_of_form_defs,
        limits.perc_form_defs_medium_limit,
        limits.perc_form_defs_large_limit,
    );
    class[10] = raw_classify_i32(
        features.num_lambdas,
        limits.num_of_lams_medium_limit,
        limits.num_of_lams_large_limit,
    );
    class[11] = if features.has_choice_sym { b'C' } else { b'N' };
    class[12] = match features.order {
        1 => b'F',
        2 => b'S',
        _ => b'H',
    };
    class[13] = match features.conj_order {
        0 => b'N',
        1 => b'F',
        2 => b'S',
        _ => b'H',
    };
    class[14] = if features.app_var_lits { b'A' } else { b'N' };

    if let Some(pattern) = pattern {
        for (index, mask) in pattern.bytes().take(RAW_CLASS_LEN).enumerate() {
            if mask == b'-' {
                class[index] = b'-';
            }
        }
    }

    features.class = String::from_utf8_lossy(&class).into_owned();
}

pub fn raw_spec_features_parse(
    scanner: &mut Scanner,
    features: &mut RawSpecFeatureCell,
) -> Result<(), Diagnostic> {
    scanner.accept_tok(TokenType::OPEN_BRACKET)?;
    features.sentence_no = parse_int(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    features.term_size = parse_int(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    features.sig_size = parse_i32(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    features.pred_size = parse_i32(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    features.predc_size = parse_i32(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    features.fun_size = parse_i32(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    features.func_size = parse_i32(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    features.num_of_definitions = parse_i32(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    features.perc_of_form_defs = parse_float(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    features.num_lambdas = parse_i32(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    features.has_choice_sym = parse_bool(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    features.order = parse_i32(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    features.conj_order = parse_i32(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    features.app_var_lits = parse_bool(scanner)?;
    scanner.accept_tok(TokenType::CLOSE_BRACKET)?;
    scanner.accept_tok(TokenType::COLON)?;

    let class = parse_plain_filename(scanner)?;
    if class.len() != RAW_PARSE_CLASS_LEN {
        return Err(Diagnostic::new(
            ErrorCode::SYNTAX_ERROR,
            "Raw class name must have 10 characters",
        ));
    }
    features.class = class;
    Ok(())
}

#[must_use]
pub fn raw_spec_features_format(features: &RawSpecFeatureCell) -> String {
    format!(
        "({:7}, {:7}, {:6}, {:6}, {:6}, {:6}, {:6}, {:6}, {:.3}, {}, {}, {}, {} ) : {}",
        features.sentence_no,
        features.term_size,
        features.sig_size,
        features.pred_size,
        features.predc_size,
        features.fun_size,
        features.func_size,
        features.num_of_definitions,
        features.perc_of_form_defs,
        features.num_lambdas,
        features.order,
        features.conj_order,
        bool_as_c_int(features.app_var_lits),
        features.class
    )
}

fn raw_classify_i64(value: i64, some: i64, many: i64) -> u8 {
    if value < some {
        b'S'
    } else if value < many {
        b'M'
    } else {
        b'L'
    }
}

fn raw_classify_i32(value: i32, some: i32, many: i32) -> u8 {
    if value < some {
        b'S'
    } else if value < many {
        b'M'
    } else {
        b'L'
    }
}

fn raw_classify_f64(value: f64, some: f64, many: f64) -> u8 {
    if value < some {
        b'S'
    } else if value < many {
        b'M'
    } else {
        b'L'
    }
}

fn parse_i32(scanner: &mut Scanner) -> Result<i32, Diagnostic> {
    let value = parse_int(scanner)?;
    i32::try_from(value)
        .map_err(|_| Diagnostic::new(ErrorCode::SYNTAX_ERROR, "Integer out of int range"))
}

const fn bool_as_c_int(value: bool) -> i32 {
    if value {
        1
    } else {
        0
    }
}

fn usize_to_i32_saturating(value: usize) -> i32 {
    i32::try_from(value).unwrap_or(i32::MAX)
}

#[cfg(test)]
mod tests {
    use super::{
        raw_spec_features_classify_for_problem_type, raw_spec_features_compute,
        raw_spec_features_format, raw_spec_features_parse, RawSpecFeatureCell, RAW_PARSE_CLASS_LEN,
    };
    use crate::basics::error::ErrorCode;
    use crate::basics::simple_stuff::ProblemType;
    use crate::clauses::clause::clause_parse;
    use crate::clauses::clause_props::{CP_IS_LAMBDA_DEF, CP_TYPE_CONJECTURE, CP_TYPE_HYPOTHESIS};
    use crate::clauses::formulasets::{formula_set_definition_statistics, WrappedFormula};
    use crate::clauses::proofstate::ProofState;
    use crate::heuristics::clausesetfeatures::SpecLimits;
    use crate::inout::scanner::{IoFormat, Scanner};
    use crate::terms::signature::{FP_IGNORE_PROPS, SIG_PHONY_APP_CODE};
    use crate::terms::simpletypes::{alloc_arrow_type, Type};
    use crate::terms::termbanks::TermBank;
    use crate::terms::termtypes::Term;

    fn insert_tstp_clause(state: &mut ProofState, input: &str) {
        let mut scanner = Scanner::from_user_string(input, false).unwrap();
        scanner.set_format(IoFormat::Tstp);
        let clause =
            clause_parse(&mut scanner, state.terms_mut(), ProblemType::FirstOrder).unwrap();
        state.axioms_mut().insert(clause);
    }

    fn typed_const_with_type(bank: &mut TermBank, name: &str, type_: &Type) -> Term {
        let f_code = bank.signature_mut().insert_id(name, 0, false);
        bank.signature_mut()
            .declare_final_type(f_code, type_.clone())
            .unwrap();
        bank.create_const_term(f_code).unwrap()
    }

    fn typed_const(bank: &mut TermBank, name: &str) -> Term {
        let type_ = bank.signature().type_bank().default_type();
        typed_const_with_type(bank, name, &type_)
    }

    fn typed_var(bank: &TermBank, code: i64) -> Term {
        let type_ = bank.signature().type_bank().default_type();
        bank.vars().var_assert_alloc(code, &type_)
    }

    fn typed_predicate_const(bank: &mut TermBank, name: &str) -> Term {
        let type_ = bank.signature().type_bank().bool_type();
        let f_code = bank.signature_mut().insert_id(name, 0, false);
        bank.signature_mut()
            .declare_final_type(f_code, type_)
            .unwrap();
        bank.signature_mut().declare_is_predicate(f_code).unwrap();
        bank.create_const_term(f_code).unwrap()
    }

    fn typed_unary_predicate(bank: &mut TermBank, name: &str, arg: &Term) -> Term {
        let arg_type = arg.type_().expect("predicate argument must have a type");
        let bool_type = bank.signature().type_bank().bool_type();
        let predicate_type = bank
            .signature_mut()
            .type_bank_mut()
            .insert_type_shared(alloc_arrow_type(vec![arg_type, bool_type.clone()]));
        let f_code = bank.signature_mut().insert_id(name, 1, false);
        bank.signature_mut()
            .declare_final_type(f_code, predicate_type)
            .unwrap();
        bank.signature_mut().declare_is_predicate(f_code).unwrap();
        let term = Term::top_alloc(f_code, 1);
        term.set_type(Some(bool_type));
        term.set_argument(0, arg.clone());
        bank.term_top_insert(term).unwrap()
    }

    fn bool_binary_with_code(bank: &mut TermBank, f_code: i64, left: &Term, right: &Term) -> Term {
        let type_ = bank.signature().type_bank().bool_type();
        let term = Term::top_alloc(f_code, 2);
        term.set_type(Some(type_));
        term.set_argument(0, left.clone());
        term.set_argument(1, right.clone());
        bank.term_top_insert(term).unwrap()
    }

    fn phony_app(bank: &mut TermBank, head: &Term, arg: &Term) -> Term {
        let type_ = bank.signature().type_bank().default_type();
        let term = Term::top_alloc(SIG_PHONY_APP_CODE, 2);
        term.set_type(Some(type_));
        term.set_argument(0, head.clone());
        term.set_argument(1, arg.clone());
        bank.term_top_insert(term).unwrap()
    }

    fn usize_to_i32_for_test(value: usize) -> i32 {
        i32::try_from(value).unwrap_or(i32::MAX)
    }

    fn small_limits() -> SpecLimits {
        SpecLimits {
            ax_some_limit: 10,
            ax_many_limit: 20,
            term_medium_limit: 10,
            term_large_limit: 20,
            symbols_medium_limit: 10,
            symbols_large_limit: 20,
            pred_medium_limit: 10,
            pred_large_limit: 20,
            predc_medium_limit: 10,
            predc_large_limit: 20,
            fun_medium_limit: 10,
            fun_large_limit: 20,
            func_medium_limit: 10,
            func_large_limit: 20,
            num_of_defs_medium_limit: 10,
            num_of_defs_large_limit: 20,
            perc_form_defs_medium_limit: 0.2,
            perc_form_defs_large_limit: 0.5,
            num_of_lams_medium_limit: 10,
            num_of_lams_large_limit: 20,
            ..SpecLimits::default_auto()
        }
    }

    #[test]
    fn compute_fills_clause_side_proof_state_features() {
        let mut state = ProofState::new(FP_IGNORE_PROPS).unwrap();
        insert_tstp_clause(&mut state, "cnf(hyp, hypothesis, (p(f(a)))).");
        insert_tstp_clause(&mut state, "cnf(goal, negated_conjecture, (~q(a))).");
        let expected_weight = state.axioms().standard_weight();
        let signature = state.terms().signature();
        let expected_signature_counts = (
            signature.count_symbols(true) + signature.count_symbols(false),
            signature.count_symbols(true) - signature.count_arity_symbols(0, true),
            signature.count_arity_symbols(0, true),
            signature.count_symbols(false) - signature.count_arity_symbols(0, false),
            signature.count_arity_symbols(0, false),
        );
        let mut features = RawSpecFeatureCell {
            class: "stale".to_owned(),
            num_of_definitions: 99,
            perc_of_form_defs: 0.5,
            num_lambdas: 7,
            app_var_lits: true,
            ..RawSpecFeatureCell::default()
        };

        raw_spec_features_compute(&mut features, &state);

        assert_eq!(features.sentence_no, 2);
        assert_eq!(features.term_size, expected_weight);
        assert_eq!(features.hypothesis_count, 1);
        assert_eq!(features.conjecture_count, 1);
        assert_eq!(
            (
                features.sig_size,
                features.pred_size,
                features.predc_size,
                features.fun_size,
                features.func_size,
            ),
            expected_signature_counts
        );
        assert!(!features.has_choice_sym);
        assert_eq!(features.order, 1);
        assert_eq!(features.conj_order, 1);
        assert_eq!(features.num_of_definitions, 0);
        assert!(features.perc_of_form_defs.abs() < f64::EPSILON);
        assert_eq!(features.num_lambdas, 0);
        assert!(!features.app_var_lits);
        assert!(features.class.is_empty());
    }

    #[test]
    fn compute_includes_owned_formula_sets() {
        let mut state = ProofState::new(FP_IGNORE_PROPS).unwrap();
        insert_tstp_clause(&mut state, "cnf(hyp, hypothesis, (p(a))).");

        let conjecture_formula = {
            let bank = state.terms_mut();
            let default_type = bank.signature().type_bank().default_type();
            let unary_type = bank
                .signature_mut()
                .type_bank_mut()
                .insert_type_shared(alloc_arrow_type(vec![default_type.clone(), default_type]));
            let higher_order_arg = typed_const_with_type(bank, "rawspec_ho_arg", &unary_type);
            let atom = typed_unary_predicate(bank, "rawspec_ho_pred", &higher_order_arg);
            let mut formula = WrappedFormula::wt_formula_alloc(atom);
            formula.set_tptp_type(CP_TYPE_CONJECTURE);
            formula
        };
        let hypothesis_formula = {
            let atom = typed_predicate_const(state.terms_mut(), "rawspec_formula_hyp");
            let mut formula = WrappedFormula::wt_formula_alloc(atom);
            formula.set_tptp_type(CP_TYPE_HYPOTHESIS);
            formula
        };
        let app_var_formula = {
            let bank = state.terms_mut();
            let eqn_code = bank.signature_mut().get_eqn_code(true);
            let app_head = typed_var(bank, -101);
            let app_arg = typed_const(bank, "rawspec_app_arg");
            let app_var = phony_app(bank, &app_head, &app_arg);
            let true_term = bank.true_term().clone();
            WrappedFormula::wt_formula_alloc(bool_binary_with_code(
                bank, eqn_code, &app_var, &true_term,
            ))
        };
        let definition_formula = {
            let bank = state.terms_mut();
            let eqn_code = bank.signature_mut().get_eqn_code(true);
            let predicate_head = typed_predicate_const(bank, "rawspec_definition_head");
            let true_term = bank.true_term().clone();
            let mut formula = WrappedFormula::wt_formula_alloc(bool_binary_with_code(
                bank,
                eqn_code,
                &predicate_head,
                &true_term,
            ));
            formula.set_prop(CP_IS_LAMBDA_DEF);
            formula
        };
        state.f_axioms_mut().insert(conjecture_formula);
        state.f_axioms_mut().insert(hypothesis_formula);
        state.f_axioms_mut().insert(app_var_formula);
        state.f_ax_archive_mut().insert(definition_formula);

        let expected_sentence_no = state.axioms().members() + state.f_axioms().cardinality();
        let expected_term_size =
            state.axioms().standard_weight() + state.f_axioms().standard_weight();
        let mut expected_hypotheses = 0;
        let expected_conjectures = state.axioms().count_conjectures(&mut expected_hypotheses)
            + state.f_axioms().count_conjectures(&mut expected_hypotheses);
        let expected_order = 1.max(
            state
                .f_axioms()
                .iter()
                .map(|formula| {
                    usize_to_i32_for_test(formula.conjecture_order(state.terms().signature()))
                })
                .max()
                .unwrap_or(0),
        );
        let expected_conj_order = 1.max(usize_to_i32_for_test(
            state.f_axioms().conjecture_order(state.terms().signature()),
        ));
        let expected_definition_stats = formula_set_definition_statistics(
            state.f_axioms(),
            state.f_ax_archive(),
            state.terms(),
        );
        let mut features = RawSpecFeatureCell {
            class: "stale".to_owned(),
            num_of_definitions: -1,
            perc_of_form_defs: -1.0,
            num_lambdas: -1,
            ..RawSpecFeatureCell::default()
        };

        raw_spec_features_compute(&mut features, &state);

        assert_eq!(features.sentence_no, expected_sentence_no);
        assert_eq!(features.term_size, expected_term_size);
        assert_eq!(features.conjecture_count, expected_conjectures);
        assert_eq!(features.hypothesis_count, expected_hypotheses);
        assert_eq!(features.order, expected_order);
        assert!(features.order > 1);
        assert_eq!(features.conj_order, expected_conj_order);
        assert_eq!(
            features.num_of_definitions,
            expected_definition_stats.num_defs
        );
        assert!((features.perc_of_form_defs - 1.0).abs() < f64::EPSILON);
        assert_eq!(features.num_lambdas, expected_definition_stats.num_lams);
        assert!(features.app_var_lits);
        assert!(features.class.is_empty());
    }

    #[test]
    fn classification_matches_c_threshold_boundaries_and_symbols() {
        let mut features = RawSpecFeatureCell {
            sentence_no: 9,
            term_size: 10,
            sig_size: 20,
            pred_size: 0,
            predc_size: 11,
            fun_size: 21,
            func_size: 9,
            num_of_definitions: 10,
            perc_of_form_defs: 0.2,
            num_lambdas: 1,
            has_choice_sym: true,
            order: 2,
            conj_order: 0,
            app_var_lits: true,
            ..RawSpecFeatureCell::default()
        };

        raw_spec_features_classify_for_problem_type(
            &mut features,
            &small_limits(),
            None,
            ProblemType::HigherOrder,
        );

        assert_eq!(features.class, "HSMLSMLSMMSCSNA");
    }

    #[test]
    fn classification_applies_dash_mask_only_to_existing_class_positions() {
        let mut features = RawSpecFeatureCell {
            sentence_no: 30,
            term_size: 30,
            sig_size: 30,
            pred_size: 30,
            predc_size: 30,
            fun_size: 30,
            func_size: 30,
            num_of_definitions: 30,
            perc_of_form_defs: 0.9,
            num_lambdas: 30,
            order: 3,
            conj_order: 3,
            ..RawSpecFeatureCell::default()
        };

        raw_spec_features_classify_for_problem_type(
            &mut features,
            &small_limits(),
            Some("--a-------------------------"),
            ProblemType::FirstOrder,
        );

        assert_eq!(features.class, "--L------------");
    }

    #[test]
    fn classification_treats_uninitialized_problem_type_as_first_order() {
        let mut features = RawSpecFeatureCell::default();

        raw_spec_features_classify_for_problem_type(
            &mut features,
            &small_limits(),
            None,
            ProblemType::NotInitialized,
        );

        assert!(features.class.starts_with('F'));
    }

    #[test]
    fn parse_reads_c_field_order_and_fourteen_byte_class() {
        let class = "FSSMMLLCCSSNAA";
        assert_eq!(class.len(), RAW_PARSE_CLASS_LEN);
        let mut scanner = Scanner::from_user_string(
            "(1, 2, 3, 4, 5, 6, 7, 8, 0.125, 9, true, 2, 0, false): FSSMMLLCCSSNAA rest",
            false,
        )
        .unwrap();
        let mut features = RawSpecFeatureCell::default();

        raw_spec_features_parse(&mut scanner, &mut features).unwrap();

        assert_eq!(features.sentence_no, 1);
        assert_eq!(features.term_size, 2);
        assert_eq!(features.sig_size, 3);
        assert_eq!(features.pred_size, 4);
        assert_eq!(features.predc_size, 5);
        assert_eq!(features.fun_size, 6);
        assert_eq!(features.func_size, 7);
        assert_eq!(features.num_of_definitions, 8);
        assert!((features.perc_of_form_defs - 0.125).abs() < f64::EPSILON);
        assert_eq!(features.num_lambdas, 9);
        assert!(features.has_choice_sym);
        assert_eq!(features.order, 2);
        assert_eq!(features.conj_order, 0);
        assert!(!features.app_var_lits);
        assert_eq!(features.class, class);
    }

    #[test]
    fn parse_rejects_class_lengths_with_c_error_text() {
        let mut scanner = Scanner::from_user_string(
            "(1, 2, 3, 4, 5, 6, 7, 8, 0.125, 9, false, 1, 1, false): FSSMMLLCCSSNAAX",
            false,
        )
        .unwrap();
        let mut features = RawSpecFeatureCell::default();

        let error = raw_spec_features_parse(&mut scanner, &mut features).unwrap_err();

        assert_eq!(error.code(), ErrorCode::SYNTAX_ERROR);
        assert_eq!(error.message(), "Raw class name must have 10 characters");
    }

    #[test]
    fn format_matches_c_print_shape_and_omits_choice_symbol_flag() {
        let features = RawSpecFeatureCell {
            sentence_no: 1,
            term_size: 23,
            sig_size: 4,
            pred_size: 5,
            predc_size: 6,
            fun_size: 7,
            func_size: 8,
            num_of_definitions: 9,
            perc_of_form_defs: 0.125,
            num_lambdas: 10,
            has_choice_sym: true,
            order: 2,
            conj_order: 0,
            app_var_lits: true,
            class: "FSSMMLLCCSSNAA".to_string(),
            ..RawSpecFeatureCell::default()
        };

        assert_eq!(
            raw_spec_features_format(&features),
            "(      1,      23,      4,      5,      6,      7,      8,      9, 0.125, 10, 2, 0, 1 ) : FSSMMLLCCSSNAA"
        );
    }
}
