use crate::clauses::f_generality::GeneralityMeasure;

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

#[cfg(test)]
mod tests {
    use super::{
        generality_measure_name, get_gen_measure, AxFilter, AxFilterSet, AxFilterType,
        GENERALITY_MEASURE_NAMES,
    };
    use crate::clauses::f_generality::GeneralityMeasure;

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
}
