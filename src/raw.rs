use core::num::NonZeroUsize;
use digest::Digest;

use crate::traits::{RawListDB, Value, ValueOf};

fn selection_at(index: NonZeroUsize, depth: u32) -> Option<usize> {
    let mut index = index.get();
    if index < 2_usize.pow(depth) {
        return None
    }

    while index > 2_usize.pow(depth + 1) {
        index = index / 2;
    }
    Some(index % 2)
}

#[derive(Clone)]
pub struct RawList<DB: RawListDB> {
    root: ValueOf<DB>,
}

impl<DB: RawListDB> Default for RawList<DB> {
    fn default() -> Self {
        Self::new()
    }
}

impl<DB: RawListDB> RawList<DB> {
    pub fn new() -> Self {
        Self {
            root: Value::End(Default::default()),
        }
    }

    pub fn root(&self) -> ValueOf<DB> {
        self.root.clone()
    }

    pub fn get(&self, db: &DB, index: NonZeroUsize) -> Option<ValueOf<DB>> {
        let mut current = match self.root.clone() {
            Value::Intermediate(intermediate) => intermediate,
            Value::End(value) => {
                if index.get() == 1 {
                    return Some(Value::End(value))
                } else {
                    return None
                }
            },
        };
        let mut depth = 1;
        loop {
            let sel = match selection_at(index, depth) {
                Some(sel) => sel,
                None => break,
            };
            current = {
                let value = match db.get(&current) {
                    Some(pair) => {
                        if sel == 0 {
                            pair.0.clone()
                        } else {
                            pair.1.clone()
                        }
                    },
                    None => return None,
                };

                match value {
                    Value::Intermediate(intermediate) => intermediate,
                    Value::End(value) => {
                        if selection_at(index, depth + 1).is_none() {
                            return Some(Value::End(value))
                        } else {
                            return None
                        }
                    },
                }
            };
            depth += 1;
        }

        Some(Value::Intermediate(current))
    }

    pub fn set(&mut self, db: &mut DB, index: NonZeroUsize, set: ValueOf<DB>) {
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
                    let sel = match selection_at(index, depth) {
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
                let sel = match selection_at(index, depth) {
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
                        current = if sel == 0 {
                            value.0.intermediate()
                        } else {
                            value.1.intermediate()
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

            if sel == 0 {
                value.0 = update.clone();
            } else {
                value.1 = update.clone();
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

    type InMemory = crate::traits::InMemoryRawListDB<Sha256, Vec<u8>>;

    #[test]
    fn test_set_empty() {
        let mut db = InMemory::default();
        let mut list = RawList::<InMemory>::new();

        let mut last_root = list.root();
        for _ in 0..3 {
            list.set(&mut db, NonZeroUsize::new(2).unwrap(), last_root.clone());
            list.set(&mut db, NonZeroUsize::new(3).unwrap(), last_root.clone());
            last_root = list.root();
        }
    }

    #[test]
    fn test_set_skip() {
        let mut db = InMemory::default();
        let mut list = RawList::<InMemory>::new();

        list.set(&mut db, NonZeroUsize::new(4).unwrap(), Value::End(vec![2]));
        assert_eq!(list.get(&db, NonZeroUsize::new(4).unwrap()), Some(Value::End(vec![2])));
        list.set(&mut db, NonZeroUsize::new(4).unwrap(), Value::End(vec![3]));
        assert_eq!(list.get(&db, NonZeroUsize::new(4).unwrap()), Some(Value::End(vec![3])));
    }

    #[test]
    fn test_set_basic() {
        let mut db = InMemory::default();
        let mut list = RawList::<InMemory>::new();

        for i in 4..8 {
            list.set(&mut db, NonZeroUsize::new(i).unwrap(), Value::End(vec![i as u8]));
        }
    }

    #[test]
    fn test_set_only() {
        let mut db1 = InMemory::default();
        let mut db2 = InMemory::default();
        let mut list1 = RawList::<InMemory>::new();
        let mut list2 = RawList::<InMemory>::new();

        for i in 32..64 {
            list1.set(&mut db1, NonZeroUsize::new(i).unwrap(), Value::End(vec![i as u8]));
        }
        for i in (32..64).rev() {
            list2.set(&mut db2, NonZeroUsize::new(i).unwrap(), Value::End(vec![i as u8]));
        }
        assert_eq!(db1.as_ref(), db2.as_ref());
        for i in 32..64 {
            let val1 = list1.get(&mut db1, NonZeroUsize::new(i).unwrap()).unwrap();
            let val2 = list2.get(&mut db2, NonZeroUsize::new(i).unwrap()).unwrap();
            assert_eq!(val1, Value::End(vec![i as u8]));
            assert_eq!(val2, Value::End(vec![i as u8]));
        }

        list1.set(&mut db1, NonZeroUsize::new(1).unwrap(), Value::End(vec![1]));
        assert!(db1.as_ref().is_empty());
    }

    #[test]
    fn test_intermediate() {
        let mut db = InMemory::default();
        let mut list = RawList::<InMemory>::new();
        list.set(&mut db, NonZeroUsize::new(2).unwrap(), Value::End(vec![]));
        assert_eq!(list.get(&mut db, NonZeroUsize::new(3).unwrap()).unwrap(), Value::End(vec![]));

        let empty1 = list.get(&mut db, NonZeroUsize::new(1).unwrap()).unwrap();
        list.set(&mut db, NonZeroUsize::new(2).unwrap(), empty1.clone());
        list.set(&mut db, NonZeroUsize::new(3).unwrap(), empty1.clone());
        for i in 4..8 {
            assert_eq!(list.get(&mut db, NonZeroUsize::new(i).unwrap()).unwrap(), Value::End(vec![]));
        }
        assert_eq!(db.as_ref().len(), 2);

        let mut db1 = db.clone();
        let mut list1 = list.clone();
        list.set(&mut db, NonZeroUsize::new(1).unwrap(), empty1.clone());
        assert_eq!(list.get(&mut db, NonZeroUsize::new(3).unwrap()).unwrap(), Value::End(vec![]));
        assert_eq!(db.as_ref().len(), 1);

        list1.set(&mut db1, NonZeroUsize::new(1).unwrap(), Value::End(vec![0]));
        assert_eq!(list1.get(&mut db1, NonZeroUsize::new(1).unwrap()).unwrap(), Value::End(vec![0]));
        assert!(db1.as_ref().is_empty());
    }
}
