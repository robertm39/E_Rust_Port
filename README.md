# Umlaut

Umlaut is an independent automated theorem prover written in Rust. It began as
a port of the E theorem prover, but its design is no longer constrained to
E's architecture, implementation choices, bugs, performance, branding, or
executable names.

The project intends to become the leading automated theorem prover and to
compete for first place at [CASC 2027](https://tptp.org/CASC/). Soundness,
proof integrity, standards compliance, licensing, and reproducible performance
evidence take precedence over short-term benchmark gains.

## Compatibility and direction

Umlaut must retain the substantive feature coverage of E and broadly compatible
command-line, TPTP-family input, SZS status, proof-output, and resource-limit
behavior. E remains a valuable read-only compatibility, regression, provenance,
and algorithmic reference. It is not Umlaut's universal design or performance
authority, and new Umlaut features do not need an E analogue.

Name-level compatibility is intentionally unsupported. The Cargo package,
library crate, and executable suite use Umlaut names exclusively; no legacy E
binary aliases are provided.

## Executables

The primary executable is `umlaut`. Companion tools share the `umlaut-`
namespace:

| Previous target | Umlaut target |
| --- | --- |
| `eprover` | `umlaut` |
| `CSSCPA_filter` | `umlaut-csscpa-filter` |
| `e_stratpar` | `umlaut-stratpar` |
| `e_ltb_runner` | `umlaut-ltb-runner` |
| `termprops` | `umlaut-termprops` |
| `term2dag` | `umlaut-term2dag` |
| `ex_commandline` | `umlaut-commandline-example` |
| `epclextract` | `umlaut-pcl-extract` |
| `epclanalyse` | `umlaut-pcl-analyse` |
| `checkproof` | `umlaut-checkproof` |
| `epcllemma` | `umlaut-pcl-lemma` |
| `edpll` | `umlaut-dpll` |
| `eground` | `umlaut-ground` |
| `classify_problem` | `umlaut-classify-problem` |
| `tsm_classify` | `umlaut-tsm-classify` |
| `direct_examples` | `umlaut-direct-examples` |
| `e_client` | `umlaut-client` |
| `e_deduction_server` | `umlaut-deduction-server` |
| `e_server` | `umlaut-server` |
| `e_axfilter` | `umlaut-axiom-filter` |
| `enormalizer` | `umlaut-normalizer` |
| `epatternize` | `umlaut-patternize` |
| `ekb_create` | `umlaut-kb-create` |
| `ekb_delete` | `umlaut-kb-delete` |
| `ekb_insert` | `umlaut-kb-insert` |
| `ekb_ginsert` | `umlaut-kb-ginsert` |

Automation that invokes a previous target name must be updated.

## Search telemetry

Pass `--search-telemetry=run.json` to the primary executable to write one
versioned, aggregate JSON record for a saturation search. The record covers
the input and search funnels, inferences, simplification and index activity,
SAT calls, term storage, proof depth, clause-set high-water counts, and
resource usage. See
[docs/search-telemetry.md](docs/search-telemetry.md) for the schema and
schedule-worker naming contract.

## Optional incremental SAT backend

The default build uses Umlaut's dependency-free internal SAT service. The
`cadical-static` feature can compile pinned CaDiCaL 3.0.1 from an explicitly
supplied `UMLAUT_CADICAL_SOURCE` tree. Runtime mode
`UMLAUT_CADICAL_MODE=always`, `auto-128`, or `auto-256` is opt-in; unset or
`off` remains the default. Build provenance, proof-checker controls, packaging,
and disablement are documented in [DOCS.md](DOCS.md) and
[docs/dependency-packaging-matrix.md](docs/dependency-packaging-matrix.md).

## Development and validation

Start with [DOCS.md](DOCS.md) for the documentation index and
[docs/rust-code-standards.md](docs/rust-code-standards.md) for mandatory Rust
standards. Rust and C formatting, compilation, tests, execution, comparisons,
benchmarks, and profiling run only through the ephemeral-Linode workflow:

```powershell
.\linode-runner.ps1 run
```

The `eprover/` directory and the other bundled theorem-proving projects are
read-only references. Their presence does not make them part of Umlaut or
authorize copying code without a compatible license and recorded provenance.
The independent VIRAS research packet is under
[viras_docs/](viras_docs/README.md).

## Licensing

The current Cargo package declares `GPL-2.0-or-later`. Moving Umlaut to
LGPL-3.0 is a project objective, not a current license claim. Third-party
license and provenance details are recorded in
[docs/third-party-licenses.md](docs/third-party-licenses.md). The enforced
dependency and distribution boundary is in
[docs/dependency-packaging-matrix.md](docs/dependency-packaging-matrix.md).
