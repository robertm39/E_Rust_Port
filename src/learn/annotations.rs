use crate::basics::ddarrays::{DDArray, DDArrayIndex};
use crate::basics::error::{Diagnostic, ErrorCode};
use crate::basics::numtrees::{NumTreeEntry, NumTreeKey};
use crate::inout::basicparser::parse_float;
use crate::inout::scanner::{token_pos_rep, Scanner, TokenType};
use std::collections::btree_map::{Entry, Iter};
use std::collections::BTreeMap;
use std::fmt::Write as _;

pub const ANNOTATION_DEFAULT_SIZE: usize = 7;

#[derive(Clone, Debug, PartialEq)]
pub struct Annotation {
    key: i64,
    values: DDArray,
    length: i64,
}

#[derive(Clone, Debug, PartialEq)]
enum AnnotationTreeRepr {
    Empty,
    One {
        key: NumTreeKey,
        entry: NumTreeEntry<Annotation, ()>,
    },
    Many {
        entries: BTreeMap<NumTreeKey, NumTreeEntry<Annotation, ()>>,
        root_key: NumTreeKey,
    },
}

/// Numeric annotation tree with inline storage for the common singleton case.
///
/// Learning corpora normally attach one proof annotation to each term. Keeping
/// that entry inline avoids allocating a full standard-library B-tree leaf for
/// every parsed term while preserving the C tree's key and recent-root
/// behavior when multiple annotations are present.
#[derive(Clone, Debug, PartialEq)]
pub struct AnnotationTree {
    repr: AnnotationTreeRepr,
}

impl Default for AnnotationTree {
    fn default() -> Self {
        Self::new()
    }
}

impl AnnotationTree {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            repr: AnnotationTreeRepr::Empty,
        }
    }

    #[must_use]
    pub fn nodes(&self) -> usize {
        match &self.repr {
            AnnotationTreeRepr::Empty => 0,
            AnnotationTreeRepr::One { .. } => 1,
            AnnotationTreeRepr::Many { entries, .. } => entries.len(),
        }
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        matches!(self.repr, AnnotationTreeRepr::Empty)
    }

    #[must_use]
    pub const fn root_key(&self) -> Option<NumTreeKey> {
        match &self.repr {
            AnnotationTreeRepr::Empty => None,
            AnnotationTreeRepr::One { key, .. } => Some(*key),
            AnnotationTreeRepr::Many { root_key, .. } => Some(*root_key),
        }
    }

    pub fn store(&mut self, key: NumTreeKey, val1: Annotation, val2: ()) -> bool {
        match &mut self.repr {
            AnnotationTreeRepr::Empty => {
                self.repr = AnnotationTreeRepr::One {
                    key,
                    entry: NumTreeEntry { val1, val2 },
                };
                true
            }
            AnnotationTreeRepr::One {
                key: stored_key, ..
            } if *stored_key == key => false,
            AnnotationTreeRepr::One { .. } => {
                let old = std::mem::replace(&mut self.repr, AnnotationTreeRepr::Empty);
                let AnnotationTreeRepr::One {
                    key: old_key,
                    entry: old_entry,
                } = old
                else {
                    unreachable!("matched singleton annotation tree")
                };
                let mut entries = BTreeMap::new();
                entries.insert(old_key, old_entry);
                entries.insert(key, NumTreeEntry { val1, val2 });
                self.repr = AnnotationTreeRepr::Many {
                    entries,
                    root_key: key,
                };
                true
            }
            AnnotationTreeRepr::Many { entries, root_key } => {
                *root_key = key;
                match entries.entry(key) {
                    Entry::Vacant(entry) => {
                        entry.insert(NumTreeEntry { val1, val2 });
                        true
                    }
                    Entry::Occupied(_) => false,
                }
            }
        }
    }

    #[must_use]
    pub fn find(&self, key: NumTreeKey) -> Option<&NumTreeEntry<Annotation, ()>> {
        match &self.repr {
            AnnotationTreeRepr::Empty => None,
            AnnotationTreeRepr::One {
                key: stored_key,
                entry,
            } => (*stored_key == key).then_some(entry),
            AnnotationTreeRepr::Many { entries, .. } => entries.get(&key),
        }
    }

    pub fn find_mut(&mut self, key: NumTreeKey) -> Option<&mut NumTreeEntry<Annotation, ()>> {
        match &mut self.repr {
            AnnotationTreeRepr::Empty => None,
            AnnotationTreeRepr::One {
                key: stored_key,
                entry,
            } => (*stored_key == key).then_some(entry),
            AnnotationTreeRepr::Many { entries, root_key } => {
                let found = entries.get_mut(&key);
                if found.is_some() {
                    *root_key = key;
                }
                found
            }
        }
    }

    pub fn extract_root(&mut self) -> Option<(NumTreeKey, NumTreeEntry<Annotation, ()>)> {
        let key = self.root_key()?;
        self.extract_entry(key)
    }

    pub fn delete_entry(&mut self, key: NumTreeKey) -> bool {
        self.extract_entry(key).is_some()
    }

    pub fn iter(&self) -> impl Iterator<Item = (NumTreeKey, &NumTreeEntry<Annotation, ()>)> {
        AnnotationTreeIter::new(&self.repr)
    }

    fn extract_entry(
        &mut self,
        key: NumTreeKey,
    ) -> Option<(NumTreeKey, NumTreeEntry<Annotation, ()>)> {
        match &mut self.repr {
            AnnotationTreeRepr::Empty => None,
            AnnotationTreeRepr::One {
                key: stored_key, ..
            } if *stored_key != key => None,
            AnnotationTreeRepr::One { .. } => {
                let old = std::mem::replace(&mut self.repr, AnnotationTreeRepr::Empty);
                let AnnotationTreeRepr::One { key, entry } = old else {
                    unreachable!("matched singleton annotation tree")
                };
                Some((key, entry))
            }
            AnnotationTreeRepr::Many { entries, root_key } => {
                let result = entries.remove_entry(&key);
                if result.is_some() {
                    match entries.len() {
                        0 => self.repr = AnnotationTreeRepr::Empty,
                        1 => {
                            let (key, entry) = entries
                                .pop_first()
                                .expect("one annotation tree entry remains");
                            self.repr = AnnotationTreeRepr::One { key, entry };
                        }
                        _ => {
                            *root_key = *entries
                                .first_key_value()
                                .expect("multi-entry annotation tree is non-empty")
                                .0;
                        }
                    }
                }
                result
            }
        }
    }
}

struct AnnotationTreeIter<'a> {
    one: Option<(NumTreeKey, &'a NumTreeEntry<Annotation, ()>)>,
    many: Option<Iter<'a, NumTreeKey, NumTreeEntry<Annotation, ()>>>,
}

impl<'a> AnnotationTreeIter<'a> {
    fn new(repr: &'a AnnotationTreeRepr) -> Self {
        match repr {
            AnnotationTreeRepr::Empty => Self {
                one: None,
                many: None,
            },
            AnnotationTreeRepr::One { key, entry } => Self {
                one: Some((*key, entry)),
                many: None,
            },
            AnnotationTreeRepr::Many { entries, .. } => Self {
                one: None,
                many: Some(entries.iter()),
            },
        }
    }
}

impl<'a> Iterator for AnnotationTreeIter<'a> {
    type Item = (NumTreeKey, &'a NumTreeEntry<Annotation, ()>);

    fn next(&mut self) -> Option<Self::Item> {
        self.one
            .take()
            .or_else(|| self.many.as_mut()?.next().map(|(key, entry)| (*key, entry)))
    }
}

impl Default for Annotation {
    fn default() -> Self {
        Self::new()
    }
}

impl Annotation {
    #[must_use]
    pub fn new() -> Self {
        Self::with_key(0)
    }

    #[must_use]
    pub fn with_key(key: i64) -> Self {
        Self {
            key,
            values: DDArray::new(ANNOTATION_DEFAULT_SIZE, ANNOTATION_DEFAULT_SIZE),
            length: 0,
        }
    }

    #[must_use]
    pub const fn key(&self) -> i64 {
        self.key
    }

    pub fn set_key(&mut self, key: i64) {
        self.key = key;
    }

    #[must_use]
    pub const fn length(&self) -> i64 {
        self.length
    }

    /// Set the parsed annotation length.
    ///
    /// # Panics
    ///
    /// Panics for negative lengths, matching the C code's assertion that
    /// annotation counts are non-negative.
    pub fn set_length(&mut self, length: i64) {
        assert!(length >= 0, "annotation length must be non-negative");
        self.length = length;
    }

    #[must_use]
    pub fn allocated_value_slots(&self) -> usize {
        self.values.size()
    }

    #[must_use]
    pub fn value(&self, index: i64) -> Option<f64> {
        self.values.existing_element(dd_index(index)?)
    }

    #[must_use]
    pub fn count(&self) -> f64 {
        self.value(0).unwrap_or(0.0)
    }

    pub fn set_count(&mut self, count: f64) {
        self.assign_value(0, count);
    }

    /// Assign an annotation value, growing the underlying dynamic double array.
    ///
    /// # Panics
    ///
    /// Panics when `index` is negative or cannot fit the dynamic-array index
    /// type. The original `DDArrayElementRef` asserts the same precondition.
    pub fn assign_value(&mut self, index: i64, value: f64) {
        self.values.assign(dd_index_or_panic(index), value);
    }

    #[must_use]
    pub fn print_string(&self) -> String {
        let mut result = String::new();
        let write_result = write!(&mut result, "{}:(", self.key);
        debug_assert!(write_result.is_ok());
        let mut sep = "";
        for index in 0..self.length {
            result.push_str(sep);
            let value = self.value(index).unwrap_or(0.0);
            let write_result = write!(&mut result, "{value:.6}");
            debug_assert!(write_result.is_ok());
            sep = ",";
        }
        result.push(')');
        result
    }

    /// Combine another annotation into this one with C's count-weighted average.
    ///
    /// # Panics
    ///
    /// Panics when this annotation is longer than `new_anno`. The C
    /// implementation asserts the same precondition, while allowing a newly
    /// allocated zero-length collector.
    pub fn combine(&mut self, new_anno: &mut Self) {
        let resw = self.value_growing(0);
        let neww = new_anno.value_growing(0);
        let length = new_anno.length;
        assert!(self.length <= length);

        for index in 1..=length {
            let resval = self.value_growing(index);
            let newval = new_anno.value_growing(index);
            self.assign_value(index, (resval * resw + newval * neww) / (resw + neww));
        }
        self.assign_value(0, resw + neww);
        self.length = length;
    }

    /// Evaluate this annotation against the supplied feature weights.
    ///
    /// # Panics
    ///
    /// Panics when `weights` has fewer entries than the annotation's non-count
    /// elements, or if the annotation length cannot fit in `usize`.
    #[must_use]
    pub fn eval(&self, weights: &[f64]) -> f64 {
        let elements = if self.length > 0 { self.length - 1 } else { 0 };
        let Ok(weight_count) = usize::try_from(elements) else {
            panic!("annotation length does not fit usize");
        };
        assert!(
            weights.len() >= weight_count,
            "annotation evaluation requires one weight per non-count element"
        );

        let mut result = 0.0;
        for (weight_index, index) in (0..elements).enumerate() {
            result += self.value(index + 1).unwrap_or(0.0) * weights[weight_index];
        }
        result
    }

    fn value_growing(&mut self, index: i64) -> f64 {
        self.values.element(dd_index_or_panic(index))
    }
}

/// Parse a single C `proof:(number[,number]*)` annotation.
///
/// # Panics
///
/// Panics when `expected` is negative, matching the assertion in
/// `AnnotationParse`.
pub fn annotation_parse(scanner: &mut Scanner, expected: i64) -> Result<Annotation, Diagnostic> {
    assert!(
        expected >= 0,
        "expected annotation length must be non-negative"
    );
    scanner.check_tok(TokenType::POS_INT)?;
    let key = annotation_key(scanner)?;
    scanner.accept_tok(TokenType::POS_INT)?;
    scanner.accept_tok(TokenType::COLON)?;
    scanner.accept_tok(TokenType::OPEN_BRACKET)?;

    let mut annotation = Annotation::with_key(key);
    let mut count = 0_i64;
    while !scanner.test_tok(TokenType::CLOSE_BRACKET) {
        if count == expected {
            return Err(annotation_parse_error(
                scanner,
                "Annotation has more elements than expected",
            ));
        }
        let value = parse_float(scanner)?;
        annotation.assign_value(count, value);
        count += 1;
        if !scanner.test_tok(TokenType::CLOSE_BRACKET) {
            scanner.accept_tok(TokenType::COMMA)?;
        }
    }
    if count < expected {
        return Err(annotation_parse_error(
            scanner,
            "Annotation has fewer elements than expected",
        ));
    }
    scanner.accept_tok(TokenType::CLOSE_BRACKET)?;
    annotation.set_length(count);

    Ok(annotation)
}

pub fn annotation_list_parse(
    scanner: &mut Scanner,
    tree: &mut AnnotationTree,
    expected: i64,
) -> Result<i64, Diagnostic> {
    let mut count = 0_i64;
    while scanner.test_tok(TokenType::POS_INT) {
        let position = token_pos_rep(scanner.current_token());
        let annotation = annotation_parse(scanner, expected)?;
        let key = annotation.key();
        if !tree.store(key, annotation, ()) {
            return Err(Diagnostic::new(
                ErrorCode::SYNTAX_ERROR,
                format!("{position} Only one annotation for each proof example allowed"),
            ));
        }
        count += 1;
        if scanner.test_tok(TokenType::COMMA) {
            scanner.next_token()?;
        }
    }
    Ok(count)
}

#[must_use]
pub fn annotation_list_print_string(tree: &AnnotationTree) -> String {
    let mut result = String::new();
    for (_key, entry) in tree.iter() {
        // C keeps an empty separator for the whole traversal.
        result.push_str(&entry.val1.print_string());
    }
    result
}

pub fn annotation_merge(
    tree: &mut AnnotationTree,
    collect: &mut Annotation,
    sources: Option<&[i64]>,
) -> i64 {
    let mut count = 0_i64;
    match sources {
        None => {
            let keys = tree.iter().map(|(key, _entry)| key).collect::<Vec<_>>();
            for key in keys {
                if let Some(entry) = tree.find_mut(key) {
                    collect.combine(&mut entry.val1);
                    count += 1;
                }
            }
        }
        Some(source_keys) => {
            for key in source_keys {
                if let Some(entry) = tree.find_mut(*key) {
                    collect.combine(&mut entry.val1);
                    count += 1;
                }
            }
        }
    }
    count
}

fn annotation_key(scanner: &Scanner) -> Result<i64, Diagnostic> {
    i64::try_from(scanner.current_token().numval())
        .map_err(|_| annotation_parse_error(scanner, "Long integer overflow"))
}

fn annotation_parse_error(scanner: &Scanner, message: &str) -> Diagnostic {
    Diagnostic::new(
        ErrorCode::SYNTAX_ERROR,
        format!("{} {message}", token_pos_rep(scanner.current_token())),
    )
}

fn dd_index(index: i64) -> Option<DDArrayIndex> {
    if index < 0 {
        return None;
    }
    DDArrayIndex::try_from(index).ok()
}

fn dd_index_or_panic(index: i64) -> DDArrayIndex {
    match dd_index(index) {
        Some(value) => value,
        None => panic!("annotation index must be non-negative and fit DDArrayIndex"),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        annotation_list_parse, annotation_list_print_string, annotation_merge, annotation_parse,
        Annotation, AnnotationTree, AnnotationTreeRepr, ANNOTATION_DEFAULT_SIZE,
    };
    use crate::basics::error::ErrorCode;
    use crate::inout::scanner::Scanner;

    fn assert_close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < f64::EPSILON,
            "expected {expected}, got {actual}"
        );
    }

    fn make_scanner(source: &str) -> Scanner {
        Scanner::from_user_string(source, false).unwrap_or_else(|err| panic!("{err}"))
    }

    #[test]
    fn annotation_alloc_starts_with_c_default_size() {
        let annotation = Annotation::new();

        assert_eq!(annotation.key(), 0);
        assert_eq!(annotation.length(), 0);
        assert_close(annotation.count(), 0.0);
        assert_eq!(annotation.allocated_value_slots(), ANNOTATION_DEFAULT_SIZE);
    }

    #[test]
    fn annotation_parse_requires_exact_expected_count() {
        let mut scanner = make_scanner("1:(2,3.5,-4) tail");
        let annotation = annotation_parse(&mut scanner, 3).unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(annotation.key(), 1);
        assert_eq!(annotation.length(), 3);
        assert_close(annotation.value(0).unwrap_or(f64::NAN), 2.0);
        assert_close(annotation.value(1).unwrap_or(f64::NAN), 3.5);
        assert_close(annotation.value(2).unwrap_or(f64::NAN), -4.0);
        assert_eq!(scanner.current_token().literal(), "tail");

        let mut too_many = make_scanner("1:(2,3)");
        let error = annotation_parse(&mut too_many, 1).unwrap_err();
        assert_eq!(error.code(), ErrorCode::SYNTAX_ERROR);
        assert!(error
            .message()
            .contains("Annotation has more elements than expected"));

        let mut too_few = make_scanner("1:(2)");
        let error = annotation_parse(&mut too_few, 2).unwrap_err();
        assert_eq!(error.code(), ErrorCode::SYNTAX_ERROR);
        assert!(error
            .message()
            .contains("Annotation has fewer elements than expected"));
    }

    #[test]
    fn annotation_list_parse_uses_sorted_tree_and_rejects_duplicates() {
        let mut scanner = make_scanner("2:(20),1:(10), tail");
        let mut tree = AnnotationTree::new();
        let parsed =
            annotation_list_parse(&mut scanner, &mut tree, 1).unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(parsed, 2);
        assert_eq!(scanner.current_token().literal(), "tail");
        assert_eq!(
            tree.iter().map(|(key, _entry)| key).collect::<Vec<_>>(),
            vec![1, 2]
        );
        assert_close(tree.find(1).unwrap().val1.value(0).unwrap(), 10.0);

        let mut duplicate = make_scanner("1:(1),1:(2)");
        let mut tree = AnnotationTree::new();
        let error = annotation_list_parse(&mut duplicate, &mut tree, 1).unwrap_err();
        assert_eq!(error.code(), ErrorCode::SYNTAX_ERROR);
        assert!(error
            .message()
            .contains("Only one annotation for each proof example allowed"));
    }

    #[test]
    fn annotation_tree_keeps_singletons_inline_and_compacts_after_deletion() {
        let mut tree = AnnotationTree::new();
        assert!(tree.store(1, Annotation::with_key(1), ()));
        assert!(matches!(tree.repr, AnnotationTreeRepr::One { key: 1, .. }));
        assert!(!tree.store(1, Annotation::with_key(1), ()));

        assert!(tree.store(2, Annotation::with_key(2), ()));
        assert!(matches!(tree.repr, AnnotationTreeRepr::Many { .. }));
        assert_eq!(tree.root_key(), Some(2));
        assert_eq!(tree.extract_root().map(|(key, _entry)| key), Some(2));

        assert!(matches!(tree.repr, AnnotationTreeRepr::One { key: 1, .. }));
        assert_eq!(
            tree.iter().map(|(key, _entry)| key).collect::<Vec<_>>(),
            vec![1]
        );
    }

    #[test]
    fn annotation_printing_preserves_c_float_and_list_separator_behavior() {
        let mut scanner = make_scanner("2:(20),1:(10)");
        let mut tree = AnnotationTree::new();
        annotation_list_parse(&mut scanner, &mut tree, 1).unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(tree.find(1).unwrap().val1.print_string(), "1:(10.000000)");
        assert_eq!(
            annotation_list_print_string(&tree),
            "1:(10.000000)2:(20.000000)"
        );
    }

    #[test]
    fn annotation_combine_preserves_weighted_average_and_one_past_loop() {
        let mut res = Annotation::with_key(0);
        res.assign_value(0, 2.0);
        res.assign_value(1, 10.0);
        res.assign_value(2, 100.0);
        res.set_length(2);

        let mut new_anno = Annotation::with_key(1);
        new_anno.assign_value(0, 3.0);
        new_anno.assign_value(1, 20.0);
        new_anno.assign_value(2, 30.0);
        new_anno.set_length(2);

        res.combine(&mut new_anno);

        assert_close(res.count(), 5.0);
        assert_eq!(res.length(), 2);
        assert_close(res.value(1).unwrap(), 16.0);
        assert_close(res.value(2).unwrap(), 58.0);
    }

    #[test]
    fn annotation_merge_can_merge_all_or_selected_sources() {
        let mut scanner = make_scanner("1:(2,10),2:(3,20),3:(4,40)");
        let mut tree = AnnotationTree::new();
        annotation_list_parse(&mut scanner, &mut tree, 2).unwrap_or_else(|err| panic!("{err}"));

        let mut selected = Annotation::new();
        assert_eq!(
            annotation_merge(&mut tree, &mut selected, Some(&[2, 99, 1])),
            2
        );
        assert_close(selected.count(), 5.0);
        assert_close(selected.value(1).unwrap(), 16.0);

        let mut all = Annotation::new();
        assert_eq!(annotation_merge(&mut tree, &mut all, None), 3);
        assert_close(all.count(), 9.0);
        assert_close(all.value(1).unwrap(), 240.0 / 9.0);
    }

    #[test]
    fn annotation_eval_skips_count_slot() {
        let mut annotation = Annotation::with_key(1);
        for (index, value) in [10.0, 1.0, 2.0, 3.0].into_iter().enumerate() {
            annotation.assign_value(i64::try_from(index).unwrap(), value);
        }
        annotation.set_length(4);

        assert_close(annotation.eval(&[5.0, 7.0, 11.0]), 52.0);
    }
}
