# Verbose progress ownership closure

## Status

Completed for Bead `E_Rust_Port-j76.2.29`. The default-feature Rust executable
now emits the represented parser, preprocessing, and proof-control progress
stream with the same verbosity thresholds, text, ordering, and stream ownership
as the pinned default C build. The vendored C checkout remains unchanged.

## Implemented ownership

Executable progress now covers C's output and input lifecycle, including nested
TPTP/TSTP includes; level-two predefined and parsed type insertion/declaration
events; specification and `$distinct` completion; SInE seed counts; relevance
extraction; conjecture negation; clausification and its actual term-GC recovery
counts; CNF and clausal-preprocessing completion; ordering generation;
weight-function and heuristic administration; proof-state initialization; and
final output close.

The parser records include lifecycle events while it recursively owns each
included scanner. Type-bank events are drained at those boundaries, so type
declarations remain between the included file's `Opened` and `Closing` lines as
they are in C. GSinE returns the seed count already computed by the selector;
the executable reports that value without a duplicate traversal.

Normal non-verbose runs keep the ordinary proof-state/type-bank allocation path.
Include event strings are constructed only when the global nonzero verbosity
gate is active, and type-event collection is enabled only above level one.

## C scope audit

The live executable term-ordering choices emit the ordinary precedence and KBO
weight-generation lines. C's `OrderFindOptimal` progress branch remains
upstream-dormant: the only `--term-ordering` parser arm that would select
`Optimize` is commented out in `PROVER/eprover.c`, consistent with the prior
1,972-candidate autoselection audit in experiment 073. Rust retains its reusable
optimizing implementation and start-message helper, but no executable-only
output was invented for a C-unreachable route.

Signal/resource-limit, proof-result, proof-object, comment-prefix, and configured
output ownership are covered by the narrower retained experiments 107, 105,
106, and 108 rather than duplicated here.

## Fresh C/Rust matrix

[`compare_verbose_progress.py`](compare_verbose_progress.py) runs both optimized
executables with separate stdout, stderr, and configured-output capture. It
normalizes only native/WSL spelling of the input, recursive-include, and output
paths. The retained [`reference.json`](reference.json) is exact in 9/9 cases:

- verbosity levels one, two, and negative-one;
- named GSinE seed reporting;
- recursive include lifecycle and interleaved type events;
- manual LPO ordering generation;
- relevance pruning;
- syntax-only early completion; and
- configured output-file routing.

The level-two baseline compares 91 stderr lines, while the recursive-include
case compares 95. The configured-output case independently matches 91 stderr
lines, the stdout side channel, and the configured file.

The retained reference has SHA-256
`5FC54B443B8C520F6C7FF42D5C46D9CA31E811B3DBCB67F9945FCDCE0811E6FC`.
The pinned C executable has SHA-256
`DC183EDAFDD6779324EEE8131E3EDED40FD6127DEAF6B5627C41E30D62034F4B`;
the compared default-feature Rust executable has SHA-256
`31CF617FAC3F74A8ED3019EE6C987CB6D77A3791DC8DC0F2B5E0AD82E30FEF5D`.

No throughput benchmark is warranted for this reporting slice: ordinary runs
retain the non-recording allocation path, and the new work is deliberately
gated behind requested verbose output.

## Reproduction

```powershell
cargo build --locked --release --bin eprover `
  --target-dir target\default-reference

C:\Users\rober\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe `
  experiments\2026-07-18-109-verbose-progress-ownership\compare_verbose_progress.py `
  --c-exe /home/rober/.cache/e-rust-port/bin/17026b1bfe61aaf223cfaae54947c8d2679c31a0/fol/eprover `
  --rust-exe target\default-reference\release\eprover.exe `
  --output target\verbose-progress-reference-check.json `
  --expected experiments\2026-07-18-109-verbose-progress-ownership\reference.json
```
