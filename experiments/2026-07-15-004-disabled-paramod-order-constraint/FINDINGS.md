# Disabled paramodulation ordering constraint

## Question

Does C's optional `check_paramod_ordering_constraint` impose an observable
paramodulation filter that Rust still needs to port?

## Evidence

The audit used upstream commit
`17026b1bfe61aaf223cfaae54947c8d2679c31a0` and its cached optimized
higher-order executable.

1. The complete static helper in `CLAUSES/ccl_paramod.c` is enclosed by
   `#ifdef NEVER_DEFINED`.
2. A repository-wide definition search finds no source or build file that
   defines `NEVER_DEFINED`:

   ```powershell
   rg -n "^\s*#\s*define\s+NEVER_DEFINED\b" eprover
   ```

3. The only apparent use in `ClauseOrderedParamod` is inside a C block comment:
   `/* && check_paramod_ordering_constraint(ocb, from, into)*/`.
4. `nm -a` over the cached optimized higher-order reference contains no
   `check_paramod_ordering_constraint` symbol.

The disabled helper would sometimes call `ClauseNotGreaterEqual` and reject a
paramodulant. Adding it only to Rust could therefore change completeness,
generated-clause counts, proof order, and performance relative to the shipped C
executable.

## Decision

No Rust implementation is appropriate while the C definition and call remain
unreachable. This is an intentional compatibility omission, not incomplete
port behavior. If upstream C enables and configures the experiment later, it
will become a new observable feature that needs direct C/Rust inference and
performance coverage at that time.

No executable behavior changed in this slice, so a new differential run or
benchmark would not add evidence beyond the source/preprocessor/symbol audit.
