use std::ffi::{c_char, c_int, c_void, CStr};
use std::fmt;
use std::path::{Path, PathBuf};
use std::ptr::NonNull;

// Runtime PicoSAT loading is the allowed external DLL/shared-library boundary.
// The public wrapper owns solver/library handles and keeps raw ABI calls local.

const PICOSAT_SATISFIABLE: c_int = 10;
const PICOSAT_UNSATISFIABLE: c_int = 20;

type PicoSatInit = unsafe extern "C" fn() -> *mut PicoSatOpaque;
type PicoSatReset = unsafe extern "C" fn(*mut PicoSatOpaque);
type PicoSatEnableTraceGeneration = unsafe extern "C" fn(*mut PicoSatOpaque) -> c_int;
type PicoSatAddLits = unsafe extern "C" fn(*mut PicoSatOpaque, *mut c_int) -> c_int;
type PicoSatAddedOriginalClauses = unsafe extern "C" fn(*mut PicoSatOpaque) -> c_int;
type PicoSatSat = unsafe extern "C" fn(*mut PicoSatOpaque, c_int) -> c_int;
type PicoSatCoreClause = unsafe extern "C" fn(*mut PicoSatOpaque, c_int) -> c_int;
type PicoSatVersion = unsafe extern "C" fn() -> *const c_char;

#[repr(C)]
struct PicoSatOpaque {
    _private: [u8; 0],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PicoSatSolveResult {
    Satisfiable,
    Unsatisfiable,
    GaveUp,
}

impl PicoSatSolveResult {
    #[must_use]
    pub const fn from_raw(result: c_int) -> Self {
        match result {
            PICOSAT_SATISFIABLE => Self::Satisfiable,
            PICOSAT_UNSATISFIABLE => Self::Unsatisfiable,
            _ => Self::GaveUp,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PicoSatError {
    LoadLibrary {
        path: PathBuf,
        message: String,
    },
    LoadSymbol {
        path: PathBuf,
        symbol: &'static str,
        message: String,
    },
    InitFailed,
    ClauseContainsZero,
    ClauseCountOutOfRange {
        count: usize,
    },
    AddedClauseCountMismatch {
        expected: c_int,
        actual: c_int,
    },
}

impl fmt::Display for PicoSatError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LoadLibrary { path, message } => {
                write!(
                    formatter,
                    "could not load PicoSAT library {}: {message}",
                    path.display()
                )
            }
            Self::LoadSymbol {
                path,
                symbol,
                message,
            } => {
                write!(
                    formatter,
                    "could not load PicoSAT symbol {symbol} from {}: {message}",
                    path.display()
                )
            }
            Self::InitFailed => formatter.write_str("picosat_init returned NULL"),
            Self::ClauseContainsZero => {
                formatter.write_str("PicoSAT clauses cannot contain the zero sentinel")
            }
            Self::ClauseCountOutOfRange { count } => {
                write!(
                    formatter,
                    "PicoSAT clause count {count} exceeds C int range"
                )
            }
            Self::AddedClauseCountMismatch { expected, actual } => {
                write!(
                    formatter,
                    "PicoSAT reported {actual} original clauses after exporting {expected}"
                )
            }
        }
    }
}

impl std::error::Error for PicoSatError {}

macro_rules! load_symbol {
    ($library:expr, $name:literal, $symbol_type:ty) => {{
        let raw = $library.symbol(concat!($name, "\0").as_bytes(), $name)?;
        // SAFETY: the symbol name is resolved from a PicoSAT shared library and
        // cast to the exact function-pointer type declared by picosat.h. The
        // DynamicLibrary is stored in PicoSat so the loaded code outlives every
        // copied function pointer.
        unsafe { std::mem::transmute::<*mut c_void, $symbol_type>(raw) }
    }};
}

#[derive(Clone, Copy)]
struct PicoSatApi {
    init: PicoSatInit,
    reset: PicoSatReset,
    enable_trace_generation: PicoSatEnableTraceGeneration,
    add_lits: PicoSatAddLits,
    added_original_clauses: PicoSatAddedOriginalClauses,
    sat: PicoSatSat,
    coreclause: PicoSatCoreClause,
    version: PicoSatVersion,
}

impl PicoSatApi {
    fn load(library: &DynamicLibrary) -> Result<Self, PicoSatError> {
        Ok(Self {
            init: load_symbol!(library, "picosat_init", PicoSatInit),
            reset: load_symbol!(library, "picosat_reset", PicoSatReset),
            enable_trace_generation: load_symbol!(
                library,
                "picosat_enable_trace_generation",
                PicoSatEnableTraceGeneration
            ),
            add_lits: load_symbol!(library, "picosat_add_lits", PicoSatAddLits),
            added_original_clauses: load_symbol!(
                library,
                "picosat_added_original_clauses",
                PicoSatAddedOriginalClauses
            ),
            sat: load_symbol!(library, "picosat_sat", PicoSatSat),
            coreclause: load_symbol!(library, "picosat_coreclause", PicoSatCoreClause),
            version: load_symbol!(library, "picosat_version", PicoSatVersion),
        })
    }
}

pub struct PicoSat {
    solver: NonNull<PicoSatOpaque>,
    api: PicoSatApi,
    _library: DynamicLibrary,
}

impl PicoSat {
    /// Opens a `PicoSAT` shared library and creates a trace-enabled solver.
    ///
    /// The library must export the reentrant `PicoSAT` API used by E 3.2.0:
    /// `picosat_init`, `picosat_reset`, `picosat_enable_trace_generation`,
    /// `picosat_add_lits`, `picosat_added_original_clauses`, `picosat_sat`,
    /// `picosat_coreclause`, and `picosat_version`.
    pub fn open(path: &Path) -> Result<Self, PicoSatError> {
        let library = DynamicLibrary::open(path)?;
        let api = PicoSatApi::load(&library)?;
        let solver = init_trace_enabled_solver(api)?;
        Ok(Self {
            solver,
            api,
            _library: library,
        })
    }

    pub fn reset(&mut self) -> Result<(), PicoSatError> {
        let replacement = init_trace_enabled_solver(self.api)?;
        let previous = std::mem::replace(&mut self.solver, replacement);
        // SAFETY: previous is the unique owned pointer returned by picosat_init
        // and has been replaced in self, so this call releases it exactly once.
        unsafe { (self.api.reset)(previous.as_ptr()) };
        Ok(())
    }

    #[must_use]
    pub fn version(&self) -> Option<String> {
        // SAFETY: picosat_version returns a process-static C string pointer or
        // NULL. The returned pointer is copied into an owned Rust String before
        // this method returns.
        let version = unsafe { (self.api.version)() };
        if version.is_null() {
            None
        } else {
            // SAFETY: non-null pointer comes from PicoSAT and is expected to be
            // NUL-terminated by picosat.h's const char* contract.
            unsafe { CStr::from_ptr(version) }
                .to_str()
                .ok()
                .map(str::to_owned)
        }
    }

    pub fn add_clause(&mut self, clause: &[i32]) -> Result<(), PicoSatError> {
        let mut literals = sentinel_terminated_clause(clause)?;
        // SAFETY: literals is a mutable, NUL-terminated c_int buffer that lives
        // for the whole picosat_add_lits call. solver is owned by self.
        let _ = unsafe { (self.api.add_lits)(self.solver.as_ptr(), literals.as_mut_ptr()) };
        Ok(())
    }

    pub fn add_clauses(&mut self, clauses: &[Vec<i32>]) -> Result<(), PicoSatError> {
        for clause in clauses {
            self.add_clause(clause)?;
        }
        self.validate_added_original_clause_count(clauses.len())
    }

    #[must_use]
    pub fn added_original_clauses(&self) -> c_int {
        // SAFETY: solver is owned by self and remains valid until Drop calls
        // picosat_reset.
        unsafe { (self.api.added_original_clauses)(self.solver.as_ptr()) }
    }

    pub fn validate_added_original_clause_count(
        &self,
        expected: usize,
    ) -> Result<(), PicoSatError> {
        let expected = c_int::try_from(expected)
            .map_err(|_| PicoSatError::ClauseCountOutOfRange { count: expected })?;
        let actual = self.added_original_clauses();
        if actual == expected {
            Ok(())
        } else {
            Err(PicoSatError::AddedClauseCountMismatch { expected, actual })
        }
    }

    #[must_use]
    pub fn solve(&mut self, decision_limit: i32) -> PicoSatSolveResult {
        // SAFETY: solver is owned by self and all clauses were added through
        // this wrapper's sentinel-checked add_clause path.
        PicoSatSolveResult::from_raw(unsafe {
            (self.api.sat)(self.solver.as_ptr(), decision_limit)
        })
    }

    pub fn core_indices(&self, exported_len: usize) -> Result<Vec<usize>, PicoSatError> {
        let mut core = Vec::new();
        for index in 0..exported_len {
            let c_index =
                c_int::try_from(index).map_err(|_| PicoSatError::ClauseCountOutOfRange {
                    count: exported_len,
                })?;
            // SAFETY: solver is owned by self, and c_index is in the exported
            // original-clause range supplied by the caller after a completed
            // UNSAT solve with trace generation enabled.
            if unsafe { (self.api.coreclause)(self.solver.as_ptr(), c_index) } != 0 {
                core.push(index);
            }
        }
        Ok(core)
    }
}

fn init_trace_enabled_solver(api: PicoSatApi) -> Result<NonNull<PicoSatOpaque>, PicoSatError> {
    // SAFETY: picosat_init is a constructor loaded from picosat.h's ABI.
    // It takes no Rust pointers and returns an owned solver pointer or NULL on
    // failure.
    let solver = NonNull::new(unsafe { (api.init)() }).ok_or(PicoSatError::InitFailed)?;
    // SAFETY: solver is the non-null pointer returned by picosat_init, and E
    // calls trace-generation setup immediately after initialization.
    let _ = unsafe { (api.enable_trace_generation)(solver.as_ptr()) };
    Ok(solver)
}

impl Drop for PicoSat {
    fn drop(&mut self) {
        // SAFETY: solver is the unique owned pointer returned by picosat_init
        // and is reset exactly once here, before the DynamicLibrary field is
        // dropped.
        unsafe { (self.api.reset)(self.solver.as_ptr()) };
    }
}

fn sentinel_terminated_clause(clause: &[i32]) -> Result<Vec<c_int>, PicoSatError> {
    if clause.contains(&0) {
        return Err(PicoSatError::ClauseContainsZero);
    }
    let mut literals = Vec::with_capacity(clause.len().saturating_add(1));
    literals.extend(clause.iter().copied());
    literals.push(0);
    Ok(literals)
}

struct DynamicLibrary {
    handle: platform::LibraryHandle,
    path: PathBuf,
}

impl DynamicLibrary {
    fn open(path: &Path) -> Result<Self, PicoSatError> {
        let handle = platform::open(path).map_err(|message| PicoSatError::LoadLibrary {
            path: path.to_path_buf(),
            message,
        })?;
        Ok(Self {
            handle,
            path: path.to_path_buf(),
        })
    }

    fn symbol(
        &self,
        nul_terminated_name: &'static [u8],
        display_name: &'static str,
    ) -> Result<*mut c_void, PicoSatError> {
        platform::symbol(&self.handle, nul_terminated_name).map_err(|message| {
            PicoSatError::LoadSymbol {
                path: self.path.clone(),
                symbol: display_name,
                message,
            }
        })
    }
}

impl Drop for DynamicLibrary {
    fn drop(&mut self) {
        platform::close(&mut self.handle);
    }
}

#[cfg(windows)]
mod platform {
    use super::c_void;
    use std::ffi::c_char;
    use std::io;
    use std::os::windows::ffi::OsStrExt;
    use std::path::Path;
    use std::ptr::NonNull;

    #[link(name = "kernel32")]
    extern "system" {
        fn LoadLibraryW(file_name: *const u16) -> *mut c_void;
        fn GetProcAddress(module: *mut c_void, proc_name: *const c_char) -> *mut c_void;
        fn FreeLibrary(module: *mut c_void) -> i32;
    }

    pub(super) struct LibraryHandle {
        handle: NonNull<c_void>,
    }

    pub(super) fn open(path: &Path) -> Result<LibraryHandle, String> {
        let mut wide_path = path.as_os_str().encode_wide().collect::<Vec<_>>();
        if wide_path.contains(&0) {
            return Err("library path contains an interior NUL".to_owned());
        }
        wide_path.push(0);
        // SAFETY: wide_path is NUL-terminated and lives for the whole
        // LoadLibraryW call. On success, the module handle is owned here and is
        // released by close.
        let handle = unsafe { LoadLibraryW(wide_path.as_ptr()) };
        NonNull::new(handle)
            .map(|handle| LibraryHandle { handle })
            .ok_or_else(|| io::Error::last_os_error().to_string())
    }

    pub(super) fn symbol(
        library: &LibraryHandle,
        nul_terminated_name: &'static [u8],
    ) -> Result<*mut c_void, String> {
        // SAFETY: library is a live module handle, and nul_terminated_name is a
        // static NUL-terminated ASCII symbol name.
        let symbol = unsafe {
            GetProcAddress(
                library.handle.as_ptr(),
                nul_terminated_name.as_ptr().cast::<c_char>(),
            )
        };
        if symbol.is_null() {
            Err(io::Error::last_os_error().to_string())
        } else {
            Ok(symbol)
        }
    }

    pub(super) fn close(library: &mut LibraryHandle) {
        // SAFETY: handle was returned by LoadLibraryW and is owned by
        // LibraryHandle until this close call.
        let _ = unsafe { FreeLibrary(library.handle.as_ptr()) };
    }
}

#[cfg(unix)]
mod platform {
    use super::{c_char, c_void};
    use std::ffi::{CStr, CString};
    use std::os::unix::ffi::OsStrExt;
    use std::path::Path;
    use std::ptr::NonNull;

    const RTLD_NOW: i32 = 2;
    const RTLD_LOCAL: i32 = 0;

    #[cfg(target_os = "linux")]
    #[link(name = "dl")]
    extern "C" {
        fn dlopen(filename: *const c_char, flags: i32) -> *mut c_void;
        fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
        fn dlclose(handle: *mut c_void) -> i32;
        fn dlerror() -> *const c_char;
    }

    #[cfg(not(target_os = "linux"))]
    extern "C" {
        fn dlopen(filename: *const c_char, flags: i32) -> *mut c_void;
        fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
        fn dlclose(handle: *mut c_void) -> i32;
        fn dlerror() -> *const c_char;
    }

    pub(super) struct LibraryHandle {
        handle: NonNull<c_void>,
    }

    pub(super) fn open(path: &Path) -> Result<LibraryHandle, String> {
        let path = CString::new(path.as_os_str().as_bytes())
            .map_err(|_| "library path contains an interior NUL".to_owned())?;
        // SAFETY: path is a NUL-terminated C string that lives for the whole
        // dlopen call. On success, the handle is owned here and released by
        // close.
        let handle = unsafe { dlopen(path.as_ptr(), RTLD_NOW | RTLD_LOCAL) };
        NonNull::new(handle)
            .map(|handle| LibraryHandle { handle })
            .ok_or_else(last_dl_error)
    }

    pub(super) fn symbol(
        library: &LibraryHandle,
        nul_terminated_name: &'static [u8],
    ) -> Result<*mut c_void, String> {
        // SAFETY: clearing dlerror before dlsym follows the POSIX lookup
        // contract and does not use any Rust-owned pointers.
        let _ = unsafe { dlerror() };
        // SAFETY: library is a live dlopen handle, and nul_terminated_name is a
        // static NUL-terminated ASCII symbol name.
        let symbol = unsafe { dlsym(library.handle.as_ptr(), nul_terminated_name.as_ptr().cast()) };
        if symbol.is_null() {
            Err(last_dl_error())
        } else {
            Ok(symbol)
        }
    }

    pub(super) fn close(library: &mut LibraryHandle) {
        // SAFETY: handle was returned by dlopen and is owned by LibraryHandle
        // until this close call.
        let _ = unsafe { dlclose(library.handle.as_ptr()) };
    }

    fn last_dl_error() -> String {
        // SAFETY: dlerror returns either NULL or a NUL-terminated diagnostic
        // string owned by the dynamic loader.
        let error = unsafe { dlerror() };
        if error.is_null() {
            "dynamic loader reported no detail".to_owned()
        } else {
            // SAFETY: non-null pointer comes from dlerror and is valid until
            // the next dynamic-loader call in this thread.
            unsafe { CStr::from_ptr(error) }
                .to_string_lossy()
                .into_owned()
        }
    }
}

#[cfg(not(any(unix, windows)))]
mod platform {
    use super::c_void;
    use std::path::Path;

    pub(super) struct LibraryHandle;

    pub(super) fn open(_path: &Path) -> Result<LibraryHandle, String> {
        Err("runtime dynamic loading is not implemented for this target".to_owned())
    }

    pub(super) fn symbol(
        _library: &LibraryHandle,
        _nul_terminated_name: &'static [u8],
    ) -> Result<*mut c_void, String> {
        Err("runtime dynamic loading is not implemented for this target".to_owned())
    }

    pub(super) fn close(_library: &mut LibraryHandle) {}
}

#[cfg(test)]
mod tests {
    use super::{
        sentinel_terminated_clause, PicoSat, PicoSatError, PicoSatSolveResult, PICOSAT_SATISFIABLE,
        PICOSAT_UNSATISFIABLE,
    };
    use std::env;
    use std::process;

    #[test]
    fn solve_result_mapping_matches_picosat_header() {
        assert_eq!(
            PicoSatSolveResult::from_raw(PICOSAT_SATISFIABLE),
            PicoSatSolveResult::Satisfiable
        );
        assert_eq!(
            PicoSatSolveResult::from_raw(PICOSAT_UNSATISFIABLE),
            PicoSatSolveResult::Unsatisfiable
        );
        assert_eq!(PicoSatSolveResult::from_raw(0), PicoSatSolveResult::GaveUp);
        assert_eq!(PicoSatSolveResult::from_raw(17), PicoSatSolveResult::GaveUp);
    }

    #[test]
    fn sentinel_clause_shape_matches_picosat_add_lits_contract() {
        assert_eq!(
            sentinel_terminated_clause(&[1, -2, 3]).unwrap(),
            [1, -2, 3, 0]
        );
        assert_eq!(
            sentinel_terminated_clause(&[1, 0, -2]).unwrap_err(),
            PicoSatError::ClauseContainsZero
        );
    }

    #[test]
    fn missing_library_reports_load_error() {
        let path =
            env::temp_dir().join(format!("missing-picosat-{}-{}.dll", process::id(), line!()));

        assert!(matches!(
            PicoSat::open(&path),
            Err(PicoSatError::LoadLibrary { .. })
        ));
    }
}
