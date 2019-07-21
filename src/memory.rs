#[cfg(feature = "std")]
use std::collections::HashMap as Map;
#[cfg(not(feature = "std"))]
use alloc::collections::BTreeMap as Map;
use generic_array::GenericArray;
use digest::Digest;
use core::marker::PhantomData;
use core::hash::Hash;

use crate::{Value, ValueOf, Construct, Backend, ReadBackend, WriteBackend};

/// Empty status.
pub trait EmptyStatus {
    /// Is the backend using unit empty.
    fn is_unit() -> bool { !Self::is_inherited() }
    /// Is the backend using inherited empty.
    fn is_inherited() -> bool { !Self::is_unit() }
}

/// Inherited empty.
pub struct InheritedEmpty;

impl EmptyStatus for InheritedEmpty {
    fn is_inherited() -> bool { true }
}

/// Unit empty.
pub struct UnitEmpty;

impl EmptyStatus for UnitEmpty {
    fn is_unit() -> bool { true }
}

/// Unit Digest construct.
pub struct UnitDigestConstruct<D: Digest, T: AsRef<[u8]> + Clone + Default>(PhantomData<(D, T)>);

impl<D: Digest, T: AsRef<[u8]> + Clone + Default> Construct for UnitDigestConstruct<D, T> {
    type Intermediate = GenericArray<u8, D::OutputSize>;
    type End = T;

    fn intermediate_of(left: &ValueOf<Self>, right: &ValueOf<Self>) -> Self::Intermediate {
        let mut digest = D::new();
        digest.input(&left.as_ref()[..]);
        digest.input(&right.as_ref()[..]);
        digest.result()
    }

    fn empty_at<DB: WriteBackend<Construct=Self>>(
        _db: &mut DB,
        _depth_to_bottom: usize
    ) -> Result<ValueOf<Self>, DB::Error> {
        Ok(Value::End(Default::default()))
    }
}

/// Inherited Digest construct.
pub struct InheritedDigestConstruct<D: Digest, T: AsRef<[u8]> + Clone + Default>(PhantomData<(D, T)>);

impl<D: Digest, T: AsRef<[u8]> + Clone + Default> Construct for InheritedDigestConstruct<D, T> {
    type Intermediate = GenericArray<u8, D::OutputSize>;
    type End = T;

    fn intermediate_of(left: &ValueOf<Self>, right: &ValueOf<Self>) -> Self::Intermediate {
        let mut digest = D::new();
        digest.input(&left.as_ref()[..]);
        digest.input(&right.as_ref()[..]);
        digest.result()
    }

    fn empty_at<DB: WriteBackend<Construct=Self>>(
        db: &mut DB,
        depth_to_bottom: usize
    ) -> Result<ValueOf<Self>, DB::Error> {
        let mut current = Value::End(Default::default());
        for _ in 0..depth_to_bottom {
            let value = (current.clone(), current);
            let key = Self::intermediate_of(&value.0, &value.1);
            db.insert(key.clone(), value)?;
            current = Value::Intermediate(key);
        }
        Ok(current)
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
);

impl<C: Construct> Default for NoopBackend<C> where
    C::Intermediate: Eq + Hash + Ord
{
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<C: Construct> Clone for NoopBackend<C> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
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
);

impl<C: Construct> Default for InMemoryBackend<C> where
    C::Intermediate: Eq + Hash + Ord
{
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<C: Construct> Clone for InMemoryBackend<C> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<C: Construct> InMemoryBackend<C> where
    C::Intermediate: Eq + Hash + Ord,
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
    C::Intermediate: Eq + Hash + Ord,
{
    fn get(&mut self, key: &C::Intermediate) -> Result<(ValueOf<C>, ValueOf<C>), Self::Error> {
        self.0.get(key).map(|v| v.0.clone()).ok_or(InMemoryBackendError::FetchingKeyNotExist)
    }
}

impl<C: Construct> WriteBackend for InMemoryBackend<C> where
    C::Intermediate: Eq + Hash + Ord,
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
