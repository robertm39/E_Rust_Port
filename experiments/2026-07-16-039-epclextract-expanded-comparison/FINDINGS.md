# epclextract expanded comparison

## Status

Completed for Bead `E_Rust_Port-j76.1.31` with expanded differential cases,
exact mixed-logic and multi-file regressions, and an evidence-backed output-error
policy. The vendored C source remained unchanged.

## Extraction surfaces

The C `PCLProtMarkProofClauses` implementation first seeds every step whose
extra field is `proof`, `final`, or `extract`, then recursively collects every
quoted precondition without restricting the referenced step's logical kind.
`PCLProtPrintPropClauses` subsequently renders every marked step through
`PCLStepPrintFormat`. Rust uses the same seed, recursive-precondition, sorted
property-filter, and format-dispatch boundaries.

The new `mixed-logic-proof-closure` matrix case contains:

- an initial formula;
- a shell step referring to that formula;
- a derived disjunction referring to the shell;
- a lemma clause with a branched `pm(1,3)` justification;
- a final shell step that seeds the extraction; and
- an unrelated clause that must not be printed.

An exact Rust regression pins all five selected PCL lines, including formula
parentheses, the empty shell logic field, the synthesized `'lemma'` extra, and
the absence of the unrelated sixth step.

## Multi-file comments

The C executable parses every input file into one protocol and configures each
scanner to forward comments immediately when `--forward-comments` is active.
Proof marking and printing happen only after all files have been parsed. The
new `multi-file-comments` differential case therefore expects the first file's
lead and tail comments, then the second file's lead and tail comments, followed
by the cross-file formula/shell proof closure. The Rust regression asserts the
complete output string rather than only checking prefixes or contained steps.

## Platform diagnostics

The new isolated `missing-input` case exercises the two-line scanner-open
diagnostic. Native Windows exits 6 and emits the stable prefix followed by:

```text
epclextract: The system cannot find the file specified. (os error 2)
```

The comparison harness canonicalizes only the established complete POSIX and
Windows not-found suffixes. Program-authored text, path, program prefix,
line structure, output channel, and exit status remain strict.

## Broken-pipe decision

The upstream `Makefile.vars` defines `-DFAST_EXIT` in its default release
flags. In that profile `epclextract.c` calls `exit(0)` immediately after proof
printing and never reaches `OutClose`. On POSIX, a write to a closed pipe can
instead terminate the process through the default `SIGPIPE`; on a runtime that
only records a final stdio-flush failure, `exit(0)` can remain successful. A
non-`FAST_EXIT` C build explicitly calls `OutClose` and reports:

```text
Output stream to be closed reports error (probably broken pipe, file system full or quota exceeded)
```

Rust keeps one deterministic checked-write and checked-flush policy, preserving
that stable non-`FAST_EXIT` diagnostic instead of reproducing build- and
platform-dependent silent failure or signal termination. The existing
`FlushFailWriter` regression pins the exact diagnostic and file-error status.
The subprocess comparison harness intentionally does not manufacture a closed
pipe because its capture pipe retains a reader; doing so portably would require
a separate shell/process-control test whose expected result necessarily differs
by C build profile and host signal model.

## Reference availability and decision

The archived comparison already establishes exact help, version, and the small
clausal extraction workload. This sandbox has no visible WSL distribution,
compiler, surviving standalone C `epclextract` binary, or compatible reference
environment, so the three new cases cannot be rerun against C in this session.
Their C behavior follows the direct branches in `epclextract.c`,
`pcl_protocol.c`, and `pcl_steps.c`; the permanent cases will run automatically
when the reference environment is restored. Together with the exact native
regressions and the explicit broken-pipe policy, this is the evidence-backed
compatibility decision permitted by the migrated work item's acceptance
criteria.

## Performance decision

No extraction implementation changed. The new corpus exercises existing proof
marking and rendering paths, and the diagnostic cases run only on failure, so a
performance benchmark is not warranted.

## Validation

- `tools/e-interop/test_e_interop.py`: 27 passed
- focused `prover::epclextract::tests`: 20 passed
- native multi-file probe: exact four-comment/two-step output
- native missing-input probe: exit 6 with the expected Windows suffix
- `cargo test --locked --lib --quiet -- --test-threads=1`: 4,127 passed
- `cargo test --locked --bins --quiet`: all binary targets passed
- `cargo test --locked --test eprover_schedule --quiet`: 4 passed
- `cargo test --locked --test e_stratpar --quiet`: 1 passed
- `cargo test --locked --test executable_inventory --quiet`: 1 passed
- `cargo build --locked --release --bin epclextract`
- `cargo fmt --all -- --check`
- `cargo check --locked --all-targets`
- `cargo clippy --locked --all-targets -- -D warnings`
- C-source coverage: 492 source files covered by 266 unit docs
- Change Later wording: 269 Markdown files checked
- local Markdown links: 269 Markdown files checked
- regeneration preservation: manual sections preserved in 268 Markdown files
- `git diff --check`
