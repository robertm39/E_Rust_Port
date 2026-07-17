# edpll Diagnostic Parity And DPLL Contract Reconciliation

## Question

Does the remaining `cpr_dpll` state-shell Bead represent missing drop-in solver
behavior, and does the current Rust `edpll` executable match C on the permanent
parser/state-construction matrix?

## DPLL Call-Path Decision

The source audit in
[`experiment 046`](../2026-07-16-046-edpll-no-solver-contract/FINDINGS.md)
remains definitive:

- `PROVER/edpll.c` parses clauses, allocates `DPLLState`, and immediately frees
  it without calling assignment, propagation, branching, or a solve loop;
- `deactivate_clauses` and `shorten_clauses` in `cpr_dpll.c` are empty stubs;
- `DPLLAssignVar` therefore records an assignment and returns false; and
- `DPLLRetractLastAss` is declared but has no C definition.

Rust already ports every implemented low-level state transition. Producing a
SAT/UNSAT result or inventing retraction would be a new solver feature, not
completion of referenced C behavior. The contradictory-unit oracle continues
to require trace-only output and success exit 0.

## Fresh Matrix And Discovered Gaps

The first fresh archived-C/Rust run is
`.artifacts/e-compare/20260717-011923-458914-tools/`. Eleven of 15 cases were
exact. Four executable diagnostics differed while retaining the same exit
codes and stdout:

- two scanner errors labeled standard input as `-` instead of C's `<stdin>`;
- the empty procedural-tail error omitted C's token position and
  `(just read '.')` context; and
- missing named input said `Cannot open file ... for reading` instead of C's
  pre-open `Cannot stat file ...` diagnostic.

These are real drop-in executable gaps, but they are independent of the absent
reference solver.

## Change

`edpll` now creates its in-memory standard-input scanner with the source label
`<stdin>`. Named files go through the shared `input_open` regular-file preflight
before their bytes are passed to the scanner, preserving C's two-line `stat`
error shape while retaining output-file creation order.

Parser diagnostics that originate from C `AktTokenError`-equivalent custom
clause checks are decorated at the `edpll` boundary with the scanner's current
token position and literal. Scanner-generated diagnostics already containing
that context pass through unchanged. The clause parser itself is not changed,
so other executable surfaces retain their established error ownership.

## Results

The accepted report is
`.artifacts/e-compare/20260717-012358-161243-tools/`:

- all 15 permanent `edpll` cases ran;
- all exit codes, status/shape records, normalized stdout, normalized stderr,
  and output-file records match archived C; and
- the report contains zero mismatches and zero expected differences.

The contradictory positive/negative unit case remains exact and prints no
satisfiability result, confirming that the diagnostic fixes did not turn the
reference parser/state shell into a solver.

## Validation

- All 23 focused `prover::edpll` tests pass with exact `<stdin>`, token-context,
  and `Cannot stat file` assertions.
- A locked optimized `edpll` build passes.
- The full 15-case archived-C/Rust matrix is exact.
- The C source remains unchanged.

## Limits

This closes the drop-in `cpr_dpll` state-shell gap and the four adjacent
executable diagnostics. A real DPLL implementation remains a deliberate
post-compatibility extension requiring new propagation, retraction, branching,
model/result, and command-line contracts.
