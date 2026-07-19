use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
#[cfg(feature = "pdt-count-nodes")]
use std::sync::atomic::AtomicU64;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::basics::error::Diagnostic;
use crate::basics::intmap::{IntMap, IntMapKey};
use crate::basics::objmaps::size_of_obj_map_node_estimate;
use crate::basics::sysdate::SysDate;
use crate::clauses::derivation::ClauseDerivationRef;
use crate::clauses::eqn_props::EqnSide;
use crate::terms::functypes::FunCode;
use crate::terms::lambda::{lambda_eta_expand_db, lambda_eta_reduce_db};
#[cfg(test)]
use crate::terms::signature::{SIG_DB_LAMBDA_CODE, SIG_NAMED_LAMBDA_CODE, SIG_PHONY_APP_CODE};
use crate::terms::simpletypes::{TypeUniqueId, INVALID_TYPE_UID};
use crate::terms::subst::Substitution;
use crate::terms::termbanks::TermBank;
use crate::terms::termfunc::term_standard_weight;
use crate::terms::termtypes::{term_identity_id, Term};

pub const PDTREE_CELL_MEM: usize = 16;
pub const PDTNODE_MEM: usize = 52;
pub const CLAUSEPOSCELL_MEM: usize = 20;
pub const PDTREE_IGNORE_TERM_WEIGHT: i64 = i64::MAX;
pub const PDTREE_IGNORE_NF_DATE: SysDate = SysDate::creation_time();
const PDT_NO_VARIABLE_CHILD: u32 = u32::MAX;

static PDT_USE_SIZE_CONSTRAINTS: AtomicBool = AtomicBool::new(true);
static PDT_USE_AGE_CONSTRAINTS: AtomicBool = AtomicBool::new(true);

#[cfg(feature = "pdt-count-nodes")]
static PDT_NODE_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PdtConstraintSettings {
    pub use_size_constraints: bool,
    pub use_age_constraints: bool,
}

impl Default for PdtConstraintSettings {
    fn default() -> Self {
        Self {
            use_size_constraints: true,
            use_age_constraints: true,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PdtTraversalStep {
    Symbols,
    Variables,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PdtTraversalOrder {
    pub first: PdtTraversalStep,
    pub second: PdtTraversalStep,
}

impl PdtTraversalOrder {
    #[must_use]
    pub const fn symbols_first() -> Self {
        Self {
            first: PdtTraversalStep::Symbols,
            second: PdtTraversalStep::Variables,
        }
    }

    #[must_use]
    pub const fn variables_first() -> Self {
        Self {
            first: PdtTraversalStep::Variables,
            second: PdtTraversalStep::Symbols,
        }
    }

    #[must_use]
    pub const fn from_prefer_general(prefer_general: bool) -> Self {
        if prefer_general {
            Self::symbols_first()
        } else {
            Self::variables_first()
        }
    }
}

impl Default for PdtTraversalOrder {
    fn default() -> Self {
        Self::symbols_first()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PdtSearchState {
    query: Vec<PrefixQueryCell>,
    pub term_weight: i64,
    pub term_date: SysDate,
    pub traversal_order: PdtTraversalOrder,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum PrefixToken {
    Fun(FunCode),
    FreeVar {
        id: usize,
        type_uid: TypeUniqueId,
        weight: i64,
    },
    DbLike(usize),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PrefixMatch {
    pub matched: usize,
    pub remains: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PdtIndexedOccurrence {
    pub clause_id: i64,
    pub side: EqnSide,
    clause_ref: ClauseDerivationRef,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PrefixQueryCell {
    span: usize,
    term: Term,
}

impl PrefixQueryCell {
    fn token(&self) -> PrefixToken {
        prefix_token(&self.term)
    }

    fn type_uid(&self) -> TypeUniqueId {
        term_type_uid(&self.term)
    }

    fn weight(&self) -> i64 {
        term_standard_weight(&self.term)
    }
}

#[cfg(test)]
struct PrefixQueryMetadata {
    token: PrefixToken,
    type_uid: TypeUniqueId,
    weight: i64,
    first_arg: usize,
    traverses_arguments: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum PrefixQueryBuildFrame {
    Enter(Term),
    Exit(usize),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct QuerySubtree {
    start: usize,
    end: usize,
}

type PdtQueryBindings = BTreeMap<(usize, TypeUniqueId, i64), QuerySubtree>;

#[derive(Clone, Debug, PartialEq)]
pub struct PdTree {
    nodes: Vec<PdNode>,
    variable_child_heads: Vec<u32>,
    variable_children: Vec<PdtVariableChild>,
    free_variable_child: u32,
    normalized_occurrence_paths: BTreeMap<(ClauseDerivationRef, i32), PdtNormalizedOccurrence>,
    term_count: usize,
    live_node_count: usize,
    arr_storage_estimate: usize,
    match_count: Cell<u64>,
    visited_count: Cell<u64>,
    search_traversal_order: Cell<PdtTraversalOrder>,
    search_term_weight: Cell<i64>,
    search_term_date: Cell<SysDate>,
    search_active: Cell<bool>,
    search_state: RefCell<Option<PdtSearchState>>,
    search_query_scratch: RefCell<Vec<PrefixQueryCell>>,
    search_query_build_stack: RefCell<Vec<PrefixQueryBuildFrame>>,
    search_cursor: RefCell<Option<PdtOccurrenceCursor>>,
    search_subst_cursor: RefCell<PdtSubstCursor>,
}

#[derive(Clone, Debug, PartialEq)]
struct PdNode {
    children: BTreeMap<PrefixToken, usize>,
    fun_alternatives: IntMap<()>,
    ref_count: usize,
    terminal_count: usize,
    terminal_entries: Vec<PdTerminalEntry>,
    size_constr: i64,
    age_constr: SysDate,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PdtVariableChild {
    node_index: usize,
    next_sibling: u32,
    variable: Option<Term>,
    type_uid: TypeUniqueId,
    weight: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PdtNormalizedOccurrence {
    code: Vec<PrefixToken>,
    weight: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PdtSubstCursor {
    frames: Vec<PdtTraversalFrame>,
    bindings: Vec<(Term, Term)>,
    base_subst: usize,
    initialized: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct PdtTraversalFrame {
    node_index: usize,
    query_index: usize,
    effective_term_weight: i64,
    binding_pos: usize,
    entered: bool,
    next_step: usize,
    next_variable_child: u32,
    terminal_position: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PdtOccurrenceCursor {
    occurrences: Vec<PdtIndexedOccurrence>,
    position: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct PdTerminalEntry {
    weight: i64,
    date: Option<SysDate>,
    occurrence: Option<PdtIndexedOccurrence>,
}

impl PdtOccurrenceCursor {
    #[must_use]
    fn new(occurrences: Vec<PdtIndexedOccurrence>) -> Self {
        Self {
            occurrences,
            position: 0,
        }
    }

    fn next(&mut self) -> Option<PdtIndexedOccurrence> {
        let occurrence = self.occurrences.get(self.position).copied()?;
        self.position += 1;
        Some(occurrence)
    }
}

impl PdtSubstCursor {
    const fn new() -> Self {
        Self {
            frames: Vec::new(),
            bindings: Vec::new(),
            base_subst: 0,
            initialized: false,
        }
    }

    fn start(&mut self, term_weight: i64, base_subst: usize, first_variable_child: u32) {
        self.frames.push(PdtTraversalFrame::new(
            0,
            0,
            term_weight,
            0,
            first_variable_child,
        ));
        self.base_subst = base_subst;
        self.initialized = true;
    }

    fn reset(&mut self) {
        self.frames.clear();
        self.bindings.clear();
        self.initialized = false;
    }
}

impl PdtTraversalFrame {
    const fn new(
        node_index: usize,
        query_index: usize,
        effective_term_weight: i64,
        binding_pos: usize,
        first_variable_child: u32,
    ) -> Self {
        Self {
            node_index,
            query_index,
            effective_term_weight,
            binding_pos,
            entered: false,
            next_step: 0,
            next_variable_child: first_variable_child,
            terminal_position: 0,
        }
    }
}

impl Default for PdTerminalEntry {
    fn default() -> Self {
        Self {
            weight: PDTREE_IGNORE_TERM_WEIGHT,
            date: None,
            occurrence: None,
        }
    }
}

impl Default for PdNode {
    fn default() -> Self {
        Self {
            children: BTreeMap::new(),
            fun_alternatives: IntMap::new(),
            ref_count: 0,
            terminal_count: 0,
            terminal_entries: Vec::new(),
            size_constr: PDTREE_IGNORE_TERM_WEIGHT,
            age_constr: SysDate::creation_time(),
        }
    }
}

impl PdTerminalEntry {
    #[must_use]
    const fn new(weight: i64, date: Option<SysDate>) -> Self {
        Self {
            weight,
            date,
            occurrence: None,
        }
    }

    #[must_use]
    const fn with_occurrence(
        weight: i64,
        date: Option<SysDate>,
        occurrence: PdtIndexedOccurrence,
    ) -> Self {
        Self {
            weight,
            date,
            occurrence: Some(occurrence),
        }
    }

    #[must_use]
    fn matches_target(self, target: Self) -> bool {
        self.weight == target.weight
            && self.date == target.date
            && target
                .occurrence
                .is_none_or(|occurrence| self.occurrence == Some(occurrence))
    }
}

impl PdtIndexedOccurrence {
    #[must_use]
    pub const fn new(clause_id: i64, side: EqnSide) -> Self {
        Self {
            clause_id,
            side,
            clause_ref: ClauseDerivationRef::new(clause_id, 0),
        }
    }

    #[must_use]
    pub const fn with_clause_ref(clause_ref: ClauseDerivationRef, side: EqnSide) -> Self {
        Self {
            clause_id: clause_ref.ident(),
            side,
            clause_ref,
        }
    }

    #[must_use]
    pub const fn clause_id(self) -> i64 {
        self.clause_id
    }

    #[must_use]
    pub const fn clause_ref(self) -> ClauseDerivationRef {
        self.clause_ref
    }
}

const fn occurrence_key(occurrence: PdtIndexedOccurrence) -> (ClauseDerivationRef, i32) {
    (occurrence.clause_ref, occurrence.side as i32)
}

impl Default for PdTree {
    fn default() -> Self {
        Self::new()
    }
}

impl PdTree {
    #[must_use]
    pub fn new() -> Self {
        Self {
            nodes: vec![PdNode::default()],
            variable_child_heads: vec![PDT_NO_VARIABLE_CHILD],
            variable_children: Vec::new(),
            free_variable_child: PDT_NO_VARIABLE_CHILD,
            normalized_occurrence_paths: BTreeMap::new(),
            term_count: 0,
            live_node_count: 0,
            arr_storage_estimate: 0,
            match_count: Cell::new(0),
            visited_count: Cell::new(0),
            search_traversal_order: Cell::new(PdtTraversalOrder::default()),
            search_term_weight: Cell::new(PDTREE_IGNORE_TERM_WEIGHT),
            search_term_date: Cell::new(PDTREE_IGNORE_NF_DATE),
            search_active: Cell::new(false),
            search_state: RefCell::new(None),
            search_query_scratch: RefCell::new(Vec::new()),
            search_query_build_stack: RefCell::new(Vec::new()),
            search_cursor: RefCell::new(None),
            search_subst_cursor: RefCell::new(PdtSubstCursor::new()),
        }
    }

    #[must_use]
    pub fn from_codes<I, C>(codes: I) -> Self
    where
        I: IntoIterator<Item = C>,
        C: AsRef<[PrefixToken]>,
    {
        let mut tree = Self::new();
        for code in codes {
            tree.insert_code(code.as_ref());
        }
        tree
    }

    #[must_use]
    pub fn node_count(&self) -> usize {
        self.live_node_count
    }

    #[must_use]
    pub const fn term_count(&self) -> usize {
        self.term_count
    }

    #[must_use]
    pub const fn arr_storage_estimate(&self) -> usize {
        self.arr_storage_estimate
    }

    #[must_use]
    pub fn storage_estimate(&self) -> usize {
        self.live_node_count
            .saturating_mul(PDTNODE_MEM)
            .saturating_add(self.arr_storage_estimate)
            .saturating_add(
                self.term_count
                    .saturating_mul(PDTREE_CELL_MEM.saturating_add(CLAUSEPOSCELL_MEM)),
            )
    }

    #[must_use]
    pub fn match_count(&self) -> u64 {
        self.match_count.get()
    }

    #[must_use]
    pub fn visited_count(&self) -> u64 {
        self.visited_count.get()
    }

    #[must_use]
    pub fn size_constraint(&self) -> i64 {
        self.nodes[0].size_constr
    }

    #[must_use]
    pub fn age_constraint(&self) -> SysDate {
        self.nodes[0].age_constr
    }

    #[must_use]
    pub fn root_satisfies_constraints(&self, term_weight: i64, term_date: SysDate) -> bool {
        self.node_satisfies_constraints(0, term_weight, term_date)
    }

    #[must_use]
    pub fn search_root_satisfies_constraints(&self) -> bool {
        self.search_state
            .borrow()
            .as_ref()
            .is_none_or(|state| self.root_satisfies_constraints(state.term_weight, state.term_date))
    }

    #[must_use]
    pub fn search_root_may_have_matchable_path(&self) -> bool {
        let state = self.search_state.borrow();
        let Some(state) = state.as_ref() else {
            return true;
        };
        self.query_may_have_matchable_path(&state.query)
    }

    #[must_use]
    pub fn search_traversal_order(&self) -> PdtTraversalOrder {
        self.search_traversal_order.get()
    }

    #[must_use]
    pub fn search_state(&self) -> Option<PdtSearchState> {
        self.search_state.borrow().clone()
    }

    #[must_use]
    pub fn search_is_active(&self) -> bool {
        self.search_active.get()
    }

    #[must_use]
    pub fn search_term_weight(&self) -> i64 {
        self.search_term_weight.get()
    }

    #[must_use]
    pub fn search_term_date(&self) -> SysDate {
        self.search_term_date.get()
    }

    pub fn record_search_attempt(&self) {
        self.match_count
            .set(self.match_count.get().saturating_add(1));
    }

    /// Initializes this tree's single active search.
    ///
    /// # Panics
    ///
    /// Panics if query construction finds an uninitialized term argument or
    /// fails to emit the root query cell.
    pub fn record_search_init(&self, term: &Term, age_constraint: SysDate, prefer_general: bool) {
        debug_assert!(
            !self.search_active.get(),
            "PDTreeSearchInit requires no active search"
        );
        self.recycle_search_query();
        let traversal_order = PdtTraversalOrder::from_prefer_general(prefer_general);
        let query = self.build_search_query(term);
        let term_weight = query
            .first()
            .expect("PDTree search query contains its root term")
            .weight();
        self.search_traversal_order.set(traversal_order);
        self.search_term_weight.set(term_weight);
        self.search_term_date.set(age_constraint);
        *self.search_state.borrow_mut() = Some(PdtSearchState {
            query,
            term_weight,
            term_date: age_constraint,
            traversal_order,
        });
        *self.search_cursor.borrow_mut() = None;
        self.search_subst_cursor.borrow_mut().reset();
        self.search_active.set(true);
        self.record_search_attempt();
    }

    /// Initializes a search after applying C's `PDTree` eta-normalization rule.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if eta expansion or reduction cannot insert a
    /// rebuilt term into `bank`.
    pub fn record_search_init_with_bank(
        &self,
        bank: &mut TermBank,
        term: &Term,
        age_constraint: SysDate,
        prefer_general: bool,
    ) -> Result<(), Diagnostic> {
        if pd_tree_term_needs_eta_normalization(term) {
            let normalized = normalize_pd_tree_term(bank, term)?;
            self.record_search_init(&normalized, age_constraint, prefer_general);
        } else {
            self.record_search_init(term, age_constraint, prefer_general);
        }
        Ok(())
    }

    pub fn record_search_exit(&self) {
        self.search_active.set(false);
        self.recycle_search_query();
        *self.search_cursor.borrow_mut() = None;
        self.search_subst_cursor.borrow_mut().reset();
    }

    fn build_search_query(&self, term: &Term) -> Vec<PrefixQueryCell> {
        let mut query = self.search_query_scratch.borrow_mut();
        debug_assert!(
            query.is_empty(),
            "PDTree query scratch is empty between searches"
        );
        let mut build_stack = self.search_query_build_stack.borrow_mut();
        debug_assert!(
            build_stack.is_empty(),
            "PDTree query build stack is empty between searches"
        );
        build_stack.push(PrefixQueryBuildFrame::Enter(term.clone()));

        while let Some(frame) = build_stack.pop() {
            match frame {
                PrefixQueryBuildFrame::Enter(term) => {
                    let start = query.len();
                    let arity = term.arity();
                    let traverses_arguments = !term.is_top_level_free_var();

                    if traverses_arguments {
                        build_stack.push(PrefixQueryBuildFrame::Exit(start));
                        let arguments = term.arguments();
                        let first_arg = usize::from(term.is_lambda() || term.is_applied_db_var());
                        for index in (first_arg..arity).rev() {
                            let argument = arguments[index].clone().unwrap_or_else(|| {
                                panic!("term argument {index} is uninitialized")
                            });
                            build_stack.push(PrefixQueryBuildFrame::Enter(argument));
                        }
                    }

                    query.push(PrefixQueryCell {
                        span: usize::from(!traverses_arguments),
                        term,
                    });
                }
                PrefixQueryBuildFrame::Exit(start) => {
                    query[start].span = query.len() - start;
                }
            }
        }

        std::mem::take(&mut *query)
    }

    fn recycle_search_query(&self) {
        let Some(mut state) = self.search_state.borrow_mut().take() else {
            return;
        };
        state.query.clear();
        let mut scratch = self.search_query_scratch.borrow_mut();
        debug_assert!(scratch.is_empty(), "PDTree query scratch has one owner");
        *scratch = state.query;
    }

    pub fn record_nodes_visited(&self, count: u64) {
        self.visited_count
            .set(self.visited_count.get().saturating_add(count));
        #[cfg(feature = "pdt-count-nodes")]
        record_global_nodes_visited(count);
    }

    pub fn insert_term(&mut self, term: &Term) -> bool {
        let entry = PdTerminalEntry::new(term_standard_weight(term), None);
        self.insert_term_with_entry(term, entry)
    }

    pub fn insert_term_with_clause_date(&mut self, term: &Term, clause_date: SysDate) -> bool {
        let entry = PdTerminalEntry::new(term_standard_weight(term), Some(clause_date));
        self.insert_term_with_entry(term, entry)
    }

    pub fn insert_term_occurrence(
        &mut self,
        term: &Term,
        clause_date: SysDate,
        occurrence: PdtIndexedOccurrence,
    ) -> bool {
        let entry = PdTerminalEntry::with_occurrence(
            term_standard_weight(term),
            Some(clause_date),
            occurrence,
        );
        self.insert_term_with_entry(term, entry)
    }

    /// Inserts an occurrence after applying C's `PDTree` eta-normalization rule.
    ///
    /// The normalized prefix code is retained so later clause extraction can
    /// delete the occurrence without mutably borrowing the term bank again.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if eta expansion or reduction cannot insert a
    /// rebuilt term into `bank`.
    pub fn insert_term_occurrence_with_bank(
        &mut self,
        bank: &mut TermBank,
        term: &Term,
        clause_date: SysDate,
        occurrence: PdtIndexedOccurrence,
    ) -> Result<bool, Diagnostic> {
        if !pd_tree_term_needs_eta_normalization(term) {
            return Ok(self.insert_term_occurrence(term, clause_date, occurrence));
        }
        let normalized = normalize_pd_tree_term(bank, term)?;
        Ok(self.insert_normalized_term_occurrence(term, &normalized, clause_date, occurrence))
    }

    /// Inserts a caller-normalized occurrence and retains any changed path for deletion.
    pub(crate) fn insert_normalized_term_occurrence(
        &mut self,
        original: &Term,
        normalized: &Term,
        clause_date: SysDate,
        occurrence: PdtIndexedOccurrence,
    ) -> bool {
        let changed = normalized != original;
        let weight = term_standard_weight(normalized);
        let code = changed.then(|| term_lr_traverse_code(normalized));
        let entry = PdTerminalEntry::with_occurrence(weight, Some(clause_date), occurrence);
        let inserted = self.insert_term_with_entry(normalized, entry);
        if inserted {
            if let Some(code) = code {
                self.normalized_occurrence_paths.insert(
                    occurrence_key(occurrence),
                    PdtNormalizedOccurrence { code, weight },
                );
            }
        }
        inserted
    }

    pub fn insert_code(&mut self, code: &[PrefixToken]) -> bool {
        let weight = i64::try_from(code.len()).unwrap_or(PDTREE_IGNORE_TERM_WEIGHT);
        self.insert_code_with_metadata(code, weight, None)
    }

    fn insert_code_with_metadata(
        &mut self,
        code: &[PrefixToken],
        weight: i64,
        date: Option<SysDate>,
    ) -> bool {
        let entry = PdTerminalEntry::new(weight, date);
        self.insert_code_with_entry(code, entry)
    }

    fn insert_term_with_entry(&mut self, term: &Term, entry: PdTerminalEntry) -> bool {
        let path = term_lr_traverse_path(term);
        self.insert_path_with_entry(path, entry)
    }

    fn insert_code_with_entry(&mut self, code: &[PrefixToken], entry: PdTerminalEntry) -> bool {
        self.insert_path_with_entry(code.iter().copied().map(|token| (token, None)), entry)
    }

    fn insert_path_with_entry<I>(&mut self, path: I, entry: PdTerminalEntry) -> bool
    where
        I: IntoIterator<Item = (PrefixToken, Option<Term>)>,
    {
        let mut node_index = 0;
        self.nodes[node_index].ref_count += 1;
        self.apply_entry_to_node(node_index, entry);

        for (token, indexed_variable) in path {
            let parent_index = node_index;
            self.select_alt_ref_for_insert(parent_index, token);
            let next_index =
                if let Some(existing) = self.nodes[parent_index].children.get(&token).copied() {
                    existing
                } else {
                    let created = self.nodes.len();
                    self.nodes.push(PdNode::default());
                    self.variable_child_heads.push(PDT_NO_VARIABLE_CHILD);
                    self.nodes[parent_index].children.insert(token, created);
                    self.live_node_count += 1;
                    self.arr_storage_estimate = self.arr_storage_estimate.saturating_add(
                        self.nodes[created]
                            .fun_alternatives
                            .constant_mem_storage_estimate(),
                    );
                    created
                };
            if let Some(variable) = indexed_variable {
                self.link_variable_child(parent_index, next_index, token, variable);
            }
            node_index = next_index;
            self.nodes[node_index].ref_count += 1;
            self.apply_entry_to_node(node_index, entry);
        }

        self.nodes[node_index].terminal_count += 1;
        self.nodes[node_index].terminal_entries.push(entry);
        self.term_count += 1;
        true
    }

    fn link_variable_child(
        &mut self,
        parent_index: usize,
        child_index: usize,
        token: PrefixToken,
        variable: Term,
    ) {
        let PrefixToken::FreeVar {
            type_uid, weight, ..
        } = token
        else {
            return;
        };
        let mut previous_link = PDT_NO_VARIABLE_CHILD;
        let mut current_link = self.variable_child_heads[parent_index];
        while let Some(current_index) = unpack_variable_child_index(current_link) {
            let current = &self.variable_children[current_index];
            let current_token = prefix_token(
                current
                    .variable
                    .as_ref()
                    .expect("linked variable child has an indexed variable"),
            );
            if token == current_token {
                debug_assert_eq!(current.node_index, child_index);
                debug_assert_eq!(current.variable.as_ref(), Some(&variable));
                return;
            }
            if token < current_token {
                break;
            }
            previous_link = current_link;
            current_link = current.next_sibling;
        }

        let child_link =
            self.allocate_variable_child(child_index, variable, type_uid, weight, current_link);
        if let Some(previous_index) = unpack_variable_child_index(previous_link) {
            self.variable_children[previous_index].next_sibling = child_link;
        } else {
            self.variable_child_heads[parent_index] = child_link;
        }
    }

    fn unlink_variable_child(&mut self, parent_index: usize, child_index: usize) {
        let mut previous_link = PDT_NO_VARIABLE_CHILD;
        let mut current_link = self.variable_child_heads[parent_index];
        while let Some(current_index) = unpack_variable_child_index(current_link) {
            if self.variable_children[current_index].node_index == child_index {
                let next_link = self.variable_children[current_index].next_sibling;
                if let Some(previous_index) = unpack_variable_child_index(previous_link) {
                    self.variable_children[previous_index].next_sibling = next_link;
                } else {
                    self.variable_child_heads[parent_index] = next_link;
                }
                self.variable_children[current_index].node_index = 0;
                self.variable_children[current_index].variable = None;
                self.variable_children[current_index].next_sibling = self.free_variable_child;
                self.free_variable_child = current_link;
                return;
            }
            previous_link = current_link;
            current_link = self.variable_children[current_index].next_sibling;
        }
    }

    fn allocate_variable_child(
        &mut self,
        node_index: usize,
        variable: Term,
        type_uid: TypeUniqueId,
        weight: i64,
        next_sibling: u32,
    ) -> u32 {
        if let Some(index) = unpack_variable_child_index(self.free_variable_child) {
            let link = self.free_variable_child;
            self.free_variable_child = self.variable_children[index].next_sibling;
            self.variable_children[index] = PdtVariableChild {
                node_index,
                next_sibling,
                variable: Some(variable),
                type_uid,
                weight,
            };
            link
        } else {
            let link = pack_variable_child_index(self.variable_children.len());
            self.variable_children.push(PdtVariableChild {
                node_index,
                next_sibling,
                variable: Some(variable),
                type_uid,
                weight,
            });
            link
        }
    }

    pub fn delete_term(&mut self, term: &Term) -> bool {
        let code = term_lr_traverse_code(term);
        self.delete_code(&code)
    }

    pub fn delete_term_with_clause_date(&mut self, term: &Term, clause_date: SysDate) -> bool {
        let code = term_lr_traverse_code(term);
        let entry = PdTerminalEntry::new(term_standard_weight(term), Some(clause_date));
        self.delete_code_with_entry(&code, Some(entry))
    }

    pub fn delete_term_occurrence(
        &mut self,
        term: &Term,
        clause_date: SysDate,
        occurrence: PdtIndexedOccurrence,
    ) -> bool {
        let key = occurrence_key(occurrence);
        let normalized = self.normalized_occurrence_paths.get(&key).cloned();
        let code = normalized
            .as_ref()
            .map_or_else(|| term_lr_traverse_code(term), |stored| stored.code.clone());
        let weight = normalized
            .as_ref()
            .map_or_else(|| term_standard_weight(term), |stored| stored.weight);
        let entry = PdTerminalEntry::with_occurrence(weight, Some(clause_date), occurrence);
        let deleted = self.delete_code_with_entry(&code, Some(entry));
        if deleted && normalized.is_some() {
            self.normalized_occurrence_paths.remove(&key);
        }
        deleted
    }

    pub fn delete_code(&mut self, code: &[PrefixToken]) -> bool {
        self.delete_code_with_entry(code, None)
    }

    fn delete_code_with_entry(
        &mut self,
        code: &[PrefixToken],
        target_entry: Option<PdTerminalEntry>,
    ) -> bool {
        let mut node_index = 0;
        let mut path = Vec::with_capacity(code.len());

        for token in code {
            let Some(next_index) = self.nodes[node_index].children.get(token).copied() else {
                return false;
            };
            path.push((node_index, *token, next_index));
            node_index = next_index;
        }

        if self.nodes[node_index].terminal_count == 0 {
            return false;
        }
        let terminal_position = if let Some(target) = target_entry {
            self.nodes[node_index]
                .terminal_entries
                .iter()
                .position(|entry| entry.matches_target(target))
        } else {
            self.nodes[node_index].terminal_entries.len().checked_sub(1)
        };
        let Some(terminal_position) = terminal_position else {
            return false;
        };

        self.nodes[node_index].terminal_count -= 1;
        self.nodes[node_index]
            .terminal_entries
            .remove(terminal_position);
        self.term_count -= 1;
        self.nodes[0].ref_count -= 1;

        for (_, _, path_node_index) in &path {
            self.nodes[*path_node_index].ref_count -= 1;
        }

        let affected_path: Vec<_> = path
            .iter()
            .map(|(_, _, path_node_index)| *path_node_index)
            .collect();
        for (parent_index, token, dead_index) in path.into_iter().rev() {
            if self.nodes[dead_index].ref_count != 0 {
                break;
            }
            self.arr_storage_estimate = self.arr_storage_estimate.saturating_sub(
                self.nodes[dead_index]
                    .fun_alternatives
                    .constant_mem_storage_estimate(),
            );
            match token {
                PrefixToken::Fun(code) => {
                    let _ = self.nodes[parent_index]
                        .fun_alternatives
                        .del_key(fun_code_key(code));
                }
                PrefixToken::FreeVar { .. } | PrefixToken::DbLike(_) => {
                    self.arr_storage_estimate = self
                        .arr_storage_estimate
                        .saturating_sub(size_of_obj_map_node_estimate());
                }
            }
            if matches!(token, PrefixToken::FreeVar { .. }) {
                self.unlink_variable_child(parent_index, dead_index);
            }
            self.nodes[parent_index].children.remove(&token);
            self.nodes[dead_index].children.clear();
            self.nodes[dead_index].fun_alternatives = IntMap::new();
            self.nodes[dead_index].terminal_count = 0;
            self.nodes[dead_index].terminal_entries.clear();
            self.nodes[dead_index].size_constr = PDTREE_IGNORE_TERM_WEIGHT;
            self.nodes[dead_index].age_constr = SysDate::creation_time();
            self.live_node_count -= 1;
        }

        let mut affected = Vec::with_capacity(code.len().saturating_add(1));
        affected.push(0);
        affected.extend(
            affected_path
                .into_iter()
                .filter(|path_node_index| self.nodes[*path_node_index].ref_count != 0),
        );
        for index in affected.into_iter().rev() {
            self.recompute_node_constraints(index);
        }

        true
    }

    fn select_alt_ref_for_insert(&mut self, node_index: usize, token: PrefixToken) {
        match token {
            PrefixToken::Fun(code) => {
                let before = self.nodes[node_index]
                    .fun_alternatives
                    .constant_mem_storage_estimate();
                let slot = self.nodes[node_index]
                    .fun_alternatives
                    .get_ref(fun_code_key(code));
                if slot.is_none() {
                    *slot = Some(());
                }
                let after = self.nodes[node_index]
                    .fun_alternatives
                    .constant_mem_storage_estimate();
                self.apply_arr_storage_delta(before, after);
            }
            PrefixToken::FreeVar { .. } | PrefixToken::DbLike(_) => {
                if !self.nodes[node_index].children.contains_key(&token) {
                    self.arr_storage_estimate = self
                        .arr_storage_estimate
                        .saturating_add(size_of_obj_map_node_estimate());
                }
            }
        }
    }

    fn apply_entry_to_node(&mut self, node_index: usize, entry: PdTerminalEntry) {
        self.nodes[node_index].size_constr = self.nodes[node_index].size_constr.min(entry.weight);
        if let Some(date) = entry.date {
            self.nodes[node_index].age_constr = self.nodes[node_index].age_constr.maximum(date);
        }
    }

    fn recompute_node_constraints(&mut self, node_index: usize) {
        let mut size_constr = PDTREE_IGNORE_TERM_WEIGHT;
        let mut age_constr = SysDate::creation_time();
        for entry in &self.nodes[node_index].terminal_entries {
            size_constr = size_constr.min(entry.weight);
            if let Some(date) = entry.date {
                age_constr = age_constr.maximum(date);
            }
        }

        let child_indices: Vec<_> = self.nodes[node_index].children.values().copied().collect();
        for child_index in child_indices {
            size_constr = size_constr.min(self.nodes[child_index].size_constr);
            age_constr = age_constr.maximum(self.nodes[child_index].age_constr);
        }

        self.nodes[node_index].size_constr = size_constr;
        self.nodes[node_index].age_constr = age_constr;
    }

    fn node_satisfies_constraints(
        &self,
        node_index: usize,
        term_weight: i64,
        term_date: SysDate,
    ) -> bool {
        let settings = pdt_constraint_settings();
        if settings.use_size_constraints && term_weight < self.nodes[node_index].size_constr {
            return false;
        }

        if settings.use_age_constraints
            && term_date != PDTREE_IGNORE_NF_DATE
            && !term_date.is_earlier_than(self.nodes[node_index].age_constr)
        {
            return false;
        }
        true
    }

    fn apply_arr_storage_delta(&mut self, before: usize, after: usize) {
        if after >= before {
            self.arr_storage_estimate = self
                .arr_storage_estimate
                .saturating_add(after.saturating_sub(before));
        } else {
            self.arr_storage_estimate = self
                .arr_storage_estimate
                .saturating_sub(before.saturating_sub(after));
        }
    }

    pub fn delete_code_occurrences(&mut self, code: &[PrefixToken]) -> usize {
        let mut deleted = 0;
        while self.delete_code(code) {
            deleted += 1;
        }
        deleted
    }

    #[must_use]
    pub fn match_prefix(&self, term: &Term) -> PrefixMatch {
        let code = term_lr_traverse_code(term);
        self.match_code_prefix(&code)
    }

    #[must_use]
    pub fn match_code_prefix(&self, code: &[PrefixToken]) -> PrefixMatch {
        let mut current = Some(0);
        let mut matched = 0;
        let mut remains = 0;

        for token in code {
            let Some(node_index) = current else {
                remains += 1;
                continue;
            };
            if let Some(next_index) = self.nodes[node_index].children.get(token).copied() {
                matched += 1;
                current = Some(next_index);
            } else {
                remains += 1;
                current = None;
            }
        }

        PrefixMatch { matched, remains }
    }

    #[must_use]
    pub fn prefix_ref_count(&self, code: &[PrefixToken]) -> usize {
        let mut node_index = 0;
        for token in code {
            let Some(next_index) = self.nodes[node_index].children.get(token).copied() else {
                return 0;
            };
            node_index = next_index;
        }
        self.nodes[node_index].ref_count
    }

    #[must_use]
    pub fn search_matching_occurrences(&self) -> Option<Vec<PdtIndexedOccurrence>> {
        let state = self.search_state.borrow();
        let state = state.as_ref()?;
        let mut occurrences = Vec::new();
        let mut bindings = PdtQueryBindings::new();
        self.collect_matching_occurrences(
            0,
            0,
            state,
            state.term_weight,
            &mut bindings,
            &mut occurrences,
        );
        Some(occurrences)
    }

    pub fn search_next_matching_occurrence(&self) -> Option<PdtIndexedOccurrence> {
        if self.search_cursor.borrow().is_none() {
            let occurrences = self.search_matching_occurrences()?;
            *self.search_cursor.borrow_mut() = Some(PdtOccurrenceCursor::new(occurrences));
        }

        self.search_cursor.borrow_mut().as_mut()?.next()
    }

    /// Returns the next first-order indexed match while keeping its bindings
    /// active in `subst`, matching C `PDTreeFindNextDemodulator`.
    #[expect(
        clippy::too_many_lines,
        reason = "Keeps the cursor traversal and backtracking state machine together"
    )]
    pub fn search_next_matching_occurrence_with_subst(
        &self,
        subst: &mut Substitution,
    ) -> Option<PdtIndexedOccurrence> {
        let state = self.search_state.borrow();
        let state = state.as_ref()?;
        let mut cursor = self.search_subst_cursor.borrow_mut();
        if !cursor.initialized {
            cursor.start(state.term_weight, subst.len(), self.variable_child_heads[0]);
        }
        debug_assert!(
            subst.len() >= cursor.base_subst,
            "PDTree cursor substitution was externally backtracked"
        );
        subst.backtrack_to_pos(cursor.base_subst);

        loop {
            let frame_index = cursor.frames.len().checked_sub(1)?;
            let node_index = cursor.frames[frame_index].node_index;
            let query_index = cursor.frames[frame_index].query_index;

            if !cursor.frames[frame_index].entered {
                cursor.frames[frame_index].entered = true;
                if !self.node_satisfies_constraints(
                    node_index,
                    cursor.frames[frame_index].effective_term_weight,
                    state.term_date,
                ) {
                    pop_subst_cursor_frame(&mut cursor);
                    continue;
                }
                if query_index == state.query.len() {
                    cursor.frames[frame_index].terminal_position =
                        self.nodes[node_index].terminal_entries.len();
                }
            }

            if query_index == state.query.len() {
                let terminal_position = cursor.frames[frame_index].terminal_position;
                if terminal_position == 0 {
                    pop_subst_cursor_frame(&mut cursor);
                    continue;
                }
                cursor.frames[frame_index].terminal_position -= 1;
                if let Some(occurrence) =
                    self.nodes[node_index].terminal_entries[terminal_position - 1].occurrence
                {
                    for (variable, binding) in &cursor.bindings {
                        debug_assert!(
                            variable.binding().is_none(),
                            "speculative PDTree binding leaked into the shared term"
                        );
                        subst.add_binding(variable, binding);
                    }
                    return Some(occurrence);
                }
                continue;
            }

            let step_index = cursor.frames[frame_index].next_step;
            if step_index >= 2 {
                pop_subst_cursor_frame(&mut cursor);
                continue;
            }
            let step = if step_index == 0 {
                state.traversal_order.first
            } else {
                state.traversal_order.second
            };

            match step {
                PdtTraversalStep::Symbols => {
                    cursor.frames[frame_index].next_step += 1;
                    let token = state.query[query_index].token();
                    if matches!(token, PrefixToken::FreeVar { .. }) {
                        continue;
                    }
                    let Some(next_index) = self.nodes[node_index].children.get(&token).copied()
                    else {
                        continue;
                    };
                    self.record_nodes_visited(1);
                    let effective_term_weight = cursor.frames[frame_index].effective_term_weight;
                    let binding_pos = cursor.bindings.len();
                    cursor.frames.push(PdtTraversalFrame::new(
                        next_index,
                        query_index + 1,
                        effective_term_weight,
                        binding_pos,
                        self.variable_child_heads[next_index],
                    ));
                }
                PdtTraversalStep::Variables => {
                    let variable_link = cursor.frames[frame_index].next_variable_child;
                    let Some(variable_index) = unpack_variable_child_index(variable_link) else {
                        cursor.frames[frame_index].next_step += 1;
                        continue;
                    };
                    let variable_child = &self.variable_children[variable_index];
                    cursor.frames[frame_index].next_variable_child = variable_child.next_sibling;
                    let next_index = variable_child.node_index;
                    let variable = variable_child.variable.as_ref()?;
                    if state.query[query_index].type_uid() != variable_child.type_uid {
                        continue;
                    }
                    let next_query_index =
                        query_index.saturating_add(state.query[query_index].span);
                    if next_query_index > state.query.len() {
                        continue;
                    }
                    let binding_pos = cursor.bindings.len();
                    let query_term = &state.query[query_index].term;
                    let matched = if let Some((_, bound)) = cursor
                        .bindings
                        .iter()
                        .rev()
                        .find(|(candidate, _)| candidate == variable)
                    {
                        *bound == *query_term
                    } else if let Some(bound) = variable.binding() {
                        bound == *query_term
                    } else {
                        cursor.bindings.push((variable.clone(), query_term.clone()));
                        true
                    };
                    if !matched {
                        cursor.bindings.truncate(binding_pos);
                        continue;
                    }
                    self.record_nodes_visited(1);
                    let effective_term_weight = adjusted_variable_edge_weight(
                        cursor.frames[frame_index].effective_term_weight,
                        state.query[query_index].weight(),
                        variable_child.weight,
                    );
                    cursor.frames.push(PdtTraversalFrame::new(
                        next_index,
                        next_query_index,
                        effective_term_weight,
                        binding_pos,
                        self.variable_child_heads[next_index],
                    ));
                }
            }
        }
    }

    fn collect_matching_occurrences(
        &self,
        node_index: usize,
        query_index: usize,
        state: &PdtSearchState,
        effective_term_weight: i64,
        bindings: &mut PdtQueryBindings,
        occurrences: &mut Vec<PdtIndexedOccurrence>,
    ) {
        if !self.node_satisfies_constraints(node_index, effective_term_weight, state.term_date) {
            return;
        }

        if query_index == state.query.len() {
            for occurrence in self.nodes[node_index]
                .terminal_entries
                .iter()
                // C traverses a pointer-keyed PTree here. Its allocator reuse
                // makes newer same-leaf ClausePos entries precede older ones.
                .rev()
                .filter_map(|entry| entry.occurrence)
            {
                if !occurrences.contains(&occurrence) {
                    occurrences.push(occurrence);
                }
            }
            return;
        }

        for step in [state.traversal_order.first, state.traversal_order.second] {
            match step {
                PdtTraversalStep::Symbols => self.collect_symbol_matching_occurrences(
                    node_index,
                    query_index,
                    state,
                    effective_term_weight,
                    bindings,
                    occurrences,
                ),
                PdtTraversalStep::Variables => self.collect_variable_matching_occurrences(
                    node_index,
                    query_index,
                    state,
                    effective_term_weight,
                    bindings,
                    occurrences,
                ),
            }
        }
    }

    fn collect_symbol_matching_occurrences(
        &self,
        node_index: usize,
        query_index: usize,
        state: &PdtSearchState,
        effective_term_weight: i64,
        bindings: &mut PdtQueryBindings,
        occurrences: &mut Vec<PdtIndexedOccurrence>,
    ) {
        let token = state.query[query_index].token();
        if !matches!(token, PrefixToken::FreeVar { .. }) {
            if let Some(next_index) = self.nodes[node_index].children.get(&token).copied() {
                self.record_nodes_visited(1);
                self.collect_matching_occurrences(
                    next_index,
                    query_index + 1,
                    state,
                    effective_term_weight,
                    bindings,
                    occurrences,
                );
            }
        }
    }

    fn collect_variable_matching_occurrences(
        &self,
        node_index: usize,
        query_index: usize,
        state: &PdtSearchState,
        effective_term_weight: i64,
        bindings: &mut PdtQueryBindings,
        occurrences: &mut Vec<PdtIndexedOccurrence>,
    ) {
        let next_query_index = query_index.saturating_add(state.query[query_index].span);
        if next_query_index > state.query.len() {
            return;
        }
        let current = QuerySubtree {
            start: query_index,
            end: next_query_index,
        };
        for (variable_id, variable_type_uid, variable_weight, next_index) in self.nodes[node_index]
            .children
            .iter()
            .filter_map(|(edge, next_index)| {
                if let PrefixToken::FreeVar {
                    id,
                    type_uid,
                    weight,
                } = edge
                {
                    Some((*id, *type_uid, *weight, *next_index))
                } else {
                    None
                }
            })
        {
            if state.query[query_index].type_uid() != variable_type_uid {
                continue;
            }
            let variable_key = (variable_id, variable_type_uid, variable_weight);
            let next_effective_term_weight = adjusted_variable_edge_weight(
                effective_term_weight,
                state.query[query_index].weight(),
                variable_weight,
            );
            if let Some(bound) = bindings.get(&variable_key).copied() {
                if query_subtrees_match(state, bound, current) {
                    self.record_nodes_visited(1);
                    self.collect_matching_occurrences(
                        next_index,
                        next_query_index,
                        state,
                        next_effective_term_weight,
                        bindings,
                        occurrences,
                    );
                }
            } else {
                bindings.insert(variable_key, current);
                self.record_nodes_visited(1);
                self.collect_matching_occurrences(
                    next_index,
                    next_query_index,
                    state,
                    next_effective_term_weight,
                    bindings,
                    occurrences,
                );
                bindings.remove(&variable_key);
            }
        }
    }

    fn query_may_have_matchable_path(&self, query: &[PrefixQueryCell]) -> bool {
        self.node_may_have_matchable_path(0, 0, query)
    }

    fn node_may_have_matchable_path(
        &self,
        node_index: usize,
        query_index: usize,
        query: &[PrefixQueryCell],
    ) -> bool {
        if query_index == query.len() {
            return self.nodes[node_index].terminal_count != 0;
        }

        let token = query[query_index].token();
        if !matches!(token, PrefixToken::FreeVar { .. })
            && self.nodes[node_index]
                .children
                .get(&token)
                .is_some_and(|next_index| {
                    self.node_may_have_matchable_path(*next_index, query_index + 1, query)
                })
        {
            return true;
        }

        let next_query_index = query_index.saturating_add(query[query_index].span);
        if next_query_index > query.len() {
            return true;
        }
        self.nodes[node_index]
            .children
            .iter()
            .filter(|(edge, _)| matches!(edge, PrefixToken::FreeVar { .. }))
            .any(|(_, next_index)| {
                self.node_may_have_matchable_path(*next_index, next_query_index, query)
            })
    }
}

fn pop_subst_cursor_frame(cursor: &mut PdtSubstCursor) {
    if let Some(frame) = cursor.frames.pop() {
        cursor.bindings.truncate(frame.binding_pos);
    }
}

fn pack_variable_child_index(index: usize) -> u32 {
    let packed = u32::try_from(index).expect("PDTree variable child index exceeds packed range");
    assert_ne!(
        packed, PDT_NO_VARIABLE_CHILD,
        "PDTree variable child index collides with sentinel"
    );
    packed
}

fn unpack_variable_child_index(index: u32) -> Option<usize> {
    (index != PDT_NO_VARIABLE_CHILD).then(|| {
        usize::try_from(index).expect("packed PDTree variable child index does not fit usize")
    })
}

fn query_subtrees_match(
    state: &PdtSearchState,
    expected: QuerySubtree,
    actual: QuerySubtree,
) -> bool {
    let Some(expected) = state.query.get(expected.start..expected.end) else {
        return false;
    };
    let Some(actual) = state.query.get(actual.start..actual.end) else {
        return false;
    };
    expected.len() == actual.len()
        && expected
            .iter()
            .zip(actual)
            .all(|(left, right)| left.token() == right.token() && left.span == right.span)
}

#[cfg(feature = "pdt-count-nodes")]
pub fn record_global_nodes_visited(count: u64) {
    PDT_NODE_COUNTER.fetch_add(count, Ordering::Relaxed);
}

#[cfg(feature = "pdt-count-nodes")]
#[must_use]
pub fn pdt_node_counter() -> u64 {
    PDT_NODE_COUNTER.load(Ordering::Relaxed)
}

#[must_use]
pub fn pdt_constraint_settings() -> PdtConstraintSettings {
    PdtConstraintSettings {
        use_size_constraints: PDT_USE_SIZE_CONSTRAINTS.load(Ordering::Relaxed),
        use_age_constraints: PDT_USE_AGE_CONSTRAINTS.load(Ordering::Relaxed),
    }
}

pub fn set_pdt_constraint_settings(settings: PdtConstraintSettings) -> PdtConstraintSettings {
    let previous = pdt_constraint_settings();
    PDT_USE_SIZE_CONSTRAINTS.store(settings.use_size_constraints, Ordering::Relaxed);
    PDT_USE_AGE_CONSTRAINTS.store(settings.use_age_constraints, Ordering::Relaxed);
    previous
}

fn fun_code_key(code: FunCode) -> IntMapKey {
    IntMapKey::try_from(code)
        .unwrap_or_else(|_| panic!("function code {code} does not fit an IntMap key"))
}

/// Extracts the C `TermLRTraverseNext` key sequence used by
/// `PDTreeInsertTerm` and `PDTreeMatchPrefix`.
///
/// # Panics
///
/// Panics if a traversed non-leaf term has an uninitialized argument, matching
/// the C traversal precondition that all argument slots contain valid terms.
#[must_use]
pub fn term_lr_traverse_code(term: &Term) -> Vec<PrefixToken> {
    let mut code = Vec::new();
    let mut stack = vec![term.clone()];

    while let Some(current) = stack.pop() {
        code.push(prefix_token(&current));
        if current.is_top_level_free_var() {
            continue;
        }

        let start = usize::from(current.is_lambda() || current.is_applied_db_var());
        for index in (start..current.arity()).rev() {
            let arg = current
                .argument(index)
                .unwrap_or_else(|| panic!("term argument {index} is uninitialized"));
            stack.push(arg);
        }
    }

    code
}

pub(crate) fn pd_tree_term_needs_eta_normalization(term: &Term) -> bool {
    term.is_non_fo_pattern() || term.has_lambda_subterm()
}

pub(crate) fn normalize_pd_tree_term(bank: &mut TermBank, term: &Term) -> Result<Term, Diagnostic> {
    if term.is_non_fo_pattern() {
        lambda_eta_expand_db(bank, term)
    } else {
        lambda_eta_reduce_db(bank, term)
    }
}

fn term_lr_traverse_path(term: &Term) -> Vec<(PrefixToken, Option<Term>)> {
    let mut path = Vec::new();
    let mut stack = vec![term.clone()];

    while let Some(current) = stack.pop() {
        let token = prefix_token(&current);
        let indexed_variable =
            matches!(token, PrefixToken::FreeVar { .. }).then(|| current.clone());
        path.push((token, indexed_variable));
        if current.is_top_level_free_var() {
            continue;
        }

        let start = usize::from(current.is_lambda() || current.is_applied_db_var());
        for index in (start..current.arity()).rev() {
            let arg = current
                .argument(index)
                .unwrap_or_else(|| panic!("term argument {index} is uninitialized"));
            stack.push(arg);
        }
    }

    path
}

#[cfg(test)]
fn term_lr_traverse_query(term: &Term) -> Vec<PrefixQueryCell> {
    let mut query = Vec::new();
    push_prefix_query_cell_reference(&mut query, term.clone());
    query
}

#[cfg(test)]
fn push_prefix_query_cell_reference(query: &mut Vec<PrefixQueryCell>, term: Term) -> usize {
    let start = query.len();
    let first_arg = usize::from(term.is_lambda() || term.is_applied_db_var());
    let arity = term.arity();
    let traverses_arguments = !term.is_top_level_free_var();
    query.push(PrefixQueryCell { span: 0, term });

    if traverses_arguments {
        for index in first_arg..arity {
            let arg = query[start]
                .term
                .argument(index)
                .unwrap_or_else(|| panic!("term argument {index} is uninitialized"));
            push_prefix_query_cell_reference(query, arg);
        }
    }

    let span = query.len() - start;
    query[start].span = span;
    span
}

#[must_use]
pub fn prefix_compute_term_code(term: &Term) -> Vec<PrefixToken> {
    term_lr_traverse_code(term)
}

#[must_use]
pub fn prefix_match_counts(term: &Term, prefixes: &[Vec<PrefixToken>]) -> (usize, usize) {
    let tree = PdTree::from_codes(prefixes);
    let result = tree.match_prefix(term);
    (result.matched, result.remains)
}

#[must_use]
pub fn prefix_code_match_counts(
    term_code: &[PrefixToken],
    prefixes: &[Vec<PrefixToken>],
) -> (usize, usize) {
    let tree = PdTree::from_codes(prefixes);
    let result = tree.match_code_prefix(term_code);
    (result.matched, result.remains)
}

#[must_use]
pub fn prefix_code_ref_count(term_code: &[PrefixToken], prefixes: &[Vec<PrefixToken>]) -> usize {
    PdTree::from_codes(prefixes).prefix_ref_count(term_code)
}

fn prefix_token(term: &Term) -> PrefixToken {
    if term.is_top_level_free_var() {
        PrefixToken::FreeVar {
            id: term_identity_id(term),
            type_uid: term_type_uid(term),
            weight: term_standard_weight(term),
        }
    } else if term.is_db_var() || term.is_applied_db_var() || term.is_lambda() {
        let key = if term.is_db_var() {
            term.clone()
        } else {
            term.argument(0)
                .unwrap_or_else(|| panic!("DB/lambda term has no head argument"))
        };
        PrefixToken::DbLike(term_identity_id(&key))
    } else {
        PrefixToken::Fun(term.f_code())
    }
}

#[cfg(test)]
fn prefix_query_metadata(term: &Term) -> PrefixQueryMetadata {
    let f_code = term.f_code();
    let is_db_var = term.is_db_var();
    let is_free_var = f_code < 0;
    let is_phony_app = !is_db_var && f_code == SIG_PHONY_APP_CODE;
    let is_lambda = !is_db_var && matches!(f_code, SIG_NAMED_LAMBDA_CODE | SIG_DB_LAMBDA_CODE);
    let head = if is_lambda {
        Some(
            term.argument(0)
                .unwrap_or_else(|| panic!("DB/lambda term has no head argument")),
        )
    } else if is_phony_app {
        term.argument(0)
    } else {
        None
    };
    let is_applied_free_var = is_phony_app && head.as_ref().is_some_and(Term::is_free_var);
    let is_applied_db_var = is_phony_app && head.as_ref().is_some_and(Term::is_db_var);
    let is_top_level_free_var = is_free_var || is_applied_free_var;
    let type_uid = term_type_uid(term);
    let weight = term_standard_weight(term);
    let token = if is_top_level_free_var {
        PrefixToken::FreeVar {
            id: term_identity_id(term),
            type_uid,
            weight,
        }
    } else if is_db_var || is_applied_db_var || is_lambda {
        PrefixToken::DbLike(term_identity_id(if is_db_var {
            term
        } else {
            head.as_ref()
                .expect("applied DB variables and lambdas have head arguments")
        }))
    } else {
        PrefixToken::Fun(f_code)
    };

    let metadata = PrefixQueryMetadata {
        token,
        type_uid,
        weight,
        first_arg: usize::from(is_lambda || is_applied_db_var),
        traverses_arguments: !is_top_level_free_var,
    };
    debug_assert_eq!(metadata.token, prefix_token(term));
    debug_assert_eq!(metadata.type_uid, term_type_uid(term));
    debug_assert_eq!(metadata.weight, term_standard_weight(term));
    debug_assert_eq!(
        metadata.first_arg,
        usize::from(term.is_lambda() || term.is_applied_db_var())
    );
    debug_assert_eq!(metadata.traverses_arguments, !term.is_top_level_free_var());
    metadata
}

fn term_type_uid(term: &Term) -> TypeUniqueId {
    term.type_()
        .map_or(INVALID_TYPE_UID, |type_| type_.type_uid())
}

fn adjusted_variable_edge_weight(
    effective_term_weight: i64,
    query_subtree_weight: i64,
    variable_weight: i64,
) -> i64 {
    effective_term_weight - query_subtree_weight + variable_weight
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "pdt-count-nodes")]
    use super::pdt_node_counter;
    use super::{
        adjusted_variable_edge_weight, normalize_pd_tree_term, prefix_code_ref_count,
        prefix_compute_term_code, prefix_match_counts, prefix_query_metadata, prefix_token,
        term_lr_traverse_query, term_type_uid, unpack_variable_child_index, PdTree,
        PdtIndexedOccurrence, PdtTraversalOrder, PrefixToken, CLAUSEPOSCELL_MEM, PDTNODE_MEM,
        PDTREE_CELL_MEM, PDTREE_IGNORE_NF_DATE, PDTREE_IGNORE_TERM_WEIGHT, PDT_NO_VARIABLE_CHILD,
    };
    use crate::basics::intmap::{INTMAPCELL_MEM, INTORP_MEM, PDARRAYCELL_MEM};
    use crate::basics::objmaps::size_of_obj_map_node_estimate;
    use crate::basics::sysdate::SysDate;
    use crate::clauses::eqn_props::EqnSide;
    use crate::inout::scanner::Scanner;
    use crate::terms::lambda::{apply_terms, close_with_type_prefix};
    use crate::terms::signature::{Signature, SIG_DB_LAMBDA_CODE, SIG_PHONY_APP_CODE};
    use crate::terms::simpletypes::{alloc_arrow_type, Type, INVALID_TYPE_UID};
    use crate::terms::subst::Substitution;
    use crate::terms::termbanks::TermBank;
    use crate::terms::termfunc::term_standard_weight;
    use crate::terms::termtypes::{DerefType, Term, DEFAULT_VWEIGHT};
    use crate::terms::typebanks::TypeBank;
    use crate::test_support::global_state_lock;

    fn parse_in_bank(bank: &mut TermBank, source: &str) -> Term {
        let mut scanner = Scanner::from_user_string(source, false).unwrap();
        bank.parse_term_simple(&mut scanner).unwrap()
    }

    fn typed_var(bank: &TermBank, f_code: i64) -> Term {
        let type_ = bank.signature().type_bank().default_type();
        bank.vars().var_assert_alloc(f_code, &type_)
    }

    fn typed_const_with_type(bank: &mut TermBank, name: &str, type_: &Type) -> Term {
        let f_code = bank.signature_mut().insert_id(name, 0, false);
        if bank.signature().get_type(f_code).is_none() {
            bank.signature_mut()
                .declare_final_type(f_code, type_.clone())
                .unwrap();
        }
        bank.create_const_term(f_code).unwrap()
    }

    fn typed_const(bank: &mut TermBank, name: &str) -> Term {
        let type_ = bank.signature().type_bank().default_type();
        typed_const_with_type(bank, name, &type_)
    }

    fn typed_binary(bank: &mut TermBank, name: &str, left: &Term, right: &Term) -> Term {
        let type_ = bank.signature().type_bank().default_type();
        let f_code = bank.signature_mut().insert_id(name, 2, false);
        if bank.signature().get_type(f_code).is_none() {
            bank.signature_mut()
                .declare_final_type(
                    f_code,
                    alloc_arrow_type(vec![type_.clone(), type_.clone(), type_.clone()]),
                )
                .unwrap();
        }
        let term = Term::top_alloc(f_code, 2);
        term.set_type(Some(type_));
        term.set_argument(0, left.clone());
        term.set_argument(1, right.clone());
        bank.insert(&term, DerefType::Never).unwrap()
    }

    #[test]
    fn term_code_uses_left_right_traversal_f_codes_for_first_order_terms() {
        let mut bank = TermBank::new(Signature::new(TypeBank::new())).unwrap();
        let term = parse_in_bank(&mut bank, "f(a,g(b))");
        let code = prefix_compute_term_code(&term);

        assert_eq!(
            code,
            vec![
                PrefixToken::Fun(bank.signature().find_f_code("f")),
                PrefixToken::Fun(bank.signature().find_f_code("a")),
                PrefixToken::Fun(bank.signature().find_f_code("g")),
                PrefixToken::Fun(bank.signature().find_f_code("b")),
            ]
        );
    }

    #[test]
    fn match_counts_follow_pdtree_path_prefix_not_stored_term_prefixes_only() {
        let mut bank = TermBank::new(Signature::new(TypeBank::new())).unwrap();
        let stored = parse_in_bank(&mut bank, "f(a,b)");
        let candidate = parse_in_bank(&mut bank, "f(a)");
        let stored_codes = vec![prefix_compute_term_code(&stored)];

        assert_eq!(prefix_match_counts(&candidate, &stored_codes), (2, 0));
    }

    #[test]
    fn tree_reuses_shared_prefix_nodes_and_counts_references() {
        let mut bank = TermBank::new(Signature::new(TypeBank::new())).unwrap();
        let stored_a = parse_in_bank(&mut bank, "f(a,b)");
        let stored_b = parse_in_bank(&mut bank, "f(a,c)");
        let query = parse_in_bank(&mut bank, "f(a,d)");
        let prefix = prefix_compute_term_code(&parse_in_bank(&mut bank, "f(a)"));
        let mut tree = PdTree::new();

        assert!(tree.insert_term(&stored_a));
        assert!(tree.insert_term(&stored_b));

        let result = tree.match_prefix(&query);
        assert_eq!((result.matched, result.remains), (2, 1));
        assert_eq!(tree.term_count(), 2);
        assert_eq!(tree.prefix_ref_count(&prefix), 2);
    }

    #[test]
    fn code_ref_count_counts_inserted_terms_below_prefix() {
        let mut bank = TermBank::new(Signature::new(TypeBank::new())).unwrap();
        let first = prefix_compute_term_code(&parse_in_bank(&mut bank, "f(a,b)"));
        let second = prefix_compute_term_code(&parse_in_bank(&mut bank, "f(a,c)"));
        let prefix = prefix_compute_term_code(&parse_in_bank(&mut bank, "f(a)"));

        assert_eq!(prefix_code_ref_count(&prefix, &[first, second]), 2);
    }

    #[test]
    fn storage_estimate_is_zero_for_empty_tree_like_c_macro() {
        let tree = PdTree::new();

        assert_eq!(tree.node_count(), 0);
        assert_eq!(tree.arr_storage_estimate(), 0);
        assert_eq!(tree.storage_estimate(), 0);
    }

    #[test]
    fn search_counters_start_zero_and_record_c_search_bookkeeping() {
        let tree = PdTree::new();
        #[cfg(feature = "pdt-count-nodes")]
        let global_before = pdt_node_counter();

        assert_eq!(tree.match_count(), 0);
        assert_eq!(tree.visited_count(), 0);
        assert!(!tree.search_is_active());
        assert_eq!(tree.search_state(), None);
        assert_eq!(tree.search_term_weight(), i64::MAX);
        assert_eq!(tree.search_term_date(), SysDate::creation_time());
        assert_eq!(
            tree.search_traversal_order(),
            PdtTraversalOrder::symbols_first()
        );

        tree.record_search_attempt();
        tree.record_nodes_visited(3);
        tree.record_nodes_visited(2);

        assert_eq!(tree.match_count(), 1);
        assert_eq!(tree.visited_count(), 5);
        #[cfg(feature = "pdt-count-nodes")]
        assert!(pdt_node_counter() >= global_before.saturating_add(5));
    }

    #[test]
    fn search_init_records_c_prefer_general_traversal_order() {
        let mut bank = TermBank::new(Signature::new(TypeBank::new())).unwrap();
        let first = parse_in_bank(&mut bank, "f(a)");
        let second = parse_in_bank(&mut bank, "g(b,c)");
        let tree = PdTree::new();

        tree.record_search_init(&first, SysDate::creation_time(), false);

        assert_eq!(tree.match_count(), 1);
        assert_eq!(
            tree.search_traversal_order(),
            PdtTraversalOrder::variables_first()
        );
        assert!(tree.search_is_active());
        assert_eq!(tree.search_term_weight(), term_standard_weight(&first));
        assert_eq!(tree.search_term_date(), SysDate::creation_time());
        let state = tree.search_state().expect("search init stores state");
        assert_eq!(
            state
                .query
                .iter()
                .map(super::PrefixQueryCell::token)
                .collect::<Vec<_>>(),
            prefix_compute_term_code(&first)
        );
        assert_eq!(
            state.query.iter().map(|cell| cell.span).collect::<Vec<_>>(),
            vec![2, 1]
        );
        assert_eq!(state.term_weight, state.query[0].weight());
        assert_eq!(state.traversal_order, PdtTraversalOrder::variables_first());

        tree.record_search_exit();

        assert!(!tree.search_is_active());
        assert_eq!(tree.search_state(), None);
        assert_eq!(tree.search_matching_occurrences(), None);
        assert_eq!(tree.search_term_weight(), term_standard_weight(&first));
        assert_eq!(tree.search_term_date(), SysDate::creation_time());
        assert_eq!(
            tree.search_traversal_order(),
            PdtTraversalOrder::variables_first()
        );

        tree.record_search_init(&second, SysDate::from_raw(7), true);

        assert_eq!(tree.match_count(), 2);
        assert_eq!(
            tree.search_traversal_order(),
            PdtTraversalOrder::symbols_first()
        );
        assert!(tree.search_is_active());
        assert_eq!(tree.search_term_weight(), term_standard_weight(&second));
        assert_eq!(tree.search_term_date(), SysDate::from_raw(7));
        let state = tree
            .search_state()
            .expect("search init stores replacement state");
        assert_eq!(
            state
                .query
                .iter()
                .map(super::PrefixQueryCell::token)
                .collect::<Vec<_>>(),
            prefix_compute_term_code(&second)
        );
        assert_eq!(
            state.query.iter().map(|cell| cell.span).collect::<Vec<_>>(),
            vec![3, 1, 1]
        );
        assert_eq!(state.traversal_order, PdtTraversalOrder::symbols_first());
    }

    #[test]
    fn search_exit_recycles_query_storage_for_the_next_search() {
        let mut bank = TermBank::new(Signature::new(TypeBank::new())).unwrap();
        let large = parse_in_bank(
            &mut bank,
            "pdt_reuse_f(pdt_reuse_g(a,b),pdt_reuse_h(c,d),pdt_reuse_i(e,f))",
        );
        let small = parse_in_bank(&mut bank, "pdt_reuse_j(a)");
        let tree = PdTree::new();

        tree.record_search_init(&large, PDTREE_IGNORE_NF_DATE, false);
        let large_capacity = tree
            .search_state
            .borrow()
            .as_ref()
            .expect("search init stores query")
            .query
            .capacity();
        tree.record_search_exit();
        assert_eq!(
            tree.search_query_scratch.borrow().capacity(),
            large_capacity
        );

        tree.record_search_init(&small, PDTREE_IGNORE_NF_DATE, false);
        assert_eq!(
            tree.search_state
                .borrow()
                .as_ref()
                .expect("next search reuses query storage")
                .query
                .capacity(),
            large_capacity
        );
    }

    #[test]
    fn iterative_query_builder_matches_recursive_cells_and_spans() {
        let mut bank = TermBank::new(Signature::new(TypeBank::new())).unwrap();
        let term = parse_in_bank(&mut bank, "span_f(span_g(span_a),span_b)");
        let expected = term_lr_traverse_query(&term);
        let tree = PdTree::new();
        let query = tree.build_search_query(&term);

        assert_eq!(query, expected);
        assert!(tree.search_query_build_stack.borrow().is_empty());

        assert_eq!(
            query
                .iter()
                .map(super::PrefixQueryCell::token)
                .collect::<Vec<_>>(),
            prefix_compute_term_code(&term)
        );
        assert_eq!(
            query.iter().map(|cell| cell.span).collect::<Vec<_>>(),
            vec![4, 2, 1, 1]
        );
    }

    #[test]
    fn query_metadata_matches_independent_term_classification() {
        let mut bank = TermBank::new(Signature::new(TypeBank::new())).unwrap();
        let type_ = bank.signature().type_bank().default_type();
        let free = typed_var(&bank, -31);
        let constant = typed_const(&mut bank, "pdt_metadata_constant");
        let db = bank.request_db_var(&type_, 0);
        let applied_free = Term::top_alloc(SIG_PHONY_APP_CODE, 2);
        applied_free.set_type(Some(type_.clone()));
        applied_free.set_argument(0, free.clone());
        applied_free.set_argument(1, constant.clone());
        let applied_db = Term::top_alloc(SIG_PHONY_APP_CODE, 2);
        applied_db.set_type(Some(type_.clone()));
        applied_db.set_argument(0, db.clone());
        applied_db.set_argument(1, constant.clone());
        let lambda = Term::top_alloc(SIG_DB_LAMBDA_CODE, 2);
        lambda.set_type(Some(type_));
        lambda.set_argument(0, db.clone());
        lambda.set_argument(1, constant.clone());
        let malformed_phony = Term::top_alloc(SIG_PHONY_APP_CODE, 0);

        for term in [
            constant,
            free,
            db,
            applied_free,
            applied_db,
            lambda,
            malformed_phony,
        ] {
            let metadata = prefix_query_metadata(&term);
            assert_eq!(metadata.token, prefix_token(&term));
            assert_eq!(metadata.type_uid, term_type_uid(&term));
            assert_eq!(metadata.weight, term_standard_weight(&term));
            assert_eq!(
                metadata.first_arg,
                usize::from(term.is_lambda() || term.is_applied_db_var())
            );
            assert_eq!(metadata.traverses_arguments, !term.is_top_level_free_var());
        }
    }

    #[test]
    fn matchable_path_variable_edge_consumes_whole_query_subtree() {
        let mut bank = TermBank::new(Signature::new(TypeBank::new())).unwrap();
        let query = parse_in_bank(&mut bank, "pdt_path_f(pdt_path_g(pdt_path_b),pdt_path_a)");
        let f_code = bank.signature().find_f_code("pdt_path_f");
        let a_code = bank.signature().find_f_code("pdt_path_a");
        let mut tree = PdTree::new();

        assert!(tree.insert_code(&[
            PrefixToken::Fun(f_code),
            PrefixToken::FreeVar {
                id: 17,
                type_uid: INVALID_TYPE_UID,
                weight: DEFAULT_VWEIGHT,
            },
            PrefixToken::Fun(a_code),
        ]));
        tree.record_search_init(&query, PDTREE_IGNORE_NF_DATE, false);

        assert!(tree.search_root_may_have_matchable_path());
    }

    #[test]
    fn matchable_path_rejects_missing_symbol_branch() {
        let mut bank = TermBank::new(Signature::new(TypeBank::new())).unwrap();
        let stored = parse_in_bank(&mut bank, "pdt_path_stored_f(pdt_path_stored_a)");
        let query = parse_in_bank(&mut bank, "pdt_path_query_g(pdt_path_stored_a)");
        let mut tree = PdTree::new();

        assert!(tree.insert_term(&stored));
        tree.record_search_init(&query, PDTREE_IGNORE_NF_DATE, false);

        assert!(!tree.search_root_may_have_matchable_path());
    }

    #[test]
    fn matchable_path_does_not_treat_query_variable_as_symbol_match() {
        let mut bank = TermBank::new(Signature::new(TypeBank::new())).unwrap();
        let stored = parse_in_bank(&mut bank, "pdt_path_const_a");
        let query = typed_var(&bank, -10);
        let mut tree = PdTree::new();

        assert!(tree.insert_term(&stored));
        tree.record_search_init(&query, PDTREE_IGNORE_NF_DATE, false);

        assert!(!tree.search_root_may_have_matchable_path());
    }

    #[test]
    fn matching_occurrences_follow_terminal_side_payloads_and_exact_deletion() {
        let mut bank = TermBank::new(Signature::new(TypeBank::new())).unwrap();
        let term = parse_in_bank(&mut bank, "pdt_occurrence_f(pdt_occurrence_a)");
        let code = prefix_compute_term_code(&term);
        let left = PdtIndexedOccurrence::new(10, EqnSide::LeftSide);
        let right = PdtIndexedOccurrence::new(10, EqnSide::RightSide);
        let mut tree = PdTree::new();

        assert!(tree.insert_term_occurrence(&term, SysDate::from_raw(7), left));
        assert!(tree.insert_term_occurrence(&term, SysDate::from_raw(7), right));
        tree.record_search_init(&term, PDTREE_IGNORE_NF_DATE, false);

        assert_eq!(tree.search_matching_occurrences(), Some(vec![right, left]));

        assert!(tree.delete_term_occurrence(&term, SysDate::from_raw(7), left));
        tree.record_search_exit();
        tree.record_search_init(&term, PDTREE_IGNORE_NF_DATE, false);

        assert_eq!(tree.term_count(), 1);
        assert_eq!(tree.prefix_ref_count(&code), 1);
        assert_eq!(tree.search_matching_occurrences(), Some(vec![right]));
    }

    #[test]
    fn matching_occurrences_follow_recorded_traversal_order() {
        let mut bank = TermBank::new(Signature::new(TypeBank::new())).unwrap();
        let query = parse_in_bank(&mut bank, "pdt_order_f(pdt_order_a)");
        let variable = typed_var(&bank, -20);
        let specific = PdtIndexedOccurrence::new(20, EqnSide::LeftSide);
        let general = PdtIndexedOccurrence::new(30, EqnSide::LeftSide);
        let mut tree = PdTree::new();

        assert!(tree.insert_term_occurrence(&query, SysDate::from_raw(7), specific));
        assert!(tree.insert_term_occurrence(&variable, SysDate::from_raw(7), general));

        tree.record_search_init(&query, PDTREE_IGNORE_NF_DATE, false);
        assert_eq!(
            tree.search_matching_occurrences(),
            Some(vec![general, specific])
        );

        tree.record_search_exit();
        tree.record_search_init(&query, PDTREE_IGNORE_NF_DATE, true);
        assert_eq!(
            tree.search_matching_occurrences(),
            Some(vec![specific, general])
        );
    }

    #[test]
    fn matching_occurrence_cursor_yields_candidates_and_clears_on_exit() {
        let mut bank = TermBank::new(Signature::new(TypeBank::new())).unwrap();
        let query = parse_in_bank(&mut bank, "pdt_cursor_f(pdt_cursor_a)");
        let variable = typed_var(&bank, -27);
        let specific = PdtIndexedOccurrence::new(100, EqnSide::LeftSide);
        let general = PdtIndexedOccurrence::new(101, EqnSide::LeftSide);
        let mut tree = PdTree::new();

        assert!(tree.insert_term_occurrence(&query, SysDate::from_raw(7), specific));
        assert!(tree.insert_term_occurrence(&variable, SysDate::from_raw(7), general));
        tree.record_search_init(&query, PDTREE_IGNORE_NF_DATE, false);

        assert_eq!(tree.search_next_matching_occurrence(), Some(general));
        assert_eq!(tree.search_next_matching_occurrence(), Some(specific));
        assert_eq!(tree.search_next_matching_occurrence(), None);

        tree.record_search_exit();

        assert_eq!(tree.search_next_matching_occurrence(), None);
    }

    #[test]
    fn substitution_cursor_preserves_order_and_live_binding() {
        let mut bank = TermBank::new(Signature::new(TypeBank::new())).unwrap();
        let query = parse_in_bank(&mut bank, "pdt_subst_cursor_f(pdt_subst_cursor_a)");
        let variable = typed_var(&bank, -28);
        let specific = PdtIndexedOccurrence::new(110, EqnSide::LeftSide);
        let general = PdtIndexedOccurrence::new(111, EqnSide::LeftSide);
        let mut tree = PdTree::new();
        let mut subst = Substitution::new();

        assert!(tree.insert_term_occurrence(&query, SysDate::from_raw(7), specific));
        assert!(tree.insert_term_occurrence(&variable, SysDate::from_raw(7), general));
        tree.record_search_init(&query, PDTREE_IGNORE_NF_DATE, false);

        assert_eq!(
            tree.search_next_matching_occurrence_with_subst(&mut subst),
            Some(general)
        );
        assert_eq!(subst.len(), 1);
        assert_eq!(variable.binding(), Some(query.clone()));
        assert_eq!(
            tree.search_next_matching_occurrence_with_subst(&mut subst),
            Some(specific)
        );
        assert!(subst.is_empty());
        assert_eq!(
            tree.search_next_matching_occurrence_with_subst(&mut subst),
            None
        );
        assert!(subst.is_empty());
    }

    #[test]
    fn substitution_cursor_rejects_inconsistent_repeated_variable_and_backtracks() {
        let mut bank = TermBank::new(Signature::new(TypeBank::new())).unwrap();
        let a = typed_const(&mut bank, "pdt_subst_repeat_a");
        let b = typed_const(&mut bank, "pdt_subst_repeat_b");
        let variable = typed_var(&bank, -29);
        let pattern = typed_binary(&mut bank, "pdt_subst_repeat_f", &variable, &variable);
        let different = typed_binary(&mut bank, "pdt_subst_repeat_f", &a, &b);
        let same = typed_binary(&mut bank, "pdt_subst_repeat_f", &a, &a);
        let occurrence = PdtIndexedOccurrence::new(112, EqnSide::LeftSide);
        let mut tree = PdTree::new();
        let mut subst = Substitution::new();

        assert!(tree.insert_term_occurrence(&pattern, SysDate::from_raw(7), occurrence));
        tree.record_search_init(&different, PDTREE_IGNORE_NF_DATE, false);
        assert_eq!(
            tree.search_next_matching_occurrence_with_subst(&mut subst),
            None
        );
        assert!(subst.is_empty());
        assert_eq!(variable.binding(), None);

        tree.record_search_exit();
        tree.record_search_init(&same, PDTREE_IGNORE_NF_DATE, false);
        assert_eq!(
            tree.search_next_matching_occurrence_with_subst(&mut subst),
            Some(occurrence)
        );
        assert_eq!(subst.len(), 1);
        assert_eq!(variable.binding(), Some(a));
        tree.record_search_exit();
        assert_eq!(subst.len(), 1);
        subst.backtrack();
    }

    #[test]
    fn substitution_cursor_updates_variable_edge_arena_after_deletion() {
        let mut bank = TermBank::new(Signature::new(TypeBank::new())).unwrap();
        let argument = typed_const(&mut bank, "pdt_subst_delete_a");
        let fixed = typed_const(&mut bank, "pdt_subst_delete_fixed");
        let first_variable = typed_var(&bank, -30);
        let second_variable = typed_var(&bank, -31);
        let query = typed_binary(&mut bank, "pdt_subst_delete_f", &argument, &fixed);
        let first_pattern = typed_binary(&mut bank, "pdt_subst_delete_f", &first_variable, &fixed);
        let second_pattern =
            typed_binary(&mut bank, "pdt_subst_delete_f", &second_variable, &fixed);
        let first = PdtIndexedOccurrence::new(113, EqnSide::LeftSide);
        let second = PdtIndexedOccurrence::new(114, EqnSide::LeftSide);
        let date = SysDate::from_raw(7);
        let mut tree = PdTree::new();
        let mut subst = Substitution::new();

        assert!(tree.insert_term_occurrence(&second_pattern, date, second));
        assert!(tree.insert_term_occurrence(&first_pattern, date, first));
        tree.record_search_init(&query, PDTREE_IGNORE_NF_DATE, false);
        let expected = tree.search_matching_occurrences().unwrap();
        tree.record_search_exit();

        tree.record_search_init(&query, PDTREE_IGNORE_NF_DATE, false);
        let mut actual = Vec::new();
        while let Some(occurrence) = tree.search_next_matching_occurrence_with_subst(&mut subst) {
            actual.push(occurrence);
        }
        assert_eq!(actual, expected);
        assert!(subst.is_empty());
        tree.record_search_exit();

        assert!(tree.delete_term_occurrence(&first_pattern, date, first));
        tree.record_search_init(&query, PDTREE_IGNORE_NF_DATE, false);
        assert_eq!(
            tree.search_next_matching_occurrence_with_subst(&mut subst),
            Some(second)
        );
        assert_eq!(second_variable.binding(), Some(query.argument(0).unwrap()));
        assert_eq!(
            tree.search_next_matching_occurrence_with_subst(&mut subst),
            None
        );
        assert!(subst.is_empty());
        assert_eq!(first_variable.binding(), None);

        tree.record_search_exit();
        assert!(tree.insert_term_occurrence(&first_pattern, date, first));
        tree.record_search_init(&query, PDTREE_IGNORE_NF_DATE, false);
        let mut after_reinsertion = Vec::new();
        while let Some(occurrence) = tree.search_next_matching_occurrence_with_subst(&mut subst) {
            after_reinsertion.push(occurrence);
        }
        assert_eq!(after_reinsertion, expected);
        assert!(subst.is_empty());
        assert_eq!(first_variable.binding(), None);
        assert_eq!(second_variable.binding(), None);
    }

    #[test]
    fn variable_child_reuse_refreshes_cached_type_and_weight() {
        let mut bank = TermBank::new(Signature::new(TypeBank::new())).unwrap();
        let individual = bank.signature().type_bank().default_type();
        let bool_type = bank.signature().type_bank().bool_type();
        let individual_variable = bank.vars().var_assert_alloc(-40, &individual);
        let bool_variable = bank.vars().var_assert_alloc(-42, &bool_type);
        let bool_argument = typed_const_with_type(&mut bank, "pdt_cache_bool", &bool_type);
        let applied_variable = Term::top_alloc(SIG_PHONY_APP_CODE, 2);
        applied_variable.set_type(Some(bool_type));
        applied_variable.set_argument(0, bool_variable);
        applied_variable.set_argument(1, bool_argument);
        let first_occurrence = PdtIndexedOccurrence::new(120, EqnSide::LeftSide);
        let second_occurrence = PdtIndexedOccurrence::new(121, EqnSide::LeftSide);
        let date = SysDate::from_raw(7);
        let mut tree = PdTree::new();

        assert!(tree.insert_term_occurrence(&individual_variable, date, first_occurrence,));
        let first_link = tree.variable_child_heads[0];
        let first_index = unpack_variable_child_index(first_link).unwrap();
        assert_eq!(
            tree.variable_children[first_index].type_uid,
            term_type_uid(&individual_variable)
        );
        assert_eq!(
            tree.variable_children[first_index].weight,
            term_standard_weight(&individual_variable)
        );

        assert!(tree.delete_term_occurrence(&individual_variable, date, first_occurrence));
        assert_eq!(tree.variable_child_heads[0], PDT_NO_VARIABLE_CHILD);
        assert_eq!(tree.free_variable_child, first_link);

        assert!(tree.insert_term_occurrence(&applied_variable, date, second_occurrence,));
        assert_eq!(tree.variable_child_heads[0], first_link);
        let reused = &tree.variable_children[first_index];
        assert_eq!(reused.type_uid, term_type_uid(&applied_variable));
        assert_eq!(reused.weight, term_standard_weight(&applied_variable));
        assert_ne!(reused.type_uid, term_type_uid(&individual_variable));
        assert_ne!(reused.weight, term_standard_weight(&individual_variable));
    }

    #[test]
    fn bank_aware_first_order_insertion_does_not_cache_unchanged_path() {
        let mut bank = TermBank::new(Signature::new(TypeBank::new())).unwrap();
        let term = parse_in_bank(&mut bank, "pdt_eta_fo(pdt_eta_fo_arg)");
        let occurrence = PdtIndexedOccurrence::new(121, EqnSide::LeftSide);
        let date = SysDate::from_raw(8);
        let mut tree = PdTree::new();

        assert!(tree
            .insert_term_occurrence_with_bank(&mut bank, &term, date, occurrence)
            .unwrap());
        assert!(tree.normalized_occurrence_paths.is_empty());
        tree.record_search_init_with_bank(&mut bank, &term, PDTREE_IGNORE_NF_DATE, false)
            .unwrap();
        assert_eq!(tree.search_next_matching_occurrence(), Some(occurrence));
        tree.record_search_exit();

        assert!(tree.delete_term_occurrence(&term, date, occurrence));
        assert_eq!(tree.term_count(), 0);
    }

    #[test]
    fn bank_aware_eta_normalization_matches_and_deletes_original_term() {
        let mut bank = TermBank::new(Signature::new(TypeBank::new())).unwrap();
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
                individual.clone(),
            ]));
        let variable = bank.vars().var_assert_alloc(-60, &binary);
        let argument = typed_const(&mut bank, "pdt_eta_argument");
        let non_pattern = apply_terms(&mut bank, &variable, &[argument]).unwrap();
        let head = typed_const_with_type(&mut bank, "pdt_eta_head", &unary);
        let db0 = bank.request_db_var(&individual, 0);
        let matrix = apply_terms(&mut bank, &head, &[db0]).unwrap();
        let eta_reducible =
            close_with_type_prefix(&mut bank, std::slice::from_ref(&individual), &matrix).unwrap();
        let wrapper_code = bank.signature_mut().insert_id("pdt_eta_wrapper", 2, false);
        bank.signature_mut()
            .declare_final_type(
                wrapper_code,
                alloc_arrow_type(vec![unary.clone(), unary, individual.clone()]),
            )
            .unwrap();
        let wrapper = Term::top_alloc(wrapper_code, 2);
        wrapper.set_type(Some(individual));
        wrapper.set_argument(0, non_pattern);
        wrapper.set_argument(1, eta_reducible);
        let original = bank.insert(&wrapper, DerefType::Never).unwrap();
        assert!(!original.is_non_fo_pattern());
        let normalized = normalize_pd_tree_term(&mut bank, &original).unwrap();
        assert_ne!(normalized, original);

        let occurrence = PdtIndexedOccurrence::new(122, EqnSide::LeftSide);
        let date = SysDate::from_raw(9);
        let mut tree = PdTree::new();
        assert!(tree
            .insert_term_occurrence_with_bank(&mut bank, &original, date, occurrence)
            .unwrap());
        assert_ne!(tree.match_prefix(&original).remains, 0);
        assert_eq!(tree.match_prefix(&normalized).remains, 0);

        tree.record_search_init_with_bank(&mut bank, &original, PDTREE_IGNORE_NF_DATE, false)
            .unwrap();
        assert_eq!(tree.search_next_matching_occurrence(), Some(occurrence));
        tree.record_search_exit();

        assert!(tree.delete_term_occurrence(&original, date, occurrence));
        assert_eq!(tree.term_count(), 0);
        assert!(tree.normalized_occurrence_paths.is_empty());
    }

    #[test]
    fn bank_aware_eta_expands_non_fo_pattern_before_indexing() {
        let mut bank = TermBank::new(Signature::new(TypeBank::new())).unwrap();
        let individual = bank.signature().type_bank().default_type();
        let unary = bank
            .signature_mut()
            .type_bank_mut()
            .insert_type_shared(alloc_arrow_type(vec![
                individual.clone(),
                individual.clone(),
            ]));
        let head = typed_const_with_type(&mut bank, "pdt_eta_expand_head", &unary);
        let collector_code = bank
            .signature_mut()
            .insert_id("pdt_eta_expand_collector", 2, false);
        bank.signature_mut()
            .declare_final_type(
                collector_code,
                alloc_arrow_type(vec![individual.clone(), unary, individual.clone()]),
            )
            .unwrap();
        let db0 = bank.request_db_var(&individual, 0);
        let matrix = Term::top_alloc(collector_code, 2);
        matrix.set_type(Some(individual.clone()));
        matrix.set_argument(0, db0);
        matrix.set_argument(1, head);
        let matrix = bank.insert(&matrix, DerefType::Never).unwrap();
        let original =
            close_with_type_prefix(&mut bank, std::slice::from_ref(&individual), &matrix).unwrap();
        assert!(original.is_non_fo_pattern());
        let normalized = normalize_pd_tree_term(&mut bank, &original).unwrap();
        assert_ne!(normalized, original);

        let occurrence = PdtIndexedOccurrence::new(123, EqnSide::RightSide);
        let date = SysDate::from_raw(10);
        let mut tree = PdTree::new();
        assert!(tree
            .insert_term_occurrence_with_bank(&mut bank, &original, date, occurrence)
            .unwrap());
        assert_ne!(tree.match_prefix(&original).remains, 0);
        assert_eq!(tree.match_prefix(&normalized).remains, 0);

        tree.record_search_init_with_bank(&mut bank, &original, PDTREE_IGNORE_NF_DATE, false)
            .unwrap();
        assert_eq!(tree.search_next_matching_occurrence(), Some(occurrence));
        tree.record_search_exit();

        assert!(tree.delete_term_occurrence(&original, date, occurrence));
        assert_eq!(tree.term_count(), 0);
        assert!(tree.normalized_occurrence_paths.is_empty());
    }

    #[test]
    fn matching_occurrences_records_successful_child_visits() {
        let mut bank = TermBank::new(Signature::new(TypeBank::new())).unwrap();
        let query = parse_in_bank(&mut bank, "pdt_visit_f(pdt_visit_a)");
        let variable = typed_var(&bank, -26);
        let specific = PdtIndexedOccurrence::new(90, EqnSide::LeftSide);
        let general = PdtIndexedOccurrence::new(91, EqnSide::LeftSide);
        let mut tree = PdTree::new();

        assert!(tree.insert_term_occurrence(&query, SysDate::from_raw(7), specific));
        assert!(tree.insert_term_occurrence(&variable, SysDate::from_raw(7), general));
        tree.record_search_init(&query, PDTREE_IGNORE_NF_DATE, false);

        let visited_before = tree.visited_count();
        assert_eq!(
            tree.search_matching_occurrences(),
            Some(vec![general, specific])
        );
        assert_eq!(tree.visited_count() - visited_before, 3);
    }

    #[test]
    fn matching_occurrences_prune_branches_by_node_age_constraints() {
        let mut bank = TermBank::new(Signature::new(TypeBank::new())).unwrap();
        let query = parse_in_bank(&mut bank, "pdt_age_f(pdt_age_a)");
        let variable = typed_var(&bank, -21);
        let old_specific = PdtIndexedOccurrence::new(40, EqnSide::LeftSide);
        let new_general = PdtIndexedOccurrence::new(50, EqnSide::LeftSide);
        let mut tree = PdTree::new();

        assert!(tree.insert_term_occurrence(&query, SysDate::from_raw(3), old_specific));
        assert!(tree.insert_term_occurrence(&variable, SysDate::from_raw(7), new_general));

        tree.record_search_init(&query, SysDate::from_raw(5), false);
        assert_eq!(tree.search_matching_occurrences(), Some(vec![new_general]));

        tree.record_search_exit();
        tree.record_search_init(&query, SysDate::from_raw(2), false);
        assert_eq!(
            tree.search_matching_occurrences(),
            Some(vec![new_general, old_specific])
        );
    }

    #[test]
    fn matching_occurrences_reject_repeated_variable_on_different_query_subtrees() {
        let mut bank = TermBank::new(Signature::new(TypeBank::new())).unwrap();
        let a = typed_const(&mut bank, "pdt_repeat_a");
        let b = typed_const(&mut bank, "pdt_repeat_b");
        let variable = typed_var(&bank, -22);
        let repeated_pattern = typed_binary(&mut bank, "pdt_repeat_f", &variable, &variable);
        let different_query = typed_binary(&mut bank, "pdt_repeat_f", &a, &b);
        let same_query = typed_binary(&mut bank, "pdt_repeat_f", &a, &a);
        let repeated = PdtIndexedOccurrence::new(60, EqnSide::LeftSide);
        let mut tree = PdTree::new();

        assert!(tree.insert_term_occurrence(&repeated_pattern, SysDate::from_raw(7), repeated));

        tree.record_search_init(&different_query, PDTREE_IGNORE_NF_DATE, false);
        assert_eq!(tree.search_matching_occurrences(), Some(Vec::new()));

        tree.record_search_exit();
        tree.record_search_init(&same_query, PDTREE_IGNORE_NF_DATE, false);
        assert_eq!(tree.search_matching_occurrences(), Some(vec![repeated]));
    }

    #[test]
    fn matching_occurrences_reject_variable_edge_with_mismatched_query_type() {
        let mut bank = TermBank::new(Signature::new(TypeBank::new())).unwrap();
        let individual = bank.signature().type_bank().default_type();
        let bool_type = bank.signature().type_bank().bool_type();
        let variable = bank.vars().var_assert_alloc(-24, &individual);
        let bool_const = typed_const_with_type(&mut bank, "pdt_type_bool", &bool_type);
        let individual_const = typed_const(&mut bank, "pdt_type_ind");
        let occurrence = PdtIndexedOccurrence::new(70, EqnSide::LeftSide);
        let mut tree = PdTree::new();

        assert!(tree.insert_term_occurrence(&variable, SysDate::from_raw(7), occurrence));

        tree.record_search_init(&bool_const, PDTREE_IGNORE_NF_DATE, false);
        assert_eq!(tree.search_matching_occurrences(), Some(Vec::new()));

        tree.record_search_exit();
        tree.record_search_init(&individual_const, PDTREE_IGNORE_NF_DATE, false);
        assert_eq!(tree.search_matching_occurrences(), Some(vec![occurrence]));
    }

    #[test]
    fn variable_edge_weight_adjustment_matches_c_descendant_size_pruning() {
        let mut bank = TermBank::new(Signature::new(TypeBank::new())).unwrap();
        let a = typed_const(&mut bank, "pdt_weight_a");
        let b = typed_const(&mut bank, "pdt_weight_b");
        let variable = typed_var(&bank, -25);
        let heavy_tail = typed_binary(&mut bank, "pdt_weight_tail", &a, &b);
        let stored = typed_binary(&mut bank, "pdt_weight_f", &variable, &heavy_tail);
        let query_head = typed_binary(&mut bank, "pdt_weight_big", &a, &b);
        let query_tail = typed_const(&mut bank, "pdt_weight_small_tail");
        let query = typed_binary(&mut bank, "pdt_weight_f", &query_head, &query_tail);
        let mut tree = PdTree::new();

        assert!(tree.insert_term_occurrence(
            &stored,
            SysDate::from_raw(7),
            PdtIndexedOccurrence::new(80, EqnSide::LeftSide),
        ));
        tree.record_search_init(&query, PDTREE_IGNORE_NF_DATE, false);
        let state = tree.search_state().unwrap();
        let root_child_index = tree.nodes[0]
            .children
            .get(&state.query[0].token())
            .copied()
            .unwrap();
        let (variable_weight, variable_child_index) = tree.nodes[root_child_index]
            .children
            .iter()
            .find_map(|(edge, next_index)| {
                if let PrefixToken::FreeVar { weight, .. } = edge {
                    Some((*weight, *next_index))
                } else {
                    None
                }
            })
            .unwrap();
        let adjusted_weight = adjusted_variable_edge_weight(
            state.term_weight,
            state.query[1].weight(),
            variable_weight,
        );

        assert_eq!(state.term_weight, term_standard_weight(&query));
        assert_eq!(state.query[1].weight(), term_standard_weight(&query_head));
        assert_eq!(variable_weight, term_standard_weight(&variable));
        assert!(tree.node_satisfies_constraints(
            variable_child_index,
            state.term_weight,
            PDTREE_IGNORE_NF_DATE,
        ));
        assert!(!tree.node_satisfies_constraints(
            variable_child_index,
            adjusted_weight,
            PDTREE_IGNORE_NF_DATE,
        ));
        assert_eq!(tree.search_matching_occurrences(), Some(Vec::new()));
    }

    #[test]
    fn constraints_track_minimum_weight_and_youngest_clause_date() {
        let _guard = global_state_lock();
        let mut bank = TermBank::new(Signature::new(TypeBank::new())).unwrap();
        let light = parse_in_bank(&mut bank, "constraint_light");
        let heavy = parse_in_bank(&mut bank, "constraint_heavy(a,b)");
        let mut tree = PdTree::new();

        assert!(tree.insert_term_with_clause_date(&heavy, SysDate::from_raw(8)));
        assert!(tree.insert_term_with_clause_date(&light, SysDate::from_raw(3)));

        assert_eq!(tree.size_constraint(), term_standard_weight(&light));
        assert_eq!(tree.age_constraint(), SysDate::from_raw(8));
        assert!(tree.root_satisfies_constraints(term_standard_weight(&heavy), SysDate::from_raw(7)));
        assert!(
            !tree.root_satisfies_constraints(term_standard_weight(&heavy), SysDate::from_raw(8))
        );
        assert!(
            tree.root_satisfies_constraints(term_standard_weight(&light), PDTREE_IGNORE_NF_DATE)
        );
        assert!(!tree
            .root_satisfies_constraints(term_standard_weight(&light) - 1, PDTREE_IGNORE_NF_DATE));
    }

    #[test]
    fn delete_recomputes_extremal_constraints_for_remaining_entries() {
        let mut bank = TermBank::new(Signature::new(TypeBank::new())).unwrap();
        let light = parse_in_bank(&mut bank, "delete_constraint_light");
        let heavy = parse_in_bank(&mut bank, "delete_constraint_heavy(a,b)");
        let mut tree = PdTree::new();

        assert!(tree.insert_term_with_clause_date(&heavy, SysDate::from_raw(8)));
        assert!(tree.insert_term_with_clause_date(&light, SysDate::from_raw(3)));

        assert!(tree.delete_term_with_clause_date(&heavy, SysDate::from_raw(8)));
        assert_eq!(tree.size_constraint(), term_standard_weight(&light));
        assert_eq!(tree.age_constraint(), SysDate::from_raw(3));

        assert!(tree.delete_term_with_clause_date(&light, SysDate::from_raw(3)));
        assert_eq!(tree.size_constraint(), PDTREE_IGNORE_TERM_WEIGHT);
        assert_eq!(tree.age_constraint(), SysDate::creation_time());
    }

    #[test]
    fn storage_estimate_counts_nodes_function_arrays_and_clause_positions() {
        let mut tree = PdTree::new();

        assert!(tree.insert_code(&[PrefixToken::Fun(1), PrefixToken::Fun(2)]));

        assert_eq!(tree.node_count(), 2);
        assert_eq!(tree.term_count(), 1);
        assert_eq!(tree.arr_storage_estimate(), 2 * INTMAPCELL_MEM);
        assert_eq!(
            tree.storage_estimate(),
            2 * PDTNODE_MEM + 2 * INTMAPCELL_MEM + PDTREE_CELL_MEM + CLAUSEPOSCELL_MEM
        );
    }

    #[test]
    fn storage_estimate_counts_variable_and_db_objmap_nodes() {
        let mut tree = PdTree::new();

        assert!(tree.insert_code(&[
            PrefixToken::FreeVar {
                id: 7,
                type_uid: INVALID_TYPE_UID,
                weight: DEFAULT_VWEIGHT,
            },
            PrefixToken::DbLike(3),
        ]));

        assert_eq!(tree.node_count(), 2);
        assert_eq!(
            tree.arr_storage_estimate(),
            2 * INTMAPCELL_MEM + 2 * size_of_obj_map_node_estimate()
        );
    }

    #[test]
    fn function_delete_preserves_c_parent_alt_storage_estimate_quirk() {
        let mut tree = PdTree::new();
        let first = [PrefixToken::Fun(1)];
        let second = [PrefixToken::Fun(100)];

        assert!(tree.insert_code(&first));
        assert!(tree.insert_code(&second));
        let root_array_storage = INTMAPCELL_MEM + PDARRAYCELL_MEM + INTORP_MEM + 104 * INTORP_MEM;
        let root_array_delta = root_array_storage - INTMAPCELL_MEM;
        assert_eq!(
            tree.arr_storage_estimate(),
            root_array_delta + 2 * INTMAPCELL_MEM
        );

        assert!(tree.delete_code(&first));

        assert_eq!(tree.node_count(), 1);
        assert_eq!(tree.term_count(), 1);
        assert_eq!(
            tree.arr_storage_estimate(),
            root_array_delta + INTMAPCELL_MEM
        );
        assert_eq!(
            tree.storage_estimate(),
            PDTNODE_MEM + root_array_delta + INTMAPCELL_MEM + PDTREE_CELL_MEM + CLAUSEPOSCELL_MEM
        );
    }

    #[test]
    fn delete_term_decrements_shared_prefix_counts_and_prunes_dead_suffix() {
        let mut bank = TermBank::new(Signature::new(TypeBank::new())).unwrap();
        let first = parse_in_bank(&mut bank, "f(a,b)");
        let second = parse_in_bank(&mut bank, "f(a,c)");
        let shared_prefix = prefix_compute_term_code(&parse_in_bank(&mut bank, "f(a)"));
        let first_code = prefix_compute_term_code(&first);
        let mut tree = PdTree::new();

        assert!(tree.insert_term(&first));
        assert!(tree.insert_term(&second));
        assert_eq!(tree.node_count(), 4);

        assert!(tree.delete_term(&first));

        assert_eq!(tree.term_count(), 1);
        assert_eq!(tree.prefix_ref_count(&shared_prefix), 1);
        assert_eq!(tree.prefix_ref_count(&first_code), 0);
        assert_eq!(tree.node_count(), 3);
        assert_eq!(tree.match_prefix(&second).remains, 0);
    }

    #[test]
    fn delete_code_removes_one_duplicate_occurrence_at_a_time() {
        let mut bank = TermBank::new(Signature::new(TypeBank::new())).unwrap();
        let term = parse_in_bank(&mut bank, "f(a)");
        let code = prefix_compute_term_code(&term);
        let mut tree = PdTree::new();

        assert!(tree.insert_code(&code));
        assert!(tree.insert_code(&code));

        assert!(tree.delete_code(&code));
        assert_eq!(tree.term_count(), 1);
        assert_eq!(tree.prefix_ref_count(&code), 1);
        assert_eq!(tree.node_count(), code.len());

        assert!(tree.delete_code(&code));
        assert_eq!(tree.term_count(), 0);
        assert_eq!(tree.prefix_ref_count(&code), 0);
        assert_eq!(tree.node_count(), 0);
        assert!(!tree.delete_code(&code));
    }

    #[test]
    fn delete_missing_code_leaves_tree_counts_unchanged() {
        let mut bank = TermBank::new(Signature::new(TypeBank::new())).unwrap();
        let stored = parse_in_bank(&mut bank, "f(a)");
        let missing = prefix_compute_term_code(&parse_in_bank(&mut bank, "g(a)"));
        let stored_code = prefix_compute_term_code(&stored);
        let mut tree = PdTree::new();

        assert!(tree.insert_term(&stored));
        assert!(!tree.delete_code(&missing));

        assert_eq!(tree.term_count(), 1);
        assert_eq!(tree.prefix_ref_count(&stored_code), 1);
        assert_eq!(tree.node_count(), stored_code.len());
    }

    #[test]
    fn delete_code_occurrences_removes_all_matching_duplicates() {
        let mut bank = TermBank::new(Signature::new(TypeBank::new())).unwrap();
        let code = prefix_compute_term_code(&parse_in_bank(&mut bank, "f(a)"));
        let other = prefix_compute_term_code(&parse_in_bank(&mut bank, "f(b)"));
        let mut tree = PdTree::new();

        assert!(tree.insert_code(&code));
        assert!(tree.insert_code(&code));
        assert!(tree.insert_code(&other));

        assert_eq!(tree.delete_code_occurrences(&code), 2);
        assert_eq!(tree.delete_code_occurrences(&code), 0);
        assert_eq!(tree.term_count(), 1);
        assert_eq!(tree.prefix_ref_count(&other), 1);
    }
}
