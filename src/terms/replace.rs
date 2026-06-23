use crate::terms::termtypes::{
    RewriteDemodulator, Term, TP_IS_REWRITTEN, TP_IS_RREWRITTEN, TP_IS_SOS_REWRITTEN,
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

#[cfg(test)]
mod tests {
    use super::{
        term_add_rw_link, term_delete_rw_link, term_follow_rw_chain, RewriteDemodulator,
        RwResultType, TP_IS_REWRITTEN, TP_IS_RREWRITTEN, TP_IS_SOS_REWRITTEN,
    };
    use crate::terms::termtypes::Term;

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
}
