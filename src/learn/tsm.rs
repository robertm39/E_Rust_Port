use crate::basics::error::Diagnostic;
use crate::basics::pdarrays::{PDArrayIndex, PDIntArray, PDPointerArray};
use crate::learn::flatannoterms::{
    flat_anno_set_add_term, flat_anno_set_alloc, flat_anno_set_eval_average,
    flat_anno_set_eval_weighted_average, flat_anno_set_flatten, FlatAnnoSet, FlatAnnoTerm,
};
use crate::learn::indexfunctions::{
    tsm_index_alloc, tsm_index_alloc_shared, tsm_index_find, tsm_index_insert,
    tsm_index_print_string, IndexType, TSMIndex, INDEX_DYNAMIC_DEPTH,
};
use crate::learn::patterns::PatternSubst;
use crate::terms::signature::Signature;
use crate::terms::termbanks::TermBank;
use crate::terms::termfunc::term_weight;
use crate::terms::termtypes::Term;
use std::fmt::Write as _;
use std::rc::Rc;

pub type TsmPartition = Vec<Option<FlatAnnoTerm>>;
pub type TsmId = usize;

struct TopIndexEval<'a, 'b> {
    set: &'a FlatAnnoSet,
    bank: &'b mut TermBank,
    subst: &'a PatternSubst,
    depth: i32,
    limit: f64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i32)]
pub enum TsmType {
    NoType = 0,
    Flat = 1,
    Recursive = 2,
    Recurrent = 3,
    RecurrentLocal = 4,
}

pub const TSM_TYPE_NAMES: [&str; 5] = ["NoType", "Flat", "Recursive", "Recurrent", "RecLocal"];
pub const TSM_MAX_TERMTOP: i32 = 5;

#[derive(Clone, Debug)]
pub struct Tsa {
    eval_weight: f64,
    eval: f64,
    arity: usize,
    arg_tsms: Vec<TsmId>,
}

#[derive(Clone, Debug)]
pub struct Tsm {
    index: TSMIndex,
    max_index: i64,
    tsas: Option<PDPointerArray<Tsa>>,
}

#[derive(Debug)]
pub struct TsmAdmin {
    tsm_type: TsmType,
    index_bank: TermBank,
    index_type: IndexType,
    index_depth: i32,
    limit: f64,
    local_limit: bool,
    eval_limit: f64,
    unmapped_eval: f64,
    unmapped_weight: f64,
    root_tsm: Option<TsmId>,
    empty_tsm: TsmId,
    tsm_stack: Vec<TsmId>,
    cache_stack: Vec<PDIntArray>,
    subst: Option<Rc<PatternSubst>>,
    tsms: Vec<Tsm>,
}

impl Tsa {
    #[must_use]
    pub const fn new(eval_weight: f64, eval: f64, arity: usize, arg_tsms: Vec<TsmId>) -> Self {
        Self {
            eval_weight,
            eval,
            arity,
            arg_tsms,
        }
    }

    #[must_use]
    pub const fn eval_weight(&self) -> f64 {
        self.eval_weight
    }

    #[must_use]
    pub const fn eval(&self) -> f64 {
        self.eval
    }

    #[must_use]
    pub const fn arity(&self) -> usize {
        self.arity
    }

    #[must_use]
    pub fn arg_tsms(&self) -> &[TsmId] {
        &self.arg_tsms
    }
}

impl Tsm {
    /// Allocates a base TSM cell with an index and no TSA array.
    ///
    /// # Panics
    ///
    /// Panics under the same internal-invariant conditions as
    /// [`tsm_index_alloc`].
    #[must_use]
    pub fn new(index_type: IndexType, depth: i32, subst: PatternSubst) -> Self {
        Self::new_shared(index_type, depth, Rc::new(subst))
    }

    fn new_shared(index_type: IndexType, depth: i32, subst: Rc<PatternSubst>) -> Self {
        Self {
            index: tsm_index_alloc_shared(index_type, depth, subst),
            max_index: -1,
            tsas: None,
        }
    }

    #[must_use]
    pub const fn index(&self) -> &TSMIndex {
        &self.index
    }

    #[must_use]
    pub const fn max_index(&self) -> i64 {
        self.max_index
    }

    #[must_use]
    pub const fn tsas(&self) -> Option<&PDPointerArray<Tsa>> {
        self.tsas.as_ref()
    }
}

impl TsmAdmin {
    /// Allocates a C-shaped `TSMAdminCell` owner.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if the private index term bank cannot be created.
    ///
    /// # Panics
    ///
    /// Panics if the internal empty TSM allocation violates the TSM index
    /// invariants.
    pub fn new(sig: Signature, tsm_type: TsmType) -> Result<Self, Diagnostic> {
        let index_bank = TermBank::new(sig)?;
        let empty_subst = Rc::new(PatternSubst::new(index_bank.signature()));
        let tsms = vec![Tsm::new_shared(IndexType::EMPTY, 0, empty_subst)];
        Ok(Self {
            tsm_type,
            index_bank,
            index_type: IndexType::NO_INDEX,
            index_depth: 0,
            limit: 0.0,
            local_limit: true,
            eval_limit: 0.0,
            unmapped_eval: 0.0,
            unmapped_weight: 0.0,
            root_tsm: None,
            empty_tsm: 0,
            tsm_stack: Vec::new(),
            cache_stack: Vec::new(),
            subst: None,
            tsms,
        })
    }

    #[must_use]
    pub const fn tsm_type(&self) -> TsmType {
        self.tsm_type
    }

    #[must_use]
    pub const fn index_type(&self) -> IndexType {
        self.index_type
    }

    #[must_use]
    pub const fn index_depth(&self) -> i32 {
        self.index_depth
    }

    #[must_use]
    pub const fn limit(&self) -> f64 {
        self.limit
    }

    #[must_use]
    pub const fn eval_limit(&self) -> f64 {
        self.eval_limit
    }

    #[must_use]
    pub const fn local_limit(&self) -> bool {
        self.local_limit
    }

    #[must_use]
    pub const fn unmapped_eval(&self) -> f64 {
        self.unmapped_eval
    }

    #[must_use]
    pub const fn unmapped_weight(&self) -> f64 {
        self.unmapped_weight
    }

    #[must_use]
    pub const fn root_tsm(&self) -> Option<TsmId> {
        self.root_tsm
    }

    #[must_use]
    pub const fn empty_tsm(&self) -> TsmId {
        self.empty_tsm
    }

    #[must_use]
    pub fn tsm_stack(&self) -> &[TsmId] {
        &self.tsm_stack
    }

    #[must_use]
    pub fn cache_stack(&self) -> &[PDIntArray] {
        &self.cache_stack
    }

    #[must_use]
    pub const fn index_bank(&self) -> &TermBank {
        &self.index_bank
    }

    #[must_use]
    pub fn subst(&self) -> Option<&PatternSubst> {
        self.subst.as_deref()
    }

    #[must_use]
    pub fn tsm(&self, id: TsmId) -> Option<&Tsm> {
        self.tsms.get(id)
    }

    #[must_use]
    pub fn tsms(&self) -> &[Tsm] {
        &self.tsms
    }

    pub fn set_local_limit(&mut self, local_limit: bool) {
        self.local_limit = local_limit;
    }

    pub fn set_limit(&mut self, limit: f64) {
        self.limit = limit;
    }

    pub fn set_eval_limit(&mut self, eval_limit: f64) {
        self.eval_limit = eval_limit;
    }

    pub fn set_unmapped_eval(&mut self, unmapped_eval: f64) {
        self.unmapped_eval = unmapped_eval;
    }

    pub fn set_unmapped_weight(&mut self, unmapped_weight: f64) {
        self.unmapped_weight = unmapped_weight;
    }

    fn allocate_base_tsm(&mut self, index_type: IndexType, depth: i32) -> TsmId {
        let subst = self
            .subst
            .clone()
            .unwrap_or_else(|| Rc::new(PatternSubst::new(self.index_bank.signature())));
        let id = self.tsms.len();
        self.tsms.push(Tsm::new_shared(index_type, depth, subst));
        id
    }

    fn required_root_tsm(&self) -> TsmId {
        self.root_tsm
            .unwrap_or_else(|| panic!("TSM admin has no root TSM"))
    }

    fn required_subst_shared(&self) -> Rc<PatternSubst> {
        self.subst.as_ref().map_or_else(
            || panic!("TSM admin has no pattern substitution"),
            Rc::clone,
        )
    }
}

#[must_use]
pub fn get_tsm_type(name: &str) -> Option<TsmType> {
    match TSM_TYPE_NAMES
        .iter()
        .position(|candidate| *candidate == name)?
    {
        0 => Some(TsmType::NoType),
        1 => Some(TsmType::Flat),
        2 => Some(TsmType::Recursive),
        3 => Some(TsmType::Recurrent),
        4 => Some(TsmType::RecurrentLocal),
        _ => None,
    }
}

#[must_use]
pub fn tsm_eval_normalize(eval: f64, limit: f64) -> i32 {
    if eval < limit {
        -1
    } else {
        1
    }
}

#[must_use]
pub fn tsm_flat_anno_set_entropy(set: &FlatAnnoSet, limit: f64) -> f64 {
    let mut pos = 0_i64;
    let mut neg = 0_i64;
    for (_key, entry) in set.iter() {
        if tsm_eval_normalize(entry.val1.eval(), limit) == -1 {
            neg += entry.val1.sources();
        } else {
            pos += entry.val1.sources();
        }
    }
    binary_entropy(pos, neg)
}

#[must_use]
pub fn tsm_remainder_entropy(
    partition: &[Option<FlatAnnoTerm>],
    limit: f64,
    max_index: i64,
) -> (f64, i64) {
    let mut result = 0.0;
    let mut global_count = 0_i64;
    let mut parts = 0_i64;

    for index in 0..=max_index {
        let list = usize::try_from(index)
            .ok()
            .and_then(|array_index| partition.get(array_index))
            .and_then(Option::as_ref);
        let (local_entropy, count) = compute_list_entropy(list, limit);
        if count != 0 {
            parts += 1;
            result += i64_to_f64(count) * local_entropy;
            global_count += count;
        }
    }

    (result / i64_to_f64(global_count), parts)
}

#[must_use]
pub fn tsm_distribution_entropy(partition: &[Option<FlatAnnoTerm>], max_index: i64) -> f64 {
    let mut sum = 0_i64;
    let mut counts = Vec::new();

    for index in 0..=max_index {
        let count = usize::try_from(index)
            .ok()
            .and_then(|array_index| partition.get(array_index))
            .and_then(Option::as_ref)
            .map_or(0, |term| count_list_sources(Some(term)));
        counts.push(count);
        sum += count;
    }

    let mut result = 0.0;
    for count in counts {
        if count != 0 {
            let relfreq = i64_to_f64(count) / i64_to_f64(sum);
            result -= relfreq * relfreq.log2();
        }
    }
    result
}

/// Partitions a flat annotation set by assigning each term to `index(term)`.
///
/// # Errors
///
/// Returns a diagnostic if the underlying term bank rejects a term inserted by
/// the TSM index.
///
/// # Panics
///
/// Panics if an index returns a negative key, or if a cache is requested for a
/// term whose entry number cannot be used as a non-negative dynamic-array index.
pub fn tsm_partition_set(
    partition: &mut TsmPartition,
    index: &mut TSMIndex,
    set: &FlatAnnoSet,
    bank: &mut TermBank,
    mut cache: Option<&mut PDIntArray>,
) -> Result<i64, Diagnostic> {
    let mut max_index = -1_i64;
    for (_key, entry) in set.iter() {
        let current = &entry.val1;
        let key = if let Some(cache) = cache.as_deref_mut() {
            let cache_index = term_entry_pd_index(current.term().entry_no());
            let cached = cache.element_int(cache_index);
            if cached != 0 {
                cached - 1
            } else {
                let inserted = tsm_index_insert(index, current.term(), bank)?;
                cache.assign(cache_index, inserted + 1);
                inserted
            }
        } else {
            tsm_index_insert(index, current.term(), bank)?
        };
        prepend_partition_term(partition, key, current);
        max_index = max_index.max(key);
    }
    Ok(max_index)
}

/// Evaluates the relative information gain of an already allocated TSM index.
///
/// # Errors
///
/// Returns a diagnostic if partitioning needs to insert a representative term
/// that the term bank rejects.
///
/// # Panics
///
/// Panics under the same internal-invariant conditions as [`tsm_partition_set`].
pub fn tsm_evaluate_index(
    set: &FlatAnnoSet,
    index: &mut TSMIndex,
    bank: &mut TermBank,
    cache: Option<&mut PDIntArray>,
    limit: f64,
) -> Result<f64, Diagnostic> {
    let mut partition = TsmPartition::new();
    let max_index = tsm_partition_set(&mut partition, index, set, bank, cache)?;
    let entropy = tsm_flat_anno_set_entropy(set, limit);
    let (remainder, parts) = tsm_remainder_entropy(&partition, limit, max_index);

    if parts == 1 {
        Ok(0.0)
    } else {
        Ok((entropy - remainder)
            / (tsm_distribution_entropy(&partition, max_index) - (entropy - remainder)))
    }
}

/// Evaluates a temporary TSM index described by type and depth.
///
/// # Errors
///
/// Returns a diagnostic if partitioning needs to insert a representative term
/// that the term bank rejects.
///
/// # Panics
///
/// Panics if `index_type` is not a concrete index type accepted by
/// [`tsm_index_alloc`].
pub fn tsm_evaluate_index_desc(
    set: &FlatAnnoSet,
    bank: &mut TermBank,
    subst: &PatternSubst,
    depth: i32,
    index_type: IndexType,
    limit: f64,
) -> Result<f64, Diagnostic> {
    let mut index = tsm_index_alloc(index_type, depth, subst.clone());
    tsm_evaluate_index(set, &mut index, bank, None, limit)
}

/// Finds the best concrete TSM index among the requested bitmask.
///
/// # Errors
///
/// Returns a diagnostic if an evaluated temporary index needs to insert a term
/// representative that the term bank rejects.
///
/// # Panics
///
/// Panics if `index_type` is `IndexNoIndex` or contains bits that do not map to
/// supported concrete C index types.
pub fn tsm_find_optimal_index(
    set: &FlatAnnoSet,
    bank: &mut TermBank,
    subst: &PatternSubst,
    depth: &mut i32,
    index_type: IndexType,
    limit: f64,
) -> Result<IndexType, Diagnostic> {
    assert_ne!(index_type, IndexType::NO_INDEX);
    assert_known_index_mask(index_type);
    let bits = index_type.bits();
    let single_index = bits > 0 && (bits & (bits - 1)) == 0;
    let mut best = -1.0;
    let mut best_index = IndexType::NO_INDEX;
    let mut best_depth = *depth;

    if index_has(index_type, IndexType::ARITY) {
        if single_index {
            best_index = index_type;
        } else {
            let relgain = tsm_evaluate_index_desc(set, bank, subst, 0, IndexType::ARITY, limit)?;
            if relgain > best {
                best = relgain;
                best_index = IndexType::ARITY;
                best_depth = 0;
            }
        }
    }
    if index_has(index_type, IndexType::SYMBOL) {
        if single_index {
            best_index = index_type;
        } else {
            let relgain = tsm_evaluate_index_desc(set, bank, subst, 0, IndexType::SYMBOL, limit)?;
            if relgain > best {
                best = relgain;
                best_index = IndexType::SYMBOL;
                best_depth = 0;
            }
        }
    }
    if index_has(index_type, IndexType::IDENTITY) {
        if single_index {
            best_index = index_type;
        } else {
            let relgain = tsm_evaluate_index_desc(set, bank, subst, 0, IndexType::IDENTITY, limit)?;
            if relgain > best {
                best = relgain;
                best_index = IndexType::IDENTITY;
                best_depth = 0;
            }
        }
    }

    if *depth == INDEX_DYNAMIC_DEPTH {
        for candidate_depth in 1..=TSM_MAX_TERMTOP {
            let mut top_eval = TopIndexEval {
                set,
                bank,
                subst,
                depth: candidate_depth,
                limit,
            };
            let relgain = evaluate_top_index(&mut top_eval, index_type, &mut best_index, best)?;
            if relgain > best {
                best_depth = candidate_depth;
                best = relgain;
            }
        }
    } else if single_index {
        best_index = index_type;
    } else {
        let mut top_eval = TopIndexEval {
            set,
            bank,
            subst,
            depth: *depth,
            limit,
        };
        let relgain = evaluate_top_index(&mut top_eval, index_type, &mut best_index, best)?;
        if relgain > best {
            best_depth = *depth;
        }
    }

    *depth = best_depth;
    Ok(best_index)
}

/// Allocates a C-shaped TSM administration cell.
///
/// # Errors
///
/// Returns a diagnostic if the private index term bank cannot be created.
///
/// # Panics
///
/// Panics if the internal empty TSM allocation violates the TSM index
/// invariants.
pub fn tsm_admin_alloc(sig: Signature, tsm_type: TsmType) -> Result<TsmAdmin, Diagnostic> {
    TsmAdmin::new(sig, tsm_type)
}

/// Builds the TSM selected by the admin type and index description.
///
/// # Errors
///
/// Returns a diagnostic if partitioning or recursive construction inserts an
/// index representative term that the private index bank rejects.
///
/// # Panics
///
/// Panics if `index_type` is `IndexNoIndex`, if the admin type is invalid, or
/// if term/index invariants documented on the lower-level helpers are violated.
pub fn tsm_admin_build_tsm(
    admin: &mut TsmAdmin,
    set: &FlatAnnoSet,
    index_type: IndexType,
    depth: i32,
    subst: PatternSubst,
) -> Result<(), Diagnostic> {
    assert_ne!(index_type, IndexType::NO_INDEX);
    admin.index_type = index_type;
    admin.index_depth = depth;
    admin.subst = Some(Rc::new(subst));
    admin.limit = flat_anno_set_eval_average(set);
    admin.eval_limit = admin.limit;

    match admin.tsm_type {
        TsmType::Recursive | TsmType::Flat => {
            tsm_create(admin, set)?;
        }
        TsmType::Recurrent => {
            let mut flatset = flat_anno_set_alloc();
            flat_anno_set_flatten(&mut flatset, set);
            tsm_create(admin, &flatset)?;
        }
        TsmType::RecurrentLocal => {
            allocate_recurrent_local_base_tsms(admin);
            let mut flatset = flat_anno_set_alloc();
            flat_anno_set_flatten(&mut flatset, set);
            admin.root_tsm = None;
            let mut best_gain = -1.0;
            for stack_index in 0..admin.tsm_stack.len() {
                let tsm_id = admin.tsm_stack[stack_index];
                tsm_complete(admin, tsm_id, &flatset)?;
                let relative_gain = evaluate_stacked_index(admin, stack_index, &flatset)?;
                if relative_gain > best_gain {
                    best_gain = relative_gain;
                    admin.root_tsm = Some(tsm_id);
                }
            }
            assert!(
                admin.root_tsm.is_some(),
                "recurrent-local TSM stack is empty"
            );
        }
        TsmType::NoType => panic!("illegal TSM type in TSMAdminBuildTSM"),
    }
    Ok(())
}

/// Creates a TSM according to the current admin configuration.
///
/// # Errors
///
/// Returns a diagnostic if index selection or partitioning inserts a rejected
/// representative term into the private index bank.
///
/// # Panics
///
/// Panics if the admin lacks a pattern substitution or has an invalid index
/// description.
pub fn tsm_create(admin: &mut TsmAdmin, set: &FlatAnnoSet) -> Result<TsmId, Diagnostic> {
    let mut depth = admin.index_depth;
    let limit = if admin.local_limit {
        flat_anno_set_eval_weighted_average(set)
    } else {
        admin.limit
    };
    let subst = admin.required_subst_shared();
    let index_type = tsm_find_optimal_index(
        set,
        &mut admin.index_bank,
        subst.as_ref(),
        &mut depth,
        admin.index_type,
        limit,
    )?;
    let tsm_id = admin.allocate_base_tsm(index_type, depth);
    if admin.root_tsm.is_none() {
        admin.root_tsm = Some(tsm_id);
    }
    tsm_complete(admin, tsm_id, set)?;
    Ok(tsm_id)
}

/// Creates a term-space annotation for one partition bucket.
///
/// # Errors
///
/// Returns a diagnostic if recursive TSM construction or recurrent-local index
/// evaluation inserts a rejected representative term into the private index
/// bank.
///
/// # Panics
///
/// Panics if the list is internally inconsistent, if the admin type is invalid,
/// or if selected direct subterms are missing.
pub fn tsa_create(admin: &mut TsmAdmin, list: &FlatAnnoTerm) -> Result<Tsa, Diagnostic> {
    let arity = list.term().arity();
    let mut eval = 0.0;
    let mut eval_weight = 0.0;
    let mut current = Some(list);
    while let Some(term) = current {
        assert_eq!(term.term().arity(), arity);
        eval += term.eval_weight() * term.eval();
        eval_weight += term.eval_weight();
        current = term.next();
    }

    let mut arg_tsms = Vec::new();
    if arity != 0 {
        for index in 0..arity {
            let arg_tsm = match admin.tsm_type {
                TsmType::Flat => admin.empty_tsm,
                TsmType::Recursive => {
                    let mut subset = flat_anno_set_alloc();
                    tsm_create_subterm_set(&mut subset, Some(list), index);
                    tsm_create(admin, &subset)?
                }
                TsmType::Recurrent => admin.required_root_tsm(),
                TsmType::RecurrentLocal => select_recurrent_local_arg_tsm(admin, list, index)?,
                TsmType::NoType => panic!("unknown TSM type in TSACreate"),
            };
            arg_tsms.push(arg_tsm);
        }
    }

    Ok(Tsa::new(eval_weight, eval / eval_weight, arity, arg_tsms))
}

/// Evaluates a term with the admin's root TSM.
///
/// # Panics
///
/// Panics if the admin has no root TSM or if index/term arity invariants are
/// violated.
pub fn tsm_eval_term(admin: &mut TsmAdmin, term: &Term, subst: &PatternSubst) -> f64 {
    let mut result = 0.0;
    let eval_weight =
        tsm_rec_eval_no_weight(admin, &mut result, admin.required_root_tsm(), term, subst);
    if eval_weight == 0.0 {
        admin.limit
    } else {
        result / eval_weight
    }
}

/// Computes the classification limit after evaluating every flat term.
///
/// # Panics
///
/// Panics if the admin has no root TSM or no stored substitution.
#[must_use]
pub fn tsm_compute_classification_limit(admin: &mut TsmAdmin, set: &FlatAnnoSet) -> f64 {
    let subst = admin.required_subst_shared();
    let mut pos_eval = 0.0;
    let mut neg_eval = 0.0;
    let mut pos = 0_i64;
    let mut neg = 0_i64;

    for (_key, entry) in set.iter() {
        let flat = &entry.val1;
        let eval = tsm_eval_term(admin, flat.term(), subst.as_ref());
        if flat.eval() < admin.limit {
            pos_eval += eval * i64_to_f64(flat.sources());
            pos += flat.sources();
        } else {
            neg_eval += eval * i64_to_f64(flat.sources());
            neg += flat.sources();
        }
    }

    if pos == 0 && neg == 0 {
        0.0
    } else if pos == 0 {
        neg_eval / i64_to_f64(neg)
    } else if neg == 0 {
        pos_eval / i64_to_f64(pos)
    } else {
        f64::midpoint(pos_eval / i64_to_f64(pos), neg_eval / i64_to_f64(neg))
    }
}

/// Computes the source-weighted average TSM evaluation for a flat set.
///
/// # Panics
///
/// Panics if the admin has no root TSM or no stored substitution.
#[must_use]
pub fn tsm_compute_average_eval(admin: &mut TsmAdmin, set: &FlatAnnoSet) -> f64 {
    if set.is_empty() {
        return 0.0;
    }

    let subst = admin.required_subst_shared();
    let mut eval = 0.0;
    let mut count = 0_i64;
    for (_key, entry) in set.iter() {
        let flat = &entry.val1;
        eval += tsm_eval_term(admin, flat.term(), subst.as_ref()) * i64_to_f64(flat.sources());
        count += flat.sources();
    }
    eval / i64_to_f64(count)
}

/// Prints a TSM's flat TSA distribution in the C debug-comment shape.
///
/// # Panics
///
/// Panics if `tsm_id` is not owned by `admin`.
#[must_use]
pub fn tsm_print_flat_string(admin: &TsmAdmin, tsm_id: TsmId) -> String {
    let tsm = admin
        .tsm(tsm_id)
        .unwrap_or_else(|| panic!("unknown TSM id {tsm_id}"));
    let mut output = String::new();
    for index in 0..=tsm.max_index {
        if let Some(tsa) = tsm_tsa(tsm, index) {
            let _ = writeln!(
                output,
                "# {index:3}: Weight = {:6.3} EvalWeight = {:6.3}",
                tsa.eval(),
                tsa.eval_weight()
            );
        }
    }
    output
}

/// Prints a recursive TSM in the C debug-comment shape.
///
/// # Panics
///
/// Panics if `tsm_id` is not owned by `admin`, or if a recurrent TSM cycle is
/// printed recursively just as the C debug printer would recurse forever.
#[must_use]
pub fn tsm_print_rek_string(admin: &TsmAdmin, tsm_id: TsmId, depth: i32) -> String {
    let tsm = admin
        .tsm(tsm_id)
        .unwrap_or_else(|| panic!("unknown TSM id {tsm_id}"));
    let mut output = tsm_index_print_string(&tsm.index, &admin.index_bank, depth);
    let indent = " ".repeat(usize::try_from(3_i32.saturating_mul(depth)).unwrap_or(0));
    for index in 0..=tsm.max_index {
        if let Some(tsa) = tsm_tsa(tsm, index) {
            let _ = writeln!(
                output,
                "# {indent}{index:4}: Weight = {:7.5} EvalWeight = {:7.5}",
                tsa.eval(),
                tsa.eval_weight()
            );
            for child_tsm in tsa.arg_tsms() {
                output.push_str(&tsm_print_rek_string(admin, *child_tsm, depth + 1));
            }
        }
    }
    output
}

/// Inserts selected direct subterms from a linked flat-annotation list.
///
/// # Panics
///
/// Panics if any listed term has no argument at `sel`, matching the C
/// assertion `term->arity > sel`.
pub fn tsm_create_subterm_set(
    set: &mut FlatAnnoSet,
    list: Option<&FlatAnnoTerm>,
    sel: usize,
) -> i64 {
    let mut count = 0_i64;
    let mut current = list;
    while let Some(term) = current {
        let subterm = term
            .term()
            .argument(sel)
            .expect("selected subterm position must exist");
        let new_term = FlatAnnoTerm::new(subterm, term.eval(), term.eval_weight(), term.sources());
        flat_anno_set_add_term(set, new_term);
        current = term.next();
        count += 1;
    }
    count
}

fn tsm_complete(admin: &mut TsmAdmin, tsm_id: TsmId, set: &FlatAnnoSet) -> Result<(), Diagnostic> {
    let mut partition = TsmPartition::new();
    let max_index = {
        let TsmAdmin {
            index_bank, tsms, ..
        } = admin;
        let tsm = tsms
            .get_mut(tsm_id)
            .unwrap_or_else(|| panic!("unknown TSM id {tsm_id}"));
        tsm_partition_set(&mut partition, &mut tsm.index, set, index_bank, None)?
    };
    admin.tsms[tsm_id].max_index = max_index;

    let mut tsas = PDPointerArray::new_pointer(tsa_array_size(max_index), 2000);
    for index in 0..=max_index {
        if let Some(part) = partition_bucket(&partition, index) {
            let tsa = tsa_create(admin, part)?;
            tsas.assign(pd_index(index), Some(tsa));
        }
    }
    admin.tsms[tsm_id].tsas = Some(tsas);
    Ok(())
}

fn allocate_recurrent_local_base_tsms(admin: &mut TsmAdmin) {
    push_recurrent_local_base_tsm(admin, IndexType::ARITY, 0, 10, 50);
    push_recurrent_local_base_tsm(admin, IndexType::SYMBOL, 0, 10, 50);
    for depth in 1..=TSM_MAX_TERMTOP {
        let depth_size = usize::try_from(depth).expect("TSM term-top depth must fit usize");
        let init = 20 * depth_size * depth_size;
        let grow = 30 * depth_size * depth_size;
        push_recurrent_local_base_tsm(admin, IndexType::TOP, depth, init, grow);
        push_recurrent_local_base_tsm(admin, IndexType::ALT_TOP, depth, init, grow);
        push_recurrent_local_base_tsm(admin, IndexType::CS_TOP, depth, init, grow);
        push_recurrent_local_base_tsm(admin, IndexType::ES_TOP, depth, init, grow);
    }
}

fn push_recurrent_local_base_tsm(
    admin: &mut TsmAdmin,
    index_type: IndexType,
    depth: i32,
    cache_init: usize,
    cache_grow: usize,
) {
    let tsm_id = admin.allocate_base_tsm(index_type, depth);
    admin.tsm_stack.push(tsm_id);
    admin
        .cache_stack
        .push(PDIntArray::new_int(cache_init, cache_grow));
}

fn select_recurrent_local_arg_tsm(
    admin: &mut TsmAdmin,
    list: &FlatAnnoTerm,
    sel: usize,
) -> Result<TsmId, Diagnostic> {
    let mut best_gain = -1.0;
    let mut best_tsm = None;
    let mut subset = flat_anno_set_alloc();
    tsm_create_subterm_set(&mut subset, Some(list), sel);
    for stack_index in 0..admin.tsm_stack.len() {
        let relative_gain = evaluate_stacked_index(admin, stack_index, &subset)?;
        if relative_gain > best_gain {
            best_gain = relative_gain;
            best_tsm = Some(admin.tsm_stack[stack_index]);
        }
    }
    Ok(best_tsm.unwrap_or_else(|| panic!("recurrent-local TSM stack is empty")))
}

fn evaluate_stacked_index(
    admin: &mut TsmAdmin,
    stack_index: usize,
    set: &FlatAnnoSet,
) -> Result<f64, Diagnostic> {
    let limit = admin.limit;
    let TsmAdmin {
        index_bank,
        tsms,
        tsm_stack,
        cache_stack,
        ..
    } = admin;
    let tsm_id = tsm_stack[stack_index];
    let tsm = tsms
        .get_mut(tsm_id)
        .unwrap_or_else(|| panic!("unknown TSM id {tsm_id}"));
    let cache = cache_stack
        .get_mut(stack_index)
        .unwrap_or_else(|| panic!("missing recurrent-local cache {stack_index}"));
    tsm_evaluate_index(set, &mut tsm.index, index_bank, Some(cache), limit)
}

fn tsm_rec_eval(
    admin: &mut TsmAdmin,
    result: &mut f64,
    tsm_id: TsmId,
    term: &Term,
    subst: &PatternSubst,
) -> f64 {
    let tsa = find_tsa_for_term(admin, tsm_id, term, subst);
    if let Some(tsa) = tsa {
        assert_eq!(tsa.arity(), term.arity());
        let mut eval_weight = tsa.eval_weight();
        *result += tsa.eval_weight() * tsa.eval();
        if admin.tsm_type != TsmType::Flat {
            for (index, child_tsm) in tsa.arg_tsms().iter().copied().enumerate() {
                let arg = term
                    .argument(index)
                    .unwrap_or_else(|| panic!("term argument {index} is uninitialized"));
                eval_weight += tsm_rec_eval(admin, result, child_tsm, &arg, subst);
            }
        }
        eval_weight
    } else {
        *result += admin.unmapped_eval * admin.unmapped_weight;
        admin.unmapped_weight
    }
}

fn tsm_rec_eval_no_weight(
    admin: &mut TsmAdmin,
    result: &mut f64,
    tsm_id: TsmId,
    term: &Term,
    subst: &PatternSubst,
) -> f64 {
    let tsa = find_tsa_for_term(admin, tsm_id, term, subst);
    let mut eval_weight = 1.0;
    if let Some(tsa) = tsa {
        assert_eq!(tsa.arity(), term.arity());
        *result += tsa.eval();
        if admin.tsm_type != TsmType::Flat {
            for (index, child_tsm) in tsa.arg_tsms().iter().copied().enumerate() {
                let arg = term
                    .argument(index)
                    .unwrap_or_else(|| panic!("term argument {index} is uninitialized"));
                eval_weight += tsm_rec_eval(admin, result, child_tsm, &arg, subst);
            }
        }
    } else {
        if admin.tsm_type == TsmType::Recursive {
            eval_weight = i64_to_f64(term_weight(term, 1, 1));
        }
        *result += eval_weight * admin.unmapped_eval;
        if admin.tsm_type == TsmType::Recurrent {
            for index in 0..term.arity() {
                let arg = term
                    .argument(index)
                    .unwrap_or_else(|| panic!("term argument {index} is uninitialized"));
                eval_weight += tsm_rec_eval(admin, result, tsm_id, &arg, subst);
            }
        }
    }
    eval_weight
}

fn find_tsa_for_term(
    admin: &mut TsmAdmin,
    tsm_id: TsmId,
    term: &Term,
    subst: &PatternSubst,
) -> Option<Tsa> {
    let TsmAdmin {
        index_bank, tsms, ..
    } = admin;
    let tsm = tsms
        .get_mut(tsm_id)
        .unwrap_or_else(|| panic!("unknown TSM id {tsm_id}"));
    let key = tsm_index_find(&mut tsm.index, term, subst, index_bank);
    if key < 0 {
        return None;
    }
    tsm_tsa(tsm, key).cloned()
}

fn tsm_tsa(tsm: &Tsm, key: i64) -> Option<&Tsa> {
    tsm.tsas
        .as_ref()
        .and_then(|tsas| tsas.existing_element(pd_index(key)))
        .and_then(Option::as_ref)
}

fn partition_bucket(partition: &TsmPartition, key: i64) -> Option<&FlatAnnoTerm> {
    usize::try_from(key)
        .ok()
        .and_then(|index| partition.get(index))
        .and_then(Option::as_ref)
}

fn tsa_array_size(max_index: i64) -> usize {
    usize::try_from(
        max_index
            .checked_add(2)
            .expect("TSM max index overflow for TSA array"),
    )
    .expect("TSM max index must fit usize")
}

fn compute_list_entropy(list: Option<&FlatAnnoTerm>, limit: f64) -> (f64, i64) {
    let mut pos = 0_i64;
    let mut neg = 0_i64;
    let mut current = list;
    while let Some(term) = current {
        if tsm_eval_normalize(term.eval(), limit) == -1 {
            neg += term.sources();
        } else {
            pos += term.sources();
        }
        current = term.next();
    }
    (binary_entropy(pos, neg), pos + neg)
}

fn count_list_sources(list: Option<&FlatAnnoTerm>) -> i64 {
    let mut result = 0_i64;
    let mut current = list;
    while let Some(term) = current {
        result += term.sources();
        current = term.next();
    }
    result
}

fn prepend_partition_term(partition: &mut TsmPartition, key: i64, term: &FlatAnnoTerm) {
    assert!(key >= 0, "partition key must be non-negative");
    let index = usize::try_from(key).expect("partition key must fit usize");
    if partition.len() <= index {
        partition.resize_with(index + 1, || None);
    }
    let old_head = partition[index].take();
    let mut new_head = term.clone();
    new_head.set_next(old_head);
    partition[index] = Some(new_head);
}

fn term_entry_pd_index(entry_no: i64) -> PDArrayIndex {
    assert!(
        entry_no >= 0,
        "term entry number must be non-negative for TSM cache"
    );
    PDArrayIndex::try_from(entry_no).expect("term entry number must fit PDArrayIndex")
}

fn pd_index(value: i64) -> PDArrayIndex {
    PDArrayIndex::try_from(value).unwrap_or(PDArrayIndex::MAX)
}

fn evaluate_top_index(
    context: &mut TopIndexEval<'_, '_>,
    index_type: IndexType,
    best_index: &mut IndexType,
    mut to_beat: f64,
) -> Result<f64, Diagnostic> {
    if index_has(index_type, IndexType::TOP) {
        to_beat = evaluate_top_candidate(context, IndexType::TOP, best_index, to_beat)?;
    }
    if index_has(index_type, IndexType::ALT_TOP) {
        to_beat = evaluate_top_candidate(context, IndexType::ALT_TOP, best_index, to_beat)?;
    }
    if index_has(index_type, IndexType::CS_TOP) {
        to_beat = evaluate_top_candidate(context, IndexType::CS_TOP, best_index, to_beat)?;
    }
    if index_has(index_type, IndexType::ES_TOP) {
        to_beat = evaluate_top_candidate(context, IndexType::ES_TOP, best_index, to_beat)?;
    }
    Ok(to_beat)
}

fn evaluate_top_candidate(
    context: &mut TopIndexEval<'_, '_>,
    candidate: IndexType,
    best_index: &mut IndexType,
    to_beat: f64,
) -> Result<f64, Diagnostic> {
    let relgain = tsm_evaluate_index_desc(
        context.set,
        context.bank,
        context.subst,
        context.depth,
        candidate,
        context.limit,
    )?;
    if relgain > to_beat {
        *best_index = candidate;
        Ok(relgain)
    } else {
        Ok(to_beat)
    }
}

fn index_has(index_type: IndexType, flag: IndexType) -> bool {
    index_type.bits() & flag.bits() != 0
}

fn assert_known_index_mask(index_type: IndexType) {
    let known = IndexType::DYNAMIC.bits() | IndexType::EMPTY.bits();
    assert_eq!(index_type.bits() & !known, 0, "unknown TSM index type bit");
}

fn binary_entropy(pos: i64, neg: i64) -> f64 {
    if pos == 0 || neg == 0 {
        return 0.0;
    }

    let total = i64_to_f64(pos + neg);
    let pos_freq = i64_to_f64(pos) / total;
    let neg_freq = i64_to_f64(neg) / total;
    pos_freq * -pos_freq.log2() - neg_freq * neg_freq.log2()
}

#[allow(clippy::cast_precision_loss)]
fn i64_to_f64(value: i64) -> f64 {
    value as f64
}

#[cfg(test)]
mod tests {
    use super::{
        get_tsm_type, tsm_admin_alloc, tsm_admin_build_tsm, tsm_compute_average_eval,
        tsm_compute_classification_limit, tsm_create_subterm_set, tsm_distribution_entropy,
        tsm_eval_normalize, tsm_eval_term, tsm_evaluate_index, tsm_find_optimal_index,
        tsm_flat_anno_set_entropy, tsm_partition_set, tsm_print_flat_string, tsm_remainder_entropy,
        TsmPartition, TsmType, TSM_MAX_TERMTOP, TSM_TYPE_NAMES,
    };
    use crate::basics::pdarrays::PDIntArray;
    use crate::inout::scanner::Scanner;
    use crate::learn::flatannoterms::{flat_anno_set_add_term, flat_anno_set_alloc, FlatAnnoTerm};
    use crate::learn::indexfunctions::{tsm_index_alloc, IndexType, INDEX_DYNAMIC_DEPTH};
    use crate::learn::patterns::{pattern_term_compute, PatternSubst};
    use crate::terms::signature::Signature;
    use crate::terms::termbanks::TermBank;
    use crate::terms::termtypes::Term;
    use crate::terms::typebanks::TypeBank;

    fn assert_close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 1.0e-12,
            "expected {expected}, got {actual}"
        );
    }

    fn term(entry_no: i64) -> Term {
        let term = Term::const_cell_alloc(entry_no + 20);
        term.set_entry_no(entry_no);
        term
    }

    fn parse_in_bank(bank: &mut TermBank, source: &str) -> Term {
        let mut scanner = Scanner::from_user_string(source, false).expect("scanner allocation");
        bank.parse_term_simple(&mut scanner)
            .expect("simple term parse")
    }

    fn bound_subst(bank: &TermBank, terms: &[&Term]) -> PatternSubst {
        let mut subst = PatternSubst::new(bank.signature());
        for term in terms {
            pattern_term_compute(&mut subst, term);
        }
        subst
    }

    fn list_names(list: Option<&FlatAnnoTerm>, bank: &TermBank) -> Vec<String> {
        let mut result = Vec::new();
        let mut current = list;
        while let Some(term) = current {
            result.push(bank.term_string(term.term(), true));
            current = term.next();
        }
        result
    }

    #[test]
    fn tsm_type_names_and_discriminants_match_c_surface() {
        assert_eq!(
            TSM_TYPE_NAMES,
            ["NoType", "Flat", "Recursive", "Recurrent", "RecLocal"]
        );
        assert_eq!(TsmType::NoType as i32, 0);
        assert_eq!(TsmType::Flat as i32, 1);
        assert_eq!(TsmType::Recursive as i32, 2);
        assert_eq!(TsmType::Recurrent as i32, 3);
        assert_eq!(TsmType::RecurrentLocal as i32, 4);
        assert_eq!(TSM_MAX_TERMTOP, 5);
        assert_eq!(get_tsm_type("Flat"), Some(TsmType::Flat));
        assert_eq!(get_tsm_type("RecLocal"), Some(TsmType::RecurrentLocal));
        assert_eq!(get_tsm_type("missing"), None);
    }

    #[test]
    fn admin_alloc_initializes_empty_tsm_and_defaults_like_c() {
        let admin = tsm_admin_alloc(Signature::new(TypeBank::new()), TsmType::Flat)
            .expect("admin allocation");

        assert_eq!(admin.tsm_type(), TsmType::Flat);
        assert_eq!(admin.index_type(), IndexType::NO_INDEX);
        assert_eq!(admin.index_depth(), 0);
        assert_close(admin.limit(), 0.0);
        assert_close(admin.eval_limit(), 0.0);
        assert_close(admin.unmapped_eval(), 0.0);
        assert_close(admin.unmapped_weight(), 0.0);
        assert!(admin.local_limit());
        assert_eq!(admin.root_tsm(), None);
        assert_eq!(admin.empty_tsm(), 0);
        assert!(admin.tsm_stack().is_empty());
        assert!(admin.cache_stack().is_empty());

        let empty = admin.tsm(admin.empty_tsm()).expect("empty tsm");
        assert_eq!(empty.index().index_type(), IndexType::EMPTY);
        assert_eq!(empty.max_index(), -1);
        assert!(empty.tsas().is_none());
    }

    #[test]
    fn eval_normalize_uses_strict_less_than_limit() {
        assert_eq!(tsm_eval_normalize(0.99, 1.0), -1);
        assert_eq!(tsm_eval_normalize(1.0, 1.0), 1);
        assert_eq!(tsm_eval_normalize(1.01, 1.0), 1);
    }

    #[test]
    fn flat_set_entropy_weights_by_sources() {
        let mut set = flat_anno_set_alloc();
        flat_anno_set_add_term(&mut set, FlatAnnoTerm::new(term(1), 0.0, 1.0, 1));
        flat_anno_set_add_term(&mut set, FlatAnnoTerm::new(term(2), 2.0, 1.0, 3));

        let entropy = tsm_flat_anno_set_entropy(&set, 1.0);

        assert_close(entropy, 0.811_278_124_459_132_8);
    }

    #[test]
    fn remainder_entropy_uses_non_empty_partition_count_and_weighted_average() {
        let mut mixed = FlatAnnoTerm::new(term(1), 0.0, 1.0, 1);
        mixed.set_next(Some(FlatAnnoTerm::new(term(2), 2.0, 1.0, 1)));
        let pure = FlatAnnoTerm::new(term(3), 3.0, 1.0, 3);
        let partition = vec![Some(mixed), Some(pure), None];

        let (entropy, parts) = tsm_remainder_entropy(&partition, 1.0, 2);

        assert_eq!(parts, 2);
        assert_close(entropy, 0.4);
    }

    #[test]
    fn remainder_entropy_empty_partition_matches_c_nan_result() {
        let (entropy, parts) = tsm_remainder_entropy(&[], 1.0, -1);

        assert_eq!(parts, 0);
        assert!(entropy.is_nan());
    }

    #[test]
    fn distribution_entropy_uses_bucket_source_counts() {
        let mut first = FlatAnnoTerm::new(term(1), 0.0, 1.0, 1);
        first.set_next(Some(FlatAnnoTerm::new(term(2), 2.0, 1.0, 2)));
        let second = FlatAnnoTerm::new(term(3), 3.0, 1.0, 1);
        let partition = vec![Some(first), Some(second)];

        assert_close(
            tsm_distribution_entropy(&partition, 1),
            0.811_278_124_459_132_8,
        );
    }

    #[test]
    fn partition_set_assigns_terms_to_index_buckets_and_prepends_lists() {
        let mut bank =
            TermBank::new(Signature::new(TypeBank::new())).expect("term bank allocation");
        let constant = parse_in_bank(&mut bank, "a");
        let unary_a = parse_in_bank(&mut bank, "f(a)");
        let unary_b = parse_in_bank(&mut bank, "g(b)");
        let subst = bound_subst(&bank, &[&constant, &unary_a, &unary_b]);
        let mut set = flat_anno_set_alloc();
        flat_anno_set_add_term(&mut set, FlatAnnoTerm::new(constant, 0.0, 1.0, 1));
        flat_anno_set_add_term(&mut set, FlatAnnoTerm::new(unary_a, 2.0, 1.0, 1));
        flat_anno_set_add_term(&mut set, FlatAnnoTerm::new(unary_b, 3.0, 1.0, 1));
        let mut index = tsm_index_alloc(IndexType::ARITY, 0, subst);
        let mut partition = TsmPartition::new();

        let max_index = tsm_partition_set(&mut partition, &mut index, &set, &mut bank, None)
            .expect("partitioning succeeds");

        assert_eq!(max_index, 1);
        assert_eq!(list_names(partition[0].as_ref(), &bank), vec!["a"]);
        assert_eq!(
            list_names(partition[1].as_ref(), &bank),
            vec!["g(b)", "f(a)"]
        );
    }

    #[test]
    fn partition_set_uses_one_based_term_entry_cache() {
        let mut bank =
            TermBank::new(Signature::new(TypeBank::new())).expect("term bank allocation");
        let first = parse_in_bank(&mut bank, "f(a)");
        let second = parse_in_bank(&mut bank, "g(a)");
        let subst = bound_subst(&bank, &[&first, &second]);
        let mut set = flat_anno_set_alloc();
        flat_anno_set_add_term(&mut set, FlatAnnoTerm::new(first.clone(), 0.0, 1.0, 1));
        flat_anno_set_add_term(&mut set, FlatAnnoTerm::new(second.clone(), 2.0, 1.0, 1));
        let mut first_index = tsm_index_alloc(IndexType::SYMBOL, 0, subst.clone());
        let mut cache = PDIntArray::new_int(10, 50);
        let mut partition = TsmPartition::new();

        assert_eq!(
            tsm_partition_set(
                &mut partition,
                &mut first_index,
                &set,
                &mut bank,
                Some(&mut cache),
            )
            .expect("cached partition"),
            1
        );
        let first_cache = cache.element_int(isize::try_from(first.entry_no()).unwrap());
        let second_cache = cache.element_int(isize::try_from(second.entry_no()).unwrap());
        assert_eq!(first_cache, 1);
        assert_eq!(second_cache, 2);

        let mut second_index = tsm_index_alloc(IndexType::SYMBOL, 0, subst);
        let mut cached_partition = TsmPartition::new();
        assert_eq!(
            tsm_partition_set(
                &mut cached_partition,
                &mut second_index,
                &set,
                &mut bank,
                Some(&mut cache),
            )
            .expect("reused cache partition"),
            1
        );
        assert_eq!(second_index.count(), 0);
    }

    #[test]
    fn evaluate_index_returns_zero_for_single_non_empty_partition() {
        let mut bank =
            TermBank::new(Signature::new(TypeBank::new())).expect("term bank allocation");
        let first = parse_in_bank(&mut bank, "a");
        let second = parse_in_bank(&mut bank, "b");
        let subst = bound_subst(&bank, &[&first, &second]);
        let mut set = flat_anno_set_alloc();
        flat_anno_set_add_term(&mut set, FlatAnnoTerm::new(first, 0.0, 1.0, 1));
        flat_anno_set_add_term(&mut set, FlatAnnoTerm::new(second, 2.0, 1.0, 1));
        let mut index = tsm_index_alloc(IndexType::ARITY, 0, subst);

        let gain =
            tsm_evaluate_index(&set, &mut index, &mut bank, None, 1.0).expect("index evaluation");

        assert_close(gain, 0.0);
    }

    #[test]
    fn evaluate_index_preserves_c_infinite_gain_for_perfect_binary_split() {
        let mut bank =
            TermBank::new(Signature::new(TypeBank::new())).expect("term bank allocation");
        let first = parse_in_bank(&mut bank, "a");
        let second = parse_in_bank(&mut bank, "b");
        let subst = bound_subst(&bank, &[&first, &second]);
        let mut set = flat_anno_set_alloc();
        flat_anno_set_add_term(&mut set, FlatAnnoTerm::new(first, 0.0, 1.0, 1));
        flat_anno_set_add_term(&mut set, FlatAnnoTerm::new(second, 2.0, 1.0, 1));
        let mut index = tsm_index_alloc(IndexType::IDENTITY, 0, subst);

        let gain =
            tsm_evaluate_index(&set, &mut index, &mut bank, None, 1.0).expect("index evaluation");

        assert!(gain.is_infinite());
        assert!(gain.is_sign_positive());
    }

    #[test]
    fn find_optimal_single_non_top_index_preserves_input_depth() {
        let mut bank =
            TermBank::new(Signature::new(TypeBank::new())).expect("term bank allocation");
        let first = parse_in_bank(&mut bank, "a");
        let second = parse_in_bank(&mut bank, "b");
        let subst = bound_subst(&bank, &[&first, &second]);
        let mut set = flat_anno_set_alloc();
        flat_anno_set_add_term(&mut set, FlatAnnoTerm::new(first, 0.0, 1.0, 1));
        flat_anno_set_add_term(&mut set, FlatAnnoTerm::new(second, 2.0, 1.0, 1));
        let mut depth = 3;

        let selected =
            tsm_find_optimal_index(&set, &mut bank, &subst, &mut depth, IndexType::ARITY, 1.0)
                .expect("optimal index");

        assert_eq!(selected, IndexType::ARITY);
        assert_eq!(depth, 3);
    }

    #[test]
    fn find_optimal_dynamic_mask_uses_c_evaluation_order_for_ties() {
        let mut bank =
            TermBank::new(Signature::new(TypeBank::new())).expect("term bank allocation");
        let first = parse_in_bank(&mut bank, "a");
        let second = parse_in_bank(&mut bank, "b");
        let subst = bound_subst(&bank, &[&first, &second]);
        let mut set = flat_anno_set_alloc();
        flat_anno_set_add_term(&mut set, FlatAnnoTerm::new(first, 0.0, 1.0, 1));
        flat_anno_set_add_term(&mut set, FlatAnnoTerm::new(second, 2.0, 1.0, 1));
        let mut depth = INDEX_DYNAMIC_DEPTH;

        let selected =
            tsm_find_optimal_index(&set, &mut bank, &subst, &mut depth, IndexType::DYNAMIC, 1.0)
                .expect("optimal dynamic index");

        assert_eq!(selected, IndexType::SYMBOL);
        assert_eq!(depth, 0);
    }

    #[test]
    fn find_optimal_single_top_with_dynamic_depth_searches_depths() {
        let mut bank =
            TermBank::new(Signature::new(TypeBank::new())).expect("term bank allocation");
        let first = parse_in_bank(&mut bank, "a");
        let second = parse_in_bank(&mut bank, "b");
        let subst = bound_subst(&bank, &[&first, &second]);
        let mut set = flat_anno_set_alloc();
        flat_anno_set_add_term(&mut set, FlatAnnoTerm::new(first, 0.0, 1.0, 1));
        flat_anno_set_add_term(&mut set, FlatAnnoTerm::new(second, 2.0, 1.0, 1));
        let mut depth = INDEX_DYNAMIC_DEPTH;

        let selected =
            tsm_find_optimal_index(&set, &mut bank, &subst, &mut depth, IndexType::TOP, 1.0)
                .expect("optimal top index");

        assert_eq!(selected, IndexType::TOP);
        assert_eq!(depth, 1);
    }

    #[test]
    fn flat_tsm_builds_tsa_array_and_evaluates_training_terms() {
        let mut bank =
            TermBank::new(Signature::new(TypeBank::new())).expect("term bank allocation");
        let low = parse_in_bank(&mut bank, "f(a)");
        let high = parse_in_bank(&mut bank, "g(b)");
        let subst = bound_subst(&bank, &[&low, &high]);
        let mut set = flat_anno_set_alloc();
        flat_anno_set_add_term(&mut set, FlatAnnoTerm::new(low.clone(), 0.0, 1.0, 1));
        flat_anno_set_add_term(&mut set, FlatAnnoTerm::new(high.clone(), 10.0, 1.0, 1));
        let mut admin =
            tsm_admin_alloc(bank.signature().clone(), TsmType::Flat).expect("admin allocation");

        tsm_admin_build_tsm(&mut admin, &set, IndexType::ARITY, 0, subst.clone())
            .expect("flat TSM build");

        assert_eq!(admin.index_type(), IndexType::ARITY);
        assert_eq!(admin.index_depth(), 0);
        assert_close(admin.limit(), 5.0);
        assert_close(admin.eval_limit(), 5.0);
        let root_id = admin.root_tsm().expect("root tsm");
        let root = admin.tsm(root_id).expect("root tsm entry");
        assert_eq!(root.max_index(), 1);
        let arity_bucket = root
            .tsas()
            .and_then(|tsas| tsas.existing_element(1))
            .and_then(Option::as_ref)
            .expect("arity-one TSA");
        assert_eq!(arity_bucket.arity(), 1);
        assert_eq!(arity_bucket.arg_tsms(), &[admin.empty_tsm()]);
        assert_close(arity_bucket.eval(), 5.0);
        assert_close(arity_bucket.eval_weight(), 2.0);

        assert_close(tsm_eval_term(&mut admin, &low, &subst), 5.0);
        assert_close(tsm_eval_term(&mut admin, &high, &subst), 5.0);
        assert_close(tsm_compute_average_eval(&mut admin, &set), 5.0);
        assert_close(tsm_compute_classification_limit(&mut admin, &set), 5.0);
        let flat_print = tsm_print_flat_string(&admin, root_id);
        assert!(flat_print.contains("#   1: Weight =  5.000 EvalWeight =  2.000"));
    }

    #[test]
    fn recurrent_local_builds_c_ordered_stack_and_caches() {
        let mut bank =
            TermBank::new(Signature::new(TypeBank::new())).expect("term bank allocation");
        let left = parse_in_bank(&mut bank, "a");
        let right = parse_in_bank(&mut bank, "b");
        let subst = bound_subst(&bank, &[&left, &right]);
        let mut set = flat_anno_set_alloc();
        flat_anno_set_add_term(&mut set, FlatAnnoTerm::new(left, 0.0, 1.0, 1));
        flat_anno_set_add_term(&mut set, FlatAnnoTerm::new(right, 10.0, 1.0, 1));
        let mut admin = tsm_admin_alloc(bank.signature().clone(), TsmType::RecurrentLocal)
            .expect("admin allocation");

        tsm_admin_build_tsm(&mut admin, &set, IndexType::DYNAMIC, 0, subst)
            .expect("recurrent-local TSM build");

        assert_eq!(admin.tsm_stack().len(), 22);
        assert_eq!(admin.cache_stack().len(), 22);
        let shared_subst = admin.subst.as_ref().expect("stored substitution");
        assert!(admin
            .tsms
            .iter()
            .skip(1)
            .all(|tsm| tsm.index.shares_subst(shared_subst)));
        assert!(admin
            .root_tsm()
            .is_some_and(|root| admin.tsm_stack().contains(&root)));
        assert_eq!(
            admin
                .tsm(admin.tsm_stack()[0])
                .unwrap()
                .index()
                .index_type(),
            IndexType::ARITY
        );
        assert_eq!(
            admin
                .tsm(admin.tsm_stack()[1])
                .unwrap()
                .index()
                .index_type(),
            IndexType::SYMBOL
        );
        assert_eq!(
            admin
                .tsm(admin.tsm_stack()[2])
                .unwrap()
                .index()
                .index_type(),
            IndexType::TOP
        );
        assert_eq!(admin.tsm(admin.tsm_stack()[2]).unwrap().index().depth(), 1);
        assert_eq!(admin.cache_stack()[0].size(), 10);
        assert_eq!(admin.cache_stack()[1].size(), 10);
        assert_eq!(admin.cache_stack()[2].size(), 20);
        assert_eq!(admin.cache_stack()[6].size(), 80);
    }

    #[test]
    fn create_subterm_set_inserts_selected_direct_subterms() {
        let mut bank =
            TermBank::new(Signature::new(TypeBank::new())).expect("term bank allocation");
        let root = parse_in_bank(&mut bank, "f(a,b)");
        let mut list = FlatAnnoTerm::new(root, 7.0, 2.0, 3);
        list.set_next(Some(FlatAnnoTerm::new(
            parse_in_bank(&mut bank, "g(c,b)"),
            5.0,
            4.0,
            1,
        )));
        let mut set = flat_anno_set_alloc();

        assert_eq!(tsm_create_subterm_set(&mut set, Some(&list), 1), 2);

        let names = set
            .iter()
            .map(|(_key, entry)| bank.term_string(entry.val1.term(), true))
            .collect::<Vec<_>>();
        assert_eq!(names, vec!["b"]);
        let stored = set.iter().next().expect("merged b subterm").1;
        assert_close(stored.val1.eval(), 34.0 / 6.0);
        assert_close(stored.val1.eval_weight(), 6.0);
        assert_eq!(stored.val1.sources(), 4);
    }
}
