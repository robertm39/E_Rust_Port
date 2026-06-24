use crate::basics::error::Diagnostic;
use crate::basics::pdarrays::{PDArrayIndex, PDIntArray};
use crate::learn::flatannoterms::{flat_anno_set_add_term, FlatAnnoSet, FlatAnnoTerm};
use crate::learn::indexfunctions::{tsm_index_insert, TSMIndex};
use crate::terms::termbanks::TermBank;

pub type TsmPartition = Vec<Option<FlatAnnoTerm>>;

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
        get_tsm_type, tsm_create_subterm_set, tsm_distribution_entropy, tsm_eval_normalize,
        tsm_evaluate_index, tsm_flat_anno_set_entropy, tsm_partition_set, tsm_remainder_entropy,
        TsmPartition, TsmType, TSM_MAX_TERMTOP, TSM_TYPE_NAMES,
    };
    use crate::basics::pdarrays::PDIntArray;
    use crate::inout::scanner::Scanner;
    use crate::learn::flatannoterms::{flat_anno_set_add_term, flat_anno_set_alloc, FlatAnnoTerm};
    use crate::learn::indexfunctions::{tsm_index_alloc, IndexType};
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
