# Heuristic lookup and finalization integration

## Status

Completed for Bead `E_Rust_Port-j76.2.77`. The remaining sentence in the
migrated item is stale or belongs to narrower scheduler tasks. No production
behavior change was required, and the vendored C checkout remained unchanged.

## Question

Does the migrated `che_heuristics` item still identify missing executable
HCB/WFCB installation, proof-state mutation, or scheduler accounting work, or
does it combine behavior that has already been completed by narrower owners?

## Method

[`audit_integration.py`](audit_integration.py) checks the C and Rust production
call graph, verifies the retained evidence from the strategy, banked-WFCB, and
scheduler audits, and compares four proof-search paths byte-for-byte against
the isolated C reference.

The executable cases cover anonymous inline `GetHeuristic` parsing, named
custom WFCB/HCB definitions, a selected predefined strategy, and generated
plain `--auto` strategy selection.

## Findings

The source audit passed every expected contract:

- C defines `finalize_auto_parms` once and never calls it. Rust likewise keeps
  its compatibility helpers outside production; wiring either helper into
  plain `--auto` would invent behavior that the current C executable does not
  have.
- The Rust executable calls `proof_control_init_with_formula_axioms` once on
  the live proof-search path. That initializer installs default WFCBs, installs
  configured WFCBs, installs default HCBs, installs configured HCBs, copies the
  finalized parameters into `ProofControl`, and resolves the active HCB through
  `get_heuristic_handle_with_context`.
- The retained reporting matrix is 11/11 byte-exact. The banked production
  audit still reports zero forbidden immutable evaluation calls and eight
  banked proof-control lifecycle calls. The multicore audit records both the
  exact two-clock CPU/resource contract and the safe exec-worker state-transfer
  decision.

All four additional proof-search cases are byte-exact:

| Case | Exit | Stdout bytes | Exact |
| --- | ---: | ---: | :---: |
| anonymous inline heuristic | 0 | 179 | yes |
| named custom definitions | 0 | 179 | yes |
| selected predefined strategy | 0 | 215 | yes |
| generated auto strategy | 0 | 412 | yes |

The exact hashes and mismatch payloads, if a future run regresses, are retained
in [`results-summary.json`](results-summary.json).

## Compatibility decision

Rust preserves the dead C `finalize_auto_parms` body as independently tested
compatibility helpers, but production follows the live C path through generated
or explicit parameters and `ProofControlInit`. The Rust owner boundary is an
explicit mutable proof-state/parser context rather than a raw `ProofState`
back-pointer; the banked-WFCB audit demonstrates that the active lifecycle uses
that owner context.

Scheduler state transfer is not a `che_heuristics` integration defect. Exact
CPU/resource behavior and worker cleanup were completed by the multicore audit.
The deliberate portable exec-worker reparse boundary remains isolated under
`E_Rust_Port-j76.2.35` and `E_Rust_Port-j76.3.80`, where its measured startup
cost can be evaluated without distorting heuristic lookup or proof-control
ownership.

## Validation

- reproducible source audit: all 14 contracts passed;
- focused C/Rust proof-search matrix: 4/4 byte-exact;
- retained strategy matrix: 11/11 byte-exact;
- retained banked production audit: zero forbidden calls and eight lifecycle
  calls; and
- experiment script compilation: passed.
