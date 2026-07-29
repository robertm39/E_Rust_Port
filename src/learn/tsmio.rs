use crate::basics::dstrings::DynamicString;
use crate::basics::error::{Diagnostic, ErrorCode};
use crate::basics::pdarrays::PDArrayIndex;
use crate::basics::pstacks::PStack;
use crate::basics::verbose::{verbout_global, verbout_global_to};
use crate::clauses::clausesets::ClauseSet;
use crate::inout::scanner::Scanner;
use crate::learn::annotations::Annotation;
use crate::learn::annoterms::{
    anno_set_compute_pattern_subst, anno_set_parse, anno_set_rec_to_flat_enc, AnnoSet,
};
use crate::learn::examplerep::{example_set_parse, example_set_select_by_dist, ExampleSet};
use crate::learn::flatannoterms::{flat_anno_set_alloc, flat_anno_set_translate, FlatAnnoSet};
use crate::learn::indexfunctions::IndexType;
use crate::learn::kbdesc::{kb_file_name, KB_ANNOTATION_NO};
use crate::learn::numfeatures::{
    compute_clause_set_num_features, Features, SEL_FEATURE_WEIGHTS, SEL_FUNC_WEIGHT,
    SEL_PRED_WEIGHT,
};
use crate::learn::patterns::PatternSubst;
use crate::learn::tsm::{tsm_admin_alloc, tsm_admin_build_tsm, Tsm, TsmAdmin, TsmId, TsmType};
use crate::terms::signature::Signature;
use crate::terms::termbanks::TermBank;
use std::io::Write;
use std::path::Path;

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
    compute_clause_set_num_features(&mut target_features, target, sig);
    example_set_prepare_with_target_features(
        flatset,
        annoset,
        eval_weights,
        examples,
        target_features,
        sel_no,
        set_part,
        dist_part,
    )
}

/// Prepare an example set from an already captured target feature vector.
///
/// This is the owned Rust equivalent of retaining C's non-owning target
/// `ClauseSet_p` until a lazy TSM evaluator is first selected. Capturing the
/// small numerical feature vector avoids retaining a deep clause-set clone.
#[allow(clippy::too_many_arguments)]
#[must_use]
pub fn example_set_prepare_with_target_features(
    flatset: &mut FlatAnnoSet,
    annoset: &mut AnnoSet,
    eval_weights: &[f64],
    examples: &mut ExampleSet,
    mut target_features: Features,
    sel_no: i64,
    set_part: f64,
    dist_part: f64,
) -> f64 {
    let mut example_ids = PStack::new();

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

#[derive(Clone, Copy)]
enum TsmTargetSource<'a> {
    Clauses(&'a ClauseSet),
    FeatureSnapshot(&'a Features),
}

/// Create a flat annotated example set from a knowledge-base directory.
///
/// # Errors
///
/// Returns scanner, parser, or recursive-to-flat recoding diagnostics from the
/// `signature`, `problems`, or annotation conversion steps.
///
/// # Panics
///
/// Panics under the same in-memory preparation conditions as
/// [`example_set_prepare`].
#[allow(clippy::too_many_arguments)]
pub fn example_set_from_kb(
    annoset: &mut AnnoSet,
    flatset: &mut FlatAnnoSet,
    flat_patterns: bool,
    bank: &mut TermBank,
    eval_weights: &[f64],
    kb: &str,
    sig: &mut Signature,
    target: &ClauseSet,
    sel_no: i64,
    set_part: f64,
    dist_part: f64,
) -> Result<f64, Diagnostic> {
    let mut filename = DynamicString::new();
    let signature_name = kb_file_name(&mut filename, kb, "signature");
    let mut signature_scanner = Scanner::from_file(Path::new(&signature_name), true)?;
    sig.parse_declarations(&mut signature_scanner, true)?;

    let problems_name = kb_file_name(&mut filename, kb, "problems");
    let mut problems_scanner = Scanner::from_file(Path::new(&problems_name), true)?;
    let mut proof_examples = ExampleSet::new();
    example_set_parse(&mut problems_scanner, &mut proof_examples)?;

    if flat_patterns {
        anno_set_rec_to_flat_enc(bank, annoset)?;
    }

    Ok(example_set_prepare(
        flatset,
        annoset,
        eval_weights,
        &mut proof_examples,
        sig,
        target,
        sel_no,
        set_part,
        dist_part,
    ))
}

/// Create a TSM admin from a knowledge-base directory.
///
/// # Errors
///
/// Returns scanner, parser, term-bank allocation, recursive-to-flat recoding,
/// or TSM construction diagnostics from the KB-loading and TSM-building steps.
///
/// # Panics
///
/// Panics under the same in-memory preparation and TSM-construction invariants
/// as [`example_set_prepare`] and [`tsm_admin_build_tsm`].
#[allow(clippy::too_many_arguments)]
pub fn tsm_from_kb(
    flat_patterns: bool,
    eval_weights: &[f64],
    kb: &str,
    sig: &mut Signature,
    target: &ClauseSet,
    sel_no: i64,
    set_part: f64,
    dist_part: f64,
    index_type: IndexType,
    tsm_type: TsmType,
    index_depth: i32,
) -> Result<TsmAdmin, Diagnostic> {
    let admin = tsm_from_kb_core(
        flat_patterns,
        eval_weights,
        kb,
        sig,
        TsmTargetSource::Clauses(target),
        sel_no,
        set_part,
        dist_part,
        index_type,
        tsm_type,
        index_depth,
    )?;
    verbout_global("TSM created\n").map_err(|error| verbose_write_diagnostic(&error))?;
    Ok(admin)
}

/// Create a TSM admin from a compact snapshot of the target problem features.
///
/// The snapshot is computed while the proof-state axioms are available and
/// retained by lazy TSM evaluators in place of an owned `ClauseSet` clone.
#[allow(clippy::too_many_arguments)]
pub(crate) fn tsm_from_kb_with_target_features(
    flat_patterns: bool,
    eval_weights: &[f64],
    kb: &str,
    sig: &mut Signature,
    target_features: &Features,
    sel_no: i64,
    set_part: f64,
    dist_part: f64,
    index_type: IndexType,
    tsm_type: TsmType,
    index_depth: i32,
) -> Result<TsmAdmin, Diagnostic> {
    let admin = tsm_from_kb_core(
        flat_patterns,
        eval_weights,
        kb,
        sig,
        TsmTargetSource::FeatureSnapshot(target_features),
        sel_no,
        set_part,
        dist_part,
        index_type,
        tsm_type,
        index_depth,
    )?;
    verbout_global("TSM created\n").map_err(|error| verbose_write_diagnostic(&error))?;
    Ok(admin)
}

/// Create a TSM admin from a knowledge-base directory and write C-shaped
/// verbose diagnostics to `verbose_output`.
///
/// This is the testable equivalent of `TSMFromKB`'s final
/// `VERBOUT("TSM created\n")` call.
///
/// # Errors
///
/// Returns the same diagnostics as [`tsm_from_kb`], plus a diagnostic if the
/// verbose output writer fails.
///
/// # Panics
///
/// Panics under the same in-memory preparation and TSM-construction invariants
/// as [`example_set_prepare`] and [`tsm_admin_build_tsm`].
#[allow(clippy::too_many_arguments)]
pub fn tsm_from_kb_with_verbose_output(
    flat_patterns: bool,
    eval_weights: &[f64],
    kb: &str,
    sig: &mut Signature,
    target: &ClauseSet,
    sel_no: i64,
    set_part: f64,
    dist_part: f64,
    index_type: IndexType,
    tsm_type: TsmType,
    index_depth: i32,
    verbose_output: &mut impl Write,
) -> Result<TsmAdmin, Diagnostic> {
    let admin = tsm_from_kb_core(
        flat_patterns,
        eval_weights,
        kb,
        sig,
        TsmTargetSource::Clauses(target),
        sel_no,
        set_part,
        dist_part,
        index_type,
        tsm_type,
        index_depth,
    )?;
    verbout_global_to(verbose_output, "TSM created\n")
        .map_err(|error| verbose_write_diagnostic(&error))?;
    Ok(admin)
}

#[allow(clippy::too_many_arguments)]
fn tsm_from_kb_core(
    flat_patterns: bool,
    eval_weights: &[f64],
    kb: &str,
    sig: &mut Signature,
    target: TsmTargetSource<'_>,
    sel_no: i64,
    set_part: f64,
    dist_part: f64,
    index_type: IndexType,
    tsm_type: TsmType,
    index_depth: i32,
) -> Result<TsmAdmin, Diagnostic> {
    let mut filename = DynamicString::new();
    let mut bank = TermBank::new(sig.clone())?;

    let clausepatterns_name = kb_file_name(&mut filename, kb, "clausepatterns");
    let mut clausepatterns_scanner = Scanner::from_file(Path::new(&clausepatterns_name), true)?;
    let mut annoset = anno_set_parse(&mut clausepatterns_scanner, &mut bank, KB_ANNOTATION_NO)?;

    let signature_name = kb_file_name(&mut filename, kb, "signature");
    let mut signature_scanner = Scanner::from_file(Path::new(&signature_name), true)?;
    bank.signature_mut()
        .parse_declarations(&mut signature_scanner, true)?;

    let problems_name = kb_file_name(&mut filename, kb, "problems");
    let mut problems_scanner = Scanner::from_file(Path::new(&problems_name), true)?;
    let mut proof_examples = ExampleSet::new();
    example_set_parse(&mut problems_scanner, &mut proof_examples)?;

    let mut flatset = flat_anno_set_alloc();
    if flat_patterns {
        anno_set_rec_to_flat_enc(&mut bank, &mut annoset)?;
    }
    // Flattening interns `$orN` constructors. Publish the completed scratch
    // signature so both live evaluation and the private TSM index use the same
    // function codes.
    *sig = bank.signature().clone();
    let target_features = match target {
        TsmTargetSource::Clauses(target) => {
            let mut target_features = Features::new();
            compute_clause_set_num_features(&mut target_features, target, sig);
            target_features
        }
        TsmTargetSource::FeatureSnapshot(target_features) => target_features.clone(),
    };
    let eval_default = example_set_prepare_with_target_features(
        &mut flatset,
        &mut annoset,
        eval_weights,
        &mut proof_examples,
        target_features,
        sel_no,
        set_part,
        dist_part,
    );

    let mut subst = PatternSubst::default_subst(bank.signature());
    anno_set_compute_pattern_subst(&mut subst, &annoset);
    let mut admin = tsm_admin_alloc(sig.clone(), tsm_type)?;
    tsm_admin_build_tsm(&mut admin, &flatset, index_type, index_depth, subst)?;
    admin.set_unmapped_eval(eval_default);
    admin.set_unmapped_weight(tsm_get_highest_weight(&admin));

    Ok(admin)
}

fn verbose_write_diagnostic(error: &std::io::Error) -> Diagnostic {
    Diagnostic::new(
        ErrorCode::SYS_ERROR,
        format!("Problem writing verbose output: {error}"),
    )
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
    use super::{
        example_set_from_kb, example_set_prepare, get_default_eval, tsm_from_kb,
        tsm_from_kb_with_verbose_output, tsm_get_highest_weight, LARGE_TSM_WEIGHT,
    };
    use crate::basics::error::init_error;
    use crate::basics::verbose::set_verbose_level;
    use crate::clauses::clausesets::ClauseSet;
    use crate::inout::scanner::Scanner;
    use crate::learn::annotations::{Annotation, AnnotationTree};
    use crate::learn::annoterms::{AnnoSet, AnnoTerm};
    use crate::learn::examplerep::{ExampleRep, ExampleSet};
    use crate::learn::flatannoterms::{flat_anno_set_add_term, flat_anno_set_alloc, FlatAnnoTerm};
    use crate::learn::indexfunctions::IndexType;
    use crate::learn::kbdesc::KB_ANNOTATION_NO;
    use crate::learn::numfeatures::{Features, FEATURE_NUMBER};
    use crate::learn::patterns::{pattern_term_compute, PatternSubst};
    use crate::learn::tsm::{tsm_admin_alloc, tsm_admin_build_tsm, TsmType};
    use crate::terms::signature::Signature;
    use crate::terms::termbanks::TermBank;
    use crate::terms::termtypes::Term;
    use crate::terms::typebanks::TypeBank;
    use crate::test_support::global_state_lock;
    use std::path::{Path, PathBuf};

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

    fn zero_feature_source() -> String {
        let mut result = String::from("PA: () FA: () (0");
        for _ in 1..FEATURE_NUMBER {
            result.push_str(", 0");
        }
        result.push(')');
        result
    }

    fn temp_kb_dir(name: &str) -> PathBuf {
        std::env::temp_dir()
            .join("umlaut-tests")
            .join(format!("tsmio-{name}-{}", std::process::id()))
    }

    fn remove_dir_if_present(path: &Path) {
        let _ = std::fs::remove_dir_all(path);
    }

    fn write_tsm_kb(kb_dir: &Path) {
        std::fs::create_dir_all(kb_dir).expect("create temporary KB directory");
        std::fs::write(
            kb_dir.join("clausepatterns"),
            "left_sym : 1:(1,0,1,2,0,0,0). \
             right_sym : 2:(1,0,3,4,0,0,0). \
             stale_sym : 99:(1,0,10,20,0,0,0).",
        )
        .expect("write clausepatterns file");
        std::fs::write(
            kb_dir.join("signature"),
            "left_sym:0 right_sym:0 stale_sym:0",
        )
        .expect("write signature file");
        let features = zero_feature_source();
        std::fs::write(
            kb_dir.join("problems"),
            format!("1: \"left\" {features} 2: \"right\" {features}"),
        )
        .expect("write problems file");
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
        let left_flat = &flat
            .find_binary(left.entry_no())
            .expect("left flat term")
            .val1;
        let right_flat = &flat
            .find_binary(right.entry_no())
            .expect("right flat term")
            .val1;
        assert_close(left_flat.eval(), 19.0 / 6.0);
        assert_close(right_flat.eval(), 7.0);
        assert_close(left_flat.eval_weight(), 1.0);
        assert_close(right_flat.eval_weight(), 1.0);
    }

    #[test]
    fn example_set_from_kb_loads_signature_and_problem_examples() {
        let kb_dir = temp_kb_dir("example-set-from-kb");
        remove_dir_if_present(&kb_dir);
        std::fs::create_dir_all(&kb_dir).expect("create temporary KB directory");
        std::fs::write(kb_dir.join("signature"), "left_sym:0 right_sym:0")
            .expect("write signature file");
        let features = zero_feature_source();
        std::fs::write(
            kb_dir.join("problems"),
            format!("1: \"left\" {features} 2: \"right\" {features}"),
        )
        .expect("write problems file");

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
        let mut flat = flat_anno_set_alloc();
        let mut signature = Signature::new(TypeBank::new());
        let mut bank =
            TermBank::new(Signature::new(TypeBank::new())).expect("term bank allocation");
        let target = ClauseSet::new();
        let kb_name = kb_dir.to_string_lossy();

        let result = example_set_from_kb(
            &mut annos,
            &mut flat,
            false,
            &mut bank,
            &[0.0, 2.0, 5.0, 0.0, 0.0, 0.0],
            &kb_name,
            &mut signature,
            &target,
            2,
            1.0,
            1.0,
        )
        .expect("example set from KB");

        assert_close(result, 7.0);
        assert_ne!(signature.find_f_code("left_sym"), 0);
        assert_eq!(flat.nodes(), 2);
        assert!(annos.get(30).is_none());

        remove_dir_if_present(&kb_dir);
    }

    #[test]
    fn tsm_from_kb_loads_clausepatterns_and_builds_admin() {
        let _guard = global_state_lock();
        init_error("Unknown program");
        set_verbose_level(0);
        let kb_dir = temp_kb_dir("tsm-from-kb");
        remove_dir_if_present(&kb_dir);
        write_tsm_kb(&kb_dir);
        let mut signature = Signature::new(TypeBank::new());
        let target = ClauseSet::new();
        let kb_name = kb_dir.to_string_lossy();

        let admin = tsm_from_kb(
            false,
            &[0.0, 2.0, 5.0, 0.0, 0.0, 0.0],
            &kb_name,
            &mut signature,
            &target,
            2,
            1.0,
            1.0,
            IndexType::ARITY,
            TsmType::Flat,
            0,
        )
        .expect("TSM from KB");

        assert_eq!(admin.tsm_type(), TsmType::Flat);
        assert!(admin.root_tsm().is_some());
        assert_close(admin.unmapped_eval(), 7.0);
        assert_close(admin.unmapped_weight(), LARGE_TSM_WEIGHT);
        assert_ne!(signature.find_f_code("left_sym"), 0);

        remove_dir_if_present(&kb_dir);
    }

    #[test]
    fn tsm_from_kb_flattens_persisted_recursive_clause_patterns() {
        let _guard = global_state_lock();
        init_error("Unknown program");
        set_verbose_level(0);
        let kb_dir = temp_kb_dir("tsm-from-generated-pattern");
        remove_dir_if_present(&kb_dir);
        std::fs::create_dir_all(&kb_dir).expect("create temporary KB directory");
        std::fs::write(
            kb_dir.join("clausepatterns"),
            "\
$cnil : 1:(1,1,0,0,0,0,0).
$or(f0_1!=f0_2,$cnil) : 1:(2,1,0,0,0,0,0).
$or(f0_1=f0_2,$cnil) : 1:(2,1,0,0,0,0,0).
",
        )
        .expect("write generated clause patterns");
        std::fs::write(kb_dir.join("signature"), "").expect("write signature file");
        std::fs::write(
            kb_dir.join("problems"),
            format!("1: \"toy\" {}", zero_feature_source()),
        )
        .expect("write problems file");
        let mut signature = Signature::new(TypeBank::new());
        signature
            .insert_internal_codes()
            .expect("internal code insertion");
        let target = ClauseSet::new();
        let kb_name = kb_dir.to_string_lossy();

        let admin = tsm_from_kb(
            true,
            &[-20.0, 20.0, -2.0, -1.0, 0.0, 2.0],
            &kb_name,
            &mut signature,
            &target,
            100_000,
            1.0,
            1.0,
            IndexType::IDENTITY,
            TsmType::Flat,
            100_000,
        )
        .expect("generated recursive patterns should flatten");

        assert_eq!(admin.tsm_type(), TsmType::Flat);
        assert!(admin.root_tsm().is_some());
        let flat_code = signature.find_f_code("$or1");
        assert_ne!(flat_code, 0);
        assert_eq!(
            flat_code,
            admin.index_bank().signature().find_f_code("$or1")
        );

        remove_dir_if_present(&kb_dir);
    }

    #[test]
    fn tsm_from_kb_verbose_output_matches_c_verbout_message() {
        let _guard = global_state_lock();
        init_error("umlaut");
        set_verbose_level(1);
        let kb_dir = temp_kb_dir("tsm-from-kb-verbose");
        remove_dir_if_present(&kb_dir);
        write_tsm_kb(&kb_dir);
        let mut signature = Signature::new(TypeBank::new());
        let target = ClauseSet::new();
        let kb_name = kb_dir.to_string_lossy();
        let mut verbose_output = Vec::new();

        let admin = tsm_from_kb_with_verbose_output(
            false,
            &[0.0, 2.0, 5.0, 0.0, 0.0, 0.0],
            &kb_name,
            &mut signature,
            &target,
            2,
            1.0,
            1.0,
            IndexType::ARITY,
            TsmType::Flat,
            0,
            &mut verbose_output,
        )
        .expect("TSM from KB with verbose output");

        assert_eq!(admin.tsm_type(), TsmType::Flat);
        assert_eq!(
            String::from_utf8(verbose_output).expect("verbose output is utf8"),
            "umlaut: TSM created\n"
        );

        init_error("Unknown program");
        set_verbose_level(0);
        remove_dir_if_present(&kb_dir);
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
