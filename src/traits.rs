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
pub type ReplaceValueOf<DB> = ReplaceValue<IntermediateOf<DB>, EndOf<DB>>;

pub trait RawListDB: Default {
    type Digest: Digest;
    type Value: AsRef<[u8]> + Clone;

    fn get(&self, key: &IntermediateOf<Self>) -> Option<(ValueOf<Self>, ValueOf<Self>)>;
    fn replace(&mut self, old: ReplaceValueOf<Self>, new: ReplaceValueOf<Self>);
}

#[derive(Clone)]
pub struct InMemoryRawListDB<D: Digest, T: AsRef<[u8]> + Clone>(
    HashMap<IntermediateOf<Self>, ((ValueOf<Self>, ValueOf<Self>), Option<usize>)>
);

impl<D: Digest, T: AsRef<[u8]> + Clone> Default for InMemoryRawListDB<D, T> {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<D: Digest, T: AsRef<[u8]> + Clone> AsRef<HashMap<IntermediateOf<Self>, ((ValueOf<Self>, ValueOf<Self>), Option<usize>)>> for InMemoryRawListDB<D, T> {
    fn as_ref(&self) -> &HashMap<IntermediateOf<Self>, ((ValueOf<Self>, ValueOf<Self>), Option<usize>)> {
        &self.0
    }
}

#[derive(Clone, Eq, PartialEq, Debug)]
pub enum ReplaceValue<I, E> {
    EndOrNone,
    Intermediate((I, (Value<I, E>, Value<I, E>))),
}

impl<D: Digest, T: AsRef<[u8]> + Clone> RawListDB for InMemoryRawListDB<D, T> {
    type Digest = D;
    type Value = T;

    fn get(&self, key: &GenericArray<u8, D::OutputSize>) -> Option<(ValueOf<Self>, ValueOf<Self>)> {
        self.0.get(key).map(|v| v.0.clone())
    }

    fn replace(&mut self, old: ReplaceValueOf<Self>, new: ReplaceValueOf<Self>) {
        print!("old: ");
        match old.clone() {
            ReplaceValue::EndOrNone => { print!("EndOrNone  "); },
            ReplaceValue::Intermediate((key, value)) => {
                print!("{:?} => ({:?}, {:?})  ", key.as_ref(), value.0.as_ref(), value.1.as_ref());
            },
        }
        print!("new: ");
        match new.clone() {
            ReplaceValue::EndOrNone => { print!("EndOrNone  "); },
            ReplaceValue::Intermediate((key, value)) => {
                print!("{:?} => ({:?}, {:?})  ", key.as_ref(), value.0.as_ref(), value.1.as_ref());
            },
        }
        println!("");

        match new {
            ReplaceValue::Intermediate((key, value)) => {
                self.0.entry(key).or_insert((value, Some(1)));
            },
            _ => (),
        }
    }
}
