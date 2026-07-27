# Borrowed free-variable dereference chain

## Question

Can `DerefType::Always` follow the common free-variable binding chain closer to
C pointer traversal by borrowing adjacent bindings and cloning only the term
that leaves the borrow window?

## Setup

- Parent source: commit `7fa99e98` (`Record rejected phony-app short circuit`).
- Deterministic workload: upstream `LUSK6.lop` under WSL Callgrind with
  `--auto --silent --cpu-limit=600 --memory-limit=2048 --detsort-rw
  --detsort-new`.
- Parent profile: 13,122,494,580 instructions with the exact proof and 4,873
  processed clauses.
- C reference: archived FOL build from upstream commit
  `17026b1bfe61aaf223cfaae54947c8d2679c31a0`, profiled at 5,254,361,329
  instructions.
- Native proof corpus: GEO288, HEN011, LUSK6, and LUSK6ext with proof objects.
- Native resource corpus: BOO020 and SWV851 at 60 process-CPU seconds and a
  2-GiB C data allowance.

Profiles are retained under
`.artifacts/experiments/2026-07-21-159-borrowed-deref-chain/`, and compatibility
reports are retained under `.artifacts/e-compare/`.

## Attribution and candidates

The accepted parent spends 407,926,774 exclusive instructions in
`term_deref_if_changed` and 633,096,520 in `deref_step`, 1,041,023,294
combined. Unlike C, whose first-order `TermDeref` loop follows raw binding
pointers, each Rust step clones the binding `Rc` before the next step and then
drops the intermediate owner.

The first candidate recursively borrowed up to eight binding links. It
preserved the proof but regressed to 13,141,007,517 instructions, 18,512,937
above the parent (+0.1411%). Its 12,627,515 chain entries made only 4,267,045
recursive calls, yet those recursive calls cost 98,347,271 inclusive
instructions. The ownership idea was locally useful, but the recursive shape
cost more globally than it saved.

The final candidate uses a fixed two-link borrow window directly in the
always-dereference step. If the first binding is another free variable, it
borrows that variable's binding before cloning; otherwise it clones the first
binding. Longer chains repeat through the existing iterative loop, so nested
borrow and call-stack depth remain constant. `DerefType::Once` retains the
original single-link behavior. A 20-variable regression covers repeated
windows and the unchanged one-step limit.

## Performance result

The final candidate preserves the exact proof at 13,021,111,518 instructions,
101,383,062 below the parent (-0.7726%). Its dereference components are
402,948,739 in `term_deref_if_changed`, 336,522,069 in
`deref_always_step`, and 193,458,937 in the remaining general `deref_step`,
932,929,745 combined. That is a local reduction of 108,093,549 instructions
(-10.38%). The C/Rust ratio improves from 2.497 to 2.478.

## Compatibility result

The final unloaded proof report
`.artifacts/e-compare/20260721-004009-845431/` has zero mismatches across all
four proof cases. HEN completes with the exact proof in 45.23 seconds. An
earlier proof run executed concurrently with SWV and reached the known
load-sensitive HEN cutoff at `.artifacts/e-compare/20260721-003041-685856/`.
The first isolated candidate retry also reached the boundary, while the
accepted parent control was exact at
`.artifacts/e-compare/20260721-003641-196277/`; a clean candidate repeat was
then exact at `.artifacts/e-compare/20260721-003839-456425/`. The final
unloaded four-case report is the acceptance evidence.

The combined resource report
`.artifacts/e-compare/20260721-004158-794030/` also has zero mismatches. BOO020
and SWV851 both return normalized `ResourceOut` without allocator failure.
The earlier focused BOO report `.artifacts/e-compare/20260721-002819-166167/`
and focused SWV report `.artifacts/e-compare/20260721-003041-685853/` agree.

## Validation

- `cargo fmt --all -- --check`
- 4,374 library tests plus all integration targets and features
- strict all-target, all-feature pedantic Clippy
- all-feature release `eprover` build
- all four documentation gates
- clean vendored C worktree

## Decision

Accept the fixed two-link borrowed dereference window and reject the recursive
eight-link draft. The retained version removes common intermediate ownership
without unbounded recursion, search-order change, or resource-boundary
regression.
