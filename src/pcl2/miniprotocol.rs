//! Port of `PCL2/pcl_miniprotocol`.

use std::collections::BTreeSet;
use std::io::Write as IoWrite;

use crate::basics::error::{Diagnostic, ErrorCode};
use crate::basics::simple_stuff::ProblemType;
use crate::clauses::inferencedoc::ProofDocOutputFormat;
use crate::inout::scanner::{token_pos_rep, Scanner, Token, TokenType};
use crate::pcl2::expressions::{pcl_step_extract, PclExpression, PclExpressionData, PclQuote};
use crate::pcl2::ministeps::{PclMiniStep, PclMiniStepParseOptions};
use crate::pcl2::steps::{PclStepProperties, PCL_IS_PROOF_STEP};
use crate::terms::signature::Signature;
use crate::terms::termbanks::TermBank;
use crate::terms::typebanks::TypeBank;

#[derive(Debug)]
pub struct PclMiniProtocol {
    terms: TermBank,
    steps: Vec<Option<PclMiniStep>>,
    max_ident: i64,
}

impl PclMiniProtocol {
    /// C `PCLMiniProtAlloc`.
    ///
    /// # Errors
    ///
    /// Returns diagnostics if internal signature or term-bank initialization
    /// fails.
    pub fn new() -> Result<Self, Diagnostic> {
        let mut signature = Signature::new(TypeBank::new());
        signature.insert_internal_codes()?;
        Ok(Self {
            terms: TermBank::new(signature)?,
            steps: vec![None],
            max_ident: 0,
        })
    }

    #[must_use]
    pub const fn term_bank(&self) -> &TermBank {
        &self.terms
    }

    pub const fn term_bank_mut(&mut self) -> &mut TermBank {
        &mut self.terms
    }

    #[must_use]
    pub const fn max_ident(&self) -> i64 {
        self.max_ident
    }

    /// C `PCLMiniProtInsertStep`.
    ///
    /// # Errors
    ///
    /// Returns a syntax diagnostic if the mini-step id cannot index the
    /// protocol's positive-id array.
    pub fn insert_step(&mut self, step: PclMiniStep) -> Result<bool, Diagnostic> {
        let index = step_index(step.id())?;
        if index >= self.steps.len() {
            self.steps.resize_with(index + 1, || None);
        }
        if self.steps[index].is_some() {
            return Ok(false);
        }
        self.max_ident = self.max_ident.max(step.id());
        self.steps[index] = Some(step);
        Ok(true)
    }

    /// C `PCLMiniProtFindStep`.
    #[must_use]
    pub fn find_step(&self, id: i64) -> Option<&PclMiniStep> {
        let index = usize::try_from(id).ok()?;
        self.steps.get(index)?.as_ref()
    }

    #[must_use]
    pub fn find_step_mut(&mut self, id: i64) -> Option<&mut PclMiniStep> {
        let index = usize::try_from(id).ok()?;
        self.steps.get_mut(index)?.as_mut()
    }

    /// C `PCLMiniProtExtractStep`.
    pub fn extract_step(&mut self, id: i64) -> Option<PclMiniStep> {
        let index = usize::try_from(id).ok()?;
        self.steps.get_mut(index)?.take()
    }

    /// C `PCLMiniProtDeleteStep`.
    pub fn delete_step(&mut self, id: i64) -> bool {
        self.extract_step(id).is_some()
    }

    /// C `PCLMiniProtParse`.
    ///
    /// # Errors
    ///
    /// Returns scanner diagnostics for invalid protocol syntax, duplicate
    /// identifiers, or diagnostics from mini-step parsing.
    pub fn parse(
        &mut self,
        scanner: &mut Scanner,
        options: PclMiniStepParseOptions,
    ) -> Result<i64, Diagnostic> {
        let mut count = 0;
        while scanner.test_tok(TokenType::POS_INT) {
            let start = scanner.current_token().clone();
            let step = PclMiniStep::parse(scanner, &mut self.terms, options)?;
            if !self.insert_step(step)? {
                return Err(duplicate_identifier_error(&start));
            }
            count += 1;
        }
        Ok(count)
    }

    /// C `PCLMiniProtParse`, including comment forwarding through explicit output.
    ///
    /// # Errors
    ///
    /// Returns scanner diagnostics for invalid protocol syntax, duplicate
    /// identifiers, diagnostics from mini-step parsing, or output write
    /// failures.
    pub fn parse_with_output(
        &mut self,
        output: &mut (impl IoWrite + ?Sized),
        scanner: &mut Scanner,
        options: PclMiniStepParseOptions,
    ) -> Result<i64, Diagnostic> {
        let mut count = 0;
        while scanner.test_tok(TokenType::POS_INT) {
            write_current_comment(output, scanner)?;
            let start = scanner.current_token().clone();
            let step = PclMiniStep::parse(scanner, &mut self.terms, options)?;
            if !self.insert_step(step)? {
                return Err(duplicate_identifier_error(&start));
            }
            count += 1;
        }
        write_current_comment(output, scanner)?;
        Ok(count)
    }

    /// C `PCLMiniProtPrint`.
    ///
    /// # Errors
    ///
    /// Returns diagnostics from mini-step rendering.
    pub fn print_string(
        &mut self,
        format: ProofDocOutputFormat,
        problem_type: ProblemType,
    ) -> Result<String, Diagnostic> {
        let mut output = String::new();
        let terms = &mut self.terms;
        for step in self.steps.iter().flatten() {
            output.push_str(&step.print_format_string(terms, problem_type, format)?);
        }
        Ok(output)
    }

    /// C `PCLMiniExprCollectPreconds`.
    ///
    /// # Errors
    ///
    /// Returns a syntax diagnostic when a quoted mini-step id is dangling or a
    /// full PCL identifier appears in a mini-protocol expression.
    pub fn collect_preconditions(&self, expr: &PclExpression) -> Result<Vec<i64>, Diagnostic> {
        let mut ids = BTreeSet::new();
        self.collect_preconditions_into(expr, &mut ids)?;
        Ok(ids.into_iter().collect())
    }

    /// C `PCLMiniProtMarkProofClauses`.
    ///
    /// # Errors
    ///
    /// Returns a syntax diagnostic when an extract-marked proof step has a
    /// dangling precondition reference.
    pub fn mark_proof_clauses(&mut self, fast: bool) -> Result<bool, Diagnostic> {
        let mut found_empty_clause = false;
        let mut to_process = self.proof_seed_ids(fast);

        while let Some(id) = to_process.pop() {
            let just = {
                let Some(step) = self.find_step_mut(id) else {
                    continue;
                };
                if (step.is_shell() && step.extra() == Some("'proof'"))
                    || (!step.is_shell() && step.is_clausal() && step.is_empty_clause())
                {
                    found_empty_clause = true;
                }
                if step.properties().query(PCL_IS_PROOF_STEP) {
                    None
                } else {
                    step.set_property(PCL_IS_PROOF_STEP);
                    Some(step.just().clone())
                }
            };
            if let Some(just) = just {
                for precondition in self.collect_preconditions(&just)? {
                    to_process.push(precondition);
                }
            }
        }
        Ok(found_empty_clause)
    }

    /// C `PCLMiniProtSetClauseProp`.
    pub fn set_clause_property(&mut self, properties: PclStepProperties) {
        for step in &mut self.steps {
            if let Some(step) = step.as_mut() {
                step.set_property(properties);
            }
        }
    }

    /// C `PCLMiniProtDelClauseProp`.
    pub fn delete_clause_property(&mut self, properties: PclStepProperties) {
        for step in &mut self.steps {
            if let Some(step) = step.as_mut() {
                step.delete_property(properties);
            }
        }
    }

    /// C `PCLMiniProtPrintProofClauses`.
    ///
    /// # Errors
    ///
    /// Returns diagnostics from mini-step rendering.
    pub fn print_proof_clauses_string(
        &mut self,
        format: ProofDocOutputFormat,
        problem_type: ProblemType,
    ) -> Result<String, Diagnostic> {
        let mut output = String::new();
        let terms = &mut self.terms;
        for step in self.steps.iter().flatten() {
            if step.properties().query(PCL_IS_PROOF_STEP) {
                output.push_str(&step.print_format_string(terms, problem_type, format)?);
                output.push('\n');
            }
        }
        Ok(output)
    }

    fn collect_preconditions_into(
        &self,
        expr: &PclExpression,
        ids: &mut BTreeSet<i64>,
    ) -> Result<(), Diagnostic> {
        match expr.data() {
            PclExpressionData::None => Err(protocol_error("Unknown PCL expression in protocol")),
            PclExpressionData::Initial(_) => Ok(()),
            PclExpressionData::Quote {
                quote: PclQuote::Mini(id),
                ..
            } => {
                if self.find_step(*id).is_none() {
                    return Err(protocol_error("Dangling reference in PCL protocol!"));
                }
                ids.insert(*id);
                Ok(())
            }
            PclExpressionData::Quote {
                quote: PclQuote::Full(_),
                ..
            } => Err(protocol_error(
                "Full PCL identifier found in mini protocol expression",
            )),
            PclExpressionData::Compound(args) => {
                for argument in args {
                    self.collect_preconditions_into(argument.expr(), ids)?;
                }
                Ok(())
            }
        }
    }

    fn proof_seed_ids(&self, fast: bool) -> Vec<i64> {
        let mut ids = Vec::new();
        if fast {
            let mut id = self.max_ident;
            while id >= 0 {
                let Some(step) = self.find_step(id) else {
                    break;
                };
                if !pcl_step_extract(step.extra()) {
                    break;
                }
                ids.push(id);
                if id == 0 {
                    break;
                }
                id -= 1;
            }
        } else {
            for id in 0..=self.max_ident {
                if self
                    .find_step(id)
                    .is_some_and(|step| pcl_step_extract(step.extra()))
                {
                    ids.push(id);
                }
            }
        }
        ids
    }
}

fn step_index(id: i64) -> Result<usize, Diagnostic> {
    usize::try_from(id).map_err(|_| protocol_error("Negative PCL identifier"))
}

fn duplicate_identifier_error(token: &Token) -> Diagnostic {
    Diagnostic::new(
        ErrorCode::SYNTAX_ERROR,
        format!(
            "{}(just read '{}'): duplicate PCL identifier",
            token_pos_rep(token),
            token.literal()
        ),
    )
}

fn write_current_comment(
    output: &mut (impl IoWrite + ?Sized),
    scanner: &mut Scanner,
) -> Result<(), Diagnostic> {
    let comment = scanner.take_current_comment_bytes();
    if comment.is_empty() {
        Ok(())
    } else {
        output.write_all(&comment).map_err(|error| {
            Diagnostic::new(
                ErrorCode::FILE_ERROR,
                format!("Error writing output: {error}"),
            )
        })
    }
}

fn protocol_error(message: &str) -> Diagnostic {
    Diagnostic::new(ErrorCode::SYNTAX_ERROR, message)
}

#[cfg(test)]
mod tests {
    use super::PclMiniProtocol;
    use crate::basics::simple_stuff::ProblemType;
    use crate::clauses::inferencedoc::ProofDocOutputFormat;
    use crate::inout::scanner::{IoFormat, Scanner};
    use crate::pcl2::ministeps::PclMiniStepParseOptions;
    use crate::pcl2::steps::{PCL_IS_EXAMPLE, PCL_IS_PROOF_STEP, PCL_TYPE_AXIOM};

    fn parse_protocol(source: &str) -> (PclMiniProtocol, Scanner) {
        let mut protocol = PclMiniProtocol::new().unwrap();
        let mut scanner = Scanner::from_user_string(source, false).unwrap();
        scanner.set_format(IoFormat::Tptp);
        let count = protocol
            .parse(
                &mut scanner,
                PclMiniStepParseOptions {
                    problem_type: ProblemType::FirstOrder,
                    support_shell_pcl: true,
                },
            )
            .unwrap();
        assert!(count > 0);
        (protocol, scanner)
    }

    #[test]
    fn parses_stores_and_prints_steps_in_id_order_without_separators() {
        let (mut protocol, scanner) =
            parse_protocol("2 : : [++q] : initial : 'proof'\n1 : lemma : [++p] : 2 tail");

        assert_eq!(protocol.max_ident(), 2);
        assert!(protocol.find_step(0).is_none());
        assert_eq!(protocol.find_step(1).unwrap().id(), 1);
        assert_eq!(protocol.find_step(2).unwrap().id(), 2);
        assert_eq!(scanner.current_token().literal(), "tail");

        assert_eq!(
            protocol
                .print_string(ProofDocOutputFormat::Pcl, ProblemType::FirstOrder)
                .unwrap(),
            "     1 : lemma : [++p] : 2     2 :  : [++q] : initial : 'proof'"
        );
    }

    #[test]
    fn parse_with_output_forwards_comments_and_clears_them() {
        let mut protocol = PclMiniProtocol::new().unwrap();
        let mut scanner = Scanner::from_user_string(
            "% lead\n1 : : [++p] : initial\n# mid\n2 : : [++q] : 1\n% tail\ntail",
            false,
        )
        .unwrap();
        scanner.set_format(IoFormat::Tptp);
        let mut output = Vec::new();

        let count = protocol
            .parse_with_output(
                &mut output,
                &mut scanner,
                PclMiniStepParseOptions {
                    problem_type: ProblemType::FirstOrder,
                    support_shell_pcl: true,
                },
            )
            .unwrap();

        assert_eq!(count, 2);
        assert_eq!(
            String::from_utf8(output).unwrap(),
            "% lead\n# mid\n% tail\n"
        );
        assert_eq!(scanner.current_token().literal(), "tail");
        assert!(scanner.current_token().comment_bytes().is_empty());

        let mut repeated = Vec::new();
        assert_eq!(
            protocol
                .parse_with_output(
                    &mut repeated,
                    &mut scanner,
                    PclMiniStepParseOptions::default()
                )
                .unwrap(),
            0
        );
        assert!(repeated.is_empty());
    }

    #[test]
    fn duplicate_ids_are_rejected_during_protocol_parse() {
        let mut protocol = PclMiniProtocol::new().unwrap();
        let mut scanner =
            Scanner::from_user_string("1 : : [++p] : initial\n1 : : [++q] : initial", false)
                .unwrap();
        scanner.set_format(IoFormat::Tptp);

        let error = protocol
            .parse(&mut scanner, PclMiniStepParseOptions::default())
            .unwrap_err();

        assert!(error.message().contains("duplicate PCL identifier"));
    }

    #[test]
    fn protocol_parser_accepts_zero_but_stops_before_negative_id() {
        let mut zero_protocol = PclMiniProtocol::new().unwrap();
        let mut zero_scanner = Scanner::from_user_string("0 : : [++p] : initial", false).unwrap();
        zero_scanner.set_format(IoFormat::Tptp);
        assert_eq!(
            zero_protocol
                .parse(&mut zero_scanner, PclMiniStepParseOptions::default())
                .unwrap(),
            1
        );
        assert_eq!(zero_protocol.find_step(0).unwrap().id(), 0);
        assert_eq!(zero_protocol.max_ident(), 0);

        let mut negative_protocol = PclMiniProtocol::new().unwrap();
        let initial_capacity = negative_protocol.steps.capacity();
        let mut negative_scanner =
            Scanner::from_user_string("-1 : : [++p] : initial", false).unwrap();
        negative_scanner.set_format(IoFormat::Tptp);
        assert_eq!(
            negative_protocol
                .parse(&mut negative_scanner, PclMiniStepParseOptions::default())
                .unwrap(),
            0
        );
        assert_eq!(negative_protocol.max_ident(), 0);
        assert_eq!(negative_protocol.steps.len(), 1);
        assert_eq!(negative_protocol.steps.capacity(), initial_capacity);
        assert!(negative_protocol.find_step(0).is_none());
    }

    #[test]
    fn missing_lookup_does_not_enlarge_owned_storage() {
        let protocol = PclMiniProtocol::new().unwrap();
        let initial_len = protocol.steps.len();
        let initial_capacity = protocol.steps.capacity();

        assert!(protocol.find_step(500_000).is_none());
        assert_eq!(protocol.steps.len(), initial_len);
        assert_eq!(protocol.steps.capacity(), initial_capacity);
    }

    #[test]
    fn extract_and_delete_remove_indexed_steps() {
        let (mut protocol, _) = parse_protocol("1 : : [++p] : initial");

        assert!(!protocol.delete_step(7));
        let mut duplicate = protocol.find_step(1).unwrap().clone();
        duplicate.set_property(PCL_IS_EXAMPLE);
        assert!(!protocol.insert_step(duplicate).unwrap());
        assert!(!protocol
            .find_step(1)
            .unwrap()
            .properties()
            .query(PCL_IS_EXAMPLE));

        let extracted = protocol.extract_step(1).unwrap();
        assert_eq!(extracted.id(), 1);
        assert!(protocol.find_step(1).is_none());
        assert_eq!(protocol.max_ident(), 1);
        assert!(protocol.insert_step(extracted).unwrap());
        assert!(protocol.delete_step(1));
        assert_eq!(protocol.max_ident(), 1);
        assert!(!protocol.delete_step(1));
    }

    #[test]
    fn preconditions_are_deduplicated_and_sorted_by_id() {
        let (protocol, _) = parse_protocol(
            "1 : : [++p] : initial\n3 : : [++q] : initial\n5 : : [++r] : pm(3,1)\n6 : : [++s] : pm(3,3)",
        );

        assert_eq!(
            protocol
                .collect_preconditions(protocol.find_step(5).unwrap().just())
                .unwrap(),
            [1, 3]
        );
        assert_eq!(
            protocol
                .collect_preconditions(protocol.find_step(6).unwrap().just())
                .unwrap(),
            [3]
        );
    }

    #[test]
    fn property_bulk_updates_touch_all_live_steps() {
        let (mut protocol, _) = parse_protocol("1 : : [++p] : initial\n3 : : [++q] : 1");

        protocol.set_clause_property(PCL_IS_EXAMPLE);
        assert!(protocol
            .find_step(1)
            .unwrap()
            .properties()
            .query(PCL_IS_EXAMPLE));
        assert!(protocol
            .find_step(3)
            .unwrap()
            .properties()
            .query(PCL_IS_EXAMPLE));

        protocol.delete_clause_property(PCL_IS_EXAMPLE);
        assert!(!protocol
            .find_step(1)
            .unwrap()
            .properties()
            .query(PCL_IS_EXAMPLE));
        assert!(!protocol
            .find_step(3)
            .unwrap()
            .properties()
            .query(PCL_IS_EXAMPLE));
    }

    #[test]
    fn slow_proof_marking_follows_preconditions_and_detects_empty_clause() {
        let (mut protocol, _) =
            parse_protocol("1 : : [++p] : initial\n2 : lemma : [++q] : 1\n3 : : [] : 2 : 'final'");

        assert!(protocol.mark_proof_clauses(false).unwrap());

        assert!(protocol
            .find_step(1)
            .unwrap()
            .properties()
            .query(PCL_IS_PROOF_STEP));
        assert!(protocol
            .find_step(2)
            .unwrap()
            .properties()
            .query(PCL_IS_PROOF_STEP));
        assert!(protocol
            .find_step(3)
            .unwrap()
            .properties()
            .query(PCL_IS_PROOF_STEP));
        assert_eq!(
            protocol
                .print_proof_clauses_string(
                    ProofDocOutputFormat::Pcl,
                    ProblemType::FirstOrder
                )
                .unwrap(),
            "     1 :  : [++p] : initial\n     2 : lemma : [++q] : 1\n     3 :  : [] : 2 : 'final'\n"
        );
    }

    #[test]
    fn fast_proof_marking_uses_only_contiguous_extract_suffix() {
        let (mut protocol, _) =
            parse_protocol("1 : : [++p] : initial : 'final'\n3 : : [++q] : initial : 'final'");

        assert!(!protocol.mark_proof_clauses(true).unwrap());

        assert!(!protocol
            .find_step(1)
            .unwrap()
            .properties()
            .query(PCL_IS_PROOF_STEP));
        assert!(protocol
            .find_step(3)
            .unwrap()
            .properties()
            .query(PCL_IS_PROOF_STEP));
    }

    #[test]
    fn dangling_preconditions_are_reported() {
        let (mut protocol, _) = parse_protocol("2 : : [++q] : 9 : 'proof'");

        let error = protocol.mark_proof_clauses(false).unwrap_err();

        assert!(error.message().contains("Dangling reference"));
    }

    #[test]
    fn shell_proof_marker_counts_as_empty_proof_clause() {
        let (mut protocol, _) = parse_protocol("1 : : : initial : 'proof'");

        assert!(protocol.mark_proof_clauses(false).unwrap());
        assert!(protocol.find_step(1).unwrap().is_shell());
        assert_eq!(
            protocol.find_step(1).unwrap().properties().query_type(),
            PCL_TYPE_AXIOM
        );
    }
}
