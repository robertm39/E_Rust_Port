use crate::basics::error::{Diagnostic, ErrorCode};
use crate::basics::numtrees::NumTree;
use crate::basics::pstacks::PStack;
use crate::basics::simple_stuff::{sort_weighted_objects, WeightedObject};
use crate::basics::stringtrees::StrTree;
use crate::basics::verbose::verbose_enabled;
use crate::inout::scanner::{token_pos_rep, Scanner, TokenType};
use crate::learn::numfeatures::{num_feature_distance, num_features_parse, Features};
use std::fmt::Write as _;

#[derive(Clone, Debug, PartialEq)]
pub struct ExampleRep {
    ident: i64,
    name: String,
    features: Features,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ExampleSet {
    count: i64,
    ident_index: NumTree<ExampleRep, ()>,
    name_index: StrTree<i64, ()>,
}

impl ExampleRep {
    #[must_use]
    pub const fn new(ident: i64, name: String, features: Features) -> Self {
        Self {
            ident,
            name,
            features,
        }
    }

    #[must_use]
    pub const fn ident(&self) -> i64 {
        self.ident
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub const fn features(&self) -> &Features {
        &self.features
    }

    pub fn features_mut(&mut self) -> &mut Features {
        &mut self.features
    }

    #[must_use]
    pub fn print_string(&self) -> String {
        let mut result = String::new();
        let write_result = writeln!(&mut result, "{}: \"{}\"", self.ident, self.name);
        debug_assert!(write_result.is_ok());
        result.push_str(&self.features.print_string());
        result.push('\n');
        result
    }

    pub fn parse(scanner: &mut Scanner) -> Result<Self, Diagnostic> {
        scanner.check_tok(TokenType::POS_INT)?;
        let ident = i64::try_from(scanner.current_token().numval())
            .map_err(|_| current_error(scanner, "Long integer overflow"))?;
        scanner.accept_tok(TokenType::POS_INT)?;
        scanner.accept_tok(TokenType::COLON)?;
        scanner.check_tok(TokenType::NAME)?;
        let name = if scanner.test_tok(TokenType::STRING) {
            strip_double_quote_core(scanner.current_token().literal_bytes())?
        } else {
            scanner.current_token().literal()
        };
        scanner.next_token()?;
        let features = num_features_parse(scanner)?;

        Ok(Self {
            ident,
            name,
            features,
        })
    }
}

impl Default for ExampleSet {
    fn default() -> Self {
        Self::new()
    }
}

impl ExampleSet {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            count: 0,
            ident_index: NumTree::new(),
            name_index: StrTree::new(),
        }
    }

    #[must_use]
    pub const fn count(&self) -> i64 {
        self.count
    }

    #[must_use]
    pub fn nodes(&self) -> usize {
        self.ident_index.nodes()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.ident_index.is_empty()
    }

    #[must_use]
    pub fn find_ident(&self, ident: i64) -> Option<&ExampleRep> {
        self.ident_index.find_binary(ident).map(|entry| &entry.val1)
    }

    pub fn find_ident_mut(&mut self, ident: i64) -> Option<&mut ExampleRep> {
        self.ident_index
            .find_mut(ident)
            .map(|entry| &mut entry.val1)
    }

    #[must_use]
    pub fn find_name(&self, name: &str) -> Option<&ExampleRep> {
        let ident = self.name_index.find_binary(name)?.val1;
        self.find_ident(ident)
    }

    pub fn iter(&self) -> impl Iterator<Item = (i64, &ExampleRep)> {
        self.ident_index
            .iter()
            .map(|(key, entry)| (key, &entry.val1))
    }

    pub fn insert(&mut self, rep: ExampleRep) -> bool {
        let ident = rep.ident;
        let name = rep.name.clone();
        if !self.ident_index.store(ident, rep, ()) {
            return false;
        }
        if !self.name_index.store(&name, ident, ()) {
            return false;
        }
        self.count = self.count.max(ident);
        true
    }

    /// Extract an example by the C representative pointer's identifying fields.
    ///
    /// # Panics
    ///
    /// Panics if the numeric entry exists but its name entry cannot be deleted.
    /// The C implementation asserts the same condition after deleting by name.
    pub fn extract(&mut self, rep: &ExampleRep) -> Option<ExampleRep> {
        let (_key, entry) = self.ident_index.extract_entry(rep.ident)?;
        assert!(
            self.name_index.delete_entry(&rep.name),
            "example name index must contain extracted representative"
        );
        Some(entry.val1)
    }

    pub fn delete_id(&mut self, ident: i64) -> bool {
        let Some(rep) = self.find_ident(ident).cloned() else {
            return false;
        };
        self.extract(&rep).is_some()
    }

    pub fn delete_name(&mut self, name: &str) -> bool {
        let Some(rep) = self.find_name(name).cloned() else {
            return false;
        };
        self.extract(&rep).is_some()
    }

    #[must_use]
    pub fn print_string(&self) -> String {
        let mut result = String::new();
        for (_key, rep) in self.iter() {
            result.push_str(&rep.print_string());
        }
        result
    }

    pub fn parse(scanner: &mut Scanner, set: &mut Self) -> Result<i64, Diagnostic> {
        let mut count = 0_i64;
        while scanner.test_tok(TokenType::POS_INT) {
            let position = token_pos_rep(scanner.current_token());
            let handle = ExampleRep::parse(scanner)?;
            let ident = handle.ident;
            if !set.insert(handle) {
                return Err(Diagnostic::new(
                    ErrorCode::SYNTAX_ERROR,
                    format!("{position} Entry {ident} conficts with existing entries"),
                ));
            }
            count += 1;
        }
        Ok(count)
    }

    /// Select the examples nearest to `target` using C's distance limit rules.
    ///
    /// # Panics
    ///
    /// Panics if the computed count limit exceeds the set size, matching the C
    /// assertion in `ExampleSetSelectByDist`, or if the supplied feature-weight
    /// vector is too short for `num_feature_distance`.
    #[allow(clippy::too_many_arguments)]
    pub fn select_by_dist(
        &mut self,
        results: &mut PStack<i64>,
        target: &mut Features,
        pred_w: f64,
        func_w: f64,
        weights: &[f64],
        sel_no: i64,
        set_part: f64,
        dist_part: f64,
    ) -> i64 {
        let set_size = self.ident_index.nodes();
        let mut distances = Vec::with_capacity(set_size);
        let mut average = 0.0;

        let keys = self
            .ident_index
            .iter()
            .map(|(key, _entry)| key)
            .collect::<Vec<_>>();
        for key in keys {
            let current = self
                .ident_index
                .find_mut(key)
                .expect("key collected from tree must remain present");
            let distance =
                num_feature_distance(target, &mut current.val1.features, pred_w, func_w, weights);
            distances.push(WeightedObject {
                weight: distance,
                object: key,
            });
            average += distance;
        }

        #[allow(clippy::cast_precision_loss)]
        let set_size_f64 = set_size as f64;
        average = if set_size == 0 {
            f64::NAN
        } else {
            average / set_size_f64
        };

        sort_weighted_objects(&mut distances);
        let climit = sel_no.min(c_double_to_long(set_part * set_size_f64));
        assert!(
            climit <= usize_to_i64(set_size),
            "example selection count limit must not exceed set size"
        );
        let dlimit = dist_part * average;

        let mut selected = 0_i64;
        let limit = usize::try_from(climit.max(0)).unwrap_or(0);
        for candidate in distances.iter().take(limit) {
            if candidate.weight > dlimit {
                break;
            }
            let current = self
                .ident_index
                .find(candidate.object)
                .expect("selected example key must remain present")
                .val1
                .clone();
            if verbose_enabled() {
                eprintln!("Selected problem {}: {}", current.ident, current.name);
            }
            results.push(current.ident);
            selected += 1;
        }

        selected
    }
}

#[must_use]
pub fn example_rep_print_string(rep: &ExampleRep) -> String {
    rep.print_string()
}

pub fn example_rep_parse(scanner: &mut Scanner) -> Result<ExampleRep, Diagnostic> {
    ExampleRep::parse(scanner)
}

#[must_use]
pub const fn example_set_alloc() -> ExampleSet {
    ExampleSet::new()
}

#[must_use]
pub fn example_set_find_name<'set>(set: &'set ExampleSet, name: &str) -> Option<&'set ExampleRep> {
    set.find_name(name)
}

pub fn example_set_insert(set: &mut ExampleSet, rep: ExampleRep) -> bool {
    set.insert(rep)
}

/// Extract an example by representative.
///
/// # Panics
///
/// Panics under the same name-index assertion as `ExampleSet::extract`.
pub fn example_set_extract(set: &mut ExampleSet, rep: &ExampleRep) -> Option<ExampleRep> {
    set.extract(rep)
}

pub fn example_set_delete_id(set: &mut ExampleSet, ident: i64) -> bool {
    set.delete_id(ident)
}

pub fn example_set_delete_name(set: &mut ExampleSet, name: &str) -> bool {
    set.delete_name(name)
}

#[must_use]
pub fn example_set_print_string(set: &ExampleSet) -> String {
    set.print_string()
}

pub fn example_set_parse(scanner: &mut Scanner, set: &mut ExampleSet) -> Result<i64, Diagnostic> {
    ExampleSet::parse(scanner, set)
}

/// Select examples by numerical feature distance.
///
/// # Panics
///
/// Panics under the same conditions as `ExampleSet::select_by_dist`.
#[allow(clippy::too_many_arguments)]
pub fn example_set_select_by_dist(
    results: &mut PStack<i64>,
    set: &mut ExampleSet,
    target: &mut Features,
    pred_w: f64,
    func_w: f64,
    weights: &[f64],
    sel_no: i64,
    set_part: f64,
    dist_part: f64,
) -> i64 {
    set.select_by_dist(
        results, target, pred_w, func_w, weights, sel_no, set_part, dist_part,
    )
}

fn strip_double_quote_core(bytes: &[u8]) -> Result<String, Diagnostic> {
    if bytes.len() < 2 {
        return Err(Diagnostic::new(
            ErrorCode::SYNTAX_ERROR,
            "Quoted string literal is too short",
        ));
    }
    Ok(String::from_utf8_lossy(&bytes[1..bytes.len() - 1]).into_owned())
}

fn current_error(scanner: &Scanner, message: &str) -> Diagnostic {
    Diagnostic::new(
        ErrorCode::SYNTAX_ERROR,
        format!("{} {message}", token_pos_rep(scanner.current_token())),
    )
}

fn usize_to_i64(value: usize) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

#[allow(clippy::cast_possible_truncation)]
fn c_double_to_long(value: f64) -> i64 {
    value as i64
}

#[cfg(test)]
mod tests {
    use super::{
        example_rep_parse, example_rep_print_string, example_set_delete_id, example_set_find_name,
        example_set_insert, example_set_parse, example_set_select_by_dist, ExampleRep, ExampleSet,
    };
    use crate::basics::error::ErrorCode;
    use crate::basics::pstacks::PStack;
    use crate::inout::scanner::Scanner;
    use crate::learn::numfeatures::{Features, FEATURE_NUMBER};

    fn assert_close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 1.0e-12,
            "expected {expected}, got {actual}"
        );
    }

    fn make_scanner(source: &str) -> Scanner {
        Scanner::from_user_string(source, false).unwrap_or_else(|err| panic!("{err}"))
    }

    fn feature_source(first_value: f64) -> String {
        let mut values = [0.0; FEATURE_NUMBER];
        values[0] = first_value;
        let mut result = String::from("PA: () FA: () (");
        result.push_str(&values[0].to_string());
        for value in &values[1..] {
            result.push_str(", ");
            result.push_str(&value.to_string());
        }
        result.push(')');
        result
    }

    fn features(first_value: f64) -> Features {
        let mut features = Features::new();
        features.set_value(0, first_value);
        features
    }

    fn rep(ident: i64, name: &str, first_value: f64) -> ExampleRep {
        ExampleRep::new(ident, name.to_owned(), features(first_value))
    }

    #[test]
    fn example_rep_parse_strips_double_quotes_and_prints_blank_line() {
        let source = format!("7: \"prob one\" {}", feature_source(3.5));
        let mut scanner = make_scanner(&source);
        let parsed = example_rep_parse(&mut scanner).unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(parsed.ident(), 7);
        assert_eq!(parsed.name(), "prob one");
        assert_close(parsed.features().value(0).unwrap(), 3.5);
        assert!(example_rep_print_string(&parsed).starts_with("7: \"prob one\"\nPA: ()"));

        let source = format!("8: bare_name {}", feature_source(2.0));
        let mut scanner = make_scanner(&source);
        let parsed = example_rep_parse(&mut scanner).unwrap_or_else(|err| panic!("{err}"));
        assert_eq!(parsed.name(), "bare_name");
    }

    #[test]
    fn example_set_insert_preserves_c_duplicate_name_side_effect() {
        let mut set = ExampleSet::new();

        assert!(example_set_insert(&mut set, rep(1, "same", 1.0)));
        assert!(!example_set_insert(&mut set, rep(1, "other", 2.0)));
        assert_eq!(set.nodes(), 1);
        assert_eq!(set.count(), 1);

        assert!(!example_set_insert(&mut set, rep(2, "same", 3.0)));
        assert_eq!(set.nodes(), 2);
        assert_eq!(set.count(), 1);
        assert_eq!(
            example_set_find_name(&set, "same").map(ExampleRep::ident),
            Some(1)
        );

        assert!(example_set_delete_id(&mut set, 2));
        assert!(set.find_ident(1).is_some());
        assert!(example_set_find_name(&set, "same").is_none());
    }

    #[test]
    fn example_set_parse_reads_until_non_example_and_reports_duplicates() {
        let source = format!(
            "1: first {} 2: second {} tail",
            feature_source(1.0),
            feature_source(2.0)
        );
        let mut scanner = make_scanner(&source);
        let mut set = ExampleSet::new();
        assert_eq!(
            example_set_parse(&mut scanner, &mut set).unwrap_or_else(|err| panic!("{err}")),
            2
        );
        assert_eq!(scanner.current_token().literal(), "tail");
        assert_eq!(set.print_string().matches("PA: ()").count(), 2);

        let source = format!(
            "1: dup {} 2: dup {}",
            feature_source(1.0),
            feature_source(2.0)
        );
        let mut scanner = make_scanner(&source);
        let mut set = ExampleSet::new();
        let error = example_set_parse(&mut scanner, &mut set).unwrap_err();
        assert_eq!(error.code(), ErrorCode::SYNTAX_ERROR);
        assert!(error.message().contains("conficts with existing entries"));
        assert_eq!(set.nodes(), 2);
    }

    #[test]
    fn example_set_select_by_dist_pushes_nearest_limited_examples() {
        let mut set = ExampleSet::new();
        assert!(set.insert(rep(1, "exact", 2.0)));
        assert!(set.insert(rep(2, "near", 4.0)));
        assert!(set.insert(rep(3, "far", 8.0)));
        let mut target = features(2.0);
        let mut results = PStack::new();

        assert_eq!(
            example_set_select_by_dist(
                &mut results,
                &mut set,
                &mut target,
                0.0,
                0.0,
                &[1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
                2,
                1.0,
                f64::INFINITY,
            ),
            2
        );
        assert_eq!(results.as_slice(), &[1, 2]);
    }
}
