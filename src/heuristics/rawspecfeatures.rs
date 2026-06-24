use crate::basics::error::{Diagnostic, ErrorCode};
use crate::basics::simple_stuff::{problem_type, ProblemType};
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

#[cfg(test)]
mod tests {
    use super::{
        raw_spec_features_classify_for_problem_type, raw_spec_features_format,
        raw_spec_features_parse, RawSpecFeatureCell, RAW_PARSE_CLASS_LEN,
    };
    use crate::basics::error::ErrorCode;
    use crate::basics::simple_stuff::ProblemType;
    use crate::heuristics::clausesetfeatures::SpecLimits;
    use crate::inout::scanner::Scanner;

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
