use crate::basics::error::Diagnostic;
use crate::basics::pdarrays::{PDArrayIndex, PDIntArray};
use crate::basics::simple_stuff::ProblemType;
use crate::clauses::clause::{
    clause_pcl_string, clause_print_lop_format_string, clause_print_lop_format_string_with_options,
    clause_print_tptp_format_string_with_options, clause_write_tstp_with_type_suffixes, Clause,
};
use crate::clauses::eqn::{Eqn, EqnPrintOptions};
use crate::clauses::eqnlist::EqnList;
use crate::heuristics::varweights::clause_count_ext_symbols as varweight_clause_count_ext_symbols;
use crate::inout::scanner::IoFormat;
use crate::terms::functypes::FunCode;
use crate::terms::signature::Signature;
use crate::terms::termbanks::TermBank;
use crate::terms::termfunc::term_depth;
use crate::terms::termtypes::Term;

const DEFAULT_COMCHAR_RAW: &str = "%";

#[must_use]
pub fn clause_count_ext_symbols(clause: &Clause, signature: &Signature, min_arity: i64) -> i64 {
    varweight_clause_count_ext_symbols(clause, signature, min_arity)
}

/// Adds free-variable occurrences by negated variable f-code.
///
/// # Panics
///
/// Panics if a traversed non-variable term has an uninitialized argument slot,
/// if a non-variable term has a non-positive f-code, or if a variable f-code
/// cannot be converted to the dynamic-array index shape used by the C helper.
pub fn term_add_var_distribution(term: &Term, dist_array: &mut PDIntArray) -> FunCode {
    let mut max_var = 0;
    let mut stack = vec![term.clone()];

    while let Some(current) = stack.pop() {
        if current.is_free_var() {
            let var = positive_variable_code(current.f_code());
            let index = pd_index_from_positive(var);
            let count = dist_array.element_int(index) + 1;
            max_var = max_var.max(var);
            assert!(
                dist_array.assign(index, count),
                "variable distribution array must cover variable codes"
            );
        } else {
            assert!(
                current.f_code() > 0,
                "non-free terms in variable distribution require positive f-codes"
            );
            stack.extend(current.argument_clones().into_iter().map(|arg| {
                arg.unwrap_or_else(|| {
                    panic!("variable distribution requires initialized term arguments")
                })
            }));
        }
    }

    max_var
}

/// Adds variable occurrences for both literal sides.
///
/// # Panics
///
/// Panics under the same conditions as [`term_add_var_distribution`].
pub fn eqn_add_var_distribution(eqn: &Eqn, dist_array: &mut PDIntArray) -> FunCode {
    term_add_var_distribution(eqn.left(), dist_array)
        .max(term_add_var_distribution(eqn.right(), dist_array))
}

/// Adds variable occurrences for every literal in the list.
///
/// # Panics
///
/// Panics under the same conditions as [`term_add_var_distribution`].
pub fn eqn_list_add_var_distribution(list: &EqnList, dist_array: &mut PDIntArray) -> FunCode {
    list.as_slice()
        .iter()
        .map(|literal| eqn_add_var_distribution(literal, dist_array))
        .max()
        .unwrap_or(0)
}

/// Adds variable occurrences for every literal in the clause.
///
/// # Panics
///
/// Panics under the same conditions as [`term_add_var_distribution`].
pub fn clause_add_var_distribution(clause: &Clause, dist_array: &mut PDIntArray) -> FunCode {
    eqn_list_add_var_distribution(clause.literals(), dist_array)
}

/// Counts distinct variable f-codes in a clause.
///
/// # Panics
///
/// Panics under the same conditions as [`term_add_var_distribution`].
#[must_use]
pub fn clause_count_variable_set(clause: &Clause) -> i64 {
    let mut dist_array = PDIntArray::new_int(20, 20);
    let max_var = clause_add_var_distribution(clause, &mut dist_array);
    count_var_indices_with(&mut dist_array, max_var, |count| count != 0)
}

/// Counts variable f-codes that occur exactly once in a clause.
///
/// # Panics
///
/// Panics under the same conditions as [`term_add_var_distribution`].
#[must_use]
pub fn clause_count_singleton_set(clause: &Clause) -> i64 {
    let mut dist_array = PDIntArray::new_int(20, 20);
    let max_var = clause_add_var_distribution(clause, &mut dist_array);
    count_var_indices_with(&mut dist_array, max_var, |count| count == 1)
}

#[must_use]
pub fn clause_count_maximal_terms(clause: &Clause) -> i64 {
    clause
        .literals()
        .as_slice()
        .iter()
        .filter(|literal| literal.is_maximal())
        .map(Eqn::count_maximal_literals)
        .sum()
}

#[must_use]
pub fn clause_count_maximal_literals(clause: &Clause) -> i64 {
    clause
        .literals()
        .as_slice()
        .iter()
        .filter(|literal| literal.is_maximal())
        .count()
        .try_into()
        .unwrap_or(i64::MAX)
}

#[must_use]
pub fn clause_count_unorientable_literals(clause: &Clause) -> i64 {
    clause
        .literals()
        .as_slice()
        .iter()
        .filter(|literal| !literal.is_oriented())
        .count()
        .try_into()
        .unwrap_or(i64::MAX)
}

/// Adds TPTP-style term-depth statistics to the provided accumulators.
///
/// Equational literals contribute both sides. Predicate literals contribute
/// only the arguments of the predicate atom, matching the C interpretation of
/// conventional TPTP literals.
///
/// # Panics
///
/// Panics if a predicate literal has an uninitialized atom argument slot.
pub fn clause_tptp_depth_info_add(
    bank: &TermBank,
    clause: &Clause,
    depthmax: &mut i64,
    depthsum: &mut i64,
    count: &mut i64,
) -> i64 {
    for literal in clause.literals().as_slice() {
        eqn_tptp_depth_info_add(bank, literal, depthmax, depthsum, count);
    }
    *depthmax
}

#[must_use]
pub fn clause_info_string(bank: &TermBank, clause: &Clause) -> String {
    let symbol_count =
        c_long_from_clause_weight(clause.literal_weight(bank, 1.0, 1.0, 1.0, 1, 1, 1.0, false));
    let variable_occurrences =
        c_long_from_clause_weight(clause.literal_weight(bank, 0.0, 1.0, 1.0, 1, 1, 1.0, false));
    format!(
        "info({}, {}, {}, {}, {}, {}, {}, {})",
        clause.ident(),
        clause.proof_depth(),
        clause.proof_size(),
        symbol_count,
        clause.depth(),
        clause.literal_number(),
        variable_occurrences,
        clause_count_variable_set(clause)
    )
}

#[must_use]
pub fn clause_line_string(
    bank: &TermBank,
    clause_text: &str,
    clause: &Clause,
    print_info: bool,
) -> String {
    clause_line_string_with_comment(bank, clause_text, clause, print_info, DEFAULT_COMCHAR_RAW)
}

#[must_use]
pub fn clause_line_print_string(bank: &TermBank, clause: &Clause, print_info: bool) -> String {
    let clause_text = clause_print_lop_format_string(bank, clause, true);
    clause_line_string(bank, &clause_text, clause, print_info)
}

/// Returns the C `ClauseLinePrint` shape with explicit `ClausePrint` dispatch.
///
/// # Errors
///
/// Returns a diagnostic if TSTP rendering rejects the clause shape.
pub fn clause_line_print_format_string(
    bank: &TermBank,
    clause: &Clause,
    print_info: bool,
    output_format: IoFormat,
    problem_type: ProblemType,
) -> Result<String, Diagnostic> {
    let options = match output_format {
        IoFormat::Tptp => EqnPrintOptions::tptp(),
        IoFormat::Lop | IoFormat::Tstp | IoFormat::Auto => EqnPrintOptions::lop(),
    };
    clause_line_print_format_string_with_options(
        bank,
        clause,
        print_info,
        output_format,
        problem_type,
        options,
    )
}

/// Returns the C `ClauseLinePrint` shape with caller-provided equation options.
///
/// # Errors
///
/// Returns a diagnostic if TSTP rendering rejects the clause shape.
pub fn clause_line_print_format_string_with_options(
    bank: &TermBank,
    clause: &Clause,
    print_info: bool,
    output_format: IoFormat,
    problem_type: ProblemType,
    eqn_print_options: EqnPrintOptions,
) -> Result<String, Diagnostic> {
    let clause_text = clause_print_for_output_format(
        bank,
        clause,
        output_format,
        problem_type,
        eqn_print_options,
    )?;
    Ok(clause_line_string(bank, &clause_text, clause, print_info))
}

#[must_use]
pub fn clause_line_string_with_comment(
    bank: &TermBank,
    clause_text: &str,
    clause: &Clause,
    print_info: bool,
    comment: &str,
) -> String {
    let mut result = String::from(clause_text);
    if print_info {
        result.push(' ');
        result.push_str(comment);
        result.push(' ');
        result.push_str(&clause_info_string(bank, clause));
    }
    result.push('\n');
    result
}

#[must_use]
pub fn clause_prop_info_stats_string(clause: &Clause) -> String {
    clause_prop_info_stats_string_with_comment(DEFAULT_COMCHAR_RAW, clause)
}

#[must_use]
pub fn clause_prop_info_string(pcl_text: &str, clause: &Clause) -> String {
    clause_prop_info_string_with_comment(pcl_text, clause, DEFAULT_COMCHAR_RAW)
}

#[must_use]
pub fn clause_prop_info_print_string(bank: &TermBank, clause: &Clause) -> String {
    let pcl_text = clause_pcl_string(bank, clause, true);
    clause_prop_info_string(&pcl_text, clause)
}

#[must_use]
pub fn clause_prop_info_string_with_comment(
    pcl_text: &str,
    clause: &Clause,
    comment: &str,
) -> String {
    let mut result = String::new();
    result.push_str(comment);
    result.push(' ');
    result.push_str(pcl_text);
    result.push_str(&clause_prop_info_stats_string_with_comment(comment, clause));
    result
}

#[must_use]
pub fn clause_prop_info_stats_string_with_comment(comment: &str, clause: &Clause) -> String {
    let standard_weight = clause.standard_weight();
    let symbol_count =
        c_long_from_clause_weight(clause.sym_type_weight(1.0, 1.0, 1.0, 1, 1, 1, 1, 1.0));
    let function_symbols =
        c_long_from_clause_weight(clause.sym_type_weight(1.0, 1.0, 1.0, 0, 1, 0, 0, 1.0));
    let variables =
        c_long_from_clause_weight(clause.sym_type_weight(1.0, 1.0, 1.0, 1, 0, 0, 0, 1.0));
    let constants =
        c_long_from_clause_weight(clause.sym_type_weight(1.0, 1.0, 1.0, 0, 0, 1, 0, 1.0));
    let predicate_symbols =
        c_long_from_clause_weight(clause.sym_type_weight(1.0, 1.0, 1.0, 0, 0, 0, 1, 1.0));
    let depth = clause.depth();
    let literals = clause.literal_number();
    let positive = clause.positive_literal_count();
    let negative = clause.negative_literal_count();

    format!(
        concat!(
            "\n{comment} Standardweight: {standard_weight:6}\n",
            "{comment} Symbol count  : {symbol_count:6}\n",
            "{comment}    F. symbols : {function_symbols:6}\n",
            "{comment}    Variables  : {variables:6}\n",
            "{comment}    Constants  : {constants:6}\n",
            "{comment}    P. symbols : {predicate_symbols:6}\n",
            "{comment} Depth         : {depth:6}\n",
            "{comment} Literals      : {literals:6}\n",
            "{comment}    ...positive: {positive:6}\n",
            "{comment}    ...negative: {negative:6}\n",
        ),
        comment = comment,
        standard_weight = standard_weight,
        symbol_count = symbol_count,
        function_symbols = function_symbols,
        variables = variables,
        constants = constants,
        predicate_symbols = predicate_symbols,
        depth = depth,
        literals = literals,
        positive = positive,
        negative = negative,
    )
}

fn clause_print_for_output_format(
    bank: &TermBank,
    clause: &Clause,
    output_format: IoFormat,
    problem_type: ProblemType,
    eqn_print_options: EqnPrintOptions,
) -> Result<String, Diagnostic> {
    match output_format {
        IoFormat::Tptp => Ok(clause_print_tptp_format_string_with_options(
            bank,
            clause,
            eqn_print_options,
        )),
        IoFormat::Tstp => {
            let mut output = String::new();
            clause_write_tstp_with_type_suffixes(
                &mut output,
                bank,
                clause,
                true,
                true,
                problem_type,
                eqn_print_options.print_types,
            )?;
            Ok(output)
        }
        IoFormat::Lop | IoFormat::Auto => Ok(clause_print_lop_format_string_with_options(
            bank,
            clause,
            true,
            eqn_print_options,
        )),
    }
}

fn eqn_tptp_depth_info_add(
    bank: &TermBank,
    eqn: &Eqn,
    depthmax: &mut i64,
    depthsum: &mut i64,
    count: &mut i64,
) -> i64 {
    if eqn.is_equ_lit(bank) {
        term_depth_info_add(eqn.left(), depthmax, depthsum, count);
        term_depth_info_add(eqn.right(), depthmax, depthsum, count);
    } else {
        for index in 0..eqn.left().arity() {
            let arg = eqn.left().argument(index).unwrap_or_else(|| {
                panic!("TPTP depth collection requires initialized predicate arguments")
            });
            term_depth_info_add(&arg, depthmax, depthsum, count);
        }
    }
    *depthmax
}

fn term_depth_info_add(
    term: &Term,
    depthmax: &mut i64,
    depthsum: &mut i64,
    count: &mut i64,
) -> i64 {
    let depth = term_depth(term);
    *depthsum += depth;
    *count += 1;
    if depth > *depthmax {
        *depthmax = depth;
    }
    *depthmax
}

fn count_var_indices_with<F>(dist_array: &mut PDIntArray, max_var: FunCode, predicate: F) -> i64
where
    F: Fn(i64) -> bool,
{
    (1..=max_var)
        .filter(|var| predicate(dist_array.element_int(pd_index_from_positive(*var))))
        .count()
        .try_into()
        .unwrap_or(i64::MAX)
}

fn positive_variable_code(f_code: FunCode) -> FunCode {
    assert!(f_code < 0, "variable f-code must be negative");
    f_code
        .checked_neg()
        .unwrap_or_else(|| panic!("variable f-code cannot be negated"))
}

fn pd_index_from_positive(value: FunCode) -> PDArrayIndex {
    PDArrayIndex::try_from(value)
        .unwrap_or_else(|_| panic!("positive variable code must fit the dynamic-array index type"))
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "C ClauseInfoPrint casts ClauseWeight's double result to long"
)]
fn c_long_from_clause_weight(weight: f64) -> i64 {
    weight as i64
}

#[cfg(test)]
mod tests {
    use super::{
        clause_add_var_distribution, clause_count_ext_symbols, clause_count_maximal_literals,
        clause_count_maximal_terms, clause_count_singleton_set, clause_count_unorientable_literals,
        clause_count_variable_set, clause_info_string, clause_line_print_format_string,
        clause_line_print_string, clause_line_string, clause_line_string_with_comment,
        clause_prop_info_print_string, clause_prop_info_stats_string,
        clause_prop_info_stats_string_with_comment, clause_prop_info_string,
        clause_prop_info_string_with_comment, clause_tptp_depth_info_add, eqn_add_var_distribution,
        eqn_list_add_var_distribution, term_add_var_distribution,
    };
    use crate::basics::pdarrays::PDIntArray;
    use crate::basics::simple_stuff::ProblemType;
    use crate::clauses::clause::Clause;
    use crate::clauses::eqn::Eqn;
    use crate::clauses::eqn_props::{EP_IS_MAXIMAL, EP_IS_ORIENTED};
    use crate::clauses::eqnlist::EqnList;
    use crate::inout::scanner::IoFormat;
    use crate::terms::functypes::FunCode;
    use crate::terms::signature::Signature;
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
        let term = Term::const_cell_alloc(f_code);
        term.set_type(Some(type_));
        bank.insert(&term, DerefType::Never)
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

    fn typed_binary(bank: &mut TermBank, name: &str, left: &Term, right: &Term) -> Term {
        let type_ = individual(bank);
        let f_code = bank.signature_mut().insert_id(name, 2, false);
        bank.signature_mut()
            .declare_final_type(
                f_code,
                alloc_arrow_type(vec![type_.clone(), type_.clone(), type_.clone()]),
            )
            .unwrap_or_else(|err| panic!("{err}"));
        let term = Term::top_alloc(f_code, 2);
        term.set_type(Some(type_));
        term.set_argument(0, left.clone());
        term.set_argument(1, right.clone());
        bank.insert(&term, DerefType::Never)
            .unwrap_or_else(|err| panic!("{err}"))
    }

    fn typed_predicate_binary(bank: &mut TermBank, name: &str, left: &Term, right: &Term) -> Term {
        let type_ = individual(bank);
        let bool_type = bank.signature().type_bank().bool_type();
        let f_code = bank.signature_mut().insert_id(name, 2, false);
        bank.signature_mut()
            .declare_final_type(
                f_code,
                alloc_arrow_type(vec![type_.clone(), type_, bool_type.clone()]),
            )
            .unwrap_or_else(|err| panic!("{err}"));
        let term = Term::top_alloc(f_code, 2);
        term.set_type(Some(bool_type));
        term.set_argument(0, left.clone());
        term.set_argument(1, right.clone());
        let shared = bank
            .insert(&term, DerefType::Never)
            .unwrap_or_else(|err| panic!("{err}"));
        shared.set_type(Some(bank.signature().type_bank().bool_type()));
        shared
    }

    fn typed_var(bank: &TermBank, f_code: FunCode) -> Term {
        bank.vars().var_assert_alloc(f_code, &individual(bank))
    }

    fn equation(bank: &mut TermBank, left: &Term, right: &Term, positive: bool) -> Eqn {
        Eqn::alloc(left.clone(), right.clone(), bank, positive)
            .unwrap_or_else(|err| panic!("{err}"))
    }

    fn predicate_literal(bank: &mut TermBank, atom: &Term) -> Eqn {
        let mut literal = Eqn::create_true_lit(bank).unwrap_or_else(|err| panic!("{err}"));
        literal.set_left_raw(atom.clone());
        literal
    }

    fn clause_from(literals: Vec<Eqn>) -> Clause {
        Clause::alloc(EqnList::from_vec(literals))
    }

    #[test]
    fn variable_distribution_counts_by_negated_f_code() {
        let mut bank = term_bank();
        let x1 = typed_var(&bank, -2);
        let x2 = typed_var(&bank, -4);
        let x1_again = typed_var(&bank, -2);
        let fx = typed_unary(&mut bank, "f", &x1);
        let gx = typed_binary(&mut bank, "g", &x1_again, &x2);
        let mut dist = PDIntArray::new_int(2, 2);

        assert_eq!(term_add_var_distribution(&gx, &mut dist), 4);
        assert_eq!(dist.element_int(2), 1);
        assert_eq!(dist.element_int(4), 1);

        let eqn = equation(&mut bank, &fx, &gx, false);
        assert_eq!(eqn_add_var_distribution(&eqn, &mut dist), 4);
        assert_eq!(dist.element_int(2), 3);
        assert_eq!(dist.element_int(4), 2);

        let list = EqnList::from_vec(vec![eqn.clone()]);
        assert_eq!(eqn_list_add_var_distribution(&list, &mut dist), 4);
        assert_eq!(dist.element_int(2), 5);
        assert_eq!(dist.element_int(4), 3);

        let clause = clause_from(vec![eqn]);
        assert_eq!(clause_add_var_distribution(&clause, &mut dist), 4);
        assert_eq!(dist.element_int(2), 7);
        assert_eq!(dist.element_int(4), 4);
    }

    #[test]
    fn variable_set_and_singleton_counts_use_variable_codes_not_identity() {
        let mut bank = term_bank();
        let x1 = typed_var(&bank, -2);
        let x1_same_code = typed_var(&bank, -2);
        let x2 = typed_var(&bank, -4);
        let a = typed_const(&mut bank, "a");
        let left = typed_binary(&mut bank, "f", &x1, &x1_same_code);
        let right = typed_binary(&mut bank, "g", &x2, &a);
        let clause = clause_from(vec![equation(&mut bank, &left, &right, false)]);

        assert_eq!(clause_count_variable_set(&clause), 2);
        assert_eq!(clause_count_singleton_set(&clause), 1);
    }

    #[test]
    fn maximal_and_unorientable_counts_follow_literal_flags() {
        let mut bank = term_bank();
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let c = typed_const(&mut bank, "c");
        let mut oriented_max = equation(&mut bank, &a, &b, true);
        oriented_max.set_prop(EP_IS_MAXIMAL | EP_IS_ORIENTED);
        let mut unoriented_max = equation(&mut bank, &b, &c, false);
        unoriented_max.set_prop(EP_IS_MAXIMAL);
        let ordinary = equation(&mut bank, &a, &c, false);
        let clause = clause_from(vec![oriented_max, unoriented_max, ordinary]);

        assert_eq!(clause_count_maximal_literals(&clause), 2);
        assert_eq!(clause_count_maximal_terms(&clause), 3);
        assert_eq!(clause_count_unorientable_literals(&clause), 2);
    }

    #[test]
    fn tptp_depth_info_counts_equation_sides_and_predicate_arguments() {
        let mut bank = term_bank();
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let fa = typed_unary(&mut bank, "f", &a);
        let gb = typed_unary(&mut bank, "g", &b);
        let h = typed_binary(&mut bank, "h", &fa, &gb);
        let p = typed_predicate_binary(&mut bank, "p", &h, &a);
        let eqn = equation(&mut bank, &h, &gb, true);
        let pred = predicate_literal(&mut bank, &p);
        let clause = clause_from(vec![eqn, pred]);

        let mut depthmax = 0;
        let mut depthsum = 0;
        let mut count = 0;
        assert_eq!(
            clause_tptp_depth_info_add(&bank, &clause, &mut depthmax, &mut depthsum, &mut count,),
            3
        );
        assert_eq!(depthmax, 3);
        assert_eq!(depthsum, 3 + 2 + 3 + 1);
        assert_eq!(count, 4);
    }

    #[test]
    fn clause_info_string_matches_c_field_order() {
        let mut bank = term_bank();
        let x = typed_var(&bank, -2);
        let a = typed_const(&mut bank, "a");
        let fx = typed_unary(&mut bank, "f", &x);
        let mut clause = clause_from(vec![equation(&mut bank, &fx, &a, true)]);
        clause.set_ident(42);
        clause.set_proof_depth(3);
        clause.set_proof_size(5);

        assert_eq!(
            clause_info_string(&bank, &clause),
            "info(42, 3, 5, 4, 2, 1, 1, 1)"
        );
    }

    #[test]
    fn clause_info_string_preserves_c_d6_weight_semantics() {
        let mut bank = term_bank();
        let a = typed_const(&mut bank, "d6_a");
        let b = typed_const(&mut bank, "d6_b");
        let mut clause = clause_from(vec![equation(&mut bank, &a, &b, true)]);
        clause.set_ident(7);

        assert_eq!(
            clause_info_string(&bank, &clause),
            "info(7, 0, 0, 3, 1, 1, 1, 0)"
        );
    }

    #[test]
    fn clause_line_string_appends_optional_c_info_segment_and_newline() {
        let mut bank = term_bank();
        let a = typed_const(&mut bank, "line_a");
        let b = typed_const(&mut bank, "line_b");
        let mut clause = clause_from(vec![equation(&mut bank, &a, &b, true)]);
        clause.set_ident(9);

        assert_eq!(
            clause_line_string(&bank, "cnf(c_9,axiom,(line_a=line_b)).", &clause, false),
            "cnf(c_9,axiom,(line_a=line_b)).\n"
        );
        assert_eq!(
            clause_line_string(&bank, "cnf(c_9,axiom,(line_a=line_b)).", &clause, true),
            "cnf(c_9,axiom,(line_a=line_b)). % info(9, 0, 0, 3, 1, 1, 1, 0)\n"
        );
        assert_eq!(
            clause_line_string_with_comment(
                &bank,
                "cnf(c_9,axiom,(line_a=line_b)).",
                &clause,
                true,
                "#",
            ),
            "cnf(c_9,axiom,(line_a=line_b)). # info(9, 0, 0, 3, 1, 1, 1, 0)\n"
        );
    }

    #[test]
    fn clause_line_print_string_uses_default_lop_clause_rendering() {
        let mut bank = term_bank();
        let a = typed_const(&mut bank, "line_print_a");
        let b = typed_const(&mut bank, "line_print_b");
        let mut clause = clause_from(vec![
            equation(&mut bank, &a, &b, true),
            equation(&mut bank, &b, &a, false),
        ]);
        clause.set_ident(12);

        assert_eq!(
            clause_line_print_string(&bank, &clause, false),
            "line_print_a=line_print_b <- line_print_b=line_print_a.\n"
        );
        assert_eq!(
            clause_line_print_string(&bank, &clause, true),
            "line_print_a=line_print_b <- line_print_b=line_print_a. % info(12, 0, 0, 6, 1, 2, 2, 0)\n"
        );
    }

    #[test]
    fn clause_line_print_format_string_dispatches_like_clause_print() {
        let mut bank = term_bank();
        let a = typed_const(&mut bank, "line_format_a");
        let b = typed_const(&mut bank, "line_format_b");
        let mut clause = clause_from(vec![
            equation(&mut bank, &a, &b, true),
            equation(&mut bank, &b, &a, false),
        ]);
        clause.set_ident(14);

        let input_clause_line = clause_line_print_format_string(
            &bank,
            &clause,
            false,
            IoFormat::Tptp,
            ProblemType::FirstOrder,
        )
        .unwrap_or_else(|err| panic!("{err}"));
        assert!(input_clause_line.starts_with("input_clause("));
        assert!(input_clause_line.contains("c_0_14"));
        assert!(input_clause_line.contains("++equal(line_format_a, line_format_b)"));
        assert!(input_clause_line.ends_with("]).\n"));
        assert!(!input_clause_line.contains("<-"));

        let wrapped_clause_line = clause_line_print_format_string(
            &bank,
            &clause,
            true,
            IoFormat::Tstp,
            ProblemType::FirstOrder,
        )
        .unwrap_or_else(|err| panic!("{err}"));
        assert!(wrapped_clause_line.starts_with("cnf(") || wrapped_clause_line.starts_with("tcf("));
        assert!(wrapped_clause_line.contains("c_0_14"));
        assert!(wrapped_clause_line.contains("line_format_a"));
        assert!(wrapped_clause_line.contains(" % info(14, 0, 0, 6, 1, 2, 2, 0)\n"));
        assert!(!wrapped_clause_line.contains("<-"));

        assert_eq!(
            clause_line_print_format_string(
                &bank,
                &clause,
                false,
                IoFormat::Auto,
                ProblemType::FirstOrder,
            )
            .unwrap_or_else(|err| panic!("{err}")),
            clause_line_print_string(&bank, &clause, false)
        );
    }

    #[test]
    fn clause_prop_info_stats_string_matches_c_stat_block_format() {
        let mut bank = term_bank();
        let x = typed_var(&bank, -2);
        let a = typed_const(&mut bank, "prop_a");
        let fx = typed_unary(&mut bank, "prop_f", &x);
        let clause = clause_from(vec![equation(&mut bank, &fx, &a, true)]);

        assert_eq!(
            clause_prop_info_stats_string(&clause),
            concat!(
                "\n% Standardweight:      5\n",
                "% Symbol count  :      3\n",
                "%    F. symbols :      1\n",
                "%    Variables  :      1\n",
                "%    Constants  :      1\n",
                "%    P. symbols :      0\n",
                "% Depth         :      2\n",
                "% Literals      :      1\n",
                "%    ...positive:      1\n",
                "%    ...negative:      0\n",
            )
        );
        assert_eq!(
            clause_prop_info_stats_string_with_comment("#", &clause),
            concat!(
                "\n# Standardweight:      5\n",
                "# Symbol count  :      3\n",
                "#    F. symbols :      1\n",
                "#    Variables  :      1\n",
                "#    Constants  :      1\n",
                "#    P. symbols :      0\n",
                "# Depth         :      2\n",
                "# Literals      :      1\n",
                "#    ...positive:      1\n",
                "#    ...negative:      0\n",
            )
        );
    }

    #[test]
    fn clause_prop_info_string_prefixes_pcl_text_and_appends_stats() {
        let mut bank = term_bank();
        let x = typed_var(&bank, -2);
        let a = typed_const(&mut bank, "prop_line_a");
        let fx = typed_unary(&mut bank, "prop_line_f", &x);
        let clause = clause_from(vec![equation(&mut bank, &fx, &a, true)]);

        assert_eq!(
            clause_prop_info_string("pcl(c_1,...)", &clause),
            concat!(
                "% pcl(c_1,...)",
                "\n% Standardweight:      5\n",
                "% Symbol count  :      3\n",
                "%    F. symbols :      1\n",
                "%    Variables  :      1\n",
                "%    Constants  :      1\n",
                "%    P. symbols :      0\n",
                "% Depth         :      2\n",
                "% Literals      :      1\n",
                "%    ...positive:      1\n",
                "%    ...negative:      0\n",
            )
        );
        assert_eq!(
            clause_prop_info_string_with_comment("pcl(c_1,...)", &clause, "#"),
            concat!(
                "# pcl(c_1,...)",
                "\n# Standardweight:      5\n",
                "# Symbol count  :      3\n",
                "#    F. symbols :      1\n",
                "#    Variables  :      1\n",
                "#    Constants  :      1\n",
                "#    P. symbols :      0\n",
                "# Depth         :      2\n",
                "# Literals      :      1\n",
                "#    ...positive:      1\n",
                "#    ...negative:      0\n",
            )
        );
    }

    #[test]
    fn clause_prop_info_print_string_uses_default_pcl_clause_rendering() {
        let mut bank = term_bank();
        let a = typed_const(&mut bank, "prop_print_a");
        let b = typed_const(&mut bank, "prop_print_b");
        let clause = clause_from(vec![equation(&mut bank, &a, &b, true)]);

        let rendered = clause_prop_info_print_string(&bank, &clause);

        assert!(rendered.starts_with("% [++equal(prop_print_a, prop_print_b)]\n"));
        assert!(rendered.contains("% Standardweight:"));
        assert!(rendered.contains("% Literals      :      1\n"));
    }

    #[test]
    fn external_symbol_count_reuses_clause_feature_contract() {
        let mut bank = term_bank();
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let fa = typed_unary(&mut bank, "f", &a);
        let gb = typed_unary(&mut bank, "g", &b);
        let clause = clause_from(vec![equation(&mut bank, &fa, &gb, true)]);

        assert_eq!(clause_count_ext_symbols(&clause, bank.signature(), 0), 4);
        assert_eq!(clause_count_ext_symbols(&clause, bank.signature(), 1), 2);
        assert_eq!(clause_count_ext_symbols(&clause, bank.signature(), 2), 0);
    }
}
