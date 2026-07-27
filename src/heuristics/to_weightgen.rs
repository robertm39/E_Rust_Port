//! Term-ordering weight generation from `che_to_weightgen`.

use crate::basics::error::{Diagnostic, ErrorCode};
use crate::basics::partial_orderings::{ordering_to_part, CompareResult, HoOrderKind};
use crate::clauses::clausesets::ClauseSet;
use crate::heuristics::fcode_featurearrays::{FCodeFeatureArray, FCodeFeatureSortCell};
use crate::heuristics::to_params::{
    OrderParmsCell, TOWeightGenMethod, TermOrdering, W_CONST_NO_SPECIAL_WEIGHT,
};
use crate::inout::scanner::Scanner;
use crate::orderings::cto_orderings::weights_parse;
use crate::orderings::ocb::OrderControlBlock;
use crate::terms::functypes::FunCode;
use crate::terms::signature::{Signature, SIG_PHONY_APP_CODE, SIG_TRUE_CODE};
use std::cmp::Ordering;

pub const W_DEFAULT_WEIGHT: i64 = 1;
pub const W_TO_BASEWEIGHT: i64 = 4;

const C_INT_MAX: i64 = 2_147_483_647;

#[derive(Clone, Copy, Debug, Default)]
pub struct WeightGenerationContext<'a> {
    pub precedence_order: Option<&'a [FunCode]>,
    pub precedence_ocb: Option<&'a OrderControlBlock>,
    pub pre_weights: Option<&'a str>,
    pub higher_order_problem: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeneratedWeights {
    pub weights: Vec<i64>,
    pub var_weight: i64,
    pub lam_weight: i64,
    pub db_weight: i64,
}

impl GeneratedWeights {
    #[must_use]
    pub fn weight(&self, symbol: FunCode) -> Option<i64> {
        usize::try_from(symbol)
            .ok()
            .and_then(|index| self.weights.get(index))
            .copied()
    }
}

/// Generates the per-symbol KBO weights computed by C `TOGenerateWeights`.
///
/// The result is indexed by f-code; index zero is unused. Methods that depend
/// on `OCBFunCompare` use `context.precedence_order` as a low-to-high total
/// precedence, or `context.precedence_ocb` for matrix-backed partial
/// precedence. User `pre_weights` parsing is applied after generated weights,
/// matching C `TOGenerateWeights`.
///
/// # Errors
///
/// Returns a diagnostic when a selected method needs a precedence order but
/// none was supplied, when the supplied total precedence is invalid for the
/// current signature, when user weight parsing fails, or when the method is the
/// invalid C sentinel.
///
/// # Panics
///
/// Panics if the signature contains invalid positive f-codes or if a symbol
/// type id lies outside the C-sized type distribution array.
pub fn generate_weights(
    signature: &mut Signature,
    axioms: &ClauseSet,
    oparms: &OrderParmsCell,
    context: WeightGenerationContext<'_>,
) -> Result<GeneratedWeights, Diagnostic> {
    let mut generated = GeneratedWeights {
        weights: vec![0; weights_size(signature.f_count())],
        var_weight: W_DEFAULT_WEIGHT,
        lam_weight: oparms.lam_w,
        db_weight: oparms.db_w,
    };

    set_symbol_weight(&mut generated.weights, SIG_TRUE_CODE, W_DEFAULT_WEIGHT);
    apply_weight_method(signature, axioms, oparms, context, &mut generated)?;
    apply_constant_overrides(signature, oparms, &mut generated);
    set_symbol_weight(&mut generated.weights, SIG_TRUE_CODE, generated.var_weight);
    if signature.f_count() >= SIG_PHONY_APP_CODE {
        set_symbol_weight(&mut generated.weights, SIG_PHONY_APP_CODE, 0);
    }
    apply_pre_weights(signature, context.pre_weights, &mut generated)?;

    Ok(generated)
}

/// Generate term-ordering weights and install them into an OCB.
///
/// User `pre_weights`, when present, are parsed after generated weights and
/// therefore override generated values just like C `TOGenerateWeights`.
///
/// # Errors
///
/// Returns diagnostics for unsupported weight methods, invalid precedence
/// contexts, scanner creation, or user weight parsing.
///
/// # Panics
///
/// Panics if the OCB was allocated for a different signature snapshot, or if it
/// has no function-weight vector.
pub fn generate_weights_into_ocb(
    signature: &mut Signature,
    axioms: &ClauseSet,
    oparms: &OrderParmsCell,
    context: WeightGenerationContext<'_>,
    ocb: &mut OrderControlBlock,
) -> Result<(), Diagnostic> {
    assert_eq!(
        ocb.sig_size,
        signature.f_count(),
        "weight generation requires a current OCB signature snapshot"
    );
    assert!(
        ocb.weights.is_some(),
        "weight generation requires OCB function-weight storage"
    );

    let pre_weights = context.pre_weights;
    let precedence_ocb = context.precedence_order.is_none().then_some(&*ocb);
    let generation_context = WeightGenerationContext {
        pre_weights: None,
        precedence_ocb,
        ..context
    };
    let generated = generate_weights(signature, axioms, oparms, generation_context)?;
    ocb.install_weights(&generated.weights);
    ocb.var_weight = generated.var_weight;
    ocb.lam_weight = generated.lam_weight;
    ocb.db_weight = generated.db_weight;

    if let Some(pre_weights) = pre_weights {
        let mut scanner = Scanner::from_user_string(pre_weights, true)?;
        weights_parse(&mut scanner, signature, ocb)?;
    }

    ocb.lam_weight = oparms.lam_w;
    ocb.db_weight = oparms.db_w;
    Ok(())
}

#[allow(clippy::too_many_lines)]
fn apply_weight_method(
    signature: &mut Signature,
    axioms: &ClauseSet,
    oparms: &OrderParmsCell,
    context: WeightGenerationContext<'_>,
    generated: &mut GeneratedWeights,
) -> Result<(), Diagnostic> {
    match oparms.to_weight_gen {
        TOWeightGenMethod::ConstantWeight => {
            generate_constant_weights(signature, &mut generated.weights);
            Ok(())
        }
        TOWeightGenMethod::NoMethod | TOWeightGenMethod::SelectMaximal => {
            generate_selmax_weights(signature, context, &mut generated.weights)
        }
        TOWeightGenMethod::ModArityWeight | TOWeightGenMethod::ModArityMax0 => {
            generate_arity_weights(
                signature,
                oparms.to_weight_gen,
                context,
                &mut generated.weights,
            )?;
            generated.var_weight = if oparms.to_const_weight == W_CONST_NO_SPECIAL_WEIGHT {
                W_TO_BASEWEIGHT
            } else {
                oparms.to_const_weight
            };
            Ok(())
        }
        TOWeightGenMethod::ArityWeight
        | TOWeightGenMethod::ArityMax0
        | TOWeightGenMethod::AritySqWeight
        | TOWeightGenMethod::AritySqMax0
        | TOWeightGenMethod::InvArityWeight
        | TOWeightGenMethod::InvArityMax0
        | TOWeightGenMethod::InvAritySqWeight
        | TOWeightGenMethod::InvAritySqMax0 => generate_arity_weights(
            signature,
            oparms.to_weight_gen,
            context,
            &mut generated.weights,
        ),
        TOWeightGenMethod::Precedence => {
            generate_precedence_weights(signature, context, &mut generated.weights)
        }
        TOWeightGenMethod::PrecedenceInv => {
            generate_invprecedence_weights(signature, context, &mut generated.weights)
        }
        TOWeightGenMethod::PrecRank5 => {
            generate_precrank_weights(signature, context, 5.0, &mut generated.weights)
        }
        TOWeightGenMethod::PrecRank10 => {
            generate_precrank_weights(signature, context, 10.0, &mut generated.weights)
        }
        TOWeightGenMethod::PrecRank20 => {
            generate_precrank_weights(signature, context, 20.0, &mut generated.weights)
        }
        TOWeightGenMethod::Frequency => {
            generate_freq_weights(signature, axioms, &mut generated.weights);
            Ok(())
        }
        TOWeightGenMethod::InvFrequency => {
            generate_invfreq_weights(signature, axioms, &mut generated.weights);
            Ok(())
        }
        TOWeightGenMethod::FrequencyRank => {
            generate_freqrank_weights(signature, axioms, &mut generated.weights);
            Ok(())
        }
        TOWeightGenMethod::InvFrequencyRank => {
            generate_invfreqrank_weights(signature, axioms, &mut generated.weights);
            Ok(())
        }
        TOWeightGenMethod::InvConjFrequencyRank => {
            generate_invconjfreqrank_weights(signature, axioms, &mut generated.weights);
            Ok(())
        }
        TOWeightGenMethod::FrequencyRankSq => {
            generate_freqranksq_weights(signature, axioms, &mut generated.weights);
            Ok(())
        }
        TOWeightGenMethod::InvFrequencyRankSq => {
            generate_invfreqranksq_weights(signature, axioms, &mut generated.weights);
            Ok(())
        }
        TOWeightGenMethod::InvModFreqRank => {
            generate_inv_modfreqrank_weights(signature, axioms, &mut generated.weights);
            Ok(())
        }
        TOWeightGenMethod::InvModFreqRankMax0 => generate_inv_modfreqrank_weights_max_0(
            signature,
            axioms,
            context,
            &mut generated.weights,
        ),
        TOWeightGenMethod::TypeFrequencyRank => {
            generate_type_freq_rank_weights(signature, axioms, &mut generated.weights);
            Ok(())
        }
        TOWeightGenMethod::TypeFrequencyCount => {
            generate_type_freq_weights(signature, axioms, &mut generated.weights);
            Ok(())
        }
        TOWeightGenMethod::InvTypeFrequencyRank => {
            generate_inv_type_freq_rank_weights(signature, axioms, &mut generated.weights);
            Ok(())
        }
        TOWeightGenMethod::InvTypeFrequencyCount => {
            generate_inv_type_freq_weights(signature, axioms, &mut generated.weights);
            Ok(())
        }
        TOWeightGenMethod::CombFrequencyRank => {
            generate_comb_freq_rank_weights(signature, axioms, &mut generated.weights);
            Ok(())
        }
        TOWeightGenMethod::CombFrequencyCount => {
            generate_comb_freq_weights(signature, axioms, &mut generated.weights);
            Ok(())
        }
        TOWeightGenMethod::InvCombFrequencyRank => {
            generate_inv_comb_freq_rank_weights(signature, axioms, &mut generated.weights);
            Ok(())
        }
        TOWeightGenMethod::InvCombFrequencyCount => {
            generate_inv_comb_freq_weights(signature, axioms, &mut generated.weights);
            Ok(())
        }
        TOWeightGenMethod::InvalidEntry => Err(Diagnostic::new(
            ErrorCode::OTHER_ERROR,
            "Weight generation method unimplemented",
        )),
    }
}

fn apply_constant_overrides(
    signature: &Signature,
    oparms: &OrderParmsCell,
    generated: &mut GeneratedWeights,
) {
    for symbol in symbols_after_true(signature) {
        if symbol_arity(signature, symbol) == 0 {
            if oparms.to_const_weight != W_CONST_NO_SPECIAL_WEIGHT {
                set_symbol_weight(
                    &mut generated.weights,
                    symbol,
                    oparms.to_const_weight.max(1),
                );
            }
            debug_assert!(symbol_weight(&generated.weights, symbol) > 0);
            if oparms.force_kbo_var_weight {
                generated.var_weight = generated
                    .var_weight
                    .min(symbol_weight(&generated.weights, symbol));
            }
        }
    }
}

fn apply_pre_weights(
    signature: &Signature,
    pre_weights: Option<&str>,
    generated: &mut GeneratedWeights,
) -> Result<(), Diagnostic> {
    let Some(pre_weights) = pre_weights else {
        return Ok(());
    };
    if pre_weights.is_empty() {
        return Ok(());
    }

    let mut ocb =
        OrderControlBlock::alloc(TermOrdering::Kbo, true, signature, HoOrderKind::LfhoOrder);
    ocb.install_weights(&generated.weights);
    ocb.var_weight = generated.var_weight;
    ocb.lam_weight = generated.lam_weight;
    ocb.db_weight = generated.db_weight;

    let mut scanner = Scanner::from_user_string(pre_weights, true)?;
    weights_parse(&mut scanner, signature, &mut ocb)?;
    if let Some(weights) = ocb.weights {
        generated.weights = weights;
    }
    generated.var_weight = ocb.var_weight;
    generated.lam_weight = ocb.lam_weight;
    generated.db_weight = ocb.db_weight;
    Ok(())
}

fn generate_constant_weights(signature: &Signature, weights: &mut [i64]) {
    for symbol in symbols_after_true(signature) {
        set_symbol_weight(weights, symbol, W_DEFAULT_WEIGHT);
    }
}

fn generate_selmax_weights(
    signature: &Signature,
    context: WeightGenerationContext<'_>,
    weights: &mut [i64],
) -> Result<(), Diagnostic> {
    generate_constant_weights(signature, weights);
    set_maximal_0(signature, context, weights)
}

fn generate_arity_weights(
    signature: &Signature,
    method: TOWeightGenMethod,
    context: WeightGenerationContext<'_>,
    weights: &mut [i64],
) -> Result<(), Diagnostic> {
    let maxarity = i64::from(signature.find_max_used_arity());
    for symbol in symbols_after_true(signature) {
        let arity = symbol_arity(signature, symbol);
        let weight = match method {
            TOWeightGenMethod::ArityWeight | TOWeightGenMethod::ArityMax0 => arity + 1,
            TOWeightGenMethod::ModArityWeight | TOWeightGenMethod::ModArityMax0 => {
                arity + W_TO_BASEWEIGHT
            }
            TOWeightGenMethod::AritySqWeight | TOWeightGenMethod::AritySqMax0 => arity * arity + 1,
            TOWeightGenMethod::InvArityWeight | TOWeightGenMethod::InvArityMax0 => {
                maxarity - arity + 1
            }
            TOWeightGenMethod::InvAritySqWeight | TOWeightGenMethod::InvAritySqMax0 => {
                maxarity * maxarity - arity * arity + 1
            }
            _ => panic!("arity weight generation called with non-arity method"),
        };
        set_symbol_weight(weights, symbol, weight * W_DEFAULT_WEIGHT);
    }

    if matches!(
        method,
        TOWeightGenMethod::ArityMax0
            | TOWeightGenMethod::ModArityMax0
            | TOWeightGenMethod::InvArityMax0
            | TOWeightGenMethod::AritySqMax0
            | TOWeightGenMethod::InvAritySqMax0
    ) {
        set_maximal_0(signature, context, weights)?;
    }
    Ok(())
}

fn generate_precedence_weights(
    signature: &Signature,
    context: WeightGenerationContext<'_>,
    weights: &mut [i64],
) -> Result<(), Diagnostic> {
    let precedence = PrecedenceSource::new(signature, context)?;
    for left in symbols_after_true(signature) {
        let mut weight = 1;
        for right in symbols_after_true(signature) {
            if precedence.compare(signature, left, right) == CompareResult::Greater {
                weight += 1;
            }
        }
        set_symbol_weight(weights, left, weight * W_DEFAULT_WEIGHT);
    }
    Ok(())
}

fn generate_invprecedence_weights(
    signature: &Signature,
    context: WeightGenerationContext<'_>,
    weights: &mut [i64],
) -> Result<(), Diagnostic> {
    let precedence = PrecedenceSource::new(signature, context)?;
    for left in symbols_after_true(signature) {
        let mut weight = 1;
        for right in symbols_after_true(signature) {
            if precedence.compare(signature, left, right) == CompareResult::Lesser {
                weight += 1;
            }
        }
        set_symbol_weight(weights, left, weight * W_DEFAULT_WEIGHT);
    }
    Ok(())
}

fn generate_precrank_weights(
    signature: &mut Signature,
    context: WeightGenerationContext<'_>,
    ranks: f32,
    weights: &mut [i64],
) -> Result<(), Diagnostic> {
    let precedence = PrecedenceSource::new(signature, context)?;
    let sorted_symbols = precedence.sorted_symbols(signature);
    let symb_no = sorted_symbols.len();
    for (index, symbol) in sorted_symbols.into_iter().enumerate() {
        set_symbol_weight(weights, symbol, precrank_weight(index, symb_no, ranks));
    }
    Ok(())
}

fn generate_freq_weights(signature: &Signature, axioms: &ClauseSet, weights: &mut [i64]) {
    let array = FCodeFeatureArray::alloc(signature, axioms);
    for symbol in symbols_after_true(signature) {
        let frequency = feature_entry(&array, symbol).freq;
        set_symbol_weight(weights, symbol, frequency.max(1) * W_DEFAULT_WEIGHT);
    }
}

fn generate_invfreq_weights(signature: &Signature, axioms: &ClauseSet, weights: &mut [i64]) {
    let array = FCodeFeatureArray::alloc(signature, axioms);
    let mut max_count = 1;
    for symbol in symbols_after_true(signature) {
        max_count = max_count.max(feature_entry(&array, symbol).freq);
    }
    max_count += 1;
    for symbol in symbols_after_true(signature) {
        let frequency = feature_entry(&array, symbol).freq;
        set_symbol_weight(
            weights,
            symbol,
            (max_count - frequency.max(1)) * W_DEFAULT_WEIGHT,
        );
    }
}

fn generate_freqrank_weights(signature: &Signature, axioms: &ClauseSet, weights: &mut [i64]) {
    let mut array = frequency_key_array(signature, axioms);
    array.sort();
    let mut weight = 0;
    let mut freq = 0;
    for entry in sorted_entries(&array, signature) {
        if freq != entry.freq {
            freq = entry.freq;
            weight += 1;
        }
        set_symbol_weight(weights, entry.symbol, weight.max(1) * W_DEFAULT_WEIGHT);
    }
}

fn generate_invfreqrank_weights(signature: &Signature, axioms: &ClauseSet, weights: &mut [i64]) {
    let mut array = frequency_key_array(signature, axioms);
    array.sort();
    let mut weight = 0;
    let mut freq = 0;
    for entry in sorted_entries(&array, signature).iter().rev() {
        if freq != entry.freq {
            freq = entry.freq;
            weight += 1;
        }
        set_symbol_weight(weights, entry.symbol, weight.max(1) * W_DEFAULT_WEIGHT);
    }
}

fn generate_invconjfreqrank_weights(
    signature: &Signature,
    axioms: &ClauseSet,
    weights: &mut [i64],
) {
    let mut array = FCodeFeatureArray::alloc(signature, axioms);
    for_symbol_entry(&mut array, |entry| {
        entry.key1 = if entry.conjfreq != 0 {
            C_INT_MAX - entry.conjfreq
        } else {
            0
        };
        entry.key2 = -entry.freq;
    });
    array.sort();

    let mut weight = 0;
    let mut freq = 0;
    let mut conjfreq = 0;
    for entry in sorted_entries(&array, signature) {
        if freq != entry.freq || conjfreq != entry.conjfreq {
            freq = entry.freq;
            conjfreq = entry.conjfreq;
            weight += 1;
        }
        set_symbol_weight(weights, entry.symbol, weight * W_DEFAULT_WEIGHT);
    }
}

fn generate_freqranksq_weights(signature: &Signature, axioms: &ClauseSet, weights: &mut [i64]) {
    let mut array = frequency_key_array(signature, axioms);
    array.sort();
    let mut weight = 0;
    let mut freq = 0;
    for entry in sorted_entries(&array, signature) {
        if freq != entry.freq {
            freq = entry.freq;
            weight += 1;
        }
        set_symbol_weight(weights, entry.symbol, weight * weight * W_DEFAULT_WEIGHT);
    }
}

fn generate_invfreqranksq_weights(signature: &Signature, axioms: &ClauseSet, weights: &mut [i64]) {
    let mut array = frequency_key_array(signature, axioms);
    array.sort();
    let mut weight = 0;
    let mut freq = 0;
    for entry in sorted_entries(&array, signature).iter().rev() {
        if freq != entry.freq {
            freq = entry.freq;
            weight += 1;
        }
        set_symbol_weight(weights, entry.symbol, weight * weight * W_DEFAULT_WEIGHT);
    }
}

fn generate_inv_modfreqrank_weights(
    signature: &Signature,
    axioms: &ClauseSet,
    weights: &mut [i64],
) {
    let mut array = frequency_key_array(signature, axioms);
    array.sort();
    assign_inverse_modified_frequency_ranks(signature, weights, &array);
}

fn generate_inv_modfreqrank_weights_max_0(
    signature: &Signature,
    axioms: &ClauseSet,
    context: WeightGenerationContext<'_>,
    weights: &mut [i64],
) -> Result<(), Diagnostic> {
    let mut array = frequency_key_array(signature, axioms);
    array.sort();
    assign_inverse_modified_frequency_ranks(signature, weights, &array);
    set_maximal_unary_0(signature, context, weights)
}

fn generate_type_freq_weights(signature: &mut Signature, axioms: &ClauseSet, weights: &mut [i64]) {
    let type_counts = type_distribution(signature, axioms);
    for symbol in symbols_after_true(signature) {
        let frequency = symbol_type_count(signature, symbol, &type_counts);
        set_symbol_weight(weights, symbol, frequency.max(1) * W_DEFAULT_WEIGHT);
    }
}

fn generate_inv_type_freq_weights(
    signature: &mut Signature,
    axioms: &ClauseSet,
    weights: &mut [i64],
) {
    let type_counts = type_distribution(signature, axioms);
    let max_aggregate = type_counts.iter().copied().max().unwrap_or(0) + 1;
    for symbol in symbols_after_true(signature) {
        let frequency = symbol_type_count(signature, symbol, &type_counts);
        set_symbol_weight(
            weights,
            symbol,
            (max_aggregate - frequency.max(1)) * W_DEFAULT_WEIGHT,
        );
    }
}

fn generate_comb_freq_weights(signature: &mut Signature, axioms: &ClauseSet, weights: &mut [i64]) {
    let array = FCodeFeatureArray::alloc(signature, axioms);
    let type_counts = symbol_frequency_type_distribution(signature, &array);
    for symbol in symbols_after_true(signature) {
        let entry = feature_entry(&array, symbol);
        let type_frequency = symbol_type_count(signature, symbol, &type_counts);
        set_symbol_weight(
            weights,
            symbol,
            (type_frequency + 2 * entry.freq).max(1) * W_DEFAULT_WEIGHT,
        );
    }
}

fn generate_inv_comb_freq_weights(
    signature: &mut Signature,
    axioms: &ClauseSet,
    weights: &mut [i64],
) {
    let array = FCodeFeatureArray::alloc(signature, axioms);
    let type_counts = type_distribution(signature, axioms);
    let mut max_comb = 0;
    for symbol in symbols_after_true(signature) {
        let entry = feature_entry(&array, symbol);
        let type_frequency = symbol_type_count(signature, symbol, &type_counts);
        max_comb = max_comb.max(type_frequency + 2 * entry.freq);
    }
    max_comb += 1;
    for symbol in symbols_after_true(signature) {
        let entry = feature_entry(&array, symbol);
        let type_frequency = symbol_type_count(signature, symbol, &type_counts);
        set_symbol_weight(
            weights,
            symbol,
            (max_comb - (type_frequency + 2 * entry.freq).max(1)) * W_DEFAULT_WEIGHT,
        );
    }
}

fn generate_type_freq_rank_weights(
    signature: &mut Signature,
    axioms: &ClauseSet,
    weights: &mut [i64],
) {
    let type_counts = type_distribution(signature, axioms);
    let mut array = FCodeFeatureArray::alloc(signature, axioms);
    for_symbol_entry(&mut array, |entry| {
        entry.key1 = symbol_type_count(signature, entry.symbol, &type_counts);
    });
    array.sort();
    assign_forward_key1_ranks(signature, weights, &array, -1);
}

fn generate_inv_type_freq_rank_weights(
    signature: &mut Signature,
    axioms: &ClauseSet,
    weights: &mut [i64],
) {
    let type_counts = type_distribution(signature, axioms);
    let mut array = FCodeFeatureArray::alloc(signature, axioms);
    for_symbol_entry(&mut array, |entry| {
        entry.key1 = symbol_type_count(signature, entry.symbol, &type_counts);
    });
    array.sort();
    assign_reverse_key1_ranks(signature, weights, &array, -1);
}

fn generate_comb_freq_rank_weights(
    signature: &mut Signature,
    axioms: &ClauseSet,
    weights: &mut [i64],
) {
    let type_counts = type_distribution(signature, axioms);
    let mut array = FCodeFeatureArray::alloc(signature, axioms);
    for_symbol_entry(&mut array, |entry| {
        let type_frequency = symbol_type_count(signature, entry.symbol, &type_counts);
        entry.key1 = type_frequency + 2 * entry.freq;
    });
    array.sort();
    assign_forward_key1_ranks(signature, weights, &array, -1);
}

fn generate_inv_comb_freq_rank_weights(
    signature: &mut Signature,
    axioms: &ClauseSet,
    weights: &mut [i64],
) {
    let type_counts = type_distribution(signature, axioms);
    let mut array = FCodeFeatureArray::alloc(signature, axioms);
    for_symbol_entry(&mut array, |entry| {
        let type_frequency = symbol_type_count(signature, entry.symbol, &type_counts);
        entry.key1 = type_frequency + 2 * entry.freq;
    });
    array.sort();
    assign_reverse_key1_ranks(signature, weights, &array, -1);
}

fn assign_inverse_modified_frequency_ranks(
    signature: &Signature,
    weights: &mut [i64],
    array: &FCodeFeatureArray,
) {
    let mut weight = 0;
    let mut base = 0;
    let mut freq = 0;
    for entry in sorted_entries(array, signature).iter().rev() {
        base += 1;
        if freq != entry.freq {
            freq = entry.freq;
            weight = base;
        }
        set_symbol_weight(weights, entry.symbol, weight * W_DEFAULT_WEIGHT);
    }
}

fn assign_forward_key1_ranks(
    signature: &Signature,
    weights: &mut [i64],
    array: &FCodeFeatureArray,
    initial_key: i64,
) {
    let mut weight = 0;
    let mut key = initial_key;
    for entry in sorted_entries(array, signature) {
        if key != entry.key1 {
            key = entry.key1;
            weight += 1;
        }
        set_symbol_weight(weights, entry.symbol, weight * W_DEFAULT_WEIGHT);
    }
}

fn assign_reverse_key1_ranks(
    signature: &Signature,
    weights: &mut [i64],
    array: &FCodeFeatureArray,
    initial_key: i64,
) {
    let mut weight = 0;
    let mut key = initial_key;
    for entry in sorted_entries(array, signature).iter().rev() {
        if key != entry.key1 {
            key = entry.key1;
            weight += 1;
        }
        set_symbol_weight(weights, entry.symbol, weight * W_DEFAULT_WEIGHT);
    }
}

fn frequency_key_array(signature: &Signature, axioms: &ClauseSet) -> FCodeFeatureArray {
    let mut array = FCodeFeatureArray::alloc(signature, axioms);
    for_symbol_entry(&mut array, |entry| {
        entry.key1 = entry.freq;
    });
    array
}

fn set_maximal_0(
    signature: &Signature,
    context: WeightGenerationContext<'_>,
    weights: &mut [i64],
) -> Result<(), Diagnostic> {
    if context.higher_order_problem {
        return Ok(());
    }
    let precedence = PrecedenceSource::new(signature, context)?;
    if let Some(symbol) = precedence.first_maximal_symbol(signature) {
        set_symbol_weight(weights, symbol, 0);
    }
    Ok(())
}

fn set_maximal_unary_0(
    signature: &Signature,
    context: WeightGenerationContext<'_>,
    weights: &mut [i64],
) -> Result<(), Diagnostic> {
    let precedence = PrecedenceSource::new(signature, context)?;
    if let Some(symbol) = precedence.first_maximal_symbol(signature) {
        if symbol_arity(signature, symbol) == 1 {
            set_symbol_weight(weights, symbol, 0);
        }
    }
    Ok(())
}

fn type_distribution(signature: &mut Signature, axioms: &ClauseSet) -> Vec<i64> {
    let size = usize::try_from(signature.type_bank().types_count() + 1)
        .unwrap_or_else(|_| panic!("type count must fit usize"));
    let mut counts = vec![0; size];
    axioms.add_type_distribution(signature, &mut counts);
    counts
}

fn symbol_frequency_type_distribution(
    signature: &Signature,
    array: &FCodeFeatureArray,
) -> Vec<i64> {
    let size = usize::try_from(signature.type_bank().types_count() + 1)
        .unwrap_or_else(|_| panic!("type count must fit usize"));
    let mut counts = vec![0; size];
    for symbol in symbols_after_true(signature) {
        let type_id = symbol_type_index(signature, symbol);
        counts[type_id] += feature_entry(array, symbol).freq;
    }
    counts
}

fn symbol_type_count(signature: &Signature, symbol: FunCode, type_counts: &[i64]) -> i64 {
    type_counts[symbol_type_index(signature, symbol)]
}

fn symbol_type_index(signature: &Signature, symbol: FunCode) -> usize {
    signature.get_type(symbol).map_or(0, |type_| {
        usize::try_from(type_.type_uid())
            .unwrap_or_else(|_| panic!("type id must fit type distribution index"))
    })
}

fn sorted_entries<'a>(
    array: &'a FCodeFeatureArray,
    signature: &Signature,
) -> &'a [FCodeFeatureSortCell] {
    let start = fcode_index(SIG_TRUE_CODE + 1);
    let end = fcode_index(signature.f_count());
    if start > end || start >= array.entries().len() {
        &[]
    } else {
        &array.entries()[start..=end]
    }
}

fn for_symbol_entry(
    array: &mut FCodeFeatureArray,
    mut update: impl FnMut(&mut FCodeFeatureSortCell),
) {
    for entry in array
        .entries_mut()
        .iter_mut()
        .skip(fcode_index(SIG_TRUE_CODE + 1))
    {
        update(entry);
    }
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss
)]
fn precrank_weight(index: usize, symb_no: usize, ranks: f32) -> i64 {
    (((index as f32) / ((symb_no as f32) / ranks)) + 1.0) as i64 * W_DEFAULT_WEIGHT
}

fn symbol_arity(signature: &Signature, symbol: FunCode) -> i64 {
    i64::from(
        signature
            .find_arity(symbol)
            .unwrap_or_else(|| panic!("valid f-code {symbol} has an arity")),
    )
}

fn feature_entry(array: &FCodeFeatureArray, symbol: FunCode) -> FCodeFeatureSortCell {
    *array
        .entry(fcode_index(symbol))
        .unwrap_or_else(|| panic!("feature array has entry for f-code {symbol}"))
}

fn symbols_after_true(signature: &Signature) -> impl Iterator<Item = FunCode> {
    (SIG_TRUE_CODE + 1)..=signature.f_count()
}

fn set_symbol_weight(weights: &mut [i64], symbol: FunCode, weight: i64) {
    weights[fcode_index(symbol)] = weight;
}

fn symbol_weight(weights: &[i64], symbol: FunCode) -> i64 {
    weights[fcode_index(symbol)]
}

fn weights_size(f_count: FunCode) -> usize {
    usize::try_from(
        f_count
            .checked_add(1)
            .unwrap_or_else(|| panic!("signature f-code count must leave room for index zero")),
    )
    .unwrap_or_else(|_| panic!("signature f-code count must fit usize"))
}

fn fcode_index(f_code: FunCode) -> usize {
    usize::try_from(f_code).unwrap_or_else(|_| panic!("f-code must fit vector index"))
}

struct PrecedenceMap {
    rank_by_symbol: Vec<usize>,
    sorted_symbols: Vec<FunCode>,
}

impl PrecedenceMap {
    fn new(signature: &Signature, order: &[FunCode]) -> Result<Self, Diagnostic> {
        let expected_len = usize::try_from(signature.f_count().saturating_sub(SIG_TRUE_CODE))
            .unwrap_or_else(|_| panic!("signature f-code count must fit usize"));
        if order.len() != expected_len {
            return Err(Diagnostic::new(
                ErrorCode::OTHER_ERROR,
                "Precedence order does not cover current signature",
            ));
        }

        let mut rank_by_symbol = vec![usize::MAX; weights_size(signature.f_count())];
        for (rank, symbol) in order.iter().copied().enumerate() {
            if symbol <= SIG_TRUE_CODE || symbol > signature.f_count() {
                return Err(Diagnostic::new(
                    ErrorCode::OTHER_ERROR,
                    "Precedence order contains symbol outside current signature",
                ));
            }
            let slot = &mut rank_by_symbol[fcode_index(symbol)];
            if *slot != usize::MAX {
                return Err(Diagnostic::new(
                    ErrorCode::OTHER_ERROR,
                    "Precedence order contains duplicate symbol",
                ));
            }
            *slot = rank;
        }

        for symbol in symbols_after_true(signature) {
            if rank_by_symbol[fcode_index(symbol)] == usize::MAX {
                return Err(Diagnostic::new(
                    ErrorCode::OTHER_ERROR,
                    "Precedence order does not cover current signature",
                ));
            }
        }

        Ok(Self {
            rank_by_symbol,
            sorted_symbols: order.to_vec(),
        })
    }

    fn compare(&self, left: FunCode, right: FunCode) -> Ordering {
        self.rank_by_symbol[fcode_index(left)].cmp(&self.rank_by_symbol[fcode_index(right)])
    }
}

enum PrecedenceSource<'a> {
    Total(PrecedenceMap),
    Ocb(&'a OrderControlBlock),
}

impl<'a> PrecedenceSource<'a> {
    fn new(
        signature: &Signature,
        context: WeightGenerationContext<'a>,
    ) -> Result<Self, Diagnostic> {
        if let Some(order) = context.precedence_order {
            return PrecedenceMap::new(signature, order).map(Self::Total);
        }
        if let Some(ocb) = context.precedence_ocb {
            return Ok(Self::Ocb(ocb));
        }
        Err(Diagnostic::new(
            ErrorCode::OTHER_ERROR,
            "Weight generation method requires precedence order",
        ))
    }

    fn compare(&self, signature: &Signature, left: FunCode, right: FunCode) -> CompareResult {
        match self {
            Self::Total(map) => ordering_to_part(map.compare(left, right)),
            Self::Ocb(ocb) => ocb.fun_compare(signature, left, right),
        }
    }

    fn first_maximal_symbol(&self, signature: &Signature) -> Option<FunCode> {
        let mut maxima = Vec::new();
        for symbol in symbols_after_true(signature) {
            let mut maximal = true;
            for index in (0..maxima.len()).rev() {
                match self.compare(signature, symbol, maxima[index]) {
                    CompareResult::Lesser => {
                        maximal = false;
                        break;
                    }
                    CompareResult::Greater => {
                        maxima.remove(index);
                    }
                    _ => {}
                }
            }
            if maximal {
                maxima.push(symbol);
            }
        }
        maxima.into_iter().min()
    }

    fn sorted_symbols(&self, signature: &mut Signature) -> Vec<FunCode> {
        match self {
            Self::Total(map) => map.sorted_symbols.clone(),
            Self::Ocb(_) => self.sorted_symbols_from_ocb(signature),
        }
    }

    fn sorted_symbols_from_ocb(&self, signature: &mut Signature) -> Vec<FunCode> {
        let mut symbols: Vec<_> = symbols_after_true(signature).collect();
        let mut alpha_ranks = vec![0; weights_size(signature.f_count())];
        for symbol in symbols.iter().copied() {
            alpha_ranks[fcode_index(symbol)] = signature.get_alpha_rank(symbol);
        }
        let signature = &*signature;
        symbols.sort_by(|left, right| {
            if left == right {
                return Ordering::Equal;
            }
            if self.compare(signature, *left, *right) == CompareResult::Lesser {
                return Ordering::Less;
            }
            if self.compare(signature, *right, *left) == CompareResult::Lesser {
                return Ordering::Greater;
            }
            alpha_ranks[fcode_index(*left)].cmp(&alpha_ranks[fcode_index(*right)])
        });
        symbols
    }
}

#[cfg(test)]
mod tests {
    use super::{
        generate_weights, generate_weights_into_ocb, GeneratedWeights, WeightGenerationContext,
        W_DEFAULT_WEIGHT, W_TO_BASEWEIGHT,
    };
    use crate::basics::error::ErrorCode;
    use crate::basics::partial_orderings::{CompareResult, HoOrderKind};
    use crate::clauses::clause::Clause;
    use crate::clauses::clause_props::CP_TYPE_CONJECTURE;
    use crate::clauses::clausesets::ClauseSet;
    use crate::clauses::eqn::Eqn;
    use crate::clauses::eqnlist::EqnList;
    use crate::heuristics::fcode_featurearrays::FCodeFeatureArray;
    use crate::heuristics::to_params::{
        OrderParmsCell, TOWeightGenMethod, TermOrdering, W_CONST_NO_SPECIAL_WEIGHT,
    };
    use crate::orderings::ocb::OrderControlBlock;
    use crate::terms::functypes::FunCode;
    use crate::terms::signature::{Signature, SIG_PHONY_APP_CODE, SIG_TRUE_CODE};
    use crate::terms::simpletypes::{alloc_arrow_type, alloc_simple_sort, Type};
    use crate::terms::termbanks::TermBank;
    use crate::terms::termtypes::{DerefType, Term};
    use crate::terms::typebanks::TypeBank;

    fn signature() -> Signature {
        let mut signature = Signature::new(TypeBank::new());
        signature
            .insert_internal_codes()
            .unwrap_or_else(|err| panic!("{err}"));
        signature
    }

    fn term_bank() -> TermBank {
        TermBank::new(signature()).unwrap_or_else(|err| panic!("{err}"))
    }

    fn individual(bank: &TermBank) -> Type {
        bank.signature().type_bank().i_type()
    }

    fn typed_symbol(signature: &mut Signature, name: &str, arity: i32) -> FunCode {
        let code = signature.insert_id(name, arity, false);
        let individual = signature.type_bank().i_type();
        let type_ = if arity == 0 {
            individual
        } else {
            let mut args = Vec::new();
            for _ in 0..arity {
                args.push(individual.clone());
            }
            args.push(individual);
            alloc_arrow_type(args)
        };
        signature
            .declare_final_type(code, type_)
            .unwrap_or_else(|err| panic!("{err}"));
        code
    }

    fn typed_const(bank: &mut TermBank, name: &str, type_: &Type) -> Term {
        let code = bank.signature_mut().insert_id(name, 0, false);
        bank.signature_mut()
            .declare_final_type(code, type_.clone())
            .unwrap_or_else(|err| panic!("{err}"));
        bank.create_const_term(code)
            .unwrap_or_else(|err| panic!("{err}"))
    }

    fn typed_unary(bank: &mut TermBank, name: &str, arg: &Term) -> Term {
        let individual = individual(bank);
        typed_unary_with_type(bank, name, arg, &individual)
    }

    fn typed_unary_with_type(bank: &mut TermBank, name: &str, arg: &Term, type_: &Type) -> Term {
        let code = bank.signature_mut().insert_id(name, 1, false);
        let function_type = bank
            .signature_mut()
            .type_bank_mut()
            .insert_type_shared(alloc_arrow_type(vec![type_.clone(), type_.clone()]));
        bank.signature_mut()
            .declare_final_type(code, function_type)
            .unwrap_or_else(|err| panic!("{err}"));
        let term = Term::top_alloc(code, 1);
        term.set_type(Some(type_.clone()));
        term.set_argument(0, arg.clone());
        bank.insert(&term, DerefType::Never)
            .unwrap_or_else(|err| panic!("{err}"))
    }

    fn literal(bank: &mut TermBank, left: &Term, right: &Term) -> Eqn {
        Eqn::alloc(left.clone(), right.clone(), bank, true).unwrap_or_else(|err| panic!("{err}"))
    }

    fn clause(literals: Vec<Eqn>) -> Clause {
        Clause::alloc(EqnList::from_vec(literals))
    }

    fn all_symbols(signature: &Signature) -> Vec<FunCode> {
        ((SIG_TRUE_CODE + 1)..=signature.f_count()).collect()
    }

    fn order_with_tail(signature: &Signature, tail: &[FunCode]) -> Vec<FunCode> {
        let mut order = Vec::new();
        for symbol in all_symbols(signature) {
            if !tail.contains(&symbol) {
                order.push(symbol);
            }
        }
        order.extend_from_slice(tail);
        order
    }

    fn order_position(order: &[FunCode], symbol: FunCode) -> usize {
        order
            .iter()
            .position(|candidate| *candidate == symbol)
            .unwrap_or_else(|| panic!("symbol {symbol} should be in order"))
    }

    fn one_based_order_position(order: &[FunCode], symbol: FunCode) -> i64 {
        i64::try_from(order_position(order, symbol) + 1).unwrap_or_else(|err| panic!("{err}"))
    }

    fn feature_frequency(array: &FCodeFeatureArray, symbol: FunCode) -> i64 {
        array
            .entry(usize::try_from(symbol).unwrap_or_else(|err| panic!("{err}")))
            .unwrap_or_else(|| panic!("feature entry for {symbol}"))
            .freq
    }

    fn weight(result: &GeneratedWeights, symbol: FunCode) -> i64 {
        result
            .weight(symbol)
            .unwrap_or_else(|| panic!("missing weight for {symbol}"))
    }

    fn raw_method_weights(
        signature: &mut Signature,
        axioms: &ClauseSet,
        params: &OrderParmsCell,
        context: WeightGenerationContext<'_>,
    ) -> GeneratedWeights {
        let mut generated = GeneratedWeights {
            weights: vec![0; super::weights_size(signature.f_count())],
            var_weight: W_DEFAULT_WEIGHT,
            lam_weight: params.lam_w,
            db_weight: params.db_w,
        };
        super::set_symbol_weight(&mut generated.weights, SIG_TRUE_CODE, W_DEFAULT_WEIGHT);
        super::apply_weight_method(signature, axioms, params, context, &mut generated)
            .unwrap_or_else(|err| panic!("{err}"));
        generated
    }

    fn params(method: TOWeightGenMethod) -> OrderParmsCell {
        OrderParmsCell {
            to_weight_gen: method,
            ..OrderParmsCell::default()
        }
    }

    #[test]
    fn constants_match_c_headers() {
        assert_eq!(W_DEFAULT_WEIGHT, 1);
        assert_eq!(W_TO_BASEWEIGHT, 4);
    }

    #[test]
    fn constant_generation_sets_symbols_and_special_weights() {
        let mut signature = signature();
        let constant = typed_symbol(&mut signature, "a", 0);
        let unary = typed_symbol(&mut signature, "f", 1);
        let params = params(TOWeightGenMethod::ConstantWeight);

        let result = generate_weights(
            &mut signature,
            &ClauseSet::new(),
            &params,
            WeightGenerationContext::default(),
        )
        .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(weight(&result, constant), 1);
        assert_eq!(weight(&result, unary), 1);
        assert_eq!(weight(&result, SIG_TRUE_CODE), result.var_weight);
        assert_eq!(weight(&result, SIG_PHONY_APP_CODE), 0);
        assert_eq!(result.var_weight, 1);
    }

    #[test]
    fn generated_weights_install_into_ocb() {
        let mut signature = signature();
        let constant = typed_symbol(&mut signature, "a", 0);
        let unary = typed_symbol(&mut signature, "f", 1);
        let mut ocb =
            OrderControlBlock::alloc(TermOrdering::Kbo, true, &signature, HoOrderKind::LfhoOrder);
        let params = params(TOWeightGenMethod::ArityWeight);

        generate_weights_into_ocb(
            &mut signature,
            &ClauseSet::new(),
            &params,
            WeightGenerationContext::default(),
            &mut ocb,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(ocb.fun_weight(constant), 1);
        assert_eq!(ocb.fun_weight(unary), 2);
        assert_eq!(ocb.var_weight, W_DEFAULT_WEIGHT);
        assert_eq!(ocb.lam_weight, params.lam_w);
        assert_eq!(ocb.db_weight, params.db_w);
    }

    #[test]
    fn generated_weights_apply_user_overrides_last() {
        let mut signature = signature();
        let constant = typed_symbol(&mut signature, "a", 0);
        let unary = typed_symbol(&mut signature, "f", 1);
        let mut ocb =
            OrderControlBlock::alloc(TermOrdering::Kbo, true, &signature, HoOrderKind::LfhoOrder);
        let params = params(TOWeightGenMethod::ArityWeight);

        generate_weights_into_ocb(
            &mut signature,
            &ClauseSet::new(),
            &params,
            WeightGenerationContext {
                pre_weights: Some("a:9"),
                ..WeightGenerationContext::default()
            },
            &mut ocb,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(ocb.fun_weight(constant), 9);
        assert_eq!(ocb.fun_weight(unary), 2);
    }

    #[test]
    fn no_method_selects_maximal_first_order_symbol() {
        let mut signature = signature();
        let constant = typed_symbol(&mut signature, "a", 0);
        let unary = typed_symbol(&mut signature, "f", 1);
        let order = order_with_tail(&signature, &[constant, unary]);
        let params = params(TOWeightGenMethod::NoMethod);

        let result = generate_weights(
            &mut signature,
            &ClauseSet::new(),
            &params,
            WeightGenerationContext {
                precedence_order: Some(&order),
                ..WeightGenerationContext::default()
            },
        )
        .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(weight(&result, constant), 1);
        assert_eq!(weight(&result, unary), 0);
    }

    #[test]
    fn select_maximal_requires_precedence_and_obeys_higher_order_skip() {
        let mut signature = signature();
        let unary = typed_symbol(&mut signature, "f", 1);
        let order = order_with_tail(&signature, &[unary]);
        let params = params(TOWeightGenMethod::SelectMaximal);

        let error = generate_weights(
            &mut signature.clone(),
            &ClauseSet::new(),
            &params,
            WeightGenerationContext::default(),
        )
        .unwrap_err();
        let higher_order = generate_weights(
            &mut signature,
            &ClauseSet::new(),
            &params,
            WeightGenerationContext {
                precedence_order: Some(&order),
                higher_order_problem: true,
                ..WeightGenerationContext::default()
            },
        )
        .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(error.code(), ErrorCode::OTHER_ERROR);
        assert_eq!(
            error.message(),
            "Weight generation method requires precedence order"
        );
        assert_eq!(weight(&higher_order, unary), 1);
    }

    #[test]
    fn arity_generation_matches_c_formulae_and_constant_override() {
        let mut signature = signature();
        let constant = typed_symbol(&mut signature, "a", 0);
        let unary = typed_symbol(&mut signature, "f", 1);
        let binary = typed_symbol(&mut signature, "g", 2);
        let maxarity = i64::from(signature.find_max_used_arity());
        let params = OrderParmsCell {
            to_weight_gen: TOWeightGenMethod::InvAritySqWeight,
            to_const_weight: W_CONST_NO_SPECIAL_WEIGHT,
            ..OrderParmsCell::default()
        };

        let result = generate_weights(
            &mut signature,
            &ClauseSet::new(),
            &params,
            WeightGenerationContext::default(),
        )
        .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(weight(&result, constant), maxarity * maxarity + 1);
        assert_eq!(weight(&result, unary), maxarity * maxarity);
        assert_eq!(weight(&result, binary), maxarity * maxarity - 3);
    }

    #[test]
    fn modarity_sets_c_var_weight_before_constant_override() {
        let mut signature = signature();
        let constant = typed_symbol(&mut signature, "a", 0);
        let unary = typed_symbol(&mut signature, "f", 1);
        let params = params(TOWeightGenMethod::ModArityWeight);

        let result = generate_weights(
            &mut signature,
            &ClauseSet::new(),
            &params,
            WeightGenerationContext::default(),
        )
        .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(weight(&result, constant), 1);
        assert_eq!(weight(&result, unary), 5);
        assert_eq!(result.var_weight, 0);
        assert_eq!(weight(&result, SIG_TRUE_CODE), 0);
    }

    #[test]
    fn precedence_weights_count_lower_and_higher_symbols() {
        let mut signature = signature();
        let a = typed_symbol(&mut signature, "a", 0);
        let f = typed_symbol(&mut signature, "f", 1);
        let g = typed_symbol(&mut signature, "g", 2);
        let order = order_with_tail(&signature, &[a, f, g]);
        let precedence_params = OrderParmsCell {
            to_weight_gen: TOWeightGenMethod::Precedence,
            to_const_weight: W_CONST_NO_SPECIAL_WEIGHT,
            ..OrderParmsCell::default()
        };
        let inverse_params = OrderParmsCell {
            to_weight_gen: TOWeightGenMethod::PrecedenceInv,
            to_const_weight: W_CONST_NO_SPECIAL_WEIGHT,
            ..OrderParmsCell::default()
        };
        let common = WeightGenerationContext {
            precedence_order: Some(&order),
            ..WeightGenerationContext::default()
        };

        let precedence = generate_weights(
            &mut signature.clone(),
            &ClauseSet::new(),
            &precedence_params,
            common,
        )
        .unwrap_or_else(|err| panic!("{err}"));
        let inverse = generate_weights(&mut signature, &ClauseSet::new(), &inverse_params, common)
            .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(weight(&precedence, a), one_based_order_position(&order, a));
        assert_eq!(weight(&precedence, f), one_based_order_position(&order, f));
        assert_eq!(weight(&precedence, g), one_based_order_position(&order, g));
        assert_eq!(
            weight(&inverse, a),
            i64::try_from(order.len() - order_position(&order, a))
                .unwrap_or_else(|err| panic!("{err}"))
        );
        assert_eq!(
            weight(&inverse, f),
            i64::try_from(order.len() - order_position(&order, f))
                .unwrap_or_else(|err| panic!("{err}"))
        );
        assert_eq!(
            weight(&inverse, g),
            i64::try_from(order.len() - order_position(&order, g))
                .unwrap_or_else(|err| panic!("{err}"))
        );
    }

    #[test]
    fn matrix_precedence_weights_count_only_comparable_symbols() {
        let mut signature = signature();
        let a = typed_symbol(&mut signature, "a", 0);
        let f = typed_symbol(&mut signature, "f", 1);
        let g = typed_symbol(&mut signature, "g", 2);
        let mut ocb =
            OrderControlBlock::alloc(TermOrdering::Kbo, false, &signature, HoOrderKind::LfhoOrder);
        ocb.precedence_add_tuple(&signature, f, a, CompareResult::Greater);
        let common = WeightGenerationContext {
            precedence_ocb: Some(&ocb),
            ..WeightGenerationContext::default()
        };
        let precedence_params = OrderParmsCell {
            to_weight_gen: TOWeightGenMethod::Precedence,
            to_const_weight: W_CONST_NO_SPECIAL_WEIGHT,
            ..OrderParmsCell::default()
        };
        let inverse_params = OrderParmsCell {
            to_weight_gen: TOWeightGenMethod::PrecedenceInv,
            to_const_weight: W_CONST_NO_SPECIAL_WEIGHT,
            ..OrderParmsCell::default()
        };

        let precedence = generate_weights(
            &mut signature.clone(),
            &ClauseSet::new(),
            &precedence_params,
            common,
        )
        .unwrap_or_else(|err| panic!("{err}"));
        let inverse = generate_weights(&mut signature, &ClauseSet::new(), &inverse_params, common)
            .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(weight(&precedence, a), 1);
        assert_eq!(weight(&precedence, f), 2);
        assert_eq!(weight(&precedence, g), 1);
        assert_eq!(weight(&inverse, a), 2);
        assert_eq!(weight(&inverse, f), 1);
        assert_eq!(weight(&inverse, g), 1);
    }

    #[test]
    fn precrank_uses_c_float_bucket_formula() {
        let mut signature = signature();
        let a = typed_symbol(&mut signature, "a", 0);
        let f = typed_symbol(&mut signature, "f", 1);
        let g = typed_symbol(&mut signature, "g", 2);
        let order = order_with_tail(&signature, &[a, f, g]);
        let params = OrderParmsCell {
            to_weight_gen: TOWeightGenMethod::PrecRank5,
            to_const_weight: W_CONST_NO_SPECIAL_WEIGHT,
            ..OrderParmsCell::default()
        };

        let result = generate_weights(
            &mut signature,
            &ClauseSet::new(),
            &params,
            WeightGenerationContext {
                precedence_order: Some(&order),
                ..WeightGenerationContext::default()
            },
        )
        .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(
            weight(&result, a),
            super::precrank_weight(order_position(&order, a), order.len(), 5.0)
        );
        assert_eq!(
            weight(&result, f),
            super::precrank_weight(order_position(&order, f), order.len(), 5.0)
        );
        assert_eq!(
            weight(&result, g),
            super::precrank_weight(order_position(&order, g), order.len(), 5.0)
        );
    }

    #[test]
    fn frequency_counts_and_inverse_counts_use_feature_array_counts() {
        let mut bank = term_bank();
        let individual = individual(&bank);
        let a = typed_const(&mut bank, "a", &individual);
        let b = typed_const(&mut bank, "b", &individual);
        let fa = typed_unary(&mut bank, "f", &a);
        let gb = typed_unary(&mut bank, "g", &b);
        let f_code = fa.f_code();
        let g_code = gb.f_code();
        let h_code = typed_symbol(bank.signature_mut(), "h", 1);
        let axioms = ClauseSet::from_clauses([
            clause(vec![literal(&mut bank, &fa, &a)]),
            clause(vec![
                literal(&mut bank, &gb, &b),
                literal(&mut bank, &gb, &a),
            ]),
        ]);
        let array = FCodeFeatureArray::alloc(bank.signature(), &axioms);
        let max_count = all_symbols(bank.signature())
            .iter()
            .map(|symbol| feature_frequency(&array, *symbol))
            .max()
            .unwrap_or(1)
            .max(1)
            + 1;
        let count_params = OrderParmsCell {
            to_weight_gen: TOWeightGenMethod::Frequency,
            to_const_weight: W_CONST_NO_SPECIAL_WEIGHT,
            ..OrderParmsCell::default()
        };
        let inv_params = OrderParmsCell {
            to_weight_gen: TOWeightGenMethod::InvFrequency,
            to_const_weight: W_CONST_NO_SPECIAL_WEIGHT,
            ..OrderParmsCell::default()
        };

        let counts = generate_weights(
            bank.signature_mut(),
            &axioms,
            &count_params,
            WeightGenerationContext::default(),
        )
        .unwrap_or_else(|err| panic!("{err}"));
        let inverse = generate_weights(
            bank.signature_mut(),
            &axioms,
            &inv_params,
            WeightGenerationContext::default(),
        )
        .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(weight(&counts, f_code), feature_frequency(&array, f_code));
        assert_eq!(weight(&counts, g_code), feature_frequency(&array, g_code));
        assert_eq!(weight(&counts, h_code), 1);
        assert_eq!(
            weight(&inverse, f_code),
            max_count - feature_frequency(&array, f_code).max(1)
        );
        assert_eq!(
            weight(&inverse, g_code),
            max_count - feature_frequency(&array, g_code).max(1)
        );
        assert_eq!(
            weight(&inverse, h_code),
            max_count - feature_frequency(&array, h_code).max(1)
        );
    }

    #[test]
    fn frequency_rank_methods_preserve_c_zero_frequency_sentinels() {
        let mut bank = term_bank();
        let individual = individual(&bank);
        let a = typed_const(&mut bank, "a", &individual);
        let fa = typed_unary(&mut bank, "f", &a);
        let f_code = fa.f_code();
        let h_code = typed_symbol(bank.signature_mut(), "h", 1);
        let axioms = ClauseSet::from_clauses([clause(vec![literal(&mut bank, &fa, &a)])]);

        let rank = generate_weights(
            bank.signature_mut(),
            &axioms,
            &params(TOWeightGenMethod::FrequencyRank),
            WeightGenerationContext::default(),
        )
        .unwrap_or_else(|err| panic!("{err}"));
        let rank_sq = generate_weights(
            bank.signature_mut(),
            &axioms,
            &params(TOWeightGenMethod::FrequencyRankSq),
            WeightGenerationContext::default(),
        )
        .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(weight(&rank, h_code), 1);
        assert_eq!(weight(&rank, f_code), 1);
        assert_eq!(weight(&rank_sq, h_code), 0);
        assert_eq!(weight(&rank_sq, f_code), 1);
    }

    #[test]
    fn inverse_modified_frequency_rank_max0_zeroes_unary_maximal_symbol() {
        let mut bank = term_bank();
        let individual = individual(&bank);
        let a = typed_const(&mut bank, "a", &individual);
        let fa = typed_unary(&mut bank, "f", &a);
        let f_code = fa.f_code();
        let h_code = typed_symbol(bank.signature_mut(), "h", 1);
        let order = order_with_tail(bank.signature(), &[h_code]);
        let axioms = ClauseSet::from_clauses([clause(vec![literal(&mut bank, &fa, &a)])]);

        let result = generate_weights(
            bank.signature_mut(),
            &axioms,
            &params(TOWeightGenMethod::InvModFreqRankMax0),
            WeightGenerationContext {
                precedence_order: Some(&order),
                ..WeightGenerationContext::default()
            },
        )
        .unwrap_or_else(|err| panic!("{err}"));

        assert!(weight(&result, f_code) > 0);
        assert_eq!(weight(&result, h_code), 0);
    }

    #[test]
    fn inverse_conjecture_frequency_rank_can_assign_c_zero_weight() {
        let mut bank = term_bank();
        let individual = individual(&bank);
        let a = typed_const(&mut bank, "a", &individual);
        let fa = typed_unary(&mut bank, "f", &a);
        let f_code = fa.f_code();
        let h_code = typed_symbol(bank.signature_mut(), "h", 1);
        let mut conjecture = clause(vec![literal(&mut bank, &fa, &a)]);
        conjecture.set_tptp_type(CP_TYPE_CONJECTURE);
        let axioms = ClauseSet::from_clauses([conjecture]);

        let result = generate_weights(
            bank.signature_mut(),
            &axioms,
            &params(TOWeightGenMethod::InvConjFrequencyRank),
            WeightGenerationContext::default(),
        )
        .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(weight(&result, h_code), 0);
        assert!(weight(&result, f_code) > weight(&result, h_code));
    }

    #[test]
    fn type_frequency_methods_use_clause_type_distribution() {
        let mut bank = term_bank();
        let animal_type_code = bank
            .signature_mut()
            .type_bank_mut()
            .define_simple_sort("animal")
            .unwrap_or_else(|err| panic!("{err}"));
        let animal = bank
            .signature_mut()
            .type_bank_mut()
            .insert_type_shared(alloc_simple_sort(animal_type_code));
        let individual = individual(&bank);
        let animal_const = typed_const(&mut bank, "animal_const", &animal);
        let ordinary_const = typed_const(&mut bank, "ordinary_const", &individual);
        let animal_symbol = animal_const.f_code();
        let ordinary_symbol = ordinary_const.f_code();
        let axioms = ClauseSet::from_clauses([
            clause(vec![literal(&mut bank, &ordinary_const, &ordinary_const)]),
            clause(vec![literal(&mut bank, &animal_const, &animal_const)]),
            clause(vec![literal(&mut bank, &animal_const, &animal_const)]),
        ]);
        let count_params = OrderParmsCell {
            to_weight_gen: TOWeightGenMethod::TypeFrequencyCount,
            to_const_weight: W_CONST_NO_SPECIAL_WEIGHT,
            ..OrderParmsCell::default()
        };
        let inv_params = OrderParmsCell {
            to_weight_gen: TOWeightGenMethod::InvTypeFrequencyCount,
            to_const_weight: W_CONST_NO_SPECIAL_WEIGHT,
            ..OrderParmsCell::default()
        };

        let counts = generate_weights(
            bank.signature_mut(),
            &axioms,
            &count_params,
            WeightGenerationContext::default(),
        )
        .unwrap_or_else(|err| panic!("{err}"));
        let inverse = generate_weights(
            bank.signature_mut(),
            &axioms,
            &inv_params,
            WeightGenerationContext::default(),
        )
        .unwrap_or_else(|err| panic!("{err}"));

        assert!(weight(&counts, animal_symbol) > weight(&counts, ordinary_symbol));
        assert!(weight(&inverse, animal_symbol) < weight(&inverse, ordinary_symbol));
    }

    fn assert_instrumented_c_fol_method_weights() {
        let mut fol_bank = term_bank();
        let fol_individual = individual(&fol_bank);
        let a = typed_const(&mut fol_bank, "a", &fol_individual);
        let fa = typed_unary(&mut fol_bank, "f", &a);
        let b = typed_const(&mut fol_bank, "b", &fol_individual);
        let ga = typed_unary(&mut fol_bank, "g", &a);
        let a_code = a.f_code();
        let f_code = fa.f_code();
        let b_code = b.f_code();
        let g_code = ga.f_code();
        // C preprocessing removes the parsed `g(a)=g(a)` tautology before
        // ordering creation but retains `g` in the signature.
        let fol_axioms = ClauseSet::from_clauses([
            clause(vec![literal(&mut fol_bank, &fa, &b)]),
            clause(vec![literal(&mut fol_bank, &fa, &b)]),
        ]);
        let mut partial = OrderControlBlock::alloc(
            TermOrdering::Kbo,
            false,
            fol_bank.signature(),
            HoOrderKind::LfhoOrder,
        );
        partial.precedence_add_tuple(fol_bank.signature(), f_code, g_code, CompareResult::Greater);

        let fol_cases = [
            (
                TOWeightGenMethod::Precedence,
                [1, 2, 1, 1],
                WeightGenerationContext {
                    precedence_ocb: Some(&partial),
                    ..WeightGenerationContext::default()
                },
            ),
            (
                TOWeightGenMethod::PrecedenceInv,
                [1, 1, 1, 2],
                WeightGenerationContext {
                    precedence_ocb: Some(&partial),
                    ..WeightGenerationContext::default()
                },
            ),
            (
                TOWeightGenMethod::PrecRank5,
                [5, 5, 5, 5],
                WeightGenerationContext {
                    precedence_ocb: Some(&partial),
                    ..WeightGenerationContext::default()
                },
            ),
            (
                TOWeightGenMethod::InvConjFrequencyRank,
                [1, 1, 1, 2],
                WeightGenerationContext::default(),
            ),
            (
                TOWeightGenMethod::FrequencyRankSq,
                [1, 1, 1, 0],
                WeightGenerationContext::default(),
            ),
            (
                TOWeightGenMethod::InvModFreqRank,
                [1, 1, 1, 4],
                WeightGenerationContext::default(),
            ),
        ];
        for (method, expected, context) in fol_cases {
            let result = raw_method_weights(
                fol_bank.signature_mut(),
                &fol_axioms,
                &OrderParmsCell {
                    to_weight_gen: method,
                    to_const_weight: W_CONST_NO_SPECIAL_WEIGHT,
                    ..OrderParmsCell::default()
                },
                context,
            );
            assert_eq!(
                [
                    weight(&result, a_code),
                    weight(&result, f_code),
                    weight(&result, b_code),
                    weight(&result, g_code),
                ],
                expected,
                "instrumented C FOL snapshot for {method:?}"
            );
        }
    }

    fn assert_instrumented_c_late_override() {
        let mut fol_bank = term_bank();
        let fol_individual = individual(&fol_bank);
        let a = typed_const(&mut fol_bank, "a", &fol_individual);
        let fa = typed_unary(&mut fol_bank, "f", &a);
        let b = typed_const(&mut fol_bank, "b", &fol_individual);
        let ga = typed_unary(&mut fol_bank, "g", &a);
        let overridden = generate_weights(
            fol_bank.signature_mut(),
            &ClauseSet::new(),
            &OrderParmsCell {
                to_weight_gen: TOWeightGenMethod::ArityWeight,
                to_const_weight: 3,
                ..OrderParmsCell::default()
            },
            WeightGenerationContext {
                pre_weights: Some("a:9"),
                ..WeightGenerationContext::default()
            },
        )
        .unwrap_or_else(|err| panic!("{err}"));
        assert_eq!(
            [
                weight(&overridden, a.f_code()),
                weight(&overridden, fa.f_code()),
                weight(&overridden, b.f_code()),
                weight(&overridden, ga.f_code()),
            ],
            [9, 2, 3, 2]
        );
    }

    fn assert_instrumented_c_typed_weight_arrays() {
        let mut typed_bank = term_bank();
        let animal_code = typed_bank
            .signature_mut()
            .type_bank_mut()
            .define_simple_sort("animal")
            .unwrap_or_else(|err| panic!("{err}"));
        let animal = typed_bank
            .signature_mut()
            .type_bank_mut()
            .insert_type_shared(alloc_simple_sort(animal_code));
        let individual = individual(&typed_bank);
        let cat = typed_const(&mut typed_bank, "cat", &animal);
        let dog = typed_const(&mut typed_bank, "dog", &animal);
        let ordinary = typed_const(&mut typed_bank, "a", &individual);
        let fcat = typed_unary_with_type(&mut typed_bank, "f", &cat, &animal);
        let ga = typed_unary(&mut typed_bank, "g", &ordinary);
        let typed_codes = [
            cat.f_code(),
            dog.f_code(),
            ordinary.f_code(),
            fcat.f_code(),
            ga.f_code(),
        ];
        let typed_axioms = ClauseSet::from_clauses([
            clause(vec![literal(&mut typed_bank, &fcat, &dog)]),
            clause(vec![literal(&mut typed_bank, &fcat, &cat)]),
            clause(vec![literal(&mut typed_bank, &ga, &ordinary)]),
        ]);
        let typed_cases = [
            (TOWeightGenMethod::TypeFrequencyRank, [4, 4, 3, 3, 2]),
            (TOWeightGenMethod::TypeFrequencyCount, [4, 4, 2, 2, 1]),
            (TOWeightGenMethod::InvTypeFrequencyRank, [1, 1, 2, 2, 3]),
            (TOWeightGenMethod::InvTypeFrequencyCount, [1, 1, 3, 3, 4]),
            (TOWeightGenMethod::CombFrequencyRank, [4, 3, 3, 3, 2]),
            (TOWeightGenMethod::CombFrequencyCount, [10, 6, 6, 6, 3]),
            (TOWeightGenMethod::InvCombFrequencyRank, [1, 2, 2, 2, 3]),
            (TOWeightGenMethod::InvCombFrequencyCount, [1, 5, 5, 5, 8]),
        ];
        for (method, expected) in typed_cases {
            let result = raw_method_weights(
                typed_bank.signature_mut(),
                &typed_axioms,
                &OrderParmsCell {
                    to_weight_gen: method,
                    to_const_weight: W_CONST_NO_SPECIAL_WEIGHT,
                    ..OrderParmsCell::default()
                },
                WeightGenerationContext {
                    higher_order_problem: true,
                    ..WeightGenerationContext::default()
                },
            );
            assert_eq!(
                typed_codes.map(|code| weight(&result, code)),
                expected,
                "instrumented C LFHO snapshot for {method:?}"
            );
        }
    }

    #[test]
    fn instrumented_c_reference_weight_arrays_match() {
        assert_instrumented_c_fol_method_weights();
        assert_instrumented_c_late_override();
        assert_instrumented_c_typed_weight_arrays();
    }

    #[test]
    fn pure_weight_generation_applies_late_user_weight_overrides() {
        let mut signature = signature();
        let a = typed_symbol(&mut signature, "a", 0);
        let f = typed_symbol(&mut signature, "f", 1);
        let params = params(TOWeightGenMethod::ConstantWeight);

        let result = generate_weights(
            &mut signature,
            &ClauseSet::new(),
            &params,
            WeightGenerationContext {
                pre_weights: Some("a:9"),
                ..WeightGenerationContext::default()
            },
        )
        .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(weight(&result, a), 9);
        assert_eq!(weight(&result, f), W_DEFAULT_WEIGHT);

        assert_eq!(
            generate_weights(
                &mut signature,
                &ClauseSet::new(),
                &params,
                WeightGenerationContext {
                    pre_weights: Some(""),
                    ..WeightGenerationContext::default()
                },
            )
            .unwrap_or_else(|err| panic!("{err}")),
            generate_weights(
                &mut signature,
                &ClauseSet::new(),
                &params,
                WeightGenerationContext::default(),
            )
            .unwrap_or_else(|err| panic!("{err}"))
        );
    }

    #[test]
    fn invalid_method_reports_unimplemented_diagnostic() {
        let mut signature = signature();
        let params = params(TOWeightGenMethod::InvalidEntry);

        let error = generate_weights(
            &mut signature,
            &ClauseSet::new(),
            &params,
            WeightGenerationContext::default(),
        )
        .unwrap_err();

        assert_eq!(error.code(), ErrorCode::OTHER_ERROR);
        assert_eq!(error.message(), "Weight generation method unimplemented");
    }
}
