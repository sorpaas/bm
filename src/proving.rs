use crate::{Backend, ReadBackend, WriteBackend, Construct};
use core::hash::Hash;
use core::ops::Deref;
use core::fmt;
use alloc::boxed::Box;
#[cfg(feature = "std")]
use std::collections::{HashMap as Map, HashSet as Set};
#[cfg(not(feature = "std"))]
use alloc::collections::{BTreeMap as Map, BTreeSet as Set};

/// Proving merkle database.
pub struct ProvingBackend<'a, DB: Backend> {
    db: &'a mut DB,
    proofs: Map<<DB::Construct as Construct>::Value,
                (<DB::Construct as Construct>::Value, <DB::Construct as Construct>::Value)>,
    inserts: Set<<DB::Construct as Construct>::Value>,
}

impl<'a, DB: Backend> ProvingBackend<'a, DB> where
    <DB::Construct as Construct>::Value: Eq + Hash + Ord,
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
        Proofs(self.proofs)
    }
}

impl<'a, DB: Backend> Backend for ProvingBackend<'a, DB> {
    type Construct = DB::Construct;
    type Error = DB::Error;
}

impl<'a, DB: ReadBackend> ReadBackend for ProvingBackend<'a, DB> where
    <DB::Construct as Construct>::Value: Eq + Hash + Ord,
{
    fn get(
        &mut self,
        key: &<DB::Construct as Construct>::Value
    ) -> Result<Option<(<DB::Construct as Construct>::Value, <DB::Construct as Construct>::Value)>, Self::Error> {
        let value = match self.db.get(key)? {
            Some(value) => value,
            None => return Ok(None),
        };
        if !self.inserts.contains(key) {
            self.proofs.insert(key.clone(), value.clone());
        }
        Ok(Some(value))
    }
}

impl<'a, DB: WriteBackend> WriteBackend for ProvingBackend<'a, DB> where
    <DB::Construct as Construct>::Value: Eq + Hash + Ord,
{
    fn rootify(&mut self, key: &<DB::Construct as Construct>::Value) -> Result<(), Self::Error> {
        self.db.rootify(key)
    }

    fn unrootify(&mut self, key: &<DB::Construct as Construct>::Value) -> Result<(), Self::Error> {
        self.db.unrootify(key)
    }

    fn insert(
        &mut self,
        key: <DB::Construct as Construct>::Value,
        value: (<DB::Construct as Construct>::Value, <DB::Construct as Construct>::Value)
    ) -> Result<(), Self::Error> {
        self.inserts.insert(key.clone());
        self.db.insert(key, value)
    }
}

/// Type of proofs.
pub struct Proofs<C: Construct>(Map<C::Value, (C::Value, C::Value)>);

impl<C: Construct> Into<Map<C::Value, (C::Value, C::Value)>> for Proofs<C> {
    fn into(self) -> Map<C::Value, (C::Value, C::Value)> {
        self.0
    }
}

impl<C: Construct> Default for Proofs<C> where
    C::Value: Eq + Hash + Ord
{
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<C: Construct> Clone for Proofs<C> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<C: Construct> Deref for Proofs<C> {
    type Target = Map<C::Value, (C::Value, C::Value)>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<C: Construct> PartialEq for Proofs<C> where
    C::Value: Eq + Hash + Ord,
{
    fn eq(&self, other: &Self) -> bool {
        self.0.eq(&other.0)
    }
}

impl<C: Construct> Eq for Proofs<C> where
    C::Value: Eq + Hash + Ord { }

impl<C: Construct> fmt::Debug for Proofs<C> where
    C::Value: Eq + Hash + Ord + fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl<C: Construct> Proofs<C> where
    C::Value: Eq + Hash + Ord,
{
    /// Create compact merkle proofs from complete entries.
    pub fn into_compact(&self, root: C::Value) -> CompactValue<C::Value> {
        if let Some((left, right)) = self.0.get(&root) {
            let compact_left = self.into_compact(left.clone());
            let compact_right = self.into_compact(right.clone());
            CompactValue::Combined(Box::new((compact_left, compact_right)))
        } else {
            CompactValue::Single(root)
        }
    }

    /// Convert the compact value into full proofs.
    pub fn from_compact(compact: CompactValue<C::Value>) -> (Self, C::Value) {
        match compact {
            CompactValue::Single(root) => (Proofs(Default::default()), root),
            CompactValue::Combined(boxed) => {
                let (compact_left, compact_right) = *boxed;
                let (left_proofs, left) = Self::from_compact(compact_left);
                let (right_proofs, right) = Self::from_compact(compact_right);
                let mut proofs = left_proofs.0.into_iter()
                    .chain(right_proofs.0.into_iter())
                    .collect::<Map<C::Value, (C::Value, C::Value)>>();
                let key = C::intermediate_of(&left, &right);
                proofs.insert(key.clone(), (left, right));
                (Proofs(proofs), key)
            },
        }
    }
}

/// Compact proofs.
#[derive(Clone, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "parity-scale-codec", derive(parity_scale_codec::Encode, parity_scale_codec::Decode))]
pub enum CompactValue<V> {
    /// Single compact value.
    Single(V),
    /// Value is combined by other left and right entries.
    Combined(Box<(CompactValue<V>, CompactValue<V>)>),
}

impl<V> CompactValue<V> {
    /// Get the length of the current value.
    pub fn len(&self) -> usize {
        match self {
            CompactValue::Single(_) => 1,
            CompactValue::Combined(boxed) => {
                boxed.as_ref().0.len() + boxed.as_ref().1.len()
            },
        }
    }
}
