# Partial-ordering comparison-symbol boundary

## Status

Completed for Bead `E_Rust_Port-j76.2.18`. Rust preserves the unchanged C enum,
inverse, quasi-order conversion, and five printable symbols while making C's
implicit cache/renderer boundary safe and explicit. The vendored C checkout
remains unchanged.

## Unchanged-C behavior

`CompareResult` has seven consecutive values, but `POCompareSymbol` has only
five entries. The omitted `to_notgteq` and `to_notleeq` values are documented
and used as one-sided LPO cache results. Indexing the symbol table with either
value would be out-of-bounds, so the probe deliberately reads only the five
defined entries.

[`capture_c_symbols.py`](capture_c_symbols.py) compiles
[`probe_compare_symbols.c`](probe_compare_symbols.c) against the pinned,
unchanged `BASICS.a`. Retained [`reference.json`](reference.json) records the
seven discriminants, five symbol bytes, all six defined inverse cases, and the
three sign outcomes of `Q_TO_PART`.

## Owner boundary

[`audit_compare_symbol_owners.py`](audit_compare_symbol_owners.py) records the
complete static boundary in [`owner-audit.json`](owner-audit.json):

- Only `cto_lpo.c` and `cto_cmpcache.c` use the two cache-only C values outside
  their defining header/helper.
- The only C table index is `OCBDebugPrint`. Its matrix starts with only
  equal/uncomparable entries; comparison-chain parsing and generated
  precedence insertions supply only equal/greater/lesser; transitive closure
  propagates only those concrete relations.
- Rust likewise has one production `symbol()` caller in OCB debug rendering.
  Its matrix insertion rejects both cache-only values before mutation, and
  permanent tests pin both rejection cases.

Therefore `CompareResult::symbol() -> Option<&str>` is the exact safe encoding
of the C surface: values zero through four render identically, while values
five and six return `None` instead of permitting an invalid array read. This is
not a missing cache feature; the LPO and comparison-cache owners retain those
values independently of rendering.

## Reproduction

```powershell
C:\Users\rober\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe `
  experiments\2026-07-18-120-compare-symbol-boundary\capture_c_symbols.py `
  --output target\compare-symbol-reference-check.json `
  --expected experiments\2026-07-18-120-compare-symbol-boundary\reference.json

C:\Users\rober\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe `
  experiments\2026-07-18-120-compare-symbol-boundary\audit_compare_symbol_owners.py `
  --output target\compare-symbol-owner-audit-check.json `
  --expected experiments\2026-07-18-120-compare-symbol-boundary\owner-audit.json
```
