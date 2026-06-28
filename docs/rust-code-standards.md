# Rust Code Standards

This project is a Rust port of the E theorem prover. Rust code must preserve the behavior, feature coverage, and performance expectations of the original C implementation while using clear, idiomatic Rust.

This file is the canonical Rust standards document for the port. Do not add a second standards document under `docs/c_source_docs/`; that tree documents the original C source.

## Required Checks

Once a Rust crate exists, all Rust code changes must pass:

```powershell
cargo fmt --check
cargo test --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings -D clippy::pedantic
```

Treat clippy pedantic findings as design feedback. Prefer small, explicit fixes that keep the port close to the original implementation model.

Executable changes must also build the release binary once the `eprover` target exists:

```powershell
cargo build --locked --release --bin eprover
```

Drop-in compatibility work must compare the Rust executable with the C reference:

```powershell
.\e-interop.ps1 build-reference
.\e-interop.ps1 compare -RustExe .\target\release\eprover.exe
```

Performance-sensitive or executable-path work must also run the benchmark harness when the Linux Rust toolchain is available:

```powershell
.\e-interop.ps1 benchmark -Runs 5
```

Docs-only changes should run the Markdown link checker from `DOCS.md`.

## Unsafe Rust

Unsafe Rust is prohibited except when it is necessary for interacting with external DLLs.

Do not add unsafe Rust for ordinary porting work, including:

- `unsafe` blocks
- `unsafe fn`
- `unsafe impl`
- `unsafe trait`
- Calls to unsafe APIs through wrapper code
- Other unsafe Rust constructs

Unsafe code for external DLL interop must be narrowly scoped, document the safety invariants at the unsafe boundary, and be wrapped behind safe Rust APIs wherever practical.

If a non-DLL porting task appears to require unsafe Rust, document the blocker and look for a safe design first. Do not add unsafe code outside the external-DLL exception without a project-level standards change.

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
- Preserve CLI option parsing, environment-variable behavior, resource-limit handling, timeout behavior, and file path semantics closely enough for `.\e-interop.ps1 compare` to pass.
- Keep deterministic ordering explicit. Do not rely on hash-map iteration order, filesystem traversal order, pointer addresses, or thread scheduling when output or proof search can observe the result.
- Choose integer widths and conversions deliberately. Match the C contract for overflow, truncation, signedness, sentinel values, and boundary checks; use checked, saturating, or wrapping operations only when they match the original behavior.
- Preserve proof-search state transitions and mutation order when they affect clause selection, simplification, indexing, ordering, scheduling, or proof objects.

## Data Structures And Ownership

Object identity, sharing, allocation reuse, and mutation ordering are often part of E's behavior and performance contract. Before replacing a C idiom with a higher-level Rust abstraction, audit whether callers depend on identity, global state, allocation lifetime, freelists, term banks, clause indexes, or fatal-error behavior.

Use safe Rust designs such as arenas, interners, stable handles, index-based storage, explicit queues, and scoped owner objects to preserve those contracts. Do not replace pointer identity with structural equality, remove sharing, clone large term/clause structures casually, or hide performance-critical indexes behind abstractions that make the original optimization hard to verify.

## Dependencies

Prefer the Rust standard library and small, focused crates. Add a dependency only when it has a clear porting, correctness, compatibility, or performance purpose.

Before adding a crate, review and document its license, maintenance status, transitive dependency impact, feature flags, and whether it changes compatibility or deployment assumptions. Use minimal features where practical.

A dependency must not bypass this project's unsafe-Rust policy through project wrapper code. If a crate exposes unsafe APIs, keep their use out of this project unless the use is required for documented external DLL interop or the unsafe policy is formally changed.

## Documentation Expectations

For each ported subsystem, identify the original C source units used as the reference, including relevant `docs/c_source_docs/` pages when available. Document compatibility-sensitive deviations and the reason for them.

When porting performance-sensitive code, record the important performance assumptions: data-structure identity, indexing strategy, allocation model, expected hot paths, and benchmark coverage.
