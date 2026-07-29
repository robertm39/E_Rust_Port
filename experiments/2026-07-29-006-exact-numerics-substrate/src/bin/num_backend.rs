use num_bigint::BigInt;
use num_rational::BigRational;
use umlaut_exact_numerics_study::{run_backend, RationalBackend};

struct NumBackend;

impl RationalBackend for NumBackend {
    type Rational = BigRational;

    fn parse(numerator: &str, denominator: &str) -> Result<Self::Rational, String> {
        let numerator = numerator
            .parse::<BigInt>()
            .map_err(|error| error.to_string())?;
        let denominator = denominator
            .parse::<BigInt>()
            .map_err(|error| error.to_string())?;
        if denominator == BigInt::from(0) {
            return Err("zero denominator".to_owned());
        }
        Ok(BigRational::new(numerator, denominator))
    }

    fn add(left: &Self::Rational, right: &Self::Rational) -> Self::Rational {
        left + right
    }

    fn subtract(left: &Self::Rational, right: &Self::Rational) -> Self::Rational {
        left - right
    }

    fn multiply(left: &Self::Rational, right: &Self::Rational) -> Self::Rational {
        left * right
    }

    fn divide(left: &Self::Rational, right: &Self::Rational) -> Self::Rational {
        left / right
    }

    fn floor(value: &Self::Rational) -> Self::Rational {
        value.floor()
    }

    fn ceiling(value: &Self::Rational) -> Self::Rational {
        value.ceil()
    }

    fn canonical_parts(value: &Self::Rational) -> (String, String) {
        (value.numer().to_string(), value.denom().to_string())
    }
}

fn main() {
    if let Err(error) = run_backend::<NumBackend>("num-rational-0.4.2") {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}
