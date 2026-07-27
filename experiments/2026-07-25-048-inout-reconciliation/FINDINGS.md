# Detailed INOUT reconciliation

## Status

Accepted for the 19 remaining open `inout` records under Beads
`E_Rust_Port-j76.4`. Direct review found no missing production I/O behavior.
The records describe intentional compatibility quirks, safe Rust ownership
substitutions for C stream/file lifetimes, platform adaptations, or already
tested low-level interfaces. No Rust or C source changed.

## Review decisions

| Record | Decision |
|---|---|
| 850 | Keep the deterministic Rust parser for C's production `strtod` surface. Decimal, named infinity/NaN, C99 hexadecimal floats, full-consumption checks, and range failures are pinned; locale-specific decimal syntax is not a portable executable contract. |
| 851 | Keep typed option slices with optional short/long names. Slice length replaces C's zero-code sentinel without changing any table-visible option, while `Option` prevents null-name dereferences. |
| 854 | Preserve `FileExists` as a readability probe by opening the path. Metadata-only existence would change the observable contract. |
| 855 | Retain safe owned-file drop for `InputClose`. Stable Rust has no safe close API that can report a consumed `File`'s close-time error, and eager input reads already report observable read/open failures. |
| 856 | Preserve the inherited `FileVarsGetBool` `strcmp` bug exactly: `"true"` maps false, while `"false"` and other stored values map true. |
| 859 | Preserve `TPTP_dir` persistence when a later `init_io` call sees no environment variable, and clear it only at `exit_io`. |
| 860 | Keep the temporary duplicate program-name state while compatibility accessors expose both surfaces. Every executable initializes the shared diagnostic owner and uses the common fatal reporter, so output has one effective source. |
| 863 | Preserve TCP header/payload progress text, empty-payload-as-closed behavior, and embedded-NUL truncation. Rust deliberately limits truncation to initialized bytes rather than exposing C's short-read uninitialized tail. |
| 864 | Keep explicit `SO_REUSEADDR` before bind on Linux and Windows, with the standard-library fallback on other targets. Both supported deployment targets and real loopback ownership are covered. |
| 866 | Retain the process-global raw output descriptor as a borrowed compatibility view of the owned writer. Supported Unix/MSVC/MinGW paths share target and file position; unrelated ABIs return an honest unsupported sentinel. |
| 867 | Preserve flush/error reporting and stdout non-closure through the typed output owner. Rust drops file owners after a successful flush without manufacturing a second unsafe `fclose` layer. |
| 868 | Keep the explicit automatic-include constructor for regression/compatibility callers. Production formula owners deliberately use explicit recursive include parsing, matching the visible C ownership surface. |
| 869 | Keep ordinary scanner lookahead bounded and the separate C-modulo accessor for the aliasing macro contract. Production parsing does not need accidental out-of-range aliasing. |
| 873 | Keep token rendering split into a string form and generic `io::Write` helper. The bytes match C without importing `FILE*` ownership. |
| 875 | Preserve the impossible-name sentinel for `include(file,[])`; selector-stack regressions prove it selects no entries without missing-name errors. |
| 878 | Preserve direct hard-timeout descriptor output, the doubled default comment prefix, SZS status, diagnostic, exit 8, and direct-before-pending-buffer ordering. |
| 879 | Keep Windows CPU limits cooperative. A Job Object quota kills with `STATUS_QUOTA_EXCEEDED` before E can emit its required timeout surface; Linux retains native `SIGXCPU`. |
| 886 | Retain eager owned stream bytes. The source file is closed during `fs::read`, so C's later `DestroyStream` close diagnostic has no corresponding live file handle; open/read errors remain reported. |
| 887 | Keep atomic `create_new` retries with the `epr_` plus six-character shape and Unix `0o600` mode. Exact libc suffix distribution and NUL-path diagnostics are not required for uniqueness, security, or lifecycle compatibility. |

## Evidence

The low-level Rust regressions cover every decision:

- C-shaped decimal/named/hex float parsing and range failures;
- option tables with optional names and all executable option surfaces;
- readability-open semantics, input close ownership, and the Boolean bug;
- persistent `TPTP` state plus synchronized diagnostic program names;
- partial TCP header/payload reads, tracing, NUL truncation, empty payloads,
  server reuse/bind/listen order, and real loopback traffic;
- raw and owned writes to the same global output file;
- automatic and explicit include handling, empty selectors, bounded and modulo
  lookahead, and token rendering;
- deterministic timeout outcomes plus native Linux descriptor delivery;
- eager stream-byte sharing without an additional allocation; and
- temporary-file creation, registration, cleanup, removal, and Unix mode.

Retained ownership experiments cover the security and platform judgments:
temporary files (experiment 340), signal delivery (341), sockets (342), raw
global output descriptors (344), executable diagnostic ownership (125), and
explicit include policy (126). The latest exact candidate passes 4,429 tests,
all 50 main-prover cases, and all 216 support-tool cases with zero unexpected
differences.

## Audit

[`audit_inout_reconciliation.py`](audit_inout_reconciliation.py) pins the exact
19 migrated identities and content hashes, checks twelve grouped
source/implementation/evidence contracts, and digests the 20 unchanged C
units, 11 Rust owners, status ledger, retained ownership findings, and current
validation reference. The audit is independent of issue status, so it remains
reproducible after closure.

## Validation

The source audit, Python syntax check, C-source documentation coverage,
Change Later wording, local links, manual-regeneration preservation, and
`git diff --check` pass. The unchanged implementation is covered by the exact
Experiment 046 lifecycle:

- Rustfmt and strict all-target/all-feature pedantic Clippy pass;
- 4,418 library plus 11 integration tests pass, 4,429 total;
- native release and compile-only Windows GNU x64 all-target/all-feature
  builds pass; and
- 50 main plus 216 support-tool comparisons have zero unexpected differences.

No Rust or C toolchain ran on the local Windows host. The vendored C checkout
is clean.

Reproduce the source audit locally:

```powershell
.\.venv\Scripts\python.exe experiments/2026-07-25-048-inout-reconciliation/audit_inout_reconciliation.py `
  --repo . `
  --expected experiments/2026-07-25-048-inout-reconciliation/audit-reference.json
```
