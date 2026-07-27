# Detailed CLAUSES reconciliation

## Status

Accepted for the 192 remaining open `clauses` records under Beads
`E_Rust_Port-j76.4`. Direct review of every migrated record, its unchanged C
source, the effective Rust owner, focused regressions, retained differential
evidence, and the current full lifecycle found no new production compatibility
gap.

The records resolve to checked C behavior, ownership-safe Rust equivalents,
work completed by later clause/formula/proof-control slices, or debug and
vendored PicoSAT surfaces that are not part of E's drop-in executable contract.
No Rust or C source changes are needed for this reconciliation.

## Decisions

The audit checks that the following four groups are a disjoint, exact cover of
all 192 migrated identities.

### Preserve the checked C behavior

Records
`135, 139, 144, 146, 152, 163, 166, 173, 180, 186, 188, 193, 209, 211,
218, 220, 221, 232, 233, 235, 236, 237, 239, 240, 241, 245, 294, 302,
305, 322, 327, 335, 347, 351, 352, 355, 373, 376, 379, 388, 390, 398,
400, 410, 416, 422, 423, 426, 427, 428, 434, 443, 450, 455, 459, 460,
462, 466, 467, 473, 477, 479, 480, 484, 485, 487, 492`.

These are explicit compatibility decisions rather than deferred work. They
retain the checked stack/list reversal and parent expansion rules, derivation
opcodes, proof-document timing, formula role and include behavior, CNF
ordering, split-definition ownership, rewrite and subsumption quirks,
quantifier/application grammar, `$distinct` and FOOL boundaries, app-encode
shape, and C's intentionally unused preprocessing arguments.

### Accept the ownership-safe Rust boundary

Records
`108, 113, 114, 116, 117, 118, 129, 130, 131, 133, 137, 138, 140, 141,
142, 143, 147, 153, 170, 174, 176, 185, 189, 204, 219, 227, 228, 247,
248, 259, 281, 282, 292, 303, 321, 338, 348, 349, 364, 367, 368, 369,
380, 399, 404, 405, 413, 418, 424, 425, 430, 431, 432, 436, 438, 444,
452, 468, 488, 490, 495`.

C uses uninitialized fields, raw clause/equation/term pointers, intrusive list
links, process globals, mutable term flags and bindings, shared signatures, and
allocator-address ordering throughout these records. Rust uses initialized
state, generation-qualified clause references, owned sets, explicit term-bank
and documentation sessions, scoped substitutions, and deterministic traversal.
Those choices preserve the supported observable contract without reproducing
undefined behavior, dangling-pointer risks, or allocator layout.

### Later implementation superseded the pending note

Records
`110, 112, 151, 155, 157, 162, 213, 215, 216, 230, 249, 250, 251, 252,
253, 255, 256, 257, 258, 264, 265, 267, 268, 269, 270, 271, 272, 273,
274, 275, 276, 277, 278, 318, 326, 334, 346, 360, 361, 362, 437, 440,
465, 469, 470, 471, 472, 474, 475, 478, 489, 491, 493, 497`.

The retained implementation now covers these paths:

- BCE and predicate-elimination tasks carry generation-qualified clause
  references and distinguish duplicate visible identifiers.
- `ProofObjectGraph` owns exact mixed clause/formula roots and C ordering, and
  derivation rendering covers all 56 operation codes.
- Production input converges on `ProofState::f_axioms`/`FormulaSet`; the
  formula set owns question annotation, FOOL, ITE/LET/lambda transforms,
  definition application, CNF documentation, archive transitions, app
  encoding, and higher-order unfolding.
- Direct higher-order paramodulation covers all six optimized-C orderings.
- PD-tree deletion, eta-normalized paths, exact clause occurrences, watchlist
  indexes, and formula/clause preprocessing ownership have dedicated closure
  evidence.

### Accept non-drop-in internal or vendored surfaces

Records
`119, 121, 125, 300, 311, 312, 336, 389, 415, 446`.

Raw splay/number-tree pointer printers, compile-time debug counters, an unused
allocation, a header typo, and a stream-identity diagnostic are internal C
debug surfaces, not E command-line compatibility requirements. PicoSAT's
unrelated utility API is likewise not an E-owned entry point. Rust supports
the eight reentrant PicoSAT operations E uses through an optional runtime
library and a tested internal fallback; that explicit deployment boundary was
accepted in Experiment 343.

## Evidence

The highest-risk ownership and behavior claims have independent retained
evidence:

- exact BCE and predicate-elimination duplicate-ID handling plus cross-bank
  tautology checks;
- exact clause-set PD-tree occurrence identity and eta-normalized deletion;
- mixed proof-object extraction, C ordering, all derivation metadata, PCL,
  TSTP, and DOT consumers;
- a nine-case fresh formula-route corpus, 28-case formula mode matrix, and the
  complete formula/CNF owner audit;
- all six higher-order ordering modes across 18 direct-paramodulation
  configurations;
- clausal preprocessing's 19/19 owner audit and the 29/29 umbrella closure;
  and
- runtime PicoSAT lifecycle, fallback, and exact predicate-elimination
  comparisons.

The source audit additionally checks permanent regressions for watchlists,
relevance, rewrite, splitting, subsumption, app encoding, and definition
unfolding.

## Audit

[`audit_clauses_reconciliation.py`](audit_clauses_reconciliation.py) pins every
migrated identity and content hash, incorporates the four disposition groups
into the decision digest, checks twelve grouped implementation/evidence
contracts, and digests every affected unchanged C source/header, generated C
source review, Rust clause module, supporting proof-control owner, retained
closure finding, and validation reference. It remains reproducible after issue
closure because status is excluded from the digest.

## Validation

No executable source changed after the comprehensive lifecycle used for the
preceding CONTROL reconciliation. Ephemeral Linode run
`.artifacts/linode/260726-234007-4e8d/` therefore validates this exact Rust/C
source snapshot:

- Rustfmt and strict all-target/all-feature pedantic Clippy pass;
- 4,419 library plus 11 integration tests pass, 4,430 total;
- native release and compile-only Windows GNU x64 all-target/all-feature
  builds pass;
- clean FOL and higher-order C references build and pass smoke checks;
- all 50 main-prover and all 216 support-tool cases have zero unexpected
  differences; and
- the ten-case aggregate is 1.083x Rust/C wall time.

The lifecycle wrote `SUCCESS` and `VALIDATION_COMPLETE`, retained its reports,
and deleted its Linode and firewall. No Rust or C toolchain ran on the local
Windows host, and the vendored C checkout was not modified.

Reproduce the source audit locally:

```powershell
.\.venv\Scripts\python.exe experiments/2026-07-25-054-clauses-reconciliation/audit_clauses_reconciliation.py `
  --repo . `
  --expected experiments/2026-07-25-054-clauses-reconciliation/audit-reference.json
```
