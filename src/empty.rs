use crate::traits::{MerkleDB, Value, ValueOf, RootStatus};
use crate::raw::MerkleRaw;
use crate::index::MerkleIndex;

const ROOT_INDEX: MerkleIndex = MerkleIndex::root();
const LEFT_INDEX: MerkleIndex = MerkleIndex::root().left();
const RIGHT_INDEX: MerkleIndex = MerkleIndex::root().right();

/// Merkle structure storing hashes of empty roots.
pub struct MerkleEmpty<R: RootStatus, DB: MerkleDB> {
    raw: MerkleRaw<R, DB>,
}

impl<R: RootStatus, DB: MerkleDB> Default for MerkleEmpty<R, DB> {
    fn default() -> Self {
        Self {
            raw: MerkleRaw::default()
        }
    }
}

impl<R: RootStatus, DB: MerkleDB> MerkleEmpty<R, DB> {
    /// Extend the current empty structure with a new depth.
    pub fn extend(&mut self, db: &mut DB) {
        let root = self.raw.root();
        self.raw.set(db, LEFT_INDEX, root.clone());
        self.raw.set(db, RIGHT_INDEX, root);
    }

    /// Shrink the current empty structure.
    pub fn shrink(&mut self, db: &mut DB) {
        match self.raw.get(db, LEFT_INDEX) {
            Some(left) => { self.raw.set(db, ROOT_INDEX, left); },
            None => { self.raw.set(db, ROOT_INDEX, Value::End(Default::default())); }
        }
    }

    /// Root of the current depth.
    pub fn root(&self) -> ValueOf<DB> {
        self.raw.root()
    }

    /// Drop the merkle tree.
    pub fn drop(self, db: &mut DB) {
        self.raw.drop(db)
    }

    /// Leak the merkle tree.
    pub fn leak(self) -> ValueOf<DB> {
        self.raw.leak()
    }

    /// Initialize this merkle tree from a previously leaked one.
    pub fn from_leaked(root: ValueOf<DB>) -> Self {
        Self {
            raw: MerkleRaw::from_leaked(root)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::Sha256;

    type InMemory = crate::traits::InMemoryMerkleDB<Sha256, Vec<u8>>;

    #[test]
    fn test_extend_shrink() {
        let mut db = InMemory::default();
        let mut empty = MerkleEmpty::<InMemory>::new();

        let mut values = Vec::new();
        for _ in 0..32 {
            values.push(empty.root());
            empty.extend(&mut db);
        }
        while let Some(root) = values.pop() {
            empty.shrink(&mut db);
            assert_eq!(root, empty.root());
        }
        assert!(db.as_ref().is_empty());
    }
}
