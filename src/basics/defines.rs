pub type IntOrPInt = i64;

pub const DEFAULT_COMCHAR_RAW: &str = "%";
pub const DEFAULT_COMCHAR_DIRECT: &str = "%%";

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IntOrP<P> {
    Int(IntOrPInt),
    Pointer(P),
}

impl<P> IntOrP<P> {
    #[must_use]
    pub const fn int(value: IntOrPInt) -> Self {
        Self::Int(value)
    }

    #[must_use]
    pub const fn pointer(value: P) -> Self {
        Self::Pointer(value)
    }

    #[must_use]
    pub const fn as_int(&self) -> Option<IntOrPInt> {
        match self {
            Self::Int(value) => Some(*value),
            Self::Pointer(_) => None,
        }
    }

    #[must_use]
    pub const fn as_pointer(&self) -> Option<&P> {
        match self {
            Self::Int(_) => None,
            Self::Pointer(value) => Some(value),
        }
    }

    pub fn into_int(self) -> Option<IntOrPInt> {
        match self {
            Self::Int(value) => Some(value),
            Self::Pointer(_) => None,
        }
    }

    pub fn into_pointer(self) -> Option<P> {
        match self {
            Self::Int(_) => None,
            Self::Pointer(value) => Some(value),
        }
    }
}
