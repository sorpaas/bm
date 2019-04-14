use digest::Digest;
use generic_array::GenericArray;

use std::collections::HashMap;

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

    fn get(&self, key: &IntermediateOf<Self>) -> (ValueOf<Self>, ValueOf<Self>);
    fn insert(&mut self, key: IntermediateOf<Self>, value: (ValueOf<Self>, ValueOf<Self>));
    fn remove(&mut self, key: &IntermediateOf<Self>);
}

pub struct InMemoryRawListDB<D: Digest, T>(HashMap<GenericArray<u8, <D as Digest>::OutputSize>, ((Value<GenericArray<u8, <D as Digest>::OutputSize>, T>, Value<GenericArray<u8, <D as Digest>::OutputSize>, T>), usize)>);

impl<D: Digest, T> RawListDB for InMemoryRawListDB<D, T> {
    type Digest = D;
    type Value = T;

    fn get(&self, key: &GenericArray<u8, D::OutputSize>) -> (ValueOf<Self>, ValueOf<Self>) {
        unimplemented!()
    }
    fn insert(&mut self, key: GenericArray<u8, D::OutputSize>, value: (ValueOf<Self>, ValueOf<Self>)) {
        unimplemented!()
    }
    fn remove(&mut self, key: &GenericArray<u8, D::OutputSize>) {
        unimplemented!()
    }
}
