use std::fmt;

use crate::basics::defines::DEFAULT_COMCHAR_RAW;
use crate::basics::error::{Diagnostic, ErrorCode};
use crate::basics::simple_stuff::ProblemType;
use crate::clauses::clause::{
    clause_write_pcl_with_options, clause_write_tstp_with_type_suffixes, Clause,
};
use crate::clauses::clause_props::{
    FormulaProperties, CP_TYPE_CONJECTURE, CP_TYPE_NEG_CONJECTURE, CP_TYPE_QUESTION, CP_WATCH_ONLY,
};
use crate::clauses::clauseinfo::{source_info_pcl_string, source_info_tstp_string};
use crate::clauses::eqn::EqnPrintOptions;
use crate::terms::termbanks::TermBank;

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
        ClauseUnaryInference, FormulaParentInference, PclStepPrintOptions, ProofDocOutputFormat,
        ProofDocSession, ProofDocWriteResult,
    };
    use crate::basics::simple_stuff::ProblemType;
    use crate::clauses::clause::Clause;
    use crate::clauses::clause_props::{
        CP_TYPE_AXIOM, CP_TYPE_CONJECTURE, CP_TYPE_HYPOTHESIS, CP_TYPE_LEMMA,
        CP_TYPE_NEG_CONJECTURE, CP_TYPE_QUESTION, CP_TYPE_UNKNOWN, CP_TYPE_WATCH_CLAUSE,
        CP_WATCH_ONLY,
    };
    use crate::clauses::clauseinfo::ClauseInfo;
    use crate::terms::signature::Signature;
    use crate::terms::termbanks::TermBank;
    use crate::terms::typebanks::TypeBank;

    fn test_bank() -> TermBank {
        TermBank::new(Signature::new(TypeBank::new())).unwrap()
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

        assert_eq!(result, ProofDocWriteResult::suppressed());
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
