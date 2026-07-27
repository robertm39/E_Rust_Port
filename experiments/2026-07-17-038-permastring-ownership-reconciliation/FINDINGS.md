# Permanent-string ownership reconciliation

## Objective

Resolve `E_Rust_Port-j76.2.99` by determining whether C permanent-string pointer identity is part of executable behavior or only an ownership mechanism. The vendored C source remains unchanged.

## Production ownership map

| C parser field | C lifetime mechanism | Rust owner and consumer |
| --- | --- | --- |
| `sine` | `PARSE_STRING` / `PermaStringStore` | `Option<String>`, borrowed by SInE selection by content |
| `heuristic_name` | `PARSE_IDENTIFIER` / `PermaString` | `String`, used for named heuristic lookup by content |
| `heuristic_def` | `PARSE_STRING` / `PermaStringStore` | `Option<String>`, cloned into the definition stack and parsed by content |
| `to_pre_prec` | `PARSE_STRING` / `PermaStringStore` | `Option<String>`, borrowed by predefined-precedence parsing |
| `to_pre_weights` | `PARSE_STRING` / `PermaStringStore` | `Option<String>`, borrowed by predefined-weight parsing |

These are the only C production calls outside the registry module and final `eprover` cleanup. C needs the global registry because scanner literals are temporary and parameter cells are copied shallowly. Every downstream use is a content operation; no caller compares these pointers for identity. `PermaStringsFree` runs only during final executable teardown after the parameter and definition owners have been released.

Rust's owned fields and ordinary clones therefore preserve the required lifetime more directly and avoid introducing process-global interning into proof-control and scheduling. The standalone `PermaStringRegistry` remains a complete compatibility surface: equal entries share one `Arc<str>` allocation within a registry epoch, owned insertion consumes its input, optional helpers represent C nulls, and clearing begins a new identity epoch while existing safe handles remain valid.

## Compatibility decision

The migrated initial-port item is complete. Dangling-pointer behavior after `PermaStringsFree`, caller-input address differences, and splay-tree locality are not executable requirements and remain explicitly deferred to `E_Rust_Port-j76.3.37` and C-source reviews `E_Rust_Port-j76.4.73` through `.76`.

## Validation

- The four permanent-string registry tests passed.
- Focused ordering-parameter, heuristic-parameter, strategy-definition, and executable proof-control tests passed for the five production string fields.
- This documentation/tracking-only reconciliation retains the exact `418e5349` runtime baseline: 4,235 default-feature library tests; 4,240 all-feature library tests; every binary target; all 7 integration tests; strict all-target, all-feature pedantic Clippy; and a release `eprover` build.
- Formatting and C-source documentation gates passed; the vendored `eprover/` worktree remained clean.
