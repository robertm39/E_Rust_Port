# StarExec package contract and local validation

## Question

Can the reversible four-file CASC runtime candidate be turned into a minimal,
audited StarExec installation package that follows the latest public CASC
contract, solves an include-using problem with valid SZS output, handles the
competition termination signals, and leaves no undeclared runtime dependency
or file?

This experiment addresses Bead `E_Rust_Port-9jt.8.6`.

## Current rule boundary

The rules were rechecked on 2026-07-28. CASC-2027 rules were not published.
The latest published competition contract was
[CASC-J13](https://tptp.org/CASC/J13/Design.html), whose competition date was
2026-07-27. It therefore provides the current public baseline, not authority to
predict the 2027 contract.

The public baseline requires:

- separate runtime and source `.tgz` deliveries;
- an Ubuntu 24.04.3 competition host, one eight-core CPU allocation, 128 GiB
  memory limit, and wall-clock problem limits of at least 120 seconds;
- problem and solution communication through standard input/output conventions,
  with an SZS status before a delimited solution on standard output;
- include resolution relative to the problem or through `TPTP`;
- handling of `SIGXCPU` and `SIGALRM`; and
- no files left after normal termination, with signal-termination residue
  restricted to `/tmp`.

The [CASC-J13 schedule](https://tptp.org/CASC/J13/Schedule.html) confirms that
its 2026 delivery and competition dates have passed. The public design says the
current organizer exemplar must be requested from the organizer, and that the
package must be installed and run as a real job in the TPTP StarExec space.
Those two organizer-controlled checks remain open.

The official
[StarExec solver-upload help](https://starexec.acorn.miami.edu/starexec/secure/add/solver.help)
and
[StarExec User Guide](https://starexec.acorn.miami.edu/starexec/public/StarExecUserGuide.pdf)
define the public wrapper contract used here. A run configuration must be
named `bin/starexec_run_*`; `bin/` is its working directory; `$1` is the
absolute benchmark path; and the installation archive must expose `bin/`
directly rather than nesting it below a wrapper directory. The wrapper inherits
`TPTP`, `STAREXEC_WALLCLOCK_LIMIT`, `STAREXEC_CPU_LIMIT`,
`STAREXEC_MAX_MEM`, and `STAREXEC_MAX_WRITE`.

## Implementation

`tools/packaging/starexec_run_default` is a POSIX `sh` wrapper packaged as mode
0755. It preserves the absolute problem path, inherits `TPTP`, maps positive
integer StarExec CPU and memory limits to Umlaut's E-compatible options, and
uses:

```text
--auto --tstp-out --proof-object=1 --output-level=0
```

StarExec continues to enforce its wall-clock limit externally because Umlaut's
corresponding CLI option is a per-core CPU limit.

`tools/packaging/verify_casc_package.py` now:

1. creates and audits the Cargo source archive;
2. extracts it and builds all 26 declared binaries offline in release mode;
3. emits a deterministic, rootless StarExec archive;
4. rejects any member outside a five-file allowlist, unsafe archive path,
   incorrect mode, link, or optional-backend name;
5. checks dynamic linkage for undeclared theorem-prover, SAT, SMT, arithmetic,
   and VIRAS/Vampire components;
6. substitutes a recording executable to test exact wrapper arguments,
   whitespace-safe problem paths, and inherited environment;
7. restores the real executable and solves an include-using FOF theorem through
   the extracted wrapper, requiring `Theorem` and delimited
   `CNFRefutation` output;
8. injects both `SIGALRM` and `SIGXCPU` into the real release executable and
   requires exit 8 plus SZS `ResourceOut`; and
9. hashes the installation before and after each probe and rejects normal-exit
   temporary/output residue.

The signal probe exposed an actual defect. `SIGXCPU` could interrupt a glibc
allocation while Umlaut's handler allocated a `Diagnostic`, causing an
allocator re-entry assertion. The native handler is now allocation-free, uses
static output bytes, and terminates with async-signal-safe `_exit`. Umlaut also
installs the resource-out handler for `SIGALRM`. A Linux integration test
delivers `SIGALRM` while a large regular TPTP file is being parsed so this
failure mode stays covered.

## Final package evidence

The final package audit ran on disposable worker
`e-rust-codex-260728-082757-7d41` (Linode `101593837`) on Ubuntu 24.04,
x86-64, glibc 2.39, Rust 1.97.1, and Cargo 1.97.1. The worker and firewall were
deleted after the run.

| Artifact | Bytes | SHA-256 |
| --- | ---: | --- |
| Audited source `.tgz` | 1,921,429 | `81abc2afedc6c7b4195ed588fe8213ff9580f65bca63810f85f66b9f65d04f1c` |
| Rootless StarExec runtime `.tgz` | 2,762,199 | `e3391e5530dfe9facb6bd20af24c4e17210d579af2082c7a0cb866b8af8ecb68` |

The runtime archive contains exactly:

```text
LICENSE
THIRD_PARTY_NOTICES.md
bin/starexec_run_default
bin/umlaut
starexec_description.txt
```

The include job returned `Theorem`, a delimited `CNFRefutation`, no stderr,
and stdout SHA-256
`9029311143112bf6c9f5d809b1a4e5cab5a19b625ec3b146b6d4df5a243de23e`.
Both `SIGALRM` and `SIGXCPU` returned exit 8 with SZS `ResourceOut` within the
five-second audit bound. Neither signal probe nor the normal job left a file in
its dedicated temporary or StarExec output directory.

The archive includes the project license and third-party notices. PicoSAT,
CaDiCaL, MiniSat, Z3, GMP, Vampire, VIRAS, and other optional backends are
neither bundled nor dynamically linked; the internal SAT fallback remains
active.

## Comprehensive validation

Normal-profile runner `260728-075710-d611` passed:

- Rustfmt, 4,432 library/unit tests, 15 Linux integration tests, and nine
  solution-validation controller tests;
- pedantic Clippy with warnings denied;
- all release binaries;
- compile-only tests and release binaries for `x86_64-pc-windows-gnu`;
- 50 main-prover cases with zero unexpected mismatches and seven expected
  differences;
- 216 tool cases with zero unexpected mismatches and 16 expected differences;
- 10 benchmark cases with zero behavior mismatches and aggregate Rust/C
  wall-time ratio `1.0687869754514174`; and
- native smoke and Callgrind runs.

The downloaded validation summary SHA-256 is
`779c29410eae094ed7853a634d8a07fe477fe7600b6502e2ba5e4d04a30df28e`.
The 442 downloaded artifacts occupy 2,788,553 bytes. The runner and firewall
were deleted.

## Reproduction

Run the package audit only on the required Ubuntu runner:

```text
python3 tools/packaging/verify_casc_package.py --output-dir "$OUTPUT_DIR"
```

The comprehensive repository gate is:

```powershell
.\linode-runner.ps1 run
```

The machine-readable compact evidence is
`package-audit-summary.json` in this directory.

## Conclusion and remaining gates

The public-contract package candidate is complete and repeatably passes local
StarExec emulation. It fixes the previous rejected wrapper-directory layout,
has exact deterministic contents, carries licensing notices, solves an
include-using problem with valid SZS proof output, handles both competition
signals without allocator re-entry, and leaves no files on normal exit.

It is not yet a final CASC-2027 submission. CASC-2027 rules do not exist
publicly, the organizer's then-current exemplar has not been obtained, and no
authorized real StarExec installation/job has been run. Closing the Bead
requires those external checks and a final comparison against the future 2027
contract; this experiment intentionally records them as false rather than
claiming or inferring success.
