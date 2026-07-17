# Term-bank non-clone proof rendering

## Objective

Resolve `E_Rust_Port-j76.2.104` and the paired `cte_termbanks` ownership review by removing the shallow term-bank copy surface without changing proof output or canonical term identity. The vendored C source remains unchanged.

## Ownership audit

`TermCellStore` stores intrusive search-tree links in shared `TermCell` objects. Deriving `Clone` therefore copied store roots and counters while retaining the same linked cells; independently mutating either copy could relink the other's store. The only production `TermBank::clone()` calls were proof-object list and DOT formula renderers. They needed the live formula handles and signature, but used the clone only because formula printing accepted `&mut TermBank`.

Rust now makes `TermBank`, `TermCellStore`, and aggregate owners containing a bank non-cloneable. Formula printing constructs read-only temporary `Eqn` views, and clause-backed proof output collects a read-only temporary `Clause` view. These paths preserve literal normalization, type checks, roles, source information, and TSTP/PCL spelling without declaring predicates, setting term properties, inserting terms, or mutating the signature. Parser capability lookahead continues to use `TermBank::detached_empty()`, which owns an independent store and a signature snapshot.

## Regression evidence

`proof_object_formula_rendering_borrows_canonical_term_bank_read_only` exercises TSTP proof-list and DOT output for both a formula-backed equality and a clause-backed formula. It snapshots every stored term property plus input, insertion, recovery, node, argument, and storage counters; after both renderers run, it verifies the snapshots and every canonical `find` identity are unchanged.

The former test-only whole-`ProofState` clone was removed. Its AC-parent archive/requeue assertions now operate on the real owning state, retaining the intended derivation-identity coverage without requiring an invalid owner snapshot.

## Validation

All validation passed:

- 49 focused proof-object, formula-printer, detached-probe, and AC-parent tests;
- 4,233 default-feature library tests;
- 4,238 all-feature library tests, every binary target, and all seven integration tests serially;
- locked all-target/all-feature Clippy with warnings and `clippy::pedantic` denied;
- formatting and all four C-source documentation gates;
- optimized `eprover` build;
- unchanged, clean vendored `eprover/` checkout.
