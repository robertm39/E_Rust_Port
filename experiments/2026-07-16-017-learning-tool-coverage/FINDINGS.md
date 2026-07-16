# Expanded learning-tool interop coverage

## Status

This is an in-progress expansion for Bead `E_Rust_Port-j76.1.2`. The permanent
case definitions, harness unit coverage, and optimized Windows candidate checks
are complete. The new cases have not yet run against the archived Linux C tools
because this desktop environment has no installed WSL distribution or locally
executable C reference-tool cache. The Bead therefore remains open.

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

## Reference-environment limitation

`wsl -l -q` reports no installed distributions. Attempting the configured
`Ubuntu-24.04` distro returns `Wsl/Service/WSL_E_DISTRO_NOT_FOUND`. The Windows
`~/.cache/e-rust-port` tree has a copied source tree but no `reference.json` or
cached executable tools. Consequently neither the new C/Rust differential nor
native-Linux TSM performance comparison can be produced honestly in this
environment.

Once the archived reference environment is available, run:

```text
python3 tools/e-interop/e_interop.py compare-tools \
  --repo-root /mnt/c/Users/rober/Code/E_Rust_Port \
  --rust-windows-bin-dir /mnt/c/Users/rober/Code/E_Rust_Port/target/release \
  --tool direct_examples --tool ekb_delete --tool tsm_classify --timeout 30
```

Then add an interleaved native-Linux benchmark for
`tsm_classify/recursive-mixed`. The known upstream `ekb_ginsert` heap-corruption
abort is unchanged and is not conflated with these five new exact-comparison
cases.

## Decision

Retain and publish the expanded harness coverage, but do not close
`E_Rust_Port-j76.1.2` until the new C/Rust differential and TSM performance run
are available. This checkpoint is useful independently: it makes the intended
corpora and platform equivalences executable and prevents the missing reference
environment from erasing completed candidate-side work.
