use core::num::NonZeroUsize;

use crate::traits::{RawListDB, EndOf, Value};
use crate::raw::RawList;

const LEN_INDEX: usize = 3;

pub struct MerkleVec<DB: RawListDB> {
    raw: RawList<DB>,
}

impl<DB: RawListDB> MerkleVec<DB> where
    EndOf<DB>: From<usize> + Into<usize>,
{
    fn set_len(&mut self, len: usize) {
        self.raw.set(
            NonZeroUsize::new(LEN_INDEX).expect("LEN_INDEX is non-zero; qed"),
            Value::End(len.into())
        );
    }

    pub fn len(&self) -> usize {
        self.raw.get(NonZeroUsize::new(LEN_INDEX).expect("LEN_INDEX is non-zero; qed"))
            .expect("Valid merkle vec must exist in item index 3.")
            .end()
            .expect("Invalid structure for merkle vec.")
            .into()
    }

    pub fn new_with_default(default_value: EndOf<DB>) -> Self {
        let mut raw = RawList::new_with_default(default_value);
        let mut ret = Self { raw };
        ret.set_len(0);
        ret
    }
}
