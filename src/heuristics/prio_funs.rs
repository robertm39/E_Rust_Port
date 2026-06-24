use crate::basics::error::{Diagnostic, ErrorCode};
use crate::clauses::clause::Clause;
use crate::clauses::clause_props::{CP_INITIAL, CP_IS_PROCESSED, CP_IS_SOS, CP_SUBSUMES_WATCH};
use crate::inout::scanner::{token_pos_rep, Scanner, TokenType};
use crate::terms::termbanks::TermBank;
use crate::terms::termtypes::Term;

pub type EvalPriority = i64;
pub type ClausePrioFun = fn(&TermBank, &Clause) -> EvalPriority;

pub const PRIO_BEST: EvalPriority = 0;
pub const PRIO_PREFER: EvalPriority = 30;
pub const PRIO_NORMAL: EvalPriority = 40;
pub const PRIO_DEFER: EvalPriority = 50;

pub const PRIO_FUN_NAMES: [&str; 42] = [
    "PreferGroundGoals",
    "PreferUnitGroundGoals",
    "PreferGround",
    "PreferNonGround",
    "PreferProcessed",
    "PreferNew",
    "PreferGoals",
    "PreferNonGoals",
    "PreferMixed",
    "PreferPositive",
    "PreferNegative",
    "PreferUnits",
    "PreferNonEqUnits",
    "PreferDemods",
    "PreferNonUnits",
    "ConstPrio",
    "ByLiteralNumber",
    "ByDerivationDepth",
    "ByDerivationSize",
    "ByNegLitDist",
    "ByGoalDifficulty",
    "SimulateSOS",
    "DeferSOS",
    "PreferHorn",
    "PreferNonHorn",
    "PreferUnitAndNonEq",
    "DeferNonUnitMaxEq",
    "ByCreationDate",
    "ByPosLitNo",
    "ByHornDist",
    "PreferWatchlist",
    "DeferWatchlist",
    "PreferAppVar",
    "PreferNonAppVar",
    "ByAppVarNum",
    "PreferHOSteps",
    "PreferLambdas",
    "DeferLambdas",
    "PreferFormulas",
    "DeferFormulas",
    "PreferEasyHO",
    "PreferFO",
];

const PRIO_FUNS: [ClausePrioFun; PRIO_FUN_NAMES.len()] = [
    prio_fun_prefer_ground_goals,
    prio_fun_prefer_unit_ground_goals,
    prio_fun_prefer_ground,
    prio_fun_prefer_non_ground,
    prio_fun_prefer_processed,
    prio_fun_prefer_new,
    prio_fun_prefer_goals,
    prio_fun_prefer_non_goals,
    prio_fun_prefer_mixed,
    prio_fun_prefer_positive,
    prio_fun_prefer_negative,
    prio_fun_prefer_units,
    prio_fun_prefer_non_eq_units,
    prio_fun_prefer_demods,
    prio_fun_prefer_non_units,
    prio_fun_const_prio,
    prio_fun_by_literal_number,
    prio_fun_by_derivation_depth,
    prio_fun_by_derivation_size,
    prio_fun_by_neg_lit_dist,
    prio_fun_goal_difficulty,
    prio_fun_simulate_sos,
    prio_fun_defer_sos,
    prio_fun_prefer_horn,
    prio_fun_prefer_non_horn,
    prio_fun_prefer_unit_and_non_eq,
    prio_fun_defer_non_unit_max_pos_eq,
    prio_fun_by_creation_date,
    prio_fun_by_pos_lit_no,
    prio_fun_by_horn_dist,
    prio_fun_prefer_watchlist,
    prio_fun_defer_watchlist,
    prio_fun_prefer_app_var,
    prio_fun_prefer_non_app_var,
    prio_fun_by_app_var_num,
    prio_fun_prefer_ho_steps,
    prio_fun_prefer_lambdas,
    prio_fun_defer_lambdas,
    prio_fun_prefer_formulas,
    prio_fun_defer_formulas,
    prio_fun_prefer_easy_ho,
    prio_fun_prefer_fo,
];

#[must_use]
pub fn get_prio_fun(name: &str) -> Option<ClausePrioFun> {
    PRIO_FUN_NAMES
        .iter()
        .zip(PRIO_FUNS)
        .find_map(|(candidate, function)| (*candidate == name).then_some(function))
}

pub fn parse_prio_fun(scanner: &mut Scanner) -> Result<ClausePrioFun, Diagnostic> {
    scanner.check_tok(TokenType::NAME)?;
    let name = scanner.current_token().literal();
    let prio_fun = get_prio_fun(&name).ok_or_else(|| {
        Diagnostic::new(
            ErrorCode::SYNTAX_ERROR,
            format!(
                "{}(just read '{}'): Not a valid priority-function",
                token_pos_rep(scanner.current_token()),
                scanner.current_token().literal()
            ),
        )
    })?;
    scanner.next_token()?;
    Ok(prio_fun)
}

#[must_use]
pub fn prio_fun_prefer_ground_goals(_bank: &TermBank, clause: &Clause) -> EvalPriority {
    prefer_if(clause.is_goal() && clause.is_ground())
}

#[must_use]
pub fn prio_fun_prefer_unit_ground_goals(_bank: &TermBank, clause: &Clause) -> EvalPriority {
    prefer_if(clause.is_unit() && clause.is_goal() && clause.is_ground())
}

#[must_use]
pub fn prio_fun_prefer_ground(_bank: &TermBank, clause: &Clause) -> EvalPriority {
    prefer_if(clause.is_ground())
}

#[must_use]
pub fn prio_fun_prefer_non_ground(_bank: &TermBank, clause: &Clause) -> EvalPriority {
    prefer_if(!clause.is_ground())
}

#[must_use]
pub fn prio_fun_prefer_processed(_bank: &TermBank, clause: &Clause) -> EvalPriority {
    prefer_if(clause.query_prop(CP_IS_PROCESSED))
}

#[must_use]
pub fn prio_fun_prefer_new(_bank: &TermBank, clause: &Clause) -> EvalPriority {
    prefer_if(!clause.query_prop(CP_IS_PROCESSED))
}

#[must_use]
pub fn prio_fun_prefer_goals(_bank: &TermBank, clause: &Clause) -> EvalPriority {
    prefer_if(clause.is_goal())
}

#[must_use]
pub fn prio_fun_prefer_non_goals(_bank: &TermBank, clause: &Clause) -> EvalPriority {
    prefer_if(!clause.is_goal())
}

#[must_use]
pub fn prio_fun_prefer_mixed(_bank: &TermBank, clause: &Clause) -> EvalPriority {
    let has_positive = clause.positive_literal_count() != 0;
    let has_negative = clause.negative_literal_count() != 0;
    prefer_if(has_positive == has_negative)
}

#[must_use]
pub fn prio_fun_prefer_positive(_bank: &TermBank, clause: &Clause) -> EvalPriority {
    prefer_if(clause.is_positive())
}

#[must_use]
pub fn prio_fun_prefer_negative(_bank: &TermBank, clause: &Clause) -> EvalPriority {
    prefer_if(clause.is_negative())
}

#[must_use]
pub fn prio_fun_prefer_units(_bank: &TermBank, clause: &Clause) -> EvalPriority {
    prefer_if(clause.is_unit())
}

#[must_use]
pub fn prio_fun_prefer_non_eq_units(bank: &TermBank, clause: &Clause) -> EvalPriority {
    prefer_if(clause.is_unit() && !clause.is_equational(bank))
}

#[must_use]
pub fn prio_fun_prefer_demods(bank: &TermBank, clause: &Clause) -> EvalPriority {
    prefer_if(clause.is_unit() && clause.is_equational(bank) && clause.is_positive())
}

#[must_use]
pub fn prio_fun_prefer_non_units(_bank: &TermBank, clause: &Clause) -> EvalPriority {
    prefer_if(!clause.is_unit())
}

#[must_use]
pub const fn prio_fun_const_prio(_bank: &TermBank, _clause: &Clause) -> EvalPriority {
    PRIO_NORMAL
}

#[must_use]
pub fn prio_fun_by_literal_number(_bank: &TermBank, clause: &Clause) -> EvalPriority {
    usize_to_eval_priority(clause.literal_number())
}

#[must_use]
pub fn prio_fun_by_app_var_num(_bank: &TermBank, clause: &Clause) -> EvalPriority {
    clause
        .literals()
        .as_slice()
        .iter()
        .map(|literal| {
            EvalPriority::from(literal.left().is_applied_free_var())
                + EvalPriority::from(literal.right().is_applied_free_var())
        })
        .sum()
}

#[must_use]
pub const fn prio_fun_by_derivation_depth(_bank: &TermBank, clause: &Clause) -> EvalPriority {
    clause.proof_depth()
}

#[must_use]
pub const fn prio_fun_by_derivation_size(_bank: &TermBank, clause: &Clause) -> EvalPriority {
    clause.proof_size()
}

#[must_use]
pub fn prio_fun_by_neg_lit_dist(_bank: &TermBank, clause: &Clause) -> EvalPriority {
    let mut result = PRIO_NORMAL;
    for literal in clause.literals().as_slice() {
        if literal.is_positive() {
            result = 400;
            break;
        }
        result += if literal.is_ground() { 1 } else { 3 };
    }
    result
}

#[must_use]
pub fn prio_fun_goal_difficulty(_bank: &TermBank, clause: &Clause) -> EvalPriority {
    if !clause.is_goal() {
        return PRIO_NORMAL;
    }
    if clause.is_unit() {
        if clause.is_ground() {
            return PRIO_PREFER;
        }
        return PRIO_PREFER + 1;
    }
    if clause.is_ground() {
        return PRIO_PREFER + 2;
    }
    PRIO_PREFER + 3
}

#[must_use]
pub fn prio_fun_simulate_sos(_bank: &TermBank, clause: &Clause) -> EvalPriority {
    if clause.is_any_prop_set(CP_INITIAL | CP_IS_SOS) {
        PRIO_NORMAL
    } else {
        PRIO_DEFER
    }
}

#[must_use]
pub fn prio_fun_defer_sos(_bank: &TermBank, clause: &Clause) -> EvalPriority {
    if clause.is_any_prop_set(CP_INITIAL | CP_IS_SOS) {
        PRIO_DEFER
    } else {
        PRIO_NORMAL
    }
}

#[must_use]
pub fn prio_fun_prefer_horn(_bank: &TermBank, clause: &Clause) -> EvalPriority {
    prefer_if(clause.is_horn())
}

#[must_use]
pub fn prio_fun_prefer_non_horn(_bank: &TermBank, clause: &Clause) -> EvalPriority {
    prefer_if(!clause.is_horn())
}

#[must_use]
pub fn prio_fun_prefer_unit_and_non_eq(bank: &TermBank, clause: &Clause) -> EvalPriority {
    if clause.is_unit() {
        return PRIO_PREFER;
    }
    if clause.is_equational(bank) {
        return PRIO_NORMAL;
    }
    PRIO_PREFER
}

#[must_use]
pub fn prio_fun_defer_non_unit_max_pos_eq(bank: &TermBank, clause: &Clause) -> EvalPriority {
    if clause.is_unit() {
        return PRIO_PREFER;
    }
    if clause.has_max_pos_eq_lit(bank) {
        return PRIO_NORMAL;
    }
    PRIO_PREFER
}

#[must_use]
pub const fn prio_fun_by_creation_date(_bank: &TermBank, clause: &Clause) -> EvalPriority {
    clause.create_date()
}

#[must_use]
pub fn prio_fun_by_pos_lit_no(_bank: &TermBank, clause: &Clause) -> EvalPriority {
    usize_to_eval_priority(clause.positive_literal_count())
}

#[must_use]
pub fn prio_fun_by_horn_dist(_bank: &TermBank, clause: &Clause) -> EvalPriority {
    usize_to_eval_priority(clause.positive_literal_count().saturating_sub(1))
}

#[must_use]
pub fn prio_fun_prefer_watchlist(_bank: &TermBank, clause: &Clause) -> EvalPriority {
    prefer_if(clause.query_prop(CP_SUBSUMES_WATCH))
}

#[must_use]
pub fn prio_fun_defer_watchlist(_bank: &TermBank, clause: &Clause) -> EvalPriority {
    if clause.query_prop(CP_SUBSUMES_WATCH) {
        PRIO_DEFER
    } else {
        PRIO_NORMAL
    }
}

#[must_use]
pub fn prio_fun_prefer_app_var(_bank: &TermBank, clause: &Clause) -> EvalPriority {
    prefer_if(clause.query_literal(crate::clauses::eqn::Eqn::has_app_var))
}

#[must_use]
pub fn prio_fun_prefer_non_app_var(_bank: &TermBank, clause: &Clause) -> EvalPriority {
    prefer_if(!clause.query_literal(crate::clauses::eqn::Eqn::has_app_var))
}

#[must_use]
pub const fn prio_fun_prefer_ho_steps(_bank: &TermBank, _clause: &Clause) -> EvalPriority {
    PRIO_NORMAL
}

#[must_use]
pub fn prio_fun_prefer_lambdas(_bank: &TermBank, clause: &Clause) -> EvalPriority {
    prefer_if(
        clause.literals().as_slice().iter().any(|literal| {
            literal.left().has_lambda_subterm() || literal.right().has_lambda_subterm()
        }),
    )
}

#[must_use]
pub fn prio_fun_defer_lambdas(bank: &TermBank, clause: &Clause) -> EvalPriority {
    defer_preferred(prio_fun_prefer_lambdas(bank, clause))
}

#[must_use]
/// # Panics
///
/// Panics if a traversed non-lambda term has an uninitialized argument slot,
/// matching the C helper's direct `args[i]` access precondition.
pub fn prio_fun_prefer_formulas(bank: &TermBank, clause: &Clause) -> EvalPriority {
    for literal in clause.literals().as_slice() {
        let mut subterms = Vec::new();
        subterms.push(literal.left().clone());
        if literal.is_equ_lit(bank) {
            subterms.push(literal.right().clone());
        }

        while let Some(term) = subterms.pop() {
            if is_formula_subterm(bank, &term) {
                return PRIO_PREFER;
            }
            if !term.is_lambda() {
                let start = usize::from(term.is_phony_app());
                for index in start..term.arity() {
                    subterms.push(term.argument(index).unwrap_or_else(|| {
                        panic!("formula priority traversal requires initialized term arguments")
                    }));
                }
            }
        }
    }
    PRIO_NORMAL
}

#[must_use]
pub fn prio_fun_defer_formulas(bank: &TermBank, clause: &Clause) -> EvalPriority {
    defer_preferred(prio_fun_prefer_formulas(bank, clause))
}

#[must_use]
pub const fn prio_fun_prefer_easy_ho(_bank: &TermBank, _clause: &Clause) -> EvalPriority {
    PRIO_NORMAL
}

#[must_use]
pub fn prio_fun_prefer_fo(_bank: &TermBank, clause: &Clause) -> EvalPriority {
    if clause
        .literals()
        .as_slice()
        .iter()
        .any(|literal| !literal.left().is_pattern() || !literal.right().is_pattern())
    {
        PRIO_DEFER
    } else {
        PRIO_NORMAL
    }
}

#[must_use]
const fn prefer_if(condition: bool) -> EvalPriority {
    if condition {
        PRIO_PREFER
    } else {
        PRIO_NORMAL
    }
}

#[must_use]
const fn defer_preferred(priority: EvalPriority) -> EvalPriority {
    if priority == PRIO_PREFER {
        PRIO_DEFER
    } else {
        priority
    }
}

#[must_use]
fn is_formula_subterm(bank: &TermBank, term: &Term) -> bool {
    !term.is_free_var()
        && term.type_().is_some_and(|type_| type_.is_bool())
        && term != bank.true_term()
        && term != bank.false_term()
        && bank.signature().is_logical_symbol(term.f_code())
}

#[must_use]
fn usize_to_eval_priority(value: usize) -> EvalPriority {
    EvalPriority::try_from(value).unwrap_or(EvalPriority::MAX)
}

#[cfg(test)]
mod tests {
    use super::{
        get_prio_fun, parse_prio_fun, prio_fun_by_app_var_num, prio_fun_by_creation_date,
        prio_fun_by_derivation_depth, prio_fun_by_derivation_size, prio_fun_by_horn_dist,
        prio_fun_by_literal_number, prio_fun_by_neg_lit_dist, prio_fun_by_pos_lit_no,
        prio_fun_const_prio, prio_fun_defer_formulas, prio_fun_defer_lambdas,
        prio_fun_defer_non_unit_max_pos_eq, prio_fun_defer_sos, prio_fun_defer_watchlist,
        prio_fun_goal_difficulty, prio_fun_prefer_app_var, prio_fun_prefer_demods,
        prio_fun_prefer_easy_ho, prio_fun_prefer_fo, prio_fun_prefer_formulas,
        prio_fun_prefer_goals, prio_fun_prefer_ground, prio_fun_prefer_ground_goals,
        prio_fun_prefer_ho_steps, prio_fun_prefer_horn, prio_fun_prefer_lambdas,
        prio_fun_prefer_mixed, prio_fun_prefer_negative, prio_fun_prefer_new,
        prio_fun_prefer_non_app_var, prio_fun_prefer_non_eq_units, prio_fun_prefer_non_goals,
        prio_fun_prefer_non_ground, prio_fun_prefer_non_horn, prio_fun_prefer_non_units,
        prio_fun_prefer_positive, prio_fun_prefer_processed, prio_fun_prefer_unit_and_non_eq,
        prio_fun_prefer_unit_ground_goals, prio_fun_prefer_units, prio_fun_prefer_watchlist,
        prio_fun_simulate_sos, EvalPriority, PRIO_BEST, PRIO_DEFER, PRIO_FUN_NAMES, PRIO_NORMAL,
        PRIO_PREFER,
    };
    use crate::basics::error::ErrorCode;
    use crate::clauses::clause::Clause;
    use crate::clauses::clause_props::{CP_INITIAL, CP_IS_PROCESSED, CP_IS_SOS, CP_SUBSUMES_WATCH};
    use crate::clauses::eqn::Eqn;
    use crate::clauses::eqn_props::{EP_IS_MAXIMAL, EP_IS_ORIENTED};
    use crate::clauses::eqnlist::EqnList;
    use crate::inout::scanner::Scanner;
    use crate::terms::functypes::FunCode;
    use crate::terms::signature::{Signature, SIG_NAMED_LAMBDA_CODE, SIG_PHONY_APP_CODE};
    use crate::terms::simpletypes::{alloc_arrow_type, Type};
    use crate::terms::termbanks::TermBank;
    use crate::terms::termtypes::{DerefType, Term, TP_HAS_NON_PATTERN_VAR};
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

    fn typed_var(bank: &TermBank, f_code: FunCode) -> Term {
        bank.vars().var_assert_alloc(f_code, &individual(bank))
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

    fn logical_not_term(bank: &mut TermBank, arg: &Term) -> Term {
        let bool_type = bank.signature().type_bank().bool_type();
        let term = Term::top_alloc(bank.signature().not_code(), 1);
        term.set_type(Some(bool_type));
        term.set_argument(0, arg.clone());
        let shared = bank
            .insert(&term, DerefType::Never)
            .unwrap_or_else(|err| panic!("{err}"));
        shared.set_type(Some(bank.signature().type_bank().bool_type()));
        shared
    }

    fn applied_var(bank: &mut TermBank, arg: &Term) -> Term {
        let type_ = individual(bank);
        let var = typed_var(bank, -2);
        let app = Term::top_alloc(SIG_PHONY_APP_CODE, 2);
        app.set_type(Some(type_));
        app.set_argument(0, var);
        app.set_argument(1, arg.clone());
        bank.insert(&app, DerefType::Never)
            .unwrap_or_else(|err| panic!("{err}"))
    }

    fn lambda_term(bank: &mut TermBank) -> Term {
        let type_ = individual(bank);
        let binder = typed_var(bank, -6);
        let body = typed_const(bank, "lambda_body");
        let lambda = Term::top_alloc(SIG_NAMED_LAMBDA_CODE, 2);
        lambda.set_type(Some(type_));
        lambda.set_argument(0, binder);
        lambda.set_argument(1, body);
        bank.insert(&lambda, DerefType::Never)
            .unwrap_or_else(|err| panic!("{err}"))
    }

    fn equation(bank: &mut TermBank, left: &Term, right: &Term, positive: bool) -> Eqn {
        Eqn::alloc(left.clone(), right.clone(), bank, positive)
            .unwrap_or_else(|err| panic!("{err}"))
    }

    fn true_literal(bank: &mut TermBank) -> Eqn {
        Eqn::create_true_lit(bank).unwrap_or_else(|err| panic!("{err}"))
    }

    fn clause_from(literals: Vec<Eqn>) -> Clause {
        Clause::alloc(EqnList::from_vec(literals))
    }

    fn priority(function_name: &str, bank: &TermBank, clause: &Clause) -> EvalPriority {
        let function = get_prio_fun(function_name).unwrap_or_else(|| panic!("{function_name}"));
        function(bank, clause)
    }

    #[test]
    fn constants_and_name_table_match_c_order() {
        assert_eq!(PRIO_BEST, 0);
        assert_eq!(PRIO_PREFER, 30);
        assert_eq!(PRIO_NORMAL, 40);
        assert_eq!(PRIO_DEFER, 50);
        assert_eq!(PRIO_FUN_NAMES[0], "PreferGroundGoals");
        assert_eq!(PRIO_FUN_NAMES[26], "DeferNonUnitMaxEq");
        assert_eq!(PRIO_FUN_NAMES[32], "PreferAppVar");
        assert_eq!(PRIO_FUN_NAMES[34], "ByAppVarNum");
        assert_eq!(PRIO_FUN_NAMES[41], "PreferFO");
        assert!(get_prio_fun("PreferGround").is_some());
        assert!(get_prio_fun("DeferNonUnitMaxPosEq").is_none());
    }

    #[test]
    fn parse_prio_fun_accepts_known_name_and_rejects_unknown_name() {
        let bank = term_bank();
        let clause = Clause::empty();
        let mut scanner =
            Scanner::from_user_string("PreferGround rest", false).unwrap_or_else(|err| {
                panic!("{err}");
            });
        let function = parse_prio_fun(&mut scanner).unwrap_or_else(|err| panic!("{err}"));
        assert_eq!(function(&bank, &clause), PRIO_PREFER);
        assert_eq!(scanner.current_token().literal(), "rest");

        let mut scanner =
            Scanner::from_user_string("NoSuchPriority", false).unwrap_or_else(|err| {
                panic!("{err}");
            });
        let err = parse_prio_fun(&mut scanner).unwrap_err();
        assert_eq!(err.code(), ErrorCode::SYNTAX_ERROR);
        assert!(err.message().contains("Not a valid priority-function"));
    }

    #[test]
    fn basic_clause_classification_priorities_match_c_macros() {
        let mut bank = term_bank();
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let x = typed_var(&bank, -2);
        let fa = typed_unary(&mut bank, "f", &a);

        let positive = clause_from(vec![equation(&mut bank, &fa, &a, true)]);
        let negative_ground = clause_from(vec![equation(&mut bank, &a, &b, false)]);
        let negative_non_ground = clause_from(vec![equation(&mut bank, &x, &a, false)]);
        let mixed = clause_from(vec![
            equation(&mut bank, &fa, &a, true),
            equation(&mut bank, &a, &b, false),
        ]);
        let empty = Clause::empty();

        assert_eq!(
            prio_fun_prefer_ground_goals(&bank, &negative_ground),
            PRIO_PREFER
        );
        assert_eq!(
            prio_fun_prefer_unit_ground_goals(&bank, &negative_ground),
            PRIO_PREFER
        );
        assert_eq!(
            prio_fun_prefer_unit_ground_goals(&bank, &negative_non_ground),
            PRIO_NORMAL
        );
        assert_eq!(prio_fun_prefer_ground(&bank, &mixed), PRIO_PREFER);
        assert_eq!(
            prio_fun_prefer_non_ground(&bank, &negative_non_ground),
            PRIO_PREFER
        );
        assert_eq!(prio_fun_prefer_goals(&bank, &negative_ground), PRIO_PREFER);
        assert_eq!(prio_fun_prefer_non_goals(&bank, &positive), PRIO_PREFER);
        assert_eq!(prio_fun_prefer_mixed(&bank, &mixed), PRIO_PREFER);
        assert_eq!(prio_fun_prefer_mixed(&bank, &empty), PRIO_PREFER);
        assert_eq!(prio_fun_prefer_positive(&bank, &empty), PRIO_PREFER);
        assert_eq!(prio_fun_prefer_negative(&bank, &empty), PRIO_PREFER);
        assert_eq!(prio_fun_prefer_units(&bank, &positive), PRIO_PREFER);
        assert_eq!(prio_fun_prefer_non_units(&bank, &mixed), PRIO_PREFER);
        assert_eq!(prio_fun_prefer_horn(&bank, &mixed), PRIO_PREFER);
        assert_eq!(prio_fun_prefer_non_horn(&bank, &mixed), PRIO_NORMAL);
    }

    #[test]
    fn equational_priority_functions_use_any_equational_literal_like_c() {
        let mut bank = term_bank();
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let predicate_unit = clause_from(vec![true_literal(&mut bank)]);
        let mut maximal_equation = equation(&mut bank, &a, &b, true);
        maximal_equation.set_prop(EP_IS_MAXIMAL | EP_IS_ORIENTED);
        let equational_unit = clause_from(vec![maximal_equation.clone()]);
        let mixed_non_unit = clause_from(vec![true_literal(&mut bank), maximal_equation]);

        assert_eq!(
            prio_fun_prefer_non_eq_units(&bank, &predicate_unit),
            PRIO_PREFER
        );
        assert_eq!(
            prio_fun_prefer_non_eq_units(&bank, &equational_unit),
            PRIO_NORMAL
        );
        assert_eq!(prio_fun_prefer_demods(&bank, &equational_unit), PRIO_PREFER);
        assert_eq!(
            prio_fun_prefer_unit_and_non_eq(&bank, &predicate_unit),
            PRIO_PREFER
        );
        assert_eq!(
            prio_fun_prefer_unit_and_non_eq(&bank, &mixed_non_unit),
            PRIO_NORMAL
        );
        assert_eq!(
            prio_fun_defer_non_unit_max_pos_eq(&bank, &mixed_non_unit),
            PRIO_NORMAL
        );
        assert_eq!(
            prio_fun_defer_non_unit_max_pos_eq(&bank, &predicate_unit),
            PRIO_PREFER
        );
    }

    #[test]
    fn scalar_and_count_priorities_match_clause_fields() {
        let mut bank = term_bank();
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let x = typed_var(&bank, -2);
        let y = typed_var(&bank, -4);
        let mut clause = clause_from(vec![
            equation(&mut bank, &x, &a, false),
            equation(&mut bank, &y, &b, false),
        ]);
        clause.set_create_date(17);
        clause.set_proof_depth(3);
        clause.set_proof_size(8);

        assert_eq!(prio_fun_const_prio(&bank, &clause), PRIO_NORMAL);
        assert_eq!(prio_fun_by_literal_number(&bank, &clause), 2);
        assert_eq!(prio_fun_by_derivation_depth(&bank, &clause), 3);
        assert_eq!(prio_fun_by_derivation_size(&bank, &clause), 8);
        assert_eq!(prio_fun_by_creation_date(&bank, &clause), 17);
        assert_eq!(prio_fun_by_pos_lit_no(&bank, &clause), 0);
        assert_eq!(prio_fun_by_horn_dist(&bank, &clause), 0);
        assert_eq!(prio_fun_by_neg_lit_dist(&bank, &clause), PRIO_NORMAL + 6);

        let positive = equation(&mut bank, &a, &b, true);
        let positive_clause = clause_from(vec![positive]);
        assert_eq!(prio_fun_by_neg_lit_dist(&bank, &positive_clause), 400);
        assert_eq!(prio_fun_goal_difficulty(&bank, &clause), PRIO_PREFER + 3);
        assert_eq!(
            prio_fun_goal_difficulty(&bank, &positive_clause),
            PRIO_NORMAL
        );
        assert_eq!(
            prio_fun_goal_difficulty(&bank, &Clause::empty()),
            PRIO_PREFER + 2
        );
    }

    #[test]
    fn property_priorities_match_processed_sos_and_watchlist_bits() {
        let bank = term_bank();
        let mut clause = Clause::empty();

        assert_eq!(prio_fun_prefer_new(&bank, &clause), PRIO_PREFER);
        assert_eq!(prio_fun_prefer_processed(&bank, &clause), PRIO_NORMAL);
        clause.set_prop(CP_IS_PROCESSED);
        assert_eq!(prio_fun_prefer_processed(&bank, &clause), PRIO_PREFER);
        assert_eq!(prio_fun_prefer_new(&bank, &clause), PRIO_NORMAL);

        assert_eq!(prio_fun_simulate_sos(&bank, &clause), PRIO_DEFER);
        assert_eq!(prio_fun_defer_sos(&bank, &clause), PRIO_NORMAL);
        clause.set_prop(CP_IS_SOS);
        assert_eq!(prio_fun_simulate_sos(&bank, &clause), PRIO_NORMAL);
        assert_eq!(prio_fun_defer_sos(&bank, &clause), PRIO_DEFER);
        clause.del_prop(CP_IS_SOS);
        clause.set_prop(CP_INITIAL);
        assert_eq!(prio_fun_simulate_sos(&bank, &clause), PRIO_NORMAL);
        assert_eq!(prio_fun_defer_sos(&bank, &clause), PRIO_DEFER);

        assert_eq!(prio_fun_prefer_watchlist(&bank, &clause), PRIO_NORMAL);
        assert_eq!(prio_fun_defer_watchlist(&bank, &clause), PRIO_NORMAL);
        clause.set_prop(CP_SUBSUMES_WATCH);
        assert_eq!(prio_fun_prefer_watchlist(&bank, &clause), PRIO_PREFER);
        assert_eq!(prio_fun_defer_watchlist(&bank, &clause), PRIO_DEFER);
    }

    #[test]
    fn applied_variable_priorities_only_count_top_level_sides() {
        let mut bank = term_bank();
        let a = typed_const(&mut bank, "a");
        let app = applied_var(&mut bank, &a);
        let wrapper = typed_unary(&mut bank, "wrap", &app);
        let top_level = clause_from(vec![equation(&mut bank, &app, &a, false)]);
        let nested_only = clause_from(vec![equation(&mut bank, &wrapper, &a, false)]);
        let both_sides = clause_from(vec![equation(&mut bank, &app, &app, false)]);

        assert_eq!(prio_fun_prefer_app_var(&bank, &top_level), PRIO_PREFER);
        assert_eq!(prio_fun_prefer_non_app_var(&bank, &top_level), PRIO_NORMAL);
        assert_eq!(prio_fun_by_app_var_num(&bank, &top_level), 1);
        assert_eq!(prio_fun_prefer_app_var(&bank, &nested_only), PRIO_NORMAL);
        assert_eq!(
            prio_fun_prefer_non_app_var(&bank, &nested_only),
            PRIO_PREFER
        );
        assert_eq!(prio_fun_by_app_var_num(&bank, &nested_only), 0);
        assert_eq!(prio_fun_by_app_var_num(&bank, &both_sides), 2);
    }

    #[test]
    fn lambda_formula_and_fo_priorities_match_c_predicates() {
        let mut bank = term_bank();
        let a = typed_const(&mut bank, "a");
        let lambda = lambda_term(&mut bank);
        let lambda_clause = clause_from(vec![equation(&mut bank, &lambda, &a, false)]);
        assert_eq!(prio_fun_prefer_lambdas(&bank, &lambda_clause), PRIO_PREFER);
        assert_eq!(prio_fun_defer_lambdas(&bank, &lambda_clause), PRIO_DEFER);

        let true_term = bank.true_term().clone();
        let not_true = logical_not_term(&mut bank, &true_term);
        let formula_clause = clause_from(vec![equation(&mut bank, &not_true, &not_true, true)]);
        assert_eq!(
            prio_fun_prefer_formulas(&bank, &formula_clause),
            PRIO_PREFER
        );
        assert_eq!(prio_fun_defer_formulas(&bank, &formula_clause), PRIO_DEFER);

        let b = typed_const(&mut bank, "b");
        let plain_clause = clause_from(vec![equation(&mut bank, &a, &b, true)]);
        assert_eq!(prio_fun_prefer_formulas(&bank, &plain_clause), PRIO_NORMAL);
        assert_eq!(prio_fun_defer_formulas(&bank, &plain_clause), PRIO_NORMAL);
        assert_eq!(prio_fun_prefer_fo(&bank, &plain_clause), PRIO_NORMAL);

        let non_pattern = typed_unary(&mut bank, "np", &a);
        non_pattern.set_prop(TP_HAS_NON_PATTERN_VAR);
        let non_pattern_clause = clause_from(vec![equation(&mut bank, &non_pattern, &a, false)]);
        assert_eq!(prio_fun_prefer_fo(&bank, &non_pattern_clause), PRIO_DEFER);
    }

    #[test]
    fn higher_order_priority_quirks_preserve_current_c_results() {
        let bank = term_bank();
        let clause = Clause::empty();
        assert_eq!(prio_fun_prefer_ho_steps(&bank, &clause), PRIO_NORMAL);
        assert_eq!(prio_fun_prefer_easy_ho(&bank, &clause), PRIO_NORMAL);
        assert_eq!(priority("PreferHOSteps", &bank, &clause), PRIO_NORMAL);
        assert_eq!(priority("PreferEasyHO", &bank, &clause), PRIO_NORMAL);
    }
}
