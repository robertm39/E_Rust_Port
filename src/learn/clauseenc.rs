use crate::basics::error::{Diagnostic, ErrorCode};
use crate::clauses::eqn::Eqn;
use crate::clauses::eqn_props::{PatEqnDirection, EQUAL_PREDICATE};
use crate::clauses::eqnlist::EqnList;
use crate::inout::scanner::{IoFormat, Scanner, TokenType};
use crate::terms::functypes::func_symb_start_token;
use crate::terms::termbanks::TermBank;
use crate::terms::termfunc::term_standard_weight;
use crate::terms::termtypes::Term;
use crate::terms::typecheck::{
    type_declare_is_not_predicate, type_declare_is_predicate, TypeInferOptions,
};

/// Ordered literal/direction entries used by the C `PStack` clause-list shape.
pub type ClauseListRep<'a> = [(&'a Eqn, PatEqnDirection)];

/// Encodes a C `PStack` clause-list representation as a flat `$orN(...)` term.
///
/// # Panics
///
/// Panics if the term-bank insertion invariants fail after encoding a valid
/// literal list, matching the C assertions around `TBTermTopInsert`.
pub fn flat_encode_clause_list_rep(
    bank: &mut TermBank,
    list: &ClauseListRep<'_>,
) -> Result<Term, Diagnostic> {
    let arity = c_arity(list.len())?;
    let f_code = bank.signature_mut().get_or_n_code(arity);
    assert_ne!(f_code, 0, "flat clause representation symbol exists");

    let handle = Term::top_alloc(f_code, list.len());
    set_clause_rep_type(bank, &handle);
    for (index, (literal, direction)) in list.iter().enumerate() {
        handle.set_argument(index, literal.tb_term_encode(bank, *direction)?);
    }

    let result = bank.term_top_insert(handle)?;
    assert_eq!(result.weight(), term_standard_weight(&result));
    Ok(result)
}

/// Encodes a C `PStack` clause-list representation as a recursive `$or` list.
///
/// # Panics
///
/// Panics if the term-bank insertion invariants fail after encoding a valid
/// literal list, matching the C assertions around `TBTermTopInsert`.
pub fn rec_encode_clause_list_rep(
    bank: &mut TermBank,
    list: &ClauseListRep<'_>,
) -> Result<Term, Diagnostic> {
    let cnil_code = bank.signature_mut().get_cnil_code();
    assert_ne!(cnil_code, 0, "recursive clause nil symbol exists");
    let rest = Term::const_cell_alloc(cnil_code);
    set_clause_rep_type(bank, &rest);
    let mut rest = bank.term_top_insert(rest)?;

    for (literal, direction) in list.iter().rev() {
        let or_code = bank.signature_mut().get_or_code();
        assert_ne!(or_code, 0, "recursive clause cons symbol exists");
        let handle = Term::top_alloc(or_code, 2);
        set_clause_rep_type(bank, &handle);
        handle.set_argument(1, rest.clone());
        handle.set_argument(0, literal.tb_term_encode(bank, *direction)?);
        rest = bank.term_top_insert(handle)?;
    }

    assert_eq!(rest.weight(), term_standard_weight(&rest));
    Ok(rest)
}

/// Encodes an equation list using normal equation directions.
pub fn term_encode_eqn_list(
    bank: &mut TermBank,
    list: &EqnList,
    flat: bool,
) -> Result<Term, Diagnostic> {
    let rep: Vec<_> = list
        .as_slice()
        .iter()
        .map(|literal| (literal, PatEqnDirection::Normal))
        .collect();
    if flat {
        flat_encode_clause_list_rep(bank, &rep)
    } else {
        rec_encode_clause_list_rep(bank, &rep)
    }
}

/// Parses the C `ParseClauseTermRep` literal-list input and encodes it.
///
/// The accepted grammar follows the C call sequence:
/// `EqnListParse(..., Semicolon)` followed by `<-.`, where the hyphen must be
/// adjacent to `<` because C uses `AcceptInpTokNoSkip`.
pub fn parse_clause_term_rep(
    scanner: &mut Scanner,
    bank: &mut TermBank,
    flat: bool,
) -> Result<Term, Diagnostic> {
    if scanner.format() != IoFormat::Lop {
        return Err(Diagnostic::new(
            ErrorCode::SYNTAX_ERROR,
            "Clause term representations require LOP scanner format",
        ));
    }

    let list = parse_lop_eqn_list(scanner, bank, TokenType::SEMICOLON)?;
    scanner.accept_tok(TokenType::LESSER_SIGN)?;
    scanner.accept_tok_no_skip(TokenType::HYPHEN)?;
    scanner.accept_tok(TokenType::FULLSTOP)?;
    term_encode_eqn_list(bank, &list, flat)
}

/// Recodes a recursive `$or(..., $cnil)` clause representation as a flat one.
pub fn flat_recode_rec_clause_rep(
    bank: &mut TermBank,
    clauserep: &Term,
) -> Result<Term, Diagnostic> {
    let mut stack = Vec::new();
    let mut current = clauserep.clone();
    let or_code = bank.signature_mut().get_or_code();

    while current.f_code() == or_code {
        let encoded = required_argument(&current, 0)?;
        let rest = required_argument(&current, 1)?;
        let positive = encoded_eqn_polarity(bank, &encoded)?;
        let left = required_argument(&encoded, 0)?;
        let right = required_argument(&encoded, 1)?;
        stack.push((
            Eqn::alloc(left, right, bank, positive)?,
            PatEqnDirection::Normal,
        ));
        current = rest;
    }

    if current.f_code() != bank.signature().cnil_code() {
        return Err(recursive_clause_error());
    }

    let rep: Vec<_> = stack
        .iter()
        .map(|(literal, direction)| (literal, *direction))
        .collect();
    flat_encode_clause_list_rep(bank, &rep)
}

fn encoded_eqn_polarity(bank: &mut TermBank, encoded: &Term) -> Result<bool, Diagnostic> {
    if encoded.f_code() == bank.signature_mut().get_eqn_code(true) {
        Ok(true)
    } else if encoded.f_code() == bank.signature_mut().get_eqn_code(false) {
        Ok(false)
    } else {
        Err(recursive_clause_error())
    }
}

fn parse_lop_eqn_list(
    scanner: &mut Scanner,
    bank: &mut TermBank,
    sep: TokenType,
) -> Result<EqnList, Diagnostic> {
    let mut list = EqnList::new();
    if lop_eqn_list_starts(scanner, bank) {
        list.push(parse_lop_eqn(scanner, bank)?);
        while scanner.test_tok(sep) {
            scanner.next_token()?;
            list.push(parse_lop_eqn(scanner, bank)?);
        }
    }
    Ok(list)
}

fn lop_eqn_list_starts(scanner: &Scanner, bank: &TermBank) -> bool {
    scanner.test_tok(lop_term_start_token() | TokenType::TILDE_SIGN)
        || (bank.signature().supports_lists() && scanner.test_tok(TokenType::OPEN_SQUARE))
}

fn lop_term_start_token() -> TokenType {
    func_symb_start_token() | TokenType::MULT
}

fn parse_lop_eqn(scanner: &mut Scanner, bank: &mut TermBank) -> Result<Eqn, Diagnostic> {
    let mut negate = false;
    if scanner.test_tok(TokenType::TILDE_SIGN) {
        negate = true;
        scanner.next_token()?;
    }

    let (left, right, mut positive) = if scanner.test_id(EQUAL_PREDICATE) {
        scanner.next_token()?;
        scanner.accept_tok(TokenType::OPEN_BRACKET)?;
        let left = bank.parse_term_with_distinct_checks(scanner)?;
        scanner.accept_tok(TokenType::COMMA)?;
        let right = bank.parse_term_with_distinct_checks(scanner)?;
        scanner.accept_tok(TokenType::CLOSE_BRACKET)?;
        (left, right, true)
    } else {
        let left = bank.parse_term_with_distinct_checks(scanner)?;
        if scanner.test_tok(TokenType::NEG_EQUAL_SIGN | TokenType::EQUAL_SIGN) {
            let positive = !scanner.test_tok(TokenType::NEG_EQUAL_SIGN);
            scanner.accept_tok(TokenType::NEG_EQUAL_SIGN | TokenType::EQUAL_SIGN)?;
            let right = bank.parse_term_with_distinct_checks(scanner)?;
            type_declare_is_not_predicate(
                bank.signature_mut(),
                &left,
                TypeInferOptions::default(),
            )?;
            type_declare_is_not_predicate(
                bank.signature_mut(),
                &right,
                TypeInferOptions::default(),
            )?;
            (left, right, positive)
        } else {
            if !left.is_free_var() {
                type_declare_is_predicate(bank.signature_mut(), &left)?;
            }
            (left, bank.true_term().clone(), true)
        }
    };

    if negate {
        positive = !positive;
    }
    Eqn::alloc(left, right, bank, positive)
}

fn required_argument(term: &Term, index: usize) -> Result<Term, Diagnostic> {
    term.argument(index).ok_or_else(recursive_clause_error)
}

fn set_clause_rep_type(bank: &TermBank, term: &Term) {
    term.set_type(Some(bank.signature().type_bank().bool_type()));
}

fn c_arity(arity: usize) -> Result<i32, Diagnostic> {
    i32::try_from(arity).map_err(|_| {
        Diagnostic::new(
            ErrorCode::RESOURCE_OUT,
            "Clause representation arity is too large for C-compatible signatures",
        )
    })
}

fn recursive_clause_error() -> Diagnostic {
    Diagnostic::new(
        ErrorCode::SYNTAX_ERROR,
        "Term is not a correct recursive clause encoding!",
    )
}

#[cfg(test)]
mod tests {
    use super::{
        flat_encode_clause_list_rep, flat_recode_rec_clause_rep, parse_clause_term_rep,
        rec_encode_clause_list_rep, term_encode_eqn_list,
    };
    use crate::basics::error::ErrorCode;
    use crate::clauses::eqn::Eqn;
    use crate::clauses::eqn_props::PatEqnDirection;
    use crate::clauses::eqnlist::EqnList;
    use crate::inout::scanner::Scanner;
    use crate::terms::signature::Signature;
    use crate::terms::termbanks::TermBank;
    use crate::terms::termfunc::term_standard_weight;
    use crate::terms::termtypes::Term;
    use crate::terms::typebanks::TypeBank;

    #[test]
    fn flat_encoding_preserves_literal_order_and_direction() {
        let mut bank = test_bank();
        let (first, second, a, b, c) = sample_literals(&mut bank);

        let flat = flat_encode_clause_list_rep(
            &mut bank,
            &[
                (&first, PatEqnDirection::Normal),
                (&second, PatEqnDirection::Reverse),
            ],
        )
        .unwrap();

        assert_eq!(flat.f_code(), bank.signature_mut().get_or_n_code(2));
        assert_eq!(flat.arity(), 2);
        assert!(flat.is_shared());
        assert_eq!(flat.weight(), term_standard_weight(&flat));
        assert_encoded_literal(
            &flat.argument(0).expect("first encoded literal"),
            bank.signature().eqn_code(),
            &a,
            &b,
        );
        assert_encoded_literal(
            &flat.argument(1).expect("second encoded literal"),
            bank.signature().neqn_code(),
            &c,
            &b,
        );
    }

    #[test]
    fn recursive_encoding_builds_right_associative_or_list() {
        let mut bank = test_bank();
        let (first, second, a, b, c) = sample_literals(&mut bank);

        let rec = rec_encode_clause_list_rep(
            &mut bank,
            &[
                (&first, PatEqnDirection::Normal),
                (&second, PatEqnDirection::Reverse),
            ],
        )
        .unwrap();

        let or_code = bank.signature_mut().get_or_code();
        let cnil_code = bank.signature().cnil_code();
        assert_eq!(rec.f_code(), or_code);
        assert_eq!(rec.weight(), term_standard_weight(&rec));
        assert_encoded_literal(
            &rec.argument(0).expect("first encoded literal"),
            bank.signature().eqn_code(),
            &a,
            &b,
        );
        let tail = rec.argument(1).expect("recursive tail");
        assert_eq!(tail.f_code(), or_code);
        assert_encoded_literal(
            &tail.argument(0).expect("second encoded literal"),
            bank.signature().neqn_code(),
            &c,
            &b,
        );
        assert_eq!(tail.argument(1).expect("recursive nil").f_code(), cnil_code);
    }

    #[test]
    fn recursive_encoding_accepts_preexisting_logical_or_symbol() {
        let mut signature = Signature::new(TypeBank::new());
        signature
            .insert_internal_codes()
            .expect("internal code insertion");
        let mut bank = TermBank::new(signature).expect("term bank allocation");
        let (first, _, _, _, _) = sample_literals(&mut bank);

        let rec =
            rec_encode_clause_list_rep(&mut bank, &[(&first, PatEqnDirection::Normal)]).unwrap();

        assert_eq!(rec.f_code(), bank.signature().or_code());
        assert_eq!(rec.type_(), Some(bank.signature().type_bank().bool_type()));
    }

    #[test]
    fn eqn_list_encoding_uses_normal_literal_directions() {
        let mut bank = test_bank();
        let (first, second, a, b, c) = sample_literals(&mut bank);
        let list = EqnList::from_vec(vec![first, second]);

        let flat = term_encode_eqn_list(&mut bank, &list, true).unwrap();

        assert_eq!(flat.f_code(), bank.signature_mut().get_or_n_code(2));
        assert_encoded_literal(
            &flat.argument(0).expect("first encoded literal"),
            bank.signature().eqn_code(),
            &a,
            &b,
        );
        assert_encoded_literal(
            &flat.argument(1).expect("second encoded literal"),
            bank.signature().neqn_code(),
            &b,
            &c,
        );
    }

    #[test]
    fn recursive_recode_round_trips_to_flat_representation() {
        let mut bank = test_bank();
        let (first, second, a, b, c) = sample_literals(&mut bank);
        let rec = rec_encode_clause_list_rep(
            &mut bank,
            &[
                (&first, PatEqnDirection::Normal),
                (&second, PatEqnDirection::Reverse),
            ],
        )
        .unwrap();

        let flat = flat_recode_rec_clause_rep(&mut bank, &rec).unwrap();

        assert_eq!(flat.f_code(), bank.signature_mut().get_or_n_code(2));
        assert_encoded_literal(
            &flat.argument(0).expect("first encoded literal"),
            bank.signature().eqn_code(),
            &a,
            &b,
        );
        assert_encoded_literal(
            &flat.argument(1).expect("second encoded literal"),
            bank.signature().neqn_code(),
            &c,
            &b,
        );
    }

    #[test]
    fn recursive_recode_rejects_malformed_terms() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let or_code = bank.signature_mut().get_or_code();
        let bad = Term::top_alloc(or_code, 2);
        bad.set_argument(0, a);
        bad.set_argument(1, bank.true_term().clone());

        let error = flat_recode_rec_clause_rep(&mut bank, &bad).unwrap_err();

        assert_eq!(error.code(), ErrorCode::SYNTAX_ERROR);
        assert_eq!(
            error.message(),
            "Term is not a correct recursive clause encoding!"
        );
    }

    #[test]
    fn parse_clause_term_rep_preserves_order_and_lop_signs() {
        let mut bank = test_bank();
        let mut scanner = Scanner::from_user_string("a=b;~p<-.", false).unwrap();

        let flat = parse_clause_term_rep(&mut scanner, &mut bank, true).unwrap();

        assert_eq!(flat.f_code(), bank.signature_mut().get_or_n_code(2));
        assert_encoded_names(
            &bank,
            &flat.argument(0).expect("first encoded literal"),
            bank.signature().eqn_code(),
            "a",
            "b",
        );
        assert_encoded_names(
            &bank,
            &flat.argument(1).expect("second encoded literal"),
            bank.signature().neqn_code(),
            "p",
            "$true",
        );
    }

    #[test]
    fn parse_clause_term_rep_uses_checked_tbtermparse_shape() {
        let mut bank = test_bank();
        let mut scanner = Scanner::from_user_string("12(a)<-.", false).unwrap();

        let error = parse_clause_term_rep(&mut scanner, &mut bank, true).unwrap_err();

        assert_eq!(error.code(), ErrorCode::SYNTAX_ERROR);
        assert!(error.message().contains("Number cannot have argument list"));
    }

    #[test]
    fn parse_clause_term_rep_accepts_list_literal_when_signature_supports_lists() {
        let mut bank =
            TermBank::new(Signature::new_with_list_support(TypeBank::new(), true)).unwrap();
        let mut scanner = Scanner::from_user_string("[a,b]=c<-.", false).unwrap();

        let flat = parse_clause_term_rep(&mut scanner, &mut bank, true).unwrap();

        let encoded = flat.argument(0).expect("encoded literal");
        let left = encoded.argument(0).expect("encoded left term");
        let right = encoded.argument(1).expect("encoded right term");
        assert_eq!(bank.term_string(&left, true), "[a,b]");
        assert_eq!(bank.signature().find_name(right.f_code()), Some("c"));
    }

    #[test]
    fn parse_clause_term_rep_keeps_list_literal_outside_start_without_list_support() {
        let mut bank = test_bank();
        let mut scanner = Scanner::from_user_string("[a,b]=c<-.", false).unwrap();

        let error = parse_clause_term_rep(&mut scanner, &mut bank, true).unwrap_err();

        assert_eq!(error.code(), ErrorCode::SYNTAX_ERROR);
        assert!(error.message().contains("Lesser than"));
    }

    #[test]
    fn parse_clause_term_rep_accepts_empty_recursive_list() {
        let mut bank = test_bank();
        let mut scanner = Scanner::from_user_string("<-.", false).unwrap();

        let rec = parse_clause_term_rep(&mut scanner, &mut bank, false).unwrap();

        assert_eq!(rec.f_code(), bank.signature().cnil_code());
    }

    #[test]
    fn parse_clause_term_rep_requires_no_skip_before_terminator_hyphen() {
        let mut bank = test_bank();
        let mut scanner = Scanner::from_user_string("a=b< -.", false).unwrap();

        let error = parse_clause_term_rep(&mut scanner, &mut bank, true).unwrap_err();

        assert_eq!(error.code(), ErrorCode::SYNTAX_ERROR);
        assert!(error.message().contains("Hyphen"));
    }

    fn test_bank() -> TermBank {
        TermBank::new(Signature::new(TypeBank::new())).expect("term bank allocation")
    }

    fn sample_literals(bank: &mut TermBank) -> (Eqn, Eqn, Term, Term, Term) {
        let a = typed_const(bank, "a");
        let b = typed_const(bank, "b");
        let c = typed_const(bank, "c");
        let first = Eqn::alloc(a.clone(), b.clone(), bank, true).expect("positive literal");
        let second = Eqn::alloc(b.clone(), c.clone(), bank, false).expect("negative literal");
        (first, second, a, b, c)
    }

    fn typed_const(bank: &mut TermBank, name: &str) -> Term {
        let f_code = bank.signature_mut().insert_id(name, 0, false);
        let type_ = bank.signature().type_bank().i_type();
        bank.signature_mut()
            .declare_type(f_code, type_)
            .expect("constant type declaration");
        bank.create_const_term(f_code).expect("constant insertion")
    }

    fn assert_encoded_literal(encoded: &Term, f_code: i64, left: &Term, right: &Term) {
        assert_eq!(encoded.f_code(), f_code);
        assert_eq!(encoded.argument(0).as_ref(), Some(left));
        assert_eq!(encoded.argument(1).as_ref(), Some(right));
        assert!(encoded.is_shared());
    }

    fn assert_encoded_names(
        bank: &TermBank,
        encoded: &Term,
        f_code: i64,
        left_name: &str,
        right_name: &str,
    ) {
        assert_eq!(encoded.f_code(), f_code);
        let left = encoded.argument(0).expect("left encoded argument");
        let right = encoded.argument(1).expect("right encoded argument");
        assert_eq!(bank.signature().find_name(left.f_code()), Some(left_name));
        assert_eq!(bank.signature().find_name(right.f_code()), Some(right_name));
        assert!(encoded.is_shared());
    }
}
