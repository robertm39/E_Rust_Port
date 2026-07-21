# Inline unary and binary term arguments

## Question

Can the Rust term representation recover part of C's single-allocation
flexible-array shape without unsafe code by storing the common unary and binary
argument slots inside the reference-counted term cell?

## Setup

- Parent source: commit `0052dfa3` (`Borrow adjacent dereference bindings`).
- Deterministic workload: upstream `LUSK6.lop` under WSL Callgrind with
  `--auto --silent --cpu-limit=600 --memory-limit=2048 --detsort-rw
  --detsort-new`.
- Parent profile: 13,021,111,518 instructions with the exact proof and 4,873
  processed clauses.
- C reference: archived FOL build from upstream commit
  `17026b1bfe61aaf223cfaae54947c8d2679c31a0`, profiled at 5,254,361,329
  instructions.
- Native proof corpus: GEO288, HEN011, LUSK6, and LUSK6ext with proof objects.
- Native resource corpus: BOO020 and SWV851 at 60 process-CPU seconds and a
  2-GiB C data allowance.

The retained profile is
`.artifacts/experiments/2026-07-21-160-inline-term-args/rust-callgrind-inline-args.out`.
Compatibility reports are retained under `.artifacts/e-compare/`.

## Representation

The parent stores a `RefCell<Box<[Option<Term>]>>` in every term. Empty slices
do not allocate, but every nonzero arity allocates a second object even though
unary and binary terms dominate first-order syntax. The candidate replaces the
boxed slice with a safe `TermArgs` enum:

- arity zero has an empty variant;
- arities one and two use a two-slot inline array;
- larger arities retain the boxed-slice fallback.

The enum is 24 bytes and its `RefCell` is 32 bytes, so `TermCell` grows from
144 to 152 bytes. Unary and binary cells eliminate their separate allocation;
zero-arity and larger cells pay eight additional inline bytes. No raw pointers,
custom allocator, or unsafe access is introduced. A layout regression pins the
new sizes, and an arity-zero-through-four regression exercises all variants,
mutation, borrowed slices, cloned arguments, and heap fallback.

## Performance result

The candidate preserves the exact proof at 12,888,451,124 instructions,
132,660,394 below the parent (-1.0188%). The C/Rust ratio improves from 2.478
to 2.453.

The four visible allocator components fall from 1,619,486,075 to
1,262,998,341 instructions, a reduction of 356,487,734 (-22.01%):

- `_int_free`: 554,516,447 to 449,751,262;
- `malloc`: 466,796,492 to 358,131,824;
- `_int_malloc`: 334,822,467 to 246,013,150;
- `free`: 263,350,669 to 209,102,105.

Enum dispatch and the larger cell raise some term-local costs; for example,
`term_top_compare_for_problem` rises from 472,964,365 to 543,968,198
instructions. The measured allocator reduction outweighs those costs by
132.7 million instructions globally.

## Compatibility result

Final proof report `.artifacts/e-compare/20260721-011521-809318/` has zero
mismatches across GEO288, HEN011, LUSK6, and LUSK6ext. Final combined resource
report `.artifacts/e-compare/20260721-011725-324979/` has zero mismatches:
BOO020 and SWV851 both return normalized `ResourceOut` without allocator
failure. The earlier focused BOO report
`.artifacts/e-compare/20260721-011007-528295/` agrees, demonstrating that the
removed secondary allocations compensate for the larger base cell at the
maintained memory boundary.

## Validation

- `cargo fmt --all -- --check`
- 4,375 library tests plus all integration targets and features
- strict all-target, all-feature pedantic Clippy
- all-feature release `eprover` build
- all four documentation gates
- clean vendored C worktree

## Decision

Accept inline unary and binary argument storage with the boxed fallback for
larger arities. It is a safe, bounded approximation of C's one-allocation term
cell that improves both deterministic performance and the observed resource
boundary.
