//! High-level heuristic lookup and inline-definition parsing from `che_heuristics`.

use std::io::{self, Write};

use crate::basics::defines::DEFAULT_COMCHAR_RAW;
use crate::basics::error::{Diagnostic, ErrorCode};
use crate::heuristics::clausesetfeatures::{spec_no_eq, SpecFeatureCell};
use crate::heuristics::hcb::{
    heuristic_parms_print_string, AcHandling, HcbCell, HeuristicParmsCell,
    DEFAULT_DELETE_BAD_LIMIT, HCB_DEFAULT_HEURISTIC,
};
use crate::heuristics::hcbadmin::{heuristic_def_parse_with_context, HcbAdmin};
use crate::heuristics::wfcbadmin::{WeightParseContext, WfcbAdmin};
use crate::inout::output::outprint_to;
use crate::inout::scanner::{Scanner, TokenType};

#[must_use]
pub fn finalize_auto_parms(
    parms: &HeuristicParmsCell,
    spec: &SpecFeatureCell,
) -> HeuristicParmsCell {
    let mut result = parms.clone();
    apply_auto_parm_adjustments(&mut result, parms, spec);
    result
}

pub fn finalize_auto_parms_with_output(
    global_output: &mut impl Write,
    stderr: &mut impl Write,
    output_level: i64,
    parms: &HeuristicParmsCell,
    spec: &SpecFeatureCell,
) -> io::Result<HeuristicParmsCell> {
    let mut result = parms.clone();
    if output_level != 0 {
        global_output.write_all(b"Selected heuristic:\n")?;
        stderr.write_all(heuristic_parms_print_string(&result).as_bytes())?;
    }

    let no_eq = spec_no_eq(spec);
    apply_auto_parm_adjustments(&mut result, parms, spec);
    if no_eq {
        let message = format!("{DEFAULT_COMCHAR_RAW} No equality, disabling AC handling.\n#\n");
        outprint_to(global_output, output_level, 1, &message)?;
    }

    Ok(result)
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    reason = "C casts rlim_t through single-precision float before assigning to long long"
)]
fn apply_auto_parm_adjustments(
    result: &mut HeuristicParmsCell,
    parms: &HeuristicParmsCell,
    spec: &SpecFeatureCell,
) {
    if parms.mem_limit > 2 && parms.delete_bad_limit == DEFAULT_DELETE_BAD_LIMIT {
        result.delete_bad_limit = ((parms.mem_limit - 2) as f32 * 0.7) as i64;
    }
    if spec_no_eq(spec) {
        result.ac_handling = AcHandling::None;
    }
}

pub fn get_heuristic<'a>(
    source: &str,
    hcbs: &'a mut HcbAdmin,
    wfcbs: &mut WfcbAdmin,
) -> Result<&'a HcbCell<()>, Diagnostic> {
    get_heuristic_with_context(source, hcbs, wfcbs, WeightParseContext::empty())
}

pub fn get_heuristic_with_context<'a>(
    source: &str,
    hcbs: &'a mut HcbAdmin,
    wfcbs: &mut WfcbAdmin,
    context: WeightParseContext<'_>,
) -> Result<&'a HcbCell<()>, Diagnostic> {
    let handle = get_heuristic_handle_with_context(source, hcbs, wfcbs, context)?;
    let Some(hcb) = hcbs.hcb(handle) else {
        return Err(Diagnostic::new(
            ErrorCode::USAGE_ERROR,
            format!("Heuristic '{source}' unknown\n"),
        ));
    };
    Ok(hcb)
}

pub fn get_heuristic_handle(
    source: &str,
    hcbs: &mut HcbAdmin,
    wfcbs: &mut WfcbAdmin,
) -> Result<usize, Diagnostic> {
    get_heuristic_handle_with_context(source, hcbs, wfcbs, WeightParseContext::empty())
}

pub fn get_heuristic_handle_with_context(
    source: &str,
    hcbs: &mut HcbAdmin,
    wfcbs: &mut WfcbAdmin,
    context: WeightParseContext<'_>,
) -> Result<usize, Diagnostic> {
    let mut scanner = Scanner::from_option_string(source, true)?;
    let name = if scanner.test_tok(TokenType::OPEN_BRACKET) {
        heuristic_def_parse_with_context(hcbs, &mut scanner, wfcbs, context)?;
        scanner.check_tok(TokenType::NO_TOKEN)?;
        HCB_DEFAULT_HEURISTIC.to_owned()
    } else {
        scanner.check_tok(TokenType::IDENTIFIER)?;
        let name = scanner.current_token().literal();
        scanner.accept_tok(TokenType::IDENTIFIER)?;
        name
    };

    hcbs.find_hcb_handle(&name).ok_or_else(|| {
        Diagnostic::new(
            ErrorCode::USAGE_ERROR,
            format!("Heuristic '{name}' unknown\n"),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::{finalize_auto_parms, finalize_auto_parms_with_output, get_heuristic};
    use crate::basics::error::ErrorCode;
    use crate::heuristics::clausesetfeatures::{SpecFeatureCell, SpecFeatureClass};
    use crate::heuristics::hcb::{
        hcb_add_wfcb, hcb_alloc, heuristic_parms_print_string, AcHandling, HeuristicParmsCell,
        DEFAULT_DELETE_BAD_LIMIT,
    };
    use crate::heuristics::hcbadmin::HcbAdmin;
    use crate::heuristics::wfcbadmin::{weight_fun_parse, WfcbAdmin};
    use crate::inout::scanner::Scanner;

    fn scanner(source: &str) -> Scanner {
        Scanner::from_user_string(source, false).unwrap_or_else(|err| panic!("{err}"))
    }

    fn add_fifo(admin: &mut WfcbAdmin, name: &str) -> usize {
        let mut scanner = scanner("FIFOWeight(ConstPrio)");
        admin.add_wfcb(
            name,
            weight_fun_parse(&mut scanner).unwrap_or_else(|err| panic!("{err}")),
        )
    }

    #[test]
    fn finalize_auto_parms_derives_delete_bad_limit_like_c() {
        let parms = HeuristicParmsCell {
            mem_limit: 1_000,
            delete_bad_limit: DEFAULT_DELETE_BAD_LIMIT,
            ac_handling: AcHandling::KeepUnits,
            ..HeuristicParmsCell::default()
        };
        let spec = SpecFeatureCell {
            eq_clauses: 1,
            eq_content: SpecFeatureClass::SomeEq,
            ..SpecFeatureCell::default()
        };

        let finalized = finalize_auto_parms(&parms, &spec);

        assert_eq!(finalized.delete_bad_limit, 698);
        assert_eq!(finalized.ac_handling, AcHandling::KeepUnits);
        assert_eq!(parms.delete_bad_limit, DEFAULT_DELETE_BAD_LIMIT);
    }

    #[test]
    fn finalize_auto_parms_preserves_explicit_delete_bad_limit() {
        let parms = HeuristicParmsCell {
            mem_limit: 1_000,
            delete_bad_limit: 77,
            ..HeuristicParmsCell::default()
        };
        let spec = SpecFeatureCell {
            eq_clauses: 2,
            ..SpecFeatureCell::default()
        };

        let finalized = finalize_auto_parms(&parms, &spec);

        assert_eq!(finalized.delete_bad_limit, 77);
    }

    #[test]
    fn finalize_auto_parms_disables_ac_when_spec_has_no_equational_clauses() {
        let parms = HeuristicParmsCell {
            mem_limit: 2,
            delete_bad_limit: DEFAULT_DELETE_BAD_LIMIT,
            ac_handling: AcHandling::KeepOrientable,
            ..HeuristicParmsCell::default()
        };
        let spec = SpecFeatureCell {
            eq_clauses: 0,
            eq_content: SpecFeatureClass::SomeEq,
            ..SpecFeatureCell::default()
        };

        let finalized = finalize_auto_parms(&parms, &spec);

        assert_eq!(finalized.delete_bad_limit, DEFAULT_DELETE_BAD_LIMIT);
        assert_eq!(finalized.ac_handling, AcHandling::None);
    }

    #[test]
    fn finalize_auto_parms_with_output_matches_c_stream_split_and_order() {
        let parms = HeuristicParmsCell {
            mem_limit: 1_000,
            delete_bad_limit: DEFAULT_DELETE_BAD_LIMIT,
            ac_handling: AcHandling::KeepOrientable,
            ..HeuristicParmsCell::default()
        };
        let spec = SpecFeatureCell {
            eq_clauses: 0,
            ..SpecFeatureCell::default()
        };
        let mut global_output = Vec::new();
        let mut stderr = Vec::new();

        let finalized =
            finalize_auto_parms_with_output(&mut global_output, &mut stderr, 1, &parms, &spec)
                .unwrap();

        assert_eq!(finalized.delete_bad_limit, 698);
        assert_eq!(finalized.ac_handling, AcHandling::None);
        assert_eq!(
            String::from_utf8(global_output).unwrap(),
            "Selected heuristic:\n% No equality, disabling AC handling.\n#\n"
        );
        let stderr = String::from_utf8(stderr).unwrap();
        assert_eq!(stderr, heuristic_parms_print_string(&parms));
        assert!(stderr.contains("   delete_bad_limit:               9223372036854775807\n"));
        assert!(stderr.contains("   ac_handling:                    3\n"));
    }

    #[test]
    fn finalize_auto_parms_with_output_preserves_c_output_level_gates() {
        let parms = HeuristicParmsCell {
            ac_handling: AcHandling::KeepUnits,
            ..HeuristicParmsCell::default()
        };
        let spec = SpecFeatureCell {
            eq_clauses: 0,
            ..SpecFeatureCell::default()
        };
        let mut zero_global_output = Vec::new();
        let mut zero_stderr = Vec::new();

        let zero_finalized = finalize_auto_parms_with_output(
            &mut zero_global_output,
            &mut zero_stderr,
            0,
            &parms,
            &spec,
        )
        .unwrap();

        assert_eq!(zero_finalized.ac_handling, AcHandling::None);
        assert!(zero_global_output.is_empty());
        assert!(zero_stderr.is_empty());

        let mut negative_global_output = Vec::new();
        let mut negative_stderr = Vec::new();
        finalize_auto_parms_with_output(
            &mut negative_global_output,
            &mut negative_stderr,
            -1,
            &parms,
            &spec,
        )
        .unwrap();

        assert_eq!(
            String::from_utf8(negative_global_output).unwrap(),
            "Selected heuristic:\n"
        );
        assert_eq!(
            String::from_utf8(negative_stderr).unwrap(),
            heuristic_parms_print_string(&parms)
        );
    }

    #[test]
    fn get_heuristic_returns_named_hcb_and_ignores_trailing_tokens() {
        let mut wfcbs = WfcbAdmin::new();
        let mut hcbs = HcbAdmin::new();
        let mut hcb = hcb_alloc();
        hcb_add_wfcb(&mut hcb, 42, 1);
        hcbs.add_hcb("Named", hcb);

        let selected = get_heuristic("Named trailing tokens", &mut hcbs, &mut wfcbs)
            .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(selected.wfcb_no(), 1);
        assert_eq!(selected.wfcb_handle(0), Some(42));
    }

    #[test]
    fn get_heuristic_parses_inline_definition_as_default() {
        let mut wfcbs = WfcbAdmin::new();
        let fifo = add_fifo(&mut wfcbs, "fifo");
        let mut hcbs = HcbAdmin::new();

        let selected =
            get_heuristic("(2*fifo)", &mut hcbs, &mut wfcbs).unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(selected.wfcb_handle(0), Some(fifo));
        assert_eq!(selected.select_switch(0), Some(2));
        assert_eq!(hcbs.name(0), Some("Default"));
    }

    #[test]
    fn inline_definition_rejects_trailing_material_after_adding_default() {
        let mut wfcbs = WfcbAdmin::new();
        add_fifo(&mut wfcbs, "fifo");
        let mut hcbs = HcbAdmin::new();

        let Err(error) = get_heuristic("(1*fifo) extra", &mut hcbs, &mut wfcbs) else {
            panic!("inline heuristic with trailing material should fail");
        };

        assert_eq!(error.code(), ErrorCode::SYNTAX_ERROR);
        assert_eq!(hcbs.name(0), Some("Default"));
        assert_eq!(
            hcbs.find_hcb("Default")
                .and_then(|hcb| hcb.select_switch(0)),
            Some(1)
        );
    }

    #[test]
    fn named_definition_text_is_treated_as_plain_lookup() {
        let mut wfcbs = WfcbAdmin::new();
        add_fifo(&mut wfcbs, "fifo");
        let mut hcbs = HcbAdmin::new();
        let mut hcb = hcb_alloc();
        hcb_add_wfcb(&mut hcb, 99, 1);
        hcbs.add_hcb("Named", hcb);

        let selected = get_heuristic("Named=(2*fifo)", &mut hcbs, &mut wfcbs)
            .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(selected.wfcb_handle(0), Some(99));
        assert_eq!(hcbs.len(), 1);
    }

    #[test]
    fn unknown_named_heuristic_reports_usage_error() {
        let mut wfcbs = WfcbAdmin::new();
        let mut hcbs = HcbAdmin::new();

        let Err(error) = get_heuristic("Missing", &mut hcbs, &mut wfcbs) else {
            panic!("unknown heuristic name should fail");
        };

        assert_eq!(error.code(), ErrorCode::USAGE_ERROR);
        assert_eq!(error.message(), "Heuristic 'Missing' unknown\n");
    }
}
