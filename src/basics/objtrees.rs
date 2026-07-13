use std::collections::BTreeSet;
use std::rc::Rc;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObjTree<T> {
    objects: BTreeSet<Rc<T>>,
    root_object: Option<Rc<T>>,
}

impl<T> Default for ObjTree<T>
where
    T: Ord + Clone,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<T> ObjTree<T>
where
    T: Ord + Clone,
{
    #[must_use]
    pub const fn new() -> Self {
        Self {
            objects: BTreeSet::new(),
            root_object: None,
        }
    }

    #[must_use]
    pub fn nodes(&self) -> usize {
        self.objects.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.objects.is_empty()
    }

    #[must_use]
    pub fn root_object(&self) -> Option<&T> {
        self.root_object.as_deref()
    }

    pub fn store(&mut self, object: T) -> Option<&T> {
        if self.objects.contains(&object) {
            let existing = self.objects.get(&object);
            self.root_object = existing.cloned();
            existing.map(Rc::as_ref)
        } else {
            let object = Rc::new(object);
            self.root_object = Some(Rc::clone(&object));
            self.objects.insert(object);
            None
        }
    }

    #[must_use]
    pub fn find(&self, key: &T) -> Option<&T> {
        self.objects.get(key).map(Rc::as_ref)
    }

    #[must_use]
    pub fn find_binary(&self, key: &T) -> Option<&T> {
        self.find(key)
    }

    pub fn find_splayed(&mut self, key: &T) -> Option<&T> {
        let found = self.objects.get(key);
        if found.is_some() {
            self.root_object = found.cloned();
        }
        found.map(Rc::as_ref)
    }

    pub fn extract_object(&mut self, key: &T) -> Option<T> {
        let removed = self.objects.take(key);
        if removed.is_some() {
            if self.root_object.as_deref() == Some(key) {
                self.root_object = None;
            }
            self.root_object = self.objects.iter().next().cloned();
        }
        removed.map(|object| Rc::try_unwrap(object).unwrap_or_else(|shared| (*shared).clone()))
    }

    pub fn extract_root_object(&mut self) -> Option<T> {
        let key = match self.root_object.as_ref() {
            Some(key) if self.objects.contains(key.as_ref()) => Rc::clone(key),
            _ => Rc::clone(self.objects.iter().next()?),
        };
        self.extract_object(key.as_ref())
    }

    /// # Panics
    ///
    /// Panics if `add` contains an object already present in this tree. This
    /// mirrors the C `PTreeObjMerge` assertion that input trees are disjoint.
    pub fn merge_unique(&mut self, add: Self) {
        let Self {
            objects,
            root_object,
        } = add;
        drop(root_object);
        for object in objects {
            let object = Rc::try_unwrap(object).unwrap_or_else(|shared| (*shared).clone());
            assert!(
                self.store(object).is_none(),
                "ObjTree merge expects disjoint trees"
            );
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.objects.iter().map(Rc::as_ref)
    }

    #[must_use]
    pub fn to_vec(&self) -> Vec<T> {
        self.objects
            .iter()
            .map(|object| (**object).clone())
            .collect()
    }

    pub fn free_with<F>(self, mut del_fun: F)
    where
        F: FnMut(T),
    {
        let Self {
            objects,
            root_object,
        } = self;
        drop(root_object);
        for object in objects {
            del_fun(Rc::try_unwrap(object).unwrap_or_else(|shared| (*shared).clone()));
        }
    }

    pub fn dummy_del_fun(_object: T) {}
}

#[cfg(test)]
mod tests {
    use super::ObjTree;
    use std::{cell::Cell, cmp::Ordering, rc::Rc};

    #[derive(Debug)]
    struct CloneCounted {
        value: i32,
        clones: Rc<Cell<usize>>,
    }

    impl Clone for CloneCounted {
        fn clone(&self) -> Self {
            self.clones.set(self.clones.get() + 1);
            Self {
                value: self.value,
                clones: Rc::clone(&self.clones),
            }
        }
    }

    impl PartialEq for CloneCounted {
        fn eq(&self, other: &Self) -> bool {
            self.value == other.value
        }
    }

    impl Eq for CloneCounted {}

    impl PartialOrd for CloneCounted {
        fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
            Some(self.cmp(other))
        }
    }

    impl Ord for CloneCounted {
        fn cmp(&self, other: &Self) -> Ordering {
            self.value.cmp(&other.value)
        }
    }

    fn tree(values: &[i32]) -> ObjTree<i32> {
        let mut tree = ObjTree::new();
        for value in values {
            tree.store(*value);
        }
        tree
    }

    #[test]
    fn store_find_and_duplicates_return_existing_object() {
        let mut tree = ObjTree::new();

        assert_eq!(tree.store(10), None);
        assert_eq!(tree.store(3), None);
        assert_eq!(tree.store(10), Some(&10));
        assert_eq!(tree.nodes(), 2);
        assert_eq!(tree.root_object(), Some(&10));
        assert_eq!(tree.find(&3), Some(&3));
        assert_eq!(tree.find_binary(&99), None);
    }

    #[test]
    fn splayed_find_tracks_recent_root_like_c() {
        let mut tree = tree(&[3, 1, 2]);

        assert_eq!(tree.find_splayed(&1), Some(&1));
        assert_eq!(tree.root_object(), Some(&1));
        assert_eq!(tree.find_splayed(&99), None);
        assert_eq!(tree.root_object(), Some(&1));
    }

    #[test]
    fn extract_object_and_root_remove_values() {
        let mut tree = tree(&[3, 1, 2]);
        assert_eq!(tree.extract_object(&2), Some(2));
        assert_eq!(tree.extract_object(&9), None);
        assert_eq!(tree.to_vec(), vec![1, 3]);

        assert_eq!(tree.extract_root_object(), Some(1));
        assert_eq!(tree.to_vec(), vec![3]);
    }

    #[test]
    fn merge_unique_consumes_disjoint_source() {
        let mut base = tree(&[1, 3]);
        base.merge_unique(tree(&[2, 4]));
        assert_eq!(base.to_vec(), vec![1, 2, 3, 4]);
    }

    #[test]
    #[should_panic(expected = "ObjTree merge expects disjoint trees")]
    fn merge_unique_asserts_on_duplicate_like_c() {
        tree(&[1, 3]).merge_unique(tree(&[3, 4]));
    }

    #[test]
    fn free_with_visits_all_objects_in_sorted_order() {
        let tree = tree(&[3, 1, 2]);
        let mut deleted = Vec::new();
        tree.free_with(|object| deleted.push(object));
        assert_eq!(deleted, vec![1, 2, 3]);
    }

    #[test]
    fn root_tracking_and_extraction_do_not_clone_payloads() {
        let clones = Rc::new(Cell::new(0));
        let counted = |value| CloneCounted {
            value,
            clones: Rc::clone(&clones),
        };
        let mut tree = ObjTree::new();

        assert!(tree.store(counted(2)).is_none());
        assert!(tree.store(counted(1)).is_none());
        assert!(tree.store(counted(2)).is_some());
        assert_eq!(
            tree.find_splayed(&counted(1)).map(|item| item.value),
            Some(1)
        );
        assert_eq!(tree.root_object().map(|item| item.value), Some(1));
        assert_eq!(
            tree.extract_object(&counted(1)).map(|item| item.value),
            Some(1)
        );
        assert_eq!(clones.get(), 0);
    }
}
