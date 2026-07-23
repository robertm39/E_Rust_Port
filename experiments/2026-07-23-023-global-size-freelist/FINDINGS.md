# Experiment 261: Global exact-size free list

## Status

Accepted for Bead `E_Rust_Port-j76.5.3`.

## Question

Can Rust recover upstream E's exact-size allocation reuse at the global
allocation boundary, where Rust ownership abstractions otherwise turn
short-lived prover objects into millions of libc `malloc` and `free` calls?

## C allocator diagnosis

The accepted Rust profile issued about 4,195,245 libc `malloc` calls and
3,679,486 libc `free` calls. The corresponding C run issued 1,005,153
`malloc` calls and only 572 `free` calls.

Upstream E routes sizes from one pointer through 8,191 bytes through
`SizeMallocReal`'s process-wide exact-size free lists. Freed blocks store the
next pointer in their first word and are reused without returning to libc. The
accepted Rust implementation had no equivalent because Rust collections and
owned values allocate through the process global allocator.

## Implementation and safety boundary

The candidate installs a private global allocator that reproduces the C size
classes:

- requested sizes in `[sizeof(pointer), 8192)` with alignment at most 16 are
  cached by exact byte size;
- cache misses use `System` with the exact requested size and 16-byte
  alignment, so a cached block satisfies every request admitted to the class;
- the first pointer-sized word of a freed block is the intrusive next link;
- one process-wide spin lock serializes every list link read and write;
- zero/small, large, and over-aligned layouts pass through to `System`
  unchanged;
- after a `System` allocation failure, cached blocks are returned to `System`
  and the allocation is retried once, matching the upstream recovery policy.

The required unsafe code is contained in one private module. The
`GlobalAlloc` implementation, every unsafe operation, and both unsafe trait
methods document their invariants. Tests cover the exact C boundaries,
normalized cached layouts, vector growth, over-aligned allocation, and
parallel reuse.

## Baseline and deterministic measurement

- Accepted source: Experiment 260.
- Accepted exact LUSK6 Callgrind: 9,596,668,097 instructions.
- Original C Callgrind: 5,254,361,329 instructions.
- Accepted Rust/C ratio: 1.826420.

The default-feature candidate preserves the exact 4,873-processed-clause
LUSK6 proof and retires 9,106,424,013 instructions. That is 490,244,084 fewer
instructions, or 5.108482%, and improves the Rust/C ratio to 1.733117.

The candidate allocator itself accounts for 173,101,164 instructions, but it
substantially reduces visible libc allocation work:

| Exclusive owner | Accepted | Candidate | Change |
| --- | ---: | ---: | ---: |
| libc `_int_malloc` | 195,683,204 | 123,558,703 | -72,124,501 |
| libc `malloc` | 188,286,956 | 42,136,367 | -146,150,589 |

The accepted profile also spent 215,253,686 instructions in `_int_free` and
103,025,673 in `free`. The major prover-algorithm call counts reproduce, and
`TermTree::insert` changes only from 658,922,451 to 658,797,132 instructions,
supporting allocation reuse rather than an algorithmic behavior change as the
source of the improvement.

The raw candidate profile is:

```text
.artifacts/experiments/2026-07-23-023-global-size-freelist/rust-callgrind-global-size-freelist.out
```

Valgrind reported a `brk segment overflow` warning during the instrumented run
but completed normally with the exact proof. A direct WSL run outside
Valgrind also completed normally in 1.84 seconds wall time, 1.62 seconds user
CPU, and 210,656 KiB maximum RSS.

## Native production measurement

After a warmup, 64 alternating default-feature Windows pairs all prove the
theorem and exit zero. The candidate executable is 8,928,768 bytes versus
8,654,336 bytes for the accepted parent.

Across all 64 pairs:

- wall mean improves 6.751363%, from 1.507653 to 1.405866 seconds;
- process-CPU mean improves 6.949040%, from 1.475586 to 1.373047 seconds;
- wall and CPU medians improve 7.069490% and 7.446809%;
- mean paired wall and CPU changes improve 6.721887% and 6.911541%;
- median paired wall and CPU changes improve 6.875266% and 6.832057%;
- the candidate wins 61 wall pairs and 62 CPU pairs, with no CPU ties.

The stable last 32 pairs remain positive:

- wall and CPU means improve 6.572095% and 6.848404%;
- wall and CPU medians improve 7.413368% and 7.446809%;
- mean paired wall and CPU changes improve 6.544957% and 6.838533%;
- median paired wall and CPU changes improve 7.007696% and 6.980914%;
- the candidate wins 29 wall pairs and 30 CPU pairs, with no CPU ties.

The measured rows are retained in `native-lusk.csv`.

## Compatibility and validation

- Strict resource report `.artifacts/e-compare/20260723-131343-800217` has
  BOO020 and SWV851 exact at the maintained 60-second/2-GiB limits.
- Focused proof report `.artifacts/e-compare/20260723-131720-589676` has GEO,
  HEN, LUSK6, and LUSK6ext exact with zero mismatches.
- Full maintained report `.artifacts/e-compare/20260723-132455-161728`
  completes all 50 cases with zero unexpected mismatches and only the declared
  `sledgehammer` normalized-output difference. It includes exact one-second
  LUSK6, higher-order, BOO, and SWV cases.
- Four focused allocator tests pass with default and all features.
- The full serial all-target/all-feature suite passes 4,392 library tests plus
  every integration and binary target.
- Strict default-library and all-target/all-feature pedantic Clippy pass.
- The locked all-feature release build, formatting, `git diff --check`, all
  four C-source documentation gates, and vendored-C cleanliness pass.

An initial full-suite attempt allowed parallel Cargo processes and exhausted
the Windows paging file while `rustc` requested another 2 MiB. That
host-pressure run is invalid; setting `CARGO_BUILD_JOBS=1` and
`RUST_TEST_THREADS=1` produced the complete passing suite reported above.

## Decision

Accept. The change ports upstream E's central exact-size reuse policy at the
only boundary that covers Rust-owned prover data, cuts exact instructions by
5.11%, improves native wall and CPU time by about 6.8-6.9%, and preserves
proof, resource, and maintained-matrix behavior. The accepted baseline becomes
9,106,424,013 instructions, or 1.733117 times C.

The port is not yet at performance parity: the exact workload still retires
73.31% more instructions than C, and HEN remains materially slower in the
maintained matrix. Bead `E_Rust_Port-j76.5.3` therefore remains open.

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=rust-callgrind-global-size-freelist.out \
  target-wsl-261-global-size-freelist/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```

```powershell
& .\experiments\2026-07-22-214-move-termtree-insert-links\run-native.ps1 `
  -ParentExe .\target\native-260-defer-eval-store-index\release\eprover.exe `
  -CandidateExe .\target\native-261-global-size-freelist\release\eprover.exe `
  -Pairs 64 `
  -OutputCsv .\experiments\2026-07-23-023-global-size-freelist\native-lusk.csv
```
