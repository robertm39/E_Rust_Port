# Project Direction and Technical Overview

This document contains the detailed compatibility, executable, optional-feature,
validation, and licensing information that previously lived in the root
README. For a newcomer-oriented introduction and quick start, see the
[public project README](../README.md).

Umlaut is an independent automated theorem prover written in Rust. It began as
a port of the E theorem prover, but its design is no longer constrained to E's
architecture, implementation choices, bugs, performance, branding, or
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

The additional `umlaut-viras-qe` executable is feature-required and has no
legacy E analogue. It provides standalone, bounded typed arithmetic
quantifier elimination only when built with `--features viras-qe`; see the
[VIRAS quantifier-elimination documentation](viras-qe.md).

## Search telemetry

Pass `--search-telemetry=run.json` to the primary executable to atomically
maintain one versioned, aggregate JSON record for a saturation search. An
initial checkpoint survives hard stops before ordinary finalization; a final
record covers the input and search funnels, inferences, simplification and
index activity, SAT calls, term storage, proof depth, clause-set high-water
counts, and resource usage. See the
[search telemetry documentation](search-telemetry.md) for the schema and
schedule-worker naming contract.

## Optional incremental SAT backend

The default build uses Umlaut's dependency-free internal SAT service. The
`cadical-static` feature can compile pinned CaDiCaL 3.0.1 from an explicitly
supplied `UMLAUT_CADICAL_SOURCE` tree. Runtime mode
`UMLAUT_CADICAL_MODE=always`, `auto-128`, or `auto-256` is opt-in; unset or
`off` remains the default. Build provenance, proof-checker controls, packaging,
and disablement are documented in the [documentation index](../DOCS.md) and
[dependency and packaging matrix](dependency-packaging-matrix.md).

## Optional arithmetic quantifier elimination

The `viras-qe` feature adds a clean-room, paper-derived exact LIRA
quantifier-elimination kernel and the standalone `umlaut-viras-qe` tool. The
feature also enables an explicit mixed-problem preprocessing path in the
primary prover, but no automatic schedule enables it by default. Its supported
typed fragment, fail-closed limits, derivation replay, output schema,
dependencies, and complete disablement boundary are documented in the
[VIRAS quantifier-elimination documentation](viras-qe.md).

## Development and validation

Start with the [documentation index](../DOCS.md) and the mandatory
[Rust code standards](rust-code-standards.md). Project-controlled Rust and C
formatting, compilation, tests, execution, comparisons, benchmarks, and
profiling run through the ephemeral Ubuntu 24.04 Linode workflow:

```powershell
.\linode-runner.ps1 run
```

Linux x86-64 is the runtime, behavioral-compatibility, and performance
authority. Windows GNU x64 is compile-only; Windows runtime behavior and MSVC
are not supported validation targets. See the
[Linode runner documentation](linode-runner.md) for the complete workflow.

When present locally, the ignored `eprover/`, `cadical/`, `minisat/`,
`vampire/`, `z3/`, and `gmp-6.3.0/` trees are read-only research references.
They are not tracked by the public repository, are not part of Umlaut, and are
excluded from source and runtime packages. Their presence never authorizes
copying code without a compatible license and recorded provenance. The
independent VIRAS research packet is under
[`viras_docs/`](../viras_docs/README.md).

## Licensing

Umlaut is currently licensed under
[GPL-2.0-or-later](../LICENSE), as declared by the Cargo package. This is also
the intended distribution license; the previously considered move to
LGPL-3.0 is no longer planned.

Third-party license and provenance details are recorded in the
[third-party license documentation](third-party-licenses.md) and
[third-party notices](../THIRD_PARTY_NOTICES.md). The enforced dependency and
distribution boundary is defined by the
[dependency and packaging matrix](dependency-packaging-matrix.md).
