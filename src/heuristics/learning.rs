use crate::basics::error::{Diagnostic, ErrorCode};
use crate::heuristics::prio_funs::parse_prio_fun;
use crate::heuristics::wfcb::ClausePrioFun;
use crate::inout::basicparser::{parse_filename, parse_float, parse_int};
use crate::inout::scanner::{token_pos_rep, Scanner, TokenType};
use crate::learn::indexfunctions::{get_index_type, IndexType};
use crate::learn::tsm::{get_tsm_type, TsmType};

pub const ANNOTATION_DEFAULT_SIZE: usize = 7;
pub const TSM_E_WEIGHT_COUNT: usize = ANNOTATION_DEFAULT_SIZE - 1;

#[derive(Clone, Debug, PartialEq)]
pub struct TsmParam {
    fweight: i64,
    vweight: i64,
    max_term_multiplier: f64,
    max_literal_multiplier: f64,
    pos_multiplier: f64,
    flat_clauses: bool,
    learnweight: f64,
    kb: String,
    sel_no: i64,
    set_part: f64,
    dist_part: f64,
    index_type: IndexType,
    tsm_type: TsmType,
    depth: i64,
    e_weights: [f64; TSM_E_WEIGHT_COUNT],
    eval_base: f64,
    eval_scale: f64,
}

impl TsmParam {
    #[must_use]
    #[expect(
        clippy::too_many_arguments,
        reason = "C-compatible constructor mirrors tsm_param_init"
    )]
    pub fn new(
        fweight: i64,
        vweight: i64,
        max_term_multiplier: f64,
        max_literal_multiplier: f64,
        pos_multiplier: f64,
        flat_clauses: bool,
        learnweight: f64,
        kb: String,
        sel_no: i64,
        set_part: f64,
        dist_part: f64,
        index_type: IndexType,
        tsm_type: TsmType,
        depth: i64,
        e_weights: [f64; TSM_E_WEIGHT_COUNT],
    ) -> Self {
        let (eval_base, eval_scale) = eval_normalization(e_weights);
        Self {
            fweight,
            vweight,
            max_term_multiplier,
            max_literal_multiplier,
            pos_multiplier,
            flat_clauses,
            learnweight,
            kb,
            sel_no,
            set_part,
            dist_part,
            index_type,
            tsm_type,
            depth,
            e_weights,
            eval_base,
            eval_scale,
        }
    }

    #[must_use]
    pub const fn fweight(&self) -> i64 {
        self.fweight
    }

    #[must_use]
    pub const fn vweight(&self) -> i64 {
        self.vweight
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
    pub const fn pos_multiplier(&self) -> f64 {
        self.pos_multiplier
    }

    #[must_use]
    pub const fn flat_clauses(&self) -> bool {
        self.flat_clauses
    }

    #[must_use]
    pub const fn learnweight(&self) -> f64 {
        self.learnweight
    }

    #[must_use]
    pub fn kb(&self) -> &str {
        &self.kb
    }

    #[must_use]
    pub const fn sel_no(&self) -> i64 {
        self.sel_no
    }

    #[must_use]
    pub const fn set_part(&self) -> f64 {
        self.set_part
    }

    #[must_use]
    pub const fn dist_part(&self) -> f64 {
        self.dist_part
    }

    #[must_use]
    pub const fn index_type(&self) -> IndexType {
        self.index_type
    }

    #[must_use]
    pub const fn tsm_type(&self) -> TsmType {
        self.tsm_type
    }

    #[must_use]
    pub const fn depth(&self) -> i64 {
        self.depth
    }

    #[must_use]
    pub const fn e_weights(&self) -> [f64; TSM_E_WEIGHT_COUNT] {
        self.e_weights
    }

    #[must_use]
    pub const fn eval_base(&self) -> f64 {
        self.eval_base
    }

    #[must_use]
    pub const fn eval_scale(&self) -> f64 {
        self.eval_scale
    }
}

pub fn tsm_weight_parse_params(
    scanner: &mut Scanner,
) -> Result<(ClausePrioFun, TsmParam), Diagnostic> {
    scanner.accept_tok(TokenType::OPEN_BRACKET)?;
    let prio_fun = parse_prio_fun(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let fweight = parse_int(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let vweight = parse_int(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let learnweight = parse_float(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let flat_clauses = parse_flat_or_rec(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let kb = parse_filename(scanner)?;
    let tail = parse_tsm_common_tail(scanner)?;
    scanner.accept_tok(TokenType::CLOSE_BRACKET)?;

    Ok((
        prio_fun,
        TsmParam::new(
            fweight,
            vweight,
            1.0,
            1.0,
            1.0,
            flat_clauses,
            learnweight,
            kb,
            tail.sel_no,
            tail.set_part,
            tail.dist_part,
            tail.index_type,
            tail.tsm_type,
            tail.depth,
            tail.e_weights,
        ),
    ))
}

pub fn tsmr_weight_parse_params(
    scanner: &mut Scanner,
) -> Result<(ClausePrioFun, TsmParam), Diagnostic> {
    scanner.accept_tok(TokenType::OPEN_BRACKET)?;
    let prio_fun = parse_prio_fun(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let fweight = parse_int(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let vweight = parse_int(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let max_term_multiplier = parse_float(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let max_literal_multiplier = parse_float(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let pos_multiplier = parse_float(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let learnweight = parse_float(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let flat_clauses = parse_flat_or_rec(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let kb = parse_filename(scanner)?;
    let tail = parse_tsm_common_tail(scanner)?;
    scanner.accept_tok(TokenType::CLOSE_BRACKET)?;

    Ok((
        prio_fun,
        TsmParam::new(
            fweight,
            vweight,
            max_term_multiplier,
            max_literal_multiplier,
            pos_multiplier,
            flat_clauses,
            learnweight,
            kb,
            tail.sel_no,
            tail.set_part,
            tail.dist_part,
            tail.index_type,
            tail.tsm_type,
            tail.depth,
            tail.e_weights,
        ),
    ))
}

struct TsmTail {
    sel_no: i64,
    set_part: f64,
    dist_part: f64,
    index_type: IndexType,
    tsm_type: TsmType,
    depth: i64,
    e_weights: [f64; TSM_E_WEIGHT_COUNT],
}

fn parse_tsm_common_tail(scanner: &mut Scanner) -> Result<TsmTail, Diagnostic> {
    scanner.accept_tok(TokenType::COMMA)?;
    let sel_no = parse_int(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let set_part = parse_float(scanner)?;
    if !(0.0..=1.0).contains(&set_part) {
        return Err(learning_parse_error(
            scanner,
            "You need to specify the part of the knowledge base to be used as a fraction between 0.0 and 1.0!",
        ));
    }
    scanner.accept_tok(TokenType::COMMA)?;
    let dist_part = parse_float(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let tsm_type = parse_tsm_type(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let index_type = parse_index_type(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let depth = parse_int(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let proofs_w = parse_float(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let dist_w = parse_float(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let p_simp_w = parse_float(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let f_simp_w = parse_float(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let p_gen_w = parse_float(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let f_gen_w = parse_float(scanner)?;

    Ok(TsmTail {
        sel_no,
        set_part,
        dist_part,
        index_type,
        tsm_type,
        depth,
        e_weights: [proofs_w, dist_w, p_simp_w, f_simp_w, p_gen_w, f_gen_w],
    })
}

fn parse_flat_or_rec(scanner: &mut Scanner) -> Result<bool, Diagnostic> {
    scanner.check_tok(TokenType::IDENTIFIER)?;
    let literal = scanner.current_token().literal();
    let flat_clauses = match literal.as_str() {
        "flat" => true,
        "rec" => false,
        _ => return Err(learning_parse_error(scanner, "expected rec|flat")),
    };
    scanner.next_token()?;
    Ok(flat_clauses)
}

fn parse_tsm_type(scanner: &mut Scanner) -> Result<TsmType, Diagnostic> {
    scanner.check_tok(TokenType::NAME)?;
    let literal = scanner.current_token().literal();
    let Some(tsm_type) = get_tsm_type(&literal) else {
        return Err(learning_parse_error(
            scanner,
            "No correct TSM type specified!",
        ));
    };
    if tsm_type == TsmType::NoType {
        return Err(learning_parse_error(
            scanner,
            "No correct TSM type specified!",
        ));
    }
    scanner.next_token()?;
    Ok(tsm_type)
}

fn parse_index_type(scanner: &mut Scanner) -> Result<IndexType, Diagnostic> {
    scanner.check_tok(TokenType::NAME)?;
    let literal = scanner.current_token().literal();
    let Some(index_type) = get_index_type(&literal) else {
        return Err(learning_parse_error(
            scanner,
            "No correct index type specified!",
        ));
    };
    if matches!(index_type, IndexType::NO_INDEX | IndexType::EMPTY) {
        return Err(learning_parse_error(
            scanner,
            "No correct index type specified!",
        ));
    }
    scanner.next_token()?;
    Ok(index_type)
}

fn eval_normalization(e_weights: [f64; TSM_E_WEIGHT_COUNT]) -> (f64, f64) {
    let mut pos_sum = 0.0;
    let mut neg_sum = 0.0;
    for weight in e_weights {
        if weight > 0.0 {
            pos_sum += weight;
        } else {
            neg_sum += weight;
        }
    }
    let scale = pos_sum - neg_sum;
    (neg_sum, if scale == 0.0 { 1.0 } else { scale })
}

fn learning_parse_error(scanner: &Scanner, message: &str) -> Diagnostic {
    Diagnostic::new(
        ErrorCode::SYNTAX_ERROR,
        format!("{} {message}", token_pos_rep(scanner.current_token())),
    )
}

#[cfg(test)]
mod tests {
    use super::{tsm_weight_parse_params, tsmr_weight_parse_params, ANNOTATION_DEFAULT_SIZE};
    use crate::basics::error::ErrorCode;
    use crate::clauses::clause::Clause;
    use crate::clauses::neweval::PRIO_NORMAL;
    use crate::heuristics::wfcb::ClausePrioFun;
    use crate::inout::scanner::Scanner;
    use crate::learn::indexfunctions::IndexType;
    use crate::learn::tsm::TsmType;
    use crate::terms::signature::Signature;
    use crate::terms::termbanks::TermBank;
    use crate::terms::typebanks::TypeBank;

    fn assert_close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < f64::EPSILON,
            "expected {expected}, got {actual}"
        );
    }

    fn assert_f64_array_bits(actual: [f64; 6], expected: [f64; 6]) {
        for (actual_value, expected_value) in actual.into_iter().zip(expected) {
            assert_eq!(actual_value.to_bits(), expected_value.to_bits());
        }
    }

    fn term_bank() -> TermBank {
        let mut signature = Signature::new(TypeBank::new());
        signature.insert_internal_codes().unwrap();
        TermBank::new(signature).unwrap()
    }

    fn assert_const_prio(prio_fun: ClausePrioFun) {
        let bank = term_bank();
        let clause = Clause::empty();
        assert_eq!(prio_fun(&bank, &clause), PRIO_NORMAL);
    }

    #[test]
    fn tsm_weight_parse_params_preserves_c_field_order_and_normalization() {
        let mut scanner = Scanner::from_user_string(
            "TSMWeight(ConstPrio,2,3,0.5,flat,kb/name,10,0.25,0.75,Flat,IndexDynamic,4,1.0,-2.0,3.0,0.0,5.0,-7.0) tail",
            false,
        )
        .unwrap_or_else(|err| panic!("{err}"));
        scanner.next_token().unwrap();
        let (prio_fun, param) =
            tsm_weight_parse_params(&mut scanner).unwrap_or_else(|err| panic!("{err}"));

        assert_const_prio(prio_fun);
        assert_eq!(param.fweight(), 2);
        assert_eq!(param.vweight(), 3);
        assert_close(param.max_term_multiplier(), 1.0);
        assert_close(param.max_literal_multiplier(), 1.0);
        assert_close(param.pos_multiplier(), 1.0);
        assert!(param.flat_clauses());
        assert_close(param.learnweight(), 0.5);
        assert_eq!(param.kb(), "kb/name");
        assert_eq!(param.sel_no(), 10);
        assert_close(param.set_part(), 0.25);
        assert_close(param.dist_part(), 0.75);
        assert_eq!(param.tsm_type(), TsmType::Flat);
        assert_eq!(param.index_type(), IndexType::DYNAMIC);
        assert_eq!(param.depth(), 4);
        assert_f64_array_bits(param.e_weights(), [1.0, -2.0, 3.0, 0.0, 5.0, -7.0]);
        assert_close(param.eval_base(), -9.0);
        assert_close(param.eval_scale(), 18.0);
        assert_eq!(ANNOTATION_DEFAULT_SIZE, 7);
        assert_eq!(scanner.current_token().literal(), "tail");
    }

    #[test]
    fn tsmr_weight_parse_params_reads_refined_multipliers() {
        let mut scanner = Scanner::from_user_string(
            "TSMRWeight(ConstPrio,2,3,4.0,5.0,6.0,0.5,rec,kb,10,0.25,0.75,Recursive,IndexTop,4,1.0,2.0,3.0,4.0,5.0,6.0) tail",
            false,
        )
        .unwrap_or_else(|err| panic!("{err}"));
        scanner.next_token().unwrap();
        let (prio_fun, param) =
            tsmr_weight_parse_params(&mut scanner).unwrap_or_else(|err| panic!("{err}"));

        assert_const_prio(prio_fun);
        assert!(!param.flat_clauses());
        assert_close(param.max_term_multiplier(), 4.0);
        assert_close(param.max_literal_multiplier(), 5.0);
        assert_close(param.pos_multiplier(), 6.0);
        assert_eq!(param.tsm_type(), TsmType::Recursive);
        assert_eq!(param.index_type(), IndexType::TOP);
        assert_close(param.eval_base(), 0.0);
        assert_close(param.eval_scale(), 21.0);
        assert_eq!(scanner.current_token().literal(), "tail");
    }

    #[test]
    fn tsm_weight_parse_params_rejects_c_invalid_name_and_range_cases() {
        for spec in [
            "TSMWeight(ConstPrio,2,3,0.5,bad,kb,10,0.25,0.75,Flat,IndexTop,4,1,1,1,1,1,1)",
            "TSMWeight(ConstPrio,2,3,0.5,flat,kb,10,1.25,0.75,Flat,IndexTop,4,1,1,1,1,1,1)",
            "TSMWeight(ConstPrio,2,3,0.5,flat,kb,10,0.25,0.75,NoType,IndexTop,4,1,1,1,1,1,1)",
            "TSMWeight(ConstPrio,2,3,0.5,flat,kb,10,0.25,0.75,Flat,IndexNoIndex,4,1,1,1,1,1,1)",
            "TSMWeight(ConstPrio,2,3,0.5,flat,kb,10,0.25,0.75,Flat,IndexEmpty,4,1,1,1,1,1,1)",
        ] {
            let mut scanner =
                Scanner::from_user_string(spec, false).unwrap_or_else(|err| panic!("{err}"));
            scanner.next_token().unwrap();
            let Err(error) = tsm_weight_parse_params(&mut scanner) else {
                panic!("invalid TSM parser input should fail: {spec}");
            };
            assert_eq!(error.code(), ErrorCode::SYNTAX_ERROR);
        }
    }
}
