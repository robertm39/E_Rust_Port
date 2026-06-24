#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i32)]
pub enum TsmType {
    NoType = 0,
    Flat = 1,
    Recursive = 2,
    Recurrent = 3,
    RecurrentLocal = 4,
}

pub const TSM_TYPE_NAMES: [&str; 5] = ["NoType", "Flat", "Recursive", "Recurrent", "RecLocal"];

#[must_use]
pub fn get_tsm_type(name: &str) -> Option<TsmType> {
    match TSM_TYPE_NAMES
        .iter()
        .position(|candidate| *candidate == name)?
    {
        0 => Some(TsmType::NoType),
        1 => Some(TsmType::Flat),
        2 => Some(TsmType::Recursive),
        3 => Some(TsmType::Recurrent),
        4 => Some(TsmType::RecurrentLocal),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{get_tsm_type, TsmType, TSM_TYPE_NAMES};

    #[test]
    fn tsm_type_names_and_discriminants_match_c_surface() {
        assert_eq!(
            TSM_TYPE_NAMES,
            ["NoType", "Flat", "Recursive", "Recurrent", "RecLocal"]
        );
        assert_eq!(TsmType::NoType as i32, 0);
        assert_eq!(TsmType::Flat as i32, 1);
        assert_eq!(TsmType::Recursive as i32, 2);
        assert_eq!(TsmType::Recurrent as i32, 3);
        assert_eq!(TsmType::RecurrentLocal as i32, 4);
        assert_eq!(get_tsm_type("Flat"), Some(TsmType::Flat));
        assert_eq!(get_tsm_type("RecLocal"), Some(TsmType::RecurrentLocal));
        assert_eq!(get_tsm_type("missing"), None);
    }
}
