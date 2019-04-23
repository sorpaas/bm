use core::num::NonZeroUsize;

use crate::traits::{MerkleDB, EndOf, Value, ValueOf};
use crate::tuple::MerkleTuple;
use crate::raw::MerkleRaw;

const LEN_INDEX: NonZeroUsize = unsafe { NonZeroUsize::new_unchecked(3) };
const ITEM_ROOT_INDEX: NonZeroUsize = unsafe { NonZeroUsize::new_unchecked(2) };

/// Binary merkle vector.
pub struct MerkleVec<DB: MerkleDB> {
    raw: MerkleRaw<DB>,
    tuple: MerkleTuple<DB>,
}

impl<DB: MerkleDB> MerkleVec<DB> where
    EndOf<DB>: From<usize> + Into<usize>,
{
    fn update_metadata(&mut self, db: &mut DB) {
        self.raw.set(db, ITEM_ROOT_INDEX, self.tuple.root());
        self.raw.set(db, LEN_INDEX, Value::End(self.tuple.len().into()));
    }

    /// Get value at index.
    pub fn get(&self, db: &DB, index: usize) -> EndOf<DB> {
        self.tuple.get(db, index)
    }

    /// Set value at index.
    pub fn set(&mut self, db: &mut DB, index: usize, value: EndOf<DB>) {
        self.tuple.set(db, index, value);
        self.update_metadata(db);
    }

    /// Root of the current merkle vector.
    pub fn root(&self) -> ValueOf<DB> {
        self.raw.root()
    }

    /// Push a new value to the vector.
    pub fn push(&mut self, db: &mut DB, value: EndOf<DB>) {
        self.tuple.push(db, value);
        self.update_metadata(db);
    }

    /// Pop a value from the vector.
    pub fn pop(&mut self, db: &mut DB) -> Option<EndOf<DB>> {
        let ret = self.tuple.pop(db);
        self.update_metadata(db);
        ret
    }

    /// Length of the vector.
    pub fn len(&self) -> usize {
        self.tuple.len()
    }

    /// Create a new vector.
    pub fn create(db: &mut DB) -> Self {
        let tuple = MerkleTuple::create(db, 0);
        let raw = MerkleRaw::new();
        let mut ret = Self { raw, tuple };
        ret.update_metadata(db);
        ret
    }

    /// Drop the current vector.
    pub fn drop(self, db: &mut DB) {
        self.raw.drop(db);
        self.tuple.drop(db);
    }

    /// Leak the current vector.
    pub fn leak(self) -> (ValueOf<DB>, ValueOf<DB>, ValueOf<DB>, usize) {
        let (tuple, empty, len) = self.tuple.leak();
        (self.raw.leak(), tuple, empty, len)
    }

    /// Initialize from a previously leaked one.
    pub fn from_leaked(raw_root: ValueOf<DB>, tuple_root: ValueOf<DB>, empty_root: ValueOf<DB>, len: usize) -> Self {
        Self {
            raw: MerkleRaw::from_leaked(raw_root),
            tuple: MerkleTuple::from_leaked(tuple_root, empty_root, len),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::Sha256;

    type InMemory = crate::traits::InMemoryMerkleDB<Sha256, VecValue>;

    #[derive(Clone, PartialEq, Eq, Debug, Default)]
    struct VecValue(Vec<u8>);

    impl AsRef<[u8]> for VecValue {
        fn as_ref(&self) -> &[u8] {
            self.0.as_ref()
        }
    }

    impl From<usize> for VecValue {
        fn from(value: usize) -> Self {
            VecValue((&(value as u64).to_le_bytes()[..]).into())
        }
    }

    impl Into<usize> for VecValue {
        fn into(self) -> usize {
            let mut raw = [0u8; 8];
            (&mut raw).copy_from_slice(&self.0[0..8]);
            u64::from_le_bytes(raw) as usize
        }
    }

    #[test]
    fn test_push_pop() {
        let mut db = InMemory::default();
        let mut vec = MerkleVec::create(&mut db);

        for i in 0..100 {
            assert_eq!(vec.len(), i);
            vec.push(&mut db, i.into());
        }
        assert_eq!(vec.len(), 100);
        for i in (0..100).rev() {
            let value = vec.pop(&mut db);
            assert_eq!(value, Some(i.into()));
            assert_eq!(vec.len(), i);
        }
        assert_eq!(vec.len(), 0);
    }

    #[test]
    fn test_set() {
        let mut db = InMemory::default();
        let mut vec = MerkleVec::create(&mut db);

        for i in 0..100 {
            assert_eq!(vec.len(), i);
            vec.push(&mut db, Default::default());
        }

        for i in 0..100 {
            vec.set(&mut db, i, i.into());
        }
        for i in 0..100 {
            assert_eq!(vec.get(&db, i), i.into());
        }
    }
}
