use crate::learn::flatannoterms::{flat_anno_set_add_term, FlatAnnoSet, FlatAnnoTerm};

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
        get_tsm_type, tsm_create_subterm_set, tsm_eval_normalize, tsm_flat_anno_set_entropy,
        tsm_remainder_entropy, TsmType, TSM_MAX_TERMTOP, TSM_TYPE_NAMES,
    };
    use crate::inout::scanner::Scanner;
    use crate::learn::flatannoterms::{flat_anno_set_add_term, flat_anno_set_alloc, FlatAnnoTerm};
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
