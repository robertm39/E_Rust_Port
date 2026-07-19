use crate::basics::numtrees::NumTree;

pub type NormSubstTree = NumTree<i64, i64>;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct NormSubstCell {
    pub used_ids: Option<NormSubstTree>,
    pub norm_funs: Option<NormSubstTree>,
    pub norm_vars: Option<NormSubstTree>,
}

impl NormSubstCell {
    #[must_use]
    pub const fn alloc() -> Self {
        Self {
            used_ids: None,
            norm_funs: None,
            norm_vars: None,
        }
    }
}

#[must_use]
pub const fn norm_subst_alloc() -> NormSubstCell {
    NormSubstCell::alloc()
}

pub fn norm_subst_free(_junk: NormSubstCell) {}

#[cfg(test)]
mod tests {
    use super::{norm_subst_alloc, norm_subst_free, NormSubstCell, NormSubstTree};

    #[test]
    fn allocation_starts_with_null_tree_roots_like_c() {
        let subst = norm_subst_alloc();

        assert_eq!(subst, NormSubstCell::alloc());
        assert!(subst.used_ids.is_none());
        assert!(subst.norm_funs.is_none());
        assert!(subst.norm_vars.is_none());
    }

    #[test]
    fn tree_fields_can_hold_numeric_maps_until_consumed_by_free() {
        let mut subst = norm_subst_alloc();
        let mut used = NormSubstTree::new();
        assert!(used.store(7, 11, 13));
        subst.used_ids = Some(used);

        assert_eq!(
            subst
                .used_ids
                .as_ref()
                .and_then(|tree| tree.find_binary(7))
                .unwrap()
                .val1,
            11
        );
        norm_subst_free(subst);
    }
}
