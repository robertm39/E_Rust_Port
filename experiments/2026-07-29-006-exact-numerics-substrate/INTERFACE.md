# Reversible exact-number interface proposal

Status: accepted design boundary; production implementation deferred.

This proposal fixes semantic requirements without exposing a backend. It is
intended for VIRAS, ALASCA-style reasoning, proof objects, parsers, and tests.

## Product boundary

Create private `theory::numbers` implementation modules only when a production
consumer is ready. Export two owned facade types:

```rust
pub struct ExactInteger(backend::Integer);
pub struct ExactRational(backend::Rational);
```

The backend fields and adapter module remain private. Do not:

- make theory algorithms generic over a numeric-backend trait;
- expose backend-associated types;
- implement `Deref` to a backend value; or
- let proof or cache formats depend on backend debug output.

This keeps a backend replacement localized and avoids spreading generic
monomorphization or FFI ownership through the prover.

## Required invariants

Every `ExactRational` is canonical:

- denominator strictly positive;
- numerator and denominator relatively prime;
- zero represented as `0/1`;
- equality and ordering are mathematical rather than representational; and
- all constructors reject a zero denominator before entering a backend.

The initial API should provide:

```rust
impl ExactInteger {
    pub fn parse_decimal(text: &str) -> Result<Self, ParseExactError>;
    pub fn bit_len(&self) -> usize;
    pub fn sign(&self) -> Ordering;
}

impl ExactRational {
    pub fn try_new(
        numerator: ExactInteger,
        denominator: ExactInteger,
    ) -> Result<Self, ZeroDenominator>;
    pub fn numerator(&self) -> &ExactInteger;
    pub fn denominator(&self) -> &ExactInteger;
    pub fn checked_div(&self, rhs: &Self)
        -> Result<Self, DivisionByZero>;
    pub fn floor(&self) -> ExactInteger;
    pub fn ceiling(&self) -> ExactInteger;
    pub fn bit_len(&self) -> usize;
}
```

Addition, subtraction, multiplication, negation, comparison, and the normal
borrowed/owned operator combinations may be added as consumers require them.
Parsing, division, and external inputs must return typed errors rather than
panic. No floating-point conversion may participate in a soundness decision.

## Stable serialization and hashing

Canonical text is ASCII decimal `numerator/denominator`, including `/1` for an
integer. It is locale-independent, has no leading plus sign or redundant
zeroes, and is the initial proof/debug interchange format.

Rust's `Hash` implementation only needs the normal equal-values/equal-hashes
contract. Persistent caches, proof fingerprints, and cross-version tests must
instead use a named `stable_fingerprint_v1` over canonical sign-and-magnitude
integer bytes. A backend's limb layout, randomized hasher, or `Display`
implementation is not a stable protocol.

## Resource and cancellation behavior

Primitive exact operations are atomic from Umlaut's point of view. Theory
algorithms must checkpoint their existing cancellation/budget object:

1. before a batch of arithmetic;
2. before an operation whose input bit lengths predict limit growth; and
3. after the operation, before storing or indexing the result.

The facade exposes bit-length estimates and enforces configured numerator and
denominator limits at construction and result boundaries. A limit breach is a
resource outcome, never an approximate answer. No backend may silently
saturate, round, or substitute a machine integer.

## Adoption gate

The first production change may pin Dashu 0.5.1 only if it also:

1. runs the independent `fractions.Fraction` conformance suite through the
   production facade, including negative floor/ceiling and zero rejection;
2. tests canonical serialization, stable fingerprints, and hash equality;
3. exercises bit limits and cancellation at theory-module boundaries;
4. benchmarks complete consumer workloads and representative CASC problems;
5. updates the root lockfile, notices, dependency matrix, source/runtime size
   measurements, and StarExec package audit; and
6. confirms a clean build has no access to ignored GMP or other reference
   trees.

If Dashu changes its normalization semantics, loses its reviewed license
boundary, or materially regresses on those gates, the facade permits replacing
it with Rug/GMP or another exact backend without changing theory algorithms or
proof formats.
