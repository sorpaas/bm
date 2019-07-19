#[cfg(feature = "std")]
use std::collections::HashMap as Map;
#[cfg(not(feature = "std"))]
use alloc::collections::BTreeMap as Map;
use generic_array::GenericArray;
use digest::Digest;

use crate::{Value, ValueOf, IntermediateOf, EndOf, Backend};

#[derive(Debug, Eq, PartialEq, Clone)]
/// Noop DB error.
pub enum NoopBackendError {
    /// Not supported get operation.
    NotSupported,
}

#[derive(Clone)]
/// Noop merkle database.
pub struct NoopBackend<D: Digest, T: AsRef<[u8]> + Clone + Default>(
    Map<IntermediateOf<Self>, ((ValueOf<Self>, ValueOf<Self>), Option<usize>)>,
    Option<EndOf<Self>>,
);

impl<D: Digest, T: AsRef<[u8]> + Clone + Default> NoopBackend<D, T> {
    /// Create an in-memory database with unit empty value.
    pub fn new_with_unit_empty(value: EndOf<Self>) -> Self {
        Self(Default::default(), Some(value))
    }

    /// Create an in-memory database with inherited empty value.
    pub fn new_with_inherited_empty() -> Self {
        Self(Default::default(), None)
    }
}

impl<D: Digest, V: AsRef<[u8]> + Clone + Default> Backend for NoopBackend<D, V> {
    type Intermediate = GenericArray<u8, D::OutputSize>;
    type End = V;
    type Error = NoopBackendError;

    fn intermediate_of(left: &ValueOf<Self>, right: &ValueOf<Self>) -> IntermediateOf<Self> {
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
                    let key = Self::intermediate_of(&value.0, &value.1);
                    self.0.insert(key.clone(), (value, None));
                    current = Value::Intermediate(key);
                }
                Ok(current)
            }
        }
    }

    fn get(&mut self, _key: &IntermediateOf<Self>) -> Result<(ValueOf<Self>, ValueOf<Self>), Self::Error> {
        Err(NoopBackendError::NotSupported)
    }

    fn rootify(&mut self, _key: &IntermediateOf<Self>) -> Result<(), Self::Error> {
        Ok(())
    }

    fn unrootify(&mut self, _key: &IntermediateOf<Self>) -> Result<(), Self::Error> {
        Ok(())
    }

    fn insert(&mut self, _key: IntermediateOf<Self>, _value: (ValueOf<Self>, ValueOf<Self>)) -> Result<(), Self::Error> {
        Ok(())
    }
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
    Map<IntermediateOf<Self>, ((ValueOf<Self>, ValueOf<Self>), Option<usize>)>,
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
    pub fn populate(&mut self, proofs: Map<IntermediateOf<Self>, (ValueOf<Self>, ValueOf<Self>)>) {
        for (key, value) in proofs {
            self.0.insert(key, (value, None));
        }
    }
}

impl<D: Digest, T: AsRef<[u8]> + Clone + Default> AsRef<Map<IntermediateOf<Self>, ((ValueOf<Self>, ValueOf<Self>), Option<usize>)>> for InMemoryBackend<D, T> {
    fn as_ref(&self) -> &Map<IntermediateOf<Self>, ((ValueOf<Self>, ValueOf<Self>), Option<usize>)> {
        &self.0
    }
}

impl<D: Digest, V: AsRef<[u8]> + Clone + Default> Backend for InMemoryBackend<D, V> {
    type Intermediate = GenericArray<u8, D::OutputSize>;
    type End = V;
    type Error = InMemoryBackendError;

    fn intermediate_of(left: &ValueOf<Self>, right: &ValueOf<Self>) -> IntermediateOf<Self> {
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
                    let key = Self::intermediate_of(&value.0, &value.1);
                    self.0.insert(key.clone(), (value, None));
                    current = Value::Intermediate(key);
                }
                Ok(current)
            }
        }
    }

    fn get(&mut self, key: &IntermediateOf<Self>) -> Result<(ValueOf<Self>, ValueOf<Self>), Self::Error> {
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
