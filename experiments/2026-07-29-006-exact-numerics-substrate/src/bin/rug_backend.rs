use rug::{Integer, Rational};
use umlaut_exact_numerics_study::{run_backend, RationalBackend};

struct RugBackend;

impl RationalBackend for RugBackend {
    type Rational = Rational;

    fn parse(numerator: &str, denominator: &str) -> Result<Self::Rational, String> {
        let numerator =
            Integer::from_str_radix(numerator, 10).map_err(|error| error.to_string())?;
        let denominator =
            Integer::from_str_radix(denominator, 10).map_err(|error| error.to_string())?;
        if denominator == 0 {
            return Err("zero denominator".to_owned());
        }
        Ok(Rational::from((numerator, denominator)))
    }

    fn add(left: &Self::Rational, right: &Self::Rational) -> Self::Rational {
        let mut result = left.clone();
        result += right;
        result
    }

    fn subtract(left: &Self::Rational, right: &Self::Rational) -> Self::Rational {
        let mut result = left.clone();
        result -= right;
        result
    }

    fn multiply(left: &Self::Rational, right: &Self::Rational) -> Self::Rational {
        let mut result = left.clone();
        result *= right;
        result
    }

    fn divide(left: &Self::Rational, right: &Self::Rational) -> Self::Rational {
        let mut result = left.clone();
        result /= right;
        result
    }

    fn floor(value: &Self::Rational) -> Self::Rational {
        value.clone().floor()
    }

    fn ceiling(value: &Self::Rational) -> Self::Rational {
        value.clone().ceil()
    }

    fn canonical_parts(value: &Self::Rational) -> (String, String) {
        (value.numer().to_string(), value.denom().to_string())
    }
}

fn main() {
    if let Err(error) = run_backend::<RugBackend>("rug-1.30.0-full-gmp-ffi") {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}
