<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# E Original C Source Overview

This directory documents the original C implementation in `eprover/` for use while building the Rust port. The original source tree is treated as read-only.

The documentation is organized by source unit: a `.c` and `.h` file with the same directory and basename are documented together, while standalone `.c` and `.h` files receive their own page.

## Coverage

- Source files covered: 492
- Source units documented: 266
- `.c`/`.h` pairs: 226
- Standalone `.c` files: 32
- Standalone `.h` files: 8

## Subsystem Map

| Directory | Units | Role |
| --- | ---: | --- |
| [`BASICS`](BASICS/clb_avlgeneric.md) | 33 | Low-level infrastructure used by the rest of E: memory allocation, dynamic arrays, stacks, trees, strings, errors, dates, OS wrappers, and generic container support. |
| [`CLAUSES`](CLAUSES/ccl_axiomsorter.md) | 54 | Clause, formula, literal, inference, simplification, indexing, subsumption, grounding, relevance, and proof-state machinery. |
| [`CONTRIB/picosat-965`](CONTRIB/picosat-965/app.md) | 7 | Vendored PicoSAT SAT-solver sources used through E's propositional/SAT integration paths. These files follow PicoSAT's API and allocation conventions. |
| [`CONTROL`](CONTROL/cco_batch_spec.md) | 19 | High-level proof-control layer: preprocessing, saturation loop orchestration, inference scheduling, contraction, SInE, splitting, server/session management, and higher-order inference control. |
| [`EXTERNAL`](EXTERNAL/CSSCPA_filter.md) | 2 | Optional external integration helpers, including CSSCPA filtering support. |
| [`HEURISTICS`](HEURISTICS/che_axfilter.md) | 42 | Clause evaluation and strategy machinery: heuristic control blocks, priority functions, weight functions, literal selection, automatic strategy selection, and feature extraction. |
| [`INOUT`](INOUT/cio_basicparser.md) | 13 | Input/output substrate: scanners, parsers, command-line handling, streams, files, temp files, signals, network helpers, and output formatting. |
| [`LEARN`](LEARN/cle_annotations.md) | 14 | Learning and knowledge-base support: example representations, feature extraction, term-space maps, annotations, and KB insertion/description helpers. |
| [`ORDERINGS`](ORDERINGS/cto_cmpcache.md) | 7 | Term ordering implementations and support structures, including KBO, LPO, order-control blocks, precedence/weight handling, and comparison caching. |
| [`PCL2`](PCL2/pcl_analysis.md) | 12 | PCL protocol and proof-object support: proof steps, mini-protocols, identifiers, positions, expressions, checking, lemmas, and proof analysis. |
| [`PROPOSITIONAL`](PROPOSITIONAL/cpr_dpll.md) | 5 | Propositional abstraction and DPLL support: propositional signatures, clauses, formulas, variable sets, and solver routines. |
| [`PROVER`](PROVER/checkproof.md) | 26 | Command-line programs and top-level prover/server/client tools that assemble library modules into executable workflows. |
| [`SIMPLE_APPS`](SIMPLE_APPS/ex_commandline.md) | 2 | Small standalone example or conversion programs built against the E libraries. |
| [`TERMS`](TERMS/cte_acterms.md) | 30 | Typed term representation and manipulation: signatures, term banks, substitutions, matching/unification, higher-order/lambda handling, variable sets, types, and term indexing. |

## Porting Guidance

- Preserve the architecture before improving it: many optimizations are encoded as ownership conventions, global caches, term/ clause sharing, and exact mutation ordering.
- Treat `BASICS`, `TERMS`, and `CLAUSES` as the foundation. Later modules assume their allocation, indexing, and object identity behavior.
- Treat comments about side effects, global variables, and fatal error behavior as part of the interface. E often reports errors by terminating rather than returning recoverable values.
- For performance-sensitive modules, keep freelists, term banks, clause indexes, discrimination/subterm indexes, and heuristic queues explicit in the Rust design.
- Vendored `CONTRIB/picosat-965` files are documented for integration awareness, but their API and license should remain distinct from E-owned code.

## Source Units

### BASICS

- [clb_avlgeneric](BASICS/clb_avlgeneric.md) - BASICS/clb_avlgeneric.h
- [clb_ddarrays](BASICS/clb_ddarrays.md) - BASICS/clb_ddarrays.h, BASICS/clb_ddarrays.c
- [clb_defines](BASICS/clb_defines.md) - BASICS/clb_defines.h
- [clb_dstacks](BASICS/clb_dstacks.md) - BASICS/clb_dstacks.h, BASICS/clb_dstacks.c
- [clb_dstrings](BASICS/clb_dstrings.md) - BASICS/clb_dstrings.h, BASICS/clb_dstrings.c
- [clb_error](BASICS/clb_error.md) - BASICS/clb_error.h, BASICS/clb_error.c
- [clb_fixdarrays](BASICS/clb_fixdarrays.md) - BASICS/clb_fixdarrays.h, BASICS/clb_fixdarrays.c
- [clb_floattrees](BASICS/clb_floattrees.md) - BASICS/clb_floattrees.h, BASICS/clb_floattrees.c
- [clb_intmap](BASICS/clb_intmap.md) - BASICS/clb_intmap.h, BASICS/clb_intmap.c
- [clb_memory](BASICS/clb_memory.md) - BASICS/clb_memory.h, BASICS/clb_memory.c
- [clb_min_heap](BASICS/clb_min_heap.md) - BASICS/clb_min_heap.h, BASICS/clb_min_heap.c
- [clb_newmem](BASICS/clb_newmem.md) - BASICS/clb_newmem.h, BASICS/clb_newmem.c
- [clb_numtrees](BASICS/clb_numtrees.md) - BASICS/clb_numtrees.h, BASICS/clb_numtrees.c
- [clb_numxtrees](BASICS/clb_numxtrees.md) - BASICS/clb_numxtrees.h, BASICS/clb_numxtrees.c
- [clb_objmaps](BASICS/clb_objmaps.md) - BASICS/clb_objmaps.h, BASICS/clb_objmaps.c
- [clb_objtrees](BASICS/clb_objtrees.md) - BASICS/clb_objtrees.h, BASICS/clb_objtrees.c
- [clb_os_wrapper](BASICS/clb_os_wrapper.md) - BASICS/clb_os_wrapper.h, BASICS/clb_os_wrapper.c
- [clb_partial_orderings](BASICS/clb_partial_orderings.md) - BASICS/clb_partial_orderings.h, BASICS/clb_partial_orderings.c
- [clb_pdarrays](BASICS/clb_pdarrays.md) - BASICS/clb_pdarrays.h, BASICS/clb_pdarrays.c
- [clb_pdrangearrays](BASICS/clb_pdrangearrays.md) - BASICS/clb_pdrangearrays.h, BASICS/clb_pdrangearrays.c
- [clb_permastrings](BASICS/clb_permastrings.md) - BASICS/clb_permastrings.h, BASICS/clb_permastrings.c
- [clb_plist](BASICS/clb_plist.md) - BASICS/clb_plist.h, BASICS/clb_plist.c
- [clb_plocalstacks](BASICS/clb_plocalstacks.md) - BASICS/clb_plocalstacks.h, BASICS/clb_plocalstacks.c
- [clb_pqueue](BASICS/clb_pqueue.md) - BASICS/clb_pqueue.h, BASICS/clb_pqueue.c
- [clb_properties](BASICS/clb_properties.md) - BASICS/clb_properties.h
- [clb_pstacks](BASICS/clb_pstacks.md) - BASICS/clb_pstacks.h, BASICS/clb_pstacks.c
- [clb_ptrees](BASICS/clb_ptrees.md) - BASICS/clb_ptrees.h, BASICS/clb_ptrees.c
- [clb_quadtrees](BASICS/clb_quadtrees.md) - BASICS/clb_quadtrees.h, BASICS/clb_quadtrees.c
- [clb_regmem](BASICS/clb_regmem.md) - BASICS/clb_regmem.h, BASICS/clb_regmem.c
- [clb_simple_stuff](BASICS/clb_simple_stuff.md) - BASICS/clb_simple_stuff.h, BASICS/clb_simple_stuff.c
- [clb_stringtrees](BASICS/clb_stringtrees.md) - BASICS/clb_stringtrees.h, BASICS/clb_stringtrees.c
- [clb_sysdate](BASICS/clb_sysdate.md) - BASICS/clb_sysdate.h, BASICS/clb_sysdate.c
- [clb_verbose](BASICS/clb_verbose.md) - BASICS/clb_verbose.h, BASICS/clb_verbose.c

### CLAUSES

- [ccl_axiomsorter](CLAUSES/ccl_axiomsorter.md) - CLAUSES/ccl_axiomsorter.h, CLAUSES/ccl_axiomsorter.c
- [ccl_bce](CLAUSES/ccl_bce.md) - CLAUSES/ccl_bce.h, CLAUSES/ccl_bce.c
- [ccl_clausecpos](CLAUSES/ccl_clausecpos.md) - CLAUSES/ccl_clausecpos.h, CLAUSES/ccl_clausecpos.c
- [ccl_clausefunc](CLAUSES/ccl_clausefunc.md) - CLAUSES/ccl_clausefunc.h, CLAUSES/ccl_clausefunc.c
- [ccl_clauseinfo](CLAUSES/ccl_clauseinfo.md) - CLAUSES/ccl_clauseinfo.h, CLAUSES/ccl_clauseinfo.c
- [ccl_clausepos](CLAUSES/ccl_clausepos.md) - CLAUSES/ccl_clausepos.h, CLAUSES/ccl_clausepos.c
- [ccl_clausepos_tree](CLAUSES/ccl_clausepos_tree.md) - CLAUSES/ccl_clausepos_tree.h, CLAUSES/ccl_clausepos_tree.c
- [ccl_clauses](CLAUSES/ccl_clauses.md) - CLAUSES/ccl_clauses.h, CLAUSES/ccl_clauses.c
- [ccl_clausesets](CLAUSES/ccl_clausesets.md) - CLAUSES/ccl_clausesets.h, CLAUSES/ccl_clausesets.c
- [ccl_clausetrees](CLAUSES/ccl_clausetrees.md) - CLAUSES/ccl_clausetrees.h
- [ccl_condensation](CLAUSES/ccl_condensation.md) - CLAUSES/ccl_condensation.h, CLAUSES/ccl_condensation.c
- [ccl_context_sr](CLAUSES/ccl_context_sr.md) - CLAUSES/ccl_context_sr.h, CLAUSES/ccl_context_sr.c
- [ccl_def_handling](CLAUSES/ccl_def_handling.md) - CLAUSES/ccl_def_handling.h, CLAUSES/ccl_def_handling.c
- [ccl_derivation](CLAUSES/ccl_derivation.md) - CLAUSES/ccl_derivation.h, CLAUSES/ccl_derivation.c
- [ccl_diseq_decomp](CLAUSES/ccl_diseq_decomp.md) - CLAUSES/ccl_diseq_decomp.h, CLAUSES/ccl_diseq_decomp.c
- [ccl_eqn](CLAUSES/ccl_eqn.md) - CLAUSES/ccl_eqn.h, CLAUSES/ccl_eqn.c
- [ccl_eqnlist](CLAUSES/ccl_eqnlist.md) - CLAUSES/ccl_eqnlist.h, CLAUSES/ccl_eqnlist.c
- [ccl_eqnresolution](CLAUSES/ccl_eqnresolution.md) - CLAUSES/ccl_eqnresolution.h, CLAUSES/ccl_eqnresolution.c
- [ccl_ext_index](CLAUSES/ccl_ext_index.md) - CLAUSES/ccl_ext_index.h, CLAUSES/ccl_ext_index.c
- [ccl_f_generality](CLAUSES/ccl_f_generality.md) - CLAUSES/ccl_f_generality.h, CLAUSES/ccl_f_generality.c
- [ccl_factor](CLAUSES/ccl_factor.md) - CLAUSES/ccl_factor.h, CLAUSES/ccl_factor.c
- [ccl_fcvindexing](CLAUSES/ccl_fcvindexing.md) - CLAUSES/ccl_fcvindexing.h, CLAUSES/ccl_fcvindexing.c
- [ccl_findex](CLAUSES/ccl_findex.md) - CLAUSES/ccl_findex.h, CLAUSES/ccl_findex.c
- [ccl_formula_wrapper](CLAUSES/ccl_formula_wrapper.md) - CLAUSES/ccl_formula_wrapper.h, CLAUSES/ccl_formula_wrapper.c
- [ccl_formulafunc](CLAUSES/ccl_formulafunc.md) - CLAUSES/ccl_formulafunc.h, CLAUSES/ccl_formulafunc.c
- [ccl_formulasets](CLAUSES/ccl_formulasets.md) - CLAUSES/ccl_formulasets.h, CLAUSES/ccl_formulasets.c
- [ccl_freqvectors](CLAUSES/ccl_freqvectors.md) - CLAUSES/ccl_freqvectors.h, CLAUSES/ccl_freqvectors.c
- [ccl_g_lithash](CLAUSES/ccl_g_lithash.md) - CLAUSES/ccl_g_lithash.h, CLAUSES/ccl_g_lithash.c
- [ccl_garbage_coll](CLAUSES/ccl_garbage_coll.md) - CLAUSES/ccl_garbage_coll.h, CLAUSES/ccl_garbage_coll.c
- [ccl_gd_transformation](CLAUSES/ccl_gd_transformation.md) - CLAUSES/ccl_gd_transformation.h, CLAUSES/ccl_gd_transformation.c
- [ccl_global_indices](CLAUSES/ccl_global_indices.md) - CLAUSES/ccl_global_indices.h, CLAUSES/ccl_global_indices.c
- [ccl_groundconstr](CLAUSES/ccl_groundconstr.md) - CLAUSES/ccl_groundconstr.h, CLAUSES/ccl_groundconstr.c
- [ccl_grounding](CLAUSES/ccl_grounding.md) - CLAUSES/ccl_grounding.h, CLAUSES/ccl_grounding.c
- [ccl_inferencedoc](CLAUSES/ccl_inferencedoc.md) - CLAUSES/ccl_inferencedoc.h, CLAUSES/ccl_inferencedoc.c
- [ccl_neweval](CLAUSES/ccl_neweval.md) - CLAUSES/ccl_neweval.h, CLAUSES/ccl_neweval.c
- [ccl_overlap_index](CLAUSES/ccl_overlap_index.md) - CLAUSES/ccl_overlap_index.h, CLAUSES/ccl_overlap_index.c
- [ccl_paramod](CLAUSES/ccl_paramod.md) - CLAUSES/ccl_paramod.h, CLAUSES/ccl_paramod.c
- [ccl_pdtrees](CLAUSES/ccl_pdtrees.md) - CLAUSES/ccl_pdtrees.h, CLAUSES/ccl_pdtrees.c
- [ccl_pred_elim](CLAUSES/ccl_pred_elim.md) - CLAUSES/ccl_pred_elim.h, CLAUSES/ccl_pred_elim.c
- [ccl_proofstate](CLAUSES/ccl_proofstate.md) - CLAUSES/ccl_proofstate.h, CLAUSES/ccl_proofstate.c
- [ccl_propclauses](CLAUSES/ccl_propclauses.md) - CLAUSES/ccl_propclauses.h, CLAUSES/ccl_propclauses.c
- [ccl_relevance](CLAUSES/ccl_relevance.md) - CLAUSES/ccl_relevance.h, CLAUSES/ccl_relevance.c
- [ccl_rewrite](CLAUSES/ccl_rewrite.md) - CLAUSES/ccl_rewrite.h, CLAUSES/ccl_rewrite.c
- [ccl_satinterface](CLAUSES/ccl_satinterface.md) - CLAUSES/ccl_satinterface.h, CLAUSES/ccl_satinterface.c
- [ccl_sine](CLAUSES/ccl_sine.md) - CLAUSES/ccl_sine.h, CLAUSES/ccl_sine.c
- [ccl_splitting](CLAUSES/ccl_splitting.md) - CLAUSES/ccl_splitting.h, CLAUSES/ccl_splitting.c
- [ccl_subsumption](CLAUSES/ccl_subsumption.md) - CLAUSES/ccl_subsumption.h, CLAUSES/ccl_subsumption.c
- [ccl_subterm_index](CLAUSES/ccl_subterm_index.md) - CLAUSES/ccl_subterm_index.h, CLAUSES/ccl_subterm_index.c
- [ccl_subterm_tree](CLAUSES/ccl_subterm_tree.md) - CLAUSES/ccl_subterm_tree.h, CLAUSES/ccl_subterm_tree.c
- [ccl_tautologies](CLAUSES/ccl_tautologies.md) - CLAUSES/ccl_tautologies.h, CLAUSES/ccl_tautologies.c
- [ccl_tcnf](CLAUSES/ccl_tcnf.md) - CLAUSES/ccl_tcnf.h, CLAUSES/ccl_tcnf.c
- [ccl_tformulae](CLAUSES/ccl_tformulae.md) - CLAUSES/ccl_tformulae.h, CLAUSES/ccl_tformulae.c
- [ccl_unfold_defs](CLAUSES/ccl_unfold_defs.md) - CLAUSES/ccl_unfold_defs.h, CLAUSES/ccl_unfold_defs.c
- [ccl_unit_simplify](CLAUSES/ccl_unit_simplify.md) - CLAUSES/ccl_unit_simplify.h, CLAUSES/ccl_unit_simplify.c

### CONTRIB/picosat-965

- [app](CONTRIB/picosat-965/app.md) - CONTRIB/picosat-965/app.c
- [main](CONTRIB/picosat-965/main.md) - CONTRIB/picosat-965/main.c
- [picogcnf](CONTRIB/picosat-965/picogcnf.md) - CONTRIB/picosat-965/picogcnf.c
- [picomcs](CONTRIB/picosat-965/picomcs.md) - CONTRIB/picosat-965/picomcs.c
- [picomus](CONTRIB/picosat-965/picomus.md) - CONTRIB/picosat-965/picomus.c
- [picosat](CONTRIB/picosat-965/picosat.md) - CONTRIB/picosat-965/picosat.h, CONTRIB/picosat-965/picosat.c
- [version](CONTRIB/picosat-965/version.md) - CONTRIB/picosat-965/version.c

### CONTROL

- [cco_batch_spec](CONTROL/cco_batch_spec.md) - CONTROL/cco_batch_spec.h, CONTROL/cco_batch_spec.c
- [cco_clausesplitting](CONTROL/cco_clausesplitting.md) - CONTROL/cco_clausesplitting.h, CONTROL/cco_clausesplitting.c
- [cco_diseq_decomp](CONTROL/cco_diseq_decomp.md) - CONTROL/cco_diseq_decomp.h, CONTROL/cco_diseq_decomp.c
- [cco_einteractive_mode](CONTROL/cco_einteractive_mode.md) - CONTROL/cco_einteractive_mode.h, CONTROL/cco_einteractive_mode.c
- [cco_eqnresolving](CONTROL/cco_eqnresolving.md) - CONTROL/cco_eqnresolving.h, CONTROL/cco_eqnresolving.c
- [cco_eserver](CONTROL/cco_eserver.md) - CONTROL/cco_eserver.h, CONTROL/cco_eserver.c
- [cco_esession](CONTROL/cco_esession.md) - CONTROL/cco_esession.h, CONTROL/cco_esession.c
- [cco_factoring](CONTROL/cco_factoring.md) - CONTROL/cco_factoring.h, CONTROL/cco_factoring.c
- [cco_forward_contraction](CONTROL/cco_forward_contraction.md) - CONTROL/cco_forward_contraction.h, CONTROL/cco_forward_contraction.c
- [cco_gproc_ctrl](CONTROL/cco_gproc_ctrl.md) - CONTROL/cco_gproc_ctrl.h, CONTROL/cco_gproc_ctrl.c
- [cco_ho_inferences](CONTROL/cco_ho_inferences.md) - CONTROL/cco_ho_inferences.h, CONTROL/cco_ho_inferences.c
- [cco_interpreted](CONTROL/cco_interpreted.md) - CONTROL/cco_interpreted.h, CONTROL/cco_interpreted.c
- [cco_paramodulation](CONTROL/cco_paramodulation.md) - CONTROL/cco_paramodulation.h, CONTROL/cco_paramodulation.c
- [cco_preprocessing](CONTROL/cco_preprocessing.md) - CONTROL/cco_preprocessing.h, CONTROL/cco_preprocessing.c
- [cco_proc_ctrl](CONTROL/cco_proc_ctrl.md) - CONTROL/cco_proc_ctrl.h, CONTROL/cco_proc_ctrl.c
- [cco_proofproc](CONTROL/cco_proofproc.md) - CONTROL/cco_proofproc.h, CONTROL/cco_proofproc.c
- [cco_scheduling](CONTROL/cco_scheduling.md) - CONTROL/cco_scheduling.h, CONTROL/cco_scheduling.c
- [cco_simplification](CONTROL/cco_simplification.md) - CONTROL/cco_simplification.h, CONTROL/cco_simplification.c
- [cco_sine](CONTROL/cco_sine.md) - CONTROL/cco_sine.h, CONTROL/cco_sine.c

### EXTERNAL

- [CSSCPA_filter](EXTERNAL/CSSCPA_filter.md) - EXTERNAL/CSSCPA_filter.c
- [cex_csscpa](EXTERNAL/cex_csscpa.md) - EXTERNAL/cex_csscpa.h, EXTERNAL/cex_csscpa.c

### HEURISTICS

- [che_axfilter](HEURISTICS/che_axfilter.md) - HEURISTICS/che_axfilter.h, HEURISTICS/che_axfilter.c
- [che_axiomscan](HEURISTICS/che_axiomscan.md) - HEURISTICS/che_axiomscan.h, HEURISTICS/che_axiomscan.c
- [che_clausefeatures](HEURISTICS/che_clausefeatures.md) - HEURISTICS/che_clausefeatures.h, HEURISTICS/che_clausefeatures.c
- [che_clausesetfeatures](HEURISTICS/che_clausesetfeatures.md) - HEURISTICS/che_clausesetfeatures.h, HEURISTICS/che_clausesetfeatures.c
- [che_clauseweight](HEURISTICS/che_clauseweight.md) - HEURISTICS/che_clauseweight.h, HEURISTICS/che_clauseweight.c
- [che_dagweight](HEURISTICS/che_dagweight.md) - HEURISTICS/che_dagweight.h, HEURISTICS/che_dagweight.c
- [che_diversityweight](HEURISTICS/che_diversityweight.md) - HEURISTICS/che_diversityweight.h, HEURISTICS/che_diversityweight.c
- [che_fcode_featurearrays](HEURISTICS/che_fcode_featurearrays.md) - HEURISTICS/che_fcode_featurearrays.h, HEURISTICS/che_fcode_featurearrays.c
- [che_fifo](HEURISTICS/che_fifo.md) - HEURISTICS/che_fifo.h, HEURISTICS/che_fifo.c
- [che_funweights](HEURISTICS/che_funweights.md) - HEURISTICS/che_funweights.h, HEURISTICS/che_funweights.c
- [che_gdweight](HEURISTICS/che_gdweight.md) - HEURISTICS/che_gdweight.h, HEURISTICS/che_gdweight.c
- [che_hcb](HEURISTICS/che_hcb.md) - HEURISTICS/che_hcb.h, HEURISTICS/che_hcb.c
- [che_hcbadmin](HEURISTICS/che_hcbadmin.md) - HEURISTICS/che_hcbadmin.h, HEURISTICS/che_hcbadmin.c
- [che_heuristics](HEURISTICS/che_heuristics.md) - HEURISTICS/che_heuristics.h, HEURISTICS/che_heuristics.c
- [che_learning](HEURISTICS/che_learning.md) - HEURISTICS/che_learning.h, HEURISTICS/che_learning.c
- [che_levweight](HEURISTICS/che_levweight.md) - HEURISTICS/che_levweight.h, HEURISTICS/che_levweight.c
- [che_lifo](HEURISTICS/che_lifo.md) - HEURISTICS/che_lifo.h, HEURISTICS/che_lifo.c
- [che_litselection](HEURISTICS/che_litselection.md) - HEURISTICS/che_litselection.h, HEURISTICS/che_litselection.c
- [che_new_autoschedule](HEURISTICS/che_new_autoschedule.md) - HEURISTICS/che_new_autoschedule.h, HEURISTICS/che_new_autoschedule.c
- [che_normsubst](HEURISTICS/che_normsubst.md) - HEURISTICS/che_normsubst.h, HEURISTICS/che_normsubst.c
- [che_orientweight](HEURISTICS/che_orientweight.md) - HEURISTICS/che_orientweight.h, HEURISTICS/che_orientweight.c
- [che_patterns](HEURISTICS/che_patterns.md) - HEURISTICS/che_patterns.h
- [che_prefixweight](HEURISTICS/che_prefixweight.md) - HEURISTICS/che_prefixweight.h, HEURISTICS/che_prefixweight.c
- [che_prio_funs](HEURISTICS/che_prio_funs.md) - HEURISTICS/che_prio_funs.h, HEURISTICS/che_prio_funs.c
- [che_proofcontrol](HEURISTICS/che_proofcontrol.md) - HEURISTICS/che_proofcontrol.h, HEURISTICS/che_proofcontrol.c
- [che_random](HEURISTICS/che_random.md) - HEURISTICS/che_random.h, HEURISTICS/che_random.c
- [che_rawspecfeatures](HEURISTICS/che_rawspecfeatures.md) - HEURISTICS/che_rawspecfeatures.h, HEURISTICS/che_rawspecfeatures.c
- [che_refinedweight](HEURISTICS/che_refinedweight.md) - HEURISTICS/che_refinedweight.h, HEURISTICS/che_refinedweight.c
- [che_simweight](HEURISTICS/che_simweight.md) - HEURISTICS/che_simweight.h, HEURISTICS/che_simweight.c
- [che_specsigfeatures](HEURISTICS/che_specsigfeatures.md) - HEURISTICS/che_specsigfeatures.h, HEURISTICS/che_specsigfeatures.c
- [che_strucweight](HEURISTICS/che_strucweight.md) - HEURISTICS/che_strucweight.h, HEURISTICS/che_strucweight.c
- [che_termweight](HEURISTICS/che_termweight.md) - HEURISTICS/che_termweight.h, HEURISTICS/che_termweight.c
- [che_termweights](HEURISTICS/che_termweights.md) - HEURISTICS/che_termweights.h, HEURISTICS/che_termweights.c
- [che_tfidfweight](HEURISTICS/che_tfidfweight.md) - HEURISTICS/che_tfidfweight.h, HEURISTICS/che_tfidfweight.c
- [che_to_autoselect](HEURISTICS/che_to_autoselect.md) - HEURISTICS/che_to_autoselect.h, HEURISTICS/che_to_autoselect.c
- [che_to_params](HEURISTICS/che_to_params.md) - HEURISTICS/che_to_params.h, HEURISTICS/che_to_params.c
- [che_to_precgen](HEURISTICS/che_to_precgen.md) - HEURISTICS/che_to_precgen.h, HEURISTICS/che_to_precgen.c
- [che_to_weightgen](HEURISTICS/che_to_weightgen.md) - HEURISTICS/che_to_weightgen.h, HEURISTICS/che_to_weightgen.c
- [che_treeweight](HEURISTICS/che_treeweight.md) - HEURISTICS/che_treeweight.h, HEURISTICS/che_treeweight.c
- [che_varweights](HEURISTICS/che_varweights.md) - HEURISTICS/che_varweights.h, HEURISTICS/che_varweights.c
- [che_wfcb](HEURISTICS/che_wfcb.md) - HEURISTICS/che_wfcb.h, HEURISTICS/che_wfcb.c
- [che_wfcbadmin](HEURISTICS/che_wfcbadmin.md) - HEURISTICS/che_wfcbadmin.h, HEURISTICS/che_wfcbadmin.c

### INOUT

- [cio_basicparser](INOUT/cio_basicparser.md) - INOUT/cio_basicparser.h, INOUT/cio_basicparser.c
- [cio_commandline](INOUT/cio_commandline.md) - INOUT/cio_commandline.h, INOUT/cio_commandline.c
- [cio_fileops](INOUT/cio_fileops.md) - INOUT/cio_fileops.h, INOUT/cio_fileops.c
- [cio_filevars](INOUT/cio_filevars.md) - INOUT/cio_filevars.h, INOUT/cio_filevars.c
- [cio_initio](INOUT/cio_initio.md) - INOUT/cio_initio.h, INOUT/cio_initio.c
- [cio_multiplexer](INOUT/cio_multiplexer.md) - INOUT/cio_multiplexer.h, INOUT/cio_multiplexer.c
- [cio_network](INOUT/cio_network.md) - INOUT/cio_network.h, INOUT/cio_network.c
- [cio_output](INOUT/cio_output.md) - INOUT/cio_output.h, INOUT/cio_output.c
- [cio_scanner](INOUT/cio_scanner.md) - INOUT/cio_scanner.h, INOUT/cio_scanner.c
- [cio_signals](INOUT/cio_signals.md) - INOUT/cio_signals.h, INOUT/cio_signals.c
- [cio_simplestuff](INOUT/cio_simplestuff.md) - INOUT/cio_simplestuff.h, INOUT/cio_simplestuff.c
- [cio_streams](INOUT/cio_streams.md) - INOUT/cio_streams.h, INOUT/cio_streams.c
- [cio_tempfile](INOUT/cio_tempfile.md) - INOUT/cio_tempfile.h, INOUT/cio_tempfile.c

### LEARN

- [cle_annotations](LEARN/cle_annotations.md) - LEARN/cle_annotations.h, LEARN/cle_annotations.c
- [cle_annoterms](LEARN/cle_annoterms.md) - LEARN/cle_annoterms.h, LEARN/cle_annoterms.c
- [cle_classification](LEARN/cle_classification.md) - LEARN/cle_classification.h, LEARN/cle_classification.c
- [cle_clauseenc](LEARN/cle_clauseenc.md) - LEARN/cle_clauseenc.h, LEARN/cle_clauseenc.c
- [cle_examplerep](LEARN/cle_examplerep.md) - LEARN/cle_examplerep.h, LEARN/cle_examplerep.c
- [cle_flatannoterms](LEARN/cle_flatannoterms.md) - LEARN/cle_flatannoterms.h, LEARN/cle_flatannoterms.c
- [cle_indexfunctions](LEARN/cle_indexfunctions.md) - LEARN/cle_indexfunctions.h, LEARN/cle_indexfunctions.c
- [cle_kbdesc](LEARN/cle_kbdesc.md) - LEARN/cle_kbdesc.h, LEARN/cle_kbdesc.c
- [cle_kbinsert](LEARN/cle_kbinsert.md) - LEARN/cle_kbinsert.h, LEARN/cle_kbinsert.c
- [cle_numfeatures](LEARN/cle_numfeatures.md) - LEARN/cle_numfeatures.h, LEARN/cle_numfeatures.c
- [cle_patterns](LEARN/cle_patterns.md) - LEARN/cle_patterns.h, LEARN/cle_patterns.c
- [cle_termtops](LEARN/cle_termtops.md) - LEARN/cle_termtops.h, LEARN/cle_termtops.c
- [cle_tsm](LEARN/cle_tsm.md) - LEARN/cle_tsm.h, LEARN/cle_tsm.c
- [cle_tsmio](LEARN/cle_tsmio.md) - LEARN/cle_tsmio.h, LEARN/cle_tsmio.c

### ORDERINGS

- [cto_cmpcache](ORDERINGS/cto_cmpcache.md) - ORDERINGS/cto_cmpcache.h, ORDERINGS/cto_cmpcache.c
- [cto_kbo](ORDERINGS/cto_kbo.md) - ORDERINGS/cto_kbo.h, ORDERINGS/cto_kbo.c
- [cto_kbolin](ORDERINGS/cto_kbolin.md) - ORDERINGS/cto_kbolin.h, ORDERINGS/cto_kbolin.c
- [cto_lpo](ORDERINGS/cto_lpo.md) - ORDERINGS/cto_lpo.h, ORDERINGS/cto_lpo.c
- [cto_lpo_debug](ORDERINGS/cto_lpo_debug.md) - ORDERINGS/cto_lpo_debug.h, ORDERINGS/cto_lpo_debug.c
- [cto_ocb](ORDERINGS/cto_ocb.md) - ORDERINGS/cto_ocb.h, ORDERINGS/cto_ocb.c
- [cto_orderings](ORDERINGS/cto_orderings.md) - ORDERINGS/cto_orderings.h, ORDERINGS/cto_orderings.c

### PCL2

- [pcl_analysis](PCL2/pcl_analysis.md) - PCL2/pcl_analysis.h, PCL2/pcl_analysis.c
- [pcl_expressions](PCL2/pcl_expressions.md) - PCL2/pcl_expressions.h, PCL2/pcl_expressions.c
- [pcl_idents](PCL2/pcl_idents.md) - PCL2/pcl_idents.h, PCL2/pcl_idents.c
- [pcl_lemmas](PCL2/pcl_lemmas.md) - PCL2/pcl_lemmas.h, PCL2/pcl_lemmas.c
- [pcl_miniclauses](PCL2/pcl_miniclauses.md) - PCL2/pcl_miniclauses.h, PCL2/pcl_miniclauses.c
- [pcl_miniprotocol](PCL2/pcl_miniprotocol.md) - PCL2/pcl_miniprotocol.h, PCL2/pcl_miniprotocol.c
- [pcl_ministeps](PCL2/pcl_ministeps.md) - PCL2/pcl_ministeps.h, PCL2/pcl_ministeps.c
- [pcl_positions](PCL2/pcl_positions.md) - PCL2/pcl_positions.h, PCL2/pcl_positions.c
- [pcl_proofcheck](PCL2/pcl_proofcheck.md) - PCL2/pcl_proofcheck.h, PCL2/pcl_proofcheck.c
- [pcl_propanalysis](PCL2/pcl_propanalysis.md) - PCL2/pcl_propanalysis.h, PCL2/pcl_propanalysis.c
- [pcl_protocol](PCL2/pcl_protocol.md) - PCL2/pcl_protocol.h, PCL2/pcl_protocol.c
- [pcl_steps](PCL2/pcl_steps.md) - PCL2/pcl_steps.h, PCL2/pcl_steps.c

### PROPOSITIONAL

- [cpr_dpll](PROPOSITIONAL/cpr_dpll.md) - PROPOSITIONAL/cpr_dpll.h, PROPOSITIONAL/cpr_dpll.c
- [cpr_dpllformula](PROPOSITIONAL/cpr_dpllformula.md) - PROPOSITIONAL/cpr_dpllformula.h, PROPOSITIONAL/cpr_dpllformula.c
- [cpr_propclauses](PROPOSITIONAL/cpr_propclauses.md) - PROPOSITIONAL/cpr_propclauses.h, PROPOSITIONAL/cpr_propclauses.c
- [cpr_propsig](PROPOSITIONAL/cpr_propsig.md) - PROPOSITIONAL/cpr_propsig.h, PROPOSITIONAL/cpr_propsig.c
- [cpr_varset](PROPOSITIONAL/cpr_varset.md) - PROPOSITIONAL/cpr_varset.h, PROPOSITIONAL/cpr_varset.c

### PROVER

- [checkproof](PROVER/checkproof.md) - PROVER/checkproof.c
- [classify_problem](PROVER/classify_problem.md) - PROVER/classify_problem.c
- [direct_examples](PROVER/direct_examples.md) - PROVER/direct_examples.c
- [e_axfilter](PROVER/e_axfilter.md) - PROVER/e_axfilter.c
- [e_client](PROVER/e_client.md) - PROVER/e_client.c
- [e_deduction_server](PROVER/e_deduction_server.md) - PROVER/e_deduction_server.c
- [e_gitcommit](PROVER/e_gitcommit.md) - PROVER/e_gitcommit.h
- [e_ltb_runner](PROVER/e_ltb_runner.md) - PROVER/e_ltb_runner.c
- [e_options](PROVER/e_options.md) - PROVER/e_options.h
- [e_server](PROVER/e_server.md) - PROVER/e_server.c
- [e_stratpar](PROVER/e_stratpar.md) - PROVER/e_stratpar.c
- [e_version](PROVER/e_version.md) - PROVER/e_version.h
- [edpll](PROVER/edpll.md) - PROVER/edpll.c
- [eground](PROVER/eground.md) - PROVER/eground.c
- [ekb_create](PROVER/ekb_create.md) - PROVER/ekb_create.c
- [ekb_delete](PROVER/ekb_delete.md) - PROVER/ekb_delete.c
- [ekb_ginsert](PROVER/ekb_ginsert.md) - PROVER/ekb_ginsert.c
- [ekb_insert](PROVER/ekb_insert.md) - PROVER/ekb_insert.c
- [enormalizer](PROVER/enormalizer.md) - PROVER/enormalizer.c
- [epatternize](PROVER/epatternize.md) - PROVER/epatternize.c
- [epclanalyse](PROVER/epclanalyse.md) - PROVER/epclanalyse.c
- [epclextract](PROVER/epclextract.md) - PROVER/epclextract.c
- [epcllemma](PROVER/epcllemma.md) - PROVER/epcllemma.c
- [eprover](PROVER/eprover.md) - PROVER/eprover.c
- [termprops](PROVER/termprops.md) - PROVER/termprops.c
- [tsm_classify](PROVER/tsm_classify.md) - PROVER/tsm_classify.c

### SIMPLE_APPS

- [ex_commandline](SIMPLE_APPS/ex_commandline.md) - SIMPLE_APPS/ex_commandline.c
- [term2dag](SIMPLE_APPS/term2dag.md) - SIMPLE_APPS/term2dag.c

### TERMS

- [cte_acterms](TERMS/cte_acterms.md) - TERMS/cte_acterms.h, TERMS/cte_acterms.c
- [cte_dbvars](TERMS/cte_dbvars.md) - TERMS/cte_dbvars.h, TERMS/cte_dbvars.c
- [cte_fixpoint_unif](TERMS/cte_fixpoint_unif.md) - TERMS/cte_fixpoint_unif.h, TERMS/cte_fixpoint_unif.c
- [cte_fp_index](TERMS/cte_fp_index.md) - TERMS/cte_fp_index.h, TERMS/cte_fp_index.c
- [cte_functypes](TERMS/cte_functypes.md) - TERMS/cte_functypes.h, TERMS/cte_functypes.c
- [cte_garbage_coll](TERMS/cte_garbage_coll.md) - TERMS/cte_garbage_coll.h, TERMS/cte_garbage_coll.c
- [cte_ho_bindings](TERMS/cte_ho_bindings.md) - TERMS/cte_ho_bindings.h, TERMS/cte_ho_bindings.c
- [cte_ho_csu](TERMS/cte_ho_csu.md) - TERMS/cte_ho_csu.h, TERMS/cte_ho_csu.c
- [cte_idx_fp](TERMS/cte_idx_fp.md) - TERMS/cte_idx_fp.h, TERMS/cte_idx_fp.c
- [cte_lambda](TERMS/cte_lambda.md) - TERMS/cte_lambda.h, TERMS/cte_lambda.c
- [cte_match_mgu_1-1](TERMS/cte_match_mgu_1-1.md) - TERMS/cte_match_mgu_1-1.h, TERMS/cte_match_mgu_1-1.c
- [cte_pattern_match_mgu](TERMS/cte_pattern_match_mgu.md) - TERMS/cte_pattern_match_mgu.h, TERMS/cte_pattern_match_mgu.c
- [cte_replace](TERMS/cte_replace.md) - TERMS/cte_replace.h, TERMS/cte_replace.c
- [cte_signature](TERMS/cte_signature.md) - TERMS/cte_signature.h, TERMS/cte_signature.c
- [cte_simplesorts](TERMS/cte_simplesorts.md) - TERMS/cte_simplesorts.h, TERMS/cte_simplesorts.c
- [cte_simpletypes](TERMS/cte_simpletypes.md) - TERMS/cte_simpletypes.h, TERMS/cte_simpletypes.c
- [cte_subst](TERMS/cte_subst.md) - TERMS/cte_subst.h, TERMS/cte_subst.c
- [cte_termbanks](TERMS/cte_termbanks.md) - TERMS/cte_termbanks.h, TERMS/cte_termbanks.c
- [cte_termcellstore](TERMS/cte_termcellstore.md) - TERMS/cte_termcellstore.h, TERMS/cte_termcellstore.c
- [cte_termcpos](TERMS/cte_termcpos.md) - TERMS/cte_termcpos.h, TERMS/cte_termcpos.c
- [cte_termfunc](TERMS/cte_termfunc.md) - TERMS/cte_termfunc.h, TERMS/cte_termfunc.c
- [cte_termpos](TERMS/cte_termpos.md) - TERMS/cte_termpos.h, TERMS/cte_termpos.c
- [cte_termtrees](TERMS/cte_termtrees.md) - TERMS/cte_termtrees.h, TERMS/cte_termtrees.c
- [cte_termtypes](TERMS/cte_termtypes.md) - TERMS/cte_termtypes.h, TERMS/cte_termtypes.c
- [cte_termvars](TERMS/cte_termvars.md) - TERMS/cte_termvars.h, TERMS/cte_termvars.c
- [cte_termweightext](TERMS/cte_termweightext.md) - TERMS/cte_termweightext.h, TERMS/cte_termweightext.c
- [cte_typebanks](TERMS/cte_typebanks.md) - TERMS/cte_typebanks.h, TERMS/cte_typebanks.c
- [cte_typecheck](TERMS/cte_typecheck.md) - TERMS/cte_typecheck.h, TERMS/cte_typecheck.c
- [cte_varhash](TERMS/cte_varhash.md) - TERMS/cte_varhash.h, TERMS/cte_varhash.c
- [cte_varsets](TERMS/cte_varsets.md) - TERMS/cte_varsets.h, TERMS/cte_varsets.c
<!-- END AUTO-GENERATED: c_source_docs -->








<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Notes

Manual source review is tracked in `review_status.md`; subsystem-level corrections and cross-cutting porting observations can be added here without being overwritten by regeneration.
<!-- END MANUAL REVIEW: c_source_docs -->
