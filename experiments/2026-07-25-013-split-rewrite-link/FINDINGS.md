# Experiment 314: Split the hot rewrite-replacement link

## Status

Complete and rejected for Bead `E_Rust_Port-j76.5.5`; production source is
unchanged.

## Question

After splitting the intrusive term-tree links, can `TermCell` retain its
152-byte layout while moving only the frequently followed rewrite-replacement
owner from the cold metadata `RefCell` into a pointer-sized
`Cell<Option<Term>>`, without changing rewrite flags, demodulator metadata,
normal-form dates, tree topology, or proof behavior?

## Baseline

- Accepted parent: commit `dd0575f4`.
- Exact matched LUSK6: `7,972,511,554` Rust instructions versus
  `5,254,418,333` C instructions (`1.517297x`).
- `term_follow_top_rw_chain` accounts for `118,116,000` exclusive
  instructions and reads the replacement through the remaining cold
  `RefCell<TermLinks>`.
- The accepted tree-link split showed that a pointer-sized safe `Cell` owner
  can remove dynamic borrow work while keeping the complete term node at 152
  bytes.

## Candidate

The candidate removes `rw_replace` from the cold binding/type `TermLinks`
record and stores it beside the existing left/right links in a private
pointer-sized `TermOwnedLink(Cell<Option<Term>>)`. Rewrite-link reads take,
clone, and synchronously restore the owning handle; writes use `Cell::set`.
The opaque debug representation does not detach any link while formatting.

`TermLinks` falls from 24 to 16 bytes, `RefCell<TermLinks>` falls from 32 to
24 bytes, and the third eight-byte owned-link wrapper keeps the aggregate link
storage at 48 bytes and the complete 64-bit `TermCell` at 152 bytes. No unsafe
code or allocation is added.

## Setup and exact commands

- Worker: `e-rust-codex-260726-042909-1f0a`, Rust 1.97.1.
- Accepted parent: commit `dd0575f4`.
- Uploaded snapshot SHA-256:
  `b863d67a0ddbe262307c26a1711dc71ca31ca73bd4322b3a9b2b65bf607bc562`.
- Exact candidate `termtypes.rs` SHA-256:
  `9da6f9a5fd302760fae6e943cac068085e7eadd51e1abb707b2a23456fd7d26a`.

Focused validation and measurement used `remote_measure.sh`. The candidate was
rejected after its first 64-pair block, so `remote_repeat.sh` and comprehensive
validation were deliberately not run.

## Falsification criteria

- `TermCell` must remain 152 bytes on 64-bit targets.
- Rewrite-link reads must leave the stored owning link installed.
- C-shaped rewrite-chain, restricted-rewrite, deletion, derivation, and GC
  behavior must remain unchanged.
- Parent and candidate must produce byte-identical LUSK6 proof output.
- Exact whole-program instructions must improve at the rewrite-chain owner.
- Alternating native measurements must confirm that any instrumented gain is
  not a throughput reversal.

## Results

Rustfmt, strict all-feature library pedantic Clippy, 18 term-cell tests, 11
rewrite-link tests, 33 clause-rewrite tests, and 125 term-bank tests pass. The
layout/lifecycle regression confirms the sizes above and proves that rewrite,
left, and right owning links remain installed after reads and debug
formatting.

Parent and candidate produce byte-identical LUSK6 proof output with SHA-256
`b50bfb29a2fa3728792a422be2bd83a54c436fa1536c79b8d95c1c1f8c7f427d`,
empty stderr, and exit zero.

Matched Callgrind work falls from `7,972,417,882` to `7,965,953,339`
instructions, a reduction of `6,464,543` (`0.081086%`). Relative to the
matched C count of `5,254,418,333`, the candidate ratio is `1.516049x`.
`term_follow_top_rw_chain` itself falls from `118,116,000` to `114,815,985`
instructions, down `3,300,015` (`2.7939%`). The release executable shrinks
680 bytes, from 8,271,984 to 8,271,304 bytes.

The local owner gain is offset by code-generation movement elsewhere.
`term_top_insert` rises from `1,081,505,063` to `1,089,130,449`
instructions, an increase of `7,625,386` (`0.7051%`), while the rewrite
normal-form owners also redistribute work. The small global Callgrind saving
therefore does not establish a robust production benefit.

All 64 alternating native pairs have the exact proof hash, but the candidate
wins only 27:

- wall mean regresses `0.202094%`, and paired mean regresses `0.232328%`;
- CPU mean regresses `0.203915%`, and paired mean regresses `0.234109%`;
- wall and CPU medians regress `0.336162%` and `0.334333%`.

The final 32 pairs strengthen the rejection: the candidate wins only 11,
wall/CPU means regress `0.564408%`/`0.566621%`, and paired means regress
`0.586933%`/`0.589147%`.

Raw evidence is retained under:

```text
.artifacts/experiments/2026-07-25-013-split-rewrite-link/experiment-314/
```

## Decision

Reject and restore the accepted `dd0575f4` representation. Splitting the
rewrite-replacement owner safely reduces its intended local instruction
boundary, but the complete executable is slower across the full native block
and more clearly slower in its final half. Do not repeat this layout without a
materially different rewrite-chain representation or code-generation reason.
The main `<=1.10x` performance target remains open.
