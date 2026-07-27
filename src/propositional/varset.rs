//! Port of `PROPOSITIONAL/cpr_varset`.

use crate::propositional::{PLiteralCode, PLITERAL_NO_LIT};

const SENTINEL: usize = 0;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct AtomSetCellId(usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct AtomSetCell {
    atom: PLiteralCode,
    prev: usize,
    succ: usize,
}

/// C `AtomSetCell`/`AtomSet_p` represented as an owned circular list.
///
/// The C API uses the same pointer type for the sentinel set handle and ordinary
/// list cells. Rust keeps the sentinel private and exposes checked cell handles,
/// while preserving the insertion-at-front order, arbitrary cell extraction, and
/// duplicate atoms of the original multiset.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AtomSet {
    cells: Vec<Option<AtomSetCell>>,
    len: usize,
}

impl Default for AtomSet {
    fn default() -> Self {
        Self::new()
    }
}

impl AtomSet {
    #[must_use]
    pub fn new() -> Self {
        Self {
            cells: vec![Some(AtomSetCell {
                atom: PLITERAL_NO_LIT,
                prev: SENTINEL,
                succ: SENTINEL,
            })],
            len: 0,
        }
    }

    #[must_use]
    pub const fn len(&self) -> usize {
        self.len
    }

    /// C `AtomSetEmpty(set)`.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.sentinel().prev == SENTINEL
    }

    #[must_use]
    pub fn front_cell(&self) -> Option<AtomSetCellId> {
        if self.is_empty() {
            None
        } else {
            Some(AtomSetCellId(self.sentinel().succ))
        }
    }

    #[must_use]
    pub fn back_cell(&self) -> Option<AtomSetCellId> {
        if self.is_empty() {
            None
        } else {
            Some(AtomSetCellId(self.sentinel().prev))
        }
    }

    /// Returns the atom currently stored in a live cell.
    ///
    /// # Panics
    ///
    /// Panics if `cell` is stale or does not belong to this set.
    #[must_use]
    pub fn atom(&self, cell: AtomSetCellId) -> PLiteralCode {
        self.live_cell(cell.0).atom
    }

    /// C `AtomSetInsert(set, atom)`.
    ///
    /// Inserts after the sentinel, so repeated front extraction observes LIFO
    /// order just like repeated `AtomSetExtract(set->succ)` in C.
    pub fn insert(&mut self, atom: PLiteralCode) -> AtomSetCellId {
        let succ = self.sentinel().succ;
        let index = self.cells.len();
        self.cells.push(Some(AtomSetCell {
            atom,
            prev: SENTINEL,
            succ,
        }));
        self.live_cell_mut(succ).prev = index;
        self.sentinel_mut().succ = index;
        self.len += 1;
        AtomSetCellId(index)
    }

    /// C `AtomSetExtract(var)`.
    ///
    /// # Panics
    ///
    /// Panics if `cell` is stale, if it is not a cell in this set, or if it
    /// stores `PLiteralNoLit`. The latter mirrors the C assertion after the
    /// node has already been unlinked.
    pub fn extract(&mut self, cell: AtomSetCellId) -> PLiteralCode {
        assert_ne!(
            cell.0, SENTINEL,
            "AtomSetExtract requires a cell, not the set sentinel"
        );

        let removed = match self.cells.get_mut(cell.0) {
            Some(slot @ Some(_)) => match slot.take() {
                Some(node) => node,
                None => unreachable!("live AtomSet slot unexpectedly vanished"),
            },
            _ => panic!("AtomSetExtract received a stale cell handle"),
        };

        self.live_cell_mut(removed.succ).prev = removed.prev;
        self.live_cell_mut(removed.prev).succ = removed.succ;
        self.len -= 1;

        assert_ne!(
            removed.atom, PLITERAL_NO_LIT,
            "AtomSetExtract requires a real propositional literal"
        );
        removed.atom
    }

    /// Extracts `set->succ`, matching the loop used by C `AtomSetFree`.
    ///
    /// # Panics
    ///
    /// Panics under the same conditions as [`Self::extract`].
    pub fn extract_front(&mut self) -> Option<PLiteralCode> {
        self.front_cell().map(|cell| self.extract(cell))
    }

    /// Drains the set in C `AtomSetFree` order.
    ///
    /// # Panics
    ///
    /// Panics if any stored cell contains `PLiteralNoLit`, matching
    /// `AtomSetExtract`.
    pub fn clear(&mut self) {
        while self.extract_front().is_some() {}
    }

    #[must_use]
    pub fn iter(&self) -> AtomSetIter<'_> {
        AtomSetIter {
            set: self,
            next: self.sentinel().succ,
        }
    }

    fn sentinel(&self) -> &AtomSetCell {
        self.live_cell(SENTINEL)
    }

    fn sentinel_mut(&mut self) -> &mut AtomSetCell {
        self.live_cell_mut(SENTINEL)
    }

    fn live_cell(&self, index: usize) -> &AtomSetCell {
        match self.cells.get(index) {
            Some(Some(cell)) => cell,
            _ => panic!("AtomSet internal cell handle is not live"),
        }
    }

    fn live_cell_mut(&mut self, index: usize) -> &mut AtomSetCell {
        match self.cells.get_mut(index) {
            Some(Some(cell)) => cell,
            _ => panic!("AtomSet internal cell handle is not live"),
        }
    }
}

pub struct AtomSetIter<'a> {
    set: &'a AtomSet,
    next: usize,
}

impl<'a> IntoIterator for &'a AtomSet {
    type IntoIter = AtomSetIter<'a>;
    type Item = PLiteralCode;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl Iterator for AtomSetIter<'_> {
    type Item = PLiteralCode;

    fn next(&mut self) -> Option<Self::Item> {
        if self.next == SENTINEL {
            return None;
        }
        let cell = self.set.live_cell(self.next);
        self.next = cell.succ;
        Some(cell.atom)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, Some(self.set.len()))
    }
}

#[cfg(test)]
mod tests {
    use super::AtomSet;
    use crate::propositional::{p_atom_p, PLITERAL_NO_LIT};

    #[test]
    fn new_set_is_empty() {
        let set = AtomSet::new();

        assert!(set.is_empty());
        assert_eq!(set.len(), 0);
        assert_eq!(set.front_cell(), None);
        assert_eq!(set.back_cell(), None);
        assert!(set.iter().next().is_none());
    }

    #[test]
    fn insertion_is_lifo_and_keeps_duplicates() {
        let mut set = AtomSet::new();

        set.insert(1);
        set.insert(2);
        set.insert(2);
        set.insert(-3);

        assert!(!set.is_empty());
        assert_eq!(set.len(), 4);
        assert_eq!(set.iter().collect::<Vec<_>>(), vec![-3, 2, 2, 1]);
        assert_eq!(set.extract_front(), Some(-3));
        assert_eq!(set.extract_front(), Some(2));
        assert_eq!(set.extract_front(), Some(2));
        assert_eq!(set.extract_front(), Some(1));
        assert_eq!(set.extract_front(), None);
        assert!(set.is_empty());
    }

    #[test]
    fn arbitrary_extraction_relinks_neighbors() {
        let mut set = AtomSet::new();
        let first = set.insert(10);
        let middle = set.insert(20);
        let last = set.insert(30);

        assert_eq!(set.iter().collect::<Vec<_>>(), vec![30, 20, 10]);
        assert_eq!(set.front_cell(), Some(last));
        assert_eq!(set.back_cell(), Some(first));

        assert_eq!(set.extract(middle), 20);
        assert_eq!(set.iter().collect::<Vec<_>>(), vec![30, 10]);
        assert_eq!(set.front_cell(), Some(last));
        assert_eq!(set.back_cell(), Some(first));

        assert_eq!(set.extract(first), 10);
        assert_eq!(set.iter().collect::<Vec<_>>(), vec![30]);
        assert_eq!(set.front_cell(), Some(last));
        assert_eq!(set.back_cell(), Some(last));

        assert_eq!(set.extract(last), 30);
        assert!(set.is_empty());
    }

    #[test]
    fn atom_reads_live_cell_payloads() {
        let mut set = AtomSet::new();
        let first = set.insert(7);
        let second = set.insert(-11);

        assert_eq!(set.atom(first), 7);
        assert_eq!(set.atom(second), -11);
    }

    #[test]
    fn clear_drains_in_extract_front_order() {
        let mut set = AtomSet::new();
        set.insert(1);
        set.insert(2);
        set.insert(3);

        set.clear();

        assert!(set.is_empty());
        assert_eq!(set.len(), 0);
        assert_eq!(set.extract_front(), None);
    }

    #[test]
    #[should_panic(expected = "AtomSetExtract received a stale cell handle")]
    fn extracting_a_removed_cell_panics_like_invalid_c_cell_use() {
        let mut set = AtomSet::new();
        let cell = set.insert(1);

        assert_eq!(set.extract(cell), 1);
        let _ = set.extract(cell);
    }

    #[test]
    #[should_panic(expected = "AtomSetExtract requires a real propositional literal")]
    fn no_literal_payload_is_rejected_on_extraction() {
        let mut set = AtomSet::new();
        set.insert(PLITERAL_NO_LIT);

        let _ = set.extract_front();
    }

    #[test]
    fn literal_code_helpers_match_c_macros() {
        assert!(p_atom_p(1));
        assert!(p_atom_p(i64::MAX));
        assert!(!p_atom_p(PLITERAL_NO_LIT));
        assert!(!p_atom_p(-1));
    }
}
