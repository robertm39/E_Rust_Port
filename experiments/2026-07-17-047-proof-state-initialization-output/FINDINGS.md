# Proof-state initialization and selected-output propagation

## Status

Completed for Bead `E_Rust_Port-j76.2.90`. Rust now matches the preserved C
reference for proof-state initialization, selected-clause progress, and
CNF-only clause sections across both file and stdin input.

## Question

Does the supported executable emit C's OutputLevel 1 initialization banner at
the correct point, and does an auto-detected TSTP input format continue to
govern every later clause-output boundary rather than only the final CNF-only
sections?

## C source behavior

The unchanged reference is commit
`17026b1bfe61aaf223cfaae54947c8d2679c31a0`.

`ProofStateInit` writes `% Initializing proof state` through `GlobalOut` before
watchlist processing, axiom copying, or the AC scan. `document_processing`
later calls `ClausePrint` for OutputLevel 1 selected-clause progress, and
`ClausePrint` dispatches through the process-global `OutputFormat`. Therefore
auto-detected TSTP input affects both proof-search progress and CNF-only
saturated sections. Supplying `--tstp-in` explicitly does not itself select a
TSTP output format; the mutation is specific to automatic input detection.

## Discovered mismatch and fix

Rust already emitted the initialization banner at the correct point and used
the detected output format for CNF-only sections. The expanded matrix exposed
one adjacent mismatch: output-only saturation rendered selected clauses with
the LOP printer unconditionally, so auto-detected TSTP proof search printed
`human(socrates) <- .` where C printed
`cnf(i_0_1, plain, (human(socrates))).`.

The saturation output context now carries the runtime `IoFormat` selected by
parsing. OutputLevel 1 `document_processing` uses the shared explicit
`ClausePrint` dispatcher with that format and the current problem type. Plain
lower-level wrappers retain their existing LOP default, while executable main
saturation and presaturation pass `config.output_format` after automatic
format side effects have been applied.

## Fixture and live result

[`socrates.p`](socrates.p) contains two first-order axioms and one conjecture.
[`compare_initialization.py`](compare_initialization.py) runs four cases against
the Windows Rust release and cached WSL C reference:

- proof search from a named file;
- proof search from stdin;
- CNF-only output from a named file; and
- CNF-only output from stdin.

All four cases now match exactly in exit code, stdout, and stderr. Every case
places initialization before AC scanning; proof cases then use TSTP
`%cnf(...)` selected-clause progress and reach `Theorem`, while CNF-only cases
retain TSTP clause sections and reach `Unknown`. No case falls back to LOP.
[`results-summary.json`](results-summary.json) preserves the complete
transcripts and individual ordering/format checks.

## Permanent regressions

Two executable tests pin the complete stable Rust output for the fixture. One
covers proof search, including all selected clauses and the final standalone
comment marker; the other covers the complete initialized CNF-only section
layout. Together they prevent either initialization chronology or detected
output-format propagation from regressing independently.

## Reproduction

```powershell
& 'C:\Users\rober\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe' `
  experiments\2026-07-17-047-proof-state-initialization-output\compare_initialization.py `
  --rust-exe target\release\eprover.exe `
  --c-exe /home/rober/.cache/e-rust-port/bin/17026b1bfe61aaf223cfaae54947c8d2679c31a0/fol/eprover `
  --distro Ubuntu-24.04 `
  --output experiments\2026-07-17-047-proof-state-initialization-output\results-summary.json
```

## Validation

- live four-case C/Rust executable comparison: exact
- initialization/output-format assertions: all passed
- focused permanent executable regressions: passed
- full serial suite: 4,255 library tests plus all binary/integration targets
- strict all-target/all-feature pedantic Clippy: passed
- formatting and all four C-source documentation integrity gates: passed
