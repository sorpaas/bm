use generic_array::{GenericArray, ArrayLength};
use core::ops::Range;
use core::cmp;
use core::marker::PhantomData;
use alloc::vec::Vec;

use crate::length::LengthMixed;
use crate::vector::Vector;
use crate::raw::Raw;
use crate::traits::{Value, EndOf, Backend, ValueOf, RootStatus, Owned, Dangling, Leak, Tree, Sequence, Error};
use crate::utils::host_len;

fn coverings<Host: ArrayLength<u8>, Value: ArrayLength<u8>>(value_index: usize) -> (usize, Vec<Range<usize>>) {
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

/// `PackedVector` with owned root.
pub type OwnedPackedVector<DB, T, H, V> = PackedVector<Owned, DB, T, H, V>;

/// `PackedVector` with dangling root.
pub type DanglingPackedVector<DB, T, H, V> = PackedVector<Dangling, DB, T, H, V>;

/// Packed merkle tuple.
pub struct PackedVector<R: RootStatus, DB: Backend, T, H: ArrayLength<u8>, V: ArrayLength<u8>> {
    tuple: Vector<R, DB>,
    len: usize,
    max_len: Option<usize>,
    _marker: PhantomData<(T, H, V)>,
}

impl<R: RootStatus, DB: Backend, T, H: ArrayLength<u8>, V: ArrayLength<u8>> PackedVector<R, DB, T, H, V> where
    EndOf<DB>: From<GenericArray<u8, H>> + Into<GenericArray<u8, H>>,
    T: From<GenericArray<u8, V>> + Into<GenericArray<u8, V>>,
{
    /// Get value at index.
    pub fn get(&self, db: &DB, index: usize) -> Result<T, Error<DB::Error>> {
        let mut ret = GenericArray::<u8, V>::default();
        let (covering_base, covering_ranges) = coverings::<H, V>(index);

        let mut value_offset = 0;
        for (i, range) in covering_ranges.into_iter().enumerate() {
            let host_value: GenericArray<u8, H> = self.tuple.get(db, covering_base + i)?
                .end().ok_or(Error::CorruptedDatabase)?.into();
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
            let mut host_value: GenericArray<u8, H> = self.tuple.get(db, covering_base + i)?
                .end().ok_or(Error::CorruptedDatabase)?.into();
            (&mut host_value[range.clone()]).copy_from_slice(&value[value_offset..(value_offset + range.end - range.start)]);
            self.tuple.set(db, covering_base + i, Value::End(host_value.into()))?;
            value_offset += range.end - range.start;
        }

        Ok(())
    }

    /// Push a new value to the tuple.
    pub fn push(&mut self, db: &mut DB, value: T) -> Result<(), Error<DB::Error>> {
        let index = self.len;
        let (covering_base, covering_ranges) = coverings::<H, V>(index);

        while self.tuple.len() < covering_base + covering_ranges.len() {
            self.tuple.push(db, Value::End(Default::default()))?;
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

            let (covering_base, covering_ranges) = coverings::<H, V>(last_index);
            while self.tuple.len() > covering_base + covering_ranges.len() {
                self.tuple.pop(db)?;
            }

            let last_value = self.get(db, last_index)?;
            self.tuple.pop(db)?;
            self.tuple.push(db, Value::End(Default::default()))?;
            self.set(db, last_index, last_value)?;
        }

        self.len -= 1;
        Ok(Some(ret))
    }

    /// Create a packed tuple from raw merkle tree.
    pub fn from_raw(raw: Raw<R, DB>, len: usize, max_len: Option<usize>) -> Self {
        let host_max_len = max_len.map(|l| host_len::<H, V>(l));
        let host_len = host_len::<H, V>(len);
        Self {
            tuple: Vector::from_raw(raw, host_len, host_max_len),
            len,
            max_len,
            _marker: PhantomData,
        }
    }
}

impl<R: RootStatus, DB: Backend, T, H: ArrayLength<u8>, V: ArrayLength<u8>> Tree for PackedVector<R, DB, T, H, V> where
    EndOf<DB>: From<GenericArray<u8, H>> + Into<GenericArray<u8, H>>,
    T: From<GenericArray<u8, V>> + Into<GenericArray<u8, V>>,
{
    type RootStatus = R;
    type Backend = DB;

    fn root(&self) -> ValueOf<DB> {
        self.tuple.root()
    }

    fn drop(self, db: &mut DB) -> Result<(), Error<DB::Error>> {
        self.tuple.drop(db)
    }

    fn into_raw(self) -> Raw<R, DB> {
        self.tuple.into_raw()
    }
}

impl<R: RootStatus, DB: Backend, T, H: ArrayLength<u8>, V: ArrayLength<u8>> Sequence for PackedVector<R, DB, T, H, V> where
    EndOf<DB>: From<GenericArray<u8, H>> + Into<GenericArray<u8, H>>,
    T: From<GenericArray<u8, V>> + Into<GenericArray<u8, V>>,
{
    fn len(&self) -> usize {
        self.len
    }
}

impl<R: RootStatus, DB: Backend, T, H: ArrayLength<u8>, V: ArrayLength<u8>> Leak for PackedVector<R, DB, T, H, V> where
    EndOf<DB>: From<GenericArray<u8, H>> + Into<GenericArray<u8, H>>,
    T: From<GenericArray<u8, V>> + Into<GenericArray<u8, V>>,
{
    type Metadata = (ValueOf<DB>, usize, Option<usize>);

    fn metadata(&self) -> Self::Metadata {
        let value_len = self.len();
        let value_max_len = self.max_len;
        let (tuple_root, _host_len, _host_max_len) = self.tuple.metadata();
        (tuple_root, value_len, value_max_len)
    }

    fn from_leaked((raw_root, value_len, value_max_len): Self::Metadata) -> Self {
        Self {
            tuple: Vector::from_leaked((raw_root, host_len::<H, V>(value_len), value_max_len.map(|l| host_len::<H, V>(l)))),
            len: value_len,
            max_len: value_max_len,
            _marker: PhantomData,
        }
    }
}

impl<DB: Backend, T, H: ArrayLength<u8>, V: ArrayLength<u8>> PackedVector<Owned, DB, T, H, V> where
    EndOf<DB>: From<GenericArray<u8, H>> + Into<GenericArray<u8, H>>,
    T: From<GenericArray<u8, V>> + Into<GenericArray<u8, V>>,
{
    /// Create a new tuple.
    pub fn create(db: &mut DB, value_len: usize, value_max_len: Option<usize>) -> Result<Self, Error<DB::Error>> {
        let host_max_len = value_max_len.map(|l| host_len::<H, V>(l));
        let host_len = host_len::<H, V>(value_len);

        let tuple = Vector::create(db, host_len, host_max_len)?;
        Ok(Self {
            tuple,
            len: value_len,
            max_len: value_max_len,
            _marker: PhantomData,
        })
    }
}

/// `PackedList` with owned root.
pub type OwnedPackedList<DB, T, H, V> = PackedList<Owned, DB, T, H, V>;

/// `PackedList` with dangling root.
pub type DanglingPackedList<DB, T, H, V> = PackedList<Dangling, DB, T, H, V>;

/// Packed merkle vector.
pub struct PackedList<R: RootStatus, DB: Backend, T, H: ArrayLength<u8>, V: ArrayLength<u8>>(
    LengthMixed<R, DB, PackedVector<Dangling, DB, T, H, V>>,
) where
    T: From<GenericArray<u8, V>> + Into<GenericArray<u8, V>>,
    EndOf<DB>: From<GenericArray<u8, H>> + Into<GenericArray<u8, H>>;

impl<R: RootStatus, DB: Backend, T, H: ArrayLength<u8>, V: ArrayLength<u8>> PackedList<R, DB, T, H, V> where
    EndOf<DB>: From<usize> + Into<usize> + From<GenericArray<u8, H>> + Into<GenericArray<u8, H>>,
    T: From<GenericArray<u8, V>> + Into<GenericArray<u8, V>>,
{
    /// Get value at index.
    pub fn get(&self, db: &DB, index: usize) -> Result<T, Error<DB::Error>> {
        self.0.with(db, |tuple, db| tuple.get(db, index))
    }

    /// Set value at index.
    pub fn set(&mut self, db: &mut DB, index: usize, value: T) -> Result<(), Error<DB::Error>> {
        self.0.with_mut(db, |tuple, db| tuple.set(db, index, value))
    }

    /// Push a new value to the vector.
    pub fn push(&mut self, db: &mut DB, value: T) -> Result<(), Error<DB::Error>> {
        self.0.with_mut(db, |tuple, db| tuple.push(db, value))
    }

    /// Pop a value from the vector.
    pub fn pop(&mut self, db: &mut DB) -> Result<Option<T>, Error<DB::Error>> {
        self.0.with_mut(db, |tuple, db| tuple.pop(db))
    }
}

impl<R: RootStatus, DB: Backend, T, H: ArrayLength<u8>, V: ArrayLength<u8>> Tree for PackedList<R, DB, T, H, V> where
    EndOf<DB>: From<usize> + Into<usize> + From<GenericArray<u8, H>> + Into<GenericArray<u8, H>>,
    T: From<GenericArray<u8, V>> + Into<GenericArray<u8, V>>,
{
    type RootStatus = R;
    type Backend = DB;

    fn root(&self) -> ValueOf<DB> {
        self.0.root()
    }

    fn drop(self, db: &mut DB) -> Result<(), Error<DB::Error>> {
        self.0.drop(db)
    }

    fn into_raw(self) -> Raw<R, DB> {
        self.0.into_raw()
    }
}

impl<R: RootStatus, DB: Backend, T, H: ArrayLength<u8>, V: ArrayLength<u8>> Sequence for PackedList<R, DB, T, H, V> where
    EndOf<DB>: From<usize> + Into<usize> + From<GenericArray<u8, H>> + Into<GenericArray<u8, H>>,
    T: From<GenericArray<u8, V>> + Into<GenericArray<u8, V>>,
{
    fn len(&self) -> usize {
        self.0.len()
    }
}

impl<R: RootStatus, DB: Backend, T, H: ArrayLength<u8>, V: ArrayLength<u8>> Leak for PackedList<R, DB, T, H, V> where
    EndOf<DB>: From<usize> + Into<usize> + From<GenericArray<u8, H>> + Into<GenericArray<u8, H>>,
    T: From<GenericArray<u8, V>> + Into<GenericArray<u8, V>>,
{
    type Metadata = <LengthMixed<R, DB, Vector<Dangling, DB>> as Leak>::Metadata;

    fn metadata(&self) -> Self::Metadata {
        self.0.metadata()
    }

    fn from_leaked(metadata: Self::Metadata) -> Self {
        Self(LengthMixed::from_leaked(metadata))
    }
}

impl<DB: Backend, T, H: ArrayLength<u8>, V: ArrayLength<u8>> PackedList<Owned, DB, T, H, V> where
    EndOf<DB>: From<usize> + Into<usize> + From<GenericArray<u8, H>> + Into<GenericArray<u8, H>>,
    T: From<GenericArray<u8, V>> + Into<GenericArray<u8, V>>,
{
    /// Create a new vector.
    pub fn create(db: &mut DB, max_len: Option<usize>) -> Result<Self, Error<DB::Error>> {
        Ok(Self(LengthMixed::create(db, |db| PackedVector::<Owned, _, T, H, V>::create(db, 0, max_len))?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::Sha256;
    use crate::traits::Owned;
    use typenum::{U8, U32};

    type InMemory = crate::memory::InMemoryBackend<Sha256, ListValue>;

    #[derive(Clone, PartialEq, Eq, Debug, Default)]
    struct ListValue([u8; 8]);

    impl AsRef<[u8]> for ListValue {
        fn as_ref(&self) -> &[u8] {
            self.0.as_ref()
        }
    }

    impl From<usize> for ListValue {
        fn from(value: usize) -> Self {
            ListValue((value as u64).to_le_bytes())
        }
    }

    impl Into<usize> for ListValue {
        fn into(self) -> usize {
            let mut raw = [0u8; 8];
            (&mut raw).copy_from_slice(&self.0[0..8]);
            u64::from_le_bytes(raw) as usize
        }
    }

    impl From<GenericArray<u8, U8>> for ListValue {
        fn from(arr: GenericArray<u8, U8>) -> ListValue {
            let mut raw = [0u8; 8];
            (&mut raw).copy_from_slice(&arr[0..8]);
            ListValue(raw)
        }
    }

    impl Into<GenericArray<u8, U8>> for ListValue {
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
        let mut db = InMemory::new_with_inherited_empty();
        let mut tuple = PackedVector::<Owned, _, GenericArray<u8, U32>, U8, U32>::create(&mut db, 0, None).unwrap();

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
        let mut db = InMemory::new_with_inherited_empty();
        let mut vec = PackedList::<Owned, _, GenericArray<u8, U32>, U8, U32>::create(&mut db, None).unwrap();

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
