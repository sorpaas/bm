use core::num::NonZeroUsize;

use crate::traits::{RawListDB, Value, EndOf, ValueOf};
use crate::raw::RawList;

const ROOT_INDEX: NonZeroUsize = unsafe { NonZeroUsize::new_unchecked(1) };
const LEFT_INDEX: NonZeroUsize = unsafe { NonZeroUsize::new_unchecked(2) };
const RIGHT_INDEX: NonZeroUsize = unsafe { NonZeroUsize::new_unchecked(3) };

pub struct MerkleEmpty<DB: RawListDB> {
    raw: RawList<DB>,
    default_value: EndOf<DB>,
}

impl<DB: RawListDB> MerkleEmpty<DB> {
    pub fn extend(&mut self, db: &mut DB) {
        let root = self.raw.root();
        self.raw.set(db, LEFT_INDEX, root.clone());
        self.raw.set(db, RIGHT_INDEX, root);
    }

    pub fn shrink(&mut self, db: &mut DB) {
        match self.raw.get(db, LEFT_INDEX) {
            Some(left) => { self.raw.set(db, ROOT_INDEX, left); },
            None => { self.raw.set(db, ROOT_INDEX, Value::End(self.default_value.clone())); }
        }
    }

    pub fn root(&self) -> ValueOf<DB> {
        self.raw.root()
    }

    pub fn new_with_default(default_value: EndOf<DB>) -> Self {
        Self {
            raw: RawList::new_with_default(default_value.clone()),
            default_value,
        }
    }

    pub fn new() -> Self where
        EndOf<DB>: Default
    {
        Self::new_with_default(Default::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::Sha256;

    type InMemory = crate::traits::InMemoryRawListDB<Sha256, Vec<u8>>;

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
