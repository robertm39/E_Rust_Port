# epclanalyse platform boundaries

## Status

Completed for Bead `E_Rust_Port-j76.1.32` with expanded differential cases,
field-scoped non-finite normalization, an exact safe zero-denominator
regression, and evidence-backed decisions for the C undefined-behavior and
broken-pipe boundaries. The vendored C source remained unchanged.

## Representative selection and unsafe C boundaries

`PCLProtPropAnalyse` calls `PCLProtFindMaxStep` four times over every protocol
step. The metric comparators rank formulas below clauses but consider formulas
equal to one another. `PCLProtPropDataPrint` then unconditionally prints all
four representatives and unconditionally calls `ClausePropInfoPrint` on the
heaviest representative's `logic.clause` union member.

That produces two source-level undefined-behavior boundaries:

- for an empty protocol, every representative is null, so the first
  `PCLStepPrint` assertion fails in a debug build and a release build proceeds
  to dereference the null step; and
- for a formula-only protocol, every representative is a formula, but the
  heaviest section reads the inactive clause union member and passes the result
  to `ClausePropInfoPrint`.

Rust deliberately does not reproduce either crash/invalid union read. It emits
the C-shaped zero-denominator summary, leaves empty representatives blank, and
prints formula representatives without clause-only property metrics. Existing
regressions pin both total behaviors. This is a safety correction for inputs on
which C has no defined output contract, not a loss of a supported feature.

## Safe zero-denominator comparison

The new `zero-denominator-safe-boundary` case contains an initial formula and a
derived empty clause. C and Rust both exclude empty clauses from the aggregate
clause count, so all ten averages have zero denominators. The clause still
outranks the formula in every representative search, keeping C's later clause
metric access valid.

The exact Rust regression verifies:

- ten non-finite average fields;
- four renderings of step 2 as the selected representative;
- exclusion of formula step 1 from representative output; and
- the empty clause's zero standard weight.

## Non-finite spellings

The C format is `%6.4f`, but the token is runtime-specific. The checked source
and earlier reference evidence establish glibc `-nan`; Rust renders `NaN`; and
legacy Microsoft runtimes can render `-1.#IND00`. The comparison harness now
maps those variants to `<NAN>` only when the line is one of
`epclanalyse`'s average fields. It also canonicalizes field padding because the
token lengths differ. Count fields, representative output, arbitrary `-nan`
text, line structure, and channels remain byte-strict.

## File and broken-pipe boundaries

The new isolated `missing-input` case exercises the scanner-open `SysError`
shape. Native Windows exits 6 and emits the stable first line followed by:

```text
epclanalyse: The system cannot find the file specified. (os error 2)
```

Only the established complete POSIX/Windows not-found suffix is canonicalized.

Unlike `epclextract`, `epclanalyse` has no `FAST_EXIT` branch: after printing it
explicitly flushes and calls `OutClose`. Rust retains the same stable
checked-close diagnostic, pinned with `FlushFailWriter`. A real POSIX closed
pipe may terminate C through the default `SIGPIPE` before `OutClose`, whereas
Windows reports a write error. The subprocess harness retains its capture-pipe
reader and therefore cannot create that condition without a separate
platform-specific shell test. Rust keeps deterministic checked-write/flush
errors rather than reproducing host-specific signal termination.

## Reference availability and decision

The archived comparison already covers help, version, and a small non-empty
protocol. This sandbox has no visible WSL distribution, compiler, surviving C
`epclanalyse` binary, or compatible reference environment, so the new cases
cannot be rerun against C in this session. The safe boundary follows the direct
branches in `pcl_propanalysis.c`; the permanent cases will run automatically
when the reference environment is restored. The scoped normalizer and explicit
undefined-behavior/broken-pipe policies are the evidence-backed compatibility
decision permitted by the migrated work item's acceptance criteria.

## Performance decision

No analysis implementation changed. Normalization runs only in the external
comparison harness, and the new workloads are tiny, so a performance benchmark
is not warranted.

## Validation

- `tools/e-interop/test_e_interop.py`: 28 passed
- focused `prover::epclanalyse::tests`: 17 passed
- native safe-boundary probe: ten `NaN` fields, four step-2 representatives,
  exit 0
- native missing-input probe: exit 6 with the expected Windows suffix
- `cargo test --locked --lib --quiet -- --test-threads=1`: 4,128 passed
- `cargo test --locked --bins --quiet`: all binary targets passed
- `cargo test --locked --test eprover_schedule --quiet`: 4 passed
- `cargo test --locked --test e_stratpar --quiet`: 1 passed
- `cargo test --locked --test executable_inventory --quiet`: 1 passed
- `cargo build --locked --release --bin epclanalyse`
- `cargo fmt --all -- --check`
- `cargo check --locked --all-targets`
- `cargo clippy --locked --all-targets -- -D warnings`
- C-source coverage: 492 source files covered by 266 unit docs
- Change Later wording: 269 Markdown files checked
- local Markdown links: 269 Markdown files checked
- regeneration preservation: manual sections preserved in 268 Markdown files
- `git diff --check`
