use crate::{Backend, ReadBackend, WriteBackend, Construct, Value, ValueOf};
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
    proofs: Map<<DB::Construct as Construct>::Intermediate, (ValueOf<DB::Construct>, ValueOf<DB::Construct>)>,
    inserts: Set<<DB::Construct as Construct>::Intermediate>,
}

impl<'a, DB: Backend> ProvingBackend<'a, DB> where
    <DB::Construct as Construct>::Intermediate: Eq + Hash + Ord,
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
    <DB::Construct as Construct>::Intermediate: Eq + Hash + Ord,
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
    <DB::Construct as Construct>::Intermediate: Eq + Hash + Ord,
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

/// Type of proofs.
pub struct Proofs<C: Construct>(Map<C::Intermediate, (ValueOf<C>, ValueOf<C>)>);

impl<C: Construct> Into<Map<C::Intermediate, (ValueOf<C>, ValueOf<C>)>> for Proofs<C> {
    fn into(self) -> Map<C::Intermediate, (ValueOf<C>, ValueOf<C>)> {
        self.0
    }
}

impl<C: Construct> Default for Proofs<C> where
    C::Intermediate: Eq + Hash + Ord
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
    type Target = Map<C::Intermediate, (ValueOf<C>, ValueOf<C>)>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<C: Construct> PartialEq for Proofs<C> where
    C::Intermediate: Eq + Hash + Ord,
    C::End: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.0.eq(&other.0)
    }
}

impl<C: Construct> Eq for Proofs<C> where
    C::Intermediate: Eq + Hash + Ord,
    C::End: Eq { }

impl<C: Construct> fmt::Debug for Proofs<C> where
    C::Intermediate: Eq + Hash + Ord + fmt::Debug,
    C::End: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl<C: Construct> Proofs<C> where
    C::Intermediate: Eq + Hash + Ord,
{
    /// Create compact merkle proofs from complete entries.
    pub fn into_compact(&self, root: ValueOf<C>) -> CompactValue<C::Intermediate, C::End> {
        match root {
            Value::End(end) => CompactValue::Single(Value::End(end)),
            Value::Intermediate(intermediate) => {
                if let Some((left, right)) = self.0.get(&intermediate) {
                    let compact_left = self.into_compact(left.clone());
                    let compact_right = self.into_compact(right.clone());
                    CompactValue::Combined(Box::new((compact_left, compact_right)))
                } else {
                    CompactValue::Single(Value::Intermediate(intermediate))
                }
            },
        }
    }

    /// Convert the compact value into full proofs.
    pub fn from_compact(compact: CompactValue<C::Intermediate, C::End>) -> (Self, ValueOf<C>) {
        match compact {
            CompactValue::Single(root) => (Proofs(Default::default()), root),
            CompactValue::Combined(boxed) => {
                let (compact_left, compact_right) = *boxed;
                let (left_proofs, left) = Self::from_compact(compact_left);
                let (right_proofs, right) = Self::from_compact(compact_right);
                let mut proofs = left_proofs.0.into_iter()
                    .chain(right_proofs.0.into_iter())
                    .collect::<Map<C::Intermediate, (ValueOf<C>, ValueOf<C>)>>();
                let key = C::intermediate_of(&left, &right);
                proofs.insert(key.clone(), (left, right));
                (Proofs(proofs), Value::Intermediate(key))
            },
        }
    }
}

/// Compact proofs.
#[derive(Clone, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "parity-scale-codec", derive(parity_scale_codec::Encode, parity_scale_codec::Decode))]
pub enum CompactValue<I, E> {
    /// Single compact value.
    Single(Value<I, E>),
    /// Value is combined by other left and right entries.
    Combined(Box<(CompactValue<I, E>, CompactValue<I, E>)>),
}

impl<I, E> CompactValue<I, E> {
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
