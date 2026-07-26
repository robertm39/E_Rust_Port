# Experiment 336: LFHO explicit-bank and cache decision

## Status

Completed for the LFHO owner-bank/cache reconciliation cluster.

## Question

Must Rust reproduce C's per-term `owner_bank`, `binding_cache`,
`cache_binding`, and WHNF-cache fields to preserve observable higher-order
semantics or comparable performance?

## C mechanism

The `ENABLE_LFHO` C build stores hidden owner and cache pointers in term cells.
Read-looking dereference, ordering, printing, variable-distribution, matching,
and unification calls can therefore mutate cache state and allocate shared
normal forms without taking a bank argument. GC must mark those cache roots.

## Rust decision under audit

Rust keeps semantic mutable state (`binding`, rewrite replacement, and type) in
the compact term link cell and passes `&mut TermBank` explicitly wherever
normalization can allocate or share terms. Global read-only helpers expand
applied variables without a cache. This makes mutation and owner lifetime
visible and avoids a self-referential pointer into a movable `TermBank`.

The cache is not an observable proof result. Its observable obligations are:

- the same expanded or weak-head-normalized term;
- correct sharing where callers require bank identity;
- binding rollback and cache-freshness-equivalent results after rebinding;
- exact ordering/inference/output behavior; and
- comparable end-to-end performance.

## Evidence

[`audit_lfho_owner_cache.py`](audit_lfho_owner_cache.py) pins 15 current source
and retained-evidence contracts:

- explicit-bank WHNF, fixpoint, complete match/MGU, KBO6, and LPO4 entry
  points;
- the zero-suffix rewrite normalization boundary;
- no semantic dependency on nonexistent cache roots in GC or varhash;
- the 136-byte compact term cell;
- 21/21 higher-order unification projections;
- 18/18 higher-order forward-modification ordering configurations;
- the 73/73 ordering option matrix; and
- the fresh comprehensive 1.0801753448x Rust/C aggregate with zero unexpected
  compatibility differences.

C builds the extra fields only for LFHO, while Rust deliberately ships the
union of first- and higher-order behavior in one executable. Adding the three
owner/cache pointers directly would grow every 64-bit Rust term cell from 136
to approximately 160 bytes, a 17.647059% increase, before any WHNF-cache field
or invalidation structure. That would penalize all first-order workloads and
reverse a substantial part of the measured compact-term work. A bank side
table would avoid inline growth but add hashing, retention, invalidation, and
GC work without evidence that repeated LFHO dereference is a current hot-path
defect.

## Exact commands

```powershell
.\.venv\Scripts\python.exe `
  experiments/2026-07-25-035-lfho-explicit-bank-cache-decision/audit_lfho_owner_cache.py `
  --repo . `
  --expected experiments/2026-07-25-035-lfho-explicit-bank-cache-decision/audit-reference.json
```

## Results

- The new source/evidence audit passes all 15/15 contracts and exactly matches
  [`audit-reference.json`](audit-reference.json).
- The retained higher-order ownership/dispatch audit independently passes all
  14/14 checks with its original SHA-256
  `d5208301e10f70dc40d210b817a3f106e863f407baab6f61b3f43a38f525b66e`.
- The retained forward-modification audit independently passes all 9/9
  contracts, including live owner-bank orientation and every higher-order
  normalization hook.
- The evidence remains based on the fresh comprehensive Experiment 329
  lifecycle: 4,419 tests, Linux and Windows GNU builds, clean FOL/HO C builds,
  zero unexpected main/tool differences, and a 1.0801753448x aggregate.
- All four C-source documentation gates pass: 492/492 source files are
  covered, Change Later wording and local links pass across 269 Markdown
  files, and regeneration preserves all 268 manual-review files.
- Eight summarized and fourteen matched detailed Beads records were closed.
  The broader term-function/formula-owner and term-bank parser/output
  umbrellas remain open, as do the distinct `TBInsertOpt(DEREF_ALWAYS)` and
  substitution-normalization WHNF audits.
- No Rust or C source changed in this reconciliation.

## Falsification rule

Reject the decision if a production higher-order normalizer lacks a live bank,
any retained C/Rust semantic matrix differs, the fresh compatibility suite has
an unexpected mismatch, the maintained aggregate exceeds 1.10x C, or focused
profiling identifies repeated uncached LFHO dereference as a material
regression.

## Conclusion

Retain explicit mutable-bank ownership and no per-term LFHO cache. This is a
specific, measured Rust replacement for C's optimization rather than an
unimplemented semantic: exact higher-order outputs and state transitions are
preserved, end-to-end performance is comparable, and the alternative would
impose at least 24 bytes of cold LFHO state on every term in the unified
binary. Cache-aware GC marking is consequently unnecessary because no cache
owns a term.

Reopen only if a future focused higher-order profile demonstrates a material
repeated-dereference regression. Such a result should motivate a sparse
bank-owned cache with explicit invalidation, not raw term-to-bank pointers.

## Limits

- `TBInsertOpt(DEREF_ALWAYS)`'s process-global higher-order WHNF policy remains
  a separate semantic audit; this decision does not declare that branch
  complete.
- Full formula/parser and global type/output ownership remain separate from
  the term-cell cache decision.
- If future higher-order profiling falsifies the decision, prefer a bank-owned
  sparse cache with explicit invalidation over adding stale self-pointers to
  every term.
- C is not modified.
