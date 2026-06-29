use crate::basics::error::{Diagnostic, ErrorCode};
use crate::clauses::f_generality::GeneralityMeasure;
use crate::inout::basicparser::parse_float;
use crate::inout::scanner::{token_pos_rep, Scanner, TokenType};
use std::sync::atomic::{AtomicU64, Ordering as AtomicOrdering};

static AX_FILTER_AUTO_ID: AtomicU64 = AtomicU64::new(0);

pub const GENERALITY_MEASURE_NAMES: [&str; 10] = [
    "None",
    "CountTerms",
    "CountLiterals",
    "CountFormulas",
    "CoutPosFormulas",
    "CountPosLiterals",
    "CountPosTerms",
    "CoutNegFormulas",
    "CountNegLiterals",
    "CountNegTerms",
];

pub const AX_FILTER_DEFAULT_SET: &str = "\
   threshold010000=Threshold(10000)
   LambdaDef=LambdaDef
   gf500_gu_R04_F100_L20000=GSinE(CountFormulas, ,false,   5.0,, 4,20000,1.0)
   gf120_gu_RUU_F100_L00500=GSinE(CountFormulas, ,false,   1.2,,,  500,1.0)
   gf120_gu_R02_F100_L20000=GSinE(CountFormulas, ,false,   1.2,, 2,20000,1.0)
   gf150_gu_RUU_F100_L20000=GSinE(CountFormulas, ,false,   1.5,,,20000,1.0)
   gf120_gu_RUU_F100_L00100=GSinE(CountFormulas, ,false,   1.2,,,  100,1.0)
   gf200_gu_R03_F100_L20000=GSinE(CountFormulas, ,false,   2.0,, 3,20000,1.0)
   gf600_gu_R05_F100_L20000=GSinE(CountFormulas, ,false,   6.0,, 5,20000,1.0, false)
   gf200_gu_RUU_F100_L20000=GSinE(CountFormulas, ,false,   2.0,,  ,20000,1.0)
   gf120_gu_RUU_F100_L01000=GSinE(CountFormulas, ,false,   1.2,,  , 1000,1.0, false)
   gf500_h_gu_R04_F100_L20000=GSinE(CountFormulas, hypos,false,   5.0,, 4,20000,1.0, false)
   gf120_h_gu_RUU_F100_L00500=GSinE(CountFormulas, hypos,false,   1.2,,,  500,1.0)
   gf120_h_gu_R02_F100_L20000=GSinE(CountFormulas, hypos,false,   1.2,, 2,20000,1.0)
   gf150_h_gu_RUU_F100_L20000=GSinE(CountFormulas, hypos,false,   1.5,,,20000,1.0)
   gf120_h_gu_RUU_F100_L00100=GSinE(CountFormulas, hypos,false,   1.2,,,  100,1.0)
   gf200_h_gu_R03_F100_L20000=GSinE(CountFormulas, hypos,false,   2.0,, 3,20000,1.0)
   gf600_h_gu_R05_F100_L20000=GSinE(CountFormulas, hypos,false,   6.0,, 5,20000,1.0,false)
   gf200_h_gu_RUU_F100_L20000=GSinE(CountFormulas, hypos,false,   2.0,,  ,20000,1.0)
   gf120_h_gu_RUU_F100_L01000=GSinE(CountFormulas, hypos,false,   1.2,,  , 1000,1.0)
   gf600_gu_R05_F100_L20000add=GSinE(CountFormulas, ,false,   6.0,, 5,20000,1.0,addnosymb)
";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i32)]
pub enum AxFilterType {
    NoFilter = 0,
    GSinE = 1,
    Threshold = 2,
    LambdaDefines = 3,
}

#[derive(Clone, Debug, PartialEq)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "AxFilterCell mirrors the C parameter record"
)]
pub struct AxFilter {
    pub name: Option<String>,
    pub type_: AxFilterType,
    pub gen_measure: GeneralityMeasure,
    pub use_hypotheses: bool,
    pub benevolence: f64,
    pub generosity: i64,
    pub max_recursion_depth: i64,
    pub max_set_size: i64,
    pub max_set_fraction: f64,
    pub add_no_symbol_axioms: bool,
    pub trim_implications: bool,
    pub defined_symbols_in_drel: bool,
    pub threshold: i64,
}

impl Default for AxFilter {
    fn default() -> Self {
        Self {
            name: None,
            type_: AxFilterType::NoFilter,
            gen_measure: GeneralityMeasure::NoMeasure,
            use_hypotheses: false,
            benevolence: 1.0,
            generosity: i64::MAX,
            max_recursion_depth: i64::from(i32::MAX),
            max_set_size: i64::MAX,
            max_set_fraction: 1.0,
            add_no_symbol_axioms: false,
            trim_implications: false,
            defined_symbols_in_drel: false,
            threshold: 0,
        }
    }
}

impl AxFilter {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn threshold(threshold: i64) -> Self {
        Self {
            type_: AxFilterType::Threshold,
            threshold,
            ..Self::default()
        }
    }

    #[must_use]
    pub fn lambda_defines() -> Self {
        Self {
            type_: AxFilterType::LambdaDefines,
            ..Self::default()
        }
    }

    #[must_use]
    pub fn g_sine(gen_measure: GeneralityMeasure) -> Self {
        Self {
            type_: AxFilterType::GSinE,
            gen_measure,
            ..Self::default()
        }
    }

    /// Parses a single unnamed axiom filter definition.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if the scanner is not positioned at a recognized
    /// filter or if any filter argument violates the C grammar.
    pub fn parse(scanner: &mut Scanner) -> Result<Self, Diagnostic> {
        scanner.check_id("GSinE|Threshold|LambdaDef")?;
        if scanner.test_id("GSinE") {
            return parse_g_sine(scanner);
        }
        if scanner.test_id("Threshold") {
            return parse_threshold(scanner);
        }
        if scanner.test_id("LambdaDef") {
            return parse_lambda_def(scanner);
        }
        Err(current_error(scanner, "Unknown axiom filter"))
    }

    /// Parses `[name=]<filter>` and assigns a C-shaped anonymous name when the
    /// name part is omitted.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if the optional name or the following filter
    /// definition is malformed.
    pub fn parse_definition(scanner: &mut Scanner) -> Result<Self, Diagnostic> {
        let name = if scanner
            .look_token(1)
            .kind()
            .intersects(TokenType::EQUAL_SIGN)
        {
            scanner.check_tok(TokenType::IDENTIFIER)?;
            let name = scanner.current_token().literal();
            scanner.next_token()?;
            scanner.accept_tok(TokenType::EQUAL_SIGN)?;
            name
        } else {
            let id = AX_FILTER_AUTO_ID.fetch_add(1, AtomicOrdering::Relaxed);
            format!("axfilter_auto{id:4}")
        };
        let mut result = Self::parse(scanner)?;
        result.name = Some(name);
        Ok(result)
    }

    #[must_use]
    /// # Panics
    ///
    /// Panics for `AxFilterType::NoFilter`, matching the C default assertion
    /// for unknown filter types.
    pub fn print_string(&self) -> String {
        match self.type_ {
            AxFilterType::GSinE => format!(
                "GSinE({}, {}, {}, {:.6}, {}, {}, {}, {:.6}, {}, {})",
                generality_measure_name(self.gen_measure),
                if self.use_hypotheses {
                    "hypos"
                } else {
                    "nohypos"
                },
                if self.defined_symbols_in_drel {
                    "true"
                } else {
                    "false"
                },
                self.benevolence,
                self.generosity,
                self.max_recursion_depth,
                self.max_set_size,
                self.max_set_fraction,
                if self.add_no_symbol_axioms {
                    "addnosymb"
                } else {
                    "ignorenosymb"
                },
                if self.trim_implications {
                    "true"
                } else {
                    "false"
                }
            ),
            AxFilterType::Threshold => format!("Threshold({})", self.threshold),
            AxFilterType::LambdaDefines => "LambdaDef".to_owned(),
            AxFilterType::NoFilter => panic!("unknown AxFilter type: {:?}", self.type_),
        }
    }

    #[must_use]
    pub fn print_buf_string(&self, buflen: usize) -> Option<String> {
        let result = self.print_string();
        (result.len() < buflen).then_some(result)
    }

    #[must_use]
    /// # Panics
    ///
    /// Panics under the same conditions as [`Self::print_string`].
    pub fn def_print_string(&self) -> String {
        format!(
            "{} = {}",
            self.name.as_deref().unwrap_or(""),
            self.print_string()
        )
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct AxFilterSet {
    filters: Vec<AxFilter>,
}

impl AxFilterSet {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn elements(&self) -> usize {
        self.filters.len()
    }

    pub fn add_filter(&mut self, filter: AxFilter) {
        self.filters.push(filter);
    }

    /// Parses filter definitions until the current token is no longer an
    /// identifier.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if any contained filter definition is malformed.
    pub fn parse(&mut self, scanner: &mut Scanner) -> Result<i64, Diagnostic> {
        let mut parsed = 0;
        while scanner.test_tok(TokenType::IDENTIFIER) {
            self.add_filter(AxFilter::parse_definition(scanner)?);
            parsed += 1;
        }
        Ok(parsed)
    }

    /// Creates a filter set from an internal string, matching
    /// `AxFilterSetCreateInternal`.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if scanner creation or filter parsing fails.
    pub fn create_internal(source: &str) -> Result<Self, Diagnostic> {
        let mut scanner = Scanner::from_internal_string(source, true)?;
        let mut result = Self::new();
        result.parse(&mut scanner)?;
        Ok(result)
    }

    /// Creates the built-in C `AxFilterDefaultSet`.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if the embedded default-set string fails to parse.
    pub fn default_set() -> Result<Self, Diagnostic> {
        Self::create_internal(AX_FILTER_DEFAULT_SET)
    }

    #[must_use]
    pub fn get_filter(&self, index: usize) -> Option<&AxFilter> {
        self.filters.get(index)
    }

    #[must_use]
    pub fn find_filter(&self, name: &str) -> Option<&AxFilter> {
        self.filters
            .iter()
            .find(|filter| filter.name.as_deref() == Some(name))
    }

    #[must_use]
    pub fn names_string(&self) -> String {
        self.filters
            .iter()
            .map(|filter| filter.name.as_deref().unwrap_or(""))
            .collect::<Vec<_>>()
            .join(", ")
    }

    #[must_use]
    pub fn print_string(&self) -> String {
        let mut result = String::new();
        for filter in &self.filters {
            result.push_str(&filter.def_print_string());
            result.push('\n');
        }
        result
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SineFilterResolution {
    filter: AxFilter,
    filters: AxFilterSet,
}

impl SineFilterResolution {
    #[must_use]
    pub const fn filter(&self) -> &AxFilter {
        &self.filter
    }

    #[must_use]
    pub const fn filters(&self) -> &AxFilterSet {
        &self.filters
    }
}

/// Resolves a `SInE` filter string as C `sine_get_filter` does in the normal
/// build: built-in names are looked up in `AxFilterDefaultSet`, while direct
/// unnamed definitions such as `Threshold(10)` are parsed and appended to that
/// default set.
///
/// # Errors
///
/// Returns a diagnostic if the input is not name-starting, a direct definition
/// is malformed, or a named filter is absent from the default set.
pub fn sine_get_filter(source: &str) -> Result<SineFilterResolution, Diagnostic> {
    let mut scanner = Scanner::from_option_string(source, true)?;
    scanner.check_tok(TokenType::NAME)?;
    let mut filters = AxFilterSet::default_set()?;

    let filter = if scanner
        .look_token(1)
        .kind()
        .intersects(TokenType::OPEN_BRACKET)
    {
        let filter = AxFilter::parse_definition(&mut scanner)?;
        filters.add_filter(filter.clone());
        filter
    } else {
        filters
            .find_filter(source)
            .cloned()
            .ok_or_else(|| unknown_sine_filter_error(source, &filters))?
    };

    Ok(SineFilterResolution { filter, filters })
}

fn unknown_sine_filter_error(source: &str, filters: &AxFilterSet) -> Diagnostic {
    Diagnostic::new(
        ErrorCode::USAGE_ERROR,
        format!(
            "Unknown SinE-filter '{source}' selected (valid choices: {})",
            filters.names_string()
        ),
    )
}

#[must_use]
pub fn get_gen_measure(name: &str) -> GeneralityMeasure {
    GENERALITY_MEASURE_NAMES
        .iter()
        .position(|candidate| *candidate == name)
        .and_then(generality_measure_from_index)
        .unwrap_or(GeneralityMeasure::NoMeasure)
}

#[must_use]
pub fn generality_measure_name(measure: GeneralityMeasure) -> &'static str {
    GENERALITY_MEASURE_NAMES[measure as usize]
}

fn generality_measure_from_index(index: usize) -> Option<GeneralityMeasure> {
    match index {
        0 => Some(GeneralityMeasure::NoMeasure),
        1 => Some(GeneralityMeasure::Terms),
        2 => Some(GeneralityMeasure::Literals),
        3 => Some(GeneralityMeasure::Formulas),
        4 => Some(GeneralityMeasure::PositiveFormula),
        5 => Some(GeneralityMeasure::PositiveLiteral),
        6 => Some(GeneralityMeasure::PositiveTerms),
        7 => Some(GeneralityMeasure::NegativeFormula),
        8 => Some(GeneralityMeasure::NegativeLiteral),
        9 => Some(GeneralityMeasure::NegativeTerms),
        _ => None,
    }
}

fn parse_g_sine(scanner: &mut Scanner) -> Result<AxFilter, Diagnostic> {
    let mut result = AxFilter::new();
    scanner.accept_id("GSinE")?;
    result.type_ = AxFilterType::GSinE;
    scanner.accept_tok(TokenType::OPEN_BRACKET)?;

    result.gen_measure = get_gen_measure(&scanner.current_token().literal());
    if result.gen_measure == GeneralityMeasure::NoMeasure {
        return Err(current_error(scanner, "Unknown generality measure"));
    }
    if !matches!(
        result.gen_measure,
        GeneralityMeasure::Terms | GeneralityMeasure::Formulas
    ) {
        return Err(current_error(
            scanner,
            "Generality measure not yet implemented",
        ));
    }
    scanner.next_token()?;
    scanner.accept_tok(TokenType::COMMA)?;

    if !scanner.test_tok(TokenType::COMMA) {
        scanner.check_id("hypos|nohypos")?;
        result.use_hypotheses = scanner.test_id("hypos");
        scanner.next_token()?;
    }
    scanner.accept_tok(TokenType::COMMA)?;

    if !scanner.test_tok(TokenType::COMMA) && scanner.test_id("true|false") {
        result.defined_symbols_in_drel = scanner.test_id("true");
        scanner.accept_id("true|false")?;
        scanner.accept_tok(TokenType::COMMA)?;
    }
    if !scanner.test_tok(TokenType::COMMA) {
        result.benevolence = parse_float(scanner)?;
    }
    scanner.accept_tok(TokenType::COMMA)?;

    if !scanner.test_tok(TokenType::COMMA) {
        result.generosity = parse_positive_i64(scanner)?;
    }
    scanner.accept_tok(TokenType::COMMA)?;
    if !scanner.test_tok(TokenType::COMMA) {
        result.max_recursion_depth = parse_positive_i64(scanner)?;
    }
    scanner.accept_tok(TokenType::COMMA)?;
    if !scanner.test_tok(TokenType::COMMA) {
        result.max_set_size = parse_positive_i64(scanner)?;
    }
    scanner.accept_tok(TokenType::COMMA)?;
    if !scanner.test_tok(TokenType::CLOSE_BRACKET | TokenType::COMMA) {
        result.max_set_fraction = parse_float(scanner)?;
    }
    if scanner.test_tok(TokenType::COMMA) && test_look_id(scanner, 1, "addnosymb|ignorenosymb") {
        scanner.accept_tok(TokenType::COMMA)?;
        result.add_no_symbol_axioms = scanner.test_id("addnosymb");
        scanner.accept_id("addnosymb|ignorenosymb")?;
    }
    if scanner.test_tok(TokenType::COMMA) && test_look_id(scanner, 1, "true|false") {
        scanner.accept_tok(TokenType::COMMA)?;
        result.trim_implications = scanner.test_id("true");
        scanner.accept_id("true|false")?;
    }
    scanner.accept_tok(TokenType::CLOSE_BRACKET)?;
    Ok(result)
}

fn parse_threshold(scanner: &mut Scanner) -> Result<AxFilter, Diagnostic> {
    let mut result = AxFilter::new();
    scanner.accept_id("Threshold")?;
    result.type_ = AxFilterType::Threshold;
    scanner.accept_tok(TokenType::OPEN_BRACKET)?;
    result.threshold = parse_positive_i64(scanner)?;
    scanner.accept_tok(TokenType::CLOSE_BRACKET)?;
    Ok(result)
}

fn parse_lambda_def(scanner: &mut Scanner) -> Result<AxFilter, Diagnostic> {
    scanner.accept_id("LambdaDef")?;
    Ok(AxFilter::lambda_defines())
}

fn parse_positive_i64(scanner: &mut Scanner) -> Result<i64, Diagnostic> {
    scanner.check_tok(TokenType::POS_INT)?;
    let value = scanner.current_token().numval();
    let Ok(value) = i64::try_from(value) else {
        return Err(current_error(scanner, "Long integer overflow"));
    };
    scanner.next_token()?;
    Ok(value)
}

fn test_look_id(scanner: &Scanner, look: usize, ids: &str) -> bool {
    crate::inout::scanner::test_id(scanner.look_token(look), ids)
}

fn current_error(scanner: &Scanner, message: &str) -> Diagnostic {
    Diagnostic::new(
        ErrorCode::SYNTAX_ERROR,
        format!(
            "{}(just read '{}'): {message}",
            token_pos_rep(scanner.current_token()),
            scanner.current_token().literal()
        ),
    )
}

#[cfg(test)]
mod tests {
    use super::{
        generality_measure_name, get_gen_measure, sine_get_filter, AxFilter, AxFilterSet,
        AxFilterType, AX_FILTER_DEFAULT_SET, GENERALITY_MEASURE_NAMES,
    };
    use crate::basics::error::ErrorCode;
    use crate::clauses::f_generality::GeneralityMeasure;
    use crate::inout::scanner::Scanner;

    #[test]
    fn ax_filter_type_discriminants_match_c_enum() {
        assert_eq!(AxFilterType::NoFilter as i32, 0);
        assert_eq!(AxFilterType::GSinE as i32, 1);
        assert_eq!(AxFilterType::Threshold as i32, 2);
        assert_eq!(AxFilterType::LambdaDefines as i32, 3);
    }

    #[test]
    fn default_ax_filter_matches_c_allocation_defaults() {
        let filter = AxFilter::new();

        assert_eq!(filter.name, None);
        assert_eq!(filter.type_, AxFilterType::NoFilter);
        assert_eq!(filter.gen_measure, GeneralityMeasure::NoMeasure);
        assert!(!filter.use_hypotheses);
        assert!((filter.benevolence - 1.0).abs() < f64::EPSILON);
        assert_eq!(filter.generosity, i64::MAX);
        assert_eq!(filter.max_recursion_depth, i64::from(i32::MAX));
        assert_eq!(filter.max_set_size, i64::MAX);
        assert!((filter.max_set_fraction - 1.0).abs() < f64::EPSILON);
        assert!(!filter.add_no_symbol_axioms);
        assert!(!filter.trim_implications);
        assert!(!filter.defined_symbols_in_drel);
        assert_eq!(filter.threshold, 0);
    }

    #[test]
    fn generality_measure_lookup_preserves_c_names_and_typos() {
        assert_eq!(GENERALITY_MEASURE_NAMES[4], "CoutPosFormulas");
        assert_eq!(GENERALITY_MEASURE_NAMES[7], "CoutNegFormulas");
        assert_eq!(get_gen_measure("CountTerms"), GeneralityMeasure::Terms);
        assert_eq!(
            get_gen_measure("CoutNegFormulas"),
            GeneralityMeasure::NegativeFormula
        );
        assert_eq!(
            get_gen_measure("CountPosFormulas"),
            GeneralityMeasure::NoMeasure
        );
        assert_eq!(get_gen_measure("missing"), GeneralityMeasure::NoMeasure);
        assert_eq!(
            generality_measure_name(GeneralityMeasure::NegativeLiteral),
            "CountNegLiterals"
        );
    }

    #[test]
    fn ax_filter_print_strings_match_c_shapes() {
        let threshold = AxFilter::threshold(10_000);
        assert_eq!(threshold.print_string(), "Threshold(10000)");

        let lambda = AxFilter::lambda_defines();
        assert_eq!(lambda.print_string(), "LambdaDef");

        let mut sine = AxFilter::g_sine(GeneralityMeasure::Terms);
        sine.name = Some("named".to_owned());
        sine.use_hypotheses = true;
        sine.defined_symbols_in_drel = true;
        sine.benevolence = 1.5;
        sine.generosity = 2;
        sine.max_recursion_depth = 3;
        sine.max_set_size = 4;
        sine.max_set_fraction = 0.25;
        sine.add_no_symbol_axioms = true;
        sine.trim_implications = true;

        assert_eq!(
            sine.print_string(),
            "GSinE(CountTerms, hypos, true, 1.500000, 2, 3, 4, 0.250000, addnosymb, true)"
        );
        assert_eq!(
            sine.def_print_string(),
            "named = GSinE(CountTerms, hypos, true, 1.500000, 2, 3, 4, 0.250000, addnosymb, true)"
        );
    }

    #[test]
    fn ax_filter_set_preserves_stack_order_and_name_lookup() {
        let mut threshold = AxFilter::threshold(10);
        threshold.name = Some("small".to_owned());
        let mut lambda = AxFilter::lambda_defines();
        lambda.name = Some("defs".to_owned());
        let mut set = AxFilterSet::new();

        set.add_filter(threshold);
        set.add_filter(lambda);

        assert_eq!(set.elements(), 2);
        assert_eq!(set.get_filter(0).unwrap().name.as_deref(), Some("small"));
        assert_eq!(
            set.find_filter("defs").unwrap().type_,
            AxFilterType::LambdaDefines
        );
        assert_eq!(set.names_string(), "small, defs");
        assert_eq!(
            set.print_string(),
            "small = Threshold(10)\ndefs = LambdaDef\n"
        );
    }

    #[test]
    fn ax_filter_parser_reads_threshold_lambda_and_sparse_gsine_definitions() {
        let mut scanner = Scanner::from_internal_string(
            "small=Threshold(10) LambdaDef GSinE(CountFormulas, ,false, 5.0,, 4,20000,1.0)",
            true,
        )
        .unwrap();
        let mut set = AxFilterSet::new();

        assert_eq!(set.parse(&mut scanner).unwrap(), 3);

        assert_eq!(set.get_filter(0).unwrap().name.as_deref(), Some("small"));
        assert_eq!(set.get_filter(0).unwrap().threshold, 10);
        let anonymous = set.get_filter(1).unwrap().name.as_deref().unwrap();
        assert!(anonymous.starts_with("axfilter_auto"));
        assert_eq!(
            set.get_filter(1).unwrap().type_,
            AxFilterType::LambdaDefines
        );
        let sine = set.get_filter(2).unwrap();
        assert_eq!(sine.type_, AxFilterType::GSinE);
        assert_eq!(sine.gen_measure, GeneralityMeasure::Formulas);
        assert!(!sine.use_hypotheses);
        assert!(!sine.defined_symbols_in_drel);
        assert!((sine.benevolence - 5.0).abs() < f64::EPSILON);
        assert_eq!(sine.generosity, i64::MAX);
        assert_eq!(sine.max_recursion_depth, 4);
        assert_eq!(sine.max_set_size, 20_000);
        assert!((sine.max_set_fraction - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn ax_filter_parser_reads_full_gsine_optional_tail() {
        let mut scanner = Scanner::from_internal_string(
            "named=GSinE(CountTerms, hypos,true, 1.25,7,8,9,0.5,addnosymb,false)",
            true,
        )
        .unwrap();

        let parsed = AxFilter::parse_definition(&mut scanner).unwrap();

        assert_eq!(parsed.name.as_deref(), Some("named"));
        assert_eq!(parsed.type_, AxFilterType::GSinE);
        assert_eq!(parsed.gen_measure, GeneralityMeasure::Terms);
        assert!(parsed.use_hypotheses);
        assert!(parsed.defined_symbols_in_drel);
        assert!((parsed.benevolence - 1.25).abs() < f64::EPSILON);
        assert_eq!(parsed.generosity, 7);
        assert_eq!(parsed.max_recursion_depth, 8);
        assert_eq!(parsed.max_set_size, 9);
        assert!((parsed.max_set_fraction - 0.5).abs() < f64::EPSILON);
        assert!(parsed.add_no_symbol_axioms);
        assert!(!parsed.trim_implications);
    }

    #[test]
    fn ax_filter_parser_preserves_unimplemented_generality_measure_diagnostic() {
        let mut scanner =
            Scanner::from_internal_string("GSinE(CountLiterals, ,false, 1.0,,,,)", true).unwrap();

        let error = AxFilter::parse(&mut scanner).unwrap_err();

        assert_eq!(error.code(), crate::basics::error::ErrorCode::SYNTAX_ERROR);
        assert!(
            error
                .message()
                .contains("Generality measure not yet implemented"),
            "{error}"
        );
    }

    #[test]
    fn default_ax_filter_set_parses_c_builtin_definitions() {
        let set = AxFilterSet::default_set().unwrap();

        assert_eq!(set.elements(), 21);
        assert_eq!(
            set.find_filter("threshold010000").unwrap().type_,
            AxFilterType::Threshold
        );
        assert_eq!(
            set.find_filter("LambdaDef").unwrap().type_,
            AxFilterType::LambdaDefines
        );
        assert!(
            set.find_filter("gf600_gu_R05_F100_L20000add")
                .unwrap()
                .add_no_symbol_axioms
        );
        assert!(AX_FILTER_DEFAULT_SET.contains("gf500_h_gu_R04_F100_L20000"));
    }

    #[test]
    fn sine_get_filter_resolves_default_names_and_direct_definitions() {
        let named = sine_get_filter("threshold010000").unwrap();
        assert_eq!(named.filter().type_, AxFilterType::Threshold);
        assert_eq!(named.filter().threshold, 10_000);
        assert_eq!(named.filters().elements(), 21);

        let direct = sine_get_filter("Threshold(7)").unwrap();
        assert_eq!(direct.filter().type_, AxFilterType::Threshold);
        assert_eq!(direct.filter().threshold, 7);
        assert_eq!(direct.filters().elements(), 22);
        assert!(direct
            .filter()
            .name
            .as_deref()
            .is_some_and(|name| { name.starts_with("axfilter_auto") }));
    }

    #[test]
    fn sine_get_filter_unknown_name_lists_default_choices() {
        let error = sine_get_filter("missing_sine_filter").unwrap_err();

        assert_eq!(error.code(), ErrorCode::USAGE_ERROR);
        assert!(error
            .message()
            .contains("Unknown SinE-filter 'missing_sine_filter' selected"));
        assert!(error.message().contains("threshold010000, LambdaDef"));
        assert!(error.message().contains("gf600_gu_R05_F100_L20000add"));
    }

    #[test]
    fn sine_get_filter_preserves_normal_build_named_definition_rejection() {
        let error = sine_get_filter("custom=Threshold(7)").unwrap_err();

        assert_eq!(error.code(), ErrorCode::USAGE_ERROR);
        assert!(error
            .message()
            .contains("Unknown SinE-filter 'custom=Threshold(7)' selected"));
    }

    #[test]
    fn print_buf_string_uses_c_strict_fit_result() {
        let filter = AxFilter::threshold(10);
        let rendered = filter.print_string();

        assert_eq!(filter.print_buf_string(rendered.len()), None);
        assert_eq!(filter.print_buf_string(rendered.len() + 1), Some(rendered));
    }
}
