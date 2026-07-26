# Experiment 313: Split hot term-tree links

## Status

Complete and accepted for Bead `E_Rust_Port-j76.5.5`. The broader performance
target remains open.

## Question

Can `TermCell` keep the compact accepted 152-byte layout while moving only
the hot intrusive left/right splay links from the shared metadata `RefCell`
into two `Cell<Option<Term>>` slots, removing dynamic borrow bookkeeping
without changing ownership, comparison keys, tree topology, or proof
behavior?

## Baseline

- Accepted parent: commit `732f9f35`.
- Corrected Experiment 312 matched LUSK6 profile:
  `8,068,126,076` Rust instructions versus `5,254,418,333` C instructions
  (`1.535494x`).
- `TermBank::term_top_insert` retains a roughly
  `1,208,481,641`-instruction visible subtree:
  `1,042,414,518` self instructions plus the visible hash, link-setter,
  duplicate-drop, applied-variable, and link-clear callees.
- The representative line attribution in Experiments 216 and 239 assigns
  about 70 million instructions to `Cell`/`RefCell` operations within
  term-tree insertion. Earlier candidates rejected raw non-owning tails,
  arena indexing, batched paired writes, and consuming insertion; none tested
  a layout-neutral split of the two hot owning links.

## Candidate

The cold binding, rewrite-replacement, and type handles remain grouped behind
one `RefCell<TermLinks>`. The left and right intrusive tree handles become
separate pointer-sized `TermTreeLink(Cell<Option<Term>>)` fields. Reading a
link temporarily takes the owned handle, clones the returned handle, and
restores the original before returning; setters and ownership transfers use
`Cell::set` and `Cell::take`. The wrapper has an opaque, mutation-free
`Debug` representation so formatting cannot temporarily detach store links
or recurse through the intrusive tree.

The 64-bit layout regression continues to require a 152-byte `TermCell`.
Focused tests retain read, set, take, topology, hashing, store-accounting, and
bank-insertion coverage.

## Setup and exact commands

Final focused validation and measurement used dedicated worker
`e-rust-codex-260726-033732-cf85` with Rust 1.97.1. The uploaded source
snapshot SHA-256 was
`2e4ad5f424f14412eb98a87651da14d9e13cb8b1dd9a8ad875c08508eca18f9a`;
the exact candidate `termtypes.rs` SHA-256 was
`65f2b5d5bbd69f1b01f4737d547a72939bc1c4595f6e797cd2a76de9d96f3a65`.
The accepted parent was commit `732f9f35`.

Exact-source comprehensive validation used fresh worker
`e-rust-codex-260726-035420-af8c`. Its downloaded artifact is:

```text
.artifacts/linode/260726-035420-af8c/
```

Focused validation and measurement used `remote_measure.sh` and
`remote_repeat.sh`. The controller lifecycle was:

```powershell
git archive -o accepted-source.tar HEAD src/terms/termtypes.rs
.\linode-runner.ps1 up
try {
    .\linode-runner.ps1 sync
    .\linode-runner.ps1 exec -- bash `
      /opt/e-rust-port/source/experiments/2026-07-25-012-split-term-tree-links/remote_measure.sh `
      /opt/e-rust-port/source `
      /opt/e-rust-port/artifacts/experiment-313
    .\linode-runner.ps1 exec -- bash `
      /opt/e-rust-port/source/experiments/2026-07-25-012-split-term-tree-links/remote_repeat.sh `
      /opt/e-rust-port/source `
      /opt/e-rust-port/artifacts/experiment-313
}
finally {
    .\linode-runner.ps1 down
}
```

## Falsification criteria

- `TermCell` must remain 152 bytes on 64-bit targets.
- Existing left/right reads must leave the stored owning link intact.
- Existing insertion, extraction, and splay topology tests must pass.
- Parent and candidate must produce byte-identical LUSK6 proof output.
- Exact instructions must improve at the intended term-tree owner.
- Alternating native measurements must confirm that any instrumented gain is
  not a code-layout throughput reversal.

## Results

Final Rustfmt, strict all-feature library pedantic Clippy, 18 term-cell tests,
five term-tree tests, seven term-store tests, and 125 term-bank tests pass.
The layout regression confirms that `TermTreeLink` is one pointer wide,
`RefCell<TermLinks>` falls from 48 to 32 bytes, and `TermCell` remains 152
bytes on 64-bit targets. The same regression formats a linked term through
the opaque debug representation and proves that both owning links remain
installed afterward.

Parent and candidate produce byte-identical LUSK6 proof output with SHA-256
`b50bfb29a2fa3728792a422be2bd83a54c436fa1536c79b8d95c1c1f8c7f427d`,
empty stderr, and exit zero.

Matched Callgrind instructions fall from `8,068,020,467` to
`7,972,511,554`, a reduction of `95,508,913` instructions (`1.183796%`).
Relative to Experiment 310's matched C count of `5,254,418,333`, the
candidate ratio is `1.517297x`. The release executable shrinks by 14,008
bytes, from 8,285,992 to 8,271,984 bytes.

The visible `term_top_insert` owner boundary changes from
`1,256,143,865` to `1,162,534,883` instructions, a reduction of
`93,608,982` (`7.4521%`) that accounts for `98.01%` of the whole-program
improvement. The parent exposes 48,880,375 instructions in duplicate
`TermLinks` drop glue, 41,730,120 in `set_left_son`, 41,536,380 in
`set_right_son`, and 753,802 in `clear_tree_links`; those outlined dynamic
borrow/link paths disappear into the smaller candidate boundary. All
unrelated leading PD-tree and substitution-normalization owners reproduce.

Two final 64-pair native blocks provide 128 alternating LUSK6 pairs. The
candidate wins 94 pairs, and every run has the exact proof hash. Across all
pairs:

- wall mean, median, paired mean, and paired median improve by `0.818993%`,
  `0.738226%`, `0.813814%`, and `0.728615%`;
- CPU mean, median, paired mean, and paired median improve by `0.818095%`,
  `0.739353%`, `0.812919%`, and `0.727836%`.

Restricting both blocks to their final halves yields 64 pairs and 47 wins:

- wall mean, median, paired mean, and paired median improve by `0.752675%`,
  `0.604884%`, `0.748469%`, and `0.851479%`;
- CPU mean, median, paired mean, and paired median improve by `0.750637%`,
  `0.601068%`, `0.746437%`, and `0.840332%`.

The first implementation attempt failed before measurement because derived
`Debug` for `Cell<Option<Term>>` requires `Term: Copy`; the private wrapper
fixed that representation requirement. A second setup attempt found the
missing `std::fmt` import, also before measurement. The first complete
measurement used a debug implementation that took and restored the link while
formatting. A post-run unwind audit rejected that cold shape because a caught
formatter panic could leave the link detached. Its focused and comprehensive
artifacts are superseded. The final opaque debug representation was rebuilt
and remeasured from the exact source above; it changes final deterministic
work by only 84 instructions relative to the superseded candidate and
independently confirms the native gain.

Final raw focused evidence is under:

```text
.artifacts/experiments/2026-07-25-012-split-term-tree-links/final/
```

Fresh exact-source comprehensive validation passes:

- 4,410 Rust tests across 33 groups, including 4,399 library tests and 11
  integration tests;
- Rustfmt, strict all-target/all-feature pedantic Clippy, and the native
  release build;
- Windows GNU x64 test and binary compile-only coverage;
- clean same-tree FOL and higher-order pinned-C builds and smoke tests;
- all 50 main-prover comparisons with zero unexpected and one declared
  difference;
- all 216 support-tool comparisons with zero unexpected and 15 declared
  differences;
- all ten behavior-comparable benchmark cases.

The final benchmark aggregate is `1.1332926602x` Rust/C wall time. LUSK6 and
LUSK6ext are `1.3784681667x` and `1.3609000579x`; BOO020 reaches its exact
resource boundary earlier in Rust and therefore measures `0.474x`. Smoke
Callgrind records `9,803,455` Rust versus `7,590,630` C instructions. The
comprehensive Rust executable SHA-256 matches the final focused candidate
(`fd0cba1276...afd79`), so the report measures the audited opaque-debug
implementation rather than the superseded unwind-unsafe shape.

## Decision

Accept the candidate. It preserves the compact node size, safe owned
links, exact comparator and topology semantics, and proof output while
reducing both deterministic work and production CPU/wall time in independent
blocks. Fresh comprehensive validation is clean. The normal `<=1.10x`
project performance target remains open at a `1.133293x` aggregate.
