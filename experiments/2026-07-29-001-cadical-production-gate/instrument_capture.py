#!/usr/bin/env python3
"""Add deterministic SATCheck capture instrumentation to a remote source copy."""

from __future__ import annotations

import argparse
from pathlib import Path

IMPORTS_BEFORE = """use std::collections::BTreeMap;
use std::fmt;
use std::time::Instant;
"""
IMPORTS_AFTER = """use std::collections::BTreeMap;
use std::fs::{create_dir_all, File};
use std::fmt;
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

static SAT_CAPTURE_ORDINAL: AtomicU64 = AtomicU64::new(0);
"""
CALL_BEFORE = """        let solver_clauses = self.export_non_pure_to_solver_clauses();
        let use_selected_backend =
"""
CALL_AFTER = """        let solver_clauses = self.export_non_pure_to_solver_clauses();
        capture_solver_workload(&solver_clauses, self.max_lit);
        let use_selected_backend =
"""
ANCHOR = """fn usize_to_u64(value: usize) -> u64 {
"""
CAPTURE = r"""fn capture_solver_workload(clauses: &[Vec<i32>], max_lit: i32) {
    let Some(root) = std::env::var_os("UMLAUT_SAT_CAPTURE_DIR") else {
        return;
    };
    let label = std::env::var("UMLAUT_SAT_CAPTURE_LABEL")
        .unwrap_or_else(|_| "unlabelled".to_owned())
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.') {
                character
            } else {
                '_'
            }
        })
        .collect::<String>();
    let ordinal = SAT_CAPTURE_ORDINAL.fetch_add(1, Ordering::Relaxed);
    let capture_max = std::env::var("UMLAUT_SAT_CAPTURE_MAX")
        .ok()
        .and_then(|value| value.parse::<u64>().ok());
    if capture_max.is_some_and(|limit| ordinal >= limit) {
        return;
    }
    let mut directory = PathBuf::from(root);
    directory.push(label);
    if create_dir_all(&directory).is_err() {
        return;
    }
    let path = directory.join(format!(
        "{ordinal:06}-v{max_lit}-c{}.cnf",
        clauses.len()
    ));
    let temporary = path.with_extension("cnf.tmp");
    let Ok(file) = File::create(&temporary) else {
        return;
    };
    let mut output = BufWriter::new(file);
    if writeln!(output, "p cnf {max_lit} {}", clauses.len()).is_err() {
        return;
    }
    for clause in clauses {
        for literal in clause {
            if write!(output, "{literal} ").is_err() {
                return;
            }
        }
        if writeln!(output, "0").is_err() {
            return;
        }
    }
    if output.flush().is_ok() {
        let _ = std::fs::rename(temporary, path);
    }
}

"""


def replace_once(source: str, before: str, after: str, label: str) -> str:
    if source.count(before) != 1:
        raise ValueError(f"expected exactly one {label} anchor")
    return source.replace(before, after, 1)


def instrument(path: Path) -> None:
    source = path.read_text(encoding="utf-8")
    source = replace_once(source, IMPORTS_BEFORE, IMPORTS_AFTER, "import")
    source = replace_once(source, CALL_BEFORE, CALL_AFTER, "capture call")
    source = replace_once(source, ANCHOR, CAPTURE + ANCHOR, "function")
    path.write_text(source, encoding="utf-8", newline="\n")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("satinterface", type=Path)
    arguments = parser.parse_args()
    instrument(arguments.satinterface)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
