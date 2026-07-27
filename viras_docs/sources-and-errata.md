# Sources, Provenance, Errata, and Ambiguities

## Sources actually used

### Local copies, deliberately excluded from version control

The three implementation-critical PDFs were downloaded into `viras_docs` on
July 26, 2026. Per project direction, the PDFs are local reference material
only: `/viras_docs/*.pdf` is ignored by Git, and none of these files is present
in repository history.

| Local filename | Bytes | Pages | SHA-256 |
|---|---:|---:|---|
| `viras-extended-easychair-13150-v2.pdf` | 746857 | 57 | `d3cc18758c49cea85361cd4980efec74de4ddaf58b0cf7825a7b7bf03eaa4d02` |
| `viras-lpar-2024.pdf` | 611554 | 18 | `8c4c1d23885357d33c4b681521873ac7eeb5ed6d992915dc9c210e6f63924d45` |
| `conflict-driven-virtual-substitution-2014.pdf` | 292908 | 15 | `4198fba502c4819a97dd6152cc37999255f5fca8f2f99de281149b39503ef7aa` |

The files can be reconstructed with PowerShell:

```powershell
$virasSourceDir = Resolve-Path "viras_docs"

Invoke-WebRequest `
  -Uri "https://easychair.org/publications/preprint/GcSq/download" `
  -OutFile "$virasSourceDir/viras-extended-easychair-13150-v2.pdf"

Invoke-WebRequest `
  -Uri "https://repositum.tuwien.at/bitstream/20.500.12708/199522/1/Schoisswohl-2024-VIRAS%20Conflict-Driven%20Quantifier%20Elimination%20for%20Integer...-vor.pdf" `
  -OutFile "$virasSourceDir/viras-lpar-2024.pdf"

Invoke-WebRequest `
  -Uri "https://korovin.gitlab.io/pub/virtual_substitution_learning.pdf" `
  -OutFile "$virasSourceDir/conflict-driven-virtual-substitution-2014.pdf"

Get-FileHash -Algorithm SHA256 "$virasSourceDir/*.pdf"
```

The EasyChair preprint publishing agreement states that EasyChair preprints
are open access; its operative agreement text applies
[CC BY-NC-ND 4.0](https://easychair.org/publications/preprint_agreement).
The TU Wien record applies the same license to the LPAR version of record. No
redistribution license was identified on the author-hosted CDVS PDF. Keeping
all three local-only avoids relying on redistribution rights and follows the
user's explicit source-control boundary.

### Primary VIRAS description

Johannes Schoisswohl, Laura Kovács, and Konstantin Korovin, "VIRAS:
Conflict-Driven Quantifier Elimination for Integer-Real Arithmetic (Extended
Version)," EasyChair Preprint 13150, version 2, May 7, 2024, 57 pages.

- [EasyChair record and current download](https://easychair.org/publications/preprint/GcSq)
- SHA-256 of the version-2 PDF inspected for this research:
  `d3cc18758c49cea85361cd4980efec74de4ddaf58b0cf7825a7b7bf03eaa4d02`
  (hexadecimal is case-insensitive)
- Accessed July 26, 2026

This is the implementation-critical source. It contains all main definitions,
the quantifier-elimination theorem, expanded examples, repeated definitions,
and proofs.

### Published VIRAS paper

Johannes Schoisswohl, Laura Kovács, and Konstantin Korovin, "VIRAS:
Conflict-Driven Quantifier Elimination for Integer-Real Arithmetic," in
*Proceedings of LPAR 2024*, EPiC Series in Computing 100, pages 147-164, 2024.

- [DOI 10.29007/kg4v](https://doi.org/10.29007/kg4v)
- [TU Wien repository record and version of record](https://repositum.tuwien.at/handle/20.500.12708/199522)

The TU Wien record labels the version of record
[CC BY-NC-ND 4.0](https://creativecommons.org/licenses/by-nc-nd/4.0/).
It corroborates the main definitions but delegates examples and proofs to the
extended preprint.

### Conflict-driven virtual substitution

Konstantin Korovin, Marek Košta, and Thomas Sturm, "Towards Conflict-Driven
Learning for Virtual Substitution," *Computer Algebra in Scientific Computing
2014*, pages 256-270.

- [Author-hosted PDF](https://korovin.gitlab.io/pub/virtual_substitution_learning.pdf)
- [DOI 10.1007/978-3-319-10515-4_19](https://doi.org/10.1007/978-3-319-10515-4_19)

VIRAS says CD-VIRAS changes only the CDVS `Leaf Conflict` and
`Inner Conflict` rules. The CDVS paper is therefore required to reconstruct the
rest of the conflict-driven state machine.

### Background sources

These sources explain the ancestry of the method but are not needed to fill any
missing VIRAS formula:

- Rüdiger Loos and Volker Weispfenning, "Applying Linear Quantifier
  Elimination," *The Computer Journal* 36(5), 450-462, 1993,
  [DOI 10.1093/comjnl/36.5.450](https://doi.org/10.1093/comjnl/36.5.450).
- Volker Weispfenning, "Mixed Real-Integer Linear Quantifier Elimination,"
  ISSAC 1999, 129-136,
  [DOI 10.1145/309831.309888](https://doi.org/10.1145/309831.309888).
- Thomas Sturm, "Thirty Years of Virtual Substitution: Foundations,
  Techniques, Applications," ISSAC 2018,
  [DOI 10.1145/3208976.3209030](https://doi.org/10.1145/3208976.3209030).
- David C. Cooper, "Theorem Proving in Arithmetic without Multiplication,"
  *Machine Intelligence* 7, 91-99, 1972.

## Excluded source

The VIRAS GitHub repository identified in the project's third-party inventory
has no license declaration. No file, source code, test, comment, generated
documentation, commit diff, or implementation detail from that repository was
used in this research.

## Definite errors in the VIRAS paper

These are errors in both the May 7 extended preprint and the LPAR version unless
otherwise noted.

### 1. Infinity substitution reverses periodic and aperiodic labels

Definition 9, case 3 (printed page 11 in the extended paper), says:

- use the infinity limit for an "aperiodic" literal with outer slope zero;
- remove infinity and evaluate the base for a "periodic" literal with outer
  slope nonzero.

Those parentheticals and conditions are reversed. Definition 15, Section 4.1,
Lemma 5, Lemma 6, the prose immediately before Definition 9, and the proofs of
Lemmas 18 and 19 all agree on the intended classification:

- periodic iff `oslp = 0`;
- aperiodic iff `oslp != 0`;
- at infinity, replace an aperiodic literal by its corresponding limit truth
  value;
- for a periodic literal, drop infinity and virtually substitute its finite
  base.

This correction is logically forced, not an optimization choice.

### 2. Example 10 changes ceiling to floor

The motivating formula uses:

`ceil(x) - x >= c`.

Example 10 prints `floor(x) - x >= c` instead. The motivating solution set,
the worked substitution, and the final answer `c <= 2/3` all require ceiling.
Keep the original ceiling literal (internally `-floor(-x) - x - c >= 0`).

Example 10 also joins per-literal elimination sets with conjunction symbols
where Definition 10 requires set union.

### 3. Example 10 negates a simple lower-bound witness

For

`t1 = x - floor(a) - 1/3`,

the zero is `floor(a) + 1/3`. Example 10 instead prints
`-floor(a) - 1/3` in the final elimination set. The motivating example and its
worked virtual substitution earlier in the paper use the positive value, and
Definition 4 gives the positive value directly.

Use `floor(a) + 1/3`.

### 4. Prose gives equality for a blocking infinity lemma

The prose before Definition 12 says that a periodic conflict yields
`rem_lambda(x) = rem_lambda(t)`. Definition 12 correctly gives the learned
blocking lemma as disequality:

`rem_lambda(x) != rem_lambda(t)`.

The disequality is required: the current residue class makes a periodic literal
false, so every solution must lie outside that class, and the learned lemma must
itself be false at the rejected virtual assignment. Lemma 7's intended argument
also requires disequality.

### 5. Several proof-only slips do not alter the algorithm

Examples include:

- a sentence in the V3 proof saying that aperiodic literals repeat, where the
  proof uses periodic literals;
- swapped empty/non-empty `breaks` case labels in part of Lemma 18;
- a typo in the statement of the solution-interval intersection lemma;
- relation and variable-name slips in several appendix calculations.

The main definitions, surrounding prose, and proof intent make these cases
unambiguous. They should not be copied as executable conditions.

## Underspecified points and conservative resolutions

### A. `lambda` is not defined in Definition 12

The infinity-lemma cases use `lambda` but Definition 12 never binds it. The
preceding prose and the proof require only that `lambda` be a positive common
period of every relevant periodic literal. The same paper already defines the
canonical construction in Definition 9:

`lambda = lcm_Q({period(L) | L is periodic})`.

Safe implementation resolution:

1. simplify or separately decide literals independent of the current
   elimination variable;
2. discard zero periods from the common-period calculation;
3. compute `lcm_Q` of the remaining positive periods;
4. the residue-conflict branch is unreachable if none remain, because with all
   aperiodic limits true there is no variable-dependent periodic literal left
   to cause that conflict.

Any positive common multiple is sound; the least rational common multiple is
the natural size-minimizing choice.

### B. `+/-` in virtual-substitution case V1 is metavariable notation

Case V1 says that if all aperiodic literals are true at a selected infinity,
finite grid representatives are decorated with that infinity. It does not
explicitly write a union when both signs qualify.

Safe implementation resolution: iterate over `sigma in {-1,+1}` and, for each
sign for which every aperiodic literal has true `sigma`-limit, add the
corresponding `s + sigma*infinity` candidates. Duplicates may be removed
structurally. Adding both when both qualify is a sound over-approximation and
matches the proof's sign-parametric reasoning.

### C. A grid period is printed as `1/sslp`

The discontinuity recurrence constructs the grid
`zero(b0) + (1/sslp)*Z`, while Definition 5 requires positive grid periods.
When `sslp < 0`, use `abs(1/sslp)`. The represented set is unchanged because
`p*Z = (-p)*Z`.

The same normalization is required for the period
`(1 - oslp/sslp)*p` in the aperiodic zero-grid construction.

### D. Common-period sets can syntactically contain zero

`lcm_Q` is defined only for finite sets without zero, but a literal independent
of `x` can have period zero and still be classified as periodic. Simplify
`x`-independent literals before candidate generation:

- an `x`-independent false literal makes the conjunction false;
- an `x`-independent true literal can be removed;
- zero periods are then absent from period LCMs.

At minimum, filter zero periods and handle the constant literal separately.

### E. Grid intersection is defined for positive interval width

Definition 5 states `k > 0`, while V2 can request a closed, zero-width core
interval when the bound over-approximation has `deltaX = 0`. Extend the
construction to `k = 0`:

- for a closed right endpoint, retain the single `n = 0` candidate;
- for an open right endpoint, return the empty set.

The construction is allowed to over-approximate the true grid intersection, so
the closed singleton candidate is safe even when symbolic parameters later show
that it is not on the grid; virtual substitution will reject it.

### F. V2 does not specify which equality to choose

If several aperiodic equality literals exist, V2 says to use the core interval
of "some" equality. Any one is sound because a solution of the conjunction must
lie in every equality's core interval. For determinism, choose the equality
whose rational core width predicts the fewest grid candidates. Intersecting
their core intervals could be a later optimization, but symbolic endpoint
ordering makes that a separate feature.

### G. Grid intersection deliberately over-approximates

The generated `n` range depends only on the rational interval width and not on
the symbolic offset between the interval start and the first grid point. It can
therefore emit an extra point beyond the true right boundary. Lemma 3 promises
a superset, not equality. Do not add a symbolic comparison requirement to the
correctness-critical first implementation; extra candidates are rejected by
substitution and simplification.

## Scope gaps that are not paper errors

### Typed integer variables

The paper uses one real domain plus floor; it does not define a separate integer
sort. A frontend with explicit integer quantifiers needs a semantics-preserving
adapter, for example:

- `exists z:Int. phi` becomes
  `exists z:Real. (z = floor(z) and phi)`;
- `forall z:Int. phi` becomes
  `forall z:Real. (z = floor(z) -> phi)`.

This adapter must also normalize casts and reject non-linear operations.

### Conflict-driven output

Appendix B describes CDVS as deciding a closed existential conjunction. Base
VIRAS is the documented symbolic quantifier-elimination procedure with free
parameters and arbitrary alternation. Treat CD-VIRAS first as a SAT/UNSAT
search mode for existentially closed conjunctions; do not assume it directly
constructs a compact parameterized quantifier-free formula.

### Implementation and complexity

The 2024 papers explicitly list implementation and tight complexity analysis as
future work. They provide correctness mathematics, not production data
structures, canonicalization rules, benchmark suites, or proof-object formats.
Those engineering choices in this packet are labeled as reconstruction or
recommendation rather than attributed to the authors.
