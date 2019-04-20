use core::num::NonZeroUsize;
use digest::Digest;

use crate::traits::{RawListDB, Value, IntermediateOf, EndOf, ValueOf};

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
    db: DB,
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
    fn remove_one(&mut self, intermediate: &IntermediateOf<DB>, remove_child: bool) -> Option<(ValueOf<DB>, ValueOf<DB>)> {
        let value = self.db.remove(intermediate);

        if remove_child {
            value.as_ref().map(|(left, right)| {
                match left {
                    Value::Intermediate(ref intermediate) => { self.remove_one(intermediate, true); },
                    Value::End(_) => (),
                }
                match right {
                    Value::Intermediate(ref intermediate) => { self.remove_one(intermediate, true); },
                    Value::End(_) => (),
                }
            });
        }

        value
    }

    fn insert_one(&mut self, intermediate: IntermediateOf<DB>, value: (ValueOf<DB>, ValueOf<DB>)) {
        self.db.insert(intermediate, value);
    }


    pub fn new_with_default(default_value: EndOf<DB>) -> Self {
        Self {
            root: Value::End(default_value.clone()),
            db: Default::default(),
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

    pub fn get(&self, index: NonZeroUsize) -> Option<ValueOf<DB>> {
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
                let value = match self.db.get(&current) {
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

    pub fn set(&mut self, index: NonZeroUsize, set: ValueOf<DB>) {
        match &set {
            Value::Intermediate(ref intermediate) => {
                let value = match self.db.get(intermediate) {
                    Some(value) => value.clone(),
                    None => panic!("Intermediate value to set does not exist"),
                };
                self.insert_one(intermediate.clone(), value);
            },
            Value::End(_) => ()
        }

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
                            match self.root.clone() {
                                Value::Intermediate(intermediate) => {
                                    self.remove_one(&intermediate, true);
                                },
                                Value::End(_) => (),
                            }

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
                    None => {
                        current.map(|cur| self.remove_one(&cur, true));
                        break
                    },
                };
                match current.clone() {
                    Some(cur) => {
                        let value = match self.remove_one(&cur, false) {
                            Some(value) => value.clone(),
                            None => (Value::End(self.default_value.clone()), Value::End(self.default_value.clone())),
                        };
                        values.push((sel, value.clone()));
                        current = if sel == 0 {
                            match value.0 {
                                Value::Intermediate(intermediate) => Some(intermediate),
                                Value::End(_) => None,
                            }
                        } else {
                            match value.1 {
                                Value::Intermediate(intermediate) => Some(intermediate),
                                Value::End(_) => None,
                            }
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

            if sel == 0 {
                value.0 = update;
            } else {
                value.1 = update;
            }
            let intermediate = {
                let mut digest = <DB::Digest as Digest>::new();
                digest.input(&value.0.as_ref()[..]);
                digest.input(&value.1.as_ref()[..]);
                digest.result()
            };
            self.insert_one(intermediate.clone(), value);
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
    fn test_set() {
        let mut list1 = RawList::<InMemory>::new();
        let mut list2 = RawList::<InMemory>::new();

        for i in 32..64 {
            list1.set(NonZeroUsize::new(i).unwrap(), Value::End(vec![i as u8]));
        }
        for i in (32..64).rev() {
            list2.set(NonZeroUsize::new(i).unwrap(), Value::End(vec![i as u8]));
        }
        assert_eq!(list1.db.as_ref(), list2.db.as_ref());
        for i in 32..64 {
            let val1 = list1.get(NonZeroUsize::new(i).unwrap()).unwrap();
            let val2 = list2.get(NonZeroUsize::new(i).unwrap()).unwrap();
            assert_eq!(val1, Value::End(vec![i as u8]));
            assert_eq!(val2, Value::End(vec![i as u8]));
        }

        list1.set(NonZeroUsize::new(1).unwrap(), Value::End(vec![1]));
        assert!(list1.db.as_ref().is_empty());
    }

    #[test]
    fn test_intermediate() {
        let mut list = RawList::<InMemory>::new_with_default(vec![0]);
        list.set(NonZeroUsize::new(2).unwrap(), Value::End(vec![0]));
        assert_eq!(list.get(NonZeroUsize::new(3).unwrap()).unwrap(), Value::End(vec![0]));

        let empty1 = list.get(NonZeroUsize::new(1).unwrap()).unwrap();
        list.set(NonZeroUsize::new(2).unwrap(), empty1.clone());
        list.set(NonZeroUsize::new(3).unwrap(), empty1.clone());
        for i in 4..8 {
            assert_eq!(list.get(NonZeroUsize::new(i).unwrap()).unwrap(), Value::End(vec![0]));
        }
        assert_eq!(list.db.as_ref().len(), 2);

        let mut list1 = list.clone();
        list.set(NonZeroUsize::new(1).unwrap(), empty1.clone());
        assert_eq!(list.get(NonZeroUsize::new(3).unwrap()).unwrap(), Value::End(vec![0]));
        assert_eq!(list.db.as_ref().len(), 1);

        list1.set(NonZeroUsize::new(1).unwrap(), Value::End(vec![0]));
        assert_eq!(list1.get(NonZeroUsize::new(1).unwrap()).unwrap(), Value::End(vec![0]));
        assert!(list1.db.as_ref().is_empty());
    }
}
