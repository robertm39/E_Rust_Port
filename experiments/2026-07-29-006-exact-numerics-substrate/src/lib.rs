//! Shared protocol for the experiment-only exact-rational backends.

use std::collections::BTreeMap;
use std::fs;
use std::hint::black_box;
use std::path::Path;
use std::time::Instant;

const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// The deliberately small surface exercised identically by every backend.
pub trait RationalBackend {
    type Rational: Clone + Ord;

    /// Parse one exact rational from decimal numerator and denominator parts.
    ///
    /// # Errors
    ///
    /// Returns an error when either part is invalid or the denominator is zero.
    fn parse(numerator: &str, denominator: &str) -> Result<Self::Rational, String>;
    fn add(left: &Self::Rational, right: &Self::Rational) -> Self::Rational;
    fn subtract(left: &Self::Rational, right: &Self::Rational) -> Self::Rational;
    fn multiply(left: &Self::Rational, right: &Self::Rational) -> Self::Rational;
    fn divide(left: &Self::Rational, right: &Self::Rational) -> Self::Rational;
    fn floor(value: &Self::Rational) -> Self::Rational;
    fn ceiling(value: &Self::Rational) -> Self::Rational;
    fn canonical_parts(value: &Self::Rational) -> (String, String);
}

#[derive(Clone)]
struct Case<R> {
    left: R,
    right: R,
}

struct Workload<R> {
    name: String,
    cases: Vec<Case<R>>,
}

struct ResultRow {
    name: String,
    cases: usize,
    iterations: u32,
    operations_per_case: u32,
    elapsed_ns: u128,
    digest: u64,
    sink: i64,
}

fn fnv_update(mut state: u64, text: &str) -> u64 {
    for byte in text.bytes() {
        state ^= u64::from(byte);
        state = state.wrapping_mul(FNV_PRIME);
    }
    state
}

fn hash_rational<B: RationalBackend>(state: u64, value: &B::Rational) -> u64 {
    let (numerator, denominator) = B::canonical_parts(value);
    let state = fnv_update(state, &numerator);
    let state = fnv_update(state, "/");
    let state = fnv_update(state, &denominator);
    fnv_update(state, "\n")
}

fn iterations_for(workload: &str) -> Result<u32, String> {
    match workload {
        "paper" => Ok(500),
        "small" => Ok(80),
        "medium" => Ok(12),
        "large" => Ok(2),
        other => Err(format!("unknown workload {other:?}")),
    }
}

fn parse_workloads<B: RationalBackend>(path: &Path) -> Result<Vec<Workload<B::Rational>>, String> {
    let contents = fs::read_to_string(path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    let mut groups: BTreeMap<String, Vec<Case<B::Rational>>> = BTreeMap::new();
    for (index, raw_line) in contents.lines().enumerate() {
        if raw_line.is_empty() || raw_line.starts_with('#') {
            continue;
        }
        let fields: Vec<&str> = raw_line.split('|').collect();
        if fields.len() != 5 {
            return Err(format!("line {} has {} fields", index + 1, fields.len()));
        }
        let left = B::parse(fields[1], fields[2])
            .map_err(|error| format!("line {} left operand: {error}", index + 1))?;
        let right = B::parse(fields[3], fields[4])
            .map_err(|error| format!("line {} right operand: {error}", index + 1))?;
        groups
            .entry(fields[0].to_owned())
            .or_default()
            .push(Case { left, right });
    }
    if groups.is_empty() {
        return Err("vector file contains no cases".to_owned());
    }
    Ok(groups
        .into_iter()
        .map(|(name, cases)| Workload { name, cases })
        .collect())
}

fn correctness_digest<B: RationalBackend>(cases: &[Case<B::Rational>]) -> u64 {
    let mut digest = FNV_OFFSET;
    for case in cases {
        let add = B::add(&case.left, &case.right);
        let subtract = B::subtract(&case.left, &case.right);
        let multiply = B::multiply(&case.left, &case.right);
        let divide = B::divide(&case.left, &case.right);
        let floor = B::floor(&case.left);
        let ceiling = B::ceiling(&case.left);
        let ordering = match case.left.cmp(&case.right) {
            std::cmp::Ordering::Less => B::parse("-1", "1"),
            std::cmp::Ordering::Equal => B::parse("0", "1"),
            std::cmp::Ordering::Greater => B::parse("1", "1"),
        }
        .expect("fixed ordering rational is valid");
        for value in [
            &case.left,
            &case.right,
            &add,
            &subtract,
            &multiply,
            &divide,
            &floor,
            &ceiling,
            &ordering,
        ] {
            digest = hash_rational::<B>(digest, value);
        }
    }
    digest
}

fn timed_workload<B: RationalBackend>(cases: &[Case<B::Rational>], iterations: u32) -> (u128, i64) {
    let mut sink = 0_i64;
    let started = Instant::now();
    for _ in 0..iterations {
        for case in cases {
            let add = B::add(&case.left, &case.right);
            let subtract = B::subtract(&case.left, &case.right);
            let multiply = B::multiply(&case.left, &case.right);
            let divide = B::divide(&case.left, &case.right);
            let floor = B::floor(&case.left);
            let ceiling = B::ceiling(&case.left);
            sink = sink.wrapping_add(match add.cmp(&subtract) {
                std::cmp::Ordering::Less => -1,
                std::cmp::Ordering::Equal => 0,
                std::cmp::Ordering::Greater => 1,
            });
            black_box((&add, &subtract, &multiply, &divide, &floor, &ceiling));
        }
    }
    (started.elapsed().as_nanos(), black_box(sink))
}

fn json_escape(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            other => escaped.push(other),
        }
    }
    escaped
}

/// Run one backend and print a single machine-readable result.
///
/// # Errors
///
/// Returns an error when the vector path is absent or its contents are invalid.
pub fn run_backend<B: RationalBackend>(backend: &str) -> Result<(), String> {
    let vector_path = std::env::args_os()
        .nth(1)
        .ok_or_else(|| "usage: BACKEND VECTOR_FILE".to_owned())?;
    let workloads = parse_workloads::<B>(Path::new(&vector_path))?;
    let mut rows = Vec::with_capacity(workloads.len());
    for workload in workloads {
        let iterations = iterations_for(&workload.name)?;
        let digest = correctness_digest::<B>(&workload.cases);
        let (elapsed_ns, sink) = timed_workload::<B>(&workload.cases, iterations);
        rows.push(ResultRow {
            name: workload.name,
            cases: workload.cases.len(),
            iterations,
            operations_per_case: 7,
            elapsed_ns,
            digest,
            sink,
        });
    }
    print!(
        "{{\"schema_version\":1,\"backend\":\"{}\",\"workloads\":[",
        json_escape(backend)
    );
    for (index, row) in rows.iter().enumerate() {
        if index != 0 {
            print!(",");
        }
        print!(
            concat!(
                "{{\"name\":\"{}\",\"cases\":{},\"iterations\":{},",
                "\"operations_per_case\":{},\"elapsed_ns\":{},",
                "\"digest\":\"{:016x}\",\"sink\":{}}}"
            ),
            json_escape(&row.name),
            row.cases,
            row.iterations,
            row.operations_per_case,
            row.elapsed_ns,
            row.digest,
            row.sink,
        );
    }
    println!("]}}");
    Ok(())
}
