// Unsafe remains denied by default; standards-compliant DLL FFI modules may
// locally allow it with documented safety invariants.
#![deny(unsafe_code)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::module_name_repetitions)]

pub mod basics;
pub mod clauses;
pub mod control;
pub mod external;
pub mod heuristics;
pub mod inout;
pub mod learn;
pub mod orderings;
pub mod pcl2;
pub mod propositional;
pub mod prover;
pub mod simple_apps;
pub mod terms;

#[cfg(test)]
pub(crate) mod test_support {
    use std::sync::{Mutex, MutexGuard, OnceLock};

    pub(crate) fn global_state_lock() -> MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        match LOCK.get_or_init(|| Mutex::new(())).lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }
}
