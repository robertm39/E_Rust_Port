use crate::terms::functypes::FunCode;
use crate::terms::simpletypes::Type;
use crate::terms::termtypes::{Term, TermProperties, TP_OP_FLAG};
use crate::terms::termvars::VarBank;

/// Computes the C `TermTop(term, depth)` representation.
///
/// # Panics
///
/// Panics if `term` is not shared, if a copied cutoff term lacks a type, or if
/// the source term has an uninitialized argument slot.
#[must_use]
pub fn term_top(term: &Term, depth: i32, freshvars: &VarBank) -> Term {
    assert!(term.is_shared(), "term-top source term must be shared");
    freshvars.reset_v_counts();
    rek_term_top(term, depth, freshvars)
}

/// Computes the C `AltTermTop(term, depth)` representation.
///
/// # Panics
///
/// Panics if `term` is not shared, if a copied cutoff term lacks a type, or if
/// the source term has an uninitialized argument slot.
#[must_use]
pub fn alt_term_top(term: &Term, depth: i32, freshvars: &VarBank) -> Term {
    assert!(term.is_shared(), "term-top source term must be shared");
    freshvars.reset_v_counts();
    let mut bindings = Vec::new();
    let result = alt_rek_term_top(term, depth, freshvars, &mut bindings);
    clear_temp_bindings(bindings);
    result
}

/// Computes the C `CSTermTop(term, depth)` compact shared top representation.
///
/// # Panics
///
/// Panics if `term` is not shared, if a copied cutoff term lacks a type, or if
/// the source term has an uninitialized argument slot.
#[must_use]
pub fn cs_term_top(term: &Term, depth: i32, freshvars: &VarBank) -> Term {
    assert!(term.is_shared(), "term-top source term must be shared");
    freshvars.reset_v_counts();
    let mut bindings = Vec::new();

    term_del_prop_level(term, depth, TP_OP_FLAG);
    term_set_prop_at_level(term, depth, TP_OP_FLAG);
    let result = term_top_marked(term, freshvars, &mut bindings);

    clear_temp_bindings(bindings);
    result
}

/// Computes the C `ESTermTop(term, depth)` extended shared top representation.
///
/// # Panics
///
/// Panics if `term` is not shared, if a copied cutoff term lacks a type, or if
/// the source term has an uninitialized argument slot.
#[must_use]
pub fn es_term_top(term: &Term, depth: i32, freshvars: &VarBank) -> Term {
    assert!(term.is_shared(), "term-top source term must be shared");
    freshvars.reset_v_counts();
    let mut bindings = Vec::new();

    term_set_prop_at_level(term, depth, TP_OP_FLAG);
    term_del_prop_level(term, depth - 1, TP_OP_FLAG);
    let result = term_top_marked(term, freshvars, &mut bindings);

    clear_temp_bindings(bindings);
    result
}

fn term_del_prop_level(term: &Term, depth: i32, prop: TermProperties) {
    if depth < 0 {
        return;
    }

    let mut stack = vec![(term.clone(), depth)];
    while let Some((current, current_depth)) = stack.pop() {
        current.del_prop(prop);
        if current_depth != 0 {
            for index in 0..current.arity() {
                assert!(
                    !current.is_free_var(),
                    "free variables with arguments cannot be traversed"
                );
                stack.push((required_argument(&current, index), current_depth - 1));
            }
        }
    }
}

fn term_set_prop_at_level(term: &Term, depth: i32, prop: TermProperties) {
    let mut stack = vec![(term.clone(), depth)];
    while let Some((current, current_depth)) = stack.pop() {
        if current_depth != 0 {
            for index in 0..current.arity() {
                assert!(
                    !current.is_free_var(),
                    "free variables with arguments cannot be traversed"
                );
                stack.push((required_argument(&current, index), current_depth - 1));
            }
        } else {
            current.set_prop(prop);
        }
    }
}

fn rek_term_top(term: &Term, depth: i32, freshvars: &VarBank) -> Term {
    if depth == 0 {
        freshvars.get_alt_fresh_var(&required_type(term))
    } else if term.is_free_var() {
        freshvars.var_assert_alloc(term.f_code(), &required_type(term))
    } else {
        let handle = term_top_cell(term.f_code(), term.arity());
        for index in 0..term.arity() {
            let arg = required_argument(term, index);
            handle.set_argument(index, rek_term_top(&arg, depth - 1, freshvars));
        }
        handle
    }
}

fn alt_rek_term_top(
    term: &Term,
    depth: i32,
    freshvars: &VarBank,
    bindings: &mut Vec<Term>,
) -> Term {
    if depth == 0 {
        temp_binding_or_fresh(term, freshvars, bindings)
    } else if term.is_free_var() {
        freshvars.var_assert_alloc(term.f_code(), &required_type(term))
    } else {
        let handle = term_top_cell(term.f_code(), term.arity());
        for index in 0..term.arity() {
            let arg = required_argument(term, index);
            handle.set_argument(
                index,
                alt_rek_term_top(&arg, depth - 1, freshvars, bindings),
            );
        }
        handle
    }
}

fn term_top_marked(term: &Term, freshvars: &VarBank, bindings: &mut Vec<Term>) -> Term {
    if term.query_prop(TP_OP_FLAG) {
        temp_binding_or_fresh(term, freshvars, bindings)
    } else if term.is_free_var() {
        freshvars.var_assert_alloc(term.f_code(), &required_type(term))
    } else {
        let handle = term_top_cell(term.f_code(), term.arity());
        for index in 0..term.arity() {
            let arg = required_argument(term, index);
            handle.set_argument(index, term_top_marked(&arg, freshvars, bindings));
        }
        handle
    }
}

fn temp_binding_or_fresh(term: &Term, freshvars: &VarBank, bindings: &mut Vec<Term>) -> Term {
    if let Some(binding) = term.binding() {
        binding
    } else {
        let handle = freshvars.get_alt_fresh_var(&required_type(term));
        term.set_binding(Some(handle.clone()));
        bindings.push(term.clone());
        handle
    }
}

fn clear_temp_bindings(mut bindings: Vec<Term>) {
    while let Some(term) = bindings.pop() {
        assert!(term.binding().is_some(), "temporary binding disappeared");
        term.set_binding(None);
    }
}

fn term_top_cell(f_code: FunCode, arity: usize) -> Term {
    let term = Term::default_cell_arity_alloc(arity);
    term.set_f_code(f_code);
    term
}

fn required_argument(term: &Term, index: usize) -> Term {
    match term.argument(index) {
        Some(arg) => arg,
        None => panic!("term-top source has no argument at index {index}"),
    }
}

fn required_type(term: &Term) -> Type {
    match term.type_() {
        Some(type_) => type_,
        None => panic!("term-top source term lacks a type"),
    }
}

#[cfg(test)]
mod tests {
    use super::{alt_term_top, cs_term_top, es_term_top, term_top};
    use crate::terms::signature::{Signature, FP_IGNORE_PROPS};
    use crate::terms::simpletypes::Type;
    use crate::terms::termbanks::TermBank;
    use crate::terms::termtypes::{Term, TP_IS_SHARED, TP_OP_FLAG};
    use crate::terms::termvars::{is_alt_var, VarBank};
    use crate::terms::typebanks::TypeBank;

    #[test]
    fn term_top_replaces_each_cutoff_independently() {
        let types = TypeBank::new();
        let freshvars = VarBank::new(&types);
        let sort = types.i_type();
        let shared = typed_const(10, &sort);
        let root = shared_fun(20, &[shared.clone(), shared], &sort);

        let top = term_top(&root, 1, &freshvars);
        let left = top.argument(0).expect("left cutoff");
        let right = top.argument(1).expect("right cutoff");

        assert_eq!(top.f_code(), 20);
        assert_eq!(top.arity(), 2);
        assert_eq!(top.type_(), None);
        assert!(is_alt_var(&left));
        assert!(is_alt_var(&right));
        assert_ne!(left, right);
        assert_eq!(left.f_code(), -1);
        assert_eq!(right.f_code(), -3);
    }

    #[test]
    fn alt_term_top_reuses_cutoff_bindings_for_shared_subterms() {
        let types = TypeBank::new();
        let freshvars = VarBank::new(&types);
        let sort = types.i_type();
        let shared = shared_fun(10, &[typed_const(11, &sort)], &sort);
        let root = shared_fun(20, &[shared.clone(), shared.clone()], &sort);

        let top = alt_term_top(&root, 1, &freshvars);
        let left = top.argument(0).expect("left cutoff");
        let right = top.argument(1).expect("right cutoff");

        assert_eq!(left, right);
        assert_eq!(left.f_code(), -1);
        assert!(shared.binding().is_none());
    }

    #[test]
    fn compact_shared_top_marks_exact_level_and_keeps_markers() {
        let types = TypeBank::new();
        let freshvars = VarBank::new(&types);
        let sort = types.i_type();
        let shared = shared_fun(10, &[typed_const(11, &sort)], &sort);
        let root = shared_fun(20, &[shared.clone(), shared.clone()], &sort);
        root.set_prop(TP_OP_FLAG);

        let top = cs_term_top(&root, 1, &freshvars);
        let left = top.argument(0).expect("left cutoff");
        let right = top.argument(1).expect("right cutoff");

        assert_eq!(left, right);
        assert_eq!(left.f_code(), -1);
        assert!(!root.query_prop(TP_OP_FLAG));
        assert!(shared.query_prop(TP_OP_FLAG));
        assert!(shared.binding().is_none());
    }

    #[test]
    fn extended_shared_top_at_depth_zero_replaces_the_root() {
        let types = TypeBank::new();
        let freshvars = VarBank::new(&types);
        let sort = types.i_type();
        let root = shared_fun(20, &[typed_const(10, &sort)], &sort);

        let top = es_term_top(&root, 0, &freshvars);

        assert!(is_alt_var(&top));
        assert_eq!(top.f_code(), -1);
        assert!(root.query_prop(TP_OP_FLAG));
        assert!(root.binding().is_none());
    }

    #[test]
    fn compact_negative_depth_preserves_existing_markers() {
        let types = TypeBank::new();
        let freshvars = VarBank::new(&types);
        let sort = types.i_type();
        let stale_marked = shared_fun(10, &[typed_const(11, &sort)], &sort);
        let root = shared_fun(20, std::slice::from_ref(&stale_marked), &sort);
        stale_marked.set_prop(TP_OP_FLAG);

        let top = cs_term_top(&root, -1, &freshvars);
        let child = top.argument(0).expect("marked child");

        assert!(is_alt_var(&child));
        assert_eq!(child.f_code(), -1);
        assert!(stale_marked.query_prop(TP_OP_FLAG));
    }

    #[test]
    fn term_top_outputs_insert_through_the_term_bank_when_typed_later() {
        let mut signature = Signature::new(TypeBank::new());
        let sort = signature.type_bank().i_type();
        let mut symbol_counter = 0;
        let f_code = signature
            .get_new_typed_f_code(
                "term_top_test",
                std::slice::from_ref(&sort),
                &mut symbol_counter,
                &sort,
                FP_IGNORE_PROPS,
            )
            .expect("typed symbol allocation");
        let freshvars = VarBank::new(signature.type_bank());
        let root = shared_fun(f_code, &[typed_const(10, &sort)], &sort);

        let top = term_top(&root, 1, &freshvars);
        let mut bank = TermBank::new(signature).expect("term bank allocation");
        let inserted = bank.term_top_insert(top).expect("top insertion");

        assert!(inserted.is_shared());
        assert_eq!(inserted.f_code(), f_code);
    }

    fn typed_const(f_code: i64, type_: &Type) -> Term {
        let term = Term::const_cell_alloc(f_code);
        term.set_type(Some(type_.clone()));
        term.set_prop(TP_IS_SHARED);
        term
    }

    fn shared_fun(f_code: i64, args: &[Term], type_: &Type) -> Term {
        let term = Term::top_alloc(f_code, args.len());
        term.set_type(Some(type_.clone()));
        term.set_prop(TP_IS_SHARED);
        for (index, arg) in args.iter().enumerate() {
            term.set_argument(index, arg.clone());
        }
        term
    }
}
