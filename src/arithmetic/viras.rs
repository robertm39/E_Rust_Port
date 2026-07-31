//! Clean-room, paper-derived base VIRAS quantifier elimination.
//!
//! This module ports the validated one-conjunction kernel from
//! `experiments/2026-07-30-004-base-viras-qe-prototype`. It does not use the
//! unlicensed VIRAS implementation. The public boundary is deliberately
//! independent of Umlaut's term bank; the typed adapter is responsible for
//! proving that imported terms belong to this exact linear real-plus-floor
//! fragment.

use num_bigint::BigInt;
use num_integer::Integer;
use num_rational::BigRational;
use num_traits::{One, Signed, Zero};
use std::collections::{BTreeMap, BTreeSet};

/// Arbitrary-precision exact rational used by the kernel.
pub type Rational = BigRational;

#[must_use]
pub fn integer(value: i64) -> Rational {
    Rational::from_integer(BigInt::from(value))
}

/// Constructs a normalized rational.
///
/// # Errors
///
/// Returns an unsupported-fragment failure when `denominator` is zero.
pub fn rational(numerator: i64, denominator: i64) -> Result<Rational, KernelFailure> {
    if denominator == 0 {
        return Err(KernelFailure::unsupported(
            "rational denominator must be nonzero",
        ));
    }
    Ok(Rational::new(
        BigInt::from(numerator),
        BigInt::from(denominator),
    ))
}

fn floor_rational(value: &Rational) -> BigInt {
    value.numer().div_floor(value.denom())
}

fn ceil_rational(value: &Rational) -> BigInt {
    value.numer().div_ceil(value.denom())
}

fn rational_lcm<'a>(values: impl IntoIterator<Item = &'a Rational>) -> Rational {
    let positives = values
        .into_iter()
        .filter(|value| !value.is_zero())
        .map(Rational::abs)
        .collect::<Vec<_>>();
    let Some(first) = positives.first() else {
        return Rational::zero();
    };

    let mut numerator = BigInt::one();
    let mut denominator = first.denom().clone();
    for value in &positives {
        numerator = numerator.lcm(value.numer());
    }
    for value in positives.iter().skip(1) {
        denominator = denominator.gcd(value.denom());
    }
    Rational::new(numerator, denominator)
}

fn fraction_text(value: &Rational) -> String {
    if value.is_integer() {
        value.to_integer().to_string()
    } else {
        format!("{}/{}", value.numer(), value.denom())
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Term {
    Constant(Rational),
    Variable(String),
    Add(Vec<Self>),
    Scale(Rational, Box<Self>),
    Floor(Box<Self>),
}

impl Term {
    #[must_use]
    pub fn variables(&self) -> BTreeSet<String> {
        let mut result = BTreeSet::new();
        self.collect_variables(&mut result);
        result
    }

    fn collect_variables(&self, output: &mut BTreeSet<String>) {
        match self {
            Self::Constant(_) => {}
            Self::Variable(name) => {
                output.insert(name.clone());
            }
            Self::Add(arguments) => {
                for argument in arguments {
                    argument.collect_variables(output);
                }
            }
            Self::Scale(_, argument) | Self::Floor(argument) => {
                argument.collect_variables(output);
            }
        }
    }

    #[must_use]
    pub fn contains(&self, variable: &str) -> bool {
        match self {
            Self::Constant(_) => false,
            Self::Variable(name) => name == variable,
            Self::Add(arguments) => arguments.iter().any(|argument| argument.contains(variable)),
            Self::Scale(_, argument) | Self::Floor(argument) => argument.contains(variable),
        }
    }

    #[must_use]
    pub fn render(&self) -> String {
        match self {
            Self::Constant(value) => fraction_text(value),
            Self::Variable(name) => name.clone(),
            Self::Add(arguments) => format!(
                "({})",
                arguments
                    .iter()
                    .map(Self::render)
                    .collect::<Vec<_>>()
                    .join(" + ")
            ),
            Self::Scale(coefficient, argument) => {
                format!("({}*{})", fraction_text(coefficient), argument.render())
            }
            Self::Floor(argument) => format!("floor({})", argument.render()),
        }
    }

    #[must_use]
    pub fn canonical_json(&self) -> String {
        match self {
            Self::Constant(value) => format!("[\"const\",\"{}\"]", fraction_text(value)),
            Self::Variable(name) => format!("[\"var\",\"{}\"]", json_string(name)),
            Self::Add(arguments) => format!(
                "[\"add\",{}]",
                arguments
                    .iter()
                    .map(Self::canonical_json)
                    .collect::<Vec<_>>()
                    .join(",")
            ),
            Self::Scale(coefficient, argument) => format!(
                "[\"scale\",\"{}\",{}]",
                fraction_text(coefficient),
                argument.canonical_json()
            ),
            Self::Floor(argument) => format!("[\"floor\",{}]", argument.canonical_json()),
        }
    }
}

fn json_string(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            control if control.is_control() => {
                use std::fmt::Write as _;
                let _ = write!(output, "\\u{:04x}", u32::from(control));
            }
            other => output.push(other),
        }
    }
    output
}

#[must_use]
pub fn constant(value: Rational) -> Term {
    Term::Constant(value)
}

#[must_use]
pub fn int_constant(value: i64) -> Term {
    constant(integer(value))
}

#[must_use]
pub fn variable(name: impl Into<String>) -> Term {
    Term::Variable(name.into())
}

#[must_use]
pub fn scale(coefficient: Rational, term: Term) -> Term {
    if coefficient.is_zero() {
        return int_constant(0);
    }
    match term {
        Term::Constant(value) => constant(coefficient * value),
        term if coefficient.is_one() => term,
        Term::Scale(inner, argument) => scale(coefficient * inner, *argument),
        Term::Add(arguments) => add(arguments
            .into_iter()
            .map(|argument| scale(coefficient.clone(), argument))),
        term => Term::Scale(coefficient, Box::new(term)),
    }
}

#[must_use]
pub fn negate(term: Term) -> Term {
    scale(integer(-1), term)
}

#[must_use]
pub fn add(terms: impl IntoIterator<Item = Term>) -> Term {
    let mut pending = Vec::new();
    for term in terms {
        match term {
            Term::Add(arguments) => pending.extend(arguments),
            other => pending.push(other),
        }
    }

    let mut constant_sum = Rational::zero();
    let mut coefficients = BTreeMap::<Term, Rational>::new();
    for term in pending {
        match term {
            Term::Constant(value) => constant_sum += value,
            Term::Scale(coefficient, argument) => {
                *coefficients.entry(*argument).or_insert_with(Rational::zero) += coefficient;
            }
            other => {
                *coefficients.entry(other).or_insert_with(Rational::zero) += Rational::one();
            }
        }
    }

    let mut children = coefficients
        .into_iter()
        .filter_map(|(term, coefficient)| {
            (!coefficient.is_zero()).then(|| scale(coefficient, term))
        })
        .collect::<Vec<_>>();
    if !constant_sum.is_zero() {
        children.push(constant(constant_sum));
    }
    if children.is_empty() {
        return int_constant(0);
    }
    children.sort_by_key(Term::render);
    if children.len() == 1 {
        return children.remove(0);
    }
    Term::Add(children)
}

#[must_use]
pub fn subtract(left: Term, right: Term) -> Term {
    add([left, negate(right)])
}

#[must_use]
pub fn floor_term(term: Term) -> Term {
    match term {
        Term::Constant(value) => constant(Rational::from_integer(floor_rational(&value))),
        Term::Add(arguments) => {
            let mut integer_shift = Rational::zero();
            let mut rest = Vec::new();
            for argument in arguments {
                match argument {
                    Term::Constant(value) if value.is_integer() => integer_shift += value,
                    other => rest.push(other),
                }
            }
            if rest.is_empty() {
                return constant(integer_shift);
            }
            if !integer_shift.is_zero() && !rest.is_empty() {
                return add([floor_term(add(rest)), constant(integer_shift)]);
            }
            Term::Floor(Box::new(add(rest)))
        }
        other => Term::Floor(Box::new(other)),
    }
}

#[must_use]
pub fn ceil_term(term: Term) -> Term {
    negate(floor_term(negate(term)))
}

#[must_use]
pub fn substitute(term: &Term, name: &str, replacement: &Term) -> Term {
    match term {
        Term::Constant(_) => term.clone(),
        Term::Variable(variable) => {
            if variable == name {
                replacement.clone()
            } else {
                term.clone()
            }
        }
        Term::Add(arguments) => add(arguments
            .iter()
            .map(|argument| substitute(argument, name, replacement))),
        Term::Scale(coefficient, argument) => {
            scale(coefficient.clone(), substitute(argument, name, replacement))
        }
        Term::Floor(argument) => floor_term(substitute(argument, name, replacement)),
    }
}

/// Evaluates a term exactly.
///
/// # Errors
///
/// Returns an unsupported-fragment failure when a variable is absent.
pub fn evaluate_term(
    term: &Term,
    environment: &BTreeMap<String, Rational>,
) -> Result<Rational, KernelFailure> {
    match term {
        Term::Constant(value) => Ok(value.clone()),
        Term::Variable(name) => environment.get(name).cloned().ok_or_else(|| {
            KernelFailure::unsupported(format!("unbound evaluation variable {name}"))
        }),
        Term::Add(arguments) => {
            let mut result = Rational::zero();
            for argument in arguments {
                result += evaluate_term(argument, environment)?;
            }
            Ok(result)
        }
        Term::Scale(coefficient, argument) => {
            Ok(coefficient * evaluate_term(argument, environment)?)
        }
        Term::Floor(argument) => Ok(Rational::from_integer(floor_rational(&evaluate_term(
            argument,
            environment,
        )?))),
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Relation {
    Eq,
    Ne,
    Gt,
    Ge,
}

impl Relation {
    #[must_use]
    pub fn evaluate(self, value: &Rational) -> bool {
        match self {
            Self::Eq => value.is_zero(),
            Self::Ne => !value.is_zero(),
            Self::Gt => value.is_positive(),
            Self::Ge => !value.is_negative(),
        }
    }

    const fn name(self) -> &'static str {
        match self {
            Self::Eq => "eq",
            Self::Ne => "ne",
            Self::Gt => "gt",
            Self::Ge => "ge",
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Literal {
    pub term: Term,
    pub relation: Relation,
}

impl Literal {
    #[must_use]
    pub const fn new(term: Term, relation: Relation) -> Self {
        Self { term, relation }
    }

    #[must_use]
    pub fn variables(&self) -> BTreeSet<String> {
        self.term.variables()
    }

    #[must_use]
    pub fn render(&self) -> String {
        let operator = match self.relation {
            Relation::Eq => "=",
            Relation::Ne => "!=",
            Relation::Gt => ">",
            Relation::Ge => ">=",
        };
        format!("{} {operator} 0", self.term.render())
    }

    #[must_use]
    pub fn canonical_json(&self) -> String {
        format!(
            "[\"{}\",{}]",
            self.relation.name(),
            self.term.canonical_json()
        )
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Formula {
    Bool(bool),
    Atom(Literal),
    And(Vec<Self>),
    Or(Vec<Self>),
    Exists(String, Box<Self>),
    Forall(String, Box<Self>),
}

impl Formula {
    #[must_use]
    pub fn variables(&self) -> BTreeSet<String> {
        match self {
            Self::Bool(_) => BTreeSet::new(),
            Self::Atom(literal) => literal.variables(),
            Self::And(children) | Self::Or(children) => {
                let mut result = BTreeSet::new();
                for child in children {
                    result.extend(child.variables());
                }
                result
            }
            Self::Exists(variable, body) | Self::Forall(variable, body) => {
                let mut result = body.variables();
                result.remove(variable);
                result
            }
        }
    }

    #[must_use]
    pub fn render(&self) -> String {
        match self {
            Self::Bool(value) => if *value { "true" } else { "false" }.to_owned(),
            Self::Atom(literal) => literal.render(),
            Self::And(children) => format!(
                "({})",
                children
                    .iter()
                    .map(Self::render)
                    .collect::<Vec<_>>()
                    .join(" & ")
            ),
            Self::Or(children) => format!(
                "({})",
                children
                    .iter()
                    .map(Self::render)
                    .collect::<Vec<_>>()
                    .join(" | ")
            ),
            Self::Exists(variable, body) => format!("exists {variable}. {}", body.render()),
            Self::Forall(variable, body) => format!("forall {variable}. {}", body.render()),
        }
    }

    #[must_use]
    pub fn canonical_json(&self) -> String {
        match self {
            Self::Bool(value) => format!("[\"bool\",{value}]"),
            Self::Atom(literal) => format!("[\"atom\",{}]", literal.canonical_json()),
            Self::And(children) => format!(
                "[\"and\",{}]",
                children
                    .iter()
                    .map(Self::canonical_json)
                    .collect::<Vec<_>>()
                    .join(",")
            ),
            Self::Or(children) => format!(
                "[\"or\",{}]",
                children
                    .iter()
                    .map(Self::canonical_json)
                    .collect::<Vec<_>>()
                    .join(",")
            ),
            Self::Exists(variable, body) => format!(
                "[\"exists\",\"{}\",{}]",
                json_string(variable),
                body.canonical_json()
            ),
            Self::Forall(variable, body) => format!(
                "[\"forall\",\"{}\",{}]",
                json_string(variable),
                body.canonical_json()
            ),
        }
    }

    /// Evaluates a quantifier-free formula exactly.
    ///
    /// # Errors
    ///
    /// Returns an unsupported-fragment failure if a quantifier remains.
    pub fn evaluate(
        &self,
        environment: &BTreeMap<String, Rational>,
    ) -> Result<bool, KernelFailure> {
        match self {
            Self::Bool(value) => Ok(*value),
            Self::Atom(literal) => Ok(literal
                .relation
                .evaluate(&evaluate_term(&literal.term, environment)?)),
            Self::And(children) => {
                for child in children {
                    if !child.evaluate(environment)? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            Self::Or(children) => {
                for child in children {
                    if child.evaluate(environment)? {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
            Self::Exists(_, _) | Self::Forall(_, _) => Err(KernelFailure::unsupported(
                "quantified formula cannot be evaluated without a domain",
            )),
        }
    }
}

#[must_use]
pub const fn boolean(value: bool) -> Formula {
    Formula::Bool(value)
}

#[must_use]
pub fn atom(literal: Literal) -> Formula {
    if literal.variables().is_empty() {
        if let Ok(value) = evaluate_term(&literal.term, &BTreeMap::new()) {
            return boolean(literal.relation.evaluate(&value));
        }
    }
    Formula::Atom(literal)
}

#[must_use]
pub fn conjunction(children: impl IntoIterator<Item = Formula>) -> Formula {
    let mut flattened = Vec::new();
    for child in children {
        match child {
            Formula::Bool(false) => return boolean(false),
            Formula::Bool(true) => {}
            Formula::And(grandchildren) => flattened.extend(grandchildren),
            other => flattened.push(other),
        }
    }
    let mut unique = BTreeMap::new();
    for child in flattened {
        unique.entry(child.canonical_json()).or_insert(child);
    }
    let mut children = unique.into_values().collect::<Vec<_>>();
    match children.len() {
        0 => boolean(true),
        1 => children.remove(0),
        _ => Formula::And(children),
    }
}

#[must_use]
pub fn disjunction(children: impl IntoIterator<Item = Formula>) -> Formula {
    let mut flattened = Vec::new();
    for child in children {
        match child {
            Formula::Bool(true) => return boolean(true),
            Formula::Bool(false) => {}
            Formula::Or(grandchildren) => flattened.extend(grandchildren),
            other => flattened.push(other),
        }
    }
    let mut unique = BTreeMap::new();
    for child in flattened {
        unique.entry(child.canonical_json()).or_insert(child);
    }
    let mut children = unique.into_values().collect::<Vec<_>>();
    match children.len() {
        0 => boolean(false),
        1 => children.remove(0),
        _ => Formula::Or(children),
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Grid {
    pub base: Term,
    pub period: Rational,
}

impl Grid {
    /// Constructs a positive-period grid.
    ///
    /// # Errors
    ///
    /// Returns an unsupported-fragment failure for a nonpositive period.
    pub fn new(base: Term, period: Rational) -> Result<Self, KernelFailure> {
        if !period.is_positive() {
            return Err(KernelFailure::unsupported("grid period must be positive"));
        }
        Ok(Self { base, period })
    }

    #[must_use]
    pub fn canonical_json(&self) -> String {
        format!(
            "[{},\"{}\"]",
            self.base.canonical_json(),
            fraction_text(&self.period)
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum InfinitySign {
    Negative,
    Positive,
}

impl InfinitySign {
    const fn factor(self) -> i8 {
        match self {
            Self::Negative => -1,
            Self::Positive => 1,
        }
    }

    const fn name(self) -> &'static str {
        match self {
            Self::Negative => "negative",
            Self::Positive => "positive",
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct VirtualTerm {
    pub base: Term,
    pub epsilon: bool,
    pub grid_period: Option<Rational>,
    pub infinity: Option<InfinitySign>,
}

impl Default for VirtualTerm {
    fn default() -> Self {
        Self {
            base: int_constant(0),
            epsilon: false,
            grid_period: None,
            infinity: None,
        }
    }
}

impl VirtualTerm {
    /// Constructs a validated virtual term.
    ///
    /// # Errors
    ///
    /// Returns an unsupported-fragment failure for a nonpositive grid or for
    /// a term that combines grid and infinity markers.
    pub fn new(
        base: Term,
        epsilon: bool,
        grid_period: Option<Rational>,
        infinity: Option<InfinitySign>,
    ) -> Result<Self, KernelFailure> {
        if grid_period
            .as_ref()
            .is_some_and(|period| !period.is_positive())
        {
            return Err(KernelFailure::unsupported(
                "virtual grid period must be positive",
            ));
        }
        if grid_period.is_some() && infinity.is_some() {
            return Err(KernelFailure::unsupported(
                "a virtual term cannot contain grid and infinity",
            ));
        }
        Ok(Self {
            base,
            epsilon,
            grid_period,
            infinity,
        })
    }

    #[must_use]
    pub fn with_epsilon(&self) -> Self {
        let mut result = self.clone();
        result.epsilon = true;
        result
    }

    #[must_use]
    pub fn canonical_json(&self) -> String {
        format!(
            "{{\"base\":{},\"epsilon\":{},\"grid_period\":{},\"infinity\":{}}}",
            self.base.canonical_json(),
            self.epsilon,
            self.grid_period.as_ref().map_or_else(
                || "null".to_owned(),
                |period| format!("\"{}\"", fraction_text(period))
            ),
            self.infinity
                .map_or_else(|| "null".to_owned(), |sign| format!("\"{}\"", sign.name()))
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Profile {
    pub outer_slope: Rational,
    pub segment_slope: Rational,
    pub period: Rational,
    pub delta_y: Rational,
    pub dist_y_minus: Term,
    pub right_limit: Term,
}

impl Profile {
    #[must_use]
    pub fn dist_y_plus(&self) -> Term {
        add([self.dist_y_minus.clone(), constant(self.delta_y.clone())])
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Limits {
    pub max_steps: usize,
    pub max_candidates: usize,
    pub max_grids: usize,
    pub max_grid_points: usize,
    pub max_dnf_branches: usize,
    pub max_formula_nodes: usize,
    pub max_rational_bits: u64,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_steps: 1_000_000,
            max_candidates: 20_000,
            max_grids: 20_000,
            max_grid_points: 50_000,
            max_dnf_branches: 20_000,
            max_formula_nodes: 200_000,
            max_rational_bits: 4_096,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UnknownKind {
    ResourceLimit,
    UnsupportedFragment,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KernelFailure {
    pub kind: UnknownKind,
    pub reason: String,
}

impl KernelFailure {
    fn resource(reason: impl Into<String>) -> Self {
        Self {
            kind: UnknownKind::ResourceLimit,
            reason: reason.into(),
        }
    }

    fn unsupported(reason: impl Into<String>) -> Self {
        Self {
            kind: UnknownKind::UnsupportedFragment,
            reason: reason.into(),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct BudgetSnapshot {
    pub steps: usize,
    pub candidates: usize,
    pub grids: usize,
    pub grid_points: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Budget {
    limits: Limits,
    usage: BudgetSnapshot,
}

impl Budget {
    const fn new(limits: Limits) -> Self {
        Self {
            limits,
            usage: BudgetSnapshot {
                steps: 0,
                candidates: 0,
                grids: 0,
                grid_points: 0,
            },
        }
    }

    fn tick(&mut self, label: &str) -> Result<(), KernelFailure> {
        self.usage.steps = self.usage.steps.saturating_add(1);
        if self.usage.steps > self.limits.max_steps {
            return Err(KernelFailure::resource(format!(
                "step limit exceeded during {label}: {}>{}",
                self.usage.steps, self.limits.max_steps
            )));
        }
        Ok(())
    }

    fn check_fraction(&self, value: &Rational, label: &str) -> Result<(), KernelFailure> {
        let bits = value.numer().magnitude().bits().max(value.denom().bits());
        if bits > self.limits.max_rational_bits {
            return Err(KernelFailure::resource(format!(
                "rational bit limit exceeded during {label}: {bits}>{}",
                self.limits.max_rational_bits
            )));
        }
        Ok(())
    }

    fn add_candidate(&mut self) -> Result<(), KernelFailure> {
        self.usage.candidates = self.usage.candidates.saturating_add(1);
        if self.usage.candidates > self.limits.max_candidates {
            return Err(KernelFailure::resource(format!(
                "candidate limit exceeded: {}>{}",
                self.usage.candidates, self.limits.max_candidates
            )));
        }
        Ok(())
    }

    fn add_grid(&mut self) -> Result<(), KernelFailure> {
        self.usage.grids = self.usage.grids.saturating_add(1);
        if self.usage.grids > self.limits.max_grids {
            return Err(KernelFailure::resource(format!(
                "grid limit exceeded: {}>{}",
                self.usage.grids, self.limits.max_grids
            )));
        }
        Ok(())
    }

    fn add_grid_point(&mut self) -> Result<(), KernelFailure> {
        self.usage.grid_points = self.usage.grid_points.saturating_add(1);
        if self.usage.grid_points > self.limits.max_grid_points {
            return Err(KernelFailure::resource(format!(
                "grid-point limit exceeded: {}>{}",
                self.usage.grid_points, self.limits.max_grid_points
            )));
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Mutations {
    pub reverse_infinity_periodicity: bool,
    pub drop_epsilon_strictness: bool,
    pub omit_last_candidate: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Candidate {
    pub virtual_term: VirtualTerm,
    pub literal_index: usize,
    pub origin_kind: &'static str,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GridFlattenRecord {
    pub case: &'static str,
    pub input: VirtualTerm,
    pub common_period: Rational,
    pub output: Vec<VirtualTerm>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Derivation {
    pub calculus: &'static str,
    pub eliminated: String,
    pub candidates: Vec<Candidate>,
    pub grid_flattening: Vec<GridFlattenRecord>,
    pub resource_usage: BudgetSnapshot,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum QeStatus {
    Success,
    Unknown,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QeOutcome {
    pub status: QeStatus,
    pub formula: Option<Formula>,
    pub unknown_kind: Option<UnknownKind>,
    pub reason: String,
    pub derivation: Derivation,
}

impl QeOutcome {
    fn success(formula: Formula, reason: impl Into<String>, derivation: Derivation) -> Self {
        Self {
            status: QeStatus::Success,
            formula: Some(formula),
            unknown_kind: None,
            reason: reason.into(),
            derivation,
        }
    }

    fn unknown(failure: KernelFailure, derivation: Derivation) -> Self {
        Self {
            status: QeStatus::Unknown,
            formula: None,
            unknown_kind: Some(failure.kind),
            reason: failure.reason,
            derivation,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Kernel {
    budget: Budget,
    mutations: Mutations,
    profiles: BTreeMap<(Term, String), Profile>,
    breaks: BTreeMap<(Term, String), Vec<Grid>>,
    flatten_records: Vec<GridFlattenRecord>,
}

impl Kernel {
    #[must_use]
    pub fn new(limits: Limits) -> Self {
        Self::with_mutations(limits, Mutations::default())
    }

    #[must_use]
    pub fn with_mutations(limits: Limits, mutations: Mutations) -> Self {
        Self {
            budget: Budget::new(limits),
            mutations,
            profiles: BTreeMap::new(),
            breaks: BTreeMap::new(),
            flatten_records: Vec::new(),
        }
    }

    fn invariant(reason: impl Into<String>) -> KernelFailure {
        KernelFailure::unsupported(format!("kernel invariant violated: {}", reason.into()))
    }

    #[allow(
        clippy::too_many_lines,
        reason = "the structural recursion keeps all profile invariants visible in one exhaustive match"
    )]
    pub fn profile(&mut self, term: &Term, eliminated: &str) -> Result<Profile, KernelFailure> {
        let key = (term.clone(), eliminated.to_owned());
        if let Some(profile) = self.profiles.get(&key) {
            return Ok(profile.clone());
        }
        self.budget.tick("profile")?;

        let result = match term {
            Term::Constant(_) => Profile {
                outer_slope: Rational::zero(),
                segment_slope: Rational::zero(),
                period: Rational::zero(),
                delta_y: Rational::zero(),
                dist_y_minus: term.clone(),
                right_limit: term.clone(),
            },
            Term::Variable(name) => {
                let is_eliminated = name == eliminated;
                Profile {
                    outer_slope: integer(i64::from(is_eliminated)),
                    segment_slope: integer(i64::from(is_eliminated)),
                    period: Rational::zero(),
                    delta_y: Rational::zero(),
                    dist_y_minus: if is_eliminated {
                        int_constant(0)
                    } else {
                        term.clone()
                    },
                    right_limit: term.clone(),
                }
            }
            Term::Scale(coefficient, argument) => {
                let inner = self.profile(argument, eliminated)?;
                let lower = if coefficient.is_negative() {
                    scale(coefficient.clone(), inner.dist_y_plus())
                } else {
                    scale(coefficient.clone(), inner.dist_y_minus.clone())
                };
                Profile {
                    outer_slope: coefficient * &inner.outer_slope,
                    segment_slope: coefficient * &inner.segment_slope,
                    period: inner.period,
                    delta_y: coefficient.abs() * inner.delta_y,
                    dist_y_minus: lower,
                    right_limit: scale(coefficient.clone(), inner.right_limit),
                }
            }
            Term::Add(arguments) => {
                let mut parts = Vec::with_capacity(arguments.len());
                for argument in arguments {
                    parts.push(self.profile(argument, eliminated)?);
                }
                Profile {
                    outer_slope: parts
                        .iter()
                        .map(|part| &part.outer_slope)
                        .fold(Rational::zero(), |sum, value| sum + value),
                    segment_slope: parts
                        .iter()
                        .map(|part| &part.segment_slope)
                        .fold(Rational::zero(), |sum, value| sum + value),
                    period: rational_lcm(parts.iter().map(|part| &part.period)),
                    delta_y: parts
                        .iter()
                        .map(|part| &part.delta_y)
                        .fold(Rational::zero(), |sum, value| sum + value),
                    dist_y_minus: add(parts.iter().map(|part| part.dist_y_minus.clone())),
                    right_limit: add(parts.iter().map(|part| part.right_limit.clone())),
                }
            }
            Term::Floor(argument) => {
                let inner = self.profile(argument, eliminated)?;
                let period = if inner.period.is_zero() && inner.outer_slope.is_zero() {
                    Rational::zero()
                } else if inner.period.is_zero() {
                    Rational::one() / inner.outer_slope.abs()
                } else {
                    Rational::from_integer(inner.period.numer().abs() * inner.outer_slope.denom())
                };
                let right_limit = if inner.segment_slope.is_negative() {
                    add([ceil_term(inner.right_limit), int_constant(-1)])
                } else {
                    floor_term(inner.right_limit)
                };
                Profile {
                    outer_slope: inner.outer_slope,
                    segment_slope: Rational::zero(),
                    period,
                    delta_y: inner.delta_y + Rational::one(),
                    dist_y_minus: add([inner.dist_y_minus, int_constant(-1)]),
                    right_limit,
                }
            }
        };

        for value in [
            &result.outer_slope,
            &result.segment_slope,
            &result.period,
            &result.delta_y,
        ] {
            self.budget.check_fraction(value, "profile")?;
        }
        if result.period.is_negative() || result.delta_y.is_negative() {
            return Err(Self::invariant(
                "profile period and width must be nonnegative",
            ));
        }
        self.profiles.insert(key, result.clone());
        Ok(result)
    }

    pub fn segment_zero(
        &mut self,
        term: &Term,
        eliminated: &str,
        base: &Term,
    ) -> Result<Term, KernelFailure> {
        let profile = self.profile(term, eliminated)?;
        if profile.segment_slope.is_zero() {
            return Err(Self::invariant("segment zero needs nonzero segment slope"));
        }
        if base.contains(eliminated) {
            return Err(Self::invariant(
                "segment-zero base contains eliminated variable",
            ));
        }
        let limit_at_base = substitute(&profile.right_limit, eliminated, base);
        Ok(add([
            base.clone(),
            scale(-Rational::one() / profile.segment_slope, limit_at_base),
        ]))
    }

    pub fn core_interval(
        &mut self,
        literal: &Literal,
        eliminated: &str,
    ) -> Result<(Term, Term, Rational), KernelFailure> {
        let profile = self.profile(&literal.term, eliminated)?;
        if profile.outer_slope.is_zero() {
            return Err(Self::invariant("periodic literal has no aperiodic core"));
        }
        let signed = if profile.outer_slope.is_positive() {
            profile.dist_y_plus()
        } else {
            profile.dist_y_minus
        };
        let lower = scale(-Rational::one() / &profile.outer_slope, signed);
        let width = profile.delta_y / profile.outer_slope.abs();
        self.budget.check_fraction(&width, "core interval")?;
        let upper = add([lower.clone(), constant(width.clone())]);
        Ok((lower, upper, width))
    }

    pub fn limit_truth(
        &mut self,
        literal: &Literal,
        eliminated: &str,
        sign: InfinitySign,
    ) -> Result<bool, KernelFailure> {
        let profile = self.profile(&literal.term, eliminated)?;
        if profile.outer_slope.is_zero() {
            return Err(Self::invariant(
                "periodic literal has no constant infinity limit",
            ));
        }
        Ok(match literal.relation {
            Relation::Eq => false,
            Relation::Ne => true,
            Relation::Gt | Relation::Ge => {
                let positive = profile.outer_slope.is_positive();
                (sign.factor() > 0) == positive
            }
        })
    }

    fn remainder(term: Term, period: &Rational) -> Result<Term, KernelFailure> {
        if !period.is_positive() {
            return Err(Self::invariant("remainder period must be positive"));
        }
        Ok(add([
            term.clone(),
            scale(
                -period.clone(),
                floor_term(scale(Rational::one() / period, term)),
            ),
        ]))
    }

    pub fn grid_ceil(&self, grid: &Grid, term: Term) -> Result<Term, KernelFailure> {
        Ok(add([
            term.clone(),
            Self::remainder(subtract(grid.base.clone(), term), &grid.period)?,
        ]))
    }

    pub fn grid_floor(&self, grid: &Grid, term: Term) -> Result<Term, KernelFailure> {
        Ok(subtract(
            term.clone(),
            Self::remainder(subtract(term, grid.base.clone()), &grid.period)?,
        ))
    }

    #[allow(
        clippy::needless_pass_by_value,
        reason = "the symbolic lower bound is deliberately consumed into the generated representatives"
    )]
    pub fn grid_intersection(
        &mut self,
        grid: &Grid,
        lower: Term,
        width: &Rational,
        lower_closed: bool,
        upper_closed: bool,
    ) -> Result<Vec<Term>, KernelFailure> {
        self.budget.tick("grid intersection")?;
        self.budget.check_fraction(width, "grid intersection")?;
        if width.is_negative() {
            return Err(Self::invariant(
                "grid intersection width must be nonnegative",
            ));
        }
        let start = if lower_closed {
            self.grid_ceil(grid, lower.clone())?
        } else {
            self.grid_floor(grid, add([lower.clone(), constant(grid.period.clone())]))?
        };
        let quotient = width / &grid.period;
        let upper_index = if upper_closed {
            floor_rational(&quotient)
        } else {
            ceil_rational(&quotient) - BigInt::one()
        };
        if upper_index.is_negative() {
            return Ok(Vec::new());
        }

        let mut result = Vec::new();
        let mut seen = BTreeSet::new();
        let mut index = BigInt::zero();
        while index <= upper_index {
            self.budget.add_grid_point()?;
            let term = add([
                start.clone(),
                constant(Rational::from_integer(index.clone()) * &grid.period),
            ]);
            if seen.insert(term.clone()) {
                result.push(term);
            }
            index += BigInt::one();
        }
        Ok(result)
    }

    pub fn breaks(&mut self, term: &Term, eliminated: &str) -> Result<Vec<Grid>, KernelFailure> {
        let key = (term.clone(), eliminated.to_owned());
        if let Some(breaks) = self.breaks.get(&key) {
            return Ok(breaks.clone());
        }
        self.budget.tick("break construction")?;

        let result = match term {
            Term::Constant(_) | Term::Variable(_) => Vec::new(),
            Term::Scale(_, argument) => self.breaks(argument, eliminated)?,
            Term::Add(arguments) => {
                let mut grids = BTreeMap::new();
                for argument in arguments {
                    for grid in self.breaks(argument, eliminated)? {
                        grids.entry(grid.canonical_json()).or_insert(grid);
                    }
                }
                grids.into_values().collect()
            }
            Term::Floor(inner_term) => {
                let inner_profile = self.profile(inner_term, eliminated)?;
                let inner_breaks = self.breaks(inner_term, eliminated)?;
                if inner_profile.segment_slope.is_zero() {
                    inner_breaks
                } else if inner_breaks.is_empty() {
                    let period = self.profile(term, eliminated)?.period;
                    if !period.is_positive() {
                        return Err(Self::invariant("nonconstant floor needs positive period"));
                    }
                    vec![Grid::new(
                        self.segment_zero(inner_term, eliminated, &int_constant(0))?,
                        period,
                    )?]
                } else {
                    let period = self.profile(term, eliminated)?.period;
                    let minimum_period = inner_breaks
                        .iter()
                        .map(|grid| grid.period.clone())
                        .min()
                        .ok_or_else(|| Self::invariant("inner break set unexpectedly empty"))?;
                    let reciprocal_slope = (Rational::one() / &inner_profile.segment_slope).abs();
                    let mut generated = inner_breaks
                        .iter()
                        .cloned()
                        .map(|grid| (grid.canonical_json(), grid))
                        .collect::<BTreeMap<_, _>>();
                    for source in &inner_breaks {
                        let segment_bases = self.grid_intersection(
                            source,
                            source.base.clone(),
                            &period,
                            true,
                            false,
                        )?;
                        for segment_base in segment_bases {
                            let zero_grid = Grid::new(
                                self.segment_zero(inner_term, eliminated, &segment_base)?,
                                reciprocal_slope.clone(),
                            )?;
                            let breaks_in_segment = self.grid_intersection(
                                &zero_grid,
                                segment_base,
                                &minimum_period,
                                true,
                                false,
                            )?;
                            for break_base in breaks_in_segment {
                                let grid = Grid::new(break_base, period.clone())?;
                                generated.entry(grid.canonical_json()).or_insert(grid);
                            }
                        }
                    }
                    generated.into_values().collect()
                }
            }
        };

        for _ in &result {
            self.budget.add_grid()?;
        }
        self.breaks.insert(key, result.clone());
        Ok(result)
    }

    fn candidate(
        &mut self,
        output: &mut Vec<Candidate>,
        virtual_term: VirtualTerm,
        literal_index: usize,
        origin_kind: &'static str,
    ) -> Result<(), KernelFailure> {
        self.budget.add_candidate()?;
        output.push(Candidate {
            virtual_term,
            literal_index,
            origin_kind,
        });
        Ok(())
    }

    #[allow(
        clippy::too_many_lines,
        reason = "candidate completeness is audited as one ordered case split matching the calculus"
    )]
    pub fn literal_candidates(
        &mut self,
        literal: &Literal,
        eliminated: &str,
        literal_index: usize,
    ) -> Result<Vec<Candidate>, KernelFailure> {
        let profile = self.profile(&literal.term, eliminated)?;
        let breaks = self.breaks(&literal.term, eliminated)?;
        let mut output = Vec::new();

        if breaks.is_empty() {
            if profile.segment_slope.is_zero() {
                self.candidate(
                    &mut output,
                    VirtualTerm {
                        infinity: Some(InfinitySign::Negative),
                        ..VirtualTerm::default()
                    },
                    literal_index,
                    "negative_tail",
                )?;
            } else {
                let zero = self.segment_zero(&literal.term, eliminated, &int_constant(0))?;
                match literal.relation {
                    Relation::Ne => {
                        self.candidate(
                            &mut output,
                            VirtualTerm {
                                infinity: Some(InfinitySign::Negative),
                                ..VirtualTerm::default()
                            },
                            literal_index,
                            "negative_tail",
                        )?;
                        self.candidate(
                            &mut output,
                            VirtualTerm {
                                base: zero,
                                epsilon: true,
                                ..VirtualTerm::default()
                            },
                            literal_index,
                            "linear_zero_right",
                        )?;
                    }
                    Relation::Eq => {
                        self.candidate(
                            &mut output,
                            VirtualTerm {
                                base: zero,
                                ..VirtualTerm::default()
                            },
                            literal_index,
                            "linear_zero",
                        )?;
                    }
                    Relation::Gt | Relation::Ge if profile.segment_slope.is_positive() => {
                        self.candidate(
                            &mut output,
                            VirtualTerm {
                                base: zero,
                                epsilon: literal.relation == Relation::Gt,
                                ..VirtualTerm::default()
                            },
                            literal_index,
                            "linear_lower_bound",
                        )?;
                    }
                    Relation::Gt | Relation::Ge => {
                        self.candidate(
                            &mut output,
                            VirtualTerm {
                                infinity: Some(InfinitySign::Negative),
                                ..VirtualTerm::default()
                            },
                            literal_index,
                            "negative_tail",
                        )?;
                    }
                }
            }
            return Ok(output);
        }

        let periodic = profile.outer_slope.is_zero();
        let mut break_terms = Vec::new();
        if periodic {
            break_terms.extend(breaks.iter().map(|grid| VirtualTerm {
                base: grid.base.clone(),
                grid_period: Some(grid.period.clone()),
                ..VirtualTerm::default()
            }));
        } else {
            let (lower, _, width) = self.core_interval(literal, eliminated)?;
            for grid in &breaks {
                break_terms.extend(
                    self.grid_intersection(grid, lower.clone(), &width, false, false)?
                        .into_iter()
                        .map(|base| VirtualTerm {
                            base,
                            ..VirtualTerm::default()
                        }),
                );
            }
        }
        for virtual_term in &break_terms {
            self.candidate(
                &mut output,
                virtual_term.clone(),
                literal_index,
                "discontinuity",
            )?;
        }

        let mut zero_terms = Vec::new();
        if periodic && !profile.segment_slope.is_zero() {
            for grid in &breaks {
                zero_terms.push(VirtualTerm {
                    base: self.segment_zero(&literal.term, eliminated, &grid.base)?,
                    grid_period: Some(grid.period.clone()),
                    ..VirtualTerm::default()
                });
            }
        } else if !profile.segment_slope.is_zero() {
            let (lower, _, width) = self.core_interval(literal, eliminated)?;
            if profile.outer_slope == profile.segment_slope {
                for grid in &breaks {
                    zero_terms.push(VirtualTerm {
                        base: self.segment_zero(&literal.term, eliminated, &grid.base)?,
                        ..VirtualTerm::default()
                    });
                }
            } else {
                for grid in &breaks {
                    let zero_period = ((Rational::one()
                        - &profile.outer_slope / &profile.segment_slope)
                        * &grid.period)
                        .abs();
                    if zero_period.is_zero() {
                        return Err(Self::invariant("zero-grid period must be positive"));
                    }
                    let zero_grid = Grid::new(
                        self.segment_zero(&literal.term, eliminated, &grid.base)?,
                        zero_period,
                    )?;
                    zero_terms.extend(
                        self.grid_intersection(&zero_grid, lower.clone(), &width, false, false)?
                            .into_iter()
                            .map(|base| VirtualTerm {
                                base,
                                ..VirtualTerm::default()
                            }),
                    );
                }
            }
        }

        let segment_terms = if profile.segment_slope.is_zero()
            || (profile.segment_slope.is_negative()
                && matches!(literal.relation, Relation::Gt | Relation::Ge))
        {
            break_terms
                .iter()
                .map(VirtualTerm::with_epsilon)
                .collect::<Vec<_>>()
        } else if profile.segment_slope.is_positive() && literal.relation == Relation::Ge {
            break_terms
                .iter()
                .map(VirtualTerm::with_epsilon)
                .chain(zero_terms.iter().cloned())
                .collect()
        } else if profile.segment_slope.is_positive() && literal.relation == Relation::Gt {
            break_terms
                .iter()
                .map(VirtualTerm::with_epsilon)
                .chain(zero_terms.iter().map(VirtualTerm::with_epsilon))
                .collect()
        } else if !profile.segment_slope.is_zero() && literal.relation == Relation::Ne {
            break_terms
                .iter()
                .chain(&zero_terms)
                .map(VirtualTerm::with_epsilon)
                .collect()
        } else if !profile.segment_slope.is_zero() && literal.relation == Relation::Eq {
            zero_terms
        } else {
            Vec::new()
        };
        for virtual_term in segment_terms {
            self.candidate(
                &mut output,
                virtual_term,
                literal_index,
                "segment_candidate",
            )?;
        }

        if !periodic {
            let (lower, upper, _) = self.core_interval(literal, eliminated)?;
            let positive = self.limit_truth(literal, eliminated, InfinitySign::Positive)?;
            let negative = self.limit_truth(literal, eliminated, InfinitySign::Negative)?;
            self.candidate(
                &mut output,
                VirtualTerm {
                    base: upper.clone(),
                    ..VirtualTerm::default()
                },
                literal_index,
                "core_upper",
            )?;
            if positive {
                self.candidate(
                    &mut output,
                    VirtualTerm {
                        base: upper,
                        epsilon: true,
                        ..VirtualTerm::default()
                    },
                    literal_index,
                    "core_upper_right",
                )?;
            }
            self.candidate(
                &mut output,
                VirtualTerm {
                    base: lower.clone(),
                    ..VirtualTerm::default()
                },
                literal_index,
                "core_lower",
            )?;
            // A descending step equality can become true immediately to the
            // right of its conservative core lower bound while being false at
            // the bound itself. The boundary value alone is then incomplete
            // when another conjunct cuts the step's interior before its next
            // discontinuity.
            if literal.relation == Relation::Eq
                && profile.segment_slope.is_zero()
                && profile.outer_slope.is_negative()
            {
                self.candidate(
                    &mut output,
                    VirtualTerm {
                        base: lower,
                        epsilon: true,
                        ..VirtualTerm::default()
                    },
                    literal_index,
                    "core_lower_right",
                )?;
            }
            if negative {
                self.candidate(
                    &mut output,
                    VirtualTerm {
                        infinity: Some(InfinitySign::Negative),
                        ..VirtualTerm::default()
                    },
                    literal_index,
                    "negative_tail",
                )?;
            }
        }
        Ok(output)
    }

    #[allow(
        clippy::too_many_lines,
        reason = "the mutually exclusive V1, V2, and V3 proof cases remain adjacent for auditability"
    )]
    pub fn flatten_grid(
        &mut self,
        literals: &[Literal],
        eliminated: &str,
        virtual_term: &VirtualTerm,
    ) -> Result<Vec<VirtualTerm>, KernelFailure> {
        let Some(virtual_period) = virtual_term.grid_period.as_ref() else {
            return Err(Self::invariant("flatten_grid requires a grid virtual term"));
        };
        self.budget.tick("grid flatten")?;

        let mut periodic = Vec::new();
        let mut aperiodic = Vec::new();
        for literal in literals {
            if self
                .profile(&literal.term, eliminated)?
                .outer_slope
                .is_zero()
            {
                periodic.push(literal);
            } else {
                aperiodic.push(literal);
            }
        }
        let mut periods = vec![virtual_period.clone()];
        for literal in periodic {
            let period = self.profile(&literal.term, eliminated)?.period;
            if !period.is_zero() {
                periods.push(period);
            }
        }
        let common_period = rational_lcm(periods.iter());
        if !common_period.is_positive() {
            return Err(Self::invariant(
                "grid flatten needs a positive common period",
            ));
        }
        let grid = Grid::new(virtual_term.base.clone(), virtual_period.clone())?;

        let mut qualifying_signs = Vec::new();
        for sign in [InfinitySign::Negative, InfinitySign::Positive] {
            let mut qualifies = true;
            for literal in &aperiodic {
                if !self.limit_truth(literal, eliminated, sign)? {
                    qualifies = false;
                    break;
                }
            }
            if qualifies {
                qualifying_signs.push(sign);
            }
        }

        let (case, flattened) = if qualifying_signs.is_empty() {
            let equalities = aperiodic
                .iter()
                .copied()
                .filter(|literal| literal.relation == Relation::Eq)
                .collect::<Vec<_>>();
            if equalities.is_empty() {
                let mut flattened = Vec::new();
                for literal in aperiodic {
                    if !self.limit_truth(literal, eliminated, InfinitySign::Negative)? {
                        let (lower, _, width) = self.core_interval(literal, eliminated)?;
                        flattened.extend(
                            self.grid_intersection(
                                &grid,
                                lower,
                                &(width + &common_period),
                                true,
                                true,
                            )?
                            .into_iter()
                            .map(|base| VirtualTerm {
                                base,
                                epsilon: virtual_term.epsilon,
                                ..VirtualTerm::default()
                            }),
                        );
                    }
                }
                if flattened.is_empty() {
                    return Err(Self::invariant("V3 needs a negative-tail blocker"));
                }
                ("V3", flattened)
            } else {
                let mut chosen = None::<(&Literal, Rational)>;
                for literal in equalities {
                    let width = self.core_interval(literal, eliminated)?.2;
                    if chosen
                        .as_ref()
                        .is_none_or(|(_, chosen_width)| width < *chosen_width)
                    {
                        chosen = Some((literal, width));
                    }
                }
                let (chosen, width) =
                    chosen.ok_or_else(|| Self::invariant("V2 equality set unexpectedly empty"))?;
                let (lower, _, _) = self.core_interval(chosen, eliminated)?;
                let flattened = self
                    .grid_intersection(&grid, lower, &width, true, true)?
                    .into_iter()
                    .map(|base| VirtualTerm {
                        base,
                        epsilon: virtual_term.epsilon,
                        ..VirtualTerm::default()
                    })
                    .collect();
                ("V2", flattened)
            }
        } else {
            let representatives = self.grid_intersection(
                &grid,
                virtual_term.base.clone(),
                &common_period,
                true,
                false,
            )?;
            let mut flattened = Vec::new();
            for sign in qualifying_signs {
                flattened.extend(representatives.iter().cloned().map(|base| VirtualTerm {
                    base,
                    epsilon: virtual_term.epsilon,
                    infinity: Some(sign),
                    ..VirtualTerm::default()
                }));
            }
            ("V1", flattened)
        };

        let mut seen = BTreeSet::new();
        let unique = flattened
            .into_iter()
            .filter(|item| seen.insert(item.clone()))
            .collect::<Vec<_>>();
        self.flatten_records.push(GridFlattenRecord {
            case,
            input: virtual_term.clone(),
            common_period,
            output: unique.clone(),
        });
        Ok(unique)
    }

    fn epsilon_literal(
        &mut self,
        literal: &Literal,
        eliminated: &str,
        base: &Term,
    ) -> Result<Formula, KernelFailure> {
        let profile = self.profile(&literal.term, eliminated)?;
        let limit = substitute(&profile.right_limit, eliminated, base);
        Ok(match literal.relation {
            Relation::Eq if !profile.segment_slope.is_zero() => boolean(false),
            Relation::Eq => atom(Literal::new(limit, Relation::Eq)),
            Relation::Ne if !profile.segment_slope.is_zero() => boolean(true),
            Relation::Ne => atom(Literal::new(limit, Relation::Ne)),
            relation if profile.segment_slope.is_positive() => atom(Literal::new(
                limit,
                if self.mutations.drop_epsilon_strictness {
                    relation
                } else {
                    Relation::Ge
                },
            )),
            relation if profile.segment_slope.is_zero() => atom(Literal::new(limit, relation)),
            Relation::Gt | Relation::Ge => atom(Literal::new(limit, Relation::Gt)),
        })
    }

    fn virtual_literal(
        &mut self,
        literal: &Literal,
        eliminated: &str,
        virtual_term: &VirtualTerm,
    ) -> Result<Formula, KernelFailure> {
        let profile = self.profile(&literal.term, eliminated)?;
        if let Some(infinity) = virtual_term.infinity {
            let mut is_aperiodic = !profile.outer_slope.is_zero();
            if self.mutations.reverse_infinity_periodicity {
                is_aperiodic = !is_aperiodic;
            }
            if is_aperiodic {
                if profile.outer_slope.is_zero() {
                    return Ok(boolean(literal.relation == Relation::Ne));
                }
                return Ok(boolean(self.limit_truth(literal, eliminated, infinity)?));
            }
            let mut finite = virtual_term.clone();
            finite.infinity = None;
            return self.virtual_literal(literal, eliminated, &finite);
        }
        if virtual_term.epsilon {
            return self.epsilon_literal(literal, eliminated, &virtual_term.base);
        }
        Ok(atom(Literal::new(
            substitute(&literal.term, eliminated, &virtual_term.base),
            literal.relation,
        )))
    }

    pub fn virtual_substitute(
        &mut self,
        literals: &[Literal],
        eliminated: &str,
        virtual_term: &VirtualTerm,
    ) -> Result<Formula, KernelFailure> {
        self.budget.tick("virtual substitution")?;
        if virtual_term.grid_period.is_some() {
            let flattened = self.flatten_grid(literals, eliminated, virtual_term)?;
            let mut children = Vec::new();
            for finite in flattened {
                children.push(self.virtual_substitute(literals, eliminated, &finite)?);
            }
            return Ok(disjunction(children));
        }
        let mut children = Vec::with_capacity(literals.len());
        for literal in literals {
            children.push(self.virtual_literal(literal, eliminated, virtual_term)?);
        }
        Ok(conjunction(children))
    }

    fn check_formula_size(&self, formula: &Formula) -> Result<(), KernelFailure> {
        let mut stack = vec![formula];
        let mut count = 0_usize;
        while let Some(current) = stack.pop() {
            count = count.saturating_add(1);
            if count > self.budget.limits.max_formula_nodes {
                return Err(KernelFailure::resource(format!(
                    "formula-node limit exceeded: {count}>{}",
                    self.budget.limits.max_formula_nodes
                )));
            }
            match current {
                Formula::And(children) | Formula::Or(children) => stack.extend(children),
                Formula::Exists(_, body) | Formula::Forall(_, body) => stack.push(body),
                Formula::Bool(_) | Formula::Atom(_) => {}
            }
        }
        Ok(())
    }

    fn derivation(&self, eliminated: &str, candidates: Vec<Candidate>) -> Derivation {
        Derivation {
            calculus: "paper-derived-base-viras-one-conjunction-v1",
            eliminated: eliminated.to_owned(),
            candidates,
            grid_flattening: self.flatten_records.clone(),
            resource_usage: self.budget.usage,
        }
    }

    fn eliminate_exists_inner(
        &mut self,
        eliminated: &str,
        literals: &[Literal],
    ) -> Result<(Formula, &'static str, Vec<Candidate>), KernelFailure> {
        if literals.is_empty() {
            return Err(KernelFailure::unsupported(
                "kernel requires a nonempty conjunction of normalized literals",
            ));
        }
        let independent = literals
            .iter()
            .filter(|literal| !literal.term.contains(eliminated))
            .cloned()
            .collect::<Vec<_>>();
        let dependent = literals
            .iter()
            .filter(|literal| literal.term.contains(eliminated))
            .cloned()
            .collect::<Vec<_>>();
        let independent_formula = conjunction(independent.into_iter().map(atom));
        if dependent.is_empty() {
            self.check_formula_size(&independent_formula)?;
            return Ok((
                independent_formula,
                "eliminated variable absent",
                Vec::new(),
            ));
        }

        let mut candidates = Vec::new();
        for (index, literal) in dependent.iter().enumerate() {
            candidates.extend(self.literal_candidates(literal, eliminated, index)?);
        }
        let mut seen = BTreeSet::new();
        candidates.retain(|candidate| seen.insert(candidate.virtual_term.clone()));
        if self.mutations.omit_last_candidate {
            candidates.pop();
        }

        let result = if candidates.is_empty() {
            conjunction([independent_formula, boolean(false)])
        } else {
            let mut substitutions = Vec::with_capacity(candidates.len());
            for candidate in &candidates {
                substitutions.push(self.virtual_substitute(
                    &dependent,
                    eliminated,
                    &candidate.virtual_term,
                )?);
            }
            conjunction([independent_formula, disjunction(substitutions)])
        };
        if result.variables().contains(eliminated) {
            return Err(Self::invariant(
                "successful result retains eliminated variable",
            ));
        }
        self.check_formula_size(&result)?;
        Ok((result, "complete finite virtual substitution", candidates))
    }

    #[must_use]
    pub fn eliminate_exists(&mut self, eliminated: &str, literals: &[Literal]) -> QeOutcome {
        self.flatten_records.clear();
        match self.eliminate_exists_inner(eliminated, literals) {
            Ok((formula, reason, candidates)) => {
                QeOutcome::success(formula, reason, self.derivation(eliminated, candidates))
            }
            Err(failure) => QeOutcome::unknown(failure, self.derivation(eliminated, Vec::new())),
        }
    }
}

#[must_use]
pub fn eliminate_exists(eliminated: &str, literals: &[Literal], limits: Limits) -> QeOutcome {
    Kernel::new(limits).eliminate_exists(eliminated, literals)
}

/// Replays and validates a successful one-conjunction derivation.
///
/// This check independently regenerates the complete candidate list, replays
/// every recorded virtual substitution, and compares both the resulting
/// formula and the grid-flattening trace.
///
/// # Errors
///
/// Returns an unsupported-fragment failure when the outcome is not successful
/// or any recorded proof component differs from a fresh derivation. Resource
/// failures are propagated from the replay kernel.
pub fn validate_derivation(
    eliminated: &str,
    literals: &[Literal],
    outcome: &QeOutcome,
    limits: Limits,
) -> Result<(), KernelFailure> {
    if outcome.status != QeStatus::Success {
        return Err(KernelFailure::unsupported(
            "only successful outcomes have replayable derivations",
        ));
    }
    if outcome.derivation.eliminated != eliminated {
        return Err(KernelFailure::unsupported(
            "derivation names a different eliminated variable",
        ));
    }
    let recorded_formula = outcome
        .formula
        .as_ref()
        .ok_or_else(|| KernelFailure::unsupported("successful outcome has no formula"))?;
    let independent = literals
        .iter()
        .filter(|literal| !literal.term.contains(eliminated))
        .cloned()
        .collect::<Vec<_>>();
    let dependent = literals
        .iter()
        .filter(|literal| literal.term.contains(eliminated))
        .cloned()
        .collect::<Vec<_>>();
    let independent_formula = conjunction(independent.into_iter().map(atom));
    let mut replay = Kernel::new(limits);

    let expected_formula = if dependent.is_empty() {
        if !outcome.derivation.candidates.is_empty() {
            return Err(KernelFailure::unsupported(
                "variable-absent derivation records candidates",
            ));
        }
        independent_formula
    } else {
        let mut regenerated = Vec::new();
        for (index, literal) in dependent.iter().enumerate() {
            regenerated.extend(replay.literal_candidates(literal, eliminated, index)?);
        }
        let mut seen = BTreeSet::new();
        regenerated.retain(|candidate| seen.insert(candidate.virtual_term.clone()));
        if regenerated != outcome.derivation.candidates {
            return Err(KernelFailure::unsupported(
                "recorded candidate list differs from complete regeneration",
            ));
        }
        let mut substitutions = Vec::with_capacity(regenerated.len());
        for candidate in &regenerated {
            substitutions.push(replay.virtual_substitute(
                &dependent,
                eliminated,
                &candidate.virtual_term,
            )?);
        }
        conjunction([independent_formula, disjunction(substitutions)])
    };

    if &expected_formula != recorded_formula {
        return Err(KernelFailure::unsupported(
            "recorded formula differs from candidate replay",
        ));
    }
    if replay.flatten_records != outcome.derivation.grid_flattening {
        return Err(KernelFailure::unsupported(
            "recorded grid-flattening trace differs from replay",
        ));
    }
    Ok(())
}

/// Negates a formula while preserving negation normal form.
#[must_use]
pub fn negate_formula(formula: Formula) -> Formula {
    match formula {
        Formula::Bool(value) => boolean(!value),
        Formula::Atom(literal) => match literal.relation {
            Relation::Eq => atom(Literal::new(literal.term, Relation::Ne)),
            Relation::Ne => atom(Literal::new(literal.term, Relation::Eq)),
            Relation::Gt => atom(Literal::new(negate(literal.term), Relation::Ge)),
            Relation::Ge => atom(Literal::new(negate(literal.term), Relation::Gt)),
        },
        Formula::And(children) => disjunction(children.into_iter().map(negate_formula)),
        Formula::Or(children) => conjunction(children.into_iter().map(negate_formula)),
        Formula::Exists(variable, body) => {
            Formula::Forall(variable, Box::new(negate_formula(*body)))
        }
        Formula::Forall(variable, body) => {
            Formula::Exists(variable, Box::new(negate_formula(*body)))
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct FormulaBudgetSnapshot {
    pub kernel: BudgetSnapshot,
    pub dnf_branches: usize,
    pub quantifiers: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FormulaDerivation {
    pub calculus: &'static str,
    pub eliminations: Vec<Derivation>,
    pub replay_validated: bool,
    pub resource_usage: FormulaBudgetSnapshot,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FormulaQeOutcome {
    pub status: QeStatus,
    pub formula: Option<Formula>,
    pub unknown_kind: Option<UnknownKind>,
    pub reason: String,
    pub derivation: FormulaDerivation,
}

#[derive(Clone, Debug)]
struct FormulaBudget {
    limits: Limits,
    usage: FormulaBudgetSnapshot,
}

impl FormulaBudget {
    const fn new(limits: Limits) -> Self {
        Self {
            limits,
            usage: FormulaBudgetSnapshot {
                kernel: BudgetSnapshot {
                    steps: 0,
                    candidates: 0,
                    grids: 0,
                    grid_points: 0,
                },
                dnf_branches: 0,
                quantifiers: 0,
            },
        }
    }

    fn tick(&mut self, label: &str) -> Result<(), KernelFailure> {
        self.usage.kernel.steps = self.usage.kernel.steps.saturating_add(1);
        if self.usage.kernel.steps > self.limits.max_steps {
            return Err(KernelFailure::resource(format!(
                "step limit exceeded during {label}: {}>{}",
                self.usage.kernel.steps, self.limits.max_steps
            )));
        }
        Ok(())
    }

    fn add_quantifier(&mut self) {
        self.usage.quantifiers = self.usage.quantifiers.saturating_add(1);
    }

    fn add_dnf_branch(&mut self) -> Result<(), KernelFailure> {
        self.usage.dnf_branches = self.usage.dnf_branches.saturating_add(1);
        if self.usage.dnf_branches > self.limits.max_dnf_branches {
            return Err(KernelFailure::resource(format!(
                "DNF branch limit exceeded: {}>{}",
                self.usage.dnf_branches, self.limits.max_dnf_branches
            )));
        }
        Ok(())
    }

    const fn remaining_limits(&self) -> Limits {
        Limits {
            max_steps: self
                .limits
                .max_steps
                .saturating_sub(self.usage.kernel.steps),
            max_candidates: self
                .limits
                .max_candidates
                .saturating_sub(self.usage.kernel.candidates),
            max_grids: self
                .limits
                .max_grids
                .saturating_sub(self.usage.kernel.grids),
            max_grid_points: self
                .limits
                .max_grid_points
                .saturating_sub(self.usage.kernel.grid_points),
            max_dnf_branches: self.limits.max_dnf_branches,
            max_formula_nodes: self.limits.max_formula_nodes,
            max_rational_bits: self.limits.max_rational_bits,
        }
    }

    fn absorb(&mut self, usage: BudgetSnapshot) {
        self.usage.kernel.steps = self.usage.kernel.steps.saturating_add(usage.steps);
        self.usage.kernel.candidates = self
            .usage
            .kernel
            .candidates
            .saturating_add(usage.candidates);
        self.usage.kernel.grids = self.usage.kernel.grids.saturating_add(usage.grids);
        self.usage.kernel.grid_points = self
            .usage
            .kernel
            .grid_points
            .saturating_add(usage.grid_points);
    }
}

/// Counts canonical formula nodes for resource accounting and telemetry.
#[must_use]
pub fn formula_node_count(formula: &Formula) -> usize {
    let mut count = 0_usize;
    let mut pending = vec![formula];
    while let Some(current) = pending.pop() {
        count = count.saturating_add(1);
        match current {
            Formula::And(children) | Formula::Or(children) => pending.extend(children),
            Formula::Exists(_, body) | Formula::Forall(_, body) => pending.push(body),
            Formula::Bool(_) | Formula::Atom(_) => {}
        }
    }
    count
}

fn check_formula_rational_bits(
    formula: &Formula,
    max_rational_bits: u64,
) -> Result<(), KernelFailure> {
    let mut formulas = vec![formula];
    let mut terms = Vec::new();
    while let Some(current) = formulas.pop() {
        match current {
            Formula::Atom(literal) => terms.push(&literal.term),
            Formula::And(children) | Formula::Or(children) => formulas.extend(children),
            Formula::Exists(_, body) | Formula::Forall(_, body) => formulas.push(body),
            Formula::Bool(_) => {}
        }
    }
    while let Some(current) = terms.pop() {
        match current {
            Term::Constant(value) => {
                let bits = value.numer().magnitude().bits().max(value.denom().bits());
                if bits > max_rational_bits {
                    return Err(KernelFailure::resource(format!(
                        "rational bit limit exceeded during formula preflight: \
                         {bits}>{max_rational_bits}"
                    )));
                }
            }
            Term::Scale(coefficient, argument) => {
                let bits = coefficient
                    .numer()
                    .magnitude()
                    .bits()
                    .max(coefficient.denom().bits());
                if bits > max_rational_bits {
                    return Err(KernelFailure::resource(format!(
                        "rational bit limit exceeded during formula preflight: \
                         {bits}>{max_rational_bits}"
                    )));
                }
                terms.push(argument);
            }
            Term::Add(arguments) => terms.extend(arguments),
            Term::Floor(argument) => terms.push(argument),
            Term::Variable(_) => {}
        }
    }
    Ok(())
}

fn to_dnf(
    formula: Formula,
    budget: &mut FormulaBudget,
) -> Result<Vec<Vec<Literal>>, KernelFailure> {
    budget.tick("DNF conversion")?;
    match formula {
        Formula::Bool(false) => Ok(Vec::new()),
        Formula::Bool(true) => {
            budget.add_dnf_branch()?;
            Ok(vec![Vec::new()])
        }
        Formula::Atom(literal) => {
            budget.add_dnf_branch()?;
            Ok(vec![vec![literal]])
        }
        Formula::Or(children) => {
            let mut result = Vec::new();
            for child in children {
                result.extend(to_dnf(child, budget)?);
                if result.len() > budget.limits.max_dnf_branches {
                    return Err(KernelFailure::resource(format!(
                        "DNF branch limit exceeded: {}>{}",
                        result.len(),
                        budget.limits.max_dnf_branches
                    )));
                }
            }
            Ok(result)
        }
        Formula::And(children) => {
            let mut result = vec![Vec::new()];
            for child in children {
                let child_branches = to_dnf(child, budget)?;
                if child_branches.is_empty() {
                    return Ok(Vec::new());
                }
                let mut product = Vec::new();
                for prefix in &result {
                    for suffix in &child_branches {
                        budget.add_dnf_branch()?;
                        let mut merged = prefix.clone();
                        merged.extend(suffix.iter().cloned());
                        merged.sort();
                        merged.dedup();
                        product.push(merged);
                    }
                }
                result = product;
            }
            Ok(result)
        }
        Formula::Exists(_, _) | Formula::Forall(_, _) => Err(KernelFailure::unsupported(
            "DNF conversion received an uneliminated quantifier",
        )),
    }
}

fn eliminate_existential_formula(
    eliminated: &str,
    body: Formula,
    budget: &mut FormulaBudget,
    derivations: &mut Vec<Derivation>,
) -> Result<Formula, KernelFailure> {
    let branches = to_dnf(body, budget)?;
    if branches.is_empty() {
        return Ok(boolean(false));
    }
    let mut results = Vec::with_capacity(branches.len());
    for literals in branches {
        if literals.is_empty() {
            results.push(boolean(true));
            continue;
        }
        let remaining = budget.remaining_limits();
        let outcome = Kernel::new(remaining).eliminate_exists(eliminated, &literals);
        budget.absorb(outcome.derivation.resource_usage);
        if outcome.status != QeStatus::Success {
            return Err(KernelFailure {
                kind: outcome
                    .unknown_kind
                    .unwrap_or(UnknownKind::UnsupportedFragment),
                reason: outcome.reason,
            });
        }
        validate_derivation(eliminated, &literals, &outcome, remaining).map_err(|failure| {
            KernelFailure::unsupported(format!(
                "successful branch failed derivation replay: {}",
                failure.reason
            ))
        })?;
        let formula = outcome
            .formula
            .ok_or_else(|| KernelFailure::unsupported("successful branch has no formula"))?;
        derivations.push(outcome.derivation);
        results.push(formula);
    }
    Ok(disjunction(results))
}

fn eliminate_formula_inner(
    formula: Formula,
    budget: &mut FormulaBudget,
    derivations: &mut Vec<Derivation>,
) -> Result<Formula, KernelFailure> {
    budget.tick("formula traversal")?;
    match formula {
        Formula::Bool(_) | Formula::Atom(_) => Ok(formula),
        Formula::And(children) => {
            let mut results = Vec::with_capacity(children.len());
            for child in children {
                results.push(eliminate_formula_inner(child, budget, derivations)?);
            }
            Ok(conjunction(results))
        }
        Formula::Or(children) => {
            let mut results = Vec::with_capacity(children.len());
            for child in children {
                results.push(eliminate_formula_inner(child, budget, derivations)?);
            }
            Ok(disjunction(results))
        }
        Formula::Exists(variable, body) => {
            budget.add_quantifier();
            let body = eliminate_formula_inner(*body, budget, derivations)?;
            eliminate_existential_formula(&variable, body, budget, derivations)
        }
        Formula::Forall(variable, body) => {
            budget.add_quantifier();
            let negated_body = negate_formula(*body);
            let negated_body = eliminate_formula_inner(negated_body, budget, derivations)?;
            eliminate_existential_formula(&variable, negated_body, budget, derivations)
                .map(negate_formula)
        }
    }
}

/// Eliminates every quantifier from a bounded Boolean combination.
///
/// The wrapper converts each innermost existential body to bounded DNF and
/// applies the one-conjunction kernel to each branch. Universal quantifiers
/// use exact NNF duality. All branches share the supplied resource limits.
#[must_use]
pub fn eliminate_formula(formula: Formula, limits: Limits) -> FormulaQeOutcome {
    let mut budget = FormulaBudget::new(limits);
    let mut eliminations = Vec::new();
    let result = check_formula_rational_bits(&formula, limits.max_rational_bits)
        .and_then(|()| eliminate_formula_inner(formula, &mut budget, &mut eliminations));
    let result = result.and_then(|formula| {
        let count = formula_node_count(&formula);
        if count > limits.max_formula_nodes {
            Err(KernelFailure::resource(format!(
                "formula-node limit exceeded: {count}>{}",
                limits.max_formula_nodes
            )))
        } else if matches!(&formula, Formula::Exists(_, _) | Formula::Forall(_, _)) {
            Err(KernelFailure::unsupported(
                "successful wrapper result retains a quantifier",
            ))
        } else {
            Ok(formula)
        }
    });
    let derivation = FormulaDerivation {
        calculus: "bounded-nnf-dnf-base-viras-wrapper-v1",
        eliminations,
        replay_validated: result.is_ok(),
        resource_usage: budget.usage,
    };
    match result {
        Ok(formula) => FormulaQeOutcome {
            status: QeStatus::Success,
            formula: Some(formula),
            unknown_kind: None,
            reason: "all arithmetic quantifiers eliminated".to_owned(),
            derivation,
        },
        Err(failure) => FormulaQeOutcome {
            status: QeStatus::Unknown,
            formula: None,
            unknown_kind: Some(failure.kind),
            reason: failure.reason,
            derivation,
        },
    }
}

/// Independently validates a successful formula-level quantifier-elimination
/// publication against its typed-source import.
///
/// The checker re-runs the bounded wrapper in a fresh kernel. That run
/// regenerates and replays every branch proof before this outer comparison
/// checks the complete result and derivation record. Callers must complete
/// this check before inserting a transformed formula into proof search.
///
/// # Errors
///
/// Returns a fail-closed kernel failure if either record is unsuccessful, the
/// publication claims validation it did not receive, or any result,
/// resource-use, candidate, grid, or branch-derivation field differs.
pub fn validate_formula_derivation(
    source: &Formula,
    published: &FormulaQeOutcome,
    limits: Limits,
) -> Result<(), KernelFailure> {
    if published.status != QeStatus::Success {
        return Err(KernelFailure::unsupported(
            "only successful formula outcomes can be published",
        ));
    }
    if !published.derivation.replay_validated {
        return Err(KernelFailure::unsupported(
            "published formula derivation is not replay-validated",
        ));
    }
    if published.formula.is_none() {
        return Err(KernelFailure::unsupported(
            "successful published formula has no result",
        ));
    }

    let checked = eliminate_formula(source.clone(), limits);
    if checked.status != QeStatus::Success || !checked.derivation.replay_validated {
        return Err(KernelFailure::unsupported(
            "fresh formula-level validation did not produce a checked result",
        ));
    }
    if checked.formula != published.formula {
        return Err(KernelFailure::unsupported(
            "published formula differs from fresh checked elimination",
        ));
    }
    if checked.derivation != published.derivation {
        return Err(KernelFailure::unsupported(
            "published formula derivation differs from fresh checked elimination",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn q(numerator: i64, denominator: i64) -> Rational {
        rational(numerator, denominator).expect("test rational has a nonzero denominator")
    }

    fn environment(bindings: &[(&str, Rational)]) -> BTreeMap<String, Rational> {
        bindings
            .iter()
            .map(|(name, value)| ((*name).to_owned(), value.clone()))
            .collect()
    }

    fn remainder(term: Term, period: Rational) -> Term {
        subtract(
            term.clone(),
            scale(
                period.clone(),
                floor_term(scale(Rational::one() / period, term)),
            ),
        )
    }

    fn subtract_remainders(left: Term, right: Term, period: Rational) -> Term {
        subtract(remainder(left, period.clone()), remainder(right, period))
    }

    fn result_value(outcome: &QeOutcome, bindings: &[(&str, Rational)]) -> bool {
        assert_eq!(outcome.status, QeStatus::Success, "{}", outcome.reason);
        outcome
            .formula
            .as_ref()
            .expect("successful outcome has a formula")
            .evaluate(&environment(bindings))
            .expect("test result is closed under the supplied environment")
    }

    #[test]
    fn exact_arithmetic_vectors_are_mathematical() {
        assert_eq!(rational_lcm([q(1, 3), q(1, 2)].iter()), Rational::one());
        assert_eq!(rational_lcm([q(2, 3), q(4, 5)].iter()), integer(4));
        assert_eq!(rational_lcm([q(3, 10), q(9, 14)].iter()), q(9, 2));

        for (value, period, quotient, expected_remainder) in [
            (integer(7), integer(3), integer(2), integer(1)),
            (integer(-1), integer(3), integer(-1), integer(2)),
            (q(-1, 2), q(2, 3), integer(-1), q(1, 6)),
        ] {
            let actual_quotient = Rational::from_integer(floor_rational(&(&value / &period)));
            assert_eq!(actual_quotient, quotient);
            assert_eq!(value - period * actual_quotient, expected_remainder);
        }
        assert_eq!(
            evaluate_term(&floor_term(constant(q(-1, 2))), &BTreeMap::new())
                .expect("closed term evaluates"),
            integer(-1)
        );
        assert_eq!(
            floor_term(Term::Add(vec![int_constant(1), int_constant(2)])),
            int_constant(3)
        );
    }

    #[test]
    fn grid_intersections_cover_documented_boundaries() {
        let mut kernel = Kernel::new(Limits::default());
        let a = variable("a");
        let grid = Grid::new(int_constant(1), integer(2)).expect("positive period");
        let result = kernel
            .grid_intersection(&grid, a, &integer(4), true, false)
            .expect("grid intersection succeeds");
        assert_eq!(result.len(), 2);
        for value in [q(-5, 2), Rational::zero(), q(7, 3)] {
            let evaluated = result
                .iter()
                .map(|term| {
                    evaluate_term(term, &environment(&[("a", value.clone())]))
                        .expect("representative evaluates")
                })
                .collect::<BTreeSet<_>>();
            let actual = (-20_i64..=20)
                .map(|index| integer(1 + 2 * index))
                .filter(|point| value <= *point && *point < &value + integer(4))
                .collect::<BTreeSet<_>>();
            assert!(actual.is_subset(&evaluated));
        }

        let unit = Grid::new(int_constant(0), Rational::one()).expect("positive period");
        assert_eq!(
            kernel
                .grid_intersection(&unit, constant(q(1, 2)), &Rational::zero(), true, true)
                .expect("closed zero-width intersection")
                .len(),
            1
        );
        assert!(kernel
            .grid_intersection(&unit, constant(q(1, 2)), &Rational::zero(), false, false)
            .expect("open zero-width intersection")
            .is_empty());
    }

    #[test]
    fn documented_profiles_and_breaks_match_the_prototype() {
        let x = variable("x");
        let z = variable("z");
        let c = variable("c");

        let mut kernel = Kernel::new(Limits::default());
        let linear = kernel.profile(&x, "x").expect("linear profile");
        assert_eq!(
            (
                linear.outer_slope,
                linear.segment_slope,
                linear.period,
                linear.delta_y
            ),
            (
                Rational::one(),
                Rational::one(),
                Rational::zero(),
                Rational::zero()
            )
        );

        let floor_three_x = floor_term(scale(integer(3), x.clone()));
        let mut kernel = Kernel::new(Limits::default());
        let profile = kernel.profile(&floor_three_x, "x").expect("floor profile");
        assert_eq!(
            (
                profile.outer_slope,
                profile.segment_slope,
                profile.period,
                profile.delta_y,
                profile.dist_y_minus
            ),
            (
                integer(3),
                Rational::zero(),
                q(1, 3),
                Rational::one(),
                int_constant(-1)
            )
        );
        assert_eq!(
            kernel.breaks(&floor_three_x, "x").expect("floor breaks"),
            vec![Grid::new(int_constant(0), q(1, 3)).expect("positive period")]
        );

        let mixed = add([
            negate(floor_term(add([scale(integer(-3), x.clone()), z.clone()]))),
            negate(x.clone()),
        ]);
        let mut kernel = Kernel::new(Limits::default());
        let profile = kernel.profile(&mixed, "x").expect("mixed profile");
        assert_eq!(
            (
                profile.outer_slope,
                profile.segment_slope,
                profile.period,
                profile.delta_y,
                profile.dist_y_minus
            ),
            (
                integer(2),
                integer(-1),
                q(1, 3),
                Rational::one(),
                negate(z.clone())
            )
        );
        assert_eq!(
            kernel.breaks(&mixed, "x").expect("mixed breaks"),
            vec![Grid::new(scale(q(1, 3), z.clone()), q(1, 3)).expect("positive period")]
        );

        let periodic = add([ceil_term(x.clone()), negate(x), negate(c.clone())]);
        let mut kernel = Kernel::new(Limits::default());
        let profile = kernel.profile(&periodic, "x").expect("periodic profile");
        assert_eq!(
            (
                profile.outer_slope,
                profile.segment_slope,
                profile.period,
                profile.delta_y,
                profile.dist_y_minus
            ),
            (
                Rational::zero(),
                integer(-1),
                Rational::one(),
                Rational::one(),
                negate(c)
            )
        );
        assert_eq!(
            kernel.breaks(&periodic, "x").expect("periodic breaks"),
            vec![Grid::new(int_constant(0), Rational::one()).expect("positive period")]
        );
    }

    fn candidate_terms(literal: &Literal) -> BTreeSet<VirtualTerm> {
        Kernel::new(Limits::default())
            .literal_candidates(literal, "x", 0)
            .expect("candidate generation")
            .into_iter()
            .map(|candidate| candidate.virtual_term)
            .collect()
    }

    #[test]
    fn no_break_and_periodic_candidate_vectors_match() {
        let x = variable("x");
        let vectors = [
            (
                Literal::new(x.clone(), Relation::Ge),
                BTreeSet::from([VirtualTerm::default()]),
            ),
            (
                Literal::new(x.clone(), Relation::Gt),
                BTreeSet::from([VirtualTerm {
                    epsilon: true,
                    ..VirtualTerm::default()
                }]),
            ),
            (
                Literal::new(negate(x.clone()), Relation::Ge),
                BTreeSet::from([VirtualTerm {
                    infinity: Some(InfinitySign::Negative),
                    ..VirtualTerm::default()
                }]),
            ),
            (
                Literal::new(x.clone(), Relation::Eq),
                BTreeSet::from([VirtualTerm::default()]),
            ),
            (
                Literal::new(x.clone(), Relation::Ne),
                BTreeSet::from([
                    VirtualTerm {
                        infinity: Some(InfinitySign::Negative),
                        ..VirtualTerm::default()
                    },
                    VirtualTerm {
                        epsilon: true,
                        ..VirtualTerm::default()
                    },
                ]),
            ),
        ];
        for (literal, expected) in vectors {
            assert_eq!(candidate_terms(&literal), expected);
        }

        let periodic = Literal::new(
            add([ceil_term(x.clone()), negate(x), negate(variable("c"))]),
            Relation::Ge,
        );
        let candidates = candidate_terms(&periodic);
        assert!(candidates.contains(&VirtualTerm {
            grid_period: Some(Rational::one()),
            ..VirtualTerm::default()
        }));
        assert!(candidates.contains(&VirtualTerm {
            epsilon: true,
            grid_period: Some(Rational::one()),
            ..VirtualTerm::default()
        }));
    }

    #[test]
    fn periodic_zero_segment_slope_is_total() {
        let x = variable("x");
        let term = add([floor_term(x.clone()), floor_term(negate(x))]);
        let mut kernel = Kernel::new(Limits::default());
        let profile = kernel.profile(&term, "x").expect("periodic profile");
        assert!(profile.outer_slope.is_zero());
        assert!(profile.segment_slope.is_zero());
        assert!(!kernel.breaks(&term, "x").expect("breaks").is_empty());
        for (relation, expected) in [
            (Relation::Eq, true),
            (Relation::Ne, true),
            (Relation::Gt, false),
            (Relation::Ge, true),
        ] {
            let outcome = eliminate_exists(
                "x",
                &[Literal::new(term.clone(), relation)],
                Limits::default(),
            );
            assert_eq!(result_value(&outcome, &[]), expected);
        }
    }

    #[test]
    fn epsilon_and_infinity_substitution_vectors_match() {
        let x = variable("x");
        for (literal, expected) in [
            (Literal::new(x.clone(), Relation::Eq), false),
            (Literal::new(x.clone(), Relation::Ne), true),
            (Literal::new(x.clone(), Relation::Ge), true),
            (Literal::new(x.clone(), Relation::Gt), true),
            (Literal::new(negate(x.clone()), Relation::Ge), false),
            (Literal::new(negate(x.clone()), Relation::Gt), false),
            (Literal::new(floor_term(x.clone()), Relation::Eq), true),
        ] {
            let result = Kernel::new(Limits::default())
                .virtual_substitute(
                    &[literal],
                    "x",
                    &VirtualTerm {
                        epsilon: true,
                        ..VirtualTerm::default()
                    },
                )
                .expect("epsilon substitution");
            assert_eq!(
                result.evaluate(&BTreeMap::new()).expect("closed result"),
                expected
            );
        }

        let c = variable("c");
        let periodic = Literal::new(subtract_remainders(x, c.clone(), integer(2)), Relation::Eq);
        let mut kernel = Kernel::new(Limits::default());
        for sign in [InfinitySign::Negative, InfinitySign::Positive] {
            let result = kernel
                .virtual_substitute(
                    std::slice::from_ref(&periodic),
                    "x",
                    &VirtualTerm {
                        base: c.clone(),
                        infinity: Some(sign),
                        ..VirtualTerm::default()
                    },
                )
                .expect("infinity substitution");
            assert!(result
                .evaluate(&environment(&[("c", q(3, 2))]))
                .expect("closed result"));
        }
    }

    #[test]
    fn grid_flattening_exercises_v1_v2_v3() {
        let x = variable("x");
        let a = variable("a");
        let c = variable("c");
        let z = variable("z");
        let residue_two = Literal::new(
            subtract_remainders(x.clone(), c.clone(), integer(2)),
            Relation::Eq,
        );

        let mut kernel = Kernel::new(Limits::default());
        let v1 = kernel
            .flatten_grid(
                &[Literal::new(x.clone(), Relation::Ge), residue_two.clone()],
                "x",
                &VirtualTerm {
                    base: c.clone(),
                    grid_period: Some(integer(2)),
                    ..VirtualTerm::default()
                },
            )
            .expect("V1 flattening");
        assert_eq!(v1.len(), 1);
        assert_eq!(v1[0].infinity, Some(InfinitySign::Positive));
        assert_eq!(kernel.flatten_records.last().expect("record").case, "V1");

        let mut kernel = Kernel::new(Limits::default());
        let v2 = kernel
            .flatten_grid(
                &[
                    Literal::new(subtract(x.clone(), a), Relation::Eq),
                    residue_two,
                ],
                "x",
                &VirtualTerm {
                    base: c.clone(),
                    grid_period: Some(integer(2)),
                    ..VirtualTerm::default()
                },
            )
            .expect("V2 flattening");
        assert_eq!(v2.len(), 1);
        assert_eq!(kernel.flatten_records.last().expect("record").case, "V2");

        let mixed = add([
            negate(floor_term(add([scale(integer(-3), x.clone()), z]))),
            negate(x.clone()),
        ]);
        let v3_literals = [
            Literal::new(mixed, Relation::Gt),
            Literal::new(negate(x.clone()), Relation::Gt),
            Literal::new(
                subtract_remainders(x.clone(), c.clone(), integer(3)),
                Relation::Eq,
            ),
            Literal::new(
                add([remainder(x, integer(2)), int_constant(-1)]),
                Relation::Ne,
            ),
        ];
        let mut kernel = Kernel::new(Limits::default());
        let v3 = kernel
            .flatten_grid(
                &v3_literals,
                "x",
                &VirtualTerm {
                    base: c,
                    grid_period: Some(integer(3)),
                    ..VirtualTerm::default()
                },
            )
            .expect("V3 flattening");
        assert_eq!(v3.len(), 3);
        let record = kernel.flatten_records.last().expect("record");
        assert_eq!(record.case, "V3");
        assert_eq!(record.common_period, integer(6));
    }

    #[test]
    fn motivating_example_reduces_to_c_at_most_two_thirds() {
        let x = variable("x");
        let a = variable("a");
        let c = variable("c");
        let literals = [
            Literal::new(
                add([x.clone(), negate(floor_term(a.clone())), constant(q(-1, 3))]),
                Relation::Ge,
            ),
            Literal::new(
                add([floor_term(a.clone()), constant(q(2, 3)), negate(x.clone())]),
                Relation::Ge,
            ),
            Literal::new(
                add([ceil_term(x.clone()), negate(x), negate(c.clone())]),
                Relation::Ge,
            ),
        ];
        let outcome = eliminate_exists("x", &literals, Limits::default());
        assert_eq!(outcome.derivation.candidates.len(), 4);
        for a_value in [
            q(-7, 3),
            integer(-2),
            q(-1, 2),
            Rational::zero(),
            q(2, 3),
            Rational::one(),
            q(5, 2),
        ] {
            for c_value in [
                integer(-2),
                q(-1, 3),
                Rational::zero(),
                q(1, 2),
                q(2, 3),
                q(3, 4),
                integer(2),
            ] {
                assert_eq!(
                    result_value(&outcome, &[("a", a_value.clone()), ("c", c_value.clone())]),
                    c_value <= q(2, 3)
                );
            }
        }
    }

    #[test]
    fn pure_linear_open_closed_matrix_matches_interval_semantics() {
        let x = variable("x");
        let a = variable("a");
        let b = variable("b");
        for lower_closed in [false, true] {
            for upper_closed in [false, true] {
                let literals = [
                    Literal::new(
                        subtract(x.clone(), a.clone()),
                        if lower_closed {
                            Relation::Ge
                        } else {
                            Relation::Gt
                        },
                    ),
                    Literal::new(
                        subtract(b.clone(), x.clone()),
                        if upper_closed {
                            Relation::Ge
                        } else {
                            Relation::Gt
                        },
                    ),
                ];
                let outcome = eliminate_exists("x", &literals, Limits::default());
                for a_value in [integer(-1), Rational::zero(), integer(2)] {
                    for b_value in [integer(-1), Rational::zero(), integer(2)] {
                        let expected = a_value < b_value
                            || (a_value == b_value && lower_closed && upper_closed);
                        assert_eq!(
                            result_value(
                                &outcome,
                                &[("a", a_value.clone()), ("b", b_value.clone())]
                            ),
                            expected
                        );
                    }
                }
            }
        }
    }

    #[derive(Clone, Copy)]
    struct Deterministic(u64);

    impl Deterministic {
        fn next(&mut self) -> u64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            self.0
        }

        fn choose(&mut self, length: usize) -> usize {
            usize::try_from(self.next() % u64::try_from(length).expect("small length"))
                .expect("small index")
        }
    }

    fn generated_linear(rng: &mut Deterministic, x: &Term) -> Term {
        let coefficients = [-3_i64, -2, -1, 1, 2, 3];
        let coefficient = coefficients[rng.choose(coefficients.len())];
        let offset = i64::try_from(rng.choose(9)).expect("small offset") - 4;
        add([scale(integer(coefficient), x.clone()), int_constant(offset)])
    }

    fn generated_term(rng: &mut Deterministic, x: &Term) -> Term {
        match rng.choose(4) {
            0 => generated_linear(rng, x),
            1 => add([
                floor_term(generated_linear(rng, x)),
                int_constant(i64::try_from(rng.choose(7)).expect("small offset") - 3),
            ]),
            2 => add([
                floor_term(generated_linear(rng, x)),
                floor_term(generated_linear(rng, x)),
                int_constant(i64::try_from(rng.choose(7)).expect("small offset") - 3),
            ]),
            _ => add([
                generated_linear(rng, x),
                floor_term(generated_linear(rng, x)),
            ]),
        }
    }

    fn brute_bounded_decision(literals: &[Literal]) -> bool {
        // Generated affine roots and floor breaks lie on the 1/6 lattice.
        // Testing that lattice and every open cell midpoint is therefore a
        // complete, kernel-independent cell decomposition of [-8, 8].
        (-96_i64..=96).any(|numerator| {
            let bindings = environment(&[("x", q(numerator, 12))]);
            literals.iter().all(|literal| {
                literal
                    .relation
                    .evaluate(&evaluate_term(&literal.term, &bindings).expect("closed literal"))
            })
        })
    }

    #[test]
    fn thousand_seeded_cases_agree_with_exact_cell_oracle() {
        let x = variable("x");
        let relations = [Relation::Eq, Relation::Ne, Relation::Gt, Relation::Ge];
        let bounds = [
            Literal::new(add([x.clone(), int_constant(8)]), Relation::Ge),
            Literal::new(add([int_constant(8), negate(x.clone())]), Relation::Ge),
        ];
        let mut rng = Deterministic(0xB451E);
        let mut decisions = BTreeMap::from([(false, 0_usize), (true, 0_usize)]);
        for case_index in 0..1_000 {
            let mut literals = bounds.clone().to_vec();
            for _ in 0..=rng.choose(4) {
                literals.push(Literal::new(
                    generated_term(&mut rng, &x),
                    relations[rng.choose(relations.len())],
                ));
            }
            let expected = brute_bounded_decision(&literals);
            let outcome = eliminate_exists("x", &literals, Limits::default());
            assert_eq!(
                outcome.status,
                QeStatus::Success,
                "generated case {case_index}: {}",
                outcome.reason
            );
            assert_eq!(
                result_value(&outcome, &[]),
                expected,
                "generated case {case_index}: {}",
                literals
                    .iter()
                    .map(Literal::render)
                    .collect::<Vec<_>>()
                    .into_iter()
                    .chain(std::iter::once(format!("outcome={outcome:#?}")))
                    .collect::<Vec<_>>()
                    .join(" & ")
            );
            assert!(!outcome
                .formula
                .as_ref()
                .expect("success")
                .variables()
                .contains("x"));
            *decisions.get_mut(&expected).expect("decision key") += 1;
        }
        assert!(decisions[&true] > 0);
        assert!(decisions[&false] > 0);
    }

    #[test]
    fn deliberate_mutations_are_detected() {
        let x = variable("x");
        let residue = Literal::new(
            subtract_remainders(x.clone(), int_constant(1), integer(2)),
            Relation::Eq,
        );
        let v1 = [Literal::new(x.clone(), Relation::Ge), residue];
        let baseline = Kernel::new(Limits::default()).eliminate_exists("x", &v1);
        let reversed = Kernel::with_mutations(
            Limits::default(),
            Mutations {
                reverse_infinity_periodicity: true,
                ..Mutations::default()
            },
        )
        .eliminate_exists("x", &v1);
        assert!(result_value(&baseline, &[]));
        assert!(!result_value(&reversed, &[]));

        let strict = [Literal::new(x.clone(), Relation::Gt)];
        let baseline = Kernel::new(Limits::default()).eliminate_exists("x", &strict);
        let weakened = Kernel::with_mutations(
            Limits::default(),
            Mutations {
                drop_epsilon_strictness: true,
                ..Mutations::default()
            },
        )
        .eliminate_exists("x", &strict);
        assert!(result_value(&baseline, &[]));
        assert!(!result_value(&weakened, &[]));

        let singleton = [
            Literal::new(x.clone(), Relation::Eq),
            Literal::new(x, Relation::Ge),
        ];
        let baseline = Kernel::new(Limits::default()).eliminate_exists("x", &singleton);
        let omitted = Kernel::with_mutations(
            Limits::default(),
            Mutations {
                omit_last_candidate: true,
                ..Mutations::default()
            },
        )
        .eliminate_exists("x", &singleton);
        assert!(result_value(&baseline, &[]));
        assert!(!result_value(&omitted, &[]));
    }

    #[test]
    fn limits_and_unsupported_inputs_fail_closed() {
        let x = variable("x");
        let simple = [Literal::new(x.clone(), Relation::Ge)];
        let floor_literal = [Literal::new(floor_term(x.clone()), Relation::Eq)];
        let mut limits = Limits {
            max_steps: 0,
            ..Limits::default()
        };
        let mut outcomes = vec![eliminate_exists("x", &simple, limits)];
        limits = Limits::default();
        limits.max_candidates = 0;
        outcomes.push(eliminate_exists("x", &simple, limits));
        limits = Limits::default();
        limits.max_grids = 0;
        outcomes.push(eliminate_exists("x", &floor_literal, limits));
        limits = Limits::default();
        limits.max_grid_points = 0;
        outcomes.push(eliminate_exists("x", &floor_literal, limits));
        limits = Limits::default();
        limits.max_formula_nodes = 0;
        outcomes.push(eliminate_exists("x", &simple, limits));
        limits = Limits::default();
        limits.max_rational_bits = 8;
        outcomes.push(eliminate_exists(
            "x",
            &[Literal::new(
                scale(Rational::from_integer(BigInt::one() << 100_usize), x),
                Relation::Ge,
            )],
            limits,
        ));
        for outcome in outcomes {
            assert_eq!(outcome.status, QeStatus::Unknown);
            assert_eq!(outcome.unknown_kind, Some(UnknownKind::ResourceLimit));
            assert!(outcome.formula.is_none());
        }

        let unsupported = eliminate_exists("x", &[], Limits::default());
        assert_eq!(unsupported.status, QeStatus::Unknown);
        assert_eq!(
            unsupported.unknown_kind,
            Some(UnknownKind::UnsupportedFragment)
        );
        assert!(unsupported.formula.is_none());
    }

    fn wrapped_result_value(outcome: &FormulaQeOutcome, bindings: &[(&str, Rational)]) -> bool {
        assert_eq!(outcome.status, QeStatus::Success, "{}", outcome.reason);
        outcome
            .formula
            .as_ref()
            .expect("successful wrapped outcome has a formula")
            .evaluate(&environment(bindings))
            .expect("wrapped test result is quantifier-free")
    }

    #[test]
    fn one_conjunction_derivation_replay_rejects_corruption() {
        let x = variable("x");
        let c = variable("c");
        let literals = [
            Literal::new(x.clone(), Relation::Ge),
            Literal::new(subtract_remainders(x, c, integer(2)), Relation::Eq),
        ];
        let outcome = eliminate_exists("x", &literals, Limits::default());
        validate_derivation("x", &literals, &outcome, Limits::default())
            .expect("authentic derivation replays");

        let mut missing_candidate = outcome.clone();
        missing_candidate.derivation.candidates.pop();
        assert!(
            validate_derivation("x", &literals, &missing_candidate, Limits::default()).is_err()
        );

        let mut corrupt_formula = outcome;
        corrupt_formula.formula = Some(boolean(false));
        assert!(validate_derivation("x", &literals, &corrupt_formula, Limits::default()).is_err());
    }

    #[test]
    fn boolean_wrapper_distributes_dnf_and_preserves_free_parameters() {
        let x = variable("x");
        let a = variable("a");
        let first = atom(Literal::new(subtract(x.clone(), a.clone()), Relation::Eq));
        let second = atom(Literal::new(
            subtract(x.clone(), add([a.clone(), int_constant(1)])),
            Relation::Eq,
        ));
        let negative = atom(Literal::new(negate(x), Relation::Gt));
        let formula = Formula::Exists(
            "x".to_owned(),
            Box::new(conjunction([disjunction([first, second]), negative])),
        );
        let outcome = eliminate_formula(formula, Limits::default());
        assert_eq!(outcome.derivation.eliminations.len(), 2);
        for value in [
            integer(-2),
            integer(-1),
            q(-1, 2),
            Rational::zero(),
            Rational::one(),
        ] {
            assert_eq!(
                wrapped_result_value(&outcome, &[("a", value.clone())]),
                value.is_negative()
            );
        }
    }

    #[test]
    fn universal_and_nested_quantifiers_use_exact_duality() {
        let x = variable("x");
        let y = variable("y");
        let a = variable("a");

        let excluded_point = Formula::Forall(
            "x".to_owned(),
            Box::new(disjunction([
                atom(Literal::new(subtract(x.clone(), a.clone()), Relation::Ne)),
                atom(Literal::new(x.clone(), Relation::Ge)),
            ])),
        );
        let outcome = eliminate_formula(excluded_point, Limits::default());
        for value in [integer(-2), Rational::zero(), integer(3)] {
            assert_eq!(
                wrapped_result_value(&outcome, &[("a", value.clone())]),
                !value.is_negative()
            );
        }

        let forall_exists = Formula::Forall(
            "x".to_owned(),
            Box::new(Formula::Exists(
                "y".to_owned(),
                Box::new(atom(Literal::new(
                    subtract(y.clone(), x.clone()),
                    Relation::Eq,
                ))),
            )),
        );
        assert!(wrapped_result_value(
            &eliminate_formula(forall_exists, Limits::default()),
            &[]
        ));

        let exists_forall = Formula::Exists(
            "x".to_owned(),
            Box::new(Formula::Forall(
                "y".to_owned(),
                Box::new(atom(Literal::new(subtract(x, y), Relation::Eq))),
            )),
        );
        assert!(!wrapped_result_value(
            &eliminate_formula(exists_forall, Limits::default()),
            &[]
        ));
    }

    #[test]
    fn generated_universal_wrappers_match_point_exclusion_semantics() {
        let x = variable("x");
        let a = variable("a");
        let c = variable("c");
        let mut checked = 0_usize;
        for a_value in -10_i64..=10 {
            for c_value in -10_i64..=10 {
                let formula = Formula::Forall(
                    "x".to_owned(),
                    Box::new(disjunction([
                        atom(Literal::new(subtract(x.clone(), a.clone()), Relation::Ne)),
                        atom(Literal::new(add([x.clone(), c.clone()]), Relation::Ge)),
                    ])),
                );
                let outcome = eliminate_formula(formula, Limits::default());
                assert_eq!(
                    wrapped_result_value(
                        &outcome,
                        &[("a", integer(a_value)), ("c", integer(c_value))]
                    ),
                    a_value + c_value >= 0
                );
                checked = checked.saturating_add(1);
            }
        }
        assert_eq!(checked, 441);
    }

    #[test]
    fn wrapper_resource_limits_fail_closed() {
        let x = variable("x");
        let body = disjunction([
            atom(Literal::new(x.clone(), Relation::Eq)),
            atom(Literal::new(
                subtract(x.clone(), int_constant(1)),
                Relation::Eq,
            )),
        ]);
        let formula = Formula::Exists("x".to_owned(), Box::new(body));
        let mut limits = Limits {
            max_dnf_branches: 0,
            ..Limits::default()
        };
        let outcome = eliminate_formula(formula.clone(), limits);
        assert_eq!(outcome.status, QeStatus::Unknown);
        assert_eq!(outcome.unknown_kind, Some(UnknownKind::ResourceLimit));
        assert!(outcome.formula.is_none());

        limits = Limits::default();
        limits.max_formula_nodes = 0;
        let outcome = eliminate_formula(formula, limits);
        assert_eq!(outcome.status, QeStatus::Unknown);
        assert_eq!(outcome.unknown_kind, Some(UnknownKind::ResourceLimit));
        assert!(outcome.formula.is_none());

        limits = Limits::default();
        limits.max_rational_bits = 8;
        let oversized_ground = Formula::Atom(Literal::new(
            constant(Rational::from_integer(BigInt::one() << 100_usize)),
            Relation::Eq,
        ));
        let outcome = eliminate_formula(oversized_ground, limits);
        assert_eq!(outcome.status, QeStatus::Unknown);
        assert_eq!(outcome.unknown_kind, Some(UnknownKind::ResourceLimit));
        assert!(outcome.formula.is_none());
        assert!(!outcome.derivation.replay_validated);
    }

    #[test]
    fn formula_derivation_validator_rejects_source_result_flag_and_branch_corruption() {
        let source = Formula::Exists(
            "x".to_owned(),
            Box::new(atom(Literal::new(
                subtract(variable("x"), int_constant(1)),
                Relation::Eq,
            ))),
        );
        let limits = Limits::default();
        let authentic = eliminate_formula(source.clone(), limits);
        validate_formula_derivation(&source, &authentic, limits)
            .expect("authentic formula derivation validates");

        let different_source = Formula::Exists(
            "x".to_owned(),
            Box::new(atom(Literal::new(
                subtract(variable("x"), int_constant(2)),
                Relation::Gt,
            ))),
        );
        assert!(validate_formula_derivation(&different_source, &authentic, limits).is_err());

        let mut corrupt_result = authentic.clone();
        corrupt_result.formula = Some(boolean(false));
        assert!(validate_formula_derivation(&source, &corrupt_result, limits).is_err());

        let mut corrupt_flag = authentic.clone();
        corrupt_flag.derivation.replay_validated = false;
        assert!(validate_formula_derivation(&source, &corrupt_flag, limits).is_err());

        let mut corrupt_branch = authentic;
        corrupt_branch
            .derivation
            .eliminations
            .first_mut()
            .expect("test formula has one elimination")
            .candidates
            .clear();
        assert!(validate_formula_derivation(&source, &corrupt_branch, limits).is_err());
    }
}
