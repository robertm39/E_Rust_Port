use crate::basics::error::{Diagnostic, ErrorCode};
use crate::heuristics::fifo::fifo_eval_parse;
use crate::heuristics::lifo::lifo_eval_parse;
use crate::heuristics::random::rand_weight_parse;
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

        let wfcb = weight_fun_parse(scanner)?;
        let stored_name = name.clone();
        self.add_wfcb(name, wfcb);
        Ok(stored_name)
    }

    pub fn weight_fun_def_list_parse(
        &mut self,
        scanner: &mut Scanner,
    ) -> Result<usize, Diagnostic> {
        let mut parsed = 0;
        while scanner.test_tok(TokenType::IDENTIFIER)
            && scanner
                .look_token(1)
                .kind()
                .intersects(TokenType::EQUAL_SIGN | TokenType::OPEN_BRACKET)
        {
            self.weight_fun_def_parse(scanner)?;
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
    matches!(name, "RandomWeight" | "FIFOWeight" | "LIFOWeight")
}

pub fn weight_fun_parse(scanner: &mut Scanner) -> Result<BoxedWfcb, Diagnostic> {
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
        get_weight_fun_parse_fun_index, weight_fun_parse, weight_fun_parser_is_ported,
        wfcb_admin_alloc, WfcbAdmin, WEIGHT_FUN_PARSE_FUN_NAMES,
    };
    use crate::clauses::clause::Clause;
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

    fn eval(data: Option<&mut TestData>, _clause: &Clause) -> f64 {
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
                .compute_eval(&clause)
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
        assert!(!weight_fun_parser_is_ported("Clauseweight"));
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
    fn weight_fun_parse_rejects_unknown_or_unported_names() {
        let mut unknown = Scanner::from_user_string("NoSuchWeight(ConstPrio)", false)
            .unwrap_or_else(|err| {
                panic!("{err}");
            });
        let Err(err) = weight_fun_parse(&mut unknown) else {
            panic!("unknown weight function should fail");
        };
        assert!(err.to_string().contains("Not a valid weight function"));

        let mut unported = Scanner::from_user_string("Clauseweight(ConstPrio)", false)
            .unwrap_or_else(|err| {
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
