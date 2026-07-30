use crate::basics::error::Diagnostic;
use crate::basics::simple_stuff::ProblemType;
use crate::clauses::clause::clause_parse;
use crate::clauses::clausesets::ClauseSet;
use crate::inout::basicparser::parse_float;
use crate::inout::scanner::{Scanner, TokenType};
use crate::learn::annotations::{Annotation, AnnotationTree};
use crate::learn::annoterms::{AnnoSet, AnnoTerm};
use crate::learn::clauseenc::rec_encode_clause_list_rep;
use crate::learn::examplerep::{ExampleRep, ExampleSet};
use crate::learn::numfeatures::{compute_clause_set_num_features, Features};
use crate::learn::patterns::{pattern_clause_compute, pattern_translate_sig, PatternSubst};
use crate::terms::signature::{Signature, SIG_LET_CODE};
use crate::terms::termbanks::TermBank;
use crate::terms::termtypes::DerefType;
use crate::terms::typebanks::TypeBank;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KbParseExampleFileResult {
    ident: i64,
    axiom_count: i64,
    parsed_example_clauses: i64,
    added_example_terms: i64,
}

impl KbParseExampleFileResult {
    #[must_use]
    pub const fn ident(self) -> i64 {
        self.ident
    }

    #[must_use]
    pub const fn axiom_count(self) -> i64 {
        self.axiom_count
    }

    #[must_use]
    pub const fn parsed_example_clauses(self) -> i64 {
        self.parsed_example_clauses
    }

    #[must_use]
    pub const fn added_example_terms(self) -> i64 {
        self.added_example_terms
    }
}

/// Allocate the private untyped signature used by persisted learning patterns.
///
/// Compact term cells interpret codes through [`SIG_LET_CODE`] as built-in
/// application, lambda, and formula constructors. E's KB signature is
/// otherwise intentionally sparse, so reserve those numeric slots before
/// normalized `f$arity_index` symbols are added. The placeholders are private
/// to the in-memory learning bank and are never serialized.
///
/// # Panics
///
/// Panics if signature insertion violates the dense function-code allocation
/// required by the reserved built-in range.
#[must_use]
pub fn kb_pattern_signature() -> Signature {
    let mut signature = Signature::new(TypeBank::new());
    while signature.f_count() < SIG_LET_CODE {
        let expected = signature.f_count() + 1;
        let inserted = signature.insert_id(&format!("$kb_reserved_{expected}"), 0, true);
        assert_eq!(
            inserted, expected,
            "learning signature reservation is dense"
        );
    }
    signature
}

/// Parse one knowledge-base example clause into an annotated recursive clause
/// representation.
///
/// C returns `NULL` if pattern computation is too expensive.
pub fn parse_example_clause(
    scanner: &mut Scanner,
    parse_terms: &mut TermBank,
    internal_terms: &mut TermBank,
    ident: i64,
    problem_type: ProblemType,
) -> Result<Option<AnnoTerm>, Diagnostic> {
    let annotations = parse_example_clause_annotation(scanner, ident)?;
    let clause = clause_parse(scanner, parse_terms, problem_type)?;
    ensure_recursive_clause_encoding_symbols(parse_terms);
    let mut pattern = pattern_clause_compute(
        &clause,
        PatternSubst::default_subst(parse_terms.signature()),
    );
    if pattern.tries() == 0 {
        return Ok(None);
    }
    let clauserep = rec_encode_clause_list_rep(parse_terms, pattern.listrep())?;
    let old_sig = parse_terms.signature().clone();
    let internal_vars = internal_terms.vars().clone();
    let translated = pattern_translate_sig(
        &clauserep,
        pattern.subst_mut(),
        &old_sig,
        internal_terms.signature_mut(),
        &internal_vars,
    );
    let newrep = internal_terms.insert(&translated, DerefType::Never)?;

    Ok(Some(AnnoTerm::new(newrep, annotations)))
}

/// Insert an axiom-set example and return the C-compatible assigned id.
///
/// This mirrors `KBAxiomsInsert`: the id is `set->count + 1`, feature
/// extraction is performed before insertion, and the insertion result is
/// ignored.
pub fn kb_axioms_insert(
    set: &mut ExampleSet,
    axioms: &ClauseSet,
    sig: &Signature,
    name: impl Into<String>,
) -> i64 {
    let ident = set.count() + 1;
    let mut features = Features::new();
    compute_clause_set_num_features(&mut features, axioms, sig);
    let rep = ExampleRep::new(ident, name.into(), features);
    let _inserted = set.insert(rep);
    ident
}

/// Parse one C `KBParseExampleFile` stream into example metadata and annotated
/// clause-pattern terms.
///
/// C allocates a temporary axiom term bank, frees it after feature extraction,
/// consumes one standalone full stop, then allocates a new parser term bank over
/// the caller's result signature and translates parsed examples into
/// `examples->terms`. Rust keeps the same parse phases but accepts the second
/// parser and destination term banks explicitly so ownership remains visible.
pub fn kb_parse_example_file(
    scanner: &mut Scanner,
    name: impl Into<String>,
    set: &mut ExampleSet,
    examples: &mut AnnoSet,
    parse_terms: &mut TermBank,
    internal_terms: &mut TermBank,
    problem_type: ProblemType,
) -> Result<KbParseExampleFileResult, Diagnostic> {
    let mut axiom_terms = TermBank::new(Signature::new(TypeBank::new()))?;
    let mut axioms = ClauseSet::new();
    let axiom_count = axioms.parse_list(scanner, &mut axiom_terms, problem_type)?;
    let ident = kb_axioms_insert(set, &axioms, axiom_terms.signature(), name);

    scanner.accept_tok(TokenType::FULLSTOP)?;

    let mut parsed_example_clauses = 0;
    let mut added_example_terms = 0;
    while !scanner.test_tok(TokenType::NO_TOKEN) {
        let parsed =
            parse_example_clause(scanner, parse_terms, internal_terms, ident, problem_type)?;
        parsed_example_clauses += 1;
        if let Some(term) = parsed {
            if examples.add_term(term) {
                added_example_terms += 1;
            }
        }
    }

    Ok(KbParseExampleFileResult {
        ident,
        axiom_count,
        parsed_example_clauses,
        added_example_terms,
    })
}

fn parse_example_clause_annotation(
    scanner: &mut Scanner,
    ident: i64,
) -> Result<AnnotationTree, Diagnostic> {
    scanner.accept_tok(TokenType::POS_INT)?;
    scanner.accept_tok(TokenType::COLON)?;

    let mut annotation = Annotation::with_key(ident);
    scanner.accept_tok(TokenType::OPEN_BRACKET)?;

    annotation.set_count(1.0);
    scanner.check_tok(TokenType::POS_INT)?;
    let first = scanner.current_token().numval();
    annotation.assign_value(1, if first == 0 { 1.0 } else { 0.0 });
    annotation.assign_value(2, u64_to_f64(first));
    scanner.accept_tok(TokenType::POS_INT)?;

    let mut length = 3_i64;
    while scanner.test_tok(TokenType::COMMA) {
        scanner.accept_tok(TokenType::COMMA)?;
        annotation.assign_value(length, parse_float(scanner)?);
        length += 1;
    }
    annotation.set_length(length);

    scanner.accept_tok(TokenType::CLOSE_BRACKET)?;
    scanner.accept_tok(TokenType::COLON)?;

    let mut annotations = AnnotationTree::new();
    assert!(
        annotations.store(ident, annotation, ()),
        "single parsed annotation must insert into empty tree"
    );
    Ok(annotations)
}

fn ensure_recursive_clause_encoding_symbols(bank: &mut TermBank) {
    bank.signature_mut().get_eqn_code(true);
    bank.signature_mut().get_eqn_code(false);
    bank.signature_mut().get_or_code();
    bank.signature_mut().get_cnil_code();
}

#[allow(clippy::cast_precision_loss)]
fn u64_to_f64(value: u64) -> f64 {
    value as f64
}

#[cfg(test)]
mod tests {
    use super::{kb_axioms_insert, kb_parse_example_file, parse_example_clause};
    use crate::basics::simple_stuff::ProblemType;
    use crate::clauses::clause::Clause;
    use crate::clauses::clausesets::ClauseSet;
    use crate::clauses::eqn::Eqn;
    use crate::clauses::eqnlist::EqnList;
    use crate::inout::scanner::{IoFormat, Scanner, TokenType};
    use crate::learn::annoterms::AnnoSet;
    use crate::learn::examplerep::{ExampleRep, ExampleSet};
    use crate::terms::signature::Signature;
    use crate::terms::simpletypes::alloc_arrow_type;
    use crate::terms::termbanks::TermBank;
    use crate::terms::termtypes::{DerefType, Term};
    use crate::terms::typebanks::TypeBank;

    fn assert_close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 1.0e-12,
            "expected {expected}, got {actual}"
        );
    }

    fn test_bank() -> TermBank {
        TermBank::new(Signature::new(TypeBank::new())).expect("term bank allocation")
    }

    fn typed_const(bank: &mut TermBank, name: &str) -> Term {
        let type_ = bank.signature().type_bank().default_type();
        let f_code = bank.signature_mut().insert_id(name, 0, false);
        bank.signature_mut()
            .declare_final_type(f_code, type_.clone())
            .expect("constant type declaration");
        let term = Term::const_cell_alloc(f_code);
        term.set_type(Some(type_));
        bank.insert(&term, DerefType::Never)
            .expect("constant insertion")
    }

    fn typed_unary(bank: &mut TermBank, name: &str, arg: &Term) -> Term {
        let type_ = bank.signature().type_bank().default_type();
        let f_code = bank.signature_mut().insert_id(name, 1, false);
        bank.signature_mut()
            .declare_final_type(f_code, alloc_arrow_type(vec![type_.clone(), type_.clone()]))
            .expect("unary type declaration");
        let term = Term::top_alloc(f_code, 1);
        term.set_type(Some(type_));
        term.set_argument(0, arg.clone());
        bank.insert(&term, DerefType::Never)
            .expect("unary insertion")
    }

    fn literal(bank: &mut TermBank, left: &Term, right: &Term, positive: bool) -> Eqn {
        Eqn::alloc(left.clone(), right.clone(), bank, positive).expect("literal allocation")
    }

    fn clause_from(literals: Vec<Eqn>) -> Clause {
        let mut clause = Clause::alloc(EqnList::from_vec(literals));
        clause.set_weight(clause.standard_weight());
        clause
    }

    #[test]
    fn kb_axioms_insert_assigns_count_plus_one_and_computes_features() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let fa = typed_unary(&mut bank, "f", &a);
        let unit = clause_from(vec![literal(&mut bank, &a, &b, true)]);
        let horn = clause_from(vec![
            literal(&mut bank, &fa, &b, true),
            literal(&mut bank, &a, &b, false),
        ]);
        let axioms = ClauseSet::from_clauses([unit, horn]);
        let mut set = ExampleSet::new();

        let ident = kb_axioms_insert(&mut set, &axioms, bank.signature(), "prob");

        assert_eq!(ident, 1);
        assert_eq!(set.count(), 1);
        let rep = set.find_ident(ident).expect("inserted example");
        assert_eq!(rep.name(), "prob");
        assert_close(rep.features().value(0).expect("unit count"), 1.0);
        assert_close(rep.features().value(1).expect("horn count"), 1.0);
        assert_close(rep.features().value(2).expect("general count"), 0.0);
        assert_eq!(rep.features().func_max_arity(), 1);
    }

    #[test]
    fn kb_axioms_insert_preserves_c_duplicate_name_side_effect() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let axioms = ClauseSet::from_clauses([clause_from(vec![literal(&mut bank, &a, &b, true)])]);
        let mut set = ExampleSet::new();

        assert_eq!(
            kb_axioms_insert(&mut set, &axioms, bank.signature(), "dup"),
            1
        );
        assert_eq!(
            kb_axioms_insert(&mut set, &axioms, bank.signature(), "dup"),
            2
        );

        assert_eq!(set.count(), 1);
        assert!(set.find_ident(2).is_some());
        assert_eq!(set.find_name("dup").map(ExampleRep::ident), Some(1));
    }

    #[test]
    fn kb_parse_example_file_reads_axioms_separator_and_example_clauses() {
        let mut scanner = Scanner::from_user_string(
            "\
a=b.
f(a)=b.
.
0:(0): a=b.
1:(3,4.5): f(a)=b.
",
            false,
        )
        .expect("scanner allocation");
        let mut examples = AnnoSet::new();
        let mut set = ExampleSet::new();
        let mut parse_terms = test_bank();
        let mut internal_terms = test_bank();

        let result = kb_parse_example_file(
            &mut scanner,
            "problem",
            &mut set,
            &mut examples,
            &mut parse_terms,
            &mut internal_terms,
            ProblemType::FirstOrder,
        )
        .expect("KB example file parses");

        assert_eq!(result.ident(), 1);
        assert_eq!(result.axiom_count(), 2);
        assert_eq!(result.parsed_example_clauses(), 2);
        assert_eq!(result.added_example_terms(), 2);
        assert!(scanner.test_tok(TokenType::NO_TOKEN));
        assert_eq!(set.count(), 1);
        let rep = set.find_ident(1).expect("example metadata inserted");
        assert_eq!(rep.name(), "problem");
        assert_close(rep.features().value(0).expect("unit count"), 2.0);
        assert_eq!(rep.features().func_max_arity(), 1);
        assert_eq!(examples.nodes(), 2);
        assert!(examples
            .iter()
            .all(|(_key, term)| term.annotations().find(1).is_some()));
    }

    #[test]
    fn kb_parse_example_file_merges_duplicate_pattern_terms_like_anno_set_add_term() {
        let mut scanner = Scanner::from_user_string(
            "\
a=b.
.
0:(0): a=b.
1:(2): a=b.
",
            false,
        )
        .expect("scanner allocation");
        let mut examples = AnnoSet::new();
        let mut set = ExampleSet::new();
        let mut parse_terms = test_bank();
        let mut internal_terms = test_bank();

        let result = kb_parse_example_file(
            &mut scanner,
            "dup-pattern",
            &mut set,
            &mut examples,
            &mut parse_terms,
            &mut internal_terms,
            ProblemType::FirstOrder,
        )
        .expect("KB example file parses");

        assert_eq!(result.ident(), 1);
        assert_eq!(result.axiom_count(), 1);
        assert_eq!(result.parsed_example_clauses(), 2);
        assert_eq!(result.added_example_terms(), 1);
        assert_eq!(examples.nodes(), 1);
        let (_term_key, term) = examples.iter().next().expect("merged annotated term");
        let annotation = &term.annotations().find(1).expect("source annotation").val1;
        assert_close(annotation.count(), 2.0);
        assert_close(annotation.value(1).expect("proof count"), 0.5);
        assert_close(annotation.value(2).expect("proof distance"), 1.0);
    }

    #[test]
    fn kb_parse_example_file_reuses_normalized_symbols_across_predicate_and_function_roles() {
        let mut scanner = Scanner::from_user_string(
            "\
.
0:(0): ssAccess(X1,authObj(X2,X3,X4)) <-
        ssUserProfile(userProfileEntry(X1,authObj(X2,X3,X4))).
1:(0): ssUserProfile(userProfileEntry(X1,authObj(X3,X4,X5))) <-
        ssHolds(X1,X2), ssSingleRole(singleRoleEntry(X2,authObj(X3,X4,X5))).
2:(0): ssSingleRole(
        singleRoleEntry(ssRole,authObj(ssObject,ssField,ssValue))) <- .
",
            false,
        )
        .expect("scanner allocation");
        let mut examples = AnnoSet::new();
        let mut set = ExampleSet::new();
        let mut internal_terms =
            TermBank::new(super::kb_pattern_signature()).expect("internal term bank allocation");
        let mut parse_terms =
            TermBank::new(internal_terms.signature().clone()).expect("parser term bank allocation");

        let result = kb_parse_example_file(
            &mut scanner,
            "mixed-normalized-roles",
            &mut set,
            &mut examples,
            &mut parse_terms,
            &mut internal_terms,
            ProblemType::FirstOrder,
        )
        .expect("untyped normalized learning symbols must not acquire conflicting result sorts");

        assert_eq!(result.parsed_example_clauses(), 3);
        assert_eq!(examples.nodes(), 3);
        assert!(scanner.test_tok(TokenType::NO_TOKEN));
    }

    #[test]
    fn parse_example_clause_builds_annotation_and_translated_term() {
        let mut parse_terms = test_bank();
        let mut internal_terms = test_bank();
        let mut scanner =
            Scanner::from_user_string("9:(0,2.5): a=b.", false).expect("scanner allocation");

        let parsed = parse_example_clause(
            &mut scanner,
            &mut parse_terms,
            &mut internal_terms,
            42,
            ProblemType::FirstOrder,
        )
        .expect("example clause parse")
        .expect("pattern computation succeeds");

        let annotation = parsed.single_annotation().expect("single annotation");
        assert_eq!(annotation.key(), 42);
        assert_eq!(annotation.length(), 4);
        assert_close(annotation.count(), 1.0);
        assert_close(annotation.value(1).expect("proof count"), 1.0);
        assert_close(annotation.value(2).expect("proof distance"), 0.0);
        assert_close(annotation.value(3).expect("extra value"), 2.5);
        assert!(parsed.term().is_shared());
        assert!(scanner.test_tok(TokenType::NO_TOKEN));
    }

    #[test]
    fn parse_example_clause_sets_nonzero_first_value_proof_count_to_zero() {
        let mut parse_terms = test_bank();
        let mut internal_terms = test_bank();
        let mut scanner =
            Scanner::from_user_string("1:(7): a=b.", false).expect("scanner allocation");

        let parsed = parse_example_clause(
            &mut scanner,
            &mut parse_terms,
            &mut internal_terms,
            5,
            ProblemType::FirstOrder,
        )
        .expect("example clause parse")
        .expect("pattern computation succeeds");

        let annotation = parsed.single_annotation().expect("single annotation");
        assert_eq!(annotation.length(), 3);
        assert_close(annotation.value(1).expect("proof count"), 0.0);
        assert_close(annotation.value(2).expect("proof distance"), 7.0);
    }

    #[test]
    fn parse_example_clause_skips_pattern_search_over_branch_limit() {
        let mut parse_terms = test_bank();
        let mut internal_terms = test_bank();
        let mut scanner = Scanner::from_user_string("9:(0): cnf(skip, axiom, (a=b | c=d)).", false)
            .expect("scanner allocation");
        scanner.set_format(IoFormat::Tstp);
        let destination_nodes = internal_terms.non_var_term_nodes();

        let parsed = parse_example_clause(
            &mut scanner,
            &mut parse_terms,
            &mut internal_terms,
            42,
            ProblemType::FirstOrder,
        )
        .expect("example clause parse");

        assert!(parsed.is_none());
        assert!(scanner.test_tok(TokenType::NO_TOKEN));
        assert_eq!(internal_terms.non_var_term_nodes(), destination_nodes);
    }
}
