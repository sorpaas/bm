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

/// Compact proofs.
#[derive(Clone)]
pub enum CompactValue<C: Construct> {
    /// Single compact value.
    Single(ValueOf<C>),
    /// Value is combined by other left and right entries.
    Combined(Box<(CompactValue<C>, CompactValue<C>)>),
}

impl<C: Construct> CompactValue<C> where
    C::Intermediate: Eq + Hash,
{
    /// Get the length of the current value.
    pub fn len(&self) -> usize {
        match self {
            CompactValue::Single(_) => 1,
            CompactValue::Combined(boxed) => {
                boxed.as_ref().0.len() + boxed.as_ref().1.len()
            },
        }
    }

    /// Create compact merkle proofs from complete entries.
    pub fn from_proofs(root: ValueOf<C>, proofs: &Proofs<C>) -> Self {
        match root {
            Value::End(end) => CompactValue::Single(Value::End(end)),
            Value::Intermediate(intermediate) => {
                if let Some((left, right)) = proofs.get(&intermediate) {
                    let compact_left = Self::from_proofs(left.clone(), proofs);
                    let compact_right = Self::from_proofs(right.clone(), proofs);
                    CompactValue::Combined(Box::new((compact_left, compact_right)))
                } else {
                    CompactValue::Single(Value::Intermediate(intermediate))
                }
            },
        }
    }

    /// Convert the compact value into full proofs.
    pub fn into_proofs(self) -> (ValueOf<C>, Proofs<C>) {
        match self {
            CompactValue::Single(root) => (root, Default::default()),
            CompactValue::Combined(boxed) => {
                let (compact_left, compact_right) = *boxed;
                let (left, left_proofs) = compact_left.into_proofs();
                let (right, right_proofs) = compact_right.into_proofs();
                let mut proofs = left_proofs.into_iter()
                    .chain(right_proofs.into_iter())
                    .collect::<Proofs<C>>();
                let key = C::intermediate_of(&left, &right);
                proofs.insert(key.clone(), (left, right));
                (Value::Intermediate(key), proofs)
            },
        }
    }
}
