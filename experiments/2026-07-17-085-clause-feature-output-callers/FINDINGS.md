# Clause-feature output call sites

## Status

Completed for Bead `E_Rust_Port-j76.2.52`. Every production caller of the C
`che_clausefeatures` print helpers has a represented Rust route, and the live
LOP, TPTP, TSTP, and PCL output families are byte-exact. The vendored C
checkout remained unchanged.

## Call-site audit

[`audit_call_sites.py`](audit_call_sites.py) pins the complete production call
graph visible in this checkout:

- `ClauseInfoPrint` is called only by `ClauseLinePrint`;
- `ClauseLinePrint` is called three times by the positive-unit,
  negative-unit, and non-unit filters in `che_clausesetfeatures`;
- those filtered printers are reached by `ProofStatePrintSelective`, whose
  executable caller is `eprover`;
- `ClausePropInfoPrint` is called only by `PCLProtPropDataPrint`, whose
  executable caller is `epclanalyse`.

Rust keeps the same two production owners. `eprover::write_saturated_output`
passes the selected `IoFormat`, problem type, and equation-print options into
`proof_state_print_selective_string`; its filtered sets use the explicit
format-aware clause-line renderer. `pcl2::propanalysis` calls the fixed PCL
property-info renderer before `epclanalyse` writes the report. The default LOP
and caller-rendered helpers remain reusable low-level APIs, not hidden global
dependencies in either executable path.

## Executable evidence

[`compare_callers.py`](compare_callers.py) compares complete stdout, stderr,
and exit status for four cases:

- `eprover` saturated output in default LOP form;
- the same selected clause and `ClauseInfoPrint` suffix under TPTP output;
- the same selected clause and suffix under TSTP output; and
- `epclanalyse` property reporting through PCL clause rendering.

The saturated cases use `--print-sat-info`, so the numeric info suffix is
exercised together with each clause renderer. All four cases are byte-exact
between unchanged C commit `17026b1bfe61aaf223cfaae54947c8d2679c31a0`
and optimized Rust. Their stdout sizes are 477 bytes for LOP, 511 for TPTP,
494 for TSTP, and 1,240 for the PCL property report.

The first PCL fixture had no negative clause and therefore reached the already
documented platform-dependent `-nan`/`NaN` average spelling. The retained
fixture includes positive, negative, and mixed clauses so this comparison is
fully strict and isolates clause property rendering rather than normalizing an
unrelated floating-point boundary.

[`reference.json`](reference.json) retains the 4/4 exact result and has
SHA-256 `2BF2C3800E5426E2865CDFC9B74CD95976C2C55187691212FD1B5AADEA26DFD7`.

## Compatibility decision

No unintegrated executable caller remains in this C checkout. Adding more
global-format adapters would duplicate already explicit Rust ownership rather
than complete the port. Future executable output paths must continue to pass
their format context explicitly; the static audit will fail if the current
call graph silently changes.

## Reproduction

```powershell
C:\Users\rober\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe `
  experiments\2026-07-17-085-clause-feature-output-callers\audit_call_sites.py `
  --repo .

cargo build --locked --release --bin eprover --bin epclanalyse --all-features

C:\Users\rober\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe `
  experiments\2026-07-17-085-clause-feature-output-callers\compare_callers.py `
  --c-eprover /home/rober/.cache/e-rust-port/src/17026b1bfe61aaf223cfaae54947c8d2679c31a0-ho-20260717b/PROVER/eprover-ho `
  --c-epclanalyse /home/rober/.cache/e-rust-port/src/17026b1bfe61aaf223cfaae54947c8d2679c31a0-ho-20260717b/PROVER/epclanalyse `
  --rust-eprover target\release\eprover.exe `
  --rust-epclanalyse target\release\epclanalyse.exe `
  --output target\clause-feature-callers-reference.json `
  --expected experiments\2026-07-17-085-clause-feature-output-callers\reference.json
```
