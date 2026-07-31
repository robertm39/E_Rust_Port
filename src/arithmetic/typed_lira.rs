//! Typed Umlaut-AST to linear integer/real arithmetic import.
//!
//! This is the production form of the conservative contract frozen by
//! `experiments/2026-07-29-023-typed-tptp-lira-adapter`. Parsing and TPTP type
//! inference remain owned by Umlaut; this module only lowers an already typed
//! formula and rejects everything outside the exact base VIRAS fragment.

use super::viras::{
    add, atom, boolean, ceil_term, conjunction, constant, disjunction, floor_term, negate, scale,
    subtract, Formula, Literal, Rational, Relation, Term as LiraTerm,
};
use crate::basics::error::{Diagnostic, ErrorCode};
use crate::basics::simple_stuff::{
    problem_type, reset_problem_type, set_problem_type, ProblemType,
};
use crate::inout::scanner::{Scanner, TokenType};
use crate::terms::functypes::FunCode;
use crate::terms::signature::{
    PredefinedArithmeticSymbol, Signature, FP_IS_FLOAT, FP_IS_INTEGER, FP_IS_RATIONAL,
    SIG_FALSE_CODE, SIG_TRUE_CODE,
};
use crate::terms::simpletypes::{ST_INTEGER, ST_RATIONAL, ST_REAL};
use crate::terms::termbanks::TermBank;
use crate::terms::termfunc::var_print_string;
use crate::terms::termtypes::Term;
use crate::terms::typebanks::TypeBank;
use num_bigint::BigInt;
use num_traits::{One, Zero};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::str::FromStr;

const MAX_DECIMAL_POWER: u32 = 16_384;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NumericSort {
    Integer,
    Rational,
    Real,
}

impl NumericSort {
    #[must_use]
    pub const fn tptp_name(self) -> &'static str {
        match self {
            Self::Integer => "$int",
            Self::Rational => "$rat",
            Self::Real => "$real",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ImportErrorCode {
    UnsupportedRatQuantifier,
    UnsupportedRealToRat,
    UnsupportedRealRationality,
    NonlinearProduct,
    NonconstantDivisor,
    ZeroDivisor,
    UnsupportedRounding,
    UnsupportedOperator,
    UninterpretedArithmetic,
    TypeMismatch,
    UnboundVariable,
    UnsupportedDialect,
    UnsupportedDocument,
    UnsupportedRole,
    UnsupportedSort,
    UnsupportedConnective,
    ArityMismatch,
    ResourceLimit,
    MalformedInput,
}

impl ImportErrorCode {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::UnsupportedRatQuantifier => "UNSUPPORTED_RAT_QUANTIFIER",
            Self::UnsupportedRealToRat => "UNSUPPORTED_REAL_TO_RAT",
            Self::UnsupportedRealRationality => "UNSUPPORTED_REAL_RATIONALITY",
            Self::NonlinearProduct => "NONLINEAR_PRODUCT",
            Self::NonconstantDivisor => "NONCONSTANT_DIVISOR",
            Self::ZeroDivisor => "ZERO_DIVISOR",
            Self::UnsupportedRounding => "UNSUPPORTED_ROUNDING",
            Self::UnsupportedOperator => "UNSUPPORTED_OPERATOR",
            Self::UninterpretedArithmetic => "UNINTERPRETED_ARITHMETIC",
            Self::TypeMismatch => "TYPE_MISMATCH",
            Self::UnboundVariable => "UNBOUND_VARIABLE",
            Self::UnsupportedDialect => "UNSUPPORTED_DIALECT",
            Self::UnsupportedDocument => "UNSUPPORTED_DOCUMENT",
            Self::UnsupportedRole => "UNSUPPORTED_ROLE",
            Self::UnsupportedSort => "UNSUPPORTED_SORT",
            Self::UnsupportedConnective => "UNSUPPORTED_CONNECTIVE",
            Self::ArityMismatch => "ARITY_MISMATCH",
            Self::ResourceLimit => "RESOURCE_LIMIT",
            Self::MalformedInput => "MALFORMED_INPUT",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ImportError {
    pub code: ImportErrorCode,
    pub message: String,
}

impl ImportError {
    fn new(code: ImportErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

impl fmt::Display for ImportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.code.as_str(), self.message)
    }
}

impl std::error::Error for ImportError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ImportTraceStep {
    pub kind: &'static str,
    pub source: String,
    pub target: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ImportResult {
    pub formula: Formula,
    pub trace: Vec<ImportTraceStep>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ImportedDocument {
    pub source_name: String,
    pub source_role: String,
    pub import: ImportResult,
}

#[derive(Clone, Debug)]
struct Binding {
    target: String,
    sort: NumericSort,
}

#[derive(Clone, Debug)]
struct TypedTerm {
    sort: NumericSort,
    term: LiraTerm,
}

pub struct Importer<'a> {
    signature: &'a Signature,
    max_rational_bits: u64,
    bindings: BTreeMap<FunCode, Binding>,
    variable_counter: usize,
    trace: Vec<ImportTraceStep>,
}

impl<'a> Importer<'a> {
    #[must_use]
    pub fn new(signature: &'a Signature) -> Self {
        Self::with_max_rational_bits(signature, 4_096)
    }

    #[must_use]
    pub fn with_max_rational_bits(signature: &'a Signature, max_rational_bits: u64) -> Self {
        Self {
            signature,
            max_rational_bits,
            bindings: BTreeMap::new(),
            variable_counter: 0,
            trace: Vec::new(),
        }
    }

    /// Imports one closed, already typed formula term.
    ///
    /// # Errors
    ///
    /// Returns a stable fail-closed code for every term, sort, connective, or
    /// binding outside the conservative LIRA contract.
    pub fn import(mut self, formula: &Term) -> Result<ImportResult, ImportError> {
        let formula = self.translate_formula(formula, false)?;
        if !self.bindings.is_empty() {
            return Err(ImportError::new(
                ImportErrorCode::MalformedInput,
                "import ended with an unclosed binder scope",
            ));
        }
        Ok(ImportResult {
            formula,
            trace: self.trace,
        })
    }

    fn record(&mut self, kind: &'static str, source: impl Into<String>, target: impl Into<String>) {
        self.trace.push(ImportTraceStep {
            kind,
            source: source.into(),
            target: target.into(),
        });
    }

    #[allow(
        clippy::unused_self,
        reason = "keeping AST access on the importer makes every malformed-node exit use one boundary"
    )]
    fn argument(&self, term: &Term, index: usize) -> Result<Term, ImportError> {
        term.argument(index).ok_or_else(|| {
            ImportError::new(
                ImportErrorCode::MalformedInput,
                format!("term argument {index} is uninitialized"),
            )
        })
    }

    fn require_arity(&self, term: &Term, expected: usize) -> Result<(), ImportError> {
        if term.arity() == expected {
            Ok(())
        } else {
            Err(ImportError::new(
                ImportErrorCode::ArityMismatch,
                format!(
                    "{} expects {expected} arguments, found {}",
                    self.symbol_name(term),
                    term.arity()
                ),
            ))
        }
    }

    fn symbol_name(&self, term: &Term) -> String {
        if term.is_any_var() {
            var_print_string(term.f_code())
        } else {
            self.signature
                .find_name(term.f_code())
                .unwrap_or("<unknown>")
                .to_owned()
        }
    }

    fn sort_from_term(&self, term: &Term) -> Result<NumericSort, ImportError> {
        let type_ = term.type_().ok_or_else(|| {
            ImportError::new(
                ImportErrorCode::TypeMismatch,
                format!("{} has no inferred type", self.symbol_name(term)),
            )
        })?;
        match type_.f_code() {
            ST_INTEGER => Ok(NumericSort::Integer),
            ST_RATIONAL => Ok(NumericSort::Rational),
            ST_REAL => Ok(NumericSort::Real),
            code => Err(ImportError::new(
                ImportErrorCode::UnsupportedSort,
                format!("unsupported arithmetic sort code {code}"),
            )),
        }
    }

    #[allow(
        clippy::too_many_lines,
        reason = "the exhaustive logical-symbol lowering is kept together to audit NNF polarity"
    )]
    fn translate_formula(&mut self, formula: &Term, negated: bool) -> Result<Formula, ImportError> {
        let code = formula.f_code();
        if code == SIG_TRUE_CODE {
            return Ok(boolean(!negated));
        }
        if code == SIG_FALSE_CODE {
            return Ok(boolean(negated));
        }
        if code == self.signature.not_code() {
            self.require_arity(formula, 1)?;
            return self.translate_formula(&self.argument(formula, 0)?, !negated);
        }
        if code == self.signature.qex_code() || code == self.signature.qall_code() {
            return self.translate_quantifier(formula, negated);
        }
        if code == self.signature.and_code() || code == self.signature.or_code() {
            self.require_arity(formula, 2)?;
            let source_is_and = code == self.signature.and_code();
            let target_is_and = source_is_and != negated;
            let left = self.translate_formula(&self.argument(formula, 0)?, negated)?;
            let right = self.translate_formula(&self.argument(formula, 1)?, negated)?;
            return Ok(if target_is_and {
                conjunction([left, right])
            } else {
                disjunction([left, right])
            });
        }
        if code == self.signature.impl_code() || code == self.signature.bimpl_code() {
            self.require_arity(formula, 2)?;
            let mut left = self.argument(formula, 0)?;
            let mut right = self.argument(formula, 1)?;
            if code == self.signature.bimpl_code() {
                std::mem::swap(&mut left, &mut right);
            }
            return if negated {
                Ok(conjunction([
                    self.translate_formula(&left, false)?,
                    self.translate_formula(&right, true)?,
                ]))
            } else {
                Ok(disjunction([
                    self.translate_formula(&left, true)?,
                    self.translate_formula(&right, false)?,
                ]))
            };
        }
        if code == self.signature.equiv_code() || code == self.signature.xor_code() {
            self.require_arity(formula, 2)?;
            let left = self.argument(formula, 0)?;
            let right = self.argument(formula, 1)?;
            let xor = negated ^ (code == self.signature.xor_code());
            return if xor {
                Ok(disjunction([
                    conjunction([
                        self.translate_formula(&left, false)?,
                        self.translate_formula(&right, true)?,
                    ]),
                    conjunction([
                        self.translate_formula(&left, true)?,
                        self.translate_formula(&right, false)?,
                    ]),
                ]))
            } else {
                Ok(disjunction([
                    conjunction([
                        self.translate_formula(&left, false)?,
                        self.translate_formula(&right, false)?,
                    ]),
                    conjunction([
                        self.translate_formula(&left, true)?,
                        self.translate_formula(&right, true)?,
                    ]),
                ]))
            };
        }
        if code == self.signature.nand_code() || code == self.signature.nor_code() {
            self.require_arity(formula, 2)?;
            let source_is_and = code == self.signature.nand_code();
            let child_negated = !negated;
            let target_is_and = source_is_and != child_negated;
            let left = self.translate_formula(&self.argument(formula, 0)?, child_negated)?;
            let right = self.translate_formula(&self.argument(formula, 1)?, child_negated)?;
            return Ok(if target_is_and {
                conjunction([left, right])
            } else {
                disjunction([left, right])
            });
        }
        if code == self.signature.eqn_code() || code == self.signature.neqn_code() {
            self.require_arity(formula, 2)?;
            let left_term = self.argument(formula, 0)?;
            let right_term = self.argument(formula, 1)?;
            let wrapped_predicate = if matches!(right_term.f_code(), SIG_TRUE_CODE | SIG_FALSE_CODE)
            {
                Some((left_term.clone(), right_term.f_code() == SIG_FALSE_CODE))
            } else if matches!(left_term.f_code(), SIG_TRUE_CODE | SIG_FALSE_CODE) {
                Some((right_term.clone(), left_term.f_code() == SIG_FALSE_CODE))
            } else {
                None
            };
            if let Some((predicate, compared_with_false)) = wrapped_predicate {
                let wrapper_negated =
                    negated ^ (code == self.signature.neqn_code()) ^ compared_with_false;
                return self.translate_formula(&predicate, wrapper_negated);
            }
            let left = self.translate_term(&left_term)?;
            let right = self.translate_term(&right_term)?;
            self.require_same_sort(left.sort, right.sort, "equality")?;
            let relation = if (code == self.signature.eqn_code()) ^ negated {
                Relation::Eq
            } else {
                Relation::Ne
            };
            self.record(
                "relation",
                if code == self.signature.eqn_code() {
                    "="
                } else {
                    "!="
                },
                match relation {
                    Relation::Eq => "eq",
                    Relation::Ne => "ne",
                    Relation::Gt | Relation::Ge => unreachable!("equality normalization"),
                },
            );
            return Ok(atom(Literal::new(
                subtract(left.term, right.term),
                relation,
            )));
        }
        if let Some(symbol) = self.signature.predefined_arithmetic_symbol(code) {
            return self.translate_predicate(formula, symbol, negated);
        }
        Err(ImportError::new(
            ImportErrorCode::UninterpretedArithmetic,
            format!(
                "Boolean-valued uninterpreted symbol {} is outside pure arithmetic",
                self.symbol_name(formula)
            ),
        ))
    }

    fn translate_quantifier(
        &mut self,
        formula: &Term,
        negated: bool,
    ) -> Result<Formula, ImportError> {
        self.require_arity(formula, 2)?;
        let variable = self.argument(formula, 0)?;
        if !variable.is_free_var() {
            return Err(ImportError::new(
                ImportErrorCode::MalformedInput,
                "quantifier binder is not a named variable",
            ));
        }
        let sort = self.sort_from_term(&variable)?;
        if sort == NumericSort::Rational {
            return Err(ImportError::new(
                ImportErrorCode::UnsupportedRatQuantifier,
                "quantified rationals are not representable in LIRA",
            ));
        }
        self.variable_counter = self.variable_counter.saturating_add(1);
        let target = format!("LIRA_V{}", self.variable_counter);
        let source = var_print_string(variable.f_code());
        let prior = self.bindings.insert(
            variable.f_code(),
            Binding {
                target: target.clone(),
                sort,
            },
        );
        self.record(
            "binder",
            format!("{source}:{}", sort.tptp_name()),
            format!(
                "{target}:$real:{}",
                if sort == NumericSort::Integer {
                    "integrality_guard"
                } else {
                    "direct"
                }
            ),
        );
        let body_result = self.translate_formula(&self.argument(formula, 1)?, negated);
        if let Some(binding) = prior {
            self.bindings.insert(variable.f_code(), binding);
        } else {
            self.bindings.remove(&variable.f_code());
        }
        let mut body = body_result?;
        let source_exists = formula.f_code() == self.signature.qex_code();
        let target_exists = source_exists != negated;
        if sort == NumericSort::Integer {
            let lira_variable = LiraTerm::Variable(target.clone());
            let guard = atom(Literal::new(
                subtract(lira_variable.clone(), floor_term(lira_variable)),
                Relation::Eq,
            ));
            body = if target_exists {
                conjunction([guard, body])
            } else {
                disjunction([super::viras::negate_formula(guard), body])
            };
        }
        Ok(if target_exists {
            Formula::Exists(target, Box::new(body))
        } else {
            Formula::Forall(target, Box::new(body))
        })
    }

    fn translate_predicate(
        &mut self,
        formula: &Term,
        symbol: PredefinedArithmeticSymbol,
        negated: bool,
    ) -> Result<Formula, ImportError> {
        use PredefinedArithmeticSymbol as Symbol;
        match symbol {
            Symbol::IsInt | Symbol::IsRat => {
                self.require_arity(formula, 1)?;
                let argument = self.translate_term(&self.argument(formula, 0)?)?;
                let result = if symbol == Symbol::IsRat {
                    if argument.sort == NumericSort::Real {
                        return Err(ImportError::new(
                            ImportErrorCode::UnsupportedRealRationality,
                            "$is_rat on a real is outside LIRA",
                        ));
                    }
                    self.record("predicate", symbol.name(), "true");
                    boolean(true)
                } else if argument.sort == NumericSort::Integer {
                    self.record("predicate", symbol.name(), "true");
                    boolean(true)
                } else {
                    self.record("predicate", symbol.name(), "X=floor(X)");
                    atom(Literal::new(
                        subtract(argument.term.clone(), floor_term(argument.term)),
                        Relation::Eq,
                    ))
                };
                return Ok(if negated {
                    super::viras::negate_formula(result)
                } else {
                    result
                });
            }
            Symbol::Less | Symbol::LessEq | Symbol::Greater | Symbol::GreaterEq => {}
            _ => {
                return Err(ImportError::new(
                    ImportErrorCode::UnsupportedOperator,
                    format!("{} is not a formula predicate", symbol.name()),
                ));
            }
        }
        self.require_arity(formula, 2)?;
        let left = self.translate_term(&self.argument(formula, 0)?)?;
        let right = self.translate_term(&self.argument(formula, 1)?)?;
        self.require_same_sort(left.sort, right.sort, symbol.name())?;
        let difference = subtract(left.term, right.term);
        let (term, relation) = match symbol {
            Symbol::Greater => (difference, Relation::Gt),
            Symbol::GreaterEq => (difference, Relation::Ge),
            Symbol::Less => (negate(difference), Relation::Gt),
            Symbol::LessEq => (negate(difference), Relation::Ge),
            _ => unreachable!("comparison set checked above"),
        };
        self.record("relation", symbol.name(), relation_name(relation));
        let result = atom(Literal::new(term, relation));
        Ok(if negated {
            super::viras::negate_formula(result)
        } else {
            result
        })
    }

    fn translate_term(&mut self, term: &Term) -> Result<TypedTerm, ImportError> {
        if term.is_free_var() {
            let binding = self.bindings.get(&term.f_code()).cloned().ok_or_else(|| {
                ImportError::new(
                    ImportErrorCode::UnboundVariable,
                    format!("unbound variable {}", var_print_string(term.f_code())),
                )
            })?;
            let actual_sort = self.sort_from_term(term)?;
            self.require_same_sort(binding.sort, actual_sort, "variable occurrence")?;
            return Ok(TypedTerm {
                sort: binding.sort,
                term: LiraTerm::Variable(binding.target),
            });
        }
        if term.is_db_var() {
            return Err(ImportError::new(
                ImportErrorCode::UnboundVariable,
                "de Bruijn variables are outside the typed TFF importer",
            ));
        }
        if term.arity() == 0 {
            if let Some(number) = self.parse_numeric_literal(term)? {
                return Ok(number);
            }
            return Err(ImportError::new(
                ImportErrorCode::UninterpretedArithmetic,
                format!(
                    "arithmetic-valued uninterpreted constant {}",
                    self.symbol_name(term)
                ),
            ));
        }
        let Some(symbol) = self.signature.predefined_arithmetic_symbol(term.f_code()) else {
            return Err(ImportError::new(
                ImportErrorCode::UninterpretedArithmetic,
                format!(
                    "arithmetic-valued uninterpreted function {}",
                    self.symbol_name(term)
                ),
            ));
        };
        self.translate_arithmetic_term(term, symbol)
    }

    fn parse_numeric_literal(&self, term: &Term) -> Result<Option<TypedTerm>, ImportError> {
        let (sort, text) = if self.signature.query_prop(term.f_code(), FP_IS_INTEGER) {
            (
                NumericSort::Integer,
                self.signature.find_name(term.f_code()).unwrap_or(""),
            )
        } else if self.signature.query_prop(term.f_code(), FP_IS_RATIONAL) {
            (
                NumericSort::Rational,
                self.signature.find_name(term.f_code()).unwrap_or(""),
            )
        } else if self.signature.query_prop(term.f_code(), FP_IS_FLOAT) {
            (
                NumericSort::Real,
                self.signature.find_name(term.f_code()).unwrap_or(""),
            )
        } else {
            return Ok(None);
        };
        let value = parse_exact_number(text)?;
        let bits = value.numer().magnitude().bits().max(value.denom().bits());
        if bits > self.max_rational_bits {
            return Err(ImportError::new(
                ImportErrorCode::ResourceLimit,
                format!(
                    "rational bit limit exceeded during import: \
                     {bits}>{}",
                    self.max_rational_bits
                ),
            ));
        }
        Ok(Some(TypedTerm {
            sort,
            term: constant(value),
        }))
    }

    #[allow(
        clippy::too_many_lines,
        reason = "the exact accepted and rejected arithmetic operator contract is one exhaustive match"
    )]
    fn translate_arithmetic_term(
        &mut self,
        term: &Term,
        symbol: PredefinedArithmeticSymbol,
    ) -> Result<TypedTerm, ImportError> {
        use PredefinedArithmeticSymbol as Symbol;
        match symbol {
            Symbol::UMinus
            | Symbol::Floor
            | Symbol::Ceiling
            | Symbol::ToInt
            | Symbol::ToRat
            | Symbol::ToReal => {
                self.require_arity(term, 1)?;
                let argument = self.translate_term(&self.argument(term, 0)?)?;
                let result = match symbol {
                    Symbol::UMinus => TypedTerm {
                        sort: argument.sort,
                        term: negate(argument.term),
                    },
                    Symbol::Floor => TypedTerm {
                        sort: argument.sort,
                        term: if argument.sort == NumericSort::Integer {
                            argument.term
                        } else {
                            floor_term(argument.term)
                        },
                    },
                    Symbol::Ceiling => TypedTerm {
                        sort: argument.sort,
                        term: if argument.sort == NumericSort::Integer {
                            argument.term
                        } else {
                            ceil_term(argument.term)
                        },
                    },
                    Symbol::ToInt => TypedTerm {
                        sort: NumericSort::Integer,
                        term: if argument.sort == NumericSort::Integer {
                            argument.term
                        } else {
                            floor_term(argument.term)
                        },
                    },
                    Symbol::ToRat => {
                        if argument.sort == NumericSort::Real {
                            return Err(ImportError::new(
                                ImportErrorCode::UnsupportedRealToRat,
                                "$to_rat from real is outside the conservative contract",
                            ));
                        }
                        TypedTerm {
                            sort: NumericSort::Rational,
                            term: argument.term,
                        }
                    }
                    Symbol::ToReal => TypedTerm {
                        sort: NumericSort::Real,
                        term: argument.term,
                    },
                    _ => unreachable!("unary set checked above"),
                };
                self.record("term", symbol.name(), result.term.render());
                Ok(result)
            }
            Symbol::Truncate | Symbol::Round => Err(ImportError::new(
                ImportErrorCode::UnsupportedRounding,
                format!("unsupported rounding operator {}", symbol.name()),
            )),
            Symbol::Sum | Symbol::Difference | Symbol::Product | Symbol::Quotient => {
                self.require_arity(term, 2)?;
                let left = self.translate_term(&self.argument(term, 0)?)?;
                let right = self.translate_term(&self.argument(term, 1)?)?;
                self.require_same_sort(left.sort, right.sort, symbol.name())?;
                let result = match symbol {
                    Symbol::Sum => TypedTerm {
                        sort: left.sort,
                        term: add([left.term, right.term]),
                    },
                    Symbol::Difference => TypedTerm {
                        sort: left.sort,
                        term: subtract(left.term, right.term),
                    },
                    Symbol::Product => {
                        let left_constant = constant_value(&left.term);
                        let right_constant = constant_value(&right.term);
                        if let Some(coefficient) = left_constant {
                            TypedTerm {
                                sort: left.sort,
                                term: scale(coefficient, right.term),
                            }
                        } else if let Some(coefficient) = right_constant {
                            TypedTerm {
                                sort: left.sort,
                                term: scale(coefficient, left.term),
                            }
                        } else {
                            return Err(ImportError::new(
                                ImportErrorCode::NonlinearProduct,
                                "product requires a compile-time rational factor",
                            ));
                        }
                    }
                    Symbol::Quotient => {
                        let divisor = constant_value(&right.term).ok_or_else(|| {
                            ImportError::new(
                                ImportErrorCode::NonconstantDivisor,
                                "quotient requires a compile-time rational divisor",
                            )
                        })?;
                        if divisor.is_zero() {
                            return Err(ImportError::new(
                                ImportErrorCode::ZeroDivisor,
                                "division by zero is unspecified in TPTP arithmetic",
                            ));
                        }
                        TypedTerm {
                            sort: if left.sort == NumericSort::Integer {
                                NumericSort::Rational
                            } else {
                                left.sort
                            },
                            term: scale(Rational::one() / divisor, left.term),
                        }
                    }
                    _ => unreachable!("binary set checked above"),
                };
                self.record("term", symbol.name(), result.term.render());
                Ok(result)
            }
            Symbol::QuotientE
            | Symbol::QuotientT
            | Symbol::QuotientF
            | Symbol::RemainderE
            | Symbol::RemainderT
            | Symbol::RemainderF
            | Symbol::Abs => Err(ImportError::new(
                ImportErrorCode::UnsupportedOperator,
                format!("unsupported operator {}", symbol.name()),
            )),
            Symbol::Less
            | Symbol::LessEq
            | Symbol::Greater
            | Symbol::GreaterEq
            | Symbol::IsInt
            | Symbol::IsRat => Err(ImportError::new(
                ImportErrorCode::UnsupportedOperator,
                format!("predicate {} used as a term", symbol.name()),
            )),
        }
    }

    #[allow(
        clippy::unused_self,
        reason = "sort checks are importer boundary operations and retain consistent method call sites"
    )]
    fn require_same_sort(
        &self,
        left: NumericSort,
        right: NumericSort,
        context: &str,
    ) -> Result<(), ImportError> {
        if left == right {
            Ok(())
        } else {
            Err(ImportError::new(
                ImportErrorCode::TypeMismatch,
                format!(
                    "{context} requires matching sorts, found {} and {}",
                    left.tptp_name(),
                    right.tptp_name()
                ),
            ))
        }
    }
}

fn relation_name(relation: Relation) -> &'static str {
    match relation {
        Relation::Eq => "eq",
        Relation::Ne => "ne",
        Relation::Gt => "gt",
        Relation::Ge => "ge",
    }
}

fn constant_value(term: &LiraTerm) -> Option<Rational> {
    match term {
        LiraTerm::Constant(value) => Some(value.clone()),
        _ => None,
    }
}

fn parse_exact_number(text: &str) -> Result<Rational, ImportError> {
    if let Some((numerator, denominator)) = text.split_once('/') {
        let numerator = parse_bigint(numerator)?;
        let denominator = parse_bigint(denominator)?;
        if denominator.is_zero() {
            return Err(ImportError::new(
                ImportErrorCode::MalformedInput,
                "numeric literal has a zero denominator",
            ));
        }
        return Ok(Rational::new(numerator, denominator));
    }
    if !text.contains('.') && !text.contains('e') && !text.contains('E') {
        return parse_bigint(text).map(Rational::from_integer);
    }

    let (mantissa, exponent) = text
        .split_once(['e', 'E'])
        .map_or((text, 0_i64), |(mantissa, exponent)| {
            (mantissa, i64::from_str(exponent).unwrap_or(i64::MIN))
        });
    if exponent == i64::MIN {
        return Err(ImportError::new(
            ImportErrorCode::MalformedInput,
            format!("invalid decimal exponent in {text}"),
        ));
    }
    let negative = mantissa.starts_with('-');
    let unsigned = mantissa.strip_prefix(['+', '-']).unwrap_or(mantissa);
    let (whole, fractional) = unsigned.split_once('.').unwrap_or((unsigned, ""));
    let digits = format!("{whole}{fractional}");
    let mut numerator = parse_bigint(&digits)?;
    if negative {
        numerator = -numerator;
    }
    let fractional_digits = i64::try_from(fractional.len()).map_err(|_| {
        ImportError::new(
            ImportErrorCode::MalformedInput,
            "decimal literal is too long",
        )
    })?;
    let scale = fractional_digits.checked_sub(exponent).ok_or_else(|| {
        ImportError::new(
            ImportErrorCode::MalformedInput,
            "decimal exponent is out of range",
        )
    })?;
    if scale >= 0 {
        let power = u32::try_from(scale).map_err(|_| {
            ImportError::new(
                ImportErrorCode::MalformedInput,
                "decimal denominator exponent is out of range",
            )
        })?;
        if power > MAX_DECIMAL_POWER {
            return Err(ImportError::new(
                ImportErrorCode::MalformedInput,
                "decimal denominator exponent exceeds the importer safety limit",
            ));
        }
        Ok(Rational::new(numerator, BigInt::from(10_u8).pow(power)))
    } else {
        let power = u32::try_from(scale.unsigned_abs()).map_err(|_| {
            ImportError::new(
                ImportErrorCode::MalformedInput,
                "decimal numerator exponent is out of range",
            )
        })?;
        if power > MAX_DECIMAL_POWER {
            return Err(ImportError::new(
                ImportErrorCode::MalformedInput,
                "decimal numerator exponent exceeds the importer safety limit",
            ));
        }
        Ok(Rational::from_integer(
            numerator * BigInt::from(10_u8).pow(power),
        ))
    }
}

fn parse_bigint(text: &str) -> Result<BigInt, ImportError> {
    BigInt::from_str(text).map_err(|_| {
        ImportError::new(
            ImportErrorCode::MalformedInput,
            format!("invalid numeric literal {text}"),
        )
    })
}

/// Imports one typed formula with a fresh alpha-renaming context.
///
/// # Errors
///
/// Returns the stable conservative importer rejection for unsupported input.
pub fn import_formula(formula: &Term, signature: &Signature) -> Result<ImportResult, ImportError> {
    Importer::new(signature).import(formula)
}

/// Imports one typed formula under an explicit exact-rational bit limit.
///
/// # Errors
///
/// Returns `RESOURCE_LIMIT` before arithmetic lowering can publish a result
/// when any numeric literal exceeds the limit.
pub fn import_formula_with_max_rational_bits(
    formula: &Term,
    signature: &Signature,
    max_rational_bits: u64,
) -> Result<ImportResult, ImportError> {
    Importer::with_max_rational_bits(signature, max_rational_bits).import(formula)
}

struct ProblemTypeScope {
    previous: ProblemType,
}

impl ProblemTypeScope {
    fn first_order() -> Result<Self, ImportError> {
        let previous = problem_type();
        reset_problem_type();
        set_problem_type(ProblemType::FirstOrder).map_err(map_parser_diagnostic)?;
        Ok(Self { previous })
    }
}

impl Drop for ProblemTypeScope {
    fn drop(&mut self) {
        reset_problem_type();
        if self.previous != ProblemType::NotInitialized {
            let _ = set_problem_type(self.previous);
        }
    }
}

#[allow(
    clippy::needless_pass_by_value,
    reason = "parser diagnostics are consumed at the boundary and converted into owned stable errors"
)]
fn map_parser_diagnostic(diagnostic: Diagnostic) -> ImportError {
    let code = if diagnostic.code() == ErrorCode::TYPE_ERROR {
        ImportErrorCode::TypeMismatch
    } else {
        ImportErrorCode::MalformedInput
    };
    ImportError::new(code, diagnostic.message())
}

/// Parses and imports exactly one closed `tff` axiom or conjecture.
///
/// This deliberately narrow document gate exists for the opt-in arithmetic
/// executable. General Umlaut parsing remains unchanged.
///
/// # Errors
///
/// Returns `UNSUPPORTED_DIALECT`, `UNSUPPORTED_DOCUMENT`, or
/// `UNSUPPORTED_ROLE` at the wrapper boundary, parser/type errors under the
/// stable importer taxonomy, and otherwise the AST import error.
pub fn import_document(source: &str) -> Result<ImportedDocument, ImportError> {
    import_document_with_max_rational_bits(source, 4_096)
}

/// Parses and imports one supported document under a rational-bit limit.
///
/// # Errors
///
/// Returns the same stable document/import taxonomy as [`import_document`],
/// including `RESOURCE_LIMIT` for an oversized exact numeric literal.
pub fn import_document_with_max_rational_bits(
    source: &str,
    max_rational_bits: u64,
) -> Result<ImportedDocument, ImportError> {
    let _problem_type = ProblemTypeScope::first_order()?;
    if let Some(variable) = first_lexically_unbound_variable(source)? {
        return Err(ImportError::new(
            ImportErrorCode::UnboundVariable,
            format!("unbound variable {variable}"),
        ));
    }
    let mut scanner = Scanner::from_user_string(source, false).map_err(map_parser_diagnostic)?;
    if !scanner.test_id("tff") {
        return Err(ImportError::new(
            ImportErrorCode::UnsupportedDialect,
            format!(
                "only tff is supported, found {}",
                scanner.current_token().literal()
            ),
        ));
    }
    scanner.accept_id("tff").map_err(map_parser_diagnostic)?;
    scanner
        .accept_tok(TokenType::OPEN_BRACKET)
        .map_err(map_parser_diagnostic)?;
    let source_name = scanner.current_token().literal();
    scanner
        .accept_tok(TokenType::NAME | TokenType::POS_INT)
        .map_err(map_parser_diagnostic)?;
    scanner
        .accept_tok(TokenType::COMMA)
        .map_err(map_parser_diagnostic)?;
    let source_role = scanner.current_token().literal();
    if !matches!(source_role.as_str(), "axiom" | "conjecture") {
        return Err(ImportError::new(
            ImportErrorCode::UnsupportedRole,
            format!("unsupported TFF role {source_role}"),
        ));
    }
    scanner
        .accept_tok(TokenType::IDENT)
        .map_err(map_parser_diagnostic)?;
    scanner
        .accept_tok(TokenType::COMMA)
        .map_err(map_parser_diagnostic)?;

    let mut signature = Signature::new(TypeBank::new());
    signature
        .insert_internal_codes()
        .map_err(map_parser_diagnostic)?;
    signature.set_typed_symbols(true);
    let mut bank = TermBank::new(signature).map_err(map_parser_diagnostic)?;
    let formula = bank
        .parse_tformula_tstp(&mut scanner)
        .map_err(map_parser_diagnostic)?;
    scanner
        .accept_tok(TokenType::CLOSE_BRACKET)
        .map_err(map_parser_diagnostic)?;
    scanner
        .accept_tok(TokenType::FULLSTOP)
        .map_err(map_parser_diagnostic)?;
    if !scanner.test_tok(TokenType::NO_TOKEN) {
        return Err(ImportError::new(
            ImportErrorCode::UnsupportedDocument,
            "the arithmetic importer accepts exactly one annotated formula",
        ));
    }
    let import =
        import_formula_with_max_rational_bits(&formula, bank.signature(), max_rational_bits)?;
    Ok(ImportedDocument {
        source_name,
        source_role,
        import,
    })
}

fn first_lexically_unbound_variable(source: &str) -> Result<Option<String>, ImportError> {
    let mut scanner = Scanner::from_user_string(source, false).map_err(map_parser_diagnostic)?;
    let mut occurrences = BTreeSet::new();
    let mut binders = BTreeSet::new();
    while !scanner.test_tok(TokenType::NO_TOKEN) {
        let literal = scanner.current_token().literal();
        let exponent_suffix = literal.strip_prefix('E').is_some_and(|suffix| {
            !suffix.is_empty() && suffix.bytes().all(|byte| byte.is_ascii_digit())
        });
        let is_variable = scanner.test_tok(TokenType::IDENTIFIER)
            && !exponent_suffix
            && literal
                .as_bytes()
                .first()
                .is_some_and(u8::is_ascii_uppercase);
        if is_variable {
            occurrences.insert(literal.clone());
            if scanner.look_token(1).kind() == TokenType::COLON && !binders.insert(literal.clone())
            {
                return Err(ImportError::new(
                    ImportErrorCode::MalformedInput,
                    format!("duplicate quantified variable {literal}"),
                ));
            }
        }
        scanner.next_token().map_err(map_parser_diagnostic)?;
    }
    Ok(occurrences
        .into_iter()
        .find(|variable| !binders.contains(variable)))
}

fn render_rational(value: &Rational) -> String {
    let exact = if value.is_integer() {
        value.to_integer().to_string()
    } else {
        format!("{}/{}", value.numer(), value.denom())
    };
    format!("$to_real({exact})")
}

/// Renders an exact LIRA term through the accepted real-sorted TFF surface.
#[must_use]
pub fn render_tff_term(term: &LiraTerm) -> String {
    match term {
        LiraTerm::Constant(value) => render_rational(value),
        LiraTerm::Variable(name) => name.clone(),
        LiraTerm::Add(arguments) => {
            let mut rendered = arguments.iter().map(render_tff_term);
            let Some(first) = rendered.next() else {
                return render_rational(&Rational::zero());
            };
            rendered.fold(first, |left, right| format!("$sum({left},{right})"))
        }
        LiraTerm::Scale(coefficient, argument) => format!(
            "$product({},{})",
            render_rational(coefficient),
            render_tff_term(argument)
        ),
        LiraTerm::Floor(argument) => format!("$floor({})", render_tff_term(argument)),
    }
}

/// Renders a normalized LIRA formula as real-sorted TFF.
#[must_use]
pub fn render_tff_formula(formula: &Formula) -> String {
    match formula {
        Formula::Bool(value) => if *value { "$true" } else { "$false" }.to_owned(),
        Formula::Atom(literal) => {
            let term = render_tff_term(&literal.term);
            let zero = render_rational(&Rational::zero());
            match literal.relation {
                Relation::Eq => format!("({term} = {zero})"),
                Relation::Ne => format!("({term} != {zero})"),
                Relation::Gt => format!("$greater({term},{zero})"),
                Relation::Ge => format!("$greatereq({term},{zero})"),
            }
        }
        Formula::And(children) | Formula::Or(children) => {
            let separator = if matches!(formula, Formula::And(_)) {
                " & "
            } else {
                " | "
            };
            format!(
                "({})",
                children
                    .iter()
                    .map(render_tff_formula)
                    .collect::<Vec<_>>()
                    .join(separator)
            )
        }
        Formula::Exists(variable, body) | Formula::Forall(variable, body) => {
            let quantifier = if matches!(formula, Formula::Exists(_, _)) {
                "?"
            } else {
                "!"
            };
            format!(
                "{quantifier} [{variable}:$real] : ({})",
                render_tff_formula(body)
            )
        }
    }
}

/// Renders one canonical transformed TFF document.
#[must_use]
pub fn render_tff_document(name: &str, role: &str, formula: &Formula) -> String {
    let safe_name = name
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '_' {
                character
            } else {
                '_'
            }
        })
        .collect::<String>();
    format!(
        "tff(umlaut_viras_{safe_name},{role},\n    {} ).\n",
        render_tff_formula(formula)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arithmetic::viras::{rational, Limits};
    use crate::terms::signature::Signature;
    use crate::test_support::global_state_lock;

    struct ProblemTypeReset;

    impl Drop for ProblemTypeReset {
        fn drop(&mut self) {
            reset_problem_type();
        }
    }

    fn parse(source: &str) -> (TermBank, Term) {
        let _global = global_state_lock();
        reset_problem_type();
        set_problem_type(ProblemType::FirstOrder).expect("set TFF test mode");
        let _reset = ProblemTypeReset;
        let mut signature = Signature::new(TypeBank::new());
        signature
            .insert_internal_codes()
            .expect("insert logical symbols");
        let mut bank = TermBank::new(signature).expect("formula bank");
        let mut scanner = Scanner::from_user_string(source, false).expect("test scanner");
        let formula = bank
            .parse_tformula_tstp(&mut scanner)
            .expect("typed test formula parses");
        (bank, formula)
    }

    #[test]
    fn exact_number_parser_handles_frozen_spellings() {
        assert_eq!(
            parse_exact_number("-1.500000").expect("decimal"),
            rational(-3, 2).expect("q")
        );
        assert_eq!(
            parse_exact_number("1/3").expect("rational"),
            rational(1, 3).expect("q")
        );
        assert_eq!(
            parse_exact_number("1.250000e+02").expect("exponent"),
            Rational::from_integer(BigInt::from(125))
        );
        assert_eq!(
            parse_exact_number("-2.5E-1").expect("negative exponent"),
            rational(-1, 4).expect("q")
        );
        assert_eq!(
            parse_exact_number("1e-16385")
                .expect_err("oversized decimal denominator")
                .code,
            ImportErrorCode::MalformedInput
        );
        assert_eq!(
            parse_exact_number("1e16385")
                .expect_err("oversized decimal numerator")
                .code,
            ImportErrorCode::MalformedInput
        );
    }

    #[test]
    fn imports_integer_guard_and_negative_floor_exactly() {
        let (bank, formula) = parse("? [I:$int] : (I = $to_int(-1.5))");
        let imported = import_formula(&formula, bank.signature()).expect("accepted formula");
        assert!(matches!(imported.formula, Formula::Exists(_, _)));
        assert!(imported.formula.render().contains("floor"));
        assert!(imported
            .trace
            .iter()
            .any(|step| { step.kind == "binder" && step.target.contains("integrality_guard") }));
    }

    #[test]
    fn imports_boolean_connectives_and_linear_arithmetic() {
        let (bank, formula) = parse(
            "! [I:$int,R:$real] : (($to_real(I) = R) => \
             ($sum($to_real(I),R) = $product(2.0,R)))",
        );
        let imported = import_formula(&formula, bank.signature()).expect("accepted formula");
        assert!(matches!(imported.formula, Formula::Forall(_, _)));
        assert!(imported.formula.variables().is_empty());
        assert!(imported.formula.render().contains("LIRA_V"));

        let (bank, formula) = parse("! [R:$real] : ($less(R,0.0) | $greatereq(R,0.0))");
        let imported = import_formula(&formula, bank.signature()).expect("partition");
        assert!(matches!(imported.formula, Formula::Forall(_, _)));
    }

    #[test]
    fn every_supported_boolean_connective_matches_its_truth_table() {
        let cases = [
            ("($less(0,1) & $less(1,0))", false),
            ("($less(0,1) | $less(1,0))", true),
            ("($less(0,1) => $less(1,0))", false),
            ("($less(1,0) <= $less(0,1))", false),
            ("($less(0,1) <=> $less(1,0))", false),
            ("($less(0,1) <~> $less(1,0))", true),
            ("($less(0,1) ~& $less(1,0))", true),
            ("($less(0,1) ~| $less(1,0))", false),
            ("~($less(1,0))", true),
        ];
        for (source, expected) in cases {
            let (bank, formula) = parse(source);
            let imported = import_formula(&formula, bank.signature()).expect("connective imports");
            assert_eq!(
                imported
                    .formula
                    .evaluate(&BTreeMap::new())
                    .expect("ground formula evaluates"),
                expected,
                "{source}"
            );
        }
    }

    #[test]
    fn all_frozen_accepted_formula_bodies_import() {
        let cases = [
            "? [I:$int] : (I = $to_int(-1.5))",
            "! [I:$int] : ($floor($to_real(I)) = $to_real(I))",
            "! [R:$real] : ($floor($sum(R,1.0)) = $sum($floor(R),1.0))",
            "$ceiling(-1.5) = -1.0",
            "$less(1/3,1/2)",
            "! [I:$int] : ($to_real($quotient(I,2)) = \
             $quotient($to_real(I),2.0))",
            "! [I:$int,R:$real] : (($to_real(I) = R) => \
             ($sum($to_real(I),R) = $product(2.0,R)))",
            "! [R:$real] : ($is_int(R) <=> ($floor(R) = R))",
            "! [R:$real] : ($product(3.0,R) = $sum(R,$sum(R,R)))",
            "! [R:$real] : ($less(R,0.0) | $greatereq(R,0.0))",
            "! [I:$int] : ($to_real($to_rat(I)) = $to_real(I))",
            "$difference(1.25E2,25.0) = 100.0",
        ];
        for source in cases {
            let (bank, formula) = parse(source);
            let first =
                import_formula(&formula, bank.signature()).expect("frozen accepted formula");
            let second = import_formula(&formula, bank.signature()).expect("repeat import");
            assert_eq!(first, second, "unstable import for {source}");
            assert!(
                first.formula.variables().is_empty(),
                "accepted source is closed: {source}"
            );
        }
    }

    #[test]
    fn exact_rejection_taxonomy_matches_frozen_ast_cases() {
        let cases = [
            (
                "! [Q:$rat] : (Q = Q)",
                ImportErrorCode::UnsupportedRatQuantifier,
            ),
            (
                "! [R:$real] : ($to_rat(R) = $to_rat(R))",
                ImportErrorCode::UnsupportedRealToRat,
            ),
            (
                "! [R:$real] : $is_rat(R)",
                ImportErrorCode::UnsupportedRealRationality,
            ),
            (
                "! [R:$real,S:$real] : ($product(R,S) = 0.0)",
                ImportErrorCode::NonlinearProduct,
            ),
            (
                "! [R:$real,S:$real] : ($quotient(R,S) = R)",
                ImportErrorCode::NonconstantDivisor,
            ),
            ("$quotient(1.0,0.0) = 1.0", ImportErrorCode::ZeroDivisor),
            (
                "$truncate(-1.5) = -1.0",
                ImportErrorCode::UnsupportedRounding,
            ),
            ("$round(1.5) = 2.0", ImportErrorCode::UnsupportedRounding),
            ("$quotient_e(5,2) = 2", ImportErrorCode::UnsupportedOperator),
            (
                "$remainder_f(5,2) = 1",
                ImportErrorCode::UnsupportedOperator,
            ),
            (
                "! [R:$real] : (f(R) = R)",
                ImportErrorCode::UninterpretedArithmetic,
            ),
        ];
        for (source, expected) in cases {
            let (bank, formula) = parse(source);
            let error =
                import_formula(&formula, bank.signature()).expect_err("frozen unsupported formula");
            assert_eq!(error.code, expected, "{source}: {error}");
        }
    }

    #[test]
    #[allow(
        clippy::too_many_lines,
        reason = "the frozen accepted and rejected document corpus remains visibly co-located"
    )]
    fn document_gate_matches_all_frozen_cases() {
        let _global = global_state_lock();
        let accepted = [
            "tff(case,axiom,? [I:$int] : (I = $to_int(-1.5))).",
            "tff(case,axiom,! [I:$int] : \
             ($floor($to_real(I)) = $to_real(I))).",
            "tff(case,axiom,! [R:$real] : \
             ($floor($sum(R,1.0)) = $sum($floor(R),1.0))).",
            "tff(case,axiom,$ceiling(-1.5) = -1.0).",
            "tff(case,axiom,$less(1/3,1/2)).",
            "tff(case,axiom,! [I:$int] : \
             ($to_real($quotient(I,2)) = $quotient($to_real(I),2.0))).",
            "tff(case,axiom,! [I:$int,R:$real] : \
             (($to_real(I) = R) => \
             ($sum($to_real(I),R) = $product(2.0,R)))).",
            "tff(case,axiom,! [R:$real] : \
             ($is_int(R) <=> ($floor(R) = R))).",
            "tff(case,axiom,! [R:$real] : \
             ($product(3.0,R) = $sum(R,$sum(R,R)))).",
            "tff(case,axiom,! [R:$real] : \
             ($less(R,0.0) | $greatereq(R,0.0))).",
            "tff(case,axiom,! [I:$int] : \
             ($to_real($to_rat(I)) = $to_real(I))).",
            "tff(case,axiom,$difference(1.25E2,25.0) = 100.0).",
        ];
        for source in accepted {
            let first = import_document(source).expect("frozen accepted document");
            let second = import_document(source).expect("repeat document import");
            assert_eq!(first, second, "unstable document import: {source}");
        }

        let rejected = [
            (
                "tff(case,axiom,! [Q:$rat] : (Q = Q)).",
                ImportErrorCode::UnsupportedRatQuantifier,
            ),
            (
                "tff(case,axiom,! [R:$real] : \
                 ($to_rat(R) = $to_rat(R))).",
                ImportErrorCode::UnsupportedRealToRat,
            ),
            (
                "tff(case,axiom,! [R:$real] : $is_rat(R)).",
                ImportErrorCode::UnsupportedRealRationality,
            ),
            (
                "tff(case,axiom,! [R:$real,S:$real] : \
                 ($product(R,S) = 0.0)).",
                ImportErrorCode::NonlinearProduct,
            ),
            (
                "tff(case,axiom,! [R:$real,S:$real] : \
                 ($quotient(R,S) = R)).",
                ImportErrorCode::NonconstantDivisor,
            ),
            (
                "tff(case,axiom,$quotient(1.0,0.0) = 1.0).",
                ImportErrorCode::ZeroDivisor,
            ),
            (
                "tff(case,axiom,$truncate(-1.5) = -1.0).",
                ImportErrorCode::UnsupportedRounding,
            ),
            (
                "tff(case,axiom,$round(1.5) = 2.0).",
                ImportErrorCode::UnsupportedRounding,
            ),
            (
                "tff(case,axiom,$quotient_e(5,2) = 2).",
                ImportErrorCode::UnsupportedOperator,
            ),
            (
                "tff(case,axiom,$remainder_f(5,2) = 1).",
                ImportErrorCode::UnsupportedOperator,
            ),
            (
                "tff(case,axiom,! [R:$real] : (f(R) = R)).",
                ImportErrorCode::UninterpretedArithmetic,
            ),
            (
                "tff(case,axiom,! [I:$int,R:$real] : \
                 ($sum(I,R) = R)).",
                ImportErrorCode::TypeMismatch,
            ),
            (
                "tff(case,axiom,! [I:$int,R:$real] : (I = R)).",
                ImportErrorCode::TypeMismatch,
            ),
            (
                "tff(case,axiom,$sum(X,1) = 2).",
                ImportErrorCode::UnboundVariable,
            ),
            (
                "fof(case,axiom,1 = 1).",
                ImportErrorCode::UnsupportedDialect,
            ),
            (
                "tff(first,axiom,1 = 1). tff(second,axiom,2 = 2).",
                ImportErrorCode::UnsupportedDocument,
            ),
        ];
        for (source, expected) in rejected {
            let error = import_document(source).expect_err("frozen rejected document");
            assert_eq!(error.code, expected, "{source}: {error}");
        }
    }

    #[test]
    fn accepted_documents_eliminate_to_true_and_reembed() {
        let _global = global_state_lock();
        let document = import_document("tff(case,conjecture,? [I:$int] : (I = $to_int(-1.5))).")
            .expect("accepted document");
        let outcome =
            super::super::viras::eliminate_formula(document.import.formula, Limits::default());
        assert_eq!(outcome.status, super::super::viras::QeStatus::Success);
        assert_eq!(outcome.formula, Some(Formula::Bool(true)));
        let rendered = render_tff_document(
            &document.source_name,
            &document.source_role,
            outcome.formula.as_ref().expect("successful formula"),
        );
        assert_eq!(
            rendered,
            "tff(umlaut_viras_case,conjecture,\n    $true ).\n"
        );
    }
}
