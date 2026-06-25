use crate::heuristics::hcb::{HeuristicParmsCell, UnifMode};
use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

static HO_CSU_PARAMS: RwLock<Option<HoCsuParams>> = RwLock::new(None);

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
    use super::HoCsuParams;
    use crate::heuristics::hcb::{HeuristicParmsCell, UnifMode};

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
