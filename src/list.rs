use crate::traits::{Backend, EndOf, Value, ValueOf, RootStatus, Dangling, Owned, Leak, Error, Tree, Sequence};
use crate::vector::Vector;
use crate::raw::Raw;
use crate::index::Index;
use crate::length::LengthMixed;

/// `List` with owned root.
pub type OwnedList<DB> = List<Owned, DB>;

/// `List` with dangling root.
pub type DanglingList<DB> = List<Dangling, DB>;

/// Binary merkle vector.
pub struct List<R: RootStatus, DB: Backend>(LengthMixed<R, DB, Vector<Dangling, DB>>);

impl<R: RootStatus, DB: Backend> List<R, DB> where
    EndOf<DB>: From<usize> + Into<usize>,
{
    /// Get value at index.
    pub fn get(&self, db: &DB, index: usize) -> Result<EndOf<DB>, Error<DB::Error>> {
        self.0.with(db, |tuple, db| tuple.get(db, index))
    }

    /// Set value at index.
    pub fn set(&mut self, db: &mut DB, index: usize, value: EndOf<DB>) -> Result<(), Error<DB::Error>> {
        self.0.with_mut(db, |tuple, db| tuple.set(db, index, value))
    }

    /// Push a new value to the vector.
    pub fn push(&mut self, db: &mut DB, value: EndOf<DB>) -> Result<(), Error<DB::Error>> {
        self.0.with_mut(db, |tuple, db| tuple.push(db, value))
    }

    /// Pop a value from the vector.
    pub fn pop(&mut self, db: &mut DB) -> Result<Option<EndOf<DB>>, Error<DB::Error>> {
        self.0.with_mut(db, |tuple, db| tuple.pop(db))
    }

    /// Deconstruct the vector into one single hash value, and leak only the hash value.
    pub fn deconstruct(self, db: &DB) -> Result<ValueOf<DB>, Error<DB::Error>> {
        self.0.deconstruct(db)
    }

    /// Reconstruct the vector from a single hash value.
    pub fn reconstruct(root: ValueOf<DB>, db: &DB) -> Result<Self, Error<DB::Error>> {
        Ok(Self(LengthMixed::reconstruct(root, db, |tuple_raw, _db, len| {
            Ok(Vector::<Dangling, DB>::from_raw(tuple_raw, len))
        })?))
    }
}

impl<R: RootStatus, DB: Backend> Tree for List<R, DB> where
    EndOf<DB>: From<usize> + Into<usize>,
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

impl<R: RootStatus, DB: Backend> Sequence for List<R, DB> where
    EndOf<DB>: From<usize> + Into<usize>,
{
    fn len(&self) -> usize {
        self.0.len()
    }
}

impl<R: RootStatus, DB: Backend> Leak for List<R, DB> where
    EndOf<DB>: From<usize> + Into<usize>,
{
    type Metadata = <LengthMixed<R, DB, Vector<Dangling, DB>> as Leak>::Metadata;

    fn metadata(&self) -> Self::Metadata {
        self.0.metadata()
    }

    fn from_leaked(metadata: Self::Metadata) -> Self {
        Self(LengthMixed::from_leaked(metadata))
    }
}

impl<DB: Backend> List<Owned, DB> where
    EndOf<DB>: From<usize> + Into<usize>
{
    /// Create a new vector.
    pub fn create(db: &mut DB) -> Result<Self, Error<DB::Error>> {
        Ok(Self(LengthMixed::create(db, |db| Vector::<Owned, _>::create(db, 0))?))
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
