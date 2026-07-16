use crate::basics::pstacks::PStack;
use crate::basics::sysdate::SysDate;
use crate::terms::functypes::FunCode;
use crate::terms::signature::{
    SIG_DB_LAMBDA_CODE, SIG_ITE_CODE, SIG_LET_CODE, SIG_NAMED_LAMBDA_CODE, SIG_PHONY_APP_CODE,
};
use crate::terms::simpletypes::{sort_is_interpreted, Type};
use std::cell::{Cell, Ref, RefCell};
use std::num::NonZeroUsize;
use std::ops::{BitAnd, BitOr, BitOrAssign, Not};
use std::rc::Rc;

pub const DEFAULT_VWEIGHT: i64 = 1;
pub const DEFAULT_FWEIGHT: i64 = 2;
pub const TERMS_INITIAL_ARGS: usize = 10;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TermProperties(u64);

impl TermProperties {
    pub const IGNORE_PROPS: Self = Self(0);
    pub const TOP_POS: Self = Self(2);
    pub const IS_GROUND: Self = Self(4);
    pub const PRED_POS: Self = Self(8);
    pub const IS_REWRITABLE: Self = Self(16);
    pub const IS_RREWRITABLE: Self = Self(32);
    pub const IS_SOS_REWRITTEN: Self = Self(64);
    pub const SPECIAL_FLAG: Self = Self(128);
    pub const OP_FLAG: Self = Self(256);
    pub const CHECK_FLAG: Self = Self(512);
    pub const OUTPUT_FLAG: Self = Self(1024);
    pub const IS_SPECIAL_VAR: Self = Self(2048);
    pub const IS_REWRITTEN: Self = Self(4096);
    pub const IS_RREWRITTEN: Self = Self(8192);
    pub const IS_SHARED: Self = Self(16_384);
    pub const GARBAGE_FLAG: Self = Self(32_768);
    pub const IS_FREE_VAR: Self = Self(65_536);
    pub const POTENTIAL_PARAMOD: Self = Self(131_072);
    pub const POS_POLARITY: Self = Self(1_u64 << 18);
    pub const NEG_POLARITY: Self = Self(1_u64 << 19);
    pub const IS_DEREFED_APP_VAR: Self = Self(1_u64 << 20);
    pub const IS_BETA_REDUCIBLE: Self = Self(1_u64 << 21);
    pub const IS_ETA_REDUCIBLE: Self = Self(1_u64 << 22);
    pub const IS_DB_VAR: Self = Self(1_u64 << 23);
    pub const HAS_LAMBDA_SUBTERM: Self = Self(1_u64 << 24);
    pub const HAS_ETA_EXPANDABLE_SUBTERM: Self = Self(1_u64 << 25);
    pub const HAS_DB_SUBTERM: Self = Self(1_u64 << 26);
    pub const HAS_NON_PATTERN_VAR: Self = Self(1_u64 << 27);
    pub const HAS_APP_VAR: Self = Self(1_u64 << 28);
    pub const HAS_EQ_NEQ_SYM: Self = Self(1_u64 << 29);
    pub const HAS_BOOL_SUBTERM: Self = Self(1_u64 << 30);
    pub const IS_CONJECTURE_TERM: Self = Self(1_u64 << 31);

    #[must_use]
    pub const fn from_bits(bits: u64) -> Self {
        Self(bits)
    }

    #[must_use]
    pub const fn bits(self) -> u64 {
        self.0
    }

    #[must_use]
    pub const fn query(self, prop: Self) -> bool {
        (self.0 & prop.0) == prop.0
    }

    #[must_use]
    pub const fn is_any_set(self, prop: Self) -> bool {
        (self.0 & prop.0) != 0
    }

    #[must_use]
    pub const fn any_set(self, prop: Self) -> Self {
        self.give(prop)
    }

    #[must_use]
    pub const fn give(self, prop: Self) -> Self {
        Self(self.0 & prop.0)
    }
}

impl BitOr for TermProperties {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for TermProperties {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl BitAnd for TermProperties {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl Not for TermProperties {
    type Output = Self;

    fn not(self) -> Self::Output {
        Self(!self.0)
    }
}

pub const TP_IGNORE_PROPS: TermProperties = TermProperties::IGNORE_PROPS;
pub const TP_TOP_POS: TermProperties = TermProperties::TOP_POS;
pub const TP_IS_GROUND: TermProperties = TermProperties::IS_GROUND;
pub const TP_PRED_POS: TermProperties = TermProperties::PRED_POS;
pub const TP_IS_REWRITABLE: TermProperties = TermProperties::IS_REWRITABLE;
pub const TP_IS_RREWRITABLE: TermProperties = TermProperties::IS_RREWRITABLE;
pub const TP_IS_SOS_REWRITTEN: TermProperties = TermProperties::IS_SOS_REWRITTEN;
pub const TP_SPECIAL_FLAG: TermProperties = TermProperties::SPECIAL_FLAG;
pub const TP_OP_FLAG: TermProperties = TermProperties::OP_FLAG;
pub const TP_CHECK_FLAG: TermProperties = TermProperties::CHECK_FLAG;
pub const TP_OUTPUT_FLAG: TermProperties = TermProperties::OUTPUT_FLAG;
pub const TP_IS_SPECIAL_VAR: TermProperties = TermProperties::IS_SPECIAL_VAR;
pub const TP_IS_REWRITTEN: TermProperties = TermProperties::IS_REWRITTEN;
pub const TP_IS_RREWRITTEN: TermProperties = TermProperties::IS_RREWRITTEN;
pub const TP_IS_SHARED: TermProperties = TermProperties::IS_SHARED;
pub const TP_GARBAGE_FLAG: TermProperties = TermProperties::GARBAGE_FLAG;
pub const TP_IS_FREE_VAR: TermProperties = TermProperties::IS_FREE_VAR;
pub const TP_POTENTIAL_PARAMOD: TermProperties = TermProperties::POTENTIAL_PARAMOD;
pub const TP_POS_POLARITY: TermProperties = TermProperties::POS_POLARITY;
pub const TP_NEG_POLARITY: TermProperties = TermProperties::NEG_POLARITY;
pub const TP_IS_DEREFED_APP_VAR: TermProperties = TermProperties::IS_DEREFED_APP_VAR;
pub const TP_IS_BETA_REDUCIBLE: TermProperties = TermProperties::IS_BETA_REDUCIBLE;
pub const TP_IS_ETA_REDUCIBLE: TermProperties = TermProperties::IS_ETA_REDUCIBLE;
pub const TP_IS_DB_VAR: TermProperties = TermProperties::IS_DB_VAR;
pub const TP_HAS_LAMBDA_SUBTERM: TermProperties = TermProperties::HAS_LAMBDA_SUBTERM;
pub const TP_HAS_ETA_EXPANDABLE_SUBTERM: TermProperties =
    TermProperties::HAS_ETA_EXPANDABLE_SUBTERM;
pub const TP_HAS_DB_SUBTERM: TermProperties = TermProperties::HAS_DB_SUBTERM;
pub const TP_HAS_NON_PATTERN_VAR: TermProperties = TermProperties::HAS_NON_PATTERN_VAR;
pub const TP_HAS_APP_VAR: TermProperties = TermProperties::HAS_APP_VAR;
pub const TP_HAS_EQ_NEQ_SYM: TermProperties = TermProperties::HAS_EQ_NEQ_SYM;
pub const TP_HAS_BOOL_SUBTERM: TermProperties = TermProperties::HAS_BOOL_SUBTERM;
pub const TP_IS_CONJECTURE_TERM: TermProperties = TermProperties::IS_CONJECTURE_TERM;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum DerefType {
    Never = 0,
    Once = 1,
    Always = 2,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum RewriteLevel {
    NoRewrite = 0,
    RuleRewrite = 1,
    FullRewrite = 2,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RewriteDemodulator {
    id: NonZeroUsize,
    generation: u64,
}

impl RewriteDemodulator {
    /// Creates an opaque rewrite-demodulator handle.
    ///
    /// # Panics
    ///
    /// Panics if `id` is zero. The C field is a nullable pointer, so zero is
    /// represented by `None` in Rust.
    #[must_use]
    pub fn new(id: usize) -> Self {
        Self::new_with_generation(id, 0)
    }

    /// Creates a rewrite-demodulator handle with an opaque clause generation.
    ///
    /// # Panics
    ///
    /// Panics if `id` is zero.
    #[must_use]
    pub fn new_with_generation(id: usize, generation: u64) -> Self {
        let id = NonZeroUsize::new(id)
            .unwrap_or_else(|| panic!("rewrite demodulator id zero is represented by None"));
        Self { id, generation }
    }

    #[must_use]
    pub const fn id(self) -> usize {
        self.id.get()
    }

    #[must_use]
    pub const fn generation(self) -> u64 {
        self.generation
    }
}

#[derive(Clone, Debug)]
pub struct Term(Rc<TermCell>);

#[derive(Debug, Default)]
struct TermLinks {
    binding: Option<Term>,
    rw_replace: Option<Term>,
    type_: Option<Type>,
    left: Option<Term>,
    right: Option<Term>,
}

#[derive(Debug)]
struct TermCell {
    f_code: Cell<FunCode>,
    properties: Cell<TermProperties>,
    args: RefCell<Vec<Option<Term>>>,
    // C stores these five nullable pointers inline in TermCell. One shared
    // interior-mutation boundary preserves that compact shape without unsafe
    // access or one borrow flag per pointer.
    links: RefCell<TermLinks>,
    entry_no: Cell<i64>,
    weight: Cell<i64>,
    v_count: Cell<u32>,
    f_count: Cell<u32>,
    nf_date: [Cell<SysDate>; 2],
    rw_demod: Cell<Option<RewriteDemodulator>>,
}

impl PartialEq for Term {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for Term {}

impl Term {
    #[must_use]
    pub fn default_cell_alloc() -> Self {
        Self::new_with_arity(0)
    }

    #[must_use]
    pub fn default_cell_arity_alloc(arity: usize) -> Self {
        Self::new_with_arity(arity)
    }

    #[must_use]
    pub fn const_cell_alloc(symbol: FunCode) -> Self {
        let term = Self::default_cell_alloc();
        term.set_f_code(symbol);
        term
    }

    #[must_use]
    pub fn top_alloc(f_code: FunCode, arity: usize) -> Self {
        let term = Self::default_cell_arity_alloc(arity);
        term.set_f_code(f_code);
        term
    }

    #[must_use]
    pub fn top_copy_without_args(source: &Self) -> Self {
        let copy = Self::default_cell_arity_alloc(source.arity());
        copy.set_properties(source.properties() & (TP_PRED_POS | TP_IS_DB_VAR));
        copy.del_prop(TP_OUTPUT_FLAG);
        copy.set_f_code(source.f_code());
        copy.set_type(source.type_());
        copy
    }

    #[must_use]
    pub fn top_copy(source: &Self) -> Self {
        let copy = Self::top_copy_without_args(source);
        for (index, arg) in source.argument_clones().into_iter().enumerate() {
            copy.set_argument_opt(index, arg);
        }
        copy
    }

    #[must_use]
    pub fn f_code(&self) -> FunCode {
        self.0.f_code.get()
    }

    pub fn set_f_code(&self, f_code: FunCode) {
        self.0.f_code.set(f_code);
    }

    #[must_use]
    pub fn arity(&self) -> usize {
        self.0.args.borrow().len()
    }

    #[must_use]
    pub fn arg_num(&self) -> usize {
        if self.is_phony_app() {
            self.arity() - 1
        } else {
            self.arity()
        }
    }

    #[must_use]
    pub fn argument(&self, index: usize) -> Option<Term> {
        self.0.args.borrow().get(index).cloned().flatten()
    }

    /// Assigns an argument slot.
    ///
    /// # Panics
    ///
    /// Panics if `index` is outside the term arity, matching the C flexible
    /// array precondition.
    pub fn set_argument(&self, index: usize, arg: Term) {
        self.set_argument_opt(index, Some(arg));
    }

    /// Assigns an argument slot, including a temporary uninitialized value.
    ///
    /// # Panics
    ///
    /// Panics if `index` is outside the term arity, matching the C flexible
    /// array precondition.
    pub fn set_argument_opt(&self, index: usize, arg: Option<Term>) {
        let mut args = self.0.args.borrow_mut();
        let slot = args
            .get_mut(index)
            .unwrap_or_else(|| panic!("term argument index {index} out of bounds"));
        *slot = arg;
    }

    /// Borrows the argument slots without cloning their reference-counted terms.
    #[must_use]
    pub fn arguments(&self) -> Ref<'_, [Option<Term>]> {
        Ref::map(self.0.args.borrow(), Vec::as_slice)
    }

    #[must_use]
    pub fn argument_clones(&self) -> Vec<Option<Term>> {
        self.0.args.borrow().clone()
    }

    #[must_use]
    pub fn binding(&self) -> Option<Term> {
        self.0.links.borrow().binding.clone()
    }

    pub fn set_binding(&self, binding: Option<Term>) {
        self.0.links.borrow_mut().binding = binding;
    }

    #[must_use]
    pub fn entry_no(&self) -> i64 {
        self.0.entry_no.get()
    }

    pub fn set_entry_no(&self, entry_no: i64) {
        self.0.entry_no.set(entry_no);
    }

    #[must_use]
    pub fn weight(&self) -> i64 {
        self.0.weight.get()
    }

    pub fn set_weight(&self, weight: i64) {
        self.0.weight.set(weight);
    }

    #[must_use]
    pub fn v_count(&self) -> u32 {
        self.0.v_count.get()
    }

    pub fn set_v_count(&self, count: u32) {
        self.0.v_count.set(count);
    }

    #[must_use]
    pub fn f_count(&self) -> u32 {
        self.0.f_count.get()
    }

    pub fn set_f_count(&self, count: u32) {
        self.0.f_count.set(count);
    }

    #[must_use]
    pub fn type_(&self) -> Option<Type> {
        self.0.links.borrow().type_.clone()
    }

    pub fn set_type(&self, type_: Option<Type>) {
        self.0.links.borrow_mut().type_ = type_;
    }

    #[must_use]
    pub fn left_son(&self) -> Option<Term> {
        self.0.links.borrow().left.clone()
    }

    pub fn set_left_son(&self, term: Option<Term>) {
        self.0.links.borrow_mut().left = term;
    }

    #[must_use]
    pub fn right_son(&self) -> Option<Term> {
        self.0.links.borrow().right.clone()
    }

    pub fn set_right_son(&self, term: Option<Term>) {
        self.0.links.borrow_mut().right = term;
    }

    #[must_use]
    pub fn take_left_son(&self) -> Option<Term> {
        self.0.links.borrow_mut().left.take()
    }

    #[must_use]
    pub fn take_right_son(&self) -> Option<Term> {
        self.0.links.borrow_mut().right.take()
    }

    pub fn clear_tree_links(&self) {
        self.set_left_son(None);
        self.set_right_son(None);
    }

    #[must_use]
    pub fn nf_date(&self, level: RewriteLevel) -> SysDate {
        let index = rewrite_index(level);
        if self.is_rewritten() {
            SysDate::creation_time()
        } else {
            self.0.nf_date[index].get()
        }
    }

    pub fn set_nf_date(&self, level: RewriteLevel, date: SysDate) {
        let index = rewrite_index(level);
        self.0.nf_date[index].set(date);
    }

    #[must_use]
    pub fn rw_replace_field(&self) -> Option<Term> {
        self.0.links.borrow().rw_replace.clone()
    }

    pub fn set_rw_replace_field(&self, replacement: Option<Term>) {
        self.0.links.borrow_mut().rw_replace = replacement;
    }

    #[must_use]
    pub fn rw_demod_field(&self) -> Option<RewriteDemodulator> {
        self.0.rw_demod.get()
    }

    pub fn set_rw_demod_field(&self, demod: Option<RewriteDemodulator>) {
        self.0.rw_demod.set(demod);
    }

    #[must_use]
    pub fn properties(&self) -> TermProperties {
        self.0.properties.get()
    }

    pub fn set_properties(&self, properties: TermProperties) {
        self.0.properties.set(properties);
    }

    pub fn set_prop(&self, prop: TermProperties) {
        self.set_properties(self.properties() | prop);
    }

    pub fn del_prop(&self, prop: TermProperties) {
        self.set_properties(self.properties() & !prop);
    }

    pub fn flip_prop(&self, prop: TermProperties) {
        self.set_properties(TermProperties::from_bits(
            self.properties().bits() ^ prop.bits(),
        ));
    }

    pub fn assign_prop(&self, selector: TermProperties, prop: TermProperties) {
        self.del_prop(selector);
        self.set_prop(selector & prop);
    }

    #[must_use]
    pub fn query_prop(&self, prop: TermProperties) -> bool {
        self.properties().query(prop)
    }

    #[must_use]
    pub fn is_any_prop_set(&self, prop: TermProperties) -> bool {
        self.properties().is_any_set(prop)
    }

    #[must_use]
    pub fn any_prop_set(&self, prop: TermProperties) -> TermProperties {
        self.properties().any_set(prop)
    }

    #[must_use]
    pub fn give_props(&self, prop: TermProperties) -> TermProperties {
        self.properties().give(prop)
    }

    #[must_use]
    pub fn is_free_var(&self) -> bool {
        self.f_code() < 0
    }

    #[must_use]
    pub fn is_db_var(&self) -> bool {
        self.query_prop(TP_IS_DB_VAR)
    }

    #[must_use]
    pub fn is_any_var(&self) -> bool {
        self.is_free_var() || self.is_db_var()
    }

    #[must_use]
    pub fn is_const(&self) -> bool {
        !self.is_any_var() && self.arity() == 0
    }

    #[must_use]
    pub fn is_phony_app(&self) -> bool {
        !self.is_db_var() && self.f_code() == SIG_PHONY_APP_CODE
    }

    #[must_use]
    pub fn is_applied_free_var(&self) -> bool {
        self.is_phony_app() && self.argument(0).is_some_and(|arg| arg.is_free_var())
    }

    #[must_use]
    pub fn is_applied_db_var(&self) -> bool {
        self.is_phony_app() && self.argument(0).is_some_and(|arg| arg.is_db_var())
    }

    #[must_use]
    pub fn is_applied_any_var(&self) -> bool {
        self.is_phony_app() && self.argument(0).is_some_and(|arg| arg.is_any_var())
    }

    #[must_use]
    pub fn is_lambda(&self) -> bool {
        !self.is_db_var() && matches!(self.f_code(), SIG_NAMED_LAMBDA_CODE | SIG_DB_LAMBDA_CODE)
    }

    #[must_use]
    pub fn is_db_lambda(&self) -> bool {
        !self.is_db_var() && self.f_code() == SIG_DB_LAMBDA_CODE
    }

    #[must_use]
    pub fn is_top_level_free_var(&self) -> bool {
        self.is_free_var() || self.is_applied_free_var()
    }

    #[must_use]
    pub fn is_top_level_db_var(&self) -> bool {
        self.is_db_var() || self.is_applied_db_var()
    }

    #[must_use]
    pub fn is_top_level_any_var(&self) -> bool {
        self.is_any_var() || self.is_applied_any_var()
    }

    #[must_use]
    pub fn is_phony_app_target(&self) -> bool {
        self.is_any_var()
            || self.is_lambda()
            || matches!(self.f_code(), SIG_ITE_CODE | SIG_LET_CODE)
    }

    #[must_use]
    pub fn is_pattern(&self) -> bool {
        !self.query_prop(TP_HAS_NON_PATTERN_VAR)
    }

    #[must_use]
    pub fn is_non_fo_pattern(&self) -> bool {
        self.is_pattern() && (self.has_lambda_subterm() || self.has_db_subterm())
    }

    #[must_use]
    pub fn is_rewritten(&self) -> bool {
        self.query_prop(TP_IS_REWRITTEN)
    }

    #[must_use]
    pub fn is_rrewritten(&self) -> bool {
        self.query_prop(TP_IS_RREWRITTEN)
    }

    #[must_use]
    pub fn is_top_rewritten(&self) -> bool {
        self.is_rewritten() && self.rw_demod_field().is_some()
    }

    #[must_use]
    pub fn is_shared(&self) -> bool {
        self.query_prop(TP_IS_SHARED)
    }

    #[must_use]
    pub fn has_eq_neq(&self) -> bool {
        self.query_prop(TP_HAS_EQ_NEQ_SYM)
    }

    #[must_use]
    pub fn has_bool_subterm(&self) -> bool {
        self.query_prop(TP_HAS_BOOL_SUBTERM)
    }

    #[must_use]
    pub fn has_lambda_subterm(&self) -> bool {
        self.query_prop(TP_HAS_LAMBDA_SUBTERM)
    }

    #[must_use]
    pub fn has_eta_expandable_subterm(&self) -> bool {
        self.query_prop(TP_HAS_ETA_EXPANDABLE_SUBTERM)
    }

    #[must_use]
    pub fn has_db_subterm(&self) -> bool {
        self.query_prop(TP_HAS_DB_SUBTERM)
    }

    #[must_use]
    pub fn has_app_var(&self) -> bool {
        self.query_prop(TP_HAS_APP_VAR)
    }

    #[must_use]
    pub fn has_higher_order_ordering_surface(&self) -> bool {
        let mut stack = vec![self.clone()];
        while let Some(term) = stack.pop() {
            if term.is_db_var() || term.is_lambda() || term.is_phony_app() {
                return true;
            }
            stack.extend(term.argument_clones().into_iter().flatten());
        }
        false
    }

    #[must_use]
    pub fn is_beta_reducible(&self) -> bool {
        self.query_prop(TP_IS_BETA_REDUCIBLE)
    }

    #[must_use]
    pub fn is_eta_reducible(&self) -> bool {
        self.query_prop(TP_IS_ETA_REDUCIBLE)
    }

    fn new_with_arity(arity: usize) -> Self {
        Self(Rc::new(TermCell {
            f_code: Cell::new(0),
            properties: Cell::new(TP_IGNORE_PROPS),
            args: RefCell::new(vec![None; arity]),
            links: RefCell::new(TermLinks::default()),
            entry_no: Cell::new(0),
            weight: Cell::new(0),
            v_count: Cell::new(0),
            f_count: Cell::new(0),
            nf_date: [
                Cell::new(SysDate::creation_time()),
                Cell::new(SysDate::creation_time()),
            ],
            rw_demod: Cell::new(None),
        }))
    }
}

#[must_use]
pub fn term_identity_id(term: &Term) -> usize {
    Rc::as_ptr(&term.0).cast::<()>() as usize
}

#[must_use]
pub fn term_identity_cmp(left: &Term, right: &Term) -> i32 {
    cmp_usize(term_identity_id(left), term_identity_id(right))
}

/// Dereferences a term according to the requested limit.
///
/// Applied free-variable dereferencing follows the C LFHO top-node expansion
/// shape without populating the C owner-bank binding cache. Bank insertion
/// paths still use `TermBank` helpers when they need shared cache terms.
///
/// # Panics
///
/// Panics if a non-variable term has an active binding or if dereferencing
/// reaches an applied-variable shape with uninitialized arguments.
#[must_use]
pub fn term_deref(term: &Term, deref: &mut DerefType) -> Term {
    assert!(
        term.is_top_level_any_var() || term.binding().is_none(),
        "only variables may have active bindings"
    );
    if *deref == DerefType::Always {
        let mut current = term.clone();
        while let Some(next) = deref_step(&current) {
            current = next;
        }
        return current;
    }

    let mut current = term.clone();
    while *deref != DerefType::Never {
        let originally_app_var = current.is_applied_free_var();
        let Some(next) = deref_step(&current) else {
            break;
        };
        current = next;
        if originally_app_var {
            break;
        }
        *deref = DerefType::Never;
    }
    current
}

/// Sets properties on every term cell reachable from `term`.
///
/// # Panics
///
/// Panics if `deref` is `DerefType::Once`, matching the C assertion that this
/// traversal is never called with one-step dereferencing.
pub fn term_set_prop(term: &Term, deref: DerefType, prop: TermProperties) {
    assert_ne!(deref, DerefType::Once);
    walk_terms_with_deref(term, deref, |current| current.set_prop(prop));
}

#[must_use]
pub fn term_search_prop(term: &Term, deref: DerefType, prop: TermProperties) -> bool {
    walk_terms_with_deref_until(term, deref, |current| current.query_prop(prop))
}

/// Verifies that all reachable cells have exactly `expected` under `prop`.
///
/// # Panics
///
/// Panics if `deref` is `DerefType::Once`, matching the C assertion that this
/// traversal is never called with one-step dereferencing.
#[must_use]
pub fn term_verify_prop(
    term: &Term,
    deref: DerefType,
    prop: TermProperties,
    expected: TermProperties,
) -> bool {
    assert_ne!(deref, DerefType::Once);
    !walk_terms_with_deref_until(term, deref, |current| current.give_props(prop) != expected)
}

/// Deletes properties on every term cell reachable from `term`.
///
/// # Panics
///
/// Panics if `deref` is `DerefType::Once`, matching the C assertion that this
/// traversal is never called with one-step dereferencing.
pub fn term_del_prop(term: &Term, deref: DerefType, prop: TermProperties) {
    assert_ne!(deref, DerefType::Once);
    walk_terms_with_deref(term, deref, |current| current.del_prop(prop));
}

pub fn term_del_prop_opt(term: &Term, prop: TermProperties) {
    walk_terms(term, |current| current.del_prop(prop));
}

/// Sets properties on every free variable reachable from `term`.
///
/// # Panics
///
/// Panics if `deref` is `DerefType::Once`, matching the C assertion that this
/// traversal is never called with one-step dereferencing.
pub fn term_var_set_prop(term: &Term, deref: DerefType, prop: TermProperties) {
    assert_ne!(deref, DerefType::Once);
    walk_terms_with_deref(term, deref, |current| {
        if current.is_free_var() {
            current.set_prop(prop);
        }
    });
}

/// Searches for properties on reachable free variables.
///
/// # Panics
///
/// Panics if `deref` is `DerefType::Once`, matching the C assertion that this
/// traversal is never called with one-step dereferencing.
#[must_use]
pub fn term_var_search_prop(term: &Term, deref: DerefType, prop: TermProperties) -> bool {
    assert_ne!(deref, DerefType::Once);
    walk_terms_with_deref_until(term, deref, |current| {
        current.is_free_var() && current.query_prop(prop)
    })
}

/// Deletes properties on every free variable reachable from `term`.
///
/// # Panics
///
/// Panics if `deref` is `DerefType::Once`, matching the C assertion that this
/// traversal is never called with one-step dereferencing.
pub fn term_var_del_prop(term: &Term, deref: DerefType, prop: TermProperties) {
    assert_ne!(deref, DerefType::Once);
    walk_terms_with_deref(term, deref, |current| {
        if current.is_free_var() {
            current.del_prop(prop);
        }
    });
}

#[must_use]
pub fn term_has_interpreted_symbol(term: &Term) -> bool {
    walk_terms_until(term, |current| {
        current
            .type_()
            .is_some_and(|type_| sort_is_interpreted(type_.f_code()))
    })
}

#[must_use]
pub fn term_is_prefix(candidate: Option<&Term>, term: &Term) -> bool {
    let Some(candidate) = candidate else {
        return false;
    };
    if candidate.is_any_var() {
        return if term.is_any_var() {
            candidate == term
        } else {
            term.is_phony_app() && term.argument(0).is_some_and(|arg| &arg == candidate)
        };
    }

    if candidate.arity() > term.arity() || candidate.f_code() != term.f_code() {
        return false;
    }
    candidate
        .argument_clones()
        .into_iter()
        .zip(term.argument_clones())
        .take(candidate.arity())
        .all(|(left, right)| left.zip(right).is_some_and(|(left, right)| left == right))
}

pub fn term_stack_set_props(stack: &PStack<Term>, prop: TermProperties) {
    for term in stack.as_slice() {
        term.set_prop(prop);
    }
}

pub fn term_stack_del_props(stack: &PStack<Term>, prop: TermProperties) {
    for term in stack.as_slice() {
        term.del_prop(prop);
    }
}

fn deref_step(term: &Term) -> Option<Term> {
    if term.is_free_var() {
        return term.binding();
    }
    if term.is_applied_free_var()
        && term
            .argument(0)
            .is_some_and(|head| head.binding().is_some())
    {
        return Some(deref_applied_free_var_no_cache(term));
    }
    None
}

fn deref_applied_free_var_no_cache(term: &Term) -> Term {
    assert!(term.is_applied_free_var(), "expected applied free variable");
    assert!(term.arity() > 1, "applied variable must have arguments");
    let head = term.argument(0).expect("applied variable has a head");
    let binding = head.binding().expect("applied variable head is bound");

    let expanded = if binding.is_any_var() || binding.is_lambda() {
        let expanded = Term::top_alloc(term.f_code(), term.arity());
        expanded.set_properties(term.give_props(TP_PRED_POS));
        expanded.set_type(term.type_());
        expanded.set_argument(0, binding);
        for index in 1..term.arity() {
            let arg = term
                .argument(index)
                .unwrap_or_else(|| panic!("term argument {index} is uninitialized"));
            expanded.set_argument(index, arg);
        }
        expanded
    } else {
        let expanded = Term::top_alloc(binding.f_code(), binding.arity() + term.arity() - 1);
        expanded.set_properties(binding.give_props(TP_PRED_POS));
        expanded.set_type(term.type_());
        for index in 0..binding.arity() {
            let arg = binding
                .argument(index)
                .unwrap_or_else(|| panic!("binding argument {index} is uninitialized"));
            expanded.set_argument(index, arg);
        }
        for index in 1..term.arity() {
            let arg = term
                .argument(index)
                .unwrap_or_else(|| panic!("term argument {index} is uninitialized"));
            expanded.set_argument(binding.arity() + index - 1, arg);
        }
        expanded
    };
    expanded.set_prop(TP_IS_DEREFED_APP_VAR);
    expanded
}

fn rewrite_index(level: RewriteLevel) -> usize {
    match level {
        RewriteLevel::RuleRewrite => 0,
        RewriteLevel::FullRewrite => 1,
        RewriteLevel::NoRewrite => panic!("rewrite level has no date slot"),
    }
}

fn cmp_usize(left: usize, right: usize) -> i32 {
    i32::from(left > right) - i32::from(left < right)
}

fn walk_terms(term: &Term, mut visit: impl FnMut(&Term)) {
    let mut stack = vec![term.clone()];
    while let Some(current) = stack.pop() {
        visit(&current);
        for arg in current.argument_clones().into_iter().flatten() {
            stack.push(arg);
        }
    }
}

fn walk_terms_until(term: &Term, mut visit: impl FnMut(&Term) -> bool) -> bool {
    let mut stack = vec![term.clone()];
    while let Some(current) = stack.pop() {
        if visit(&current) {
            return true;
        }
        for arg in current.argument_clones().into_iter().flatten() {
            stack.push(arg);
        }
    }
    false
}

fn walk_terms_with_deref(term: &Term, deref: DerefType, mut visit: impl FnMut(&Term)) {
    let mut stack = vec![(term.clone(), deref)];
    while let Some((candidate, mut current_deref)) = stack.pop() {
        let current = term_deref(&candidate, &mut current_deref);
        visit(&current);
        for arg in current.argument_clones().into_iter().flatten() {
            stack.push((arg, current_deref));
        }
    }
}

fn walk_terms_with_deref_until(
    term: &Term,
    deref: DerefType,
    mut visit: impl FnMut(&Term) -> bool,
) -> bool {
    let mut stack = vec![(term.clone(), deref)];
    while let Some((candidate, mut current_deref)) = stack.pop() {
        let current = term_deref(&candidate, &mut current_deref);
        if visit(&current) {
            return true;
        }
        for arg in current.argument_clones().into_iter().flatten() {
            stack.push((arg, current_deref));
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::{
        term_del_prop, term_has_interpreted_symbol, term_is_prefix, term_search_prop,
        term_set_prop, term_stack_del_props, term_stack_set_props, term_var_del_prop,
        term_var_search_prop, term_var_set_prop, term_verify_prop, DerefType, RewriteLevel, Term,
        DEFAULT_FWEIGHT, DEFAULT_VWEIGHT, TERMS_INITIAL_ARGS, TP_CHECK_FLAG, TP_HAS_BOOL_SUBTERM,
        TP_HAS_ETA_EXPANDABLE_SUBTERM, TP_IGNORE_PROPS, TP_IS_DB_VAR, TP_IS_DEREFED_APP_VAR,
        TP_IS_REWRITTEN, TP_IS_SHARED, TP_OUTPUT_FLAG, TP_PRED_POS, TP_TOP_POS,
    };
    use crate::basics::pstacks::PStack;
    use crate::basics::sysdate::SysDate;
    use crate::terms::signature::SIG_PHONY_APP_CODE;
    use crate::terms::simpletypes::{alloc_simple_sort, ST_INTEGER};

    #[test]
    fn constants_match_c_header_values() {
        assert_eq!(DEFAULT_VWEIGHT, 1);
        assert_eq!(DEFAULT_FWEIGHT, 2);
        assert_eq!(TERMS_INITIAL_ARGS, 10);
        assert_eq!(TP_IGNORE_PROPS.bits(), 0);
        assert_eq!(TP_TOP_POS.bits(), 2);
        assert_eq!(TP_PRED_POS.bits(), 8);
        assert_eq!(TP_IS_SHARED.bits(), 16_384);
        assert_eq!(TP_HAS_ETA_EXPANDABLE_SUBTERM.bits(), 1_u64 << 25);
        assert_eq!(TP_HAS_BOOL_SUBTERM.bits(), 1_u64 << 30);

        let props = TP_PRED_POS | TP_IS_SHARED;
        assert!(props.is_any_set(TP_IS_SHARED | TP_OUTPUT_FLAG));
        assert_eq!(props.any_set(TP_IS_SHARED | TP_OUTPUT_FLAG), TP_IS_SHARED);
        assert_eq!(props.give(TP_PRED_POS | TP_OUTPUT_FLAG), TP_PRED_POS);
    }

    #[test]
    fn allocation_and_top_copy_preserve_c_cell_shape() {
        let source = Term::top_alloc(7, 2);
        let left = Term::const_cell_alloc(1);
        let right = Term::const_cell_alloc(2);
        source.set_argument(0, left.clone());
        source.set_argument(1, right.clone());
        source.set_prop(TP_PRED_POS | TP_OUTPUT_FLAG | TP_IS_DB_VAR);

        let without_args = Term::top_copy_without_args(&source);
        assert_eq!(without_args.f_code(), 7);
        assert_eq!(without_args.arity(), 2);
        assert!(without_args.query_prop(TP_PRED_POS | TP_IS_DB_VAR));
        assert!(!without_args.query_prop(TP_OUTPUT_FLAG));
        assert_eq!(
            without_args.any_prop_set(TP_PRED_POS | TP_OUTPUT_FLAG),
            TP_PRED_POS
        );
        assert!(without_args.argument(0).is_none());

        let copy = Term::top_copy(&source);
        assert_eq!(copy.argument(0), Some(left));
        assert_eq!(copy.argument(1), Some(right));
        assert!(copy.left_son().is_none());
        assert!(copy.right_son().is_none());
    }

    #[test]
    #[cfg(target_pointer_width = "64")]
    fn term_links_share_one_compact_interior_mutation_boundary() {
        assert_eq!(std::mem::size_of::<super::TermLinks>(), 40);
        assert_eq!(
            std::mem::size_of::<std::cell::RefCell<super::TermLinks>>(),
            48
        );
        assert_eq!(std::mem::size_of::<super::TermCell>(), 152);

        let term = Term::const_cell_alloc(1);
        let binding = Term::const_cell_alloc(2);
        let replacement = Term::const_cell_alloc(3);
        let left = Term::const_cell_alloc(4);
        let right = Term::const_cell_alloc(5);
        let type_ = alloc_simple_sort(ST_INTEGER);

        term.set_binding(Some(binding.clone()));
        term.set_rw_replace_field(Some(replacement.clone()));
        term.set_type(Some(type_.clone()));
        term.set_left_son(Some(left.clone()));
        term.set_right_son(Some(right.clone()));

        assert_eq!(term.binding(), Some(binding));
        assert_eq!(term.rw_replace_field(), Some(replacement));
        assert_eq!(term.type_(), Some(type_));
        assert_eq!(term.take_left_son(), Some(left));
        assert_eq!(term.take_right_son(), Some(right));
    }

    #[test]
    fn property_traversals_use_deref_and_cover_variables_separately() {
        let root = Term::top_alloc(10, 2);
        let var = Term::const_cell_alloc(-2);
        let bound = Term::const_cell_alloc(11);
        let leaf = Term::const_cell_alloc(12);
        var.set_binding(Some(bound.clone()));
        root.set_argument(0, var.clone());
        root.set_argument(1, leaf.clone());

        term_set_prop(&root, DerefType::Always, TP_CHECK_FLAG);
        assert!(root.query_prop(TP_CHECK_FLAG));
        assert!(bound.query_prop(TP_CHECK_FLAG));
        assert!(leaf.query_prop(TP_CHECK_FLAG));
        assert!(!var.query_prop(TP_CHECK_FLAG));
        assert!(term_search_prop(&root, DerefType::Always, TP_CHECK_FLAG));
        assert!(term_verify_prop(
            &root,
            DerefType::Always,
            TP_CHECK_FLAG,
            TP_CHECK_FLAG
        ));

        term_del_prop(&root, DerefType::Always, TP_CHECK_FLAG);
        assert!(!term_search_prop(&root, DerefType::Always, TP_CHECK_FLAG));

        term_var_set_prop(&root, DerefType::Never, TP_TOP_POS);
        assert!(var.query_prop(TP_TOP_POS));
        assert!(term_var_search_prop(&root, DerefType::Never, TP_TOP_POS));
        term_var_del_prop(&root, DerefType::Never, TP_TOP_POS);
        assert!(!var.query_prop(TP_TOP_POS));
    }

    #[test]
    fn deref_once_updates_remaining_limit_like_c() {
        let var = Term::const_cell_alloc(-2);
        let bound = Term::const_cell_alloc(4);
        var.set_binding(Some(bound.clone()));

        let mut deref = DerefType::Once;
        assert_eq!(super::term_deref(&var, &mut deref), bound);
        assert_eq!(deref, DerefType::Never);
    }

    #[test]
    fn deref_once_expands_applied_free_var_without_consuming_limit() {
        let var = Term::const_cell_alloc(-2);
        let prefix = Term::top_alloc(20, 1);
        let prefix_arg = Term::const_cell_alloc(21);
        prefix.set_argument(0, prefix_arg.clone());
        var.set_binding(Some(prefix));
        let suffix_arg = Term::const_cell_alloc(22);
        let app = Term::top_alloc(SIG_PHONY_APP_CODE, 2);
        app.set_argument(0, var);
        app.set_argument(1, suffix_arg.clone());
        app.set_prop(TP_PRED_POS);

        let mut deref = DerefType::Once;
        let expanded = super::term_deref(&app, &mut deref);

        assert_eq!(deref, DerefType::Once);
        assert_eq!(expanded.f_code(), 20);
        assert_eq!(expanded.arity(), 2);
        assert_eq!(expanded.argument(0), Some(prefix_arg));
        assert_eq!(expanded.argument(1), Some(suffix_arg));
        assert!(expanded.query_prop(TP_IS_DEREFED_APP_VAR));
        assert!(!expanded.query_prop(TP_PRED_POS));
    }

    #[test]
    fn deref_always_repeatedly_expands_bound_applied_free_var_heads() {
        let x = Term::const_cell_alloc(-2);
        let y = Term::const_cell_alloc(-4);
        let prefix = Term::top_alloc(30, 1);
        let prefix_arg = Term::const_cell_alloc(31);
        prefix.set_argument(0, prefix_arg.clone());
        y.set_binding(Some(prefix));
        x.set_binding(Some(y));
        let suffix_arg = Term::const_cell_alloc(32);
        let app = Term::top_alloc(SIG_PHONY_APP_CODE, 2);
        app.set_argument(0, x);
        app.set_argument(1, suffix_arg.clone());

        let mut deref = DerefType::Always;
        let expanded = super::term_deref(&app, &mut deref);

        assert_eq!(deref, DerefType::Always);
        assert_eq!(expanded.f_code(), 30);
        assert_eq!(expanded.arity(), 2);
        assert_eq!(expanded.argument(0), Some(prefix_arg));
        assert_eq!(expanded.argument(1), Some(suffix_arg));
    }

    #[test]
    fn optional_delete_and_stack_helpers_touch_only_present_cells() {
        let root = Term::top_alloc(10, 1);
        let leaf = Term::const_cell_alloc(11);
        root.set_argument(0, leaf.clone());
        root.set_prop(TP_CHECK_FLAG);
        leaf.set_prop(TP_CHECK_FLAG);

        super::term_del_prop_opt(&root, TP_CHECK_FLAG);
        assert!(!root.query_prop(TP_CHECK_FLAG));
        assert!(!leaf.query_prop(TP_CHECK_FLAG));

        let mut stack = PStack::new();
        stack.push(root.clone());
        stack.push(leaf.clone());
        term_stack_set_props(&stack, TP_TOP_POS);
        assert!(root.query_prop(TP_TOP_POS));
        assert!(leaf.query_prop(TP_TOP_POS));
        term_stack_del_props(&stack, TP_TOP_POS);
        assert!(!root.query_prop(TP_TOP_POS));
        assert!(!leaf.query_prop(TP_TOP_POS));
    }

    #[test]
    fn interpreted_symbol_search_checks_term_types() {
        let root = Term::top_alloc(10, 1);
        let leaf = Term::const_cell_alloc(11);
        root.set_argument(0, leaf.clone());
        assert!(!term_has_interpreted_symbol(&root));

        leaf.set_type(Some(alloc_simple_sort(ST_INTEGER)));
        assert!(term_has_interpreted_symbol(&root));
    }

    #[test]
    fn prefix_checks_match_pointer_identity_cases() {
        let f = Term::top_alloc(20, 2);
        let a = Term::const_cell_alloc(1);
        let b = Term::const_cell_alloc(2);
        f.set_argument(0, a.clone());
        f.set_argument(1, b);

        let prefix = Term::top_alloc(20, 1);
        prefix.set_argument(0, a.clone());
        assert!(term_is_prefix(Some(&prefix), &f));

        let different_a = Term::const_cell_alloc(1);
        let not_prefix = Term::top_alloc(20, 1);
        not_prefix.set_argument(0, different_a);
        assert!(!term_is_prefix(Some(&not_prefix), &f));

        let app = Term::top_alloc(SIG_PHONY_APP_CODE, 2);
        let var = Term::const_cell_alloc(-2);
        app.set_argument(0, var.clone());
        app.set_argument(1, a);
        assert!(term_is_prefix(Some(&var), &app));
    }

    #[test]
    fn rewrite_date_and_rewrite_flag_follow_macro_shape() {
        let term = Term::const_cell_alloc(1);
        assert_eq!(
            term.nf_date(RewriteLevel::RuleRewrite),
            SysDate::creation_time()
        );
        term.set_prop(TP_IS_REWRITTEN);
        assert_eq!(
            term.nf_date(RewriteLevel::FullRewrite),
            SysDate::creation_time()
        );
    }

    #[test]
    fn free_var_and_higher_order_predicates_match_macro_shapes() {
        let var = Term::const_cell_alloc(-2);
        assert!(var.is_free_var());
        assert!(var.is_any_var());
        assert!(var.is_top_level_free_var());

        let db = Term::const_cell_alloc(3);
        db.set_prop(TP_IS_DB_VAR);
        assert!(db.is_db_var());
        assert!(db.is_any_var());

        let app = Term::top_alloc(SIG_PHONY_APP_CODE, 1);
        app.set_argument(0, var);
        assert!(app.is_phony_app());
        assert!(app.is_applied_free_var());
        assert_eq!(app.arg_num(), 0);
    }

    #[test]
    fn tree_links_and_identity_comparison_are_handle_based() {
        let parent = Term::const_cell_alloc(1);
        let left = Term::const_cell_alloc(2);
        let right = Term::const_cell_alloc(3);

        parent.set_left_son(Some(left.clone()));
        parent.set_right_son(Some(right.clone()));
        assert_eq!(parent.left_son(), Some(left.clone()));
        assert_eq!(parent.right_son(), Some(right.clone()));
        assert_eq!(super::term_identity_cmp(&left, &left), 0);
        assert_ne!(super::term_identity_cmp(&left, &right), 0);

        assert_eq!(parent.take_left_son(), Some(left));
        parent.clear_tree_links();
        assert!(parent.left_son().is_none());
        assert!(parent.right_son().is_none());
    }
}
