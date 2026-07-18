# Ax-filter selected-owner closure

## Status

Completed for Bead `E_Rust_Port-j76.2.40`. The earlier deferred stable-handle
note is resolved by the represented owner model, permanent relinking tests,
and a fresh 9/9 unchanged-C executable comparison. The vendored C source was
not modified.

## Ownership decision

C `StructFOFSpecGetProblem` returns non-owning clause and formula pointer
stacks. `e_axfilter::filter_problem` prints those stacks while the source
`StructFOFSpec` still owns every selected object, so this executable boundary
does not perform selected-axiom movement at all.

Rust preserves that contract with `PStack<&Clause>` and
`PStack<&WrappedFormula>`. A new regression applies the same threshold filter
twice, observes identical clause/formula identities both times, and confirms
the owning `StructFofSpec` still contains both sets and the formula entry.

C proof-state SInE separately moves raw pointers through intrusive current-set
links. Rust's safe replacement uses `Clause::alloc` process-unique identifiers
and allocation-unique `WrappedFormula::entry_id` values. Production
replacement computes each selected identity's last occurrence, drains each
source owner once into an identity map, and rebuilds in that order. This
preserves C's duplicate-pointer move-to-tail result in expected `O(n + k)`
time for `n` owned and `k` selected entries. Existing formula coverage and the
new clause counterpart each reverse 2,048 owners and repeat the original head
at the selection tail.

## Fresh executable comparison

The first current run exposed two test-boundary problems rather than selection
differences:

- the missing filter-file diagnostic used Rust scanner wording `Cannot open`
  instead of C `InputOpen` wording `Cannot stat`; and
- the THF LambdaDef fixture was incorrectly sent to the FOL C tool and omitted
  parentheses around each lambda body application required by the C parser.

`e_axfilter::load_filters` now performs the C metadata/regular-file preflight
before scanner construction and retains the C two-line `SysError` shape. The interop case now
declares `reference_mode: ho`, resolves the tool from the manifest-recorded HO
build, and uses C-valid lambda bodies.

The corrected matrix is 9/9 exact with zero expected differences:

- FOL help, version, default-filter dump, threshold, formula GSinE, seeded
  all/largest/diverse output, missing output parent, and missing filter file;
- HO formula LambdaDef output; and
- every declared stdout, stderr, exit status, configured output, and generated
  problem artifact.

The retained concise result is
[`comparison-reference.json`](comparison-reference.json). Its source report
SHA-256 is
`dc2eefb12f708598f8455a75835dcd4213d817232b352936ef10ca669804de9d`,
and it records the exact reference and candidate binary hashes.

## Performance decision

No selection algorithm changed. Borrowed `e_axfilter` results avoid object
clones, while destructive proof-state replacement remains the already-audited
single-drain `O(n + k)` implementation. The two 2,048-owner regressions cover
the performance-sensitive shape, so no additional benchmark is warranted.

## Validation

- corrected `e_axfilter` comparison: 9 cases, 0 mismatches, 0 expected differences;
- interop unit tests: 33 passed;
- focused repeated-borrow, 2,048-clause, and filter-file diagnostic tests;
- formatting, full all-target/all-feature tests, strict pedantic Clippy, release
  builds, and all documentation gates.
