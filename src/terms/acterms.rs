use crate::terms::functypes::FunCode;
use crate::terms::signature::{Signature, FP_COMMUTATIVE, FP_IS_AC, SIG_DB_LAMBDA_CODE};
use crate::terms::termfunc::{term_standard_weight, var_print_string};
use crate::terms::termtypes::Term;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

static AC_TELEMETRY_ENABLED: AtomicBool = AtomicBool::new(false);
static AC_EQUALITY_CHECKS: AtomicU64 = AtomicU64::new(0);
static AC_EQUALITY_HITS: AtomicU64 = AtomicU64::new(0);
static AC_NORMALIZATIONS: AtomicU64 = AtomicU64::new(0);
static AC_INPUT_NODES: AtomicU64 = AtomicU64::new(0);
static AC_NORMALIZED_NODES: AtomicU64 = AtomicU64::new(0);
static AC_FLATTENED_NODES: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct AcTelemetrySnapshot {
    pub equality_checks: u64,
    pub equality_hits: u64,
    pub normalizations: u64,
    pub input_nodes: u64,
    pub normalized_nodes: u64,
    pub flattened_nodes: u64,
}

impl AcTelemetrySnapshot {
    #[must_use]
    pub const fn since(self, baseline: Self) -> Self {
        Self {
            equality_checks: self
                .equality_checks
                .saturating_sub(baseline.equality_checks),
            equality_hits: self.equality_hits.saturating_sub(baseline.equality_hits),
            normalizations: self.normalizations.saturating_sub(baseline.normalizations),
            input_nodes: self.input_nodes.saturating_sub(baseline.input_nodes),
            normalized_nodes: self
                .normalized_nodes
                .saturating_sub(baseline.normalized_nodes),
            flattened_nodes: self
                .flattened_nodes
                .saturating_sub(baseline.flattened_nodes),
        }
    }
}

#[must_use]
pub(crate) fn ac_telemetry_snapshot() -> AcTelemetrySnapshot {
    AcTelemetrySnapshot {
        equality_checks: AC_EQUALITY_CHECKS.load(Ordering::Relaxed),
        equality_hits: AC_EQUALITY_HITS.load(Ordering::Relaxed),
        normalizations: AC_NORMALIZATIONS.load(Ordering::Relaxed),
        input_nodes: AC_INPUT_NODES.load(Ordering::Relaxed),
        normalized_nodes: AC_NORMALIZED_NODES.load(Ordering::Relaxed),
        flattened_nodes: AC_FLATTENED_NODES.load(Ordering::Relaxed),
    }
}

pub(crate) struct AcTelemetryGuard;

impl Drop for AcTelemetryGuard {
    fn drop(&mut self) {
        AC_TELEMETRY_ENABLED.store(false, Ordering::Relaxed);
    }
}

#[must_use]
pub(crate) fn enable_ac_telemetry() -> AcTelemetryGuard {
    AC_TELEMETRY_ENABLED.store(true, Ordering::Relaxed);
    AcTelemetryGuard
}

#[derive(Default)]
struct AcNormalizationMetrics {
    input_nodes: u64,
    normalized_nodes: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AcTerm {
    f_code: FunCode,
    args: Vec<AcTerm>,
}

impl AcTerm {
    #[must_use]
    pub const fn f_code(&self) -> FunCode {
        self.f_code
    }

    #[must_use]
    pub fn args(&self) -> &[Self] {
        &self.args
    }
}

#[must_use]
pub const fn ac_term_alloc(f_code: FunCode) -> AcTerm {
    AcTerm {
        f_code,
        args: Vec::new(),
    }
}

#[must_use]
pub fn ac_term_compare(left: &AcTerm, right: &AcTerm) -> i32 {
    let mut result = if left.f_code == SIG_DB_LAMBDA_CODE || right.f_code == SIG_DB_LAMBDA_CODE {
        -1
    } else {
        cmp_fun_code(left.f_code, right.f_code)
    };

    if result == 0 && left.f_code > 0 {
        for index in 0.. {
            match (left.args.get(index), right.args.get(index)) {
                (None, None) => break,
                (None, Some(_)) => {
                    result = -1;
                    break;
                }
                (Some(_), None) => {
                    result = 1;
                    break;
                }
                (Some(left_arg), Some(right_arg)) => {
                    result = ac_term_compare(left_arg, right_arg);
                    if result != 0 {
                        break;
                    }
                }
            }
        }
    }

    result
}

#[must_use]
pub fn ac_term_normalize(sig: &Signature, term: &Term) -> AcTerm {
    let mut metrics = AcNormalizationMetrics::default();
    let normalized = ac_term_normalize_with_metrics(sig, term, &mut metrics);
    record_normalization(metrics);
    normalized
}

fn ac_term_normalize_with_metrics(
    sig: &Signature,
    term: &Term,
    metrics: &mut AcNormalizationMetrics,
) -> AcTerm {
    metrics.input_nodes = metrics.input_nodes.saturating_add(1);
    metrics.normalized_nodes = metrics.normalized_nodes.saturating_add(1);
    let mut handle = ac_term_alloc(term.f_code());

    if !term.is_any_var() && !term.is_lambda() && term.arity() != 0 {
        if sig.query_prop(term.f_code(), FP_IS_AC) {
            ac_collect_args_with_metrics(&mut handle.args, sig, term.f_code(), term, metrics);
            handle.args.sort_by(|left, right| {
                ac_term_compare(left, right)
                    .cmp(&0)
                    .then_with(|| left.f_code.cmp(&right.f_code))
            });
        } else if sig.query_prop(term.f_code(), FP_COMMUTATIVE) && term.arity() == 2 {
            let mut args = term.argument_clones().into_iter().flatten();
            if let (Some(left), Some(right)) = (args.next(), args.next()) {
                let mut left = ac_term_normalize_with_metrics(sig, &left, metrics);
                let mut right = ac_term_normalize_with_metrics(sig, &right, metrics);
                if ac_term_compare(&left, &right) > 0 {
                    std::mem::swap(&mut left, &mut right);
                }
                handle.args.push(left);
                handle.args.push(right);
            }
        } else {
            for arg in term.argument_clones().into_iter().flatten() {
                handle
                    .args
                    .push(ac_term_normalize_with_metrics(sig, &arg, metrics));
            }
        }
    }

    handle
}

#[must_use]
pub fn ac_term_print_string(term: &AcTerm, sig: &Signature) -> String {
    if term.f_code < 0 {
        return var_print_string(term.f_code);
    }

    let mut result = sig
        .find_name(term.f_code)
        .map_or_else(|| term.f_code.to_string(), ToOwned::to_owned);
    if let Some((first, rest)) = term.args.split_first() {
        result.push('(');
        result.push_str(&ac_term_print_string(first, sig));
        for arg in rest {
            result.push(',');
            result.push_str(&ac_term_print_string(arg, sig));
        }
        result.push(')');
    }
    result
}

#[must_use]
pub fn term_ac_equal(sig: &Signature, left: &Term, right: &Term) -> bool {
    record_ac_counter(&AC_EQUALITY_CHECKS, 1);
    if term_standard_weight(left) != term_standard_weight(right)
        || left.is_phony_app()
        || right.is_phony_app()
    {
        return false;
    }

    let left = ac_term_normalize(sig, left);
    let right = ac_term_normalize(sig, right);
    let equal = ac_term_compare(&left, &right) == 0;
    if equal {
        record_ac_counter(&AC_EQUALITY_HITS, 1);
    }
    equal
}

fn ac_collect_args_with_metrics(
    args: &mut Vec<AcTerm>,
    sig: &Signature,
    f_code: FunCode,
    term: &Term,
    metrics: &mut AcNormalizationMetrics,
) {
    if term.f_code() == f_code && !term.is_any_var() && !term.is_lambda() {
        for arg in term.argument_clones().into_iter().flatten() {
            if arg.f_code() == f_code && !arg.is_any_var() && !arg.is_lambda() {
                metrics.input_nodes = metrics.input_nodes.saturating_add(1);
                ac_collect_args_with_metrics(args, sig, f_code, &arg, metrics);
            } else {
                args.push(ac_term_normalize_with_metrics(sig, &arg, metrics));
            }
        }
    } else {
        args.push(ac_term_normalize_with_metrics(sig, term, metrics));
    }
}

fn record_normalization(metrics: AcNormalizationMetrics) {
    if !AC_TELEMETRY_ENABLED.load(Ordering::Relaxed) {
        return;
    }
    AC_NORMALIZATIONS.fetch_add(1, Ordering::Relaxed);
    AC_INPUT_NODES.fetch_add(metrics.input_nodes, Ordering::Relaxed);
    AC_NORMALIZED_NODES.fetch_add(metrics.normalized_nodes, Ordering::Relaxed);
    AC_FLATTENED_NODES.fetch_add(
        metrics.input_nodes.saturating_sub(metrics.normalized_nodes),
        Ordering::Relaxed,
    );
}

fn record_ac_counter(counter: &AtomicU64, value: u64) {
    if AC_TELEMETRY_ENABLED.load(Ordering::Relaxed) {
        counter.fetch_add(value, Ordering::Relaxed);
    }
}

fn cmp_fun_code(left: FunCode, right: FunCode) -> i32 {
    i32::from(left > right) - i32::from(left < right)
}

#[cfg(test)]
mod tests {
    use super::{
        ac_telemetry_snapshot, ac_term_alloc, ac_term_compare, ac_term_normalize,
        ac_term_print_string, enable_ac_telemetry, term_ac_equal,
    };
    use crate::terms::signature::{
        Signature, FP_COMMUTATIVE, FP_IS_AC, SIG_DB_LAMBDA_CODE, SIG_PHONY_APP_CODE,
    };
    use crate::terms::termtypes::Term;
    use crate::terms::typebanks::TypeBank;

    fn signature_with_symbols() -> (Signature, i64, i64, i64) {
        let mut sig = Signature::new(TypeBank::new());
        let f = sig.insert_id("f", 2, false);
        let a = sig.insert_id("a", 0, false);
        let b = sig.insert_id("b", 0, false);
        (sig, f, a, b)
    }

    fn binary(f_code: i64, left: &Term, right: &Term) -> Term {
        let term = Term::top_alloc(f_code, 2);
        term.set_argument(0, left.clone());
        term.set_argument(1, right.clone());
        term
    }

    #[test]
    fn allocation_and_compare_follow_c_shape() {
        let var = ac_term_alloc(-2);
        let constant = ac_term_alloc(3);

        assert_eq!(var.f_code(), -2);
        assert!(var.args().is_empty());
        assert_eq!(ac_term_compare(&var, &constant), -1);
        assert_eq!(ac_term_compare(&constant, &var), 1);
    }

    #[test]
    fn compare_preserves_db_lambda_quirk() {
        let lambda = ac_term_alloc(SIG_DB_LAMBDA_CODE);

        assert_eq!(ac_term_compare(&lambda, &lambda), -1);
    }

    #[test]
    fn commutative_normalization_sorts_binary_arguments() {
        let (mut sig, f, a_code, b_code) = signature_with_symbols();
        sig.set_func_prop(f, FP_COMMUTATIVE);
        let a = Term::const_cell_alloc(a_code);
        let b = Term::const_cell_alloc(b_code);
        let term = binary(f, &b, &a);

        let normalized = ac_term_normalize(&sig, &term);

        assert_eq!(normalized.args()[0].f_code(), a_code);
        assert_eq!(normalized.args()[1].f_code(), b_code);
        assert_eq!(ac_term_print_string(&normalized, &sig), "f(a,b)");
        assert!(term_ac_equal(&sig, &binary(f, &a, &b), &term));
    }

    #[test]
    fn ac_normalization_flattens_and_sorts_nested_terms() {
        let (mut sig, f, a_code, b_code) = signature_with_symbols();
        sig.set_func_prop(f, FP_IS_AC);
        let a = Term::const_cell_alloc(a_code);
        let b = Term::const_cell_alloc(b_code);
        let left_assoc = binary(f, &b, &binary(f, &a, &b));
        let right_assoc = binary(f, &binary(f, &b, &a), &b);

        let normalized = ac_term_normalize(&sig, &left_assoc);

        assert_eq!(normalized.args().len(), 3);
        assert_eq!(
            normalized
                .args()
                .iter()
                .map(super::AcTerm::f_code)
                .collect::<Vec<_>>(),
            vec![a_code, b_code, b_code]
        );
        assert_eq!(ac_term_print_string(&normalized, &sig), "f(a,b,b)");
        assert!(term_ac_equal(&sig, &left_assoc, &right_assoc));
    }

    #[test]
    fn ac_equality_rejects_weight_mismatches_and_phony_apps() {
        let (mut sig, f, a_code, b_code) = signature_with_symbols();
        sig.set_func_prop(f, FP_IS_AC);
        let a = Term::const_cell_alloc(a_code);
        let b = Term::const_cell_alloc(b_code);
        let flat = Term::top_alloc(f, 3);
        flat.set_argument(0, a.clone());
        flat.set_argument(1, b.clone());
        flat.set_argument(2, b.clone());
        let nested = binary(f, &a, &binary(f, &b, &b));
        let phony = Term::top_alloc(SIG_PHONY_APP_CODE, 1);
        phony.set_argument(0, a);

        assert!(!term_ac_equal(&sig, &flat, &nested));
        assert!(!term_ac_equal(&sig, &phony, &phony));
    }

    #[test]
    fn ac_normalization_canonicalizes_permutations_and_associations() {
        let (mut sig, f, a_code, b_code) = signature_with_symbols();
        let c_code = sig.insert_id("c", 0, false);
        sig.set_func_prop(f, FP_IS_AC);
        let constants = [
            Term::const_cell_alloc(a_code),
            Term::const_cell_alloc(b_code),
            Term::const_cell_alloc(c_code),
        ];
        let permutations = [
            [0, 1, 2],
            [0, 2, 1],
            [1, 0, 2],
            [1, 2, 0],
            [2, 0, 1],
            [2, 1, 0],
        ];
        let expected = binary(f, &binary(f, &constants[0], &constants[1]), &constants[2]);

        for permutation in permutations {
            let left_associated = binary(
                f,
                &binary(f, &constants[permutation[0]], &constants[permutation[1]]),
                &constants[permutation[2]],
            );
            let right_associated = binary(
                f,
                &constants[permutation[0]],
                &binary(f, &constants[permutation[1]], &constants[permutation[2]]),
            );
            assert!(term_ac_equal(&sig, &expected, &left_associated));
            assert!(term_ac_equal(&sig, &expected, &right_associated));
        }
    }

    #[test]
    fn ac_normalization_preserves_multiplicity_and_variable_identity() {
        let (mut sig, f, a_code, b_code) = signature_with_symbols();
        sig.set_func_prop(f, FP_IS_AC);
        let a = Term::const_cell_alloc(a_code);
        let b = Term::const_cell_alloc(b_code);
        let x = Term::const_cell_alloc(-2);
        let y = Term::const_cell_alloc(-4);

        assert!(!term_ac_equal(
            &sig,
            &binary(f, &a, &binary(f, &a, &b)),
            &binary(f, &a, &binary(f, &b, &b))
        ));
        assert!(term_ac_equal(
            &sig,
            &binary(f, &x, &binary(f, &y, &x)),
            &binary(f, &x, &binary(f, &x, &y))
        ));
        assert!(!term_ac_equal(
            &sig,
            &binary(f, &x, &binary(f, &y, &x)),
            &binary(f, &x, &binary(f, &y, &y))
        ));
    }

    #[test]
    fn nested_commutative_terms_are_canonical_inside_ac_terms() {
        let (mut sig, f, a_code, b_code) = signature_with_symbols();
        let g = sig.insert_id("g", 2, false);
        sig.set_func_prop(f, FP_IS_AC);
        sig.set_func_prop(g, FP_COMMUTATIVE);
        let a = Term::const_cell_alloc(a_code);
        let b = Term::const_cell_alloc(b_code);

        assert!(term_ac_equal(
            &sig,
            &binary(f, &binary(g, &b, &a), &a),
            &binary(f, &a, &binary(g, &a, &b))
        ));
    }

    #[test]
    fn scoped_telemetry_counts_ac_checks_hits_and_flattening() {
        let (mut sig, f, a_code, b_code) = signature_with_symbols();
        sig.set_func_prop(f, FP_IS_AC);
        let a = Term::const_cell_alloc(a_code);
        let b = Term::const_cell_alloc(b_code);
        let left = binary(f, &a, &binary(f, &b, &a));
        let right = binary(f, &binary(f, &a, &b), &a);
        let baseline = ac_telemetry_snapshot();

        {
            let _guard = enable_ac_telemetry();
            assert!(term_ac_equal(&sig, &left, &right));
        }

        let delta = ac_telemetry_snapshot().since(baseline);
        assert_eq!(delta.equality_checks, 1);
        assert_eq!(delta.equality_hits, 1);
        assert_eq!(delta.normalizations, 2);
        assert_eq!(delta.input_nodes, 10);
        assert_eq!(delta.normalized_nodes, 8);
        assert_eq!(delta.flattened_nodes, 2);
    }
}
