use core::num::NonZeroUsize;
use digest::Digest;

use crate::traits::{RawListDB, Value, IntermediateOf, EndOf, ValueOf, ReplaceValue};

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
    default_value: EndOf<DB>,
    root: ValueOf<DB>,
}

impl<DB: RawListDB> Default for RawList<DB> where
    EndOf<DB>: Default
{
    fn default() -> Self {
        Self::new()
    }
}

impl<DB: RawListDB> RawList<DB> {
    pub fn new_with_default(default_value: EndOf<DB>) -> Self {
        Self {
            root: Value::End(default_value.clone()),
            default_value,
        }
    }

    pub fn new() -> Self where
        EndOf<DB>: Default
    {
        Self::new_with_default(Default::default())
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
        let old = match self.get(db, index) {
            None | Some(Value::End(_)) => ReplaceValue::EndOrNone,
            Some(Value::Intermediate(key)) => {
                let value = db.get(&key).expect("Local database is invalid");
                ReplaceValue::Intermediate((key, value))
            },
        };
        let new = match set.clone() {
            Value::End(_) => ReplaceValue::EndOrNone,
            Value::Intermediate(key) => {
                let value = db.get(&key).expect("Intermediate to set does not exist");
                ReplaceValue::Intermediate((key, value))
            },
        };
        db.replace(old, new);

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
                            self.root = set;
                            return
                        },
                    };
                    values.push((sel, (Value::End(self.default_value.clone()), Value::End(self.default_value.clone()))));
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
                            None => (Value::End(self.default_value.clone()), Value::End(self.default_value.clone())),
                        };
                        values.push((sel, value.clone()));
                        current = if sel == 0 {
                            value.0.intermediate()
                        } else {
                            value.1.intermediate()
                        };
                    },
                    None => {
                        values.push((sel, (Value::End(self.default_value.clone()), Value::End(self.default_value.clone()))));
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

            let old_value = match values.last() {
                Some((sel, ref v)) => {
                    if *sel == 0 {
                        v.0.clone()
                    } else {
                        v.1.clone()
                    }
                },
                None => self.root.clone(),
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

            let old = match old_value {
                Value::End(_) => ReplaceValue::EndOrNone,
                Value::Intermediate(key) => {
                    let value = db.get(&key).expect("Intermediate to set does not exist");
                    ReplaceValue::Intermediate((key, value))
                },
            };
            let new = ReplaceValue::Intermediate((intermediate.clone(), value));
            db.replace(old, new);

            update = Value::Intermediate(intermediate);
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
    fn test_set_skip() {
        let mut db = InMemory::default();
        let mut list = RawList::<InMemory>::new();

        list.set(&mut db, NonZeroUsize::new(4).unwrap(), Value::End(vec![2]));
        assert_eq!(list.get(&db, NonZeroUsize::new(4).unwrap()), Some(Value::End(vec![2])));
        list.set(&mut db, NonZeroUsize::new(4).unwrap(), Value::End(vec![3]));
        assert_eq!(list.get(&db, NonZeroUsize::new(4).unwrap()), Some(Value::End(vec![3])));
    }

    #[test]
    fn test_set() {
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
        let mut list = RawList::<InMemory>::new_with_default(vec![0]);
        list.set(&mut db, NonZeroUsize::new(2).unwrap(), Value::End(vec![0]));
        assert_eq!(list.get(&mut db, NonZeroUsize::new(3).unwrap()).unwrap(), Value::End(vec![0]));

        let empty1 = list.get(&mut db, NonZeroUsize::new(1).unwrap()).unwrap();
        list.set(&mut db, NonZeroUsize::new(2).unwrap(), empty1.clone());
        list.set(&mut db, NonZeroUsize::new(3).unwrap(), empty1.clone());
        for i in 4..8 {
            assert_eq!(list.get(&mut db, NonZeroUsize::new(i).unwrap()).unwrap(), Value::End(vec![0]));
        }
        assert_eq!(db.as_ref().len(), 2);

        let mut db1 = db.clone();
        let mut list1 = list.clone();
        list.set(&mut db, NonZeroUsize::new(1).unwrap(), empty1.clone());
        assert_eq!(list.get(&mut db, NonZeroUsize::new(3).unwrap()).unwrap(), Value::End(vec![0]));
        assert_eq!(db.as_ref().len(), 1);

        list1.set(&mut db1, NonZeroUsize::new(1).unwrap(), Value::End(vec![0]));
        assert_eq!(list1.get(&mut db1, NonZeroUsize::new(1).unwrap()).unwrap(), Value::End(vec![0]));
        assert!(db1.as_ref().is_empty());
    }
}
