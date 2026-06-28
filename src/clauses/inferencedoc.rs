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
        (true, None) => output.write_str(if options.compact { ":'wl'" } else { ": 'wl'" })?,
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

#[cfg(test)]
mod tests {
    use super::{
        pcl_formula_print_end, pcl_print_end, pcl_print_start, pcl_type_str,
        tstp_formula_print_end, tstp_print_end, PclStepPrintOptions,
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
        assert_eq!(rendered, ": 'wl'\n");
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
}
