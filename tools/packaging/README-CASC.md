# Umlaut CASC runtime candidate

This archive is a minimal Linux runtime candidate produced by
`verify_casc_package.py`. It is not yet a final StarExec installation package:
the competition-specific StarExec wrapper must be based on the organizer's
current exemplar and validated on StarExec before submission.

The executable is `bin/umlaut`. It accepts TPTP-family problem files and the
documented Umlaut/E-compatible options. A representative direct invocation is:

```text
bin/umlaut PROBLEM.p --auto --silent --cpu-limit=300
```

Set `TPTP` to the TPTP library root when a problem uses include files that are
not relative to the problem itself. For a CASC-like 128 GiB prover limit, add:

```text
--memory-limit=131072
```

The runtime candidate deliberately contains no PicoSAT, CaDiCaL, MiniSat, Z3,
GMP, Vampire, VIRAS, or other optional backend. Umlaut uses its internal SAT
fallback when no PicoSAT-compatible shared library is configured.
