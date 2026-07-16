use crate::basics::defines::DEFAULT_COMCHAR_RAW;
use crate::basics::error::{Diagnostic, ErrorCode};
use crate::basics::simple_stuff::ProblemType;
use crate::clauses::clause::{clause_parse, Clause};
use crate::clauses::clausesets::ClauseSet;
use crate::clauses::subsumption::{
    clause_set_subsumes_clause_with_bank, clause_subsume_order_sort_lits,
    clause_subsumes_clause_with_bank, unit_clause_set_subsumes_clause_with_bank,
    unit_clause_subsumes_clause_with_bank,
};
use crate::clauses::tautologies::clause_is_tautology;
use crate::inout::basicparser::parse_float;
use crate::inout::scanner::{IoFormat, Scanner, TokenType};
use crate::terms::signature::Signature;
use crate::terms::termbanks::TermBank;
use crate::terms::typebanks::TypeBank;
use std::fmt::Write as _;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CsscpaClauseStatus {
    Contradicts,
    Improved,
    Rejected,
    Forced,
    Requested,
    Unknown,
}

impl CsscpaClauseStatus {
    #[must_use]
    pub const fn c_value(self) -> i32 {
        match self {
            Self::Contradicts => 0,
            Self::Improved => 1,
            Self::Rejected => 2,
            Self::Forced => 3,
            Self::Requested => 4,
            Self::Unknown => 5,
        }
    }

    #[must_use]
    pub const fn as_c_str(self) -> &'static str {
        match self {
            Self::Contradicts => "contradicts",
            Self::Improved => "improved",
            Self::Rejected => "rejected",
            Self::Forced => "forced",
            Self::Requested => "requested",
            Self::Unknown => "unknown",
        }
    }

    #[must_use]
    pub const fn is_accepted(self) -> bool {
        matches!(self, Self::Contradicts | Self::Improved | Self::Forced)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CsscpaProcessResult {
    status: CsscpaClauseStatus,
    accepted: bool,
    trace: String,
    trace_flush_offsets: Vec<usize>,
}

impl CsscpaProcessResult {
    fn new(status: CsscpaClauseStatus, trace: String, trace_flush_offsets: Vec<usize>) -> Self {
        Self {
            accepted: status.is_accepted(),
            status,
            trace,
            trace_flush_offsets,
        }
    }

    #[must_use]
    pub const fn status(&self) -> CsscpaClauseStatus {
        self.status
    }

    #[must_use]
    pub const fn accepted(&self) -> bool {
        self.accepted
    }

    #[must_use]
    pub fn trace(&self) -> &str {
        &self.trace
    }

    #[must_use]
    pub fn trace_flush_offsets(&self) -> &[usize] {
        &self.trace_flush_offsets
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CsscpaLoopResult {
    output_level: i64,
    processed: usize,
    accepted: usize,
    trace: String,
    trace_flush_offsets: Vec<usize>,
}

impl CsscpaLoopResult {
    #[must_use]
    pub const fn output_level(&self) -> i64 {
        self.output_level
    }

    #[must_use]
    pub const fn processed(&self) -> usize {
        self.processed
    }

    #[must_use]
    pub const fn accepted(&self) -> usize {
        self.accepted
    }

    #[must_use]
    pub fn trace(&self) -> &str {
        &self.trace
    }

    #[must_use]
    pub fn trace_flush_offsets(&self) -> &[usize] {
        &self.trace_flush_offsets
    }
}

#[derive(Clone, Debug)]
pub struct CsscpaState {
    terms: TermBank,
    pos_units: ClauseSet,
    neg_units: ClauseSet,
    non_units: ClauseSet,
    literals: i64,
    clauses: i64,
    weight: i64,
}

impl CsscpaState {
    pub fn new() -> Result<Self, Diagnostic> {
        let mut signature = Signature::new(TypeBank::new());
        // The C FOL build compiles phony-application checks away. Rust selects
        // FOL/HO behavior at runtime, so fixed helper codes must remain reserved
        // even for this first-order-only parser or ordinary user symbols can be
        // mistaken for lambda/application cells once the signature grows.
        signature.insert_internal_codes()?;
        let terms = TermBank::new(signature)?;
        Ok(Self {
            terms,
            pos_units: ClauseSet::new(),
            neg_units: ClauseSet::new(),
            non_units: ClauseSet::new(),
            literals: 0,
            clauses: 0,
            weight: 0,
        })
    }

    #[must_use]
    pub const fn terms(&self) -> &TermBank {
        &self.terms
    }

    pub const fn terms_mut(&mut self) -> &mut TermBank {
        &mut self.terms
    }

    #[must_use]
    pub const fn pos_units(&self) -> &ClauseSet {
        &self.pos_units
    }

    #[must_use]
    pub const fn neg_units(&self) -> &ClauseSet {
        &self.neg_units
    }

    #[must_use]
    pub const fn non_units(&self) -> &ClauseSet {
        &self.non_units
    }

    #[must_use]
    pub const fn literals(&self) -> i64 {
        self.literals
    }

    #[must_use]
    pub const fn clauses(&self) -> i64 {
        self.clauses
    }

    #[must_use]
    pub const fn weight(&self) -> i64 {
        self.weight
    }

    #[must_use]
    pub fn state_line(&self, status: CsscpaClauseStatus, source_clause: Option<&Clause>) -> String {
        let source = source_clause.map_or(0, Clause::query_csscpa_source);
        self.state_line_for_source(status, source)
    }

    #[must_use]
    pub fn state_line_for_source(&self, status: CsscpaClauseStatus, source: u64) -> String {
        format!(
            "{DEFAULT_COMCHAR_RAW} CSSCPAState: {:<10} by {source}, {}, {}, {} (system, clauses,literals,weight)\n",
            status.as_c_str(),
            self.clauses,
            self.literals,
            self.weight,
        )
    }

    pub fn process_clause(
        &mut self,
        clause: Clause,
        accept: bool,
        weight_delta: f32,
        average_delta: f32,
    ) -> Result<bool, Diagnostic> {
        self.process_clause_with_trace(clause, accept, weight_delta, average_delta, 0)
            .map(|result| result.accepted)
    }

    pub fn process_clause_with_trace(
        &mut self,
        mut clause: Clause,
        accept: bool,
        weight_delta: f32,
        average_delta: f32,
        output_level: i64,
    ) -> Result<CsscpaProcessResult, Diagnostic> {
        let mut trace = String::new();
        let mut trace_flush_offsets = Vec::new();
        let mut status = if accept {
            CsscpaClauseStatus::Forced
        } else {
            CsscpaClauseStatus::Unknown
        };

        if self.clause_is_tautology(&clause)? {
            status = CsscpaClauseStatus::Rejected;
            if output_level_is_enabled(output_level) {
                let _ = writeln!(
                    trace,
                    "{DEFAULT_COMCHAR_RAW} Clause {} rejected (Tautology)",
                    clause.ident()
                );
            }
        }

        if status != CsscpaClauseStatus::Rejected {
            prepare_clause_for_subsumption(&mut clause, &self.terms);
            if let Some(handle_id) = self.subsuming_clause_id(&clause)? {
                status = CsscpaClauseStatus::Rejected;
                if output_level_is_enabled(output_level) {
                    let _ = writeln!(
                        trace,
                        "{DEFAULT_COMCHAR_RAW} Clause {} rejected (subsumed by {handle_id})",
                        clause.ident()
                    );
                }
            }
        }

        if status != CsscpaClauseStatus::Rejected {
            let subsumed = self.collect_subsumed(&clause)?;
            let sub_weight = subsumed.iter().map(|entry| entry.weight).sum::<i64>();
            let accepted_source = clause.query_csscpa_source();
            let improves = i64_to_f32(sub_weight - clause.weight())
                > weight_delta * i64_to_f32(self.weight)
                || (self.clauses != 0
                    && (i64_to_f64(self.weight + clause.weight())
                        / (i64_to_f64(self.clauses) + 1.0))
                        < ((1.0 - f64::from(average_delta)) * i64_to_f64(self.weight)
                            / i64_to_f64(self.clauses)));
            if improves {
                status = CsscpaClauseStatus::Improved;
            } else if clause.is_unit() && self.find_unit_contradiction(&clause)?.is_some() {
                status = CsscpaClauseStatus::Contradicts;
                if output_level_allows(output_level, 1) {
                    let _ = writeln!(trace, "{DEFAULT_COMCHAR_RAW} Unit contradiction found!");
                }
            }

            if status.is_accepted() {
                // C collects matching clauses onto a stack and removes them by
                // popping that stack, so observable removal traces run in the
                // reverse of clause-set traversal order.
                for entry in subsumed.into_iter().rev() {
                    if let Some(removed) = self.remove_subsumed(entry.bucket, entry.ident) {
                        self.clauses -= 1;
                        self.literals -= usize_to_i64(removed.literal_number());
                        self.weight -= removed.weight();
                        if output_level_is_enabled(output_level) {
                            let _ = writeln!(
                                trace,
                                "{DEFAULT_COMCHAR_RAW} Clause {} removed from list (subsumed by {})",
                                removed.ident(),
                                clause.ident()
                            );
                        }
                    }
                }
                self.clauses += 1;
                self.literals += usize_to_i64(clause.literal_number());
                self.weight += clause.weight();

                if output_level_is_enabled(output_level) {
                    let _ = writeln!(
                        trace,
                        "{DEFAULT_COMCHAR_RAW} Clause {} accepted from {} ({})",
                        clause.ident(),
                        clause.query_csscpa_source(),
                        status.as_c_str()
                    );
                }
                self.insert_clause(clause);
                if matches!(
                    status,
                    CsscpaClauseStatus::Contradicts | CsscpaClauseStatus::Improved
                ) && output_level_is_enabled(output_level)
                {
                    trace.push_str(&self.state_line_for_source(status, accepted_source));
                    trace_flush_offsets.push(trace.len());
                }
            } else {
                status = CsscpaClauseStatus::Rejected;
                if output_level_is_enabled(output_level) {
                    let _ = writeln!(
                        trace,
                        "{DEFAULT_COMCHAR_RAW} Clause {} rejected (weighty)",
                        clause.ident()
                    );
                }
            }
        }

        trace_flush_offsets.push(trace.len());
        Ok(CsscpaProcessResult::new(status, trace, trace_flush_offsets))
    }

    pub fn process_loop(
        &mut self,
        scanner: &mut Scanner,
        initial_output_level: i64,
    ) -> Result<CsscpaLoopResult, Diagnostic> {
        let mut output_level = initial_output_level;
        let mut processed = 0;
        let mut accepted = 0;
        let mut trace = String::new();
        let mut trace_flush_offsets = Vec::new();

        while !scanner.test_tok(TokenType::NO_TOKEN) {
            if scanner.test_id("output_level") {
                scanner.next_token()?;
                output_level = parse_csscpa_output_level(scanner, output_level)?;
                continue;
            }

            if scanner.test_id("state") {
                scanner.next_token()?;
                scanner.accept_tok(TokenType::COLON)?;
                trace.push_str(&self.state_line_for_source(CsscpaClauseStatus::Requested, 0));
                trace_flush_offsets.push(trace.len());
                continue;
            }

            if scanner.test_id("Please") {
                accept_please_sequence(scanner)?;
                continue;
            }

            let accept = parse_accept_or_check(scanner)?;
            let source = parse_optional_csscpa_source(scanner)?;
            let (weight_delta, average_delta) = parse_optional_improve(scanner)?;
            scanner.accept_tok(TokenType::COLON)?;

            let mut clause = parse_csscpa_loop_clause(scanner, self.terms_mut())?;
            clause.set_csscpa_source(source);
            let result = self.process_clause_with_trace(
                clause,
                accept,
                weight_delta,
                average_delta,
                output_level,
            )?;
            processed += 1;
            if result.accepted() {
                accepted += 1;
            }
            let trace_base = trace.len();
            trace.push_str(result.trace());
            trace_flush_offsets.extend(
                result
                    .trace_flush_offsets()
                    .iter()
                    .map(|offset| trace_base + offset),
            );
        }

        Ok(CsscpaLoopResult {
            output_level,
            processed,
            accepted,
            trace,
            trace_flush_offsets,
        })
    }

    fn clause_is_tautology(&self, clause: &Clause) -> Result<bool, Diagnostic> {
        let mut work_bank = TermBank::new(self.terms.signature().clone())?;
        clause_is_tautology(&mut work_bank, clause)
    }

    fn subsuming_clause_id(&mut self, clause: &Clause) -> Result<Option<i64>, Diagnostic> {
        let Self {
            terms,
            pos_units,
            neg_units,
            non_units,
            ..
        } = self;
        if clause.positive_literal_count() != 0 {
            if let Some(handle) =
                unit_clause_set_subsumes_clause_with_bank(terms, pos_units, clause, false)?
            {
                return Ok(Some(handle.ident()));
            }
        }
        if clause.negative_literal_count() != 0 {
            if let Some(handle) =
                unit_clause_set_subsumes_clause_with_bank(terms, neg_units, clause, false)?
            {
                return Ok(Some(handle.ident()));
            }
        }
        if clause.literal_number() > 1 {
            if let Some(handle) = clause_set_subsumes_clause_with_bank(non_units, clause, terms)? {
                return Ok(Some(handle.ident()));
            }
        }
        Ok(None)
    }

    fn collect_subsumed(&mut self, clause: &Clause) -> Result<Vec<SubsumedClause>, Diagnostic> {
        let Self {
            terms,
            pos_units,
            neg_units,
            non_units,
            ..
        } = self;
        let mut result = Vec::new();
        if clause.is_unit() && clause.is_positive() {
            collect_unit_subsumed(
                &mut result,
                ClauseBucket::Positive,
                pos_units,
                clause,
                terms,
            )?;
        } else if clause.is_unit() {
            collect_unit_subsumed(
                &mut result,
                ClauseBucket::Negative,
                neg_units,
                clause,
                terms,
            )?;
        }
        collect_clause_subsumed(&mut result, ClauseBucket::NonUnit, non_units, clause, terms)?;
        Ok(result)
    }

    fn find_unit_contradiction(&mut self, clause: &Clause) -> Result<Option<&Clause>, Diagnostic> {
        debug_assert!(clause.is_unit());
        let Some(literal) = clause.literals().as_slice().first() else {
            return Ok(None);
        };
        let Self {
            terms,
            pos_units,
            neg_units,
            ..
        } = self;
        let set = if literal.is_positive() {
            neg_units
        } else {
            pos_units
        };
        for candidate in set.iter() {
            debug_assert!(candidate.is_unit());
            let Some(candidate_literal) = candidate.literals().as_slice().first() else {
                continue;
            };
            if literal.is_positive() != candidate.is_positive()
                && literal.unify_p_with_bank(candidate_literal, terms)?
            {
                return Ok(Some(candidate));
            }
        }
        Ok(None)
    }

    fn insert_clause(&mut self, clause: Clause) {
        if clause.is_unit() && clause.is_positive() {
            self.pos_units
                .indexed_insert_clause_owned(clause, &self.terms);
        } else if clause.is_unit() {
            self.neg_units
                .indexed_insert_clause_owned(clause, &self.terms);
        } else {
            self.non_units
                .indexed_insert_clause_owned(clause, &self.terms);
        }
    }

    fn remove_subsumed(&mut self, bucket: ClauseBucket, ident: i64) -> Option<Clause> {
        match bucket {
            ClauseBucket::Positive => self.pos_units.extract_by_id(ident),
            ClauseBucket::Negative => self.neg_units.extract_by_id(ident),
            ClauseBucket::NonUnit => self.non_units.extract_by_id(ident),
        }
    }
}

pub fn csscpa_loop(
    scanner: &mut Scanner,
    state: &mut CsscpaState,
    initial_output_level: i64,
) -> Result<CsscpaLoopResult, Diagnostic> {
    state.process_loop(scanner, initial_output_level)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ClauseBucket {
    Positive,
    Negative,
    NonUnit,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct SubsumedClause {
    bucket: ClauseBucket,
    ident: i64,
    weight: i64,
}

fn prepare_clause_for_subsumption(clause: &mut Clause, bank: &TermBank) {
    clause.set_weight(clause.standard_weight());
    clause_subsume_order_sort_lits(clause, bank);
}

fn collect_unit_subsumed(
    result: &mut Vec<SubsumedClause>,
    bucket: ClauseBucket,
    set: &ClauseSet,
    clause: &Clause,
    bank: &mut TermBank,
) -> Result<(), Diagnostic> {
    for candidate in set.iter() {
        if unit_clause_subsumes_clause_with_bank(bank, clause, candidate)? {
            result.push(SubsumedClause {
                bucket,
                ident: candidate.ident(),
                weight: candidate.weight(),
            });
        }
    }
    Ok(())
}

fn collect_clause_subsumed(
    result: &mut Vec<SubsumedClause>,
    bucket: ClauseBucket,
    set: &ClauseSet,
    clause: &Clause,
    bank: &mut TermBank,
) -> Result<(), Diagnostic> {
    for candidate in set.iter() {
        let subsumed = if clause.is_unit() {
            unit_clause_subsumes_clause_with_bank(bank, clause, candidate)?
        } else {
            clause_subsumes_clause_with_bank(clause, candidate, bank)?
        };
        if subsumed {
            result.push(SubsumedClause {
                bucket,
                ident: candidate.ident(),
                weight: candidate.weight(),
            });
        }
    }
    Ok(())
}

fn usize_to_i64(value: usize) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

const fn output_level_is_enabled(output_level: i64) -> bool {
    output_level != 0
}

const fn output_level_allows(output_level: i64, required_level: i64) -> bool {
    required_level <= output_level
}

fn parse_csscpa_output_level(
    scanner: &mut Scanner,
    current_output_level: i64,
) -> Result<i64, Diagnostic> {
    scanner.check_tok(TokenType::POS_INT)?;
    let parsed = scanner.current_token().numval();
    scanner.accept_tok(TokenType::POS_INT)?;
    Ok(match parsed {
        0 => 0,
        1 => 1,
        _ => current_output_level,
    })
}

fn parse_accept_or_check(scanner: &mut Scanner) -> Result<bool, Diagnostic> {
    scanner.check_id("accept|check")?;
    let accept = scanner.test_id("accept");
    scanner.next_token()?;
    Ok(accept)
}

fn parse_optional_csscpa_source(scanner: &mut Scanner) -> Result<u64, Diagnostic> {
    if !scanner.test_id("from") {
        return Ok(0);
    }
    scanner.next_token()?;
    scanner.check_tok(TokenType::POS_INT)?;
    let source = scanner.current_token().numval();
    if !(2..=15).contains(&source) {
        return Err(csscpa_syntax_error(
            "CSSCPA source specifier must be in the range 2...15",
        ));
    }
    scanner.accept_tok(TokenType::POS_INT)?;
    Ok(source)
}

fn parse_optional_improve(scanner: &mut Scanner) -> Result<(f32, f32), Diagnostic> {
    if !scanner.test_id("improve") {
        return Ok((0.0, 0.0));
    }
    scanner.next_token()?;
    scanner.accept_tok(TokenType::OPEN_BRACKET)?;
    let weight_delta = f64_to_f32(parse_float(scanner)?);
    scanner.accept_tok(TokenType::COMMA)?;
    let average_delta = f64_to_f32(parse_float(scanner)?);
    scanner.accept_tok(TokenType::CLOSE_BRACKET)?;
    Ok((weight_delta, average_delta))
}

fn parse_csscpa_loop_clause(
    scanner: &mut Scanner,
    bank: &mut TermBank,
) -> Result<Clause, Diagnostic> {
    let saved_format = scanner.format();
    if saved_format == IoFormat::Tstp && scanner.test_id("input_clause") {
        scanner.set_format(IoFormat::Tptp);
        let result = clause_parse(scanner, bank, ProblemType::FirstOrder);
        scanner.set_format(saved_format);
        return result;
    }
    clause_parse(scanner, bank, ProblemType::FirstOrder)
}

fn accept_please_sequence(scanner: &mut Scanner) -> Result<(), Diagnostic> {
    scanner.accept_id("Please")?;
    scanner.accept_id("process")?;
    scanner.accept_id("clauses")?;
    scanner.accept_id("now")?;
    scanner.accept_tok(TokenType::COMMA)?;
    scanner.accept_id("I")?;
    scanner.accept_id("beg")?;
    scanner.accept_id("you")?;
    scanner.accept_tok(TokenType::COMMA)?;
    scanner.accept_id("great")?;
    scanner.accept_id("shining")?;
    scanner.accept_id("CSSCPA")?;
    scanner.accept_tok(TokenType::COMMA)?;
    scanner.accept_id("wonder")?;
    scanner.accept_id("of")?;
    scanner.accept_id("the")?;
    scanner.accept_id("world")?;
    scanner.accept_tok(TokenType::COMMA)?;
    scanner.accept_id("most")?;
    scanner.accept_id("beautiful")?;
    scanner.accept_id("program")?;
    scanner.accept_id("ever")?;
    scanner.accept_id("written")?;
    scanner.accept_tok(TokenType::FULLSTOP)
}

fn csscpa_syntax_error(message: &str) -> Diagnostic {
    Diagnostic::new(ErrorCode::SYNTAX_ERROR, message)
}

#[allow(clippy::cast_precision_loss)]
fn i64_to_f32(value: i64) -> f32 {
    value as f32
}

#[allow(clippy::cast_precision_loss)]
fn i64_to_f64(value: i64) -> f64 {
    value as f64
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss
)]
fn f64_to_f32(value: f64) -> f32 {
    value as f32
}

#[cfg(test)]
mod tests {
    use super::{csscpa_loop, CsscpaClauseStatus, CsscpaState};
    use crate::basics::error::ErrorCode;
    use crate::basics::simple_stuff::{reset_problem_type, set_problem_type, ProblemType};
    use crate::clauses::clause::{clause_parse, Clause};
    use crate::clauses::eqn::Eqn;
    use crate::clauses::eqnlist::EqnList;
    use crate::inout::scanner::{IoFormat, Scanner};
    use crate::terms::lambda::apply_terms;
    use crate::terms::simpletypes::alloc_arrow_type;
    use crate::terms::termbanks::TermBank;
    use crate::terms::termtypes::Term;
    use crate::test_support::global_state_lock;

    struct ProblemTypeReset;

    impl Drop for ProblemTypeReset {
        fn drop(&mut self) {
            reset_problem_type();
        }
    }

    fn set_problem_type_for_test(problem_type: ProblemType) -> ProblemTypeReset {
        reset_problem_type();
        set_problem_type(problem_type).unwrap();
        ProblemTypeReset
    }

    fn parse_clause(state: &mut CsscpaState, source: &str) -> Clause {
        let mut scanner = Scanner::from_user_string(source, false).expect("scanner allocation");
        if source.starts_with("cnf(") {
            scanner.set_format(IoFormat::Tstp);
        }
        clause_parse(&mut scanner, state.terms_mut(), ProblemType::FirstOrder)
            .expect("CSSCPA test clause parses")
    }

    fn typed_const(bank: &mut TermBank, name: &str) -> Term {
        let type_ = bank.signature().type_bank().default_type();
        let f_code = bank.signature_mut().insert_id(name, 0, false);
        bank.signature_mut()
            .declare_final_type(f_code, type_)
            .unwrap();
        bank.create_const_term(f_code).unwrap()
    }

    #[test]
    fn status_discriminants_and_strings_match_c_enum() {
        assert_eq!(CsscpaClauseStatus::Contradicts.c_value(), 0);
        assert_eq!(CsscpaClauseStatus::Improved.c_value(), 1);
        assert_eq!(CsscpaClauseStatus::Rejected.c_value(), 2);
        assert_eq!(CsscpaClauseStatus::Forced.c_value(), 3);
        assert_eq!(CsscpaClauseStatus::Requested.c_value(), 4);
        assert_eq!(CsscpaClauseStatus::Unknown.c_value(), 5);
        assert_eq!(CsscpaClauseStatus::Contradicts.as_c_str(), "contradicts");
        assert_eq!(CsscpaClauseStatus::Requested.as_c_str(), "requested");
    }

    #[test]
    fn forced_clause_enters_matching_unit_bucket_and_updates_counts() {
        let mut state = CsscpaState::new().expect("CSSCPA state allocation");
        let mut clause = parse_clause(&mut state, "p(a).");
        clause.set_csscpa_source(3);
        let ident = clause.ident();

        let result = state
            .process_clause_with_trace(clause, true, 0.0, 0.0, 1)
            .expect("forced processing succeeds");

        assert!(result.accepted());
        assert_eq!(result.status(), CsscpaClauseStatus::Forced);
        assert_eq!(state.clauses(), 1);
        assert_eq!(state.literals(), 1);
        assert!(state.pos_units().find_by_id(ident).is_some());
        assert!(result
            .trace()
            .contains(&format!("% Clause {ident} accepted from 3 (forced)\n")));
    }

    #[test]
    fn subsumed_checked_clause_is_rejected_without_state_change() {
        let mut state = CsscpaState::new().expect("CSSCPA state allocation");
        let unit = parse_clause(&mut state, "p(X).");
        let unit_ident = unit.ident();
        assert!(state
            .process_clause(unit, true, 0.0, 0.0)
            .expect("unit accepted"));

        let candidate = parse_clause(&mut state, "cnf(csscpa_candidate,axiom,(p(a)|q(a))).");
        let candidate_ident = candidate.ident();
        let result = state
            .process_clause_with_trace(candidate, false, 0.0, 0.0, 1)
            .expect("subsumed check succeeds");

        assert!(!result.accepted());
        assert_eq!(result.status(), CsscpaClauseStatus::Rejected);
        assert_eq!(state.clauses(), 1);
        assert!(state.pos_units().find_by_id(unit_ident).is_some());
        assert!(state.non_units().find_by_id(candidate_ident).is_none());
        assert!(result.trace().contains(&format!(
            "% Clause {candidate_ident} rejected (subsumed by {unit_ident})\n"
        )));
    }

    #[test]
    fn improving_checked_clause_removes_subsumed_non_unit() {
        let mut state = CsscpaState::new().expect("CSSCPA state allocation");
        let wide = parse_clause(&mut state, "cnf(csscpa_wide,axiom,(p(a)|q(a))).");
        let wide_ident = wide.ident();
        assert!(state
            .process_clause(wide, true, 0.0, 0.0)
            .expect("wide clause accepted"));

        let narrow = parse_clause(&mut state, "p(a).");
        let narrow_ident = narrow.ident();
        let result = state
            .process_clause_with_trace(narrow, false, 0.0, 0.0, 1)
            .expect("improving check succeeds");

        assert!(result.accepted());
        assert_eq!(result.status(), CsscpaClauseStatus::Improved);
        assert_eq!(state.clauses(), 1);
        assert!(state.non_units().find_by_id(wide_ident).is_none());
        assert!(state.pos_units().find_by_id(narrow_ident).is_some());
        assert!(result.trace().contains(&format!(
            "% Clause {wide_ident} removed from list (subsumed by {narrow_ident})\n"
        )));
        assert!(result.trace().contains("% CSSCPAState: improved  "));
    }

    #[test]
    fn improving_clause_removes_all_subsumed_entries_in_c_stack_order() {
        let mut state = CsscpaState::new().expect("CSSCPA state allocation");
        let positive = parse_clause(&mut state, "p(a).");
        let positive_ident = positive.ident();
        assert!(state
            .process_clause(positive, true, 0.0, 0.0)
            .expect("positive unit accepted"));

        let first_wide = parse_clause(&mut state, "cnf(csscpa_first,axiom,(p(b)|q(b))).");
        let first_wide_ident = first_wide.ident();
        assert!(state
            .process_clause(first_wide, true, 0.0, 0.0)
            .expect("first wide clause accepted"));
        let second_wide = parse_clause(&mut state, "cnf(csscpa_second,axiom,(p(c)|r(c))).");
        let second_wide_ident = second_wide.ident();
        assert!(state
            .process_clause(second_wide, true, 0.0, 0.0)
            .expect("second wide clause accepted"));

        let general = parse_clause(&mut state, "p(X).");
        let general_ident = general.ident();
        let result = state
            .process_clause_with_trace(general, false, 0.0, 1.0, 1)
            .expect("generalizing check succeeds");

        assert_eq!(result.status(), CsscpaClauseStatus::Improved);
        assert_eq!(state.clauses(), 1);
        assert_eq!(state.pos_units().len(), 1);
        assert!(state.pos_units().find_by_id(general_ident).is_some());
        let second_removal = format!(
            "% Clause {second_wide_ident} removed from list (subsumed by {general_ident})\n"
        );
        let first_removal = format!(
            "% Clause {first_wide_ident} removed from list (subsumed by {general_ident})\n"
        );
        let positive_removal =
            format!("% Clause {positive_ident} removed from list (subsumed by {general_ident})\n");
        let trace = result.trace();
        let second_offset = trace
            .find(&second_removal)
            .expect("second wide removal traced");
        let first_offset = trace
            .find(&first_removal)
            .expect("first wide removal traced");
        let positive_offset = trace
            .find(&positive_removal)
            .expect("positive unit removal traced");
        assert!(second_offset < first_offset);
        assert!(first_offset < positive_offset);
    }

    #[test]
    fn unit_contradiction_accepts_checked_clause() {
        let mut state = CsscpaState::new().expect("CSSCPA state allocation");
        let negative = parse_clause(&mut state, "~p(a).");
        assert!(state
            .process_clause(negative, true, 0.0, 0.0)
            .expect("negative unit accepted"));

        let positive = parse_clause(&mut state, "p(a).");
        let positive_ident = positive.ident();
        let result = state
            .process_clause_with_trace(positive, false, 1.0, 0.0, 1)
            .expect("contradiction check succeeds");

        assert!(result.accepted());
        assert_eq!(result.status(), CsscpaClauseStatus::Contradicts);
        assert_eq!(state.clauses(), 2);
        assert!(state.pos_units().find_by_id(positive_ident).is_some());
        assert!(result.trace().contains("% Unit contradiction found!\n"));
        assert!(result.trace().contains("% CSSCPAState: contradicts"));
        let trace_len = result.trace().len();
        assert_eq!(result.trace_flush_offsets(), &[trace_len, trace_len]);
    }

    #[test]
    fn unit_contradiction_uses_banked_higher_order_mgu() {
        let _global_state = global_state_lock();
        let _problem_type = set_problem_type_for_test(ProblemType::HigherOrder);
        let mut state = CsscpaState::new().expect("CSSCPA state allocation");
        let (negative, positive, function) = {
            let bank = state.terms_mut();
            let individual = bank.signature().type_bank().default_type();
            let unary = bank
                .signature_mut()
                .type_bank_mut()
                .insert_type_shared(alloc_arrow_type(vec![
                    individual.clone(),
                    individual.clone(),
                ]));
            let binary = bank
                .signature_mut()
                .type_bank_mut()
                .insert_type_shared(alloc_arrow_type(vec![
                    individual.clone(),
                    individual.clone(),
                    individual,
                ]));
            let function = bank.vars().get_fresh_var(&unary);
            let prefix = typed_const(bank, "csscpa_ho_prefix");
            let suffix = typed_const(bank, "csscpa_ho_suffix");
            let rhs = typed_const(bank, "csscpa_ho_rhs");
            let rigid_code = bank.signature_mut().insert_id("csscpa_ho_rigid", 0, false);
            bank.signature_mut()
                .declare_final_type(rigid_code, binary)
                .unwrap();
            let rigid = bank.create_const_term(rigid_code).unwrap();
            let flex_application =
                apply_terms(bank, &function, std::slice::from_ref(&suffix)).unwrap();
            let rigid_application = apply_terms(bank, &rigid, &[prefix, suffix]).unwrap();
            let negative = Clause::alloc(EqnList::from_vec(vec![Eqn::alloc(
                rigid_application,
                rhs.clone(),
                bank,
                false,
            )
            .unwrap()]));
            let positive = Clause::alloc(EqnList::from_vec(vec![Eqn::alloc(
                flex_application,
                rhs,
                bank,
                true,
            )
            .unwrap()]));
            (negative, positive, function)
        };

        assert!(state
            .process_clause(negative, true, 0.0, 0.0)
            .expect("negative higher-order unit accepted"));
        let result = state
            .process_clause_with_trace(positive, false, 1.0, 1.0, 1)
            .expect("higher-order contradiction check succeeds");

        assert_eq!(result.status(), CsscpaClauseStatus::Contradicts);
        assert!(result.accepted());
        assert!(function.binding().is_none());
    }

    #[test]
    fn unit_contradiction_level_minus_one_matches_c_output_gates() {
        let mut state = CsscpaState::new().expect("CSSCPA state allocation");
        let negative = parse_clause(&mut state, "~p(a).");
        assert!(state
            .process_clause(negative, true, 0.0, 0.0)
            .expect("negative unit accepted"));

        let positive = parse_clause(&mut state, "p(a).");
        let positive_ident = positive.ident();
        let result = state
            .process_clause_with_trace(positive, false, 1.0, 0.0, -1)
            .expect("contradiction check succeeds");

        assert!(result.accepted());
        assert_eq!(result.status(), CsscpaClauseStatus::Contradicts);
        assert!(result.trace().contains(&format!(
            "% Clause {positive_ident} accepted from 0 (contradicts)\n"
        )));
        assert!(result.trace().contains("% CSSCPAState: contradicts"));
        assert!(!result.trace().contains("% Unit contradiction found!\n"));
        let trace_len = result.trace().len();
        assert_eq!(result.trace_flush_offsets(), &[trace_len, trace_len]);
    }

    #[test]
    fn loop_parses_commands_state_request_and_please_sequence() {
        let mut state = CsscpaState::new().expect("CSSCPA state allocation");
        let mut scanner = Scanner::from_user_string(
            "\
output_level 0
state:
output_level 1
accept from 2: cnf(csscpa_unit,axiom,p(a)).
check improve(0.0,0.0): cnf(csscpa_candidate,axiom,(p(a)|q(a))).
Please process clauses now, I beg you, great shining CSSCPA,
wonder of the world, most beautiful program ever written.
state:",
            false,
        )
        .expect("CSSCPA loop scanner allocation");
        scanner.set_format(IoFormat::Tstp);

        let result = csscpa_loop(&mut scanner, &mut state, 1).expect("CSSCPA loop parses");

        assert_eq!(result.output_level(), 1);
        assert_eq!(result.processed(), 2);
        assert_eq!(result.accepted(), 1);
        assert_eq!(state.clauses(), 1);
        assert_eq!(state.pos_units().len(), 1);
        assert!(result
            .trace()
            .starts_with("% CSSCPAState: requested  by 0, 0, 0, 0"));
        assert!(result.trace().contains("accepted from 2 (forced)"));
        assert!(result.trace().contains("rejected (subsumed by"));
        assert!(result
            .trace()
            .ends_with(" (system, clauses,literals,weight)\n"));
        let flush_offsets = result.trace_flush_offsets();
        assert_eq!(flush_offsets.len(), 4);
        assert!(flush_offsets
            .windows(2)
            .all(|window| window[0] <= window[1]));
        let first_flush_segment = result
            .trace()
            .get(..flush_offsets[0])
            .expect("first CSSCPA flush offset is a string boundary");
        assert!(first_flush_segment.ends_with("0, 0, 0 (system, clauses,literals,weight)\n"));
        assert_eq!(flush_offsets.last().copied(), Some(result.trace().len()));
    }

    #[test]
    fn loop_output_level_accepts_only_zero_or_one_as_state_changes() {
        let mut state = CsscpaState::new().expect("CSSCPA state allocation");
        let mut scanner = Scanner::from_user_string(
            "output_level 2
accept: cnf(csscpa_hidden,axiom,p(a)).",
            false,
        )
        .expect("CSSCPA loop scanner allocation");
        scanner.set_format(IoFormat::Tstp);

        let result = state
            .process_loop(&mut scanner, 0)
            .expect("CSSCPA loop parses output_level command");

        assert_eq!(result.output_level(), 0);
        assert_eq!(result.processed(), 1);
        assert_eq!(result.accepted(), 1);
        assert!(result.trace().is_empty());
        assert_eq!(result.trace_flush_offsets(), &[0]);
    }

    #[test]
    fn loop_rejects_csscpa_source_outside_c_range() {
        let mut state = CsscpaState::new().expect("CSSCPA state allocation");
        let mut scanner =
            Scanner::from_user_string("accept from 1: cnf(csscpa_bad,axiom,p(a)).", false)
                .expect("CSSCPA loop scanner allocation");
        scanner.set_format(IoFormat::Tstp);

        let error = state.process_loop(&mut scanner, 1).unwrap_err();

        assert_eq!(error.code(), ErrorCode::SYNTAX_ERROR);
        assert!(error
            .message()
            .contains("CSSCPA source specifier must be in the range 2...15"));
    }

    #[test]
    fn loop_dispatches_to_current_scanner_clause_format() {
        let mut state = CsscpaState::new().expect("CSSCPA state allocation");
        let mut scanner =
            Scanner::from_user_string("accept: input_clause(c_0_1,axiom,[++p(a)]).", false)
                .expect("CSSCPA loop scanner allocation");
        scanner.set_format(IoFormat::Tptp);

        let result = state
            .process_loop(&mut scanner, 1)
            .expect("CSSCPA loop parses old TPTP input clause");

        assert_eq!(result.processed(), 1);
        assert_eq!(result.accepted(), 1);
        assert_eq!(state.clauses(), 1);
        assert!(result.trace().contains("accepted from 0 (forced)"));
    }

    #[test]
    fn loop_accepts_old_tptp_input_clause_under_tstp_filter_mode() {
        let mut state = CsscpaState::new().expect("CSSCPA state allocation");
        let mut scanner =
            Scanner::from_user_string("accept: input_clause(c_0_1,axiom,[++p(a)]).", false)
                .expect("CSSCPA loop scanner allocation");
        scanner.set_format(IoFormat::Tstp);

        let result = state
            .process_loop(&mut scanner, 1)
            .expect("CSSCPA loop parses old TPTP input clause under filter mode");

        assert_eq!(scanner.format(), IoFormat::Tstp);
        assert_eq!(result.processed(), 1);
        assert_eq!(state.clauses(), 1);
        assert!(result.trace().contains("accepted from 0 (forced)"));
    }
}
