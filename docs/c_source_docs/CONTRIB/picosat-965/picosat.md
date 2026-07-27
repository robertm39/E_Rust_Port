<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# CONTRIB/picosat-965 / picosat

## Source Files

- [CONTRIB/picosat-965/picosat.h](../../../../eprover/CONTRIB/picosat-965/picosat.h)
- [CONTRIB/picosat-965/picosat.c](../../../../eprover/CONTRIB/picosat-965/picosat.c)

## Purpose

Vendored PicoSAT SAT solver implementation and public API.

Within the source tree, this unit belongs to `CONTRIB/picosat-965`. Vendored PicoSAT SAT-solver sources used through E's propositional/SAT integration paths. These files follow PicoSAT's API and allocation conventions.

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `PicoSAT`
- `picosat_free`
- `picosat_malloc`
- `picosat_realloc`

### Macros And Constants

- `ABORT(msg)`
- `ABORTIF(cond,msg)`
- `AVERAGE(a,b)`
- `BLK_FILL_BYTES`
- `CHECK_SORTED(cmp,a,n)`
- `CLR(p)`
- `CLRN(p,n)`
- `CLS2ACT(c)`
- `CLS2IDX(c)`
- `CLS2TRD(c)`
- `CMPSWAPFLT(a,b)`
- `COMPACT_TRACECHECK_TRACE_FMT`
- `DELETEN(p,n)`
- `ENDOFCLS(c)`
- `ENLARGE(start,head,end)`
- `EOC`
- `EPSFLT`
- `EXPORTIDX(idx)`
- `EXTENDED_TRACECHECK_TRACE_FMT`
- `FALSE`
- `FFLIPPED`
- `FFLIPPEDPREC`
- `FLTCARRY`
- `FLTEXPONENT(d)`
- `FLTMANTISSA(d)`
- `FLTMAXEXPONENT`
- `FLTMAXMANTISSA`
- `FLTMINEXPONENT`
- `FLTMSB`
- `FREDADJ`
- `FREDUCE`
- `FRESTART`
- `IDX2CLS(i)`
- `IDX2LIDX(idx)`
- `IDX2OIDX(idx)`
- `IDX2ZHN(i)`
- `INFFLT`
- `INSERTION_SORT(T,cmp,a,n)`
- `INSERTION_SORT_LIMIT`
- `INTERRUPTLIM`
- `ISLIDX(idx)`
- `ISLITREASON(C)`
- `LDMAXGLUE`
- `LIDX2IDX(idx)`
- `LIT2DHTPS(l)`
- `LIT2HTPS(l)`
- `LIT2IDX(l)`
- `LIT2IMPLS(l)`
- `LIT2INT(l)`
- `LIT2JWH(l)`
- `LIT2REASON(L)`
- `LIT2SGN(l)`
- `LIT2VAR(l)`
- `LOG(code)`
- `MAXCILS`
- `MAXGLUE`
- `MAXRESTART`
- `MINRESTART`
- `NADC`
- `NDSC`
- `NEWN(p,n)`
- `NFL`
- `NLUBY`
- `NOLOG(code)`
- `NOTLIT(l)`
- `NO_BINARY_CLAUSES`
- `NXC(p)`
- `OIDX2IDX(idx)`
- `ONLYLOG(code)`
- `PERCENT(a,b)`
- `PICOSAT_API_VERSION`
- `PICOSAT_REENTRANT_API`
- `PICOSAT_SATISFIABLE`
- `PICOSAT_UNKNOWN`
- `PICOSAT_UNSATISFIABLE`
- `PRIMES`
- `PTR2BLK(void_ptr)`
- `QUICKSORT(T,cmp,a,n)`
- `QUICKSORT_PARTITION(T,cmp,a,l,r)`
- `RDECIDE`
- ... 24 more

### Globals

- None found in the source scan.

### Exported Functions

- `PicoSAT * picosat_init (void)`
- `PicoSAT * picosat_minit (void * state, picosat_malloc, picosat_realloc, picosat_free)`
- `const char *picosat_config (void)`
- `const char *picosat_copyright (void)`
- `const char *picosat_version (void)`
- `const int * picosat_failed_assumptions (PicoSAT *)`
- `const int * picosat_humus (PicoSAT *, void (*callback)(void * state, int nmcs, int nhumus), void * state)`
- `const int * picosat_maximal_satisfiable_subset_of_assumptions (PicoSAT *)`
- `const int * picosat_mus_assumptions (PicoSAT *, void *, void(*)(void*,const int*),int)`
- `const int * picosat_next_maximal_satisfiable_subset_of_assumptions (PicoSAT *)`
- `const int * picosat_next_minimal_correcting_subset_of_assumptions (PicoSAT *)`
- `double picosat_seconds (PicoSAT *)`
- `double picosat_time_stamp (void)`
- `int picosat_add (PicoSAT *, int lit)`
- `int picosat_add_arg (PicoSAT *, ...)`
- `int picosat_add_lits (PicoSAT *, int * lits)`
- `int picosat_added_original_clauses (PicoSAT *)`
- `int picosat_changed (PicoSAT *)`
- `int picosat_context (PicoSAT *)`
- `int picosat_coreclause (PicoSAT *, int i)`
- `int picosat_corelit (PicoSAT *, int lit)`
- `int picosat_deref (PicoSAT *, int lit)`
- `int picosat_deref_partial (PicoSAT *, int lit)`
- `int picosat_deref_toplevel (PicoSAT *, int lit)`
- `int picosat_enable_trace_generation (PicoSAT *)`
- `int picosat_failed_assumption (PicoSAT *, int lit)`
- `int picosat_failed_context (PicoSAT *, int lit)`
- `int picosat_inc_max_var (PicoSAT *)`
- `int picosat_inconsistent (PicoSAT *)`
- `int picosat_pop (PicoSAT *)`
- `int picosat_push (PicoSAT *)`
- `int picosat_res (PicoSAT *)`
- `int picosat_sat (PicoSAT *, int decision_limit)`
- `int picosat_usedlit (PicoSAT *, int lit)`
- `int picosat_variables (PicoSAT *)`
- `size_t picosat_max_bytes_allocated (PicoSAT *)`
- `unsigned long long picosat_decisions (PicoSAT *)`
- `unsigned long long picosat_propagations (PicoSAT *)`
- `unsigned long long picosat_visits (PicoSAT *)`
- `void picosat_add_ado_lit (PicoSAT *, int)`
- `void picosat_adjust (PicoSAT *, int max_idx)`
- `void picosat_assume (PicoSAT *, int lit)`
- `void picosat_measure_all_calls (PicoSAT *)`
- `void picosat_message (PicoSAT *, int verbosity_level, const char * fmt, ...)`
- `void picosat_print (PicoSAT *, FILE *)`
- `void picosat_remove_learned (PicoSAT *, unsigned percentage)`
- `void picosat_reset (PicoSAT *)`
- `void picosat_reset_phases (PicoSAT *)`
- `void picosat_reset_scores (PicoSAT *)`
- `void picosat_save_original_clauses (PicoSAT *)`
- `void picosat_set_default_phase_lit (PicoSAT *, int lit, int phase)`
- `void picosat_set_global_default_phase (PicoSAT *, int)`
- `void picosat_set_incremental_rup_file (PicoSAT *, FILE * file, int m, int n)`
- `void picosat_set_interrupt (PicoSAT *, void * external_state, int (*interrupted)(void * external_state))`
- `void picosat_set_less_important_lit (PicoSAT *, int lit)`
- `void picosat_set_more_important_lit (PicoSAT *, int lit)`
- `void picosat_set_output (PicoSAT *, FILE *)`
- `void picosat_set_plain (PicoSAT *, int new_plain_value)`
- `void picosat_set_prefix (PicoSAT *, const char *)`
- `void picosat_set_propagation_limit (PicoSAT *, unsigned long long limit)`
- `void picosat_set_seed (PicoSAT *, unsigned random_number_generator_seed)`
- `void picosat_set_verbosity (PicoSAT *, int new_verbosity_level)`
- `void picosat_simplify (PicoSAT *)`
- `void picosat_stats (PicoSAT *)`
- `void picosat_write_clausal_core (PicoSAT *, FILE * core_file)`
- `void picosat_write_compact_trace (PicoSAT *, FILE * trace_file)`
- `void picosat_write_extended_trace (PicoSAT *, FILE * trace_file)`
- `void picosat_write_rup_trace (PicoSAT *, FILE * trace_file)`

## Implementation Notes

### Internal Functions

- `add_ado`
- `add_lit`
- `add_resolved`
- `add_simplified_clause`
- `add_zhain`
- `addflt`
- `adecide`
- `analyze`
- `ascii2flt`
- `assign_forced`
- `assign_phase`
- `assign_reason`
- `assume`
- `assume_contexts`
- `assumptions_satisfied`
- `avglevel`
- `backtrack`
- `base2flt`
- `bcp`
- `bcp_queue_is_empty`
- `bpushc`
- `bpushd`
- `bpushu`
- `bytes_clause`
- `check_mss_flags_clean`
- `check_ready`
- `check_sat_or_unsat_or_unknown_state`
- `check_sat_state`
- `check_trace_support_and_execute`
- `check_unsat_state`
- `clause_is_toplevel_satisfied`
- `clause_satisfied`
- `cmp_ado`
- `cmp_glue_activity_size`
- `cmp_inverse_jwh_rnk`
- `cmp_ptr`
- `cmp_resolved`
- `cmp_rnk`
- `cmpflt`
- `collect_clause`
- `collect_clauses`
- `connect_head_tail`
- `core`
- `crescore`
- `decide`
- `decide_phase`
- `delete`
- `delete_clause`
- `delete_clauses`
- `delete_prefix`
- `delete_zhain`
- `delete_zhains`
- `disconnect_clause`
- `drive`
- `dumpcls`
- `dumpclsnl`
- `dumplits`
- `dynamic_flips_per_assignment_per_mille`
- `end_of_lits`
- `enlarge`
- `enlarge_adotab`
- `enter`
- `enumstr`
- `extract_all_failed_assumptions`
- `faillits`
- `fanalyze`
- `find_ado`
- `fix_added_lits`
- `fix_ado`
- `fix_ados`
- `fix_assumed_lits`
- `fix_clause_lits`
- `fix_cls_lits`
- `fix_heap_rnks`
- `fix_impl_lits`
- `fix_trail_lits`
- `fixvar`
- `flbcp`
- `flt2double`
- `force`
- `gcd`
- `hash_ado`
- `hashlevel`
- `hdown`
- `high_agility`
- `hpop`
- `htop`
- `hup`
- `impl2reason`
- `import_lit`
- `inc_activity`
- `inc_cinc`
- `inc_ddrestart`
- `inc_drestart`
- `inc_lreduce`
- `inc_lrestart`
- `inc_score`
- `inc_vinc`
- `incincs`
- `incjwh`
- `init`
- `init_reduce`
- `init_restart`
- `int2lit`
- `int2unsigned`
- `iteration`
- `leave`
- `lit_has_binary_clauses`
- `llength`
- `log2flt`
- `lpush`
- `lrelease`
- `luby`
- `mark_clause_to_be_collected`
- `mark_var`
- `mb`
- `medium_agility`
- `minautarky`
- `mss`
- `mulflt`
- ... 64 more

### Source-Level Behavior

- No structured function-comment blocks were found; rely on the declaration lists and direct source review.

### Dependencies

- `"picosat.h"`
- `<R.h>`
- `<assert.h>`
- `<ctype.h>`
- `<limits.h>`
- `<stdarg.h>`
- `<stddef.h>`
- `<stdint.h>`
- `<stdio.h>`
- `<stdlib.h>`
- `<string.h>`
- `<sys/resource.h>`
- `<sys/time.h>`
- `<sys/unistd.h>`
- `<unistd.h>`

### Compile-Time Conditions

- `LOGGING`
- `NADC`
- `NDEBUG`
- `NDEDBUG`
- `NDSC`
- `NFL`
- `NGETRUSAGE`
- `NLUBY`
- `NO_BINARY_CLAUSES`
- `RCODE`
- `STATS`
- `STATSA`
- `TRACE`
- `VISCORES`
- `WRITEGIF`
- `picosat_h_INCLUDED`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Preserve compile-time feature gates and debug-only behavior as explicit Rust configuration or narrowly scoped runtime options.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `CONTRIB/picosat-965/picosat.h`, `CONTRIB/picosat-965/picosat.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `CONTRIB/picosat-965` covering 2 source file(s), about 9162 lines, 88 scanned public declarations, 184 scanned internal function definitions, and 0 structured function-comment blocks.
- Vendored PicoSAT solver implementation and public API. E integration should depend on the documented API boundary, not internal solver globals.
- Vendored PicoSAT code. Keep the boundary explicit: document API expectations and integration points, but avoid blending PicoSAT implementation assumptions into E-owned Rust modules.
- SAT/propositional integration has a separate assignment/result vocabulary; keep conversions and ownership boundaries explicit.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- File-static state should be audited for thread-safety and reset behavior in the Rust port.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.

### Change Later

- E's C build vendors the full PicoSAT implementation in-tree, while Rust currently treats PicoSAT as an optional runtime-loaded shared library behind a narrow FFI wrapper and falls back to the internal solver when no library is found. After drop-in compatibility is secured, decide whether vendoring, runtime loading, or a Cargo feature should be the supported deployment model.
- PicoSAT exposes a large mutable solver object with custom allocator hooks, optional trace/core support, and many compile-time feature branches. Rust should keep the safe wrapper focused on the API calls E actually uses; broader API coverage should be added only with lifecycle tests for ownership, reset behavior, and trace/core semantics.
- The C solver uses extensive internal assertions and process-level diagnostics. If Rust ever ports the solver itself instead of loading it, model solver-state invariants explicitly rather than translating assertion failures into ordinary recoverable errors.
<!-- END MANUAL REVIEW: c_source_docs -->
