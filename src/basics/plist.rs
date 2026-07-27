use crate::basics::defines::{IntOrP, IntOrPInt};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct PListHandle(usize);

impl PListHandle {
    #[must_use]
    pub const fn index(self) -> usize {
        self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PListCell<T> {
    key: Option<T>,
    pred: Option<PListHandle>,
    succ: Option<PListHandle>,
    is_anchor: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PListArena<T> {
    cells: Vec<Option<PListCell<T>>>,
    free_cells: Vec<usize>,
}

impl<T> Default for PListArena<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> PListArena<T> {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            cells: Vec::new(),
            free_cells: Vec::new(),
        }
    }

    pub fn alloc_list(&mut self) -> PListHandle {
        let handle = self.alloc_slot();
        self.cells[handle.index()] = Some(PListCell {
            key: None,
            pred: Some(handle),
            succ: Some(handle),
            is_anchor: true,
        });
        handle
    }

    #[must_use]
    pub fn is_valid(&self, handle: PListHandle) -> bool {
        self.cell(handle).is_some()
    }

    #[must_use]
    pub fn is_anchor(&self, handle: PListHandle) -> bool {
        self.cell(handle).is_some_and(|cell| cell.is_anchor)
    }

    #[must_use]
    pub fn is_detached(&self, handle: PListHandle) -> bool {
        self.cell(handle)
            .is_some_and(|cell| !cell.is_anchor && cell.pred.is_none() && cell.succ.is_none())
    }

    #[must_use]
    pub fn is_empty(&self, anchor: PListHandle) -> bool {
        self.cell(anchor)
            .is_some_and(|cell| cell.is_anchor && cell.pred == Some(anchor))
    }

    #[must_use]
    pub fn cardinality(&self, anchor: PListHandle) -> usize {
        self.handles(anchor).len()
    }

    /// Delete every element from the list headed by `anchor`.
    ///
    /// # Panics
    ///
    /// Panics when `anchor` is not a valid list anchor, matching the C
    /// `PListFree`/`PListEmpty` expectation that callers pass a real anchor.
    pub fn clear_list(&mut self, anchor: PListHandle) {
        assert!(
            self.is_anchor(anchor),
            "PListFree expects a valid list anchor"
        );
        while let Some(first) = self.first(anchor) {
            let _ = self.delete(first);
        }
    }

    /// Delete every element and free the anchor cell.
    ///
    /// # Panics
    ///
    /// Panics when `anchor` is not a valid list anchor.
    pub fn free_list(&mut self, anchor: PListHandle) {
        self.clear_list(anchor);
        let _ = self.free_cell_or_panic(anchor, "PListFree");
    }

    /// Allocate a value cell and insert it after `where_handle`.
    ///
    /// # Panics
    ///
    /// Panics when `where_handle` is not linked into a valid list.
    pub fn store_after(&mut self, where_handle: PListHandle, value: T) -> PListHandle {
        let cell = self.alloc_detached(value);
        self.insert_after(where_handle, cell);
        cell
    }

    /// Insert a detached cell after `where_handle`.
    ///
    /// # Panics
    ///
    /// Panics when `where_handle` is not linked into a valid list, when
    /// `cell_handle` is not a detached value cell, or when the handles would
    /// create an invalid self-link. These are safe Rust checks for the raw C
    /// `PListInsert` pointer preconditions.
    pub fn insert_after(&mut self, where_handle: PListHandle, cell_handle: PListHandle) {
        assert_ne!(
            where_handle, cell_handle,
            "PListInsert expects distinct where and cell handles"
        );
        assert!(
            self.is_detached(cell_handle),
            "PListInsert expects a detached cell"
        );
        let successor = self
            .cell_or_panic(where_handle, "PListInsert")
            .succ
            .unwrap_or_else(|| panic!("PListInsert expects where cell to be linked"));
        assert_ne!(
            successor, cell_handle,
            "PListInsert expects successor to differ from inserted cell"
        );
        assert!(
            self.is_valid(successor),
            "PListInsert successor must be a valid cell"
        );

        let cell = self.cell_mut_or_panic(cell_handle, "PListInsert");
        cell.pred = Some(where_handle);
        cell.succ = Some(successor);
        self.cell_mut_or_panic(successor, "PListInsert").pred = Some(cell_handle);
        self.cell_mut_or_panic(where_handle, "PListInsert").succ = Some(cell_handle);
    }

    /// # Panics
    ///
    /// Panics when `element` names a valid anchor or a detached/corrupt cell.
    /// This mirrors the C `PListExtract` assertions that the input cell is
    /// linked into a list and is not the anchor.
    #[must_use]
    pub fn extract(&mut self, element: PListHandle) -> PListHandle {
        let (is_anchor, pred, succ) = {
            let cell = self.cell_or_panic(element, "PListExtract");
            (cell.is_anchor, cell.pred, cell.succ)
        };
        assert!(!is_anchor, "PListExtract expects a non-anchor cell");
        let pred = pred.unwrap_or_else(|| panic!("PListExtract expects a linked predecessor"));
        let succ = succ.unwrap_or_else(|| panic!("PListExtract expects a linked successor"));
        assert_ne!(
            pred, element,
            "PListExtract expects predecessor to differ from element"
        );
        assert_ne!(
            succ, element,
            "PListExtract expects successor to differ from element"
        );
        assert!(
            self.is_valid(pred),
            "PListExtract predecessor must be a valid cell"
        );
        assert!(
            self.is_valid(succ),
            "PListExtract successor must be a valid cell"
        );

        self.cell_mut_or_panic(pred, "PListExtract").succ = Some(succ);
        self.cell_mut_or_panic(succ, "PListExtract").pred = Some(pred);
        let cell = self.cell_mut_or_panic(element, "PListExtract");
        cell.pred = None;
        cell.succ = None;
        element
    }

    /// Delete a linked value cell and return its owned payload.
    ///
    /// # Panics
    ///
    /// Panics when `element` is not a valid linked value cell, matching the C
    /// `PListDelete` call through `PListExtract`.
    #[must_use]
    pub fn delete(&mut self, element: PListHandle) -> T {
        let extracted = self.extract(element);
        self.free_cell_or_panic(extracted, "PListDelete")
            .key
            .unwrap_or_else(|| panic!("PListDelete expects a value cell"))
    }

    #[must_use]
    pub fn value(&self, handle: PListHandle) -> Option<&T> {
        self.cell(handle).and_then(|cell| cell.key.as_ref())
    }

    pub fn value_mut(&mut self, handle: PListHandle) -> Option<&mut T> {
        self.cell_mut(handle).and_then(|cell| cell.key.as_mut())
    }

    #[must_use]
    pub fn first(&self, anchor: PListHandle) -> Option<PListHandle> {
        let cell = self.cell(anchor)?;
        if !cell.is_anchor || cell.succ == Some(anchor) {
            None
        } else {
            cell.succ
        }
    }

    #[must_use]
    pub fn last(&self, anchor: PListHandle) -> Option<PListHandle> {
        let cell = self.cell(anchor)?;
        if !cell.is_anchor || cell.pred == Some(anchor) {
            None
        } else {
            cell.pred
        }
    }

    #[must_use]
    pub fn successor(&self, anchor: PListHandle, element: PListHandle) -> Option<PListHandle> {
        let successor = self.cell(element)?.succ?;
        (successor != anchor).then_some(successor)
    }

    #[must_use]
    pub fn predecessor(&self, anchor: PListHandle, element: PListHandle) -> Option<PListHandle> {
        let predecessor = self.cell(element)?.pred?;
        (predecessor != anchor).then_some(predecessor)
    }

    #[must_use]
    pub fn handles(&self, anchor: PListHandle) -> Vec<PListHandle> {
        let Some(anchor_cell) = self.cell(anchor) else {
            return Vec::new();
        };
        if !anchor_cell.is_anchor {
            return Vec::new();
        }

        let mut result = Vec::new();
        let mut current = anchor_cell.succ;
        while let Some(handle) = current {
            if handle == anchor || result.len() >= self.cells.len() {
                break;
            }
            let Some(cell) = self.cell(handle) else {
                break;
            };
            result.push(handle);
            current = cell.succ;
        }
        result
    }

    #[must_use]
    pub fn entries(&self, anchor: PListHandle) -> Vec<(PListHandle, &T)> {
        self.handles(anchor)
            .into_iter()
            .filter_map(|handle| self.value(handle).map(|value| (handle, value)))
            .collect()
    }

    fn alloc_detached(&mut self, value: T) -> PListHandle {
        let handle = self.alloc_slot();
        self.cells[handle.index()] = Some(PListCell {
            key: Some(value),
            pred: None,
            succ: None,
            is_anchor: false,
        });
        handle
    }

    fn alloc_slot(&mut self) -> PListHandle {
        if let Some(index) = self.free_cells.pop() {
            debug_assert!(self.cells.get(index).is_some_and(Option::is_none));
            PListHandle(index)
        } else {
            let handle = PListHandle(self.cells.len());
            self.cells.push(None);
            handle
        }
    }

    fn free_cell(&mut self, handle: PListHandle) -> Option<PListCell<T>> {
        let cell = self.cells.get_mut(handle.index())?.take()?;
        self.free_cells.push(handle.index());
        Some(cell)
    }

    fn free_cell_or_panic(&mut self, handle: PListHandle, caller: &str) -> PListCell<T> {
        self.free_cell(handle)
            .unwrap_or_else(|| panic!("{caller} called with invalid handle {}", handle.index()))
    }

    fn cell(&self, handle: PListHandle) -> Option<&PListCell<T>> {
        self.cells.get(handle.index())?.as_ref()
    }

    fn cell_or_panic(&self, handle: PListHandle, caller: &str) -> &PListCell<T> {
        self.cell(handle)
            .unwrap_or_else(|| panic!("{caller} called with invalid handle {}", handle.index()))
    }

    fn cell_mut(&mut self, handle: PListHandle) -> Option<&mut PListCell<T>> {
        self.cells.get_mut(handle.index())?.as_mut()
    }

    fn cell_mut_or_panic(&mut self, handle: PListHandle, caller: &str) -> &mut PListCell<T> {
        self.cell_mut(handle)
            .unwrap_or_else(|| panic!("{caller} called with invalid handle {}", handle.index()))
    }
}

impl<P> PListArena<IntOrP<P>> {
    pub fn store_int_after(&mut self, where_handle: PListHandle, value: IntOrPInt) -> PListHandle {
        self.store_after(where_handle, IntOrP::Int(value))
    }

    pub fn store_pointer_after(&mut self, where_handle: PListHandle, value: P) -> PListHandle {
        self.store_after(where_handle, IntOrP::Pointer(value))
    }

    #[must_use]
    pub fn value_int(&self, handle: PListHandle) -> Option<IntOrPInt> {
        self.value(handle).and_then(IntOrP::as_int)
    }

    #[must_use]
    pub fn value_pointer(&self, handle: PListHandle) -> Option<&P> {
        self.value(handle).and_then(IntOrP::as_pointer)
    }
}

#[cfg(test)]
mod tests {
    use super::{IntOrP, PListArena};

    #[test]
    fn alloc_list_creates_empty_self_linked_anchor() {
        let mut arena = PListArena::<usize>::new();
        let anchor = arena.alloc_list();

        assert!(arena.is_anchor(anchor));
        assert!(arena.is_empty(anchor));
        assert_eq!(arena.cardinality(anchor), 0);
        assert_eq!(arena.first(anchor), None);
        assert_eq!(arena.last(anchor), None);
    }

    #[test]
    fn store_inserts_after_requested_cell_like_c() {
        let mut arena = PListArena::new();
        let anchor = arena.alloc_list();
        let first = arena.store_after(anchor, 1);
        let second = arena.store_after(anchor, 2);

        assert_eq!(arena.handles(anchor), vec![second, first]);
        assert_eq!(arena.entries(anchor), vec![(second, &2), (first, &1)]);
        assert_eq!(arena.first(anchor), Some(second));
        assert_eq!(arena.last(anchor), Some(first));
    }

    #[test]
    fn extract_detaches_cell_and_allows_reinsertion_elsewhere() {
        let mut arena = PListArena::new();
        let source = arena.alloc_list();
        let target = arena.alloc_list();
        let one = arena.store_after(source, "one");
        let two = arena.store_after(source, "two");

        assert_eq!(arena.extract(one), one);
        assert!(arena.is_detached(one));
        assert_eq!(arena.handles(source), vec![two]);
        arena.insert_after(target, one);
        assert_eq!(arena.entries(target), vec![(one, &"one")]);
        assert_eq!(arena.predecessor(target, one), None);
        assert_eq!(arena.successor(target, one), None);
    }

    #[test]
    fn delete_removes_cell_and_returns_owned_value() {
        let mut arena = PListArena::new();
        let anchor = arena.alloc_list();
        let first = arena.store_after(anchor, 10);
        let second = arena.store_after(first, 20);

        assert_eq!(arena.delete(first), 10);
        assert!(!arena.is_valid(first));
        assert_eq!(arena.entries(anchor), vec![(second, &20)]);
    }

    #[test]
    #[should_panic(expected = "PListExtract called with invalid handle")]
    fn delete_panics_on_invalid_handle_like_c_pointer_contract() {
        let mut arena = PListArena::new();
        let anchor = arena.alloc_list();
        let first = arena.store_after(anchor, 10);

        assert_eq!(arena.delete(first), 10);
        let _ = arena.delete(first);
    }

    #[test]
    fn deleted_cells_are_reused_by_later_allocations() {
        let mut arena = PListArena::new();
        let anchor = arena.alloc_list();
        let first = arena.store_after(anchor, 10);
        let second = arena.store_after(first, 20);

        assert_eq!(arena.delete(first), 10);
        let reused = arena.store_after(anchor, 30);

        assert_eq!(reused, first);
        assert_eq!(arena.entries(anchor), vec![(reused, &30), (second, &20)]);
    }

    #[test]
    fn clear_and_free_list_preserve_c_anchor_lifetime_shapes() {
        let mut arena = PListArena::new();
        let anchor = arena.alloc_list();
        let first = arena.store_after(anchor, "a");
        arena.store_after(first, "b");

        arena.clear_list(anchor);
        assert!(arena.is_empty(anchor));
        assert!(arena.is_anchor(anchor));
        arena.free_list(anchor);
        assert!(!arena.is_valid(anchor));
    }

    #[test]
    fn freed_anchor_slots_are_reused_by_new_lists() {
        let mut arena = PListArena::new();
        let anchor = arena.alloc_list();
        arena.store_after(anchor, "x");

        arena.free_list(anchor);
        let reused_anchor = arena.alloc_list();

        assert_eq!(reused_anchor, anchor);
        assert!(arena.is_empty(reused_anchor));
    }

    #[test]
    #[should_panic(expected = "PListInsert expects a detached cell")]
    fn attached_reinsertion_panics_like_c_contract_violation() {
        let mut arena = PListArena::new();
        let anchor = arena.alloc_list();
        let first = arena.store_after(anchor, 1);

        arena.insert_after(anchor, first);
    }

    #[test]
    #[should_panic(expected = "PListExtract expects a non-anchor cell")]
    fn extract_asserts_on_anchor_like_c() {
        let mut arena = PListArena::<i32>::new();
        let anchor = arena.alloc_list();

        let _ = arena.extract(anchor);
    }

    #[test]
    #[should_panic(expected = "PListExtract expects a linked predecessor")]
    fn extract_asserts_on_detached_cell_like_c() {
        let mut arena = PListArena::new();
        let source = arena.alloc_list();
        let cell = arena.store_after(source, 1);
        assert_eq!(arena.extract(cell), cell);

        let _ = arena.extract(cell);
    }

    #[test]
    fn mixed_int_pointer_helpers_share_int_or_pointer_shape() {
        let mut arena = PListArena::<IntOrP<&str>>::new();
        let anchor = arena.alloc_list();
        let int_cell = arena.store_int_after(anchor, 5);
        let ptr_cell = arena.store_pointer_after(anchor, "formula");

        assert_eq!(arena.value_int(int_cell), Some(5));
        assert_eq!(arena.value_pointer(ptr_cell), Some(&"formula"));
        assert_eq!(arena.value_int(ptr_cell), None);
        assert_eq!(arena.delete(ptr_cell).into_pointer(), Some("formula"));
    }
}
