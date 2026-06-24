use crate::basics::error::{Diagnostic, ErrorCode};
use crate::clauses::clausesets::ClauseSet;
use crate::heuristics::clauseweight::{
    clause_weight_parse, cmax_weight_parse, default_weight_parse, lmax_weight_parse,
    uniq_weight_parse,
};
use crate::heuristics::dagweight::{
    dag_weight_parse, rdag_weight2_parse, rdag_weight3_parse, rdag_weight_parse,
};
use crate::heuristics::diversityweight::diversity_weight_parse;
use crate::heuristics::fifo::fifo_eval_parse;
use crate::heuristics::funweights::{
    conjecture_relative_symbol_weight_parse, conjecture_simplified_symbol_weight_parse,
    conjecture_symbol_weight_parse, fun_weight_parse, sym_offset_weight_parse,
};
use crate::heuristics::gdweight::gd_clause_weight_parse;
use crate::heuristics::lifo::lifo_eval_parse;
use crate::heuristics::orientweight::{clause_orient_weight_parse, orient_lmax_weight_parse};
use crate::heuristics::random::rand_weight_parse;
use crate::heuristics::refinedweight::{clause_refined_weight2_parse, clause_refined_weight_parse};
use crate::heuristics::simweight::sim_weight_parse;
use crate::heuristics::varweights::{
    clause_weight_age_parse, depth_weight_parse, nl_weight_parse, pn_refined_weight_parse,
    proof_weight_parse, sig_weight_parse, staggered_weight_parse, sym_type_weight_parse,
    tptp_type_weight_parse, weight_less_depth_parse,
};
use crate::heuristics::wfcb::BoxedWfcb;
use crate::inout::scanner::{token_pos_rep, Scanner, TokenType};

pub const WEIGHT_FUN_PARSE_FUN_NAMES: [&str; 46] = [
    "Clauseweight",
    "ClauseLMaxWeight",
    "ClauseCMaxWeight",
    "Uniqweight",
    "Defaultweight",
    "DAGweight",
    "RDAGweight",
    "RDAGweight2",
    "RDAGweight3",
    "Refinedweight",
    "Refinedweight2",
    "Diversityweight",
    "PNRefinedweight",
    "TPTPTypeweight",
    "Sigweight",
    "NLweight",
    "RandomWeight",
    "SymbolTypeweight",
    "Depthweight",
    "WLessDWeight",
    "Proofweight",
    "Orientweight",
    "OrientLMaxWeight",
    "Simweight",
    "FIFOWeight",
    "LIFOWeight",
    "StaggeredWeight",
    "ClauseWeightAge",
    "TSMWeight",
    "TSMRWeight",
    "ConjectureSymbolWeight",
    "ConjectureGeneralSymbolWeight",
    "ConjectureRelativeSymbolWeight",
    "ConjectureRelativeTypeSymbolWeight",
    "ConjectureTypeBasedWeight",
    "RelevanceLevelWeight",
    "RelevanceLevelWeight2",
    "FunWeight",
    "SymOffsetWeight",
    "ConjectureRelativeTermWeight",
    "ConjectureTermTfIdfWeight",
    "ConjectureLevDistanceWeight",
    "ConjectureTreeDistanceWeight",
    "ConjectureTermPrefixWeight",
    "ConjectureStrucDistanceWeight",
    "GDWeight",
];

#[derive(Clone, Copy, Debug, Default)]
pub struct WeightParseContext<'a> {
    axioms: Option<&'a ClauseSet>,
}

impl<'a> WeightParseContext<'a> {
    #[must_use]
    pub const fn empty() -> Self {
        Self { axioms: None }
    }

    #[must_use]
    pub const fn new(axioms: &'a ClauseSet) -> Self {
        Self {
            axioms: Some(axioms),
        }
    }

    #[must_use]
    pub const fn axioms(self) -> Option<&'a ClauseSet> {
        self.axioms
    }

    fn require_axioms(self, scanner: &Scanner, name: &str) -> Result<&'a ClauseSet, Diagnostic> {
        self.axioms.ok_or_else(|| {
            weight_fun_error(
                scanner,
                &format!("Weight function parser requires proof-state axioms: {name}"),
            )
        })
    }
}

pub struct WfcbAdmin {
    entries: Vec<WfcbAdminEntry>,
    anon_counter: i64,
}

struct WfcbAdminEntry {
    name: String,
    wfcb: BoxedWfcb,
}

impl Default for WfcbAdmin {
    fn default() -> Self {
        Self::new()
    }
}

impl WfcbAdmin {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            entries: Vec::new(),
            anon_counter: 0,
        }
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    #[must_use]
    pub const fn anon_counter(&self) -> i64 {
        self.anon_counter
    }

    #[must_use]
    pub fn name(&self, index: usize) -> Option<&str> {
        self.entries.get(index).map(|entry| entry.name.as_str())
    }

    pub fn add_wfcb(&mut self, name: impl Into<String>, wfcb: BoxedWfcb) -> usize {
        self.entries.push(WfcbAdminEntry {
            name: name.into(),
            wfcb,
        });
        self.entries.len() - 1
    }

    #[must_use]
    pub fn find_wfcb(&self, name: &str) -> Option<&dyn crate::heuristics::wfcb::WfcbOps> {
        self.entries
            .iter()
            .rev()
            .find(|entry| entry.name == name)
            .map(|entry| entry.wfcb.as_ref())
    }

    pub fn find_wfcb_mut(
        &mut self,
        name: &str,
    ) -> Option<&mut (dyn crate::heuristics::wfcb::WfcbOps + 'static)> {
        self.entries
            .iter_mut()
            .rev()
            .find(|entry| entry.name == name)
            .map(|entry| entry.wfcb.as_mut())
    }

    #[must_use]
    pub fn next_anonymous_name(&mut self) -> String {
        let name = format!("~${:09}", self.anon_counter);
        self.anon_counter += 1;
        name
    }

    pub fn weight_fun_def_parse(&mut self, scanner: &mut Scanner) -> Result<String, Diagnostic> {
        let context = WeightParseContext::empty();
        self.weight_fun_def_parse_with_context(scanner, context)
    }

    pub fn weight_fun_def_parse_with_context(
        &mut self,
        scanner: &mut Scanner,
        context: WeightParseContext<'_>,
    ) -> Result<String, Diagnostic> {
        let name = if scanner
            .look_token(1)
            .kind()
            .intersects(TokenType::EQUAL_SIGN)
        {
            scanner.check_tok(TokenType::IDENTIFIER)?;
            let name = scanner.current_token().literal();
            scanner.next_token()?;
            scanner.accept_tok(TokenType::EQUAL_SIGN)?;
            name
        } else {
            self.next_anonymous_name()
        };

        let wfcb = weight_fun_parse_with_context(scanner, context)?;
        let stored_name = name.clone();
        self.add_wfcb(name, wfcb);
        Ok(stored_name)
    }

    pub fn weight_fun_def_list_parse(
        &mut self,
        scanner: &mut Scanner,
    ) -> Result<usize, Diagnostic> {
        let context = WeightParseContext::empty();
        self.weight_fun_def_list_parse_with_context(scanner, context)
    }

    pub fn weight_fun_def_list_parse_with_context(
        &mut self,
        scanner: &mut Scanner,
        context: WeightParseContext<'_>,
    ) -> Result<usize, Diagnostic> {
        let mut parsed = 0;
        while scanner.test_tok(TokenType::IDENTIFIER)
            && scanner
                .look_token(1)
                .kind()
                .intersects(TokenType::EQUAL_SIGN | TokenType::OPEN_BRACKET)
        {
            self.weight_fun_def_parse_with_context(scanner, context)?;
            parsed += 1;
        }
        Ok(parsed)
    }
}

#[must_use]
pub const fn wfcb_admin_alloc() -> WfcbAdmin {
    WfcbAdmin::new()
}

#[must_use]
pub fn get_weight_fun_parse_fun_index(name: &str) -> Option<usize> {
    WEIGHT_FUN_PARSE_FUN_NAMES
        .iter()
        .position(|candidate| *candidate == name)
}

#[must_use]
pub fn weight_fun_parser_is_ported(name: &str) -> bool {
    matches!(
        name,
        "Clauseweight"
            | "ClauseLMaxWeight"
            | "ClauseCMaxWeight"
            | "Uniqweight"
            | "Defaultweight"
            | "DAGweight"
            | "RDAGweight"
            | "RDAGweight2"
            | "RDAGweight3"
            | "Refinedweight"
            | "Refinedweight2"
            | "Diversityweight"
            | "PNRefinedweight"
            | "TPTPTypeweight"
            | "Sigweight"
            | "NLweight"
            | "SymbolTypeweight"
            | "Depthweight"
            | "WLessDWeight"
            | "Proofweight"
            | "Orientweight"
            | "OrientLMaxWeight"
            | "Simweight"
            | "ClauseWeightAge"
            | "StaggeredWeight"
            | "GDWeight"
            | "ConjectureSymbolWeight"
            | "ConjectureGeneralSymbolWeight"
            | "ConjectureRelativeSymbolWeight"
            | "FunWeight"
            | "SymOffsetWeight"
            | "RandomWeight"
            | "FIFOWeight"
            | "LIFOWeight"
    )
}

pub fn weight_fun_parse(scanner: &mut Scanner) -> Result<BoxedWfcb, Diagnostic> {
    let context = WeightParseContext::empty();
    weight_fun_parse_with_context(scanner, context)
}

pub fn weight_fun_parse_with_context(
    scanner: &mut Scanner,
    context: WeightParseContext<'_>,
) -> Result<BoxedWfcb, Diagnostic> {
    scanner.check_tok(TokenType::IDENTIFIER)?;
    let name = scanner.current_token().literal();
    if get_weight_fun_parse_fun_index(&name).is_none() {
        return Err(weight_fun_error(
            scanner,
            &format!("Not a valid weight function specifier: {name}"),
        ));
    }
    if !weight_fun_parser_is_ported(&name) {
        return Err(weight_fun_error(
            scanner,
            &format!("Weight function parser is not ported yet: {name}"),
        ));
    }

    scanner.next_token()?;
    match name.as_str() {
        "Clauseweight" => Ok(Box::new(clause_weight_parse(scanner)?)),
        "ClauseLMaxWeight" => Ok(Box::new(lmax_weight_parse(scanner)?)),
        "ClauseCMaxWeight" => Ok(Box::new(cmax_weight_parse(scanner)?)),
        "Uniqweight" => Ok(Box::new(uniq_weight_parse(scanner)?)),
        "Defaultweight" => Ok(Box::new(default_weight_parse(scanner)?)),
        "DAGweight" => Ok(Box::new(dag_weight_parse(scanner)?)),
        "RDAGweight" => Ok(Box::new(rdag_weight_parse(scanner)?)),
        "RDAGweight2" => Ok(Box::new(rdag_weight2_parse(scanner)?)),
        "RDAGweight3" => Ok(Box::new(rdag_weight3_parse(scanner)?)),
        "Refinedweight" => Ok(Box::new(clause_refined_weight_parse(scanner)?)),
        "Refinedweight2" => Ok(Box::new(clause_refined_weight2_parse(scanner)?)),
        "Diversityweight" => Ok(Box::new(diversity_weight_parse(scanner)?)),
        "PNRefinedweight" => Ok(Box::new(pn_refined_weight_parse(scanner)?)),
        "TPTPTypeweight" => Ok(Box::new(tptp_type_weight_parse(scanner)?)),
        "Sigweight" => Ok(Box::new(sig_weight_parse(scanner)?)),
        "NLweight" => Ok(Box::new(nl_weight_parse(scanner)?)),
        "SymbolTypeweight" => Ok(Box::new(sym_type_weight_parse(scanner)?)),
        "Depthweight" => Ok(Box::new(depth_weight_parse(scanner)?)),
        "WLessDWeight" => Ok(Box::new(weight_less_depth_parse(scanner)?)),
        "Proofweight" => Ok(Box::new(proof_weight_parse(scanner)?)),
        "Orientweight" => Ok(Box::new(clause_orient_weight_parse(scanner)?)),
        "OrientLMaxWeight" => Ok(Box::new(orient_lmax_weight_parse(scanner)?)),
        "Simweight" => Ok(Box::new(sim_weight_parse(scanner)?)),
        "ClauseWeightAge" => Ok(Box::new(clause_weight_age_parse(scanner)?)),
        "StaggeredWeight" => {
            let axioms = context.require_axioms(scanner, &name)?;
            Ok(Box::new(staggered_weight_parse(scanner, axioms)?))
        }
        "GDWeight" => {
            let axioms = context.require_axioms(scanner, &name)?;
            Ok(Box::new(gd_clause_weight_parse(scanner, axioms)?))
        }
        "ConjectureSymbolWeight" => {
            let axioms = context.require_axioms(scanner, &name)?;
            Ok(Box::new(conjecture_simplified_symbol_weight_parse(
                scanner, axioms,
            )?))
        }
        "ConjectureGeneralSymbolWeight" => {
            let axioms = context.require_axioms(scanner, &name)?;
            Ok(Box::new(conjecture_symbol_weight_parse(scanner, axioms)?))
        }
        "ConjectureRelativeSymbolWeight" => {
            let axioms = context.require_axioms(scanner, &name)?;
            Ok(Box::new(conjecture_relative_symbol_weight_parse(
                scanner, axioms,
            )?))
        }
        "FunWeight" => Ok(Box::new(fun_weight_parse(scanner)?)),
        "SymOffsetWeight" => Ok(Box::new(sym_offset_weight_parse(scanner)?)),
        "RandomWeight" => Ok(Box::new(rand_weight_parse(scanner)?)),
        "FIFOWeight" => Ok(Box::new(fifo_eval_parse(scanner)?)),
        "LIFOWeight" => Ok(Box::new(lifo_eval_parse(scanner)?)),
        _ => unreachable!("ported weight parser should be handled"),
    }
}

fn weight_fun_error(scanner: &Scanner, message: &str) -> Diagnostic {
    Diagnostic::new(
        ErrorCode::SYNTAX_ERROR,
        format!(
            "{}(just read '{}'): {message}",
            token_pos_rep(scanner.current_token()),
            scanner.current_token().literal()
        ),
    )
}

#[cfg(test)]
mod tests {
    use super::{
        get_weight_fun_parse_fun_index, weight_fun_parse, weight_fun_parse_with_context,
        weight_fun_parser_is_ported, wfcb_admin_alloc, WeightParseContext, WfcbAdmin,
        WEIGHT_FUN_PARSE_FUN_NAMES,
    };
    use crate::clauses::clause::Clause;
    use crate::clauses::clausesets::ClauseSet;
    use crate::clauses::neweval::{evals_alloc, PRIO_NORMAL};
    use crate::heuristics::wfcb::{wfcb_alloc, BoxedWfcb};
    use crate::inout::scanner::Scanner;
    use crate::terms::signature::Signature;
    use crate::terms::termbanks::TermBank;
    use crate::terms::typebanks::TypeBank;
    use std::cell::Cell;
    use std::rc::Rc;

    #[derive(Debug)]
    struct TestData {
        weight: f64,
        exit_count: Rc<Cell<i32>>,
    }

    fn eval(data: Option<&mut TestData>, _bank: &TermBank, _clause: &Clause) -> f64 {
        data.map_or(0.0, |data| data.weight)
    }

    fn priority(_bank: &TermBank, _clause: &Clause) -> i64 {
        40
    }

    fn term_bank() -> TermBank {
        TermBank::new(Signature::new(TypeBank::new())).unwrap_or_else(|err| panic!("{err}"))
    }

    fn exit(data: TestData) {
        let TestData {
            weight: _,
            exit_count,
        } = data;
        exit_count.set(exit_count.get() + 1);
    }

    fn boxed_wfcb(weight: f64, exit_count: Rc<Cell<i32>>) -> BoxedWfcb {
        Box::new(wfcb_alloc(
            eval,
            priority,
            exit,
            Some(TestData { weight, exit_count }),
        ))
    }

    #[test]
    fn allocation_starts_empty_with_zero_anonymous_counter() {
        let admin = wfcb_admin_alloc();

        assert_eq!(admin.len(), 0);
        assert!(admin.is_empty());
        assert_eq!(admin.anon_counter(), 0);
        assert_eq!(admin.name(0), None);
    }

    #[test]
    fn add_wfcb_returns_index_and_find_returns_last_duplicate_name() {
        let exit_count = Rc::new(Cell::new(0));
        let mut admin = WfcbAdmin::new();
        let first = admin.add_wfcb("weight", boxed_wfcb(1.0, Rc::clone(&exit_count)));
        let second = admin.add_wfcb("other", boxed_wfcb(2.0, Rc::clone(&exit_count)));
        let third = admin.add_wfcb("weight", boxed_wfcb(3.0, Rc::clone(&exit_count)));
        let clause = Clause::empty();

        assert_eq!(first, 0);
        assert_eq!(second, 1);
        assert_eq!(third, 2);
        assert_eq!(admin.name(first), Some("weight"));
        assert_eq!(admin.name(second), Some("other"));
        assert_eq!(
            admin
                .find_wfcb_mut("weight")
                .expect("duplicate name should be found")
                .compute_eval(&term_bank(), &clause)
                .to_bits(),
            3.0_f64.to_bits()
        );
        assert!(admin.find_wfcb("missing").is_none());
    }

    #[test]
    fn dropping_admin_frees_all_stored_wfcbs() {
        let exit_count = Rc::new(Cell::new(0));
        let mut admin = WfcbAdmin::new();
        admin.add_wfcb("a", boxed_wfcb(1.0, Rc::clone(&exit_count)));
        admin.add_wfcb("b", boxed_wfcb(2.0, Rc::clone(&exit_count)));

        drop(admin);

        assert_eq!(exit_count.get(), 2);
    }

    #[test]
    fn anonymous_names_match_c_sprintf_shape() {
        let mut admin = WfcbAdmin::new();

        assert_eq!(admin.next_anonymous_name(), "~$000000000");
        assert_eq!(admin.next_anonymous_name(), "~$000000001");
        assert_eq!(admin.anon_counter(), 2);
    }

    #[test]
    fn parse_function_names_preserve_c_order_and_lookup() {
        assert_eq!(WEIGHT_FUN_PARSE_FUN_NAMES.len(), 46);
        assert_eq!(get_weight_fun_parse_fun_index("Clauseweight"), Some(0));
        assert_eq!(get_weight_fun_parse_fun_index("FIFOWeight"), Some(24));
        assert_eq!(
            get_weight_fun_parse_fun_index("ConjectureSymbolWeight"),
            Some(30)
        );
        assert_eq!(get_weight_fun_parse_fun_index("GDWeight"), Some(45));
        assert_eq!(get_weight_fun_parse_fun_index("NoSuchWeight"), None);
        assert!(weight_fun_parser_is_ported("FIFOWeight"));
        assert!(weight_fun_parser_is_ported("LIFOWeight"));
        assert!(weight_fun_parser_is_ported("RandomWeight"));
        assert!(weight_fun_parser_is_ported("Clauseweight"));
        assert!(weight_fun_parser_is_ported("ClauseLMaxWeight"));
        assert!(weight_fun_parser_is_ported("ClauseCMaxWeight"));
        assert!(weight_fun_parser_is_ported("Uniqweight"));
        assert!(weight_fun_parser_is_ported("Defaultweight"));
        assert!(weight_fun_parser_is_ported("DAGweight"));
        assert!(weight_fun_parser_is_ported("RDAGweight"));
        assert!(weight_fun_parser_is_ported("RDAGweight2"));
        assert!(weight_fun_parser_is_ported("RDAGweight3"));
        assert!(weight_fun_parser_is_ported("Refinedweight"));
        assert!(weight_fun_parser_is_ported("Refinedweight2"));
        assert!(weight_fun_parser_is_ported("Diversityweight"));
        assert!(weight_fun_parser_is_ported("PNRefinedweight"));
        assert!(weight_fun_parser_is_ported("TPTPTypeweight"));
        assert!(weight_fun_parser_is_ported("Sigweight"));
        assert!(weight_fun_parser_is_ported("NLweight"));
        assert!(weight_fun_parser_is_ported("SymbolTypeweight"));
        assert!(weight_fun_parser_is_ported("Depthweight"));
        assert!(weight_fun_parser_is_ported("WLessDWeight"));
        assert!(weight_fun_parser_is_ported("Proofweight"));
        assert!(weight_fun_parser_is_ported("Orientweight"));
        assert!(weight_fun_parser_is_ported("OrientLMaxWeight"));
        assert!(weight_fun_parser_is_ported("Simweight"));
        assert!(weight_fun_parser_is_ported("ClauseWeightAge"));
        assert!(weight_fun_parser_is_ported("StaggeredWeight"));
        assert!(weight_fun_parser_is_ported("GDWeight"));
        assert!(weight_fun_parser_is_ported("ConjectureSymbolWeight"));
        assert!(weight_fun_parser_is_ported("ConjectureGeneralSymbolWeight"));
        assert!(weight_fun_parser_is_ported(
            "ConjectureRelativeSymbolWeight"
        ));
        assert!(weight_fun_parser_is_ported("FunWeight"));
        assert!(weight_fun_parser_is_ported("SymOffsetWeight"));
        assert!(!weight_fun_parser_is_ported(
            "ConjectureRelativeTypeSymbolWeight"
        ));
        assert!(!weight_fun_parser_is_ported("TSMWeight"));
    }

    #[test]
    fn weight_fun_parse_dispatches_ported_fifo_and_lifo_parsers() {
        let clause = Clause::empty();
        let bank = term_bank();
        let mut fifo_scanner = Scanner::from_user_string("FIFOWeight(ConstPrio) tail", false)
            .unwrap_or_else(|err| {
                panic!("{err}");
            });
        let mut lifo_scanner = Scanner::from_user_string("LIFOWeight(ConstPrio) tail", false)
            .unwrap_or_else(|err| {
                panic!("{err}");
            });
        let mut fifo = weight_fun_parse(&mut fifo_scanner).unwrap_or_else(|err| panic!("{err}"));
        let mut lifo = weight_fun_parse(&mut lifo_scanner).unwrap_or_else(|err| panic!("{err}"));
        let mut fifo_eval = evals_alloc(1);
        let mut lifo_eval = evals_alloc(1);

        fifo.add_evaluation(&mut fifo_eval, &bank, &clause, 0, false);
        lifo.add_evaluation(&mut lifo_eval, &bank, &clause, 0, false);

        assert_eq!(fifo_eval.eval(0).heuristic().to_bits(), 1.0_f32.to_bits());
        assert_eq!(
            lifo_eval.eval(0).heuristic().to_bits(),
            (-1.0_f32).to_bits()
        );
        assert_eq!(fifo_eval.eval(0).priority(), PRIO_NORMAL);
        assert_eq!(lifo_eval.eval(0).priority(), PRIO_NORMAL);
        assert_eq!(fifo_scanner.current_token().literal(), "tail");
        assert_eq!(lifo_scanner.current_token().literal(), "tail");
    }

    #[test]
    fn weight_fun_parse_dispatches_random_weight_parser() {
        let clause = Clause::empty();
        let bank = term_bank();
        let mut scanner =
            Scanner::from_user_string("RandomWeight(ConstPrio,0,10.0,2.0) tail", false)
                .unwrap_or_else(|err| panic!("{err}"));
        let mut wfcb = weight_fun_parse(&mut scanner).unwrap_or_else(|err| panic!("{err}"));
        let mut evaluations = evals_alloc(1);

        wfcb.add_evaluation(&mut evaluations, &bank, &clause, 0, false);

        assert_eq!(evaluations.eval(0).priority(), PRIO_NORMAL);
        assert_eq!(scanner.current_token().literal(), "tail");
    }

    #[test]
    fn weight_fun_parse_dispatches_clause_weight_family_parsers() {
        let clause = Clause::empty();
        let bank = term_bank();
        let specs = [
            "Clauseweight(ConstPrio,2,1,3.0) tail",
            "ClauseLMaxWeight(ConstPrio,2,1,3.0) tail",
            "ClauseCMaxWeight(ConstPrio,2,1,3.0) tail",
            "Uniqweight(ConstPrio) tail",
            "Defaultweight(ConstPrio) tail",
        ];

        for spec in specs {
            let mut scanner =
                Scanner::from_user_string(spec, false).unwrap_or_else(|err| panic!("{err}"));
            let mut wfcb = weight_fun_parse(&mut scanner).unwrap_or_else(|err| panic!("{err}"));
            let mut evaluations = evals_alloc(1);

            wfcb.add_evaluation(&mut evaluations, &bank, &clause, 0, false);

            assert_eq!(evaluations.eval(0).priority(), PRIO_NORMAL);
            assert_eq!(scanner.current_token().literal(), "tail");
        }
    }

    #[test]
    fn weight_fun_parse_dispatches_dag_weight_family_parsers() {
        let clause = Clause::empty();
        let bank = term_bank();
        let specs = [
            "DAGweight(ConstPrio,2,1,3.0,1,true,false,false,true,false,false,false) tail",
            "RDAGweight(ConstPrio,10,3,1,5.0,2.0,7.0,4.0) tail",
            "RDAGweight2(ConstPrio,10,3,1,4.0,2.0) tail",
            "RDAGweight3(ConstPrio,2,1,13,17,1,3.0,5.0,7.0,11.0) tail",
        ];

        for spec in specs {
            let mut scanner =
                Scanner::from_user_string(spec, false).unwrap_or_else(|err| panic!("{err}"));
            let mut wfcb = weight_fun_parse(&mut scanner).unwrap_or_else(|err| panic!("{err}"));
            let mut evaluations = evals_alloc(1);

            wfcb.add_evaluation(&mut evaluations, &bank, &clause, 0, false);

            assert_eq!(evaluations.eval(0).priority(), PRIO_NORMAL);
            assert_eq!(scanner.current_token().literal(), "tail");
        }
    }

    #[test]
    fn weight_fun_parse_dispatches_refined_weight_family_parsers() {
        let clause = Clause::empty();
        let bank = term_bank();
        let specs = [
            "Refinedweight(ConstPrio,2,1,7.0,5.0,3.0) tail",
            "Refinedweight2(ConstPrio,2,1,7.0,5.0,3.0) tail",
            "Diversityweight(ConstPrio,2,3,1.0,1.0,1.0,10.0,1.0,20.0,2.0) tail",
            "PNRefinedweight(ConstPrio,2,1,13,17,1.0,1.0,1.0) tail",
        ];

        for spec in specs {
            let mut scanner =
                Scanner::from_user_string(spec, false).unwrap_or_else(|err| panic!("{err}"));
            let mut wfcb = weight_fun_parse(&mut scanner).unwrap_or_else(|err| panic!("{err}"));
            let mut evaluations = evals_alloc(1);

            wfcb.add_evaluation(&mut evaluations, &bank, &clause, 0, false);

            assert_eq!(evaluations.eval(0).priority(), PRIO_NORMAL);
            assert_eq!(scanner.current_token().literal(), "tail");
        }
    }

    #[test]
    fn weight_fun_parse_dispatches_var_weight_family_parsers() {
        let clause = Clause::empty();
        let bank = term_bank();
        let specs = [
            "TPTPTypeweight(ConstPrio,2,1,1.0,1.0,1.0,7.0,5.0) tail",
            "Sigweight(ConstPrio,2,1,1.0,1.0,1.0,3.0) tail",
            "NLweight(ConstPrio,2,7,1,1.0,1.0,1.0) tail",
            "SymbolTypeweight(ConstPrio,2,1,3,11,1.0,1.0,1.0) tail",
            "Depthweight(ConstPrio,2,1,3.0,1.0,7.0,11.0) tail",
            "WLessDWeight(ConstPrio,2,1,3.0,1.0,7.0,0.5) tail",
            "Proofweight(ConstPrio,2,1,1.0,1.0,1.0,8.0,6.0) tail",
            "ClauseWeightAge(ConstPrio,2,1,1.0,4.0) tail",
        ];

        for spec in specs {
            let mut scanner =
                Scanner::from_user_string(spec, false).unwrap_or_else(|err| panic!("{err}"));
            let mut wfcb = weight_fun_parse(&mut scanner).unwrap_or_else(|err| panic!("{err}"));
            let mut evaluations = evals_alloc(1);

            wfcb.add_evaluation(&mut evaluations, &bank, &clause, 0, false);

            assert_eq!(evaluations.eval(0).priority(), PRIO_NORMAL);
            assert_eq!(scanner.current_token().literal(), "tail");
        }
    }

    #[test]
    fn weight_fun_parse_dispatches_orient_and_sim_weight_parsers() {
        let clause = Clause::empty();
        let bank = term_bank();
        let specs = [
            "Orientweight(ConstPrio,2,1,7.0,5.0,3.0) tail",
            "OrientLMaxWeight(ConstPrio,2,1,7.0,5.0,3.0) tail",
            "Simweight(ConstPrio,100.0,3.0,5.0,7.0) tail",
        ];

        for spec in specs {
            let mut scanner =
                Scanner::from_user_string(spec, false).unwrap_or_else(|err| panic!("{err}"));
            let mut wfcb = weight_fun_parse(&mut scanner).unwrap_or_else(|err| panic!("{err}"));
            let mut evaluations = evals_alloc(1);

            wfcb.add_evaluation(&mut evaluations, &bank, &clause, 0, false);

            assert_eq!(evaluations.eval(0).priority(), PRIO_NORMAL);
            assert_eq!(scanner.current_token().literal(), "tail");
        }
    }

    #[test]
    fn weight_fun_parse_with_context_dispatches_axiom_backed_parsers() {
        let clause = Clause::empty();
        let bank = term_bank();
        let axioms = ClauseSet::new();
        let context = WeightParseContext::new(&axioms);
        let specs = [
            "StaggeredWeight(ConstPrio,1.0) tail",
            "GDWeight(ConstPrio,2,1,1.0,0.0,5) tail",
            "ConjectureSymbolWeight(ConstPrio,10,99,1,88,1,1.0,1.0,1.0) tail",
            "ConjectureGeneralSymbolWeight(ConstPrio,10,3,99,1,2,88,1,1.0,1.0,1.0) tail",
            "ConjectureRelativeSymbolWeight(ConstPrio,0.5,10,4,99,1,1.0,1.0,1.0) tail",
        ];

        assert_eq!(context.axioms().map(ClauseSet::len), Some(0));
        for spec in specs {
            let mut scanner =
                Scanner::from_user_string(spec, false).unwrap_or_else(|err| panic!("{err}"));
            let mut wfcb = weight_fun_parse_with_context(&mut scanner, context)
                .unwrap_or_else(|err| panic!("{err}"));
            let mut evaluations = evals_alloc(1);

            wfcb.add_evaluation(&mut evaluations, &bank, &clause, 0, false);

            assert_eq!(evaluations.eval(0).priority(), PRIO_NORMAL);
            assert_eq!(scanner.current_token().literal(), "tail");
        }
    }

    #[test]
    fn weight_fun_parse_dispatches_fun_weight_family_parsers() {
        let clause = Clause::empty();
        let bank = term_bank();
        let specs = [
            "FunWeight(ConstPrio,2,1,1.0,1.0,1.0) tail",
            "SymOffsetWeight(ConstPrio,2,1,1.0,1.0,1.0) tail",
        ];

        for spec in specs {
            let mut scanner =
                Scanner::from_user_string(spec, false).unwrap_or_else(|err| panic!("{err}"));
            let mut wfcb = weight_fun_parse(&mut scanner).unwrap_or_else(|err| panic!("{err}"));
            let mut evaluations = evals_alloc(1);

            wfcb.add_evaluation(&mut evaluations, &bank, &clause, 0, false);

            assert_eq!(evaluations.eval(0).priority(), PRIO_NORMAL);
            assert_eq!(scanner.current_token().literal(), "tail");
        }
    }

    #[test]
    fn weight_fun_parse_rejects_unknown_or_unported_names() {
        let mut unknown = Scanner::from_user_string("NoSuchWeight(ConstPrio)", false)
            .unwrap_or_else(|err| {
                panic!("{err}");
            });
        let Err(err) = weight_fun_parse(&mut unknown) else {
            panic!("unknown weight function should fail");
        };
        assert!(err.to_string().contains("Not a valid weight function"));

        let mut no_context = Scanner::from_user_string("StaggeredWeight(ConstPrio,1)", false)
            .unwrap_or_else(|err| {
                panic!("{err}");
            });
        let Err(err) = weight_fun_parse(&mut no_context) else {
            panic!("context-backed weight function should fail without axioms");
        };
        assert!(err.to_string().contains("requires proof-state axioms"));

        let mut conjecture_without_context =
            Scanner::from_user_string("ConjectureSymbolWeight(ConstPrio,2,1,1,1,1,1,1,1)", false)
                .unwrap_or_else(|err| {
                    panic!("{err}");
                });
        let Err(err) = weight_fun_parse(&mut conjecture_without_context) else {
            panic!("conjecture symbol weight should fail without axioms");
        };
        assert!(err.to_string().contains("requires proof-state axioms"));

        let mut unported =
            Scanner::from_user_string("TSMWeight(ConstPrio)", false).unwrap_or_else(|err| {
                panic!("{err}");
            });
        let Err(err) = weight_fun_parse(&mut unported) else {
            panic!("unported weight function should fail");
        };
        assert!(err.to_string().contains("not ported yet"));
    }

    #[test]
    fn weight_fun_def_parse_adds_named_and_anonymous_definitions() {
        let mut admin = WfcbAdmin::new();
        let mut scanner = Scanner::from_user_string(
            "fresh=FIFOWeight(ConstPrio) RandomWeight(ConstPrio,0,0,1) LIFOWeight(ConstPrio) done",
            false,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        let parsed = admin
            .weight_fun_def_list_parse(&mut scanner)
            .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(parsed, 3);
        assert_eq!(admin.len(), 3);
        assert_eq!(admin.name(0), Some("fresh"));
        assert_eq!(admin.name(1), Some("~$000000000"));
        assert_eq!(admin.name(2), Some("~$000000001"));
        assert_eq!(admin.anon_counter(), 2);
        assert!(admin.find_wfcb("fresh").is_some());
        assert!(admin.find_wfcb("~$000000000").is_some());
        assert!(admin.find_wfcb("~$000000001").is_some());
        assert_eq!(scanner.current_token().literal(), "done");
    }
}
