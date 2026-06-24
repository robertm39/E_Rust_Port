use crate::basics::fixdarrays::FixedDArray;
use crate::basics::numtrees::NumTree;
use crate::basics::pstacks::PStack;
use crate::clauses::clause::Clause;
use crate::clauses::clause_props::CP_TYPE_NEG_CONJECTURE;
use crate::clauses::clausesets::ClauseSet;
use crate::terms::functypes::FunCode;
use crate::terms::signature::Signature;
use crate::terms::termbanks::TermBank;
use crate::terms::termfunc::{term_copy_normalize_vars, VarNormStyle};
use crate::terms::termtypes::{Term, TP_PRED_POS, TP_TOP_POS};
use crate::terms::termvars::VarBank;
use std::fmt::Write as _;

pub const TERM_MAX_GENS: usize = 1000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i32)]
pub enum RelatedTermSet {
    ConjectureTerms = 0,
    ConjectureSubterms = 1,
    ConjectureSubtermsTopGens = 2,
    ConjectureSubtermsAllGens = 3,
}

impl RelatedTermSet {
    #[must_use]
    pub const fn from_c_value(value: i32) -> Option<Self> {
        match value {
            0 => Some(Self::ConjectureTerms),
            1 => Some(Self::ConjectureSubterms),
            2 => Some(Self::ConjectureSubtermsTopGens),
            3 => Some(Self::ConjectureSubtermsAllGens),
            _ => None,
        }
    }
}

pub type TermFrequencyTree = NumTree<i64, i64>;

#[must_use]
pub fn compute_subterms_generalizations(term: &Term, vars: &VarBank) -> PStack<Term> {
    let mut all = PStack::new();
    let mut term_vars = NumTree::<Vec<Term>, ()>::new();
    let mut fresh_var_code: FunCode = -2;

    let _gens = compute_subterms_generalizations_inner(
        term,
        vars,
        &mut all,
        &mut term_vars,
        &mut fresh_var_code,
    );

    all
}

#[must_use]
/// # Panics
///
/// Panics if a traversed compound term has an uninitialized argument, if the
/// signature f-count or symbol arity does not fit the Rust target size, or if
/// an occurred signature symbol has a negative arity. These match the C helper
/// assumptions that terms are fully initialized and occurred symbols have
/// ordinary top-cell arities.
pub fn compute_top_generalizations(term: &Term, vars: &VarBank, sig: &Signature) -> PStack<Term> {
    let occurs_len = usize::try_from(sig.f_count() + 1).expect("signature f-count fits in usize");
    let mut occurs = vec![false; occurs_len];
    let mut stack = vec![term.clone()];

    while let Some(subterm) = stack.pop() {
        if subterm.is_free_var() || subterm.is_const() {
            continue;
        }

        if let Ok(index) = usize::try_from(subterm.f_code()) {
            if let Some(slot) = occurs.get_mut(index) {
                *slot = true;
            }
        }

        for index in 1..subterm.arity() {
            let arg = subterm
                .argument(index)
                .unwrap_or_else(|| panic!("term argument {index} is uninitialized"));
            stack.push(arg);
        }
    }

    let mut topgens = PStack::new();
    for code in 1..=sig.f_count() {
        let index = usize::try_from(code).expect("positive f-code fits in usize");
        if !occurs[index] {
            continue;
        }

        let arity = sig
            .find_arity(code)
            .and_then(|arity| usize::try_from(arity).ok())
            .expect("occurred symbol has non-negative arity");
        let topgen = Term::top_alloc(code, arity);
        let var_type = sig.type_bank().i_type();
        for arg_index in 0..arity {
            let offset = FunCode::try_from(arg_index + 1).expect("argument index fits f-code");
            topgen.set_argument(arg_index, vars.var_assert_alloc(-2 * offset, &var_type));
        }
        if sig.is_predicate(code) {
            topgen.set_prop(TP_PRED_POS);
            topgen.set_type(Some(sig.type_bank().bool_type()));
        } else {
            topgen.set_type(Some(sig.type_bank().i_type()));
        }
        topgens.push(topgen);
    }

    topgens
}

pub fn free_generalizations(gens: PStack<Term>) {
    drop(gens);
}

#[must_use]
pub fn tuple_init(cur: &mut FixedDArray) -> bool {
    cur.initialize(0);
    cur.size() > 0
}

/// Advances `cur` to the next C-style tuple under inclusive component maxima.
///
/// # Panics
///
/// Panics if `cur` and `max` have different sizes. The C helper assumes
/// matching fixed-array sizes.
#[must_use]
pub fn tuple_next(cur: &mut FixedDArray, max: &FixedDArray) -> bool {
    assert_eq!(cur.size(), max.size());
    if cur.size() == 0 {
        return false;
    }

    let mut increment_index = None;
    for index in (0..cur.size()).rev() {
        if cur.as_slice()[index] < max.as_slice()[index] {
            increment_index = Some(index);
            break;
        }
    }

    let Some(index) = increment_index else {
        return false;
    };

    cur.as_mut_slice()[index] += 1;
    for value in &mut cur.as_mut_slice()[index + 1..] {
        *value = 0;
    }
    true
}

#[must_use]
pub fn tuple_print_string(tuple: &FixedDArray) -> String {
    let mut result = "(".to_owned();
    for value in tuple.as_slice() {
        let write_result = write!(&mut result, "{value},");
        debug_assert!(write_result.is_ok());
    }
    result.push_str(")\n");
    result
}

/// # Panics
///
/// Panics if a traversed compound term has an uninitialized argument, matching
/// the C helper's valid-term precondition.
pub fn tb_inc_subterms_freqs(term: &Term, freqs: &mut TermFrequencyTree) {
    let mut stack = vec![term.clone()];
    while let Some(subterm) = stack.pop() {
        if subterm.is_free_var() {
            continue;
        }

        let key = subterm.entry_no();
        if let Some(entry) = freqs.find_mut(key) {
            entry.val1 += 1;
        } else {
            freqs.store(key, 1, 1);
        }

        for index in 0..subterm.arity() {
            let arg = subterm
                .argument(index)
                .unwrap_or_else(|| panic!("term argument {index} is uninitialized"));
            stack.push(arg);
        }
    }
}

#[must_use]
pub fn tb_count_term_freqs(bank: &TermBank) -> TermFrequencyTree {
    let mut freqs = TermFrequencyTree::new();
    for term in bank.stored_terms() {
        if term.query_prop(TP_TOP_POS) {
            tb_inc_subterms_freqs(&term, &mut freqs);
        }
    }
    freqs
}

/// Collects normalized terms related to negated conjecture clauses.
///
/// Unlike the term-frequency based C helper, this preserves duplicates and
/// encounter order. That is the shape used by the Levenshtein/tree/structural
/// distance initializers before any strategy-specific deduplication.
///
/// # Panics
///
/// Panics if related subterm traversal or generalization construction hits an
/// uninitialized argument. This matches the C helper preconditions for valid
/// clause terms.
#[must_use]
pub fn collect_related_conjecture_terms(
    axioms: &ClauseSet,
    vars: &VarBank,
    sig: &Signature,
    var_norm: VarNormStyle,
    rel_terms: RelatedTermSet,
) -> Vec<Term> {
    let mut related = Vec::new();
    for clause in axioms.iter() {
        if clause.query_tptp_type() == CP_TYPE_NEG_CONJECTURE {
            collect_clause_related_terms(clause, vars, sig, var_norm, rel_terms, &mut related);
        }
    }
    related
}

fn collect_clause_related_terms(
    clause: &Clause,
    vars: &VarBank,
    sig: &Signature,
    var_norm: VarNormStyle,
    rel_terms: RelatedTermSet,
    related: &mut Vec<Term>,
) {
    for literal in clause.literals().as_slice() {
        collect_term_related_terms(literal.left(), vars, sig, var_norm, rel_terms, related);
        collect_term_related_terms(literal.right(), vars, sig, var_norm, rel_terms, related);
    }
}

fn collect_term_related_terms(
    term: &Term,
    vars: &VarBank,
    sig: &Signature,
    var_norm: VarNormStyle,
    rel_terms: RelatedTermSet,
    related: &mut Vec<Term>,
) {
    match rel_terms {
        RelatedTermSet::ConjectureTerms => push_normalized_term(related, vars, term, var_norm),
        RelatedTermSet::ConjectureSubterms => {
            collect_normalized_subterms(term, vars, var_norm, related);
        }
        RelatedTermSet::ConjectureSubtermsTopGens => {
            collect_normalized_subterms(term, vars, var_norm, related);
            let topgens = compute_top_generalizations(term, vars, sig);
            for topgen in topgens.as_slice() {
                push_normalized_term(related, vars, topgen, var_norm);
            }
            free_generalizations(topgens);
        }
        RelatedTermSet::ConjectureSubtermsAllGens => {
            let subgens = compute_subterms_generalizations(term, vars);
            for subgen in subgens.as_slice() {
                push_normalized_term(related, vars, subgen, var_norm);
            }
            free_generalizations(subgens);
        }
    }
}

fn collect_normalized_subterms(
    term: &Term,
    vars: &VarBank,
    var_norm: VarNormStyle,
    related: &mut Vec<Term>,
) {
    let mut stack = vec![term.clone()];
    while let Some(subterm) = stack.pop() {
        if subterm.is_free_var() {
            continue;
        }
        push_normalized_term(related, vars, &subterm, var_norm);
        stack.extend(subterm.argument_clones().into_iter().flatten());
    }
}

fn push_normalized_term(
    related: &mut Vec<Term>,
    vars: &VarBank,
    term: &Term,
    var_norm: VarNormStyle,
) {
    related.push(term_copy_normalize_vars(vars, term, var_norm));
}

fn compute_subterms_generalizations_inner(
    term: &Term,
    vars: &VarBank,
    all: &mut PStack<Term>,
    term_vars: &mut NumTree<Vec<Term>, ()>,
    fresh_var_code: &mut FunCode,
) -> Vec<Term> {
    let mut gens = get_subterm_generalizing_vars(term, vars, term_vars, fresh_var_code);

    if term.is_any_var() {
        return gens;
    }

    if term.is_const() {
        let copy = term_top_copy_with_all_properties(term);
        gens.push(copy.clone());
        all.push(copy);
        return gens;
    }

    assert!(term.arity() > 0);
    let mut subterm_gens = Vec::with_capacity(term.arity());
    let mut max = FixedDArray::new(term.arity());
    for index in 0..term.arity() {
        let arg = term
            .argument(index)
            .unwrap_or_else(|| panic!("term argument {index} is uninitialized"));
        let child_gens =
            compute_subterms_generalizations_inner(&arg, vars, all, term_vars, fresh_var_code);
        assert!(!child_gens.is_empty());
        max.as_mut_slice()[index] =
            i64::try_from(child_gens.len() - 1).expect("generalization count fits in i64");
        subterm_gens.push(child_gens);
    }

    let mut cur = FixedDArray::new(term.arity());
    let mut iter_counter = 0_usize;
    let mut is_current = tuple_init(&mut cur);
    while is_current {
        if iter_counter > TERM_MAX_GENS {
            break;
        }

        let copy = term_top_copy_with_all_properties(term);
        for (index, child_gens) in subterm_gens.iter().enumerate() {
            let gen_index =
                usize::try_from(cur.as_slice()[index]).expect("tuple component fits in usize");
            copy.set_argument(index, child_gens[gen_index].clone());
        }
        gens.push(copy.clone());
        all.push(copy);

        iter_counter += 1;
        is_current = tuple_next(&mut cur, &max);
    }

    gens
}

fn get_subterm_generalizing_vars(
    term: &Term,
    vars: &VarBank,
    term_vars: &mut NumTree<Vec<Term>, ()>,
    fresh_var_code: &mut FunCode,
) -> Vec<Term> {
    let fresh_var = vars.var_assert_alloc(*fresh_var_code, &vars.default_type());
    *fresh_var_code -= 2;
    let key = term.entry_no();
    if let Some(entry) = term_vars.find_mut(key) {
        entry.val1.push(fresh_var);
        return entry.val1.clone();
    }

    let gen_vars = vec![fresh_var];
    let inserted = term_vars.store(key, gen_vars.clone(), ());
    debug_assert!(inserted);
    gen_vars
}

fn term_top_copy_with_all_properties(term: &Term) -> Term {
    let copy = Term::top_alloc(term.f_code(), term.arity());
    copy.set_properties(term.properties());
    copy.set_type(term.type_());
    copy
}

#[cfg(test)]
mod tests {
    use super::{
        collect_related_conjecture_terms, compute_subterms_generalizations,
        compute_top_generalizations, free_generalizations, tb_count_term_freqs,
        tb_inc_subterms_freqs, tuple_init, tuple_next, tuple_print_string, RelatedTermSet,
        TERM_MAX_GENS,
    };
    use crate::basics::fixdarrays::FixedDArray;
    use crate::clauses::clause::Clause;
    use crate::clauses::clause_props::CP_TYPE_NEG_CONJECTURE;
    use crate::clauses::clausesets::ClauseSet;
    use crate::clauses::eqn::Eqn;
    use crate::clauses::eqnlist::EqnList;
    use crate::inout::scanner::Scanner;
    use crate::terms::signature::Signature;
    use crate::terms::simpletypes::alloc_arrow_type;
    use crate::terms::termbanks::TermBank;
    use crate::terms::termfunc::VarNormStyle;
    use crate::terms::termtypes::{DerefType, Term, TP_CHECK_FLAG, TP_PRED_POS, TP_TOP_POS};
    use crate::terms::termvars::VarBank;
    use crate::terms::typebanks::TypeBank;

    fn array(values: &[i64]) -> FixedDArray {
        let mut array = FixedDArray::new(values.len());
        array.as_mut_slice().copy_from_slice(values);
        array
    }

    fn parse_simple(source: &str) -> (TermBank, Term) {
        let mut bank = TermBank::new(Signature::new(TypeBank::new())).unwrap();
        let mut scanner = Scanner::from_user_string(source, false).unwrap();
        let term = bank.parse_term_simple(&mut scanner).unwrap();
        (bank, term)
    }

    fn parse_in_bank(bank: &mut TermBank, source: &str) -> Term {
        let mut scanner = Scanner::from_user_string(source, false).unwrap();
        bank.parse_term_simple(&mut scanner).unwrap()
    }

    fn term_names(bank: &TermBank, terms: &[Term]) -> Vec<String> {
        terms
            .iter()
            .map(|term| {
                bank.signature()
                    .find_name(term.f_code())
                    .unwrap_or("<var>")
                    .to_owned()
            })
            .collect()
    }

    #[test]
    fn constants_and_related_term_set_discriminants_match_c_header() {
        assert_eq!(TERM_MAX_GENS, 1000);
        assert_eq!(RelatedTermSet::ConjectureTerms as i32, 0);
        assert_eq!(RelatedTermSet::ConjectureSubterms as i32, 1);
        assert_eq!(RelatedTermSet::ConjectureSubtermsTopGens as i32, 2);
        assert_eq!(RelatedTermSet::ConjectureSubtermsAllGens as i32, 3);
        assert_eq!(
            RelatedTermSet::from_c_value(0),
            Some(RelatedTermSet::ConjectureTerms)
        );
        assert_eq!(
            RelatedTermSet::from_c_value(3),
            Some(RelatedTermSet::ConjectureSubtermsAllGens)
        );
        assert_eq!(RelatedTermSet::from_c_value(4), None);
    }

    #[test]
    fn related_conjecture_term_collection_preserves_c_subterm_order() {
        let mut bank = TermBank::new(Signature::new(TypeBank::new())).unwrap();
        let left = parse_in_bank(&mut bank, "f(a,g(b))");
        let right = parse_in_bank(&mut bank, "h(c)");
        let literal = Eqn::alloc(left, right, &mut bank, false).unwrap();
        let mut clause = Clause::alloc(EqnList::from_vec(vec![literal]));
        clause.set_tptp_type(CP_TYPE_NEG_CONJECTURE);
        let ignored = Clause::alloc(EqnList::from_vec(vec![Eqn::alloc(
            parse_in_bank(&mut bank, "ignored"),
            parse_in_bank(&mut bank, "a"),
            &mut bank,
            false,
        )
        .unwrap()]));
        let axioms = ClauseSet::from_clauses([ignored, clause]);
        let vars = VarBank::new(bank.signature().type_bank());

        let top_terms = collect_related_conjecture_terms(
            &axioms,
            &vars,
            bank.signature(),
            VarNormStyle::None,
            RelatedTermSet::ConjectureTerms,
        );
        assert_eq!(term_names(&bank, &top_terms), vec!["f", "h"]);

        let subterms = collect_related_conjecture_terms(
            &axioms,
            &vars,
            bank.signature(),
            VarNormStyle::None,
            RelatedTermSet::ConjectureSubterms,
        );
        assert_eq!(
            term_names(&bank, &subterms),
            vec!["f", "g", "b", "a", "h", "c"]
        );
    }

    #[test]
    fn tuple_helpers_follow_c_lexicographic_order_and_print_shape() {
        let mut cur = array(&[9, 9]);
        let max = array(&[1, 2]);

        assert!(tuple_init(&mut cur));
        let mut seen = vec![cur.as_slice().to_vec()];
        while tuple_next(&mut cur, &max) {
            seen.push(cur.as_slice().to_vec());
        }

        assert_eq!(
            seen,
            vec![
                vec![0, 0],
                vec![0, 1],
                vec![0, 2],
                vec![1, 0],
                vec![1, 1],
                vec![1, 2]
            ]
        );
        assert_eq!(tuple_print_string(&cur), "(1,2,)\n");

        let mut empty = FixedDArray::new(0);
        assert!(!tuple_init(&mut empty));
        assert!(!tuple_next(&mut empty, &FixedDArray::new(0)));
        assert_eq!(tuple_print_string(&empty), "()\n");
    }

    #[test]
    fn subterm_frequency_counter_skips_free_variables_and_counts_repeated_terms() {
        let (_bank, term) = parse_simple("f(a,X,g(a))");
        let a = term.argument(0).unwrap();
        let variable = term.argument(1).unwrap();
        let g = term.argument(2).unwrap();
        let mut freqs = super::TermFrequencyTree::new();

        tb_inc_subterms_freqs(&term, &mut freqs);

        assert_eq!(freqs.find(term.entry_no()).unwrap().val1, 1);
        assert_eq!(freqs.find(a.entry_no()).unwrap().val1, 2);
        assert!(freqs.find(variable.entry_no()).is_none());
        assert_eq!(freqs.find(g.entry_no()).unwrap().val1, 1);
    }

    #[test]
    fn bank_frequency_counter_scans_only_top_position_terms() {
        let mut bank = TermBank::new(Signature::new(TypeBank::new())).unwrap();
        let mut scanner = Scanner::from_user_string("f(a,g(a))", false).unwrap();
        let first = bank.parse_term_simple(&mut scanner).unwrap();
        let mut scanner = Scanner::from_user_string("h(a)", false).unwrap();
        let second = bank.parse_term_simple(&mut scanner).unwrap();
        first.set_prop(TP_TOP_POS);
        second.set_prop(TP_TOP_POS);
        let a = first.argument(0).unwrap();
        let g = first.argument(1).unwrap();
        g.set_prop(TP_CHECK_FLAG);

        let freqs = tb_count_term_freqs(&bank);

        assert_eq!(freqs.find(first.entry_no()).unwrap().val1, 1);
        assert_eq!(freqs.find(second.entry_no()).unwrap().val1, 1);
        assert_eq!(freqs.find(a.entry_no()).unwrap().val1, 3);
        assert_eq!(freqs.find(g.entry_no()).unwrap().val1, 1);
    }

    #[test]
    fn top_generalizations_preserve_argument_zero_skip_quirk() {
        let (bank, term) = parse_simple("f(g(a),h(b))");
        let gens = compute_top_generalizations(&term, bank.vars(), bank.signature());
        let f_code = bank.signature().find_f_code("f");
        let h_code = bank.signature().find_f_code("h");
        let g_code = bank.signature().find_f_code("g");
        let seen = gens.as_slice().iter().map(Term::f_code).collect::<Vec<_>>();

        assert!(seen.contains(&f_code));
        assert!(seen.contains(&h_code));
        assert!(!seen.contains(&g_code));
        assert!(gens.as_slice().iter().all(|gen| gen
            .argument_clones()
            .into_iter()
            .flatten()
            .all(|arg| arg.is_free_var())));
    }

    #[test]
    fn top_generalizations_assign_predicate_type_and_property() {
        let mut sig = Signature::new(TypeBank::new());
        let individual = sig.type_bank().i_type();
        let bool_type = sig.type_bank().bool_type();
        let pred = sig.insert_id("p", 1, false);
        sig.declare_final_type(pred, alloc_arrow_type(vec![individual, bool_type.clone()]))
            .unwrap();
        let mut bank = TermBank::new(sig).unwrap();
        let arg_code = bank.signature_mut().insert_id("a", 0, false);
        let individual = bank.signature().type_bank().i_type();
        bank.signature_mut()
            .declare_type(arg_code, individual)
            .unwrap();
        let arg = bank.create_const_term(arg_code).unwrap();
        let term = Term::top_alloc(pred, 1);
        term.set_argument(0, arg);
        let term = bank.insert(&term, DerefType::Never).unwrap();

        let gens = compute_top_generalizations(&term, bank.vars(), bank.signature());
        let gen = gens
            .as_slice()
            .iter()
            .find(|gen| gen.f_code() == pred)
            .unwrap();

        assert!(gen.query_prop(TP_PRED_POS));
        assert_eq!(gen.type_(), Some(bool_type));
    }

    #[test]
    fn subterm_generalizations_allocate_variables_per_repeated_entry_visit() {
        let (bank, term) = parse_simple("f(a,a)");
        let gens = compute_subterms_generalizations(&term, bank.vars());
        let f_code = bank.signature().find_f_code("f");
        let a_code = bank.signature().find_f_code("a");
        let constants = gens
            .as_slice()
            .iter()
            .filter(|gen| gen.f_code() == a_code)
            .count();
        let f_gens = gens
            .as_slice()
            .iter()
            .filter(|gen| gen.f_code() == f_code)
            .collect::<Vec<_>>();

        assert_eq!(constants, 2);
        assert_eq!(f_gens.len(), 6);
        assert!(f_gens.iter().any(|gen| {
            gen.argument(0).unwrap().is_free_var() && gen.argument(1).unwrap().f_code() == a_code
        }));
        free_generalizations(gens);
    }
}
