pub mod axiomsorter;
pub mod bce;
#[cfg(feature = "cadical-static")]
// Allowed static C++ FFI boundary: cadical keeps every raw call and callback
// invariant behind a safe incremental service.
#[allow(unsafe_code)]
pub mod cadical;
pub mod clause;
pub mod clause_props;
pub mod clausecpos;
pub mod clausefunc;
pub mod clauseinfo;
pub mod clausepos;
pub mod clausepos_tree;
pub mod clausesets;
pub mod condensation;
pub mod context_sr;
pub mod derivation;
pub mod diseq_decomp;
pub mod eqn;
pub mod eqn_props;
pub mod eqnlist;
pub mod eqnresolution;
pub mod ext_index;
pub mod f_generality;
pub mod factor;
pub mod fcvindexing;
pub mod findex;
pub mod formulasets;
pub mod freqvectors;
pub mod g_lithash;
pub mod garbage_coll;
pub mod gd_transformation;
pub mod global_indices;
pub mod groundconstr;
pub mod grounding;
pub mod inferencedoc;
pub mod neweval;
pub mod overlap_index;
pub mod paramodulation;
pub mod pdtrees;
// Allowed external DLL/shared-library boundary: picosat keeps runtime loading
// and PicoSAT ABI calls behind a safe solver wrapper.
#[allow(unsafe_code)]
pub mod picosat;
pub mod pred_elim;
pub mod proofstate;
pub mod propclauses;
pub mod relevance;
pub mod rewrite;
pub mod satinterface;
pub mod satservice;
pub mod sine;
pub mod splitting;
pub mod subsumption;
pub mod subterm_index;
pub mod subterm_tree;
pub mod tautologies;
pub mod unfold_defs;
pub mod unit_simplify;
