//! Dependency-free exact replay checker for ground difference-logic evidence.

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::env;
use std::fmt;
use std::fs;
use std::io;
use std::path::Path;

#[derive(Clone, Copy, Debug, Eq)]
struct Rational {
    numerator: i128,
    denominator: i128,
}

impl Rational {
    fn new(numerator: i128, denominator: i128) -> Result<Self, String> {
        if denominator == 0 {
            return Err("zero rational denominator".to_owned());
        }
        let (mut numerator, mut denominator) = (numerator, denominator);
        if denominator < 0 {
            numerator = numerator
                .checked_neg()
                .ok_or_else(|| "rational sign overflow".to_owned())?;
            denominator = denominator
                .checked_neg()
                .ok_or_else(|| "rational sign overflow".to_owned())?;
        }
        let divisor = gcd(numerator.unsigned_abs(), denominator as u128) as i128;
        Ok(Self {
            numerator: numerator / divisor,
            denominator: denominator / divisor,
        })
    }

    fn zero() -> Self {
        Self {
            numerator: 0,
            denominator: 1,
        }
    }

    fn checked_add(self, other: Self) -> Result<Self, String> {
        let left = self
            .numerator
            .checked_mul(other.denominator)
            .ok_or_else(|| "rational addition overflow".to_owned())?;
        let right = other
            .numerator
            .checked_mul(self.denominator)
            .ok_or_else(|| "rational addition overflow".to_owned())?;
        let numerator = left
            .checked_add(right)
            .ok_or_else(|| "rational addition overflow".to_owned())?;
        let denominator = self
            .denominator
            .checked_mul(other.denominator)
            .ok_or_else(|| "rational denominator overflow".to_owned())?;
        Self::new(numerator, denominator)
    }

    fn checked_sub(self, other: Self) -> Result<Self, String> {
        let negative = Self::new(
            other
                .numerator
                .checked_neg()
                .ok_or_else(|| "rational negation overflow".to_owned())?,
            other.denominator,
        )?;
        self.checked_add(negative)
    }

    fn checked_cmp(self, other: Self) -> Result<Ordering, String> {
        let left = self
            .numerator
            .checked_mul(other.denominator)
            .ok_or_else(|| "rational comparison overflow".to_owned())?;
        let right = other
            .numerator
            .checked_mul(self.denominator)
            .ok_or_else(|| "rational comparison overflow".to_owned())?;
        Ok(left.cmp(&right))
    }
}

impl PartialEq for Rational {
    fn eq(&self, other: &Self) -> bool {
        self.numerator == other.numerator && self.denominator == other.denominator
    }
}

fn gcd(mut left: u128, mut right: u128) -> u128 {
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    left.max(1)
}

#[derive(Clone, Debug)]
struct Constraint {
    label: String,
    lhs: String,
    rhs: String,
    bound: Rational,
}

#[derive(Clone, Debug)]
struct Decision {
    backend: String,
    workload: String,
    branch: String,
    sort: String,
    status: String,
    constraints: Vec<Constraint>,
    core: Vec<String>,
    model: HashMap<String, Rational>,
}

#[derive(Debug)]
struct ReplayError {
    decision: String,
    reason: String,
}

impl fmt::Display for ReplayError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.decision, self.reason)
    }
}

fn parse_certificate(path: &Path) -> Result<Vec<Decision>, String> {
    let text = fs::read_to_string(path).map_err(|error| error.to_string())?;
    let mut lines = text.lines();
    if lines.next() != Some("UMLAUT_GROUND_THEORY_CERT_V1") {
        return Err("invalid certificate header".to_owned());
    }
    let mut decisions = Vec::new();
    let mut current: Option<Decision> = None;
    let mut ended = false;
    for (offset, line) in lines.enumerate() {
        let line_number = offset + 2;
        let fields: Vec<&str> = line.split('\t').collect();
        match fields.as_slice() {
            ["DECISION", backend, workload, branch, sort, status]
                if current.is_none() && !ended =>
            {
                if *sort != "Int" && *sort != "Real" {
                    return Err(format!("line {line_number}: invalid sort"));
                }
                if *status != "sat" && *status != "unsat" {
                    return Err(format!("line {line_number}: invalid status"));
                }
                current = Some(Decision {
                    backend: (*backend).to_owned(),
                    workload: (*workload).to_owned(),
                    branch: (*branch).to_owned(),
                    sort: (*sort).to_owned(),
                    status: (*status).to_owned(),
                    constraints: Vec::new(),
                    core: Vec::new(),
                    model: HashMap::new(),
                });
            }
            ["CONSTRAINT", label, lhs, rhs, numerator, denominator] => {
                let bound = Rational::new(
                    parse_i128(numerator, line_number)?,
                    parse_i128(denominator, line_number)?,
                )?;
                current_mut(&mut current, line_number)?
                    .constraints
                    .push(Constraint {
                        label: (*label).to_owned(),
                        lhs: (*lhs).to_owned(),
                        rhs: (*rhs).to_owned(),
                        bound,
                    });
            }
            ["CORE", labels] => {
                let decision = current_mut(&mut current, line_number)?;
                if !decision.core.is_empty() {
                    return Err(format!("line {line_number}: duplicate core record"));
                }
                decision.core = labels
                    .split(',')
                    .filter(|label| !label.is_empty())
                    .map(str::to_owned)
                    .collect();
            }
            ["MODEL", variable, numerator, denominator] => {
                let value = Rational::new(
                    parse_i128(numerator, line_number)?,
                    parse_i128(denominator, line_number)?,
                )?;
                if current_mut(&mut current, line_number)?
                    .model
                    .insert((*variable).to_owned(), value)
                    .is_some()
                {
                    return Err(format!("line {line_number}: duplicate model variable"));
                }
            }
            ["END_DECISION"] => {
                let decision = current
                    .take()
                    .ok_or_else(|| format!("line {line_number}: no active decision"))?;
                decisions.push(decision);
            }
            ["END"] if current.is_none() => ended = true,
            _ => return Err(format!("line {line_number}: malformed certificate record")),
        }
    }
    if !ended || current.is_some() {
        return Err("incomplete certificate".to_owned());
    }
    Ok(decisions)
}

fn parse_i128(text: &str, line_number: usize) -> Result<i128, String> {
    text.parse::<i128>()
        .map_err(|_| format!("line {line_number}: invalid integer"))
}

fn current_mut(
    current: &mut Option<Decision>,
    line_number: usize,
) -> Result<&mut Decision, String> {
    current
        .as_mut()
        .ok_or_else(|| format!("line {line_number}: record outside decision"))
}

fn replay(decision: &Decision) -> Result<(), ReplayError> {
    let identity = format!(
        "{}/{}/{}",
        decision.backend, decision.workload, decision.branch
    );
    if decision.sort != "Int" && decision.sort != "Real" {
        return Err(ReplayError {
            decision: identity,
            reason: "unsupported sort".to_owned(),
        });
    }
    let labels: HashSet<&str> = decision
        .constraints
        .iter()
        .map(|constraint| constraint.label.as_str())
        .collect();
    if labels.len() != decision.constraints.len() {
        return Err(ReplayError {
            decision: identity,
            reason: "duplicate constraint label".to_owned(),
        });
    }
    if decision.sort == "Int"
        && decision
            .constraints
            .iter()
            .any(|constraint| constraint.bound.denominator != 1)
    {
        return Err(ReplayError {
            decision: identity,
            reason: "fractional integer bound".to_owned(),
        });
    }
    let valid = if decision.status == "unsat" {
        verify_negative_cycle(decision)?
    } else {
        verify_model(decision)?
    };
    if valid {
        Ok(())
    } else {
        Err(ReplayError {
            decision: identity,
            reason: format!("{} evidence did not replay", decision.status),
        })
    }
}

fn verify_negative_cycle(decision: &Decision) -> Result<bool, ReplayError> {
    let identity = format!(
        "{}/{}/{}",
        decision.backend, decision.workload, decision.branch
    );
    if decision.core.is_empty() || !decision.model.is_empty() {
        return Ok(false);
    }
    let by_label: HashMap<&str, &Constraint> = decision
        .constraints
        .iter()
        .map(|constraint| (constraint.label.as_str(), constraint))
        .collect();
    let mut seen = HashSet::new();
    let mut selected = Vec::new();
    for label in &decision.core {
        if !seen.insert(label) {
            return Ok(false);
        }
        let constraint = by_label.get(label.as_str()).ok_or_else(|| ReplayError {
            decision: identity.clone(),
            reason: format!("unknown core label {label}"),
        })?;
        selected.push(*constraint);
    }
    let mut vertices = HashSet::from(["zero".to_owned()]);
    for constraint in &selected {
        vertices.insert(constraint.lhs.clone());
        vertices.insert(constraint.rhs.clone());
    }
    let mut distances: HashMap<String, Rational> = vertices
        .iter()
        .map(|vertex| (vertex.clone(), Rational::zero()))
        .collect();
    for iteration in 0..vertices.len() {
        let mut changed = false;
        for constraint in &selected {
            let source = distances[&constraint.rhs];
            let target = distances[&constraint.lhs];
            let candidate = source
                .checked_add(constraint.bound)
                .map_err(|reason| ReplayError {
                    decision: identity.clone(),
                    reason,
                })?;
            if candidate
                .checked_cmp(target)
                .map_err(|reason| ReplayError {
                    decision: identity.clone(),
                    reason,
                })?
                == Ordering::Less
            {
                distances.insert(constraint.lhs.clone(), candidate);
                changed = true;
                if iteration == vertices.len() - 1 {
                    return Ok(true);
                }
            }
        }
        if !changed {
            return Ok(false);
        }
    }
    Ok(false)
}

fn verify_model(decision: &Decision) -> Result<bool, ReplayError> {
    if !decision.core.is_empty() || decision.model.is_empty() {
        return Ok(false);
    }
    let identity = format!(
        "{}/{}/{}",
        decision.backend, decision.workload, decision.branch
    );
    let zero = Rational::zero();
    for constraint in &decision.constraints {
        let lhs =
            endpoint_value(&decision.model, &constraint.lhs, zero).ok_or_else(|| ReplayError {
                decision: identity.clone(),
                reason: format!("model is missing {}", constraint.lhs),
            })?;
        let rhs =
            endpoint_value(&decision.model, &constraint.rhs, zero).ok_or_else(|| ReplayError {
                decision: identity.clone(),
                reason: format!("model is missing {}", constraint.rhs),
            })?;
        let difference = lhs.checked_sub(rhs).map_err(|reason| ReplayError {
            decision: identity.clone(),
            reason,
        })?;
        if difference
            .checked_cmp(constraint.bound)
            .map_err(|reason| ReplayError {
                decision: identity.clone(),
                reason,
            })?
            == Ordering::Greater
        {
            return Ok(false);
        }
    }
    let used_variables: HashSet<&str> = decision
        .constraints
        .iter()
        .flat_map(|constraint| [constraint.lhs.as_str(), constraint.rhs.as_str()])
        .filter(|variable| *variable != "zero")
        .collect();
    Ok(used_variables
        .iter()
        .all(|variable| decision.model.contains_key(*variable)))
}

fn endpoint_value(
    model: &HashMap<String, Rational>,
    variable: &str,
    zero: Rational,
) -> Option<Rational> {
    if variable == "zero" {
        Some(zero)
    } else {
        model.get(variable).copied()
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let arguments: Vec<String> = env::args().collect();
    if arguments.len() != 2 {
        eprintln!("usage: {} CERTIFICATE", arguments[0]);
        return Err(io::Error::other("invalid arguments").into());
    }
    let decisions = parse_certificate(Path::new(&arguments[1])).map_err(io::Error::other)?;
    let mut invalid = Vec::new();
    for decision in &decisions {
        if let Err(error) = replay(decision) {
            invalid.push(error);
        }
    }
    println!(
        "SUMMARY\ttotal\t{}\tverified\t{}\tinvalid\t{}",
        decisions.len(),
        decisions.len() - invalid.len(),
        invalid.len()
    );
    for error in &invalid {
        eprintln!("INVALID\t{error}");
    }
    if invalid.is_empty() {
        Ok(())
    } else {
        Err(io::Error::other("certificate replay failed").into())
    }
}
