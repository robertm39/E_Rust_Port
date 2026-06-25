use crate::basics::error::{Diagnostic, ErrorCode};
use crate::terms::termbanks::TermBank;
use crate::terms::termpos::TermPos;
use crate::terms::termtypes::{
    DerefType, RewriteDemodulator, Term, TP_IS_REWRITTEN, TP_IS_RREWRITTEN, TP_IS_SOS_REWRITTEN,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i32)]
pub enum RwResultType {
    NotRewritable = 0,
    LimitedRewritable = 1,
    AlwaysRewritable = 2,
}

/// Adds a rewrite link from `term` to `replace`.
///
/// # Panics
///
/// Panics if `term` and `replace` are the same term handle, matching the C
/// assertion in `TermAddRWLink`.
pub fn term_add_rw_link(
    term: &Term,
    replace: &Term,
    demod: Option<RewriteDemodulator>,
    sos: bool,
    result_type: RwResultType,
) {
    assert_ne!(term, replace, "rewrite link must point to another term");

    term.set_prop(TP_IS_REWRITTEN);
    if result_type == RwResultType::AlwaysRewritable {
        term.set_prop(TP_IS_RREWRITTEN);
    }
    term.set_rw_replace_field(Some(replace.clone()));
    term.set_rw_demod_field(demod);

    if sos {
        term.set_prop(TP_IS_SOS_REWRITTEN);
    }
}

pub fn term_delete_rw_link(term: &Term) {
    term.del_prop(TP_IS_REWRITTEN | TP_IS_RREWRITTEN | TP_IS_SOS_REWRITTEN);
    term.set_rw_replace_field(None);
    term.set_rw_demod_field(None);
}

/// Follows an existing rewrite chain to its final replacement.
///
/// # Panics
///
/// Panics if a term is marked rewritten without a replacement link, matching
/// the C assertion while traversing `TermRWReplaceField`.
#[must_use]
pub fn term_follow_rw_chain(term: &Term) -> Term {
    let mut current = term.clone();
    while current.is_rewritten() {
        current = current
            .rw_replace_field()
            .expect("rewritten term must have a replacement");
    }
    current
}

/// Replaces the subterm denoted by `pos` with `repl` and shares the result in
/// `bank`.
///
/// The ordinary first-order path mirrors C `TBTermPosReplace`: rebuild the term
/// from the designated position inside-out using top-cell copies, then insert
/// the resulting temporary term through `TBInsertNoProps`.
///
/// # Errors
///
/// Returns a diagnostic for the LFHO prefix-rewrite branch where C calls
/// `MakeRewrittenTerm`; that helper still depends on lambda normalization that
/// has not been ported.
///
/// # Panics
///
/// Panics if a stored position component has become invalid for its superterm,
/// matching the C assertions on `TermPos` stack contents.
pub fn tb_term_pos_replace(
    bank: &mut TermBank,
    repl: &Term,
    pos: &TermPos,
    deref: DerefType,
    remains: i32,
    _old_into: Option<&Term>,
) -> Result<Term, Diagnostic> {
    if remains > 0 {
        return Err(Diagnostic::new(
            ErrorCode::OTHER_ERROR,
            "LFHO MakeRewrittenTerm path for TBTermPosReplace is not yet implemented",
        ));
    }
    assert!(
        remains >= -1,
        "TBTermPosReplace remains sentinel must be -1, 0, or positive"
    );

    let mut replacement = repl.clone();
    let components = pos
        .components()
        .map(|(superterm, index)| (superterm.clone(), index))
        .collect::<Vec<_>>();
    for (old, subscript) in components.into_iter().rev() {
        let handle = Term::top_copy(&old);
        assert!(
            subscript < handle.arity(),
            "term-position index must select an existing argument"
        );
        handle.set_argument(subscript, replacement);
        replacement = handle;
    }

    bank.insert_no_props(&replacement, deref)
}

#[cfg(test)]
mod tests {
    use super::{
        tb_term_pos_replace, term_add_rw_link, term_delete_rw_link, term_follow_rw_chain,
        RewriteDemodulator, RwResultType, TP_IS_REWRITTEN, TP_IS_RREWRITTEN, TP_IS_SOS_REWRITTEN,
    };
    use crate::inout::scanner::Scanner;
    use crate::terms::signature::Signature;
    use crate::terms::termbanks::TermBank;
    use crate::terms::termpos::TermPos;
    use crate::terms::termtypes::{DerefType, Term, TP_CHECK_FLAG};
    use crate::terms::typebanks::TypeBank;

    fn parse_simple(bank: &mut TermBank, source: &str) -> Term {
        let mut scanner = Scanner::from_user_string(source, false).unwrap();
        bank.parse_term_simple(&mut scanner).unwrap()
    }

    #[test]
    fn rw_result_type_discriminants_match_c_enum() {
        assert_eq!(RwResultType::NotRewritable as i32, 0);
        assert_eq!(RwResultType::LimitedRewritable as i32, 1);
        assert_eq!(RwResultType::AlwaysRewritable as i32, 2);
    }

    #[test]
    fn add_and_delete_rewrite_link_preserve_c_fields() {
        let term = Term::const_cell_alloc(1);
        let replace = Term::const_cell_alloc(2);
        let demod = RewriteDemodulator::new(7);

        term_add_rw_link(
            &term,
            &replace,
            Some(demod),
            true,
            RwResultType::AlwaysRewritable,
        );

        assert!(term.query_prop(TP_IS_REWRITTEN));
        assert!(term.query_prop(TP_IS_RREWRITTEN));
        assert!(term.query_prop(TP_IS_SOS_REWRITTEN));
        assert_eq!(term.rw_replace_field(), Some(replace));
        assert_eq!(term.rw_demod_field(), Some(demod));
        assert!(term.is_top_rewritten());

        term_delete_rw_link(&term);

        assert!(!term.query_prop(TP_IS_REWRITTEN));
        assert!(!term.query_prop(TP_IS_RREWRITTEN));
        assert!(!term.query_prop(TP_IS_SOS_REWRITTEN));
        assert!(term.rw_replace_field().is_none());
        assert!(term.rw_demod_field().is_none());
        assert!(!term.is_top_rewritten());
    }

    #[test]
    fn limited_rewrite_link_does_not_set_root_rewrite_or_sos_flags() {
        let term = Term::const_cell_alloc(1);
        let replace = Term::const_cell_alloc(2);

        term_add_rw_link(
            &term,
            &replace,
            None,
            false,
            RwResultType::LimitedRewritable,
        );

        assert!(term.query_prop(TP_IS_REWRITTEN));
        assert!(!term.query_prop(TP_IS_RREWRITTEN));
        assert!(!term.query_prop(TP_IS_SOS_REWRITTEN));
        assert_eq!(term.rw_replace_field(), Some(replace));
        assert!(!term.is_top_rewritten());
    }

    #[test]
    fn follow_rw_chain_returns_final_unrewritten_term() {
        let first = Term::const_cell_alloc(1);
        let second = Term::const_cell_alloc(2);
        let third = Term::const_cell_alloc(3);

        term_add_rw_link(
            &first,
            &second,
            None,
            false,
            RwResultType::LimitedRewritable,
        );
        term_add_rw_link(
            &second,
            &third,
            None,
            false,
            RwResultType::LimitedRewritable,
        );

        assert_eq!(term_follow_rw_chain(&first), third);
    }

    #[test]
    fn term_pos_replace_rebuilds_nested_superterms_inside_out() {
        let mut bank = TermBank::new(Signature::new(TypeBank::new())).unwrap();
        let root = parse_simple(&mut bank, "f(a,g(b))");
        let repl = parse_simple(&mut bank, "c");
        let nested = root.argument(1).unwrap();
        root.set_prop(TP_CHECK_FLAG);
        nested.set_prop(TP_CHECK_FLAG);
        let mut pos = TermPos::new();
        pos.push_component(root.clone(), 1);
        pos.push_component(nested, 0);

        let replaced =
            tb_term_pos_replace(&mut bank, &repl, &pos, DerefType::Never, -1, None).unwrap();

        assert_eq!(bank.term_string(&replaced, true), "f(a,g(c))");
        assert!(replaced.is_shared());
        assert!(!replaced.query_prop(TP_CHECK_FLAG));
        assert!(!replaced.argument(1).unwrap().query_prop(TP_CHECK_FLAG));
        assert_eq!(bank.find(&replaced), Some(replaced));
    }

    #[test]
    fn term_pos_replace_top_position_uses_insert_no_props_deref() {
        let mut bank = TermBank::new(Signature::new(TypeBank::new())).unwrap();
        let replacement = parse_simple(&mut bank, "c");
        let var = parse_simple(&mut bank, "X");
        var.set_binding(Some(replacement.clone()));
        let pos = TermPos::new();

        let replaced =
            tb_term_pos_replace(&mut bank, &var, &pos, DerefType::Always, 0, None).unwrap();

        assert_eq!(replaced, replacement);
    }

    #[test]
    fn term_pos_replace_reports_deferred_lfho_remaining_arguments() {
        let mut bank = TermBank::new(Signature::new(TypeBank::new())).unwrap();
        let root = parse_simple(&mut bank, "f(a)");
        let repl = parse_simple(&mut bank, "g(a)");
        let pos = TermPos::new();

        let error = tb_term_pos_replace(&mut bank, &repl, &pos, DerefType::Never, 1, Some(&root))
            .unwrap_err();

        assert_eq!(
            error.message(),
            "LFHO MakeRewrittenTerm path for TBTermPosReplace is not yet implemented"
        );
    }
}
