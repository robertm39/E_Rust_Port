# Shared TSM proof-state bank

## Status

Completed for Bead `E_Rust_Port-j76.2.79`. Production `TSMWeight` and
`TSMRWeight` evaluation now uses the active proof-state term bank supplied by
the banked HCB path. The vendored C checkout remained unchanged.

## Ownership reconciliation

C stores a borrowed `ProofState_p` in `TSMParamCell`. On the first score,
`TSMFromKB` mutates `state->terms->sig` with the KB signature; every later
candidate representation is encoded directly in `state->terms`.

Rust already installed built-in, option-defined, and inline learned WFCBs
through production `ProofControl`, and every production HCB evaluation site
already supplied the mutable owner bank. The remaining mismatch was inside the
evaluator: it cloned the signature, created a private `TermBank`, and copied
every scored clause into that bank.

The learned WFCBs now register a banked callback. Lazy initialization loads the
KB through the active bank's signature, records the proof-state owner mode, and
encodes the original clause directly in that bank. The compact target-feature
snapshot retained at parse time still avoids owning a cloned proof-state axiom
set. The immutable `compute_eval` entry point retains a private-bank adapter for
low-level tests and staged callers; it is not used by production proof search.

Two permanent regressions pin both boundaries:

- direct banked evaluation proves that lazy KB signature declarations appear
  in the supplied proof-state bank and that the evaluator records shared-bank
  ownership; and
- a user-defined learned WFCB plus HCB installed through `ProofControl` reaches
  the same bank through the active heuristic evaluation path.

## C/Rust executable comparison

[`compare_and_benchmark.py`](compare_and_benchmark.py) runs both `TSMWeight`
and `TSMRWeight` through option-defined WFCBs, an option-defined active HCB, and
proof search. Both 2/2 cases match the cached C reference at commit
`17026b1bfe61aaf223cfaae54947c8d2679c31a0` on exit status, `Unsatisfiable`
status, empty stderr, and six parsed/initial/processed/current clause statistics.

The executable fixture uses C's default flat learned-clause encoding and an
empty recursive `$cnil` training pattern. The current C reference's nonempty
recursive candidate encoding reuses the now-logical `$or` symbol with a
`$cnil` second argument and terminates with a Boolean/individual type mismatch;
Rust's separately tested recursive encoder is retained as the intended learned
feature instead of reproducing that modern C type collision.

## Performance

The benchmark generates 4,000 LOP equations over a fixed 64-symbol vocabulary,
disables preprocessing, and stops before saturation processing. This isolates
initial HCB/TSM scoring from the unrelated unique-signature scaling seen in an
early exploratory workload. Each executable receives one warm-up and five
measured runs.

| Executable | Median wall time |
| --- | ---: |
| Rust shared proof-state bank | 0.242778 s |
| Rust private-bank parent `1c637bc4` | 0.284686 s |
| C reference through WSL | 0.199410 s |

The shared-bank path is `0.853x` the parent-commit time, a 14.7% reduction, and
is `1.217x` the C reference wall time. All measured processes returned the same
processed-clause-limit exit status with empty stderr. Commands, individual
samples, hashes, and extracted compatibility fields are retained in
[`results-summary.json`](results-summary.json).

## Validation

- executable learned-strategy comparison: 2/2 cases;
- learned evaluator tests: 8/8, including the shared-bank owner regression;
- focused proof-control learned-strategy installation regression: passed;
- complete all-target/all-feature suite: 4,264 library tests plus every
  auxiliary target passed;
- strict all-target/all-feature pedantic Clippy and formatting: passed;
- release `eprover` build and final optimized benchmark: passed;
- experiment script compilation and all C-source documentation gates: passed;
  and
- `eprover/` remained unchanged.

## Residual scope

Manual lifecycle ownership is complete through Rust drop. The immutable WFCB
adapter deliberately remains private-bank based, while production proof search
uses shared banked evaluation. Broader learned-data ingestion/session work and
the C recursive-encoding type collision are independent of the completed
proof-control installation and hot-path owner reconciliation.
