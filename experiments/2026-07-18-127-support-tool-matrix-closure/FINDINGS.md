# Support-tool matrix closure

## Status

Completed for Bead `E_Rust_Port-j76.2.11`. A live comparison against the
unchanged archived C commit now covers all 216 configured cases across all 25
support tools. The final report has zero unexpected mismatches and eight exact
declared differences. The vendored C checkout remains unchanged.

## Stale baseline

The migrated gap described a 75-case run with three residual differences. The
maintained matrix had since grown to 216 cases, while several expanded cases
had been accepted from source inspection when WSL and the archived binaries
were unavailable. The first complete rerun found 32 unexpected mismatches and
showed that some of those source-only expectations were wrong.

## Exact fixes

- File-backed scanner construction now follows C `InputOpen`: it performs the
  named-path `stat` boundary before reading. Tool adapters retain the C
  two-line `SysError` shape, and injected stdin scanners use `<stdin>` rather
  than `-` as the diagnostic source.
- Higher-order type-declaration formula targets use the encoded propositional
  truth formula that C prints as `($true)`, restoring exact THF wrapper output.
- Cross-platform comparison rewrites only the archived reference executable
  path to the tool name. This matches Windows `argv[0]` presentation for the
  four invocation-owned tools without weakening diagnostic text.
- Unified Rust higher-order cases use the archived HO tool build. Raw and
  merged THF classification are exact under that like-for-like comparison.
- Explicit-TSTP epatternize fixtures now contain modern TSTP records only.
  Old `input_formula` and `input_clause` records remain covered by the separate
  old-TPTP case; nested include selectors still span formula and clause owners.

## Declared compatibility decisions

The eight differences are field-exact, and an extra or missing field still
fails the matrix:

- two existing `checkproof` cases retain Rust's correct recognition of a real
  single-percent proof banner;
- C's FOL CSSCPA signature reaches a compiled-away reserved symbol code and
  renders one ordinary predicate as `$let`; Rust's unified runtime reserves
  the internal block and retains the ordinary predicate;
- two classify FOOL cases are accepted by Rust, while the HO C tool reports a
  type error for the valid typed `$let` term;
- C `ekb_ginsert` and C epatternize multi-file output abort in glibc with heap
  corruption, while Rust completes and writes the requested files; and
- C omits `TPHasBoolSubterm` from one shared typed `term2dag` root, while Rust
  retains the logically required property bit. Only the debug property integer
  differs.

## Retained evidence

The complete volatile report is
`.artifacts/e-compare/20260719-014142-789717-tools/tool-comparison.json`.
[`reference.json`](reference.json) retains its stable inventory, archived
commit, per-tool counts, and exact expected-difference fields.
[`audit_support_tool_matrix.py`](audit_support_tool_matrix.py) regenerates that
stable projection and rejects a changed inventory, archived commit, unexpected
mismatch, or changed expected-difference count.

## Reproduction

```powershell
cargo build --locked --release --bins --target-dir target\default-reference
.\e-interop.ps1 compare-tools -RustBinDir .\target\default-reference\release

C:\Users\rober\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe `
  experiments\2026-07-18-127-support-tool-matrix-closure\audit_support_tool_matrix.py `
  --report .artifacts\e-compare\20260719-014142-789717-tools\tool-comparison.json `
  --output target\support-tool-matrix-summary-check.json `
  --expected experiments\2026-07-18-127-support-tool-matrix-closure\reference.json
```
