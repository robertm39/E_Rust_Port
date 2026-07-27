# Experiment 304: Linux fallible clause admission

## Status

Complete and accepted for Bead `E_Rust_Port-j76.5.3`.

## Question

Can Linux reuse the existing transactional clause-page and evaluation-index
capacity preflight so BOO020 and SWV851 leave through the normal saturation
`ResourceOut` path instead of Rust's infallible allocation abort?

## Candidate

- Compile `ClauseSet`'s existing fallible page, evaluation-tree, and
  evaluation-object reservations on Linux as well as non-Linux targets.
- Preserve Linux's asynchronous `SIGXCPU` behavior and disable the non-Linux
  one-second proactive page-deadline lookahead on Linux.
- Honor the resulting cooperative time latch during paramodulation,
  generated-clause admission, clause evaluation, and queue transfer so no
  later phase attempts another large allocation after a reservation fails.
- Match C's incremental `TermLRTraverseNext` shape in `PDTreeMatchPrefix`:
  traverse a term directly through a reusable tree-owned stack instead of
  materializing a complete `Vec<PrefixToken>` and walking it a second time.
- Propagate rejected paramodulant insertion directly from all three generation
  loops, avoiding an atomic deadline read on every iteration while still
  stopping immediately after a fallible admission rejects a clause.

No memory limit is raised, clause layout and capacity growth are unchanged,
and the ordinary allocator cache policy is unchanged.

## Setup

- Focused ephemeral Linode run: `260725-172549-7e20`.
- Final synced source snapshot:
  `fe6837cee67573587236cd477599b2bca5dd9f57b8dbcb5ef2b64b2ea3d34aa7`.
- Rust 1.97.1, Ubuntu 24.04, locked default-feature fat-LTO release profile.
- Problems and flags:
  - `eprover/EXAMPLE_PROBLEMS/SMOKETEST/BOO020-1.p`
  - `eprover/EXAMPLE_PROBLEMS/TPTP/SWV851-1.p`
  - `--auto --silent --cpu-limit=60 --memory-limit=2048 --detsort-rw`
    `--detsort-new --proof-object=1`
- Focused final artifacts:
  `.artifacts/experiments/2026-07-25-003-linux-fallible-clause-admission/experiment-304-lean/`.
- Diagnostic pre-streaming artifacts:
  `.artifacts/experiments/2026-07-25-003-linux-fallible-clause-admission/experiment-304/`.
- Comprehensive Linode run:
  `.artifacts/linode/260725-181459-0a58/`.

Reusable harnesses:

- `remote_resource_check.py` from Experiment 303 runs the two exact resource
  cases and records exit, hashes, wall time, and peak child RSS.
- `remote_gdb_swv.sh` runs SWV851 under external GDB while passing `SIGXCPU`
  through to the program. External GDB is necessary because
  `RUST_BACKTRACE=full` itself tried to allocate after the original failure.
- `remote_profile.sh` records a default-feature LUSK6 Callgrind profile,
  proof hash, binary hash/size, and `/usr/bin/time -v` metadata.

## Results

### Allocation diagnosis

The first candidate fixed BOO020 but SWV851 still aborted on a 2,048-byte
allocation:

| Problem | Exit | Wall (s) | Peak child RSS (KiB) | ResourceOut |
| --- | ---: | ---: | ---: | --- |
| BOO020-1.p | 8 | 28.396 | 1,902,164 | yes |
| SWV851-1.p | -6 (`SIGABRT`) | 77.559 | 1,997,336 | no |

GDB placed the failed `RawVec::grow_one` under:

```text
term_lr_traverse_code
PdTree::match_prefix
TermWeightExtension::term_weight
conjecture_term_prefix_weight_wfcb_compute_with_bank
hcb_clause_evaluate_with_bank
proof_state_eval_clause_set
proof_state_insert_new_clauses_impl
```

This falsified the narrower hypothesis that transactional `ClauseSet`
reservation alone covered SWV851. C consumes `TermLRTraverseNext` incrementally;
Rust was allocating a complete prefix-token vector inside heuristic evaluation.
The final candidate uses one reusable `Vec<Term>` traversal stack and updates
the match state as each term is visited.

### Focused resource result

| Problem | Exit | Wall (s) | Peak child RSS (KiB) | ResourceOut |
| --- | ---: | ---: | ---: | --- |
| BOO020-1.p | 8 | 28.449 | 1,902,184 | yes |
| SWV851-1.p | 8 | 36.516 | 1,997,336 | yes |

Both cases emit the normal `Resource limit exceeded (time)` message and SZS
`ResourceOut`; neither reaches Rust's allocation-error handler.

### Exact performance

The deterministic workload is upstream `LUSK6.lop` with
`--auto --silent --cpu-limit=600 --memory-limit=2048 --detsort-rw
--detsort-new`.

| Variant | Instructions | Native binary bytes |
| --- | ---: | ---: |
| Fresh accepted parent | 8,367,710,262 | 8,555,520 |
| Final candidate | 8,389,058,378 | 8,563,248 |
| Delta | +21,348,116 (+0.255125%) | +7,728 (+0.090328%) |

The first direct-failure-propagation version retired `8,389,495,558`
instructions. Returning `ClauseSet::insert` failure directly from the three
paramodulation loops removes 437,180 instructions and 224 binary bytes.

The final candidate and parent produce the exact 378-byte proof SHA-256
`b50bfb29a2fa3728792a422be2bd83a54c436fa1536c79b8d95c1c1f8c7f427d`.
The narrowly bounded 0.255% exact-work cost is accepted for converting two
hard allocator aborts into compatible resource outcomes. The comprehensive
ten-case native benchmark remains far outside project-wide performance parity
at a `2.667x` aggregate Rust/C wall ratio; this experiment does not claim to
close that broader gap.

### Validation

Focused validation passed:

- Rustfmt;
- 53 paramodulation tests;
- 56 clause-set tests;
- 42 PD-tree tests.

The comprehensive run then passed:

- 4,398 Rust tests across the library and all test targets;
- strict pedantic Clippy;
- documentation and default/all-feature Linux builds;
- Windows GNU x64 test-target and release cross-compilation;
- clean disposable upstream FOL and HO C builds;
- ten-case native timing and Rust/C smoke Callgrind.

The 50-case main matrix falls from three unexpected mismatches to one.
BOO020 and SWV851 both match C exactly on normalized output, status, and exit
code. The remaining mismatch is the pre-existing `SWB008+1.p` normalized
stdout difference after both implementations return `ResourceOut`. The
declared `sledgehammer.p` difference remains one expected difference.

The 216-case support-tool matrix retains 33 unexpected and eight expected
differences. Those are independent pre-existing port work and keep the
top-level comprehensive runner nonzero.

## Falsification checks and limits

- The focused commands use the same optimized profile, problems, and exact
  resource flags as the maintained main matrix.
- The pre-streaming GDB run proves SWV851's remaining failure was outside
  clause storage, so the final fix covers the observed allocation owner rather
  than relying on altered timing.
- A regression compares streaming prefix matching with the existing
  materialized-code reference and verifies the scratch stack is empty after
  use while retaining capacity for the next traversal.
- The complete matrix confirms BOO020 and SWV851 on a fresh worker rather than
  only on the diagnostic host.
- Linux intentionally retains asynchronous `SIGXCPU`; it does not use the
  non-Linux one-second proactive clause-page lookahead.
- If no CPU limit is active, a failed reservation cannot currently latch the
  cooperative time deadline. General memory-only exhaustion remains a separate
  control-flow boundary and is not claimed closed here.
- BOO020 now stops before C's 60-second CPU boundary because Rust exhausts its
  available reservation headroom sooner. Status, exit code, normalized output,
  and configured resource semantics match; the different wall time reflects
  the still-larger Rust working set.

## Decision

Accept. Linux now uses the same transactional evaluated-clause admission
boundary as other targets, generation observes rejected insertions, and
`PDTreeMatchPrefix` follows C's incremental traversal shape with reusable
storage. Together they eliminate both maintained allocator aborts without
raising memory limits or changing proof output. Keep Bead `E_Rust_Port-j76.5.3`
open for the remaining SWB008 main mismatch and the broader performance gap.
