use crate::basics::error::Diagnostic;
use crate::clauses::clause::Clause;
use crate::clauses::eqn::Eqn;
use crate::heuristics::prio_funs::parse_prio_fun;
use crate::heuristics::wfcb::{wfcb_alloc, ClausePrioFun, Wfcb};
use crate::inout::basicparser::parse_float;
use crate::inout::scanner::{Scanner, TokenType};
use crate::terms::termbanks::TermBank;
use crate::terms::termfunc::term_weight;

const APP_VAR_MULT_DEFAULT: f64 = 1.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SimWeightParam {
    equal_weight: f64,
    var_var_clash: f64,
    var_term_clash: f64,
    term_term_clash: f64,
    app_var_mult: f64,
}

impl SimWeightParam {
    #[must_use]
    pub const fn new(
        equal_weight: f64,
        var_var_clash: f64,
        var_term_clash: f64,
        term_term_clash: f64,
        app_var_mult: f64,
    ) -> Self {
        Self {
            equal_weight,
            var_var_clash,
            var_term_clash,
            term_term_clash,
            app_var_mult,
        }
    }

    #[must_use]
    pub const fn equal_weight(self) -> f64 {
        self.equal_weight
    }

    #[must_use]
    pub const fn var_var_clash(self) -> f64 {
        self.var_var_clash
    }

    #[must_use]
    pub const fn var_term_clash(self) -> f64 {
        self.var_term_clash
    }

    #[must_use]
    pub const fn term_term_clash(self) -> f64 {
        self.term_term_clash
    }

    #[must_use]
    pub const fn app_var_mult(self) -> f64 {
        self.app_var_mult
    }
}

#[must_use]
pub const fn sim_weight_init(
    equal_weight: f64,
    var_var_clash: f64,
    var_term_clash: f64,
    term_term_clash: f64,
    app_var_mult: f64,
) -> SimWeightParam {
    SimWeightParam::new(
        equal_weight,
        var_var_clash,
        var_term_clash,
        term_term_clash,
        app_var_mult,
    )
}

#[must_use]
pub fn sim_weight_wfcb_init(
    prio_fun: ClausePrioFun,
    equal_weight: f64,
    var_var_clash: f64,
    var_term_clash: f64,
    term_term_clash: f64,
    app_var_mult: f64,
) -> Wfcb<SimWeightParam> {
    wfcb_alloc(
        sim_weight_wfcb_compute,
        prio_fun,
        sim_weight_exit,
        Some(sim_weight_init(
            equal_weight,
            var_var_clash,
            var_term_clash,
            term_term_clash,
            app_var_mult,
        )),
    )
}

pub fn sim_weight_parse(scanner: &mut Scanner) -> Result<Wfcb<SimWeightParam>, Diagnostic> {
    scanner.accept_tok(TokenType::OPEN_BRACKET)?;
    let prio_fun = parse_prio_fun(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let equal_weight = parse_float(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let var_var_clash = parse_float(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let var_term_clash = parse_float(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let term_term_clash = parse_float(scanner)?;

    let mut app_var_mult = APP_VAR_MULT_DEFAULT;
    if scanner.test_tok(TokenType::COMMA) {
        scanner.accept_tok(TokenType::COMMA)?;
        app_var_mult = parse_float(scanner)?;
    }

    scanner.accept_tok(TokenType::CLOSE_BRACKET)?;
    Ok(sim_weight_wfcb_init(
        prio_fun,
        equal_weight,
        var_var_clash,
        var_term_clash,
        term_term_clash,
        app_var_mult,
    ))
}

/// # Panics
///
/// Panics if two paired terms have the same top symbol but do not both have
/// initialized arguments for the left term's arity, matching the C helper's
/// direct argument traversal precondition.
#[must_use]
pub fn sim_eqn_weight(eqn: &Eqn, param: &SimWeightParam) -> f64 {
    let mut clash_weight = 0.0;
    let mut stack = vec![(eqn.left().clone(), eqn.right().clone())];

    while let Some((left, right)) = stack.pop() {
        if left.f_code() == right.f_code() {
            for index in 0..left.arity() {
                let left_arg = left
                    .argument(index)
                    .expect("sim weight requires initialized left arguments");
                let right_arg = right
                    .argument(index)
                    .expect("sim weight requires initialized right arguments");
                stack.push((left_arg, right_arg));
            }
        } else if left.is_free_var() {
            if right.is_free_var() {
                clash_weight += param.var_var_clash;
            } else {
                clash_weight += param.var_term_clash;
            }
        } else if right.is_free_var() {
            clash_weight += param.var_term_clash;
        } else {
            clash_weight += param.term_term_clash
                * i64_to_f64(term_weight(&left, 1, 1) + term_weight(&right, 1, 1));
        }
    }

    clash_weight
}

#[must_use]
pub fn sim_weight(bank: &TermBank, clause: &Clause, param: &SimWeightParam) -> f64 {
    let similarity = clause
        .literals()
        .as_slice()
        .iter()
        .map(|literal| sim_eqn_weight(literal, param))
        .sum::<f64>();
    similarity * 5.0 + clause.literal_weight(bank, 1.0, 1.0, 1.0, 1, 2, param.app_var_mult, false)
}

#[must_use]
pub fn sim_weight_compute(param: &SimWeightParam, bank: &TermBank, clause: &Clause) -> f64 {
    sim_weight(bank, clause, param)
}

fn sim_weight_wfcb_compute(
    data: Option<&mut SimWeightParam>,
    bank: &TermBank,
    clause: &Clause,
) -> f64 {
    match data {
        Some(data) => sim_weight_compute(data, bank, clause),
        None => panic!("Simweight WFCB requires initialized weight parameters"),
    }
}

fn sim_weight_exit(_data: SimWeightParam) {}

#[allow(clippy::cast_precision_loss)]
fn i64_to_f64(value: i64) -> f64 {
    value as f64
}

#[cfg(test)]
mod tests {
    use super::{sim_eqn_weight, sim_weight_compute, sim_weight_init, sim_weight_parse};
    use crate::clauses::clause::Clause;
    use crate::clauses::eqn::Eqn;
    use crate::clauses::eqnlist::EqnList;
    use crate::clauses::neweval::PRIO_NORMAL;
    use crate::inout::scanner::Scanner;
    use crate::terms::signature::Signature;
    use crate::terms::simpletypes::alloc_arrow_type;
    use crate::terms::termbanks::TermBank;
    use crate::terms::termtypes::{DerefType, Term};
    use crate::terms::typebanks::TypeBank;

    fn assert_close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < f64::EPSILON,
            "expected {expected}, got {actual}"
        );
    }

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

    fn typed_unary(bank: &mut TermBank, name: &str, arg: &Term) -> Term {
        let type_ = bank.signature().type_bank().default_type();
        let f_code = bank.signature_mut().insert_id(name, 1, false);
        bank.signature_mut()
            .declare_final_type(f_code, alloc_arrow_type(vec![type_.clone(), type_.clone()]))
            .unwrap();
        typed_unary_with_code(bank, f_code, arg)
    }

    fn typed_unary_with_code(bank: &mut TermBank, f_code: i64, arg: &Term) -> Term {
        let type_ = bank.signature().type_bank().default_type();
        let term = Term::top_alloc(f_code, 1);
        term.set_type(Some(type_));
        term.set_argument(0, arg.clone());
        bank.insert(&term, DerefType::Never).unwrap()
    }

    fn typed_var(bank: &TermBank, f_code: i64) -> Term {
        let type_ = bank.signature().type_bank().default_type();
        bank.vars().var_assert_alloc(f_code, &type_)
    }

    fn unit_clause(bank: &mut TermBank, left: &Term, right: &Term, positive: bool) -> Clause {
        let literal = Eqn::alloc(left.clone(), right.clone(), bank, positive).unwrap();
        Clause::alloc(EqnList::from_vec(vec![literal]))
    }

    #[test]
    fn sim_eqn_weight_descends_matching_symbols_to_variable_clash() {
        let mut bank = test_bank();
        let x = typed_var(&bank, -2);
        let y = typed_var(&bank, -4);
        let type_ = bank.signature().type_bank().default_type();
        let f_code = bank.signature_mut().insert_id("f", 1, false);
        bank.signature_mut()
            .declare_final_type(f_code, alloc_arrow_type(vec![type_.clone(), type_.clone()]))
            .unwrap();
        let fx = typed_unary_with_code(&mut bank, f_code, &x);
        let fy = typed_unary_with_code(&mut bank, f_code, &y);
        let literal = Eqn::alloc(fx, fy, &mut bank, true).unwrap();
        let param = sim_weight_init(100.0, 3.0, 5.0, 7.0, 1.0);

        assert_close(sim_eqn_weight(&literal, &param), 3.0);
        assert_close(param.equal_weight(), 100.0);
        assert_close(param.var_var_clash(), 3.0);
        assert_close(param.var_term_clash(), 5.0);
        assert_close(param.term_term_clash(), 7.0);
        assert_close(param.app_var_mult(), 1.0);
    }

    #[test]
    fn sim_weight_uses_term_clash_times_five_plus_base_clause_weight() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let fa = typed_unary(&mut bank, "f", &a);
        let gb = typed_unary(&mut bank, "g", &b);
        let clause = unit_clause(&mut bank, &fa, &gb, true);
        let param = sim_weight_init(100.0, 3.0, 5.0, 7.0, 1.0);

        assert_close(sim_weight_compute(&param, &bank, &clause), 150.0);

        let ignored_equal = sim_weight_init(-1.0, 3.0, 5.0, 7.0, 1.0);
        assert_close(sim_weight_compute(&ignored_equal, &bank, &clause), 150.0);
    }

    #[test]
    fn sim_weight_parse_wraps_existing_scoring_core() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let fa = typed_unary(&mut bank, "f", &a);
        let gb = typed_unary(&mut bank, "g", &b);
        let clause = unit_clause(&mut bank, &fa, &gb, true);
        let mut scanner = Scanner::from_user_string("(ConstPrio,100.0,3.0,5.0,7.0) tail", false)
            .unwrap_or_else(|err| panic!("{err}"));
        let mut wfcb = sim_weight_parse(&mut scanner).unwrap_or_else(|err| panic!("{err}"));

        assert_close(wfcb.compute_eval(&bank, &clause), 150.0);
        assert_eq!(wfcb.compute_priority(&bank, &clause), PRIO_NORMAL);
        assert_eq!(scanner.current_token().literal(), "tail");
    }
}
