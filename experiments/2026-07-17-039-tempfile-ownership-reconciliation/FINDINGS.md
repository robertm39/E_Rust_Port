# Temporary-file ownership reconciliation

## Objective

Resolve `E_Rust_Port-j76.2.98` by comparing C and Rust temporary-file ownership, creation, removal, and termination boundaries. The vendored C source remains unchanged.

## Compatibility map

| Behavior | C reference | Rust implementation |
| --- | --- | --- |
| Directory | `TMPDIR` when present, otherwise literal `/tmp` | `TMPDIR` when present; `/tmp` on Unix-like targets and the native directory on Windows |
| Name creation | `mkstemp("epr_XXXXXX")`, close descriptor | 1,024 atomic `create_new` attempts with `epr_` plus six base-36 characters |
| Unix permissions | `mkstemp` owner-only mode | explicit `OpenOptionsExt::mode(0o600)` |
| Registration | file-static content-keyed `StrTree` holding path pointers | process-global content-keyed `BTreeSet<PathBuf>` holding owned path copies |
| Explicit removal | unlink, then assert successful registry deletion | unlink, then content-keyed registry deletion; an asserting wrapper preserves the C precondition |
| Global cleanup | warn on failed unlink and always delete the registry entry | collect a warning on failed removal and always clear the registration |
| Termination | first SIGTERM/SIGINT cleans the global registry, then re-raises | first SIGTERM/SIGINT finalization cleans the registry once and returns an explicit termination outcome |

C's raw path pointer is only an ownership convenience for its string tree. Production callers retain and later pass the same path text, while the registry compares by content; no caller observes the pointer identity or tree topology. Rust's owned `PathBuf` registration therefore preserves the required lifetime without reproducing dangling-path hazards.

Both creation paths atomically reserve an empty file before exposing the name. The Rust retry loop preserves the security and lifecycle contract without an unsafe libc boundary, and the source-copy path leaves its registration live if opening or copying fails so termination cleanup still owns the file. Explicit removal retains the registration after unlink failure in both ports; global cleanup deliberately forgets failed paths after warning in both ports.

The Windows fallback to the native temporary directory was introduced for scheduled standard-input replay, which must operate without an MSYS `/tmp`. It is an intentional platform adaptation rather than a hidden ownership change. Whether drop-in compatibility should instead interpret C's literal `/tmp` on Windows remains assigned to `E_Rust_Port-j76.3.305` together with eventual scoped run-state ownership. Exact libc suffix selection and NUL-path diagnostics remain assigned to `E_Rust_Port-j76.4.887`; cleanup and assertion-policy reviews remain `E_Rust_Port-j76.4.888` and `.889`.

## Compatibility decision

The migrated initial-port item is complete. The safe implementation preserves the observable creation, ownership, removal, cleanup, and signal boundaries needed by current production callers. The remaining differences are narrower platform-policy questions already tracked after the compatibility milestone.

## Validation

- The temporary-file unit tests passed, including exact portable prefix/suffix shape, registration, source-copy, cleanup, and removal coverage.
- Focused signal, proof-checking, batch-runner, and scheduled-standard-input owner paths passed.
- The all-target, all-feature suite passed with one test thread: 4,240 library tests and all 7 integration tests. Two default-parallel attempts exposed unrelated pre-existing global-state flakes in the represented-formula parser and LTB variant-worker tests; each passed in isolation, and follow-up `E_Rust_Port-9yb` now tracks restoring a deterministic default-parallel gate.
- Strict all-target, all-feature pedantic Clippy passed.
- Formatting and C-source documentation gates passed; the vendored `eprover/` worktree remained clean.
