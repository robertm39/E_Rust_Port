# Format-option integration parity

Date: 2026-07-18

Bead: `E_Rust_Port-j76.2.34`

Upstream reference commit: `17026b1bfe61aaf223cfaae54947c8d2679c31a0`

## Question

Do the parser, proof-documentation, syntax-only formula, and saturated-clause output paths all consume the same format-option state as upstream E?

The audited options were `--lop-in`, `--tptp-in`, `--tstp-in`, `--tptp-format`, `--tstp-format`, `--tptp-out`, `--tstp-out`, `--pcl-out`, `--pcl-shell-level`, `--pcl-compact`, `--pcl-terms-compressed`, `--print-types`, `--eqn-no-infix`, `--full-equational-rep`, and `--print-oriented-eqlits-as-rules`.

## Method

`compare_format_options.py` runs 18 production CLI cases against an isolated upstream higher-order build and the Rust release binary. It projects only stable semantic output, normalizes generated `c_0_*` and `i_0_*` identifiers, and compares exit codes, SZS status, stderr, proof documentation, and printed clauses exactly.

The retained reference is `reference.json`. Its SHA-256 is `9f319ec0eddf7a7586f94f3cd20739175525e013e5da446603511cf7012306e2`.

Reproduction from the repository root:

```powershell
cargo build --locked --release --bin eprover --all-features
C:\Users\rober\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe experiments\2026-07-18-104-format-option-integration\compare_format_options.py --c-exe /home/rober/.cache/e-rust-port/src/17026b1bfe61aaf223cfaae54947c8d2679c31a0-ho-20260717b/PROVER/eprover-ho --rust-exe target\release\eprover.exe --output target\format-option-comparison.json --expected experiments\2026-07-18-104-format-option-integration\reference.json
```

## Findings

The first comparison found four ownership and propagation defects:

1. Rust documented post-CNF clauses as fresh initial clauses. This produced C's deliberate `XX` watchlist markers for ordinary formula input and shifted proof IDs. Upstream instead documents formula owners and pre-existing clause owners before CNF, then documents watchlist clauses after CNF.
2. Formula PCL steps used clause-list syntax and later used the configured compressed-term setting. Upstream uses formula syntax and keeps preprocessing formula terms full; `PCLFullTerms` is changed only when proof-search initialization begins.
3. Formula proof steps did not propagate `--print-types` or the global equation output format. Upstream's `TFormulaTPTPPrint` reaches `EqnFOFPrint`, which observes both `TermPrintTypes` and `OutputFormat`.
4. Clause-origin formula owners were rendered as `cnf`/`tcf` proof records. Upstream passes `as_formula=true` for initial formula documentation, producing `fof`/`tff`/`thf` records and universally closing clause variables.

The corrected ownership is:

| State | Owner | Rendering |
| --- | --- | --- |
| Parsed formula or clause promoted into `f_axioms` | Formula set | Formula PCL body; `fof`/`tff`/`thf` TSTP record |
| Parsed clause retained in the clause set | Clause set | Clause PCL/TSTP initial record, including upstream `XX` side-channel markers |
| Loaded watchlist clause | Watchlist clause set | Initial clause record after formula CNF |
| Formula preprocessing | Formula set | Full terms, configured type suffixes, global equation output mode |
| Main proof search and proof object | Clause derivation | Configured compressed/full terms, compact/shell controls, requested clause format |
| `--print-formulas` | Formula set | TSTP formula wrappers while equality syntax follows global `OutputFormat` |

## Result

The final report validates 18 of 18 exact C/Rust cases. All eleven independent effect assertions are true: distinct shell levels, compact steps, full preprocessing terms followed by compressed search terms, type printing, watchlist side-channel preservation, absence of `XX` for ordinary formulas, no-infix equality, full-equational predicates, oriented rules, and TPTP/TSTP output selection.
