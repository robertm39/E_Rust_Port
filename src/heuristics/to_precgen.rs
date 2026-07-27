//! Precedence key generation from `che_to_precgen`.

use crate::basics::error::{Diagnostic, ErrorCode};
use crate::basics::partial_orderings::CompareResult;
use crate::clauses::clausesets::ClauseSet;
use crate::heuristics::fcode_featurearrays::{
    FCodeFeatureArray, FCodeFeatureKeyModifiers, FCodeFeatureSortCell,
};
use crate::heuristics::to_params::{OrderParmsCell, TOPrecGenMethod};
use crate::inout::scanner::Scanner;
use crate::orderings::cto_orderings::precedence_parse;
use crate::orderings::ocb::OrderControlBlock;
use crate::terms::functypes::FunCode;
use crate::terms::signature::{Signature, SIG_TRUE_CODE};

pub const FREQ_SEMI_INFTY: i64 = 2_000_000;

const C_INT_MAX: i64 = 2_147_483_647;

/// Generates the C feature array used to install an ordering precedence.
///
/// The returned array is sorted exactly as `TOGeneratePrecedence` sorts it, but
/// the final mutation of OCB precedence weights/tuples remains with the future
/// ordering-control port.
///
/// # Errors
///
/// Returns a diagnostic for the C `orient_axioms` method, which also reports
/// "Not yet implemented" in the original.
///
/// # Panics
///
/// Panics if the signature contains an invalid positive f-code or if
/// type-frequency generation observes a type id outside the C-sized
/// distribution array.
pub fn generate_precedence_array(
    signature: &mut Signature,
    axioms: &ClauseSet,
    oparms: &OrderParmsCell,
) -> Result<FCodeFeatureArray, Diagnostic> {
    let mut array = FCodeFeatureArray::alloc(signature, axioms);
    let modifiers = FCodeFeatureKeyModifiers {
        conj_only_mod: oparms.conj_only_mod,
        conj_axiom_mod: oparms.conj_axiom_mod,
        axiom_only_mod: oparms.axiom_only_mod,
        skolem_mod: oparms.skolem_mod,
        defpred_mod: oparms.defpred_mod,
    };
    array.update_occ_key(&modifiers);
    array.update_symb_key(signature, &modifiers);
    apply_precedence_method(signature, axioms, &mut array, oparms.to_prec_gen)?;
    array.sort();
    Ok(array)
}

/// Returns the low-to-high symbol order encoded by `TOGeneratePrecedence`.
///
/// C writes the sorted array position as each symbol's precedence weight and
/// handles `$true` separately. This helper therefore starts at
/// `SIG_TRUE_CODE + 1` and returns the symbols in sorted array order.
///
/// # Errors
///
/// Returns a diagnostic for unsupported C precedence-generation methods.
///
/// # Panics
///
/// Panics under the same preconditions as [`generate_precedence_array`].
pub fn generate_precedence_order(
    signature: &mut Signature,
    axioms: &ClauseSet,
    oparms: &OrderParmsCell,
) -> Result<Vec<FunCode>, Diagnostic> {
    let array = generate_precedence_array(signature, axioms, oparms)?;
    Ok(precedence_order_from_array(&array, signature.f_count()))
}

/// Install a generated or predefined precedence into an OCB.
///
/// # Errors
///
/// Returns scanner diagnostics for predefined precedence strings or generation
/// diagnostics for unsupported precedence methods.
///
/// # Panics
///
/// Panics if the OCB was allocated for a different signature snapshot, or if it
/// has neither precedence weights nor a precedence matrix.
pub fn generate_precedence_into_ocb(
    signature: &mut Signature,
    axioms: &ClauseSet,
    predefined: Option<&str>,
    oparms: &OrderParmsCell,
    ocb: &mut OrderControlBlock,
) -> Result<(), Diagnostic> {
    generate_precedence_into_ocb_with_order(signature, axioms, predefined, oparms, ocb).map(|_| ())
}

/// Install a generated or predefined precedence into an OCB and return the
/// generated total order when one was produced.
///
/// A predefined-only precedence in C is matrix-backed and can be partial; this
/// helper therefore returns `None` when `predefined` is present and
/// `oparms.to_prec_gen` is `PNoMethod`.
///
/// # Errors
///
/// Returns scanner diagnostics for predefined precedence strings or generation
/// diagnostics for unsupported precedence methods.
///
/// # Panics
///
/// Panics if the OCB was allocated for a different signature snapshot, or if it
/// has neither precedence weights nor a precedence matrix.
pub fn generate_precedence_into_ocb_with_order(
    signature: &mut Signature,
    axioms: &ClauseSet,
    predefined: Option<&str>,
    oparms: &OrderParmsCell,
    ocb: &mut OrderControlBlock,
) -> Result<Option<Vec<FunCode>>, Diagnostic> {
    assert_eq!(
        ocb.sig_size,
        signature.f_count(),
        "precedence generation requires a current OCB signature snapshot"
    );
    assert!(
        ocb.prec_weights.is_some() || ocb.precedence.is_some(),
        "precedence generation requires OCB precedence storage"
    );

    if let Some(predefined) = predefined {
        let mut scanner = Scanner::from_user_string(predefined, true)?;
        precedence_parse(&mut scanner, signature, ocb)?;
        if oparms.to_prec_gen == TOPrecGenMethod::NoMethod {
            return Ok(None);
        }
    }

    let array = generate_precedence_array(signature, axioms, oparms)?;
    let order = precedence_order_from_array(&array, signature.f_count());
    install_precedence_array(signature, &array, ocb);
    Ok(Some(order))
}

/// Install a sorted precedence array into an OCB.
///
/// # Panics
///
/// Panics if the array size does not match the OCB signature snapshot, or if
/// the OCB has neither precedence weights nor a precedence matrix.
pub fn install_precedence_array(
    signature: &Signature,
    array: &FCodeFeatureArray,
    ocb: &mut OrderControlBlock,
) {
    assert_eq!(
        ocb.sig_size,
        FunCode::try_from(array.size())
            .unwrap_or_else(|_| panic!("feature-array size must fit f-code"))
            - 1,
        "precedence array must match OCB signature size"
    );

    if ocb.prec_weights.is_some() {
        for index in fcode_index(SIG_TRUE_CODE + 1)
            .unwrap_or_else(|| panic!("SIG_TRUE_CODE + 1 must fit array index"))
            ..=fcode_index(ocb.sig_size).unwrap_or_else(|| panic!("sig_size must fit array index"))
        {
            let symbol = FunCode::try_from(index)
                .unwrap_or_else(|_| panic!("feature-array index must fit f-code"));
            if signature.find_arity(symbol) == Some(0)
                && !signature.is_predicate(symbol)
                && !signature.is_special(symbol)
            {
                if let Some(type_) = signature.get_type(symbol) {
                    ocb.cond_set_min_const(type_, symbol);
                }
            }
            let rank = i64::try_from(index)
                .unwrap_or_else(|_| panic!("precedence rank must fit signed long"));
            ocb.set_fun_prec_weight(array.entries()[index].symbol, rank);
        }
        ocb.set_fun_prec_weight(SIG_TRUE_CODE, i64::MIN / 2);
    } else if ocb.precedence.is_some() {
        let mut last = SIG_TRUE_CODE;
        for index in fcode_index(SIG_TRUE_CODE + 1)
            .unwrap_or_else(|| panic!("SIG_TRUE_CODE + 1 must fit array index"))
            ..=fcode_index(ocb.sig_size).unwrap_or_else(|| panic!("sig_size must fit array index"))
        {
            let symbol = array.entries()[index].symbol;
            ocb.precedence_add_tuple(signature, last, symbol, CompareResult::Lesser);
            last = symbol;
        }
    } else {
        panic!("precedence array installation requires OCB precedence storage");
    }
}

/// C macro-compatible default precedence generation (`PUnaryFirst`).
///
/// # Errors
///
/// This currently has no error path, but keeps the same `Result` shape as the
/// configurable generator.
///
/// # Panics
///
/// Panics under the same preconditions as [`generate_precedence_array`].
pub fn generate_default_precedence_order(
    signature: &mut Signature,
    axioms: &ClauseSet,
) -> Result<Vec<FunCode>, Diagnostic> {
    let oparms = OrderParmsCell {
        to_prec_gen: TOPrecGenMethod::UnaryFirst,
        ..OrderParmsCell::default()
    };
    generate_precedence_order(signature, axioms, &oparms)
}

fn apply_precedence_method(
    signature: &mut Signature,
    axioms: &ClauseSet,
    array: &mut FCodeFeatureArray,
    method: TOPrecGenMethod,
) -> Result<(), Diagnostic> {
    match method {
        TOPrecGenMethod::NoMethod | TOPrecGenMethod::UnaryFirst => {
            generate_unary_first_precedence(signature, array);
            Ok(())
        }
        TOPrecGenMethod::UnaryFirstFreq => {
            generate_unary_first_freq_precedence(signature, array);
            Ok(())
        }
        TOPrecGenMethod::Arity => {
            generate_arity_precedence(signature, array);
            Ok(())
        }
        TOPrecGenMethod::InvArity => {
            generate_invarity_precedence(signature, array);
            Ok(())
        }
        TOPrecGenMethod::ConstMax => {
            generate_const_max_precedence(signature, array);
            Ok(())
        }
        TOPrecGenMethod::InvArConstMin => {
            generate_const_min_precedence(signature, array);
            Ok(())
        }
        TOPrecGenMethod::ByFrequency => {
            generate_freq_precedence(signature, array);
            Ok(())
        }
        TOPrecGenMethod::ByInvFrequency => {
            generate_invfreq_precedence(signature, array);
            Ok(())
        }
        TOPrecGenMethod::ByInvConjFrequency => {
            generate_invconjfreq_precedence(signature, array);
            Ok(())
        }
        TOPrecGenMethod::ByInvFreqConjMax => {
            generate_invfreq_conjmax_precedence(signature, array);
            Ok(())
        }
        TOPrecGenMethod::ByInvFreqConjMin => {
            generate_invfreq_conjmin_precedence(signature, array);
            Ok(())
        }
        TOPrecGenMethod::ByInvFreqConstMin => {
            generate_invfreq_constmin_precedence(signature, array);
            Ok(())
        }
        TOPrecGenMethod::ByInvFreqHack => {
            generate_invfreq_hack_precedence(signature, array);
            Ok(())
        }
        TOPrecGenMethod::ByTypeFreq => {
            generate_type_freq_precedence(signature, axioms, array);
            Ok(())
        }
        TOPrecGenMethod::ByInvTypeFreq => {
            generate_inv_type_freq_precedence(signature, axioms, array);
            Ok(())
        }
        TOPrecGenMethod::ByCombFreq => {
            generate_comb_freq_precedence(signature, axioms, array);
            Ok(())
        }
        TOPrecGenMethod::ByInvCombFreq => {
            generate_inv_comb_freq_precedence(signature, axioms, array);
            Ok(())
        }
        TOPrecGenMethod::ArrayOpt => {
            generate_arrayopt_precedence(signature, array);
            Ok(())
        }
        TOPrecGenMethod::OrientAxioms => Err(Diagnostic::new(
            ErrorCode::OTHER_ERROR,
            "Not yet implemented",
        )),
        TOPrecGenMethod::InvalidEntry => Err(Diagnostic::new(
            ErrorCode::OTHER_ERROR,
            "Precedence generation method unimplemented",
        )),
    }
}

fn generate_unary_first_precedence(signature: &Signature, array: &mut FCodeFeatureArray) {
    for_symbol_entry(array, |entry| {
        let arity = signature_arity(signature, entry.symbol);
        entry.key1 = if arity == 1 { C_INT_MAX } else { arity };
    });
}

fn generate_unary_first_freq_precedence(signature: &Signature, array: &mut FCodeFeatureArray) {
    for_symbol_entry(array, |entry| {
        match signature_arity(signature, entry.symbol) {
            1 => entry.key1 = 2,
            0 => entry.key1 = 0,
            _ => entry.key1 = 1,
        }
        entry.key2 = entry.freq;
    });
}

fn generate_arity_precedence(signature: &Signature, array: &mut FCodeFeatureArray) {
    for_symbol_entry(array, |entry| {
        entry.key1 = signature_arity(signature, entry.symbol);
    });
}

fn generate_invarity_precedence(signature: &Signature, array: &mut FCodeFeatureArray) {
    for_symbol_entry(array, |entry| {
        entry.key1 = -signature_arity(signature, entry.symbol);
    });
}

fn generate_const_max_precedence(signature: &Signature, array: &mut FCodeFeatureArray) {
    for_symbol_entry(array, |entry| {
        entry.key1 = signature_arity(signature, entry.symbol);
        if entry.key1 == 0 {
            entry.key1 = C_INT_MAX;
        }
    });
}

fn generate_const_min_precedence(signature: &Signature, array: &mut FCodeFeatureArray) {
    for_symbol_entry(array, |entry| {
        entry.key1 = -signature_arity(signature, entry.symbol);
        if entry.key1 == 0 {
            entry.key1 = -FREQ_SEMI_INFTY;
        }
    });
}

fn generate_freq_precedence(signature: &Signature, array: &mut FCodeFeatureArray) {
    for_symbol_entry(array, |entry| {
        entry.key1 = entry.freq;
        entry.key2 = signature_arity(signature, entry.symbol);
    });
}

fn generate_invfreq_precedence(signature: &Signature, array: &mut FCodeFeatureArray) {
    for_symbol_entry(array, |entry| {
        entry.key1 = -entry.freq;
        entry.key2 = signature_arity(signature, entry.symbol);
    });
}

fn generate_invconjfreq_precedence(signature: &Signature, array: &mut FCodeFeatureArray) {
    for_symbol_entry(array, |entry| {
        entry.key1 = if entry.conjfreq != 0 {
            C_INT_MAX - entry.conjfreq
        } else {
            0
        };
        entry.key2 = -entry.freq;
        entry.key3 = signature_arity(signature, entry.symbol);
    });
}

fn generate_invfreq_conjmax_precedence(signature: &Signature, array: &mut FCodeFeatureArray) {
    for_symbol_entry(array, |entry| {
        entry.key1 = i64::from(entry.conjfreq != 0);
        entry.key2 = -entry.freq;
        entry.key3 = signature_arity(signature, entry.symbol);
    });
}

fn generate_invfreq_conjmin_precedence(signature: &Signature, array: &mut FCodeFeatureArray) {
    for_symbol_entry(array, |entry| {
        entry.key1 = i64::from(entry.conjfreq == 0);
        entry.key2 = -entry.freq;
        entry.key3 = signature_arity(signature, entry.symbol);
    });
}

fn generate_invfreq_constmin_precedence(signature: &Signature, array: &mut FCodeFeatureArray) {
    for_symbol_entry(array, |entry| {
        let arity = signature_arity(signature, entry.symbol);
        if arity == 0 {
            entry.key1 = -FREQ_SEMI_INFTY;
            entry.key2 = entry.freq;
        } else {
            entry.key1 = -entry.freq;
            entry.key2 = arity;
        }
    });
}

fn generate_invfreq_hack_precedence(signature: &Signature, array: &mut FCodeFeatureArray) {
    let max_unary_frequency = array
        .entries()
        .iter()
        .skip(1)
        .filter(|entry| signature_arity(signature, entry.symbol) == 1)
        .map(|entry| entry.freq)
        .max()
        .unwrap_or(-1);

    for_symbol_entry(array, |entry| {
        let arity = signature_arity(signature, entry.symbol);
        if arity == 0 {
            entry.key1 = -FREQ_SEMI_INFTY;
            entry.key2 = -entry.freq;
        } else if arity == 1 && entry.freq == max_unary_frequency {
            entry.key1 = FREQ_SEMI_INFTY;
            entry.key2 = 0;
        } else {
            entry.key1 = -entry.freq;
            entry.key2 = arity;
        }
    });
}

fn generate_type_freq_precedence(
    signature: &mut Signature,
    axioms: &ClauseSet,
    array: &mut FCodeFeatureArray,
) {
    let type_counts = type_distribution(signature, axioms);
    for_symbol_entry(array, |entry| {
        entry.key1 = symbol_type_frequency(signature, entry.symbol, &type_counts);
        entry.key2 = entry.freq;
    });
}

fn generate_comb_freq_precedence(
    signature: &mut Signature,
    axioms: &ClauseSet,
    array: &mut FCodeFeatureArray,
) {
    let type_counts = type_distribution(signature, axioms);
    for_symbol_entry(array, |entry| {
        entry.key1 = symbol_type_frequency(signature, entry.symbol, &type_counts) + 2 * entry.freq;
        entry.key2 = entry.freq;
    });
}

fn generate_inv_type_freq_precedence(
    signature: &mut Signature,
    axioms: &ClauseSet,
    array: &mut FCodeFeatureArray,
) {
    let type_counts = type_distribution(signature, axioms);
    for_symbol_entry(array, |entry| {
        entry.key1 = -symbol_type_frequency(signature, entry.symbol, &type_counts);
        entry.key2 = entry.freq;
    });
}

fn generate_inv_comb_freq_precedence(
    signature: &mut Signature,
    axioms: &ClauseSet,
    array: &mut FCodeFeatureArray,
) {
    let type_counts = type_distribution(signature, axioms);
    for_symbol_entry(array, |entry| {
        entry.key1 =
            -(symbol_type_frequency(signature, entry.symbol, &type_counts) + 2 * entry.freq);
        entry.key2 = entry.freq;
    });
}

fn generate_arrayopt_precedence(signature: &Signature, array: &mut FCodeFeatureArray) {
    for_symbol_entry(array, |entry| {
        let name = signature.find_name(entry.symbol).unwrap_or("");
        entry.key1 = if name == "store" {
            30
        } else if name == "select" {
            25
        } else if name == "sk" {
            20
        } else if name.starts_with("a_") || name.starts_with("b_") {
            10
        } else if name.starts_with('a') || name.starts_with('b') {
            15
        } else if name.starts_with("e_") {
            5
        } else if name.starts_with('e') {
            7
        } else if name.starts_with("i_") {
            0
        } else if name.starts_with('i') {
            2
        } else {
            5
        };
        entry.key2 = -entry.freq;
    });
}

fn for_symbol_entry(
    array: &mut FCodeFeatureArray,
    mut update: impl FnMut(&mut FCodeFeatureSortCell),
) {
    for entry in array.entries_mut().iter_mut().skip(1) {
        debug_assert!(entry.symbol > 0);
        update(entry);
    }
}

fn type_distribution(signature: &mut Signature, axioms: &ClauseSet) -> Vec<i64> {
    let size = usize::try_from(signature.type_bank().types_count() + 1)
        .expect("type count must fit usize");
    let mut counts = vec![0; size];
    axioms.add_type_distribution(signature, &mut counts);
    counts
}

fn symbol_type_frequency(signature: &Signature, symbol: FunCode, type_counts: &[i64]) -> i64 {
    signature
        .get_type(symbol)
        .and_then(|type_| usize::try_from(type_.type_uid()).ok())
        .map_or(0, |index| {
            *type_counts
                .get(index)
                .unwrap_or_else(|| panic!("type id {index} fits distribution array"))
        })
}

fn signature_arity(signature: &Signature, symbol: FunCode) -> i64 {
    i64::from(
        signature
            .find_arity(symbol)
            .unwrap_or_else(|| panic!("valid f-code {symbol} has an arity")),
    )
}

fn precedence_order_from_array(array: &FCodeFeatureArray, f_count: FunCode) -> Vec<FunCode> {
    let Some(start) = fcode_index(SIG_TRUE_CODE + 1) else {
        return Vec::new();
    };
    let Some(end) = fcode_index(f_count) else {
        return Vec::new();
    };
    if start > end || start >= array.entries().len() {
        return Vec::new();
    }
    array.entries()[start..=end]
        .iter()
        .map(|entry| entry.symbol)
        .collect()
}

fn fcode_index(f_code: FunCode) -> Option<usize> {
    usize::try_from(f_code).ok()
}

#[cfg(test)]
mod tests {
    use super::{
        generate_default_precedence_order, generate_precedence_array, generate_precedence_into_ocb,
        generate_precedence_order, FREQ_SEMI_INFTY,
    };
    use crate::basics::error::ErrorCode;
    use crate::basics::partial_orderings::{CompareResult, HoOrderKind};
    use crate::clauses::clause::Clause;
    use crate::clauses::clause_props::{CP_TYPE_AXIOM, CP_TYPE_CONJECTURE};
    use crate::clauses::clausesets::ClauseSet;
    use crate::clauses::eqn::Eqn;
    use crate::clauses::eqnlist::EqnList;
    use crate::heuristics::to_params::{OrderParmsCell, TOPrecGenMethod, TermOrdering};
    use crate::orderings::ocb::OrderControlBlock;
    use crate::terms::functypes::FunCode;
    use crate::terms::signature::{Signature, SIG_TRUE_CODE};
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

    fn user_order(order: &[FunCode], symbols: &[FunCode]) -> Vec<FunCode> {
        order
            .iter()
            .copied()
            .filter(|symbol| symbols.contains(symbol))
            .collect()
    }

    #[test]
    fn constant_matches_c_header() {
        assert_eq!(FREQ_SEMI_INFTY, 2_000_000);
    }

    #[test]
    fn no_method_falls_through_to_unary_first_precedence() {
        let mut sig = signature();
        let constant = typed_symbol(&mut sig, "a", 0);
        let unary = typed_symbol(&mut sig, "f", 1);
        let binary = typed_symbol(&mut sig, "g", 2);
        let symbols = [constant, unary, binary];
        let axioms = ClauseSet::new();
        let no_method = OrderParmsCell {
            to_prec_gen: TOPrecGenMethod::NoMethod,
            ..OrderParmsCell::default()
        };
        let unary_first = OrderParmsCell {
            to_prec_gen: TOPrecGenMethod::UnaryFirst,
            ..OrderParmsCell::default()
        };

        let mut no_method_signature = sig.clone();
        let no_method_order =
            generate_precedence_order(&mut no_method_signature, &axioms, &no_method)
                .unwrap_or_else(|err| panic!("{err}"));
        let unary_order = generate_precedence_order(&mut sig, &axioms, &unary_first)
            .unwrap_or_else(|err| panic!("{err}"));
        let mut default_sig = signature();
        let expected_default_len =
            usize::try_from(default_sig.f_count()).unwrap_or_else(|err| panic!("{err}")) - 1;

        assert_eq!(no_method_order, unary_order);
        assert_eq!(
            user_order(&unary_order, &symbols),
            vec![constant, binary, unary]
        );
        assert_eq!(
            generate_default_precedence_order(&mut default_sig, &axioms)
                .unwrap_or_else(|err| panic!("{err}"))
                .len(),
            expected_default_len
        );
    }

    #[test]
    fn generated_precedence_installs_rank_weights_into_ocb() {
        let mut signature = signature();
        let individual = signature.type_bank().i_type();
        let constant = typed_symbol(&mut signature, "a", 0);
        let _unary = typed_symbol(&mut signature, "f", 1);
        let _binary = typed_symbol(&mut signature, "g", 2);
        let axioms = ClauseSet::new();
        let params = OrderParmsCell {
            to_prec_gen: TOPrecGenMethod::UnaryFirst,
            ..OrderParmsCell::default()
        };
        let mut order_signature = signature.clone();
        let order = generate_precedence_order(&mut order_signature, &axioms, &params)
            .unwrap_or_else(|err| panic!("{err}"));
        let mut ocb =
            OrderControlBlock::alloc(TermOrdering::Kbo, true, &signature, HoOrderKind::LfhoOrder);

        generate_precedence_into_ocb(&mut signature, &axioms, None, &params, &mut ocb)
            .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(ocb.fun_prec_weight(SIG_TRUE_CODE), i64::MIN / 2);
        for (offset, symbol) in order.iter().copied().enumerate() {
            let rank =
                i64::try_from(offset).unwrap_or_else(|err| panic!("{err}")) + SIG_TRUE_CODE + 1;
            assert_eq!(ocb.fun_prec_weight(symbol), rank);
        }
        assert_eq!(ocb.min_const(&individual), constant);
    }

    #[test]
    fn generated_precedence_installs_matrix_chain_into_ocb() {
        let mut signature = signature();
        typed_symbol(&mut signature, "a", 0);
        typed_symbol(&mut signature, "f", 1);
        typed_symbol(&mut signature, "g", 2);
        let axioms = ClauseSet::new();
        let params = OrderParmsCell {
            to_prec_gen: TOPrecGenMethod::UnaryFirst,
            ..OrderParmsCell::default()
        };
        let mut order_signature = signature.clone();
        let order = generate_precedence_order(&mut order_signature, &axioms, &params)
            .unwrap_or_else(|err| panic!("{err}"));
        let mut ocb =
            OrderControlBlock::alloc(TermOrdering::Lpo, false, &signature, HoOrderKind::LfhoOrder);

        generate_precedence_into_ocb(&mut signature, &axioms, None, &params, &mut ocb)
            .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(
            ocb.fun_compare(&signature, SIG_TRUE_CODE, order[0]),
            CompareResult::Lesser
        );
        for pair in order.windows(2) {
            assert_eq!(
                ocb.fun_compare(&signature, pair[0], pair[1]),
                CompareResult::Lesser
            );
        }
        assert_eq!(
            ocb.fun_compare(&signature, *order.last().unwrap_or(&0), order[0]),
            CompareResult::Greater
        );
    }

    #[test]
    fn predefined_no_method_precedence_does_not_generate_fallthrough() {
        let mut signature = signature();
        let a = typed_symbol(&mut signature, "a", 0);
        let b = typed_symbol(&mut signature, "b", 0);
        let c = typed_symbol(&mut signature, "c", 0);
        let mut ocb =
            OrderControlBlock::alloc(TermOrdering::Lpo, false, &signature, HoOrderKind::LfhoOrder);
        let params = OrderParmsCell {
            to_prec_gen: TOPrecGenMethod::NoMethod,
            ..OrderParmsCell::default()
        };

        generate_precedence_into_ocb(
            &mut signature,
            &ClauseSet::new(),
            Some("a > b"),
            &params,
            &mut ocb,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(ocb.fun_compare(&signature, a, b), CompareResult::Greater);
        assert_eq!(
            ocb.fun_compare(&signature, a, c),
            CompareResult::Uncomparable
        );
    }

    #[test]
    fn arrayopt_precedence_uses_c_name_classes() {
        let mut signature = signature();
        let i_under = typed_symbol(&mut signature, "i_slot", 0);
        let i_plain = typed_symbol(&mut signature, "idx", 0);
        let e_under = typed_symbol(&mut signature, "e_slot", 0);
        let e_plain = typed_symbol(&mut signature, "elem", 0);
        let a_under = typed_symbol(&mut signature, "a_slot", 0);
        let a_plain = typed_symbol(&mut signature, "arr", 0);
        let sk = typed_symbol(&mut signature, "sk", 0);
        let select = typed_symbol(&mut signature, "select", 0);
        let store = typed_symbol(&mut signature, "store", 0);
        let symbols = [
            i_under, i_plain, e_under, e_plain, a_under, a_plain, sk, select, store,
        ];
        let params = OrderParmsCell {
            to_prec_gen: TOPrecGenMethod::ArrayOpt,
            ..OrderParmsCell::default()
        };

        let order = generate_precedence_order(&mut signature, &ClauseSet::new(), &params)
            .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(
            user_order(&order, &symbols),
            vec![i_under, i_plain, e_under, e_plain, a_under, a_plain, sk, select, store]
        );
    }

    #[test]
    fn frequency_and_conjecture_methods_use_feature_array_counts() {
        let mut bank = term_bank();
        let individual = individual(&bank);
        let a = typed_const(&mut bank, "a", &individual);
        let b = typed_const(&mut bank, "b", &individual);
        let fa = typed_unary(&mut bank, "f", &a);
        let gb = typed_unary(&mut bank, "g", &b);
        let f_code = fa.f_code();
        let g_code = gb.f_code();
        let mut axiom = clause(vec![literal(&mut bank, &fa, &a)]);
        axiom.set_tptp_type(CP_TYPE_AXIOM);
        let mut conjecture = clause(vec![
            literal(&mut bank, &gb, &a),
            literal(&mut bank, &gb, &b),
        ]);
        conjecture.set_tptp_type(CP_TYPE_CONJECTURE);
        let axioms = ClauseSet::from_clauses([axiom, conjecture]);
        let params = OrderParmsCell {
            to_prec_gen: TOPrecGenMethod::ByInvFreqConjMax,
            ..OrderParmsCell::default()
        };

        let order = generate_precedence_order(bank.signature_mut(), &axioms, &params)
            .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(user_order(&order, &[f_code, g_code]), vec![f_code, g_code]);
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
        let type_freq = OrderParmsCell {
            to_prec_gen: TOPrecGenMethod::ByTypeFreq,
            ..OrderParmsCell::default()
        };
        let inv_type_freq = OrderParmsCell {
            to_prec_gen: TOPrecGenMethod::ByInvTypeFreq,
            ..OrderParmsCell::default()
        };

        let order = generate_precedence_order(bank.signature_mut(), &axioms, &type_freq)
            .unwrap_or_else(|err| panic!("{err}"));
        let inv_order = generate_precedence_order(bank.signature_mut(), &axioms, &inv_type_freq)
            .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(
            user_order(&order, &[ordinary_symbol, animal_symbol]),
            vec![ordinary_symbol, animal_symbol]
        );
        assert_eq!(
            user_order(&inv_order, &[ordinary_symbol, animal_symbol]),
            vec![animal_symbol, ordinary_symbol]
        );
    }

    #[test]
    fn modifiers_feed_key0_before_method_specific_keys() {
        let mut bank = term_bank();
        let individual = individual(&bank);
        let axiom_only = typed_const(&mut bank, "axiom_only", &individual);
        let conj_only = typed_const(&mut bank, "conj_only", &individual);
        let axiom_code = axiom_only.f_code();
        let conj_code = conj_only.f_code();
        let mut axiom = clause(vec![literal(&mut bank, &axiom_only, &axiom_only)]);
        axiom.set_tptp_type(CP_TYPE_AXIOM);
        let mut conjecture = clause(vec![literal(&mut bank, &conj_only, &conj_only)]);
        conjecture.set_tptp_type(CP_TYPE_CONJECTURE);
        let axioms = ClauseSet::from_clauses([axiom, conjecture]);
        let params = OrderParmsCell {
            to_prec_gen: TOPrecGenMethod::Arity,
            conj_only_mod: 50,
            axiom_only_mod: -50,
            ..OrderParmsCell::default()
        };

        let order = generate_precedence_order(bank.signature_mut(), &axioms, &params)
            .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(
            user_order(&order, &[axiom_code, conj_code]),
            vec![axiom_code, conj_code]
        );
    }

    fn assert_instrumented_c_fol_precedence_orders() {
        let mut bank = term_bank();
        let individual = individual(&bank);
        let a = typed_const(&mut bank, "a", &individual);
        let fa = typed_unary(&mut bank, "f", &a);
        let b = typed_const(&mut bank, "b", &individual);
        let ga = typed_unary(&mut bank, "g", &a);
        let [a_code, f_code, b_code, g_code] = [a.f_code(), fa.f_code(), b.f_code(), ga.f_code()];
        let symbols = [a_code, f_code, b_code, g_code];
        let mut axiom = clause(vec![literal(&mut bank, &fa, &b)]);
        axiom.set_tptp_type(CP_TYPE_AXIOM);
        let mut conjecture = clause(vec![literal(&mut bank, &ga, &b)]);
        conjecture.set_tptp_type(CP_TYPE_CONJECTURE);
        let axioms = ClauseSet::from_clauses([axiom, conjecture]);

        let low_b_a_f_g = [b_code, a_code, f_code, g_code];
        let low_f_g_b_a = [f_code, g_code, b_code, a_code];
        let low_f_b_a_g = [f_code, b_code, a_code, g_code];
        let low_b_a_g_f = [b_code, a_code, g_code, f_code];
        let cases: [(TOPrecGenMethod, [FunCode; 4]); 13] = [
            (TOPrecGenMethod::UnaryFirst, low_b_a_f_g),
            (TOPrecGenMethod::UnaryFirstFreq, low_b_a_f_g),
            (TOPrecGenMethod::Arity, low_b_a_f_g),
            (TOPrecGenMethod::InvArity, low_f_g_b_a),
            (TOPrecGenMethod::ConstMax, low_f_g_b_a),
            (TOPrecGenMethod::InvArConstMin, low_b_a_f_g),
            (TOPrecGenMethod::ByFrequency, low_f_g_b_a),
            (TOPrecGenMethod::ByInvFrequency, low_b_a_f_g),
            (TOPrecGenMethod::ByInvConjFrequency, low_f_b_a_g),
            (TOPrecGenMethod::ByInvFreqConjMax, low_f_b_a_g),
            (TOPrecGenMethod::ByInvFreqConjMin, low_b_a_g_f),
            (TOPrecGenMethod::ByInvFreqConstMin, low_b_a_f_g),
            (TOPrecGenMethod::ByInvFreqHack, low_b_a_f_g),
        ];
        for (method, expected) in cases {
            let order = generate_precedence_order(
                bank.signature_mut(),
                &axioms,
                &OrderParmsCell {
                    to_prec_gen: method,
                    ..OrderParmsCell::default()
                },
            )
            .unwrap_or_else(|err| panic!("{err}"));
            assert_eq!(
                user_order(&order, &symbols),
                expected,
                "instrumented C FOL precedence for {method:?}"
            );
        }
    }

    fn assert_instrumented_c_typed_precedence_orders() {
        let mut bank = term_bank();
        let animal_code = bank
            .signature_mut()
            .type_bank_mut()
            .define_simple_sort("animal")
            .unwrap_or_else(|err| panic!("{err}"));
        let animal = bank
            .signature_mut()
            .type_bank_mut()
            .insert_type_shared(alloc_simple_sort(animal_code));
        let individual = individual(&bank);
        let cat = typed_const(&mut bank, "cat", &animal);
        let dog = typed_const(&mut bank, "dog", &animal);
        let ordinary = typed_const(&mut bank, "a", &individual);
        let fcat = typed_unary_with_type(&mut bank, "f", &cat, &animal);
        let ga = typed_unary(&mut bank, "g", &ordinary);
        let [cat_code, dog_code, a_code, f_code, g_code] = [
            cat.f_code(),
            dog.f_code(),
            ordinary.f_code(),
            fcat.f_code(),
            ga.f_code(),
        ];
        let symbols = [cat_code, dog_code, a_code, f_code, g_code];
        let axioms = ClauseSet::from_clauses([
            clause(vec![literal(&mut bank, &fcat, &dog)]),
            clause(vec![literal(&mut bank, &fcat, &cat)]),
            clause(vec![literal(&mut bank, &ga, &ordinary)]),
        ]);
        let cases = [
            (
                TOPrecGenMethod::ByTypeFreq,
                [g_code, f_code, a_code, dog_code, cat_code],
            ),
            (
                TOPrecGenMethod::ByInvTypeFreq,
                [dog_code, cat_code, f_code, a_code, g_code],
            ),
            (
                TOPrecGenMethod::ByCombFreq,
                [g_code, dog_code, f_code, a_code, cat_code],
            ),
            (
                TOPrecGenMethod::ByInvCombFreq,
                [cat_code, dog_code, f_code, a_code, g_code],
            ),
        ];
        for (method, expected) in cases {
            let order = generate_precedence_order(
                bank.signature_mut(),
                &axioms,
                &OrderParmsCell {
                    to_prec_gen: method,
                    ..OrderParmsCell::default()
                },
            )
            .unwrap_or_else(|err| panic!("{err}"));
            assert_eq!(
                user_order(&order, &symbols),
                expected,
                "instrumented C LFHO precedence for {method:?}"
            );
        }
    }

    fn assert_instrumented_c_arrayopt_precedence_order() {
        let mut signature = signature();
        let symbols = [
            typed_symbol(&mut signature, "i_slot", 0),
            typed_symbol(&mut signature, "idx", 0),
            typed_symbol(&mut signature, "e_slot", 0),
            typed_symbol(&mut signature, "elem", 0),
            typed_symbol(&mut signature, "a_slot", 0),
            typed_symbol(&mut signature, "arr", 0),
            typed_symbol(&mut signature, "sk", 0),
            typed_symbol(&mut signature, "select", 0),
            typed_symbol(&mut signature, "store", 0),
        ];
        let order = generate_precedence_order(
            &mut signature,
            &ClauseSet::new(),
            &OrderParmsCell {
                to_prec_gen: TOPrecGenMethod::ArrayOpt,
                ..OrderParmsCell::default()
            },
        )
        .unwrap_or_else(|err| panic!("{err}"));
        assert_eq!(user_order(&order, &symbols), symbols);
    }

    #[test]
    fn instrumented_c_reference_precedence_orders_match() {
        assert_instrumented_c_fol_precedence_orders();
        assert_instrumented_c_typed_precedence_orders();
        assert_instrumented_c_arrayopt_precedence_order();
    }

    #[test]
    fn orient_axioms_reports_c_not_implemented_error_after_array_setup() {
        let mut signature = signature();
        let params = OrderParmsCell {
            to_prec_gen: TOPrecGenMethod::OrientAxioms,
            ..OrderParmsCell::default()
        };

        let error =
            generate_precedence_array(&mut signature, &ClauseSet::new(), &params).unwrap_err();

        assert_eq!(error.code(), ErrorCode::OTHER_ERROR);
        assert_eq!(error.message(), "Not yet implemented");
    }
}
