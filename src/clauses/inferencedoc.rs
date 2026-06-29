use std::fmt;

use crate::clauses::clause::{clause_write_pcl_with_options, Clause};
use crate::clauses::clause_props::{
    FormulaProperties, CP_TYPE_CONJECTURE, CP_TYPE_NEG_CONJECTURE, CP_TYPE_QUESTION, CP_WATCH_ONLY,
};
use crate::clauses::eqn::EqnPrintOptions;
use crate::terms::termbanks::TermBank;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PclStepPrintOptions {
    pub full_terms: bool,
    pub compact: bool,
    pub eqn_print_options: EqnPrintOptions,
}

impl Default for PclStepPrintOptions {
    fn default() -> Self {
        Self {
            full_terms: true,
            compact: false,
            eqn_print_options: EqnPrintOptions::tptp(),
        }
    }
}

#[must_use]
pub const fn pcl_type_str(type_: FormulaProperties) -> &'static str {
    match type_ {
        CP_TYPE_CONJECTURE => "conj",
        CP_TYPE_QUESTION => "que",
        CP_TYPE_NEG_CONJECTURE => "neg",
        _ => "",
    }
}

pub fn pcl_print_start(
    output: &mut impl fmt::Write,
    bank: &TermBank,
    clause: &Clause,
    print_clause: bool,
    options: PclStepPrintOptions,
) -> fmt::Result {
    if options.compact {
        write!(output, "{}:", clause.ident())?;
    } else {
        write!(output, "{:6} : ", clause.ident())?;
    }
    write!(output, "{}:", pcl_type_str(clause.query_tptp_type()))?;
    if print_clause {
        clause_write_pcl_with_options(
            output,
            bank,
            clause,
            options.full_terms,
            options.eqn_print_options,
        )?;
    }
    output.write_str(" : ")
}

pub fn pcl_formula_print_start(
    output: &mut impl fmt::Write,
    ident: i64,
    type_: FormulaProperties,
    rendered_formula: Option<&str>,
    options: PclStepPrintOptions,
) -> fmt::Result {
    if options.compact {
        write!(output, "{ident}:")?;
    } else {
        write!(output, "{ident:6} : ")?;
    }
    write!(output, "{}:", pcl_type_str(type_))?;
    if let Some(rendered_formula) = rendered_formula {
        output.write_str(rendered_formula)?;
    }
    output.write_str(" : ")
}

pub fn pcl_print_end(
    output: &mut impl fmt::Write,
    clause: &Clause,
    comment: Option<&str>,
    options: PclStepPrintOptions,
) -> fmt::Result {
    match (clause.query_prop(CP_WATCH_ONLY), comment) {
        (true, Some(comment)) => write!(
            output,
            "{}'wl,{comment}'",
            if options.compact { ":" } else { ": " }
        )?,
        (false, Some(comment)) => write!(
            output,
            "{}'{comment}'",
            if options.compact { ":" } else { " : " }
        )?,
        (true, None) => output.write_str(if options.compact { ":'wl'" } else { " : 'wl'" })?,
        (false, None) => {}
    }
    output.write_char('\n')
}

pub fn tstp_print_end(
    output: &mut impl fmt::Write,
    clause: &Clause,
    comment: Option<&str>,
) -> fmt::Result {
    match (clause.query_prop(CP_WATCH_ONLY), comment) {
        (true, Some(comment)) => write!(output, ",['wl,{comment}']")?,
        (false, Some(comment)) => write!(output, ",['{comment}']")?,
        (true, None) => output.write_str(",['wl']")?,
        (false, None) => {}
    }
    output.write_str(").\n")
}

pub fn pcl_formula_print_end(
    output: &mut impl fmt::Write,
    comment: Option<&str>,
    options: PclStepPrintOptions,
) -> fmt::Result {
    if let Some(comment) = comment {
        write!(
            output,
            "{}'{comment}'",
            if options.compact { ":" } else { " : " }
        )?;
    }
    output.write_char('\n')
}

pub fn tstp_formula_print_end(output: &mut impl fmt::Write, comment: Option<&str>) -> fmt::Result {
    if let Some(comment) = comment {
        write!(output, ",['{comment}']")?;
    }
    output.write_str(").\n")
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FormulaParentInference {
    SplitEquiv,
    Simplification,
    NegConjecture,
    Nnf,
    ShiftQuantors,
    VarRename,
    Skolemize,
    Distribute,
    AnnotateQuestion,
}

impl FormulaParentInference {
    #[must_use]
    pub const fn pcl_name(self) -> &'static str {
        match self {
            Self::SplitEquiv => "split_equiv",
            Self::Simplification => "fof_simplification",
            Self::NegConjecture => "assume_negation",
            Self::Nnf => "fof_nnf",
            Self::ShiftQuantors => "shift_quantors",
            Self::VarRename => "variable_rename",
            Self::Skolemize => "skolemize",
            Self::Distribute => "distribute",
            Self::AnnotateQuestion => "add_answer_literal",
        }
    }

    #[must_use]
    pub const fn tstp_status(self) -> &'static str {
        match self {
            Self::NegConjecture => "cth",
            Self::Skolemize => "esa",
            Self::SplitEquiv
            | Self::Simplification
            | Self::Nnf
            | Self::ShiftQuantors
            | Self::VarRename
            | Self::Distribute
            | Self::AnnotateQuestion => "thm",
        }
    }
}

pub fn write_pcl_formula_intro_def_inference(output: &mut impl fmt::Write) -> fmt::Result {
    output.write_str("introduced")
}

pub fn write_tstp_formula_intro_def_inference(output: &mut impl fmt::Write) -> fmt::Result {
    output.write_str("introduced(definition)")
}

pub fn write_pcl_formula_parent_inference(
    output: &mut impl fmt::Write,
    inference: FormulaParentInference,
    parent_id: i64,
) -> fmt::Result {
    write!(output, "{}({parent_id})", inference.pcl_name())
}

pub fn write_tstp_formula_parent_inference(
    output: &mut impl fmt::Write,
    inference: FormulaParentInference,
    parent_id: i64,
) -> fmt::Result {
    let name = inference.pcl_name();
    let status = inference.tstp_status();
    match inference {
        FormulaParentInference::SplitEquiv | FormulaParentInference::Skolemize => {
            write!(
                output,
                "inference({name}, [status({status})], [c_0_{parent_id}])"
            )
        }
        FormulaParentInference::AnnotateQuestion => write!(
            output,
            "inference({name}, [status({status})],[c_0_{parent_id},theory(answers)])"
        ),
        FormulaParentInference::Simplification
        | FormulaParentInference::NegConjecture
        | FormulaParentInference::Nnf
        | FormulaParentInference::ShiftQuantors
        | FormulaParentInference::VarRename
        | FormulaParentInference::Distribute => {
            write!(
                output,
                "inference({name}, [status({status})],[c_0_{parent_id}])"
            )
        }
    }
}

pub fn write_pcl_formula_apply_defs_inference(
    output: &mut impl fmt::Write,
    parent_id: i64,
    def_ids: &[i64],
) -> fmt::Result {
    for _ in def_ids {
        output.write_str("apply_def(")?;
    }
    write!(output, "{parent_id}")?;
    for def_id in def_ids {
        write!(output, ",{def_id})")?;
    }
    Ok(())
}

pub fn write_tstp_formula_apply_defs_inference(
    output: &mut impl fmt::Write,
    parent_id: i64,
    def_ids: &[i64],
) -> fmt::Result {
    for _ in def_ids {
        output.write_str("inference(apply_def,[status(thm)],[")?;
    }
    write!(output, "c_0_{parent_id}")?;
    for def_id in def_ids {
        write!(output, ",c_0_{def_id}])")?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        pcl_formula_print_end, pcl_formula_print_start, pcl_print_end, pcl_print_start,
        pcl_type_str, tstp_formula_print_end, tstp_print_end,
        write_pcl_formula_apply_defs_inference, write_pcl_formula_intro_def_inference,
        write_pcl_formula_parent_inference, write_tstp_formula_apply_defs_inference,
        write_tstp_formula_intro_def_inference, write_tstp_formula_parent_inference,
        FormulaParentInference, PclStepPrintOptions,
    };
    use crate::clauses::clause::Clause;
    use crate::clauses::clause_props::{
        CP_TYPE_AXIOM, CP_TYPE_CONJECTURE, CP_TYPE_HYPOTHESIS, CP_TYPE_LEMMA,
        CP_TYPE_NEG_CONJECTURE, CP_TYPE_QUESTION, CP_TYPE_UNKNOWN, CP_TYPE_WATCH_CLAUSE,
        CP_WATCH_ONLY,
    };
    use crate::terms::signature::Signature;
    use crate::terms::termbanks::TermBank;
    use crate::terms::typebanks::TypeBank;

    fn test_bank() -> TermBank {
        TermBank::new(Signature::new(TypeBank::new())).unwrap()
    }

    #[test]
    fn pcl_type_str_matches_c_explicit_roles() {
        assert_eq!(pcl_type_str(CP_TYPE_CONJECTURE), "conj");
        assert_eq!(pcl_type_str(CP_TYPE_QUESTION), "que");
        assert_eq!(pcl_type_str(CP_TYPE_NEG_CONJECTURE), "neg");
    }

    #[test]
    fn pcl_type_str_collapses_default_roles_to_empty_plain_axiom_surface() {
        for type_ in [
            CP_TYPE_UNKNOWN,
            CP_TYPE_AXIOM,
            CP_TYPE_HYPOTHESIS,
            CP_TYPE_LEMMA,
            CP_TYPE_WATCH_CLAUSE,
        ] {
            assert_eq!(pcl_type_str(type_), "");
        }
    }

    #[test]
    fn pcl_print_start_matches_c_spacing_and_clause_gate() {
        let bank = test_bank();
        let mut clause = Clause::empty();
        clause.set_ident(7);
        clause.set_tptp_type(CP_TYPE_NEG_CONJECTURE);

        let mut rendered = String::new();
        pcl_print_start(
            &mut rendered,
            &bank,
            &clause,
            true,
            PclStepPrintOptions::default(),
        )
        .unwrap();
        assert_eq!(rendered, "     7 : neg:[] : ");

        let mut rendered = String::new();
        pcl_print_start(
            &mut rendered,
            &bank,
            &clause,
            false,
            PclStepPrintOptions {
                compact: true,
                ..PclStepPrintOptions::default()
            },
        )
        .unwrap();
        assert_eq!(rendered, "7:neg: : ");
    }

    #[test]
    fn pcl_formula_print_start_matches_c_spacing_and_render_gate() {
        let mut rendered = String::new();
        pcl_formula_print_start(
            &mut rendered,
            7,
            CP_TYPE_NEG_CONJECTURE,
            Some("p(a)"),
            PclStepPrintOptions::default(),
        )
        .unwrap();
        assert_eq!(rendered, "     7 : neg:p(a) : ");

        let mut rendered = String::new();
        pcl_formula_print_start(
            &mut rendered,
            7,
            CP_TYPE_CONJECTURE,
            None,
            PclStepPrintOptions {
                compact: true,
                ..PclStepPrintOptions::default()
            },
        )
        .unwrap();
        assert_eq!(rendered, "7:conj: : ");
    }

    #[test]
    fn pcl_print_end_matches_c_comment_and_watchlist_spacing() {
        let plain = Clause::empty();
        let mut watch = Clause::empty();
        watch.set_prop(CP_WATCH_ONLY);

        let mut rendered = String::new();
        pcl_print_end(
            &mut rendered,
            &plain,
            Some("proof"),
            PclStepPrintOptions::default(),
        )
        .unwrap();
        assert_eq!(rendered, " : 'proof'\n");

        let mut rendered = String::new();
        pcl_print_end(
            &mut rendered,
            &plain,
            Some("proof"),
            PclStepPrintOptions {
                compact: true,
                ..PclStepPrintOptions::default()
            },
        )
        .unwrap();
        assert_eq!(rendered, ":'proof'\n");

        let mut rendered = String::new();
        pcl_print_end(
            &mut rendered,
            &watch,
            Some("proof"),
            PclStepPrintOptions::default(),
        )
        .unwrap();
        assert_eq!(rendered, ": 'wl,proof'\n");

        let mut rendered = String::new();
        pcl_print_end(&mut rendered, &watch, None, PclStepPrintOptions::default()).unwrap();
        assert_eq!(rendered, " : 'wl'\n");
    }

    #[test]
    fn tstp_print_end_matches_c_comment_and_watchlist_suffixes() {
        let plain = Clause::empty();
        let mut watch = Clause::empty();
        watch.set_prop(CP_WATCH_ONLY);

        let mut rendered = String::new();
        tstp_print_end(&mut rendered, &plain, Some("proof")).unwrap();
        assert_eq!(rendered, ",['proof']).\n");

        let mut rendered = String::new();
        tstp_print_end(&mut rendered, &watch, Some("proof")).unwrap();
        assert_eq!(rendered, ",['wl,proof']).\n");

        let mut rendered = String::new();
        tstp_print_end(&mut rendered, &watch, None).unwrap();
        assert_eq!(rendered, ",['wl']).\n");

        let mut rendered = String::new();
        tstp_print_end(&mut rendered, &plain, None).unwrap();
        assert_eq!(rendered, ").\n");
    }

    #[test]
    fn pcl_formula_print_end_matches_c_comment_spacing() {
        let mut rendered = String::new();
        pcl_formula_print_end(
            &mut rendered,
            Some("fof_simpl"),
            PclStepPrintOptions::default(),
        )
        .unwrap();
        assert_eq!(rendered, " : 'fof_simpl'\n");

        let mut rendered = String::new();
        pcl_formula_print_end(
            &mut rendered,
            Some("fof_simpl"),
            PclStepPrintOptions {
                compact: true,
                ..PclStepPrintOptions::default()
            },
        )
        .unwrap();
        assert_eq!(rendered, ":'fof_simpl'\n");

        let mut rendered = String::new();
        pcl_formula_print_end(&mut rendered, None, PclStepPrintOptions::default()).unwrap();
        assert_eq!(rendered, "\n");
    }

    #[test]
    fn tstp_formula_print_end_matches_c_comment_suffix() {
        let mut rendered = String::new();
        tstp_formula_print_end(&mut rendered, Some("fof_simpl")).unwrap();
        assert_eq!(rendered, ",['fof_simpl']).\n");

        let mut rendered = String::new();
        tstp_formula_print_end(&mut rendered, None).unwrap();
        assert_eq!(rendered, ").\n");
    }

    #[test]
    fn formula_intro_def_inference_names_match_c_pcl_and_tstp_split() {
        let mut rendered = String::new();
        write_pcl_formula_intro_def_inference(&mut rendered).unwrap();
        assert_eq!(rendered, "introduced");

        let mut rendered = String::new();
        write_tstp_formula_intro_def_inference(&mut rendered).unwrap();
        assert_eq!(rendered, "introduced(definition)");
    }

    #[test]
    fn formula_parent_inference_rendering_matches_c_status_and_spacing() {
        let mut rendered = String::new();
        write_pcl_formula_parent_inference(
            &mut rendered,
            FormulaParentInference::Simplification,
            12,
        )
        .unwrap();
        assert_eq!(rendered, "fof_simplification(12)");

        let mut rendered = String::new();
        write_tstp_formula_parent_inference(&mut rendered, FormulaParentInference::SplitEquiv, 12)
            .unwrap();
        assert_eq!(rendered, "inference(split_equiv, [status(thm)], [c_0_12])");

        let mut rendered = String::new();
        write_tstp_formula_parent_inference(
            &mut rendered,
            FormulaParentInference::NegConjecture,
            12,
        )
        .unwrap();
        assert_eq!(
            rendered,
            "inference(assume_negation, [status(cth)],[c_0_12])"
        );

        let mut rendered = String::new();
        write_tstp_formula_parent_inference(&mut rendered, FormulaParentInference::Skolemize, 12)
            .unwrap();
        assert_eq!(rendered, "inference(skolemize, [status(esa)], [c_0_12])");

        let mut rendered = String::new();
        write_tstp_formula_parent_inference(
            &mut rendered,
            FormulaParentInference::AnnotateQuestion,
            12,
        )
        .unwrap();
        assert_eq!(
            rendered,
            "inference(add_answer_literal, [status(thm)],[c_0_12,theory(answers)])"
        );
    }

    #[test]
    fn formula_apply_defs_inference_nests_definitions_like_c_stack_loop() {
        let mut rendered = String::new();
        write_pcl_formula_apply_defs_inference(&mut rendered, 9, &[21, 22]).unwrap();
        assert_eq!(rendered, "apply_def(apply_def(9,21),22)");

        let mut rendered = String::new();
        write_tstp_formula_apply_defs_inference(&mut rendered, 9, &[21, 22]).unwrap();
        assert_eq!(
            rendered,
            "inference(apply_def,[status(thm)],[inference(apply_def,[status(thm)],[c_0_9,c_0_21]),c_0_22])"
        );

        let mut rendered = String::new();
        write_pcl_formula_apply_defs_inference(&mut rendered, 9, &[]).unwrap();
        assert_eq!(rendered, "9");

        let mut rendered = String::new();
        write_tstp_formula_apply_defs_inference(&mut rendered, 9, &[]).unwrap();
        assert_eq!(rendered, "c_0_9");
    }
}
