# Experiment 246: Inline common substitution bindings

## Question

Can the substitution backtracking stack retain four bindings inline and spill
wider substitutions to a retained vector, eliminating its common first heap
growth without changing binding order, stack positions, or backtracking?

## Baseline

- Accepted source: Experiment 245, commit `e4555196`.
- Exact LUSK6 Callgrind: 9,898,434,766 instructions.
- Original C Callgrind: 5,254,361,329 instructions.
- Rust/C ratio: 1.883851.
- `Substitution::add_binding` causes 269,213 `RawVec` growth calls in the
  accepted profile.

## Candidate

Retain four `Option<Term>` slots directly in `Substitution`. This matches the
four-term capacity Rust's `Vec` obtains on its first growth. On a fifth live
binding, move the initialized prefix into a retained overflow vector and keep
using that vector until the substitution is fully backtracked. The next
independent binding sequence returns to inline storage while retaining overflow
capacity for later wide sequences.

Iteration joins the initialized inline prefix with overflow storage, preserving
the original binding order. Stack positions remain the total live length, and
pop/backtrack continue to clear variable bindings newest first.

## Validation

- All ten candidate substitution tests pass. The new regression covers six
  live bindings, ordered spill, partial backtracking, re-binding while
  overflow remains active, full backtracking, and subsequent inline reuse.
- Strict all-feature library pedantic Clippy and formatting pass for the
  candidate.
- The candidate reaches the exact 4,873-processed-clause LUSK6 proof and exits
  zero under Callgrind.
- After rejection, production source is restored byte-for-byte. All nine
  accepted substitution tests and formatting pass again.
- Native and compatibility gates were skipped after the decisive deterministic
  regression.

## Measurement

The candidate retires 10,032,883,355 instructions, 134,448,589 above the
9,898,434,766-instruction parent. This is a 1.358281% whole-prover regression,
and the hypothetical Rust/C ratio rises from 1.883851 to 1.909439.

Rust allocator calls do fall from 4,290,002 to 4,159,299, removing 130,703 or
3.046689%. The storage machinery nevertheless raises the comparable
binding-push, single-backtrack, and substitution-drop aggregate from
64,877,208 to 79,775,356 instructions, an increase of 14,898,148 or
22.963608%.

The larger substitution representation also changes the compiler's inlining
boundary in the dominant normalizer. The parent keeps always-dereference work
inside `Substitution::norm_term` at 437,245,456 instructions. The candidate
records 317,042,749 instructions in the normalizer plus a newly standalone
276,328,019-instruction `term_deref_always`, for a comparable 593,370,768.
That is an increase of 156,125,312 or 35.706560%. Recovering this boundary
would require coupling the storage change to the forced wrapper annotation
that Experiment 221 already rejected on native production evidence.

## Result

Reject. Safe inline substitution bindings remove allocations, but their
per-operation state, spill handling, larger stack frames, and compiler-layout
effects cost substantially more than the allocator work they avoid. Do not
combine this representation with the independently rejected forced-inline
wrapper merely to recover the accepted code-generation boundary. Production
source is restored exactly to Experiment 245 at 9,898,434,766 instructions,
or 1.883851 times C.

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=rust-callgrind-inline-substitution-bindings.out \
  target-wsl-246-inline-substitution-bindings/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```
