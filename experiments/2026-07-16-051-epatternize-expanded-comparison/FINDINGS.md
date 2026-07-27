# epatternize expanded executable comparison

## Status

Completed for Bead `E_Rust_Port-j76.1.43`. The permanent support-tool matrix
now exercises malformed inputs, every first-order outer record family through
nested selected includes, a larger mixed formula/clause corpus, multi-file
output, and stable filesystem failures. The vendored C source remained
unchanged.

Historical note: experiment 052 later extends the permanent matrix from 13 to
16 cases with the shared parser's unrecognized-tail contract. This experiment
and its runner intentionally retain the exact 13-case snapshot validated here.

## C execution path

`epatternize.c` processes options before opening `GlobalOut`, inserts `-` only
after the output destination is open, and then allocates a fresh proof state and
default pattern substitution for each positional input file. Each file runs
through `FormulaAndClauseSetParse`, optional SInE, conjecture preprocessing,
`FormulaSetCNF2`, per-clause substitution backtracking, flat clause-list
encoding, and `PatternTermPrint`. The final `fflush` and `OutClose` own the
output-close diagnostic.

`FormulaAndClauseSetParse` accepts raw LOP clauses or the non-LOP outer record
families `input_formula`, `input_clause`, `fof`, `cnf`, `tff`, `tcf`, `thf`,
and `include`. A non-HO C build rejects `thf` before parsing, so the permanent
first-order differential corpus deliberately covers the other record families;
Rust's higher-order epatternize path remains covered by focused library tests.
The caller-provided `skip_includes` tree starts empty and is never populated in
this source snapshot, so repeated includes are parsed repeatedly rather than
deduplicated.

## Parity fixes

The library already attached C `ErrorCode` values to every failure, but
`src/bin/epatternize.rs` printed all diagnostics and returned status 1. The
wrapper now returns `error.code().exit_status()`: malformed scanner inputs exit
3, the invalid compatibility mask exits 5, and file-open failures exit 6.

A missing top-level input already used C's two-line `SysError` form. A missing
file reached recursively through `include`, however, retained the shared
scanner's one-line `Cannot open ...: <OS error>` diagnostic. The epatternize
parse boundary now applies the same executable-specific normalization to
recursive scanner opens. Both paths preserve the stable first line and put the
platform suffix on a second `epatternize: ...` line.

## Permanent matrix

The matrix grew from three cases to thirteen:

- archived help, version, and single-clause LOP cases remain;
- an old-TPTP case combines `input_formula` and `input_clause` owners;
- a 16-record TSTP corpus combines five type declarations, legacy owners,
  FOF/TFF/TCF formulas, CNF clauses, quantifiers, connectives, equality, and an
  ignored watchlist owner, producing 17 exact pattern lines after CNF;
- a nested selected include combines legacy and modern formula/clause records,
  TFF and TCF owners, source/useful-info fields, a selected watchlist entry,
  dropped records at both include depths, and a local formula, producing exactly
  seven normal patterns and no watchlist/dropped output;
- two positional files write two exact patterns to one output file while stdout
  remains empty;
- malformed LOP and TSTP inputs pin complete scanner diagnostics and exit 3;
- missing include, input, and output-parent cases pin first diagnostic lines,
  two-line system-error structure, exit 6, and failed-output absence; and
- a short class mask pins the compatibility wording and exit 5.

`run_native.py` asserts stable stdout, exact malformed/usage stderr, output-file
bytes, exit statuses, absence of failed output paths, and the platform-neutral
portion of filesystem errors for every case.

## Reference and platform boundaries

The archived built-C report at
`.artifacts/e-compare/20260715-203258-985096-tools/tool-comparison.json` proves
byte-for-byte equality and equal exit status for help, version, and the original
LOP pattern workload using upstream commit
`17026b1bfe61aaf223cfaae54947c8d2679c31a0` under Ubuntu 24.04/GCC 13.3.0.

This desktop session has no installed WSL distribution, visible cached C
executable, or native POSIX C toolchain, so the ten new cases could not be run
against the archived ELF binary. Their expected behavior is source-reviewed and
the permanent matrix is ready for the normal differential command when the
reference environment returns:

```powershell
cargo build --locked --release --bins
.\e-interop.ps1 build-reference
.\e-interop.ps1 compare-tools -RustBinDir .\target\release -Tool epatternize
```

The comparison harness canonicalizes only the complete known POSIX/Windows
system-error suffix; program name, path, action text, line structure, channel,
and status remain strict. Broken-pipe behavior is not a deterministic
cross-platform batch case: POSIX C may terminate by `SIGPIPE`, while other
hosts or redirected stream kinds can reach `OutClose`. Rust's injected flush
failure test pins the reachable C `OutClose` wording and `FILE_ERROR`; actual
signal delivery, full filesystems, and quota exhaustion remain host-policy
boundaries rather than claims of unobserved byte equality.

## Validation

- focused `epatternize` library tests: 21 passed;
- full library suite: 4,158 passed;
- all binary targets passed;
- integration targets `eprover_schedule`, `e_stratpar`, and
  `executable_inventory`: 4, 1, and 1 passed;
- `cargo check --locked --all-targets`: passed;
- `cargo clippy --locked --all-targets -- -D warnings`: passed;
- `cargo fmt --all -- --check`: passed;
- bundled-Python `tools/e-interop` discovery: 32 passed;
- release `epatternize` build: passed;
- optimized native `epatternize` matrix: all 13 expected outcomes passed;
- C-source documentation coverage: 492 sources / 266 unit docs;
- Change Later wording and local-link checks: 269 Markdown files each; and
- regeneration preservation: 268 Markdown files.
