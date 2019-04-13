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

pub struct RawList<D: Digest> {
    db: HashMap<GenericArray<u8, D::OutputSize>, (Value<D>, Value<D>)>,
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

    pub fn set_end(&mut self, index: NonZeroUsize, set: Vec<u8>) {
        let mut values = {
            let mut values = Vec::new();
            let mut depth = 1;
            let mut current = match self.root.clone() {
                Value::Intermediate(intermediate) => Some(intermediate),
                Value::End(_) => {
                    if selection_at(index, depth).is_none() {
                        self.root = Value::End(set);
                        return
                    } else {
                        values.push((0, (Value::End(self.default_value.clone()), Value::End(self.default_value.clone()))));
                        depth += 1;
                        None
                    }
                },
            };

            loop {
                let sel = match selection_at(index, depth) {
                    Some(sel) => sel,
                    None => break,
                };
                match current.clone() {
                    Some(cur) => {
                        let value = match self.db.get(&cur) {
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

        let mut update = Value::End(set);
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
            self.db.insert(intermediate.clone(), value);
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
        let mut list = RawList::<Sha256>::new();

        for i in 1..16 {
            list.set_end(NonZeroUsize::new(i).unwrap(), vec![i as u8]);
        }
        for i in 8..16 {
            let val = list.get(NonZeroUsize::new(i).unwrap()).unwrap();
            assert_eq!(val, Value::End(vec![i as u8]));
        }
    }
}
