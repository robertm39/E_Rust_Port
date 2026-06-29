use crate::basics::error::{Diagnostic, ErrorCode};
use crate::terms::lambda::beta_normalize_db;
use crate::terms::signature::SIG_PHONY_APP_CODE;
use crate::terms::termbanks::TermBank;
use crate::terms::termpos::TermPos;
use crate::terms::termtypes::{
    term_deref, DerefType, RewriteDemodulator, Term, TP_IS_REWRITTEN, TP_IS_RREWRITTEN,
    TP_IS_SOS_REWRITTEN, TP_PRED_POS,
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

/// Follows only top-level rewrite links and reports whether any traversed link
/// came from an `SoS` clause.
///
/// This mirrors C `term_follow_top_RW_chain`: traversal requires a demodulator
/// pointer on the rewrite link, and `restricted_rw` follows only restricted
/// rewrite links.
///
/// # Panics
///
/// Panics if a traversed top rewrite link is missing its replacement, matching
/// the C assertion while traversing `TermRWReplaceField`.
#[must_use]
pub fn term_follow_top_rw_chain(term: &Term, restricted_rw: bool) -> (Term, bool) {
    let mut current = term.clone();
    let mut sos_rewritten = false;

    while current.is_top_rewritten() && (!restricted_rw || current.is_rrewritten()) {
        if current.query_prop(TP_IS_SOS_REWRITTEN) {
            sos_rewritten = true;
        }
        current = current
            .rw_replace_field()
            .expect("top-rewritten term must have a replacement");
    }

    (current, sos_rewritten)
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
/// Returns a diagnostic if the C prefix-rewrite shape is missing the original
/// term required to append retained arguments, or if term-bank sharing or
/// beta-normalization fails.
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
    mut remains: i32,
    old_into: Option<&Term>,
) -> Result<Term, Diagnostic> {
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
        if remains == -1 {
            handle.set_argument(subscript, replacement);
        } else {
            let old_arg = handle
                .argument(subscript)
                .unwrap_or_else(|| panic!("position argument is initialized"));
            let old_arg = deref_always(&old_arg);
            let derefed_replacement = deref_always(&replacement);
            let rewritten = make_rewritten_term(
                bank,
                &old_arg,
                &derefed_replacement,
                usize::try_from(remains).expect("nonnegative remains fits usize"),
            )?;
            handle.set_argument(subscript, rewritten);
            remains = -1;
        }
        replacement = handle;
    }

    if remains > 0 {
        let old_into = old_into.ok_or_else(|| {
            Diagnostic::new(
                ErrorCode::OTHER_ERROR,
                "TBTermPosReplace prefix rewrite requires original term",
            )
        })?;
        let old_into = deref_always(old_into);
        let derefed_replacement = deref_always(&replacement);
        replacement = make_rewritten_term(
            bank,
            &old_into,
            &derefed_replacement,
            usize::try_from(remains).expect("positive remains fits usize"),
        )?;
    }

    bank.insert_no_props(&replacement, deref)
}

fn make_rewritten_term(
    bank: &mut TermBank,
    orig: &Term,
    new: &Term,
    remaining_orig: usize,
) -> Result<Term, Diagnostic> {
    assert!(
        remaining_orig <= orig.arity(),
        "remaining original arguments must be a suffix of the original term"
    );

    if remaining_orig == 0 {
        return beta_normalize_db(bank, new);
    }

    let retained_start = orig.arity() - remaining_orig;
    let retained = (retained_start..orig.arity())
        .map(|index| {
            orig.argument(index)
                .unwrap_or_else(|| panic!("original suffix argument is initialized"))
        })
        .collect::<Vec<_>>();

    let rewritten = if new.is_any_var() || new.is_lambda() {
        let rewritten = Term::top_alloc(SIG_PHONY_APP_CODE, remaining_orig + 1);
        rewritten.set_argument(0, new.clone());
        for (index, arg) in retained.into_iter().enumerate() {
            rewritten.set_argument(index + 1, arg);
        }
        rewritten
    } else {
        let rewritten = Term::top_alloc(new.f_code(), new.arity() + remaining_orig);
        for (index, arg) in new.argument_clones().into_iter().enumerate() {
            rewritten.set_argument_opt(index, arg);
        }
        for (index, arg) in retained.into_iter().enumerate() {
            rewritten.set_argument(new.arity() + index, arg);
        }
        rewritten
    };

    rewritten.set_type(orig.type_());
    rewritten.set_properties(orig.give_props(TP_PRED_POS));
    let shared = bank.term_top_insert(rewritten)?;
    beta_normalize_db(bank, &shared)
}

fn deref_always(term: &Term) -> Term {
    let mut deref = DerefType::Always;
    term_deref(term, &mut deref)
}

#[cfg(test)]
mod tests {
    use super::{
        tb_term_pos_replace, term_add_rw_link, term_delete_rw_link, term_follow_rw_chain,
        term_follow_top_rw_chain, RewriteDemodulator, RwResultType, TP_IS_REWRITTEN,
        TP_IS_RREWRITTEN, TP_IS_SOS_REWRITTEN,
    };
    use crate::inout::scanner::Scanner;
    use crate::terms::lambda::{apply_terms, close_with_type_prefix};
    use crate::terms::signature::Signature;
    use crate::terms::simpletypes::alloc_arrow_type;
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
    fn follow_top_rw_chain_stops_at_non_demodulator_links_and_reports_sos() {
        let first = Term::const_cell_alloc(1);
        let second = Term::const_cell_alloc(2);
        let third = Term::const_cell_alloc(3);
        let demod = RewriteDemodulator::new(11);

        term_add_rw_link(
            &first,
            &second,
            Some(demod),
            true,
            RwResultType::AlwaysRewritable,
        );
        term_add_rw_link(&second, &third, None, false, RwResultType::AlwaysRewritable);

        let (followed, sos_rewritten) = term_follow_top_rw_chain(&first, false);

        assert_eq!(followed, second);
        assert!(sos_rewritten);
    }

    #[test]
    fn follow_top_rw_chain_honors_restricted_rewrite_links() {
        let term = Term::const_cell_alloc(1);
        let replacement = Term::const_cell_alloc(2);
        let demod = RewriteDemodulator::new(13);

        term_add_rw_link(
            &term,
            &replacement,
            Some(demod),
            false,
            RwResultType::LimitedRewritable,
        );

        assert_eq!(term_follow_top_rw_chain(&term, false), (replacement, false));
        assert_eq!(term_follow_top_rw_chain(&term, true), (term, false));
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
    fn term_pos_replace_top_prefix_appends_remaining_original_arguments() {
        let mut bank = TermBank::new(Signature::new(TypeBank::new())).unwrap();
        let root = parse_simple(&mut bank, "f(a,b,c)");
        let repl = parse_simple(&mut bank, "g(a)");
        let pos = TermPos::new();

        let replaced =
            tb_term_pos_replace(&mut bank, &repl, &pos, DerefType::Never, 2, Some(&root)).unwrap();

        assert_eq!(bank.term_string(&replaced, true), "g(a,b,c)");
        assert!(replaced.is_shared());
        assert_eq!(bank.find(&replaced), Some(replaced));
    }

    #[test]
    fn term_pos_replace_nested_prefix_appends_remaining_original_arguments() {
        let mut bank = TermBank::new(Signature::new(TypeBank::new())).unwrap();
        let root = parse_simple(&mut bank, "f(g(a,b),c)");
        let repl = parse_simple(&mut bank, "h(a)");
        let nested = root.argument(0).unwrap();
        let mut pos = TermPos::new();
        pos.push_component(root, 0);

        let replaced =
            tb_term_pos_replace(&mut bank, &repl, &pos, DerefType::Never, 1, Some(&nested))
                .unwrap();

        assert_eq!(bank.term_string(&replaced, true), "f(h(a,b),c)");
    }

    #[test]
    fn term_pos_replace_prefix_beta_normalizes_lambda_replacement() {
        let mut signature = Signature::new(TypeBank::new());
        signature.insert_internal_codes().unwrap();
        let mut bank = TermBank::new(signature).unwrap();
        let individual = bank.signature().type_bank().i_type();
        let unary = bank
            .signature_mut()
            .type_bank_mut()
            .insert_type_shared(alloc_arrow_type(vec![
                individual.clone(),
                individual.clone(),
            ]));
        let k_code = bank.signature_mut().insert_id("rewrite_lambda_k", 1, false);
        bank.signature_mut()
            .declare_final_type(k_code, unary)
            .unwrap();
        let holder_code = bank
            .signature_mut()
            .insert_id("rewrite_lambda_holder", 1, false);
        bank.signature_mut()
            .declare_final_type(
                holder_code,
                alloc_arrow_type(vec![individual.clone(), individual.clone()]),
            )
            .unwrap();
        let b_code = bank.signature_mut().insert_id("rewrite_lambda_b", 0, false);
        bank.signature_mut()
            .declare_final_type(b_code, individual.clone())
            .unwrap();
        let k = bank.create_const_term(k_code).unwrap();
        let b = bank.create_const_term(b_code).unwrap();
        let db0 = bank.request_db_var(&individual, 0);
        let matrix = apply_terms(&mut bank, &k, std::slice::from_ref(&db0)).unwrap();
        let lambda =
            close_with_type_prefix(&mut bank, std::slice::from_ref(&individual), &matrix).unwrap();
        let holder = Term::top_alloc(holder_code, 1);
        holder.set_type(Some(individual));
        holder.set_argument(0, b.clone());
        let holder = bank.insert(&holder, DerefType::Never).unwrap();
        let pos = TermPos::new();

        let replaced =
            tb_term_pos_replace(&mut bank, &lambda, &pos, DerefType::Never, 1, Some(&holder))
                .unwrap();

        assert_eq!(replaced.f_code(), k_code);
        assert_eq!(replaced.argument(0), Some(b));
    }
}
