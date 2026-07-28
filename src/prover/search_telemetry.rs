//! Stable, opt-in aggregate telemetry for one saturation run.

use crate::basics::os_wrapper::ResourceUsage;
use crate::basics::simple_stuff::ProblemType;
use crate::clauses::proofstate::{ProofState, SearchTelemetryHighWater};
use crate::heuristics::proofcontrol::{
    ProcessClauseReturnReason, SaturateOutcome, SaturateReturnReason, SaturateStopReason,
};
use std::fmt::{self, Write as _};
use std::sync::atomic::Ordering;

pub(crate) const SEARCH_TELEMETRY_SCHEMA: &str = "umlaut.search-telemetry";
pub(crate) const SEARCH_TELEMETRY_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct SearchTelemetryCounterSnapshot {
    clause_subsumption_calls: u64,
    recursive_clause_subsumption_calls: u64,
    clause_subsumption_successes: u64,
    unit_subsumption_calls: u64,
    rewrite_unbound_variable_failures: u64,
    backward_rewrite_match_attempts: u64,
    backward_rewrite_match_successes: u64,
    condensation_attempts: u64,
    condensation_successes: u64,
}

impl SearchTelemetryCounterSnapshot {
    #[must_use]
    pub(crate) fn capture() -> Self {
        Self {
            clause_subsumption_calls: nonnegative_counter(
                crate::clauses::subsumption::clause_clause_subsumption_calls(),
            ),
            recursive_clause_subsumption_calls: nonnegative_counter(
                crate::clauses::subsumption::clause_clause_subsumption_calls_rec(),
            ),
            clause_subsumption_successes: nonnegative_counter(
                crate::clauses::subsumption::clause_clause_subsumption_successes(),
            ),
            unit_subsumption_calls: nonnegative_counter(
                crate::clauses::subsumption::unit_clause_clause_subsumption_calls(),
            ),
            rewrite_unbound_variable_failures: crate::clauses::rewrite::REWRITE_UNBOUND_VAR_FAILS
                .load(Ordering::Relaxed),
            backward_rewrite_match_attempts: crate::clauses::rewrite::BWRW_MATCH_ATTEMPTS
                .load(Ordering::Relaxed),
            backward_rewrite_match_successes: crate::clauses::rewrite::BWRW_MATCH_SUCCESSES
                .load(Ordering::Relaxed),
            condensation_attempts: nonnegative_counter(
                crate::clauses::condensation::condensation_attempts(),
            ),
            condensation_successes: nonnegative_counter(
                crate::clauses::condensation::condensation_successes(),
            ),
        }
    }

    fn since(self, baseline: Self) -> Self {
        Self {
            clause_subsumption_calls: self
                .clause_subsumption_calls
                .saturating_sub(baseline.clause_subsumption_calls),
            recursive_clause_subsumption_calls: self
                .recursive_clause_subsumption_calls
                .saturating_sub(baseline.recursive_clause_subsumption_calls),
            clause_subsumption_successes: self
                .clause_subsumption_successes
                .saturating_sub(baseline.clause_subsumption_successes),
            unit_subsumption_calls: self
                .unit_subsumption_calls
                .saturating_sub(baseline.unit_subsumption_calls),
            rewrite_unbound_variable_failures: self
                .rewrite_unbound_variable_failures
                .saturating_sub(baseline.rewrite_unbound_variable_failures),
            backward_rewrite_match_attempts: self
                .backward_rewrite_match_attempts
                .saturating_sub(baseline.backward_rewrite_match_attempts),
            backward_rewrite_match_successes: self
                .backward_rewrite_match_successes
                .saturating_sub(baseline.backward_rewrite_match_successes),
            condensation_attempts: self
                .condensation_attempts
                .saturating_sub(baseline.condensation_attempts),
            condensation_successes: self
                .condensation_successes
                .saturating_sub(baseline.condensation_successes),
        }
    }
}

pub(crate) struct SearchTelemetryRecord<'a> {
    pub files: &'a [String],
    pub problem_type: ProblemType,
    pub heuristic: &'a str,
    pub outcome: &'a SaturateOutcome,
    pub exit_status: u8,
    pub parsed_axioms: i64,
    pub relevancy_pruned: i64,
    pub raw_clauses: i64,
    pub preprocessing_removed: i64,
    pub state: &'a ProofState,
    pub counter_baseline: SearchTelemetryCounterSnapshot,
    pub resource_usage: ResourceUsage,
}

#[derive(Clone, Copy, Debug)]
struct SearchTelemetryDerived {
    counters: SearchTelemetryCounterSnapshot,
    final_processed: i64,
    final_unprocessed: i64,
    final_total: i64,
    final_archived: i64,
    high_water: SearchTelemetryHighWater,
    generated_clauses: u64,
    generated_literals: u64,
    returned_clause_depth: Option<i64>,
}

impl SearchTelemetryDerived {
    fn from_record(record: &SearchTelemetryRecord<'_>) -> Self {
        let state = record.state;
        let statistics = state.statistics();
        let final_processed = state.processed_cardinality();
        let final_unprocessed = state.unprocessed_cardinality();
        let final_total = final_processed.saturating_add(final_unprocessed);
        let final_archived = state.archive().members();
        Self {
            counters: SearchTelemetryCounterSnapshot::capture().since(record.counter_baseline),
            final_processed,
            final_unprocessed,
            final_total,
            final_archived,
            high_water: state
                .search_telemetry_high_water()
                .unwrap_or(SearchTelemetryHighWater {
                    processed_clauses: final_processed,
                    unprocessed_clauses: final_unprocessed,
                    total_clauses: final_total,
                    archived_clauses: final_archived,
                }),
            generated_clauses: statistics
                .generated_count
                .saturating_sub(statistics.backward_rewritten_count),
            generated_literals: statistics
                .generated_lit_count
                .saturating_sub(statistics.backward_rewritten_lit_count),
            returned_clause_depth: match record.outcome {
                SaturateOutcome::Returned { clause, .. } => Some(clause.proof_depth()),
                SaturateOutcome::Stopped { .. } => None,
            },
        }
    }
}

pub(crate) fn render_search_telemetry(
    record: &SearchTelemetryRecord<'_>,
) -> Result<String, fmt::Error> {
    let derived = SearchTelemetryDerived::from_record(record);
    let mut output = String::with_capacity(4_096);
    writeln!(output, "{{")?;
    write_identity_and_outcome(&mut output, record)?;
    write_search_funnel(&mut output, record, &derived)?;
    write_search_activity(&mut output, record, &derived)?;
    write_resources_and_proof(&mut output, record, &derived)?;
    writeln!(output, "}}")?;
    Ok(output)
}

fn write_identity_and_outcome(
    output: &mut String,
    record: &SearchTelemetryRecord<'_>,
) -> fmt::Result {
    writeln!(
        output,
        "  \"schema\": {},",
        json_string(SEARCH_TELEMETRY_SCHEMA)
    )?;
    writeln!(
        output,
        "  \"schema_version\": {SEARCH_TELEMETRY_SCHEMA_VERSION},"
    )?;
    write!(output, "  \"problem\": {{\"files\": [")?;
    write_json_string_array(output, record.files)?;
    writeln!(
        output,
        "], \"type\": {}}},",
        json_string(problem_type_name(record.problem_type))
    )?;
    writeln!(
        output,
        "  \"configuration\": {{\"heuristic\": {}}},",
        json_string(record.heuristic)
    )?;
    writeln!(
        output,
        "  \"outcome\": {{\"kind\": {}, \"reason\": {}, \"processed_steps\": {}, \"exit_status\": {}}},",
        json_string(outcome_kind(record.outcome)),
        json_string(outcome_reason(record.outcome)),
        record.outcome.processed_steps(),
        record.exit_status
    )?;
    writeln!(
        output,
        "  \"input_funnel\": {{\"parsed_axioms\": {}, \"relevancy_pruned\": {}, \"raw_clauses\": {}, \"preprocessing_removed\": {}}},",
        record.parsed_axioms,
        record.relevancy_pruned,
        record.raw_clauses,
        record.preprocessing_removed
    )
}

fn write_search_funnel(
    output: &mut String,
    record: &SearchTelemetryRecord<'_>,
    derived: &SearchTelemetryDerived,
) -> fmt::Result {
    let statistics = record.state.statistics();
    writeln!(
        output,
        "  \"search_funnel\": {{\"processed\": {}, \"trivial\": {}, \"forward_subsumed\": {}, \"processed_non_trivial\": {}, \"other_redundant\": {}, \"generated\": {}, \"generated_non_trivial\": {}, \"generated_literals\": {}, \"final_processed\": {}, \"final_unprocessed\": {}, \"final_total\": {}, \"final_archived\": {}, \"high_water_processed\": {}, \"high_water_unprocessed\": {}, \"high_water_total\": {}, \"high_water_archived\": {}}},",
        statistics.processed_count,
        statistics.proc_trivial_count,
        statistics.proc_forward_subsumed_count,
        statistics.proc_non_trivial_count,
        statistics.other_redundant_count,
        derived.generated_clauses,
        statistics.non_trivial_generated_count,
        derived.generated_literals,
        derived.final_processed,
        derived.final_unprocessed,
        derived.final_total,
        derived.final_archived,
        derived.high_water.processed_clauses,
        derived.high_water.unprocessed_clauses,
        derived.high_water.total_clauses,
        derived.high_water.archived_clauses
    )
}

fn write_search_activity(
    output: &mut String,
    record: &SearchTelemetryRecord<'_>,
    derived: &SearchTelemetryDerived,
) -> fmt::Result {
    let state = record.state;
    let statistics = state.statistics();
    let counters = derived.counters;
    writeln!(
        output,
        "  \"inferences\": {{\"paramodulations\": {}, \"factorizations\": {}, \"equation_resolutions\": {}, \"disequality_decompositions\": {}, \"negative_extensionality\": {}}},",
        statistics.paramod_count,
        statistics.factor_count,
        statistics.resolv_count,
        statistics.disequ_deco_count,
        statistics.neg_ext_count
    )?;
    writeln!(
        output,
        "  \"simplification\": {{\"rewrite_steps\": {}, \"contextual_simplify_reflections\": {}, \"backward_subsumed\": {}, \"backward_rewritten\": {}, \"aggressively_forward_subsumed\": {}, \"condensation_attempts\": {}, \"condensation_successes\": {}, \"rewrite_unbound_variable_failures\": {}}},",
        statistics.rw_count,
        statistics.context_sr_count,
        statistics.backward_subsumed_count,
        statistics.backward_rewritten_count,
        statistics.aggressive_forward_subsumed_count,
        counters.condensation_attempts,
        counters.condensation_successes,
        counters.rewrite_unbound_variable_failures
    )?;
    writeln!(
        output,
        "  \"indices\": {{\"clause_subsumption_calls\": {}, \"recursive_clause_subsumption_calls\": {}, \"clause_subsumption_successes\": {}, \"unit_subsumption_calls\": {}, \"oriented_demodulation_matches\": {}, \"unoriented_demodulation_matches\": {}, \"backward_rewrite_match_attempts\": {}, \"backward_rewrite_match_successes\": {}}},",
        counters.clause_subsumption_calls,
        counters.recursive_clause_subsumption_calls,
        counters.clause_subsumption_successes,
        counters.unit_subsumption_calls,
        state.processed_pos_rules().demod_index_match_count(),
        state.processed_pos_eqns().demod_index_match_count(),
        counters.backward_rewrite_match_attempts,
        counters.backward_rewrite_match_successes
    )
}

fn write_resources_and_proof(
    output: &mut String,
    record: &SearchTelemetryRecord<'_>,
    derived: &SearchTelemetryDerived,
) -> fmt::Result {
    let state = record.state;
    let statistics = state.statistics();
    writeln!(
        output,
        "  \"sat\": {{\"checks\": {}, \"satisfiable\": {}, \"unsatisfiable\": {}, \"input_clauses\": {}, \"post_purity_clauses\": {}, \"unsat_core_clauses\": {}, \"preprocessing_cpu_seconds\": {:.6}, \"encoding_cpu_seconds\": {:.6}, \"solver_cpu_seconds\": {:.6}}},",
        statistics.satcheck_count,
        statistics.satcheck_satisfiable,
        statistics.satcheck_success,
        statistics.satcheck_full_size,
        statistics.satcheck_actual_size,
        statistics.satcheck_core_size,
        statistics.satcheck_preproc_time,
        statistics.satcheck_encoding_time,
        statistics.satcheck_solver_time
    )?;
    writeln!(
        output,
        "  \"terms\": {{\"shared_nodes\": {}, \"insertions\": {}, \"recovered\": {}, \"storage_estimate_bytes\": {}}},",
        state.terms().term_nodes(),
        state.terms().insertions(),
        state.terms().recovered(),
        state.terms().storage_estimate()
    )?;
    write!(
        output,
        "  \"proof\": {{\"answer_count\": {}, \"returned_clause_depth\": ",
        statistics.answer_count
    )?;
    match derived.returned_clause_depth {
        Some(depth) => write!(output, "{depth}")?,
        None => output.write_str("null")?,
    }
    writeln!(
        output,
        ", \"proof_object_given_clauses\": {}, \"search_given_clauses\": {}}},",
        statistics.gc_used_count, statistics.gc_count
    )?;
    writeln!(
        output,
        "  \"resources\": {{\"user_cpu_seconds\": {:.6}, \"system_cpu_seconds\": {:.6}, \"total_cpu_seconds\": {:.6}, \"maximum_resident_pages\": {}}}",
        record.resource_usage.user_time_seconds,
        record.resource_usage.system_time_seconds,
        record.resource_usage.user_time_seconds + record.resource_usage.system_time_seconds,
        record.resource_usage.max_resident_pages
    )
}

fn write_json_string_array(output: &mut String, values: &[String]) -> fmt::Result {
    for (index, value) in values.iter().enumerate() {
        if index != 0 {
            output.write_str(", ")?;
        }
        write!(output, "{}", json_string(value))?;
    }
    Ok(())
}

#[derive(Clone, Copy, Debug)]
struct JsonString<'a>(&'a str);

impl fmt::Display for JsonString<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_char('"')?;
        for character in self.0.chars() {
            match character {
                '"' => formatter.write_str("\\\"")?,
                '\\' => formatter.write_str("\\\\")?,
                '\u{08}' => formatter.write_str("\\b")?,
                '\u{0c}' => formatter.write_str("\\f")?,
                '\n' => formatter.write_str("\\n")?,
                '\r' => formatter.write_str("\\r")?,
                '\t' => formatter.write_str("\\t")?,
                control if control <= '\u{1f}' => {
                    write!(formatter, "\\u{:04x}", u32::from(control))?;
                }
                printable => formatter.write_char(printable)?,
            }
        }
        formatter.write_char('"')
    }
}

const fn json_string(value: &str) -> JsonString<'_> {
    JsonString(value)
}

fn outcome_kind(outcome: &SaturateOutcome) -> &'static str {
    match outcome {
        SaturateOutcome::Returned { .. } => "returned",
        SaturateOutcome::Stopped { .. } => "stopped",
    }
}

fn outcome_reason(outcome: &SaturateOutcome) -> &'static str {
    match outcome {
        SaturateOutcome::Returned { reason, .. } => return_reason_name(*reason),
        SaturateOutcome::Stopped { reason, .. } => stop_reason_name(*reason),
    }
}

const fn return_reason_name(reason: SaturateReturnReason) -> &'static str {
    match reason {
        SaturateReturnReason::ProcessClause(ProcessClauseReturnReason::EmptyClause) => {
            "empty_clause"
        }
        SaturateReturnReason::ProcessClause(ProcessClauseReturnReason::AnswerLimit) => {
            "answer_limit"
        }
        SaturateReturnReason::ReplacingInference => "replacing_inference",
        SaturateReturnReason::GeneratedClause => "generated_clause",
        SaturateReturnReason::Cleanup => "cleanup",
        SaturateReturnReason::Filter => "filter",
        SaturateReturnReason::SatCheckPreprocessing => "sat_check_preprocessing",
        SaturateReturnReason::SatCheck => "sat_check",
    }
}

const fn stop_reason_name(reason: SaturateStopReason) -> &'static str {
    match reason {
        SaturateStopReason::TimeLimit => "time_limit",
        SaturateStopReason::Saturated => "saturated",
        SaturateStopReason::StepLimit => "step_limit",
        SaturateStopReason::ProcessedLimit => "processed_limit",
        SaturateStopReason::UnprocessedLimit => "unprocessed_limit",
        SaturateStopReason::TotalLimit => "total_limit",
        SaturateStopReason::GeneratedLimit => "generated_limit",
        SaturateStopReason::TermBankInsertionLimit => "term_bank_insertion_limit",
        SaturateStopReason::WatchlistEmpty => "watchlist_empty",
    }
}

const fn problem_type_name(problem_type: ProblemType) -> &'static str {
    match problem_type {
        ProblemType::NotInitialized => "not_initialized",
        ProblemType::FirstOrder => "first_order",
        ProblemType::HigherOrder => "higher_order",
    }
}

fn nonnegative_counter(value: i64) -> u64 {
    u64::try_from(value).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::{
        json_string, return_reason_name, stop_reason_name, SearchTelemetryCounterSnapshot,
    };
    use crate::heuristics::proofcontrol::{
        ProcessClauseReturnReason, SaturateReturnReason, SaturateStopReason,
    };

    #[test]
    fn json_string_escapes_control_and_path_characters() {
        assert_eq!(
            json_string("a\\b\"\n\t\u{0001}").to_string(),
            "\"a\\\\b\\\"\\n\\t\\u0001\""
        );
    }

    #[test]
    fn outcome_reason_names_are_stable_snake_case() {
        assert_eq!(
            return_reason_name(SaturateReturnReason::ProcessClause(
                ProcessClauseReturnReason::EmptyClause
            )),
            "empty_clause"
        );
        assert_eq!(
            return_reason_name(SaturateReturnReason::SatCheckPreprocessing),
            "sat_check_preprocessing"
        );
        assert_eq!(
            stop_reason_name(SaturateStopReason::TermBankInsertionLimit),
            "term_bank_insertion_limit"
        );
    }

    #[test]
    fn counter_snapshots_use_per_run_saturating_deltas() {
        let baseline = SearchTelemetryCounterSnapshot {
            clause_subsumption_calls: 10,
            condensation_successes: 5,
            ..SearchTelemetryCounterSnapshot::default()
        };
        let current = SearchTelemetryCounterSnapshot {
            clause_subsumption_calls: 14,
            condensation_successes: 3,
            ..SearchTelemetryCounterSnapshot::default()
        };
        let delta = current.since(baseline);
        assert_eq!(delta.clause_subsumption_calls, 4);
        assert_eq!(delta.condensation_successes, 0);
    }
}
