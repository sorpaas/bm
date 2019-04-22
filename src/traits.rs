use digest::Digest;
use generic_array::GenericArray;

use std::collections::HashMap;

#[derive(Clone, Eq, PartialEq, Debug)]
pub enum Value<I, E> {
    Intermediate(I),
    End(E),
}

impl<I, E> Value<I, E> {
    pub fn intermediate(self) -> Option<I> {
        match self {
            Value::Intermediate(intermediate) => Some(intermediate),
            Value::End(_) => None,
        }
    }

    pub fn end(self) -> Option<E> {
        match self {
            Value::Intermediate(_) => None,
            Value::End(end) => Some(end),
        }
    }
}

impl<I: AsRef<[u8]>, E: AsRef<[u8]>> AsRef<[u8]> for Value<I, E> {
    fn as_ref(&self) -> &[u8] {
        match self {
            Value::Intermediate(ref intermediate) => intermediate.as_ref(),
            Value::End(ref end) => end.as_ref(),
        }
    }
}

pub type IntermediateOf<DB> = GenericArray<u8, <<DB as RawListDB>::Digest as Digest>::OutputSize>;
pub type EndOf<DB> = <DB as RawListDB>::Value;
pub type ValueOf<DB> = Value<IntermediateOf<DB>, EndOf<DB>>;

pub trait RawListDB: Default {
    type Digest: Digest;
    type Value: AsRef<[u8]> + Clone;

    fn get(&self, key: &IntermediateOf<Self>) -> Option<(ValueOf<Self>, ValueOf<Self>)>;
    fn rootify(&mut self, key: &IntermediateOf<Self>);
    fn unrootify(&mut self, key: &IntermediateOf<Self>);
    fn insert(&mut self, key: IntermediateOf<Self>, value: (ValueOf<Self>, ValueOf<Self>));
}

#[derive(Clone)]
pub struct InMemoryRawListDB<D: Digest, T: AsRef<[u8]> + core::fmt::Debug + Clone>(
    HashMap<IntermediateOf<Self>, ((ValueOf<Self>, ValueOf<Self>), usize)>
);

impl<D: Digest, T: AsRef<[u8]> + core::fmt::Debug + Clone> InMemoryRawListDB<D, T> {
    fn remove(&mut self, old_key: &IntermediateOf<Self>) {
        let (old_value, to_remove) = {
            let value = self.0.get_mut(old_key).expect("Set key does not exist");
            value.1 -= 1;
            (value.0.clone(), value.1 == 0)
        };

        if to_remove {
            match old_value.0 {
                Value::Intermediate(subkey) => { self.remove(&subkey); },
                Value::End(_) => (),
            }

            match old_value.1 {
                Value::Intermediate(subkey) => { self.remove(&subkey); },
                Value::End(_) => (),
            }

            self.0.remove(old_key);
        }
    }
}

impl<D: Digest, T: AsRef<[u8]> + core::fmt::Debug + Clone> Default for InMemoryRawListDB<D, T> {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<D: Digest, T: AsRef<[u8]> + core::fmt::Debug + Clone> AsRef<HashMap<IntermediateOf<Self>, ((ValueOf<Self>, ValueOf<Self>), usize)>> for InMemoryRawListDB<D, T> {
    fn as_ref(&self) -> &HashMap<IntermediateOf<Self>, ((ValueOf<Self>, ValueOf<Self>), usize)> {
        &self.0
    }
}

impl<D: Digest, T: AsRef<[u8]> + core::fmt::Debug + Clone> RawListDB for InMemoryRawListDB<D, T> {
    type Digest = D;
    type Value = T;

    fn get(&self, key: &GenericArray<u8, D::OutputSize>) -> Option<(ValueOf<Self>, ValueOf<Self>)> {
        self.0.get(key).map(|v| v.0.clone())
    }

    fn rootify(&mut self, key: &IntermediateOf<Self>) {
        self.0.get_mut(key).expect("Trying to rootify a non-existing key").1 += 1;
    }

    fn unrootify(&mut self, key: &IntermediateOf<Self>) {
        self.remove(key);
    }

    fn insert(&mut self, key: IntermediateOf<Self>, value: (ValueOf<Self>, ValueOf<Self>)) {
        if self.0.contains_key(&key) {
            return
        }

        let (left, right) = value;

        match &left {
            Value::Intermediate(ref subkey) => {
                self.0.get_mut(subkey).expect("Set subkey does not exist").1 += 1;
            },
            Value::End(_) => (),
        }
        match &right {
            Value::Intermediate(ref subkey) => {
                self.0.get_mut(subkey).expect("Set subkey does not exist").1 += 1;
            },
            Value::End(_) => (),
        }

        self.0.insert(key, ((left, right), 0));
    }
}
