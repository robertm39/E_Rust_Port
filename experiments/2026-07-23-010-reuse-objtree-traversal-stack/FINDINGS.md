# Experiment 248: Reuse object-tree traversal stack per index query

## Question

Can one query-scoped traversal stack replace the fresh `ObjTreeIter` stack for
every fingerprint-index payload without changing payload order, proof search,
or unbounded retained state?

## Baseline

- Accepted source: Experiment 245, commit `e4555196`.
- Exact LUSK6 Callgrind: 9,898,434,766 instructions.
- Original C Callgrind: 5,254,361,329 instructions.
- Rust/C ratio: 1.883851.
- The accepted profile records 46,551 object-tree traversals in 3,174 hot
  overlap-index queries and 56,911 `RawVec` growth calls owned by
  `ObjTreeIter::next`.
- Accepted whole-program Rust allocation calls: 4,290,002.

## Candidates

Both candidates add operation-local pending-node storage to the overlap and
subterm index queries and reuse it across the payload trees returned by the
fingerprint index. The storage is dropped at the end of each query.

The first candidate traverses each object tree directly into the result
vector. The refinement retains Rust's former `Iterator` plus `Vec::extend`
shape, but its iterator borrows the query-scoped pending-node vector instead
of owning a fresh vector. Both preserve the former in-order payload walk and
the reverse fingerprint-payload order.

## Validation

- The direct candidate passes nine object-tree, six subterm-index, and nine
  overlap-index tests. Its new regression checks concatenated in-order output,
  an empty stack after each traversal, and retained capacity across trees.
- Strict all-feature library pedantic Clippy, formatting, and `git diff
  --check` pass for the direct candidate.
- The iterator refinement passes the nine object-tree tests and formatting.
- Both candidates reach the exact 4,873-processed-clause LUSK6 proof and exit
  zero under Callgrind.
- Four alternating warmup pairs precede each native candidate. Every warmup
  and all 192 measured pairs prove and exit zero.
- Production source and the candidate-only test are restored byte-for-byte to
  Experiment 245 after native rejection. The 23 accepted object-tree and index
  tests plus formatting pass after restoration.
- Compatibility and resource matrices are skipped after replicated native
  rejection.

## Measurement

### Direct collector

The direct candidate retires 9,880,198,850 instructions, 18,235,916 below the
9,898,434,766-instruction parent. This is a 0.184230% whole-prover reduction,
and the hypothetical Rust/C ratio improves from 1.883851 to 1.880381.

Rust allocation calls fall from 4,290,002 to 4,245,063, a reduction of 44,939
or 1.047529%. Traversal-owned growth calls fall from 56,911 to 18,475, down
38,436 or 67.537032%. The direct collector's exclusive work is 7,957,964
instructions versus 10,985,676 in the former iterator `next` function.

Native production evidence reverses the deterministic result in two
independently started 64-pair blocks. Across all 128 pairs, candidate wall and
CPU means regress 0.427254% and 0.499930%; medians regress 0.702104% and tie;
mean paired wall and CPU changes regress 0.623698% and 0.678676%. The
candidate wins 65 wall and 62 CPU pairs, with eight CPU ties.

Across the combined stable last 32 pairs of both blocks, wall mean improves
0.073052% while CPU mean regresses 0.281294%. Wall median regresses 0.426892%
and CPU median improves 0.450450%; paired wall and CPU means regress 0.100236%
and 0.453493%. The mixed stable-half wall statistics do not overcome two
same-direction full-block mean regressions. The candidate binary is 1,536
bytes smaller than the 8,654,336-byte parent.

### Borrowed-stack iterator

The iterator refinement retires 9,885,381,708 instructions, 13,053,058 below
the parent, a 0.131870% reduction and a hypothetical Rust/C ratio of
1.881367. It preserves the same 4,245,063 Rust allocation calls, but costs
5,182,858 more instructions than direct collection.

Its 64-pair native block regresses wall and CPU means 0.152223% and
1.061008%; medians regress 0.744614% and 1.515152%; mean paired changes
regress 0.214312% and 1.136844%. It wins 31 wall pairs but only 19 CPU pairs,
with ten CPU ties.

The stable last 32 pairs remain worse: wall and CPU means regress 0.410810%
and 1.044634%, medians regress 0.803203% and 1.538462%, and paired means
regress 0.497483% and 1.167472%. The refinement binary is the same size as the
parent.

## Result

Reject both candidates. Query-scoped traversal storage removes 44,939
allocations and improves exact instrumented work, but direct collection
regresses both independent native blocks and the iterator refinement produces
an even clearer native CPU regression. Preserve the fresh `ObjTreeIter` stack
and former `Vec::extend` path. Production source remains Experiment 245 at
9,898,434,766 instructions, or 1.883851 times C.

The result rules out both direct result collection and a borrowed-stack
iterator at this reuse boundary. A future object-tree traversal change needs
a materially different representation or new profile evidence rather than
another code-shape variation of this experiment.

## Raw artifacts

- Direct Callgrind:
  `.artifacts/experiments/2026-07-23-010-reuse-objtree-traversal-stack/rust-callgrind-reuse-objtree-traversal-stack.out`
- Iterator Callgrind:
  `.artifacts/experiments/2026-07-23-010-reuse-objtree-traversal-stack/rust-callgrind-reuse-objtree-scratch-iterator.out`
- Direct native blocks: `native-lusk.csv` and `native-lusk-2.csv`.
- Iterator native block: `native-iterator-lusk.csv`.

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=rust-callgrind-reuse-objtree-traversal-stack.out \
  target-wsl-248-reuse-objtree-traversal-stack/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```

```powershell
& .\experiments\2026-07-22-214-move-termtree-insert-links\run-native.ps1 `
  -ParentExe .\target\native-245-single-maximal-candidate-vector\release\eprover.exe `
  -CandidateExe .\target\native-248-reuse-objtree-traversal-stack\release\eprover.exe `
  -Pairs 64 `
  -OutputCsv .\experiments\2026-07-23-010-reuse-objtree-traversal-stack\native-lusk.csv
```
