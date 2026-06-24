use crate::basics::error::{Diagnostic, ErrorCode};
use crate::basics::pstacks::PStack;
use crate::basics::simple_stuff::ProblemType;
use crate::clauses::clause::{clause_write_tstp, Clause};
use crate::terms::termbanks::TermBank;
use std::fmt;

/// Writes the C `PStackClausePrintTSTP` shape.
///
/// # Errors
///
/// Returns a diagnostic if a clause needs an unported `ClauseTSTPPrint` branch,
/// or if the output writer reports a formatting error.
///
/// # Panics
///
/// Panics if a printed clause violates the C clause/literal/term printing
/// preconditions.
pub fn pstack_clause_write_tstp(
    output: &mut impl fmt::Write,
    bank: &TermBank,
    stack: &PStack<&Clause>,
    problem_type: ProblemType,
) -> Result<(), Diagnostic> {
    for clause in stack.as_slice() {
        clause_write_tstp(output, bank, clause, true, true, problem_type)?;
        output.write_char('\n').map_err(tstp_stack_write_error)?;
    }
    Ok(())
}

/// Returns the C `PStackClausePrintTSTP` shape.
///
/// # Errors
///
/// Returns a diagnostic under the same conditions as
/// [`pstack_clause_write_tstp`].
///
/// # Panics
///
/// Panics if a printed clause violates the C clause/literal/term printing
/// preconditions.
pub fn pstack_clause_print_tstp_string(
    bank: &TermBank,
    stack: &PStack<&Clause>,
    problem_type: ProblemType,
) -> Result<String, Diagnostic> {
    let mut output = String::new();
    pstack_clause_write_tstp(&mut output, bank, stack, problem_type)?;
    Ok(output)
}

fn tstp_stack_write_error(_error: fmt::Error) -> Diagnostic {
    Diagnostic::new(ErrorCode::OTHER_ERROR, "failed to write TSTP clause stack")
}

#[cfg(test)]
mod tests {
    use super::pstack_clause_print_tstp_string;
    use crate::basics::pstacks::PStack;
    use crate::basics::simple_stuff::ProblemType;
    use crate::clauses::clause::Clause;
    use crate::clauses::clause_props::{CP_INPUT_FORMULA, CP_TYPE_AXIOM, CP_TYPE_NEG_CONJECTURE};
    use crate::clauses::eqn::Eqn;
    use crate::clauses::eqnlist::EqnList;
    use crate::terms::signature::Signature;
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
        bank.signature_mut()
            .declare_final_type(f_code, type_.clone())
            .unwrap();
        let term = Term::const_cell_alloc(f_code);
        term.set_type(Some(type_));
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
    fn pstack_clause_print_tstp_string_preserves_stack_order_and_newlines() {
        let mut bank = test_bank();
        let first = typed_const(&mut bank, "sine_a");
        let second = typed_const(&mut bank, "sine_b");
        let third = typed_const(&mut bank, "sine_c");
        let mut unit = clause_from(vec![literal(&mut bank, &first, &second, true)]);
        unit.set_ident(1);
        unit.set_tptp_type(CP_TYPE_AXIOM);
        unit.set_prop(CP_INPUT_FORMULA);
        let mut mixed = clause_from(vec![
            literal(&mut bank, &second, &third, true),
            literal(&mut bank, &third, &first, false),
        ]);
        mixed.set_ident(2);
        mixed.set_tptp_type(CP_TYPE_NEG_CONJECTURE);
        let mut stack = PStack::new();
        stack.push(&unit);
        stack.push(&mixed);

        assert_eq!(
            pstack_clause_print_tstp_string(&bank, &stack, ProblemType::FirstOrder).unwrap(),
            concat!(
                "cnf(c_0_1, axiom, (sine_a=sine_b)).\n",
                "cnf(c_0_2, negated_conjecture, (sine_b=sine_c|sine_c!=sine_a)).\n",
            )
        );
    }

    #[test]
    fn pstack_clause_print_tstp_string_handles_empty_stack() {
        let bank = test_bank();
        let stack = PStack::new();

        assert_eq!(
            pstack_clause_print_tstp_string(&bank, &stack, ProblemType::FirstOrder).unwrap(),
            ""
        );
    }
}
