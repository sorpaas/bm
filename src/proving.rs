use crate::{Backend, ReadBackend, WriteBackend, Construct, Value, ValueOf};
use core::hash::Hash;
#[cfg(feature = "std")]
use std::collections::{HashMap as Map, HashSet as Set};
#[cfg(not(feature = "std"))]
use alloc::collections::{BTreeMap as Map, BTreeSet as Set};

/// Type of proofs.
pub type Proofs<C> = Map<<C as Construct>::Intermediate, (ValueOf<C>, ValueOf<C>)>;

/// Proving merkle database.
pub struct ProvingBackend<'a, DB: Backend> {
    db: &'a mut DB,
    proofs: Proofs<DB::Construct>,
    inserts: Set<<DB::Construct as Construct>::Intermediate>,
}

impl<'a, DB: Backend> ProvingBackend<'a, DB> where
    <DB::Construct as Construct>::Intermediate: Eq + Hash,
{
    /// Create a new proving database.
    pub fn new(db: &'a mut DB) -> Self {
        Self {
            db,
            proofs: Default::default(),
            inserts: Default::default(),
        }
    }

    /// Get the current pooofs.
    pub fn into_proofs(self) -> Proofs<DB::Construct> {
        self.proofs
    }
}

impl<'a, DB: Backend> Backend for ProvingBackend<'a, DB> {
    type Construct = DB::Construct;
    type Error = DB::Error;
}

impl<'a, DB: ReadBackend> ReadBackend for ProvingBackend<'a, DB> where
    <DB::Construct as Construct>::Intermediate: Eq + Hash,
{
    fn get(
        &mut self,
        key: &<DB::Construct as Construct>::Intermediate
    ) -> Result<(ValueOf<DB::Construct>, ValueOf<DB::Construct>), Self::Error> {
        let value = self.db.get(key)?;
        if !self.inserts.contains(key) {
            self.proofs.insert(key.clone(), value.clone());
        }
        Ok(value)
    }
}

impl<'a, DB: WriteBackend> WriteBackend for ProvingBackend<'a, DB> where
    <DB::Construct as Construct>::Intermediate: Eq + Hash,
{
    fn rootify(&mut self, key: &<DB::Construct as Construct>::Intermediate) -> Result<(), Self::Error> {
        self.db.rootify(key)
    }

    fn unrootify(&mut self, key: &<DB::Construct as Construct>::Intermediate) -> Result<(), Self::Error> {
        self.db.unrootify(key)
    }

    fn insert(
        &mut self,
        key: <DB::Construct as Construct>::Intermediate,
        value: (ValueOf<DB::Construct>, ValueOf<DB::Construct>)
    ) -> Result<(), Self::Error> {
        self.inserts.insert(key.clone());
        self.db.insert(key, value)
    }
}

/// A compact proof entry.
pub type CompactValue<C> = Option<Value<<C as Construct>::Intermediate, <C as Construct>::End>>;

/// Compact merkle proofs.
pub struct CompactProofs<C: Construct>(Map<C::Intermediate, (CompactValue<C>, CompactValue<C>)>);

impl<C: Construct> AsRef<Map<C::Intermediate, (CompactValue<C>, CompactValue<C>)>> for CompactProofs<C> {
    fn as_ref(&self) -> &Map<C::Intermediate, (CompactValue<C>, CompactValue<C>)> {
        &self.0
    }
}

impl<C: Construct> CompactProofs<C> where
    C::Intermediate: Eq + Hash,
{
    /// Create compact merkle proofs from complete entries.
    pub fn from_full(proofs: Proofs<C>) -> Self {
        let mut compacts = Map::new();

        for (key, (left, right)) in proofs.clone() {
            let left = match left {
                Value::Intermediate(left) => {
                    if proofs.contains_key(&left) {
                        None
                    } else {
                        Some(Value::Intermediate(left))
                    }
                },
                Value::End(left) => Some(Value::End(left)),
            };

            let right = match right {
                Value::Intermediate(right) => {
                    if proofs.contains_key(&right) {
                        None
                    } else {
                        Some(Value::Intermediate(right))
                    }
                },
                Value::End(right) => Some(Value::End(right)),
            };

            let skip_key = match (&left, &right) {
                (None, None) => true,
                _ => false,
            };

            if !skip_key {
                compacts.insert(key, (left, right));
            }
        }
        Self(compacts)
    }

    /// Convert the compact proof into full proofs.
    pub fn into_full(self) -> Proofs<C> {
        unimplemented!()
    }
}
