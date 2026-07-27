# Rejected direct top-cell copy construction

## Question

Can `Term::top_copy_without_args` construct the new `Rc<TermCell>` with its
source symbol, filtered properties, and type already in place, instead of
allocating a default cell and then assigning those fields through public
setters and the destination `TermLinks` borrow boundary?

## Setup

- Parent source: commit `0f06f64e` (`Construct rewritten terms without
  staging vectors`), accepted Experiment 223.
- Candidate: initialize the complete `TermCell` in one `Rc::new` expression.
  Argument storage, empty binding/rewrite/tree links, filtered properties,
  zero counts and weight, creation dates, and copied type are identical to the
  existing constructor/setter sequence.
- Deterministic workload: upstream `LUSK6.lop` under WSL Callgrind with
  `--auto --silent --cpu-limit=600 --memory-limit=2048 --detsort-rw
  --detsort-new`.
- Parent profile:
  `.artifacts/experiments/2026-07-22-223-direct-rewrite-term/rust-callgrind-direct-rewrite-term.out`.
- Candidate profile:
  `.artifacts/experiments/2026-07-22-224-direct-top-copy/rust-callgrind-direct-top-copy.out`.
- C reference: archived FOL build from upstream commit
  `17026b1bfe61aaf223cfaae54947c8d2679c31a0`, profiled at 5,254,361,329
  instructions.

The candidate also omits the explicit `TP_OUTPUT_FLAG` deletion after masking
properties down to `TP_PRED_POS | TP_IS_DB_VAR`; that flag is already absent
from the masked value. No allocation shape, argument representation, or public
API changes.

## Deterministic result

The candidate preserves the exact 4,873-processed-clause LUSK6 proof and
retires 10,421,072,451 instructions. This is 76,811,845 below the
10,497,884,296-instruction parent, a 0.731689% whole-prover reduction. The
hypothetical C/Rust ratio improves from 1.997937 to 1.983319.

The rewrite reconstruction edge into `top_copy_without_args` falls from
74,598,817 to 64,631,722 instructions, down 9,967,095 or 13.360929%. The
replacement-insertion edge falls from 251,336,522 to 216,569,857, down
34,766,665 or 13.832715%. Those two directly comparable edges explain
44,733,760 instructions, or 58.238101% of the global gain. Their enclosing
recursive rewrite and replacement paths fall by another 15,668,294 and
63,099,314 inclusive instructions; those totals overlap their child costs and
are not summed.

The PD-tree cursor, substitution normalizer exclusive work, `insert_repl`
exclusive work, and all 5,845,869 Rust allocation calls reproduce exactly.
`TermTree::insert` rises 760,392 instructions or 0.115438%, a small layout
effect compared with the intended constructor reduction.

## Native result

Both binaries first completed two alternating warmup pairs. Two independent
64-pair production-feature Windows blocks then completed all 256 measured
processes with exit zero.

The native result consistently reverses the Callgrind result:

| Metric | Block A | Block B | Combined 128 pairs |
| --- | ---: | ---: | ---: |
| Wall mean | +0.038918% | +0.559120% | +0.298448% |
| CPU mean | +0.230259% | +0.366549% | +0.298325% |

Across the combined sample, wall median regresses 0.172730%, paired wall mean
regresses 0.350493%, and paired wall median regresses 0.150462%. CPU medians
tie at 1.796875 seconds because Windows reports process CPU in coarse 15.625 ms
quanta, while paired CPU mean regresses 0.382972%. The candidate wins only 60
of 128 wall pairs and 56 CPU pairs, with 14 CPU ties.

The stable tails do not rescue the candidate. Block A's last 32 pairs are
essentially wall-neutral at a 0.010950% improvement but regress CPU mean
0.135722%. Block B's last 32 regress wall mean 0.079559% and CPU mean
0.371535%. The candidate executable also grows 23,040 bytes, from 8,647,680
to 8,670,720.

## Validation

- All 18 focused term-cell tests pass with the candidate, including complete
  top-copy metadata, inline/heap argument storage, link-boundary, identity, and
  drop-semantics coverage.
- Strict all-feature library pedantic Clippy and formatting pass.
- The candidate produces the exact LUSK6 proof and exits zero under Callgrind.
- All 256 native measured processes plus four warmup processes prove and exit
  zero.
- Source is restored byte-for-byte; all 18 focused tests and formatting pass
  after restoration.
- Compatibility matrices and broader quality gates were skipped after the
  replicated native performance gate failed.
- The vendored C checkout was not modified.

## Decision

Reject direct top-cell construction and retain the default-allocation plus
setter sequence. The candidate removes substantial deterministic work, but
two independent warmed native blocks agree on a roughly 0.3% production CPU
regression and the binary grows 23 KiB. Whole-program native performance is
the acceptance criterion when it conflicts with instrumentation. Accepted
Experiment 223 remains the baseline at 10,497,884,296 instructions, or
1.997937 times C.

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=rust-callgrind-direct-top-copy.out \
  target-wsl-224-direct-top-copy/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```

```powershell
& .\experiments\2026-07-22-214-move-termtree-insert-links\run-native.ps1 `
  -ParentExe .\target\native-223-direct-rewrite-term\release\eprover.exe `
  -CandidateExe .\target\native-224-direct-top-copy\release\eprover.exe `
  -Pairs 64 `
  -OutputCsv .\native-lusk.csv
```
