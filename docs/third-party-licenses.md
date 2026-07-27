# Third-Party Licenses

The `licenses/` directory contains verbatim copies of the license notices
distributed with the third-party source trees bundled as references for
Umlaut. These trees provide provenance, compatibility evidence, algorithmic
ideas, and comparative implementations; their presence does not make them
part of Umlaut or authorize automatic incorporation of their code. Any
incorporation requires a compatible license and recorded provenance. The
copies retain each project's copyright notice even when multiple projects use
the same license family.

| Project | Bundled revision | Declared license | Verbatim copy |
| --- | --- | --- | --- |
| [CaDiCaL](https://github.com/arminbiere/cadical) | `c60730422e758ef1cebe7aeddf2dda31c996bf04` | MIT | [`cadical-MIT.txt`](../licenses/cadical-MIT.txt) |
| [E](https://github.com/eprover/eprover) | `17026b1bfe61aaf223cfaae54947c8d2679c31a0` | Dual-licensed under GPL-2.0-or-later or LGPL-2.1-or-later | [`eprover-GPL-2.0-or-later_OR_LGPL-2.1-or-later.txt`](../licenses/eprover-GPL-2.0-or-later_OR_LGPL-2.1-or-later.txt) |
| [GMP](https://gmplib.org/) | 6.3.0 | The retained library, including Mini-GMP, is dual-licensed under LGPL-3.0-or-later or GPL-2.0-or-later. The upstream demo and test trees have been removed. Some retained distribution build helpers use GPL-3.0-or-later with special exceptions. | [`gmp-LGPL-3.0-or-later.txt`](../licenses/gmp-LGPL-3.0-or-later.txt), [`gmp-GPL-2.0-or-later.txt`](../licenses/gmp-GPL-2.0-or-later.txt), and [`gmp-GPL-3.0-or-later.txt`](../licenses/gmp-GPL-3.0-or-later.txt) |
| [MiniSat](https://github.com/niklasso/minisat) | `37dc6c67e2af26379d88ce349eb9c4c6160e8543` | MIT | [`minisat-MIT.txt`](../licenses/minisat-MIT.txt) |
| [Vampire](https://github.com/vprover/vampire) | `3677326861181f990ce3ef461e90471ba9749225` | Modified BSD 3-Clause | [`vampire-BSD-3-Clause.txt`](../licenses/vampire-BSD-3-Clause.txt) |
| [VIRAS](https://github.com/joe-hauns/viras) | `8b8928f57f8d6415662cf43289de2c0d36443240` | No license declaration is present in this revision | No license text is available to copy |
| [Z3](https://github.com/Z3Prover/z3) | `2d48fd119ce5074b880944c2b1c59e537c99cd46` | MIT | [`z3-MIT.txt`](../licenses/z3-MIT.txt) |

## Provenance

The CaDiCaL, E, MiniSat, and Z3 copies come from `LICENSE`, `COPYING`,
`LICENSE`, and `LICENSE.txt`, respectively, in the bundled checkouts. The
Vampire copy comes from `LICENCE` at the revision above. The GMP copies come
from the 6.3.0 distribution's `COPYING.LESSERv3`, `COPYINGv2`, and
`COPYINGv3`; the [GMP copying conditions](https://gmplib.org/manual/Copying)
describe which parts use each license.

The local `gmp-6.3.0/` reference tree intentionally omits `demos/`, `tests/`,
and `mini-gmp/tests/` so those GPL-only programs are not available as
implementation guidance. Do not restore or use those directories as source
references. The GPL-3.0-or-later copy remains necessary for retained
distribution build helpers that carry GPL-3 terms with special exceptions;
those helpers are not GMP library implementation sources.

VIRAS requires special attention: its complete Git tree at the bundled
revision has no license file, and its source headers and README contain no
licensing statement. Consequently this repository cannot include a license
copy for VIRAS or use that implementation as source guidance. Treat its
license status as unresolved and obtain an explicit license from the upstream
maintainer before redistributing it. Umlaut's VIRAS design work uses only the
paper-derived clean-room packet under `viras_docs/`.

This inventory records the top-level license of each named project. Individual
source distributions can contain separately licensed third-party or
documentation components whose notices remain in those distributions.
