#[cfg(feature = "print-index-stats")]
use crate::basics::defines::DEFAULT_COMCHAR_RAW;
use crate::basics::simple_stuff::ProblemType;
use crate::clauses::clause::Clause;
use crate::clauses::clause_props::CP_IS_GLOBAL_INDEXED;
use crate::clauses::clausepos_tree::ClauseTPosTree;
use crate::clauses::clausesets::ClauseSet;
use crate::clauses::ext_index::ExtIndex;
use crate::clauses::overlap_index::{
    overlap_index_delete_into_clause2, overlap_index_insert_into_clause2, OverlapIndex,
};
use crate::clauses::subterm_index::SubtermIndex;
#[cfg(feature = "print-index-stats")]
use crate::clauses::subterm_tree::subterm_occurrences_dot_record_string;
use crate::clauses::subterm_tree::SubtermOcc;
use crate::terms::idx_fp::get_fp_index_function;
use crate::terms::signature::Signature;
use crate::terms::termbanks::TermBank;
use crate::terms::termtypes::Term;
#[cfg(feature = "print-index-stats")]
use std::fmt::{self, Write as FmtWrite};
#[cfg(feature = "print-index-stats")]
use std::io::{self, Write as IoWrite};

pub struct GlobalIndices<'sig> {
    signature: Option<&'sig Signature>,
    rw_bw_index_type: String,
    pm_from_index_type: String,
    pm_into_index_type: String,
    pm_negp_index_type: String,
    bw_rw_index: Option<SubtermIndex<'sig>>,
    pm_from_index: Option<OverlapIndex<'sig>>,
    pm_into_index: Option<OverlapIndex<'sig>>,
    pm_negp_index: Option<OverlapIndex<'sig>>,
    ext_sup_into_index: Option<ExtIndex>,
    ext_sup_from_index: Option<ExtIndex>,
    ext_rules_max_depth: i32,
    problem_type: ProblemType,
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
            pm_from_index: None,
            pm_into_index: None,
            pm_negp_index: None,
            ext_sup_into_index: None,
            ext_sup_from_index: None,
            ext_rules_max_depth: 0,
            problem_type: ProblemType::NotInitialized,
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
        Self::new_for_problem(
            signature,
            rw_bw_index_type,
            pm_from_index_type,
            pm_into_index_type,
            ext_rules_max_depth,
            ProblemType::FirstOrder,
        )
    }

    #[must_use]
    pub fn new_for_problem(
        signature: &'sig Signature,
        rw_bw_index_type: &str,
        pm_from_index_type: &str,
        pm_into_index_type: &str,
        ext_rules_max_depth: i32,
        problem_type: ProblemType,
    ) -> Self {
        let mut indices = Self::null();
        indices.init_for_problem(
            signature,
            rw_bw_index_type,
            pm_from_index_type,
            pm_into_index_type,
            ext_rules_max_depth,
            problem_type,
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
        self.init_for_problem(
            signature,
            rw_bw_index_type,
            pm_from_index_type,
            pm_into_index_type,
            ext_rules_max_depth,
            ProblemType::FirstOrder,
        );
    }

    pub fn init_for_problem(
        &mut self,
        signature: &'sig Signature,
        rw_bw_index_type: &str,
        pm_from_index_type: &str,
        pm_into_index_type: &str,
        ext_rules_max_depth: i32,
        problem_type: ProblemType,
    ) {
        self.free_indices();
        self.signature = Some(signature);
        rw_bw_index_type.clone_into(&mut self.rw_bw_index_type);
        pm_from_index_type.clone_into(&mut self.pm_from_index_type);
        pm_into_index_type.clone_into(&mut self.pm_into_index_type);
        pm_into_index_type.clone_into(&mut self.pm_negp_index_type);
        self.ext_rules_max_depth = ext_rules_max_depth;
        self.problem_type = problem_type;

        self.bw_rw_index = get_fp_index_function(rw_bw_index_type)
            .map(|fp_fun| SubtermIndex::new(fp_fun, signature));
        self.pm_from_index = get_fp_index_function(pm_from_index_type)
            .map(|fp_fun| OverlapIndex::new(fp_fun, signature));
        self.pm_into_index = get_fp_index_function(pm_into_index_type)
            .map(|fp_fun| OverlapIndex::new(fp_fun, signature));
        self.pm_negp_index = get_fp_index_function(pm_into_index_type)
            .map(|fp_fun| OverlapIndex::new(fp_fun, signature));
        if problem_type == ProblemType::HigherOrder {
            self.ext_sup_into_index = Some(ExtIndex::new());
            self.ext_sup_from_index = Some(ExtIndex::new());
        }
    }

    pub fn free_indices(&mut self) {
        self.bw_rw_index = None;
        self.pm_from_index = None;
        self.pm_into_index = None;
        self.pm_negp_index = None;
        self.ext_sup_into_index = None;
        self.ext_sup_from_index = None;
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
        let problem_type = self.problem_type;
        self.init_for_problem(
            signature,
            &rw_bw_index_type,
            &pm_from_index_type,
            &pm_into_index_type,
            ext_rules_max_depth,
            problem_type,
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
    pub const fn problem_type(&self) -> ProblemType {
        self.problem_type
    }

    #[must_use]
    pub const fn has_bw_rw_index(&self) -> bool {
        self.bw_rw_index.is_some()
    }

    #[must_use]
    pub const fn bw_rw_index(&self) -> Option<&SubtermIndex<'sig>> {
        self.bw_rw_index.as_ref()
    }

    #[must_use]
    pub const fn has_pm_from_index(&self) -> bool {
        self.pm_from_index.is_some()
    }

    #[must_use]
    pub const fn has_pm_into_index(&self) -> bool {
        self.pm_into_index.is_some()
    }

    #[must_use]
    pub const fn has_pm_negp_index(&self) -> bool {
        self.pm_negp_index.is_some()
    }

    #[must_use]
    pub const fn has_ext_into_index(&self) -> bool {
        self.ext_sup_into_index.is_some()
    }

    #[must_use]
    pub const fn has_ext_from_index(&self) -> bool {
        self.ext_sup_from_index.is_some()
    }

    #[must_use]
    pub fn find_bw_rw_occurrence(&self, term: &Term) -> Option<&SubtermOcc> {
        self.bw_rw_index
            .as_ref()
            .and_then(|index| index.find_occurrence(term))
    }

    #[must_use]
    pub fn find_pm_from_occurrence(&self, term: &Term) -> Option<&SubtermOcc> {
        self.pm_from_index
            .as_ref()
            .and_then(|index| index.find_occurrence(term))
    }

    #[must_use]
    pub fn find_pm_into_occurrence(&self, term: &Term) -> Option<&SubtermOcc> {
        self.pm_into_index
            .as_ref()
            .and_then(|index| index.find_occurrence(term))
    }

    #[must_use]
    pub fn find_pm_negp_occurrence(&self, term: &Term) -> Option<&SubtermOcc> {
        self.pm_negp_index
            .as_ref()
            .and_then(|index| index.find_occurrence(term))
    }

    #[must_use]
    pub fn find_ext_into_symbol(&self, f_code: i64) -> Option<&ClauseTPosTree> {
        self.ext_sup_into_index
            .as_ref()
            .and_then(|index| index.find(f_code))
    }

    #[must_use]
    pub fn find_ext_from_symbol(&self, f_code: i64) -> Option<&ClauseTPosTree> {
        self.ext_sup_from_index
            .as_ref()
            .and_then(|index| index.find(f_code))
    }

    #[must_use]
    pub const fn pm_from_index(&self) -> Option<&OverlapIndex<'sig>> {
        self.pm_from_index.as_ref()
    }

    #[must_use]
    pub const fn pm_into_index(&self) -> Option<&OverlapIndex<'sig>> {
        self.pm_into_index.as_ref()
    }

    #[must_use]
    pub const fn pm_negp_index(&self) -> Option<&OverlapIndex<'sig>> {
        self.pm_negp_index.as_ref()
    }

    #[must_use]
    pub fn pm_paramodulation_indexes(
        &self,
    ) -> Option<(
        &OverlapIndex<'sig>,
        &OverlapIndex<'sig>,
        &OverlapIndex<'sig>,
    )> {
        Some((
            self.pm_into_index.as_ref()?,
            self.pm_negp_index.as_ref()?,
            self.pm_from_index.as_ref()?,
        ))
    }

    /// # Panics
    ///
    /// Panics if `clause` is already marked as globally indexed.
    pub fn insert_clause(&mut self, clause: &mut Clause, bank: &TermBank, lambda_demod: bool) {
        assert!(
            !clause.query_prop(CP_IS_GLOBAL_INDEXED),
            "global index insert expects an unindexed clause"
        );
        clause.set_prop(CP_IS_GLOBAL_INDEXED);
        if let Some(index) = self.bw_rw_index.as_mut() {
            let _timer = crate::basics::perf_counters::start(
                crate::basics::perf_counters::PerfCounter::BwrwIndexTimer,
            );
            index.insert_clause(clause, lambda_demod);
        }
        if let Some(pm_into_index) = self.pm_into_index.as_mut() {
            let _timer = crate::basics::perf_counters::start(
                crate::basics::perf_counters::PerfCounter::PmIndexTimer,
            );
            let pm_negp_index = self
                .pm_negp_index
                .as_mut()
                .expect("PM-into index requires matching negative-predicate index");
            overlap_index_insert_into_clause2(pm_into_index, pm_negp_index, clause, bank);
        }
        if let Some(index) = self.pm_from_index.as_mut() {
            let _timer = crate::basics::perf_counters::start(
                crate::basics::perf_counters::PerfCounter::PmIndexTimer,
            );
            index.insert_from_clause(clause);
        }
        if let Some(index) = self.ext_sup_into_index.as_mut() {
            index.insert_into_clause(clause, self.ext_rules_max_depth);
            self.ext_sup_from_index
                .as_mut()
                .expect("ExtSup into index requires matching from index")
                .insert_from_clause(clause, self.ext_rules_max_depth);
        }
    }

    /// # Panics
    ///
    /// Panics if `clause` is not marked as globally indexed.
    pub fn delete_clause(&mut self, clause: &mut Clause, bank: &TermBank, lambda_demod: bool) {
        assert!(
            clause.query_prop(CP_IS_GLOBAL_INDEXED),
            "global index delete expects an indexed clause"
        );
        clause.del_prop(CP_IS_GLOBAL_INDEXED);
        if let Some(index) = self.bw_rw_index.as_mut() {
            let _timer = crate::basics::perf_counters::start(
                crate::basics::perf_counters::PerfCounter::BwrwIndexTimer,
            );
            index.delete_clause(clause, lambda_demod);
        }
        if let Some(pm_into_index) = self.pm_into_index.as_mut() {
            let _timer = crate::basics::perf_counters::start(
                crate::basics::perf_counters::PerfCounter::PmIndexTimer,
            );
            let pm_negp_index = self
                .pm_negp_index
                .as_mut()
                .expect("PM-into index requires matching negative-predicate index");
            overlap_index_delete_into_clause2(pm_into_index, pm_negp_index, clause, bank);
        }
        if let Some(index) = self.pm_from_index.as_mut() {
            let _timer = crate::basics::perf_counters::start(
                crate::basics::perf_counters::PerfCounter::PmIndexTimer,
            );
            index.delete_from_clause(clause);
        }
        if let Some(index) = self.ext_sup_into_index.as_mut() {
            index.delete_into_clause(clause);
            self.ext_sup_from_index
                .as_mut()
                .expect("ExtSup into index requires matching from index")
                .delete_from_clause(clause);
        }
    }

    pub fn insert_clause_set(
        &mut self,
        set: &mut ClauseSet,
        bank: &TermBank,
        lambda_demod: bool,
    ) -> i64 {
        // C GlobalIndicesInsertClauseSet returns before inserting into any
        // configured index unless the backward rewrite index is present.
        if self.bw_rw_index.is_none() {
            return 0;
        }
        let mut inserted = 0;
        for clause in set.iter_mut() {
            self.insert_clause(clause, bank, lambda_demod);
            inserted += 1;
        }
        inserted
    }

    #[cfg(feature = "print-index-stats")]
    #[must_use]
    pub fn index_statistics_string(&self, bank: &TermBank) -> String {
        let mut output = String::new();
        let _ = self.write_index_statistics(&mut output, bank);
        output
    }

    #[cfg(feature = "print-index-stats")]
    pub fn write_index_statistics(
        &self,
        output: &mut impl FmtWrite,
        bank: &TermBank,
    ) -> fmt::Result {
        write!(output, "{DEFAULT_COMCHAR_RAW} Backwards rewriting index : ")?;
        write_subterm_index_distrib_data(output, self.bw_rw_index.as_ref())?;
        output.write_char('\n')?;
        write!(output, "{DEFAULT_COMCHAR_RAW} Paramod-from index        : ")?;
        write_overlap_index_distrib_data(output, self.pm_from_index.as_ref())?;
        output.write_char('\n')?;
        if let Some(index) = &self.pm_from_index {
            output.write_str(&index.dot_string("pm_from_index", |payload, _signature| {
                subterm_occurrences_dot_record_string(
                    &format!("{payload:p}"),
                    payload.iter(),
                    bank,
                    ProblemType::FirstOrder,
                )
            }))?;
        }
        write!(output, "{DEFAULT_COMCHAR_RAW} Paramod-into index        : ")?;
        write_overlap_index_distrib_data(output, self.pm_into_index.as_ref())?;
        output.write_char('\n')?;
        write!(output, "{DEFAULT_COMCHAR_RAW} Paramod-neg-atom index    : ")?;
        write_overlap_index_distrib_data(output, self.pm_negp_index.as_ref())?;
        output.write_char('\n')?;
        Ok(())
    }

    #[cfg(feature = "print-index-stats")]
    pub fn write_index_statistics_io(
        &self,
        output: &mut impl IoWrite,
        bank: &TermBank,
    ) -> io::Result<()> {
        let mut text = String::new();
        self.write_index_statistics(&mut text, bank)
            .map_err(|_| io::Error::other("failed to format global index statistics"))?;
        output.write_all(text.as_bytes())
    }
}

#[must_use]
pub fn global_indices_null<'sig>() -> GlobalIndices<'sig> {
    GlobalIndices::null()
}

#[cfg(feature = "print-index-stats")]
fn write_subterm_index_distrib_data(
    output: &mut impl FmtWrite,
    index: Option<&SubtermIndex<'_>>,
) -> fmt::Result {
    match index {
        Some(index) => index.write_distrib_data(output),
        None => write_null_fp_index_distrib_data(output),
    }
}

#[cfg(feature = "print-index-stats")]
fn write_overlap_index_distrib_data(
    output: &mut impl FmtWrite,
    index: Option<&OverlapIndex<'_>>,
) -> fmt::Result {
    match index {
        Some(index) => index.write_distrib_data(output),
        None => write_null_fp_index_distrib_data(output),
    }
}

#[cfg(feature = "print-index-stats")]
fn write_null_fp_index_distrib_data(output: &mut impl FmtWrite) -> fmt::Result {
    crate::terms::fp_index::FPIndexDistrib {
        nodes: 0,
        leaves: 0,
        average: 0.0,
        stddev: 0.0,
    }
    .write_data(output)
}

#[cfg(test)]
mod tests {
    use super::{global_indices_null, GlobalIndices};
    use crate::basics::simple_stuff::ProblemType;
    use crate::clauses::clause::Clause;
    use crate::clauses::clause_props::CP_IS_GLOBAL_INDEXED;
    use crate::clauses::clausesets::ClauseSet;
    use crate::clauses::eqn::Eqn;
    use crate::clauses::eqn_props::{EP_IS_MAXIMAL, EP_IS_ORIENTED};
    use crate::clauses::eqnlist::EqnList;
    use crate::terms::signature::Signature;
    use crate::terms::simpletypes::{alloc_arrow_type, Type};
    use crate::terms::termbanks::TermBank;
    use crate::terms::termtypes::{DerefType, Term};
    use crate::terms::typebanks::TypeBank;

    fn test_bank() -> TermBank {
        let mut signature = Signature::new(TypeBank::new());
        signature.insert_internal_codes().unwrap();
        TermBank::new(signature).unwrap()
    }

    fn typed_const_of_type(bank: &mut TermBank, name: &str, type_: Type) -> Term {
        let f_code = bank.signature_mut().insert_id(name, 0, false);
        if bank.signature().get_type(f_code).is_none() {
            bank.signature_mut()
                .declare_final_type(f_code, type_.clone())
                .unwrap();
        }
        let term = Term::const_cell_alloc(f_code);
        term.set_type(Some(type_));
        bank.insert(&term, DerefType::Never).unwrap()
    }

    fn typed_const(bank: &mut TermBank, name: &str) -> Term {
        let type_ = bank.signature().type_bank().default_type();
        typed_const_of_type(bank, name, type_)
    }

    fn typed_unary_with_return(
        bank: &mut TermBank,
        name: &str,
        arg: &Term,
        return_type: Type,
    ) -> Term {
        let arg_type = arg
            .type_()
            .unwrap_or_else(|| bank.signature().type_bank().default_type());
        let f_code = bank.signature_mut().insert_id(name, 1, false);
        if bank.signature().get_type(f_code).is_none() {
            bank.signature_mut()
                .declare_final_type(
                    f_code,
                    alloc_arrow_type(vec![arg_type, return_type.clone()]),
                )
                .unwrap();
        }
        let term = Term::top_alloc(f_code, 1);
        term.set_type(Some(return_type));
        term.set_argument(0, arg.clone());
        bank.insert(&term, DerefType::Never).unwrap()
    }

    fn typed_unary(bank: &mut TermBank, name: &str, arg: &Term) -> Term {
        let type_ = bank.signature().type_bank().default_type();
        typed_unary_with_return(bank, name, arg, type_)
    }

    fn typed_predicate(bank: &mut TermBank, name: &str, arg: &Term) -> Term {
        let individual = bank.signature().type_bank().default_type();
        let bool_type = bank.signature().type_bank().bool_type();
        let p_type = bank
            .signature_mut()
            .type_bank_mut()
            .insert_type_shared(alloc_arrow_type(vec![individual, bool_type]));
        let f_code = bank.signature_mut().insert_id(name, 1, false);
        if bank.signature().get_type(f_code).is_none() {
            bank.signature_mut()
                .declare_final_type(f_code, p_type)
                .unwrap();
        }
        let term = Term::top_alloc(f_code, 1);
        term.set_type(Some(bank.signature().type_bank().bool_type()));
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

    fn maximal_unit_clause(bank: &mut TermBank, name: &str, ident: i64) -> (Clause, Term, Term) {
        let left = typed_const(bank, name);
        let right_arg = typed_const(bank, &format!("{name}_arg"));
        let right = typed_unary(bank, &format!("{name}_f"), &right_arg);
        let mut literal = Eqn::alloc(left.clone(), right.clone(), bank, true).unwrap();
        literal.set_prop(EP_IS_MAXIMAL);
        let mut clause = Clause::alloc(EqnList::from_vec(vec![literal]));
        clause.set_ident(ident);
        (clause, left, right)
    }

    fn maximal_negative_atom_clause(
        bank: &mut TermBank,
        name: &str,
        ident: i64,
    ) -> (Clause, Term, Term) {
        let arg = typed_const(bank, &format!("{name}_arg"));
        let body = typed_unary(bank, &format!("{name}_f"), &arg);
        let atom = typed_predicate(bank, &format!("{name}_p"), &body);
        let mut literal = Eqn::alloc(atom.clone(), bank.true_term().clone(), bank, false).unwrap();
        literal.set_prop(EP_IS_MAXIMAL | EP_IS_ORIENTED);
        let mut clause = Clause::alloc(EqnList::from_vec(vec![literal]));
        clause.set_ident(ident);
        (clause, atom, body)
    }

    #[test]
    fn null_indices_start_without_configured_indexes() {
        let indices = global_indices_null();

        assert!(!indices.has_bw_rw_index());
        assert!(!indices.has_pm_from_index());
        assert!(!indices.has_pm_into_index());
        assert!(!indices.has_pm_negp_index());
        assert!(!indices.has_ext_into_index());
        assert!(!indices.has_ext_from_index());
        assert_eq!(indices.rw_bw_index_type(), "");
        assert_eq!(indices.ext_rules_max_depth(), 0);
        assert_eq!(indices.problem_type(), ProblemType::NotInitialized);
    }

    #[test]
    fn init_stores_names_and_allocates_only_known_backward_index() {
        let bank = test_bank();
        let indices = GlobalIndices::new(bank.signature(), "FP1", "NoIndex", "FP7", -1);

        assert!(indices.has_bw_rw_index());
        assert!(!indices.has_pm_from_index());
        assert!(indices.has_pm_into_index());
        assert!(indices.has_pm_negp_index());
        assert!(!indices.has_ext_into_index());
        assert!(!indices.has_ext_from_index());
        assert_eq!(indices.rw_bw_index_type(), "FP1");
        assert_eq!(indices.pm_from_index_type(), "NoIndex");
        assert_eq!(indices.pm_into_index_type(), "FP7");
        assert_eq!(indices.pm_negp_index_type(), "FP7");
        assert_eq!(indices.ext_rules_max_depth(), -1);
        assert_eq!(indices.problem_type(), ProblemType::FirstOrder);
    }

    #[test]
    fn higher_order_init_allocates_extension_indexes() {
        let bank = test_bank();
        let indices = GlobalIndices::new_for_problem(
            bank.signature(),
            "NoIndex",
            "NoIndex",
            "NoIndex",
            3,
            ProblemType::HigherOrder,
        );

        assert!(!indices.has_bw_rw_index());
        assert!(!indices.has_pm_from_index());
        assert!(!indices.has_pm_into_index());
        assert!(!indices.has_pm_negp_index());
        assert!(indices.has_ext_into_index());
        assert!(indices.has_ext_from_index());
        assert_eq!(indices.ext_rules_max_depth(), 3);
        assert_eq!(indices.problem_type(), ProblemType::HigherOrder);
    }

    #[test]
    fn insert_clause_sets_global_prop_and_populates_backward_index() {
        let mut bank = test_bank();
        let (mut clause, left) = unit_clause(&mut bank, "gidx_clause", 10);
        let mut indices = GlobalIndices::new(bank.signature(), "FP1", "NoIndex", "NoIndex", 0);

        indices.insert_clause(&mut clause, &bank, false);

        assert!(clause.query_prop(CP_IS_GLOBAL_INDEXED));
        assert!(indices.find_bw_rw_occurrence(&left).is_some());

        indices.delete_clause(&mut clause, &bank, false);

        assert!(!clause.query_prop(CP_IS_GLOBAL_INDEXED));
        assert!(indices.find_bw_rw_occurrence(&left).is_none());
    }

    #[test]
    fn insert_clause_populates_paramodulation_overlap_indexes() {
        let mut bank = test_bank();
        let (mut clause, left, right) = maximal_unit_clause(&mut bank, "gidx_pm_clause", 11);
        let mut indices = GlobalIndices::new(bank.signature(), "NoIndex", "FP1", "FP1", 0);

        indices.insert_clause(&mut clause, &bank, false);

        assert!(clause.query_prop(CP_IS_GLOBAL_INDEXED));
        assert!(indices.find_pm_into_occurrence(&left).is_some());
        assert!(indices.find_pm_into_occurrence(&right).is_some());
        assert!(indices.find_pm_from_occurrence(&left).is_some());
        assert!(indices.find_pm_from_occurrence(&right).is_some());
        assert!(indices.find_pm_negp_occurrence(&left).is_none());

        indices.delete_clause(&mut clause, &bank, false);

        assert!(!clause.query_prop(CP_IS_GLOBAL_INDEXED));
        assert!(indices.find_pm_into_occurrence(&left).is_none());
        assert!(indices.find_pm_from_occurrence(&left).is_none());
    }

    #[test]
    fn insert_clause_routes_negative_atom_heads_to_negp_index() {
        let mut bank = test_bank();
        let (mut clause, atom, body) = maximal_negative_atom_clause(&mut bank, "gidx_negp", 12);
        let mut indices = GlobalIndices::new(bank.signature(), "NoIndex", "NoIndex", "FP1", 0);

        indices.insert_clause(&mut clause, &bank, false);

        assert!(indices.find_pm_negp_occurrence(&atom).is_some());
        assert!(indices.find_pm_into_occurrence(&atom).is_none());
        assert!(indices.find_pm_into_occurrence(&body).is_some());

        indices.delete_clause(&mut clause, &bank, false);

        assert!(indices.find_pm_negp_occurrence(&atom).is_none());
        assert!(indices.find_pm_into_occurrence(&body).is_none());
    }

    #[test]
    fn insert_clause_populates_extension_indexes_for_higher_order_problem() {
        let mut bank = test_bank();
        let individual = bank.signature().type_bank().default_type();
        let arrow_type = alloc_arrow_type(vec![individual.clone(), individual.clone()]);
        let arrow = typed_const_of_type(&mut bank, "gidx_ext_arrow", arrow_type);
        let left = typed_unary_with_return(&mut bank, "gidx_ext_left", &arrow, individual.clone());
        let right = typed_const(&mut bank, "gidx_ext_right");
        let literal = Eqn::alloc(left.clone(), right, &mut bank, true).unwrap();
        let mut clause = Clause::alloc(EqnList::from_vec(vec![literal]));
        clause.set_ident(50);
        let mut indices = GlobalIndices::new_for_problem(
            bank.signature(),
            "NoIndex",
            "NoIndex",
            "NoIndex",
            5,
            ProblemType::HigherOrder,
        );

        indices.insert_clause(&mut clause, &bank, false);

        assert!(clause.query_prop(CP_IS_GLOBAL_INDEXED));
        assert!(indices
            .find_ext_into_symbol(left.f_code())
            .unwrap()
            .find(&clause)
            .is_some());
        assert!(indices
            .find_ext_from_symbol(left.f_code())
            .unwrap()
            .find(&clause)
            .is_some());

        indices.delete_clause(&mut clause, &bank, false);

        assert!(!clause.query_prop(CP_IS_GLOBAL_INDEXED));
        assert!(indices.find_ext_into_symbol(left.f_code()).is_none());
        assert!(indices.find_ext_from_symbol(left.f_code()).is_none());
    }

    #[test]
    fn extension_indexes_keep_insert_depth_gate_through_global_owner() {
        let mut bank = test_bank();
        let individual = bank.signature().type_bank().default_type();
        let arrow_type = alloc_arrow_type(vec![individual.clone(), individual.clone()]);
        let arrow = typed_const_of_type(&mut bank, "gidx_ext_deep_arrow", arrow_type);
        let left =
            typed_unary_with_return(&mut bank, "gidx_ext_deep_left", &arrow, individual.clone());
        let right = typed_const(&mut bank, "gidx_ext_deep_right");
        let literal = Eqn::alloc(left.clone(), right, &mut bank, true).unwrap();
        let mut clause = Clause::alloc(EqnList::from_vec(vec![literal]));
        clause.set_ident(51);
        clause.set_proof_depth(6);
        let mut indices = GlobalIndices::new_for_problem(
            bank.signature(),
            "NoIndex",
            "NoIndex",
            "NoIndex",
            5,
            ProblemType::HigherOrder,
        );

        indices.insert_clause(&mut clause, &bank, false);

        assert!(clause.query_prop(CP_IS_GLOBAL_INDEXED));
        assert!(indices.find_ext_into_symbol(left.f_code()).is_none());
        assert!(indices.find_ext_from_symbol(left.f_code()).is_none());

        indices.delete_clause(&mut clause, &bank, false);

        assert!(!clause.query_prop(CP_IS_GLOBAL_INDEXED));
    }

    #[test]
    fn insert_clause_set_is_noop_without_backward_index() {
        let mut bank = test_bank();
        let (clause, _) = unit_clause(&mut bank, "gidx_noindex", 20);
        let mut set = ClauseSet::new();
        set.insert(clause);
        let mut indices = GlobalIndices::new(bank.signature(), "NoIndex", "FP1", "FP1", 0);

        assert_eq!(indices.insert_clause_set(&mut set, &bank, false), 0);
        assert!(set
            .iter()
            .all(|clause| !clause.query_prop(CP_IS_GLOBAL_INDEXED)));
    }

    #[test]
    fn insert_clause_set_skips_extension_index_without_backward_index_like_c() {
        let mut bank = test_bank();
        let individual = bank.signature().type_bank().default_type();
        let arrow_type = alloc_arrow_type(vec![individual.clone(), individual.clone()]);
        let arrow = typed_const_of_type(&mut bank, "gidx_set_ext_arrow", arrow_type);
        let left = typed_unary_with_return(&mut bank, "gidx_set_ext_left", &arrow, individual);
        let right = typed_const(&mut bank, "gidx_set_ext_right");
        let literal = Eqn::alloc(left.clone(), right, &mut bank, true).unwrap();
        let mut clause = Clause::alloc(EqnList::from_vec(vec![literal]));
        clause.set_ident(21);
        let mut set = ClauseSet::new();
        set.insert(clause);
        let mut indices = GlobalIndices::new_for_problem(
            bank.signature(),
            "NoIndex",
            "NoIndex",
            "NoIndex",
            3,
            ProblemType::HigherOrder,
        );
        assert!(indices.has_ext_into_index());
        assert!(indices.has_ext_from_index());

        assert_eq!(indices.insert_clause_set(&mut set, &bank, false), 0);

        assert!(set
            .iter()
            .all(|clause| !clause.query_prop(CP_IS_GLOBAL_INDEXED)));
        assert!(indices.find_ext_into_symbol(left.f_code()).is_none());
        assert!(indices.find_ext_from_symbol(left.f_code()).is_none());
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

        assert_eq!(indices.insert_clause_set(&mut set, &bank, false), 2);

        assert!(set
            .iter()
            .all(|clause| clause.query_prop(CP_IS_GLOBAL_INDEXED)));
        assert!(indices.find_bw_rw_occurrence(&left).is_some());
    }

    #[test]
    fn reset_rebuilds_configured_backward_index_empty() {
        let mut bank = test_bank();
        let (mut clause, left) = unit_clause(&mut bank, "gidx_reset", 40);
        let mut indices = GlobalIndices::new(bank.signature(), "FP1", "FP1", "FP1", 2);
        indices.insert_clause(&mut clause, &bank, false);

        indices.reset();

        assert!(indices.has_bw_rw_index());
        assert!(indices.has_pm_from_index());
        assert!(indices.has_pm_into_index());
        assert!(indices.has_pm_negp_index());
        assert!(!indices.has_ext_into_index());
        assert!(!indices.has_ext_from_index());
        assert_eq!(indices.rw_bw_index_type(), "FP1");
        assert_eq!(indices.ext_rules_max_depth(), 2);
        assert_eq!(indices.problem_type(), ProblemType::FirstOrder);
        assert!(indices.find_bw_rw_occurrence(&left).is_none());
        assert!(indices.find_pm_from_occurrence(&left).is_none());
    }

    #[test]
    fn reset_rebuilds_higher_order_extension_indexes_empty() {
        let mut bank = test_bank();
        let individual = bank.signature().type_bank().default_type();
        let arrow_type = alloc_arrow_type(vec![individual.clone(), individual.clone()]);
        let arrow = typed_const_of_type(&mut bank, "gidx_ext_reset_arrow", arrow_type);
        let left =
            typed_unary_with_return(&mut bank, "gidx_ext_reset_left", &arrow, individual.clone());
        let right = typed_const(&mut bank, "gidx_ext_reset_right");
        let literal = Eqn::alloc(left.clone(), right, &mut bank, true).unwrap();
        let mut clause = Clause::alloc(EqnList::from_vec(vec![literal]));
        clause.set_ident(52);
        let mut indices = GlobalIndices::new_for_problem(
            bank.signature(),
            "NoIndex",
            "NoIndex",
            "NoIndex",
            5,
            ProblemType::HigherOrder,
        );
        indices.insert_clause(&mut clause, &bank, false);

        indices.reset();

        assert!(indices.has_ext_into_index());
        assert!(indices.has_ext_from_index());
        assert_eq!(indices.ext_rules_max_depth(), 5);
        assert_eq!(indices.problem_type(), ProblemType::HigherOrder);
        assert!(indices.find_ext_into_symbol(left.f_code()).is_none());
        assert!(indices.find_ext_from_symbol(left.f_code()).is_none());
    }

    #[cfg(feature = "print-index-stats")]
    #[test]
    fn index_statistics_string_prints_c_optional_index_stats_block() {
        let mut bank = test_bank();
        let (mut clause, _, _) = maximal_unit_clause(&mut bank, "gidx_stats", 70);
        let mut indices = GlobalIndices::new(bank.signature(), "FP1", "FP1", "FP1", 0);

        indices.insert_clause(&mut clause, &bank, false);

        let stats = indices.index_statistics_string(&bank);
        let mut written_stats = String::new();
        indices
            .write_index_statistics(&mut written_stats, &bank)
            .unwrap();
        assert_eq!(written_stats, stats);
        let mut io_stats = Vec::new();
        indices
            .write_index_statistics_io(&mut io_stats, &bank)
            .unwrap();
        assert_eq!(String::from_utf8(io_stats).unwrap(), stats);
        assert!(stats.contains("% Backwards rewriting index :"));
        assert!(stats.contains("% Paramod-from index        :"));
        assert!(stats.contains("graph pm_from_index{\n   rankdir=LR\n   nodesep=0.05\n"));
        assert!(stats.contains("subgraph g"));
        assert!(stats.contains("shape=record"));
        assert!(stats.contains("gidx_stats"));
        assert!(stats.contains("-- t"));
        assert!(stats.contains("% Paramod-into index        :"));
        assert!(stats.contains("% Paramod-neg-atom index    :"));
    }
}
