# Rejected redundant output-flag deletion removal

## Question

After `Term::top_copy_without_args` masks source properties down to
`TP_PRED_POS | TP_IS_DB_VAR`, can it omit the following explicit
`TP_OUTPUT_FLAG` deletion without changing behavior or whole-program
performance?

## Setup

- Parent source: commit `2f818fa1` (`Record rejected direct term top copy
  construction`), whose executable source remains accepted Experiment 223.
- Candidate: delete only `copy.del_prop(TP_OUTPUT_FLAG)` from
  `top_copy_without_args`; retain the established default allocation and every
  metadata setter.
- Deterministic workload: upstream `LUSK6.lop` under WSL Callgrind with
  `--auto --silent --cpu-limit=600 --memory-limit=2048 --detsort-rw
  --detsort-new`.
- Parent profile:
  `.artifacts/experiments/2026-07-22-223-direct-rewrite-term/rust-callgrind-direct-rewrite-term.out`.
- Candidate profile:
  `.artifacts/experiments/2026-07-22-225-remove-output-flag-delete/rust-callgrind-remove-output-flag-delete.out`.
- C reference: archived FOL build from upstream commit
  `17026b1bfe61aaf223cfaae54947c8d2679c31a0`, profiled at 5,254,361,329
  instructions.

The removal is semantically sound because the mask cannot retain
`TP_OUTPUT_FLAG`. The experiment tests whether LLVM already exploits that fact
and whether the smaller source shape improves the final executable.

## Result

The candidate preserves the exact 4,873-processed-clause LUSK6 proof but
retires 10,498,629,565 instructions. This is 745,269 above the
10,497,884,296-instruction parent, a 0.007099% regression. The hypothetical
C/Rust ratio worsens from 1.997937 to 1.998079.

The two directly comparable `top_copy_without_args` call edges improve by only
10,635 and 15,773 instructions, 26,408 combined across 1,435,164 calls. That
tiny change confirms the redundant get/mask/set sequence was already
effectively optimized away. Whole-program code layout reverses it:
`TermTree::insert` rises 513,919 instructions or 0.078020%, `term_top_insert`
under recursive rewriting rises 106,232, and the enclosing rewrite and
replacement paths rise 193,789 and 254,403 inclusive instructions. The PD-tree
cursor, substitution normalizer, and `insert_repl` exclusive work reproduce
exactly.

## Validation

- All 18 focused term-cell tests pass with the candidate, including the
  assertion that copied terms never retain `TP_OUTPUT_FLAG`.
- Strict all-feature library pedantic Clippy and formatting pass.
- The candidate produces the exact LUSK6 proof and exits zero under Callgrind.
- Source is restored byte-for-byte; all 18 focused tests and formatting pass
  after restoration.
- Native and compatibility matrices were skipped after the deterministic gate
  failed.
- The vendored C checkout was not modified.

## Decision

Reject the one-line removal and preserve the C-shaped explicit flag deletion.
It is semantically redundant and almost entirely optimized out, while the
resulting code-layout change regresses the whole prover. Accepted Experiment
223 remains the baseline at 10,497,884,296 instructions, or 1.997937 times C.

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=rust-callgrind-remove-output-flag-delete.out \
  target-wsl-225-remove-output-flag-delete/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```
