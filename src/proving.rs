use crate::{Backend, ValueOf, IntermediateOf};
use core::hash::Hash;
use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

/// Proving merkle database.
pub struct ProvingBackend<'a, DB: Backend> where
    IntermediateOf<DB>: Eq + Hash,
{
    db: &'a mut DB,
    proofs: HashMap<IntermediateOf<Self>, (ValueOf<Self>, ValueOf<Self>)>,
    inserts: HashSet<IntermediateOf<Self>>,
}

impl<'a, DB: Backend> ProvingBackend<'a, DB> where
    IntermediateOf<DB>: Eq + Hash,
{
    /// Create a new proving database.
    pub fn new(db: &'a mut DB) -> Self {
        Self {
            db,
            proofs: Default::default(),
            inserts: Default::default(),
        }
    }

    /// Reset the proving database and get all the proofs.
    pub fn reset(&mut self) -> HashMap<IntermediateOf<Self>, (ValueOf<Self>, ValueOf<Self>)> {
        let proofs = self.proofs.clone();
        self.proofs = Default::default();
        self.inserts = Default::default();
        proofs
    }
}

impl<'a, DB: Backend> Backend for ProvingBackend<'a, DB> where
    IntermediateOf<DB>: Eq + Hash,
{
    type Intermediate = DB::Intermediate;
    type End = DB::End;
    type Error = DB::Error;

    fn intermediate_of(left: &ValueOf<Self>, right: &ValueOf<Self>) -> IntermediateOf<Self> {
        DB::intermediate_of(left, right)
    }

    fn empty_at(&mut self, depth_to_bottom: usize) -> Result<ValueOf<Self>, Self::Error> {
        self.db.empty_at(depth_to_bottom)
    }

    fn get(
        &mut self,
        key: &IntermediateOf<Self>
    ) -> Result<(ValueOf<Self>, ValueOf<Self>), Self::Error> {
        let value = self.db.get(key)?;
        if !self.inserts.contains(key) {
            self.proofs.insert(key.clone(), value.clone());
        }
        Ok(value)
    }

    fn rootify(&mut self, key: &IntermediateOf<Self>) -> Result<(), Self::Error> {
        self.db.rootify(key)
    }

    fn unrootify(&mut self, key: &IntermediateOf<Self>) -> Result<(), Self::Error> {
        self.db.unrootify(key)
    }

    fn insert(
        &mut self,
        key: IntermediateOf<Self>,
        value: (ValueOf<Self>, ValueOf<Self>)
    ) -> Result<(), Self::Error> {
        self.inserts.insert(key.clone());
        self.db.insert(key, value)
    }
}
