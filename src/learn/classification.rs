use std::fmt;

use crate::learn::flatannoterms::{FlatAnnoSet, FlatAnnoTerm};
use crate::learn::patterns::PatternSubst;
use crate::learn::tsm::{tsm_eval_normalize, tsm_eval_term, TsmAdmin};
use crate::terms::termtypes::Term;

/// Classifies a term with a TSM, returning `-1.0` below the admin eval limit
/// and `1.0` otherwise.
///
/// # Panics
///
/// Panics if the admin has no root TSM or if term/index invariants are
/// violated during TSM evaluation.
pub fn tsm_term_classify(admin: &mut TsmAdmin, term: &Term, subst: &PatternSubst) -> f64 {
    let eval = tsm_eval_term(admin, term, subst);
    f64::from(tsm_eval_normalize(eval, admin.eval_limit()))
}

/// Classifies a term and writes the C progress text.
///
/// # Errors
///
/// Returns a formatting error if `output` rejects a write.
///
/// # Panics
///
/// Panics under the same internal-invariant conditions as
/// [`tsm_term_classify`].
pub fn tsm_term_classify_write(
    admin: &mut TsmAdmin,
    term: &Term,
    subst: &PatternSubst,
    output: &mut impl fmt::Write,
) -> Result<f64, fmt::Error> {
    let eval = tsm_eval_term(admin, term, subst);
    write!(output, "Evaluation: {eval:7.4} ")?;
    Ok(f64::from(tsm_eval_normalize(eval, admin.eval_limit())))
}

/// Classifies an annotated term and compares the predicted class with
/// `FlatAnnoTerm.eval`.
///
/// # Panics
///
/// Panics if the admin has no stored substitution, has no root TSM, or if
/// term/index invariants are violated during TSM evaluation.
pub fn tsm_classified_term_check(admin: &mut TsmAdmin, term: &FlatAnnoTerm) -> bool {
    let subst = admin
        .subst()
        .cloned()
        .unwrap_or_else(|| panic!("TSM admin has no pattern substitution"));
    classification_matches_eval(tsm_term_classify(admin, term.term(), &subst), term.eval())
}

/// Classifies an annotated term, compares it to `FlatAnnoTerm.eval`, and writes
/// the C progress text.
///
/// # Errors
///
/// Returns a formatting error if `output` rejects a write.
///
/// # Panics
///
/// Panics under the same internal-invariant conditions as
/// [`tsm_classified_term_check`].
pub fn tsm_classified_term_check_write(
    admin: &mut TsmAdmin,
    term: &FlatAnnoTerm,
    output: &mut impl fmt::Write,
) -> Result<bool, fmt::Error> {
    let subst = admin
        .subst()
        .cloned()
        .unwrap_or_else(|| panic!("TSM admin has no pattern substitution"));
    let result = tsm_term_classify_write(admin, term.term(), &subst, output)?;
    write!(output, " Termeval: {:7.4} ", term.eval())?;
    Ok(classification_matches_eval(result, term.eval()))
}

/// Classifies every term in a flat annotation set and returns the source count
/// of successful classifications.
///
/// # Panics
///
/// Panics if the admin has no stored substitution, has no root TSM, or if
/// term/index invariants are violated during TSM evaluation.
#[must_use]
pub fn tsm_classify_set(admin: &mut TsmAdmin, set: &FlatAnnoSet) -> i64 {
    let mut result = 0_i64;
    for (_key, entry) in set.iter() {
        let term = &entry.val1;
        if tsm_classified_term_check(admin, term) {
            result += term.sources();
        }
    }
    result
}

/// Classifies every term in a flat annotation set, writes the C progress text,
/// and returns the source count of successful classifications.
///
/// # Errors
///
/// Returns a formatting error if `output` rejects a write.
///
/// # Panics
///
/// Panics under the same internal-invariant conditions as
/// [`tsm_classify_set`].
pub fn tsm_classify_set_write(
    admin: &mut TsmAdmin,
    set: &FlatAnnoSet,
    output: &mut impl fmt::Write,
) -> Result<i64, fmt::Error> {
    let mut result = 0_i64;
    for (_key, entry) in set.iter() {
        let term = &entry.val1;
        if tsm_classified_term_check_write(admin, term, output)? {
            write!(output, "OKOK ")?;
            result += term.sources();
        } else {
            write!(output, "FAIL ")?;
        }
        writeln!(
            output,
            "{}",
            admin.index_bank().term_string(term.term(), true)
        )?;
    }
    Ok(result)
}

#[allow(clippy::float_cmp)]
fn classification_matches_eval(classification: f64, eval: f64) -> bool {
    classification == eval
}

#[cfg(test)]
mod tests {
    use super::{
        tsm_classified_term_check, tsm_classified_term_check_write, tsm_classify_set,
        tsm_classify_set_write, tsm_term_classify, tsm_term_classify_write,
    };
    use crate::inout::scanner::Scanner;
    use crate::learn::flatannoterms::{
        flat_anno_set_add_term, flat_anno_set_alloc, FlatAnnoSet, FlatAnnoTerm,
    };
    use crate::learn::indexfunctions::IndexType;
    use crate::learn::patterns::{pattern_term_compute, PatternSubst};
    use crate::learn::tsm::{tsm_admin_alloc, tsm_admin_build_tsm, TsmAdmin, TsmType};
    use crate::terms::signature::Signature;
    use crate::terms::termbanks::TermBank;
    use crate::terms::termtypes::Term;
    use crate::terms::typebanks::TypeBank;

    fn assert_close(actual: f64, expected: f64) {
        assert!((actual - expected).abs() < f64::EPSILON);
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

    fn trained_admin() -> (TsmAdmin, FlatAnnoSet, PatternSubst, Term, Term) {
        let mut bank =
            TermBank::new(Signature::new(TypeBank::new())).expect("term bank allocation");
        let negative = parse_in_bank(&mut bank, "a");
        let positive = parse_in_bank(&mut bank, "b");
        let subst = bound_subst(&bank, &[&negative, &positive]);
        let mut set = flat_anno_set_alloc();
        flat_anno_set_add_term(&mut set, FlatAnnoTerm::new(negative.clone(), -1.0, 1.0, 2));
        flat_anno_set_add_term(&mut set, FlatAnnoTerm::new(positive.clone(), 1.0, 1.0, 3));
        let mut admin =
            tsm_admin_alloc(bank.signature().clone(), TsmType::Flat).expect("admin allocation");
        tsm_admin_build_tsm(&mut admin, &set, IndexType::IDENTITY, 0, subst.clone())
            .expect("TSM build");
        (admin, set, subst, negative, positive)
    }

    #[test]
    fn term_classify_uses_eval_limit_and_writes_c_progress_text() {
        let (mut admin, _set, subst, negative, positive) = trained_admin();
        admin.set_eval_limit(0.0);
        let mut output = String::new();

        assert_close(tsm_term_classify(&mut admin, &negative, &subst), -1.0);
        assert_close(
            tsm_term_classify_write(&mut admin, &positive, &subst, &mut output)
                .expect("classification write"),
            1.0,
        );

        assert_eq!(output, "Evaluation:  1.0000 ");
    }

    #[test]
    fn classified_term_check_compares_class_to_raw_eval_label() {
        let (mut admin, _set, _subst, negative, _positive) = trained_admin();
        let labeled = FlatAnnoTerm::new(negative.clone(), -1.0, 1.0, 1);
        let scored = FlatAnnoTerm::new(negative, 0.0, 1.0, 1);
        let mut output = String::new();

        assert!(tsm_classified_term_check(&mut admin, &labeled));
        assert!(!tsm_classified_term_check(&mut admin, &scored));
        assert!(
            tsm_classified_term_check_write(&mut admin, &labeled, &mut output)
                .expect("check write")
        );

        assert_eq!(output, "Evaluation: -1.0000  Termeval: -1.0000 ");
    }

    #[test]
    fn classify_set_sums_successful_sources_and_prints_terms() {
        let (mut admin, set, _subst, _negative, _positive) = trained_admin();
        let mut output = String::new();

        assert_eq!(tsm_classify_set(&mut admin, &set), 5);
        assert_eq!(
            tsm_classify_set_write(&mut admin, &set, &mut output).expect("set write"),
            5
        );

        assert!(output.contains("OKOK a\n"));
        assert!(output.contains("OKOK b\n"));
        assert_eq!(output.matches("Evaluation:").count(), 2);
    }
}
