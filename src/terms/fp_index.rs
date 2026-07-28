use crate::basics::defines::DEFAULT_COMCHAR_RAW;
use crate::basics::objtrees::ObjTree;
use crate::basics::pstacks::{PStack, PStackInt};
use crate::terms::functypes::FunCode;
use crate::terms::idx_fp::{
    index_dt_create, FingerprintIndexFunction, IndexFingerprint, ANY_VAR, BELOW_VAR, NOT_IN_TERM,
};
use crate::terms::signature::Signature;
use crate::terms::termtypes::Term;
use std::collections::btree_map::Entry;
use std::collections::BTreeMap;
use std::fmt::{self, Write};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

static INDEX_TELEMETRY_ENABLED: AtomicBool = AtomicBool::new(false);
static INDEX_INSERTIONS: AtomicU64 = AtomicU64::new(0);
static INDEX_DELETIONS: AtomicU64 = AtomicU64::new(0);
static INDEX_UNIFICATION_QUERIES: AtomicU64 = AtomicU64::new(0);
static INDEX_UNIFICATION_LEAVES: AtomicU64 = AtomicU64::new(0);
static INDEX_UNIFICATION_CANDIDATES: AtomicU64 = AtomicU64::new(0);
static INDEX_MATCH_QUERIES: AtomicU64 = AtomicU64::new(0);
static INDEX_MATCH_LEAVES: AtomicU64 = AtomicU64::new(0);
static INDEX_MATCH_CANDIDATES: AtomicU64 = AtomicU64::new(0);
static PARAMODULATION_CANDIDATES: AtomicU64 = AtomicU64::new(0);
static PARAMODULATION_UNIFIABLE_CANDIDATES: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct FingerprintIndexTelemetrySnapshot {
    pub insertions: u64,
    pub deletions: u64,
    pub unification_queries: u64,
    pub unification_leaves: u64,
    pub unification_candidates: u64,
    pub match_queries: u64,
    pub match_leaves: u64,
    pub match_candidates: u64,
    pub paramodulation_candidates: u64,
    pub paramodulation_unifiable_candidates: u64,
}

impl FingerprintIndexTelemetrySnapshot {
    #[must_use]
    pub const fn since(self, baseline: Self) -> Self {
        Self {
            insertions: self.insertions.saturating_sub(baseline.insertions),
            deletions: self.deletions.saturating_sub(baseline.deletions),
            unification_queries: self
                .unification_queries
                .saturating_sub(baseline.unification_queries),
            unification_leaves: self
                .unification_leaves
                .saturating_sub(baseline.unification_leaves),
            unification_candidates: self
                .unification_candidates
                .saturating_sub(baseline.unification_candidates),
            match_queries: self.match_queries.saturating_sub(baseline.match_queries),
            match_leaves: self.match_leaves.saturating_sub(baseline.match_leaves),
            match_candidates: self
                .match_candidates
                .saturating_sub(baseline.match_candidates),
            paramodulation_candidates: self
                .paramodulation_candidates
                .saturating_sub(baseline.paramodulation_candidates),
            paramodulation_unifiable_candidates: self
                .paramodulation_unifiable_candidates
                .saturating_sub(baseline.paramodulation_unifiable_candidates),
        }
    }
}

#[must_use]
pub(crate) fn fingerprint_index_telemetry_snapshot() -> FingerprintIndexTelemetrySnapshot {
    FingerprintIndexTelemetrySnapshot {
        insertions: INDEX_INSERTIONS.load(Ordering::Relaxed),
        deletions: INDEX_DELETIONS.load(Ordering::Relaxed),
        unification_queries: INDEX_UNIFICATION_QUERIES.load(Ordering::Relaxed),
        unification_leaves: INDEX_UNIFICATION_LEAVES.load(Ordering::Relaxed),
        unification_candidates: INDEX_UNIFICATION_CANDIDATES.load(Ordering::Relaxed),
        match_queries: INDEX_MATCH_QUERIES.load(Ordering::Relaxed),
        match_leaves: INDEX_MATCH_LEAVES.load(Ordering::Relaxed),
        match_candidates: INDEX_MATCH_CANDIDATES.load(Ordering::Relaxed),
        paramodulation_candidates: PARAMODULATION_CANDIDATES.load(Ordering::Relaxed),
        paramodulation_unifiable_candidates: PARAMODULATION_UNIFIABLE_CANDIDATES
            .load(Ordering::Relaxed),
    }
}

pub(crate) struct FingerprintIndexTelemetryGuard;

impl Drop for FingerprintIndexTelemetryGuard {
    fn drop(&mut self) {
        INDEX_TELEMETRY_ENABLED.store(false, Ordering::Relaxed);
    }
}

#[must_use]
pub(crate) fn enable_fingerprint_index_telemetry() -> FingerprintIndexTelemetryGuard {
    INDEX_TELEMETRY_ENABLED.store(true, Ordering::Relaxed);
    FingerprintIndexTelemetryGuard
}

pub(crate) fn record_paramodulation_candidate_result(unifiable: bool) {
    if !INDEX_TELEMETRY_ENABLED.load(Ordering::Relaxed) {
        return;
    }
    PARAMODULATION_CANDIDATES.fetch_add(1, Ordering::Relaxed);
    if unifiable {
        PARAMODULATION_UNIFIABLE_CANDIDATES.fetch_add(1, Ordering::Relaxed);
    }
}

fn record_index_update(counter: &AtomicU64) {
    if INDEX_TELEMETRY_ENABLED.load(Ordering::Relaxed) {
        counter.fetch_add(1, Ordering::Relaxed);
    }
}

fn record_index_query(
    query_counter: &AtomicU64,
    leaf_counter: &AtomicU64,
    candidate_counter: &AtomicU64,
    leaves: usize,
    candidates: usize,
) {
    if !INDEX_TELEMETRY_ENABLED.load(Ordering::Relaxed) {
        return;
    }
    query_counter.fetch_add(1, Ordering::Relaxed);
    leaf_counter.fetch_add(u64::try_from(leaves).unwrap_or(u64::MAX), Ordering::Relaxed);
    candidate_counter.fetch_add(
        u64::try_from(candidates).unwrap_or(u64::MAX),
        Ordering::Relaxed,
    );
}

#[derive(Clone, Debug, Default)]
pub struct FPTree<T>
where
    T: Ord + Clone,
{
    alternatives: BTreeMap<FunCode, Self>,
    count: usize,
    payload: Option<ObjTree<T>>,
}

impl<T> FPTree<T>
where
    T: Ord + Clone,
{
    #[must_use]
    pub const fn new() -> Self {
        Self {
            alternatives: BTreeMap::new(),
            count: 0,
            payload: None,
        }
    }

    #[must_use]
    pub const fn child_count(&self) -> usize {
        self.count
    }

    #[must_use]
    pub const fn payload(&self) -> Option<&ObjTree<T>> {
        self.payload.as_ref()
    }

    pub fn payload_mut(&mut self) -> Option<&mut ObjTree<T>> {
        self.payload.as_mut()
    }

    pub fn ensure_payload(&mut self) -> &mut ObjTree<T> {
        self.payload.get_or_insert_with(ObjTree::new)
    }

    pub fn store_payload(&mut self, object: T) -> Option<&T> {
        self.ensure_payload().store(object)
    }

    pub fn clear_payload(&mut self) -> Option<ObjTree<T>> {
        self.payload.take()
    }

    #[must_use]
    pub fn payload_nodes(&self) -> usize {
        self.payload.as_ref().map_or(0, ObjTree::nodes)
    }

    #[must_use]
    pub fn find(&self, key: &IndexFingerprint) -> Option<&Self> {
        let mut current = self;
        for sample in key.samples() {
            current = current.alternative(*sample)?;
        }
        Some(current)
    }

    pub fn find_mut(&mut self, key: &IndexFingerprint) -> Option<&mut Self> {
        let mut current = self;
        for sample in key.samples() {
            current = current.alternative_mut(*sample)?;
        }
        Some(current)
    }

    pub fn insert(&mut self, key: &IndexFingerprint) -> &mut Self {
        let mut current = self;
        for sample in key.samples() {
            current = current.alternative_ref(*sample);
        }
        current
    }

    pub fn delete(&mut self, key: &IndexFingerprint) {
        let _ = self.delete_rek(key.samples(), 0);
    }

    pub fn find_unifiable<'a>(
        &'a self,
        key: &IndexFingerprint,
        sig: &Signature,
        collect: &mut Vec<Option<&'a ObjTree<T>>>,
    ) -> usize {
        self.find_unifiable_rek(key.samples(), sig, 0, collect)
    }

    pub fn find_matchable<'a>(
        &'a self,
        key: &IndexFingerprint,
        sig: &Signature,
        collect: &mut Vec<Option<&'a ObjTree<T>>>,
    ) -> usize {
        self.find_matchable_rek(key.samples(), sig, 0, collect)
    }

    pub fn find_dt_unifiable<'a>(
        &'a self,
        key: &IndexFingerprint,
        sig: &Signature,
        collect: &mut Vec<Option<&'a ObjTree<T>>>,
    ) -> usize {
        self.dt_find_unifiable_rek(key.samples(), sig, 0, 0, 0, collect)
    }

    pub fn find_dt_matchable<'a>(
        &'a self,
        key: &IndexFingerprint,
        sig: &Signature,
        collect: &mut Vec<Option<&'a ObjTree<T>>>,
    ) -> usize {
        self.dt_find_matchable_rek(key.samples(), sig, 0, 0, collect)
    }

    pub fn collect_leaves<'a>(&'a self, result: &mut Vec<&'a Self>) -> usize {
        let start = result.len();
        self.collect_leaves_rek(result);
        result.len() - start
    }

    pub fn print_with<F>(&self, mut print_leaf: F) -> String
    where
        F: FnMut(&[FunCode], &Self, &mut String),
    {
        let mut output = String::new();
        let _ = self.write_print_with(&mut output, |path, leaf, output| {
            print_leaf(path, leaf, output);
            Ok(())
        });
        output
    }

    pub fn write_print_with<W, F>(&self, output: &mut W, mut print_leaf: F) -> fmt::Result
    where
        W: Write + ?Sized,
        F: FnMut(&[FunCode], &Self, &mut W) -> fmt::Result,
    {
        let mut payload_paths = Vec::new();
        self.collect_payload_paths(&mut payload_paths);
        for (path, leaf) in payload_paths {
            print_leaf(&path, leaf, output)?;
        }
        Ok(())
    }

    pub fn write_distrib(&self, output: &mut impl Write) -> fmt::Result {
        let mut payload_paths = Vec::new();
        self.collect_payload_paths(&mut payload_paths);
        let leaves = payload_paths.len();
        let entries = payload_paths
            .iter()
            .map(|(_path, leaf)| leaf.payload_nodes())
            .sum::<usize>();
        for (path, leaf) in payload_paths {
            write_leaf_size(&path, leaf, output)?;
        }
        #[expect(
            clippy::cast_precision_loss,
            reason = "C FPIndexDistribPrint casts long counters to double for the summary"
        )]
        let entries_per_leaf = entries as f64 / leaves as f64;
        writeln!(
            output,
            "{DEFAULT_COMCHAR_RAW} {entries} entries, {leaves} leaves, {entries_per_leaf:.6} entries/leaf"
        )
    }

    #[must_use]
    pub fn distrib_string(&self) -> String {
        let mut output = String::new();
        let _ = self.write_distrib(&mut output);
        output
    }

    #[must_use]
    pub fn dot_string<F>(&self, name: &str, sig: &Signature, mut print_payload: F) -> String
    where
        F: FnMut(&ObjTree<T>, &Signature) -> String,
    {
        let mut output = String::new();
        let _ = writeln!(output, "graph {name}{{");
        output.push_str("   rankdir=LR\n   nodesep=0.05\n");

        let mut path = Vec::new();
        self.write_dot_nodes(sig, &mut path, &mut output);
        self.write_dot_edges(sig, &mut output);

        let mut leaves = Vec::new();
        self.collect_leaves(&mut leaves);
        for leaf in leaves {
            if let Some(payload) = &leaf.payload {
                output.push_str(&print_payload(payload, sig));
                let _ = writeln!(
                    output,
                    "   {} -- t{:p} [ranksep=0.1]",
                    leaf.dot_node_id(),
                    payload
                );
            }
        }

        output.push_str("}\n");
        output
    }

    #[must_use]
    pub fn collect_distrib(&self) -> FPIndexDistrib {
        let mut payload_sizes = PStack::<PStackInt>::new();
        let nodes = self.collect_distrib_rek(&mut payload_sizes);
        let entries = payload_sizes
            .as_slice()
            .iter()
            .map(|value| usize::try_from(*value).unwrap_or(usize::MAX))
            .fold(0_usize, usize::saturating_add);
        let (average, stddev) = payload_sizes.compute_average();
        FPIndexDistrib {
            nodes,
            leaves: payload_sizes.len(),
            entries,
            average,
            stddev,
        }
    }

    fn alternative(&self, f_code: FunCode) -> Option<&Self> {
        self.alternatives.get(&f_code)
    }

    fn alternative_mut(&mut self, f_code: FunCode) -> Option<&mut Self> {
        self.alternatives.get_mut(&f_code)
    }

    fn alternative_ref(&mut self, f_code: FunCode) -> &mut Self {
        match self.alternatives.entry(f_code) {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => {
                self.count += 1;
                entry.insert(Self::new())
            }
        }
    }

    fn delete_rek(&mut self, samples: &[FunCode], current: usize) -> bool {
        if current == samples.len() {
            return self.payload.is_none();
        }

        let sample = samples[current];
        let delete = self
            .alternatives
            .get_mut(&sample)
            .is_some_and(|child| child.delete_rek(samples, current + 1));
        if delete && self.alternatives.remove(&sample).is_some() {
            self.count = self.count.saturating_sub(1);
        }
        self.count == 0
    }

    fn find_unifiable_rek<'a>(
        &'a self,
        samples: &[FunCode],
        sig: &Signature,
        current: usize,
        collect: &mut Vec<Option<&'a ObjTree<T>>>,
    ) -> usize {
        if current == samples.len() {
            collect.push(self.payload.as_ref());
            return 1;
        }

        let sample = samples[current];
        let mut result = 0;
        if sample > 0 {
            result += self.alternative(sample).map_or(0, |child| {
                child.find_unifiable_rek(samples, sig, current + 1, collect)
            });
            if sig.symbol_unifies_with_var(sample) {
                result += self.alternative(ANY_VAR).map_or(0, |child| {
                    child.find_unifiable_rek(samples, sig, current + 1, collect)
                });
                result += self.alternative(BELOW_VAR).map_or(0, |child| {
                    child.find_unifiable_rek(samples, sig, current + 1, collect)
                });
            }
        } else if sample == NOT_IN_TERM {
            result += self.alternative(NOT_IN_TERM).map_or(0, |child| {
                child.find_unifiable_rek(samples, sig, current + 1, collect)
            });
            result += self.alternative(BELOW_VAR).map_or(0, |child| {
                child.find_unifiable_rek(samples, sig, current + 1, collect)
            });
        } else if sample == BELOW_VAR || sample == ANY_VAR {
            result += self.alternative(ANY_VAR).map_or(0, |child| {
                child.find_unifiable_rek(samples, sig, current + 1, collect)
            });
            result += self.alternative(BELOW_VAR).map_or(0, |child| {
                child.find_unifiable_rek(samples, sig, current + 1, collect)
            });

            let iter_start = if sample == BELOW_VAR { NOT_IN_TERM } else { 1 };
            for (f_code, child) in self.alternatives.range(iter_start..) {
                if *f_code <= 0 || sig.symbol_unifies_with_var(*f_code) {
                    result += child.find_unifiable_rek(samples, sig, current + 1, collect);
                }
            }
        }
        result
    }

    fn find_matchable_rek<'a>(
        &'a self,
        samples: &[FunCode],
        sig: &Signature,
        current: usize,
        collect: &mut Vec<Option<&'a ObjTree<T>>>,
    ) -> usize {
        if current == samples.len() {
            collect.push(self.payload.as_ref());
            return 1;
        }

        let sample = samples[current];
        let mut result = 0;
        if sample > 0 {
            result += self.alternative(sample).map_or(0, |child| {
                child.find_matchable_rek(samples, sig, current + 1, collect)
            });
        } else if sample == NOT_IN_TERM {
            result += self.alternative(NOT_IN_TERM).map_or(0, |child| {
                child.find_matchable_rek(samples, sig, current + 1, collect)
            });
        } else if sample == BELOW_VAR || sample == ANY_VAR {
            result += self.alternative(ANY_VAR).map_or(0, |child| {
                child.find_matchable_rek(samples, sig, current + 1, collect)
            });
            if sample == BELOW_VAR {
                result += self.alternative(BELOW_VAR).map_or(0, |child| {
                    child.find_matchable_rek(samples, sig, current + 1, collect)
                });
            }

            let iter_start = if sample == BELOW_VAR { NOT_IN_TERM } else { 1 };
            for (f_code, child) in self.alternatives.range(iter_start..) {
                if *f_code <= 0 || sig.symbol_unifies_with_var(*f_code) {
                    result += child.find_matchable_rek(samples, sig, current + 1, collect);
                }
            }
        }
        result
    }

    fn dt_find_matchable_rek<'a>(
        &'a self,
        samples: &[FunCode],
        sig: &Signature,
        current: usize,
        skip_term: i32,
        collect: &mut Vec<Option<&'a ObjTree<T>>>,
    ) -> usize {
        if skip_term > 0 {
            let mut result = 0;
            for (f_code, child) in self.alternatives.range(BELOW_VAR..) {
                result += child.dt_find_matchable_rek(
                    samples,
                    sig,
                    current,
                    skip_term - 1 + symbol_arity(sig, *f_code),
                    collect,
                );
            }
            return result;
        }
        if current == samples.len() {
            collect.push(self.payload.as_ref());
            return 1;
        }

        let sample = samples[current];
        if sample == ANY_VAR {
            let mut result = 0;
            for (f_code, child) in self.alternatives.range(BELOW_VAR..) {
                if *f_code <= 0 || sig.symbol_unifies_with_var(*f_code) {
                    result += child.dt_find_matchable_rek(
                        samples,
                        sig,
                        current + 1,
                        symbol_arity(sig, *f_code),
                        collect,
                    );
                }
            }
            result
        } else {
            self.alternative(sample).map_or(0, |child| {
                child.dt_find_matchable_rek(samples, sig, current + 1, 0, collect)
            })
        }
    }

    fn dt_find_unifiable_rek<'a>(
        &'a self,
        samples: &[FunCode],
        sig: &Signature,
        current: usize,
        skip_term: i32,
        skip_key: i32,
        collect: &mut Vec<Option<&'a ObjTree<T>>>,
    ) -> usize {
        if skip_term > 0 {
            let mut result = 0;
            for (f_code, child) in self.alternatives.range(BELOW_VAR..) {
                result += child.dt_find_unifiable_rek(
                    samples,
                    sig,
                    current,
                    skip_term - 1 + symbol_arity(sig, *f_code),
                    0,
                    collect,
                );
            }
            return result;
        }
        if skip_key > 0 {
            let Some(sample) = samples.get(current).copied() else {
                return 0;
            };
            return self.dt_find_unifiable_rek(
                samples,
                sig,
                current + 1,
                0,
                skip_key - 1 + symbol_arity(sig, sample),
                collect,
            );
        }
        if current == samples.len() {
            collect.push(self.payload.as_ref());
            return 1;
        }

        let sample = samples[current];
        if sample == ANY_VAR {
            let mut result = 0;
            for (f_code, child) in self.alternatives.range(BELOW_VAR..) {
                if *f_code <= 0 || sig.symbol_unifies_with_var(*f_code) {
                    result += child.dt_find_unifiable_rek(
                        samples,
                        sig,
                        current + 1,
                        symbol_arity(sig, *f_code),
                        0,
                        collect,
                    );
                }
            }
            result
        } else {
            let mut result = self.alternative(sample).map_or(0, |child| {
                child.dt_find_unifiable_rek(samples, sig, current + 1, 0, 0, collect)
            });
            if sample <= 0 || sig.symbol_unifies_with_var(sample) {
                result += self.alternative(ANY_VAR).map_or(0, |child| {
                    child.dt_find_unifiable_rek(
                        samples,
                        sig,
                        current + 1,
                        0,
                        symbol_arity(sig, sample),
                        collect,
                    )
                });
            }
            result
        }
    }

    fn collect_leaves_rek<'a>(&'a self, result: &mut Vec<&'a Self>) {
        if self.alternatives.is_empty() {
            result.push(self);
        } else {
            for child in self.alternatives.values() {
                child.collect_leaves_rek(result);
            }
        }
    }

    fn collect_distrib_rek(&self, payload_sizes: &mut PStack<PStackInt>) -> usize {
        if let Some(payload) = &self.payload {
            let payload_nodes = PStackInt::try_from(payload.nodes()).unwrap_or(PStackInt::MAX);
            payload_sizes.push(payload_nodes);
        }

        1 + self
            .alternatives
            .values()
            .map(|child| child.collect_distrib_rek(payload_sizes))
            .sum::<usize>()
    }

    fn collect_payload_paths<'a>(&'a self, result: &mut Vec<(Vec<FunCode>, &'a Self)>) {
        let mut path = Vec::new();
        self.collect_payload_paths_rek(&mut path, result);
    }

    fn collect_payload_paths_rek<'a>(
        &'a self,
        path: &mut Vec<FunCode>,
        result: &mut Vec<(Vec<FunCode>, &'a Self)>,
    ) {
        if self.payload.is_some() {
            result.push((path.clone(), self));
        }

        for (sample, child) in &self.alternatives {
            path.push(*sample);
            child.collect_payload_paths_rek(path, result);
            let _ = path.pop();
        }
    }

    fn write_dot_nodes(&self, sig: &Signature, path: &mut Vec<FunCode>, output: &mut String) {
        let label = fp_path_label(sig, path);
        let _ = writeln!(output, "   {} [label=\"{}\"]", self.dot_node_id(), label);

        for (sample, child) in &self.alternatives {
            path.push(*sample);
            child.write_dot_nodes(sig, path, output);
            let _ = path.pop();
        }
    }

    fn write_dot_edges(&self, sig: &Signature, output: &mut String) {
        for (sample, child) in &self.alternatives {
            let _ = writeln!(
                output,
                "   {} -- {} [label={}]",
                self.dot_node_id(),
                child.dot_node_id(),
                fp_symbol(sig, *sample)
            );
            child.write_dot_edges(sig, output);
        }
    }

    fn dot_node_id(&self) -> String {
        format!("l{self:p}")
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FPIndexDistrib {
    pub nodes: usize,
    pub leaves: usize,
    pub entries: usize,
    pub average: f64,
    pub stddev: f64,
}

impl FPIndexDistrib {
    pub fn write_data(self, output: &mut impl Write) -> fmt::Result {
        write!(
            output,
            "{:5} nodes, {:5} leaves, {:6.2}+/-{:4.3} terms/leaf",
            self.nodes, self.leaves, self.average, self.stddev
        )
    }

    #[must_use]
    pub fn data_string(self) -> String {
        let mut output = String::new();
        let _ = self.write_data(&mut output);
        output
    }
}

#[derive(Clone, Debug)]
pub struct FPIndex<T>
where
    T: Ord + Clone,
{
    index: FPTree<T>,
    fp_fun: FingerprintIndexFunction,
    discrimination_tree: bool,
}

impl<T> FPIndex<T>
where
    T: Ord + Clone,
{
    #[must_use]
    pub fn new(fp_fun: FingerprintIndexFunction) -> Self {
        Self {
            index: FPTree::new(),
            fp_fun,
            discrimination_tree: std::ptr::fn_addr_eq(
                fp_fun,
                index_dt_create as FingerprintIndexFunction,
            ),
        }
    }

    #[must_use]
    pub const fn root(&self) -> &FPTree<T> {
        &self.index
    }

    pub fn root_mut(&mut self) -> &mut FPTree<T> {
        &mut self.index
    }

    #[must_use]
    pub fn find(&self, term: &Term) -> Option<&FPTree<T>> {
        let key = (self.fp_fun)(term);
        self.index.find(&key)
    }

    pub fn find_mut(&mut self, term: &Term) -> Option<&mut FPTree<T>> {
        let key = (self.fp_fun)(term);
        self.index.find_mut(&key)
    }

    pub fn insert(&mut self, term: &Term) -> &mut FPTree<T> {
        record_index_update(&INDEX_INSERTIONS);
        let key = (self.fp_fun)(term);
        self.index.insert(&key)
    }

    pub fn delete(&mut self, term: &Term) {
        record_index_update(&INDEX_DELETIONS);
        let key = (self.fp_fun)(term);
        self.index.delete(&key);
    }

    /// Finds unifiable leaves using a caller-provided live signature.
    ///
    /// Proof search may add generated symbols after an index was constructed.
    /// Discrimination-tree traversal needs their current arities, so callers
    /// with a mutable term-bank owner must use its live signature here.
    pub fn find_unifiable<'b>(
        &'b self,
        term: &Term,
        sig: &Signature,
        collect: &mut Vec<Option<&'b ObjTree<T>>>,
    ) -> usize {
        let _timer = crate::basics::perf_counters::start(
            crate::basics::perf_counters::PerfCounter::IndexUnifTimer,
        );
        let start = collect.len();
        let key = (self.fp_fun)(term);
        let leaves = if self.discrimination_tree {
            self.index.find_dt_unifiable(&key, sig, collect)
        } else {
            self.index.find_unifiable(&key, sig, collect)
        };
        let candidates = if INDEX_TELEMETRY_ENABLED.load(Ordering::Relaxed) {
            collect[start..]
                .iter()
                .flatten()
                .map(|payload| payload.nodes())
                .fold(0_usize, usize::saturating_add)
        } else {
            0
        };
        record_index_query(
            &INDEX_UNIFICATION_QUERIES,
            &INDEX_UNIFICATION_LEAVES,
            &INDEX_UNIFICATION_CANDIDATES,
            leaves,
            candidates,
        );
        leaves
    }

    /// Finds matchable leaves using a caller-provided live signature.
    pub fn find_matchable<'b>(
        &'b self,
        term: &Term,
        sig: &Signature,
        collect: &mut Vec<Option<&'b ObjTree<T>>>,
    ) -> usize {
        let _timer = crate::basics::perf_counters::start(
            crate::basics::perf_counters::PerfCounter::IndexMatchTimer,
        );
        let start = collect.len();
        let key = (self.fp_fun)(term);
        let leaves = if self.discrimination_tree {
            self.index.find_dt_matchable(&key, sig, collect)
        } else {
            self.index.find_matchable(&key, sig, collect)
        };
        let candidates = if INDEX_TELEMETRY_ENABLED.load(Ordering::Relaxed) {
            collect[start..]
                .iter()
                .flatten()
                .map(|payload| payload.nodes())
                .fold(0_usize, usize::saturating_add)
        } else {
            0
        };
        record_index_query(
            &INDEX_MATCH_QUERIES,
            &INDEX_MATCH_LEAVES,
            &INDEX_MATCH_CANDIDATES,
            leaves,
            candidates,
        );
        leaves
    }

    pub fn collect_leaves<'b>(&'b self, result: &mut Vec<&'b FPTree<T>>) -> usize {
        self.index.collect_leaves(result)
    }

    pub fn print_with<F>(&self, print_leaf: F) -> String
    where
        F: FnMut(&[FunCode], &FPTree<T>, &mut String),
    {
        self.index.print_with(print_leaf)
    }

    pub fn write_print_with<W, F>(&self, output: &mut W, print_leaf: F) -> fmt::Result
    where
        W: Write + ?Sized,
        F: FnMut(&[FunCode], &FPTree<T>, &mut W) -> fmt::Result,
    {
        self.index.write_print_with(output, print_leaf)
    }

    pub fn write_distrib(&self, output: &mut impl Write) -> fmt::Result {
        self.index.write_distrib(output)
    }

    #[must_use]
    pub fn distrib_string(&self) -> String {
        self.index.distrib_string()
    }

    #[must_use]
    pub fn dot_string<F>(&self, name: &str, sig: &Signature, print_payload: F) -> String
    where
        F: FnMut(&ObjTree<T>, &Signature) -> String,
    {
        self.index.dot_string(name, sig, print_payload)
    }

    #[must_use]
    pub fn collect_distrib(&self) -> FPIndexDistrib {
        self.index.collect_distrib()
    }
}

fn symbol_arity(sig: &Signature, f_code: FunCode) -> i32 {
    if f_code > 0 {
        sig.find_arity(f_code)
            .expect("discrimination-tree index requires known positive f-code")
    } else {
        0
    }
}

fn write_leaf_size<T>(path: &[FunCode], leaf: &FPTree<T>, output: &mut impl Write) -> fmt::Result
where
    T: Ord + Clone,
{
    write!(output, "{DEFAULT_COMCHAR_RAW} ")?;
    for sample in path {
        write!(output, "{sample:4}.")?;
    }
    writeln!(output, ":{} terms", leaf.payload_nodes())
}

fn fp_path_label(sig: &Signature, path: &[FunCode]) -> String {
    path.iter()
        .map(|sample| fp_symbol(sig, *sample))
        .collect::<Vec<_>>()
        .join(", ")
}

fn fp_symbol(sig: &Signature, symbol: FunCode) -> String {
    match symbol {
        BELOW_VAR => "B".to_owned(),
        ANY_VAR => "A".to_owned(),
        NOT_IN_TERM => "N".to_owned(),
        _ => sig
            .find_name(symbol)
            .expect("fingerprint sample must name a signature symbol")
            .to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::{FPIndex, FPTree};
    use crate::basics::objtrees::ObjTree;
    use crate::basics::simple_stuff::{set_problem_type, ProblemType};
    use crate::terms::idx_fp::{
        get_fp_index_function, index_dt_create, index_fp1_create, index_fp2_create,
        IndexFingerprint, ANY_VAR, BELOW_VAR, FP_INDEX_NAMES, NOT_IN_TERM,
    };
    use crate::terms::match_mgu::{subst_match_complete, subst_mgu_complete};
    use crate::terms::signature::Signature;
    use crate::terms::simpletypes::Type;
    use crate::terms::subst::Substitution;
    use crate::terms::termtypes::Term;
    use crate::terms::typebanks::TypeBank;
    use crate::test_support::global_state_lock;
    use std::collections::BTreeSet;

    struct TestSignature {
        sig: Signature,
        f: i64,
        g: i64,
        a: i64,
        b: i64,
    }

    fn test_signature() -> TestSignature {
        let mut sig = Signature::new(TypeBank::new());
        let f = sig.insert_id_for_problem("f", 2, false, ProblemType::FirstOrder);
        let g = sig.insert_id_for_problem("g", 1, false, ProblemType::FirstOrder);
        let a = sig.insert_id_for_problem("a", 0, false, ProblemType::FirstOrder);
        let b = sig.insert_id_for_problem("b", 0, false, ProblemType::FirstOrder);
        TestSignature { sig, f, g, a, b }
    }

    fn leaf(code: i64) -> Term {
        Term::const_cell_alloc(code)
    }

    fn var(code: i64) -> Term {
        Term::const_cell_alloc(code)
    }

    fn term(code: i64, args: &[Term]) -> Term {
        let term = Term::top_alloc(code, args.len());
        for (index, arg) in args.iter().enumerate() {
            term.set_argument(index, arg.clone());
        }
        term
    }

    fn typed_leaf(code: i64, type_: &Type) -> Term {
        let term = leaf(code);
        term.set_type(Some(type_.clone()));
        term
    }

    fn typed_term(code: i64, args: &[Term], type_: &Type) -> Term {
        let term = term(code, args);
        term.set_type(Some(type_.clone()));
        term
    }

    fn exact_unification_result(left: &Term, right: &Term) -> bool {
        let mut substitution = Substitution::new();
        let result = subst_mgu_complete(left, right, &mut substitution);
        substitution.backtrack();
        result
    }

    fn exact_match_result(pattern: &Term, target: &Term) -> bool {
        let mut substitution = Substitution::new();
        let result = subst_match_complete(pattern, target, &mut substitution);
        substitution.backtrack();
        result
    }

    fn filtered_result_set(
        index: &FPIndex<usize>,
        indexed_terms: &[(usize, Term)],
        query: &Term,
        signature: &Signature,
        unification: bool,
    ) -> BTreeSet<usize> {
        let mut payloads = Vec::new();
        if unification {
            index.find_unifiable(query, signature, &mut payloads);
        } else {
            index.find_matchable(query, signature, &mut payloads);
        }
        payloads
            .into_iter()
            .flatten()
            .flat_map(ObjTree::to_vec)
            .filter(|candidate| {
                let term = &indexed_terms[*candidate].1;
                if unification {
                    exact_unification_result(query, term)
                } else {
                    exact_match_result(query, term)
                }
            })
            .collect()
    }

    fn slow_result_set(
        active: &BTreeSet<usize>,
        indexed_terms: &[(usize, Term)],
        query: &Term,
        unification: bool,
    ) -> BTreeSet<usize> {
        active
            .iter()
            .copied()
            .filter(|candidate| {
                let term = &indexed_terms[*candidate].1;
                if unification {
                    exact_unification_result(query, term)
                } else {
                    exact_match_result(query, term)
                }
            })
            .collect()
    }

    fn delete_indexed_term(index: &mut FPIndex<usize>, identifier: usize, term: &Term) {
        let payload_empty = {
            let leaf = index.find_mut(term).expect("indexed term must have a leaf");
            let payload = leaf
                .payload_mut()
                .expect("indexed term leaf must have a payload");
            assert_eq!(payload.extract_object(&identifier), Some(identifier));
            let empty = payload.is_empty();
            if empty {
                leaf.clear_payload();
            }
            empty
        };
        if payload_empty {
            index.delete(term);
        }
    }

    fn assert_replay_queries(
        index: &FPIndex<usize>,
        active: &BTreeSet<usize>,
        indexed_terms: &[(usize, Term)],
        queries: &[Term],
        signature: &Signature,
        variant: &str,
    ) {
        for query in queries {
            for unification in [false, true] {
                assert_eq!(
                    filtered_result_set(index, indexed_terms, query, signature, unification),
                    slow_result_set(active, indexed_terms, query, unification),
                    "{variant} changed the exact retrieval result set"
                );
            }
        }
    }

    fn payload_values(stack: &[Option<&ObjTree<i32>>]) -> Vec<i32> {
        stack
            .iter()
            .filter_map(|payload| *payload)
            .flat_map(ObjTree::to_vec)
            .collect()
    }

    #[test]
    fn fingerprint_constructor_accepts_raw_sample_vectors() {
        let fp = IndexFingerprint::from_samples(vec![10, ANY_VAR, NOT_IN_TERM]);

        assert_eq!(fp.raw(), &[4, 10, ANY_VAR, NOT_IN_TERM]);
        assert_eq!(fp.samples(), &[10, ANY_VAR, NOT_IN_TERM]);
    }

    #[test]
    fn tree_insert_find_and_delete_preserve_payload_guard() {
        let mut tree = FPTree::<i32>::new();
        let key = IndexFingerprint::from_samples(vec![10]);

        assert_eq!(tree.insert(&key).store_payload(7), None);
        assert_eq!(tree.child_count(), 1);
        assert_eq!(tree.find(&key).map(FPTree::payload_nodes), Some(1));

        tree.delete(&key);
        assert_eq!(tree.find(&key).map(FPTree::payload_nodes), Some(1));

        tree.find_mut(&key).unwrap().clear_payload();
        tree.delete(&key);
        assert!(tree.find(&key).is_none());
        assert_eq!(tree.child_count(), 0);
    }

    #[test]
    fn ordinary_fingerprint_search_keeps_c_match_and_unify_alternatives() {
        let data = test_signature();
        let query = term(data.f, &[leaf(data.a), leaf(data.b)]);
        let variable = var(-1);
        let other = term(data.g, &[leaf(data.a)]);
        let mut index = FPIndex::new(index_fp2_create);

        index.insert(&query).store_payload(1);
        index.insert(&variable).store_payload(2);
        index.insert(&other).store_payload(3);

        let mut unifiable = Vec::new();
        assert_eq!(index.find_unifiable(&query, &data.sig, &mut unifiable), 2);
        assert_eq!(payload_values(&unifiable), vec![1, 2]);

        let mut matchable = Vec::new();
        assert_eq!(index.find_matchable(&query, &data.sig, &mut matchable), 1);
        assert_eq!(payload_values(&matchable), vec![1]);
    }

    #[test]
    fn not_in_term_and_below_var_follow_different_match_and_unify_rules() {
        let data = test_signature();
        let mut tree = FPTree::<i32>::new();
        let not_in_term = IndexFingerprint::from_samples(vec![data.f, NOT_IN_TERM]);
        let below_var = IndexFingerprint::from_samples(vec![data.f, BELOW_VAR]);

        tree.insert(&not_in_term).store_payload(1);
        tree.insert(&below_var).store_payload(2);

        let mut unifiable = Vec::new();
        assert_eq!(
            tree.find_unifiable(&not_in_term, &data.sig, &mut unifiable),
            2
        );
        assert_eq!(payload_values(&unifiable), vec![1, 2]);

        let mut matchable = Vec::new();
        assert_eq!(
            tree.find_matchable(&not_in_term, &data.sig, &mut matchable),
            1
        );
        assert_eq!(payload_values(&matchable), vec![1]);
    }

    #[test]
    fn discrimination_tree_search_skips_indexed_and_query_subterms() {
        let data = test_signature();
        let exact = term(data.f, &[leaf(data.a), leaf(data.b)]);
        let other = term(data.g, &[leaf(data.a)]);
        let variable = var(-1);
        let mut index = FPIndex::new(index_dt_create);

        index.insert(&exact).store_payload(1);
        index.insert(&other).store_payload(2);
        index.insert(&variable).store_payload(3);

        let mut variable_query = Vec::new();
        assert_eq!(
            index.find_matchable(&variable, &data.sig, &mut variable_query),
            3
        );
        assert_eq!(payload_values(&variable_query), vec![3, 1, 2]);

        let mut concrete_query = Vec::new();
        assert_eq!(
            index.find_unifiable(&exact, &data.sig, &mut concrete_query),
            2
        );
        assert_eq!(payload_values(&concrete_query), vec![1, 3]);

        let mut concrete_match = Vec::new();
        assert_eq!(
            index.find_matchable(&exact, &data.sig, &mut concrete_match),
            1
        );
        assert_eq!(payload_values(&concrete_match), vec![1]);
    }

    #[test]
    fn discrimination_tree_can_query_symbols_added_after_index_creation() {
        let mut data = test_signature();
        let generated = data.sig.insert_id_for_problem(
            "generated_after_index_init",
            1,
            false,
            ProblemType::FirstOrder,
        );
        let generated_term = term(generated, &[leaf(data.a)]);
        let mut index = FPIndex::new(index_dt_create);
        index.insert(&generated_term).store_payload(7);

        let mut unifiable = Vec::new();
        assert_eq!(
            index.find_unifiable(&generated_term, &data.sig, &mut unifiable),
            1
        );
        assert_eq!(payload_values(&unifiable), vec![7]);
    }

    #[test]
    fn wrapper_find_delete_leaves_and_distribution_match_c_shapes() {
        let data = test_signature();
        let f = leaf(data.f);
        let g = leaf(data.g);
        let variable = var(-1);
        let mut index = FPIndex::new(index_fp1_create);

        index.insert(&f).store_payload(1);
        index.insert(&g).store_payload(2);
        index.insert(&variable).store_payload(3);
        assert_eq!(index.find(&f).map(FPTree::payload_nodes), Some(1));

        let mut leaves = Vec::new();
        assert_eq!(index.collect_leaves(&mut leaves), 3);

        let distrib = index.collect_distrib();
        assert_eq!(distrib.nodes, 4);
        assert_eq!(distrib.leaves, 3);
        assert_eq!(distrib.entries, 3);
        assert!((distrib.average - 1.0).abs() < f64::EPSILON);
        assert!(distrib.stddev.abs() < f64::EPSILON);
        assert_eq!(
            distrib.data_string(),
            "    4 nodes,     3 leaves,   1.00+/-0.000 terms/leaf"
        );

        index.delete(&g);
        assert!(index.find(&g).is_some());
        index.find(&g).unwrap().payload().unwrap();
        index
            .root_mut()
            .find_mut(&index_fp1_create(&g))
            .unwrap()
            .clear_payload();
        index.delete(&g);
        assert!(index.find(&g).is_none());
    }

    #[test]
    fn all_index_variants_replay_dynamic_workload_with_exact_result_sets() {
        let _lock = global_state_lock();
        set_problem_type(ProblemType::FirstOrder).unwrap();
        let data = test_signature();
        let type_ = data.sig.type_bank().i_type();
        let a = typed_leaf(data.a, &type_);
        let b = typed_leaf(data.b, &type_);
        let indexed_x = typed_leaf(-2, &type_);
        let indexed_y = typed_leaf(-4, &type_);
        let query_x = typed_leaf(-20, &type_);
        let query_y = typed_leaf(-22, &type_);
        let g_a = typed_term(data.g, std::slice::from_ref(&a), &type_);
        let g_b = typed_term(data.g, std::slice::from_ref(&b), &type_);
        let g_x = typed_term(data.g, std::slice::from_ref(&indexed_x), &type_);
        let indexed_terms = vec![
            (0, a.clone()),
            (1, b.clone()),
            (2, typed_term(data.f, &[a.clone(), b.clone()], &type_)),
            (
                3,
                typed_term(data.f, &[indexed_x.clone(), b.clone()], &type_),
            ),
            (
                4,
                typed_term(data.f, &[indexed_y.clone(), indexed_y.clone()], &type_),
            ),
            (
                5,
                typed_term(data.f, &[g_a.clone(), indexed_x.clone()], &type_),
            ),
            (6, typed_term(data.g, std::slice::from_ref(&g_x), &type_)),
            (7, typed_term(data.f, &[g_b.clone(), g_a.clone()], &type_)),
        ];
        let queries = vec![
            query_x.clone(),
            typed_term(data.g, std::slice::from_ref(&query_x), &type_),
            typed_term(data.f, &[query_x.clone(), b.clone()], &type_),
            typed_term(data.f, &[query_y.clone(), query_y.clone()], &type_),
            typed_term(
                data.f,
                &[
                    typed_term(data.g, std::slice::from_ref(&query_x), &type_),
                    query_y,
                ],
                &type_,
            ),
            typed_term(data.f, &[b.clone(), a.clone()], &type_),
        ];

        for variant in FP_INDEX_NAMES
            .iter()
            .copied()
            .filter(|name| *name != "NoIndex")
        {
            let mut index = FPIndex::new(
                get_fp_index_function(variant).expect("listed index variant must resolve"),
            );
            let mut active = BTreeSet::new();
            for (identifier, term) in &indexed_terms[..4] {
                index.insert(term).store_payload(*identifier);
                active.insert(*identifier);
            }
            assert_replay_queries(
                &index,
                &active,
                &indexed_terms,
                &queries,
                &data.sig,
                variant,
            );

            for (identifier, term) in &indexed_terms[4..] {
                index.insert(term).store_payload(*identifier);
                active.insert(*identifier);
            }
            assert_replay_queries(
                &index,
                &active,
                &indexed_terms,
                &queries,
                &data.sig,
                variant,
            );

            delete_indexed_term(&mut index, 3, &indexed_terms[3].1);
            active.remove(&3);
            assert_replay_queries(
                &index,
                &active,
                &indexed_terms,
                &queries,
                &data.sig,
                variant,
            );
        }
    }

    #[test]
    fn distribution_prints_payload_paths_and_c_summary_shape() {
        let data = test_signature();
        let mut tree = FPTree::<i32>::new();

        tree.insert(&IndexFingerprint::from_samples(vec![data.f]))
            .store_payload(1);
        tree.insert(&IndexFingerprint::from_samples(vec![data.f, ANY_VAR]))
            .store_payload(2);

        assert_eq!(
            tree.distrib_string(),
            format!(
                "% {f:4}.:1 terms\n% {f:4}.{any_var:4}.:1 terms\n% 2 entries, 2 leaves, 1.000000 entries/leaf\n",
                f = data.f,
                any_var = ANY_VAR
            )
        );
        let mut distrib_output = String::new();
        tree.write_distrib(&mut distrib_output).unwrap();
        assert_eq!(distrib_output, tree.distrib_string());

        let rendered = tree.print_with(|path, leaf, output| {
            output.push('[');
            for sample in path {
                output.push_str(&sample.to_string());
                output.push(',');
            }
            output.push_str("]=");
            output.push_str(&leaf.payload_nodes().to_string());
            output.push('\n');
        });
        assert_eq!(
            rendered,
            format!(
                "[{f},]=1\n[{f},{any_var},]=1\n",
                f = data.f,
                any_var = ANY_VAR
            )
        );
        let mut fallible_rendered = String::new();
        tree.write_print_with(&mut fallible_rendered, |path, leaf, output| {
            output.push('[');
            for sample in path {
                output.push_str(&sample.to_string());
                output.push(',');
            }
            output.push_str("]=");
            output.push_str(&leaf.payload_nodes().to_string());
            output.push('\n');
            Ok(())
        })
        .unwrap();
        assert_eq!(fallible_rendered, rendered);
    }

    #[test]
    fn dot_prints_c_pointer_ids_and_only_structural_leaf_payload_edges() {
        let data = test_signature();
        let mut tree = FPTree::<i32>::new();

        tree.insert(&IndexFingerprint::from_samples(vec![data.f]))
            .store_payload(1);
        tree.insert(&IndexFingerprint::from_samples(vec![data.f, ANY_VAR]))
            .store_payload(2);

        let dot = tree.dot_string("fp", &data.sig, |payload, _sig| {
            format!(
                "     t{:p} [shape=box label=\"{} terms\"]\n",
                payload,
                payload.nodes()
            )
        });

        assert!(dot.starts_with("graph fp{\n   rankdir=LR\n   nodesep=0.05\n"));
        assert!(dot.contains("[label=\"\"]\n"));
        assert!(dot.contains("[label=\"f\"]\n"));
        assert!(dot.contains("[label=\"f, A\"]\n"));
        assert!(dot.contains("[label=f]\n"));
        assert!(dot.contains("[label=A]\n"));
        assert_eq!(dot.matches("shape=box").count(), 1);
        assert_eq!(dot.matches("[ranksep=0.1]").count(), 1);
        assert!(dot.ends_with("}\n"));
    }
}
