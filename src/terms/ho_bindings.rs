use crate::terms::ho_csu::Limits;

pub const IMIT_MASK: Limits = 63;
pub const PROJ_MASK: Limits = IMIT_MASK << 6;
pub const IDENT_MASK: Limits = PROJ_MASK << 6;
pub const ELIM_MASK: Limits = IDENT_MASK << 6;

/// Mirrors C `GET_IMIT(c)`.
#[must_use]
pub const fn imitation_count(limits: Limits) -> Limits {
    limits & IMIT_MASK
}

/// Mirrors C `GET_PROJ(c)`.
#[must_use]
pub const fn projection_count(limits: Limits) -> Limits {
    (limits & PROJ_MASK) >> 6
}

/// Mirrors C `GET_IDENT(c)`.
#[must_use]
pub const fn identification_count(limits: Limits) -> Limits {
    (limits & IDENT_MASK) >> 12
}

/// Mirrors C `GET_ELIM(c)`.
#[must_use]
pub const fn elimination_count(limits: Limits) -> Limits {
    (limits & ELIM_MASK) >> 18
}

/// Mirrors C `INC_IMIT(c)`.
#[must_use]
pub const fn inc_imitation(limits: Limits) -> Limits {
    (imitation_count(limits) + 1) | (!IMIT_MASK & limits)
}

/// Mirrors C `INC_PROJ(c)`.
#[must_use]
pub const fn inc_projection(limits: Limits) -> Limits {
    ((projection_count(limits) + 1) << 6) | (!PROJ_MASK & limits)
}

/// Mirrors C `INC_IDENT(c)`.
#[must_use]
pub const fn inc_identification(limits: Limits) -> Limits {
    ((identification_count(limits) + 1) << 12) | (!IDENT_MASK & limits)
}

/// Mirrors C `INC_ELIM(c)`.
#[must_use]
pub const fn inc_elimination(limits: Limits) -> Limits {
    ((elimination_count(limits) + 1) << 18) | (!ELIM_MASK & limits)
}

#[cfg(test)]
mod tests {
    use super::{
        elimination_count, identification_count, imitation_count, inc_elimination,
        inc_identification, inc_imitation, inc_projection, projection_count, ELIM_MASK, IDENT_MASK,
        IMIT_MASK, PROJ_MASK,
    };

    #[test]
    fn limit_masks_match_c_layout() {
        assert_eq!(IMIT_MASK, 63);
        assert_eq!(PROJ_MASK, 63 << 6);
        assert_eq!(IDENT_MASK, 63 << 12);
        assert_eq!(ELIM_MASK, 63 << 18);
    }

    #[test]
    fn limit_accessors_read_c_bit_fields() {
        let limits = 5 | (6 << 6) | (7 << 12) | (8 << 18);
        assert_eq!(imitation_count(limits), 5);
        assert_eq!(projection_count(limits), 6);
        assert_eq!(identification_count(limits), 7);
        assert_eq!(elimination_count(limits), 8);
    }

    #[test]
    fn limit_incrementers_preserve_other_fields() {
        let limits = 5 | (6 << 6) | (7 << 12) | (8 << 18);
        assert_eq!(imitation_count(inc_imitation(limits)), 6);
        assert_eq!(projection_count(inc_imitation(limits)), 6);

        assert_eq!(projection_count(inc_projection(limits)), 7);
        assert_eq!(identification_count(inc_projection(limits)), 7);

        assert_eq!(identification_count(inc_identification(limits)), 8);
        assert_eq!(elimination_count(inc_identification(limits)), 8);

        assert_eq!(elimination_count(inc_elimination(limits)), 9);
        assert_eq!(imitation_count(inc_elimination(limits)), 5);
    }

    #[test]
    fn limit_incrementers_do_not_mask_overflow_like_c_macros() {
        let carried = inc_imitation(IMIT_MASK);
        assert_eq!(imitation_count(carried), 0);
        assert_eq!(projection_count(carried), 1);
    }
}
