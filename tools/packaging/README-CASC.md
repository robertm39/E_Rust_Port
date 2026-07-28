# Umlaut StarExec installation package

`verify_casc_package.py` produces this minimal Linux StarExec installation
package from a clean, audited source archive. The archive has no wrapper
directory: StarExec extracts `bin/` directly and runs
`bin/starexec_run_default` with `bin/` as the working directory.

The run configuration invokes `bin/umlaut` in automatic mode with TSTP proof
output. It preserves the benchmark path exactly, inherits `TPTP` for include
resolution, maps a positive integer `STAREXEC_CPU_LIMIT` to `--cpu-limit`, and
maps a positive integer `STAREXEC_MAX_MEM` (MiB) to `--memory-limit`.
`STAREXEC_WALLCLOCK_LIMIT` remains in the environment for StarExec to enforce;
Umlaut has a per-core CPU limit rather than a wall-clock limit.

This package follows the current public CASC-J13 and StarExec contract as of
2026-07-28. CASC-2027 rules are not yet published. Before a 2027 submission,
compare it with the organizer's then-current exemplar and validate installation
and an SZS-postprocessed job in the TPTP StarExec space.

The package deliberately contains no PicoSAT, CaDiCaL, MiniSat, Z3, GMP,
Vampire, VIRAS, or other optional backend. Umlaut uses its internal SAT fallback
when no PicoSAT-compatible shared library is configured.
