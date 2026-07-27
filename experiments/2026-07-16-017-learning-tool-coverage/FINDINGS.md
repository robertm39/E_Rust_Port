# Expanded learning-tool interop coverage

## Status

Complete for Bead `E_Rust_Port-j76.1.2`. The permanent cases, harness coverage,
optimized Windows candidate checks, archived-C differential, and native-Linux
recursive-TSM benchmark have all run. The differential exposed one real
missing-input diagnostic mismatch; Rust now preserves C's pre-open `stat`
boundary, and the final 14-case tool comparison is exact.

## Added cases

The support-tool comparison matrix now includes five additional learning cases:

- `direct_examples/branching-protocol`: a 12-step protocol with two initial
  clauses, branching paramodulation references, shared external variables, a
  final empty proof step, and non-proof descendants under both negative-example
  controls;
- `direct_examples/missing-input`: an isolated-workdir missing-file diagnostic;
- `ekb_delete/drop-middle-example`: deletion from a four-example knowledge base
  with annotations shared across all four examples, a pattern owned only by the
  removed example, three retained payload files, and explicit absence checking;
- `tsm_classify/recursive-mixed`: 12 training and 12 test terms using a symbol
  index at depth three and recursive TSM construction, including nested mapped
  and previously unseen term shapes;
- `tsm_classify/empty-test-set`: the zero-node classification edge that produces
  a platform-dependent NaN spelling.

The functional-case metadata now supports `isolated_workdir` so missing-file
cases cannot accidentally resolve a file in the repository root.

## Cross-platform normalization

Normalization is deliberately narrow. It canonicalizes only:

- the known Linux/Windows file-not-found suffixes;
- the known Linux/Windows broken-pipe suffixes;
- NaN spellings only in the final `successes, ... percent` TSM summary field.

The stable program-authored diagnostic prefix, exit status, stdout/stderr
channel, all other output, generated files, and generated-directory effects
remain strict comparison fields. A unit test pairs Linux and Windows examples
for all three platform spellings.

## Candidate validation

The optimized Windows binaries were built with:

```text
cargo build --locked --release --bin direct_examples --bin ekb_delete --bin tsm_classify
```

`check_windows_candidates.py` then materialized the real harness cases in fresh
temporary workdirs and checked exit codes, required/absent output files, stable
semantic output snippets, and timeouts. All five passed. The stable output
details were:

| Case | Exit | Stdout bytes | Stderr bytes | Stdout SHA-256 |
| --- | ---: | ---: | ---: | --- |
| `direct_examples/branching-protocol` | 0 | 733 | 0 | `5337ab62a6c11bf2d59a6e7f450b37f1dc6c6b509faefd665c12819e98e7b919` |
| `direct_examples/missing-input` | 6 | 0 | 146 | empty SHA-256 |
| `ekb_delete/drop-middle-example` | 0 | 0 | 0 | empty SHA-256 |
| `tsm_classify/recursive-mixed` | 0 | 1,045 | 0 | `b4a9d0e41de9c7a794e9b781da10545f180454f02e6f1ccacdbeaf4335f54b99` |
| `tsm_classify/empty-test-set` | 0 | 80 | 0 | `9dd4c5cd75dc0b93ebdab1a3895e4db285fd5c9ed3b41fb4feac16d02b1511c8` |

The candidate checker requires the branching protocol to emit both Axiom and
Example sections, the recursive case to classify 12 terms, the empty-test case
to normalize to `<NAN>`, and the missing-input case to normalize to the
file-not-found equivalence class.

The Python harness passes all 25 unit tests and all three edited Python files
pass bytecode compilation.

## Final C/Rust differential

The archived reference environment later became available under WSL. The exact
command was:

```text
python3 tools/e-interop/e_interop.py compare-tools \
  --repo-root /mnt/c/Users/rober/Code/E_Rust_Port \
  --rust-windows-bin-dir /mnt/c/Users/rober/Code/E_Rust_Port/target/release \
  --tool direct_examples --tool ekb_delete --tool tsm_classify --timeout 30
```

The first run, artifact
`.artifacts/e-compare/20260716-171657-148910-tools`, found one mismatch:
`direct_examples/missing-input` had the same exit status and normalized OS
suffix, but Rust reported `Cannot open file` where C's `InputOpen` first reports
`Cannot stat file`. Rust now routes named direct-example inputs through the
already ported `input_open` boundary before scanner construction. Permanent
unit coverage pins the `stat` diagnostic and the existing output-file-before-
input-failure side effect.

The final report is
`.artifacts/e-compare/20260716-172050-819680-tools`: all 14 cases match, including
all five additions in this experiment. The known upstream `ekb_ginsert`
heap-corruption abort is unchanged and is not conflated with these cases.

## Native-Linux recursive-TSM benchmark

`benchmark-recursive-tsm-wsl.sh` runs the exact `recursive-mixed` input in
alternating C/Rust order. Each of 11 batches launches the tool 200 times with
`IndexSymbol`, depth three, and recursive TSM construction. The script stages
both executables together on WSL's native filesystem before timing; an initial
exploratory run with Rust executed from `/mnt/c` was rejected because mounted-
filesystem executable startup dominated the wall result.

The accepted raw data is
`.artifacts/learning-tool-coverage/recursive-tsm-native-batches.csv`:

| Implementation | Median wall / 200 | Median user | Median system | Median CPU | Peak RSS |
| --- | ---: | ---: | ---: | ---: | ---: |
| Archived C | 0.87 s | 0.24 s | 0.12 s | 0.36 s | 3,200 KiB |
| Rust | 1.05 s | 0.29 s | 0.13 s | 0.42 s | 3,200 KiB |

Rust/C is `1.207x` by batch wall time and `1.167x` by median batch CPU. This is
an exact small-workload/startup-sensitive performance baseline, not evidence of
comparable performance under the repository's `1.10x` target. The residual gap
is tracked as Bead `E_Rust_Port-j76.5.1` so completion of the requested coverage
does not hide it.

## Decision

Retain and publish the expanded harness coverage, the exact C pre-open `stat`
fix, the reproducible recursive corpus, and the unbiased benchmark harness.
Close `E_Rust_Port-j76.1.2`: every coverage item named by the migrated pending
work now has executable evidence. Continue the measured recursive-TSM
performance work in follow-up Bead `E_Rust_Port-j76.5.1`.
