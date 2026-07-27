# Exhausted higher-order terminal-status reconciliation

## Status

Completed for bug `E_Rust_Port-j76.2.140`. Exhausting the unprocessed set for
a higher-order axioms-only problem now reports C's `GaveUp` status and exit 10
instead of claiming satisfiability. Focused first-order satisfiable and
higher-order theorem controls remain exact. The vendored C checkout remained
unchanged.

## Root cause

After saturation, C distinguishes an empty unprocessed set from semantic
satisfiability. Its satisfiable branch requires all of the following:

- the proof state is complete;
- the represented inference system is complete;
- no unimplemented interpreted symbols remain; and
- `problemType != PROBLEM_HO`.

The last condition is deliberate compatibility behavior: upstream E does not
claim that an exhausted higher-order search establishes satisfiability. It
falls through to `Failure: Out of unprocessed clauses!`, reports `GaveUp`, and
returns `INCOMPLETE_PROOFSTATE` (exit 10).

Rust already checked the other conditions but omitted the problem-type gate in
both `write_saturated_final_result` and `saturate_outcome_exit_status`. That
made the output and process exit disagree with C in exactly the exposed case.
Both decision sites now include the same higher-order exclusion.

## Executable comparison

[`compare_terminal_status.py`](compare_terminal_status.py) compares the
Windows Rust release executable with the isolated `--enable-ho` C build from
upstream commit `17026b1bfe61aaf223cfaae54947c8d2679c31a0`. The C executable
SHA-256 is
`317e261b4915d16834de9f5a133ecd07fe6e21dfdc8c5f06072ed75b3e56b7e1`;
the compared Rust executable SHA-256 is
`2acabc1d1283bb4fabe685e6fbf0a6ef79a66bf40a19a88ff012b94f4dd7de4f`.

All three cases match exactly on exit status, stderr, ordered result/status
lines, final clauses, and selected counters:

| Case | Exit | Ordered terminal result |
| --- | ---: | --- |
| exhausted higher-order axioms | 10 | `Failure: Out of unprocessed clauses!`; `GaveUp` |
| first-order satisfiable control | 1 | `No proof found!`; `Satisfiable` |
| higher-order theorem control | 0 | `Proof found!`; `Theorem` |

For the exhausted fixture, both binaries also retain the same two normalized
final clauses, two initial/processed clauses, zero generated clauses, and one
rewrite step. This connects the corrected terminal decision to the earlier
unification experiment without broadening the change into inference logic.

The retained comparison report has SHA-256
`dd6605777180ae39bf6a5d06016b6336422ec63ecc487c977488cbb67fdc46dc`.

## Permanent regression

The executable unit regression runs the same higher-order rewrite fixture with
`--processed-clauses-limit=2`, asserts exit 10, pins the exact ordered failure
and `GaveUp` tail, and rejects any `Satisfiable` status. Existing first-order
satisfiable, FOF counter-satisfiable, and higher-order theorem tests remain
unchanged and pass alongside the new regression.

## Validation

- exact C/Rust terminal-status comparison: 3/3 cases;
- focused permanent exhausted-HO regression;
- retained-reference rerun and experiment-script compilation;
- full all-target/all-feature Rust suite and strict pedantic Clippy;
- release `eprover` build and all C-source documentation integrity gates; and
- clean nested `eprover/` worktree.
