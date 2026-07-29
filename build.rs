#![allow(clippy::missing_errors_doc, clippy::module_name_repetitions)]

#[path = "src/heuristics/schedule_vars_parser.rs"]
mod schedule_vars_parser;

use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::fmt::Write as _;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

use schedule_vars_parser::{
    parse_schedule_vars, ParsedSchedule, ParsedScheduleCell, ParsedScheduleClass,
};

fn main() -> Result<(), Box<dyn Error>> {
    println!("cargo:rerun-if-changed=src/heuristics/schedule.vars");
    println!("cargo:rerun-if-changed=src/heuristics/schedule_vars_parser.rs");

    let manifest_dir = PathBuf::from(
        env::var_os("CARGO_MANIFEST_DIR")
            .ok_or_else(|| io::Error::other("CARGO_MANIFEST_DIR is not set"))?,
    );
    let schedule_path = manifest_dir.join("src/heuristics/schedule.vars");
    let source = fs::read_to_string(&schedule_path)?;
    let data = parse_schedule_vars(&source).map_err(io::Error::other)?;
    let generated = generate_tables(&data)?;
    let output_dir = PathBuf::from(
        env::var_os("OUT_DIR").ok_or_else(|| io::Error::other("OUT_DIR is not set"))?,
    );
    fs::write(output_dir.join("schedule_tables.rs"), generated)?;
    if env::var_os("CARGO_FEATURE_CADICAL_STATIC").is_some() {
        build_static_cadical(&manifest_dir, &output_dir)?;
    }
    Ok(())
}

#[expect(
    clippy::too_many_lines,
    reason = "the optional native build keeps source selection, compilation, archiving, and link directives in one auditable transaction"
)]
fn build_static_cadical(manifest_dir: &Path, output_dir: &Path) -> Result<(), Box<dyn Error>> {
    const EXPECTED_VERSION: &str = "3.0.1";

    println!("cargo:rerun-if-env-changed=UMLAUT_CADICAL_SOURCE");
    println!("cargo:rerun-if-env-changed=CXX");
    println!("cargo:rerun-if-env-changed=CC");
    println!("cargo:rerun-if-env-changed=AR");
    println!("cargo:rerun-if-changed=native/cadical_ffi/umlaut_cadical.cpp");
    println!("cargo:rerun-if-changed=native/cadical_ffi/umlaut_cadical.h");

    let source_root = PathBuf::from(env::var_os("UMLAUT_CADICAL_SOURCE").ok_or_else(|| {
        io::Error::other(
            "cadical-static requires UMLAUT_CADICAL_SOURCE pointing to pinned CaDiCaL 3.0.1",
        )
    })?);
    let version_path = source_root.join("VERSION");
    let version = fs::read_to_string(&version_path)?.trim().to_owned();
    if version != EXPECTED_VERSION {
        return Err(io::Error::other(format!(
            "UMLAUT_CADICAL_SOURCE has version {version:?}, expected {EXPECTED_VERSION:?}"
        ))
        .into());
    }
    let source_dir = source_root.join("src");
    if !source_dir.join("cadical.hpp").is_file() {
        return Err(io::Error::other(format!(
            "{} does not contain src/cadical.hpp",
            source_root.display()
        ))
        .into());
    }

    let target = env::var("TARGET")?;
    let build_dir = output_dir.join("umlaut-cadical");
    fs::create_dir_all(&build_dir)?;
    let cxx = compiler_from_env("CXX", &target, default_cxx(&target));
    let cc = compiler_from_env("CC", &target, default_cc(&target));
    let ar = compiler_from_env("AR", &target, default_ar(&target));
    let mut objects = Vec::new();

    let mut cpp_sources = fs::read_dir(&source_dir)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "cpp"))
        .filter(|path| {
            !matches!(
                path.file_name().and_then(|name| name.to_str()),
                Some("cadical.cpp" | "mobical.cpp" | "ipasir.cpp")
            )
        })
        .collect::<Vec<_>>();
    cpp_sources.sort();
    cpp_sources.push(manifest_dir.join("native/cadical_ffi/umlaut_cadical.cpp"));

    for source in cpp_sources {
        println!("cargo:rerun-if-changed={}", source.display());
        let object = object_path(&build_dir, &source, "cpp");
        let mut command = Command::new(&cxx);
        command
            .arg("-std=c++17")
            .arg("-O3")
            .arg("-DNDEBUG")
            .arg("-DNBUILD")
            .arg("-I")
            .arg(&source_dir)
            .arg("-I")
            .arg(manifest_dir.join("native/cadical_ffi"))
            .arg("-c")
            .arg(&source)
            .arg("-o")
            .arg(&object);
        if target.contains("windows-gnu") {
            command.arg("-DNUNLOCKED");
        } else {
            command.arg("-fPIC");
        }
        run_build_command(&mut command, "compile CaDiCaL C++ source")?;
        objects.push(object);
    }

    let mut c_sources = fs::read_dir(&source_dir)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "c"))
        .collect::<Vec<_>>();
    c_sources.sort();
    for source in c_sources {
        println!("cargo:rerun-if-changed={}", source.display());
        let object = object_path(&build_dir, &source, "c");
        let mut command = Command::new(&cc);
        command
            .arg("-O3")
            .arg("-DNDEBUG")
            .arg("-DNBUILD")
            .arg("-I")
            .arg(&source_dir)
            .arg("-c")
            .arg(&source)
            .arg("-o")
            .arg(&object);
        if target.contains("windows-gnu") {
            command.arg("-DNUNLOCKED");
        } else {
            command.arg("-fPIC");
        }
        run_build_command(&mut command, "compile CaDiCaL C source")?;
        objects.push(object);
    }

    let archive = build_dir.join("libumlaut_cadical.a");
    let mut archive_command = Command::new(&ar);
    archive_command.arg("crs").arg(&archive).args(&objects);
    run_build_command(&mut archive_command, "archive CaDiCaL static library")?;

    println!("cargo:rustc-link-search=native={}", build_dir.display());
    println!("cargo:rustc-link-lib=static=umlaut_cadical");
    if target.contains("windows-gnu") {
        println!("cargo:rustc-link-lib=stdc++");
        println!("cargo:rustc-link-lib=winpthread");
    } else if target.contains("apple") {
        println!("cargo:rustc-link-lib=c++");
    } else {
        println!("cargo:rustc-link-lib=stdc++");
    }
    Ok(())
}

fn compiler_from_env(prefix: &str, target: &str, fallback: &str) -> String {
    let target_key = target.replace('-', "_");
    env::var(format!("{prefix}_{target_key}"))
        .ok()
        .or_else(|| env::var(format!("TARGET_{prefix}")).ok())
        .or_else(|| env::var(prefix).ok())
        .unwrap_or_else(|| fallback.to_owned())
}

fn default_cxx(target: &str) -> &'static str {
    if target == "x86_64-pc-windows-gnu" {
        "x86_64-w64-mingw32-g++"
    } else {
        "c++"
    }
}

fn default_cc(target: &str) -> &'static str {
    if target == "x86_64-pc-windows-gnu" {
        "x86_64-w64-mingw32-gcc"
    } else {
        "cc"
    }
}

fn default_ar(target: &str) -> &'static str {
    if target == "x86_64-pc-windows-gnu" {
        "x86_64-w64-mingw32-ar"
    } else {
        "ar"
    }
}

fn object_path(build_dir: &Path, source: &Path, language: &str) -> PathBuf {
    let stem = source
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("source");
    build_dir.join(format!("{language}-{stem}.o"))
}

fn run_build_command(command: &mut Command, action: &str) -> Result<(), Box<dyn Error>> {
    let status = command.status()?;
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::other(format!("{action} failed with {status}: {command:?}")).into())
    }
}

fn generate_tables(
    data: &schedule_vars_parser::ParsedScheduleData,
) -> Result<String, Box<dyn Error>> {
    let mut output = String::with_capacity(2_500_000);
    writeln!(output, "// Generated from src/heuristics/schedule.vars.")?;
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
