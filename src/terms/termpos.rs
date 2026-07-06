use crate::basics::simple_stuff::ProblemType;
use crate::terms::termbanks::TermBank;
use crate::terms::termtypes::{term_identity_id, DerefType, Term};
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

    /// Writes C `TermPosDebugPrint` output for a non-null signature.
    ///
    /// C prints each stored superterm twice: first with `DEREF_NEVER`, then
    /// after a literal `...` with `DEREF_ALWAYS`, followed by the selected
    /// child index.
    ///
    /// # Panics
    ///
    /// Panics if a printed non-constant term has an uninitialized argument.
    pub fn write_debug_terms(
        &self,
        output: &mut impl Write,
        bank: &TermBank,
        problem_type: ProblemType,
    ) -> fmt::Result {
        writeln!(output, "# TermPos--")?;
        for component in &self.components {
            write!(output, "# ")?;
            bank.write_term_deref_for_problem(
                output,
                &component.superterm,
                problem_type,
                DerefType::Never,
            )?;
            write!(output, "...")?;
            bank.write_term_deref_for_problem(
                output,
                &component.superterm,
                problem_type,
                DerefType::Always,
            )?;
            writeln!(output, " Subterm {}", component.index)?;
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
    use crate::basics::simple_stuff::ProblemType;
    use crate::inout::scanner::Scanner;
    use crate::terms::lambda::{apply_terms, close_with_type_prefix};
    use crate::terms::signature::{Signature, SIG_LET_CODE};
    use crate::terms::simpletypes::{alloc_arrow_type, Type};
    use crate::terms::termbanks::TermBank;
    use crate::terms::termtypes::Term;
    use crate::terms::typebanks::TypeBank;

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

    fn parse_in_bank(bank: &mut TermBank, source: &str) -> Term {
        let mut scanner = Scanner::from_user_string(source, false).unwrap();
        bank.parse_term_simple(&mut scanner).unwrap()
    }

    fn formula_bank() -> TermBank {
        let mut signature = Signature::new(TypeBank::new());
        signature.insert_internal_codes().unwrap();
        TermBank::new(signature).unwrap()
    }

    fn typed_const(bank: &mut TermBank, name: &str) -> Term {
        let type_ = bank.signature().type_bank().default_type();
        typed_const_with_type(bank, name, type_)
    }

    fn typed_const_with_type(bank: &mut TermBank, name: &str, type_: Type) -> Term {
        let f_code = bank.signature_mut().insert_id(name, 0, false);
        if bank.signature().get_type(f_code).is_none() {
            bank.signature_mut()
                .declare_final_type(f_code, type_)
                .unwrap();
        }
        bank.create_const_term(f_code).unwrap()
    }

    fn bool_binary_with_code(bank: &mut TermBank, f_code: i64, left: &Term, right: &Term) -> Term {
        let type_ = bank.signature().type_bank().bool_type();
        let term = Term::top_alloc(f_code, 2);
        term.set_type(Some(type_));
        term.set_argument(0, left.clone());
        term.set_argument(1, right.clone());
        bank.term_top_insert(term).unwrap()
    }

    fn first_order_let_term(bank: &mut TermBank) -> Term {
        let type_ = bank.signature().type_bank().default_type();
        let local_code = bank.signature_mut().insert_id("tp_let_f", 0, false);
        bank.signature_mut()
            .declare_final_type(local_code, type_.clone())
            .unwrap();
        let lhs = bank.create_const_term(local_code).unwrap();
        let rhs = typed_const(bank, "tp_let_value");
        let eqn_code = bank.signature().eqn_code();
        let definition = bool_binary_with_code(bank, eqn_code, &lhs, &rhs);
        let term = Term::top_alloc(SIG_LET_CODE, 2);
        term.set_type(Some(type_));
        term.set_argument(0, definition);
        term.set_argument(1, lhs);
        bank.term_top_insert(term).unwrap()
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

    #[test]
    fn debug_term_print_uses_never_and_always_deref_pair() {
        let mut bank = TermBank::new(Signature::new(TypeBank::new())).unwrap();
        let root = parse_in_bank(&mut bank, "f(X,g(a))");
        let binding = parse_in_bank(&mut bank, "h(b)");
        let var = bank.vars().ext_name_find("X").unwrap();
        var.set_binding(Some(binding));
        let mut pos = TermPos::new();
        pos.push_component(root, 1);

        let mut output = String::new();
        pos.write_debug_terms(&mut output, &bank, ProblemType::FirstOrder)
            .unwrap();

        assert_eq!(
            output,
            "# TermPos--\n# f(X1,g(a))...f(h(b),g(a)) Subterm 1\n# --TermPos\n"
        );
    }

    #[test]
    fn debug_term_print_uses_higher_order_term_surface() {
        let mut bank = TermBank::new(Signature::new(TypeBank::new())).unwrap();
        let root = parse_in_bank(&mut bank, "f(X)");
        let binding = parse_in_bank(&mut bank, "g(a)");
        let var = bank.vars().ext_name_find("X").unwrap();
        var.set_binding(Some(binding));
        let mut pos = TermPos::new();
        pos.push_component(root, 0);

        let mut output = String::new();
        pos.write_debug_terms(&mut output, &bank, ProblemType::HigherOrder)
            .unwrap();

        assert_eq!(
            output,
            "# TermPos--\n# f @ X1...f @ (g @ a) Subterm 0\n# --TermPos\n"
        );
    }

    #[test]
    fn debug_term_print_uses_first_order_let_surface() {
        let mut bank = formula_bank();
        let let_term = first_order_let_term(&mut bank);
        let mut pos = TermPos::new();
        pos.push_component(let_term, 1);

        let mut output = String::new();
        pos.write_debug_terms(&mut output, &bank, ProblemType::FirstOrder)
            .unwrap();

        assert_eq!(
            output,
            "# TermPos--\n# $let(tp_let_f : $i, tp_let_f := tp_let_value, tp_let_f)...\
             $let(tp_let_f : $i, tp_let_f := tp_let_value, tp_let_f) Subterm 1\n# --TermPos\n"
        );
    }

    #[test]
    fn debug_term_print_uses_higher_order_fool_formula_surface() {
        let mut bank = formula_bank();
        let left = typed_const(&mut bank, "tp_fool_left");
        let right = typed_const(&mut bank, "tp_fool_right");
        let eqn_code = bank.signature().eqn_code();
        let equality = bool_binary_with_code(&mut bank, eqn_code, &left, &right);
        let mut pos = TermPos::new();
        pos.push_component(equality, 0);

        let mut output = String::new();
        pos.write_debug_terms(&mut output, &bank, ProblemType::HigherOrder)
            .unwrap();

        assert_eq!(
            output,
            "# TermPos--\n# ((tp_fool_left)=(tp_fool_right))...\
             ((tp_fool_left)=(tp_fool_right)) Subterm 0\n# --TermPos\n"
        );
    }

    #[test]
    fn debug_term_print_uses_higher_order_db_lambda_surface() {
        let mut bank = formula_bank();
        let type_ = bank.signature().type_bank().default_type();
        let unary_type = alloc_arrow_type(vec![type_.clone(), type_.clone()]);
        let function = typed_const_with_type(&mut bank, "tp_lambda_f", unary_type);
        let db0 = bank.request_db_var(&type_, 0);
        let matrix = apply_terms(&mut bank, &function, std::slice::from_ref(&db0)).unwrap();
        let lambda =
            close_with_type_prefix(&mut bank, std::slice::from_ref(&type_), &matrix).unwrap();
        let mut pos = TermPos::new();
        pos.push_component(lambda, 1);

        let mut output = String::new();
        pos.write_debug_terms(&mut output, &bank, ProblemType::HigherOrder)
            .unwrap();

        assert_eq!(
            output,
            "# TermPos--\n# ^[Z0:$i]:(tp_lambda_f @ Z0)...\
             ^[Z0:$i]:(tp_lambda_f @ Z0) Subterm 1\n# --TermPos\n"
        );
    }
}
