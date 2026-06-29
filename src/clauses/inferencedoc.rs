use std::fmt;

use crate::basics::defines::DEFAULT_COMCHAR_RAW;
use crate::basics::error::{Diagnostic, ErrorCode};
use crate::basics::pstacks::PStack;
use crate::basics::simple_stuff::ProblemType;
use crate::clauses::clause::{
    clause_write_pcl_with_options, clause_write_tstp_with_type_suffixes, Clause,
};
use crate::clauses::clause_props::{
    FormulaProperties, CP_INPUT_FORMULA, CP_TYPE_AXIOM, CP_TYPE_CONJECTURE, CP_TYPE_HYPOTHESIS,
    CP_TYPE_LEMMA, CP_TYPE_NEG_CONJECTURE, CP_TYPE_QUESTION, CP_WATCH_ONLY,
};
use crate::clauses::clauseinfo::{source_info_pcl_string, source_info_tstp_string, ClauseInfo};
use crate::clauses::clausepos::{term_compute_rw_sequence, ClausePos, RewriteSequenceEntry};
use crate::clauses::eqn::EqnPrintOptions;
use crate::terms::termbanks::TermBank;
use crate::terms::termtypes::Term;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PclStepPrintOptions {
    pub full_terms: bool,
    pub compact: bool,
    pub print_types: bool,
    pub eqn_print_options: EqnPrintOptions,
}

impl Default for PclStepPrintOptions {
    fn default() -> Self {
        Self {
            full_terms: true,
            compact: false,
            print_types: false,
            eqn_print_options: EqnPrintOptions::tptp(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i32)]
pub enum ProofDocOutputFormat {
    NoFormat = 0,
    Lop = 1,
    Pcl = 2,
    Tstp = 3,
    Tptp = 4,
    Xml = 5,
}

impl ProofDocOutputFormat {
    #[must_use]
    pub const fn c_value(self) -> i32 {
        self as i32
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClauseCreationInference {
    Initial,
    Paramodulation,
    SimultaneousParamodulation,
    EqualityResolution,
    EqualityFactoring,
    Factoring,
    Split,
}

impl ClauseCreationInference {
    #[must_use]
    pub const fn binary(self) -> Option<ClauseBinaryInference> {
        match self {
            Self::Paramodulation => Some(ClauseBinaryInference::Paramodulation),
            Self::SimultaneousParamodulation => {
                Some(ClauseBinaryInference::SimultaneousParamodulation)
            }
            Self::Initial
            | Self::EqualityResolution
            | Self::EqualityFactoring
            | Self::Factoring
            | Self::Split => None,
        }
    }

    #[must_use]
    pub const fn unary(self) -> Option<ClauseUnaryInference> {
        match self {
            Self::EqualityResolution => Some(ClauseUnaryInference::EqualityResolution),
            Self::EqualityFactoring => Some(ClauseUnaryInference::EqualityFactoring),
            Self::Factoring => Some(ClauseUnaryInference::Factoring),
            Self::Split => Some(ClauseUnaryInference::Split),
            Self::Initial | Self::Paramodulation | Self::SimultaneousParamodulation => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClauseModificationInference {
    SimplifyReflect,
    ContextSimplifyReflect,
    Condense,
    Minimize,
    EvalAnswerLiteral,
    DestructiveEqualityResolution,
    AcResolution,
}

impl ClauseModificationInference {
    #[must_use]
    pub const fn binary(self) -> Option<ClauseBinaryInference> {
        match self {
            Self::SimplifyReflect => Some(ClauseBinaryInference::SimplifyReflect),
            Self::ContextSimplifyReflect => Some(ClauseBinaryInference::ContextSimplifyReflect),
            Self::Condense
            | Self::Minimize
            | Self::EvalAnswerLiteral
            | Self::DestructiveEqualityResolution
            | Self::AcResolution => None,
        }
    }

    #[must_use]
    pub const fn unary(self) -> Option<ClauseUnaryInference> {
        match self {
            Self::Condense => Some(ClauseUnaryInference::Condense),
            Self::Minimize => Some(ClauseUnaryInference::Normalize),
            Self::EvalAnswerLiteral => Some(ClauseUnaryInference::EvalAnswerLiteral),
            Self::DestructiveEqualityResolution => Some(ClauseUnaryInference::EqualityResolution),
            Self::SimplifyReflect | Self::ContextSimplifyReflect | Self::AcResolution => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FormulaCreationInference {
    Initial,
    IntroDef,
    SplitEquiv,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FormulaModificationInference {
    Simplification,
    NegConjecture,
    Nnf,
    ShiftQuantors,
    VarRename,
    Skolemize,
    Distribute,
    AnnotateQuestion,
    Other,
}

impl FormulaModificationInference {
    #[must_use]
    pub const fn parent_inference(self) -> Option<FormulaParentInference> {
        match self {
            Self::Simplification => Some(FormulaParentInference::Simplification),
            Self::NegConjecture => Some(FormulaParentInference::NegConjecture),
            Self::Nnf => Some(FormulaParentInference::Nnf),
            Self::ShiftQuantors => Some(FormulaParentInference::ShiftQuantors),
            Self::VarRename => Some(FormulaParentInference::VarRename),
            Self::Skolemize => Some(FormulaParentInference::Skolemize),
            Self::Distribute => Some(FormulaParentInference::Distribute),
            Self::AnnotateQuestion => Some(FormulaParentInference::AnnotateQuestion),
            Self::Other => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ClauseModificationEvidence<'a> {
    pub partner: Option<&'a Clause>,
    pub ac_axiom_ids: &'a [i64],
}

impl<'a> ClauseModificationEvidence<'a> {
    #[must_use]
    pub const fn none() -> Self {
        Self {
            partner: None,
            ac_axiom_ids: &[],
        }
    }

    #[must_use]
    pub const fn partner(partner: &'a Clause) -> Self {
        Self {
            partner: Some(partner),
            ac_axiom_ids: &[],
        }
    }

    #[must_use]
    pub const fn ac_resolution(ac_axiom_ids: &'a [i64]) -> Self {
        Self {
            partner: None,
            ac_axiom_ids,
        }
    }
}

impl Default for ClauseModificationEvidence<'_> {
    fn default() -> Self {
        Self::none()
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FormulaCreationParents<'a> {
    pub parent1: Option<&'a FormulaDocView<'a>>,
    pub parent2: Option<&'a FormulaDocView<'a>>,
}

impl<'a> FormulaCreationParents<'a> {
    #[must_use]
    pub const fn none() -> Self {
        Self {
            parent1: None,
            parent2: None,
        }
    }

    #[must_use]
    pub const fn unary(parent: &'a FormulaDocView<'a>) -> Self {
        Self {
            parent1: Some(parent),
            parent2: None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FormulaDocView<'a> {
    ident: i64,
    properties: FormulaProperties,
    rendered_formula: &'a str,
    info: Option<&'a ClauseInfo>,
    is_untyped: bool,
}

impl<'a> FormulaDocView<'a> {
    #[must_use]
    pub const fn new(ident: i64, properties: FormulaProperties, rendered_formula: &'a str) -> Self {
        Self {
            ident,
            properties,
            rendered_formula,
            info: None,
            is_untyped: true,
        }
    }

    #[must_use]
    pub const fn with_info(mut self, info: &'a ClauseInfo) -> Self {
        self.info = Some(info);
        self
    }

    #[must_use]
    pub const fn with_untyped(mut self, is_untyped: bool) -> Self {
        self.is_untyped = is_untyped;
        self
    }

    #[must_use]
    pub const fn ident(&self) -> i64 {
        self.ident
    }

    pub fn set_ident(&mut self, ident: i64) {
        self.ident = ident;
    }

    #[must_use]
    pub const fn rendered_formula(&self) -> &'a str {
        self.rendered_formula
    }

    #[must_use]
    pub const fn properties(&self) -> FormulaProperties {
        self.properties
    }

    pub fn set_prop(&mut self, prop: FormulaProperties) {
        self.properties.set(prop);
    }

    pub fn del_prop(&mut self, prop: FormulaProperties) {
        self.properties.delete(prop);
    }

    #[must_use]
    pub const fn query_prop(&self, prop: FormulaProperties) -> bool {
        self.properties.query(prop)
    }

    #[must_use]
    pub const fn query_tptp_type(&self) -> FormulaProperties {
        self.properties.query_tptp_type()
    }

    #[must_use]
    pub const fn info(&self) -> Option<&'a ClauseInfo> {
        self.info
    }

    #[must_use]
    pub const fn is_untyped(&self) -> bool {
        self.is_untyped
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ClauseCreationParents<'a> {
    pub parent1: Option<&'a Clause>,
    pub parent2: Option<&'a Clause>,
}

impl<'a> ClauseCreationParents<'a> {
    #[must_use]
    pub const fn none() -> Self {
        Self {
            parent1: None,
            parent2: None,
        }
    }

    #[must_use]
    pub const fn unary(parent: &'a Clause) -> Self {
        Self {
            parent1: Some(parent),
            parent2: None,
        }
    }

    #[must_use]
    pub const fn binary(parent1: &'a Clause, parent2: &'a Clause) -> Self {
        Self {
            parent1: Some(parent1),
            parent2: Some(parent2),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProofDocWriteResult {
    pub printed: bool,
    pub stdout_before: &'static str,
    pub stdout_after: &'static str,
    pub stdout_after_offset: Option<usize>,
}

impl ProofDocWriteResult {
    #[must_use]
    pub const fn suppressed() -> Self {
        Self {
            printed: false,
            stdout_before: "",
            stdout_after: "",
            stdout_after_offset: None,
        }
    }

    #[must_use]
    pub const fn not_printed() -> Self {
        Self {
            printed: false,
            stdout_before: "",
            stdout_after: "",
            stdout_after_offset: None,
        }
    }

    #[must_use]
    pub const fn printed() -> Self {
        Self {
            printed: true,
            stdout_before: "",
            stdout_after: "",
            stdout_after_offset: None,
        }
    }

    #[must_use]
    pub const fn pcl_initial(stdout_after_offset: usize) -> Self {
        Self {
            printed: true,
            stdout_before: "XX\n",
            stdout_after: "XX\n",
            stdout_after_offset: Some(stdout_after_offset),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProofDocIdSource {
    current_ident: i64,
}

impl Default for ProofDocIdSource {
    fn default() -> Self {
        Self::new()
    }
}

impl ProofDocIdSource {
    #[must_use]
    pub const fn new() -> Self {
        Self { current_ident: 0 }
    }

    #[must_use]
    pub const fn from_current(current_ident: i64) -> Self {
        Self { current_ident }
    }

    #[must_use]
    pub const fn current_ident(&self) -> i64 {
        self.current_ident
    }

    pub fn next_ident(&mut self) -> i64 {
        self.current_ident = self.current_ident.saturating_add(1);
        self.current_ident
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProofDocSession {
    pub output_format: ProofDocOutputFormat,
    pub output_level: i64,
    pub problem_type: ProblemType,
    pub pcl_shell_level: i32,
    pub step_options: PclStepPrintOptions,
    pub id_source: ProofDocIdSource,
}

#[derive(Clone, Copy, Debug)]
struct ClauseModificationRender<'a> {
    clause: &'a Clause,
    old_id: i64,
    inference: ClauseModificationInference,
    evidence: ClauseModificationEvidence<'a>,
    comment: Option<&'a str>,
}

impl ProofDocSession {
    #[must_use]
    pub fn new(
        output_format: ProofDocOutputFormat,
        output_level: i64,
        problem_type: ProblemType,
    ) -> Self {
        Self {
            output_format,
            output_level,
            problem_type,
            pcl_shell_level: 0,
            step_options: PclStepPrintOptions::default(),
            id_source: ProofDocIdSource::new(),
        }
    }

    /// Ports the C `DocClauseCreation` dispatch for represented clause
    /// creation inferences.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if TSTP clause rendering reports an unsupported
    /// clause shape.
    ///
    /// # Panics
    ///
    /// Panics if the supplied parent shape does not match the requested
    /// inference, matching C assertions in `DocClauseCreation`.
    pub fn doc_clause_creation(
        &mut self,
        output: &mut impl fmt::Write,
        bank: &TermBank,
        clause: &mut Clause,
        inference: ClauseCreationInference,
        parent_refs: ClauseCreationParents<'_>,
        comment: Option<&str>,
    ) -> Result<ProofDocWriteResult, Diagnostic> {
        if self.output_level < 2 {
            return Ok(ProofDocWriteResult::suppressed());
        }

        clause.set_ident(self.id_source.next_ident());

        match self.output_format {
            ProofDocOutputFormat::Pcl => self.write_pcl_clause_creation(
                output,
                bank,
                clause,
                inference,
                parent_refs,
                comment,
            ),
            ProofDocOutputFormat::Tstp => self.write_tstp_clause_creation(
                output,
                bank,
                clause,
                inference,
                parent_refs,
                comment,
            ),
            ProofDocOutputFormat::NoFormat
            | ProofDocOutputFormat::Lop
            | ProofDocOutputFormat::Tptp
            | ProofDocOutputFormat::Xml => {
                write_unsupported_doc_format(output).map_err(doc_write_error)?;
                Ok(ProofDocWriteResult::printed())
            }
        }
    }

    /// Ports C `DocClauseModification` for represented clause modification
    /// inferences.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if TSTP clause rendering reports an unsupported
    /// clause shape.
    ///
    /// # Panics
    ///
    /// Panics if the supplied partner shape does not match the requested
    /// inference, matching C assertions in `DocClauseModification`.
    pub fn doc_clause_modification(
        &mut self,
        output: &mut impl fmt::Write,
        bank: &TermBank,
        clause: &mut Clause,
        inference: ClauseModificationInference,
        partner: Option<&Clause>,
        comment: Option<&str>,
    ) -> Result<ProofDocWriteResult, Diagnostic> {
        let evidence = partner.map_or_else(
            ClauseModificationEvidence::none,
            ClauseModificationEvidence::partner,
        );
        self.doc_clause_modification_with_evidence(
            output, bank, clause, inference, evidence, comment,
        )
    }

    /// Ports C `DocClauseModification` with branch-specific evidence.
    ///
    /// Use this form for AC-resolution so the signature-owned AC axiom stack
    /// can be passed as stable clause ids.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if TSTP clause rendering reports an unsupported
    /// clause shape.
    ///
    /// # Panics
    ///
    /// Panics if the supplied evidence shape does not match the requested
    /// inference, matching C assertions in `DocClauseModification`.
    pub fn doc_clause_modification_with_evidence(
        &mut self,
        output: &mut impl fmt::Write,
        bank: &TermBank,
        clause: &mut Clause,
        inference: ClauseModificationInference,
        evidence: ClauseModificationEvidence<'_>,
        comment: Option<&str>,
    ) -> Result<ProofDocWriteResult, Diagnostic> {
        clause.del_prop(CP_INPUT_FORMULA);
        if self.output_level < 2 {
            return Ok(ProofDocWriteResult::suppressed());
        }

        let old_id = clause.ident();
        clause.set_ident(self.id_source.next_ident());

        match self.output_format {
            ProofDocOutputFormat::Pcl => self.write_pcl_clause_modification(
                output,
                bank,
                ClauseModificationRender {
                    clause,
                    old_id,
                    inference,
                    evidence,
                    comment,
                },
            ),
            ProofDocOutputFormat::Tstp => self.write_tstp_clause_modification(
                output,
                bank,
                ClauseModificationRender {
                    clause,
                    old_id,
                    inference,
                    evidence,
                    comment,
                },
            ),
            ProofDocOutputFormat::NoFormat
            | ProofDocOutputFormat::Lop
            | ProofDocOutputFormat::Tptp
            | ProofDocOutputFormat::Xml => {
                write_unsupported_doc_format(output).map_err(doc_write_error)?;
                Ok(ProofDocWriteResult::printed())
            }
        }
    }

    /// Ports the represented cases of C `DocFormulaCreation`.
    ///
    /// The formula text is supplied pre-rendered until the Rust port has a
    /// full `WFormula` owner.
    ///
    /// # Panics
    ///
    /// Panics if the supplied parent shape does not match the requested
    /// inference, matching C assertions in `DocFormulaCreation`.
    pub fn doc_formula_creation(
        &mut self,
        output: &mut impl fmt::Write,
        formula: &mut FormulaDocView<'_>,
        inference: FormulaCreationInference,
        parent_refs: FormulaCreationParents<'_>,
        comment: Option<&str>,
    ) -> Result<ProofDocWriteResult, Diagnostic> {
        if self.output_level < 2 {
            return Ok(ProofDocWriteResult::suppressed());
        }

        formula.set_ident(self.id_source.next_ident());

        match self.output_format {
            ProofDocOutputFormat::Pcl => {
                self.write_pcl_formula_creation(output, formula, inference, parent_refs, comment)
            }
            ProofDocOutputFormat::Tstp => {
                self.write_tstp_formula_creation(output, formula, inference, parent_refs, comment)
            }
            ProofDocOutputFormat::NoFormat
            | ProofDocOutputFormat::Lop
            | ProofDocOutputFormat::Tptp
            | ProofDocOutputFormat::Xml => {
                write_unsupported_doc_format(output).map_err(doc_write_error)?;
                Ok(ProofDocWriteResult::printed())
            }
        }
    }

    /// Ports the represented cases of C `DocFormulaModification`.
    ///
    /// The `Other` variant preserves C's default branch: after clearing
    /// `CPInputFormula` and assigning a new id, it prints nothing.
    pub fn doc_formula_modification(
        &mut self,
        output: &mut impl fmt::Write,
        formula: &mut FormulaDocView<'_>,
        inference: FormulaModificationInference,
        comment: Option<&str>,
    ) -> Result<ProofDocWriteResult, Diagnostic> {
        formula.del_prop(CP_INPUT_FORMULA);
        if self.output_level < 2 {
            return Ok(ProofDocWriteResult::suppressed());
        }

        let old_id = formula.ident();
        formula.set_ident(self.id_source.next_ident());
        let Some(parent_inference) = inference.parent_inference() else {
            return Ok(ProofDocWriteResult::not_printed());
        };

        match self.output_format {
            ProofDocOutputFormat::Pcl => self.write_pcl_formula_modification(
                output,
                formula,
                parent_inference,
                old_id,
                comment,
            ),
            ProofDocOutputFormat::Tstp => self.write_tstp_formula_modification(
                output,
                formula,
                parent_inference,
                old_id,
                comment,
            ),
            ProofDocOutputFormat::NoFormat
            | ProofDocOutputFormat::Lop
            | ProofDocOutputFormat::Tptp
            | ProofDocOutputFormat::Xml => {
                write_unsupported_doc_format(output).map_err(doc_write_error)?;
                Ok(ProofDocWriteResult::printed())
            }
        }
    }

    /// Ports C `DocClauseFromForm`.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if TSTP clause rendering reports an unsupported
    /// clause shape.
    pub fn doc_clause_from_form(
        &mut self,
        output: &mut impl fmt::Write,
        bank: &TermBank,
        clause: &mut Clause,
        parent: &FormulaDocView<'_>,
    ) -> Result<ProofDocWriteResult, Diagnostic> {
        clause.del_prop(CP_INPUT_FORMULA);
        if self.output_level < 2 {
            return Ok(ProofDocWriteResult::suppressed());
        }

        match self.output_format {
            ProofDocOutputFormat::Pcl => {
                clause.set_ident(self.id_source.next_ident());
                pcl_print_start(
                    output,
                    bank,
                    clause,
                    self.pcl_shell_level < 1,
                    self.step_options,
                )
                .map_err(doc_write_error)?;
                write_pcl_clause_unary_inference(
                    output,
                    ClauseUnaryInference::SplitConjunct,
                    parent.ident(),
                )
                .map_err(doc_write_error)?;
                pcl_print_end(output, clause, None, self.step_options).map_err(doc_write_error)?;
                Ok(ProofDocWriteResult::printed())
            }
            ProofDocOutputFormat::Tstp => {
                clause.set_ident(self.id_source.next_ident());
                clause_write_tstp_with_type_suffixes(
                    output,
                    bank,
                    clause,
                    self.step_options.full_terms,
                    false,
                    self.problem_type,
                    self.step_options.print_types,
                )?;
                output.write_char(',').map_err(doc_write_error)?;
                write_tstp_clause_unary_inference(
                    output,
                    ClauseUnaryInference::SplitConjunct,
                    parent.ident(),
                )
                .map_err(doc_write_error)?;
                tstp_print_end(output, clause, None).map_err(doc_write_error)?;
                Ok(ProofDocWriteResult::printed())
            }
            ProofDocOutputFormat::NoFormat
            | ProofDocOutputFormat::Lop
            | ProofDocOutputFormat::Tptp
            | ProofDocOutputFormat::Xml => {
                write_unsupported_doc_format(output).map_err(doc_write_error)?;
                Ok(ProofDocWriteResult::printed())
            }
        }
    }

    /// Ports C `DocClauseQuote`.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if TSTP clause rendering reports an unsupported
    /// clause shape.
    ///
    /// # Panics
    ///
    /// Panics if an optional partner is provided without a comment, matching
    /// C's assertion.
    pub fn doc_clause_quote(
        &mut self,
        output: &mut impl fmt::Write,
        bank: &TermBank,
        target_level: i64,
        clause: &mut Clause,
        comment: Option<&str>,
        opt_partner: Option<&Clause>,
    ) -> Result<ProofDocWriteResult, Diagnostic> {
        clause.del_prop(CP_INPUT_FORMULA);
        let old_id = clause.ident();
        if self.output_level < target_level {
            return Ok(ProofDocWriteResult::suppressed());
        }

        match self.output_format {
            ProofDocOutputFormat::Pcl => {
                clause.set_ident(self.id_source.next_ident());
                pcl_print_start(
                    output,
                    bank,
                    clause,
                    self.pcl_shell_level < 1,
                    self.step_options,
                )
                .map_err(doc_write_error)?;
                write!(output, "{old_id}").map_err(doc_write_error)?;
                if let Some(partner) = opt_partner {
                    let Some(comment) = comment else {
                        panic!("clause quote with partner needs comment");
                    };
                    writeln!(output, " : '{comment}({})'", partner.ident())
                        .map_err(doc_write_error)?;
                } else {
                    pcl_print_end(output, clause, comment, self.step_options)
                        .map_err(doc_write_error)?;
                }
                Ok(ProofDocWriteResult::printed())
            }
            ProofDocOutputFormat::Tstp => {
                clause.set_ident(self.id_source.next_ident());
                clause_write_tstp_with_type_suffixes(
                    output,
                    bank,
                    clause,
                    self.step_options.full_terms,
                    false,
                    self.problem_type,
                    self.step_options.print_types,
                )?;
                write!(output, ", c_0_{old_id}").map_err(doc_write_error)?;
                if let Some(partner) = opt_partner {
                    let Some(comment) = comment else {
                        panic!("clause quote with partner needs comment");
                    };
                    writeln!(output, ",['{comment}(c_0_{})']).", partner.ident())
                        .map_err(doc_write_error)?;
                } else if let Some(comment) = comment {
                    writeln!(output, ",['{comment}']).").map_err(doc_write_error)?;
                } else {
                    output.write_str(").\n").map_err(doc_write_error)?;
                }
                Ok(ProofDocWriteResult::printed())
            }
            ProofDocOutputFormat::NoFormat
            | ProofDocOutputFormat::Lop
            | ProofDocOutputFormat::Tptp
            | ProofDocOutputFormat::Xml => {
                write_unsupported_doc_format(output).map_err(doc_write_error)?;
                Ok(ProofDocWriteResult::printed())
            }
        }
    }

    /// Ports C `DocClauseRewrite`.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if TSTP clause rendering reports an unsupported
    /// clause shape.
    ///
    /// # Panics
    ///
    /// Panics if the clause position is not clause-backed, lacks a literal, or
    /// if the term rewrite chain does not connect `old_term` to the selected
    /// side, matching C assertions in `DocClauseRewrite` and
    /// `TermComputeRWSequence`.
    pub fn doc_clause_rewrite<T>(
        &mut self,
        output: &mut impl fmt::Write,
        bank: &TermBank,
        rewritten: &mut ClausePos<T>,
        old_term: &Term,
        comment: Option<&str>,
    ) -> Result<ProofDocWriteResult, Diagnostic> {
        rewritten
            .clause_mut()
            .expect("clause rewrite documentation needs clause")
            .del_prop(CP_INPUT_FORMULA);
        if self.output_level < 2 {
            return Ok(ProofDocWriteResult::suppressed());
        }

        assert!(
            rewritten.literal().is_some(),
            "clause rewrite documentation needs literal"
        );
        let normal_form = rewritten
            .get_side()
            .expect("clause rewrite documentation needs selected side");
        let old_id = rewritten
            .clause()
            .expect("clause rewrite documentation needs clause")
            .ident();
        rewritten
            .clause_mut()
            .expect("clause rewrite documentation needs clause")
            .set_ident(self.id_source.next_ident());
        let demodulator_ids = compute_rewrite_demodulator_ids(old_term, &normal_form);

        match self.output_format {
            ProofDocOutputFormat::Pcl => {
                let clause = rewritten
                    .clause()
                    .expect("clause rewrite documentation needs clause");
                pcl_print_start(
                    output,
                    bank,
                    clause,
                    self.pcl_shell_level < 1,
                    self.step_options,
                )
                .map_err(doc_write_error)?;
                write_pcl_clause_rewrite_inference(output, old_id, &demodulator_ids)
                    .map_err(doc_write_error)?;
                pcl_print_end(output, clause, comment, self.step_options)
                    .map_err(doc_write_error)?;
                Ok(ProofDocWriteResult::printed())
            }
            ProofDocOutputFormat::Tstp => {
                let clause = rewritten
                    .clause()
                    .expect("clause rewrite documentation needs clause");
                clause_write_tstp_with_type_suffixes(
                    output,
                    bank,
                    clause,
                    self.step_options.full_terms,
                    false,
                    self.problem_type,
                    self.step_options.print_types,
                )?;
                output.write_char(',').map_err(doc_write_error)?;
                write_tstp_clause_rewrite_inference(output, old_id, &demodulator_ids)
                    .map_err(doc_write_error)?;
                tstp_print_end(output, clause, comment).map_err(doc_write_error)?;
                Ok(ProofDocWriteResult::printed())
            }
            ProofDocOutputFormat::NoFormat
            | ProofDocOutputFormat::Lop
            | ProofDocOutputFormat::Tptp
            | ProofDocOutputFormat::Xml => {
                write_unsupported_doc_format(output).map_err(doc_write_error)?;
                Ok(ProofDocWriteResult::printed())
            }
        }
    }

    /// Ports C `DocClauseEqUnfold`.
    ///
    /// `demod_pos_count` is the visible part of C's `demod_pos` stack for this
    /// renderer; C ignores the positions themselves and repeats the same
    /// demodulator id once per stack entry.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if TSTP clause rendering reports an unsupported
    /// clause shape.
    pub fn doc_clause_eq_unfold(
        &mut self,
        output: &mut impl fmt::Write,
        bank: &TermBank,
        rewritten: &mut Clause,
        demodulator: &Clause,
        demod_pos_count: usize,
    ) -> Result<ProofDocWriteResult, Diagnostic> {
        rewritten.del_prop(CP_INPUT_FORMULA);
        if self.output_level < 2 {
            return Ok(ProofDocWriteResult::suppressed());
        }

        let old_id = rewritten.ident();
        rewritten.set_ident(self.id_source.next_ident());
        let demodulator_ids = vec![demodulator.ident(); demod_pos_count];

        match self.output_format {
            ProofDocOutputFormat::Pcl => {
                pcl_print_start(
                    output,
                    bank,
                    rewritten,
                    self.pcl_shell_level < 1,
                    self.step_options,
                )
                .map_err(doc_write_error)?;
                write_pcl_clause_rewrite_inference(output, old_id, &demodulator_ids)
                    .map_err(doc_write_error)?;
                pcl_print_end(output, rewritten, Some("unfolding"), self.step_options)
                    .map_err(doc_write_error)?;
                Ok(ProofDocWriteResult::printed())
            }
            ProofDocOutputFormat::Tstp => {
                clause_write_tstp_with_type_suffixes(
                    output,
                    bank,
                    rewritten,
                    self.step_options.full_terms,
                    false,
                    self.problem_type,
                    self.step_options.print_types,
                )?;
                output.write_char(',').map_err(doc_write_error)?;
                write_tstp_clause_rewrite_inference(output, old_id, &demodulator_ids)
                    .map_err(doc_write_error)?;
                tstp_print_end(output, rewritten, Some("Unfolding")).map_err(doc_write_error)?;
                Ok(ProofDocWriteResult::printed())
            }
            ProofDocOutputFormat::NoFormat
            | ProofDocOutputFormat::Lop
            | ProofDocOutputFormat::Tptp
            | ProofDocOutputFormat::Xml => {
                write_unsupported_doc_format(output).map_err(doc_write_error)?;
                Ok(ProofDocWriteResult::printed())
            }
        }
    }

    /// Ports C `DocIntroSplitDef`.
    pub fn doc_intro_split_def(
        &mut self,
        output: &mut impl fmt::Write,
        formula: &mut FormulaDocView<'_>,
    ) -> Result<ProofDocWriteResult, Diagnostic> {
        self.doc_formula_creation(
            output,
            formula,
            FormulaCreationInference::IntroDef,
            FormulaCreationParents::none(),
            Some("split"),
        )
    }

    /// Ports C `DocIntroSplitDefRest`.
    ///
    /// The C function accepts a comment parameter but always emits no comment;
    /// this port keeps the argument to preserve the call shape.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if TSTP clause rendering reports an unsupported
    /// clause shape.
    pub fn doc_intro_split_def_rest(
        &mut self,
        output: &mut impl fmt::Write,
        bank: &TermBank,
        clause: &mut Clause,
        parent: &FormulaDocView<'_>,
        _comment: Option<&str>,
    ) -> Result<ProofDocWriteResult, Diagnostic> {
        if self.output_level < 2 {
            return Ok(ProofDocWriteResult::suppressed());
        }

        match self.output_format {
            ProofDocOutputFormat::Pcl => {
                clause.set_ident(self.id_source.next_ident());
                pcl_print_start(
                    output,
                    bank,
                    clause,
                    self.pcl_shell_level < 1,
                    self.step_options,
                )
                .map_err(doc_write_error)?;
                write_pcl_clause_unary_inference(
                    output,
                    ClauseUnaryInference::SplitEquiv,
                    parent.ident(),
                )
                .map_err(doc_write_error)?;
                pcl_print_end(output, clause, None, self.step_options).map_err(doc_write_error)?;
                Ok(ProofDocWriteResult::printed())
            }
            ProofDocOutputFormat::Tstp => {
                clause.set_ident(self.id_source.next_ident());
                clause_write_tstp_with_type_suffixes(
                    output,
                    bank,
                    clause,
                    self.step_options.full_terms,
                    false,
                    self.problem_type,
                    self.step_options.print_types,
                )?;
                output.write_char(',').map_err(doc_write_error)?;
                write_tstp_clause_unary_inference(
                    output,
                    ClauseUnaryInference::SplitEquiv,
                    parent.ident(),
                )
                .map_err(doc_write_error)?;
                tstp_print_end(output, clause, None).map_err(doc_write_error)?;
                Ok(ProofDocWriteResult::printed())
            }
            ProofDocOutputFormat::NoFormat
            | ProofDocOutputFormat::Lop
            | ProofDocOutputFormat::Tptp
            | ProofDocOutputFormat::Xml => {
                write_unsupported_doc_format(output).map_err(doc_write_error)?;
                Ok(ProofDocWriteResult::printed())
            }
        }
    }

    /// Ports C `DocClauseApplyDefs`.
    ///
    /// The C function accepts a comment parameter but hard-codes `split` in
    /// both PCL and TSTP output.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if TSTP clause rendering reports an unsupported
    /// clause shape.
    pub fn doc_clause_apply_defs(
        &mut self,
        output: &mut impl fmt::Write,
        bank: &TermBank,
        clause: &mut Clause,
        parent_id: i64,
        def_ids: &[i64],
        _comment: Option<&str>,
    ) -> Result<ProofDocWriteResult, Diagnostic> {
        if self.output_level < 2 {
            return Ok(ProofDocWriteResult::suppressed());
        }

        match self.output_format {
            ProofDocOutputFormat::Pcl => {
                clause.set_ident(self.id_source.next_ident());
                pcl_print_start(
                    output,
                    bank,
                    clause,
                    self.pcl_shell_level < 1,
                    self.step_options,
                )
                .map_err(doc_write_error)?;
                write_pcl_clause_apply_defs_inference(output, parent_id, def_ids)
                    .map_err(doc_write_error)?;
                pcl_print_end(output, clause, Some("split"), self.step_options)
                    .map_err(doc_write_error)?;
                Ok(ProofDocWriteResult::printed())
            }
            ProofDocOutputFormat::Tstp => {
                clause.set_ident(self.id_source.next_ident());
                clause_write_tstp_with_type_suffixes(
                    output,
                    bank,
                    clause,
                    self.step_options.full_terms,
                    false,
                    self.problem_type,
                    self.step_options.print_types,
                )?;
                output.write_char(',').map_err(doc_write_error)?;
                write_tstp_clause_apply_defs_inference(output, parent_id, def_ids)
                    .map_err(doc_write_error)?;
                tstp_print_end(output, clause, Some("split")).map_err(doc_write_error)?;
                Ok(ProofDocWriteResult::printed())
            }
            ProofDocOutputFormat::NoFormat
            | ProofDocOutputFormat::Lop
            | ProofDocOutputFormat::Tptp
            | ProofDocOutputFormat::Xml => {
                write_unsupported_doc_format(output).map_err(doc_write_error)?;
                Ok(ProofDocWriteResult::printed())
            }
        }
    }

    /// Ports C `DocFormulaIntroDefs`.
    pub fn doc_formula_intro_defs(
        &mut self,
        output: &mut impl fmt::Write,
        formula: &mut FormulaDocView<'_>,
        def_ids: &[i64],
        comment: Option<&str>,
    ) -> Result<ProofDocWriteResult, Diagnostic> {
        if self.output_level < 2 {
            return Ok(ProofDocWriteResult::suppressed());
        }

        let old_id = formula.ident();
        formula.set_ident(self.id_source.next_ident());
        match self.output_format {
            ProofDocOutputFormat::Pcl => {
                pcl_formula_print_start(
                    output,
                    formula.ident(),
                    formula.query_tptp_type(),
                    (self.pcl_shell_level < 1).then_some(formula.rendered_formula()),
                    self.step_options,
                )
                .map_err(doc_write_error)?;
                write_pcl_formula_apply_defs_inference(output, old_id, def_ids)
                    .map_err(doc_write_error)?;
                pcl_formula_print_end(output, comment, self.step_options)
                    .map_err(doc_write_error)?;
                Ok(ProofDocWriteResult::printed())
            }
            ProofDocOutputFormat::Tstp => {
                write_tstp_formula_start(output, formula, self.problem_type)
                    .map_err(doc_write_error)?;
                output.write_str(", ").map_err(doc_write_error)?;
                write_tstp_formula_apply_defs_inference(output, old_id, def_ids)
                    .map_err(doc_write_error)?;
                tstp_formula_print_end(output, comment).map_err(doc_write_error)?;
                Ok(ProofDocWriteResult::printed())
            }
            ProofDocOutputFormat::NoFormat
            | ProofDocOutputFormat::Lop
            | ProofDocOutputFormat::Tptp
            | ProofDocOutputFormat::Xml => {
                write_unsupported_doc_format(output).map_err(doc_write_error)?;
                Ok(ProofDocWriteResult::printed())
            }
        }
    }

    fn write_pcl_clause_creation(
        &self,
        output: &mut impl fmt::Write,
        bank: &TermBank,
        clause: &Clause,
        inference: ClauseCreationInference,
        parent_refs: ClauseCreationParents<'_>,
        comment: Option<&str>,
    ) -> Result<ProofDocWriteResult, Diagnostic> {
        match inference {
            ClauseCreationInference::Initial => {
                assert!(
                    parent_refs.parent1.is_none() && parent_refs.parent2.is_none(),
                    "initial clause documentation must not have parents"
                );
                let mut prefix = String::new();
                pcl_print_start(
                    &mut prefix,
                    bank,
                    clause,
                    self.pcl_shell_level < 2,
                    self.step_options,
                )
                .map_err(doc_write_error)?;
                let stdout_after_offset = prefix.len();
                output.write_str(&prefix).map_err(doc_write_error)?;
                output
                    .write_str(&source_info_pcl_string(clause.info()))
                    .map_err(doc_write_error)?;
                pcl_print_end(output, clause, comment, self.step_options)
                    .map_err(doc_write_error)?;
                Ok(ProofDocWriteResult::pcl_initial(stdout_after_offset))
            }
            ClauseCreationInference::Paramodulation
            | ClauseCreationInference::SimultaneousParamodulation => {
                let Some(left_parent) = parent_refs.parent1 else {
                    panic!("binary clause documentation needs first parent");
                };
                let Some(right_parent) = parent_refs.parent2 else {
                    panic!("binary clause documentation needs second parent");
                };
                pcl_print_start(
                    output,
                    bank,
                    clause,
                    self.pcl_shell_level < 1,
                    self.step_options,
                )
                .map_err(doc_write_error)?;
                let Some(binary) = inference.binary() else {
                    unreachable!("binary creation inference must map to a PCL name");
                };
                write_pcl_clause_binary_inference(
                    output,
                    binary,
                    left_parent.ident(),
                    right_parent.ident(),
                )
                .map_err(doc_write_error)?;
                pcl_print_end(output, clause, comment, self.step_options)
                    .map_err(doc_write_error)?;
                Ok(ProofDocWriteResult::printed())
            }
            ClauseCreationInference::EqualityResolution
            | ClauseCreationInference::EqualityFactoring
            | ClauseCreationInference::Factoring
            | ClauseCreationInference::Split => {
                let Some(source_parent) = parent_refs.parent1 else {
                    panic!("unary clause documentation needs first parent");
                };
                assert!(
                    parent_refs.parent2.is_none(),
                    "unary clause documentation must not have second parent"
                );
                pcl_print_start(
                    output,
                    bank,
                    clause,
                    self.pcl_shell_level < 1,
                    self.step_options,
                )
                .map_err(doc_write_error)?;
                let Some(unary) = inference.unary() else {
                    unreachable!("unary creation inference must map to a PCL name");
                };
                write_pcl_clause_unary_inference(output, unary, source_parent.ident())
                    .map_err(doc_write_error)?;
                pcl_print_end(output, clause, comment, self.step_options)
                    .map_err(doc_write_error)?;
                Ok(ProofDocWriteResult::printed())
            }
        }
    }

    fn write_pcl_clause_modification(
        &self,
        output: &mut impl fmt::Write,
        bank: &TermBank,
        render: ClauseModificationRender<'_>,
    ) -> Result<ProofDocWriteResult, Diagnostic> {
        pcl_print_start(
            output,
            bank,
            render.clause,
            self.pcl_shell_level < 1,
            self.step_options,
        )
        .map_err(doc_write_error)?;
        match render.inference {
            ClauseModificationInference::SimplifyReflect
            | ClauseModificationInference::ContextSimplifyReflect => {
                let Some(partner) = render.evidence.partner else {
                    panic!("binary clause modification documentation needs partner");
                };
                let Some(binary) = render.inference.binary() else {
                    unreachable!("binary modification inference must map to a PCL name");
                };
                write_pcl_clause_binary_inference(output, binary, render.old_id, partner.ident())
                    .map_err(doc_write_error)?;
            }
            ClauseModificationInference::Condense
            | ClauseModificationInference::Minimize
            | ClauseModificationInference::EvalAnswerLiteral => {
                assert!(
                    render.evidence.partner.is_none(),
                    "unary clause modification documentation must not have partner"
                );
                let Some(unary) = render.inference.unary() else {
                    unreachable!("unary modification inference must map to a PCL name");
                };
                write_pcl_clause_unary_inference(output, unary, render.old_id)
                    .map_err(doc_write_error)?;
            }
            ClauseModificationInference::DestructiveEqualityResolution => {
                assert!(
                    render.evidence.partner.is_some(),
                    "destructive equality-resolution documentation needs partner"
                );
                let Some(unary) = render.inference.unary() else {
                    unreachable!("destructive equality-resolution must map to a PCL name");
                };
                write_pcl_clause_unary_inference(output, unary, render.old_id)
                    .map_err(doc_write_error)?;
            }
            ClauseModificationInference::AcResolution => {
                assert!(
                    render.evidence.partner.is_none(),
                    "AC-resolution documentation must not have partner"
                );
                write_pcl_clause_ac_resolution_inference(
                    output,
                    render.old_id,
                    render.evidence.ac_axiom_ids,
                )
                .map_err(doc_write_error)?;
            }
        }
        pcl_print_end(output, render.clause, render.comment, self.step_options)
            .map_err(doc_write_error)?;
        Ok(ProofDocWriteResult::printed())
    }

    fn write_tstp_clause_modification(
        &self,
        output: &mut impl fmt::Write,
        bank: &TermBank,
        render: ClauseModificationRender<'_>,
    ) -> Result<ProofDocWriteResult, Diagnostic> {
        clause_write_tstp_with_type_suffixes(
            output,
            bank,
            render.clause,
            self.step_options.full_terms,
            false,
            self.problem_type,
            self.step_options.print_types,
        )?;
        output.write_char(',').map_err(doc_write_error)?;
        match render.inference {
            ClauseModificationInference::SimplifyReflect
            | ClauseModificationInference::ContextSimplifyReflect => {
                let Some(partner) = render.evidence.partner else {
                    panic!("binary clause modification documentation needs partner");
                };
                let Some(binary) = render.inference.binary() else {
                    unreachable!("binary modification inference must map to a TSTP name");
                };
                write_tstp_clause_binary_inference(output, binary, render.old_id, partner.ident())
                    .map_err(doc_write_error)?;
            }
            ClauseModificationInference::Condense
            | ClauseModificationInference::Minimize
            | ClauseModificationInference::EvalAnswerLiteral => {
                assert!(
                    render.evidence.partner.is_none(),
                    "unary clause modification documentation must not have partner"
                );
                let Some(unary) = render.inference.unary() else {
                    unreachable!("unary modification inference must map to a TSTP name");
                };
                write_tstp_clause_unary_inference(output, unary, render.old_id)
                    .map_err(doc_write_error)?;
            }
            ClauseModificationInference::DestructiveEqualityResolution => {
                assert!(
                    render.evidence.partner.is_some(),
                    "destructive equality-resolution documentation needs partner"
                );
                let Some(unary) = render.inference.unary() else {
                    unreachable!("destructive equality-resolution must map to a TSTP name");
                };
                write_tstp_clause_unary_inference(output, unary, render.old_id)
                    .map_err(doc_write_error)?;
            }
            ClauseModificationInference::AcResolution => {
                assert!(
                    render.evidence.partner.is_none(),
                    "AC-resolution documentation must not have partner"
                );
                write_tstp_clause_ac_resolution_inference(
                    output,
                    render.old_id,
                    render.evidence.ac_axiom_ids,
                )
                .map_err(doc_write_error)?;
            }
        }
        tstp_print_end(output, render.clause, render.comment).map_err(doc_write_error)?;
        Ok(ProofDocWriteResult::printed())
    }

    fn write_tstp_clause_creation(
        &self,
        output: &mut impl fmt::Write,
        bank: &TermBank,
        clause: &Clause,
        inference: ClauseCreationInference,
        parent_refs: ClauseCreationParents<'_>,
        comment: Option<&str>,
    ) -> Result<ProofDocWriteResult, Diagnostic> {
        clause_write_tstp_with_type_suffixes(
            output,
            bank,
            clause,
            self.step_options.full_terms,
            false,
            self.problem_type,
            self.step_options.print_types,
        )?;
        match inference {
            ClauseCreationInference::Initial => {
                assert!(
                    parent_refs.parent1.is_none() && parent_refs.parent2.is_none(),
                    "initial clause documentation must not have parents"
                );
                write!(output, ", {}", source_info_tstp_string(clause.info()))
                    .map_err(doc_write_error)?;
            }
            ClauseCreationInference::Paramodulation
            | ClauseCreationInference::SimultaneousParamodulation => {
                let Some(left_parent) = parent_refs.parent1 else {
                    panic!("binary clause documentation needs first parent");
                };
                let Some(right_parent) = parent_refs.parent2 else {
                    panic!("binary clause documentation needs second parent");
                };
                output.write_char(',').map_err(doc_write_error)?;
                let Some(binary) = inference.binary() else {
                    unreachable!("binary creation inference must map to a TSTP name");
                };
                write_tstp_clause_binary_inference(
                    output,
                    binary,
                    left_parent.ident(),
                    right_parent.ident(),
                )
                .map_err(doc_write_error)?;
            }
            ClauseCreationInference::EqualityResolution
            | ClauseCreationInference::EqualityFactoring
            | ClauseCreationInference::Factoring
            | ClauseCreationInference::Split => {
                let Some(source_parent) = parent_refs.parent1 else {
                    panic!("unary clause documentation needs first parent");
                };
                assert!(
                    parent_refs.parent2.is_none(),
                    "unary clause documentation must not have second parent"
                );
                output.write_char(',').map_err(doc_write_error)?;
                let Some(unary) = inference.unary() else {
                    unreachable!("unary creation inference must map to a TSTP name");
                };
                write_tstp_clause_unary_inference(output, unary, source_parent.ident())
                    .map_err(doc_write_error)?;
            }
        }
        tstp_print_end(output, clause, comment).map_err(doc_write_error)?;
        Ok(ProofDocWriteResult::printed())
    }

    fn write_pcl_formula_creation(
        &self,
        output: &mut impl fmt::Write,
        formula: &FormulaDocView<'_>,
        inference: FormulaCreationInference,
        parent_refs: FormulaCreationParents<'_>,
        comment: Option<&str>,
    ) -> Result<ProofDocWriteResult, Diagnostic> {
        match inference {
            FormulaCreationInference::Initial => {
                assert!(
                    parent_refs.parent1.is_none() && parent_refs.parent2.is_none(),
                    "initial formula documentation must not have parents"
                );
                pcl_formula_print_start(
                    output,
                    formula.ident(),
                    formula.query_tptp_type(),
                    (self.pcl_shell_level < 2).then_some(formula.rendered_formula()),
                    self.step_options,
                )
                .map_err(doc_write_error)?;
                output
                    .write_str(&source_info_pcl_string(formula.info()))
                    .map_err(doc_write_error)?;
            }
            FormulaCreationInference::IntroDef => {
                assert!(
                    parent_refs.parent1.is_none() && parent_refs.parent2.is_none(),
                    "formula definition introduction must not have parents"
                );
                pcl_formula_print_start(
                    output,
                    formula.ident(),
                    formula.query_tptp_type(),
                    (self.pcl_shell_level < 1).then_some(formula.rendered_formula()),
                    self.step_options,
                )
                .map_err(doc_write_error)?;
                write_pcl_formula_intro_def_inference(output).map_err(doc_write_error)?;
            }
            FormulaCreationInference::SplitEquiv => {
                let Some(parent) = parent_refs.parent1 else {
                    panic!("formula split-equivalence documentation needs parent");
                };
                assert!(
                    parent_refs.parent2.is_none(),
                    "formula split-equivalence documentation must not have second parent"
                );
                pcl_formula_print_start(
                    output,
                    formula.ident(),
                    formula.query_tptp_type(),
                    (self.pcl_shell_level < 1).then_some(formula.rendered_formula()),
                    self.step_options,
                )
                .map_err(doc_write_error)?;
                write_pcl_formula_parent_inference(
                    output,
                    FormulaParentInference::SplitEquiv,
                    parent.ident(),
                )
                .map_err(doc_write_error)?;
            }
        }
        pcl_formula_print_end(output, comment, self.step_options).map_err(doc_write_error)?;
        Ok(ProofDocWriteResult::printed())
    }

    fn write_tstp_formula_creation(
        &self,
        output: &mut impl fmt::Write,
        formula: &FormulaDocView<'_>,
        inference: FormulaCreationInference,
        parent_refs: FormulaCreationParents<'_>,
        comment: Option<&str>,
    ) -> Result<ProofDocWriteResult, Diagnostic> {
        match inference {
            FormulaCreationInference::Initial => {
                assert!(
                    parent_refs.parent1.is_none() && parent_refs.parent2.is_none(),
                    "initial formula documentation must not have parents"
                );
                write_tstp_formula_start(output, formula, self.problem_type)
                    .map_err(doc_write_error)?;
                output.write_str(", ").map_err(doc_write_error)?;
                output
                    .write_str(&source_info_tstp_string(formula.info()))
                    .map_err(doc_write_error)?;
            }
            FormulaCreationInference::IntroDef => {
                assert!(
                    parent_refs.parent1.is_none() && parent_refs.parent2.is_none(),
                    "formula definition introduction must not have parents"
                );
                write_tstp_formula_start(output, formula, self.problem_type)
                    .map_err(doc_write_error)?;
                output.write_str(", ").map_err(doc_write_error)?;
                write_tstp_formula_intro_def_inference(output).map_err(doc_write_error)?;
            }
            FormulaCreationInference::SplitEquiv => {
                let Some(parent) = parent_refs.parent1 else {
                    panic!("formula split-equivalence documentation needs parent");
                };
                assert!(
                    parent_refs.parent2.is_none(),
                    "formula split-equivalence documentation must not have second parent"
                );
                write_tstp_formula_start(output, formula, self.problem_type)
                    .map_err(doc_write_error)?;
                output.write_str(", ").map_err(doc_write_error)?;
                write_tstp_formula_parent_inference(
                    output,
                    FormulaParentInference::SplitEquiv,
                    parent.ident(),
                )
                .map_err(doc_write_error)?;
            }
        }
        tstp_formula_print_end(output, comment).map_err(doc_write_error)?;
        Ok(ProofDocWriteResult::printed())
    }

    fn write_pcl_formula_modification(
        &self,
        output: &mut impl fmt::Write,
        formula: &FormulaDocView<'_>,
        inference: FormulaParentInference,
        old_id: i64,
        comment: Option<&str>,
    ) -> Result<ProofDocWriteResult, Diagnostic> {
        pcl_formula_print_start(
            output,
            formula.ident(),
            formula.query_tptp_type(),
            (self.pcl_shell_level < 1).then_some(formula.rendered_formula()),
            self.step_options,
        )
        .map_err(doc_write_error)?;
        write_pcl_formula_parent_inference(output, inference, old_id).map_err(doc_write_error)?;
        pcl_formula_print_end(output, comment, self.step_options).map_err(doc_write_error)?;
        Ok(ProofDocWriteResult::printed())
    }

    fn write_tstp_formula_modification(
        &self,
        output: &mut impl fmt::Write,
        formula: &FormulaDocView<'_>,
        inference: FormulaParentInference,
        old_id: i64,
        comment: Option<&str>,
    ) -> Result<ProofDocWriteResult, Diagnostic> {
        write_tstp_formula_start(output, formula, self.problem_type).map_err(doc_write_error)?;
        output.write_char(',').map_err(doc_write_error)?;
        write_tstp_formula_parent_inference(output, inference, old_id).map_err(doc_write_error)?;
        tstp_formula_print_end(output, comment).map_err(doc_write_error)?;
        Ok(ProofDocWriteResult::printed())
    }
}

#[must_use]
pub const fn pcl_type_str(type_: FormulaProperties) -> &'static str {
    match type_ {
        CP_TYPE_CONJECTURE => "conj",
        CP_TYPE_QUESTION => "que",
        CP_TYPE_NEG_CONJECTURE => "neg",
        _ => "",
    }
}

fn write_unsupported_doc_format(output: &mut impl fmt::Write) -> fmt::Result {
    writeln!(
        output,
        "{DEFAULT_COMCHAR_RAW} Output format not implemented."
    )
}

fn compute_rewrite_demodulator_ids(old_term: &Term, normal_form: &Term) -> Vec<i64> {
    let mut steps = PStack::new();
    assert!(
        term_compute_rw_sequence(&mut steps, old_term, normal_form, 0),
        "clause rewrite documentation requires a non-empty rewrite sequence"
    );
    steps
        .as_slice()
        .iter()
        .map(|entry| match entry {
            RewriteSequenceEntry::Demodulator(demodulator) => {
                demodulator.id().try_into().unwrap_or(i64::MAX)
            }
            RewriteSequenceEntry::Operation(_)
            | RewriteSequenceEntry::ClauseParent(_)
            | RewriteSequenceEntry::FormulaParent(_)
            | RewriteSequenceEntry::NumericArg(_) => {
                panic!("rewrite documentation sequence has non-demodulator entry")
            }
        })
        .collect()
}

fn formula_tstp_identifier(formula: &FormulaDocView<'_>) -> String {
    if formula.ident() >= 0 {
        format!("c_0_{}", formula.ident())
    } else {
        let offset = i128::from(formula.ident()) - i128::from(i64::MIN);
        format!("i_0_{offset}")
    }
}

fn formula_tstp_kind(formula: &FormulaDocView<'_>, problem_type: ProblemType) -> &'static str {
    if problem_type == ProblemType::HigherOrder {
        "thf"
    } else if formula.is_untyped() {
        "fof"
    } else {
        "tff"
    }
}

fn formula_tstp_role(formula: &FormulaDocView<'_>) -> &'static str {
    match formula.query_tptp_type() {
        CP_TYPE_AXIOM if formula.query_prop(CP_INPUT_FORMULA) => "axiom",
        CP_TYPE_HYPOTHESIS => "hypothesis",
        CP_TYPE_CONJECTURE => "conjecture",
        CP_TYPE_QUESTION => "question",
        CP_TYPE_LEMMA => "lemma",
        CP_TYPE_NEG_CONJECTURE => "negated_conjecture",
        _ => "plain",
    }
}

fn write_tstp_formula_start(
    output: &mut impl fmt::Write,
    formula: &FormulaDocView<'_>,
    problem_type: ProblemType,
) -> fmt::Result {
    write!(
        output,
        "{}({}, {}, {}",
        formula_tstp_kind(formula, problem_type),
        formula_tstp_identifier(formula),
        formula_tstp_role(formula),
        formula.rendered_formula()
    )
}

fn doc_write_error(_error: fmt::Error) -> Diagnostic {
    Diagnostic::new(
        ErrorCode::OTHER_ERROR,
        "failed to write proof documentation",
    )
}

pub fn pcl_print_start(
    output: &mut impl fmt::Write,
    bank: &TermBank,
    clause: &Clause,
    print_clause: bool,
    options: PclStepPrintOptions,
) -> fmt::Result {
    if options.compact {
        write!(output, "{}:", clause.ident())?;
    } else {
        write!(output, "{:6} : ", clause.ident())?;
    }
    write!(output, "{}:", pcl_type_str(clause.query_tptp_type()))?;
    if print_clause {
        clause_write_pcl_with_options(
            output,
            bank,
            clause,
            options.full_terms,
            options.eqn_print_options,
        )?;
    }
    output.write_str(" : ")
}

pub fn pcl_formula_print_start(
    output: &mut impl fmt::Write,
    ident: i64,
    type_: FormulaProperties,
    rendered_formula: Option<&str>,
    options: PclStepPrintOptions,
) -> fmt::Result {
    if options.compact {
        write!(output, "{ident}:")?;
    } else {
        write!(output, "{ident:6} : ")?;
    }
    write!(output, "{}:", pcl_type_str(type_))?;
    if let Some(rendered_formula) = rendered_formula {
        output.write_str(rendered_formula)?;
    }
    output.write_str(" : ")
}

pub fn pcl_print_end(
    output: &mut impl fmt::Write,
    clause: &Clause,
    comment: Option<&str>,
    options: PclStepPrintOptions,
) -> fmt::Result {
    match (clause.query_prop(CP_WATCH_ONLY), comment) {
        (true, Some(comment)) => write!(
            output,
            "{}'wl,{comment}'",
            if options.compact { ":" } else { ": " }
        )?,
        (false, Some(comment)) => write!(
            output,
            "{}'{comment}'",
            if options.compact { ":" } else { " : " }
        )?,
        (true, None) => output.write_str(if options.compact { ":'wl'" } else { " : 'wl'" })?,
        (false, None) => {}
    }
    output.write_char('\n')
}

pub fn tstp_print_end(
    output: &mut impl fmt::Write,
    clause: &Clause,
    comment: Option<&str>,
) -> fmt::Result {
    match (clause.query_prop(CP_WATCH_ONLY), comment) {
        (true, Some(comment)) => write!(output, ",['wl,{comment}']")?,
        (false, Some(comment)) => write!(output, ",['{comment}']")?,
        (true, None) => output.write_str(",['wl']")?,
        (false, None) => {}
    }
    output.write_str(").\n")
}

pub fn pcl_formula_print_end(
    output: &mut impl fmt::Write,
    comment: Option<&str>,
    options: PclStepPrintOptions,
) -> fmt::Result {
    if let Some(comment) = comment {
        write!(
            output,
            "{}'{comment}'",
            if options.compact { ":" } else { " : " }
        )?;
    }
    output.write_char('\n')
}

pub fn tstp_formula_print_end(output: &mut impl fmt::Write, comment: Option<&str>) -> fmt::Result {
    if let Some(comment) = comment {
        write!(output, ",['{comment}']")?;
    }
    output.write_str(").\n")
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClauseBinaryInference {
    Paramodulation,
    SimultaneousParamodulation,
    SimplifyReflect,
    ContextSimplifyReflect,
}

impl ClauseBinaryInference {
    #[must_use]
    pub const fn pcl_name(self) -> &'static str {
        match self {
            Self::Paramodulation => "pm",
            Self::SimultaneousParamodulation => "spm",
            Self::SimplifyReflect => "sr",
            Self::ContextSimplifyReflect => "csr",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClauseUnaryInference {
    EqualityResolution,
    EqualityFactoring,
    Factoring,
    Split,
    SplitConjunct,
    SplitEquiv,
    Normalize,
    Condense,
    EvalAnswerLiteral,
}

impl ClauseUnaryInference {
    #[must_use]
    pub const fn pcl_name(self) -> &'static str {
        match self {
            Self::EqualityResolution => "er",
            Self::EqualityFactoring => "ef",
            Self::Factoring => "of",
            Self::Split => "split",
            Self::SplitConjunct => "split_conjunct",
            Self::SplitEquiv => "split_equiv",
            Self::Normalize => "cn",
            Self::Condense => "condense",
            Self::EvalAnswerLiteral => "eval_answer_literal",
        }
    }
}

pub fn write_pcl_clause_binary_inference(
    output: &mut impl fmt::Write,
    inference: ClauseBinaryInference,
    parent1_id: i64,
    parent2_id: i64,
) -> fmt::Result {
    write!(
        output,
        "{}({parent1_id},{parent2_id})",
        inference.pcl_name()
    )
}

pub fn write_tstp_clause_binary_inference(
    output: &mut impl fmt::Write,
    inference: ClauseBinaryInference,
    parent1_id: i64,
    parent2_id: i64,
) -> fmt::Result {
    write!(
        output,
        "inference({},[status(thm)],[c_0_{parent1_id},c_0_{parent2_id}])",
        inference.pcl_name()
    )
}

pub fn write_pcl_clause_unary_inference(
    output: &mut impl fmt::Write,
    inference: ClauseUnaryInference,
    parent_id: i64,
) -> fmt::Result {
    write!(output, "{}({parent_id})", inference.pcl_name())
}

pub fn write_tstp_clause_unary_inference(
    output: &mut impl fmt::Write,
    inference: ClauseUnaryInference,
    parent_id: i64,
) -> fmt::Result {
    match inference {
        ClauseUnaryInference::Split => {
            write!(
                output,
                "inference(split,[split(esplit,[])],[c_0_{parent_id}])"
            )
        }
        ClauseUnaryInference::SplitConjunct => write!(
            output,
            "inference(split_conjunct, [status(thm)],[c_0_{parent_id}])"
        ),
        ClauseUnaryInference::SplitEquiv => write!(
            output,
            "inference(split_equiv, [status(thm)],[c_0_{parent_id}])"
        ),
        ClauseUnaryInference::EvalAnswerLiteral => write!(
            output,
            "inference(eval_answer_literal,[status(thm)],[c_0_{parent_id}, theory(answers)])"
        ),
        ClauseUnaryInference::EqualityResolution
        | ClauseUnaryInference::EqualityFactoring
        | ClauseUnaryInference::Factoring
        | ClauseUnaryInference::Normalize
        | ClauseUnaryInference::Condense => write!(
            output,
            "inference({},[status(thm)],[c_0_{parent_id}])",
            inference.pcl_name()
        ),
    }
}

/// Writes the C `print_ac_res` inference expression.
///
/// # Panics
///
/// Panics if `ac_axiom_ids` is empty, matching the C assertion that the
/// signature-owned AC axiom stack is non-empty when AC-resolution is printed.
pub fn write_pcl_clause_ac_resolution_inference(
    output: &mut impl fmt::Write,
    old_id: i64,
    ac_axiom_ids: &[i64],
) -> fmt::Result {
    assert!(
        !ac_axiom_ids.is_empty(),
        "AC-resolution documentation requires at least one AC axiom"
    );
    write!(output, "ar({old_id}")?;
    for axiom_id in ac_axiom_ids {
        write!(output, ",{axiom_id}")?;
    }
    output.write_char(')')
}

/// Writes the C `print_ac_res` TSTP inference expression.
///
/// # Panics
///
/// Panics if `ac_axiom_ids` is empty, matching the C assertion that the
/// signature-owned AC axiom stack is non-empty when AC-resolution is printed.
pub fn write_tstp_clause_ac_resolution_inference(
    output: &mut impl fmt::Write,
    old_id: i64,
    ac_axiom_ids: &[i64],
) -> fmt::Result {
    assert!(
        !ac_axiom_ids.is_empty(),
        "AC-resolution documentation requires at least one AC axiom"
    );
    write!(output, "inference(ar,[status(thm)],[c_0_{old_id}")?;
    for axiom_id in ac_axiom_ids {
        write!(output, ",c_0_{axiom_id}")?;
    }
    output.write_str("])")
}

pub fn write_pcl_clause_rewrite_inference(
    output: &mut impl fmt::Write,
    old_id: i64,
    demodulator_ids: &[i64],
) -> fmt::Result {
    for _ in demodulator_ids {
        output.write_str("rw(")?;
    }
    write!(output, "{old_id}")?;
    for demodulator_id in demodulator_ids {
        write!(output, ",{demodulator_id})")?;
    }
    Ok(())
}

pub fn write_tstp_clause_rewrite_inference(
    output: &mut impl fmt::Write,
    old_id: i64,
    demodulator_ids: &[i64],
) -> fmt::Result {
    for _ in demodulator_ids {
        output.write_str("inference(rw, [status(thm)],[")?;
    }
    write!(output, "c_0_{old_id}")?;
    for demodulator_id in demodulator_ids {
        write!(output, ",c_0_{demodulator_id}])")?;
    }
    Ok(())
}

pub fn write_pcl_clause_apply_defs_inference(
    output: &mut impl fmt::Write,
    parent_id: i64,
    def_ids: &[i64],
) -> fmt::Result {
    for _ in def_ids {
        output.write_str("apply_def(")?;
    }
    write!(output, "{parent_id}")?;
    for def_id in def_ids {
        write!(output, ",{def_id})")?;
    }
    Ok(())
}

pub fn write_tstp_clause_apply_defs_inference(
    output: &mut impl fmt::Write,
    parent_id: i64,
    def_ids: &[i64],
) -> fmt::Result {
    for _ in def_ids {
        output.write_str("inference(apply_def, [status(thm)],[")?;
    }
    write!(output, "c_0_{parent_id}")?;
    for def_id in def_ids {
        write!(output, ",c_0_{def_id}])")?;
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FormulaParentInference {
    SplitEquiv,
    Simplification,
    NegConjecture,
    Nnf,
    ShiftQuantors,
    VarRename,
    Skolemize,
    Distribute,
    AnnotateQuestion,
}

impl FormulaParentInference {
    #[must_use]
    pub const fn pcl_name(self) -> &'static str {
        match self {
            Self::SplitEquiv => "split_equiv",
            Self::Simplification => "fof_simplification",
            Self::NegConjecture => "assume_negation",
            Self::Nnf => "fof_nnf",
            Self::ShiftQuantors => "shift_quantors",
            Self::VarRename => "variable_rename",
            Self::Skolemize => "skolemize",
            Self::Distribute => "distribute",
            Self::AnnotateQuestion => "add_answer_literal",
        }
    }

    #[must_use]
    pub const fn tstp_status(self) -> &'static str {
        match self {
            Self::NegConjecture => "cth",
            Self::Skolemize => "esa",
            Self::SplitEquiv
            | Self::Simplification
            | Self::Nnf
            | Self::ShiftQuantors
            | Self::VarRename
            | Self::Distribute
            | Self::AnnotateQuestion => "thm",
        }
    }
}

pub fn write_pcl_formula_intro_def_inference(output: &mut impl fmt::Write) -> fmt::Result {
    output.write_str("introduced")
}

pub fn write_tstp_formula_intro_def_inference(output: &mut impl fmt::Write) -> fmt::Result {
    output.write_str("introduced(definition)")
}

pub fn write_pcl_formula_parent_inference(
    output: &mut impl fmt::Write,
    inference: FormulaParentInference,
    parent_id: i64,
) -> fmt::Result {
    write!(output, "{}({parent_id})", inference.pcl_name())
}

pub fn write_tstp_formula_parent_inference(
    output: &mut impl fmt::Write,
    inference: FormulaParentInference,
    parent_id: i64,
) -> fmt::Result {
    let name = inference.pcl_name();
    let status = inference.tstp_status();
    match inference {
        FormulaParentInference::SplitEquiv | FormulaParentInference::Skolemize => {
            write!(
                output,
                "inference({name}, [status({status})], [c_0_{parent_id}])"
            )
        }
        FormulaParentInference::AnnotateQuestion => write!(
            output,
            "inference({name}, [status({status})],[c_0_{parent_id},theory(answers)])"
        ),
        FormulaParentInference::Simplification
        | FormulaParentInference::NegConjecture
        | FormulaParentInference::Nnf
        | FormulaParentInference::ShiftQuantors
        | FormulaParentInference::VarRename
        | FormulaParentInference::Distribute => {
            write!(
                output,
                "inference({name}, [status({status})],[c_0_{parent_id}])"
            )
        }
    }
}

pub fn write_pcl_formula_apply_defs_inference(
    output: &mut impl fmt::Write,
    parent_id: i64,
    def_ids: &[i64],
) -> fmt::Result {
    for _ in def_ids {
        output.write_str("apply_def(")?;
    }
    write!(output, "{parent_id}")?;
    for def_id in def_ids {
        write!(output, ",{def_id})")?;
    }
    Ok(())
}

pub fn write_tstp_formula_apply_defs_inference(
    output: &mut impl fmt::Write,
    parent_id: i64,
    def_ids: &[i64],
) -> fmt::Result {
    for _ in def_ids {
        output.write_str("inference(apply_def,[status(thm)],[")?;
    }
    write!(output, "c_0_{parent_id}")?;
    for def_id in def_ids {
        write!(output, ",c_0_{def_id}])")?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        pcl_formula_print_end, pcl_formula_print_start, pcl_print_end, pcl_print_start,
        pcl_type_str, tstp_formula_print_end, tstp_print_end,
        write_pcl_clause_ac_resolution_inference, write_pcl_clause_apply_defs_inference,
        write_pcl_clause_binary_inference, write_pcl_clause_rewrite_inference,
        write_pcl_clause_unary_inference, write_pcl_formula_apply_defs_inference,
        write_pcl_formula_intro_def_inference, write_pcl_formula_parent_inference,
        write_tstp_clause_ac_resolution_inference, write_tstp_clause_apply_defs_inference,
        write_tstp_clause_binary_inference, write_tstp_clause_rewrite_inference,
        write_tstp_clause_unary_inference, write_tstp_formula_apply_defs_inference,
        write_tstp_formula_intro_def_inference, write_tstp_formula_parent_inference,
        ClauseBinaryInference, ClauseCreationInference, ClauseCreationParents,
        ClauseModificationEvidence, ClauseModificationInference, ClauseUnaryInference,
        FormulaCreationInference, FormulaCreationParents, FormulaDocView,
        FormulaModificationInference, FormulaParentInference, PclStepPrintOptions,
        ProofDocOutputFormat, ProofDocSession, ProofDocWriteResult,
    };
    use crate::basics::simple_stuff::ProblemType;
    use crate::clauses::clause::Clause;
    use crate::clauses::clause_props::{
        CP_INPUT_FORMULA, CP_TYPE_AXIOM, CP_TYPE_CONJECTURE, CP_TYPE_HYPOTHESIS, CP_TYPE_LEMMA,
        CP_TYPE_NEG_CONJECTURE, CP_TYPE_QUESTION, CP_TYPE_UNKNOWN, CP_TYPE_WATCH_CLAUSE,
        CP_WATCH_ONLY,
    };
    use crate::clauses::clauseinfo::ClauseInfo;
    use crate::clauses::clausepos::ClausePos;
    use crate::clauses::eqn::Eqn;
    use crate::clauses::eqn_props::EqnSide;
    use crate::clauses::eqnlist::EqnList;
    use crate::terms::replace::{term_add_rw_link, RwResultType};
    use crate::terms::signature::Signature;
    use crate::terms::termbanks::TermBank;
    use crate::terms::termtypes::{RewriteDemodulator, Term};
    use crate::terms::typebanks::TypeBank;

    fn test_bank() -> TermBank {
        TermBank::new(Signature::new(TypeBank::new())).unwrap()
    }

    fn typed_const(bank: &mut TermBank, name: &str) -> Term {
        let type_ = bank.signature().type_bank().default_type();
        let f_code = bank.signature_mut().insert_id(name, 0, false);
        bank.signature_mut()
            .declare_final_type(f_code, type_)
            .unwrap();
        bank.create_const_term(f_code).unwrap()
    }

    fn eqn(bank: &mut TermBank, left: &Term, right: &Term, positive: bool) -> Eqn {
        Eqn::alloc(left.clone(), right.clone(), bank, positive).unwrap()
    }

    #[test]
    fn proof_doc_output_format_discriminants_match_c_enum() {
        assert_eq!(ProofDocOutputFormat::NoFormat.c_value(), 0);
        assert_eq!(ProofDocOutputFormat::Lop.c_value(), 1);
        assert_eq!(ProofDocOutputFormat::Pcl.c_value(), 2);
        assert_eq!(ProofDocOutputFormat::Tstp.c_value(), 3);
        assert_eq!(ProofDocOutputFormat::Tptp.c_value(), 4);
        assert_eq!(ProofDocOutputFormat::Xml.c_value(), 5);
    }

    #[test]
    fn doc_clause_creation_suppresses_below_level_two_without_assigning_id() {
        let bank = test_bank();
        let mut session =
            ProofDocSession::new(ProofDocOutputFormat::Pcl, 1, ProblemType::FirstOrder);
        let mut clause = Clause::empty();
        clause.set_ident(42);
        let mut rendered = String::new();

        let result = session
            .doc_clause_creation(
                &mut rendered,
                &bank,
                &mut clause,
                ClauseCreationInference::Initial,
                ClauseCreationParents::none(),
                None,
            )
            .unwrap();

        assert_eq!(result, ProofDocWriteResult::not_printed());
        assert!(rendered.is_empty());
        assert_eq!(clause.ident(), 42);
        assert_eq!(session.id_source.current_ident(), 0);
    }

    #[test]
    fn doc_clause_creation_prints_initial_pcl_and_exposes_c_stdout_markers() {
        let bank = test_bank();
        let mut session =
            ProofDocSession::new(ProofDocOutputFormat::Pcl, 2, ProblemType::FirstOrder);
        let mut clause = Clause::empty();
        clause.set_info(Some(ClauseInfo::new(Some("ax1"), Some("problem.p"), 1, 2)));
        let mut rendered = String::new();

        let result = session
            .doc_clause_creation(
                &mut rendered,
                &bank,
                &mut clause,
                ClauseCreationInference::Initial,
                ClauseCreationParents::none(),
                Some("input"),
            )
            .unwrap();

        assert_eq!(
            result,
            ProofDocWriteResult::pcl_initial("     1 : :[] : ".len())
        );
        assert_eq!(clause.ident(), 1);
        assert_eq!(
            rendered,
            "     1 : :[] : initial(\"problem.p\", ax1) : 'input'\n"
        );
    }

    #[test]
    fn doc_clause_creation_prints_binary_and_unary_pcl_steps_with_new_ids() {
        let bank = test_bank();
        let mut session =
            ProofDocSession::new(ProofDocOutputFormat::Pcl, 2, ProblemType::FirstOrder);
        let mut left = Clause::empty();
        left.set_ident(10);
        let mut right = Clause::empty();
        right.set_ident(11);
        let mut paramod = Clause::empty();
        let mut rendered = String::new();

        let result = session
            .doc_clause_creation(
                &mut rendered,
                &bank,
                &mut paramod,
                ClauseCreationInference::Paramodulation,
                ClauseCreationParents::binary(&left, &right),
                Some("generated"),
            )
            .unwrap();

        assert_eq!(result, ProofDocWriteResult::printed());
        assert_eq!(paramod.ident(), 1);
        assert_eq!(rendered, "     1 : :[] : pm(10,11) : 'generated'\n");

        rendered.clear();
        let mut factor = Clause::empty();
        session
            .doc_clause_creation(
                &mut rendered,
                &bank,
                &mut factor,
                ClauseCreationInference::EqualityFactoring,
                ClauseCreationParents::unary(&left),
                None,
            )
            .unwrap();

        assert_eq!(factor.ident(), 2);
        assert_eq!(rendered, "     2 : :[] : ef(10)\n");
    }

    #[test]
    fn doc_clause_creation_prints_tstp_creation_steps() {
        let bank = test_bank();
        let mut session =
            ProofDocSession::new(ProofDocOutputFormat::Tstp, 2, ProblemType::FirstOrder);
        let mut initial = Clause::empty();
        initial.set_info(Some(ClauseInfo::new(Some("ax1"), Some("problem.p"), 1, 2)));
        let mut rendered = String::new();

        session
            .doc_clause_creation(
                &mut rendered,
                &bank,
                &mut initial,
                ClauseCreationInference::Initial,
                ClauseCreationParents::none(),
                None,
            )
            .unwrap();

        assert_eq!(
            rendered,
            "cnf(c_0_1, plain, ($false), file('problem.p', ax1)).\n"
        );

        let mut parent = Clause::empty();
        parent.set_ident(10);
        let mut split = Clause::empty();
        rendered.clear();
        session
            .doc_clause_creation(
                &mut rendered,
                &bank,
                &mut split,
                ClauseCreationInference::Split,
                ClauseCreationParents::unary(&parent),
                Some("split"),
            )
            .unwrap();

        assert_eq!(
            rendered,
            "cnf(c_0_2, plain, ($false),inference(split,[split(esplit,[])],[c_0_10]),['split']).\n"
        );
    }

    #[test]
    fn doc_clause_creation_preserves_c_unsupported_format_fallback() {
        let bank = test_bank();
        let mut session =
            ProofDocSession::new(ProofDocOutputFormat::NoFormat, 2, ProblemType::FirstOrder);
        let mut clause = Clause::empty();
        let mut rendered = String::new();

        let result = session
            .doc_clause_creation(
                &mut rendered,
                &bank,
                &mut clause,
                ClauseCreationInference::Initial,
                ClauseCreationParents::none(),
                None,
            )
            .unwrap();

        assert_eq!(result, ProofDocWriteResult::printed());
        assert_eq!(clause.ident(), 1);
        assert_eq!(rendered, "% Output format not implemented.\n");
    }

    #[test]
    fn doc_clause_modification_suppresses_below_level_two_but_clears_input_formula() {
        let bank = test_bank();
        let mut session =
            ProofDocSession::new(ProofDocOutputFormat::Pcl, 1, ProblemType::FirstOrder);
        let mut clause = Clause::empty();
        clause.set_ident(42);
        clause.set_prop(CP_INPUT_FORMULA);
        let mut rendered = String::new();

        let result = session
            .doc_clause_modification(
                &mut rendered,
                &bank,
                &mut clause,
                ClauseModificationInference::Minimize,
                None,
                None,
            )
            .unwrap();

        assert_eq!(result, ProofDocWriteResult::suppressed());
        assert!(rendered.is_empty());
        assert_eq!(clause.ident(), 42);
        assert_eq!(session.id_source.current_ident(), 0);
        assert!(!clause.query_prop(CP_INPUT_FORMULA));
    }

    #[test]
    fn doc_clause_modification_prints_binary_and_unary_pcl_steps_with_old_id() {
        let bank = test_bank();
        let mut session =
            ProofDocSession::new(ProofDocOutputFormat::Pcl, 2, ProblemType::FirstOrder);
        let mut clause = Clause::empty();
        clause.set_ident(7);
        let mut partner = Clause::empty();
        partner.set_ident(3);
        let mut rendered = String::new();

        let result = session
            .doc_clause_modification(
                &mut rendered,
                &bank,
                &mut clause,
                ClauseModificationInference::SimplifyReflect,
                Some(&partner),
                Some("simp"),
            )
            .unwrap();

        assert_eq!(result, ProofDocWriteResult::printed());
        assert_eq!(clause.ident(), 1);
        assert_eq!(rendered, "     1 : :[] : sr(7,3) : 'simp'\n");

        rendered.clear();
        clause.set_ident(9);
        session
            .doc_clause_modification(
                &mut rendered,
                &bank,
                &mut clause,
                ClauseModificationInference::Minimize,
                None,
                Some("min"),
            )
            .unwrap();

        assert_eq!(clause.ident(), 2);
        assert_eq!(rendered, "     2 : :[] : cn(9) : 'min'\n");
    }

    #[test]
    fn doc_clause_modification_prints_tstp_steps_with_old_id() {
        let bank = test_bank();
        let mut session =
            ProofDocSession::new(ProofDocOutputFormat::Tstp, 2, ProblemType::FirstOrder);
        let mut clause = Clause::empty();
        clause.set_ident(7);
        let mut rendered = String::new();

        session
            .doc_clause_modification(
                &mut rendered,
                &bank,
                &mut clause,
                ClauseModificationInference::Condense,
                None,
                Some("condensed"),
            )
            .unwrap();

        assert_eq!(
            rendered,
            "cnf(c_0_1, plain, ($false),inference(condense,[status(thm)],[c_0_7]),['condensed']).\n"
        );

        let mut partner = Clause::empty();
        partner.set_ident(5);
        clause.set_ident(8);
        rendered.clear();
        session
            .doc_clause_modification(
                &mut rendered,
                &bank,
                &mut clause,
                ClauseModificationInference::DestructiveEqualityResolution,
                Some(&partner),
                None,
            )
            .unwrap();

        assert_eq!(
            rendered,
            "cnf(c_0_2, plain, ($false),inference(er,[status(thm)],[c_0_8])).\n"
        );
    }

    #[test]
    fn doc_clause_modification_prints_ac_resolution_axiom_list() {
        let bank = test_bank();
        let mut session =
            ProofDocSession::new(ProofDocOutputFormat::Pcl, 2, ProblemType::FirstOrder);
        let mut clause = Clause::empty();
        clause.set_ident(7);
        let mut rendered = String::new();

        session
            .doc_clause_modification_with_evidence(
                &mut rendered,
                &bank,
                &mut clause,
                ClauseModificationInference::AcResolution,
                ClauseModificationEvidence::ac_resolution(&[70, 71]),
                Some("ac"),
            )
            .unwrap();

        assert_eq!(clause.ident(), 1);
        assert_eq!(rendered, "     1 : :[] : ar(7,70,71) : 'ac'\n");

        let mut session =
            ProofDocSession::new(ProofDocOutputFormat::Tstp, 2, ProblemType::FirstOrder);
        clause.set_ident(8);
        rendered.clear();
        session
            .doc_clause_modification_with_evidence(
                &mut rendered,
                &bank,
                &mut clause,
                ClauseModificationInference::AcResolution,
                ClauseModificationEvidence::ac_resolution(&[70, 71]),
                None,
            )
            .unwrap();

        assert_eq!(
            rendered,
            "cnf(c_0_1, plain, ($false),inference(ar,[status(thm)],[c_0_8,c_0_70,c_0_71])).\n"
        );
    }

    #[test]
    fn doc_clause_modification_preserves_c_unsupported_format_fallback_after_id_assignment() {
        let bank = test_bank();
        let mut session =
            ProofDocSession::new(ProofDocOutputFormat::NoFormat, 2, ProblemType::FirstOrder);
        let mut clause = Clause::empty();
        clause.set_ident(7);
        clause.set_prop(CP_INPUT_FORMULA);
        let mut rendered = String::new();

        let result = session
            .doc_clause_modification(
                &mut rendered,
                &bank,
                &mut clause,
                ClauseModificationInference::EvalAnswerLiteral,
                None,
                None,
            )
            .unwrap();

        assert_eq!(result, ProofDocWriteResult::printed());
        assert_eq!(clause.ident(), 1);
        assert!(!clause.query_prop(CP_INPUT_FORMULA));
        assert_eq!(rendered, "% Output format not implemented.\n");
    }

    #[test]
    fn doc_formula_creation_suppresses_below_level_two_without_assigning_id_or_clearing_input() {
        let mut session =
            ProofDocSession::new(ProofDocOutputFormat::Pcl, 1, ProblemType::FirstOrder);
        let mut formula = FormulaDocView::new(42, CP_TYPE_AXIOM | CP_INPUT_FORMULA, "p(a)");
        let mut rendered = String::new();

        let result = session
            .doc_formula_creation(
                &mut rendered,
                &mut formula,
                FormulaCreationInference::Initial,
                FormulaCreationParents::none(),
                None,
            )
            .unwrap();

        assert_eq!(result, ProofDocWriteResult::suppressed());
        assert!(rendered.is_empty());
        assert_eq!(formula.ident(), 42);
        assert!(formula.query_prop(CP_INPUT_FORMULA));
        assert_eq!(session.id_source.current_ident(), 0);
    }

    #[test]
    fn doc_formula_creation_prints_pcl_initial_intro_def_and_split_equiv() {
        let info = ClauseInfo::new(Some("form1"), Some("problem.p"), 3, 4);
        let mut session =
            ProofDocSession::new(ProofDocOutputFormat::Pcl, 2, ProblemType::FirstOrder);
        let mut formula =
            FormulaDocView::new(0, CP_TYPE_AXIOM | CP_INPUT_FORMULA, "p(a)").with_info(&info);
        let mut rendered = String::new();

        session
            .doc_formula_creation(
                &mut rendered,
                &mut formula,
                FormulaCreationInference::Initial,
                FormulaCreationParents::none(),
                Some("input"),
            )
            .unwrap();

        assert_eq!(formula.ident(), 1);
        assert_eq!(
            rendered,
            "     1 : :p(a) : initial(\"problem.p\", form1) : 'input'\n"
        );

        rendered.clear();
        let mut definition = FormulaDocView::new(0, CP_TYPE_AXIOM, "def");
        session
            .doc_formula_creation(
                &mut rendered,
                &mut definition,
                FormulaCreationInference::IntroDef,
                FormulaCreationParents::none(),
                Some("split"),
            )
            .unwrap();

        assert_eq!(definition.ident(), 2);
        assert_eq!(rendered, "     2 : :def : introduced : 'split'\n");

        rendered.clear();
        let parent = FormulaDocView::new(10, CP_TYPE_AXIOM, "left <=> right");
        let mut split = FormulaDocView::new(0, CP_TYPE_NEG_CONJECTURE, "left => right");
        session
            .doc_formula_creation(
                &mut rendered,
                &mut split,
                FormulaCreationInference::SplitEquiv,
                FormulaCreationParents::unary(&parent),
                None,
            )
            .unwrap();

        assert_eq!(split.ident(), 3);
        assert_eq!(rendered, "     3 : neg:left => right : split_equiv(10)\n");
    }

    #[test]
    fn doc_formula_creation_prints_tstp_initial_and_split_equiv() {
        let info = ClauseInfo::new(Some("form1"), Some("problem.p"), 3, 4);
        let mut session =
            ProofDocSession::new(ProofDocOutputFormat::Tstp, 2, ProblemType::FirstOrder);
        let mut formula =
            FormulaDocView::new(0, CP_TYPE_AXIOM | CP_INPUT_FORMULA, "p(a)").with_info(&info);
        let mut rendered = String::new();

        session
            .doc_formula_creation(
                &mut rendered,
                &mut formula,
                FormulaCreationInference::Initial,
                FormulaCreationParents::none(),
                Some("input"),
            )
            .unwrap();

        assert_eq!(
            rendered,
            "fof(c_0_1, axiom, p(a), file('problem.p', form1),['input']).\n"
        );

        rendered.clear();
        let parent = FormulaDocView::new(10, CP_TYPE_AXIOM, "left <=> right");
        let mut split = FormulaDocView::new(0, CP_TYPE_AXIOM, "left => right");
        session
            .doc_formula_creation(
                &mut rendered,
                &mut split,
                FormulaCreationInference::SplitEquiv,
                FormulaCreationParents::unary(&parent),
                None,
            )
            .unwrap();

        assert_eq!(
            rendered,
            "fof(c_0_2, plain, left => right, inference(split_equiv, [status(thm)], [c_0_10])).\n"
        );
    }

    #[test]
    fn doc_formula_modification_suppresses_below_level_two_but_clears_input_formula() {
        let mut session =
            ProofDocSession::new(ProofDocOutputFormat::Pcl, 1, ProblemType::FirstOrder);
        let mut formula = FormulaDocView::new(42, CP_TYPE_AXIOM | CP_INPUT_FORMULA, "p(a)");
        let mut rendered = String::new();

        let result = session
            .doc_formula_modification(
                &mut rendered,
                &mut formula,
                FormulaModificationInference::Simplification,
                None,
            )
            .unwrap();

        assert_eq!(result, ProofDocWriteResult::suppressed());
        assert!(rendered.is_empty());
        assert_eq!(formula.ident(), 42);
        assert!(!formula.query_prop(CP_INPUT_FORMULA));
        assert_eq!(session.id_source.current_ident(), 0);
    }

    #[test]
    fn doc_formula_modification_prints_pcl_and_tstp_parent_steps() {
        let mut session =
            ProofDocSession::new(ProofDocOutputFormat::Pcl, 2, ProblemType::FirstOrder);
        let mut formula = FormulaDocView::new(7, CP_TYPE_AXIOM | CP_INPUT_FORMULA, "p(a)");
        let mut rendered = String::new();

        session
            .doc_formula_modification(
                &mut rendered,
                &mut formula,
                FormulaModificationInference::Simplification,
                Some("simp"),
            )
            .unwrap();

        assert_eq!(formula.ident(), 1);
        assert!(!formula.query_prop(CP_INPUT_FORMULA));
        assert_eq!(
            rendered,
            "     1 : :p(a) : fof_simplification(7) : 'simp'\n"
        );

        let mut session =
            ProofDocSession::new(ProofDocOutputFormat::Tstp, 2, ProblemType::FirstOrder);
        let mut formula = FormulaDocView::new(8, CP_TYPE_NEG_CONJECTURE, "sk");
        rendered.clear();
        session
            .doc_formula_modification(
                &mut rendered,
                &mut formula,
                FormulaModificationInference::Skolemize,
                None,
            )
            .unwrap();

        assert_eq!(
            rendered,
            "fof(c_0_1, negated_conjecture, sk,inference(skolemize, [status(esa)], [c_0_8])).\n"
        );
    }

    #[test]
    fn doc_formula_modification_assigns_id_but_prints_nothing_for_unrepresented_op() {
        let mut session =
            ProofDocSession::new(ProofDocOutputFormat::Tstp, 2, ProblemType::FirstOrder);
        let mut formula = FormulaDocView::new(7, CP_TYPE_AXIOM | CP_INPUT_FORMULA, "p(a)");
        let mut rendered = String::new();

        let result = session
            .doc_formula_modification(
                &mut rendered,
                &mut formula,
                FormulaModificationInference::Other,
                Some("ignored"),
            )
            .unwrap();

        assert_eq!(result, ProofDocWriteResult::suppressed());
        assert!(rendered.is_empty());
        assert_eq!(formula.ident(), 1);
        assert!(!formula.query_prop(CP_INPUT_FORMULA));
        assert_eq!(session.id_source.current_ident(), 1);
    }

    #[test]
    fn doc_formula_helpers_preserve_c_unsupported_format_fallback_after_id_assignment() {
        let mut session =
            ProofDocSession::new(ProofDocOutputFormat::NoFormat, 2, ProblemType::FirstOrder);
        let mut formula = FormulaDocView::new(7, CP_TYPE_AXIOM | CP_INPUT_FORMULA, "p(a)");
        let mut rendered = String::new();

        session
            .doc_formula_creation(
                &mut rendered,
                &mut formula,
                FormulaCreationInference::Initial,
                FormulaCreationParents::none(),
                None,
            )
            .unwrap();

        assert_eq!(formula.ident(), 1);
        assert!(formula.query_prop(CP_INPUT_FORMULA));
        assert_eq!(rendered, "% Output format not implemented.\n");

        rendered.clear();
        session
            .doc_formula_modification(
                &mut rendered,
                &mut formula,
                FormulaModificationInference::NegConjecture,
                None,
            )
            .unwrap();

        assert_eq!(formula.ident(), 2);
        assert!(!formula.query_prop(CP_INPUT_FORMULA));
        assert_eq!(rendered, "% Output format not implemented.\n");
    }

    #[test]
    fn doc_clause_from_form_clears_input_and_prints_split_conjunct_parent() {
        let bank = test_bank();
        let parent = FormulaDocView::new(17, CP_TYPE_AXIOM, "p(a)");
        let mut session =
            ProofDocSession::new(ProofDocOutputFormat::Pcl, 2, ProblemType::FirstOrder);
        let mut clause = Clause::empty();
        clause.set_ident(42);
        clause.set_prop(CP_INPUT_FORMULA);
        let mut rendered = String::new();

        session
            .doc_clause_from_form(&mut rendered, &bank, &mut clause, &parent)
            .unwrap();

        assert_eq!(clause.ident(), 1);
        assert!(!clause.query_prop(CP_INPUT_FORMULA));
        assert_eq!(rendered, "     1 : :[] : split_conjunct(17)\n");

        let mut session =
            ProofDocSession::new(ProofDocOutputFormat::Tstp, 2, ProblemType::FirstOrder);
        clause.set_ident(43);
        clause.set_prop(CP_INPUT_FORMULA);
        rendered.clear();
        session
            .doc_clause_from_form(&mut rendered, &bank, &mut clause, &parent)
            .unwrap();

        assert_eq!(clause.ident(), 1);
        assert!(!clause.query_prop(CP_INPUT_FORMULA));
        assert_eq!(
            rendered,
            "cnf(c_0_1, plain, ($false),inference(split_conjunct, [status(thm)],[c_0_17])).\n"
        );
    }

    #[test]
    fn doc_clause_from_form_unsupported_format_clears_but_does_not_assign_id() {
        let bank = test_bank();
        let parent = FormulaDocView::new(17, CP_TYPE_AXIOM, "p(a)");
        let mut session =
            ProofDocSession::new(ProofDocOutputFormat::NoFormat, 2, ProblemType::FirstOrder);
        let mut clause = Clause::empty();
        clause.set_ident(42);
        clause.set_prop(CP_INPUT_FORMULA);
        let mut rendered = String::new();

        session
            .doc_clause_from_form(&mut rendered, &bank, &mut clause, &parent)
            .unwrap();

        assert_eq!(clause.ident(), 42);
        assert!(!clause.query_prop(CP_INPUT_FORMULA));
        assert_eq!(session.id_source.current_ident(), 0);
        assert_eq!(rendered, "% Output format not implemented.\n");
    }

    #[test]
    fn doc_clause_quote_prints_old_id_and_optional_partner_shapes() {
        let bank = test_bank();
        let mut session =
            ProofDocSession::new(ProofDocOutputFormat::Pcl, 2, ProblemType::FirstOrder);
        let mut clause = Clause::empty();
        clause.set_ident(7);
        clause.set_prop(CP_INPUT_FORMULA);
        let mut partner = Clause::empty();
        partner.set_ident(20);
        let mut rendered = String::new();

        session
            .doc_clause_quote(&mut rendered, &bank, 2, &mut clause, Some("proof"), None)
            .unwrap();

        assert_eq!(clause.ident(), 1);
        assert!(!clause.query_prop(CP_INPUT_FORMULA));
        assert_eq!(rendered, "     1 : :[] : 7 : 'proof'\n");

        clause.set_ident(8);
        rendered.clear();
        session
            .doc_clause_quote(
                &mut rendered,
                &bank,
                2,
                &mut clause,
                Some("subsumed"),
                Some(&partner),
            )
            .unwrap();

        assert_eq!(clause.ident(), 2);
        assert_eq!(rendered, "     2 : :[] : 8 : 'subsumed(20)'\n");

        let mut session =
            ProofDocSession::new(ProofDocOutputFormat::Tstp, 2, ProblemType::FirstOrder);
        clause.set_ident(9);
        rendered.clear();
        session
            .doc_clause_quote(
                &mut rendered,
                &bank,
                2,
                &mut clause,
                Some("subsumed"),
                Some(&partner),
            )
            .unwrap();

        assert_eq!(
            rendered,
            "cnf(c_0_1, plain, ($false), c_0_9,['subsumed(c_0_20)']).\n"
        );
    }

    #[test]
    fn doc_clause_quote_respects_target_level_and_unsupported_id_timing() {
        let bank = test_bank();
        let mut session =
            ProofDocSession::new(ProofDocOutputFormat::Pcl, 1, ProblemType::FirstOrder);
        let mut clause = Clause::empty();
        clause.set_ident(42);
        clause.set_prop(CP_INPUT_FORMULA);
        let mut rendered = String::new();

        let result = session
            .doc_clause_quote(&mut rendered, &bank, 2, &mut clause, Some("proof"), None)
            .unwrap();

        assert_eq!(result, ProofDocWriteResult::suppressed());
        assert!(rendered.is_empty());
        assert_eq!(clause.ident(), 42);
        assert!(!clause.query_prop(CP_INPUT_FORMULA));
        assert_eq!(session.id_source.current_ident(), 0);

        let mut session =
            ProofDocSession::new(ProofDocOutputFormat::NoFormat, 2, ProblemType::FirstOrder);
        clause.set_ident(43);
        rendered.clear();
        session
            .doc_clause_quote(&mut rendered, &bank, 2, &mut clause, Some("proof"), None)
            .unwrap();

        assert_eq!(clause.ident(), 43);
        assert_eq!(session.id_source.current_ident(), 0);
        assert_eq!(rendered, "% Output format not implemented.\n");
    }

    #[test]
    fn doc_split_def_rest_and_clause_apply_defs_match_c_comments_and_spacing() {
        let bank = test_bank();
        let parent = FormulaDocView::new(10, CP_TYPE_AXIOM, "p(a)");
        let mut session =
            ProofDocSession::new(ProofDocOutputFormat::Pcl, 2, ProblemType::FirstOrder);
        let mut clause = Clause::empty();
        let mut rendered = String::new();

        session
            .doc_intro_split_def_rest(&mut rendered, &bank, &mut clause, &parent, Some("ignored"))
            .unwrap();

        assert_eq!(clause.ident(), 1);
        assert_eq!(rendered, "     1 : :[] : split_equiv(10)\n");

        rendered.clear();
        session
            .doc_clause_apply_defs(
                &mut rendered,
                &bank,
                &mut clause,
                9,
                &[70, 71],
                Some("ignored"),
            )
            .unwrap();

        assert_eq!(clause.ident(), 2);
        assert_eq!(
            rendered,
            "     2 : :[] : apply_def(apply_def(9,70),71) : 'split'\n"
        );

        let mut session =
            ProofDocSession::new(ProofDocOutputFormat::Tstp, 2, ProblemType::FirstOrder);
        rendered.clear();
        session
            .doc_intro_split_def_rest(&mut rendered, &bank, &mut clause, &parent, None)
            .unwrap();

        assert_eq!(
            rendered,
            "cnf(c_0_1, plain, ($false),inference(split_equiv, [status(thm)],[c_0_10])).\n"
        );

        rendered.clear();
        session
            .doc_clause_apply_defs(&mut rendered, &bank, &mut clause, 9, &[70, 71], None)
            .unwrap();

        assert_eq!(
            rendered,
            "cnf(c_0_2, plain, ($false),inference(apply_def, [status(thm)],[inference(apply_def, [status(thm)],[c_0_9,c_0_70]),c_0_71]),['split']).\n"
        );
    }

    #[test]
    fn doc_formula_intro_defs_and_split_wrapper_match_c_dispatch() {
        let mut session =
            ProofDocSession::new(ProofDocOutputFormat::Pcl, 2, ProblemType::FirstOrder);
        let mut formula = FormulaDocView::new(7, CP_TYPE_AXIOM, "p(a)");
        let mut rendered = String::new();

        session
            .doc_formula_intro_defs(&mut rendered, &mut formula, &[20, 21], Some("defs"))
            .unwrap();

        assert_eq!(formula.ident(), 1);
        assert_eq!(
            rendered,
            "     1 : :p(a) : apply_def(apply_def(7,20),21) : 'defs'\n"
        );

        rendered.clear();
        let mut split_def = FormulaDocView::new(0, CP_TYPE_AXIOM, "def");
        session
            .doc_intro_split_def(&mut rendered, &mut split_def)
            .unwrap();

        assert_eq!(split_def.ident(), 2);
        assert_eq!(rendered, "     2 : :def : introduced : 'split'\n");

        let mut session =
            ProofDocSession::new(ProofDocOutputFormat::Tstp, 2, ProblemType::FirstOrder);
        formula.set_ident(7);
        rendered.clear();
        session
            .doc_formula_intro_defs(&mut rendered, &mut formula, &[20, 21], None)
            .unwrap();

        assert_eq!(
            rendered,
            "fof(c_0_1, plain, p(a), inference(apply_def,[status(thm)],[inference(apply_def,[status(thm)],[c_0_7,c_0_20]),c_0_21])).\n"
        );
    }

    #[test]
    fn doc_formula_intro_defs_unsupported_format_assigns_id_like_c() {
        let mut session =
            ProofDocSession::new(ProofDocOutputFormat::NoFormat, 2, ProblemType::FirstOrder);
        let mut formula = FormulaDocView::new(7, CP_TYPE_AXIOM, "p(a)");
        let mut rendered = String::new();

        session
            .doc_formula_intro_defs(&mut rendered, &mut formula, &[20], Some("defs"))
            .unwrap();

        assert_eq!(formula.ident(), 1);
        assert_eq!(session.id_source.current_ident(), 1);
        assert_eq!(rendered, "% Output format not implemented.\n");
    }

    #[test]
    fn doc_clause_rewrite_computes_demodulator_sequence_from_rewrite_links() {
        let mut bank = test_bank();
        let old = typed_const(&mut bank, "old");
        let nf = typed_const(&mut bank, "nf");
        let literal = eqn(&mut bank, &nf, &nf, true);
        let mut clause = Clause::alloc(EqnList::from_vec(vec![literal]));
        clause.set_ident(7);
        clause.set_prop(CP_INPUT_FORMULA);
        let mut rewritten = ClausePos::<()>::for_clause(clause);
        rewritten.set_side(EqnSide::LeftSide);
        let demodulator = RewriteDemodulator::new(17);
        term_add_rw_link(
            &old,
            &nf,
            Some(demodulator),
            false,
            RwResultType::LimitedRewritable,
        );
        let mut session =
            ProofDocSession::new(ProofDocOutputFormat::Pcl, 2, ProblemType::FirstOrder);
        session.pcl_shell_level = 1;
        let mut rendered = String::new();

        session
            .doc_clause_rewrite(&mut rendered, &bank, &mut rewritten, &old, Some("rw"))
            .unwrap();

        let clause = rewritten.clause().unwrap();
        assert_eq!(clause.ident(), 1);
        assert!(!clause.query_prop(CP_INPUT_FORMULA));
        assert_eq!(rendered, "     1 : : : rw(7,17) : 'rw'\n");

        let literal = eqn(&mut bank, &nf, &nf, true);
        let mut clause = Clause::alloc(EqnList::from_vec(vec![literal]));
        clause.set_ident(8);
        let mut rewritten = ClausePos::<()>::for_clause(clause);
        rewritten.set_side(EqnSide::LeftSide);
        let mut session =
            ProofDocSession::new(ProofDocOutputFormat::Tstp, 2, ProblemType::FirstOrder);
        rendered.clear();

        session
            .doc_clause_rewrite(&mut rendered, &bank, &mut rewritten, &old, None)
            .unwrap();

        assert_eq!(
            rendered,
            "cnf(c_0_1, plain, (nf=nf),inference(rw, [status(thm)],[c_0_8,c_0_17])).\n"
        );
    }

    #[test]
    fn doc_clause_rewrite_suppresses_below_level_but_clears_input() {
        let mut bank = test_bank();
        let old = typed_const(&mut bank, "old");
        let nf = typed_const(&mut bank, "nf");
        let literal = eqn(&mut bank, &nf, &nf, true);
        let mut clause = Clause::alloc(EqnList::from_vec(vec![literal]));
        clause.set_ident(7);
        clause.set_prop(CP_INPUT_FORMULA);
        let mut rewritten = ClausePos::<()>::for_clause(clause);
        let mut session =
            ProofDocSession::new(ProofDocOutputFormat::Pcl, 1, ProblemType::FirstOrder);
        let mut rendered = String::new();

        let result = session
            .doc_clause_rewrite(&mut rendered, &bank, &mut rewritten, &old, Some("rw"))
            .unwrap();

        let clause = rewritten.clause().unwrap();
        assert_eq!(result, ProofDocWriteResult::suppressed());
        assert!(rendered.is_empty());
        assert_eq!(clause.ident(), 7);
        assert!(!clause.query_prop(CP_INPUT_FORMULA));
        assert_eq!(session.id_source.current_ident(), 0);
    }

    #[test]
    fn doc_clause_eq_unfold_repeats_demodulator_for_each_position() {
        let bank = test_bank();
        let mut rewritten = Clause::empty();
        rewritten.set_ident(7);
        rewritten.set_prop(CP_INPUT_FORMULA);
        let mut demodulator = Clause::empty();
        demodulator.set_ident(30);
        let mut session =
            ProofDocSession::new(ProofDocOutputFormat::Pcl, 2, ProblemType::FirstOrder);
        let mut rendered = String::new();

        session
            .doc_clause_eq_unfold(&mut rendered, &bank, &mut rewritten, &demodulator, 2)
            .unwrap();

        assert_eq!(rewritten.ident(), 1);
        assert!(!rewritten.query_prop(CP_INPUT_FORMULA));
        assert_eq!(rendered, "     1 : :[] : rw(rw(7,30),30) : 'unfolding'\n");

        let mut session =
            ProofDocSession::new(ProofDocOutputFormat::Tstp, 2, ProblemType::FirstOrder);
        rewritten.set_ident(8);
        rendered.clear();
        session
            .doc_clause_eq_unfold(&mut rendered, &bank, &mut rewritten, &demodulator, 2)
            .unwrap();

        assert_eq!(
            rendered,
            "cnf(c_0_1, plain, ($false),inference(rw, [status(thm)],[inference(rw, [status(thm)],[c_0_8,c_0_30]),c_0_30]),['Unfolding']).\n"
        );
    }

    #[test]
    fn pcl_type_str_matches_c_explicit_roles() {
        assert_eq!(pcl_type_str(CP_TYPE_CONJECTURE), "conj");
        assert_eq!(pcl_type_str(CP_TYPE_QUESTION), "que");
        assert_eq!(pcl_type_str(CP_TYPE_NEG_CONJECTURE), "neg");
    }

    #[test]
    fn pcl_type_str_collapses_default_roles_to_empty_plain_axiom_surface() {
        for type_ in [
            CP_TYPE_UNKNOWN,
            CP_TYPE_AXIOM,
            CP_TYPE_HYPOTHESIS,
            CP_TYPE_LEMMA,
            CP_TYPE_WATCH_CLAUSE,
        ] {
            assert_eq!(pcl_type_str(type_), "");
        }
    }

    #[test]
    fn pcl_print_start_matches_c_spacing_and_clause_gate() {
        let bank = test_bank();
        let mut clause = Clause::empty();
        clause.set_ident(7);
        clause.set_tptp_type(CP_TYPE_NEG_CONJECTURE);

        let mut rendered = String::new();
        pcl_print_start(
            &mut rendered,
            &bank,
            &clause,
            true,
            PclStepPrintOptions::default(),
        )
        .unwrap();
        assert_eq!(rendered, "     7 : neg:[] : ");

        let mut rendered = String::new();
        pcl_print_start(
            &mut rendered,
            &bank,
            &clause,
            false,
            PclStepPrintOptions {
                compact: true,
                ..PclStepPrintOptions::default()
            },
        )
        .unwrap();
        assert_eq!(rendered, "7:neg: : ");
    }

    #[test]
    fn pcl_formula_print_start_matches_c_spacing_and_render_gate() {
        let mut rendered = String::new();
        pcl_formula_print_start(
            &mut rendered,
            7,
            CP_TYPE_NEG_CONJECTURE,
            Some("p(a)"),
            PclStepPrintOptions::default(),
        )
        .unwrap();
        assert_eq!(rendered, "     7 : neg:p(a) : ");

        let mut rendered = String::new();
        pcl_formula_print_start(
            &mut rendered,
            7,
            CP_TYPE_CONJECTURE,
            None,
            PclStepPrintOptions {
                compact: true,
                ..PclStepPrintOptions::default()
            },
        )
        .unwrap();
        assert_eq!(rendered, "7:conj: : ");
    }

    #[test]
    fn pcl_print_end_matches_c_comment_and_watchlist_spacing() {
        let plain = Clause::empty();
        let mut watch = Clause::empty();
        watch.set_prop(CP_WATCH_ONLY);

        let mut rendered = String::new();
        pcl_print_end(
            &mut rendered,
            &plain,
            Some("proof"),
            PclStepPrintOptions::default(),
        )
        .unwrap();
        assert_eq!(rendered, " : 'proof'\n");

        let mut rendered = String::new();
        pcl_print_end(
            &mut rendered,
            &plain,
            Some("proof"),
            PclStepPrintOptions {
                compact: true,
                ..PclStepPrintOptions::default()
            },
        )
        .unwrap();
        assert_eq!(rendered, ":'proof'\n");

        let mut rendered = String::new();
        pcl_print_end(
            &mut rendered,
            &watch,
            Some("proof"),
            PclStepPrintOptions::default(),
        )
        .unwrap();
        assert_eq!(rendered, ": 'wl,proof'\n");

        let mut rendered = String::new();
        pcl_print_end(&mut rendered, &watch, None, PclStepPrintOptions::default()).unwrap();
        assert_eq!(rendered, " : 'wl'\n");
    }

    #[test]
    fn tstp_print_end_matches_c_comment_and_watchlist_suffixes() {
        let plain = Clause::empty();
        let mut watch = Clause::empty();
        watch.set_prop(CP_WATCH_ONLY);

        let mut rendered = String::new();
        tstp_print_end(&mut rendered, &plain, Some("proof")).unwrap();
        assert_eq!(rendered, ",['proof']).\n");

        let mut rendered = String::new();
        tstp_print_end(&mut rendered, &watch, Some("proof")).unwrap();
        assert_eq!(rendered, ",['wl,proof']).\n");

        let mut rendered = String::new();
        tstp_print_end(&mut rendered, &watch, None).unwrap();
        assert_eq!(rendered, ",['wl']).\n");

        let mut rendered = String::new();
        tstp_print_end(&mut rendered, &plain, None).unwrap();
        assert_eq!(rendered, ").\n");
    }

    #[test]
    fn pcl_formula_print_end_matches_c_comment_spacing() {
        let mut rendered = String::new();
        pcl_formula_print_end(
            &mut rendered,
            Some("fof_simpl"),
            PclStepPrintOptions::default(),
        )
        .unwrap();
        assert_eq!(rendered, " : 'fof_simpl'\n");

        let mut rendered = String::new();
        pcl_formula_print_end(
            &mut rendered,
            Some("fof_simpl"),
            PclStepPrintOptions {
                compact: true,
                ..PclStepPrintOptions::default()
            },
        )
        .unwrap();
        assert_eq!(rendered, ":'fof_simpl'\n");

        let mut rendered = String::new();
        pcl_formula_print_end(&mut rendered, None, PclStepPrintOptions::default()).unwrap();
        assert_eq!(rendered, "\n");
    }

    #[test]
    fn tstp_formula_print_end_matches_c_comment_suffix() {
        let mut rendered = String::new();
        tstp_formula_print_end(&mut rendered, Some("fof_simpl")).unwrap();
        assert_eq!(rendered, ",['fof_simpl']).\n");

        let mut rendered = String::new();
        tstp_formula_print_end(&mut rendered, None).unwrap();
        assert_eq!(rendered, ").\n");
    }

    #[test]
    fn clause_binary_inference_rendering_matches_c_names_and_spacing() {
        let mut rendered = String::new();
        write_pcl_clause_binary_inference(
            &mut rendered,
            ClauseBinaryInference::Paramodulation,
            11,
            12,
        )
        .unwrap();
        assert_eq!(rendered, "pm(11,12)");

        let mut rendered = String::new();
        write_tstp_clause_binary_inference(
            &mut rendered,
            ClauseBinaryInference::SimultaneousParamodulation,
            11,
            12,
        )
        .unwrap();
        assert_eq!(rendered, "inference(spm,[status(thm)],[c_0_11,c_0_12])");

        let mut rendered = String::new();
        write_pcl_clause_binary_inference(
            &mut rendered,
            ClauseBinaryInference::SimplifyReflect,
            21,
            22,
        )
        .unwrap();
        assert_eq!(rendered, "sr(21,22)");

        let mut rendered = String::new();
        write_tstp_clause_binary_inference(
            &mut rendered,
            ClauseBinaryInference::ContextSimplifyReflect,
            21,
            22,
        )
        .unwrap();
        assert_eq!(rendered, "inference(csr,[status(thm)],[c_0_21,c_0_22])");
    }

    #[test]
    fn clause_unary_inference_rendering_matches_c_special_cases() {
        let mut rendered = String::new();
        write_pcl_clause_unary_inference(
            &mut rendered,
            ClauseUnaryInference::EqualityResolution,
            9,
        )
        .unwrap();
        assert_eq!(rendered, "er(9)");

        let mut rendered = String::new();
        write_tstp_clause_unary_inference(
            &mut rendered,
            ClauseUnaryInference::EqualityFactoring,
            9,
        )
        .unwrap();
        assert_eq!(rendered, "inference(ef,[status(thm)],[c_0_9])");

        let mut rendered = String::new();
        write_tstp_clause_unary_inference(&mut rendered, ClauseUnaryInference::Factoring, 9)
            .unwrap();
        assert_eq!(rendered, "inference(of,[status(thm)],[c_0_9])");

        let mut rendered = String::new();
        write_tstp_clause_unary_inference(&mut rendered, ClauseUnaryInference::Split, 9).unwrap();
        assert_eq!(rendered, "inference(split,[split(esplit,[])],[c_0_9])");

        let mut rendered = String::new();
        write_tstp_clause_unary_inference(&mut rendered, ClauseUnaryInference::SplitConjunct, 9)
            .unwrap();
        assert_eq!(rendered, "inference(split_conjunct, [status(thm)],[c_0_9])");

        let mut rendered = String::new();
        write_tstp_clause_unary_inference(
            &mut rendered,
            ClauseUnaryInference::EvalAnswerLiteral,
            9,
        )
        .unwrap();
        assert_eq!(
            rendered,
            "inference(eval_answer_literal,[status(thm)],[c_0_9, theory(answers)])"
        );

        let mut rendered = String::new();
        write_tstp_clause_unary_inference(&mut rendered, ClauseUnaryInference::Normalize, 9)
            .unwrap();
        assert_eq!(rendered, "inference(cn,[status(thm)],[c_0_9])");

        let mut rendered = String::new();
        write_tstp_clause_unary_inference(&mut rendered, ClauseUnaryInference::Condense, 9)
            .unwrap();
        assert_eq!(rendered, "inference(condense,[status(thm)],[c_0_9])");
    }

    #[test]
    fn clause_ac_resolution_inference_includes_all_signature_axioms() {
        let mut rendered = String::new();
        write_pcl_clause_ac_resolution_inference(&mut rendered, 3, &[70, 71]).unwrap();
        assert_eq!(rendered, "ar(3,70,71)");

        let mut rendered = String::new();
        write_tstp_clause_ac_resolution_inference(&mut rendered, 3, &[70, 71]).unwrap();
        assert_eq!(
            rendered,
            "inference(ar,[status(thm)],[c_0_3,c_0_70,c_0_71])"
        );
    }

    #[test]
    fn clause_rewrite_and_apply_defs_inferences_nest_like_c_stack_loops() {
        let mut rendered = String::new();
        write_pcl_clause_rewrite_inference(&mut rendered, 3, &[70, 71]).unwrap();
        assert_eq!(rendered, "rw(rw(3,70),71)");

        let mut rendered = String::new();
        write_tstp_clause_rewrite_inference(&mut rendered, 3, &[70, 71]).unwrap();
        assert_eq!(
            rendered,
            "inference(rw, [status(thm)],[inference(rw, [status(thm)],[c_0_3,c_0_70]),c_0_71])"
        );

        let mut rendered = String::new();
        write_pcl_clause_apply_defs_inference(&mut rendered, 3, &[70, 71]).unwrap();
        assert_eq!(rendered, "apply_def(apply_def(3,70),71)");

        let mut rendered = String::new();
        write_tstp_clause_apply_defs_inference(&mut rendered, 3, &[70, 71]).unwrap();
        assert_eq!(
            rendered,
            "inference(apply_def, [status(thm)],[inference(apply_def, [status(thm)],[c_0_3,c_0_70]),c_0_71])"
        );
    }

    #[test]
    fn formula_intro_def_inference_names_match_c_pcl_and_tstp_split() {
        let mut rendered = String::new();
        write_pcl_formula_intro_def_inference(&mut rendered).unwrap();
        assert_eq!(rendered, "introduced");

        let mut rendered = String::new();
        write_tstp_formula_intro_def_inference(&mut rendered).unwrap();
        assert_eq!(rendered, "introduced(definition)");
    }

    #[test]
    fn formula_parent_inference_rendering_matches_c_status_and_spacing() {
        let mut rendered = String::new();
        write_pcl_formula_parent_inference(
            &mut rendered,
            FormulaParentInference::Simplification,
            12,
        )
        .unwrap();
        assert_eq!(rendered, "fof_simplification(12)");

        let mut rendered = String::new();
        write_tstp_formula_parent_inference(&mut rendered, FormulaParentInference::SplitEquiv, 12)
            .unwrap();
        assert_eq!(rendered, "inference(split_equiv, [status(thm)], [c_0_12])");

        let mut rendered = String::new();
        write_tstp_formula_parent_inference(
            &mut rendered,
            FormulaParentInference::NegConjecture,
            12,
        )
        .unwrap();
        assert_eq!(
            rendered,
            "inference(assume_negation, [status(cth)],[c_0_12])"
        );

        let mut rendered = String::new();
        write_tstp_formula_parent_inference(&mut rendered, FormulaParentInference::Skolemize, 12)
            .unwrap();
        assert_eq!(rendered, "inference(skolemize, [status(esa)], [c_0_12])");

        let mut rendered = String::new();
        write_tstp_formula_parent_inference(
            &mut rendered,
            FormulaParentInference::AnnotateQuestion,
            12,
        )
        .unwrap();
        assert_eq!(
            rendered,
            "inference(add_answer_literal, [status(thm)],[c_0_12,theory(answers)])"
        );
    }

    #[test]
    fn formula_apply_defs_inference_nests_definitions_like_c_stack_loop() {
        let mut rendered = String::new();
        write_pcl_formula_apply_defs_inference(&mut rendered, 9, &[21, 22]).unwrap();
        assert_eq!(rendered, "apply_def(apply_def(9,21),22)");

        let mut rendered = String::new();
        write_tstp_formula_apply_defs_inference(&mut rendered, 9, &[21, 22]).unwrap();
        assert_eq!(
            rendered,
            "inference(apply_def,[status(thm)],[inference(apply_def,[status(thm)],[c_0_9,c_0_21]),c_0_22])"
        );

        let mut rendered = String::new();
        write_pcl_formula_apply_defs_inference(&mut rendered, 9, &[]).unwrap();
        assert_eq!(rendered, "9");

        let mut rendered = String::new();
        write_tstp_formula_apply_defs_inference(&mut rendered, 9, &[]).unwrap();
        assert_eq!(rendered, "c_0_9");
    }
}
