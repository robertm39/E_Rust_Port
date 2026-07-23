# Experiment 236: Reject non-owning term-tree splay tails

## Question

Can the top-down term-tree splay move each traversed node directly into the
partial tree and retain a private non-owning tail cursor, matching C's raw
tail pointers instead of cloning and later dropping one extra `Rc<TermCell>`
handle per tail extension?

## Baseline

- Source: commit `233f84a7`, whose executable source remains accepted
  Experiment 231.
- Exact LUSK6 Callgrind: 9,923,564,772 instructions.
- `TermTree::insert`: 658,858,502 exclusive instructions.
- Original C Callgrind: 5,254,361,329 instructions.
- Rust/C ratio: 1.888634.

## Candidate

- Add a private non-owning cursor containing the stable `TermCell` allocation
  pointer obtained from an `Rc<TermCell>`.
- Move each traversed splay node directly into the partial tree instead of
  cloning it into the tree and retaining the original handle as the tail.
- Keep the cursor's two raw link setters behind a safe private splay boundary,
  with every dereference justified by the live root/link chain and with
  `RefCell` still enforcing link-borrow aliasing.
- Add a 64-node zig-zag regression in addition to the four existing term-tree
  tests.

## Validation

- All five focused term-tree tests and all 18 focused term-cell tests pass.
- Strict all-feature library pedantic Clippy accepts the unsafe documentation
  and containment.
- Formatting and diff checks pass.
- The deterministic LUSK6 run proves Unsatisfiable with the expected 4,873
  processed clauses and exits zero.

## Measurement

Exact Callgrind instructions regress from 9,923,564,772 to 9,927,554,890: an
increase of 3,990,118 or 0.040209%. The implied Rust/C ratio worsens from
1.888634 to 1.889393.

The intended owner also regresses: `TermTree::insert` rises from 658,858,502
to 663,221,211 exclusive instructions, an increase of 4,362,709 or 0.662162%.
Work outside that function improves by only 372,591 instructions. Allocator
entries and every other leading algorithm reproduce closely, so the raw-tail
representation costs more locally than the optimized owning `Term` tail.

The raw candidate profile is
`.artifacts/experiments/2026-07-22-236-non-owning-termtree-tails/rust-callgrind-non-owning-termtree-tails.out`.
The retained parent profile is
`.artifacts/experiments/2026-07-22-231-specialize-pdt-cursor/rust-callgrind-specialize-pdt-cursor.out`.

## Decision

Reject. Although the cursor invariant is sound and closely matches C's tail
pointers, LLVM's optimized owning-handle shape is cheaper. Native timing and
compatibility matrices are skipped after the deterministic and intended-owner
regressions. All raw-pointer production code is removed; source is restored
byte-for-byte to the safe Experiment 231 accepted baseline.

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=rust-callgrind-non-owning-termtree-tails.out \
  target-wsl-236-non-owning-termtree-tails/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```
