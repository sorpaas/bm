use digest::Digest;
use generic_array::GenericArray;

use std::collections::HashMap;

#[derive(Clone, Eq, PartialEq)]
pub enum Value<I, E> {
    Intermediate(I),
    End(E),
}

pub type IntermediateOf<DB> = GenericArray<u8, <<DB as RawListDB>::Digest as Digest>::OutputSize>;
pub type EndOf<DB> = <DB as RawListDB>::Value;
pub type ValueOf<DB> = Value<IntermediateOf<DB>, EndOf<DB>>;

pub trait RawListDB {
    type Digest: Digest;
    type Value;

    fn get(&self, key: &IntermediateOf<Self>) -> Option<(ValueOf<Self>, ValueOf<Self>)>;
    fn insert(&mut self, key: IntermediateOf<Self>, value: (ValueOf<Self>, ValueOf<Self>));
    fn remove(&mut self, key: &IntermediateOf<Self>) -> Option<(ValueOf<Self>, ValueOf<Self>)>;
}

pub struct InMemoryRawListDB<D: Digest, T: Clone>(
    HashMap<IntermediateOf<Self>, ((ValueOf<Self>, ValueOf<Self>), usize)>
);

impl<D: Digest, T: Clone> RawListDB for InMemoryRawListDB<D, T> {
    type Digest = D;
    type Value = T;

    fn get(&self, key: &GenericArray<u8, D::OutputSize>) -> Option<(ValueOf<Self>, ValueOf<Self>)> {
        self.0.get(key).map(|v| v.0.clone())
    }

    fn insert(&mut self, key: GenericArray<u8, D::OutputSize>, value: (ValueOf<Self>, ValueOf<Self>)) {
        self.0.entry(key)
            .and_modify(|value| value.1 += 1)
            .or_insert((value, 1));
    }

    fn remove(&mut self, key: &GenericArray<u8, D::OutputSize>) -> Option<(ValueOf<Self>, ValueOf<Self>)> {
        let (to_remove, value) = self.0.get_mut(key)
            .map(|value| {
                value.1 -= 1;
                (value.1 == 0, Some(value.0.clone()))
            })
            .unwrap_or((false, None));

        if to_remove {
            self.0.remove(key);
        }

        value
    }
}
