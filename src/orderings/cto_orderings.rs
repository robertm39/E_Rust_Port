//! Generic ordering parser helpers from `cto_orderings`.

use crate::basics::error::{Diagnostic, ErrorCode};
use crate::basics::partial_orderings::CompareResult;
use crate::basics::pstacks::PStackPointer;
use crate::inout::scanner::{token_pos_rep, Scanner, Token, TokenType};
use crate::orderings::ocb::{OrderControlBlock, W_DEFAULT_WEIGHT};
use crate::terms::signature::Signature;

/// Parse `<`, `>`, or `=` into a comparison relation.
pub fn compare_symbol_parse(scanner: &mut Scanner) -> Result<CompareResult, Diagnostic> {
    scanner.check_tok(TokenType::LESSER_SIGN | TokenType::GREATER_SIGN | TokenType::EQUAL_SIGN)?;
    let result = if scanner.test_tok(TokenType::LESSER_SIGN) {
        CompareResult::Lesser
    } else if scanner.test_tok(TokenType::GREATER_SIGN) {
        CompareResult::Greater
    } else {
        CompareResult::Equal
    };
    scanner.next_token()?;
    Ok(result)
}

/// Parse one symbol comparison chain and insert its constraints into `ocb`.
///
/// # Panics
///
/// Panics if the OCB has no precedence matrix, or if a parsed symbol is outside
/// the OCB's saved signature range.
pub fn symbol_comparison_chain_parse(
    scanner: &mut Scanner,
    signature: &Signature,
    ocb: &mut OrderControlBlock,
) -> Result<PStackPointer, Diagnostic> {
    let mut left_token = scanner.current_token().clone();
    let mut left = signature.parse_known_operator(scanner)?;
    let mut ocb_state = ocb.precedence_state();

    while scanner.test_tok(TokenType::LESSER_SIGN | TokenType::GREATER_SIGN | TokenType::EQUAL_SIGN)
    {
        let relation = compare_symbol_parse(scanner)?;
        let right_token = scanner.current_token().clone();
        let right = signature.parse_known_operator(scanner)?;

        ocb_state = ocb.precedence_add_tuple(signature, left, right, relation);
        if ocb_state == 0 {
            return Err(precedence_conflict_error(&left_token));
        }

        left_token = right_token;
        left = right;
    }

    Ok(ocb_state)
}

/// Parse a comma-separated precedence-chain list into a matrix-backed OCB.
///
/// # Panics
///
/// Panics if `ocb` was allocated for a signature size different from
/// `signature.f_count()`, or if this OCB has no precedence matrix.
pub fn precedence_parse(
    scanner: &mut Scanner,
    signature: &Signature,
    ocb: &mut OrderControlBlock,
) -> Result<PStackPointer, Diagnostic> {
    assert_eq!(
        ocb.sig_size,
        signature.f_count(),
        "predefined precedence parsing requires a current OCB signature snapshot"
    );
    assert!(
        ocb.precedence.is_some(),
        "predefined precedence parsing requires a precedence matrix"
    );

    let mut result = ocb.precedence_state();
    if scanner.test_tok(TokenType::IDENTIFIER) {
        result = symbol_comparison_chain_parse(scanner, signature, ocb)?;
        while scanner.test_tok(TokenType::COMMA) {
            scanner.accept_tok(TokenType::COMMA)?;
            result = symbol_comparison_chain_parse(scanner, signature, ocb)?;
        }
    }
    Ok(result)
}

/// Parse one `symbol:weight` assignment into a weighted OCB.
///
/// # Panics
///
/// Panics if this OCB has no function-weight vector, or if the parsed symbol is
/// outside the OCB's saved signature range.
pub fn symbol_weight_parse(
    scanner: &mut Scanner,
    signature: &Signature,
    ocb: &mut OrderControlBlock,
) -> Result<(), Diagnostic> {
    let symbol = signature.parse_known_operator(scanner)?;
    scanner.accept_tok(TokenType::COLON)?;
    scanner.check_tok(TokenType::POS_INT)?;
    let weight = i64::try_from(scanner.current_token().numval()).map_err(|_| {
        Diagnostic::new(
            ErrorCode::SYNTAX_ERROR,
            format!(
                "{} weight does not fit signed long",
                token_pos_rep(scanner.current_token())
            ),
        )
    })?;
    scanner.next_token()?;
    ocb.set_fun_weight(
        symbol,
        weight
            .checked_mul(W_DEFAULT_WEIGHT)
            .ok_or_else(|| weight_overflow_error(scanner))?,
    );
    Ok(())
}

/// Parse a comma-separated list of `symbol:weight` assignments.
///
/// # Panics
///
/// Panics if `ocb` was allocated for a signature size different from
/// `signature.f_count()`, or if this OCB has no function-weight vector.
pub fn weights_parse(
    scanner: &mut Scanner,
    signature: &Signature,
    ocb: &mut OrderControlBlock,
) -> Result<i64, Diagnostic> {
    assert_eq!(
        ocb.sig_size,
        signature.f_count(),
        "predefined weight parsing requires a current OCB signature snapshot"
    );

    let mut result = 0;
    if scanner.test_tok(TokenType::IDENTIFIER) {
        symbol_weight_parse(scanner, signature, ocb)?;
        result += 1;
        while scanner.test_tok(TokenType::COMMA) {
            scanner.accept_tok(TokenType::COMMA)?;
            symbol_weight_parse(scanner, signature, ocb)?;
            result += 1;
        }
    }
    Ok(result)
}

fn precedence_conflict_error(token: &Token) -> Diagnostic {
    Diagnostic::new(
        ErrorCode::INPUT_SEMANTIC_ERROR,
        format!(
            "{} Precedence incompatible with previous ordering",
            token_pos_rep(token)
        ),
    )
}

fn weight_overflow_error(scanner: &Scanner) -> Diagnostic {
    Diagnostic::new(
        ErrorCode::SYNTAX_ERROR,
        format!(
            "{} weight overflows signed long",
            token_pos_rep(scanner.current_token())
        ),
    )
}

#[cfg(test)]
mod tests {
    use super::{
        compare_symbol_parse, precedence_parse, symbol_comparison_chain_parse, weights_parse,
    };
    use crate::basics::error::ErrorCode;
    use crate::basics::partial_orderings::{CompareResult, HoOrderKind};
    use crate::heuristics::to_params::TermOrdering;
    use crate::inout::scanner::Scanner;
    use crate::orderings::ocb::OrderControlBlock;
    use crate::terms::signature::Signature;
    use crate::terms::typebanks::TypeBank;

    fn signature() -> Signature {
        let mut signature = Signature::new(TypeBank::new());
        signature
            .insert_internal_codes()
            .unwrap_or_else(|err| panic!("{err}"));
        signature.insert_id("a", 0, false);
        signature.insert_id("b", 0, false);
        signature.insert_id("c", 0, false);
        signature
    }

    fn scanner(input: &str) -> Scanner {
        Scanner::from_option_string(input, true).unwrap_or_else(|err| panic!("{err}"))
    }

    #[test]
    fn comparison_symbol_parse_matches_c_tokens() {
        let mut lesser = scanner("<");
        let mut greater = scanner(">");
        let mut equal = scanner("=");

        assert_eq!(
            compare_symbol_parse(&mut lesser).unwrap(),
            CompareResult::Lesser
        );
        assert_eq!(
            compare_symbol_parse(&mut greater).unwrap(),
            CompareResult::Greater
        );
        assert_eq!(
            compare_symbol_parse(&mut equal).unwrap(),
            CompareResult::Equal
        );
    }

    #[test]
    fn precedence_parse_accepts_comma_separated_chains() {
        let signature = signature();
        let a = signature.find_f_code("a");
        let b = signature.find_f_code("b");
        let c = signature.find_f_code("c");
        let mut ocb =
            OrderControlBlock::alloc(TermOrdering::Lpo, false, &signature, HoOrderKind::LfhoOrder);
        let mut first_scan = scanner("a > b = c");

        assert_eq!(
            precedence_parse(&mut first_scan, &signature, &mut ocb),
            Ok(1)
        );
        assert_eq!(ocb.fun_compare(&signature, a, b), CompareResult::Greater);
        assert_eq!(ocb.fun_compare(&signature, b, c), CompareResult::Equal);
        assert_eq!(ocb.fun_compare(&signature, a, c), CompareResult::Greater);

        let mut second = scanner("a > b, b > c");
        let mut second_ocb =
            OrderControlBlock::alloc(TermOrdering::Lpo, false, &signature, HoOrderKind::LfhoOrder);
        assert_eq!(
            precedence_parse(&mut second, &signature, &mut second_ocb),
            Ok(1)
        );
        assert_eq!(
            second_ocb.fun_compare(&signature, a, c),
            CompareResult::Greater
        );
    }

    #[test]
    fn precedence_parse_reports_incompatible_chain_at_left_symbol() {
        let signature = signature();
        let mut ocb =
            OrderControlBlock::alloc(TermOrdering::Lpo, false, &signature, HoOrderKind::LfhoOrder);
        let mut scanner = scanner("a > b > a");

        let error = precedence_parse(&mut scanner, &signature, &mut ocb).unwrap_err();

        assert_eq!(error.code(), ErrorCode::INPUT_SEMANTIC_ERROR);
        assert!(error.message().contains("Precedence incompatible"));
    }

    #[test]
    fn empty_precedence_parse_keeps_current_state() {
        let signature = signature();
        let mut ocb =
            OrderControlBlock::alloc(TermOrdering::Lpo, false, &signature, HoOrderKind::LfhoOrder);
        let state = ocb.precedence_state();
        let mut scanner = scanner("");

        assert_eq!(
            precedence_parse(&mut scanner, &signature, &mut ocb),
            Ok(state)
        );
    }

    #[test]
    fn symbol_chain_parse_rejects_unknown_symbols_through_signature() {
        let signature = signature();
        let mut ocb =
            OrderControlBlock::alloc(TermOrdering::Lpo, false, &signature, HoOrderKind::LfhoOrder);
        let mut scanner = scanner("a > missing");

        let error = symbol_comparison_chain_parse(&mut scanner, &signature, &mut ocb).unwrap_err();

        assert_eq!(error.code(), ErrorCode::SYNTAX_ERROR);
        assert!(error.message().contains("undeclared"));
    }

    #[test]
    fn weights_parse_sets_kbo_weights_and_counts_assignments() {
        let signature = signature();
        let a = signature.find_f_code("a");
        let b = signature.find_f_code("b");
        let c = signature.find_f_code("c");
        let mut ocb =
            OrderControlBlock::alloc(TermOrdering::Kbo, true, &signature, HoOrderKind::LfhoOrder);
        let mut scanner = scanner("a:7,b:003");

        assert_eq!(weights_parse(&mut scanner, &signature, &mut ocb), Ok(2));
        assert_eq!(ocb.fun_weight(a), 7);
        assert_eq!(ocb.fun_weight(b), 3);
        assert_eq!(ocb.fun_weight(c), 1);
    }

    #[test]
    fn empty_weights_parse_returns_zero() {
        let signature = signature();
        let mut ocb =
            OrderControlBlock::alloc(TermOrdering::Kbo, true, &signature, HoOrderKind::LfhoOrder);
        let mut scanner = scanner("");

        assert_eq!(weights_parse(&mut scanner, &signature, &mut ocb), Ok(0));
    }
}
