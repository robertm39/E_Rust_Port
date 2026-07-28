# Third-party notices

Umlaut began as a Rust port of the E theorem prover. The current source tree
contains independently maintained Rust implementations informed by E, plus two
exact E-derived data inputs:

- `src/heuristics/schedule.vars`
- `tests/fixtures/eprover-17026b1/e_options.h`

Both inputs were copied from E revision
`17026b1bfe61aaf223cfaae54947c8d2679c31a0`. E is copyright Stephan
Schulz and the E contributors and is offered under GPL-2.0-or-later or
LGPL-2.1-or-later. Umlaut's current package selects GPL-2.0-or-later,
consistent with `Cargo.toml` and the root `LICENSE`. The combined upstream E
notice is retained in
`licenses/eprover-GPL-2.0-or-later_OR_LGPL-2.1-or-later.txt`.

Umlaut can optionally load a user-supplied PicoSAT 965-compatible shared
library at runtime. No PicoSAT binary or source is included in the default
source or runtime package, and the internal solver is used when the library is
absent. Distributors that add PicoSAT must include its MIT notice, retained in
`licenses/picosat-MIT.txt`, and must re-run the package audit described in
`docs/dependency-packaging-matrix.md`.

The ignored E, Vampire, CaDiCaL, MiniSat, Z3, GMP, and local artifact trees are
reference or experiment inputs. They are not part of Umlaut's distributable
package. In particular, the pinned local Vampire executable incorporates an
unlicensed VIRAS revision and must not be committed, published, redistributed,
or placed in a Umlaut or CASC package.
