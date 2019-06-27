use generic_array::{GenericArray, ArrayLength};
use core::ops::Range;
use core::cmp;
use core::marker::PhantomData;

use crate::tuple::MerkleTuple;
use crate::raw::MerkleRaw;
use crate::index::MerkleIndex;
use crate::traits::{EndOf, Value, MerkleDB, ValueOf, RootStatus, OwnedRoot, DanglingRoot, Leak, Error};

pub fn coverings<Host: ArrayLength<u8>, Value: ArrayLength<u8>>(value_index: usize) -> (usize, Vec<Range<usize>>) {
    let host_len = Host::to_usize();
    let value_len = Value::to_usize();

    let bytes = value_len * value_index;
    let host_index = bytes / host_len;
    let offset = bytes - host_len * host_index;

    let mut ranges = Vec::new();
    ranges.push(offset..cmp::min(offset + value_len, host_len));
    let mut covered = cmp::min(offset + value_len, host_len) - offset;

    while covered < value_len {
        let rest = value_len - covered;
        ranges.push(0..cmp::min(rest, host_len));
        covered += cmp::min(rest, host_len);
    }

    (host_index, ranges)
}

/// `MerklePackedTuple` with owned root.
pub type OwnedMerklePackedTuple<DB, T, H, V> = MerklePackedTuple<OwnedRoot, DB, T, H, V>;

/// `MerklePackedTuple` with dangling root.
pub type DanglingMerklePackedTuple<DB, T, H, V> = MerklePackedTuple<DanglingRoot, DB, T, H, V>;

/// Packed merkle tuple.
pub struct MerklePackedTuple<R: RootStatus, DB: MerkleDB, T, H: ArrayLength<u8>, V: ArrayLength<u8>> {
    tuple: MerkleTuple<R, DB>,
    len: usize,
    _marker: PhantomData<(T, H, V)>,
}

impl<R: RootStatus, DB: MerkleDB, T, H: ArrayLength<u8>, V: ArrayLength<u8>> MerklePackedTuple<R, DB, T, H, V> where
    EndOf<DB>: From<GenericArray<u8, H>> + Into<GenericArray<u8, H>>,
    T: From<GenericArray<u8, V>> + Into<GenericArray<u8, V>>,
{
    /// Get value at index.
    pub fn get(&self, db: &DB, index: usize) -> Result<T, Error<DB::Error>> {
        let mut ret = GenericArray::<u8, V>::default();
        let (covering_base, covering_ranges) = coverings::<H, V>(index);

        let mut value_offset = 0;
        for (i, range) in covering_ranges.into_iter().enumerate() {
            let host_value: GenericArray<u8, H> = self.tuple.get(db, covering_base + i)?.into();
            (&mut ret[value_offset..(value_offset + range.end - range.start)]).copy_from_slice(&host_value[range.clone()]);
            value_offset += range.end - range.start;
        }

        Ok(ret.into())
    }

    /// Set value at index.
    pub fn set(&mut self, db: &mut DB, index: usize, value: T) -> Result<(), Error<DB::Error>> {
        let value: GenericArray<u8, V> = value.into();
        let (covering_base, covering_ranges) = coverings::<H, V>(index);

        let mut value_offset = 0;
        for (i, range) in covering_ranges.into_iter().enumerate() {
            let mut host_value: GenericArray<u8, H> = self.tuple.get(db, covering_base + i)?.into();
            (&mut host_value[range.clone()]).copy_from_slice(&value[value_offset..(value_offset + range.end - range.start)]);
            self.tuple.set(db, covering_base + i, host_value.into())?;
            value_offset += range.end - range.start;
        }

        Ok(())
    }

    /// Root of the current merkle packed tuple.
    pub fn root(&self) -> ValueOf<DB> { self.tuple.root() }

    /// Root of empty merkle.
    pub fn empty_root(&self) -> ValueOf<DB> { self.tuple.empty_root() }

    /// Push a new value to the tuple.
    pub fn push(&mut self, db: &mut DB, value: T) -> Result<(), Error<DB::Error>> {
        let index = self.len;
        let (covering_base, covering_ranges) = coverings::<H, V>(index);

        while self.tuple.len() < covering_base + covering_ranges.len() {
            self.tuple.push(db, Default::default())?;
        }
        self.set(db, index, value)?;
        self.len += 1;
        Ok(())
    }

    /// Pop a value from the tuple.
    pub fn pop(&mut self, db: &mut DB) -> Result<Option<T>, Error<DB::Error>> {
        if self.len == 0 {
            return Ok(None)
        }

        let index = self.len - 1;
        let ret = self.get(db, index)?;

        if self.len == 1 {
            while self.tuple.len() > 0 {
                self.tuple.pop(db)?;
            }
        } else {
            let last_index = index - 1;

            let (covering_base, covering_ranges) = coverings::<H, V>(index);
            while self.tuple.len() > covering_base + covering_ranges.len() {
                self.tuple.pop(db)?;
            }

            let last_value = self.get(db, last_index)?;
            self.tuple.pop(db)?;
            self.tuple.push(db, Default::default())?;
            self.set(db, last_index, last_value)?;
        }

        self.len -= 1;
        Ok(Some(ret))
    }

    /// Get the length of the tuple.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Drop the current tuple.
    pub fn drop(self, db: &mut DB) {
        self.tuple.drop(db);
    }
}

impl<R: RootStatus, DB: MerkleDB, T, H: ArrayLength<u8>, V: ArrayLength<u8>> Leak for MerklePackedTuple<R, DB, T, H, V> where
    EndOf<DB>: From<GenericArray<u8, H>> + Into<GenericArray<u8, H>>,
    T: From<GenericArray<u8, V>> + Into<GenericArray<u8, V>>,
{
    type Metadata = (ValueOf<DB>, ValueOf<DB>, usize, usize);

    fn metadata(&self) -> Self::Metadata {
        let value_len = self.len();
        let (tuple_root, empty_root, host_len) = self.tuple.metadata();
        (tuple_root, empty_root, host_len, value_len)
    }

    fn from_leaked((raw_root, empty_root, len, value_len): Self::Metadata) -> Self {
        Self {
            tuple: MerkleTuple::from_leaked((raw_root, empty_root, len)),
            len: value_len,
            _marker: PhantomData,
        }
    }
}

impl<DB: MerkleDB, T, H: ArrayLength<u8>, V: ArrayLength<u8>> MerklePackedTuple<OwnedRoot, DB, T, H, V> where
    EndOf<DB>: From<GenericArray<u8, H>> + Into<GenericArray<u8, H>>,
    T: From<GenericArray<u8, V>> + Into<GenericArray<u8, V>>,
{
    /// Create a new tuple.
    pub fn create(db: &mut DB, value_len: usize) -> Result<Self, Error<DB::Error>> {
        let host_len = if value_len == 0 {
            0
        } else {
            let (covering_base, covering_ranges) = coverings::<H, V>(value_len - 1);
            covering_base + covering_ranges.len()
        };

        let tuple = MerkleTuple::create(db, host_len)?;
        Ok(Self {
            tuple,
            len: value_len,
            _marker: PhantomData,
        })
    }
}

/// `MerklePackedVec` with owned root.
pub type OwnedMerklePackedVec<DB, T, H, V> = MerklePackedVec<OwnedRoot, DB, T, H, V>;

/// `MerklePackedVec` with dangling root.
pub type DanglingMerklePackedVec<DB, T, H, V> = MerklePackedVec<DanglingRoot, DB, T, H, V>;

/// Packed merkle vector.
pub struct MerklePackedVec<R: RootStatus, DB: MerkleDB, T, H: ArrayLength<u8>, V: ArrayLength<u8>> {
    tuple: MerklePackedTuple<DanglingRoot, DB, T, H, V>,
    raw: MerkleRaw<R, DB>,
}

const LEN_INDEX: MerkleIndex = MerkleIndex::root().right();
const ITEM_ROOT_INDEX: MerkleIndex = MerkleIndex::root().left();

impl<R: RootStatus, DB: MerkleDB, T, H: ArrayLength<u8>, V: ArrayLength<u8>> MerklePackedVec<R, DB, T, H, V> where
    EndOf<DB>: From<usize> + Into<usize> + From<GenericArray<u8, H>> + Into<GenericArray<u8, H>>,
    T: From<GenericArray<u8, V>> + Into<GenericArray<u8, V>>,
{
    fn update_metadata(&mut self, db: &mut DB) -> Result<(), Error<DB::Error>> {
        self.raw.set(db, ITEM_ROOT_INDEX, self.tuple.root())?;
        self.raw.set(db, LEN_INDEX, Value::End(self.tuple.len().into()))?;
        Ok(())
    }

    /// Get value at index.
    pub fn get(&self, db: &DB, index: usize) -> Result<T, Error<DB::Error>> {
        self.tuple.get(db, index)
    }

    /// Set value at index.
    pub fn set(&mut self, db: &mut DB, index: usize, value: T) -> Result<(), Error<DB::Error>> {
        self.tuple.set(db, index, value)?;
        self.update_metadata(db)?;
        Ok(())
    }

    /// Root of the current merkle vector.
    pub fn root(&self) -> ValueOf<DB> {
        self.raw.root()
    }

    /// Push a new value to the vector.
    pub fn push(&mut self, db: &mut DB, value: T) -> Result<(), Error<DB::Error>> {
        self.tuple.push(db, value)?;
        self.update_metadata(db)?;
        Ok(())
    }

    /// Pop a value from the vector.
    pub fn pop(&mut self, db: &mut DB) -> Result<Option<T>, Error<DB::Error>> {
        let ret = self.tuple.pop(db)?;
        self.update_metadata(db)?;
        Ok(ret)
    }

    /// Length of the vector.
    pub fn len(&self) -> usize {
        self.tuple.len()
    }

    /// Drop the current vector.
    pub fn drop(self, db: &mut DB) {
        self.raw.drop(db);
        self.tuple.drop(db);
    }
}

impl<R: RootStatus, DB: MerkleDB, T, H: ArrayLength<u8>, V: ArrayLength<u8>> Leak for MerklePackedVec<R, DB, T, H, V> where
    EndOf<DB>: From<usize> + Into<usize> + From<GenericArray<u8, H>> + Into<GenericArray<u8, H>>,
    T: From<GenericArray<u8, V>> + Into<GenericArray<u8, V>>,
{
    type Metadata = (ValueOf<DB>, ValueOf<DB>, ValueOf<DB>, usize, usize);

    fn metadata(&self) -> Self::Metadata {
        let (tuple, empty, host_len, len) = self.tuple.metadata();
        (self.raw.metadata(), tuple, empty, host_len, len)
    }

    fn from_leaked((raw_root, tuple_root, empty_root, host_len, len): Self::Metadata) -> Self {
        Self {
            raw: MerkleRaw::from_leaked(raw_root),
            tuple: MerklePackedTuple::from_leaked((tuple_root, empty_root, host_len, len)),
        }
    }
}

impl<DB: MerkleDB, T, H: ArrayLength<u8>, V: ArrayLength<u8>> MerklePackedVec<OwnedRoot, DB, T, H, V> where
    EndOf<DB>: From<usize> + Into<usize> + From<GenericArray<u8, H>> + Into<GenericArray<u8, H>>,
    T: From<GenericArray<u8, V>> + Into<GenericArray<u8, V>>,
{
    /// Create a new vector.
    pub fn create(db: &mut DB) -> Result<Self, Error<DB::Error>> {
        let tuple = MerklePackedTuple::<OwnedRoot, DB, T, H, V>::create(db, 0)?;
        let mut raw = MerkleRaw::default();

        raw.set(db, ITEM_ROOT_INDEX, tuple.root())?;
        raw.set(db, LEN_INDEX, Value::End(tuple.len().into()))?;
        let metadata = tuple.metadata();
        tuple.drop(db);
        let dangling_tuple = MerklePackedTuple::from_leaked(metadata);

        Ok(Self { raw, tuple: dangling_tuple })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::Sha256;
    use crate::traits::OwnedRoot;
    use typenum::{U8, U32};

    type InMemory = crate::traits::InMemoryMerkleDB<Sha256, VecValue>;

    #[derive(Clone, PartialEq, Eq, Debug, Default)]
    struct VecValue([u8; 8]);

    impl AsRef<[u8]> for VecValue {
        fn as_ref(&self) -> &[u8] {
            self.0.as_ref()
        }
    }

    impl From<usize> for VecValue {
        fn from(value: usize) -> Self {
            VecValue((value as u64).to_le_bytes())
        }
    }

    impl Into<usize> for VecValue {
        fn into(self) -> usize {
            let mut raw = [0u8; 8];
            (&mut raw).copy_from_slice(&self.0[0..8]);
            u64::from_le_bytes(raw) as usize
        }
    }

    impl From<GenericArray<u8, U8>> for VecValue {
        fn from(arr: GenericArray<u8, U8>) -> VecValue {
            let mut raw = [0u8; 8];
            (&mut raw).copy_from_slice(&arr[0..8]);
            VecValue(raw)
        }
    }

    impl Into<GenericArray<u8, U8>> for VecValue {
        fn into(self) -> GenericArray<u8, U8> {
            let mut arr: GenericArray<u8, U8> = Default::default();
            (&mut arr[..]).copy_from_slice(&self.0[..]);
            arr
        }
    }

    #[test]
    fn test_coverings() {
        assert_eq!(coverings::<U32, U8>(3), (0, vec![24..32]));
        assert_eq!(coverings::<U32, U8>(4), (1, vec![0..8]));
        assert_eq!(coverings::<U8, U32>(1), (4, vec![0..8, 0..8, 0..8, 0..8]));
    }

    #[test]
    fn test_tuple() {
        let mut db = InMemory::default();
        let mut tuple = MerklePackedTuple::<OwnedRoot, _, GenericArray<u8, U32>, U8, U32>::create(&mut db, 0).unwrap();

        for i in 0..100 {
            let mut value = GenericArray::<u8, U32>::default();
            value[0] = i as u8;
            tuple.push(&mut db, value).unwrap();
        }

        for i in 0..100 {
            let value = tuple.get(&db, i).unwrap();
            assert_eq!(value.as_ref(), &[i as u8, 0, 0, 0, 0, 0, 0, 0,
                                            0, 0, 0, 0, 0, 0, 0, 0,
                                            0, 0, 0, 0, 0, 0, 0, 0,
                                            0, 0, 0, 0, 0, 0, 0, 0]);
        }

        for i in (0..100).rev() {
            let value = tuple.pop(&mut db).unwrap();
            assert_eq!(value.unwrap().as_ref(), &[i as u8, 0, 0, 0, 0, 0, 0, 0,
                                                  0, 0, 0, 0, 0, 0, 0, 0,
                                                  0, 0, 0, 0, 0, 0, 0, 0,
                                                  0, 0, 0, 0, 0, 0, 0, 0]);
        }
    }

    #[test]
    fn test_vec() {
        let mut db = InMemory::default();
        let mut vec = MerklePackedVec::<OwnedRoot, _, GenericArray<u8, U32>, U8, U32>::create(&mut db).unwrap();

        for i in 0..100 {
            let mut value = GenericArray::<u8, U32>::default();
            value[0] = i as u8;
            vec.push(&mut db, value).unwrap();
        }

        for i in 0..100 {
            let value = vec.get(&db, i).unwrap();
            assert_eq!(value.as_ref(), &[i as u8, 0, 0, 0, 0, 0, 0, 0,
                                            0, 0, 0, 0, 0, 0, 0, 0,
                                            0, 0, 0, 0, 0, 0, 0, 0,
                                            0, 0, 0, 0, 0, 0, 0, 0]);
        }

        for i in (0..100).rev() {
            let value = vec.pop(&mut db).unwrap();
            assert_eq!(value.unwrap().as_ref(), &[i as u8, 0, 0, 0, 0, 0, 0, 0,
                                                  0, 0, 0, 0, 0, 0, 0, 0,
                                                  0, 0, 0, 0, 0, 0, 0, 0,
                                                  0, 0, 0, 0, 0, 0, 0, 0]);
        }
    }
}
