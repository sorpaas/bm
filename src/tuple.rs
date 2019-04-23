use core::num::NonZeroUsize;

use crate::traits::{MerkleDB, EndOf, Value, ValueOf};
use crate::empty::MerkleEmpty;
use crate::raw::MerkleRaw;

/// Binary merkle tuple.
pub struct MerkleTuple<DB: MerkleDB> {
    raw: MerkleRaw<DB>,
    len: usize,
}

impl<DB: MerkleDB> MerkleTuple<DB> {
    fn raw_index(&self, i: usize) -> NonZeroUsize {
        NonZeroUsize::new(self.max_len() + i).expect("Got usize must be greater than 0")
    }

    fn max_len(&self) -> usize {
        crate::utils::next_power_of_two(self.len())
    }

    /// Create a new tuple.
    pub fn create(db: &mut DB, len: usize) -> Self {
        let mut empty = MerkleEmpty::<DB>::new();

        let mut max_len = 1;
        while len < max_len {
            empty.extend(db);
            max_len *= 2;
        }

        let root = empty.leak();

        Self {
            raw: MerkleRaw::<DB>::from_leaked(root),
            len,
        }
    }

    /// Get value at index.
    pub fn get(&self, db: &DB, index: usize) -> EndOf<DB> {
        assert!(index < self.len());

        let raw_index = self.raw_index(index);
        self.raw.get(db, raw_index).expect("Invalid database")
            .end()
            .expect("Invalid database")
    }

    /// Set value at index.
    pub fn set(&mut self, db: &mut DB, index: usize, value: EndOf<DB>) {
        assert!(index < self.len());

        let raw_index = self.raw_index(index);
        self.raw.set(db, raw_index, Value::End(value));
    }

    /// Get the length of the tuple.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Root of the current merkle tuple.
    pub fn root(&self) -> ValueOf<DB> {
        self.raw.root()
    }

    /// Drop the current tuple.
    pub fn drop(self, db: &mut DB) {
        self.raw.drop(db);
    }

    /// Leak the current tuple.
    pub fn leak(self) -> (ValueOf<DB>, usize) {
        let len = self.len();
        (self.raw.leak(), len)
    }

    /// Initialize from a previously leaked one.
    pub fn from_leaked(raw_root: ValueOf<DB>, len: usize) -> Self {
        Self {
            raw: MerkleRaw::from_leaked(raw_root),
            len,
        }
    }
}
