<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# HEURISTICS / che_to_weightgen

## Source Files

- [HEURISTICS/che_to_weightgen.h](../../../eprover/HEURISTICS/che_to_weightgen.h)
- [HEURISTICS/che_to_weightgen.c](../../../eprover/HEURISTICS/che_to_weightgen.c)

## Purpose

Routines for generating weights for term orderings the GNU Lesser General Public License.

Within the source tree, this unit belongs to `HEURISTICS`. Clause evaluation and strategy machinery: heuristic control blocks, priority functions, weight functions, literal selection, automatic strategy selection, and feature extraction.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- None found in the source scan.

### Macros And Constants

- `CHE_TO_WEIGHTGEN`
- `TOGenerateDefaultWeights(ocb)`
- `W_TO_BASEWEIGHT`

### Globals

- None found in the source scan.

### Exported Functions

- `TOGenerateWeights((ocb), NULL, WSelectMaximal, \ W_DEFAULT_WEIGHT) void TOGenerateWeights(OCB_p ocb, ClauseSet_p axioms, char *pre_weights, OrderParms_p oparms)`

## Implementation Notes

### Internal Functions

- `find_max_symbols`
- `generate_arity_weights`
- `generate_comb_freq_rank_weights`
- `generate_comb_freq_weights`
- `generate_constant_weights`
- `generate_freq_weights`
- `generate_freqrank_weights`
- `generate_freqranksq_weights`
- `generate_inv_comb_freq_rank_weights`
- `generate_inv_comb_freq_weights`
- `generate_inv_modfreqrank_weights`
- `generate_inv_modfreqrank_weights_max_0`
- `generate_inv_type_freq_rank_weights`
- `generate_inv_type_freq_weights`
- `generate_invconjfreqrank_weights`
- `generate_invfreq_weights`
- `generate_invfreqrank_weights`
- `generate_invfreqranksq_weights`
- `generate_invprecedence_weights`
- `generate_precedence_weights`
- `generate_precrank10_weights`
- `generate_precrank20_weights`
- `generate_precrank5_weights`
- `generate_precrank_weights`
- `generate_selmax_weights`
- `generate_type_freq_rank_weights`
- `generate_type_freq_weights`
- `print_weight_array`
- `set_maximal_0`
- `set_maximal_unary_0`

### Source-Level Behavior

- `print_weight_array`: Print the function symbol weights.
- `prec_rank_cell_cmp`: Comparison function for sorting signatures by precedence.
- `generate_precrank_weights`: Sort symbols by precedence, split them into n ranks, then assign weights to each rank (lowest rank = 1, then up). Assumes a total precedence (and will make it total by alpha-rank in a pinch). Note to self: Is is always kosher? Better only try on complete precedences (but then, most are).
- `find_max_symbols`: Find all maximal (in the precedence) symbols in ocb->sig and return a stack containing them.
- `set_maximal_0`: Set the weight of the first non-constant maximal symbol in OCB to 0.
- `set_maximal_unary_0`: Set the weight of the first unary maximal symbol in OCB to 0.
- `generate_constant_weights`: Assign the constant W_DEFAULT_WEIGHT to all smbols.
- `generate_selmax_weights`: Assign weight W_DEFAULT_WEIGHT to all symbols except the first maximal one, which get weight 0. Constants alway get W_DEFAULT_WEIGHT.
- `generate_arity_weights`: Generate arity-based weights for function symbols.
- `generate_precedence_weights`: Weight(f) = |{g|g<f}|, i.e. weight is number of smaller function symbols in the signature (+1).
- `generate_invprecedence_weights`: Weight(f) = |{g|g>f}|, i.e. weight is number of bigger function symbols in the signature (+1).
- `generate_precrank5_weights`: Weight(f) = rank (of 5) in the precedence.
- `generate_precrank10_weights`: Weight(f) = rank (of 10) in the precedence.
- `generate_precrank20_weights`: Weight(f) = rank (of 20) in the precedence.
- `generate_freq_weights`: Make the weight of a function symbol equal to its frequency count in the axiom set.
- `generate_type_freq_weights`: Assign function symbols weights that are equal to sum of occurrences of all function symbols that are of the same type.
- `generate_comb_freq_weights`: Assign function symbols weights that are equal to sum of occurrences of all function symbols that are of the same type + double the occurrence of the symbol itself.
- `generate_inv_comb_freq_weights`: Inverse version of generate_comb_freq_weights()
- `generate_inv_typefreq_weights`: Assign function symbols weights that are equal to difference of maximal sum of occurences of symbols of one type and sum of occurrences of all function symbols that are of the same type.
- `generate_invfreq_weights`: Make the weight of a function symbol equal to the maximum frequency count minus its frequency count in the axiom set.
- `generate_freqrank_weights`: Make the weight of a function symbol equal to its "frequency rank".
- `generate_type_freq_rank_weights`: Make the weight of a function symbol equal "frequency rank" of its type.
- `generate_comb_freq_rank_weights`: Make the weight of a function symbol equal to rank of "frequency of type + 2*frequency of symbol"
- `generate_invfreqrank_weights`: Make the weight of a function symbol equal to its inverse "frequency rank".
- `generate_inv_type_freq_rank_weights`: Make the weight of a function symbol equal to inverse of its type "frequency rank".
- `generate_inv_comb_freq_rank_weights`: Make the weight of a function symbol equal to inverse of its type "frequency rank".
- `generate_invconjfreqrank_weights`: Make the weight of a function symbol equal to its inverse "conjecture frequency rank".
- `generate_freqranksq_weights`: Make the weight of a function symbol equal to its "frequency rank" squared.
- `generate_invfreqranksq_weights`: Make the weight of a function symbol equal to the square of its inverse "frequency rank".
- `generate_inv_modfreqrank_weights`: Make the weight of a function symbol equal to its modified frequency rank.
- `generate_inv_modfreqrank_weights_max0`: Make the weight of a function symbol equal to its modified frequency rank, but make the first unary maximal symbol 0.
- `set_user_weights`: Given a user weight string, set the symbols to the desired weight.
- `TOTranslateWeightGenMethod`: Given a string, return the corresponding TOWeightGenMethod token.
- `TOGenerateWeights`: Given a pre-initialized OCB, assign weights to the function symbols. Some methods require a precedence, some require the axioms.

### Dependencies

- `"che_to_weightgen.h"`
- `<che_fcode_featurearrays.h>`
- `<che_to_params.h>`

### Compile-Time Conditions

- `CHE_TO_WEIGHTGEN`
- `ENABLE_LFHO`
- `PRINT_FUNWEIGHTS`

## E Reference Notes

- Use the ownership model visible in this unit's allocation/free helpers and exported APIs as evidence; preserve it only where correctness, supported compatibility, or measured performance requires it.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Audit compile-time feature gates and debug-only behavior; map supported variants to explicit Rust configuration or document why Umlaut intentionally chooses one path.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions reveal performance assumptions; measure allocation granularity and reuse before choosing Umlaut's representation.
- Container APIs often transfer raw pointers without ownership annotations; document and encode ownership at the Rust boundary.
- Audit where clause/literal mutation order affects indexing, derivation, proof reconstruction, or deterministic behavior before changing it.
- Parser routines usually advance scanner state and may report fatal errors; preserve supported input behavior or document and test an intentional divergence.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `HEURISTICS/che_to_weightgen.h`, `HEURISTICS/che_to_weightgen.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `HEURISTICS` covering 2 source file(s), about 1548 lines, 3 scanned public declarations, 30 scanned internal function definitions, and 34 structured function-comment blocks.
- Routines for generating weights for term orderings the GNU Lesser General Public License.
- Strategy code. Preserve clause-evaluation semantics, priority ordering, weight formulas, and feature extraction because small changes can alter search behavior.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Ordering comparisons feed simplification and inference eligibility; preserve tie-breakers, cache use, and incomparability results.
- Heuristic values are part of strategy behavior; preserve formulae, defaults, and parse names before optimizing.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Compatibility Notes

- `TOGenerateWeights` loops over every f-code after `SIG_TRUE_CODE`, including built-in internal symbols. Rust tests should compute ranks, maxima, and frequency maxima over the full signature rather than only over symbols introduced by a test.
- `WNoMethod` falls through to `generate_selmax_weights`, so a precedence is still required. `set_maximal_0` silently does nothing for higher-order problems through the global `problemType`, while `set_maximal_unary_0` does not have that higher-order skip. Production Rust now passes the parsed problem type explicitly into ordering creation, preventing higher-order runs from incorrectly assigning zero weight to the first maximal user symbol.
- The constant-weight post-pass overwrites arity/frequency/precedence-generated weights for arity-zero symbols whenever `to_const_weight != WConstNoSpecialWeight`, using `MAX(to_const_weight, 1)` without multiplying by `W_DEFAULT_WEIGHT`. Since `W_DEFAULT_WEIGHT` is currently 1 this is numerically invisible, but the formula is still a compatibility detail.
- `WModArityWeight` and `WModArityMax0` set `ocb->var_weight` to `W_TO_BASEWEIGHT` only for `WConstNoSpecialWeight`; otherwise they copy `to_const_weight` directly, so the default `WConstNoWeight` can make `$true`/variable weight zero unless the later constant-forcing option lowers it differently.
- Several rank methods intentionally allow zero symbol weights because their sentinel variables start at zero and no `MAX(weight, 1)` clamp is applied. This includes the square-rank paths, inverse conjecture-frequency rank for the initial zero/zero class, and modified inverse frequency rank for all-zero frequency groups.
- The LFHO combined-frequency count method builds type counts by summing `FCodeFeatureArray` symbol frequencies by type, while inverse combined count and the combined rank variants use `ClauseSetAddTypeDistribution`. Preserve the inconsistency until reference strategy tests prove a cleanup is safe.
- `ENABLE_LFHO` inserts eight type/combined-frequency methods into the middle of `TOWeightGenNames`; a non-LFHO executable rejects those names and omits them from diagnostics. Rust intentionally exposes the union in one executable, and the complete method/diagnostic surface is reference-tested against matching FOL and higher-order C builds.
- `generate_comb_freq_rank_weights` frees `type_counts` with `SizeFree(type_counts, sizeof(max_types*sizeof(long)))`, which passes the size of the expression rather than the allocated byte count. This is a C allocation-accounting hazard, not semantic weight behavior, and should be cleaned only after compatibility-sensitive memory tracing is out of scope.
- Precedence-dependent weight generation uses `OCBFunCompare` directly, so predefined partial precedences do not invent comparisons for `WPrecedence`/`WInvPrecedence`: incomparable symbols contribute to neither count. `generate_precrank_weights` is the exception that forces a total sort by falling back to `SigGetAlphaRank` for incomparable pairs. Rust preserves both behaviors; revisit only if strategy compatibility tests allow a clearer partial-precedence policy.
- `generate_precrank_weights` uses single-precision `float` division and assigns the result to `long`, truncating the bucket value. Rust should keep that conversion shape for byte-for-byte rank bucket compatibility.
- `set_user_weights` parses a user string through `TOWeightsParse` after generated weights and prints `setting user weights` to stderr first. Rust preserves the late OCB override and now emits the exact line from the executable proof-search owner when final strategy parameters select KBO/KBO6 with non-null predefined weights; reusable ordering helpers remain output-free, and LPO continues to ignore the override without the line.
- An isolated `-DPRINT_FUNWEIGHTS` build of the unchanged C source exposes the actual post-generation OCB arrays. Fifteen retained snapshots cover late overrides, partial precedence counting and pre-rank totalization, zero-sentinel rank methods, and all eight LFHO type/combined frequency variants; the paired permanent Rust regression matches every user-symbol weight exactly. The instrumentation and results are recorded in [`experiments/2026-07-17-071-weightgen-state/FINDINGS.md`](../../../experiments/2026-07-17-071-weightgen-state/FINDINGS.md).

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
