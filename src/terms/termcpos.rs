use crate::terms::termpos::TermPos;
use crate::terms::termtypes::Term;

pub type TermCPos = i64;

#[must_use]
pub const fn term_cpos_is_top_pos(pos: TermCPos) -> bool {
    pos == 0
}

#[must_use]
pub fn term_cpos_get_subterm(term: &Term, pos: TermCPos) -> Option<Term> {
    let mut pos = pos;
    term_cpos_get_subterm_rec(term, &mut pos)
}

#[must_use]
pub fn term_cpos_from_term_pos(root: &Term, pos: &TermPos) -> Option<TermCPos> {
    let target = pos.get_subterm(root);
    let mut next = 0;
    find_preorder_pos(root, &target, &mut next)
}

#[must_use]
pub fn term_pos_from_term_cpos(term: &Term, pos: TermCPos) -> Option<TermPos> {
    let mut next = 0;
    let mut path = Vec::new();
    find_preorder_path(term, pos, &mut next, &mut path).map(|components| {
        let mut term_pos = TermPos::new();
        for (superterm, index) in components {
            term_pos.push_component(superterm, index);
        }
        term_pos
    })
}

fn term_cpos_get_subterm_rec(term: &Term, pos: &mut TermCPos) -> Option<Term> {
    if *pos == 0 {
        return Some(term.clone());
    }
    *pos -= 1;
    for arg in term.argument_clones().into_iter().flatten() {
        if let Some(res) = term_cpos_get_subterm_rec(&arg, pos) {
            return Some(res);
        }
    }
    None
}

fn find_preorder_pos(term: &Term, target: &Term, next: &mut TermCPos) -> Option<TermCPos> {
    if term == target {
        return Some(*next);
    }
    *next += 1;
    for arg in term.argument_clones().into_iter().flatten() {
        if let Some(found) = find_preorder_pos(&arg, target, next) {
            return Some(found);
        }
    }
    None
}

fn find_preorder_path(
    term: &Term,
    target: TermCPos,
    next: &mut TermCPos,
    path: &mut Vec<(Term, usize)>,
) -> Option<Vec<(Term, usize)>> {
    if *next == target {
        return Some(path.clone());
    }
    *next += 1;
    for index in 0..term.arity() {
        let arg = term.argument(index)?;
        path.push((term.clone(), index));
        if let Some(found) = find_preorder_path(&arg, target, next, path) {
            return Some(found);
        }
        path.pop();
    }
    None
}

#[cfg(test)]
mod tests {
    use super::{
        term_cpos_from_term_pos, term_cpos_get_subterm, term_cpos_is_top_pos,
        term_pos_from_term_cpos,
    };
    use crate::terms::termpos::TermPos;
    use crate::terms::termtypes::Term;

    fn sample_term() -> (Term, Term, Term, Term) {
        let root = Term::top_alloc(10, 2);
        let a = Term::const_cell_alloc(1);
        let g = Term::top_alloc(11, 1);
        let b = Term::const_cell_alloc(2);
        g.set_argument(0, b.clone());
        root.set_argument(0, a.clone());
        root.set_argument(1, g.clone());
        (root, a, g, b)
    }

    #[test]
    fn top_position_is_zero() {
        assert!(term_cpos_is_top_pos(0));
        assert!(!term_cpos_is_top_pos(1));
    }

    #[test]
    fn compact_positions_follow_left_right_preorder() {
        let (root, a, g, b) = sample_term();

        assert_eq!(term_cpos_get_subterm(&root, 0), Some(root.clone()));
        assert_eq!(term_cpos_get_subterm(&root, 1), Some(a));
        assert_eq!(term_cpos_get_subterm(&root, 2), Some(g));
        assert_eq!(term_cpos_get_subterm(&root, 3), Some(b));
        assert_eq!(term_cpos_get_subterm(&root, 4), None);
        assert_eq!(term_cpos_get_subterm(&root, -1), None);
    }

    #[test]
    fn conversions_between_explicit_and_compact_positions_are_available() {
        let (root, _, _, b) = sample_term();
        let mut pos = TermPos::new();
        pos.push_component(root.clone(), 1);
        pos.push_component(root.argument(1).unwrap(), 0);

        assert_eq!(term_cpos_from_term_pos(&root, &pos), Some(3));
        let rebuilt = term_pos_from_term_cpos(&root, 3).unwrap();
        assert_eq!(rebuilt.get_subterm(&root), b);
        assert_eq!(rebuilt.print_string(), "1.0\n");
        assert!(term_pos_from_term_cpos(&root, 4).is_none());
    }
}
