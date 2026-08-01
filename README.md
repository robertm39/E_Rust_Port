# Umlaut

> This README was written by GPT-5.6 Sol.

Umlaut is an independent automated theorem prover written in Rust. It began as
a port of the [E theorem prover](https://www.eprover.org/),
retains broad compatibility with E's proving interfaces and feature surface,
and now follows its own architecture and research direction.

The project's ambition is to become the leading automated theorem prover and
to compete for first place at [CASC 2027](https://tptp.org/CASC/).

> **Research preview:** Umlaut is under active development. The current Cargo
> package is version 0.1.0 and is not published to crates.io. Interfaces,
> strategies, and performance characteristics may change before a stable
> release.

## What Umlaut supports

- First-order and higher-order TPTP-family problems, including CNF, FOF, TFF,
  TCF, and THF input.
- Automatic strategy selection with `--auto`, along with explicit CPU and
  memory limits.
- Standard SZS statuses and TSTP-compatible proof output.
- Broadly E-compatible command-line behavior without legacy E executable
  names.
- A dependency-free default build with an internal SAT service, plus optional
  feature-gated research components.
- Search telemetry, proof validation, reproducible packaging, and extensive
  compatibility and performance evidence.

The primary executable is `umlaut`. The repository also contains companion
analysis, proof, classification, filtering, server, and knowledge-base tools.
See the [project direction and technical overview](docs/project-direction.md)
for the complete executable map and advanced feature boundaries.

## Quick start

Umlaut's authoritative runtime and validation platform is x86-64 Linux; the
project currently validates on Ubuntu 24.04. Install Git and a recent stable
Rust toolchain, then build the primary prover from source:

```bash
git clone https://github.com/robertm39/E_Rust_Port.git umlaut
cd umlaut
cargo build --locked --release --bin umlaut
```

Create a small TPTP problem:

```bash
cat > socrates.p <<'TPTP'
fof(socrates_is_human, axiom, human(socrates)).
fof(all_humans_are_mortal, axiom, ![X]: (human(X) => mortal(X))).
fof(socrates_is_mortal, conjecture, mortal(socrates)).
TPTP
```

Ask Umlaut to prove the conjecture and emit a TSTP proof object:

```bash
./target/release/umlaut \
  --auto \
  --tstp-out \
  --proof-object=1 \
  --cpu-limit=10 \
  socrates.p
```

The output should include:

```text
% SZS status Theorem
```

Run `./target/release/umlaut --help` for the full command-line surface. TPTP
`include(...)` directives resolve relative to the input file or through the
standard `TPTP` environment variable.

## Project status and direction

Umlaut preserves substantive E feature coverage and broadly compatible input,
output, proof, scheduling, and resource-limit behavior, but it is not intended
to remain a line-for-line port or a name-level drop-in replacement. New
algorithms and capabilities do not need an E analogue.

Current development includes saturation search, higher-order reasoning,
proof-producing preprocessing, finite-model search, optional incremental SAT,
and clean-room arithmetic quantifier elimination. Experimental features remain
off automatic schedules until their soundness and held-out utility gates pass.
The detailed policies and opt-in controls are documented in the
[technical overview](docs/project-direction.md).

## Documentation

- [Project direction and technical overview](docs/project-direction.md) —
  compatibility policy, executable names, telemetry, optional features,
  validation platforms, and licensing.
- [Documentation index](DOCS.md) — the complete technical and agent-facing
  documentation map.
- [Soundness validation](docs/soundness-validation.md) — proof, model, and
  differential validation gates.
- [Rust code standards](docs/rust-code-standards.md) — mandatory implementation
  and review standards.
- [Dependency and packaging matrix](docs/dependency-packaging-matrix.md) —
  reproducible source/runtime boundaries and optional dependencies.
- [Third-party licenses](docs/third-party-licenses.md) — provenance and license
  records for upstream references and adopted components.

Project-controlled builds, tests, compatibility comparisons, benchmarks, and
profiling use the documented
[ephemeral Ubuntu 24.04 Linode workflow](docs/linode-runner.md). Windows GNU
x64 is compile-only; Windows runtime behavior and MSVC are not supported
validation targets.

## License and provenance

Umlaut is licensed under
[GNU GPL version 2 or later](LICENSE). The current and intended distribution
license is GPL-2.0-or-later.

Umlaut contains independently maintained Rust implementations informed by E,
plus the specifically identified E-derived data recorded in
[THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md). Ignored local checkouts of E,
CaDiCaL, MiniSat, Vampire, Z3, GMP, and other research inputs are not tracked by
the public repository and are excluded from Umlaut's distributable packages.
