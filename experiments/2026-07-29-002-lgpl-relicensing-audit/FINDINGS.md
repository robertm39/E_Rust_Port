# LGPL-3.0 relicensing readiness findings

Bead: `E_Rust_Port-9jt.8.8`

Status: complete engineering audit; relicensing is **not authorized** by the
available evidence. Umlaut remains `GPL-2.0-or-later`.

This is a provenance and implementation-readiness record, not legal advice.
It deliberately does not edit `Cargo.toml`, `LICENSE`, notices, or package
metadata.

## Decision

Do not relicense the repository or either package yet.

There is a technically plausible route for E-derived expression and the exact
E schedule data: pinned E revision
`17026b1bfe61aaf223cfaae54947c8d2679c31a0` has a distribution-wide
`COPYING` notice offering GPL-2.0-or-later or LGPL-2.1-or-later, and both SPDX
and the FSF describe the "or later" choice as permitting selection of a later
LGPL version. That route is not the only required authority, however.

The repository lacks:

1. a dated rights-holder attestation covering the Umlaut-authored expression,
   employer/client/school rights, both Git author-name forms, and AI-assisted
   output;
2. qualified review confirming that E's distribution-wide dual-license grant
   governs every translated/reimplemented unit despite 45 relevant upstream
   source headers that mention only GPL and two that contain no GNU-license
   phrase;
3. a rights-holder decision between `LGPL-3.0-only` and
   `LGPL-3.0-or-later`; and
4. qualified review of the proposed notice, source-offer, modification-notice,
   and static/dynamic optional-backend obligations for actual distribution.

The first two are authorization blockers. The third prevents an exact SPDX and
license-text change. The fourth prevents claiming that a mechanically changed
package is legally ready.

## Complete distributable boundary

The inventory uses the final clean package audit from
`E_Rust_Port-9jt.4.8`:

| Boundary | Members | Archive SHA-256 | Unclassified |
| --- | ---: | --- | ---: |
| Source package | 314 | `2d82e62955b0f2eb1a9a1c2c77007e05fefc3af0c4130aee83618416664a5b3f` | 0 |
| StarExec runtime | 5 | `e79448ef845c83e1f7022a2b9b12949a16db722812862a15e104526197c687a3` | 0 |

Every source member has path, exact archived/current-Git bytes and hashes,
origin, upstream project/revision, copyright-holder statement, current
license, proposed route, and authority status in `source-members.jsonl`.
Every runtime member has the equivalent fields in `runtime-members.jsonl`.

The source classifications are:

| Origin | Members | Relicensing treatment |
| --- | ---: | --- |
| Mixed/uncertain E-port and Umlaut Rust/build source | 287 | Conservatively require both the E route and Umlaut owner authority |
| Clearly new Umlaut CaDiCaL service/shim source | 4 | Require Umlaut owner authority |
| Umlaut packaging source | 4 | Require Umlaut owner authority |
| Umlaut documentation | 5 | Require Umlaut owner authority; retain attributed facts |
| Umlaut package metadata | 1 | Require Umlaut owner authority |
| Exact E `schedule.vars` data | 1 | Select E's LGPL route only after review |
| Cargo-generated manifest/lock metadata | 2 | Regenerate from authorized inputs |
| Root GPL-2.0 license text | 1 | Replace with unmodified official LGPL-3.0 text only after authorization |
| Verbatim third-party license records | 9 | Retain verbatim; never relicense the records |

The four clearly new source files are:

- `native/cadical_ffi/umlaut_cadical.cpp`;
- `native/cadical_ffi/umlaut_cadical.h`;
- `src/clauses/cadical.rs`; and
- `src/clauses/satservice.rs`.

Classifying the other 287 implementation/build files as mixed is deliberately
conservative. It does not assert that every line is an E derivative. It avoids
making an unsupported file-by-file non-derivation claim about a project whose
history and task instructions explicitly describe a Rust port, whose commit
history repeatedly says "Port", and whose source documentation maps Rust
behavior to E units.

The exact E data member is
`src/heuristics/schedule.vars`, 2,066,481 bytes, SHA-256
`491145ab45477620ed02ed8cd789d6b5e3e6e0d38f413fdbc62163e09a9cb068`.
The tracked E option fixture
`tests/fixtures/eprover-17026b1/e_options.h`, SHA-256
`9b432caf9253a8e3b5b47901154ff419a17ba1ef7d788a17fcaf019186c87f3d`,
is not in either distributable archive. No other repository test fixture,
ordinary Rust/controller test, historical experiment, Beads state, ignored
reference tree, local artifact, PDF, or corpus member is distributed. The one
distributable package self-test,
`tools/packaging/test_verify_casc_package.py`, is inventoried as Umlaut
packaging source.

The source archive was created before its measurements were written back to
`docs/dependency-packaging-matrix.md`. Comparing archive files with canonical
Git `HEAD` blobs shows 313/314 content matches after CRLF-to-LF normalization;
that evidence-only matrix update is the sole mismatch. Package membership and
all implementation/data inputs are unchanged.

## Runtime boundary

The StarExec package contains exactly:

| Member | Origin/authority |
| --- | --- |
| `bin/umlaut` | Compiled aggregate; follows all product and E-derived source blockers |
| `bin/starexec_run_default` | Umlaut packaging source; owner attestation required |
| `starexec_description.txt` | Generated from packaging inputs |
| `THIRD_PARTY_NOTICES.md` | Umlaut-authored notice with attributed third-party facts |
| `LICENSE` | Verbatim GPL-2.0 text; replacement blocked |

The default runtime contains no PicoSAT, CaDiCaL, MiniSat, Z3, GMP, Vampire,
VIRAS, or other optional backend. The source archive includes the independent
CaDiCaL shim and MIT record but no upstream CaDiCaL source. The nine
third-party license files are provenance/notice records, not relicensing
targets. They remain under their own terms.

VIRAS source, binary, and paper PDFs are absent from both archives. The
unlicensed VIRAS revision remains reference-only and cannot become a product
or package input unless its separate license Bead is resolved.

## E route and unresolved header scope

The pinned E `COPYING` file has SHA-256
`6252a1a029c731e5161ff89b89abc0d5661e9965242c5218b331f15d583546a2`
and says that E from version 1.2 is offered under both
GPL-2.0-or-later and LGPL-2.1-or-later. The exact file is preserved in
`licenses/eprover-GPL-2.0-or-later_OR_LGPL-2.1-or-later.txt`.

The audit scanned all 481 tracked C/H files in the relevant E core
directories at the same revision:

| Header result | Files |
| --- | ---: |
| Mentions both GPL and LGPL | 434 |
| Mentions GPL but not LGPL | 45 |
| Contains neither GNU-license phrase in its first 45 lines | 2 |

The exact path, hash, raw header author, and class for every upstream file is
in `e-source-headers.jsonl`. The 45 GPL-only-phrase files include direct
counterparts of shipped Rust areas such as permanent strings, registered
memory, condensation, derivations, SAT interface, server/session, scheduling,
SInE, network/multiplexer, type checking, and variable sets. The two files
without a phrase are `CLAUSES/ccl_ext_index.h` and `PROVER/e_gitcommit.h`.

An engineering reading is that the distribution-wide `COPYING` grant likely
controls and the shorter headers refer readers back to it. This audit does not
turn that reading into a legal conclusion. Qualified review must confirm the
scope and document the rationale before the project selects LGPL-3.0 for any
translated or modified E expression.

## Umlaut contributor and AI-output authority

At audited repository commit
`96561cd3defd333da100ccb50ff12b9da1e65d10`, the reachable `HEAD` history has
2,159 commits. All Git author records use one email address,
`robertpmorton39@gmail.com`, under the names `Robert Morton` or `robertm39`.
The only other committer identity is GitHub's no-reply merge machinery. No
`Co-authored-by`, `Signed-off-by`, or `Copyright` commit trailer was found.

That is useful negative evidence, not proof of copyright ownership. Git author
metadata does not establish whether an employer, client, school, coauthor, or
upstream rights holder owns expression in a commit. It also does not establish
that the two author-name forms identify the same legal person.

OpenAI's current public terms say that, as between the user and OpenAI and to
the extent permitted by law, the user owns Output and OpenAI assigns any
OpenAI interest in it. The same terms say Output may not be unique, exclude
other users' and third-party output from that assignment, place responsibility
for rights/accuracy on the user, and require human review. The actual agreement
governing each development session and the copyrightability/provenance of the
result therefore still require owner and legal review.

`OWNER_ATTESTATION_TEMPLATE.md` captures the minimum missing factual
attestation without pretending that this agent can sign it.

## Exact follow-up

If the owner attestation and qualified review approve the route:

1. record whether the target is `LGPL-3.0-only` or
   `LGPL-3.0-or-later`;
2. retain the signed/dated authority record and counsel decision outside or
   inside the repository as advised;
3. replace root `LICENSE` with the unmodified official selected LGPL-3.0 text;
4. update the `license` field in `Cargo.toml`;
5. update current-license statements in `README.md`,
   `THIRD_PARTY_NOTICES.md`, `docs/dependency-packaging-matrix.md`, and
   `docs/third-party-licenses.md`, while preserving E and every permissive
   third-party notice;
6. correct any "informed by E" wording that understates the conservative
   mixed/translated provenance decision;
7. add a package-verifier assertion that the Cargo SPDX expression, root
   license text, notices, source package, and StarExec package agree;
8. regenerate `Cargo.toml`, `Cargo.lock`, the source package, runtime package,
   manifests, hashes, and size measurements on Ubuntu 24.04;
9. rebuild all binaries offline and repeat the full Linux/Windows, proof,
   compatibility, performance, package, signal, and StarExec gates; and
10. preserve Git history and make clear that recipients of earlier
    GPL-2.0-or-later versions keep the rights already granted to them.

If review rejects the E route or owner authority cannot be established, keep
`GPL-2.0-or-later`. The only technical replacement path is a separately
authorized clean-room rewrite of every affected mixed file and the exact E
schedule data; the current audit does not claim that such a rewrite is
practical or necessary.

## Evidence hashes

| File | Bytes | SHA-256 |
| --- | ---: | --- |
| `source-members.jsonl` | 350,224 | `3baeb221fb71037dff51177dcd9a08685a5a9c35bdccf719f8c0c78ff61d9b67` |
| `runtime-members.jsonl` | 2,947 | `5437def83540d3da826dba37f12ae12a1676f857fd0f30deaffb382bf2c7674b` |
| `e-source-headers.jsonl` | 182,472 | `34f751216da8770b4b790393136732ffa08d807261e706aa0430cc0e17b6fac0` |
| `summary.json` | 2,730 | `e8b0090698a038b0b9b75d0ce3facf5347d702c854e11ff4b5d43206cf3302d4` |

The generator and seven focused tests fail closed on unclassified package
members, unknown runtime members, unsafe archive paths, unexpected archive
hashes, the wrong E revision, and lost GPL-only header ambiguity.
