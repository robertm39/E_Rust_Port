use crate::terms::termtypes::{term_identity_id, Term};
use std::fmt::{self, Write};

pub const TERM_POS_ELEMENT_SIZE: usize = 2;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TermPos {
    components: Vec<TermPosComponent>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct TermPosComponent {
    superterm: Term,
    index: usize,
}

impl TermPos {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            components: Vec::new(),
        }
    }

    #[must_use]
    pub fn is_top_pos(&self) -> bool {
        self.components.is_empty()
    }

    #[must_use]
    pub fn component_count(&self) -> usize {
        self.components.len()
    }

    #[must_use]
    pub fn stack_len(&self) -> usize {
        self.components.len() * TERM_POS_ELEMENT_SIZE
    }

    pub fn components(&self) -> impl Iterator<Item = (&Term, usize)> + '_ {
        self.components
            .iter()
            .map(|component| (&component.superterm, component.index))
    }

    pub fn clear(&mut self) {
        self.components.clear();
    }

    /// Pushes one position component.
    ///
    /// # Panics
    ///
    /// Panics if `index` is outside `superterm`'s arity, matching the C stack
    /// representation's invariant.
    pub fn push_component(&mut self, superterm: Term, index: usize) {
        assert!(
            index < superterm.arity(),
            "term-position index must select an existing argument"
        );
        self.components.push(TermPosComponent { superterm, index });
    }

    /// Returns the subterm denoted by this position.
    ///
    /// # Panics
    ///
    /// Panics if the stored final component no longer points at an initialized
    /// argument. The C helper has the same live-term invariant.
    #[must_use]
    pub fn get_subterm(&self, term: &Term) -> Term {
        if let Some(component) = self.components.last() {
            component
                .superterm
                .argument(component.index)
                .expect("term-position component must have an initialized argument")
        } else {
            term.clone()
        }
    }

    /// Moves to the first leftmost-innermost position of `term`.
    ///
    /// # Panics
    ///
    /// Panics if a traversed argument slot is uninitialized.
    pub fn first_li_position(&mut self, term: &Term) -> Term {
        self.clear();
        let mut current = term.clone();
        while current.arity() != 0 {
            self.push_component(current.clone(), 0);
            current = current
                .argument(0)
                .expect("leftmost-innermost traversal requires initialized args");
        }
        current
    }

    /// Advances to the next leftmost-innermost position.
    ///
    /// # Panics
    ///
    /// Panics if a traversed argument slot is uninitialized.
    pub fn next_li_position(&mut self) -> Option<Term> {
        let component = self.components.pop()?;
        let mut index = component.index;
        let mut current = component.superterm;
        if index < current.arity() - 1 {
            index += 1;
            while current.arity() != 0 {
                self.push_component(current.clone(), index);
                current = current
                    .argument(index)
                    .expect("leftmost-innermost traversal requires initialized args");
                index = 0;
            }
        }
        Some(current)
    }

    pub fn write_to(&self, output: &mut impl Write) -> fmt::Result {
        if self.components.is_empty() {
            return Ok(());
        }
        write!(output, "{}", self.components[0].index)?;
        for component in &self.components[1..] {
            writeln!(output, ".{}", component.index)?;
        }
        Ok(())
    }

    #[must_use]
    pub fn print_string(&self) -> String {
        let mut output = String::new();
        let _ = self.write_to(&mut output);
        output
    }

    pub fn write_debug_addresses(&self, output: &mut impl Write) -> fmt::Result {
        writeln!(output, "# TermPos--")?;
        for component in &self.components {
            writeln!(
                output,
                "# <0x{:x}> Subterm {}",
                term_identity_id(&component.superterm),
                component.index
            )?;
        }
        writeln!(output, "# --TermPos")
    }
}

#[must_use]
pub fn term_pos_is_top_pos(pos: &TermPos) -> bool {
    pos.is_top_pos()
}

#[cfg(test)]
mod tests {
    use super::{term_pos_is_top_pos, TermPos, TERM_POS_ELEMENT_SIZE};
    use crate::terms::termtypes::Term;

    fn sample_term() -> (Term, Term, Term, Term, Term) {
        let root = Term::top_alloc(10, 2);
        let a = Term::const_cell_alloc(1);
        let g = Term::top_alloc(11, 1);
        let b = Term::const_cell_alloc(2);
        g.set_argument(0, b.clone());
        root.set_argument(0, a.clone());
        root.set_argument(1, g.clone());
        (root, a, g, b, Term::const_cell_alloc(99))
    }

    #[test]
    fn top_position_and_stack_shape_match_c_representation() {
        let mut pos = TermPos::new();
        assert!(pos.is_top_pos());
        assert!(term_pos_is_top_pos(&pos));
        assert_eq!(pos.stack_len(), 0);
        assert_eq!(TERM_POS_ELEMENT_SIZE, 2);

        let (root, _, _, _, _) = sample_term();
        pos.push_component(root, 0);
        assert!(!pos.is_top_pos());
        assert_eq!(pos.component_count(), 1);
        assert_eq!(pos.stack_len(), 2);
    }

    #[test]
    fn get_subterm_uses_last_stored_superterm_and_index() {
        let (root, a, g, b, other) = sample_term();
        let mut pos = TermPos::new();
        assert_eq!(pos.get_subterm(&root), root);

        pos.push_component(root.clone(), 1);
        assert_eq!(pos.get_subterm(&other), g);
        pos.push_component(root.argument(1).unwrap(), 0);
        assert_eq!(pos.get_subterm(&other), b);
        assert_ne!(pos.get_subterm(&other), a);
    }

    #[test]
    fn leftmost_innermost_iteration_matches_c_order() {
        let (root, a, g, b, _) = sample_term();
        let mut pos = TermPos::new();

        assert_eq!(pos.first_li_position(&root), a);
        assert_eq!(pos.print_string(), "0");
        assert_eq!(pos.next_li_position(), Some(b));
        assert_eq!(pos.print_string(), "1.0\n");
        assert_eq!(pos.next_li_position(), Some(g));
        assert_eq!(pos.print_string(), "1");
        assert_eq!(pos.next_li_position(), Some(root));
        assert!(pos.is_top_pos());
        assert_eq!(pos.next_li_position(), None);
    }

    #[test]
    fn debug_address_print_uses_comment_lines() {
        let (root, _, _, _, _) = sample_term();
        let mut pos = TermPos::new();
        pos.push_component(root, 0);
        let mut output = String::new();
        pos.write_debug_addresses(&mut output).unwrap();
        assert!(output.starts_with("# TermPos--\n# <0x"));
        assert!(output.ends_with(" Subterm 0\n# --TermPos\n"));
    }
}
