# Lazy C-shaped PD-tree query traversal

## Question

Can first-order PD-tree demodulator search stop flattening and retaining every
query subterm before traversal, and instead follow C's lazy `term_stack` /
`term_proc` organization without changing candidate order, substitutions, or
backtracking behavior?

## Setup

- Parent source: commit `8a016963` (`Optimize sparse clause slot iteration`).
- Deterministic workload: upstream `LUSK6.lop` under WSL Callgrind with
  `--auto --silent --cpu-limit=600 --memory-limit=2048 --detsort-rw
  --detsort-new`.
- Parent profile: 13,863,033,680 instructions with the exact proof and 4,873
  processed clauses.
- C reference: archived FOL build from upstream commit
  `17026b1bfe61aaf223cfaae54947c8d2679c31a0`, profiled at 5,254,361,329
  instructions.
- Native proof corpus: retained GEO/HEN/LUSK four-case corpus with proof
  objects enabled.
- Native resource corpus: `BOO020-1.p` and `SWV851-1.p` at 60 process-CPU
  seconds and a 2-GiB C data allowance.

The profile is retained at
`.artifacts/experiments/2026-07-20-154-lazy-pdt-query/`. Compatibility reports
are retained under `.artifacts/e-compare/`.

## Structural attribution

The retained line-enabled profile recorded 877,339 PD-tree search
initializations. Rust built a complete prefix vector on every initialization,
cloning one reference-counted `Term` handle per query cell and using a second
enter/exit frame stack to calculate subtree spans. The current exact profile
charged 1,062,299,033 exclusive instructions to `record_search_init`.

C `PDTreeSearchInit` instead stores one query-root pointer and initializes a
one-entry `term_stack`. `PDTreeFindNextDemodulator` expands child terms only
when a symbol edge advances, while `term_proc` records consumed roots so
backtracking can restore the traversal stack. This made eager Rust query
flattening a confirmed porting and performance gap rather than an isolated
micro-optimization opportunity.

## Lazy cursor

The retained substitution cursor mirrors the C state machine with safe owned
handles and no recursion:

- search state retains only the root `Term` for the production demodulator
  path;
- `query_stack` contains pending left-to-right traversal terms;
- successful symbol edges expand only their direct children;
- variable edges consume the complete current subtree without expanding it;
- `query_steps` retains each consumed root and its direct expansion count, so
  frame pop restores the pending stack exactly;
- speculative variable bindings refer to stable query-step indices, avoiding
  another `Term` clone while preserving repeated-variable comparison and live
  substitution construction.

The general compatibility collectors still use the former flat prefix query.
That vector is now materialized on demand by `search_state`, matchable-path,
and collection helpers and is recycled exactly as before. The production
`search_next_matching_occurrence_with_subst` path never constructs it.

The focused regression verifies that substitution traversal leaves the flat
query absent, preserves general-before-specific candidate order and the live
binding, and restores the root-only pending stack with no processed steps or
frames after exhaustion. Existing PD-tree tests cover inconsistent repeated
variables, external bindings, deletion/reinsertion, constraints, traversal
order, higher-order normalization, flat-query compatibility, and scratch
reuse. All 37 focused PD-tree tests pass.

## Performance result

The exact-proof profile falls to 13,412,948,963 instructions, 450,084,717
below the parent (-3.2467%). The deterministic C/Rust ratio improves from
2.638 to 2.553. `record_search_init` falls from 1,062,299,033 to 63,168,408
exclusive instructions (-94.05%). Lazy work moves into the actual search:
`search_next_matching_occurrence_with_subst` rises from 1,305,576,641 to
1,532,906,560 (+17.41%), and the now-visible backtracking helper costs
276,837,018 instructions. The global reduction confirms that avoiding work on
unvisited query subtrees outweighs the extra owned-stack bookkeeping.

## Compatibility result

The final four-case proof report at
`.artifacts/e-compare/20260720-202113-080169/` has zero mismatches across
GEO288, HEN011, LUSK6, and LUSK6ext. The final resource report at
`.artifacts/e-compare/20260720-202323-870648/` has zero mismatches for BOO020
and SWV851. Thus the cursor preserves proof order and the maintained resource
outcomes while reducing transient query ownership.

The complete Rust suite passes 4,371 library tests plus every integration
target. Strict all-target/all-feature pedantic Clippy, formatting, and the
all-feature release build pass.

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=rust-callgrind-lazy-query.out \
  target-wsl-154-lazy-pdt-query/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```

```powershell
cargo build --locked --release --bin eprover `
  --target-dir target\native-154-lazy-pdt-query
& .\e-interop.ps1 compare `
  -Corpus .\.artifacts\e-corpus\diversity-scratch-139\proof `
  -RustExe .\target\native-154-lazy-pdt-query\release\eprover.exe `
  -TimeoutSeconds 60 -MemoryLimitMb 2048
& .\e-interop.ps1 compare `
  -Corpus .\.artifacts\e-corpus\diversity-scratch-139\resource `
  -RustExe .\target\native-154-lazy-pdt-query\release\eprover.exe `
  -TimeoutSeconds 60 -MemoryLimitMb 2048
```

## Decision

Accept lazy PD-tree query traversal. It restores C's demand-driven search
organization, removes almost all eager initialization cost, preserves the
existing compatibility collectors, and passes exact proof and resource gates.
Keep the main parity issue open: the synthetic one-second LUSK cutoff and the
remaining 2.553 deterministic C/Rust instruction ratio still fail the
project-wide acceptance criteria.
