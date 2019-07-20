#[cfg(feature = "std")]
use std::collections::HashMap as Map;
#[cfg(not(feature = "std"))]
use alloc::collections::BTreeMap as Map;
use generic_array::GenericArray;
use digest::Digest;
use core::marker::PhantomData;
use core::hash::Hash;

use crate::{Value, ValueOf, Construct, Backend, ReadBackend, WriteBackend, EmptyBackend};

/// Digest construct.
pub struct DigestConstruct<D: Digest, T: AsRef<[u8]> + Clone + Default>(PhantomData<(D, T)>);

impl<D: Digest, T: AsRef<[u8]> + Clone + Default> Construct for DigestConstruct<D, T> {
    type Intermediate = GenericArray<u8, D::OutputSize>;
    type End = T;

    fn intermediate_of(left: &ValueOf<Self>, right: &ValueOf<Self>) -> Self::Intermediate {
        let mut digest = D::new();
        digest.input(&left.as_ref()[..]);
        digest.input(&right.as_ref()[..]);
        digest.result()
    }
}

#[derive(Debug, Eq, PartialEq, Clone)]
/// Noop DB error.
pub enum NoopBackendError {
    /// Not supported get operation.
    NotSupported,
}

/// Noop merkle database.
pub struct NoopBackend<C: Construct>(
    Map<C::Intermediate, ((ValueOf<C>, ValueOf<C>), Option<usize>)>,
    Option<C::End>,
);

impl<C: Construct> Clone for NoopBackend<C> {
    fn clone(&self) -> Self {
        Self(self.0.clone(), self.1.clone())
    }
}

impl<C: Construct> NoopBackend<C> where
    C::Intermediate: Eq + Hash,
{
    /// Create an in-memory database with unit empty value.
    pub fn new_with_unit_empty(value: C::End) -> Self {
        Self(Default::default(), Some(value))
    }

    /// Create an in-memory database with inherited empty value.
    pub fn new_with_inherited_empty() -> Self {
        Self(Default::default(), None)
    }
}

impl<C: Construct> Backend for NoopBackend<C> {
    type Construct = C;
    type Error = NoopBackendError;
}

impl<C: Construct> ReadBackend for NoopBackend<C> {
    fn get(
        &mut self,
        _key: &C::Intermediate,
    ) -> Result<(ValueOf<C>, ValueOf<C>), Self::Error> {
        Err(NoopBackendError::NotSupported)
    }
}

impl<C: Construct> WriteBackend for NoopBackend<C> {
    fn rootify(&mut self, _key: &C::Intermediate) -> Result<(), Self::Error> {
        Ok(())
    }

    fn unrootify(&mut self, _key: &C::Intermediate) -> Result<(), Self::Error> {
        Ok(())
    }

    fn insert(
        &mut self,
        _key: C::Intermediate,
        _value: (ValueOf<C>, ValueOf<C>)
    ) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl<C: Construct> EmptyBackend for NoopBackend<C> where
    C::Intermediate: Eq + Hash,
{
    fn empty_at(&mut self, depth_to_bottom: usize) -> Result<ValueOf<C>, Self::Error> {
        match &self.1 {
            Some(end) => Ok(Value::End(end.clone())),
            None => {
                let mut current = Value::End(Default::default());
                for _ in 0..depth_to_bottom {
                    let value = (current.clone(), current);
                    let key = C::intermediate_of(&value.0, &value.1);
                    self.0.insert(key.clone(), (value, None));
                    current = Value::Intermediate(key);
                }
                Ok(current)
            }
        }
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

/// In-memory merkle database.
pub struct InMemoryBackend<C: Construct>(
    Map<C::Intermediate, ((ValueOf<C>, ValueOf<C>), Option<usize>)>,
    Option<C::End>,
);

impl<C: Construct> Clone for InMemoryBackend<C> {
    fn clone(&self) -> Self {
        Self(self.0.clone(), self.1.clone())
    }
}

impl<C: Construct> InMemoryBackend<C> where
    C::Intermediate: Eq + Hash,
{
    fn remove(&mut self, old_key: &C::Intermediate) -> Result<(), InMemoryBackendError> {
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
    pub fn new_with_unit_empty(value: C::End) -> Self {
        Self(Default::default(), Some(value))
    }

    /// Create an in-memory database with inherited empty value.
    pub fn new_with_inherited_empty() -> Self {
        Self(Default::default(), None)
    }

    /// Populate the database with proofs.
    pub fn populate(&mut self, proofs: Map<C::Intermediate, (ValueOf<C>, ValueOf<C>)>) {
        for (key, value) in proofs {
            self.0.insert(key, (value, None));
        }
    }
}

impl<C: Construct> AsRef<Map<C::Intermediate, ((ValueOf<C>, ValueOf<C>), Option<usize>)>> for InMemoryBackend<C> {
    fn as_ref(&self) -> &Map<C::Intermediate, ((ValueOf<C>, ValueOf<C>), Option<usize>)> {
        &self.0
    }
}

impl<C: Construct> Backend for InMemoryBackend<C> {
    type Construct = C;
    type Error = InMemoryBackendError;
}

impl<C: Construct> ReadBackend for InMemoryBackend<C> where
    C::Intermediate: Eq + Hash,
{
    fn get(&mut self, key: &C::Intermediate) -> Result<(ValueOf<C>, ValueOf<C>), Self::Error> {
        self.0.get(key).map(|v| v.0.clone()).ok_or(InMemoryBackendError::FetchingKeyNotExist)
    }
}

impl<C: Construct> WriteBackend for InMemoryBackend<C> where
    C::Intermediate: Eq + Hash,
{
    fn rootify(&mut self, key: &C::Intermediate) -> Result<(), Self::Error> {
        self.0.get_mut(key).ok_or(InMemoryBackendError::RootifyKeyNotExist)?.1
            .as_mut().map(|v| *v += 1);
        Ok(())
    }

    fn unrootify(&mut self, key: &C::Intermediate) -> Result<(), Self::Error> {
        self.remove(key)?;
        Ok(())
    }

    fn insert(
        &mut self,
        key: C::Intermediate,
        value: (ValueOf<C>, ValueOf<C>)
    ) -> Result<(), Self::Error> {
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

impl<C: Construct> EmptyBackend for InMemoryBackend<C> where
    C::Intermediate: Eq + Hash,
{
    fn empty_at(&mut self, depth_to_bottom: usize) -> Result<ValueOf<C>, Self::Error> {
        match &self.1 {
            Some(end) => Ok(Value::End(end.clone())),
            None => {
                let mut current = Value::End(Default::default());
                for _ in 0..depth_to_bottom {
                    let value = (current.clone(), current);
                    let key = C::intermediate_of(&value.0, &value.1);
                    self.0.insert(key.clone(), (value, None));
                    current = Value::Intermediate(key);
                }
                Ok(current)
            }
        }
    }
}
