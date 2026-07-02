use std::collections::BTreeMap;

use crate::terms::functypes::FunCode;
use crate::terms::termtypes::{term_identity_id, Term};

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

#[derive(Clone, Debug)]
pub struct PdTree {
    nodes: Vec<PdNode>,
    term_count: usize,
}

#[derive(Clone, Debug, Default)]
struct PdNode {
    children: BTreeMap<PrefixToken, usize>,
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
        self.nodes.len().saturating_sub(1)
    }

    #[must_use]
    pub const fn term_count(&self) -> usize {
        self.term_count
    }

    pub fn insert_term(&mut self, term: &Term) -> bool {
        let code = term_lr_traverse_code(term);
        self.insert_code(&code)
    }

    pub fn insert_code(&mut self, code: &[PrefixToken]) -> bool {
        let mut node_index = 0;
        self.nodes[node_index].ref_count += 1;

        for token in code {
            let next_index =
                if let Some(existing) = self.nodes[node_index].children.get(token).copied() {
                    existing
                } else {
                    let created = self.nodes.len();
                    self.nodes.push(PdNode::default());
                    self.nodes[node_index].children.insert(*token, created);
                    created
                };
            node_index = next_index;
            self.nodes[node_index].ref_count += 1;
        }

        self.nodes[node_index].terminal_count += 1;
        self.term_count += 1;
        true
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
    use super::{
        prefix_code_ref_count, prefix_compute_term_code, prefix_match_counts, PdTree, PrefixToken,
    };
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
}
