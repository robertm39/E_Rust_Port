//! Port of `PCL2/pcl_protocol`.

use std::cmp::Ordering;

use crate::basics::error::{Diagnostic, ErrorCode};
use crate::basics::simple_stuff::ProblemType;
use crate::clauses::inferencedoc::ProofDocOutputFormat;
use crate::inout::scanner::{token_pos_rep, Scanner, Token, TokenType};
use crate::pcl2::expressions::{
    pcl_step_extract, PclExprArgument, PclExpression, PclExpressionData, PclQuote,
};
use crate::pcl2::idents::PclId;
use crate::pcl2::steps::{
    pcl_step_id_compare, PclStep, PclStepParseOptions, PclStepProperties, PCL_IS_EXAMPLE,
    PCL_IS_FOF_STEP, PCL_IS_PROOF_STEP,
};
use crate::terms::signature::Signature;
use crate::terms::termbanks::TermBank;
use crate::terms::typebanks::TypeBank;

#[derive(Debug)]
pub struct PclProtocol {
    terms: TermBank,
    steps: Vec<PclStep>,
    is_ordered: bool,
}

impl PclProtocol {
    /// C `PCLProtAlloc`.
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
            steps: Vec::new(),
            is_ordered: false,
        })
    }

    #[must_use]
    pub const fn term_bank(&self) -> &TermBank {
        &self.terms
    }

    pub const fn term_bank_mut(&mut self) -> &mut TermBank {
        &mut self.terms
    }

    /// C `PCLProtStepNo`.
    #[must_use]
    pub fn step_count(&self) -> usize {
        self.steps.len()
    }

    #[must_use]
    pub fn step_ids(&mut self) -> Vec<PclId> {
        self.serialize();
        self.steps.iter().map(|step| step.id().clone()).collect()
    }

    #[must_use]
    pub(crate) fn ordered_steps(&mut self) -> &[PclStep] {
        self.serialize();
        &self.steps
    }

    /// C `PCLProtInsertStep`.
    ///
    /// Returns `Ok(false)` when a duplicate C-comparator id is already stored.
    pub fn insert_step(&mut self, step: PclStep) -> Result<bool, Diagnostic> {
        self.serialize();
        match self.find_step_index(step.id()) {
            Ok(_) => Ok(false),
            Err(index) => {
                self.steps.insert(index, step);
                self.is_ordered = true;
                Ok(true)
            }
        }
    }

    /// C `PCLProtFindStep`.
    #[must_use]
    pub fn find_step(&self, id: &PclId) -> Option<&PclStep> {
        let index = self.find_step_index(id).ok()?;
        self.steps.get(index)
    }

    #[must_use]
    pub fn find_step_mut(&mut self, id: &PclId) -> Option<&mut PclStep> {
        self.serialize();
        let index = self.find_step_index(id).ok()?;
        self.steps.get_mut(index)
    }

    /// C `PCLProtExtractStep`.
    pub fn extract_step(&mut self, id: &PclId) -> Option<PclStep> {
        self.serialize();
        let index = self.find_step_index(id).ok()?;
        self.is_ordered = false;
        Some(self.steps.remove(index))
    }

    /// C `PCLProtDeleteStep`.
    pub fn delete_step(&mut self, id: &PclId) -> bool {
        self.extract_step(id).is_some()
    }

    /// C `PCLProtSerialize`.
    pub fn serialize(&mut self) {
        if !self.is_ordered {
            self.steps.sort_by(compare_steps);
            self.is_ordered = true;
        }
    }

    /// C `PCLProtParse`.
    ///
    /// # Errors
    ///
    /// Returns scanner diagnostics for invalid protocol syntax, duplicate
    /// identifiers, or diagnostics from step parsing.
    pub fn parse(
        &mut self,
        scanner: &mut Scanner,
        options: PclStepParseOptions,
    ) -> Result<i64, Diagnostic> {
        let mut count = 0;
        while scanner.test_tok(TokenType::POS_INT) {
            let start = scanner.current_token().clone();
            let step = PclStep::parse(scanner, &mut self.terms, options)?;
            if !self.insert_step(step)? {
                return Err(duplicate_identifier_error(&start));
            }
            count += 1;
        }
        Ok(count)
    }

    /// C `PCLProtPrintExtra`.
    ///
    /// # Errors
    ///
    /// Returns diagnostics from step rendering.
    pub fn print_extra_string(
        &mut self,
        data: bool,
        format: ProofDocOutputFormat,
        problem_type: ProblemType,
    ) -> Result<String, Diagnostic> {
        self.serialize();
        let mut output = String::new();
        let terms = &mut self.terms;
        for step in &self.steps {
            output.push_str(&step.print_format_string(terms, problem_type, data, format)?);
            output.push('\n');
        }
        Ok(output)
    }

    /// C `PCLStepHasFOFParent`.
    ///
    /// # Errors
    ///
    /// Returns a syntax diagnostic when a referenced parent is missing.
    pub fn step_has_fof_parent(&self, step: &PclStep) -> Result<bool, Diagnostic> {
        Ok(self
            .collect_preconditions(step.just())?
            .into_iter()
            .any(|id| self.find_step(&id).is_some_and(PclStep::is_fof)))
    }

    /// C `PCLProtStripFOF`.
    ///
    /// # Errors
    ///
    /// Returns a syntax diagnostic when a referenced parent is missing.
    pub fn strip_fof(&mut self) -> Result<i64, Diagnostic> {
        self.serialize();
        let fof_ids: Vec<PclId> = self
            .steps
            .iter()
            .filter(|step| step.properties().query(PCL_IS_FOF_STEP))
            .map(|step| step.id().clone())
            .collect();
        let removed = i64::try_from(fof_ids.len())
            .map_err(|_| protocol_error("PCL protocol step count overflow"))?;
        if removed != 0 {
            let mut reset_ids = Vec::new();
            for step in self
                .steps
                .iter()
                .filter(|step| !step.properties().query(PCL_IS_FOF_STEP))
            {
                if self.step_has_fof_parent(step)? {
                    reset_ids.push(step.id().clone());
                }
            }
            for id in reset_ids {
                if let Some(step) = self.find_step_mut(&id) {
                    step.set_justification(PclExpression::initial(None));
                }
            }
            for id in fof_ids {
                let deleted = self.delete_step(&id);
                debug_assert!(deleted, "FOF step collected from protocol should delete");
            }
        }
        Ok(removed)
    }

    /// C `PCLProtResetTreeData`.
    pub fn reset_tree_data(&mut self, just_weights: bool) {
        self.serialize();
        for step in &mut self.steps {
            step.reset_tree_data(just_weights);
        }
    }

    /// C `PCLExprCollectPreconds`.
    ///
    /// # Errors
    ///
    /// Returns a syntax diagnostic when a quoted full step id is dangling or a
    /// mini identifier appears in a full protocol expression.
    pub fn collect_preconditions(&self, expr: &PclExpression) -> Result<Vec<PclId>, Diagnostic> {
        let mut ids = Vec::new();
        self.collect_preconditions_into(expr, &mut ids)?;
        Ok(ids)
    }

    /// C `PCLExprGetQuotedArg`.
    #[must_use]
    pub fn quoted_arg_step(&self, expr: &PclExpression, arg: usize) -> Option<&PclStep> {
        let PclExpressionData::Compound(args) = expr.data() else {
            return None;
        };
        let argument = args.get(arg)?;
        quoted_argument_id(argument).and_then(|id| self.find_step(id))
    }

    /// C `PCLProtMarkProofClauses`.
    ///
    /// # Errors
    ///
    /// Returns a syntax diagnostic when an extract-marked proof step has a
    /// dangling precondition reference.
    pub fn mark_proof_clauses(&mut self) -> Result<bool, Diagnostic> {
        self.serialize();
        let found_empty_clause = self.steps.iter().any(|step| {
            (step.is_shell() && step.extra() == Some("'proof'"))
                || (!step.is_shell() && !step.is_fof() && step.is_empty_clause())
        });
        let mut to_process: Vec<PclId> = self
            .steps
            .iter()
            .filter(|step| pcl_step_extract(step.extra()))
            .map(|step| step.id().clone())
            .collect();

        while let Some(id) = to_process.pop() {
            let just = {
                let Some(step) = self.find_step_mut(&id) else {
                    continue;
                };
                if step.properties().query(PCL_IS_PROOF_STEP) {
                    None
                } else {
                    step.set_property(PCL_IS_PROOF_STEP);
                    Some(step.just().clone())
                }
            };
            if let Some(just) = just {
                for parent in self.collect_preconditions(&just)? {
                    to_process.push(parent);
                }
            }
        }
        Ok(found_empty_clause)
    }

    /// C `PCLProtSetProp`.
    pub fn set_property(&mut self, properties: PclStepProperties) {
        self.serialize();
        for step in &mut self.steps {
            step.set_property(properties);
        }
    }

    /// C `PCLProtDelProp`.
    pub fn delete_property(&mut self, properties: PclStepProperties) {
        self.serialize();
        for step in &mut self.steps {
            step.delete_property(properties);
        }
    }

    /// C `PCLProtCountProp`.
    #[must_use]
    pub fn count_property(&mut self, properties: PclStepProperties) -> i64 {
        self.serialize();
        self.steps
            .iter()
            .filter(|step| step.properties().query(properties))
            .count()
            .try_into()
            .unwrap_or(i64::MAX)
    }

    /// C `PCLProtCollectPropSteps`.
    #[must_use]
    pub fn collect_property_step_ids(&mut self, properties: PclStepProperties) -> Vec<PclId> {
        self.serialize();
        self.steps
            .iter()
            .filter(|step| step.properties().query(properties))
            .map(|step| step.id().clone())
            .collect()
    }

    /// C `PCLProtPrintPropClauses`.
    ///
    /// # Errors
    ///
    /// Returns diagnostics from step rendering.
    pub fn print_property_steps_string(
        &mut self,
        property: PclStepProperties,
        format: ProofDocOutputFormat,
        problem_type: ProblemType,
    ) -> Result<String, Diagnostic> {
        self.serialize();
        let mut output = String::new();
        let terms = &mut self.terms;
        for step in &self.steps {
            if step.properties().query(property) {
                output.push_str(&step.print_format_string(terms, problem_type, false, format)?);
                output.push('\n');
            }
        }
        Ok(output)
    }

    /// C `PCLProtPrintExamples`.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if the protocol index cannot be represented as a C
    /// `long`-shaped Rust `i64`.
    pub fn print_examples_string(&mut self) -> Result<String, Diagnostic> {
        let proof_steps = self.count_property(PCL_IS_PROOF_STEP);
        let total_steps = i64::try_from(self.steps.len())
            .map_err(|_| protocol_error("PCL protocol step count overflow"))?;
        let mut output = String::new();
        for (index, step) in self.steps.iter().enumerate() {
            if step.properties().query(PCL_IS_EXAMPLE) {
                let id = i64::try_from(index)
                    .map_err(|_| protocol_error("PCL protocol index overflow"))?;
                output.push_str(&step.print_example_string(
                    &self.terms,
                    id,
                    proof_steps,
                    total_steps,
                ));
                output.push('\n');
            }
        }
        Ok(output)
    }

    fn collect_preconditions_into(
        &self,
        expr: &PclExpression,
        ids: &mut Vec<PclId>,
    ) -> Result<(), Diagnostic> {
        match expr.data() {
            PclExpressionData::None => Err(protocol_error("Unknown PCL expression in protocol")),
            PclExpressionData::Initial(_) => Ok(()),
            PclExpressionData::Quote {
                quote: PclQuote::Full(id),
                ..
            } => {
                if self.find_step(id).is_none() {
                    return Err(protocol_error("Dangling reference in PCL protocol!"));
                }
                insert_unique_id(ids, id.clone());
                Ok(())
            }
            PclExpressionData::Quote {
                quote: PclQuote::Mini(_),
                ..
            } => Err(protocol_error(
                "Mini PCL identifier found in full protocol expression",
            )),
            PclExpressionData::Compound(args) => {
                for argument in args {
                    self.collect_preconditions_into(argument.expr(), ids)?;
                }
                Ok(())
            }
        }
    }

    fn find_step_index(&self, id: &PclId) -> Result<usize, usize> {
        self.steps
            .binary_search_by(|step| compare_step_id(step.id(), id))
    }
}

fn compare_steps(left: &PclStep, right: &PclStep) -> Ordering {
    compare_c_to_ordering(pcl_step_id_compare(left, right))
}

fn compare_step_id(left: &PclId, right: &PclId) -> Ordering {
    compare_c_to_ordering(left.compare_c_value(right))
}

fn compare_c_to_ordering(value: i32) -> Ordering {
    value.cmp(&0)
}

fn insert_unique_id(ids: &mut Vec<PclId>, id: PclId) {
    match ids.binary_search_by(|probe| compare_step_id(probe, &id)) {
        Ok(_) => {}
        Err(index) => ids.insert(index, id),
    }
}

fn quoted_argument_id(argument: &PclExprArgument) -> Option<&PclId> {
    match argument.expr().data() {
        PclExpressionData::Quote {
            quote: PclQuote::Full(id),
            ..
        } => Some(id),
        _ => None,
    }
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

fn protocol_error(message: &str) -> Diagnostic {
    Diagnostic::new(ErrorCode::SYNTAX_ERROR, message)
}

#[cfg(test)]
mod tests {
    use super::PclProtocol;
    use crate::basics::simple_stuff::ProblemType;
    use crate::clauses::inferencedoc::ProofDocOutputFormat;
    use crate::inout::scanner::{IoFormat, Scanner};
    use crate::pcl2::idents::PclId;
    use crate::pcl2::steps::{
        PclStepParseOptions, PCL_IS_EXAMPLE, PCL_IS_MARKED, PCL_IS_PROOF_STEP,
    };

    fn parse_id(source: &str) -> PclId {
        let mut scanner = Scanner::from_user_string(source, false).unwrap();
        PclId::parse(&mut scanner).unwrap()
    }

    fn parse_protocol(source: &str) -> (PclProtocol, Scanner) {
        let mut protocol = PclProtocol::new().unwrap();
        let mut scanner = Scanner::from_user_string(source, false).unwrap();
        scanner.set_format(IoFormat::Tptp);
        let count = protocol
            .parse(
                &mut scanner,
                PclStepParseOptions {
                    problem_type: ProblemType::FirstOrder,
                    support_shell_pcl: true,
                },
            )
            .unwrap();
        assert!(count > 0);
        (protocol, scanner)
    }

    #[test]
    fn parses_stores_and_prints_steps_in_identifier_order() {
        let (mut protocol, scanner) =
            parse_protocol("2 : : [++q] : initial\n1 : lemma : [++p] : 2 tail");

        assert_eq!(protocol.step_count(), 2);
        assert_eq!(
            protocol.find_step(&parse_id("1")).unwrap().id().elements(),
            [1]
        );
        assert_eq!(
            protocol.find_step(&parse_id("2")).unwrap().id().elements(),
            [2]
        );
        assert_eq!(scanner.current_token().literal(), "tail");

        assert_eq!(
            protocol
                .print_extra_string(false, ProofDocOutputFormat::Pcl, ProblemType::FirstOrder)
                .unwrap(),
            "      1 : lemma : [++p] : 2 : 'lemma'\n      2 :  : [++q] : initial\n"
        );
    }

    #[test]
    fn duplicate_ids_are_rejected_during_protocol_parse() {
        let mut protocol = PclProtocol::new().unwrap();
        let mut scanner =
            Scanner::from_user_string("1 : : [++p] : initial\n1 : : [++q] : initial", false)
                .unwrap();
        scanner.set_format(IoFormat::Tptp);

        let error = protocol
            .parse(&mut scanner, PclStepParseOptions::default())
            .unwrap_err();

        assert!(error.message().contains("duplicate PCL identifier"));
    }

    #[test]
    fn extract_and_delete_remove_steps_by_full_identifier() {
        let (mut protocol, _) = parse_protocol("1.2 : : [++p] : initial");
        let id = parse_id("1.2");

        assert!(!protocol.delete_step(&parse_id("7")));
        let extracted = protocol.extract_step(&id).unwrap();
        assert_eq!(extracted.id().elements(), [1, 2]);
        assert!(protocol.find_step(&id).is_none());
        assert!(protocol.insert_step(extracted).unwrap());
    }

    #[test]
    fn proof_marking_follows_full_preconditions_and_detects_empty_clause() {
        let (mut protocol, _) =
            parse_protocol("1 : : [++p] : initial\n2 : lemma : [++q] : 1\n3 : : [] : 2 : 'final'");

        assert!(protocol.mark_proof_clauses().unwrap());

        assert_eq!(protocol.count_property(PCL_IS_PROOF_STEP), 3);
        assert_eq!(
            protocol
                .print_property_steps_string(
                    PCL_IS_PROOF_STEP,
                    ProofDocOutputFormat::Pcl,
                    ProblemType::FirstOrder,
                )
                .unwrap(),
            "      1 :  : [++p] : initial\n      2 : lemma : [++q] : 1 : 'lemma'\n      3 :  : [] : 2 : 'final'\n"
        );
    }

    #[test]
    fn dangling_preconditions_are_reported() {
        let (mut protocol, _) = parse_protocol("2 : : [++q] : 9 : 'proof'");

        let error = protocol.mark_proof_clauses().unwrap_err();

        assert!(error.message().contains("Dangling reference"));
    }

    #[test]
    fn quoted_arg_step_returns_only_direct_quote_arguments() {
        let (protocol, _) = parse_protocol("1 : : [++p] : initial\n2 : : [++q] : rw(1,1)");
        let step = protocol.find_step(&parse_id("2")).unwrap();

        assert_eq!(
            protocol
                .quoted_arg_step(step.just(), 0)
                .unwrap()
                .id()
                .elements(),
            [1]
        );
        assert!(protocol.quoted_arg_step(step.just(), 99).is_none());
    }

    #[test]
    fn strip_fof_deletes_formula_steps_and_initializes_clause_dependents() {
        let (mut protocol, _) =
            parse_protocol("1 : : p(a) : initial\n2 : : [++q] : 1\n3 : : [++r] : 2");

        assert_eq!(protocol.strip_fof().unwrap(), 1);
        assert!(protocol.find_step(&parse_id("1")).is_none());
        assert_eq!(protocol.step_count(), 2);
        assert_eq!(
            protocol
                .find_step(&parse_id("2"))
                .unwrap()
                .just()
                .print_string(false),
            "initial"
        );
        assert_eq!(
            protocol
                .find_step(&parse_id("3"))
                .unwrap()
                .just()
                .print_string(false),
            "2"
        );
    }

    #[test]
    fn property_bulk_operations_count_collect_and_print_examples() {
        let (mut protocol, _) = parse_protocol("1 : : [++p] : initial\n2 : : [++q] : 1");
        protocol.set_property(PCL_IS_EXAMPLE | PCL_IS_MARKED);
        assert_eq!(protocol.count_property(PCL_IS_EXAMPLE), 2);
        assert_eq!(protocol.collect_property_step_ids(PCL_IS_MARKED).len(), 2);

        protocol
            .find_step_mut(&parse_id("1"))
            .unwrap()
            .tree_data_mut()
            .proof_distance = 4;
        protocol
            .find_step_mut(&parse_id("2"))
            .unwrap()
            .set_property(PCL_IS_PROOF_STEP);
        let examples = protocol.print_examples_string().unwrap();
        assert!(examples.contains("   0:(4, 0.000000,0.000000,0.000000,0.000000):p <- ."));
        assert!(examples.contains("   1:(-1, 0.000000,0.000000,0.000000,0.000000):q <- ."));

        protocol.delete_property(PCL_IS_MARKED);
        assert_eq!(protocol.count_property(PCL_IS_MARKED), 0);
    }
}
