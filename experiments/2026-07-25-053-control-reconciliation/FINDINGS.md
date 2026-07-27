# Detailed CONTROL reconciliation

## Status

Accepted for the 107 remaining open `control` records under Beads
`E_Rust_Port-j76.4`. Direct source review found one real production lifecycle
gap, record 742. The concrete LTB batch runner now performs the complete C
`StructFOFSpecBacktrackToSpec` sequence: remove problem-specific distribution
and set roots, collect the term bank against the surviving shared roots, then
backtrack the signature. Two focused regressions pin both the owner operation
and its production caller.

The other 106 records are resolved by preserving intentional C behavior,
accepting an ownership-safe Rust representation with the same supported
observable result, or recognizing that the work described as pending was
completed by later proof-control, higher-order, preprocessing, scheduling,
SInE, and proof-object work.

## Decisions

Every migrated identity receives one of the following final decisions. The
audit script checks that these four lists are a disjoint, exact cover of all
107 records.

### Preserve the checked C behavior

Records
`514, 515, 522, 530, 533, 534, 537, 539, 543, 545, 547, 562, 565, 568,
571, 580, 587, 594, 597, 603, 610, 611, 613, 614, 615, 616, 618, 623,
628, 630, 634, 639, 641, 646, 653, 656, 668, 669, 675, 688, 690, 696,
704, 720, 721, 723, 736, 744`.

These are compatibility decisions, not deferred work. They retain such
details as the LTB spelling/path and socket asymmetries, TSTP-only loading,
reverse stack drains, repeated higher-order normalization hooks, the checked
PosExt gate, proof-documentation omissions and parent order, generated-clause
partial-state returns, authoritative index behavior, signed/unsigned limit
semantics, and missing-shared-include handling. Focused tests and the native
comparison matrices make the supported observable contracts explicit.

### Accept the ownership-safe Rust boundary

Records
`523, 524, 536, 541, 546, 550, 559, 561, 566, 567, 578, 582, 583, 602,
622, 644, 655, 666, 673, 677, 682, 683, 684, 687, 689, 705, 713, 714,
716, 727, 732, 733, 737, 738`.

These records describe C raw-pointer aliasing, fork inheritance, mutable
process globals, intrusive-set identity, or mixed output/ownership helpers.
Rust uses owned sets and clauses, explicit request and process-control
objects, stable identifiers or generation-qualified references, typed parser
state, structured cleanup metadata, and worker re-exec. The accepted boundary
preserves current executable ordering, output, cleanup, proof ancestry, and
search behavior while declining to reproduce dangling pointers, unchecked
overflow, or allocator-address semantics.

### Later implementation superseded the pending note

Records
`585, 588, 606, 617, 627, 632, 643, 647, 652, 654, 659, 679, 692, 693,
697, 707, 708, 715, 718, 719, 725, 726, 734, 741`.

The retained implementation now covers these paths. In particular:

- forward modification has the exact 9/9 source audit and 18/18
  higher-order ordering matrix;
- higher-order matching/CSU ownership is complete across all production call
  sites and its 21/21 comparison;
- simultaneous and super-simultaneous indexed paramodulation reuse active
  bindings as C does;
- preprocessing is closed by the 29/29 umbrella audit;
- cleanup, watchlist, destructive equality resolution, splitting, generated
  inference, indexed backward simplification, extraction roots, and SAT
  threshold progression run in the owned proof-control path;
- the corrected default-schedule retry has exact two-case evidence; and
- threshold, GSinE, and LambdaDef selection run across represented
  clause/formula owners, backed by the 9/9 axfilter and formula-proof-search
  closures.

### Implement now

Record `742`.

`StructFofSpec::backtrack_to_spec_with_bank` owns the missing coupled
operation. It first delegates the already exact distribution/set truncation,
then calls `tb_gc_collect` with the surviving structured-spec clause and
formula sets, and finally calls `Signature::backtrack` at the recorded
shared-axiom symbol boundary. The production
`BatchSpec::process_problem_with_runner_backend` now uses this method on both
success and error unwinding before runner cleanup/result propagation.

The owner regression creates shared and problem-only symbols/terms and proves
that rollback keeps the shared symbol and term while removing the problem
symbol and term. The production regression constructs a problem-only symbol
through the real batch backend and proves the same cleanup happens at the
caller boundary.

## Evidence

The source and retained experiments cover every decision:

- batch grammar, printing, filtering, temporary-file/process ownership,
  socket-result asymmetry, includes, TSTP loading, typed declarations, variant
  ordering, and time budgets have focused tests;
- interactive stage/add/load/run/quit and TCP framing are regression-pinned;
- factor generation, forward modification, cleanup, higher-order
  extensionality/choice/primitive enumeration, and parameterized
  paramodulation are exercised through their production proof-control owners;
- preprocessing, selected-axiom movement, schedule retry, watchlist/global
  indices, proof documentation, and ordered extraction have dedicated
  closure experiments; and
- the new full lifecycle validates the exact modified source rather than
  relying on the earlier unchanged-source reconciliation reference.

## Audit

[`audit_control_reconciliation.py`](audit_control_reconciliation.py) pins all
107 migrated identities and their content hashes, incorporates the four
decision classes into the decision digest, checks ten grouped
source/implementation/evidence contracts, and digests the 34 unchanged C
files, the effective Rust owners, retained closure findings, and the current
validation reference. It is independent of issue status and therefore remains
reproducible after closure.

## Validation

Ephemeral Linode run `.artifacts/linode/260726-234007-4e8d/` passed the full
repository lifecycle on this exact candidate:

- Rustfmt and strict all-target/all-feature pedantic Clippy pass;
- 4,419 library plus 11 integration tests pass, 4,430 total;
- native release and compile-only Windows GNU x64 all-target/all-feature
  builds pass;
- clean FOL and higher-order C references build and pass smoke checks;
- all 50 main-prover and all 216 support-tool cases have zero unexpected
  differences; and
- the ten-case aggregate is 1.083x Rust/C wall time.

The lifecycle wrote `SUCCESS` and `VALIDATION_COMPLETE`, retained its reports,
and deleted its Linode and firewall. No Rust or C toolchain ran on the local
Windows host, and the vendored C checkout was not modified.

Reproduce the source audit locally:

```powershell
.\.venv\Scripts\python.exe experiments/2026-07-25-053-control-reconciliation/audit_control_reconciliation.py `
  --repo . `
  --expected experiments/2026-07-25-053-control-reconciliation/audit-reference.json
```
