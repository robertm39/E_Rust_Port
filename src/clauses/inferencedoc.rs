use crate::clauses::clause_props::{
    FormulaProperties, CP_TYPE_CONJECTURE, CP_TYPE_NEG_CONJECTURE, CP_TYPE_QUESTION,
};

#[must_use]
pub const fn pcl_type_str(type_: FormulaProperties) -> &'static str {
    match type_ {
        CP_TYPE_CONJECTURE => "conj",
        CP_TYPE_QUESTION => "que",
        CP_TYPE_NEG_CONJECTURE => "neg",
        _ => "",
    }
}

#[cfg(test)]
mod tests {
    use super::pcl_type_str;
    use crate::clauses::clause_props::{
        CP_TYPE_AXIOM, CP_TYPE_CONJECTURE, CP_TYPE_HYPOTHESIS, CP_TYPE_LEMMA,
        CP_TYPE_NEG_CONJECTURE, CP_TYPE_QUESTION, CP_TYPE_UNKNOWN, CP_TYPE_WATCH_CLAUSE,
    };

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
}
