# Ordered mixed proof-object extraction audit

## Status

Completed for Bead `E_Rust_Port-j76.1.12`. The represented proof-object graph
now owns the C-shaped ordering operation over its exact borrowed clause/formula
nodes, and focused regressions cover mixed multi-root sibling interleaving plus
the complete derivation printing metadata and structural grammar.

## Question

Does the current Rust proof graph already provide the identity, ordering, and
PCL/TSTP rendering required by C `DerivationExtract`, `DerivationTopoSort`,
`DerivationPrint`, `DerivedPCLPrint`, and `DerivedTSTPPrint`, or is a second
pointer arena/order owner necessary?

## C contract

C allocates one `DerivedCell` per distinct `Clause_p` or `WFormula_p`. Extraction
counts incoming child references while walking backward from the roots.
`DerivationTopoSort` then:

1. queues zero-reference roots in root-stack order;
2. processes roots and newly released derived parents through a FIFO queue;
3. releases clause parents before formula parents because they occupy separate
   temporary stacks;
4. defers newly released axioms to a LIFO stack;
5. appends that axiom stack to the internal order; and
6. lets `DerivationPrint` traverse the result backward.

The derivation expression printers are table-driven by `opids`, `opstatus`, and
`optheory`. Their distinct structural cases are direct CNF/FOF quotes,
zero/one/two-parent operations, numeric `DCACRes`, augmenting `DCCnfAddArg`, and
the special introduced-definition spelling.

## Rust ownership decision

`ProofObjectGraph` already retains references to the exact extracted owners for
the lifetime of an immutable proof-state borrow. Clause generations and formula
entry sources distinguish nodes that share visible identifiers, and experiment
019 established that an address arena would not add useful identity.

The C ordering algorithm previously existed only as a private executable
display helper. It is now `ProofObjectGraph::c_ordered_nodes`, so ordering is a
property of the borrowed owner graph and is shared by list and DOT consumers.
It computes reference counts transiently without cloning or renumbering the
nodes. This is the safe Rust equivalent of C's pointer-owned `DerivedCell`
graph; persistent mutable `is_fresh` and destructive `ref_count` fields are not
needed by a current consumer.

## New regression coverage

The mixed-order regression builds two roots (one clause and one formula), two
derived siblings, one shared derived parent, and clause/formula axioms. It pins
the exact final C order:

```text
formula axiom, clause axiom, shared parent,
formula sibling, clause sibling, formula root, clause root
```

This exercises root FIFO order, sibling release timing, shared-parent reference
counts, clause-before-formula release, and final axiom-stack reversal in one
graph.

The derivation regressions independently pin all 56 Rust `DerivationCode`
entries against C's operation-id/status/theory tables (the two quote variants
share one C opcode), then cover direct quotes, nested zero-parent operations,
CNF and FOF parents, two CNF parents, extra CNF arguments, AC axiom expansion,
theory annotations, formula-name remapping, and introduced definitions in both
PCL and TSTP expression output. Existing executable tests compose those
expressions into clause and formula derived-step records.

## Reference evidence

The production ordering algorithm is unchanged; it moved from the executable
module into the graph owner and now avoids the former temporary mixed-edge
clone. The focused `ans_test06.p`, synthetic Socrates, and `ALL_RULES.p` proof
tests still pass byte-exact stored expectations. Earlier archived C/Rust
comparisons already establish exact normalized proof lists for those workloads
and `LUSK6.lop`, covering formula/clause interleaving, formula copy ancestry,
demodulator display ids, AC parents, and final root marking.

No WSL distribution or local C executable is installed in this environment, so
a fresh live C run was unavailable. The new multi-root order expected by the
unit test is derived directly from the vendored `DerivationTopoSort` loops and
their `DerivationPrint` reverse traversal; the upstream source tree remains
unchanged.

## Performance decision

The refactor performs the same linear graph traversal and allocates the same
reference-count, parent-list, queue, stack, and result vectors as before. It no
longer clones the complete mixed-edge vector before ordering. Proof extraction
and rendering are off the saturation hot path, so a standalone benchmark would
not provide actionable signal. Full functional and lint gates are the relevant
validation.

## Validation

- mixed multi-root/sibling ordering regression: passed
- complete C derivation operation metadata regression: passed
- two-parent, theory, and introduced-definition render regression: passed
- 15 focused proof-object list/display regressions: passed
- `ans_test06.p`, synthetic Socrates, and `ALL_RULES.p` proof regressions: passed
- `cargo fmt --all -- --check`
- `cargo test --locked --lib --quiet`: 4,092 passed
- `cargo test --locked --bins --quiet`: all binary targets passed
- `cargo test --locked --test eprover_schedule --quiet`: 3 passed
- `cargo test --locked --test executable_inventory --quiet`: 1 passed
- `cargo check --locked --all-targets`
- `cargo clippy --locked --all-targets -- -D warnings`
- C-source coverage: 492 source files covered by 266 unit docs
- Change Later wording: 269 Markdown files checked
- local Markdown links: 269 Markdown files checked
- regeneration preservation: manual sections preserved in 268 Markdown files
