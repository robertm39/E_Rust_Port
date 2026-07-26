# Experiment 310: Matched C/Rust LUSK profile

## Status

Complete diagnostic experiment for Bead `E_Rust_Port-j76.5.5`; production
source is unchanged.

## Question

Which source-shaped proof-search functions account for the remaining native
long-proof gap after static autoschedule tables and candidate-only term-store
garbage collection are accepted?

## Baseline

Accepted source is commit `1cae0c68`. Comprehensive run
`.artifacts/linode/260725-231530-96af/` is behavior-exact across the maintained
main, support-tool, and benchmark matrices. Its ten-case aggregate Rust/C
wall-time ratio is `1.1481929570688398x`, above the normal `1.10x` completion
target. The largest repeatable non-resource benchmark gaps are:

- `LUSK6.lop`: `1.3793x`;
- `LUSK6ext.lop`: `1.3854x`; and
- `LUSK3.p`: `1.6729x`, although its absolute runtime is only about 12 ms.

The accepted Rust LUSK6 Callgrind profile from Experiment 308 records
`8,305,759,465` instructions. No matched C LUSK6 Callgrind artifact exists, so
individual Rust hot functions cannot yet be distinguished from proof-search
work that is equally expensive in C.

## Setup and exact commands

A fresh Ubuntu 24.04 worker
`e-rust-codex-260725-234436-f061` used Rust 1.97.1 and Callgrind 3.22.0.
The uploaded accepted source was rooted at commit `1cae0c68`; the profiling
snapshot SHA-256 was
`9782dbb359b4cf06daf31fe667837518423e83c61988c41c856920fc4d3ad3f3`.
The later script-only correction changed the snapshot hash to
`af20d0dd1b1b68477155686c5fbbba41f27692df88e639334f3a7ef3d0a3ea32`
without changing either profiled executable.

The focused lifecycle was:

```powershell
.\linode-runner.ps1 up
.\linode-runner.ps1 sync
.\linode-runner.ps1 exec -- bash `
  /opt/e-rust-port/source/experiments/2026-07-25-009-lusk-c-rust-profile/remote_profile.sh `
  /opt/e-rust-port/source `
  /opt/e-rust-port/artifacts/experiment-310
.\linode-runner.ps1 down
```

The remote script built the accepted default-feature Rust release executable
and a clean FOL C reference from unchanged upstream commit
`17026b1bfe61aaf223cfaae54947c8d2679c31a0`, then profiled both with:

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=callgrind-LABEL.out \
  BINARY eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```

Both invocations ran on the same worker, problem bytes, command line, and
Callgrind version. The worker and temporary firewall rule were deleted after
the artifacts were downloaded.

## Results

Both executables exited zero and emitted byte-identical 378-byte proof output
with SHA-256
`b50bfb29a2fa3728792a422be2bd83a54c436fa1536c79b8d95c1c1f8c7f427d`.
The proof reports `Unsatisfiable` and the accepted 4,873 processed clauses.

| Executable | Instructions | Relative to C |
| --- | ---: | ---: |
| C reference | 5,254,418,333 | 1.000000x |
| Rust | 8,306,398,955 | 1.580841x |

The matched call trees prevent several misleading self-cost comparisons:

- C leaves its PD-tree traversal maps, term traversal, matching, lookup, and
  backtracking helpers out of line. `PDTreeFindNextIndexedLeaf` therefore has
  1,119,601,083 self instructions but a 1,952,048,584-instruction subtree.
  Rust folds most analogous work into
  `search_next_matching_occurrence_impl::<true>`; its corresponding cursor
  subtree is 1,651,692,727 instructions. The Rust cursor is about 300 million
  instructions cheaper at this matched boundary and is not the current gap.
- C's substitution-normalization subtree is 271,592,163 instructions:
  192,675,144 in `SubstNormTerm`, 59,501,119 in `TermDerefAlways`, and about
  19.4 million in fresh-variable acquisition. Rust spends about 553 million:
  439,641,877 in `Substitution::norm_term` plus about 114 million in
  fresh-variable, stack-growth, and binding callees. This remains about a
  281-million-instruction differential.
- Fat LTO folds Rust's term-tree insertion and splay into
  `TermBank::term_top_insert`. Its 909,604,136 self instructions plus about
  319 million in visible hash, link, and duplicate-drop callees form a roughly
  1.23-billion-instruction subtree. The analogous C
  `tb_termtop_insert`/`TermCellStoreInsert`/`TermTreeInsert`/splay/free work is
  roughly 567 million instructions. The 2,479,632 Rust calls and 2,548,243 C
  store calls are close enough that the approximately 660-million difference
  is representation and ownership cost, not extra proof-search work.
- The recursive replacement boundary reflects the same insertion gap:
  `insert_repl_for_problem::<true>` reaches 2,232,394,091 inclusive
  instructions in Rust versus 949,217,756 for C `TBInsertRepl`.

The historical audit closes the tempting local edits:

- Experiment 238 already tested first-order-only normalization dereferencing.
  It saved 19.7 million instructions but reproducibly regressed native
  throughput and was rejected.
- Experiments 218 and 297 tested dense and retained-entry fresh-variable
  lookups. Their deterministic wins reversed or failed to reproduce natively.
- Experiment 296 removed release `VarBank::var_assert_alloc` checks. Its local
  instruction win also reversed in native measurement.
- Experiment 300 retested forced `Term::arguments` inlining under fat LTO and
  rejected it on both independent stable native halves.
- Experiments 242, 243, 254, and 265 rejected consuming store insertion,
  accumulated metadata, batched tree-link writes, and arena-backed term-store
  variants. Those ownership/layout changes must not be repeated unchanged.

Raw evidence:

```text
.artifacts/experiments/2026-07-25-009-lusk-c-rust-profile/callgrind-reference.out
.artifacts/experiments/2026-07-25-009-lusk-c-rust-profile/callgrind-candidate.out
.artifacts/experiments/2026-07-25-009-lusk-c-rust-profile/callgrind-reference-self.txt
.artifacts/experiments/2026-07-25-009-lusk-c-rust-profile/callgrind-candidate-self.txt
.artifacts/experiments/2026-07-25-009-lusk-c-rust-profile/callgrind-reference-tree.txt
.artifacts/experiments/2026-07-25-009-lusk-c-rust-profile/callgrind-candidate-tree.txt
.artifacts/experiments/2026-07-25-009-lusk-c-rust-profile/instruction-totals.txt
.artifacts/experiments/2026-07-25-009-lusk-c-rust-profile/output-sha256.txt
```

## Falsification checks and limits

- C and Rust must use the same problem bytes, options, worker, and reference
  commit.
- Status, stdout, and stderr must match the accepted proof behavior.
- Total instructions and function rankings must come from the same Callgrind
  version and fresh worker.
- A useful next candidate must target a Rust/C differential, not merely a hot
  function that consumes comparable work in both implementations.
- This experiment is diagnostic; it accepts no implementation change by
  itself.
- The initial script mixed Valgrind's own diagnostics into program stderr.
  Both profiles and exact stdout/status evidence were valid, but the final
  empty-stderr postcheck failed. The retained script now sends Valgrind
  diagnostics to a separate `--log-file`; `analyze_profiles.sh` regenerated
  the annotations, hashes, and totals from the complete raw profiles.
- Fat-LTO symbol ownership prevents exact one-to-one self-function
  comparisons. The conclusions above use matched inclusive boundaries and
  explicitly identify approximate aggregates.

## Decision

Accept the profile as diagnostic evidence and leave production source
unchanged. Do not target the already faster matched PD-tree cursor or repeat
the rejected local substitutions and inlining ablations. The next experiment
should attack the untested temporary top-cell lifetime cost within the
term-bank owner while preserving canonical term identity and bounding retained
memory; substitution normalization remains the second-priority differential.
