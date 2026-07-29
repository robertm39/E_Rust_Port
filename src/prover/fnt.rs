//! Explicit, bounded typed finite-model search.
//!
//! This worker is deliberately separate from saturation and portfolio
//! scheduling.  It imports the clausifier's first-order typed clauses, grows
//! one backend-neutral incremental SAT encoding, checks every decoded model
//! against the clauses, and only then renders a complete TPTP interpretation.

#![cfg_attr(not(feature = "cadical-static"), allow(dead_code))]

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::time::{Duration, Instant};

use crate::clauses::clause_props::CP_TYPE_NEG_CONJECTURE;
use crate::clauses::clausesets::ClauseSet;
use crate::clauses::satservice::{
    IncrementalSatService, SatSolveOptions, SatSolveOutcome, SatUnknownReason,
};
use crate::terms::functypes::FunCode;
use crate::terms::signature::{FP_DISTINCT_PROP, FP_INTERPRETED};
use crate::terms::simpletypes::{sort_is_interpreted, TypeConsCode, ST_BOOL};
use crate::terms::termbanks::TermBank;
use crate::terms::termtypes::Term;

type SortId = TypeConsCode;
type SymbolId = FunCode;
type VariableId = FunCode;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FiniteModelConfig {
    pub enabled: bool,
    pub maximum_size: usize,
    pub maximum_vectors: usize,
    pub maximum_ground_instances: usize,
    pub maximum_clauses: usize,
    pub maximum_variables: usize,
    pub sat_timeout: Duration,
}

impl Default for FiniteModelConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            maximum_size: 3,
            maximum_vectors: 2_048,
            maximum_ground_instances: 5_000_000,
            maximum_clauses: 10_000_000,
            maximum_variables: 10_000_000,
            sat_timeout: Duration::from_secs(5),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum FiniteModelOutcome {
    Model(String),
    BoundsExhausted,
    ResourceOut(String),
    Inappropriate(String),
    Error(String),
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
struct SymbolType {
    arguments: Vec<SortId>,
    result: SortId,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
enum InputTerm {
    Variable {
        code: VariableId,
        sort: SortId,
    },
    Application {
        symbol: SymbolId,
        sort: SortId,
        arguments: Vec<Self>,
    },
}

impl InputTerm {
    const fn sort(&self) -> SortId {
        match self {
            Self::Variable { sort, .. } | Self::Application { sort, .. } => *sort,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum InputLiteral {
    Truth(bool),
    Predicate {
        symbol: SymbolId,
        arguments: Vec<InputTerm>,
        negated: bool,
    },
    Equality {
        left: InputTerm,
        right: InputTerm,
        negated: bool,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct InputClause {
    name: String,
    variables: Vec<(VariableId, SortId)>,
    literals: Vec<InputLiteral>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Problem {
    clauses: Vec<InputClause>,
    functions: BTreeMap<SymbolId, SymbolType>,
    predicates: BTreeMap<SymbolId, SymbolType>,
    sorts: Vec<SortId>,
    symbol_names: BTreeMap<SymbolId, String>,
    sort_names: BTreeMap<SortId, String>,
    has_conjecture: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
enum GroundTermKey {
    Element(SortId, usize),
    Application(SymbolId, SortId, Vec<Self>),
}

#[derive(Clone, Debug)]
struct ValueVector {
    fixed: Option<usize>,
    variables: Vec<i32>,
}

impl ValueVector {
    fn selections(&self) -> Vec<(usize, Vec<i32>)> {
        self.fixed.map_or_else(
            || {
                self.variables
                    .iter()
                    .copied()
                    .enumerate()
                    .map(|(value, variable)| (value, vec![variable]))
                    .collect()
            },
            |value| vec![(value, Vec::new())],
        )
    }
}

#[derive(Clone, Copy, Debug)]
enum TruthValue {
    Constant(bool),
    Literal(i32),
}

impl TruthValue {
    const fn negate(self) -> Self {
        match self {
            Self::Constant(value) => Self::Constant(!value),
            Self::Literal(literal) => Self::Literal(-literal),
        }
    }
}

#[derive(Clone, Debug)]
struct DecodedModel {
    functions: BTreeMap<(SymbolId, Vec<usize>), usize>,
    predicates: BTreeMap<(SymbolId, Vec<usize>), bool>,
}

struct Encoding<'a, S> {
    problem: &'a Problem,
    config: &'a FiniteModelConfig,
    service: S,
    next_variable: i32,
    clause_count: usize,
    activity: BTreeMap<(SortId, usize), i32>,
    function_tables: BTreeMap<(SymbolId, Vec<usize>, usize), i32>,
    predicate_tables: BTreeMap<(SymbolId, Vec<usize>), i32>,
    term_cache: BTreeMap<GroundTermKey, ValueVector>,
    atom_cache: BTreeMap<(GroundTermKey, GroundTermKey), TruthValue>,
    predicate_cache: BTreeMap<(SymbolId, Vec<GroundTermKey>), i32>,
    grounded: BTreeSet<(usize, Vec<usize>)>,
}

impl<'a, S: IncrementalSatService> Encoding<'a, S> {
    fn new(
        problem: &'a Problem,
        config: &'a FiniteModelConfig,
        service: S,
    ) -> Result<Self, String> {
        let mut encoding = Self {
            problem,
            config,
            service,
            next_variable: 1,
            clause_count: 0,
            activity: BTreeMap::new(),
            function_tables: BTreeMap::new(),
            predicate_tables: BTreeMap::new(),
            term_cache: BTreeMap::new(),
            atom_cache: BTreeMap::new(),
            predicate_cache: BTreeMap::new(),
            grounded: BTreeSet::new(),
        };
        encoding.build_global_tables()?;
        Ok(encoding)
    }

    fn variable_count(&self) -> usize {
        usize::try_from(self.next_variable - 1).unwrap_or(usize::MAX)
    }

    fn new_variable(&mut self) -> Result<i32, String> {
        if self.variable_count() >= self.config.maximum_variables {
            return Err(format!(
                "propositional variable limit {} reached",
                self.config.maximum_variables
            ));
        }
        let variable = self.next_variable;
        self.next_variable = self
            .next_variable
            .checked_add(1)
            .ok_or_else(|| "propositional variable space exhausted".to_owned())?;
        Ok(variable)
    }

    fn add_clause(&mut self, clause: &[i32]) -> Result<(), String> {
        if self.clause_count >= self.config.maximum_clauses {
            return Err(format!(
                "propositional clause limit {} reached",
                self.config.maximum_clauses
            ));
        }
        self.service
            .add_clause(clause)
            .map_err(|error| error.to_string())?;
        self.clause_count += 1;
        Ok(())
    }

    fn exactly_one(&mut self, variables: &[i32]) -> Result<(), String> {
        self.add_clause(variables)?;
        for (index, &left) in variables.iter().enumerate() {
            for &right in &variables[index + 1..] {
                self.add_clause(&[-left, -right])?;
            }
        }
        Ok(())
    }

    fn active(&self, sort: SortId, value: usize) -> Result<i32, String> {
        self.activity
            .get(&(sort, value))
            .copied()
            .ok_or_else(|| "missing domain-activity variable".to_owned())
    }

    fn build_global_tables(&mut self) -> Result<(), String> {
        for &sort in &self.problem.sorts {
            for value in 0..self.config.maximum_size {
                let variable = self.new_variable()?;
                self.activity.insert((sort, value), variable);
            }
            self.add_clause(&[self.active(sort, 0)?])?;
            for value in 1..self.config.maximum_size {
                self.add_clause(&[-self.active(sort, value)?, self.active(sort, value - 1)?])?;
            }
        }

        for (&symbol, signature) in &self.problem.functions {
            let arity = signature.arguments.len();
            let maximum = self.config.maximum_size;
            for_each_uniform_tuple(arity, maximum, |arguments| {
                let mut row = Vec::with_capacity(maximum);
                for output in 0..maximum {
                    let variable = self.new_variable()?;
                    self.function_tables
                        .insert((symbol, arguments.to_vec(), output), variable);
                    row.push(variable);
                }
                self.exactly_one(&row)?;
                for (output, variable) in row.into_iter().enumerate() {
                    let mut clause = Vec::with_capacity(arguments.len() + 2);
                    for (&sort, &value) in signature.arguments.iter().zip(arguments) {
                        clause.push(-self.active(sort, value)?);
                    }
                    clause.push(self.active(signature.result, output)?);
                    clause.push(-variable);
                    self.add_clause(&clause)?;
                }
                Ok(())
            })?;
        }

        for (&symbol, signature) in &self.problem.predicates {
            for_each_uniform_tuple(
                signature.arguments.len(),
                self.config.maximum_size,
                |arguments| {
                    let variable = self.new_variable()?;
                    self.predicate_tables
                        .insert((symbol, arguments.to_vec()), variable);
                    Ok(())
                },
            )?;
        }
        Ok(())
    }

    fn table_row(&self, symbol: SymbolId, arguments: &[usize]) -> Result<Vec<i32>, String> {
        (0..self.config.maximum_size)
            .map(|output| {
                self.function_tables
                    .get(&(symbol, arguments.to_vec(), output))
                    .copied()
                    .ok_or_else(|| "missing function-table row".to_owned())
            })
            .collect()
    }

    fn ground_term_key(
        term: &InputTerm,
        assignment: &BTreeMap<VariableId, usize>,
    ) -> Result<GroundTermKey, String> {
        match term {
            InputTerm::Variable { code, sort } => assignment
                .get(code)
                .copied()
                .map(|value| GroundTermKey::Element(*sort, value))
                .ok_or_else(|| "ground assignment omitted a clause variable".to_owned()),
            InputTerm::Application {
                symbol,
                sort,
                arguments,
            } => arguments
                .iter()
                .map(|argument| Self::ground_term_key(argument, assignment))
                .collect::<Result<Vec<_>, _>>()
                .map(|arguments| GroundTermKey::Application(*symbol, *sort, arguments)),
        }
    }

    fn term_values(
        &mut self,
        term: &InputTerm,
        assignment: &BTreeMap<VariableId, usize>,
    ) -> Result<ValueVector, String> {
        if let InputTerm::Variable { code, .. } = term {
            return assignment
                .get(code)
                .copied()
                .map(|value| ValueVector {
                    fixed: Some(value),
                    variables: Vec::new(),
                })
                .ok_or_else(|| "ground assignment omitted a clause variable".to_owned());
        }
        let key = Self::ground_term_key(term, assignment)?;
        if let Some(cached) = self.term_cache.get(&key) {
            return Ok(cached.clone());
        }
        let InputTerm::Application {
            symbol, arguments, ..
        } = term
        else {
            return Err("unexpected variable term".to_owned());
        };
        let argument_values = arguments
            .iter()
            .map(|argument| self.term_values(argument, assignment))
            .collect::<Result<Vec<_>, _>>()?;
        let result = if argument_values
            .iter()
            .all(|argument| argument.fixed.is_some())
        {
            let indices = argument_values
                .iter()
                .filter_map(|argument| argument.fixed)
                .collect::<Vec<_>>();
            ValueVector {
                fixed: None,
                variables: self.table_row(*symbol, &indices)?,
            }
        } else {
            let outputs = (0..self.config.maximum_size)
                .map(|_| self.new_variable())
                .collect::<Result<Vec<_>, _>>()?;
            self.exactly_one(&outputs)?;
            let selections = argument_values
                .iter()
                .map(ValueVector::selections)
                .collect::<Vec<_>>();
            for_each_selection(&selections, |indices, selectors| {
                let row = self.table_row(*symbol, indices)?;
                for (&table, &output) in row.iter().zip(&outputs) {
                    let mut forward = selectors.iter().map(|lit| -*lit).collect::<Vec<_>>();
                    forward.extend([-table, output]);
                    self.add_clause(&forward)?;
                    let mut reverse = selectors.iter().map(|lit| -*lit).collect::<Vec<_>>();
                    reverse.extend([-output, table]);
                    self.add_clause(&reverse)?;
                }
                Ok(())
            })?;
            ValueVector {
                fixed: None,
                variables: outputs,
            }
        };
        self.term_cache.insert(key, result.clone());
        Ok(result)
    }

    fn predicate_truth(
        &mut self,
        symbol: SymbolId,
        terms: &[InputTerm],
        assignment: &BTreeMap<VariableId, usize>,
    ) -> Result<TruthValue, String> {
        let keys = terms
            .iter()
            .map(|term| Self::ground_term_key(term, assignment))
            .collect::<Result<Vec<_>, _>>()?;
        if let Some(&cached) = self.predicate_cache.get(&(symbol, keys.clone())) {
            return Ok(TruthValue::Literal(cached));
        }
        let arguments = terms
            .iter()
            .map(|term| self.term_values(term, assignment))
            .collect::<Result<Vec<_>, _>>()?;
        if arguments.iter().all(|argument| argument.fixed.is_some()) {
            let indices = arguments
                .iter()
                .filter_map(|argument| argument.fixed)
                .collect::<Vec<_>>();
            return self
                .predicate_tables
                .get(&(symbol, indices))
                .copied()
                .map(TruthValue::Literal)
                .ok_or_else(|| "missing predicate-table row".to_owned());
        }
        let truth = self.new_variable()?;
        let selections = arguments
            .iter()
            .map(ValueVector::selections)
            .collect::<Vec<_>>();
        for_each_selection(&selections, |indices, selectors| {
            let table = self
                .predicate_tables
                .get(&(symbol, indices.to_vec()))
                .copied()
                .ok_or_else(|| "missing predicate-table row".to_owned())?;
            let mut forward = selectors.iter().map(|lit| -*lit).collect::<Vec<_>>();
            forward.extend([-truth, table]);
            self.add_clause(&forward)?;
            let mut reverse = selectors.iter().map(|lit| -*lit).collect::<Vec<_>>();
            reverse.extend([truth, -table]);
            self.add_clause(&reverse)
        })?;
        self.predicate_cache.insert((symbol, keys), truth);
        Ok(TruthValue::Literal(truth))
    }

    fn equality_truth(
        &mut self,
        left: &InputTerm,
        right: &InputTerm,
        assignment: &BTreeMap<VariableId, usize>,
    ) -> Result<TruthValue, String> {
        let left_key = Self::ground_term_key(left, assignment)?;
        let right_key = Self::ground_term_key(right, assignment)?;
        let key = if left_key <= right_key {
            (left_key, right_key)
        } else {
            (right_key, left_key)
        };
        if let Some(&cached) = self.atom_cache.get(&key) {
            return Ok(cached);
        }
        let left_values = self.term_values(left, assignment)?;
        let right_values = self.term_values(right, assignment)?;
        let result = if let (Some(left), Some(right)) = (left_values.fixed, right_values.fixed) {
            TruthValue::Constant(left == right)
        } else {
            let truth = self.new_variable()?;
            for (left_value, left_selectors) in left_values.selections() {
                for (right_value, right_selectors) in right_values.selections() {
                    let mut clause = left_selectors
                        .iter()
                        .chain(&right_selectors)
                        .map(|lit| -*lit)
                        .collect::<Vec<_>>();
                    clause.push(if left_value == right_value {
                        truth
                    } else {
                        -truth
                    });
                    self.add_clause(&clause)?;
                }
            }
            TruthValue::Literal(truth)
        };
        self.atom_cache.insert(key, result);
        Ok(result)
    }

    fn literal_truth(
        &mut self,
        literal: &InputLiteral,
        assignment: &BTreeMap<VariableId, usize>,
    ) -> Result<TruthValue, String> {
        match literal {
            InputLiteral::Truth(value) => Ok(TruthValue::Constant(*value)),
            InputLiteral::Predicate {
                symbol,
                arguments,
                negated,
            } => {
                let value = self.predicate_truth(*symbol, arguments, assignment)?;
                Ok(if *negated { value.negate() } else { value })
            }
            InputLiteral::Equality {
                left,
                right,
                negated,
            } => {
                let value = self.equality_truth(left, right, assignment)?;
                Ok(if *negated { value.negate() } else { value })
            }
        }
    }

    fn ground_clause(
        &mut self,
        clause_index: usize,
        clause: &InputClause,
        values: &[usize],
    ) -> Result<(), String> {
        let key = (clause_index, values.to_vec());
        if self.grounded.contains(&key) {
            return Ok(());
        }
        if self.grounded.len() >= self.config.maximum_ground_instances {
            return Err(format!(
                "ground-instance limit {} reached",
                self.config.maximum_ground_instances
            ));
        }
        let assignment = clause
            .variables
            .iter()
            .zip(values)
            .map(|(&(variable, _), &value)| (variable, value))
            .collect::<BTreeMap<_, _>>();
        let mut propositional = clause
            .variables
            .iter()
            .zip(values)
            .map(|(&(_, sort), &value)| self.active(sort, value).map(|lit| -lit))
            .collect::<Result<Vec<_>, _>>()?;
        for literal in &clause.literals {
            match self.literal_truth(literal, &assignment)? {
                TruthValue::Constant(true) => {
                    self.grounded.insert(key);
                    return Ok(());
                }
                TruthValue::Constant(false) => {}
                TruthValue::Literal(value) => propositional.push(value),
            }
        }
        self.add_clause(&propositional)?;
        self.grounded.insert(key);
        Ok(())
    }

    fn extend_grounding(&mut self, sizes: &BTreeMap<SortId, usize>) -> Result<usize, String> {
        let before = self.grounded.len();
        for clause_index in 0..self.problem.clauses.len() {
            let clause = self.problem.clauses[clause_index].clone();
            let lengths = clause
                .variables
                .iter()
                .map(|(_, sort)| {
                    sizes
                        .get(sort)
                        .copied()
                        .ok_or_else(|| "domain vector omitted a sort".to_owned())
                })
                .collect::<Result<Vec<_>, _>>()?;
            for_each_tuple(&lengths, |values| {
                self.ground_clause(clause_index, &clause, values)
            })?;
        }
        Ok(self.grounded.len() - before)
    }

    fn assumptions(&self, sizes: &BTreeMap<SortId, usize>) -> Result<Vec<i32>, String> {
        let mut assumptions =
            Vec::with_capacity(self.problem.sorts.len() * self.config.maximum_size);
        for &sort in &self.problem.sorts {
            let size = sizes
                .get(&sort)
                .copied()
                .ok_or_else(|| "domain vector omitted a sort".to_owned())?;
            for value in 0..self.config.maximum_size {
                let active = self.active(sort, value)?;
                assumptions.push(if value < size { active } else { -active });
            }
        }
        Ok(assumptions)
    }

    fn solve(&mut self, sizes: &BTreeMap<SortId, usize>) -> SatSolveOutcome {
        let assumptions = match self.assumptions(sizes) {
            Ok(assumptions) => assumptions,
            Err(message) => {
                return SatSolveOutcome::Error(
                    crate::clauses::satservice::SatServiceError::Backend(message),
                );
            }
        };
        self.service.solve(
            &assumptions,
            &SatSolveOptions {
                deadline: Some(self.config.sat_timeout),
                external_stop: Some(super::umlaut::finite_model_stop_requested),
                ..SatSolveOptions::default()
            },
        )
    }

    fn decode(
        &self,
        sizes: &BTreeMap<SortId, usize>,
        model: &[i32],
    ) -> Result<DecodedModel, String> {
        if model.len() < self.variable_count() {
            return Err(format!(
                "SAT backend returned an incomplete model ({} < {})",
                model.len(),
                self.variable_count()
            ));
        }
        for (index, &literal) in model.iter().take(self.variable_count()).enumerate() {
            let expected = i32::try_from(index + 1)
                .map_err(|_| "SAT model index cannot be represented".to_owned())?;
            if literal.checked_abs() != Some(expected) {
                return Err("SAT backend model is not an ordered complete assignment".to_owned());
            }
        }
        let selected = |variable: i32| {
            usize::try_from(variable - 1)
                .ok()
                .and_then(|index| model.get(index))
                .is_some_and(|literal| *literal > 0)
        };
        let mut functions = BTreeMap::new();
        for (&symbol, signature) in &self.problem.functions {
            let lengths = signature
                .arguments
                .iter()
                .map(|sort| sizes[sort])
                .collect::<Vec<_>>();
            for_each_tuple(&lengths, |arguments| {
                let mut outputs = (0..self.config.maximum_size).filter(|&output| {
                    self.function_tables
                        .get(&(symbol, arguments.to_vec(), output))
                        .is_some_and(|&variable| selected(variable))
                });
                let output = outputs
                    .next()
                    .ok_or_else(|| "function row has no selected output".to_owned())?;
                if outputs.next().is_some() {
                    return Err("function row has multiple selected outputs".to_owned());
                }
                if output >= sizes[&signature.result] {
                    return Err("function row maps outside its active result sort".to_owned());
                }
                functions.insert((symbol, arguments.to_vec()), output);
                Ok(())
            })?;
        }
        let mut predicates = BTreeMap::new();
        for (&symbol, signature) in &self.problem.predicates {
            let lengths = signature
                .arguments
                .iter()
                .map(|sort| sizes[sort])
                .collect::<Vec<_>>();
            for_each_tuple(&lengths, |arguments| {
                let variable = self
                    .predicate_tables
                    .get(&(symbol, arguments.to_vec()))
                    .copied()
                    .ok_or_else(|| "missing predicate row while decoding".to_owned())?;
                predicates.insert((symbol, arguments.to_vec()), selected(variable));
                Ok(())
            })?;
        }
        Ok(DecodedModel {
            functions,
            predicates,
        })
    }
}

#[expect(
    clippy::too_many_lines,
    reason = "the importer keeps all fail-closed clause and symbol checks together"
)]
fn import_problem(
    clauses: &ClauseSet,
    bank: &TermBank,
    mut has_conjecture: bool,
) -> Result<Problem, String> {
    let signature = bank.signature();
    let mut functions = BTreeMap::new();
    let mut predicates = BTreeMap::new();
    let mut sorts = BTreeSet::new();
    let mut symbol_names = BTreeMap::new();
    let mut imported_clauses = Vec::new();

    for (clause_index, clause) in clauses.iter().enumerate() {
        has_conjecture |= clause.query_prop(CP_TYPE_NEG_CONJECTURE);
        let mut variables = BTreeMap::new();
        let mut literals = Vec::new();
        for literal in clause.literals().as_slice() {
            let imported = if literal.is_equ_lit(bank) {
                let left = import_term(
                    literal.left(),
                    bank,
                    &mut functions,
                    &mut sorts,
                    &mut symbol_names,
                    &mut variables,
                )?;
                let right = import_term(
                    literal.right(),
                    bank,
                    &mut functions,
                    &mut sorts,
                    &mut symbol_names,
                    &mut variables,
                )?;
                if left.sort() != right.sort() {
                    return Err("clausifier produced an ill-sorted equality".to_owned());
                }
                InputLiteral::Equality {
                    left,
                    right,
                    negated: literal.is_negative(),
                }
            } else if literal.left() == bank.true_term() {
                InputLiteral::Truth(literal.is_positive())
            } else {
                let atom = literal.left();
                validate_symbol(atom.f_code(), bank)?;
                let arguments = atom
                    .argument_clones()
                    .into_iter()
                    .map(|argument| {
                        argument
                            .ok_or_else(|| "predicate has an uninitialized argument".to_owned())
                            .and_then(|argument| {
                                import_term(
                                    &argument,
                                    bank,
                                    &mut functions,
                                    &mut sorts,
                                    &mut symbol_names,
                                    &mut variables,
                                )
                            })
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                let symbol_type = SymbolType {
                    arguments: arguments.iter().map(InputTerm::sort).collect(),
                    result: ST_BOOL,
                };
                register_symbol(
                    &mut predicates,
                    &functions,
                    atom.f_code(),
                    &symbol_type,
                    "predicate",
                )?;
                symbol_names.insert(
                    atom.f_code(),
                    signature
                        .find_name(atom.f_code())
                        .ok_or_else(|| "predicate symbol has no printable name".to_owned())?
                        .to_owned(),
                );
                InputLiteral::Predicate {
                    symbol: atom.f_code(),
                    arguments,
                    negated: literal.is_negative(),
                }
            };
            literals.push(imported);
        }
        imported_clauses.push(InputClause {
            name: clause
                .info()
                .and_then(crate::clauses::clauseinfo::ClauseInfo::name)
                .map_or_else(|| format!("clause_{}", clause_index + 1), ToOwned::to_owned),
            variables: variables.into_iter().collect(),
            literals,
        });
    }

    if sorts.is_empty() {
        sorts.insert(crate::terms::simpletypes::ST_INDIVIDUALS);
    }
    if let Some(symbol) = functions
        .keys()
        .find(|symbol| predicates.contains_key(symbol))
    {
        return Err(format!(
            "symbol {symbol} is used as both a function and predicate"
        ));
    }
    let sort_names = sorts
        .iter()
        .copied()
        .map(|sort| {
            signature
                .type_bank()
                .find_tc_name(sort)
                .map(|name| (sort, name.to_owned()))
                .ok_or_else(|| format!("sort code {sort} has no printable name"))
        })
        .collect::<Result<BTreeMap<_, _>, _>>()?;
    Ok(Problem {
        clauses: imported_clauses,
        functions,
        predicates,
        sorts: sorts.into_iter().collect(),
        symbol_names,
        sort_names,
        has_conjecture,
    })
}

fn import_term(
    term: &Term,
    bank: &TermBank,
    functions: &mut BTreeMap<SymbolId, SymbolType>,
    sorts: &mut BTreeSet<SortId>,
    symbol_names: &mut BTreeMap<SymbolId, String>,
    variables: &mut BTreeMap<VariableId, SortId>,
) -> Result<InputTerm, String> {
    let type_ = term
        .type_()
        .ok_or_else(|| "clausified term has no type".to_owned())?;
    if type_.is_arrow() || type_.is_bool() || type_.is_kind() || sort_is_interpreted(type_.f_code())
    {
        return Err(
            "higher-order, Boolean-term, kind, and arithmetic terms are unsupported".to_owned(),
        );
    }
    let sort = type_.f_code();
    sorts.insert(sort);
    if term.is_free_var() {
        if let Some(previous) = variables.insert(term.f_code(), sort) {
            if previous != sort {
                return Err("one clause variable has inconsistent sorts".to_owned());
            }
        }
        return Ok(InputTerm::Variable {
            code: term.f_code(),
            sort,
        });
    }
    if term.is_db_var() || term.is_phony_app() || term.is_lambda() {
        return Err("higher-order term structure is unsupported".to_owned());
    }
    validate_symbol(term.f_code(), bank)?;
    let arguments = term
        .argument_clones()
        .into_iter()
        .map(|argument| {
            argument
                .ok_or_else(|| "function has an uninitialized argument".to_owned())
                .and_then(|argument| {
                    import_term(&argument, bank, functions, sorts, symbol_names, variables)
                })
        })
        .collect::<Result<Vec<_>, _>>()?;
    register_symbol(
        functions,
        &BTreeMap::new(),
        term.f_code(),
        &SymbolType {
            arguments: arguments.iter().map(InputTerm::sort).collect(),
            result: sort,
        },
        "function",
    )?;
    symbol_names.insert(
        term.f_code(),
        bank.signature()
            .find_name(term.f_code())
            .ok_or_else(|| "function symbol has no printable name".to_owned())?
            .to_owned(),
    );
    Ok(InputTerm::Application {
        symbol: term.f_code(),
        sort,
        arguments,
    })
}

fn validate_symbol(symbol: SymbolId, bank: &TermBank) -> Result<(), String> {
    let signature = bank.signature();
    if symbol <= signature.internal_symbols()
        || signature.query_prop(symbol, FP_INTERPRETED | FP_DISTINCT_PROP)
        || signature.is_logical_symbol(symbol)
        || signature.is_special(symbol)
    {
        return Err(format!(
            "interpreted or special symbol {} is unsupported",
            signature.find_name(symbol).unwrap_or("<unnamed>")
        ));
    }
    Ok(())
}

fn register_symbol(
    target: &mut BTreeMap<SymbolId, SymbolType>,
    other: &BTreeMap<SymbolId, SymbolType>,
    symbol: SymbolId,
    symbol_type: &SymbolType,
    kind: &str,
) -> Result<(), String> {
    if other.contains_key(&symbol) {
        return Err(format!("symbol {symbol} is both a function and predicate"));
    }
    if target
        .insert(symbol, symbol_type.clone())
        .is_some_and(|previous| previous != *symbol_type)
    {
        return Err(format!("symbol {symbol} has inconsistent {kind} types"));
    }
    Ok(())
}

fn for_each_uniform_tuple(
    arity: usize,
    maximum: usize,
    callback: impl FnMut(&[usize]) -> Result<(), String>,
) -> Result<(), String> {
    for_each_tuple(&vec![maximum; arity], callback)
}

fn for_each_tuple(
    lengths: &[usize],
    mut callback: impl FnMut(&[usize]) -> Result<(), String>,
) -> Result<(), String> {
    fn recurse(
        lengths: &[usize],
        tuple: &mut Vec<usize>,
        callback: &mut impl FnMut(&[usize]) -> Result<(), String>,
    ) -> Result<(), String> {
        if tuple.len() == lengths.len() {
            return callback(tuple);
        }
        let length = lengths[tuple.len()];
        for value in 0..length {
            tuple.push(value);
            recurse(lengths, tuple, callback)?;
            tuple.pop();
        }
        Ok(())
    }
    recurse(
        lengths,
        &mut Vec::with_capacity(lengths.len()),
        &mut callback,
    )
}

fn for_each_selection(
    selections: &[Vec<(usize, Vec<i32>)>],
    mut callback: impl FnMut(&[usize], &[i32]) -> Result<(), String>,
) -> Result<(), String> {
    fn recurse(
        selections: &[Vec<(usize, Vec<i32>)>],
        indices: &mut Vec<usize>,
        literals: &mut Vec<i32>,
        callback: &mut impl FnMut(&[usize], &[i32]) -> Result<(), String>,
    ) -> Result<(), String> {
        if indices.len() == selections.len() {
            return callback(indices, literals);
        }
        for (value, selected) in &selections[indices.len()] {
            indices.push(*value);
            let previous = literals.len();
            literals.extend(selected);
            recurse(selections, indices, literals, callback)?;
            literals.truncate(previous);
            indices.pop();
        }
        Ok(())
    }
    recurse(
        selections,
        &mut Vec::with_capacity(selections.len()),
        &mut Vec::new(),
        &mut callback,
    )
}

fn domain_vectors(
    sorts: &[SortId],
    maximum: usize,
    limit: usize,
) -> Result<Vec<BTreeMap<SortId, usize>>, String> {
    let mut result = Vec::new();
    for total in sorts.len()..=sorts.len().saturating_mul(maximum) {
        fn recurse(
            sorts: &[SortId],
            maximum: usize,
            total: usize,
            values: &mut Vec<usize>,
            limit: usize,
            result: &mut Vec<BTreeMap<SortId, usize>>,
        ) {
            if result.len() >= limit {
                return;
            }
            if values.len() == sorts.len() {
                if values.iter().sum::<usize>() == total {
                    result.push(sorts.iter().copied().zip(values.iter().copied()).collect());
                }
                return;
            }
            let used = values.iter().sum::<usize>();
            let remaining_slots = sorts.len() - values.len() - 1;
            for value in 1..=maximum {
                let next = used + value;
                if next + remaining_slots <= total
                    && next + remaining_slots.saturating_mul(maximum) >= total
                {
                    values.push(value);
                    recurse(sorts, maximum, total, values, limit, result);
                    values.pop();
                }
            }
        }
        recurse(sorts, maximum, total, &mut Vec::new(), limit, &mut result);
        if result.len() >= limit {
            break;
        }
    }
    if result.is_empty() {
        return Err("finite-model vector limit is zero".to_owned());
    }
    Ok(result)
}

fn evaluate_term(
    term: &InputTerm,
    variables: &BTreeMap<VariableId, usize>,
    model: &DecodedModel,
) -> Result<usize, String> {
    match term {
        InputTerm::Variable { code, .. } => variables
            .get(code)
            .copied()
            .ok_or_else(|| "validator assignment omitted a variable".to_owned()),
        InputTerm::Application {
            symbol, arguments, ..
        } => {
            let arguments = arguments
                .iter()
                .map(|argument| evaluate_term(argument, variables, model))
                .collect::<Result<Vec<_>, _>>()?;
            model
                .functions
                .get(&(*symbol, arguments))
                .copied()
                .ok_or_else(|| "decoded model omitted a function row".to_owned())
        }
    }
}

fn evaluate_literal(
    literal: &InputLiteral,
    variables: &BTreeMap<VariableId, usize>,
    model: &DecodedModel,
) -> Result<bool, String> {
    match literal {
        InputLiteral::Truth(value) => Ok(*value),
        InputLiteral::Predicate {
            symbol,
            arguments,
            negated,
        } => {
            let arguments = arguments
                .iter()
                .map(|argument| evaluate_term(argument, variables, model))
                .collect::<Result<Vec<_>, _>>()?;
            let value = model
                .predicates
                .get(&(*symbol, arguments))
                .copied()
                .ok_or_else(|| "decoded model omitted a predicate row".to_owned())?;
            Ok(if *negated { !value } else { value })
        }
        InputLiteral::Equality {
            left,
            right,
            negated,
        } => {
            let value =
                evaluate_term(left, variables, model)? == evaluate_term(right, variables, model)?;
            Ok(if *negated { !value } else { value })
        }
    }
}

fn validate_model(
    problem: &Problem,
    sizes: &BTreeMap<SortId, usize>,
    model: &DecodedModel,
) -> Result<(), String> {
    for clause in &problem.clauses {
        let lengths = clause
            .variables
            .iter()
            .map(|(_, sort)| sizes[sort])
            .collect::<Vec<_>>();
        for_each_tuple(&lengths, |values| {
            let variables = clause
                .variables
                .iter()
                .zip(values)
                .map(|(&(variable, _), &value)| (variable, value))
                .collect::<BTreeMap<_, _>>();
            for literal in &clause.literals {
                if evaluate_literal(literal, &variables, model)? {
                    return Ok(());
                }
            }
            Err(format!(
                "decoded SAT model falsifies {} at {values:?}",
                clause.name
            ))
        })?;
    }
    Ok(())
}

fn safe_fragment(name: &str) -> String {
    let value = name
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '_' {
                character
            } else {
                '_'
            }
        })
        .collect::<String>();
    if value.starts_with(char::is_alphabetic) {
        value
    } else {
        format!("s_{value}")
    }
}

fn tptp_symbol(name: &str) -> String {
    let valid = name
        .strip_prefix('$')
        .unwrap_or(name)
        .chars()
        .enumerate()
        .all(|(index, character)| {
            if index == 0 {
                character.is_ascii_lowercase()
            } else {
                character.is_ascii_alphanumeric() || character == '_'
            }
        });
    if valid {
        name.to_owned()
    } else {
        format!("'{}'", name.replace('\\', "\\\\").replace('\'', "\\'"))
    }
}

#[expect(
    clippy::too_many_lines,
    reason = "deterministic TPTP model rendering keeps the complete interpretation order visible"
)]
fn render_model(
    problem_name: &str,
    problem: &Problem,
    sizes: &BTreeMap<SortId, usize>,
    model: &DecodedModel,
) -> Result<String, String> {
    let occupied = problem.symbol_names.values().collect::<Vec<_>>();
    let mut prefix = "umlaut_fmb_d_".to_owned();
    while occupied.iter().any(|name| name.starts_with(&prefix)) {
        prefix.push_str("x_");
    }
    let element = |sort: SortId, value: usize| {
        format!(
            "{prefix}{}_{}",
            safe_fragment(&problem.sort_names[&sort]),
            value
        )
    };
    let status = if problem.has_conjecture {
        "CounterSatisfiable"
    } else {
        "Satisfiable"
    };
    let mut output = format!(
        "% SZS status {status} for {problem_name}\n% SZS output start FiniteModel for {problem_name}\n"
    );
    let mut serial = 0_usize;
    for &sort in &problem.sorts {
        let sort_name = tptp_symbol(&problem.sort_names[&sort]);
        for value in 0..sizes[&sort] {
            writeln!(
                output,
                "tff(umlaut_fmb_type_{serial},type,{}:{sort_name}).",
                element(sort, value)
            )
            .map_err(|error| error.to_string())?;
            serial += 1;
        }
        let domain = (0..sizes[&sort])
            .map(|value| format!("X = {}", element(sort, value)))
            .collect::<Vec<_>>()
            .join(" | ");
        writeln!(
            output,
            "tff(finite_domain_{},axiom,! [X:{sort_name}] : ({domain})).",
            safe_fragment(&problem.sort_names[&sort])
        )
        .map_err(|error| error.to_string())?;
        let inequalities = (0..sizes[&sort])
            .flat_map(|left| {
                ((left + 1)..sizes[&sort]).map(move |right| {
                    format!("{} != {}", element(sort, left), element(sort, right))
                })
            })
            .collect::<Vec<_>>();
        if !inequalities.is_empty() {
            writeln!(
                output,
                "tff(distinct_domain_{},axiom,{}).",
                safe_fragment(&problem.sort_names[&sort]),
                inequalities.join(" & ")
            )
            .map_err(|error| error.to_string())?;
        }
    }
    for ((symbol, arguments), output_value) in &model.functions {
        let signature = &problem.functions[symbol];
        let mut left = tptp_symbol(&problem.symbol_names[symbol]);
        if !arguments.is_empty() {
            left.push('(');
            left.push_str(
                &signature
                    .arguments
                    .iter()
                    .zip(arguments)
                    .map(|(&sort, &value)| element(sort, value))
                    .collect::<Vec<_>>()
                    .join(","),
            );
            left.push(')');
        }
        writeln!(
            output,
            "tff(umlaut_fmb_{serial},axiom,{left} = {}).",
            element(signature.result, *output_value)
        )
        .map_err(|error| error.to_string())?;
        serial += 1;
    }
    for ((symbol, arguments), truth) in &model.predicates {
        let signature = &problem.predicates[symbol];
        let mut atom = tptp_symbol(&problem.symbol_names[symbol]);
        if !arguments.is_empty() {
            atom.push('(');
            atom.push_str(
                &signature
                    .arguments
                    .iter()
                    .zip(arguments)
                    .map(|(&sort, &value)| element(sort, value))
                    .collect::<Vec<_>>()
                    .join(","),
            );
            atom.push(')');
        }
        writeln!(
            output,
            "tff(umlaut_fmb_{serial},axiom,{}).",
            if *truth { atom } else { format!("~({atom})") }
        )
        .map_err(|error| error.to_string())?;
        serial += 1;
    }
    writeln!(output, "% SZS output end FiniteModel for {problem_name}")
        .map_err(|error| error.to_string())?;
    Ok(output)
}

fn render_sizes(problem: &Problem, sizes: &BTreeMap<SortId, usize>) -> String {
    problem
        .sorts
        .iter()
        .map(|sort| format!("{}={}", problem.sort_names[sort], sizes[sort]))
        .collect::<Vec<_>>()
        .join(",")
}

pub(crate) fn search_with_service<S: IncrementalSatService>(
    clauses: &ClauseSet,
    bank: &TermBank,
    has_conjecture: bool,
    problem_name: &str,
    config: &FiniteModelConfig,
    service: S,
) -> FiniteModelOutcome {
    if config.maximum_size == 0 {
        return FiniteModelOutcome::Inappropriate(
            "finite-model maximum size must be positive".to_owned(),
        );
    }
    let problem = match import_problem(clauses, bank, has_conjecture) {
        Ok(problem) => problem,
        Err(message) => return FiniteModelOutcome::Inappropriate(message),
    };
    search_problem_with_service(&problem, problem_name, config, service)
}

fn search_problem_with_service<S: IncrementalSatService>(
    problem: &Problem,
    problem_name: &str,
    config: &FiniteModelConfig,
    service: S,
) -> FiniteModelOutcome {
    let vectors = match domain_vectors(&problem.sorts, config.maximum_size, config.maximum_vectors)
    {
        Ok(vectors) => vectors,
        Err(message) => return FiniteModelOutcome::ResourceOut(message),
    };
    let mut encoding = match Encoding::new(problem, config, service) {
        Ok(encoding) => encoding,
        Err(message) => return FiniteModelOutcome::ResourceOut(message),
    };
    let mut telemetry = String::new();
    for sizes in vectors {
        if super::umlaut::finite_model_stop_requested() {
            return FiniteModelOutcome::ResourceOut("external time or signal limit".to_owned());
        }
        let clauses_before = encoding.clause_count;
        let grounding_started = Instant::now();
        let new_ground = match encoding.extend_grounding(&sizes) {
            Ok(count) => count,
            Err(message) => return FiniteModelOutcome::ResourceOut(message),
        };
        let grounding_seconds = grounding_started.elapsed().as_secs_f64();
        let sat_started = Instant::now();
        let outcome = encoding.solve(&sizes);
        let sat_seconds = sat_started.elapsed().as_secs_f64();
        let sat_label = match &outcome {
            SatSolveOutcome::Sat { .. } => "sat",
            SatSolveOutcome::Unsat { .. } => "unsat",
            SatSolveOutcome::Unknown(_) => "unknown",
            SatSolveOutcome::Error(_) => "error",
        };
        let _ = writeln!(
            telemetry,
            "% FNT bound sizes={} new_ground={} cumulative_ground={} new_clauses={} cumulative_clauses={} variables={} grounding_seconds={grounding_seconds:.6} sat_seconds={sat_seconds:.6} sat_status={sat_label}",
            render_sizes(problem, &sizes),
            new_ground,
            encoding.grounded.len(),
            encoding.clause_count - clauses_before,
            encoding.clause_count,
            encoding.variable_count(),
        );
        match outcome {
            SatSolveOutcome::Sat { model } => {
                let decoded = match encoding.decode(&sizes, &model) {
                    Ok(model) => model,
                    Err(message) => return FiniteModelOutcome::Error(message),
                };
                if let Err(message) = validate_model(problem, &sizes, &decoded) {
                    return FiniteModelOutcome::Error(message);
                }
                return match render_model(problem_name, problem, &sizes, &decoded) {
                    Ok(model) => FiniteModelOutcome::Model(format!("{telemetry}{model}")),
                    Err(message) => FiniteModelOutcome::Error(message),
                };
            }
            SatSolveOutcome::Unsat { .. } => {}
            SatSolveOutcome::Unknown(reason) => {
                let reason = match reason {
                    SatUnknownReason::DecisionLimit => "SAT decision limit",
                    SatUnknownReason::Deadline => "SAT deadline",
                    SatUnknownReason::Cancelled => "SAT cancellation",
                    SatUnknownReason::ExternalStop => "external time or signal limit",
                    SatUnknownReason::Backend => "SAT backend resource exhaustion",
                };
                return FiniteModelOutcome::ResourceOut(reason.to_owned());
            }
            SatSolveOutcome::Error(error) => {
                return FiniteModelOutcome::Error(error.to_string());
            }
        }
    }
    FiniteModelOutcome::BoundsExhausted
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::{
        domain_vectors, for_each_tuple, search_problem_with_service, validate_model, DecodedModel,
        FiniteModelConfig, FiniteModelOutcome, InputClause, InputLiteral, InputTerm, Problem,
        SymbolType,
    };
    use crate::clauses::satservice::{
        IncrementalSatService, InternalSatService, SatServiceCapabilities, SatServiceError,
        SatSolveOptions, SatSolveOutcome, SatUnknownReason,
    };

    const SORT: i64 = 10;
    const OTHER_SORT: i64 = 11;
    const FUNCTION: i64 = 20;
    const PREDICATE: i64 = 21;
    const CONSTANT: i64 = 22;
    const SECOND_CONSTANT: i64 = 23;
    const OTHER_CONSTANT: i64 = 24;
    const SECOND_OTHER_CONSTANT: i64 = 25;
    const VARIABLE_X: i64 = -2;
    const VARIABLE_Y: i64 = -4;

    struct FixedOutcomeService {
        outcome: SatSolveOutcome,
        clauses: usize,
    }

    impl FixedOutcomeService {
        const fn new(outcome: SatSolveOutcome) -> Self {
            Self {
                outcome,
                clauses: 0,
            }
        }
    }

    impl IncrementalSatService for FixedOutcomeService {
        fn backend_name(&self) -> &'static str {
            "fixed-test-service"
        }

        fn capabilities(&self) -> SatServiceCapabilities {
            InternalSatService::new().capabilities()
        }

        fn add_clause(&mut self, _clause: &[i32]) -> Result<(), SatServiceError> {
            self.clauses += 1;
            Ok(())
        }

        fn solve(&mut self, _assumptions: &[i32], _options: &SatSolveOptions) -> SatSolveOutcome {
            self.outcome.clone()
        }

        fn reset(&mut self) -> Result<(), SatServiceError> {
            self.clauses = 0;
            Ok(())
        }

        fn permanent_clause_count(&self) -> usize {
            self.clauses
        }
    }

    fn variable(code: i64, sort: i64) -> InputTerm {
        InputTerm::Variable { code, sort }
    }

    fn application(symbol: i64, sort: i64, arguments: Vec<InputTerm>) -> InputTerm {
        InputTerm::Application {
            symbol,
            sort,
            arguments,
        }
    }

    fn equality(left: InputTerm, right: InputTerm, negated: bool) -> InputLiteral {
        InputLiteral::Equality {
            left,
            right,
            negated,
        }
    }

    fn base_problem() -> Problem {
        Problem {
            clauses: Vec::new(),
            functions: BTreeMap::new(),
            predicates: BTreeMap::new(),
            sorts: vec![SORT],
            symbol_names: BTreeMap::from([
                (FUNCTION, "f".to_owned()),
                (PREDICATE, "p".to_owned()),
                (CONSTANT, "c".to_owned()),
                (SECOND_CONSTANT, "d".to_owned()),
            ]),
            sort_names: BTreeMap::from([(SORT, "s".to_owned())]),
            has_conjecture: false,
        }
    }

    fn unary_problem() -> Problem {
        let mut problem = base_problem();
        problem.functions.insert(
            FUNCTION,
            SymbolType {
                arguments: vec![SORT],
                result: SORT,
            },
        );
        problem.predicates.insert(
            PREDICATE,
            SymbolType {
                arguments: vec![SORT],
                result: crate::terms::simpletypes::ST_BOOL,
            },
        );
        let x = variable(VARIABLE_X, SORT);
        let fx = application(FUNCTION, SORT, vec![x.clone()]);
        let ffx = application(FUNCTION, SORT, vec![fx.clone()]);
        problem.clauses = vec![
            InputClause {
                name: "nested_identity".to_owned(),
                variables: vec![(VARIABLE_X, SORT)],
                literals: vec![equality(ffx, x, false)],
            },
            InputClause {
                name: "positive_predicate".to_owned(),
                variables: vec![(VARIABLE_X, SORT)],
                literals: vec![InputLiteral::Predicate {
                    symbol: PREDICATE,
                    arguments: vec![fx],
                    negated: false,
                }],
            },
        ];
        problem
    }

    fn infinite_only_problem() -> Problem {
        let mut problem = base_problem();
        problem.functions.insert(
            FUNCTION,
            SymbolType {
                arguments: vec![SORT],
                result: SORT,
            },
        );
        problem.functions.insert(
            CONSTANT,
            SymbolType {
                arguments: Vec::new(),
                result: SORT,
            },
        );
        let x = variable(VARIABLE_X, SORT);
        let y = variable(VARIABLE_Y, SORT);
        let fx = application(FUNCTION, SORT, vec![x.clone()]);
        let fy = application(FUNCTION, SORT, vec![y.clone()]);
        let constant = application(CONSTANT, SORT, Vec::new());
        problem.clauses = vec![
            InputClause {
                name: "misses_constant".to_owned(),
                variables: vec![(VARIABLE_X, SORT)],
                literals: vec![equality(fx.clone(), constant, true)],
            },
            InputClause {
                name: "injective".to_owned(),
                variables: vec![(VARIABLE_X, SORT), (VARIABLE_Y, SORT)],
                literals: vec![equality(fx, fy, true), equality(x, y, false)],
            },
        ];
        problem
    }

    fn test_config(maximum_size: usize) -> FiniteModelConfig {
        FiniteModelConfig {
            maximum_size,
            maximum_vectors: 128,
            maximum_ground_instances: 10_000,
            maximum_clauses: 100_000,
            maximum_variables: 100_000,
            ..FiniteModelConfig::default()
        }
    }

    #[test]
    fn tuple_enumerator_includes_the_empty_tuple() {
        let mut tuples = Vec::new();
        for_each_tuple(&[], |tuple| {
            tuples.push(tuple.to_vec());
            Ok(())
        })
        .unwrap();
        assert_eq!(tuples, vec![Vec::<usize>::new()]);
    }

    #[test]
    fn domain_vectors_are_total_size_then_lexicographic() {
        let vectors = domain_vectors(&[2, 7], 3, 20).unwrap();
        let values = vectors
            .iter()
            .map(|vector| (vector[&2], vector[&7]))
            .collect::<Vec<_>>();
        assert_eq!(
            values,
            vec![
                (1, 1),
                (1, 2),
                (2, 1),
                (1, 3),
                (2, 2),
                (3, 1),
                (2, 3),
                (3, 2),
                (3, 3)
            ]
        );
    }

    #[test]
    fn unary_and_nested_function_model_is_complete_and_checked() {
        let outcome = search_problem_with_service(
            &unary_problem(),
            "unary.p",
            &test_config(2),
            InternalSatService::new(),
        );
        let FiniteModelOutcome::Model(model) = outcome else {
            panic!("expected a finite model, got {outcome:?}");
        };
        assert!(model.contains("% SZS status Satisfiable for unary.p"));
        assert!(model.contains("f(umlaut_fmb_d_s_0) = umlaut_fmb_d_s_0"));
        assert!(model.contains("p(umlaut_fmb_d_s_0)"));
        assert!(model.contains("% FNT bound sizes=s=1"));
    }

    #[test]
    fn model_status_tracks_a_clausified_conjecture() {
        let mut problem = unary_problem();
        problem.has_conjecture = true;
        let outcome = search_problem_with_service(
            &problem,
            "counter.p",
            &test_config(1),
            InternalSatService::new(),
        );
        let FiniteModelOutcome::Model(model) = outcome else {
            panic!("expected a countermodel, got {outcome:?}");
        };
        assert!(model.contains("% SZS status CounterSatisfiable for counter.p"));
    }

    #[test]
    fn two_native_sorts_and_positive_arity_function_render_complete_tables() {
        let mut problem = base_problem();
        problem.sorts.push(OTHER_SORT);
        problem.sort_names.insert(OTHER_SORT, "other".to_owned());
        problem.symbol_names.extend([
            (OTHER_CONSTANT, "b".to_owned()),
            (SECOND_OTHER_CONSTANT, "e".to_owned()),
        ]);
        for (symbol, sort) in [
            (CONSTANT, SORT),
            (SECOND_CONSTANT, SORT),
            (OTHER_CONSTANT, OTHER_SORT),
            (SECOND_OTHER_CONSTANT, OTHER_SORT),
        ] {
            problem.functions.insert(
                symbol,
                SymbolType {
                    arguments: Vec::new(),
                    result: sort,
                },
            );
        }
        problem.functions.insert(
            FUNCTION,
            SymbolType {
                arguments: vec![SORT],
                result: OTHER_SORT,
            },
        );
        let c = application(CONSTANT, SORT, Vec::new());
        let d = application(SECOND_CONSTANT, SORT, Vec::new());
        let b = application(OTHER_CONSTANT, OTHER_SORT, Vec::new());
        let e = application(SECOND_OTHER_CONSTANT, OTHER_SORT, Vec::new());
        problem.clauses = vec![
            InputClause {
                name: "two_source_elements".to_owned(),
                variables: Vec::new(),
                literals: vec![equality(c.clone(), d.clone(), true)],
            },
            InputClause {
                name: "two_target_elements".to_owned(),
                variables: Vec::new(),
                literals: vec![equality(b.clone(), e.clone(), true)],
            },
            InputClause {
                name: "first_row".to_owned(),
                variables: Vec::new(),
                literals: vec![equality(
                    application(FUNCTION, OTHER_SORT, vec![c]),
                    e,
                    false,
                )],
            },
            InputClause {
                name: "second_row".to_owned(),
                variables: Vec::new(),
                literals: vec![equality(
                    application(FUNCTION, OTHER_SORT, vec![d]),
                    b,
                    false,
                )],
            },
        ];
        let outcome = search_problem_with_service(
            &problem,
            "typed.p",
            &test_config(2),
            InternalSatService::new(),
        );
        let FiniteModelOutcome::Model(model) = outcome else {
            panic!("expected a typed model, got {outcome:?}");
        };
        assert!(model.contains("finite_domain_s"));
        assert!(model.contains("finite_domain_other"));
        assert_eq!(model.matches("f(umlaut_fmb_d_s_").count(), 2);
    }

    #[test]
    fn bounded_search_does_not_claim_an_infinite_only_problem() {
        let outcome = search_problem_with_service(
            &infinite_only_problem(),
            "infinite.p",
            &test_config(3),
            InternalSatService::new(),
        );
        assert_eq!(outcome, FiniteModelOutcome::BoundsExhausted);
    }

    #[test]
    fn semantic_checker_rejects_a_corrupted_function_row() {
        let mut problem = base_problem();
        problem.functions.insert(
            FUNCTION,
            SymbolType {
                arguments: vec![SORT],
                result: SORT,
            },
        );
        let x = variable(VARIABLE_X, SORT);
        problem.clauses.push(InputClause {
            name: "identity".to_owned(),
            variables: vec![(VARIABLE_X, SORT)],
            literals: vec![equality(
                application(FUNCTION, SORT, vec![x.clone()]),
                x,
                false,
            )],
        });
        let sizes = BTreeMap::from([(SORT, 2)]);
        let corrupt = DecodedModel {
            functions: BTreeMap::from([((FUNCTION, vec![0]), 1), ((FUNCTION, vec![1]), 1)]),
            predicates: BTreeMap::new(),
        };
        assert!(validate_model(&problem, &sizes, &corrupt)
            .unwrap_err()
            .contains("falsifies identity"));
    }

    #[test]
    fn incremental_and_fresh_encodings_agree_at_every_bound() {
        let problem = infinite_only_problem();
        let config = test_config(3);
        let vectors = domain_vectors(&problem.sorts, 3, 16).unwrap();
        let mut incremental =
            super::Encoding::new(&problem, &config, InternalSatService::new()).unwrap();
        for sizes in vectors {
            incremental.extend_grounding(&sizes).unwrap();
            let incremental_sat = matches!(incremental.solve(&sizes), SatSolveOutcome::Sat { .. });
            let mut fresh =
                super::Encoding::new(&problem, &config, InternalSatService::new()).unwrap();
            fresh.extend_grounding(&sizes).unwrap();
            let fresh_sat = matches!(fresh.solve(&sizes), SatSolveOutcome::Sat { .. });
            assert_eq!(incremental_sat, fresh_sat, "sizes {sizes:?}");
        }
        assert!(incremental.service.permanent_clause_count() > 0);
    }

    #[test]
    fn encoding_limits_fail_closed() {
        let mut config = test_config(2);
        config.maximum_variables = 1;
        let outcome = search_problem_with_service(
            &unary_problem(),
            "limited.p",
            &config,
            InternalSatService::new(),
        );
        assert!(matches!(outcome, FiniteModelOutcome::ResourceOut(_)));
    }

    #[test]
    fn sat_timeout_and_backend_error_fail_closed() {
        let timeout = search_problem_with_service(
            &unary_problem(),
            "timeout.p",
            &test_config(1),
            FixedOutcomeService::new(SatSolveOutcome::Unknown(SatUnknownReason::Deadline)),
        );
        assert_eq!(
            timeout,
            FiniteModelOutcome::ResourceOut("SAT deadline".to_owned())
        );

        let failure = search_problem_with_service(
            &unary_problem(),
            "error.p",
            &test_config(1),
            FixedOutcomeService::new(SatSolveOutcome::Error(SatServiceError::Backend(
                "injected failure".to_owned(),
            ))),
        );
        assert!(
            matches!(failure, FiniteModelOutcome::Error(message) if message.contains("injected failure"))
        );
    }
}
