# PCL lemma compatibility edges

## Question

Are the remaining `pcl_lemmas` differences missing behavior, or intentional
safe and deterministic representations of undefined, dead, or surprising C
semantics?

## Source audit

`InferenceWeightsAlloc` allocates a 30-slot array but assigns only 15 slots.
`PCLOpIntroDef`, `PCLOpSatCheck`, `PCLOpCondense`, `PCLOpSplitEquiv`,
`PCLOpApplyDef`, and all FOF transformation slots retain allocator bytes. This
is true even though the header defines a condense weight. Rust initializes the
whole typed array to zero before applying every explicit C assignment.

The header's documented quality formula and `LemmaParamCell` include
`proof_tree_w` and `proof_dag_w`, but `PCLStepComputeLemmaWeight` reads neither.
The executable formula multiplies its reference term directly by
`1 + proof_tree_size`; Rust preserves the implementation rather than the stale
comment.

`PCLProtSeqFindLemmas` marks and increments before testing
`res > max_number`, so a zero maximum selects one qualifying lemma. Rust keeps
that externally visible off-by-one. `PCLStepUpdateRefs` null-dereferences a
missing top-level quoted parent, while missing nested counter parents are
silently ignored. Rust keeps counter updates non-fatal and reports missing
parents when proof-size traversal needs them.

## Differential corpora

`uninitialized-opcode-weights.pcl` routes a three-step proof through
`condense` and `cdclpropres`, exercising two C-unassigned inference-weight
slots. With relative quality enabled, both values feed proof size and the
printed minimum quality. `dangling-parent.pcl` contains one missing top-level
parent.

```powershell
wsl.exe -d Ubuntu-24.04 -- `
  /home/rober/.cache/e-rust-port/bin/17026b1bfe61aaf223cfaae54947c8d2679c31a0/tools/epcllemma `
  --min-lemma-quality-rel=1 --max-lemmas=0 --output-level=3 `
  /mnt/c/Users/rober/Code/E_Rust_Port/experiments/2026-07-16-063-pcl-lemma-edges/uninitialized-opcode-weights.pcl

.\target\release\epcllemma.exe `
  --min-lemma-quality-rel=1 --max-lemmas=0 --output-level=3 `
  experiments\2026-07-16-063-pcl-lemma-edges\uninitialized-opcode-weights.pcl
```

The archived upstream tool is commit
`17026b1bfe61aaf223cfaae54947c8d2679c31a0`, built with GCC 13.3 and glibc
2.39.

## Results

The uninitialized-slot corpus is byte-identical. Both tools print minimum
quality `1.142857`, mark PCL id 1 despite `--max-lemmas=0`, and leave ids 2 and
3 unmarked. The archived allocator happened to supply zero in the two
unassigned slots, matching Rust's deterministic values. A different heap byte
is legal under the C implementation and is not a reproducible contract.

The archived tool terminates with signal 11 on `dangling-parent.pcl`. Rust
prints its two status lines and then reports
`epcllemma: Reference to non-existing step`. The Rust behavior preserves the
valid-input algorithm and turns C undefined behavior into a stable diagnostic.

The first current permanent comparison run,
`.artifacts/e-compare/20260716-221956-126266-tools/`, found two executable
diagnostic mismatches unrelated to lemma scoring:

- stdin parser errors named the Rust source `-` instead of C `<stdin>`;
- missing named inputs used scanner-open wording instead of C's pre-open
  `InputOpen` stat boundary.

Rust now uses `<stdin>` for scanner diagnostics and routes named inputs through
the shared C-shaped `input_open` boundary while preserving output-file creation
before later input failure. The final report,
`.artifacts/e-compare/20260716-222402-439924-tools/`, has all 15 `epcllemma`
cases exact.

## Retained regressions

The nine focused core tests now pin deterministic zeroes for all 15 unassigned
opcodes, bit-identical quality after changing both dead proof-weight fields,
non-fatal missing reference-counter parents, the exact proof-size diagnostic,
the sequential zero-limit off-by-one, inference weights/caching, reference
classes, quality gates, and recursive/flat selection. The 23 executable tests
pin the corrected source and stat diagnostics alongside the full option/output
surface.

## Compatibility decision

Rust will not expose allocator residue or reproduce a null dereference. Zero is
the only deterministic neutral weight consistent with the archived run and the
explicit zero-valued inference defaults. The executable quality formula and
sequential limit check remain exact even where the header comment or API shape
suggests different behavior. Cleanup of dead parameters and the off-by-one is
correctly deferred until after drop-in compatibility.

## Validation

The nine focused core and 23 executable tests pass, and the current 15-case
archived-C differential is exact. Final repository gates pass formatting,
all-target/all-feature checking, pedantic Clippy with warnings denied, all
4,189 library tests plus binary and integration targets, and a locked release
build of every binary. The 32 Python interoperability tests and all four
C-source documentation checks also pass.
