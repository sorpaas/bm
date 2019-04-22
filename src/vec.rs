use core::num::NonZeroUsize;

use crate::traits::{MerkleDB, EndOf, Value, ValueOf};
use crate::empty::MerkleEmpty;
use crate::raw::MerkleRaw;

const EXTEND_INDEX: NonZeroUsize = unsafe { NonZeroUsize::new_unchecked(4) };
const LEN_INDEX: NonZeroUsize = unsafe { NonZeroUsize::new_unchecked(3) };
const ITEM_ROOT_INDEX: NonZeroUsize = unsafe { NonZeroUsize::new_unchecked(2) };
const ROOT_INDEX: NonZeroUsize = unsafe { NonZeroUsize::new_unchecked(1) };

/// Binary merkle vector.
pub struct MerkleVec<DB: MerkleDB> {
    raw: MerkleRaw<DB>,
    empty: MerkleEmpty<DB>,
}

impl<DB: MerkleDB> MerkleVec<DB> where
    EndOf<DB>: From<usize> + Into<usize>,
{
    fn set_len(&mut self, db: &mut DB, len: usize) {
        self.raw.set(db, LEN_INDEX, Value::End(len.into()));
    }

    fn extend(&mut self, db: &mut DB) {
        self.empty.extend(db);
        let len_raw = self.raw.get(db, LEN_INDEX).expect("Len must exist");
        let item_root_raw = self.raw.get(db, ITEM_ROOT_INDEX).expect("Item root must exist");
        let mut new_raw = MerkleRaw::new();
        new_raw.set(db, ITEM_ROOT_INDEX, self.empty.root());
        new_raw.set(db, LEN_INDEX, len_raw);
        new_raw.set(db, EXTEND_INDEX, item_root_raw);
        self.raw.set(db, ROOT_INDEX, Value::End(Default::default()));
        self.raw = new_raw;
    }

    fn shrink(&mut self, db: &mut DB) {
        self.empty.shrink(db);
        match self.raw.get(db, EXTEND_INDEX) {
            Some(extended_value) => { self.raw.set(db, ITEM_ROOT_INDEX, extended_value); },
            None => { self.raw.set(db, ITEM_ROOT_INDEX, Value::End(Default::default())); },
        }
    }

    fn raw_index(&self, db: &mut DB, i: usize) -> NonZeroUsize {
        let max_len = self.max_len(db);
        NonZeroUsize::new(max_len * 2 + i).expect("Got usize must be greater than 0")
    }

    fn max_len(&self, db: &mut DB) -> usize {
        let len = self.len(db);
        if len == 0 {
            return 0
        } else {
            let mut ret = 2;
            while ret < len {
                ret *= 2;
            }
            ret
        }
    }

    /// Root of the current merkle vector.
    pub fn root(&self) -> ValueOf<DB> {
        self.raw.root()
    }

    /// Push a new value to the vector.
    pub fn push(&mut self, db: &mut DB, value: EndOf<DB>) {
        let old_len = self.len(db);
        if old_len == self.max_len(db) {
            self.extend(db);
        }
        let len = old_len + 1;
        let index = old_len;
        self.set_len(db, len);

        let raw_index = self.raw_index(db, index);
        self.raw.set(db, raw_index, Value::End(value));
    }

    /// Pop a value from the vector.
    pub fn pop(&mut self, db: &mut DB) -> Option<EndOf<DB>> {
        let old_len = self.len(db);
        if old_len == 0 {
            return None
        }

        let len = old_len - 1;
        let index = old_len - 1;
        let raw_index = self.raw_index(db, index);
        let value = self.raw.get(db, raw_index).map(|value| value.end().expect("Invalid format"));

        if len <= self.max_len(db) / 2 && len != 1 {
            self.shrink(db);
        }
        self.set_len(db, len);
        value
    }

    /// Length of the vector.
    pub fn len(&self, db: &mut DB) -> usize {
        self.raw.get(db, LEN_INDEX)
            .expect("Valid merkle vec must exist in item index 3.")
            .end()
            .expect("Invalid structure for merkle vec.")
            .into()
    }

    /// Create a new vector.
    pub fn create(db: &mut DB) -> Self {
        let empty = MerkleEmpty::new();
        let raw = MerkleRaw::new();
        let mut ret = Self { raw, empty };
        ret.set_len(db, 0);
        ret
    }

    /// Drop the current vector.
    pub fn drop(self, db: &mut DB) {
        self.raw.drop(db);
        self.empty.drop(db);
    }

    /// Leak the current vector.
    pub fn leak(self) -> (ValueOf<DB>, ValueOf<DB>) {
        (self.raw.leak(), self.empty.leak())
    }

    /// Initialize from a previously leaked one.
    pub fn from_leaked(raw_root: ValueOf<DB>, empty_root: ValueOf<DB>) -> Self {
        Self {
            raw: MerkleRaw::from_leaked(raw_root),
            empty: MerkleEmpty::from_leaked(empty_root),
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
            assert_eq!(vec.len(&mut db), i);
            vec.push(&mut db, i.into());
        }
        assert_eq!(vec.len(&mut db), 100);
        for i in (0..100).rev() {
            let value = vec.pop(&mut db);
            assert_eq!(value, Some(i.into()));
            assert_eq!(vec.len(&mut db), i);
        }
        assert_eq!(vec.len(&mut db), 0);
    }
}
