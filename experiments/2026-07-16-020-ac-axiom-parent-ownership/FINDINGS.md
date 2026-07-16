# Signature AC-axiom parent ownership audit

## Status

Completed for Bead `E_Rust_Port-j76.1.11`. Signature-owned AC parent
references now retain the identity and current visible metadata of the exact
proof-state-owned clause across proof-documentation renumbering, proof-state
cloning, owner-set moves, and archive/requeue replacement.

## C ownership contract

`SigCell::ac_axioms` is a `PStack_p` of raw `Clause_p` values. Initial proof
state setup scans the evaluated copies in `state->unprocessed`, and dynamic
scanning records the selected clause after forward contraction. The clause cell
then moves between intrusive unprocessed, processed, temporary, and archive
sets without changing address. Proof-documentation functions can mutate its
visible `ident`; later AC-resolution documentation and `DCACRes` expansion see
that current value by dereferencing the same pointer.

Archive/requeue is intentionally asymmetric. C moves the original cell into
the archive and creates a new quoted cell for requeue, so the signature keeps
pointing at the archived original rather than following the new active copy.

## Discovered Rust failure

Rust already stored `ClauseDerivationRef { ident, source, generation }` in the
signature and searched every proof-state owner set. However, derived equality,
ordering, and hashing treated all three fields as identity. A level-6 dynamic
AC scan could therefore capture `(4162, source, generation)` and then
`DocClauseQuote` could renumber the same clause to `(1, source, generation)`.
The signature reference no longer resolved even though its generation still
identified the exact proof node.

Generation-zero dynamic clauses had a second exposure: without promotion before
retention, no immutable component survived visible renumbering.

## Implemented mapping

- A nonzero derivation generation is the complete process-local identity for
  equality, ordering, and hashing. `ident` and CSSCPA `source` remain rendering
  metadata. Generation-zero legacy/test references continue to compare by the
  original `(ident, source)` pair.
- AC scanning now takes mutable clauses/sets and assigns a fresh generation
  before retaining a generation-zero clause in the signature. Repeated scans
  preserve C's property-transition and duplicate-suppression behavior.
- `ProofState::ac_axiom_parent_refs` resolves signature identities through the
  current proof owners and reconstructs current rendering metadata. AC
  modification documentation uses those resolved parents, reproducing C's
  live-pointer identifier after proof-output renumbering.
- Ordinary owner-set moves preserve the generation. `ProofState::clone` copies
  the signature references and owner clauses together, so each snapshot
  resolves into its own storage. Archive/requeue assigns only the new quoted
  node a fresh generation, leaving the signature on the archived original.

No address-stabilizing arena, per-clause allocation, reference counting, or
unsafe code is introduced.

## Reset boundary

C `ProofStateResetClauseSets` frees all ordinary owners of signature AC pointers
without clearing `sig->ac_axioms` or the associated function properties. The
upstream tree has no production caller. Rust preserves the broader reset shape;
its stale value references resolve to no clause rather than becoming dangling
pointers. Clearing only the Rust vector would also make the retained AC
properties inconsistent with a later rescan, so reusable reset semantics remain
a post-compatibility design issue under Bead `E_Rust_Port-j76.4.348`.

## Regression coverage

The new tests establish that:

- generation identity remains equal and hashes/orders identically after visible
  identifier and source changes, while generation-zero references retain legacy
  pair identity;
- commutativity and associativity scans promote generation-zero parents and do
  not duplicate signature entries;
- a dynamically detected clause remains resolvable after level-6 `new_given`
  renumbering and processed-set insertion;
- a cloned proof state resolves the same signature identity into a distinct,
  clone-owned clause allocation;
- archive/requeue retains the signature parent in the archive and gives the
  requeued copy a distinct identity;
- AC-resolution proof documentation prints the current owner ids rather than
  stale signature snapshots.

The upstream source comparison is decisive for the mutation contract: C stores
and later dereferences the same raw pointer. Earlier `ALL_RULES.p` GDB/output
evidence in experiment 001 and commit `045a18e7` already verified the selected
AC-node/archive asymmetry against C. This environment has no installed WSL
distribution, so a new executable C/Rust run was not available for the newly
pinned level-6 dynamic-renumbering edge case.

## Performance decision

The change adds no allocation and no work to ordinary clause creation. The only
new generation assignment occurs once when an AC property first retains a
generation-zero clause. Reference comparison/hash uses a small generation-zero
branch, and resolving current AC metadata scans the already small signature AC
list only on proof-documenting AC modification paths. A separate benchmark
would not provide useful signal; the full Rust quality gates cover the affected
hot-path integration.

## Validation

- Focused identity, AC scan, dynamic processing, clone/archive/requeue, and
  AC-resolution documentation regressions: passed
- `cargo test --locked --lib --quiet`: 4,089 passed
- `cargo test --locked --bins --quiet`: all binary targets passed
- `cargo test --locked --test eprover_schedule --quiet`: 3 passed
- `cargo test --locked --test executable_inventory --quiet`: 1 passed
- `cargo fmt --all -- --check`
- `cargo check --locked --all-targets`
- `cargo clippy --locked --all-targets -- -D warnings`
- C-source coverage, Change Later wording, Markdown-link, and
  regeneration-preservation checks passed
