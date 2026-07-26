# PROPOSITIONAL Change Later reconciliation

## Status

Accepted for the five remaining `propositional` records under Beads
`E_Rust_Port-j76.4`. Earlier `edpll` work already established the executable
contract with an exact 15-case C/Rust matrix. The remaining representation
choices preserve every implemented C behavior while replacing undefined,
non-portable, or pointer-coupled internals with deterministic safe ownership.
The original C checkout remains unchanged.

## Decisions

- `j76.4.989`: do not invent `DPLLRetractLastAss`. C declares it but provides
  no definition, its deactivate/shorten helpers are zero-return stubs, and
  `edpll` only allocates then frees the state. Rust preserves the implemented
  assignment-marker/false-result shell. Propagation, retraction, branching, and
  SAT/UNSAT output remain a deliberate new-feature boundary, not port debt.
- `j76.4.993`: retain the explicit `&mut TermBank` parser boundary. It preserves
  C's borrowed-signature parsing behavior without manufacturing a temporary
  owner whose signature pointer must be nulled before destruction. Direct
  parser tests and the exact executable matrix cover the observable behavior.
- `j76.4.994`: retain the documented absolute-atom order with positive before
  negative for equal atoms. C's `abs_a2 = ABS(*a1)` typo violates the comparator
  contract and has no portable `qsort` result to reproduce. The deterministic
  Rust normalization order, duplicate removal, complementary-pair detection,
  and integrated trace behavior have direct and executable reference coverage.
- `j76.4.996`: keep the literal vector's original length after normalization,
  exactly retaining C's allocated storage and inactive slots. The focused
  regression pins five live literals in seven storage slots; no measured
  mutation pattern justifies changing this already compatible cold path.
- `j76.4.999`: retain separately owned strings in the encoding vector and name
  map. Shared C `char*` identity is not observable through the propositional
  API; encoding allocation, duplicate lookup, reverse lookup, assertions, and
  print order are tested. Introducing an interning handle would add ownership
  complexity without a compatibility benefit.

[`audit_propositional_reconciliation.py`](audit_propositional_reconciliation.py)
pins all five migrated identities and content hashes plus six grouped
call-path, parser, normalization, storage, ownership, and validation checks.
The audit is independent of Beads status.

## Validation

This review changes only Beads and experiment documentation. The accepted
`edpll` evidence has 15 exact C/Rust cases with no expected differences. The
exact Experiment 041 snapshot passes 4,427 tests, strict
all-target/all-feature pedantic Clippy, native and Windows GNU x64 builds, and
both maintained compatibility matrices with zero unexpected differences.
Local source and documentation audits pass; no Rust/C toolchain ran locally.

Reproduce locally:

```powershell
.\.venv\Scripts\python.exe experiments/2026-07-25-045-propositional-reconciliation/audit_propositional_reconciliation.py `
  --repo . `
  --expected experiments/2026-07-25-045-propositional-reconciliation/audit-reference.json
```
