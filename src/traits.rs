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

/// Root status of a merkle tree.
pub trait RootStatus {
    /// Whether it is a dangling root.
    fn is_dangling() -> bool;
    /// Whether it is an owned root.
    fn is_owned() -> bool { !Self::is_dangling() }
}

/// Dangling root status.
pub struct DanglingRoot;

impl RootStatus for DanglingRoot {
    fn is_dangling() -> bool { true }
}

/// Owned root status.
pub struct OwnedRoot;

impl RootStatus for OwnedRoot {
    fn is_dangling() -> bool { false }
}

/// Intermediate value of a database.
pub type IntermediateOf<DB> = GenericArray<u8, IntermediateSizeOf<DB>>;
/// End value of a database.
pub type EndOf<DB> = <DB as MerkleDB>::End;
/// Value of a database.
pub type ValueOf<DB> = Value<IntermediateOf<DB>, EndOf<DB>>;
/// Length of the digest.
pub type IntermediateSizeOf<DB> = <<DB as MerkleDB>::Digest as Digest>::OutputSize;

/// Set error.
#[derive(Debug, Eq, PartialEq, Clone)]
pub enum Error<DBError> {
    /// The database is corrupted.
    CorruptedDatabase,
    /// Intermediate value to set not found.
    IntermediateNotFound,
    /// Backend database error.
    Backend(DBError),
}

impl<DBError> From<DBError> for Error<DBError> {
    fn from(err: DBError) -> Self {
        Error::Backend(err)
    }
}

/// Traits for a merkle database.
pub trait MerkleDB {
    /// Hash function for merkle tree.
    type Digest: Digest;
    /// End value stored in this merkle database.
    type End: AsRef<[u8]> + Clone + Default;
    /// Error type for DB access.
    type Error;

    /// Get an internal item by key.
    fn get(&self, key: &IntermediateOf<Self>) -> Result<Option<(ValueOf<Self>, ValueOf<Self>)>, Self::Error>;
    /// Rootify a key.
    fn rootify(&mut self, key: &IntermediateOf<Self>) -> Result<(), Self::Error>;
    /// Unrootify a key.
    fn unrootify(&mut self, key: &IntermediateOf<Self>) -> Result<(), Self::Error>;
    /// Insert a new internal item.
    fn insert(&mut self, key: IntermediateOf<Self>, value: (ValueOf<Self>, ValueOf<Self>)) -> Result<(), Self::Error>;
}

#[derive(Debug, Eq, PartialEq, Clone)]
/// In-memory DB error.
pub enum InMemoryMerkleDBError {
    /// Trying to rootify a non-existing key.
    RootifyKeyNotExist,
    /// Set subkey does not exist.
    SetIntermediateNotExist
}

#[derive(Clone)]
/// In-memory merkle database.
pub struct InMemoryMerkleDB<D: Digest, T: AsRef<[u8]> + Clone + Default>(
    HashMap<IntermediateOf<Self>, ((ValueOf<Self>, ValueOf<Self>), usize)>
);

impl<D: Digest, T: AsRef<[u8]> + Clone + Default> InMemoryMerkleDB<D, T> {
    fn remove(&mut self, old_key: &IntermediateOf<Self>) -> Result<(), InMemoryMerkleDBError> {
        let (old_value, to_remove) = {
            let value = self.0.get_mut(old_key).ok_or(InMemoryMerkleDBError::SetIntermediateNotExist)?;
            value.1 -= 1;
            (value.0.clone(), value.1 == 0)
        };

        if to_remove {
            match old_value.0 {
                Value::Intermediate(subkey) => { self.remove(&subkey)?; },
                Value::End(_) => (),
            }

            match old_value.1 {
                Value::Intermediate(subkey) => { self.remove(&subkey)?; },
                Value::End(_) => (),
            }

            self.0.remove(old_key);
        }

        Ok(())
    }
}

impl<D: Digest, T: AsRef<[u8]> + Clone + Default> Default for InMemoryMerkleDB<D, T> {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<D: Digest, T: AsRef<[u8]> + Clone + Default> AsRef<HashMap<IntermediateOf<Self>, ((ValueOf<Self>, ValueOf<Self>), usize)>> for InMemoryMerkleDB<D, T> {
    fn as_ref(&self) -> &HashMap<IntermediateOf<Self>, ((ValueOf<Self>, ValueOf<Self>), usize)> {
        &self.0
    }
}

impl<D: Digest, V: AsRef<[u8]> + Clone + Default> MerkleDB for InMemoryMerkleDB<D, V> {
    type Digest = D;
    type End = V;
    type Error = InMemoryMerkleDBError;

    fn get(&self, key: &IntermediateOf<Self>) -> Result<Option<(ValueOf<Self>, ValueOf<Self>)>, Self::Error> {
        Ok(self.0.get(key).map(|v| v.0.clone()))
    }

    fn rootify(&mut self, key: &IntermediateOf<Self>) -> Result<(), Self::Error> {
        self.0.get_mut(key).ok_or(InMemoryMerkleDBError::RootifyKeyNotExist)?.1 += 1;
        Ok(())
    }

    fn unrootify(&mut self, key: &IntermediateOf<Self>) -> Result<(), Self::Error> {
        self.remove(key)?;
        Ok(())
    }

    fn insert(&mut self, key: IntermediateOf<Self>, value: (ValueOf<Self>, ValueOf<Self>)) -> Result<(), Self::Error> {
        if self.0.contains_key(&key) {
            return Ok(())
        }

        let (left, right) = value;

        match &left {
            Value::Intermediate(ref subkey) => {
                self.0.get_mut(subkey).ok_or(InMemoryMerkleDBError::SetIntermediateNotExist)?.1 += 1;
            },
            Value::End(_) => (),
        }
        match &right {
            Value::Intermediate(ref subkey) => {
                self.0.get_mut(subkey).ok_or(InMemoryMerkleDBError::SetIntermediateNotExist)?.1 += 1;
            },
            Value::End(_) => (),
        }

        self.0.insert(key, ((left, right), 0));
        Ok(())
    }
}

/// Leakable value, whose default behavior of drop is to leak.
pub trait Leak {
    /// Metadata to represent this merkle struct.
    type Metadata;

    /// Initialize from a previously leaked value.
    fn from_leaked(metadata: Self::Metadata) -> Self;
    /// Metadata of the value.
    fn metadata(&self) -> Self::Metadata;
}
