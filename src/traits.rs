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
pub struct Dangling;

impl RootStatus for Dangling {
    fn is_dangling() -> bool { true }
}

/// Owned root status.
pub struct Owned;

impl RootStatus for Owned {
    fn is_dangling() -> bool { false }
}

/// Intermediate value of a database.
pub type IntermediateOf<DB> = <DB as Backend>::Intermediate;
/// End value of a database.
pub type EndOf<DB> = <DB as Backend>::End;
/// Value of a database.
pub type ValueOf<DB> = Value<IntermediateOf<DB>, EndOf<DB>>;

/// Set error.
#[derive(Debug, Eq, PartialEq, Clone)]
pub enum Error<DBError> {
    /// The database is corrupted.
    CorruptedDatabase,
    /// Backend database error.
    Backend(DBError),
}

impl<DBError> From<DBError> for Error<DBError> {
    fn from(err: DBError) -> Self {
        Error::Backend(err)
    }
}

/// Traits for a merkle database.
pub trait Backend {
    /// Intermediate value stored in this merkle database.
    type Intermediate: AsRef<[u8]> + Clone;
    /// End value stored in this merkle database.
    type End: AsRef<[u8]> + Clone + Default;
    /// Error type for DB access.
    type Error;

    /// Get the intermediate value of given left and right child.
    fn intermediate_of(&self, left: &ValueOf<Self>, right: &ValueOf<Self>) -> IntermediateOf<Self>;
    /// Get or create the empty value at given depth-to-bottom.
    fn empty_at(&mut self, depth_to_bottom: usize) -> Result<ValueOf<Self>, Self::Error>;
    /// Get an internal item by key.
    fn get(&self, key: &IntermediateOf<Self>) -> Result<(ValueOf<Self>, ValueOf<Self>), Self::Error>;
    /// Rootify a key.
    fn rootify(&mut self, key: &IntermediateOf<Self>) -> Result<(), Self::Error>;
    /// Unrootify a key.
    fn unrootify(&mut self, key: &IntermediateOf<Self>) -> Result<(), Self::Error>;
    /// Insert a new internal item.
    fn insert(&mut self, key: IntermediateOf<Self>, value: (ValueOf<Self>, ValueOf<Self>)) -> Result<(), Self::Error>;
}

#[derive(Debug, Eq, PartialEq, Clone)]
/// In-memory DB error.
pub enum InMemoryBackendError {
    /// Fetching key not exist.
    FetchingKeyNotExist,
    /// Trying to rootify a non-existing key.
    RootifyKeyNotExist,
    /// Set subkey does not exist.
    SetIntermediateNotExist
}

#[derive(Clone)]
/// In-memory merkle database.
pub struct InMemoryBackend<D: Digest, T: AsRef<[u8]> + Clone + Default>(
    HashMap<IntermediateOf<Self>, ((ValueOf<Self>, ValueOf<Self>), Option<usize>)>,
    Option<EndOf<Self>>,
);

impl<D: Digest, T: AsRef<[u8]> + Clone + Default> InMemoryBackend<D, T> {
    fn remove(&mut self, old_key: &IntermediateOf<Self>) -> Result<(), InMemoryBackendError> {
        let (old_value, to_remove) = {
            let value = self.0.get_mut(old_key).ok_or(InMemoryBackendError::SetIntermediateNotExist)?;
            value.1.as_mut().map(|v| *v -= 1);
            (value.0.clone(), value.1.map(|v| v == 0).unwrap_or(false))
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

    /// Create an in-memory database with unit empty value.
    pub fn new_with_unit_empty(value: EndOf<Self>) -> Self {
        Self(Default::default(), Some(value))
    }

    /// Create an in-memory database with inherited empty value.
    pub fn new_with_inherited_empty() -> Self {
        Self(Default::default(), None)
    }

    /// Populate the database with proofs.
    pub fn populate(&mut self, proofs: HashMap<IntermediateOf<Self>, (ValueOf<Self>, ValueOf<Self>)>) {
        for (key, value) in proofs {
            self.0.insert(key, (value, None));
        }
    }
}

impl<D: Digest, T: AsRef<[u8]> + Clone + Default> AsRef<HashMap<IntermediateOf<Self>, ((ValueOf<Self>, ValueOf<Self>), Option<usize>)>> for InMemoryBackend<D, T> {
    fn as_ref(&self) -> &HashMap<IntermediateOf<Self>, ((ValueOf<Self>, ValueOf<Self>), Option<usize>)> {
        &self.0
    }
}

impl<D: Digest, V: AsRef<[u8]> + Clone + Default> Backend for InMemoryBackend<D, V> {
    type Intermediate = GenericArray<u8, D::OutputSize>;
    type End = V;
    type Error = InMemoryBackendError;

    fn intermediate_of(&self, left: &ValueOf<Self>, right: &ValueOf<Self>) -> IntermediateOf<Self> {
        let mut digest = D::new();
        digest.input(&left.as_ref()[..]);
        digest.input(&right.as_ref()[..]);
        digest.result()
    }

    fn empty_at(&mut self, depth_to_bottom: usize) -> Result<ValueOf<Self>, Self::Error> {
        match &self.1 {
            Some(end) => Ok(Value::End(end.clone())),
            None => {
                let mut current = Value::End(Default::default());
                for _ in 0..depth_to_bottom {
                    let value = (current.clone(), current);
                    let key = self.intermediate_of(&value.0, &value.1);
                    self.0.insert(key.clone(), (value, None));
                    current = Value::Intermediate(key);
                }
                Ok(current)
            }
        }
    }

    fn get(&self, key: &IntermediateOf<Self>) -> Result<(ValueOf<Self>, ValueOf<Self>), Self::Error> {
        self.0.get(key).map(|v| v.0.clone()).ok_or(InMemoryBackendError::FetchingKeyNotExist)
    }

    fn rootify(&mut self, key: &IntermediateOf<Self>) -> Result<(), Self::Error> {
        self.0.get_mut(key).ok_or(InMemoryBackendError::RootifyKeyNotExist)?.1
            .as_mut().map(|v| *v += 1);
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
                self.0.get_mut(subkey).ok_or(InMemoryBackendError::SetIntermediateNotExist)?.1
                    .as_mut().map(|v| *v += 1);
            },
            Value::End(_) => (),
        }
        match &right {
            Value::Intermediate(ref subkey) => {
                self.0.get_mut(subkey).ok_or(InMemoryBackendError::SetIntermediateNotExist)?.1
                    .as_mut().map(|v| *v += 1);
            },
            Value::End(_) => (),
        }

        self.0.insert(key, ((left, right), Some(0)));
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
