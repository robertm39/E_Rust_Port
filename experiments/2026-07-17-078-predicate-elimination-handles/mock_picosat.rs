use std::os::raw::{c_char, c_int};

#[derive(Default)]
pub struct MockPicoSat {
    clauses: Vec<Vec<c_int>>,
    core: Vec<bool>,
}

#[no_mangle]
pub extern "C" fn picosat_init() -> *mut MockPicoSat {
    Box::into_raw(Box::new(MockPicoSat::default()))
}

#[no_mangle]
pub unsafe extern "C" fn picosat_reset(solver: *mut MockPicoSat) {
    if !solver.is_null() {
        drop(Box::from_raw(solver));
    }
}

#[no_mangle]
pub extern "C" fn picosat_enable_trace_generation(_solver: *mut MockPicoSat) -> c_int {
    1
}

#[no_mangle]
pub unsafe extern "C" fn picosat_add_lits(
    solver: *mut MockPicoSat,
    mut literals: *mut c_int,
) -> c_int {
    let solver = &mut *solver;
    let mut clause = Vec::new();
    while *literals != 0 {
        clause.push(*literals);
        literals = literals.add(1);
    }
    solver.clauses.push(clause);
    0
}

#[no_mangle]
pub unsafe extern "C" fn picosat_added_original_clauses(solver: *mut MockPicoSat) -> c_int {
    (*solver).clauses.len().try_into().unwrap_or(c_int::MAX)
}

#[no_mangle]
pub unsafe extern "C" fn picosat_sat(solver: *mut MockPicoSat, _decision_limit: c_int) -> c_int {
    let solver = &mut *solver;
    let max_variable = solver
        .clauses
        .iter()
        .flatten()
        .map(|literal| literal.unsigned_abs() as usize)
        .max()
        .unwrap_or(0);
    assert!(max_variable < usize::BITS as usize);
    let satisfiable = (0..(1_usize << max_variable)).any(|assignment| {
        solver.clauses.iter().all(|clause| {
            clause.iter().any(|literal| {
                let variable = literal.unsigned_abs() as usize - 1;
                let value = assignment & (1_usize << variable) != 0;
                (*literal > 0) == value
            })
        })
    });
    solver.core = vec![!satisfiable; solver.clauses.len()];
    if satisfiable {
        10
    } else {
        20
    }
}

#[no_mangle]
pub unsafe extern "C" fn picosat_coreclause(solver: *mut MockPicoSat, id: c_int) -> c_int {
    let solver = &*solver;
    usize::try_from(id)
        .ok()
        .and_then(|index| solver.core.get(index))
        .copied()
        .map(c_int::from)
        .unwrap_or(0)
}

#[no_mangle]
pub extern "C" fn picosat_version() -> *const c_char {
    c"predicate-elimination-mock".as_ptr()
}
