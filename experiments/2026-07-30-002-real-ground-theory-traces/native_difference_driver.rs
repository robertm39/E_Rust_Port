//! Dependency-free exact ground difference-logic decision prototype.

#![deny(unsafe_code)]

use std::cmp::Ordering;
use std::collections::{BTreeSet, HashMap};
use std::env;
use std::fmt;
use std::fs;
use std::io;
use std::path::Path;
use std::time::Instant;

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
        let divisor = gcd(numerator.unsigned_abs(), denominator.cast_unsigned()).cast_signed();
        Ok(Self {
            numerator: numerator / divisor,
            denominator: denominator / divisor,
        })
    }

    const fn zero() -> Self {
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
        let numerator = other
            .numerator
            .checked_neg()
            .ok_or_else(|| "rational negation overflow".to_owned())?;
        self.checked_add(Self::new(numerator, other.denominator)?)
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

impl fmt::Display for Rational {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.denominator == 1 {
            write!(formatter, "{}", self.numerator)
        } else {
            write!(formatter, "{}/{}", self.numerator, self.denominator)
        }
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
struct Query {
    id: String,
    sort: String,
    constraints: Vec<Constraint>,
}

#[derive(Clone, Debug)]
enum Decision {
    Sat(HashMap<String, Rational>),
    Unsat(Vec<String>),
}

fn parse_i128(text: &str, line_number: usize) -> Result<i128, String> {
    text.parse::<i128>()
        .map_err(|_| format!("line {line_number}: invalid integer"))
}

fn current_query(current: &mut Option<Query>, line_number: usize) -> Result<&mut Query, String> {
    current
        .as_mut()
        .ok_or_else(|| format!("line {line_number}: record outside query"))
}

fn parse_protocol(path: &Path) -> Result<Vec<Query>, String> {
    let text = fs::read_to_string(path).map_err(|error| error.to_string())?;
    let mut lines = text.lines();
    if lines.next() != Some("UMLAUT_REAL_GROUND_NATIVE_V1") {
        return Err("invalid protocol header".to_owned());
    }
    let mut queries = Vec::new();
    let mut current = None;
    let mut ended = false;
    for (offset, line) in lines.enumerate() {
        let line_number = offset + 2;
        let fields: Vec<&str> = line.split('\t').collect();
        match fields.as_slice() {
            ["QUERY", id, sort] if current.is_none() && !ended => {
                if *sort != "Int" && *sort != "Real" {
                    return Err(format!("line {line_number}: invalid sort"));
                }
                current = Some(Query {
                    id: (*id).to_owned(),
                    sort: (*sort).to_owned(),
                    constraints: Vec::new(),
                });
            }
            ["CONSTRAINT", label, lhs, rhs, numerator, denominator] => {
                let bound = Rational::new(
                    parse_i128(numerator, line_number)?,
                    parse_i128(denominator, line_number)?,
                )?;
                if current_query(&mut current, line_number)?
                    .constraints
                    .iter()
                    .any(|constraint| constraint.label == *label)
                {
                    return Err(format!("line {line_number}: duplicate label"));
                }
                current_query(&mut current, line_number)?
                    .constraints
                    .push(Constraint {
                        label: (*label).to_owned(),
                        lhs: (*lhs).to_owned(),
                        rhs: (*rhs).to_owned(),
                        bound,
                    });
            }
            ["END_QUERY"] => {
                let query = current
                    .take()
                    .ok_or_else(|| format!("line {line_number}: no active query"))?;
                if query.constraints.is_empty() {
                    return Err(format!("line {line_number}: empty query"));
                }
                queries.push(query);
            }
            ["END"] if current.is_none() => ended = true,
            _ => return Err(format!("line {line_number}: malformed protocol record")),
        }
    }
    if !ended || current.is_some() {
        return Err("incomplete protocol".to_owned());
    }
    Ok(queries)
}

fn decide(query: &Query) -> Result<Decision, String> {
    let mut vertices = BTreeSet::from(["zero".to_owned()]);
    for constraint in &query.constraints {
        if query.sort == "Int" && constraint.bound.denominator != 1 {
            return Err("fractional integer bound".to_owned());
        }
        vertices.insert(constraint.lhs.clone());
        vertices.insert(constraint.rhs.clone());
    }
    let mut distances: HashMap<String, Rational> = vertices
        .iter()
        .map(|vertex| (vertex.clone(), Rational::zero()))
        .collect();
    for iteration in 0..vertices.len() {
        let mut changed = false;
        for constraint in &query.constraints {
            let candidate = distances[&constraint.rhs].checked_add(constraint.bound)?;
            if candidate.checked_cmp(distances[&constraint.lhs])? == Ordering::Less {
                distances.insert(constraint.lhs.clone(), candidate);
                changed = true;
                if iteration == vertices.len() - 1 {
                    return Ok(Decision::Unsat(
                        query
                            .constraints
                            .iter()
                            .map(|item| item.label.clone())
                            .collect(),
                    ));
                }
            }
        }
        if !changed {
            let zero = distances["zero"];
            let mut model = HashMap::new();
            for vertex in &vertices {
                if vertex != "zero" {
                    model.insert(vertex.clone(), distances[vertex].checked_sub(zero)?);
                }
            }
            return Ok(Decision::Sat(model));
        }
    }
    Err("decision loop ended without a verdict".to_owned())
}

fn write_results(path: &Path, queries: &[Query]) -> Result<(), String> {
    let mut lines = vec![
        "META\tprotocol\tumlaut-real-ground-native-v1".to_owned(),
        format!("META\tquery_count\t{}", queries.len()),
    ];
    for query in queries {
        let started = Instant::now();
        let decision = decide(query)?;
        let elapsed_ns = started.elapsed().as_nanos();
        match decision {
            Decision::Unsat(core) => lines.push(format!(
                "RESULT\t{}\tunsat\t{}\t{}\t\texact_negative_cycle",
                query.id,
                elapsed_ns,
                core.join(",")
            )),
            Decision::Sat(model) => {
                let mut fields: Vec<_> = model.into_iter().collect();
                fields.sort_by(|left, right| left.0.cmp(&right.0));
                let rendered = fields
                    .into_iter()
                    .map(|(variable, value)| format!("{variable}={value}"))
                    .collect::<Vec<_>>()
                    .join(";");
                lines.push(format!(
                    "RESULT\t{}\tsat\t{}\t\t{}\texact_potential",
                    query.id, elapsed_ns, rendered
                ));
            }
        }
    }
    fs::write(path, lines.join("\n") + "\n").map_err(|error| error.to_string())
}

fn run(arguments: &[String]) -> Result<(), String> {
    if arguments.len() != 4 || arguments[1] != "run" {
        return Err(format!(
            "usage: {} run PROTOCOL RESULTS",
            arguments
                .first()
                .map_or("native-difference-driver", String::as_str)
        ));
    }
    let queries = parse_protocol(Path::new(&arguments[2]))?;
    write_results(Path::new(&arguments[3]), &queries)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let arguments: Vec<String> = env::args().collect();
    run(&arguments).map_err(|error| io::Error::other(error).into())
}

#[cfg(test)]
mod tests {
    use super::{decide, Constraint, Decision, Query, Rational};

    fn constraint(label: &str, lhs: &str, rhs: &str, bound: i128) -> Constraint {
        Constraint {
            label: label.to_owned(),
            lhs: lhs.to_owned(),
            rhs: rhs.to_owned(),
            bound: Rational::new(bound, 1).expect("integer rational"),
        }
    }

    #[test]
    fn negative_cycle_returns_replayable_superset_core() {
        let query = Query {
            id: "q".to_owned(),
            sort: "Int".to_owned(),
            constraints: vec![
                constraint("a", "x", "zero", 0),
                constraint("b", "zero", "x", -1),
            ],
        };
        let Decision::Unsat(core) = decide(&query).expect("decision") else {
            panic!("negative cycle was not rejected");
        };
        assert_eq!(core, ["a", "b"]);
    }

    #[test]
    fn feasible_graph_returns_anchored_model() {
        let query = Query {
            id: "q".to_owned(),
            sort: "Real".to_owned(),
            constraints: vec![
                constraint("a", "x", "zero", 3),
                constraint("b", "zero", "x", -1),
            ],
        };
        let Decision::Sat(model) = decide(&query).expect("decision") else {
            panic!("feasible graph was rejected");
        };
        let x = model.get("x").expect("x model value");
        assert!(
            x.checked_cmp(Rational::new(1, 1).expect("one"))
                .expect("comparison")
                != std::cmp::Ordering::Less
        );
        assert!(
            x.checked_cmp(Rational::new(3, 1).expect("three"))
                .expect("comparison")
                != std::cmp::Ordering::Greater
        );
    }
}
