//! Checked VIRAS preprocessing for formulas already owned by Umlaut.
//!
//! This boundary is deliberately narrower than general arithmetic
//! simplification. It imports one closed typed formula, runs bounded
//! quantifier elimination, independently validates the complete publication,
//! and then round-trips the result through Umlaut's ordinary typed parser
//! before returning a replacement term.

use super::typed_lira::{
    import_formula_with_max_rational_bits, render_tff_formula, ImportErrorCode,
};
use super::viras::{
    eliminate_formula, formula_node_count, validate_formula_derivation, Formula, FormulaQeOutcome,
    Limits, QeStatus, UnknownKind,
};
use crate::inout::scanner::{Scanner, TokenType};
use crate::terms::termbanks::TermBank;
use crate::terms::termtypes::Term;
use std::fmt;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VirasAppliedFormula {
    pub replacement: Term,
    pub source_nodes: usize,
    pub result_nodes: usize,
    pub branch_proofs: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum VirasFormulaPreprocessOutcome {
    Applied(VirasAppliedFormula),
    Unsupported {
        code: ImportErrorCode,
        reason: String,
    },
    Unknown {
        kind: UnknownKind,
        reason: String,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum VirasPublicationError {
    ProofCheck(String),
    Reembedding(String),
}

impl fmt::Display for VirasPublicationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ProofCheck(message) => write!(formatter, "VIRAS proof check failed: {message}"),
            Self::Reembedding(message) => {
                write!(formatter, "VIRAS result re-embedding failed: {message}")
            }
        }
    }
}

impl std::error::Error for VirasPublicationError {}

/// Produces a checked replacement for one closed typed formula.
///
/// Unsupported importer cases and bounded kernel Unknowns are ordinary
/// pass-through outcomes. Publication and typed re-embedding failures are
/// hard errors because no unchecked formula may enter clausification.
///
/// # Errors
///
/// Returns [`VirasPublicationError`] if native proof validation or canonical
/// typed re-embedding fails.
pub fn preprocess_formula(
    source: &Term,
    bank: &mut TermBank,
    limits: Limits,
) -> Result<VirasFormulaPreprocessOutcome, VirasPublicationError> {
    preprocess_formula_with_publication_mutation(source, bank, limits, |_| {})
}

fn preprocess_formula_with_publication_mutation(
    source: &Term,
    bank: &mut TermBank,
    limits: Limits,
    mutate: impl FnOnce(&mut FormulaQeOutcome),
) -> Result<VirasFormulaPreprocessOutcome, VirasPublicationError> {
    let imported = match import_formula_with_max_rational_bits(
        source,
        bank.signature(),
        limits.max_rational_bits,
    ) {
        Ok(imported) => imported,
        Err(error) => {
            return Ok(VirasFormulaPreprocessOutcome::Unsupported {
                code: error.code,
                reason: error.message,
            });
        }
    };
    let source_formula = imported.formula;
    let source_nodes = formula_node_count(&source_formula);
    let mut publication = eliminate_formula(source_formula.clone(), limits);
    if publication.status != QeStatus::Success {
        return Ok(VirasFormulaPreprocessOutcome::Unknown {
            kind: publication
                .unknown_kind
                .unwrap_or(UnknownKind::UnsupportedFragment),
            reason: publication.reason,
        });
    }

    mutate(&mut publication);
    validate_formula_derivation(&source_formula, &publication, limits)
        .map_err(|failure| VirasPublicationError::ProofCheck(failure.reason))?;
    let result = publication.formula.as_ref().ok_or_else(|| {
        VirasPublicationError::ProofCheck("validated publication has no result formula".to_owned())
    })?;
    if has_quantifier(result) || !result.variables().is_empty() {
        return Err(VirasPublicationError::ProofCheck(
            "validated result is not closed and quantifier-free".to_owned(),
        ));
    }

    let replacement = parse_result_into_bank(result, bank)?;
    let reimported = import_formula_with_max_rational_bits(
        &replacement,
        bank.signature(),
        limits.max_rational_bits,
    )
    .map_err(|error| {
        VirasPublicationError::Reembedding(format!(
            "round-tripped formula was rejected with {}: {}",
            error.code.as_str(),
            error.message
        ))
    })?;
    if &reimported.formula != result {
        return Err(VirasPublicationError::Reembedding(
            "round-tripped formula differs from the checked canonical result".to_owned(),
        ));
    }

    Ok(VirasFormulaPreprocessOutcome::Applied(
        VirasAppliedFormula {
            replacement,
            source_nodes,
            result_nodes: formula_node_count(result),
            branch_proofs: publication.derivation.eliminations.len(),
        },
    ))
}

fn parse_result_into_bank(
    result: &Formula,
    bank: &mut TermBank,
) -> Result<Term, VirasPublicationError> {
    let rendered = render_tff_formula(result);
    let mut scanner = Scanner::from_user_string(&rendered, false)
        .map_err(|diagnostic| VirasPublicationError::Reembedding(diagnostic.to_string()))?;
    let replacement = bank
        .parse_tformula_tstp(&mut scanner)
        .map_err(|diagnostic| VirasPublicationError::Reembedding(diagnostic.to_string()))?;
    if !scanner.test_tok(TokenType::NO_TOKEN) {
        return Err(VirasPublicationError::Reembedding(format!(
            "canonical result left trailing token {}",
            scanner.current_token().literal()
        )));
    }
    Ok(replacement)
}

fn has_quantifier(formula: &Formula) -> bool {
    match formula {
        Formula::Exists(_, _) | Formula::Forall(_, _) => true,
        Formula::And(children) | Formula::Or(children) => children.iter().any(has_quantifier),
        Formula::Bool(_) | Formula::Atom(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arithmetic::viras::boolean;
    use crate::basics::simple_stuff::{set_problem_type, ProblemType};
    use crate::terms::signature::Signature;
    use crate::terms::typebanks::TypeBank;
    use crate::test_support::global_state_lock;

    fn parsed_formula(source: &str) -> (TermBank, Term) {
        let mut signature = Signature::new(TypeBank::new());
        signature
            .insert_internal_codes()
            .expect("insert logical symbols");
        signature.set_typed_symbols(true);
        let mut bank = TermBank::new(signature).expect("term bank");
        let mut scanner = Scanner::from_user_string(source, false).expect("formula scanner");
        let formula = bank
            .parse_tformula_tstp(&mut scanner)
            .expect("typed formula parses");
        (bank, formula)
    }

    #[test]
    fn checked_preprocessing_round_trips_an_eligible_formula() {
        let _global = global_state_lock();
        set_problem_type(ProblemType::FirstOrder).expect("set typed test mode");
        let (mut bank, source) =
            parsed_formula("? [X:$real] : ($greater(X,$to_real(0)) & $less(X,$to_real(1)))");
        let outcome =
            preprocess_formula(&source, &mut bank, Limits::default()).expect("publication");
        let VirasFormulaPreprocessOutcome::Applied(applied) = outcome else {
            panic!("eligible formula must be applied");
        };
        assert!(applied.source_nodes > applied.result_nodes);
        assert_eq!(applied.branch_proofs, 1);
        assert_eq!(
            import_formula_with_max_rational_bits(
                &applied.replacement,
                bank.signature(),
                Limits::default().max_rational_bits,
            )
            .expect("round-tripped import")
            .formula,
            boolean(true)
        );
    }

    #[test]
    fn unsupported_and_resource_unknown_are_pass_through_outcomes() {
        let _global = global_state_lock();
        set_problem_type(ProblemType::FirstOrder).expect("set typed test mode");
        let (mut bank, unsupported) = parsed_formula("? [X:$real] : ($product(X,X) = $to_real(1))");
        assert!(matches!(
            preprocess_formula(&unsupported, &mut bank, Limits::default())
                .expect("unsupported outcome"),
            VirasFormulaPreprocessOutcome::Unsupported { .. }
        ));

        let (mut bank, bounded) =
            parsed_formula("? [X:$real] : ($greater(X,$to_real(0)) | $less(X,$to_real(1)))");
        let limits = Limits {
            max_dnf_branches: 0,
            ..Limits::default()
        };
        assert!(matches!(
            preprocess_formula(&bounded, &mut bank, limits).expect("bounded outcome"),
            VirasFormulaPreprocessOutcome::Unknown {
                kind: UnknownKind::ResourceLimit,
                ..
            }
        ));
    }

    #[test]
    fn publication_mutation_is_rejected_before_a_replacement_is_returned() {
        let _global = global_state_lock();
        set_problem_type(ProblemType::FirstOrder).expect("set typed test mode");
        let (mut bank, source) = parsed_formula("? [X:$real] : $greater(X,$to_real(0))");
        let error = preprocess_formula_with_publication_mutation(
            &source,
            &mut bank,
            Limits::default(),
            |publication| publication.formula = Some(boolean(false)),
        )
        .expect_err("corrupt publication must fail closed");
        assert!(matches!(error, VirasPublicationError::ProofCheck(_)));
    }
}
