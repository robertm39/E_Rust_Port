#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IndexType(i32);

impl IndexType {
    pub const NO_INDEX: Self = Self(0);
    pub const ARITY: Self = Self(1);
    pub const SYMBOL: Self = Self(2);
    pub const TOP: Self = Self(4);
    pub const ALT_TOP: Self = Self(8);
    pub const CS_TOP: Self = Self(16);
    pub const ES_TOP: Self = Self(32);
    pub const IDENTITY: Self = Self(64);
    pub const EMPTY: Self = Self(128);
    pub const DYNAMIC: Self = Self(
        Self::ARITY.0
            | Self::SYMBOL.0
            | Self::TOP.0
            | Self::ALT_TOP.0
            | Self::CS_TOP.0
            | Self::ES_TOP.0
            | Self::IDENTITY.0,
    );

    #[must_use]
    pub const fn bits(self) -> i32 {
        self.0
    }
}

pub const INDEX_DYNAMIC_DEPTH: i32 = 0;

pub const INDEX_FUN_NAMES: [&str; 10] = [
    "IndexNoIndex",
    "IndexDynamic",
    "IndexArity",
    "IndexSymbol",
    "IndexTop",
    "IndexAltTop",
    "IndexCSTop",
    "IndexESTop",
    "IndexIdentity",
    "IndexEmpty",
];

#[must_use]
pub fn get_index_type(name: &str) -> Option<IndexType> {
    let position = INDEX_FUN_NAMES
        .iter()
        .position(|candidate| *candidate == name)?;
    if position == 0 {
        return Some(IndexType::NO_INDEX);
    }
    if position == 1 {
        return Some(IndexType::DYNAMIC);
    }
    Some(IndexType(1 << (position - 2)))
}

#[must_use]
pub fn get_index_name(index_type: IndexType) -> &'static str {
    match index_type {
        IndexType::NO_INDEX => INDEX_FUN_NAMES[0],
        IndexType::ARITY => INDEX_FUN_NAMES[2],
        IndexType::SYMBOL => INDEX_FUN_NAMES[3],
        IndexType::TOP => INDEX_FUN_NAMES[4],
        IndexType::ALT_TOP => INDEX_FUN_NAMES[5],
        IndexType::CS_TOP => INDEX_FUN_NAMES[6],
        IndexType::ES_TOP => INDEX_FUN_NAMES[7],
        IndexType::IDENTITY => INDEX_FUN_NAMES[8],
        IndexType::EMPTY => INDEX_FUN_NAMES[9],
        _ => INDEX_FUN_NAMES[1],
    }
}

#[cfg(test)]
mod tests {
    use super::{get_index_name, get_index_type, IndexType, INDEX_DYNAMIC_DEPTH, INDEX_FUN_NAMES};

    #[test]
    fn index_type_names_and_values_match_c_surface() {
        assert_eq!(INDEX_FUN_NAMES[0], "IndexNoIndex");
        assert_eq!(INDEX_FUN_NAMES[1], "IndexDynamic");
        assert_eq!(INDEX_DYNAMIC_DEPTH, 0);
        assert_eq!(IndexType::NO_INDEX.bits(), 0);
        assert_eq!(IndexType::ARITY.bits(), 1);
        assert_eq!(IndexType::SYMBOL.bits(), 2);
        assert_eq!(IndexType::TOP.bits(), 4);
        assert_eq!(IndexType::ALT_TOP.bits(), 8);
        assert_eq!(IndexType::CS_TOP.bits(), 16);
        assert_eq!(IndexType::ES_TOP.bits(), 32);
        assert_eq!(IndexType::IDENTITY.bits(), 64);
        assert_eq!(IndexType::EMPTY.bits(), 128);
    }

    #[test]
    fn get_index_type_expands_dynamic_like_c_helper() {
        assert_eq!(get_index_type("IndexNoIndex"), Some(IndexType::NO_INDEX));
        assert_eq!(get_index_type("IndexArity"), Some(IndexType::ARITY));
        assert_eq!(get_index_type("IndexEmpty"), Some(IndexType::EMPTY));
        assert_eq!(get_index_type("missing"), None);
        assert_eq!(
            get_index_type("IndexDynamic").map(IndexType::bits),
            Some(
                IndexType::ARITY.bits()
                    | IndexType::SYMBOL.bits()
                    | IndexType::TOP.bits()
                    | IndexType::ALT_TOP.bits()
                    | IndexType::CS_TOP.bits()
                    | IndexType::ES_TOP.bits()
                    | IndexType::IDENTITY.bits()
            )
        );
    }

    #[test]
    fn get_index_name_maps_composites_to_dynamic_name() {
        assert_eq!(get_index_name(IndexType::NO_INDEX), "IndexNoIndex");
        assert_eq!(get_index_name(IndexType::TOP), "IndexTop");
        assert_eq!(get_index_name(IndexType::DYNAMIC), "IndexDynamic");
    }
}
