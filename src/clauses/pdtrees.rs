use std::cell::Cell;
use std::collections::BTreeMap;
#[cfg(feature = "pdt-count-nodes")]
use std::sync::atomic::AtomicU64;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::basics::intmap::{IntMap, IntMapKey};
use crate::basics::objmaps::size_of_obj_map_node_estimate;
use crate::terms::functypes::FunCode;
use crate::terms::termtypes::{term_identity_id, Term};

pub const PDTREE_CELL_MEM: usize = 16;
pub const PDTNODE_MEM: usize = 52;
pub const CLAUSEPOSCELL_MEM: usize = 20;

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

#[derive(Clone, Debug, PartialEq)]
pub struct PdTree {
    nodes: Vec<PdNode>,
    term_count: usize,
    live_node_count: usize,
    arr_storage_estimate: usize,
    match_count: Cell<u64>,
    visited_count: Cell<u64>,
}

#[derive(Clone, Debug, Default, PartialEq)]
struct PdNode {
    children: BTreeMap<PrefixToken, usize>,
    fun_alternatives: IntMap<()>,
    ref_count: usize,
    terminal_count: usize,
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

    pub fn record_search_attempt(&self) {
        self.match_count
            .set(self.match_count.get().saturating_add(1));
    }

    pub fn record_nodes_visited(&self, count: u64) {
        self.visited_count
            .set(self.visited_count.get().saturating_add(count));
        #[cfg(feature = "pdt-count-nodes")]
        record_global_nodes_visited(count);
    }

    pub fn insert_term(&mut self, term: &Term) -> bool {
        let code = term_lr_traverse_code(term);
        self.insert_code(&code)
    }

    pub fn insert_code(&mut self, code: &[PrefixToken]) -> bool {
        let mut node_index = 0;
        self.nodes[node_index].ref_count += 1;

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
        }

        self.nodes[node_index].terminal_count += 1;
        self.term_count += 1;
        true
    }

    pub fn delete_term(&mut self, term: &Term) -> bool {
        let code = term_lr_traverse_code(term);
        self.delete_code(&code)
    }

    pub fn delete_code(&mut self, code: &[PrefixToken]) -> bool {
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

        self.nodes[node_index].terminal_count -= 1;
        self.term_count -= 1;
        self.nodes[0].ref_count -= 1;

        for (_, _, path_node_index) in &path {
            self.nodes[*path_node_index].ref_count -= 1;
        }

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
            self.live_node_count -= 1;
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
        prefix_code_ref_count, prefix_compute_term_code, prefix_match_counts, PdTree, PrefixToken,
        CLAUSEPOSCELL_MEM, PDTNODE_MEM, PDTREE_CELL_MEM,
    };
    use crate::basics::intmap::{INTMAPCELL_MEM, INTORP_MEM, PDARRAYCELL_MEM};
    use crate::basics::objmaps::size_of_obj_map_node_estimate;
    use crate::inout::scanner::Scanner;
    use crate::terms::signature::Signature;
    use crate::terms::termbanks::TermBank;
    use crate::terms::termtypes::Term;
    use crate::terms::typebanks::TypeBank;

    fn parse_in_bank(bank: &mut TermBank, source: &str) -> Term {
        let mut scanner = Scanner::from_user_string(source, false).unwrap();
        bank.parse_term_simple(&mut scanner).unwrap()
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

        tree.record_search_attempt();
        tree.record_nodes_visited(3);
        tree.record_nodes_visited(2);

        assert_eq!(tree.match_count(), 1);
        assert_eq!(tree.visited_count(), 5);
        #[cfg(feature = "pdt-count-nodes")]
        assert!(pdt_node_counter() >= global_before.saturating_add(5));
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
