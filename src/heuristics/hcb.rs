pub const NO_EXT_SUP: i32 = -1;
pub const NO_ELIM_LEIBNIZ: i32 = -1;

pub const HCB_DEFAULT_HEURISTIC: &str = "Default";
pub const DEFAULT_SYM_OCCS: i64 = 512;
pub const DEFAULT_MINISCOPE_LIMIT: i64 = 1_048_576;
pub const DEFAULT_FILTER_ORPHANS_LIMIT: i64 = i64::MAX;
pub const DEFAULT_FORWARD_CONTRACT_LIMIT: i64 = i64::MAX;
pub const DEFAULT_DELETE_BAD_LIMIT: i64 = i64::MAX;
pub const DEFAULT_RW_BW_INDEX_NAME: &str = "FP7";
pub const DEFAULT_PM_FROM_INDEX_NAME: &str = "FP7";
pub const DEFAULT_PM_INTO_INDEX_NAME: &str = "FP7";

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[repr(i32)]
pub enum AcHandling {
    None = 0,
    #[default]
    DiscardAll = 1,
    KeepUnits = 2,
    KeepOrientable = 3,
}

impl AcHandling {
    #[must_use]
    pub const fn from_c_value(value: i32) -> Option<Self> {
        match value {
            0 => Some(Self::None),
            1 => Some(Self::DiscardAll),
            2 => Some(Self::KeepUnits),
            3 => Some(Self::KeepOrientable),
            _ => None,
        }
    }

    #[must_use]
    pub const fn c_value(self) -> i32 {
        self as i32
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[repr(i32)]
pub enum ExtInferenceType {
    AllLits = 0,
    MaxLits = 1,
    #[default]
    NoLits = 2,
}

impl ExtInferenceType {
    #[must_use]
    pub const fn from_c_value(value: i32) -> Option<Self> {
        match value {
            0 => Some(Self::AllLits),
            1 => Some(Self::MaxLits),
            2 => Some(Self::NoLits),
            _ => None,
        }
    }

    #[must_use]
    pub const fn c_value(self) -> i32 {
        self as i32
    }

    #[must_use]
    pub const fn name(self) -> &'static str {
        ext_inference_type_name_raw(self.c_value())
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[repr(i32)]
pub enum PrimEnumMode {
    Neg = 0,
    And = 1,
    Or = 2,
    Eq = 3,
    #[default]
    Pragmatic = 4,
    Full = 5,
    LogSymbol = 6,
}

impl PrimEnumMode {
    #[must_use]
    pub const fn from_c_value(value: i32) -> Option<Self> {
        match value {
            0 => Some(Self::Neg),
            1 => Some(Self::And),
            2 => Some(Self::Or),
            3 => Some(Self::Eq),
            4 => Some(Self::Pragmatic),
            5 => Some(Self::Full),
            6 => Some(Self::LogSymbol),
            _ => None,
        }
    }

    #[must_use]
    pub const fn c_value(self) -> i32 {
        self as i32
    }

    #[must_use]
    pub const fn name(self) -> &'static str {
        prim_enum_mode_name_raw(self.c_value())
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[repr(i32)]
pub enum UnifMode {
    #[default]
    Single = 0,
    Multi = 1,
}

impl UnifMode {
    #[must_use]
    pub const fn from_c_value(value: i32) -> Option<Self> {
        match value {
            0 => Some(Self::Single),
            1 => Some(Self::Multi),
            _ => None,
        }
    }

    #[must_use]
    pub const fn c_value(self) -> i32 {
        self as i32
    }

    #[must_use]
    pub const fn name(self) -> &'static str {
        unif_mode_name_raw(self.c_value())
    }
}

#[must_use]
pub const fn ext_inference_type_name_raw(value: i32) -> &'static str {
    match value {
        0 => "all",
        1 => "max",
        _ => "off",
    }
}

#[must_use]
pub const fn prim_enum_mode_name_raw(value: i32) -> &'static str {
    match value {
        0 => "neg",
        1 => "and",
        2 => "or",
        3 => "eq",
        4 => "pragmatic",
        5 => "full",
        6 => "logsymbol",
        _ => "unknown",
    }
}

#[must_use]
pub const fn unif_mode_name_raw(value: i32) -> &'static str {
    match value {
        0 => "single",
        _ => "multi",
    }
}

#[must_use]
pub fn str_to_ext_inference_type(value: &str) -> Option<ExtInferenceType> {
    match value {
        "all" => Some(ExtInferenceType::AllLits),
        "max" => Some(ExtInferenceType::MaxLits),
        "off" => Some(ExtInferenceType::NoLits),
        _ => None,
    }
}

#[must_use]
pub fn str_to_prim_enum_mode_raw(value: &str) -> i32 {
    match value {
        "neg" => PrimEnumMode::Neg.c_value(),
        "and" => PrimEnumMode::And.c_value(),
        "or" => PrimEnumMode::Or.c_value(),
        "eq" => PrimEnumMode::Eq.c_value(),
        "pragmatic" => PrimEnumMode::Pragmatic.c_value(),
        "full" => PrimEnumMode::Full.c_value(),
        "logsymbol" => PrimEnumMode::LogSymbol.c_value(),
        _ => -1,
    }
}

#[must_use]
pub fn str_to_prim_enum_mode(value: &str) -> Option<PrimEnumMode> {
    PrimEnumMode::from_c_value(str_to_prim_enum_mode_raw(value))
}

#[must_use]
pub fn str_to_unif_mode_raw(value: &str) -> i32 {
    match value {
        "single" => UnifMode::Single.c_value(),
        "multi" => UnifMode::Multi.c_value(),
        _ => -1,
    }
}

#[must_use]
pub fn str_to_unif_mode(value: &str) -> Option<UnifMode> {
    UnifMode::from_c_value(str_to_unif_mode_raw(value))
}

#[cfg(test)]
mod tests {
    use super::{
        ext_inference_type_name_raw, prim_enum_mode_name_raw, str_to_ext_inference_type,
        str_to_prim_enum_mode, str_to_prim_enum_mode_raw, str_to_unif_mode, str_to_unif_mode_raw,
        unif_mode_name_raw, AcHandling, ExtInferenceType, PrimEnumMode, UnifMode,
        DEFAULT_DELETE_BAD_LIMIT, DEFAULT_FILTER_ORPHANS_LIMIT, DEFAULT_FORWARD_CONTRACT_LIMIT,
        DEFAULT_MINISCOPE_LIMIT, DEFAULT_PM_FROM_INDEX_NAME, DEFAULT_PM_INTO_INDEX_NAME,
        DEFAULT_RW_BW_INDEX_NAME, DEFAULT_SYM_OCCS, HCB_DEFAULT_HEURISTIC, NO_ELIM_LEIBNIZ,
        NO_EXT_SUP,
    };

    #[test]
    fn hcb_default_constants_match_c_defines() {
        assert_eq!(NO_EXT_SUP, -1);
        assert_eq!(NO_ELIM_LEIBNIZ, -1);
        assert_eq!(HCB_DEFAULT_HEURISTIC, "Default");
        assert_eq!(DEFAULT_SYM_OCCS, 512);
        assert_eq!(DEFAULT_MINISCOPE_LIMIT, 1_048_576);
        assert_eq!(DEFAULT_FILTER_ORPHANS_LIMIT, i64::MAX);
        assert_eq!(DEFAULT_FORWARD_CONTRACT_LIMIT, i64::MAX);
        assert_eq!(DEFAULT_DELETE_BAD_LIMIT, i64::MAX);
        assert_eq!(DEFAULT_RW_BW_INDEX_NAME, "FP7");
        assert_eq!(DEFAULT_PM_FROM_INDEX_NAME, "FP7");
        assert_eq!(DEFAULT_PM_INTO_INDEX_NAME, "FP7");
    }

    #[test]
    fn enum_discriminants_match_c_declaration_order() {
        assert_eq!(AcHandling::None.c_value(), 0);
        assert_eq!(AcHandling::DiscardAll.c_value(), 1);
        assert_eq!(AcHandling::KeepUnits.c_value(), 2);
        assert_eq!(AcHandling::KeepOrientable.c_value(), 3);

        assert_eq!(ExtInferenceType::AllLits.c_value(), 0);
        assert_eq!(ExtInferenceType::MaxLits.c_value(), 1);
        assert_eq!(ExtInferenceType::NoLits.c_value(), 2);

        assert_eq!(PrimEnumMode::Neg.c_value(), 0);
        assert_eq!(PrimEnumMode::And.c_value(), 1);
        assert_eq!(PrimEnumMode::Or.c_value(), 2);
        assert_eq!(PrimEnumMode::Eq.c_value(), 3);
        assert_eq!(PrimEnumMode::Pragmatic.c_value(), 4);
        assert_eq!(PrimEnumMode::Full.c_value(), 5);
        assert_eq!(PrimEnumMode::LogSymbol.c_value(), 6);

        assert_eq!(UnifMode::Single.c_value(), 0);
        assert_eq!(UnifMode::Multi.c_value(), 1);
    }

    #[test]
    fn from_c_value_rejects_unknown_discriminants() {
        assert_eq!(AcHandling::from_c_value(4), None);
        assert_eq!(ExtInferenceType::from_c_value(-1), None);
        assert_eq!(PrimEnumMode::from_c_value(7), None);
        assert_eq!(UnifMode::from_c_value(2), None);
    }

    #[test]
    fn raw_name_helpers_preserve_macro_fallbacks() {
        assert_eq!(ExtInferenceType::AllLits.name(), "all");
        assert_eq!(ExtInferenceType::MaxLits.name(), "max");
        assert_eq!(ExtInferenceType::NoLits.name(), "off");
        assert_eq!(ext_inference_type_name_raw(-1), "off");
        assert_eq!(ext_inference_type_name_raw(99), "off");

        assert_eq!(PrimEnumMode::LogSymbol.name(), "logsymbol");
        assert_eq!(prim_enum_mode_name_raw(-1), "unknown");
        assert_eq!(prim_enum_mode_name_raw(99), "unknown");

        assert_eq!(UnifMode::Single.name(), "single");
        assert_eq!(UnifMode::Multi.name(), "multi");
        assert_eq!(unif_mode_name_raw(-1), "multi");
        assert_eq!(unif_mode_name_raw(99), "multi");
    }

    #[test]
    fn hcb_strategy_parser_names_match_c_spellings() {
        assert_eq!(
            str_to_ext_inference_type("all"),
            Some(ExtInferenceType::AllLits)
        );
        assert_eq!(
            str_to_ext_inference_type("max"),
            Some(ExtInferenceType::MaxLits)
        );
        assert_eq!(
            str_to_ext_inference_type("off"),
            Some(ExtInferenceType::NoLits)
        );
        assert_eq!(str_to_ext_inference_type("none"), None);

        assert_eq!(str_to_prim_enum_mode_raw("logsymbol"), 6);
        assert_eq!(str_to_prim_enum_mode_raw("logsym"), -1);
        assert_eq!(
            str_to_prim_enum_mode("logsymbol"),
            Some(PrimEnumMode::LogSymbol)
        );
        assert_eq!(str_to_prim_enum_mode("logsym"), None);

        assert_eq!(str_to_unif_mode_raw("single"), 0);
        assert_eq!(str_to_unif_mode_raw("multi"), 1);
        assert_eq!(str_to_unif_mode_raw("many"), -1);
        assert_eq!(str_to_unif_mode("single"), Some(UnifMode::Single));
        assert_eq!(str_to_unif_mode("multi"), Some(UnifMode::Multi));
        assert_eq!(str_to_unif_mode("many"), None);
    }
}
