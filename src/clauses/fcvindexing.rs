use crate::basics::error::{Diagnostic, ErrorCode};
use crate::basics::intmap::{IntMap, IntMapKey};
use crate::basics::simple_stuff::ProblemType;
use crate::clauses::clause::{
    clause_print_lop_format_string, clause_print_tptp_format_string, clause_tstp_string, Clause,
};
use crate::clauses::clausesets::ClauseSet;
use crate::clauses::freqvectors::{
    bill_features_collect_alloc, bill_plus_features_collect_alloc, fv_pack_clause,
    optimized_var_freq_vector_compute, FreqVector, FvCollect, FvCollectLayout, FvIndexType,
    FvOverflowSpec, FvPackedClause, PermVector, FVINDEX_MAX_FEATURES_DEFAULT,
    FVINDEX_SYMBOL_SLACK_DEFAULT,
};
use crate::clauses::subsumption::clause_subsume_order_sort_lits;
use crate::inout::scanner::IoFormat;
use crate::terms::signature::Signature;
use crate::terms::termbanks::TermBank;
use std::collections::BTreeMap;
use std::fmt::Write as _;

const FVINDEX_MEM: usize = 16;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FvIndexParams {
    cspec: FvCollect,
    use_perm_vectors: bool,
    eliminate_uninformative: bool,
    max_symbols: usize,
    symbol_slack: usize,
}

impl Default for FvIndexParams {
    fn default() -> Self {
        let mut cspec = FvCollect::new(FvCollectLayout::new(FvIndexType::AcFold, false, 0, 0));
        cspec.set_max_symbols(0);
        Self {
            cspec,
            use_perm_vectors: false,
            eliminate_uninformative: false,
            max_symbols: FVINDEX_MAX_FEATURES_DEFAULT,
            symbol_slack: FVINDEX_SYMBOL_SLACK_DEFAULT,
        }
    }
}

impl FvIndexParams {
    #[must_use]
    pub fn new(
        feature_type: FvIndexType,
        use_perm_vectors: bool,
        eliminate_uninformative: bool,
        max_symbols: usize,
        symbol_slack: usize,
    ) -> Self {
        let mut cspec = FvCollect::new(FvCollectLayout::new(feature_type, false, 0, 0));
        cspec.set_max_symbols(0);
        Self {
            cspec,
            use_perm_vectors,
            eliminate_uninformative,
            max_symbols,
            symbol_slack,
        }
    }

    #[must_use]
    pub const fn cspec(&self) -> &FvCollect {
        &self.cspec
    }

    #[must_use]
    pub const fn use_perm_vectors(&self) -> bool {
        self.use_perm_vectors
    }

    #[must_use]
    pub const fn eliminate_uninformative(&self) -> bool {
        self.eliminate_uninformative
    }

    #[must_use]
    pub const fn max_symbols(&self) -> usize {
        self.max_symbols
    }

    #[must_use]
    pub const fn symbol_slack(&self) -> usize {
        self.symbol_slack
    }

    pub const fn set_symbol_slack(&mut self, symbol_slack: usize) {
        self.symbol_slack = symbol_slack;
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FvIndexInitSpecs {
    original_symbols: usize,
    symbols: usize,
    cspec: FvCollect,
    def_store_cspec: FvCollect,
}

impl FvIndexInitSpecs {
    #[must_use]
    pub const fn original_symbols(&self) -> usize {
        self.original_symbols
    }

    #[must_use]
    pub const fn symbols(&self) -> usize {
        self.symbols
    }

    #[must_use]
    pub const fn cspec(&self) -> &FvCollect {
        &self.cspec
    }

    #[must_use]
    pub const fn def_store_cspec(&self) -> &FvCollect {
        &self.def_store_cspec
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct FvIndexInitAnchors {
    processed_non_units: Option<FvIndexAnchor>,
    processed_pos_rules: Option<FvIndexAnchor>,
    processed_pos_eqns: Option<FvIndexAnchor>,
    processed_neg_units: Option<FvIndexAnchor>,
    watchlist: Option<FvIndexAnchor>,
    def_store: FvIndexAnchor,
}

pub struct FvIndexInitTargetSets<'a> {
    pub processed_non_units: &'a mut ClauseSet,
    pub processed_pos_rules: &'a mut ClauseSet,
    pub processed_pos_eqns: &'a mut ClauseSet,
    pub processed_neg_units: &'a mut ClauseSet,
    pub watchlist: Option<&'a mut ClauseSet>,
    pub def_store: &'a mut ClauseSet,
}

impl FvIndexInitAnchors {
    #[must_use]
    pub fn processed_non_units(&self) -> Option<&FvIndexAnchor> {
        self.processed_non_units.as_ref()
    }

    #[must_use]
    pub fn processed_pos_rules(&self) -> Option<&FvIndexAnchor> {
        self.processed_pos_rules.as_ref()
    }

    #[must_use]
    pub fn processed_pos_eqns(&self) -> Option<&FvIndexAnchor> {
        self.processed_pos_eqns.as_ref()
    }

    #[must_use]
    pub fn processed_neg_units(&self) -> Option<&FvIndexAnchor> {
        self.processed_neg_units.as_ref()
    }

    #[must_use]
    pub fn watchlist(&self) -> Option<&FvIndexAnchor> {
        self.watchlist.as_ref()
    }

    #[must_use]
    pub const fn def_store(&self) -> &FvIndexAnchor {
        &self.def_store
    }

    pub fn install(self, targets: FvIndexInitTargetSets<'_>) {
        let FvIndexInitTargetSets {
            processed_non_units,
            processed_pos_rules,
            processed_pos_eqns,
            processed_neg_units,
            watchlist,
            def_store,
        } = targets;

        processed_non_units.set_fv_anchor(self.processed_non_units);
        processed_pos_rules.set_fv_anchor(self.processed_pos_rules);
        processed_pos_eqns.set_fv_anchor(self.processed_pos_eqns);
        processed_neg_units.set_fv_anchor(self.processed_neg_units);
        if let Some(watchlist) = watchlist {
            watchlist.set_fv_anchor(self.watchlist);
        }
        def_store.set_fv_anchor(Some(self.def_store));
    }
}

/// Computes the feature-vector collection specs built by C `fvi_param_init`.
///
/// # Errors
///
/// Returns a diagnostic if C-shaped symbol counts cannot be represented in the
/// Rust index layout.
pub fn fvi_param_init_specs(
    signature: &Signature,
    params: &FvIndexParams,
) -> Result<FvIndexInitSpecs, Diagnostic> {
    let original_symbols = usize_from_i64(signature.f_count(), "signature f_count")?;
    let symbols = original_symbols
        .saturating_add(params.symbol_slack)
        .min(params.max_symbols);
    let mut cspec = match params.cspec.features() {
        FvIndexType::BillFeatures => {
            bill_features_collect_alloc(signature, symbols_times_two_plus(symbols, 2)?)
        }
        FvIndexType::BillPlusFeatures => {
            bill_plus_features_collect_alloc(signature, symbols_times_two_plus(symbols, 4)?)
        }
        FvIndexType::AcFold => ac_fold_collect(symbols)?,
        FvIndexType::AcStagger => ac_stagger_collect(symbols)?,
        FvIndexType::CollectFeatures => collect_features_from_params(params, symbols),
        features => FvCollect::new(FvCollectLayout::new(features, false, 0, 0)),
    };
    cspec.set_max_symbols(symbols);
    let def_store_cspec = ac_fold_collect(symbols)?;

    Ok(FvIndexInitSpecs {
        original_symbols,
        symbols,
        cspec,
        def_store_cspec,
    })
}

/// Builds the empty feature-vector anchors installed by C `fvi_param_init`.
///
/// The caller still owns attaching these anchors to concrete proof-state clause
/// sets. The active processed/watchlist anchors all receive copies of the same
/// permutation vector computed from the active spec; the definition-store anchor
/// receives the same effective vector with its separate AC-fold collection spec.
#[must_use]
pub fn fvi_param_init_anchors(
    axioms: &ClauseSet,
    specs: &FvIndexInitSpecs,
    params: &FvIndexParams,
    include_watchlist: bool,
) -> FvIndexInitAnchors {
    let perm = axioms.perm_vector_compute(specs.cspec(), params.eliminate_uninformative());
    let active_enabled = params.cspec().features() != FvIndexType::NoFeatures;

    FvIndexInitAnchors {
        processed_non_units: optional_anchor(active_enabled, specs.cspec(), perm.as_ref()),
        processed_pos_rules: optional_anchor(active_enabled, specs.cspec(), perm.as_ref()),
        processed_pos_eqns: optional_anchor(active_enabled, specs.cspec(), perm.as_ref()),
        processed_neg_units: optional_anchor(active_enabled, specs.cspec(), perm.as_ref()),
        watchlist: optional_anchor(
            active_enabled && include_watchlist,
            specs.cspec(),
            perm.as_ref(),
        ),
        def_store: FvIndexAnchor::new(specs.def_store_cspec().clone(), perm),
    }
}

fn optional_anchor(
    enabled: bool,
    cspec: &FvCollect,
    perm: Option<&PermVector>,
) -> Option<FvIndexAnchor> {
    enabled.then(|| FvIndexAnchor::new(cspec.clone(), perm.cloned()))
}

fn ac_fold_collect(symbols: usize) -> Result<FvCollect, Diagnostic> {
    let symbols = usize_to_i64(symbols, "FV index symbols")?;
    let result_len = symbols_times_two_plus_i64(symbols, 2)?;
    let mut layout = FvCollectLayout::new(FvIndexType::CollectFeatures, true, 0, result_len);
    layout.pos_count = FvOverflowSpec::new(2, 0, symbols);
    layout.neg_count =
        FvOverflowSpec::new(checked_i64_add(symbols, 2, "FV index symbols")?, 0, symbols);
    Ok(FvCollect::new(layout))
}

fn ac_stagger_collect(symbols: usize) -> Result<FvCollect, Diagnostic> {
    let symbols = usize_to_i64(symbols, "FV index symbols")?;
    let double_symbols = checked_i64_mul(symbols, 2, "FV index symbols")?;
    let result_len = checked_i64_add(double_symbols, 2, "FV index result length")?;
    let mut layout = FvCollectLayout::new(
        FvIndexType::CollectFeatures,
        true,
        0,
        i64_to_usize(result_len, "FV index result length")?,
    );
    layout.pos_count = FvOverflowSpec::new(2, 0, double_symbols);
    layout.neg_count = FvOverflowSpec::new(
        2,
        checked_i64_add(symbols, 2, "FV index symbols")?,
        double_symbols,
    );
    Ok(FvCollect::new(layout))
}

fn collect_features_from_params(params: &FvIndexParams, symbols: usize) -> FvCollect {
    let mut layout = FvCollectLayout::new(
        params.cspec.features(),
        params.cspec.use_litcount(),
        params.cspec.assembly_vector().len(),
        symbols,
    );
    layout.pos_count = params.cspec.pos_count_overflow();
    layout.neg_count = params.cspec.neg_count_overflow();
    layout.pos_depth = params.cspec.pos_depth_overflow();
    layout.neg_depth = params.cspec.neg_depth_overflow();
    FvCollect::new(layout)
}

fn symbols_times_two_plus(symbols: usize, addend: usize) -> Result<usize, Diagnostic> {
    symbols
        .checked_mul(2)
        .and_then(|value| value.checked_add(addend))
        .ok_or_else(|| fv_index_error("FV index symbol-derived vector length overflows usize"))
}

fn symbols_times_two_plus_i64(symbols: i64, addend: i64) -> Result<usize, Diagnostic> {
    let doubled = checked_i64_mul(symbols, 2, "FV index symbols")?;
    i64_to_usize(
        checked_i64_add(doubled, addend, "FV index result length")?,
        "FV index result length",
    )
}

fn usize_from_i64(value: i64, context: &str) -> Result<usize, Diagnostic> {
    usize::try_from(value).map_err(|_| fv_index_error(&format!("{context} must fit usize")))
}

fn usize_to_i64(value: usize, context: &str) -> Result<i64, Diagnostic> {
    i64::try_from(value).map_err(|_| fv_index_error(&format!("{context} must fit i64")))
}

fn i64_to_usize(value: i64, context: &str) -> Result<usize, Diagnostic> {
    usize::try_from(value).map_err(|_| fv_index_error(&format!("{context} must fit usize")))
}

fn checked_i64_add(lhs: i64, rhs: i64, context: &str) -> Result<i64, Diagnostic> {
    lhs.checked_add(rhs)
        .ok_or_else(|| fv_index_error(&format!("{context} addition overflows i64")))
}

fn checked_i64_mul(lhs: i64, rhs: i64, context: &str) -> Result<i64, Diagnostic> {
    lhs.checked_mul(rhs)
        .ok_or_else(|| fv_index_error(&format!("{context} multiplication overflows i64")))
}

fn fv_index_error(message: &str) -> Diagnostic {
    Diagnostic::new(ErrorCode::OTHER_ERROR, message.to_owned())
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct FvIndex {
    final_node: bool,
    clause_count: i64,
    successors: BTreeMap<i64, FvIndex>,
    successor_storage: Option<IntMap<()>>,
    clauses: BTreeMap<i64, Clause>,
}

impl FvIndex {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            final_node: false,
            clause_count: 0,
            successors: BTreeMap::new(),
            successor_storage: None,
            clauses: BTreeMap::new(),
        }
    }

    #[must_use]
    pub const fn is_final(&self) -> bool {
        self.final_node
    }

    #[must_use]
    pub const fn clause_count(&self) -> i64 {
        self.clause_count
    }

    #[must_use]
    pub const fn clauses(&self) -> &BTreeMap<i64, Clause> {
        &self.clauses
    }

    #[must_use]
    pub const fn successors(&self) -> &BTreeMap<i64, FvIndex> {
        &self.successors
    }

    /// Returns the successor for `key` only if its subtree is non-empty.
    ///
    /// # Panics
    ///
    /// Panics if called on a final node, matching the C assertion.
    #[must_use]
    pub fn get_next_non_empty_node(&self, key: i64) -> Option<&Self> {
        assert!(!self.final_node, "final FV-index nodes have no successors");
        self.successors
            .get(&key)
            .filter(|successor| successor.clause_count != 0)
    }

    /// Counts nodes using the C `FVIndexCountNodes` flags.
    ///
    /// # Panics
    ///
    /// Panics if a final node has a zero/nonzero clause count that disagrees
    /// with whether the leaf stores any clauses, matching the C `EQUIV`
    /// assertion.
    #[must_use]
    pub fn count_nodes(&self, leaves: bool, empty: bool) -> i64 {
        let mut result = 0;
        if self.final_node {
            if !empty || self.clauses.is_empty() {
                result += 1;
            }
            assert_eq!(
                self.clause_count != 0,
                !self.clauses.is_empty(),
                "final FV-index node count must match stored-clause presence"
            );
        } else {
            if !(empty || leaves) {
                result += 1;
            }
            for successor in self.successors.values() {
                result += successor.count_nodes(leaves, empty);
            }
        }
        result
    }

    #[must_use]
    pub fn debug_print_string(&self) -> String {
        let mut output = String::new();
        self.write_debug(&mut output, 0);
        output
    }

    #[must_use]
    pub fn print_lop_string(&self, bank: &TermBank, full_terms: bool) -> String {
        self.print_string_with_clause_renderer(|clause| {
            clause_print_lop_format_string(bank, clause, full_terms)
        })
    }

    /// Returns the C `FVIndexPrint` tree shape with explicit `ClausePrint` dispatch.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if TSTP rendering rejects a stored clause.
    pub fn print_format_string(
        &self,
        bank: &TermBank,
        full_terms: bool,
        output_format: IoFormat,
        problem_type: ProblemType,
    ) -> Result<String, Diagnostic> {
        self.print_string_with_result_clause_renderer(|clause| {
            fv_index_clause_rendered_string(bank, clause, full_terms, output_format, problem_type)
        })
    }

    #[must_use]
    pub fn print_string_with_clause_renderer<R>(&self, mut render_clause: R) -> String
    where
        R: FnMut(&Clause) -> String,
    {
        let mut output = "* ROOT *\n".to_owned();
        self.write_print(&mut output, 0, &mut render_clause);
        output
    }

    fn print_string_with_result_clause_renderer<R>(
        &self,
        mut render_clause: R,
    ) -> Result<String, Diagnostic>
    where
        R: FnMut(&Clause) -> Result<String, Diagnostic>,
    {
        let mut output = "* ROOT *\n".to_owned();
        self.write_print_result(&mut output, 0, &mut render_clause)?;
        Ok(output)
    }

    fn insert_vector_clause(
        &mut self,
        vector: &FreqVector,
        clause_identity: i64,
        clause: Clause,
    ) -> FvIndexInsertResult {
        let mut node = self;
        let mut storage_delta = 0_i128;
        node.clause_count += 1;
        for value in vector.as_slice() {
            assert!(
                !node.final_node,
                "final FV-index nodes cannot have successors"
            );
            if !node.successors.contains_key(value) {
                storage_delta += node.insert_empty_successor_storage_delta(*value);
                node.successors.insert(*value, Self::new());
            }
            node = node
                .successors
                .get_mut(value)
                .expect("FV-index successor missing after insertion");
            node.clause_count += 1;
        }
        node.final_node = true;
        FvIndexInsertResult {
            inserted_clause: node.clauses.insert(clause_identity, clause).is_none(),
            storage_delta,
        }
    }

    fn insert_empty_successor_storage_delta(&mut self, value: i64) -> i128 {
        assert!(
            value >= 0,
            "FV-index successor keys must be nonnegative C long values"
        );
        let key =
            IntMapKey::try_from(value).expect("FV-index successor key must fit IntMapKey/isize");
        let before = self
            .successor_storage
            .as_ref()
            .map_or(0, IntMap::constant_mem_storage_estimate);
        self.successor_storage
            .get_or_insert_with(IntMap::new)
            .assign(key, ());
        let after = self
            .successor_storage
            .as_ref()
            .map_or(0, IntMap::constant_mem_storage_estimate);
        i128::try_from(after).unwrap_or(i128::MAX) - i128::try_from(before).unwrap_or(i128::MAX)
            + i128::try_from(FVINDEX_MEM).unwrap_or(i128::MAX)
    }

    fn delete_vector_clause(&mut self, vector: &FreqVector, clause: &Clause) -> bool {
        let mut node = self;
        node.clause_count -= 1;
        for value in vector.as_slice() {
            assert!(
                !node.final_node,
                "final FV-index nodes cannot have successors"
            );
            let Some(next) = node.successors.get_mut(value) else {
                return false;
            };
            next.clause_count -= 1;
            node = next;
        }
        node.clauses.remove(&fv_clause_key(clause)).is_some()
    }

    fn write_debug(&self, output: &mut String, level: usize) {
        if self.final_node {
            for clause in self.clauses.values() {
                for _ in 0..=level {
                    output.push_str("--");
                }
                let _ = writeln!(output, "clause#{}", clause.ident());
            }
        } else {
            for (key, successor) in &self.successors {
                for _ in 0..level {
                    output.push_str("--");
                }
                let _ = writeln!(output, "Alternative {key}: ");
                successor.write_debug(output, level + 1);
            }
        }
    }

    fn write_print<R>(&self, output: &mut String, level: usize, render_clause: &mut R)
    where
        R: FnMut(&Clause) -> String,
    {
        if self.final_node {
            for clause in self.clauses.values() {
                for _ in 0..=level {
                    output.push_str("--");
                }
                output.push_str(&render_clause(clause));
                output.push_str(" \n");
            }
        } else {
            for (key, successor) in &self.successors {
                for _ in 0..level {
                    output.push_str("--");
                }
                let _ = writeln!(output, "Alternative {key}: ");
                successor.write_print(output, level + 1, render_clause);
            }
        }
    }

    fn write_print_result<R>(
        &self,
        output: &mut String,
        level: usize,
        render_clause: &mut R,
    ) -> Result<(), Diagnostic>
    where
        R: FnMut(&Clause) -> Result<String, Diagnostic>,
    {
        if self.final_node {
            for clause in self.clauses.values() {
                for _ in 0..=level {
                    output.push_str("--");
                }
                output.push_str(&render_clause(clause)?);
                output.push_str(" \n");
            }
        } else {
            for (key, successor) in &self.successors {
                for _ in 0..level {
                    output.push_str("--");
                }
                let _ = writeln!(output, "Alternative {key}: ");
                successor.write_print_result(output, level + 1, render_clause)?;
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct FvIndexInsertResult {
    inserted_clause: bool,
    storage_delta: i128,
}

fn fv_index_clause_rendered_string(
    bank: &TermBank,
    clause: &Clause,
    full_terms: bool,
    output_format: IoFormat,
    problem_type: ProblemType,
) -> Result<String, Diagnostic> {
    match output_format {
        IoFormat::Tptp => Ok(clause_print_tptp_format_string(bank, clause)),
        IoFormat::Tstp => clause_tstp_string(bank, clause, full_terms, true, problem_type),
        IoFormat::Lop | IoFormat::Auto => {
            Ok(clause_print_lop_format_string(bank, clause, full_terms))
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct FvIndexAnchor {
    cspec: FvCollect,
    perm_vector: Option<PermVector>,
    index: FvIndex,
    storage: usize,
}

impl FvIndexAnchor {
    #[must_use]
    pub fn new(cspec: FvCollect, perm_vector: Option<PermVector>) -> Self {
        Self {
            cspec,
            perm_vector,
            index: FvIndex::new(),
            storage: 0,
        }
    }

    #[must_use]
    pub const fn cspec(&self) -> &FvCollect {
        &self.cspec
    }

    #[must_use]
    pub const fn perm_vector(&self) -> Option<&PermVector> {
        self.perm_vector.as_ref()
    }

    #[must_use]
    pub const fn index(&self) -> &FvIndex {
        &self.index
    }

    #[must_use]
    pub const fn storage_estimate(&self) -> usize {
        self.storage
    }

    /// Inserts a packed clause into the feature-vector index.
    ///
    /// # Panics
    ///
    /// Panics if `packed` does not contain a vector, or if an existing final
    /// node is encountered before all vector coordinates are consumed.
    pub fn insert(&mut self, packed: &mut FvPackedClause, bank: &TermBank) -> bool {
        let _timer = crate::basics::perf_counters::start(
            crate::basics::perf_counters::PerfCounter::FvIndexTimer,
        );
        let vector = packed
            .vector()
            .expect("FV-index insertion requires a packed frequency vector")
            .clone();
        clause_subsume_order_sort_lits(packed.clause_mut(), bank);
        let clause_identity = fv_clause_key(packed.clause());
        let result =
            self.index
                .insert_vector_clause(&vector, clause_identity, packed.clause().clone());
        self.apply_storage_delta(result.storage_delta);
        result.inserted_clause
    }

    fn apply_storage_delta(&mut self, delta: i128) {
        if delta >= 0 {
            self.storage = self
                .storage
                .checked_add(usize::try_from(delta).unwrap_or(usize::MAX))
                .expect("FV-index storage estimate overflow");
        } else {
            self.storage = self
                .storage
                .checked_sub(usize::try_from(-delta).unwrap_or(usize::MAX))
                .expect("FV-index storage estimate underflow");
        }
    }

    /// Deletes a clause from the feature-vector index.
    ///
    /// This preserves the C behavior of leaving empty leaves and interior nodes
    /// in place after deletion.
    ///
    /// # Panics
    ///
    /// Panics if an existing final node is encountered before all vector
    /// coordinates are consumed.
    pub fn delete(&mut self, clause: &Clause) -> bool {
        let vector =
            optimized_var_freq_vector_compute(clause, self.perm_vector.as_ref(), &self.cspec);
        let _timer = crate::basics::perf_counters::start(
            crate::basics::perf_counters::PerfCounter::FvIndexTimer,
        );
        self.index.delete_vector_clause(&vector, clause)
    }

    #[must_use]
    pub fn count_nodes(&self, leaves: bool, empty: bool) -> i64 {
        self.index.count_nodes(leaves, empty)
    }
}

#[must_use]
pub fn fv_index_storage(index: Option<&FvIndexAnchor>) -> usize {
    index.map_or(0, FvIndexAnchor::storage_estimate)
}

#[must_use]
pub fn fv_index_pack_clause(clause: Clause, anchor: Option<&FvIndexAnchor>) -> FvPackedClause {
    match anchor {
        Some(anchor) => fv_pack_clause(clause, anchor.perm_vector(), Some(anchor.cspec())),
        None => fv_pack_clause(clause, None, None),
    }
}

fn fv_clause_key(clause: &Clause) -> i64 {
    clause.ident()
}

#[cfg(test)]
mod tests {
    use crate::basics::intmap::{IntMap, IntMapType, INTMAPCELL_MEM};

    use super::{
        fv_index_pack_clause, fv_index_storage, fvi_param_init_anchors, fvi_param_init_specs,
        FvIndex, FvIndexAnchor, FvIndexInitTargetSets, FvIndexParams, FVINDEX_MEM,
    };
    use crate::basics::simple_stuff::ProblemType;
    use crate::clauses::clause::Clause;
    use crate::clauses::clausesets::ClauseSet;
    use crate::clauses::eqn::Eqn;
    use crate::clauses::eqnlist::EqnList;
    use crate::clauses::freqvectors::{
        FreqVector, FvCollect, FvCollectLayout, FvIndexType, FvOverflowSpec,
        FVINDEX_MAX_FEATURES_DEFAULT,
    };
    use crate::inout::scanner::IoFormat;
    use crate::terms::signature::Signature;
    use crate::terms::simpletypes::alloc_arrow_type;
    use crate::terms::termbanks::TermBank;
    use crate::terms::termtypes::{DerefType, Term};
    use crate::terms::typebanks::TypeBank;

    fn test_bank() -> TermBank {
        let mut signature = Signature::new(TypeBank::new());
        signature.insert_internal_codes().unwrap();
        TermBank::new(signature).unwrap()
    }

    fn typed_const(bank: &mut TermBank, name: &str) -> Term {
        let type_ = bank.signature().type_bank().default_type();
        let f_code = bank.signature_mut().insert_id(name, 0, false);
        if bank.signature().get_type(f_code).is_none() {
            bank.signature_mut()
                .declare_final_type(f_code, type_.clone())
                .unwrap();
        }
        bank.create_const_term(f_code).unwrap()
    }

    fn typed_unary(bank: &mut TermBank, name: &str, arg: &Term) -> Term {
        let type_ = bank.signature().type_bank().default_type();
        let f_code = bank.signature_mut().insert_id(name, 1, false);
        if bank.signature().get_type(f_code).is_none() {
            bank.signature_mut()
                .declare_final_type(f_code, alloc_arrow_type(vec![type_.clone(), type_]))
                .unwrap();
        }
        let term = Term::top_alloc(f_code, 1);
        term.set_type(Some(bank.signature().type_bank().default_type()));
        term.set_argument(0, arg.clone());
        bank.insert(&term, DerefType::Never).unwrap()
    }

    fn literal(bank: &mut TermBank, left: &Term, right: &Term, positive: bool) -> Eqn {
        Eqn::alloc(left.clone(), right.clone(), bank, positive).unwrap()
    }

    fn clause_from(literals: Vec<Eqn>, ident: i64) -> Clause {
        let mut clause = Clause::alloc(EqnList::from_vec(literals));
        clause.set_ident(ident);
        clause.set_weight(clause.standard_weight());
        clause
    }

    fn ac_anchor(max_symbols: usize) -> FvIndexAnchor {
        let mut cspec = FvCollect::new(FvCollectLayout::new(FvIndexType::AcFeatures, false, 0, 0));
        cspec.set_max_symbols(max_symbols);
        FvIndexAnchor::new(cspec, None)
    }

    fn two_clause_axioms(bank: &mut TermBank) -> ClauseSet {
        let first = typed_const(bank, "fvi_anchor_a");
        let second = typed_const(bank, "fvi_anchor_b");
        let third = typed_const(bank, "fvi_anchor_c");
        ClauseSet::from_clauses([
            clause_from(vec![literal(bank, &first, &second, true)], 100),
            clause_from(vec![literal(bank, &second, &third, false)], 101),
        ])
    }

    #[test]
    fn default_parameters_match_c_shape() {
        let params = FvIndexParams::default();
        assert_eq!(params.cspec().features(), FvIndexType::AcFold);
        assert!(!params.use_perm_vectors());
        assert!(!params.eliminate_uninformative());
        assert_eq!(params.max_symbols(), 17);
        assert_eq!(params.symbol_slack(), 0);
        assert_eq!(params.cspec().max_symbols(), 0);
    }

    #[test]
    fn fvi_param_init_specs_builds_ac_fold_and_def_store_shapes() {
        let bank = test_bank();
        let params = FvIndexParams::new(FvIndexType::AcFold, false, false, 5, 3);

        let specs =
            fvi_param_init_specs(bank.signature(), &params).unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(
            specs.original_symbols(),
            usize::try_from(bank.signature().f_count()).unwrap()
        );
        assert_eq!(specs.symbols(), 5);
        assert_eq!(specs.cspec().features(), FvIndexType::CollectFeatures);
        assert!(specs.cspec().use_litcount());
        assert_eq!(specs.cspec().result_len(), 12);
        assert_eq!(specs.cspec().max_symbols(), 5);
        assert_eq!(
            specs.cspec().pos_count_overflow(),
            FvOverflowSpec::new(2, 0, 5)
        );
        assert_eq!(
            specs.cspec().neg_count_overflow(),
            FvOverflowSpec::new(7, 0, 5)
        );
        assert_eq!(
            specs.def_store_cspec().max_symbols(),
            FVINDEX_MAX_FEATURES_DEFAULT
        );
        assert_eq!(specs.def_store_cspec().result_len(), 12);
    }

    #[test]
    fn fvi_param_init_specs_builds_ac_stagger_shape() {
        let bank = test_bank();
        let params = FvIndexParams::new(FvIndexType::AcStagger, false, false, 4, 0);

        let specs =
            fvi_param_init_specs(bank.signature(), &params).unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(specs.symbols(), 4);
        assert_eq!(specs.cspec().result_len(), 10);
        assert_eq!(
            specs.cspec().pos_count_overflow(),
            FvOverflowSpec::new(2, 0, 8)
        );
        assert_eq!(
            specs.cspec().neg_count_overflow(),
            FvOverflowSpec::new(2, 6, 8)
        );
    }

    #[test]
    fn fvi_param_init_specs_preserves_no_feature_request_with_symbol_cap() {
        let bank = test_bank();
        let original = usize::try_from(bank.signature().f_count()).unwrap();
        let params = FvIndexParams::new(FvIndexType::NoFeatures, false, false, original + 3, 2);

        let specs =
            fvi_param_init_specs(bank.signature(), &params).unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(specs.symbols(), original + 2);
        assert_eq!(specs.cspec().features(), FvIndexType::NoFeatures);
        assert_eq!(specs.cspec().result_len(), 0);
        assert_eq!(specs.cspec().max_symbols(), original + 2);
    }

    #[test]
    fn fvi_param_init_anchors_build_processed_watchlist_and_def_store_anchors() {
        let mut bank = test_bank();
        let axioms = two_clause_axioms(&mut bank);
        let params = FvIndexParams::new(FvIndexType::AcFold, false, true, 9, 1);
        let specs =
            fvi_param_init_specs(bank.signature(), &params).unwrap_or_else(|err| panic!("{err}"));
        let expected_perm = axioms.perm_vector_compute(specs.cspec(), true);

        let anchors = fvi_param_init_anchors(&axioms, &specs, &params, true);

        for anchor in [
            anchors.processed_non_units(),
            anchors.processed_pos_rules(),
            anchors.processed_pos_eqns(),
            anchors.processed_neg_units(),
            anchors.watchlist(),
        ] {
            let anchor = anchor.unwrap_or_else(|| panic!("active FV anchor should be installed"));
            assert_eq!(anchor.cspec(), specs.cspec());
            assert_eq!(anchor.perm_vector(), expected_perm.as_ref());
            assert_eq!(anchor.index().clause_count(), 0);
        }
        assert_eq!(anchors.def_store().cspec(), specs.def_store_cspec());
        assert_eq!(anchors.def_store().perm_vector(), expected_perm.as_ref());
        assert_eq!(anchors.def_store().index().clause_count(), 0);
    }

    #[test]
    fn fvi_param_init_anchors_skip_processed_sets_for_no_features() {
        let mut bank = test_bank();
        let axioms = two_clause_axioms(&mut bank);
        let params = FvIndexParams::new(FvIndexType::NoFeatures, false, false, 9, 1);
        let specs =
            fvi_param_init_specs(bank.signature(), &params).unwrap_or_else(|err| panic!("{err}"));

        let anchors = fvi_param_init_anchors(&axioms, &specs, &params, true);

        assert!(anchors.processed_non_units().is_none());
        assert!(anchors.processed_pos_rules().is_none());
        assert!(anchors.processed_pos_eqns().is_none());
        assert!(anchors.processed_neg_units().is_none());
        assert!(anchors.watchlist().is_none());
        assert_eq!(anchors.def_store().cspec(), specs.def_store_cspec());
        assert!(anchors.def_store().perm_vector().is_none());
    }

    #[test]
    fn fvi_param_init_anchors_omit_watchlist_when_state_has_none() {
        let mut bank = test_bank();
        let axioms = two_clause_axioms(&mut bank);
        let params = FvIndexParams::new(FvIndexType::AcFold, true, false, 9, 1);
        let specs =
            fvi_param_init_specs(bank.signature(), &params).unwrap_or_else(|err| panic!("{err}"));

        let anchors = fvi_param_init_anchors(&axioms, &specs, &params, false);

        assert!(anchors.processed_non_units().is_some());
        assert!(anchors.processed_pos_rules().is_some());
        assert!(anchors.processed_pos_eqns().is_some());
        assert!(anchors.processed_neg_units().is_some());
        assert!(anchors.watchlist().is_none());
    }

    #[test]
    fn fvi_param_init_anchors_install_into_clause_set_targets() {
        let mut bank = test_bank();
        let axioms = two_clause_axioms(&mut bank);
        let params = FvIndexParams::new(FvIndexType::AcFold, false, true, 9, 1);
        let specs =
            fvi_param_init_specs(bank.signature(), &params).unwrap_or_else(|err| panic!("{err}"));
        let mut non_units = ClauseSet::new();
        let mut rules = ClauseSet::new();
        let mut equations = ClauseSet::new();
        let mut negative_units = ClauseSet::new();
        let mut watch = ClauseSet::new();
        let mut defs = ClauseSet::new();

        fvi_param_init_anchors(&axioms, &specs, &params, true).install(FvIndexInitTargetSets {
            processed_non_units: &mut non_units,
            processed_pos_rules: &mut rules,
            processed_pos_eqns: &mut equations,
            processed_neg_units: &mut negative_units,
            watchlist: Some(&mut watch),
            def_store: &mut defs,
        });

        for anchor in [
            non_units.fv_anchor(),
            rules.fv_anchor(),
            equations.fv_anchor(),
            negative_units.fv_anchor(),
            watch.fv_anchor(),
        ] {
            let anchor = anchor.unwrap_or_else(|| panic!("active FV anchor should be installed"));
            assert_eq!(anchor.cspec(), specs.cspec());
            assert_eq!(anchor.index().clause_count(), 0);
        }
        assert_eq!(defs.fv_anchor().unwrap().cspec(), specs.def_store_cspec());
    }

    #[test]
    fn fvi_param_init_anchors_install_skips_processed_targets_for_no_features() {
        let mut bank = test_bank();
        let axioms = two_clause_axioms(&mut bank);
        let params = FvIndexParams::new(FvIndexType::NoFeatures, false, false, 9, 1);
        let specs =
            fvi_param_init_specs(bank.signature(), &params).unwrap_or_else(|err| panic!("{err}"));
        let mut non_units = ClauseSet::new();
        let mut rules = ClauseSet::new();
        let mut equations = ClauseSet::new();
        let mut negative_units = ClauseSet::new();
        let mut watch = ClauseSet::new();
        let mut defs = ClauseSet::new();

        fvi_param_init_anchors(&axioms, &specs, &params, true).install(FvIndexInitTargetSets {
            processed_non_units: &mut non_units,
            processed_pos_rules: &mut rules,
            processed_pos_eqns: &mut equations,
            processed_neg_units: &mut negative_units,
            watchlist: Some(&mut watch),
            def_store: &mut defs,
        });

        assert!(non_units.fv_anchor().is_none());
        assert!(rules.fv_anchor().is_none());
        assert!(equations.fv_anchor().is_none());
        assert!(negative_units.fv_anchor().is_none());
        assert!(watch.fv_anchor().is_none());
        assert_eq!(defs.fv_anchor().unwrap().cspec(), specs.def_store_cspec());
    }

    #[test]
    fn pack_clause_uses_anchor_vector_when_available() {
        let mut bank = test_bank();
        let first = typed_const(&mut bank, "a");
        let second = typed_const(&mut bank, "b");
        let clause = clause_from(vec![literal(&mut bank, &first, &second, true)], 10);
        let anchor = ac_anchor(usize::try_from(second.f_code()).unwrap() + 1);

        let dummy = fv_index_pack_clause(clause.clone(), None);
        assert!(dummy.vector().is_none());

        let packed = fv_index_pack_clause(clause, Some(&anchor));
        assert!(packed.vector().is_some());
    }

    #[test]
    fn insert_tracks_counts_and_non_empty_successors() {
        let mut bank = test_bank();
        let first = typed_const(&mut bank, "a");
        let second = typed_const(&mut bank, "b");
        let f_of_first = typed_unary(&mut bank, "f", &first);
        let clause = clause_from(vec![literal(&mut bank, &f_of_first, &second, true)], 20);
        let mut anchor = ac_anchor(usize::try_from(f_of_first.f_code()).unwrap() + 1);
        let mut packed = fv_index_pack_clause(clause, Some(&anchor));
        let first_value = packed.vector().unwrap().as_slice()[0];
        let vector_len = i64::try_from(packed.vector().unwrap().len()).unwrap();

        assert!(anchor.insert(&mut packed, &bank));
        assert_eq!(
            fv_index_storage(Some(&anchor)),
            usize::try_from(vector_len).unwrap() * (FVINDEX_MEM + INTMAPCELL_MEM)
        );
        assert_eq!(fv_index_storage(None), 0);
        assert_eq!(anchor.index().clause_count(), 1);
        assert_eq!(anchor.count_nodes(true, false), 1);
        assert_eq!(anchor.count_nodes(false, false), vector_len + 1);
        assert!(anchor
            .index()
            .get_next_non_empty_node(first_value)
            .is_some());
    }

    #[test]
    fn delete_leaves_empty_leaf_and_suppresses_non_empty_lookup() {
        let mut bank = test_bank();
        let first = typed_const(&mut bank, "a");
        let second = typed_const(&mut bank, "b");
        let clause = clause_from(vec![literal(&mut bank, &first, &second, true)], 30);
        let mut anchor = ac_anchor(usize::try_from(second.f_code()).unwrap() + 1);
        let mut packed = fv_index_pack_clause(clause, Some(&anchor));
        let first_value = packed.vector().unwrap().as_slice()[0];

        assert!(anchor.insert(&mut packed, &bank));
        let storage_after_insert = fv_index_storage(Some(&anchor));
        assert!(anchor.delete(packed.clause()));
        assert_eq!(fv_index_storage(Some(&anchor)), storage_after_insert);
        assert_eq!(anchor.index().clause_count(), 0);
        assert_eq!(anchor.count_nodes(true, true), 1);
        assert!(anchor
            .index()
            .get_next_non_empty_node(first_value)
            .is_none());
    }

    #[test]
    fn delete_uses_clause_identity_after_value_moves() {
        let mut bank = test_bank();
        let first = typed_const(&mut bank, "a");
        let second = typed_const(&mut bank, "b");
        let clause = clause_from(vec![literal(&mut bank, &first, &second, true)], 31);
        let mut anchor = ac_anchor(usize::try_from(second.f_code()).unwrap() + 1);
        let mut packed = fv_index_pack_clause(clause, Some(&anchor));
        let moved_clause = packed.clause().clone();

        assert!(anchor.insert(&mut packed, &bank));
        assert!(anchor.delete(&moved_clause));
        assert_eq!(anchor.index().clause_count(), 0);
        assert_eq!(anchor.count_nodes(true, true), 1);
        assert_eq!(anchor.count_nodes(true, false), 1);
    }

    #[test]
    fn deleting_missing_clause_preserves_c_count_mutation_shape() {
        let mut bank = test_bank();
        let first = typed_const(&mut bank, "a");
        let second = typed_const(&mut bank, "b");
        let indexed = clause_from(vec![literal(&mut bank, &first, &second, true)], 40);
        let missing = clause_from(vec![literal(&mut bank, &first, &second, true)], 41);
        let mut anchor = ac_anchor(usize::try_from(second.f_code()).unwrap() + 1);
        let mut packed = fv_index_pack_clause(indexed, Some(&anchor));
        let first_value = packed.vector().unwrap().as_slice()[0];

        assert!(anchor.insert(&mut packed, &bank));
        assert!(!anchor.delete(&missing));
        assert_eq!(anchor.index().clause_count(), 0);
        assert!(anchor
            .index()
            .get_next_non_empty_node(first_value)
            .is_none());
    }

    #[test]
    fn index_print_lop_string_renders_root_alternatives_and_clauses() {
        let mut bank = test_bank();
        let first = typed_const(&mut bank, "fv_index_a");
        let second = typed_const(&mut bank, "fv_index_b");
        let clause = clause_from(vec![literal(&mut bank, &first, &second, true)], 50);
        let mut index = FvIndex::new();
        let vector = FreqVector::from_values(vec![2, 0]);

        assert!(
            index
                .insert_vector_clause(&vector, 1, clause)
                .inserted_clause
        );

        assert_eq!(
            index.print_lop_string(&bank, true),
            "* ROOT *\nAlternative 2: \n--Alternative 0: \n------fv_index_a=fv_index_b <- . \n"
        );
    }

    #[test]
    fn index_print_format_string_dispatches_clause_output() {
        let mut bank = test_bank();
        let first = typed_const(&mut bank, "fv_format_a");
        let second = typed_const(&mut bank, "fv_format_b");
        let clause = clause_from(vec![literal(&mut bank, &first, &second, true)], 51);
        let mut index = FvIndex::new();
        let vector = FreqVector::from_values(vec![3, 1]);

        assert!(
            index
                .insert_vector_clause(&vector, 1, clause)
                .inserted_clause
        );

        let input_clause_tree = index
            .print_format_string(&bank, true, IoFormat::Tptp, ProblemType::FirstOrder)
            .unwrap_or_else(|err| panic!("{err}"));
        assert!(input_clause_tree.starts_with("* ROOT *\nAlternative 3: "));
        assert!(input_clause_tree.contains("------input_clause("));
        assert!(input_clause_tree.contains("++equal(fv_format_a, fv_format_b)"));
        assert!(!input_clause_tree.contains("<-"));

        let wrapped_clause_tree = index
            .print_format_string(&bank, true, IoFormat::Tstp, ProblemType::FirstOrder)
            .unwrap_or_else(|err| panic!("{err}"));
        assert!(
            wrapped_clause_tree.contains("------cnf(")
                || wrapped_clause_tree.contains("------tcf(")
        );
        assert!(wrapped_clause_tree.contains("fv_format_a"));
        assert!(!wrapped_clause_tree.contains("<-"));

        assert_eq!(
            index
                .print_format_string(&bank, true, IoFormat::Auto, ProblemType::FirstOrder)
                .unwrap_or_else(|err| panic!("{err}")),
            index.print_lop_string(&bank, true)
        );
    }

    #[test]
    fn insert_storage_tracks_c_successor_map_transitions() {
        let mut bank = test_bank();
        let first = typed_const(&mut bank, "fv_storage_a");
        let second = typed_const(&mut bank, "fv_storage_b");
        let mut dense = FvIndex::new();

        let first_insert = dense.insert_vector_clause(
            &FreqVector::from_values(vec![0]),
            60,
            clause_from(vec![literal(&mut bank, &first, &second, true)], 60),
        );
        assert!(first_insert.inserted_clause);
        assert_eq!(
            first_insert.storage_delta,
            i128::try_from(FVINDEX_MEM + INTMAPCELL_MEM).unwrap()
        );

        let dense_insert = dense.insert_vector_clause(
            &FreqVector::from_values(vec![1]),
            61,
            clause_from(vec![literal(&mut bank, &first, &second, true)], 61),
        );
        assert!(dense_insert.inserted_clause);
        assert_eq!(dense_insert.storage_delta, 72);

        let mut sparse = FvIndex::new();
        let _first_sparse = sparse.insert_vector_clause(
            &FreqVector::from_values(vec![100]),
            62,
            clause_from(vec![literal(&mut bank, &first, &second, true)], 62),
        );
        let sparse_insert = sparse.insert_vector_clause(
            &FreqVector::from_values(vec![0]),
            63,
            clause_from(vec![literal(&mut bank, &first, &second, true)], 63),
        );
        assert!(sparse_insert.inserted_clause);
        assert_eq!(sparse_insert.storage_delta, 64);
    }

    #[test]
    fn insert_storage_can_decrease_when_c_intmap_switches_tree_to_array() {
        let mut bank = test_bank();
        let first = typed_const(&mut bank, "fv_storage_switch_a");
        let second = typed_const(&mut bank, "fv_storage_switch_b");
        let mut index = FvIndex::new();
        let mut clause_id = 70;

        for key in [100, 0] {
            let result = index.insert_vector_clause(
                &FreqVector::from_values(vec![key]),
                clause_id,
                clause_from(vec![literal(&mut bank, &first, &second, true)], clause_id),
            );
            assert!(result.inserted_clause);
            clause_id += 1;
        }
        for key in 1..=23 {
            let result = index.insert_vector_clause(
                &FreqVector::from_values(vec![key]),
                clause_id,
                clause_from(vec![literal(&mut bank, &first, &second, true)], clause_id),
            );
            assert_eq!(result.storage_delta, 40);
            clause_id += 1;
        }
        assert_eq!(
            index.successor_storage.as_ref().map(IntMap::map_type),
            Some(IntMapType::Tree)
        );

        let switch_result = index.insert_vector_clause(
            &FreqVector::from_values(vec![24]),
            clause_id,
            clause_from(vec![literal(&mut bank, &first, &second, true)], clause_id),
        );

        assert!(switch_result.inserted_clause);
        assert_eq!(switch_result.storage_delta, -144);
        assert_eq!(
            index.successor_storage.as_ref().map(IntMap::map_type),
            Some(IntMapType::Array)
        );
    }
}
