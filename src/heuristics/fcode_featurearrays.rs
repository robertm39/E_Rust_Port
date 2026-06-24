use crate::clauses::clausesets::ClauseSet;
use crate::terms::functypes::FunCode;
use crate::terms::signature::{Signature, FP_DEF_PRED, FP_SKOLEM_SYMBOL, SIG_TRUE_CODE};
use std::cmp::Ordering;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct FCodeFeatureSortCell {
    pub key0: i64,
    pub key1: i64,
    pub key2: i64,
    pub key3: i64,
    pub freq: i64,
    pub conjfreq: i64,
    pub axiomfreq: i64,
    pub pos_rank: i64,
    pub symbol: FunCode,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct FCodeFeatureKeyModifiers {
    pub conj_only_mod: i64,
    pub conj_axiom_mod: i64,
    pub axiom_only_mod: i64,
    pub skolem_mod: i64,
    pub defpred_mod: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FCodeFeatureArray {
    array: Vec<FCodeFeatureSortCell>,
}

impl FCodeFeatureArray {
    /// Allocates and initializes feature cells for every positive f-code.
    ///
    /// # Panics
    ///
    /// Panics if the signature f-code count cannot be represented as a Rust
    /// vector size.
    #[must_use]
    pub fn alloc(signature: &Signature, axioms: &ClauseSet) -> Self {
        let size = feature_array_size(signature.f_count());
        let mut rank_array = vec![0; size];
        let mut dist_array = vec![0; size];
        let mut conjdist_array = vec![0; size];
        let mut axiomdist_array = vec![0; size];
        let mut rank = 0;

        axioms.compute_function_ranks(&mut rank_array, &mut rank);
        axioms.add_symbol_distribution(&mut dist_array);
        axioms.add_conj_symbol_distribution(&mut conjdist_array);
        axioms.add_axiom_symbol_distribution(&mut axiomdist_array);

        let mut array = vec![FCodeFeatureSortCell::default(); size];
        for symbol in 1..=signature.f_count() {
            let index = fcode_index(symbol);
            array[index] = FCodeFeatureSortCell {
                key0: 0,
                key1: 0,
                key2: 0,
                key3: 0,
                freq: dist_array[index],
                conjfreq: conjdist_array[index],
                axiomfreq: axiomdist_array[index],
                pos_rank: rank_array[index],
                symbol,
            };
        }

        Self { array }
    }

    #[must_use]
    pub fn size(&self) -> usize {
        self.array.len()
    }

    #[must_use]
    pub fn entries(&self) -> &[FCodeFeatureSortCell] {
        &self.array
    }

    pub fn entries_mut(&mut self) -> &mut [FCodeFeatureSortCell] {
        &mut self.array
    }

    #[must_use]
    pub fn entry(&self, index: usize) -> Option<&FCodeFeatureSortCell> {
        self.array.get(index)
    }

    pub fn update_occ_key(&mut self, modifiers: &FCodeFeatureKeyModifiers) {
        for entry in self.array.iter_mut().skip(1) {
            if entry.conjfreq != 0 {
                if entry.axiomfreq != 0 {
                    entry.key0 += modifiers.conj_axiom_mod;
                } else {
                    entry.key0 += modifiers.conj_only_mod;
                }
            } else if entry.axiomfreq != 0 {
                entry.key0 += modifiers.axiom_only_mod;
            }
        }
    }

    /// Adds key modifiers for Skolem and definition-predicate symbols.
    ///
    /// # Panics
    ///
    /// Panics if an array position cannot be represented as a signature
    /// f-code.
    pub fn update_symb_key(&mut self, signature: &Signature, modifiers: &FCodeFeatureKeyModifiers) {
        for index in 1..self.array.len() {
            let symbol = FunCode::try_from(index)
                .unwrap_or_else(|_| panic!("feature-array index must fit FunCode"));
            if signature.query_prop(symbol, FP_SKOLEM_SYMBOL) {
                self.array[index].key0 += modifiers.skolem_mod;
            }
            if signature.query_prop(symbol, FP_DEF_PRED) {
                self.array[index].key0 += modifiers.defpred_mod;
            }
        }
    }

    pub fn sort(&mut self) {
        let start = fcode_index(SIG_TRUE_CODE + 1);
        if start < self.array.len() {
            self.array[start..].sort_unstable_by(feature_compare);
        }
    }
}

fn feature_compare(left: &FCodeFeatureSortCell, right: &FCodeFeatureSortCell) -> Ordering {
    left.key0
        .cmp(&right.key0)
        .then_with(|| left.key1.cmp(&right.key1))
        .then_with(|| left.key2.cmp(&right.key2))
        .then_with(|| left.key3.cmp(&right.key3))
        .then_with(|| left.pos_rank.cmp(&right.pos_rank))
}

fn feature_array_size(f_count: FunCode) -> usize {
    usize::try_from(
        f_count
            .checked_add(1)
            .unwrap_or_else(|| panic!("signature f-code count must leave room for index zero")),
    )
    .unwrap_or_else(|_| panic!("signature f-code count must fit usize"))
}

fn fcode_index(f_code: FunCode) -> usize {
    usize::try_from(f_code).unwrap_or_else(|_| panic!("f-code must fit feature-array index"))
}

#[cfg(test)]
mod tests {
    use super::{FCodeFeatureArray, FCodeFeatureKeyModifiers};
    use crate::clauses::clause::Clause;
    use crate::clauses::clause_props::{CP_TYPE_AXIOM, CP_TYPE_CONJECTURE};
    use crate::clauses::clausesets::ClauseSet;
    use crate::clauses::eqn::Eqn;
    use crate::clauses::eqnlist::EqnList;
    use crate::terms::functypes::FunCode;
    use crate::terms::signature::{Signature, FP_DEF_PRED, FP_SKOLEM_SYMBOL, SIG_TRUE_CODE};
    use crate::terms::simpletypes::{alloc_arrow_type, Type};
    use crate::terms::termbanks::TermBank;
    use crate::terms::termtypes::{DerefType, Term};
    use crate::terms::typebanks::TypeBank;

    fn term_bank() -> TermBank {
        let mut signature = Signature::new(TypeBank::new());
        signature
            .insert_internal_codes()
            .unwrap_or_else(|err| panic!("{err}"));
        TermBank::new(signature).unwrap_or_else(|err| panic!("{err}"))
    }

    fn individual(bank: &TermBank) -> Type {
        bank.signature().type_bank().default_type()
    }

    fn typed_const(bank: &mut TermBank, name: &str) -> Term {
        let type_ = individual(bank);
        let f_code = bank.signature_mut().insert_id(name, 0, false);
        bank.signature_mut()
            .declare_final_type(f_code, type_.clone())
            .unwrap_or_else(|err| panic!("{err}"));
        bank.create_const_term(f_code)
            .unwrap_or_else(|err| panic!("{err}"))
    }

    fn typed_unary(bank: &mut TermBank, name: &str, arg: &Term) -> Term {
        let type_ = individual(bank);
        let f_code = bank.signature_mut().insert_id(name, 1, false);
        bank.signature_mut()
            .declare_final_type(f_code, alloc_arrow_type(vec![type_.clone(), type_.clone()]))
            .unwrap_or_else(|err| panic!("{err}"));
        let term = Term::top_alloc(f_code, 1);
        term.set_type(Some(type_));
        term.set_argument(0, arg.clone());
        bank.insert(&term, DerefType::Never)
            .unwrap_or_else(|err| panic!("{err}"))
    }

    fn equation(bank: &mut TermBank, left: &Term, right: &Term, positive: bool) -> Eqn {
        Eqn::alloc(left.clone(), right.clone(), bank, positive)
            .unwrap_or_else(|err| panic!("{err}"))
    }

    fn clause_from(literals: Vec<Eqn>) -> Clause {
        Clause::alloc(EqnList::from_vec(literals))
    }

    fn entry(array: &FCodeFeatureArray, symbol: FunCode) -> super::FCodeFeatureSortCell {
        *array
            .entry(usize::try_from(symbol).unwrap_or_else(|err| panic!("{err}")))
            .unwrap_or_else(|| panic!("feature entry should exist"))
    }

    #[test]
    fn allocation_collects_symbol_frequencies_and_ranks() {
        let mut bank = term_bank();
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let fa = typed_unary(&mut bank, "f", &a);
        let gb = typed_unary(&mut bank, "g", &b);
        let a_code = a.f_code();
        let b_code = b.f_code();
        let f_code = fa.f_code();
        let g_code = gb.f_code();
        let mut axiom = clause_from(vec![equation(&mut bank, &fa, &b, true)]);
        axiom.set_tptp_type(CP_TYPE_AXIOM);
        let mut conjecture = clause_from(vec![equation(&mut bank, &gb, &fa, false)]);
        conjecture.set_tptp_type(CP_TYPE_CONJECTURE);
        let set = ClauseSet::from_clauses([axiom, conjecture]);

        let array = FCodeFeatureArray::alloc(bank.signature(), &set);

        assert_eq!(
            array.size(),
            usize::try_from(bank.signature().f_count() + 1).unwrap_or_else(|err| panic!("{err}"))
        );
        assert_eq!(entry(&array, a_code).freq, 2);
        assert_eq!(entry(&array, a_code).axiomfreq, 1);
        assert_eq!(entry(&array, a_code).conjfreq, 1);
        assert_eq!(entry(&array, b_code).freq, 2);
        assert_eq!(entry(&array, b_code).axiomfreq, 1);
        assert_eq!(entry(&array, b_code).conjfreq, 1);
        assert_eq!(entry(&array, f_code).freq, 2);
        assert_eq!(entry(&array, g_code).freq, 1);
        assert_eq!(entry(&array, g_code).axiomfreq, 0);
        assert_eq!(entry(&array, g_code).conjfreq, 1);
        assert_eq!(entry(&array, a_code).pos_rank, 4);
        assert_eq!(entry(&array, b_code).pos_rank, 2);
        assert_eq!(entry(&array, f_code).pos_rank, 1);
        assert_eq!(entry(&array, g_code).pos_rank, 3);
    }

    #[test]
    fn occurrence_key_update_matches_c_branch_order() {
        let mut array = FCodeFeatureArray {
            array: vec![
                super::FCodeFeatureSortCell::default(),
                super::FCodeFeatureSortCell {
                    conjfreq: 1,
                    axiomfreq: 0,
                    ..super::FCodeFeatureSortCell::default()
                },
                super::FCodeFeatureSortCell {
                    conjfreq: 1,
                    axiomfreq: 1,
                    ..super::FCodeFeatureSortCell::default()
                },
                super::FCodeFeatureSortCell {
                    conjfreq: 0,
                    axiomfreq: 1,
                    ..super::FCodeFeatureSortCell::default()
                },
                super::FCodeFeatureSortCell::default(),
            ],
        };
        let modifiers = FCodeFeatureKeyModifiers {
            conj_only_mod: 10,
            conj_axiom_mod: 20,
            axiom_only_mod: 30,
            skolem_mod: 40,
            defpred_mod: 50,
        };

        array.update_occ_key(&modifiers);

        assert_eq!(
            array
                .entries()
                .iter()
                .map(|entry| entry.key0)
                .collect::<Vec<_>>(),
            vec![0, 10, 20, 30, 0]
        );
    }

    #[test]
    fn symbol_key_update_uses_signature_properties_by_position() {
        let mut bank = term_bank();
        let skolem = typed_const(&mut bank, "sk");
        let defpred = typed_const(&mut bank, "dp");
        let both = typed_const(&mut bank, "both");
        bank.signature_mut()
            .set_func_prop(skolem.f_code(), FP_SKOLEM_SYMBOL);
        bank.signature_mut()
            .set_func_prop(defpred.f_code(), FP_DEF_PRED);
        bank.signature_mut()
            .set_func_prop(both.f_code(), FP_SKOLEM_SYMBOL | FP_DEF_PRED);
        let set = ClauseSet::new();
        let mut array = FCodeFeatureArray::alloc(bank.signature(), &set);
        let modifiers = FCodeFeatureKeyModifiers {
            skolem_mod: 7,
            defpred_mod: 11,
            ..FCodeFeatureKeyModifiers::default()
        };

        array.update_symb_key(bank.signature(), &modifiers);

        assert_eq!(entry(&array, skolem.f_code()).key0, 7);
        assert_eq!(entry(&array, defpred.f_code()).key0, 11);
        assert_eq!(entry(&array, both.f_code()).key0, 18);
    }

    #[test]
    fn sort_uses_feature_keys_and_keeps_true_position_unsorted() {
        let mut array = FCodeFeatureArray {
            array: vec![
                super::FCodeFeatureSortCell {
                    symbol: 0,
                    key0: 99,
                    ..super::FCodeFeatureSortCell::default()
                },
                super::FCodeFeatureSortCell {
                    symbol: SIG_TRUE_CODE,
                    key0: 99,
                    ..super::FCodeFeatureSortCell::default()
                },
                super::FCodeFeatureSortCell {
                    symbol: 2,
                    key0: 3,
                    pos_rank: 1,
                    ..super::FCodeFeatureSortCell::default()
                },
                super::FCodeFeatureSortCell {
                    symbol: 3,
                    key0: 1,
                    key1: 2,
                    ..super::FCodeFeatureSortCell::default()
                },
                super::FCodeFeatureSortCell {
                    symbol: 4,
                    key0: 1,
                    key1: 1,
                    ..super::FCodeFeatureSortCell::default()
                },
                super::FCodeFeatureSortCell {
                    symbol: 5,
                    key0: 3,
                    pos_rank: 0,
                    ..super::FCodeFeatureSortCell::default()
                },
            ],
        };

        array.sort();

        assert_eq!(array.entries()[0].symbol, 0);
        assert_eq!(array.entries()[1].symbol, SIG_TRUE_CODE);
        assert_eq!(
            array
                .entries()
                .iter()
                .skip(2)
                .map(|entry| entry.symbol)
                .collect::<Vec<_>>(),
            vec![4, 3, 5, 2]
        );
    }
}
