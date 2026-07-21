# Inline KBO6 balance traversal frames

## Question

Can the KBO6 variable/weight-balance walkers eliminate their per-call heap
stack without changing proof order or reopening the maintained BOO020 memory
boundary?

## Setup

- Parent source: commit `0d25b04d` (`Optimize variable bank lookup with sparse
  pages`).
- Deterministic workload: upstream `LUSK6.lop` under WSL Callgrind with
  `--auto --silent --cpu-limit=600 --memory-limit=2048 --detsort-rw
  --detsort-new`.
- Parent profile: 12,778,448,460 instructions with the exact proof and 4,873
  processed clauses.
- C reference: archived FOL build from upstream commit
  `17026b1bfe61aaf223cfaae54947c8d2679c31a0`, profiled at 5,254,361,329
  instructions.

C's `mfyvwblhs` and `mfyvwbrhs` allocate a local tagged pointer stack through
E's size-specific allocator. Rust used a fresh `Vec<(Term, DerefType)>` in
each of the first-order, LFHO, and Lambda-order walkers. The candidate retained
a small safe `Option` array in the traversal frame and used a `Vec` only after
that inline region filled. A focused regression preserved exact LIFO order
across the inline/overflow boundary.

Raw profiles are retained under
`.artifacts/experiments/2026-07-21-164-inline-kbo-balance/`.

## Capacity sweep

Every capacity preserves the exact LUSK6 proof:

| Inline frames | Instructions | Change from parent | C/Rust ratio |
| ---: | ---: | ---: | ---: |
| 2 | 12,672,414,924 | -106,033,536 (-0.8298%) | 2.412 |
| 4 | 12,648,714,963 | -129,733,497 (-1.0153%) | 2.407 |
| 8 | 12,643,111,692 | -135,336,768 (-1.0591%) | 2.406 |
| 16 | 12,661,280,550 | -117,167,910 (-0.9169%) | 2.410 |

Four frames are the best deterministic point before diminishing returns;
eight saves only another 5,603,271 instructions (0.0443%), and sixteen loses
18,168,858 instructions relative to eight. On the four-frame profile,
`mfy_vwb` itself grows from 197,475,444 to 233,285,358 exclusive instructions
because safe inline bookkeeping replaces the simple `Vec` path. The global
win comes from allocator reductions: `_int_free` falls by about 39.3 million,
`_int_malloc` by 34.2 million, `malloc` by 11.8 million, and `free` by 7.4
million instructions, with additional savings in Rust allocation glue.

## Resource falsification

The first four-frame native build passed combined BOO020/SWV851 resource report
`.artifacts/e-compare/20260721-043300-442326/` and four-case proof report
`.artifacts/e-compare/20260721-043714-122030/`, both with zero mismatches.

The capacity sweep exposed that this was not robust enough for acceptance:

- Eight frames fail BOO020 with allocator exit 9 on a 139,264-byte request in
  combined report `.artifacts/e-compare/20260721-045139-382906/`; SWV851 is
  exact.
- The restored four-frame candidate fails the same BOO allocation in full
  report `.artifacts/e-compare/20260721-050206-944517/`. That 50-case report's
  other unexpected row is the already narrow synthetic one-second LUSK6
  cutoff, and its sole declared difference is `sledgehammer.p` proof order.
  All remaining rows, including HEN011, GEO288, SWV851, LCL365, and ordinary
  LUSK6/LUSK6ext proofs, are exact.
- Two frames fail focused BOO report
  `.artifacts/e-compare/20260721-051347-756461/` on the same allocation.
- A control run of the unchanged experiment-163 parent immediately afterward
  also fails focused BOO report
  `.artifacts/e-compare/20260721-051607-609761/`. The resource boundary was
  therefore noisy in the final host state, but that does not supply positive
  evidence for retaining a layout whose four- and eight-frame forms already
  failed maintained suites.

The inline stack makes proof search faster under the same CPU limit, which can
also let it admit more search state before the deadline guard runs. At the
current allocator-sensitive 2-GiB boundary, deterministic speed alone is not
sufficient evidence that the altered allocation schedule is safe.

## Validation and decision

The four-frame candidate passed formatting, 4,377 library tests plus every
integration target and feature, strict all-target/all-feature pedantic Clippy,
and the all-feature release build. These gates establish implementation
correctness but do not override the resource failure.

Reject every inline capacity and restore `src/orderings/cto_kbolin.rs` exactly
to commit `0d25b04d`. The experiment identifies a real 0.83-1.06% allocation
opportunity, but it should be revisited only after BOO020's live-set/deadline
boundary has more margin or with a reusable traversal buffer whose resource
effect can be proved under repeated constrained runs.

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=rust-callgrind-inline-balance.out \
  target-wsl-164-inline-balance/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```

```powershell
& .\e-interop.ps1 compare `
  -Corpus .\.artifacts\e-corpus\diversity-scratch-139\resource `
  -RustExe .\target\native-164-inline-balance\release\eprover.exe `
  -TimeoutSeconds 60 -MemoryLimitMb 2048
```
