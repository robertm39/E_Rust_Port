# Rust Code Standards

This project is a Rust port of the E theorem prover. Rust code must preserve the behavior, feature coverage, and performance expectations of the original C implementation while using clear, idiomatic Rust.

## Required Checks

All Rust code must pass:

```powershell
cargo clippy --all-targets --all-features -- -D warnings -D clippy::pedantic
```

Treat clippy pedantic findings as design feedback. Prefer small, explicit fixes that keep the port close to the original implementation model.

## Unsafe Rust

Unsafe Rust is not permitted.

Do not add:

- `unsafe` blocks
- `unsafe fn`
- `unsafe impl`
- `unsafe trait`
- Calls to unsafe APIs through wrapper code
- Other unsafe Rust constructs

If a porting task appears to require unsafe Rust, document the blocker and look for a safe design first. Do not add unsafe code without a project-level standards change.

## Porting Style

- Preserve command-line behavior, output compatibility, parsing rules, proof behavior, and edge cases from the C executable.
- Keep data structures and algorithms close enough to the original source that future audits can compare Rust behavior against `eprover/`.
- Use idiomatic Rust ownership and error handling, but avoid abstractions that obscure the correspondence with the original implementation.
- Prefer deterministic behavior and explicit state transitions, especially in prover logic, indexing, ordering, and scheduling code.
- Add tests for compatibility-sensitive behavior and performance-relevant code paths.
