# PCL property-analysis compatibility edges

## Question

Does `pcl_propanalysis` still have a valid-input compatibility gap, or are its
remaining differences deliberate safe representations of undefined C
behavior?

## Source audit

`pcl_prot_global_count` skips FOF steps and empty clauses before incrementing
any aggregate clause, literal, or symbol counter. `PCLProtFindMaxStep` instead
scans every protocol step four times. Its comparators rank FOF steps below
non-FOF steps, treat FOF steps as equal, and read `logic.clause` for every
other step. Consequently, an empty clause is excluded from every aggregate but
can still be the longest, largest, heaviest, and deepest representative.

`PCLProtPropDataPrint` performs all ten divisions without denominator guards.
It then unconditionally prints each representative and passes the heaviest
representative's `logic.clause` union member to `ClausePropInfoPrint`. This
creates three invalid boundaries:

- an empty protocol supplies null representatives;
- a FOF-only protocol supplies a formula through the inactive clause union
  arm; and
- an internal shell-containing protocol is classified as non-FOF, so the
  comparators themselves read the inactive clause arm.

C `epclanalyse` rejects shell PCL during parsing, making the third case an
internal property-analysis boundary rather than an executable input surface.

## Archived C evidence

The archived upstream tool is commit
`17026b1bfe61aaf223cfaae54947c8d2679c31a0`, built with GCC 13.3 and glibc
2.39. Running its release `epclanalyse` on `empty-protocol.pcl` and
`fof-only.pcl` produces `Command terminated by signal 11` in both cases. This
confirms the source-level null/inactive-union analysis; neither crash is a
defined output contract to reproduce.

The permanent `zero-denominator-safe-boundary` corpus pairs one formula with
one empty clause. Both implementations exclude the empty clause from aggregate
counts, preserve all ten non-finite averages, select the empty clause for all
four maxima, and print standard weight zero. The empty clause keeps every C
representative access valid, so this is the executable compatibility oracle
for the count/max-scan split.

## Rust decision and regressions

Rust keeps the C arithmetic and selection rules while making invalid logical
content access impossible. Three focused core tests now pin:

- no representatives and ten `NaN` fields for an empty protocol;
- four formula representatives, no clause metrics, and zero aggregate clauses
  for a FOF-only protocol; and
- four shell representatives with zero metrics, zero aggregate clauses, and
  no clause-property access for a shell-only protocol.

The existing mixed FOF/empty-clause test pins exact safe-boundary rendering.
Aggregate tests continue to prove that non-empty positive, negative, and mixed
clauses alone contribute to counts.

The first current permanent executable report,
`.artifacts/e-compare/20260716-223141-877881-tools/`, found one of five cases
mismatched: Rust opened a missing named input directly through `Scanner`, while
C first used `InputOpen` and reported `Cannot stat file ...`. Rust now uses the
shared pre-open regular-file boundary and names stdin diagnostics `<stdin>`,
matching the adjacent support tools. The final report,
`.artifacts/e-compare/20260716-223836-192823-tools/`, has all five
`epclanalyse` cases exact.

## Compatibility decision

Zero-denominator output and empty-clause maximum eligibility remain visible
compatibility behavior. Null dereferences and inactive-union reads do not.
Rust's total rendering is the only safe representation that preserves the
defined portions of C behavior. Explicit unavailable markers remain deferred
post-compatibility cleanup because they would change the current output shape.

## Validation

The six focused property-analysis and 17 executable tests pass. The final
five-case archived-C differential is exact. Final repository gates pass
formatting, all-target/all-feature checking, pedantic Clippy with warnings
denied, all 4,192 library tests plus binary and integration targets, and a
locked release build of every binary. The 32 Python interoperability tests and
all four C-source documentation checks also pass.
