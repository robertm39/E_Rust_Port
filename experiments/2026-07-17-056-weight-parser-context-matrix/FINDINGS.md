# Weight-parser proof-state context reconciliation

## Status

Completed for Bead `E_Rust_Port-j76.2.82`. All 46 entries in C's
`WeightFunParseFunNames`/`parse_fun_array` tables are wired through Rust's
production proof-state context. No Rust implementation change was needed, and
the vendored C checkout remained unchanged.

## Production ownership audit

Executable proof search obtains the live term bank, clause axioms, and formula
axioms from `ProofState`, then calls `proof_control_init_with_formula_axioms`.
That initializer constructs a `WeightParseContext` containing:

- the clause axioms required by staggered, GD, conjecture, distance, and
  relevance weight initializers;
- the represented formula axioms consumed by both relevance-level variants;
  and
- the live signature required by the TSM/TSMR parser path.

Both built-in and option-defined WFCB lists are parsed through
`weight_fun_def_list_parse_with_context`. Inline WFCB definitions in HCBs use
the same context. The context-free `WeightFunParse`/definition wrappers remain
intentional low-level APIs for context-free parsers and return a diagnostic for
state-dependent names; the executable does not use that path.

## C/Rust executable matrix

[`compare_surfaces.py`](compare_surfaces.py) passes one named
`--define-weight-function` for each of the 46 C parser-table entries through a
small proof run. It also covers the anonymous-definition branch of
`WeightFunDefParse`. Every option is therefore consumed by the production
`WeightFunDefListParse` boundary after proof-state construction.

The cached C reference is commit
`17026b1bfe61aaf223cfaae54947c8d2679c31a0`. Exit status, stdout bytes, and
stderr bytes are compared without normalization. The result is 47/47 exact,
including TSM/TSMR, both relevance-level variants, all conjecture-backed
families, and the table quirk where `ConjectureSymbolWeight` dispatches the
simplified parser. Compact hashes are retained in
[`results-summary.json`](results-summary.json).

## Permanent Rust coverage

Existing `wfcbadmin` tests pin the 46-name order and lookup indices, exercise
every table entry with a complete valid specification, verify clause/formula/
signature contexts, reject unknown and contextless state-dependent names, and
cover explicit plus `~$%09ld` anonymous definitions. The executable regression
`run_proof_search_installs_user_weight_and_heuristic_definitions` additionally
selects and evaluates an option-defined WFCB through a custom HCB.

## Validation

- executable C/Rust matrix: 47/47 byte-exact;
- `cargo test --locked --all-features --lib wfcbadmin --quiet`: 17 passed;
- experiment script compilation: passed;
- formatting and all C-source documentation gates: passed; and
- the immediately preceding unchanged-production baseline passed 4,260
  all-feature library tests plus every target, strict pedantic Clippy, and the
  release build.

## Residual scope

Duplicate-name shadowing and C's redundant explicit-name allocation remain
separate post-compatibility questions under `E_Rust_Port-j76.3.143` and
`.3.144`. They do not represent missing parser dispatch or proof-state context.
