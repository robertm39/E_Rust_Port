use crate::basics::ddarrays::{DDArray, DDArrayIndex};
use crate::basics::numtrees::NumTree;
use crate::learn::annotations::{
    annotation_merge, Annotation, AnnotationTree, ANNOTATION_DEFAULT_SIZE,
};
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
        self.set.find(entry_no).map(|entry| &entry.val1)
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
}

fn annotation_collect_max(max_values: &mut DDArray, annotation: &Annotation) {
    let elements = annotation.length().saturating_sub(1);
    for index in 0..elements {
        let old_max = max_values.element(dd_index(index)).unwrap_or(0.0);
        let old_val = annotation.value(index + 1).unwrap_or(0.0);
        max_values.assign(dd_index(index), old_max.max(old_val));
    }
}

#[allow(clippy::float_cmp)]
fn annotation_normalize(annotation: &mut Annotation, max_values: &mut DDArray) {
    let elements = annotation.length().saturating_sub(1);
    for index in 0..elements {
        let old_max = max_values.element(dd_index(index)).unwrap_or(0.0);
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
    use super::{AnnoSet, AnnoTerm};
    use crate::learn::annotations::{Annotation, AnnotationTree};
    use crate::terms::termtypes::Term;

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
}
