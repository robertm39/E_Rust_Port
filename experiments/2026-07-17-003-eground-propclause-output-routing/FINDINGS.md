# eground compact propositional output routing

## Status

Completed for Bead `E_Rust_Port-j76.2.119`. The permanent support-tool matrix
now exercises `ccl_propclauses` through the real `eground` executable in every
non-DIMACS output route. No Rust source change was required; the vendored C
tree remained unchanged.

## Question

Does Rust's explicit output-format parameter reach the compact non-unit clause
printer everywhere C relies on the process-global `OutputFormat` observed by
`ClausePrint`?

## Added cases

Four permanent cases use a genuine compact non-unit ground clause:

- LOP input with the default LOP output fallback;
- LOP input with `--tptp-out`;
- LOP input with `--tstp-out`; and
- auto-detected TSTP input, which makes `eground` select TSTP output without an
  explicit output option.

The path is `eground` result writing to `GroundSet::print_format_string`, then
`PropClauseSet::print_format_string`, then the temporary ordinary-clause
renderer. Set-level newlines remain outside the single-clause printer exactly
as in C.

## Result

All four new cases are byte-for-byte exact against the archived upstream C
`eground` from commit `17026b1bfe61aaf223cfaae54947c8d2679c31a0`, including
exit status, stdout, and stderr. The fresh report is:

`.artifacts/e-compare/20260717-014049-604631-tools/`

The complete 21-case matrix still reports five pre-existing `eground`
diagnostic/estimate mismatches outside this output-routing task: verbose GC and
stdin wording, two stdin diagnostic source labels, the `--give-up=1` estimate
decision, and named missing-input wording. Those remain visible for the next
executable-parity slice; none of the four new routing cases mismatches.

## Validation

- four new archived-C/Rust compact non-unit cases: exact;
- existing direct `ccl_propclauses` unit coverage exercises clause conversion,
  LOP/TPTP/TSTP dispatch, and set-level newline ownership;
- `eprover/` status remained clean.
