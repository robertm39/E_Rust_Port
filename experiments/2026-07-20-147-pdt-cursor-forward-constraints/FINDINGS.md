# PD-tree forward constraints and Windows data-limit headroom

## Question

Can the incremental PD-tree cursor avoid repeated per-frame entry work while
preserving exact traversal order, live substitutions, and node-constraint
pruning? Can the resulting source also make the intermittent BOO020 Windows
allocation boundary deterministic without weakening C's `RLIMIT_DATA`
allowance?

## Setup

- Parent source: commit `6f87cfe0` (`Document Windows polling cadence
  boundary`).
- C reference: archived FOL build from upstream commit
  `17026b1bfe61aaf223cfaae54947c8d2679c31a0`.
- Deterministic workload: upstream `LUSK6.lop` under WSL Callgrind with
  `--auto --silent --cpu-limit=600 --memory-limit=2048 --detsort-rw
  --detsort-new`.
- Native proof corpus: the retained GEO/HEN/LUSK four-case corpus with proof
  objects enabled.
- Native resource corpus: `BOO020-1.p` and `SWV851-1.p` at 60 process-CPU
  seconds and a 2-GiB C data allowance.

The source-resolved baseline and optimized Callgrind profiles are retained at
`.artifacts/experiments/2026-07-20-147-pdt-cursor-lines/`. Compatibility
reports are retained under `.artifacts/e-compare/`.

## Cursor result

The parent cursor pushed a frame and then used an `entered` flag on every loop
to perform node-constraint validation and terminal initialization once. The
optimized cursor validates a child before pushing its frame, initializes the
terminal position at that boundary, and validates the root once when the
cursor starts. A rejected variable child truncates its speculative binding
before the cursor continues to the next sibling. Successful-child visit
counting, symbols-first/variables-first order, and accepted live substitutions
are unchanged.

The parent production profile retires 14,571,495,734 instructions. The
optimized profile at `rust-callgrind-forward-constraints.out` retires
14,432,790,787, a reduction of 138,704,947 instructions (0.95%), while
producing the exact proof. Cursor-exclusive instructions fall from
1,442,770,095 to 1,302,541,615, a 9.72% reduction. A separate line-enabled
baseline (`rust-callgrind-lines.out`, 14,569,881,882 instructions) identified
the repeated entry branch; its slightly different debug-info code generation
is not used for the production percentage.

The first optimized four-case proof report at
`.artifacts/e-compare/20260720-133318-170068/` has zero mismatches.

## Resource diagnosis

The first optimized resource report at
`.artifacts/e-compare/20260720-133517-423753/` exposed the existing
intermittent BOO failure: Rust aborted while requesting 139,264 bytes although
C and SWV reached `ResourceOut`. Repeated isolated runs reproduced the abort.

Two Windows-specific boundary defects were found:

1. The clause-page deadline guard recognized a new overflow page but not the
   first inline page's geometric 512-to-1,024-slot growth, even though both
   request the same 139,264-byte allocation. The guard now recognizes that
   first-page quantum without polling on smaller geometric growths.
2. C applies the 2-GiB limit to `RLIMIT_DATA`, while the Windows Job Object
   charges the entire process. Rust allowed only 12.5%, capped at 256 MiB, for
   executable, stack, allocator-segment, and non-data commit. The translation
   now allows 25%, capped at 512 MiB, while preserving the requested data
   allowance and saturating behavior.

With the corrected translation, three consecutive exact-source BOO runs
terminate normally as `ResourceOut`/8 at the CPU boundary. The final focused
BOO/SWV report at `.artifacts/e-compare/20260720-144107-114966/` has zero
mismatches. The final guard checks only growths large enough to request the
full clause-page quantum, so small-vector initialization does not add repeated
Windows process-clock queries.

## Compatibility result

The exact-source loaded 50-case report at
`.artifacts/e-compare/20260720-145422-391938/` completes every case with one
unexpected row plus the declared sledgehammer difference. BOO, SWV, HEN, and
all proof/protocol cases match. The unexpected row is the pre-existing
synthetic one-second `LUSK6.lop` boundary: C proves in 0.35 seconds while Rust
reaches `ResourceOut` at 1.05 seconds. Five isolated runs after narrowing the
allocation guard all reproduce `ResourceOut`, confirming that the short-budget
performance gap remains open rather than being caused by the guard.

## Falsification checks

- Existing cursor tests cover branch order, age and adjusted-weight pruning,
  repeated variables, mismatched types, accepted live substitutions, and both
  optional counting configurations.
- The new clause-store regression reaches the inline page's 512-slot capacity
  and asserts that the next 139,264-byte growth is recognized.
- Windows process-limit tests pin small proportional, 2-GiB capped, 4-GiB
  capped, and saturating translations.
- Three isolated BOO runs precede the focused BOO/SWV comparison, preventing a
  one-shot success from hiding the intermittent allocator abort.
- The deterministic Linux profile is unaffected by Windows-only resource
  logic and retains exact proof output.

## Reproduction

```powershell
cargo build --locked --release --bin eprover `
  --target-dir target\native-147-forward-constraints
& .\e-interop.ps1 compare `
  -Corpus .\.artifacts\e-corpus\diversity-scratch-139\resource `
  -RustExe .\target\native-147-forward-constraints\release\eprover.exe `
  -TimeoutSeconds 60 -MemoryLimitMb 2048
```

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=rust-callgrind-forward-constraints.out \
  target-wsl-147-forward-constraints/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```

## Decision

Accept forward node validation and terminal initialization in the PD-tree
cursor, first-inline-page allocation detection, and the corrected Windows
`RLIMIT_DATA`-to-process-limit allowance. Keep the main parity issue open: the
cursor is 0.95% cheaper globally, but the deterministic instruction ratio and
the synthetic one-second cutoff still fail the project-wide performance
requirement.
