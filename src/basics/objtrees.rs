use std::collections::BTreeSet;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObjTree<T> {
    objects: BTreeSet<T>,
    root_object: Option<T>,
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
    pub const fn root_object(&self) -> Option<&T> {
        self.root_object.as_ref()
    }

    pub fn store(&mut self, object: T) -> Option<&T> {
        if self.objects.contains(&object) {
            let existing = self.objects.get(&object);
            self.root_object = existing.cloned();
            existing
        } else {
            self.root_object = Some(object.clone());
            self.objects.insert(object);
            None
        }
    }

    #[must_use]
    pub fn find(&self, key: &T) -> Option<&T> {
        self.objects.get(key)
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
        found
    }

    pub fn extract_object(&mut self, key: &T) -> Option<T> {
        let removed = self.objects.take(key);
        if removed.is_some() {
            self.root_object = self.objects.iter().next().cloned();
        }
        removed
    }

    pub fn extract_root_object(&mut self) -> Option<T> {
        let key = match self.root_object.as_ref() {
            Some(key) if self.objects.contains(key) => key.clone(),
            _ => self.objects.iter().next()?.clone(),
        };
        self.extract_object(&key)
    }

    pub fn merge_unique(&mut self, add: Self) -> bool {
        let mut all_unique = true;
        for object in add.objects {
            if self.store(object).is_some() {
                all_unique = false;
            }
        }
        all_unique
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.objects.iter()
    }

    #[must_use]
    pub fn to_vec(&self) -> Vec<T> {
        self.objects.iter().cloned().collect()
    }

    pub fn free_with<F>(self, mut del_fun: F)
    where
        F: FnMut(T),
    {
        for object in self.objects {
            del_fun(object);
        }
    }

    pub fn dummy_del_fun(_object: T) {}
}

#[cfg(test)]
mod tests {
    use super::ObjTree;

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
    fn merge_unique_consumes_source_and_reports_duplicates() {
        let mut base = tree(&[1, 3]);
        assert!(base.merge_unique(tree(&[2, 4])));
        assert_eq!(base.to_vec(), vec![1, 2, 3, 4]);
        assert!(!base.merge_unique(tree(&[4, 5])));
        assert_eq!(base.to_vec(), vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn free_with_visits_all_objects_in_sorted_order() {
        let tree = tree(&[3, 1, 2]);
        let mut deleted = Vec::new();
        tree.free_with(|object| deleted.push(object));
        assert_eq!(deleted, vec![1, 2, 3]);
    }
}
