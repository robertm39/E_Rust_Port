//! Generic ordering parser helpers from `cto_orderings`.

use crate::basics::error::{Diagnostic, ErrorCode};
use crate::basics::partial_orderings::CompareResult;
use crate::basics::pstacks::PStackPointer;
use crate::heuristics::to_params::TermOrdering;
use crate::inout::scanner::{token_pos_rep, Scanner, Token, TokenType};
use crate::orderings::cto_kbo::{kbo_compare, kbo_greater};
use crate::orderings::cto_kbolin::{
    kbo6_compare, kbo6_compare_with_bank, kbo6_greater, kbo6_greater_with_bank,
};
use crate::orderings::cto_lpo::{
    lpo4_compare, lpo4_compare_copy, lpo4_compare_with_bank, lpo4_greater, lpo4_greater_copy,
    lpo4_greater_with_bank, lpo_compare, lpo_compare_copy, lpo_greater, lpo_greater_copy,
};
use crate::orderings::ocb::{OrderControlBlock, W_DEFAULT_WEIGHT};
use crate::terms::signature::Signature;
use crate::terms::termbanks::TermBank;
use crate::terms::termtypes::{DerefType, Term};

/// Test whether `s` is greater than `t` in the ordering described by `ocb`.
///
/// # Panics
///
/// Panics for ordering variants whose concrete comparison algorithm is not
/// ported yet, or under the selected algorithm's internal invariants.
pub fn to_greater(
    ocb: &mut OrderControlBlock,
    signature: &Signature,
    s: &Term,
    t: &Term,
    deref_s: DerefType,
    deref_t: DerefType,
) -> bool {
    match ocb.ordering_type {
        TermOrdering::Lpo => lpo_greater(ocb, signature, s, t, deref_s, deref_t),
        TermOrdering::LpoCopy => lpo_greater_copy(ocb, signature, s, t, deref_s, deref_t),
        TermOrdering::Lpo4 => lpo4_greater(ocb, signature, s, t, deref_s, deref_t),
        TermOrdering::Lpo4Copy => lpo4_greater_copy(ocb, signature, s, t, deref_s, deref_t),
        TermOrdering::Kbo => kbo_greater(ocb, signature, s, t, deref_s, deref_t),
        TermOrdering::Kbo6 => kbo6_greater(ocb, signature, s, t, deref_s, deref_t),
        TermOrdering::Empty => false,
        TermOrdering::Rpo => panic!(
            "term ordering {:?} comparison is not ported yet",
            ocb.ordering_type
        ),
        TermOrdering::NoOrdering | TermOrdering::Optimize => {
            panic!("non-concrete term ordering cannot compare terms")
        }
    }
}

/// Test whether `s` is greater than `t`, using bank-backed comparison paths
/// where the selected ordering needs term-bank normalization.
///
/// # Errors
///
/// Returns a diagnostic if bank-backed ordering preparation fails.
///
/// # Panics
///
/// Panics under the same invariants as [`to_greater`].
pub fn to_greater_with_bank(
    ocb: &mut OrderControlBlock,
    bank: &mut TermBank,
    s: &Term,
    t: &Term,
    deref_s: DerefType,
    deref_t: DerefType,
) -> Result<bool, Diagnostic> {
    match ocb.ordering_type {
        TermOrdering::Kbo6 => kbo6_greater_with_bank(ocb, bank, s, t, deref_s, deref_t),
        TermOrdering::Lpo4 => lpo4_greater_with_bank(ocb, bank, s, t, deref_s, deref_t),
        _ => Ok(to_greater(ocb, bank.signature(), s, t, deref_s, deref_t)),
    }
}

/// Compare `s` and `t` in the ordering described by `ocb`.
///
/// # Panics
///
/// Panics for ordering variants whose concrete comparison algorithm is not
/// ported yet, or under the selected algorithm's internal invariants.
#[must_use]
pub fn to_compare(
    ocb: &mut OrderControlBlock,
    signature: &Signature,
    s: &Term,
    t: &Term,
    deref_s: DerefType,
    deref_t: DerefType,
) -> CompareResult {
    match ocb.ordering_type {
        TermOrdering::Lpo => lpo_compare(ocb, signature, s, t, deref_s, deref_t),
        TermOrdering::LpoCopy => lpo_compare_copy(ocb, signature, s, t, deref_s, deref_t),
        TermOrdering::Lpo4 => lpo4_compare(ocb, signature, s, t, deref_s, deref_t),
        TermOrdering::Lpo4Copy => lpo4_compare_copy(ocb, signature, s, t, deref_s, deref_t),
        TermOrdering::Kbo => kbo_compare(ocb, signature, s, t, deref_s, deref_t),
        TermOrdering::Kbo6 => kbo6_compare(ocb, signature, s, t, deref_s, deref_t),
        TermOrdering::Empty => CompareResult::Uncomparable,
        TermOrdering::Rpo => panic!(
            "term ordering {:?} comparison is not ported yet",
            ocb.ordering_type
        ),
        TermOrdering::NoOrdering | TermOrdering::Optimize => {
            panic!("non-concrete term ordering cannot compare terms")
        }
    }
}

/// Compare `s` and `t`, using bank-backed comparison paths where the selected
/// ordering needs term-bank normalization.
///
/// # Errors
///
/// Returns a diagnostic if bank-backed ordering preparation fails.
///
/// # Panics
///
/// Panics under the same invariants as [`to_compare`].
pub fn to_compare_with_bank(
    ocb: &mut OrderControlBlock,
    bank: &mut TermBank,
    s: &Term,
    t: &Term,
    deref_s: DerefType,
    deref_t: DerefType,
) -> Result<CompareResult, Diagnostic> {
    match ocb.ordering_type {
        TermOrdering::Kbo6 => kbo6_compare_with_bank(ocb, bank, s, t, deref_s, deref_t),
        TermOrdering::Lpo4 => lpo4_compare_with_bank(ocb, bank, s, t, deref_s, deref_t),
        _ => Ok(to_compare(ocb, bank.signature(), s, t, deref_s, deref_t)),
    }
}

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
        compare_symbol_parse, precedence_parse, symbol_comparison_chain_parse, to_compare,
        to_compare_with_bank, to_greater, to_greater_with_bank, weights_parse,
    };
    use crate::basics::error::ErrorCode;
    use crate::basics::partial_orderings::{CompareResult, HoOrderKind};
    use crate::basics::simple_stuff::{reset_problem_type, set_problem_type, ProblemType};
    use crate::heuristics::to_params::TermOrdering;
    use crate::inout::scanner::Scanner;
    use crate::orderings::ocb::OrderControlBlock;
    use crate::terms::functypes::FunCode;
    use crate::terms::signature::{Signature, SIG_PHONY_APP_CODE};
    use crate::terms::simpletypes::alloc_arrow_type;
    use crate::terms::subst::Substitution;
    use crate::terms::termbanks::TermBank;
    use crate::terms::termtypes::{DerefType, Term};
    use crate::terms::typebanks::TypeBank;
    use crate::test_support::global_state_lock;

    struct ProblemTypeReset;

    impl Drop for ProblemTypeReset {
        fn drop(&mut self) {
            reset_problem_type();
        }
    }

    fn set_problem_type_for_test(problem_type: ProblemType) -> ProblemTypeReset {
        reset_problem_type();
        set_problem_type(problem_type).unwrap_or_else(|err| panic!("{err}"));
        ProblemTypeReset
    }

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

    fn app(symbol: FunCode, args: &[Term]) -> Term {
        let term = Term::top_alloc(symbol, args.len());
        for (index, arg) in args.iter().enumerate() {
            term.set_argument(index, arg.clone());
        }
        term
    }

    fn test_bank() -> TermBank {
        TermBank::new(signature()).unwrap_or_else(|err| panic!("{err}"))
    }

    fn typed_const(bank: &mut TermBank, name: &str) -> Term {
        let type_ = bank.signature().type_bank().default_type();
        let f_code = bank.signature().find_f_code(name);
        if bank.signature().get_type(f_code).is_none() {
            bank.signature_mut()
                .declare_final_type(f_code, type_.clone())
                .unwrap_or_else(|err| panic!("{err}"));
        }
        bank.create_const_term(f_code)
            .unwrap_or_else(|err| panic!("{err}"))
    }

    fn typed_unary_const(bank: &mut TermBank, name: &str) -> Term {
        let type_ = bank.signature().type_bank().default_type();
        let symbol_type = bank
            .signature_mut()
            .type_bank_mut()
            .insert_type_shared(alloc_arrow_type(vec![type_.clone(), type_]));
        let f_code = bank.signature_mut().insert_id(name, 0, false);
        if bank.signature().get_type(f_code).is_none() {
            bank.signature_mut()
                .declare_final_type(f_code, symbol_type)
                .unwrap_or_else(|err| panic!("{err}"));
        }
        bank.create_const_term(f_code)
            .unwrap_or_else(|err| panic!("{err}"))
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

    #[test]
    fn to_compare_dispatches_ported_orderings() {
        let mut signature = signature();
        let f = signature.insert_id("f", 1, false);
        let x = Term::const_cell_alloc(-2);
        let f_x = app(f, std::slice::from_ref(&x));

        for ordering in [
            TermOrdering::Kbo,
            TermOrdering::Kbo6,
            TermOrdering::Lpo,
            TermOrdering::LpoCopy,
            TermOrdering::Lpo4,
            TermOrdering::Lpo4Copy,
        ] {
            let mut ocb =
                OrderControlBlock::alloc(ordering, true, &signature, HoOrderKind::LfhoOrder);
            assert_eq!(
                to_compare(
                    &mut ocb,
                    &signature,
                    &f_x,
                    &x,
                    DerefType::Never,
                    DerefType::Never
                ),
                CompareResult::Greater,
                "dispatch failed for {ordering:?}"
            );
        }

        let mut lpo =
            OrderControlBlock::alloc(TermOrdering::Lpo, true, &signature, HoOrderKind::LfhoOrder);
        assert!(to_greater(
            &mut lpo,
            &signature,
            &f_x,
            &x,
            DerefType::Never,
            DerefType::Never
        ));
    }

    #[test]
    fn to_compare_legacy_orderings_accept_first_order_surface_in_higher_order_problem() {
        let _guard = global_state_lock();
        let _problem_type = set_problem_type_for_test(ProblemType::HigherOrder);
        let mut signature = signature();
        let f = signature.insert_id("ho_fo_dispatch_f", 1, false);
        let x = Term::const_cell_alloc(-2);
        let f_x = app(f, std::slice::from_ref(&x));

        for ordering in [
            TermOrdering::Kbo,
            TermOrdering::Lpo,
            TermOrdering::LpoCopy,
            TermOrdering::Lpo4,
            TermOrdering::Lpo4Copy,
        ] {
            let mut ocb =
                OrderControlBlock::alloc(ordering, true, &signature, HoOrderKind::LfhoOrder);
            assert_eq!(
                to_compare(
                    &mut ocb,
                    &signature,
                    &f_x,
                    &x,
                    DerefType::Never,
                    DerefType::Never
                ),
                CompareResult::Greater,
                "higher-order first-order-surface dispatch failed for {ordering:?}"
            );
        }
    }

    #[test]
    fn to_compare_legacy_orderings_accept_higher_order_surface_like_release() {
        let _guard = global_state_lock();
        let _problem_type = set_problem_type_for_test(ProblemType::HigherOrder);
        let signature = signature();
        let head = Term::const_cell_alloc(-2);
        let arg = Term::const_cell_alloc(signature.find_f_code("a"));
        let applied = app(SIG_PHONY_APP_CODE, &[head, arg.clone()]);

        for ordering in [TermOrdering::Kbo, TermOrdering::Lpo, TermOrdering::LpoCopy] {
            let mut ocb =
                OrderControlBlock::alloc(ordering, true, &signature, HoOrderKind::LfhoOrder);
            assert_eq!(
                to_compare(
                    &mut ocb,
                    &signature,
                    &applied,
                    &arg,
                    DerefType::Never,
                    DerefType::Never,
                ),
                CompareResult::Greater,
                "optimized-C higher-order dispatch failed for {ordering:?}"
            );
        }
    }

    #[test]
    fn to_compare_lpo4_accepts_equal_higher_order_surface_before_structural_equality() {
        let _guard = global_state_lock();
        let _problem_type = set_problem_type_for_test(ProblemType::HigherOrder);
        let signature = signature();
        let head = Term::const_cell_alloc(-2);
        let arg = Term::const_cell_alloc(signature.find_f_code("a"));
        let applied = app(SIG_PHONY_APP_CODE, &[head, arg]);
        let mut ocb =
            OrderControlBlock::alloc(TermOrdering::Lpo4, true, &signature, HoOrderKind::LfhoOrder);

        assert_eq!(
            to_compare(
                &mut ocb,
                &signature,
                &applied,
                &applied,
                DerefType::Never,
                DerefType::Never,
            ),
            CompareResult::Equal
        );
    }

    #[test]
    fn to_compare_with_bank_dispatches_lpo4_applied_variable_instantiation() {
        let _guard = global_state_lock();
        let _problem_type = set_problem_type_for_test(ProblemType::HigherOrder);
        let mut bank = test_bank();
        let type_ = bank.signature().type_bank().default_type();
        let head_binding = typed_unary_const(&mut bank, "lpo4_dispatch_applied_binding");
        let head_type = head_binding.type_().expect("binding must have a type");
        let head = bank.vars().get_fresh_var(&head_type);
        let a = typed_const(&mut bank, "a");
        let applied = app(SIG_PHONY_APP_CODE, &[head.clone(), a.clone()]);
        applied.set_type(Some(type_));
        let mut subst = Substitution::new();
        subst.add_binding(&head, &head_binding);
        let mut ocb = OrderControlBlock::alloc(
            TermOrdering::Lpo4,
            true,
            bank.signature(),
            HoOrderKind::LfhoOrder,
        );
        ocb.set_fun_prec_weight(head_binding.f_code(), 20);
        ocb.set_fun_prec_weight(a.f_code(), 10);

        assert_eq!(
            to_compare_with_bank(
                &mut ocb,
                &mut bank,
                &applied,
                &a,
                DerefType::Once,
                DerefType::Never
            )
            .unwrap_or_else(|err| panic!("{err}")),
            CompareResult::Greater
        );
        assert!(to_greater_with_bank(
            &mut ocb,
            &mut bank,
            &applied,
            &a,
            DerefType::Once,
            DerefType::Never
        )
        .unwrap_or_else(|err| panic!("{err}")));

        subst.backtrack();
    }

    #[test]
    fn to_compare_empty_ordering_matches_c_compare_surface() {
        let signature = signature();
        let mut ocb = OrderControlBlock::alloc(
            TermOrdering::Empty,
            true,
            &signature,
            HoOrderKind::LfhoOrder,
        );
        let x = Term::const_cell_alloc(-2);

        assert_eq!(
            to_compare(
                &mut ocb,
                &signature,
                &x,
                &x,
                DerefType::Never,
                DerefType::Never
            ),
            CompareResult::Uncomparable
        );
        assert!(!to_greater(
            &mut ocb,
            &signature,
            &x,
            &x,
            DerefType::Never,
            DerefType::Never
        ));
    }
}
