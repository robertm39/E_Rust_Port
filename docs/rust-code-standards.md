# Rust Code Standards

Umlaut is an independent automated theorem prover written in Rust. It began as
a port of E and must retain E's substantive feature coverage, but its design is
not constrained to E's architecture, implementation choices, bugs,
performance, branding, or executable names.

This file is the canonical Rust standards document for Umlaut. The
`docs/c_source_docs/` tree documents the original E source and must not become
a second Umlaut standards document.

## Product Invariants And Engineering Direction

Soundness, proof integrity, standards compliance, licensing, and provenance are
non-negotiable. Within those invariants, prefer designs that improve
capability, maintainability, reproducibility, or measured performance and
advance the goal of winning CASC.

E remains a read-only compatibility, regression, provenance, and algorithmic
reference. New Umlaut features do not need an E analogue. Intentional
implementation or behavioral divergences are permitted when they are sound,
documented, tested, license-compatible, and supported by evidence. Do not
reproduce a documented upstream defect merely for byte-for-byte parity.

Umlaut must retain E's substantive features and broadly compatible CLI,
TPTP-family input, SZS status, proof-output, and resource-limit behavior.
Package, library-crate, executable, help-banner, and diagnostic program names
are deliberately Umlaut-specific and have no legacy aliases.

## Required Checks

Do not format, compile, test, execute, benchmark, or profile Rust or C on the
local computer or under WSL. Every Rust code change must use the comprehensive
ephemeral-Linode lifecycle:

```powershell
.\linode-runner.ps1 run
```

The local PowerShell command only provisions, uploads, orchestrates, downloads
artifacts, and tears down. On Ubuntu 24.04 the worker runs Rustfmt, all-target
and all-feature tests, Clippy with warnings and pedantic findings denied,
release builds for every binary, native C/Umlaut compatibility matrices,
timing benchmarks, and Callgrind. It also compiles all binaries and test
targets for `x86_64-pc-windows-gnu` without executing them. Treat Clippy
pedantic findings as design feedback and use small, explicit fixes whose
correctness and performance implications remain auditable.

This rule also prohibits quick local smoke tests and running the toolchain in
WSL, a local container, or another local virtual machine. Normal validation
must use `.\linode-runner.ps1 run`. If focused work needs an individual remote
command, use the runbook's guarded `up`/`sync`/`exec`/`down` lifecycle and put
`down` in a PowerShell `finally` block. Never use a direct local Cargo, Rust,
C, prover, benchmark, Valgrind, or Callgrind command as a preliminary check.

Docs-only changes should run the Markdown link checker from `DOCS.md`.

## Platform Support

Native Linux is the only supported execution platform and the authority for
Umlaut behavior and performance. E comparisons on that platform are regression
evidence for supported compatibility surfaces, not a ceiling on Umlaut's
behavior or performance. Windows GNU x64 is a compile-only portability target.
Windows executables must never be run as part of project validation, including
on the Linode through Wine or another emulator, and the project makes no
Windows runtime, behavioral, performance, or MSVC guarantee.

Keep the `x86_64-pc-windows-gnu` build working, including Windows-gated code.
Cross-platform abstractions must protect Umlaut's documented Linux contracts;
do not add untested Windows-specific behavior.

## Unsafe Rust

Unsafe Rust is permitted when there is a concrete reason that safe Rust cannot
adequately satisfy. Valid reasons include interoperability, correctness
requirements that cannot be expressed safely, supported compatibility
contracts, and measured performance needs. Convenience alone is not
sufficient. Prefer a safe design whenever it can meet the same requirements.

This permission covers unsafe blocks and functions, definitions and
implementations of unsafe traits, FFI, and calls to unsafe APIs exposed by
dependencies. Implementing an unsafe trait is permitted when the implementation
satisfies and documents every invariant required by that trait.

Keep unsafe implementation details narrowly scoped and contained behind safe
APIs. Every externally usable boundary must be safe and must validate, encode,
or otherwise uphold the preconditions of the unsafe implementation. Internal
unsafe functions may exist only within that contained implementation, with all
callers required to uphold their documented contracts.

Every use of unsafe Rust must document both why unsafe code is justified and
why it cannot result in Undefined Behavior:

- Put a `SAFETY:` comment immediately next to each unsafe operation or block.
  State the applicable invariants and explain how the code establishes them.
- Put a `SAFETY:` comment next to each unsafe trait implementation that
  addresses every safety requirement imposed by the trait.
- Give every unsafe function and unsafe trait a `# Safety` documentation
  section that states the caller or implementer obligations.
- Address pointer provenance, validity, alignment, initialization, aliasing,
  lifetimes, thread safety, ABI contracts, and other relevant sources of
  Undefined Behavior rather than relying on a generic assurance.

Keep `#![deny(unsafe_code)]` at the crate level. An item or module that
genuinely needs unsafe Rust may use the smallest practical local
`allow(unsafe_code)`, accompanied by a comment identifying the reason, safe API
boundary, and documented invariants that make the implementation sound.

## Panics And Fatal Errors

Production code must not use `unwrap`, `expect`, or panic-driven control flow
for recoverable states. Use explicit error handling, checked access, or
internal helper APIs that make the failure mode clear.

Panics are acceptable only for narrow internal invariants that cannot be
triggered by valid user input, valid problem files, CLI options, environment
variables, or resource limits. Document the invariant at the point of use.

For a supported compatibility surface, preserve the relevant diagnostic
stream, exit status, and partial-output behavior unless an intentional
divergence is documented. Diagnostics must identify the Umlaut executable,
not E. Tests may use `unwrap` or `expect` when it makes test failures clearer.

## Prover Implementation Style

- Prefer deterministic behavior and explicit state transitions in parsing,
  inference, simplification, indexing, ordering, scheduling, and proof output.
- Keep soundness-critical invariants visible and testable.
- Use E source documentation to understand contracts and optimization ideas,
  not to forbid clearer or faster Rust designs.
- Record evidence before replacing a performance-sensitive representation or
  algorithm, and keep enough provenance to audit the decision.
- Add tests for standards contracts, proof behavior, compatibility-sensitive
  surfaces, intentional divergences, and performance-relevant paths.

## Public Compatibility Contracts

- Preserve supported TPTP-family parsing, SZS status semantics, proof validity,
  include handling, stdin behavior, output ordering, line-ending handling, and
  resource-limit behavior.
- Preserve supported CLI options and the three legacy user configuration
  variables `E_RUST_PORT_COMPAT_ROOT`, `E_RUST_PORT_COMPAT_ARTIFACT_ROOT`, and
  `E_RUST_PORT_PICOSAT_LIBRARY`. Their names remain unchanged for operational
  compatibility.
- Do not provide old E package or executable aliases. Compatibility begins
  after invoking the corresponding Umlaut executable.
- Keep deterministic ordering explicit. Do not rely on hash-map iteration,
  filesystem traversal, pointer addresses, or thread scheduling when output or
  proof search can observe the result.
- Choose integer widths and conversions deliberately. Preserve required
  overflow, truncation, signedness, sentinel, and boundary semantics, while
  fixing upstream defects when the divergence is documented and tested.
- Preserve proof-search state transitions when they are required for soundness,
  proof reconstruction, or a supported interface. Other mutation order is an
  implementation choice subject to correctness and performance evidence.

## Data Structures And Ownership

Audit whether identity, sharing, lifetime, allocation reuse, caches, term
banks, clause indexes, or global state affect correctness, proof output,
supported behavior, or measured performance. Those facts—not source-level
similarity—determine the Rust design.

Prefer safe arenas, interners, stable handles, index-based storage, explicit
queues, and scoped owners when appropriate. Structural equality, cloning,
sharing, and alternative indexing strategies are acceptable when their
semantics are correct and their performance is measured. Keep
performance-critical structures explicit enough to profile and reason about.

## Dependencies And Source Provenance

Prefer the Rust standard library and small, focused crates. A dependency may
serve correctness, interoperability, supported compatibility, theorem-proving
capability, research, maintainability, or measured performance.

Before adding a crate, review and document its license, maintenance status,
transitive dependency impact, feature flags, deployment effects, and
reproducibility. Use minimal features where practical.

Bundled theorem-proving source trees are references, not automatic
implementation sources. Incorporate code only when its license is compatible
with Umlaut's current and intended licensing and its provenance is recorded.
Do not inspect or use unlicensed VIRAS implementation source; use the
paper-derived clean-room packet under `viras_docs/`.

A dependency must not bypass this project's unsafe-Rust policy through project
wrapper code. Calls to unsafe dependency APIs require an allowed concrete
reason, a safe project API boundary, and documentation of every relevant
safety requirement.

## Documentation Expectations

For E-derived or compatibility-sensitive work, identify the E source units and
relevant `docs/c_source_docs/` pages used as references. For independent work,
record the papers, standards, implementations, experiments, licenses, and
design evidence that informed it.

Document intentional compatibility deviations and their justification. For
performance-sensitive work, record representation and algorithm assumptions,
expected hot paths, benchmark coverage, and the evidence used to accept the
change.
