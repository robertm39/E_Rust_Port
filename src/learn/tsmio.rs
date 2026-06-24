use crate::basics::pdarrays::PDArrayIndex;
use crate::basics::pstacks::PStack;
use crate::clauses::clausesets::ClauseSet;
use crate::learn::annotations::Annotation;
use crate::learn::annoterms::AnnoSet;
use crate::learn::examplerep::{example_set_select_by_dist, ExampleSet};
use crate::learn::flatannoterms::{flat_anno_set_translate, FlatAnnoSet};
use crate::learn::kbdesc::KB_ANNOTATION_NO;
use crate::learn::numfeatures::{
    compute_clause_set_num_features, Features, SEL_FEATURE_WEIGHTS, SEL_FUNC_WEIGHT,
    SEL_PRED_WEIGHT,
};
use crate::learn::tsm::{Tsm, TsmAdmin, TsmId, TsmType};
use crate::terms::signature::Signature;

const LARGE_TSM_WEIGHT: f64 = 1_000_000_000_000.0;
const LARGE_ADMIN_WEIGHT: f64 = 100_000_000.0;

/// Return C's default evaluation for a flattened annotation set.
///
/// # Panics
///
/// Panics if any annotated term does not have exactly one flattened
/// annotation, or if `eval_weights` is shorter than C's non-count annotation
/// slots.
#[must_use]
pub fn get_default_eval(annoset: &AnnoSet, eval_weights: &[f64]) -> f64 {
    let mut annotation = Annotation::new();
    annotation.set_length(KB_ANNOTATION_NO);
    let mut count = 0_i64;

    for (_key, term) in annoset.iter() {
        let current = term
            .single_annotation()
            .unwrap_or_else(|| panic!("default TSM eval requires flattened annotations"));
        let current_count = c_double_to_long(current.count());
        for index in 3..=KB_ANNOTATION_NO {
            let old_value = annotation.value(index).unwrap_or(0.0);
            let added = current.value(index).unwrap_or(0.0) * i64_to_f64(current_count);
            annotation.assign_value(index, old_value + added);
        }
        let old_proof_distance = annotation.value(2).unwrap_or(0.0);
        annotation.assign_value(2, old_proof_distance.max(current.value(2).unwrap_or(0.0)));
        count += current_count;
    }

    if count != 0 {
        for index in 3..=KB_ANNOTATION_NO {
            let old_value = annotation.value(index).unwrap_or(0.0);
            annotation.assign_value(index, old_value / i64_to_f64(count));
        }
    }
    annotation.assign_value(2, annotation.value(2).unwrap_or(0.0) + 1.0);

    annotation.eval(eval_weights)
}

/// Create a flat annotated example set tailored to `target`.
///
/// The returned value preserves the C implementation's `long` temporary in
/// `ExampleSetPrepare`, so fractional default evaluations are truncated before
/// being returned.
///
/// # Panics
///
/// Panics if the example-selection invariants are violated, if flattening does
/// not leave one annotation per retained term, or if `eval_weights` is too
/// short for annotation evaluation.
#[allow(clippy::too_many_arguments)]
#[must_use]
pub fn example_set_prepare(
    flatset: &mut FlatAnnoSet,
    annoset: &mut AnnoSet,
    eval_weights: &[f64],
    examples: &mut ExampleSet,
    sig: &Signature,
    target: &ClauseSet,
    sel_no: i64,
    set_part: f64,
    dist_part: f64,
) -> f64 {
    let mut target_features = Features::new();
    let mut example_ids = PStack::new();

    compute_clause_set_num_features(&mut target_features, target, sig);
    example_set_select_by_dist(
        &mut example_ids,
        examples,
        &mut target_features,
        SEL_PRED_WEIGHT,
        SEL_FUNC_WEIGHT,
        &SEL_FEATURE_WEIGHTS,
        sel_no,
        set_part,
        dist_part,
    );
    annoset.flatten(Some(example_ids.as_slice()));
    annoset.normalize_flat_annos();
    let result = get_default_eval(annoset, eval_weights);
    flat_anno_set_translate(flatset, annoset, eval_weights);

    i64_to_f64(c_double_to_long(result))
}

/// Return C's post-build TSM "highest" weight value.
///
/// This mirrors the C helper exactly, including its very large initial values
/// in the internal scans.
///
/// # Panics
///
/// Panics if `admin` does not contain the root or child TSMs expected by its
/// current TSM type, or if the type is `NoType`.
#[must_use]
pub fn tsm_get_highest_weight(admin: &TsmAdmin) -> f64 {
    let mut result = LARGE_ADMIN_WEIGHT;

    match admin.tsm_type() {
        TsmType::Flat | TsmType::Recurrent => {
            result = level_get_highest_weight(required_tsm(admin, required_root(admin)));
        }
        TsmType::Recursive => {
            result = rec_get_highest_weight(admin, required_root(admin));
        }
        TsmType::RecurrentLocal => {
            for tsm_id in admin.tsm_stack() {
                let tmp = level_get_highest_weight(required_tsm(admin, *tsm_id));
                result = result.max(tmp);
            }
        }
        TsmType::NoType => panic!("not a valid TSM type"),
    }

    result
}

fn rec_get_highest_weight(admin: &TsmAdmin, tsm_id: TsmId) -> f64 {
    let mut result = LARGE_TSM_WEIGHT;
    let tsm = required_tsm(admin, tsm_id);

    for index in 0..=tsm.max_index() {
        let Some(tsa) = tsm
            .tsas()
            .and_then(|tsas| tsas.existing_element(pd_index(index)))
            .and_then(Option::as_ref)
        else {
            continue;
        };
        result = result.max(tsa.eval_weight());
        for child_tsm in tsa.arg_tsms() {
            result = result.max(rec_get_highest_weight(admin, *child_tsm));
        }
    }

    result
}

fn level_get_highest_weight(tsm: &Tsm) -> f64 {
    let mut result = LARGE_TSM_WEIGHT;

    for index in 0..=tsm.max_index() {
        let Some(tsa) = tsm
            .tsas()
            .and_then(|tsas| tsas.existing_element(pd_index(index)))
            .and_then(Option::as_ref)
        else {
            continue;
        };
        result = result.max(tsa.eval_weight());
    }

    result
}

fn required_root(admin: &TsmAdmin) -> TsmId {
    admin
        .root_tsm()
        .unwrap_or_else(|| panic!("TSM admin has no root TSM"))
}

fn required_tsm(admin: &TsmAdmin, tsm_id: TsmId) -> &Tsm {
    admin
        .tsm(tsm_id)
        .unwrap_or_else(|| panic!("TSM id {tsm_id} is not allocated"))
}

fn pd_index(value: i64) -> PDArrayIndex {
    PDArrayIndex::try_from(value).unwrap_or(PDArrayIndex::MAX)
}

#[allow(clippy::cast_possible_truncation)]
fn c_double_to_long(value: f64) -> i64 {
    value as i64
}

#[allow(clippy::cast_precision_loss)]
fn i64_to_f64(value: i64) -> f64 {
    value as f64
}

#[cfg(test)]
mod tests {
    use super::{example_set_prepare, get_default_eval, tsm_get_highest_weight, LARGE_TSM_WEIGHT};
    use crate::clauses::clausesets::ClauseSet;
    use crate::inout::scanner::Scanner;
    use crate::learn::annotations::{Annotation, AnnotationTree};
    use crate::learn::annoterms::{AnnoSet, AnnoTerm};
    use crate::learn::examplerep::{ExampleRep, ExampleSet};
    use crate::learn::flatannoterms::{flat_anno_set_add_term, flat_anno_set_alloc, FlatAnnoTerm};
    use crate::learn::indexfunctions::IndexType;
    use crate::learn::kbdesc::KB_ANNOTATION_NO;
    use crate::learn::numfeatures::Features;
    use crate::learn::patterns::{pattern_term_compute, PatternSubst};
    use crate::learn::tsm::{tsm_admin_alloc, tsm_admin_build_tsm, TsmType};
    use crate::terms::signature::Signature;
    use crate::terms::termbanks::TermBank;
    use crate::terms::termtypes::Term;
    use crate::terms::typebanks::TypeBank;

    fn assert_close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 1.0e-12,
            "expected {expected}, got {actual}"
        );
    }

    fn term(entry_no: i64) -> Term {
        let term = Term::const_cell_alloc(entry_no + 100);
        term.set_entry_no(entry_no);
        term
    }

    fn annotation(key: i64, count: f64, slots: &[(i64, f64)]) -> Annotation {
        let mut annotation = Annotation::with_key(key);
        annotation.assign_value(0, count);
        for (index, value) in slots {
            annotation.assign_value(*index, *value);
        }
        annotation.set_length(KB_ANNOTATION_NO);
        annotation
    }

    fn annotation_tree(annotation: Annotation) -> AnnotationTree {
        let mut tree = AnnotationTree::new();
        tree.store(annotation.key(), annotation, ());
        tree
    }

    fn parse_in_bank(bank: &mut TermBank, source: &str) -> Term {
        let mut scanner = Scanner::from_user_string(source, false).expect("scanner allocation");
        bank.parse_term_simple(&mut scanner)
            .expect("simple term parse")
    }

    fn bound_subst(bank: &TermBank, terms: &[&Term]) -> PatternSubst {
        let mut subst = PatternSubst::new(bank.signature());
        for term in terms {
            pattern_term_compute(&mut subst, term);
        }
        subst
    }

    #[test]
    fn default_eval_uses_proof_distance_max_and_truncated_counts() {
        let mut annos = AnnoSet::new();
        annos.add_term(AnnoTerm::new(
            term(1),
            annotation_tree(annotation(
                0,
                2.8,
                &[
                    (2, 4.0),
                    (3, 10.0),
                    (4, 20.0),
                    (5, 30.0),
                    (6, 40.0),
                    (7, 50.0),
                ],
            )),
        ));
        annos.add_term(AnnoTerm::new(
            term(2),
            annotation_tree(annotation(
                0,
                3.2,
                &[
                    (2, 1.0),
                    (3, 20.0),
                    (4, 40.0),
                    (5, 60.0),
                    (6, 80.0),
                    (7, 100.0),
                ],
            )),
        ));

        let default_eval = get_default_eval(&annos, &[10.0, 2.0, 3.0, 5.0, 7.0, 11.0]);

        assert_close(default_eval, 1258.0);
    }

    #[test]
    fn example_set_prepare_selects_examples_and_truncates_default_eval() {
        let left = term(10);
        let right = term(20);
        let removed = term(30);
        let mut annos = AnnoSet::new();
        annos.add_term(AnnoTerm::new(
            left.clone(),
            annotation_tree(annotation(1, 1.0, &[(2, 1.0), (3, 2.0)])),
        ));
        annos.add_term(AnnoTerm::new(
            right.clone(),
            annotation_tree(annotation(2, 1.0, &[(2, 3.0), (3, 4.0)])),
        ));
        annos.add_term(AnnoTerm::new(
            removed,
            annotation_tree(annotation(99, 1.0, &[(2, 10.0), (3, 20.0)])),
        ));

        let mut examples = ExampleSet::new();
        assert!(examples.insert(ExampleRep::new(1, "left".to_owned(), Features::new())));
        assert!(examples.insert(ExampleRep::new(2, "right".to_owned(), Features::new())));
        let signature = Signature::new(TypeBank::new());
        let target = ClauseSet::new();
        let mut flat = flat_anno_set_alloc();

        let result = example_set_prepare(
            &mut flat,
            &mut annos,
            &[0.0, 2.0, 5.0, 0.0, 0.0, 0.0],
            &mut examples,
            &signature,
            &target,
            2,
            1.0,
            1.0,
        );

        assert_close(result, 7.0);
        assert_eq!(annos.nodes(), 2);
        assert!(annos.get(30).is_none());
        assert_eq!(flat.nodes(), 2);
        let left_flat = &flat.find(left.entry_no()).expect("left flat term").val1;
        let right_flat = &flat.find(right.entry_no()).expect("right flat term").val1;
        assert_close(left_flat.eval(), 19.0 / 6.0);
        assert_close(right_flat.eval(), 7.0);
        assert_close(left_flat.eval_weight(), 1.0);
        assert_close(right_flat.eval_weight(), 1.0);
    }

    #[test]
    fn highest_weight_preserves_c_large_initial_value() {
        let mut bank =
            TermBank::new(Signature::new(TypeBank::new())).expect("term bank allocation");
        let left = parse_in_bank(&mut bank, "a");
        let right = parse_in_bank(&mut bank, "b");
        let subst = bound_subst(&bank, &[&left, &right]);
        let mut set = flat_anno_set_alloc();
        flat_anno_set_add_term(&mut set, FlatAnnoTerm::new(left, 0.0, 2.0, 2));
        flat_anno_set_add_term(&mut set, FlatAnnoTerm::new(right, 10.0, 3.0, 3));
        let mut admin =
            tsm_admin_alloc(bank.signature().clone(), TsmType::Flat).expect("admin allocation");

        tsm_admin_build_tsm(&mut admin, &set, IndexType::ARITY, 0, subst).expect("flat TSM build");

        assert_close(tsm_get_highest_weight(&admin), LARGE_TSM_WEIGHT);
    }
}
