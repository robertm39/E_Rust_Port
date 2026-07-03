use crate::heuristics::hcb::{HeuristicParmsCell, UnifMode};
use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

static HO_CSU_PARAMS: RwLock<Option<HoCsuParams>> = RwLock::new(None);

pub type StateTag = u64;
pub type Limits = u64;

pub const INIT_TAG: StateTag = 0;
pub const RIGID_PROCESSED_TAG: StateTag = 1;
pub const SOLVED_BY_ORACLE_TAG: StateTag = 2;
pub const DECOMPOSED_VAR: StateTag = 3;

pub const BT_STEP_SIZE: usize = 4;
pub const BURY_KIND: i32 = 0;
pub const STORE_KIND: i32 = 1;

/// Mirrors C `CONSTRAINT_STATE(c)`.
#[must_use]
pub const fn constraint_state(constraint: StateTag) -> StateTag {
    constraint & 3
}

/// Mirrors C `CONSTRAINT_COUNTER(c)`.
#[must_use]
pub const fn constraint_counter(constraint: StateTag) -> StateTag {
    constraint >> 2
}

/// Mirrors C `BUILD_CONSTR(c, s)`.
#[must_use]
pub const fn build_constraint(counter: StateTag, state: StateTag) -> StateTag {
    (counter << 2) | state
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HoCsuParams {
    pub func_proj_limit: i32,
    pub imit_limit: i32,
    pub ident_limit: i32,
    pub elim_limit: i32,
    pub unif_mode: UnifMode,
    pub pattern_oracle: bool,
    pub fixpoint_oracle: bool,
    pub max_unifiers: i32,
    pub max_unif_steps: i32,
}

impl HoCsuParams {
    #[must_use]
    pub const fn from_heuristic_parms(parms: &HeuristicParmsCell) -> Self {
        Self {
            func_proj_limit: parms.func_proj_limit,
            imit_limit: parms.imit_limit,
            ident_limit: parms.ident_limit,
            elim_limit: parms.elim_limit,
            unif_mode: parms.unif_mode,
            pattern_oracle: parms.pattern_oracle,
            fixpoint_oracle: parms.fixpoint_oracle,
            max_unifiers: parms.max_unifiers,
            max_unif_steps: parms.max_unif_steps,
        }
    }
}

pub fn init_unif_limits(parms: &HeuristicParmsCell) {
    *write_params() = Some(HoCsuParams::from_heuristic_parms(parms));
}

#[must_use]
pub fn current_unif_limits() -> Option<HoCsuParams> {
    *read_params()
}

fn read_params() -> RwLockReadGuard<'static, Option<HoCsuParams>> {
    match HO_CSU_PARAMS.read() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

fn write_params() -> RwLockWriteGuard<'static, Option<HoCsuParams>> {
    match HO_CSU_PARAMS.write() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        build_constraint, constraint_counter, constraint_state, HoCsuParams, BT_STEP_SIZE,
        BURY_KIND, DECOMPOSED_VAR, INIT_TAG, RIGID_PROCESSED_TAG, SOLVED_BY_ORACLE_TAG, STORE_KIND,
    };
    use crate::heuristics::hcb::{HeuristicParmsCell, UnifMode};

    #[test]
    fn state_tag_values_match_c_header() {
        assert_eq!(INIT_TAG, 0);
        assert_eq!(RIGID_PROCESSED_TAG, 1);
        assert_eq!(SOLVED_BY_ORACLE_TAG, 2);
        assert_eq!(DECOMPOSED_VAR, 3);
        assert_eq!(BT_STEP_SIZE, 4);
        assert_eq!(BURY_KIND, 0);
        assert_eq!(STORE_KIND, 1);
    }

    #[test]
    fn constraint_bit_packing_matches_c_macros() {
        let encoded = build_constraint(17, DECOMPOSED_VAR);
        assert_eq!(encoded, (17 << 2) | 3);
        assert_eq!(constraint_state(encoded), DECOMPOSED_VAR);
        assert_eq!(constraint_counter(encoded), 17);
    }

    #[test]
    fn constraint_build_does_not_mask_state_like_c_macro() {
        let encoded = build_constraint(0, 4);
        assert_eq!(encoded, 4);
        assert_eq!(constraint_state(encoded), INIT_TAG);
        assert_eq!(constraint_counter(encoded), 1);
    }

    #[test]
    fn ho_csu_params_snapshot_keeps_fields_read_by_c_csu_helpers() {
        let parms = HeuristicParmsCell {
            func_proj_limit: 1,
            imit_limit: 2,
            ident_limit: 3,
            elim_limit: 4,
            unif_mode: UnifMode::Multi,
            pattern_oracle: false,
            fixpoint_oracle: false,
            max_unifiers: 8,
            max_unif_steps: 9,
            ..HeuristicParmsCell::default()
        };

        assert_eq!(
            HoCsuParams::from_heuristic_parms(&parms),
            HoCsuParams {
                func_proj_limit: 1,
                imit_limit: 2,
                ident_limit: 3,
                elim_limit: 4,
                unif_mode: UnifMode::Multi,
                pattern_oracle: false,
                fixpoint_oracle: false,
                max_unifiers: 8,
                max_unif_steps: 9,
            }
        );
    }
}
