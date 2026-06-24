use crate::basics::simple_stuff::{problem_type, ProblemType};
use crate::terms::functypes::FunCode;
use crate::terms::signature::SIG_DB_LAMBDA_CODE;
use crate::terms::simpletypes::type_get_max_arity;
use crate::terms::termtypes::{Term, TP_PRED_POS};
use std::sync::{Mutex, MutexGuard, OnceLock};

pub const BELOW_VAR: FunCode = -2;
pub const ANY_VAR: FunCode = -1;
pub const NOT_IN_TERM: FunCode = 0;
pub const MAX_PM_INDEX_NAME_LEN: usize = 20;

pub const FP_INDEX_NAMES: &[&str] = &[
    "FP0", "FPfp", "FP1", "FP2", "FP3D", "FP3W", "FP4D", "FP4W", "FP4M", "FP5M", "FP6M", "FP7",
    "FP7M", "FP4X2_2", "FP3DFlex", "NPDT", "NoIndex",
];

pub type FingerprintIndexFunction = fn(&Term) -> IndexFingerprint;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IndexFingerprint {
    raw: Vec<FunCode>,
}

impl IndexFingerprint {
    #[must_use]
    pub fn from_samples(samples: Vec<FunCode>) -> Self {
        fingerprint_from_sample_vec(samples)
    }

    #[must_use]
    pub fn raw(&self) -> &[FunCode] {
        &self.raw
    }

    #[must_use]
    pub fn samples(&self) -> &[FunCode] {
        &self.raw[1..]
    }

    #[must_use]
    pub fn print_string(&self) -> String {
        if self.raw.first().copied().unwrap_or(0) < 2 {
            return "<>".to_owned();
        }

        let mut output = String::new();
        output.push('<');
        output.push_str(&self.raw[1].to_string());
        for sample in &self.raw[2..] {
            output.push(',');
            output.push_str(&sample.to_string());
        }
        output.push('>');
        output
    }
}

#[must_use]
pub fn term_fp_sample_fo(term: &Term, position: &[usize]) -> FunCode {
    let mut current = term.clone();
    for &pos in position {
        if current.is_free_var() {
            return BELOW_VAR;
        }
        if pos >= current.arity() {
            return NOT_IN_TERM;
        }
        current = required_arg(&current, pos);
    }
    if current.is_free_var() {
        ANY_VAR
    } else {
        current.f_code()
    }
}

#[must_use]
pub fn term_fp_sample_ho(term: &Term, position: &[usize]) -> FunCode {
    let mut current = term.clone();
    for &pos in position {
        while current.is_lambda() {
            current = required_arg(&current, 1);
        }

        if current.is_top_level_free_var() {
            return BELOW_VAR;
        }

        let arity = current.arity();
        if pos < arity {
            current = required_arg(&current, pos);
        } else if pos < arity + term_type_max_arity(&current) {
            return SIG_DB_LAMBDA_CODE;
        } else {
            return NOT_IN_TERM;
        }
    }

    if current.is_top_level_free_var() {
        ANY_VAR
    } else if current.is_top_level_any_var() {
        SIG_DB_LAMBDA_CODE
    } else {
        current.f_code()
    }
}

/// Samples a term from a C-shaped integer sequence.
///
/// The sequence encodes one position terminated by `-1`. `cursor` is advanced
/// past that terminator, or past the skipped terminator after early failure.
///
/// # Panics
///
/// Panics if the sequence is missing its `-1` terminator, contains a negative
/// non-terminator position, or if a traversed term argument is uninitialized.
pub fn term_fp_flex_sample_fo(term: &Term, sequence: &[i64], cursor: &mut usize) -> FunCode {
    let mut current = term.clone();
    let mut failed = false;
    let mut result = NOT_IN_TERM;

    loop {
        let pos = *sequence
            .get(*cursor)
            .expect("fingerprint position sequence must be terminated by -1");
        if pos == -1 {
            break;
        }
        if !failed {
            if current.is_free_var() {
                result = BELOW_VAR;
                failed = true;
            } else {
                let pos = usize::try_from(pos)
                    .expect("fingerprint position must be non-negative before terminator");
                if pos >= current.arity() {
                    result = NOT_IN_TERM;
                    failed = true;
                } else {
                    current = required_arg(&current, pos);
                }
            }
        }
        *cursor += 1;
    }

    if !failed {
        result = if current.is_free_var() {
            ANY_VAR
        } else {
            current.f_code()
        };
    }
    *cursor += 1;
    result
}

#[must_use]
pub fn term_fp_sample_for_problem(
    term: &Term,
    position: &[usize],
    problem_type: ProblemType,
) -> FunCode {
    if problem_type == ProblemType::HigherOrder {
        term_fp_sample_ho(term, position)
    } else {
        term_fp_sample_fo(term, position)
    }
}

/// Samples a term using higher-order fingerprinting.
///
/// # Panics
///
/// Panics if the sequence is missing its `-1` terminator, contains a negative
/// non-terminator position, if a traversed term argument is uninitialized, or
/// if higher-order trailing-argument sampling reaches an untyped term.
pub fn term_fp_flex_sample_ho(term: &Term, sequence: &[i64], cursor: &mut usize) -> FunCode {
    let mut current = term.clone();
    let mut failed = false;
    let mut result = NOT_IN_TERM;

    loop {
        let pos = *sequence
            .get(*cursor)
            .expect("fingerprint position sequence must be terminated by -1");
        if pos == -1 {
            break;
        }
        if !failed {
            while current.is_lambda() {
                current = required_arg(&current, 1);
            }

            if current.is_top_level_free_var() {
                result = BELOW_VAR;
                failed = true;
            } else {
                let pos = usize::try_from(pos)
                    .expect("fingerprint position must be non-negative before terminator");
                let arity = current.arity();
                if pos < arity {
                    current = required_arg(&current, pos);
                } else if pos < arity + term_type_max_arity(&current) {
                    result = SIG_DB_LAMBDA_CODE;
                    failed = true;
                } else {
                    result = NOT_IN_TERM;
                    failed = true;
                }
            }
        }
        *cursor += 1;
    }

    if !failed {
        result = if current.is_top_level_free_var() {
            ANY_VAR
        } else if current.is_top_level_any_var() {
            SIG_DB_LAMBDA_CODE
        } else {
            current.f_code()
        };
    }
    *cursor += 1;
    result
}

#[must_use]
pub fn term_fp_sample(term: &Term, position: &[usize]) -> FunCode {
    term_fp_sample_for_problem(term, position, problem_type())
}

/// Samples a term with the first-order or higher-order flexible sampler.
///
/// # Panics
///
/// Panics under the same malformed sequence, uninitialized-argument, or
/// higher-order typing conditions as the selected sampler.
pub fn term_fp_flex_sample_for_problem(
    term: &Term,
    sequence: &[i64],
    cursor: &mut usize,
    problem_type: ProblemType,
) -> FunCode {
    if problem_type == ProblemType::HigherOrder {
        term_fp_flex_sample_ho(term, sequence, cursor)
    } else {
        term_fp_flex_sample_fo(term, sequence, cursor)
    }
}

/// C-compatible dispatch wrapper for flexible sampling.
///
/// # Panics
///
/// Panics under the same malformed sequence, uninitialized-argument, or
/// higher-order typing conditions as the selected first-order or higher-order
/// sampler.
pub fn term_fp_flex_sample(term: &Term, sequence: &[i64], cursor: &mut usize) -> FunCode {
    term_fp_flex_sample_for_problem(term, sequence, cursor, problem_type())
}

#[must_use]
pub fn index_fp0_create(_term: &Term) -> IndexFingerprint {
    fingerprint_from_samples([])
}

#[must_use]
pub fn index_fp_fp_create(term: &Term) -> IndexFingerprint {
    let mut sample = term_fp_sample(term, &[]);
    if sample > 0 {
        let mut reps = lock_or_recover(fp_fp_representatives());
        let representative = if term.query_prop(TP_PRED_POS) {
            reps.predicate.get_or_insert(sample)
        } else {
            reps.function.get_or_insert(sample)
        };
        sample = *representative;
    }
    fingerprint_from_samples([sample])
}

#[must_use]
pub fn index_fp1_create(term: &Term) -> IndexFingerprint {
    fingerprint_from_samples([term_fp_sample(term, &[])])
}

#[must_use]
pub fn index_fp2_create(term: &Term) -> IndexFingerprint {
    fingerprint_from_samples([term_fp_sample(term, &[]), term_fp_sample(term, &[0])])
}

#[must_use]
pub fn index_fp3d_create(term: &Term) -> IndexFingerprint {
    fingerprint_from_samples([
        term_fp_sample(term, &[]),
        term_fp_sample(term, &[0]),
        term_fp_sample(term, &[0, 0]),
    ])
}

#[must_use]
pub fn index_fp3w_create(term: &Term) -> IndexFingerprint {
    fingerprint_from_samples([
        term_fp_sample(term, &[]),
        term_fp_sample(term, &[0]),
        term_fp_sample(term, &[1]),
    ])
}

#[must_use]
pub fn index_fp4d_create(term: &Term) -> IndexFingerprint {
    fingerprint_from_samples([
        term_fp_sample(term, &[]),
        term_fp_sample(term, &[0]),
        term_fp_sample(term, &[0, 0]),
        term_fp_sample(term, &[0, 0, 0]),
    ])
}

#[must_use]
pub fn index_fp4w_create(term: &Term) -> IndexFingerprint {
    fingerprint_from_samples([
        term_fp_sample(term, &[]),
        term_fp_sample(term, &[0]),
        term_fp_sample(term, &[1]),
        term_fp_sample(term, &[2]),
    ])
}

#[must_use]
pub fn index_fp4m_create(term: &Term) -> IndexFingerprint {
    fingerprint_from_samples([
        term_fp_sample(term, &[]),
        term_fp_sample(term, &[0]),
        term_fp_sample(term, &[1]),
        term_fp_sample(term, &[0, 0]),
    ])
}

#[must_use]
pub fn index_fp5m_create(term: &Term) -> IndexFingerprint {
    fingerprint_from_samples([
        term_fp_sample(term, &[]),
        term_fp_sample(term, &[0]),
        term_fp_sample(term, &[1]),
        term_fp_sample(term, &[2]),
        term_fp_sample(term, &[0, 0]),
    ])
}

#[must_use]
pub fn index_fp6m_create(term: &Term) -> IndexFingerprint {
    fingerprint_from_samples([
        term_fp_sample(term, &[]),
        term_fp_sample(term, &[0]),
        term_fp_sample(term, &[1]),
        term_fp_sample(term, &[2]),
        term_fp_sample(term, &[0, 0]),
        term_fp_sample(term, &[0, 1]),
    ])
}

#[must_use]
pub fn index_fp7_create(term: &Term) -> IndexFingerprint {
    fingerprint_from_samples([
        term_fp_sample(term, &[]),
        term_fp_sample(term, &[0]),
        term_fp_sample(term, &[1]),
        term_fp_sample(term, &[0, 0]),
        term_fp_sample(term, &[0, 1]),
        term_fp_sample(term, &[1, 0]),
        term_fp_sample(term, &[1, 1]),
    ])
}

#[must_use]
pub fn index_fp7m_create(term: &Term) -> IndexFingerprint {
    fingerprint_from_samples([
        term_fp_sample(term, &[]),
        term_fp_sample(term, &[0]),
        term_fp_sample(term, &[1]),
        term_fp_sample(term, &[2]),
        term_fp_sample(term, &[3]),
        term_fp_sample(term, &[0, 0]),
        term_fp_sample(term, &[0, 1]),
    ])
}

#[must_use]
pub fn index_fp4x2_2_create(term: &Term) -> IndexFingerprint {
    fingerprint_from_samples([
        term_fp_sample(term, &[]),
        term_fp_sample(term, &[0]),
        term_fp_sample(term, &[1]),
        term_fp_sample(term, &[2]),
        term_fp_sample(term, &[3]),
        term_fp_sample(term, &[0, 0]),
        term_fp_sample(term, &[0, 1]),
        term_fp_sample(term, &[0, 2]),
        term_fp_sample(term, &[1, 0]),
        term_fp_sample(term, &[1, 1]),
        term_fp_sample(term, &[1, 2]),
        term_fp_sample(term, &[2, 0]),
        term_fp_sample(term, &[2, 1]),
        term_fp_sample(term, &[2, 2]),
        term_fp_sample(term, &[0, 0, 0]),
        term_fp_sample(term, &[1, 0, 0]),
    ])
}

/// Creates a fingerprint from concatenated `-1`-terminated positions.
///
/// The whole list must end with `-2`, matching the C `IndexFPFlexCreate`
/// position-stack sentinel.
///
/// # Panics
///
/// Panics if the position sequence is malformed, contains fewer than `len`
/// positions, or if a traversed term argument is uninitialized.
#[must_use]
pub fn index_fp_flex_create(term: &Term, positions: &[i64], len: usize) -> IndexFingerprint {
    let mut cursor = 0;
    let mut samples = Vec::with_capacity(len);
    while positions
        .get(cursor)
        .copied()
        .expect("flex fingerprint sequence must end with -2")
        != -2
    {
        samples.push(term_fp_flex_sample(term, positions, &mut cursor));
    }
    assert_eq!(
        samples.len(),
        len,
        "flex fingerprint length must match sampled positions"
    );
    fingerprint_from_sample_vec(samples)
}

#[must_use]
pub fn index_fp3d_flex_create(term: &Term) -> IndexFingerprint {
    index_fp_flex_create(term, &[-1, 0, -1, 0, 0, -1, -2], 3)
}

#[must_use]
pub fn index_dt_create(term: &Term) -> IndexFingerprint {
    let mut samples = Vec::new();
    push_fcodes(&mut samples, term);
    fingerprint_from_sample_vec(samples)
}

#[must_use]
pub fn get_fp_index_function(name: &str) -> Option<FingerprintIndexFunction> {
    if name == "NoIndex" {
        return None;
    }

    match name {
        "FP0" => Some(index_fp0_create),
        "FPfp" => Some(index_fp_fp_create),
        "FP1" => Some(index_fp1_create),
        "FP2" => Some(index_fp2_create),
        "FP3D" => Some(index_fp3d_create),
        "FP3W" => Some(index_fp3w_create),
        "FP4D" => Some(index_fp4d_create),
        "FP4W" => Some(index_fp4w_create),
        "FP4M" => Some(index_fp4m_create),
        "FP5M" => Some(index_fp5m_create),
        "FP6M" => Some(index_fp6m_create),
        "FP7" => Some(index_fp7_create),
        "FP7M" => Some(index_fp7m_create),
        "FP4X2_2" => Some(index_fp4x2_2_create),
        "FP3DFlex" => Some(index_fp3d_flex_create),
        "NPDT" => Some(index_dt_create),
        _ => None,
    }
}

fn fingerprint_from_samples<const N: usize>(samples: [FunCode; N]) -> IndexFingerprint {
    fingerprint_from_sample_vec(samples.to_vec())
}

fn fingerprint_from_sample_vec(samples: Vec<FunCode>) -> IndexFingerprint {
    let len = samples
        .len()
        .checked_add(1)
        .and_then(|len| FunCode::try_from(len).ok())
        .expect("fingerprint length must fit in FunCode");
    let mut raw = Vec::with_capacity(samples.len() + 1);
    raw.push(len);
    raw.extend(samples);
    IndexFingerprint { raw }
}

fn push_fcodes(samples: &mut Vec<FunCode>, term: &Term) {
    if term.is_free_var() {
        samples.push(ANY_VAR);
        return;
    }

    if !term.is_phony_app() {
        samples.push(term.f_code());
    }
    for index in 0..term.arity() {
        push_fcodes(samples, &required_arg(term, index));
    }
}

fn required_arg(term: &Term, index: usize) -> Term {
    term.argument(index)
        .unwrap_or_else(|| panic!("term argument {index} is uninitialized"))
}

fn term_type_max_arity(term: &Term) -> usize {
    term.type_()
        .as_ref()
        .map(type_get_max_arity)
        .expect("higher-order fingerprint sample requires typed term")
}

#[derive(Debug, Default)]
struct FpFpRepresentatives {
    function: Option<FunCode>,
    predicate: Option<FunCode>,
}

static FP_FP_REPRESENTATIVES: OnceLock<Mutex<FpFpRepresentatives>> = OnceLock::new();

fn fp_fp_representatives() -> &'static Mutex<FpFpRepresentatives> {
    FP_FP_REPRESENTATIVES.get_or_init(|| Mutex::new(FpFpRepresentatives::default()))
}

fn lock_or_recover<T>(mutex: &'static Mutex<T>) -> MutexGuard<'static, T> {
    match mutex.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

#[cfg(test)]
fn reset_fp_fp_representatives_for_tests() {
    *lock_or_recover(fp_fp_representatives()) = FpFpRepresentatives::default();
}

#[cfg(test)]
mod tests {
    use super::{
        get_fp_index_function, index_dt_create, index_fp0_create, index_fp1_create,
        index_fp2_create, index_fp3d_create, index_fp3d_flex_create, index_fp3w_create,
        index_fp4d_create, index_fp4m_create, index_fp4w_create, index_fp4x2_2_create,
        index_fp5m_create, index_fp6m_create, index_fp7_create, index_fp7m_create,
        index_fp_flex_create, index_fp_fp_create, reset_fp_fp_representatives_for_tests,
        term_fp_flex_sample_fo, term_fp_flex_sample_for_problem, term_fp_sample_fo,
        term_fp_sample_for_problem, ANY_VAR, BELOW_VAR, FP_INDEX_NAMES, MAX_PM_INDEX_NAME_LEN,
        NOT_IN_TERM,
    };
    use crate::basics::simple_stuff::ProblemType;
    use crate::terms::signature::{SIG_DB_LAMBDA_CODE, SIG_PHONY_APP_CODE};
    use crate::terms::simpletypes::{alloc_arrow_type, Type};
    use crate::terms::termtypes::{Term, TP_IS_DB_VAR, TP_PRED_POS};
    use crate::terms::typebanks::TypeBank;

    fn leaf(code: i64) -> Term {
        Term::const_cell_alloc(code)
    }

    fn var(code: i64) -> Term {
        Term::const_cell_alloc(code)
    }

    fn term(code: i64, args: &[Term]) -> Term {
        let term = Term::top_alloc(code, args.len());
        for (index, arg) in args.iter().enumerate() {
            term.set_argument(index, arg.clone());
        }
        term
    }

    fn typed_leaf(code: i64, type_: &Type) -> Term {
        let term = leaf(code);
        term.set_type(Some(type_.clone()));
        term
    }

    fn typed_var(code: i64, type_: &Type) -> Term {
        let term = var(code);
        term.set_type(Some(type_.clone()));
        term
    }

    fn typed_term(code: i64, args: &[Term], type_: &Type) -> Term {
        let term = term(code, args);
        term.set_type(Some(type_.clone()));
        term
    }

    fn sample_term() -> Term {
        let a = leaf(10);
        let g = term(30, &[a]);
        let x = var(-2);
        term(20, &[g, x])
    }

    #[test]
    fn constants_and_names_match_c_surface() {
        assert_eq!(BELOW_VAR, -2);
        assert_eq!(ANY_VAR, -1);
        assert_eq!(NOT_IN_TERM, 0);
        assert_eq!(MAX_PM_INDEX_NAME_LEN, 20);
        assert_eq!(
            FP_INDEX_NAMES,
            &[
                "FP0", "FPfp", "FP1", "FP2", "FP3D", "FP3W", "FP4D", "FP4W", "FP4M", "FP5M",
                "FP6M", "FP7", "FP7M", "FP4X2_2", "FP3DFlex", "NPDT", "NoIndex"
            ]
        );
    }

    #[test]
    fn first_order_sampling_reports_symbols_variables_below_var_and_absence() {
        let root = sample_term();

        assert_eq!(term_fp_sample_fo(&root, &[]), 20);
        assert_eq!(term_fp_sample_fo(&root, &[0]), 30);
        assert_eq!(term_fp_sample_fo(&root, &[0, 0]), 10);
        assert_eq!(term_fp_sample_fo(&root, &[1]), ANY_VAR);
        assert_eq!(term_fp_sample_fo(&root, &[1, 0]), BELOW_VAR);
        assert_eq!(term_fp_sample_fo(&root, &[2]), NOT_IN_TERM);
        assert_eq!(term_fp_sample_fo(&root, &[0, 1]), NOT_IN_TERM);
    }

    #[test]
    fn flexible_sampling_advances_past_each_position() {
        let root = sample_term();
        let sequence = [-1, 0, -1, 0, 0, -1, 1, 0, -1, -2];
        let mut cursor = 0;

        assert_eq!(term_fp_flex_sample_fo(&root, &sequence, &mut cursor), 20);
        assert_eq!(cursor, 1);
        assert_eq!(term_fp_flex_sample_fo(&root, &sequence, &mut cursor), 30);
        assert_eq!(cursor, 3);
        assert_eq!(term_fp_flex_sample_fo(&root, &sequence, &mut cursor), 10);
        assert_eq!(cursor, 6);
        assert_eq!(
            term_fp_flex_sample_fo(&root, &sequence, &mut cursor),
            BELOW_VAR
        );
        assert_eq!(cursor, 9);
    }

    #[test]
    fn higher_order_sampling_skips_lambdas_and_uses_trailing_type_arity() {
        let type_bank = TypeBank::new();
        let individual = type_bank.i_type();
        let predicate_type = alloc_arrow_type(vec![individual.clone(), type_bank.bool_type()]);
        let a = typed_leaf(10, &individual);
        let body = typed_term(20, std::slice::from_ref(&a), &predicate_type);
        let binder = typed_var(-2, &individual);
        let lambda = typed_term(SIG_DB_LAMBDA_CODE, &[binder, body.clone()], &predicate_type);

        assert_eq!(
            term_fp_sample_for_problem(&lambda, &[], ProblemType::HigherOrder),
            SIG_DB_LAMBDA_CODE
        );
        assert_eq!(
            term_fp_sample_for_problem(&lambda, &[0], ProblemType::HigherOrder),
            10
        );
        assert_eq!(
            term_fp_sample_for_problem(&body, &[1], ProblemType::HigherOrder),
            SIG_DB_LAMBDA_CODE
        );
        assert_eq!(
            term_fp_sample_for_problem(&body, &[2], ProblemType::HigherOrder),
            NOT_IN_TERM
        );
        assert_eq!(
            term_fp_sample_for_problem(&body, &[1], ProblemType::FirstOrder),
            NOT_IN_TERM
        );
    }

    #[test]
    fn higher_order_sampling_classifies_applied_variables_like_c_macros() {
        let type_bank = TypeBank::new();
        let individual = type_bank.i_type();
        let free_head = typed_var(-2, &individual);
        let db_head = typed_leaf(99, &individual);
        db_head.set_prop(TP_IS_DB_VAR);
        let arg = typed_leaf(10, &individual);
        let free_app = typed_term(SIG_PHONY_APP_CODE, &[free_head, arg.clone()], &individual);
        let db_app = typed_term(SIG_PHONY_APP_CODE, &[db_head, arg], &individual);

        assert_eq!(
            term_fp_sample_for_problem(&free_app, &[], ProblemType::HigherOrder),
            ANY_VAR
        );
        assert_eq!(
            term_fp_sample_for_problem(&free_app, &[0], ProblemType::HigherOrder),
            BELOW_VAR
        );
        assert_eq!(
            term_fp_sample_for_problem(&db_app, &[], ProblemType::HigherOrder),
            SIG_DB_LAMBDA_CODE
        );
    }

    #[test]
    fn higher_order_flexible_sampling_advances_past_each_position() {
        let type_bank = TypeBank::new();
        let individual = type_bank.i_type();
        let predicate_type = alloc_arrow_type(vec![individual.clone(), type_bank.bool_type()]);
        let a = typed_leaf(10, &individual);
        let body = typed_term(20, &[a], &predicate_type);
        let sequence = [1, -1, 2, -1, -2];
        let mut cursor = 0;

        assert_eq!(
            term_fp_flex_sample_for_problem(
                &body,
                &sequence,
                &mut cursor,
                ProblemType::HigherOrder
            ),
            SIG_DB_LAMBDA_CODE
        );
        assert_eq!(cursor, 2);
        assert_eq!(
            term_fp_flex_sample_for_problem(
                &body,
                &sequence,
                &mut cursor,
                ProblemType::HigherOrder
            ),
            NOT_IN_TERM
        );
        assert_eq!(cursor, 4);
    }

    #[test]
    fn fixed_fingerprint_constructors_match_c_position_sets() {
        let root = sample_term();

        assert_eq!(index_fp0_create(&root).raw(), &[1]);
        assert_eq!(index_fp1_create(&root).raw(), &[2, 20]);
        assert_eq!(index_fp2_create(&root).raw(), &[3, 20, 30]);
        assert_eq!(index_fp3d_create(&root).raw(), &[4, 20, 30, 10]);
        assert_eq!(index_fp3w_create(&root).raw(), &[4, 20, 30, ANY_VAR]);
        assert_eq!(
            index_fp4d_create(&root).raw(),
            &[5, 20, 30, 10, NOT_IN_TERM]
        );
        assert_eq!(
            index_fp4w_create(&root).raw(),
            &[5, 20, 30, ANY_VAR, NOT_IN_TERM]
        );
        assert_eq!(index_fp4m_create(&root).raw(), &[5, 20, 30, ANY_VAR, 10]);
        assert_eq!(
            index_fp5m_create(&root).raw(),
            &[6, 20, 30, ANY_VAR, NOT_IN_TERM, 10]
        );
        assert_eq!(
            index_fp6m_create(&root).raw(),
            &[7, 20, 30, ANY_VAR, NOT_IN_TERM, 10, NOT_IN_TERM]
        );
    }

    #[test]
    fn wide_fingerprint_constructors_match_c_position_sets() {
        let root = sample_term();

        assert_eq!(
            index_fp7_create(&root).raw(),
            &[8, 20, 30, ANY_VAR, 10, NOT_IN_TERM, BELOW_VAR, BELOW_VAR]
        );
        assert_eq!(
            index_fp7m_create(&root).raw(),
            &[
                8,
                20,
                30,
                ANY_VAR,
                NOT_IN_TERM,
                NOT_IN_TERM,
                10,
                NOT_IN_TERM
            ]
        );
        assert_eq!(
            index_fp4x2_2_create(&root).raw(),
            &[
                17,
                20,
                30,
                ANY_VAR,
                NOT_IN_TERM,
                NOT_IN_TERM,
                10,
                NOT_IN_TERM,
                NOT_IN_TERM,
                BELOW_VAR,
                BELOW_VAR,
                BELOW_VAR,
                NOT_IN_TERM,
                NOT_IN_TERM,
                NOT_IN_TERM,
                NOT_IN_TERM,
                BELOW_VAR
            ]
        );
    }

    #[test]
    fn flexible_and_discrimination_tree_fingerprints_match_c_shapes() {
        let root = sample_term();

        assert_eq!(
            index_fp_flex_create(&root, &[-1, 0, -1, 0, 0, -1, -2], 3).raw(),
            &[4, 20, 30, 10]
        );
        assert_eq!(index_fp3d_flex_create(&root), index_fp3d_create(&root));
        assert_eq!(index_dt_create(&root).raw(), &[5, 20, 30, 10, ANY_VAR]);
    }

    #[test]
    fn discrimination_tree_fingerprint_skips_phony_app_symbol() {
        let app = term(SIG_PHONY_APP_CODE, &[var(-2), leaf(10)]);
        assert_eq!(index_dt_create(&app).raw(), &[3, ANY_VAR, 10]);
    }

    #[test]
    fn fp_fp_constructor_uses_process_static_function_and_predicate_representatives() {
        reset_fp_fp_representatives_for_tests();
        let predicate_one = leaf(100);
        let predicate_two = leaf(200);
        predicate_one.set_prop(TP_PRED_POS);
        predicate_two.set_prop(TP_PRED_POS);

        assert_eq!(index_fp_fp_create(&predicate_one).raw(), &[2, 100]);
        assert_eq!(index_fp_fp_create(&predicate_two).raw(), &[2, 100]);
        assert_eq!(index_fp_fp_create(&leaf(300)).raw(), &[2, 300]);
        assert_eq!(index_fp_fp_create(&leaf(400)).raw(), &[2, 300]);
    }

    #[test]
    fn lookup_maps_names_to_functions_and_null_entries_to_none() {
        let root = sample_term();
        let fp3d = get_fp_index_function("FP3D").unwrap();
        let npdt = get_fp_index_function("NPDT").unwrap();

        assert_eq!(fp3d(&root), index_fp3d_create(&root));
        assert_eq!(npdt(&root), index_dt_create(&root));
        assert!(get_fp_index_function("NoIndex").is_none());
        assert!(get_fp_index_function("missing").is_none());
    }

    #[test]
    fn print_string_matches_angle_bracket_c_output() {
        let root = sample_term();

        assert_eq!(index_fp0_create(&root).print_string(), "<>");
        assert_eq!(index_fp3d_create(&root).print_string(), "<20,30,10>");
        assert_eq!(index_fp3d_create(&root).samples(), &[20, 30, 10]);
    }
}
