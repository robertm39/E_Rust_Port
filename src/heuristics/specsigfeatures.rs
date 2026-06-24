use crate::clauses::clause::Clause;
use crate::clauses::clausesets::ClauseSet;
use crate::clauses::eqn::Eqn;
use crate::terms::signature::Signature;
use crate::terms::termbanks::TermBank;
use crate::terms::termtypes::Term;
use std::fmt::Write as _;

pub const SIG_FEATURE_ARITY_LIMIT_WIDTH: usize = 6;
pub const SPECSIG_SIGFTRS: usize = 3 * SIG_FEATURE_ARITY_LIMIT_WIDTH;
pub const SPECSIG_CS_FTRS: usize = 2 + 2 * SPECSIG_SIGFTRS;
pub const SPECSIG_TOTAL_FTR_NO: usize =
    (2 * SPECSIG_CS_FTRS) + 3 + (2 * SIG_FEATURE_ARITY_LIMIT_WIDTH);

pub const SPECSIG_POS_EL_OFFSET: usize = 0;
pub const SPECSIG_NEG_EL_OFFSET: usize = 1;
pub const SPECSIG_SYMD_OFFSET: usize = 2;

pub const SPECSIG_AX_FTRS: usize = 0;
pub const SPECSIG_AX_POSEQ: usize = SPECSIG_AX_FTRS + SPECSIG_POS_EL_OFFSET;
pub const SPECSIG_AX_NEGEQ: usize = SPECSIG_AX_FTRS + SPECSIG_NEG_EL_OFFSET;
pub const SPECSIG_AX_SYMD: usize = SPECSIG_AX_FTRS + SPECSIG_SYMD_OFFSET;
pub const SPECSIG_AX_SYMD_POS: usize = SPECSIG_AX_SYMD;
pub const SPECSIG_AX_SYMD_NEG: usize = SPECSIG_AX_SYMD_POS + SPECSIG_SIGFTRS;

pub const SPECSIG_CJ_FTRS: usize = SPECSIG_CS_FTRS;
pub const SPECSIG_CJ_POSEQ: usize = SPECSIG_CJ_FTRS + SPECSIG_POS_EL_OFFSET;
pub const SPECSIG_CJ_NEGEQ: usize = SPECSIG_CJ_FTRS + SPECSIG_NEG_EL_OFFSET;
pub const SPECSIG_CJ_SYMD: usize = SPECSIG_CJ_FTRS + SPECSIG_SYMD_OFFSET;
pub const SPECSIG_CJ_SYMD_POS: usize = SPECSIG_CJ_SYMD;
pub const SPECSIG_CJ_SYMD_NEG: usize = SPECSIG_CJ_SYMD_POS + SPECSIG_SIGFTRS;

pub const SPECSIG_GLOBAL_FTRS: usize = 2 * SPECSIG_CS_FTRS;
pub const SPECSIG_GLOBAL_UNIT: usize = SPECSIG_GLOBAL_FTRS;
pub const SPECSIG_GLOBAL_HORN: usize = SPECSIG_GLOBAL_FTRS + 1;
pub const SPECSIG_GLOBAL_GNRL: usize = SPECSIG_GLOBAL_FTRS + 2;
pub const SPECSIG_GLOBAL_SIG: usize = (2 * SPECSIG_CS_FTRS) + 3;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SpecSigFeatureCell {
    pub features: [i64; SPECSIG_TOTAL_FTR_NO],
}

impl Default for SpecSigFeatureCell {
    fn default() -> Self {
        Self::new()
    }
}

impl SpecSigFeatureCell {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            features: [0; SPECSIG_TOTAL_FTR_NO],
        }
    }

    pub fn init(&mut self) {
        spec_sig_feature_init(self);
    }
}

pub fn spec_sig_feature_init(specftrs: &mut SpecSigFeatureCell) {
    specftrs.features.fill(0);
}

#[must_use]
pub fn spec_sig_feature_format(specftrs: &SpecSigFeatureCell) -> String {
    let mut result = String::new();
    for (index, feature) in specftrs.features.iter().enumerate() {
        if index != 0 {
            result.push_str(", ");
        }
        let _ = write!(&mut result, "{feature:5}");
    }
    result
}

/// Collects signature arity and function-depth features from one term.
///
/// # Panics
///
/// Panics if `features` has fewer than [`SPECSIG_SIGFTRS`] entries, if a
/// positive f-code is not valid in `signature`, if a non-variable term has an
/// uninitialized argument, or if term depth overflows `i64`.
pub fn term_collect_sig_features(signature: &mut Signature, term: &Term, features: &mut [i64]) {
    assert!(
        features.len() >= SPECSIG_SIGFTRS,
        "term signature features require {SPECSIG_SIGFTRS} slots"
    );

    let mut stack = vec![(term.clone(), 1_i64)];
    while let Some((current, depth)) = stack.pop() {
        let f_code = current.f_code();
        if f_code <= 0 {
            continue;
        }

        let feature_offset = feature_index(signature.get_feature_offset(f_code));
        features[feature_offset] += 1;

        if !signature.is_predicate(f_code) {
            let depth_index = feature_index(signature.get_depth_feature_offset(f_code));
            if depth > features[depth_index] {
                features[depth_index] = depth;
            }
        }

        let next_depth = depth
            .checked_add(1)
            .unwrap_or_else(|| panic!("term signature feature depth overflow"));
        for arg in current.argument_clones().into_iter().rev() {
            stack.push((
                arg.unwrap_or_else(|| {
                    panic!("signature feature collection requires initialized term arguments")
                }),
                next_depth,
            ));
        }
    }
}

/// Collects signature features for both sides of one literal.
///
/// # Panics
///
/// Panics under the same conditions as [`term_collect_sig_features`].
pub fn eqn_collect_sig_features(bank: &mut TermBank, eqn: &Eqn, features: &mut [i64]) {
    term_collect_sig_features(bank.signature_mut(), eqn.left(), features);
    term_collect_sig_features(bank.signature_mut(), eqn.right(), features);
}

/// Adds one clause's positive/negative signature features to a clause vector.
///
/// # Panics
///
/// Panics if `features` has fewer than [`SPECSIG_CS_FTRS`] entries, if a
/// literal violates the bank-backed equational-literal invariants, or under the
/// same conditions as [`term_collect_sig_features`].
pub fn clause_collect_sig_features(bank: &mut TermBank, clause: &Clause, features: &mut [i64]) {
    assert!(
        features.len() >= SPECSIG_CS_FTRS,
        "clause signature features require {SPECSIG_CS_FTRS} slots"
    );

    for literal in clause.literals().as_slice() {
        if literal.is_positive() {
            eqn_collect_sig_features(
                bank,
                literal,
                &mut features[SPECSIG_SYMD_OFFSET..SPECSIG_SYMD_OFFSET + SPECSIG_SIGFTRS],
            );
            if literal.is_equ_lit(bank) {
                features[SPECSIG_POS_EL_OFFSET] += 1;
            }
        } else {
            eqn_collect_sig_features(
                bank,
                literal,
                &mut features[SPECSIG_SYMD_OFFSET + SPECSIG_SIGFTRS
                    ..SPECSIG_SYMD_OFFSET + (2 * SPECSIG_SIGFTRS)],
            );
            if literal.is_equ_lit(bank) {
                features[SPECSIG_NEG_EL_OFFSET] += 1;
            }
        }
    }
}

/// Zeros and then computes one clause's signature features.
///
/// # Panics
///
/// Panics under the same conditions as [`clause_collect_sig_features`].
pub fn clause_compute_sig_features(bank: &mut TermBank, clause: &Clause, features: &mut [i64]) {
    assert!(
        features.len() >= SPECSIG_CS_FTRS,
        "clause signature features require {SPECSIG_CS_FTRS} slots"
    );
    features[..SPECSIG_CS_FTRS].fill(0);
    clause_collect_sig_features(bank, clause, features);
}

/// Adds clause-set signature features and global external-signature counts.
///
/// # Panics
///
/// Panics under the same conditions as [`clause_collect_sig_features`], or if
/// an external signature f-code has no valid feature offset.
pub fn clause_set_collect_sig_features(
    bank: &mut TermBank,
    set: &ClauseSet,
    specftrs: &mut SpecSigFeatureCell,
) {
    for clause in set.iter() {
        if clause.is_conjecture() {
            clause_collect_sig_features(
                bank,
                clause,
                &mut specftrs.features[SPECSIG_CJ_FTRS..SPECSIG_CJ_FTRS + SPECSIG_CS_FTRS],
            );
        } else {
            clause_collect_sig_features(
                bank,
                clause,
                &mut specftrs.features[SPECSIG_AX_FTRS..SPECSIG_AX_FTRS + SPECSIG_CS_FTRS],
            );
        }

        if clause.is_unit() {
            specftrs.features[SPECSIG_GLOBAL_UNIT] += 1;
        } else if clause.is_horn() {
            specftrs.features[SPECSIG_GLOBAL_HORN] += 1;
        } else {
            specftrs.features[SPECSIG_GLOBAL_GNRL] += 1;
        }
    }

    let internal_symbols = bank.signature().internal_symbols();
    let f_count = bank.signature().f_count();
    for f_code in (internal_symbols + 1)..=f_count {
        let offset = feature_index(bank.signature_mut().get_feature_offset(f_code));
        specftrs.features[SPECSIG_GLOBAL_SIG + offset] += 1;
    }
}

fn feature_index(offset: i32) -> usize {
    usize::try_from(offset)
        .unwrap_or_else(|_| panic!("signature feature offset must be non-negative"))
}

#[cfg(test)]
mod tests {
    use super::{
        clause_collect_sig_features, clause_compute_sig_features, clause_set_collect_sig_features,
        spec_sig_feature_format, spec_sig_feature_init, term_collect_sig_features,
        SpecSigFeatureCell, SIG_FEATURE_ARITY_LIMIT_WIDTH, SPECSIG_AX_POSEQ, SPECSIG_AX_SYMD_POS,
        SPECSIG_CJ_SYMD_NEG, SPECSIG_CS_FTRS, SPECSIG_GLOBAL_GNRL, SPECSIG_GLOBAL_HORN,
        SPECSIG_GLOBAL_SIG, SPECSIG_GLOBAL_UNIT, SPECSIG_SIGFTRS, SPECSIG_TOTAL_FTR_NO,
    };
    use crate::clauses::clause::Clause;
    use crate::clauses::clause_props::CP_TYPE_CONJECTURE;
    use crate::clauses::clausesets::ClauseSet;
    use crate::clauses::eqn::Eqn;
    use crate::clauses::eqnlist::EqnList;
    use crate::terms::signature::{Signature, SIG_FEATURE_ARITY_LIMIT};
    use crate::terms::simpletypes::alloc_arrow_type;
    use crate::terms::termbanks::TermBank;
    use crate::terms::termtypes::{DerefType, Term};
    use crate::terms::typebanks::TypeBank;

    #[test]
    fn constants_and_init_match_c_vector_layout() {
        assert_eq!(
            SIG_FEATURE_ARITY_LIMIT_WIDTH,
            SIG_FEATURE_ARITY_LIMIT as usize
        );
        assert_eq!(SPECSIG_SIGFTRS, 18);
        assert_eq!(SPECSIG_CS_FTRS, 38);
        assert_eq!(SPECSIG_TOTAL_FTR_NO, 91);

        let mut features = SpecSigFeatureCell::new();
        features.features[0] = 7;
        features.features[SPECSIG_TOTAL_FTR_NO - 1] = 9;
        spec_sig_feature_init(&mut features);

        assert!(features.features.iter().all(|feature| *feature == 0));
    }

    #[test]
    fn format_prints_comma_separated_five_wide_values() {
        let mut features = SpecSigFeatureCell::new();
        features.features[0] = 1;
        features.features[1] = -2;
        features.features[2] = 30;

        let rendered = spec_sig_feature_format(&features);

        assert!(rendered.starts_with("    1,    -2,    30,     0"));
    }

    #[test]
    fn term_collection_counts_predicate_and_function_arity_depth_features() {
        let (mut bank, codes) = fixture_bank();
        let function = function_term(&mut bank, codes.f, codes.a);
        let term = predicate_term(&mut bank, codes.p, &[function]);
        let mut features = [0; SPECSIG_SIGFTRS];

        term_collect_sig_features(bank.signature_mut(), &term, &mut features);

        assert_eq!(features[0], 1);
        assert_eq!(features[1], 1);
        assert_eq!(features[7], 1);
        assert_eq!(features[12], 3);
        assert_eq!(features[13], 2);
    }

    #[test]
    fn clause_collection_splits_positive_and_negative_literals() {
        let (mut bank, codes) = fixture_bank();
        let positive_eq = positive_equality(&mut bank, codes);
        let negative_pred = negative_predicate(&mut bank, codes);
        let clause = Clause::alloc(EqnList::from_vec(vec![negative_pred, positive_eq]));
        let mut features = [0; SPECSIG_CS_FTRS];

        clause_collect_sig_features(&mut bank, &clause, &mut features);

        assert_eq!(features[0], 1);
        assert_eq!(features[1], 0);
        assert_eq!(features[2], 2);
        assert_eq!(features[3], 1);
        assert_eq!(features[14], 2);
        assert_eq!(features[15], 1);
        assert_eq!(features[20], 1);
        assert_eq!(features[21], 1);
        assert_eq!(features[27], 1);
        assert_eq!(features[32], 3);
        assert_eq!(features[33], 2);
    }

    #[test]
    fn clause_compute_zeroes_existing_clause_feature_slots() {
        let (mut bank, codes) = fixture_bank();
        let clause = Clause::alloc(EqnList::from_vec(vec![positive_equality(&mut bank, codes)]));
        let mut features = [99; SPECSIG_CS_FTRS + 2];

        clause_compute_sig_features(&mut bank, &clause, &mut features);

        assert_eq!(features[0], 1);
        assert_eq!(features[SPECSIG_CS_FTRS], 99);
        assert_eq!(features[SPECSIG_CS_FTRS + 1], 99);
    }

    #[test]
    fn clause_set_collection_accumulates_axiom_conjecture_and_global_features() {
        let (mut bank, codes) = fixture_bank();
        let unit_axiom =
            Clause::alloc(EqnList::from_vec(vec![positive_equality(&mut bank, codes)]));
        let mut horn_conjecture = Clause::alloc(EqnList::from_vec(vec![
            positive_equality(&mut bank, codes),
            negative_predicate(&mut bank, codes),
        ]));
        horn_conjecture.set_tptp_type(CP_TYPE_CONJECTURE);
        let general_axiom = Clause::alloc(EqnList::from_vec(vec![
            positive_equality(&mut bank, codes),
            positive_equality(&mut bank, codes),
        ]));
        let set = ClauseSet::from_clauses([unit_axiom, horn_conjecture, general_axiom]);
        let mut spec = SpecSigFeatureCell::new();

        clause_set_collect_sig_features(&mut bank, &set, &mut spec);

        assert_eq!(spec.features[SPECSIG_AX_POSEQ], 3);
        assert_eq!(spec.features[SPECSIG_AX_SYMD_POS], 6);
        assert_eq!(spec.features[SPECSIG_AX_SYMD_POS + 1], 3);
        assert_eq!(spec.features[SPECSIG_CJ_SYMD_NEG + 7], 1);
        assert_eq!(spec.features[SPECSIG_GLOBAL_UNIT], 1);
        assert_eq!(spec.features[SPECSIG_GLOBAL_HORN], 1);
        assert_eq!(spec.features[SPECSIG_GLOBAL_GNRL], 1);
        assert_eq!(spec.features[SPECSIG_GLOBAL_SIG], 1);
        assert_eq!(spec.features[SPECSIG_GLOBAL_SIG + 1], 1);
        assert_eq!(spec.features[SPECSIG_GLOBAL_SIG + 7], 1);
    }

    #[derive(Clone, Copy)]
    struct Codes {
        a: i64,
        f: i64,
        p: i64,
    }

    fn fixture_bank() -> (TermBank, Codes) {
        let mut signature = Signature::new(TypeBank::new());
        let individual = signature.type_bank().i_type();
        let bool_type = signature.type_bank().bool_type();
        let f_type = signature
            .type_bank_mut()
            .insert_type_shared(alloc_arrow_type(vec![
                individual.clone(),
                individual.clone(),
            ]));
        let p_type = signature
            .type_bank_mut()
            .insert_type_shared(alloc_arrow_type(vec![individual.clone(), bool_type]));
        let a = signature.insert_id("a", 0, false);
        signature.declare_final_type(a, individual).unwrap();
        let f = signature.insert_id("f", 1, false);
        signature.declare_final_type(f, f_type).unwrap();
        let p = signature.insert_id("p", 1, false);
        signature.declare_final_type(p, p_type).unwrap();
        (TermBank::new(signature).unwrap(), Codes { a, f, p })
    }

    fn shared_const(bank: &mut TermBank, f_code: i64) -> Term {
        let type_ = bank.signature().get_type(f_code).cloned();
        let term = Term::const_cell_alloc(f_code);
        term.set_type(type_);
        bank.insert(&term, DerefType::Never).unwrap()
    }

    fn function_term(bank: &mut TermBank, f_code: i64, arg_code: i64) -> Term {
        let type_ = bank.signature().type_bank().i_type();
        let term = Term::top_alloc(f_code, 1);
        term.set_type(Some(type_));
        term.set_argument(0, shared_const(bank, arg_code));
        bank.insert(&term, DerefType::Never).unwrap()
    }

    fn predicate_term(bank: &mut TermBank, f_code: i64, args: &[Term]) -> Term {
        let type_ = bank.signature().type_bank().bool_type();
        let term = Term::top_alloc(f_code, args.len());
        term.set_type(Some(type_));
        for (index, arg) in args.iter().enumerate() {
            term.set_argument(index, arg.clone());
        }
        bank.insert(&term, DerefType::Never).unwrap()
    }

    fn positive_equality(bank: &mut TermBank, codes: Codes) -> Eqn {
        Eqn::alloc(
            function_term(bank, codes.f, codes.a),
            shared_const(bank, codes.a),
            bank,
            true,
        )
        .unwrap()
    }

    fn negative_predicate(bank: &mut TermBank, codes: Codes) -> Eqn {
        let function = function_term(bank, codes.f, codes.a);
        let atom = predicate_term(bank, codes.p, &[function]);
        Eqn::alloc(atom, bank.true_term().clone(), bank, false).unwrap()
    }
}
