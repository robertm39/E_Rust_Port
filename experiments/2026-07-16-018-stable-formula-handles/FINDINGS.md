# Stable formula-handle ownership audit

## Status

Completed for Bead `E_Rust_Port-j76.1.9`. The existing `WrappedFormula`
`entry_id` is sufficient as the Rust replacement for C wrapper-pointer identity;
no address-stabilizing allocation or additional indirection is required.

## C identity contract

The original implementation allocates each `WFormulaCell` separately and uses
its pointer in four identity-sensitive roles:

- intrusive formula-set membership and moves;
- formula parents stored in derivation stacks;
- temporary SInE selection stacks whose members are moved between sets;
- relevance-analysis lists and indexes that refer to the set-owned wrappers.

`WFormulaFlatCopy` allocates a new wrapper pointer but retains the visible
formula `ident`. `FormulaSetArchive` moves the original pointer into the archive,
creates a flat copy for the active set, and makes that copy quote the original.
Thus formula number alone is not sufficient to resolve proof parents.

## Rust mapping

| C pointer responsibility | Rust representation | Lifetime result |
| --- | --- | --- |
| Set membership and moves | `WrappedFormula::entry_id` plus owned deque moves | ID is preserved by extract/insert and deque relocation |
| Flat-copy distinction | `flat_copy` allocates a fresh `entry_id` while retaining `ident` | Original and copy remain independently resolvable |
| Derivation parent | `FormulaDerivationRef { ident, source: entry_id }` | Proof lookup selects the exact wrapper, including archived originals |
| SInE mutation boundary | selection references are converted to entry IDs before set mutation | No borrowed address survives a move |
| Relevance pruning | working clones preserve `entry_id`; the selected clones replace the original state sets | Logical identity survives the ownership-transfer emulation |
| Formula indexes | `PListHandle` indexes arena entries | Index validity depends on arena handles, not wrapper addresses |

The remaining `*const WrappedFormula` values in proof-object and SInE code are
temporary visited/deduplication keys. Rust lifetimes keep the containing formula
sets immutably borrowed for their entire use, so those addresses cannot cross a
deque mutation.

`WrappedFormula::clone` deliberately preserves logical wrapper identity for
state snapshots and ownership-transfer emulation. C `WFormulaFlatCopy` semantics
remain explicit through `flat_copy`, which creates the new identity.

## Regression coverage

The wrapper regression now distinguishes the two copy contracts:

- ordinary clone preserves `entry_id`, derivation reference, and source info;
- flat copy receives a new `entry_id` while preserving C-visible formula state
  and dropping source-only info.

A proof-state regression forces growth of both active and archive formula deques,
moves an original formula from the active set into the archive by stable ID,
inserts its quoted flat copy back into the active set, and verifies that:

- both source-qualified derivation references resolve the intended wrappers;
- the quote chain still resolves to the archived original after both storage
  relocation and the inter-set move.

Existing regressions additionally cover ordered set moves, 4,096-element front
drains, archive/flat-copy derivations, distinct-formula expansion, relevance
pruning identity, and SInE formula moves.

## Performance and compatibility decision

The accepted design adds no runtime code or allocation. A boxed wrapper arena
would make Rust addresses stable but would add pointer chasing and one allocation
per formula without serving any current consumer. Stable logical IDs preserve
the observable C identity contract while allowing compact owned formula sets.

This is a representation audit with no output or algorithm change, so a new
C/Rust output comparison or benchmark would not add signal. The full Rust suite,
all binaries, formatting, checking, and Clippy are the relevant validation gates.

## Validation

- `cargo fmt --all -- --check`
- `cargo test --locked --lib --quiet`: 4,086 passed
- `cargo test --locked --bins --quiet`: all binary targets passed
- `cargo test --locked --test eprover_schedule --quiet`: 3 passed
- `cargo test --locked --test executable_inventory --quiet`: 1 passed
- `cargo check --locked --all-targets`
- `cargo clippy --locked --all-targets -- -D warnings`
