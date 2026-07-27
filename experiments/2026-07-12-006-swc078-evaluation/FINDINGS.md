# SWC078 evaluation and ownership profile

## Question

Why did Rust exhaust the canonical SWC078 resource limit after matching C's
selected-clause sequence, and did the apparent next clause differ in HCB
evaluation or FIFO order?

## Reproduction

The experiment uses the cached C reference at upstream commit
`17026b1bfe61aaf223cfaae54947c8d2679c31a0` and bundled `SWC078-1.p`.

```sh
cargo build --release --locked --bin eprover
bash experiments/2026-07-12-006-swc078-evaluation/run-prefix-timing.sh
bash experiments/2026-07-12-006-swc078-evaluation/run-prefix-compare.sh
bash experiments/2026-07-12-006-swc078-evaluation/run-callgrind.sh
```

`trace-c-target-eval.gdb` records the C target evaluation. Generated traces and
profiles are under `.artifacts/experiments/2026-07-12-006-swc078-evaluation/`;
the B-tree, SipHash, custom-hash-only, and final profiles are retained.

## Findings

- The prior comparator parsed Rust's `% Failure:` final-state dump as selected
  clauses. After stopping at failure/SZS boundaries, all 6,367 clauses selected
  by pre-optimization Rust match C's prefix; C continues to 8,499. There was no
  demonstrated ordinal-6,368 divergence.
- C final-state clause `i_0_18945` has runtime id
  `-9223372036854756863`. C and Rust assign the same FIFO value and evaluation
  vector: `[50:3450:16818][40:4280:16818][40:16620:16818]`
  `[30:3450:16818][50:255:16818]`. Generation, evaluation, and FIFO ordering
  do not explain the resource boundary.
- The original Callgrind run executes 16,196,105,688 instructions. Rebuilding
  two `BTreeSet<ClauseDerivationRef>` liveness snapshots at every selection is
  the largest self cost; the dead set is redundant because queries only test
  whether a parent is live.
- A preallocated `HashSet` with an integer-key hasher lowers the profile to
  12,336,511,699 instructions. A retained SipHash profile showed hashing itself
  consuming about 20%, falsifying the standard hasher for this hot path.
- The next inclusive profile showed `ObjTree<SubtermOcc>` cloning whole
  payloads merely to remember a root. `SubtermOcc` owns maps containing clause
  copies, so ordinary store/find/extract operations recursively cloned indexed
  clauses. C stores object pointers and never copies those payloads.
- `ObjTree` now stores one typed `Rc<T>` per ordered payload and shares it with
  its recent-root reference. Extraction uses `Rc::try_unwrap`; cloning is only
  a defensive fallback for an unexpected remaining reference. A clone-count
  regression pins the ordinary zero-payload-clone path.
- The final capped profile executes 7,556,572,729 instructions: 53.3% fewer
  than the original and 38.7% fewer than the hash-only candidate. Five native
  timings give Rust 0.96-1.07 seconds (median 1.01) versus C 0.23-0.28 seconds
  (median 0.25). Before `Rc` sharing, Rust's median was about 2.33 seconds; the
  original representative run was 2.90 seconds.
- The canonical uncapped 60-second comparison exits successfully in both
  implementations and normalizes the same complete sequence of 8,499 selected
  clauses. SWC078 proof behavior is restored without changing search order.

## Falsification And Limits

- Matching every target evaluation cell rules out HCB arithmetic and FIFO
  assignment as the cause at the former boundary.
- Matching the complete selected-clause sequence before and after the ownership
  changes rules out an ordering/search shortcut as the source of the speedup.
- `Rc` avoids deep payload copies but does not recreate C's splay topology;
  `ObjTree` still uses `BTreeSet` ordering plus explicit recent-root tracking.
- Parent liveness is still rebuilt at selection boundaries. The final profile
  attributes 9.55% of instructions to hash insertion, so maintained liveness
  metadata or stable arena handles remain future work.
- The capped workload remains about 4.0 times C's wall time. This is a major
  improvement, not performance parity; repository-wide results are recorded in
  `docs/rust-port-status.md`.
