use digest::Digest;

use crate::traits::{MerkleDB, Value, ValueOf};

/// Merkle selection.
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub enum MerkleSelection {
    /// Choose left at current depth.
    Left,
    /// Choose right at current depth.
    Right,
}

/// Merkle route.
#[derive(Clone, Eq, PartialEq, Debug)]
pub enum MerkleRoute {
    /// Root of the merkle tree.
    Root,
    /// Select items from the root.
    Select(Vec<MerkleSelection>),
}

impl MerkleRoute {
    /// Get selection at depth, where root is considered depth 0.
    pub fn at_depth(&self, depth: usize) -> Option<MerkleSelection> {
        match self {
            MerkleRoute::Root => None,
            MerkleRoute::Select(selections) => {
                if depth == 0 || depth > selections.len() {
                    None
                } else {
                    Some(selections[depth - 1])
                }
            },
        }
    }
}

/// Raw merkle index.
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub struct MerkleIndex(usize);

impl MerkleIndex {
    /// Root merkle index.
    pub const fn root() -> Self {
        Self(1)
    }

    /// Get left child of current index.
    pub const fn left(&self) -> Self {
        Self(2 * self.0)
    }

    /// Get right child of current index.
    pub const fn right(&self) -> Self {
        Self(2 * self.0 + 1)
    }

    /// Get the parent of current merkle index.
    pub fn parent(&self) -> Option<Self> {
        if self.0 == 1 {
            None
        } else {
            Some(Self(self.0 / 2))
        }
    }

    /// From one-based index.
    pub fn from_one(value: usize) -> Option<Self> {
        if value == 0 {
            None
        } else {
            Some(Self(value))
        }
    }

    /// From zero-based index.
    pub fn from_zero(value: usize) -> Self {
        Self(value + 1)
    }

    /// Get selections from current index.
    pub fn route(&self) -> MerkleRoute {
        let mut value = self.0;
        let mut selections = Vec::<MerkleSelection>::new();

        loop {
            if value >> 1 == 0 {
                debug_assert!(value == 1);

                if selections.is_empty() {
                    return MerkleRoute::Root
                } else {
                    selections.reverse();
                    return MerkleRoute::Select(selections)
                }
            }

            let sel = value & 0b1;
            if sel == 0 {
                selections.push(MerkleSelection::Left)
            } else {
                selections.push(MerkleSelection::Right)
            }

            value = value >> 1;
        }
    }
}

/// Raw merkle tree.
pub struct MerkleRaw<DB: MerkleDB> {
    root: ValueOf<DB>,
}

impl<DB: MerkleDB> Default for MerkleRaw<DB> {
    fn default() -> Self {
        Self::new()
    }
}

impl<DB: MerkleDB> MerkleRaw<DB> {
    /// Create a new raw tree.
    pub fn new() -> Self {
        Self {
            root: Value::End(Default::default()),
        }
    }

    /// Return the root of the tree.
    pub fn root(&self) -> ValueOf<DB> {
        self.root.clone()
    }

    /// Drop the current tree.
    pub fn drop(self, db: &mut DB) {
        self.root().intermediate().map(|key| {
            db.unrootify(&key);
        });
    }

    /// Leak this tree and return the root.
    pub fn leak(self) -> ValueOf<DB> {
        self.root()
    }

    /// Create from leaked value.
    pub fn from_leaked(root: ValueOf<DB>) -> Self {
        Self { root }
    }

    /// Get value from the tree via generalized merkle index.
    pub fn get(&self, db: &DB, index: MerkleIndex) -> Option<ValueOf<DB>> {
        match index.route() {
            MerkleRoute::Root => Some(self.root.clone()),
            MerkleRoute::Select(selections) => {
                let mut current = self.root.clone();

                for selection in selections {
                    let intermediate = match current {
                        Value::Intermediate(intermediate) => intermediate,
                        Value::End(_) => return None,
                    };

                    current = match db.get(&intermediate) {
                        Some(pair) => {
                            match selection {
                                MerkleSelection::Left => pair.0.clone(),
                                MerkleSelection::Right => pair.1.clone(),
                            }
                        },
                        None => return None,
                    };
                }

                Some(current)
            },
        }
    }

    /// Set value of the merkle tree via generalized merkle index.
    pub fn set(&mut self, db: &mut DB, index: MerkleIndex, set: ValueOf<DB>) {
        let route = index.route();

        match set.clone() {
            Value::End(_) => (),
            Value::Intermediate(key) => {
                let value = db.get(&key).expect("Intermediate to set does not exist");
                db.insert(key, value);
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
                                Value::Intermediate(key) => { db.rootify(key); }
                            }
                            self.root = set;
                            return
                        },
                    };
                    values.push((sel, (Value::End(Default::default()), Value::End(Default::default()))));
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
                        let value = match db.get(&cur) {
                            Some(value) => value.clone(),
                            None => (Value::End(Default::default()), Value::End(Default::default())),
                        };
                        values.push((sel, value.clone()));
                        current = match sel {
                            MerkleSelection::Left => value.0.intermediate(),
                            MerkleSelection::Right => value.1.intermediate(),
                        };
                    },
                    None => {
                        values.push((sel, (Value::End(Default::default()), Value::End(Default::default()))));
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
                MerkleSelection::Left => { value.0 = update.clone(); }
                MerkleSelection::Right => { value.1 = update.clone(); }
            }

            let intermediate = {
                let mut digest = <DB::Digest as Digest>::new();
                digest.input(&value.0.as_ref()[..]);
                digest.input(&value.1.as_ref()[..]);
                digest.result()
            };

            db.insert(intermediate.clone(), value);
            update = Value::Intermediate(intermediate);
        }

        match &update {
            Value::Intermediate(ref key) => { db.rootify(key); }
            Value::End(_) => (),
        }
        match &self.root {
            Value::Intermediate(ref key) => { db.unrootify(key); }
            Value::End(_) => (),
        }

        self.root = update;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::Sha256;

    type InMemory = crate::traits::InMemoryMerkleDB<Sha256, Vec<u8>>;

    #[test]
    fn test_merkle_selections() {
        assert_eq!(MerkleIndex::root().route(), MerkleRoute::Root);
        assert_eq!(MerkleIndex::root().left().route(),
                   MerkleRoute::Select(vec![
                       MerkleSelection::Left
                   ]));
        assert_eq!(MerkleIndex::root().left().right().route(),
                   MerkleRoute::Select(vec![
                       MerkleSelection::Left,
                       MerkleSelection::Right,
                   ]));
        assert_eq!(MerkleIndex::root().right().left().left().route(),
                   MerkleRoute::Select(vec![
                       MerkleSelection::Right,
                       MerkleSelection::Left,
                       MerkleSelection::Left,
                   ]));
        assert_eq!(MerkleIndex::root().left().left().right().left().route(),
                   MerkleRoute::Select(vec![
                       MerkleSelection::Left,
                       MerkleSelection::Left,
                       MerkleSelection::Right,
                       MerkleSelection::Left,
                   ]));
        assert_eq!(MerkleIndex::root().left().right().right().left().right().route(),
                   MerkleRoute::Select(vec![
                       MerkleSelection::Left,
                       MerkleSelection::Right,
                       MerkleSelection::Right,
                       MerkleSelection::Left,
                       MerkleSelection::Right,
                   ]));
    }

    #[test]
    fn test_selection_at() {
        fn selection_at(index: MerkleIndex, depth: u32) -> Option<usize> {
            let mut index = index.0;
            if index < 2_usize.pow(depth) {
                return None
            }

            while index > 2_usize.pow(depth + 1) {
                index = index / 2;
            }
            Some(index % 2)
        }

        assert_eq!(MerkleIndex::root().right().route().at_depth(1), Some(MerkleSelection::Right));
        assert_eq!(selection_at(MerkleIndex::root().right(), 1), Some(1));
    }

    #[test]
    fn test_set_empty() {
        let mut db = InMemory::default();
        let mut list = MerkleRaw::<InMemory>::new();

        let mut last_root = list.root();
        for _ in 0..3 {
            list.set(&mut db, MerkleIndex::from_one(2).unwrap(), last_root.clone());
            list.set(&mut db, MerkleIndex::from_one(3).unwrap(), last_root.clone());
            last_root = list.root();
        }
    }

    #[test]
    fn test_set_skip() {
        let mut db = InMemory::default();
        let mut list = MerkleRaw::<InMemory>::new();

        list.set(&mut db, MerkleIndex::from_one(4).unwrap(), Value::End(vec![2]));
        assert_eq!(list.get(&db, MerkleIndex::from_one(4).unwrap()), Some(Value::End(vec![2])));
        list.set(&mut db, MerkleIndex::from_one(4).unwrap(), Value::End(vec![3]));
        assert_eq!(list.get(&db, MerkleIndex::from_one(4).unwrap()), Some(Value::End(vec![3])));
    }

    #[test]
    fn test_set_basic() {
        let mut db = InMemory::default();
        let mut list = MerkleRaw::<InMemory>::new();

        for i in 4..8 {
            list.set(&mut db, MerkleIndex::from_one(i).unwrap(), Value::End(vec![i as u8]));
        }
    }

    #[test]
    fn test_set_only() {
        let mut db1 = InMemory::default();
        let mut db2 = InMemory::default();
        let mut list1 = MerkleRaw::<InMemory>::new();
        let mut list2 = MerkleRaw::<InMemory>::new();

        for i in 32..64 {
            list1.set(&mut db1, MerkleIndex::from_one(i).unwrap(), Value::End(vec![i as u8]));
        }
        for i in (32..64).rev() {
            list2.set(&mut db2, MerkleIndex::from_one(i).unwrap(), Value::End(vec![i as u8]));
        }
        assert_eq!(db1.as_ref(), db2.as_ref());
        for i in 32..64 {
            let val1 = list1.get(&mut db1, MerkleIndex::from_one(i).unwrap()).unwrap();
            let val2 = list2.get(&mut db2, MerkleIndex::from_one(i).unwrap()).unwrap();
            assert_eq!(val1, Value::End(vec![i as u8]));
            assert_eq!(val2, Value::End(vec![i as u8]));
        }

        list1.set(&mut db1, MerkleIndex::from_one(1).unwrap(), Value::End(vec![1]));
        assert!(db1.as_ref().is_empty());
    }

    #[test]
    fn test_intermediate() {
        let mut db = InMemory::default();
        let mut list = MerkleRaw::<InMemory>::new();
        list.set(&mut db, MerkleIndex::from_one(2).unwrap(), Value::End(vec![]));
        assert_eq!(list.get(&mut db, MerkleIndex::from_one(3).unwrap()).unwrap(), Value::End(vec![]));

        let empty1 = list.get(&mut db, MerkleIndex::from_one(1).unwrap()).unwrap();
        list.set(&mut db, MerkleIndex::from_one(2).unwrap(), empty1.clone());
        list.set(&mut db, MerkleIndex::from_one(3).unwrap(), empty1.clone());
        for i in 4..8 {
            assert_eq!(list.get(&mut db, MerkleIndex::from_one(i).unwrap()).unwrap(), Value::End(vec![]));
        }
        assert_eq!(db.as_ref().len(), 2);

        let mut db1 = db.clone();
        let mut list1 = MerkleRaw::<InMemory>::from_leaked(list.root());
        list.set(&mut db, MerkleIndex::from_one(1).unwrap(), empty1.clone());
        assert_eq!(list.get(&mut db, MerkleIndex::from_one(3).unwrap()).unwrap(), Value::End(vec![]));
        assert_eq!(db.as_ref().len(), 1);

        list1.set(&mut db1, MerkleIndex::from_one(1).unwrap(), Value::End(vec![0]));
        assert_eq!(list1.get(&mut db1, MerkleIndex::from_one(1).unwrap()).unwrap(), Value::End(vec![0]));
        assert!(db1.as_ref().is_empty());
    }
}
