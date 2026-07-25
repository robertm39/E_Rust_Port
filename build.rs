#![allow(clippy::missing_errors_doc, clippy::module_name_repetitions)]

#[path = "src/heuristics/schedule_vars_parser.rs"]
mod schedule_vars_parser;

use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::fmt::Write as _;
use std::fs;
use std::io;
use std::path::PathBuf;

use schedule_vars_parser::{
    parse_schedule_vars, ParsedSchedule, ParsedScheduleCell, ParsedScheduleClass,
};

fn main() -> Result<(), Box<dyn Error>> {
    println!("cargo:rerun-if-changed=eprover/HEURISTICS/schedule.vars");
    println!("cargo:rerun-if-changed=src/heuristics/schedule_vars_parser.rs");

    let manifest_dir = PathBuf::from(
        env::var_os("CARGO_MANIFEST_DIR")
            .ok_or_else(|| io::Error::other("CARGO_MANIFEST_DIR is not set"))?,
    );
    let schedule_path = manifest_dir.join("eprover/HEURISTICS/schedule.vars");
    let source = fs::read_to_string(&schedule_path)?;
    let data = parse_schedule_vars(&source).map_err(io::Error::other)?;
    let generated = generate_tables(&data)?;
    let output_dir = PathBuf::from(
        env::var_os("OUT_DIR").ok_or_else(|| io::Error::other("OUT_DIR is not set"))?,
    );
    fs::write(output_dir.join("schedule_tables.rs"), generated)?;
    Ok(())
}

fn generate_tables(
    data: &schedule_vars_parser::ParsedScheduleData,
) -> Result<String, Box<dyn Error>> {
    let mut output = String::with_capacity(2_500_000);
    writeln!(
        output,
        "// Generated from eprover/HEURISTICS/schedule.vars."
    )?;
    writeln!(
        output,
        "const PREDEFINED_STRATEGIES: &[PredefinedStrategy] = &["
    )?;
    for strategy in &data.strategies {
        writeln!(
            output,
            "    PredefinedStrategy {{ name: {:?}, definition: {:?} }},",
            strategy.name, strategy.definition
        )?;
    }
    writeln!(output, "];")?;

    let schedule_indexes = data
        .schedules
        .iter()
        .enumerate()
        .map(|(index, schedule)| (schedule.name.as_str(), index))
        .collect::<HashMap<_, _>>();
    for (index, schedule) in data.schedules.iter().enumerate() {
        generate_schedule(&mut output, index, schedule)?;
    }

    generate_class_map(
        &mut output,
        "PREPROCESSING_MAP",
        &data.preprocessing_map,
        &schedule_indexes,
    )?;
    generate_class_map(
        &mut output,
        "SEARCH_MAP",
        &data.search_map,
        &schedule_indexes,
    )?;
    let default_index = schedule_indexes
        .get("_DEFAULT_SCHEDULE")
        .ok_or_else(|| io::Error::other("parsed default schedule disappeared"))?;
    writeln!(
        output,
        "const DEFAULT_SCHEDULE: &[StaticScheduleCell] = SCHEDULE_{default_index};"
    )?;

    writeln!(output, "#[cfg(test)]")?;
    writeln!(
        output,
        "const GENERATED_SCHEDULES: &[StaticNamedSchedule] = &["
    )?;
    for (index, schedule) in data.schedules.iter().enumerate() {
        writeln!(
            output,
            "    StaticNamedSchedule {{ name: {:?}, cells: SCHEDULE_{index} }},",
            schedule.name
        )?;
    }
    writeln!(output, "];")?;
    generate_schedule_names(
        &mut output,
        "GENERATED_PREPROCESSING_SCHEDULE_NAMES",
        &data.preprocessing_map,
    )?;
    generate_schedule_names(
        &mut output,
        "GENERATED_SEARCH_SCHEDULE_NAMES",
        &data.search_map,
    )?;

    Ok(output)
}

fn generate_schedule(
    output: &mut String,
    index: usize,
    schedule: &ParsedSchedule,
) -> Result<(), std::fmt::Error> {
    writeln!(output, "#[allow(")?;
    writeln!(output, "    clippy::approx_constant,")?;
    writeln!(
        output,
        "    reason = \"generated schedule fractions are authoritative upstream data\""
    )?;
    writeln!(output, ")]")?;
    writeln!(output, "const SCHEDULE_{index}: &[StaticScheduleCell] = &[")?;
    for cell in &schedule.cells {
        generate_schedule_cell(output, cell)?;
    }
    writeln!(output, "];")
}

fn generate_schedule_cell(
    output: &mut String,
    cell: &ParsedScheduleCell,
) -> Result<(), std::fmt::Error> {
    let ordering = rust_ordering_name(&cell.ordering);
    let sine = cell
        .sine
        .as_ref()
        .map_or_else(|| "None".to_owned(), |value| format!("Some({value:?})"));
    writeln!(
        output,
        "    StaticScheduleCell {{ heuristic_name: {:?}, ordering: {ordering}, sine: {sine}, time_fraction: {:?}, time_absolute: {}, cores: {} }},",
        cell.heuristic_name, cell.time_fraction, cell.time_absolute, cell.cores
    )
}

fn generate_class_map(
    output: &mut String,
    constant_name: &str,
    entries: &[ParsedScheduleClass],
    schedule_indexes: &HashMap<&str, usize>,
) -> Result<(), Box<dyn Error>> {
    writeln!(output, "const {constant_name}: &[ScheduleClass] = &[")?;
    for entry in entries {
        let index = schedule_indexes
            .get(entry.schedule_name.as_str())
            .ok_or_else(|| {
                io::Error::other(format!(
                    "missing generated schedule {}",
                    entry.schedule_name
                ))
            })?;
        writeln!(
            output,
            "    ScheduleClass {{ key: {:?}, schedule: SCHEDULE_{index}, class_size: {} }},",
            entry.key, entry.class_size
        )?;
    }
    writeln!(output, "];")?;
    Ok(())
}

fn generate_schedule_names(
    output: &mut String,
    constant_name: &str,
    entries: &[ParsedScheduleClass],
) -> Result<(), std::fmt::Error> {
    writeln!(output, "#[cfg(test)]")?;
    writeln!(output, "const {constant_name}: &[&str] = &[")?;
    for entry in entries {
        writeln!(output, "    {:?},", entry.schedule_name)?;
    }
    writeln!(output, "];")
}

fn rust_ordering_name(name: &str) -> &'static str {
    match name {
        "NoOrdering" => "TermOrdering::NoOrdering",
        "Optimize" => "TermOrdering::Optimize",
        "KBO" => "TermOrdering::Kbo",
        "KBO6" => "TermOrdering::Kbo6",
        "LPO" => "TermOrdering::Lpo",
        "LPOCopy" => "TermOrdering::LpoCopy",
        "LPO4" => "TermOrdering::Lpo4",
        "LPO4Copy" => "TermOrdering::Lpo4Copy",
        "RPO" => "TermOrdering::Rpo",
        "Empty" => "TermOrdering::Empty",
        _ => panic!("unknown term ordering {name} in schedule.vars"),
    }
}
