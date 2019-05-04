use digest::Digest;
use generic_array::GenericArray;

use std::collections::HashMap;

/// Value in a merkle tree.
#[derive(Clone, Eq, PartialEq, Debug)]
pub enum Value<I, E> {
    /// Intermediate, which is hash of two sub-items.
    Intermediate(I),
    /// End value of the tree.
    End(E),
}

impl<I, E> Value<I, E> {
    /// Return `Some` if this value is an intermediate, otherwise return `None`.
    pub fn intermediate(self) -> Option<I> {
        match self {
            Value::Intermediate(intermediate) => Some(intermediate),
            Value::End(_) => None,
        }
    }

    /// Return `Some` if this value is an end value, otherwise return `None`.
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

/// Intermediate value of a database.
pub type IntermediateOf<DB> = GenericArray<u8, IntermediateSizeOf<DB>>;
/// End value of a database.
pub type EndOf<DB> = <DB as MerkleDB>::Value;
/// Value of a database.
pub type ValueOf<DB> = Value<IntermediateOf<DB>, EndOf<DB>>;
/// Length of the digest.
pub type IntermediateSizeOf<DB> = <<DB as MerkleDB>::Digest as Digest>::OutputSize;

/// Traits for a merkle database.
pub trait MerkleDB: Default {
    /// Hash function for merkle tree.
    type Digest: Digest;
    /// End value stored in this merkle database.
    type Value: AsRef<[u8]> + Clone + Default;

    /// Get an internal item by key.
    fn get(&self, key: &IntermediateOf<Self>) -> Option<(ValueOf<Self>, ValueOf<Self>)>;
    /// Rootify a key.
    fn rootify(&mut self, key: &IntermediateOf<Self>);
    /// Unrootify a key.
    fn unrootify(&mut self, key: &IntermediateOf<Self>);
    /// Insert a new internal item.
    fn insert(&mut self, key: IntermediateOf<Self>, value: (ValueOf<Self>, ValueOf<Self>));
}

#[derive(Clone)]
/// In-memory merkle database.
pub struct InMemoryMerkleDB<D: Digest, T: AsRef<[u8]> + Clone + Default>(
    HashMap<IntermediateOf<Self>, ((ValueOf<Self>, ValueOf<Self>), usize)>,
);

impl<D: Digest, T: AsRef<[u8]> + Clone + Default> InMemoryMerkleDB<D, T> {
    fn remove(&mut self, old_key: &IntermediateOf<Self>) {
        let (old_value, to_remove) = {
            let value = self.0.get_mut(old_key).expect("Set key does not exist");
            value.1 -= 1;
            (value.0.clone(), value.1 == 0)
        };

        if to_remove {
            match old_value.0 {
                Value::Intermediate(subkey) => {
                    self.remove(&subkey);
                }
                Value::End(_) => (),
            }

            match old_value.1 {
                Value::Intermediate(subkey) => {
                    self.remove(&subkey);
                }
                Value::End(_) => (),
            }

            self.0.remove(old_key);
        }
    }
}

impl<D: Digest, T: AsRef<[u8]> + Clone + Default> Default for InMemoryMerkleDB<D, T> {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<D: Digest, T: AsRef<[u8]> + Clone + Default>
    AsRef<HashMap<IntermediateOf<Self>, ((ValueOf<Self>, ValueOf<Self>), usize)>>
    for InMemoryMerkleDB<D, T>
{
    fn as_ref(&self) -> &HashMap<IntermediateOf<Self>, ((ValueOf<Self>, ValueOf<Self>), usize)> {
        &self.0
    }
}

impl<D: Digest, T: AsRef<[u8]> + Clone + Default> MerkleDB for InMemoryMerkleDB<D, T> {
    type Digest = D;
    type Value = T;

    fn get(&self, key: &GenericArray<u8, D::OutputSize>) -> Option<(ValueOf<Self>, ValueOf<Self>)> {
        self.0.get(key).map(|v| v.0.clone())
    }

    fn rootify(&mut self, key: &IntermediateOf<Self>) {
        self.0
            .get_mut(key)
            .expect("Trying to rootify a non-existing key")
            .1 += 1;
    }

    fn unrootify(&mut self, key: &IntermediateOf<Self>) {
        self.remove(key);
    }

    fn insert(&mut self, key: IntermediateOf<Self>, value: (ValueOf<Self>, ValueOf<Self>)) {
        if self.0.contains_key(&key) {
            return;
        }

        let (left, right) = value;

        match &left {
            Value::Intermediate(ref subkey) => {
                self.0.get_mut(subkey).expect("Set subkey does not exist").1 += 1;
            }
            Value::End(_) => (),
        }
        match &right {
            Value::Intermediate(ref subkey) => {
                self.0.get_mut(subkey).expect("Set subkey does not exist").1 += 1;
            }
            Value::End(_) => (),
        }

        self.0.insert(key, ((left, right), 0));
    }
}
