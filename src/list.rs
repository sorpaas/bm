use crate::traits::{Backend, EndOf, Value, ValueOf, RootStatus, Dangling, Owned, Leak, Error};
use crate::vector::Vector;
use crate::raw::Raw;
use crate::index::Index;

const LEN_INDEX: Index = Index::root().right();
const ITEM_ROOT_INDEX: Index = Index::root().left();

/// `List` with owned root.
pub type OwnedList<DB> = List<Owned, DB>;

/// `List` with dangling root.
pub type DanglingList<DB> = List<Dangling, DB>;

/// Binary merkle vector.
pub struct List<R: RootStatus, DB: Backend> {
    raw: Raw<R, DB>,
    tuple: Vector<Dangling, DB>,
}

impl<R: RootStatus, DB: Backend> List<R, DB> where
    EndOf<DB>: From<usize> + Into<usize>,
{
    fn update_metadata(&mut self, db: &mut DB) -> Result<(), Error<DB::Error>> {
        self.raw.set(db, ITEM_ROOT_INDEX, self.tuple.root())?;
        self.raw.set(db, LEN_INDEX, Value::End(self.tuple.len().into()))?;
        Ok(())
    }

    /// Get value at index.
    pub fn get(&self, db: &DB, index: usize) -> Result<EndOf<DB>, Error<DB::Error>> {
        self.tuple.get(db, index)
    }

    /// Set value at index.
    pub fn set(&mut self, db: &mut DB, index: usize, value: EndOf<DB>) -> Result<(), Error<DB::Error>> {
        self.tuple.set(db, index, value)?;
        self.update_metadata(db)?;
        Ok(())
    }

    /// Root of the current merkle vector.
    pub fn root(&self) -> ValueOf<DB> {
        self.raw.root()
    }

    /// Push a new value to the vector.
    pub fn push(&mut self, db: &mut DB, value: EndOf<DB>) -> Result<(), Error<DB::Error>> {
        self.tuple.push(db, value)?;
        self.update_metadata(db)?;
        Ok(())
    }

    /// Pop a value from the vector.
    pub fn pop(&mut self, db: &mut DB) -> Result<Option<EndOf<DB>>, Error<DB::Error>> {
        let ret = self.tuple.pop(db);
        self.update_metadata(db)?;
        ret
    }

    /// Length of the vector.
    pub fn len(&self) -> usize {
        self.tuple.len()
    }

    /// Drop the current vector.
    pub fn drop(self, db: &mut DB) -> Result<(), Error<DB::Error>> {
        self.raw.drop(db)?;
        self.tuple.drop(db)?;
        Ok(())
    }

    /// Deconstruct the vector into one single hash value, and leak only the hash value.
    pub fn deconstruct(self, db: &mut DB) -> Result<ValueOf<DB>, Error<DB::Error>> {
        self.raw.get(db, LEN_INDEX)?;
        self.raw.get(db, ITEM_ROOT_INDEX)?;
        Ok(self.raw.metadata())
    }

    /// Reconstruct the vector from a single hash value.
    pub fn reconstruct(root: ValueOf<DB>, db: &mut DB) -> Result<Self, Error<DB::Error>> {
        let raw = Raw::<R, DB>::from_leaked(root);
        let len: usize = raw.get(db, LEN_INDEX)?
            .ok_or(Error::CorruptedDatabase)?
            .end()
            .ok_or(Error::CorruptedDatabase)?
            .into();
        let tuple_root = raw.get(db, ITEM_ROOT_INDEX)?
            .ok_or(Error::CorruptedDatabase)?;

        let tuple = Vector::<Dangling, DB>::from_leaked((tuple_root, len));

        Ok(Self {
            raw,
            tuple,
        })
    }
}

impl<R: RootStatus, DB: Backend> Leak for List<R, DB> where
    EndOf<DB>: From<usize> + Into<usize>,
{
    type Metadata = (ValueOf<DB>, ValueOf<DB>, usize);

    fn metadata(&self) -> Self::Metadata {
        let (tuple, len) = self.tuple.metadata();
        (self.raw.metadata(), tuple, len)
    }

    fn from_leaked((raw_root, tuple_root, len): Self::Metadata) -> Self {
        Self {
            raw: Raw::from_leaked(raw_root),
            tuple: Vector::from_leaked((tuple_root, len)),
        }
    }
}

impl<DB: Backend> List<Owned, DB> where
    EndOf<DB>: From<usize> + Into<usize>
{
    /// Create a new vector.
    pub fn create(db: &mut DB) -> Result<Self, Error<DB::Error>> {
        let tuple = Vector::create(db, 0)?;
        let mut raw = Raw::default();

        raw.set(db, ITEM_ROOT_INDEX, tuple.root())?;
        raw.set(db, LEN_INDEX, Value::End(tuple.len().into()))?;
        let metadata = tuple.metadata();
        tuple.drop(db)?;
        let dangling_tuple = Vector::from_leaked(metadata);

        Ok(Self { raw, tuple: dangling_tuple })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::Sha256;

    type InMemory = crate::traits::InMemoryBackend<Sha256, ListValue>;

    #[derive(Clone, PartialEq, Eq, Debug, Default)]
    struct ListValue(Vec<u8>);

    impl AsRef<[u8]> for ListValue {
        fn as_ref(&self) -> &[u8] {
            self.0.as_ref()
        }
    }

    impl From<usize> for ListValue {
        fn from(value: usize) -> Self {
            ListValue((&(value as u64).to_le_bytes()[..]).into())
        }
    }

    impl Into<usize> for ListValue {
        fn into(self) -> usize {
            let mut raw = [0u8; 8];
            (&mut raw).copy_from_slice(&self.0[0..8]);
            u64::from_le_bytes(raw) as usize
        }
    }

    fn assert_push_pop_with_db(mut db: InMemory) {
        let mut vec = List::create(&mut db).unwrap();
        let mut roots = Vec::new();

        for i in 0..100 {
            assert_eq!(vec.len(), i);
            vec.push(&mut db, i.into()).unwrap();
            roots.push(vec.root());
        }
        assert_eq!(vec.len(), 100);
        for i in (0..100).rev() {
            assert_eq!(vec.root(), roots.pop().unwrap());
            let value = vec.pop(&mut db).unwrap();
            assert_eq!(value, Some(i.into()));
            assert_eq!(vec.len(), i);
        }
        assert_eq!(vec.len(), 0);
    }

    #[test]
    fn test_push_pop_inherited() {
        assert_push_pop_with_db(InMemory::new_with_inherited_empty());
    }

    #[test]
    fn test_push_pop_unit() {
        assert_push_pop_with_db(InMemory::new_with_unit_empty(ListValue(vec![255])))
    }

    #[test]
    fn test_set() {
        let mut db = InMemory::new_with_inherited_empty();
        let mut vec = List::create(&mut db).unwrap();

        for i in 0..100 {
            assert_eq!(vec.len(), i);
            vec.push(&mut db, Default::default()).unwrap();
        }

        for i in 0..100 {
            vec.set(&mut db, i, i.into()).unwrap();
        }
        for i in 0..100 {
            assert_eq!(vec.get(&db, i).unwrap(), i.into());
        }
    }

    #[test]
    fn test_deconstruct_reconstruct() {
        let mut db = InMemory::new_with_inherited_empty();
        let mut vec = OwnedList::create(&mut db).unwrap();

        for i in 0..100 {
            assert_eq!(vec.len(), i);
            vec.push(&mut db, i.into()).unwrap();
        }
        let vec_hash = vec.deconstruct(&mut db).unwrap();

        let vec = OwnedList::reconstruct(vec_hash, &mut db).unwrap();
        assert_eq!(vec.len(), 100);
        for i in (0..100).rev() {
            assert_eq!(vec.get(&db, i).unwrap(), i.into());
        }
    }
}
