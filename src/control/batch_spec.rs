use crate::basics::error::{Diagnostic, ErrorCode};
use crate::inout::basicparser::{
    accept_dotted_id, parse_basic_include, parse_continuous, parse_dotted_id, parse_filename,
    parse_int,
};
use crate::inout::scanner::{token_pos_rep, IoFormat, Scanner, TokenType};
use std::io::{self, Write};

pub const BATCH_FILTERS: &[&str] = &[
    "threshold010000",
    "gf600_h_gu_R05_F100_L20000",
    "gf120_h_gu_R02_F100_L20000",
    "gf200_gu_RUU_F100_L20000",
    "gf200_h_gu_R03_F100_L20000",
    "gf120_h_gu_RUU_F100_L00100",
    "gf500_h_gu_R04_F100_L20000",
    "gf150_gu_RUU_F100_L20000",
    "gf120_h_gu_RUU_F100_L00500",
    "gf120_gu_RUU_F100_L01000",
    "gf120_gu_R02_F100_L20000",
    "gf500_gu_R04_F100_L20000",
    "gf600_gu_R05_F100_L20000",
];

pub const BATCH_STRATEGIES: &[&str] = &[
    "--satauto-schedule --assume-incompleteness",
    "--satauto-schedule --assume-incompleteness",
    "--satauto-schedule --assume-incompleteness",
    "--satauto-schedule --assume-incompleteness",
    "--satauto-schedule --assume-incompleteness",
    "--satauto-schedule --assume-incompleteness",
    "--satauto-schedule --assume-incompleteness",
    "--satauto-schedule --assume-incompleteness",
    "--satauto-schedule --assume-incompleteness",
    "--satauto-schedule --assume-incompleteness",
    "--satauto-schedule --assume-incompleteness",
    "--satauto-schedule --assume-incompleteness",
    "--satauto-schedule --assume-incompleteness",
];

pub const BATCH_FILTERS_DIV: &[&str] = &[
    "threshold010000",
    "gf600_h_gu_R05_F100_L20000",
    "gf120_h_gu_R02_F100_L20000",
    "gf200_gu_RUU_F100_L20000",
    "gf200_h_gu_R03_F100_L20000",
    "gf120_h_gu_RUU_F100_L00100",
    "gf500_h_gu_R04_F100_L20000",
    "gf150_gu_RUU_F100_L20000",
    "gf120_h_gu_RUU_F100_L00500",
    "gf120_gu_RUU_F100_L01000",
    "gf120_gu_R02_F100_L20000",
    "gf500_gu_R04_F100_L20000",
    "gf600_gu_R05_F100_L20000",
    "gf600_h_gu_R05_F100_L20000",
    "gf600_h_gu_R05_F100_L20000",
    "gf600_h_gu_R05_F100_L20000",
    "gf600_h_gu_R05_F100_L20000",
];

pub const BATCH_STRATEGIES_DIV: &[&str] = &[
    "--auto-schedule --assume-incompleteness",
    "--satauto-schedule --assume-incompleteness",
    "--satauto-schedule --assume-incompleteness",
    "--satauto-schedule --assume-incompleteness",
    "--satauto-schedule --assume-incompleteness",
    "--satauto-schedule --assume-incompleteness",
    "--satauto-schedule --assume-incompleteness",
    "--satauto-schedule --assume-incompleteness",
    "--satauto-schedule --assume-incompleteness",
    "--satauto-schedule --assume-incompleteness",
    "--satauto-schedule --assume-incompleteness",
    "--satauto-schedule --assume-incompleteness",
    "--satauto-schedule --assume-incompleteness",
    "-xAutoSched2 -tAutoSched2 --assume-incompleteness",
    "-xAutoSched3 -tAutoSched3 --assume-incompleteness",
    "-xAutoSched4 -tAutoSched4 --assume-incompleteness",
    "-xAutoSched5 -tAutoSched5 --assume-incompleteness",
];

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[repr(i32)]
pub enum BatchOutputType {
    #[default]
    NoOutput = 0,
    Desired = 1,
    Required = 2,
}

impl BatchOutputType {
    #[must_use]
    pub const fn c_value(self) -> i32 {
        self as i32
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BatchSpecHeader {
    pub category: String,
    pub train_dir: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BatchSpec {
    pub executable: String,
    pub format: IoFormat,
    pub category: Option<String>,
    pub train_dir: Option<String>,
    pub ordered: bool,
    pub res_assurance: BatchOutputType,
    pub res_proof: BatchOutputType,
    pub res_model: BatchOutputType,
    pub res_answer: BatchOutputType,
    pub res_list_fof: BatchOutputType,
    pub per_prob_limit: i64,
    pub total_wtc_limit: i64,
    pub includes: Vec<String>,
    pub source_files: Vec<String>,
    pub dest_files: Vec<String>,
}

impl BatchSpec {
    #[must_use]
    pub fn new(executable: impl Into<String>, format: IoFormat) -> Self {
        Self {
            executable: executable.into(),
            format,
            category: None,
            train_dir: None,
            ordered: false,
            res_assurance: BatchOutputType::NoOutput,
            res_proof: BatchOutputType::NoOutput,
            res_model: BatchOutputType::NoOutput,
            res_answer: BatchOutputType::NoOutput,
            res_list_fof: BatchOutputType::NoOutput,
            per_prob_limit: 0,
            total_wtc_limit: 0,
            includes: Vec::new(),
            source_files: Vec::new(),
            dest_files: Vec::new(),
        }
    }

    pub fn parse(
        scanner: &mut Scanner,
        executable: impl Into<String>,
        category: &str,
        train_dir: Option<&str>,
        format: IoFormat,
    ) -> Result<Self, Diagnostic> {
        let stdout = io::stdout();
        let mut stdout = stdout.lock();
        Self::parse_with_include_output(
            scanner,
            executable,
            category,
            train_dir,
            format,
            &mut stdout,
        )
    }

    pub fn parse_with_include_output<W: Write + ?Sized>(
        scanner: &mut Scanner,
        executable: impl Into<String>,
        category: &str,
        train_dir: Option<&str>,
        format: IoFormat,
        include_output: &mut W,
    ) -> Result<Self, Diagnostic> {
        let mut spec = Self::new(executable, format);
        spec.category = Some(category.to_owned());
        spec.train_dir = train_dir.map(str::to_owned);

        if scanner.test_id("execution") {
            accept_dotted_id(scanner, "execution.order")?;
            spec.ordered = scanner.test_id("ordered");
            scanner.accept_id("ordered|unordered")?;
        }

        accept_dotted_id(scanner, "output.required")?;
        parse_output_line(scanner, &mut spec, BatchOutputType::Required)?;

        if scanner.test_id("output") {
            accept_dotted_id(scanner, "output.desired")?;
            parse_output_line(scanner, &mut spec, BatchOutputType::Desired)?;
        }

        accept_dotted_id(scanner, "limit.time.problem.wc")?;
        spec.per_prob_limit = parse_int(scanner)?;

        if scanner.test_id("limit") {
            accept_dotted_id(scanner, "limit.time.overall.wc")?;
            spec.total_wtc_limit = parse_int(scanner)?;
        }

        while scanner.test_id("include") {
            let include = parse_basic_include(scanner)?;
            writeln!(include_output, "% Accepted {include} for parsing")
                .map_err(|error| output_error(&error))?;
            spec.includes.push(include);
        }

        while scanner.test_tok(TokenType::SLASH) || scanner.test_id("Problem|Problems") {
            let source = parse_filename(scanner)?;
            let dest = parse_filename(scanner)?;
            spec.source_files.push(source);
            spec.dest_files.push(dest);
        }

        Ok(spec)
    }

    #[must_use]
    pub fn problem_no(&self) -> usize {
        self.source_files.len()
    }

    pub fn write_to<W: Write + ?Sized>(&self, output: &mut W) -> Result<(), Diagnostic> {
        writeln!(output, "% SZS start BatchConfiguration").map_err(|error| output_error(&error))?;
        writeln!(
            output,
            "division.category {}",
            self.category.as_deref().unwrap_or("")
        )
        .map_err(|error| output_error(&error))?;
        if let Some(train_dir) = &self.train_dir {
            writeln!(output, "division.category.training_directory {train_dir}")
                .map_err(|error| output_error(&error))?;
        }
        if self.ordered {
            writeln!(output, "execution.order ordered").map_err(|error| output_error(&error))?;
        }

        write!(output, "output.required").map_err(|error| output_error(&error))?;
        self.write_output_line(output, BatchOutputType::Required)?;
        writeln!(output).map_err(|error| output_error(&error))?;

        write!(output, "output.desired").map_err(|error| output_error(&error))?;
        self.write_output_line(output, BatchOutputType::Desired)?;
        writeln!(output).map_err(|error| output_error(&error))?;

        writeln!(output, "limit.time.problem.wc {}", self.per_prob_limit)
            .map_err(|error| output_error(&error))?;
        writeln!(output, "limit.time.overall.wc {}", self.total_wtc_limit)
            .map_err(|error| output_error(&error))?;
        writeln!(output, "% SZS end BatchConfiguration").map_err(|error| output_error(&error))?;
        writeln!(output, "% SZS start BatchIncludes").map_err(|error| output_error(&error))?;
        for include in &self.includes {
            writeln!(output, "include('{include}').").map_err(|error| output_error(&error))?;
        }
        writeln!(output, "% SZS end BatchIncludes").map_err(|error| output_error(&error))?;
        writeln!(output, "% SZS start BatchProblems").map_err(|error| output_error(&error))?;
        for (source, dest) in self.source_files.iter().zip(&self.dest_files) {
            writeln!(output, "{source} {dest}").map_err(|error| output_error(&error))?;
        }
        writeln!(output, "% SZS end BatchProblems").map_err(|error| output_error(&error))
    }

    pub fn print_string(&self) -> Result<String, Diagnostic> {
        let mut output = Vec::new();
        self.write_to(&mut output)?;
        String::from_utf8(output).map_err(|error| {
            Diagnostic::new(
                ErrorCode::FILE_ERROR,
                format!("Could not build batch specification output: {error}"),
            )
        })
    }

    fn write_output_line<W: Write + ?Sized>(
        &self,
        output: &mut W,
        state: BatchOutputType,
    ) -> Result<(), Diagnostic> {
        if self.res_assurance == state {
            write!(output, " Assurance").map_err(|error| output_error(&error))?;
        }
        if self.res_proof == state {
            write!(output, " Proof").map_err(|error| output_error(&error))?;
        }
        if self.res_model == state {
            write!(output, " Model").map_err(|error| output_error(&error))?;
        }
        if self.res_answer == state {
            write!(output, " Answer").map_err(|error| output_error(&error))?;
        }
        if self.res_list_fof == state {
            write!(output, " ListOfFOF").map_err(|error| output_error(&error))?;
        }
        Ok(())
    }
}

pub fn parse_ltb_header(scanner: &mut Scanner) -> Result<BatchSpecHeader, Diagnostic> {
    accept_dotted_id(scanner, "division.category")?;
    let category = parse_dotted_id(scanner)?;
    let train_dir = if scanner.test_id("division") {
        accept_dotted_id(scanner, "division.category.training_data")?;
        Some(parse_continuous(scanner)?)
    } else {
        None
    };

    Ok(BatchSpecHeader {
        category,
        train_dir,
    })
}

#[must_use]
pub fn abstract_to_concrete(name: &str, variant: &str, postfix: &str) -> String {
    let prefix = name.split_once('*').map_or(name, |(prefix, _)| prefix);
    let mut result = String::with_capacity(prefix.len() + variant.len() + postfix.len());
    result.push_str(prefix);
    result.push_str(variant);
    result.push_str(postfix);
    result
}

fn parse_output_line(
    scanner: &mut Scanner,
    spec: &mut BatchSpec,
    state: BatchOutputType,
) -> Result<(), Diagnostic> {
    while scanner.test_id("Assurance|Proof|Model|Answer|ListOfFOF") {
        match scanner.current_token().literal().as_str() {
            "Assurance" => spec.res_assurance = state,
            "Proof" => spec.res_proof = state,
            "Model" => spec.res_model = state,
            "Answer" => spec.res_answer = state,
            "ListOfFOF" => spec.res_list_fof = state,
            _ => {
                return Err(Diagnostic::new(
                    ErrorCode::SYNTAX_ERROR,
                    format!(
                        "{} Unknown batch output field {}",
                        token_pos_rep(scanner.current_token()),
                        scanner.current_token().literal()
                    ),
                ));
            }
        }
        scanner.accept_tok(TokenType::IDENT)?;
    }
    Ok(())
}

fn output_error(error: &io::Error) -> Diagnostic {
    Diagnostic::new(
        ErrorCode::FILE_ERROR,
        format!("Could not write batch specification output: {error}"),
    )
}

#[cfg(test)]
mod tests {
    use super::{
        abstract_to_concrete, parse_ltb_header, BatchOutputType, BatchSpec, BATCH_FILTERS,
        BATCH_FILTERS_DIV, BATCH_STRATEGIES, BATCH_STRATEGIES_DIV,
    };
    use crate::inout::scanner::{IoFormat, Scanner};

    #[test]
    fn batch_spec_defaults_match_c_allocation_shape() {
        let spec = BatchSpec::new("eprover", IoFormat::Tstp);

        assert_eq!(spec.executable, "eprover");
        assert_eq!(spec.format, IoFormat::Tstp);
        assert_eq!(spec.category, None);
        assert_eq!(spec.train_dir, None);
        assert!(!spec.ordered);
        assert_eq!(spec.res_assurance, BatchOutputType::NoOutput);
        assert_eq!(spec.res_proof, BatchOutputType::NoOutput);
        assert_eq!(spec.res_model, BatchOutputType::NoOutput);
        assert_eq!(spec.res_answer, BatchOutputType::NoOutput);
        assert_eq!(spec.res_list_fof, BatchOutputType::NoOutput);
        assert_eq!(spec.per_prob_limit, 0);
        assert_eq!(spec.total_wtc_limit, 0);
        assert_eq!(spec.problem_no(), 0);
    }

    #[test]
    fn parse_ltb_header_preserves_training_data_input_spelling() {
        let mut scanner = Scanner::from_user_string(
            "division.category LTB.SAT\n\
             division.category.training_data /tmp/train/set-01\n\
             output.required Proof\n",
            true,
        )
        .unwrap();

        let header = parse_ltb_header(&mut scanner).unwrap();

        assert_eq!(header.category, "LTB.SAT");
        assert_eq!(header.train_dir.as_deref(), Some("/tmp/train/set-01"));
        assert!(scanner.test_id("output"));
    }

    #[test]
    fn parse_batch_spec_preserves_loose_c_control_flow() {
        let mut scanner = Scanner::from_user_string(
            "execution.order unordered\n\
             output.required Assurance Proof ListOfFOF\n\
             output.desired Model Answer\n\
             limit.time.problem.wc 17\n\
             limit.time.overall.wc 90\n\
             include('Axioms/SET001.ax').\n\
             /tmp/prob1.p /tmp/out1\n\
             Problems/TSTP/prob2.p Problems/Out/prob2.out\n\
             tail\n",
            true,
        )
        .unwrap();
        let mut notices = Vec::new();

        let spec = BatchSpec::parse_with_include_output(
            &mut scanner,
            "eprover",
            "LTB.SAT",
            Some("/train"),
            IoFormat::Tstp,
            &mut notices,
        )
        .unwrap();

        assert!(!spec.ordered);
        assert_eq!(spec.res_assurance, BatchOutputType::Required);
        assert_eq!(spec.res_proof, BatchOutputType::Required);
        assert_eq!(spec.res_model, BatchOutputType::Desired);
        assert_eq!(spec.res_answer, BatchOutputType::Desired);
        assert_eq!(spec.res_list_fof, BatchOutputType::Required);
        assert_eq!(spec.per_prob_limit, 17);
        assert_eq!(spec.total_wtc_limit, 90);
        assert_eq!(spec.includes, ["Axioms/SET001.ax"]);
        assert_eq!(spec.source_files, ["/tmp/prob1.p", "Problems/TSTP/prob2.p"]);
        assert_eq!(spec.dest_files, ["/tmp/out1", "Problems/Out/prob2.out"]);
        assert_eq!(spec.problem_no(), 2);
        assert_eq!(
            String::from_utf8(notices).unwrap(),
            "% Accepted Axioms/SET001.ax for parsing\n"
        );
        assert!(scanner.test_id("tail"));
    }

    #[test]
    fn print_batch_spec_uses_c_field_order_and_training_directory_spelling() {
        let mut spec = BatchSpec::new("eprover", IoFormat::Tstp);
        spec.category = Some("LTB.SAT".to_owned());
        spec.train_dir = Some("/train".to_owned());
        spec.ordered = true;
        spec.res_assurance = BatchOutputType::Required;
        spec.res_proof = BatchOutputType::Desired;
        spec.res_model = BatchOutputType::Required;
        spec.per_prob_limit = 11;
        spec.total_wtc_limit = 22;
        spec.includes.push("Axioms/SET001.ax".to_owned());
        spec.source_files.push("Problems/TSTP/prob.p".to_owned());
        spec.dest_files.push("Problems/Out/prob.out".to_owned());

        assert_eq!(
            spec.print_string().unwrap(),
            "% SZS start BatchConfiguration\n\
             division.category LTB.SAT\n\
             division.category.training_directory /train\n\
             execution.order ordered\n\
             output.required Assurance Model\n\
             output.desired Proof\n\
             limit.time.problem.wc 11\n\
             limit.time.overall.wc 22\n\
             % SZS end BatchConfiguration\n\
             % SZS start BatchIncludes\n\
             include('Axioms/SET001.ax').\n\
             % SZS end BatchIncludes\n\
             % SZS start BatchProblems\n\
             Problems/TSTP/prob.p Problems/Out/prob.out\n\
             % SZS end BatchProblems\n"
        );
    }

    #[test]
    fn abstract_to_concrete_ignores_text_after_star() {
        assert_eq!(
            abstract_to_concrete("Problems/*/ignored.p", "ALG001", ".p"),
            "Problems/ALG001.p"
        );
        assert_eq!(abstract_to_concrete("plain", "VAR", ".ax"), "plainVAR.ax");
    }

    #[test]
    fn batch_variant_tables_preserve_c_lengths_and_pairing() {
        assert_eq!(BATCH_FILTERS.len(), BATCH_STRATEGIES.len());
        assert_eq!(BATCH_FILTERS_DIV.len(), BATCH_STRATEGIES_DIV.len());
        assert_eq!(BATCH_FILTERS[0], "threshold010000");
        assert_eq!(
            BATCH_STRATEGIES_DIV[0],
            "--auto-schedule --assume-incompleteness"
        );
        assert_eq!(
            BATCH_STRATEGIES_DIV[13],
            "-xAutoSched2 -tAutoSched2 --assume-incompleteness"
        );
    }

    #[test]
    fn output_type_values_match_c_enum_order() {
        assert_eq!(BatchOutputType::NoOutput.c_value(), 0);
        assert_eq!(BatchOutputType::Desired.c_value(), 1);
        assert_eq!(BatchOutputType::Required.c_value(), 2);
    }
}
