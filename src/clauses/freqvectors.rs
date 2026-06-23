use crate::basics::fixdarrays::FixedDArray;
use crate::clauses::clause::Clause;
use crate::terms::functypes::FunCode;
use crate::terms::signature::Signature;
use std::collections::BTreeSet;
use std::fmt::Write as _;

pub type PermVector = FixedDArray;

pub const FVINDEX_MAX_FEATURES_DEFAULT: usize = 17;
pub const FVINDEX_SYMBOL_SLACK_DEFAULT: usize = 0;
pub const FV_CLAUSE_FEATURES: usize = 2;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i32)]
pub enum FvIndexType {
    NoFeatures = 0,
    AcFeatures = 1,
    SsFeatures = 2,
    AllFeatures = 3,
    BillFeatures = 4,
    BillPlusFeatures = 5,
    AcFold = 6,
    AcStagger = 7,
    CollectFeatures = 8,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct FvOverflowSpec {
    pub base: i64,
    pub offset: i64,
    pub modulus: i64,
}

impl FvOverflowSpec {
    #[must_use]
    pub const fn new(base: i64, offset: i64, modulus: i64) -> Self {
        Self {
            base,
            offset,
            modulus,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FvCollectLayout {
    pub features: FvIndexType,
    pub use_litcount: bool,
    pub assembly_len: usize,
    pub result_len: usize,
    pub pos_count: FvOverflowSpec,
    pub neg_count: FvOverflowSpec,
    pub pos_depth: FvOverflowSpec,
    pub neg_depth: FvOverflowSpec,
}

impl FvCollectLayout {
    #[must_use]
    pub const fn new(
        features: FvIndexType,
        use_litcount: bool,
        assembly_len: usize,
        result_len: usize,
    ) -> Self {
        Self {
            features,
            use_litcount,
            assembly_len,
            result_len,
            pos_count: FvOverflowSpec::new(0, 0, 0),
            neg_count: FvOverflowSpec::new(0, 0, 0),
            pos_depth: FvOverflowSpec::new(0, 0, 0),
            neg_depth: FvOverflowSpec::new(0, 0, 0),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FvCollect {
    features: FvIndexType,
    use_litcount: bool,
    assembly_vector: Vec<i64>,
    result_len: usize,
    pos_count: FvOverflowSpec,
    neg_count: FvOverflowSpec,
    pos_depth: FvOverflowSpec,
    neg_depth: FvOverflowSpec,
    max_symbols: usize,
}

impl FvCollect {
    #[must_use]
    pub fn new(layout: FvCollectLayout) -> Self {
        Self {
            features: layout.features,
            use_litcount: layout.use_litcount,
            assembly_vector: vec![-1; layout.assembly_len],
            result_len: layout.result_len,
            pos_count: layout.pos_count,
            neg_count: layout.neg_count,
            pos_depth: layout.pos_depth,
            neg_depth: layout.neg_depth,
            max_symbols: FVINDEX_MAX_FEATURES_DEFAULT,
        }
    }

    #[must_use]
    pub const fn features(&self) -> FvIndexType {
        self.features
    }

    #[must_use]
    pub const fn use_litcount(&self) -> bool {
        self.use_litcount
    }

    #[must_use]
    pub fn assembly_vector(&self) -> &[i64] {
        &self.assembly_vector
    }

    pub fn assembly_vector_mut(&mut self) -> &mut [i64] {
        &mut self.assembly_vector
    }

    #[must_use]
    pub const fn result_len(&self) -> usize {
        self.result_len
    }

    #[must_use]
    pub const fn pos_count_overflow(&self) -> FvOverflowSpec {
        self.pos_count
    }

    #[must_use]
    pub const fn neg_count_overflow(&self) -> FvOverflowSpec {
        self.neg_count
    }

    #[must_use]
    pub const fn pos_depth_overflow(&self) -> FvOverflowSpec {
        self.pos_depth
    }

    #[must_use]
    pub const fn neg_depth_overflow(&self) -> FvOverflowSpec {
        self.neg_depth
    }

    #[must_use]
    pub const fn max_symbols(&self) -> usize {
        self.max_symbols
    }

    pub const fn set_max_symbols(&mut self, max_symbols: usize) {
        self.max_symbols = max_symbols;
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FreqVector {
    array: Vec<i64>,
    clause_ident: Option<i64>,
}

impl FreqVector {
    #[must_use]
    pub fn new(size: usize) -> Self {
        Self {
            array: vec![0; size],
            clause_ident: None,
        }
    }

    #[must_use]
    pub fn from_values(values: Vec<i64>) -> Self {
        Self {
            array: values,
            clause_ident: None,
        }
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.array.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.array.is_empty()
    }

    #[must_use]
    pub fn as_slice(&self) -> &[i64] {
        &self.array
    }

    #[must_use]
    pub fn as_mut_slice(&mut self) -> &mut [i64] {
        &mut self.array
    }

    #[must_use]
    pub const fn clause_ident(&self) -> Option<i64> {
        self.clause_ident
    }

    pub const fn set_clause_ident(&mut self, clause_ident: Option<i64>) {
        self.clause_ident = clause_ident;
    }

    pub fn initialize(&mut self, value: i64) {
        self.array.fill(value);
    }

    /// Component-wise addition of two frequency vectors.
    ///
    /// # Panics
    ///
    /// Panics if the source and destination vector sizes differ, matching the C
    /// assertion contract.
    pub fn add_from(&mut self, left: &Self, right: &Self) {
        self.assert_compatible(left, right);
        for ((dest, left), right) in self.array.iter_mut().zip(&left.array).zip(&right.array) {
            *dest = *left + *right;
        }
    }

    /// Component-wise weighted addition of two frequency vectors.
    ///
    /// # Panics
    ///
    /// Panics if the source and destination vector sizes differ, matching the C
    /// assertion contract.
    pub fn mul_add_from(&mut self, left: &Self, left_factor: i64, right: &Self, right_factor: i64) {
        self.assert_compatible(left, right);
        for ((dest, left), right) in self.array.iter_mut().zip(&left.array).zip(&right.array) {
            *dest = left_factor * *left + right_factor * *right;
        }
    }

    /// Component-wise maximum of two frequency vectors.
    ///
    /// # Panics
    ///
    /// Panics if the source and destination vector sizes differ, matching the C
    /// assertion contract.
    pub fn max_from(&mut self, left: &Self, right: &Self) {
        self.assert_compatible(left, right);
        for ((dest, left), right) in self.array.iter_mut().zip(&left.array).zip(&right.array) {
            *dest = (*left).max(*right);
        }
    }

    /// Component-wise minimum of two frequency vectors.
    ///
    /// # Panics
    ///
    /// Panics if the source and destination vector sizes differ, matching the C
    /// assertion contract.
    pub fn min_from(&mut self, left: &Self, right: &Self) {
        self.assert_compatible(left, right);
        for ((dest, left), right) in self.array.iter_mut().zip(&left.array).zip(&right.array) {
            *dest = (*left).min(*right);
        }
    }

    #[must_use]
    pub fn print_string(&self) -> String {
        let mut result = match self.clause_ident {
            Some(ident) => format!("% FV for clause #{ident}.\n"),
            None => "% FV, no clause given.\n".to_owned(),
        };
        let _ = write!(&mut result, "% FV(len={}):", self.len());
        for value in &self.array {
            let _ = write!(&mut result, " {value}");
        }
        result.push('\n');
        result
    }

    fn assert_compatible(&self, left: &Self, right: &Self) {
        assert_eq!(
            left.len(),
            self.len(),
            "left frequency vector size must match destination"
        );
        assert_eq!(
            right.len(),
            self.len(),
            "right frequency vector size must match destination"
        );
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FvPackedClause {
    vector: Option<FreqVector>,
    clause: Clause,
}

impl FvPackedClause {
    #[must_use]
    pub const fn vector(&self) -> Option<&FreqVector> {
        self.vector.as_ref()
    }

    #[must_use]
    pub const fn clause(&self) -> &Clause {
        &self.clause
    }

    pub fn clause_mut(&mut self) -> &mut Clause {
        &mut self.clause
    }

    #[must_use]
    pub fn into_clause(self) -> Clause {
        self.clause
    }

    #[must_use]
    pub fn into_parts(self) -> (Option<FreqVector>, Clause) {
        (self.vector, self.clause)
    }
}

#[must_use]
pub const fn fv_ac_compat_size(size: usize) -> usize {
    (size + 1) * 2 + FV_CLAUSE_FEATURES
}

#[must_use]
pub const fn fv_ss_compat_size(size: usize) -> usize {
    (size + 1) * 2
}

#[must_use]
pub const fn fv_full_size(size: usize) -> usize {
    (size + 1) * 4 + FV_CLAUSE_FEATURES
}

#[must_use]
pub const fn fv_size(size: usize, features: FvIndexType) -> usize {
    match features {
        FvIndexType::AcFeatures => fv_ac_compat_size(size),
        FvIndexType::SsFeatures => fv_ss_compat_size(size),
        _ => fv_full_size(size),
    }
}

/// Computes the feature permutation/selection vector used by FV indexing.
///
/// # Panics
///
/// Panics if the input vectors have different sizes or are empty.
#[must_use]
pub fn perm_vector_compute_internal(
    fmax: &FreqVector,
    fmin: &FreqVector,
    fsum: &FreqVector,
    max_len: usize,
    eliminate_uninformative: bool,
) -> PermVector {
    assert_eq!(fsum.len(), fmax.len(), "sum/max vector sizes must match");
    assert_eq!(fsum.len(), fmin.len(), "sum/min vector sizes must match");
    assert!(
        !fsum.is_empty(),
        "permutation vector input must be non-empty"
    );

    let mut tuples: Vec<Tuple3> = (0..fsum.len())
        .map(|pos| Tuple3 {
            pos,
            diff: fmax.as_slice()[pos] - fmin.as_slice()[pos],
            value: fsum.as_slice()[pos],
        })
        .collect();
    tuples.sort_by(|left, right| {
        left.diff
            .cmp(&right.diff)
            .then_with(|| left.value.cmp(&right.value))
            .then_with(|| right.pos.cmp(&left.pos))
    });

    let mut start = 0;
    if fsum.len() > max_len {
        start = fsum.len() - max_len;
    }
    if eliminate_uninformative {
        let start1 = tuples
            .iter()
            .position(|tuple| tuple.diff != 0)
            .unwrap_or(fsum.len());
        start = start.max(start1);
    }
    if start == fsum.len() {
        start -= 1;
    }

    let mut result = PermVector::new(fsum.len() - start);
    for (slot, tuple) in result.as_mut_slice().iter_mut().zip(&tuples[start..]) {
        *slot = usize_to_i64(tuple.pos, "feature position fits in C long");
    }
    result
}

/// Adds literal-count and symbol-frequency/depth values to an existing vector.
///
/// # Panics
///
/// Panics if `features` is not one of AC/SS/All, or if `vec` does not have the
/// exact C-compatible size for `symbols` and `features`.
pub fn var_freq_vector_add_vals(
    vec: &mut FreqVector,
    symbols: usize,
    features: FvIndexType,
    clause: &Clause,
) {
    assert!(
        matches!(
            features,
            FvIndexType::AcFeatures | FvIndexType::SsFeatures | FvIndexType::AllFeatures
        ),
        "classic frequency-vector features must be AC, SS, or All"
    );
    assert_eq!(
        vec.len(),
        fv_size(symbols, features),
        "frequency vector size must match selected feature layout"
    );

    match features {
        FvIndexType::AcFeatures => add_ac_features(vec, symbols, clause),
        FvIndexType::SsFeatures => add_ss_features(vec, symbols, clause),
        FvIndexType::AllFeatures => add_all_features(vec, symbols, clause),
        _ => unreachable!("feature kind was asserted above"),
    }
}

/// Computes a variable-frequency vector for `clause` under `cspec`.
///
/// # Panics
///
/// Panics if `cspec` uses a feature kind that the C computation routine does
/// not accept.
#[must_use]
pub fn var_freq_vector_compute(clause: &Clause, cspec: &FvCollect) -> FreqVector {
    assert!(
        matches!(
            cspec.features(),
            FvIndexType::AcFeatures
                | FvIndexType::SsFeatures
                | FvIndexType::AllFeatures
                | FvIndexType::CollectFeatures
        ),
        "frequency-vector computation requires AC, SS, All, or Collect features"
    );

    if cspec.features() == FvIndexType::CollectFeatures {
        return fv_collect_freq_vector_compute(clause, cspec);
    }

    let mut vec = FreqVector::new(fv_size(cspec.max_symbols(), cspec.features()));
    vec.set_clause_ident(Some(clause.ident()));
    var_freq_vector_add_vals(&mut vec, cspec.max_symbols(), cspec.features(), clause);
    vec
}

/// Computes an optimized frequency vector by applying an optional permutation.
///
/// # Panics
///
/// Panics if `perm` contains a negative or out-of-range source index.
#[must_use]
pub fn optimized_var_freq_vector_compute(
    clause: &Clause,
    perm: Option<&PermVector>,
    cspec: &FvCollect,
) -> FreqVector {
    let vec = var_freq_vector_compute(clause, cspec);
    let Some(perm) = perm else {
        return vec;
    };

    let mut result = FreqVector::new(perm.size());
    result.set_clause_ident(vec.clause_ident());
    for (index, source) in perm.as_slice().iter().copied().enumerate() {
        assert!(
            source >= 0,
            "permutation vector positions must be non-negative"
        );
        let source = i64_to_usize(source, "permutation vector position fits usize");
        assert!(
            source < vec.len(),
            "permutation vector position must address source vector"
        );
        result.as_mut_slice()[index] = vec.as_slice()[source];
    }
    result
}

/// Computes a collect-style frequency vector from a full four-slot feature vector.
///
/// # Panics
///
/// Panics if `cspec` requests literal counts without at least two result slots,
/// if mapped result indexes are negative or out of range, or if feature-array
/// size arithmetic overflows.
#[must_use]
pub fn fv_collect_freq_vector_compute(clause: &Clause, cspec: &FvCollect) -> FreqVector {
    let mut vec = FreqVector::new(cspec.result_len());
    vec.set_clause_ident(Some(clause.ident()));

    if !clause.is_empty() {
        if cspec.use_litcount() {
            assert!(
                vec.len() >= FV_CLAUSE_FEATURES,
                "literal-count feature vectors need two result slots"
            );
            vec.as_mut_slice()[0] = usize_to_i64(
                clause.positive_literal_count(),
                "positive literal count fits in C long",
            );
            vec.as_mut_slice()[1] = usize_to_i64(
                clause.negative_literal_count(),
                "negative literal count fits in C long",
            );
        }

        let max_fun = max_clause_fun_code(clause);
        let mut full_vec = vec![0; full_feature_len(max_fun)];
        let mut mod_stack = Vec::new();
        clause.add_symbol_features(&mut mod_stack, &mut full_vec);

        while let Some(findex) = mod_stack.pop() {
            gather_feature_vec(cspec, &full_vec, &mut vec, findex);
            full_vec[findex] = 0;
            let depth_index = findex + 1;
            gather_feature_vec(cspec, &full_vec, &mut vec, depth_index);
            full_vec[depth_index] = 0;
        }
    }

    vec
}

/// Builds a Bill-style feature collection descriptor.
///
/// # Panics
///
/// Panics if `len <= 2`, if signature f-code arithmetic overflows, or if the
/// signature reports inconsistent predicate/function counts.
#[must_use]
pub fn bill_features_collect_alloc(sig: &Signature, len: usize) -> FvCollect {
    assert!(
        len > FV_CLAUSE_FEATURES,
        "Bill feature vector length must exceed 2"
    );
    let mut predicates = non_negative_i32_to_usize(sig.count_symbols(true), "predicate count");
    let mut functions = non_negative_i32_to_usize(sig.count_symbols(false), "function count");

    while bill_feature_len(FV_CLAUSE_FEATURES, predicates, functions) > len {
        if predicates > functions {
            predicates -= 1;
        } else {
            functions -= 1;
        }
    }

    let assembly_len = full_feature_len(i64_to_usize(
        sig.f_count(),
        "signature f-count must fit usize",
    )) + FV_CLAUSE_FEATURES;
    let layout = FvCollectLayout::new(FvIndexType::CollectFeatures, true, assembly_len, len);
    let mut cspec = FvCollect::new(layout);
    fill_bill_assembly(sig, predicates, functions, &mut cspec);
    cspec
}

/// Builds a BillPlus-style feature collection descriptor with overflow slots.
///
/// # Panics
///
/// Panics if `len <= 2`, if signature f-code arithmetic overflows, or if the
/// signature reports inconsistent predicate/function counts.
#[must_use]
pub fn bill_plus_features_collect_alloc(sig: &Signature, len: usize) -> FvCollect {
    assert!(
        len > FV_CLAUSE_FEATURES,
        "BillPlus feature vector length must exceed 2"
    );
    let mut predicates = non_negative_i32_to_usize(sig.count_symbols(true), "predicate count");
    let mut functions = non_negative_i32_to_usize(sig.count_symbols(false), "function count");

    while bill_feature_len(FV_CLAUSE_FEATURES + 4, predicates, functions) > len {
        if predicates > functions {
            predicates -= 1;
        } else {
            functions -= 1;
        }
    }

    let assembly_len = full_feature_len(i64_to_usize(
        sig.f_count(),
        "signature f-count must fit usize",
    )) + FV_CLAUSE_FEATURES;
    let mut layout = FvCollectLayout::new(FvIndexType::CollectFeatures, true, assembly_len, len);
    layout.pos_count = FvOverflowSpec::new(usize_to_i64(len - 4, "overflow base fits"), 0, 1);
    layout.neg_count = FvOverflowSpec::new(usize_to_i64(len - 3, "overflow base fits"), 0, 1);
    layout.pos_depth = FvOverflowSpec::new(usize_to_i64(len - 2, "overflow base fits"), 0, 1);
    layout.neg_depth = FvOverflowSpec::new(usize_to_i64(len - 1, "overflow base fits"), 0, 1);
    let mut cspec = FvCollect::new(layout);
    fill_bill_assembly(sig, predicates, functions, &mut cspec);
    cspec
}

#[must_use]
pub fn fv_pack_clause(
    clause: Clause,
    perm: Option<&PermVector>,
    cspec: Option<&FvCollect>,
) -> FvPackedClause {
    let vector = cspec.and_then(|cspec| {
        (cspec.features() != FvIndexType::NoFeatures)
            .then(|| optimized_var_freq_vector_compute(&clause, perm, cspec))
    });
    FvPackedClause { vector, clause }
}

#[must_use]
pub fn fv_unpack_clause(pack: FvPackedClause) -> Clause {
    pack.into_clause()
}

fn add_ac_features(vec: &mut FreqVector, symbols: usize, clause: &Clause) {
    add_literal_counts(vec, clause);
    let section_len = section_len(symbols);
    let (_, rest) = vec.array.split_at_mut(FV_CLAUSE_FEATURES);
    let (neg_freq, pos_freq) = rest.split_at_mut(section_len);
    let pos_freq = &mut pos_freq[..section_len];
    let mut unused_depth = vec![0; section_len];
    for literal in clause.literals().as_slice() {
        if literal.is_positive() {
            literal.add_symbol_features_limited(pos_freq, &mut unused_depth, symbols);
        } else {
            literal.add_symbol_features_limited(neg_freq, &mut unused_depth, symbols);
        }
    }
}

fn add_ss_features(vec: &mut FreqVector, symbols: usize, clause: &Clause) {
    let section_len = section_len(symbols);
    let (neg_depth, pos_depth) = vec.array.split_at_mut(section_len);
    let pos_depth = &mut pos_depth[..section_len];
    let mut unused_freq = vec![0; section_len];
    for literal in clause.literals().as_slice() {
        if literal.is_positive() {
            literal.add_symbol_features_limited(&mut unused_freq, pos_depth, symbols);
        } else {
            literal.add_symbol_features_limited(&mut unused_freq, neg_depth, symbols);
        }
    }
}

fn add_all_features(vec: &mut FreqVector, symbols: usize, clause: &Clause) {
    add_literal_counts(vec, clause);
    let section_len = section_len(symbols);
    let (_, rest) = vec.array.split_at_mut(FV_CLAUSE_FEATURES);
    let (neg_freq, rest) = rest.split_at_mut(section_len);
    let (pos_freq, rest) = rest.split_at_mut(section_len);
    let (pos_depth, neg_depth) = rest.split_at_mut(section_len);
    debug_assert_eq!(neg_depth.len(), section_len);
    for literal in clause.literals().as_slice() {
        if literal.is_positive() {
            literal.add_symbol_features_limited(pos_freq, pos_depth, symbols);
        } else {
            literal.add_symbol_features_limited(neg_freq, neg_depth, symbols);
        }
    }
}

fn add_literal_counts(vec: &mut FreqVector, clause: &Clause) {
    vec.as_mut_slice()[0] += usize_to_i64(
        clause.positive_literal_count(),
        "positive literal count fits in C long",
    );
    vec.as_mut_slice()[1] += usize_to_i64(
        clause.negative_literal_count(),
        "negative literal count fits in C long",
    );
}

fn gather_feature_vec(cspec: &FvCollect, full_vec: &[i64], vec: &mut FreqVector, findex: usize) {
    let resindex = if findex < cspec.assembly_vector.len() {
        index_from_raw(
            cspec.assembly_vector[findex],
            "assembly vector result index",
        )
    } else {
        let spec = match findex % 4 {
            0 => cspec.pos_count,
            1 => cspec.pos_depth,
            2 => cspec.neg_count,
            3 => cspec.neg_depth,
            _ => unreachable!("modulo four result is in 0..4"),
        };
        if spec.modulus == 0 {
            None
        } else {
            let symbol = usize_to_i64(findex / 4, "feature symbol index fits in C long");
            let raw = spec.base + (spec.offset + symbol) % spec.modulus;
            Some(i64_to_usize(raw, "overflow result index must fit usize"))
        }
    };

    if let Some(resindex) = resindex {
        assert!(
            resindex < vec.len(),
            "gathered feature index must address result vector"
        );
        vec.as_mut_slice()[resindex] += full_vec[findex];
    }
}

fn fill_bill_assembly(sig: &Signature, predicates: usize, functions: usize, cspec: &mut FvCollect) {
    let mut pos = FV_CLAUSE_FEATURES;
    let mut remaining_predicates = predicates;
    let mut f_code = sig.internal_symbols() + 1;
    while remaining_predicates != 0 {
        assert!(
            f_code <= sig.f_count(),
            "signature predicate count must match selectable symbols"
        );
        if !sig.is_special(f_code) && sig.is_predicate(f_code) {
            assign_assembly(cspec, f_code, 0, pos);
            pos += 1;
            assign_assembly(cspec, f_code, 1, pos);
            pos += 1;
            remaining_predicates -= 1;
        }
        f_code += 1;
    }

    let mut remaining_functions = functions;
    f_code = sig.internal_symbols() + 1;
    while remaining_functions != 0 {
        assert!(
            f_code <= sig.f_count(),
            "signature function count must match selectable symbols"
        );
        if !sig.is_special(f_code) && sig.is_function(f_code) {
            assign_assembly(cspec, f_code, 0, pos);
            pos += 1;
            assign_assembly(cspec, f_code, 1, pos);
            pos += 1;
            assign_assembly(cspec, f_code, 2, pos);
            pos += 1;
            assign_assembly(cspec, f_code, 3, pos);
            pos += 1;
            remaining_functions -= 1;
        }
        f_code += 1;
    }
}

fn assign_assembly(cspec: &mut FvCollect, f_code: FunCode, offset: usize, value: usize) {
    let slot = feature_slot(f_code, offset);
    assert!(
        slot < cspec.assembly_vector.len(),
        "assembly vector must cover signature f-code slots"
    );
    cspec.assembly_vector_mut()[slot] = usize_to_i64(value, "assembly result index fits");
}

fn feature_slot(f_code: FunCode, offset: usize) -> usize {
    i64_to_usize(f_code, "positive f-code must fit usize")
        .checked_mul(4)
        .and_then(|slot| slot.checked_add(offset))
        .unwrap_or_else(|| panic!("feature slot arithmetic overflowed"))
}

fn max_clause_fun_code(clause: &Clause) -> usize {
    let mut fcodes = BTreeSet::new();
    let _ = clause.collect_fcodes(&mut fcodes);
    let max = fcodes.into_iter().max().unwrap_or(0);
    i64_to_usize(max, "maximum clause f-code must fit usize")
}

fn full_feature_len(max_fun: usize) -> usize {
    max_fun
        .checked_add(1)
        .and_then(|value| value.checked_mul(4))
        .unwrap_or_else(|| panic!("full feature-vector length overflowed"))
}

const fn bill_feature_len(base: usize, predicates: usize, functions: usize) -> usize {
    base + 2 * predicates + 4 * functions
}

const fn section_len(symbols: usize) -> usize {
    symbols + 1
}

fn index_from_raw(raw: i64, context: &str) -> Option<usize> {
    (raw != -1).then(|| i64_to_usize(raw, context))
}

fn i64_to_usize(value: i64, context: &str) -> usize {
    assert!(value >= 0, "{context} must be non-negative");
    usize::try_from(value).unwrap_or_else(|_| panic!("{context}"))
}

fn usize_to_i64(value: usize, context: &str) -> i64 {
    i64::try_from(value).unwrap_or_else(|_| panic!("{context}"))
}

fn non_negative_i32_to_usize(value: i32, context: &str) -> usize {
    assert!(value >= 0, "{context} must be non-negative");
    usize::try_from(value).unwrap_or_else(|_| panic!("{context} fits usize"))
}

struct Tuple3 {
    pos: usize,
    diff: i64,
    value: i64,
}

#[cfg(test)]
mod tests {
    use super::{
        bill_plus_features_collect_alloc, fv_collect_freq_vector_compute, fv_pack_clause, fv_size,
        fv_unpack_clause, optimized_var_freq_vector_compute, perm_vector_compute_internal,
        var_freq_vector_compute, FreqVector, FvCollect, FvCollectLayout, FvIndexType,
        FvOverflowSpec, FV_CLAUSE_FEATURES,
    };
    use crate::clauses::clause::Clause;
    use crate::clauses::eqn::Eqn;
    use crate::clauses::eqnlist::EqnList;
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

    fn clause_from(literals: Vec<Eqn>) -> Clause {
        let mut clause = Clause::alloc(EqnList::from_vec(literals));
        clause.set_weight(clause.standard_weight());
        clause
    }

    #[test]
    fn permutation_vector_selects_most_informative_tail_like_c() {
        let fmax = FreqVector::from_values(vec![1, 8, 10, 4, 3]);
        let fmin = FreqVector::from_values(vec![1, 3, 5, 3, 3]);
        let fsum = FreqVector::from_values(vec![1, 2, 9, 4, 3]);

        let perm = perm_vector_compute_internal(&fmax, &fmin, &fsum, 3, false);
        assert_eq!(perm.as_slice(), &[3, 1, 2]);

        let eliminated = perm_vector_compute_internal(&fmax, &fmin, &fsum, 99, true);
        assert_eq!(eliminated.as_slice(), &[3, 1, 2]);

        let flat = FreqVector::from_values(vec![5, 7, 7]);
        let fallback = perm_vector_compute_internal(&flat, &flat, &flat, 99, true);
        assert_eq!(fallback.as_slice(), &[1]);
    }

    #[test]
    fn classic_frequency_vectors_match_ac_ss_and_all_layouts() {
        let mut bank = test_bank();
        let first = typed_const(&mut bank, "a");
        let second = typed_const(&mut bank, "b");
        let f_of_first = typed_unary(&mut bank, "f", &first);
        let max_code = usize::try_from(f_of_first.f_code()).unwrap();
        let symbols = max_code + 1;
        let clause = clause_from(vec![
            literal(&mut bank, &f_of_first, &second, true),
            literal(&mut bank, &first, &second, false),
        ]);

        let mut all_spec =
            FvCollect::new(FvCollectLayout::new(FvIndexType::AllFeatures, false, 0, 0));
        all_spec.set_max_symbols(symbols);
        let all = var_freq_vector_compute(&clause, &all_spec);
        let section = symbols + 1;
        let neg_freq = FV_CLAUSE_FEATURES;
        let pos_freq = neg_freq + section;
        let pos_depth = pos_freq + section;
        let neg_depth = pos_depth + section;

        assert_eq!(all.as_slice()[0], 1);
        assert_eq!(all.as_slice()[1], 1);
        assert_eq!(all.as_slice()[pos_freq + max_code], 1);
        assert_eq!(
            all.as_slice()[pos_freq + usize::try_from(first.f_code()).unwrap()],
            1
        );
        assert_eq!(
            all.as_slice()[neg_freq + usize::try_from(second.f_code()).unwrap()],
            1
        );
        assert_eq!(
            all.as_slice()[pos_depth + usize::try_from(first.f_code()).unwrap()],
            1
        );
        assert_eq!(
            all.as_slice()[neg_depth + usize::try_from(first.f_code()).unwrap()],
            0
        );

        let mut ac_spec =
            FvCollect::new(FvCollectLayout::new(FvIndexType::AcFeatures, false, 0, 0));
        ac_spec.set_max_symbols(symbols);
        let ac = var_freq_vector_compute(&clause, &ac_spec);
        assert_eq!(ac.len(), fv_size(symbols, FvIndexType::AcFeatures));
        assert_eq!(ac.as_slice()[0], 1);
        assert_eq!(ac.as_slice()[1], 1);
        assert_eq!(ac.as_slice()[FV_CLAUSE_FEATURES + section + max_code], 1);

        let mut ss_spec =
            FvCollect::new(FvCollectLayout::new(FvIndexType::SsFeatures, false, 0, 0));
        ss_spec.set_max_symbols(symbols);
        let ss = var_freq_vector_compute(&clause, &ss_spec);
        assert_eq!(ss.len(), fv_size(symbols, FvIndexType::SsFeatures));
        assert_eq!(
            ss.as_slice()[section + usize::try_from(first.f_code()).unwrap()],
            1
        );
    }

    #[test]
    fn collect_frequency_vector_folds_full_symbol_features() {
        let mut bank = test_bank();
        let first = typed_const(&mut bank, "a");
        let second = typed_const(&mut bank, "b");
        let f_of_first = typed_unary(&mut bank, "f", &first);
        let clause = clause_from(vec![
            literal(&mut bank, &f_of_first, &second, true),
            literal(&mut bank, &first, &second, false),
        ]);
        let mut layout = FvCollectLayout::new(FvIndexType::CollectFeatures, false, 0, 4);
        layout.pos_count = FvOverflowSpec::new(0, 0, 1);
        layout.pos_depth = FvOverflowSpec::new(1, 0, 1);
        layout.neg_count = FvOverflowSpec::new(2, 0, 1);
        layout.neg_depth = FvOverflowSpec::new(3, 0, 1);
        let cspec = FvCollect::new(layout);

        let vector = fv_collect_freq_vector_compute(&clause, &cspec);
        assert_eq!(vector.as_slice(), &[3, 1, 2, 0]);
    }

    #[test]
    fn optimized_vector_applies_permutation_positions() {
        let mut bank = test_bank();
        let first = typed_const(&mut bank, "a");
        let second = typed_const(&mut bank, "b");
        let clause = clause_from(vec![literal(&mut bank, &first, &second, true)]);
        let mut cspec = FvCollect::new(FvCollectLayout::new(FvIndexType::AcFeatures, false, 0, 0));
        cspec.set_max_symbols(usize::try_from(second.f_code()).unwrap() + 1);
        let full = var_freq_vector_compute(&clause, &cspec);
        let mut perm = super::PermVector::new(2);
        perm.as_mut_slice()[0] = 0;
        perm.as_mut_slice()[1] = i64::try_from(full.len() - 1).unwrap();

        let optimized = optimized_var_freq_vector_compute(&clause, Some(&perm), &cspec);
        assert_eq!(
            optimized.as_slice(),
            &[full.as_slice()[0], full.as_slice()[full.len() - 1]]
        );
    }

    #[test]
    fn bill_plus_collect_descriptor_maps_signature_slots_and_overflow() {
        let mut sig = Signature::new(TypeBank::new());
        sig.insert_internal_codes().unwrap();
        let individual = sig.type_bank().default_type();
        let bool_type = sig.type_bank().bool_type();
        let predicate = sig.insert_id("p", 1, false);
        sig.declare_final_type(
            predicate,
            alloc_arrow_type(vec![individual.clone(), bool_type]),
        )
        .unwrap();
        let function = sig.insert_id("f", 1, false);
        sig.declare_final_type(
            function,
            alloc_arrow_type(vec![individual.clone(), individual]),
        )
        .unwrap();

        let cspec = bill_plus_features_collect_alloc(&sig, 12);
        assert!(cspec.use_litcount());
        assert_eq!(
            cspec.assembly_vector()[usize::try_from(4 * predicate).unwrap()],
            2
        );
        assert_eq!(
            cspec.assembly_vector()[usize::try_from(4 * predicate + 1).unwrap()],
            3
        );
        assert_eq!(
            cspec.assembly_vector()[usize::try_from(4 * function).unwrap()],
            4
        );
        assert_eq!(
            cspec.assembly_vector()[usize::try_from(4 * function + 3).unwrap()],
            7
        );
        assert_eq!(cspec.pos_count_overflow(), FvOverflowSpec::new(8, 0, 1));
        assert_eq!(cspec.neg_depth_overflow(), FvOverflowSpec::new(11, 0, 1));
    }

    #[test]
    fn packed_clause_owns_clause_and_optional_vector() {
        let mut bank = test_bank();
        let first = typed_const(&mut bank, "a");
        let second = typed_const(&mut bank, "b");
        let clause = clause_from(vec![literal(&mut bank, &first, &second, true)]);
        let ident = clause.ident();

        let dummy = fv_pack_clause(clause.clone(), None, None);
        assert!(dummy.vector().is_none());
        assert_eq!(fv_unpack_clause(dummy).ident(), ident);

        let mut cspec = FvCollect::new(FvCollectLayout::new(FvIndexType::AcFeatures, false, 0, 0));
        cspec.set_max_symbols(usize::try_from(second.f_code()).unwrap() + 1);
        let packed = fv_pack_clause(clause, None, Some(&cspec));
        assert!(packed.vector().is_some());
        assert_eq!(packed.clause().ident(), ident);
    }
}
