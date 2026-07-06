use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
#[cfg(feature = "pdt-count-nodes")]
use std::sync::atomic::AtomicU64;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::basics::intmap::{IntMap, IntMapKey};
use crate::basics::objmaps::size_of_obj_map_node_estimate;
use crate::basics::sysdate::SysDate;
use crate::clauses::eqn_props::EqnSide;
use crate::terms::functypes::FunCode;
use crate::terms::termfunc::term_standard_weight;
use crate::terms::termtypes::{term_identity_id, Term};

pub const PDTREE_CELL_MEM: usize = 16;
pub const PDTNODE_MEM: usize = 52;
pub const CLAUSEPOSCELL_MEM: usize = 20;
pub const PDTREE_IGNORE_TERM_WEIGHT: i64 = i64::MAX;
pub const PDTREE_IGNORE_NF_DATE: SysDate = SysDate::creation_time();

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
    pub term_code: Vec<PrefixToken>,
    pub term_spans: Vec<usize>,
    pub term_weight: i64,
    pub term_date: SysDate,
    pub traversal_order: PdtTraversalOrder,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum PrefixToken {
    Fun(FunCode),
    FreeVar(usize),
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
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct PrefixQueryCell {
    token: PrefixToken,
    span: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PdTree {
    nodes: Vec<PdNode>,
    term_count: usize,
    live_node_count: usize,
    arr_storage_estimate: usize,
    match_count: Cell<u64>,
    visited_count: Cell<u64>,
    search_traversal_order: Cell<PdtTraversalOrder>,
    search_active: Cell<bool>,
    search_state: RefCell<Option<PdtSearchState>>,
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct PdTerminalEntry {
    weight: i64,
    date: Option<SysDate>,
    occurrence: Option<PdtIndexedOccurrence>,
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
        Self { clause_id, side }
    }
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
            term_count: 0,
            live_node_count: 0,
            arr_storage_estimate: 0,
            match_count: Cell::new(0),
            visited_count: Cell::new(0),
            search_traversal_order: Cell::new(PdtTraversalOrder::default()),
            search_active: Cell::new(false),
            search_state: RefCell::new(None),
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
        self.code_may_have_matchable_path(&state.term_code, &state.term_spans)
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
        self.search_state
            .borrow()
            .as_ref()
            .map_or(PDTREE_IGNORE_TERM_WEIGHT, |state| state.term_weight)
    }

    #[must_use]
    pub fn search_term_date(&self) -> SysDate {
        self.search_state
            .borrow()
            .as_ref()
            .map_or(PDTREE_IGNORE_NF_DATE, |state| state.term_date)
    }

    pub fn record_search_attempt(&self) {
        self.match_count
            .set(self.match_count.get().saturating_add(1));
    }

    pub fn record_search_init(&self, term: &Term, age_constraint: SysDate, prefer_general: bool) {
        let traversal_order = PdtTraversalOrder::from_prefer_general(prefer_general);
        let query = term_lr_traverse_query(term);
        self.search_traversal_order.set(traversal_order);
        *self.search_state.borrow_mut() = Some(PdtSearchState {
            term_code: query.iter().map(|cell| cell.token).collect(),
            term_spans: query.iter().map(|cell| cell.span).collect(),
            term_weight: term_standard_weight(term),
            term_date: age_constraint,
            traversal_order,
        });
        self.search_active.set(true);
        self.record_search_attempt();
    }

    pub fn record_search_exit(&self) {
        self.search_active.set(false);
    }

    pub fn record_nodes_visited(&self, count: u64) {
        self.visited_count
            .set(self.visited_count.get().saturating_add(count));
        #[cfg(feature = "pdt-count-nodes")]
        record_global_nodes_visited(count);
    }

    pub fn insert_term(&mut self, term: &Term) -> bool {
        let code = term_lr_traverse_code(term);
        self.insert_code_with_metadata(&code, term_standard_weight(term), None)
    }

    pub fn insert_term_with_clause_date(&mut self, term: &Term, clause_date: SysDate) -> bool {
        let code = term_lr_traverse_code(term);
        self.insert_code_with_metadata(&code, term_standard_weight(term), Some(clause_date))
    }

    pub fn insert_term_occurrence(
        &mut self,
        term: &Term,
        clause_date: SysDate,
        occurrence: PdtIndexedOccurrence,
    ) -> bool {
        let code = term_lr_traverse_code(term);
        let entry = PdTerminalEntry::with_occurrence(
            term_standard_weight(term),
            Some(clause_date),
            occurrence,
        );
        self.insert_code_with_entry(&code, entry)
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

    fn insert_code_with_entry(&mut self, code: &[PrefixToken], entry: PdTerminalEntry) -> bool {
        let mut node_index = 0;
        self.nodes[node_index].ref_count += 1;
        self.apply_entry_to_node(node_index, entry);

        for token in code {
            self.select_alt_ref_for_insert(node_index, *token);
            let next_index =
                if let Some(existing) = self.nodes[node_index].children.get(token).copied() {
                    existing
                } else {
                    let created = self.nodes.len();
                    self.nodes.push(PdNode::default());
                    self.nodes[node_index].children.insert(*token, created);
                    self.live_node_count += 1;
                    self.arr_storage_estimate = self.arr_storage_estimate.saturating_add(
                        self.nodes[created]
                            .fun_alternatives
                            .constant_mem_storage_estimate(),
                    );
                    created
                };
            node_index = next_index;
            self.nodes[node_index].ref_count += 1;
            self.apply_entry_to_node(node_index, entry);
        }

        self.nodes[node_index].terminal_count += 1;
        self.nodes[node_index].terminal_entries.push(entry);
        self.term_count += 1;
        true
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
        let code = term_lr_traverse_code(term);
        let entry = PdTerminalEntry::with_occurrence(
            term_standard_weight(term),
            Some(clause_date),
            occurrence,
        );
        self.delete_code_with_entry(&code, Some(entry))
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
                PrefixToken::FreeVar(_) | PrefixToken::DbLike(_) => {
                    self.arr_storage_estimate = self
                        .arr_storage_estimate
                        .saturating_sub(size_of_obj_map_node_estimate());
                }
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
            PrefixToken::FreeVar(_) | PrefixToken::DbLike(_) => {
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
        if state.term_code.len() != state.term_spans.len() {
            return None;
        }
        if !self.root_satisfies_constraints(state.term_weight, state.term_date) {
            return Some(Vec::new());
        }

        let mut occurrences = Vec::new();
        self.collect_matching_occurrences(
            0,
            0,
            &state.term_code,
            &state.term_spans,
            state.traversal_order,
            &mut occurrences,
        );
        Some(occurrences)
    }

    fn code_may_have_matchable_path(&self, code: &[PrefixToken], spans: &[usize]) -> bool {
        if code.len() != spans.len() {
            return true;
        }
        self.node_may_have_matchable_path(0, 0, code, spans)
    }

    fn node_may_have_matchable_path(
        &self,
        node_index: usize,
        query_index: usize,
        code: &[PrefixToken],
        spans: &[usize],
    ) -> bool {
        if query_index == code.len() {
            return self.nodes[node_index].terminal_count != 0;
        }

        let token = code[query_index];
        if !matches!(token, PrefixToken::FreeVar(_))
            && self.nodes[node_index]
                .children
                .get(&token)
                .is_some_and(|next_index| {
                    self.node_may_have_matchable_path(*next_index, query_index + 1, code, spans)
                })
        {
            return true;
        }

        let next_query_index = query_index.saturating_add(spans[query_index]);
        if next_query_index > code.len() {
            return true;
        }
        self.nodes[node_index]
            .children
            .iter()
            .filter(|(edge, _)| matches!(edge, PrefixToken::FreeVar(_)))
            .any(|(_, next_index)| {
                self.node_may_have_matchable_path(*next_index, next_query_index, code, spans)
            })
    }

    fn collect_matching_occurrences(
        &self,
        node_index: usize,
        query_index: usize,
        code: &[PrefixToken],
        spans: &[usize],
        traversal_order: PdtTraversalOrder,
        occurrences: &mut Vec<PdtIndexedOccurrence>,
    ) {
        if query_index == code.len() {
            for occurrence in self.nodes[node_index]
                .terminal_entries
                .iter()
                .filter_map(|entry| entry.occurrence)
            {
                if !occurrences.contains(&occurrence) {
                    occurrences.push(occurrence);
                }
            }
            return;
        }

        for step in [traversal_order.first, traversal_order.second] {
            match step {
                PdtTraversalStep::Symbols => self.collect_symbol_matching_occurrences(
                    node_index,
                    query_index,
                    code,
                    spans,
                    traversal_order,
                    occurrences,
                ),
                PdtTraversalStep::Variables => self.collect_variable_matching_occurrences(
                    node_index,
                    query_index,
                    code,
                    spans,
                    traversal_order,
                    occurrences,
                ),
            }
        }
    }

    fn collect_symbol_matching_occurrences(
        &self,
        node_index: usize,
        query_index: usize,
        code: &[PrefixToken],
        spans: &[usize],
        traversal_order: PdtTraversalOrder,
        occurrences: &mut Vec<PdtIndexedOccurrence>,
    ) {
        let token = code[query_index];
        if !matches!(token, PrefixToken::FreeVar(_)) {
            if let Some(next_index) = self.nodes[node_index].children.get(&token).copied() {
                self.collect_matching_occurrences(
                    next_index,
                    query_index + 1,
                    code,
                    spans,
                    traversal_order,
                    occurrences,
                );
            }
        }
    }

    fn collect_variable_matching_occurrences(
        &self,
        node_index: usize,
        query_index: usize,
        code: &[PrefixToken],
        spans: &[usize],
        traversal_order: PdtTraversalOrder,
        occurrences: &mut Vec<PdtIndexedOccurrence>,
    ) {
        let next_query_index = query_index.saturating_add(spans[query_index]);
        if next_query_index > code.len() {
            return;
        }
        for next_index in self.nodes[node_index]
            .children
            .iter()
            .filter_map(|(edge, next_index)| {
                matches!(edge, PrefixToken::FreeVar(_)).then_some(*next_index)
            })
        {
            self.collect_matching_occurrences(
                next_index,
                next_query_index,
                code,
                spans,
                traversal_order,
                occurrences,
            );
        }
    }
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

fn term_lr_traverse_query(term: &Term) -> Vec<PrefixQueryCell> {
    let mut query = Vec::new();
    push_prefix_query_cell(&mut query, term);
    query
}

fn push_prefix_query_cell(query: &mut Vec<PrefixQueryCell>, term: &Term) -> usize {
    let start = query.len();
    query.push(PrefixQueryCell {
        token: prefix_token(term),
        span: 0,
    });

    if !term.is_top_level_free_var() {
        let first_arg = usize::from(term.is_lambda() || term.is_applied_db_var());
        for index in first_arg..term.arity() {
            let arg = term
                .argument(index)
                .unwrap_or_else(|| panic!("term argument {index} is uninitialized"));
            push_prefix_query_cell(query, &arg);
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
        PrefixToken::FreeVar(term_identity_id(term))
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
mod tests {
    #[cfg(feature = "pdt-count-nodes")]
    use super::pdt_node_counter;
    use super::{
        prefix_code_ref_count, prefix_compute_term_code, prefix_match_counts,
        term_lr_traverse_query, PdTree, PdtIndexedOccurrence, PdtTraversalOrder, PrefixToken,
        CLAUSEPOSCELL_MEM, PDTNODE_MEM, PDTREE_CELL_MEM, PDTREE_IGNORE_NF_DATE,
        PDTREE_IGNORE_TERM_WEIGHT,
    };
    use crate::basics::intmap::{INTMAPCELL_MEM, INTORP_MEM, PDARRAYCELL_MEM};
    use crate::basics::objmaps::size_of_obj_map_node_estimate;
    use crate::basics::sysdate::SysDate;
    use crate::clauses::eqn_props::EqnSide;
    use crate::inout::scanner::Scanner;
    use crate::terms::signature::Signature;
    use crate::terms::termbanks::TermBank;
    use crate::terms::termfunc::term_standard_weight;
    use crate::terms::termtypes::Term;
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
        assert_eq!(state.term_code, prefix_compute_term_code(&first));
        assert_eq!(state.term_spans, vec![2, 1]);
        assert_eq!(state.traversal_order, PdtTraversalOrder::variables_first());

        tree.record_search_exit();

        assert!(!tree.search_is_active());
        assert_eq!(tree.search_term_weight(), term_standard_weight(&first));

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
        assert_eq!(state.term_code, prefix_compute_term_code(&second));
        assert_eq!(state.term_spans, vec![3, 1, 1]);
        assert_eq!(state.traversal_order, PdtTraversalOrder::symbols_first());
    }

    #[test]
    fn query_spans_count_whole_lr_subtrees() {
        let mut bank = TermBank::new(Signature::new(TypeBank::new())).unwrap();
        let term = parse_in_bank(&mut bank, "span_f(span_g(span_a),span_b)");
        let query = term_lr_traverse_query(&term);

        assert_eq!(
            query.iter().map(|cell| cell.token).collect::<Vec<_>>(),
            prefix_compute_term_code(&term)
        );
        assert_eq!(
            query.iter().map(|cell| cell.span).collect::<Vec<_>>(),
            vec![4, 2, 1, 1]
        );
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
            PrefixToken::FreeVar(17),
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

        assert_eq!(tree.search_matching_occurrences(), Some(vec![left, right]));

        assert!(tree.delete_term_occurrence(&term, SysDate::from_raw(7), left));
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

        tree.record_search_init(&query, PDTREE_IGNORE_NF_DATE, true);
        assert_eq!(
            tree.search_matching_occurrences(),
            Some(vec![specific, general])
        );
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

        assert!(tree.insert_code(&[PrefixToken::FreeVar(7), PrefixToken::DbLike(3)]));

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
