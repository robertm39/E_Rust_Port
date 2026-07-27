#!/usr/bin/env python3
"""Audit the production formula-owner convergence and retained compatibility shims."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
from pathlib import Path


def read(repo: Path, relative: str) -> str:
    return (repo / relative).read_text(encoding="utf-8")


def contains(repo: Path, relative: str, *needles: str) -> bool:
    source = read(repo, relative)
    return all(needle in source for needle in needles)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo", type=Path, default=Path.cwd())
    parser.add_argument("--expected", type=Path)
    args = parser.parse_args()
    repo = args.repo.resolve()

    driver = read(repo, "src/prover/eprover.rs")
    destination = re.search(
        r"enum InputOwnerDestination<'a> \{(?P<body>.*?)\n\}", driver, re.DOTALL
    )
    handling = re.search(
        r"enum InputFormulaOwnerHandling \{(?P<body>.*?)\n\}", driver, re.DOTALL
    )
    if destination is None or handling is None:
        raise RuntimeError("input-owner destination definitions were not found")

    destination_body = destination.group("body")
    handling_body = handling.group("body")
    checks = {
        # Unchanged-C semantic anchors.
        "c_driver_reads_raw_features_before_cnf": contains(
            repo,
            "eprover/PROVER/eprover.c",
            "RawSpecFeaturesCompute(raw_features, proofstate);",
            "FormulaSetCNF2(proofstate->f_axioms",
        ),
        "c_driver_sine_and_app_encode_use_formula_set": contains(
            repo,
            "eprover/PROVER/eprover.c",
            "ProofStateSinE(proofstate, h_parms->sine)",
            "FormulaSetAppEncode(stdout, proofstate->f_axioms)",
        ),
        "c_formula_wrapper_uses_all_three_term_formula_parsers": contains(
            repo,
            "eprover/CLAUSES/ccl_formula_wrapper.c",
            "TFormulaTPTPParse(in, terms)",
            "TcfTSTPParse(in, terms)",
            "TFormulaTSTPParse(in, terms)",
        ),
        "c_formula_cnf_has_formula_set_owner": contains(
            repo,
            "eprover/CLAUSES/ccl_formulafunc.c",
            "long FormulaSetCNF2(FormulaSet_p set, FormulaSet_p archive",
        ),
        "c_sine_selects_clause_and_formula_sets": contains(
            repo,
            "eprover/CLAUSES/ccl_sine.c",
            "long SelectDefinitions(PStack_p clause_sets, PStack_p formula_sets",
        ),
        # Production input ownership. ClauseBridge is intentionally test-only.
        "production_destination_has_print_formula_set": (
            "FormulasForPrint(&'a mut FormulaSet)" in destination_body
        ),
        "production_destination_has_cnf_formula_set": (
            "FormulasForCnf(&'a mut FormulaSet)" in destination_body
        ),
        "clause_destination_is_test_only": bool(
            re.search(
                r"#\[cfg\(test\)\]\s+Clauses\(&'a mut ClauseSet\)",
                destination_body,
            )
        ),
        "clause_bridge_is_test_only": bool(
            re.search(r"#\[cfg\(test\)\]\s+ClauseBridge", handling_body)
        ),
        "production_input_targets_proof_state_formula_axioms": contains(
            repo,
            "src/prover/eprover.rs",
            "fn parse_input_files_into_formula_owners(",
            "state.terms_f_axioms_watchlist_mut()",
        ),
        "production_input_processes_distinct_after_aggregate_dialect": contains(
            repo,
            "src/prover/eprover.rs",
            "set_problem_type(parsed_problem_type)?;",
            "state.process_distinct()?;",
        ),
        "clause_records_are_wrapped_as_formulas": contains(
            repo,
            "src/prover/eprover.rs",
            "WrappedFormula::form_clause_alloc(bank, clause, ProblemType::FirstOrder)?;",
            "formulas.insert(formula);",
        ),
        "parsed_formula_owner_is_inserted_without_lowering": contains(
            repo,
            "src/prover/eprover.rs",
            "if let Some(formula) = parsed.owner_formula {",
            "formulas.insert(formula);",
        ),
        # Direct represented parsing and the compatibility fallback.
        "old_tptp_uses_term_formula_parser": contains(
            repo,
            "src/prover/eprover.rs",
            "let formula = bank.parse_tformula_tptp(scanner)?;",
            "parse_represented_tptp_formula_clause_body(",
        ),
        "tstp_dispatches_tcf_and_general_formula_parsers": contains(
            repo,
            "src/prover/eprover.rs",
            "if formula_kind == \"tcf\" {",
            "tcf_tstp_parse(scanner, bank, problem_type)",
            "bank.parse_tformula_tstp(scanner)",
        ),
        "represented_owner_rejects_non_boolean_root": contains(
            repo,
            "src/prover/eprover.rs",
            "if !formula_is_distinct && !formula.type_().as_ref().is_some_and(Type::is_bool)",
            "thf_formula_requires_full_pipeline_error(scanner)",
        ),
        "represented_owner_rejects_tstp_free_variables": contains(
            repo,
            "src/prover/eprover.rs",
            "tformula_has_free_vars(bank, &formula).is_some()",
            "tstp_formula_free_variables_error(formula_position)",
        ),
        "represented_owner_preserves_roles_and_source": contains(
            repo,
            "src/prover/eprover.rs",
            "role_types.raw_formula_type | CP_INPUT_FORMULA",
            "WrappedFormula::wt_formula_alloc(formula)",
            "ClauseInfo::new(Some(name), source, line, column)",
        ),
        "questions_are_annotated_on_formula_owner": contains(
            repo,
            "src/prover/eprover.rs",
            "owner_formula.annotate_question(",
            "formula_preprocessing.conjectures_are_questions",
        ),
        "type_declarations_retain_formula_placeholder": contains(
            repo,
            "src/prover/eprover.rs",
            "let formulas = [SimpleFofFormula::Truth(true)];",
            "CP_TYPE_AXIOM | CP_INPUT_FORMULA",
        ),
        "fallback_retains_original_raw_formula_features": contains(
            repo,
            "src/prover/eprover.rs",
            "let raw_formula_features =",
            "simple_fof_raw_formula_features(&formulas, role_types.raw_formula_type, bank);",
        ),
        # Formula-set preprocessing/CNF.
        "proof_search_archives_formula_owners_before_cnf": contains(
            repo,
            "src/prover/eprover.rs",
            "let _archived = formulas.archive_into(archive);",
            "formulas.preproc_conjectures",
        ),
        "proof_search_uses_formula_set_cnf": contains(
            repo,
            "src/prover/eprover.rs",
            "formulas.cnf2_into_with_docs_and_gc_context(",
            "formulas.cnf2_into_with_gc_context(",
        ),
        "cnf_orders_named_db_ite_let_and_unfold": contains(
            repo,
            "src/clauses/formulasets.rs",
            "self.named_to_db_lambdas(bank, options.problem_type)?;",
            "self.lift_ites(bank, options.problem_type)?;",
            "self.lift_lets(bank, options.problem_type)?;",
            "self.unfold_def_symbols(",
        ),
        "cnf_orders_lambda_fool_defs_and_extraction": contains(
            repo,
            "src/clauses/formulasets.rs",
            "self.lambda_normalize_forall(bank, options.problem_type)?;",
            "self.unroll_fool(bank)?;",
            "self.introduce_defs(archive, bank, options.def_limit)?;",
            "drain_formula_set_to_cnf(",
        ),
        # Formula-aware consumers.
        "raw_features_count_live_formula_owners": contains(
            repo,
            "src/heuristics/rawspecfeatures.rs",
            "let f_axioms = state.f_axioms();",
            "f_axioms.cardinality()",
            "f_axioms.standard_weight()",
            "f_axioms.count_conjectures",
        ),
        "raw_features_compensate_only_lowered_fallback": contains(
            repo,
            "src/heuristics/rawspecfeatures.rs",
            "raw_formula_features.lowered_clause_no",
            "raw_formula_features.lowered_clause_term_size",
            "raw_formula_features.lowered_conjecture_count",
        ),
        "raw_features_read_formula_order_and_definitions": contains(
            repo,
            "src/heuristics/rawspecfeatures.rs",
            "f_axioms.conjecture_order(signature)",
            "formula_set_definition_statistics(f_axioms, f_ax_archive, state.terms())",
        ),
        "threshold_sine_selects_both_owner_sets": contains(
            repo,
            "src/prover/eprover.rs",
            "select_threshold_clause_formula_sets(",
            "clause_sets.push(state.axioms());",
            "formula_sets.push(state.f_axioms());",
        ),
        "gsine_selects_stable_formula_entry_ids": contains(
            repo,
            "src/prover/eprover.rs",
            ".map(|formula| formula.entry_id())",
            "take_selected_formula_entry_ids(state.f_axioms_mut(), selected_formula_ids)",
        ),
        "definition_selection_has_formula_set_route": contains(
            repo,
            "src/clauses/sine.rs",
            "pub fn select_definitions_formula_sets<'a>(",
            "res_formulas.push(formula);",
        ),
        "formula_entry_identity_survives_owner_moves": contains(
            repo,
            "src/clauses/formulasets.rs",
            "Moving or cloning a wrapper preserves its logical `entry_id`.",
            "pub const fn entry_id(&self) -> u64",
            "pub fn move_formula_from(&mut self, from: &mut Self, entry_id: u64)",
        ),
        # Syntax/print/app-encode modes.
        "syntax_only_enters_shared_formula_owner_parse": contains(
            repo,
            "src/prover/eprover.rs",
            "fn run_syntax_only(",
            "parse_input_files_into_formula_owners(config, &mut state, stderr)?;",
        ),
        "app_encode_owns_a_formula_set": contains(
            repo,
            "src/prover/eprover.rs",
            "parse_app_encode_file_with_verbose_output(",
            "formula_set.app_encode_string_with_type_suffixes(bank, problem_type, true, print_types)?;",
        ),
        "app_encode_preserves_include_echo_side_channel": contains(
            repo,
            "src/prover/eprover.rs",
            "include_echoes.push_str(&parse_app_encode_ignored_include(scanner)?);",
            "output.write_stdout_side_channel(include_echoes.as_bytes())?;",
        ),
        "app_encode_preloads_formula_types": contains(
            repo,
            "src/clauses/formulasets.rs",
            "tformula_preload_types(bank, formula.formula())",
            "app_encode_string_with_type_suffixes(",
        ),
        # Retained evidence.
        "mode_matrix_is_28_of_28_exact": contains(
            repo,
            "experiments/2026-07-17-048-formula-owner-mode-matrix/FINDINGS.md",
            "All 28",
            "match exactly in exit status, stdout, and stderr",
        ),
        "rawspec_matrix_is_2_of_2_exact": contains(
            repo,
            "experiments/2026-07-17-083-rawspec-bridge-compensation/FINDINGS.md",
            "Both cases",
            "byte-exact",
        ),
        "sine_owner_comparison_is_exact": contains(
            repo,
            "experiments/2026-07-18-101-sine-formula-proof-search-closure/FINDINGS.md",
            "Both retained projections match exactly",
            "stable clause identifiers and allocation-unique formula entry ids",
        ),
        "formula_pipeline_has_69_check_audit": contains(
            repo,
            "experiments/2026-07-18-095-formula-pipeline-scope/FINDINGS.md",
            "passes 69/69 checks",
            "All five vendored",
        ),
        "route_corpus_found_no_c_accepted_bridge_dependency": contains(
            repo,
            "experiments/2026-07-15-006-formula-owner-route-corpus/FINDINGS.md",
            "found no C-accepted formula body that still requires the temporary simple-formula bridge",
        ),
        "typed_app_encode_ownership_is_exact": contains(
            repo,
            "experiments/2026-07-17-046-app-encode-typed-application-types/FINDINGS.md",
            "live C/Rust normalized executable comparison: exact",
        ),
        "explicit_bank_cache_decision_remains_green": contains(
            repo,
            "experiments/2026-07-25-035-lfho-explicit-bank-cache-decision/audit-reference.json",
            '"passed": 15',
            '"term_cell_bytes": 136',
        ),
    }

    report = {
        "check_count": len(checks),
        "checks": checks,
        "passed": sum(checks.values()),
        "schema_version": 1,
    }
    canonical = json.dumps(report, sort_keys=True, separators=(",", ":")).encode("utf-8")
    report["sha256"] = hashlib.sha256(canonical).hexdigest()
    encoded = json.dumps(report, indent=2, sort_keys=True) + "\n"
    sys.stdout.write(encoded)

    if not all(checks.values()):
        return 1
    if args.expected is not None:
        expected = args.expected.read_text(encoding="utf-8")
        if encoded != expected:
            print("formula-owner convergence audit reference changed", file=sys.stderr)
            return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
