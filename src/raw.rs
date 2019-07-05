use core::marker::PhantomData;

use crate::index::{Index, IndexSelection, IndexRoute};
use crate::traits::{Backend, Value, ValueOf, RootStatus, Owned, Dangling, Leak, Error};

/// `Raw` with owned root.
pub type OwnedRaw<DB> = Raw<Owned, DB>;

/// `Raw` with dangling root.
pub type DanglingRaw<DB> = Raw<Dangling, DB>;

/// Raw merkle tree.
pub struct Raw<R: RootStatus, DB: Backend> {
    root: ValueOf<DB>,
    _marker: PhantomData<R>,
}

impl<R: RootStatus, DB: Backend> Default for Raw<R, DB> {
    fn default() -> Self {
        Self {
            root: Value::End(Default::default()),
            _marker: PhantomData,
        }
    }
}

impl<R: RootStatus, DB: Backend> Raw<R, DB> {
    /// Return the root of the tree.
    pub fn root(&self) -> ValueOf<DB> {
        self.root.clone()
    }

    /// Return a reference to a subtree.
    pub fn subtree(&self, db: &DB, index: Index) -> Result<DanglingRaw<DB>, Error<DB::Error>> {
        let subroot = self.get(db, index)?.ok_or(Error::CorruptedDatabase)?;
        Ok(Raw {
            root: subroot,
            _marker: PhantomData,
        })
    }

    /// Get value from the tree via generalized merkle index.
    pub fn get(&self, db: &DB, index: Index) -> Result<Option<ValueOf<DB>>, Error<DB::Error>> {
        match index.route() {
            IndexRoute::Root => Ok(Some(self.root.clone())),
            IndexRoute::Select(selections) => {
                let mut current = self.root.clone();

                for selection in selections {
                    let intermediate = match current {
                        Value::Intermediate(intermediate) => intermediate,
                        Value::End(_) => return Ok(None),
                    };

                    let pair = db.get(&intermediate)?;
                    current = match selection {
                        IndexSelection::Left => pair.0.clone(),
                        IndexSelection::Right => pair.1.clone(),
                    };
                }

                Ok(Some(current))
            },
        }
    }

    /// Set value of the merkle tree via generalized merkle index.
    pub fn set(
        &mut self,
        db: &mut DB,
        index: Index,
        set: ValueOf<DB>
    ) -> Result<(), Error<DB::Error>> {
        let route = index.route();

        match set.clone() {
            Value::End(_) => (),
            Value::Intermediate(key) => {
                let value = db.get(&key)?;
                db.insert(key, value)?;
            },
        };

        let mut values = {
            let mut values = Vec::new();
            let mut depth = 1;
            let mut current = match self.root.clone() {
                Value::Intermediate(intermediate) => {
                    Some(intermediate)
                },
                Value::End(_) => {
                    let sel = match route.at_depth(depth) {
                        Some(sel) => sel,
                        None => {
                            match &set {
                                Value::End(_) => (),
                                Value::Intermediate(key) => {
                                    if R::is_owned() {
                                        db.rootify(key)?;
                                    }
                                }
                            }
                            self.root = set;
                            return Ok(())
                        },
                    };
                    values.push((sel, (db.empty_at(0)?, db.empty_at(0)?)));
                    depth += 1;
                    None
                },
            };

            loop {
                let sel = match route.at_depth(depth) {
                    Some(sel) => sel,
                    None => break,
                };
                match current.clone() {
                    Some(cur) => {
                        let value = db.get(&cur)?;
                        values.push((sel, value.clone()));
                        current = match sel {
                            IndexSelection::Left => value.0.intermediate(),
                            IndexSelection::Right => value.1.intermediate(),
                        };
                    },
                    None => {
                        values.push((sel, (db.empty_at(0)?, db.empty_at(0)?)));
                    },
                }
                depth += 1;
            }

            values
        };

        let mut update = set;
        loop {
            let (sel, mut value) = match values.pop() {
                Some(v) => v,
                None => break,
            };

            match sel {
                IndexSelection::Left => { value.0 = update.clone(); }
                IndexSelection::Right => { value.1 = update.clone(); }
            }

            let intermediate = db.intermediate_of(&value.0, &value.1);

            db.insert(intermediate.clone(), value)?;
            update = Value::Intermediate(intermediate);
        }

        match &update {
            Value::Intermediate(ref key) => {
                if R::is_owned() {
                    db.rootify(key)?;
                }
            }
            Value::End(_) => (),
        }
        match &self.root {
            Value::Intermediate(ref key) => {
                if R::is_owned() {
                    db.unrootify(key)?;
                }
            }
            Value::End(_) => (),
        }

        self.root = update;
        Ok(())
    }

    /// Drop the current tree.
    pub fn drop(self, db: &mut DB) -> Result<(), Error<DB::Error>> {
        if R::is_owned() {
            if let Some(key) = self.root().intermediate() {
                db.unrootify(&key)?;
            }
        }
        Ok(())
    }
}

impl<R: RootStatus, DB: Backend> Leak for Raw<R, DB> {
    type Metadata = ValueOf<DB>;

    fn metadata(&self) -> Self::Metadata {
        self.root()
    }

    fn from_leaked(root: Self::Metadata) -> Self {
        Self {
            root,
            _marker: PhantomData,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::traits::Owned;
    use sha2::Sha256;

    type InMemory = crate::traits::InMemoryBackend<Sha256, Vec<u8>>;

    #[test]
    fn test_merkle_selections() {
        assert_eq!(Index::root().route(), IndexRoute::Root);
        assert_eq!(Index::root().left().route(),
                   IndexRoute::Select(vec![
                       IndexSelection::Left
                   ]));
        assert_eq!(Index::root().left().right().route(),
                   IndexRoute::Select(vec![
                       IndexSelection::Left,
                       IndexSelection::Right,
                   ]));
        assert_eq!(Index::root().right().left().left().route(),
                   IndexRoute::Select(vec![
                       IndexSelection::Right,
                       IndexSelection::Left,
                       IndexSelection::Left,
                   ]));
        assert_eq!(Index::root().left().left().right().left().route(),
                   IndexRoute::Select(vec![
                       IndexSelection::Left,
                       IndexSelection::Left,
                       IndexSelection::Right,
                       IndexSelection::Left,
                   ]));
        assert_eq!(Index::root().left().right().right().left().right().route(),
                   IndexRoute::Select(vec![
                       IndexSelection::Left,
                       IndexSelection::Right,
                       IndexSelection::Right,
                       IndexSelection::Left,
                       IndexSelection::Right,
                   ]));
    }

    #[test]
    fn test_selection_at() {
        assert_eq!(Index::root().right().route().at_depth(1), Some(IndexSelection::Right));
    }

    #[test]
    fn test_set_empty() {
        let mut db = InMemory::new_with_inherited_empty();
        let mut list = Raw::<Owned, InMemory>::default();

        let mut last_root = list.root();
        for _ in 0..3 {
            list.set(&mut db, Index::from_one(2).unwrap(), last_root.clone()).unwrap();
            list.set(&mut db, Index::from_one(3).unwrap(), last_root.clone()).unwrap();
            last_root = list.root();
        }
    }

    #[test]
    fn test_set_skip() {
        let mut db = InMemory::new_with_inherited_empty();
        let mut list = Raw::<Owned, InMemory>::default();

        list.set(&mut db, Index::from_one(4).unwrap(), Value::End(vec![2])).unwrap();
        assert_eq!(list.get(&db, Index::from_one(4).unwrap()).unwrap(), Some(Value::End(vec![2])));
        list.set(&mut db, Index::from_one(4).unwrap(), Value::End(vec![3])).unwrap();
        assert_eq!(list.get(&db, Index::from_one(4).unwrap()).unwrap(), Some(Value::End(vec![3])));
    }

    #[test]
    fn test_set_basic() {
        let mut db = InMemory::new_with_inherited_empty();
        let mut list = Raw::<Owned, InMemory>::default();

        for i in 4..8 {
            list.set(&mut db, Index::from_one(i).unwrap(), Value::End(vec![i as u8])).unwrap();
        }
    }

    #[test]
    fn test_set_only() {
        let mut db1 = InMemory::new_with_inherited_empty();
        let mut db2 = InMemory::new_with_inherited_empty();
        let mut list1 = Raw::<Owned, InMemory>::default();
        let mut list2 = Raw::<Owned, InMemory>::default();

        for i in 32..64 {
            list1.set(&mut db1, Index::from_one(i).unwrap(), Value::End(vec![i as u8])).unwrap();
        }
        for i in (32..64).rev() {
            list2.set(&mut db2, Index::from_one(i).unwrap(), Value::End(vec![i as u8])).unwrap();
        }
        assert_eq!(db1.as_ref(), db2.as_ref());
        for i in 32..64 {
            let val1 = list1.get(&mut db1, Index::from_one(i).unwrap()).unwrap().unwrap();
            let val2 = list2.get(&mut db2, Index::from_one(i).unwrap()).unwrap().unwrap();
            assert_eq!(val1, Value::End(vec![i as u8]));
            assert_eq!(val2, Value::End(vec![i as u8]));
        }

        list1.set(&mut db1, Index::from_one(1).unwrap(), Value::End(vec![1])).unwrap();
        assert!(db1.as_ref().is_empty());
    }

    #[test]
    fn test_intermediate() {
        let mut db = InMemory::new_with_inherited_empty();
        let mut list = Raw::<Owned, InMemory>::default();
        list.set(&mut db, Index::from_one(2).unwrap(), Value::End(vec![])).unwrap();
        assert_eq!(list.get(&mut db, Index::from_one(3).unwrap()).unwrap().unwrap(), Value::End(vec![]));

        let empty1 = list.get(&mut db, Index::from_one(1).unwrap()).unwrap().unwrap();
        list.set(&mut db, Index::from_one(2).unwrap(), empty1.clone()).unwrap();
        list.set(&mut db, Index::from_one(3).unwrap(), empty1.clone()).unwrap();
        for i in 4..8 {
            assert_eq!(list.get(&mut db, Index::from_one(i).unwrap()).unwrap().unwrap(), Value::End(vec![]));
        }
        assert_eq!(db.as_ref().len(), 2);

        let mut db1 = db.clone();
        let mut list1 = Raw::<Owned, InMemory>::from_leaked(list.root());
        list.set(&mut db, Index::from_one(1).unwrap(), empty1.clone()).unwrap();
        assert_eq!(list.get(&mut db, Index::from_one(3).unwrap()).unwrap().unwrap(), Value::End(vec![]));
        assert_eq!(db.as_ref().len(), 1);

        list1.set(&mut db1, Index::from_one(1).unwrap(), Value::End(vec![0])).unwrap();
        assert_eq!(list1.get(&mut db1, Index::from_one(1).unwrap()).unwrap().unwrap(), Value::End(vec![0]));
        assert!(db1.as_ref().is_empty());
    }
}
