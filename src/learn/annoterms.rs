use crate::basics::ddarrays::{DDArray, DDArrayIndex};
use crate::basics::defines::DEFAULT_COMCHAR_RAW;
use crate::basics::error::Diagnostic;
use crate::basics::numtrees::NumTree;
use crate::inout::scanner::{Scanner, TokenType};
use crate::learn::annotations::{
    annotation_list_parse, annotation_list_print_string, annotation_merge, Annotation,
    AnnotationTree, ANNOTATION_DEFAULT_SIZE,
};
use crate::learn::clauseenc::{flat_recode_rec_clause_rep, parse_recursive_clause_pattern};
use crate::learn::patterns::{pattern_term_compute, PatternSubst};
use crate::terms::functypes::func_symb_start_token;
use crate::terms::termbanks::TermBank;
use crate::terms::termtypes::Term;

#[derive(Clone, Debug, PartialEq)]
pub struct AnnoTerm {
    term: Term,
    annotations: AnnotationTree,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AnnoSet {
    set: NumTree<AnnoTerm, ()>,
}

impl AnnoTerm {
    #[must_use]
    pub const fn new(term: Term, annotations: AnnotationTree) -> Self {
        Self { term, annotations }
    }

    #[must_use]
    pub const fn term(&self) -> &Term {
        &self.term
    }

    #[must_use]
    pub const fn annotations(&self) -> &AnnotationTree {
        &self.annotations
    }

    #[must_use]
    pub fn single_annotation(&self) -> Option<&Annotation> {
        if self.annotations.nodes() == 1 {
            self.annotations
                .iter()
                .next()
                .map(|(_key, entry)| &entry.val1)
        } else {
            None
        }
    }

    pub fn rec_to_flat_enc(&mut self, bank: &mut TermBank) -> Result<(), Diagnostic> {
        self.term = flat_recode_rec_clause_rep(bank, &self.term)?;
        Ok(())
    }
}

impl Default for AnnoSet {
    fn default() -> Self {
        Self::new()
    }
}

impl AnnoSet {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            set: NumTree::new(),
        }
    }

    #[must_use]
    pub fn nodes(&self) -> usize {
        self.set.nodes()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.set.is_empty()
    }

    #[must_use]
    pub fn get(&self, entry_no: i64) -> Option<&AnnoTerm> {
        self.set.find_binary(entry_no).map(|entry| &entry.val1)
    }

    pub fn iter(&self) -> impl Iterator<Item = (i64, &AnnoTerm)> {
        self.set.iter().map(|(key, entry)| (key, &entry.val1))
    }

    pub fn add_term(&mut self, mut term: AnnoTerm) -> bool {
        let key = term.term.entry_no();
        if let Some(existing) = self.set.find_mut(key) {
            while let Some((annotation_key, entry)) = term.annotations.extract_root() {
                let mut incoming = entry.val1;
                if let Some(conflict) = existing.val1.annotations.find_mut(annotation_key) {
                    conflict.val1.combine(&mut incoming);
                } else {
                    existing
                        .val1
                        .annotations
                        .store(annotation_key, incoming, ());
                }
            }
            false
        } else {
            self.set.store(key, term, ());
            true
        }
    }

    pub fn remove_by_ident(&mut self, set_ident: i64) -> i64 {
        let mut count = 0_i64;
        let mut to_delete = Vec::new();
        let keys = self.set.iter().map(|(key, _entry)| key).collect::<Vec<_>>();
        for key in keys {
            let Some(entry) = self.set.find_mut(key) else {
                continue;
            };
            entry.val1.annotations.delete_entry(set_ident);
            if entry.val1.annotations.is_empty() {
                to_delete.push(key);
            }
        }

        for key in to_delete {
            if self.set.delete_entry(key) {
                count += 1;
            }
        }
        count
    }

    /// Keep only annotations whose source id appears in `set_idents`.
    ///
    /// `None` represents C's `ANNOTATIONS_MERGE_ALL` sentinel and leaves the set
    /// unchanged. Returns the number of terms deleted because no annotation
    /// remained after filtering.
    pub fn remove_except_ident_list(&mut self, set_idents: Option<&[i64]>) -> i64 {
        let Some(set_idents) = set_idents else {
            return 0;
        };

        self.remove_except_ident_list_with_bounds(set_idents, std::iter::repeat(set_idents.len()))
    }

    /// Compatibility surface for C's `AnnoSetRemoveExceptIdentList` loop bound.
    ///
    /// The C implementation accidentally uses the `NumTree` traversal stack's
    /// current stack pointer as the loop bound while indexing into the caller's
    /// `set_idents` stack. This helper makes that bound explicit for reference
    /// tests or future callers that need to reproduce the accident exactly.
    ///
    /// # Panics
    ///
    /// Panics if any traversal bound is larger than the provided id list,
    /// matching the eventual `PStackElementInt(set_idents, i)` assertion in C.
    pub fn remove_except_ident_list_with_traversal_bounds(
        &mut self,
        set_idents: Option<&[i64]>,
        traversal_bounds: &[usize],
    ) -> i64 {
        let Some(set_idents) = set_idents else {
            return 0;
        };

        assert_eq!(
            traversal_bounds.len(),
            self.set.nodes(),
            "C traversal-bound list must match AnnoSet term count"
        );
        self.remove_except_ident_list_with_bounds(set_idents, traversal_bounds.iter().copied())
    }

    fn remove_except_ident_list_with_bounds(
        &mut self,
        set_idents: &[i64],
        bounds: impl IntoIterator<Item = usize>,
    ) -> i64 {
        let mut count = 0_i64;
        let mut to_delete = Vec::new();
        let keys = self.set.iter().map(|(key, _entry)| key).collect::<Vec<_>>();
        for (key, bound) in keys.into_iter().zip(bounds) {
            assert!(
                bound <= set_idents.len(),
                "C AnnoSetRemoveExceptIdentList traversal bound exceeds id stack"
            );
            let retained = &set_idents[..bound];
            let Some(entry) = self.set.find_mut(key) else {
                continue;
            };
            let annotation_keys = entry
                .val1
                .annotations
                .iter()
                .map(|(annotation_key, _entry)| annotation_key)
                .collect::<Vec<_>>();
            for annotation_key in annotation_keys {
                if !retained.contains(&annotation_key) {
                    entry.val1.annotations.delete_entry(annotation_key);
                }
            }
            if entry.val1.annotations.is_empty() {
                to_delete.push(key);
            }
        }

        for key in to_delete {
            if self.set.delete_entry(key) {
                count += 1;
            }
        }
        count
    }

    /// Flatten annotation lists into one merged annotation per retained term.
    ///
    /// Returns C's implemented result value. Although the C comment says this
    /// is the number of remaining terms, the implementation never increments
    /// the local `count` and therefore always returns zero.
    pub fn flatten(&mut self, set_idents: Option<&[i64]>) -> i64 {
        let count = 0_i64;
        let mut to_delete = Vec::new();
        let keys = self.set.iter().map(|(key, _entry)| key).collect::<Vec<_>>();
        for key in keys {
            let Some(entry) = self.set.find_mut(key) else {
                continue;
            };
            let mut annotation = Annotation::new();
            annotation.set_key(0);
            let annos_found =
                annotation_merge(&mut entry.val1.annotations, &mut annotation, set_idents);
            if annos_found != 0 {
                let length = annotation_tree_root_length(&entry.val1.annotations).unwrap_or(0);
                annotation.set_length(length);
                let mut flattened = AnnotationTree::new();
                flattened.store(0, annotation, ());
                entry.val1.annotations = flattened;
            } else {
                to_delete.push(key);
            }
        }

        for key in to_delete {
            self.set.delete_entry(key);
        }
        count
    }

    pub fn normalize_flat_annos(&mut self) {
        let mut max_values = DDArray::new(ANNOTATION_DEFAULT_SIZE, ANNOTATION_DEFAULT_SIZE);
        let keys = self.set.iter().map(|(key, _entry)| key).collect::<Vec<_>>();
        for key in &keys {
            let Some(entry) = self.set.find_mut(*key) else {
                continue;
            };
            if let Some(annotation) = root_annotation_mut(&mut entry.val1.annotations) {
                annotation_collect_max(&mut max_values, annotation);
            }
        }

        for key in keys {
            let Some(entry) = self.set.find_mut(key) else {
                continue;
            };
            if let Some(annotation) = root_annotation_mut(&mut entry.val1.annotations) {
                annotation_normalize(annotation, &mut max_values);
            }
        }
    }

    pub fn rec_to_flat_enc(&mut self, bank: &mut TermBank) -> Result<i64, Diagnostic> {
        let keys = self.set.iter().map(|(key, _entry)| key).collect::<Vec<_>>();
        let mut result = 0_i64;
        for key in keys {
            let Some(entry) = self.set.find_mut(key) else {
                continue;
            };
            entry.val1.rec_to_flat_enc(bank)?;
            result += 1;
        }
        Ok(result)
    }

    pub fn compute_pattern_subst(&self, subst: &mut PatternSubst) -> bool {
        let mut result = false;
        for (_key, term) in self.iter() {
            result = pattern_term_compute(subst, term.term()) || result;
        }
        result
    }
}

#[must_use]
pub fn anno_set_alloc(bank: &mut TermBank) -> AnnoSet {
    bank.signature_mut().get_eqn_code(true);
    bank.signature_mut().get_eqn_code(false);
    bank.signature_mut().get_or_code();
    bank.signature_mut().get_cnil_code();
    AnnoSet::new()
}

pub fn anno_term_parse(
    scanner: &mut Scanner,
    bank: &mut TermBank,
    expected: i64,
) -> Result<AnnoTerm, Diagnostic> {
    let term = bank.parse_term_with_distinct_checks(scanner)?;
    parse_anno_term_tail(scanner, term, expected)
}

pub fn anno_term_parse_clause_pattern(
    scanner: &mut Scanner,
    bank: &mut TermBank,
    expected: i64,
) -> Result<AnnoTerm, Diagnostic> {
    let term = if scanner.test_id("$or") || scanner.test_id("$cnil") {
        parse_recursive_clause_pattern(scanner, bank)?
    } else {
        bank.parse_term_with_distinct_checks(scanner)?
    };
    parse_anno_term_tail(scanner, term, expected)
}

fn parse_anno_term_tail(
    scanner: &mut Scanner,
    term: Term,
    expected: i64,
) -> Result<AnnoTerm, Diagnostic> {
    scanner.accept_tok(TokenType::COLON)?;
    let mut annotations = AnnotationTree::new();
    annotation_list_parse(scanner, &mut annotations, expected)?;
    scanner.accept_tok(TokenType::FULLSTOP)?;
    Ok(AnnoTerm::new(term, annotations))
}

#[must_use]
pub fn anno_term_print_string(term: &AnnoTerm, bank: &TermBank, fullterms: bool) -> String {
    format!(
        "{} : {}.",
        bank.term_string(&term.term, fullterms),
        annotation_list_print_string(&term.annotations)
    )
}

pub fn anno_term_rec_to_flat_enc(
    bank: &mut TermBank,
    term: &mut AnnoTerm,
) -> Result<(), Diagnostic> {
    term.rec_to_flat_enc(bank)
}

pub fn anno_set_parse(
    scanner: &mut Scanner,
    bank: &mut TermBank,
    expected: i64,
) -> Result<AnnoSet, Diagnostic> {
    let mut set = anno_set_alloc(bank);
    while anno_term_starts(scanner, bank) {
        let term = anno_term_parse(scanner, bank, expected)?;
        set.add_term(term);
    }
    Ok(set)
}

pub fn anno_set_parse_clause_patterns(
    scanner: &mut Scanner,
    bank: &mut TermBank,
    expected: i64,
) -> Result<AnnoSet, Diagnostic> {
    let mut set = anno_set_alloc(bank);
    while anno_term_starts(scanner, bank) {
        let term = anno_term_parse_clause_pattern(scanner, bank, expected)?;
        set.add_term(term);
    }
    Ok(set)
}

#[must_use]
pub fn anno_set_print_string(set: &AnnoSet, bank: &TermBank) -> String {
    let mut result = format!("\n{DEFAULT_COMCHAR_RAW} Annotated terms:\n");
    for (_key, term) in set.iter() {
        result.push_str(&anno_term_print_string(term, bank, true));
        result.push('\n');
    }
    result
}

pub fn anno_set_rec_to_flat_enc(bank: &mut TermBank, set: &mut AnnoSet) -> Result<i64, Diagnostic> {
    set.rec_to_flat_enc(bank)
}

pub fn anno_set_remove_except_ident_list(set: &mut AnnoSet, set_idents: Option<&[i64]>) -> i64 {
    set.remove_except_ident_list(set_idents)
}

pub fn anno_set_remove_except_ident_list_with_traversal_bounds(
    set: &mut AnnoSet,
    set_idents: Option<&[i64]>,
    traversal_bounds: &[usize],
) -> i64 {
    set.remove_except_ident_list_with_traversal_bounds(set_idents, traversal_bounds)
}

pub fn anno_set_compute_pattern_subst(subst: &mut PatternSubst, set: &AnnoSet) -> bool {
    set.compute_pattern_subst(subst)
}

fn anno_term_starts(scanner: &Scanner, bank: &TermBank) -> bool {
    scanner.test_tok(func_symb_start_token() | TokenType::MULT)
        || (bank.signature().supports_lists() && scanner.test_tok(TokenType::OPEN_SQUARE))
}

fn annotation_collect_max(max_values: &mut DDArray, annotation: &Annotation) {
    let elements = annotation.length().saturating_sub(1);
    for index in 0..elements {
        let old_max = max_values.element(dd_index(index));
        let old_val = annotation.value(index + 1).unwrap_or(0.0);
        max_values.assign(dd_index(index), old_max.max(old_val));
    }
}

#[allow(clippy::float_cmp)]
fn annotation_normalize(annotation: &mut Annotation, max_values: &mut DDArray) {
    let elements = annotation.length().saturating_sub(1);
    for index in 0..elements {
        let old_max = max_values.element(dd_index(index));
        if old_max != 0.0 {
            let old_val = annotation.value(index + 1).unwrap_or(0.0);
            annotation.assign_value(index + 1, old_val / old_max);
        }
    }
}

fn annotation_tree_root_length(tree: &AnnotationTree) -> Option<i64> {
    let root_key = tree.root_key()?;
    tree.find(root_key).map(|entry| entry.val1.length())
}

fn root_annotation_mut(tree: &mut AnnotationTree) -> Option<&mut Annotation> {
    let root_key = tree.root_key()?;
    tree.find_mut(root_key).map(|entry| &mut entry.val1)
}

fn dd_index(index: i64) -> DDArrayIndex {
    match DDArrayIndex::try_from(index) {
        Ok(value) => value,
        Err(error) => panic!("annotation index must fit DDArrayIndex: {error}"),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        anno_set_alloc, anno_set_compute_pattern_subst, anno_set_parse, anno_set_print_string,
        anno_set_remove_except_ident_list, anno_set_remove_except_ident_list_with_traversal_bounds,
        anno_term_parse, anno_term_print_string, anno_term_rec_to_flat_enc, AnnoSet, AnnoTerm,
    };
    use crate::basics::error::ErrorCode;
    use crate::clauses::eqn::Eqn;
    use crate::clauses::eqn_props::PatEqnDirection;
    use crate::inout::scanner::{Scanner, TokenType};
    use crate::learn::annotations::{Annotation, AnnotationTree};
    use crate::learn::clauseenc::rec_encode_clause_list_rep;
    use crate::learn::patterns::PatternSubst;
    use crate::terms::signature::Signature;
    use crate::terms::termbanks::TermBank;
    use crate::terms::termtypes::Term;
    use crate::terms::typebanks::TypeBank;

    fn assert_close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < f64::EPSILON,
            "expected {expected}, got {actual}"
        );
    }

    fn term(entry_no: i64) -> Term {
        let term = Term::const_cell_alloc(entry_no + 10);
        term.set_entry_no(entry_no);
        term
    }

    fn annotation(key: i64, count: f64, values: &[f64]) -> Annotation {
        let mut annotation = Annotation::with_key(key);
        annotation.assign_value(0, count);
        for (index, value) in values.iter().copied().enumerate() {
            annotation.assign_value(i64::try_from(index + 1).unwrap(), value);
        }
        annotation.set_length(i64::try_from(values.len() + 1).unwrap());
        annotation
    }

    fn annotation_tree(annotations: Vec<Annotation>) -> AnnotationTree {
        let mut tree = AnnotationTree::new();
        for annotation in annotations {
            tree.store(annotation.key(), annotation, ());
        }
        tree
    }

    fn test_bank() -> TermBank {
        TermBank::new(Signature::new(TypeBank::new())).expect("term bank allocation")
    }

    fn make_scanner(source: &str) -> Scanner {
        Scanner::from_user_string(source, false).expect("scanner allocation")
    }

    fn typed_const(bank: &mut TermBank, name: &str) -> Term {
        let f_code = bank.signature_mut().insert_id(name, 0, false);
        let type_ = bank.signature().type_bank().i_type();
        bank.signature_mut()
            .declare_type(f_code, type_)
            .expect("constant type declaration");
        bank.create_const_term(f_code).expect("constant insertion")
    }

    #[test]
    fn anno_set_add_term_merges_annotations_for_duplicate_terms() {
        let shared = term(1);
        let mut set = AnnoSet::new();
        assert!(set.add_term(AnnoTerm::new(
            shared.clone(),
            annotation_tree(vec![annotation(10, 2.0, &[10.0])])
        )));
        assert!(!set.add_term(AnnoTerm::new(
            shared.clone(),
            annotation_tree(vec![
                annotation(10, 3.0, &[20.0]),
                annotation(11, 1.0, &[7.0]),
            ])
        )));

        let stored = set.get(shared.entry_no()).unwrap();
        assert_eq!(stored.annotations().nodes(), 2);
        let merged = &stored.annotations().find(10).unwrap().val1;
        assert_close(merged.count(), 5.0);
        assert_close(merged.value(1).unwrap(), 16.0);
        assert_close(
            stored
                .annotations()
                .find(11)
                .unwrap()
                .val1
                .value(1)
                .unwrap(),
            7.0,
        );
    }

    #[test]
    fn anno_set_remove_by_ident_deletes_empty_terms() {
        let keep = term(1);
        let remove = term(2);
        let mut set = AnnoSet::new();
        set.add_term(AnnoTerm::new(
            keep.clone(),
            annotation_tree(vec![
                annotation(1, 1.0, &[10.0]),
                annotation(2, 1.0, &[20.0]),
            ]),
        ));
        set.add_term(AnnoTerm::new(
            remove.clone(),
            annotation_tree(vec![annotation(1, 1.0, &[30.0])]),
        ));

        assert_eq!(set.remove_by_ident(1), 1);
        assert!(set.get(remove.entry_no()).is_none());
        let stored = set.get(keep.entry_no()).unwrap();
        assert!(stored.annotations().find(1).is_none());
        assert!(stored.annotations().find(2).is_some());
    }

    #[test]
    fn anno_set_flatten_merges_selected_sources_and_preserves_c_zero_return() {
        let keep = term(1);
        let remove = term(2);
        let mut set = AnnoSet::new();
        set.add_term(AnnoTerm::new(
            keep.clone(),
            annotation_tree(vec![
                annotation(1, 2.0, &[10.0]),
                annotation(2, 3.0, &[20.0]),
            ]),
        ));
        set.add_term(AnnoTerm::new(
            remove.clone(),
            annotation_tree(vec![annotation(3, 1.0, &[30.0])]),
        ));

        assert_eq!(set.flatten(Some(&[2])), 0);
        assert!(set.get(remove.entry_no()).is_none());
        let stored = set.get(keep.entry_no()).unwrap();
        assert_eq!(stored.annotations().nodes(), 1);
        let flat = stored.single_annotation().unwrap();
        assert_eq!(flat.key(), 0);
        assert_close(flat.count(), 3.0);
        assert_close(flat.value(1).unwrap(), 20.0);
    }

    #[test]
    fn anno_set_flatten_all_uses_weighted_annotation_merge() {
        let shared = term(1);
        let mut set = AnnoSet::new();
        set.add_term(AnnoTerm::new(
            shared.clone(),
            annotation_tree(vec![
                annotation(1, 2.0, &[10.0]),
                annotation(2, 3.0, &[20.0]),
            ]),
        ));

        assert_eq!(set.flatten(None), 0);
        let flat = set
            .get(shared.entry_no())
            .unwrap()
            .single_annotation()
            .unwrap();
        assert_close(flat.count(), 5.0);
        assert_close(flat.value(1).unwrap(), 16.0);
    }

    #[test]
    fn anno_set_normalize_flat_annos_divides_by_column_maxima() {
        let left = term(1);
        let right = term(2);
        let mut set = AnnoSet::new();
        set.add_term(AnnoTerm::new(
            left.clone(),
            annotation_tree(vec![annotation(0, 1.0, &[10.0, 6.0])]),
        ));
        set.add_term(AnnoTerm::new(
            right.clone(),
            annotation_tree(vec![annotation(0, 1.0, &[20.0, 3.0])]),
        ));

        set.normalize_flat_annos();

        let left_anno = set
            .get(left.entry_no())
            .unwrap()
            .single_annotation()
            .unwrap();
        assert_close(left_anno.count(), 1.0);
        assert_close(left_anno.value(1).unwrap(), 0.5);
        assert_close(left_anno.value(2).unwrap(), 1.0);

        let right_anno = set
            .get(right.entry_no())
            .unwrap()
            .single_annotation()
            .unwrap();
        assert_close(right_anno.count(), 1.0);
        assert_close(right_anno.value(1).unwrap(), 1.0);
        assert_close(right_anno.value(2).unwrap(), 0.5);
    }

    #[test]
    fn anno_set_alloc_eagerly_creates_clause_encoding_symbols() {
        let mut bank = test_bank();
        assert_eq!(bank.signature().eqn_code(), 0);
        assert_eq!(bank.signature().cnil_code(), 0);

        let set = anno_set_alloc(&mut bank);

        assert!(set.is_empty());
        assert_ne!(bank.signature().eqn_code(), 0);
        assert_ne!(bank.signature().neqn_code(), 0);
        assert_ne!(bank.signature().or_code(), 0);
        assert_ne!(bank.signature().cnil_code(), 0);
    }

    #[test]
    fn anno_set_compute_pattern_subst_visits_all_terms() {
        let mut bank = test_bank();
        let left = typed_const(&mut bank, "left");
        let right = typed_const(&mut bank, "right");
        let mut set = AnnoSet::new();
        set.add_term(AnnoTerm::new(
            left.clone(),
            annotation_tree(vec![annotation(1, 1.0, &[2.0])]),
        ));
        set.add_term(AnnoTerm::new(
            right.clone(),
            annotation_tree(vec![annotation(1, 1.0, &[3.0])]),
        ));
        let mut subst = PatternSubst::new(bank.signature());

        assert!(anno_set_compute_pattern_subst(&mut subst, &set));
        assert!(subst.symbol_is_bound(left.f_code()));
        assert!(subst.symbol_is_bound(right.f_code()));
        assert!(!anno_set_compute_pattern_subst(&mut subst, &set));
    }

    #[test]
    fn anno_set_remove_except_ident_list_keeps_requested_sources() {
        let left = term(10);
        let right = term(20);
        let mut set = AnnoSet::new();
        set.add_term(AnnoTerm::new(
            left.clone(),
            annotation_tree(vec![
                annotation(1, 1.0, &[10.0]),
                annotation(2, 1.0, &[20.0]),
            ]),
        ));
        set.add_term(AnnoTerm::new(
            right,
            annotation_tree(vec![annotation(3, 1.0, &[30.0])]),
        ));

        assert_eq!(anno_set_remove_except_ident_list(&mut set, Some(&[2])), 1);

        assert_eq!(set.nodes(), 1);
        let kept = set.get(left.entry_no()).expect("left term remains");
        assert!(kept.annotations().find(1).is_none());
        assert!(kept.annotations().find(2).is_some());
    }

    #[test]
    fn anno_set_remove_except_ident_list_all_sentinel_is_noop() {
        let left = term(10);
        let mut set = AnnoSet::new();
        set.add_term(AnnoTerm::new(
            left.clone(),
            annotation_tree(vec![annotation(1, 1.0, &[10.0])]),
        ));

        assert_eq!(anno_set_remove_except_ident_list(&mut set, None), 0);

        assert!(set.get(left.entry_no()).is_some());
        assert_eq!(set.nodes(), 1);
    }

    #[test]
    fn anno_set_remove_except_ident_list_c_bound_uses_traversal_stack_depth() {
        let first = term(10);
        let second = term(20);
        let mut set = AnnoSet::new();
        set.add_term(AnnoTerm::new(
            first.clone(),
            annotation_tree(vec![
                annotation(1, 1.0, &[10.0]),
                annotation(2, 1.0, &[20.0]),
            ]),
        ));
        set.add_term(AnnoTerm::new(
            second.clone(),
            annotation_tree(vec![annotation(2, 1.0, &[30.0])]),
        ));

        assert_eq!(
            anno_set_remove_except_ident_list_with_traversal_bounds(
                &mut set,
                Some(&[1, 2]),
                &[1, 2],
            ),
            0
        );

        let first_kept = set.get(first.entry_no()).expect("first term remains");
        assert!(first_kept.annotations().find(1).is_some());
        assert!(first_kept.annotations().find(2).is_none());
        let second_kept = set.get(second.entry_no()).expect("second term remains");
        assert!(second_kept.annotations().find(2).is_some());
    }

    #[test]
    fn anno_set_remove_except_ident_list_c_bound_all_sentinel_is_noop() {
        let left = term(10);
        let mut set = AnnoSet::new();
        set.add_term(AnnoTerm::new(
            left.clone(),
            annotation_tree(vec![annotation(1, 1.0, &[10.0])]),
        ));

        assert_eq!(
            anno_set_remove_except_ident_list_with_traversal_bounds(&mut set, None, &[]),
            0
        );

        assert!(set.get(left.entry_no()).is_some());
    }

    #[test]
    #[should_panic(expected = "C AnnoSetRemoveExceptIdentList traversal bound exceeds id stack")]
    fn anno_set_remove_except_ident_list_c_bound_panics_like_pstack_index_assertion() {
        let left = term(10);
        let mut set = AnnoSet::new();
        set.add_term(AnnoTerm::new(
            left,
            annotation_tree(vec![annotation(1, 1.0, &[10.0])]),
        ));

        let _deleted =
            anno_set_remove_except_ident_list_with_traversal_bounds(&mut set, Some(&[1]), &[2]);
    }

    #[test]
    fn anno_term_parse_and_print_preserve_c_shape() {
        let mut bank = test_bank();
        let mut scanner = make_scanner("f(a) : 2:(1,3.5),1:(2,4). tail");

        let parsed = anno_term_parse(&mut scanner, &mut bank, 2).unwrap();

        assert_eq!(bank.term_string(parsed.term(), true), "f(a)");
        assert_eq!(parsed.annotations().nodes(), 2);
        assert_eq!(scanner.current_token().literal(), "tail");
        assert_eq!(
            anno_term_print_string(&parsed, &bank, true),
            "f(a) : 1:(2.000000,4.000000)2:(1.000000,3.500000)."
        );
    }

    #[test]
    fn anno_term_parse_uses_checked_tbtermparse_shape() {
        let mut bank = test_bank();
        let mut scanner = make_scanner("12(a) : 1:(1,2).");

        let error = anno_term_parse(&mut scanner, &mut bank, 1).unwrap_err();

        assert_eq!(error.code(), ErrorCode::SYNTAX_ERROR);
        assert!(error.message().contains("Number cannot have argument list"));
    }

    #[test]
    fn anno_set_parse_accepts_list_literals_when_signature_supports_lists() {
        let mut bank =
            TermBank::new(Signature::new_with_list_support(TypeBank::new(), true)).unwrap();
        let mut scanner = make_scanner("[a,b] : 1:(2,10). f(a) : 1:(1,5).");

        let set = anno_set_parse(&mut scanner, &mut bank, 2).unwrap();

        assert_eq!(set.nodes(), 2);
        assert_eq!(scanner.current_token().kind(), TokenType::NO_TOKEN);
        let rendered = set
            .iter()
            .map(|(_key, term)| bank.term_string(term.term(), true))
            .collect::<Vec<_>>();
        assert_eq!(rendered, vec!["[a,b]", "f(a)"]);
    }

    #[test]
    fn anno_set_parse_stops_before_list_literal_without_list_support() {
        let mut bank = test_bank();
        let mut scanner = make_scanner("[a,b] : 1:(2,10).");

        let set = anno_set_parse(&mut scanner, &mut bank, 1).unwrap();

        assert!(set.is_empty());
        assert_eq!(scanner.current_token().kind(), TokenType::OPEN_SQUARE);
    }

    #[test]
    fn anno_set_parse_merges_duplicate_terms_and_prints_sorted_entries() {
        let mut bank = test_bank();
        let mut scanner = make_scanner("f(a) : 1:(2,10). f(a) : 1:(3,20). g(a) : 2:(1,30).");

        let set = anno_set_parse(&mut scanner, &mut bank, 2).unwrap();

        assert_eq!(set.nodes(), 2);
        assert_eq!(scanner.current_token().kind(), TokenType::NO_TOKEN);
        let f_entry = set
            .iter()
            .find(|(_key, term)| bank.term_string(term.term(), true) == "f(a)")
            .map(|(_key, term)| term)
            .expect("merged f(a) entry");
        let annotation = &f_entry.annotations().find(1).unwrap().val1;
        assert_close(annotation.count(), 5.0);
        assert_close(annotation.value(1).unwrap(), 16.0);
        assert_eq!(
            anno_set_print_string(&set, &bank),
            "\n% Annotated terms:\nf(a) : 1:(5.000000,16.000000).\ng(a) : 2:(1.000000,30.000000).\n"
        );
    }

    #[test]
    fn anno_term_rec_to_flat_enc_rewrites_recursive_clause_terms() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let literal = Eqn::alloc(a, b, &mut bank, true).unwrap();
        let rec =
            rec_encode_clause_list_rep(&mut bank, &[(&literal, PatEqnDirection::Normal)]).unwrap();
        let mut annotated = AnnoTerm::new(rec, annotation_tree(vec![annotation(1, 1.0, &[2.0])]));

        anno_term_rec_to_flat_enc(&mut bank, &mut annotated).unwrap();

        assert_eq!(
            annotated.term().f_code(),
            bank.signature_mut().get_or_n_code(1)
        );
    }
}
