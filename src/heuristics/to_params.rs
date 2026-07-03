use crate::basics::defines::bool_to_str;
use crate::basics::error::{Diagnostic, ErrorCode};
use crate::basics::partial_orderings::HoOrderKind;
use crate::inout::basicparser::{parse_bool, parse_int};
use crate::inout::scanner::{describe_token, token_pos_rep, Scanner, TokenType};

pub const DEFAULT_LAMBDA_WEIGHT: i64 = 20;
pub const DEFAULT_DB_WEIGHT: i64 = 10;
pub const W_CONST_NO_SPECIAL_WEIGHT: i64 = -1;
pub const W_CONST_NO_WEIGHT: i64 = 0;

pub const TERM_ORDERING_NAMES: [&str; 10] = [
    "NoOrdering",
    "Optimize",
    "KBO",
    "KBO6",
    "LPO",
    "LPOCopy",
    "LPO4",
    "LPO4Copy",
    "RPO",
    "Empty",
];

pub const TO_PREC_GEN_NAMES: [&str; 20] = [
    "none",
    "unary_first",
    "unary_freq",
    "arity",
    "invarity",
    "const_max",
    "const_min",
    "freq",
    "invfreq",
    "invconjfreq",
    "invfreqconjmax",
    "invfreqconjmin",
    "invfreqconstmin",
    "invfreqhack",
    "typefreq",
    "invtypefreq",
    "combfreq",
    "invcombfreq",
    "arrayopt",
    "orient_axioms",
];

pub const TO_WEIGHT_GEN_NAMES: [&str; 35] = [
    "none",
    "firstmaximal0",
    "arity",
    "aritymax0",
    "modarity",
    "modaritymax0",
    "aritysquared",
    "aritysquaredmax0",
    "invarity",
    "invaritymax0",
    "invaritysquared",
    "invaritysquaredmax0",
    "precedence",
    "invprecedence",
    "precrank5",
    "precrank10",
    "precrank20",
    "freqcount",
    "invfreqcount",
    "freqrank",
    "invfreqrank",
    "invconjfreqrank",
    "freqranksquare",
    "invfreqranksquare",
    "invmodfreqrank",
    "invmodfreqrankmax0",
    "typefreqrank",
    "typefreqcount",
    "invtypefreqrank",
    "invtypefreqcount",
    "combfreqrank",
    "combfreqcount",
    "invcombfreqrank",
    "invcombfreqcount",
    "constant",
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i32)]
pub enum TermOrdering {
    NoOrdering = 0,
    Optimize = 1,
    Kbo = 2,
    Kbo6 = 3,
    Lpo = 4,
    LpoCopy = 5,
    Lpo4 = 6,
    Lpo4Copy = 7,
    Rpo = 8,
    Empty = 9,
}

impl TermOrdering {
    #[must_use]
    pub const fn from_c_value(value: i32) -> Option<Self> {
        match value {
            0 => Some(Self::NoOrdering),
            1 => Some(Self::Optimize),
            2 => Some(Self::Kbo),
            3 => Some(Self::Kbo6),
            4 => Some(Self::Lpo),
            5 => Some(Self::LpoCopy),
            6 => Some(Self::Lpo4),
            7 => Some(Self::Lpo4Copy),
            8 => Some(Self::Rpo),
            9 => Some(Self::Empty),
            _ => None,
        }
    }

    #[must_use]
    pub const fn c_value(self) -> i32 {
        self as i32
    }

    #[must_use]
    pub fn name(self) -> &'static str {
        TERM_ORDERING_NAMES[usize_from_c_enum(self.c_value())]
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i32)]
pub enum LiteralCmp {
    NoCmp = 0,
    Normal = 1,
    TfoEqMax = 2,
    TfoEqMin = 3,
}

impl LiteralCmp {
    #[must_use]
    pub const fn from_c_value(value: i32) -> Option<Self> {
        match value {
            0 => Some(Self::NoCmp),
            1 => Some(Self::Normal),
            2 => Some(Self::TfoEqMax),
            3 => Some(Self::TfoEqMin),
            _ => None,
        }
    }

    #[must_use]
    pub const fn c_value(self) -> i32 {
        self as i32
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i32)]
pub enum TOPrecGenMethod {
    InvalidEntry = -1,
    NoMethod = 0,
    UnaryFirst = 1,
    UnaryFirstFreq = 2,
    Arity = 3,
    InvArity = 4,
    ConstMax = 5,
    InvArConstMin = 6,
    ByFrequency = 7,
    ByInvFrequency = 8,
    ByInvConjFrequency = 9,
    ByInvFreqConjMax = 10,
    ByInvFreqConjMin = 11,
    ByInvFreqConstMin = 12,
    ByInvFreqHack = 13,
    ByTypeFreq = 14,
    ByInvTypeFreq = 15,
    ByCombFreq = 16,
    ByInvCombFreq = 17,
    ArrayOpt = 18,
    OrientAxioms = 19,
}

impl TOPrecGenMethod {
    #[must_use]
    pub const fn from_c_value(value: i32) -> Option<Self> {
        match value {
            -1 => Some(Self::InvalidEntry),
            0 => Some(Self::NoMethod),
            1 => Some(Self::UnaryFirst),
            2 => Some(Self::UnaryFirstFreq),
            3 => Some(Self::Arity),
            4 => Some(Self::InvArity),
            5 => Some(Self::ConstMax),
            6 => Some(Self::InvArConstMin),
            7 => Some(Self::ByFrequency),
            8 => Some(Self::ByInvFrequency),
            9 => Some(Self::ByInvConjFrequency),
            10 => Some(Self::ByInvFreqConjMax),
            11 => Some(Self::ByInvFreqConjMin),
            12 => Some(Self::ByInvFreqConstMin),
            13 => Some(Self::ByInvFreqHack),
            14 => Some(Self::ByTypeFreq),
            15 => Some(Self::ByInvTypeFreq),
            16 => Some(Self::ByCombFreq),
            17 => Some(Self::ByInvCombFreq),
            18 => Some(Self::ArrayOpt),
            19 => Some(Self::OrientAxioms),
            _ => None,
        }
    }

    #[must_use]
    pub const fn c_value(self) -> i32 {
        self as i32
    }

    #[must_use]
    pub fn name(self) -> Option<&'static str> {
        name_from_c_value(self.c_value(), &TO_PREC_GEN_NAMES)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i32)]
pub enum TOWeightGenMethod {
    InvalidEntry = -1,
    NoMethod = 0,
    SelectMaximal = 1,
    ArityWeight = 2,
    ArityMax0 = 3,
    ModArityWeight = 4,
    ModArityMax0 = 5,
    AritySqWeight = 6,
    AritySqMax0 = 7,
    InvArityWeight = 8,
    InvArityMax0 = 9,
    InvAritySqWeight = 10,
    InvAritySqMax0 = 11,
    Precedence = 12,
    PrecedenceInv = 13,
    PrecRank5 = 14,
    PrecRank10 = 15,
    PrecRank20 = 16,
    Frequency = 17,
    InvFrequency = 18,
    FrequencyRank = 19,
    InvFrequencyRank = 20,
    InvConjFrequencyRank = 21,
    FrequencyRankSq = 22,
    InvFrequencyRankSq = 23,
    InvModFreqRank = 24,
    InvModFreqRankMax0 = 25,
    TypeFrequencyRank = 26,
    TypeFrequencyCount = 27,
    InvTypeFrequencyRank = 28,
    InvTypeFrequencyCount = 29,
    CombFrequencyRank = 30,
    CombFrequencyCount = 31,
    InvCombFrequencyRank = 32,
    InvCombFrequencyCount = 33,
    ConstantWeight = 34,
}

impl TOWeightGenMethod {
    #[must_use]
    pub const fn from_c_value(value: i32) -> Option<Self> {
        match value {
            -1 => Some(Self::InvalidEntry),
            0 => Some(Self::NoMethod),
            1 => Some(Self::SelectMaximal),
            2 => Some(Self::ArityWeight),
            3 => Some(Self::ArityMax0),
            4 => Some(Self::ModArityWeight),
            5 => Some(Self::ModArityMax0),
            6 => Some(Self::AritySqWeight),
            7 => Some(Self::AritySqMax0),
            8 => Some(Self::InvArityWeight),
            9 => Some(Self::InvArityMax0),
            10 => Some(Self::InvAritySqWeight),
            11 => Some(Self::InvAritySqMax0),
            12 => Some(Self::Precedence),
            13 => Some(Self::PrecedenceInv),
            14 => Some(Self::PrecRank5),
            15 => Some(Self::PrecRank10),
            16 => Some(Self::PrecRank20),
            17 => Some(Self::Frequency),
            18 => Some(Self::InvFrequency),
            19 => Some(Self::FrequencyRank),
            20 => Some(Self::InvFrequencyRank),
            21 => Some(Self::InvConjFrequencyRank),
            22 => Some(Self::FrequencyRankSq),
            23 => Some(Self::InvFrequencyRankSq),
            24 => Some(Self::InvModFreqRank),
            25 => Some(Self::InvModFreqRankMax0),
            26 => Some(Self::TypeFrequencyRank),
            27 => Some(Self::TypeFrequencyCount),
            28 => Some(Self::InvTypeFrequencyRank),
            29 => Some(Self::InvTypeFrequencyCount),
            30 => Some(Self::CombFrequencyRank),
            31 => Some(Self::CombFrequencyCount),
            32 => Some(Self::InvCombFrequencyRank),
            33 => Some(Self::InvCombFrequencyCount),
            34 => Some(Self::ConstantWeight),
            _ => None,
        }
    }

    #[must_use]
    pub const fn c_value(self) -> i32 {
        self as i32
    }

    #[must_use]
    pub fn name(self) -> Option<&'static str> {
        name_from_c_value(self.c_value(), &TO_WEIGHT_GEN_NAMES)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OrderParmsCell {
    pub ordertype: TermOrdering,
    pub to_weight_gen: TOWeightGenMethod,
    pub to_prec_gen: TOPrecGenMethod,
    pub conj_only_mod: i64,
    pub conj_axiom_mod: i64,
    pub axiom_only_mod: i64,
    pub skolem_mod: i64,
    pub defpred_mod: i64,
    pub force_kbo_var_weight: bool,
    pub rewrite_strong_rhs_inst: bool,
    pub to_pre_prec: Option<String>,
    pub to_pre_weights: Option<String>,
    pub to_const_weight: i64,
    pub to_defs_min: bool,
    pub lit_cmp: i64,
    pub ho_order_kind: HoOrderKind,
    pub lam_w: i64,
    pub db_w: i64,
}

impl Default for OrderParmsCell {
    fn default() -> Self {
        Self {
            ordertype: TermOrdering::Kbo6,
            to_weight_gen: TOWeightGenMethod::NoMethod,
            to_prec_gen: TOPrecGenMethod::NoMethod,
            conj_only_mod: 0,
            conj_axiom_mod: 0,
            axiom_only_mod: 0,
            skolem_mod: 0,
            defpred_mod: 0,
            force_kbo_var_weight: false,
            rewrite_strong_rhs_inst: false,
            to_pre_prec: None,
            to_pre_weights: None,
            to_const_weight: W_CONST_NO_WEIGHT,
            to_defs_min: false,
            lit_cmp: i64::from(LiteralCmp::Normal.c_value()),
            ho_order_kind: HoOrderKind::LfhoOrder,
            lam_w: DEFAULT_LAMBDA_WEIGHT,
            db_w: DEFAULT_DB_WEIGHT,
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct OrderParmsParseReport {
    pub complete: bool,
    pub missing_fields: Vec<&'static str>,
    pub warnings: Vec<Diagnostic>,
}

pub fn order_parms_initialize(handle: &mut OrderParmsCell) {
    *handle = OrderParmsCell::default();
}

#[must_use]
pub fn order_parms_print_string(handle: &OrderParmsCell) -> String {
    format!(
        concat!(
            "   {{\n",
            "      ordertype:               {}\n",
            "      to_weight_gen:           {}\n",
            "      to_prec_gen:             {}\n",
            "      rewrite_strong_rhs_inst: {}\n",
            "      to_pre_prec:             \"{}\"\n",
            "      conj_only_mod:           {}\n",
            "      conj_axiom_mod:          {}\n",
            "      axiom_only_mod:          {}\n",
            "      skolem_mod:              {}\n",
            "      defpred_mod:             {}\n",
            "      force_kbo_var_weight:    {}\n",
            "      to_pre_weights:          \"{}\"\n",
            "      to_const_weight:         {}\n",
            "      to_defs_min:             {}\n",
            "      lit_cmp:                 {}\n",
            "      lam_w:                   {}\n",
            "      db_w:                    {}\n",
            "      ho_order_kind:           {}\n",
            "   }}\n"
        ),
        handle.ordertype.name(),
        handle.to_weight_gen.name().unwrap_or(""),
        handle.to_prec_gen.name().unwrap_or(""),
        bool_name(handle.rewrite_strong_rhs_inst),
        handle.to_pre_prec.as_deref().unwrap_or(""),
        handle.conj_only_mod,
        handle.conj_axiom_mod,
        handle.axiom_only_mod,
        handle.skolem_mod,
        handle.defpred_mod,
        bool_name(handle.force_kbo_var_weight),
        handle.to_pre_weights.as_deref().unwrap_or(""),
        handle.to_const_weight,
        bool_name(handle.to_defs_min),
        handle.lit_cmp,
        handle.lam_w,
        handle.db_w,
        ho_order_kind_name(handle.ho_order_kind)
    )
}

pub fn order_parms_parse_into(
    scanner: &mut Scanner,
    handle: &mut OrderParmsCell,
    warn_missing: bool,
) -> Result<bool, Diagnostic> {
    Ok(order_parms_parse_into_report(scanner, handle, warn_missing)?.complete)
}

pub fn order_parms_parse_into_report(
    scanner: &mut Scanner,
    handle: &mut OrderParmsCell,
    warn_missing: bool,
) -> Result<OrderParmsParseReport, Diagnostic> {
    let mut report = OrderParmsParseReport {
        complete: true,
        ..OrderParmsParseReport::default()
    };

    scanner.accept_tok(TokenType::OPEN_CURLY)?;
    parse_order_header_fields(scanner, handle, &mut report, warn_missing)?;
    parse_order_modifier_fields(scanner, handle, &mut report, warn_missing)?;
    parse_order_tail_fields(scanner, handle, &mut report, warn_missing)?;
    scanner.accept_tok(TokenType::CLOSE_CURLY)?;

    Ok(report)
}

fn parse_order_header_fields(
    scanner: &mut Scanner,
    handle: &mut OrderParmsCell,
    report: &mut OrderParmsParseReport,
    warn_missing: bool,
) -> Result<(), Diagnostic> {
    parse_term_ordering_field(scanner, handle, report, warn_missing)?;
    parse_weight_gen_field(scanner, handle, report, warn_missing)?;
    parse_prec_gen_field(scanner, handle, report, warn_missing)?;
    parse_bool_field(
        scanner,
        "rewrite_strong_rhs_inst",
        &mut handle.rewrite_strong_rhs_inst,
        report,
        warn_missing,
    )?;
    parse_string_field(
        scanner,
        "to_pre_prec",
        &mut handle.to_pre_prec,
        report,
        warn_missing,
    )?;
    Ok(())
}

fn parse_order_modifier_fields(
    scanner: &mut Scanner,
    handle: &mut OrderParmsCell,
    report: &mut OrderParmsParseReport,
    warn_missing: bool,
) -> Result<(), Diagnostic> {
    parse_int_field(
        scanner,
        "conj_only_mod",
        &mut handle.conj_only_mod,
        report,
        warn_missing,
    )?;
    parse_int_field(
        scanner,
        "conj_axiom_mod",
        &mut handle.conj_axiom_mod,
        report,
        warn_missing,
    )?;
    parse_int_field(
        scanner,
        "axiom_only_mod",
        &mut handle.axiom_only_mod,
        report,
        warn_missing,
    )?;
    parse_int_field(
        scanner,
        "skolem_mod",
        &mut handle.skolem_mod,
        report,
        warn_missing,
    )?;
    parse_int_field(
        scanner,
        "defpred_mod",
        &mut handle.defpred_mod,
        report,
        warn_missing,
    )?;
    Ok(())
}

fn parse_order_tail_fields(
    scanner: &mut Scanner,
    handle: &mut OrderParmsCell,
    report: &mut OrderParmsParseReport,
    warn_missing: bool,
) -> Result<(), Diagnostic> {
    parse_bool_field(
        scanner,
        "force_kbo_var_weight",
        &mut handle.force_kbo_var_weight,
        report,
        warn_missing,
    )?;
    parse_string_field(
        scanner,
        "to_pre_weights",
        &mut handle.to_pre_weights,
        report,
        warn_missing,
    )?;
    parse_int_field(
        scanner,
        "to_const_weight",
        &mut handle.to_const_weight,
        report,
        warn_missing,
    )?;
    parse_bool_field(
        scanner,
        "to_defs_min",
        &mut handle.to_defs_min,
        report,
        warn_missing,
    )?;
    parse_int_field(
        scanner,
        "lit_cmp",
        &mut handle.lit_cmp,
        report,
        warn_missing,
    )?;
    parse_int_field(scanner, "lam_w", &mut handle.lam_w, report, warn_missing)?;
    parse_int_field(scanner, "db_w", &mut handle.db_w, report, warn_missing)?;
    parse_ho_order_kind_field(scanner, handle, report, warn_missing)?;
    Ok(())
}

#[must_use]
pub fn to_translate_prec_gen_method(name: &str) -> TOPrecGenMethod {
    name_index(name, &TO_PREC_GEN_NAMES)
        .and_then(|index| TOPrecGenMethod::from_c_value(i32_from_usize(index)))
        .unwrap_or(TOPrecGenMethod::NoMethod)
}

#[must_use]
pub fn to_translate_weight_gen_method(name: &str) -> TOWeightGenMethod {
    name_index(name, &TO_WEIGHT_GEN_NAMES)
        .and_then(|index| TOWeightGenMethod::from_c_value(i32_from_usize(index)))
        .unwrap_or(TOWeightGenMethod::NoMethod)
}

#[must_use]
pub const fn ho_order_kind_name(kind: HoOrderKind) -> &'static str {
    match kind {
        HoOrderKind::LfhoOrder => "lfho",
        HoOrderKind::LambdaOrder => "lambda",
    }
}

pub fn str_to_ho_order_kind(value: &str) -> Result<HoOrderKind, Diagnostic> {
    match value {
        "lfho" => Ok(HoOrderKind::LfhoOrder),
        "lambda" => Ok(HoOrderKind::LambdaOrder),
        _ => Err(Diagnostic::new(
            ErrorCode::USAGE_ERROR,
            "Unknown HOOrderKind",
        )),
    }
}

fn parse_term_ordering_field(
    scanner: &mut Scanner,
    handle: &mut OrderParmsCell,
    report: &mut OrderParmsParseReport,
    warn_missing: bool,
) -> Result<(), Diagnostic> {
    if parse_field_prefix(scanner, "ordertype")? {
        let index = parse_named_index(scanner, &TERM_ORDERING_NAMES)?;
        handle.ordertype = TermOrdering::from_c_value(i32_from_usize(index)).ok_or_else(|| {
            Diagnostic::new(ErrorCode::OTHER_ERROR, "Term ordering name table mismatch")
        })?;
    } else {
        note_missing(report, "ordertype", warn_missing);
    }
    Ok(())
}

fn parse_weight_gen_field(
    scanner: &mut Scanner,
    handle: &mut OrderParmsCell,
    report: &mut OrderParmsParseReport,
    warn_missing: bool,
) -> Result<(), Diagnostic> {
    if parse_field_prefix(scanner, "to_weight_gen")? {
        let index = parse_named_index(scanner, &TO_WEIGHT_GEN_NAMES)?;
        handle.to_weight_gen =
            TOWeightGenMethod::from_c_value(i32_from_usize(index)).ok_or_else(|| {
                Diagnostic::new(
                    ErrorCode::OTHER_ERROR,
                    "Weight-generation name table mismatch",
                )
            })?;
    } else {
        note_missing(report, "to_weight_gen", warn_missing);
    }
    Ok(())
}

fn parse_prec_gen_field(
    scanner: &mut Scanner,
    handle: &mut OrderParmsCell,
    report: &mut OrderParmsParseReport,
    warn_missing: bool,
) -> Result<(), Diagnostic> {
    if parse_field_prefix(scanner, "to_prec_gen")? {
        let index = parse_named_index(scanner, &TO_PREC_GEN_NAMES)?;
        handle.to_prec_gen =
            TOPrecGenMethod::from_c_value(i32_from_usize(index)).ok_or_else(|| {
                Diagnostic::new(
                    ErrorCode::OTHER_ERROR,
                    "Precedence-generation name table mismatch",
                )
            })?;
    } else {
        note_missing(report, "to_prec_gen", warn_missing);
    }
    Ok(())
}

fn parse_bool_field(
    scanner: &mut Scanner,
    name: &'static str,
    target: &mut bool,
    report: &mut OrderParmsParseReport,
    warn_missing: bool,
) -> Result<(), Diagnostic> {
    if parse_field_prefix(scanner, name)? {
        *target = parse_bool(scanner)?;
    } else {
        note_missing(report, name, warn_missing);
    }
    Ok(())
}

fn parse_int_field(
    scanner: &mut Scanner,
    name: &'static str,
    target: &mut i64,
    report: &mut OrderParmsParseReport,
    warn_missing: bool,
) -> Result<(), Diagnostic> {
    if parse_field_prefix(scanner, name)? {
        *target = parse_int(scanner)?;
    } else {
        note_missing(report, name, warn_missing);
    }
    Ok(())
}

fn parse_string_field(
    scanner: &mut Scanner,
    name: &'static str,
    target: &mut Option<String>,
    report: &mut OrderParmsParseReport,
    warn_missing: bool,
) -> Result<(), Diagnostic> {
    if parse_field_prefix(scanner, name)? {
        let parsed = parse_c_string(scanner)?;
        *target = if parsed.is_empty() {
            None
        } else {
            Some(parsed)
        };
    } else {
        note_missing(report, name, warn_missing);
    }
    Ok(())
}

fn parse_ho_order_kind_field(
    scanner: &mut Scanner,
    handle: &mut OrderParmsCell,
    report: &mut OrderParmsParseReport,
    warn_missing: bool,
) -> Result<(), Diagnostic> {
    if parse_field_prefix(scanner, "ho_order_kind")? {
        scanner.check_tok(TokenType::STRING | TokenType::IDENTIFIER)?;
        handle.ho_order_kind = str_to_ho_order_kind(&scanner.current_token().literal())?;
        scanner.next_token()?;
    } else {
        note_missing(report, "ho_order_kind", warn_missing);
    }
    Ok(())
}

fn parse_field_prefix(scanner: &mut Scanner, name: &str) -> Result<bool, Diagnostic> {
    if scanner.test_id(name) {
        scanner.next_token()?;
        scanner.accept_tok(TokenType::COLON)?;
        Ok(true)
    } else {
        Ok(false)
    }
}

fn parse_named_index(scanner: &mut Scanner, names: &[&str]) -> Result<usize, Diagnostic> {
    scanner.check_tok(TokenType::IDENTIFIER)?;
    let literal = scanner.current_token().literal();
    let Some(index) = name_index(&literal, names) else {
        return Err(named_value_error(scanner, names));
    };
    scanner.next_token()?;
    Ok(index)
}

fn parse_c_string(scanner: &mut Scanner) -> Result<String, Diagnostic> {
    scanner.check_tok(TokenType::STRING)?;
    let bytes = scanner.current_token().literal_bytes();
    if bytes.len() < 2 {
        return Err(Diagnostic::new(
            ErrorCode::SYNTAX_ERROR,
            "Quoted string literal is too short",
        ));
    }
    let result = String::from_utf8_lossy(&bytes[1..bytes.len() - 1]).into_owned();
    scanner.next_token()?;
    Ok(result)
}

fn note_missing(report: &mut OrderParmsParseReport, name: &'static str, warn_missing: bool) {
    report.complete = false;
    report.missing_fields.push(name);
    if warn_missing {
        report.warnings.push(Diagnostic::new(
            ErrorCode::OTHER_ERROR,
            format!("Config misses {name}"),
        ));
    }
}

fn named_value_error(scanner: &Scanner, names: &[&str]) -> Diagnostic {
    Diagnostic::new(
        ErrorCode::SYNTAX_ERROR,
        format!(
            "{}(just read '{}'): Identifier ({}) expected, but {}('{}') read ",
            token_pos_rep(scanner.current_token()),
            scanner.current_token().literal(),
            names.join("|"),
            describe_token(scanner.current_token().kind()),
            scanner.current_token().literal()
        ),
    )
}

fn name_from_c_value(value: i32, names: &'static [&'static str]) -> Option<&'static str> {
    usize::try_from(value)
        .ok()
        .and_then(|index| names.get(index))
        .copied()
}

fn name_index(name: &str, names: &[&str]) -> Option<usize> {
    names.iter().position(|entry| *entry == name)
}

fn bool_name(value: bool) -> &'static str {
    bool_to_str(value)
}

fn usize_from_c_enum(value: i32) -> usize {
    usize::try_from(value).unwrap_or_else(|_| panic!("C enum value must be non-negative"))
}

fn i32_from_usize(value: usize) -> i32 {
    i32::try_from(value).unwrap_or_else(|_| panic!("small C name-table index must fit i32"))
}

#[cfg(test)]
mod tests {
    use super::{
        ho_order_kind_name, order_parms_initialize, order_parms_parse_into,
        order_parms_parse_into_report, order_parms_print_string, str_to_ho_order_kind,
        to_translate_prec_gen_method, to_translate_weight_gen_method, LiteralCmp, OrderParmsCell,
        TOPrecGenMethod, TOWeightGenMethod, TermOrdering, DEFAULT_DB_WEIGHT, DEFAULT_LAMBDA_WEIGHT,
        TERM_ORDERING_NAMES, TO_PREC_GEN_NAMES, TO_WEIGHT_GEN_NAMES, W_CONST_NO_SPECIAL_WEIGHT,
        W_CONST_NO_WEIGHT,
    };
    use crate::basics::error::ErrorCode;
    use crate::basics::partial_orderings::HoOrderKind;
    use crate::inout::scanner::Scanner;

    fn scanner(source: &str) -> Scanner {
        Scanner::from_user_string(source, false).unwrap_or_else(|err| panic!("{err}"))
    }

    #[test]
    fn enum_discriminants_and_public_name_tables_match_c() {
        assert_eq!(TermOrdering::NoOrdering.c_value(), 0);
        assert_eq!(TermOrdering::Kbo.c_value(), 2);
        assert_eq!(TermOrdering::Kbo6.c_value(), 3);
        assert_eq!(TermOrdering::Empty.c_value(), 9);
        assert_eq!(LiteralCmp::NoCmp.c_value(), 0);
        assert_eq!(LiteralCmp::Normal.c_value(), 1);
        assert_eq!(LiteralCmp::TfoEqMax.c_value(), 2);
        assert_eq!(LiteralCmp::TfoEqMin.c_value(), 3);

        assert_eq!(TOPrecGenMethod::InvalidEntry.c_value(), -1);
        assert_eq!(TOPrecGenMethod::NoMethod.c_value(), 0);
        assert_eq!(TOPrecGenMethod::ByInvFreqHack.c_value(), 13);
        assert_eq!(TOPrecGenMethod::ByTypeFreq.c_value(), 14);
        assert_eq!(TOPrecGenMethod::OrientAxioms.c_value(), 19);
        assert_eq!(TOWeightGenMethod::InvalidEntry.c_value(), -1);
        assert_eq!(TOWeightGenMethod::NoMethod.c_value(), 0);
        assert_eq!(TOWeightGenMethod::InvModFreqRankMax0.c_value(), 25);
        assert_eq!(TOWeightGenMethod::TypeFrequencyRank.c_value(), 26);
        assert_eq!(TOWeightGenMethod::ConstantWeight.c_value(), 34);

        assert_eq!(TERM_ORDERING_NAMES[3], "KBO6");
        assert_eq!(TO_PREC_GEN_NAMES[14], "typefreq");
        assert_eq!(TO_PREC_GEN_NAMES[19], "orient_axioms");
        assert_eq!(TO_WEIGHT_GEN_NAMES[26], "typefreqrank");
        assert_eq!(TO_WEIGHT_GEN_NAMES[34], "constant");
    }

    #[test]
    fn translate_methods_fall_back_to_no_method_for_unknown_names() {
        assert_eq!(
            to_translate_prec_gen_method("orient_axioms"),
            TOPrecGenMethod::OrientAxioms
        );
        assert_eq!(
            to_translate_prec_gen_method("does_not_exist"),
            TOPrecGenMethod::NoMethod
        );
        assert_eq!(
            to_translate_weight_gen_method("invmodfreqrankmax0"),
            TOWeightGenMethod::InvModFreqRankMax0
        );
        assert_eq!(
            to_translate_weight_gen_method("does_not_exist"),
            TOWeightGenMethod::NoMethod
        );
    }

    #[test]
    fn defaults_and_initializer_match_order_parms_initialize() {
        let mut params = OrderParmsCell {
            ordertype: TermOrdering::Lpo,
            lam_w: 99,
            ..OrderParmsCell::default()
        };

        order_parms_initialize(&mut params);

        assert_eq!(params.ordertype, TermOrdering::Kbo6);
        assert_eq!(params.to_weight_gen, TOWeightGenMethod::NoMethod);
        assert_eq!(params.to_prec_gen, TOPrecGenMethod::NoMethod);
        assert!(!params.rewrite_strong_rhs_inst);
        assert_eq!(params.to_pre_prec, None);
        assert_eq!(params.to_const_weight, W_CONST_NO_WEIGHT);
        assert!(!params.to_defs_min);
        assert_eq!(params.lit_cmp, i64::from(LiteralCmp::Normal.c_value()));
        assert_eq!(params.ho_order_kind, HoOrderKind::LfhoOrder);
        assert_eq!(params.lam_w, DEFAULT_LAMBDA_WEIGHT);
        assert_eq!(params.db_w, DEFAULT_DB_WEIGHT);
        assert_eq!(W_CONST_NO_SPECIAL_WEIGHT, -1);
    }

    #[test]
    fn print_string_matches_c_field_order_and_spacing() {
        let params = OrderParmsCell {
            ordertype: TermOrdering::Lpo4Copy,
            to_weight_gen: TOWeightGenMethod::InvModFreqRankMax0,
            to_prec_gen: TOPrecGenMethod::OrientAxioms,
            rewrite_strong_rhs_inst: true,
            to_pre_prec: Some("f>g".to_owned()),
            conj_only_mod: -1,
            conj_axiom_mod: 2,
            axiom_only_mod: 3,
            skolem_mod: 4,
            defpred_mod: 5,
            force_kbo_var_weight: true,
            to_pre_weights: Some("f=3".to_owned()),
            to_const_weight: -1,
            to_defs_min: true,
            lit_cmp: 99,
            lam_w: 40,
            db_w: 11,
            ho_order_kind: HoOrderKind::LambdaOrder,
        };

        assert_eq!(
            order_parms_print_string(&params),
            concat!(
                "   {\n",
                "      ordertype:               LPO4Copy\n",
                "      to_weight_gen:           invmodfreqrankmax0\n",
                "      to_prec_gen:             orient_axioms\n",
                "      rewrite_strong_rhs_inst: true\n",
                "      to_pre_prec:             \"f>g\"\n",
                "      conj_only_mod:           -1\n",
                "      conj_axiom_mod:          2\n",
                "      axiom_only_mod:          3\n",
                "      skolem_mod:              4\n",
                "      defpred_mod:             5\n",
                "      force_kbo_var_weight:    true\n",
                "      to_pre_weights:          \"f=3\"\n",
                "      to_const_weight:         -1\n",
                "      to_defs_min:             true\n",
                "      lit_cmp:                 99\n",
                "      lam_w:                   40\n",
                "      db_w:                    11\n",
                "      ho_order_kind:           lambda\n",
                "   }\n"
            )
        );
    }

    #[test]
    fn parse_full_cell_updates_every_field_and_consumes_close_curly() {
        let mut scanner = scanner(
            r#"{
                ordertype: LPO4Copy
                to_weight_gen: invmodfreqrankmax0
                to_prec_gen: orient_axioms
                rewrite_strong_rhs_inst: true
                to_pre_prec: "f>g"
                conj_only_mod: -1
                conj_axiom_mod: 2
                axiom_only_mod: 3
                skolem_mod: 4
                defpred_mod: 5
                force_kbo_var_weight: true
                to_pre_weights: "f=3"
                to_const_weight: -1
                to_defs_min: true
                lit_cmp: 99
                lam_w: 40
                db_w: 11
                ho_order_kind: lambda
            } rest"#,
        );
        let mut params = OrderParmsCell::default();

        let report = order_parms_parse_into_report(&mut scanner, &mut params, true)
            .unwrap_or_else(|err| panic!("{err}"));

        assert!(report.complete);
        assert!(report.missing_fields.is_empty());
        assert!(report.warnings.is_empty());
        assert_eq!(scanner.current_token().literal(), "rest");
        assert_eq!(params.ordertype, TermOrdering::Lpo4Copy);
        assert_eq!(params.to_weight_gen, TOWeightGenMethod::InvModFreqRankMax0);
        assert_eq!(params.to_prec_gen, TOPrecGenMethod::OrientAxioms);
        assert!(params.rewrite_strong_rhs_inst);
        assert_eq!(params.to_pre_prec.as_deref(), Some("f>g"));
        assert_eq!(params.conj_only_mod, -1);
        assert_eq!(params.conj_axiom_mod, 2);
        assert_eq!(params.axiom_only_mod, 3);
        assert_eq!(params.skolem_mod, 4);
        assert_eq!(params.defpred_mod, 5);
        assert!(params.force_kbo_var_weight);
        assert_eq!(params.to_pre_weights.as_deref(), Some("f=3"));
        assert_eq!(params.to_const_weight, -1);
        assert!(params.to_defs_min);
        assert_eq!(params.lit_cmp, 99);
        assert_eq!(params.lam_w, 40);
        assert_eq!(params.db_w, 11);
        assert_eq!(params.ho_order_kind, HoOrderKind::LambdaOrder);
    }

    #[test]
    fn parse_empty_strings_restore_null_optional_fields() {
        let mut scanner = scanner(r#"{ to_pre_prec: "" to_pre_weights: "" }"#);
        let mut params = OrderParmsCell {
            to_pre_prec: Some("existing".to_owned()),
            to_pre_weights: Some("weights".to_owned()),
            ..OrderParmsCell::default()
        };

        let complete =
            order_parms_parse_into(&mut scanner, &mut params, false).unwrap_or_else(|err| {
                panic!("{err}");
            });

        assert!(!complete);
        assert_eq!(params.to_pre_prec, None);
        assert_eq!(params.to_pre_weights, None);
    }

    #[test]
    fn parse_missing_fields_preserves_existing_values_and_collects_warnings() {
        let mut scanner = scanner("{ skolem_mod: 12 ho_order_kind: lfho } tail");
        let mut params = OrderParmsCell {
            ordertype: TermOrdering::Rpo,
            skolem_mod: 7,
            ..OrderParmsCell::default()
        };

        let report = order_parms_parse_into_report(&mut scanner, &mut params, true)
            .unwrap_or_else(|err| panic!("{err}"));

        assert!(!report.complete);
        assert!(report.missing_fields.contains(&"ordertype"));
        assert!(report.missing_fields.contains(&"to_weight_gen"));
        assert!(report.missing_fields.contains(&"db_w"));
        assert!(!report.missing_fields.contains(&"skolem_mod"));
        assert!(!report.missing_fields.contains(&"ho_order_kind"));
        assert_eq!(report.warnings.len(), report.missing_fields.len());
        assert_eq!(params.ordertype, TermOrdering::Rpo);
        assert_eq!(params.skolem_mod, 12);
        assert_eq!(params.ho_order_kind, HoOrderKind::LfhoOrder);
        assert_eq!(scanner.current_token().literal(), "tail");
    }

    #[test]
    fn parse_accepts_idnum_names_used_by_ordering_table() {
        let mut scanner = scanner("{ ordertype: KBO6 }");
        let mut params = OrderParmsCell {
            ordertype: TermOrdering::Lpo,
            ..OrderParmsCell::default()
        };

        let complete =
            order_parms_parse_into(&mut scanner, &mut params, false).unwrap_or_else(|err| {
                panic!("{err}");
            });

        assert!(!complete);
        assert_eq!(params.ordertype, TermOrdering::Kbo6);
    }

    #[test]
    fn parse_rejects_unknown_named_values_with_syntax_error() {
        let mut scanner = scanner("{ ordertype: Nope }");
        let mut params = OrderParmsCell::default();

        let error = order_parms_parse_into_report(&mut scanner, &mut params, false).unwrap_err();

        assert_eq!(error.code(), ErrorCode::SYNTAX_ERROR);
        assert!(error.message().contains("NoOrdering|Optimize|KBO|KBO6"));
    }

    #[test]
    fn ho_order_kind_helpers_match_c_macros_and_quoted_value_quirk() {
        assert_eq!(ho_order_kind_name(HoOrderKind::LfhoOrder), "lfho");
        assert_eq!(ho_order_kind_name(HoOrderKind::LambdaOrder), "lambda");
        assert_eq!(
            str_to_ho_order_kind("lambda").unwrap_or_else(|err| panic!("{err}")),
            HoOrderKind::LambdaOrder
        );

        let error = str_to_ho_order_kind("\"lambda\"").unwrap_err();
        assert_eq!(error.code(), ErrorCode::USAGE_ERROR);
        assert_eq!(error.message(), "Unknown HOOrderKind");

        let mut scanner = scanner(r#"{ ho_order_kind: "lfho" }"#);
        let mut params = OrderParmsCell::default();
        let error = order_parms_parse_into_report(&mut scanner, &mut params, false).unwrap_err();

        assert_eq!(error.code(), ErrorCode::USAGE_ERROR);
        assert_eq!(error.message(), "Unknown HOOrderKind");
    }
}
