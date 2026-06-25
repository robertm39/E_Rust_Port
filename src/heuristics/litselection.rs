use crate::clauses::clause::Clause;
use crate::clauses::eqn_props::EP_IS_SELECTED;
use crate::orderings::ocb::OrderControlBlock;

pub const NO_SELECTION: &str = "NoSelection";
pub const NO_GENERATION: &str = "NoGeneration";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnsupportedLiteralSelection {
    strategy: String,
}

impl UnsupportedLiteralSelection {
    #[must_use]
    pub fn new(strategy: impl Into<String>) -> Self {
        Self {
            strategy: strategy.into(),
        }
    }

    #[must_use]
    pub fn strategy(&self) -> &str {
        &self.strategy
    }
}

impl std::fmt::Display for UnsupportedLiteralSelection {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "literal selection strategy '{}' is not ported yet",
            self.strategy
        )
    }
}

/// C `SelectNoLiterals`: assert that no literal is selected and otherwise do
/// nothing.
///
/// # Panics
///
/// Panics in debug builds if the caller has not already cleared selected
/// literal properties.
pub fn select_no_literals(_ocb: Option<&mut OrderControlBlock>, clause: &mut Clause) {
    debug_assert_eq!(clause.prop_lit_number(EP_IS_SELECTED), 0);
}

/// C `SelectNoGeneration`: same no-op body as `SelectNoLiterals`.
///
/// # Panics
///
/// Panics in debug builds if the caller has not already cleared selected
/// literal properties.
pub fn select_no_generation(_ocb: Option<&mut OrderControlBlock>, clause: &mut Clause) {
    debug_assert_eq!(clause.prop_lit_number(EP_IS_SELECTED), 0);
}

/// Applies the subset of literal-selection functions that has been ported.
///
/// # Errors
///
/// Returns `UnsupportedLiteralSelection` for valid C selector names whose
/// selector bodies have not been ported yet.
pub fn apply_ported_literal_selector(
    name: &str,
    ocb: Option<&mut OrderControlBlock>,
    clause: &mut Clause,
) -> Result<(), UnsupportedLiteralSelection> {
    match name {
        NO_SELECTION => {
            select_no_literals(ocb, clause);
            Ok(())
        }
        NO_GENERATION => {
            select_no_generation(ocb, clause);
            Ok(())
        }
        _ => Err(UnsupportedLiteralSelection::new(name)),
    }
}

#[cfg(test)]
mod tests {
    use super::{apply_ported_literal_selector, NO_GENERATION, NO_SELECTION};
    use crate::clauses::clause::Clause;
    use crate::clauses::eqn_props::EP_IS_SELECTED;

    #[test]
    fn no_selection_and_no_generation_are_noop_selectors() {
        let mut clause = Clause::empty();

        apply_ported_literal_selector(NO_SELECTION, None, &mut clause).unwrap_or_else(|err| {
            panic!("{err}");
        });
        apply_ported_literal_selector(NO_GENERATION, None, &mut clause).unwrap_or_else(|err| {
            panic!("{err}");
        });

        assert_eq!(clause.prop_lit_number(EP_IS_SELECTED), 0);
    }

    #[test]
    fn unported_selector_reports_name() {
        let mut clause = Clause::empty();
        let error =
            apply_ported_literal_selector("SelectNegativeLiterals", None, &mut clause).unwrap_err();

        assert_eq!(error.strategy(), "SelectNegativeLiterals");
        assert!(error.to_string().contains("not ported yet"));
    }
}
