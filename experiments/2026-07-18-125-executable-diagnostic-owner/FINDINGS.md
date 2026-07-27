# Executable diagnostic program-name ownership

## Status

Completed for Bead `E_Rust_Port-j76.2.13`. Every Rust executable initializes
the C-shaped global program-name owner and reports returned top-level fatal
diagnostics through one shared path. The vendored C checkout remains unchanged.

## Complete entry-point mapping

[`audit_entrypoints.py`](audit_entrypoints.py) retains all 26 Cargo binaries,
their unchanged C main source, and their initialization mode in
[`entrypoint-audit.json`](entrypoint-audit.json). The C split is exact:

- 22 entry points call `InitIO(NAME)` and Rust calls
  `init_error(PROGRAM_NAME)`; and
- `term2dag`, `ex_commandline`, `ekb_ginsert`, and `termprops` initialize from
  `argv[0]`, represented by Rust `init_error_from_invocation(PROGRAM_NAME)` with
  the canonical name as the theoretical empty-argument fallback.

Every binary then calls `report_fatal_diagnostic` for its top-level returned
error. No binary retains the former direct `writeln!(stderr,
"{PROGRAM_NAME}: ...")` path. The reporter reads the owned global name, writes
the one-line C shape, and returns the diagnostic exit status even if stderr
fails, preserving the previous fatal-wrapper behavior.

Explicit name-taking render and writer helpers remain useful below the process
boundary: they permit writer injection and concurrent tests without making
ordinary library functions terminate. The global is now owned exactly where C
closes over `ProgName`: executable initialization, verbose compatibility
wrappers, saved-errno rendering, and top-level fatal reporting.

## Exact unchanged-C comparison

[`capture_fatal_diagnostic.py`](capture_fatal_diagnostic.py) invokes the pinned
unchanged C prover and the rebuilt Rust release prover with the same unknown
option. Retained [`reference.json`](reference.json) proves both processes return
usage status 5, write nothing to stdout, and emit exactly:

```text
eprover: Unknown Option: --definitely-invalid-option (Use -h for a list of valid options)
```

Permanent Rust runtime tests invoke both a canonical-name entry (`eprover`) and
an invocation-name entry (`termprops`). On Unix the latter also runs through an
aliased image path; Windows command construction normalizes that child argument
to the canonical executable stem, which the test records as the platform's
actual `argv[0]` behavior.

## Reproduction

```powershell
C:\Users\rober\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe `
  experiments\2026-07-18-125-executable-diagnostic-owner\audit_entrypoints.py `
  --output target\entrypoint-owner-audit-check.json `
  --expected experiments\2026-07-18-125-executable-diagnostic-owner\entrypoint-audit.json

C:\Users\rober\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe `
  experiments\2026-07-18-125-executable-diagnostic-owner\capture_fatal_diagnostic.py `
  --output target\fatal-diagnostic-check.json `
  --expected experiments\2026-07-18-125-executable-diagnostic-owner\reference.json
```
