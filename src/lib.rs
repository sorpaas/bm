mod traits;

use digest::Digest;
use generic_array::GenericArray;

use std::collections::HashMap;
use core::num::NonZeroUsize;

#[derive(Eq, Debug)]
pub enum Value<D: Digest> {
    Intermediate(GenericArray<u8, D::OutputSize>),
    End(Vec<u8>),
}

impl<D: Digest> AsRef<[u8]> for Value<D> {
    fn as_ref(&self) -> &[u8] {
        match self {
            Value::Intermediate(ref intermediate) => intermediate.as_ref(),
            Value::End(ref end) => end.as_ref(),
        }
    }
}

impl<D: Digest> PartialEq for Value<D> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Intermediate(s1), Value::Intermediate(s2)) => s1 == s2,
            (Value::End(s1), Value::End(s2)) => s1 == s2,
            _ => false,
        }
    }
}

impl<D: Digest> Clone for Value<D> {
    fn clone(&self) -> Self {
        match self {
            Value::Intermediate(intermediate) => Value::Intermediate(intermediate.clone()),
            Value::End(end) => Value::End(end.clone()),
        }
    }
}

#[derive(Clone, Debug)]
pub struct RawList<D: Digest> {
    db: HashMap<GenericArray<u8, D::OutputSize>, ((Value<D>, Value<D>), usize)>,
    default_value: Vec<u8>,
    root: Value<D>,
}

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

impl<D: Digest> RawList<D> {
    fn remove_one(&mut self, intermediate: &GenericArray<u8, D::OutputSize>, remove_child: bool) -> Option<(Value<D>, Value<D>)> {
        let (to_remove, value) = self.db.get_mut(intermediate)
            .map(|value| {
                value.1 -= 1;
                (value.1 == 0, Some(value.0.clone()))
            })
            .unwrap_or((false, None));
        if to_remove {
            self.db.remove(intermediate);

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
        }
        value
    }

    fn insert_one(&mut self, intermediate: GenericArray<u8, D::OutputSize>, value: (Value<D>, Value<D>)) {
        self.db.entry(intermediate)
            .and_modify(|value| value.1 += 1)
            .or_insert((value, 1));
    }

    pub fn new_with_default(default_value: Vec<u8>) -> Self {
        Self {
            root: Value::End(default_value.clone()),
            db: Default::default(),
            default_value,
        }
    }

    pub fn new() -> Self {
        Self::new_with_default(Default::default())
    }

    pub fn get(&self, index: NonZeroUsize) -> Option<Value<D>> {
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
                            (pair.0).0.clone()
                        } else {
                            (pair.0).1.clone()
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

    pub fn set(&mut self, index: NonZeroUsize, set: Value<D>) {
        match &set {
            Value::Intermediate(ref intermediate) => {
                let value = match self.db.get(intermediate) {
                    Some(value) => value.0.clone(),
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
                let mut digest = D::new();
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

    #[test]
    fn test_set() {
        let mut list1 = RawList::<Sha256>::new();
        let mut list2 = RawList::<Sha256>::new();

        for i in 32..64 {
            list1.set(NonZeroUsize::new(i).unwrap(), Value::End(vec![i as u8]));
        }
        for i in (32..64).rev() {
            list2.set(NonZeroUsize::new(i).unwrap(), Value::End(vec![i as u8]));
        }
        assert_eq!(list1.db, list2.db);
        for i in 32..64 {
            let val1 = list1.get(NonZeroUsize::new(i).unwrap()).unwrap();
            let val2 = list2.get(NonZeroUsize::new(i).unwrap()).unwrap();
            assert_eq!(val1, Value::End(vec![i as u8]));
            assert_eq!(val2, Value::End(vec![i as u8]));
        }

        list1.set(NonZeroUsize::new(1).unwrap(), Value::End(vec![1]));
        assert!(list1.db.is_empty());
    }

    #[test]
    fn test_intermediate() {
        let mut list = RawList::<Sha256>::new_with_default(vec![0]);
        list.set(NonZeroUsize::new(2).unwrap(), Value::End(vec![0]));
        assert_eq!(list.get(NonZeroUsize::new(3).unwrap()).unwrap(), Value::End(vec![0]));

        let empty1 = list.get(NonZeroUsize::new(1).unwrap()).unwrap();
        list.set(NonZeroUsize::new(2).unwrap(), empty1.clone());
        list.set(NonZeroUsize::new(3).unwrap(), empty1.clone());
        for i in 4..8 {
            assert_eq!(list.get(NonZeroUsize::new(i).unwrap()).unwrap(), Value::End(vec![0]));
        }
        assert_eq!(list.db.len(), 2);

        let mut list1 = list.clone();
        list.set(NonZeroUsize::new(1).unwrap(), empty1.clone());
        assert_eq!(list.get(NonZeroUsize::new(3).unwrap()).unwrap(), Value::End(vec![0]));
        assert_eq!(list.db.len(), 1);

        list1.set(NonZeroUsize::new(1).unwrap(), Value::End(vec![0]));
        assert_eq!(list1.get(NonZeroUsize::new(1).unwrap()).unwrap(), Value::End(vec![0]));
        assert!(list1.db.is_empty());
    }
}
