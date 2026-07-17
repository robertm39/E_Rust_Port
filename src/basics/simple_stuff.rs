use crate::basics::error::{Diagnostic, ErrorCode};
use std::cell::RefCell;
use std::cmp::Ordering;
use std::sync::{Mutex, MutexGuard, OnceLock};

pub const MAX_INDENT_SPACES: usize = 72;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i32)]
pub enum ProverResult {
    NoResult = 0,
    Theorem = 1,
    Unsatisfiable = 2,
    Satisfiable = 3,
    CounterSatisfiable = 4,
    Failure = 5,
    GaveUp = 6,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[repr(i32)]
pub enum ProblemType {
    #[default]
    NotInitialized = -1,
    FirstOrder = 0,
    HigherOrder = 1,
}

#[derive(Clone, Debug, PartialEq)]
pub struct WeightedObject<T> {
    pub weight: f64,
    pub object: T,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RandState {
    pub xstate: u32,
    pub ystate: u32,
    pub zstate: u32,
    pub cstate: u32,
}

impl Default for RandState {
    fn default() -> Self {
        Self {
            xstate: 123_456_789,
            ystate: 987_654_321,
            zstate: 43_219_876,
            cstate: 6_543_217,
        }
    }
}

impl RandState {
    #[must_use]
    pub const fn new(xstate: u32, ystate: u32, zstate: u32, cstate: u32) -> Self {
        Self {
            xstate,
            ystate,
            zstate,
            cstate,
        }
    }

    pub fn seed(&mut self, seed1: i32, seed2: i32, seed3: i32) {
        self.xstate = seed1.cast_unsigned();
        self.ystate = seed2.cast_unsigned();
        self.zstate = seed3.cast_unsigned();
    }

    #[must_use]
    pub fn next_u32(&mut self) -> u32 {
        self.xstate = 314_527_869_u32
            .wrapping_mul(self.xstate)
            .wrapping_add(1_234_567);
        self.ystate ^= self.ystate.wrapping_shl(5);
        self.ystate ^= self.ystate.wrapping_shr(7);
        self.ystate ^= self.ystate.wrapping_shl(22);

        let carry = 4_294_584_393_u64
            .wrapping_mul(u64::from(self.zstate))
            .wrapping_add(u64::from(self.cstate));
        let high_bits = carry >> 32;
        debug_assert!(u32::try_from(high_bits).is_ok());
        self.cstate = u32::try_from(high_bits).unwrap_or(u32::MAX);

        let low_bits = carry & u64::from(u32::MAX);
        debug_assert!(u32::try_from(low_bits).is_ok());
        self.zstate = u32::try_from(low_bits).unwrap_or(u32::MAX);

        self.xstate
            .wrapping_add(self.ystate)
            .wrapping_add(self.zstate)
    }

    #[must_use]
    pub fn next_f64(&mut self) -> f64 {
        f64::from(self.next_u32()) / 4_294_967_296.0
    }
}

thread_local! {
    static PROBLEM_TYPE: RefCell<ProblemType> =
        const { RefCell::new(ProblemType::NotInitialized) };
}
static GLOBAL_JKISS_STATE: OnceLock<Mutex<RandState>> = OnceLock::new();
static JKISS_SEED_SHADOW: OnceLock<Mutex<RandState>> = OnceLock::new();

fn global_jkiss_state() -> &'static Mutex<RandState> {
    GLOBAL_JKISS_STATE.get_or_init(|| Mutex::new(RandState::default()))
}

fn jkiss_seed_shadow() -> &'static Mutex<RandState> {
    JKISS_SEED_SHADOW.get_or_init(|| Mutex::new(RandState::default()))
}

fn lock_or_recover<T>(mutex: &'static Mutex<T>) -> MutexGuard<'static, T> {
    match mutex.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

#[must_use]
pub fn str_distance(left: &str, right: &str) -> usize {
    let mut distance = 0_usize;
    let mut left_iter = c_string_prefix(left).iter();
    let mut right_iter = c_string_prefix(right).iter();

    loop {
        match (left_iter.next(), right_iter.next()) {
            (Some(left_byte), Some(right_byte)) => {
                if left_byte != right_byte {
                    distance += 1;
                }
            }
            (Some(_), None) => {
                distance += 1 + left_iter.count();
                break;
            }
            (None, Some(_)) => {
                distance += 1 + right_iter.count();
                break;
            }
            (None, None) => break,
        }
    }
    distance
}

#[must_use]
pub fn weighted_object_compare<T>(left: &WeightedObject<T>, right: &WeightedObject<T>) -> Ordering {
    if left.weight < right.weight {
        Ordering::Less
    } else if left.weight > right.weight {
        Ordering::Greater
    } else {
        Ordering::Equal
    }
}

pub fn sort_weighted_objects<T>(objects: &mut [WeightedObject<T>]) {
    objects.sort_unstable_by(weighted_object_compare);
}

pub fn jkiss_seed(state: Option<&mut RandState>, seed1: i32, seed2: i32, seed3: i32) {
    if let Some(state) = state {
        state.seed(seed1, seed2, seed3);
    } else {
        lock_or_recover(jkiss_seed_shadow()).seed(seed1, seed2, seed3);
    }
}

#[must_use]
pub fn jkiss_rand(_state: Option<&mut RandState>) -> u32 {
    // The C implementation accepts a state pointer but mutates the file-static
    // x/y/z/c variables instead. Preserve that observable exported behavior.
    lock_or_recover(global_jkiss_state()).next_u32()
}

#[must_use]
pub fn jkiss_rand_double(state: Option<&mut RandState>) -> f64 {
    f64::from(jkiss_rand(state)) / 4_294_967_296.0
}

#[must_use]
pub fn indent_str(level: i32) -> String {
    let spaces = usize::try_from(level).unwrap_or(0).min(MAX_INDENT_SPACES);
    " ".repeat(spaces)
}

#[must_use]
pub fn string_starts_with(pattern: &str, prefix: &str) -> bool {
    c_string_prefix(pattern).starts_with(c_string_prefix(prefix))
}

#[must_use]
pub fn string_index(key: &str, list: &[&str]) -> Option<usize> {
    list.iter()
        .position(|candidate| c_strings_equal(candidate, key))
}

#[must_use]
pub fn string_index_c(key: &str, list: &[Option<&str>]) -> isize {
    list.iter()
        .take_while(|candidate| candidate.is_some())
        .position(|candidate| candidate.is_some_and(|candidate| c_strings_equal(candidate, key)))
        .map_or(-1, |index| isize::try_from(index).unwrap_or(isize::MAX))
}

#[must_use]
pub fn string_array_cardinality(array: &[Option<&str>]) -> usize {
    array
        .iter()
        .take_while(|candidate| candidate.is_some())
        .count()
}

#[must_use]
pub fn compute_gcd(mut left: i64, mut right: i64) -> i64 {
    if left < 0 || right < 0 {
        return 0;
    }
    loop {
        if left == 0 {
            return right;
        }
        if right == 0 {
            return left;
        }
        if left > right {
            left %= right;
        } else {
            right %= left;
        }
    }
}

fn c_string_prefix(text: &str) -> &[u8] {
    let bytes = text.as_bytes();
    let nul_pos = bytes
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(bytes.len());
    &bytes[..nul_pos]
}

fn c_strings_equal(left: &str, right: &str) -> bool {
    c_string_prefix(left) == c_string_prefix(right)
}

#[must_use]
pub fn problem_type() -> ProblemType {
    PROBLEM_TYPE.with(|current| *current.borrow())
}

pub fn set_problem_type(problem_type: ProblemType) -> Result<(), Diagnostic> {
    PROBLEM_TYPE.with(|current| {
        let mut current = current.borrow_mut();
        set_problem_type_value(&mut current, problem_type)
    })
}

fn set_problem_type_value(
    current: &mut ProblemType,
    problem_type: ProblemType,
) -> Result<(), Diagnostic> {
    if *current == ProblemType::NotInitialized || *current == problem_type {
        *current = problem_type;
        Ok(())
    } else {
        Err(problem_type_conflict_diagnostic())
    }
}

fn problem_type_conflict_diagnostic() -> Diagnostic {
    Diagnostic::new(
        ErrorCode::SYNTAX_ERROR,
        "Mixing of first order and higher order syntax is not allowed.",
    )
}

pub fn reset_problem_type() {
    PROBLEM_TYPE.with(|current| {
        *current.borrow_mut() = ProblemType::NotInitialized;
    });
}

#[cfg(test)]
fn reset_problem_type_for_tests() {
    reset_problem_type();
}

#[cfg(test)]
pub(crate) fn reset_jkiss_for_tests() {
    *lock_or_recover(global_jkiss_state()) = RandState::default();
    *lock_or_recover(jkiss_seed_shadow()) = RandState::default();
}

#[cfg(test)]
mod tests {
    use super::{
        compute_gcd, indent_str, jkiss_rand, jkiss_rand_double, jkiss_seed, problem_type,
        reset_jkiss_for_tests, reset_problem_type_for_tests, sort_weighted_objects, str_distance,
        string_array_cardinality, string_index, string_index_c, string_starts_with,
        weighted_object_compare, ProblemType, RandState, WeightedObject, MAX_INDENT_SPACES,
    };
    use crate::basics::error::ErrorCode;
    use crate::test_support::global_state_lock;
    use std::cmp::Ordering;
    use std::{
        sync::{Arc, Barrier},
        thread,
    };

    #[test]
    fn string_distance_counts_byte_differences_and_length_delta() {
        assert_eq!(str_distance("abc", "abc"), 0);
        assert_eq!(str_distance("abc", "axc"), 1);
        assert_eq!(str_distance("abc", "axcde"), 3);
        assert_eq!(str_distance("abcd", "ab"), 2);
        assert_eq!(str_distance("ab\0xxx", "ac\0yyy"), 1);
        assert_eq!(str_distance("ab\0xxx", "abcd"), 2);
    }

    #[test]
    fn weighted_object_comparison_and_sort_match_c_weight_ordering() {
        let mut objects = vec![
            WeightedObject {
                weight: 3.0,
                object: "three",
            },
            WeightedObject {
                weight: 1.0,
                object: "one",
            },
            WeightedObject {
                weight: 2.0,
                object: "two",
            },
        ];
        assert_eq!(
            weighted_object_compare(&objects[0], &objects[1]),
            Ordering::Greater
        );
        sort_weighted_objects(&mut objects);
        let ordered = objects
            .iter()
            .map(|object| object.object)
            .collect::<Vec<_>>();
        assert_eq!(ordered, vec!["one", "two", "three"]);

        let nan_left = WeightedObject {
            weight: f64::NAN,
            object: 1,
        };
        let nan_right = WeightedObject {
            weight: 10.0,
            object: 2,
        };
        assert_eq!(
            weighted_object_compare(&nan_left, &nan_right),
            Ordering::Equal
        );
    }

    #[test]
    fn rand_state_generates_known_jkiss_sequence_and_keeps_c_seed_shape() {
        let mut state = RandState::default();
        let values = (0..5).map(|_step| state.next_u32()).collect::<Vec<_>>();
        assert_eq!(
            values,
            vec![
                560_241_513,
                2_602_615_593,
                2_542_353_780,
                3_322_652_092,
                2_306_311_670
            ]
        );

        state.seed(-1, 2, 3);
        assert_eq!(state.xstate, u32::MAX);
        assert_eq!(state.ystate, 2);
        assert_eq!(state.zstate, 3);
        assert_eq!(state.cstate, 3_838_331_424);
    }

    #[test]
    fn global_jkiss_wrapper_preserves_c_static_state_quirk() {
        let _guard = global_state_lock();
        reset_jkiss_for_tests();

        let mut local = RandState::new(1, 2, 3, 4);
        jkiss_seed(Some(&mut local), 10, 20, 30);
        assert_eq!(local.xstate, 10);
        assert_eq!(local.ystate, 20);
        assert_eq!(local.zstate, 30);
        assert_eq!(local.cstate, 4);

        assert_eq!(jkiss_rand(Some(&mut local)), 560_241_513);
        let random_double = jkiss_rand_double(None);
        assert!(random_double >= 0.0);
        assert!(random_double < 1.0);
    }

    #[test]
    fn null_jkiss_seed_does_not_change_exported_random_sequence() {
        let _guard = global_state_lock();
        reset_jkiss_for_tests();

        jkiss_seed(None, 10, 20, 30);

        assert_eq!(jkiss_rand(None), 560_241_513);
    }

    #[test]
    fn indent_and_string_helpers_match_c_shapes() {
        assert_eq!(indent_str(-4), "");
        assert_eq!(indent_str(3), "   ");
        assert_eq!(indent_str(100).len(), MAX_INDENT_SPACES);

        assert!(string_starts_with("abcdef", "abc"));
        assert!(string_starts_with("abcdef", ""));
        assert!(!string_starts_with("abc", "abcd"));
        assert!(string_starts_with("abc\0hidden", "abc\0ignored"));
        assert!(!string_starts_with("ab\0hidden", "abc"));
        assert_eq!(string_index("beta", &["alpha", "beta", "gamma"]), Some(1));
        assert_eq!(
            string_index("beta\0key", &["alpha", "beta\0candidate", "beta"]),
            Some(1)
        );
        assert_eq!(string_index("delta", &["alpha", "beta"]), None);
    }

    #[test]
    fn c_shaped_null_terminated_string_helpers_stop_at_none() {
        let list = [
            Some("alpha"),
            Some("beta\0candidate"),
            None,
            Some("beta\0ignored"),
        ];
        assert_eq!(string_index_c("beta", &list), 1);
        assert_eq!(string_index_c("beta\0key", &list), 1);
        assert_eq!(string_index_c("ignored", &list), -1);
        assert_eq!(string_array_cardinality(&list), 2);
    }

    #[test]
    fn gcd_matches_positive_only_c_contract() {
        assert_eq!(compute_gcd(54, 24), 6);
        assert_eq!(compute_gcd(0, 9), 9);
        assert_eq!(compute_gcd(0, 0), 0);
        assert_eq!(compute_gcd(-1, 9), 0);
        assert_eq!(compute_gcd(9, -1), 0);
    }

    #[test]
    fn problem_type_setter_rejects_mixed_first_and_higher_order_syntax() {
        let _guard = global_state_lock();
        reset_problem_type_for_tests();
        assert_eq!(problem_type(), ProblemType::NotInitialized);
        assert!(super::set_problem_type(ProblemType::FirstOrder).is_ok());
        assert!(super::set_problem_type(ProblemType::FirstOrder).is_ok());

        let error = super::set_problem_type(ProblemType::HigherOrder).unwrap_err();
        assert_eq!(error.code(), ErrorCode::SYNTAX_ERROR);
        assert_eq!(
            error.message(),
            "Mixing of first order and higher order syntax is not allowed."
        );
        reset_problem_type_for_tests();
    }

    #[test]
    fn problem_type_is_isolated_between_concurrent_server_threads() {
        let barrier = Arc::new(Barrier::new(2));
        let threads: Vec<_> = [ProblemType::FirstOrder, ProblemType::HigherOrder]
            .into_iter()
            .map(|expected| {
                let barrier = Arc::clone(&barrier);
                thread::spawn(move || {
                    reset_problem_type_for_tests();
                    super::set_problem_type(expected).unwrap();
                    barrier.wait();
                    assert_eq!(problem_type(), expected);
                })
            })
            .collect();

        for thread in threads {
            thread.join().unwrap();
        }
    }
}
