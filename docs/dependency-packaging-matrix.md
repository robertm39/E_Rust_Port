# Dependency, provenance, and CASC packaging matrix

Last reviewed: 2026-07-30 for `E_Rust_Port-9jt.5.11`.

This is the decision boundary for code, data, libraries, models, solvers, and
reference artifacts considered for Umlaut. It is an engineering and provenance
record, not legal advice. Adding a component to an ignored checkout or using it
in an experiment does not adopt it as a product dependency.

Umlaut currently declares `GPL-2.0-or-later`. LGPL-3.0 remains the intended
future license, not a current package claim. A license change requires a
separate provenance and contributor-rights audit; this matrix must not be used
to imply that the change has already happened.

## Package sets

The current distributable boundary has two independently auditable archives:

| Package | Included | Excluded |
| --- | --- | --- |
| Source `.tgz` | `Cargo.toml`, the exact optional-dependency `Cargo.lock`, `build.rs`, Rust sources, the tracked schedule input, the independent `native/cadical_ffi` interface shim, package tooling and its self-tests, the root license, notices, this matrix, and verbatim license records | Upstream CaDiCaL source; vendored Cargo crate sources; Rust and controller tests; historical experiments; Beads and Git/Dolt state; local PDFs; build output; every ignored reference tree; every local artifact |
| Linux runtime candidate `.tgz` | `bin/umlaut`, a runtime readme, `LICENSE`, and `THIRD_PARTY_NOTICES.md` | Source code; feature-required `umlaut-viras-qe`; all optional crate and backend code; companion/development binaries; all reference trees and experiment artifacts |

The runtime archive is a CASC packaging candidate, not yet a final StarExec
installation package. The current
[CASC-J13 delivery rules](https://tptp.org/CASC/J13/Design.html) require a
runtime installation `.tgz` containing only what is needed to run the system
and a separate source `.tgz` containing the source and files needed to build
the runtime package. The exact StarExec wrapper must be based on the
organizer's current exemplar and tested on StarExec. Recheck the CASC-2027
rules before submission rather than treating the J13 format as permanent.

## Adopted and candidate component matrix

"Package impact" is the baseline impact. A candidate has zero bytes until a
separate Bead adopts it, updates this matrix and the notices, and reruns the
package audit.

| ID | Component, exact input, and use | License, notice, and obligations | Link and transitive boundary | Package impact, reproduction, fallback, and disablement |
| --- | --- | --- | --- | --- |
| `UMLAUT` | Current tracked Rust implementation at the package-building commit. This is the product, including E-informed reimplementations recorded in `docs/e-port-history.md`. | `GPL-2.0-or-later`; root `LICENSE`. Preserve source, license, notices, and modification history when distributing. | The default Cargo feature closure has no external crate code. Rust standard library is linked into the Linux executable; normal target system libraries remain dynamic. | Required. Build every default-eligible binary from the extracted source archive with `cargo build --locked --release --bins --offline`. Cannot be disabled because it is the product. |
| `E-DATA` | Exact copies from E revision `17026b1bfe61aaf223cfaae54947c8d2679c31a0`: `src/heuristics/schedule.vars` (SHA-256 `491145ab45477620ed02ed8cd789d6b5e3e6e0d38f413fdbc62163e09a9cb068`) and test-only `tests/fixtures/eprover-17026b1/e_options.h` (SHA-256 `9b432caf9253a8e3b5b47901154ff419a17ba1ef7d788a17fcaf019186c87f3d`). The schedule is compiled into generated Rust tables; the header only validates option parity in repository tests. | E offers GPL-2.0-or-later or LGPL-2.1-or-later. The current package selects GPL-2.0-or-later. Preserve the E copyright/provenance notice and corresponding license; see `licenses/eprover-GPL-2.0-or-later_OR_LGPL-2.1-or-later.txt`. | No external link. `build.rs` reads only the tracked schedule. The source package excludes the ignored E checkout and does not need it to build. | Required schedule input; test header is repository-only. Regenerate from the pinned file only after reviewing and recording a new E revision and hash. Removing the schedule would require replacing the built-in strategy tables; it is not an optional backend. |
| `LINUX-SYSTEM` | Ubuntu 24.04 target runtime and its standard ELF loader/libraries. | Supplied by the target operating system under their own licenses; they are not copied into the runtime candidate. | Dynamic dependencies are recorded by `ldd` in every package audit. The audit rejects any linked optional solver/backend. | Zero packaged bytes. Reproduce on the mandatory Linode target. Disablement means choosing a different supported target and repeating the complete build/runtime audit. |
| `RUST-TOOLCHAIN` | Rust/Cargo versions recorded in `package-audit.json`; build-time input only. | Rust toolchain components retain their upstream licenses. They are not redistributed in either current archive. | The default build needs no registry crate source. Feature builds resolve only the checksummed optional graph in `Cargo.lock`. | Zero packaged bytes. The default source archive builds with `--locked --offline`; a `viras-qe` build additionally needs the audited crates available from Cargo. A toolchain change requires rerunning all quality and package gates. |
| `PICOSAT-965` | Optional runtime-loaded PicoSAT 965-compatible shared library through `src/clauses/picosat.rs`; no library is supplied. The `E_Rust_Port-9jt.4.1` bake-off also evaluated the pinned E copy but did not select it for the new incremental service. | MIT; verbatim notice in `licenses/picosat-MIT.txt`. A distributor that bundles the library must ship the notice and audit that exact binary/source revision. | Dynamic, late-bound ABI only. No Cargo dependency or transitive library is adopted. The experiment's static adapter is evidence, not a new link. | Zero baseline bytes. Unset `E_RUST_PORT_PICOSAT_LIBRARY` and omit executable-adjacent PicoSAT libraries to disable it; Umlaut uses its internal SAT solver. |
| `CADICAL` | Adopted optional incremental-SAT backend at `c60730422e758ef1cebe7aeddf2dda31c996bf04` (3.0.1). The source package ships only the independent `native/cadical_ffi` C ABI shim; `UMLAUT_CADICAL_SOURCE` supplies upstream source at feature-build time. The pinned Vampire reference separately contains an unrelated CaDiCaL revision. | MIT; verbatim notice in `licenses/cadical-MIT.txt` and runtime notice in `THIRD_PARTY_NOTICES.md`. Preserve both with any feature binary or source offer. No CaDiCaL source was copied or translated into the shim. | `cadical-static` compiles upstream C/C++ plus the shim into `libumlaut_cadical.a`. The Linux feature ELF dynamically uses the normal system `libstdc++`, `libgcc_s`, `libm`, and `libc`; the Windows-GNU PE imports `libstdc++-6.dll` plus Windows runtime DLLs. The proof checker is a separate caller-supplied process and is not linked or shipped. No Cargo dependency is added. | Default source/runtime packages still contain zero upstream solver bytes and build offline without the feature. Feature builds require the exact external source and a C++17 toolchain. The measured clean Linux feature ELF is 9,922,496 bytes (SHA-256 `691a23aa6651cd978a14a3f6c746ff64e0835c29024aafadfc885897cd774b4b`); the compile-only Windows-GNU PE is 9,596,246 bytes (SHA-256 `129e7de4b235c239f627893e7487a0c5fb97669b538b4bfb919a0c53ce126fc1`). Omit the feature for complete build-time removal; at runtime `UMLAUT_CADICAL_MODE=off` is the default and uses the internal service. `always`, `auto-128`, and `auto-256` are explicit opt-ins. |
| `MINISAT` | Evaluated but rejected incremental-SAT candidate at `37dc6c67e2af26379d88ce349eb9c4c6160e8543`; no code or binary is adopted. It had the best validation aggregate timing but no proof output, looser cancellation, and more Windows build maintenance than CaDiCaL. | MIT; `licenses/minisat-MIT.txt`. Preserve the notice if any substantial code is incorporated. | None today. The isolated build needed `-fpermissive`, MinGW zlib, and a temporary compatibility correction. | Zero bytes. Disabled by absence; the internal solver remains. Any later reconsideration requires new proof/trust and held-out evidence rather than silent copying. |
| `Z3` | Evaluated ground-theory/SMT backend and ignored reference at `2d48fd119ce5074b880944c2b1c59e537c99cd46`; no code or binary is adopted. `E_Rust_Port-9jt.5.7` found replayable synthetic difference-logic pruning through both a persistent process and an experiment-only Rust C API driver. The pinned Vampire reference separately contains static Z3 `3c47fd96cf5645d0c42b2c819d9e9a84380aa721`. | MIT; `licenses/z3-MIT.txt`. The exact source archive SHA-256 is `9b78c0cc9f330dab9f39c132aba39c92fdba2dbc0aac26dd07b3946592dd21d8`. Preserve the notice and record generated/build components if adopted. | None today. The measured self-contained process executable was 37,152,512 bytes and used only ordinary system libraries. The shared library was 37,187,200 bytes; the standalone Rust FFI prototype plus that library was 41,672,304 bytes and required unsafe ABI/lifetime/cancellation invariants plus the C++ runtime. The build tree occupied 238,421,568 bytes. Exact core/model replay, timeout, Unknown, and cancellation passed, but only on a synthetic difference-logic corpus; `E_Rust_Port-9jt.5.10` owns real branch validation. | Zero baseline bytes. Disabled by absence; Umlaut retains its no-SMT path and explicit `Unknown` behavior. Neither backend may enter the source/runtime package before the real-branch, proof, notice, lockfile, size, clean-package, Windows, and StarExec gates pass. |
| `NUM-EXACT` | Adopted only for the `viras-qe` arithmetic subsystem: `num-bigint` 0.4.8, `num-integer` 0.1.46, `num-rational` 0.4.2, and `num-traits` 0.2.19, with build dependency `autocfg` 1.5.1. All versions and crates.io checksums are exact in `Cargo.lock`. | Every crate is `MIT OR Apache-2.0`. Verbatim texts are in `licenses/rust-num-MIT.txt`, `licenses/autocfg-MIT.txt`, and `licenses/rust-num-and-autocfg-Apache-2.0.txt`; the runtime attribution is in `THIRD_PARTY_NOTICES.md`. No unsafe project wrapper or native library is introduced. | Pure Rust only. Direct graph: Umlaut to the four `num` crates; `num-bigint` to `num-integer`/`num-traits`; `num-rational` to all three; `num-integer` to `num-traits`; `num-traits` builds through `autocfg`. | Zero bytes in the default runtime and no default binary link. Enable with `--features viras-qe`; this adds the feature-required `umlaut-viras-qe` binary. Disable completely by omitting the feature, which Cargo also uses to omit that target. Source manifests and notices remain for reproducibility. |
| `EXACT-NUMERICS-CANDIDATES` | The earlier substrate study also evaluated Dashu 0.5.1, Rug 1.30.0 over `gmp-mpfr-sys` 1.7.1/full GMP 6.3.0, and GMP 6.3.0 Mini-GMP/Mini-MPQ. None is adopted. The `num` graph advanced for exact arithmetic correctness and minimal integration risk despite Dashu's benchmark lead. | Dashu is MIT and/or Apache-2.0. Rug and its FFI declare LGPL-3.0-or-later. GMP library and Mini-GMP are LGPL-3.0-or-later or GPL-2.0-or-later; retained build helpers can have GPL-3.0 terms with exceptions. Exact study checksums are in `experiments/2026-07-29-006-exact-numerics-substrate/FINDINGS.md`; GMP notices are in `licenses/gmp-*`. | None of these candidate graphs is linked. The experiment-only crate is excluded from packages. | Zero bytes; disabled by absence. Any future replacement of `NUM-EXACT` requires facade conformance, proof/hash stability, full-prover performance, notice, lockfile, clean-package, Windows, and StarExec gates. |
| `ML-RUNTIME` | Candidate custom, ONNX-style, or external-process neural inference for `E_Rust_Port-9jt.3.4`; no model, training corpus, runtime, or code is adopted. | Unknown until a concrete runtime/model/data choice is proposed. License, model provenance, training-data rights, and generated-weight redistribution must all be reviewed. | None today. A future runtime must document native libraries, transitive dependencies, model format, and deterministic CPU fallback. | Zero bytes. Disabled by absence; hand-engineered clause selection remains. Package/model size and removal must be measured before adoption. |
| `VAMPIRE-REF` | Ignored Vampire source at `3677326861181f990ce3ef461e90471ba9749225` and canonical local Linux reference executable SHA-256 `3fd88f402d2b74ddf6bf96d49a2bf3c9383710b19d1c9c2c5ecb740265a5c665`. It is benchmark/reference infrastructure only. | Vampire is BSD-3-Clause (`licenses/vampire-BSD-3-Clause.txt`), but that does not cure the missing VIRAS license in the executable. | The local executable statically contains CaDiCaL `f13d74439a5b5c963ac5b02d05ce93a8098018b8`, VIRAS `8b8928f57f8d6415662cf43289de2c0d36443240`, and Z3 `3c47fd96cf5645d0c42b2c819d9e9a84380aa721`. | Always zero package bytes. It is disabled by simply omitting `.artifacts/` and `vampire/`, as the package allowlist and verifier require. It must not be committed, published, redistributed, or treated as an Umlaut backend. |
| `VIRAS` | Unlicensed implementation revision `8b8928f57f8d6415662cf43289de2c0d36443240` appears only inside the ignored Vampire reference build. Umlaut's tracked `viras_docs/` material is an independent paper-derived clean-room design packet; local paper PDFs are ignored. | The implementation revision has no declared license. Resolution remains owned by `E_Rust_Port-mlf`. Do not inspect, copy, link, build, or redistribute its source. Paper-derived implementation work must retain paper citations and clean-room provenance. | No product link or dependency is permitted. | Always zero package bytes unless the upstream license is resolved and a later explicit review authorizes a new boundary. Current disablement is absolute absence. |
| `REFERENCE-TREES` | Ignored E, CaDiCaL, MiniSat, Vampire, Z3, and GMP trees are compatibility, provenance, algorithm, and benchmark references. | See `docs/third-party-licenses.md`; a top-level license never automatically covers every nested component or authorizes copying. | No build may discover them implicitly. The package audit fails if any reference-root path enters an archive. | Always zero package bytes. A clean source-package build is the disablement and falsification test. |
| `TPTP-CORPUS` | External TPTP/CASC problems and the `TPTP` runtime path; only small, explicitly provenanced parser fixtures are tracked for repository tests. | Problem and solution provenance must be reviewed per dataset. The competition corpus is not an Umlaut dependency or package component. | Runtime file input only. Includes resolve relative to the problem or `TPTP`. | Zero source/runtime package bytes. Disablement means running a self-contained problem; package correctness must not depend on a local `problems/` tree. |

## Source-derived implementation paths

| Path | Allowed evidence and required record | Package rule |
| --- | --- | --- |
| E compatibility port | The pinned E source, the per-unit C documentation, `docs/e-port-history.md`, and focused experiments. Record the E unit/revision and whether behavior or implementation was adopted or intentionally changed. | Rust product code and the two declared data inputs may ship under the current GPL-2.0-or-later package. The ignored E tree never ships. |
| VIRAS arithmetic | The papers and tracked clean-room packet in `viras_docs/`; never the unlicensed implementation. Record paper theorem/section, errata, and independent tests. | Only new Umlaut code and attributed documentation may ship. VIRAS source/binaries and ignored PDFs do not. |
| Cross-prover algorithm study | CaDiCaL, MiniSat, Vampire, Z3, GMP, papers, and specifications may inform experiments when their licenses permit inspection. Record exact revision, files/ideas used, and whether code was copied, translated, or independently implemented. | No reference tree or binary ships. Any copied or linked component needs an adopted row, notice, transitive audit, package-size measurement, and disablement path first. |
| Independent Rust design | Rust standard-library APIs, specifications, measured profiles, and new Umlaut experiments. Record unsafe/FFI invariants and performance evidence under the Rust standards. | Ships as `UMLAUT`; a new Cargo dependency is forbidden until this matrix and `Cargo.lock` are deliberately updated. |

## Enforced invariants and reproduction

`tools/packaging/verify_casc_package.py` is the executable gate. On the target
Ubuntu 24.04 Linode it:

1. proves the default feature graph is dependency-free and the optional
   `viras-qe` lock graph is exactly the five audited crates;
2. creates the Cargo source archive from the explicit allowlist;
3. rejects reference roots, artifacts, Beads/Git/Dolt state, experiments, PDFs,
   and archive links;
4. extracts the archive into a clean temporary directory with no ignored
   checkout available;
5. builds every default-eligible binary with
   `--locked --release --bins --offline` and confirms the feature-required
   arithmetic binary is omitted;
6. runs the extracted `umlaut --version`;
7. rejects dynamically linked optional backends from the default runtime
   candidate;
8. creates a deterministic four-file runtime candidate; and
9. records toolchain, archive/binary sizes, SHA-256 values, members, and dynamic
   libraries in `package-audit.json`.

Run it only through the Linode controller:

```text
python3 tools/packaging/verify_casc_package.py \
  --output-dir .artifacts/package-audit/CURRENT
```

The reviewed experiment and exact measurements are retained in
`experiments/2026-07-27-001-reversible-casc-packaging/`.

The 2026-07-29 Ubuntu 24.04 audit produced:

| Measured artifact | Bytes | SHA-256 |
| --- | ---: | --- |
| Source `.tgz` (314 members) | 1,955,806 | `2d82e62955b0f2eb1a9a1c2c77007e05fefc3af0c4130aee83618416664a5b3f` |
| Minimal StarExec candidate `.tgz` (five members) | 2,794,141 | `e79448ef845c83e1f7022a2b9b12949a16db722812862a15e104526197c687a3` |
| Uncompressed primary Linux ELF | 8,255,312 | `84897ed61fd114a08582780a67665ad321b923cfa270bce334a71e17be8dba17` |

The extracted source built all 26 default-eligible release binaries offline.
The feature-required arithmetic binary was omitted. The source
archive contains the independent `native/cadical_ffi` shim and excludes
upstream CaDiCaL source. The primary binary dynamically needs only the Linux
loader, `libgcc_s`, `libm`, and `libc`; no optional backend was linked. StarExec
include resolution, wrapper argument forwarding, `SIGALRM`, and `SIGXCPU`
emulation all passed. These measurements are a baseline, not a permanent size
allowance. Re-run the audit after any source, toolchain, profile, or
package-content change.

The optional feature is reproduced separately by the Linode bootstrap. It
downloads the exact upstream archive, requires SHA-256
`ad639a302b7c4cb4a24f37b7cd0cf7533674e6069c20a561505bccef1c2b4444`,
checks `VERSION` is `3.0.1`, exports `UMLAUT_CADICAL_SOURCE`, and uses the
POSIX-thread MinGW C/C++ compilers for `x86_64-pc-windows-gnu`. This feature
path does not weaken the clean default package gate.

## Change gate and unresolved questions

Every proposed dependency remains rejected-by-default. Before adopting one:

1. pin its exact source and binary revisions;
2. record files or ideas used and provenance;
3. verify its license and every required notice;
4. document modification, source-offer, static/dynamic-link, patent, model, and
   data obligations that apply;
5. enumerate build tools and transitive dependencies;
6. measure source archive, runtime archive, installed, and peak-build size;
7. define explicit failure, fallback, and disablement behavior;
8. rerun correctness, proof, performance, clean-package, and StarExec checks;
9. update `Cargo.lock`, notices, this matrix, and Beads in the same change.

Open questions are deliberately visible:

- `E_Rust_Port-mlf` owns the missing VIRAS license resolution. Until it closes
  with explicit upstream evidence, VIRAS implementation artifacts are
  non-distributable.
- Umlaut's intended LGPL-3.0 move still needs a complete provenance and
  contributor-rights decision. The current package remains GPL-2.0-or-later.
- Bundling PicoSAT would change the current zero-byte optional boundary and
  requires an exact binary/source revision, notice, ABI, and package audit.
- The final CASC-2027 StarExec wrapper and installation package must use the
  organizer's then-current exemplar and pass an actual StarExec job; the
  present runtime archive validates contents and linkage, not that external
  platform integration.
