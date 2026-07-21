# Rejected evaluation-splay ordering return

## Question

Can `EvalIndexTree::splay` return its final key/root ordering together with the
new root so insertion and removal avoid immediately comparing the same pair
again?

## Setup

- Parent source: commit `8021fd83` (`Use direct links in evaluation splay`).
- Deterministic workload: upstream `LUSK6.lop` under WSL Callgrind with
  `--auto --silent --cpu-limit=600 --memory-limit=2048 --detsort-rw
  --detsort-new`.
- Parent profile:
  `.artifacts/experiments/2026-07-21-187-direct-eval-splay-links/rust-callgrind-direct-splay-links-clippy.out`.
- Candidate profile:
  `.artifacts/experiments/2026-07-21-188-eval-splay-ordering/rust-callgrind-splay-ordering.out`.

## Candidate

The candidate changed the private splay return from one arena index to
`(index, Ordering)`. Every exit carried the ordering already computed by the
last loop iteration. Insert matched directly on it, removal rejected a
non-equal root from it, and the secondary left-subtree splay ignored it.
Tree rotations, direct sentinel links, comparator semantics, allocation,
duplicate handling, and node layout were unchanged.

The focused evaluation-index regression passed, including sorted order,
removal, duplicate suppression, freed-slot reuse, equality, and the 48-byte
node invariant. Deterministic LUSK6 preserved the exact 4,873-clause proof.

## Result

The candidate retires 11,707,970,962 instructions, 3,796,093 above the
11,704,174,869-instruction parent, a 0.0324% whole-prover regression.
`EvalIndexTree::splay` itself rises from 306,825,308 to 313,810,070 exclusive
instructions, an increase of 6,984,762 or 2.2765%. Unrelated compiler-layout
reductions in term-tree and allocator entries partially hide that local loss.

Returning and propagating the extra ordering costs more than recomputing the
single post-splay comparison in the current generated code. The source was
restored exactly to commit `8021fd83` before native matrices were run.

## Decision

Reject the ordering-return tuple. Keep Experiment 187 at 11,704,174,869
instructions and a 2.2275 C/Rust ratio. The evaluation splay should retain its
single-index return contract.

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=rust-callgrind-splay-ordering.out \
  target-wsl-188-eval-splay-ordering/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```
