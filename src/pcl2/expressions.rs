//! Port of `PCL2/pcl_expressions`.

use crate::basics::error::Diagnostic;
use crate::clauses::clauseinfo::ClauseInfo;
use crate::inout::scanner::{Scanner, TokenType};
use crate::pcl2::idents::PclId;
use crate::pcl2::positions::Pcl2Position;
use crate::pcl2::{current_error, parse_pos_int_as_long, strip_quote_core};
use std::fmt::Write as _;

pub const PCL_VAR_ARG: i64 = -1;

pub const PCL_EVALGC: &str = "evalgc";
pub const PCL_ER: &str = "er";
pub const PCL_PM: &str = "pm";
pub const PCL_SPM: &str = "spm";
pub const PCL_EF: &str = "ef";
pub const PCL_SAT: &str = "cdclpropres";
pub const PCL_SPLIT: &str = "split";
pub const TSTP_SPLIT_REFINED: &str = "esplit";
pub const TSTP_SPLIT_BASE: &str = "split";
pub const PCL_RW: &str = "rw";
pub const PCL_SR: &str = "sr";
pub const PCL_CSR: &str = "csr";
pub const PCL_ACRES: &str = "ar";
pub const PCL_CN: &str = "cn";
pub const PCL_CONDENSE: &str = "condense";
pub const PCL_SC: &str = "split_conjunct";
pub const PCL_SE: &str = "split_equiv";
pub const PCL_FS: &str = "fof_simplification";
pub const PCL_NNF: &str = "fof_nnf";
pub const PCL_ID: &str = "introduced";
pub const PCL_ID_DEF: &str = "introduced(definition)";
pub const PCL_AD: &str = "apply_def";
pub const PCL_SQ: &str = "shift_quantors";
pub const PCL_VR: &str = "variable_rename";
pub const PCL_SK: &str = "skolemize";
pub const PCL_DSTR: &str = "distribute";
pub const PCL_ANNOQ: &str = "add_answer_literal";
pub const PCL_EVANS: &str = "eval_answer_literal";
pub const PCL_NC: &str = "assume_negation";

const PCL_OPERATOR_IDS: &str = "evalgc|er|pm|spm|ef|cdclpropres|condense|rw|sr|csr|ar|cn|split|split_conjunct|split_equiv|fof_simplification|fof_nnf|introduced|apply_def|shift_quantors|variable_rename|skolemize|distribute|add_answer_literal|eval_answer_literal|assume_negation";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i32)]
pub enum PclOpCode {
    NoOp = 0,
    Initial = 1,
    IntroDef = 2,
    Quote = 3,
    EvalGc = 4,
    Paramod = 5,
    SimParamod = 6,
    EResolution = 7,
    SatCheck = 8,
    Condense = 9,
    EFactoring = 10,
    SimplifyReflect = 11,
    ContextSimplifyReflect = 12,
    ACResolution = 13,
    Rewrite = 14,
    URewrite = 15,
    ClauseNormalize = 16,
    SplitClause = 17,
    SplitEquiv = 18,
    ApplyDef = 19,
    FofSplitConjunct = 20,
    FofSimplify = 21,
    FofDeMorgan = 22,
    FofDistributeQuantors = 23,
    FofDistributeDisjunction = 24,
    AnnotateQuestion = 25,
    EvalAnswers = 26,
    FofVarRename = 27,
    FofSkolemize = 28,
    FofAssumeNegation = 29,
    MaxOp = 30,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PclQuote {
    Full(PclId),
    Mini(i64),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PclExprArgument {
    expr: Box<PclExpression>,
    position: Option<Pcl2Position>,
}

impl PclExprArgument {
    #[must_use]
    pub fn new(expr: PclExpression, position: Option<Pcl2Position>) -> Self {
        Self {
            expr: Box::new(expr),
            position,
        }
    }

    #[must_use]
    pub const fn expr(&self) -> &PclExpression {
        &self.expr
    }

    #[must_use]
    pub const fn position(&self) -> Option<&Pcl2Position> {
        self.position.as_ref()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PclExpressionData {
    None,
    Quote {
        quote: PclQuote,
        position: Option<Pcl2Position>,
    },
    Initial(Option<ClauseInfo>),
    Compound(Vec<PclExprArgument>),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PclExpression {
    op: PclOpCode,
    data: PclExpressionData,
}

impl Default for PclExpression {
    fn default() -> Self {
        Self::new()
    }
}

impl PclExpression {
    /// C `PCLExprAlloc`.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            op: PclOpCode::NoOp,
            data: PclExpressionData::None,
        }
    }

    #[must_use]
    pub const fn op(&self) -> PclOpCode {
        self.op
    }

    #[must_use]
    pub const fn data(&self) -> &PclExpressionData {
        &self.data
    }

    #[must_use]
    pub fn initial(info: Option<ClauseInfo>) -> Self {
        Self {
            op: PclOpCode::Initial,
            data: PclExpressionData::Initial(info),
        }
    }

    /// C `expr->arg_no`.
    #[must_use]
    pub fn arg_no(&self) -> usize {
        match &self.data {
            PclExpressionData::None => 0,
            PclExpressionData::Quote { .. } => 1,
            PclExpressionData::Initial(info) => usize::from(info.is_some()),
            PclExpressionData::Compound(args) => args.len(),
        }
    }

    /// C `PCLExprParse`.
    ///
    /// # Errors
    ///
    /// Returns scanner diagnostics when the expression syntax is invalid.
    pub fn parse(scanner: &mut Scanner, mini: bool) -> Result<Self, Diagnostic> {
        if scanner.test_tok(TokenType::POS_INT) {
            return parse_quote(scanner, mini);
        }
        if scanner.test_id("initial") {
            return parse_initial(scanner);
        }
        parse_compound(scanner, mini)
    }

    /// C `PCLExprPrint`.
    ///
    /// # Panics
    ///
    /// Panics for internally inconsistent expression/operator shapes, matching
    /// the C print-time assertions.
    #[must_use]
    pub fn print_string(&self, mini: bool) -> String {
        let mut output = String::new();
        self.write_pcl(&mut output, mini);
        output
    }

    /// C `PCLExprPrintTSTP`.
    ///
    /// # Panics
    ///
    /// Panics for internally inconsistent expression/operator shapes, matching
    /// the C print-time assertions.
    #[must_use]
    pub fn print_tstp_string(&self, mini: bool) -> String {
        let mut output = String::new();
        self.write_tstp(&mut output, mini);
        output
    }

    fn write_pcl(&self, output: &mut String, mini: bool) {
        match &self.data {
            PclExpressionData::Initial(info) => {
                if let Some(info) = info {
                    output.push_str(&info.source_info_pcl_string());
                } else {
                    output.push_str("initial");
                }
            }
            PclExpressionData::Quote { quote, position } => {
                write_quote_pcl(output, quote, mini);
                if let Some(position) = position {
                    output.push_str(&position.print_string());
                }
            }
            PclExpressionData::Compound(args) => {
                let op_name = self
                    .op
                    .pcl_name()
                    .unwrap_or_else(|| panic!("Unknown PCL operator"));
                output.push_str(op_name);
                self.op.assert_pcl_arg_count(args.len());
                if let Some((first, rest)) = args.split_first() {
                    output.push('(');
                    first.expr.write_pcl(output, mini);
                    if let Some(position) = &first.position {
                        output.push_str(&position.print_string());
                    }
                    for argument in rest {
                        output.push(',');
                        argument.expr.write_pcl(output, mini);
                        if let Some(position) = &argument.position {
                            output.push_str(&position.print_string());
                        }
                    }
                    output.push(')');
                }
            }
            PclExpressionData::None => panic!("Unknown PCL operator"),
        }
    }

    fn write_tstp(&self, output: &mut String, mini: bool) {
        match &self.data {
            PclExpressionData::Initial(info) => {
                if let Some(info) = info {
                    output.push_str(&info.source_info_tstp_string());
                } else {
                    output.push_str("unknown()");
                }
            }
            PclExpressionData::Quote { quote, .. } => write_quote_tstp(output, quote, mini),
            PclExpressionData::Compound(args) if self.op == PclOpCode::IntroDef => {
                assert!(
                    args.is_empty(),
                    "introduced PCL expression takes no arguments"
                );
                output.push_str(PCL_ID_DEF);
            }
            PclExpressionData::Compound(args) => {
                self.op.assert_tstp_arg_count(args.len());
                output.push_str("inference(");
                output.push_str(self.op.tstp_inference_head());
                let status = self.op.tstp_status();
                output.push_str(status);
                output.push_str(",[");
                let (first, rest) = args
                    .split_first()
                    .unwrap_or_else(|| panic!("PCL inference expression requires arguments"));
                first.expr.write_tstp(output, mini);
                for argument in rest {
                    output.push(',');
                    argument.expr.write_tstp(output, mini);
                }
                if self.op.needs_answer_theory() {
                    output.push_str(",theory(answers)");
                }
                output.push_str("])");
            }
            PclExpressionData::None => panic!("Unknown PCL operator"),
        }
    }
}

fn parse_quote(scanner: &mut Scanner, mini: bool) -> Result<PclExpression, Diagnostic> {
    let quote = if mini {
        PclQuote::Mini(parse_pos_int_as_long(scanner)?)
    } else {
        PclQuote::Full(PclId::parse(scanner)?)
    };
    let position = if scanner.test_tok(TokenType::OPEN_BRACKET) {
        Some(Pcl2Position::parse(scanner)?)
    } else {
        None
    };
    Ok(PclExpression {
        op: PclOpCode::Quote,
        data: PclExpressionData::Quote { quote, position },
    })
}

fn parse_initial(scanner: &mut Scanner) -> Result<PclExpression, Diagnostic> {
    scanner.next_token()?;
    let info = if scanner.test_tok(TokenType::OPEN_BRACKET) {
        scanner.next_token()?;
        scanner.check_tok(TokenType::STRING)?;
        let source = strip_quote_core(scanner.current_token().literal_bytes())?;
        scanner.next_token()?;
        scanner.accept_tok(TokenType::COMMA)?;
        let name = scanner.current_token().literal();
        scanner.accept_tok(TokenType::NAME | TokenType::POS_INT | TokenType::SQ_STRING)?;
        scanner.accept_tok(TokenType::CLOSE_BRACKET)?;
        Some(ClauseInfo::new(Some(&name), Some(&source), -1, -1))
    } else {
        None
    };
    Ok(PclExpression {
        op: PclOpCode::Initial,
        data: PclExpressionData::Initial(info),
    })
}

fn parse_compound(scanner: &mut Scanner, mini: bool) -> Result<PclExpression, Diagnostic> {
    scanner.check_id(PCL_OPERATOR_IDS)?;
    let literal = scanner.current_token().literal();
    let spec = operator_spec(&literal)
        .unwrap_or_else(|| unreachable!("checked PCL operator must have a spec"));
    scanner.next_token()?;
    let args = if spec.arg_requirement.allows_zero_without_parens() {
        Vec::new()
    } else {
        parse_compound_args(scanner, mini, spec.arg_requirement)?
    };
    Ok(PclExpression {
        op: spec.op,
        data: PclExpressionData::Compound(args),
    })
}

fn parse_compound_args(
    scanner: &mut Scanner,
    mini: bool,
    requirement: ArgRequirement,
) -> Result<Vec<PclExprArgument>, Diagnostic> {
    scanner.accept_tok(TokenType::OPEN_BRACKET)?;
    let mut args = Vec::new();
    args.push(parse_argument(scanner, mini)?);
    while scanner.test_tok(TokenType::COMMA) {
        scanner.next_token()?;
        args.push(parse_argument(scanner, mini)?);
    }
    if !requirement.accepts(args.len()) {
        return Err(current_error(
            scanner,
            "Wrong number of arguments in PCL expression",
        ));
    }
    scanner.accept_tok(TokenType::CLOSE_BRACKET)?;
    Ok(args)
}

fn parse_argument(scanner: &mut Scanner, mini: bool) -> Result<PclExprArgument, Diagnostic> {
    let expr = PclExpression::parse(scanner, mini)?;
    let position = if scanner.test_tok(TokenType::OPEN_BRACKET) {
        Some(Pcl2Position::parse(scanner)?)
    } else {
        None
    };
    Ok(PclExprArgument::new(expr, position))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ArgRequirement {
    Exact(usize),
    VariableAtLeastOne,
}

impl ArgRequirement {
    const fn allows_zero_without_parens(self) -> bool {
        matches!(self, Self::Exact(0))
    }

    const fn accepts(self, count: usize) -> bool {
        match self {
            Self::Exact(expected) => count == expected,
            Self::VariableAtLeastOne => count > 0,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct OperatorSpec {
    op: PclOpCode,
    arg_requirement: ArgRequirement,
}

fn operator_spec(name: &str) -> Option<OperatorSpec> {
    let (op, arg_requirement) = match name {
        PCL_EVALGC => (PclOpCode::EvalGc, ArgRequirement::Exact(1)),
        PCL_ER => (PclOpCode::EResolution, ArgRequirement::Exact(1)),
        PCL_PM => (PclOpCode::Paramod, ArgRequirement::Exact(2)),
        PCL_SPM => (PclOpCode::SimParamod, ArgRequirement::Exact(2)),
        PCL_EF => (PclOpCode::EFactoring, ArgRequirement::Exact(1)),
        PCL_SAT => (PclOpCode::SatCheck, ArgRequirement::VariableAtLeastOne),
        PCL_CONDENSE => (PclOpCode::Condense, ArgRequirement::Exact(1)),
        PCL_RW => (PclOpCode::Rewrite, ArgRequirement::Exact(2)),
        PCL_SR => (PclOpCode::SimplifyReflect, ArgRequirement::Exact(2)),
        PCL_CSR => (PclOpCode::ContextSimplifyReflect, ArgRequirement::Exact(2)),
        PCL_ACRES => (PclOpCode::ACResolution, ArgRequirement::VariableAtLeastOne),
        PCL_CN => (PclOpCode::ClauseNormalize, ArgRequirement::Exact(1)),
        PCL_SPLIT => (PclOpCode::SplitClause, ArgRequirement::Exact(1)),
        PCL_SC => (PclOpCode::FofSplitConjunct, ArgRequirement::Exact(1)),
        PCL_SE => (PclOpCode::SplitEquiv, ArgRequirement::Exact(1)),
        PCL_FS => (PclOpCode::FofSimplify, ArgRequirement::Exact(1)),
        PCL_NNF => (PclOpCode::FofDeMorgan, ArgRequirement::Exact(1)),
        PCL_ID => (PclOpCode::IntroDef, ArgRequirement::Exact(0)),
        PCL_AD => (PclOpCode::ApplyDef, ArgRequirement::Exact(2)),
        PCL_SQ => (PclOpCode::FofDistributeQuantors, ArgRequirement::Exact(1)),
        PCL_VR => (PclOpCode::FofVarRename, ArgRequirement::Exact(1)),
        PCL_SK => (PclOpCode::FofSkolemize, ArgRequirement::Exact(1)),
        PCL_DSTR => (
            PclOpCode::FofDistributeDisjunction,
            ArgRequirement::Exact(1),
        ),
        PCL_ANNOQ => (PclOpCode::AnnotateQuestion, ArgRequirement::Exact(1)),
        PCL_EVANS => (PclOpCode::EvalAnswers, ArgRequirement::Exact(1)),
        PCL_NC => (PclOpCode::FofAssumeNegation, ArgRequirement::Exact(1)),
        _ => return None,
    };
    Some(OperatorSpec {
        op,
        arg_requirement,
    })
}

impl PclOpCode {
    fn pcl_name(self) -> Option<&'static str> {
        match self {
            Self::IntroDef => Some(PCL_ID),
            Self::Paramod => Some(PCL_PM),
            Self::SimParamod => Some(PCL_SPM),
            Self::EResolution => Some(PCL_ER),
            Self::EvalGc => Some(PCL_EVALGC),
            Self::EFactoring => Some(PCL_EF),
            Self::SatCheck => Some(PCL_SAT),
            Self::Condense => Some(PCL_CONDENSE),
            Self::SimplifyReflect => Some(PCL_SR),
            Self::ContextSimplifyReflect => Some(PCL_CSR),
            Self::ACResolution => Some(PCL_ACRES),
            Self::Rewrite => Some(PCL_RW),
            Self::ClauseNormalize => Some(PCL_CN),
            Self::ApplyDef => Some(PCL_AD),
            Self::SplitClause => Some(PCL_SPLIT),
            Self::FofSplitConjunct => Some(PCL_SC),
            Self::SplitEquiv => Some(PCL_SE),
            Self::FofSimplify => Some(PCL_FS),
            Self::FofDeMorgan => Some(PCL_NNF),
            Self::FofDistributeQuantors => Some(PCL_SQ),
            Self::AnnotateQuestion => Some(PCL_ANNOQ),
            Self::EvalAnswers => Some(PCL_EVANS),
            Self::FofDistributeDisjunction => Some(PCL_DSTR),
            Self::FofVarRename => Some(PCL_VR),
            Self::FofSkolemize => Some(PCL_SK),
            Self::FofAssumeNegation => Some(PCL_NC),
            Self::NoOp | Self::Initial | Self::Quote | Self::URewrite | Self::MaxOp => None,
        }
    }

    fn assert_pcl_arg_count(self, count: usize) {
        match self {
            Self::IntroDef => assert_eq!(count, 0),
            Self::Paramod
            | Self::SimParamod
            | Self::SimplifyReflect
            | Self::ContextSimplifyReflect
            | Self::Rewrite
            | Self::ApplyDef => assert_eq!(count, 2),
            Self::SatCheck | Self::ACResolution => assert!(count > 0),
            Self::EResolution
            | Self::EvalGc
            | Self::EFactoring
            | Self::Condense
            | Self::ClauseNormalize
            | Self::SplitClause
            | Self::FofSplitConjunct
            | Self::SplitEquiv
            | Self::FofSimplify
            | Self::FofDeMorgan
            | Self::FofDistributeQuantors
            | Self::AnnotateQuestion
            | Self::EvalAnswers
            | Self::FofDistributeDisjunction
            | Self::FofVarRename
            | Self::FofSkolemize
            | Self::FofAssumeNegation => assert_eq!(count, 1),
            Self::NoOp | Self::Initial | Self::Quote | Self::URewrite | Self::MaxOp => {
                panic!("Unknown PCL operator")
            }
        }
    }

    fn assert_tstp_arg_count(self, count: usize) {
        self.assert_pcl_arg_count(count);
    }

    fn tstp_inference_head(self) -> &'static str {
        match self {
            Self::SplitClause => "split,[split(esplit,[])]",
            _ => self
                .pcl_name()
                .unwrap_or_else(|| panic!("Unknown PCL operator")),
        }
    }

    fn tstp_status(self) -> &'static str {
        match self {
            Self::SplitClause => "",
            Self::FofSkolemize => ",[status(esa)]",
            Self::FofAssumeNegation => ",[status(cth)]",
            Self::NoOp
            | Self::Initial
            | Self::Quote
            | Self::IntroDef
            | Self::URewrite
            | Self::MaxOp => ",[status(unknown)]",
            _ => ",[status(thm)]",
        }
    }

    const fn needs_answer_theory(self) -> bool {
        matches!(self, Self::AnnotateQuestion | Self::EvalAnswers)
    }
}

fn write_quote_pcl(output: &mut String, quote: &PclQuote, mini: bool) {
    match (quote, mini) {
        (PclQuote::Mini(id), true) => {
            let _ = write!(output, "{id}");
        }
        (PclQuote::Full(id), false) => output.push_str(&id.print_string()),
        (PclQuote::Mini(_), false) | (PclQuote::Full(_), true) => {
            panic!("PCL quote kind does not match mini/full print mode")
        }
    }
}

fn write_quote_tstp(output: &mut String, quote: &PclQuote, mini: bool) {
    match (quote, mini) {
        (PclQuote::Mini(id), true) => {
            let _ = write!(output, "{id}");
        }
        (PclQuote::Full(id), false) => output.push_str(&id.print_tstp_string()),
        (PclQuote::Mini(_), false) | (PclQuote::Full(_), true) => {
            panic!("PCL quote kind does not match mini/full print mode")
        }
    }
}

/// C `PCLStepExtract`.
#[must_use]
pub fn pcl_step_extract(extra: Option<&str>) -> bool {
    let Some(mut extra) = extra else {
        return false;
    };
    if extra.starts_with('"') || extra.starts_with('\'') {
        extra = &extra[1..];
    }
    extra.starts_with("proof") || extra.starts_with("final") || extra.starts_with("extract")
}

#[cfg(test)]
mod tests {
    use super::{
        pcl_step_extract, PclExprArgument, PclExpression, PclExpressionData, PclOpCode, PclQuote,
    };
    use crate::inout::scanner::Scanner;
    use crate::pcl2::idents::PclId;
    use crate::pcl2::positions::Pcl2Position;

    fn parse(source: &str, mini: bool) -> PclExpression {
        let mut scanner = Scanner::from_user_string(source, false).unwrap();
        PclExpression::parse(&mut scanner, mini).unwrap()
    }

    fn parse_id(source: &str) -> PclId {
        let mut scanner = Scanner::from_user_string(source, false).unwrap();
        PclId::parse(&mut scanner).unwrap()
    }

    fn parse_position(source: &str) -> Pcl2Position {
        let mut scanner = Scanner::from_user_string(source, false).unwrap();
        Pcl2Position::parse(&mut scanner).unwrap()
    }

    #[test]
    fn enum_values_match_c_order() {
        let opcodes = [
            PclOpCode::NoOp,
            PclOpCode::Initial,
            PclOpCode::IntroDef,
            PclOpCode::Quote,
            PclOpCode::EvalGc,
            PclOpCode::Paramod,
            PclOpCode::SimParamod,
            PclOpCode::EResolution,
            PclOpCode::SatCheck,
            PclOpCode::Condense,
            PclOpCode::EFactoring,
            PclOpCode::SimplifyReflect,
            PclOpCode::ContextSimplifyReflect,
            PclOpCode::ACResolution,
            PclOpCode::Rewrite,
            PclOpCode::URewrite,
            PclOpCode::ClauseNormalize,
            PclOpCode::SplitClause,
            PclOpCode::SplitEquiv,
            PclOpCode::ApplyDef,
            PclOpCode::FofSplitConjunct,
            PclOpCode::FofSimplify,
            PclOpCode::FofDeMorgan,
            PclOpCode::FofDistributeQuantors,
            PclOpCode::FofDistributeDisjunction,
            PclOpCode::AnnotateQuestion,
            PclOpCode::EvalAnswers,
            PclOpCode::FofVarRename,
            PclOpCode::FofSkolemize,
            PclOpCode::FofAssumeNegation,
            PclOpCode::MaxOp,
        ];

        for (discriminant, opcode) in opcodes.into_iter().enumerate() {
            assert_eq!(opcode as usize, discriminant);
        }
    }

    #[test]
    fn alloc_initializes_c_defaults() {
        let expr = PclExpression::new();
        assert_eq!(expr.op(), PclOpCode::NoOp);
        assert_eq!(expr.arg_no(), 0);
        assert!(matches!(expr.data(), PclExpressionData::None));
    }

    #[test]
    fn parses_and_prints_full_quotes() {
        let expr = parse("12.3", false);
        assert_eq!(expr.op(), PclOpCode::Quote);
        assert_eq!(expr.arg_no(), 1);
        assert_eq!(expr.print_string(false), "12.3");
        assert_eq!(expr.print_tstp_string(false), "pclid12_3");
        let PclExpressionData::Quote {
            quote: PclQuote::Full(id),
            position,
        } = expr.data()
        else {
            panic!("quote expected");
        };
        assert_eq!(id.elements(), [12, 3]);
        assert!(position.is_none());
    }

    #[test]
    fn parses_mini_quote_as_single_integer_and_leaves_compound_tail() {
        let mut scanner = Scanner::from_user_string("12.3", false).unwrap();
        let expr = PclExpression::parse(&mut scanner, true).unwrap();
        assert_eq!(expr.print_string(true), "12");
        assert_eq!(expr.print_tstp_string(true), "12");
        assert_eq!(scanner.current_token().literal(), ".");
    }

    #[test]
    fn parses_initial_without_and_with_source_info() {
        let plain = parse("initial", false);
        assert_eq!(plain.print_string(false), "initial");
        assert_eq!(plain.print_tstp_string(false), "unknown()");

        let sourced = parse(r#"initial("problem.p", ax1)"#, false);
        assert_eq!(sourced.print_string(false), r#"initial("problem.p", ax1)"#);
        assert_eq!(sourced.print_tstp_string(false), "file('problem.p', ax1)");
        assert_eq!(sourced.arg_no(), 1);
    }

    #[test]
    fn parses_fixed_arity_compound_expression() {
        let expr = parse("pm(1,2.3)", false);
        assert_eq!(expr.op(), PclOpCode::Paramod);
        assert_eq!(expr.arg_no(), 2);
        assert_eq!(expr.print_string(false), "pm(1,2.3)");
        assert_eq!(
            expr.print_tstp_string(false),
            "inference(pm,[status(thm)],[1,pclid2_3])"
        );
    }

    #[test]
    fn parses_variable_arity_compound_expression() {
        let expr = parse("ar(1,2,3)", false);
        assert_eq!(expr.op(), PclOpCode::ACResolution);
        assert_eq!(expr.print_string(false), "ar(1,2,3)");
        assert_eq!(
            expr.print_tstp_string(false),
            "inference(ar,[status(thm)],[1,2,3])"
        );
    }

    #[test]
    fn every_c_parser_operator_round_trips_and_has_exact_tstp_output() {
        let cases = [
            (
                "evalgc(1)",
                PclOpCode::EvalGc,
                "inference(evalgc,[status(thm)],[1])",
            ),
            (
                "er(1)",
                PclOpCode::EResolution,
                "inference(er,[status(thm)],[1])",
            ),
            (
                "pm(1,2)",
                PclOpCode::Paramod,
                "inference(pm,[status(thm)],[1,2])",
            ),
            (
                "spm(1,2)",
                PclOpCode::SimParamod,
                "inference(spm,[status(thm)],[1,2])",
            ),
            (
                "ef(1)",
                PclOpCode::EFactoring,
                "inference(ef,[status(thm)],[1])",
            ),
            (
                "cdclpropres(1,2)",
                PclOpCode::SatCheck,
                "inference(cdclpropres,[status(thm)],[1,2])",
            ),
            (
                "condense(1)",
                PclOpCode::Condense,
                "inference(condense,[status(thm)],[1])",
            ),
            (
                "rw(1,2)",
                PclOpCode::Rewrite,
                "inference(rw,[status(thm)],[1,2])",
            ),
            (
                "sr(1,2)",
                PclOpCode::SimplifyReflect,
                "inference(sr,[status(thm)],[1,2])",
            ),
            (
                "csr(1,2)",
                PclOpCode::ContextSimplifyReflect,
                "inference(csr,[status(thm)],[1,2])",
            ),
            (
                "ar(1,2)",
                PclOpCode::ACResolution,
                "inference(ar,[status(thm)],[1,2])",
            ),
            (
                "cn(1)",
                PclOpCode::ClauseNormalize,
                "inference(cn,[status(thm)],[1])",
            ),
            (
                "split(1)",
                PclOpCode::SplitClause,
                "inference(split,[split(esplit,[])],[1])",
            ),
            (
                "split_conjunct(1)",
                PclOpCode::FofSplitConjunct,
                "inference(split_conjunct,[status(thm)],[1])",
            ),
            (
                "split_equiv(1)",
                PclOpCode::SplitEquiv,
                "inference(split_equiv,[status(thm)],[1])",
            ),
            (
                "fof_simplification(1)",
                PclOpCode::FofSimplify,
                "inference(fof_simplification,[status(thm)],[1])",
            ),
            (
                "fof_nnf(1)",
                PclOpCode::FofDeMorgan,
                "inference(fof_nnf,[status(thm)],[1])",
            ),
            ("introduced", PclOpCode::IntroDef, "introduced(definition)"),
            (
                "apply_def(1,2)",
                PclOpCode::ApplyDef,
                "inference(apply_def,[status(thm)],[1,2])",
            ),
            (
                "shift_quantors(1)",
                PclOpCode::FofDistributeQuantors,
                "inference(shift_quantors,[status(thm)],[1])",
            ),
            (
                "variable_rename(1)",
                PclOpCode::FofVarRename,
                "inference(variable_rename,[status(thm)],[1])",
            ),
            (
                "skolemize(1)",
                PclOpCode::FofSkolemize,
                "inference(skolemize,[status(esa)],[1])",
            ),
            (
                "distribute(1)",
                PclOpCode::FofDistributeDisjunction,
                "inference(distribute,[status(thm)],[1])",
            ),
            (
                "add_answer_literal(1)",
                PclOpCode::AnnotateQuestion,
                "inference(add_answer_literal,[status(thm)],[1,theory(answers)])",
            ),
            (
                "eval_answer_literal(1)",
                PclOpCode::EvalAnswers,
                "inference(eval_answer_literal,[status(thm)],[1,theory(answers)])",
            ),
            (
                "assume_negation(1)",
                PclOpCode::FofAssumeNegation,
                "inference(assume_negation,[status(cth)],[1])",
            ),
        ];

        for (source, opcode, tstp) in cases {
            let expression = parse(source, false);
            assert_eq!(expression.op(), opcode, "{source}");
            assert_eq!(expression.print_string(false), source, "{source}");
            assert_eq!(expression.print_tstp_string(false), tstp, "{source}");
        }
    }

    #[test]
    fn variable_arity_storage_grows_to_large_proofs_without_layout_sentinels() {
        let arguments = (0..2_048)
            .map(|argument| argument.to_string())
            .collect::<Vec<_>>()
            .join(",");
        let source = format!("ar({arguments})");
        let expression = parse(&source, false);
        let PclExpressionData::Compound(stored) = expression.data() else {
            panic!("compound expression expected");
        };

        assert_eq!(stored.len(), 2_048);
        assert!(stored.capacity() >= stored.len());
        assert_eq!(expression.print_string(false), source);
    }

    #[test]
    fn prints_tstp_special_cases() {
        assert_eq!(
            parse("split(7)", false).print_tstp_string(false),
            "inference(split,[split(esplit,[])],[7])"
        );
        assert_eq!(
            parse("skolemize(7)", false).print_tstp_string(false),
            "inference(skolemize,[status(esa)],[7])"
        );
        assert_eq!(
            parse("assume_negation(7)", false).print_tstp_string(false),
            "inference(assume_negation,[status(cth)],[7])"
        );
        assert_eq!(
            parse("add_answer_literal(7)", false).print_tstp_string(false),
            "inference(add_answer_literal,[status(thm)],[7,theory(answers)])"
        );
        assert_eq!(
            parse("introduced", false).print_tstp_string(false),
            "introduced(definition)"
        );
    }

    #[test]
    fn rejects_wrong_fixed_arity_expression() {
        let mut scanner = Scanner::from_user_string("pm(1)", false).unwrap();
        let error = PclExpression::parse(&mut scanner, false).unwrap_err();
        assert!(error
            .message()
            .contains("Wrong number of arguments in PCL expression"));
    }

    #[test]
    fn preserves_c_position_guard_mismatch_in_expression_parser() {
        let mut scanner = Scanner::from_user_string("pm(1(2),3)", false).unwrap();
        let error = PclExpression::parse(&mut scanner, false).unwrap_err();
        assert!(error.message().contains("Integer"));
        assert_eq!(scanner.current_token().literal(), "(");
    }

    #[test]
    fn stored_positions_print_in_pcl_and_are_omitted_from_tstp_like_c() {
        let position = parse_position("3.L.12.5");
        let quote = PclExpression {
            op: PclOpCode::Quote,
            data: PclExpressionData::Quote {
                quote: PclQuote::Full(parse_id("7")),
                position: Some(position.clone()),
            },
        };
        assert_eq!(quote.print_string(false), "73.L125");
        assert_eq!(quote.print_tstp_string(false), "7");

        let compound = PclExpression {
            op: PclOpCode::Paramod,
            data: PclExpressionData::Compound(vec![
                PclExprArgument::new(parse("1", false), Some(position)),
                PclExprArgument::new(parse("2", false), None),
            ]),
        };
        assert_eq!(compound.print_string(false), "pm(13.L125,2)");
        assert_eq!(
            compound.print_tstp_string(false),
            "inference(pm,[status(thm)],[1,2])"
        );
    }

    #[test]
    fn quote_position_guard_has_the_same_c_parser_mismatch() {
        let mut scanner = Scanner::from_user_string("1(2)", false).unwrap();
        let error = PclExpression::parse(&mut scanner, false).unwrap_err();
        assert!(error.message().contains("Integer"));
        assert_eq!(scanner.current_token().literal(), "(");
    }

    #[test]
    fn step_extract_matches_c_prefix_logic() {
        assert!(!pcl_step_extract(None));
        assert!(pcl_step_extract(Some("proof root")));
        assert!(pcl_step_extract(Some("'final answer'")));
        assert!(pcl_step_extract(Some("\"extract this\"")));
        assert!(pcl_step_extract(Some("proofless")));
        assert!(!pcl_step_extract(Some("other proof")));
    }
}
