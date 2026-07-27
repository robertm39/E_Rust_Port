# PCL mini-step ownership and compatibility audit

## Status

Completed for Bead `E_Rust_Port-j76.2.129`. Rust's owning logic enum,
call-scoped shell option, and explicit protocol-bank rendering are safe
replacements for C's untagged union, process-global option, and stored raw bank
pointer. Observable mini-step grammar and output quirks remain compatible. The
vendored C source remained unchanged.

## C ownership and call graph

`PCLMiniStepCell` stores an untagged clause/formula union plus a `TB_p` captured
at parse time. Property bits decide which union arm is freed or printed. Formula
printing uses the stored bank; clause printing accepts a bank argument.

The only production calls to `PCLMiniStepPrintFormat` come from
`PCLMiniProtPrint` and `PCLMiniProtPrintProofClauses`. Both pass the same
protocol-owned bank used by `PCLMiniStepParse`, so there is no supported call
path in which formula and clause rendering intentionally use different banks.

## Rust ownership equivalence

`PclMiniStepLogic` is an owning discriminated enum. Formula terms and compact
clause literals are safe shared term handles; `PclMiniStep` stores no reference
or raw pointer to its owner. `PclMiniProtocol` owns the term bank and supplies it
to every mini-step render, making the C production lifetime invariant explicit
and preventing stale self-references when the protocol moves.

C's `SupportShellPCL` defaults to false and is assigned true only by
`epclextract`. Rust passes false from the corresponding ordinary tools and true
from `epclextract`; disabled/enabled/disabled regression parsing proves the
choice is local to each call.

## Legacy behavior retained

- compound PCL identifiers are rejected in mini mode;
- the optional extra field accepts only a single-quoted string;
- standalone id zero parses and prints, while Rust destruction remains safe
  rather than reproducing C's contradictory free-time assertion; and
- a clausal shell step retains the empty logical-content slot, for example
  `cnf(3,plain,,2).`.

Possible cleanup remains isolated in post-compatibility Beads
`E_Rust_Port-j76.4.950`, `.955`, `.956`, `.957`, `.958`, `.959`, and `.979`.

## Performance

The Rust step has no bank back-pointer and needs no custom destructor. Enum
dispatch replaces property-bit-selected union access without heap allocation;
term handles remain reference-counted as elsewhere in the port. Rendering uses
the already-owned protocol bank directly, so the safe design adds no lookup or
copying layer to the production call graph.

## Validation

- focused mini-step tests cover clause, formula, shell, format dispatch,
  compound-id rejection, narrow extras, call-scoped shell mode, standalone
  zero-id destruction, and exact PCL/TSTP output;
- formatting and strict Clippy pass;
- full all-target/all-feature tests pass; and
- source-document coverage, Change Later wording, links, and regeneration
  preservation pass.
