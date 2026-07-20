# Owned first-order MGU queue consumption

## Question

Can the first-order unifier consume queued terms by ownership, avoiding
reference-count clones and retained owners in already-consumed queue slots,
without changing the C-compatible generic `PQueue` inspection surface or the
unification search path?

## Setup

- Parent source: commit `296a356c` (`Speed PDTree cursor and stabilize Windows
  limits`).
- Deterministic workload: upstream `LUSK6.lop` under WSL Callgrind with
  `--auto --silent --cpu-limit=600 --memory-limit=2048 --detsort-rw
  --detsort-new`.
- C reference: archived FOL build from upstream commit
  `17026b1bfe61aaf223cfaae54947c8d2679c31a0`.
- Native proof corpus: retained GEO/HEN/LUSK four-case corpus with proof
  objects enabled.
- Native resource corpus: `BOO020-1.p` and `SWV851-1.p` at 60 process-CPU
  seconds and a 2-GiB C data allowance.

Profiles are retained in
`.artifacts/experiments/2026-07-20-148-next-hotspot/`. The committed production
baseline is retained at
`.artifacts/experiments/2026-07-20-147-pdt-cursor-lines/rust-callgrind-forward-constraints.out`.
Compatibility reports are retained under `.artifacts/e-compare/`.

## Rejected dereference dispatch

A preliminary candidate replaced the existing dereference predicates with one
manual function-code dispatch. It preserved the exact proof but retired
14,439,203,656 instructions, 6,412,869 more than the 14,432,790,787 parent
(+0.044%). The change was reverted. Its profile is retained as
`rust-callgrind-deref-dispatch.out`.

The separate line-enabled parent profile `rust-callgrind-lines.out` is retained
for source attribution only. Debug information slightly changes release code
generation, so its 14,429,647,291-instruction total is not used for the
production comparison.

## Owned queue result

The C `PQueueGetLast` returns a raw pointer. Rust's compatibility-shaped
`PQueue::get_last` clones the stored value and deliberately leaves the
consumed backing slot readable through absolute indexing. In
`subst_compute_mgu`, each cloned `Term` was then cloned again when
dereferencing made no change, and the queue kept stale reference-counted term
owners until later overwrite or queue destruction.

The accepted candidate adds a crate-private `take_last` operation that moves a
value out and clears only that consumed slot. Existing `get_last` behavior is
unchanged for compatibility callers. An owned dereference helper returns its
input directly on the unchanged path. Only the first-order unifier uses these
owned variants; higher-order queue consumption remains unchanged.

The candidate produces the exact proof and retires 14,421,005,745
instructions, 11,785,042 fewer than the parent (-0.082%). Exclusive
`subst_compute_mgu` work falls from 237,427,797 to 228,604,174 instructions,
an 8,823,623-instruction reduction (-3.72%). `deref_step` remains exactly
633,096,520 instructions, while allocator-function counts are effectively
flat, confirming that the improvement comes from local queue/ownership work
rather than a changed proof-search path. The accepted profile is retained as
`rust-callgrind-owned-pqueue.out`.

## Compatibility result

The four-case proof report at
`.artifacts/e-compare/20260720-153945-339530/` has zero mismatches. The
two-case resource report at `.artifacts/e-compare/20260720-154145-119404/`
also has zero mismatches: both BOO020 and SWV851 retain the expected
`ResourceOut` boundary rather than aborting in the allocator.

The complete Rust suite passes 4,369 unit tests plus every integration target.
Strict all-target/all-feature pedantic Clippy and formatting also pass.

## Falsification checks

- The new queue regression consumes a non-`Clone` value and verifies that each
  moved slot is empty, proving the optimized API does not silently clone.
- Existing `get_next` and `get_last` tests continue to inspect consumed slots
  through absolute indices, pinning the compatibility behavior separately.
- Focused first-order MGU tests cover disjoint variables, occurrence failure,
  applied free-variable heads, and predicate-position rejection.
- The deterministic Callgrind workload preserves the exact proof and leaves
  the principal dereference count unchanged.
- Native proof and resource corpora check normalized output, exit behavior,
  and the maintained Windows allocation boundary.

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=rust-callgrind-owned-pqueue.out \
  target-wsl-148-owned-pqueue/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```

```powershell
cargo build --locked --release --bin eprover `
  --target-dir target\native-148-owned-pqueue
& .\e-interop.ps1 compare `
  -Corpus .\.artifacts\e-corpus\diversity-scratch-139\proof `
  -RustExe .\target\native-148-owned-pqueue\release\eprover.exe `
  -TimeoutSeconds 60 -MemoryLimitMb 2048
```

## Decision

Accept owned queue consumption and unchanged-term dereferencing in the
first-order MGU path. Reject the manual dereference dispatch. Keep the main
parity issue open: this is a measured local improvement, but the synthetic
one-second LUSK cutoff and the overall C/Rust performance ratio remain outside
the project-wide acceptance criteria.
