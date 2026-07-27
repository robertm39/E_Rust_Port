# Experiment 311: Reuse duplicate term-bank top shells

## Status

Complete and accepted for Bead `E_Rust_Port-j76.5.5`.

## Question

Can each `TermBank` retain a tiny arity-indexed pool of uniquely owned
temporary top cells rejected as duplicates, then reuse those cells for later
bank-local top copies, reducing the measured Rust allocation/drop gap without
changing canonical term identity, store topology, or proof behavior?

## Baseline

- Accepted source: commit `9351582c`.
- Matched Experiment 310 LUSK6 profile:
  8,306,398,955 Rust instructions versus 5,254,418,333 C instructions
  (`1.580841x`).
- Rust `TermBank::term_top_insert` has 2,479,632 calls and 1,955,273 duplicate
  cells on this workload. Its fat-LTO subtree is roughly 1.23 billion
  instructions versus about 567 million across the analogous C top-insert,
  store, tree/splay, and top-free owners.
- The accepted ten-case native aggregate is `1.1481929571x` C, above the
  normal `1.10x` completion target.

## Candidate

`TermBank` now owns a bounded pool of rejected top cells for arities zero
through two. Bank-local top-copy construction first tries the corresponding
pool and otherwise allocates normally. When `term_top_insert` finds an existing
canonical term, it merges the temporary cell's properties into the canonical
cell and offers the temporary cell back to the pool.

Recycling succeeds only when `Rc::get_mut` proves that no other strong handle
exists. The reset clears the function code, arity, properties, type and
binding/rewrite/store links, arguments, metadata counters, dates, and
demodulator state before the shell enters the pool. A retained external clone
therefore prevents recycling and remains unchanged. The pool holds at most
eight cells for each eligible arity, or 24 idle `TermCell` allocations per
bank. Larger arities continue to use normal allocation and destruction so the
pool never retains separately allocated argument buffers.

Ten bank-owned top-copy sites and the hot rewrite top-copy site use this
allocator. Canonical term cells, shared handles, variables, and term-store
splay links never enter the pool. Focused regressions require both successful
unique-shell reuse and non-mutation of an externally retained duplicate.

## Setup and exact commands

Focused measurement used dedicated worker
`e-rust-codex-260726-000531-036a` with Rust 1.97.1. The accepted parent was
commit `9351582c`; its three production files were transferred in
`accepted-source.tar` so candidate and parent could be built from the same
worker without relying on repository metadata or private GitHub access. The
effective worker commands are preserved verbatim in `remote_measure.sh` and
`remote_repeat.sh`; the controller invoked them as:

```powershell
.\linode-runner.ps1 up
try {
    .\linode-runner.ps1 sync
    .\linode-runner.ps1 exec -- bash `
      /opt/e-rust-port/source/experiments/2026-07-25-010-reuse-duplicate-top-shells/remote_measure.sh `
      /opt/e-rust-port/source `
      /opt/e-rust-port/artifacts/experiment-311
    .\linode-runner.ps1 exec -- bash `
      /opt/e-rust-port/source/experiments/2026-07-25-010-reuse-duplicate-top-shells/remote_repeat.sh `
      /opt/e-rust-port/source `
      /opt/e-rust-port/artifacts/experiment-311
}
finally {
    .\linode-runner.ps1 down
}
.\linode-runner.ps1 run
```

The final command created comprehensive worker
`e-rust-codex-260726-002526-1b72`, validated the accepted candidate, collected
`.artifacts/linode/260726-002526-1b72/`, and deleted the worker and firewall.

## Results

Focused Rustfmt, the 125 term-bank tests, and strict all-feature library
pedantic Clippy pass. Parent and candidate produce byte-identical LUSK6 proof
output with SHA-256
`b50bfb29a2fa3728792a422be2bd83a54c436fa1536c79b8d95c1c1f8c7f427d`,
empty stderr, and exit 0.

Matched Callgrind instructions fall from `8,306,006,831` to
`8,146,788,965`, a reduction of `159,217,866` instructions (`1.916900%`).
Relative to Experiment 310's matched C count of `5,254,418,333`, the
candidate ratio is `1.550464x`. The release binary grows by 2,512 bytes, from
8,282,072 to 8,284,584 bytes.

The profile confirms the intended mechanism:

- `default_cell_arity_alloc` calls fall from 2,497,335 to 542,128
  (`78.29%`);
- allocator self work falls from 162,168,993 to 91,781,838 instructions;
- duplicate `Rc::drop_slow` calls attributed to `term_top_insert` fall from
  1,955,273 to 58; and
- reset/drop bookkeeping moves into the insertion owner, but its added work is
  smaller than the removed allocation and reference-count destruction work.

Two independent 64-pair native blocks provide 128 alternating LUSK6 pairs.
Across all pairs, the candidate wins 75, reduces mean wall time by `0.393614%`,
paired mean wall time by `0.377946%`, median wall time by `0.362923%`, and
paired median wall time by `0.196260%`. CPU results agree: mean `-0.394953%`,
paired mean `-0.379309%`, median `-0.361832%`, and paired median
`-0.193761%`. Restricting each block to its final half yields 42 wins in 64
pairs and paired mean improvements of `0.495262%` wall and `0.496637%` CPU.
Every run has the same proof hash.

Fresh comprehensive run `.artifacts/linode/260726-002526-1b72/` validates the
exact accepted source:

- 4,407 Rust tests across 33 result groups, Rustfmt, strict
  all-target/all-feature pedantic Clippy, and the native release build pass;
- every binary and test target cross-compiles for Windows GNU x64;
- clean same-tree FOL and higher-order C references build and pass smoke
  checks;
- all 50 main cases have zero unexpected differences and one declared
  difference;
- all 216 support-tool cases have zero unexpected differences and 15 declared
  differences;
- all ten benchmark cases preserve behavior;
- smoke Callgrind records 9,610,254 Rust versus 7,590,630 C instructions; and
- the ten-case aggregate Rust/C wall ratio improves from `1.1481929571x` to
  `1.1326882018x`, a `1.3504%` relative reduction.

The lifecycle wrote `VALIDATION_COMPLETE` and `SUCCESS`, collected the complete
reports, and deleted its Linode and firewall.

Raw focused artifacts:

```text
.artifacts/experiments/2026-07-25-010-reuse-duplicate-top-shells/callgrind-instructions.txt
.artifacts/experiments/2026-07-25-010-reuse-duplicate-top-shells/callgrind-parent.out
.artifacts/experiments/2026-07-25-010-reuse-duplicate-top-shells/callgrind-candidate.out
.artifacts/experiments/2026-07-25-010-reuse-duplicate-top-shells/callgrind-parent-tree.txt
.artifacts/experiments/2026-07-25-010-reuse-duplicate-top-shells/callgrind-candidate-tree.txt
.artifacts/experiments/2026-07-25-010-reuse-duplicate-top-shells/native-lusk.csv
.artifacts/experiments/2026-07-25-010-reuse-duplicate-top-shells/native-lusk-2.csv
.artifacts/experiments/2026-07-25-010-reuse-duplicate-top-shells/native-summary.json
.artifacts/experiments/2026-07-25-010-reuse-duplicate-top-shells/native-summary-2.json
.artifacts/experiments/2026-07-25-010-reuse-duplicate-top-shells/native-summary-combined.json
.artifacts/linode/260726-002526-1b72/validation-summary.json
```

## Falsification checks and limits

- A cell is reset only when `Rc::get_mut` proves that the bank owns its sole
  strong handle; an externally retained duplicate must remain untouched.
- Only arities zero through two are eligible, matching the inline argument
  representations that avoid separately retained heap buffers.
- Each eligible arity retains at most eight shells, bounding each bank's
  additional idle storage to 24 `TermCell` allocations.
- Canonical fresh cells, term-store splay links, entry numbers, and shared
  handles are never recycled.
- Parent and candidate must produce byte-identical proofs and use the same
  worker, compiler, release profile, problem, and command line.
- Exact instructions are the first performance gate; native alternating pairs
  remain authoritative for acceptance.
- The first two controller attempts were setup-only failures before
  compilation: the transferred snapshot had no `.git` metadata, then a
  private GitHub fetch lacked credentials. The accepted-source archive removed
  both dependencies; neither failed attempt contributed measurements.
- Native gains are intentionally modest and noisy, so two independent blocks,
  aggregate and final-half statistics, exact proof hashes, and deterministic
  Callgrind work are all retained.
- The fresh aggregate remains `1.132688x`, above the normal `1.10x`
  completion target. This experiment closes one allocation boundary, not the
  performance epic.

## Decision

Accept. The bounded bank-local pool safely reuses only uniquely owned rejected
top shells, the focused tests falsify retained-handle mutation, exact proof and
compatibility behavior remain unchanged, deterministic work falls 1.92%, both
native blocks favor the candidate, and the comprehensive aggregate improves
1.35% relative. Main-prover performance parity remains open.
