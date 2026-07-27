# Experiment 239: Term-tree insertion line attribution

## Question

Which operations dominate the accepted 658,858,502-instruction
`TermTree::insert`, after the accepted child-link moves and the rejected
non-owning splay-tail representation, and what bounded ownership candidate
remains untested?

## Baseline

- Accepted source: Experiment 231.
- Exact LUSK6 Callgrind: 9,923,564,772 instructions.
- Original C Callgrind: 5,254,361,329 instructions.
- Rust/C ratio: 1.888634.
- Rust `TermTree::insert`: 658,858,502 exclusive instructions.
- C `TermTreeInsert` plus `splay_term_tree`: approximately 333.6 million
  instructions.

## Attribution

Experiment 237's optimized line-table profile is representative within
0.002362% of the accepted whole-program profile. In that build,
`TermTree::insert` costs 659,166,400 instructions, only 307,898 or 0.046733%
above the accepted 658,858,502-instruction owner.

Inlining attributes the function across its source, `Term` accessors, and
standard-library machinery:

| Attributed component | Instructions |
| --- | ---: |
| `termtrees.rs` | 217,095,394 |
| `termtypes.rs` | 97,392,897 |
| `Cell`/`RefCell` operations | 69,963,385 |
| comparison machinery | 64,239,913 |
| `Option` operations | 57,365,976 |
| argument `zip` adapter | 52,599,628 |
| move/drop machinery | 48,550,320 |
| pointer machinery | 12,950,972 |
| signed-integer comparison macros | 12,048,072 |
| `Rc` operations | 8,533,068 |
| `problem_type()` body | 7,340,574 |
| intrinsic operations | 5,133,182 |
| thread-local access | 4,893,716 |
| unsigned-integer macros | 1,059,303 |

This closely reproduces Experiment 216 after the later accepted work. The
52.6-million-instruction zipped argument comparison was already falsified by
Experiment 217, where an indexed loop regressed both the intended owner and
whole-program code generation. Experiment 236 likewise falsified non-owning
splay tails.

The remaining distinct boundary is the syntax-mode read. There are 2,479,632
insertion calls: 32,774 empty-tree insertions and 2,446,858 nonempty
insertions. Every nonempty insertion reads the thread-local problem type once
before splaying. The `problem_type()` body plus thread-local access costs
12,234,290 instructions, or 1.856% of the line-table insertion owner and
0.123% of the whole profile.

`TermCellStore` is the only production owner of `TermTree`; one store contains
all 32,768 hash-bucket trees. A single store-local cache of the first
initialized problem type can therefore avoid repeated thread-local access
without adding mode state to every tree. Calls made while the global mode is
still `NotInitialized` must remain uncached, and tests must prove that later
initialization is observed.

## Result

Diagnostic only; executable source is unchanged.

Test the store-local initialized-mode cache separately. Pass the selected mode
to private `TermTree` find/insert/extract operations while preserving the
existing public global-mode wrappers, comparator key, splay topology, and
standalone tests. Treat 12,234,290 instructions as an affected-area upper
bound, not an expected saving: a cache branch and field access replace the
thread-local lookup.

Raw line data is reused from:

```text
.artifacts/experiments/2026-07-22-237-subst-norm-line-attribution/rust-callgrind-subst-norm-lines.out
```

Reproduction:

```bash
callgrind_annotate --inclusive=no --tree=both --auto=no --threshold=0 \
  .artifacts/experiments/2026-07-22-237-subst-norm-line-attribution/rust-callgrind-subst-norm-lines.out \
  src/terms/termtrees.rs
```
