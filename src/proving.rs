use crate::{MerkleDB, ValueOf, IntermediateOf};
use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

/// Proving merkle database.
pub struct ProvingMerkleDB<'a, DB: MerkleDB> {
    db: &'a mut DB,
    proofs: Mutex<HashMap<IntermediateOf<Self>, (ValueOf<Self>, ValueOf<Self>)>>,
    inserts: HashSet<IntermediateOf<Self>>,
}

impl<'a, DB: MerkleDB> MerkleDB for ProvingMerkleDB<'a, DB> {
    type Digest = DB::Digest;
    type End = DB::End;
    type Error = DB::Error;

    fn get(
        &self,
        key: &IntermediateOf<Self>
    ) -> Result<(ValueOf<Self>, ValueOf<Self>), Self::Error> {
        let value = self.db.get(key)?;
        if !self.inserts.contains(key) {
            self.proofs.lock().expect("Lock is poisoned").insert(key.clone(), value.clone());
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
