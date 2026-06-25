use crate::clauses::clause::Clause;
use crate::clauses::clause_props::CP_IS_GLOBAL_INDEXED;
use crate::clauses::clausesets::ClauseSet;
use crate::clauses::subterm_index::SubtermIndex;
use crate::clauses::subterm_tree::SubtermOcc;
use crate::terms::idx_fp::get_fp_index_function;
use crate::terms::signature::Signature;
use crate::terms::termtypes::Term;

pub struct GlobalIndices<'sig> {
    signature: Option<&'sig Signature>,
    rw_bw_index_type: String,
    pm_from_index_type: String,
    pm_into_index_type: String,
    pm_negp_index_type: String,
    bw_rw_index: Option<SubtermIndex<'sig>>,
    ext_rules_max_depth: i32,
}

impl Default for GlobalIndices<'_> {
    fn default() -> Self {
        Self::null()
    }
}

impl<'sig> GlobalIndices<'sig> {
    #[must_use]
    pub fn null() -> Self {
        Self {
            signature: None,
            rw_bw_index_type: String::new(),
            pm_from_index_type: String::new(),
            pm_into_index_type: String::new(),
            pm_negp_index_type: String::new(),
            bw_rw_index: None,
            ext_rules_max_depth: 0,
        }
    }

    #[must_use]
    pub fn new(
        signature: &'sig Signature,
        rw_bw_index_type: &str,
        pm_from_index_type: &str,
        pm_into_index_type: &str,
        ext_rules_max_depth: i32,
    ) -> Self {
        let mut indices = Self::null();
        indices.init(
            signature,
            rw_bw_index_type,
            pm_from_index_type,
            pm_into_index_type,
            ext_rules_max_depth,
        );
        indices
    }

    pub fn init(
        &mut self,
        signature: &'sig Signature,
        rw_bw_index_type: &str,
        pm_from_index_type: &str,
        pm_into_index_type: &str,
        ext_rules_max_depth: i32,
    ) {
        self.free_indices();
        self.signature = Some(signature);
        rw_bw_index_type.clone_into(&mut self.rw_bw_index_type);
        pm_from_index_type.clone_into(&mut self.pm_from_index_type);
        pm_into_index_type.clone_into(&mut self.pm_into_index_type);
        pm_into_index_type.clone_into(&mut self.pm_negp_index_type);
        self.ext_rules_max_depth = ext_rules_max_depth;

        self.bw_rw_index = get_fp_index_function(rw_bw_index_type)
            .map(|fp_fun| SubtermIndex::new(fp_fun, signature));
    }

    pub fn free_indices(&mut self) {
        self.bw_rw_index = None;
    }

    pub fn reset(&mut self) {
        let Some(signature) = self.signature else {
            self.free_indices();
            return;
        };
        let rw_bw_index_type = self.rw_bw_index_type.clone();
        let pm_from_index_type = self.pm_from_index_type.clone();
        let pm_into_index_type = self.pm_into_index_type.clone();
        let ext_rules_max_depth = self.ext_rules_max_depth;
        self.init(
            signature,
            &rw_bw_index_type,
            &pm_from_index_type,
            &pm_into_index_type,
            ext_rules_max_depth,
        );
    }

    #[must_use]
    pub const fn rw_bw_index_type(&self) -> &str {
        self.rw_bw_index_type.as_str()
    }

    #[must_use]
    pub const fn pm_from_index_type(&self) -> &str {
        self.pm_from_index_type.as_str()
    }

    #[must_use]
    pub const fn pm_into_index_type(&self) -> &str {
        self.pm_into_index_type.as_str()
    }

    #[must_use]
    pub const fn pm_negp_index_type(&self) -> &str {
        self.pm_negp_index_type.as_str()
    }

    #[must_use]
    pub const fn ext_rules_max_depth(&self) -> i32 {
        self.ext_rules_max_depth
    }

    #[must_use]
    pub const fn has_bw_rw_index(&self) -> bool {
        self.bw_rw_index.is_some()
    }

    #[must_use]
    pub fn find_bw_rw_occurrence(&self, term: &Term) -> Option<&SubtermOcc> {
        self.bw_rw_index
            .as_ref()
            .and_then(|index| index.find_occurrence(term))
    }

    /// # Panics
    ///
    /// Panics if `clause` is already marked as globally indexed.
    pub fn insert_clause(&mut self, clause: &mut Clause, lambda_demod: bool) {
        assert!(
            !clause.query_prop(CP_IS_GLOBAL_INDEXED),
            "global index insert expects an unindexed clause"
        );
        clause.set_prop(CP_IS_GLOBAL_INDEXED);
        if let Some(index) = self.bw_rw_index.as_mut() {
            index.insert_clause(clause, lambda_demod);
        }
    }

    /// # Panics
    ///
    /// Panics if `clause` is not marked as globally indexed.
    pub fn delete_clause(&mut self, clause: &mut Clause, lambda_demod: bool) {
        assert!(
            clause.query_prop(CP_IS_GLOBAL_INDEXED),
            "global index delete expects an indexed clause"
        );
        clause.del_prop(CP_IS_GLOBAL_INDEXED);
        if let Some(index) = self.bw_rw_index.as_mut() {
            index.delete_clause(clause, lambda_demod);
        }
    }

    pub fn insert_clause_set(&mut self, set: &mut ClauseSet, lambda_demod: bool) -> i64 {
        if self.bw_rw_index.is_none() {
            return 0;
        }
        let mut inserted = 0;
        for clause in set.iter_mut() {
            self.insert_clause(clause, lambda_demod);
            inserted += 1;
        }
        inserted
    }
}

#[must_use]
pub fn global_indices_null<'sig>() -> GlobalIndices<'sig> {
    GlobalIndices::null()
}

#[cfg(test)]
mod tests {
    use super::{global_indices_null, GlobalIndices};
    use crate::clauses::clause::Clause;
    use crate::clauses::clause_props::CP_IS_GLOBAL_INDEXED;
    use crate::clauses::clausesets::ClauseSet;
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
        if bank.signature().get_type(f_code).is_none() {
            bank.signature_mut()
                .declare_final_type(f_code, type_)
                .unwrap();
        }
        bank.create_const_term(f_code).unwrap()
    }

    fn typed_unary(bank: &mut TermBank, name: &str, arg: &Term) -> Term {
        let type_ = bank.signature().type_bank().default_type();
        let f_code = bank.signature_mut().insert_id(name, 1, false);
        if bank.signature().get_type(f_code).is_none() {
            bank.signature_mut()
                .declare_final_type(
                    f_code,
                    crate::terms::simpletypes::alloc_arrow_type(vec![type_.clone(), type_]),
                )
                .unwrap();
        }
        let term = Term::top_alloc(f_code, 1);
        term.set_type(Some(bank.signature().type_bank().default_type()));
        term.set_argument(0, arg.clone());
        bank.insert(&term, DerefType::Never).unwrap()
    }

    fn unit_clause(bank: &mut TermBank, name: &str, ident: i64) -> (Clause, Term) {
        let left = typed_const(bank, name);
        let right_arg = typed_const(bank, &format!("{name}_arg"));
        let right = typed_unary(bank, &format!("{name}_f"), &right_arg);
        let literal = Eqn::alloc(left.clone(), right, bank, true).unwrap();
        let mut clause = Clause::alloc(EqnList::from_vec(vec![literal]));
        clause.set_ident(ident);
        (clause, left)
    }

    #[test]
    fn null_indices_start_without_configured_indexes() {
        let indices = global_indices_null();

        assert!(!indices.has_bw_rw_index());
        assert_eq!(indices.rw_bw_index_type(), "");
        assert_eq!(indices.ext_rules_max_depth(), 0);
    }

    #[test]
    fn init_stores_names_and_allocates_only_known_backward_index() {
        let bank = test_bank();
        let indices = GlobalIndices::new(bank.signature(), "FP1", "NoIndex", "FP7", -1);

        assert!(indices.has_bw_rw_index());
        assert_eq!(indices.rw_bw_index_type(), "FP1");
        assert_eq!(indices.pm_from_index_type(), "NoIndex");
        assert_eq!(indices.pm_into_index_type(), "FP7");
        assert_eq!(indices.pm_negp_index_type(), "FP7");
        assert_eq!(indices.ext_rules_max_depth(), -1);
    }

    #[test]
    fn insert_clause_sets_global_prop_and_populates_backward_index() {
        let mut bank = test_bank();
        let (mut clause, left) = unit_clause(&mut bank, "gidx_clause", 10);
        let mut indices = GlobalIndices::new(bank.signature(), "FP1", "NoIndex", "NoIndex", 0);

        indices.insert_clause(&mut clause, false);

        assert!(clause.query_prop(CP_IS_GLOBAL_INDEXED));
        assert!(indices.find_bw_rw_occurrence(&left).is_some());

        indices.delete_clause(&mut clause, false);

        assert!(!clause.query_prop(CP_IS_GLOBAL_INDEXED));
        assert!(indices.find_bw_rw_occurrence(&left).is_none());
    }

    #[test]
    fn insert_clause_set_is_noop_without_backward_index() {
        let mut bank = test_bank();
        let (clause, _) = unit_clause(&mut bank, "gidx_noindex", 20);
        let mut set = ClauseSet::new();
        set.insert(clause);
        let mut indices = GlobalIndices::new(bank.signature(), "NoIndex", "NoIndex", "NoIndex", 0);

        assert_eq!(indices.insert_clause_set(&mut set, false), 0);
        assert!(set
            .iter()
            .all(|clause| !clause.query_prop(CP_IS_GLOBAL_INDEXED)));
    }

    #[test]
    fn insert_clause_set_marks_all_clauses_when_backward_index_exists() {
        let mut bank = test_bank();
        let (first, left) = unit_clause(&mut bank, "gidx_set_first", 30);
        let (second, _) = unit_clause(&mut bank, "gidx_set_second", 31);
        let mut set = ClauseSet::new();
        set.insert(first);
        set.insert(second);
        let mut indices = GlobalIndices::new(bank.signature(), "FP1", "NoIndex", "NoIndex", 0);

        assert_eq!(indices.insert_clause_set(&mut set, false), 2);

        assert!(set
            .iter()
            .all(|clause| clause.query_prop(CP_IS_GLOBAL_INDEXED)));
        assert!(indices.find_bw_rw_occurrence(&left).is_some());
    }

    #[test]
    fn reset_rebuilds_configured_backward_index_empty() {
        let mut bank = test_bank();
        let (mut clause, left) = unit_clause(&mut bank, "gidx_reset", 40);
        let mut indices = GlobalIndices::new(bank.signature(), "FP1", "NoIndex", "NoIndex", 2);
        indices.insert_clause(&mut clause, false);

        indices.reset();

        assert!(indices.has_bw_rw_index());
        assert_eq!(indices.rw_bw_index_type(), "FP1");
        assert_eq!(indices.ext_rules_max_depth(), 2);
        assert!(indices.find_bw_rw_occurrence(&left).is_none());
    }
}
