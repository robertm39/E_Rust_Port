use dashu_ratio::RBig;
use umlaut_exact_numerics_study::{run_backend, RationalBackend};

struct DashuBackend;

impl RationalBackend for DashuBackend {
    type Rational = RBig;

    fn parse(numerator: &str, denominator: &str) -> Result<Self::Rational, String> {
        format!("{numerator}/{denominator}")
            .parse::<RBig>()
            .map_err(|error| error.to_string())
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
        value.floor().into()
    }

    fn ceiling(value: &Self::Rational) -> Self::Rational {
        value.ceil().into()
    }

    fn canonical_parts(value: &Self::Rational) -> (String, String) {
        (
            value.numerator().to_string(),
            value.denominator().to_string(),
        )
    }
}

fn main() {
    if let Err(error) = run_backend::<DashuBackend>("dashu-ratio-0.5.1") {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}
