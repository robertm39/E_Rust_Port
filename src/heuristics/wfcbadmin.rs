use crate::heuristics::wfcb::BoxedWfcb;

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

#[cfg(test)]
mod tests {
    use super::{
        get_weight_fun_parse_fun_index, wfcb_admin_alloc, WfcbAdmin, WEIGHT_FUN_PARSE_FUN_NAMES,
    };
    use crate::clauses::clause::Clause;
    use crate::heuristics::wfcb::{wfcb_alloc, BoxedWfcb};
    use crate::terms::termbanks::TermBank;
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
    }
}
