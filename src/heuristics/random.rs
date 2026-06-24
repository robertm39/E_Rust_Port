use crate::basics::simple_stuff::{jkiss_rand_double, RandState};
use crate::clauses::clause::Clause;

const RANDOM_X_DEFAULT: u32 = 684_291_357;
const RANDOM_Y_DEFAULT: u32 = 123_459_876;
const RANDOM_Z_DEFAULT: u32 = 918_273_645;
const RANDOM_C_DEFAULT: u32 = 129_834_675;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RandomWeightEvaluator {
    fifo_counter: f64,
    rand_range: u32,
    fifo_weight: f64,
    sc_weight: f64,
    rand_state: RandState,
}

impl RandomWeightEvaluator {
    #[must_use]
    pub fn new(
        rand_range: i64,
        fifo_weight: f64,
        sc_weight: f64,
        seed1: u32,
        seed2: u32,
        seed3: u32,
    ) -> Self {
        let xstate = if seed1 == 0 { RANDOM_X_DEFAULT } else { seed1 };
        let ystate = if seed2 == 0 { RANDOM_Y_DEFAULT } else { seed2 };
        let zstate = if seed3 == 0 { RANDOM_Z_DEFAULT } else { seed3 };
        Self {
            fifo_counter: 0.0,
            rand_range: c_long_to_uint(rand_range),
            fifo_weight,
            sc_weight,
            rand_state: RandState::new(xstate, ystate, zstate, RANDOM_C_DEFAULT),
        }
    }

    #[must_use]
    pub const fn fifo_counter(self) -> f64 {
        self.fifo_counter
    }

    #[must_use]
    pub const fn rand_range(self) -> u32 {
        self.rand_range
    }

    #[must_use]
    pub const fn fifo_weight(self) -> f64 {
        self.fifo_weight
    }

    #[must_use]
    pub const fn sc_weight(self) -> f64 {
        self.sc_weight
    }

    #[must_use]
    pub const fn rand_state(self) -> RandState {
        self.rand_state
    }

    pub fn compute(&mut self, clause: &Clause) -> f64 {
        rand_weight_compute(self, clause)
    }
}

#[must_use]
pub fn rand_weight_init(
    rand_range: i64,
    fifo_weight: f64,
    sc_weight: f64,
    seed1: u32,
    seed2: u32,
    seed3: u32,
) -> RandomWeightEvaluator {
    RandomWeightEvaluator::new(rand_range, fifo_weight, sc_weight, seed1, seed2, seed3)
}

pub fn rand_weight_compute(evaluator: &mut RandomWeightEvaluator, clause: &Clause) -> f64 {
    let sc = i64_to_f64(clause.standard_weight());
    let fifo = evaluator.fifo_counter;
    evaluator.fifo_counter += 1.0;
    let rnd = jkiss_rand_double(Some(&mut evaluator.rand_state));

    rnd * f64::from(evaluator.rand_range) + sc * evaluator.sc_weight + fifo * evaluator.fifo_weight
}

#[allow(clippy::cast_precision_loss)]
fn i64_to_f64(value: i64) -> f64 {
    value as f64
}

fn c_long_to_uint(value: i64) -> u32 {
    let modulus = i64::from(u32::MAX) + 1;
    let wrapped = value.rem_euclid(modulus);
    u32::try_from(wrapped).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests {
    use super::{
        rand_weight_compute, rand_weight_init, RandState, RANDOM_C_DEFAULT, RANDOM_X_DEFAULT,
        RANDOM_Y_DEFAULT, RANDOM_Z_DEFAULT,
    };
    use crate::clauses::clause::Clause;
    use crate::clauses::eqn::Eqn;
    use crate::clauses::eqnlist::EqnList;
    use crate::terms::signature::Signature;
    use crate::terms::termbanks::TermBank;
    use crate::terms::termtypes::{DerefType, Term};
    use crate::terms::typebanks::TypeBank;

    fn test_bank() -> TermBank {
        let mut signature = Signature::new(TypeBank::new());
        signature.insert_internal_codes().unwrap();
        TermBank::new(signature).unwrap()
    }

    fn typed_const(bank: &mut TermBank, name: &str) -> Term {
        let type_ = bank.signature().type_bank().default_type();
        let f_code = bank.signature_mut().insert_id(name, 0, false);
        bank.signature_mut()
            .declare_final_type(f_code, type_.clone())
            .unwrap();
        let term = Term::const_cell_alloc(f_code);
        term.set_type(Some(type_));
        bank.insert(&term, DerefType::Never).unwrap()
    }

    fn unit_clause() -> Clause {
        let mut bank = test_bank();
        let left = typed_const(&mut bank, "a");
        let right = typed_const(&mut bank, "b");
        let literal = Eqn::alloc(left, right, &mut bank, true).unwrap();
        Clause::alloc(EqnList::from_vec(vec![literal]))
    }

    fn assert_close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < f64::EPSILON,
            "expected {expected}, got {actual}"
        );
    }

    #[test]
    fn rand_weight_init_uses_c_defaults_and_nonzero_seed_overrides() {
        let defaulted = rand_weight_init(17, 2.5, 3.5, 0, 0, 0);
        assert_eq!(defaulted.rand_range(), 17);
        assert_close(defaulted.fifo_weight(), 2.5);
        assert_close(defaulted.sc_weight(), 3.5);
        assert_eq!(
            defaulted.rand_state(),
            RandState::new(
                RANDOM_X_DEFAULT,
                RANDOM_Y_DEFAULT,
                RANDOM_Z_DEFAULT,
                RANDOM_C_DEFAULT
            )
        );

        let seeded = rand_weight_init(0, 0.0, 0.0, 11, 0, 13);
        assert_eq!(
            seeded.rand_state(),
            RandState::new(11, RANDOM_Y_DEFAULT, 13, RANDOM_C_DEFAULT)
        );

        let wrapped = rand_weight_init(-1, 0.0, 0.0, 0, 0, 0);
        assert_eq!(wrapped.rand_range(), u32::MAX);
    }

    #[test]
    fn rand_weight_compute_uses_old_fifo_counter_then_increments() {
        let clause = unit_clause();
        let mut evaluator = rand_weight_init(0, 10.0, 2.0, 0, 0, 0);

        assert_eq!(clause.standard_weight(), 4);
        assert_close(rand_weight_compute(&mut evaluator, &clause), 8.0);
        assert_close(evaluator.compute(&clause), 18.0);
        assert_close(evaluator.fifo_counter(), 2.0);
    }
}
