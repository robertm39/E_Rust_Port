# Rust Code Standards

This project is a Rust port of the E theorem prover. Rust code must preserve the behavior, feature coverage, and performance expectations of the original C implementation while using clear, idiomatic Rust.

This file is the canonical Rust standards document for the port. Do not add a second standards document under `docs/c_source_docs/`; that tree documents the original C source.

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
release builds for every binary, native C/Rust compatibility matrices, timing
benchmarks, and Callgrind. It also compiles all binaries and test targets for
`x86_64-pc-windows-gnu` without executing them. Treat Clippy pedantic findings
as design feedback and prefer small, explicit fixes that keep the port close to
the original implementation model.

This rule also prohibits quick local smoke tests and running the toolchain in
WSL, a local container, or another local virtual machine. Normal validation
must use `.\linode-runner.ps1 run`. If focused work needs an individual remote
command, use the runbook's guarded `up`/`sync`/`exec`/`down` lifecycle and put
`down` in a PowerShell `finally` block. Never use a direct local Cargo, Rust,
C, prover, benchmark, Valgrind, or Callgrind command as a preliminary check.

Docs-only changes should run the Markdown link checker from `DOCS.md`.

## Platform Support

Native Linux is the only supported execution platform and the authority for
behavioral and performance comparisons with upstream E. Windows GNU x64 is a
compile-only portability target. Windows executables must never be run as part
of project validation, including on the Linode through Wine or another
emulator, and the project makes no Windows runtime, behavioral, performance,
or MSVC guarantee.

Keep the `x86_64-pc-windows-gnu` build working, including Windows-gated code,
but do not add Windows-specific behavior solely to mimic results that are not
tested. Cross-platform abstractions should preserve the upstream Linux
contract first.

## Unsafe Rust

Unsafe Rust is permitted when there is a concrete reason that safe Rust cannot adequately satisfy. Valid reasons include interoperability, compatibility with the original C implementation, correctness requirements that cannot be expressed safely, and measured performance needs. Convenience alone is not sufficient. Prefer a safe design whenever it can meet the same requirements.

This permission covers unsafe blocks and functions, definitions and implementations of unsafe traits, FFI, and calls to unsafe APIs exposed by dependencies. Implementing an unsafe trait is permitted when the implementation satisfies and documents every invariant required by that trait.

Keep unsafe implementation details narrowly scoped and contained behind safe APIs. Every externally usable boundary must be safe and must validate, encode, or otherwise uphold the preconditions of the unsafe implementation. Internal unsafe functions may exist only within that contained implementation, with all callers required to uphold their documented contracts.

Every use of unsafe Rust must document both why unsafe code is justified and why it cannot result in Undefined Behavior:

- Put a `SAFETY:` comment immediately next to each unsafe operation or block. State the applicable invariants and explain how the code establishes them.
- Put a `SAFETY:` comment next to each unsafe trait implementation that addresses every safety requirement imposed by the trait.
- Give every unsafe function and unsafe trait a `# Safety` documentation section that states the caller or implementer obligations.
- Address pointer provenance, validity, alignment, initialization, aliasing, lifetimes, thread safety, ABI contracts, and other relevant sources of Undefined Behavior rather than relying on a generic assurance.

Keep `#![deny(unsafe_code)]` at the crate level. An item or module that genuinely needs unsafe Rust may use the smallest practical local `allow(unsafe_code)`, accompanied by a comment identifying the reason, the safe API boundary, and the documented invariants that make the implementation sound.

## Panics And Fatal Errors

Production code must not use `unwrap`, `expect`, or panic-driven control flow for recoverable states. Use explicit error handling, checked access, or internal helper APIs that make the failure mode clear.

Panics are acceptable only for narrow internal invariants that cannot be triggered by valid user input, valid problem files, CLI options, environment variables, or resource limits. Document the invariant at the point of use.

When the C executable reports an observable fatal error, the Rust port should match the C behavior: diagnostic stream, wording where compatibility depends on it, exit status, and whether partial output is emitted before termination. Tests may use `unwrap` or `expect` when it makes test failures clearer.

## Porting Style

- Preserve command-line behavior, output compatibility, parsing rules, proof behavior, and edge cases from the C executable.
- Keep data structures and algorithms close enough to the original source that future audits can compare Rust behavior against `eprover/`.
- Use idiomatic Rust ownership and error handling, but avoid abstractions that obscure the correspondence with the original implementation.
- Prefer deterministic behavior and explicit state transitions, especially in prover logic, indexing, ordering, and scheduling code.
- Add tests for compatibility-sensitive behavior and performance-relevant code paths.

## Compatibility Rules

- Preserve stdout/stderr structure, SZS status output, proof-output order, parser diagnostics, include handling, stdin behavior, and line-ending normalization.
- Preserve CLI option parsing, environment-variable behavior, resource-limit handling, timeout behavior, and file path semantics closely enough for the native-Linux Linode compatibility matrices to pass.
- Keep deterministic ordering explicit. Do not rely on hash-map iteration order, filesystem traversal order, pointer addresses, or thread scheduling when output or proof search can observe the result.
- Choose integer widths and conversions deliberately. Match the C contract for overflow, truncation, signedness, sentinel values, and boundary checks; use checked, saturating, or wrapping operations only when they match the original behavior.
- Preserve proof-search state transitions and mutation order when they affect clause selection, simplification, indexing, ordering, scheduling, or proof objects.

## Data Structures And Ownership

Object identity, sharing, allocation reuse, and mutation ordering are often part of E's behavior and performance contract. Before replacing a C idiom with a higher-level Rust abstraction, audit whether callers depend on identity, global state, allocation lifetime, freelists, term banks, clause indexes, or fatal-error behavior.

Use safe Rust designs such as arenas, interners, stable handles, index-based storage, explicit queues, and scoped owner objects to preserve those contracts. Do not replace pointer identity with structural equality, remove sharing, clone large term/clause structures casually, or hide performance-critical indexes behind abstractions that make the original optimization hard to verify.

## Dependencies

Prefer the Rust standard library and small, focused crates. Add a dependency only when it has a clear porting, correctness, compatibility, or performance purpose.

Before adding a crate, review and document its license, maintenance status, transitive dependency impact, feature flags, and whether it changes compatibility or deployment assumptions. Use minimal features where practical.

A dependency must not bypass this project's unsafe-Rust policy through project wrapper code. Calls to unsafe dependency APIs are permitted only for a concrete reason allowed by the policy above, must remain behind a safe project API, and must document why all safety requirements are upheld and Undefined Behavior cannot occur.

## Documentation Expectations

For each ported subsystem, identify the original C source units used as the reference, including relevant `docs/c_source_docs/` pages when available. Document compatibility-sensitive deviations and the reason for them.

When porting performance-sensitive code, record the important performance assumptions: data-structure identity, indexing strategy, allocation model, expected hot paths, and benchmark coverage.
