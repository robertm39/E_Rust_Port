# Experiment 241: Term-bank top-insert attribution

## Question

Which operations account for the remaining Rust/C gap in canonical
`TermBank::term_top_insert`, after the accepted direct rewrite construction
and the rejected term-cell-store mode cache, and what bounded ownership change
remains untested?

## Baseline

- Accepted source: Experiment 231.
- Exact LUSK6 Callgrind: 9,923,564,772 instructions.
- Original C Callgrind: 5,254,361,329 instructions.
- Rust/C ratio: 1.888634.
- Rust `TermBank::term_top_insert`: 260,571,383 exclusive instructions.
- C `tb_termtop_insert`: 127,261,512 exclusive instructions.

## Attribution

Experiment 237's optimized line-table profile is representative within
0.002362% of the accepted whole-program profile. Its source-attributed
`TermBank::term_top_insert` total exactly reproduces the accepted
260,571,383-instruction owner:

| Attributed component | Instructions |
| --- | ---: |
| `termbanks.rs` | 99,671,555 |
| `Cell`/`RefCell` operations | 37,572,080 |
| move/drop machinery | 37,301,691 |
| `termtypes.rs` | 30,847,242 |
| `Option` operations | 16,624,406 |
| `Rc` operations | 15,854,162 |
| intrinsic operations | 7,020,526 |
| signed-integer macros | 4,540,932 |
| non-null pointer operations | 3,489,860 |
| slice iteration | 2,513,385 |
| `simpletypes.rs` | 2,025,164 |
| mutable pointer operations | 1,573,050 |
| `Result` operations | 1,048,794 |
| enumeration | 488,232 |
| other | 304 |

There are 2,479,632 calls. Exact line counts identify 1,955,273 duplicate
insertions and 524,359 fresh insertions. The current boundary clones the
incoming `Term` before all 2,479,632 store calls; its source line alone costs
12,398,160 instructions before ownership work attributed to the callee and
standard-library machinery.

C transfers the unshared top-cell pointer into `TermCellStoreInsert`. A
duplicate returns the existing shared pointer and frees the input; a fresh
cell remains in the store and the caller continues using the same raw pointer.
Safe Rust needs two strong references for a fresh cell—one in the tree and one
returned to the bank—but it does not need to clone the input on the 78.855%
duplicate path.

## Result

Diagnostic only; executable source is unchanged.

Test a store insertion outcome that consumes the candidate top cell and
returns the stored handle plus fresh/duplicate status. On a fresh insertion,
clone the new splay root once for the caller; on a duplicate, return the
existing root without cloning the discarded input. Preserve the public
`TermCellStore::insert` API for standalone callers and use the new boundary
only from `TermBank::term_top_insert`.

Raw line data is reused from:

```text
.artifacts/experiments/2026-07-22-237-subst-norm-line-attribution/rust-callgrind-subst-norm-lines.out
```

Reproduction:

```bash
callgrind_annotate --inclusive=no --tree=both --auto=no --threshold=95 \
  .artifacts/experiments/2026-07-22-237-subst-norm-line-attribution/rust-callgrind-subst-norm-lines.out \
  src/terms/termbanks.rs
```
