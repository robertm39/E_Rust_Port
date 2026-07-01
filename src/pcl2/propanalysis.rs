//! Port of `PCL2/pcl_propanalysis`.

use std::cmp::Ordering;
use std::fmt::Write as _;

use crate::basics::defines::DEFAULT_COMCHAR_RAW;
use crate::basics::error::Diagnostic;
use crate::basics::simple_stuff::ProblemType;
use crate::clauses::clause::Clause;
use crate::heuristics::clausefeatures::clause_prop_info_print_string;
use crate::pcl2::idents::PclId;
use crate::pcl2::protocol::PclProtocol;
use crate::pcl2::steps::{PclStep, PclStepLogic};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PclPropData {
    pub fof_formulae: i64,
    pub pos_clauses: i64,
    pub neg_clauses: i64,
    pub mix_clauses: i64,
    pub pos_clause_literals: i64,
    pub neg_clause_literals: i64,
    pub mix_clause_literals: i64,
    pub pos_literals: i64,
    pub neg_literals: i64,
    pub const_count: i64,
    pub func_count: i64,
    pub pred_count: i64,
    pub var_count: i64,
    pub longest_clause: Option<PclId>,
    pub max_symbol_clause: Option<PclId>,
    pub max_standard_weight_clause: Option<PclId>,
    pub max_depth_clause: Option<PclId>,
}

/// C `PCLProtFindMaxStep`.
#[must_use]
pub fn protocol_find_max_step_by<Cmp>(protocol: &mut PclProtocol, cmp: Cmp) -> Option<PclId>
where
    Cmp: Fn(&PclStep, &PclStep) -> Ordering,
{
    let mut steps = protocol.ordered_steps().iter();
    let mut best = steps.next()?;
    for step in steps {
        if cmp(step, best).is_gt() {
            best = step;
        }
    }
    Some(best.id().clone())
}

/// C `PCLProtPropAnalyse`.
#[must_use]
pub fn protocol_prop_analyse(protocol: &mut PclProtocol) -> PclPropData {
    let mut data = PclPropData {
        max_standard_weight_clause: protocol_find_max_step_by(protocol, pcl_weight_compare),
        longest_clause: protocol_find_max_step_by(protocol, pcl_litno_compare),
        max_symbol_clause: protocol_find_max_step_by(protocol, pcl_sc_compare),
        max_depth_clause: protocol_find_max_step_by(protocol, pcl_depth_compare),
        ..PclPropData::default()
    };
    protocol_global_count(protocol, &mut data);
    data
}

/// C `PCLProtPropDataPrint`.
///
/// # Errors
///
/// Returns diagnostics from PCL step rendering.
pub fn protocol_prop_data_print_string(
    protocol: &mut PclProtocol,
    data: &PclPropData,
    problem_type: ProblemType,
) -> Result<String, Diagnostic> {
    let clauses = data.pos_clauses + data.neg_clauses + data.mix_clauses;
    let mut output = String::new();
    let _ = write!(
        output,
        concat!(
            "{comment} Protocol properties\n",
            "{comment} ===================\n",
            "{comment} Number of clauses                  : {clauses:6}\n",
            "{comment} ...of those positive               : {pos_clauses:6}\n",
            "{comment} ...of those negative               : {neg_clauses:6}\n",
            "{comment} ...of those mixed                  : {mix_clauses:6}\n",
            "{comment} Average number of literals         : {avg_literals:6.4}\n",
            "{comment} ...in positive clauses             : {avg_pos_clause_literals:6.4}\n",
            "{comment} ...in negative clauses             : {avg_neg_clause_literals:6.4}\n",
            "{comment} ...in mixed clauses                : {avg_mix_clause_literals:6.4}\n",
            "{comment} ...positive literals only          : {avg_pos_literals:6.4}\n",
            "{comment} ...negative literals only          : {avg_neg_literals:6.4}\n",
            "{comment} Average number of function  symbols: {avg_func_count:6.4}\n",
            "{comment} Average number of variable  symbols: {avg_var_count:6.4}\n",
            "{comment} Average number of constant  symbols: {avg_const_count:6.4}\n",
            "{comment} Average number of predicate symbols: {avg_pred_count:6.4}\n",
        ),
        comment = DEFAULT_COMCHAR_RAW,
        clauses = clauses,
        pos_clauses = data.pos_clauses,
        neg_clauses = data.neg_clauses,
        mix_clauses = data.mix_clauses,
        avg_literals = c_average(data.pos_literals + data.neg_literals, clauses),
        avg_pos_clause_literals = c_average(data.pos_clause_literals, data.pos_clauses),
        avg_neg_clause_literals = c_average(data.neg_clause_literals, data.neg_clauses),
        avg_mix_clause_literals = c_average(data.mix_clause_literals, data.mix_clauses),
        avg_pos_literals = c_average(data.pos_literals, clauses),
        avg_neg_literals = c_average(data.neg_literals, clauses),
        avg_func_count = c_average(data.func_count, clauses),
        avg_var_count = c_average(data.var_count, clauses),
        avg_const_count = c_average(data.const_count, clauses),
        avg_pred_count = c_average(data.pred_count, clauses),
    );

    let _ = writeln!(output, "{DEFAULT_COMCHAR_RAW} Longest Clause (if any): ");
    push_step_print(
        protocol,
        data.longest_clause.as_ref(),
        problem_type,
        &mut output,
    )?;
    let _ = writeln!(output, "\n{DEFAULT_COMCHAR_RAW} Largest Clause (if any): ");
    push_step_print(
        protocol,
        data.max_symbol_clause.as_ref(),
        problem_type,
        &mut output,
    )?;
    let _ = writeln!(output, "\n{DEFAULT_COMCHAR_RAW} Heaviest Clause (if any): ");
    push_clause_prop_info(
        protocol,
        data.max_standard_weight_clause.as_ref(),
        &mut output,
    );
    push_step_print(
        protocol,
        data.max_standard_weight_clause.as_ref(),
        problem_type,
        &mut output,
    )?;
    let _ = writeln!(output, "\n{DEFAULT_COMCHAR_RAW} Deepest Clause (if any): ");
    push_step_print(
        protocol,
        data.max_depth_clause.as_ref(),
        problem_type,
        &mut output,
    )?;
    output.push('\n');
    Ok(output)
}

fn protocol_global_count(protocol: &mut PclProtocol, data: &mut PclPropData) {
    for step in protocol.ordered_steps() {
        if step.is_fof() {
            data.fof_formulae += 1;
            continue;
        }

        let Some(clause) = step_clause(step) else {
            continue;
        };
        if clause.is_empty() {
            continue;
        }

        let literals = usize_to_i64(clause.literal_number());
        if clause.is_positive() {
            data.pos_clauses += 1;
            data.pos_clause_literals += literals;
        } else if clause.is_negative() {
            data.neg_clauses += 1;
            data.neg_clause_literals += literals;
        } else {
            data.mix_clauses += 1;
            data.mix_clause_literals += literals;
        }
        data.pos_literals += usize_to_i64(clause.positive_literal_count());
        data.neg_literals += usize_to_i64(clause.negative_literal_count());
        data.const_count += clause_symbol_weight(clause, 0, 0, 1, 0);
        data.func_count += clause_symbol_weight(clause, 0, 1, 0, 0);
        data.pred_count += clause_symbol_weight(clause, 0, 0, 0, 1);
        data.var_count += clause_symbol_weight(clause, 1, 0, 0, 0);
    }
}

fn pcl_weight_compare(left: &PclStep, right: &PclStep) -> Ordering {
    compare_clause_metric(left, right, Clause::standard_weight)
}

fn pcl_sc_compare(left: &PclStep, right: &PclStep) -> Ordering {
    compare_clause_metric(left, right, |clause| {
        clause_symbol_weight(clause, 1, 1, 1, 1)
    })
}

fn pcl_litno_compare(left: &PclStep, right: &PclStep) -> Ordering {
    compare_clause_metric(left, right, |clause| usize_to_i64(clause.literal_number()))
}

fn pcl_depth_compare(left: &PclStep, right: &PclStep) -> Ordering {
    compare_clause_metric(left, right, Clause::depth)
}

fn compare_clause_metric<Metric>(left: &PclStep, right: &PclStep, metric: Metric) -> Ordering
where
    Metric: Fn(&Clause) -> i64,
{
    match (left.is_fof(), right.is_fof()) {
        (true, true) => Ordering::Equal,
        (true, false) => Ordering::Less,
        (false, true) => Ordering::Greater,
        (false, false) => {
            let left_metric = step_clause(left).map_or(0, &metric);
            let right_metric = step_clause(right).map_or(0, metric);
            left_metric.cmp(&right_metric)
        }
    }
}

fn step_clause(step: &PclStep) -> Option<&Clause> {
    match step.logic() {
        PclStepLogic::Clause(clause) => Some(clause),
        PclStepLogic::Shell | PclStepLogic::Formula(_) => None,
    }
}

fn push_step_print(
    protocol: &mut PclProtocol,
    id: Option<&PclId>,
    problem_type: ProblemType,
    output: &mut String,
) -> Result<(), Diagnostic> {
    let Some(step) = id.and_then(|id| protocol.find_step(id)).cloned() else {
        return Ok(());
    };
    output.push_str(&step.print_extra_string(protocol.term_bank_mut(), problem_type, false)?);
    Ok(())
}

fn push_clause_prop_info(protocol: &PclProtocol, id: Option<&PclId>, output: &mut String) {
    let Some(step) = id.and_then(|id| protocol.find_step(id)) else {
        return;
    };
    let Some(clause) = step_clause(step) else {
        return;
    };
    output.push_str(&clause_prop_info_print_string(protocol.term_bank(), clause));
}

fn clause_symbol_weight(
    clause: &Clause,
    vweight: i64,
    fweight: i64,
    cweight: i64,
    pweight: i64,
) -> i64 {
    c_long_from_clause_weight(
        clause.sym_type_weight(1.0, 1.0, 1.0, vweight, fweight, cweight, pweight, 1.0),
    )
}

#[allow(clippy::cast_possible_truncation)]
fn c_long_from_clause_weight(weight: f64) -> i64 {
    weight as i64
}

fn usize_to_i64(value: usize) -> i64 {
    value.try_into().unwrap_or(i64::MAX)
}

#[allow(clippy::cast_precision_loss)]
fn c_average(numerator: i64, denominator: i64) -> f64 {
    numerator as f64 / denominator as f64
}

#[cfg(test)]
mod tests {
    use super::{protocol_prop_analyse, protocol_prop_data_print_string};
    use crate::basics::simple_stuff::ProblemType;
    use crate::inout::scanner::{IoFormat, Scanner};
    use crate::pcl2::idents::PclId;
    use crate::pcl2::protocol::PclProtocol;
    use crate::pcl2::steps::PclStepParseOptions;

    fn parse_id(source: &str) -> PclId {
        let mut scanner = Scanner::from_user_string(source, false).unwrap();
        PclId::parse(&mut scanner).unwrap()
    }

    fn parse_protocol(source: &str) -> PclProtocol {
        let mut protocol = PclProtocol::new().unwrap();
        let mut scanner = Scanner::from_user_string(source, false).unwrap();
        scanner.set_format(IoFormat::Tptp);
        protocol
            .parse(
                &mut scanner,
                PclStepParseOptions {
                    problem_type: ProblemType::FirstOrder,
                    support_shell_pcl: true,
                },
            )
            .unwrap();
        protocol
    }

    #[test]
    fn global_counts_skip_fof_and_empty_clauses_like_c() {
        let mut protocol = parse_protocol(
            "1 : : p(a) : initial\n\
             2 : : [++p(a)] : initial\n\
             3 : : [--q(X),--r(a)] : initial\n\
             4 : : [++s(f(a)),--t(X)] : initial\n\
             5 : : [] : initial",
        );

        let data = protocol_prop_analyse(&mut protocol);

        assert_eq!(data.fof_formulae, 1);
        assert_eq!(data.pos_clauses, 1);
        assert_eq!(data.neg_clauses, 1);
        assert_eq!(data.mix_clauses, 1);
        assert_eq!(data.pos_clause_literals, 1);
        assert_eq!(data.neg_clause_literals, 2);
        assert_eq!(data.mix_clause_literals, 2);
        assert_eq!(data.pos_literals, 2);
        assert_eq!(data.neg_literals, 3);
        assert!(data.const_count > 0);
        assert!(data.func_count > 0);
        assert!(data.pred_count > 0);
        assert!(data.var_count > 0);
    }

    #[test]
    fn max_step_selection_uses_clause_metrics_and_first_tie() {
        let mut protocol = parse_protocol(
            "1 : : p(a) : initial\n\
             2 : : [++p(a)] : initial\n\
             3 : : [++q(a),++r(a)] : initial\n\
             4 : : [++s(f(g(a)))] : initial\n\
             5 : : [++u(a),++v(a)] : initial",
        );

        let data = protocol_prop_analyse(&mut protocol);

        assert_eq!(data.longest_clause, Some(parse_id("3")));
        assert_eq!(data.max_depth_clause, Some(parse_id("4")));
        assert_ne!(data.max_standard_weight_clause, Some(parse_id("1")));
        assert_eq!(data.max_symbol_clause, data.max_standard_weight_clause);
    }

    #[test]
    fn property_data_prints_c_shaped_summary_and_step_sections() {
        let mut protocol = parse_protocol(
            "1 : : [++p(a)] : initial\n\
             2 : : [++q(a),--r(X)] : 1 : 'derived'",
        );
        let data = protocol_prop_analyse(&mut protocol);

        let rendered =
            protocol_prop_data_print_string(&mut protocol, &data, ProblemType::FirstOrder).unwrap();

        assert!(rendered.contains("% Protocol properties\n% ===================\n"));
        assert!(rendered.contains("% Number of clauses                  :      2\n"));
        assert!(rendered.contains("% Longest Clause (if any): \n"));
        assert!(rendered.contains("% Heaviest Clause (if any): \n"));
        assert!(rendered.contains("% Standardweight:"));
        assert!(rendered.contains("      2 :  : [++q(a),--r(X1)] : 1 : 'derived'"));
    }
}
