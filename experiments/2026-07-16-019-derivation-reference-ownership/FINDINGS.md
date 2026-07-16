# Derivation-reference ownership audit

## Status

Completed for Bead `E_Rust_Port-j76.1.10`. Compact clause and formula
derivation references preserve the identity contract of C proof-parent pointers;
state-owned address handles are not required by any current consumer.

## C identity contract

C stores `Clause_p` and `WFormula_p` directly in derivation stacks. The pointed
objects are individually allocated, moved between intrusive owner sets without
changing address, and retained in proof archives. Flat archive/requeue copies
are different pointers even when they deliberately retain a visible clause or
formula identifier.

With `CLAUSE_PERM_IDENT`, the C debug build also assigns every allocated clause
cell a never-modified allocation number. This confirms that wrapper allocation,
not the printable `ident` alone, is the relevant internal identity.

## Rust clause identity

`ClauseDerivationRef` is the process-local logical equivalent of `Clause_p`:

- `ident` preserves C-visible proof and clause numbering;
- `source` preserves the four-bit CSSCPA source used in printed clause IDs;
- `generation` distinguishes separately allocated proof nodes that retain the
  same `ident` and source.

Newly allocated clauses have unique `ident` values, so generation zero is
unambiguous while that visible metadata remains unchanged. Working clones/copies
preserve generation because they represent the same logical clause. Long-lived
references whose clauses can be proof-output-renumbered must first receive a
nonzero generation; the signature AC-parent case and generation-based equality
are audited in experiment 020. Every represented archive/requeue path that
creates a new proof node while retaining the ID calls
`refresh_derivation_generation` before capturing or storing its derivation
reference:

- `clause_archive`;
- `clause_archive_copy`;
- proof-state axiom evaluation copies;
- processed-clause reset/requeue copies;
- backward-simplification requeues.

The generation counter uses checked process-wide allocation and fails explicitly
on exhaustion instead of silently aliasing a live proof node.

## Storage and consumer audit

| C pointer use | Rust representation | Mutation safety |
| --- | --- | --- |
| Derivation parent | `ClauseDerivationRef` or `FormulaDerivationRef` | Value key survives moves and relocation |
| Clause archives/requeues | owned `Clause` move plus refreshed generation for a new node | Original and copy resolve independently |
| Formula archives/flat copies | stable wrapper `entry_id` source key | Audited in experiment 018 |
| Rewrite demodulator trace | clause ID encoding plus derivation generation | Survives archive/requeue copies |
| AC-axiom parents | exact clause derivation refs in the signature | Proof lookup checks all owner sets |
| Parent-liveness snapshot | hash set of exact clause refs | No borrowed address is retained |
| Clause-set indexes | stable local slots/evaluation handles rebuilt after compaction | No derivation stores a set-local slot |
| Proof graph visited sets | temporary borrowed addresses | Sets remain immutably borrowed for the graph lifetime |

Legacy generation-zero references remain supported for imported/test-facing
ident/source data. Proof lookup first searches every owner set for an exact
reference and only then permits the deliberate sourceless ID fallback.

## New regression

The focused clause-set regression creates an original and a requeued clause with
the same visible ID and CSSCPA source but different derivation generations. It
then:

1. creates enough holes to trigger sparse-store compaction;
2. verifies both exact references after physical relocation;
3. transfers the entire set into a different owner set;
4. verifies both exact references again.

The test also pins clone semantics: an ordinary clone is a snapshot of the same
logical clause and preserves its complete derivation reference.

Existing proof-state regressions distinguish archived/requeued generations,
check exact references across every proof owner before legacy fallback, preserve
rewrite generations, follow quote chains, and retain mixed formula/clause proof
parents.

## Compatibility and performance decision

No state-owned address arena is introduced. It would centralize all clause and
formula ownership solely to recreate pointer stability that the logical keys
already provide, while set-local handles cannot survive the C-compatible moves
between active, processed, temporary, and archive sets.

This decision is consistent with the measured ownership experiments:

- experiment 013 rejected boxed clause storage after process RSS regressed on
  both repeated- and unique-owner workloads despite a lower Massif total;
- experiment 018 found no formula consumer that retains an address across a set
  mutation and retained compact deque ownership without extra allocation.

This slice changes documentation and regression coverage only. There is no
runtime, output, or allocation change to compare against C and no performance
benchmark is warranted. Full Rust quality gates are the relevant validation.

## Validation

- `cargo fmt --all -- --check`
- focused compaction/transfer, exact-generation lookup, demodulator-generation,
  and archive-copy regressions: 6 passed
- `cargo test --locked --lib --quiet`: 4,087 passed
- `cargo test --locked --bins --quiet`: all binary targets passed
- `cargo test --locked --test eprover_schedule --quiet`: 3 passed
- `cargo test --locked --test executable_inventory --quiet`: 1 passed
- `cargo check --locked --all-targets`
- `cargo clippy --locked --all-targets -- -D warnings`
