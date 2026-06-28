use crate::basics::error::{Diagnostic, ErrorCode};
use crate::clauses::clause::Clause;
use crate::clauses::clause_props::{CP_IS_SOS, CP_TYPE_MASK};
use crate::clauses::clausesets::ClauseSet;
use crate::clauses::eqn::Eqn;
use crate::clauses::eqn_props::EP_IS_SPLIT_LIT;
use crate::clauses::eqnlist::EqnList;
use crate::clauses::subsumption::{
    clause_set_find_variant_clause_indexed, clause_subsume_order_sort_lits, clause_subsumes_clause,
};
use crate::terms::signature::FP_CL_SPLIT_DEF;
use crate::terms::simpletypes::alloc_arrow_type;
use crate::terms::termbanks::TermBank;
use crate::terms::termtypes::{term_identity_id, DerefType, Term};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClauseSplitType {
    GroundNone,
    GroundOne,
    GroundFull,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ClauseSplitOutcome {
    Unsplit(Box<Clause>),
    Split(Vec<Clause>, usize),
}

impl ClauseSplitOutcome {
    #[must_use]
    pub fn split_count(&self) -> usize {
        match self {
            Self::Unsplit(_) => 0,
            Self::Split(_, count) => *count,
        }
    }

    #[must_use]
    pub fn queued_count(&self) -> usize {
        match self {
            Self::Unsplit(_) => 0,
            Self::Split(clauses, _) => clauses.len(),
        }
    }
}

#[derive(Debug)]
pub struct SplitDefinitionStore<'a> {
    clauses: &'a mut ClauseSet,
    predicates: &'a mut BTreeMap<i64, i64>,
}

impl<'a> SplitDefinitionStore<'a> {
    pub fn new(clauses: &'a mut ClauseSet, predicates: &'a mut BTreeMap<i64, i64>) -> Self {
        Self {
            clauses,
            predicates,
        }
    }

    #[must_use]
    pub fn clauses(&self) -> &ClauseSet {
        self.clauses
    }

    #[must_use]
    pub fn predicates(&self) -> &BTreeMap<i64, i64> {
        self.predicates
    }
}

#[derive(Clone, Debug)]
struct LitSplitDesc {
    literal: Eqn,
    part: usize,
    varset: BTreeSet<usize>,
}

/// Returns whether the clause contains a C `EPIsSplitLit` marker.
///
/// C `ClauseHasSplitLiteral` checks the literal property, not the generated
/// predicate flag, even though other callers treat either marker as a split
/// literal.
#[must_use]
pub fn clause_has_split_literal(clause: &Clause) -> bool {
    clause
        .literals()
        .as_slice()
        .iter()
        .any(|literal| literal.query_prop(EP_IS_SPLIT_LIT))
}

/// Generates a C `GenDefLit`-style split definition literal.
///
/// This is the arity-zero convenience wrapper used by ordinary `ClauseSplit`.
///
/// # Errors
///
/// Returns a diagnostic if signature type declaration or term-bank insertion
/// fails.
pub fn gen_ground_def_lit(
    bank: &mut TermBank,
    pred: i64,
    positive: bool,
) -> Result<Eqn, Diagnostic> {
    gen_def_lit(bank, pred, positive, &[])
}

/// Generates a C `GenDefLit`-style split definition literal.
///
/// `split_vars` are used as the arguments of the generated predicate. The
/// ordinary `ClauseSplit` path passes an empty slice; `ClauseSplitGeneral` uses
/// a non-empty slice for variables shared between split parts.
///
/// # Errors
///
/// Returns a diagnostic if signature type declaration or term-bank insertion
/// fails.
pub fn gen_def_lit(
    bank: &mut TermBank,
    pred: i64,
    positive: bool,
    split_vars: &[Term],
) -> Result<Eqn, Diagnostic> {
    if pred <= 0 {
        return Err(Diagnostic::new(
            ErrorCode::OTHER_ERROR,
            "split definition predicate must be positive",
        ));
    }

    let bool_type = bank.signature().type_bank().bool_type();
    let mut split_var_types = Vec::with_capacity(split_vars.len());
    for variable in split_vars {
        let Some(type_) = variable.type_() else {
            return Err(Diagnostic::new(
                ErrorCode::TYPE_ERROR,
                "split variable has no type",
            ));
        };
        split_var_types.push(type_);
    }
    if bank.signature().get_type(pred).is_none() {
        let pred_type = if split_var_types.is_empty() {
            bool_type.clone()
        } else {
            split_var_types.push(bool_type.clone());
            alloc_arrow_type(split_var_types)
        };
        bank.signature_mut().declare_type(pred, pred_type)?;
    }
    bank.signature_mut().set_func_prop(pred, FP_CL_SPLIT_DEF);

    let term = if split_vars.is_empty() {
        Term::const_cell_alloc(pred)
    } else {
        let term = Term::top_alloc(pred, split_vars.len());
        for (index, variable) in split_vars.iter().enumerate() {
            term.set_argument(index, variable.clone());
        }
        term
    };
    term.set_type(Some(bool_type));
    let term = bank.insert(&term, DerefType::Never)?;
    let true_term = bank.true_term().clone();
    let mut literal = Eqn::alloc(term, true_term, bank, positive)?;
    literal.set_prop(EP_IS_SPLIT_LIT);
    Ok(literal)
}

/// Performs the fresh-definition C `ClauseSplit` path.
///
/// The returned `Split` vector contains the definition clauses followed by the
/// residual clause, matching the clauses inserted into the destination set by C.
/// Definition reuse (`fresh_defs == false`) and formula-archive side effects
/// are not represented here.
///
/// # Errors
///
/// Returns diagnostics from generated predicate typing, term-bank insertion, or
/// split-literal allocation.
pub fn clause_split_fresh(
    bank: &mut TermBank,
    clause: Clause,
    how: ClauseSplitType,
) -> Result<ClauseSplitOutcome, Diagnostic> {
    clause_split(bank, None, clause, how, true)
}

/// Performs the C `ClauseSplit` path, including non-fresh definition reuse.
///
/// When `fresh_defs` is false, this uses `store` to find variant definition
/// bodies and reuses their generated split predicate. Formula archive and
/// proof-output side effects remain with the future formula owner.
///
/// # Errors
///
/// Returns diagnostics from generated predicate typing, term-bank insertion,
/// split-literal allocation, or a missing definition store when reuse is
/// requested.
pub fn clause_split(
    bank: &mut TermBank,
    store: Option<&mut SplitDefinitionStore<'_>>,
    clause: Clause,
    how: ClauseSplitType,
    fresh_defs: bool,
) -> Result<ClauseSplitOutcome, Diagnostic> {
    clause_split_general(bank, store, clause, how, fresh_defs, &[])
}

/// Performs the fresh-definition C `clause_split_general` path.
///
/// Variables in `split_vars` are treated as parameters shared by split parts and
/// become arguments of the generated split predicates.
///
/// # Errors
///
/// Returns diagnostics from generated predicate typing, term-bank insertion, or
/// split-literal allocation.
pub fn clause_split_general_fresh(
    bank: &mut TermBank,
    clause: Clause,
    how: ClauseSplitType,
    split_vars: &[Term],
) -> Result<ClauseSplitOutcome, Diagnostic> {
    clause_split_general(bank, None, clause, how, true, split_vars)
}

/// Performs the C `clause_split_general` path.
///
/// Non-fresh definition reuse is only used for the ordinary arity-zero split
/// path. This matches C: once `ClauseSplitGeneral` selects split variables, the
/// inner path always creates fresh parameterized split predicates.
///
/// # Errors
///
/// Returns diagnostics from generated predicate typing, term-bank insertion,
/// split-literal allocation, or a missing definition store when reuse is
/// requested.
pub fn clause_split_general(
    bank: &mut TermBank,
    mut store: Option<&mut SplitDefinitionStore<'_>>,
    mut clause: Clause,
    how: ClauseSplitType,
    fresh_defs: bool,
    split_vars: &[Term],
) -> Result<ClauseSplitOutcome, Diagnostic> {
    let lit_no = clause.literal_number();
    if lit_no <= 1 || clause_has_split_literal(&clause) {
        return Ok(ClauseSplitOutcome::Unsplit(Box::new(clause)));
    }

    let props = clause.give_props(CP_TYPE_MASK | CP_IS_SOS);
    let ignored_vars = split_var_ids(split_vars);
    let mut lit_table = initialize_lit_table(clause.literals().as_slice(), how, &ignored_vars);
    let mut part = 0;

    if how == ClauseSplitType::GroundOne && c_truthy_find_free_literal(&lit_table) {
        part += 1;
    }

    while let Some(index) = find_free_literal(&lit_table) {
        part += 1;
        build_part(&mut lit_table, index, part);
    }

    if part <= 1 {
        return Ok(ClauseSplitOutcome::Unsplit(Box::new(clause)));
    }

    let mut split_clauses = Vec::with_capacity(part + 1);
    let mut residual_literals = Vec::with_capacity(part);
    let arity = i32::try_from(split_vars.len()).map_err(|_| {
        Diagnostic::new(
            ErrorCode::OTHER_ERROR,
            "split predicate arity does not fit C int",
        )
    })?;
    for part_index in 1..=part {
        let split_literals = assemble_part_literals(&lit_table, part_index);
        let pred = if fresh_defs || !split_vars.is_empty() {
            let pred = bank.signature_mut().get_new_predicate_code(arity);
            let mut clause_literals = Vec::with_capacity(lit_no + 1);
            clause_literals.push(gen_def_lit(bank, pred, true, split_vars)?);
            clause_literals.extend(split_literals);

            let mut new_clause = Clause::alloc(EqnList::from_vec(clause_literals));
            new_clause.set_properties(props);
            split_clauses.push(new_clause);
            pred
        } else {
            let store = store.as_deref_mut().ok_or_else(|| {
                Diagnostic::new(
                    ErrorCode::OTHER_ERROR,
                    "split definition reuse requires an initialized definition store",
                )
            })?;
            let definition = get_or_create_definition(bank, store, split_literals)?;
            if let Some(mut new_clause) = definition.new_clause {
                new_clause.set_properties(props);
                split_clauses.push(new_clause);
            }
            definition.pred
        };

        residual_literals.push(gen_def_lit(bank, pred, false, split_vars)?);
    }

    residual_literals.reverse();
    clause.replace_literals(EqnList::from_vec(residual_literals));
    split_clauses.push(clause);
    Ok(ClauseSplitOutcome::Split(split_clauses, part + 1))
}

/// Performs C `ClauseSplitGeneral` with fresh split definitions.
///
/// This first tries ordinary `ClauseSplit` with `SplitGroundOne`, then tries
/// variable subsets of increasing cardinality while `tries` remains positive.
///
/// # Errors
///
/// Returns diagnostics from generated predicate typing, term-bank insertion, or
/// split-literal allocation.
pub fn clause_split_general_search_fresh(
    bank: &mut TermBank,
    clause: Clause,
    tries: i64,
) -> Result<ClauseSplitOutcome, Diagnostic> {
    clause_split_general_search(bank, None, clause, tries, true)
}

/// Performs C `ClauseSplitGeneral`.
///
/// The initial ordinary split observes `fresh_defs`; later split-variable
/// subset attempts still use fresh parameterized definitions, matching C.
///
/// # Errors
///
/// Returns diagnostics from generated predicate typing, term-bank insertion,
/// split-literal allocation, or a missing definition store when reuse is
/// requested.
pub fn clause_split_general_search(
    bank: &mut TermBank,
    store: Option<&mut SplitDefinitionStore<'_>>,
    clause: Clause,
    tries: i64,
    fresh_defs: bool,
) -> Result<ClauseSplitOutcome, Diagnostic> {
    let mut clause = match clause_split_general(
        bank,
        store,
        clause,
        ClauseSplitType::GroundOne,
        fresh_defs,
        &[],
    )? {
        ClauseSplitOutcome::Split(clauses, count) => {
            return Ok(ClauseSplitOutcome::Split(clauses, count));
        }
        ClauseSplitOutcome::Unsplit(clause) => *clause,
    };

    let mut variables = BTreeMap::new();
    let var_no = clause.collect_variables(&mut variables);
    if var_no <= 2 {
        return Ok(ClauseSplitOutcome::Unsplit(Box::new(clause)));
    }

    let vars = variables.into_values().collect::<Vec<_>>();
    let mut set_size = 1;
    let mut permutation = initialize_permute_stack(set_size);
    let mut tries = tries;
    while tries > 0 {
        let split_vars = permutation
            .iter()
            .map(|index| vars[*index].clone())
            .collect::<Vec<_>>();
        match clause_split_general_fresh(
            bank,
            clause,
            ClauseSplitType::GroundNone,
            split_vars.as_slice(),
        )? {
            ClauseSplitOutcome::Split(clauses, count) => {
                return Ok(ClauseSplitOutcome::Split(clauses, count));
            }
            ClauseSplitOutcome::Unsplit(unsplit) => {
                clause = *unsplit;
            }
        }

        if !permute_stack_next(&mut permutation, vars.len()) {
            if set_size == vars.len().saturating_sub(2) {
                break;
            }
            set_size += 1;
            permutation = initialize_permute_stack(set_size);
        }
        tries -= 1;
    }
    Ok(ClauseSplitOutcome::Unsplit(Box::new(clause)))
}

/// Splits all clauses from `from_set` into `to_set`, matching the fresh
/// `ClauseSetSplitClauses` path.
///
/// Unsplit clauses are moved unchanged. The return value counts only clauses
/// produced by successful splits, matching C.
///
/// # Errors
///
/// Returns diagnostics from generated predicate typing, term-bank insertion, or
/// split-literal allocation.
pub fn clause_set_split_clauses_fresh(
    bank: &mut TermBank,
    from_set: &mut ClauseSet,
    to_set: &mut ClauseSet,
    how: ClauseSplitType,
) -> Result<i64, Diagnostic> {
    clause_set_split_clauses(bank, None, from_set, to_set, how, true)
}

/// Splits all clauses from `from_set` into `to_set`, matching
/// `ClauseSetSplitClauses`.
///
/// # Errors
///
/// Returns diagnostics from generated predicate typing, term-bank insertion,
/// split-literal allocation, or a missing definition store when reuse is
/// requested.
pub fn clause_set_split_clauses(
    bank: &mut TermBank,
    mut store: Option<&mut SplitDefinitionStore<'_>>,
    from_set: &mut ClauseSet,
    to_set: &mut ClauseSet,
    how: ClauseSplitType,
    fresh_defs: bool,
) -> Result<i64, Diagnostic> {
    let mut result = 0;
    while let Some(clause) = from_set.extract_first() {
        let outcome = match store.as_mut() {
            Some(store) => clause_split(bank, Some(&mut **store), clause, how, fresh_defs),
            None => clause_split(bank, None, clause, how, fresh_defs),
        }?;
        match outcome {
            ClauseSplitOutcome::Unsplit(clause) => to_set.insert(*clause),
            ClauseSplitOutcome::Split(clauses, count) => {
                result += usize_to_i64(count);
                for clause in clauses {
                    to_set.insert(clause);
                }
            }
        }
    }
    Ok(result)
}

/// Splits all clauses from `from_set` into `to_set`, matching the fresh
/// `ClauseSetSplitClausesGeneral` path.
///
/// # Errors
///
/// Returns diagnostics from generated predicate typing, term-bank insertion, or
/// split-literal allocation.
pub fn clause_set_split_clauses_general_fresh(
    bank: &mut TermBank,
    from_set: &mut ClauseSet,
    to_set: &mut ClauseSet,
    tries: i64,
) -> Result<i64, Diagnostic> {
    clause_set_split_clauses_general(bank, None, from_set, to_set, tries, true)
}

/// Splits all clauses from `from_set` into `to_set`, matching
/// `ClauseSetSplitClausesGeneral`.
///
/// # Errors
///
/// Returns diagnostics from generated predicate typing, term-bank insertion,
/// split-literal allocation, or a missing definition store when reuse is
/// requested.
pub fn clause_set_split_clauses_general(
    bank: &mut TermBank,
    mut store: Option<&mut SplitDefinitionStore<'_>>,
    from_set: &mut ClauseSet,
    to_set: &mut ClauseSet,
    tries: i64,
    fresh_defs: bool,
) -> Result<i64, Diagnostic> {
    let mut result = 0;
    while let Some(clause) = from_set.extract_first() {
        let outcome = match store.as_mut() {
            Some(store) => {
                clause_split_general_search(bank, Some(&mut **store), clause, tries, fresh_defs)
            }
            None => clause_split_general_search(bank, None, clause, tries, fresh_defs),
        }?;
        match outcome {
            ClauseSplitOutcome::Unsplit(clause) => to_set.insert(*clause),
            ClauseSplitOutcome::Split(clauses, count) => {
                result += usize_to_i64(count);
                for clause in clauses {
                    to_set.insert(clause);
                }
            }
        }
    }
    Ok(result)
}

fn initialize_lit_table(
    literals: &[Eqn],
    how: ClauseSplitType,
    ignored_vars: &BTreeSet<usize>,
) -> Vec<LitSplitDesc> {
    literals
        .iter()
        .cloned()
        .map(|literal| {
            let varset = literal_varset(&literal, ignored_vars);
            let part = usize::from(
                matches!(
                    how,
                    ClauseSplitType::GroundOne | ClauseSplitType::GroundNone
                ) && varset.is_empty(),
            );
            LitSplitDesc {
                literal,
                part,
                varset,
            }
        })
        .collect()
}

fn split_var_ids(split_vars: &[Term]) -> BTreeSet<usize> {
    split_vars.iter().map(term_identity_id).collect()
}

fn literal_varset(literal: &Eqn, ignored_vars: &BTreeSet<usize>) -> BTreeSet<usize> {
    let mut variables = BTreeMap::new();
    let _ = literal.collect_variables(&mut variables);
    variables
        .keys()
        .filter(|identity| !ignored_vars.contains(identity))
        .copied()
        .collect()
}

fn find_free_literal(lit_table: &[LitSplitDesc]) -> Option<usize> {
    lit_table
        .iter()
        .enumerate()
        .find_map(|(index, desc)| (desc.part == 0).then_some(index))
}

fn c_truthy_find_free_literal(lit_table: &[LitSplitDesc]) -> bool {
    find_free_literal(lit_table) != Some(0)
}

fn build_part(lit_table: &mut [LitSplitDesc], lit_index: usize, part: usize) {
    lit_table[lit_index].part = part;
    let mut new_vars = true;
    while new_vars {
        new_vars = false;
        for index in lit_index + 1..lit_table.len() {
            if lit_table[index].part == 0
                && lit_table[lit_index]
                    .varset
                    .iter()
                    .any(|var| lit_table[index].varset.contains(var))
            {
                lit_table[index].part = part;
                let varset = std::mem::take(&mut lit_table[index].varset);
                for variable in varset {
                    new_vars |= lit_table[lit_index].varset.insert(variable);
                }
            }
        }
    }
}

fn assemble_part_literals(lit_table: &[LitSplitDesc], part: usize) -> Vec<Eqn> {
    let mut literals = Vec::new();
    for desc in lit_table {
        if desc.part == part {
            literals.push(desc.literal.clone());
        }
    }
    literals.reverse();
    literals
}

#[derive(Debug)]
struct DefinitionLookup {
    pred: i64,
    new_clause: Option<Clause>,
}

fn get_or_create_definition(
    bank: &mut TermBank,
    store: &mut SplitDefinitionStore<'_>,
    split_literals: Vec<Eqn>,
) -> Result<DefinitionLookup, Diagnostic> {
    let mut def_clause = Clause::alloc(EqnList::from_vec(split_literals.clone()));
    def_clause.set_weight(def_clause.standard_weight());
    clause_subsume_order_sort_lits(&mut def_clause, bank);

    if let Some(variant_id) = find_definition_variant(store.clauses, &def_clause, bank) {
        let pred = store.predicates.get(&variant_id).copied().ok_or_else(|| {
            Diagnostic::new(
                ErrorCode::OTHER_ERROR,
                "split definition variant has no predicate association",
            )
        })?;
        return Ok(DefinitionLookup {
            pred,
            new_clause: None,
        });
    }

    let pred = bank.signature_mut().get_new_predicate_code(0);
    let mut clause_literals = Vec::with_capacity(split_literals.len() + 1);
    clause_literals.push(gen_ground_def_lit(bank, pred, true)?);
    clause_literals.extend(split_literals);
    let new_clause = Clause::alloc(EqnList::from_vec(clause_literals));

    let def_ident = def_clause.ident();
    let _ = store.predicates.insert(def_ident, pred);
    store.clauses.indexed_insert_clause_owned(def_clause, bank);

    Ok(DefinitionLookup {
        pred,
        new_clause: Some(new_clause),
    })
}

fn find_definition_variant(store: &ClauseSet, query: &Clause, bank: &TermBank) -> Option<i64> {
    if let Some(anchor) = store.fv_anchor() {
        return clause_set_find_variant_clause_indexed(anchor, query, bank).map(Clause::ident);
    }

    store
        .iter()
        .find(|candidate| {
            clause_subsumes_clause(candidate, query, bank)
                && clause_subsumes_clause(query, candidate, bank)
        })
        .map(Clause::ident)
}

fn initialize_permute_stack(size: usize) -> Vec<usize> {
    (0..size).collect()
}

fn permute_stack_next(permutation: &mut [usize], var_no: usize) -> bool {
    let size = permutation.len();
    for index in (0..size).rev() {
        let limit = var_no - (size - index);
        if permutation[index] < limit {
            permutation[index] += 1;
            for next in index + 1..size {
                permutation[next] = permutation[next - 1] + 1;
            }
            return true;
        }
    }
    false
}

fn usize_to_i64(value: usize) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

#[cfg(test)]
mod tests {
    use super::{
        clause_has_split_literal, clause_split, clause_split_fresh, clause_split_general_fresh,
        clause_split_general_search_fresh, ClauseSplitOutcome, ClauseSplitType,
        SplitDefinitionStore,
    };
    use crate::clauses::clause::Clause;
    use crate::clauses::clause_props::{CP_IS_SOS, CP_TYPE_AXIOM};
    use crate::clauses::clausesets::ClauseSet;
    use crate::clauses::eqn::Eqn;
    use crate::clauses::eqn_props::EP_IS_SPLIT_LIT;
    use crate::clauses::eqnlist::EqnList;
    use crate::terms::signature::{Signature, FP_CL_SPLIT_DEF};
    use crate::terms::termbanks::TermBank;
    use crate::terms::termtypes::{DerefType, Term};
    use crate::terms::typebanks::TypeBank;
    use std::collections::BTreeMap;

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

    fn typed_var(bank: &TermBank, f_code: i64) -> Term {
        let type_ = bank.signature().type_bank().default_type();
        bank.vars().var_assert_alloc(f_code, &type_)
    }

    fn lit(bank: &mut TermBank, left: &Term, right: &Term, positive: bool) -> Eqn {
        Eqn::alloc(left.clone(), right.clone(), bank, positive).unwrap()
    }

    fn split_literal_count(clause: &Clause, bank: &TermBank) -> usize {
        clause
            .literals()
            .as_slice()
            .iter()
            .filter(|literal| literal.query_prop(EP_IS_SPLIT_LIT) && literal.is_split_lit(bank))
            .count()
    }

    #[test]
    fn clause_split_fresh_splits_variable_disjoint_parts() {
        let mut bank = test_bank();
        let left_var = typed_var(&bank, -2);
        let right_var = typed_var(&bank, -4);
        let left_const = typed_const(&mut bank, "split_left");
        let right_const = typed_const(&mut bank, "split_right");
        let mut clause = Clause::alloc(EqnList::from_vec(vec![
            lit(&mut bank, &left_var, &left_const, true),
            lit(&mut bank, &right_var, &right_const, true),
        ]));
        clause.set_ident(7_001);
        clause.set_prop(CP_TYPE_AXIOM | CP_IS_SOS);

        let outcome = clause_split_fresh(&mut bank, clause, ClauseSplitType::GroundFull).unwrap();
        let ClauseSplitOutcome::Split(clauses, count) = outcome else {
            panic!("variable-disjoint clause should split");
        };

        assert_eq!(count, 3);
        assert_eq!(clauses.len(), 3);
        assert!(clauses[0].query_prop(CP_TYPE_AXIOM | CP_IS_SOS));
        assert!(clauses[1].query_prop(CP_TYPE_AXIOM | CP_IS_SOS));
        assert_eq!(split_literal_count(&clauses[0], &bank), 1);
        assert_eq!(split_literal_count(&clauses[1], &bank), 1);
        assert_eq!(clauses[2].ident(), 7_001);
        assert_eq!(clauses[2].literal_number(), 2);
        assert!(clauses[2]
            .literals()
            .as_slice()
            .iter()
            .all(Eqn::is_negative));
        assert_eq!(split_literal_count(&clauses[2], &bank), 2);
        for literal in clauses[2].literals().as_slice() {
            assert!(bank
                .signature()
                .query_prop(literal.pred_code_fo(&bank), FP_CL_SPLIT_DEF));
        }
    }

    #[test]
    fn clause_split_fresh_combines_ground_literals_for_ground_none() {
        let mut bank = test_bank();
        let variable = typed_var(&bank, -2);
        let first_const = typed_const(&mut bank, "split_ground_first");
        let second_const = typed_const(&mut bank, "split_ground_second");
        let third_const = typed_const(&mut bank, "split_ground_third");
        let clause = Clause::alloc(EqnList::from_vec(vec![
            lit(&mut bank, &first_const, &second_const, true),
            lit(&mut bank, &variable, &third_const, true),
        ]));

        let outcome = clause_split_fresh(&mut bank, clause, ClauseSplitType::GroundNone).unwrap();

        assert_eq!(outcome.split_count(), 0);
    }

    #[test]
    fn clause_split_fresh_splits_ground_literals_for_ground_full() {
        let mut bank = test_bank();
        let first_const = typed_const(&mut bank, "split_full_first");
        let second_const = typed_const(&mut bank, "split_full_second");
        let third_const = typed_const(&mut bank, "split_full_third");
        let fourth_const = typed_const(&mut bank, "split_full_fourth");
        let clause = Clause::alloc(EqnList::from_vec(vec![
            lit(&mut bank, &first_const, &second_const, true),
            lit(&mut bank, &third_const, &fourth_const, true),
        ]));

        let outcome = clause_split_fresh(&mut bank, clause, ClauseSplitType::GroundFull).unwrap();

        assert_eq!(outcome.split_count(), 3);
    }

    #[test]
    fn clause_split_fresh_refuses_existing_split_literal() {
        let mut bank = test_bank();
        let first_const = typed_const(&mut bank, "split_marked_first");
        let second_const = typed_const(&mut bank, "split_marked_second");
        let third_const = typed_const(&mut bank, "split_marked_third");
        let fourth_const = typed_const(&mut bank, "split_marked_fourth");
        let mut marked = lit(&mut bank, &first_const, &second_const, true);
        marked.set_prop(EP_IS_SPLIT_LIT);
        let clause = Clause::alloc(EqnList::from_vec(vec![
            marked,
            lit(&mut bank, &third_const, &fourth_const, true),
        ]));

        let outcome = clause_split_fresh(&mut bank, clause, ClauseSplitType::GroundFull).unwrap();

        assert_eq!(outcome.split_count(), 0);
        let ClauseSplitOutcome::Unsplit(clause) = outcome else {
            panic!("marked split literal should block resplitting");
        };
        assert!(clause_has_split_literal(&clause));
    }

    #[test]
    fn gen_ground_def_lit_inserts_shared_predicate_literal() {
        let mut bank = test_bank();
        let pred = bank.signature_mut().get_new_predicate_code(0);

        let literal = super::gen_ground_def_lit(&mut bank, pred, false).unwrap();

        assert!(literal.is_negative());
        assert!(literal.query_prop(EP_IS_SPLIT_LIT));
        assert!(literal.is_split_lit(&bank));
        assert_eq!(literal.right(), bank.true_term());
        assert!(bank.find(literal.left()).is_some());
    }

    #[test]
    fn gen_def_lit_builds_parameterized_split_literal() {
        let mut bank = test_bank();
        let variable = typed_var(&bank, -2);
        let pred = bank.signature_mut().get_new_predicate_code(1);

        let literal =
            super::gen_def_lit(&mut bank, pred, true, std::slice::from_ref(&variable)).unwrap();

        assert!(literal.is_positive());
        assert!(literal.query_prop(EP_IS_SPLIT_LIT));
        assert!(literal.is_split_lit(&bank));
        assert_eq!(literal.left().arity(), 1);
        assert_eq!(literal.left().argument(0).as_ref(), Some(&variable));
        assert_eq!(literal.right(), bank.true_term());
    }

    #[test]
    fn split_definition_literal_reuses_existing_constant_cell() {
        let mut bank = test_bank();
        let pred = bank.signature_mut().get_new_predicate_code(0);
        let first = super::gen_ground_def_lit(&mut bank, pred, true).unwrap();
        let second_term = Term::const_cell_alloc(pred);
        second_term.set_type(Some(bank.signature().type_bank().bool_type()));
        let second_term = bank.insert(&second_term, DerefType::Never).unwrap();

        assert_eq!(first.left(), &second_term);
    }

    #[test]
    fn clause_split_general_fresh_uses_split_variables_as_parameters() {
        let mut bank = test_bank();
        let shared = typed_var(&bank, -2);
        let left_var = typed_var(&bank, -4);
        let right_var = typed_var(&bank, -6);
        let clause = Clause::alloc(EqnList::from_vec(vec![
            lit(&mut bank, &shared, &left_var, true),
            lit(&mut bank, &shared, &right_var, true),
        ]));

        let unsplit =
            clause_split_fresh(&mut bank, clause.clone(), ClauseSplitType::GroundFull).unwrap();
        assert_eq!(unsplit.split_count(), 0);

        let outcome = clause_split_general_fresh(
            &mut bank,
            clause,
            ClauseSplitType::GroundNone,
            std::slice::from_ref(&shared),
        )
        .unwrap();
        let ClauseSplitOutcome::Split(clauses, count) = outcome else {
            panic!("shared parameter should enable splitting");
        };

        assert_eq!(count, 3);
        assert_eq!(clauses.len(), 3);
        assert_eq!(clauses[0].literals().as_slice()[0].left().arity(), 1);
        assert_eq!(
            clauses[0].literals().as_slice()[0].left().argument(0),
            Some(shared.clone())
        );
        assert_eq!(clauses[2].literal_number(), 2);
        assert!(clauses[2]
            .literals()
            .as_slice()
            .iter()
            .all(|literal| literal.left().arity() == 1));
    }

    #[test]
    fn clause_split_general_search_fresh_finds_parameter_subset() {
        let mut bank = test_bank();
        let shared = typed_var(&bank, -2);
        let left_var = typed_var(&bank, -4);
        let right_var = typed_var(&bank, -6);
        let mut clause = Clause::alloc(EqnList::from_vec(vec![
            lit(&mut bank, &shared, &left_var, true),
            lit(&mut bank, &shared, &right_var, true),
        ]));
        clause.set_ident(7_002);

        let outcome = clause_split_general_search_fresh(&mut bank, clause, 3).unwrap();
        let ClauseSplitOutcome::Split(clauses, count) = outcome else {
            panic!("search should find the shared parameter subset");
        };

        assert_eq!(count, 3);
        assert_eq!(clauses.len(), 3);
        assert_eq!(clauses[2].ident(), 7_002);
        assert_eq!(split_literal_count(&clauses[2], &bank), 2);
    }

    #[test]
    fn clause_split_reuses_nonfresh_variant_definitions() {
        let mut bank = test_bank();
        let first_const = typed_const(&mut bank, "split_reuse_first");
        let second_const = typed_const(&mut bank, "split_reuse_second");
        let first_var = typed_var(&bank, -2);
        let second_var = typed_var(&bank, -4);
        let third_var = typed_var(&bank, -6);
        let fourth_var = typed_var(&bank, -8);
        let first_clause = Clause::alloc(EqnList::from_vec(vec![
            lit(&mut bank, &first_var, &first_const, true),
            lit(&mut bank, &second_var, &second_const, true),
        ]));
        let second_clause = Clause::alloc(EqnList::from_vec(vec![
            lit(&mut bank, &third_var, &first_const, true),
            lit(&mut bank, &fourth_var, &second_const, true),
        ]));
        let mut definitions = ClauseSet::new();
        let mut predicates = BTreeMap::new();
        let mut store = SplitDefinitionStore::new(&mut definitions, &mut predicates);

        let first = clause_split(
            &mut bank,
            Some(&mut store),
            first_clause,
            ClauseSplitType::GroundFull,
            false,
        )
        .unwrap();
        let ClauseSplitOutcome::Split(first_clauses, first_count) = first else {
            panic!("first clause should split");
        };
        assert_eq!(first_count, 3);
        assert_eq!(first_clauses.len(), 3);
        assert_eq!(store.clauses().members(), 2);
        assert_eq!(store.predicates().len(), 2);
        let first_residual_preds = first_clauses[2]
            .literals()
            .as_slice()
            .iter()
            .map(|literal| literal.pred_code_fo(&bank))
            .collect::<Vec<_>>();

        let second = clause_split(
            &mut bank,
            Some(&mut store),
            second_clause,
            ClauseSplitType::GroundFull,
            false,
        )
        .unwrap();
        let ClauseSplitOutcome::Split(second_clauses, second_count) = second else {
            panic!("second clause should split");
        };

        assert_eq!(second_count, 3);
        assert_eq!(second_clauses.len(), 1);
        assert_eq!(store.clauses().members(), 2);
        assert_eq!(store.predicates().len(), 2);
        let second_residual_preds = second_clauses[0]
            .literals()
            .as_slice()
            .iter()
            .map(|literal| literal.pred_code_fo(&bank))
            .collect::<Vec<_>>();
        assert_eq!(second_residual_preds, first_residual_preds);
    }

    #[test]
    fn clause_set_split_clauses_fresh_moves_split_and_unsplit_clauses() {
        let mut bank = test_bank();
        let left_var = typed_var(&bank, -2);
        let right_var = typed_var(&bank, -4);
        let first_const = typed_const(&mut bank, "split_set_first");
        let second_const = typed_const(&mut bank, "split_set_second");
        let third_const = typed_const(&mut bank, "split_set_third");
        let fourth_const = typed_const(&mut bank, "split_set_fourth");
        let split = Clause::alloc(EqnList::from_vec(vec![
            lit(&mut bank, &left_var, &first_const, true),
            lit(&mut bank, &right_var, &second_const, true),
        ]));
        let mut unsplit = Clause::alloc(EqnList::from_vec(vec![lit(
            &mut bank,
            &third_const,
            &fourth_const,
            true,
        )]));
        unsplit.set_ident(7_003);
        let mut from_set = ClauseSet::from_clauses([split, unsplit]);
        let mut to_set = ClauseSet::new();

        let count = super::clause_set_split_clauses_fresh(
            &mut bank,
            &mut from_set,
            &mut to_set,
            ClauseSplitType::GroundFull,
        )
        .unwrap();

        assert_eq!(count, 3);
        assert!(from_set.is_empty());
        assert_eq!(to_set.members(), 4);
        assert!(to_set.find_by_id(7_003).is_some());
    }

    #[test]
    fn clause_set_split_clauses_reuses_nonfresh_definitions() {
        let mut bank = test_bank();
        let first_const = typed_const(&mut bank, "split_set_reuse_first");
        let second_const = typed_const(&mut bank, "split_set_reuse_second");
        let first_var = typed_var(&bank, -2);
        let second_var = typed_var(&bank, -4);
        let third_var = typed_var(&bank, -6);
        let fourth_var = typed_var(&bank, -8);
        let first_clause = Clause::alloc(EqnList::from_vec(vec![
            lit(&mut bank, &first_var, &first_const, true),
            lit(&mut bank, &second_var, &second_const, true),
        ]));
        let second_clause = Clause::alloc(EqnList::from_vec(vec![
            lit(&mut bank, &third_var, &first_const, true),
            lit(&mut bank, &fourth_var, &second_const, true),
        ]));
        let mut from_set = ClauseSet::from_clauses([first_clause, second_clause]);
        let mut to_set = ClauseSet::new();
        let mut definitions = ClauseSet::new();
        let mut predicates = BTreeMap::new();
        let mut store = SplitDefinitionStore::new(&mut definitions, &mut predicates);

        let count = super::clause_set_split_clauses(
            &mut bank,
            Some(&mut store),
            &mut from_set,
            &mut to_set,
            ClauseSplitType::GroundFull,
            false,
        )
        .unwrap();

        assert_eq!(count, 6);
        assert!(from_set.is_empty());
        assert_eq!(to_set.members(), 4);
        assert_eq!(store.clauses().members(), 2);
        assert_eq!(store.predicates().len(), 2);
    }

    #[test]
    fn clause_set_split_clauses_general_fresh_uses_search() {
        let mut bank = test_bank();
        let shared = typed_var(&bank, -2);
        let left_var = typed_var(&bank, -4);
        let right_var = typed_var(&bank, -6);
        let clause = Clause::alloc(EqnList::from_vec(vec![
            lit(&mut bank, &shared, &left_var, true),
            lit(&mut bank, &shared, &right_var, true),
        ]));
        let mut from_set = ClauseSet::from_clauses([clause]);
        let mut to_set = ClauseSet::new();

        let count =
            super::clause_set_split_clauses_general_fresh(&mut bank, &mut from_set, &mut to_set, 3)
                .unwrap();

        assert_eq!(count, 3);
        assert!(from_set.is_empty());
        assert_eq!(to_set.members(), 3);
    }
}
